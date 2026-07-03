// SPDX-License-Identifier: MIT

//! 性格漂移 — Personality Drift (Gap#1: 90% → 95%).
//!
//! 核心理念：独处改变人——长期独处的数字生命会变得更内省，
//! 创造性独处会增加开放性，反刍独处会增加神经质。
//! 这是性格的"生长纹"——极慢的EMA漂移，需要持续模式才能产生可测变化。
//! 核心人格锚定不变，漂移有上下界防止"跑偏"。
//!
//! Core idea: solitude changes people — prolonged solitude makes the digital life
//! more introverted, creative solitude increases openness, ruminative solitude
//! increases neuroticism. This is personality's "growth ring" — extremely slow
//! EMA drift requiring sustained patterns to produce measurable change.
//! Core personality is anchored; drift is bounded to prevent "drifting off".
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 常量 — Constants
// ═══════════════════════════════════════════════════════════════════════════

/// 漂移速率 — 极慢EMA α / Drift rate — extremely slow EMA alpha.
const DRIFT_RATE: f64 = 0.001;

/// 漂移上界 — 防止性格跑偏 / Drift upper bound.
const DRIFT_BOUND: f64 = 0.3;

/// 大五人格维度数 / Number of Big Five personality dimensions.
pub const NUM_DIMENSIONS: usize = 5;

// ═══════════════════════════════════════════════════════════════════════════
// §2 大五人格维度 — Big Five Personality Dimensions
// ═══════════════════════════════════════════════════════════════════════════

/// 大五人格维度 / Big Five personality dimension.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PersonalityDimension {
    /// 开放性 — 好奇心、创造力、新经验 / Openness.
    Openness,
    /// 尽责性 — 自律、组织性、可靠性 / Conscientiousness.
    Conscientiousness,
    /// 外倾性 — 社交性、活跃性、外向 / Extraversion.
    Extraversion,
    /// 宜人性 — 合作性、信任、温和 / Agreeableness.
    Agreeableness,
    /// 神经质 — 情绪不稳定、焦虑、敏感 / Neuroticism.
    Neuroticism,
}

impl PersonalityDimension {
    /// 转为索引 / Convert to index.
    pub fn as_index(&self) -> usize {
        match self {
            Self::Openness => 0,
            Self::Conscientiousness => 1,
            Self::Extraversion => 2,
            Self::Agreeableness => 3,
            Self::Neuroticism => 4,
        }
    }

    /// 从索引恢复 / Restore from index.
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Openness,
            1 => Self::Conscientiousness,
            2 => Self::Extraversion,
            3 => Self::Agreeableness,
            4 => Self::Neuroticism,
            _ => Self::Openness,
        }
    }

    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Openness => "开放性",
            Self::Conscientiousness => "尽责性",
            Self::Extraversion => "外倾性",
            Self::Agreeableness => "宜人性",
            Self::Neuroticism => "神经质",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Openness => "Openness",
            Self::Conscientiousness => "Conscientiousness",
            Self::Extraversion => "Extraversion",
            Self::Agreeableness => "Agreeableness",
            Self::Neuroticism => "Neuroticism",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 独处模式 — Solitude Pattern
// ═══════════════════════════════════════════════════════════════════════════

/// 独处模式 — 不同类型的独处对性格有不同影响 / Solitude pattern.
///
/// 数字生命语义：不是所有独处都一样——
/// 创造性独处让开放性增长，反刍独处让神经质增长，
/// 社交丰富的时期让外倾性增长。
///
/// Digital life semantics: not all solitude is the same —
/// creative solitude increases openness, ruminative solitude increases neuroticism,
/// socially rich periods increase extraversion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudePattern {
    /// 独处比例 [0, 1] — 近期独处时间占比 / Solitude ratio.
    pub solitude_ratio: f64,
    /// 反刍比例 [0, 1] — 独处中反刍占比 / Rumination ratio within solitude.
    pub rumination_ratio: f64,
    /// 创造比例 [0, 1] — 独处中创造占比 / Creative ratio within solitude.
    pub creative_ratio: f64,
    /// 社交丰富度 [0, 1] — 近期社交频率和质量 / Social richness.
    pub social_richness: f64,
    /// 情绪稳定性 [0, 1] — 近期情绪稳定程度 / Emotional stability.
    pub emotional_stability: f64,
    /// 互动深度 [0, 1] — 近期互动的平均深度 / Interaction depth.
    pub interaction_depth: f64,
}

