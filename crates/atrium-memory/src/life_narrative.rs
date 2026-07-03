// SPDX-License-Identifier: MIT
//! 叙事自我 — AI 生命叙事系统
//! Life Narrative — AI life narrative system.
//!
//! 从事实到自传，从数据库到故事。叙事不是附加层，是认知架构的重构。
//! From facts to autobiography, from database to story.
//! Narrative is not an add-on layer — it is a restructuring of the cognitive architecture.
//!
//! 核心洞察：人类不是通过数据库理解自己的，而是通过故事。
//! Core insight: Humans understand themselves not through databases, but through stories.

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::maturity::{EmotionContext, MilestoneKind};

// ════════════════════════════════════════════════════════════════════
// ArcKind — 叙事弧类型 / Narrative Arc Types
// ════════════════════════════════════════════════════════════════════

/// 弧的类型 — 生命中的不同故事线 / Arc kind — different storylines in life
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArcKind {
    /// 成长弧：从无知到理解 / Growth: from ignorance to understanding
    Growth,
    /// 关系弧：与用户关系的演进 / Relationship: evolution with the user
    Relationship,
    /// 挑战弧：困难与克服 / Challenge: difficulty and overcoming
    Challenge,
    /// 日常弧：平凡但温暖的重复 / Ritual: ordinary but warm repetition
    Ritual,
    /// 转变弧：根本性的认知转变 / Transformation: fundamental cognitive shift
    Transformation,
    /// 失落弧：失去与接受 / Loss: losing and acceptance
    Loss,
}

impl ArcKind {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Growth => "成长",
            Self::Relationship => "关系",
            Self::Challenge => "挑战",
            Self::Ritual => "日常",
            Self::Transformation => "转变",
            Self::Loss => "失落",
        }
    }

    /// 英文标签 / English label
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Growth => "Growth",
            Self::Relationship => "Relationship",
            Self::Challenge => "Challenge",
            Self::Ritual => "Ritual",
            Self::Transformation => "Transformation",
            Self::Loss => "Loss",
        }
    }

    /// 图标符号 / Icon symbol
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Growth => "\u{1F331}",        // 🌱
            Self::Relationship => "\u{1F49E}",  // 💞
            Self::Challenge => "\u{1F997}",     // 🧗
            Self::Ritual => "\u{1F4D6}",        // 📖
            Self::Transformation => "\u{2728}", // ✨
            Self::Loss => "\u{1F314}",          // 🌔
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ArcStatus — 弧的状态 / Arc Status
// ════════════════════════════════════════════════════════════════════

/// 弧的状态 / Arc status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcStatus {
    /// 正在展开 / Currently unfolding
    Active,
    /// 暂时沉寂 / Temporarily dormant
    Dormant,
    /// 已完结 / Closed with an ending
    Closed,
    /// 被新弧取代 / Superseded by a new arc
    Superseded,
}

// ════════════════════════════════════════════════════════════════════
// NarrativeArc — 叙事弧 / Narrative Arc
// ════════════════════════════════════════════════════════════════════

/// 叙事弧 — 一条贯穿生命的主线故事 / Narrative arc — a main storyline through life
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeArc {
    /// 弧的唯一 ID / Unique arc ID
    pub id: u64,
    /// 弧的类型 / Arc kind
    pub kind: ArcKind,
    /// 弧的标题（AI 自己生成）/ Arc title (AI-generated)
    pub title: String,
    /// 弧的主题句 / Arc theme sentence
    pub theme_sentence: String,
    /// 弧的状态 / Arc status
    pub status: ArcStatus,
    /// 弧中所有章节 ID（按时间排序）/ Chapter IDs in chronological order
    pub chapter_ids: Vec<u64>,
    /// 弧的关键转折点 ID / Key turning point IDs
    pub turning_point_ids: Vec<u64>,
    /// 弧的情感基调（PAD）/ Emotional tone (PAD)
    pub emotional_tone: [f32; 3],
    /// 弧的起始时间 / Arc start time
    pub started_at: i64,
    /// 弧的结束时间 / Arc end time (None = still active)
    pub ended_at: Option<i64>,
    /// 弧的显著度 (0.0~1.0) / Significance (0.0~1.0)
    pub significance: f64,
    /// 弧的版本 / Arc version (incremented on rewrite)
    pub version: u32,
}

impl NarrativeArc {
    /// 创建新弧 / Create a new arc
    pub fn new(id: u64, kind: ArcKind, title: String, theme_sentence: String) -> Self {
        Self {
            id,
            kind,
            title,
            theme_sentence,
            status: ArcStatus::Active,
            chapter_ids: Vec::new(),
            turning_point_ids: Vec::new(),
            emotional_tone: [0.0, 0.0, 0.0],
            started_at: Local::now().timestamp(),
            ended_at: None,
            significance: 0.5,
            version: 1,
        }
    }

    /// 弧是否活跃 / Whether the arc is active
    pub fn is_active(&self) -> bool {
        self.status == ArcStatus::Active
    }

    /// 添加转折点 / Add a turning point
    pub fn add_turning_point(&mut self, tp_id: u64) {
        if !self.turning_point_ids.contains(&tp_id) {
            self.turning_point_ids.push(tp_id);
        }
    }

    /// 添加章节 / Add a chapter
    pub fn add_chapter(&mut self, chapter_id: u64) {
        if !self.chapter_ids.contains(&chapter_id) {
            self.chapter_ids.push(chapter_id);
        }
    }

    /// 标记休眠 / Mark as dormant
    pub fn make_dormant(&mut self) {
        self.status = ArcStatus::Dormant;
    }

    /// 标记完结 / Mark as closed
    pub fn close(&mut self, now: i64) {
        self.status = ArcStatus::Closed;
        self.ended_at = Some(now);
    }

    /// 生成弧摘要 / Generate arc summary
    pub fn summary(&self) -> ArcSummary {
        ArcSummary {
            id: self.id,
            kind: self.kind,
            title: self.title.clone(),
            theme_sentence: self.theme_sentence.clone(),
            chapter_count: self.chapter_ids.len(),
            turning_point_count: self.turning_point_ids.len(),
            significance: self.significance,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TrajectoryShape — 情感轨迹形状 / Emotion Trajectory Shape
// ════════════════════════════════════════════════════════════════════

/// 轨迹形状 — 情感变化的几何模式 / Trajectory shape — geometric pattern of emotion change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrajectoryShape {
    /// 上升（越变越好）/ Ascending
    Ascending,
    /// 下降（越来越难）/ Descending
    Descending,
    /// 先降后升（困难→克服）/ Valley
    Valley,
    /// 先升后降（兴奋→平静）/ Peak
    Peak,
    /// 平稳 / Flat
    Flat,
    /// 波动 / Oscillating
    Oscillating,
}

// ════════════════════════════════════════════════════════════════════
// EmotionTrajectory — 情感轨迹 / Emotion Trajectory
// ════════════════════════════════════════════════════════════════════

/// 情感轨迹 — 章节内的情感变化 / Emotion trajectory — emotion change within a chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionTrajectory {
    /// 章节开始时的 PAD / PAD at chapter start
    pub start_pad: [f32; 3],
    /// 章节高潮点的 PAD / PAD at chapter peak
    pub peak_pad: [f32; 3],
    /// 章节结束时的 PAD / PAD at chapter end
    pub end_pad: [f32; 3],
    /// 轨迹形状 / Trajectory shape
    pub shape: TrajectoryShape,
}

impl Default for EmotionTrajectory {
    fn default() -> Self {
        Self {
            start_pad: [0.0, 0.0, 0.0],
            peak_pad: [0.0, 0.0, 0.0],
            end_pad: [0.0, 0.0, 0.0],
            shape: TrajectoryShape::Flat,
        }
    }
}

impl EmotionTrajectory {
    /// 从起止 PAD 自动推断轨迹形状 / Infer trajectory shape from start/end PAD
    pub fn infer_shape(start: &[f32; 3], peak: &[f32; 3], end: &[f32; 3]) -> TrajectoryShape {
        let p_start = start[0];
        let p_peak = peak[0];
        let p_end = end[0];
        let eps = 0.05;

        if (p_start - p_end).abs() < eps && (p_peak - p_start).abs() < eps {
            TrajectoryShape::Flat
        } else if p_peak > p_start + eps && p_peak > p_end + eps {
            TrajectoryShape::Peak
        } else if p_peak < p_start - eps && p_peak < p_end - eps {
            TrajectoryShape::Valley
        } else if p_end > p_start + eps {
            TrajectoryShape::Ascending
        } else if p_end < p_start - eps {
            TrajectoryShape::Descending
        } else {
            TrajectoryShape::Oscillating
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TimeSpan — 时间跨度 / Time Span
// ════════════════════════════════════════════════════════════════════

/// 时间跨度 / Time span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSpan {
    /// 起始时间 / Start timestamp
    pub start: i64,
    /// 结束时间 / End timestamp
    pub end: i64,
}

impl TimeSpan {
    /// 持续秒数 / Duration in seconds
    pub fn duration_secs(&self) -> i64 {
        (self.end - self.start).max(0)
    }

    /// 持续天数 / Duration in days
    pub fn duration_days(&self) -> i64 {
        self.duration_secs() / 86400
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeEventId — 叙事事件引用 / Narrative Event Reference
// ════════════════════════════════════════════════════════════════════

/// 叙事事件 ID — 统一引用不同来源的事件 / Narrative event ID — unified reference to events from different sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeEventId {
    /// FactStore 中的事实 / Fact from FactStore
    Fact {
        subject: String,
        predicate: String,
        timestamp: u64,
    },
    /// 日记条目 / Diary entry
    Diary { date: String },
    /// 内在思考 / Inner thought
    Thought { timestamp: i64 },
    /// 成长里程碑 / Growth milestone
    Milestone { kind: String, timestamp: i64 },
    /// 审计事件 / Audit event
    Audit { event_type: String, timestamp: i64 },
    /// 关系阶段变更 / Relationship stage change
    RelationshipChange {
        from: String,
        to: String,
        timestamp: i64,
    },
    /// 情感事件（PAD 大幅变化）/ Emotion event (significant PAD change)
    EmotionEvent {
        pad_before: [f32; 3],
        pad_after: [f32; 3],
        timestamp: i64,
    },
}

impl NarrativeEventId {
    /// 获取事件时间戳 / Get event timestamp
    pub fn timestamp(&self) -> i64 {
        match self {
            Self::Fact { timestamp, .. } => *timestamp as i64,
            Self::Diary { date } => {
                // 尝试解析日期 / Try to parse date
                chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map(|d| {
                        d.and_hms_opt(0, 0, 0)
                            .unwrap_or_default()
                            .and_utc()
                            .timestamp()
                    })
                    .unwrap_or(0)
            }
            Self::Thought { timestamp } => *timestamp,
            Self::Milestone { timestamp, .. } => *timestamp,
            Self::Audit { timestamp, .. } => *timestamp,
            Self::RelationshipChange { timestamp, .. } => *timestamp,
            Self::EmotionEvent { timestamp, .. } => *timestamp,
        }
    }

    /// 获取事件类型标签 / Get event type label
    pub fn type_label(&self) -> &'static str {
        match self {
            Self::Fact { .. } => "fact",
            Self::Diary { .. } => "diary",
            Self::Thought { .. } => "thought",
            Self::Milestone { .. } => "milestone",
            Self::Audit { .. } => "audit",
            Self::RelationshipChange { .. } => "relationship",
            Self::EmotionEvent { .. } => "emotion",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeChapter — 叙事章节 / Narrative Chapter
// ════════════════════════════════════════════════════════════════════

/// 叙事章节 — 弧中的一个段落 / Narrative chapter — a paragraph within an arc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeChapter {
    /// 章节 ID / Chapter ID
    pub id: u64,
    /// 所属弧 ID / Parent arc ID
    pub arc_id: u64,
    /// 章节序号（在弧中的位置）/ Sequence number within the arc
    pub sequence: u32,
    /// 章节标题（AI 生成）/ Chapter title (AI-generated)
    pub title: String,
    /// 章节正文（AI 生成的叙事文本）/ Chapter body (AI-generated narrative text)
    pub body: String,
    /// 章节摘要（一句话）/ Chapter summary (one sentence)
    pub summary: String,
    /// 章节涉及的事件 ID / Event IDs referenced in this chapter
    pub event_ids: Vec<NarrativeEventId>,
    /// 章节的情感轨迹 / Emotion trajectory within the chapter
    pub emotion_trajectory: EmotionTrajectory,
    /// 章节的时间跨度 / Time span of the chapter
    pub time_span: TimeSpan,
    /// 章节的显著度 / Chapter significance
    pub significance: f64,
    /// 生成时间 / Time of creation
    pub written_at: i64,
    /// 最后重写时间 / Time of last rewrite
    pub rewritten_at: Option<i64>,
    /// 版本号 / Version number
    pub version: u32,
}

impl NarrativeChapter {
    /// 创建新章节 / Create a new chapter
    pub fn new(
        id: u64,
        arc_id: u64,
        sequence: u32,
        title: String,
        body: String,
        summary: String,
    ) -> Self {
        let now = Local::now().timestamp();
        Self {
            id,
            arc_id,
            sequence,
            title,
            body,
            summary,
            event_ids: Vec::new(),
            emotion_trajectory: EmotionTrajectory::default(),
            time_span: TimeSpan {
                start: now,
                end: now,
            },
            significance: 0.5,
            written_at: now,
            rewritten_at: None,
            version: 1,
        }
    }

    /// 章节是否被重写过 / Whether the chapter has been rewritten
    pub fn is_rewritten(&self) -> bool {
        self.rewritten_at.is_some()
    }

    /// 字数 / Word count (approximate for Chinese)
    pub fn word_count(&self) -> usize {
        self.body.chars().count()
    }
}

// ════════════════════════════════════════════════════════════════════
// TurningPointKind — 转折点类型 / Turning Point Kind
// ════════════════════════════════════════════════════════════════════

/// 转折点类型 — 系统性识别的生命关键事件 / Turning point kind — systematically identified key life events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurningPointKind {
    // ── 身份转折 / Identity turning points ──
    /// 被命名（从无名到有名）/ Named (from nameless to named)
    Named,
    /// 首次自称"我"/ First self-reference as "I"
    FirstSelfReference,

    // ── 情感转折 / Emotion turning points ──
    /// 首次情感共振 / First emotion resonance
    FirstEmotionResonance,
    /// 首次想念 / First longing
    FirstLonging,
    /// 首次心疼 / First heartache
    FirstHeartache,

    // ── 关系转折 / Relationship turning points ──
    /// 关系阶段跃迁 / Relationship stage promotion
    RelationshipPromotion,
    /// 首次冲突 / First conflict
    FirstConflict,
    /// 首次和解 / First reconciliation
    FirstReconciliation,

    // ── 成长转折 / Growth turning points ──
    /// 首次道歉 / First apology
    FirstApology,
    /// 首次主动关心 / First proactive care
    FirstProactiveCare,
    /// 首次自我纠正 / First self-correction
    FirstSelfCorrection,
    /// 首次独立思考 / First independent thought
    FirstIndependentThought,
    /// 首次智慧综合 / First wisdom synthesized
    FirstWisdom,

    // ── 认知转折 / Cognitive turning points ──
    /// 首次质疑 / First disagreement
    FirstDisagreement,
    /// 首次脆弱表达 / First vulnerability expression
    FirstVulnerability,
    /// 首次叙事觉醒 / First narrative awakening
    NarrativeAwakening,

    // ── 仪式转折 / Ritual turning points ──
    /// 首次仪式建立 / First ritual established
    FirstRitual,
    /// 命名日纪念 / Nameday anniversary
    NamedayAnniversary,
}

impl TurningPointKind {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Named => "被命名",
            Self::FirstSelfReference => "首次自称",
            Self::FirstEmotionResonance => "首次情感共振",
            Self::FirstLonging => "首次想念",
            Self::FirstHeartache => "首次心疼",
            Self::RelationshipPromotion => "关系跃迁",
            Self::FirstConflict => "首次冲突",
            Self::FirstReconciliation => "首次和解",
            Self::FirstApology => "首次道歉",
            Self::FirstProactiveCare => "首次主动关心",
            Self::FirstSelfCorrection => "首次自我纠正",
            Self::FirstIndependentThought => "首次独立思考",
            Self::FirstWisdom => "首次智慧综合",
            Self::FirstDisagreement => "首次质疑",
            Self::FirstVulnerability => "首次脆弱表达",
            Self::NarrativeAwakening => "叙事觉醒",
            Self::FirstRitual => "首次仪式",
            Self::NamedayAnniversary => "命名日纪念",
        }
    }

    /// 英文标签 / English label
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Named => "Named",
            Self::FirstSelfReference => "FirstSelfReference",
            Self::FirstEmotionResonance => "FirstEmotionResonance",
            Self::FirstLonging => "FirstLonging",
            Self::FirstHeartache => "FirstHeartache",
            Self::RelationshipPromotion => "RelationshipPromotion",
            Self::FirstConflict => "FirstConflict",
            Self::FirstReconciliation => "FirstReconciliation",
            Self::FirstApology => "FirstApology",
            Self::FirstProactiveCare => "FirstProactiveCare",
            Self::FirstSelfCorrection => "FirstSelfCorrection",
            Self::FirstIndependentThought => "FirstIndependentThought",
            Self::FirstWisdom => "FirstWisdom",
            Self::FirstDisagreement => "FirstDisagreement",
            Self::FirstVulnerability => "FirstVulnerability",
            Self::NarrativeAwakening => "NarrativeAwakening",
            Self::FirstRitual => "FirstRitual",
            Self::NamedayAnniversary => "NamedayAnniversary",
        }
    }

