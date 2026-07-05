// SPDX-License-Identifier: MIT
//! FactStore 存储和查询接口
//! 使用 sled 做持久化，重启后自动恢复
//! FactStore — Fact storage and query interface.
//! Uses sled for persistence, auto-recovers on restart.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ════════════════════════════════════════════════════════════════════
// EmotionContext — 事实的情感上下文标注
// ════════════════════════════════════════════════════════════════════

/// 事实创建时的情感上下文快照
///
/// 记录 AI 当时的情绪状态和用户的情绪倾向，
/// 让 AI 能"回忆"情感时刻——"上次我这么难过的时候，主人跟我说了什么？"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionContext {
    /// AI 当前情感标签（9 种基本情绪之一：愉悦/兴奋/放松/悲伤/愤怒/恐惧/惊讶/厌恶/平静）
    pub ai_emotion_label: String,
    /// AI PAD 值 [pleasure, arousal, dominance]
    pub ai_pad: [f32; 3],
    /// 情感强度 (0.0~1.0)，越高表示情绪越强烈
    pub intensity: f32,
    /// 用户情绪倾向 (valence: -1.0~1.0)，正值积极、负值消极
    pub user_mood: Option<f32>,
    /// 创建时间（Unix 秒）
    pub timestamp: u64,
}

// ════════════════════════════════════════════════════════════════════
// Fact — 事实结构体
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64, // 置信度 0.0 - 1.0
    pub source: String,
    pub created_at: u64,
    pub verified_at: u64,
    pub verify_count: u32,
    /// 事实创建时的情感上下文
    #[serde(default)]
    pub emotion_context: Option<EmotionContext>,
}

