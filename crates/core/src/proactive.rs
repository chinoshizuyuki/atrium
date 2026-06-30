// SPDX-License-Identifier: MIT
//! 主动决策引擎 — 状态感知的主动消息生成
//! Proactive Decision Engine — State-aware proactive message generation.
//!
//! 让 AI 在对的时机说对的话，而不是到时间了就说话。
//!
//! 核心组件：
//! - TimingJudge: 6 条规则决定什么时候不该说话
//! - AwayDetector: 推断用户是否离开了对话
//! - TopicSelector: 基于上下文选择最合适的话题（3 个内置源）
//! - EventMemory: 记住用户提过的未来事件，适时提醒
//! - SilenceBudget: 沉默也有价值，不是"多久没说话就该说"
//! - DecisionCooldown: 指数退避防止频繁主动打扰
//!
//! 所有决策为纯规则匹配，延迟 < 1μs/决策。

use std::time::{Duration, Instant};

use chrono::{Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};

use crate::config::ProactiveCfg;

// ════════════════════════════════════════════════════════════════════
//  枚举定义
// ════════════════════════════════════════════════════════════════════

/// 对话状态
#[derive(Clone, Debug, PartialEq)]
pub enum ConversationState {
    /// 对话活跃进行中
    Active {
        last_message_ago: Duration,
        momentum: f32,
    },
    /// 用户暂时离开（推断）
    UserAway {
        away_since: Duration,
        away_reason: AwayReason,
    },
    /// 对话自然结束
    Concluded { concluded_since: Duration },
    /// 用户在思考（短沉默，不应打扰）
    Thinking { silence_duration: Duration },
}

/// 离开原因
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AwayReason {
    /// 短离开（倒水、接电话）
    ShortBreak,
    /// 去做别的事了
    TaskSwitch,
    /// 今天不会再来了（推断）
    DayEnded,
    /// 不确定
    Unknown,
}

/// 沉默原因（为什么现在不说话）
#[derive(Clone, Debug)]
pub enum SilenceReason {
    /// 用户在思考
    UserIsThinking,
    /// 对话刚自然结束
    JustConcluded,
    /// 用户离开了
    UserIsAway,
    /// 冷却期
    CooldownActive,
    /// 用户可能在睡觉
    UserLikelySleeping,
    /// 综合评分太低
    ScoreTooLow(f32),
}

/// 话题类型
#[derive(Clone, Debug)]
pub enum TopicKind {
    /// 延续上次的话题
    FollowUp,
    /// 基于用户兴趣
    InterestBased,
    /// 分享 AI 自己的"想法"
    AiThought,
    /// 关心用户近况
    CareCheck,
    /// 回顾过去的某件事
    MemoryRecall,
    /// 提醒即将到来的事件
    Reminder,
}

/// 主动决策输出 / Proactive decision output.
#[derive(Clone, Debug)]
pub enum ProactiveDecision {
    /// 现在不说话（最常见、最重要的决策）/ Stay silent (most common, most important decision).
    StaySilent { reason: SilenceReason },
    /// 发起一个话题 / Initiate a topic.
    InitiateTopic {
        topic: TopicSuggestion,
        confidence: f32,
    },
    /// 关心一下用户 / Show care to the user.
    ShowCare {
        reason: CareReason,
        message_hint: String,
    },
    /// 提醒用户某件事 / Remind the user about something.
    Remind {
        event: ScheduledEvent,
        urgency: RemindUrgency,
    },
    /// 分享一个发现 / Share a discovery.
    ShareDiscovery { discovery: String, relevance: f32 },
    /// 表达想念 / Express longing for the user.
    ///
    /// 当用户长时间离开且想念强度超过阈值时触发，
    /// 受关系阶段和冷却时间调制。
    ExpressLonging {
        /// 想念强度 [0, 1] / Longing intensity
        intensity: f32,
        /// 消息提示 / Message hint for LLM generation
        hint: String,
    },
}

/// 提醒紧急度
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RemindUrgency {
    Low,
    Medium,
    High,
}

/// 关心原因
#[derive(Clone, Debug)]
pub enum CareReason {
    /// 用户情绪低落
    UserMoodDeclining,
    /// 长时间沉默
    LongSilence,
    /// 特殊日期
    SpecialDate,
    /// 一般性关心
    GeneralCheckIn,
}

/// 说话时机
#[derive(Clone, Debug)]
pub enum SpeakTiming {
    /// 好时机
    GoodTime { confidence: f32 },
    /// 还可以
    OkayTime { confidence: f32 },
    /// 现在不行
    NotNow(SilenceReason),
}

// ════════════════════════════════════════════════════════════════════
//  数据结构
// ════════════════════════════════════════════════════════════════════

/// 话题建议
#[derive(Clone, Debug)]
pub struct TopicSuggestion {
    pub kind: TopicKind,
    /// 给 LLM 的提示
    pub hint: String,
    /// 与当前上下文的相关度
    pub relevance: f32,
    /// 新鲜度（避免重复话题）
    pub novelty: f32,
}

/// 用户提过的未来事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub id: String,
    pub description: String,
    /// 如果有明确时间（Unix ms）
    pub scheduled_time: Option<i64>,
    /// 重要度 0..1
    pub importance: f32,
    /// 用户原话
    pub source_message: String,
    /// 是否已提醒
    pub reminded: bool,
    /// 创建时间
    pub created_at: i64,
}

