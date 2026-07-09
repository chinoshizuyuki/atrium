use super::*;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// NarrativeError — 叙事系统统一错误 / Narrative System Error
// ════════════════════════════════════════════════════════════════════

/// 叙事系统统一错误 / Unified narrative system error
#[derive(Debug)]
pub enum NarrativeError {
    /// 存储层错误 / Storage layer error
    Storage(String),
    /// 序列化错误 / Codec error
    Codec(String),
    /// 弧未找到 / Arc not found
    ArcNotFound(u64),
    /// 章节未找到 / Chapter not found
    ChapterNotFound(u64),
    /// 转折点未找到 / Turning point not found
    TurningPointNotFound(u64),
    /// 配置无效 / Invalid configuration
    InvalidConfig(String),
    /// LLM 生成失败 / LLM generation failed
    LlmFailed(String),
    /// 预算超限 / Budget exceeded
    BudgetExceeded { used: usize, budget: usize },
}

impl std::fmt::Display for NarrativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(e) => write!(f, "narrative storage error: {}", e),
            Self::Codec(e) => write!(f, "narrative codec error: {}", e),
            Self::ArcNotFound(id) => write!(f, "arc not found: {}", id),
            Self::ChapterNotFound(id) => write!(f, "chapter not found: {}", id),
            Self::TurningPointNotFound(id) => write!(f, "turning point not found: {}", id),
            Self::InvalidConfig(e) => write!(f, "invalid narrative config: {}", e),
            Self::LlmFailed(e) => write!(f, "narrative LLM generation failed: {}", e),
            Self::BudgetExceeded { used, budget } => {
                write!(f, "narrative budget exceeded: {}/{}", used, budget)
            }
        }
    }
}

impl std::error::Error for NarrativeError {}

// ════════════════════════════════════════════════════════════════════
// NarrativeSnapshot — 叙事快照 / Narrative Snapshot
// ════════════════════════════════════════════════════════════════════

/// 叙事快照 — 对外暴露的只读叙事状态 / Narrative snapshot — read-only state for external consumption
///
/// 用于 PromptWeaver 注入、API 响应等场景，避免暴露可变模型。
/// Used for PromptWeaver injection, API responses, etc., without exposing mutable model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeSnapshot {
    /// 自我认知摘要 / Self-cognition summary
    pub self_summary: String,
    /// 自我认知详述 / Self-cognition detailed description
    pub self_description: String,
    /// 身份标签 / Identity tags
    pub identity_tags: Vec<IdentityTag>,
    /// 活跃弧摘要 / Active arc summaries
    pub active_arcs: Vec<ArcSummary>,
    /// 最近转折点摘要 / Recent turning point summaries
    pub recent_turning_points: Vec<TurningPointSummary>,
    /// 关系叙事 / Relationship narrative
    pub relationship_narrative: String,
    /// 叙事统计 / Narrative statistics
    pub stats: NarrativeStats,
}

/// 弧摘要 — 快照中的弧精简表示 / Arc summary — compact arc in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcSummary {
    /// 弧 ID / Arc ID
    pub id: u64,
    /// 弧类型 / Arc kind
    pub kind: ArcKind,
    /// 弧标题 / Arc title
    pub title: String,
    /// 主题句 / Theme sentence
    pub theme_sentence: String,
    /// 章节数 / Chapter count
    pub chapter_count: usize,
    /// 转折点数 / Turning point count
    pub turning_point_count: usize,
    /// 显著度 / Significance
    pub significance: f64,
}

/// 转折点摘要 — 快照中的转折点精简表示 / Turning point summary — compact in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPointSummary {
    /// 转折点 ID / Turning point ID
    pub id: u64,
    /// 转折类型 / Turning point kind
    pub kind: TurningPointKind,
    /// 摘要 / Summary
    pub narrative_summary: String,
    /// 时间 / Timestamp
    pub timestamp: i64,
    /// 显著度 / Significance
    pub significance: f64,
}

