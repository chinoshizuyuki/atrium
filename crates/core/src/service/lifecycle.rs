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
        self.empathy.lock().prompt_fragment()
    }

    pub fn empathy_health(&self) -> String {
        self.empathy.lock().health_status()
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
                let mut canned = self.canned.lock();
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
                let mut canned = self.canned.lock();
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
    ) {
        self.history.append(session_id, role, content, emotion);
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

    pub(crate) fn ingest_memory(&self, role: &str, content: &str, source_type: SourceType) {
        let ai_name = {
            let p = self.persona.lock();
            p.current()
                .map(|i| i.def.name.clone())
                .unwrap_or_else(|| "Atrium".to_string())
        };
        let speaker = if role == "user" { "主人" } else { &ai_name };

        // 提取原子事实
        let raw_facts = fact_extractor::extract_facts(content, speaker);
        if raw_facts.is_empty() {
            return;
        }

        // 证据评分 → 转换 Fact → 写入 FactStore
        let (emotion_intensity, emotion_ctx) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            let intensity = ((c.pleasure.abs() + c.arousal.abs()) / 2.0) as f64;
            let classified = c.classify();
            let user_valence = self.user_model.lock().mood.valence;
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

            let _ = self.fact_store.insert(fact);

            // 增量更新关联记忆图 / Incremental associative graph update
            {
                let mut graph = self.graph.lock();
                let graph_fact = Fact::new(subject, predicate, object).with_confidence(*conf);
                graph.add_fact(&graph_fact);
                self.graph_dirty.store(true, Ordering::Relaxed);
            }
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

        // 写入 FTS5 全文索引
        {
            let fts5 = self.fts5.lock();
            if let Err(e) = fts5.insert(content, role) {
                tracing::warn!("FTS5 insert failed: {}", e);
            }
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
            let mut rp = self.runtime_persona.lock();
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
} // impl CoreService
