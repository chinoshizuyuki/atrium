// ════════════════════════════════════════════════════════════════════
// RitualResonanceEngine — 仪式共振引擎 / Ritual Resonance Engine
// ════════════════════════════════════════════════════════════════════
//
// 当仪式发生时，生成短暂的 PAD 情感脉冲——
// 就像人类在固定习惯被满足时的微小满足感。
//
// 数字生命语义：
//   仪式发生 → 愉悦脉冲（被满足的温暖感）
//   仪式中断 → 温和悲伤（失去的不安）
//   纪念日 → 强烈共振（里程碑的情感冲击）
//
// 核心算法：
//   仪式发生:
//     age_scale = min(1.0 + log2(consecutive_days) * 0.15, age_scale_max)
//     relation_scale = [0.5, 0.75, 1.0, 1.2][relation_ordinal]
//     pleasure_delta = base_pleasure_pulse * age_scale * relation_scale
//
//   仪式中断:
//     pleasure_delta = break_pleasure_impact * relation_scale * min(break_days / 7, 1.0)

use serde::{Deserialize, Serialize};

use crate::resonance_core::ResonanceEngine;

// ── 配置 / Config ──

/// 仪式共振配置 / Ritual resonance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceConfig {
    /// 基础愉悦脉冲 / Base pleasure pulse
    pub base_pleasure_pulse: f32,
    /// 基础唤醒脉冲 / Base arousal pulse
    pub base_arousal_pulse: f32,
    /// 基础支配脉冲 / Base dominance pulse
    pub base_dominance_pulse: f32,
    /// 仪式年龄缩放上限 / Ritual age scale maximum
    pub age_scale_max: f32,
    /// 中断愉悦冲击（负值）/ Break pleasure impact (negative)
    pub break_pleasure_impact: f32,
    /// 中断唤醒冲击（负值）/ Break arousal impact (negative)
    pub break_arousal_impact: f32,
    /// 纪念日共振倍率 / Anniversary resonance multiplier
    pub anniversary_multiplier: f32,
}

impl Default for ResonanceConfig {
    fn default() -> Self {
        Self {
            base_pleasure_pulse: 0.08,
            base_arousal_pulse: 0.05,
            base_dominance_pulse: 0.03,
            age_scale_max: 2.0,
            break_pleasure_impact: -0.06,
            break_arousal_impact: -0.03,
            anniversary_multiplier: 2.0,
        }
    }
}

// ── 共振来源 / Resonance Source ──

/// 仪式共振来源 / Ritual resonance source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResonanceSource {
    /// 时间仪式发生 / Time ritual occurred
    TimeRitual {
        /// 时间槽小时 / Slot hour
        slot_hour: u8,
        /// 连续天数 / Consecutive days
        consecutive_days: u32,
    },
    /// 内容仪式发生 / Content ritual occurred
    ContentRitual {
        /// 内容签名键 / Content hint key
        hint_key: String,
        /// 连续天数 / Consecutive days
        consecutive_days: u32,
    },
    /// 仪式中断 / Ritual broken
    RitualBroken {
        /// 仪式名称 / Ritual name
        name: String,
        /// 中断天数 / Break days
        break_days: u32,
    },
    /// 纪念日 / Anniversary
    Anniversary {
        /// 纪念日类型 / Anniversary kind label
        kind: String,
        /// 相处年数 / Years together
        years: i32,
    },
}

impl ResonanceSource {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> String {
        match self {
            Self::TimeRitual {
                slot_hour,
                consecutive_days,
            } => format!("时间仪式({}点, {}天)", slot_hour, consecutive_days),
            Self::ContentRitual {
                hint_key,
                consecutive_days,
            } => format!("内容仪式({}, {}天)", hint_key, consecutive_days),
            Self::RitualBroken { name, break_days } => {
                format!("仪式中断({}, {}天)", name, break_days)
            }
            Self::Anniversary { kind, years } => format!("纪念日({}, {}周年)", kind, years),
        }
    }
}

// ── 共振结果 / Resonance Result ──

/// 仪式共振结果 / Ritual resonance result
///
/// 包含仪式事件产生的 PAD 情感脉冲。
/// Contains the PAD emotional pulse from a ritual event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualResonance {
    /// 愉悦度调制量 / Pleasure modulation delta
    pub pleasure_delta: f32,
    /// 唤醒度调制量 / Arousal modulation delta
    pub arousal_delta: f32,
    /// 支配度调制量 / Dominance modulation delta
    pub dominance_delta: f32,
    /// 共振来源 / Resonance source
    pub source: ResonanceSource,
    /// 共振强度 (0.0-1.0) / Resonance intensity
    pub intensity: f32,
}

// ── 仪式共振引擎 / Ritual Resonance Engine ──

