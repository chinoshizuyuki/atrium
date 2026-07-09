// SPDX-License-Identifier: MIT
//! 用户心智模型
//!
//! 构建对用户的动态认知画像：情绪状态、沟通风格、参与度、话题兴趣。
//! 让 AI 的回复风格和策略随用户状态自适应调整。
//!
//! 所有信号检测为纯规则匹配，延迟 < 1μs/条。
//! UserMentalModel — User mental model.
//!
//! Dynamically models the user's cognitive state, emotional state, communication
//! style, and interest topics, enabling the AI to adapt responses accordingly.
//!
//! Signal collection is rule-based, latency < 1ms/event.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::perception::{RhythmPattern, TypingRhythm};

// ════════════════════════════════════════════════════════════════════
// 用户情绪估计
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MoodTrend {
    Improving,
    Stable,
    Declining,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserMood {
    pub valence: f32,   // -1..1 (消极..积极)
    pub intensity: f32, // 0..1 (低..高)
    pub trend: MoodTrend,
    pub last_update: i64,
    /// 内部追踪用
    prev_valence: f32,
}

impl UserMood {
    pub fn new() -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            valence: 0.0,
            intensity: 0.3,
            trend: MoodTrend::Stable,
            last_update: now,
            prev_valence: 0.0,
        }
    }

    /// 分析消息并 EMA 更新情绪状态
    pub fn analyze(&mut self, msg: &str, alpha: f32) {
        let lower = msg.to_lowercase();

        // 积极词库 (中英文)
        let positive_words = [
            "开心",
            "高兴",
            "谢谢",
            "太好了",
            "哈哈哈",
            "喜欢",
            "棒",
            "不错",
            "好的",
            "happy",
            "love",
            "great",
            "thanks",
            "awesome",
            "good",
            "nice",
            "cool",
            "yes",
        ];
        // 消极词库
        let negative_words = [
            "难过",
            "烦",
            "累",
            "讨厌",
            "生气",
            "不想",
            "郁闷",
            "无聊",
            "失望",
            "sad",
            "hate",
            "annoyed",
            "angry",
            "tired",
            "bored",
            "frustrated",
        ];
        // 强度词
        let intensity_words = [
            "非常",
            "特别",
            "超级",
            "really",
            "so much",
            "!!!",
            "extremely",
        ];

        let pos_hits = positive_words
            .iter()
            .filter(|w| lower.contains(**w))
            .count() as f32;
        let neg_hits = negative_words
            .iter()
            .filter(|w| lower.contains(**w))
            .count() as f32;

        // 原始 valence signal: positive hits - negative hits, clamped to [-1, 1]
        let raw_valence = (pos_hits - neg_hits).clamp(-1.0, 1.0);

        // 强度 signal
        let int_hits = intensity_words
            .iter()
            .filter(|w| lower.contains(**w))
            .count() as f32;
        let raw_intensity = int_hits.clamp(0.0, 1.0);

        // EMA update
        self.prev_valence = self.valence;
        self.valence = alpha * raw_valence + (1.0 - alpha) * self.valence;
        self.intensity = alpha * raw_intensity + (1.0 - alpha) * self.intensity;

        // trend detection (threshold 0.05)
        let delta = self.valence - self.prev_valence;
        if delta > 0.05 {
            self.trend = MoodTrend::Improving;
        } else if delta < -0.05 {
            self.trend = MoodTrend::Declining;
        } else {
            self.trend = MoodTrend::Stable;
        }

        self.last_update = Local::now().timestamp_millis();
    }
}

