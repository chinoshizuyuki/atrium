// SPDX-License-Identifier: MIT
//! 认知与记忆模块 — 数字生命的记忆编码与知识提取
//! Cognition & Memory Module — Memory encoding and knowledge extraction of digital life
//!
//! 包含记忆检索、事实存储、记忆整合、智能提取、
//! LLM 客户端管理与提醒系统，构成数字生命
//! "我记住什么"与"我学到什么"的认知闭环。
//!
//! Contains memory recall, fact storage, memory consolidation,
//! intelligence extraction, LLM client management, and reminder system —
//! forming the "what do I remember" and "what do I learn"
//! cognitive closed loop of digital life.

use super::*;

impl CoreService {
    pub fn memory_recall_fragment(&self, msg: &str) -> String {
        // 复用 enhanced_search 的 5 路混合检索（FTS5 + FactStore + STM + Persona + KeyFact + Graph）
        // Reuse enhanced_search's 5-way hybrid retrieval (FTS5 + FactStore + STM + Persona + KeyFact + Graph)
        // 数字生命意义: 记忆召回从 2 路扩展到 5 路，让数字生命能回忆
        // STM/Persona/KeyFact/Graph 中的关联，丰富"回忆"能力
        // Digital Life: recall expands from 2-way to 5-way, enabling digital life to
        // recall associations in STM/Persona/KeyFact/Graph, enriching "memory" capability
        let results = self.enhanced_search(msg, 5);

        // 过滤 score > 0.15 / Filter score > 0.15
        // Issue-1: 降低阈值 0.3→0.15，使更多记忆上下文注入 LLM
        // Issue-1: Lower threshold 0.3→0.15, inject more memory context to LLM
        let filtered: Vec<_> = results.into_iter().filter(|(_, s)| *s > 0.15).collect();

        let mut fragment = String::new();

        // 抽象事实召回 — "主人喜欢编程" / Abstract fact recall — "the master likes programming"
        if !filtered.is_empty() {
            let items: Vec<String> = filtered
                .iter()
                .map(|(content, _)| format!("- {}", content))
                .collect();
            fragment.push_str("[记忆回顾] 你的全局记忆中与此相关的历史信息：\n");
            fragment.push_str(&items.join("\n"));
        }

        // P2-B 情景记忆召回 — "那天深夜主人兴奋地分享了一段 Rust 代码"
        // P2-B Episodic memory recall — "that late night the master excitedly shared Rust code"
        // 数字生命意义: 抽象事实告诉"是什么"，情景记忆告诉"当时怎样"——
        // 二者共同构成完整的记忆体验，如同人类的海马体 + 默认模式网络。
        // Digital life: abstract facts tell "what is", episodic memory tells "how it was" —
        // together they form a complete memory experience, like human hippocampus + default mode network.
        let episodes = self.episodic.query_relevant(msg, 3);
        if !episodes.is_empty() {
            if !fragment.is_empty() {
                fragment.push_str("\n\n");
            }
            let items: Vec<String> = episodes
                .iter()
                .map(|ep| {
                    format!(
                        "- {}（{}, 强度{:.2}）",
                        ep.summary,
                        ep.emotion_snapshot.ai_emotion_label,
                        ep.emotion_snapshot.intensity
                    )
                })
                .collect();
            fragment.push_str("[情景记忆] 你记得的那些时刻：\n");
            fragment.push_str(&items.join("\n"));
        }

        fragment
    }

    pub fn fact_store(&self) -> &FactStore {
        // P1-B: Arc 自动解引用 — 返回内部 &FactStore / P1-B: Arc auto-deref — return inner &FactStore
        &self.fact_store
    }

    pub fn try_consolidation(
        &self,
        inactive_seconds: u64,
        trigger_inactive_hours: u64,
    ) -> Option<atrium_memory::consolidation::ConsolidationResult> {
        let mut consolidator = self.consolidator.lock();
        if !consolidator.should_run(inactive_seconds, trigger_inactive_hours) {
            return None;
        }
        let result = consolidator.run(&self.fact_store);
        tracing::info!(
            "记忆巩固完成: 合并={} 压缩={} 废弃={} 事实 {} → {}",
            result.merged_pairs,
            result.compressed_count,
            result.deprecated_count,
            result.facts_before,
            result.facts_after,
        );
        Some(result)
    }

