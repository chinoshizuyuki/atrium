// SPDX-License-Identifier: MIT
//! Self-Play Pipeline — AI 独处时的自主思考
//! Self-Play Pipeline — AI autonomous thinking during idle time.
//!
//! 后台常驻线程: CronScheduler → ThoughtFactory → GroupTopicSelector
//! AI 在空闲时持续产生新洞察，不再只是被动响应。
//! Background daemon: CronScheduler + ThoughtFactory + GroupTopicSelector.
//! AI generates spontaneous thoughts when idle, not just reactive responses.

use crate::replay::DiscoveredPattern;
use std::collections::VecDeque;

/// 思考类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThoughtType {
    /// 悖论：发现矛盾的事实
    Paradox,
    /// 类比：发现相似模式
    Analogy,
    /// 推理：从现有事实推导新结论
    Deduction,
    /// 模式：发现重复出现的行为
    Pattern,
    /// 分享：AI 主动想说的话
    Sharing,
}

/// AI 自主思考产出
#[derive(Debug, Clone)]
pub struct Thought {
    pub summary: String,
    pub thought_type: ThoughtType,
    pub confidence: f64,
    pub generated_at: i64,
    /// 是否适合群聊分享
    pub shareable: bool,
}

/// 思考工厂
pub struct ThoughtFactory {
    thoughts: VecDeque<Thought>,
    max_thoughts: usize,
    /// 上次思考时间
    last_thought: i64,
    /// 思考间隔（秒）
    interval_secs: i64,
}

impl Default for ThoughtFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ThoughtFactory {
    pub fn new() -> Self {
        Self {
            thoughts: VecDeque::new(),
            max_thoughts: 100,
            last_thought: 0,
            interval_secs: 60,
        }
    }

    /// 从回放管道发现的模式中产生思考
    pub fn produce(&mut self, patterns: &[DiscoveredPattern]) -> Vec<&Thought> {
        let now = chrono::Utc::now().timestamp();
        if now - self.last_thought < self.interval_secs {
            return self.recent(5);
        }
        self.last_thought = now;

        for pattern in patterns {
            if pattern.confidence < 0.5 {
                continue;
            }

            let thought_type = match pattern.kind {
                crate::replay::PatternKind::FrequentFact => ThoughtType::Pattern,
                crate::replay::PatternKind::EntityCluster => ThoughtType::Analogy,
                crate::replay::PatternKind::ConfidenceTrend => ThoughtType::Deduction,
                crate::replay::PatternKind::TemporalCluster => ThoughtType::Pattern,
            };

            let shareable = pattern.confidence > 0.7;

            // 按 ThoughtType 添加叙事性前缀 — 让洞察更像"思考过程"而非"事实重述"（G-09 修复）
            // Add narrative prefix by ThoughtType — make insights feel like
            // "thinking" rather than "fact restating" (G-09 fix).
            let narrative_summary = match thought_type {
                ThoughtType::Pattern => format!("我注意到一个规律：{}", pattern.summary),
                ThoughtType::Analogy => format!("我发现了一些相似之处：{}", pattern.summary),
                ThoughtType::Deduction => format!("我推导出一个结论：{}", pattern.summary),
                _ => pattern.summary.clone(),
            };

            let thought = Thought {
                summary: narrative_summary,
                thought_type,
                confidence: pattern.confidence,
                generated_at: now,
                shareable,
            };

            if self.thoughts.len() >= self.max_thoughts {
                self.thoughts.pop_front();
            }
            self.thoughts.push_back(thought);
        }

        self.recent(5)
    }

    /// 获取最近的思考
    pub fn recent(&self, n: usize) -> Vec<&Thought> {
        self.thoughts.iter().rev().take(n).collect()
    }

    /// 获取可分享的思考（适合群聊/直播）
    pub fn shareable(&self) -> Vec<&Thought> {
        self.thoughts.iter().filter(|t| t.shareable).collect()
    }

    pub fn count(&self) -> usize {
        self.thoughts.len()
    }
}

/// 群聊话题选择器
pub struct GroupTopicSelector {
    cooldown_topics: Vec<String>,
}