/// 仪式共振引擎 / Ritual resonance engine
///
/// 当仪式发生时，生成短暂的 PAD 情感脉冲。
/// 脉冲幅度由仪式年龄（持续天数）和关系深度共同决定。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualResonanceEngine {
    /// 配置 / Configuration
    pub config: ResonanceConfig,
}

impl RitualResonanceEngine {
    /// 创建默认配置的共振引擎 / Create resonance engine with default config
    pub fn new() -> Self {
        Self {
            config: ResonanceConfig::default(),
        }
    }

    /// 创建指定配置的共振引擎 / Create resonance engine with custom config
    pub fn with_config(config: ResonanceConfig) -> Self {
        Self { config }
    }

    /// 仪式发生时的共振脉冲 / Resonance pulse when a ritual occurs
    ///
    /// @param consecutive_days 连续天数 / Consecutive days
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @param source 共振来源 / Resonance source
    /// @return 共振结果 / Resonance result
    pub fn on_ritual_occurred(
        &self,
        consecutive_days: u32,
        relation_ordinal: u8,
        source: ResonanceSource,
    ) -> RitualResonance {
        let age_scale = self.age_scale(consecutive_days);
        let rel_scale = relation_scale(relation_ordinal);
        let combined = age_scale * rel_scale;

        let pleasure_delta = self.config.base_pleasure_pulse * combined;
        let arousal_delta = self.config.base_arousal_pulse * combined;
        let dominance_delta = self.config.base_dominance_pulse * combined;
        let intensity = (combined / self.config.age_scale_max).clamp(0.0, 1.0);

        RitualResonance {
            pleasure_delta,
            arousal_delta,
            dominance_delta,
            source,
            intensity,
        }
    }

    /// 仪式中断时的共振脉冲 / Resonance pulse when a ritual breaks
    ///
    /// 中断产生温和的负面情感冲击，随中断天数线性增长（7天饱和）。
    /// Break produces gentle negative emotional impact, growing linearly with
    /// break days (saturating at 7 days).
    ///
    /// @param break_days 中断天数 / Break days
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @param name 仪式名称 / Ritual name
    /// @return 共振结果 / Resonance result
    pub fn on_ritual_broken(
        &self,
        break_days: u32,
        relation_ordinal: u8,
        name: String,
    ) -> RitualResonance {
        let rel_scale = relation_scale(relation_ordinal);
        let break_factor = (break_days as f32 / 7.0).min(1.0);

        let pleasure_delta = self.config.break_pleasure_impact * rel_scale * break_factor;
        let arousal_delta = self.config.break_arousal_impact * rel_scale * break_factor;
        // 中断不影响支配度 / Break does not affect dominance
        let dominance_delta = 0.0;
        let intensity = break_factor * rel_scale / 1.2;

        RitualResonance {
            pleasure_delta,
            arousal_delta,
            dominance_delta,
            source: ResonanceSource::RitualBroken { name, break_days },
            intensity: intensity.clamp(0.0, 1.0),
        }
    }

    /// 纪念日共振脉冲 / Anniversary resonance pulse
    ///
    /// 纪念日产生比普通仪式更强烈的共振。
    /// Anniversaries produce stronger resonance than regular rituals.
    ///
    /// @param years 相处年数 / Years together
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @param kind 纪念日类型标签 / Anniversary kind label
    /// @return 共振结果 / Resonance result
    pub fn on_anniversary(
        &self,
        years: i32,
        relation_ordinal: u8,
        kind: String,
    ) -> RitualResonance {
        let rel_scale = relation_scale(relation_ordinal);
        let year_scale = 1.0 + (years.max(1) as f32).ln() * 0.3;
        let multiplier = self.config.anniversary_multiplier;
        let combined = year_scale * rel_scale * multiplier;

        let pleasure_delta = self.config.base_pleasure_pulse * combined;
        let arousal_delta = self.config.base_arousal_pulse * combined;
        let dominance_delta = self.config.base_dominance_pulse * combined;
        let intensity = (combined / (self.config.age_scale_max * multiplier)).clamp(0.0, 1.0);

        RitualResonance {
            pleasure_delta,
            arousal_delta,
            dominance_delta,
            source: ResonanceSource::Anniversary { kind, years },
            intensity,
        }
    }

    /// 仪式年龄缩放因子 / Ritual age scale factor
    ///
    /// 仪式越老（持续越久），共振越强，但有上限。
    /// Older rituals (more consecutive days) produce stronger resonance,
    /// capped at `age_scale_max`.
    fn age_scale(&self, consecutive_days: u32) -> f32 {
        if consecutive_days <= 1 {
            return 1.0;
        }
        let log2_days = (consecutive_days as f32).log2();
        (1.0 + log2_days * 0.15).min(self.config.age_scale_max)
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self, resonance: &RitualResonance) -> String {
        format!(
            "仪式共振: {} — P{:+.4} A{:+.4} D{:+.4} (强度{:.1}%)",
            resonance.source.label_zh(),
            resonance.pleasure_delta,
            resonance.arousal_delta,
            resonance.dominance_delta,
            resonance.intensity * 100.0,
        )
    }
}

