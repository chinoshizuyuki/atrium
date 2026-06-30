// SPDX-License-Identifier: MIT
//! IntelligenceExtractor — 统一 LLM 提取管线
//! IntelligenceExtractor — Unified LLM extraction pipeline.
//!
//! 将用户对话批量发送给 LLM，以 JSON 结构化输出同时提取：
//! Sends user conversation to LLM, requesting structured JSON output:
//! - 用户偏好 / User preferences (like/dislike/habit/interest)
//! - 行为规则 / Behavioral rules (user-described triggers + actions)
//!
//! 本模块只负责 prompt 构建 + JSON 解析，实际 LLM 调用由 CoreService 异步完成。
//! This module handles prompt assembly + JSON parsing only; actual LLM calls
//! are asynchronously orchestrated by CoreService.

use crate::history::ChatMessage;
use crate::preference::PreferenceLayer;
use crate::rules::{BehaviorRule, RuleAction, TriggerCondition};
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 提取结果类型
// ════════════════════════════════════════════════════════════════════

/// LLM 提取的用户偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPreference {
    /// 偏好类别（如 "lang", "food", "hobby", "schedule"）
    pub key: String,
    /// 偏好值（如 "Rust", "火锅", "早起"）
    pub value: String,
    /// 情感倾向: "like" | "dislike" | "neutral"
    pub sentiment: String,
    /// 可选上下文（从哪句话提取的）
    #[serde(default)]
    pub context: String,
}

/// LLM 提取的自然语言规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRule {
    /// 规则名称（如 "加班提醒"）
    pub name: String,
    /// 触发类型: "keyword" | "time_range" | "idle"
    pub trigger_type: String,
    /// 关键词列表（trigger_type = "keyword" 时使用）
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 开始时间（trigger_type = "time_range" 时使用，格式 HH:MM）
    #[serde(default)]
    pub time_start: String,
    /// 结束时间（trigger_type = "time_range" 时使用，格式 HH:MM）
    #[serde(default)]
    pub time_end: String,
    /// 空闲秒数（trigger_type = "idle" 时使用）
    #[serde(default)]
    pub idle_seconds: u64,
    /// 提醒消息
    pub reminder: String,
}

/// LLM 一次调用的完整提取结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    #[serde(default)]
    pub preferences: Vec<ExtractedPreference>,
    #[serde(default)]
    pub rules: Vec<ExtractedRule>,
}

// ════════════════════════════════════════════════════════════════════
// Prompt 构建
// ════════════════════════════════════════════════════════════════════

const SYSTEM_PROMPT: &str = r#"你是一个对话分析引擎。分析用户和AI之间的对话，提取结构化信息。

你必须返回一个合法的 JSON 对象，格式如下：
{
 "preferences": [
 {"key": "类别", "value": "值", "sentiment": "like|dislike|neutral", "context": "原文摘录"}
 ],
 "rules": [
 {
 "name": "规则名称",
 "trigger_type": "keyword|time_range|idle",
 "keywords": ["关键词1", "关键词2"],
 "time_start": "HH:MM",
 "time_end": "HH:MM",
 "idle_seconds": 3600,
 "reminder": "提醒消息"
 }
 ]
}

提取规则：
- preferences: 用户表达的喜好、厌恶、习惯、兴趣。key 应简短（如 lang, food, hobby, schedule, music）。sentiment 只能是 like/dislike/neutral。
- rules: 用户用自然语言要求的行为规则。例如"以后提到加班的时候提醒我休息"→ keyword 触发；"每天晚上11点提醒我睡觉"→ time_range 触发。
- 只提取对话中明确存在的信息。不要猜测或编造。
- 如果没有可提取的内容，返回 {"preferences": [], "rules": []}。"#;

/// 从对话历史构建提取 prompt
pub fn build_extraction_prompt(messages: &[ChatMessage]) -> String {
    let mut prompt = String::with_capacity(messages.len() * 80);
    prompt.push_str("以下是最近的对话记录：\n\n");

    for msg in messages {
        let role_label = if msg.role == "user" { "用户" } else { "AI" };
        let content = &msg.content;
        // 截断过长消息
        let truncated = if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.clone()
        };
        prompt.push_str(&format!("【{}】: {}\n", role_label, truncated));
    }

    prompt.push_str("\n请分析以上对话，提取用户偏好和行为规则。返回 JSON。");
    prompt
}

/// 获取 system prompt（供外部调用 LLM 时传入）
pub fn system_prompt() -> &'static str {
    SYSTEM_PROMPT
}

// ════════════════════════════════════════════════════════════════════
// JSON 解析
// ════════════════════════════════════════════════════════════════════

/// 解析 LLM 返回的 JSON 文本为 ExtractionResult
///
/// 容错策略：
/// - 如果整体 JSON 解析失败 → 返回空结果（不崩溃）
/// - 如果某个 preference/rule 缺少必要字段 → 跳过该项
pub fn parse_extraction_response(json_text: &str) -> ExtractionResult {
    // 尝试直接解析
    if let Ok(result) = serde_json::from_str::<ExtractionResult>(json_text) {
        return result;
    }

    // 尝试从 markdown code block 中提取 JSON
    let cleaned = extract_json_from_markdown(json_text);
    if let Ok(result) = serde_json::from_str::<ExtractionResult>(&cleaned) {
        return result;
    }

    // 尝试提取第一个 { ... } 块
    if let Some(json_block) = extract_json_object(json_text) {
        if let Ok(result) = serde_json::from_str::<ExtractionResult>(&json_block) {
            return result;
        }
    }

    tracing::warn!(
        "IntelligenceExtractor: JSON 解析失败, 原文前 200 字符: {}",
        &json_text[..json_text.len().min(200)]
    );
    ExtractionResult::default()
}