/// 用户活动模式（简化版）
#[derive(Clone, Debug)]
pub struct UserActivityPattern {
    /// 用户平均回复间隔（秒）
    pub avg_reply_gap_secs: f64,
    /// 活跃时段（每小时一个 bool）
    pub active_hours: [bool; 24],
    /// 样本数
    pub sample_count: u64,
}

impl Default for UserActivityPattern {
    fn default() -> Self {
        Self {
            avg_reply_gap_secs: 60.0,
            active_hours: [true; 24],
            sample_count: 0,
        }
    }
}

impl UserActivityPattern {
    /// 判断用户在某个小时是否通常活跃
    pub fn is_active_at(&self, hour: f32) -> bool {
        let h = (hour as usize).min(23);
        self.active_hours[h]
    }
}

/// 主动决策上下文 — 汇总所有需要的信号
#[derive(Clone, Debug)]
pub struct ProactiveContext {
    /// AI 自身 arousal（来自情感引擎）
    pub ai_arousal: f32,
    /// AI 自身 pleasure
    pub ai_pleasure: f32,
    /// 距离上次用户消息的沉默时长
    pub silence_duration: Duration,
    /// 当前小时（0.0-23.99）
    pub current_hour: f32,
    /// 当前对话状态
    pub conversation_state: ConversationState,
    /// 待提醒事件数
    pub pending_reminders: usize,
    /// 关系阶段主动加成
    pub relationship_proactive_bonus: f32,
    /// 用户情绪 valence（-1..1，None 表示无数据）
    pub user_valence: Option<f32>,
    /// 用户参与度分数（0..1，None 表示无数据）
    pub user_engagement: Option<f32>,
    /// 用户消息长度（用于判断思考状态）
    pub user_avg_message_length: Option<f32>,
}

// ════════════════════════════════════════════════════════════════════
//  AwayDetector — 离开检测
// ════════════════════════════════════════════════════════════════════

/// 推断用户是否离开了对话
pub struct AwayDetector {
    /// 用户平均回复间隔的基线（从历史数据计算）
    baseline_gap: Duration,
}

impl AwayDetector {
    pub fn new(baseline_gap: Duration) -> Self {
        Self { baseline_gap }
    }

    /// 根据沉默时长判断对话状态
    pub fn detect(&self, silence_duration: Duration) -> ConversationState {
        let baseline = self.baseline_gap;

        // 沉默时间在正常回复间隔范围内 → 用户在思考
        if silence_duration < baseline * 2 {
            return ConversationState::Thinking { silence_duration };
        }

        // 沉默超过基线 2~5 倍且 < 10 分钟 → 可能短离开
        if silence_duration < baseline * 5 && silence_duration < Duration::from_secs(600) {
            return ConversationState::UserAway {
                away_since: silence_duration,
                away_reason: AwayReason::ShortBreak,
            };
        }

        // 沉默超过 10 分钟但 < 1 小时 → 可能去做别的事了
        if silence_duration < Duration::from_secs(3600) {
            return ConversationState::UserAway {
                away_since: silence_duration,
                away_reason: AwayReason::TaskSwitch,
            };
        }

        // 超过 1 小时 → 今天可能不会再来了
        ConversationState::UserAway {
            away_since: silence_duration,
            away_reason: AwayReason::DayEnded,
        }
    }

    /// 更新基线回复间隔
    pub fn update_baseline(&mut self, new_gap: Duration) {
        // EMA 平滑
        let new_ms = new_gap.as_millis() as f64;
        let old_ms = self.baseline_gap.as_millis() as f64;
        let smoothed = 0.2 * new_ms + 0.8 * old_ms;
        self.baseline_gap = Duration::from_millis(smoothed as u64);
    }
}

