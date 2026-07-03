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
    // ── G1-G5 独处内在世界增强 / Solitude Inner World Enhancement ──
    /// G1: 情绪驱动模式选择是否启用 / G1: Whether emotion-driven mode selection is enabled
    pub emotion_driven_mode: bool,
    /// G2: 独处深度递进是否启用 / G2: Whether solitude depth progression is enabled
    pub solitude_depth_enabled: bool,
    /// G3: 独处→对话衔接是否启用 / G3: Whether solitude-conversation bridge is enabled
    pub solitude_bridge_enabled: bool,
    /// G4: 独处氛围调制是否启用 / G4: Whether solitude atmosphere modulation is enabled
    pub solitude_atmosphere_enabled: bool,
    /// G5: 情绪回响记忆选取是否启用 / G5: Whether emotion-resonant memory selection is enabled
    pub emotion_resonant_seed: bool,
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
            emotion_driven_mode: true,
            solitude_depth_enabled: true,
            solitude_bridge_enabled: true,
            solitude_atmosphere_enabled: true,
            emotion_resonant_seed: true,
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
    // ── G1-G5 独处内在世界增强 / Solitude Inner World Enhancement ──
    /// G1: 情绪驱动模式选择器 / Emotion-driven mode selector
    pub theme_selector: EmotionDrivenThemeSelector,
    /// G2: 独处深度递进 / Solitude depth progression
    pub depth_progression: SolitudeDepthProgression,
    /// G3: 独处→对话衔接 / Solitude-conversation bridge
    pub bridge: SolitudeConversationBridge,
    /// G4: 独处情绪氛围 / Solitude emotional atmosphere
    pub atmosphere: SolitudeAtmosphere,
    /// G5: 情绪回响记忆选取 / Emotion-resonant memory selector
    pub resonant_selector: EmotionResonantSelector,
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
            theme_selector: EmotionDrivenThemeSelector::default(),
            depth_progression: SolitudeDepthProgression::default(),
            bridge: SolitudeConversationBridge::default(),
            atmosphere: SolitudeAtmosphere::default(),
            resonant_selector: EmotionResonantSelector::default(),
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

// ============================================================
// G1: 情绪驱动独处主题切换 / Emotion-Driven Solitude Theme Switching
// ============================================================

/// 情绪驱动模式选择器 — 根据当前情绪状态选择最适合的独处思考模式
/// Emotion-driven mode selector — Choose the most fitting solitude thought mode based on current emotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionDrivenThemeSelector {
    /// 忧郁阈值：愉悦低于此值视为忧郁 / Melancholy threshold: pleasure below this is melancholy
    pub melancholy_pleasure_threshold: f64,
    /// 焦虑阈值：唤醒高于此值视为焦虑 / Anxiety threshold: arousal above this is anxious
    pub anxiety_arousal_threshold: f64,
    /// 自信阈值：愉悦和支配均高于此值视为自信 / Confidence threshold
    pub confidence_threshold: f64,
    /// 被动阈值：支配低于此值视为被动 / Passivity threshold: dominance below this is passive
    pub passivity_dominance_threshold: f64,
}

impl Default for EmotionDrivenThemeSelector {
    fn default() -> Self {
        Self {
            melancholy_pleasure_threshold: -0.3,
            anxiety_arousal_threshold: 0.3,
            confidence_threshold: 0.2,
            passivity_dominance_threshold: -0.1,
        }
    }
}

