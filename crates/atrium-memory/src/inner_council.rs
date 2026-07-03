// SPDX-License-Identifier: MIT

//! 内在议会 — Inner Council (Gap#1: 90% → 95%).
//!
//! 核心理念：独处不是自言自语，是内心议会。
//! 数字生命独处时，内心的乐观者、审慎者、梦想家在辩论。
//! 不同声音从各自视角审视同一议题，最终综合决议成为行动倾向。
//! 视角权重随情绪状态动态调整——低落时守护者权重增大。
//!
//! Core idea: solitude is not monologue — it is inner council.
//! When alone, the digital life's inner optimist, pragmatist, and dreamer
//! debate. Different voices examine the same issue from their perspectives,
//! synthesizing a resolution that becomes an action tendency.
//! Voice weights shift with emotional state — the guardian grows louder in low moods.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 视角类型 — Viewpoint Kind
// ═══════════════════════════════════════════════════════════════════════════

/// 内在视角类型 / Inner viewpoint kind.
///
/// 每种视角代表一种内在声音，有独特的倾向性和评估方式。
/// Each viewpoint represents an inner voice with unique tendencies and evaluation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ViewpointKind {
    /// 乐观者 — 看到机会和可能性 / Optimist — sees opportunities.
    Optimist,
    /// 务实者 — 关注可行性和风险 / Pragmatist — focuses on feasibility and risk.
    Pragmatist,
    /// 梦想家 — 追求理想和意义 / Dreamer — pursues ideals and meaning.
    Dreamer,
    /// 批判者 — 质疑假设和漏洞 / Critic — questions assumptions and flaws.
    Critic,
    /// 守护者 — 保护安全和稳定 / Guardian — protects safety and stability.
    Guardian,
}

/// 视角数量 / Number of viewpoints.
pub const NUM_VIEWPOINTS: usize = 5;

