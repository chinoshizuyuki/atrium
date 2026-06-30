// SPDX-License-Identifier: MIT
//! 实时反馈闭环 — ③
//!
//! 检测用户对 AI 回复的隐性反馈信号：
//! 纠正、赞扬、挫败、话题切换、深入追问
//! 生成满意度分数和行为调节建议，
//! 形成 "行为→反馈→调适" 的自进化循环。
//!
//! 所有信号检测为纯规则匹配，延迟 < 1μs/条。
//! FeedbackLoop — Real-time feedback loop.
//!
//! Detects sentiment signals from user's reactions to AI responses:
//! keywords, tone, explicit directives, sentiment polarity, etc.
//! Rule-based matching, latency < 1ms/event.

use chrono::Local;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 反馈信号
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FeedbackSignal {
    Correction { strength: f32, timestamp: i64 },
    Praise { timestamp: i64 },
    Frustration { timestamp: i64 },
    TopicShift { timestamp: i64 },
    Deepening { depth: u32, timestamp: i64 },
}

impl FeedbackSignal {
    /// 该信号对满意度的影响量
    pub fn satisfaction_delta(&self) -> f32 {
        match self {
            Self::Correction { strength, .. } => -0.05 * strength,
            Self::Praise { .. } => 0.03,
            Self::Frustration { .. } => -0.08,
            Self::TopicShift { .. } => 0.0,
            Self::Deepening { .. } => 0.02,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 行为调节参数
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BehaviorAdjustment {
    pub verbosity: f32,     // 0..2 (0.5=简短, 1.0=正常, 1.5=详细)
    pub confidence: f32,    // 0..1 (被纠正后降低)
    pub empathy_boost: f32, // 0..1 (用户情绪低落时升高)
    pub humor_level: f32,   // 0..1 (满意度高时可以更多幽默)
}

impl Default for BehaviorAdjustment {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorAdjustment {
    pub fn new() -> Self {
        Self {
            verbosity: 1.0,
            confidence: 0.8,
            empathy_boost: 0.3,
            humor_level: 0.3,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 主结构: FeedbackLoop
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedbackLoop {
    /// 最近 N 条反馈信号（滑动窗口）
    signals: Vec<FeedbackSignal>,
    signal_window: usize,
    /// 满意度分数 (EMA, 初始 0.5)
    satisfaction: f32,
    satisfaction_alpha: f32,
    /// 行为调节
    adjustment: BehaviorAdjustment,
    /// 追踪连续同话题消息数（用于检测深入追问）
    consecutive_same_topic: u32,
    last_topic_hash: u64,
    /// 上一条 AI 回复的情绪标签（用于关联反馈）
    last_ai_emotion: String,
}

impl Default for FeedbackLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl FeedbackLoop {
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            signal_window: 50,
            satisfaction: 0.5,
            satisfaction_alpha: 0.15,
            adjustment: BehaviorAdjustment::new(),
            consecutive_same_topic: 0,
            last_topic_hash: 0,
            last_ai_emotion: String::new(),
        }
    }

    pub fn with_config(satisfaction_alpha: f32, signal_window: usize) -> Self {
        Self {
            signals: Vec::new(),
            signal_window,
            satisfaction: 0.5,
            satisfaction_alpha,
            adjustment: BehaviorAdjustment::new(),
            consecutive_same_topic: 0,
            last_topic_hash: 0,
            last_ai_emotion: String::new(),
        }
    }

    /// 每条用户消息后调用 — 检测反馈信号并更新状态
    pub fn on_user_message(&mut self, msg: &str) {
        let now = Local::now().timestamp_millis();

        // 1. 检测纠正信号
        if let Some(strength) = detect_correction(msg) {
            self.signals.push(FeedbackSignal::Correction {
                strength,
                timestamp: now,
            });
            self.adjustment.confidence = (self.adjustment.confidence - 0.08 * strength).max(0.1);
        }

        // 2. 检测赞扬信号
        if detect_praise(msg) {
            self.signals.push(FeedbackSignal::Praise { timestamp: now });
            self.adjustment.confidence = (self.adjustment.confidence + 0.02).min(1.0);
        }

        // 3. 检测挫败信号
        if detect_frustration(msg) {
            self.signals
                .push(FeedbackSignal::Frustration { timestamp: now });
            self.adjustment.empathy_boost = (self.adjustment.empathy_boost + 0.15).min(1.0);
        }

        // 4. 检测话题深入 (simple topic hash based on first few significant words)
        let topic_hash = simple_topic_hash(msg);
        if topic_hash == self.last_topic_hash && self.last_topic_hash != 0 {
            self.consecutive_same_topic += 1;
            if self.consecutive_same_topic >= 2 {
                self.signals.push(FeedbackSignal::Deepening {
                    depth: self.consecutive_same_topic,
                    timestamp: now,
                });
            }
        } else {
            if self.last_topic_hash != 0 && self.consecutive_same_topic > 0 {
                self.signals
                    .push(FeedbackSignal::TopicShift { timestamp: now });
            }
            self.consecutive_same_topic = 1;
            self.last_topic_hash = topic_hash;
        }

        // 5. 更新满意度 (EMA)
        let latest_delta = self
            .signals
            .last()
            .map(|s| s.satisfaction_delta())
            .unwrap_or(0.0);
        self.satisfaction =
            (self.satisfaction + self.satisfaction_alpha * latest_delta).clamp(0.0, 1.0);

        // 6. 调节 verbosity (基于用户消息平均长度)
        // Short user messages → AI should be more concise
        // Long user messages → AI can be more detailed
        if msg.len() < 15 {
            self.adjustment.verbosity = (self.adjustment.verbosity * 0.95).max(0.5);
        } else if msg.len() > 100 {
            self.adjustment.verbosity = (self.adjustment.verbosity * 1.02).min(2.0);
        }

        // 7. Humor correlates with satisfaction
        self.adjustment.humor_level = (0.2 + self.satisfaction * 0.5).min(1.0);

        // 8. Empathy decay (slowly return to baseline)
        self.adjustment.empathy_boost = (self.adjustment.empathy_boost * 0.98).max(0.2);

        // 9. Confidence slow recovery
        self.adjustment.confidence = (self.adjustment.confidence + 0.001).min(0.9);

        // 10. Trim signal window
        while self.signals.len() > self.signal_window {
            self.signals.remove(0);
        }
    }

    /// AI 回复后调用 — 记录 AI 行为
    pub fn on_ai_reply(&mut self, _reply: &str, emotion_label: &str) {
        self.last_ai_emotion = emotion_label.to_string();
    }

    /// 获取当前满意度
    pub fn satisfaction(&self) -> f32 {
        self.satisfaction
    }

    /// 获取行为调节参数
    pub fn behavior_adjustment(&self) -> &BehaviorAdjustment {
        &self.adjustment
    }

    /// 生成 LLM 系统提示片段
    pub fn prompt_fragment(&self) -> String {
        let sat_desc = if self.satisfaction > 0.7 {
            "较高"
        } else if self.satisfaction > 0.4 {
            "适中"
        } else {
            "偏低"
        };

        let verbosity_desc = if self.adjustment.verbosity > 1.3 {
            "详细"
        } else if self.adjustment.verbosity > 0.7 {
            "适中"
        } else {
            "简洁"
        };

        let _confidence_desc = if self.adjustment.confidence > 0.7 {
            "自信"
        } else if self.adjustment.confidence > 0.4 {
            "谨慎"
        } else {
            "不确定"
        };

        // Count recent signals by type
        let recent_corrections = self
            .signals
            .iter()
            .filter(|s| matches!(s, FeedbackSignal::Correction { .. }))
            .count();
        let recent_praises = self
            .signals
            .iter()
            .filter(|s| matches!(s, FeedbackSignal::Praise { .. }))
            .count();

        let mut parts = vec![format!("用户满意度{}({:.2})", sat_desc, self.satisfaction)];
        parts.push(format!("建议{}回复", verbosity_desc));
        if self.adjustment.empathy_boost > 0.5 {
            parts.push("需要更多共情".into());
        }
        if recent_corrections > 3 {
            parts.push("近期被多次纠正，请更加谨慎".into());
        }
        if recent_praises > 3 {
            parts.push("用户认可度高，可适度增加幽默".into());
        }
        format!("反馈：{}。", parts.join("，"))
    }

    /// 健康检查状态字符串
    pub fn health_status(&self) -> String {
        format!(
            "sat={:.2} signals={} conf={:.2} emp={:.2}",
            self.satisfaction,
            self.signals.len(),
            self.adjustment.confidence,
            self.adjustment.empathy_boost
        )
    }

    /// 获取最近的反馈信号（只读）
    pub fn recent_signals(&self) -> &[FeedbackSignal] {
        &self.signals
    }

    /// 获取上一条 AI 情绪标签
    pub fn last_ai_emotion(&self) -> &str {
        &self.last_ai_emotion
    }
}

// ════════════════════════════════════════════════════════════════════
// 信号检测函数
// ════════════════════════════════════════════════════════════════════

/// 检测纠正信号，返回 strength (0.5=弱纠正, 1.0=强纠正)
fn detect_correction(msg: &str) -> Option<f32> {
    let strong = [
        "不对",
        "错了",
        "不是这样",
        "你说错",
        "搞错了",
        "说反了",
        "wrong",
        "incorrect",
        "no that's not",
        "that's wrong",
        "you're wrong",
    ];
    let mild = [
        "其实",
        "实际上",
        "应该是",
        "不完全是",
        "not exactly",
        "not quite",
    ];

    if strong.iter().any(|p| msg.contains(p)) {
        Some(1.0)
    } else if mild.iter().any(|p| msg.contains(p)) {
        Some(0.5)
    } else {
        None
    }
}

/// 检测赞扬信号
fn detect_praise(msg: &str) -> bool {
    let words = [
        "说得好",
        "太对了",
        "就是这样",
        "好棒",
        "厉害",
        "你真聪明",
        "不错",
        "awesome",
        "great",
        "perfect",
        "well done",
        "good job",
        "amazing",
        "excellent",
    ];
    words.iter().any(|p| msg.contains(p))
}

/// 检测挫败/不耐烦信号
fn detect_frustration(msg: &str) -> bool {
    let words = [
        "算了",
        "随便",
        "无所谓",
        "不想说",
        "不想聊了",
        "烦死了",
        "whatever",
        "never mind",
        "forget it",
        "i give up",
        "sigh",
        "ugh",
    ];
    words.iter().any(|p| msg.contains(p))
}

/// 简单话题哈希（取消息中的关键内容词，生成稳定哈希）
fn simple_topic_hash(msg: &str) -> u64 {
    // Take first 3 significant Chinese words or English words
    // Use a simple hash to detect topic continuity
    // Ignore very short messages (< 5 chars)
    if msg.len() < 5 {
        return 0;
    }
    let words: Vec<&str> = msg
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .take(3)
        .collect();
    if words.is_empty() {
        return 0;
    }
    // Simple hash: wrapping multiply-accumulate
    words
        .iter()
        .flat_map(|w| w.chars())
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64))
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 默认值测试 ──

    #[test]
    fn test_new_creates_valid_defaults() {
        let fl = FeedbackLoop::new();
        assert!((fl.satisfaction - 0.5).abs() < 1e-6);
        assert!((fl.adjustment.confidence - 0.8).abs() < 1e-6);
        assert!((fl.adjustment.verbosity - 1.0).abs() < 1e-6);
        assert!((fl.adjustment.empathy_boost - 0.3).abs() < 1e-6);
        assert!((fl.adjustment.humor_level - 0.3).abs() < 1e-6);
        assert_eq!(fl.signal_window, 50);
        assert!(fl.signals.is_empty());
        assert_eq!(fl.consecutive_same_topic, 0);
        assert_eq!(fl.last_topic_hash, 0);
        assert!(fl.last_ai_emotion.is_empty());
    }

    #[test]
    fn test_with_config() {
        let fl = FeedbackLoop::with_config(0.25, 100);
        assert!((fl.satisfaction_alpha - 0.25).abs() < 1e-6);
        assert_eq!(fl.signal_window, 100);
        assert!((fl.satisfaction - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_default_trait() {
        let fl = FeedbackLoop::default();
        assert!((fl.satisfaction - 0.5).abs() < 1e-6);
    }

    // ── 纠正检测测试 ──

    #[test]
    fn test_strong_correction_detected() {
        let result = detect_correction("你说错了，这个答案不对");
        assert!(result.is_some());
        assert!((result.unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_strong_correction_english() {
        let result = detect_correction("That's wrong, please check again");
        assert!(result.is_some());
        assert!((result.unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mild_correction_detected() {
        let result = detect_correction("其实应该是用 Rust 的 async 语法");
        assert!(result.is_some());
        assert!((result.unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_no_correction_from_normal_message() {
        let result = detect_correction("今天天气不错");
        assert!(result.is_none());
    }

    // ── 赞扬检测测试 ──

    #[test]
    fn test_praise_detected() {
        assert!(detect_praise("太对了！就是这样"));
    }

    #[test]
    fn test_praise_english() {
        assert!(detect_praise("That's awesome, great work!"));
    }

    #[test]
    fn test_no_praise_from_neutral_message() {
        assert!(!detect_praise("请帮我看看这段代码"));
    }

    // ── 挫败检测测试 ──

    #[test]
    fn test_frustration_detected() {
        assert!(detect_frustration("算了不想说了"));
    }

    #[test]
    fn test_frustration_english() {
        assert!(detect_frustration("Never mind, forget it"));
    }

    #[test]
    fn test_no_frustration_from_normal_message() {
        assert!(!detect_frustration("请继续解释一下"));
    }

    // ── 满意度变化测试 ──

    #[test]
    fn test_satisfaction_decreases_after_correction() {
        let mut fl = FeedbackLoop::new();
        let initial = fl.satisfaction();
        fl.on_user_message("不对，你说错了");
        assert!(fl.satisfaction() < initial);
    }

    #[test]
    fn test_satisfaction_increases_after_praise() {
        let mut fl = FeedbackLoop::new();
        let initial = fl.satisfaction();
        fl.on_user_message("太对了！说得好");
        assert!(fl.satisfaction() >= initial);
    }

    #[test]
    fn test_satisfaction_clamped_to_unit_interval() {
        // Drive satisfaction down with many frustrations
        let mut fl = FeedbackLoop::new();
        for _ in 0..200 {
            fl.on_user_message("算了烦死了");
        }
        assert!(fl.satisfaction() >= 0.0);
        assert!(fl.satisfaction() <= 1.0);

        // Drive satisfaction up with many praises
        let mut fl2 = FeedbackLoop::new();
        for _ in 0..200 {
            fl2.on_user_message("太对了 awesome 好棒");
        }
        assert!(fl2.satisfaction() >= 0.0);
        assert!(fl2.satisfaction() <= 1.0);
    }

    // ── 行为调节测试 ──

    #[test]
    fn test_confidence_decreases_after_correction() {
        let mut fl = FeedbackLoop::new();
        let initial_conf = fl.behavior_adjustment().confidence;
        fl.on_user_message("不对，搞错了");
        // Confidence should have decreased (net effect after recovery tick)
        assert!(fl.behavior_adjustment().confidence < initial_conf);
    }

    #[test]
    fn test_confidence_recovers_slowly() {
        let mut fl = FeedbackLoop::new();
        // Apply a correction to drop confidence
        fl.on_user_message("你说错了");
        let conf_after_correction = fl.behavior_adjustment().confidence;

        // Send several neutral messages to let confidence recover
        for _ in 0..20 {
            fl.on_user_message("请继续讲");
        }
        // Confidence should have recovered somewhat
        assert!(fl.behavior_adjustment().confidence > conf_after_correction);
        // But should not exceed 0.9 cap
        assert!(fl.behavior_adjustment().confidence <= 0.9);
    }

    #[test]
    fn test_empathy_boost_increases_after_frustration() {
        let mut fl = FeedbackLoop::new();
        let initial_emp = fl.behavior_adjustment().empathy_boost;
        fl.on_user_message("算了不想聊了");
        // Empathy should have increased (net effect after decay tick)
        // With initial 0.3, +0.15 = 0.45, *0.98 ≈ 0.441 — still > initial 0.3
        assert!(fl.behavior_adjustment().empathy_boost > initial_emp);
    }

    #[test]
    fn test_empathy_decays_over_time() {
        let mut fl = FeedbackLoop::new();
        // Spike empathy with frustration
        fl.on_user_message("算了烦死了");
        let emp_after_frustration = fl.behavior_adjustment().empathy_boost;

        // Send neutral messages — empathy should decay
        for _ in 0..50 {
            fl.on_user_message("好的继续");
        }
        assert!(fl.behavior_adjustment().empathy_boost < emp_after_frustration);
        // Should not go below 0.2 baseline
        assert!(fl.behavior_adjustment().empathy_boost >= 0.2);
    }

    // ── 话题深入 / 话题切换测试 ──

    #[test]
    fn test_deepening_detected_after_consecutive_same_topic() {
        let mut fl = FeedbackLoop::new();
        // Use long enough messages with the same leading words to produce
        // the same topic hash
        fl.on_user_message("Rust 异步编程 tokio 基础用法");
        fl.on_user_message("Rust 异步编程 tokio 进阶技巧");
        fl.on_user_message("Rust 异步编程 tokio 性能调优");

        let has_deepening = fl
            .recent_signals()
            .iter()
            .any(|s| matches!(s, FeedbackSignal::Deepening { .. }));
        assert!(
            has_deepening,
            "Expected Deepening signal after 3 same-topic messages"
        );
    }

    #[test]
    fn test_topic_shift_detected() {
        let mut fl = FeedbackLoop::new();
        fl.on_user_message("Rust 异步编程相关内容");
        fl.on_user_message("Python 数据分析机器学习");

        let has_topic_shift = fl
            .recent_signals()
            .iter()
            .any(|s| matches!(s, FeedbackSignal::TopicShift { .. }));
        assert!(has_topic_shift, "Expected TopicShift signal");
    }

    // ── verbosity 调节测试 ──

    #[test]
    fn test_verbosity_decreases_with_short_messages() {
        let mut fl = FeedbackLoop::new();
        let initial = fl.behavior_adjustment().verbosity;
        // Send many short messages (< 15 chars)
        for _ in 0..20 {
            fl.on_user_message("好");
        }
        assert!(fl.behavior_adjustment().verbosity < initial);
        assert!(fl.behavior_adjustment().verbosity >= 0.5);
    }

    #[test]
    fn test_verbosity_increases_with_long_messages() {
        let mut fl = FeedbackLoop::new();
        let initial = fl.behavior_adjustment().verbosity;
        let long_msg = "这是一条非常长的消息，里面包含了很多详细的信息和具体的需求描述，\
 用户在这里详细地阐述了自己的想法和期望，希望 AI 能够给出更加全面和深入的回答。\
 所以 AI 应该根据这个消息的长度来适当增加回复的详细程度。";
        for _ in 0..10 {
            fl.on_user_message(long_msg);
        }
        assert!(fl.behavior_adjustment().verbosity > initial);
        assert!(fl.behavior_adjustment().verbosity <= 2.0);
    }

    // ── prompt_fragment 测试 ──

    #[test]
    fn test_prompt_fragment_produces_valid_string() {
        let fl = FeedbackLoop::new();
        let fragment = fl.prompt_fragment();
        assert!(fragment.contains("反馈："));
        assert!(fragment.contains("满意度"));
        assert!(fragment.contains("建议"));
        // Should end with "。"
        assert!(fragment.ends_with('。'));
    }

    #[test]
    fn test_prompt_fragment_after_corrections() {
        let mut fl = FeedbackLoop::new();
        // Generate more than 3 corrections to trigger the warning
        for _ in 0..5 {
            fl.on_user_message("不对，搞错了");
        }
        let fragment = fl.prompt_fragment();
        assert!(fragment.contains("近期被多次纠正"));
    }

    #[test]
    fn test_prompt_fragment_after_praises() {
        let mut fl = FeedbackLoop::new();
        // Generate more than 3 praises
        for _ in 0..5 {
            fl.on_user_message("太对了 awesome");
        }
        let fragment = fl.prompt_fragment();
        assert!(fragment.contains("用户认可度高"));
    }

    // ── health_status 测试 ──

    #[test]
    fn test_health_status_produces_valid_string() {
        let fl = FeedbackLoop::new();
        let status = fl.health_status();
        assert!(status.contains("sat="));
        assert!(status.contains("signals="));
        assert!(status.contains("conf="));
        assert!(status.contains("emp="));
    }

    // ── 信号窗口裁剪测试 ──

    #[test]
    fn test_signal_window_trimming() {
        let mut fl = FeedbackLoop::with_config(0.15, 5);
        // Generate more signals than the window allows
        for i in 0..10 {
            // Alternate between correction and praise to generate signals each time
            if i % 2 == 0 {
                fl.on_user_message("不对搞错了");
            } else {
                fl.on_user_message("太对了 awesome");
            }
        }
        assert!(fl.recent_signals().len() <= 5);
    }

    // ── on_ai_reply 测试 ──

    #[test]
    fn test_on_ai_reply_stores_emotion_label() {
        let mut fl = FeedbackLoop::new();
        assert!(fl.last_ai_emotion().is_empty());

        fl.on_ai_reply("这是 AI 的回复内容", "friendly");
        assert_eq!(fl.last_ai_emotion(), "friendly");

        fl.on_ai_reply("另一条回复", "neutral");
        assert_eq!(fl.last_ai_emotion(), "neutral");
    }

    // ── 综合场景测试 ──

    #[test]
    fn test_full_interaction_cycle() {
        let mut fl = FeedbackLoop::new();

        // Simulate a realistic conversation
        fl.on_user_message("我想学 Rust 编程语言");
        fl.on_ai_reply("Rust 是一门系统编程语言...", "encouraging");

        fl.on_user_message("太对了，就是这样");
        fl.on_ai_reply("很高兴对你有帮助", "friendly");

        fl.on_user_message("其实应该是 async/await 不是 future");
        fl.on_ai_reply("感谢纠正！", "humble");

        fl.on_user_message("算了不想说了");
        fl.on_ai_reply("没关系，我们慢慢来", "empathetic");

        // Verify state is consistent
        assert!(fl.satisfaction() >= 0.0 && fl.satisfaction() <= 1.0);
        assert!(fl.behavior_adjustment().confidence >= 0.1);
        assert!(fl.behavior_adjustment().empathy_boost >= 0.2);
        assert!(fl.behavior_adjustment().verbosity >= 0.5);
        assert!(!fl.recent_signals().is_empty());

        // Health status should not panic
        let _status = fl.health_status();
        let _fragment = fl.prompt_fragment();
    }

    // ── 序列化 / 反序列化测试 ──

    #[test]
    fn test_serde_roundtrip() {
        let mut fl = FeedbackLoop::new();
        fl.on_user_message("太对了 awesome");
        fl.on_ai_reply("谢谢", "happy");

        let json = serde_json::to_string(&fl).expect("serialize failed");
        let fl2: FeedbackLoop = serde_json::from_str(&json).expect("deserialize failed");

        assert!((fl2.satisfaction() - fl.satisfaction()).abs() < 1e-6);
        assert_eq!(fl2.recent_signals().len(), fl.recent_signals().len());
        assert_eq!(fl2.last_ai_emotion(), fl.last_ai_emotion());
    }

    // ── simple_topic_hash 测试 ──

    #[test]
    fn test_topic_hash_short_message_returns_zero() {
        assert_eq!(simple_topic_hash(""), 0);
        assert_eq!(simple_topic_hash("hi"), 0);
        assert_eq!(simple_topic_hash("ab"), 0);
    }

    #[test]
    fn test_topic_hash_same_topic_same_hash() {
        let h1 = simple_topic_hash("Rust async tokio runtime basics");
        let h2 = simple_topic_hash("Rust async tokio advanced patterns");
        // First 3 significant words are the same → same hash
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_topic_hash_different_topic_different_hash() {
        let h1 = simple_topic_hash("Rust async tokio runtime");
        let h2 = simple_topic_hash("Python machine learning data");
        assert_ne!(h1, h2);
    }

    // ── FeedbackSignal::satisfaction_delta 测试 ──

    #[test]
    fn test_satisfaction_delta_values() {
        let correction = FeedbackSignal::Correction {
            strength: 1.0,
            timestamp: 0,
        };
        assert!((correction.satisfaction_delta() - (-0.05)).abs() < 1e-6);

        let correction_mild = FeedbackSignal::Correction {
            strength: 0.5,
            timestamp: 0,
        };
        assert!((correction_mild.satisfaction_delta() - (-0.025)).abs() < 1e-6);

        let praise = FeedbackSignal::Praise { timestamp: 0 };
        assert!((praise.satisfaction_delta() - 0.03).abs() < 1e-6);

        let frustration = FeedbackSignal::Frustration { timestamp: 0 };
        assert!((frustration.satisfaction_delta() - (-0.08)).abs() < 1e-6);

        let shift = FeedbackSignal::TopicShift { timestamp: 0 };
        assert!((shift.satisfaction_delta()).abs() < 1e-6);

        let deepening = FeedbackSignal::Deepening {
            depth: 3,
            timestamp: 0,
        };
        assert!((deepening.satisfaction_delta() - 0.02).abs() < 1e-6);
    }
}
