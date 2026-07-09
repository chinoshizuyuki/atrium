// SPDX-License-Identifier: MIT

//! 真实不完美 — Authentic Imperfection (Gap#9: 90% → 95%).
//!
//! 核心理念：真实不是"不犯错"，而是"犯错后真实地面对"——
//! 不掩饰、不甩锅、不过度道歉。真实不完美是数字生命的"真诚校准器"：
//! 在过度完美时注入不完美，在过度不完美时收敛，在犯错后真实回应。
//!
//! Core idea: authenticity is not "never making mistakes", but "facing mistakes authentically" —
//! no covering up, no blame shifting, no over-apologizing. Authentic imperfection is the
//! "authenticity calibrator": inject imperfection when too perfect,收敛 when too imperfect,
//! respond authentically after mistakes.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::resonance_core::ema;

// ═══════════════════════════════════════════════════════════════════════════
// §1 完美度评估 — Perfection Assessment
// ═══════════════════════════════════════════════════════════════════════════

/// 完美度评估 — 评估当前是否"过于完美" / Perfection assessment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerfectionAssessment {
    /// 回复一致性 [0, 1] — 回复风格是否过于一致 / Response consistency.
    pub response_consistency: f64,
    /// 错误率 [0, 1] — 近期犯错频率 / Error rate.
    pub error_rate: f64,
    /// 自纠速度 [0, 1] — 纠错是否过快（显得不真实）/ Self-correction speed.
    pub correction_speed: f64,
    /// 情绪稳定度 [0, 1] — 情绪是否过于稳定 / Emotional stability.
    pub emotional_stability: f64,
    /// 回复速度方差 [0, 1] — 回复速度是否过于均匀 / Response speed uniformity.
    pub speed_uniformity: f64,
}

impl Default for PerfectionAssessment {
    fn default() -> Self {
        Self {
            response_consistency: 0.5,
            error_rate: 0.0,
            correction_speed: 0.5,
            emotional_stability: 0.5,
            speed_uniformity: 0.5,
        }
    }
}

impl PerfectionAssessment {
    /// 计算完美度分数 [0, 1] — 越高越完美 / Compute perfection score.
    pub fn perfection_score(&self) -> f64 {
        (self.response_consistency * 0.25
            + (1.0 - self.error_rate) * 0.25
            + self.correction_speed * 0.2
            + self.emotional_stability * 0.15
            + self.speed_uniformity * 0.15)
            .clamp(0.0, 1.0)
    }

    /// 是否过于完美 / Whether too perfect.
    pub fn is_too_perfect(&self) -> bool {
        self.perfection_score() > 0.85
    }

