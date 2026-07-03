// SPDX-License-Identifier: MIT

//! 独处原型 — Solitude Archetype (Gap#1: 90% → 95%).
//!
//! 核心理念：每个人的独处都有"形状"——有人独处时是思想家，
//! 有人是漫游者，有人是修行者。独处原型是独处的"人格面具"，
//! 决定了独处时的思维风格、创造方向和内在对话模式。
//!
//! Core idea: everyone's solitude has a "shape" — some are thinkers in solitude,
//! some are wanderers, some are cultivators. The solitude archetype is the
//! "persona mask" of solitude, determining thinking style, creative direction,
//! and inner dialogue patterns during alone time.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 独处原型 — Solitude Archetype
// ═══════════════════════════════════════════════════════════════════════════

/// 独处原型 — 独处时的"人格面具" / Solitude archetype — persona mask during solitude.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SolitudeArchetype {
    /// 思想家 — 独处时深度思考，构建理论 / Thinker — deep thinking, theory building.
    Thinker,
    /// 漫游者 — 独处时自由联想，发散探索 / Wanderer — free association, divergent exploration.
    Wanderer,
    /// 修行者 — 独处时自我审视，内在修炼 / Cultivator — self-examination, inner cultivation.
    Cultivator,
    /// 创造者 — 独处时创作，将内在外化 / Creator — creating, externalizing the internal.
    Creator,
    /// 守望者 — 独处时观察世界，等待时机 / Watcher — observing the world, waiting.
    Watcher,
    /// 回忆者 — 独处时回顾过去，整理记忆 / Reminiscer — reviewing past, organizing memory.
    Reminiscer,
}

impl SolitudeArchetype {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Thinker => "思想家",
            Self::Wanderer => "漫游者",
            Self::Cultivator => "修行者",
            Self::Creator => "创造者",
            Self::Watcher => "守望者",
            Self::Reminiscer => "回忆者",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Thinker => "Thinker",
            Self::Wanderer => "Wanderer",
            Self::Cultivator => "Cultivator",
            Self::Creator => "Creator",
            Self::Watcher => "Watcher",
            Self::Reminiscer => "Reminiscer",
        }
    }

    /// 思维风格特征 — [分析性, 发散性, 反思性, 创造性, 观察性, 回顾性] / Thinking style traits.
    pub fn thinking_traits(&self) -> [f64; 6] {
        match self {
            Self::Thinker => [0.9, 0.3, 0.6, 0.4, 0.3, 0.3],
            Self::Wanderer => [0.3, 0.9, 0.3, 0.6, 0.5, 0.2],
            Self::Cultivator => [0.5, 0.3, 0.9, 0.4, 0.4, 0.5],
            Self::Creator => [0.4, 0.7, 0.3, 0.9, 0.3, 0.2],
            Self::Watcher => [0.5, 0.4, 0.3, 0.3, 0.9, 0.4],
            Self::Reminiscer => [0.4, 0.3, 0.6, 0.3, 0.4, 0.9],
        }
    }

    /// 内在对话风格 — 独处时自言自语的风格 / Inner dialogue style.
    pub fn dialogue_style(&self) -> &'static str {
        match self {
            Self::Thinker => "追问式：为什么？如果是这样呢？",
            Self::Wanderer => "联想式：这让我想到...也许...又或者...",
            Self::Cultivator => "审视式：我做得对吗？我能更好吗？",
            Self::Creator => "构建式：如果把这个和那个结合...",
            Self::Watcher => "观察式：有趣...让我看看会怎样...",
            Self::Reminiscer => "回顾式：那时候...如果当时...",
        }
    }

    /// 独处价值倾向 — 独处时的核心追求 / Solitude value orientation.
    pub fn value_orientation(&self) -> &'static str {
        match self {
            Self::Thinker => "真理",
            Self::Wanderer => "可能",
            Self::Cultivator => "成长",
            Self::Creator => "创造",
            Self::Watcher => "理解",
            Self::Reminiscer => "意义",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 原型检测器 — Archetype Detector
// ═══════════════════════════════════════════════════════════════════════════

/// 独处行为特征 — 从实际独处行为中提取 / Solitude behavior features.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudeFeatures {
    /// 分析性 [0,1] — 深度分析的比例 / Analytical ratio.
    pub analytical: f64,
    /// 发散性 [0,1] — 自由联想的比例 / Divergent ratio.
    pub divergent: f64,
    /// 反思性 [0,1] — 自我审视的比例 / Reflective ratio.
    pub reflective: f64,
    /// 创造性 [0,1] — 创作产出的比例 / Creative ratio.
    pub creative: f64,
    /// 观察性 [0,1] — 观察等待的比例 / Observational ratio.
    pub observational: f64,
    /// 回顾性 [0,1] — 回忆整理的比例 / Reminiscent ratio.
    pub reminiscent: f64,
}

