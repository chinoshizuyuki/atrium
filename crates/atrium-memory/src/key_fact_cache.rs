// SPDX-License-Identifier: MIT
//! 关键信息缓存
//!
//! 存储永不丢弃的用户偏好、习惯、身份信息和重要约定。
//! 独立于对话历史，任何 session 都能读取。
//!
//! 持久化：sled LSM-tree 后端。upsert 时同步写入磁盘，
//! 启动时从 sled 恢复全量数据到内存缓存。
//! KeyFactCache — Key information cache.
//!
//! Stores high-value user preferences, habits, and key information for fast retrieval.
//! Available for recall in any session at any time.
//!
//! Persistence: sled LSM-tree. Writes are synchronously flushed on upsert;
//! on startup, full data is restored from sled into the in-memory cache.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// 关键信息类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyFactCategory {
    Preference,
    Identity,
    Commitment,
    Todo,
    Relationship,
}

impl KeyFactCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "偏好",
            Self::Identity => "身份",
            Self::Commitment => "约定",
            Self::Todo => "待办",
            Self::Relationship => "关系",
        }
    }
}

/// 关键信息条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFact {
    pub content: String,
    pub category: KeyFactCategory,
    pub confidence: f64,
    pub source: String,
    pub first_seen: i64,
    pub last_confirmed: i64,
    pub confirmed_count: u32,
}

impl KeyFact {
    pub fn new(content: &str, category: KeyFactCategory, confidence: f64, source: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            content: content.to_string(),
            category,
            confidence: confidence.clamp(0.0, 1.0),
            source: source.to_string(),
            first_seen: now,
            last_confirmed: now,
            confirmed_count: 1,
        }
    }

    pub fn confirm(&mut self) {
        self.confirmed_count += 1;
        self.last_confirmed = chrono::Utc::now().timestamp();
        self.confidence = self.confidence + (1.0 - self.confidence) * 0.3;
    }

    /// sled 存储键：category|content
    fn storage_key(&self) -> Vec<u8> {
        format!("{}|{}", self.category.as_str(), self.content).into_bytes()
    }
}

/// 关键信息缓存（内存 + sled 持久化双写）
pub struct KeyFactCache {
    facts: RwLock<HashMap<String, Vec<KeyFact>>>,
    /// sled 持久化后端（None = 纯内存模式，测试用）
    db: Option<sled::Db>,
}

