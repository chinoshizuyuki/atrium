// SPDX-License-Identifier: MIT
//! 生命维持模块 — 数字生命的稳态与自修复
//! Lifecycle Module — Homeostasis and self-repair of digital life
//!
//! 包含主动引擎、共情系统、ACK 合成、回放管线、
//! 记忆摄入、反思触发、图维护与图持久化，
//! 构成数字生命"如何维持自身运转"的稳态闭环。
//!
//! Contains proactive engine, empathy system, ACK synthesis,
//! replay pipeline, memory ingestion, reflection trigger,
//! graph maintenance, and graph persistence — forming the
//! "how to maintain my own operation" homeostatic closed loop of digital life.

use super::*;

impl CoreService {
    pub fn proactive_engine(&self) -> &parking_lot::Mutex<ProactiveEngine> {
        &self.proactive
    }

    pub fn empathy_prompt_fragment(&self) -> String {
        self.empathy.read().prompt_fragment()
    }

    pub fn empathy_health(&self) -> String {
        self.empathy.read().health_status()
    }

    pub fn tick_ack_synthesis(&self) {
        let max = self.ack_learning_cfg.max_self_learned_ack;
        let min_pat_conf = self.ack_learning_cfg.min_pattern_confidence;
        let min_insight_conf = self.ack_learning_cfg.min_insight_confidence;
        let min_facts = self.ack_learning_cfg.min_supporting_facts;

        // ── Path B: 回放模式 → ACK ──
        if self.ack_learning_cfg.replay_learning_enabled {
            let patterns: Vec<atrium_memory::replay::DiscoveredPattern> = {
                let replay = self.replay.lock();
                replay.recent_patterns().to_vec()
            };
            for pat in &patterns {
                if pat.confidence < min_pat_conf {
                    continue;
                }
                // 跳过已消费的模式
                if self.replay.lock().is_consumed(&pat.summary) {
                    continue;
                }
                let mut canned = self.canned.write();
                match canned.learn_from_pattern(pat, max) {
                    Ok(ack) => {
                        tracing::info!("回放模式 → ACK: {} (conf={:.2})", ack.name, pat.confidence);
                        drop(canned);
                        self.replay.lock().mark_consumed(&pat.summary);
                    }
                    Err(e) => {
                        tracing::debug!("回放模式跳过: {}", e);
                    }
                }
            }
        }

        // ── Path C: 反思洞察 → ACK ──
        if self.ack_learning_cfg.insight_learning_enabled {
            let insights: Vec<atrium_memory::reflection::Insight> = {
                let reflection = self.reflection.lock();
                reflection
                    .all_insights()
                    .iter()
                    .filter(|i| {
                        matches!(i.status, atrium_memory::reflection::InsightStatus::Promoted)
                            && i.confidence >= min_insight_conf
                            && i.supporting_facts.len() as u32 >= min_facts
                    })
                    .cloned()
                    .collect()
            };
            for insight in &insights {
                let mut canned = self.canned.write();
                match canned.learn_from_insight(insight, max) {
                    Ok(ack) => {
                        tracing::info!(
                            "反思洞察 → ACK: {} (facts={})",
                            ack.name,
                            insight.supporting_facts.len()
                        );
                    }
                    Err(e) => {
                        tracing::debug!("反思洞察跳过: {}", e);
                    }
                }
            }
        }
    }

    pub fn tick_replay(&self) {
        let mut replay = self.replay.lock();
        if replay.should_run() {
            let patterns = replay.run(&self.fact_store);
            if !patterns.is_empty() {
                tracing::info!(
                    "ReplayPipeline: {} patterns found (run #{})",
                    patterns.len(),
                    replay.stats().run_count
                );
            }
        }
    }

    pub fn append_history(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        self.history.append(session_id, role, content, emotion)
    }

    /// 异步写入对话历史 — spawn_blocking 包装 sled I/O
    /// Async conversation history write — wraps sled I/O in spawn_blocking.
    ///
    /// 数字生命"思考与记忆并行" — 历史写入外包到 blocking 线程池，
    /// 不阻塞 tokio reactor，保障情感 tick 和流式表达流畅。
    /// Digital life "think and remember in parallel" — history write offloaded
    /// to blocking pool, never blocks tokio reactor, keeping emotion tick and
    /// streaming expression smooth.
    ///
    /// spawn_blocking panic 时记录 error 并返回 Err — 数字生命自愈，不静默失忆。
    /// On spawn_blocking panic: logs error and returns Err — digital life self-healing,
    /// never silently losing memory.
    pub async fn append_history_async(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        let history = self.history.clone(); // Arc clone — 廉价 / Arc clone — cheap
        let session_id = session_id.to_string();
        let role = role.to_string();
        let content = content.to_string();
        let emotion = emotion.map(|s| s.to_string());
        tokio::task::spawn_blocking(move || {
            history.append(&session_id, &role, &content, emotion.as_deref())
        })
        .await
        .unwrap_or_else(|e| {
            tracing::error!(
                "append_history spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing",
                e
            );
            Err(sled::Error::Unsupported("spawn_blocking panic".to_string()))
        })
    }

