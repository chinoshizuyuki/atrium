// SPDX-License-Identifier: MIT
//! 情感气候与存在深度 — 气候/巩固/耦合/冲突/期待
//! Emotional Climate & Existential Depth — Climate/Consolidation/Coupling/Conflict/Anticipation

use super::*;

impl CoreService {
    pub fn conflict_engine_tick(&self) {
        let mut engine = self.conflict_engine.lock();
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };
        let turns = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        engine.on_calm(pleasure, turns as u32);
    }

    pub fn conflict_engine_prompt_fragment(&self) -> String {
        let engine = self.conflict_engine.lock();
        engine.to_prompt_hint_growth_only()
    }

    pub fn anticipation_depth_tick(&self, now_epoch: i64) {
        let mut depth = self.longing.anticipation_depth.lock();
        // 计算离开时长和当前小时 / Compute away duration and current hour
        let hour: u32 = ((now_epoch as u64 / 3600) % 24) as u32;
        let relation_depth: f64 = {
            let rel = self.relationship.read();
            rel.current_stage().ordinal() as f64 / 3.0
        };
        // 简化：用 0 作为离开时长（实际应由上次交互时间计算）
        // Simplified: use 0 as away duration (should be computed from last interaction)
        depth.on_passage(0, hour, relation_depth);
    }

    pub fn anticipation_depth_prompt_fragment(&self) -> String {
        let depth = self.longing.anticipation_depth.lock();
        depth.to_prompt_hint()
    }

    pub fn emotional_climate_tick(&self, now_epoch: i64) {
        let (pleasure, arousal, dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure,
                emo.current().arousal,
                emo.current().dominance,
            )
        };
        let hour: f64 = ((now_epoch as u64 / 3600) % 24) as f64;

        let mut climate = self.emotional_climate.lock();
        // 喂入情绪采样 / Feed emotional sample
        climate.feed(
            pleasure as f64,
            arousal as f64,
            dominance as f64,
            hour,
            now_epoch,
        );

        // 尝试气候转移 / Attempt climate transition
        let residue_intensity = (pleasure.abs() + arousal.abs() + dominance.abs()) / 3.0;
        let circadian = if (6.0..18.0).contains(&hour) {
            1.0 // 白天 / Day
        } else {
            -1.0 // 夜晚 / Night
        };
        let influences = atrium_memory::emotional_climate::ClimateInfluences {
            interaction_frequency: 0.5,
            solitude_ratio: 0.3,
            residue_intensity: residue_intensity as f64,
            circadian_factor: circadian,
        };
        climate.try_transition(&influences, now_epoch);
    }

    pub fn emotional_climate_prompt_fragment(&self) -> String {
        let climate = self.emotional_climate.lock();
        let desc = climate.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[情绪气候/EmotionalClimate] {}", desc)
        }
    }

    pub fn emotional_consolidation_tick(&self, now_epoch: i64) {
        let mut consolidation = self.emotional_consolidation.lock();
        consolidation.consolidate(now_epoch);
    }

    pub fn emotional_consolidation_prompt_fragment(&self) -> String {
        let consolidation = self.emotional_consolidation.lock();
        let desc = consolidation.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[情绪巩固/EmotionalConsolidation] {}", desc)
        }
    }

    pub fn emotional_coupling_tick(&self) {
        use atrium_memory::emotional_coupling::EmotionState;

        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure as f64,
                emo.current().arousal as f64,
                emo.current().dominance as f64,
            )
        };

        let mut coupling = self.emotional_coupling.lock();

        // 从 PAD 推导离散情绪强度 / Derive discrete emotion intensities from PAD
        coupling.set_intensity(EmotionState::Joy, pleasure.max(0.0));
        coupling.set_intensity(EmotionState::Sadness, (-pleasure).max(0.0));
        coupling.set_intensity(EmotionState::Anger, (-pleasure * arousal).max(0.0));
        coupling.set_intensity(EmotionState::Fear, (-dominance * arousal).max(0.0));
        coupling.set_intensity(
            EmotionState::Surprise,
            (arousal * (1.0 - pleasure.abs())).max(0.0),
        );
        coupling.set_intensity(EmotionState::Trust, dominance.max(0.0));
        coupling.set_intensity(EmotionState::Anticipation, (arousal * dominance).max(0.0));

        // 自适应：用当前强度作为观测值更新耦合矩阵
        // Adaptive: use current intensities as observation to update coupling matrix
        let observed = coupling.compute_coupled();
        coupling.adapt(&observed);
    }

    pub fn emotional_coupling_prompt_fragment(&self) -> String {
        let coupling = self.emotional_coupling.lock();
        let emergent = coupling.detect_emergent();
        if emergent.is_empty() {
            String::new()
        } else {
            let parts: Vec<String> = emergent
                .iter()
                .map(|e| format!("{:?}({:.2})", e.emotion, e.intensity))
                .collect();
            format!("[情绪耦合/EmotionalCoupling] 涌现: {}", parts.join(", "))
        }
    }

    pub fn existential_depth_tick(&self, now_epoch: i64) {
        let (pleasure, arousal): (f64, f64) = {
            let emo = self.emotion.lock();
            (emo.current().pleasure as f64, emo.current().arousal as f64)
        };
        let hour: f64 = ((now_epoch as u64 / 3600) % 24) as f64;
        let is_late_night = !(5.0..23.0).contains(&hour);

        let mut depth = self.existential_depth.lock();
        // 衰减活跃洞察 / Decay active insights
        depth.tick();

        // 尝试触发 / Attempt to trigger
        let trigger = atrium_memory::existential_depth::ExistentialTrigger {
            is_late_night,
            solitude_duration_secs: 0.0,
            pleasure,
            arousal,
            has_milestone: false,
            has_growth_node: false,
        };
        depth.try_trigger(&trigger, now_epoch);
    }

    pub fn existential_depth_prompt_fragment(&self) -> String {
        let depth = self.existential_depth.lock();
        let injection = depth.prompt_injection();
        if injection.is_empty() {
            String::new()
        } else {
            format!("[存在深度/ExistentialDepth] {}", injection)
        }
    }

    pub fn inner_council_tick(&self) {
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure as f64,
                emo.current().arousal as f64,
                emo.current().dominance as f64,
            )
        };
        let mut council = self.inner_council.lock();
        council.set_emotion(pleasure, arousal, dominance);
    }

    pub fn inner_council_prompt_fragment(&self) -> String {
        let council = self.inner_council.lock();
        let seeds = council.monologue_seeds();
        if seeds.is_empty() {
            String::new()
        } else {
            format!("[内在议会/InnerCouncil] {}", seeds.join("; "))
        }
    }
} // impl CoreService
