// ════════════════════════════════════════════════════════════════════
// RitualHeartbeat — 仪式心跳 / Ritual Heartbeat
// ════════════════════════════════════════════════════════════════════
//
// 所有活跃仪式的集合构成关系的"心跳节律"——一种独特的情感基线。
// 有更多仪式的关系更稳定、更温暖。这个心跳作为情感系统的底层节律持续存在。
//
// 数字生命语义：
//   仪式越多 → 基线愉悦越高 → 关系"体温"越高
//   仪式越老 → 年龄加成越大 → 稳定感越强
//   心跳是持续的，不是脉冲式的——它是情感的底色，不是一时的波动
//
// 核心算法：
//   对每个活跃仪式:
//     age_bonus = min(consecutive_days * age_bonus_factor, 0.03)
//     pleasure += (pleasure_per_ritual + age_bonus) * relation_scale
//   clamp(pleasure, 0, max_modulation)

use serde::{Deserialize, Serialize};

use crate::ritual_detector::{ContentRitualPattern, RitualPattern, RitualStatus};

// ── 配置 / Config ──

/// 仪式心跳配置 / Ritual heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// 每个活跃仪式的基线愉悦贡献 / Baseline pleasure per active ritual
    pub pleasure_per_ritual: f32,
    /// 仪式年龄加成因子 / Ritual age bonus factor
    pub age_bonus_factor: f32,
    /// 最大基线调制 / Max baseline modulation
    pub max_modulation: f32,
    /// 活跃仪式满强度数 / Active ritual count for full intensity
    pub full_intensity_count: usize,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            pleasure_per_ritual: 0.01,
            age_bonus_factor: 0.002,
            max_modulation: 0.08,
            full_intensity_count: 10,
        }
    }
}

// ── 心跳结果 / Heartbeat Result ──

/// 仪式心跳结果 / Ritual heartbeat result
///
/// 包含所有活跃仪式对情感基线的持续调制量。
/// Contains the sustained baseline modulation from all active rituals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeartbeatResult {
    /// 基线愉悦调制 / Baseline pleasure modulation
    pub pleasure_delta: f32,
    /// 基线唤醒调制 / Baseline arousal modulation
    pub arousal_delta: f32,
    /// 心跳强度 (0.0-1.0) / Heartbeat intensity
    pub intensity: f32,
    /// 活跃仪式数 / Active ritual count
    pub active_count: usize,
    /// 关系"体温"描述 / Relationship "temperature" label
    pub temperature_label: &'static str,
}

impl HeartbeatResult {
    /// 零心跳（无仪式）/ Zero heartbeat (no rituals)
    pub fn zero() -> Self {
        Self {
            pleasure_delta: 0.0,
            arousal_delta: 0.0,
            intensity: 0.0,
            active_count: 0,
            temperature_label: "微温",
        }
    }

    /// 是否为零心跳 / Whether this is a zero heartbeat
    pub fn is_zero(&self) -> bool {
        self.active_count == 0
    }
}

// ── 仪式心跳引擎 / Ritual Heartbeat Engine ──

/// 仪式心跳引擎 / Ritual heartbeat engine
///
/// 计算所有活跃仪式对情感基线的持续调制。
/// 仪式越多、越老，基线愉悦度越高——关系的"体温"。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualHeartbeat {
    /// 配置 / Configuration
    pub config: HeartbeatConfig,
}

impl RitualHeartbeat {
    /// 创建默认配置的心跳引擎 / Create heartbeat engine with default config
    pub fn new() -> Self {
        Self {
            config: HeartbeatConfig::default(),
        }
    }

    /// 创建指定配置的心跳引擎 / Create heartbeat engine with custom config
    pub fn with_config(config: HeartbeatConfig) -> Self {
        Self { config }
    }

