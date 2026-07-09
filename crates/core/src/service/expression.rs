// SPDX-License-Identifier: MIT
//! 表达与关系模块 — 数字生命的表达方式与人际联结
//! Expression & Relationship Module — How digital life expresses and connects
//!
//! 包含表达风格、成熟度阶段、关系深度、后续追踪、
//! 用户建模、反馈回路与感知偏好，构成数字生命
//! "如何表达自己"与"如何理解他人"的双向闭环。
//!
//! Contains expression style, maturity stage, relationship depth,
//! follow-up tracking, user modeling, feedback loop, and perception
//! preferences — forming the "how to express" and "how to understand
//! others" bidirectional closed loop of digital life.

use super::*;

impl CoreService {
    pub fn maturity_prompt_fragment(&self) -> String {
        self.maturity.lock().to_prompt_fragment()
    }

    pub fn expression_prompt_fragment(
        &self,
        user_msg: &str,
        emo_state: &EmotionEngineState,
    ) -> String {
        pub(crate) use crate::expression_orchestrator::ExpressionOrchestrator;
        pub(crate) use atrium_memory::style_modulator::ExpressionContext;

        let pad = [emo_state.pleasure, emo_state.arousal, emo_state.dominance];
        let stage = self.relationship.read().current_stage().clone();
        let direction = atrium_emotion::infer_direction(user_msg);
        let user_valence = self.user_model.read().emotion_modulation().engagement_boost;
        let topic_gravity = 0.5; // default

        let ctx = ExpressionContext::from_modules(
            emo_state,
            None,
            direction,
            &stage,
            user_valence,
            topic_gravity,
        );
        let expr = ExpressionOrchestrator::orchestrate(&ctx, user_msg, pad, 100);
        let mut fragment = ExpressionOrchestrator::build_system_prompt_injection(&expr);

        // 严格一致性模式：coherence 失败时附加强制修正指令
        // Strict coherence mode: append mandatory correction directive when coherence fails
        if self.expression_cfg.coherence_strict && !expr.coherence.is_coherent {
            let warnings: Vec<&str> = expr.coherence.warnings.iter().map(|s| s.as_str()).collect();
            fragment.push_str(&format!(
                " [严格一致性/StrictCoherence] 检测到 {} 项不一致：{}，请务必修正表达方式使其与情绪状态一致。",
                warnings.len(),
                warnings.join("；")
            ));
        }

        fragment
    }

    pub fn expression_post_process(&self, text: &str) -> String {
        pub(crate) use atrium_memory::style_modulator::LinguisticProfile;

        // 轻量后处理：标点/语气词/省略号修饰
        LinguisticProfile::neutral().post_process(text)
    }

