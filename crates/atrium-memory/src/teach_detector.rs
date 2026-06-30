// SPDX-License-Identifier: MIT
//! 教学意图检测器 — 纯规则，<1μs
//!
//! ACK 自学习 Path A：检测用户是否在"教"AI 知识。
//! 三组模式：显式记忆指令 / 知识传授 / 规则设定。
//! TeachDetector — Teaching intent detector, latency <1ms.
//!
//! ACK self-learning Path A: detects whether the user is "teaching" the AI new knowledge.
//! Pattern matching: explicit instructions / knowledge statements / rule definitions.

use serde::{Deserialize, Serialize};

/// 教学模式分组
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachPatternGroup {
    /// 显式记忆指令: "记住，..." / "以后记得..."
    ExplicitRemember,
    /// 知识传授: "我教你..." / "告诉你一个..."
    KnowledgeTeaching,
    /// 规则设定: "以后遇到X就Y" / "每次...的时候..."
    RuleSetting,
}

/// 教学意图检测结果
#[derive(Debug, Clone)]
pub struct TeachIntent {
    /// 置信度 0.0~1.0
    pub confidence: f64,
    /// 匹配的模式分组
    pub pattern_group: TeachPatternGroup,
    /// 提取的教学内容（去除前缀后的文本）
    pub knowledge_text: String,
}

/// 前缀匹配规则
struct PrefixRule {
    prefix: &'static str,
    confidence: f64,
    group: TeachPatternGroup,
}

/// 所有前缀规则（编译时常量）
static PREFIX_RULES: &[PrefixRule] = &[
    // ── 组 1: 显式记忆指令 (conf 0.85~0.95) ──
    PrefixRule {
        prefix: "记住，",
        confidence: 0.95,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "记住:",
        confidence: 0.95,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "记住 ",
        confidence: 0.92,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "以后记得",
        confidence: 0.90,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "帮我记一下",
        confidence: 0.88,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "帮我记住",
        confidence: 0.88,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "请记住",
        confidence: 0.87,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "你要记住",
        confidence: 0.86,
        group: TeachPatternGroup::ExplicitRemember,
    },
    PrefixRule {
        prefix: "记下来",
        confidence: 0.85,
        group: TeachPatternGroup::ExplicitRemember,
    },
    // ── 组 2: 知识传授 (conf 0.75~0.85) ──
    PrefixRule {
        prefix: "我教你",
        confidence: 0.85,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    PrefixRule {
        prefix: "告诉你一个",
        confidence: 0.82,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    PrefixRule {
        prefix: "科普一下",
        confidence: 0.80,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    PrefixRule {
        prefix: "你知道吗，",
        confidence: 0.78,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    PrefixRule {
        prefix: "有个技巧",
        confidence: 0.76,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    PrefixRule {
        prefix: "教你一招",
        confidence: 0.75,
        group: TeachPatternGroup::KnowledgeTeaching,
    },
    // ── 组 3: 规则设定 (conf 0.75~0.80) ──
    PrefixRule {
        prefix: "以后遇到",
        confidence: 0.80,
        group: TeachPatternGroup::RuleSetting,
    },
    PrefixRule {
        prefix: "每次",
        confidence: 0.78,
        group: TeachPatternGroup::RuleSetting,
    },
    PrefixRule {
        prefix: "以后",
        confidence: 0.76,
        group: TeachPatternGroup::RuleSetting,
    },
    PrefixRule {
        prefix: "从今天起",
        confidence: 0.77,
        group: TeachPatternGroup::RuleSetting,
    },
    PrefixRule {
        prefix: "从现在开始",
        confidence: 0.78,
        group: TeachPatternGroup::RuleSetting,
    },
];

/// 检测消息中是否包含教学意图
///
/// 遍历所有前缀规则，返回置信度最高的匹配结果。
/// 纯规则匹配，延迟 <1μs。
pub fn detect_teach_intent(message: &str) -> Option<TeachIntent> {
    let msg = message.trim();
    if msg.is_empty() {
        return None;
    }

    let mut best: Option<(&PrefixRule, usize)> = None;

    for rule in PREFIX_RULES {
        if msg.starts_with(rule.prefix) {
            let prefix_len = rule.prefix.len();
            // 选择: 置信度更高，或置信度相同但前缀更长（更精确）
            let is_better = match best {
                None => true,
                Some((prev_rule, _)) => {
                    rule.confidence > prev_rule.confidence
                        || (rule.confidence == prev_rule.confidence
                            && prefix_len > prev_rule.prefix.len())
                }
            };
            if is_better {
                best = Some((rule, prefix_len));
            }
        }
    }

    best.map(|(rule, prefix_len)| {
        let raw = &msg[prefix_len..];
        let knowledge_text = raw
            .trim_start_matches(['，', '。', ':', '：', ' ', '\t'])
            .trim()
            .to_string();
        TeachIntent {
            confidence: rule.confidence,
            pattern_group: rule.group,
            knowledge_text,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_explicit_remember() {
        let result = detect_teach_intent("记住，我喜欢画画").unwrap();
        assert_eq!(result.pattern_group, TeachPatternGroup::ExplicitRemember);
        assert!(result.confidence >= 0.90);
        assert_eq!(result.knowledge_text, "我喜欢画画");
    }

    #[test]
    fn test_detect_knowledge_teaching() {
        let result = detect_teach_intent("告诉你一个技巧，用tab补全").unwrap();
        assert_eq!(result.pattern_group, TeachPatternGroup::KnowledgeTeaching);
        assert!(result.confidence >= 0.75);
        assert!(result.knowledge_text.contains("用tab补全"));
    }

    #[test]
    fn test_detect_rule_setting() {
        let result = detect_teach_intent("以后遇到这种问题就换个思路").unwrap();
        assert_eq!(result.pattern_group, TeachPatternGroup::RuleSetting);
        assert!(result.confidence >= 0.75);
    }

    #[test]
    fn test_no_teaching_intent() {
        assert!(detect_teach_intent("今天天气不错").is_none());
        assert!(detect_teach_intent("你好啊").is_none());
        assert!(detect_teach_intent("").is_none());
    }

    #[test]
    fn test_knowledge_text_extraction() {
        let result = detect_teach_intent("帮我记一下，明天下午3点开会").unwrap();
        assert_eq!(result.knowledge_text, "明天下午3点开会");

        let result = detect_teach_intent("记住:Rust的所有权模型").unwrap();
        assert_eq!(result.knowledge_text, "Rust的所有权模型");
    }

    #[test]
    fn test_longer_prefix_wins() {
        // "帮我记住" should match over "记住" due to higher specificity
        let result = detect_teach_intent("帮我记住这个API地址").unwrap();
        assert_eq!(result.pattern_group, TeachPatternGroup::ExplicitRemember);
        assert_eq!(result.knowledge_text, "这个API地址");
    }
}