/// 从 markdown ```json ... ``` 中提取
fn extract_json_from_markdown(text: &str) -> String {
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    text.to_string()
}

/// 提取第一个完整的 { ... } JSON 对象
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for (i, ch) in text[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(text[start..start + i + 1].to_string());
            }
        }
    }
    None
}

// ════════════════════════════════════════════════════════════════════
// 结果转换（→ PreferenceManager / RuleEngine 可直接消费的格式）
// ════════════════════════════════════════════════════════════════════

impl ExtractedPreference {
    /// 转换为 (key, value, PreferenceLayer) 三元组，供 PreferenceManager::upsert 使用
    pub fn to_preference_tuple(&self) -> (&str, &str, PreferenceLayer) {
        (&self.key, &self.value, PreferenceLayer::LLMExtraction)
    }
}

impl ExtractedRule {
    /// 转换为 BehaviorRule，供 RuleEngine::add 使用
    pub fn to_behavior_rule(&self) -> BehaviorRule {
        let condition = match self.trigger_type.as_str() {
            "time_range" => TriggerCondition::TimeRange {
                start: self.time_start.clone(),
                end: self.time_end.clone(),
            },
            "idle" => TriggerCondition::Idle {
                seconds: if self.idle_seconds > 0 {
                    self.idle_seconds
                } else {
                    3600
                },
            },
            // 默认: keyword
            _ => TriggerCondition::Keyword {
                words: if self.keywords.is_empty() {
                    vec![self.name.clone()]
                } else {
                    self.keywords.clone()
                },
            },
        };

        BehaviorRule::new(
            self.name.clone(),
            10, // 用户创建的规则优先级低于内置规则（1-3）
            600,
            condition,
            RuleAction::Notify {
                message: self.reminder.clone(),
            },
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::ChatMessage;

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.into(),
            content: content.into(),
            timestamp_ms: 0,
            emotion: None,
        }
    }

    #[test]
    fn test_build_extraction_prompt() {
        let msgs = vec![
            make_msg("user", "我喜欢用 Rust 编程"),
            make_msg("assistant", "Rust 是一门优秀的语言"),
        ];
        let prompt = build_extraction_prompt(&msgs);
        assert!(prompt.contains("用户"));
        assert!(prompt.contains("AI"));
        assert!(prompt.contains("Rust"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_parse_valid_json() {
        let json = r#"{
 "preferences": [
 {"key": "lang", "value": "Rust", "sentiment": "like", "context": "我喜欢Rust"}
 ],
 "rules": [
 {
 "name": "加班提醒",
 "trigger_type": "keyword",
 "keywords": ["加班", "熬夜"],
 "reminder": "该休息了"
 }
 ]
 }"#;

        let result = parse_extraction_response(json);
        assert_eq!(result.preferences.len(), 1);
        assert_eq!(result.preferences[0].key, "lang");
        assert_eq!(result.preferences[0].sentiment, "like");
        assert_eq!(result.rules.len(), 1);
        assert_eq!(result.rules[0].name, "加班提醒");
        assert_eq!(result.rules[0].keywords, vec!["加班", "熬夜"]);
    }

    #[test]
    fn test_parse_empty_response() {
        let json = r#"{"preferences": [], "rules": []}"#;
        let result = parse_extraction_response(json);
        assert!(result.preferences.is_empty());
        assert!(result.rules.is_empty());
    }

    #[test]
    fn test_parse_markdown_wrapped_json() {
        let text = r#"Here is the result:
```json
{"preferences": [{"key": "food", "value": "火锅", "sentiment": "like"}], "rules": []}
```"#;
        let result = parse_extraction_response(text);
        assert_eq!(result.preferences.len(), 1);
        assert_eq!(result.preferences[0].value, "火锅");
    }

    #[test]
    fn test_parse_garbage_returns_empty() {
        let result = parse_extraction_response("I'm sorry, I cannot help with that.");
        assert!(result.preferences.is_empty());
        assert!(result.rules.is_empty());
    }

    #[test]
    fn test_extracted_rule_to_behavior_rule_keyword() {
        let rule = ExtractedRule {
            name: "加班提醒".into(),
            trigger_type: "keyword".into(),
            keywords: vec!["加班".into(), "熬夜".into()],
            time_start: String::new(),
            time_end: String::new(),
            idle_seconds: 0,
            reminder: "该休息了哦".into(),
        };

        let br = rule.to_behavior_rule();
        assert_eq!(br.name, "加班提醒");
        assert!(br.enabled);
        assert!(matches!(br.condition, TriggerCondition::Keyword { .. }));
        assert!(matches!(br.action, RuleAction::Notify { .. }));
    }

    #[test]
    fn test_extracted_rule_to_behavior_rule_time_range() {
        let rule = ExtractedRule {
            name: "睡觉提醒".into(),
            trigger_type: "time_range".into(),
            keywords: vec![],
            time_start: "23:00".into(),
            time_end: "06:00".into(),
            idle_seconds: 0,
            reminder: "该睡觉了".into(),
        };

        let br = rule.to_behavior_rule();
        assert!(matches!(br.condition, TriggerCondition::TimeRange { .. }));
        if let TriggerCondition::TimeRange { start, end } = &br.condition {
            assert_eq!(start, "23:00");
            assert_eq!(end, "06:00");
        }
    }

    #[test]
    fn test_extracted_preference_to_tuple() {
        let pref = ExtractedPreference {
            key: "food".into(),
            value: "火锅".into(),
            sentiment: "like".into(),
            context: "我最爱吃火锅".into(),
        };
        let (k, v, layer) = pref.to_preference_tuple();
        assert_eq!(k, "food");
        assert_eq!(v, "火锅");
        assert!(matches!(layer, PreferenceLayer::LLMExtraction));
    }
}
