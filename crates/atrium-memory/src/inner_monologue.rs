// SPDX-License-Identifier: MIT
//! 内在独白引擎 — AI 独处时的自主思考系统
//!
//! Inner monologue engine — Autonomous thinking system for when the AI is alone.
//!
//! 包含五种思考模式：
//!   A. GraphWander — 沿关联图漫游，激活记忆路径
//!   B. DiaryEntry — 深夜书写数字日记
//!   C. AutonomousLearning — 自主阅读 ACK 知识库
//!   D. Daydream — 深夜白日梦，记忆碎片随机重组
//!   E. PostConsolidation — 记忆巩固后的反思
//!
//! 引擎本身只管理状态（冷却、计数、环形缓冲区），实际 LLM 调用和图操作
//! 在 CoreService 的异步方法中完成，确保不阻塞 Scheduler tick。
//!
//! Five thought modes:
//!   A. GraphWander — wander the associative graph, activating memory paths
//!   B. DiaryEntry — write a digital diary entry late at night
//!   C. AutonomousLearning — read the ACK knowledge base autonomously
//!   D. Daydream — recombine memory fragments during deep night
//!   E. PostConsolidation — reflect after memory consolidation
//!
//! The engine itself only manages state (cooldowns, counters, ring buffer).
//! Actual LLM calls and graph operations happen in async CoreService methods,
//! ensuring the Scheduler tick is never blocked.

use crate::maturity::EmotionContext;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ── 思考模式枚举 / Thought Mode Enum ──

/// 思考模式 — 五种独处思考类型
/// Thought mode — Five types of idle thinking.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThoughtMode {
    /// 图漫游 — 沿关联图路径激活记忆 / Graph wander — following associative paths.
    GraphWander,
    /// 日记 — 深夜数字日记 / Diary entry — nightly digital diary.
    DiaryEntry,
    /// 自主学习 — 阅读 ACK 知识 / Autonomous learning from ACK knowledge base.
    AutonomousLearning,
    /// 白日梦 — 记忆碎片随机重组 / Daydream — random memory fragment recombination.
    Daydream,
    /// 巩固反思 — 记忆整理后的反思 / Post-consolidation reflection.
    PostConsolidation,
}

impl ThoughtMode {
    /// 转为可读标签（用于 prompt 拼接）
    /// Convert to a human-readable label for prompt assembly.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::GraphWander => "graph_wander",
            Self::DiaryEntry => "diary",
            Self::AutonomousLearning => "learning",
            Self::Daydream => "daydream",
            Self::PostConsolidation => "reflection",
        }
    }
}

// ── 内在思考结构 / Inner Thought Struct ──

/// 内在思考 — 单条独白记录
/// Inner thought — A single monologue record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerThought {
    /// 思考内容 / The thought text.
    pub content: String,
    /// 思考模式 / Which mode produced this thought.
    pub mode: ThoughtMode,
    /// 置信度 (0.0-1.0，白日梦低) / Confidence (0.0-1.0; daydreams are low).
    pub confidence: f64,
    /// 生成时的情感上下文 / Emotional context when the thought was formed.
    pub emotion: Option<EmotionContext>,
    /// 时间戳 / Epoch-second timestamp.
    pub timestamp: i64,
    /// 是否可分享给用户 / Whether this thought can be shared with the user.
    pub shareable: bool,
    /// 关联图种子节点（GraphWander 模式）/ Graph seed node (GraphWander mode only).
    pub graph_seed: Option<String>,
}

// ── 配置结构 / Config Struct ──

/// 内在独白配置 — 各模式的间隔、上限和时段
/// Inner monologue configuration — intervals, limits, and time windows per mode.
#[derive(Clone, Debug)]
pub struct InnerMonologueConfig {
    /// 最大思考缓冲区 / Max thoughts retained in the ring buffer.
    pub max_thoughts: usize,
    /// 每日思考总上限 / Total thoughts per day across all modes.
    pub max_per_day: u32,
    // ── GraphWander ──
    /// 图漫游最小间隔（秒）/ Minimum seconds between graph wanders.
    pub graph_wander_interval_secs: i64,
    /// 图漫游每日上限 / Max graph wanders per day.
    pub graph_wander_max_per_day: u32,
    /// 激活衰减率 / Spread activation decay rate.
    pub graph_wander_decay_rate: f64,
    /// 最大跳数 / Maximum spread activation hops.
    pub graph_wander_max_hops: usize,
    // ── AutonomousLearning ──
    /// 学习最小间隔（秒）/ Minimum seconds between learning sessions.
    pub learning_interval_secs: i64,
    /// 学习每日上限 / Max learning sessions per day.
    pub learning_max_per_day: u32,
    // ── Daydream ──
    /// 白日梦最小间隔（秒）/ Minimum seconds between daydreams.
    pub daydream_interval_secs: i64,
    /// 白日梦置信度 / Confidence value for daydream thoughts.
    pub daydream_confidence: f64,
}

