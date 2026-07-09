// SPDX-License-Identifier: MIT
// LongingParams + LongingState — 想念参数与状态 / Longing params & state

use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LongingParams {
    /// 想念基线 PAD / Longing PAD baseline (pleasure↓, arousal↑微, dominance↓)
    pub baseline: [f64; 3],
    /// OU 波动率 / OU volatility (想念时的情感微扰)
    pub volatility: f64,
    /// 均值回归率 / Mean reversion rate (向当前基线拉回的速率)
    pub mean_reversion: f64,
    /// 想念起始阈值（秒）/ Onset threshold in seconds
    pub onset_threshold_secs: u64,
    /// 想念饱和阈值（秒）/ Saturation threshold (到达想念基线的最短时长)
    pub saturation_threshold_secs: u64,
}

impl Default for LongingParams {
    fn default() -> Self {
        Self {
            baseline: [-0.25, 0.05, -0.15],
            volatility: 0.001,
            mean_reversion: 0.0005,
            onset_threshold_secs: 600,
            saturation_threshold_secs: 7200,
        }
    }
}

impl LongingParams {
    /// 一步 OU 过程，向指定基线漂移，返回 [ΔP, ΔA, ΔD]
    /// One OU step toward the given baseline, returns [ΔP, ΔA, ΔD].
    ///
    /// @param current 当前 PAD 值 / Current PAD values
    /// @param target  目标基线 / Target baseline
    /// @return [ΔP, ΔA, ΔD] 增量 / Incremental delta
    pub fn step_toward(&self, current: [f64; 3], target: [f64; 3]) -> [f64; 3] {
        let mut rng = rand::thread_rng();
        let mut delta = [0.0f64; 3];
        for i in 0..3 {
            let noise: f64 = rng.gen_range(-1.0..1.0);
            delta[i] = self.mean_reversion * (target[i] - current[i]) + self.volatility * noise;
        }
        delta
    }
}

/// 想念运行时状态 / Longing runtime state
///
/// 由 CoreService::longing_tick() 每 20 tick 更新，
/// 存储当前想念强度、离开时长、插值后的漂移基线。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LongingState {
    /// 当前想念强度 [0, 1] / Current longing intensity
    pub intensity: f32,
    /// 用户离开时长（秒）/ Away duration in seconds
    pub away_secs: u64,
    /// 当前漂移基线（插值结果）/ Current interpolated baseline
    pub current_baseline: [f64; 3],
    /// 上次更新时间戳 / Last update timestamp (unix epoch seconds)
    pub last_update: i64,
}

impl Default for LongingState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            away_secs: 0,
            current_baseline: [0.0, 0.0, 0.0],
            last_update: 0,
        }
    }
}

impl LongingState {
    /// 构造初始想念状态（零强度，基线已知）
    /// Create initial longing state with zero intensity and known baseline.
    pub fn new(baseline: [f64; 3]) -> Self {
        Self {
            intensity: 0.0,
            away_secs: 0,
            current_baseline: baseline,
            last_update: chrono::Utc::now().timestamp(),
        }
    }

    /// 根据离开时长计算想念强度 [0, 1]
    /// Compute longing intensity from away duration.
    ///
    /// @param away_secs 离开秒数 / Away seconds
    /// @param params 想念参数 / Longing parameters
    /// @param rel_mult 关系乘数 (0.8~1.2) / Relationship multiplier
    /// @param engagement 用户参与度 (0~1) / User engagement score
    /// @return 想念强度 [0, 1] / Longing intensity
    pub fn compute_intensity(
        away_secs: u64,
        params: &LongingParams,
        rel_mult: f32,
        engagement: f32,
    ) -> f32 {
        if away_secs <= params.onset_threshold_secs {
            return 0.0;
        }
        if away_secs >= params.saturation_threshold_secs {
            return rel_mult * (0.5 + 0.5 * engagement).clamp(0.0, 1.0);
        }
        let raw = (away_secs - params.onset_threshold_secs) as f32
            / (params.saturation_threshold_secs - params.onset_threshold_secs) as f32;
        raw * rel_mult * (0.5 + 0.5 * engagement).clamp(0.0, 1.0)
    }

    /// 线性插值基线 / Linearly interpolate between neutral and longing baselines.
    ///
    /// @param neutral 中性基线 / Neutral baseline
    /// @param longing 想念基线 / Longing baseline
    /// @param intensity 插值权重 [0, 1] / Interpolation weight
    /// @return 插值后的基线 / Interpolated baseline
    pub fn interpolate_baseline(
        neutral: &[f64; 3],
        longing: &[f64; 3],
        intensity: f32,
    ) -> [f64; 3] {
        let t = intensity as f64;
        [
            neutral[0] * (1.0 - t) + longing[0] * t,
            neutral[1] * (1.0 - t) + longing[1] * t,
            neutral[2] * (1.0 - t) + longing[2] * t,
        ]
    }

    /// 关系阶段门控 — 是否应表达想念 / Relationship stage gate for expressing longing.
    ///
    /// 门控规则：
    /// - Acquaintance (0): 不表达想念 / Never express longing
    /// - Familiar (1): 需更高强度（1.5x 阈值）/ Need higher intensity (1.5x threshold)
    /// - Trusted (2) / Deep (3): 正常阈值 / Normal threshold
    ///
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    ///   (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    /// @param threshold 想念表达阈值 / Longing expression threshold
    /// @return 是否应表达想念 / Whether longing should be expressed
    pub fn should_express_longing(&self, relation_ordinal: u8, threshold: f32) -> bool {
        match relation_ordinal {
            0 => false,                            // 初识不表达想念 / Acquaintance: never express
            1 => self.intensity > threshold * 1.5, // 熟悉需更高强度 / Familiar: need higher
            _ => self.intensity > threshold,       // 信任/深度正常阈值 / Trusted/Deep: normal
        }
    }
}