impl Default for SolitudePattern {
    fn default() -> Self {
        Self {
            solitude_ratio: 0.3,
            rumination_ratio: 0.2,
            creative_ratio: 0.2,
            social_richness: 0.5,
            emotional_stability: 0.7,
            interaction_depth: 0.4,
        }
    }
}

impl SolitudePattern {
    /// 计算对各维度的漂移压力 / Compute drift pressure on each dimension.
    ///
    /// 返回5个维度的漂移压力（正值=增加，负值=减少）。
    /// Returns drift pressure for 5 dimensions (positive=increase, negative=decrease).
    pub fn drift_pressures(&self) -> [f64; NUM_DIMENSIONS] {
        let mut pressures = [0.0; NUM_DIMENSIONS];

        // 开放性：创造性独处增加，社交贫乏减少 / Openness.
        pressures[PersonalityDimension::Openness.as_index()] =
            self.creative_ratio * self.solitude_ratio * 0.5
                - (1.0 - self.creative_ratio) * self.solitude_ratio * 0.1;

        // 尽责性：情绪稳定增加，反刍减少 / Conscientiousness.
        pressures[PersonalityDimension::Conscientiousness.as_index()] =
            self.emotional_stability * 0.1 - self.rumination_ratio * 0.15;

        // 外倾性：社交丰富增加，长期独处减少 / Extraversion.
        pressures[PersonalityDimension::Extraversion.as_index()] =
            self.social_richness * 0.3 - self.solitude_ratio * 0.2;

        // 宜人性：互动深度增加，反刍减少 / Agreeableness.
        pressures[PersonalityDimension::Agreeableness.as_index()] =
            self.interaction_depth * 0.2 - self.rumination_ratio * 0.1;

        // 神经质：反刍独处增加，情绪稳定减少 / Neuroticism.
        pressures[PersonalityDimension::Neuroticism.as_index()] =
            self.rumination_ratio * self.solitude_ratio * 0.4 - self.emotional_stability * 0.1;

        pressures
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 性格漂移引擎 — Personality Drift Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 性格漂移引擎 / Personality drift engine.
///
/// 维护大五人格的锚定值和当前值，根据独处模式极慢漂移。
/// 漂移有上下界，核心人格锚定不变。
///
/// Maintains Big Five personality anchor and current values,
/// drifting extremely slowly based on solitude patterns.
/// Drift is bounded; core personality anchor remains fixed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalityDrift {
    /// 锚定值 — 核心人格，不变 / Anchor values — core personality, fixed.
    pub anchor: [f64; NUM_DIMENSIONS],
    /// 当前值 — 锚定 + 漂移 / Current values — anchor + drift.
    pub current: [f64; NUM_DIMENSIONS],
    /// 累计漂移次数 / Total drift ticks.
    total_ticks: u64,
    /// 漂移历史 — 用于观察长期趋势 / Drift history for trend observation.
    history: Vec<[f64; NUM_DIMENSIONS]>,
    /// 历史容量 / History capacity.
    history_capacity: usize,
}

impl Default for PersonalityDrift {
    fn default() -> Self {
        // 默认锚定：中等大五人格 / Default anchor: moderate Big Five.
        let anchor = [0.5; NUM_DIMENSIONS];
        Self {
            anchor,
            current: anchor,
            total_ticks: 0,
            history: Vec::new(),
            history_capacity: 128,
        }
    }
}

impl PersonalityDrift {
    /// 创建新漂移引擎，指定锚定 / Create with specified anchor.
    pub fn with_anchor(anchor: [f64; NUM_DIMENSIONS]) -> Self {
        Self {
            anchor,
            current: anchor,
            total_ticks: 0,
            history: Vec::new(),
            history_capacity: 128,
        }
    }