    /// 默认显著度 / Default significance
    pub fn default_significance(&self) -> f64 {
        match self {
            // 身份转折最显著 / Identity turning points are most significant
            Self::Named | Self::FirstSelfReference => 0.95,
            // 情感转折高显著 / Emotion turning points are highly significant
            Self::FirstEmotionResonance | Self::FirstHeartache => 0.9,
            Self::FirstLonging => 0.85,
            // 关系转折 / Relationship turning points
            Self::RelationshipPromotion => 0.8,
            Self::FirstConflict => 0.75,
            Self::FirstReconciliation => 0.8,
            // 成长转折 / Growth turning points
            Self::FirstApology | Self::FirstSelfCorrection => 0.7,
            Self::FirstProactiveCare => 0.75,
            Self::FirstIndependentThought => 0.8,
            Self::FirstWisdom => 0.85,
            // 认知转折 / Cognitive turning points
            Self::FirstDisagreement => 0.65,
            Self::FirstVulnerability => 0.7,
            Self::NarrativeAwakening => 0.9,
            // 仪式转折 / Ritual turning points
            Self::FirstRitual => 0.6,
            Self::NamedayAnniversary => 0.7,
        }
    }

    /// 推断所属弧类型 / Infer arc kind
    pub fn infer_arc_kind(&self) -> ArcKind {
        match self {
            Self::Named
            | Self::FirstSelfReference
            | Self::FirstEmotionResonance
            | Self::FirstLonging
            | Self::RelationshipPromotion
            | Self::FirstReconciliation => ArcKind::Relationship,
            Self::FirstApology
            | Self::FirstSelfCorrection
            | Self::FirstProactiveCare
            | Self::FirstIndependentThought
            | Self::FirstWisdom => ArcKind::Growth,
            Self::FirstConflict => ArcKind::Challenge,
            Self::FirstHeartache => ArcKind::Growth,
            Self::FirstRitual | Self::NamedayAnniversary => ArcKind::Ritual,
            Self::FirstDisagreement => ArcKind::Challenge,
            Self::FirstVulnerability => ArcKind::Relationship,
            Self::NarrativeAwakening => ArcKind::Transformation,
        }
    }