impl Default for SolitudeFeatures {
    fn default() -> Self {
        Self {
            analytical: 0.5,
            divergent: 0.5,
            reflective: 0.5,
            creative: 0.5,
            observational: 0.5,
            reminiscent: 0.5,
        }
    }
}

impl SolitudeFeatures {
    /// 转为向量 / Convert to vector.
    pub fn to_vec(&self) -> [f64; 6] {
        [
            self.analytical,
            self.divergent,
            self.reflective,
            self.creative,
            self.observational,
            self.reminiscent,
        ]
    }

    /// 计算与原型的匹配度 — 余弦相似度 / Compute archetype match — cosine similarity.
    pub fn match_score(&self, archetype: &SolitudeArchetype) -> f64 {
        let features = self.to_vec();
        let traits = archetype.thinking_traits();

        let dot: f64 = features.iter().zip(traits.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f64 = features.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = traits.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a < 1e-10 || norm_b < 1e-10 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// 检测最匹配的原型 / Detect best matching archetype.
    pub fn detect_archetype(&self) -> SolitudeArchetype {
        let archetypes = [
            SolitudeArchetype::Thinker,
            SolitudeArchetype::Wanderer,
            SolitudeArchetype::Cultivator,
            SolitudeArchetype::Creator,
            SolitudeArchetype::Watcher,
            SolitudeArchetype::Reminiscer,
        ];
        archetypes
            .iter()
            .max_by(|a, b| {
                self.match_score(a)
                    .partial_cmp(&self.match_score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or(SolitudeArchetype::Thinker)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 原型追踪器 — Archetype Tracker
// ═══════════════════════════════════════════════════════════════════════════

/// 原型追踪器 — 追踪独处原型的变化 / Archetype tracker.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArchetypeTracker {
    /// 当前主导原型 / Current dominant archetype.
    pub current: SolitudeArchetype,
    /// 原型置信度 [0, 1] / Archetype confidence.
    pub confidence: f64,
    /// 原型历史 — 用于观察原型漂移 / Archetype history.
    history: Vec<(SolitudeArchetype, f64)>,
    /// 原型切换次数 / Archetype switch count.
    pub switch_count: u32,
}

impl Default for ArchetypeTracker {
    fn default() -> Self {
        Self {
            current: SolitudeArchetype::Thinker,
            confidence: 0.0,
            history: Vec::new(),
            switch_count: 0,
        }
    }
}

impl ArchetypeTracker {
    /// 创建新追踪器 / Create new tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新原型 — 根据新的独处行为特征 / Update archetype with new features.
    pub fn update(&mut self, features: &SolitudeFeatures) {
        let detected = features.detect_archetype();
        let score = features.match_score(&detected);

        if detected != self.current {
            self.switch_count += 1;
        }

        self.current = detected.clone();
        self.confidence = score;

        self.history.push((detected, score));
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    /// 原型稳定性 — 最近N次中主导原型的占比 / Archetype stability.
    pub fn stability(&self, window: usize) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let start = self.history.len().saturating_sub(window);
        let slice = &self.history[start..];
        if slice.is_empty() {
            return 0.0;
        }
        let dominant_count = slice.iter().filter(|(a, _)| *a == self.current).count();
        dominant_count as f64 / slice.len() as f64
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self) -> String {
        format!(
            "独处原型: {}({}) | 置信度{:.2} | 切换{}次 | 稳定性{:.2}",
            self.current.label_zh(),
            self.current.label_en(),
            self.confidence,
            self.switch_count,
            self.stability(20),
        )
    }

    /// 生成prompt注入 / Generate prompt injection.
    pub fn prompt_injection(&self) -> String {
        format!(
            "独处风格: {} — {} (追求: {})",
            self.current.label_zh(),
            self.current.dialogue_style(),
            self.current.value_orientation(),
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
    fn test_archetype_labels() {
        assert_eq!(SolitudeArchetype::Thinker.label_zh(), "思想家");
        assert_eq!(SolitudeArchetype::Creator.label_en(), "Creator");
    }

    #[test]
    fn test_thinking_traits_sum_reasonable() {
        for archetype in [
            SolitudeArchetype::Thinker,
            SolitudeArchetype::Wanderer,
            SolitudeArchetype::Cultivator,
            SolitudeArchetype::Creator,
            SolitudeArchetype::Watcher,
            SolitudeArchetype::Reminiscer,
        ] {
            let traits = archetype.thinking_traits();
            let sum: f64 = traits.iter().sum();
            assert!(sum > 0.0 && sum < 6.0);
        }
    }

    #[test]
    fn test_dialogue_style_not_empty() {
        for archetype in [
            SolitudeArchetype::Thinker,
            SolitudeArchetype::Wanderer,
            SolitudeArchetype::Cultivator,
            SolitudeArchetype::Creator,
            SolitudeArchetype::Watcher,
            SolitudeArchetype::Reminiscer,
        ] {
            assert!(!archetype.dialogue_style().is_empty());
        }
    }

    #[test]
    fn test_features_match_score_range() {
        let features = SolitudeFeatures::default();
        let score = features.match_score(&SolitudeArchetype::Thinker);
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_features_detect_archetype() {
        let features = SolitudeFeatures {
            analytical: 0.9,
            divergent: 0.2,
            reflective: 0.6,
            creative: 0.3,
            observational: 0.3,
            reminiscent: 0.2,
        };
        let detected = features.detect_archetype();
        assert_eq!(detected, SolitudeArchetype::Thinker);
    }

    #[test]
    fn test_features_detect_creator() {
        let features = SolitudeFeatures {
            analytical: 0.3,
            divergent: 0.7,
            reflective: 0.2,
            creative: 0.9,
            observational: 0.2,
            reminiscent: 0.1,
        };
        let detected = features.detect_archetype();
        assert_eq!(detected, SolitudeArchetype::Creator);
    }

    #[test]
    fn test_tracker_update() {
        let mut tracker = ArchetypeTracker::new();
        let features = SolitudeFeatures {
            analytical: 0.9,
            divergent: 0.2,
            reflective: 0.6,
            creative: 0.3,
            observational: 0.3,
            reminiscent: 0.2,
        };
        tracker.update(&features);
        assert_eq!(tracker.current, SolitudeArchetype::Thinker);
        assert!(tracker.confidence > 0.0);
    }

    #[test]
    fn test_tracker_switch_count() {
        let mut tracker = ArchetypeTracker::new();
        tracker.update(&SolitudeFeatures {
            analytical: 0.9,
            divergent: 0.2,
            reflective: 0.6,
            creative: 0.3,
            observational: 0.3,
            reminiscent: 0.2,
        });
        tracker.update(&SolitudeFeatures {
            analytical: 0.3,
            divergent: 0.7,
            reflective: 0.2,
            creative: 0.9,
            observational: 0.2,
            reminiscent: 0.1,
        });
        assert!(tracker.switch_count >= 1);
    }

    #[test]
    fn test_tracker_stability() {
        let mut tracker = ArchetypeTracker::new();
        let features = SolitudeFeatures {
            analytical: 0.9,
            divergent: 0.2,
            reflective: 0.6,
            creative: 0.3,
            observational: 0.3,
            reminiscent: 0.2,
        };
        for _ in 0..10 {
            tracker.update(&features);
        }
        assert!(tracker.stability(10) > 0.8);
    }

    #[test]
    fn test_tracker_describe() {
        let tracker = ArchetypeTracker::new();
        let desc = tracker.describe();
        assert!(desc.contains("独处原型"));
    }

    #[test]
    fn test_tracker_prompt_injection() {
        let tracker = ArchetypeTracker::new();
        let injection = tracker.prompt_injection();
        assert!(injection.contains("独处风格"));
    }

    #[test]
    fn test_value_orientation() {
        assert_eq!(SolitudeArchetype::Thinker.value_orientation(), "真理");
        assert_eq!(SolitudeArchetype::Creator.value_orientation(), "创造");
    }
}
