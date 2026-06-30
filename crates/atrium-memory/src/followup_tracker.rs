// SPDX-License-Identifier: MIT
#![allow(dead_code)]

//! 追问追踪系统 — Follow-up tracker with extraction, decay, recall, trigger, expression, and reaction.
//!
//! 包含 ExtractPipeline（提取管线）、DecayEngine（非线性衰减）、RecallEngine（自发回忆）、
//! TriggerJudge（触发判决）、ExpressionWeaver（表达编织）、ReactionAnalyzer（反应分析）、
//! FollowUpStore（持久化存储）以及 FollowUpTracker（顶层组合）。
//! Includes ExtractPipeline, DecayEngine, RecallEngine, TriggerJudge,
//! ExpressionWeaver, ReactionAnalyzer, FollowUpStore, and FollowUpTracker.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

// ═══════════════════════════════════════════════════════════════════════════
// 1. 数据结构 — Data Structures
// ═══════════════════════════════════════════════════════════════════════════

/// 追问类别 — 7 种事项类别
/// Follow-up category — 7 types of follow-up items.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FollowUpCategory {
    /// 计划 / Plan
    Plan,
    /// 担忧 / Worry
    Worry,
    /// 承诺 / Commitment
    Commitment,
    /// 健康 / Health
    Health,
    /// 关系 / Relationship
    Relationship,
    /// 工作 / Work
    Work,
    /// 兴趣 / Interest
    Interest,
}

/// 追问状态 — 6 种生命周期状态
/// Follow-up status — 6 lifecycle states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FollowUpStatus {
    /// 活跃 / Active — 待跟进
    Active,
    /// 已问 / Asked — 已追问过
    Asked,
    /// 已解决 / Resolved
    Resolved,
    /// 被回避 / Deflected
    Deflected,
    /// 已过期 / Expired
    Expired,
    /// 已关闭 / Closed
    Closed,
}

/// 追问深度 — 3 级递进
/// Follow-up depth — 3 progressive levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FollowUpDepth {
    /// 浅层 / Surface
    Surface,
    /// 中层 / Moderate
    Moderate,
    /// 深层 / Deep
    Deep,
}

/// 追问风格 — 4 种表达方式
/// Follow-up style — 4 expression styles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FollowUpStyle {
    /// 直接 / Direct
    Direct,
    /// 间接 / Indirect
    Indirect,
    /// 关切 / Caring
    Caring,
    /// 陪伴 / Companionate
    Companionate,
}

/// 触发原因 — 6 种触发信号
/// Trigger reason — 6 types of trigger signals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriggerReason {
    /// 时间临近 / Time approaching
    TimeApproaching,
    /// 时间刚过 / Time just passed
    TimeJustPassed,
    /// 话题相关 / Topic related
    TopicRelated,
    /// 情感关联 / Emotion associated
    EmotionAssociated,
    /// 长期沉默 / Long silence
    LongSilence,
    /// 再次提及 / Re-mentioned
    ReMentioned,
}

/// 用户反应 — 追问后的用户反馈
/// User reaction — User feedback after a follow-up.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UserReaction {
    /// 是否正面回应 / Whether the user engaged
    pub engaged: bool,
    /// 情感倾向 [-1, 1] / Sentiment polarity
    pub sentiment: f32,
    /// 是否回避 / Whether the user deflected
    pub deflected: bool,
    /// 是否展开详谈 / Whether the user elaborated
    pub elaborated: bool,
}

/// 追问记录 — 单次追问的历史条目
/// Follow-up record — A single follow-up history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpRecord {
    /// 追问时间戳 / Timestamp when asked
    pub asked_at: i64,
    /// 追问深度 / Depth of the follow-up
    pub depth: FollowUpDepth,
    /// 追问风格 / Style of the follow-up
    pub style: FollowUpStyle,
    /// 用户反应 / User's reaction (filled after reply)
    pub user_reaction: Option<UserReaction>,
}

/// 上下文快照 — 提取时的对话上下文
/// Context snapshot — Conversation context at extraction time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// 原始文本 / Original user text
    pub original_text: String,
    /// 前文上下文 / Preceding context
    pub preceding_context: String,
    /// 当时 AI 回复 / AI reply at that time
    pub ai_reply_at_time: String,
}

/// 追踪单元 — 单个待跟进事项
/// Follow-up item — A single trackable follow-up item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpItem {
    /// 唯一 ID / Unique identifier
    pub id: u64,
    /// 事项描述 / Description
    pub description: String,
    /// 类别 / Category
    pub category: FollowUpCategory,
    /// 首次提及时间 / First mention timestamp
    pub first_mentioned_at: i64,
    /// 最后提及时间 / Last mention timestamp
    pub last_mentioned_at: i64,
    /// 提及次数 / Mention count
    pub mention_count: u32,
    /// 预期时间 / Expected time (e.g. deadline)
    pub expected_at: Option<i64>,
    /// 是否逾期 / Whether overdue
    pub is_overdue: bool,
    /// 当前状态 / Current status
    pub status: FollowUpStatus,
    /// 追问历史 / Follow-up history
    pub follow_up_history: Vec<FollowUpRecord>,
    /// 上下文快照 / Context snapshot
    pub context_snapshot: ContextSnapshot,
    /// 情感权重 / Emotional weight
    pub emotional_weight: f32,
    /// 创建时关系深度 / Relationship depth at creation
    pub relationship_depth_at_creation: f32,
    /// 衰减率 / Decay rate (category-specific default)
    pub decay_rate: f32,
}

/// 触发判决结果 — 判定是否应追问及如何追问
/// Trigger verdict — Decision on whether and how to follow up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerVerdict {
    /// 是否应触发 / Whether to trigger
    pub should_trigger: bool,
    /// 触发原因 / Trigger reason
    pub trigger_reason: TriggerReason,
    /// 紧急度 / Urgency score
    pub urgency: f32,
    /// 建议深度 / Suggested depth
    pub suggested_depth: FollowUpDepth,
    /// 建议风格 / Suggested style
    pub suggested_style: FollowUpStyle,
}

impl TriggerVerdict {
    /// 返回一个 should_trigger=false 的默认判决
    /// Returns a default blocked verdict with should_trigger=false.
    pub fn blocked() -> Self {
        Self {
            should_trigger: false,
            trigger_reason: TriggerReason::LongSilence,
            urgency: 0.0,
            suggested_depth: FollowUpDepth::Surface,
            suggested_style: FollowUpStyle::Indirect,
        }
    }
}

/// 统计信息 — 各状态计数
/// Follow-up stats — Counts by status.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FollowUpStats {
    /// 总数 / Total items
    pub total: usize,
    /// 活跃数 / Active items
    pub active: usize,
    /// 已问数 / Asked items
    pub asked: usize,
    /// 已解决数 / Resolved items
    pub resolved: usize,
    /// 已过期数 / Expired items
    pub expired: usize,
}