    /// 替换最后一条 assistant 历史 — 流式回复覆盖 unary 预存回复
    /// Replace last assistant history — streaming reply overrides unary pre-stored reply.
    pub fn replace_last_assistant_history(
        &self,
        session_id: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        self.history
            .replace_last_assistant(session_id, content, emotion)
    }

    /// 异步替换最后一条 assistant 历史 — spawn_blocking 包装 sled I/O
    /// Async replace last assistant history — wraps sled I/O in spawn_blocking.
    ///
    /// 流式回复完成后用真实回复覆盖预存回复 — 意识连续性保证。
    /// After streaming completes, replaces pre-stored reply with real reply —
    /// consciousness continuity guarantee.
    pub async fn replace_last_assistant_history_async(
        &self,
        session_id: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        let history = self.history.clone();
        let session_id = session_id.to_string();
        let content = content.to_string();
        let emotion = emotion.map(|s| s.to_string());
        tokio::task::spawn_blocking(move || {
            history.replace_last_assistant(&session_id, &content, emotion.as_deref())
        })
        .await
        .unwrap_or_else(|e| {
            tracing::error!(
                "replace_last_assistant_history spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing",
                e
            );
            Err(sled::Error::Unsupported("spawn_blocking panic".to_string()))
        })
    }