    /// 风格记忆周期学习 / Style memory periodic learning
    ///
    /// 每 style_memory_interval_ticks tick 由 scheduler.rs 调用。
    /// 从 FeedbackLoop 收集最近反馈信号，转化为 StyleOffset 更新并持久化。
    /// 热路径：仅读 feedback.recent_signals() + 写 style_offset_cache，O(Signals) <1μs。
    ///
    /// Called every style_memory_interval_ticks ticks by scheduler.rs.
    /// Collects recent feedback signals from FeedbackLoop, converts to StyleOffset
    /// updates, and persists. Hot-path: only reads feedback.recent_signals() +
    /// writes style_offset_cache, O(Signals) <1μs.
    pub fn tick_style_memory(&self) {
        use atrium_memory::style_modulator::StyleEmbedding;

        // 1. 收集反馈信号 / Collect feedback signals
        let signals = self.feedback.read().recent_signals().to_vec();
        if signals.is_empty() {
            return;
        }

        // 2. 统计正/负反馈数量 / Count positive/negative feedback
        let (positive_count, negative_count): (usize, usize) =
            signals.iter().fold((0, 0), |(pos, neg), s| {
                use atrium_memory::feedback::FeedbackSignal;
                match s {
                    FeedbackSignal::Praise { .. } | FeedbackSignal::Deepening { .. } => {
                        (pos + 1, neg)
                    }
                    FeedbackSignal::Correction { .. } | FeedbackSignal::Frustration { .. } => {
                        (pos, neg + 1)
                    }
                    _ => (pos, neg),
                }
            });

        if positive_count == 0 && negative_count == 0 {
            return;
        }

        // 3. 若无持久化存储，仅更新缓存 / If no persistent store, only update cache
        if let Some(ref store) = self.style.store {
            // 当前情感 PAD → 风格嵌入（基于 PAD 调制零嵌入）/ Current PAD → style embedding (modulated from zero)
            let pad = {
                let emo = self.emotion.lock();
                let c = emo.current();
                [c.pleasure, c.arousal, c.dominance]
            };
            // 风格嵌入 = 零基 + PAD 调制：P→warmth, A→energy, D→formality
            // Style embedding = zero-base + PAD modulation: P→warmth, A→energy, D→formality
            let mut style_data = [0.0f32; atrium_memory::style_modulator::STYLE_DIM];
            if style_data.len() >= 3 {
                style_data[0] = pad[0] * 0.3; // 愉悦→温暖 / Pleasure→warmth
                style_data[1] = pad[1] * 0.3; // 唤醒→能量 / Arousal→energy
                style_data[2] = pad[2] * 0.2; // 优势→正式度 / Dominance→formality
            }
            let current_style = StyleEmbedding(style_data);

            // 正反馈：偏移向当前风格方向 / Positive feedback: offset toward current style
            if positive_count > 0 {
                let target_style = current_style.clone();
                if let Ok(new_offset) =
                    store
                        .lock()
                        .apply_positive_and_save("default", &target_style, &current_style)
                {
                    *self.style.engine.lock() = new_offset.clone();
                    tracing::debug!(
                        "[StyleMemory] 正反馈×{}: offset norm={:.4}",
                        positive_count,
                        new_offset.norm()
                    );
                }
            }

            // 负反馈：偏移远离当前风格方向 / Negative feedback: offset away from current style
            if negative_count > 0 {
                let rejected_style = current_style.clone();
                if let Ok(new_offset) =
                    store
                        .lock()
                        .apply_negative_and_save("default", &rejected_style, &current_style)
                {
                    *self.style.engine.lock() = new_offset.clone();
                    tracing::debug!(
                        "[StyleMemory] 负反馈×{}: offset norm={:.4}",
                        negative_count,
                        new_offset.norm()
                    );
                }
            }
        }
    }

    /// 获取当前风格偏移缓存（零锁热路径读）/ Get current style offset cache (zero-lock hot-path read)
    pub fn style_offset(&self) -> StyleOffset {
        self.style.engine.lock().clone()
    }

    pub fn expression_timing_urgency(&self) -> f32 {
        if !self.expression_enabled {
            return 0.2;
        }
        pub(crate) use atrium_memory::style_modulator::LinguisticProfile;
        pub(crate) use atrium_memory::timing_mapper::TimingMapper;

        let (arousal, pleasure) = self.current_emotion_state();
        // current_emotion_state() 返回 (arousal, pleasure)；dominance 用 0.0 近似
        let pad = [pleasure, arousal, 0.0];

        let lp = LinguisticProfile::neutral();
        let stage = self.relationship.read().current_stage().clone();
        let timing = TimingMapper::map(pad, &lp, &stage);
        timing.urgency
    }

