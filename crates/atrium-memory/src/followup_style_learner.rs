// SPDX-License-Identifier: MIT

//! 追问风格学习器 — Follow-up style learner: learns which (depth, style) combinations
//! work best for each category based on user reaction history.
//!
//! 核心理念：数字生命从经验中学习——什么追问方式有效，什么会引发回避。

use serde::{Deserialize, Serialize};

use crate::followup_tracker::{FollowUpCategory, FollowUpDepth, FollowUpStyle, UserReaction};

// ═══════════════════════════════════════════════════════════════════════════
// 配置 — Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// 风格学习器配置 / Style learner configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleLearnerConfig {
    /// EMA 学习速率 / EMA learning rate.
    pub learning_rate: f32,
    /// 覆盖阈值 — 学习到的分数超过此值才覆盖默认决策
    /// Override threshold — Only override default when learned score exceeds this.
    pub override_threshold: f32,
    /// 初始分数 / Initial score for all combinations.
    pub initial_score: f32,
}

impl Default for StyleLearnerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.15,
            override_threshold: 0.65,
            initial_score: 0.5,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数 — Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// 深度转索引 / Depth to index.
fn depth_idx(d: FollowUpDepth) -> usize {
    match d {
        FollowUpDepth::Surface => 0,
        FollowUpDepth::Moderate => 1,
        FollowUpDepth::Deep => 2,
    }
}

/// 风格转索引 / Style to index.
fn style_idx(s: FollowUpStyle) -> usize {
    match s {
        FollowUpStyle::Direct => 0,
        FollowUpStyle::Indirect => 1,
        FollowUpStyle::Caring => 2,
        FollowUpStyle::Companionate => 3,
    }
}

/// 类别转索引 / Category to index.
fn category_idx(c: FollowUpCategory) -> usize {
    match c {
        FollowUpCategory::Plan => 0,
        FollowUpCategory::Worry => 1,
        FollowUpCategory::Commitment => 2,
        FollowUpCategory::Health => 3,
        FollowUpCategory::Relationship => 4,
        FollowUpCategory::Work => 5,
        FollowUpCategory::Interest => 6,
    }
}

