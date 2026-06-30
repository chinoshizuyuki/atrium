// SPDX-License-Identifier: MIT
//! Vector embedding index — Text-to-vector embedding and similarity search.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::collections::VecDeque;
/// 向量化接口 文本 - 浮点向量
pub trait Vectorizer: Send + Sync {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
}

/// 基于 fastembed 实现
pub struct FastEmbedVectorizer {
    model: TextEmbedding,
}

impl FastEmbedVectorizer {
    pub fn new() -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }
}

impl Vectorizer for FastEmbedVectorizer {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let batches = self.model.embed(vec![text], None)?;
        Ok(batches.into_iter().next().unwrap_or_default())
    }
}

/// 暴力搜索索引
pub struct BruteForceIndex {
    vectors: Vec<(u64, Vec<f32>)>, // 记忆id, 向量
}

impl BruteForceIndex {
    pub fn new() -> Self {
        Self {
            vectors: Vec::new(),
        }
    }

    /// 插入向量，自动检测维度一致性
    pub fn insert(&mut self, id: u64, vec: Vec<f32>) -> anyhow::Result<()> {
        if let Some((_, first)) = self.vectors.first() {
            if vec.len() != first.len() {
                anyhow::bail!(
                    "Dimension mismatch: expected {} dimensions, received {} dimensions",
                    first.len(),
                    vec.len()
                );
            }
        }
        self.vectors.push((id, vec));
        Ok(())
    }

    /// 余弦相似度
    fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum();
        let nb: f32 = b.iter().map(|x| x * x).sum();
        dot / (na.sqrt() * nb.sqrt() + 1e-8)
    }

    /// 搜索 top_k 条，返回 (id, 相似度)
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(u64, f32)> {
        let mut scored: Vec<(u64, f32)> = self
            .vectors
            .iter()
            .map(|(id, vec)| (*id, Self::cosine_sim(query, vec)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

/// 语义缓存 - 缓存最近查询到的结果，避免重复嵌入 + 全盘扫描
pub struct SemanticCache {
    cache: VecDeque<(String, Vec<(u64, f32)>)>,
    max_entries: usize,
}

impl SemanticCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    /// 命中返回 Some，并将该条目移到末尾（最近使用）
    pub fn get(&mut self, query: &str) -> Option<&Vec<(u64, f32)>> {
        if let Some(pos) = self.cache.iter().position(|(q, _)| q.as_str() == query) {
            let entry = self.cache.remove(pos).expect("index init");
            self.cache.push_back(entry);
            Some(&self.cache.back().expect("index init").1)
        } else {
            None
        }
    }

    /// 写入缓存，超出上限时淘汰最久未用的（队首）
    pub fn set(&mut self, query: String, results: Vec<(u64, f32)>) {
        if let Some(pos) = self.cache.iter().position(|(q, _)| q.as_str() == query) {
            self.cache.remove(pos);
        }
        if self.cache.len() >= self.max_entries {
            self.cache.pop_front();
        }
        self.cache.push_back((query, results));
    }

    /// 批量写入，单次淘汰避免多次 pop_front 的开销
    pub fn set_batch(&mut self, entries: Vec<(String, Vec<(u64, f32)>)>) {
        for (query, results) in entries {
            if let Some(pos) = self.cache.iter().position(|(q, _)| q.as_str() == query) {
                self.cache.remove(pos);
            }
            self.cache.push_back((query, results));
        }
        // 批量写入后一次性裁剪到容量上限
        while self.cache.len() > self.max_entries {
            self.cache.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    // ── BruteForceIndex 测试 ──

    #[test]
    fn test_insert_and_search() {
        let mut idx = BruteForceIndex::new();
        idx.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        idx.insert(2, vec![0.0, 1.0, 0.0]).unwrap();
        idx.insert(3, vec![0.0, 0.0, 1.0]).unwrap();

        // 查询与 1 相似的
        let results = idx.search(&vec![0.9, 0.1, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1); // 1 最相似
    }

    #[test]
    fn test_empty_index_search() {
        let idx = BruteForceIndex::new();
        let results = idx.search(&vec![1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut idx = BruteForceIndex::new();
        idx.insert(1, vec![1.0, 0.0]).unwrap();
        let err = idx.insert(2, vec![1.0, 0.0, 0.0]).unwrap_err();
        assert!(err.to_string().contains("Dimension mismatch"));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![3.0, 4.0];
        let sim = BruteForceIndex::cosine_sim(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let sim = BruteForceIndex::cosine_sim(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let sim = BruteForceIndex::cosine_sim(&[1.0, 0.0], &[-1.0, 0.0]);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_search_returns_top_k() {
        let mut idx = BruteForceIndex::new();
        // 让每个向量有不同的方向
        for i in 0..20 {
            let v = vec![1.0, (i as f32) * 0.1, 0.0, 0.0, 0.0];
            idx.insert(i as u64, v).unwrap();
        }
        // 查询最接近 (1.0, 0.0, 0.0, 0.0, 0.0) 的——即第 1 维最小的
        let results = idx.search(&vec![1.0, 0.0, 0.0, 0.0, 0.0], 5);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].0, 0); // id=0 的第 1 维最小 (0.0)，最接近查询
    }

    #[test]
    fn test_single_vector() {
        let mut idx = BruteForceIndex::new();
        idx.insert(42, vec![0.5, 0.5]).unwrap();
        let results = idx.search(&vec![0.5, 0.5], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 42);
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    // ── SemanticCache 测试 ──

    #[test]
    fn test_cache_miss() {
        let mut cache = SemanticCache::new(3);
        assert!(cache.get("hello").is_none());
    }

    #[test]
    fn test_cache_set_and_hit() {
        let mut cache = SemanticCache::new(3);
        cache.set("query1".into(), vec![(1, 0.9), (2, 0.8)]);
        let result = cache.get("query1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
        assert_eq!(result.unwrap()[0].0, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = SemanticCache::new(2);
        cache.set("a".into(), vec![(1, 1.0)]);
        cache.set("b".into(), vec![(2, 1.0)]);
        cache.set("c".into(), vec![(3, 1.0)]); // 挤出 a

        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_lru_update() {
        let mut cache = SemanticCache::new(3);
        cache.set("a".into(), vec![(1, 1.0)]);
        cache.set("b".into(), vec![(2, 1.0)]);
        cache.set("c".into(), vec![(3, 1.0)]);

        // 访问 a，把 a 移到末尾
        cache.get("a");
        // 再插入 d，应挤出 b（最久未用）
        cache.set("d".into(), vec![(4, 1.0)]);

        assert!(cache.get("b").is_none()); // b 被挤出
        assert!(cache.get("a").is_some()); // a 还在
        assert!(cache.get("c").is_some());
        assert!(cache.get("d").is_some());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = SemanticCache::new(5);
        cache.set("a".into(), vec![(1, 1.0)]);
        cache.set("b".into(), vec![(2, 1.0)]);
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = SemanticCache::new(3);
        cache.set("key".into(), vec![(1, 1.0)]);
        cache.set("key".into(), vec![(2, 1.0)]); // 覆盖
        assert_eq!(cache.len(), 1);
        let result = cache.get("key").unwrap();
        assert_eq!(result[0].0, 2); // 新值
    }
}
