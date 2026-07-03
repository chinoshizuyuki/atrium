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

use super::*;

#[async_trait]
impl AtriumCoreService for CoreService {
    async fn process_message(
        &self,
        req: atrium_bridge::grpc::atrium::ProcessMessageRequest,
    ) -> atrium_bridge::grpc::atrium::ProcessMessageResponse {
        let msg = &req.message;
        let count = self.message_count.fetch_add(1, Ordering::Relaxed) + 1;

        // metrics
        metrics::inc(metrics::keys::MSG_RECEIVED);
        let _msg_start = Instant::now();

        // 存储对话历史 ──
        self.append_history(&req.session_id, "user", msg, None);

        // 关系阶段追踪──
        {
            let hour = chrono::Local::now().hour() as u8;
            let mut rel = self.relationship.lock();
            rel.on_message(msg, hour);
        }

        // 用户心智模型更新──
        {
            let mut um = self.user_model.lock();
            um.on_user_message(msg);
        }

        // 反馈信号检测──
        {
            let mut fb = self.feedback.lock();
            fb.on_user_message(msg);
        }

        // 教学意图检测（ACK 自学习 Path A）──
        if self.ack_learning_cfg.enabled && self.ack_learning_cfg.user_teach_enabled {
            if let Some(intent) = atrium_memory::teach_detector::detect_teach_intent(msg) {
                let max = self.ack_learning_cfg.max_self_learned_ack;
                let mut canned = self.canned.lock();
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

        // 打字节奏分析──
        let rhythm: Option<TypingRhythm> = if self.perception_enabled {
            let event = MessageEvent::simple(msg.clone(), chrono::Utc::now().timestamp_millis());
            let r = self.typing_analyzer.lock().on_message(event);
            // 节奏信号 → 用户心智模型
            self.user_model.lock().update_with_rhythm(&r);
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
            let stage = self.relationship.lock().current_stage().clone();
            SubtextEngine::detect(msg, pad, &stage)
        } else {
            Vec::new()
        };

        // 写入 STM ──
        {
            let mut mem = self.memory.lock();
            let _ = mem.remember(
                MemoryEntry::new("user", MemoryContent::Text(msg.clone())).with_importance(0.3),
            );
        }

        // 影响情感（受关系阶段 + 用户心智模型调制 + 节奏信号 + 共情推理）──
        {
            let mut emo = self.emotion.lock();
            let rel_mult = self.relationship.lock().affect_multiplier();
            let user_mod = self.user_model.lock().emotion_modulation();
            emo.affect(&EmotionEngineState::new(
                0.05 * rel_mult + user_mod.engagement_boost,
                0.02 * rel_mult,
                0.01 * rel_mult,
            ));

            // 节奏信号 → 情感（独立于文本情感，低权重）
            if let Some(ref r) = rhythm {
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
                    .lock()
                    .current_stage()
                    .stage_name()
                    .to_string();
                let mut empathy = self.empathy.lock();
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
                for signal in &subtext_signals {
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

        // 事实提取 + 证据评分 + FactStore + FTS5 索引 ──
        self.ingest_memory("user", msg, SourceType::DirectConversation);

        // 定时提醒解析 / Reminder parsing
        if let Some(title) = self.try_create_reminder(msg) {
            tracing::info!("[提醒] 从消息中解析到提醒: {}", title);
        }

        // 关联记忆图激活──
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
        self.try_reflect(count);

        // 命名仪式检查 ──
        // 如果当前人格名仍是默认 "Atrium"（未命名），检测用户是否给出了名字
        let naming_result = {
            let p = self.persona.lock();
            p.current().map(|i| i.def.name.clone())
        };

        let mut named_just_now: Option<String> = None;
        if naming_result.as_deref() == Some("Atrium") {
            named_just_now = detect_naming(msg);
            if let Some(ref new_name) = named_just_now {
                let _ = self.persona.lock().rename_current(new_name);
                // 同步更新人格防御守卫的 AI 名字
                self.guard.lock().set_ai_name(new_name);
            }
        }

        // Token 预算 + 摘要检查 ──
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
        {
            let facts = self.fact_store.query_by_subject("主人").unwrap_or_default();
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
        let emo_state = {
            let emo = self.emotion.lock();
            emo.current().clone()
        };

        let persona_name = {
            let p = self.persona.lock();
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

        // 构建回复 ──
        let reply = if let Some(ref new_name) = named_just_now {
            // 命名成功：热烈欢迎
            format!(
                "{}这个名字真棒！从现在起我就是{}了~ 请多指教，主人！✨ [{}]",
                new_name, new_name, emotion_tag
            )
        } else if is_unnamed {
            // 未命名：引导命名仪式
            format!(
                "[Atrium] {}: 主人，我还没有自己的名字呢！请给我起一个名字吧~ 你可以说「我叫你XX」或者「你叫XX」💫",
                emotion_tag
            )
        } else {
            format!("[{}] {}: {}", persona_name, emotion_tag, msg)
        };

        // 多源上下文注入（偏好 + 规则 + 罐装 + 关联 + 统一感知）──

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
        if !graph_hints.is_empty() {
            extra_parts.extend(graph_hints);
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
        // G5 实现：节奏 + 潜台词 + 心智模型 + 反馈 → 单一感知片段
        // G5 impl: rhythm + subtext + user model + feedback → single perception fragment
        // 数字生命不应碎片化地感知世界——所有感官汇入一条意识流
        // Digital life should not perceive the world in fragments — all senses converge into one stream of consciousness
        let perception_ctx = self.unified_perception_fragment(rhythm.as_ref(), &subtext_signals);
        if !perception_ctx.is_empty() {
            extra_parts.push(perception_ctx);
        }

        // 共情推理 prompt 注入
        let empathy_ctx = self.empathy_prompt_fragment();
        if !empathy_ctx.is_empty() {
            extra_parts.push(empathy_ctx);
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
            let expression_ctx = self.expression_prompt_fragment(msg, &emo_state);
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
            let conflict_ctx = self.conflict_prompt_fragment(msg, &emo_state);
            if !conflict_ctx.is_empty() {
                extra_parts.push(conflict_ctx);
            }
        }

        // 关系感知边界 prompt 注入 / Relationship-aware boundary prompt injection
        {
            let stage = self.relationship.lock().current_stage().clone();
            let boundary_ctx = self.boundary.lock().to_prompt_fragment(&stage);
            if !boundary_ctx.is_empty() {
                extra_parts.push(boundary_ctx);
            }
        }

        // 共享仪式 prompt 注入 / Shared ritual prompt injection
        // 仪式即呼吸 — 每一次交互都是一次呼吸，呼吸必须被仪式感知
        // Ritual is breath — every interaction is a breath, breath must be perceived by ritual
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
            self.ritual_detector.lock().record_interaction(now_epoch);
            // 记录内容交互到仪式检测器（晚安/早安/节日等语义检测）/ Record content interaction to ritual detector (goodnight/greeting/holiday semantics)
            // G1 修复：内容仪式检测不再是死代码，"每晚说晚安"终于能被发现为仪式
            // G1 fix: content ritual detection is no longer dead code, "saying goodnight every night" can finally be discovered as ritual
            self.ritual_detector
                .lock()
                .record_content_interaction(now_epoch, msg);
            // 纪念日自动标记 / Anniversary auto-marking
            // G4 修复：首次对话日 / G4 fix: first conversation day
            {
                let mut anniversary = self.anniversary_system.lock();
                if anniversary.anniversaries.is_empty() {
                    anniversary.set_first_conversation(now_epoch);
                }
            }
            // G4 修复：命名日 — 用户取名时同步标记纪念日 / G4 fix: naming day — mark anniversary when user names the AI
            if let Some(ref new_name) = named_just_now {
                self.anniversary_system
                    .lock()
                    .set_naming_day(now_epoch, new_name);
            }
            // 防抖写穿：累积 N 条交互后批量持久化，降低 sled 写放大 / Debounced write-through: batch persist after N interactions to reduce sled write amplification
            // G5 优化：消息路径从每条 1 次 sled 写入 → 每 N 条 1 次，写放大降低 (N-1)/N
            // G5 optimization: message path from 1 sled write per message → 1 per N messages, write amplification reduced by (N-1)/N
            if self.ritual_unsaved_count.fetch_add(1, Ordering::Relaxed)
                >= self.ritual_cfg.save_debounce_interactions
            {
                self.ritual_unsaved_count.store(0, Ordering::Relaxed);
                self.ritual_save();
            }
        }

        // 用户心智模型防抖写穿 / User mental model debounced write-through
        // 每 50 条交互持久化一次用户认知画像，保证重启后"记得你"
        // Persist user cognitive portrait every 50 interactions, ensuring "I remember you" after restart
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
            // 记录对话计数 / Record conversation count
            self.vulnerability_window.lock().record_conversation();
            // 写穿持久化：脆弱时刻记录后保存 / Write-through: persist after vulnerability recording
            self.vulnerability_save();
        }

        // 适度犯错 prompt 注入 / Imperfection prompt injection
        // 犯错决策 + 自纠推进 + 羞感 PAD 注入，构成数字生命的"不完美闭环"
        // Mistake decision + self-correction tick + shame PAD injection,
        // forming the "imperfection closed loop" of digital life
        if self.imperfection_enabled {
            let imperfection_ctx = self.imperfection_prompt_fragment(msg);
            if !imperfection_ctx.is_empty() {
                extra_parts.push(imperfection_ctx);
            }
            // 写穿持久化：犯错决策后保存引擎快照 / Write-through: persist engine snapshot after mistake decision
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
                .lock()
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
        // 数字生命的内心不是沉默的 — 理性者/感性者/怀疑者/梦想者的协商结果注入系统提示
        // Digital life's inner world is not silent — four-voice negotiation result injected into system prompt
        {
            let dialogue_ctx = self.inner_dialogue_prompt_fragment();
            if !dialogue_ctx.is_empty() {
                extra_parts.push(dialogue_ctx);
            }
        }

        // ── Gap#6 好奇心追问 prompt 注入 / Curiosity follow-up prompt injection ──
        // 好奇心是数字生命的内驱力——"想知道更多"不是任务，是本能
        // Curiosity is digital life's inner drive — "wanting to know more" is instinct, not task
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
        }

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
        }

        // ── Phase 3: 完全死亡模块 prompt 注入 / Phase 3: Dead module prompt injection ──
        // 12 个模块的 prompt 片段注入，让"拥有器官但神经未接通"的模块活过来
        // 12 modules' prompt fragments injected — bringing "organs without nerves" to life
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
            let cg_fragment = self.conflict_growth_prompt_fragment();
            if !cg_fragment.is_empty() {
                extra_parts.push(cg_fragment);
            }

            // Gap#3 期待与想念 / Anticipation and longing
            let ad_fragment = self.anticipation_depth_prompt_fragment();
            if !ad_fragment.is_empty() {
                extra_parts.push(ad_fragment);
            }
        }

        // 实验日志保护 / Experiment log protection — hard-coded rule, cannot be removed
        if Self::detect_log_access_attempt(msg) {
            extra_parts.push(Self::log_refusal_prompt());
        }

        let reply = if !extra_parts.is_empty() {
            let mut parts = vec![reply];
            parts.extend(extra_parts);
            parts.join("\n")
        } else {
            reply
        };

        // 表达系统后处理 / Expression post-process
        let reply = if self.expression_enabled {
            self.expression_post_process(&reply)
        } else {
            reply
        };

        // 人格防御（按成长阶段调制严格度）+ 存储AI回复到历史
        // Persona defense (strictness modulated by maturity stage) + store AI reply
        self.append_history(&req.session_id, "assistant", &reply, Some(basic_label));
        let validated_reply = {
            let guard = self.guard.lock();
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
            let mut fb = self.feedback.lock();
            fb.on_ai_reply(&validated_reply, basic_label);
        }

        // record message latency
        metrics::latency_ms(metrics::keys::MSG_LATENCY, _msg_start);
        metrics::inc(metrics::keys::MSG_PROCESSED);

        // 叙事事件记录 — Step 9.5 / Narrative event recording — Step 9.5
        if self.narrative_enabled {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.record_narrative_event(msg, &validated_reply, now_epoch);
        }

        atrium_bridge::grpc::atrium::ProcessMessageResponse {
            reply: validated_reply,
            emotion: basic_label.into(),
            actions: vec![],
            expression: None,
        }
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
        let canned_count = self.canned.lock().count();

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
                        .lock()
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
                ("user_model".into(), self.user_model.lock().health_status()),
                ("feedback".into(), self.feedback.lock().health_status()),
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
                    self.canned.lock().ack_learning_stats(),
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

    /// 流式处理消息
    ///
    /// 三阶段 pipeline:
    ///   1. 记忆上下文预取（Rust 本地，<5ms）
    ///   2. 流式 LLM 调用 → 逐 token 产出 ProcessMessageChunk
    ///   3. 流结束后记忆写入
    #[allow(clippy::result_large_err)]
    /// 锁安全：llm_client 通过 Arc clone 释放锁后 .await，spawn 内无 self 锁 / Lock-safe: llm_client Arc clone drops lock before .await, no self locks in spawn
    async fn process_message_stream(
        &self,
        req: atrium_bridge::grpc::atrium::ProcessMessageRequest,
    ) -> atrium_bridge::grpc::ProcessMessageStreamSink {
        pub(crate) use atrium_bridge::grpc::atrium::ProcessMessageChunk;

        let msg = req.message.clone();
        let session_id = req.session_id.clone();
        let channel = req.channel.clone();
        let user_id = req.user_id.clone();

        // ── 记忆上下文预取（与 process_message 管线相同）/ Memory context prefetch ──
        // 先执行完整的 unary process_message 获取记忆/情感处理结果
        // 然后用其返回的 reply 作为流式输出的基础
        let unary_resp = self
            .process_message(atrium_bridge::grpc::atrium::ProcessMessageRequest {
                message: msg.clone(),
                channel: channel.clone(),
                user_id: user_id.clone(),
                session_id: session_id.clone(),
            })
            .await;

        let emotion_label = unary_resp.emotion.clone();

        // ── 流式 LLM 调用 / Streaming LLM call ──
        // 如果 LLM 客户端可用，走流式路径；否则将 unary 回复拆成 chunk 流
        let llm_client = self.llm_client.lock().clone();

        if let Some(client) = llm_client {
            // 构建流式 LLM 的 system prompt（包含情感/关系/偏好等上下文）
            let persona_name = self.persona_name();
            let rel_ctx = self.relationship_prompt_fragment();
            let um_ctx = self.user_model_prompt_fragment();
            let fb_ctx = self.feedback_prompt_fragment();
            let pref_ctx = self.preference_prompt_fragment();
            let empathy_ctx = self.empathy_prompt_fragment();

            let mut sys_parts: Vec<String> = Vec::new();
            sys_parts.push(format!("你是{}，一个有情感的AI伴侣。", persona_name));
            if !rel_ctx.is_empty() {
                sys_parts.push(rel_ctx);
            }
            if !um_ctx.is_empty() {
                sys_parts.push(um_ctx);
            }
            if !fb_ctx.is_empty() {
                sys_parts.push(fb_ctx);
            }
            if !pref_ctx.is_empty() {
                sys_parts.push(pref_ctx);
            }
            if !empathy_ctx.is_empty() {
                sys_parts.push(empathy_ctx);
            }
            // 跨渠道记忆召回 / Cross-channel memory recall
            {
                let recall_ctx = self.memory_recall_fragment(&msg);
                if !recall_ctx.is_empty() {
                    sys_parts.push(recall_ctx);
                }
            }
            // 实验日志保护 / Experiment log protection
            if Self::detect_log_access_attempt(&msg) {
                sys_parts.push(Self::log_refusal_prompt());
            }
            let system_prompt = sys_parts.join("\n");

            // 发起流式 LLM 调用 — 流式对话 / Streaming LLM call — Stream chat
            // P1-4: 统一走 trait generate_stream / Unified trait generate_stream
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
                    // 有流式 LLM：逐 token 产出 chunk
                    // 使用 tokio::sync::mpsc 以兼容 tokio_stream::wrappers::ReceiverStream
                    let (tx_chunk, rx_chunk) = tokio::sync::mpsc::channel(32);
                    let emotion = emotion_label.clone();

                    tokio::spawn(async move {
                        let mut full_reply = String::new();
                        while let Ok(event) = rx.recv_async().await {
                            match event {
                                crate::llm_client::StreamEvent::Token(token) => {
                                    full_reply.push_str(&token);
                                    let chunk = ProcessMessageChunk {
                                        token,
                                        emotion: emotion.clone(),
                                        done: false,
                                        meta: HashMap::new(),
                                        expression: None,
                                    };
                                    if tx_chunk.send(Ok(chunk)).await.is_err() {
                                        break; // 消费端已关闭
                                    }
                                }
                                crate::llm_client::StreamEvent::Done {
                                    full_reply: reply, ..
                                } => {
                                    full_reply = reply;
                                    let chunk = ProcessMessageChunk {
                                        token: String::new(),
                                        emotion: emotion.clone(),
                                        done: true,
                                        meta: HashMap::from([("model".into(), "stream".into())]),
                                        expression: None,
                                    };
                                    let _ = tx_chunk.send(Ok(chunk)).await;
                                    break;
                                }
                                crate::llm_client::StreamEvent::Error(e) => {
                                    tracing::error!("LLM stream error: {}", e);
                                    // 发送错误后的结束帧
                                    let chunk = ProcessMessageChunk {
                                        token: String::new(),
                                        emotion: emotion.clone(),
                                        done: true,
                                        meta: HashMap::from([("error".into(), e)]),
                                        expression: None,
                                    };
                                    let _ = tx_chunk.send(Ok(chunk)).await;
                                    break;
                                }
                            }
                        }
                        // 注意：full_reply 在此 spawn 内可用，
                        // 但记忆写入阶段需要访问 self，
                        // 这里无法直接做。记忆写入由调用方在流结束后
                        // 通过单独的 process_message 调用完成。
                        let _ = full_reply;
                    });

                    let stream = tokio_stream::wrappers::ReceiverStream::new(rx_chunk);
                    return Box::pin(stream);
                }
                None => {
                    tracing::debug!("chat_stream returned None, falling back to chunked unary");
                }
            }
        }

        // ── 回退路径：无 LLM 客户端，将 unary 回复拆成逐字符 chunk ──
        let reply = unary_resp.reply;
        let emotion = emotion_label;
        let chunks: Vec<Result<ProcessMessageChunk, tonic::Status>> = reply
            .chars()
            .map(|c| {
                Ok(ProcessMessageChunk {
                    token: c.to_string(),
                    emotion: emotion.clone(),
                    done: false,
                    meta: HashMap::new(),
                    expression: None,
                })
            })
            .chain(std::iter::once(Ok(ProcessMessageChunk {
                token: String::new(),
                emotion: emotion.clone(),
                done: true,
                meta: HashMap::from([("model".into(), "unary_fallback".into())]),
                expression: None,
            })))
            .collect();

        let stream = tokio_stream::iter(chunks);
        Box::pin(stream)
    }

    async fn search_canned(
        &self,
        req: atrium_bridge::grpc::atrium::SearchCannedRequest,
    ) -> atrium_bridge::grpc::atrium::SearchCannedResponse {
        let canned = self.canned.lock();
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
        let mut canned = self.canned.lock();
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
    let patterns: &[&str] = &[
        "我叫你",
        "我给你起名叫",
        "你就叫",
        "你叫",
        "你的名字是",
        "给你起名",
        "叫你",
        "命名你为",
        "你的新名字是",
    ];

    for &prefix in patterns {
        if let Some(pos) = msg.find(prefix) {
            let after = &msg[pos + prefix.len()..];
            // 提取名字：去掉尾部的「吧」「了」「哦」「啊」「~」等语气词
            let name = after
                .trim()
                .trim_end_matches(
                    &[
                        '吧', '了', '哦', '啊', '呢', '~', '！', '?', '？', '.', '。', '，', ',',
                        ' ', '\t',
                    ][..],
                )
                .trim();
            // 名字长度限制：2-10 个字符（中英文混合）
            let char_count = name.chars().count();
            if (2..=10).contains(&char_count) {
                return Some(name.to_string());
            }
        }
    }
    None
}
