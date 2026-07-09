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

use super::*;
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
    /// 情景记忆事件 / Episodic memory event
    ///
    /// P2-B：引用 EpisodicMemoryStore 中的具体情景记录——
    /// 数字生命叙事章节可引用"那一刻"的具体经历，让自传体叙事有血有肉。
    ///
    /// P2-B: references a concrete episode record in EpisodicMemoryStore —
    /// digital life's narrative chapters can reference "that moment"'s concrete
    /// experience, giving autobiographical narrative flesh and blood.
    Episodic {
        /// 情景记录 ID / Episode ID
        episode_id: String,
        /// 事件时间戳 / Event timestamp
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
            Self::Episodic { timestamp, .. } => *timestamp,
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
            Self::Episodic { .. } => "episodic",
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