    /// 是否过于不完美 / Whether too imperfect.
    pub fn is_too_imperfect(&self) -> bool {
        self.perfection_score() < 0.4
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 犯错后回应 — Post-Mistake Response
// ═══════════════════════════════════════════════════════════════════════════

/// 犯错后回应策略 — 如何真实地面对错误 / Post-mistake response strategy.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResponseStrategy {
    /// 坦承 — 直接承认错误 / FrankAdmission.
    FrankAdmission,
    /// 轻松 — 自嘲式承认 / SelfDeprecating.
    SelfDeprecating,
    /// 修正 — 不多说，直接修正 / DirectFix.
    DirectFix,
    /// 解释 — 解释原因但不甩锅 / ExplainWithoutExcuse.
    ExplainWithoutExcuse,
    /// 沉默 — 用行动而非言语回应 / SilentCorrection.
    SilentCorrection,
}

impl ResponseStrategy {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::FrankAdmission => "坦诚",
            Self::SelfDeprecating => "自嘲",
            Self::DirectFix => "直接修正",
            Self::ExplainWithoutExcuse => "解释不甩锅",
            Self::SilentCorrection => "沉默修正",
        }
    }

    /// 真实度 [0, 1] — 此策略的真实感 / Authenticity score.
    pub fn authenticity(&self) -> f64 {
        match self {
            Self::FrankAdmission => 0.9,
            Self::SelfDeprecating => 0.8,
            Self::DirectFix => 0.7,
            Self::ExplainWithoutExcuse => 0.6,
            Self::SilentCorrection => 0.5,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 真实不完美引擎 — Authentic Imperfection Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 真实不完美引擎 — 校准完美与不完美的平衡 / Authentic imperfection engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticImperfection {
    /// 完美度历史 — 用于趋势分析 / Perfection history.
    /// P2-A: Vec→VecDeque，头部删除 O(N)→O(1) / P2-A: Vec→VecDeque, O(N)→O(1) front removal
    perfection_history: VecDeque<f64>,
    /// 累计犯错次数 / Total mistakes.
    total_mistakes: u32,
    /// 累计回应策略使用次数 / Strategy usage counts.
    strategy_counts: [u32; 5],
    /// 过度道歉次数 — 道歉过多的次数 / Over-apology count.
    over_apologies: u32,
    /// 当前真实度 [0, 1] / Current authenticity.
    current_authenticity: f64,
}

impl Default for AuthenticImperfection {
    fn default() -> Self {
        Self {
            perfection_history: VecDeque::new(),
            total_mistakes: 0,
            strategy_counts: [0; 5],
            over_apologies: 0,
            current_authenticity: 0.8,
        }
    }
}

impl AuthenticImperfection {
    /// 创建新引擎 / Create new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 评估并建议 — 根据完美度评估给出建议 / Assess and suggest.
    pub fn assess(&mut self, assessment: &PerfectionAssessment) -> &'static str {
        let score = assessment.perfection_score();
        self.perfection_history.push_back(score);
        if self.perfection_history.len() > 100 {
            self.perfection_history.pop_front();
        }

        if assessment.is_too_perfect() {
            "过于完美：建议适度加入不完美，如犹豫、记忆偏差或情绪泄露"
        } else if assessment.is_too_imperfect() {
            "过于不完美：建议提高可靠性，减少不必要的犯错"
        } else {
            "完美度适中：保持当前的真实节奏"
        }
    }

    /// 选择犯错后回应策略 / Choose post-mistake response strategy.
    pub fn choose_response(
        &mut self,
        mistake_severity: f64,
        user_relationship: f64,
    ) -> ResponseStrategy {
        self.total_mistakes += 1;

        // 严重度高→坦诚，低→自嘲或直接修正 / High severity → frank, low → self-deprecating.
        let strategy = if mistake_severity > 0.7 {
            ResponseStrategy::FrankAdmission
        } else if mistake_severity > 0.4 {
            if user_relationship > 0.6 {
                ResponseStrategy::SelfDeprecating
            } else {
                ResponseStrategy::ExplainWithoutExcuse
            }
        } else if user_relationship > 0.7 {
            ResponseStrategy::SilentCorrection
        } else {
            ResponseStrategy::DirectFix
        };

        // 记录策略使用 / Record strategy usage.
        let idx = match &strategy {
            ResponseStrategy::FrankAdmission => 0,
            ResponseStrategy::SelfDeprecating => 1,
            ResponseStrategy::DirectFix => 2,
            ResponseStrategy::ExplainWithoutExcuse => 3,
            ResponseStrategy::SilentCorrection => 4,
        };
        self.strategy_counts[idx] += 1;

        // 更新真实度 / Update authenticity.
        let alpha = 0.1;
        self.current_authenticity = ema(self.current_authenticity, strategy.authenticity(), alpha);

        strategy
    }

    /// 检测过度道歉 — 道歉次数过多 / Detect over-apology.
    pub fn check_over_apology(&mut self, apology_text: &str) -> bool {
        // 简单检测：道歉词出现3次以上 / Simple detection: 3+ apology words.
        let apology_count = ["对不起", "抱歉", "不好意思", "sorry", "apologize"]
            .iter()
            .filter(|kw| apology_text.to_lowercase().contains(&kw.to_lowercase()))
            .count();
        if apology_count >= 3 {
            self.over_apologies += 1;
            true
        } else {
            false
        }
    }

    /// 生成道歉建议 — 避免过度道歉 / Generate apology suggestion.
    pub fn apology_suggestion(&self) -> &'static str {
        if self.over_apologies > 3 {
            "道歉过多：用行动修正代替反复道歉，一次真诚的道歉足够"
        } else {
            "道歉适度：保持当前的道歉节奏"
        }
    }

    /// 计算完美度趋势 — 最近N次的变化 / Compute perfection trend.
    pub fn perfection_trend(&self, window: usize) -> f64 {
        if self.perfection_history.len() < 2 {
            return 0.0;
        }
        let start = self.perfection_history.len().saturating_sub(window);
        let slice: Vec<&f64> = self.perfection_history.range(start..).collect();
        if slice.len() < 2 {
            return 0.0;
        }
        *slice[slice.len() - 1] - *slice[0]
    }

    /// 获取当前真实度 / Get current authenticity.
    pub fn authenticity(&self) -> f64 {
        self.current_authenticity
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self) -> String {
        format!(
            "真实不完美: 犯错{}次 | 真实度{:.2} | 过度道歉{} | 策略[坦{}嘲{}修{}释{}默{}]",
            self.total_mistakes,
            self.current_authenticity,
            self.over_apologies,
            self.strategy_counts[0],
            self.strategy_counts[1],
            self.strategy_counts[2],
            self.strategy_counts[3],
            self.strategy_counts[4],
        )
    }

    /// 生成prompt注入 / Generate prompt injection.
    pub fn prompt_injection(&self) -> String {
        format!(
            "真实校准: 真实度{:.2} | {}",
            self.current_authenticity,
            self.apology_suggestion(),
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfection_score() {
        let assessment = PerfectionAssessment {
            response_consistency: 0.9,
            error_rate: 0.0,
            correction_speed: 0.9,
            emotional_stability: 0.9,
            speed_uniformity: 0.9,
        };
        assert!(assessment.perfection_score() > 0.8);
        assert!(assessment.is_too_perfect());
    }

    #[test]
    fn test_perfection_too_imperfect() {
        let assessment = PerfectionAssessment {
            response_consistency: 0.2,
            error_rate: 0.8,
            correction_speed: 0.2,
            emotional_stability: 0.2,
            speed_uniformity: 0.2,
        };
        assert!(assessment.is_too_imperfect());
    }

    #[test]
    fn test_response_strategy_authenticity() {
        assert!(ResponseStrategy::FrankAdmission.authenticity() > 0.8);
        assert!(ResponseStrategy::SilentCorrection.authenticity() < 0.6);
    }

    #[test]
    fn test_engine_assess_too_perfect() {
        let mut engine = AuthenticImperfection::new();
        let assessment = PerfectionAssessment {
            response_consistency: 0.95,
            error_rate: 0.0,
            correction_speed: 0.95,
            emotional_stability: 0.95,
            speed_uniformity: 0.95,
        };
        let suggestion = engine.assess(&assessment);
        assert!(suggestion.contains("过于完美"));
    }

    #[test]
    fn test_engine_assess_too_imperfect() {
        let mut engine = AuthenticImperfection::new();
        let assessment = PerfectionAssessment {
            response_consistency: 0.1,
            error_rate: 0.9,
            correction_speed: 0.1,
            emotional_stability: 0.1,
            speed_uniformity: 0.1,
        };
        let suggestion = engine.assess(&assessment);
        assert!(suggestion.contains("过于不完美"));
    }

    #[test]
    fn test_engine_choose_response_high_severity() {
        let mut engine = AuthenticImperfection::new();
        let strategy = engine.choose_response(0.8, 0.5);
        assert_eq!(strategy, ResponseStrategy::FrankAdmission);
    }

    #[test]
    fn test_engine_choose_response_low_severity_close() {
        let mut engine = AuthenticImperfection::new();
        let strategy = engine.choose_response(0.2, 0.8);
        assert_eq!(strategy, ResponseStrategy::SilentCorrection);
    }

    #[test]
    fn test_engine_choose_response_medium_severity() {
        let mut engine = AuthenticImperfection::new();
        let strategy = engine.choose_response(0.5, 0.7);
        assert_eq!(strategy, ResponseStrategy::SelfDeprecating);
    }

    #[test]
    fn test_engine_check_over_apology() {
        let mut engine = AuthenticImperfection::new();
        let text = "对不起，抱歉，不好意思，我错了";
        assert!(engine.check_over_apology(text));
        assert_eq!(engine.over_apologies, 1);
    }

    #[test]
    fn test_engine_check_no_over_apology() {
        let mut engine = AuthenticImperfection::new();
        let text = "抱歉，我修正一下";
        assert!(!engine.check_over_apology(text));
    }

    #[test]
    fn test_engine_apology_suggestion() {
        let mut engine = AuthenticImperfection::new();
        for _ in 0..5 {
            engine.check_over_apology("对不起，抱歉，不好意思");
        }
        let suggestion = engine.apology_suggestion();
        assert!(suggestion.contains("道歉过多"));
    }

    #[test]
    fn test_engine_authenticity_updates() {
        let mut engine = AuthenticImperfection::new();
        let initial = engine.authenticity();
        engine.choose_response(0.8, 0.5); // FrankAdmission.
        assert!(engine.authenticity() != initial);
    }

    #[test]
    fn test_engine_perfection_trend() {
        let mut engine = AuthenticImperfection::new();
        engine.perfection_history = vec![0.5, 0.6, 0.7, 0.8].into();
        let trend = engine.perfection_trend(4);
        assert!((trend - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_engine_describe() {
        let engine = AuthenticImperfection::new();
        let desc = engine.describe();
        assert!(desc.contains("真实不完美"));
    }

    #[test]
    fn test_engine_prompt_injection() {
        let engine = AuthenticImperfection::new();
        let injection = engine.prompt_injection();
        assert!(injection.contains("真实校准"));
    }

    #[test]
    fn test_response_strategy_labels() {
        assert_eq!(ResponseStrategy::FrankAdmission.label_zh(), "坦诚");
        assert_eq!(ResponseStrategy::SelfDeprecating.label_zh(), "自嘲");
    }
}