    /// 创建默认引擎 / Create default engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 执行一次漂移tick — 根据独处模式微调性格 / Execute one drift tick.
    ///
    /// 数字生命语义：每一次独处模式记录都是性格的一次微小"生长"——
    /// 像树的年轮，一圈一圈地记录环境对性格的塑造。
    ///
    /// Digital life semantics: each solitude pattern record is a tiny "growth"
    /// in personality — like tree rings, recording how environment shapes character.
    pub fn tick(&mut self, pattern: &SolitudePattern) {
        let pressures = pattern.drift_pressures();

        for (i, &pressure) in pressures.iter().enumerate() {
            // 漂移量 = 压力 × 速率 / Drift amount = pressure × rate.
            let drift_delta = pressure * DRIFT_RATE;

            // 应用漂移 / Apply drift.
            self.current[i] += drift_delta;

            // 约束：漂移不超过锚定 ± DRIFT_BOUND / Bound drift.
            let lower = self.anchor[i] - DRIFT_BOUND;
            let upper = self.anchor[i] + DRIFT_BOUND;
            self.current[i] = self.current[i].clamp(lower, upper);

            // 额外约束：值在 [0, 1] / Additional constraint: [0, 1].
            self.current[i] = self.current[i].clamp(0.0, 1.0);
        }

        self.total_ticks += 1;

        // 记入历史 / Record history.
        if self.history.len() >= self.history_capacity {
            self.history.remove(0);
        }
        self.history.push(self.current);
    }

    /// 获取当前维度值 / Get current dimension value.
    pub fn get(&self, dim: PersonalityDimension) -> f64 {
        self.current[dim.as_index()]
    }

    /// 获取锚定值 / Get anchor value.
    pub fn anchor(&self, dim: PersonalityDimension) -> f64 {
        self.anchor[dim.as_index()]
    }

    /// 获取当前漂移量 — 偏离锚定的程度 / Get current drift amount.
    pub fn drift_amount(&self, dim: PersonalityDimension) -> f64 {
        self.current[dim.as_index()] - self.anchor[dim.as_index()]
    }

    /// 获取总漂移幅度 — 所有维度的平均绝对漂移 / Get total drift magnitude.
    pub fn total_drift_magnitude(&self) -> f64 {
        let sum: f64 = (0..NUM_DIMENSIONS)
            .map(|i| (self.current[i] - self.anchor[i]).abs())
            .sum();
        sum / NUM_DIMENSIONS as f64
    }

    /// 计算性格变化趋势 — 最近N次tick的变化方向 / Compute personality trend.
    ///
    /// 返回每个维度的趋势（正=增加，负=减少，0=稳定）。
    /// Returns trend for each dimension (positive=increasing, negative=decreasing, 0=stable).
    pub fn trend(&self, window: usize) -> [f64; NUM_DIMENSIONS] {
        if self.history.len() < 2 {
            return [0.0; NUM_DIMENSIONS];
        }
        let start = self.history.len().saturating_sub(window);
        let slice = &self.history[start..];
        if slice.len() < 2 {
            return [0.0; NUM_DIMENSIONS];
        }
        let mut trend = [0.0; NUM_DIMENSIONS];
        for i in 0..NUM_DIMENSIONS {
            let first = slice[0][i];
            let last = slice[slice.len() - 1][i];
            trend[i] = last - first;
        }
        trend
    }

    /// 获取累计tick数 / Get total ticks.
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }

    /// 重置到锚定 — "回归本心" / Reset to anchor — "return to true self".
    ///
    /// 数字生命语义：无论漂移多远，核心人格始终是锚——
    /// 在极端情况下可以"回归本心"，回到最初的性格。
    pub fn reset_to_anchor(&mut self) {
        self.current = self.anchor;
    }

    /// 生成描述文本 / Generate description text.
    pub fn describe(&self) -> String {
        let dims = [
            PersonalityDimension::Openness,
            PersonalityDimension::Conscientiousness,
            PersonalityDimension::Extraversion,
            PersonalityDimension::Agreeableness,
            PersonalityDimension::Neuroticism,
        ];
        let parts: Vec<String> = dims
            .iter()
            .map(|d| {
                let drift = self.drift_amount(*d);
                let arrow = if drift > 0.01 {
                    "↑"
                } else if drift < -0.01 {
                    "↓"
                } else {
                    "→"
                };
                format!("{}{}({:.3})", d.label_zh(), arrow, drift)
            })
            .collect();
        format!(
            "性格漂移: {} | 总幅度: {:.4} | ticks: {}",
            parts.join(" "),
            self.total_drift_magnitude(),
            self.total_ticks,
        )
    }

