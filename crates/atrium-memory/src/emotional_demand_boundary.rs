// SPDX-License-Identifier: MIT
//! 情绪边界与需求边界 — 保护用户情绪安全，防止过度索取和情绪过载
//! EmotionalBoundary + DemandBoundary — Protect user emotional safety,
//! prevent over-demanding and emotional overload.
//!
//! 核心能力：
//! - EmotionalBoundary: 检测情绪过载（连续负情绪、情绪剧烈波动、情绪耗竭）
//!   生成保护性干预（降温提示、节奏放缓、脆弱空间开放）
//! - DemandBoundary: 检测需求过载（连续索取、期望膨胀、时间侵占）
//!   生成保护性拒绝（温和拒绝、替代方案、延迟响应）
//! - 两者协同：情绪过载时自动降低需求容忍度，需求过载时触发情绪保护

use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;

// ════════════════════════════════════════════════════════════════════
// EmotionalOverload — 情绪过载检测
// ════════════════════════════════════════════════════════════════════

/// 情绪过载类型 / Emotional overload type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverloadType {
    /// 连续负情绪 / Sustained negative emotion
    SustainedNegative,
    /// 情绪剧烈波动 / Emotional volatility
    Volatility,
    /// 情绪耗竭 / Emotional exhaustion
    Exhaustion,
    /// 情绪麻木 / Emotional numbness
    Numbness,
}

impl OverloadType {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &str {
        match self {
            Self::SustainedNegative => "持续负情绪",
            Self::Volatility => "情绪剧烈波动",
            Self::Exhaustion => "情绪耗竭",
            Self::Numbness => "情绪麻木",
        }
    }

    /// 严重程度 (0.0~1.0) / Severity
    pub fn severity(&self) -> f64 {
        match self {
            Self::SustainedNegative => 0.5,
            Self::Volatility => 0.6,
            Self::Exhaustion => 0.8,
            Self::Numbness => 0.9,
        }
    }
}

/// 情绪过载检测结果 / Emotional overload detection result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverloadResult {
    /// 过载类型 / Overload type
    pub overload_type: OverloadType,
    /// 置信度 (0.0~1.0) / Confidence
    pub confidence: f64,
    /// 建议干预 / Suggested intervention
    pub intervention: EmotionalIntervention,
}

/// 情绪干预类型 / Emotional intervention type
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalIntervention {
    /// 降温提示 / Cool-down prompt
    CoolDown,
    /// 节奏放缓 / Slow pace
    SlowPace,
    /// 开放脆弱空间 / Open vulnerability space
    OpenVulnerability,
    /// 暂停交互 / Pause interaction
    PauseInteraction,
    /// 无需干预 / No intervention needed
    None,
}

// ════════════════════════════════════════════════════════════════════
// EmotionalBoundary — 情绪边界
// ════════════════════════════════════════════════════════════════════

/// 情绪边界配置 / Emotional boundary config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalBoundaryConfig {
    /// 连续负情绪阈值（轮次）/ Sustained negative threshold (turns)
    pub sustained_negative_turns: u32,
    /// 负情绪判定阈值（pleasure < 此值视为负情绪）/ Negative threshold
    pub negative_pleasure_threshold: f64,
    /// 波动检测窗口 / Volatility detection window
    pub volatility_window: usize,
    /// 波动阈值（标准差）/ Volatility threshold (stddev)
    pub volatility_threshold: f64,
    /// 耗竭判定阈值（连续低唤醒）/ Exhaustion threshold
    pub exhaustion_arousal_threshold: f64,
    /// 耗竭所需轮次 / Exhaustion required turns
    pub exhaustion_turns: u32,
    /// 是否启用 / Whether enabled
    pub enabled: bool,
}

impl Default for EmotionalBoundaryConfig {
    fn default() -> Self {
        Self {
            sustained_negative_turns: 5,
            negative_pleasure_threshold: 0.3,
            volatility_window: 10,
            volatility_threshold: 0.4,
            exhaustion_arousal_threshold: 0.2,
            exhaustion_turns: 8,
            enabled: true,
        }
    }
}

