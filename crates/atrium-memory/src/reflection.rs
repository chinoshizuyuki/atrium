// SPDX-License-Identifier: MIT
//! Reflection 高阶洞察 — sled 持久化，重启不丢
//! ReflectionEngine — Higher-order reflection + sled persistence.
use crate::fact_store::Fact;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 洞察状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightStatus {
    Pending,    // 刚生成，待收集更多证据
    Validated,  // 有足够证据支持
    Promoted,   // 晋升为稳定洞察
    Deprecated, // 被新证据推翻
}

/// 高阶洞察
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub summary: String,
    pub supporting_facts: Vec<String>,
    pub confidence: f64,
    pub status: InsightStatus,
    pub min_evidence: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Insight {
    pub fn new(summary: &str, facts: Vec<String>, confidence: f64) -> Self {
        let now = now_secs();
        let mut s = Self {
            summary: summary.to_string(),
            supporting_facts: facts,
            confidence,
            status: InsightStatus::Pending,
            min_evidence: 2,
            created_at: now,
            updated_at: now,
        };
        s.evaluate();
        s
    }

    pub fn add_evidence(&mut self, fact_key: &str, fact_confidence: f64) {
        if self.supporting_facts.contains(&fact_key.to_string()) {
            return;
        }
        self.supporting_facts.push(fact_key.to_string());
        self.updated_at = now_secs();
        let total = self.supporting_facts.len() as f64;
        self.confidence = (self.confidence * (total - 1.0) + fact_confidence) / total;
        self.evaluate();
    }

    fn evaluate(&mut self) {
        let count = self.supporting_facts.len() as u32;
        if count >= self.min_evidence * 2 {
            self.status = InsightStatus::Promoted;
        } else if count >= self.min_evidence {
            self.status = InsightStatus::Validated;
        }
    }

    pub fn is_deprecated(&self) -> bool {
        self.confidence < 0.3
    }
}

pub struct ReflectionEngine {
    insights: Vec<Insight>,
    db: Option<sled::Db>,
}

impl Default for ReflectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ReflectionEngine {
    pub fn new() -> Self {
        Self {
            insights: Vec::new(),
            db: None,
        }
    }

    /// 打开持久化存储，自动恢复已有洞察
    pub fn open(db_path: &str) -> Self {
        let db = sled::open(db_path).ok();
        let mut insights = Vec::new();
        if let Some(ref db) = db {
            for item in db.iter().flatten() {
                let (_, value) = item;
                if let Ok(insight) = bincode::deserialize::<Insight>(&value) {
                    insights.push(insight);
                }
            }
            tracing::info!(
                "ReflectionEngine: loaded {} insights from sled",
                insights.len()
            );
        }
        Self { insights, db }
    }

    pub fn new_in_memory() -> Self {
        Self {
            insights: Vec::new(),
            db: None,
        }
    }

    fn persist_all(&self) {
        if let Some(ref db) = self.db {
            // 清空旧数据
            for key in db.iter().keys().flatten() {
                let _ = db.remove(key);
            }
            // 写入全部
            for (i, insight) in self.insights.iter().enumerate() {
                if let Ok(data) = bincode::serialize(insight) {
                    let _ = db.insert(format!("insight_{}", i).as_bytes(), data);
                }
            }
            let _ = db.flush();
        }
    }

    pub fn reflect(&mut self, facts: &[Fact]) -> Vec<&Insight> {
        let patterns = self.discover_patterns(facts);
        for (summary, keys, conf) in patterns {
            self.add_or_update_insight(&summary, keys, conf);
        }
        self.persist_all();
        self.insights
            .iter()
            .filter(|i| i.status != InsightStatus::Deprecated)
            .collect()
    }

    fn discover_patterns(&self, facts: &[Fact]) -> Vec<(String, Vec<String>, f64)> {
        let mut by_subject: HashMap<&str, Vec<&Fact>> = HashMap::new();
        for fact in facts {
            by_subject
                .entry(fact.subject.as_str())
                .or_default()
                .push(fact);
        }

        let mut patterns = Vec::new();
        for (subject, group) in &by_subject {
            if group.len() >= 2 {
                let avg = group.iter().map(|f| f.confidence).sum::<f64>() / group.len() as f64;
                patterns.push((
                    format!("{}有{}个相关事实", subject, group.len()),
                    group.iter().map(|f| f.canonical_form()).collect(),
                    avg,
                ));
            }

            let likes = group.iter().filter(|f| f.predicate == "喜欢").count();
            if likes >= 2 {
                let targets: Vec<&str> = group
                    .iter()
                    .filter(|f| f.predicate == "喜欢")
                    .map(|f| f.object.as_str())
                    .collect();
                let avg = group
                    .iter()
                    .filter(|f| f.predicate == "喜欢")
                    .map(|f| f.confidence)
                    .sum::<f64>()
                    / likes as f64;
                patterns.push((
                    format!("{}偏好{}", subject, targets.join("、")),
                    group
                        .iter()
                        .filter(|f| f.predicate == "喜欢")
                        .map(|f| f.canonical_form())
                        .collect(),
                    avg,
                ));
            }
        }
        patterns
    }

    pub fn add_or_update_insight(&mut self, summary: &str, keys: Vec<String>, conf: f64) {
        if let Some(existing) = self.insights.iter_mut().find(|i| i.summary == summary) {
            for k in &keys {
                existing.add_evidence(k, conf);
            }
        } else {
            self.insights.push(Insight::new(summary, keys, conf));
        }
    }

    pub fn all_insights(&self) -> &[Insight] {
        &self.insights
    }
    pub fn promoted_insights(&self) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| i.status == InsightStatus::Promoted)
            .collect()
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_preference() {
        let mut engine = ReflectionEngine::new();
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "编程").with_confidence(0.8),
        ];
        let active = engine.reflect(&facts);
        assert!(active.iter().any(|i| i.summary.contains("偏好")));
    }

    #[test]
    fn test_insight_state_transition() {
        let mut engine = ReflectionEngine::new();
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "AI").with_confidence(0.8),
            Fact::new("主人", "喜欢", "编程").with_confidence(0.85),
            Fact::new("主人", "喜欢", "游戏").with_confidence(0.7),
        ];
        engine.reflect(&facts);
        let promoted = engine.promoted_insights();
        assert!(!promoted.is_empty());
    }

    #[test]
    fn test_deprecated() {
        let ins = Insight::new("test", vec!["f1".to_string()], 0.2);
        assert!(ins.is_deprecated());
    }
}
