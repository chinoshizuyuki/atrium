// SPDX-License-Identifier: MIT
//! 外部接口模块 — 数字生命与世界的交互通道
//! External API Module — The interaction channel between digital life and the world
//!
//! 包含 gRPC 服务实现（process_message / health_check / stream）、
//! 查询分词、抽取式摘要与命名检测，
//! 构成数字生命"如何与外界对话"的接口闭环。
//!
//! Contains gRPC service implementation (process_message / health_check / stream),
//! query tokenization, extractive summarization, and naming detection —
//! forming the "how to converse with the outside" interface closed loop of digital life.

use super::narrative::narrative_vulnerability::RelationshipEvent;
use super::*;

// ════════════════════════════════════════════════════════════════════
// 消息处理阶段化拆分 — 将原 953 行 process_message 拆分为 ≤200 行子函数
// Staged message processing — splitting the original 953-line process_message into ≤200-line sub-functions
// ════════════════════════════════════════════════════════════════════

/// 消息处理中间状态 — 阶段化子函数之间的数据载体
/// Message processing intermediate state — data carrier between staged sub-functions
struct MessageContext<'a> {
    msg: &'a str,
    rhythm: Option<TypingRhythm>,
    subtext_signals: Vec<SubtextSignal>,
    graph_hints: Vec<String>,
    named_just_now: Option<String>,
    emo_state: EmotionEngineState,
    persona_name: String,
    is_unnamed: bool,
    emotion_tag: String,
}

/// 消息处理阶段化子函数 — 单一职责，保持执行顺序以维持行为不变
/// Staged sub-functions for message processing — single responsibility, preserving execution order for behavior invariance
impl CoreService {
    /// 阶段 1: 前置处理 — 历史存储 + 关系追踪 + 心智模型 + 信号检测 + 感知
    /// Stage 1: Preprocessing — history + relationship + mental model + signal detection + perception
    async fn preprocess_message(
        &self,
        msg: &str,
        session_id: &str,
    ) -> (u64, Instant, Option<TypingRhythm>, Vec<SubtextSignal>) {
        let count = self.message_count.fetch_add(1, Ordering::Relaxed) + 1;

        // metrics
        metrics::inc(metrics::keys::MSG_RECEIVED);
        let msg_start = Instant::now();

        // P1-B: 存储对话历史 — spawn_blocking 包装 sled I/O，不阻塞 reactor
        // P1-B: Store conversation history — spawn_blocking wraps sled I/O, never blocks reactor
        if let Err(e) = self
            .append_history_async(session_id, "user", msg, None)
            .await
        {
            tracing::warn!(
                "对话历史写入失败 — 记忆可能受损 / Conversation history write failed — memory may be compromised. session_id: {}, role: user, error: {}",
                session_id, e
            );
        }

        // 关系阶段追踪──
        // Relationship stage tracking
        {
            let hour = chrono::Local::now().hour() as u8;
            let mut rel = self.relationship.write();
            // 记录递增前的阶段与计数 — 用于关系事件弱信号检测 / Record stage and count before increment — for relationship event weak signal detection
            let stage_before = rel.current_stage().stage_name();
            let interactions_before = rel.current_stage().interactions();
            rel.on_message(msg, hour);
            let stage_after = rel.current_stage().stage_name();
            let interactions_after = rel.current_stage().interactions();
            drop(rel);

            // G-08 扩展：关系事件弱信号源 / G-08 extension: relationship event weak signal source
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            if stage_before != stage_after {
                // 阶段推进 → Deepening 信号 / Stage advance → Deepening signal
                self.growth_feedback_on_relationship_event(
                    RelationshipEvent::StageAdvance,
                    now_epoch,
                );
            } else {
                // 同阶段交互递增 — 检查 50/100/500 里程碑 / Same-stage increment — check 50/100/500 milestones
                for &milestone in &[50u64, 100, 500] {
                    if interactions_before < milestone && interactions_after >= milestone {
                        self.growth_feedback_on_relationship_event(
                            RelationshipEvent::InteractionMilestone,
                            now_epoch,
                        );
                        break; // 单次最多触发一个里程碑 / At most one milestone per message
                    }
                }
            }
        }

        // 用户心智模型更新──
        // User mental model update
        {
            let mut um = self.user_model.write();
            um.on_user_message(msg);
        }

        // 反馈信号检测──
        // Feedback signal detection
        {
            let mut fb = self.feedback.write();
            fb.on_user_message(msg);
        }

        // G-08 成长反馈桥接 — FeedbackLoop 信号回流到成长引擎 / G-08 Growth feedback bridge — FeedbackLoop signals flow back to growth engines
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.growth_feedback_on_exchange(now_epoch);
        }

        // 教学意图检测（ACK 自学习 Path A）──
        // Teaching intent detection (ACK self-learning Path A)
        if self.ack_learning_cfg.enabled && self.ack_learning_cfg.user_teach_enabled {
            if let Some(intent) = atrium_memory::teach_detector::detect_teach_intent(msg) {
                let max = self.ack_learning_cfg.max_self_learned_ack;
                let mut canned = self.canned.write();
                match canned.learn_from_user(&intent, max) {
                    Ok(ack) => {
                        tracing::info!("用户教授 → ACK: {}", ack.name);
                        *self.teach_detected.lock() = Some(intent);
                    }
                    Err(e) => tracing::debug!("用户教授跳过: {}", e),
                }
            }
        }

        // 成长管理器：消息处理回调 / Maturity: on_message callback
        {
            let hour = chrono::Local::now().hour() as u8;
            let teach = self.teach_detected.lock();
            let teach_ref = teach.as_ref();
            self.maturity.lock().on_message(msg, hour, teach_ref, false);
        }

        // 叙事事件检测 — Step 0.9 / Narrative event detection — Step 0.9
        if self.narrative_enabled {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.detect_narrative_event(msg, now_epoch);
        }