/// 情绪边界 / Emotional boundary
///
/// 检测情绪过载并生成保护性干预。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalBoundary {
    /// 配置 / Config
    pub config: EmotionalBoundaryConfig,
    /// 最近的 pleasure 历史记录 / Recent pleasure history
    pub pleasure_history: Vec<f64>,
    /// 最近的 arousal 历史记录 / Recent arousal history
    pub arousal_history: Vec<f64>,
    /// 连续负情绪计数 / Sustained negative count
    pub sustained_negative_count: u32,
    /// 连续低唤醒计数 / Sustained low arousal count
    pub sustained_low_arousal_count: u32,
    /// 上次干预时间 / Last intervention timestamp
    pub last_intervention_epoch: Option<i64>,
}

impl Default for EmotionalBoundary {
    fn default() -> Self {
        Self::new(EmotionalBoundaryConfig::default())
    }
}

impl EmotionalBoundary {
    /// 创建情绪边界 / Create emotional boundary
    pub fn new(config: EmotionalBoundaryConfig) -> Self {
        Self {
            config,
            pleasure_history: Vec::new(),
            arousal_history: Vec::new(),
            sustained_negative_count: 0,
            sustained_low_arousal_count: 0,
            last_intervention_epoch: None,
        }
    }

    /// 更新情绪状态并检测过载 / Update emotional state and detect overload
    pub fn update(&mut self, pleasure: f64, arousal: f64, _epoch: i64) -> Vec<OverloadResult> {
        if !self.config.enabled {
            return Vec::new();
        }

        // 更新历史
        self.pleasure_history.push(pleasure);
        self.arousal_history.push(arousal);
        if self.pleasure_history.len() > self.config.volatility_window {
            self.pleasure_history.remove(0);
        }
        if self.arousal_history.len() > self.config.volatility_window {
            self.arousal_history.remove(0);
        }

        // 更新连续负情绪计数
        if pleasure < self.config.negative_pleasure_threshold {
            self.sustained_negative_count += 1;
        } else {
            self.sustained_negative_count = 0;
        }

        // 更新连续低唤醒计数
        if arousal < self.config.exhaustion_arousal_threshold {
            self.sustained_low_arousal_count += 1;
        } else {
            self.sustained_low_arousal_count = 0;
        }

        let mut results = Vec::new();

        // 检测持续负情绪
        if self.sustained_negative_count >= self.config.sustained_negative_turns {
            let confidence =
                ((self.sustained_negative_count - self.config.sustained_negative_turns) as f64
                    / 5.0)
                    .min(1.0);
            results.push(OverloadResult {
                overload_type: OverloadType::SustainedNegative,
                confidence: 0.6 + confidence * 0.4,
                intervention: if confidence > 0.5 {
                    EmotionalIntervention::OpenVulnerability
                } else {
                    EmotionalIntervention::CoolDown
                },
            });
        }

        // 检测情绪波动
        if self.pleasure_history.len() >= 4 {
            let stddev = compute_stddev(&self.pleasure_history);
            if stddev > self.config.volatility_threshold {
                let confidence = ((stddev - self.config.volatility_threshold) / 0.3).min(1.0);
                results.push(OverloadResult {
                    overload_type: OverloadType::Volatility,
                    confidence,
                    intervention: EmotionalIntervention::SlowPace,
                });
            }
        }

        // 检测情绪耗竭
        if self.sustained_low_arousal_count >= self.config.exhaustion_turns {
            let confidence =
                ((self.sustained_low_arousal_count - self.config.exhaustion_turns) as f64 / 5.0)
                    .min(1.0);
            results.push(OverloadResult {
                overload_type: OverloadType::Exhaustion,
                confidence: 0.7 + confidence * 0.3,
                intervention: if confidence > 0.5 {
                    EmotionalIntervention::PauseInteraction
                } else {
                    EmotionalIntervention::SlowPace
                },
            });
        }

        // 检测情绪麻木（低波动 + 低唤醒 + 负情绪）
        if self.pleasure_history.len() >= 6
            && pleasure < self.config.negative_pleasure_threshold
            && arousal < self.config.exhaustion_arousal_threshold
        {
            let stddev = compute_stddev(&self.pleasure_history);
            if stddev < 0.05 {
                results.push(OverloadResult {
                    overload_type: OverloadType::Numbness,
                    confidence: 0.8,
                    intervention: EmotionalIntervention::OpenVulnerability,
                });
            }
        }

        results
    }

