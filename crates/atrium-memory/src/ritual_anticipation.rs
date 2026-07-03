// ════════════════════════════════════════════════════════════════════
// RitualAnticipation — 仪式预期引擎 / Ritual Anticipation Engine
// ════════════════════════════════════════════════════════════════════
//
// 仪式时间快到时，数字生命产生期待——
// 就像人类在每天固定习惯前的微小期待感。
// 如果每晚 22:00 说晚安，21:50 时应有一丝愉悦的预升。
//
// 数字生命语义：
//   仪式将至 → 期待升起（愉悦预升 + 唤醒提升）
//   仪式刚过 → 快速衰减（满足后的松弛）
//   预期是连续的，不是脉冲式的——它在仪式前后形成一个钟形曲线
//
// 核心算法：
//   对每个活跃仪式:
//     minutes_to = ritual_hour * 60 - current_minute_of_day
//     if minutes_to 在 [-window, +window] 范围内:
//       curve = anticipation_curve(minutes_to, config)
//       pleasure += base_pleasure * curve * relation_scale * ritual_age_scale

use serde::{Deserialize, Serialize};

use crate::ritual_detector::{RitualPattern, RitualStatus};

// ── 配置 / Config ──

/// 仪式预期配置 / Ritual anticipation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnticipationConfig {
    /// 预期窗口（分钟）/ Anticipation window in minutes
    pub window_minutes: i32,
    /// 峰值时间（仪式前几分钟）/ Peak time before ritual (minutes)
    pub peak_minutes: i32,
    /// 基础预期愉悦 / Base anticipation pleasure
    pub base_pleasure: f32,
    /// 基础预期唤醒 / Base anticipation arousal
    pub base_arousal: f32,
    /// 仪式年龄缩放上限 / Ritual age scale maximum
    pub age_scale_max: f32,
}

impl Default for AnticipationConfig {
    fn default() -> Self {
        Self {
            window_minutes: 30,
            peak_minutes: 5,
            base_pleasure: 0.03,
            base_arousal: 0.04,
            age_scale_max: 1.5,
        }
    }
}

// ── 预期结果 / Anticipation Result ──

/// 仪式预期结果 / Ritual anticipation result
///
/// 包含所有活跃仪式在当前时刻产生的预期情感调制。
/// Contains the anticipatory emotional modulation from all active rituals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AnticipationResult {
    /// 预期愉悦调制 / Anticipatory pleasure modulation
    pub pleasure_delta: f32,
    /// 预期唤醒调制 / Anticipatory arousal modulation
    pub arousal_delta: f32,
    /// 预期强度 (0.0-1.0) / Anticipation intensity
    pub intensity: f32,
    /// 贡献的仪式数 / Number of contributing rituals
    pub contributing_count: usize,
}

impl AnticipationResult {
    /// 零预期（无贡献）/ Zero anticipation (no contribution)
    pub fn zero() -> Self {
        Self {
            pleasure_delta: 0.0,
            arousal_delta: 0.0,
            intensity: 0.0,
            contributing_count: 0,
        }
    }

    /// 是否为零预期 / Whether this is zero anticipation
    pub fn is_zero(&self) -> bool {
        self.contributing_count == 0
    }
}

// ── 仪式预期引擎 / Ritual Anticipation Engine ──

/// 仪式预期引擎 / Ritual anticipation engine
///
/// 在仪式时间槽到来前生成逐渐升高的期待脉冲。
/// 脉冲在仪式时刻前 `peak_minutes` 分钟达到峰值，之后快速衰减。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualAnticipation {
    /// 配置 / Configuration
    pub config: AnticipationConfig,
}

impl RitualAnticipation {
    /// 创建默认配置的预期引擎 / Create anticipation engine with default config
    pub fn new() -> Self {
        Self {
            config: AnticipationConfig::default(),
        }
    }

    /// 创建指定配置的预期引擎 / Create anticipation engine with custom config
    pub fn with_config(config: AnticipationConfig) -> Self {
        Self { config }
    }