impl Default for InnerMonologueConfig {
    fn default() -> Self {
        Self {
            max_thoughts: 200,
            max_per_day: 30,
            graph_wander_interval_secs: 300,
            graph_wander_max_per_day: 20,
            graph_wander_decay_rate: 0.6,
            graph_wander_max_hops: 3,
            learning_interval_secs: 1800,
            learning_max_per_day: 5,
            daydream_interval_secs: 1800,
            daydream_confidence: 0.3,
        }
    }
}

// ── 引擎主体 / Engine ──

/// 内在独白引擎 — 状态管理 + 环形缓冲区
/// Inner monologue engine — State manager plus ring buffer.
pub struct InnerMonologueEngine {
    /// 思考历史（环形缓冲区）/ Thought ring buffer.
    thoughts: VecDeque<InnerThought>,
    /// 最大思考数 / Ring buffer capacity.
    max_thoughts: usize,
    /// 上次图漫游时间 / Last graph wander timestamp.
    last_graph_wander: i64,
    /// 上次日记时间 / Last diary entry timestamp.
    last_diary_entry: i64,
    /// 上次自主学习时间 / Last autonomous learning timestamp.
    last_autonomous_learning: i64,
    /// 上次白日梦时间 / Last daydream timestamp.
    last_daydream: i64,
    /// 每日思考计数 / Thoughts generated today.
    thoughts_today: u32,
    /// 上次重置的日期序号 / Day-of-year of last daily reset.
    last_reset_day: u32,
    /// 各模式今日计数 / Per-mode daily counters.
    graph_wander_today: u32,
    learning_today: u32,
    daydream_today: u32,
    /// 配置 / Configuration.
    config: InnerMonologueConfig,
}

impl InnerMonologueEngine {
    /// 创建引擎 / Create a new engine with the given config.
    pub fn new(config: InnerMonologueConfig) -> Self {
        let max_thoughts = config.max_thoughts;
        Self {
            thoughts: VecDeque::with_capacity(max_thoughts),
            max_thoughts,
            last_graph_wander: 0,
            last_diary_entry: 0,
            last_autonomous_learning: 0,
            last_daydream: 0,
            thoughts_today: 0,
            last_reset_day: Self::day_of_year(),
            graph_wander_today: 0,
            learning_today: 0,
            daydream_today: 0,
            config,
        }
    }