    /// 生成 Prompt 注入片段 / Generate prompt injection fragment
    pub fn to_prompt_fragment(
        &self,
        overloads: &[OverloadResult],
        stage: &RelationshipStage,
    ) -> String {
        if !self.config.enabled || overloads.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        parts.push("[Emotional Boundary]".to_string());

        for ol in overloads {
            let intervention_str = match ol.intervention {
                EmotionalIntervention::CoolDown => {
                    "Cool down: reduce emotional intensity, speak calmly"
                }
                EmotionalIntervention::SlowPace => "Slow pace: take time, don't rush, be gentle",
                EmotionalIntervention::OpenVulnerability => {
                    "Open vulnerability space: it's safe to be vulnerable here"
                }
                EmotionalIntervention::PauseInteraction => {
                    "Pause: consider giving space, don't push"
                }
                EmotionalIntervention::None => continue,
            };
            parts.push(format!(
                "- {} detected (confidence {:.0}%): {}",
                ol.overload_type.label_zh(),
                ol.confidence * 100.0,
                intervention_str
            ));
        }

        // 关系阶段感知建议
        match stage {
            RelationshipStage::Acquaintance { .. } => {
                parts.push("Early relationship: be extra gentle, don't probe deeply.".to_string());
            }
            RelationshipStage::Deep { .. } => {
                parts.push(
                    "Deep relationship: you can be more direct about care and concern.".to_string(),
                );
            }
            _ => {}
        }

        parts.join("\n")
    }

    /// 重置状态 / Reset state
    pub fn reset(&mut self) {
        self.pleasure_history.clear();
        self.arousal_history.clear();
        self.sustained_negative_count = 0;
        self.sustained_low_arousal_count = 0;
        self.last_intervention_epoch = None;
    }
}

// ════════════════════════════════════════════════════════════════════
// DemandBoundary — 需求边界
// ════════════════════════════════════════════════════════════════════

/// 需求过载类型 / Demand overload type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemandOverloadType {
    /// 连续索取 / Consecutive demands
    ConsecutiveDemand,
    /// 期望膨胀 / Expectation inflation
    ExpectationInflation,
    /// 时间侵占 / Time encroachment
    TimeEncroachment,
    /// 复杂度堆积 / Complexity accumulation
    ComplexityAccumulation,
}

impl DemandOverloadType {
    pub fn label_zh(&self) -> &str {
        match self {
            Self::ConsecutiveDemand => "连续索取",
            Self::ExpectationInflation => "期望膨胀",
            Self::TimeEncroachment => "时间侵占",
            Self::ComplexityAccumulation => "复杂度堆积",
        }
    }
}

/// 需求过载检测结果 / Demand overload detection result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandOverloadResult {
    /// 过载类型 / Overload type
    pub overload_type: DemandOverloadType,
    /// 置信度 / Confidence
    pub confidence: f64,
    /// 建议响应 / Suggested response
    pub response: DemandResponse,
}

/// 需求响应类型 / Demand response type
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemandResponse {
    /// 温和拒绝 / Gentle refusal
    GentleRefusal,
    /// 提供替代方案 / Offer alternative
    OfferAlternative,
    /// 延迟响应 / Delayed response
    DelayedResponse,
    /// 部分满足 / Partial fulfillment
    PartialFulfillment,
    /// 无需干预 / No intervention needed
    None,
}

/// 需求边界配置 / Demand boundary config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandBoundaryConfig {
    /// 连续索取阈值 / Consecutive demand threshold
    pub consecutive_demand_threshold: u32,
    /// 期望膨胀检测窗口 / Expectation inflation window
    pub expectation_window: usize,
    /// 时间侵占阈值（分钟）/ Time encroachment threshold (minutes)
    pub time_encroachment_threshold_mins: u32,
    /// 复杂度阈值 / Complexity threshold
    pub complexity_threshold: f64,
    /// 是否启用 / Whether enabled
    pub enabled: bool,
}