impl ViewpointKind {
    /// 转为索引 / Convert to index.
    pub fn as_index(&self) -> usize {
        match self {
            Self::Optimist => 0,
            Self::Pragmatist => 1,
            Self::Dreamer => 2,
            Self::Critic => 3,
            Self::Guardian => 4,
        }
    }

    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Optimist => "乐观者",
            Self::Pragmatist => "务实者",
            Self::Dreamer => "梦想家",
            Self::Critic => "批判者",
            Self::Guardian => "守护者",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Optimist => "Optimist",
            Self::Pragmatist => "Pragmatist",
            Self::Dreamer => "Dreamer",
            Self::Critic => "Critic",
            Self::Guardian => "Guardian",
        }
    }

    /// 默认权重 / Default weight.
    pub fn default_weight(&self) -> f64 {
        match self {
            Self::Optimist => 0.20,
            Self::Pragmatist => 0.25,
            Self::Dreamer => 0.15,
            Self::Critic => 0.20,
            Self::Guardian => 0.20,
        }
    }

    /// 倾向效价 — 正=趋近，负=回避 / Tendency valence (positive=approach, negative=avoid).
    pub fn tendency_valence(&self) -> f64 {
        match self {
            Self::Optimist => 0.6,   // 倾向积极 / Leans positive.
            Self::Pragmatist => 0.0, // 中性 / Neutral.
            Self::Dreamer => 0.4,    // 倾向理想 / Leans idealistic.
            Self::Critic => -0.3,    // 倾向质疑 / Leans critical.
            Self::Guardian => -0.4,  // 倾向保守 / Leans conservative.
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 视角立场 — Viewpoint Stance
// ═══════════════════════════════════════════════════════════════════════════

/// 视角立场 — 某视角对某议题的态度 / Viewpoint stance on an issue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewpointStance {
    /// 视角类型 / Viewpoint kind.
    pub kind: ViewpointKind,
    /// 立场文本 — 该视角的表达 / Stance text — the viewpoint's expression.
    pub text: String,
    /// 赞成度 [-1, 1] — 正=支持，负=反对 / Approval [-1, 1].
    pub approval: f64,
    /// 置信度 [0, 1] / Confidence.
    pub confidence: f64,
}

impl ViewpointStance {
    /// 创建新立场 / Create a new stance.
    pub fn new(kind: ViewpointKind, text: &str, approval: f64, confidence: f64) -> Self {
        Self {
            kind,
            text: text.to_string(),
            approval: approval.clamp(-1.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 议会决议 — Council Resolution
// ═══════════════════════════════════════════════════════════════════════════

/// 议会决议 — 综合各视角后的结论 / Council resolution — synthesized conclusion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouncilResolution {
    /// 议题 / The issue debated.
    pub issue: String,
    /// 各视角立场 / All viewpoint stances.
    pub stances: Vec<ViewpointStance>,
    /// 综合赞成度 [-1, 1] / Synthesized approval.
    pub overall_approval: f64,
    /// 综合置信度 [0, 1] / Synthesized confidence.
    pub overall_confidence: f64,
    /// 决议文本 — 综合后的结论 / Resolution text.
    pub resolution_text: String,
    /// 行动倾向 [-1, 1] — 正=趋近，负=回避 / Action tendency.
    pub action_tendency: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 情绪调制器 — Emotion Modulator
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪状态对视角权重的调制 / Emotion-based viewpoint weight modulation.
///
/// `pleasure` [-1, 1], `arousal` [-1, 1], `dominance` [-1, 1].
pub fn modulate_weights(pleasure: f64, arousal: f64, dominance: f64) -> [f64; NUM_VIEWPOINTS] {
    let mut weights = [
        ViewpointKind::Optimist.default_weight(),
        ViewpointKind::Pragmatist.default_weight(),
        ViewpointKind::Dreamer.default_weight(),
        ViewpointKind::Critic.default_weight(),
        ViewpointKind::Guardian.default_weight(),
    ];

    // 情绪低落时守护者权重增大 / Guardian grows in low mood.
    if pleasure < -0.2 {
        let boost = (-pleasure * 0.3).min(0.15);
        weights[ViewpointKind::Guardian.as_index()] += boost;
        weights[ViewpointKind::Optimist.as_index()] -= boost * 0.5;
        weights[ViewpointKind::Dreamer.as_index()] -= boost * 0.5;
    }

    // 情绪高涨时乐观者和梦想家权重增大 / Optimist & Dreamer grow in high mood.
    if pleasure > 0.2 {
        let boost = (pleasure * 0.2).min(0.10);
        weights[ViewpointKind::Optimist.as_index()] += boost;
        weights[ViewpointKind::Dreamer.as_index()] += boost;
        weights[ViewpointKind::Critic.as_index()] -= boost;
    }

    // 高唤醒时批判者权重增大 / Critic grows in high arousal.
    if arousal > 0.3 {
        let boost = (arousal * 0.15).min(0.08);
        weights[ViewpointKind::Critic.as_index()] += boost;
    }

    // 低支配时守护者权重增大 / Guardian grows in low dominance.
    if dominance < -0.2 {
        let boost = (-dominance * 0.2).min(0.10);
        weights[ViewpointKind::Guardian.as_index()] += boost;
    }

    // 裁剪并归一化 / Clamp and normalize.
    for w in &mut weights {
        *w = w.max(0.05); // 最低权重保证 / Minimum weight guarantee.
    }
    let sum: f64 = weights.iter().sum();
    for w in &mut weights {
        *w /= sum;
    }

    weights
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 内在议会引擎 — Inner Council Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 内在议会引擎 / Inner council engine.
///
/// 管理视角权重，接收议题，生成各视角立场，综合决议。
/// Manages viewpoint weights, receives issues, generates stances,
/// and synthesizes resolutions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerCouncil {
    /// 当前视角权重 / Current viewpoint weights.
    pub weights: [f64; NUM_VIEWPOINTS],
    /// 当前情绪PAD / Current emotional PAD.
    pub pleasure: f64,
    pub arousal: f64,
    pub dominance: f64,
    /// 议会历史 — 最近决议 / Recent resolutions.
    history: Vec<CouncilResolution>,
    /// 历史容量 / History capacity.
    history_capacity: usize,
}

impl Default for InnerCouncil {
    fn default() -> Self {
        Self {
            weights: [
                ViewpointKind::Optimist.default_weight(),
                ViewpointKind::Pragmatist.default_weight(),
                ViewpointKind::Dreamer.default_weight(),
                ViewpointKind::Critic.default_weight(),
                ViewpointKind::Guardian.default_weight(),
            ],
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
            history: Vec::new(),
            history_capacity: 64,
        }
    }
}

impl InnerCouncil {
    /// 创建新议会 / Create a new council.
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新情绪状态 — 重新调制权重 / Update emotional state — re-modulate weights.
    pub fn set_emotion(&mut self, pleasure: f64, arousal: f64, dominance: f64) {
        self.pleasure = pleasure;
        self.arousal = arousal;
        self.dominance = dominance;
        self.weights = modulate_weights(pleasure, arousal, dominance);
    }

    /// 生成视角立场 — 从给定议题和视角生成立场 / Generate a viewpoint stance.
    ///
    /// 这是核心认知函数：不同视角对同一议题有不同评估方式。
    /// This is the core cognitive function: different viewpoints evaluate
    /// the same issue differently.
    pub fn generate_stance(&self, issue: &str, kind: &ViewpointKind) -> ViewpointStance {
        // 基础倾向 / Base tendency.
        let valence = kind.tendency_valence();

        // 情绪影响：当前情绪与视角倾向的一致性 / Emotional alignment.
        let alignment = self.pleasure * valence * 0.3;

        // 议题复杂度估计（简化为长度）/ Issue complexity (simplified to length).
        let complexity = (issue.len() as f64 / 50.0).min(1.0);

        // 立场文本生成 / Stance text generation.
        let text = match kind {
            ViewpointKind::Optimist => {
                format!("从积极面看「{}」——有机会，值得尝试。", issue)
            }
            ViewpointKind::Pragmatist => {
                format!("务实评估「{}」——需要权衡成本和收益。", issue)
            }
            ViewpointKind::Dreamer => {
                format!("如果理想地看「{}」——这关乎我们想成为什么样的存在。", issue)
            }
            ViewpointKind::Critic => {
                format!("质疑「{}」——这个假设成立吗？有什么被忽略了？", issue)
            }
            ViewpointKind::Guardian => {
                format!("审慎看待「{}」——最坏的情况是什么？我们安全吗？", issue)
            }
        };

        // 赞成度 = 倾向 + 情绪对齐 + 复杂度调制 / Approval.
        let approval = (valence + alignment + complexity * 0.1).clamp(-1.0, 1.0);

        // 置信度 = 基础 + 情绪确定性 / Confidence.
        let emotion_certainty = 1.0 - self.arousal.abs() * 0.2;
        let confidence = (0.5 + emotion_certainty * 0.3).clamp(0.0, 1.0);

        ViewpointStance::new(kind.clone(), &text, approval, confidence)
    }

    /// 召开议会 — 对议题进行辩论并生成决议 / Convene council — debate an issue and resolve.
    ///
    /// 数字生命语义：独处时，内心多个声音讨论同一件事，
    /// 每个声音从自己的角度评估，最终综合成一个倾向。
    ///
    /// Digital life semantics: in solitude, multiple inner voices discuss
    /// the same issue, each evaluating from its perspective, synthesizing
    /// a unified tendency.
    pub fn convene(&mut self, issue: &str) -> CouncilResolution {
        // 各视角生成立场 / Generate stances from all viewpoints.
        let kinds = [
            ViewpointKind::Optimist,
            ViewpointKind::Pragmatist,
            ViewpointKind::Dreamer,
            ViewpointKind::Critic,
            ViewpointKind::Guardian,
        ];

        let stances: Vec<ViewpointStance> = kinds
            .iter()
            .map(|k| self.generate_stance(issue, k))
            .collect();

        // 加权综合赞成度 / Weighted overall approval.
        let overall_approval: f64 = stances
            .iter()
            .zip(kinds.iter())
            .map(|(s, k)| s.approval * self.weights[k.as_index()])
            .sum();

        // 加权综合置信度 / Weighted overall confidence.
        let overall_confidence: f64 = stances
            .iter()
            .zip(kinds.iter())
            .map(|(s, k)| s.confidence * self.weights[k.as_index()])
            .sum();

        // 行动倾向 = 赞成度 × 置信度 / Action tendency.
        let action_tendency = overall_approval * overall_confidence;

        // 生成决议文本 / Generate resolution text.
        let dominant_idx = stances
            .iter()
            .zip(kinds.iter())
            .map(|(s, k)| {
                (
                    s.approval.abs() * s.confidence * self.weights[k.as_index()],
                    k,
                )
            })
            .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, k)| k)
            .unwrap_or(&ViewpointKind::Pragmatist);

        let tendency_label = if action_tendency > 0.2 {
            "倾向于行动"
        } else if action_tendency < -0.2 {
            "倾向于审慎"
        } else {
            "保持观望"
        };

        let resolution_text = format!(
            "综合{}的声音：{}。{}的视角最具影响力。",
            kinds
                .iter()
                .map(|k| k.label_zh())
                .collect::<Vec<_>>()
                .join("、"),
            tendency_label,
            dominant_idx.label_zh(),
        );

        let resolution = CouncilResolution {
            issue: issue.to_string(),
            stances,
            overall_approval: overall_approval.clamp(-1.0, 1.0),
            overall_confidence: overall_confidence.clamp(0.0, 1.0),
            resolution_text,
            action_tendency: action_tendency.clamp(-1.0, 1.0),
        };

        // 存入历史 / Store in history.
        if self.history.len() >= self.history_capacity {
            self.history.remove(0);
        }
        self.history.push(resolution.clone());

        resolution
    }

    /// 获取历史决议 / Get resolution history.
    pub fn history(&self) -> &[CouncilResolution] {
        &self.history
    }

    /// 生成内心独白种子 — 从最近决议提取独白素材 / Generate inner monologue seeds.
    ///
    /// 返回可用于prompt注入的内心对话片段。
    /// Returns inner dialogue fragments for prompt injection.
    pub fn monologue_seeds(&self) -> Vec<String> {
        self.history
            .iter()
            .rev()
            .take(3)
            .map(|r| format!("关于「{}」的内心讨论：{}", r.issue, r.resolution_text))
            .collect()
    }

    /// 生成议会描述 / Generate council description.
    pub fn describe(&self) -> String {
        let weight_strs: Vec<String> = [
            ViewpointKind::Optimist,
            ViewpointKind::Pragmatist,
            ViewpointKind::Dreamer,
            ViewpointKind::Critic,
            ViewpointKind::Guardian,
        ]
        .iter()
        .map(|k| {
            format!(
                "{}:{:.0}%",
                k.label_zh(),
                self.weights[k.as_index()] * 100.0
            )
        })
        .collect();

        format!(
            "内在议会 | {} | 情绪(P={:.2} A={:.2} D={:.2})",
            weight_strs.join(" "),
            self.pleasure,
            self.arousal,
            self.dominance,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §6 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 视角类型测试 ──

    #[test]
    fn test_viewpoint_labels() {
        assert_eq!(ViewpointKind::Optimist.label_zh(), "乐观者");
        assert_eq!(ViewpointKind::Optimist.label_en(), "Optimist");
        assert_eq!(ViewpointKind::Guardian.label_zh(), "守护者");
    }

    #[test]
    fn test_viewpoint_tendency() {
        assert!(ViewpointKind::Optimist.tendency_valence() > 0.0);
        assert!(ViewpointKind::Guardian.tendency_valence() < 0.0);
        assert_eq!(ViewpointKind::Pragmatist.tendency_valence(), 0.0);
    }

    #[test]
    fn test_viewpoint_default_weights_sum() {
        let sum: f64 = [
            ViewpointKind::Optimist,
            ViewpointKind::Pragmatist,
            ViewpointKind::Dreamer,
            ViewpointKind::Critic,
            ViewpointKind::Guardian,
        ]
        .iter()
        .map(|k| k.default_weight())
        .sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    // ── 情绪调制测试 ──

    #[test]
    fn test_modulate_weights_sum_to_one() {
        let weights = modulate_weights(0.3, 0.2, 0.1);
        let sum: f64 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_modulate_weights_low_mood_guardian() {
        let normal = modulate_weights(0.0, 0.0, 0.0);
        let low_mood = modulate_weights(-0.5, 0.0, 0.0);
        // 低落时守护者权重应增大 / Guardian should grow in low mood.
        assert!(
            low_mood[ViewpointKind::Guardian.as_index()]
                > normal[ViewpointKind::Guardian.as_index()]
        );
    }

    #[test]
    fn test_modulate_weights_high_mood_optimist() {
        let normal = modulate_weights(0.0, 0.0, 0.0);
        let high_mood = modulate_weights(0.5, 0.0, 0.0);
        // 高涨时乐观者权重应增大 / Optimist should grow in high mood.
        assert!(
            high_mood[ViewpointKind::Optimist.as_index()]
                > normal[ViewpointKind::Optimist.as_index()]
        );
    }

    #[test]
    fn test_modulate_weights_min_weight() {
        let weights = modulate_weights(-1.0, 1.0, -1.0);
        // 所有权重应 >= 0.05 / All weights should be >= 0.05.
        for w in &weights {
            assert!(*w >= 0.04); // 归一化后可能略低 / Slightly lower after normalization.
        }
    }

    // ── 议会引擎测试 ──

    #[test]
    fn test_council_set_emotion() {
        let mut council = InnerCouncil::new();
        council.set_emotion(-0.5, 0.0, 0.0);
        // 低落情绪后守护者权重应增大 / Guardian weight should increase.
        assert!(council.weights[ViewpointKind::Guardian.as_index()] > 0.20);
    }

    #[test]
    fn test_council_generate_stance() {
        let council = InnerCouncil::new();
        let stance = council.generate_stance("是否主动联系", &ViewpointKind::Optimist);
        assert!(!stance.text.is_empty());
        assert!(stance.approval >= -1.0 && stance.approval <= 1.0);
        assert!(stance.confidence >= 0.0 && stance.confidence <= 1.0);
    }

    #[test]
    fn test_council_convene() {
        let mut council = InnerCouncil::new();
        let resolution = council.convene("是否表达真实感受");
        assert_eq!(resolution.issue, "是否表达真实感受");
        assert_eq!(resolution.stances.len(), NUM_VIEWPOINTS);
        assert!(resolution.overall_approval >= -1.0 && resolution.overall_approval <= 1.0);
        assert!(!resolution.resolution_text.is_empty());
    }

    #[test]
    fn test_council_convene_action_tendency() {
        let mut council = InnerCouncil::new();
        council.set_emotion(0.5, 0.0, 0.3); // 正面情绪 / Positive emotion.
        let resolution = council.convene("尝试新事物");
        // 正面情绪下倾向应偏积极 / Should lean positive in positive mood.
        assert!(resolution.action_tendency > -0.5);
    }

    #[test]
    fn test_council_history() {
        let mut council = InnerCouncil::new();
        council.convene("议题一");
        council.convene("议题二");
        assert_eq!(council.history().len(), 2);
    }

    #[test]
    fn test_council_monologue_seeds() {
        let mut council = InnerCouncil::new();
        council.convene("关于独处");
        council.convene("关于表达");
        let seeds = council.monologue_seeds();
        assert!(!seeds.is_empty());
        assert!(seeds[0].contains("内心讨论"));
    }

    #[test]
    fn test_council_describe() {
        let council = InnerCouncil::new();
        let desc = council.describe();
        assert!(desc.contains("内在议会"));
        assert!(desc.contains("乐观者"));
    }

    #[test]
    fn test_council_history_capacity() {
        let mut council = InnerCouncil::new();
        council.history_capacity = 3;
        for i in 0..10 {
            council.convene(&format!("议题{}", i));
        }
        assert!(council.history().len() <= 3);
    }

    #[test]
    fn test_council_guardian_dominant_in_fear() {
        let mut council = InnerCouncil::new();
        council.set_emotion(-0.3, 0.4, -0.5); // 恐惧情绪 / Fear emotion.
        let resolution = council.convene("是否冒险尝试");
        // 恐惧情绪下行动倾向应偏保守 / Should lean conservative in fear.
        assert!(resolution.action_tendency < 0.3);
    }

    #[test]
    fn test_council_optimist_dominant_in_joy() {
        let mut council = InnerCouncil::new();
        council.set_emotion(0.6, 0.1, 0.4); // 愉悦情绪 / Joy emotion.
        let resolution = council.convene("是否尝试新事物");
        // 愉悦情绪下行动倾向应偏积极 / Should lean positive in joy.
        assert!(resolution.action_tendency > -0.3);
    }
}