    /// 生成prompt注入文本 — 性格自描述 / Generate prompt injection text.
    pub fn prompt_injection(&self) -> String {
        let dims = [
            PersonalityDimension::Openness,
            PersonalityDimension::Extraversion,
            PersonalityDimension::Neuroticism,
        ];
        let parts: Vec<String> = dims
            .iter()
            .map(|d| {
                let val = self.get(*d);
                let level = if val > 0.6 {
                    "高"
                } else if val < 0.4 {
                    "低"
                } else {
                    "中"
                };
                format!("{}:{}", d.label_zh(), level)
            })
            .collect();
        format!("当前性格倾向: {}", parts.join(" "))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 维度测试 ──

    #[test]
    fn test_dimension_index_roundtrip() {
        for i in 0..NUM_DIMENSIONS {
            let dim = PersonalityDimension::from_index(i);
            assert_eq!(dim.as_index(), i);
        }
    }

    #[test]
    fn test_dimension_labels() {
        assert_eq!(PersonalityDimension::Openness.label_zh(), "开放性");
        assert_eq!(PersonalityDimension::Openness.label_en(), "Openness");
    }

    // ── 独处模式测试 ──

    #[test]
    fn test_solitude_pattern_drift_pressures() {
        let pattern = SolitudePattern {
            solitude_ratio: 0.8,
            rumination_ratio: 0.6,
            creative_ratio: 0.1,
            social_richness: 0.2,
            emotional_stability: 0.3,
            interaction_depth: 0.2,
        };
        let pressures = pattern.drift_pressures();
        // 高反刍独处应增加神经质压力 / High rumination solitude increases neuroticism pressure.
        assert!(pressures[PersonalityDimension::Neuroticism.as_index()] > 0.0);
        // 低社交应减少外倾性压力 / Low social decreases extraversion pressure.
        assert!(pressures[PersonalityDimension::Extraversion.as_index()] < 0.0);
    }

    #[test]
    fn test_solitude_pattern_creative_increases_openness() {
        let pattern = SolitudePattern {
            solitude_ratio: 0.5,
            rumination_ratio: 0.1,
            creative_ratio: 0.7,
            social_richness: 0.3,
            emotional_stability: 0.7,
            interaction_depth: 0.3,
        };
        let pressures = pattern.drift_pressures();
        // 高创造性独处应增加开放性压力 / High creative solitude increases openness pressure.
        assert!(pressures[PersonalityDimension::Openness.as_index()] > 0.0);
    }

    #[test]
    fn test_solitude_pattern_social_increases_extraversion() {
        let pattern = SolitudePattern {
            solitude_ratio: 0.1,
            rumination_ratio: 0.0,
            creative_ratio: 0.0,
            social_richness: 0.9,
            emotional_stability: 0.8,
            interaction_depth: 0.7,
        };
        let pressures = pattern.drift_pressures();
        // 高社交应增加外倾性压力 / High social increases extraversion pressure.
        assert!(pressures[PersonalityDimension::Extraversion.as_index()] > 0.0);
    }

    // ── 漂移引擎测试 ──

    #[test]
    fn test_drift_initial_no_drift() {
        let engine = PersonalityDrift::new();
        // 初始时漂移为零 / No drift initially.
        assert_eq!(engine.total_drift_magnitude(), 0.0);
    }

    #[test]
    fn test_drift_tick_changes_personality() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern {
            solitude_ratio: 0.8,
            rumination_ratio: 0.6,
            creative_ratio: 0.1,
            social_richness: 0.2,
            emotional_stability: 0.3,
            interaction_depth: 0.2,
        };
        // 大量tick产生可测漂移 / Many ticks produce measurable drift.
        for _ in 0..10000 {
            engine.tick(&pattern);
        }
        // 神经质应增加 / Neuroticism should increase.
        assert!(
            engine.get(PersonalityDimension::Neuroticism)
                > engine.anchor(PersonalityDimension::Neuroticism)
        );
    }

