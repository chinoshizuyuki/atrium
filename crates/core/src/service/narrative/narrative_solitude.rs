// SPDX-License-Identifier: MIT
//! 独处内在世界 — 人格漂移/孤独质量/原型/创造
//! Solitude Inner World — Personality Drift/Quality/Archetype/Creativity

use super::*;

impl CoreService {
    pub fn authentic_expression_prompt_fragment(&self) -> String {
        let (pleasure, arousal, dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            let cur = emo.current();
            (cur.pleasure, cur.arousal, cur.dominance)
        };
        let relation_ordinal: u8 = {
            let rel = self.relationship.read();
            rel.current_stage().ordinal()
        };

        let modulator = self.vulnerability.authentic_expression.lock();
        // 用当前情感状态调制一个基础模板 / Modulate a base template with current emotional state
        let base = "保持真实，适度展现脆弱";
        let context = atrium_memory::vulnerability_window::ConversationContext::DeepTalk;
        let modulated = modulator.modulate(
            base,
            pleasure,
            arousal,
            dominance,
            relation_ordinal,
            context,
        );
        if modulated == base {
            String::new()
        } else {
            format!("[真实表达/AuthenticExpression] {}", modulated)
        }
    }

    pub fn personality_drift_tick(&self) {
        let mut drift = self.solitude.drift.lock();
        // 简化：用默认 SolitudePattern 推进 / Simplified: advance with default pattern
        let pattern = atrium_memory::personality_drift::SolitudePattern::default();
        drift.tick(&pattern);
    }

    pub fn solitude_quality_tick(&self) {
        // 提取当前情感状态 / Extract current emotional state
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };

        let mut quality = self.solitude.quality.lock();

        // 从品质追踪器获取中间量 / Get intermediates from quality tracker
        let novelty = quality.quality.novelty();
        let emotional_range = quality.quality.emotional_range();
        let repeat_rate = quality.quality.repeat_rate();
        let negative_emotion = quality.quality.negative_emotion_intensity();
        let loop_count = quality.quality.loop_count();
        let positive_emotion = quality.quality.positive_emotion_mean();

        // 情感正向且新颖度高 → 反思路径 / Positive + novel → reflective path
        if pleasure > 0.0 || novelty > 0.5 {
            quality.quality.update_reflective(novelty, emotional_range);
        }
        // 情感负向且重复率高 → 反刍路径 / Negative + repetitive → ruminative path
        if pleasure < 0.0 || repeat_rate > 0.3 {
            quality
                .quality
                .update_ruminative(repeat_rate, negative_emotion, loop_count);
        }
        // 创造性维度始终更新 / Creative dimension always updates
        quality
            .quality
            .update_creative(novelty, positive_emotion.max(0.0));

        // 更新辩论强度 / Update debate intensity
        let diversity = quality.dialogue.perspective_diversity();
        let engagement = emotional_range + positive_emotion;
        quality
            .dialogue
            .update_debate_intensity(diversity, engagement.clamp(0.0, 1.0));
    }

    pub fn solitude_quality_on_thought(&self, content: &str, timestamp: i64) {
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };
        let mut quality = self.solitude.quality.lock();
        quality.on_thought(content, pleasure, timestamp);
    }

    pub fn solitude_archetype_tick(&self) {
        // 从情感状态推断独处特征 / Infer solitude features from emotional state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        // 消息计数作为活跃度代理 / Message count as activity proxy
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let activity = (msg_count as f64).min(100.0) / 100.0;

        // 用情感 PAD 和活跃度构建特征 / Build features from PAD and activity
        // 简化策略：避免 NLP 依赖，用情感状态作为代理变量
        // Simplified: use emotional state as proxy, no NLP dependency
        let features = atrium_memory::solitude_archetype::SolitudeFeatures {
            analytical: (0.5 + dominance * 0.3).clamp(0.0, 1.0),
            divergent: (0.5 + arousal * 0.3).clamp(0.0, 1.0),
            reflective: (0.5 + (1.0 - activity) * 0.3).clamp(0.0, 1.0),
            creative: (0.5 + pleasure * 0.3).clamp(0.0, 1.0),
            observational: (0.5 + (1.0 - arousal.abs()) * 0.2).clamp(0.0, 1.0),
            reminiscent: (0.5 + (1.0 - activity) * 0.2).clamp(0.0, 1.0),
        };

        let mut tracker = self.solitude.archetype.lock();
        tracker.update(&features);
    }

    pub fn solitude_creativity_tick(&self, idle_secs: i64) {
        // 从情感状态构建认知松弛度 / Build cognitive relaxation from emotional state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let activity = (msg_count as f64).min(100.0) / 100.0;

        let relaxation = atrium_memory::solitude_creativity::CognitiveRelaxation {
            cognitive_load: activity.clamp(0.0, 1.0),
            emotional_calmness: (0.5 + pleasure * 0.3).clamp(0.0, 1.0),
            time_abundance: (idle_secs as f64 / 3600.0).clamp(0.0, 1.0),
            purposelessness: (0.5 + (1.0 - activity) * 0.3).clamp(0.0, 1.0),
            physical_quietness: (0.5 + (1.0 - arousal.abs()) * 0.3).clamp(0.0, 1.0),
        };

        let mut creativity = self.solitude.creativity.lock();
        let potential = creativity.creative_potential(&relaxation, idle_secs);

        // 当创造潜势高且用户不活跃时，自动记录一条创意 / Auto-record when potential is high
        if potential > 0.5 && activity < 0.3 {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            creativity.record(atrium_memory::solitude_creativity::CreativeOutput {
                content: String::new(), // 空内容——代表"无具体想法但有创造氛围"
                novelty: potential,
                depth: (0.5 + dominance * 0.2).clamp(0.0, 1.0),
                connectivity: 0.3, // 默认连接度 / Default connectivity
                timestamp: now_epoch,
                solitude_duration: idle_secs,
            });
        }
    }

    pub fn personality_drift_prompt_fragment(&self) -> String {
        let drift = self.solitude.drift.lock();
        drift.prompt_injection()
    }

    pub fn solitude_archetype_prompt_fragment(&self) -> String {
        let tracker = self.solitude.archetype.lock();
        tracker.prompt_injection()
    }

    pub fn solitude_creativity_prompt_fragment(&self) -> String {
        let creativity = self.solitude.creativity.lock();
        creativity.prompt_injection()
    }

    pub fn solitude_quality_prompt_fragment(&self) -> String {
        let quality = self.solitude.quality.lock();
        quality.to_prompt_hint()
    }
} // impl CoreService
