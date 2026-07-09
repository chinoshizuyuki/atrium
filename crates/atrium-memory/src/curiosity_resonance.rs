// SPDX-License-Identifier: MIT

//! 好奇心PAD共振 — Curiosity resonance: maps curiosity states to PAD emotional
//! modulations with exponential decay.
//!
//! 核心理念：好奇心不只是认知状态，更是情感状态——好奇时回复的语气、
//! 用词、节奏都应该不同。

use serde::{Deserialize, Serialize};

use crate::resonance_core::{
    exponential_decay_f32, pad_delta_to_array, pad_magnitude, PadSource, ResonanceEngine,
};

// ═══════════════════════════════════════════════════════════════════════════
// 配置 — Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇心共振配置 / Curiosity resonance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityResonanceConfig {
    /// 触发时愉悦调制 / Pleasure modulation on trigger.
    pub trigger_pleasure: f32,
    /// 触发时激活调制 / Arousal modulation on trigger.
    pub trigger_arousal: f32,
    /// 触发时支配调制 / Dominance modulation on trigger.
    pub trigger_dominance: f32,
    /// 满足时愉悦调制 / Pleasure modulation on satisfaction.
    pub satisfy_pleasure: f32,
    /// 满足时激活调制 / Arousal modulation on satisfaction.
    pub satisfy_arousal: f32,
    /// 衰减半衰期（秒）/ Decay half-life in seconds.
    pub half_life_secs: f32,
}