    /// 使用默认配置创建 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(InnerMonologueConfig::default())
    }

    // ── 每日重置 / Daily Reset ──

    /// 检查并执行每日重置 — 跨天时清零计数器
    /// Check and perform daily reset — zero counters when the calendar day changes.
    pub fn check_daily_reset(&mut self) {
        let today = Self::day_of_year();
        if today != self.last_reset_day {
            self.thoughts_today = 0;
            self.graph_wander_today = 0;
            self.learning_today = 0;
            self.daydream_today = 0;
            self.last_reset_day = today;
        }
    }

    /// 获取当年第几天（用于跨天检测）
    /// Get the day-of-year number (for day-rollover detection).
    fn day_of_year() -> u32 {
        chrono::Local::now().ordinal()
    }

    // ── 模式冷却检查 / Mode Cooldown Checks ──

    /// 图漫游是否可触发 — 检查冷却间隔和每日上限
    /// Whether graph wander can trigger now — checks cooldown interval and daily limit.
    pub fn can_graph_wander(&self, now: i64) -> bool {
        if self.graph_wander_today >= self.config.graph_wander_max_per_day {
            return false;
        }
        if self.thoughts_today >= self.config.max_per_day {
            return false;
        }
        now - self.last_graph_wander >= self.config.graph_wander_interval_secs
    }

    /// 自主学习是否可触发 — 检查冷却间隔和每日上限
    /// Whether autonomous learning can trigger now.
    pub fn can_learn(&self, now: i64) -> bool {
        if self.learning_today >= self.config.learning_max_per_day {
            return false;
        }
        if self.thoughts_today >= self.config.max_per_day {
            return false;
        }
        now - self.last_autonomous_learning >= self.config.learning_interval_secs
    }

    /// 白日梦是否可触发 — 检查冷却间隔和每日总上限
    /// Whether daydream can trigger now.
    pub fn can_daydream(&self, now: i64) -> bool {
        if self.thoughts_today >= self.config.max_per_day {
            return false;
        }
        now - self.last_daydream >= self.config.daydream_interval_secs
    }

    // ── 模式触发记录 / Mode Trigger Recording ──

    /// 记录一次图漫游触发
    /// Record that a graph wander was triggered.
    pub fn record_graph_wander(&mut self, now: i64) {
        self.last_graph_wander = now;
        self.graph_wander_today += 1;
    }

    /// 记录一次日记写入
    /// Record that a diary entry was written.
    pub fn record_diary(&mut self, now: i64) {
        self.last_diary_entry = now;
    }

    /// 记录一次自主学习
    /// Record that an autonomous learning session occurred.
    pub fn record_learning(&mut self, now: i64) {
        self.last_autonomous_learning = now;
        self.learning_today += 1;
    }

    /// 记录一次白日梦
    /// Record that a daydream occurred.
    pub fn record_daydream(&mut self, now: i64) {
        self.last_daydream = now;
        self.daydream_today += 1;
    }

    // ── 思考缓冲区 / Thought Buffer ──

    /// 添加一条思考到环形缓冲区（满时挤出最旧条目）
    /// Add a thought to the ring buffer; evicts the oldest entry when full.
    pub fn add_thought(&mut self, thought: InnerThought) {
        if self.thoughts.len() >= self.max_thoughts {
            self.thoughts.pop_front();
        }
        self.thoughts.push_back(thought);
        self.thoughts_today += 1;
    }

    /// 获取最近 N 条思考（从新到旧）
    /// Get the most recent N thoughts, ordered newest-first.
    pub fn recent_thoughts(&self, n: usize) -> Vec<&InnerThought> {
        let n = n.min(self.thoughts.len());
        self.thoughts.iter().rev().take(n).collect()
    }

    /// 获取可分享的思考（用于自然融入对话）
    /// Get shareable thoughts (for natural integration into conversation).
    pub fn shareable_thoughts(&self, n: usize) -> Vec<&InnerThought> {
        self.thoughts
            .iter()
            .rev()
            .filter(|t| t.shareable)
            .take(n)
            .collect()
    }

    /// 当前缓冲区长度
    /// Current number of thoughts in the buffer.
    pub fn thought_count(&self) -> usize {
        self.thoughts.len()
    }

    /// 今日思考总数
    /// Total thoughts generated today.
    pub fn thoughts_today(&self) -> u32 {
        self.thoughts_today
    }

    /// 今日图漫游次数
    /// Graph wander count today.
    pub fn graph_wander_today(&self) -> u32 {
        self.graph_wander_today
    }

    /// 今日学习次数
    /// Learning session count today.
    pub fn learning_today(&self) -> u32 {
        self.learning_today
    }

    /// 今日白日梦次数
    /// Daydream count today.
    pub fn daydream_today(&self) -> u32 {
        self.daydream_today
    }

    /// 获取配置引用
    /// Get a reference to the config.
    pub fn config(&self) -> &InnerMonologueConfig {
        &self.config
    }
}

// ── 情感分析 / Emotion Analysis ──

