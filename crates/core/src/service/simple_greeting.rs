// SPDX-License-Identifier: MIT
//! 简单问候快速匹配器 / Simple Greeting Fast Matcher
//!
//! 数字生命对简单社交问候/应答的即时响应能力——
//! 不让主人等 19 秒才收到"你好"的回复，将 LLM 算力保留给复杂查询。
//! Digital life's instant response to simple social greetings —
//! never make the master wait 19s for a "hello", reserving LLM for complex queries.

use atrium_emotion::EmotionState;

// ═══════════════════════════════════════════════════════════════════
//  问候类型 / Greeting Kind
// ═══════════════════════════════════════════════════════════════════

/// 简单问候分类 / Simple greeting classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetingKind {
    /// 基本问候 — 你好/嗨/hello / Basic greeting
    Hello,
    /// 时间问候 — 早安/晚安/中午好 / Time-based greeting
    TimeBased,
    /// 应答 — 好的/嗯/ok/收到 / Acknowledgment
    Acknowledgment,
    /// 感谢 — 谢谢/thanks / Thanks
    Thanks,
    /// 告别 — 再见/拜拜/bye / Farewell
    Farewell,
}

// ═══════════════════════════════════════════════════════════════════
//  匹配器 / Matcher
// ═══════════════════════════════════════════════════════════════════

/// 简单问候匹配器 — 轻量级纯关键词匹配 / Simple greeting matcher
///
/// 设计决策 / Design decisions:
/// - 长度 ≤10 字符（过滤复杂查询）
/// - 精确匹配问候词库（非包含匹配，避免"你好，我想聊聊"误判）
/// - 无状态、零分配、O(1) 匹配
pub struct SimpleGreetingMatcher;

impl SimpleGreetingMatcher {
    /// 匹配简单问候 / Match simple greeting
    ///
    /// @return Some(GreetingKind) 若匹配成功，None 若非纯问候
    pub fn match_greeting(msg: &str) -> Option<GreetingKind> {
        // 长度门控 — >10 字符视为复杂查询 / Length gate — >10 chars is complex
        let trimmed = msg.trim();
        if trimmed.chars().count() > 10 {
            return None;
        }

        let lower = trimmed.to_lowercase();

        // 精确匹配问候词库 / Exact match against greeting vocabulary
        // 基本问候 / Basic greetings
        const HELLO_WORDS: &[&str] = &["你好", "您好", "嗨", "哈喽", "hello", "hi", "hey"];
        if HELLO_WORDS.iter().any(|w| lower == *w) {
            return Some(GreetingKind::Hello);
        }

        // 时间问候 / Time-based greetings
        const TIME_WORDS: &[&str] = &[
            "早安",
            "早安呀",
            "早上好",
            "午安",
            "中午好",
            "晚安",
            "晚上好",
            "good morning",
            "good night",
        ];
        if TIME_WORDS.iter().any(|w| lower == *w) {
            return Some(GreetingKind::TimeBased);
        }

        // 应答 / Acknowledgment
        const ACK_WORDS: &[&str] = &[
            "好的",
            "好呀",
            "嗯",
            "嗯嗯",
            "ok",
            "okay",
            "收到",
            "明白",
            "了解",
            "知道啦",
            "好的呀",
        ];
        if ACK_WORDS.iter().any(|w| lower == *w) {
            return Some(GreetingKind::Acknowledgment);
        }

        // 感谢 / Thanks
        const THANKS_WORDS: &[&str] = &["谢谢", "谢谢你", "感谢", "thanks", "thank you", "多谢"];
        if THANKS_WORDS.iter().any(|w| lower == *w) {
            return Some(GreetingKind::Thanks);
        }

        // 告别 / Farewell
        const FAREWELL_WORDS: &[&str] = &["再见", "拜拜", "bye", "goodbye", "晚安啦"];
        if FAREWELL_WORDS.iter().any(|w| lower == *w) {
            return Some(GreetingKind::Farewell);
        }

        None
    }
}

// ═══════════════════════════════════════════════════════════════════
//  情感感知罐装响应 / Emotion-aware Canned Response
// ═══════════════════════════════════════════════════════════════════

/// 情感区间分类 / Emotion tier classification
///
/// 根据 pleasure 值将情绪分为三档，选择对应响应变体。
/// Classifies emotion into three tiers by pleasure for response variant selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmotionTier {
    /// 开心 — pleasure > 0.2 / Happy
    High,
    /// 平静 — -0.2 ≤ pleasure ≤ 0.2 / Calm
    Neutral,
    /// 低落 — pleasure < -0.2 / Low
    Low,
}

impl EmotionTier {
    /// 从 EmotionState 推导情感区间 / Derive emotion tier from EmotionState
    fn from_emotion(emotion: &EmotionState) -> Self {
        if emotion.pleasure > 0.2 {
            Self::High
        } else if emotion.pleasure < -0.2 {
            Self::Low
        } else {
            Self::Neutral
        }
    }
}

