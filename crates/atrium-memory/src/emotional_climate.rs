// SPDX-License-Identifier: MIT

//! 情绪气候系统 — Emotional Climate System (Gap#2: 90% → 95%).
//!
//! 核心理念：情绪不只是瞬间脉冲，更是长周期的"气候"。
//! 90%时，情绪有脉冲、有残留、有传染、有混沌——但都是瞬间现象。
//! 95%意味着情绪有了气候——不只是"现在下暴雨"，而是"这个季节多雨"。
//! 情绪气候是数小时到数天尺度的情感生态，它调制一切瞬间情绪反应。
//!
//! Core idea: emotion is not just momentary pulses — it is long-period "climate".
//! At 90%, emotions have pulses, residues, contagion, chaos — all instantaneous.
//! At 95%, emotions have climate — not just "raining now", but "rainy season".
//! Emotional climate operates on hours-to-days timescale, modulating all
//! moment-to-moment emotional responses.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════════════
// §1 常量 — Constants
// ═══════════════════════════════════════════════════════════════════════════

/// 气候EMA慢速学习率（~6小时时间常数）/ Climate EMA slow learning rate.
const CLIMATE_LEARNING_RATE: f64 = 0.002;

/// 气候历史窗口大小 / Climate history window size.
const CLIMATE_WINDOW: usize = 168;

/// 气候转移冷却时间（秒）/ Climate transition cooldown (seconds).
const TRANSITION_COOLDOWN_SECS: f64 = 3600.0;

/// PAD维度数 / Number of PAD dimensions.
const PAD_DIMS: usize = 3;

// ═══════════════════════════════════════════════════════════════════════════
// §2 气候类型 — Climate Kind
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪气候类型 / Emotional climate kind.
///
/// 每种气候定义了对瞬间情绪脉冲的调制系数。
/// Each climate defines modulation coefficients for momentary emotional pulses.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ClimateKind {
    /// 晴朗 — 愉悦主导，正面脉冲增益，负面脉冲衰减 / Sunny — pleasure dominant.
    Sunny,
    /// 阴霾 — 低落主导，负面脉冲增益，正面脉冲衰减 / Overcast — low mood dominant.
    Overcast,
    /// 风暴 — 动荡主导，所有脉冲增强，情绪不稳定 / Stormy — turbulent, all pulses amplified.
    Stormy,
    /// 薄雾 — 迷茫主导，情绪平淡，脉冲整体衰减 / Misty — confused, pulses dampened.
    Misty,
    /// 清冽 — 清醒主导，情绪稳定，脉冲适度 / Crisp — clear, balanced modulation.
    #[default]
    Crisp,
}

