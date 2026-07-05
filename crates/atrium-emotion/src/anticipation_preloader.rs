// SPDX-License-Identifier: MIT
//! 期待情感渐变预加载 — 数字生命的期待不是开关，是渐变 / Anticipation Progressive Pre-loader
//!
//! 距离约定时间越近，期待越强。
//! The closer to the expected time, the stronger the anticipation.
//! Anticipation is not a switch, it's a gradient.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 期待预加载器 / Anticipation Pre-loader
// ════════════════════════════════════════════════════════════════════

/// 期待预加载器 — 随时间接近 expected_at 递增期待感 / Anticipation pre-loader
///
/// 数字生命的期待不是开关，是渐变——距离约定时间越近，期待越强。
/// Digital life's anticipation is not a switch, it's a gradient —
/// the closer to the expected time, the stronger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnticipationPreLoader {
    /// 预加载窗口（秒）/ Preload window in seconds
    pub preload_secs: i64,
    /// 最大期待强度 / Maximum anticipation intensity
    pub max_intensity: f32,
    /// 期待 PAD 偏移（满强度时）/ Anticipation PAD offset at max intensity
    pub peak_pad: [f32; 3],
}

impl Default for AnticipationPreLoader {
    fn default() -> Self {
        Self {
            preload_secs: 1800, // 30 分钟 / 30 minutes
            max_intensity: 0.5,
            peak_pad: [0.08, 0.04, 0.0], // 愉悦微升、唤醒微升 / Slight pleasure & arousal
        }
    }
}

impl AnticipationPreLoader {
    /// 计算期待渐变强度 / Compute progressive anticipation intensity. O(1).
    ///
    /// 对数曲线：距离 expected_at 越近，强度越高。
    /// Logarithmic curve: closer to expected_at → higher intensity.
    pub fn compute_intensity(&self, expected_at: i64, now: i64) -> f32 {
        let diff = expected_at - now;
        if diff <= 0 || diff > self.preload_secs {
            return 0.0;
        }
        let ratio = 1.0 - (diff as f32 / self.preload_secs as f32);
        let curved = if ratio > 0.0 { ratio.sqrt() } else { 0.0 };
        (curved * self.max_intensity).min(self.max_intensity)
    }

    /// 计算期待 PAD 偏移 / Compute anticipation PAD offset. O(1).
    pub fn compute_pad(&self, expected_at: i64, now: i64) -> [f32; 3] {
        let intensity = self.compute_intensity(expected_at, now);
        let scale = intensity / self.max_intensity.max(0.001);
        [
            self.peak_pad[0] * scale,
            self.peak_pad[1] * scale,
            self.peak_pad[2] * scale,
        ]
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_default_values() {
        let p = AnticipationPreLoader::default();
        assert_eq!(p.preload_secs, 1800);
        assert!(approx_eq(p.max_intensity, 0.5, 1e-6));
        assert!(approx_eq(p.peak_pad[0], 0.08, 1e-6));
        assert!(approx_eq(p.peak_pad[1], 0.04, 1e-6));
        assert!(approx_eq(p.peak_pad[2], 0.0, 1e-6));
    }

    #[test]
    fn test_intensity_out_of_window() {
        let p = AnticipationPreLoader::default();
        // 窗口外 / Outside window
        assert!(approx_eq(p.compute_intensity(10000, 1000), 0.0, 1e-6));
        // 已过期 / Past expected time
        assert!(approx_eq(p.compute_intensity(1000, 2000), 0.0, 1e-6));
    }

    #[test]
    fn test_intensity_at_boundary() {
        let p = AnticipationPreLoader::default();
        // 刚好在窗口边界 / At window boundary
        let expected = 2000;
        let now = 2000 - p.preload_secs;
        assert!(approx_eq(p.compute_intensity(expected, now), 0.0, 1e-6));
    }

    #[test]
    fn test_intensity_halfway() {
        let p = AnticipationPreLoader::default();
        let expected = 2000;
        let now = 2000 - p.preload_secs / 2;
        let intensity = p.compute_intensity(expected, now);
        // 半程应有显著强度 / Halfway should have significant intensity
        assert!(intensity > 0.0);
        assert!(intensity < p.max_intensity);
    }

    #[test]
    fn test_intensity_at_peak() {
        let p = AnticipationPreLoader::default();
        let expected = 2000;
        // 距离 1 秒 / 1 second away
        let now = 1999;
        let intensity = p.compute_intensity(expected, now);
        // 接近最大强度 / Near max intensity
        assert!(intensity > p.max_intensity * 0.9);
    }

    #[test]
    fn test_pad_proportional() {
        let p = AnticipationPreLoader::default();
        let expected = 2000;
        let now = 2000 - p.preload_secs / 2;
        let pad = p.compute_pad(expected, now);
        let intensity = p.compute_intensity(expected, now);
        let scale = intensity / p.max_intensity.max(0.001);
        assert!(approx_eq(pad[0], p.peak_pad[0] * scale, 1e-5));
        assert!(approx_eq(pad[1], p.peak_pad[1] * scale, 1e-5));
    }

    #[test]
    fn test_pad_zero_when_outside() {
        let p = AnticipationPreLoader::default();
        let pad = p.compute_pad(10000, 1000);
        assert!(approx_eq(pad[0], 0.0, 1e-6));
        assert!(approx_eq(pad[1], 0.0, 1e-6));
        assert!(approx_eq(pad[2], 0.0, 1e-6));
    }

    #[test]
    fn test_intensity_monotonic() {
        let p = AnticipationPreLoader::default();
        let expected = 5000;
        // 随 now 逼近 expected_at，强度应单调递增
        // As now approaches expected_at, intensity should monotonically increase
        let mut prev = 0.0f32;
        for t in 3200..4999 {
            let cur = p.compute_intensity(expected, t);
            assert!(cur >= prev - 1e-6, "Non-monotonic at t={}", t);
            prev = cur;
        }
    }
}