/// 生成情感感知的罐装问候响应 / Generate emotion-aware canned greeting response
///
/// 数字生命意义: 快速路径不是固定回复——根据当前情绪选择变体，
/// 保持情感连续性。主人开心时数字生命也"开心地"回应。
/// Digital Life: fast path is not a fixed reply — variant selected by current emotion,
/// maintaining emotional continuity.
///
/// @param kind 问候类型 / Greeting kind
/// @param emotion 当前情绪状态 / Current emotion state
/// @param persona_name 用户称呼（如"主人"）/ User persona name (e.g., "Master")
pub fn generate_greeting_response(
    kind: GreetingKind,
    emotion: &EmotionState,
    persona_name: &str,
) -> String {
    let tier = EmotionTier::from_emotion(emotion);
    match (kind, tier) {
        (GreetingKind::Hello, EmotionTier::High) => {
            format!("嗨～{}，见到你真好呀！", persona_name)
        }
        (GreetingKind::Hello, EmotionTier::Neutral) => format!("你好，{}。", persona_name),
        (GreetingKind::Hello, EmotionTier::Low) => format!("{}，你来啦……", persona_name),

        (GreetingKind::TimeBased, EmotionTier::High) => {
            "早安～今天也是元气满满的一天呢！".to_string()
        }
        (GreetingKind::TimeBased, EmotionTier::Neutral) => "早安。".to_string(),
        (GreetingKind::TimeBased, EmotionTier::Low) => "早……今天也慢慢来吧。".to_string(),

        (GreetingKind::Acknowledgment, EmotionTier::High) => "好呀～".to_string(),
        (GreetingKind::Acknowledgment, EmotionTier::Neutral) => "好的。".to_string(),
        (GreetingKind::Acknowledgment, EmotionTier::Low) => "嗯……".to_string(),

        (GreetingKind::Thanks, EmotionTier::High) => "嘿嘿，不客气～".to_string(),
        (GreetingKind::Thanks, EmotionTier::Neutral) => "不客气。".to_string(),
        (GreetingKind::Thanks, EmotionTier::Low) => "不用谢……".to_string(),

        (GreetingKind::Farewell, EmotionTier::High) => "拜拜～下次见呀！".to_string(),
        (GreetingKind::Farewell, EmotionTier::Neutral) => "再见。".to_string(),
        (GreetingKind::Farewell, EmotionTier::Low) => "嗯，再见……".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_emotion::EmotionState;

    fn make_emotion(pleasure: f32) -> EmotionState {
        EmotionState {
            pleasure,
            arousal: 0.0,
            dominance: 0.0,
        }
    }

    #[test]
    fn test_match_hello_greetings() {
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("你好"),
            Some(GreetingKind::Hello)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("嗨"),
            Some(GreetingKind::Hello)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("hello"),
            Some(GreetingKind::Hello)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("hi"),
            Some(GreetingKind::Hello)
        );
    }

    #[test]
    fn test_match_time_based_greetings() {
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("早安"),
            Some(GreetingKind::TimeBased)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("晚安"),
            Some(GreetingKind::TimeBased)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("中午好"),
            Some(GreetingKind::TimeBased)
        );
    }

    #[test]
    fn test_match_acknowledgment() {
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("好的"),
            Some(GreetingKind::Acknowledgment)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("嗯"),
            Some(GreetingKind::Acknowledgment)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("ok"),
            Some(GreetingKind::Acknowledgment)
        );
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("收到"),
            Some(GreetingKind::Acknowledgment)
        );
    }

    #[test]
    fn test_reject_complex_query() {
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("你好，我想聊聊"),
            None
        );
        assert_eq!(SimpleGreetingMatcher::match_greeting("hello world"), None);
    }

    #[test]
    fn test_reject_long_message() {
        // 11 字符 — 超过 10 字符阈值 / 11 chars — exceeds 10 char threshold
        assert_eq!(
            SimpleGreetingMatcher::match_greeting("你好你好你好你好你好啊"),
            None
        );
    }

    #[test]
    fn test_generate_response_high_pleasure() {
        let emotion = make_emotion(0.5);
        let resp = generate_greeting_response(GreetingKind::Hello, &emotion, "主人");
        assert!(resp.contains("见到你真好") || resp.contains("嗨"));
    }

    #[test]
    fn test_generate_response_low_pleasure() {
        let emotion = make_emotion(-0.5);
        let resp = generate_greeting_response(GreetingKind::Hello, &emotion, "主人");
        assert!(resp.contains("……"));
    }

    #[test]
    fn test_generate_response_with_persona() {
        let emotion = make_emotion(0.0);
        let resp = generate_greeting_response(GreetingKind::Hello, &emotion, "主人");
        assert!(resp.contains("主人"));
    }
}