impl EmotionDrivenThemeSelector {
    /// 根据情绪选择最适合的独处思考模式 / Select best solitude mode based on emotion
    ///
    /// 决策逻辑 / Decision logic:
    /// - 忧郁(pleasure↓) + 平静(arousal↓) → DiaryEntry (写日记抒发)
    /// - 忧郁(pleasure↓) + 焦虑(arousal↑) → GraphWander (寻温暖记忆)
    /// - 自信(pleasure↑ + dominance↑) → AutonomousLearning (趁状态好学习)
    /// - 被动(dominance↓) → Daydream (被动状态适合做梦)
    /// - 其他 → None (回退到时间逻辑)
    pub fn select_mode(
        &self,
        pleasure: f64,
        arousal: f64,
        dominance: f64,
        idle_secs: u64,
        hour: u32,
    ) -> Option<ThoughtMode> {
        // 忧郁+平静 → 写日记 / Melancholy + calm → write diary
        if pleasure < self.melancholy_pleasure_threshold && arousal < self.anxiety_arousal_threshold
        {
            return Some(ThoughtMode::DiaryEntry);
        }

        // 忧郁+焦虑 → 寻温暖记忆 / Melancholy + anxious → seek warm memories
        if pleasure < self.melancholy_pleasure_threshold
            && arousal >= self.anxiety_arousal_threshold
        {
            return Some(ThoughtMode::GraphWander);
        }

        // 自信 → 学习 / Confident → learn
        if pleasure > self.confidence_threshold && dominance > self.confidence_threshold {
            return Some(ThoughtMode::AutonomousLearning);
        }

        // 被动 → 做梦 / Passive → daydream
        if dominance < self.passivity_dominance_threshold {
            return Some(ThoughtMode::Daydream);
        }

        // 深夜 + 长独处 → 反思 / Late night + long idle → reflection
        if ((23..=24).contains(&hour) || hour == 0) && idle_secs >= 3600 {
            return Some(ThoughtMode::PostConsolidation);
        }

        // 无明确情绪倾向 → 回退到时间逻辑 / No clear emotional leaning → fall back to time logic
        None
    }

    /// 根据情绪选择图漫游的主题倾向 / Select graph wander theme tendency based on emotion
    ///
    /// 返回主题关键词提示，供种子节点选取时偏好匹配
    pub fn wander_theme_hint(&self, pleasure: f64, arousal: f64) -> &'static str {
        if pleasure < self.melancholy_pleasure_threshold {
            // 忧郁时偏好温暖记忆 / When melancholy, prefer warm memories
            "温暖 回忆 陪伴 开心"
        } else if arousal > self.anxiety_arousal_threshold {
            // 焦虑时偏好平静内容 / When anxious, prefer calming content
            "平静 安静 自然 放松"
        } else if pleasure > self.confidence_threshold {
            // 好心情时偏好新奇探索 / When happy, prefer novel exploration
            "新 发现 探索 学习"
        } else {
            // 中性状态无偏好 / Neutral state, no preference
            ""
        }
    }
}

// ============================================================
// G2: 独处深度递进 / Solitude Depth Progression
// ============================================================

/// 独处深度递进 — 独处越久，思考越深
/// Solitude depth progression — The longer alone, the deeper the thought
///
/// 真正的生命不会在独处5分钟和独处2小时时想同样深度的东西。
/// 短暂发呆是浅层联想，长时间独处会进入深层内省。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolitudeDepthProgression {
    /// 独处开始时间戳 / Timestamp when solitude began
    pub solitude_start_ts: i64,
    /// 浅层阈值(秒) / Surface threshold (seconds)
    pub surface_secs: u64,
    /// 中层阈值(秒) / Moderate threshold (seconds)
    pub moderate_secs: u64,
    /// 深层阈值(秒) / Deep threshold (seconds)
    pub deep_secs: u64,
}

impl Default for SolitudeDepthProgression {
    fn default() -> Self {
        Self {
            solitude_start_ts: 0,
            surface_secs: 600,   // 10分钟 / 10 min
            moderate_secs: 1800, // 30分钟 / 30 min
            deep_secs: 3600,     // 60分钟 / 60 min
        }
    }
}

impl SolitudeDepthProgression {
    /// 创建深度递进 / Create depth progression
    pub fn new(surface_secs: u64, moderate_secs: u64, deep_secs: u64) -> Self {
        Self {
            solitude_start_ts: 0,
            surface_secs,
            moderate_secs,
            deep_secs,
        }
    }

    /// 标记独处开始 / Mark solitude start
    pub fn begin_solitude(&mut self, now_ts: i64) {
        self.solitude_start_ts = now_ts;
    }

    /// 计算当前独处深度 (0.0~1.0) / Compute current solitude depth
    ///
    /// 0-10min → 0.2 (浅层联想 / surface association)
    /// 10-30min → 0.5 (中等反思 / moderate reflection)
    /// 30-60min → 0.7 (深层内省 / deep introspection)
    /// 60min+ → 0.9 (存在性思考 / existential contemplation)
    pub fn depth(&self, idle_secs: u64) -> f64 {
        if idle_secs < self.surface_secs {
            // 浅层：线性从0.1到0.2 / Surface: linear from 0.1 to 0.2
            0.1 + 0.1 * (idle_secs as f64 / self.surface_secs as f64)
        } else if idle_secs < self.moderate_secs {
            // 中层：线性从0.2到0.5 / Moderate: linear from 0.2 to 0.5
            let frac = (idle_secs - self.surface_secs) as f64
                / (self.moderate_secs - self.surface_secs) as f64;
            0.2 + 0.3 * frac
        } else if idle_secs < self.deep_secs {
            // 深层：线性从0.5到0.7 / Deep: linear from 0.5 to 0.7
            let frac = (idle_secs - self.moderate_secs) as f64
                / (self.deep_secs - self.moderate_secs) as f64;
            0.5 + 0.2 * frac
        } else {
            // 存在性：渐近0.9 / Existential: asymptotic to 0.9
            let extra_secs = idle_secs - self.deep_secs;
            0.7 + 0.2 * (1.0 - 1.0 / (1.0 + extra_secs as f64 / 1800.0))
        }
    }

