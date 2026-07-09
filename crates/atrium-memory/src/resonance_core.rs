// SPDX-License-Identifier: MIT
//! 共振核心 / Resonance Core — 三引擎共享的接口与工具
//!
//! 数字生命中，仪式、脆弱、好奇心三种共振引擎各自产生 PAD 情感脉冲。
//! 本模块提供统一的 `ResonanceEngine` trait 和共享工具函数，
//! 让所有共振引擎通过同一接口向情绪系统注入情感增量。
//!
//! In the digital life system, ritual, vulnerability, and curiosity resonance
//! engines each produce PAD emotional pulses. This module provides a unified
//! `ResonanceEngine` trait and shared utilities, allowing all resonance engines
//! to inject emotional deltas through a single interface.

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// PAD 情感增量 / PAD Emotional Delta
// ═══════════════════════════════════════════════════════════════════════════

/// PAD 情感增量 / PAD emotional delta (pleasure, arousal, dominance)
///
/// 所有共振引擎产出的情感增量统一用此类型表示。
/// All resonance engines produce emotional deltas using this type.
pub type PadDelta = (f32, f32, f32);

/// PAD 增量的向量模长 / Vector magnitude of PAD delta
///
/// 用于计算活跃度指标——模长越大，情感回响越强。
/// Used to compute activity level — larger magnitude means stronger echo.
#[inline]
pub fn pad_magnitude(p: f32, a: f32, d: f32) -> f32 {
    (p * p + a * a + d * d).sqrt()
}

/// PadDelta 转换为 f64 数组 / Convert PadDelta to f64 array
///
/// 统一 (f32, f32, f32) → [f64; 3] 的转换路径。
#[inline]
pub fn pad_delta_to_array(d: PadDelta) -> [f64; 3] {
    [d.0 as f64, d.1 as f64, d.2 as f64]
}

// ═══════════════════════════════════════════════════════════════════════════
// PAD 情感源 trait / PAD Source Trait
// ═══════════════════════════════════════════════════════════════════════════

/// PAD 情感源 — 统一所有产生 PAD 三元组的模块接口 / PAD emotional source trait
///
/// 数字生命中，期待深度、情绪气候、共振引擎各自产生 PAD 情感增量。
/// 此 trait 统一这些模块的接口，让上层只需调用 `pad_delta()` 即可获取情感偏移。
///
/// In the digital life system, anticipation depth, emotional climate, and
/// resonance engines each produce PAD emotional deltas. This trait unifies
/// their interfaces so callers simply call `pad_delta()` to get the offset.
pub trait PadSource {
    /// 当前 PAD 增量 [pleasure, arousal, dominance] / Current PAD delta
    fn pad_delta(&self) -> [f64; 3];
}

// ═══════════════════════════════════════════════════════════════════════════
// 共享工具函数 / Shared Utility Functions
// ═══════════════════════════════════════════════════════════════════════════

/// 指数衰减因子 / Exponential decay factor
///
/// 衰减公式：`factor = 2^(-elapsed / half_life)`
/// - elapsed=0 → factor=1.0（完整强度）
/// - elapsed=half_life → factor=0.5（半衰期）
/// - elapsed→∞ → factor→0（完全衰减）
///
/// Decay formula: exponential half-life decay.
#[inline]
pub fn exponential_decay(elapsed: f64, half_life: f64) -> f64 {
    if half_life <= 0.0 {
        return 0.0;
    }
    let e = elapsed.max(0.0);
    2f64.powf(-e / half_life)
}

/// f32 指数衰减因子 / f32 Exponential decay factor
///
/// f32 版本的 `exponential_decay`，用于对性能敏感且不需要 f64 精度的场景。
/// f32 version of `exponential_decay` for performance-sensitive paths.
#[inline]
pub fn exponential_decay_f32(elapsed: f32, half_life: f32) -> f32 {
    if half_life <= 0.0 {
        return 0.0;
    }
    let e = elapsed.max(0.0);
    2f32.powf(-e / half_life)
}

/// 指数移动平均 / Exponential Moving Average (EMA)
///
/// EMA 公式：`old + alpha * (new - old)`，等价于 `old * (1-alpha) + new * alpha`。
/// - alpha=0 → 保持旧值 / keep old
/// - alpha=1 → 完全替换 / full replacement
/// - alpha∈(0,1) → 平滑过渡 / smooth transition
///
/// EMA formula: `old + alpha * (new - old)`, equivalent to `old * (1-alpha) + new * alpha`.
#[inline]
pub fn ema(old: f64, new: f64, alpha: f64) -> f64 {
    old + alpha * (new - old)
}

