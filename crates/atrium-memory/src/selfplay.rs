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

            let thought = Thought {
                summary: pattern.summary.clone(),
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
    pub fn select(&self, thoughts: &[&Thought]) -> Option<String> {
        let shareable: Vec<&&Thought> = thoughts.iter().filter(|t| t.shareable).collect();
        if shareable.is_empty() {
            return None;
        }

        // 选置信度最高且不在冷却中的
        for t in shareable {
            if !self.cooldown_topics.contains(&t.summary) {
                return Some(format!("💭 我刚刚想到: {}", t.summary));
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
        let selector = GroupTopicSelector::new();
        let topic = selector.select(&[&thoughts[0]]);
        assert!(topic.is_some());
        assert!(topic.unwrap().contains("有趣发现"));
    }
}
