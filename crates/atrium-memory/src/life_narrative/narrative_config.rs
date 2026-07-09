use super::*;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// CrossArcTheme — 跨弧主题 / Cross-Arc Theme
// ════════════════════════════════════════════════════════════════════

/// 跨弧主题 — 多条弧的共同主题 / Cross-arc theme — shared theme across multiple arcs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossArcTheme {
    /// 主题名称 / Theme name
    pub name: String,
    /// 主题描述 / Theme description
    pub description: String,
    /// 涉及的弧 ID / Involved arc IDs
    pub arc_ids: Vec<u64>,
    /// 主题显著度 / Theme significance
    pub significance: f64,
}

// ════════════════════════════════════════════════════════════════════
// CausalLink — 因果链 / Causal Link
// ════════════════════════════════════════════════════════════════════

/// 因果链 — 事件之间的因果叙事 / Causal link — causal narrative between events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    /// 因 / Cause
    pub cause: NarrativeEventId,
    /// 果 / Effect
    pub effect: NarrativeEventId,
    /// 因果叙述 / Causal narrative text
    pub narrative: String,
    /// 因果强度 / Causal strength
    pub strength: f64,
}

// ════════════════════════════════════════════════════════════════════
// NarrativeTone — 叙事语气 / Narrative Tone
// ════════════════════════════════════════════════════════════════════

/// 叙事语气 / Narrative tone
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NarrativeTone {
    /// 温暖怀旧 / Warm nostalgia
    WarmNostalgia,
    /// 苦涩怀念 / Bitter longing
    BitterLonging,
    /// 客观叙述 / Objective recall
    #[default]
    ObjectiveRecall,
    /// 活泼重现 / Vivid relive
    VividRelive,
    /// 谨慎回忆 / Tentative recall
    TentativeRecall,
    /// 自嘲回顾 / Self-deprecating
    SelfDeprecating,
}

impl NarrativeTone {
    /// 从 PAD 推断语气 / Infer tone from PAD
    pub fn from_pad(pad: &[f32; 3]) -> Self {
        let p = pad[0];
        let a = pad[1];

        if p > 0.3 && a > 0.3 {
            Self::VividRelive
        } else if p > 0.1 && a < 0.1 {
            Self::WarmNostalgia
        } else if p < -0.3 && a < 0.1 {
            Self::BitterLonging
        } else if p < -0.1 && a > 0.3 {
            Self::SelfDeprecating
        } else if p.abs() < 0.1 && a < 0.1 {
            Self::ObjectiveRecall
        } else {
            Self::TentativeRecall
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::WarmNostalgia => "温暖怀旧",
            Self::BitterLonging => "苦涩怀念",
            Self::ObjectiveRecall => "客观叙述",
            Self::VividRelive => "活泼重现",
            Self::TentativeRecall => "谨慎回忆",
            Self::SelfDeprecating => "自嘲回顾",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativePerspective / NarrativeStyle — 叙事视角与风格 / Narrative Perspective & Style
// ════════════════════════════════════════════════════════════════════

/// 叙事视角 / Narrative perspective
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NarrativePerspective {
    /// 第一人称 / First person
    #[default]
    FirstPerson,
    /// 第三人称 / Third person
    ThirdPerson,
    /// 双视角 / Dual perspective
    DualPerspective,
}

/// 叙事风格 / Narrative style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NarrativeStyle {
    /// 内省式 / Introspective
    Introspective,
    /// 记叙式 / Narrative
    Narrative,
    /// 抒情式 / Lyrical
    Lyrical,
    /// 混合式 / Adaptive
    #[default]
    Adaptive,
}

// ════════════════════════════════════════════════════════════════════
// NarrativeCfg — 叙事系统配置 / Narrative System Config
// ════════════════════════════════════════════════════════════════════

/// 叙事系统配置 / Narrative system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeCfg {
    /// 是否启用 / Whether enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 叙事视角 / Narrative perspective
    #[serde(default)]
    pub perspective: NarrativePerspective,
    /// 叙事风格 / Narrative style
    #[serde(default)]
    pub style: NarrativeStyle,
    /// 章节正文字数下限 / Min body word count
    #[serde(default = "default_body_min")]
    pub body_min_words: usize,
    /// 章节正文字数上限 / Max body word count
    #[serde(default = "default_body_max")]
    pub body_max_words: usize,
    /// 摘要最大字数 / Max summary word count
    #[serde(default = "default_summary_max")]
    pub summary_max_words: usize,
    /// Prompt 注入预算（字符数）/ Prompt injection budget (chars)
    #[serde(default = "default_prompt_budget")]
    pub prompt_budget: usize,
    /// 转折点检测配置 / Turning point detection config
    #[serde(default)]
    pub turning_point: TurningPointConfig,
    /// 弧最少转折点数 / Min turning points per arc
    #[serde(default = "default_min_arc_tp")]
    pub min_arc_turning_points: usize,
    /// 弧休眠天数 / Arc dormancy threshold (days)
    #[serde(default = "default_dormancy_days")]
    pub arc_dormancy_days: i64,
    /// 弧完结天数 / Arc closure threshold (days)
    #[serde(default = "default_closure_days")]
    pub arc_closure_days: i64,
    /// 自我描述重写间隔天数 / Self-description rewrite interval (days)
    #[serde(default = "default_rewrite_days")]
    pub self_description_rewrite_interval_days: i64,
    /// 叙事 tick 间隔 / Narrative tick interval
    #[serde(default = "default_tick_interval")]
    pub tick_narrative_interval: u64,
}

pub(super) fn default_true() -> bool {
    true
}
pub(super) fn default_body_min() -> usize {
    200
}
pub(super) fn default_body_max() -> usize {
    500
}
pub(super) fn default_summary_max() -> usize {
    50
}
pub(super) fn default_prompt_budget() -> usize {
    800
}
pub(super) fn default_min_arc_tp() -> usize {
    2
}
pub(super) fn default_dormancy_days() -> i64 {
    14
}
pub(super) fn default_closure_days() -> i64 {
    60
}
pub(super) fn default_rewrite_days() -> i64 {
    30
}
pub(super) fn default_tick_interval() -> u64 {
    1000
}

impl Default for NarrativeCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            perspective: NarrativePerspective::FirstPerson,
            style: NarrativeStyle::Adaptive,
            body_min_words: 200,
            body_max_words: 500,
            summary_max_words: 50,
            prompt_budget: 800,
            turning_point: TurningPointConfig::default(),
            min_arc_turning_points: 2,
            arc_dormancy_days: 14,
            arc_closure_days: 60,
            self_description_rewrite_interval_days: 30,
            tick_narrative_interval: 1000,
        }
    }
}