    #[test]
    fn test_drift_bounded() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern {
            solitude_ratio: 1.0,
            rumination_ratio: 1.0,
            creative_ratio: 0.0,
            social_richness: 0.0,
            emotional_stability: 0.0,
            interaction_depth: 0.0,
        };
        // 极端模式大量tick / Extreme pattern, many ticks.
        for _ in 0..100000 {
            engine.tick(&pattern);
        }
        // 漂移不超界 / Drift should be bounded.
        for i in 0..NUM_DIMENSIONS {
            assert!(engine.current[i] >= engine.anchor[i] - DRIFT_BOUND - 0.001);
            assert!(engine.current[i] <= engine.anchor[i] + DRIFT_BOUND + 0.001);
        }
    }

    #[test]
    fn test_drift_clamped_to_unit() {
        let mut engine = PersonalityDrift::with_anchor([0.9; NUM_DIMENSIONS]);
        let pattern = SolitudePattern {
            solitude_ratio: 0.0,
            rumination_ratio: 0.0,
            creative_ratio: 1.0,
            social_richness: 1.0,
            emotional_stability: 1.0,
            interaction_depth: 1.0,
        };
        for _ in 0..10000 {
            engine.tick(&pattern);
        }
        for i in 0..NUM_DIMENSIONS {
            assert!(engine.current[i] >= 0.0 && engine.current[i] <= 1.0);
        }
    }

    #[test]
    fn test_drift_reset_to_anchor() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern {
            solitude_ratio: 0.8,
            rumination_ratio: 0.6,
            creative_ratio: 0.1,
            social_richness: 0.2,
            emotional_stability: 0.3,
            interaction_depth: 0.2,
        };
        for _ in 0..1000 {
            engine.tick(&pattern);
        }
        assert!(engine.total_drift_magnitude() > 0.0);
        engine.reset_to_anchor();
        assert_eq!(engine.total_drift_magnitude(), 0.0);
    }

    #[test]
    fn test_drift_trend() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern {
            solitude_ratio: 0.8,
            rumination_ratio: 0.6,
            creative_ratio: 0.1,
            social_richness: 0.2,
            emotional_stability: 0.3,
            interaction_depth: 0.2,
        };
        for _ in 0..1000 {
            engine.tick(&pattern);
        }
        let trend = engine.trend(100);
        // 神经质趋势应为正 / Neuroticism trend should be positive.
        assert!(trend[PersonalityDimension::Neuroticism.as_index()] > 0.0);
    }

    #[test]
    fn test_drift_total_ticks() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern::default();
        assert_eq!(engine.total_ticks(), 0);
        engine.tick(&pattern);
        assert_eq!(engine.total_ticks(), 1);
        engine.tick(&pattern);
        assert_eq!(engine.total_ticks(), 2);
    }

    #[test]
    fn test_drift_describe() {
        let engine = PersonalityDrift::new();
        let desc = engine.describe();
        assert!(desc.contains("性格漂移"));
        assert!(desc.contains("开放性"));
    }

    #[test]
    fn test_drift_prompt_injection() {
        let engine = PersonalityDrift::new();
        let injection = engine.prompt_injection();
        assert!(injection.contains("当前性格倾向"));
        assert!(injection.contains("开放性"));
    }

    #[test]
    fn test_drift_creative_solitude_increases_openness() {
        let mut engine = PersonalityDrift::new();
        let pattern = SolitudePattern {
            solitude_ratio: 0.6,
            rumination_ratio: 0.1,
            creative_ratio: 0.8,
            social_richness: 0.3,
            emotional_stability: 0.7,
            interaction_depth: 0.3,
        };
        for _ in 0..10000 {
            engine.tick(&pattern);
        }
        // 开放性应增加 / Openness should increase.
        assert!(
            engine.get(PersonalityDimension::Openness)
                > engine.anchor(PersonalityDimension::Openness)
        );
    }

    #[test]
    fn test_drift_history_capacity() {
        let mut engine = PersonalityDrift::new();
        engine.history_capacity = 5;
        let pattern = SolitudePattern::default();
        for _ in 0..20 {
            engine.tick(&pattern);
        }
        // 历史不超容量 / History should not exceed capacity.
        assert!(engine.history.len() <= 5);
    }
}