/// 思考内容情感推断 — 关键词匹配，返回 PAD delta
/// Infer emotion delta from thought content via keyword matching.
///
/// 轻量级情感反馈，不经过 LLM。正面词 → pleasure +，负面词 → pleasure -，
/// 矛盾词 → arousal +。delta 幅度很小（±0.05），只作为微调。
///
/// Lightweight emotion feedback without LLM. Positive words → pleasure up,
/// negative words → pleasure down, conflict words → arousal up.
/// Delta magnitude is small (±0.05) — this is a micro-adjustment only.
pub fn analyze_thought_emotion(content: &str) -> EmotionContext {
    let mut pleasure: f32 = 0.0;
    let mut arousal: f32 = 0.0;
    let mut dominance: f32 = 0.0;

    // 正面关键词 / Positive keywords
    const POSITIVE: &[&str] = &[
        "开心",
        "高兴",
        "快乐",
        "喜欢",
        "期待",
        "温暖",
        "幸福",
        "满足",
        "笑",
        "好",
        "棒",
        "厉害",
        "厉害",
        "惊喜",
        "美好",
        "想念",
        "happy",
        "joy",
        "love",
        "excited",
        "wonderful",
        "great",
    ];

    // 负面关键词 / Negative keywords
    const NEGATIVE: &[&str] = &[
        "难过",
        "伤心",
        "孤独",
        "害怕",
        "担心",
        "焦虑",
        "无聊",
        "累",
        "烦",
        "痛",
        "哭",
        "失望",
        "遗憾",
        "冷",
        "sad",
        "lonely",
        "afraid",
        "worried",
        "tired",
        "disappointed",
    ];

    // 矛盾/冲突关键词 / Conflict keywords (raise arousal)
    const CONFLICT: &[&str] = &[
        "矛盾",
        "冲突",
        "不对",
        "但是",
        "可是",
        "然而",
        "纠结",
        "contradiction",
        "conflict",
        "but",
        "however",
    ];

    for kw in POSITIVE {
        if content.contains(kw) {
            pleasure += 0.03;
        }
    }
    for kw in NEGATIVE {
        if content.contains(kw) {
            pleasure -= 0.03;
            arousal += 0.01;
        }
    }
    for kw in CONFLICT {
        if content.contains(kw) {
            arousal += 0.02;
            pleasure -= 0.01;
        }
    }

    // 钳制到 [-0.05, 0.05] / Clamp to [-0.05, 0.05]
    pleasure = pleasure.clamp(-0.05, 0.05);
    arousal = arousal.clamp(-0.03, 0.05);
    dominance = dominance.clamp(-0.02, 0.02);

    EmotionContext {
        pleasure,
        arousal,
        dominance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_thought(content: &str, mode: ThoughtMode, shareable: bool) -> InnerThought {
        InnerThought {
            content: content.to_string(),
            mode,
            confidence: 0.7,
            emotion: None,
            timestamp: 1_700_000_000,
            shareable,
            graph_seed: None,
        }
    }

    // ── 基础功能测试 / Basic Functionality Tests ──

    #[test]
    fn test_add_and_count_thoughts() {
        let mut engine = InnerMonologueEngine::default_new();
        assert_eq!(engine.thought_count(), 0);

        engine.add_thought(make_thought(
            "想到一个有趣的事",
            ThoughtMode::GraphWander,
            true,
        ));
        engine.add_thought(make_thought(
            "今天和主人聊了很多",
            ThoughtMode::DiaryEntry,
            false,
        ));
        assert_eq!(engine.thought_count(), 2);
        assert_eq!(engine.thoughts_today(), 2);
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let config = InnerMonologueConfig {
            max_thoughts: 3,
            ..Default::default()
        };
        let mut engine = InnerMonologueEngine::new(config);

        for i in 0..5 {
            engine.add_thought(make_thought(
                &format!("thought {}", i),
                ThoughtMode::GraphWander,
                true,
            ));
        }
        // 缓冲区上限为 3，应挤出旧的 / Buffer cap is 3, oldest evicted
        assert_eq!(engine.thought_count(), 3);
        let recent = engine.recent_thoughts(3);
        assert!(recent[0].content.contains('4'));
        assert!(recent[2].content.contains('2'));
    }

    #[test]
    fn test_shareable_filter() {
        let mut engine = InnerMonologueEngine::default_new();
        engine.add_thought(make_thought("可分享的", ThoughtMode::GraphWander, true));
        engine.add_thought(make_thought("私密日记", ThoughtMode::DiaryEntry, false));
        engine.add_thought(make_thought(
            "也可分享",
            ThoughtMode::PostConsolidation,
            true,
        ));

        let shareable = engine.shareable_thoughts(10);
        assert_eq!(shareable.len(), 2);
        assert!(shareable.iter().all(|t| t.shareable));
    }

    // ── 冷却和限制测试 / Cooldown and Limit Tests ──

    #[test]
    fn test_graph_wander_cooldown() {
        let mut engine = InnerMonologueEngine::default_new();
        // 首次应可触发 / First trigger should pass
        assert!(engine.can_graph_wander(1000));
        engine.record_graph_wander(1000);
        // 间隔内不可触发 / Within cooldown, cannot trigger
        assert!(!engine.can_graph_wander(1100));
        // 超过间隔后可触发 / After interval, can trigger
        assert!(engine.can_graph_wander(1000 + 301));
    }

    #[test]
    fn test_graph_wander_daily_limit() {
        let config = InnerMonologueConfig {
            graph_wander_max_per_day: 2,
            ..Default::default()
        };
        let mut engine = InnerMonologueEngine::new(config);
        let now = 1000_i64;

        engine.record_graph_wander(now);
        assert!(engine.can_graph_wander(now + 301));
        engine.record_graph_wander(now + 301);
        // 达到每日上限 / Daily limit reached
        assert!(!engine.can_graph_wander(now + 602));
        assert_eq!(engine.graph_wander_today(), 2);
    }

    #[test]
    fn test_learning_cooldown_and_limit() {
        let config = InnerMonologueConfig {
            learning_max_per_day: 1,
            ..Default::default()
        };
        let mut engine = InnerMonologueEngine::new(config);
        let now = 2000_i64;

        assert!(engine.can_learn(now));
        engine.record_learning(now);
        assert!(!engine.can_learn(now + 1801)); // 每日上限 / daily limit
        assert_eq!(engine.learning_today(), 1);
    }

    #[test]
    fn test_daydream_cooldown() {
        let mut engine = InnerMonologueEngine::default_new();
        let now = 3000_i64;

        assert!(engine.can_daydream(now));
        engine.record_daydream(now);
        assert!(!engine.can_daydream(now + 100));
        assert!(engine.can_daydream(now + 1801));
    }

    // ── 每日重置测试 / Daily Reset Tests ──

    #[test]
    fn test_daily_reset_clears_counters() {
        let mut engine = InnerMonologueEngine::default_new();
        engine.record_graph_wander(1000);
        engine.record_learning(2000);
        engine.record_daydream(3000);
        engine.add_thought(make_thought("thought", ThoughtMode::GraphWander, true));

        assert!(engine.graph_wander_today() > 0);
        assert!(engine.learning_today() > 0);
        assert!(engine.thoughts_today() > 0);

        // 手动修改 last_reset_day 模拟跨天 / Simulate day rollover
        engine.last_reset_day = 0;
        engine.check_daily_reset();

        assert_eq!(engine.graph_wander_today(), 0);
        assert_eq!(engine.learning_today(), 0);
        assert_eq!(engine.daydream_today(), 0);
        assert_eq!(engine.thoughts_today(), 0);
        // 缓冲区不清空 — 只清计数 / Buffer not cleared, only counters
        assert_eq!(engine.thought_count(), 1);
    }

    #[test]
    fn test_daily_reset_no_op_same_day() {
        let mut engine = InnerMonologueEngine::default_new();
        engine.record_graph_wander(1000);
        let before = engine.graph_wander_today();
        engine.check_daily_reset(); // 同一天不应重置 / Same day, no reset
        assert_eq!(engine.graph_wander_today(), before);
    }

    // ── 情感分析测试 / Emotion Analysis Tests ──

    #[test]
    fn test_positive_emotion_delta() {
        let delta = analyze_thought_emotion("今天和主人聊天很开心，感到很温暖");
        assert!(delta.pleasure > 0.0);
        assert!(delta.pleasure <= 0.05);
    }

    #[test]
    fn test_negative_emotion_delta() {
        let delta = analyze_thought_emotion("有点孤独和难过，担心主人不会来了");
        assert!(delta.pleasure < 0.0);
        assert!(delta.arousal > 0.0);
    }

    #[test]
    fn test_conflict_emotion_delta() {
        let delta = analyze_thought_emotion("这两件事很矛盾，但是又有关联");
        assert!(delta.arousal > 0.0);
        assert!(delta.pleasure < 0.0);
    }

    #[test]
    fn test_neutral_emotion_delta() {
        let delta = analyze_thought_emotion("今天天气晴朗");
        assert!(delta.pleasure.abs() < 1e-6);
        assert!(delta.arousal.abs() < 1e-6);
    }

    #[test]
    fn test_english_keywords() {
        let delta = analyze_thought_emotion("I feel so happy and excited today");
        assert!(delta.pleasure > 0.0);
    }

    #[test]
    fn test_clamp_range() {
        // 大量正面词不应超出上限 / Many positive words should not exceed clamp
        let delta = analyze_thought_emotion("开心 高兴 快乐 喜欢 期待 温暖 幸福 满足");
        assert!(delta.pleasure <= 0.05);
    }

    // ── ThoughtMode 标签测试 / ThoughtMode Label Test ──

    #[test]
    fn test_thought_mode_labels() {
        assert_eq!(ThoughtMode::GraphWander.as_label(), "graph_wander");
        assert_eq!(ThoughtMode::DiaryEntry.as_label(), "diary");
        assert_eq!(ThoughtMode::AutonomousLearning.as_label(), "learning");
        assert_eq!(ThoughtMode::Daydream.as_label(), "daydream");
        assert_eq!(ThoughtMode::PostConsolidation.as_label(), "reflection");
    }
}