impl Default for GroupTopicSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupTopicSelector {
    pub fn new() -> Self {
        Self {
            cooldown_topics: Vec::new(),
        }
    }

    /// 从思考中选出最有群聊价值的话题
    /// Select the most group-chat-worthy topic from thoughts.
    ///
    /// 选中后会写入 `cooldown_topics` 防止同一洞察反复注入（G-09 修复）。
    /// Selected summaries are pushed into `cooldown_topics` to prevent
    /// repeat injection of the same insight (G-09 fix).
    pub fn select(&mut self, thoughts: &[&Thought]) -> Option<String> {
        let shareable: Vec<&&Thought> = thoughts.iter().filter(|t| t.shareable).collect();
        if shareable.is_empty() {
            return None;
        }

        // 选置信度最高且不在冷却中的 / Select highest confidence not in cooldown
        for t in shareable {
            if !self.cooldown_topics.contains(&t.summary) {
                // 选中后加入冷却列表，避免反复注入 / Add to cooldown after selection to prevent repeat
                self.cooldown_topics.push(t.summary.clone());
                // 冷却列表上限 10 条，FIFO 淘汰 / Cooldown cap 10, FIFO eviction
                if self.cooldown_topics.len() > 10 {
                    self.cooldown_topics.remove(0);
                }
                // 叙事前缀已在 produce() 中写入，这里直接返回 summary
                // Narrative prefix is already written in produce(); return summary directly
                return Some(t.summary.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{DiscoveredPattern, PatternKind};

    #[test]
    fn test_produce_thoughts() {
        let mut factory = ThoughtFactory::new();
        let patterns = vec![DiscoveredPattern {
            summary: "主人喜欢深夜写代码".into(),
            kind: PatternKind::FrequentFact,
            confidence: 0.85,
            discovered_at: 0,
        }];
        let thoughts = factory.produce(&patterns);
        assert!(!thoughts.is_empty());
        assert!(thoughts[0].shareable);
    }

    #[test]
    fn test_topic_selector() {
        let thoughts = [Thought {
            summary: "有趣发现".into(),
            thought_type: ThoughtType::Pattern,
            confidence: 0.9,
            generated_at: 0,
            shareable: true,
        }];
        let mut selector = GroupTopicSelector::new();
        let topic = selector.select(&[&thoughts[0]]);
        assert!(topic.is_some());
        let topic = topic.unwrap();
        assert!(topic.contains("有趣发现"));
        // select() 不再添加 💭 前缀 / select() no longer prepends 💭 prefix
        assert!(!topic.contains("💭"));
    }

    /// 验证 cooldown 防重复：同一 thought 第二次 select() 应返回 None
    /// Verify cooldown prevents repeat: second select() on the same thought returns None.
    #[test]
    fn test_cooldown_prevents_repeat_selection() {
        let thoughts = [Thought {
            summary: "深夜写代码".into(),
            thought_type: ThoughtType::Pattern,
            confidence: 0.9,
            generated_at: 0,
            shareable: true,
        }];
        let mut selector = GroupTopicSelector::new();

        // 第一次选中 / First selection succeeds
        let first = selector.select(&[&thoughts[0]]);
        assert!(first.is_some());

        // 第二次因冷却返回 None / Second returns None due to cooldown
        let second = selector.select(&[&thoughts[0]]);
        assert!(second.is_none());
    }

    /// 验证 FIFO 淘汰：11 条不同 thought，第 11 次选第 1 条（已淘汰出冷却）
    /// Verify FIFO eviction: 11 distinct thoughts; 11th pick re-selects the 1st
    /// (which has been evicted from cooldown).
    #[test]
    fn test_cooldown_fifo_eviction() {
        let thoughts: Vec<Thought> = (0..11)
            .map(|i| Thought {
                summary: format!("topic-{i}"),
                thought_type: ThoughtType::Pattern,
                confidence: 0.5 + i as f64 * 0.01,
                generated_at: 0,
                shareable: true,
            })
            .collect();

        let mut selector = GroupTopicSelector::new();
        // 依次选 10 条，全部进入冷却 / Pick 10 in sequence; all enter cooldown
        for t in &thoughts[0..10] {
            let picked = selector.select(&[t]);
            assert!(picked.is_some());
        }
        // 此时冷却列表已有 10 条（topic-0..topic-9），刚好达上限
        // Cooldown now holds 10 entries (topic-0..topic-9), at the cap.

        // 第 11 次只能选 topic-10 / 11th pick can only be topic-10
        let eleventh = selector.select(&[&thoughts[10]]);
        assert!(eleventh.is_some());
        assert_eq!(eleventh.unwrap(), "topic-10");

        // 此时 topic-0 已 FIFO 淘汰出冷却，可被再次选中
        // topic-0 has been FIFO-evicted from cooldown and can be re-selected.
        let replay = selector.select(&[&thoughts[0]]);
        assert!(replay.is_some());
        assert_eq!(replay.unwrap(), "topic-0");
    }

    /// 验证 produce() 按 ThoughtType 添加叙事性前缀
    /// Verify produce() adds narrative prefix by ThoughtType.
    #[test]
    fn test_produce_adds_narrative_prefix_by_type() {
        let mut factory = ThoughtFactory::new();
        let patterns = vec![
            DiscoveredPattern {
                summary: "深夜编码".into(),
                kind: PatternKind::FrequentFact,
                confidence: 0.85,
                discovered_at: 0,
            },
            DiscoveredPattern {
                summary: "实体聚集".into(),
                kind: PatternKind::EntityCluster,
                confidence: 0.85,
                discovered_at: 0,
            },
            DiscoveredPattern {
                summary: "置信度上升".into(),
                kind: PatternKind::ConfidenceTrend,
                confidence: 0.85,
                discovered_at: 0,
            },
        ];
        let thoughts = factory.produce(&patterns);
        assert_eq!(thoughts.len(), 3);

        // recent() 返回逆序（最新在前）/ recent() returns reverse order (newest first)
        // patterns 入队顺序: FrequentFact, EntityCluster, ConfidenceTrend
        // thoughts 顺序: ConfidenceTrend(Deduction), EntityCluster(Analogy), FrequentFact(Pattern)
        // ConfidenceTrend → Deduction → "我推导出一个结论："
        assert!(
            thoughts[0].summary.starts_with("我推导出一个结论："),
            "Deduction summary missing prefix: {}",
            thoughts[0].summary
        );
        // EntityCluster → Analogy → "我发现了一些相似之处："
        assert!(
            thoughts[1].summary.starts_with("我发现了一些相似之处："),
            "Analogy summary missing prefix: {}",
            thoughts[1].summary
        );
        // FrequentFact → Pattern → "我注意到一个规律："
        assert!(
            thoughts[2].summary.starts_with("我注意到一个规律："),
            "Pattern summary missing prefix: {}",
            thoughts[2].summary
        );
    }

    /// 验证 select() 直接返回叙事性 summary，不再带 "💭" 前缀
    /// Verify select() returns narrative summary directly, without "💭" prefix.
    #[test]
    fn test_select_returns_narrative_summary() {
        let mut factory = ThoughtFactory::new();
        let patterns = vec![DiscoveredPattern {
            summary: "深夜写代码".into(),
            kind: PatternKind::FrequentFact,
            confidence: 0.85,
            discovered_at: 0,
        }];
        let thoughts = factory.produce(&patterns);
        assert!(!thoughts.is_empty());

        let mut selector = GroupTopicSelector::new();
        let topic = selector.select(&thoughts);
        assert!(topic.is_some());
        let topic = topic.unwrap();
        // 不应包含 💭 前缀 / Should not contain 💭 prefix
        assert!(
            !topic.contains("💭"),
            "topic should not contain 💭: {topic}"
        );
        // 应直接是叙事性 summary（含 Pattern 前缀）
        // Should be narrative summary directly (with Pattern prefix)
        assert!(
            topic.starts_with("我注意到一个规律："),
            "topic should start with narrative prefix: {topic}"
        );
    }
}