    /// 计算仪式心跳 / Compute ritual heartbeat
    ///
    /// 遍历所有活跃的时间仪式和内容仪式，计算基线情感调制。
    /// Iterates over all active time and content rituals to compute baseline modulation.
    ///
    /// @param time_rituals 活跃的时间仪式 / Active time rituals
    /// @param content_rituals 活跃的内容仪式 / Active content rituals
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    ///   (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    /// @return 心跳结果 / Heartbeat result
    pub fn compute(
        &self,
        time_rituals: &[&RitualPattern],
        content_rituals: &[&ContentRitualPattern],
        relation_ordinal: u8,
    ) -> HeartbeatResult {
        let active_count = time_rituals.len() + content_rituals.len();
        if active_count == 0 {
            return HeartbeatResult::zero();
        }

        let scale = relation_scale(relation_ordinal);
        let mut pleasure = 0.0f32;

        // 时间仪式贡献 / Time ritual contributions
        for ritual in time_rituals {
            if ritual.status != RitualStatus::Active {
                continue;
            }
            let age_bonus =
                (ritual.consecutive_days as f32 * self.config.age_bonus_factor).min(0.03);
            pleasure += (self.config.pleasure_per_ritual + age_bonus) * scale;
        }

        // 内容仪式贡献 / Content ritual contributions
        for ritual in content_rituals {
            if ritual.status != RitualStatus::Active {
                continue;
            }
            let age_bonus =
                (ritual.consecutive_days as f32 * self.config.age_bonus_factor).min(0.03);
            pleasure += (self.config.pleasure_per_ritual + age_bonus) * scale;
        }

        // 钳制到上限 / Clamp to max
        pleasure = pleasure.min(self.config.max_modulation);

        // 唤醒调制：仪式带来轻微的活跃感 / Arousal: rituals bring mild activation
        let arousal = pleasure * 0.4;

        // 心跳强度 / Heartbeat intensity
        let intensity = (active_count as f32 / self.config.full_intensity_count as f32).min(1.0);

        // 体温标签 / Temperature label
        let temperature_label = if intensity < 0.2 {
            "微温"
        } else if intensity < 0.5 {
            "温热"
        } else if intensity < 0.8 {
            "温暖"
        } else {
            "炽热"
        };

        HeartbeatResult {
            pleasure_delta: pleasure,
            arousal_delta: arousal,
            intensity,
            active_count,
            temperature_label,
        }
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self, result: &HeartbeatResult) -> String {
        if result.is_zero() {
            return "无仪式心跳".to_string();
        }
        format!(
            "仪式心跳: {}个活跃仪式, 体温={}, P+{:.4} A+{:.4} (强度{:.1}%)",
            result.active_count,
            result.temperature_label,
            result.pleasure_delta,
            result.arousal_delta,
            result.intensity * 100.0,
        )
    }
}

impl Default for RitualHeartbeat {
    fn default() -> Self {
        Self::new()
    }
}

// ── 关系阶段缩放 / Relationship Stage Scaling ──