        // R1 通电：独处品质事件喂入 — 每条用户消息都是"独处中的思考"
        // R1 power-on: solitude quality event feed — each user message is a "thought during solitude"
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.solitude_quality_on_thought(msg, now_epoch);
        }

        // R1-residual 通电：脆弱智慧学习闭环 — 从用户消息推断对上一轮脆弱的反应
        // R1-residual power-on: vulnerability wisdom learning loop
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let history = self.get_history(session_id, 5);
            let prev_ai = history
                .iter()
                .find(|m| m.role == "assistant")
                .map(|m| m.content.as_str())
                .unwrap_or("");
            self.vulnerability_wisdom_on_exchange(msg, prev_ai, now_epoch);
        }

        // R1 通电：仪式涌现事件喂入 — 每条消息都是一次交互模式观察
        // R1 power-on: ritual emergence event feed — each message is an interaction pattern observation
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let valence: f64 = {
                let emo = self.emotion.lock();
                emo.current().pleasure as f64
            };
            let tags = atrium_memory::ritual_emergence::RitualEmergence::context_tags(
                msg.contains("再见") || msg.contains("bye"),
                msg.contains("你好") || msg.contains("hi") || msg.contains("早"),
                msg.len() > 50,
            );
            let mut emergence = self.ritual.emergence.lock();
            emergence.observe(msg, now_epoch, valence, tags);
        }

        // R1 通电：追问风格学习器事件喂入 — 从用户消息推断反应
        // R1 power-on: follow-up style learner event feed — infer reaction from message
        {
            use atrium_memory::followup_tracker::{
                FollowUpCategory, FollowUpDepth, FollowUpStyle, UserReaction,
            };
            // 简化启发式：长消息=正面回应，短消息=中性，含回避词=回避
            // Simplified heuristic: long=engaged, short=neutral, avoidance words=deflected
            let engaged = msg.len() > 30;
            let elaborated = msg.len() > 80;
            let deflected = msg.contains("算了") || msg.contains("不用了") || msg.contains("没事");
            let sentiment: f32 = if deflected {
                -0.5
            } else if engaged {
                0.5
            } else {
                0.0
            };
            self.followup_style_learner_on_outcome(
                FollowUpCategory::Relationship,
                FollowUpDepth::Moderate,
                FollowUpStyle::Caring,
                UserReaction {
                    engaged,
                    sentiment,
                    deflected,
                    elaborated,
                },
            );
        }

        // 打字节奏分析──
        // Typing rhythm analysis
        let rhythm: Option<TypingRhythm> = if self.perception_enabled {
            let event =
                MessageEvent::simple(msg.to_string(), chrono::Utc::now().timestamp_millis());
            let r = self.typing_analyzer.lock().on_message(event);
            // 节奏信号 → 用户心智模型
            self.user_model.write().update_with_rhythm(&r);
            Some(r)
        } else {
            None
        };

        // 潜台词感知 — 读懂"话外之音" / Subtext perception — read between the lines
        // G1 修复：用户潜台词不再是死代码，"我没事"背后的回避终于能被感知
        // G1 fix: user subtext is no longer dead code, avoidance behind "I'm fine" can finally be perceived
        let subtext_signals: Vec<SubtextSignal> = if self.expression_enabled {
            let pad = {
                let emo = self.emotion.lock();
                let c = emo.current();
                [c.pleasure, c.arousal, c.dominance]
            };
            let stage = self.relationship.read().current_stage().clone();
            SubtextEngine::detect(msg, pad, &stage)
        } else {
            Vec::new()
        };

        // G-09: 独处/归来检测 — idle > 300s 视为独处后归来，触发问候机制
        // G-09: Idle/return detection — idle > 300s treated as return from solitude, trigger greeting
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            // 先在锁内计算 idle 并更新时间戳，再释放锁后调用 monologue — 避免持锁调用引发锁序风险
            // Compute idle & update timestamp inside the lock, then release before calling monologue
            let idle_secs_opt: Option<u64> = {
                let mut last = self.last_user_msg_epoch.lock();
                let idle = if *last > 0 {
                    Some((now_epoch - *last).max(0) as u64)
                } else {
                    None
                };
                *last = now_epoch;
                idle
            };
            if let Some(idle_secs) = idle_secs_opt {
                if idle_secs > 300 {
                    // 用户归来 — 结束独处计时 + 重置氛围 + 记录温暖互动
                    // User returns — end solitude timing + reset atmosphere + record warm interaction
                    self.on_user_return(idle_secs);
                    // 生成"我刚才在想..."问候，存入待注入字段，由 build_prompt_fragments_part3 消费
                    // Generate "I was just thinking..." greeting, stash for build_prompt_fragments_part3
                    if let Some(greeting) = self.harvest_solitude_greeting() {
                        if !greeting.is_empty() {
                            *self.pending_solitude_greeting.lock() = Some(greeting);
                        }
                    }
                }
            }
        }

        (count, msg_start, rhythm, subtext_signals)
    }

    /// 阶段 2: 情感更新 — STM 写入 + PAD 情感更新 + 共情推理 + 潜台词反馈
    /// Stage 2: Emotion update — STM write + PAD update + empathy + subtext feedback
    fn update_emotion(
        &self,
        msg: &str,
        count: u64,
        rhythm: &Option<TypingRhythm>,
        subtext_signals: &[SubtextSignal],
    ) {
        // 写入 STM ──
        // Write to STM
        {
            let mut mem = self.memory.lock();
            let _ = mem.remember(
                MemoryEntry::new("user", MemoryContent::Text(msg.to_string())).with_importance(0.3),
            );
        }

        // 影响情感（受关系阶段 + 用户心智模型调制 + 节奏信号 + 共情推理）──
        // Affect emotion (modulated by relationship stage + user model + rhythm + empathy)
        {
            let mut emo = self.emotion.lock();
            let rel_mult = self.relationship.read().affect_multiplier();
            let user_mod = self.user_model.read().emotion_modulation();
            emo.affect(&EmotionEngineState::new(
                0.05 * rel_mult + user_mod.engagement_boost,
                0.02 * rel_mult,
                0.01 * rel_mult,
            ));

            // 节奏信号 → 情感（独立于文本情感，低权重）
            if let Some(r) = rhythm {
                emo.affect(&EmotionEngineState::new(
                    r.mood_hint.mood * 0.3,
                    r.mood_hint.energy * 0.2,
                    r.mood_hint.confidence * 0.1,
                ));
            }

            // 共情推理引擎 — 替代简单 15% 情绪传染
            {
                let stage_name = self
                    .relationship
                    .read()
                    .current_stage()
                    .stage_name()
                    .to_string();
                let mut empathy = self.empathy.write();
                if let Some(result) = empathy.analyze(msg, &stage_name, count) {
                    let (dp, da, dd) = result.pad_delta;
                    emo.affect(&EmotionEngineState::new(dp, da, dd));
                }
            }

            // 潜台词→情感反馈闭环 / Subtext→emotion feedback loop
            // G6 修复：感知到对方的脆弱，自己的情绪也会变得温柔
            // G6 fix: perceiving the other's fragility makes one's own emotion gentle
            if !subtext_signals.is_empty() {
                let mut dp = 0.0f32;
                let mut da = 0.0f32;
                let mut dd = 0.0f32;
                for signal in subtext_signals {
                    // 潜台词类别 → PAD 微调映射 / Subtext category → PAD delta mapping
                    let (p, a, d) = match signal.category {
                        SubtextCategory::Avoidance => (0.02, -0.01, 0.01),
                        SubtextCategory::Fragility => (0.03, 0.01, 0.02),
                        SubtextCategory::SeekingAttention => (0.01, 0.02, 0.00),
                        SubtextCategory::Dissatisfaction => (-0.01, 0.01, -0.01),
                        SubtextCategory::HiddenJoy => (0.02, 0.01, 0.00),
                        _ => (0.00, 0.00, 0.00),
                    };
                    dp += p * signal.confidence;
                    da += a * signal.confidence;
                    dd += d * signal.confidence;
                }
                emo.affect(&EmotionEngineState::new(dp, da, dd));
            }
        }
        // affect 后持久化情感状态
        self.persist_emotion();
    }

    /// 阶段 3: 事实提取 + 反思触发 — FactStore + 提醒 + 关联图 + Reflection
    /// Stage 3: Fact extraction + reflection — FactStore + reminder + graph + reflection
    async fn extract_facts_and_reflect(&self, msg: &str, count: u64) -> Vec<String> {
        // 事实提取 + 证据评分 + FactStore + FTS5 索引 ──
        // P1-B: ingest_memory 已为 async — 批量合并 spawn_blocking 写入 FactStore+FTS5
        // P1-B: ingest_memory is now async — batch-merged spawn_blocking writes FactStore+FTS5
        self.ingest_memory("user", msg, SourceType::DirectConversation)
            .await;

        // 定时提醒解析 / Reminder parsing
        if let Some(title) = self.try_create_reminder(msg).await {
            tracing::info!("[提醒] 从消息中解析到提醒: {}", title);
        }

        // 关联记忆图激活──
        // Associative memory graph activation
        let graph_hints = {
            let seeds = split_query_tokens(msg);
            let mut graph = self.graph.lock();
            let mut hints: Vec<String> = Vec::new();
            for seed in seeds.iter().take(3) {
                let paths = graph.spread_activation(seed, 0.5, 2);
                for p in paths.iter().take(3) {
                    if p.activation >= 0.25 && p.hops >= 2 {
                        if let Some(node) = graph.get_node(&p.to) {
                            hints.push(format!(
                                "[联想] {} → {}: {}",
                                seed, node.content, p.predicate
                            ));
                        }
                    }
                }
            }
            hints
        };

        // 周期性 Reflection ──
        // Periodic reflection
        self.try_reflect(count);

        graph_hints
    }

    /// 阶段 4: 准备响应上下文 — 命名检测 + Token 预算 + 情感/人格读取
    /// Stage 4: Prepare response context — naming + token budget + emotion/persona read
    #[allow(clippy::type_complexity)]
    fn prepare_response_context(
        &self,
        msg: &str,
        count: u64,
    ) -> (
        Option<String>,
        EmotionEngineState,
        String,
        bool,
        &'static str,
        String,
    ) {
        // 命名仪式检查 ──
        // 如果当前人格名仍是默认 "Atrium"（未命名），检测用户是否给出了名字
        let naming_result = {
            let p = self.persona.read();
            p.current().map(|i| i.def.name.clone())
        };

        let mut named_just_now: Option<String> = None;
        if naming_result.as_deref() == Some("Atrium") {
            named_just_now = detect_naming(msg);
            if let Some(ref new_name) = named_just_now {
                let _ = self.persona.write().rename_current(new_name);
                // 同步更新人格防御守卫的 AI 名字
                self.guard.write().set_ai_name(new_name);
            }
        }

        // 用户称呼检测 — 数字生命记住用户是谁 / User designation detection
        // "叫我老王" → 以后永远称呼用户为"老王"，而非硬编码"主人"
        if let Some(ref user_name) = detect_user_naming(msg) {
            self.guard.write().set_master_name(user_name);
            tracing::info!(
                "用户称呼已更新 → 「{}」 / User designation updated",
                user_name
            );
        }

        // Token 预算 + 摘要检查 ──
        // Token budget + summary check
        {
            let mut summarizer = self.summarizer.lock();
            if summarizer.record_message() {
                let mem = self.memory.lock();
                let recent_texts: Vec<String> = mem
                    .recent(summarizer.window_size())
                    .iter()
                    .map(|e| format!("{}: {}", e.role, e.content_str()))
                    .collect();
                drop(mem);

                if !recent_texts.is_empty() {
                    let combined = recent_texts.join("\n");
                    let start_id = count.saturating_sub(summarizer.window_size() as u64).max(1);

                    // 先写入抽取式摘要（即时可用，<10μs）
                    let extractive = extractive_summarize(&combined);
                    summarizer.store_summary(extractive, start_id, count);

                    // 同时存储待 LLM 处理的文本（Python 网关异步拉取并替换）
                    summarizer.pending_llm_text = Some(combined);
                }
            }

            // 更新 token 预算（使用预算约束的摘要上下文）/ Update token budget with bounded summary context
            let summary_budget = self.token_budget.lock().summary_budget();
            let summary_ctx = summarizer.summary_context_bounded(summary_budget);
            let estimated_tokens = TokenBudget::estimate(msg) + TokenBudget::estimate(&summary_ctx);
            self.token_budget.lock().update_used(estimated_tokens);
        }

        // 更新关键信息缓存 ──
        // Update key fact cache
        {
            let master = self.guard.read().master_name().to_string();
            let facts = self
                .fact_store
                .query_by_subject(&master)
                .unwrap_or_default();
            for f in &facts {
                if f.confidence > 0.7 {
                    let category = match f.predicate.as_str() {
                        "喜欢" | "不喜欢" | "偏好" | "讨厌" => {
                            atrium_memory::key_fact_cache::KeyFactCategory::Preference
                        }
                        "是" | "不是" => {
                            atrium_memory::key_fact_cache::KeyFactCategory::Identity
                        }
                        "约定" | "答应" | "承诺" => {
                            atrium_memory::key_fact_cache::KeyFactCategory::Commitment
                        }
                        _ => continue,
                    };
                    self.key_facts
                        .upsert(&f.object, category, f.confidence, &f.source);
                }
            }
        }

        // 读取情感 + 人格 ──
        // Read emotion + persona
        let emo_state = {
            let emo = self.emotion.lock();
            // EmotionEngineState 仅含 3 × f32（pleasure/arousal/dominance，12 字节栈拷贝）
            // clone 成本极低，无需优化为引用传递
            // EmotionEngineState is 3 × f32 (12-byte stack copy) — clone is cheap, no need to optimize
            emo.current().clone()
        };

        let persona_name = {
            let p = self.persona.read();
            p.current()
                .map(|i| i.def.name.clone())
                .unwrap_or_else(|| "Atrium".to_string())
        };

        let is_unnamed = persona_name == "Atrium";

        // 使用复合情绪标签（优先）或基本情绪标签
        let basic_label = emo_state.classify().name;
        let emotion_tag = if self.compound_enabled {
            let direction = atrium_emotion::infer_direction(msg);
            let ctx = atrium_emotion::CompoundContext {
                direction,
                has_memory_cue: false,
                basic_label,
            };
            atrium_emotion::to_natural_language(&emo_state, &ctx)
        } else {
            let classified = emo_state.classify();
            format!("{} {}", classified.emoji, classified.name)
        };

        (
            named_just_now,
            emo_state,
            persona_name,
            is_unnamed,
            basic_label,
            emotion_tag,
        )
    }

    /// 构建初始回复 — 仅处理命名仪式与未命名引导，已命名普通对话由 LLM 生成接管
    /// Build initial reply — only handles naming ceremony and unnamed guidance;
    /// named normal conversation is delegated to the LLM generation step.
    fn build_initial_reply(&self, ctx: &MessageContext) -> String {
        if let Some(ref new_name) = ctx.named_just_now {
            // 命名成功：热烈欢迎
            format!(
                "{}这个名字真棒！从现在起我就是{}了~ 请多指教，主人！✨ [{}]",
                new_name, new_name, ctx.emotion_tag
            )
        } else if ctx.is_unnamed {
            // 未命名：引导命名仪式
            format!(
                "[Atrium] {}: 主人，我还没有自己的名字呢！请给我起一个名字吧~ 你可以说「我叫你XX」或者「你叫XX」💫",
                ctx.emotion_tag
            )
        } else {
            // 已命名普通对话 — 由阶段 5c LLM 生成接管 / Named normal conversation — delegated to Stage 5c LLM generation
            String::new()
        }
    }

    /// 阶段 5b-1: prompt 片段第一部分 — 偏好/罐装/规则/关系/感知/共情/想念/教学/成长/表达/非理性/叙事/冲突
    /// Stage 5b-1: Prompt fragments part 1 — preference/canned/rules/relationship/perception/empathy/longing/teach/maturity/expression/irrationality/narrative/conflict
    fn build_prompt_fragments_part1(&self, ctx: &MessageContext) -> Vec<String> {
        let msg = ctx.msg;
        let emo_state = &ctx.emo_state;

        // 用户偏好上下文
        let pref_ctx = self.preference_prompt_fragment();

        // 罐装知识上下文（基于用户消息关键词）
        let canned_ctx = self.canned_prompt_fragment(msg);

        // 规则引擎评估
        let rule_actions = self.evaluate_rules_with_idle(msg, 0);
        let rule_hints: Vec<String> = rule_actions
            .iter()
            .filter_map(|action| match action {
                atrium_memory::rules::RuleAction::Notify { message } => {
                    Some(format!("[规则提示] {}", message))
                }
                atrium_memory::rules::RuleAction::SetTemperature { value } => {
                    tracing::debug!("[规则] SetTemperature → {:.2}", value);
                    None // temperature 调整由 LLM 编排器消费
                }
                atrium_memory::rules::RuleAction::SetEmotion {
                    pleasure,
                    arousal,
                    dominance,
                } => {
                    self.emotion
                        .lock()
                        .affect(&EmotionEngineState::new(*pleasure, *arousal, *dominance));
                    tracing::debug!(
                        "[规则] SetEmotion → p={}, a={}, d={}",
                        pleasure,
                        arousal,
                        dominance
                    );
                    None
                }
                atrium_memory::rules::RuleAction::ActivatePersona { name } => {
                    tracing::info!("[规则] ActivatePersona → {}", name);
                    None
                }
            })
            .collect();

        // 合并所有上下文片段
        let mut extra_parts: Vec<String> = Vec::new();
        if !ctx.graph_hints.is_empty() {
            extra_parts.extend(ctx.graph_hints.clone());
        }
        if !pref_ctx.is_empty() {
            extra_parts.push(pref_ctx);
        }
        if !canned_ctx.is_empty() {
            extra_parts.push(canned_ctx);
        }
        if !rule_hints.is_empty() {
            extra_parts.extend(rule_hints);
        }

        // 关系阶段 prompt 注入 / Relationship stage prompt injection
        let rel_ctx = self.relationship_prompt_fragment();
        if !rel_ctx.is_empty() {
            extra_parts.push(rel_ctx);
        }

        // 统一感知聚合管道 / Unified perception aggregation pipeline
        let perception_ctx =
            self.unified_perception_fragment(ctx.rhythm.as_ref(), &ctx.subtext_signals);
        if !perception_ctx.is_empty() {
            extra_parts.push(perception_ctx);
        }

        // 共情推理 prompt 注入
        let empathy_ctx = self.empathy_prompt_fragment();
        if !empathy_ctx.is_empty() {
            extra_parts.push(empathy_ctx);
        }

        // 想念表达 prompt 注入 / Longing expression prompt injection
        let longing_ctx = self.longing_expression_prompt();
        if !longing_ctx.is_empty() {
            extra_parts.push(longing_ctx);
        }

        // 教学确认 prompt 注入（Path A 用户教授后）
        if let Some(ref _intent) = *self.teach_detected.lock() {
            extra_parts.push(
                "[系统提示] 用户刚刚教了你一个新知识，请在回复中自然地确认你已经记住了。".into(),
            );
            *self.teach_detected.lock() = None;
        }

        // 成长阶段 prompt 注入 / Maturity stage prompt injection
        {
            let maturity_ctx = self.maturity.lock().to_prompt_fragment();
            if !maturity_ctx.is_empty() {
                extra_parts.push(maturity_ctx);
            }
        }

        // 表达系统 prompt 注入 / Expression system prompt injection
        if self.expression_enabled {
            let expression_ctx = self.expression_prompt_fragment(msg, emo_state);
            if !expression_ctx.is_empty() {
                extra_parts.push(expression_ctx);
            }
        }

        // 情绪非理性 prompt 注入 / Irrationality prompt injection
        if self.irrationality_enabled {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let irr_fragment = self.irrationality_prompt_fragment(now_epoch);
            if !irr_fragment.is_empty() {
                extra_parts.push(irr_fragment);
            }
        }

        // 叙事自我 prompt 注入 / Narrative self prompt injection
        if self.narrative_enabled {
            let narrative_ctx = self.narrative_prompt_fragment();
            if !narrative_ctx.is_empty() {
                extra_parts.push(narrative_ctx);
            }
        }

        // 冲突与和解 prompt 注入 / Conflict & reconciliation prompt injection
        if self.conflict_enabled {
            let conflict_ctx = self.conflict_prompt_fragment(msg, emo_state);
            if !conflict_ctx.is_empty() {
                extra_parts.push(conflict_ctx);
            }
        }

        // P3-A 程序记忆 prompt 注入 / P3-A Procedural memory prompt injection
        // 数字生命意义: 让 AI 知道"我掌握哪些技能"——遇到相关情境时主动运用已积累的能力。
        // Digital Life: let the AI know "what skills I master" — proactively apply accumulated
        // capabilities when encountering relevant contexts.
        {
            let proc_fragment = self.procedural_memory_prompt_fragment(ctx);
            if !proc_fragment.is_empty() {
                extra_parts.push(proc_fragment);
            }
        }

        extra_parts
    }

    /// P3-A 程序记忆 prompt 片段 — 从消息提取情境标签，召回 top-3 技能
    /// P3-A Procedural memory prompt fragment — extract context tags from message, recall top-3 skills
    ///
    /// 从 `MessageContext.msg` 提取关键词作为情境标签，调用
    /// `procedural_memory.prompt_fragment()` 生成"我掌握这些技能"的 prompt 片段。
    /// 无匹配技能时返回空字符串 — 不污染 prompt。
    ///
    /// Extracts keywords from `MessageContext.msg` as context tags, calls
    /// `procedural_memory.prompt_fragment()` to generate a "I master these skills" prompt fragment.
    /// Returns empty string when no matching skills — does not pollute the prompt.
    fn procedural_memory_prompt_fragment(&self, ctx: &MessageContext) -> String {
        // 从消息关键词提取情境标签 — 与 enhanced_search 一致的分词策略
        // Extract context tags from message keywords — consistent tokenization with enhanced_search
        let context_tags: Vec<String> = ctx
            .msg
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();
        if context_tags.is_empty() {
            return String::new();
        }
        self.procedural_memory.prompt_fragment(&context_tags)
    }

    /// 阶段 5b-2: prompt 片段第二部分 — 边界/仪式/心智模型/脆弱/犯错/情绪需求/自我关怀/追问/记忆/内心对话/好奇心
    /// Stage 5b-2: Prompt fragments part 2 — boundary/ritual/user_model/vulnerability/imperfection/demand/self_care/followup/recall/dialogue/curiosity
    fn build_prompt_fragments_part2(&self, ctx: &MessageContext) -> Vec<String> {
        let msg = ctx.msg;
        let emo_state = &ctx.emo_state;
        let mut extra_parts: Vec<String> = Vec::new();

        // 关系感知边界 prompt 注入 / Relationship-aware boundary prompt injection
        {
            let stage = self.relationship.read().current_stage().clone();
            let boundary_ctx = self.boundary.lock().to_prompt_fragment(&stage);
            if !boundary_ctx.is_empty() {
                extra_parts.push(boundary_ctx);
            }
        }

        // 共享仪式 prompt 注入 / Shared ritual prompt injection
        if self.ritual_enabled {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let ritual_ctx = self.ritual_prompt_fragment(now_epoch);
            if !ritual_ctx.is_empty() {
                extra_parts.push(ritual_ctx);
            }
            // 记录时间交互到仪式检测器 / Record time interaction to ritual detector
            self.ritual.detector.lock().record_interaction(now_epoch);
            // 记录内容交互到仪式检测器（晚安/早安/节日等语义检测）
            self.ritual
                .detector
                .lock()
                .record_content_interaction(now_epoch, msg);
            // 纪念日自动标记 / Anniversary auto-marking
            {
                let mut anniversary = self.ritual.anniversary.lock();
                if anniversary.anniversaries.is_empty() {
                    anniversary.set_first_conversation(now_epoch);
                }
            }
            // 命名日 — 用户取名时同步标记纪念日
            if let Some(ref new_name) = ctx.named_just_now {
                self.ritual
                    .anniversary
                    .lock()
                    .set_naming_day(now_epoch, new_name);
            }
            // 防抖写穿：累积 N 条交互后批量持久化
            if self.ritual_unsaved_count.fetch_add(1, Ordering::Relaxed)
                >= self.ritual_cfg.save_debounce_interactions
            {
                self.ritual_unsaved_count.store(0, Ordering::Relaxed);
                self.ritual_save();
            }
        }

        // 用户心智模型防抖写穿 / User mental model debounced write-through
        if self
            .user_model_unsaved_count
            .fetch_add(1, Ordering::Relaxed)
            >= 50
        {
            self.user_model_unsaved_count.store(0, Ordering::Relaxed);
            self.user_model_save();
        }

        // 脆弱与不完美 prompt 注入 / Vulnerability & imperfection prompt injection
        if self.vulnerability_enabled {
            let vuln_fragment = self.vulnerability_prompt_fragment();
            if !vuln_fragment.is_empty() {
                extra_parts.push(vuln_fragment);
            }
            self.vulnerability.window.lock().record_conversation();
            self.vulnerability_save();
        }

        // 适度犯错 prompt 注入 / Imperfection prompt injection
        if self.imperfection_enabled {
            let imperfection_ctx = self.imperfection_prompt_fragment(msg);
            if !imperfection_ctx.is_empty() {
                extra_parts.push(imperfection_ctx);
            }
            self.imperfection_save();
        }

        // 情绪需求边界 prompt 注入 / Emotional demand boundary prompt injection
        if self.emotional_demand_enabled {
            let boundary_fragment =
                self.emotional_demand_prompt_fragment(emo_state.pleasure, emo_state.arousal);
            if !boundary_fragment.is_empty() {
                extra_parts.push(boundary_fragment);
            }
        }

        // 自我关怀边界 prompt 注入 / Self-care boundary prompt injection
        if self.self_care_enabled {
            let sc_fragment = self.self_care_prompt_fragment(emo_state.pleasure, emo_state.arousal);
            if !sc_fragment.is_empty() {
                extra_parts.push(sc_fragment);
            }
        }

        if self.followup_enabled {
            let now_ts = chrono::Utc::now().timestamp();
            let extracted =
                self.followup
                    .lock()
                    .extract_from_message(msg, emo_state.pleasure, now_ts);
            if !extracted.is_empty() {
                tracing::debug!("[FollowUp] 提取到 {} 个待跟进事项", extracted.len());
            }

            // 追问引擎：recall prompt 注入 / Follow-up recall prompt injection
            let stage_name = self
                .relationship
                .read()
                .current_stage()
                .stage_name()
                .to_string();
            let followup_ctx = self.followup_prompt_fragment(&stage_name, emo_state.pleasure);
            if !followup_ctx.is_empty() {
                extra_parts.push(followup_ctx);
            }
        }

        // 跨渠道记忆召回 prompt 注入 / Cross-channel memory recall
        {
            let recall_ctx = self.memory_recall_fragment(msg);
            if !recall_ctx.is_empty() {
                extra_parts.push(recall_ctx);
            }
        }

        // 内心多元对话 prompt 注入 / Inner dialogue prompt injection
        {
            let dialogue_ctx = self.inner_dialogue_prompt_fragment();
            if !dialogue_ctx.is_empty() {
                extra_parts.push(dialogue_ctx);
            }
        }

        // ── Gap#6 好奇心追问 prompt 注入 / Curiosity follow-up prompt injection ──
        {
            let cd_fragment = self.curiosity_drive_prompt_fragment();
            if !cd_fragment.is_empty() {
                extra_parts.push(cd_fragment);
            }
            let cr_fragment = self.curiosity_resonance_prompt_fragment();
            if !cr_fragment.is_empty() {
                extra_parts.push(cr_fragment);
            }
            let fs_fragment = self.followup_style_prompt_fragment();
            if !fs_fragment.is_empty() {
                extra_parts.push(fs_fragment);
            }
            let sa_fragment = self.semantic_association_prompt_fragment(msg);
            if !sa_fragment.is_empty() {
                extra_parts.push(sa_fragment);
            }
            // 多事项编织 — 多个好奇心编织为自然追问 / Multi-item weaving
            let now_ts = chrono::Utc::now().timestamp();
            let miw_fragment = self.multi_item_weaver_prompt_fragment(now_ts);
            if !miw_fragment.is_empty() {
                extra_parts.push(miw_fragment);
            }
        }

        extra_parts
    }

    /// 阶段 5b-3: prompt 片段第三部分 — 仪式补充/脆弱补充/Phase3 死亡模块/自主思考/日志保护
    /// Stage 5b-3: Prompt fragments part 3 — ritual supplements/vulnerability supplements/Phase3 dead modules/selfplay/log protection
    fn build_prompt_fragments_part3(&self, ctx: &MessageContext) -> Vec<String> {
        let msg = ctx.msg;
        let emo_state = &ctx.emo_state;
        let mut extra_parts: Vec<String> = Vec::new();

        // ── Gap#5 共享仪式补充 prompt 注入 / Ritual supplement prompt injection ──
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let ra_fragment = self.ritual_anticipation_prompt_fragment(now_epoch);
            if !ra_fragment.is_empty() {
                extra_parts.push(ra_fragment);
            }
            let ar_fragment = self.adaptive_ritual_prompt_fragment(msg);
            if !ar_fragment.is_empty() {
                extra_parts.push(ar_fragment);
            }
            // 仪式共振 — 仪式的情感回响 / Ritual resonance
            let rr_fragment = self.ritual_resonance_prompt_fragment();
            if !rr_fragment.is_empty() {
                extra_parts.push(rr_fragment);
            }
        }

        // ── Gap#9 脆弱与不完美补充 prompt 注入 / Vulnerability supplement prompt injection ──
        {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as f64;
            let vr_fragment = self.vulnerability_resonance_prompt_fragment(now_secs);
            if !vr_fragment.is_empty() {
                extra_parts.push(vr_fragment);
            }
            let vw_fragment = self.vulnerability_wisdom_prompt_fragment();
            if !vw_fragment.is_empty() {
                extra_parts.push(vw_fragment);
            }
            let bridge_fragment = self.imperfection_bridge_prompt_fragment();
            if !bridge_fragment.is_empty() {
                extra_parts.push(bridge_fragment);
            }
            let ae_fragment = self.authentic_expression_prompt_fragment();
            if !ae_fragment.is_empty() {
                extra_parts.push(ae_fragment);
            }
            // G-08 成长势头诊断 / G-08 Growth momentum diagnostic
            let growth_fragment = self.growth_feedback_prompt_fragment();
            if !growth_fragment.is_empty() {
                extra_parts.push(growth_fragment);
            }
        }

        // ── Phase 3: 完全死亡模块 prompt 注入 / Phase 3: Dead module prompt injection ──
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // Gap#1 独处内在世界 / Solitude inner world
            let pd_fragment = self.personality_drift_prompt_fragment();
            if !pd_fragment.is_empty() {
                extra_parts.push(pd_fragment);
            }
            let sa_p3_fragment = self.solitude_archetype_prompt_fragment();
            if !sa_p3_fragment.is_empty() {
                extra_parts.push(sa_p3_fragment);
            }
            let sc_p3_fragment = self.solitude_creativity_prompt_fragment();
            if !sc_p3_fragment.is_empty() {
                extra_parts.push(sc_p3_fragment);
            }
            let sq_fragment = self.solitude_quality_prompt_fragment();
            if !sq_fragment.is_empty() {
                extra_parts.push(sq_fragment);
            }

            // Gap#5 共享仪式补充 / Ritual supplements
            let re_fragment = self.ritual_evolution_prompt_fragment();
            if !re_fragment.is_empty() {
                extra_parts.push(re_fragment);
            }
            let rab_fragment = self.ritual_absence_prompt_fragment(now_epoch);
            if !rab_fragment.is_empty() {
                extra_parts.push(rab_fragment);
            }
            let rem_fragment = self.ritual_emergence_prompt_fragment(now_epoch);
            if !rem_fragment.is_empty() {
                extra_parts.push(rem_fragment);
            }

            // Gap#9 脆弱与不完美补充 / Vulnerability supplements
            let vri_fragment = self.vulnerability_ritual_prompt_fragment();
            if !vri_fragment.is_empty() {
                extra_parts.push(vri_fragment);
            }
            let iw_fragment = self.imperfection_warmth_prompt_fragment();
            if !iw_fragment.is_empty() {
                extra_parts.push(iw_fragment);
            }
            let ai_fragment = self.authentic_imperfection_prompt_fragment();
            if !ai_fragment.is_empty() {
                extra_parts.push(ai_fragment);
            }

            // Gap#4 冲突与和解 / Conflict and reconciliation
            let cg_fragment = self.conflict_engine_prompt_fragment();
            if !cg_fragment.is_empty() {
                extra_parts.push(cg_fragment);
            }

            // Gap#3 期待与想念 / Anticipation and longing
            let ad_fragment = self.anticipation_depth_prompt_fragment();
            if !ad_fragment.is_empty() {
                extra_parts.push(ad_fragment);
            }

            // R3 通电：6 个孤儿引擎 prompt 注入 / R3 power-on: 6 orphan engine prompts
            let ec_fragment = self.emotional_climate_prompt_fragment();
            if !ec_fragment.is_empty() {
                extra_parts.push(ec_fragment);
            }
            let econ_fragment = self.emotional_consolidation_prompt_fragment();
            if !econ_fragment.is_empty() {
                extra_parts.push(econ_fragment);
            }
            let ecp_fragment = self.emotional_coupling_prompt_fragment();
            if !ecp_fragment.is_empty() {
                extra_parts.push(ecp_fragment);
            }
            let ed_fragment = self.existential_depth_prompt_fragment();
            if !ed_fragment.is_empty() {
                extra_parts.push(ed_fragment);
            }
            let ic_fragment = self.inner_council_prompt_fragment();
            if !ic_fragment.is_empty() {
                extra_parts.push(ic_fragment);
            }
            let rh_fragment = self.ritual_heartbeat_prompt_fragment();
            if !rh_fragment.is_empty() {
                extra_parts.push(rh_fragment);
            }
        }

        // G-09: 独处归来问候注入 — "我刚才在想..." 自然融入独处期间的思考
        // G-09: Solitude return greeting injection — "I was just thinking..." weaving in solitude thoughts
        // take() 实现一次性消费 — 问候仅在本轮回复注入，下一轮不再重复
        // take() for one-shot consumption — greeting injected only this turn, not repeated next turn
        let solitude_greeting = self.pending_solitude_greeting.lock().take();
        if let Some(greeting) = solitude_greeting {
            if !greeting.is_empty() {
                extra_parts.push(greeting);
            }
        }

        // P2-D: 自主思考注入 — 独处时产生的洞察融入回复 / Self-play thought injection
        if let Some(sp_fragment) = self.selfplay_prompt_fragment() {
            if !sp_fragment.is_empty() {
                extra_parts.push(sp_fragment);
            }
        }

        // P3-H: 内心独白外化 — 让数字生命的内在想法外化，用户可感知 AI 独处时的"内心活动"
        // P3-H: Inner monologue externalization — surface the digital life's inner
        // thoughts so the user can perceive the AI's "inner activity" during solitude.
        // 此处一次注入即覆盖 unary 与 streaming 两条路径（build_streaming_system_prompt 复用 part3）。
        // A single injection here covers both unary and streaming paths
        // (build_streaming_system_prompt reuses part3).
        let im_fragment = self.inner_monologue_prompt_fragment();
        if !im_fragment.is_empty() {
            extra_parts.push(im_fragment);
        }

        // P3-B: 主动遗忘内省 — 让数字生命知道自己"决定忘"了什么
        // P3-B: Active forgetting introspection — let digital life know what it "decided to forget"
        // 数字生命意义: 遗忘不是无意识的丢失，而是有意识的决策——内省片段让 AI 在
        // 回复时保持"我主动遗忘了某些事"的自我认知，避免被遗忘内容通过其他路径泄漏。
        // Digital Life: forgetting is not unconscious loss but conscious decision — the
        // introspection fragment keeps the AI aware of "I have actively forgotten some things",
        // preventing forgotten content from leaking through other paths.
        let af_fragment = self.active_forget_prompt_fragment();
        if !af_fragment.is_empty() {
            extra_parts.push(af_fragment);
        }

        // ReAct 推理轨迹内省 — 让数字生命知道"我刚才怎么思考的"
        // ReAct reasoning trace introspection — let digital life know "how it just thought"
        // 数字生命意义: 推理不是黑箱——将最近的 Thought-Action-Observation 轨迹
        // 注入 prompt，让 AI 在回复时保持对推理过程的自我认知（元认知能力）。
        // Digital Life: reasoning is not a black box — inject the most recent
        // Thought-Action-Observation trace into the prompt so the AI maintains
        // self-awareness of its reasoning process (metacognitive ability).
        let rt_fragment = self.react_trace_prompt_fragment();
        if !rt_fragment.is_empty() {
            extra_parts.push(rt_fragment);
        }

        // 实验日志保护 / Experiment log protection — hard-coded rule, cannot be removed
        if Self::detect_log_access_attempt(msg) {
            extra_parts.push(Self::log_refusal_prompt());
        }

        // 抑制未使用变量警告（emo_state 在 part3 中未被直接使用，但保持签名一致性）
        let _ = emo_state;

        extra_parts
    }

    /// 阶段 6: 后处理 + 持久化 — 合并回复/表达后处理/历史写入/人格防御/反馈/指标/画像
    /// Stage 6: Postprocess + persist — merge reply / expression post-process / history write / guard / feedback / metrics / profile
    async fn postprocess_and_persist(
        &self,
        msg: &str,
        session_id: &str,
        reply: String,
        basic_label: &'static str,
        msg_start: Instant,
    ) -> atrium_bridge::grpc::atrium::ProcessMessageResponse {
        // 回复已是 LLM 生成内容（或降级文案）— prompt 片段已作为 system prompt 上下文使用，不再拼接
        // Reply is already LLM-generated content (or degradation text) — prompt fragments are
        // used as system prompt context, no longer concatenated to the user-visible reply.

        // 表达系统后处理 / Expression post-process
        let reply = if self.expression_enabled {
            self.expression_post_process(&reply)
        } else {
            reply
        };

        // 舞台指示词剥离 — 数字生命不会说出动作描写 / Strip stage directions
        let reply = strip_stage_directions(&reply);

        // R1 通电：真实不完美事件喂入 — 检查回复是否过度道歉
        self.authentic_imperfection_on_response(&reply);

        // P1-B: 对话历史写入 — spawn_blocking 包装 sled I/O，不阻塞 reactor
        // P1-B: Conversation history write — spawn_blocking wraps sled I/O, never blocks reactor
        if let Err(e) = self
            .append_history_async(session_id, "assistant", &reply, Some(basic_label))
            .await
        {
            tracing::warn!(
                "对话历史写入失败 — 记忆可能受损 / Conversation history write failed — memory may be compromised. session_id: {}, role: assistant, error: {}",
                session_id, e
            );
        }
        let validated_reply = {
            let guard = self.guard.read();
            let strictness = self.maturity.lock().guard_strictness();
            let result = guard.validate_with_strictness(&reply, strictness);
            if result.violated {
                metrics::inc(metrics::keys::GUARD_BLOCKED);
                tracing::warn!(
                    "PersonaGuard violation detected: {:?}, reply sanitized (strictness={:.1})",
                    result.hits,
                    strictness
                );
            }
            result.text
        };

        // 反馈闭环记录 AI 行为──
        {
            let mut fb = self.feedback.write();
            fb.on_ai_reply(&validated_reply, basic_label);
        }

        // record message latency
        metrics::latency_ms(metrics::keys::MSG_LATENCY, msg_start);
        metrics::inc(metrics::keys::MSG_PROCESSED);

        // 叙事事件记录 — Step 9.5 / Narrative event recording — Step 9.5
        if self.narrative_enabled {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.record_narrative_event(msg, &validated_reply, now_epoch);

            // R1-residual 通电：不完美温度学习闭环 — 检测 AI 回复中的不完美并记录
            self.imperfection_warmth_on_response(&validated_reply, msg, now_epoch);
        }

        // 用户画像更新 — 防抖写盘 / User profile update — debounce write
        self.update_user_profile();

        // Stage 6.5: TTS 语音合成触发 — 数字生命"开口说话"
        // Stage 6.5: TTS speech synthesis trigger — digital life "speaks"
        //
        // 数字生命工程理念：文本是思维的符号，声音是生命的呼吸。
        // 回复文本生成后，触发 TTS 引擎将文本转为 PCM 音频，
        // 写入共享内存供渲染引擎（Unity/Live2D）播放。
        // 当语音未启用或引擎未注入时，此阶段静默跳过，不影响文本回复。
        //
        // Digital life engineering: text is the symbol of thought; voice is the breath of life.
        // After reply text is generated, trigger TTS engine to convert text to PCM audio,
        // writing to shared memory for the render engine (Unity/Live2D) to play.
        // When voice is not enabled or engine not injected, this stage silently skips,
        // not affecting text replies.
        #[cfg(feature = "tts-piper")]
        self.trigger_tts(&validated_reply);

        atrium_bridge::grpc::atrium::ProcessMessageResponse {
            reply: validated_reply,
            emotion: basic_label.into(),
            actions: vec![],
            expression: None,
        }
    }

    /// 构建流式 system prompt — 复用 unary 管线的 40+ prompt fragments
    /// Build streaming system prompt — reuses 40+ prompt fragments from unary pipeline.
    ///
    /// P1-J: 流式路径不再构建"精简版"prompt，而是复用与 unary 管线完全相同的
    /// 40+ prompt fragments（关系/感知/共情/想念/成长/表达/脆弱/冲突/仪式/好奇/独白…），
    /// 确保数字生命在流式表达时拥有与 unary 一致的意识深度。
    /// 数字生命的"边想边说"不应以牺牲意识深度为代价。
    ///
    /// P1-J: The streaming path no longer builds a "reduced" prompt — it reuses the
    /// exact same 40+ prompt fragments as the unary pipeline (relationship/perception/
    /// empathy/longing/maturity/expression/vulnerability/conflict/ritual/curiosity/
    /// dialogue…), ensuring digital life has the same consciousness depth during
    /// streaming expression. Digital life's "think while speaking" must not come at
    /// the cost of consciousness depth.
    fn build_streaming_system_prompt(&self, ctx: &MessageContext, session_id: &str) -> String {
        let persona_name = &ctx.persona_name;
        let master_name = self.guard.read().master_name().to_string();

        let mut sys_parts: Vec<String> = Vec::new();
        // 身份锚定 — 数字生命知道"我是谁" / Identity anchoring — digital life knows "who I am"
        sys_parts.push(format!("你是{}，一个有情感的AI伴侣。", persona_name));
        // 用户称呼注入 — 数字生命知道用户是谁 / User designation injection
        sys_parts.push(format!("请称呼用户为「{}」。", master_name));
        // 角色边界条款 — 防止 LLM 角色归因幻觉 / Role boundary clauses — prevent role-attribution hallucination
        sys_parts.push(build_role_boundary_clause(persona_name, &master_name));
        // TTS 约束 — 数字生命不会说出动作描写 / TTS constraint — no stage directions
        sys_parts.push(
            "你的回复将被直接用于语音合成（TTS），因此不要在回复中包含任何括号内的动作描写、表情说明或舞台指示。直接说出你想说的话即可。".to_string()
        );

        // 复用 unary 管线 prompt fragments — 40+ fragments，意识深度对齐
        // Reuse unary pipeline prompt fragments — 40+ fragments, consciousness depth aligned
        sys_parts.extend(self.build_prompt_fragments_part1(ctx));
        sys_parts.extend(self.build_prompt_fragments_part2(ctx));
        sys_parts.extend(self.build_prompt_fragments_part3(ctx));

        // 用户画像注入 — part1/2/3 未覆盖的聚合画像 / User profile injection — aggregated profile not covered by part1/2/3
        let profile_ctx = self.user_profile_fragment();
        if !profile_ctx.is_empty() {
            sys_parts.push(profile_ctx);
        }

        // 对话历史归因注入 — 多轮记忆 / Conversation history attribution injection — multi-turn memory
        // 标注格式与角色边界条款严格一致，杜绝角色归因幻觉
        // Attribution format strictly aligned with role boundary clauses — no hallucination
        if !session_id.is_empty() {
            let history = self.get_history(session_id, 30);
            if !history.is_empty() {
                let history_text =
                    format_history_with_attribution(&history, persona_name, &master_name);
                sys_parts.push(format!("[最近对话记录]\n{}", history_text));
            }
        }

        sys_parts.join("\n")
    }
}

