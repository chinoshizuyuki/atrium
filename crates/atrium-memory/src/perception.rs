// SPDX-License-Identifier: MIT
//! 非语言感知层
//! 从消息元数据中提取零成本情感信号：打字速度、消息长度趋势、标点模式、emoji 变化、时间模式。
//! 让 AI 从"读文本"升级为"读人"。
//!
//! 核心组件:
//! - `TypingBaseline`: EMA 学习器，从历史数据学习用户打字习惯
//! - `TypingRhythmAnalyzer`: 节奏分析器，分类节奏模式并推断情绪暗示
//! - `compile_rhythm_hint()`: 生成 LLM 可读的节奏提示文本
//!
//! 所有信号检测为纯规则计算，延迟 < 1μs/条。
//! Non-verbal perception layer.
//!
//! Extracts low-cost non-verbal signals from message metadata:
//! typing speed, message length, emoji patterns, time-of-day patterns, etc.
//! Enables the AI to "read between the lines" and perceive user mood.
//!
//! Components:
//! - `TypingBaseline`: EMA learning of user's historical typing rhythm
//! - `TypingRhythmAnalyzer`: real-time typing pattern analysis and mood inference
//! - `compile_rhythm_hint()`: compiles LLM-readable rhythm hint text

use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ════════════════════════════════════════════════════════════════════
// 客户端类型
// ════════════════════════════════════════════════════════════════════

/// 客户端类型 — 决定可采集的元数据丰富度
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ClientType {
    /// Web 前端（可以采集最多元数据）
    Web,
    /// 终端（只有消息和时间）
    Terminal,
    /// 移动端（可以采集打字速度）
    Mobile,
    /// API 调用（只有消息）
    Api,
    /// 未知
    Unknown,
}

// ════════════════════════════════════════════════════════════════════
// 消息事件
// ════════════════════════════════════════════════════════════════════

/// 消息事件 — 前端/网关上报
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageEvent {
    /// 消息文本
    pub content: String,
    /// 消息发送时间（Unix 毫秒）
    pub timestamp: i64,
    /// 输入开始时间（如果有前端支持）
    pub typing_started_at: Option<i64>,
    /// 是否有编辑/删除重打（如果有前端支持）
    pub edit_count: Option<u32>,
    /// 客户端类型
    pub client_type: ClientType,
}