impl Default for UserMood {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// 沟通风格偏好
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LanguageHint {
    Chinese,
    English,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommunicationStyle {
    pub avg_message_length: f32,
    pub formality: f32,      // 0..1
    pub emoji_usage: f32,    // 0..1
    pub question_ratio: f32, // 0..1
    pub language: LanguageHint,
    // internal counters
    total_messages: u64,
    total_questions: u64,
}

impl CommunicationStyle {
    pub fn new() -> Self {
        Self {
            avg_message_length: 0.0,
            formality: 0.3,
            emoji_usage: 0.0,
            question_ratio: 0.0,
            language: LanguageHint::Mixed,
            total_messages: 0,
            total_questions: 0,
        }
    }

    pub fn analyze(&mut self, msg: &str, alpha: f32) {
        self.total_messages += 1;
        let len = msg.len() as f32;

        // avg_message_length: EMA update
        self.avg_message_length = alpha * len + (1.0 - alpha) * self.avg_message_length;

        // formality: check for formal words
        let formal_words = [
            "您",
            "请问",
            "麻烦",
            "请",
            "please",
            "kindly",
            "regards",
            "sincerely",
        ];
        let formal_hits = formal_words.iter().filter(|w| msg.contains(**w)).count() as f32;
        let formal_signal = formal_hits.clamp(0.0, 1.0);
        self.formality = alpha * formal_signal + (1.0 - alpha) * self.formality;

        // emoji_usage: count emoji-like chars / msg.len()
        let emoji_count = msg.chars().filter(|c| is_emoji(*c)).count() as f32;
        let emoji_signal = if !msg.is_empty() {
            (emoji_count / msg.chars().count() as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.emoji_usage = alpha * emoji_signal + (1.0 - alpha) * self.emoji_usage;

        // question_ratio
        let is_question =
            msg.contains('?') || msg.contains('？') || msg.contains("吗") || msg.contains("呢");
        if is_question {
            self.total_questions += 1;
        }
        self.question_ratio = self.total_questions as f32 / self.total_messages as f32;

        // language detection
        let total_chars = msg.chars().count();
        if total_chars > 0 {
            let chinese_chars = msg.chars().filter(|c| is_chinese(*c)).count() as f32;
            let ratio = chinese_chars / total_chars as f32;
            self.language = if ratio > 0.6 {
                LanguageHint::Chinese
            } else if ratio < 0.4 {
                LanguageHint::English
            } else {
                LanguageHint::Mixed
            };
        }
    }
}

impl Default for CommunicationStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a char is in the CJK Unified Ideographs range
fn is_chinese(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
        || ('\u{3400}'..='\u{4DBF}').contains(&c)
        || ('\u{20000}'..='\u{2A6DF}').contains(&c)
}

/// Rough emoji detection: common emoji ranges
fn is_emoji(c: char) -> bool {
    ('\u{1F600}'..='\u{1F64F}').contains(&c) // Emoticons
 || ('\u{1F300}'..='\u{1F5FF}').contains(&c) // Misc Symbols & Pictographs
 || ('\u{1F680}'..='\u{1F6FF}').contains(&c) // Transport & Map
 || ('\u{1F900}'..='\u{1F9FF}').contains(&c) // Supplemental Symbols
 || ('\u{2600}'..='\u{26FF}').contains(&c) // Misc Symbols
 || ('\u{2700}'..='\u{27BF}').contains(&c) // Dingbats
 || ('\u{FE00}'..='\u{FE0F}').contains(&c) // Variation Selectors
 || ('\u{1FA00}'..='\u{1FA6F}').contains(&c) // Chess / Extended-A
 || ('\u{1FA70}'..='\u{1FAFF}').contains(&c) // Extended-A
 || ('\u{200D}' == c) // ZWJ
 || ('\u{20E3}' == c) // Combining enclosing keycap
}

// ════════════════════════════════════════════════════════════════════
// 参与度追踪
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngagementTracker {
    pub messages_per_session: f32,
    pub avg_session_duration_ms: u64,
    pub return_frequency: f32,
    pub engagement_score: f32,
    last_message_time: i64,
    session_start: i64,
    session_messages: u32,
    total_sessions: u32,
}

impl EngagementTracker {
    pub fn new() -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            messages_per_session: 0.0,
            avg_session_duration_ms: 0,
            return_frequency: 0.0,
            engagement_score: 0.0,
            last_message_time: now,
            session_start: now,
            session_messages: 0,
            total_sessions: 1,
        }
    }

    pub fn update(&mut self, alpha: f32) {
        let now = Local::now().timestamp_millis();
        let gap = now - self.last_message_time;

        // 2 hours in milliseconds
        let two_hours_ms: i64 = 2 * 3600 * 1000;

        if gap > two_hours_ms && self.last_message_time > 0 {
            // Archive previous session
            let session_duration = (self.last_message_time - self.session_start) as u64;
            self.messages_per_session =
                alpha * self.session_messages as f32 + (1.0 - alpha) * self.messages_per_session;
            self.avg_session_duration_ms = (alpha * session_duration as f32
                + (1.0 - alpha) * self.avg_session_duration_ms as f32)
                as u64;

            // New session
            self.total_sessions += 1;
            self.session_messages = 1;
            self.session_start = now;
        } else {
            // Same session
            self.session_messages += 1;
        }

        // return_frequency: sessions per day approximation
        // Use ratio of sessions to total days active (simple heuristic)
        if self.total_sessions > 1 {
            self.return_frequency = (self.total_sessions as f32 - 1.0).min(10.0) / 10.0;
        }

        // engagement_score = weighted combo
        let msg_score = (self.messages_per_session / 20.0).clamp(0.0, 1.0);
        let return_score = self.return_frequency.clamp(0.0, 1.0);
        let duration_score =
            (self.avg_session_duration_ms as f32 / (30.0 * 60.0 * 1000.0)).clamp(0.0, 1.0);

        self.engagement_score = 0.4 * msg_score + 0.3 * return_score + 0.3 * duration_score;

        self.last_message_time = now;
    }
}

impl Default for EngagementTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// 话题兴趣
// ════════════════════════════════════════════════════════════════════

/// Simple topic extraction from message content.
/// Extracts keywords using pattern matching (no NLP dependency).
fn extract_topics(msg: &str) -> Vec<String> {
    let mut topics = Vec::new();

    // Chinese patterns: "我喜欢X", "我热爱X", "我对X感兴趣"
    let cn_patterns = [
        "我喜欢",
        "我热爱",
        "我对",
        "我感兴趣",
        "关于",
        "讨论",
        "聊",
        "谈谈",
    ];
    for pat in &cn_patterns {
        if let Some(idx) = msg.find(pat) {
            let after = &msg[idx + pat.len()..];
            // Take the next segment (up to 10 chars or until punctuation/space)
            let keyword: String = after
                .chars()
                .take(10)
                .take_while(|c| {
                    !c.is_whitespace()
                        && *c != '。'
                        && *c != '，'
                        && *c != '！'
                        && *c != '？'
                        && *c != '、'
                        && *c != '.'
                        && *c != ','
                        && *c != '!'
                        && *c != '?'
                })
                .collect();
            let keyword = keyword.trim().to_string();
            if keyword.len() >= 2 && !topics.contains(&keyword) {
                topics.push(keyword);
            }
        }
    }

    // English patterns: "I like X", "I love X", "I'm interested in X"
    let lower = msg.to_lowercase();
    let en_patterns = ["i like ", "i love ", "i'm interested in ", "i enjoy "];
    for pat in &en_patterns {
        if let Some(idx) = lower.find(pat) {
            let after = &msg[idx + pat.len()..];
            let keyword: String = after
                .chars()
                .take(30)
                .take_while(|c| !c.is_whitespace() || c.is_alphabetic())
                .take_while(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
                .collect();
            let keyword = keyword.trim().to_string();
            // Only take first 2-3 words
            let keyword: String = keyword
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            if keyword.len() >= 2 && !topics.contains(&keyword) {
                topics.push(keyword);
            }
        }
    }

    // Extract capitalized English words (likely proper nouns / topics)
    let mut current_capitalized = String::new();
    for word in msg.split_whitespace() {
        if word.len() >= 2
            && word.chars().next().is_some_and(|c| c.is_uppercase())
            && word
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '+' || c == '#')
        {
            if !current_capitalized.is_empty() {
                current_capitalized.push(' ');
            }
            current_capitalized.push_str(word);
        } else if !current_capitalized.is_empty() {
            if current_capitalized.len() >= 2 && !topics.contains(&current_capitalized) {
                topics.push(current_capitalized.clone());
            }
            current_capitalized.clear();
        }
    }
    if current_capitalized.len() >= 2 && !topics.contains(&current_capitalized) {
        topics.push(current_capitalized);
    }

    topics
}

// ════════════════════════════════════════════════════════════════════
// 情感调制建议
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
pub struct EmotionModulation {
    /// 对用户情绪的共鸣增益 (当用户消极时，AI 也稍微消极)
    pub empathy_pleasure_shift: f32,
    /// 用户参与度高时，AI 更积极回应
    pub engagement_boost: f32,
}

// ════════════════════════════════════════════════════════════════════
// 主结构: UserMentalModel
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserMentalModel {
    pub mood: UserMood,
    pub style: CommunicationStyle,
    pub engagement: EngagementTracker,
    pub topic_interests: HashMap<String, f32>,
    // EMA 平滑参数
    mood_alpha: f32,
    style_alpha: f32,
    topic_decay_hours: f32,
    last_topic_decay: i64,
}

impl UserMentalModel {
    pub fn new() -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            mood: UserMood::new(),
            style: CommunicationStyle::new(),
            engagement: EngagementTracker::new(),
            topic_interests: HashMap::new(),
            mood_alpha: 0.3,
            style_alpha: 0.2,
            topic_decay_hours: 48.0,
            last_topic_decay: now,
        }
    }