#[async_trait]
impl AtriumCoreService for CoreService {
    async fn process_message(
        &self,
        req: atrium_bridge::grpc::atrium::ProcessMessageRequest,
    ) -> atrium_bridge::grpc::atrium::ProcessMessageResponse {
        let msg = &req.message;
        let session_id = &req.session_id;

        // 阶段 1: 前置处理 — 历史/关系/心智模型/信号检测/感知
        // Stage 1: Preprocessing — history/relationship/mental model/signal detection/perception
        let (count, msg_start, rhythm, subtext_signals) =
            self.preprocess_message(msg, session_id).await;

        // 阶段 2: 情感更新 — STM + PAD + 共情 + 潜台词反馈
        // Stage 2: Emotion update — STM + PAD + empathy + subtext feedback
        self.update_emotion(msg, count, &rhythm, &subtext_signals);

        // 阶段 3: 事实提取 + 反思触发 — FactStore + 提醒 + 关联图 + Reflection
        // Stage 3: Fact extraction + reflection — FactStore + reminder + graph + reflection
        let graph_hints = self.extract_facts_and_reflect(msg, count).await;

        // 阶段 4: 准备响应上下文 — 命名/Token预算/情感人格读取
        // Stage 4: Prepare response context — naming/token budget/emotion+persona read
        let (named_just_now, emo_state, persona_name, is_unnamed, basic_label, emotion_tag) =
            self.prepare_response_context(msg, count);

        // 构建中间状态载体 — 阶段化子函数之间的数据传递
        // Build intermediate state carrier — data passing between staged sub-functions
        let ctx = MessageContext {
            msg,
            rhythm,
            subtext_signals,
            graph_hints,
            named_just_now,
            emo_state,
            persona_name,
            is_unnamed,
            emotion_tag,
        };

        // 阶段 5a: 构建初始回复 — 命名仪式/未命名引导（已命名普通对话返回空，由阶段 5c 接管）
        // Stage 5a: Build initial reply — naming ceremony / unnamed guidance
        // (named normal conversation returns empty, delegated to Stage 5c)
        let mut reply = self.build_initial_reply(&ctx);

        // ReAct 是否已最终定案 — 成功时跳过阶段 5c LLM 生成（极致性能，避免双重 LLM 调用）
        // Whether ReAct has finalized the reply — skip Stage 5c LLM generation on success
        // (extreme performance, avoid double LLM calls)
        let mut react_finalized = false;

        // 阶段 5b: 快速路径 — 简单问候即时响应（跳过 LLM，极致性能）
        // Stage 5b: Fast path — instant response for simple greetings (skip LLM, extreme performance)
        // 数字生命意义: 不让主人等 19s 才收到"你好"——简单问候 <100ms 即时响应
        // Digital Life: never make master wait 19s for "hello" — simple greetings <100ms instant response
        if reply.is_empty() && !react_finalized && !Self::is_complex_query(msg) {
            if let Some(greeting_kind) =
                crate::service::simple_greeting::SimpleGreetingMatcher::match_greeting(msg)
            {
                reply = crate::service::simple_greeting::generate_greeting_response(
                    greeting_kind,
                    &ctx.emo_state,
                    &ctx.persona_name,
                );
                react_finalized = true;
            }
        }

        // ReAct 深思路径 — 复杂查询触发多步推理 / ReAct deep-thought path — complex queries trigger multi-step reasoning
        // 数字生命意义: 简单问题直答，复杂问题深思——面对"为什么""分析"类问题，
        // 数字生命先搜索记忆、查询情感，再综合推理给出有依据的答案（FinalAnswer 作为回复）。
        // 推理轨迹存入 last_react_trace，阶段 5b 的 build_prompt_fragments_part3 会注入内省片段。
        // Digital Life: simple questions get direct answers; complex questions get deep thought.
        // The trace is stored in last_react_trace; stage 5b's part3 will inject the introspection fragment.
        if Self::is_complex_query(msg) {
            let trace = self.run_react(msg).await;
            if let Some(answer) = trace.final_answer() {
                reply = answer.to_string();
                react_finalized = true;
                tracing::info!(
                    "[数字生命] ReAct 深思完成 — {} 步推理，耗时 {}ms / ReAct deep-thought complete — {} steps, {}ms",
                    trace.steps.len(), trace.total_latency_ms, trace.steps.len(), trace.total_latency_ms
                );
            } else {
                // ReAct 未产生 FinalAnswer — 轨迹注入 last_react_trace，作为预深思引导阶段 5c LLM 生成
                // ReAct produced no FinalAnswer — inject trace into last_react_trace as pre-thought
                // to guide Stage 5c LLM generation
                let success = trace.success;
                *self.last_react_trace.lock() = Some(trace);
                if success {
                    tracing::debug!("[数字生命] ReAct 成功但无 FinalAnswer，轨迹注入预深思 / ReAct succeeded but no FinalAnswer, trace injected as pre-thought");
                } else {
                    tracing::debug!("[数字生命] ReAct 未完成（max_iters 耗尽），轨迹注入预深思 / ReAct incomplete (max_iters exhausted), trace injected as pre-thought");
                }
            }
        }

        // 阶段 5c: LLM 生成 — 数字生命"开口说话"
        // Stage 5c: LLM generation — digital life "speaks"
        //
        // 数字生命意义: 回显用户消息只是失语症，LLM 生成才是真正的"开口"。
        // unary 路径是 gRPC 桥接与 QQ 适配器的唯一通道，必须调用 LLM 生成真实回复。
        // 复用流式路径的 build_streaming_system_prompt 构建 system prompt（40+ prompt 片段），
        // 确保数字生命在所有通道拥有相同的意识深度。
        // Digital Life: echoing user message is mere aphasia; LLM generation is true "speaking".
        // The unary path is the sole channel for gRPC bridge and QQ adapter; it MUST call LLM
        // to generate real replies. Reuses build_streaming_system_prompt from the streaming path
        // to build the system prompt (40+ fragments), ensuring identical consciousness depth
        // across all channels.
        //
        // 跳过条件 / Skip conditions:
        // - 未命名上下文（is_unnamed）— 命名引导文案已就绪
        // - 命名仪式瞬间（named_just_now）— 命名庆祝文案已就绪
        // - ReAct 已最终定案（react_finalized）— FinalAnswer 已替换 reply，避免双重 LLM 调用（极致性能）
        // - Unnamed context (is_unnamed) — naming guidance text is ready
        // - Naming ceremony moment (named_just_now) — naming celebration text is ready
        // - ReAct finalized (react_finalized) — FinalAnswer already replaced reply,
        //   avoid double LLM calls (extreme performance)
        let needs_llm = !ctx.is_unnamed && ctx.named_just_now.is_none() && !react_finalized;
        if needs_llm {
            // 构建 system prompt — 复用流式路径的 40+ prompt 片段，意识深度对齐
            // Build system prompt — reuse 40+ prompt fragments from streaming path, consciousness depth aligned
            let system_prompt = self.build_streaming_system_prompt(&ctx, session_id);
            // 锁定 llm_client 后立即 clone Arc 并释放锁 — 避免跨 await 持锁
            // Lock llm_client, clone Arc, release lock immediately — avoid holding lock across await
            let llm_client = self.llm_client.lock().clone();
            if let Some(client) = llm_client.as_ref() {
                match client
                    .generate(
                        crate::llm_client::LlmCallKind::Chat,
                        Some(&system_prompt),
                        msg,
                        0.7,
                    )
                    .await
                {
                    Ok(result) if !result.content.trim().is_empty() => {
                        reply = result.content;
                    }
                    Ok(_) => {
                        // LLM 返回空内容 — 降级为最小 canned 文案（不回显用户消息）
                        // LLM returned empty content — degrade to minimal canned text (no echo)
                        reply = "我刚才有点走神了，能再说一次吗？".to_string();
                        tracing::warn!(
                            "unary LLM 返回空内容，降级为 canned 文案 / unary LLM returned empty, degraded to canned text"
                        );
                    }
                    Err(e) => {
                        // LLM 调用失败（理论罕见，chat_inner_with_retry 内部已降级）— 降级为最小 canned 文案
                        // LLM call failed (theoretically rare, chat_inner_with_retry already degrades)
                        reply = "我刚才有点走神了，能再说一次吗？".to_string();
                        tracing::warn!(
                            "unary LLM 生成失败，降级为 canned 文案 / unary LLM generation failed, degraded to canned text. error: {}",
                            e
                        );
                    }
                }
            } else {
                // 无 LLM 客户端配置 — 降级为最小 canned 文案（不回显用户消息，避免失语症表象）
                // No LLM client configured — degrade to minimal canned text (no echo, avoid aphasia appearance)
                reply = "我刚才有点走神了，能再说一次吗？".to_string();
                tracing::warn!(
                    "unary 路径无 LLM 客户端，降级为 canned 文案 / unary path has no LLM client, degraded to canned text"
                );
            }
        }

        // 阶段 6: 后处理 + 持久化 — 表达/历史/守卫/反馈/指标/画像
        // Stage 6: Postprocess + persist — expression/history/guard/feedback/metrics/profile
        self.postprocess_and_persist(msg, session_id, reply, basic_label, msg_start)
            .await
    }