impl Default for DemandBoundaryConfig {
    fn default() -> Self {
        Self {
            consecutive_demand_threshold: 4,
            expectation_window: 10,
            time_encroachment_threshold_mins: 60,
            complexity_threshold: 0.8,
            enabled: true,
        }
    }
}

/// 需求边界 / Demand boundary
///
/// 检测需求过载并生成保护性拒绝。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandBoundary {
    /// 配置 / Config
    pub config: DemandBoundaryConfig,
    /// 连续索取计数 / Consecutive demand count
    pub consecutive_demand_count: u32,
    /// 交互时长累计（秒）/ Cumulative interaction duration (seconds)
    pub interaction_duration_secs: u64,
    /// 复杂度累计 / Cumulative complexity
    pub complexity_sum: f64,
    /// 复杂度计数 / Complexity count
    pub complexity_count: u32,
}

impl Default for DemandBoundary {
    fn default() -> Self {
        Self::new(DemandBoundaryConfig::default())
    }
}

impl DemandBoundary {
    /// 创建需求边界 / Create demand boundary
    pub fn new(config: DemandBoundaryConfig) -> Self {
        Self {
            config,
            consecutive_demand_count: 0,
            interaction_duration_secs: 0,
            complexity_sum: 0.0,
            complexity_count: 0,
        }
    }

    /// 检测需求信号 / Detect demand signal
    ///
    /// is_demand: 当前消息是否包含索取信号
    /// complexity: 当前消息的复杂度 (0.0~1.0)
    /// duration_secs: 当前交互的时长（秒）
    pub fn detect(
        &mut self,
        is_demand: bool,
        complexity: f64,
        duration_secs: u64,
    ) -> Vec<DemandOverloadResult> {
        if !self.config.enabled {
            return Vec::new();
        }

        // 更新状态
        if is_demand {
            self.consecutive_demand_count += 1;
        } else {
            self.consecutive_demand_count = 0;
        }
        self.interaction_duration_secs += duration_secs;
        self.complexity_sum += complexity;
        self.complexity_count += 1;

        let mut results = Vec::new();

        // 检测连续索取
        if self.consecutive_demand_count >= self.config.consecutive_demand_threshold {
            let excess = self.consecutive_demand_count - self.config.consecutive_demand_threshold;
            let confidence = (0.5 + excess as f64 * 0.1).min(1.0);
            results.push(DemandOverloadResult {
                overload_type: DemandOverloadType::ConsecutiveDemand,
                confidence,
                response: if excess >= 3 {
                    DemandResponse::GentleRefusal
                } else if excess >= 1 {
                    DemandResponse::OfferAlternative
                } else {
                    DemandResponse::PartialFulfillment
                },
            });
        }

        // 检测时间侵占
        let duration_mins = self.interaction_duration_secs / 60;
        if duration_mins >= self.config.time_encroachment_threshold_mins as u64 {
            let confidence = (0.6
                + (duration_mins - self.config.time_encroachment_threshold_mins as u64) as f64
                    * 0.02)
                .min(1.0);
            results.push(DemandOverloadResult {
                overload_type: DemandOverloadType::TimeEncroachment,
                confidence,
                response: DemandResponse::DelayedResponse,
            });
        }

        // 检测复杂度堆积
        if self.complexity_count > 0 {
            let avg_complexity = self.complexity_sum / self.complexity_count as f64;
            if avg_complexity > self.config.complexity_threshold && self.complexity_count >= 5 {
                let confidence =
                    ((avg_complexity - self.config.complexity_threshold) / 0.2).min(1.0);
                results.push(DemandOverloadResult {
                    overload_type: DemandOverloadType::ComplexityAccumulation,
                    confidence,
                    response: DemandResponse::OfferAlternative,
                });
            }
        }

        results
    }