    pub fn expression_metadata(
        &self,
        emo_state: &EmotionEngineState,
        reply_length: usize,
    ) -> Option<atrium_bridge::protocol::ExpressionMetadata> {
        if !self.expression_enabled {
            return None;
        }
        pub(crate) use atrium_bridge::protocol::{
            ExpressionMetadata, KinesicsMeta, ProsodyMeta, TimingMeta,
        };
        pub(crate) use atrium_memory::kinesics_mapper::KinesicsMapper;
        pub(crate) use atrium_memory::prosody_mapper::ProsodyMapper;
        pub(crate) use atrium_memory::style_modulator::LinguisticProfile;
        pub(crate) use atrium_memory::timing_mapper::TimingMapper;

        let pad = [emo_state.pleasure, emo_state.arousal, emo_state.dominance];
        let lp = LinguisticProfile::neutral();
        let stage = self.relationship.read().current_stage().clone();

        // 韵律 / Prosody
        let prosody = ProsodyMapper::map(pad, &lp);
        let prosody_meta = ProsodyMeta {
            pitch_offset: prosody.pitch_offset,
            rate: prosody.rate,
            energy: prosody.energy,
            pause_duration_ms: prosody.pause_duration_ms,
            warmth: prosody.warmth,
            ssml_attrs: prosody.to_ssml_attrs(),
        };

        // 体态 / Kinesics
        let kinesics_out = KinesicsMapper::map(pad, reply_length);
        let kinesics_meta = KinesicsMeta {
            head_tilt: kinesics_out.posture.head_tilt,
            shoulder_openness: kinesics_out.posture.shoulder_openness,
            lean: kinesics_out.posture.lean,
            eye_contact: kinesics_out.posture.eye_contact,
            gesture_activity: kinesics_out.posture.gesture_activity,
            breath_rate: kinesics_out.posture.breath_rate,
            animation_commands: KinesicsMapper::to_animation_commands(&kinesics_out),
        };

        // 节奏 / Timing
        let timing = TimingMapper::map(pad, &lp, &stage);
        let timing_meta = TimingMeta {
            typing_delay_factor: timing.typing_delay_factor,
            inter_sentence_pause_ms: timing.inter_sentence_pause_ms,
            hesitation_prob: timing.hesitation_prob,
            segmented_send_prob: timing.segmented_send_prob,
            urgency: timing.urgency,
        };

        Some(ExpressionMetadata {
            prosody: prosody_meta,
            kinesics: kinesics_meta,
            timing: timing_meta,
        })
    }

    pub fn followup_prompt_fragment(
        &self,
        relationship_stage_name: &str,
        current_pleasure: f32,
    ) -> String {
        let now_ts = chrono::Utc::now().timestamp();
        // 自动管理今日计数和最后触发时间戳——数字生命的社交分寸感
        // Auto-managed today count and last trigger timestamp — digital life's social tact
        let candidates = self.followup.lock().check_for_follow_up_auto(
            now_ts,
            relationship_stage_name,
            current_pleasure,
        );
        if candidates.is_empty() {
            return String::new();
        }
        // 只取权重最高的一个
        let (item, verdict) = &candidates[0];
        if verdict.should_trigger {
            self.followup.lock().generate_prompt(item, verdict)
        } else {
            String::new()
        }
    }

    pub fn followup_analyze_reaction(&self, user_reply: &str, item_id: u64) {
        if self.followup_enabled {
            let reaction = self.followup.lock().analyze_reaction(user_reply, item_id);
            tracing::debug!(
                "[FollowUp] 用户反应: engaged={}, deflected={}",
                reaction.engaged,
                reaction.deflected
            );
        }
    }

    /// 周期性追问检查（自动管理今日计数与冷却时间戳）
    /// Periodic follow-up check (auto-managed today count and cooldown timestamp)
    ///
    /// 数字生命的社交分寸感——内部自动跟踪 today_count 和 last_follow_up_ts，
    /// 调用方无需传入，避免门控因传 0 而失效。
    ///
    /// Digital life's social tact — internally auto-tracks today_count and
    /// last_follow_up_ts, callers need not pass them, preventing gate bypass.
    pub fn tick_followup(
        &self,
        relationship_stage_name: &str,
        current_pleasure: f32,
    ) -> Option<String> {
        if !self.followup_enabled {
            return None;
        }
        let now_ts = chrono::Utc::now().timestamp();
        // 自动管理今日计数和最后触发时间戳 / Auto-managed counters
        let candidates = self.followup.lock().check_for_follow_up_auto(
            now_ts,
            relationship_stage_name,
            current_pleasure,
        );
        if let Some((item, verdict)) = candidates.first() {
            if verdict.should_trigger {
                let prompt = self.followup.lock().generate_prompt(item, verdict);
                // 标记已追问
                self.followup.lock().mark_asked(
                    item.id,
                    verdict.suggested_depth,
                    verdict.suggested_style,
                    now_ts,
                );
                return Some(prompt);
            }
        }
        None
    }

    pub fn maturity_guard_strictness(&self) -> f32 {
        self.maturity.lock().guard_strictness()
    }