impl Default for RitualResonanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── 关系阶段缩放 / Relationship Stage Scaling ──

/// 关系阶段对共振的缩放因子 / Relationship stage scaling for resonance
fn relation_scale(ordinal: u8) -> f32 {
    match ordinal {
        0 => 0.5,  // 初识：轻微 / Acquaintance: subtle
        1 => 0.75, // 熟悉：中等 / Familiar: moderate
        2 => 1.0,  // 信任：标准 / Trusted: standard
        3 => 1.2,  // 深度：强化 / Deep: enhanced
        _ => 1.0,  // 未知：标准 / Unknown: standard
    }
}

// ════════════════════════════════════════════════════════════════════
// ResonanceEngine trait 实现 / ResonanceEngine trait impl
// ════════════════════════════════════════════════════════════════════

/// 仪式共振引擎是无状态计算器——脉冲在被调用时即时产生并消费，
/// 不维护持续的情感残留，因此 PAD 增量恒为零，活跃度恒为零。
///
/// Ritual resonance engine is a stateless calculator — pulses are produced
/// and consumed on demand, with no persistent emotional state.
impl ResonanceEngine for RitualResonanceEngine {
    /// 当前 PAD 增量 — 无状态，恒为零 / Current PAD delta — stateless, always zero
    fn current_pad_delta(&self, _now_secs: f64) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }

    /// 时间步进 — 无状态，空操作 / Time tick — stateless, no-op
    fn tick(&mut self, _now_secs: f64) {}

    /// 活跃度 — 无持续状态，恒为零 / Activity — no persistent state, always zero
    fn activity(&self) -> f32 {
        0.0
    }

    /// 共振类型标签 / Resonance type label
    fn resonance_label(&self) -> &'static str {
        "仪式共振/RitualResonance"
    }
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ritual_occurred_basic() {
        let engine = RitualResonanceEngine::new();
        let r = engine.on_ritual_occurred(
            7,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        // 7天仪式: age_scale = 1.0 + log2(7)*0.15 ≈ 1.0 + 2.807*0.15 ≈ 1.421
        // relation_scale(2) = 1.0
        // pleasure = 0.08 * 1.421 * 1.0 ≈ 0.1137
        assert!(r.pleasure_delta > 0.0);
        assert!(r.pleasure_delta < 0.2); // 合理范围 / Reasonable range
        assert!(r.intensity > 0.0 && r.intensity <= 1.0);
    }

    #[test]
    fn test_age_scale_increases() {
        let engine = RitualResonanceEngine::new();
        let r1 = engine.on_ritual_occurred(
            1,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 1,
            },
        );
        let r7 = engine.on_ritual_occurred(
            7,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        let r30 = engine.on_ritual_occurred(
            30,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 30,
            },
        );
        // 仪式越老共振越强 / Older rituals resonate stronger
        assert!(r1.pleasure_delta < r7.pleasure_delta);
        assert!(r7.pleasure_delta < r30.pleasure_delta);
    }

    #[test]
    fn test_age_scale_capped() {
        let engine = RitualResonanceEngine::new();
        // 1000天仪式不应超过 age_scale_max * base
        let r = engine.on_ritual_occurred(
            1000,
            3, // 深度关系 scale=1.2
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 1000,
            },
        );
        // age_scale_max=2.0, rel_scale=1.2, base=0.08
        // max pleasure = 0.08 * 2.0 * 1.2 = 0.192
        assert!(r.pleasure_delta <= 0.192 + 1e-6);
    }

    #[test]
    fn test_relation_scale() {
        let engine = RitualResonanceEngine::new();
        let r0 = engine.on_ritual_occurred(
            7,
            0,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        let r3 = engine.on_ritual_occurred(
            7,
            3,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        // 深度关系共振更强 / Deep relationship resonates stronger
        assert!(r0.pleasure_delta < r3.pleasure_delta);
    }

    #[test]
    fn test_ritual_broken_negative() {
        let engine = RitualResonanceEngine::new();
        let r = engine.on_ritual_broken(3, 2, "晚安仪式".to_string());
        assert!(r.pleasure_delta < 0.0); // 中断产生负面愉悦 / Break produces negative pleasure
        assert!(r.arousal_delta < 0.0);
        assert_eq!(r.dominance_delta, 0.0); // 不影响支配度 / No dominance impact
    }

    #[test]
    fn test_break_saturates() {
        let engine = RitualResonanceEngine::new();
        let r3 = engine.on_ritual_broken(3, 2, "test".to_string());
        let r7 = engine.on_ritual_broken(7, 2, "test".to_string());
        let r30 = engine.on_ritual_broken(30, 2, "test".to_string());
        // 7天后饱和 / Saturates after 7 days
        assert!(r3.pleasure_delta > r7.pleasure_delta); // 更负 = 更小
        assert!((r7.pleasure_delta - r30.pleasure_delta).abs() < 1e-6);
    }

    #[test]
    fn test_anniversary_stronger_than_regular() {
        let engine = RitualResonanceEngine::new();
        let regular = engine.on_ritual_occurred(
            7,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        let anniversary = engine.on_anniversary(1, 2, "首次对话日".to_string());
        // 纪念日应比普通仪式更强 / Anniversary should be stronger
        assert!(anniversary.pleasure_delta > regular.pleasure_delta);
    }

    #[test]
    fn test_anniversary_years_scale() {
        let engine = RitualResonanceEngine::new();
        let r1 = engine.on_anniversary(1, 2, "test".to_string());
        let r5 = engine.on_anniversary(5, 2, "test".to_string());
        // 年数越久共振越强 / More years = stronger resonance
        assert!(r5.pleasure_delta > r1.pleasure_delta);
    }

    #[test]
    fn test_content_ritual_resonance() {
        let engine = RitualResonanceEngine::new();
        let r = engine.on_ritual_occurred(
            10,
            2,
            ResonanceSource::ContentRitual {
                hint_key: "goodnight".to_string(),
                consecutive_days: 10,
            },
        );
        assert!(r.pleasure_delta > 0.0);
        assert!(r.intensity > 0.0);
    }

    #[test]
    fn test_description_zh() {
        let engine = RitualResonanceEngine::new();
        let r = engine.on_ritual_occurred(
            7,
            2,
            ResonanceSource::TimeRitual {
                slot_hour: 22,
                consecutive_days: 7,
            },
        );
        let desc = engine.description_zh(&r);
        assert!(desc.contains("仪式共振"));
        assert!(desc.contains("时间仪式"));
    }

    #[test]
    fn test_source_label_zh() {
        let s = ResonanceSource::RitualBroken {
            name: "晚安仪式".to_string(),
            break_days: 5,
        };
        assert!(s.label_zh().contains("仪式中断"));
        assert!(s.label_zh().contains("5天"));

        let s = ResonanceSource::Anniversary {
            kind: "命名日".to_string(),
            years: 2,
        };
        assert!(s.label_zh().contains("纪念日"));
        assert!(s.label_zh().contains("2周年"));
    }

    #[test]
    fn test_intensity_in_range() {
        let engine = RitualResonanceEngine::new();
        // 各种参数下强度都应在 [0, 1]
        for &days in &[1u32, 7, 30, 365] {
            for &ord in &[0u8, 1, 2, 3] {
                let r = engine.on_ritual_occurred(
                    days,
                    ord,
                    ResonanceSource::TimeRitual {
                        slot_hour: 22,
                        consecutive_days: days,
                    },
                );
                assert!(
                    r.intensity >= 0.0 && r.intensity <= 1.0,
                    "intensity {} out of range for days={}, ord={}",
                    r.intensity,
                    days,
                    ord
                );
            }
        }
    }

    // ── ResonanceEngine trait 测试 / Trait Tests ──

    #[test]
    fn test_trait_ritual_pad_delta_always_zero() {
        // 无状态引擎，PAD 恒为零 / Stateless engine, PAD always zero
        let engine = RitualResonanceEngine::new();
        assert_eq!(engine.current_pad_delta(0.0), (0.0, 0.0, 0.0));
        assert_eq!(engine.current_pad_delta(99999.0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_trait_ritual_tick_noop() {
        // tick 不改变任何状态 / tick does not change state
        let mut engine = RitualResonanceEngine::new();
        let before = engine.config.base_pleasure_pulse;
        engine.tick(100.0);
        assert_eq!(engine.config.base_pleasure_pulse, before);
    }

    #[test]
    fn test_trait_ritual_activity_zero() {
        // 无状态，活跃度恒为零 / Stateless, activity always zero
        let engine = RitualResonanceEngine::new();
        assert_eq!(engine.activity(), 0.0);
    }

    #[test]
    fn test_trait_ritual_label() {
        let engine = RitualResonanceEngine::new();
        assert_eq!(engine.resonance_label(), "仪式共振/RitualResonance");
    }

    #[test]
    fn test_trait_ritual_prompt_fragment_empty() {
        // PAD 为零 → 不注入 / Zero PAD → no injection
        let engine = RitualResonanceEngine::new();
        assert!(engine.prompt_fragment(0.0).is_empty());
    }
}