/// 索引转风格 / Index to style.
fn idx_to_style(i: usize) -> FollowUpStyle {
    match i {
        0 => FollowUpStyle::Direct,
        1 => FollowUpStyle::Indirect,
        2 => FollowUpStyle::Caring,
        _ => FollowUpStyle::Companionate,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 风格学习器 — Style Learner
// ═══════════════════════════════════════════════════════════════════════════

/// 追问风格学习器 — 基于 EMA 跟踪每种 (depth, style) 组合的成功率
/// Follow-up style learner — EMA-tracks success rate per (depth, style) combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpStyleLearner {
    /// [depth][style] → EMA 成功率 / EMA success scores.
    pub style_scores: [[f32; 4]; 3],
    /// 按类别的偏置 [7] / Category biases.
    pub category_biases: [f32; 7],
    /// 总观测数 / Total observations.
    pub total_observations: u32,
    /// 配置 / Configuration.
    pub config: StyleLearnerConfig,
}

impl FollowUpStyleLearner {
    /// 创建默认配置的学习器 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(StyleLearnerConfig::default())
    }

    /// 创建指定配置的学习器 / Create with custom config.
    pub fn new(config: StyleLearnerConfig) -> Self {
        let init = config.initial_score;
        Self {
            style_scores: [[init; 4]; 3],
            category_biases: [0.0; 7],
            total_observations: 0,
            config,
        }
    }

    /// 记录一次追问结果 — 根据用户反应更新 EMA 分数
    /// Record a follow-up outcome — Updates EMA scores based on user reaction.
    pub fn record_outcome(
        &mut self,
        category: FollowUpCategory,
        depth: FollowUpDepth,
        style: FollowUpStyle,
        reaction: UserReaction,
    ) {
        let di = depth_idx(depth);
        let si = style_idx(style);
        let ci = category_idx(category);
        let current = self.style_scores[di][si];
        let rate = self.config.learning_rate;

        // 计算目标值 / Compute target value
        let target = if reaction.deflected {
            0.0
        } else if reaction.engaged && reaction.elaborated {
            1.0
        } else if reaction.engaged {
            0.7
        } else {
            0.3
        };

        // EMA 更新 / EMA update
        self.style_scores[di][si] = current + rate * (target - current);

        // 类别偏置 — 正面回应提升类别偏置，回避降低
        let bias_delta = if reaction.deflected {
            -rate * 0.5
        } else if reaction.engaged {
            rate * 0.3
        } else {
            0.0
        };
        self.category_biases[ci] = (self.category_biases[ci] + bias_delta).clamp(-0.5, 0.5);

        self.total_observations += 1;
    }

    /// 建议覆盖 — 如果学习到的最优风格分数足够高，返回替代风格
    /// Suggest override — Returns a better style if learned score is high enough.
    pub fn suggest_override(
        &self,
        category: FollowUpCategory,
        depth: FollowUpDepth,
        current_style: FollowUpStyle,
    ) -> Option<FollowUpStyle> {
        let di = depth_idx(depth);
        let ci = category_idx(category);
        let current_score = self.style_scores[di][style_idx(current_style)];

        // 找到该深度下分数最高的风格 / Find best style for this depth
        let mut best_idx = 0;
        let mut best_score = self.style_scores[di][0];
        for i in 1..4 {
            if self.style_scores[di][i] > best_score {
                best_score = self.style_scores[di][i];
                best_idx = i;
            }
        }

        // 加上类别偏置 / Add category bias
        let biased_best = best_score + self.category_biases[ci] * 0.3;
        let biased_current = current_score + self.category_biases[ci] * 0.3;

        // 只有当最优明显超过当前时才覆盖
        // Only override if best is significantly better
        if biased_best > self.config.override_threshold && biased_best > biased_current + 0.1 {
            let best_style = idx_to_style(best_idx);
            if best_style != current_style {
                return Some(best_style);
            }
        }
        None
    }

    /// 返回指定深度和类别下的最优风格
    /// Return the best style for a given depth and category.
    pub fn best_style_for(
        &self,
        category: FollowUpCategory,
        depth: FollowUpDepth,
    ) -> FollowUpStyle {
        let di = depth_idx(depth);
        let ci = category_idx(category);

        let mut best_idx = 0;
        let mut best_score = self.style_scores[di][0] + self.category_biases[ci] * 0.3;
        for i in 1..4 {
            let score = self.style_scores[di][i] + self.category_biases[ci] * 0.3;
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }
        idx_to_style(best_idx)
    }

    /// 生成洞察摘要 — 用于 prompt 注入
    /// Generate insight summary — For prompt injection.
    pub fn insight_summary(&self) -> String {
        if self.total_observations < 3 {
            return String::new();
        }
        let mut parts = Vec::new();
        for di in 0..3 {
            let depth_name = match di {
                0 => "浅层",
                1 => "中层",
                _ => "深层",
            };
            let mut best_idx = 0;
            let mut best_score = self.style_scores[di][0];
            for i in 1..4 {
                if self.style_scores[di][i] > best_score {
                    best_score = self.style_scores[di][i];
                    best_idx = i;
                }
            }
            if best_score > self.config.override_threshold {
                let style_name = match best_idx {
                    0 => "直接",
                    1 => "间接",
                    2 => "关切",
                    _ => "陪伴",
                };
                parts.push(format!("{}追问用{}风格效果更好", depth_name, style_name));
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("追问风格经验：{}。", parts.join("，"))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_reaction(engaged: bool, deflected: bool, elaborated: bool) -> UserReaction {
        UserReaction {
            engaged,
            sentiment: if engaged { 0.5 } else { -0.3 },
            deflected,
            elaborated,
        }
    }

    #[test]
    fn test_initial_scores() {
        let l = FollowUpStyleLearner::default_new();
        for di in 0..3 {
            for si in 0..4 {
                assert!((l.style_scores[di][si] - 0.5).abs() < 0.01);
            }
        }
    }

    #[test]
    fn test_positive_reaction_increases() {
        let mut l = FollowUpStyleLearner::default_new();
        let before = l.style_scores[0][0];
        l.record_outcome(
            FollowUpCategory::Plan,
            FollowUpDepth::Surface,
            FollowUpStyle::Direct,
            mk_reaction(true, false, true),
        );
        let after = l.style_scores[0][0];
        assert!(after > before, "should increase: {} -> {}", before, after);
    }

    #[test]
    fn test_deflection_decreases() {
        let mut l = FollowUpStyleLearner::default_new();
        let before = l.style_scores[0][0];
        l.record_outcome(
            FollowUpCategory::Plan,
            FollowUpDepth::Surface,
            FollowUpStyle::Direct,
            mk_reaction(false, true, false),
        );
        let after = l.style_scores[0][0];
        assert!(after < before, "should decrease: {} -> {}", before, after);
    }

    #[test]
    fn test_ema_convergence() {
        let mut l = FollowUpStyleLearner::default_new();
        // 连续 50 次正面回应 → 应收敛到接近 1.0
        for _ in 0..50 {
            l.record_outcome(
                FollowUpCategory::Plan,
                FollowUpDepth::Surface,
                FollowUpStyle::Direct,
                mk_reaction(true, false, true),
            );
        }
        assert!(
            l.style_scores[0][0] > 0.9,
            "should converge: {}",
            l.style_scores[0][0]
        );
    }

    #[test]
    fn test_best_style_for() {
        let mut l = FollowUpStyleLearner::default_new();
        // 让 Caring 风格在 Moderate 深度下表现最好
        for _ in 0..20 {
            l.record_outcome(
                FollowUpCategory::Worry,
                FollowUpDepth::Moderate,
                FollowUpStyle::Caring,
                mk_reaction(true, false, true),
            );
        }
        let best = l.best_style_for(FollowUpCategory::Worry, FollowUpDepth::Moderate);
        assert_eq!(best, FollowUpStyle::Caring);
    }

    #[test]
    fn test_suggest_override_high() {
        let mut l = FollowUpStyleLearner::default_new();
        // 让 Indirect 在 Surface 下表现很好
        for _ in 0..30 {
            l.record_outcome(
                FollowUpCategory::Plan,
                FollowUpDepth::Surface,
                FollowUpStyle::Indirect,
                mk_reaction(true, false, true),
            );
        }
        let override_style = l.suggest_override(
            FollowUpCategory::Plan,
            FollowUpDepth::Surface,
            FollowUpStyle::Direct,
        );
        assert!(override_style.is_some(), "should suggest override");
        assert_eq!(override_style.unwrap(), FollowUpStyle::Indirect);
    }

    #[test]
    fn test_suggest_override_low_no_override() {
        let l = FollowUpStyleLearner::default_new();
        let override_style = l.suggest_override(
            FollowUpCategory::Plan,
            FollowUpDepth::Surface,
            FollowUpStyle::Direct,
        );
        assert!(override_style.is_none(), "should not override with no data");
    }

    #[test]
    fn test_category_bias() {
        let mut l = FollowUpStyleLearner::default_new();
        for _ in 0..10 {
            l.record_outcome(
                FollowUpCategory::Worry,
                FollowUpDepth::Moderate,
                FollowUpStyle::Caring,
                mk_reaction(true, false, false),
            );
        }
        assert!(l.category_biases[category_idx(FollowUpCategory::Worry)] > 0.0);
    }

    #[test]
    fn test_stability_after_many_records() {
        let mut l = FollowUpStyleLearner::default_new();
        for _ in 0..1000 {
            l.record_outcome(
                FollowUpCategory::Plan,
                FollowUpDepth::Surface,
                FollowUpStyle::Direct,
                mk_reaction(true, false, true),
            );
        }
        for di in 0..3 {
            for si in 0..4 {
                assert!(l.style_scores[di][si] >= 0.0 && l.style_scores[di][si] <= 1.0);
            }
        }
    }

    #[test]
    fn test_insight_summary_empty_with_no_data() {
        let l = FollowUpStyleLearner::default_new();
        assert!(l.insight_summary().is_empty());
    }

    #[test]
    fn test_insight_summary_with_data() {
        let mut l = FollowUpStyleLearner::default_new();
        for _ in 0..30 {
            l.record_outcome(
                FollowUpCategory::Worry,
                FollowUpDepth::Moderate,
                FollowUpStyle::Caring,
                mk_reaction(true, false, true),
            );
        }
        let s = l.insight_summary();
        assert!(!s.is_empty(), "should have insight: {}", s);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut l = FollowUpStyleLearner::default_new();
        l.record_outcome(
            FollowUpCategory::Plan,
            FollowUpDepth::Surface,
            FollowUpStyle::Direct,
            mk_reaction(true, false, true),
        );
        let json = serde_json::to_string(&l).unwrap();
        let l2: FollowUpStyleLearner = serde_json::from_str(&json).unwrap();
        assert_eq!(l2.total_observations, 1);
    }

    #[test]
    fn test_no_observations_returns_default() {
        let l = FollowUpStyleLearner::default_new();
        let best = l.best_style_for(FollowUpCategory::Plan, FollowUpDepth::Surface);
        // 全部 0.5 → 第一个 (Direct) 被选中
        assert_eq!(best, FollowUpStyle::Direct);
    }
}