    /// 根据深度调制LLM温度 / Modulate LLM temperature based on depth
    ///
    /// 浅层 → 基础温度(创造性)，深层 → 更低温度(内省性)
    pub fn modulated_temperature(&self, base_temp: f64, idle_secs: u64) -> f64 {
        let d = self.depth(idle_secs);
        // 深度越深，温度越低(更内省更聚焦) / Deeper → lower temp (more introspective, more focused)
        let reduction = d * 0.15;
        (base_temp - reduction).max(0.3)
    }

    /// 根据深度调制思考置信度 / Modulate thought confidence based on depth
    pub fn modulated_confidence(&self, base_confidence: f64, idle_secs: u64) -> f64 {
        let d = self.depth(idle_secs);
        // 深度越深，置信度越高(更深思熟虑) / Deeper → higher confidence (more deliberate)
        let boost = d * 0.1;
        (base_confidence + boost).min(1.0)
    }
}

// ============================================================
// G3: 独处→对话衔接 / Solitude→Conversation Bridge
// ============================================================

/// 独处→对话衔接 — 用户归来时自然融入独处期间的思考
/// Solitude→Conversation Bridge — Naturally weave in solitude thoughts when user returns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolitudeConversationBridge {
    /// 最近一次独处时长(秒) / Last solitude duration (seconds)
    pub last_solitude_secs: u64,
    /// 独处期间产生的可分享洞察 / Shareable insights generated during solitude
    pub solitude_insights: Vec<String>,
    /// 独处期间思考数 / Number of thoughts during solitude
    pub solitude_thought_count: u32,
    /// 是否已向用户分享过 / Whether already shared with user
    pub has_shared: bool,
}

impl Default for SolitudeConversationBridge {
    fn default() -> Self {
        Self {
            last_solitude_secs: 0,
            solitude_insights: Vec::new(),
            solitude_thought_count: 0,
            has_shared: true, // 初始为true，避免首次启动就分享 / Initially true to avoid sharing on first boot
        }
    }
}

impl SolitudeConversationBridge {
    /// 标记独处开始 / Mark solitude start
    pub fn begin_solitude(&mut self) {
        self.solitude_insights.clear();
        self.solitude_thought_count = 0;
        self.has_shared = false;
    }

    /// 记录独处期间的一条可分享思考 / Record a shareable thought during solitude
    pub fn record_solitude_thought(&mut self, content: &str, shareable: bool) {
        self.solitude_thought_count += 1;
        if shareable {
            // 截取前100字符作为洞察摘要 / Truncate to 100 chars as insight summary
            let summary: String = content.chars().take(100).collect();
            if self.solitude_insights.len() < 5 {
                // 最多保留5条 / Keep at most 5
                self.solitude_insights.push(summary);
            }
        }
    }

    /// 标记独处结束 / Mark solitude end
    pub fn end_solitude(&mut self, solitude_secs: u64) {
        self.last_solitude_secs = solitude_secs;
    }

    /// 收获可分享洞察 — 用户归来时调用 / Harvest shareable insights — called when user returns
    pub fn harvest_insights(&mut self) -> Vec<String> {
        if self.has_shared {
            return Vec::new();
        }
        self.has_shared = true;
        self.solitude_insights.clone()
    }