/// 最近触发记录 — 用于冷却判断
/// Recent recall — Used for cooldown checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRecall {
    /// 事项 ID / Item ID
    pub item_id: u64,
    /// 触发时间 / Trigger timestamp
    pub timestamp: i64,
    /// 触发原因 / Trigger reason
    pub reason: TriggerReason,
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. ExtractPipeline — 提取管线
// ═══════════════════════════════════════════════════════════════════════════

/// 提取模式 — 单条关键词匹配规则
/// Extract pattern — A single keyword matching rule.
#[derive(Debug, Clone)]
struct ExtractPattern {
    /// 关键词列表 / Keywords to match
    keywords: Vec<String>,
    /// 对应类别 / Corresponding category
    category: FollowUpCategory,
    /// 基础置信度 / Base confidence
    confidence: f32,
    /// 是否含时间表达式 / Whether keywords contain time expressions
    has_time_expression: bool,
}

/// 提取管线 — 从用户消息中提取待跟进事项
/// Extract pipeline — Extracts follow-up items from user messages.
#[derive(Debug, Clone)]
pub struct ExtractPipeline {
    /// 提取模式列表 / List of extraction patterns
    patterns: Vec<ExtractPattern>,
}

impl ExtractPipeline {
    /// 初始化中文提取规则 / Initialize with Chinese extraction rules.
    pub fn new() -> Self {
        let patterns = vec![
            // Plan — 计划
            ExtractPattern {
                keywords: vec!["我要".into(), "打算".into(), "准备".into(), "计划".into()],
                category: FollowUpCategory::Plan,
                confidence: 0.85,
                has_time_expression: false,
            },
            // Worry — 担忧
            ExtractPattern {
                keywords: vec![
                    "怕".into(),
                    "担心".into(),
                    "焦虑".into(),
                    "紧张".into(),
                    "害怕".into(),
                    "万一".into(),
                ],
                category: FollowUpCategory::Worry,
                confidence: 0.9,
                has_time_expression: false,
            },
            // Commitment — 承诺（含时间表达式）
            ExtractPattern {
                keywords: vec![
                    "明天".into(),
                    "下周".into(),
                    "周末".into(),
                    "答应".into(),
                    "说好了".into(),
                ],
                category: FollowUpCategory::Commitment,
                confidence: 0.88,
                has_time_expression: true,
            },
            // Health — 健康
            ExtractPattern {
                keywords: vec![
                    "不舒服".into(),
                    "生病".into(),
                    "发烧".into(),
                    "失眠".into(),
                    "头疼".into(),
                    "胃痛".into(),
                    "过敏".into(),
                    "住院".into(),
                    "手术".into(),
                    "体检".into(),
                ],
                category: FollowUpCategory::Health,
                confidence: 0.82,
                has_time_expression: false,
            },
            // Relationship — 关系
            ExtractPattern {
                keywords: vec![
                    "吵架".into(),
                    "分手".into(),
                    "冷战".into(),
                    "闹翻".into(),
                    "离婚".into(),
                    "去世".into(),
                ],
                category: FollowUpCategory::Relationship,
                confidence: 0.85,
                has_time_expression: false,
            },
            // Work — 工作
            ExtractPattern {
                keywords: vec![
                    "面试".into(),
                    "考试".into(),
                    "答辩".into(),
                    "deadline".into(),
                    "ddl".into(),
                    "加班".into(),
                    "项目".into(),
                    "述职".into(),
                    "转正".into(),
                ],
                category: FollowUpCategory::Work,
                confidence: 0.8,
                has_time_expression: false,
            },
            // Interest — 兴趣
            ExtractPattern {
                keywords: vec!["想学".into(), "想去".into(), "想看".into(), "想买".into()],
                category: FollowUpCategory::Interest,
                confidence: 0.8,
                has_time_expression: false,
            },
        ];
        Self { patterns }
    }

    /// 从用户消息提取待跟进事项
    /// Extract follow-up items from a user message.
    pub fn extract(&self, text: &str, emotion_pleasure: f32, now: i64) -> Vec<FollowUpItem> {
        let mut raw: Vec<(FollowUpCategory, String, f32, Option<i64>, String)> = Vec::new();

        for pattern in &self.patterns {
            for kw in &pattern.keywords {
                if let Some(pos) = text.find(kw.as_str()) {
                    // 提取关键词后的文本作为 description
                    let rest = &text[pos + kw.len()..];
                    let desc = Self::capture_description(rest);
                    if desc.is_empty() {
                        continue;
                    }

                    // 尝试提取时间表达式
                    let expected_at = if pattern.has_time_expression {
                        Self::extract_time(kw, now)
                    } else {
                        None
                    };

                    raw.push((
                        pattern.category,
                        desc,
                        pattern.confidence,
                        expected_at,
                        text.to_string(),
                    ));
                }
            }
        }

        // 计算情感权重
        let emotional_weight = if emotion_pleasure < -0.2 {
            0.8
        } else if emotion_pleasure > 0.2 {
            0.2
        } else {
            0.5
        };

        // 去重：同一消息中相同 category+description 只保留置信度最高的
        let mut seen: Vec<(FollowUpCategory, String)> = Vec::new();
        let mut items = Vec::new();
        // 先按置信度降序排列
        raw.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        for (category, description, _confidence, expected_at, original_text) in raw {
            let key = (category, description.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);

            let decay_rate = default_decay_rate(&category);
            let item = FollowUpItem {
                id: 0, // 由 store 分配
                description,
                category,
                first_mentioned_at: now,
                last_mentioned_at: now,
                mention_count: 1,
                expected_at,
                is_overdue: false,
                status: FollowUpStatus::Active,
                follow_up_history: vec![],
                context_snapshot: ContextSnapshot {
                    original_text,
                    preceding_context: String::new(),
                    ai_reply_at_time: String::new(),
                },
                emotional_weight,
                relationship_depth_at_creation: 0.5,
                decay_rate,
            };
            items.push(item);
        }

        items
    }

    /// 截取描述文本 — 到句号/逗号/换行或最多30字
    /// Capture description text — up to period/comma/newline or 30 chars.
    fn capture_description(rest: &str) -> String {
        let end = rest
            .find(|c: char| {
                c == '。'
                    || c == '，'
                    || c == ','
                    || c == '.'
                    || c == '\n'
                    || c == '！'
                    || c == '？'
            })
            .unwrap_or(rest.len());
        let bound = end.min(30);
        let s = rest[..bound].trim();
        // 去掉尾部标点
        s.trim_end_matches([' ', ',', '.', '，', '。']).to_string()
    }

    /// 简单时间提取 — 只处理"明天"和"下周"
    /// Simple time extraction — Only handles "明天" (tomorrow) and "下周" (next week).
    fn extract_time(kw: &str, now: i64) -> Option<i64> {
        match kw {
            "明天" => Some(now + 86400),
            "下周" => Some(now + 604800),
            _ => None,
        }
    }
}

