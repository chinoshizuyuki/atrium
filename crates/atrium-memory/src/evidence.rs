// SPDX-License-Identifier: MIT
//! Evidence 证据评分系统
//! Evidence — Evidence scoring system.

use crate::fact_store::Fact;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// 来源类型

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    DirectConversation, // 直接对话 (0.95)
    SelfReported,       // 用户提供 (0.85)
    LLMInference,       // LLM推断 (0.65)
    FileExtraction,     // 文件提取 (0.75)
    SystemDefault,      // 系统默认 (0.30)
}

impl SourceType {
    pub fn base_credibility(&self) -> f64 {
        match self {
            Self::DirectConversation => 0.95,
            Self::SelfReported => 0.85,
            Self::LLMInference => 0.65,
            Self::FileExtraction => 0.75,
            Self::SystemDefault => 0.30,
        }
    }
}

pub fn parse_source(source: &str) -> SourceType {
    if source.starts_with("对话")
        || source.starts_with("聊天")
        || source.starts_with("chat")
        || source.starts_with("conversation")
    {
        SourceType::DirectConversation
    } else if source.starts_with("用户") || source.starts_with("self") {
        SourceType::SelfReported
    } else if source.starts_with("推断") || source.starts_with("llm") {
        SourceType::LLMInference
    } else if source.starts_with("文件") || source.starts_with("file") {
        SourceType::FileExtraction
    } else {
        SourceType::SystemDefault
    }
}

// 配置

pub struct EvidenceConfig {
    pub source_weight: f64,
    pub consistency_weight: f64,
    pub recency_weight: f64,
    pub verify_count_weight: f64,
    pub emotion_weight: f64,
    pub max_age_seconds: u64,
    pub contradiction_penalty: f64,
}

impl Default for EvidenceConfig {
    fn default() -> Self {
        Self {
            source_weight: 0.30,
            consistency_weight: 0.25,
            recency_weight: 0.20,
            verify_count_weight: 0.15,
            emotion_weight: 0.10,
            max_age_seconds: 30 * 24 * 3600,
            contradiction_penalty: 0.5,
        }
    }
}

// 评分结果

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceScore {
    pub total: f64,
    pub breakdown: ScoreBreakdown,
    pub is_reliable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub source_score: f64,
    pub consistency_score: f64,
    pub recency_score: f64,
    pub verify_count_score: f64,
    pub emotion_adjustment: f64,
}

// 评分器

pub struct EvidenceScorer {
    config: EvidenceConfig,
}

impl EvidenceScorer {
    pub fn new(config: EvidenceConfig) -> Self {
        Self { config }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(EvidenceConfig::default())
    }

    pub fn evaluate(
        &self,
        fact: &Fact,
        soucre_type: SourceType,
        related: &[Fact],
        emotion_intensity: f64,
    ) -> EvidenceScore {
        let now = now_secs();
        let source_score = soucre_type.base_credibility();
        let consistency_score = self.score_consistency(fact, related);
        let recency_score = self.score_recency(fact.created_at, now);
        let verify_count_score = self.score_verify_count(fact.verify_count);
        let emotion_score = self.score_emotion(emotion_intensity);

        let total = source_score * self.config.source_weight
            + consistency_score * self.config.consistency_weight
            + recency_score * self.config.recency_weight
            + verify_count_score * self.config.verify_count_weight
            + emotion_score * self.config.emotion_weight;

        EvidenceScore {
            total: total.clamp(0.0, 1.0),
            breakdown: ScoreBreakdown {
                source_score,
                consistency_score,
                recency_score,
                verify_count_score,
                emotion_adjustment: emotion_score,
            },
            is_reliable: total >= 0.5,
        }
    }

    /// 各维度评分
    fn score_consistency(&self, fact: &Fact, related: &[Fact]) -> f64 {
        if related.is_empty() {
            return 1.0;
        }
        let mut contradictions = 0;
        let mut confirmations = 0;
        for other in related {
            if other.subject.to_lowercase() == fact.subject.to_lowercase()
                && other.object.to_lowercase() == fact.object.to_lowercase()
            {
                let a = fact.predicate.to_lowercase();
                let b = other.predicate.to_lowercase();
                if is_contradictory(&a, &b) {
                    contradictions += 1;
                } else if a == b {
                    confirmations += 1;
                }
            }
        }
        if contradictions > 0 {
            (1.0 - self.config.contradiction_penalty * contradictions as f64).max(0.0)
        } else {
            (1.0 + (confirmations as f64).min(1.0) * 0.1).min(1.0)
        }
    }