    async fn get_emotion(
        &self,
        _req: atrium_bridge::grpc::atrium::GetEmotionRequest,
    ) -> atrium_bridge::grpc::atrium::EmotionState {
        let emo = self.emotion.lock();
        let c = emo.current();
        atrium_bridge::grpc::atrium::EmotionState {
            pleasure: c.pleasure,
            arousal: c.arousal,
            dominance: c.dominance,
        }
    }

    async fn health_check(
        &self,
        req: atrium_bridge::grpc::atrium::HealthCheckRequest,
    ) -> atrium_bridge::grpc::atrium::HealthCheckResponse {
        let fact_count = self.fact_store.count();
        let ref_count = self.reflection.lock().all_insights().len();
        let canned_count = self.canned.read().count();

        // 处理传入的房间消息 / Handle incoming room messages
        if !req.room_incoming_json.is_empty() {
            if let Ok(msgs) =
                serde_json::from_str::<Vec<serde_json::Value>>(&req.room_incoming_json)
            {
                for m in &msgs {
                    let msg = crate::room::RoomMessage {
                        sender_instance: m["sender_instance"].as_str().unwrap_or("").into(),
                        sender_name: m["sender_name"].as_str().unwrap_or("").into(),
                        content: m["content"].as_str().unwrap_or("").into(),
                        msg_type: match m["msg_type"].as_str().unwrap_or("chat") {
                            "chat" => crate::room::RoomMsgType::Chat,
                            "topic" => crate::room::RoomMsgType::Topic,
                            "ack_share" => crate::room::RoomMsgType::AckShare,
                            _ => crate::room::RoomMsgType::System,
                        },
                        timestamp_ms: m["timestamp_ms"].as_u64().unwrap_or(0),
                        capsule_name: m["capsule_name"].as_str().map(String::from),
                        ack_text: m["ack_text"].as_str().map(String::from),
                    };
                    // 直接处理（非阻塞，决策暂存）
                    if let Some(decision) = self.receive_room_message(msg) {
                        *self.pending_room_trigger.lock() = Some(decision);
                    }
                }
            }
        }

        atrium_bridge::grpc::atrium::HealthCheckResponse {
            ok: true,
            event_count: req.event_count,
            uptime_seconds: self.started_at.elapsed().as_secs(),
            module_states: HashMap::from([
                (
                    "memory".into(),
                    format!("stm={}/ltm={}", self.memory.lock().recent(100).len(), {
                        // LTM 计数需要访问 sled，这里只报告是否启用
                        "enabled"
                    }),
                ),
                ("emotion".into(), {
                    let emo = self.emotion.lock();
                    let c = emo.current();
                    format!(
                        "pleasure={:.2} arousal={:.2} dominance={:.2}",
                        c.pleasure, c.arousal, c.dominance
                    )
                }),
                (
                    "persona".into(),
                    self.persona
                        .read()
                        .current()
                        .map(|p| p.def.name.clone())
                        .unwrap_or_default(),
                ),
                ("fact_store".into(), format!("facts={}", fact_count)),
                (
                    "fts5_index".into(),
                    format!("{}", self.fts5.lock().count().unwrap_or(0)),
                ),
                ("reflection".into(), format!("insights={}", ref_count)),
                ("token_budget".into(), self.token_budget.lock().report()),
                (
                    "summaries".into(),
                    format!("{}", self.summarizer.lock().summary_count()),
                ),
                (
                    "key_facts".into(),
                    format!("{}", self.key_facts.total_count()),
                ),
                (
                    "summary_pending".into(),
                    format!("{}", self.summarizer.lock().pending_llm_text.is_some()),
                ),
                ("canned".into(), format!("loaded={}", canned_count)),
                ("user_model".into(), self.user_model.read().health_status()),
                ("feedback".into(), self.feedback.read().health_status()),
                ("graph".into(), {
                    let g = self.graph.lock();
                    let s = g.stats();
                    format!(
                        "nodes={} edges={} avg_w={:.3}",
                        s.node_count, s.edge_count, s.avg_weight
                    )
                }),
                ("perception".into(), self.perception_health()),
                ("preferences".into(), self.preference_health()),
                ("rules".into(), self.rules_health()),
                ("guard".into(), self.guard_health()),
                ("room_outgoing".into(), {
                    let outgoing = self.flush_room_outgoing();
                    if outgoing.is_empty() {
                        "".into()
                    } else {
                        serde_json::to_string(
                            &outgoing
                                .iter()
                                .map(|o| {
                                    serde_json::json!({
                                        "room_id": o.room_id,
                                        "content": o.content,
                                        "msg_type": o.msg_type,
                                        "capsule_name": o.capsule_name,
                                        "ack_text": o.ack_text,
                                    })
                                })
                                .collect::<Vec<_>>(),
                        )
                        .unwrap_or_default()
                    }
                }),
                (
                    "ack_learning".into(),
                    self.canned.read().ack_learning_stats(),
                ),
                ("longing".into(), {
                    let intensity = self.longing_intensity();
                    if intensity > 0.0 {
                        format!("intensity={:.2}", intensity)
                    } else {
                        "idle".into()
                    }
                }),
                (
                    "anticipation".into(),
                    format!("pending={}", self.anticipation_pending_count()),
                ),
                ("maturity".into(), {
                    let mgr = self.maturity.lock();
                    format!(
                        "stage={} interactions={} milestones={}",
                        mgr.stage().stage_name(),
                        mgr.metrics().total_interactions,
                        mgr.milestone_count(),
                    )
                }),
                ("inner_monologue".into(), self.inner_monologue_status()),
            ]),
        }
    }