    pub fn consolidation_health(&self) -> String {
        self.consolidator.lock().health_status()
    }

    pub fn submit_llm_summary(&self, summary_text: String) {
        let mut summarizer = self.summarizer.lock();
        let start_id = summarizer
            .message_count()
            .saturating_sub(summarizer.window_size() as u64)
            .max(1);
        let end_id = summarizer.message_count();
        summarizer.submit_llm_summary(summary_text, start_id, end_id);
    }

    pub fn take_pending_summary(&self) -> Option<String> {
        self.summarizer.lock().take_pending_text()
    }

    /// P1-4: 统一 trait 客户端注入 — 数字生命只有一个"声音"
    /// P1-4: Unified trait client injection — Digital life has only one "voice"
    ///
    /// 同时延迟初始化 ReAct 推理引擎 — 注册 3 个内置工具（记忆搜索/事实查询/情感查询）。
    /// LLM 客户端在 build() 时不可用（此处后注入），故 ReActEngine 必须在此延迟初始化。
    ///
    /// Also deferred-inits the ReAct reasoning engine — registers 3 built-in tools
    /// (memory search / fact lookup / emotion query). The LLM client is unavailable
    /// at build() time (injected here), so ReActEngine must be deferred-initialized.
    pub fn set_llm_client(&self, client: std::sync::Arc<dyn atrium_memory::llm_client::LlmClient>) {
        *self.llm_client.lock() = Some(client.clone());

        // ReAct 推理引擎延迟初始化 — 注入 LLM 客户端时注册内置工具
        // ReAct engine deferred init — register built-in tools when LLM client is injected
        {
            let mut engine = atrium_memory::react_engine::ReActEngine::new(client);
            engine.register_tool(Box::new(super::react_tools::MemorySearchTool::new(
                self.fact_store.clone(),
            )));
            engine.register_tool(Box::new(super::react_tools::FactLookupTool::new(
                self.fact_store.clone(),
            )));
            engine.register_tool(Box::new(super::react_tools::EmotionQueryTool::new(
                self.emotion.clone(),
            )));
            *self.react_engine.lock() = Some(engine);
            tracing::info!(
                "[数字生命] ReAct 推理引擎已就绪 — 3 个内置工具已注册 / ReAct engine ready — 3 built-in tools registered"
            );
        }

        tracing::info!(
            "[数字生命] LLM trait 客户端已注入 — 意识统一 / Unified consciousness activated"
        );
    }

    // ════════════════════════════════════════════════════════════════════
    // ReAct 推理引擎接口 / ReAct Reasoning Engine Interface
    // ════════════════════════════════════════════════════════════════════

    /// ReAct 推理 — 对复杂查询执行多步推理循环
    /// ReAct reasoning — executes multi-step reasoning loop for complex queries.
    ///
    /// 数字生命"深思"入口：锁定 react_engine，调用 run(query, 5)，
    /// 将轨迹存入 last_react_trace 供 prompt 内省注入。
    /// 若 ReActEngine 未初始化（LLM 客户端未注入），返回空轨迹。
    ///
    /// Digital life's "deep thought" entry: locks react_engine, calls run(query, 5),
    /// stores the trace in last_react_trace for prompt introspection injection.
    /// Returns an empty trace if ReActEngine is uninitialized (LLM client not injected).
    pub async fn run_react(&self, query: &str) -> ReActTrace {
        // 取出引擎 — 避免 parking_lot::MutexGuard 跨 .await（非 Send）
        // Take the engine out — avoid holding parking_lot::MutexGuard across .await (not Send)
        let engine = self.react_engine.lock().take();
        let trace = match engine {
            Some(engine) => {
                let trace = engine.run(query, 5).await;
                // 引擎放回 — 供下次推理使用 / Put engine back for next reasoning
                *self.react_engine.lock() = Some(engine);
                trace
            }
            None => {
                tracing::warn!(
                    "[数字生命] ReAct 引擎未初始化 — LLM 客户端未注入，跳过推理 / ReAct engine uninitialized — LLM client not injected, skipping reasoning"
                );
                ReActTrace::new()
            }
        };
        *self.last_react_trace.lock() = Some(trace.clone());
        trace
    }