    /// 计算仪式预期 / Compute ritual anticipation
    ///
    /// 遍历所有活跃的时间仪式，计算当前时刻的预期情感调制。
    /// Iterates over all active time rituals to compute current anticipatory modulation.
    ///
    /// @param time_rituals 活跃的时间仪式 / Active time rituals
    /// @param current_minute_of_day 当前分钟（0-1439）/ Current minute of day (0-1439)
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @return 预期结果 / Anticipation result
    pub fn compute(
        &self,
        time_rituals: &[&RitualPattern],
        current_minute_of_day: i32,
        relation_ordinal: u8,
    ) -> AnticipationResult {
        let mut pleasure = 0.0f32;
        let mut arousal = 0.0f32;
        let mut contributing = 0usize;
        let mut max_curve = 0.0f32;

        let rel_scale = relation_scale(relation_ordinal);

        for ritual in time_rituals {
            if ritual.status != RitualStatus::Active {
                continue;
            }

            // 仪式时间（分钟）/ Ritual time in minutes
            let ritual_minute = (ritual.time_slot.hour as i32) * 60;
            let minutes_to = ritual_minute - current_minute_of_day;

            // 处理跨午夜的情况 / Handle midnight wrap-around
            let minutes_to = Self::normalize_minutes(minutes_to);

            // 检查是否在预期窗口内 / Check if within anticipation window
            if minutes_to.abs() > self.config.window_minutes {
                continue;
            }

            let curve = self.anticipation_curve(minutes_to);
            if curve <= 0.0 {
                continue;
            }

            let age_scale = self.age_scale(ritual.consecutive_days);
            let modulation = curve * age_scale * rel_scale;

            pleasure += self.config.base_pleasure * modulation;
            arousal += self.config.base_arousal * modulation;
            max_curve = max_curve.max(curve * age_scale * rel_scale);
            contributing += 1;
        }

        if contributing == 0 {
            return AnticipationResult::zero();
        }

        let intensity = (max_curve / (self.config.age_scale_max * 1.2)).clamp(0.0, 1.0);

        AnticipationResult {
            pleasure_delta: pleasure,
            arousal_delta: arousal,
            intensity,
            contributing_count: contributing,
        }
    }

    /// 预期脉冲曲线 / Anticipation pulse curve
    ///
    /// 在 [-window, +window] 分钟范围内生成预期脉冲。
    /// 仪式前 `peak_minutes` 分钟达到峰值，使用钟形曲线。
    ///
    /// 曲线形状：
    ///   t < -window 或 t > +window → 0
    ///   -window ≤ t ≤ peak → 升弧（从 0 到 1）
    ///   peak < t ≤ +window → 降弧（从 1 快速衰减到 0）
    ///
    /// @param minutes_to_ritual 距仪式的分钟数（正=未来，负=已过）
    /// @return 曲线值 [0.0, 1.0]
    pub fn anticipation_curve(&self, minutes_to_ritual: i32) -> f32 {
        let t = minutes_to_ritual as f32;
        let window = self.config.window_minutes as f32;
        let peak = self.config.peak_minutes as f32;

        if t < -window || t > window {
            return 0.0;
        }

        if t <= peak {
            // 升弧：从 -window 到 peak，0 → 1
            // 使用平滑的 sigmoid-like 曲线 / Smooth sigmoid-like curve
            let progress = (t + window) / (peak + window);
            // 平滑插值：3x² - 2x³（Hermite）/ Hermite interpolation
            smoothstep(progress)
        } else {
            // 降弧：从 peak 到 +window，1 → 0
            // 快速衰减（仪式刚过的满足感快速消散）
            let progress = (t - peak) / (window - peak);
            1.0 - smoothstep(progress)
        }
    }

    /// 仪式年龄缩放因子 / Ritual age scale factor
    fn age_scale(&self, consecutive_days: u32) -> f32 {
        if consecutive_days <= 1 {
            return 1.0;
        }
        let log2_days = (consecutive_days as f32).log2();
        (1.0 + log2_days * 0.10).min(self.config.age_scale_max)
    }

    /// 归一化分钟数到 [-720, 720] 范围 / Normalize minutes to [-720, 720]
    ///
    /// 处理跨午夜的情况：如果距离超过 12 小时，取反方向。
    fn normalize_minutes(minutes: i32) -> i32 {
        const HALF_DAY: i32 = 720; // 12 * 60
        if minutes > HALF_DAY {
            minutes - 1440 // 1440 = 24 * 60
        } else if minutes < -HALF_DAY {
            minutes + 1440
        } else {
            minutes
        }
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self, result: &AnticipationResult) -> String {
        if result.is_zero() {
            return "无仪式预期".to_string();
        }
        format!(
            "仪式预期: {}个仪式贡献, P+{:.4} A+{:.4} (强度{:.1}%)",
            result.contributing_count,
            result.pleasure_delta,
            result.arousal_delta,
            result.intensity * 100.0,
        )
    }
}