    fn score_recency(&self, created_at: u64, now: u64) -> f64 {
        let age = now.saturating_sub(created_at);
        if age >= self.config.max_age_seconds {
            0.0
        } else {
            1.0 - age as f64 / self.config.max_age_seconds as f64
        }
    }

    fn score_verify_count(&self, count: u32) -> f64 {
        match count {
            0 => 0.1,
            1 => 0.4,
            2 => 0.6,
            3 => 0.75,
            4 => 0.85,
            5..=10 => 0.9,
            _ => 1.0,
        }
    }

    fn score_emotion(&self, intensity: f64) -> f64 {
        (if intensity <= 0.3 {
            1.0
        } else if intensity <= 0.7 {
            1.0 - (intensity - 0.3) * 0.3
        } else {
            0.88 - (intensity - 0.7) * 0.3
        })
        .max(0.5)
    }
}

/// 矛盾检测
pub fn is_contradictory(a: &str, b: &str) -> bool {
    if a == b {
        return false;
    }
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // 中文否定
    let neg_a = a.contains('不') || a.contains('没');
    let neg_b = b.contains('不') || b.contains('没');

    // 英文否定
    let neg_a_en =
        a_lower.starts_with("un") || a_lower.starts_with("dis") || a_lower.starts_with("not_");
    let neg_b_en =
        b_lower.starts_with("un") || b_lower.starts_with("dis") || b_lower.starts_with("not_");

    if (neg_a || neg_a_en) != (neg_b || neg_b_en) {
        let core_a = a.replace("不", "").replace("没", "");
        let core_b = b.replace("不", "").replace("没", "");
        if core_a.to_lowercase() == core_b.to_lowercase() {
            return true;
        }
    }

    // 中英文反义词
    let pairs = [
        ("喜欢", "讨厌"),
        ("like", "hate"),
        ("like", "dislike"),
        ("是", "不是"),
        ("is", "isnt"),
        ("is", "is_not"),
        ("在", "不在"),
        ("好", "坏"),
        ("大", "小"),
    ];
    for (x, y) in &pairs {
        let xl = x.to_lowercase();
        let yl = y.to_lowercase();
        if (a_lower == xl && b_lower == yl) || (a_lower == yl && b_lower == xl) {
            return true;
        }
    }
    false
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact_store::Fact;

    #[test]
    fn test_source_credibility() {
        assert!((SourceType::DirectConversation.base_credibility() - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_config_weights_sum() {
        let c = EvidenceConfig::default();
        let s = c.source_weight
            + c.consistency_weight
            + c.recency_weight
            + c.verify_count_weight
            + c.emotion_weight;
        assert!((s - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_high_confidence_score() {
        let s = EvidenceScorer::default();
        let f = Fact::new("主人", "喜欢", "Rust").with_source("对话");
        let score = s.evaluate(&f, SourceType::DirectConversation, &[], 0.0);
        assert!(score.total > 0.7);
        assert!(score.is_reliable);
    }

    #[test]
    fn test_contradiction_reduces_score() {
        let s = EvidenceScorer::default();
        let f = Fact::new("主人", "喜欢", "Rust");
        let bad = vec![Fact::new("主人", "讨厌", "Rust")];
        let score = s.evaluate(&f, SourceType::DirectConversation, &bad, 0.0);
        assert!(score.breakdown.consistency_score < 1.0);
    }

    #[test]
    fn test_verify_count_improves() {
        let s = EvidenceScorer::default();
        assert!(s.score_verify_count(1) < s.score_verify_count(5));
    }

    #[test]
    fn test_emotion_penalty() {
        let s = EvidenceScorer::default();
        assert!(s.score_emotion(0.1) >= s.score_emotion(0.9));
    }
}