impl Default for CuriosityResonanceConfig {
    fn default() -> Self {
        Self {
            trigger_pleasure: 0.15,
            trigger_arousal: 0.35,
            trigger_dominance: -0.20,
            satisfy_pleasure: 0.30,
            satisfy_arousal: -0.25,
            half_life_secs: 60.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 好奇心PAD共振 — Curiosity Resonance
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇心PAD共振 — 管理好奇心触发和满足的 PAD 脉冲
/// Curiosity resonance — Manages PAD pulses from curiosity trigger and satisfaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityResonance {
    /// 当前 PAD 调制 (pleasure, arousal, dominance) / Current PAD delta.
    pub current_pad: (f32, f32, f32),
    /// 满足度脉冲 / Satisfaction pulse magnitude.
    pub satisfaction_pulse: f32,
    /// 上次更新时间戳 / Last update timestamp.
    pub last_tick: i64,
    /// 配置 / Configuration.
    pub config: CuriosityResonanceConfig,
}

impl CuriosityResonance {
    /// 创建默认配置的共振 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(CuriosityResonanceConfig::default())
    }

    /// 创建指定配置的共振 / Create with custom config.
    pub fn new(config: CuriosityResonanceConfig) -> Self {
        Self {
            current_pad: (0.0, 0.0, 0.0),
            satisfaction_pulse: 0.0,
            last_tick: 0,
            config,
        }
    }

    /// 好奇心触发 — 产生 PAD 脉冲
    /// Curiosity triggered — Produces a PAD pulse.
    pub fn on_curiosity_triggered(&mut self, intensity: f32, now: i64) {
        let i = intensity.clamp(0.0, 1.0);
        self.current_pad.0 += self.config.trigger_pleasure * i;
        self.current_pad.1 += self.config.trigger_arousal * i;
        self.current_pad.2 += self.config.trigger_dominance * i;
        self.last_tick = now;
    }

    /// 用户满足 — 好奇心得到回应，产生满足脉冲
    /// User satisfied — Curiosity answered, produces satisfaction pulse.
    pub fn on_satisfied(&mut self, intensity: f32, now: i64) {
        let i = intensity.clamp(0.0, 1.0);
        self.current_pad.0 += self.config.satisfy_pleasure * i;
        self.current_pad.1 += self.config.satisfy_arousal * i;
        self.satisfaction_pulse = (self.satisfaction_pulse + i).min(1.0);
        self.last_tick = now;
    }

    /// 时间衰减 — 指数衰减 PAD 调制
    /// Tick — Exponential decay of PAD modulation.
    pub fn tick(&mut self, now: i64) {
        if self.last_tick == 0 {
            self.last_tick = now;
            return;
        }
        let elapsed = (now - self.last_tick).max(0) as f32;
        let decay = exponential_decay_f32(elapsed, self.config.half_life_secs);
        self.current_pad.0 *= decay;
        self.current_pad.1 *= decay;
        self.current_pad.2 *= decay;
        self.satisfaction_pulse *= decay;
        self.last_tick = now;
    }

    /// 获取当前 PAD 调制 / Get current PAD modulation.
    pub fn current_pad(&self) -> (f32, f32, f32) {
        self.current_pad
    }

    /// 生成情感色彩 prompt 后缀 — 描述当前好奇心状态
    /// Generate emotion-tinted prompt suffix.
    pub fn prompt_suffix(&self) -> String {
        let (_, arousal, dominance) = self.current_pad;
        if arousal.abs() < 0.05 && dominance.abs() < 0.05 {
            return String::new();
        }
        let mut parts = Vec::new();
        if arousal > 0.1 {
            parts.push("语气中带着好奇");
        }
        if dominance < -0.1 {
            parts.push("不确定但想知道");
        }
        if self.satisfaction_pulse > 0.3 {
            parts.push("因为知道了而愉悦");
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("（{}）", parts.join("，"))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ResonanceEngine trait 实现 / ResonanceEngine trait impl
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇共振引擎是有状态的——维护当前 PAD 调制和满足度脉冲，
/// 随时间指数衰减。trait 方法委托到已有实现，并桥接 i64↔f64 时间戳。
///
/// Curiosity resonance is stateful — maintains current PAD modulation and
/// satisfaction pulse with exponential decay. Trait methods delegate to
/// existing implementations, bridging i64↔f64 timestamps.
impl ResonanceEngine for CuriosityResonance {
    /// 当前 PAD 情感增量 / Current PAD emotional delta
    fn current_pad_delta(&self, _now_secs: f64) -> (f32, f32, f32) {
        self.current_pad
    }

    /// 时间步进 — 指数衰减 / Time tick — exponential decay
    fn tick(&mut self, now_secs: f64) {
        // 桥接 f64→i64 时间戳 / Bridge f64→i64 timestamp
        CuriosityResonance::tick(self, now_secs as i64);
    }

    /// 活跃度 = PAD 模长（钳制到 [0, 1]）/ Activity = PAD magnitude clamped
    fn activity(&self) -> f32 {
        let (p, a, d) = self.current_pad;
        pad_magnitude(p, a, d).min(1.0)
    }

    /// 共振类型标签 / Resonance type label
    fn resonance_label(&self) -> &'static str {
        "好奇共振/CuriosityResonance"
    }

    /// 生成 prompt 注入片段 / Generate prompt injection fragment
    ///
    /// 覆盖默认实现，使用好奇共振特有的中文情感描述。
    fn prompt_fragment(&self, now_secs: f64) -> String {
        let (p, a, d) = self.current_pad_delta(now_secs);
        if p.abs() < 0.01 && a.abs() < 0.01 {
            return String::new();
        }
        // 复用已有 prompt_suffix 生成情感色彩描述 / Reuse existing prompt_suffix
        let suffix = self.prompt_suffix();
        if suffix.is_empty() {
            format!(
                "[好奇共振/CuriosityResonance] PAD: P{:+.3} A{:+.3} D{:+.3} — 好奇的情感回响仍在",
                p, a, d
            )
        } else {
            format!(
                "[好奇共振/CuriosityResonance] PAD: P{:+.3} A{:+.3} D{:+.3} {}",
                p, a, d, suffix
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

// PadSource trait 桥接 — 统一 PAD 情感源接口 / PadSource trait bridge
impl PadSource for CuriosityResonance {
    /// 当前好奇心共振 PAD 增量 / Current curiosity resonance PAD delta
    #[inline]
    fn pad_delta(&self) -> [f64; 3] {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as f64;
        pad_delta_to_array(self.current_pad_delta(now_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_pad_zero() {
        let r = CuriosityResonance::default_new();
        assert_eq!(r.current_pad, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_trigger_produces_pad() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let (p, a, d) = r.current_pad;
        assert!(p > 0.0, "pleasure should be positive: {}", p);
        assert!(a > 0.0, "arousal should be positive: {}", a);
        assert!(d < 0.0, "dominance should be negative: {}", d);
    }

    #[test]
    fn test_satisfied_produces_pad() {
        let mut r = CuriosityResonance::default_new();
        r.on_satisfied(1.0, 100);
        let (p, a, _) = r.current_pad;
        assert!(p > 0.0, "satisfaction pleasure should be positive: {}", p);
        assert!(a < 0.0, "satisfaction arousal should be negative: {}", a);
        assert!(r.satisfaction_pulse > 0.0);
    }

    #[test]
    fn test_tick_decays() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let before = r.current_pad.1;
        r.tick(100 + 60); // one half-life
        let after = r.current_pad.1;
        assert!(after < before, "should decay: {} -> {}", before, after);
        assert!(
            (after - before * 0.5).abs() < 0.01,
            "should be ~half: {}",
            after
        );
    }

    #[test]
    fn test_multiple_triggers_accumulate() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(0.5, 100);
        let after_first = r.current_pad.1;
        r.on_curiosity_triggered(0.5, 100);
        let after_second = r.current_pad.1;
        assert!(after_second > after_first, "should accumulate");
    }

    #[test]
    fn test_decay_to_near_zero() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        r.tick(100 + 600); // 10 half-lives
        let (p, a, d) = r.current_pad;
        assert!(p.abs() < 0.01, "pleasure should be near zero: {}", p);
        assert!(a.abs() < 0.01, "arousal should be near zero: {}", a);
        assert!(d.abs() < 0.01, "dominance should be near zero: {}", d);
    }

    #[test]
    fn test_prompt_suffix_with_curiosity() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let s = r.prompt_suffix();
        assert!(!s.is_empty(), "should have suffix: {}", s);
    }

    #[test]
    fn test_prompt_suffix_empty_when_calm() {
        let r = CuriosityResonance::default_new();
        assert!(r.prompt_suffix().is_empty());
    }

    #[test]
    fn test_high_intensity_large_pad() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let high = r.current_pad.1;
        let mut r2 = CuriosityResonance::default_new();
        r2.on_curiosity_triggered(0.3, 100);
        let low = r2.current_pad.1;
        assert!(high > low, "high intensity should produce larger PAD");
    }

    #[test]
    fn test_low_intensity_small_pad() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(0.1, 100);
        let (_, a, _) = r.current_pad;
        assert!(a < 0.1, "low intensity should produce small PAD: {}", a);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(0.8, 100);
        let json = serde_json::to_string(&r).unwrap();
        let r2: CuriosityResonance = serde_json::from_str(&json).unwrap();
        assert!((r2.current_pad.1 - r.current_pad.1).abs() < 0.01);
    }

    // ── ResonanceEngine trait 测试 / Trait Tests ──

    #[test]
    fn test_trait_curiosity_pad_delta_matches_current_pad() {
        // trait 方法应返回 current_pad / Trait method should return current_pad
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(0.8, 100);
        assert_eq!(r.current_pad_delta(100.0), r.current_pad);
    }

    #[test]
    fn test_trait_curiosity_tick_decays() {
        // trait tick 应触发衰减 / Trait tick should trigger decay
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let before = r.current_pad.1;
        ResonanceEngine::tick(&mut r, 160.0); // 一个半衰期 / One half-life
        let after = r.current_pad.1;
        assert!(after < before, "should decay via trait tick");
    }

    #[test]
    fn test_trait_curiosity_activity_zero_when_calm() {
        // 平静时活跃度为零 / Activity is zero when calm
        let r = CuriosityResonance::default_new();
        assert_eq!(r.activity(), 0.0);
    }

    #[test]
    fn test_trait_curiosity_activity_positive_when_triggered() {
        // 触发后活跃度大于零 / Activity is positive after trigger
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        assert!(r.activity() > 0.0);
        assert!(r.activity() <= 1.0); // 钳制到 [0, 1] / Clamped to [0, 1]
    }

    #[test]
    fn test_trait_curiosity_label() {
        let r = CuriosityResonance::default_new();
        assert_eq!(r.resonance_label(), "好奇共振/CuriosityResonance");
    }

    #[test]
    fn test_trait_curiosity_prompt_fragment_when_active() {
        // 活跃时 prompt_fragment 非空 / Active → non-empty fragment
        let mut r = CuriosityResonance::default_new();
        r.on_curiosity_triggered(1.0, 100);
        let frag = r.prompt_fragment(100.0);
        assert!(!frag.is_empty(), "should have fragment: {}", frag);
        assert!(frag.contains("好奇共振/CuriosityResonance"));
    }

    #[test]
    fn test_trait_curiosity_prompt_fragment_empty_when_calm() {
        // 平静时 prompt_fragment 为空 / Calm → empty fragment
        let r = CuriosityResonance::default_new();
        assert!(r.prompt_fragment(0.0).is_empty());
    }
}