/// 按类别返回默认衰减率
/// Returns the default decay rate for a category.
fn default_decay_rate(category: &FollowUpCategory) -> f32 {
    match category {
        FollowUpCategory::Worry => 72.0,
        FollowUpCategory::Health => 168.0,
        FollowUpCategory::Relationship => 96.0,
        FollowUpCategory::Commitment => 72.0,
        FollowUpCategory::Work => 48.0,
        FollowUpCategory::Plan => 72.0,
        FollowUpCategory::Interest => 24.0,
    }
}

impl Default for ExtractPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. DecayEngine — 非线性时间衰减
// ═══════════════════════════════════════════════════════════════════════════

/// 衰减引擎 — 按类别计算有效权重
/// Decay engine — Computes effective weight by category-specific decay curves.
pub struct DecayEngine;

impl DecayEngine {
    /// 计算有效权重 — 综合基础衰减 + 调制因子
    /// Compute effective weight — Combines base decay with modulation factors.
    pub fn effective_weight(item: &FollowUpItem, now: i64) -> f32 {
        // elapsed hours since last mention
        let elapsed_hours = (now - item.last_mentioned_at).max(0) as f32 / 3600.0;

        // 基础衰减 — 按类别不同
        let base_decay = match item.category {
            // Worry: 半衰期 72h
            FollowUpCategory::Worry => 0.5f32.powf(elapsed_hours / 72.0),
            // Health: 半衰期 168h（1 周）
            FollowUpCategory::Health => 0.5f32.powf(elapsed_hours / 168.0),
            // Relationship: 非单调，基础衰减 + 回避反弹项
            FollowUpCategory::Relationship => {
                let d = 0.5f32.powf(elapsed_hours / 96.0);
                // 如果最近有回避，权重反弹（回避后不宜追问但事项仍在）
                let has_recent_deflection = item
                    .follow_up_history
                    .iter()
                    .rev()
                    .take(2)
                    .any(|r| r.user_reaction.is_some_and(|u| u.deflected));
                if has_recent_deflection {
                    (d + 0.15).min(1.0)
                } else {
                    d
                }
            }
            // Commitment: 阶梯衰减（到期前缓升，到期时峰值，过期后快降）
            FollowUpCategory::Commitment => {
                if let Some(exp) = item.expected_at {
                    let secs_left = exp - now;
                    if secs_left > 0 {
                        // 到期前 — 缓慢上升
                        1.0
                    } else if secs_left > -43200 {
                        // 到期后 12h — 峰值
                        0.8
                    } else {
                        // 过期后 — 快速衰减
                        0.8 * 0.5f32.powf(((-secs_left as f32 / 3600.0) - 12.0).max(0.0) / 24.0)
                    }
                } else {
                    // 无预期时间，按半衰期 72h 衰减
                    0.5f32.powf(elapsed_hours / 72.0)
                }
            }
            // Work: 半衰期 48h
            FollowUpCategory::Work => 0.5f32.powf(elapsed_hours / 48.0),
            // Plan: 半衰期 72h
            FollowUpCategory::Plan => 0.5f32.powf(elapsed_hours / 72.0),
            // Interest: 半衰期 24h
            FollowUpCategory::Interest => 0.5f32.powf(elapsed_hours / 24.0),
        };

        // 调制因子
        let emotional_modulation = 1.0 + item.emotional_weight * 0.5;
        let mention_modulation = 1.0 + (1.0 + item.mention_count as f32 - 1.0).ln() * 0.3;
        let relationship_modulation = 0.5 + item.relationship_depth_at_creation * 0.5;

        (base_decay * emotional_modulation * mention_modulation * relationship_modulation).min(1.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. RecallEngine — 自发回忆
// ═══════════════════════════════════════════════════════════════════════════

/// 情感关联 — 情感区间与类别的关联
/// Emotion association — Maps emotion ranges to related categories.
#[derive(Debug, Clone)]
struct EmotionAssociation {
    /// 情感区间 (pleasure_low, pleasure_high)
    emotion_range: (f32, f32),
    /// 关联类别 / Related categories
    related_categories: Vec<FollowUpCategory>,
    /// 关联强度 / Association strength
    strength: f32,
}

/// 回忆配置 — RecallEngine 的参数
/// Recall config — Parameters for RecallEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallConfig {
    /// 最大最近触发记录数 / Max recent recalls to keep
    pub max_recent_recalls: usize,
    /// 冷却间隔（秒） / Cooldown in seconds
    pub recall_cooldown_secs: i64,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            max_recent_recalls: 50,
            recall_cooldown_secs: 3600,
        }
    }
}

/// 自发回忆引擎 — 基于情感关联触发回忆
/// Recall engine — Triggers recall based on emotion associations.
#[derive(Debug, Clone)]
pub struct RecallEngine {
    /// 情感关联列表 / Emotion associations
    emotion_associations: Vec<EmotionAssociation>,
    /// 最近触发记录 / Recent recall records
    recent_recalls: VecDeque<RecentRecall>,
    /// 配置 / Config
    config: RecallConfig,
}

impl RecallEngine {
    /// 初始化默认关联 / Initialize with default emotion associations.
    pub fn new(config: RecallConfig) -> Self {
        let emotion_associations = vec![
            // 悲伤 → Worry/Health/Relationship
            EmotionAssociation {
                emotion_range: (-0.6, -0.2),
                related_categories: vec![
                    FollowUpCategory::Worry,
                    FollowUpCategory::Health,
                    FollowUpCategory::Relationship,
                ],
                strength: 0.8,
            },
            // 焦虑 → Work/Plan
            EmotionAssociation {
                emotion_range: (-0.2, 0.1),
                related_categories: vec![FollowUpCategory::Work, FollowUpCategory::Plan],
                strength: 0.6,
            },
            // 开心 → Interest/Plan
            EmotionAssociation {
                emotion_range: (0.3, 0.8),
                related_categories: vec![FollowUpCategory::Interest, FollowUpCategory::Plan],
                strength: 0.4,
            },
        ];
        Self {
            emotion_associations,
            recent_recalls: VecDeque::new(),
            config,
        }
    }

    /// 计算情感关联分数 — 给定情感和类别
    /// Compute emotion association score for a given pleasure and category.
    pub fn emotion_recall_score(&self, pleasure: f32, category: FollowUpCategory) -> f32 {
        let mut best = 0.0f32;
        for assoc in &self.emotion_associations {
            if pleasure >= assoc.emotion_range.0
                && pleasure < assoc.emotion_range.1
                && assoc.related_categories.contains(&category)
            {
                best = best.max(assoc.strength);
            }
        }
        best
    }