impl MessageEvent {
    /// 从消息文本和时间戳快速构造（无元数据场景）
    pub fn simple(content: String, timestamp: i64) -> Self {
        Self {
            content,
            timestamp,
            typing_started_at: None,
            edit_count: None,
            client_type: ClientType::Terminal,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 打字基线（EMA 学习器）
// ════════════════════════════════════════════════════════════════════

/// 用户打字基线 — 从历史数据自动学习
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypingBaseline {
    /// 平均消息间隔（秒）
    pub avg_gap_seconds: f64,
    /// 平均消息长度（字符数）
    pub avg_message_length: f64,
    /// 平均 emoji 频率（含 emoji 的消息占比，0..1）
    pub emoji_frequency: f64,
    /// 平均标点密度（标点字符 / 总字符）
    pub punctuation_density: f64,
    /// 活跃时段分布（24 小时直方图）
    pub active_hours: [f32; 24],
    /// 样本数量（越多越可靠）
    pub sample_count: u64,
    /// EMA 学习率
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
}

fn default_learning_rate() -> f64 {
    0.05
}

impl TypingBaseline {
    /// 创建默认基线
    pub fn new(learning_rate: f64) -> Self {
        Self {
            avg_gap_seconds: 0.0,
            avg_message_length: 0.0,
            emoji_frequency: 0.0,
            punctuation_density: 0.0,
            active_hours: [0.0; 24],
            sample_count: 0,
            learning_rate,
        }
    }

    /// 每条消息后增量更新基线
    pub fn update(&mut self, event: &MessageEvent, prev_event: Option<&MessageEvent>) {
        let alpha = self.learning_rate;

        // 更新平均间隔
        if let Some(prev) = prev_event {
            let gap = (event.timestamp - prev.timestamp) as f64 / 1000.0;
            if gap > 0.0 && gap < 3600.0 {
                // 忽略负间隔和超过 1 小时的间隔（可能是新会话）
                if self.avg_gap_seconds == 0.0 {
                    self.avg_gap_seconds = gap;
                } else {
                    self.avg_gap_seconds = lerp(self.avg_gap_seconds, gap, alpha);
                }
            }
        }

        // 更新平均消息长度
        let len = event.content.chars().count() as f64;
        if self.sample_count == 0 {
            self.avg_message_length = len;
        } else {
            self.avg_message_length = lerp(self.avg_message_length, len, alpha);
        }

        // 更新 emoji 频率
        let has_emoji = contains_emoji(&event.content);
        let emoji_val = if has_emoji { 1.0 } else { 0.0 };
        self.emoji_frequency = lerp(self.emoji_frequency, emoji_val, alpha);

        // 更新标点密度
        let punct_count = event
            .content
            .chars()
            .filter(|c| c.is_ascii_punctuation() || is_cn_punct(*c))
            .count();
        let density = punct_count as f64 / len.max(1.0);
        self.punctuation_density = lerp(self.punctuation_density, density, alpha);

        // 更新活跃时段
        if let Some(dt) = chrono::DateTime::from_timestamp_millis(event.timestamp) {
            let hour = dt.with_timezone(&chrono::Local).hour() as usize;
            for (i, h) in self.active_hours.iter_mut().enumerate() {
                if i == hour {
                    *h = lerp(*h as f64, 1.0, alpha) as f32;
                } else {
                    *h *= 1.0 - alpha as f32 * 0.1;
                }
            }
        }

        self.sample_count += 1;
    }

    /// 基线是否足够可靠（至少 N 个样本）
    pub fn is_reliable(&self, min_samples: u64) -> bool {
        self.sample_count >= min_samples
    }
}

impl Default for TypingBaseline {
    fn default() -> Self {
        Self::new(0.05)
    }
}

// ════════════════════════════════════════════════════════════════════
// 节奏模式与情绪暗示
// ════════════════════════════════════════════════════════════════════

/// 节奏模式分类
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RhythmPattern {
    /// 正常节奏（与基线一致）
    Normal,
    /// 加速（消息间隔持续缩短）
    Accelerating,
    /// 减速（消息间隔持续增长）
    Decelerating,
    /// 爆发（突然多条短消息）
    Bursting,
    /// 枯竭（回复越来越短、越来越慢）
    Depleting,
    /// 不规则（节奏混乱）
    Erratic,
}

impl RhythmPattern {
    /// 节奏模式的中文描述（用于 LLM 提示）
    pub fn description_zh(&self) -> &'static str {
        match self {
            RhythmPattern::Normal => "正常",
            RhythmPattern::Accelerating => "加速",
            RhythmPattern::Decelerating => "减速",
            RhythmPattern::Bursting => "爆发",
            RhythmPattern::Depleting => "枯竭",
            RhythmPattern::Erratic => "不规则",
        }
    }
}

/// 从节奏推断的情绪暗示
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypingMoodHint {
    /// 能量暗示（来自节奏，不是文本），[-1, 1]
    pub energy: f32,
    /// 确信度（来自节奏），[-1, 1] 负=犹豫，正=果断
    pub confidence: f32,
    /// 情绪暗示，[-1, 1]
    pub mood: f32,
}

impl Default for TypingMoodHint {
    fn default() -> Self {
        Self {
            energy: 0.0,
            confidence: 0.0,
            mood: 0.0,
        }
    }
}

/// 当前会话的打字节奏状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypingRhythm {
    /// 节奏类型
    pub pattern: RhythmPattern,
    /// 与基线的偏差度（0.0 = 完全正常，1.0 = 极度异常）
    pub deviation: f32,
    /// 推断的情绪暗示
    pub mood_hint: TypingMoodHint,
}

impl Default for TypingRhythm {
    fn default() -> Self {
        Self {
            pattern: RhythmPattern::Normal,
            deviation: 0.0,
            mood_hint: TypingMoodHint::default(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 打字节奏分析器
// ════════════════════════════════════════════════════════════════════

/// 打字节奏分析器 — 从消息元数据中提取非语言信号
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypingRhythmAnalyzer {
    /// 用户消息时间序列（保留最近 window_size 条）
    message_timeline: VecDeque<MessageEvent>,
    /// 用户的基线指标（从历史数据学习）
    pub baseline: TypingBaseline,
    /// 当前会话的节奏状态
    current_rhythm: TypingRhythm,
    /// 分析窗口大小
    window_size: usize,
}

impl TypingRhythmAnalyzer {
    /// 创建新的节奏分析器
    ///
    /// - `learning_rate`: EMA 学习率（默认 0.05）
    /// - `window_size`: 分析窗口大小（默认 8）
    pub fn new(learning_rate: f64, window_size: usize) -> Self {
        Self {
            message_timeline: VecDeque::with_capacity(window_size + 1),
            baseline: TypingBaseline::new(learning_rate),
            current_rhythm: TypingRhythm::default(),
            window_size: window_size.max(3), // 至少需要 3 条才能分析趋势
        }
    }

    /// 接收消息事件 → 更新基线 → 分析节奏 → 返回结果
    pub fn on_message(&mut self, event: MessageEvent) -> TypingRhythm {
        // 1. 更新基线（需要前一条消息做间隔计算）
        let prev = self.message_timeline.back().cloned();
        self.baseline.update(&event, prev.as_ref());

        // 2. 加入时间序列
        self.message_timeline.push_back(event);
        while self.message_timeline.len() > self.window_size {
            self.message_timeline.pop_front();
        }

        // 3. 分析节奏
        self.current_rhythm = self.analyze();
        self.current_rhythm.clone()
    }

    /// 分析最近 N 条消息的节奏
    pub fn analyze(&self) -> TypingRhythm {
        let recent: Vec<&MessageEvent> = self.message_timeline.iter().collect();

        if recent.len() < 3 {
            return TypingRhythm::default();
        }

        // 计算间隔序列
        let gaps: Vec<f64> = recent
            .windows(2)
            .map(|w| (w[1].timestamp - w[0].timestamp) as f64 / 1000.0)
            .collect();

        // 计算长度序列
        let lengths: Vec<usize> = recent.iter().map(|e| e.content.chars().count()).collect();

        // 判断节奏模式
        let pattern = self.classify_pattern(&gaps, &lengths);

        // 计算与基线的偏差
        let deviation = self.compute_deviation(&gaps, &lengths);

        // 推断情绪暗示
        let mood_hint = self.infer_mood_from_rhythm(&gaps, &lengths, &pattern);

        TypingRhythm {
            pattern,
            deviation,
            mood_hint,
        }
    }

    /// 获取当前缓存的节奏状态
    pub fn current_rhythm(&self) -> &TypingRhythm {
        &self.current_rhythm
    }

    /// 获取消息时间序列长度
    pub fn timeline_len(&self) -> usize {
        self.message_timeline.len()
    }

    // ── 节奏分类 ──

    fn classify_pattern(&self, gaps: &[f64], lengths: &[usize]) -> RhythmPattern {
        if gaps.is_empty() {
            return RhythmPattern::Normal;
        }

        let gap_trend = trend_direction(gaps);
        let length_trend = trend_direction(&lengths.iter().map(|l| *l as f64).collect::<Vec<_>>());

        let baseline_gap = self.baseline.avg_gap_seconds;
        let baseline_len = self.baseline.avg_message_length;

        // 爆发：大部分间隔远短于基线，且大部分消息远短于基线（优先检测）
        if baseline_gap > 0.0 && baseline_len > 0.0 {
            let short_gap_count = gaps.iter().filter(|g| **g < baseline_gap * 0.5).count();
            let short_msg_count = lengths
                .iter()
                .filter(|l| (**l as f64) < baseline_len * 0.5)
                .count();
            let gap_ratio = short_gap_count as f64 / gaps.len().max(1) as f64;
            let msg_ratio = short_msg_count as f64 / lengths.len().max(1) as f64;
            if gap_ratio >= 0.75 && msg_ratio >= 0.75 {
                return RhythmPattern::Bursting;
            }
        }

        // 枯竭：间隔增长 + 长度缩短
        if gap_trend > 0.2 && length_trend < -0.15 {
            return RhythmPattern::Depleting;
        }

        // 加速：间隔持续缩短
        if gap_trend < -0.3 {
            return RhythmPattern::Accelerating;
        }

        // 减速：间隔增长
        if gap_trend > 0.3 {
            return RhythmPattern::Decelerating;
        }

        // 不规则：存在极端间隔
        if baseline_gap > 0.0 && gaps.iter().any(|g| *g > baseline_gap * 5.0) {
            return RhythmPattern::Erratic;
        }

        RhythmPattern::Normal
    }

    fn compute_deviation(&self, gaps: &[f64], lengths: &[usize]) -> f32 {
        if self.baseline.sample_count < 5 {
            return 0.0; // 基线不够可靠，不计算偏差
        }

        // 间隔偏差
        let avg_gap = mean(gaps);
        let gap_dev = if self.baseline.avg_gap_seconds > 0.0 {
            ((avg_gap - self.baseline.avg_gap_seconds) / self.baseline.avg_gap_seconds).abs()
        } else {
            0.0
        };

        // 长度偏差
        let avg_len = mean(&lengths.iter().map(|l| *l as f64).collect::<Vec<_>>());
        let len_dev = if self.baseline.avg_message_length > 0.0 {
            ((avg_len - self.baseline.avg_message_length) / self.baseline.avg_message_length).abs()
        } else {
            0.0
        };

        ((gap_dev + len_dev) / 2.0).min(1.0) as f32
    }

    fn infer_mood_from_rhythm(
        &self,
        _gaps: &[f64],
        lengths: &[usize],
        pattern: &RhythmPattern,
    ) -> TypingMoodHint {
        // 能量暗示
        let energy = match pattern {
            RhythmPattern::Accelerating => 0.4,
            RhythmPattern::Bursting => 0.6,
            RhythmPattern::Depleting => -0.5,
            RhythmPattern::Decelerating => -0.2,
            RhythmPattern::Erratic => -0.1,
            RhythmPattern::Normal => 0.0,
        };

        // 确信度：消息长度稳定 = 果断，频繁编辑 = 犹豫
        let edit_rate = self.recent_edit_rate();
        let confidence = if edit_rate > 0.3 {
            -0.4
        } else if lengths.iter().all(|l| *l > 20) {
            0.3
        } else {
            0.0
        };

        // 标点情绪
        let excl_density = self.recent_punctuation_density('!');
        let ellipsis_density = self.recent_punctuation_density('…');
        let mood = if excl_density > 0.05 {
            0.3
        } else if ellipsis_density > 0.05 {
            -0.2
        } else {
            0.0
        };

        TypingMoodHint {
            energy,
            confidence,
            mood,
        }
    }

    // ── 辅助查询 ──

    fn recent_edit_rate(&self) -> f64 {
        let total = self.message_timeline.len();
        if total == 0 {
            return 0.0;
        }
        let with_edits = self
            .message_timeline
            .iter()
            .filter(|e| e.edit_count.is_some_and(|c| c > 0))
            .count();
        with_edits as f64 / total as f64
    }

    fn recent_punctuation_density(&self, target: char) -> f64 {
        let total_chars: usize = self
            .message_timeline
            .iter()
            .map(|e| e.content.chars().count().max(1))
            .sum();
        let target_count: usize = self
            .message_timeline
            .iter()
            .map(|e| e.content.chars().filter(|c| *c == target).count())
            .sum();
        target_count as f64 / total_chars.max(1) as f64
    }
}

impl Default for TypingRhythmAnalyzer {
    fn default() -> Self {
        Self::new(0.05, 8)
    }
}

// ════════════════════════════════════════════════════════════════════
// LLM 提示生成
// ════════════════════════════════════════════════════════════════════

/// 生成 LLM 可读的节奏提示文本
///
/// 当节奏偏差 < 0.2 时返回空字符串（正常节奏不需要提示）。
pub fn compile_rhythm_hint(rhythm: &TypingRhythm) -> String {
    if rhythm.deviation < 0.2 {
        return String::new();
    }

    match rhythm.pattern {
        RhythmPattern::Bursting => "[用户打字很快，连续发了多条短消息，情绪可能比较激动]".into(),
        RhythmPattern::Depleting => "[用户回复越来越短、越来越慢，可能在失去兴趣或感到疲惫]".into(),
        RhythmPattern::Accelerating => "[用户打字速度在加快，看起来很有表达欲]".into(),
        RhythmPattern::Decelerating => "[用户打字在变慢，可能在思考或分心]".into(),
        RhythmPattern::Erratic => "[用户打字节奏不太规律，可能状态不太稳定]".into(),
        RhythmPattern::Normal => String::new(),
    }
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数
// ════════════════════════════════════════════════════════════════════

/// 检测中文标点
fn is_cn_punct(c: char) -> bool {
    matches!(
        c,
        '\u{FF01}' | // ！
 '\u{FF1F}' | // ？
 '\u{3002}' | // 。
 '\u{FF0C}' | // ，
 '\u{3001}' | // 、
 '\u{FF1B}' | // ；
 '\u{FF1A}' | // ：
 '\u{201C}' | // "
 '\u{201D}' | // "
 '\u{2018}' | // '
 '\u{2019}' | // '
 '\u{2026}' | // …
 '\u{300A}' | // 《
 '\u{300B}' // 》
    )
}

/// EMA 线性插值
fn lerp(a: f64, b: f64, alpha: f64) -> f64 {
    a + alpha * (b - a)
}

/// 均值
fn mean(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    vals.iter().sum::<f64>() / vals.len() as f64
}

/// 趋势方向 — 简单线性回归斜率的归一化值
///
/// 返回 [-1, 1] 范围：
/// - 正 = 上升趋势
/// - 负 = 下降趋势
/// - 接近 0 = 无明显趋势
fn trend_direction(vals: &[f64]) -> f64 {
    let n = vals.len();
    if n < 2 {
        return 0.0;
    }

    // 简单线性回归: y = a + b*x, 其中 x = 0, 1, 2, ...
    let x_mean = (n - 1) as f64 / 2.0;
    let y_mean = mean(vals);

    let mut num = 0.0;
    let mut den = 0.0;
    for (i, v) in vals.iter().enumerate() {
        let dx = i as f64 - x_mean;
        num += dx * (v - y_mean);
        den += dx * dx;
    }

    if den.abs() < f64::EPSILON {
        return 0.0;
    }

    let slope = num / den;

    // 归一化：slope / y_mean（避免绝对值影响）
    if y_mean.abs() > f64::EPSILON {
        (slope / y_mean).clamp(-1.0, 1.0)
    } else {
        slope.clamp(-1.0, 1.0)
    }
}

/// 检测文本是否包含 emoji
fn contains_emoji(s: &str) -> bool {
    s.chars().any(is_emoji)
}

/// 粗略 emoji 检测（与 user_model.rs 一致的逻辑）
fn is_emoji(c: char) -> bool {
    ('\u{1F600}'..='\u{1F64F}').contains(&c)
        || ('\u{1F300}'..='\u{1F5FF}').contains(&c)
        || ('\u{1F680}'..='\u{1F6FF}').contains(&c)
        || ('\u{1F900}'..='\u{1F9FF}').contains(&c)
        || ('\u{2600}'..='\u{26FF}').contains(&c)
        || ('\u{2700}'..='\u{27BF}').contains(&c)
        || ('\u{FE00}'..='\u{FE0F}').contains(&c)
        || ('\u{1FA00}'..='\u{1FA6F}').contains(&c)
        || ('\u{1FA70}'..='\u{1FAFF}').contains(&c)
        || ('\u{200D}' == c)
        || ('\u{20E3}' == c)
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(content: &str, timestamp_ms: i64) -> MessageEvent {
        MessageEvent::simple(content.into(), timestamp_ms)
    }

    fn make_event_with_edits(content: &str, timestamp_ms: i64, edits: u32) -> MessageEvent {
        MessageEvent {
            content: content.into(),
            timestamp: timestamp_ms,
            typing_started_at: None,
            edit_count: Some(edits),
            client_type: ClientType::Terminal,
        }
    }

    // ── TypingBaseline ──

    #[test]
    fn test_baseline_initial_values() {
        let baseline = TypingBaseline::default();
        assert_eq!(baseline.avg_gap_seconds, 0.0);
        assert_eq!(baseline.avg_message_length, 0.0);
        assert_eq!(baseline.emoji_frequency, 0.0);
        assert_eq!(baseline.punctuation_density, 0.0);
        assert_eq!(baseline.sample_count, 0);
        assert!(!baseline.is_reliable(1));
    }

    #[test]
    fn test_baseline_ema_update() {
        let mut baseline = TypingBaseline::new(0.5); // 高学习率便于测试

        let e1 = make_event("hello world", 1000);
        baseline.update(&e1, None);
        assert_eq!(baseline.sample_count, 1);
        // 第一条消息：长度直接设定
        assert!(baseline.avg_message_length > 0.0);

        let e2 = make_event("this is a longer message for testing purposes", 11000);
        baseline.update(&e2, Some(&e1));
        assert_eq!(baseline.sample_count, 2);
        // 间隔 = 10s
        assert!(
            (baseline.avg_gap_seconds - 10.0).abs() < 0.1,
            "avg_gap 应接近 10s: {}",
            baseline.avg_gap_seconds
        );
        // 平均长度应介于两条消息长度之间
        assert!(
            baseline.avg_message_length > 11.0 && baseline.avg_message_length < 50.0,
            "avg_message_length 应 EMA 混合: {}",
            baseline.avg_message_length
        );
    }

    #[test]
    fn test_baseline_learning_rate_config() {
        let b1 = TypingBaseline::new(0.1);
        let b2 = TypingBaseline::new(0.9);
        assert!((b1.learning_rate - 0.1).abs() < f64::EPSILON);
        assert!((b2.learning_rate - 0.9).abs() < f64::EPSILON);

        // 高学习率 → 基线快速趋近最新值
        let mut b_fast = TypingBaseline::new(0.9);
        let mut b_slow = TypingBaseline::new(0.01);

        let e1 = make_event("short", 1000);
        let e2 = make_event("this is a much much much longer message!", 2000);

        b_fast.update(&e1, None);
        b_fast.update(&e2, Some(&e1));
        b_slow.update(&e1, None);
        b_slow.update(&e2, Some(&e1));

        // 高学习率的 avg_message_length 应更接近第二条消息的长度
        assert!(
            b_fast.avg_message_length > b_slow.avg_message_length,
            "高学习率应更快趋近新值: fast={} slow={}",
            b_fast.avg_message_length,
            b_slow.avg_message_length
        );
    }

    #[test]
    fn test_baseline_emoji_detection() {
        let mut baseline = TypingBaseline::new(0.5);
        let e1 = make_event("no emoji here", 1000);
        baseline.update(&e1, None);
        assert!(baseline.emoji_frequency < 0.1);

        let e2 = make_event("hello 🎉🎊🥳", 2000);
        baseline.update(&e2, Some(&e1));
        assert!(
            baseline.emoji_frequency > 0.0,
            "含 emoji 后 emoji_frequency 应 > 0: {}",
            baseline.emoji_frequency
        );
    }

    // ── RhythmPattern ──

    #[test]
    fn test_rhythm_normal() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 8);

        // 先建立基线（稳定间隔 5s，稳定长度 ~20 字符）
        let base_ts = 1_000_000i64;
        for i in 0..10 {
            let event = make_event("这是一条普通长度的消息内容", base_ts + i * 5000);
            analyzer.on_message(event);
        }

        // 继续发送稳定节奏的消息
        for i in 10..15 {
            let event = make_event("这还是一条普通长度的消息", base_ts + i * 5000);
            let rhythm = analyzer.on_message(event);
            // 稳定节奏 → deviation 应较小
            assert!(
                rhythm.deviation < 0.5,
                "稳定节奏 deviation 应较小: {}",
                rhythm.deviation
            );
        }
    }

    #[test]
    fn test_rhythm_accelerating() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 8);

        let base_ts = 1_000_000i64;

        // 建立基线: 间隔 10s
        for i in 0..10 {
            let event = make_event("这是一条普通消息用来建立基线的", base_ts + i * 10000);
            analyzer.on_message(event);
        }

        // 加速：间隔从 8s 逐步缩短到 1s
        let mut ts = base_ts + 100_000;
        for gap in [8000, 6000, 4000, 3000, 2000, 1000] {
            ts += gap;
            let event = make_event("消息内容逐渐加快发送", ts);
            analyzer.on_message(event);
        }

        let rhythm = analyzer.analyze();
        assert_eq!(
            rhythm.pattern,
            RhythmPattern::Accelerating,
            "间隔持续缩短应为 Accelerating, got {:?}",
            rhythm.pattern
        );
        assert!(rhythm.mood_hint.energy > 0.0, "加速 → 正能量");
    }

    #[test]
    fn test_rhythm_depleting() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 8);