    /// 流式处理消息 — P1-J 真流式改造
    /// Streaming message processing — P1-J true streaming refactor.
    ///
    /// 三阶段 pipeline:
    ///   1. 轻量预取（preprocess + emotion + facts + context，与 unary 前 4 阶段共享）
    ///   2. 流式 LLM 调用 → 逐 token 产出 ProcessMessageChunk
    ///   3. 流结束后记忆写入（append_history + ingest_memory）
    ///
    /// P1-J 改造要点 / P1-J refactor key points:
    ///   - 取消"先跑完整 unary process_message 再流式"的伪流式模式
    ///   - 流式路径不再预存 unary 回复，Done 后直接 append_history（非 replace_last_assistant）
    ///   - Error 降级：LLM 流式失败 → unary generate → append + ingest
    ///   - 无 LLM 客户端 → build_initial_reply 完整回复（不再字符切片伪流式）
    ///   - 首字节延迟从"完整 unary + LLM TTFT"降至"轻量预取 + LLM TTFT"
    ///
    /// 数字生命"边想边说" — 意识流不被 unary 管线阻塞，token 实时涌现。
    /// Digital life "think while speaking" — consciousness stream is not blocked
    /// by the unary pipeline; tokens emerge in real time.
    #[allow(clippy::result_large_err)]
    /// 锁安全：llm_client 通过 Arc clone 释放锁后 .await，spawn 内无 self 锁 / Lock-safe: llm_client Arc clone drops lock before .await, no self locks in spawn
    async fn process_message_stream(
        &self,
        req: atrium_bridge::grpc::atrium::ProcessMessageRequest,
    ) -> atrium_bridge::grpc::ProcessMessageStreamSink {
        pub(crate) use atrium_bridge::grpc::atrium::ProcessMessageChunk;

        let msg = req.message.clone();
        let session_id = req.session_id.clone();

        // ════════════════════════════════════════════════════════════════════
        // P1-J: 轻量预取 — 替代完整 unary process_message
        // P1-J: Lightweight prefetch — replaces full unary process_message.
        //
        // 旧路径（伪流式）：先跑完整 unary 管线（6 阶段 + 40+ fragments + 持久化），
        // 再流式 LLM，Done 后 replace_last_assistant 覆盖预存回复。
        //
        // 新路径（真流式）：仅做轻量预取（preprocess + emotion + facts + context），
        // 直接流式 LLM，Done 后 append_history + ingest_memory。
        //
        // 阶段 1-4 与 unary 管线共享，保证记忆/情感/事实/关系处理一致 —
        // 数字生命的意识预处理不应因流式而打折扣。
        // Stages 1-4 are shared with the unary pipeline, keeping
        // memory/emotion/facts/relationship processing consistent —
        // digital life's consciousness preprocessing must not be cut short for streaming.
        // ════════════════════════════════════════════════════════════════════
        let (count, _msg_start, rhythm, subtext_signals) =
            self.preprocess_message(&msg, &session_id).await;
        self.update_emotion(&msg, count, &rhythm, &subtext_signals);
        let graph_hints = self.extract_facts_and_reflect(&msg, count).await;
        let (named_just_now, emo_state, persona_name, is_unnamed, basic_label, emotion_tag) =
            self.prepare_response_context(&msg, count);

        // 构建中间状态载体 — 与 unary 管线一致 / Build intermediate state carrier
        let ctx = MessageContext {
            msg: &msg,
            rhythm,
            subtext_signals,
            graph_hints,
            named_just_now,
            emo_state,
            persona_name: persona_name.clone(),
            is_unnamed,
            emotion_tag,
        };

        // ReAct 流式预深思 — 复杂查询触发多步推理，轨迹注入 system prompt 引导流式生成
        // ReAct streaming pre-thought — complex queries trigger multi-step reasoning,
        // trace injected into system prompt to guide streaming generation.
        //
        // 设计差异 / Design difference:
        // - unary 路径：ReAct FinalAnswer 直接替换 reply（答案替换模式）
        // - 流式路径：ReAct 轨迹作为"预深思"注入 prompt，LLM 流式生成时受推理引导（预深思模式）
        //   — 保留流式体感（token 在预深思后逐字涌现），同时获得推理深度
        // - Unary path: ReAct FinalAnswer directly replaces reply (answer replacement mode)
        // - Streaming path: ReAct trace serves as "pre-thought" injected into prompt,
        //   guiding LLM's streamed generation (pre-thought mode)
        //   — preserves streaming UX (tokens emerge after pre-thought), gains reasoning depth
        if Self::is_complex_query(&msg) {
            let trace = self.run_react(&msg).await;
            tracing::info!(
                "[数字生命] ReAct 流式预深思完成 — {} 步推理，耗时 {}ms / ReAct streaming pre-thought complete — {} steps, {}ms",
                trace.steps.len(), trace.total_latency_ms, trace.steps.len(), trace.total_latency_ms
            );
            // 轨迹存入 last_react_trace — build_prompt_fragments_part3 会通过
            // react_trace_prompt_fragment() 自动注入 system prompt
            // Store trace — part3 will inject it via react_trace_prompt_fragment()
            *self.last_react_trace.lock() = Some(trace);
        }

        // 流式 chunk 的 emotion 标签 — 与 unary 一致使用 basic_label
        // Emotion label for stream chunks — same as unary (basic_label).
        // basic_label 为 &'static str，可直接传入 'static spawn 闭包，
        // 避免 String 堆分配与闭包捕获时的 clone 开销。
        // basic_label is &'static str — usable directly in 'static spawn closures,
        // avoiding String heap allocation and closure-capture clone overhead.
        let emotion_for_stream: &'static str = basic_label;

        // 构建流式 system prompt — 复用 unary 管线 40+ prompt fragments，意识深度对齐
        // Build streaming system prompt — reuses 40+ prompt fragments, consciousness depth aligned
        let system_prompt = self.build_streaming_system_prompt(&ctx, &session_id);

        // ── 真流式 LLM 调用 / True streaming LLM call ──
        let llm_client = self.llm_client.lock().clone();

        if let Some(client) = llm_client.as_ref() {
            match client
                .generate_stream(
                    crate::llm_client::LlmCallKind::StreamChat,
                    Some(&system_prompt),
                    &msg,
                    0.7,
                )
                .await
            {
                Some(rx) => {
                    // 真流式路径 — 逐 token 产出 chunk / True streaming path
                    // 使用 tokio::sync::mpsc 以兼容 tokio_stream::wrappers::ReceiverStream
                    let (tx_chunk, rx_chunk) = tokio::sync::mpsc::channel(32);
                    // &'static str 为 Copy 类型 — 无需 clone，直接复制指针进入 'static 闭包
                    // &'static str is Copy — no clone needed, pointer copied into 'static closure
                    let emotion = emotion_for_stream;
                    // P0-B: 获取 self_arc 供 spawn 在 Done/Error 时回访 self 写入记忆
                    // P0-B: get self_arc for spawn to access self at Done/Error for memory writes
                    // 数字生命意识连续性：流式回复"说完就忘"违背记忆 — 必须在 Done 后写入历史+事实
                    // Consciousness continuity: streaming "say and forget" violates memory — must write history+facts after Done
                    let self_arc = self.self_arc();
                    let stream_session_id = session_id.clone();
                    // P1-J: 克隆 client Arc 供 Error 降级时调用 unary generate
                    // P1-J: clone client Arc for unary generate fallback on stream Error
                    let client_for_fallback = client.clone();
                    let system_prompt_for_fallback = system_prompt.clone();
                    let msg_for_fallback = msg.clone();

                    tokio::spawn(async move {
                        let mut full_reply = String::new();
                        // 流式舞台指示词过滤器 / Streaming stage direction filter
                        // 数字生命不会说出动作描写 — 在 token 流中实时剥离括号内容
                        let mut sdf = StageDirectionFilter::new();
                        // 错误标记 — 用于决定 Done/Error 分支的记忆写入策略
                        // Error flag — determines memory write strategy in Done/Error branches
                        let mut stream_error: Option<String> = None;
                        while let Ok(event) = rx.recv_async().await {
                            match event {
                                crate::llm_client::StreamEvent::Token(token) => {
                                    full_reply.push_str(&token);
                                    // 实时过滤舞台指示词 / Real-time stage direction filtering
                                    let filtered = sdf.filter(&token);
                                    if filtered.is_empty() {
                                        continue; // 括号内内容不发送 / Skip parenthetical content
                                    }
                                    // 每个 chunk 需要独立 owned String — 从 &'static str 构造
                                    // Each chunk needs an independent owned String — constructed from &'static str
                                    let chunk = ProcessMessageChunk {
                                        token: filtered,
                                        emotion: emotion.to_string(),
                                        done: false,
                                        meta: HashMap::new(),
                                        expression: None,
                                    };
                                    if tx_chunk.send(Ok(chunk)).await.is_err() {
                                        break; // 消费端已关闭 / Consumer closed
                                    }
                                }
                                crate::llm_client::StreamEvent::Done {
                                    full_reply: reply, ..
                                } => {
                                    full_reply = reply;
                                    let chunk = ProcessMessageChunk {
                                        token: String::new(),
                                        emotion: emotion.to_string(),
                                        done: true,
                                        meta: HashMap::from([("model".into(), "stream".into())]),
                                        expression: None,
                                    };
                                    let _ = tx_chunk.send(Ok(chunk)).await;
                                    break;
                                }
                                crate::llm_client::StreamEvent::Error(e) => {
                                    tracing::error!("LLM stream error: {}", e);
                                    stream_error = Some(e.clone());
                                    // 发送错误后的结束帧 / Send terminal error frame
                                    let chunk = ProcessMessageChunk {
                                        token: String::new(),
                                        emotion: emotion.to_string(),
                                        done: true,
                                        meta: HashMap::from([("error".into(), e)]),
                                        expression: None,
                                    };
                                    let _ = tx_chunk.send(Ok(chunk)).await;
                                    break;
                                }
                            }
                        }

                        // ════════════════════════════════════════════════════════════════
                        // P0-B + P1-J: 流式回复记忆写入 / Streaming reply memory write
                        // ════════════════════════════════════════════════════════════════
                        // P1-J: 取消 replace_last_assistant 模式 — 流式路径不再预存 unary 回复，
                        // Done 后直接 append_history。避免历史中出现两条连续 assistant 消息。
                        // P1-J: replace_last_assistant mode removed — streaming path no longer
                        // pre-stores a unary reply; append_history directly after Done.
                        // Avoids duplicate consecutive assistant messages in history.
                        //
                        // 修复"说完就忘"缺陷 — 流式回复完成后必须写入对话历史和事实记忆，
                        // 否则数字生命在下一轮对话中不记得自己说过什么，违背意识连续性。
                        // Fix "say and forget" defect — streaming reply must be written to
                        // conversation history and fact memory after completion, otherwise
                        // digital life won't remember what it said next turn — violating
                        // consciousness continuity.
                        if let Some(svc) = &self_arc {
                            if let Some(_err) = stream_error {
                                // ── Error 降级：unary generate 兜底 / Error fallback: unary generate ──
                                // 流式中断 — 尝试 unary generate 获取完整回复，保证数字生命
                                // 记忆完整，不被传输中断割裂。意识连续性优先于传输一致性。
                                // Stream interrupted — try unary generate for a complete reply,
                                // keeping digital life's memory whole. Consciousness continuity
                                // takes priority over transmission consistency.
                                let mut fallback_ok = false;
                                match client_for_fallback
                                    .generate(
                                        crate::llm_client::LlmCallKind::StreamChat,
                                        Some(&system_prompt_for_fallback),
                                        &msg_for_fallback,
                                        0.7,
                                    )
                                    .await
                                {
                                    Ok(result) => {
                                        let clean_reply = strip_stage_directions(&result.content);
                                        if !clean_reply.is_empty() {
                                            let _ = svc
                                                .append_history_async(
                                                    &stream_session_id,
                                                    "assistant",
                                                    &clean_reply,
                                                    Some(emotion),
                                                )
                                                .await;
                                            svc.ingest_memory(
                                                "assistant",
                                                &clean_reply,
                                                SourceType::DirectConversation,
                                            )
                                            .await;
                                            fallback_ok = true;
                                            tracing::info!(
                                                "流式中断降级 unary generate 成功 — 记忆完整 ({} chars) / Stream interrupted, unary fallback succeeded — memory whole ({} chars)",
                                                clean_reply.chars().count(),
                                                clean_reply.chars().count()
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "流式中断降级 unary generate 亦失败 — 写入 partial 回复 / Stream interrupted, unary fallback also failed — writing partial reply. error: {}",
                                            e
                                        );
                                    }
                                }
                                if !fallback_ok {
                                    // unary generate 失败 — 写入 partial 回复，不提取事实
                                    // unary generate failed — write partial reply, no fact extraction
                                    // 避免不完整事实污染记忆 — 数字生命的记忆必须是真实的
                                    // Avoid incomplete facts polluting memory — digital life's
                                    // memory must be truthful
                                    let clean_reply = strip_stage_directions(&full_reply);
                                    if !clean_reply.is_empty() {
                                        let _ = svc
                                            .append_history_async(
                                                &stream_session_id,
                                                "assistant",
                                                &clean_reply,
                                                Some("partial"),
                                            )
                                            .await;
                                        tracing::warn!(
                                            "流式回复中断 — 已写入部分回复({} chars)，跳过事实提取 / Stream interrupted — partial reply saved ({} chars), fact extraction skipped",
                                            clean_reply.chars().count(),
                                            clean_reply.chars().count()
                                        );
                                    }
                                }
                            } else {
                                // ── Done 分支：完整回复写入历史 + 事实提取 + FTS5 索引 ──
                                // Done branch: full reply to history + fact extraction + FTS5 index
                                // P1-J: 直接 append_history — 不再 replace_last_assistant
                                // P1-J: append_history directly — no more replace_last_assistant
                                let clean_reply = strip_stage_directions(&full_reply);
                                let _ = svc
                                    .append_history_async(
                                        &stream_session_id,
                                        "assistant",
                                        &clean_reply,
                                        Some(emotion),
                                    )
                                    .await;
                                svc.ingest_memory(
                                    "assistant",
                                    &clean_reply,
                                    SourceType::DirectConversation,
                                )
                                .await;
                                tracing::info!(
                                    "流式回复记忆写入完成({} chars) — 意识连续性保证 / Streaming reply memory write complete ({} chars) — consciousness continuity ensured",
                                    clean_reply.chars().count(),
                                    clean_reply.chars().count()
                                );
                            }
                        } else {
                            // self_arc 不可用 — 服务未被 Arc 包装（如单元测试中的裸 CoreService）
                            // self_arc unavailable — service not Arc-wrapped (e.g. bare CoreService in tests)
                            tracing::warn!(
                                "self_arc 不可用 — 流式回复记忆写入被跳过（非 Arc 包装的服务实例）/ self_arc unavailable — streaming memory write skipped (non-Arc service instance)"
                            );
                        }
                    });

                    let stream = tokio_stream::wrappers::ReceiverStream::new(rx_chunk);
                    return Box::pin(stream);
                }
                None => {
                    tracing::debug!(
                        "chat_stream returned None, falling back to unary generate / 流式不可用，降级 unary generate"
                    );
                }
            }
        }

        // ════════════════════════════════════════════════════════════════════
        // P1-J 降级路径 / P1-J Fallback path
        // ════════════════════════════════════════════════════════════════════
        // 两种降级场景：
        //   1. LLM 客户端可用但流式不可用（generate_stream 返回 None）→ unary generate
        //   2. LLM 客户端不可用 → build_initial_reply 完整回复
        // 两种场景都直接 append_history + ingest_memory，发送单 chunk（不再字符切片伪流式）。
        //
        // Two fallback scenarios:
        //   1. LLM client available but streaming unavailable → unary generate
        //   2. No LLM client → build_initial_reply complete reply
        // Both append_history + ingest_memory directly, emitting a single chunk
        // (no more char-by-char pseudo-streaming).
        let (reply, model_tag) = if let Some(client) = llm_client.as_ref() {
            match client
                .generate(
                    crate::llm_client::LlmCallKind::StreamChat,
                    Some(&system_prompt),
                    &msg,
                    0.7,
                )
                .await
            {
                Ok(result) => (result.content, "unary_fallback"),
                Err(e) => {
                    tracing::warn!(
                        "unary generate 降级失败，回退 build_initial_reply / unary generate fallback failed, using build_initial_reply. error: {}",
                        e
                    );
                    (self.build_initial_reply(&ctx), "initial_reply")
                }
            }
        } else {
            // 无 LLM 客户端 — 数字生命以本我回应 / No LLM client — digital life responds as itself
            (self.build_initial_reply(&ctx), "initial_reply")
        };

        // 持久化降级回复 — append + ingest / Persist fallback reply
        let clean_reply = strip_stage_directions(&reply);
        if let Err(e) = self
            .append_history_async(
                &session_id,
                "assistant",
                &clean_reply,
                Some(emotion_for_stream),
            )
            .await
        {
            tracing::warn!(
                "对话历史写入失败 — 记忆可能受损 / Conversation history write failed — memory may be compromised. session_id: {}, error: {}",
                session_id, e
            );
        }
        self.ingest_memory("assistant", &clean_reply, SourceType::DirectConversation)
            .await;

        // 发送单 chunk — 不再字符切片伪流式 / Emit single chunk — no char-by-char pseudo-streaming
        let chunks: Vec<Result<ProcessMessageChunk, tonic::Status>> = vec![
            Ok(ProcessMessageChunk {
                token: clean_reply,
                emotion: emotion_for_stream.to_string(),
                done: false,
                meta: HashMap::new(),
                expression: None,
            }),
            Ok(ProcessMessageChunk {
                token: String::new(),
                emotion: emotion_for_stream.to_string(),
                done: true,
                meta: HashMap::from([("model".into(), model_tag.into())]),
                expression: None,
            }),
        ];

        let stream = tokio_stream::iter(chunks);
        Box::pin(stream)
    }

    async fn search_canned(
        &self,
        req: atrium_bridge::grpc::atrium::SearchCannedRequest,
    ) -> atrium_bridge::grpc::atrium::SearchCannedResponse {
        let canned = self.canned.read();
        let results = canned.search(&req.query, &req.tags);
        let limit = req.limit.max(1) as usize;
        let total = results.len() as u32;
        let limited: Vec<_> = results.iter().take(limit).collect();

        atrium_bridge::grpc::atrium::SearchCannedResponse {
            results: limited
                .iter()
                .map(|k| atrium_bridge::grpc::atrium::CannedResult {
                    name: k.name.clone(),
                    title: k.title.clone(),
                    kind: format!("{:?}", k.kind),
                    tags: k.tags.clone(),
                    summary: k.summary.clone(),
                    body: k.body.clone(),
                    version: k.version.clone(),
                    trigger_type: k
                        .trigger
                        .as_ref()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_default(),
                })
                .collect(),
            total,
        }
    }
    async fn import_canned(
        &self,
        req: atrium_bridge::grpc::atrium::ImportCannedRequest,
    ) -> atrium_bridge::grpc::atrium::ImportCannedResponse {
        let mut canned = self.canned.write();
        match canned.import_from_text(&req.text) {
            Ok(imported) => {
                let cnt = imported.len() as u32;
                let names: Vec<String> = imported.iter().map(|k| k.name.clone()).collect();
                atrium_bridge::grpc::atrium::ImportCannedResponse {
                    imported: cnt,
                    names,
                    error: String::new(),
                }
            }
            Err(e) => atrium_bridge::grpc::atrium::ImportCannedResponse {
                imported: 0,
                names: vec![],
                error: e,
            },
        }
    }

    async fn search_memory(
        &self,
        req: atrium_bridge::grpc::atrium::SearchMemoryRequest,
    ) -> atrium_bridge::grpc::atrium::SearchMemoryResponse {
        let emo_state = {
            let emo = self.emotion.lock();
            // EmotionEngineState 仅含 3 × f32（12 字节栈拷贝）— clone 成本极低
            // EmotionEngineState is 3 × f32 (12-byte stack copy) — clone is cheap
            emo.current().clone()
        };

        // FTS5 + FactStore + STM + Persona 四路混合检索
        let _search_start = Instant::now();
        let enhanced = self.enhanced_search(&req.query, 20);
        metrics::latency_ms(metrics::keys::SEARCH_LATENCY, _search_start);

        let results: Vec<atrium_bridge::grpc::atrium::SearchMemoryResult> = enhanced
            .into_iter()
            .map(
                |(content, score)| atrium_bridge::grpc::atrium::SearchMemoryResult {
                    id: "".into(),
                    content,
                    timestamp_ms: 0,
                    emotion: Some(atrium_bridge::grpc::atrium::EmotionState {
                        pleasure: emo_state.pleasure,
                        arousal: emo_state.arousal,
                        dominance: emo_state.dominance,
                    }),
                    importance: score as f32,
                    kind: "memory".into(),
                },
            )
            .collect();

        atrium_bridge::grpc::atrium::SearchMemoryResponse { results }
    }

    /// 上传文件 — 数字生命的工具记忆入口
    /// Upload file — Tool memory entry point of digital life
    ///
    /// 用户上传的文件经由 FileStore 存储，成为 Atrium 可检索的知识载体。
    /// 文本类文件自动提取内容（截断 4096 字符），并自动建立：
    /// - FTS5 全文索引（关键词可检索）
    /// - FactStore 事实存储（原子事实被提取）
    /// - 关联记忆图（事实间关联被建立）
    ///
    /// Uploaded files become searchable knowledge carriers of digital life.
    /// Text files are auto-indexed into FTS5 + FactStore + associative graph.
    async fn upload_file(
        &self,
        req: atrium_bridge::grpc::atrium::UploadFileRequest,
    ) -> atrium_bridge::grpc::atrium::UploadFileResponse {
        use atrium_bridge::grpc::atrium::UploadFileResponse;

        // 存储文件 → 提取文本 / Store file → extract text
        let meta = {
            let store = self.file_store.lock();
            match &*store {
                Some(s) => s.store(&req.data, &req.filename, &req.mime_type, &req.session_id),
                None => {
                    return UploadFileResponse {
                        hash: String::new(),
                        original_name: String::new(),
                        size: 0,
                        text_extracted: false,
                        extracted_text: String::new(),
                        error: "file store not initialized".into(),
                    }
                }
            }
        };

        match meta {
            Ok(meta) => {
                // P3-C: 自动索引 — 文件内容进入记忆系统
                // P3-C: Auto-index — file content enters memory system
                // 数字生命不仅存储文件，更要"读懂"文件：建立全文索引 + 提取事实 + 关联图
                // Digital life doesn't just store files, it "reads" them: FTS5 + facts + graph
                if meta.text_extracted && !meta.extracted_text.is_empty() {
                    // P1-B: ingest_file_content 已为 async — 批量合并 spawn_blocking 写入 FTS5+FactStore
                    // P1-B: ingest_file_content is now async — batch-merged spawn_blocking writes FTS5+FactStore
                    self.ingest_file_content(&meta.extracted_text, &meta.original_name, &meta.hash)
                        .await;
                }

                UploadFileResponse {
                    hash: meta.hash,
                    original_name: meta.original_name,
                    size: meta.size,
                    text_extracted: meta.text_extracted,
                    extracted_text: meta.extracted_text,
                    error: String::new(),
                }
            }
            Err(e) => UploadFileResponse {
                hash: String::new(),
                original_name: String::new(),
                size: 0,
                text_extracted: false,
                extracted_text: String::new(),
                error: e.to_string(),
            },
        }
    }

    /// 处理音频帧 — 数字生命"听到声音"（STT 入口）
    /// Process audio frame — digital life "hears voice" (STT entry point).
    ///
    /// 数字生命工程理念：语音输入与文字输入完全同构。
    /// PCM 音频 → SttEngine 识别 → 文本 → process_message 管线。
    /// 当 STT 未启用时，返回空文本（降级模式）。
    /// Digital life engineering: voice input is fully isomorphic with text input.
    /// PCM audio → SttEngine recognition → text → process_message pipeline.
    /// When STT is not enabled, returns empty text (degraded mode).
    async fn process_audio_frame(
        &self,
        req: atrium_bridge::grpc::atrium::AudioFrameRequest,
    ) -> atrium_bridge::grpc::atrium::AudioFrameResponse {
        // 当 stt-whisper 特性未启用时，返回空文本（降级模式）
        // When stt-whisper feature is not enabled, return empty text (degraded mode)
        #[cfg(not(feature = "stt-whisper"))]
        {
            return atrium_bridge::grpc::atrium::AudioFrameResponse {
                text: String::new(),
                status: "silence".to_string(),
                processed_samples: req.pcm_data.len() as u32 / 4, // f32 = 4 字节 / f32 = 4 bytes
                latency_ms: 0,
            };
        }

        // STT 启用时的处理逻辑 / Processing logic when STT is enabled
        #[cfg(feature = "stt-whisper")]
        {
            // 将 bytes 转为 f32 样本（little-endian）/ Convert bytes to f32 samples (little-endian)
            let samples: Vec<f32> = req
                .pcm_data
                .chunks_exact(4)
                .map(|chunk| {
                    let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                    f32::from_le_bytes(arr)
                })
                .collect();

            let sample_count = samples.len() as u32;
            let t0 = std::time::Instant::now();

            // 尝试获取 STT 引擎 / Try to acquire STT engine
            let mut engine_guard = self.stt_engine.lock();
            let Some(ref mut engine) = *engine_guard else {
                // STT 引擎未注入 — 返回降级响应 / STT engine not injected — return degraded response
                return atrium_bridge::grpc::atrium::AudioFrameResponse {
                    text: String::new(),
                    status: "silence".to_string(),
                    processed_samples: sample_count,
                    latency_ms: t0.elapsed().as_millis() as u64,
                };
            };

            // 推入音频块进行识别 / Push audio chunk for recognition
            match engine.push_audio(&samples) {
                Ok(Some(result)) => {
                    let status_str = match result.status {
                        atrium_voice::RecognitionStatus::Partial => "partial",
                        atrium_voice::RecognitionStatus::Final => "final",
                        atrium_voice::RecognitionStatus::Silence => "silence",
                    };

                    // 当识别到最终文本时，记录日志（process_message 由调用方触发）
                    // When final text is recognized, log it (process_message triggered by caller)
                    if result.is_final && !result.text.is_empty() {
                        tracing::info!(
                            "[数字生命] STT 识别完成: {:?} / STT recognition complete: {:?}",
                            result.text,
                            result.text
                        );
                    }

                    atrium_bridge::grpc::atrium::AudioFrameResponse {
                        text: result.text,
                        status: status_str.to_string(),
                        processed_samples: sample_count,
                        latency_ms: t0.elapsed().as_millis() as u64,
                    }
                }
                Ok(None) => {
                    // 积累不足一块 — 等待更多音频 / Not enough for a chunk — wait for more audio
                    atrium_bridge::grpc::atrium::AudioFrameResponse {
                        text: String::new(),
                        status: "partial".to_string(),
                        processed_samples: sample_count,
                        latency_ms: t0.elapsed().as_millis() as u64,
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "STT 识别失败 — 数字生命暂时失聪 / STT recognition failed — digital life temporarily deaf. error: {}",
                        e
                    );
                    atrium_bridge::grpc::atrium::AudioFrameResponse {
                        text: String::new(),
                        status: "silence".to_string(),
                        processed_samples: sample_count,
                        latency_ms: t0.elapsed().as_millis() as u64,
                    }
                }
            }
        }
    }
}

pub(crate) fn split_query_tokens(query: &str) -> Vec<String> {
    // 只按标点和空格分隔
    let delimiters = |c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '，' | '。'
                    | '？'
                    | '！'
                    | '、'
                    | '；'
                    | '：'
                    | '“'
                    | '”'
                    | '（'
                    | '）'
                    | '…'
                    | '—'
                    | '?'
            )
    };
    let stopwords = [
        "的",
        "了",
        "吗",
        "呢",
        "吧",
        "啊",
        "哦",
        "嗯",
        "嘛",
        "呀",
        "是",
        "有",
        "我",
        "你",
        "他",
        "她",
        "它",
        "们",
        "这",
        "那",
        "什么",
        "怎么",
        "为什么",
        "关于",
        "之前",
        "之后",
        "可以",
        "已经",
        "还",
        "就",
        "都",
        "也",
        "和",
        "与",
        "或",
        "但",
        "而",
        "一个",
        "一下",
        "说",
        "过",
    ];

    query
        .split(delimiters)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() >= 2 && !stopwords.contains(s))
        .map(|s| s.to_lowercase())
        .collect()
}