    /// 记录一次触发 / Record a recall trigger.
    pub fn record_recall(&mut self, item_id: u64, now: i64, reason: TriggerReason) {
        self.recent_recalls.push_back(RecentRecall {
            item_id,
            timestamp: now,
            reason,
        });
        // 修剪到最大容量
        while self.recent_recalls.len() > self.config.max_recent_recalls {
            self.recent_recalls.pop_front();
        }
    }

    /// 冷却检查 — 该事项是否仍在冷却期
    /// Check if an item is still in cooldown.
    pub fn is_in_cooldown(&self, item_id: u64, now: i64) -> bool {
        for recall in self.recent_recalls.iter().rev() {
            if recall.item_id == item_id {
                let elapsed = now - recall.timestamp;
                return elapsed < self.config.recall_cooldown_secs;
            }
        }
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. TriggerJudge — 触发判决
// ═══════════════════════════════════════════════════════════════════════════

/// 触发配置 — TriggerJudge 的参数
/// Trigger config — Parameters for TriggerJudge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// 每日追问上限 / Max follow-ups per day
    pub max_per_day: u32,
    /// 最小追问间隔（秒） / Min interval between follow-ups in seconds
    pub min_interval_secs: i64,
    /// 触发阈值 / Trigger threshold
    pub trigger_threshold: f32,
    /// 最小权重阈值 / Min weight threshold
    pub min_weight_threshold: f32,
    /// 时间信号权重 / Time signal weight
    pub time_weight: f32,
    /// 话题相关权重 / Topic relevance weight
    pub topic_weight: f32,
    /// 情感关联权重 / Emotion association weight
    pub emotion_weight: f32,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            max_per_day: 5,
            min_interval_secs: 3600,
            trigger_threshold: 0.3,
            min_weight_threshold: 0.1,
            time_weight: 0.4,
            topic_weight: 0.3,
            emotion_weight: 0.3,
        }
    }
}

/// 触发判决器 — 决定是否及如何追问
/// Trigger judge — Decides whether and how to follow up.
#[derive(Debug, Clone)]
pub struct TriggerJudge {
    /// 配置 / Config
    config: TriggerConfig,
}

impl TriggerJudge {
    /// 创建触发判决器 / Create a new trigger judge.
    pub fn new(config: TriggerConfig) -> Self {
        Self { config }
    }

    /// 判决 — 综合硬性门控和软性评分
    /// Judge — Combines hard gates and soft scoring.
    #[allow(clippy::too_many_arguments)]
    pub fn judge(
        &self,
        item: &FollowUpItem,
        now: i64,
        relationship_stage_name: &str,
        today_count: u32,
        last_follow_up_secs: i64,
        current_pleasure: f32,
        recall_engine: &RecallEngine,
    ) -> TriggerVerdict {
        // ── 硬性门控 ──

        // 关系阶段：acquaintance 不触发
        if relationship_stage_name == "acquaintance" {
            return TriggerVerdict::blocked();
        }

        // 每日上限
        if today_count >= self.config.max_per_day {
            return TriggerVerdict::blocked();
        }

        // 冷却间隔
        if now - last_follow_up_secs < self.config.min_interval_secs {
            return TriggerVerdict::blocked();
        }

        // 回避降级 — 最近 3 次追问中有 2 次以上回避则不触发
        let recent_deflection_count = item
            .follow_up_history
            .iter()
            .rev()
            .take(3)
            .filter(|r| r.user_reaction.is_some_and(|u| u.deflected))
            .count();
        if recent_deflection_count >= 2 {
            return TriggerVerdict::blocked();
        }

        // ── 软性评分 ──
        let effective_weight = DecayEngine::effective_weight(item, now);

        // 最小权重门控
        if effective_weight < self.config.min_weight_threshold {
            return TriggerVerdict {
                should_trigger: false,
                trigger_reason: TriggerReason::LongSilence,
                urgency: effective_weight,
                ..TriggerVerdict::blocked()
            };
        }

        // 时间信号分数
        let time_score = self.time_signal_score(item, now);

        // 情感关联分数
        let emotion_score = recall_engine.emotion_recall_score(current_pleasure, item.category);

        // 综合评分
        let composite = effective_weight
            * (1.0
                + time_score * self.config.time_weight
                + emotion_score * self.config.emotion_weight);

        // 确定触发原因
        let trigger_reason = if time_score > 0.5 {
            if let Some(exp) = item.expected_at {
                if exp > now {
                    TriggerReason::TimeApproaching
                } else {
                    TriggerReason::TimeJustPassed
                }
            } else {
                TriggerReason::TimeApproaching
            }
        } else if emotion_score > 0.3 {
            TriggerReason::EmotionAssociated
        } else {
            TriggerReason::LongSilence
        };

        // ── 深度决策 ──
        let depth = Self::decide_depth(item);

        // ── 风格决策 ──
        let forced_companionate = recent_deflection_count >= 1;
        let style = Self::decide_style(item, forced_companionate, relationship_stage_name);

        TriggerVerdict {
            should_trigger: composite >= self.config.trigger_threshold,
            trigger_reason,
            urgency: composite.min(1.0),
            suggested_depth: depth,
            suggested_style: style,
        }
    }