        let base_ts = 1_000_000i64;

        // 建立基线: 间隔 5s, 长度 ~20 字符
        for i in 0..10 {
            let event = make_event("这是一条普通长度的消息内容", base_ts + i * 5000);
            analyzer.on_message(event);
        }

        // 枯竭：间隔增长 + 长度缩短
        let mut ts = base_ts + 50_000;
        let msgs = [
            ("消息变短了", 8000),
            ("更短", 12000),
            ("嗯", 18000),
            ("…", 25000),
            ("哦", 35000),
        ];
        for (content, gap) in &msgs {
            ts += gap;
            let event = make_event(content, ts);
            analyzer.on_message(event);
        }

        let rhythm = analyzer.analyze();
        assert_eq!(
            rhythm.pattern,
            RhythmPattern::Depleting,
            "间隔增长+长度缩短应为 Depleting, got {:?}",
            rhythm.pattern
        );
        assert!(rhythm.mood_hint.energy < 0.0, "枯竭 → 负能量");
    }

    #[test]
    fn test_rhythm_bursting() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.05, 8); // 低学习率保持基线稳定

        let base_ts = 1_000_000i64;

        // 建立基线: 间隔 30s, 长度 ~16 字符
        for i in 0..30 {
            let event = make_event("这是一条比较长的消息用来建立基线数据", base_ts + i * 30000);
            analyzer.on_message(event);
        }

        // 爆发：间隔 1s，短消息
        let mut ts = base_ts + 500_000;
        for _ in 0..6 {
            ts += 1000;
            let event = make_event("啊", ts);
            analyzer.on_message(event);
        }

        let rhythm = analyzer.analyze();
        assert_eq!(
            rhythm.pattern,
            RhythmPattern::Bursting,
            "短间隔+短消息应为 Bursting, got {:?}",
            rhythm.pattern
        );
        assert!(
            rhythm.mood_hint.energy > 0.0,
            "爆发 → 高正能量: {}",
            rhythm.mood_hint.energy
        );
    }

    #[test]
    fn test_mood_hint_energy() {
        // Bursting → energy > 0
        let hint_burst = TypingMoodHint {
            energy: 0.6,
            confidence: 0.0,
            mood: 0.3,
        };
        assert!(hint_burst.energy > 0.0);

        // Depleting → energy < 0
        let hint_deplete = TypingMoodHint {
            energy: -0.5,
            confidence: -0.2,
            mood: -0.1,
        };
        assert!(hint_deplete.energy < 0.0);
    }

    #[test]
    fn test_compile_rhythm_hint() {
        // deviation < 0.2 → 空字符串
        let normal = TypingRhythm {
            pattern: RhythmPattern::Normal,
            deviation: 0.1,
            mood_hint: TypingMoodHint::default(),
        };
        assert!(compile_rhythm_hint(&normal).is_empty());

        // Bursting → 含"很快"
        let bursting = TypingRhythm {
            pattern: RhythmPattern::Bursting,
            deviation: 0.8,
            mood_hint: TypingMoodHint {
                energy: 0.6,
                confidence: 0.0,
                mood: 0.3,
            },
        };
        let hint = compile_rhythm_hint(&bursting);
        assert!(hint.contains("很快"), "Bursting hint 应含 '很快': {}", hint);

        // Depleting → 含"越来越短"
        let depleting = TypingRhythm {
            pattern: RhythmPattern::Depleting,
            deviation: 0.5,
            mood_hint: TypingMoodHint {
                energy: -0.5,
                confidence: -0.2,
                mood: -0.1,
            },
        };
        let hint = compile_rhythm_hint(&depleting);
        assert!(
            hint.contains("越来越短"),
            "Depleting hint 应含 '越来越短': {}",
            hint
        );
    }

    #[test]
    fn test_on_message_pipeline() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.3, 8);

        // 模拟 10 条消息，间隔从 10s 逐渐缩短到 2s
        let base_ts = 1_700_000_000_000i64;
        let mut ts = base_ts;

        for i in 0..10 {
            let gap = (10 - i as i64) * 1000; // 10s, 9s, 8s, ..., 1s
            ts += gap;
            let event = make_event(&format!("消息编号 {} 内容长度适中", i), ts);
            let rhythm = analyzer.on_message(event);

            // 基线应该在学习中
            assert_eq!(analyzer.baseline.sample_count, (i + 1) as u64);

            if i < 2 {
                // 前几条消息不够分析窗口
                assert_eq!(rhythm.pattern, RhythmPattern::Normal);
            }
        }

        // 最终分析：间隔持续缩短 → 应该检测到 Accelerating
        let final_rhythm = analyzer.analyze();
        assert!(
            final_rhythm.deviation > 0.0,
            "有变化时 deviation 应 > 0: {}",
            final_rhythm.deviation
        );

        // mood_hint 应合理
        assert!(
            final_rhythm.mood_hint.energy >= -1.0 && final_rhythm.mood_hint.energy <= 1.0,
            "energy 应在 [-1, 1] 范围: {}",
            final_rhythm.mood_hint.energy
        );
    }

    #[test]
    fn test_edit_count_affects_confidence() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 8);
        let base_ts = 1_000_000i64;

        // 发送带编辑的消息
        for i in 0..8 {
            let event = make_event_with_edits(
                "这是一条经过多次修改的消息内容",
                base_ts + i * 5000,
                3, // 每条都编辑了 3 次
            );
            analyzer.on_message(event);
        }

        let rhythm = analyzer.analyze();
        // 频繁编辑 → 犹豫 → confidence < 0
        assert!(
            rhythm.mood_hint.confidence < 0.0,
            "频繁编辑应导致 confidence < 0: {}",
            rhythm.mood_hint.confidence
        );
    }

    #[test]
    fn test_exclamation_affects_mood() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 8);
        let base_ts = 1_000_000i64;

        // 发送大量感叹号的消息
        for i in 0..8 {
            let event = make_event("太棒了!!! 真好!!! 开心!!! 太好了!!!", base_ts + i * 5000);
            analyzer.on_message(event);
        }

        let rhythm = analyzer.analyze();
        // 感叹号多 → mood > 0
        assert!(
            rhythm.mood_hint.mood > 0.0,
            "大量感叹号应导致 mood > 0: {}",
            rhythm.mood_hint.mood
        );
    }

    #[test]
    fn test_timeline_window_limit() {
        let mut analyzer = TypingRhythmAnalyzer::new(0.5, 5);
        let base_ts = 1_000_000i64;

        for i in 0..20 {
            let event = make_event("消息内容", base_ts + i * 5000);
            analyzer.on_message(event);
        }

        // 时间序列不应超过 window_size
        assert!(
            analyzer.timeline_len() <= 5,
            "timeline 不应超过 window_size: {}",
            analyzer.timeline_len()
        );
        // 但基线应该累计了所有样本
        assert_eq!(analyzer.baseline.sample_count, 20);
    }
}