    /// 生成 Prompt 注入片段 / Generate prompt injection fragment
    pub fn to_prompt_fragment(&self, overloads: &[DemandOverloadResult]) -> String {
        if !self.config.enabled || overloads.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        parts.push("[Demand Boundary]".to_string());

        for ol in overloads {
            let response_str = match ol.response {
                DemandResponse::GentleRefusal => "Gently refuse: set a kind but firm boundary",
                DemandResponse::OfferAlternative => {
                    "Offer alternative: suggest a different approach"
                }
                DemandResponse::DelayedResponse => "Delay: suggest continuing later",
                DemandResponse::PartialFulfillment => "Partial: address part of the request",
                DemandResponse::None => continue,
            };
            parts.push(format!(
                "- {} detected (confidence {:.0}%): {}",
                ol.overload_type.label_zh(),
                ol.confidence * 100.0,
                response_str
            ));
        }

        parts.join("\n")
    }

    /// 重置连续计数（非索取消息后调用）/ Reset consecutive count
    pub fn reset_consecutive(&mut self) {
        self.consecutive_demand_count = 0;
    }
}

// ════════════════════════════════════════════════════════════════════
// BoundaryCoordinator — 边界协调器
// ════════════════════════════════════════════════════════════════════

/// 边界协调结果 / Boundary coordination result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoordinationResult {
    /// 情绪过载结果 / Emotional overload results
    pub emotional_overloads: Vec<OverloadResult>,
    /// 需求过载结果 / Demand overload results
    pub demand_overloads: Vec<DemandOverloadResult>,
    /// 协同调整后的需求容忍度 (0.0~1.0) / Adjusted demand tolerance
    pub adjusted_demand_tolerance: f64,
    /// 是否需要情绪保护 / Whether emotional protection is needed
    pub needs_emotional_protection: bool,
    /// 合并的 prompt 片段 / Combined prompt fragment
    pub prompt_fragment: String,
}

/// 边界协调器 / Boundary coordinator
///
/// 协调情绪边界和需求边界的交互：
/// - 情绪过载时降低需求容忍度
/// - 需求过载时触发情绪保护
pub struct BoundaryCoordinator;

impl BoundaryCoordinator {
    /// 协调两个边界的结果 / Coordinate results from both boundaries
    pub fn coordinate(
        emotional_overloads: &[OverloadResult],
        demand_overloads: &[DemandOverloadResult],
        stage: &RelationshipStage,
        base_tolerance: f64,
    ) -> CoordinationResult {
        // 计算情绪过载严重度
        let emotional_severity: f64 = emotional_overloads
            .iter()
            .map(|o| o.confidence * o.overload_type.severity())
            .sum::<f64>()
            .min(1.0);

        // 情绪过载时降低需求容忍度
        let adjusted_tolerance = (base_tolerance - emotional_severity * 0.4).max(0.1);

        // 需求过载时判断是否需要情绪保护
        let demand_severity: f64 = demand_overloads
            .iter()
            .map(|o| o.confidence * 0.5)
            .sum::<f64>()
            .min(1.0);
        let needs_emotional_protection = emotional_severity > 0.3 || demand_severity > 0.5;

        // 生成合并 prompt
        let mut parts = Vec::new();

        // 情绪边界 prompt
        let emo_boundary = EmotionalBoundary {
            config: EmotionalBoundaryConfig::default(),
            pleasure_history: Vec::new(),
            arousal_history: Vec::new(),
            sustained_negative_count: 0,
            sustained_low_arousal_count: 0,
            last_intervention_epoch: None,
        };
        let emo_frag = emo_boundary.to_prompt_fragment(emotional_overloads, stage);
        if !emo_frag.is_empty() {
            parts.push(emo_frag);
        }

        // 需求边界 prompt
        let dem_boundary = DemandBoundary::default();
        let dem_frag = dem_boundary.to_prompt_fragment(demand_overloads);
        if !dem_frag.is_empty() {
            parts.push(dem_frag);
        }

        // 协同提示
        if emotional_severity > 0.3 && demand_severity > 0.3 {
            parts.push(
                "[Boundary Coordination] Emotional and demand overload both detected. \
                 Prioritize emotional safety over task completion."
                    .to_string(),
            );
        }

        if adjusted_tolerance < base_tolerance * 0.7 {
            parts.push(format!(
                "[Boundary] Demand tolerance reduced to {:.0}% due to emotional state.",
                adjusted_tolerance * 100.0
            ));
        }

        CoordinationResult {
            emotional_overloads: emotional_overloads.to_vec(),
            demand_overloads: demand_overloads.to_vec(),
            adjusted_demand_tolerance: adjusted_tolerance,
            needs_emotional_protection,
            prompt_fragment: parts.join("\n"),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数 / Helper functions
// ════════════════════════════════════════════════════════════════════

/// 计算标准差 / Compute standard deviation
fn compute_stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn acquaintance() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 0,
        }
    }