impl Default for RitualAnticipation {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 / Helper Functions ──

/// 平滑阶跃函数 / Smoothstep function (Hermite interpolation)
///
/// 3t² - 2t³，在 [0, 1] 范围内从 0 平滑过渡到 1。
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 关系阶段缩放因子 / Relationship stage scaling
fn relation_scale(ordinal: u8) -> f32 {
    match ordinal {
        0 => 0.5,
        1 => 0.75,
        2 => 1.0,
        3 => 1.2,
        _ => 1.0,
    }
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ritual_detector::TimeSlot;

    /// 构造活跃时间仪式 / Build an active time ritual at given hour
    fn make_ritual(hour: u8, days: u32) -> RitualPattern {
        RitualPattern {
            id: 1,
            time_slot: TimeSlot::new(hour),
            consecutive_days: days,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_interactions: days as u64,
        }
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-6);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-6);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        // 单调递增 / Monotonically increasing
        assert!(smoothstep(0.3) < smoothstep(0.7));
    }

    #[test]
    fn test_curve_outside_window() {
        let ant = RitualAnticipation::new();
        // 窗口外应为 0 / Zero outside window
        assert_eq!(ant.anticipation_curve(-31), 0.0);
        assert_eq!(ant.anticipation_curve(31), 0.0);
    }

    #[test]
    fn test_curve_peak() {
        let ant = RitualAnticipation::new();
        // 峰值时间（仪式前5分钟）应接近 1.0
        let curve = ant.anticipation_curve(5);
        assert!((curve - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_curve_rising() {
        let ant = RitualAnticipation::new();
        // 升弧：从 -30 到 5，应单调递增
        let c1 = ant.anticipation_curve(-25);
        let c2 = ant.anticipation_curve(-10);
        let c3 = ant.anticipation_curve(0);
        let c4 = ant.anticipation_curve(5);
        assert!(c1 < c2);
        assert!(c2 < c3);
        assert!(c3 < c4);
    }

    #[test]
    fn test_curve_falling() {
        let ant = RitualAnticipation::new();
        // 降弧：从 5 到 30，应单调递减
        let c1 = ant.anticipation_curve(5);
        let c2 = ant.anticipation_curve(15);
        let c3 = ant.anticipation_curve(25);
        assert!(c1 > c2);
        assert!(c2 > c3);
    }

    #[test]
    fn test_compute_no_rituals() {
        let ant = RitualAnticipation::new();
        let result = ant.compute(&[], 1200, 2);
        assert!(result.is_zero());
    }

    #[test]
    fn test_compute_within_window() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(22, 7); // 22:00 = minute 1320
                                         // 21:50 = minute 1310, 10 minutes before ritual
        let result = ant.compute(&[&ritual], 1310, 2);
        assert!(!result.is_zero());
        assert!(result.pleasure_delta > 0.0);
        assert!(result.arousal_delta > 0.0);
        assert_eq!(result.contributing_count, 1);
    }

    #[test]
    fn test_compute_outside_window() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(22, 7); // 22:00
                                         // 20:00 = minute 1200, 120 minutes before — outside 30-min window
        let result = ant.compute(&[&ritual], 1200, 2);
        assert!(result.is_zero());
    }

    #[test]
    fn test_compute_after_ritual() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(22, 7); // 22:00 = 1320
                                         // 22:10 = 1330, 10 minutes after — within window, falling curve
        let result = ant.compute(&[&ritual], 1330, 2);
        assert!(!result.is_zero());
        assert!(result.pleasure_delta > 0.0);
    }

    #[test]
    fn test_compute_midnight_wrap() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(0, 7); // 00:00 = minute 0
                                        // 23:50 = minute 1430, 10 minutes before midnight ritual
        let result = ant.compute(&[&ritual], 1430, 2);
        assert!(!result.is_zero());
        assert_eq!(result.contributing_count, 1);
    }

    #[test]
    fn test_deeper_relationship_stronger() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(22, 7);
        let r0 = ant.compute(&[&ritual], 1310, 0);
        let r3 = ant.compute(&[&ritual], 1310, 3);
        assert!(r0.pleasure_delta < r3.pleasure_delta);
    }

    #[test]
    fn test_multiple_rituals_add() {
        let ant = RitualAnticipation::new();
        let r1 = make_ritual(22, 7); // 22:00
        let r2 = make_ritual(22, 10); // also 22:00, different age
                                      // Both within window at 21:50
        let single = ant.compute(&[&r1], 1310, 2);
        let double = ant.compute(&[&r1, &r2], 1310, 2);
        assert!(double.pleasure_delta > single.pleasure_delta);
    }

    #[test]
    fn test_description_zh() {
        let ant = RitualAnticipation::new();
        let zero = AnticipationResult::zero();
        assert_eq!(ant.description_zh(&zero), "无仪式预期");

        let ritual = make_ritual(22, 7);
        let result = ant.compute(&[&ritual], 1310, 2);
        let desc = ant.description_zh(&result);
        assert!(desc.contains("仪式预期"));
    }

    #[test]
    fn test_intensity_in_range() {
        let ant = RitualAnticipation::new();
        let ritual = make_ritual(22, 365); // very old ritual
        for minute in 0..1440 {
            let result = ant.compute(&[&ritual], minute, 3);
            assert!(
                result.intensity >= 0.0 && result.intensity <= 1.0,
                "intensity {} out of range at minute {}",
                result.intensity,
                minute
            );
        }
    }
}