impl NarrativeSnapshot {
    /// 从 NarrativeSelf 创建快照 / Create snapshot from NarrativeSelf
    pub fn from_model(model: &NarrativeSelf, max_recent_tp: usize) -> Self {
        let active_arcs = model
            .active_arcs
            .iter()
            .map(|a| ArcSummary {
                id: a.id,
                kind: a.kind,
                title: a.title.clone(),
                theme_sentence: a.theme_sentence.clone(),
                chapter_count: a.chapter_ids.len(),
                turning_point_count: a.turning_point_ids.len(),
                significance: a.significance,
            })
            .collect();

        // 取最近 N 个转折点（按时间倒序）/ Take N most recent TPs (reverse chronological)
        let mut tp_sorted: Vec<_> = model.turning_points.iter().collect();
        tp_sorted.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        let recent_turning_points = tp_sorted
            .into_iter()
            .take(max_recent_tp)
            .map(|t| TurningPointSummary {
                id: t.id,
                kind: t.kind,
                narrative_summary: t.narrative_summary.clone(),
                timestamp: t.timestamp,
                significance: t.significance,
            })
            .collect();

        Self {
            self_summary: model.self_summary.clone(),
            self_description: model.self_description.clone(),
            identity_tags: model.identity_tags.clone(),
            active_arcs,
            recent_turning_points,
            relationship_narrative: model.relationship_narrative.clone(),
            stats: model.stats.clone(),
        }
    }

    /// 快照是否为空 / Whether the snapshot is empty
    pub fn is_empty(&self) -> bool {
        self.active_arcs.is_empty()
            && self.recent_turning_points.is_empty()
            && self.identity_tags.is_empty()
            && self.self_summary.is_empty()
    }
}

// ════════════════════════════════════════════════════════════════════
// TurningPointPattern — 转折点检测模式 / Turning Point Detection Pattern
// ════════════════════════════════════════════════════════════════════

/// 转折点检测模式 — 从历史中学习的检测规则 / Turning point pattern — learned detection rule
///
/// 每个模式描述一种可识别的转折点信号模式，包括 PAD 变化方向、
/// 事件标签、关系阶段要求等。系统可从已确认的转折点中自动提炼模式。
/// Each pattern describes a recognizable signal pattern, including PAD change
/// direction, event tags, relationship stage requirements, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPointPattern {
    /// 模式 ID / Pattern ID
    pub id: u64,
    /// 对应的转折点类型 / Corresponding turning point kind
    pub kind: TurningPointKind,
    /// PAD 变化方向（正=增加，负=减少）/ PAD change direction
    pub pad_direction: [f32; 3],
    /// PAD 变化最小幅度 / Minimum PAD change magnitude
    pub pad_min_magnitude: f32,
    /// 需要的事件标签 / Required event tags
    pub required_tags: Vec<String>,
    /// 需要的关系阶段（None=任意）/ Required relationship stage (None=any)
    pub required_relationship_stage: Option<String>,
    /// 模式置信度 / Pattern confidence
    pub confidence: f64,
    /// 模式命中次数 / Pattern hit count
    pub hit_count: u32,
    /// 模式误报次数 / Pattern false positive count
    pub miss_count: u32,
}

impl TurningPointPattern {
    /// 创建新模式 / Create a new pattern
    pub fn new(
        id: u64,
        kind: TurningPointKind,
        pad_direction: [f32; 3],
        pad_min_magnitude: f32,
    ) -> Self {
        Self {
            id,
            kind,
            pad_direction,
            pad_min_magnitude,
            required_tags: Vec::new(),
            required_relationship_stage: None,
            confidence: 0.5,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// 模式精度 / Pattern precision
    pub fn precision(&self) -> f64 {
        if self.hit_count + self.miss_count == 0 {
            0.5
        } else {
            self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
        }
    }

    /// 记录命中 / Record a hit
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
        self.confidence = self.precision();
    }

    /// 记录误报 / Record a miss
    pub fn record_miss(&mut self) {
        self.miss_count += 1;
        self.confidence = self.precision();
    }

    /// 检查 PAD 变化是否匹配此模式 / Check if PAD change matches this pattern
    pub fn matches_pad_change(&self, pad_before: &[f32; 3], pad_after: &[f32; 3]) -> bool {
        let mut direction_ok = true;
        let mut magnitude_ok = true;
        for i in 0..3 {
            let delta = pad_after[i] - pad_before[i];
            // 方向检查：变化方向应与模式方向同号 / Direction: delta same sign as pattern
            if self.pad_direction[i].abs() > 0.01 && delta * self.pad_direction[i] < 0.0 {
                direction_ok = false;
            }
            // 幅度检查 / Magnitude check
            if self.pad_direction[i].abs() > 0.01
                && delta.abs() < self.pad_min_magnitude * self.pad_direction[i].abs()
            {
                magnitude_ok = false;
            }
        }
        direction_ok && magnitude_ok
    }
}