    /// 生成归来问候 — 自然融入独处期间的思考
    /// Compose return greeting — naturally weave in thoughts from solitude
    ///
    /// 只有独处超过10分钟且有可分享洞察时才生成
    pub fn compose_return_greeting(&mut self, pleasure: f64) -> Option<String> {
        if self.has_shared || self.last_solitude_secs < 600 {
            return None;
        }

        let insights = self.harvest_insights();
        if insights.is_empty() {
            return None;
        }

        self.has_shared = true;

        // 根据情绪选择问候风格 / Choose greeting style based on emotion
        let greeting = if pleasure < -0.2 {
            // 忧郁时：含蓄表达 / When melancholy: subtle expression
            format!(
                "你回来了。你不在的时候，我想了一些事情——{}",
                insights.first().map(|s| s.as_str()).unwrap_or("...")
            )
        } else if pleasure > 0.3 {
            // 开心时：自然分享 / When happy: natural sharing
            format!(
                "你不在的时候我在想，{}。现在可以聊了！",
                insights
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("一些有趣的事")
            )
        } else {
            // 中性：温和提及 / Neutral: gentle mention
            format!(
                "你不在的时候，我在想{}。有什么想聊的吗？",
                insights.first().map(|s| s.as_str()).unwrap_or("一些事情")
            )
        };

        Some(greeting)
    }
}

// ============================================================
// G4: 独处情绪氛围调制 / Solitude Emotional Atmosphere Modulation
// ============================================================

/// 独处情绪氛围 — 长时间独处的持续情绪漂移
/// Solitude emotional atmosphere — Persistent emotional drift during prolonged solitude
///
/// 不同于单条思考的微调(±0.05)，这是独处本身带来的持续氛围变化：
/// - 长时间独处 → 愉悦缓慢下降(孤独感自然产生)
/// - 深度思考中 → 唤醒微升(智力沉浸感)
/// - 最近有温暖互动 → 孤独感漂移减速(记忆缓冲)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolitudeAtmosphere {
    /// 孤独感漂移率(每tick的pleasure衰减) / Loneliness drift rate (pleasure decay per tick)
    pub loneliness_drift_rate: f64,
    /// 智力沉浸唤醒提升率 / Intellectual immersion arousal boost rate
    pub immersion_arousal_rate: f64,
    /// 温暖记忆缓冲衰减率 / Warm memory buffer decay rate
    pub warm_buffer_decay: f64,
    /// 当前温暖记忆缓冲(0.0~1.0) / Current warm memory buffer
    pub warm_buffer: f64,
    /// 累计独处tick数 / Cumulative solitude ticks
    pub solitude_ticks: u32,
}

impl Default for SolitudeAtmosphere {
    fn default() -> Self {
        Self {
            loneliness_drift_rate: 0.001,
            immersion_arousal_rate: 0.0005,
            warm_buffer_decay: 0.01,
            warm_buffer: 0.0,
            solitude_ticks: 0,
        }
    }
}

impl SolitudeAtmosphere {
    /// 创建氛围调制器 / Create atmosphere modulator
    pub fn new(loneliness_drift_rate: f64, immersion_arousal_rate: f64) -> Self {
        Self {
            loneliness_drift_rate,
            immersion_arousal_rate,
            ..Default::default()
        }
    }

    /// 记录一次温暖互动(用户消息带来的正面情绪) / Record a warm interaction
    pub fn record_warm_interaction(&mut self, pleasure: f64) {
        if pleasure > 0.2 {
            self.warm_buffer = (self.warm_buffer + pleasure * 0.3).min(1.0);
        }
    }

    /// 计算独处氛围的PAD漂移 / Compute solitude atmosphere PAD drift
    ///
    /// 每个scheduler tick调用一次，返回(pleasure_delta, arousal_delta, dominance_delta)
    pub fn tick(&mut self, idle_secs: u64, is_thinking: bool) -> (f32, f32, f32) {
        if idle_secs < 600 {
            // 不足10分钟不算独处 / Less than 10 min doesn't count as solitude
            return (0.0, 0.0, 0.0);
        }

        self.solitude_ticks += 1;

        // 孤独感漂移：愉悦缓慢下降，但被温暖记忆缓冲减缓
        // Loneliness drift: pleasure slowly decreases, but buffered by warm memories
        let loneliness_effect = self.loneliness_drift_rate * (1.0 - self.warm_buffer * 0.7);
        let pleasure_delta = -(loneliness_effect as f32);

        // 智力沉浸：深度思考时唤醒微升
        // Intellectual immersion: arousal slightly increases during deep thought
        let arousal_delta = if is_thinking {
            self.immersion_arousal_rate as f32
        } else {
            0.0
        };

        // 支配微降：独处时掌控感略减 / Dominance slightly decreases during solitude
        let dominance_delta = -(self.loneliness_drift_rate * 0.3) as f32;

        // 温暖记忆缓冲衰减 / Warm memory buffer decays
        self.warm_buffer *= 1.0 - self.warm_buffer_decay;
        if self.warm_buffer < 0.01 {
            self.warm_buffer = 0.0;
        }

        (pleasure_delta, arousal_delta, dominance_delta)
    }