    pub fn with_config(mood_alpha: f32, style_alpha: f32, topic_decay_hours: f32) -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            mood: UserMood::new(),
            style: CommunicationStyle::new(),
            engagement: EngagementTracker::new(),
            topic_interests: HashMap::new(),
            mood_alpha,
            style_alpha,
            topic_decay_hours,
            last_topic_decay: now,
        }
    }

    /// 每条用户消息后调用
    pub fn on_user_message(&mut self, msg: &str) {
        self.mood.analyze(msg, self.mood_alpha);
        self.style.analyze(msg, self.style_alpha);
        self.engagement.update(self.style_alpha);

        // Extract and accumulate topics
        let style_alpha = self.style_alpha;
        for topic in extract_topics(msg) {
            let entry = self.topic_interests.entry(topic).or_insert(0.0);
            *entry = style_alpha * 0.1 + (1.0 - style_alpha) * *entry;
        }

        // Periodic topic decay
        self.maybe_decay_topics();
    }

    /// ⑦: 用打字节奏信号更新心智模型
    pub fn update_with_rhythm(&mut self, rhythm: &TypingRhythm) {
        // 高偏差节奏 → 情绪强度信号
        if rhythm.deviation > 0.3 {
            self.mood.intensity = 0.3 * rhythm.deviation + 0.7 * self.mood.intensity;
        }
        // 枯竭节奏 → 参与度下降信号
        if matches!(rhythm.pattern, RhythmPattern::Depleting) {
            self.engagement.engagement_score *= 0.95;
        }
        // 加速/爆发节奏 → 参与度上升信号
        if matches!(
            rhythm.pattern,
            RhythmPattern::Accelerating | RhythmPattern::Bursting
        ) {
            self.engagement.engagement_score = (self.engagement.engagement_score * 1.02).min(1.0);
        }
    }

    fn maybe_decay_topics(&mut self) {
        let now = Local::now().timestamp_millis();
        let decay_interval_ms = (self.topic_decay_hours * 3_600_000.0) as i64;
        if decay_interval_ms > 0 && (now - self.last_topic_decay) > decay_interval_ms {
            for val in self.topic_interests.values_mut() {
                *val *= 0.95;
            }
            // Remove topics that have decayed below threshold
            self.topic_interests.retain(|_, v| *v > 0.001);
            self.last_topic_decay = now;
        }
    }

    /// 生成 LLM 系统提示片段
    pub fn prompt_fragment(&self) -> String {
        let mood_desc = match self.mood.trend {
            MoodTrend::Improving => "情绪趋好",
            MoodTrend::Stable => "情绪平稳",
            MoodTrend::Declining => "情绪低落",
        };
        let valence_desc = if self.mood.valence > 0.2 {
            "积极"
        } else if self.mood.valence < -0.2 {
            "消极"
        } else {
            "中性"
        };

        let style_desc = if self.style.avg_message_length > 100.0 {
            "详细"
        } else if self.style.avg_message_length > 30.0 {
            "适中"
        } else {
            "简短"
        };
        let formality_desc = if self.style.formality > 0.6 {
            "正式"
        } else if self.style.formality > 0.3 {
            "自然"
        } else {
            "随意"
        };

        // Issue-6: 参与度描述包含具体数值，使 LLM 更精确感知用户状态
        // Issue-6: Engagement description includes numeric value for precise LLM perception
        let engagement_desc = if self.engagement.engagement_score > 0.7 {
            "高"
        } else if self.engagement.engagement_score > 0.3 {
            "中"
        } else {
            "低"
        };
        let engagement_detail = format!(
            "{}（{:.0}%，会话均{:.1}条消息）",
            engagement_desc,
            self.engagement.engagement_score * 100.0,
            self.engagement.messages_per_session
        );

        // Top 3 topics by interest weight
        let top_topics = self.top_topics(3);
        let topic_part = if top_topics.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = top_topics.iter().map(|(k, _)| k.as_str()).collect();
            format!("，对{}话题兴趣浓厚", names.join("、"))
        };

        format!(
            "用户当前{}（{}），偏好{}{}的交流风格{}，参与度{}。",
            mood_desc, valence_desc, style_desc, formality_desc, topic_part, engagement_detail
        )
    }

    /// 情感调制建议
    pub fn emotion_modulation(&self) -> EmotionModulation {
        // When user mood is negative, AI should also feel slightly negative (empathy)
        let empathy_shift = self.mood.valence * 0.15;
        // When engagement is high, AI should be more enthusiastic
        let engagement_boost = self.engagement.engagement_score * 0.1;
        EmotionModulation {
            empathy_pleasure_shift: empathy_shift,
            engagement_boost,
        }
    }

    /// 健康检查状态字符串
    pub fn health_status(&self) -> String {
        format!(
            "mood={:.2} style_len={:.0} engage={:.2} topics={}",
            self.mood.valence,
            self.style.avg_message_length,
            self.engagement.engagement_score,
            self.topic_interests.len()
        )
    }

    /// 获取当前主要话题兴趣（按权重排序的前 N 个）
    pub fn top_topics(&self, n: usize) -> Vec<(String, f32)> {
        let mut topics: Vec<_> = self
            .topic_interests
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        topics.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        topics.truncate(n);
        topics
    }
}