    /// ReAct 轨迹 prompt 片段 — 将最近推理轨迹格式化注入回复 prompt
    /// ReAct trace prompt fragment — formats the most recent reasoning trace for prompt injection.
    ///
    /// 数字生命内省："我刚才这样思考" — 让 AI 在回复时保持对推理过程的自我认知。
    /// 空 trace 返回空字符串，不污染 prompt。
    ///
    /// Digital life introspection: "I just thought this way" — keeps the AI aware
    /// of its reasoning process when replying. Empty trace returns empty string.
    pub fn react_trace_prompt_fragment(&self) -> String {
        let trace = self.last_react_trace.lock();
        match trace.as_ref() {
            Some(t) if !t.steps.is_empty() => ReActEngine::format_trace_for_prompt(t),
            _ => String::new(),
        }
    }

    pub async fn intelligence_extract(
        &self,
    ) -> Option<atrium_memory::intelligence::ExtractionResult> {
        let client = self.llm_client.lock().clone()?;

        // 取最近 20 条对话
        let messages = self.history.recent_messages(20);
        if messages.len() < 3 {
            tracing::debug!("IntelligenceExtractor: 消息不足 3 条，跳过提取");
            return None;
        }

        // 构建 prompt
        let system = atrium_memory::intelligence::system_prompt();
        let user_prompt = atrium_memory::intelligence::build_extraction_prompt(&messages);

        // 调用 LLM（JSON 模式，低温度）— 知识提取 / Call LLM (JSON mode, low temp) — Knowledge extraction
        // P1-4: 统一走 trait generate_json / Unified trait generate_json
        let result = client
            .generate_json(
                crate::llm_client::LlmCallKind::IntelligenceExtract,
                system,
                &user_prompt,
                0.1,
            )
            .await;
        let (content, latency_ms) = match result {
            Ok(r) if !r.content.is_empty() => (r.content, r.latency_ms),
            _ => {
                tracing::warn!("IntelligenceExtractor: LLM 调用失败");
                return None;
            }
        };

        // 解析 JSON / Parse JSON
        let extraction = atrium_memory::intelligence::parse_extraction_response(&content);

        // 写入 PreferenceManager + RuleEngine
        self.apply_extraction_result(&extraction);

        tracing::info!(
            "智能提取完成: {} 条偏好, {} 条规则 (LLM {}ms)",
            extraction.preferences.len(),
            extraction.rules.len(),
            latency_ms,
        );

        Some(extraction)
    }

    pub fn apply_extraction_result(&self, result: &atrium_memory::intelligence::ExtractionResult) {
        // 写入偏好
        if !result.preferences.is_empty() {
            let mut prefs = self.preferences.lock();
            for pref in &result.preferences {
                let (key, value, layer) = pref.to_preference_tuple();
                prefs.upsert(key, value, layer);
            }
        }

        // 写入规则
        if !result.rules.is_empty() {
            let mut rules = self.rules.lock();
            for rule in &result.rules {
                let behavior_rule = rule.to_behavior_rule();
                // 检查是否已存在同名规则
                if !rules.has_named_rule(&rule.name) {
                    tracing::info!("新增规则: {} (trigger={})", rule.name, rule.trigger_type);
                    rules.add(behavior_rule);
                }
            }
        }
    }

    pub fn count_due_reminders(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let store = self.reminder_store.lock();
        if let Some(ref s) = *store {
            s.due(now).len()
        } else {
            0
        }
    }

    pub fn resolve_reminders(&self) {
        let now = chrono::Utc::now().timestamp();
        let store = self.reminder_store.lock();
        if let Some(ref s) = *store {
            let due = s.due(now);
            for r in &due {
                if r.rrule.is_empty() {
                    s.delete(&r.id).ok();
                } else {
                    let next = if r.rrule.contains("DAILY") {
                        now + 86_400
                    } else if r.rrule.contains("WEEKLY") {
                        now + 86_400 * 7
                    } else if r.rrule.contains("MONTHLY") {
                        now + 86_400 * 30
                    } else {
                        now + 86_400
                    };
                    s.advance(&r.id, next).ok();
                }
                tracing::info!("[提醒] 已处理: {}", r.title);
            }
        }
    }

