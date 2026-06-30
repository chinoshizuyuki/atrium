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
        pub(crate) use std::collections::HashMap;

        let tokens = super::api_handler::split_query_tokens(msg);
        let queries: Vec<&str> = if tokens.len() > 1 {
            tokens.iter().map(|s| s.as_str()).collect()
        } else {
            vec![msg]
        };

        let mut results: HashMap<String, f64> = HashMap::new();
        for q in &queries {
            // FTS5 全文搜索 / FTS5 full-text search
            if let Ok(fts_results) = self.fts5.lock().search(q, 10) {
                for r in &fts_results {
                    let score = 1.0 / (1.0 + r.rank.abs());
                    results
                        .entry(r.content.clone())
                        .and_modify(|s| *s = s.max(score))
                        .or_insert(score);
                }
            }
            // FactStore 语义匹配 / FactStore semantic matching
            if let Ok(fact_results) = self.fact_store.query(q) {
                for f in fact_results {
                    let key = f.canonical_form();
                    results
                        .entry(key)
                        .and_modify(|s| *s = s.max(f.confidence * 0.8))
                        .or_insert(f.confidence * 0.8);
                }
            }
        }

        // 取 top 3，分数 > 0.3 / Take top 3, score > 0.3
        let mut sorted: Vec<_> = results.into_iter().filter(|(_, s)| *s > 0.3).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(3);

        if sorted.is_empty() {
            return String::new();
        }

        let items: Vec<String> = sorted
            .iter()
            .map(|(content, _)| format!("- {}", content))
            .collect();
        let mut fragment = String::from("[记忆回顾] 你的全局记忆中与此相关的历史信息：\n");
        fragment.push_str(&items.join("\n"));
        fragment
    }

    pub fn fact_store(&self) -> &FactStore {
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
    pub fn set_llm_client(&self, client: std::sync::Arc<dyn atrium_memory::llm_client::LlmClient>) {
        *self.llm_client.lock() = Some(client);
        tracing::info!(
            "[数字生命] LLM trait 客户端已注入 — 意识统一 / Unified consciousness activated"
        );
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

    pub fn try_create_reminder(&self, msg: &str) -> Option<String> {
        if !msg.contains("提醒") && !msg.contains("记得") && !msg.contains("别忘了") {
            return None;
        }

        // 1. 正则解析（<1μs） / 1. Regex parsing (<1μs)
        let parsed = atrium_memory::time_parser::parse_time(msg);
        if let Some(parsed) = parsed {
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

        // 2. LLM 兜底（regex 未命中时） / 2. LLM fallback (when regex misses)
        None // TODO: async LLM fallback via tokio spawn
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