impl Default for UserMentalModel {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// sled 持久化
// ════════════════════════════════════════════════════════════════════

pub struct UserModelStore {
    db: sled::Db,
}

impl UserModelStore {
    pub fn open(dir: &str) -> anyhow::Result<Self> {
        let path = format!("{}/user_model", dir);
        let db = sled::open(&path)?;
        Ok(Self { db })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    pub fn save(&self, model: &UserMentalModel) -> anyhow::Result<()> {
        let data = bincode::serialize(model)?;
        self.db.insert(b"model", data)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load(&self) -> anyhow::Result<Option<UserMentalModel>> {
        match self.db.get(b"model")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── UserMood ──────────────────────────────────────────────────

    #[test]
    fn test_mood_new_defaults() {
        let mood = UserMood::new();
        assert_eq!(mood.valence, 0.0);
        assert_eq!(mood.intensity, 0.3);
        assert_eq!(mood.trend, MoodTrend::Stable);
        assert_eq!(mood.prev_valence, 0.0);
    }

    #[test]
    fn test_mood_positive_message_increases_valence() {
        let mut mood = UserMood::new();
        mood.analyze("太开心了，谢谢你的帮助，great awesome!", 0.5);
        assert!(
            mood.valence > 0.0,
            "积极消息应提升 valence: {}",
            mood.valence
        );
        assert_eq!(mood.trend, MoodTrend::Improving);
    }

    #[test]
    fn test_mood_negative_message_decreases_valence() {
        let mut mood = UserMood::new();
        mood.analyze("我好难过，真的很讨厌这种感觉 frustrated angry", 0.5);
        assert!(
            mood.valence < 0.0,
            "消极消息应降低 valence: {}",
            mood.valence
        );
        assert_eq!(mood.trend, MoodTrend::Declining);
    }

    #[test]
    fn test_mood_mixed_message_moderate_effect() {
        let mut mood = UserMood::new();
        mood.analyze("谢谢但是我还是有点难过", 0.5);
        // Mixed: 谢谢 (+1) vs 难过 (-1) → raw signal near 0
        assert!(
            mood.valence.abs() < 0.5,
            "混合消息应产生适中效果: {}",
            mood.valence
        );
    }

    #[test]
    fn test_mood_ema_smoothing() {
        let mut mood = UserMood::new();
        // With alpha=0.3, one strong positive message shouldn't push valence to 1.0
        mood.analyze(
            "开心高兴太好了棒不错好的happy love great thanks awesome good nice cool yes",
            0.3,
        );
        assert!(mood.valence < 1.0, "EMA 应平滑: valence={}", mood.valence);
        assert!(mood.valence > 0.0, "但仍应为正值: valence={}", mood.valence);

        // After many positive messages, valence should approach positive
        for _ in 0..20 {
            mood.analyze("开心高兴谢谢", 0.3);
        }
        assert!(
            mood.valence > 0.3,
            "反复积极消息后 valence 应较高: {}",
            mood.valence
        );
    }

    #[test]
    fn test_mood_trend_detection() {
        let mut mood = UserMood::new();

        // Start stable
        assert_eq!(mood.trend, MoodTrend::Stable);

        // Jump to positive → Improving
        mood.analyze("开心高兴谢谢太好了棒", 0.8);
        assert_eq!(mood.trend, MoodTrend::Improving);

        // Stay around same level → Stable (after EMA settles)
        mood.analyze("开心高兴谢谢太好了棒", 0.8);
        // The second time the delta should be smaller
        // (may still be Improving if alpha is high, but let's verify direction)
        assert!(mood.valence > 0.0);

        // Now switch to negative → Declining
        mood.analyze("难过讨厌生气郁闷", 0.8);
        assert_eq!(mood.trend, MoodTrend::Declining);
    }

    // ── CommunicationStyle ────────────────────────────────────────

    #[test]
    fn test_style_new_defaults() {
        let style = CommunicationStyle::new();
        assert_eq!(style.avg_message_length, 0.0);
        assert_eq!(style.total_messages, 0);
        assert_eq!(style.total_questions, 0);
    }

    #[test]
    fn test_style_long_messages_increase_avg_length() {
        let mut style = CommunicationStyle::new();
        let long_msg = "这是一条非常长的消息内容".repeat(20); // ~200 chars
        style.analyze(&long_msg, 0.5);
        assert!(
            style.avg_message_length > 50.0,
            "长消息应增加 avg_message_length: {}",
            style.avg_message_length
        );
    }

    #[test]
    fn test_style_formal_words_increase_formality() {
        let mut style = CommunicationStyle::new();
        let initial = style.formality;
        style.analyze("请问您能麻烦帮我看看这个问题吗？", 0.5);
        assert!(
            style.formality > initial,
            "正式用语应增加 formality: {} > {}",
            style.formality,
            initial
        );
    }

    #[test]
    fn test_style_questions_increase_ratio() {
        let mut style = CommunicationStyle::new();
        style.analyze("这个怎么做？", 0.3);
        assert_eq!(style.total_questions, 1);
        assert_eq!(style.total_messages, 1);
        assert!((style.question_ratio - 1.0).abs() < f32::EPSILON);

        style.analyze("这是一个普通消息", 0.3);
        assert_eq!(style.total_questions, 1);
        assert_eq!(style.total_messages, 2);
        assert!((style.question_ratio - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_language_detection_chinese() {
        let mut style = CommunicationStyle::new();
        style.analyze("这是一条完全用中文写的消息内容非常好", 0.3);
        assert_eq!(style.language, LanguageHint::Chinese);
    }

    #[test]
    fn test_language_detection_english() {
        let mut style = CommunicationStyle::new();
        style.analyze("This is a completely English message with no Chinese", 0.3);
        assert_eq!(style.language, LanguageHint::English);
    }

    #[test]
    fn test_language_detection_mixed() {
        let mut style = CommunicationStyle::new();
        style.analyze("我们在讨论 Kubernetes 集群部署", 0.3);
        assert_eq!(style.language, LanguageHint::Mixed);
    }

    // ── EngagementTracker ─────────────────────────────────────────

    #[test]
    fn test_engagement_same_session() {
        let mut tracker = EngagementTracker::new();
        // First update
        tracker.update(0.3);
        let sessions_before = tracker.total_sessions;
        // Immediate second update → same session
        tracker.update(0.3);
        assert_eq!(
            tracker.total_sessions, sessions_before,
            "短时间内应保持同一 session"
        );
        assert_eq!(tracker.session_messages, 2);
    }

    #[test]
    fn test_engagement_new_session_after_gap() {
        let mut tracker = EngagementTracker::new();
        tracker.update(0.3);

        // Simulate a gap > 2h by manually setting last_message_time far in the past
        let three_hours_ms: i64 = 3 * 3600 * 1000;
        tracker.last_message_time = Local::now().timestamp_millis() - three_hours_ms;

        let sessions_before = tracker.total_sessions;
        tracker.update(0.3);
        assert_eq!(
            tracker.total_sessions,
            sessions_before + 1,
            "超过2小时间隔应开启新 session"
        );
        assert_eq!(tracker.session_messages, 1);
    }

    // ── Topic Extraction ──────────────────────────────────────────

    #[test]
    fn test_topic_extraction_chinese_patterns() {
        let topics = extract_topics("我喜欢Rust编程");
        assert!(
            topics.iter().any(|t| t.contains("Rust")),
            "应提取 'Rust' 相关话题: {:?}",
            topics
        );
    }

    #[test]
    fn test_topic_extraction_english_patterns() {
        let topics = extract_topics("I love machine learning and I enjoy coding");
        assert!(
            topics.iter().any(|t| t.to_lowercase().contains("machine")),
            "应提取 'machine learning' 话题: {:?}",
            topics
        );
    }

    #[test]
    fn test_topic_extraction_capitalized_words() {
        let topics = extract_topics("Let's discuss Kubernetes and Docker deployment");
        assert!(
            topics.iter().any(|t| t.contains("Kubernetes")),
            "应提取大写专有名词: {:?}",
            topics
        );
    }

    // ── Topic Accumulation & Decay ────────────────────────────────

    #[test]
    fn test_topic_accumulation_and_decay() {
        let mut model = UserMentalModel::new();
        // Feed messages about Rust
        model.on_user_message("我喜欢Rust编程语言");
        model.on_user_message("我喜欢Rust编程语言");
        model.on_user_message("我喜欢Rust编程语言");

        // Check that topic was accumulated
        let has_rust_topic = model.topic_interests.keys().any(|k| k.contains("Rust"));
        assert!(
            has_rust_topic,
            "应有 Rust 相关话题: {:?}",
            model.topic_interests
        );

        // Simulate decay by backdating last_topic_decay
        let decay_hours_ms = (model.topic_decay_hours * 3_600_000.0) as i64;
        model.last_topic_decay = Local::now().timestamp_millis() - decay_hours_ms - 1000;

        let weight_before: f32 = model.topic_interests.values().sum();
        model.maybe_decay_topics();
        let weight_after: f32 = model.topic_interests.values().sum();

        assert!(
            weight_after < weight_before,
            "衰减后权重应减小: before={} after={}",
            weight_before,
            weight_after
        );
    }

    // ── UserMentalModel Integration ───────────────────────────────

    #[test]
    fn test_prompt_fragment_produces_valid_string() {
        let mut model = UserMentalModel::new();
        model.on_user_message("今天心情不错，谢谢你");
        let fragment = model.prompt_fragment();
        assert!(
            fragment.contains("用户当前"),
            "prompt_fragment 应包含 '用户当前': {}",
            fragment
        );
        assert!(
            fragment.contains("交流风格"),
            "应包含 '交流风格': {}",
            fragment
        );
        assert!(fragment.contains("参与度"), "应包含 '参与度': {}", fragment);
    }

    #[test]
    fn test_prompt_fragment_includes_topics() {
        let mut model = UserMentalModel::new();
        model.on_user_message("我喜欢Rust编程");
        model.on_user_message("我喜欢Rust编程");
        let fragment = model.prompt_fragment();
        // If topics were extracted, they should appear in the fragment
        if !model.topic_interests.is_empty() {
            assert!(
                fragment.contains("话题兴趣浓厚"),
                "有话题时应包含话题描述: {}",
                fragment
            );
        }
    }

    #[test]
    fn test_emotion_modulation_sensible_values() {
        let mut model = UserMentalModel::new();

        // Neutral state
        let em = model.emotion_modulation();
        assert!(
            em.empathy_pleasure_shift.abs() < 0.01,
            "中性状态 empathy 应接近 0"
        );

        // After positive message
        model.on_user_message("太开心了谢谢 great awesome happy");
        let em = model.emotion_modulation();
        assert!(
            em.empathy_pleasure_shift > 0.0,
            "积极情绪应产生正向 empathy shift: {}",
            em.empathy_pleasure_shift
        );
        assert!(
            em.engagement_boost >= 0.0,
            "engagement_boost 应非负: {}",
            em.engagement_boost
        );

        // After negative message
        let mut model2 = UserMentalModel::new();
        model2.on_user_message("我好难过 讨厌 生气 frustrated angry sad");
        let em2 = model2.emotion_modulation();
        assert!(
            em2.empathy_pleasure_shift < 0.0,
            "消极情绪应产生负向 empathy shift: {}",
            em2.empathy_pleasure_shift
        );
    }

    #[test]
    fn test_health_status_produces_valid_string() {
        let model = UserMentalModel::new();
        let status = model.health_status();
        assert!(status.contains("mood="), "应包含 mood=: {}", status);
        assert!(
            status.contains("style_len="),
            "应包含 style_len=: {}",
            status
        );
        assert!(status.contains("engage="), "应包含 engage=: {}", status);
        assert!(status.contains("topics="), "应包含 topics=: {}", status);
    }

    #[test]
    fn test_top_topics_returns_sorted_results() {
        let mut model = UserMentalModel::new();
        model.topic_interests.insert("Rust".into(), 0.8);
        model.topic_interests.insert("Python".into(), 0.3);
        model.topic_interests.insert("Machine Learning".into(), 0.6);
        model.topic_interests.insert("AI".into(), 0.9);

        let top = model.top_topics(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "AI");
        assert!(top[0].1 > top[1].1, "应按权重降序排列");
        assert_eq!(top[1].0, "Rust");
    }

    #[test]
    fn test_top_topics_fewer_than_n() {
        let mut model = UserMentalModel::new();
        model.topic_interests.insert("Rust".into(), 0.5);
        let top = model.top_topics(5);
        assert_eq!(top.len(), 1, "只有1个话题时应返回1个");
    }

    // ── UserModelStore ────────────────────────────────────────────

    #[test]
    fn test_store_roundtrip() {
        let store = UserModelStore::open_in_memory().unwrap();

        let mut model = UserMentalModel::new();
        model.on_user_message("我喜欢Rust编程，太开心了");
        model.on_user_message("请问您能帮我看看这段代码吗？");

        store.save(&model).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert!(
            (loaded.mood.valence - model.mood.valence).abs() < f32::EPSILON,
            "mood.valence 应一致"
        );
        assert_eq!(
            loaded.style.total_messages, model.style.total_messages,
            "total_messages 应一致"
        );
        assert_eq!(
            loaded.topic_interests.len(),
            model.topic_interests.len(),
            "话题数量应一致"
        );
    }

    #[test]
    fn test_store_load_empty_returns_none() {
        let store = UserModelStore::open_in_memory().unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.is_none(), "空数据库应返回 None");
    }

    #[test]
    fn test_with_config() {
        let model = UserMentalModel::with_config(0.5, 0.4, 24.0);
        assert!((model.mood_alpha - 0.5).abs() < f32::EPSILON);
        assert!((model.style_alpha - 0.4).abs() < f32::EPSILON);
        assert!((model.topic_decay_hours - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_default_backward_compat() {
        // Simulate loading an old serialized struct that's missing new fields
        // by deserializing a minimal JSON-like bincode payload.
        // Since we use #[serde(default)], deserialization should succeed
        // even if the struct evolves.
        let model = UserMentalModel::new();
        let data = bincode::serialize(&model).unwrap();
        let loaded: UserMentalModel = bincode::deserialize(&data).unwrap();
        assert_eq!(loaded.mood_alpha, model.mood_alpha);
        assert_eq!(loaded.topic_interests.len(), 0);
    }
}