/// f32 指数移动平均 / f32 Exponential Moving Average (EMA)
///
/// f32 版本的 EMA，用于对性能敏感的路径。
/// f32 version of EMA for performance-sensitive paths.
#[inline]
pub fn ema_f32(old: f32, new: f32, alpha: f32) -> f32 {
    old + alpha * (new - old)
}

/// 关系阶段缩放因子 / Relationship stage scaling factor
///
/// 关系越深，共振越强——初识时轻微，深度时强化。
/// Deeper relationship produces stronger resonance.
///
/// | 序数 | 阶段 | 缩放 |
/// |------|------|------|
/// | 0 | 初识 / Acquaintance | 0.5 |
/// | 1 | 熟悉 / Familiar | 0.75 |
/// | 2 | 信任 / Trusted | 1.0 |
/// | 3 | 深度 / Deep | 1.2 |
/// | _ | 未知 / Unknown | 1.0 |
#[inline]
pub fn relation_scale(ordinal: u8) -> f32 {
    match ordinal {
        0 => 0.5,  // 初识：轻微 / Acquaintance: subtle
        1 => 0.75, // 熟悉：中等 / Familiar: moderate
        2 => 1.0,  // 信任：标准 / Trusted: standard
        3 => 1.2,  // 深度：强化 / Deep: enhanced
        _ => 1.0,  // 未知：标准 / Unknown: standard
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 共振引擎核心接口 / Core Resonance Engine Trait
// ═══════════════════════════════════════════════════════════════════════════

/// 共振引擎核心接口 / Core resonance engine trait
///
/// 数字生命中所有共振引擎的统一接口——
/// 无论是仪式、脆弱还是好奇心，都在情感空间中产生回响。
/// 此 trait 定义了时间步进、当前情感增量和活跃度查询的统一契约。
///
/// Unified interface for all resonance engines in the digital life system.
/// Whether ritual, vulnerability, or curiosity, all produce emotional echoes
/// in PAD space. This trait defines a uniform contract for time evolution,
/// current emotional delta, and activity level.
pub trait ResonanceEngine: Send + Sync {
    /// 当前 PAD 情感增量 / Current PAD emotional delta
    ///
    /// 返回当前时刻所有活跃共振的叠加 PAD 增量。
    /// Returns the combined PAD delta from all active resonances at this moment.
    fn current_pad_delta(&self, now_secs: f64) -> PadDelta;

    /// 时间步进 — 衰减/清理 / Time tick — decay/cleanup
    ///
    /// 推进共振引擎的内部状态：衰减脉冲、清理过期项。
    /// Advances internal state: decay pulses, remove expired entries.
    fn tick(&mut self, now_secs: f64);

    /// 活跃度 (0.0-1.0) / Activity level
    ///
    /// 0.0 = 无活跃共振，1.0 = 满载共振。
    /// 用于监控和调度优先级。
    fn activity(&self) -> f32;

    /// 共振类型标签 / Resonance type label (for prompt injection)
    ///
    /// 中文/英文双语标签，用于 prompt 注入时标识来源。
    fn resonance_label(&self) -> &'static str;

    /// 生成 prompt 注入片段 / Generate prompt injection fragment
    ///
    /// 当 PAD 增量低于阈值时返回空字符串（不注入）。
    /// 默认实现可被覆盖以定制格式。
    ///
    /// Returns empty string when PAD delta is below threshold (no injection).
    /// Default implementation can be overridden for custom formatting.
    fn prompt_fragment(&self, now_secs: f64) -> String {
        let (p, a, d) = self.current_pad_delta(now_secs);
        if p.abs() < 0.01 && a.abs() < 0.01 {
            return String::new();
        }
        format!(
            "[{label}] PAD: P{p:+.3} A{a:+.3} D{d:+.3} — 情感回响仍在",
            label = self.resonance_label(),
            p = p,
            a = a,
            d = d
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 共振活跃度快照 / Resonance Activity Snapshot
// ═══════════════════════════════════════════════════════════════════════════

/// 共振活跃度快照 / Resonance activity snapshot
///
/// 统一记录各共振引擎的活跃度，用于监控和调度。
/// Uniformly records activity levels across resonance engines for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceSnapshot {
    /// 仪式共振活跃度 / Ritual resonance activity
    pub ritual: f32,
    /// 脆弱共振活跃度 / Vulnerability resonance activity
    pub vulnerability: f32,
    /// 好奇共振活跃度 / Curiosity resonance activity
    pub curiosity: f32,
}

impl ResonanceSnapshot {
    /// 总活跃度 (0.0-1.0) / Total activity level
    ///
    /// 取三者最大值——任一共振活跃即代表数字生命"有情感回响"。
    pub fn total(&self) -> f32 {
        self.ritual.max(self.vulnerability).max(self.curiosity)
    }

    /// 是否有任何活跃共振 / Whether any resonance is active
    pub fn is_active(&self) -> bool {
        self.total() > 0.01
    }
}

impl Default for ResonanceSnapshot {
    fn default() -> Self {
        Self {
            ritual: 0.0,
            vulnerability: 0.0,
            curiosity: 0.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 衰减函数测试 / Decay Function Tests ──

    #[test]
    fn test_exponential_decay_full_at_zero() {
        // 零 elapsed → 完整强度 / Zero elapsed → full strength
        assert!((exponential_decay(0.0, 60.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_decay_half_at_half_life() {
        // 半衰期处 → 0.5 / At half-life → 0.5
        assert!((exponential_decay(60.0, 60.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_decay_near_zero_far_future() {
        // 远未来 → ≈0 / Far future → ≈0
        assert!(exponential_decay(6000.0, 60.0) < 0.001);
    }

    #[test]
    fn test_exponential_decay_negative_elapsed_clamped() {
        // 负 elapsed → 按 0 处理 / Negative elapsed → treated as 0
        assert!((exponential_decay(-10.0, 60.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_decay_zero_half_life() {
        // 零半衰期 → 0（安全退化）/ Zero half-life → 0 (safe degeneration)
        assert_eq!(exponential_decay(10.0, 0.0), 0.0);
    }

    // ── 关系缩放测试 / Relation Scale Tests ──

    #[test]
    fn test_relation_scale_stages() {
        assert_eq!(relation_scale(0), 0.5);
        assert_eq!(relation_scale(1), 0.75);
        assert_eq!(relation_scale(2), 1.0);
        assert_eq!(relation_scale(3), 1.2);
    }

    #[test]
    fn test_relation_scale_unknown_ordinal() {
        // 未知序数 → 标准缩放 / Unknown ordinal → standard scale
        assert_eq!(relation_scale(255), 1.0);
    }

    #[test]
    fn test_relation_scale_monotonic() {
        // 关系越深缩放越大（0-3区间）/ Deeper relationship → larger scale
        assert!(relation_scale(0) < relation_scale(1));
        assert!(relation_scale(1) < relation_scale(2));
        assert!(relation_scale(2) < relation_scale(3));
    }

    // ── PAD 模长测试 / PAD Magnitude Tests ──

    #[test]
    fn test_pad_magnitude_zero() {
        assert_eq!(pad_magnitude(0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn test_pad_magnitude_positive() {
        let m = pad_magnitude(0.3, 0.4, 0.0);
        assert!((m - 0.5).abs() < 1e-6); // 3-4-5 三角
    }

    // ── 快照测试 / Snapshot Tests ──

    #[test]
    fn test_snapshot_total_max() {
        let s = ResonanceSnapshot {
            ritual: 0.1,
            vulnerability: 0.5,
            curiosity: 0.3,
        };
        assert!((s.total() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_snapshot_is_active() {
        let s = ResonanceSnapshot::default();
        assert!(!s.is_active());

        let s = ResonanceSnapshot {
            ritual: 0.0,
            vulnerability: 0.0,
            curiosity: 0.5,
        };
        assert!(s.is_active());
    }

    // ── Trait 默认 prompt_fragment 测试 / Trait Default prompt_fragment ──

    /// 测试用桩引擎 / Test stub engine
    struct StubEngine {
        pad: PadDelta,
        label: &'static str,
    }

    impl ResonanceEngine for StubEngine {
        fn current_pad_delta(&self, _now: f64) -> PadDelta {
            self.pad
        }
        fn tick(&mut self, _now: f64) {}
        fn activity(&self) -> f32 {
            pad_magnitude(self.pad.0, self.pad.1, self.pad.2) / 3.0
        }
        fn resonance_label(&self) -> &'static str {
            self.label
        }
    }

    #[test]
    fn test_default_prompt_fragment_below_threshold() {
        let e = StubEngine {
            pad: (0.005, 0.005, 0.0),
            label: "测试/Test",
        };
        assert!(e.prompt_fragment(0.0).is_empty());
    }

    #[test]
    fn test_default_prompt_fragment_above_threshold() {
        let e = StubEngine {
            pad: (0.15, 0.35, -0.20),
            label: "测试/Test",
        };
        let frag = e.prompt_fragment(0.0);
        assert!(frag.contains("[测试/Test]"));
        assert!(frag.contains("P+0.150"));
        assert!(frag.contains("A+0.350"));
    }

    #[test]
    fn test_default_prompt_fragment_zero_pad() {
        let e = StubEngine {
            pad: (0.0, 0.0, 0.0),
            label: "空/Empty",
        };
        assert!(e.prompt_fragment(0.0).is_empty());
    }
}