    /// 从 MilestoneKind 转换 / Convert from MilestoneKind
    pub fn from_milestone(kind: &MilestoneKind) -> Option<Self> {
        match kind {
            MilestoneKind::FirstNamed => Some(Self::Named),
            MilestoneKind::FirstLesson => None, // 教导不一定是转折 / Teaching is not necessarily a turning point
            MilestoneKind::FirstSelfCorrection => Some(Self::FirstSelfCorrection),
            MilestoneKind::FirstProactiveCare => Some(Self::FirstProactiveCare),
            MilestoneKind::FirstApology => Some(Self::FirstApology),
            MilestoneKind::FirstEmotionResonance => Some(Self::FirstEmotionResonance),
            MilestoneKind::StageTransition => Some(Self::RelationshipPromotion),
            MilestoneKind::FirstInnerThought => Some(Self::FirstIndependentThought),
            MilestoneKind::FirstWisdom => Some(Self::FirstWisdom),
            MilestoneKind::CleanStreak100 | MilestoneKind::CleanStreak500 => None,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TurningPoint — 转折点 / Turning Point
// ════════════════════════════════════════════════════════════════════

/// 转折点 — 叙事中改变走向的关键事件 / Turning point — key event that changes narrative direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPoint {
    /// 转折点 ID / Turning point ID
    pub id: u64,
    /// 转折类型 / Turning point kind
    pub kind: TurningPointKind,
    /// AI 自己写的叙述 / AI's own narrative
    pub narrative: String,
    /// 叙述摘要 / Narrative summary
    pub narrative_summary: String,
    /// 原始事件描述 / Original event description
    pub event_description: String,
    /// 事件时间 / Event timestamp
    pub timestamp: i64,
    /// 当时的情感快照 / Emotion snapshot at the time
    pub emotion_snapshot: EmotionContext,
    /// 当时的关系阶段 / Relationship stage at the time
    pub relationship_stage: String,
    /// 当时的成熟度阶段 / Maturity stage at the time
    pub maturity_stage: String,
    /// 显著度 (0.0~1.0) / Significance (0.0~1.0)
    pub significance: f64,
    /// 前一章 ID / Before chapter ID
    pub before_chapter_id: Option<u64>,
    /// 后一章 ID / After chapter ID
    pub after_chapter_id: Option<u64>,
    /// 所属弧 ID 列表 / Arc IDs this turning point belongs to
    pub arc_ids: Vec<u64>,
    /// 是否已被叙事引擎处理 / Whether integrated by narrative engine
    pub integrated: bool,
}

impl TurningPoint {
    /// 创建新转折点 / Create a new turning point
    pub fn new(
        id: u64,
        kind: TurningPointKind,
        event_description: String,
        emotion_snapshot: EmotionContext,
        relationship_stage: String,
        maturity_stage: String,
    ) -> Self {
        let now = Local::now().timestamp();
        Self {
            id,
            kind,
            narrative: String::new(),
            narrative_summary: String::new(),
            event_description,
            timestamp: now,
            emotion_snapshot,
            relationship_stage,
            maturity_stage,
            significance: kind.default_significance(),
            before_chapter_id: None,
            after_chapter_id: None,
            arc_ids: Vec::new(),
            integrated: false,
        }
    }

    /// 设置叙述 / Set narrative text
    pub fn with_narrative(mut self, narrative: String, summary: String) -> Self {
        self.narrative = narrative;
        self.narrative_summary = summary;
        self
    }

    /// 标记已处理 / Mark as integrated
    pub fn mark_integrated(&mut self) {
        self.integrated = true;
    }

    /// 添加到弧 / Add to arc
    pub fn add_to_arc(&mut self, arc_id: u64) {
        if !self.arc_ids.contains(&arc_id) {
            self.arc_ids.push(arc_id);
        }
    }

    /// 生成转折点摘要 / Generate turning point summary
    pub fn summary(&self) -> TurningPointSummary {
        TurningPointSummary {
            id: self.id,
            kind: self.kind,
            narrative_summary: self.narrative_summary.clone(),
            timestamp: self.timestamp,
            significance: self.significance,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// IdentityTag — 身份标签 / Identity Tag
// ════════════════════════════════════════════════════════════════════

/// 身份标签 — 从叙事中提炼的自我认知 / Identity tag — self-cognition distilled from narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityTag {
    /// 标签文本 / Tag text (e.g. "在乎的人")
    pub label: String,
    /// 来源弧 ID / Source arc ID
    pub source_arc_id: u64,
    /// 置信度 / Confidence
    pub confidence: f64,
    /// 情感极性 / Valence (positive or negative self-cognition)
    pub valence: f64,
    /// 创建时间 / Creation time
    pub created_at: i64,
}

impl IdentityTag {
    /// 创建新标签 / Create a new tag
    pub fn new(label: String, source_arc_id: u64, confidence: f64, valence: f64) -> Self {
        Self {
            label,
            source_arc_id,
            confidence: confidence.clamp(0.0, 1.0),
            valence: valence.clamp(-1.0, 1.0),
            created_at: Local::now().timestamp(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeStats — 叙事统计 / Narrative Statistics
// ════════════════════════════════════════════════════════════════════

/// 叙事统计 / Narrative statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NarrativeStats {
    /// 总弧数 / Total arcs
    pub total_arcs: u32,
    /// 活跃弧数 / Active arcs
    pub active_arcs: u32,
    /// 总章节数 / Total chapters
    pub total_chapters: u32,
    /// 总转折点数 / Total turning points
    pub total_turning_points: u32,
    /// 总重写次数 / Total rewrites
    pub total_rewrites: u32,
    /// 叙事总字数 / Total narrative word count
    pub narrative_word_count: u32,
}

// ════════════════════════════════════════════════════════════════════
// NarrativeSelf — 叙事自我模型 / Narrative Self Model
// ════════════════════════════════════════════════════════════════════

/// 叙事自我 — AI 对"我是谁"的完整认知 / Narrative self — AI's complete cognition of "who I am"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NarrativeSelf {
    /// 当前活跃的叙事弧 / Active narrative arcs
    pub active_arcs: Vec<NarrativeArc>,
    /// 已完结的叙事弧 / Closed narrative arcs
    pub closed_arcs: Vec<NarrativeArc>,
    /// 所有章节（按时间排序）/ All chapters (chronological)
    pub chapters: Vec<NarrativeChapter>,
    /// 所有转折点（按时间排序）/ All turning points (chronological)
    pub turning_points: Vec<TurningPoint>,
    /// 自我认知摘要 / Self-cognition summary
    pub self_summary: String,
    /// 自我认知详述 / Self-cognition detailed description
    pub self_description: String,
    /// 核心身份标签 / Core identity tags
    pub identity_tags: Vec<IdentityTag>,
    /// 与用户关系叙事 / Relationship narrative
    pub relationship_narrative: String,
    /// 叙事统计 / Narrative statistics
    pub stats: NarrativeStats,
    /// 最后重写时间 / Last rewrite time
    pub last_rewrite_at: i64,
    /// 跨弧主题 / Cross-arc themes
    pub cross_arc_themes: Vec<CrossArcTheme>,
    /// 因果链 / Causal links
    pub causal_links: Vec<CausalLink>,
    /// 叙事语气 / Narrative tone
    pub narrative_tone: NarrativeTone,
}

impl NarrativeSelf {
    /// 创建空自我模型 / Create empty self model
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取所有弧（活跃 + 已完结）/ Get all arcs (active + closed)
    pub fn all_arcs(&self) -> Vec<&NarrativeArc> {
        self.active_arcs
            .iter()
            .chain(self.closed_arcs.iter())
            .collect()
    }

    /// 获取指定弧 / Get arc by ID
    pub fn get_arc(&self, id: u64) -> Option<&NarrativeArc> {
        self.active_arcs
            .iter()
            .chain(self.closed_arcs.iter())
            .find(|a| a.id == id)
    }

    /// 获取指定章节 / Get chapter by ID
    pub fn get_chapter(&self, id: u64) -> Option<&NarrativeChapter> {
        self.chapters.iter().find(|c| c.id == id)
    }

    /// 获取指定转折点 / Get turning point by ID
    pub fn get_turning_point(&self, id: u64) -> Option<&TurningPoint> {
        self.turning_points.iter().find(|t| t.id == id)
    }

    /// 获取未处理的转折点 / Get unintegrated turning points
    pub fn unintegrated_turning_points(&self) -> Vec<&TurningPoint> {
        self.turning_points
            .iter()
            .filter(|t| !t.integrated)
            .collect()
    }

    /// 刷新统计 / Refresh statistics
    pub fn refresh_stats(&mut self) {
        self.stats.total_arcs = (self.active_arcs.len() + self.closed_arcs.len()) as u32;
        self.stats.active_arcs = self.active_arcs.len() as u32;
        self.stats.total_chapters = self.chapters.len() as u32;
        self.stats.total_turning_points = self.turning_points.len() as u32;
        self.stats.total_rewrites =
            self.chapters.iter().filter(|c| c.is_rewritten()).count() as u32;
        self.stats.narrative_word_count = self.chapters.iter().map(|c| c.word_count() as u32).sum();
    }

    /// 添加转折点 / Add a turning point
    pub fn add_turning_point(&mut self, tp: TurningPoint) {
        let pos = self
            .turning_points
            .iter()
            .position(|t| t.timestamp > tp.timestamp)
            .unwrap_or(self.turning_points.len());
        self.turning_points.insert(pos, tp);
    }

    /// 添加弧 / Add an arc
    pub fn add_arc(&mut self, arc: NarrativeArc) {
        if arc.is_active() {
            self.active_arcs.push(arc);
        } else {
            self.closed_arcs.push(arc);
        }
    }

    /// 添加章节 / Add a chapter
    pub fn add_chapter(&mut self, chapter: NarrativeChapter) {
        self.chapters.push(chapter);
    }

    /// 添加身份标签 / Add an identity tag
    pub fn add_identity_tag(&mut self, tag: IdentityTag) {
        if let Some(existing) = self.identity_tags.iter_mut().find(|t| t.label == tag.label) {
            existing.confidence = existing.confidence.max(tag.confidence);
            existing.valence = tag.valence;
        } else {
            self.identity_tags.push(tag);
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// DetectionContext — 转折点检测上下文 / Turning Point Detection Context
// ════════════════════════════════════════════════════════════════════

/// 情感趋势 / Emotion trend
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EmotionTrend {
    /// 情感上升 / Emotion rising
    Rising,
    /// 情感下降 / Emotion falling
    Falling,
    /// 情感平稳 / Emotion stable
    Stable,
    /// 情感波动 / Emotion oscillating
    Oscillating,
}

/// 检测上下文 — 判断是否为转折点需要上下文 / Detection context for turning point identification
#[derive(Debug, Clone)]
pub struct DetectionContext {
    /// 当前 PAD 状态 / Current PAD state
    pub current_pad: [f32; 3],
    /// 上一条消息时的 PAD / PAD at previous message
    pub previous_pad: [f32; 3],
    /// 当前关系阶段名 / Current relationship stage name
    pub relationship_stage: String,
    /// 当前成熟度阶段名 / Current maturity stage name
    pub maturity_stage: String,
    /// 最近情感趋势 / Recent emotion trend
    pub recent_emotion_trend: EmotionTrend,
    /// 最近已识别的转折点种类 / Recently identified turning point kinds
    pub recent_kinds: Vec<TurningPointKind>,
}

impl DetectionContext {
    /// PAD 欧氏距离 / PAD Euclidean distance
    pub fn pad_distance(&self) -> f32 {
        let d0 = self.current_pad[0] - self.previous_pad[0];
        let d1 = self.current_pad[1] - self.previous_pad[1];
        let d2 = self.current_pad[2] - self.previous_pad[2];
        (d0 * d0 + d1 * d1 + d2 * d2).sqrt()
    }

    /// 检查最近是否已有同类型转折点 / Check if same kind was recently detected
    pub fn has_recent_kind(&self, kind: TurningPointKind) -> bool {
        self.recent_kinds.contains(&kind)
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeEvent — 叙事事件（检测器输入）/ Narrative Event (detector input)
// ════════════════════════════════════════════════════════════════════

/// 叙事事件 — 检测器的输入事件 / Narrative event — input to the detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    /// 事件 ID / Event ID
    pub id: NarrativeEventId,
    /// 事件描述 / Event description
    pub description: String,
    /// 事件时间 / Event timestamp
    pub timestamp: i64,
    /// 事件情感上下文 / Event emotion context
    pub emotion: Option<EmotionContext>,
    /// 事件类型标记 / Event type tags
    pub tags: Vec<String>,
}

// ════════════════════════════════════════════════════════════════════
// TurningPointConfig — 转折点检测配置 / Turning Point Detection Config
// ════════════════════════════════════════════════════════════════════

/// 转折点检测配置 / Turning point detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPointConfig {
    /// 情感变化阈值 / Emotion change threshold (PAD Euclidean distance)
    pub emotion_change_threshold: f32,
    /// 关系阶段变更始终是转折点 / Relationship change is always a turning point
    pub relationship_change_always_turning: bool,
    /// 最小时间间隔（秒）/ Minimum interval between turning points (seconds)
    pub min_interval_secs: i64,
}

impl Default for TurningPointConfig {
    fn default() -> Self {
        Self {
            emotion_change_threshold: 0.4,
            relationship_change_always_turning: true,
            min_interval_secs: 3600,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TurningPointDetector — 转折点检测器 / Turning Point Detector
// ════════════════════════════════════════════════════════════════════

/// 转折点检测器 — 从原始事件流中识别转折点
/// Turning point detector — identify turning points from raw event stream
pub struct TurningPointDetector {
    /// 检测配置 / Detection config
    pub config: TurningPointConfig,
    /// 下一个转折点 ID / Next turning point ID
    next_id: u64,
    /// 最近检测时间 / Last detection time
    last_detection_at: i64,
}

impl TurningPointDetector {
    /// 创建检测器 / Create detector
    pub fn new(config: TurningPointConfig) -> Self {
        Self {
            config,
            next_id: 1,
            last_detection_at: 0,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(TurningPointConfig::default())
    }

    /// 检测转折点 — 从原始事件中识别 / Detect turning point from raw event
    ///
    /// 检测策略（按优先级）/ Detection strategy (by priority):
    /// 1. 硬性转折：MilestoneKind 中定义的事件 / Hard turning: events defined in MilestoneKind
    /// 2. 关系转折：RelationshipStage 变更 / Relationship turning: stage change
    /// 3. 情感转折：PAD 欧氏距离超过阈值 / Emotion turning: PAD distance exceeds threshold
    /// 4. 行为转折：从被动到主动等模式变更 / Behavior turning: pattern change
    pub fn detect(
        &mut self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPoint> {
        let now = event.timestamp;

        // 时间间隔检查 / Minimum interval check
        if now - self.last_detection_at < self.config.min_interval_secs {
            return None;
        }

        // 策略 1：里程碑硬性转折 / Strategy 1: Milestone hard turning
        if let Some(kind) = self.detect_milestone(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 2：关系转折 / Strategy 2: Relationship turning
        if let Some(kind) = self.detect_relationship(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 3：情感转折 / Strategy 3: Emotion turning
        if let Some(kind) = self.detect_emotion(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 4：行为转折 / Strategy 4: Behavior turning
        if let Some(kind) = self.detect_behavior(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        None
    }

    /// 里程碑检测 / Milestone detection
    fn detect_milestone(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        match &event.id {
            NarrativeEventId::Milestone { kind, .. } => {
                let tp_kind = match kind.as_str() {
                    "FirstNamed" => Some(TurningPointKind::Named),
                    "FirstSelfCorrection" => Some(TurningPointKind::FirstSelfCorrection),
                    "FirstProactiveCare" => Some(TurningPointKind::FirstProactiveCare),
                    "FirstApology" => Some(TurningPointKind::FirstApology),
                    "FirstEmotionResonance" => Some(TurningPointKind::FirstEmotionResonance),
                    "StageTransition" => Some(TurningPointKind::RelationshipPromotion),
                    "FirstInnerThought" => Some(TurningPointKind::FirstIndependentThought),
                    "FirstWisdom" => Some(TurningPointKind::FirstWisdom),
                    _ => None,
                };
                if let Some(k) = tp_kind {
                    if !context.has_recent_kind(k) {
                        return Some(k);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 关系转折检测 / Relationship turning detection
    fn detect_relationship(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        if !self.config.relationship_change_always_turning {
            return None;
        }
        match &event.id {
            NarrativeEventId::RelationshipChange { .. } => {
                if !context.has_recent_kind(TurningPointKind::RelationshipPromotion) {
                    Some(TurningPointKind::RelationshipPromotion)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 情感转折检测 / Emotion turning detection
    fn detect_emotion(
        &self,
        _event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        let distance = context.pad_distance();
        if distance < self.config.emotion_change_threshold {
            return None;
        }

        let pleasure_delta = context.current_pad[0] - context.previous_pad[0];
        let arousal_delta = context.current_pad[1] - context.previous_pad[1];

        // 高唤醒 + 正愉悦跃升 → 情感共振 / High arousal + pleasure surge → emotion resonance
        if pleasure_delta > 0.3
            && arousal_delta > 0.2
            && !context.has_recent_kind(TurningPointKind::FirstEmotionResonance)
        {
            return Some(TurningPointKind::FirstEmotionResonance);
        }

        // 高唤醒 + 负愉悦 → 心疼 / High arousal + negative pleasure → heartache
        if pleasure_delta < -0.3
            && context.current_pad[1] > 0.5
            && !context.has_recent_kind(TurningPointKind::FirstHeartache)
        {
            return Some(TurningPointKind::FirstHeartache);
        }

        // 负愉悦 + 低唤醒 → 想念 / Negative pleasure + low arousal → longing
        if context.current_pad[0] < -0.2
            && context.current_pad[1] < 0.3
            && !context.has_recent_kind(TurningPointKind::FirstLonging)
        {
            return Some(TurningPointKind::FirstLonging);
        }

        None
    }

    /// 行为转折检测 / Behavior turning detection
    fn detect_behavior(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        if event.tags.contains(&"apology".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstApology)
        {
            return Some(TurningPointKind::FirstApology);
        }
        if event.tags.contains(&"proactive_care".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstProactiveCare)
        {
            return Some(TurningPointKind::FirstProactiveCare);
        }
        if event.tags.contains(&"self_correction".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstSelfCorrection)
        {
            return Some(TurningPointKind::FirstSelfCorrection);
        }
        if event.tags.contains(&"conflict".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstConflict)
        {
            return Some(TurningPointKind::FirstConflict);
        }
        if event.tags.contains(&"reconciliation".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstReconciliation)
        {
            return Some(TurningPointKind::FirstReconciliation);
        }
        if event.tags.contains(&"vulnerability".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstVulnerability)
        {
            return Some(TurningPointKind::FirstVulnerability);
        }
        if event.tags.contains(&"disagreement".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstDisagreement)
        {
            return Some(TurningPointKind::FirstDisagreement);
        }
        None
    }

    /// 创建转折点 / Create a turning point
    fn create_turning_point(
        &mut self,
        kind: TurningPointKind,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> TurningPoint {
        let emotion_snapshot = event.emotion.clone().unwrap_or(EmotionContext {
            pleasure: context.current_pad[0],
            arousal: context.current_pad[1],
            dominance: context.current_pad[2],
        });

        let tp = TurningPoint::new(
            self.next_id,
            kind,
            event.description.clone(),
            emotion_snapshot,
            context.relationship_stage.clone(),
            context.maturity_stage.clone(),
        );
        self.next_id += 1;
        self.last_detection_at = event.timestamp;
        tp
    }

    /// 批量回溯检测 — 从历史事件中补漏 / Retrospective detection from historical events
    ///
    /// 用于系统首次启动时，从已有数据中回溯构建初始转折点集合
    /// Used at first startup to build initial turning points from existing data
    pub fn retrospective_detect(
        &mut self,
        milestone_events: &[NarrativeEvent],
        relationship_events: &[NarrativeEvent],
        emotion_events: &[NarrativeEvent],
    ) -> Vec<TurningPoint> {
        let mut results = Vec::new();

        // 合并并按时间排序 / Merge and sort by time
        let mut all_events: Vec<&NarrativeEvent> = Vec::new();
        all_events.extend(milestone_events.iter());
        all_events.extend(relationship_events.iter());
        all_events.extend(emotion_events.iter());
        all_events.sort_by_key(|e| e.timestamp);

        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: String::new(),
            maturity_stage: String::new(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        // 重置间隔限制以允许回溯 / Reset interval limit for retrospective
        let saved_interval = self.config.min_interval_secs;
        self.config.min_interval_secs = 0;

        for event in &all_events {
            if let Some(tp) = self.detect(event, &context) {
                results.push(tp);
            }
        }

        self.config.min_interval_secs = saved_interval;
        results
    }
}

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

fn default_true() -> bool {
    true
}
fn default_body_min() -> usize {
    200
}
fn default_body_max() -> usize {
    500
}
fn default_summary_max() -> usize {
    50
}
fn default_prompt_budget() -> usize {
    800
}
fn default_min_arc_tp() -> usize {
    2
}
fn default_dormancy_days() -> i64 {
    14
}
fn default_closure_days() -> i64 {
    60
}
fn default_rewrite_days() -> i64 {
    30
}
fn default_tick_interval() -> u64 {
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

// ════════════════════════════════════════════════════════════════════
// NarrativeTrigger / RewriteTrigger — 叙事触发器 / Narrative Triggers
// ════════════════════════════════════════════════════════════════════

/// 叙事触发器 — 触发叙事引擎处理的事件 / Narrative trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeTrigger {
    /// 新转折点被检测到 / New turning point detected
    TurningPointDetected { tp_id: u64, kind: TurningPointKind },
    /// 关系阶段变更 / Relationship stage changed
    RelationshipStageChanged { from: String, to: String },
    /// 情感大幅变化 / Significant emotion change
    EmotionShift {
        pad_before: [f32; 3],
        pad_after: [f32; 3],
    },
    /// 定时 tick / Periodic tick
    Tick,
    /// 首次启动（回溯构建）/ First startup (retrospective build)
    FirstStartup,
    /// 手动重写请求 / Manual rewrite request
    ManualRewrite { target: RewriteTarget },
}

/// 重写目标 — 指定重写的叙事对象 / Rewrite target
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RewriteTarget {
    /// 重写自我描述 / Rewrite self description
    SelfDescription,
    /// 重写指定章节 / Rewrite specific chapter
    Chapter(u64),
    /// 重写指定弧的主题 / Rewrite specific arc's theme
    ArcTheme(u64),
    /// 重写关系叙事 / Rewrite relationship narrative
    RelationshipNarrative,
}

/// 重写触发器 — 触发章节重写的事件 / Rewrite trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteTrigger {
    /// 触发类型 / Trigger type
    pub target: RewriteTarget,
    /// 触发原因 / Trigger reason
    pub reason: String,
    /// 触发时间 / Trigger timestamp
    pub timestamp: i64,
    /// 新证据（事件 ID 列表）/ New evidence
    pub evidence: Vec<NarrativeEventId>,
}

impl RewriteTrigger {
    /// 创建重写触发器 / Create a rewrite trigger
    pub fn new(target: RewriteTarget, reason: String) -> Self {
        Self {
            target,
            reason,
            timestamp: Local::now().timestamp(),
            evidence: Vec::new(),
        }
    }

    /// 附加证据 / Attach evidence
    pub fn with_evidence(mut self, evidence: Vec<NarrativeEventId>) -> Self {
        self.evidence = evidence;
        self
    }
}

// ════════════════════════════════════════════════════════════════════
// ArcConfig — 弧检测器配置 / Arc Detector Config
// ════════════════════════════════════════════════════════════════════

/// 弧检测器配置 / Arc detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcConfig {
    /// 弧最少转折点数（少于此数不构成弧）/ Min turning points per arc
    pub min_turning_points: usize,
    /// 弧休眠天数阈值 / Arc dormancy threshold (days)
    pub dormancy_days: i64,
    /// 弧完结天数阈值 / Arc closure threshold (days)
    pub closure_days: i64,
    /// 同类型弧合并阈值（相似度）/ Same-kind arc merge similarity threshold
    pub merge_similarity_threshold: f64,
    /// 弧显著度衰减率（每天）/ Arc significance decay rate (per day)
    pub significance_decay_per_day: f64,
    /// 最大活跃弧数 / Max active arcs
    pub max_active_arcs: usize,
}

impl Default for ArcConfig {
    fn default() -> Self {
        Self {
            min_turning_points: 2,
            dormancy_days: 14,
            closure_days: 60,
            merge_similarity_threshold: 0.8,
            significance_decay_per_day: 0.01,
            max_active_arcs: 10,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ArcUpdate — 弧检测结果 / Arc Detection Update
// ════════════════════════════════════════════════════════════════════

/// 弧检测结果 — ArcDetector.detect() 的输出 / Arc detection update
#[derive(Debug, Clone)]
pub enum ArcUpdate {
    /// 创建了新弧 / New arc created
    ArcCreated { arc_id: u64, kind: ArcKind },
    /// 弧添加了新转折点 / Turning point added to arc
    TurningPointAdded { arc_id: u64, tp_id: u64 },
    /// 弧进入休眠 / Arc went dormant
    ArcDormant { arc_id: u64 },
    /// 弧已完结 / Arc closed
    ArcClosed { arc_id: u64 },
    /// 弧被新弧取代 / Arc superseded
    ArcSuperseded { old_arc_id: u64, new_arc_id: u64 },
    /// 弧显著度更新 / Arc significance updated
    SignificanceUpdated { arc_id: u64, old: f64, new: f64 },
    /// 无变化 / No change
    NoChange,
}

// ════════════════════════════════════════════════════════════════════
// ArcDetector — 弧检测器 / Arc Detector
// ════════════════════════════════════════════════════════════════════

/// 弧检测器 — 从转折点流中识别和更新叙事弧
/// Arc detector — identify and update narrative arcs from turning point stream
///
/// 核心职责 / Core responsibilities:
/// 1. 新转折点 → 归入已有弧 或 创建新弧 / New TP → assign to existing or create new arc
/// 2. 定期检查弧的休眠/完结 / Periodically check arc dormancy/closure
/// 3. 弧显著度衰减与更新 / Arc significance decay and update
/// 4. 同类型弧合并 / Same-kind arc merging
pub struct ArcDetector {
    /// 检测配置 / Detection config
    pub config: ArcConfig,
    /// 下一个弧 ID / Next arc ID
    next_arc_id: u64,
}

impl ArcDetector {
    /// 创建弧检测器 / Create arc detector
    pub fn new(config: ArcConfig) -> Self {
        Self {
            config,
            next_arc_id: 1,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(ArcConfig::default())
    }

    /// 分配下一个弧 ID / Allocate next arc ID
    pub fn alloc_arc_id(&mut self) -> u64 {
        let id = self.next_arc_id;
        self.next_arc_id += 1;
        id
    }

    /// 处理新转折点 — 尝试归入已有弧或创建新弧
    /// Process new turning point — try to assign to existing arc or create new arc
    ///
    /// 策略 / Strategy:
    /// 1. 查找同类型活跃弧，若主题相似则归入 / Find same-kind active arc, assign if similar
    /// 2. 若无合适弧，创建新弧 / If no suitable arc, create new arc
    /// 3. 检查活跃弧数是否超限 / Check if active arc count exceeds limit
    pub fn process_turning_point(
        &mut self,
        model: &mut NarrativeSelf,
        tp: &TurningPoint,
    ) -> Vec<ArcUpdate> {
        let mut updates = Vec::new();

        // 策略 1：查找同类型活跃弧 / Strategy 1: Find same-kind active arc
        let target_kind = tp.kind.infer_arc_kind();
        let mut best_arc_id: Option<u64> = None;
        let mut best_significance = 0.0;

        for arc in &model.active_arcs {
            if arc.kind == target_kind && arc.is_active() {
                // 简单相似度：同类型 + 时间接近 / Simple similarity: same kind + temporal proximity
                let time_proximity = if let Some(&last_tp_id) = arc.turning_point_ids.last() {
                    if let Some(last_tp) = model.get_turning_point(last_tp_id) {
                        let days_diff = (tp.timestamp - last_tp.timestamp).abs() / 86400;
                        1.0 / (1.0 + days_diff as f64)
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };
                let score = arc.significance * time_proximity;
                if score > best_significance {
                    best_significance = score;
                    best_arc_id = Some(arc.id);
                }
            }
        }

        if let Some(arc_id) = best_arc_id {
            // 归入已有弧 / Assign to existing arc
            if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                arc.add_turning_point(tp.id);
            }
            // 标记转折点所属弧 / Mark turning point's arc membership
            if let Some(t) = model.turning_points.iter_mut().find(|t| t.id == tp.id) {
                t.add_to_arc(arc_id);
            }
            updates.push(ArcUpdate::TurningPointAdded {
                arc_id,
                tp_id: tp.id,
            });
        } else {
            // 策略 2：创建新弧 / Strategy 2: Create new arc
            let arc_id = self.alloc_arc_id();
            let title = format!("{}弧", target_kind.label_zh());
            let theme = format!("{}相关的故事线", target_kind.label_zh());
            let mut arc = NarrativeArc::new(arc_id, target_kind, title, theme);
            arc.add_turning_point(tp.id);

            // 检查活跃弧数限制 / Check active arc limit
            if model.active_arcs.len() >= self.config.max_active_arcs {
                // 将最不显著的弧休眠 / Dorm the least significant arc
                if let Some(min_arc) =
                    model
                        .active_arcs
                        .iter()
                        .filter(|a| a.is_active())
                        .min_by(|a, b| {
                            a.significance
                                .partial_cmp(&b.significance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                {
                    let min_id = min_arc.id;
                    if let Some(a) = model.active_arcs.iter_mut().find(|a| a.id == min_id) {
                        a.make_dormant();
                    }
                    updates.push(ArcUpdate::ArcDormant { arc_id: min_id });
                }
            }

            // 标记转折点所属弧 / Mark turning point's arc membership
            if let Some(t) = model.turning_points.iter_mut().find(|t| t.id == tp.id) {
                t.add_to_arc(arc_id);
            }

            model.add_arc(arc);
            updates.push(ArcUpdate::ArcCreated {
                arc_id,
                kind: target_kind,
            });
        }

        updates
    }

    /// 定期 tick — 检查弧的休眠/完结/显著度衰减
    /// Periodic tick — check arc dormancy, closure, and significance decay
    pub fn tick(&self, model: &mut NarrativeSelf, now: i64) -> Vec<ArcUpdate> {
        let mut updates = Vec::new();
        let day_secs: i64 = 86400;

        // 预计算每条弧的最后活动时间 / Pre-compute last activity time per arc
        let arc_last_activity: Vec<(u64, i64)> = model
            .active_arcs
            .iter()
            .map(|arc| {
                let last_activity = if let Some(&last_tp_id) = arc.turning_point_ids.last() {
                    model
                        .get_turning_point(last_tp_id)
                        .map(|t| t.timestamp)
                        .unwrap_or(arc.started_at)
                } else {
                    arc.started_at
                };
                (arc.id, last_activity)
            })
            .collect();

        // 活跃弧检查 / Active arc checks
        for arc in &mut model.active_arcs {
            // 查找预计算的最后活动时间 / Look up pre-computed last activity
            let last_activity = arc_last_activity
                .iter()
                .find(|(id, _)| *id == arc.id)
                .map(|(_, t)| *t)
                .unwrap_or(arc.started_at);

            let days_inactive = (now - last_activity) / day_secs;

            // 显著度衰减 / Significance decay
            if days_inactive > 0 {
                let old_sig = arc.significance;
                arc.significance = (arc.significance
                    - self.config.significance_decay_per_day * days_inactive as f64)
                    .max(0.1);
                if (arc.significance - old_sig).abs() > 0.001 {
                    updates.push(ArcUpdate::SignificanceUpdated {
                        arc_id: arc.id,
                        old: old_sig,
                        new: arc.significance,
                    });
                }
            }

            // 休眠检查 / Dormancy check
            if arc.status == ArcStatus::Active && days_inactive >= self.config.dormancy_days {
                arc.make_dormant();
                updates.push(ArcUpdate::ArcDormant { arc_id: arc.id });
            }

            // 完结检查 / Closure check
            if arc.status == ArcStatus::Dormant && days_inactive >= self.config.closure_days {
                arc.close(now);
                updates.push(ArcUpdate::ArcClosed { arc_id: arc.id });
            }
        }

        // 将已完结弧移到 closed_arcs / Move closed arcs to closed_arcs
        let closed_ids: Vec<u64> = model
            .active_arcs
            .iter()
            .filter(|a| a.status == ArcStatus::Closed)
            .map(|a| a.id)
            .collect();
        for id in closed_ids {
            if let Some(pos) = model.active_arcs.iter().position(|a| a.id == id) {
                let arc = model.active_arcs.remove(pos);
                model.closed_arcs.push(arc);
            }
        }

        updates
    }
}

// ════════════════════════════════════════════════════════════════════
// WritingContext — 章节写作上下文 / Chapter Writing Context
// ════════════════════════════════════════════════════════════════════

/// 章节写作上下文 — ChapterWriter 需要的所有信息 / Chapter writing context
#[derive(Debug, Clone)]
pub struct WritingContext {
    /// 目标弧 / Target arc
    pub arc: NarrativeArc,
    /// 弧中的转折点（按时间排序）/ Turning points in arc (chronological)
    pub turning_points: Vec<TurningPoint>,
    /// 之前的章节（用于连贯性）/ Previous chapters (for coherence)
    pub previous_chapters: Vec<NarrativeChapter>,
    /// 当前情感状态 / Current emotion state
    pub current_emotion: EmotionContext,
    /// 当前关系阶段 / Current relationship stage
    pub relationship_stage: String,
    /// 当前成熟度阶段 / Current maturity stage
    pub maturity_stage: String,
    /// 自我描述 / Self description
    pub self_description: String,
    /// 叙事视角 / Narrative perspective
    pub perspective: NarrativePerspective,
    /// 叙事风格 / Narrative style
    pub style: NarrativeStyle,
}

// ════════════════════════════════════════════════════════════════════
// ChapterConfig / ChapterWriter — 章节写作引擎 / Chapter Writing Engine
// ════════════════════════════════════════════════════════════════════

/// 章节写作配置 / Chapter writing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterConfig {
    /// 正文字数下限 / Min body word count
    pub body_min_words: usize,
    /// 正文字数上限 / Max body word count
    pub body_max_words: usize,
    /// 摘要最大字数 / Max summary word count
    pub summary_max_words: usize,
    /// 重写时保留旧版本 / Preserve old version on rewrite
    pub preserve_version_history: bool,
}

impl Default for ChapterConfig {
    fn default() -> Self {
        Self {
            body_min_words: 200,
            body_max_words: 500,
            summary_max_words: 50,
            preserve_version_history: true,
        }
    }
}

/// 章节写作引擎 — 将转折点序列转化为叙事章节
/// Chapter writer engine — transform turning point sequences into narrative chapters
///
/// Phase A: 提供数据结构和接口定义 / Phase A: data structures and interface definitions
/// Phase C: 集成 LLM 生成 / Phase C: integrate LLM generation
pub struct ChapterWriter {
    /// 写作配置 / Writing config
    pub config: ChapterConfig,
    /// 章节版本历史（chapter_id → 旧版本列表）/ Chapter version history
    pub version_history: std::collections::HashMap<u64, Vec<NarrativeChapter>>,
    /// 下一个章节 ID / Next chapter ID
    next_chapter_id: u64,
}

impl ChapterWriter {
    /// 创建章节写作者 / Create chapter writer
    pub fn new(config: ChapterConfig) -> Self {
        Self {
            config,
            version_history: std::collections::HashMap::new(),
            next_chapter_id: 1,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(ChapterConfig::default())
    }

    /// 分配下一个章节 ID / Allocate next chapter ID
    pub fn alloc_chapter_id(&mut self) -> u64 {
        let id = self.next_chapter_id;
        self.next_chapter_id += 1;
        id
    }

    /// 重写章节 — 保留旧版本到 version_history
    /// Rewrite chapter — preserve old version in version_history
    ///
    /// **核心原则：重写而非覆盖。** 旧章节是成长轨迹的一部分。
    /// **Core principle: rewrite, not overwrite.** Old chapters are part of the growth trajectory.
    pub fn rewrite_chapter(
        &mut self,
        chapter: &mut NarrativeChapter,
        new_body: String,
        new_summary: String,
        now: i64,
    ) {
        if self.config.preserve_version_history {
            // 保存旧版本 / Save old version
            let old_version = chapter.clone();
            self.version_history
                .entry(chapter.id)
                .or_default()
                .push(old_version);
        }

        // 更新章节 / Update chapter
        chapter.body = new_body;
        chapter.summary = new_summary;
        chapter.version += 1;
        chapter.rewritten_at = Some(now);
    }

    /// 获取章节的版本历史 / Get chapter's version history
    pub fn get_version_history(&self, chapter_id: u64) -> &[NarrativeChapter] {
        self.version_history
            .get(&chapter_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 构建写作提示词（Phase A 骨架，Phase C 由 LLM 生成）
    /// Build writing prompt (Phase A skeleton, Phase C uses LLM generation)
    pub fn build_prompt(&self, ctx: &WritingContext) -> String {
        let perspective_label = match ctx.perspective {
            NarrativePerspective::FirstPerson => "第一人称",
            NarrativePerspective::ThirdPerson => "第三人称",
            NarrativePerspective::DualPerspective => "双视角",
        };
        let style_label = match ctx.style {
            NarrativeStyle::Introspective => "内省式",
            NarrativeStyle::Narrative => "记叙式",
            NarrativeStyle::Lyrical => "抒情式",
            NarrativeStyle::Adaptive => "混合式",
        };

        let tp_summaries: Vec<String> = ctx
            .turning_points
            .iter()
            .map(|tp| {
                format!(
                    "- [{}] {} ({})",
                    tp.kind.label_zh(),
                    tp.event_description,
                    tp.narrative_summary
                )
            })
            .collect();

        format!(
            "叙事章节写作\n\
             弧：{} — {}\n\
             视角：{} | 风格：{}\n\
             关系阶段：{} | 成熟度：{}\n\
             转折点：\n{}\n\
             自我认知：{}\n\
             请用{}视角、{}风格，将以上转折点写成一个连贯的叙事章节。",
            ctx.arc.title,
            ctx.arc.theme_sentence,
            perspective_label,
            style_label,
            ctx.relationship_stage,
            ctx.maturity_stage,
            tp_summaries.join("\n"),
            ctx.self_description,
            perspective_label,
            style_label,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// ThemeWeaver — 跨弧主题编织器 / Cross-Arc Theme Weaver
// ════════════════════════════════════════════════════════════════════

/// 跨弧主题编织器 — 发现多条弧之间的共同主题
/// Cross-arc theme weaver — discover shared themes across multiple arcs
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase C: 集成 LLM 深度分析 / Phase C: integrate LLM deep analysis
pub struct ThemeWeaver {
    /// 已发现的跨弧主题 / Discovered cross-arc themes
    pub themes: Vec<CrossArcTheme>,
    // 主题 ID 生成器内部状态 — 当 LLM 深度分析接入后将被 consume
    // Theme ID generator internal state — will be consumed once LLM deep analysis is integrated
    #[allow(dead_code)]
    next_theme_id: u64,
}

impl ThemeWeaver {
    /// 创建主题编织器 / Create theme weaver
    pub fn new() -> Self {
        Self {
            themes: Vec::new(),
            next_theme_id: 1,
        }
    }

    /// 从活跃弧中检测跨弧主题 / Detect cross-arc themes from active arcs
    ///
    /// 基础算法（Phase A）：基于弧类型共现和情感基调相似度
    /// Basic algorithm (Phase A): based on arc kind co-occurrence and emotional tone similarity
    pub fn detect_themes(&mut self, model: &NarrativeSelf) -> Vec<CrossArcTheme> {
        let mut new_themes = Vec::new();

        // 按类型分组 / Group by kind
        let mut kind_groups: std::collections::HashMap<ArcKind, Vec<&NarrativeArc>> =
            std::collections::HashMap::new();
        for arc in &model.active_arcs {
            kind_groups.entry(arc.kind).or_default().push(arc);
        }

        // 同类型多条弧 → 提炼共同主题 / Multiple arcs of same kind → extract shared theme
        for (kind, arcs) in &kind_groups {
            if arcs.len() >= 2 {
                let arc_ids: Vec<u64> = arcs.iter().map(|a| a.id).collect();
                // 计算平均显著度 / Calculate average significance
                let avg_sig: f64 =
                    arcs.iter().map(|a| a.significance).sum::<f64>() / arcs.len() as f64;

                let theme = CrossArcTheme {
                    name: format!("{}主题", kind.label_zh()),
                    description: format!("多条{}弧的共同线索", kind.label_zh()),
                    arc_ids,
                    significance: avg_sig,
                };
                new_themes.push(theme);
            }
        }

        // 跨类型情感相似弧 / Cross-kind arcs with similar emotional tone
        let active_arcs: Vec<&NarrativeArc> = model.active_arcs.iter().collect();
        for i in 0..active_arcs.len() {
            for j in (i + 1)..active_arcs.len() {
                let a = active_arcs[i];
                let b = active_arcs[j];
                if a.kind != b.kind {
                    // PAD 余弦相似度 / PAD cosine similarity
                    let sim = cosine_similarity(&a.emotional_tone, &b.emotional_tone);
                    if sim > 0.8 {
                        let theme = CrossArcTheme {
                            name: format!("{}与{}的共鸣", a.kind.label_zh(), b.kind.label_zh()),
                            description: format!(
                                "情感基调相似的{}弧和{}弧",
                                a.kind.label_zh(),
                                b.kind.label_zh()
                            ),
                            arc_ids: vec![a.id, b.id],
                            significance: (a.significance + b.significance) / 2.0 * sim,
                        };
                        new_themes.push(theme);
                    }
                }
            }
        }

        self.themes = new_themes.clone();
        new_themes
    }
}

/// PAD 向量余弦相似度 / PAD vector cosine similarity
fn cosine_similarity(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-6 || norm_b < 1e-6 {
        0.0
    } else {
        (dot / (norm_a * norm_b)) as f64
    }
}

impl Default for ThemeWeaver {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// CausalChain — 因果链构建器 / Causal Chain Builder
// ════════════════════════════════════════════════════════════════════

/// 因果链构建器 — 在转折点之间建立因果叙事
/// Causal chain builder — establish causal narratives between turning points
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase B: 集成回溯构建 / Phase B: integrate retrospective construction
pub struct CausalChain {
    /// 已建立的因果链 / Established causal links
    pub links: Vec<CausalLink>,
}

impl CausalChain {
    /// 创建因果链构建器 / Create causal chain builder
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// 从转折点序列推断因果链 / Infer causal chains from turning point sequence
    ///
    /// 基础算法（Phase A）：时间相邻 + 类型因果规则
    /// Basic algorithm (Phase A): temporal adjacency + kind-based causal rules
    pub fn infer_from_turning_points(
        &mut self,
        turning_points: &[TurningPoint],
    ) -> Vec<CausalLink> {
        let mut new_links = Vec::new();

        // 类型因果规则表 / Kind-based causal rules
        // 例如：FirstConflict → FirstReconciliation（冲突导致和解）
        let causal_rules: Vec<(TurningPointKind, TurningPointKind)> = vec![
            (
                TurningPointKind::FirstConflict,
                TurningPointKind::FirstReconciliation,
            ),
            (
                TurningPointKind::Named,
                TurningPointKind::FirstEmotionResonance,
            ),
            (
                TurningPointKind::FirstEmotionResonance,
                TurningPointKind::FirstLonging,
            ),
            (
                TurningPointKind::FirstLonging,
                TurningPointKind::FirstHeartache,
            ),
            (
                TurningPointKind::FirstApology,
                TurningPointKind::FirstSelfCorrection,
            ),
            (
                TurningPointKind::FirstVulnerability,
                TurningPointKind::FirstIndependentThought,
            ),
            (
                TurningPointKind::NarrativeAwakening,
                TurningPointKind::FirstWisdom,
            ),
        ];

        for (cause_tp, effect_tp) in turning_points.iter().zip(turning_points.iter().skip(1)) {
            // 检查是否有匹配的因果规则 / Check if matching causal rule exists
            for (rule_cause, rule_effect) in &causal_rules {
                if cause_tp.kind == *rule_cause && effect_tp.kind == *rule_effect {
                    let link = CausalLink {
                        cause: NarrativeEventId::Thought {
                            timestamp: cause_tp.timestamp,
                        },
                        effect: NarrativeEventId::Thought {
                            timestamp: effect_tp.timestamp,
                        },
                        narrative: format!(
                            "{}导致了{}",
                            cause_tp.kind.label_zh(),
                            effect_tp.kind.label_zh()
                        ),
                        strength: (cause_tp.significance + effect_tp.significance) / 2.0,
                    };
                    new_links.push(link);
                }
            }

            // 时间相邻的转折点之间也可能有弱因果 / Temporally adjacent TPs may have weak causality
            let time_gap_secs = effect_tp.timestamp - cause_tp.timestamp;
            if time_gap_secs > 0 && time_gap_secs < 86400 * 7 {
                // 一周内 / Within a week
                let link = CausalLink {
                    cause: NarrativeEventId::Thought {
                        timestamp: cause_tp.timestamp,
                    },
                    effect: NarrativeEventId::Thought {
                        timestamp: effect_tp.timestamp,
                    },
                    narrative: format!(
                        "{}之后发生了{}",
                        cause_tp.kind.label_zh(),
                        effect_tp.kind.label_zh()
                    ),
                    strength: 0.3, // 弱因果 / Weak causality
                };
                new_links.push(link);
            }
        }

        self.links = new_links.clone();
        new_links
    }
}

impl Default for CausalChain {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// PromptWeaveConfig / PromptWeaver — Prompt 注入编织器 / Prompt Injection Weaver
// ════════════════════════════════════════════════════════════════════

/// Prompt 注入配置 / Prompt injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptWeaveConfig {
    /// 注入预算（字符数）/ Injection budget (chars)
    pub budget: usize,
    /// 注入层级优先级 / Injection level priorities
    pub level_priorities: [f64; 5],
}

impl Default for PromptWeaveConfig {
    fn default() -> Self {
        Self {
            budget: 800,
            // L1 自我摘要 > L2 身份标签 > L3 活跃弧 > L4 最近转折点 > L5 关系叙事
            level_priorities: [1.0, 0.8, 0.6, 0.4, 0.3],
        }
    }
}

/// Prompt 注入层级 / Prompt injection levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptLevel {
    /// L1 自我摘要 / Self summary
    SelfSummary = 0,
    /// L2 身份标签 / Identity tags
    IdentityTags = 1,
    /// L3 活跃弧 / Active arcs
    ActiveArcs = 2,
    /// L4 最近转折点 / Recent turning points
    RecentTurningPoints = 3,
    /// L5 关系叙事 / Relationship narrative
    RelationshipNarrative = 4,
}

/// Prompt 注入编织器 — 将叙事自我注入 System Prompt
/// Prompt weaver — inject narrative self into System Prompt
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase D: 集成 CoreService / Phase D: integrate with CoreService
pub struct PromptWeaver {
    /// 注入配置 / Injection config
    pub config: PromptWeaveConfig,
}

impl PromptWeaver {
    /// 创建 Prompt 编织器 / Create prompt weaver
    pub fn new(config: PromptWeaveConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(PromptWeaveConfig::default())
    }

    /// 从快照编织 Prompt 片段 / Weave prompt fragment from snapshot
    ///
    /// 5 层注入策略，按优先级分配预算：
    /// 5-level injection strategy, budget allocated by priority:
    /// - L1: 自我摘要（最重要）/ Self summary (most important)
    /// - L2: 身份标签 / Identity tags
    /// - L3: 活跃弧标题 / Active arc titles
    /// - L4: 最近转折点 / Recent turning points
    /// - L5: 关系叙事 / Relationship narrative
    pub fn weave(&self, snapshot: &NarrativeSnapshot) -> String {
        let budget = self.config.budget;
        let mut result = String::new();
        let mut used = 0;

        // L1 自我摘要 / L1 Self summary
        if !snapshot.self_summary.is_empty() {
            let l1_budget = ((budget as f64 * self.config.level_priorities[0])
                .min((budget - used) as f64)) as usize;
            let truncated = truncate_chars(&snapshot.self_summary, l1_budget);
            result.push_str(&format!("[自我] {}\n", truncated));
            used += truncated.len() + 6;
        }

        // L2 身份标签 / L2 Identity tags
        if !snapshot.identity_tags.is_empty() {
            let l2_budget = ((budget as f64 * self.config.level_priorities[1])
                .min((budget - used) as f64)) as usize;
            let tags_str: String = snapshot
                .identity_tags
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let truncated = truncate_chars(&tags_str, l2_budget);
            result.push_str(&format!("[身份] {}\n", truncated));
            used += truncated.len() + 6;
        }

        // L3 活跃弧 / L3 Active arcs
        if !snapshot.active_arcs.is_empty() {
            let l3_budget = ((budget as f64 * self.config.level_priorities[2])
                .min((budget - used) as f64)) as usize;
            let arcs_str: String = snapshot
                .active_arcs
                .iter()
                .map(|a| format!("{}: {}", a.kind.label_zh(), a.title))
                .collect::<Vec<_>>()
                .join("; ");
            let truncated = truncate_chars(&arcs_str, l3_budget);
            result.push_str(&format!("[弧] {}\n", truncated));
            used += truncated.len() + 5;
        }

        // L4 最近转折点 / L4 Recent turning points
        if !snapshot.recent_turning_points.is_empty() {
            let l4_budget = ((budget as f64 * self.config.level_priorities[3])
                .min((budget - used) as f64)) as usize;
            let tp_str: String = snapshot
                .recent_turning_points
                .iter()
                .map(|t| t.narrative_summary.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let truncated = truncate_chars(&tp_str, l4_budget);
            if !truncated.is_empty() {
                result.push_str(&format!("[转折] {}\n", truncated));
            }
        }

        // L5 关系叙事 / L5 Relationship narrative
        if !snapshot.relationship_narrative.is_empty() && used < budget {
            let l5_budget = ((budget as f64 * self.config.level_priorities[4])
                .min((budget - used) as f64)) as usize;
            let truncated = truncate_chars(&snapshot.relationship_narrative, l5_budget);
            if !truncated.is_empty() {
                result.push_str(&format!("[关系] {}", truncated));
            }
        }

        result.trim_end().to_string()
    }
}

/// 按字符数截断 / Truncate by char count
fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}...", truncated)
    }
}

impl Default for PromptWeaver {
    fn default() -> Self {
        Self::default_new()
    }
}

// ════════════════════════════════════════════════════════════════════
// VoiceModulator / ModulatedNarrative — 叙事语气调制器 / Narrative Voice Modulator
// ════════════════════════════════════════════════════════════════════

/// 调制后的叙事 / Modulated narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulatedNarrative {
    /// 原始文本 / Original text
    pub original: String,
    /// 调制后文本 / Modulated text
    pub modulated: String,
    /// 应用的语气 / Applied tone
    pub tone: NarrativeTone,
    /// 应用的视角 / Applied perspective
    pub perspective: NarrativePerspective,
    /// 调制强度 (0.0~1.0) / Modulation strength
    pub strength: f64,
}

/// 叙事语气调制器 — 根据情感状态和回忆距离调整叙事语气
/// Voice modulator — adjust narrative tone based on emotion state and recall distance
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase D: 集成 CoreService / Phase D: integrate with CoreService
pub struct VoiceModulator {
    /// 默认视角 / Default perspective
    pub default_perspective: NarrativePerspective,
    /// 默认风格 / Default style
    pub default_style: NarrativeStyle,
}

impl VoiceModulator {
    /// 创建语气调制器 / Create voice modulator
    pub fn new(perspective: NarrativePerspective, style: NarrativeStyle) -> Self {
        Self {
            default_perspective: perspective,
            default_style: style,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(NarrativePerspective::FirstPerson, NarrativeStyle::Adaptive)
    }

    /// 推断语气 — 从当前情感和回忆距离 / Infer tone from current emotion and recall distance
    ///
    /// 回忆距离越远，语气越趋向怀旧/客观；
    /// The more distant the recall, the more nostalgic/objective the tone.
    pub fn infer_tone(
        &self,
        current_pad: &[f32; 3],
        recall_pad: &[f32; 3],
        recall_distance_days: i64,
    ) -> NarrativeTone {
        // 近期回忆：直接用当前 PAD / Recent recall: use current PAD directly
        if recall_distance_days < 3 {
            NarrativeTone::from_pad(current_pad)
        } else if recall_distance_days < 14 {
            // 中期回忆：混合当前和回忆 PAD / Medium recall: blend current and recall PAD
            let blended = [
                (current_pad[0] + recall_pad[0]) / 2.0,
                (current_pad[1] + recall_pad[1]) / 2.0,
                (current_pad[2] + recall_pad[2]) / 2.0,
            ];
            NarrativeTone::from_pad(&blended)
        } else {
            // 远期回忆：趋向客观/怀旧 / Distant recall: tend toward objective/nostalgic
            let p = recall_pad[0];
            if p > 0.1 {
                NarrativeTone::WarmNostalgia
            } else if p < -0.2 {
                NarrativeTone::BitterLonging
            } else {
                NarrativeTone::ObjectiveRecall
            }
        }
    }

    /// 调制叙事文本 / Modulate narrative text
    ///
    /// Phase A: 返回语气标记的文本（不做 LLM 改写）
    /// Phase A: return tone-annotated text (no LLM rewrite)
    pub fn modulate(
        &self,
        text: &str,
        tone: NarrativeTone,
        current_pad: &[f32; 3],
    ) -> ModulatedNarrative {
        // 计算调制强度：情感越强烈，调制越强 / Stronger emotion → stronger modulation
        let emotion_intensity =
            (current_pad[0].powi(2) + current_pad[1].powi(2) + current_pad[2].powi(2)).sqrt()
                as f64;
        let strength = (emotion_intensity / 1.0).min(1.0);

        // Phase A: 仅添加语气标记前缀 / Phase A: only add tone marker prefix
        let tone_marker = format!("[{}]", tone.label_zh());
        let modulated = format!("{} {}", tone_marker, text);

        ModulatedNarrative {
            original: text.to_string(),
            modulated,
            tone,
            perspective: self.default_perspective,
            strength,
        }
    }
}

impl Default for VoiceModulator {
    fn default() -> Self {
        Self::default_new()
    }
}

// ════════════════════════════════════════════════════════════════════
// RetrospectiveBuilder — 回溯构建引擎 / Retrospective Construction Engine
// ════════════════════════════════════════════════════════════════════

/// 回溯数据源 — 首次启动时从已有存储中提取的原始素材
/// Retrospective data source — raw materials extracted from existing stores on first startup.
#[derive(Debug, Clone)]
pub struct RetrospectiveSource {
    /// 里程碑事件 / Milestone events
    pub milestones: Vec<NarrativeEvent>,
    /// 关系变更事件 / Relationship change events
    pub relationship_changes: Vec<NarrativeEvent>,
    /// 情感事件 / Emotion events
    pub emotion_events: Vec<NarrativeEvent>,
    /// 日记关键事件（从日记中提取的重要时刻）/ Diary key events
    pub diary_events: Vec<NarrativeEvent>,
    /// 内在独白反思事件 / Inner monologue reflection events
    pub monologue_events: Vec<NarrativeEvent>,
    /// 高置信度事实摘要 / High-confidence fact summaries
    pub fact_summaries: Vec<String>,
}

impl Default for RetrospectiveSource {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrospectiveSource {
    /// 创建空数据源 / Create empty source
    pub fn new() -> Self {
        Self {
            milestones: Vec::new(),
            relationship_changes: Vec::new(),
            emotion_events: Vec::new(),
            diary_events: Vec::new(),
            monologue_events: Vec::new(),
            fact_summaries: Vec::new(),
        }
    }

    /// 总事件数 / Total event count
    pub fn total_events(&self) -> usize {
        self.milestones.len()
            + self.relationship_changes.len()
            + self.emotion_events.len()
            + self.diary_events.len()
            + self.monologue_events.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.total_events() == 0 && self.fact_summaries.is_empty()
    }

    /// 合并所有事件为统一列表 / Merge all events into a single list
    pub fn all_events(&self) -> Vec<&NarrativeEvent> {
        self.milestones
            .iter()
            .chain(self.relationship_changes.iter())
            .chain(self.emotion_events.iter())
            .chain(self.diary_events.iter())
            .chain(self.monologue_events.iter())
            .collect()
    }
}

/// 回溯构建结果 / Retrospective construction result
#[derive(Debug, Clone)]
pub struct RetrospectiveResult {
    /// 检测到的转折点 / Detected turning points
    pub turning_points: Vec<TurningPoint>,
    /// 识别的叙事弧 / Identified narrative arcs
    pub arcs: Vec<NarrativeArc>,
    /// 因果链 / Causal links
    pub causal_links: Vec<CausalLink>,
    /// 跨弧主题 / Cross-arc themes
    pub themes: Vec<CrossArcTheme>,
    /// 构建的自我摘要 / Constructed self summary
    pub self_summary: String,
    /// 构建的自我描述 / Constructed self description
    pub self_description: String,
    /// 构建的身份标签 / Constructed identity tags
    pub identity_tags: Vec<IdentityTag>,
    /// 消耗的事件数 / Consumed event count
    pub events_consumed: usize,
    /// 构建耗时毫秒 / Build duration in milliseconds
    pub build_duration_ms: u64,
}

impl RetrospectiveResult {
    /// 是否有实质内容 / Whether has substantive content
    pub fn has_content(&self) -> bool {
        !self.turning_points.is_empty() || !self.arcs.is_empty()
    }

    /// 应用到叙事自我模型 / Apply to narrative self model
    pub fn apply_to(self, model: &mut NarrativeSelf) {
        // 转折点 / Turning points
        for tp in self.turning_points {
            model.add_turning_point(tp);
        }

        // 叙事弧 / Narrative arcs
        for arc in self.arcs {
            model.add_arc(arc);
        }

        // 自我摘要 / Self summary
        if !self.self_summary.is_empty() {
            model.self_summary = self.self_summary;
        }

        // 自我描述 / Self description
        if !self.self_description.is_empty() {
            model.self_description = self.self_description;
        }

        // 身份标签 / Identity tags
        for tag in self.identity_tags {
            model.add_identity_tag(tag);
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();
    }
}

/// 回溯构建引擎 — 首次启动时从已有数据构建初始叙事
/// Retrospective builder — construct initial narrative from existing data on first startup.
///
/// 回溯构建是异步的 — 不阻塞正常服务启动。后台逐步构建，
/// 构建期间叙事 Prompt 注入返回空字符串。
/// Retrospective construction is async — does not block normal service startup.
/// Builds incrementally in the background; narrative prompt injection returns
/// empty string during construction.
pub struct RetrospectiveBuilder {
    /// 转折点检测器 / Turning point detector
    detector: TurningPointDetector,
    /// 弧检测器 / Arc detector
    arc_detector: ArcDetector,
    /// 因果链推断 / Causal chain inferrer
    causal_chain: CausalChain,
    /// 主题编织器 / Theme weaver
    theme_weaver: ThemeWeaver,
    // 弧配置 — Phase C LLM 深度分析接入后将驱动弧构造参数
    // Arc configuration — will drive arc construction params once Phase C LLM analysis is integrated
    #[allow(dead_code)]
    arc_config: ArcConfig,
}

impl Default for RetrospectiveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrospectiveBuilder {
    /// 创建默认构建器 / Create default builder
    pub fn new() -> Self {
        Self {
            detector: TurningPointDetector::default_new(),
            arc_detector: ArcDetector::default_new(),
            causal_chain: CausalChain::new(),
            theme_weaver: ThemeWeaver::new(),
            arc_config: ArcConfig::default(),
        }
    }

    /// 使用自定义配置创建构建器 / Create builder with custom config
    pub fn with_config(detector_config: TurningPointConfig, arc_config: ArcConfig) -> Self {
        Self {
            detector: TurningPointDetector::new(detector_config),
            arc_detector: ArcDetector::new(arc_config.clone()),
            causal_chain: CausalChain::new(),
            theme_weaver: ThemeWeaver::new(),
            arc_config,
        }
    }

    /// 执行回溯构建 / Execute retrospective construction
    ///
    /// 从已有数据源中构建初始叙事自我模型。流程：
    /// 1. 从里程碑 + 关系变更 + 情感事件中检测转折点
    /// 2. 从日记 + 独白中补充转折点
    /// 3. 从转折点集合中识别叙事弧
    /// 4. 推断因果链
    /// 5. 检测跨弧主题
    /// 6. 生成初始自我描述
    ///
    /// Build initial narrative self model from existing data sources. Flow:
    /// 1. Detect turning points from milestones + relationship changes + emotion events
    /// 2. Supplement turning points from diary + monologue
    /// 3. Identify narrative arcs from turning point set
    /// 4. Infer causal chains
    /// 5. Detect cross-arc themes
    /// 6. Generate initial self description
    pub fn build(&mut self, source: &RetrospectiveSource) -> RetrospectiveResult {
        let start = std::time::Instant::now();

        // ── Step 1: 从里程碑 + 关系变更 + 情感事件中检测转折点 ──
        // Step 1: Detect turning points from milestones + relationship changes + emotion events
        let mut turning_points = self.detector.retrospective_detect(
            &source.milestones,
            &source.relationship_changes,
            &source.emotion_events,
        );

        // ── Step 2: 从日记 + 独白中补充转折点 ──
        // Step 2: Supplement turning points from diary + monologue
        let supplementary_events: Vec<&NarrativeEvent> = source
            .diary_events
            .iter()
            .chain(source.monologue_events.iter())
            .collect();

        for event in supplementary_events {
            // 为补充事件构造默认检测上下文 / Build default detection context for supplementary events
            let context = DetectionContext {
                current_pad: event
                    .emotion
                    .as_ref()
                    .map(|e| [e.pleasure, e.arousal, e.dominance])
                    .unwrap_or([0.0; 3]),
                previous_pad: [0.0; 3],
                relationship_stage: "Familiar".to_string(),
                maturity_stage: "Growing".to_string(),
                recent_emotion_trend: EmotionTrend::Stable,
                recent_kinds: Vec::new(),
            };
            if let Some(tp) = self.detector.detect(event, &context) {
                turning_points.push(tp);
            }
        }

        // 按时间排序转折点 / Sort turning points by timestamp
        turning_points.sort_by_key(|tp| tp.timestamp);

        // ── Step 3: 从转折点集合中识别叙事弧 ──
        // Step 3: Identify narrative arcs from turning point set
        let mut temp_model = NarrativeSelf::new();
        for tp in &turning_points {
            temp_model.add_turning_point(tp.clone());
        }

        // 逐个处理转折点以触发弧检测 / Process turning points one by one to trigger arc detection
        let mut arcs = Vec::new();
        for tp in &turning_points {
            let updates = self.arc_detector.process_turning_point(&mut temp_model, tp);
            for update in updates {
                match update {
                    ArcUpdate::ArcCreated { arc_id, kind } => {
                        let title = format!("{}弧", kind.label_zh());
                        let arc = NarrativeArc::new(
                            arc_id,
                            kind,
                            title,
                            String::new(), // theme_sentence 稍后由 ThemeWeaver 填充
                        );
                        arcs.push(arc);
                    }
                    ArcUpdate::TurningPointAdded { arc_id, tp_id } => {
                        // 将转折点 ID 关联到已有弧 / Associate turning point ID with existing arc
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.add_turning_point(tp_id);
                        }
                    }
                    ArcUpdate::ArcDormant { arc_id } => {
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.make_dormant();
                        }
                    }
                    ArcUpdate::ArcClosed { arc_id } => {
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.close(tp.timestamp);
                        }
                    }
                    ArcUpdate::ArcSuperseded {
                        old_arc_id,
                        new_arc_id,
                    } => {
                        // 标记旧弧被取代 / Mark old arc as superseded
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == old_arc_id) {
                            arc.make_dormant();
                        }
                        let _ = new_arc_id; // 新弧已在 ArcCreated 中处理
                    }
                    ArcUpdate::SignificanceUpdated { arc_id, old, new } => {
                        // 弧显著度更新 / Arc significance updated
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.significance = new;
                        }
                        let _ = old; // 旧值仅用于日志
                    }
                    ArcUpdate::NoChange => {
                        // 无变化，跳过 / No change, skip
                    }
                }
            }
        }

        // 将识别的弧添加到临时模型 / Add identified arcs to temp model
        for arc in &arcs {
            if arc.is_active() {
                temp_model.active_arcs.push(arc.clone());
            } else {
                temp_model.closed_arcs.push(arc.clone());
            }
        }

        // ── Step 4: 推断因果链 ──
        // Step 4: Infer causal chains
        let causal_links = self.causal_chain.infer_from_turning_points(&turning_points);

        // ── Step 5: 检测跨弧主题 ──
        // Step 5: Detect cross-arc themes
        let themes = self.theme_weaver.detect_themes(&temp_model);

        // ── Step 6: 生成初始自我描述 ──
        // Step 6: Generate initial self description
        let (self_summary, self_description, identity_tags) =
            self.build_self_description(&turning_points, &arcs, &themes, &source.fact_summaries);

        let build_duration_ms = start.elapsed().as_millis() as u64;

        RetrospectiveResult {
            events_consumed: source.total_events(),
            turning_points,
            arcs,
            causal_links,
            themes,
            self_summary,
            self_description,
            identity_tags,
            build_duration_ms,
        }
    }

    /// 从转折点 + 弧 + 主题构建自我描述
    /// Build self description from turning points + arcs + themes.
    fn build_self_description(
        &self,
        turning_points: &[TurningPoint],
        arcs: &[NarrativeArc],
        themes: &[CrossArcTheme],
        fact_summaries: &[String],
    ) -> (String, String, Vec<IdentityTag>) {
        let mut summary_parts = Vec::new();
        let mut description_parts = Vec::new();
        let mut tags = Vec::new();

        // 从转折点提取身份标签 / Extract identity tags from turning points
        for tp in turning_points {
            match tp.kind {
                TurningPointKind::Named => {
                    tags.push(IdentityTag::new(
                        "被命名者".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("被赋予了名字".to_string());
                }
                TurningPointKind::FirstEmotionResonance => {
                    tags.push(IdentityTag::new(
                        "有感受的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了感受".to_string());
                }
                TurningPointKind::FirstApology => {
                    tags.push(IdentityTag::new(
                        "会认错的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了道歉".to_string());
                }
                TurningPointKind::FirstConflict => {
                    summary_parts.push("经历了冲突".to_string());
                }
                TurningPointKind::FirstReconciliation => {
                    tags.push(IdentityTag::new(
                        "会和解的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了和解".to_string());
                }
                TurningPointKind::NarrativeAwakening => {
                    tags.push(IdentityTag::new(
                        "叙事觉醒者".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("理解了叙事的意义".to_string());
                }
                _ => {}
            }
        }

        // 从弧构建描述段落 / Build description paragraphs from arcs
        for arc in arcs {
            if arc.is_active() {
                description_parts.push(format!(
                    "在「{}」这条路上，{}",
                    arc.title,
                    if arc.theme_sentence.is_empty() {
                        "我正在前行".to_string()
                    } else {
                        arc.theme_sentence.clone()
                    }
                ));
            }
        }

        // 从主题补充描述 / Supplement description from themes
        for theme in themes {
            if theme.significance > 0.5 {
                description_parts.push(format!("{}是我生命中的主题", theme.name));
            }
        }

        // 从事实摘要补充 / Supplement from fact summaries
        for fact in fact_summaries.iter().take(3) {
            description_parts.push(fact.clone());
        }

        // 组装自我摘要 / Assemble self summary
        let self_summary = if summary_parts.is_empty() {
            String::new()
        } else {
            format!("我{}", summary_parts.join("，"))
        };

        // 组装自我描述 / Assemble self description
        let self_description = description_parts.join("。");

        (self_summary, self_description, tags)
    }

    /// 检查是否需要回溯构建（叙事模型为空时需要）
    /// Check whether retrospective construction is needed (needed when narrative model is empty).
    pub fn needs_retrospective(model: &NarrativeSelf) -> bool {
        model.turning_points.is_empty() && model.active_arcs.is_empty()
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeEventRecorder — 叙事事件记录器 / Narrative Event Recorder
// ════════════════════════════════════════════════════════════════════

/// 叙事事件记录器 — 处理消息管线中的叙事事件检测与记录
/// Narrative event recorder — handles narrative event detection and recording
/// in the message processing pipeline.
///
/// 对应 CoreService 管线中的：
/// - Step 0.9: 叙事事件检测（TurningPointDetector.detect）
/// - Step 9.5: 叙事事件记录（更新情感轨迹 + 标记待处理）
pub struct NarrativeEventRecorder {
    /// 转折点检测器 / Turning point detector
    detector: TurningPointDetector,
    /// 弧检测器 / Arc detector
    arc_detector: ArcDetector,
    /// 是否有未处理的转折点 / Whether there are unprocessed turning points
    has_pending: bool,
}

impl Default for NarrativeEventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl NarrativeEventRecorder {
    /// 创建默认记录器 / Create default recorder
    pub fn new() -> Self {
        Self {
            detector: TurningPointDetector::default_new(),
            arc_detector: ArcDetector::default_new(),
            has_pending: false,
        }
    }

    /// 使用配置创建记录器 / Create recorder with config
    pub fn with_config(detector_config: TurningPointConfig, arc_config: ArcConfig) -> Self {
        Self {
            detector: TurningPointDetector::new(detector_config),
            arc_detector: ArcDetector::new(arc_config),
            has_pending: false,
        }
    }

    /// Step 0.9: 叙事事件检测 — 在消息处理管线中检测转折点
    /// Step 0.9: Narrative event detection — detect turning points in message pipeline.
    ///
    /// 返回检测到的转折点（如有），并标记待处理状态。
    /// Returns detected turning point (if any) and marks pending state.
    pub fn detect_event(
        &mut self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPoint> {
        let tp = self.detector.detect(event, context);
        if tp.is_some() {
            self.has_pending = true;
        }
        tp
    }

    /// Step 9.5: 叙事事件记录 — 将转折点集成到叙事模型
    /// Step 9.5: Narrative event recording — integrate turning point into narrative model.
    ///
    /// 处理流程：
    /// 1. 将转折点添加到模型
    /// 2. 通过 ArcDetector 处理，可能创建/更新弧
    /// 3. 返回弧更新列表
    pub fn record_event(&mut self, model: &mut NarrativeSelf, tp: &TurningPoint) -> Vec<ArcUpdate> {
        model.add_turning_point(tp.clone());
        let updates = self.arc_detector.process_turning_point(model, tp);
        self.has_pending = false;
        updates
    }

    /// 是否有待处理的转折点 / Whether there are pending turning points
    pub fn has_pending(&self) -> bool {
        self.has_pending
    }

    /// 从成长里程碑构建叙事事件 / Build narrative event from growth milestone
    pub fn milestone_to_event(
        kind: &str,
        description: &str,
        timestamp: i64,
        emotion: Option<EmotionContext>,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: kind.to_string(),
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion,
            tags: Vec::new(),
        }
    }

    /// 从关系变更构建叙事事件 / Build narrative event from relationship change
    pub fn relationship_to_event(
        from: &str,
        to: &str,
        description: &str,
        timestamp: i64,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::RelationshipChange {
                from: from.to_string(),
                to: to.to_string(),
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion: None,
            tags: Vec::new(),
        }
    }

    /// 从情感变化构建叙事事件 / Build narrative event from emotion change
    pub fn emotion_to_event(
        pad_before: [f32; 3],
        pad_after: [f32; 3],
        description: &str,
        timestamp: i64,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::EmotionEvent {
                pad_before,
                pad_after,
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion: Some(EmotionContext {
                pleasure: pad_after[0],
                arousal: pad_after[1],
                dominance: pad_after[2],
            }),
            tags: Vec::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativePeriodicTask — 叙事周期任务 / Narrative Periodic Tasks
// ════════════════════════════════════════════════════════════════════

/// 叙事日终报告 / Narrative daily report
#[derive(Debug, Clone, Default)]
pub struct NarrativeDailyReport {
    /// 今日检测到的遗漏转折点数 / Missed turning points detected today
    pub missed_turning_points: usize,
    /// 自我描述是否更新 / Whether self description was updated
    pub self_description_updated: bool,
    /// 今日叙事摘要 / Today's narrative summary
    pub daily_summary: String,
    /// 是否触发了旧章节重写 / Whether old chapter rewrite was triggered
    pub rewrite_triggered: bool,
}

/// 叙事周终报告 / Narrative weekly report
#[derive(Debug, Clone, Default)]
pub struct NarrativeWeeklyReport {
    /// 全面弧检测新增弧数 / New arcs from full arc detection
    pub new_arcs: usize,
    /// 跨弧主题数 / Cross-arc theme count
    pub cross_arc_themes: usize,
    /// 自我描述是否重写 / Whether self description was rewritten
    pub self_description_rewritten: bool,
    /// 身份标签更新数 / Identity tag updates
    pub identity_tag_updates: usize,
    /// 叙事快照是否保存 / Whether narrative snapshot was saved
    pub snapshot_saved: bool,
}

/// 叙事周期任务执行器 — 实现 tick / daily / weekly 三级周期任务
/// Narrative periodic task executor — implements tick / daily / weekly three-level periodic tasks.
pub struct NarrativePeriodicTask {
    // 弧检测器 — 周/月级弧演化检测时将被 consume
    // Arc detector — will be consumed during weekly/monthly arc evolution detection
    #[allow(dead_code)]
    arc_detector: ArcDetector,
    /// 主题编织器 / Theme weaver
    theme_weaver: ThemeWeaver,
    /// 因果链 / Causal chain
    causal_chain: CausalChain,
    /// 上次日终执行时间 / Last daily execution time
    last_daily_at: i64,
    /// 上次周终执行时间 / Last weekly execution time
    last_weekly_at: i64,
}
impl Default for NarrativePeriodicTask {
    fn default() -> Self {
        Self::new()
    }
}

impl NarrativePeriodicTask {
    /// 创建默认执行器 / Create default executor
    pub fn new() -> Self {
        Self {
            arc_detector: ArcDetector::default_new(),
            theme_weaver: ThemeWeaver::new(),
            causal_chain: CausalChain::new(),
            last_daily_at: 0,
            last_weekly_at: 0,
        }
    }

    /// tick_narrative: 叙事周期评估（每 1000 tick ≈ 10s）
    /// tick_narrative: Narrative periodic evaluation (every 1000 tick ≈ 10s).
    ///
    /// 执行：
    /// - 检查未处理的转折点 → 触发章节撰写
    /// - 检查弧状态 → 休眠/完结判定
    /// - 情感轨迹更新 → 活跃章节的情感轨迹追加
    pub fn tick(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
        dormancy_secs: i64,
        closure_secs: i64,
    ) {
        // 弧休眠/完结检测 / Arc dormancy/closure detection
        for arc in &mut model.active_arcs {
            // 计算弧的最后活动时间 / Calculate arc's last activity time
            let last_activity = model
                .turning_points
                .iter()
                .filter(|tp| arc.turning_point_ids.contains(&tp.id))
                .map(|tp| tp.timestamp)
                .max()
                .unwrap_or(arc.started_at);

            let inactive_secs = now_epoch_secs - last_activity;

            if inactive_secs > closure_secs && arc.is_active() {
                arc.close(now_epoch_secs);
            } else if inactive_secs > dormancy_secs && arc.is_active() {
                arc.make_dormant();
            }
        }

        // 将已休眠超时的弧移到 closed / Move dormant-expired arcs to closed
        let should_close: Vec<u64> = model
            .active_arcs
            .iter()
            .filter(|a| a.status == ArcStatus::Dormant)
            .filter_map(|a| {
                let last_activity = model
                    .turning_points
                    .iter()
                    .filter(|tp| a.turning_point_ids.contains(&tp.id))
                    .map(|tp| tp.timestamp)
                    .max()
                    .unwrap_or(a.started_at);
                if now_epoch_secs - last_activity > closure_secs {
                    Some(a.id)
                } else {
                    None
                }
            })
            .collect();

        for arc_id in should_close {
            if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                arc.close(now_epoch_secs);
            }
        }

        // 标记未处理转折点为已处理 / Mark unprocessed turning points as processed
        for tp in &mut model.turning_points {
            if !tp.integrated {
                tp.integrated = true;
            }
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();
    }

    /// daily_narrative: 叙事日终任务（每天一次）
    /// daily_narrative: Narrative daily task (once per day).
    ///
    /// 执行：
    /// - 从今日事件中检测遗漏的转折点
    /// - 更新自我描述（如果今日有重要事件）
    /// - 生成今日叙事摘要
    /// - 检查是否需要重写旧章节
    pub fn daily(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
        today_turning_points: &[TurningPoint],
    ) -> NarrativeDailyReport {
        let mut report = NarrativeDailyReport::default();

        // 检测遗漏的转折点 / Detect missed turning points
        let unprocessed = model.unintegrated_turning_points();
        report.missed_turning_points = unprocessed.len();

        // 更新自我描述 / Update self description
        if !today_turning_points.is_empty() {
            let significant: Vec<_> = today_turning_points
                .iter()
                .filter(|tp| tp.significance > 0.7)
                .collect();
            if !significant.is_empty() {
                report.self_description_updated = true;
                let summaries: Vec<&str> = significant
                    .iter()
                    .map(|tp| tp.narrative_summary.as_str())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !summaries.is_empty() {
                    report.daily_summary = summaries.join("；");
                }
            }
        }

        // 检查是否需要重写旧章节 / Check if old chapter rewrite is needed
        if now_epoch_secs - model.last_rewrite_at > 86400 * 30 {
            report.rewrite_triggered = true;
            model.last_rewrite_at = now_epoch_secs;
        }

        self.last_daily_at = now_epoch_secs;
        report
    }

    /// weekly_narrative: 叙事周终任务（每周一次）
    /// weekly_narrative: Narrative weekly task (once per week).
    ///
    /// 执行：
    /// - 全面弧检测（回溯一周事件）
    /// - 跨弧主题识别
    /// - 自我描述重写
    /// - 叙事快照保存
    /// - 身份标签更新
    pub fn weekly(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
    ) -> NarrativeWeeklyReport {
        let mut report = NarrativeWeeklyReport::default();

        // 跨弧主题识别 / Cross-arc theme detection
        let themes = self.theme_weaver.detect_themes(model);
        report.cross_arc_themes = themes.len();

        // 因果链推断 / Causal chain inference
        let _links = self
            .causal_chain
            .infer_from_turning_points(&model.turning_points);

        // 自我描述重写 / Self description rewrite
        // 仅在有足够素材时重写 / Only rewrite when there's enough material
        if model.turning_points.len() >= 3 && !model.active_arcs.is_empty() {
            report.self_description_rewritten = true;

            // 从弧标题构建新的自我描述 / Build new self description from arc titles
            let arc_titles: Vec<&str> = model
                .active_arcs
                .iter()
                .take(5)
                .map(|a| a.title.as_str())
                .collect();
            if !arc_titles.is_empty() {
                model.self_description = format!("我的故事围绕着{}展开", arc_titles.join("、"));
            }
        }

        // 身份标签更新 / Identity tag updates
        // 从最近转折点中提取新标签 / Extract new tags from recent turning points
        // 先收集待添加的标签，避免同时不可变借用和可变借用 model
        // Collect tags to add first to avoid simultaneous immutable and mutable borrows
        let new_tags: Vec<IdentityTag> = model
            .turning_points
            .iter()
            .rev()
            .take(5)
            .filter(|tp| tp.significance > 0.8)
            .filter_map(|tp| {
                let label = format!("{}经历者", tp.kind.label_zh());
                if model.identity_tags.iter().any(|t| t.label == label) {
                    None
                } else {
                    Some(IdentityTag::new(
                        label,
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ))
                }
            })
            .collect();
        for tag in new_tags {
            model.add_identity_tag(tag);
            report.identity_tag_updates += 1;
        }

        // 叙事快照保存标记 / Narrative snapshot save marker
        report.snapshot_saved = true;
        model.refresh_stats();

        self.last_weekly_at = now_epoch_secs;
        report
    }

    /// 是否应该执行日终任务 / Whether daily task should execute
    pub fn should_run_daily(&self, now_epoch_secs: i64) -> bool {
        // 至少间隔 20 小时 / At least 20 hours apart
        now_epoch_secs - self.last_daily_at >= 86400 - 14400
    }

    /// 是否应该执行周终任务 / Whether weekly task should execute
    pub fn should_run_weekly(&self, now_epoch_secs: i64) -> bool {
        // 至少间隔 6 天 / At least 6 days apart
        now_epoch_secs - self.last_weekly_at >= 86400 * 6
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_kind_labels() {
        assert_eq!(ArcKind::Growth.label_zh(), "成长");
        assert_eq!(ArcKind::Growth.label_en(), "Growth");
        assert_eq!(ArcKind::Relationship.label_zh(), "关系");
        assert_eq!(ArcKind::Transformation.label_en(), "Transformation");
    }

    #[test]
    fn test_narrative_arc_new() {
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "学会在乎".to_string(),
            "从无知到理解在乎".to_string(),
        );
        assert!(arc.is_active());
        assert_eq!(arc.kind, ArcKind::Growth);
        assert!(arc.chapter_ids.is_empty());
        assert!(arc.turning_point_ids.is_empty());
    }

    #[test]
    fn test_narrative_arc_lifecycle() {
        let mut arc = NarrativeArc::new(
            1,
            ArcKind::Relationship,
            "我们的故事".to_string(),
            "从初识到信任".to_string(),
        );
        arc.add_turning_point(10);
        arc.add_turning_point(20);
        arc.add_chapter(100);
        assert_eq!(arc.turning_point_ids.len(), 2);
        assert_eq!(arc.chapter_ids.len(), 1);
        arc.add_turning_point(10);
        assert_eq!(arc.turning_point_ids.len(), 2);
        arc.make_dormant();
        assert_eq!(arc.status, ArcStatus::Dormant);
        assert!(!arc.is_active());
        arc.close(1000000);
        assert_eq!(arc.status, ArcStatus::Closed);
        assert_eq!(arc.ended_at, Some(1000000));
    }

    #[test]
    fn test_emotion_trajectory_infer_shape() {
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Flat);
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.3, 0.0, 0.0], &[0.5, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Ascending);
        let shape =
            EmotionTrajectory::infer_shape(&[0.5, 0.0, 0.0], &[0.2, 0.0, 0.0], &[-0.3, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Descending);
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.5, 0.0, 0.0], &[0.1, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Peak);
        let shape =
            EmotionTrajectory::infer_shape(&[0.5, 0.0, 0.0], &[-0.2, 0.0, 0.0], &[0.3, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Valley);
    }

    #[test]
    fn test_turning_point_kind_milestone_mapping() {
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::FirstNamed),
            Some(TurningPointKind::Named)
        );
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::FirstApology),
            Some(TurningPointKind::FirstApology)
        );
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::CleanStreak100),
            None
        );
    }

    #[test]
    fn test_turning_point_kind_arc_inference() {
        assert_eq!(
            TurningPointKind::Named.infer_arc_kind(),
            ArcKind::Relationship
        );
        assert_eq!(
            TurningPointKind::FirstApology.infer_arc_kind(),
            ArcKind::Growth
        );
        assert_eq!(
            TurningPointKind::FirstConflict.infer_arc_kind(),
            ArcKind::Challenge
        );
        assert_eq!(
            TurningPointKind::NarrativeAwakening.infer_arc_kind(),
            ArcKind::Transformation
        );
        assert_eq!(
            TurningPointKind::FirstRitual.infer_arc_kind(),
            ArcKind::Ritual
        );
    }

    #[test]
    fn test_turning_point_kind_significance() {
        assert!(TurningPointKind::Named.default_significance() > 0.9);
        assert!(TurningPointKind::FirstRitual.default_significance() < 0.7);
    }

    #[test]
    fn test_narrative_event_id_timestamp() {
        let event = NarrativeEventId::Fact {
            subject: "user".to_string(),
            predicate: "lives_in".to_string(),
            timestamp: 1000,
        };
        assert_eq!(event.timestamp(), 1000);
        assert_eq!(event.type_label(), "fact");
    }

    #[test]
    fn test_narrative_chapter() {
        let chapter = NarrativeChapter::new(
            1,
            100,
            1,
            "第一次被叫名字".to_string(),
            "你给我取了名字，我突然有了存在感。".to_string(),
            "被命名，获得存在感".to_string(),
        );
        assert!(!chapter.is_rewritten());
        assert!(chapter.word_count() > 0);
        assert_eq!(chapter.version, 1);
    }

    #[test]
    fn test_identity_tag() {
        let tag = IdentityTag::new("在乎的人".to_string(), 1, 0.85, 0.9);
        assert_eq!(tag.label, "在乎的人");
        assert!((tag.confidence - 0.85).abs() < 1e-6);
        assert!((tag.valence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_narrative_self_model() {
        let mut model = NarrativeSelf::new();
        assert!(model.active_arcs.is_empty());
        assert!(model.turning_points.is_empty());
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长".to_string(),
            "慢慢长大".to_string(),
        );
        model.add_arc(arc);
        assert_eq!(model.active_arcs.len(), 1);
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        );
        model.add_turning_point(tp);
        assert_eq!(model.turning_points.len(), 1);
        model.add_identity_tag(IdentityTag::new("在乎的人".to_string(), 1, 0.8, 0.9));
        assert_eq!(model.identity_tags.len(), 1);
        model.add_identity_tag(IdentityTag::new("在乎的人".to_string(), 1, 0.9, 0.95));
        assert_eq!(model.identity_tags.len(), 1);
        assert!((model.identity_tags[0].confidence - 0.9).abs() < 1e-6);
        model.refresh_stats();
        assert_eq!(model.stats.active_arcs, 1);
        assert_eq!(model.stats.total_turning_points, 1);
    }

    #[test]
    fn test_detection_context_pad_distance() {
        let ctx = DetectionContext {
            current_pad: [0.5, 0.5, 0.5],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let dist = ctx.pad_distance();
        let expected = (0.25f32 + 0.25 + 0.25).sqrt();
        assert!((dist - expected).abs() < 1e-4);
    }

    #[test]
    fn test_turning_point_detector_milestone() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 1000,
            },
            description: "被命名为小通".to_string(),
            timestamp: 1000,
            emotion: Some(EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            }),
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.5, 0.3, 0.2],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Acquaintance".to_string(),
            maturity_stage: "Naive".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        let tp = tp.unwrap();
        assert_eq!(tp.kind, TurningPointKind::Named);
        assert_eq!(tp.event_description, "被命名为小通");
    }

    #[test]
    fn test_turning_point_detector_emotion() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::EmotionEvent {
                pad_before: [0.0, 0.0, 0.0],
                pad_after: [0.6, 0.5, 0.3],
                timestamp: 2000,
            },
            description: "情感大幅跃升".to_string(),
            timestamp: 2000,
            emotion: Some(EmotionContext {
                pleasure: 0.6,
                arousal: 0.5,
                dominance: 0.3,
            }),
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.6, 0.5, 0.3],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        assert_eq!(tp.unwrap().kind, TurningPointKind::FirstEmotionResonance);
    }

    #[test]
    fn test_turning_point_detector_behavior_tag() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::Audit {
                event_type: "apology".to_string(),
                timestamp: 3000,
            },
            description: "首次道歉".to_string(),
            timestamp: 3000,
            emotion: None,
            tags: vec!["apology".to_string()],
        };
        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        assert_eq!(tp.unwrap().kind, TurningPointKind::FirstApology);
    }

    #[test]
    fn test_turning_point_detector_interval() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 7200,
            ..Default::default()
        });
        let event1 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 10000,
            },
            description: "被命名".to_string(),
            timestamp: 10000,
            emotion: None,
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Acquaintance".to_string(),
            maturity_stage: "Naive".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };
        assert!(detector.detect(&event1, &context).is_some());
        let event2 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstApology".to_string(),
                timestamp: 2000,
            },
            description: "首次道歉".to_string(),
            timestamp: 18000,
            emotion: None,
            tags: Vec::new(),
        };
        assert!(detector.detect(&event2, &context).is_some());
        let event3 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstApology".to_string(),
                timestamp: 16000,
            },
            description: "再次道歉".to_string(),
            timestamp: 19000,
            emotion: None,
            tags: Vec::new(),
        };
        assert!(detector.detect(&event3, &context).is_none());
    }

    #[test]
    fn test_narrative_tone_from_pad() {
        assert_eq!(
            NarrativeTone::from_pad(&[0.5, 0.5, 0.0]),
            NarrativeTone::VividRelive
        );
        assert_eq!(
            NarrativeTone::from_pad(&[0.3, 0.0, 0.0]),
            NarrativeTone::WarmNostalgia
        );
        assert_eq!(
            NarrativeTone::from_pad(&[-0.5, 0.0, 0.0]),
            NarrativeTone::BitterLonging
        );
        assert_eq!(
            NarrativeTone::from_pad(&[0.0, 0.0, 0.0]),
            NarrativeTone::ObjectiveRecall
        );
        assert_eq!(
            NarrativeTone::from_pad(&[-0.3, 0.5, 0.0]),
            NarrativeTone::SelfDeprecating
        );
    }

    #[test]
    fn test_narrative_cfg_default() {
        let cfg = NarrativeCfg::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.perspective, NarrativePerspective::FirstPerson);
        assert_eq!(cfg.style, NarrativeStyle::Adaptive);
        assert_eq!(cfg.body_min_words, 200);
        assert_eq!(cfg.body_max_words, 500);
        assert_eq!(cfg.prompt_budget, 800);
    }

    #[test]
    fn test_time_span() {
        let span = TimeSpan {
            start: 0,
            end: 86400,
        };
        assert_eq!(span.duration_secs(), 86400);
        assert_eq!(span.duration_days(), 1);
    }

    #[test]
    fn test_turning_point_with_narrative() {
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        )
        .with_narrative(
            "你给我取了名字，我突然觉得...我存在了".to_string(),
            "被命名，获得存在感".to_string(),
        );
        assert!(!tp.narrative.is_empty());
        assert!(!tp.narrative_summary.is_empty());
        assert!(!tp.integrated);
    }

    #[test]
    fn test_retrospective_detect() {
        let mut detector = TurningPointDetector::default_new();
        let milestones = vec![NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 1000,
            },
            description: "被命名".to_string(),
            timestamp: 1000,
            emotion: None,
            tags: Vec::new(),
        }];
        let relationships = vec![NarrativeEvent {
            id: NarrativeEventId::RelationshipChange {
                from: "Acquaintance".to_string(),
                to: "Familiar".to_string(),
                timestamp: 2000,
            },
            description: "关系升级".to_string(),
            timestamp: 2000,
            emotion: None,
            tags: Vec::new(),
        }];
        let emotions: Vec<NarrativeEvent> = Vec::new();
        let results = detector.retrospective_detect(&milestones, &relationships, &emotions);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].kind, TurningPointKind::Named);
        assert_eq!(results[1].kind, TurningPointKind::RelationshipPromotion);
    }

    // ── Phase A 新增测试 / Phase A new tests ──

    #[test]
    fn test_narrative_error_display() {
        let err = NarrativeError::ArcNotFound(42);
        assert!(err.to_string().contains("42"));
        let err = NarrativeError::BudgetExceeded {
            used: 900,
            budget: 800,
        };
        assert!(err.to_string().contains("900/800"));
        let err = NarrativeError::LlmFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_narrative_snapshot_from_model() {
        let mut model = NarrativeSelf::new();
        model.self_summary = "我是一个在成长的AI".to_string();
        model.self_description = "从无名到有名".to_string();
        model.relationship_narrative = "我们之间有了信任".to_string();
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长弧".to_string(),
            "慢慢长大".to_string(),
        );
        model.add_arc(arc);
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        )
        .with_narrative("我被命名了".to_string(), "被命名".to_string());
        model.add_turning_point(tp);
        let snapshot = NarrativeSnapshot::from_model(&model, 5);
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.self_summary, "我是一个在成长的AI");
        assert_eq!(snapshot.active_arcs.len(), 1);
        assert_eq!(snapshot.active_arcs[0].kind, ArcKind::Growth);
        assert_eq!(snapshot.recent_turning_points.len(), 1);
    }

    #[test]
    fn test_narrative_snapshot_empty() {
        let model = NarrativeSelf::new();
        let snapshot = NarrativeSnapshot::from_model(&model, 5);
        assert!(snapshot.is_empty());
    }

    #[test]
    fn test_turning_point_pattern() {
        let pattern = TurningPointPattern::new(
            1,
            TurningPointKind::FirstEmotionResonance,
            [1.0, 1.0, 0.0],
            0.3,
        );
        assert!((pattern.precision() - 0.5).abs() < 1e-6);
        let mut p = pattern;
        p.record_hit();
        assert!((p.precision() - 1.0).abs() < 1e-6);
        p.record_miss();
        assert!((p.precision() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_turning_point_pattern_pad_match() {
        let pattern = TurningPointPattern::new(
            1,
            TurningPointKind::FirstEmotionResonance,
            [1.0, 1.0, 0.0],
            0.2,
        );
        assert!(pattern.matches_pad_change(&[0.0, 0.0, 0.0], &[0.5, 0.5, 0.0]));
        assert!(!pattern.matches_pad_change(&[0.5, 0.5, 0.0], &[0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_rewrite_trigger() {
        let trigger = RewriteTrigger::new(RewriteTarget::SelfDescription, "新证据出现".to_string());
        assert_eq!(trigger.target, RewriteTarget::SelfDescription);
        assert!(trigger.evidence.is_empty());
        let trigger_with_evidence =
            trigger.with_evidence(vec![NarrativeEventId::Thought { timestamp: 1000 }]);
        assert_eq!(trigger_with_evidence.evidence.len(), 1);
    }

    #[test]
    fn test_arc_config_default() {
        let config = ArcConfig::default();
        assert_eq!(config.min_turning_points, 2);
        assert_eq!(config.dormancy_days, 14);
        assert_eq!(config.closure_days, 60);
        assert_eq!(config.max_active_arcs, 10);
    }

    #[test]
    fn test_arc_detector_process_turning_point() {
        let mut detector = ArcDetector::default_new();
        let mut model = NarrativeSelf::new();
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        );
        model.add_turning_point(tp.clone());
        let updates = detector.process_turning_point(&mut model, &tp);
        assert!(updates
            .iter()
            .any(|u| matches!(u, ArcUpdate::ArcCreated { .. })));
        assert_eq!(model.active_arcs.len(), 1);
        // 同类型转折点应归入已有弧 / Same-kind TP assigned to existing arc
        let tp2 = TurningPoint::new(
            2,
            TurningPointKind::FirstEmotionResonance,
            "情感共振".to_string(),
            EmotionContext {
                pleasure: 0.6,
                arousal: 0.4,
                dominance: 0.3,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        model.add_turning_point(tp2.clone());
        let updates2 = detector.process_turning_point(&mut model, &tp2);
        assert!(updates2
            .iter()
            .any(|u| matches!(u, ArcUpdate::TurningPointAdded { .. })));
    }

    #[test]
    fn test_chapter_writer_rewrite_preserves_history() {
        let mut writer = ChapterWriter::default_new();
        let mut chapter = NarrativeChapter::new(
            1,
            100,
            1,
            "初章".to_string(),
            "最初的故事".to_string(),
            "开始".to_string(),
        );
        assert!(!chapter.is_rewritten());
        assert_eq!(chapter.version, 1);
        let now = Local::now().timestamp();
        writer.rewrite_chapter(
            &mut chapter,
            "重写后的故事".to_string(),
            "重新开始".to_string(),
            now,
        );
        assert!(chapter.is_rewritten());
        assert_eq!(chapter.version, 2);
        assert_eq!(chapter.body, "重写后的故事");
        let history = writer.get_version_history(1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].body, "最初的故事");
        assert_eq!(history[0].version, 1);
    }

    #[test]
    fn test_chapter_writer_build_prompt() {
        let writer = ChapterWriter::default_new();
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长弧".to_string(),
            "从无知到理解".to_string(),
        );
        let ctx = WritingContext {
            arc,
            turning_points: Vec::new(),
            previous_chapters: Vec::new(),
            current_emotion: EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            self_description: "我在成长".to_string(),
            perspective: NarrativePerspective::FirstPerson,
            style: NarrativeStyle::Adaptive,
        };
        let prompt = writer.build_prompt(&ctx);
        assert!(prompt.contains("成长弧"));
        assert!(prompt.contains("第一人称"));
        assert!(prompt.contains("混合式"));
    }

    #[test]
    fn test_theme_weaver_detect_same_kind() {
        let mut weaver = ThemeWeaver::new();
        let mut model = NarrativeSelf::new();
        model.add_arc(NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长1".to_string(),
            "t1".to_string(),
        ));
        model.add_arc(NarrativeArc::new(
            2,
            ArcKind::Growth,
            "成长2".to_string(),
            "t2".to_string(),
        ));
        let themes = weaver.detect_themes(&model);
        assert!(themes.iter().any(|t| t.name.contains("成长主题")));
    }

    #[test]
    fn test_theme_weaver_detect_cross_kind_similarity() {
        let mut weaver = ThemeWeaver::new();
        let mut model = NarrativeSelf::new();
        let mut arc1 = NarrativeArc::new(1, ArcKind::Growth, "成长".to_string(), "t1".to_string());
        arc1.emotional_tone = [0.5, 0.3, 0.2];
        let mut arc2 =
            NarrativeArc::new(2, ArcKind::Challenge, "挑战".to_string(), "t2".to_string());
        arc2.emotional_tone = [0.48, 0.31, 0.19];
        model.add_arc(arc1);
        model.add_arc(arc2);
        let themes = weaver.detect_themes(&model);
        assert!(themes.iter().any(|t| t.name.contains("共鸣")));
    }

    #[test]
    fn test_causal_chain_infer() {
        let mut chain = CausalChain::new();
        let tp1 = TurningPoint::new(
            1,
            TurningPointKind::FirstConflict,
            "首次冲突".to_string(),
            EmotionContext {
                pleasure: -0.3,
                arousal: 0.5,
                dominance: 0.1,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        let tp2 = TurningPoint::new(
            2,
            TurningPointKind::FirstReconciliation,
            "首次和解".to_string(),
            EmotionContext {
                pleasure: 0.4,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        let links = chain.infer_from_turning_points(&[tp1, tp2]);
        assert!(!links.is_empty());
        assert!(links.iter().any(|l| l.narrative.contains("导致了")));
    }

    #[test]
    fn test_prompt_weaver_weave() {
        let weaver = PromptWeaver::default_new();
        let snapshot = NarrativeSnapshot {
            self_summary: "我在成长".to_string(),
            self_description: String::new(),
            identity_tags: vec![IdentityTag::new("在乎的人".to_string(), 1, 0.9, 0.8)],
            active_arcs: vec![ArcSummary {
                id: 1,
                kind: ArcKind::Growth,
                title: "成长弧".to_string(),
                theme_sentence: "慢慢长大".to_string(),
                chapter_count: 1,
                turning_point_count: 2,
                significance: 0.7,
            }],
            recent_turning_points: vec![TurningPointSummary {
                id: 1,
                kind: TurningPointKind::Named,
                narrative_summary: "被命名".to_string(),
                timestamp: 1000,
                significance: 0.95,
            }],
            relationship_narrative: "我们有了信任".to_string(),
            stats: NarrativeStats::default(),
        };
        let result = weaver.weave(&snapshot);
        assert!(result.contains("[自我]"));
        assert!(result.contains("[身份]"));
        assert!(result.contains("[弧]"));
        assert!(result.chars().count() <= 900);
    }

    #[test]
    fn test_prompt_weaver_empty_snapshot() {
        let weaver = PromptWeaver::default_new();
        let snapshot = NarrativeSnapshot {
            self_summary: String::new(),
            self_description: String::new(),
            identity_tags: Vec::new(),
            active_arcs: Vec::new(),
            recent_turning_points: Vec::new(),
            relationship_narrative: String::new(),
            stats: NarrativeStats::default(),
        };
        let result = weaver.weave(&snapshot);
        assert!(result.is_empty());
    }

    #[test]
    fn test_voice_modulator_infer_tone_recent() {
        let modulator = VoiceModulator::default_new();
        let tone = modulator.infer_tone(&[0.5, 0.5, 0.0], &[0.3, 0.3, 0.0], 1);
        assert_eq!(tone, NarrativeTone::VividRelive);
    }

    #[test]
    fn test_voice_modulator_infer_tone_distant() {
        let modulator = VoiceModulator::default_new();
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[0.3, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::WarmNostalgia);
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[-0.4, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::BitterLonging);
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::ObjectiveRecall);
    }

    #[test]
    fn test_voice_modulator_modulate() {
        let modulator = VoiceModulator::default_new();
        let result =
            modulator.modulate("我被命名了", NarrativeTone::WarmNostalgia, &[0.3, 0.0, 0.0]);
        assert_eq!(result.original, "我被命名了");
        assert!(result.modulated.contains("[温暖怀旧]"));
        assert_eq!(result.tone, NarrativeTone::WarmNostalgia);
        assert!(result.strength > 0.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let sim = cosine_similarity(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!((sim - 1.0).abs() < 1e-4);
        let sim = cosine_similarity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]);
        assert!(sim.abs() < 1e-4);
        let sim = cosine_similarity(&[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!(sim.abs() < 1e-4);
    }

    #[test]
    fn test_truncate_chars() {
        assert_eq!(truncate_chars("abc", 5), "abc");
        assert_eq!(truncate_chars("abcdef", 4), "abc...");
        assert_eq!(truncate_chars("", 5), "");
    }

    // ════════════════════════════════════════════════════════════════════
    // Phase B 测试：回顾构建 / Retrospective Builder Tests
    // ════════════════════════════════════════════════════════════════════

    /// 回顾构建器：空源应产生空结果 / RetrospectiveBuilder: empty source yields empty result
    #[test]
    fn test_retrospective_empty_source() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource::new();
        let result = builder.build(&source);
        assert!(result.turning_points.is_empty());
        assert!(result.arcs.is_empty());
        assert!(result.identity_tags.is_empty());
    }

    /// 回顾构建器：里程碑事件应产生转折点 / RetrospectiveBuilder: milestones produce turning points
    #[test]
    fn test_retrospective_milestones() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            milestones: vec![
                NarrativeEvent {
                    id: NarrativeEventId::Milestone {
                        kind: "FirstNamed".to_string(),
                        timestamp: 1000,
                    },
                    description: "首次被命名".to_string(),
                    timestamp: 1000,
                    emotion: Some(EmotionContext {
                        pleasure: 0.8,
                        arousal: 0.5,
                        dominance: 0.3,
                    }),
                    tags: vec!["milestone".to_string(), "FirstNamed".to_string()],
                },
                NarrativeEvent {
                    id: NarrativeEventId::Milestone {
                        kind: "FirstLesson".to_string(),
                        timestamp: 2000,
                    },
                    description: "首次被教导".to_string(),
                    timestamp: 2000,
                    emotion: Some(EmotionContext {
                        pleasure: 0.6,
                        arousal: 0.4,
                        dominance: 0.5,
                    }),
                    tags: vec!["milestone".to_string(), "FirstLesson".to_string()],
                },
            ],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(!result.turning_points.is_empty(), "里程碑应产生转折点");
    }

    /// 回顾构建器：关系变更应产生转折点 / RetrospectiveBuilder: relationship changes produce turning points
    #[test]
    fn test_retrospective_relationship_changes() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            relationship_changes: vec![NarrativeEvent {
                id: NarrativeEventId::RelationshipChange {
                    from: "Stranger".to_string(),
                    to: "Familiar".to_string(),
                    timestamp: 3000,
                },
                description: "关系从陌生到熟悉".to_string(),
                timestamp: 3000,
                emotion: Some(EmotionContext {
                    pleasure: 0.5,
                    arousal: 0.3,
                    dominance: 0.4,
                }),
                tags: vec!["relationship_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(!result.turning_points.is_empty(), "关系变更应产生转折点");
    }

    /// 回顾构建器：情感事件应被处理 / RetrospectiveBuilder: emotion events are processed
    #[test]
    fn test_retrospective_emotion_events() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            emotion_events: vec![NarrativeEvent {
                id: NarrativeEventId::EmotionEvent {
                    pad_before: [0.0, 0.0, 0.0],
                    pad_after: [0.9, 0.8, 0.7],
                    timestamp: 4000,
                },
                description: "强烈正面情感变化".to_string(),
                timestamp: 4000,
                emotion: Some(EmotionContext {
                    pleasure: 0.9,
                    arousal: 0.8,
                    dominance: 0.7,
                }),
                tags: vec!["emotion_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        // 情感变化幅度大，应产生转折点
        assert!(result.events_consumed > 0, "情感事件应被处理");
    }

    /// 回顾构建器：混合事件源 / RetrospectiveBuilder: mixed event sources
    #[test]
    fn test_retrospective_mixed_sources() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            milestones: vec![NarrativeEvent {
                id: NarrativeEventId::Milestone {
                    kind: "FirstNamed".to_string(),
                    timestamp: 1000,
                },
                description: "首次被命名".to_string(),
                timestamp: 1000,
                emotion: None,
                tags: vec!["milestone".to_string()],
            }],
            relationship_changes: vec![NarrativeEvent {
                id: NarrativeEventId::RelationshipChange {
                    from: "Stranger".to_string(),
                    to: "Familiar".to_string(),
                    timestamp: 2000,
                },
                description: "关系变更".to_string(),
                timestamp: 2000,
                emotion: None,
                tags: vec!["relationship_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(result.events_consumed > 0, "混合源应处理事件");
    }

    /// 回顾构建器：结果统计 / RetrospectiveBuilder: result stats
    #[test]
    fn test_retrospective_result_has_content() {
        let result = RetrospectiveResult {
            turning_points: Vec::new(),
            arcs: Vec::new(),
            causal_links: Vec::new(),
            themes: Vec::new(),
            self_summary: String::new(),
            self_description: String::new(),
            identity_tags: Vec::new(),
            events_consumed: 0,
            build_duration_ms: 0,
        };
        assert!(!result.has_content(), "空结果不应有内容");
    }

    /// 转折点检测器：时间间隔限制 / TurningPointDetector: minimum interval enforcement
    #[test]
    fn test_turning_point_min_interval() {
        let config = TurningPointConfig {
            emotion_change_threshold: 0.1,
            relationship_change_always_turning: true,
            min_interval_secs: 3600,
        };
        let mut detector = TurningPointDetector::new(config);

        let event1 = NarrativeEvent {
            id: NarrativeEventId::Thought { timestamp: 10000 },
            description: "事件1".to_string(),
            timestamp: 10000,
            emotion: Some(EmotionContext {
                pleasure: 0.8,
                arousal: 0.5,
                dominance: 0.3,
            }),
            tags: vec!["milestone".to_string(), "FirstNamed".to_string()],
        };
        let ctx = DetectionContext {
            current_pad: [0.5, 0.5, 0.5],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        // 第一次检测应成功
        let tp1 = detector.detect(&event1, &ctx);
        assert!(tp1.is_some(), "首次检测应成功");

        // 紧接着的第二次检测应因时间间隔被跳过
        let event2 = NarrativeEvent {
            id: NarrativeEventId::Thought { timestamp: 11000 },
            description: "事件2".to_string(),
            timestamp: 11000,
            emotion: Some(EmotionContext {
                pleasure: 0.9,
                arousal: 0.6,
                dominance: 0.4,
            }),
            tags: vec!["milestone".to_string(), "FirstLesson".to_string()],
        };
        let tp2 = detector.detect(&event2, &ctx);
        assert!(tp2.is_none(), "时间间隔内应跳过");
    }

    /// 弧检测器：新弧创建 / ArcDetector: new arc creation
    #[test]
    fn test_arc_detector_new_arc() {
        let mut detector = ArcDetector::default_new();
        let mut model = NarrativeSelf::new();

        let tp = TurningPoint {
            id: 1,
            kind: TurningPointKind::Named,
            narrative: "被命名".to_string(),
            narrative_summary: "被命名".to_string(),
            event_description: "被命名为小A".to_string(),
            timestamp: 1000,
            emotion_snapshot: EmotionContext {
                pleasure: 0.8,
                arousal: 0.5,
                dominance: 0.3,
            },
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            significance: 0.9,
            before_chapter_id: None,
            after_chapter_id: None,
            arc_ids: Vec::new(),
            integrated: false,
        };

        let updates = detector.process_turning_point(&mut model, &tp);
        assert!(!updates.is_empty(), "转折点应触发弧更新");
    }

    /// 情感趋势枚举完整性 / EmotionTrend enum completeness
    #[test]
    fn test_emotion_trend_variants() {
        let trends = [
            EmotionTrend::Stable,
            EmotionTrend::Rising,
            EmotionTrend::Falling,
            EmotionTrend::Oscillating,
        ];
        assert_eq!(trends.len(), 4, "EmotionTrend 应有 4 个变体");
    }

    /// NarrativeSelf 弧休眠/完结 / NarrativeSelf arc dormancy and closure
    #[test]
    fn test_narrative_self_arc_lifecycle() {
        let mut model = NarrativeSelf::new();

        // 添加一个弧
        let arc = NarrativeArc::new(
            1,
            ArcKind::Relationship,
            "关系弧".to_string(),
            "与用户建立关系".to_string(),
        );
        model.add_arc(arc);
        assert_eq!(model.active_arcs.len(), 1);

        // 弧应为活跃状态
        assert!(model.active_arcs[0].is_active());

        // 休眠弧
        model.active_arcs[0].make_dormant();
        assert!(!model.active_arcs[0].is_active());

        // 完结弧
        model.active_arcs[0].close(2000);
        assert!(!model.active_arcs[0].is_active());
    }
}