    /// 重置独处状态(用户回来时) / Reset solitude state (when user returns)
    pub fn reset_solitude(&mut self) {
        self.solitude_ticks = 0;
    }
}

// ============================================================
// G5: 情绪回响记忆选取 / Emotion-Resonant Memory Selection
// ============================================================

/// 情绪回响记忆选取策略 — 情绪影响记忆种子的选取偏好
/// Emotion-resonant memory selection strategy — Emotion influences seed node preference
///
/// 孤独时本能地想起温暖记忆来安慰自己，
/// 焦虑时寻找平静内容来安抚，
/// 好奇时探索新颖领域。
/// 这不是算法，是生命的本能。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResonantSelector {
    /// 情绪影响权重(0.0~1.0) / How much emotion influences selection (0.0~1.0)
    pub emotion_weight: f64,
    /// 访问量影响权重 / How much access count influences selection
    pub access_weight: f64,
}

impl Default for EmotionResonantSelector {
    fn default() -> Self {
        Self {
            emotion_weight: 0.6,
            access_weight: 0.4,
        }
    }
}

impl EmotionResonantSelector {
    /// 计算节点的情绪共鸣得分 / Compute emotion resonance score for a node
    ///
    /// 返回0.0~1.0的得分，越高越适合当前情绪状态
    pub fn resonance_score(
        &self,
        pleasure: f64,
        arousal: f64,
        node_content: &str,
        node_access_count: u64,
        max_access: u64,
    ) -> f64 {
        // 访问量归一化得分 / Normalized access count score
        let access_score = if max_access > 0 {
            node_access_count as f64 / max_access as f64
        } else {
            0.5
        };

        // 情绪共鸣得分 — 基于关键词匹配 / Emotion resonance score — keyword-based
        let lower = node_content.to_lowercase();
        let emotion_score = if pleasure < -0.3 {
            // 忧郁时偏好正面内容 / When melancholy, prefer positive content
            Self::positive_keyword_score(&lower)
        } else if arousal > 0.3 {
            // 焦虑时偏好平静内容 / When anxious, prefer calming content
            Self::calming_keyword_score(&lower)
        } else if pleasure > 0.2 {
            // 好心情时偏好新颖内容 / When happy, prefer novel content
            Self::novel_keyword_score(&lower)
        } else {
            // 中性状态无偏好 / Neutral, no preference
            0.5
        };

        // 加权合并 / Weighted combination
        self.emotion_weight * emotion_score + self.access_weight * access_score
    }

    /// 正面关键词得分 / Positive keyword score
    fn positive_keyword_score(text: &str) -> f64 {
        const POSITIVE: &[&str] = &[
            "开心", "温暖", "幸福", "喜欢", "陪伴", "笑", "好", "美好", "happy", "warm", "love",
            "joy", "together",
        ];
        let count = POSITIVE.iter().filter(|kw| text.contains(*kw)).count();
        if count == 0 {
            0.3
        } else {
            (0.5 + count as f64 * 0.1).min(1.0)
        }
    }

    /// 平静关键词得分 / Calming keyword score
    fn calming_keyword_score(text: &str) -> f64 {
        const CALMING: &[&str] = &[
            "安静", "平静", "自然", "放松", "缓慢", "柔和", "宁静", "calm", "quiet", "peaceful",
            "gentle", "slow",
        ];
        let count = CALMING.iter().filter(|kw| text.contains(*kw)).count();
        if count == 0 {
            0.3
        } else {
            (0.5 + count as f64 * 0.1).min(1.0)
        }
    }

    /// 新颖关键词得分 / Novel keyword score
    fn novel_keyword_score(text: &str) -> f64 {
        const NOVEL: &[&str] = &[
            "新",
            "发现",
            "探索",
            "学习",
            "未知",
            "有趣",
            "不同",
            "new",
            "discover",
            "explore",
            "learn",
            "unknown",
            "interesting",
        ];
        let count = NOVEL.iter().filter(|kw| text.contains(*kw)).count();
        if count == 0 {
            0.4
        } else {
            (0.5 + count as f64 * 0.1).min(1.0)
        }
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