impl Fact {
    pub fn new<S: Into<String>>(subject: S, predicate: S, object: S) -> Self {
        let now = now_secs();
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence: 1.0,
            source: String::new(),
            created_at: now,
            verified_at: now,
            verify_count: 1,
            emotion_context: None,
        }
    }

    pub fn with_confidence(mut self, c: f64) -> Self {
        // 防止 NaN 进入系统：NaN 降级为 0.0
        self.confidence = if c.is_nan() { 0.0 } else { c.clamp(0.0, 1.0) };
        self
    }

    pub fn with_source<S: Into<String>>(mut self, s: S) -> Self {
        self.source = s.into();
        self
    }

    /// 附加情感上下文
    pub fn with_emotion(mut self, ctx: EmotionContext) -> Self {
        self.emotion_context = Some(ctx);
        self
    }

    /// 唯一键（去重）
    pub fn canonical_form(&self) -> String {
        format!(
            "{} | {} | {}",
            self.subject.to_lowercase().trim(),
            self.predicate.to_lowercase().trim(),
            self.object.to_lowercase().trim()
        )
    }

    /// 合并置信度（加权平均）
    pub fn merge_confidence(&mut self, new: f64) {
        // 防止 NaN 污染
        let new = if new.is_nan() {
            0.0
        } else {
            new.clamp(0.0, 1.0)
        };
        let w = 1.0 / (self.verify_count as f64 + 1.0);
        self.confidence = self.confidence * (1.0 - w) + new * w;
        self.verify_count += 1;
        self.verified_at = now_secs();
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// 统一使用 store_core::StoreError / Unified StoreError from store_core
pub type Result<T> = std::result::Result<T, crate::store_core::StoreError>;

pub struct FactStore {
    inner: Mutex<HashMap<String, Fact>>,
    db: Option<sled::Db>,
}

impl FactStore {
    pub fn new(db_path: &str) -> Result<Self> {
        if db_path.is_empty() {
            return Ok(Self {
                inner: Mutex::new(HashMap::new()),
                db: None,
            });
        }
        let db = sled::open(db_path)
            .map_err(|e| crate::store_core::StoreError::Sled(format!("sled open: {}", e)))?;
        // 从 sled 恢复所有事实到内存
        let mut map = HashMap::new();
        for item in db.iter() {
            let (key, value) =
                item.map_err(|e| crate::store_core::StoreError::Sled(format!("sled iter: {}", e)))?;
            if key.as_ref().starts_with(b"fact:") {
                let fact: Fact = bincode::deserialize(&value)
                    .map_err(|e| crate::store_core::StoreError::Codec(format!("bincode: {}", e)))?;
                let canonical = fact.canonical_form();
                map.insert(canonical, fact);
            }
        }
        tracing::info!("FactStore: loaded {} facts from sled", map.len());
        Ok(Self {
            inner: Mutex::new(map),
            db: Some(db),
        })
    }

    pub fn new_in_memory() -> Result<Self> {
        let db = sled::Config::default()
            .temporary(true)
            .open()
            .map_err(|e| crate::store_core::StoreError::Sled(format!("sled open: {}", e)))?;
        Ok(Self {
            inner: Mutex::new(HashMap::new()),
            db: Some(db),
        })
    }

    /// 插入事实, 返回 true = 新插入, false = 已存在并合并置信度
    pub fn insert(&self, fact: Fact) -> Result<bool> {
        let key = fact.canonical_form();
        let mut map = self.inner.lock().expect("fact_store init");
        if let Some(existing) = map.get_mut(&key) {
            existing.merge_confidence(fact.confidence);
            existing.source = format!("{}, {}", existing.source, fact.source);
            // 持久化更新
            self.persist_one(&key, existing);
            Ok(false)
        } else {
            self.persist_one(&key, &fact);
            map.insert(key, fact);
            Ok(true)
        }
    }

    fn persist_one(&self, key: &str, fact: &Fact) {
        if let Some(ref db) = self.db {
            let db_key = format!("fact:{}", key);
            if let Ok(data) = bincode::serialize(fact) {
                let _ = db.insert(db_key.as_bytes(), data);
            }
        }
    }

    /// 批量插入
    pub fn insert_batch(&self, facts: Vec<Fact>) -> Result<(usize, usize)> {
        let (mut new, mut merged) = (0, 0);
        for f in facts {
            if self.insert(f)? {
                new += 1;
            } else {
                merged += 1;
            }
        }
        Ok((new, merged))
    }

    /// 关键词查询（匹配 subject / predicate / object）
    pub fn query(&self, keyword: &str) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let kw = keyword.to_lowercase();
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| {
                f.subject.to_lowercase().contains(&kw)
                    || f.predicate.to_lowercase().contains(&kw)
                    || f.object.to_lowercase().contains(&kw)
            })
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results.into_iter().cloned().collect())
    }

    /// 查询某个主体的所有事实
    pub fn query_by_subject(&self, subject: &str) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let subj = subject.to_lowercase();
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| f.subject.to_lowercase() == subj)
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results.into_iter().cloned().collect())
    }

    /// 按情感标签查询事实
    ///
    /// "上次我这么难过的时候，主人跟我说了什么？"
    /// 按情感强度降序排列，最强烈的情感记忆排在前面。
    pub fn query_by_emotion(&self, label: &str) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| {
                f.emotion_context
                    .as_ref()
                    .map(|ctx| ctx.ai_emotion_label == label)
                    .unwrap_or(false)
            })
            .collect();
        // 按情感强度降序 → 再按时间降序
        results.sort_by(|a, b| {
            let a_intensity = a
                .emotion_context
                .as_ref()
                .map(|c| c.intensity)
                .unwrap_or(0.0);
            let b_intensity = b
                .emotion_context
                .as_ref()
                .map(|c| c.intensity)
                .unwrap_or(0.0);
            b_intensity
                .partial_cmp(&a_intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.created_at
                        .partial_cmp(&a.created_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        Ok(results.into_iter().cloned().collect())
    }

    /// 关键词查询 + 可选情感过滤
    ///
    /// 在现有 query() 基础上，可按情感标签缩小范围。
    pub fn query_with_emotion_filter(
        &self,
        keyword: &str,
        emotion_filter: Option<&str>,
    ) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let kw = keyword.to_lowercase();
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| {
                let text_match = f.subject.to_lowercase().contains(&kw)
                    || f.predicate.to_lowercase().contains(&kw)
                    || f.object.to_lowercase().contains(&kw);
                if !text_match {
                    return false;
                }
                if let Some(label) = emotion_filter {
                    f.emotion_context
                        .as_ref()
                        .map(|ctx| ctx.ai_emotion_label == label)
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results.into_iter().cloned().collect())
    }

    pub fn count(&self) -> usize {
        self.inner.lock().expect("fact_store init").len()
    }

    /// 获取所有事实的快照（供巩固模块分析）
    pub fn all_facts(&self) -> Vec<Fact> {
        self.inner
            .lock()
            .expect("fact_store init")
            .values()
            .cloned()
            .collect()
    }

    /// 按 canonical form 删除事实（供巩固模块清理）
    pub fn remove(&self, canonical: &str) -> bool {
        let mut map = self.inner.lock().expect("fact_store init");
        if map.remove(canonical).is_some() {
            if let Some(ref db) = self.db {
                let db_key = format!("fact:{}", canonical);
                let _ = db.remove(db_key.as_bytes());
            }
            true
        } else {
            false
        }
    }
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    fn new_store() -> FactStore {
        FactStore::new_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_query() {
        let s = new_store();
        s.insert(Fact::new("主人", "喜欢", "编程").with_source("对话"))
            .unwrap();
        let r = s.query("编程").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].subject, "主人");
    }

    #[test]
    fn test_dedup_merges() {
        let s = new_store();
        assert!(s
            .insert(Fact::new("主人", "喜欢", "Rust").with_confidence(0.9))
            .unwrap());
        assert!(!s
            .insert(Fact::new("主人", "喜欢", "Rust").with_confidence(0.5))
            .unwrap());
        let r = s.query("Rust").unwrap();
        assert_eq!(r[0].verify_count, 2);
        assert!((r[0].confidence - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_batch() {
        let s = new_store();
        let (n, m) = s
            .insert_batch(vec![
                Fact::new("A", "is", "B"),
                Fact::new("A", "is", "C"),
                Fact::new("A", "is", "B"),
            ])
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(m, 1);
        assert_eq!(s.count(), 2);
    }

    fn make_emotion_ctx(label: &str, intensity: f32) -> EmotionContext {
        EmotionContext {
            ai_emotion_label: label.to_string(),
            ai_pad: [0.5, 0.3, 0.1],
            intensity,
            user_mood: Some(0.4),
            timestamp: now_secs(),
        }
    }

    #[test]
    fn test_emotion_context_attached() {
        let s = new_store();
        let ctx = make_emotion_ctx("愉悦", 0.6);
        s.insert(Fact::new("主人", "分享", "好消息").with_emotion(ctx.clone()))
            .unwrap();

        let r = s.query("好消息").unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].emotion_context.is_some());
        let ec = r[0].emotion_context.as_ref().unwrap();
        assert_eq!(ec.ai_emotion_label, "愉悦");
        assert!((ec.intensity - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_query_by_emotion() {
        let s = new_store();
        s.insert(Fact::new("主人", "说", "今天好开心").with_emotion(make_emotion_ctx("愉悦", 0.8)))
            .unwrap();
        s.insert(
            Fact::new("主人", "说", "工作压力好大").with_emotion(make_emotion_ctx("悲伤", 0.6)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "说", "明天要去旅行").with_emotion(make_emotion_ctx("兴奋", 0.9)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "说", "周末打算休息").with_emotion(make_emotion_ctx("平静", 0.2)),
        )
        .unwrap();

        // 查询「愉悦」标签 → 只匹配第 1 条
        let happy = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(happy.len(), 1);
        assert_eq!(happy[0].object, "今天好开心");

        // 查询「悲伤」标签 → 只匹配第 2 条
        let sad = s.query_by_emotion("悲伤").unwrap();
        assert_eq!(sad.len(), 1);
        assert_eq!(sad[0].object, "工作压力好大");

        // 查询不存在的标签 → 空
        let none = s.query_by_emotion("恐惧").unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn test_query_by_emotion_sorted_by_intensity() {
        let s = new_store();
        s.insert(Fact::new("主人", "说", "一般般开心").with_emotion(make_emotion_ctx("愉悦", 0.4)))
            .unwrap();
        s.insert(Fact::new("主人", "说", "超级开心").with_emotion(make_emotion_ctx("愉悦", 0.95)))
            .unwrap();
        s.insert(Fact::new("主人", "说", "有点开心").with_emotion(make_emotion_ctx("愉悦", 0.2)))
            .unwrap();

        let results = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(results.len(), 3);
        // 最高强度排第一
        assert_eq!(results[0].object, "超级开心");
        assert!(
            results[0].emotion_context.as_ref().unwrap().intensity
                >= results[1].emotion_context.as_ref().unwrap().intensity
        );
    }

    #[test]
    fn test_query_with_emotion_filter() {
        let s = new_store();
        s.insert(
            Fact::new("主人", "提到", "Rust 编程").with_emotion(make_emotion_ctx("兴奋", 0.7)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "提到", "Rust 所有权").with_emotion(make_emotion_ctx("平静", 0.3)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "提到", "Python 编程").with_emotion(make_emotion_ctx("兴奋", 0.5)),
        )
        .unwrap();

        // 关键词 "Rust" + 情感 "兴奋" → 只匹配第 1 条
        let r = s.query_with_emotion_filter("Rust", Some("兴奋")).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "Rust 编程");

        // 关键词 "编程" + 无过滤 → 匹配第 1 和第 3 条
        let r = s.query_with_emotion_filter("编程", None).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_backward_compat_no_emotion_context() {
        let s = new_store();
        // 插入没有 emotion_context 的事实（模拟旧数据）
        s.insert(Fact::new("主人", "喜欢", "咖啡")).unwrap();

        // query_by_emotion 应跳过无标注的事实
        let r = s.query_by_emotion("愉悦").unwrap();
        assert!(r.is_empty());

        // 常规 query 仍然正常返回
        let r = s.query("咖啡").unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].emotion_context.is_none());
    }
}