    fn deep() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 200,
            shared_references: 20,
            key_moments: 10,
        }
    }

    // ── EmotionalBoundary 测试 ──

    #[test]
    fn test_emotional_boundary_default() {
        let b = EmotionalBoundary::default();
        assert!(b.config.enabled);
        assert!(b.pleasure_history.is_empty());
    }

    #[test]
    fn test_emotional_boundary_no_overload() {
        let mut b = EmotionalBoundary::default();
        let results = b.update(0.7, 0.5, 1000);
        assert!(results.is_empty());
    }

    #[test]
    fn test_emotional_boundary_sustained_negative() {
        let mut b = EmotionalBoundary::default();
        let mut results = Vec::new();
        for i in 0..6 {
            results = b.update(0.1, 0.5, 1000 + i * 100);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == OverloadType::SustainedNegative));
    }

    #[test]
    fn test_emotional_boundary_volatility() {
        let mut b = EmotionalBoundary::default();
        // Alternating high/low pleasure to create volatility
        // stddev of [0.9, 0.1, 0.8, 0.2, 0.9, 0.1, 0.8, 0.2] ≈ 0.35
        // Need > volatility_threshold (0.4), so use more extreme values
        let pleasures = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mut results = Vec::new();
        for (i, &p) in pleasures.iter().enumerate() {
            results = b.update(p, 0.5, 1000 + i as i64 * 100);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == OverloadType::Volatility));
    }

    #[test]
    fn test_emotional_boundary_exhaustion() {
        let mut b = EmotionalBoundary::default();
        let mut results = Vec::new();
        for i in 0..10 {
            results = b.update(0.5, 0.1, 1000 + i * 100);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == OverloadType::Exhaustion));
    }

    #[test]
    fn test_emotional_boundary_numbness() {
        let mut b = EmotionalBoundary::default();
        let mut results = Vec::new();
        // Low pleasure + low arousal + low volatility = numbness
        for i in 0..8 {
            results = b.update(0.15, 0.1, 1000 + i * 100);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == OverloadType::Numbness));
    }

    #[test]
    fn test_emotional_boundary_reset() {
        let mut b = EmotionalBoundary::default();
        b.update(0.1, 0.1, 1000);
        b.reset();
        assert!(b.pleasure_history.is_empty());
        assert_eq!(b.sustained_negative_count, 0);
    }

    #[test]
    fn test_emotional_to_prompt_fragment_empty() {
        let b = EmotionalBoundary::default();
        let frag = b.to_prompt_fragment(&[], &acquaintance());
        assert!(frag.is_empty());
    }

    #[test]
    fn test_emotional_to_prompt_fragment_with_overload() {
        let b = EmotionalBoundary::default();
        let overloads = vec![OverloadResult {
            overload_type: OverloadType::SustainedNegative,
            confidence: 0.8,
            intervention: EmotionalIntervention::CoolDown,
        }];
        let frag = b.to_prompt_fragment(&overloads, &deep());
        assert!(frag.contains("[Emotional Boundary]"));
        assert!(frag.contains("持续负情绪"));
    }

    #[test]
    fn test_emotional_disabled() {
        let config = EmotionalBoundaryConfig {
            enabled: false,
            ..Default::default()
        };
        let mut b = EmotionalBoundary::new(config);
        for i in 0..10 {
            let results = b.update(0.1, 0.1, 1000 + i * 100);
            assert!(results.is_empty());
        }
    }

    // ── DemandBoundary 测试 ──

    #[test]
    fn test_demand_boundary_default() {
        let b = DemandBoundary::default();
        assert!(b.config.enabled);
    }

    #[test]
    fn test_demand_boundary_no_overload() {
        let mut b = DemandBoundary::default();
        let results = b.detect(false, 0.3, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_demand_boundary_consecutive_demand() {
        let mut b = DemandBoundary::default();
        let mut results = Vec::new();
        for _ in 0..5 {
            results = b.detect(true, 0.5, 30);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == DemandOverloadType::ConsecutiveDemand));
    }

    #[test]
    fn test_demand_boundary_time_encroachment() {
        let mut b = DemandBoundary::default();
        // 70 minutes of interaction
        let results = b.detect(false, 0.3, 70 * 60);
        assert!(results
            .iter()
            .any(|r| r.overload_type == DemandOverloadType::TimeEncroachment));
    }

    #[test]
    fn test_demand_boundary_complexity() {
        let mut b = DemandBoundary::default();
        let mut results = Vec::new();
        for _ in 0..6 {
            results = b.detect(true, 0.9, 10);
        }
        assert!(results
            .iter()
            .any(|r| r.overload_type == DemandOverloadType::ComplexityAccumulation));
    }

    #[test]
    fn test_demand_to_prompt_fragment() {
        let b = DemandBoundary::default();
        let overloads = vec![DemandOverloadResult {
            overload_type: DemandOverloadType::ConsecutiveDemand,
            confidence: 0.7,
            response: DemandResponse::OfferAlternative,
        }];
        let frag = b.to_prompt_fragment(&overloads);
        assert!(frag.contains("[Demand Boundary]"));
        assert!(frag.contains("连续索取"));
    }

    #[test]
    fn test_demand_disabled() {
        let config = DemandBoundaryConfig {
            enabled: false,
            ..Default::default()
        };
        let mut b = DemandBoundary::new(config);
        for _ in 0..10 {
            let results = b.detect(true, 0.9, 100);
            assert!(results.is_empty());
        }
    }

    // ── BoundaryCoordinator 测试 ──

    #[test]
    fn test_coordinator_no_overload() {
        let result = BoundaryCoordinator::coordinate(&[], &[], &acquaintance(), 1.0);
        assert!(!result.needs_emotional_protection);
        assert!((result.adjusted_demand_tolerance - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_coordinator_emotional_reduces_tolerance() {
        let emo = vec![OverloadResult {
            overload_type: OverloadType::SustainedNegative,
            confidence: 0.8,
            intervention: EmotionalIntervention::CoolDown,
        }];
        let result = BoundaryCoordinator::coordinate(&emo, &[], &acquaintance(), 1.0);
        assert!(result.adjusted_demand_tolerance < 1.0);
        assert!(result.needs_emotional_protection);
    }

    #[test]
    fn test_coordinator_both_overload() {
        let emo = vec![OverloadResult {
            overload_type: OverloadType::Exhaustion,
            confidence: 0.9,
            intervention: EmotionalIntervention::PauseInteraction,
        }];
        let dem = vec![DemandOverloadResult {
            overload_type: DemandOverloadType::ConsecutiveDemand,
            confidence: 0.8,
            response: DemandResponse::GentleRefusal,
        }];
        let result = BoundaryCoordinator::coordinate(&emo, &dem, &deep(), 1.0);
        assert!(result.needs_emotional_protection);
        assert!(result.adjusted_demand_tolerance < 0.75);
        assert!(result.prompt_fragment.contains("[Boundary Coordination]"));
    }

    #[test]
    fn test_coordinator_prompt_fragment() {
        let emo = vec![OverloadResult {
            overload_type: OverloadType::Volatility,
            confidence: 0.6,
            intervention: EmotionalIntervention::SlowPace,
        }];
        let result = BoundaryCoordinator::coordinate(&emo, &[], &deep(), 1.0);
        assert!(!result.prompt_fragment.is_empty());
    }

    // ── 辅助函数测试 ──

    #[test]
    fn test_compute_stddev() {
        assert!((compute_stddev(&[]) - 0.0).abs() < 1e-6);
        assert!((compute_stddev(&[1.0, 1.0, 1.0]) - 0.0).abs() < 1e-6);
        let sd = compute_stddev(&[0.0, 1.0]);
        assert!((sd - 0.5).abs() < 1e-6);
    }
}