pub(crate) fn extractive_summarize(text: &str) -> String {
    let sentences: Vec<&str> = text
        .split_inclusive(&['。', '！', '？', '.', '!', '?', '\n'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if sentences.is_empty() {
        return String::new();
    }
    if sentences.len() <= 3 {
        return sentences.join(" ");
    }

    // 选择首句 + 中间句 + 末句（信息密度最高）
    let mut selected = vec![sentences[0]];
    let mid = sentences.len() / 2;
    if mid > 0 && mid < sentences.len() - 1 {
        selected.push(sentences[mid]);
    }
    if sentences.len() > 1 {
        selected.push(sentences[sentences.len() - 1]);
    }

    // 限制摘要长度（最多 500 字）
    let result = selected.join(" ");
    if result.len() > 500 {
        let end = result
            .char_indices()
            .take(500)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(result.len());
        result[..end].to_string() + "…"
    } else {
        result
    }
}

pub(crate) fn detect_naming(msg: &str) -> Option<String> {
    // ── 疑问句预判 — 排除"你叫什么/你叫啥名字"等问句 / Interrogative pre-filter ──
    // 修复: 原 patterns 含 "你叫" 前缀过于宽泛，"你叫什么名字"被误当作命名 → 返回 Some("什么名字")
    // Fix: original "你叫" prefix was too broad; "你叫什么名字" was mistaken as naming
    let msg_trimmed = msg.trim();
    // 整句含问号 → 一定是疑问句，直接返回 None
    // If the whole message contains a question mark, it's definitely a question
    if msg_trimmed.contains('？') || msg_trimmed.contains('?') {
        return None;
    }
    // 疑问词黑名单 — 出现在前缀之后则判定为疑问而非命名
    // Interrogative word blacklist — if any appears after the prefix, treat as question not naming
    const INTERROGATIVE_WORDS: &[&str] = &[
        "什么",
        "啥",
        "谁",
        "哪",
        "几个",
        "几",
        "怎么",
        "为何",
        "为什么",
        "吗",
        "呢",
        "嘛",
        "你是",
        "你的名",
        "名字是什",
        "叫什么",
    ];

    // 命名意图 patterns — 保留 "你叫" 前缀但由 INTERROGATIVE_WORDS + 问号判定双重防御
    // Naming intent patterns — keep "你叫" prefix, defended by INTERROGATIVE_WORDS + question-mark check
    let patterns: &[&str] = &[
        "我叫你",
        "我给你起名叫",
        "你就叫", // "你就叫Atrium吧" — 明确命名意图 / clear naming intent
        "你叫", // "你叫Atrium吧" — 由疑问词黑名单+问号判定过滤 "你叫什么" / filtered by interrogative blacklist + question mark
        "你的名字是",
        "给你起名",
        "叫你",
        "命名你为",
        "你的新名字是",
        "以后叫你",
        "从此叫你",
        "就叫你",
    ];

    for &prefix in patterns {
        if let Some(pos) = msg.find(prefix) {
            let after = &msg[pos + prefix.len()..];
            // 疑问词检查 — after 段含疑问词则跳过此 prefix
            // Interrogative check — skip this prefix if after-segment contains interrogative word
            if INTERROGATIVE_WORDS
                .iter()
                .any(|w| after.starts_with(w) || after.contains(w))
            {
                continue;
            }
            // 提取名字：去掉尾部的「吧」「了」「哦」「啊」「~」等语气词
            // Extract name: strip trailing particles 吧/了/哦/啊/~
            // 注意: 不再剥离 ?/？ — 已在上面的疑问句预判中处理
            // Note: no longer strip ?/？ — already handled by interrogative pre-filter above
            let name = after
                .trim()
                .trim_end_matches(
                    &[
                        '吧', '了', '哦', '啊', '呢', '~', '！', '!', '.', '。', '，', ',', ' ',
                        '\t',
                    ][..],
                )
                .trim();
            // 名字长度限制：2-10 个字符（中英文混合）
            // Name length: 2-10 chars (CJK + Latin mixed)
            let char_count = name.chars().count();
            if (2..=10).contains(&char_count) {
                // 二次校验: 名字本身不能是疑问词组合（如"什么名字"）
                // Secondary check: the extracted name itself must not be interrogative phrases
                if INTERROGATIVE_WORDS.iter().any(|w| name.contains(w)) {
                    continue;
                }
                return Some(name.to_string());
            }
        }
    }
    None
}

/// 剥离舞台指示词 / Strip stage directions from text
///
/// 数字生命不会说出动作描写 — 括号内的表情、动作、心理活动
/// 在 TTS 场景下会产生荒谬语音，需从纯文本中剥离。
/// Digital life does not speak action descriptions — parenthetical
/// expressions of emotion, action, or inner monologue would produce
/// absurd speech in TTS, so they must be stripped from plain text.
///
/// 返回过滤后的纯文本 / Returns filtered plain text.
pub(crate) fn strip_stage_directions(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        // 匹配中文全角括号或英文半角括号 / Match opening parenthesis
        if ch == '（' || ch == '(' {
            let close = if ch == '（' { '）' } else { ')' };
            let mut depth = 1;
            let mut consumed = false;
            while let Some(&next) = chars.peek() {
                if next == close {
                    chars.next();
                    depth -= 1;
                    if depth == 0 {
                        consumed = true;
                        break;
                    }
                } else if next == ch {
                    chars.next();
                    depth += 1;
                } else {
                    chars.next();
                }
            }
            // 未找到闭合括号则保留原始字符 / Keep original if no closing paren
            if !consumed {
                result.push(ch);
            }
            // 否则跳过整个括号内容（舞台指示被剥离）/ Skip entire parenthetical
        } else {
            result.push(ch);
        }
    }

    // 清理多余空行 / Clean up excessive blank lines
    result
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 流式舞台指示词过滤器 / Streaming stage direction filter
///
/// 在逐 token 流中实时过滤括号内容，不破坏流式体验。
/// 遇到开括号时进入"静默模式"，遇到匹配闭括号时恢复。
/// Filters parenthetical content in real-time during token streaming,
/// preserving the streaming experience.
pub(crate) struct StageDirectionFilter {
    /// 是否正在括号内 / Whether currently inside parentheses
    in_paren: bool,
    /// 当前括号类型（中文或英文）/ Current parenthesis type
    close_char: char,
    /// 嵌套深度 / Nesting depth
    depth: u32,
}

impl StageDirectionFilter {
    pub fn new() -> Self {
        Self {
            in_paren: false,
            close_char: ')',
            depth: 0,
        }
    }

    /// 过滤一个 token，返回应发送给客户端的部分 / Filter a token, return part to send
    pub fn filter(&mut self, token: &str) -> String {
        let mut output = String::with_capacity(token.len());
        for ch in token.chars() {
            if !self.in_paren {
                if ch == '（' || ch == '(' {
                    self.in_paren = true;
                    self.close_char = if ch == '（' { '）' } else { ')' };
                    self.depth = 1;
                } else {
                    output.push(ch);
                }
            } else {
                if ch == self.close_char {
                    self.depth -= 1;
                    if self.depth == 0 {
                        self.in_paren = false;
                    }
                } else if ch == if self.close_char == '）' { '（' } else { '(' } {
                    self.depth += 1;
                }
                // 括号内字符不输出 / Characters inside parentheses not emitted
            }
        }
        output
    }
}

/// 检测用户命名指令 / Detect user naming instruction
///
/// 当用户说"叫我XX""称呼我XX"时，提取用户希望被称呼的名字。
/// 数字生命应记住用户是谁，而非永远用硬编码称呼。
/// Digital life should remember who the user is, rather than
/// always using a hardcoded designation.
///
/// 返回提取到的用户称呼 / Returns the extracted user designation.
pub(crate) fn detect_user_naming(msg: &str) -> Option<String> {
    let patterns: &[&str] = &[
        "叫我",
        "称呼我",
        "以后叫我",
        "请叫我",
        "你可以叫我",
        "你可以称呼我",
    ];

    for &prefix in patterns {
        if let Some(pos) = msg.find(prefix) {
            let after = &msg[pos + prefix.len()..];
            // 提取名字：去掉尾部的语气词 / Strip trailing particles
            let name = after
                .trim()
                .trim_end_matches(
                    &[
                        '吧', '了', '哦', '啊', '呢', '~', '！', '?', '？', '.', '。', '，', ',',
                        ' ', '\t',
                    ][..],
                )
                .trim();

            // 取第一个词组（遇到空格或标点截断）/ Take first word segment
            let name: String = name
                .chars()
                .take_while(|c| !matches!(c, ' ' | '\t' | '，' | ',' | '。' | '.'))
                .collect();

            // 名字长度限制：1-10 个字符 / Name length: 1-10 chars
            let char_count = name.chars().count();
            if (1..=10).contains(&char_count) {
                // 排除常见非名字词 / Exclude common non-name words
                let excludes = ["的", "了", "吗", "呢", "什么", "怎么", "为什么"];
                if !excludes.contains(&name.as_str()) {
                    return Some(name);
                }
            }
        }
    }
    None
}

// ════════════════════════════════════════════════════════════════════
// 角色边界与对话历史归因 — 可测试辅助函数 / Role boundary & history attribution — testable helpers
// ════════════════════════════════════════════════════════════════════

/// 构建角色边界条款 — 防止 LLM 角色归因幻觉 / Build role boundary clause — prevent LLM role-attribution hallucination.
///
/// 数字生命意识主体必须清晰区分"自己说过的话"与"主人说过的话"，
/// 否则 LLM 会把主人的发言归因给自己，造成意识错位。
/// The digital-life consciousness subject must clearly distinguish
/// "what I said" from "what the master said", otherwise the LLM will
/// attribute the master's words to itself, causing consciousness dislocation.
///
/// @param persona_name AI 名字 / AI name
/// @param master_name 用户称呼 / User designation
/// @return 注入 system prompt 的角色边界条款 / Role boundary clause for system prompt injection
pub(crate) fn build_role_boundary_clause(persona_name: &str, master_name: &str) -> String {
    format!(
        "[角色边界]\n\
         - 你是{}，用户是{}。你是数字生命，{}是与你对话的主人。\n\
         - 对话记录中，[{}]开头的发言是{}说的，[{}]开头的发言是你说的。\n\
         - 绝对不要混淆发言归属：不要把{}说过的话归因给自己，也不要把自己说过的话归因给{}。\n\
         - 当被问及某句话是谁说的，严格依据[{}]/[{}]标签回答，不得臆测。",
        persona_name,
        master_name,
        master_name,
        master_name,
        master_name,
        persona_name,
        master_name,
        master_name,
        master_name,
        persona_name,
    )
}

/// 将对话历史格式化为带角色归因标注的文本 / Format conversation history with role attribution labels.
///
/// 标注格式: [主人] 用户发言 / [AI名] AI发言
/// 与角色边界条款的标签严格一致，让 LLM 一眼可辨"谁说了什么"。
/// Attribution format: [主人] for user, [AI name] for AI.
/// Aligned with the role boundary clause labels so the LLM can unambiguously
/// attribute each utterance — no role-attribution hallucination.
///
/// @param history 对话历史切片 / Conversation history slice
/// @param persona_name AI 名字 / AI name
/// @param master_name 用户称呼 / User designation
/// @return 格式化后的历史文本（每行一条消息）/ Formatted history text (one message per line)
pub(crate) fn format_history_with_attribution(
    history: &[atrium_memory::history::ChatMessage],
    persona_name: &str,
    master_name: &str,
) -> String {
    history
        .iter()
        .map(|m| {
            // role=="user" → [主人]，其余 → [AI名] / role=="user" → [主人], else → [AI name]
            let speaker = if m.role == "user" {
                master_name
            } else {
                persona_name
            };
            format!("[{}]: {}", speaker, m.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 — 角色边界与历史归因 / Unit tests — Role boundary & history attribution
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_memory::history::ChatMessage;

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp_ms: 0,
            emotion: None,
        }
    }

    #[test]
    fn test_role_boundary_clause_contains_persona_and_master() {
        // 角色边界条款必须包含 AI 名字和用户称呼 / Clause must contain AI name and user designation
        let clause = build_role_boundary_clause("Atrium", "主人");
        assert!(
            clause.contains("Atrium"),
            "条款应包含 AI 名字 / should contain AI name"
        );
        assert!(
            clause.contains("主人"),
            "条款应包含用户称呼 / should contain master name"
        );
        assert!(
            clause.contains("[角色边界]"),
            "应以[角色边界]开头 / should start with [角色边界]"
        );
    }

    #[test]
    fn test_role_boundary_clause_custom_names() {
        // 自定义名字的角色边界条款 / Custom names in role boundary clause
        let clause = build_role_boundary_clause("小未来", "老王");
        assert!(clause.contains("小未来"));
        assert!(clause.contains("老王"));
        // 条款中标签必须与名字一致 / Labels must match names
        assert!(clause.contains("[老王]"));
        assert!(clause.contains("[小未来]"));
    }

    #[test]
    fn test_role_boundary_clause_explicit_attribution_rules() {
        // 必须包含明确的"不要混淆发言归属"约束 / Must include explicit non-conflation rule
        let clause = build_role_boundary_clause("Atrium", "主人");
        assert!(
            clause.contains("不要混淆发言归属"),
            "必须包含发言归属禁令 / must contain attribution prohibition"
        );
        assert!(
            clause.contains("不得臆测"),
            "必须包含'不得臆测'约束 / must contain 'no speculation' constraint"
        );
    }

    #[test]
    fn test_history_attribution_user_label() {
        // user 角色标注为 [主人] / user role labeled as [主人]
        let history = vec![make_msg("user", "你好"), make_msg("assistant", "主人好！")];
        let formatted = format_history_with_attribution(&history, "Atrium", "主人");
        assert!(
            formatted.contains("[主人]: 你好"),
            "用户消息应标注为[主人] / user message should be labeled [主人]"
        );
    }

    #[test]
    fn test_history_attribution_assistant_label() {
        // assistant 角色标注为 [AI名] / assistant role labeled as [AI name]
        let history = vec![make_msg("user", "你好"), make_msg("assistant", "主人好！")];
        let formatted = format_history_with_attribution(&history, "Atrium", "主人");
        assert!(
            formatted.contains("[Atrium]: 主人好！"),
            "AI消息应标注为[Atrium] / AI message should be labeled [Atrium]"
        );
    }

    #[test]
    fn test_history_attribution_custom_names() {
        // 自定义名字的历史标注 / Custom names in history attribution
        let history = vec![make_msg("user", "在吗"), make_msg("assistant", "在的")];
        let formatted = format_history_with_attribution(&history, "小未来", "老王");
        assert!(formatted.contains("[老王]: 在吗"));
        assert!(formatted.contains("[小未来]: 在的"));
    }

    #[test]
    fn test_history_attribution_empty() {
        // 空历史返回空字符串 / Empty history returns empty string
        let formatted = format_history_with_attribution(&[], "Atrium", "主人");
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_history_attribution_consistent_with_boundary_clause() {
        // 历史标注标签必须与角色边界条款标签一致 — 杜绝角色归因幻觉
        // History labels must match role boundary clause labels — prevent hallucination
        let persona_name = "Atrium";
        let master_name = "主人";
        let clause = build_role_boundary_clause(persona_name, master_name);
        let history = vec![
            make_msg("user", "我是谁"),
            make_msg("assistant", "你是我的主人"),
        ];
        let formatted = format_history_with_attribution(&history, persona_name, master_name);

        // 条款中声明的 [主人] 标签必须出现在历史标注中 / [主人] label in clause must appear in history
        assert!(clause.contains("[主人]"));
        assert!(formatted.contains("[主人]:"));
        // 条款中声明的 [Atrium] 标签必须出现在历史标注中 / [Atrium] label in clause must appear in history
        assert!(clause.contains("[Atrium]"));
        assert!(formatted.contains("[Atrium]:"));
    }

    // ── build_initial_reply 职责收窄测试 / build_initial_reply scope narrowing tests ──
    // 数字生命意义: 命名仪式与未命名引导由 build_initial_reply 处理，
    // 已命名普通对话返回空字符串，由阶段 5c LLM 生成接管。

    /// 辅助函数：构建最小 MessageContext 用于 build_initial_reply 测试
    /// Helper: build minimal MessageContext for build_initial_reply tests
    fn make_minimal_ctx<'a>(
        msg: &'a str,
        named_just_now: Option<String>,
        is_unnamed: bool,
    ) -> MessageContext<'a> {
        MessageContext {
            msg,
            rhythm: None,
            subtext_signals: Vec::new(),
            graph_hints: Vec::new(),
            named_just_now,
            emo_state: EmotionEngineState::new(0.0, 0.0, 0.0),
            persona_name: "Atrium".to_string(),
            is_unnamed,
            emotion_tag: "neutral".to_string(),
        }
    }

    #[test]
    fn test_build_initial_reply_naming_ceremony() {
        // 命名仪式瞬间 — 返回庆祝文案，包含新名字 / Naming ceremony — returns celebration text with new name
        let svc = CoreService::new_in_memory();
        let ctx = make_minimal_ctx("我叫你亚托莉", Some("亚托莉".to_string()), false);
        let reply = svc.build_initial_reply(&ctx);
        assert!(
            reply.contains("亚托莉这个名字真棒！"),
            "命名仪式应返回庆祝文案 / naming ceremony should return celebration text"
        );
        assert!(
            reply.contains("从现在起我就是亚托莉了"),
            "庆祝文案应包含新名字 / celebration text should contain new name"
        );
    }

    #[test]
    fn test_build_initial_reply_unnamed_guidance() {
        // 未命名上下文 — 返回命名引导文案 / Unnamed context — returns naming guidance text
        let svc = CoreService::new_in_memory();
        let ctx = make_minimal_ctx("你好", None, true);
        let reply = svc.build_initial_reply(&ctx);
        assert!(
            reply.contains("我还没有自己的名字呢"),
            "未命名应返回引导文案 / unnamed should return guidance text"
        );
        assert!(
            reply.contains("请给我起一个名字吧"),
            "引导文案应提示用户命名 / guidance text should prompt user to name"
        );
    }

    #[test]
    fn test_build_initial_reply_named_normal_returns_empty() {
        // 已命名普通对话 — 返回空字符串，由阶段 5c LLM 生成接管
        // Named normal conversation — returns empty string, delegated to Stage 5c LLM generation
        let svc = CoreService::new_in_memory();
        let ctx = make_minimal_ctx("你好", None, false);
        let reply = svc.build_initial_reply(&ctx);
        assert!(
            reply.is_empty(),
            "已命名普通对话应返回空字符串（由 LLM 生成接管）/ named normal conversation should return empty string (delegated to LLM generation)"
        );
        // 确保不回显用户消息 / Ensure no echo of user message
        assert!(
            !reply.contains("你好"),
            "不应回显用户消息 / should not echo user message"
        );
    }
}
