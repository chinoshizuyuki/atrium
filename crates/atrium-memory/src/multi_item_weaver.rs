// SPDX-License-Identifier: MIT

//! 多事项编织器 — Multi-item weaver: combines multiple follow-up items into
//! a single natural-language prompt fragment.
//!
//! 核心理念：人类不会一次只问一件事——"考试怎么样了？上次你还担心面试来着"
//! 是两个事项的自然编织。

use crate::followup_tracker::{FollowUpDepth, FollowUpItem, FollowUpStyle, TriggerVerdict};

// ═══════════════════════════════════════════════════════════════════════════
// 配置 — Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// 多事项编织器配置 / Multi-item weaver configuration.
#[derive(Debug, Clone)]
pub struct WeaverConfig {
    /// 每条消息最大事项数 / Max items per message.
    pub max_items_per_message: usize,
}

impl Default for WeaverConfig {
    fn default() -> Self {
        Self {
            max_items_per_message: 2,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 多事项编织器 — Multi-Item Weaver
// ═══════════════════════════════════════════════════════════════════════════

/// 多事项编织器 — 将多个触发事项编织为一条自然的追问提示
/// Multi-item weaver — Weaves multiple triggered items into one natural prompt.
#[derive(Debug, Clone)]
pub struct MultiItemWeaver {
    /// 配置 / Configuration.
    pub config: WeaverConfig,
}

impl MultiItemWeaver {
    /// 创建默认配置的编织器 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(WeaverConfig::default())
    }

    /// 创建指定配置的编织器 / Create with custom config.
    pub fn new(config: WeaverConfig) -> Self {
        Self { config }
    }

    /// 编织多个事项为一条追问提示
    /// Weave multiple items into a single follow-up prompt.
    pub fn weave(&self, items: &[(FollowUpItem, TriggerVerdict)]) -> String {
        if items.is_empty() {
            return String::new();
        }

        // 按权重（urgency）降序排列 / Sort by urgency descending
        let mut sorted: Vec<&(FollowUpItem, TriggerVerdict)> = items.iter().collect();
        sorted.sort_by(|a, b| {
            b.1.urgency
                .partial_cmp(&a.1.urgency)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 截断到最大数量 / Truncate to max items
        let max = self.config.max_items_per_message.min(sorted.len());
        let selected = &sorted[..max];

        if selected.len() == 1 {
            // 单事项 — 直接生成 / Single item — direct generation
            return self.weave_single(&selected[0].0, &selected[0].1);
        }

        // 多事项 — 分组并编织 / Multiple items — group and weave
        let groups = self.group_related(selected);
        self.merge_groups(&groups)
    }

    /// 编织单个事项 / Weave a single item.
    fn weave_single(&self, item: &FollowUpItem, verdict: &TriggerVerdict) -> String {
        let desc = &item.description;
        match (verdict.suggested_depth, verdict.suggested_style) {
            (FollowUpDepth::Surface, FollowUpStyle::Direct) => {
                format!(
                    "用户之前提到过「{}」，如果话题自然，可以顺便问一下进展。",
                    desc
                )
            }
            (FollowUpDepth::Surface, FollowUpStyle::Indirect) => {
                format!("用户曾提到「{}」，可以在相关话题出现时自然提及。", desc)
            }
            (FollowUpDepth::Surface, FollowUpStyle::Caring) => {
                format!("用户曾提到「{}」，可以适时关心。", desc)
            }
            (FollowUpDepth::Surface, FollowUpStyle::Companionate) => {
                format!(
                    "用户之前提到「{}」但回避了追问。不要直接问，用陪伴方式。",
                    desc
                )
            }
            (FollowUpDepth::Moderate, FollowUpStyle::Direct) => {
                format!("用户之前提到「{}」，可以直接问问进展。", desc)
            }
            (FollowUpDepth::Moderate, FollowUpStyle::Indirect) => {
                format!(
                    "用户之前提到「{}」。不要直接追问，在相关话题出现时自然接。",
                    desc
                )
            }
            (FollowUpDepth::Moderate, FollowUpStyle::Caring) => {
                format!(
                    "用户之前提到「{}」，看起来比较在意。可以轻声关心一下。",
                    desc
                )
            }
            (FollowUpDepth::Moderate, FollowUpStyle::Companionate) => {
                format!(
                    "用户之前提到「{}」但回避了追问。不要直接问，用陪伴方式。",
                    desc
                )
            }
            (FollowUpDepth::Deep, FollowUpStyle::Direct) => {
                format!("用户多次提及「{}」，可以直接询问最新进展。", desc)
            }
            (FollowUpDepth::Deep, FollowUpStyle::Indirect) => {
                format!("用户曾深度提及「{}」，在合适时机自然深入聊聊。", desc)
            }
            (FollowUpDepth::Deep, FollowUpStyle::Caring) => {
                format!(
                    "用户曾深度提及「{}」，在合适时机表达你一直在意这件事。",
                    desc
                )
            }
            (FollowUpDepth::Deep, FollowUpStyle::Companionate) => {
                format!(
                    "用户提到「{}」时回避了追问。用陪伴方式，让用户知道你一直在。",
                    desc
                )
            }
        }
    }

    /// 按类别分组相关事项 / Group related items by category.
    fn group_related<'a>(
        &self,
        items: &[&'a (FollowUpItem, TriggerVerdict)],
    ) -> Vec<Vec<&'a (FollowUpItem, TriggerVerdict)>> {
        let mut groups: Vec<Vec<&(FollowUpItem, TriggerVerdict)>> = Vec::new();
        for item in items {
            let cat = item.0.category;
            let mut found = false;
            for group in &mut groups {
                if group[0].0.category == cat {
                    group.push(*item);
                    found = true;
                    break;
                }
            }
            if !found {
                groups.push(vec![*item]);
            }
        }
        groups
    }

    /// 合并分组为一条消息 / Merge groups into a single message.
    fn merge_groups(&self, groups: &[Vec<&(FollowUpItem, TriggerVerdict)>]) -> String {
        let mut parts = Vec::new();
        for (i, group) in groups.iter().enumerate() {
            if i == 0 {
                parts.push(self.weave_group(group));
            } else {
                // 用"对了"连接不同类别 / Connect different categories with "对了"
                parts.push(format!("对了，{}", self.weave_group(group)));
            }
        }
        parts.join("")
    }

    /// 编织单组（同类别）事项 / Weave a single group (same category).
    fn weave_group(&self, group: &[&(FollowUpItem, TriggerVerdict)]) -> String {
        if group.len() == 1 {
            return self.weave_single(&group[0].0, &group[0].1);
        }

        // 同类别多事项 — 合并描述 / Same category multi-item — merge descriptions
        let descs: Vec<&str> = group.iter().map(|g| g.0.description.as_str()).collect();
        let merged = descs.join("和");
        let verdict = &group[0].1;

        match verdict.suggested_style {
            FollowUpStyle::Caring => {
                format!("用户之前提到「{}」，可以关心一下进展。", merged)
            }
            FollowUpStyle::Direct => {
                format!("用户之前提到的「{}」都怎么样了？", merged)
            }
            FollowUpStyle::Indirect => {
                format!("用户曾提到「{}」，可以在合适时机自然提及。", merged)
            }
            FollowUpStyle::Companionate => {
                format!("用户之前提到「{}」但回避了追问。用陪伴方式。", merged)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::followup_tracker::{ContextSnapshot, FollowUpCategory, FollowUpStatus};

    fn mk_item(desc: &str, cat: FollowUpCategory) -> FollowUpItem {
        FollowUpItem {
            id: 1,
            description: desc.into(),
            category: cat,
            first_mentioned_at: 0,
            last_mentioned_at: 0,
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
            emotional_weight: 0.5,
            relationship_depth_at_creation: 0.5,
            decay_rate: 72.0,
        }
    }

    fn mk_verdict(urgency: f32) -> TriggerVerdict {
        TriggerVerdict {
            should_trigger: true,
            trigger_reason: crate::followup_tracker::TriggerReason::LongSilence,
            urgency,
            suggested_depth: FollowUpDepth::Moderate,
            suggested_style: FollowUpStyle::Direct,
        }
    }

    #[test]
    fn test_empty_items() {
        let w = MultiItemWeaver::default_new();
        assert!(w.weave(&[]).is_empty());
    }

    #[test]
    fn test_single_item() {
        let w = MultiItemWeaver::default_new();
        let item = mk_item("考研", FollowUpCategory::Plan);
        let verdict = mk_verdict(0.8);
        let s = w.weave(&[(item, verdict)]);
        assert!(s.contains("考研"));
    }

    #[test]
    fn test_same_category_merged() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("考研", FollowUpCategory::Work), mk_verdict(0.8)),
            (mk_item("面试", FollowUpCategory::Work), mk_verdict(0.7)),
        ];
        let s = w.weave(&items);
        assert!(s.contains("考研"), "should contain 考研: {}", s);
        assert!(s.contains("面试"), "should contain 面试: {}", s);
    }

    #[test]
    fn test_different_categories_connected() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("考试", FollowUpCategory::Work), mk_verdict(0.8)),
            (mk_item("失眠", FollowUpCategory::Health), mk_verdict(0.7)),
        ];
        let s = w.weave(&items);
        assert!(s.contains("对了"), "should connect with 对了: {}", s);
    }