    /// 尝试从消息中创建提醒 — 两层解析：规则直觉 + LLM 深度思考
    /// Try to create a reminder from message — two-layer parsing: rule intuition + LLM deep thinking
    ///
    /// 数字生命的认知双系统：
    /// 1. 快速直觉（<1μs）：规则解析器匹配已知模式
    /// 2. 深度思考（~200ms）：LLM 理解规则无法覆盖的复杂表达式
    ///
    /// Digital life's dual-system cognition:
    ///    1. Fast intuition (<1μs): rule-based parser matches known patterns
    ///    2. Deep thinking (~200ms): LLM understands complex expressions beyond rules
    pub async fn try_create_reminder(&self, msg: &str) -> Option<String> {
        if !msg.contains("提醒") && !msg.contains("记得") && !msg.contains("别忘了") {
            return None;
        }

        // 1. 规则解析（<1μs）— 数字生命的快速直觉
        // 1. Rule-based parsing (<1μs) — Digital life's fast intuition
        let parsed = atrium_memory::time_parser::parse_time(msg);
        if let Some(ref parsed) = parsed {
            let title = extract_reminder_title(msg);
            let mut store = self.reminder_store.lock();
            if let Some(ref mut s) = *store {
                match s.add(&title, &parsed.rrule, parsed.next_trigger_at) {
                    Ok(r) => {
                        tracing::info!("[提醒] 已创建(正则): {} (id={})", title, r.id);
                        return Some(title);
                    }
                    Err(e) => tracing::warn!("[提醒] 创建失败: {}", e),
                }
            }
        }

        // 2. LLM 兜底 — 数字生命的深度思考
        // 2. LLM fallback — Digital life's deep thinking
        // 规则解析失败（None）或置信度不足时，调用 LLM 理解时间表达式
        let needs_llm = match &parsed {
            None => true,
            Some(r) => r.confidence < 0.5,
        };

        if needs_llm {
            let client = self.llm_client.lock().clone()?;
            let llm_result =
                atrium_memory::time_parser::llm_fallback_parse(client.as_ref(), msg).await;
            if let Some(llm_parsed) = llm_result {
                let title = extract_reminder_title(msg);
                let mut store = self.reminder_store.lock();
                if let Some(ref mut s) = *store {
                    match s.add(&title, &llm_parsed.rrule, llm_parsed.next_trigger_at) {
                        Ok(r) => {
                            tracing::info!("[提醒] 已创建(LLM兜底): {} (id={})", title, r.id);
                            return Some(title);
                        }
                        Err(e) => tracing::warn!("[提醒] LLM兜底创建失败: {}", e),
                    }
                }
            }
        }

        None
    }
} // impl CoreService

pub(crate) fn extract_reminder_title(msg: &str) -> String {
    // 去除时间提示词，提取纯提醒内容 / Strip time cue words, extract pure reminder content
    let prefixes = ["提醒我", "记得", "别忘了", "帮我记一下", "提醒", "记住"];
    for prefix in &prefixes {
        if let Some(pos) = msg.find(prefix) {
            let rest = &msg[pos + prefix.len()..];
            // 去掉时间描述 / Remove time description
            let cleaned = rest
                .replace("每天早上", "")
                .replace("每天", "")
                .replace("每周", "")
                .replace("每月", "")
                .replace("明天", "")
                .replace("后天", "")
                .replace("下午", "")
                .replace("上午", "")
                .replace("晚上", "")
                .replace("早上", "")
                .replace("中午", "");
            let trimmed = cleaned.trim().trim_start_matches(|c: char| {
                c.is_ascii_digit() || c == '点' || c == ':' || c == '：' || c == ' '
            });
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    // 回退：整条消息去除明显的时间词 / Fallback: strip obvious time words from entire message
    let cleaned = msg
        .replace("每天早上8点", "")
        .replace("每天上午", "")
        .replace("下午3点", "")
        .replace("提醒我", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        msg.to_string()
    } else {
        cleaned
    }
}