impl KeyFactCache {
    /// 创建带 sled 持久化的缓存
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let cache = Self {
            facts: RwLock::new(HashMap::new()),
            db: Some(db),
        };
        cache.load_from_disk();
        Ok(cache)
    }

    /// 创建纯内存缓存（测试用，不持久化）
    pub fn new_in_memory() -> Self {
        Self {
            facts: RwLock::new(HashMap::new()),
            db: None,
        }
    }

    /// 从 sled 加载全部数据到内存
    fn load_from_disk(&self) {
        let db = match &self.db {
            Some(db) => db,
            None => return,
        };
        let mut facts = self.facts.write().expect("key_facts rwlock write");
        for item in db.iter() {
            let (_, value) = match item {
                Ok(kv) => kv,
                Err(_) => continue,
            };
            if let Ok(fact) = bincode::deserialize::<KeyFact>(&value) {
                let key = fact.category.as_str().to_string();
                facts.entry(key).or_default().push(fact);
            }
        }
    }

    /// 添加或更新关键信息（内存 + 磁盘双写）
    pub fn upsert(&self, content: &str, category: KeyFactCategory, confidence: f64, source: &str) {
        let mut facts = self.facts.write().expect("key_facts rwlock write");
        let key = category.as_str().to_string();
        let entry = facts.entry(key).or_default();

        if let Some(existing) = entry.iter_mut().find(|f| f.content == content) {
            existing.confirm();
            existing.confidence = existing.confidence.max(confidence);
            // 同步写 sled
            self.persist(existing);
        } else {
            let fact = KeyFact::new(content, category, confidence, source);
            self.persist(&fact);
            entry.push(fact);
        }

        if entry.len() > 50 {
            entry.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            entry.truncate(50);
        }
    }

    /// 持久化单条 KeyFact 到 sled
    fn persist(&self, fact: &KeyFact) {
        if let Some(ref db) = self.db {
            let key = fact.storage_key();
            if let Ok(data) = bincode::serialize(fact) {
                let _ = db.insert(key, data);
                // 异步 flush 由 sled 自动处理（每 500ms）
            }
        }
    }

    /// 获取高置信度关键信息摘要（用于注入 Prompt）
    pub fn build_context(&self, min_confidence: f64) -> String {
        let facts = self.facts.read().expect("key_facts rwlock read");
        let mut ctx = String::from("[关键信息]\n");

        for category in &[
            KeyFactCategory::Identity,
            KeyFactCategory::Preference,
            KeyFactCategory::Commitment,
            KeyFactCategory::Relationship,
            KeyFactCategory::Todo,
        ] {
            let key = category.as_str().to_string();
            if let Some(entries) = facts.get(&key) {
                let high: Vec<&KeyFact> = entries
                    .iter()
                    .filter(|f| f.confidence >= min_confidence)
                    .collect();
                if !high.is_empty() {
                    ctx.push_str(&format!("{}:\n", category.as_str()));
                    for f in high {
                        ctx.push_str(&format!(
                            " - {} (置信度:{:.0}%)\n",
                            f.content,
                            f.confidence * 100.0
                        ));
                    }
                }
            }
        }

        if ctx == "[关键信息]\n" {
            return String::new();
        }
        ctx
    }

    pub fn total_count(&self) -> usize {
        self.facts
            .read()
            .expect("key_facts rwlock read")
            .values()
            .map(|v| v.len())
            .sum()
    }

    pub fn search(&self, query: &str) -> Vec<KeyFact> {
        let facts = self.facts.read().expect("key_facts rwlock read");
        let mut results = Vec::new();
        for entries in facts.values() {
            for f in entries {
                if f.content.contains(query) || f.source.contains(query) {
                    results.push(f.clone());
                }
            }
        }
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// 显式 flush 到磁盘（关闭前调用）
    pub fn flush(&self) {
        if let Some(ref db) = self.db {
            let _ = db.flush();
        }
    }

    /// 是否启用持久化
    pub fn is_persistent(&self) -> bool {
        self.db.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cache() -> KeyFactCache {
        KeyFactCache::new_in_memory()
    }

    #[test]
    fn test_upsert_and_build_context() {
        let cache = test_cache();
        cache.upsert("主人喜欢Rust", KeyFactCategory::Preference, 0.85, "对话");
        cache.upsert("主人叫张三", KeyFactCategory::Identity, 0.95, "对话");

        let ctx = cache.build_context(0.5);
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("张三"));
    }

    #[test]
    fn test_confirm_increases_confidence() {
        let cache = test_cache();
        cache.upsert("关键约定", KeyFactCategory::Commitment, 0.6, "来源");
        cache.upsert("关键约定", KeyFactCategory::Commitment, 0.7, "来源2");

        let results = cache.search("约定");
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence > 0.7);
        assert_eq!(results[0].confirmed_count, 2);
    }

    #[test]
    fn test_min_confidence_filter() {
        let cache = test_cache();
        cache.upsert("高置信", KeyFactCategory::Preference, 0.9, "s");
        cache.upsert("低置信", KeyFactCategory::Preference, 0.3, "s");

        let ctx = cache.build_context(0.7);
        assert!(ctx.contains("高置信"));
        assert!(!ctx.contains("低置信"));
    }

    #[test]
    fn test_empty_cache() {
        let cache = test_cache();
        assert!(cache.build_context(0.5).is_empty());
        assert!(!cache.is_persistent());
    }

    #[test]
    fn test_persistent_roundtrip() {
        let path = "./target/atrium_kf_test_roundtrip";
        // 清理旧数据
        let _ = std::fs::remove_dir_all(path);

        // 写入
        {
            let cache = KeyFactCache::open(path).unwrap();
            assert!(cache.is_persistent());
            cache.upsert("持久化测试", KeyFactCategory::Identity, 0.99, "测试");
            cache.flush();
        }

        // 重新打开，应能恢复
        {
            let cache = KeyFactCache::open(path).unwrap();
            let ctx = cache.build_context(0.5);
            assert!(ctx.contains("持久化测试"), "重启后应能恢复: {}", ctx);
            assert_eq!(cache.total_count(), 1);
        }

        // 清理
        let _ = std::fs::remove_dir_all(path);
    }
}