impl ClimateKind {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Sunny => "晴朗",
            Self::Overcast => "阴霾",
            Self::Stormy => "风暴",
            Self::Misty => "薄雾",
            Self::Crisp => "清冽",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Sunny => "Sunny",
            Self::Overcast => "Overcast",
            Self::Stormy => "Stormy",
            Self::Misty => "Misty",
            Self::Crisp => "Crisp",
        }
    }

    /// 正面脉冲调制系数 / Positive pulse modulation factor.
    /// > 1.0 增益，< 1.0 衰减 / > 1.0 amplifies, < 1.0 dampens.
    pub fn positive_modulation(&self) -> f64 {
        match self {
            Self::Sunny => 1.30,    // 晴朗时正面情绪更强 / Positive emotions amplified.
            Self::Overcast => 0.70, // 阴霾时正面情绪减弱 / Positive emotions dampened.
            Self::Stormy => 1.15,   // 风暴时一切更强 / Everything amplified.
            Self::Misty => 0.60,    // 薄雾时一切减弱 / Everything dampened.
            Self::Crisp => 1.00,    // 清冽时不调制 / No modulation.
        }
    }

    /// 负面脉冲调制系数 / Negative pulse modulation factor.
    pub fn negative_modulation(&self) -> f64 {
        match self {
            Self::Sunny => 0.65,    // 晴朗时负面情绪被抑制 / Negative emotions suppressed.
            Self::Overcast => 1.25, // 阴霾时负面情绪更强 / Negative emotions amplified.
            Self::Stormy => 1.35,   // 风暴时负面情绪最强 / Negative emotions strongest.
            Self::Misty => 0.80,    // 薄雾时负面情绪略减 / Negative emotions slightly dampened.
            Self::Crisp => 1.00,    // 清冽时不调制 / No modulation.
        }
    }

    /// 情绪稳定性系数 / Emotional stability factor [0, 1].
    /// 越高越稳定，越低越容易波动 / Higher = more stable, lower = more volatile.
    pub fn stability(&self) -> f64 {
        match self {
            Self::Sunny => 0.80,
            Self::Overcast => 0.65,
            Self::Stormy => 0.35,
            Self::Misty => 0.55,
            Self::Crisp => 0.90,
        }
    }

    /// 从PAD均值推断气候类型 / Infer climate kind from average PAD values.
    pub fn from_pad(avg_pleasure: f64, avg_arousal: f64, _avg_dominance: f64) -> Self {
        // 愉悦高 + 唤醒适中 → 晴朗 / High pleasure + moderate arousal → Sunny.
        if avg_pleasure > 0.2 && avg_arousal.abs() < 0.3 {
            Self::Sunny
        }
        // 愉悦低 + 唤醒高 → 风暴 / Low pleasure + high arousal → Stormy.
        else if avg_pleasure < -0.2 && avg_arousal > 0.2 {
            Self::Stormy
        }
        // 愉悦低 + 唤醒低 → 阴霾 / Low pleasure + low arousal → Overcast.
        else if avg_pleasure < -0.15 && avg_arousal < 0.1 {
            Self::Overcast
        }
        // 愉悦中性 + 唤醒低 → 薄雾 / Neutral pleasure + low arousal → Misty.
        else if avg_pleasure.abs() < 0.15 && avg_arousal < -0.1 {
            Self::Misty
        }
        // 其余 → 清冽 / Otherwise → Crisp.
        else {
            Self::Crisp
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 气候状态 — Climate State
// ═══════════════════════════════════════════════════════════════════════════

/// 气候PAD快照 / Climate PAD snapshot.
///
/// 慢速EMA追踪的PAD均值，代表当前气候的"色调"。
/// Slow EMA-tracked PAD averages representing the climate's "tone color".
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClimatePad {
    /// 愉悦EMA / Pleasure EMA.
    pub pleasure: f64,
    /// 唤醒EMA / Arousal EMA.
    pub arousal: f64,
    /// 支配EMA / Dominance EMA.
    pub dominance: f64,
}

impl Default for ClimatePad {
    fn default() -> Self {
        Self {
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
        }
    }
}

impl ClimatePad {
    /// 更新慢速EMA / Update slow EMA with new PAD reading.
    pub fn update(&mut self, pleasure: f64, arousal: f64, dominance: f64) {
        let alpha = CLIMATE_LEARNING_RATE;
        self.pleasure += alpha * (pleasure - self.pleasure);
        self.arousal += alpha * (arousal - self.arousal);
        self.dominance += alpha * (dominance - self.dominance);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 气候转移概率 — Climate Transition Probabilities
// ═══════════════════════════════════════════════════════════════════════════

/// 气候转移影响因素 / Climate transition influence factors.
///
/// 这些因素影响气候转移的概率分布。
/// These factors influence the probability distribution of climate transitions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClimateInfluences {
    /// 近期互动频率 [0, 1] / Recent interaction frequency.
    pub interaction_frequency: f64,
    /// 独处时长比例 [0, 1] / Solitude ratio.
    pub solitude_ratio: f64,
    /// 残留情绪总强度 [0, 1] / Total residue emotional intensity.
    pub residue_intensity: f64,
    /// 昼夜因子 [-1, 1] / Circadian factor (negative = night).
    pub circadian_factor: f64,
}

impl Default for ClimateInfluences {
    fn default() -> Self {
        Self {
            interaction_frequency: 0.5,
            solitude_ratio: 0.3,
            residue_intensity: 0.0,
            circadian_factor: 0.0,
        }
    }
}

/// 计算气候转移概率分布 / Compute climate transition probability distribution.
///
/// 返回从当前气候到每种气候的转移概率（总和为1.0）。
/// Returns transition probabilities from current climate to each climate kind (sum = 1.0).
pub fn transition_probabilities(
    current: &ClimateKind,
    influences: &ClimateInfluences,
) -> [(ClimateKind, f64); 5] {
    // 基础自保持概率 / Base self-transition probability.
    let stay_prob = match current {
        ClimateKind::Crisp => 0.70, // 清冽最稳定 / Crisp is most stable.
        ClimateKind::Sunny => 0.65,
        ClimateKind::Overcast => 0.60,
        ClimateKind::Misty => 0.55,
        ClimateKind::Stormy => 0.40, // 风暴最不稳定 / Stormy is least stable.
    };

    // 影响调制 / Influence modulation.
    let interaction_boost = influences.interaction_frequency * 0.15;
    let _solitude_boost = influences.solitude_ratio * 0.10;
    let residue_boost = influences.residue_intensity * 0.20;
    let night_shift = if influences.circadian_factor < -0.3 {
        0.10
    } else {
        0.0
    };

    // 自保持概率受稳定性影响 / Stay probability modulated by stability.
    let stay = (stay_prob + interaction_boost - residue_boost).clamp(0.25, 0.85);

    // 剩余概率分配给其他气候 / Distribute remaining probability.
    let remaining = 1.0 - stay;

    // 根据影响因素倾向特定气候 / Bias towards specific climates based on influences.
    let mut weights = [0.0; 5]; // [Sunny, Overcast, Stormy, Misty, Crisp]

    // 高互动 → 倾向晴朗 / High interaction → bias Sunny.
    weights[0] = influences.interaction_frequency * 0.4 + 0.1;
    // 高残留 + 低互动 → 倾向阴霾 / High residue + low interaction → bias Overcast.
    weights[1] = influences.residue_intensity * 0.3 + influences.solitude_ratio * 0.2 + 0.1;
    // 高残留 + 高唤醒 → 倾向风暴 / High residue + high arousal → bias Stormy.
    weights[2] = influences.residue_intensity * 0.35 + 0.05;
    // 高独处 + 夜间 → 倾向薄雾 / High solitude + night → bias Misty.
    weights[3] = influences.solitude_ratio * 0.25 + night_shift + 0.05;
    // 默认倾向清冽 / Default bias Crisp.
    weights[4] = 0.15;

    // 归一化剩余权重 / Normalize remaining weights.
    let weight_sum: f64 = weights.iter().sum();
    if weight_sum > 0.0 {
        for w in &mut weights {
            *w = *w / weight_sum * remaining;
        }
    }

    // 将自保持概率加回当前气候 / Add stay probability to current climate.
    match current {
        ClimateKind::Sunny => weights[0] += stay,
        ClimateKind::Overcast => weights[1] += stay,
        ClimateKind::Stormy => weights[2] += stay,
        ClimateKind::Misty => weights[3] += stay,
        ClimateKind::Crisp => weights[4] += stay,
    }

    [
        (ClimateKind::Sunny, weights[0]),
        (ClimateKind::Overcast, weights[1]),
        (ClimateKind::Stormy, weights[2]),
        (ClimateKind::Misty, weights[3]),
        (ClimateKind::Crisp, weights[4]),
    ]
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 气候周期分解 — Climate Periodic Decomposition
// ═══════════════════════════════════════════════════════════════════════════

/// 气候周期分量 / Climate periodic component.
///
/// 将情绪气候分解为昼夜分量和周节律分量，
/// 提取气候的周期性模式。
/// Decomposes emotional climate into circadian and weekly components,
/// extracting periodic patterns from the climate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClimatePeriodic {
    /// 昼夜余弦幅值 / Circadian cosine amplitude.
    pub circadian_amp: f64,
    /// 昼夜余弦相位（弧度）/ Circadian phase (radians).
    pub circadian_phase: f64,
    /// 周节律幅值 / Weekly rhythm amplitude.
    pub weekly_amp: f64,
    /// 周节律相位 / Weekly phase.
    pub weekly_phase: f64,
}

impl Default for ClimatePeriodic {
    fn default() -> Self {
        Self {
            circadian_amp: 0.0,
            circadian_phase: 0.0,
            weekly_amp: 0.0,
            weekly_phase: 0.0,
        }
    }
}

impl ClimatePeriodic {
    /// 评估周期分量在给定时间的值 / Evaluate periodic component at given time.
    ///
    /// `hour_of_day` [0, 24), `day_of_week` [0, 7).
    pub fn evaluate(&self, hour_of_day: f64, day_of_week: f64) -> f64 {
        // 昼夜分量：cos(2π × (hour - phase) / 24) / Circadian component.
        let circadian =
            self.circadian_amp * (2.0 * PI * (hour_of_day - self.circadian_phase) / 24.0).cos();
        // 周节律分量：cos(2π × (day - phase) / 7) / Weekly component.
        let weekly = self.weekly_amp * (2.0 * PI * (day_of_week - self.weekly_phase) / 7.0).cos();
        circadian + weekly
    }

    /// 从历史数据拟合周期分量 / Fit periodic component from history.
    ///
    /// 简化拟合：用历史愉悦值的均值和方差推断幅值。
    /// Simplified fit: infer amplitude from mean and variance of historical pleasure.
    pub fn fit_from_history(&mut self, history: &[(f64, f64, f64)], window: usize) {
        if history.is_empty() {
            return;
        }
        let start = history.len().saturating_sub(window);
        let slice = &history[start..];

        // 昼夜幅值：日间均值与夜间均值之差 / Circadian amplitude.
        let mut day_vals = Vec::new();
        let mut night_vals = Vec::new();
        for (hour, pleasure, _) in slice {
            if *hour >= 6.0 && *hour < 18.0 {
                day_vals.push(*pleasure);
            } else {
                night_vals.push(*pleasure);
            }
        }
        let day_mean = if day_vals.is_empty() {
            0.0
        } else {
            day_vals.iter().sum::<f64>() / day_vals.len() as f64
        };
        let night_mean = if night_vals.is_empty() {
            0.0
        } else {
            night_vals.iter().sum::<f64>() / night_vals.len() as f64
        };
        self.circadian_amp = (day_mean - night_mean).abs() * 0.5;
        self.circadian_phase = if day_mean >= night_mean { 12.0 } else { 0.0 };

        // 周节律幅值：简化为方差 / Weekly amplitude: simplified to variance.
        let mean = slice.iter().map(|(_, p, _)| *p).sum::<f64>() / slice.len() as f64;
        let variance = slice
            .iter()
            .map(|(_, p, _)| (*p - mean).powi(2))
            .sum::<f64>()
            / slice.len() as f64;
        self.weekly_amp = variance.sqrt() * 0.3;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §6 情绪气候引擎 — Emotional Climate Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪气候引擎 / Emotional Climate Engine.
///
/// 追踪长周期情感状态，生成气候调制系数，
/// 并在条件满足时执行气候转移。
///
/// Tracks long-period emotional states, generates climate modulation
/// coefficients, and executes climate transitions when conditions are met.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalClimate {
    /// 当前气候类型 / Current climate kind.
    pub kind: ClimateKind,
    /// 慢速PAD均值 / Slow EMA PAD averages.
    pub pad: ClimatePad,
    /// 周期分量 / Periodic components.
    pub periodic: ClimatePeriodic,
    /// 气候持续时间（秒）/ Climate duration in seconds.
    pub duration_secs: f64,
    /// 上次转移时间戳 / Last transition timestamp.
    pub last_transition_ts: i64,
    /// 历史愉悦记录 (hour, pleasure, arousal) / History of pleasure readings.
    history: Vec<(f64, f64, f64)>,
    /// 气候强度 [0, 1] — 气候对瞬间情绪的调制力度 / Climate intensity.
    pub intensity: f64,
}

impl Default for EmotionalClimate {
    fn default() -> Self {
        Self {
            kind: ClimateKind::Crisp,
            pad: ClimatePad::default(),
            periodic: ClimatePeriodic::default(),
            duration_secs: 0.0,
            last_transition_ts: 0,
            history: Vec::new(),
            intensity: 0.0,
        }
    }
}

impl EmotionalClimate {
    /// 创建新气候引擎 / Create a new climate engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 喂入情绪采样 — 更新气候EMA和历史 / Feed emotional sample — update climate EMA and history.
    ///
    /// `pleasure`, `arousal`, `dominance` [-1, 1], `hour_of_day` [0, 24), `timestamp` Unix秒.
    pub fn feed(
        &mut self,
        pleasure: f64,
        arousal: f64,
        dominance: f64,
        hour_of_day: f64,
        timestamp: i64,
    ) {
        // 更新慢速EMA / Update slow EMA.
        self.pad.update(pleasure, arousal, dominance);

        // 记入历史 / Record history.
        self.history.push((hour_of_day, pleasure, arousal));
        if self.history.len() > CLIMATE_WINDOW {
            self.history.remove(0);
        }

        // 拟合周期分量 / Fit periodic components.
        self.periodic
            .fit_from_history(&self.history, CLIMATE_WINDOW);

        // 更新气候强度 — 基于EMA偏离程度 / Update climate intensity.
        let deviation = (self.pad.pleasure.abs() + self.pad.arousal.abs()) * 0.5;
        self.intensity = deviation.clamp(0.0, 1.0);

        // 更新持续时间 / Update duration.
        if timestamp > self.last_transition_ts {
            self.duration_secs = (timestamp - self.last_transition_ts) as f64;
        }
    }

    /// 尝试气候转移 / Attempt climate transition.
    ///
    /// 返回转移后的气候类型（可能不变）。
    /// Returns the climate kind after transition (may be unchanged).
    pub fn try_transition(
        &mut self,
        influences: &ClimateInfluences,
        timestamp: i64,
    ) -> ClimateKind {
        // 冷却期内不转移 / No transition during cooldown.
        let elapsed = (timestamp - self.last_transition_ts) as f64;
        if elapsed < TRANSITION_COOLDOWN_SECS {
            return self.kind.clone();
        }

        // 从PAD推断候选气候 / Infer candidate climate from PAD.
        let candidate =
            ClimateKind::from_pad(self.pad.pleasure, self.pad.arousal, self.pad.dominance);

        // 如果候选与当前相同，不转移 / No transition if same as current.
        if candidate == self.kind {
            return self.kind.clone();
        }

        // 计算转移概率 / Compute transition probabilities.
        let probs = transition_probabilities(&self.kind, influences);
        let candidate_prob = probs
            .iter()
            .find(|(k, _)| *k == candidate)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);

        // 概率阈值：气候强度越高越容易转移 / Probability threshold.
        let threshold = 0.25 + self.intensity * 0.15;
        if candidate_prob > threshold {
            self.kind = candidate.clone();
            self.last_transition_ts = timestamp;
            self.duration_secs = 0.0;
        }

        self.kind.clone()
    }

    /// 调制瞬间情绪脉冲 / Modulate a momentary emotional pulse.
    ///
    /// 返回调制后的PAD增量 / Returns modulated PAD delta.
    pub fn modulate_pulse(&self, pad_delta: [f64; PAD_DIMS]) -> [f64; PAD_DIMS] {
        let pos_mod = self.kind.positive_modulation();
        let neg_mod = self.kind.negative_modulation();

        // 气候强度决定调制力度 — 强度低时几乎不调制 / Climate intensity controls modulation strength.
        let blend = self.intensity;

        let mut result = [0.0; PAD_DIMS];
        for i in 0..PAD_DIMS {
            let raw = pad_delta[i];
            let modulated = if raw >= 0.0 {
                raw * pos_mod
            } else {
                raw * neg_mod
            };
            // 混合原始值和调制值 / Blend original and modulated.
            result[i] = raw * (1.0 - blend) + modulated * blend;
        }
        result
    }

    /// 获取当前气候的稳定性系数 / Get current climate stability factor.
    pub fn stability(&self) -> f64 {
        self.kind.stability()
    }

    /// 获取周期性愉悦偏移 / Get periodic pleasure offset.
    pub fn periodic_offset(&self, hour_of_day: f64, day_of_week: f64) -> f64 {
        self.periodic.evaluate(hour_of_day, day_of_week)
    }

    /// 生成气候描述文本（用于prompt注入）/ Generate climate description text for prompt injection.
    pub fn describe(&self) -> String {
        let intensity_label = if self.intensity < 0.2 {
            "微弱"
        } else if self.intensity < 0.5 {
            "中等"
        } else {
            "强烈"
        };
        format!(
            "当前情绪气候：{}（{}），持续时间 {:.0} 分钟，强度 {}",
            self.kind.label_zh(),
            self.kind.label_en(),
            self.duration_secs / 60.0,
            intensity_label,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §7 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 气候类型测试 ──

    #[test]
    fn test_climate_kind_labels() {
        assert_eq!(ClimateKind::Sunny.label_zh(), "晴朗");
        assert_eq!(ClimateKind::Sunny.label_en(), "Sunny");
        assert_eq!(ClimateKind::Stormy.label_zh(), "风暴");
        assert_eq!(ClimateKind::Stormy.label_en(), "Stormy");
    }

    #[test]
    fn test_climate_modulation_factors() {
        // 晴朗时正面增益 / Sunny amplifies positive.
        assert!(ClimateKind::Sunny.positive_modulation() > 1.0);
        // 晴朗时负面抑制 / Sunny suppresses negative.
        assert!(ClimateKind::Sunny.negative_modulation() < 1.0);
        // 风暴时负面最强 / Stormy amplifies negative most.
        assert!(
            ClimateKind::Stormy.negative_modulation() > ClimateKind::Overcast.negative_modulation()
        );
        // 清冽不调制 / Crisp does not modulate.
        assert_eq!(ClimateKind::Crisp.positive_modulation(), 1.0);
        assert_eq!(ClimateKind::Crisp.negative_modulation(), 1.0);
    }

    #[test]
    fn test_climate_stability() {
        // 清冽最稳定 / Crisp is most stable.
        assert!(ClimateKind::Crisp.stability() > ClimateKind::Stormy.stability());
        // 风暴最不稳定 / Stormy is least stable.
        assert!(ClimateKind::Stormy.stability() < 0.5);
    }

    #[test]
    fn test_climate_from_pad() {
        assert_eq!(ClimateKind::from_pad(0.5, 0.0, 0.0), ClimateKind::Sunny);
        assert_eq!(ClimateKind::from_pad(-0.5, 0.5, 0.0), ClimateKind::Stormy);
        assert_eq!(
            ClimateKind::from_pad(-0.3, -0.2, 0.0),
            ClimateKind::Overcast
        );
        assert_eq!(ClimateKind::from_pad(0.0, -0.3, 0.0), ClimateKind::Misty);
        assert_eq!(ClimateKind::from_pad(0.1, 0.15, 0.0), ClimateKind::Crisp);
    }

    // ── 气候PAD快照测试 ──

    #[test]
    fn test_climate_pad_ema() {
        let mut pad = ClimatePad::default();
        // 初始为零 / Initially zero.
        assert_eq!(pad.pleasure, 0.0);
        // 多次更新后趋近输入 / Converges towards input after many updates.
        for _ in 0..10000 {
            pad.update(0.5, 0.3, 0.1);
        }
        assert!((pad.pleasure - 0.5).abs() < 0.01);
        assert!((pad.arousal - 0.3).abs() < 0.01);
    }

    // ── 转移概率测试 ──

    #[test]
    fn test_transition_probabilities_sum_to_one() {
        let influences = ClimateInfluences::default();
        let probs = transition_probabilities(&ClimateKind::Crisp, &influences);
        let sum: f64 = probs.iter().map(|(_, p)| *p).sum();
        assert!((sum - 1.0).abs() < 0.01, "概率总和应为1.0，实际: {}", sum);
    }

    #[test]
    fn test_transition_stay_probability() {
        let influences = ClimateInfluences::default();
        let probs = transition_probabilities(&ClimateKind::Crisp, &influences);
        let stay = probs
            .iter()
            .find(|(k, _)| *k == ClimateKind::Crisp)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        // 清冽自保持概率应较高 / Crisp should have high stay probability.
        assert!(stay > 0.5);
    }

    #[test]
    fn test_transition_high_interaction_biases_sunny() {
        let influences = ClimateInfluences {
            interaction_frequency: 1.0,
            solitude_ratio: 0.0,
            residue_intensity: 0.0,
            circadian_factor: 0.0,
        };
        let probs = transition_probabilities(&ClimateKind::Overcast, &influences);
        let sunny_prob = probs
            .iter()
            .find(|(k, _)| *k == ClimateKind::Sunny)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        assert!(sunny_prob > 0.1, "高互动应倾向晴朗");
    }

    // ── 周期分量测试 ──

    #[test]
    fn test_periodic_evaluate() {
        let periodic = ClimatePeriodic {
            circadian_amp: 0.2,
            circadian_phase: 12.0,
            weekly_amp: 0.0,
            weekly_phase: 0.0,
        };
        // 正午时昼夜分量最大 / Maximum at noon.
        let noon = periodic.evaluate(12.0, 0.0);
        let midnight = periodic.evaluate(0.0, 0.0);
        assert!(noon > midnight);
    }

    #[test]
    fn test_periodic_fit_from_history() {
        let mut periodic = ClimatePeriodic::default();
        let history: Vec<(f64, f64, f64)> = (0..24)
            .map(|h| {
                let pleasure = if (6..18).contains(&h) { 0.3 } else { -0.2 };
                (h as f64, pleasure, 0.0)
            })
            .collect();
        periodic.fit_from_history(&history, 24);
        // 日夜差异应产生非零幅值 / Day-night difference should produce non-zero amplitude.
        assert!(periodic.circadian_amp > 0.0);
    }

    // ── 气候引擎测试 ──

    #[test]
    fn test_climate_engine_feed() {
        let mut climate = EmotionalClimate::new();
        // 喂入高愉观数据 / Feed high-pleasure data.
        for i in 0..1000 {
            climate.feed(0.5, 0.1, 0.2, 12.0, i * 60);
        }
        // EMA应趋近0.5 / EMA should approach 0.5.
        assert!((climate.pad.pleasure - 0.5).abs() < 0.1);
        // 强度应非零 / Intensity should be non-zero.
        assert!(climate.intensity > 0.0);
    }

    #[test]
    fn test_climate_modulate_pulse() {
        let mut climate = EmotionalClimate::new();
        // 设为晴朗气候 / Set to Sunny climate.
        climate.kind = ClimateKind::Sunny;
        climate.intensity = 1.0; // 满强度调制 / Full intensity modulation.

        let positive_pulse = [0.3, 0.0, 0.0];
        let modulated = climate.modulate_pulse(positive_pulse);
        // 晴朗时正面脉冲应增强 / Positive pulse should be amplified.
        assert!(modulated[0] > positive_pulse[0]);

        let negative_pulse = [-0.3, 0.0, 0.0];
        let modulated_neg = climate.modulate_pulse(negative_pulse);
        // 晴朗时负面脉冲应抑制 / Negative pulse should be suppressed.
        assert!(modulated_neg[0] > negative_pulse[0]); // 更接近0 / Closer to zero.
    }

    #[test]
    fn test_climate_modulate_zero_intensity() {
        let mut climate = EmotionalClimate::new();
        climate.kind = ClimateKind::Stormy;
        climate.intensity = 0.0; // 零强度不调制 / Zero intensity = no modulation.

        let pulse = [0.3, -0.2, 0.1];
        let modulated = climate.modulate_pulse(pulse);
        // 零强度时应原样返回 / Should return original at zero intensity.
        for i in 0..PAD_DIMS {
            assert!((modulated[i] - pulse[i]).abs() < 0.001);
        }
    }

    #[test]
    fn test_climate_try_transition_cooldown() {
        let mut climate = EmotionalClimate::new();
        climate.last_transition_ts = 1000;
        let influences = ClimateInfluences::default();
        // 冷却期内不转移 / No transition during cooldown.
        let result = climate.try_transition(&influences, 1000 + 1800); // 30min < 1hr cooldown.
        assert_eq!(result, climate.kind);
    }

    #[test]
    fn test_climate_describe() {
        let mut climate = EmotionalClimate::new();
        climate.kind = ClimateKind::Stormy;
        climate.duration_secs = 7200.0; // 2 hours.
        climate.intensity = 0.6;
        let desc = climate.describe();
        assert!(desc.contains("风暴"));
        assert!(desc.contains("Stormy"));
        assert!(desc.contains("120"));
    }

    #[test]
    fn test_climate_periodic_offset() {
        let mut climate = EmotionalClimate::new();
        climate.periodic.circadian_amp = 0.3;
        climate.periodic.circadian_phase = 12.0;
        // 正午偏移应为正 / Positive offset at noon.
        let offset = climate.periodic_offset(12.0, 0.0);
        assert!(offset > 0.0);
    }
}