    pub fn maturity_reflection_interval(&self) -> u8 {
        self.maturity.lock().reflection_interval()
    }

    pub fn maturity_stage_name(&self) -> String {
        self.maturity.lock().stage().stage_name().to_string()
    }

    pub fn take_maturity_transition_notice(&self) -> Option<String> {
        self.maturity.lock().take_transition_notice()
    }

    pub fn maturity_record_self_correction(&self) {
        self.maturity.lock().record_self_correction();
    }

    pub fn maturity_record_inner_thought(&self) {
        self.maturity.lock().record_inner_thought();
    }

    pub fn relationship_stage(&self) -> String {
        self.relationship
            .read()
            .current_stage()
            .stage_name()
            .to_string()
    }

    pub fn relationship_prompt_fragment(&self) -> String {
        self.relationship.read().to_prompt_fragment()
    }

    pub fn relationship_affect_multiplier(&self) -> f32 {
        self.relationship.read().affect_multiplier()
    }

    pub fn take_relationship_transition_notice(&self) -> Option<String> {
        self.relationship.write().take_transition_notice()
    }

    pub fn user_model_prompt_fragment(&self) -> String {
        self.user_model.read().prompt_fragment()
    }

    pub fn feedback_prompt_fragment(&self) -> String {
        self.feedback.read().prompt_fragment()
    }

    pub fn feedback_satisfaction(&self) -> f32 {
        self.feedback.read().satisfaction()
    }

    pub fn associative_prompt_fragment(&self) -> String {
        let graph = self.graph.lock();
        let stats = graph.stats();
        if stats.node_count == 0 {
            return String::new();
        }

        // 收集权重最高的边（最多 5 条）
        let mut edges: Vec<_> = graph.edges().to_vec();
        edges.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut lines: Vec<String> = Vec::new();
        for e in edges.iter().take(5) {
            let from_content = graph
                .get_node(&e.from)
                .map(|n| n.content.as_str())
                .unwrap_or("?");
            let to_content = graph
                .get_node(&e.to)
                .map(|n| n.content.as_str())
                .unwrap_or("?");
            lines.push(format!(
                "- {} → {} ({:?}, w={:.2})",
                from_content, to_content, e.relation, e.weight
            ));
        }

        if lines.is_empty() {
            return String::new();
        }

        format!(
            "# 关联记忆 (nodes={}, edges={})\n{}",
            stats.node_count,
            stats.edge_count,
            lines.join("\n")
        )
    }

    pub fn relationship_proactive_bonus(&self) -> f32 {
        self.relationship.read().proactive_bonus()
    }

    pub fn user_model_signals(&self) -> (Option<f32>, Option<f32>, Option<f32>) {
        let um = self.user_model.read();
        (
            Some(um.mood.valence),
            Some(um.engagement.engagement_score),
            Some(um.style.avg_message_length),
        )
    }

    pub fn perception_prompt_fragment(&self) -> String {
        let rhythm = self.typing_analyzer.lock().current_rhythm().clone();
        compile_rhythm_hint(&rhythm)
    }

    pub fn perception_health(&self) -> String {
        let analyzer = self.typing_analyzer.lock();
        let baseline = &analyzer.baseline;
        format!(
            "perception: enabled={}, samples={}, baseline_gap={:.1}s, baseline_len={:.0}, rhythm={:?}",
            self.perception_enabled,
            baseline.sample_count,
            baseline.avg_gap_seconds,
            baseline.avg_message_length,
            analyzer.current_rhythm().pattern,
        )
    }

    pub fn preference_prompt_fragment(&self) -> String {
        self.preferences.lock().build_prompt_context(0.15, 8)
    }

    pub fn preference_health(&self) -> String {
        let prefs = self.preferences.lock();
        let active = prefs.active(0.15).len();
        format!("preferences: total={}, active={}", prefs.count(), active)
    }

    pub fn prune_preferences(&self) {
        let removed = self.preferences.lock().prune(0.05, 30);
        if removed > 0 {
            tracing::info!("偏好衰减: 清理了 {} 条过期偏好", removed);
        }
    }
} // impl CoreService