    /// 时间信号分数 — 到期临近或刚过时较高
    /// Time signal score — Higher when deadline is approaching or just passed.
    fn time_signal_score(&self, item: &FollowUpItem, now: i64) -> f32 {
        if let Some(exp) = item.expected_at {
            let delta = exp - now;
            if delta > 0 && delta <= 86400 {
                // 24h 内到期
                1.0
            } else if (-43200..=0).contains(&delta) {
                // 过期 12h 内
                0.8
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// 深度决策 — 按追问次数和类别递进
    /// Decide depth — Progresses by follow-up count and category.
    fn decide_depth(item: &FollowUpItem) -> FollowUpDepth {
        match item.follow_up_history.len() {
            0 => match item.category {
                FollowUpCategory::Worry
                | FollowUpCategory::Health
                | FollowUpCategory::Relationship => FollowUpDepth::Moderate,
                _ => FollowUpDepth::Surface,
            },
            1 => FollowUpDepth::Moderate,
            _ => {
                // 第三次+：正面回应 → Deep，否则 Moderate
                if item
                    .follow_up_history
                    .last()
                    .is_some_and(|r| r.user_reaction.is_some_and(|u| u.engaged && !u.deflected))
                {
                    FollowUpDepth::Deep
                } else {
                    FollowUpDepth::Moderate
                }
            }
        }
    }

    /// 风格决策 — 按类别、回避历史和关系深度
    /// Decide style — Based on category, deflection history, and relationship depth.
    fn decide_style(
        item: &FollowUpItem,
        forced_companionate: bool,
        relationship_stage: &str,
    ) -> FollowUpStyle {
        if forced_companionate {
            return FollowUpStyle::Companionate;
        }
        match item.category {
            FollowUpCategory::Worry | FollowUpCategory::Health | FollowUpCategory::Relationship => {
                FollowUpStyle::Caring
            }
            _ => {
                // 深度关系 → Direct
                if relationship_stage == "deep" {
                    FollowUpStyle::Direct
                } else {
                    FollowUpStyle::Indirect
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. ExpressionWeaver — 表达编织
// ═══════════════════════════════════════════════════════════════════════════

/// 表达编织器 — 生成追问提示片段
/// Expression weaver — Generates follow-up prompt fragments.
pub struct ExpressionWeaver;

impl ExpressionWeaver {
    /// 编织追问提示 — 15 种组合的中文自然语言提示
    /// Weave a follow-up prompt — 15 combinations of Chinese natural language hints.
    pub fn weave(
        item: &FollowUpItem,
        depth: FollowUpDepth,
        style: FollowUpStyle,
        _urgency: f32,
    ) -> String {
        let desc = &item.description;
        let tc = time_context_phrase(item);
        match (depth, style) {
            // Surface + Direct
            (FollowUpDepth::Surface, FollowUpStyle::Direct) => {
                format!(
                    "用户之前提到过「{}」，如果话题自然，可以顺便问一下进展。",
                    desc
                )
            }
            // Surface + Indirect
            (FollowUpDepth::Surface, FollowUpStyle::Indirect) => {
                format!("用户曾提到「{}」，可以在相关话题出现时自然提及。", desc)
            }
            // Surface + Caring
            (FollowUpDepth::Surface, FollowUpStyle::Caring) => {
                format!("用户曾提到「{}」，可以适时关心。", desc)
            }
            // Surface + Companionate
            (FollowUpDepth::Surface, FollowUpStyle::Companionate) => {
                format!(
                    "用户之前提到「{}」但回避了追问。不要直接问，用陪伴方式。",
                    desc
                )
            }
            // Moderate + Direct
            (FollowUpDepth::Moderate, FollowUpStyle::Direct) => {
                format!("用户{}提到「{}」，可以直接问问进展。", tc, desc)
            }
            // Moderate + Indirect
            (FollowUpDepth::Moderate, FollowUpStyle::Indirect) => {
                format!(
                    "用户之前提到「{}」。不要直接追问，在相关话题出现时自然接。",
                    desc
                )
            }
            // Moderate + Caring
            (FollowUpDepth::Moderate, FollowUpStyle::Caring) => {
                format!(
                    "用户之前{}提到「{}」，看起来比较在意。可以轻声关心一下。",
                    tc, desc
                )
            }
            // Moderate + Companionate
            (FollowUpDepth::Moderate, FollowUpStyle::Companionate) => {
                format!(
                    "用户之前提到「{}」但回避了追问。不要直接问，用陪伴方式。",
                    desc
                )
            }
            // Deep + Direct
            (FollowUpDepth::Deep, FollowUpStyle::Direct) => {
                format!("用户多次提及「{}」，可以直接询问最新进展。", desc)
            }
            // Deep + Indirect
            (FollowUpDepth::Deep, FollowUpStyle::Indirect) => {
                format!("用户曾深度提及「{}」，在合适时机自然深入聊聊。", desc)
            }
            // Deep + Caring
            (FollowUpDepth::Deep, FollowUpStyle::Caring) => {
                format!(
                    "用户曾深度提及「{}」，在合适时机表达你一直在意这件事。",
                    desc
                )
            }
            // Deep + Companionate
            (FollowUpDepth::Deep, FollowUpStyle::Companionate) => {
                format!(
                    "用户提到「{}」时回避了追问。用陪伴方式，让用户知道你一直在。",
                    desc
                )
            }
        }
    }
}

/// 时间上下文短语 — "今天"/"昨天"/"前几天"/"上周"/"上个月"/"之前"
/// Time context phrase — Returns a relative time description.
fn time_context_phrase(item: &FollowUpItem) -> String {
    let hours = (item.last_mentioned_at - item.first_mentioned_at).max(0) as f32 / 3600.0;
    if hours <= 24.0 {
        "今天".to_string()
    } else if hours <= 48.0 {
        "昨天".to_string()
    } else if hours <= 168.0 {
        "前几天".to_string()
    } else if hours <= 336.0 {
        "上周".to_string()
    } else if hours <= 720.0 {
        "上个月".to_string()
    } else {
        "之前".to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. ReactionAnalyzer — 用户反应分析
// ═══════════════════════════════════════════════════════════════════════════

/// 反应分析器 — 分析用户对追问的反应
/// Reaction analyzer — Analyzes user reaction to a follow-up.
pub struct ReactionAnalyzer;

impl ReactionAnalyzer {
    /// 分析用户回复 — 判定 engaged/sentiment/deflected/elaborated
    /// Analyze user reply — Determines engaged, sentiment, deflected, elaborated.
    pub fn analyze(user_reply: &str) -> UserReaction {
        let deflected = Self::is_deflection(user_reply);
        let engaged = !deflected && Self::is_engaged(user_reply);
        let sentiment = Self::sentiment_score(user_reply);
        let elaborated = !deflected && user_reply.chars().count() > 20;
        UserReaction {
            engaged,
            sentiment,
            deflected,
            elaborated,
        }
    }

    /// 回避检测 / Deflection detection.
    fn is_deflection(text: &str) -> bool {
        const DEFL_KEYWORDS: &[&str] = &[
            "别提了",
            "不想说",
            "不想谈",
            "算了",
            "别问了",
            "没什么好说的",
            "过去了",
            "别管了",
            "不用管",
            "无所谓",
            "不想聊这个",
            "换个话题",
        ];
        DEFL_KEYWORDS.iter().any(|k| text.contains(k))
    }

    /// 正面回应检测 / Engagement detection.
    fn is_engaged(text: &str) -> bool {
        const ENG_KEYWORDS: &[&str] = &[
            "嗯", "是啊", "对", "结果", "后来", "终于", "还好", "挺", "确实", "其实",
        ];
        ENG_KEYWORDS.iter().any(|k| text.contains(k))
    }

    /// 情感倾向 — 正面词 vs 负面词
    /// Sentiment score — Positive vs negative words.
    fn sentiment_score(text: &str) -> f32 {
        const POS: &[&str] = &["好", "棒", "开心", "轻松", "终于", "成功", "过了"];
        const NEG: &[&str] = &["没", "不", "失败", "难过", "烦", "差", "糟"];
        let pos_count = POS.iter().filter(|k| text.contains(*k)).count();
        let neg_count = NEG.iter().filter(|k| text.contains(*k)).count();
        if pos_count + neg_count == 0 {
            0.0
        } else {
            (pos_count as f32 - neg_count as f32) / (pos_count + neg_count) as f32
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. FollowUpStore — 持久化存储
// ═══════════════════════════════════════════════════════════════════════════

/// 追问存储 — 基于 sled 的持久化
/// Follow-up store — Sled-backed persistence.
pub struct FollowUpStore {
    db: sled::Db,
    id_counter: AtomicU64,
}

impl FollowUpStore {
    /// 打开持久化存储 / Open a persistent store at the given path.
    pub fn open(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| e.to_string())?;
        // 找到最大 ID 以避免冲突
        let max_id = db
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, _)| u64::from_be_bytes(k.as_ref().try_into().unwrap_or([0u8; 8])))
            .max()
            .unwrap_or(0);
        Ok(Self {
            db,
            id_counter: AtomicU64::new(max_id),
        })
    }

    /// 打开内存存储 / Open an in-memory store.
    pub fn open_in_memory() -> Result<Self, String> {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            db,
            id_counter: AtomicU64::new(0),
        })
    }

    /// 写入或更新 / Upsert a follow-up item.
    pub fn upsert(&self, item: &FollowUpItem) -> Result<(), String> {
        let value = bincode::serialize(item).map_err(|e| e.to_string())?;
        self.db
            .insert(item.id.to_be_bytes(), value)
            .map_err(|e| e.to_string())?;
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 按 ID 读取 / Get a follow-up item by ID.
    pub fn get(&self, id: u64) -> Result<Option<FollowUpItem>, String> {
        match self.db.get(id.to_be_bytes()).map_err(|e| e.to_string())? {
            Some(ivec) => Ok(Some(
                bincode::deserialize(&ivec).map_err(|e| e.to_string())?,
            )),
            None => Ok(None),
        }
    }

    /// 获取所有活跃事项 / Get all active items.
    pub fn active_items(&self) -> Result<Vec<FollowUpItem>, String> {
        let mut items = Vec::new();
        for r in self.db.iter() {
            let (_, value) = r.map_err(|e| e.to_string())?;
            if let Ok(item) = bincode::deserialize::<FollowUpItem>(&value) {
                if item.status == FollowUpStatus::Active {
                    items.push(item);
                }
            }
        }
        Ok(items)
    }

    /// 获取候选事项 — Active + Asked，按 effective_weight 降序
    /// Get candidate items — Active + Asked, sorted by effective weight descending.
    pub fn candidates_for_check(
        &self,
        now: i64,
        limit: usize,
    ) -> Result<Vec<FollowUpItem>, String> {
        let mut items = Vec::new();
        for r in self.db.iter() {
            let (_, value) = r.map_err(|e| e.to_string())?;
            if let Ok(item) = bincode::deserialize::<FollowUpItem>(&value) {
                if item.status == FollowUpStatus::Active || item.status == FollowUpStatus::Asked {
                    items.push(item);
                }
            }
        }
        items.sort_by(|a, b| {
            DecayEngine::effective_weight(b, now)
                .partial_cmp(&DecayEngine::effective_weight(a, now))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(limit);
        Ok(items)
    }

    /// 设置状态 / Set the status of an item.
    pub fn set_status(&self, id: u64, status: FollowUpStatus) -> Result<(), String> {
        let mut item = self.get(id)?.ok_or_else(|| "not found".to_string())?;
        item.status = status;
        self.upsert(&item)
    }

    /// 添加追问记录 / Add a follow-up record to an item.
    pub fn add_follow_up_record(&self, id: u64, record: FollowUpRecord) -> Result<(), String> {
        let mut item = self.get(id)?.ok_or_else(|| "not found".to_string())?;
        item.follow_up_history.push(record);
        self.upsert(&item)
    }

    /// 更新用户反应 — 更新最后一条 record 的 user_reaction
    /// Update user reaction — Updates the user_reaction of the last follow-up record.
    pub fn update_user_reaction(&self, id: u64, reaction: UserReaction) -> Result<(), String> {
        let mut item = self.get(id)?.ok_or_else(|| "not found".to_string())?;
        if let Some(last) = item.follow_up_history.last_mut() {
            last.user_reaction = Some(reaction);
        }
        self.upsert(&item)
    }

    /// 清理过期事项 — 删除 Expired/Closed 且 last_mentioned_at < before
    /// Prune expired items — Remove Expired/Closed items older than `before`.
    pub fn prune_expired(&self, before: i64) -> Result<usize, String> {
        let mut pruned = 0usize;
        let mut to_remove = Vec::new();
        for r in self.db.iter() {
            let (key, value) = r.map_err(|e| e.to_string())?;
            if let Ok(item) = bincode::deserialize::<FollowUpItem>(&value) {
                if (item.status == FollowUpStatus::Expired || item.status == FollowUpStatus::Closed)
                    && item.last_mentioned_at < before
                {
                    to_remove.push(key);
                }
            }
        }
        for key in to_remove {
            self.db.remove(&key).map_err(|e| e.to_string())?;
            pruned += 1;
        }
        if pruned > 0 {
            self.db.flush().map_err(|e| e.to_string())?;
        }
        Ok(pruned)
    }

    /// 统计信息 / Compute stats.
    pub fn stats(&self) -> FollowUpStats {
        let mut s = FollowUpStats::default();
        for r in self.db.iter().filter_map(|r| r.ok()) {
            if let Ok(item) = bincode::deserialize::<FollowUpItem>(&r.1) {
                s.total += 1;
                match item.status {
                    FollowUpStatus::Active => s.active += 1,
                    FollowUpStatus::Asked => s.asked += 1,
                    FollowUpStatus::Resolved => s.resolved += 1,
                    FollowUpStatus::Expired => s.expired += 1,
                    _ => {}
                }
            }
        }
        s
    }

    /// 分配下一个 ID / Allocate the next ID.
    pub fn next_id(&self) -> u64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst) + 1
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. FollowUpTracker — 顶层组合
// ═══════════════════════════════════════════════════════════════════════════

/// 追问追踪器 — 顶层组合，串联提取/衰减/回忆/触发/表达/反应
/// Follow-up tracker — Top-level orchestrator connecting all sub-engines.
pub struct FollowUpTracker {
    /// 提取管线 / Extract pipeline
    extract: ExtractPipeline,
    /// 回忆引擎（需 Mutex 因为有内部可变性）/ Recall engine (Mutex for interior mutability)
    recall: parking_lot::Mutex<RecallEngine>,
    /// 触发判决器 / Trigger judge
    judge: TriggerJudge,
    /// 持久化存储 / Persistent store
    store: FollowUpStore,
}

impl FollowUpTracker {
    /// 创建追问追踪器 / Create a new follow-up tracker.
    pub fn new(
        store: FollowUpStore,
        recall_config: RecallConfig,
        trigger_config: TriggerConfig,
    ) -> Self {
        Self {
            extract: ExtractPipeline::new(),
            recall: parking_lot::Mutex::new(RecallEngine::new(recall_config)),
            judge: TriggerJudge::new(trigger_config),
            store,
        }
    }

    /// 从用户消息提取并保存待跟进事项
    /// Extract follow-up items from a user message and persist them.
    pub fn extract_from_message(
        &self,
        text: &str,
        emotion_pleasure: f32,
        now: i64,
    ) -> Vec<FollowUpItem> {
        let items = self.extract.extract(text, emotion_pleasure, now);
        items
            .into_iter()
            .map(|mut item| {
                let id = self.store.next_id();
                item.id = id;
                let _ = self.store.upsert(&item);
                item
            })
            .collect()
    }

    /// 检查所有候选事项是否应追问
    /// Check all candidate items for follow-up triggers.
    pub fn check_for_follow_up(
        &self,
        now: i64,
        relationship_stage_name: &str,
        today_count: u32,
        last_follow_up_secs: i64,
        current_pleasure: f32,
    ) -> Vec<(FollowUpItem, TriggerVerdict)> {
        let recall = self.recall.lock();
        let candidates = self.store.candidates_for_check(now, 20).unwrap_or_default();
        candidates
            .into_iter()
            .filter_map(|item| {
                // 冷却检查
                if recall.is_in_cooldown(item.id, now) {
                    return None;
                }
                let verdict = self.judge.judge(
                    &item,
                    now,
                    relationship_stage_name,
                    today_count,
                    last_follow_up_secs,
                    current_pleasure,
                    &recall,
                );
                if verdict.should_trigger {
                    Some((item, verdict))
                } else {
                    None
                }
            })
            .collect()
    }

    /// 生成追问提示 / Generate a follow-up prompt.
    pub fn generate_prompt(&self, item: &FollowUpItem, verdict: &TriggerVerdict) -> String {
        ExpressionWeaver::weave(
            item,
            verdict.suggested_depth,
            verdict.suggested_style,
            verdict.urgency,
        )
    }

    /// 分析用户反应并更新存储
    /// Analyze user reaction and update the store.
    pub fn analyze_reaction(&self, user_reply: &str, item_id: u64) -> UserReaction {
        let reaction = ReactionAnalyzer::analyze(user_reply);
        let _ = self.store.update_user_reaction(item_id, reaction);
        if reaction.deflected {
            let _ = self.store.set_status(item_id, FollowUpStatus::Deflected);
        }
        reaction
    }

    /// 标记已追问 — 添加记录并更新状态
    /// Mark an item as asked — Add a follow-up record and set status.
    pub fn mark_asked(&self, item_id: u64, depth: FollowUpDepth, style: FollowUpStyle, now: i64) {
        let _ = self.store.set_status(item_id, FollowUpStatus::Asked);
        let record = FollowUpRecord {
            asked_at: now,
            depth,
            style,
            user_reaction: None,
        };
        let _ = self.store.add_follow_up_record(item_id, record);
    }

    /// 活跃事项数 / Count of active items.
    pub fn active_count(&self) -> usize {
        self.store.active_items().map(|v| v.len()).unwrap_or(0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：构造一个 FollowUpItem
    fn mk_item(cat: FollowUpCategory, ts: i64, ew: f32, rd: f32) -> FollowUpItem {
        FollowUpItem {
            id: 1,
            description: "test".into(),
            category: cat,
            first_mentioned_at: ts,
            last_mentioned_at: ts,
            mention_count: 1,
            expected_at: None,
            is_overdue: false,
            status: FollowUpStatus::Active,
            follow_up_history: vec![],
            context_snapshot: ContextSnapshot {
                original_text: String::new(),
                preceding_context: String::new(),
                ai_reply_at_time: String::new(),
            },
            emotional_weight: ew,
            relationship_depth_at_creation: rd,
            decay_rate: default_decay_rate(&cat),
        }
    }

    // ── ExtractPipeline 测试 ──

    #[test]
    fn test_extract_plan() {
        let ep = ExtractPipeline::new();
        let r = ep.extract("我要考研", 0.5, 0);
        assert!(!r.is_empty());
        assert_eq!(r[0].category, FollowUpCategory::Plan);
        assert_eq!(r[0].description, "考研");
    }

    #[test]
    fn test_extract_worry() {
        let ep = ExtractPipeline::new();
        let r = ep.extract("担心面试过不了", 0.7, 0);
        assert!(!r.is_empty());
        assert_eq!(r[0].category, FollowUpCategory::Worry);
        assert_eq!(r[0].description, "面试过不了");
    }

    #[test]
    fn test_extract_health() {
        let ep = ExtractPipeline::new();
        let r = ep.extract("最近失眠了", 0.3, 0);
        assert!(!r.is_empty());
        assert_eq!(r[0].category, FollowUpCategory::Health);
    }

    #[test]
    fn test_extract_commitment() {
        let ep = ExtractPipeline::new();
        let now: i64 = 1_000_000;
        let r = ep.extract("明天再来", 0.3, now);
        assert!(!r.is_empty());
        assert_eq!(r[0].category, FollowUpCategory::Commitment);
        assert_eq!(r[0].expected_at.unwrap(), now + 86400);
    }

    #[test]
    fn test_extract_multiple() {
        let ep = ExtractPipeline::new();
        let r = ep.extract("我要考研，但是担心面试过不了", 0.5, 0);
        // 应至少提取出 Plan 和 Worry
        let cats: Vec<_> = r.iter().map(|i| i.category).collect();
        assert!(cats.contains(&FollowUpCategory::Plan));
        assert!(cats.contains(&FollowUpCategory::Worry));
    }

    #[test]
    fn test_extract_no_match() {
        let ep = ExtractPipeline::new();
        let r = ep.extract("今天天气不错", 0.1, 0);
        assert!(r.is_empty());
    }

    // ── DecayEngine 测试 ──

    #[test]
    fn test_decay_worry_slow() {
        // Worry 半衰期 72h → 72h 后权重约 0.5（受调制因子影响）
        let now: i64 = 259200; // 72h in seconds
        let item = mk_item(FollowUpCategory::Worry, 0, 0.0, 1.0);
        let w = DecayEngine::effective_weight(&item, now);
        // 基础衰减 = 0.5, emotional_mod = 1.0, mention_mod = 1.0, rel_mod = 1.0
        assert!((w - 0.5).abs() < 0.05, "got {}", w);
    }

    #[test]
    fn test_decay_interest_fast() {
        // Interest 半衰期 24h → 24h 后权重约 0.5
        let now: i64 = 86400; // 24h
        let item = mk_item(FollowUpCategory::Interest, 0, 0.0, 1.0);
        let w = DecayEngine::effective_weight(&item, now);
        assert!((w - 0.5).abs() < 0.05, "got {}", w);
    }

    #[test]
    fn test_decay_commitment_staircase() {
        let now: i64 = 1_000_000;
        let mut item = mk_item(FollowUpCategory::Commitment, now - 3600, 0.0, 1.0);

        // 到期前 → 权重高
        item.expected_at = Some(now + 43200);
        assert!(DecayEngine::effective_weight(&item, now) > 0.9);

        // 刚过期 → 仍有较高权重
        item.expected_at = Some(now - 3600);
        assert!(DecayEngine::effective_weight(&item, now) > 0.5);
    }

    #[test]
    fn test_decay_emotional_modulation() {
        // 悲伤时说的权重更高 — 用较大时间间隔避免 clamp 到 1.0
        let now: i64 = 259200; // 72h 后，基础衰减 ≈ 0.5
        let item_sad = mk_item(FollowUpCategory::Worry, 0, 0.8, 1.0);
        let item_neutral = mk_item(FollowUpCategory::Worry, 0, 0.2, 1.0);
        let w_sad = DecayEngine::effective_weight(&item_sad, now);
        let w_neutral = DecayEngine::effective_weight(&item_neutral, now);
        assert!(w_sad > w_neutral, "sad={} neutral={}", w_sad, w_neutral);
    }

    // ── RecallEngine 测试 ──

    #[test]
    fn test_recall_emotion_association() {
        let re = RecallEngine::new(RecallConfig::default());
        // 悲伤 → 联想到 Worry
        let score = re.emotion_recall_score(-0.4, FollowUpCategory::Worry);
        assert!(score > 0.0, "score={}", score);
        // 开心 → 不联想到 Worry
        let score2 = re.emotion_recall_score(0.5, FollowUpCategory::Worry);
        assert_eq!(score2, 0.0);
    }

    // ── TriggerJudge 测试 ──

    #[test]
    fn test_trigger_judge_acquaintance_blocked() {
        let tj = TriggerJudge::new(TriggerConfig::default());
        let re = RecallEngine::new(RecallConfig::default());
        let item = mk_item(FollowUpCategory::Worry, 0, 0.5, 0.5);
        let verdict = tj.judge(&item, 3600, "acquaintance", 0, 0, 0.0, &re);
        assert!(!verdict.should_trigger);
    }

    #[test]
    fn test_trigger_judge_daily_limit() {
        let tj = TriggerJudge::new(TriggerConfig::default());
        let re = RecallEngine::new(RecallConfig::default());
        let item = mk_item(FollowUpCategory::Worry, 0, 0.5, 0.5);
        let verdict = tj.judge(&item, 7200, "familiar", 5, 0, 0.0, &re);
        assert!(!verdict.should_trigger);
    }

    #[test]
    fn test_trigger_judge_cooldown() {
        let tj = TriggerJudge::new(TriggerConfig::default());
        let re = RecallEngine::new(RecallConfig::default());
        let item = mk_item(FollowUpCategory::Worry, 0, 0.5, 0.5);
        // last_follow_up_secs = 100, now = 200 → 间隔 100s < 3600s
        let verdict = tj.judge(&item, 200, "familiar", 0, 100, 0.0, &re);
        assert!(!verdict.should_trigger);
    }

    #[test]
    fn test_trigger_judge_deflection_downgrade() {
        let tj = TriggerJudge::new(TriggerConfig::default());
        let re = RecallEngine::new(RecallConfig::default());
        let mut item = mk_item(FollowUpCategory::Worry, 0, 0.5, 0.5);
        // 添加 2 次回避记录
        item.follow_up_history.push(FollowUpRecord {
            asked_at: 100,
            depth: FollowUpDepth::Surface,
            style: FollowUpStyle::Indirect,
            user_reaction: Some(UserReaction {
                engaged: false,
                sentiment: -0.5,
                deflected: true,
                elaborated: false,
            }),
        });
        item.follow_up_history.push(FollowUpRecord {
            asked_at: 200,
            depth: FollowUpDepth::Moderate,
            style: FollowUpStyle::Caring,
            user_reaction: Some(UserReaction {
                engaged: false,
                sentiment: -0.3,
                deflected: true,
                elaborated: false,
            }),
        });
        let verdict = tj.judge(&item, 7200, "familiar", 0, 0, 0.0, &re);
        assert!(!verdict.should_trigger);
    }

    // ── ExpressionWeaver 测试 ──

    #[test]
    fn test_expression_weaver_surface_direct() {
        let item = mk_item(FollowUpCategory::Plan, 0, 0.5, 0.5);
        let s = ExpressionWeaver::weave(&item, FollowUpDepth::Surface, FollowUpStyle::Direct, 0.5);
        assert!(s.contains("顺便问一下"));
    }

    #[test]
    fn test_expression_weaver_deep_caring() {
        let item = mk_item(FollowUpCategory::Worry, 0, 0.5, 0.5);
        let s = ExpressionWeaver::weave(&item, FollowUpDepth::Deep, FollowUpStyle::Caring, 0.8);
        assert!(s.contains("一直在意"));
    }

    #[test]
    fn test_expression_weaver_companionate() {
        let item = mk_item(FollowUpCategory::Health, 0, 0.5, 0.5);
        let s = ExpressionWeaver::weave(
            &item,
            FollowUpDepth::Moderate,
            FollowUpStyle::Companionate,
            0.5,
        );
        assert!(s.contains("陪伴"));
    }

    // ── ReactionAnalyzer 测试 ──

    #[test]
    fn test_reaction_analyzer_engaged() {
        let r = ReactionAnalyzer::analyze("嗯，结果出来了");
        assert!(r.engaged);
        assert!(!r.deflected);
    }

    #[test]
    fn test_reaction_analyzer_deflected() {
        let r = ReactionAnalyzer::analyze("别提了");
        assert!(r.deflected);
        assert!(!r.engaged);
    }

    #[test]
    fn test_reaction_analyzer_elaborated() {
        let r = ReactionAnalyzer::analyze(
            "嗯，后来我去看了医生，他说没什么大问题，开了点药让我回去休息",
        );
        assert!(r.elaborated);
        assert!(r.engaged);
    }

    // ── FollowUpStore 测试 ──

    #[test]
    fn test_store_crud() {
        let store = FollowUpStore::open_in_memory().unwrap();
        let mut item = mk_item(FollowUpCategory::Plan, 1000, 0.5, 0.5);
        item.id = store.next_id();
        store.upsert(&item).unwrap();

        let loaded = store.get(item.id).unwrap().unwrap();
        assert_eq!(loaded.description, "test");
        assert_eq!(store.active_items().unwrap().len(), 1);

        // 更新状态
        store.set_status(item.id, FollowUpStatus::Resolved).unwrap();
        let loaded2 = store.get(item.id).unwrap().unwrap();
        assert_eq!(loaded2.status, FollowUpStatus::Resolved);
    }

    #[test]
    fn test_store_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        {
            let store = FollowUpStore::open(path).unwrap();
            let mut item = mk_item(FollowUpCategory::Worry, 1000, 0.5, 0.5);
            item.id = store.next_id();
            store.upsert(&item).unwrap();
        }
        // 重新打开应能读到
        let store2 = FollowUpStore::open(path).unwrap();
        assert_eq!(store2.get(1).unwrap().unwrap().description, "test");
    }
}