impl Default for AwayDetector {
    fn default() -> Self {
        Self {
            baseline_gap: Duration::from_secs(60),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
//  TimingJudge — 时机判断
// ════════════════════════════════════════════════════════════════════

/// 时机判断器 — 决定什么时候不该说话
#[derive(Default)]
pub struct TimingJudge {
    /// 用户行为模式
    user_pattern: UserActivityPattern,
}

impl TimingJudge {
    pub fn new(user_pattern: UserActivityPattern) -> Self {
        Self { user_pattern }
    }

    /// 判断现在是否适合主动说话
    pub fn should_speak(&self, ctx: &ProactiveContext) -> SpeakTiming {
        // ─── 规则 1: 用户正在思考，绝对不打扰 ───
        if matches!(ctx.conversation_state, ConversationState::Thinking { .. }) {
            return SpeakTiming::NotNow(SilenceReason::UserIsThinking);
        }

        // ─── 规则 2: 对话刚自然结束，给一段缓冲 ───
        if let ConversationState::Concluded { concluded_since } = &ctx.conversation_state {
            if *concluded_since < Duration::from_secs(300) {
                return SpeakTiming::NotNow(SilenceReason::JustConcluded);
            }
        }

        // ─── 规则 3: 用户短离开，等回来 ───
        if let ConversationState::UserAway {
            away_reason: AwayReason::ShortBreak,
            ..
        } = &ctx.conversation_state
        {
            return SpeakTiming::NotNow(SilenceReason::UserIsAway);
        }

        // ─── 规则 4: 深夜 + 用户通常此时不在 → 沉默 ───
        if ctx.current_hour >= 0.0
            && ctx.current_hour < 7.0
            && !self.user_pattern.is_active_at(ctx.current_hour)
        {
            return SpeakTiming::NotNow(SilenceReason::UserLikelySleeping);
        }

        // ─── 规则 6: 综合评分 ───
        let score = self.compute_readiness_score(ctx);

        if score > 0.7 {
            SpeakTiming::GoodTime { confidence: score }
        } else if score > 0.4 {
            SpeakTiming::OkayTime { confidence: score }
        } else {
            SpeakTiming::NotNow(SilenceReason::ScoreTooLow(score))
        }
    }

    /// 综合评分：0.0 = 完全不该说话，1.0 = 非常合适的时机
    fn compute_readiness_score(&self, ctx: &ProactiveContext) -> f32 {
        let mut score = 0.5; // 基础分

        // AI 自身状态（来自自主情感循环）
        // 当 AI arousal 低（"无聊"）时，稍微增加说话意愿
        if ctx.ai_arousal < -0.3 {
            score += 0.1;
        }

        // AI pleasure 高时更愿意主动交流
        if ctx.ai_pleasure > 0.3 {
            score += 0.05;
        }

        // 沉默时长（非线性）
        let silence_secs = ctx.silence_duration.as_secs();
        match silence_secs {
            0..=120 => score -= 0.3,       // 太短，不急
            121..=600 => score += 0.0,     // 适中
            601..=3600 => score += 0.15,   // 有点久了
            3601..=21600 => score += 0.25, // 很久了
            _ => score += 0.1,             // 太久了反而不该随便打破沉默
        }

        // 用户最近情绪
        if let Some(valence) = ctx.user_valence {
            if valence < -0.4 {
                // 用户心情不好，关心一下
                score += 0.2;
            } else if valence > 0.4 {
                // 用户心情好，聊起来更开心
                score += 0.1;
            }
        }

        // 待提醒事件
        if ctx.pending_reminders > 0 {
            score += 0.2;
        }

        // 关系阶段影响（深度关系可以更主动）
        score += ctx.relationship_proactive_bonus;

        // 用户参与度
        if let Some(engagement) = ctx.user_engagement {
            if engagement > 0.6 {
                score += 0.1; // 高参与度用户，多互动
            }
        }

        score.clamp(0.0, 1.0)
    }
}

// ════════════════════════════════════════════════════════════════════
//  SilenceBudget — 沉默预算
// ════════════════════════════════════════════════════════════════════

/// 沉默预算 — 不是"多久没说话就该说"，而是"沉默也有价值"
pub struct SilenceBudget {
    /// 当前沉默时长
    current_silence: Duration,
    /// "有价值的沉默"阈值（超过这个时间才有意义去打破）
    meaningful_threshold: Duration,
    /// "关心的沉默"阈值（用户心情不好时，沉默更久才去关心）
    care_threshold: Duration,
}

impl SilenceBudget {
    pub fn new(meaningful_threshold: Duration, care_threshold: Duration) -> Self {
        Self {
            current_silence: Duration::ZERO,
            meaningful_threshold,
            care_threshold,
        }
    }

    /// 更新当前沉默时长
    pub fn update(&mut self, silence: Duration) {
        self.current_silence = silence;
    }

    /// 沉默是否已经"有意义"（值得考虑打破）
    pub fn is_meaningful(&self) -> bool {
        self.current_silence >= self.meaningful_threshold
    }

    /// 是否应该关心用户（沉默超过关心阈值）
    pub fn should_care(&self) -> bool {
        self.current_silence >= self.care_threshold
    }

    /// 当前沉默时长
    pub fn current(&self) -> Duration {
        self.current_silence
    }
}

// ════════════════════════════════════════════════════════════════════
//  DecisionCooldown — 决策冷却
// ════════════════════════════════════════════════════════════════════

/// 决策冷却 — 防止频繁主动打扰
pub struct DecisionCooldown {
    /// 上次主动说话的时间
    last_proactive_at: Option<Instant>,
    /// 最小冷却时间
    min_cooldown: Duration,
    /// 冷却倍数（用户没回应 → 冷却翻倍）
    backoff_multiplier: f32,
    /// 最大冷却倍数
    max_backoff: f32,
}

impl DecisionCooldown {
    pub fn new(min_cooldown: Duration, backoff: f32, max_backoff: f32) -> Self {
        Self {
            last_proactive_at: None,
            min_cooldown,
            backoff_multiplier: backoff,
            max_backoff,
        }
    }

    /// 剩余冷却时间
    pub fn remaining(&self) -> Duration {
        match self.last_proactive_at {
            Some(last) => {
                let actual_cooldown = self
                    .min_cooldown
                    .mul_f32(self.backoff_multiplier)
                    .min(self.min_cooldown.mul_f32(self.max_backoff));
                let elapsed = last.elapsed();
                if elapsed >= actual_cooldown {
                    Duration::ZERO
                } else {
                    actual_cooldown - elapsed
                }
            }
            None => Duration::ZERO,
        }
    }

    /// 是否在冷却中
    pub fn is_active(&self) -> bool {
        self.remaining() > Duration::ZERO
    }

    /// 主动说话后调用
    pub fn record_proactive(&mut self, user_responded: bool) {
        self.last_proactive_at = Some(Instant::now());
        if !user_responded {
            // 用户没回应 → 冷却时间翻倍
            self.backoff_multiplier = (self.backoff_multiplier * 2.0).min(self.max_backoff);
        } else {
            // 用户回应了 → 恢复正常冷却
            self.backoff_multiplier = 1.0;
        }
    }

    /// 重置冷却状态
    pub fn reset(&mut self) {
        self.last_proactive_at = None;
        self.backoff_multiplier = 1.0;
    }
}

// ════════════════════════════════════════════════════════════════════
//  EventMemory — 事件记忆
// ════════════════════════════════════════════════════════════════════

/// 记住用户提过的未来事件，适时提醒
pub struct EventMemory {
    events: Vec<ScheduledEvent>,
    /// 事件计数器（用于生成唯一 ID）
    counter: u64,
}

impl EventMemory {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            counter: 0,
        }
    }

    /// 从用户消息中提取未来事件（关键词匹配）
    pub fn extract_events(&mut self, message: &str) -> Vec<ScheduledEvent> {
        let mut new_events = Vec::new();
        let now = Local::now().timestamp_millis();

        // 时间关键词 → 天数偏移
        let time_patterns: &[(&str, i64)] = &[
            ("明天", 1),
            ("后天", 2),
            ("大后天", 3),
            ("下周", 7),
            ("下个月", 30),
            ("今晚", 0),
            ("下午", 0),
            ("晚上", 0),
            ("deadline", 0),
            ("考试", 0),
            ("面试", 0),
            ("生日", 0),
            ("纪念", 0),
            ("提交", 0),
            ("截止", 0),
        ];

        for (keyword, day_offset) in time_patterns {
            if message.contains(keyword) {
                let desc = extract_event_description(message, keyword);
                if !desc.is_empty() {
                    let scheduled_time = if *day_offset > 0 {
                        Some(now + day_offset * 86_400_000)
                    } else {
                        None
                    };
                    self.counter += 1;
                    let event = ScheduledEvent {
                        id: format!("event-{}", self.counter),
                        description: desc,
                        scheduled_time,
                        importance: 0.5,
                        source_message: message.to_string(),
                        reminded: false,
                        created_at: now,
                    };
                    new_events.push(event.clone());
                    self.events.push(event);
                }
                // 一个消息可能匹配多个关键词，但只取第一个有效描述
                if !new_events.is_empty() {
                    break;
                }
            }
        }

        new_events
    }

    /// 获取当前应该提醒的事件
    pub fn pending_reminders(&self, current_time: i64) -> Vec<&ScheduledEvent> {
        self.events
            .iter()
            .filter(|e| {
                !e.reminded
                    && e.importance > 0.3
                    && match e.scheduled_time {
                        Some(time) => {
                            // 提前 2 小时提醒
                            let remind_at = time - 2 * 3600 * 1000;
                            current_time >= remind_at && current_time <= time
                        }
                        None => false,
                    }
            })
            .collect()
    }

    /// 标记事件为已提醒
    pub fn mark_reminded(&mut self, event_id: &str) {
        if let Some(event) = self.events.iter_mut().find(|e| e.id == event_id) {
            event.reminded = true;
        }
    }

    /// 清理已提醒且超过 7 天的事件
    pub fn cleanup(&mut self) {
        let now = Local::now().timestamp_millis();
        let cutoff = now - 7 * 86_400_000;
        self.events.retain(|e| !e.reminded || e.created_at > cutoff);
    }

    /// 事件总数
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// 从消息中提取事件描述（简化版）
fn extract_event_description(msg: &str, keyword: &str) -> String {
    if let Some(idx) = msg.find(keyword) {
        let after = &msg[idx..];
        // 取关键词后的内容，最多 50 个字符
        let desc: String = after
            .chars()
            .take(50)
            .take_while(|c| *c != '\n' && *c != '\r')
            .collect();
        let desc = desc.trim().to_string();
        if desc.len() >= 2 {
            return desc;
        }
    }
    String::new()
}

// ════════════════════════════════════════════════════════════════════
//  TopicSource trait + 内置话题源
// ════════════════════════════════════════════════════════════════════

/// 话题来源 trait
pub trait TopicSource: Send + Sync {
    /// 话题源名称
    fn name(&self) -> &str;

    /// 根据上下文建议一个话题
    fn suggest(&self, ctx: &ProactiveContext) -> Option<TopicSuggestion>;
}

/// 话题源 1: 未完成的讨论
pub struct UnfinishedTopicSource;

impl TopicSource for UnfinishedTopicSource {
    fn name(&self) -> &str {
        "unfinished"
    }

    fn suggest(&self, _ctx: &ProactiveContext) -> Option<TopicSuggestion> {
        // 简化版 — 从对话历史找未完成的话题 / Simplified — find unfinished topics from conversation history
        None
    }
}

/// 话题源 2: 基于用户兴趣
pub struct InterestTopicSource {
    /// 用户兴趣关键词（从 PreferenceManager 获取）
    interests: Vec<String>,
}

impl InterestTopicSource {
    pub fn new(interests: Vec<String>) -> Self {
        Self { interests }
    }

    pub fn update_interests(&mut self, interests: Vec<String>) {
        self.interests = interests;
    }
}

impl TopicSource for InterestTopicSource {
    fn name(&self) -> &str {
        "interest"
    }

    fn suggest(&self, _ctx: &ProactiveContext) -> Option<TopicSuggestion> {
        // 找到一个感兴趣的话题
        // 简化版 — 随机选一个兴趣 / Simplified — pick a random interest
        if self.interests.is_empty() {
            return None;
        }
        let idx = (Local::now().timestamp_millis() as usize) % self.interests.len();
        let interest = &self.interests[idx];
        Some(TopicSuggestion {
            kind: TopicKind::InterestBased,
            hint: format!("用户对 {} 感兴趣，可以聊聊这个话题", interest),
            relevance: 0.6,
            novelty: 0.7,
        })
    }
}

/// 话题源 3: AI 自己的"想法"
pub struct AiThoughtSource;

impl TopicSource for AiThoughtSource {
    fn name(&self) -> &str {
        "ai_thought"
    }

    fn suggest(&self, ctx: &ProactiveContext) -> Option<TopicSuggestion> {
        // 当 AI arousal 低（"无聊"）时，更可能产生自己的想法
        if ctx.ai_arousal < -0.2 {
            return Some(TopicSuggestion {
                kind: TopicKind::AiThought,
                hint: "我想到一个有趣的话题，基于最近的思考...".into(),
                relevance: 0.4,
                novelty: 0.9,
            });
        }
        None
    }
}

// ════════════════════════════════════════════════════════════════════
//  TopicSelector — 话题选择器
// ════════════════════════════════════════════════════════════════════

/// 话题选择器 — 从多个来源中选择最合适的话题
pub struct TopicSelector {
    sources: Vec<Box<dyn TopicSource>>,
}

impl TopicSelector {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// 注册话题源
    pub fn register(&mut self, source: Box<dyn TopicSource>) {
        self.sources.push(source);
    }

    /// 获取最佳话题建议
    pub fn select(&self, ctx: &ProactiveContext) -> Option<TopicSuggestion> {
        let mut best: Option<TopicSuggestion> = None;
        let mut best_score = 0.0;

        for source in &self.sources {
            if let Some(suggestion) = source.suggest(ctx) {
                let score = suggestion.relevance * 0.6 + suggestion.novelty * 0.4;
                if score > best_score {
                    best_score = score;
                    best = Some(suggestion);
                }
            }
        }

        best
    }

    /// 话题源数量
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

impl Default for TopicSelector {
    fn default() -> Self {
        let mut selector = Self::new();
        // 注册默认话题源
        selector.register(Box::new(UnfinishedTopicSource));
        selector.register(Box::new(InterestTopicSource::new(Vec::new())));
        selector.register(Box::new(AiThoughtSource));
        selector
    }
}

// ════════════════════════════════════════════════════════════════════
//  ProactiveEngine — 主引擎
// ════════════════════════════════════════════════════════════════════

/// 主动决策引擎
pub struct ProactiveEngine {
    /// 时机判断器
    timing: TimingJudge,
    /// 话题选择器
    topic_selector: TopicSelector,
    /// 事件记忆
    event_memory: EventMemory,
    /// 沉默预算
    silence_budget: SilenceBudget,
    /// 决策冷却
    cooldown: DecisionCooldown,
    /// 离开检测器
    away_detector: AwayDetector,
    /// 是否启用
    enabled: bool,
    /// 检查间隔（tick 数）
    check_interval_ticks: u64,
    /// 每天最多主动次数
    max_proactive_per_day: u32,
    /// 当天主动次数
    proactive_today: u32,
    /// 上次重置日期（用于每日计数重置）
    last_reset_day: u32,
    /// 上次交互时间
    last_interaction_at: Option<Instant>,
    /// 上次回复间隔记录（用于 AwayDetector 基线更新）
    recent_gaps: Vec<Duration>,
    /// 缓存的外部信号（由 Scheduler 注入，build_context 读取）
    /// Cached external signals (injected by Scheduler, read by build_context).
    cached_arousal: f32,
    cached_pleasure: f32,
    cached_relationship_bonus: f32,
    cached_user_valence: Option<f32>,
    cached_user_engagement: Option<f32>,
    cached_user_msg_length: Option<f32>,
}

impl ProactiveEngine {
    pub fn new(cfg: &ProactiveCfg) -> Self {
        let away_detector = AwayDetector::default();
        let timing = TimingJudge::default();
        let topic_selector = TopicSelector::default();
        let event_memory = EventMemory::new();
        let silence_budget = SilenceBudget::new(
            Duration::from_secs(cfg.silence_meaningful_threshold),
            Duration::from_secs(cfg.silence_care_threshold),
        );
        let cooldown = DecisionCooldown::new(
            Duration::from_secs(cfg.cooldown_min_seconds),
            cfg.cooldown_backoff,
            24.0,
        );

        Self {
            timing,
            topic_selector,
            event_memory,
            silence_budget,
            cooldown,
            away_detector,
            enabled: cfg.enabled,
            check_interval_ticks: cfg.check_interval_ticks,
            max_proactive_per_day: cfg.max_proactive_per_day,
            proactive_today: 0,
            last_reset_day: 0,
            last_interaction_at: None,
            recent_gaps: Vec::new(),
            cached_arousal: 0.0,
            cached_pleasure: 0.0,
            cached_relationship_bonus: 0.0,
            cached_user_valence: None,
            cached_user_engagement: None,
            cached_user_msg_length: None,
        }
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 检查间隔
    pub fn check_interval_ticks(&self) -> u64 {
        self.check_interval_ticks
    }

    /// 事件记忆的可变引用（供外部提取事件）
    pub fn event_memory_mut(&mut self) -> &mut EventMemory {
        &mut self.event_memory
    }

    /// 事件记忆的不可变引用
    pub fn event_memory(&self) -> &EventMemory {
        &self.event_memory
    }

    /// 记录一次主动行为（供 Scheduler 调用）
    /// - user_responded: 用户是否回应了这次主动行为
    pub fn record_proactive(&mut self, user_responded: bool) {
        self.cooldown.record_proactive(user_responded);
    }

    /// 话题选择器的可变引用（供外部注册话题源）
    pub fn topic_selector_mut(&mut self) -> &mut TopicSelector {
        &mut self.topic_selector
    }

    /// 记录用户消息（更新离开检测器基线 + 提取事件）
    pub fn on_user_message(&mut self, msg: &str) {
        let now = Instant::now();

        // 更新回复间隔基线
        if let Some(last) = self.last_interaction_at {
            let gap = now.duration_since(last);
            self.recent_gaps.push(gap);
            if self.recent_gaps.len() > 20 {
                self.recent_gaps.remove(0);
            }
            // 计算平均间隔并更新基线
            let avg_ms: f64 = self
                .recent_gaps
                .iter()
                .map(|d| d.as_millis() as f64)
                .sum::<f64>()
                / self.recent_gaps.len() as f64;
            self.away_detector
                .update_baseline(Duration::from_millis(avg_ms as u64));
        }

        self.last_interaction_at = Some(now);

        // 提取事件
        self.event_memory.extract_events(msg);

        // 用户回应了 → 重置冷却
        self.cooldown.record_proactive(true);

        // 重置每日计数
        self.maybe_reset_daily();
    }

    /// 构建决策上下文
    pub fn build_context(&self, silence_duration: Duration) -> ProactiveContext {
        let now = Local::now();
        let current_hour = now.hour() as f32 + now.minute() as f32 / 60.0;

        // 离开检测
        let conversation_state = self.away_detector.detect(silence_duration);

        // 更新沉默预算
        let mut silence_budget = SilenceBudget::new(
            self.silence_budget.meaningful_threshold,
            self.silence_budget.care_threshold,
        );
        silence_budget.update(silence_duration);

        let pending_reminders = self
            .event_memory
            .pending_reminders(now.timestamp_millis())
            .len();

        ProactiveContext {
            ai_arousal: self.cached_arousal,
            ai_pleasure: self.cached_pleasure,
            silence_duration,
            current_hour,
            conversation_state,
            pending_reminders,
            relationship_proactive_bonus: self.cached_relationship_bonus,
            user_valence: self.cached_user_valence,
            user_engagement: self.cached_user_engagement,
            user_avg_message_length: self.cached_user_msg_length,
        }
    }

    /// 主决策入口
    pub fn decide(&mut self, ctx: &ProactiveContext) -> ProactiveDecision {
        if !self.enabled {
            return ProactiveDecision::StaySilent {
                reason: SilenceReason::ScoreTooLow(0.0),
            };
        }

        // 每日限制检查
        self.maybe_reset_daily();
        if self.proactive_today >= self.max_proactive_per_day {
            return ProactiveDecision::StaySilent {
                reason: SilenceReason::ScoreTooLow(0.0),
            };
        }

        // 冷却检查
        if self.cooldown.is_active() {
            return ProactiveDecision::StaySilent {
                reason: SilenceReason::CooldownActive,
            };
        }

        // 沉默预算检查
        if !self.silence_budget.is_meaningful() {
            return ProactiveDecision::StaySilent {
                reason: SilenceReason::ScoreTooLow(0.0),
            };
        }

        // 时机判断
        let timing = self.timing.should_speak(ctx);
        match timing {
            SpeakTiming::NotNow(reason) => {
                return ProactiveDecision::StaySilent { reason };
            }
            SpeakTiming::OkayTime { .. } | SpeakTiming::GoodTime { .. } => {
                // 继续选择话题
            }
        }

        // 先检查是否有待提醒
        if ctx.pending_reminders > 0 {
            let now = Local::now().timestamp_millis();
            let pending = self.event_memory.pending_reminders(now);
            if let Some(event) = pending.first() {
                let urgency = if event.importance > 0.7 {
                    RemindUrgency::High
                } else if event.importance > 0.4 {
                    RemindUrgency::Medium
                } else {
                    RemindUrgency::Low
                };
                let event = (*event).clone();
                self.event_memory.mark_reminded(&event.id);
                self.record_decision();
                return ProactiveDecision::Remind { event, urgency };
            }
        }

        // 沉默超过关心阈值 → 关心用户
        if self.silence_budget.should_care() {
            if let Some(valence) = ctx.user_valence {
                if valence < -0.3 {
                    self.record_decision();
                    return ProactiveDecision::ShowCare {
                        reason: CareReason::UserMoodDeclining,
                        message_hint: "用户情绪低落，已沉默一段时间，表达关心".into(),
                    };
                }
            }
            self.record_decision();
            return ProactiveDecision::ShowCare {
                reason: CareReason::LongSilence,
                message_hint: "用户已沉默较长时间，温和地关心一下近况".into(),
            };
        }

        // 话题选择
        if let Some(topic) = self.topic_selector.select(ctx) {
            let confidence = match timing {
                SpeakTiming::GoodTime { confidence } => confidence,
                SpeakTiming::OkayTime { confidence } => confidence,
                _ => 0.5,
            };
            self.record_decision();
            return ProactiveDecision::InitiateTopic { topic, confidence };
        }

        // 没有合适的话题 → 保持沉默
        ProactiveDecision::StaySilent {
            reason: SilenceReason::ScoreTooLow(0.3),
        }
    }

    /// 记录一次主动决策
    fn record_decision(&mut self) {
        self.proactive_today += 1;
        self.cooldown.record_proactive(false);
    }

    /// 每日计数重置
    fn maybe_reset_daily(&mut self) {
        let today = Local::now().ordinal();
        if today != self.last_reset_day {
            self.proactive_today = 0;
            self.last_reset_day = today;
        }
    }

    /// 更新 AI 情感状态（供 Scheduler 在构建上下文前调用）
    pub fn update_emotion(&mut self, arousal: f32, pleasure: f32) {
        self.cached_arousal = arousal;
        self.cached_pleasure = pleasure;
    }

    /// 更新用户模型数据
    pub fn update_user_model(
        &mut self,
        valence: Option<f32>,
        engagement: Option<f32>,
        msg_length: Option<f32>,
    ) {
        self.cached_user_valence = valence;
        self.cached_user_engagement = engagement;
        self.cached_user_msg_length = msg_length;
    }

    /// 更新关系阶段加成
    pub fn update_relationship_bonus(&mut self, bonus: f32) {
        self.cached_relationship_bonus = bonus;
    }
}

// ════════════════════════════════════════════════════════════════════
//  测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx_thinking() -> ProactiveContext {
        ProactiveContext {
            ai_arousal: 0.0,
            ai_pleasure: 0.0,
            silence_duration: Duration::from_secs(30),
            current_hour: 14.0,
            conversation_state: ConversationState::Thinking {
                silence_duration: Duration::from_secs(30),
            },
            pending_reminders: 0,
            relationship_proactive_bonus: 0.0,
            user_valence: None,
            user_engagement: None,
            user_avg_message_length: None,
        }
    }

    fn make_ctx_concluded() -> ProactiveContext {
        ProactiveContext {
            ai_arousal: 0.0,
            ai_pleasure: 0.0,
            silence_duration: Duration::from_secs(60),
            current_hour: 14.0,
            conversation_state: ConversationState::Concluded {
                concluded_since: Duration::from_secs(60),
            },
            pending_reminders: 0,
            relationship_proactive_bonus: 0.0,
            user_valence: None,
            user_engagement: None,
            user_avg_message_length: None,
        }
    }

    fn make_ctx_normal() -> ProactiveContext {
        ProactiveContext {
            ai_arousal: 0.0,
            ai_pleasure: 0.0,
            silence_duration: Duration::from_secs(600),
            current_hour: 14.0,
            conversation_state: ConversationState::Active {
                last_message_ago: Duration::from_secs(600),
                momentum: 0.3,
            },
            pending_reminders: 0,
            relationship_proactive_bonus: 0.0,
            user_valence: None,
            user_engagement: None,
            user_avg_message_length: None,
        }
    }

    // ─── TimingJudge 测试 ───

    #[test]
    fn test_timing_judge_user_thinking() {
        let judge = TimingJudge::default();
        let ctx = make_ctx_thinking();
        let result = judge.should_speak(&ctx);
        assert!(matches!(
            result,
            SpeakTiming::NotNow(SilenceReason::UserIsThinking)
        ));
    }

    #[test]
    fn test_timing_judge_just_concluded() {
        let judge = TimingJudge::default();
        let ctx = make_ctx_concluded();
        let result = judge.should_speak(&ctx);
        assert!(matches!(
            result,
            SpeakTiming::NotNow(SilenceReason::JustConcluded)
        ));
    }

    #[test]
    fn test_timing_judge_night_sleeping() {
        let _judge = TimingJudge::default();
        let mut pattern = UserActivityPattern::default();
        // 标记凌晨 3 点不活跃
        pattern.active_hours[3] = false;
        let night_judge = TimingJudge::new(pattern);

        let ctx = ProactiveContext {
            current_hour: 3.0,
            conversation_state: ConversationState::Active {
                last_message_ago: Duration::from_secs(600),
                momentum: 0.3,
            },
            ..make_ctx_normal()
        };
        let result = night_judge.should_speak(&ctx);
        assert!(matches!(
            result,
            SpeakTiming::NotNow(SilenceReason::UserLikelySleeping)
        ));
    }

    #[test]
    fn test_timing_judge_good_time() {
        let judge = TimingJudge::default();
        let ctx = ProactiveContext {
            silence_duration: Duration::from_secs(7200), // 2 小时
            user_valence: Some(-0.5),                    // 情绪低落
            pending_reminders: 2,
            relationship_proactive_bonus: 0.1,
            user_engagement: Some(0.8),
            ..make_ctx_normal()
        };
        let result = judge.should_speak(&ctx);
        assert!(matches!(result, SpeakTiming::GoodTime { .. }));
    }

    // ─── AwayDetector 测试 ───

    #[test]
    fn test_away_detector_thinking() {
        let detector = AwayDetector::new(Duration::from_secs(60));
        let state = detector.detect(Duration::from_secs(30));
        assert!(matches!(state, ConversationState::Thinking { .. }));
    }

    #[test]
    fn test_away_detector_short_break() {
        let detector = AwayDetector::new(Duration::from_secs(60));
        let state = detector.detect(Duration::from_secs(180));
        assert!(matches!(
            state,
            ConversationState::UserAway {
                away_reason: AwayReason::ShortBreak,
                ..
            }
        ));
    }

    #[test]
    fn test_away_detector_task_switch() {
        let detector = AwayDetector::new(Duration::from_secs(60));
        let state = detector.detect(Duration::from_secs(900)); // 15 分钟
        assert!(matches!(
            state,
            ConversationState::UserAway {
                away_reason: AwayReason::TaskSwitch,
                ..
            }
        ));
    }

    #[test]
    fn test_away_detector_day_ended() {
        let detector = AwayDetector::new(Duration::from_secs(60));
        let state = detector.detect(Duration::from_secs(7200)); // 2 小时
        assert!(matches!(
            state,
            ConversationState::UserAway {
                away_reason: AwayReason::DayEnded,
                ..
            }
        ));
    }

    // ─── EventMemory 测试 ───

    #[test]
    fn test_event_memory_extract_tomorrow() {
        let mut memory = EventMemory::new();
        let events = memory.extract_events("我明天有个重要的面试");
        assert_eq!(events.len(), 1);
        assert!(events[0].description.contains("明天"));
        assert!(events[0].scheduled_time.is_some());
    }

    #[test]
    fn test_event_memory_extract_deadline() {
        let mut memory = EventMemory::new();
        let events = memory.extract_events("这周五是 deadline，要提交报告");
        assert_eq!(events.len(), 1);
        assert!(events[0].description.contains("deadline"));
    }

    #[test]
    fn test_event_memory_no_event() {
        let mut memory = EventMemory::new();
        let events = memory.extract_events("今天天气真好");
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_memory_pending_reminders() {
        let mut memory = EventMemory::new();
        let now = Local::now().timestamp_millis();
        let event = ScheduledEvent {
            id: "test-1".into(),
            description: "明天的面试".into(),
            scheduled_time: Some(now + 3600 * 1000), // 1 小时后
            importance: 0.8,
            source_message: "明天有面试".into(),
            reminded: false,
            created_at: now,
        };
        memory.events.push(event);
        let pending = memory.pending_reminders(now);
        assert_eq!(pending.len(), 1);
    }

    // ─── DecisionCooldown 测试 ───

    #[test]
    fn test_cooldown_initial() {
        let cooldown = DecisionCooldown::new(Duration::from_secs(600), 2.0, 24.0);
        assert!(!cooldown.is_active());
    }

    #[test]
    fn test_cooldown_after_proactive() {
        let mut cooldown = DecisionCooldown::new(Duration::from_secs(600), 2.0, 24.0);
        cooldown.record_proactive(false);
        assert!(cooldown.is_active());
    }

    #[test]
    fn test_cooldown_backoff() {
        let mut cooldown = DecisionCooldown::new(Duration::from_secs(1), 2.0, 24.0);
        // 第一次无回应
        cooldown.record_proactive(false);
        assert!(cooldown.backoff_multiplier > 1.0);
        // 第二次无回应
        cooldown.record_proactive(false);
        assert!(cooldown.backoff_multiplier >= 4.0);
    }

    #[test]
    fn test_cooldown_reset_on_response() {
        let mut cooldown = DecisionCooldown::new(Duration::from_secs(1), 2.0, 24.0);
        cooldown.record_proactive(false);
        cooldown.record_proactive(false);
        // 用户回应
        cooldown.record_proactive(true);
        assert!((cooldown.backoff_multiplier - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cooldown_reset() {
        let mut cooldown = DecisionCooldown::new(Duration::from_secs(600), 2.0, 24.0);
        cooldown.record_proactive(false);
        cooldown.reset();
        assert!(!cooldown.is_active());
    }

    // ─── SilenceBudget 测试 ───

    #[test]
    fn test_silence_budget_meaningful() {
        let mut budget = SilenceBudget::new(Duration::from_secs(300), Duration::from_secs(1800));
        budget.update(Duration::from_secs(100));
        assert!(!budget.is_meaningful());
        budget.update(Duration::from_secs(400));
        assert!(budget.is_meaningful());
    }

    #[test]
    fn test_silence_budget_care() {
        let mut budget = SilenceBudget::new(Duration::from_secs(300), Duration::from_secs(1800));
        budget.update(Duration::from_secs(1000));
        assert!(!budget.should_care());
        budget.update(Duration::from_secs(2000));
        assert!(budget.should_care());
    }

    // ─── ProactiveEngine 集成测试 ───

    #[test]
    fn test_engine_stay_silent_when_disabled() {
        let cfg = ProactiveCfg {
            enabled: false,
            ..Default::default()
        };
        let mut engine = ProactiveEngine::new(&cfg);
        let ctx = make_ctx_normal();
        let decision = engine.decide(&ctx);
        assert!(matches!(decision, ProactiveDecision::StaySilent { .. }));
    }

    #[test]
    fn test_engine_stay_silent_when_cooldown() {
        let cfg = ProactiveCfg {
            enabled: true,
            cooldown_min_seconds: 3600,
            ..Default::default()
        };
        let mut engine = ProactiveEngine::new(&cfg);
        // 记录一次主动行为来触发冷却
        engine.cooldown.record_proactive(false);
        let ctx = make_ctx_normal();
        let decision = engine.decide(&ctx);
        assert!(matches!(decision, ProactiveDecision::StaySilent { .. }));
    }
}