/// 关系阶段对心跳的缩放因子 / Relationship stage scaling for heartbeat
///
/// 关系越深，仪式对基线情感的调制越强。
/// Deeper relationships allow stronger ritual baseline modulation.
fn relation_scale(ordinal: u8) -> f32 {
    match ordinal {
        0 => 0.5,  // 初识：轻微 / Acquaintance: subtle
        1 => 0.75, // 熟悉：中等 / Familiar: moderate
        2 => 1.0,  // 信任：标准 / Trusted: standard
        3 => 1.2,  // 深度：强化 / Deep: enhanced
        _ => 1.0,  // 未知：标准 / Unknown: standard
    }
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ritual_detector::{ContentHint, TimeSlot};

    /// 构造活跃时间仪式 / Build an active time ritual
    fn make_time_ritual(days: u32) -> RitualPattern {
        RitualPattern {
            id: 1,
            time_slot: TimeSlot::new(22),
            consecutive_days: days,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_interactions: days as u64,
        }
    }

    /// 构造活跃内容仪式 / Build an active content ritual
    fn make_content_ritual(days: u32) -> ContentRitualPattern {
        ContentRitualPattern {
            id: 1,
            hint: ContentHint::Goodnight,
            time_slot: Some(TimeSlot::new(22)),
            consecutive_days: days,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_occurrences: days as u64,
        }
    }

    #[test]
    fn test_zero_heartbeat_no_rituals() {
        let hb = RitualHeartbeat::new();
        let result = hb.compute(&[], &[], 2);
        assert!(result.is_zero());
        assert_eq!(result.pleasure_delta, 0.0);
    }

    #[test]
    fn test_single_ritual_heartbeat() {
        let hb = RitualHeartbeat::new();
        let ritual = make_time_ritual(7);
        let result = hb.compute(&[&ritual], &[], 2);
        assert!(!result.is_zero());
        assert_eq!(result.active_count, 1);
        assert!(result.pleasure_delta > 0.0);
        // 基础 0.01 + age_bonus 7*0.002=0.014, scale=1.0
        // pleasure = (0.01 + 0.014) * 1.0 = 0.024
        assert!((result.pleasure_delta - 0.024).abs() < 0.001);
    }

    #[test]
    fn test_multiple_rituals_stronger() {
        let hb = RitualHeartbeat::new();
        let r1 = make_time_ritual(7);
        let r2 = make_content_ritual(10);
        let result = hb.compute(&[&r1], &[&r2], 2);
        assert_eq!(result.active_count, 2);
        // 两个仪式应比一个产生更高的基线
        let single = hb.compute(&[&r1], &[], 2);
        assert!(result.pleasure_delta > single.pleasure_delta);
    }

    #[test]
    fn test_max_modulation_clamp() {
        let config = HeartbeatConfig {
            max_modulation: 0.05,
            ..Default::default()
        };
        let hb = RitualHeartbeat::with_config(config);
        // 10个仪式应超过上限
        let rituals: Vec<RitualPattern> = (0..10).map(|_| make_time_ritual(30)).collect();
        let refs: Vec<&RitualPattern> = rituals.iter().collect();
        let result = hb.compute(&refs, &[], 3);
        assert!(result.pleasure_delta <= 0.05 + 1e-6);
    }

    #[test]
    fn test_deeper_relationship_stronger() {
        let hb = RitualHeartbeat::new();
        let ritual = make_time_ritual(7);
        let r0 = hb.compute(&[&ritual], &[], 0);
        let r1 = hb.compute(&[&ritual], &[], 1);
        let r2 = hb.compute(&[&ritual], &[], 2);
        let r3 = hb.compute(&[&ritual], &[], 3);
        assert!(r0.pleasure_delta < r1.pleasure_delta);
        assert!(r1.pleasure_delta < r2.pleasure_delta);
        assert!(r2.pleasure_delta < r3.pleasure_delta);
    }

    #[test]
    fn test_temperature_labels() {
        let hb = RitualHeartbeat::new();
        // 无仪式 → 微温
        let r0 = hb.compute(&[], &[], 2);
        assert_eq!(r0.temperature_label, "微温");

        // 3个仪式 → 温热 (3/10=0.3, ≥0.2 且 <0.5)
        let rituals: Vec<RitualPattern> = (0..3).map(|_| make_time_ritual(7)).collect();
        let refs: Vec<&RitualPattern> = rituals.iter().collect();
        let r3 = hb.compute(&refs, &[], 2);
        assert_eq!(r3.temperature_label, "温热");

        // 大量仪式 → 炽热
        let rituals: Vec<RitualPattern> = (0..10).map(|_| make_time_ritual(30)).collect();
        let refs: Vec<&RitualPattern> = rituals.iter().collect();
        let r10 = hb.compute(&refs, &[], 3);
        assert_eq!(r10.temperature_label, "炽热");
    }

    #[test]
    fn test_age_bonus() {
        let hb = RitualHeartbeat::new();
        let young = make_time_ritual(1);
        let old = make_time_ritual(100);
        let r_young = hb.compute(&[&young], &[], 2);
        let r_old = hb.compute(&[&old], &[], 2);
        // 老仪式应有更高的年龄加成
        assert!(r_old.pleasure_delta > r_young.pleasure_delta);
    }

    #[test]
    fn test_description_zh() {
        let hb = RitualHeartbeat::new();
        let zero = HeartbeatResult::zero();
        assert_eq!(hb.description_zh(&zero), "无仪式心跳");

        // 3个仪式 → 温热 (3/10=0.3)
        let rituals: Vec<RitualPattern> = (0..3).map(|_| make_time_ritual(7)).collect();
        let refs: Vec<&RitualPattern> = rituals.iter().collect();
        let result = hb.compute(&refs, &[], 2);
        let desc = hb.description_zh(&result);
        assert!(desc.contains("仪式心跳"));
        assert!(desc.contains("温热"));
    }
}