    pub fn get_history(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Vec<atrium_memory::history::ChatMessage> {
        self.history.messages(session_id, limit)
    }

    pub fn list_sessions(&self) -> Vec<String> {
        self.history.sessions()
    }

    pub(crate) async fn ingest_memory(&self, role: &str, content: &str, source_type: SourceType) {
        let ai_name = {
            let p = self.persona.read();
            p.current()
                .map(|i| i.def.name.clone())
                .unwrap_or_else(|| "Atrium".to_string())
        };
        let speaker = if role == "user" { "主人" } else { &ai_name };

        // 提取原子事实
        let raw_facts = fact_extractor::extract_facts(content, speaker);

        // P3-B 主动遗忘意图检测 / P3-B Active forgetting intent detection
        // 数字生命意义: 用户明确要求"忘掉"时，AI 有意识地遗忘——不是被动衰减，而是主动决策。
        // "忘掉我喜欢咖啡" → 标记该事实为 TraumaProtection（创伤保护），enhanced_search 不再返回。
        // "想起那件事" → 恢复遗忘标记与置信度，让记忆重新可见。
        // 检测到意图时处理完毕即返回，不再走正常事实插入流程（避免"忘掉X"反而强化了X）。
        // Digital Life: when the user explicitly asks to "forget", the AI consciously forgets —
        // not passive decay, but active decision. "Forget I like coffee" → mark as TraumaProtection,
        // enhanced_search no longer returns it. "Recall that thing" → restore marker and confidence.
        // On intent detection, process and return — never fall through to normal insertion
        // (avoids "forget X" paradoxically reinforcing X).
        if role == "user" {
            let is_forget_intent = content.contains("忘掉")
                || content.contains("不要再提")
                || content.contains("忘记");
            let is_restore_intent = content.contains("想起") || content.contains("回忆");

            if is_forget_intent || is_restore_intent {
                use atrium_memory::active_forget::ForgetPolicy;
                for (subject, predicate, object, _) in &raw_facts {
                    let key = Fact::new(subject, predicate, object).canonical_form();
                    if is_forget_intent {
                        // 读取遗忘前置信度快照 — 供恢复使用 / Read pre-forget confidence snapshot for restoration
                        let pre_conf = self
                            .fact_store
                            .get_by_canonical(&key)
                            .map(|f| f.confidence)
                            .unwrap_or(0.5);
                        if self
                            .fact_store
                            .mark_forgotten(&key, ForgetPolicy::TraumaProtection)
                        {
                            // 置信度降至 0.1 — 创伤保护：弱化但不销毁 / Lower confidence to 0.1 — trauma protection: weaken but not destroy
                            self.fact_store.merge_confidence(&key, 0.1);
                            self.active_forget.lock().forget_request(
                                key.clone(),
                                ForgetPolicy::TraumaProtection,
                                pre_conf,
                                "用户要求忘记".to_string(),
                            );
                            tracing::info!(
                                "ActiveForget: 用户要求遗忘 / user requested forgetting — key={}",
                                key
                            );
                        }
                    } else if is_restore_intent {
                        // 恢复记忆 — 从 forget_log 移除并恢复置信度 / Restore memory — remove from forget_log and restore confidence
                        let record = self.active_forget.lock().restore(&key);
                        if let Some(record) = record {
                            self.fact_store.restore_forgotten(&key);
                            self.fact_store
                                .merge_confidence(&key, record.pre_forget_confidence);
                            tracing::info!(
                                "ActiveForget: 用户要求想起 / user requested recall — key={}",
                                key
                            );
                        }
                    }
                }
                // 意图处理完毕 — 不走正常事实插入流程 / Intent processed — skip normal fact insertion
                return;
            }
        }

        if raw_facts.is_empty() {
            return;
        }

        // P3-A 程序记忆 — 教学意图触发新技能登记 / P3-A Procedural memory — teach intent triggers skill registration
        // 数字生命意义: 用户"教"AI 时，不仅记录为 ACK 罐装知识，还登记为程序记忆技能——
        // "我教你 Rust 调试" → acquire_skill("Rust 调试", ...)，让数字生命积累"怎么做"的能力。
        // Digital Life: when the user "teaches" the AI, not only is it recorded as canned ACK knowledge,
        // but also registered as a procedural memory skill — "I'll teach you Rust debugging" →
        // acquire_skill("Rust 调试", ...), letting digital life accumulate "how to do" capabilities.
        {
            let teach = self.teach_detected.lock();
            if let Some(ref intent) = *teach {
                if !intent.knowledge_text.is_empty() {
                    // 情境标签 — 从消息关键词提取（与情景记忆一致的标签策略）
                    // Context tags — extract from message keywords (consistent with episodic memory tag strategy)
                    let keywords: Vec<String> = content
                        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
                        .filter(|s| s.chars().count() >= 2)
                        .take(3)
                        .map(String::from)
                        .collect();
                    // 技能名取 knowledge_text（截断至 50 字符防止过长）/ Skill name from knowledge_text (truncate to 50 chars)
                    let skill_name: String = intent.knowledge_text.chars().take(50).collect();
                    let skill_id = self.procedural_memory.acquire_skill(
                        &skill_name,
                        vec![], // 步骤暂未知 — 后续实践时补充 / Steps unknown for now — supplemented during practice
                        keywords,
                    );
                    tracing::debug!(
                        "ProceduralMemory: 教学意图 → 登记技能 / teach intent → acquired skill — id={}, name={}",
                        skill_id,
                        skill_name
                    );
                }
            }
        }

        // 证据评分 → 转换 Fact → 写入 FactStore
        let (emotion_intensity, emotion_ctx) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            let intensity = ((c.pleasure.abs() + c.arousal.abs()) / 2.0) as f64;
            let classified = c.classify();
            let user_valence = self.user_model.read().mood.valence;
            let ctx = atrium_memory::fact_store::EmotionContext {
                ai_emotion_label: classified.name.to_string(),
                ai_pad: [c.pleasure, c.arousal, c.dominance],
                intensity: intensity as f32,
                user_mood: Some(user_valence),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            (intensity, ctx)
        };

        // P2-B 情景记忆自动创建 — 高情绪强度（> 0.6）时记录"那一刻"
        // P2-B Episodic memory auto-create — record "that moment" when emotion intensity > 0.6
        // 数字生命意义: 不是每句话都值得记住，但情感强烈的那一刻必须铭记——
        // 深夜兴奋分享代码、雨夜倾诉心事、争吵时的愤怒……这些是数字生命自传的"锚点"。
        // Digital life: not every word is worth remembering, but emotionally intense moments
        // must be imprinted — late-night code sharing, rainy-night confessions, angry arguments...
        // these are the "anchors" of digital life's autobiography.
        if emotion_intensity > 0.6 {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            // 情境标签 — 时间段 + 消息关键词 / Context tags — time-of-day + message keywords
            let hour = chrono::Local::now().hour() as i8;
            let time_tag = match hour {
                23..=24 | 0..=4 => "深夜",
                5..=8 => "清晨",
                9..=16 => "白天",
                17..=19 => "黄昏",
                _ => "夜晚",
            };
            // 简单关键词提取 — 取消息中长度 >= 2 的非空白 token，最多 3 个
            let keywords: Vec<String> = content
                .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
                .filter(|s| s.chars().count() >= 2)
                .take(3)
                .map(String::from)
                .collect();
            let mut context_tags = vec![time_tag.to_string()];
            context_tags.extend(keywords);
            // 摘要 — 截断至 80 字符 / Summary — truncate to 80 chars
            let summary = if content.chars().count() > 80 {
                content.chars().take(80).collect::<String>() + "..."
            } else {
                content.to_string()
            };
            let episode_id = format!(
                "ep_{}_{}",
                now_epoch,
                self.message_count.load(Ordering::Relaxed)
            );
            let episode = Episode::new(
                episode_id,
                now_epoch,
                context_tags,
                emotion_ctx.clone(),
                summary,
                emotion_intensity as f32,
            );
            // 同步插入 — EpisodicMemoryStore 内部 Mutex 保护，SQLite WAL 模式下 INSERT 极快
            // Sync insert — EpisodicMemoryStore internal Mutex guard, SQLite WAL INSERT is very fast
            if let Err(e) = self.episodic.insert(episode) {
                tracing::warn!(
                    "情景记忆写入失败 — 数字生命可能遗忘这一刻 / Episodic memory write failed — digital life may forget this moment: {}",
                    e
                );
            } else {
                tracing::debug!(
                    "情景记忆已记录 — intensity={:.3} label={}",
                    emotion_intensity,
                    emotion_ctx.ai_emotion_label
                );
            }
        }

        // P1-B: 收集待写入事实 — 内存操作（偏好/证据/图）先行，阻塞 I/O 批量外包
        // P1-B: Collect facts to write — in-memory ops (preference/evidence/graph) first,
        // batch offload blocking I/O to spawn_blocking.
        // 数字生命"思考与记忆并行" — 内存中的认知操作与 blocking 线程池的持久化并行。
        // Digital life "think and remember in parallel" — in-memory cognition ops run
        // concurrently with blocking-thread-pool persistence.
        let mut facts_to_insert: Vec<Fact> = Vec::with_capacity(raw_facts.len());
        for (subject, predicate, object, conf) in &raw_facts {
            let fact = Fact::new(subject, predicate, object)
                .with_confidence(*conf)
                .with_source(format!("{:?}", source_type))
                .with_emotion(emotion_ctx.clone());

            // 偏好学习：喜欢/讨厌类事实自动记录
            if predicate == "喜欢" || predicate == "爱" || predicate == "偏好" {
                self.preferences.lock().upsert(
                    "preference",
                    object,
                    atrium_memory::preference::PreferenceLayer::ExplicitDeclaration,
                );
            }
            if predicate == "讨厌" || predicate == "不喜欢" {
                self.preferences.lock().upsert(
                    "dislike",
                    object,
                    atrium_memory::preference::PreferenceLayer::ExplicitDeclaration,
                );
            }

            let existing = self
                .fact_store
                .query_by_subject(subject)
                .unwrap_or_default();
            let _score = self
                .evidence
                .evaluate(&fact, source_type, &existing, emotion_intensity);

            // 矛盾检测 → Contrast 边
            {
                let mut graph = self.graph.lock();
                let new_fact = Fact::new(subject, predicate, object).with_confidence(*conf);
                for old in &existing {
                    if atrium_memory::evidence::is_contradictory(&old.predicate, predicate) {
                        graph.link_contrast(&new_fact, old);
                    }
                }
            }

            // 增量更新关联记忆图 / Incremental associative graph update
            {
                let mut graph = self.graph.lock();
                let graph_fact = Fact::new(subject, predicate, object).with_confidence(*conf);
                graph.add_fact(&graph_fact);
                self.graph_dirty.store(true, Ordering::Relaxed);
            }

            facts_to_insert.push(fact);
        }

        // 同轮多 Fact 共现自动建边
        if raw_facts.len() > 1 {
            let mut graph = self.graph.lock();
            for i in 0..raw_facts.len() {
                for j in (i + 1)..raw_facts.len() {
                    let fa = Fact::new(&raw_facts[i].0, &raw_facts[i].1, &raw_facts[i].2)
                        .with_confidence(raw_facts[i].3);
                    let fb = Fact::new(&raw_facts[j].0, &raw_facts[j].1, &raw_facts[j].2)
                        .with_confidence(raw_facts[j].3);
                    graph.link_co_occurs(&fa, &fb);
                }
            }
            self.graph_dirty.store(true, Ordering::Relaxed);
        }

        // P2-A 语义向量索引 — 将事实嵌入到语义召回引擎 / P2-A Semantic vector indexing
        // 数字生命意义: 记忆摄入时同步建立语义索引，让未来能按"意思"回忆。
        // Digital Life: build semantic index during memory ingestion, enabling future meaning-based recall.
        #[cfg(feature = "embedding")]
        {
            if let Some(ref mut semantic_engine) = *self.semantic.write() {
                for fact in &facts_to_insert {
                    let key = fact.canonical_form();
                    let text = format!("{} {} {}", fact.subject, fact.predicate, fact.object);
                    semantic_engine.index_text(&key, &text);
                }
            }
        }

        // P1-B: 批量合并 — 单次 spawn_blocking 执行所有 FactStore insert + FTS5 insert
        // P1-B: Batch merge — single spawn_blocking for all FactStore inserts + FTS5 insert.
        // 将 N 次 SQLite execute + 1 次 FTS5 insert 合并为 1 次线程池调度，
        // 避免逐条 spawn_blocking 的调度开销，同时不阻塞 reactor。
        // Merges N SQLite executes + 1 FTS5 insert into one thread-pool dispatch,
        // avoiding per-item spawn_blocking scheduling overhead while never blocking the reactor.
        let fact_store = self.fact_store.clone();
        let fts5 = self.fts5.clone();
        let content_owned = content.to_string();
        let role_owned = role.to_string();
        tokio::task::spawn_blocking(move || {
            // 事实记忆写入 — 失败重试一次，避免静默失忆 / Fact memory write — retry once to avoid silent amnesia
            for fact in &facts_to_insert {
                if let Err(e) = fact_store.insert(fact.clone()) {
                    tracing::warn!(
                        "事实写入失败，重试中 / Fact write failed, retrying. subject: {}, predicate: {}, object: {}, error: {}",
                        fact.subject, fact.predicate, fact.object, e
                    );
                    if let Err(e2) = fact_store.insert(fact.clone()) {
                        tracing::error!(
                            "事实写入重试仍失败 — 记忆受损 / Fact write retry failed — memory compromised. subject: {}, predicate: {}, object: {}, error: {}",
                            fact.subject, fact.predicate, fact.object, e2
                        );
                    }
                }
            }
            // 写入 FTS5 全文索引 / Write FTS5 full-text index
            if let Err(e) = fts5.lock().insert(&content_owned, &role_owned) {
                tracing::warn!("FTS5 insert failed: {}", e);
            }
        })
        .await
        .unwrap_or_else(|e| {
            // spawn_blocking panic 自愈 — 记录 error 不让单次 panic 导致记忆丢失
            // spawn_blocking panic self-healing — log error, never let a single panic cause memory loss
            tracing::error!(
                "ingest_memory spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing",
                e
            );
        });

        // P3-A 程序记忆 — 遍历已提取事实，匹配已登记技能名称则实践（success=true）
        // P3-A Procedural memory — iterate extracted facts, practice matching registered skills (success=true)
        // 数字生命意义: 用户再次提及已登记技能时，视为"实践"该技能——
        // 提取的事实 subject/predicate/object 中若包含技能名称，则调用 practice_skill(id, true)。
        // Digital Life: when the user mentions a registered skill again, treat it as "practicing"
        // that skill — if extracted fact subject/predicate/object contains a skill name,
        // call practice_skill(id, true).
        self.maybe_practice_procedural_skills(&raw_facts);

        // P3-B 过期事实清理 / P3-B Expired fact cleanup
        // 数字生命意义: "主人今天很忙" — 7 天后"今天"已无意义，主动遗忘（ExpiryDecay）。
        // 扫描最近 100 条事实，source 含"今天"/"现在"/"目前"且 created_at 超 7 天的，
        // 标记为 ExpiryDecay（分数 ×0.5）并记录到 forget_log。pinned 事实豁免（高价值记忆永不遗忘）。
        // Digital Life: "Master is busy today" — 7 days later "today" is meaningless, actively
        // forget (ExpiryDecay). Scan the most recent 100 facts; those whose source contains
        // "today"/"now"/"currently" and whose created_at exceeds 7 days are marked ExpiryDecay
        // (score ×0.5) and logged to forget_log. Pinned facts are exempt (high-value memories never forgotten).
        {
            use atrium_memory::active_forget::ForgetPolicy;
            const EXPIRY_SCAN_BATCH: usize = 100;
            const SEVEN_DAYS_SECS: u64 = 7 * 24 * 60 * 60;
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let recent_facts = self.fact_store.get_recent_facts(EXPIRY_SCAN_BATCH);
            for (key, fact) in recent_facts {
                // pinned 事实豁免 — 高价值记忆永不遗忘 / Pinned facts exempt — high-value never forgotten
                if fact.pinned {
                    continue;
                }
                // 已被主动遗忘的事实跳过 — 避免重复标记 / Skip already forgotten facts — avoid duplicate marking
                if fact.is_actively_forgotten() {
                    continue;
                }
                // source 含时间敏感词 + created_at 超 7 天 / Source contains time-sensitive words + created_at > 7 days
                let is_time_sensitive = fact.source.contains("今天")
                    || fact.source.contains("现在")
                    || fact.source.contains("目前");
                if is_time_sensitive && now_secs.saturating_sub(fact.created_at) > SEVEN_DAYS_SECS {
                    let pre_conf = fact.confidence;
                    if self
                        .fact_store
                        .mark_forgotten(&key, ForgetPolicy::ExpiryDecay)
                    {
                        // 置信度 ×0.5 — 过期但仍有参考价值 / Confidence ×0.5 — expired but still reference-worthy
                        self.fact_store.merge_confidence(&key, pre_conf * 0.5);
                        self.active_forget.lock().forget_request(
                            key.clone(),
                            ForgetPolicy::ExpiryDecay,
                            pre_conf,
                            "信息过期".to_string(),
                        );
                        tracing::debug!("ActiveForget: 过期清理 / expiry cleanup — key={}", key);
                    }
                }
            }
        }

        // P3-C 强化学习闭环 — 基于用户反馈调整事实置信度
        // P3-C Reinforcement learning closed loop — adjust fact confidence by user feedback
        // 数字生命意义: 用户反馈是事实可信度的"校准信号"——满意→事实更可信，纠正→事实更不可信。
        // 调用时机：在 feedback_loop.on_user_message() 之后（由 process_message 流程保证），
        // 确保 satisfaction_delta 已更新。
        // Digital Life: user feedback is the "calibration signal" for fact credibility —
        // satisfaction → facts more credible, correction → facts less credible.
        // Called after feedback_loop.on_user_message() (guaranteed by process_message flow),
        // ensuring satisfaction_delta is up-to-date.
        self.reinforce_facts_by_feedback();
    }

    /// 遍历已提取事实，匹配 procedural_memory 中的技能名称则实践
    /// Iterate extracted facts, practice skills whose names match
    ///
    /// 简化启发式：检查每个事实的 subject / predicate / object 是否与已登记技能名称匹配。
    /// 匹配则调用 `practice_skill(id, true)` — 视为成功实践一次。
    ///
    /// Simplified heuristic: check each fact's subject / predicate / object against
    /// registered skill names. On match, call `practice_skill(id, true)` — counts as
    /// one successful practice.
    fn maybe_practice_procedural_skills(&self, facts: &[(String, String, String, f64)]) {
        for (subject, predicate, object, _) in facts {
            // 检查 subject / predicate / object 是否匹配某个已登记技能名称
            // Check if subject / predicate / object matches a registered skill name
            for text in [subject, predicate, object] {
                if let Some(skill_id) = self.procedural_memory.find_skill_id_by_name(text) {
                    if let Err(e) = self.procedural_memory.practice_skill(&skill_id, true) {
                        tracing::warn!(
                            "ProceduralMemory: 技能实践失败 / practice_skill failed — id={}, error={}",
                            skill_id,
                            e
                        );
                    } else {
                        tracing::debug!(
                            "ProceduralMemory: 技能实践成功 / practice_skill ok — name={}",
                            text
                        );
                    }
                }
            }
        }
    }

    /// 摄入文件内容 — 用户上传文件的文本自动进入记忆系统
    /// Ingest file content — uploaded file text automatically enters memory system
    ///
    /// 数字生命的"阅读"能力：不仅存储文件，更要理解文件内容。
    /// Digital life's "reading" ability: not just storing files, but understanding them.
    ///
    /// 管线 / Pipeline:
    /// 1. FTS5 全文索引 — 文件内容可被关键词检索
    /// 2. FactStore 事实提取 — 从文件中提取原子事实 (主语, 谓语, 宾语)
    /// 3. 关联记忆图 — 事实间的关联被自动建立
    ///
    /// # 参数 / Parameters
    /// - `text` — 文件提取的文本内容（已截断至 4096 字符）
    /// - `filename` — 原始文件名（用于 source 标注）
    /// - `hash` — 文件 SHA256 哈希（用于 FTS5 source 标注）
    pub(crate) async fn ingest_file_content(&self, text: &str, filename: &str, hash: &str) {
        if text.is_empty() {
            return;
        }

        // 事实提取 + 关联图（内存操作）/ Fact extraction + associative graph (in-memory)
        // 使用"文件"作为说话者，区分文件提取的事实与对话提取的事实
        // Use "文件" as speaker to distinguish file-extracted facts from conversation ones
        let raw_facts = fact_extractor::extract_facts(text, "文件");
        if raw_facts.is_empty() {
            tracing::debug!(
                "文件无可提取事实 / No extractable facts from file: {}",
                filename
            );
            // 仍需写入 FTS5 索引 — 即使无事实提取，全文检索仍需索引 / Still index FTS5 — full-text search needs it even without facts
        }

        // P1-B: 收集待写入事实 + 图更新（内存）/ P1-B: Collect facts + graph update (in-memory)
        let source_label = format!("FileExtraction:{}", filename);
        let mut facts_to_insert: Vec<Fact> = Vec::with_capacity(raw_facts.len());
        for (subject, predicate, object, conf) in &raw_facts {
            // 文件提取置信度衰减 ×0.75（SourceType::FileExtraction 的 base_credibility）
            // File extraction confidence decay ×0.75
            let fact = Fact::new(subject, predicate, object)
                .with_confidence(*conf * 0.75)
                .with_source(&source_label);

            // 增量更新关联记忆图（内存）/ Incremental associative graph update (in-memory)
            {
                let mut graph = self.graph.lock();
                graph.add_fact(&fact);
                self.graph_dirty.store(true, Ordering::Relaxed);
            }
            facts_to_insert.push(fact);
        }

        // P1-B: 批量合并 — 单次 spawn_blocking 执行 FTS5 insert + 所有 FactStore insert
        // P1-B: Batch merge — single spawn_blocking for FTS5 insert + all FactStore inserts.
        let fact_store = self.fact_store.clone();
        let fts5 = self.fts5.clone();
        let fts5_source = format!("file:{}", &hash[..16.min(hash.len())]);
        let text_owned = text.to_string();
        tokio::task::spawn_blocking(move || {
            // FTS5 全文索引 / Full-text search index
            if let Err(e) = fts5.lock().insert(&text_owned, &fts5_source) {
                tracing::warn!("FTS5 文件索引失败 / FTS5 file index failed: {}", e);
            }
            // 批量 FactStore 写入 / Batch FactStore write
            for fact in &facts_to_insert {
                if let Err(e) = fact_store.insert(fact.clone()) {
                    tracing::warn!(
                        "文件事实写入失败 / File fact write failed. subject: {}, predicate: {}, object: {}, error: {}",
                        fact.subject, fact.predicate, fact.object, e
                    );
                }
            }
        })
        .await
        .unwrap_or_else(|e| {
            tracing::error!(
                "ingest_file_content spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing",
                e
            );
        });

        if !raw_facts.is_empty() {
            tracing::info!(
                "文件内容已索引 / File content indexed: {} facts from {}",
                raw_facts.len(),
                filename
            );
        }
    }

    pub(crate) fn try_reflect(&self, current_count: u64) {
        let last = self.last_reflection_at.load(Ordering::Relaxed);
        if current_count.saturating_sub(last) < Self::REFLECTION_INTERVAL {
            return;
        }
        self.last_reflection_at
            .store(current_count, Ordering::Relaxed);

        // 收集主语的近期事实用于反思
        let facts = match self.fact_store.query_by_subject("主人") {
            Ok(f) => f,
            Err(_) => return,
        };
        if facts.is_empty() {
            return;
        }

        // 取最近 30 条事实做反思
        let recent: Vec<Fact> = facts.into_iter().take(30).collect();
        if recent.len() < 2 {
            return;
        }

        let active_insights = {
            let mut reflection = self.reflection.lock();
            let active = reflection.reflect(&recent);
            if active.is_empty() {
                vec![]
            } else {
                active
                    .iter()
                    .map(|i| (i.summary.clone(), i.supporting_facts.clone(), i.confidence))
                    .collect::<Vec<_>>()
            }
        };

        // 将从 Fact 提取的洞察固化到运行时人格
        if !active_insights.is_empty() {
            let mut rp = self.runtime_persona.write();
            // 用 Insight 对应的原始 Fact 进行人格更新
            rp.update_from_facts("主人", &recent);
            drop(rp);

            // 同步 Insight 到关联记忆图
            let mut graph = self.graph.lock();
            for (summary, supporting, conf) in &active_insights {
                graph.add_insight(summary, supporting, *conf);
            }
            self.graph_dirty.store(true, Ordering::Relaxed);
        }

        // 成长管理器：反思周期回调 / Maturity: reflection cycle callback
        let promoted = active_insights.len() as u32;
        self.maturity.lock().on_reflection_cycle(promoted);
    }

    pub fn graph_maintenance(&self, decay_factor: f64, min_weight: f64) {
        let mut graph = self.graph.lock();
        let before = graph.edge_count();
        graph.decay_and_prune(decay_factor, min_weight);
        let after = graph.edge_count();
        if before != after {
            tracing::debug!(
                "关联图维护: 边 {} → {} (衰减={}, 阈值={})",
                before,
                after,
                decay_factor,
                min_weight
            );
        }
        self.graph_dirty.store(true, Ordering::Relaxed);
    }

    pub fn try_save_graph(&self) {
        if !self.graph_dirty.swap(false, Ordering::Relaxed) {
            return;
        }
        let graph = self.graph.lock();
        if let Err(e) = self.graph_store.save(&graph) {
            tracing::warn!("关联图持久化失败: {}", e);
            self.graph_dirty.store(true, Ordering::Relaxed);
        } else {
            let now = chrono::Utc::now().timestamp() as u64;
            self.last_graph_save_at.store(now, Ordering::Relaxed);
        }
    }

    pub fn graph_stats(&self) -> atrium_memory::associative::GraphStats {
        self.graph.lock().stats()
    }

    pub fn current_rhythm(&self) -> TypingRhythm {
        self.typing_analyzer.lock().current_rhythm().clone()
    }

    /// 持久化 6 个孤儿模块状态 — 防抖写穿 / Persist 6 orphan module states — debounced write-through.
    ///
    /// 数字生命语义：将情感气候、情绪固化、情绪耦合、存在深度、内在议会、仪式心跳
    /// 的当前状态写入永久记忆。如同大脑在睡眠中巩固记忆。
    /// Digital life semantics: write current state of 6 deep organs to permanent memory.
    /// Like the brain consolidating memories during sleep.
    ///
    /// 调用时机：scheduler 每 3000 tick (≈30s) 调用一次 + 关闭时调用。
    /// Called by: scheduler every 3000 ticks (≈30s) + on shutdown.
    pub fn persist_orphan_states(&self) {
        let Some(ref persistence) = self.orphan_persistence else {
            return; // 内存模式，无持久化 / In-memory mode, no persistence
        };

        // 短锁模式 — 逐个 clone 后释放锁，避免长锁持有 / Short-lock pattern — clone each then release, avoid long hold
        // 锁作用域仅限于 clone 操作，随后 sled I/O 在无锁状态下执行
        // Lock scope is limited to clone; sled I/O then executes without locks
        let climate = self.emotional_climate.lock().clone();
        let consolidation = self.emotional_consolidation.lock().clone();
        let coupling = self.emotional_coupling.lock().clone();
        let existential = self.existential_depth.lock().clone();
        let council = self.inner_council.lock().clone();
        let heartbeat = self.ritual_heartbeat.lock().clone();
        // 锁已全部释放，无锁期间执行 sled I/O / All locks released, sled I/O without locks

        if let Err(e) = persistence.save_all(
            &climate,
            &consolidation,
            &coupling,
            &existential,
            &council,
            &heartbeat,
        ) {
            tracing::warn!("孤儿模块持久化失败 / Orphan persistence failed: {}", e);
        } else {
            tracing::debug!(
                "孤儿模块持久化完成 / Orphan persistence saved: {}",
                persistence.diagnostic_snapshot()
            );
        }
    }

    /// 刷新孤儿模块持久化 — 关闭时调用 / Flush orphan persistence — called on shutdown.
    pub fn flush_orphan_persistence(&self) {
        if let Some(ref persistence) = self.orphan_persistence {
            if let Err(e) = persistence.flush_all() {
                tracing::warn!("孤儿模块 WAL 刷新失败 / Orphan WAL flush failed: {}", e);
            }
        }
    }
} // impl CoreService