    #[test]
    fn test_exceeds_max_truncates() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("a", FollowUpCategory::Plan), mk_verdict(0.9)),
            (mk_item("b", FollowUpCategory::Worry), mk_verdict(0.8)),
            (mk_item("c", FollowUpCategory::Health), mk_verdict(0.7)),
        ];
        let s = w.weave(&items);
        assert!(s.contains("a"), "should contain top-1: {}", s);
        assert!(s.contains("b"), "should contain top-2: {}", s);
        assert!(!s.contains("c"), "should not contain top-3: {}", s);
    }

    #[test]
    fn test_sorted_by_urgency() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("low", FollowUpCategory::Plan), mk_verdict(0.3)),
            (mk_item("high", FollowUpCategory::Worry), mk_verdict(0.9)),
        ];
        let s = w.weave(&items);
        // high urgency should appear first
        let high_pos = s.find("high").unwrap_or(usize::MAX);
        let low_pos = s.find("low").unwrap_or(usize::MAX);
        assert!(high_pos < low_pos, "high urgency should come first: {}", s);
    }

    #[test]
    fn test_depth_style_passed() {
        let w = MultiItemWeaver::default_new();
        let item = mk_item("test", FollowUpCategory::Worry);
        let verdict = TriggerVerdict {
            should_trigger: true,
            trigger_reason: crate::followup_tracker::TriggerReason::LongSilence,
            urgency: 0.8,
            suggested_depth: FollowUpDepth::Deep,
            suggested_style: FollowUpStyle::Caring,
        };
        let s = w.weave(&[(item, verdict)]);
        assert!(
            s.contains("一直在意"),
            "Deep+Caring should contain 一直在意: {}",
            s
        );
    }

    #[test]
    fn test_three_items_top_two() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("first", FollowUpCategory::Plan), mk_verdict(0.9)),
            (mk_item("second", FollowUpCategory::Worry), mk_verdict(0.8)),
            (mk_item("third", FollowUpCategory::Health), mk_verdict(0.7)),
        ];
        let s = w.weave(&items);
        assert!(s.contains("first") && s.contains("second"));
        assert!(!s.contains("third"));
    }

    #[test]
    fn test_merged_descriptions() {
        let w = MultiItemWeaver::default_new();
        let items = vec![
            (mk_item("考试", FollowUpCategory::Work), mk_verdict(0.8)),
            (mk_item("答辩", FollowUpCategory::Work), mk_verdict(0.7)),
        ];
        let s = w.weave(&items);
        assert!(s.contains("考试和答辩"), "should merge: {}", s);
    }

    #[test]
    fn test_contains_original_description() {
        let w = MultiItemWeaver::default_new();
        let item = mk_item("考研准备", FollowUpCategory::Plan);
        let verdict = mk_verdict(0.8);
        let s = w.weave(&[(item, verdict)]);
        assert!(s.contains("考研准备"));
    }
}
