// SPDX-License-Identifier: MIT
//! Vector embedding index — Text-to-vector embedding and similarity search.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::collections::HashMap;
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
/// Semantic cache — Caches recent query results to avoid redundant embedding + full scan.
///
/// 使用索引化双向链表实现 O(1) LRU：
/// Uses an index-based doubly-linked list for O(1) LRU operations:
/// - `key_to_slot`: HashMap 提供 O(1) 键查找 / O(1) key lookup
/// - `entries`: 槽位数组，prev/next 索引构成双向链表 / Slot array with prev/next indices
/// - `head`/`tail`: LRU/MRU 端指针 / LRU/MRU end pointers
/// - `free_list`: 回收已淘汰槽位，避免重复分配 / Recycled slots to avoid reallocation
pub struct SemanticCache {
    /// 键 → 槽位索引 / Key to slot index mapping
    key_to_slot: HashMap<String, usize>,
    /// 槽位数组 / Slot array
    entries: Vec<Slot>,
    /// 空闲槽位回收链 / Free slot recycling list
    free_list: Vec<usize>,
    /// LRU 端（最久未用，下一个淘汰） / LRU end (least recently used, next to evict)
    head: Option<usize>,
    /// MRU 端（最近使用） / MRU end (most recently used)
    tail: Option<usize>,
    /// 当前条目数 / Current entry count
    len: usize,
    /// 容量上限 / Capacity limit
    capacity: usize,
}

/// 缓存槽位 — 存储键值对及双向链表指针
/// Cache slot — Stores key-value pair and doubly-linked list pointers.
struct Slot {
    /// 查询键 / Query key
    key: String,
    /// 搜索结果 / Search results
    value: Vec<(u64, f32)>,
    /// 前驱槽位索引 / Previous slot index
    prev: Option<usize>,
    /// 后继槽位索引 / Next slot index
    next: Option<usize>,
}

impl SemanticCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            key_to_slot: HashMap::with_capacity(max_entries),
            entries: Vec::with_capacity(max_entries),
            free_list: Vec::new(),
            head: None,
            tail: None,
            len: 0,
            capacity: max_entries,
        }
    }

    /// 从双向链表中摘除指定槽位 / Unlink a slot from the doubly-linked list.
    fn unlink_slot(&mut self, slot_idx: usize) {
        let prev = self.entries[slot_idx].prev;
        let next = self.entries[slot_idx].next;

        // 修复前驱的 next 指针 / Fix predecessor's next pointer
        match prev {
            Some(p) => self.entries[p].next = next,
            None => self.head = next, // 摘除的是 head / Unlinking head
        }

        // 修复后继的 prev 指针 / Fix successor's prev pointer
        match next {
            Some(n) => self.entries[n].prev = prev,
            None => self.tail = prev, // 摘除的是 tail / Unlinking tail
        }

        // 清除被摘除槽位的指针 / Clear unlinked slot's pointers
        self.entries[slot_idx].prev = None;
        self.entries[slot_idx].next = None;
    }

    /// 将槽位链接到 MRU 端（尾部） / Link a slot to the MRU end (tail).
    fn link_tail(&mut self, slot_idx: usize) {
        self.entries[slot_idx].next = None;
        self.entries[slot_idx].prev = self.tail;

        if let Some(t) = self.tail {
            self.entries[t].next = Some(slot_idx);
        } else {
            // 链表为空，新槽位同时是 head / List is empty, new slot is also head
            self.head = Some(slot_idx);
        }
        self.tail = Some(slot_idx);
    }

    /// 分配一个新槽位，优先复用已回收的 / Allocate a new slot, reusing recycled ones first.
    fn alloc_slot(&mut self, key: String, value: Vec<(u64, f32)>) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.entries[idx] = Slot {
                key,
                value,
                prev: None,
                next: None,
            };
            idx
        } else {
            let idx = self.entries.len();
            self.entries.push(Slot {
                key,
                value,
                prev: None,
                next: None,
            });
            idx
        }
    }

    /// 淘汰 LRU 端（头部）槽位 / Evict the LRU end (head) slot.
    fn evict_head(&mut self) {
        if let Some(h) = self.head {
            self.unlink_slot(h);
            // 从索引中移除键 / Remove key from index
            let key = std::mem::take(&mut self.entries[h].key);
            self.key_to_slot.remove(&key);
            // 回收槽位 / Recycle slot
            self.free_list.push(h);
            self.len -= 1;
        }
    }

    /// 命中返回 Some，并将该条目移到 MRU 端（最近使用）
    /// Returns cached result on hit, moving the entry to MRU end.
    /// O(1): HashMap 查找 + 链表 unlink + link_tail
    pub fn get(&mut self, query: &str) -> Option<&Vec<(u64, f32)>> {
        if let Some(&slot_idx) = self.key_to_slot.get(query) {
            // 摘除并重新链接到尾部 / Unlink and re-link to tail
            self.unlink_slot(slot_idx);
            self.link_tail(slot_idx);
            Some(&self.entries[slot_idx].value)
        } else {
            None
        }
    }

    /// 写入缓存，超出上限时淘汰最久未用的（LRU 端）
    /// Inserts/updates a cache entry, evicting LRU end if over capacity.
    /// O(1): HashMap 查找 + 链表操作
    pub fn set(&mut self, query: String, results: Vec<(u64, f32)>) {
        if let Some(&slot_idx) = self.key_to_slot.get(&query) {
            // 键已存在：更新值并移到尾部 / Key exists: update value and move to tail
            self.unlink_slot(slot_idx);
            self.entries[slot_idx].value = results;
            self.link_tail(slot_idx);
        } else {
            // 新键：分配槽位并链接 / New key: allocate slot and link
            if self.len >= self.capacity && self.capacity > 0 {
                self.evict_head();
            }
            let slot_idx = self.alloc_slot(query.clone(), results);
            self.key_to_slot.insert(query, slot_idx);
            self.link_tail(slot_idx);
            self.len += 1;
        }
    }

    /// 批量写入，每条 O(1) / Batch insert, each entry O(1).
    /// 总复杂度 O(M) where M = entries.len()
    pub fn set_batch(&mut self, entries: Vec<(String, Vec<(u64, f32)>)>) {
        for (query, results) in entries {
            self.set(query, results);
        }
    }

    pub fn clear(&mut self) {
        self.key_to_slot.clear();
        self.entries.clear();
        self.free_list.clear();
        self.head = None;
        self.tail = None;
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ════════════════════════════════════════════════════════════════════
// SemanticRecallEngine — 语义召回引擎 / Semantic Recall Engine
// ════════════════════════════════════════════════════════════════════

/// 语义召回引擎 — 封装向量化 + 索引 + 缓存 / Semantic recall engine
///
/// 数字生命的"语义记忆皮层"——将文本转为向量，按余弦相似度召回。
/// 当 embedding feature 开启且模型可用时，数字生命能按"意思"回忆；
/// 否则降级为关键词召回（调用方处理）。
///
/// The digital life's "semantic memory cortex" — converts text to vectors,
/// recalls by cosine similarity. When embedding feature is enabled and model
/// is available, digital life can recall by meaning; otherwise falls back to
/// keyword recall (handled by caller).
#[cfg(feature = "embedding")]
pub struct SemanticRecallEngine {
    vectorizer: FastEmbedVectorizer,
    index: BruteForceIndex,
    cache: SemanticCache,
    /// canonical_key → u64 id 映射 / canonical_key → u64 id mapping
    key_to_id: HashMap<String, u64>,
    /// u64 id → canonical_key 反向映射（搜索结果转回 key）/ Reverse mapping for search result conversion
    id_to_key: HashMap<u64, String>,
    next_id: u64,
}

#[cfg(feature = "embedding")]
impl SemanticRecallEngine {
    /// 创建语义召回引擎 — 尝试加载模型，失败返回 None
    /// Create semantic recall engine — tries to load model, returns None on failure
    pub fn new() -> Option<Self> {
        match FastEmbedVectorizer::new() {
            Ok(vectorizer) => {
                tracing::info!(
                    "FastEmbed 模型加载成功 — 语义召回引擎就绪 / FastEmbed model loaded — semantic recall engine ready"
                );
                Some(Self {
                    vectorizer,
                    index: BruteForceIndex::new(),
                    cache: SemanticCache::new(128),
                    key_to_id: HashMap::new(),
                    id_to_key: HashMap::new(),
                    next_id: 0,
                })
            }
            Err(e) => {
                tracing::warn!(
                    "FastEmbed 模型加载失败 — 降级为关键词召回 / FastEmbed model load failed — falling back to keyword recall: {}",
                    e
                );
                None
            }
        }
    }

    /// 嵌入文本并存入向量索引 — key→id 映射建立
    /// Embed text and store in vector index — establishes key→id mapping
    ///
    /// 同一 key 重复索引会追加新向量（BruteForceIndex 不支持更新），
    /// 搜索时可能返回同一 key 的多个向量，调用方通过 canonical key 去重。
    ///
    /// Re-indexing the same key appends a new vector (BruteForceIndex has no update);
    /// search may return multiple vectors for the same key, caller deduplicates by canonical key.
    pub fn index_text(&mut self, key: &str, text: &str) {
        let vec = match self.vectorizer.embed(text) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("嵌入失败 / Embedding failed: {}", e);
                return;
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        self.key_to_id.insert(key.to_string(), id);
        self.id_to_key.insert(id, key.to_string());
        if let Err(e) = self.index.insert(id, vec) {
            tracing::debug!("向量索引插入失败 / Vector index insert failed: {}", e);
        }
    }

    /// 语义搜索 — 返回 (canonical_key, similarity) 列表
    /// Semantic search — returns (canonical_key, similarity) list
    ///
    /// 内部使用 SemanticCache 缓存最近查询，避免重复嵌入 + 全盘扫描。
    /// Uses SemanticCache internally to avoid redundant embedding + full scan.
    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        // 查缓存 — 命中则直接返回 / Check cache — return on hit
        if let Some(cached) = self.cache.get(query) {
            return cached
                .iter()
                .filter_map(|(id, sim)| self.id_to_key.get(id).map(|k| (k.clone(), *sim)))
                .collect();
        }
        // 缓存未命中 — 嵌入查询并搜索 / Cache miss — embed query and search
        let query_vec = match self.vectorizer.embed(query) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!("查询嵌入失败 / Query embedding failed: {}", e);
                return Vec::new();
            }
        };
        let results = self.index.search(&query_vec, top_k);
        // 写入缓存 / Write to cache
        self.cache.set(query.to_string(), results.clone());
        // 转换 id → key / Convert id → key
        results
            .iter()
            .filter_map(|(id, sim)| self.id_to_key.get(id).map(|k| (k.clone(), *sim)))
            .collect()
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

    // ── O(1) 性能验证测试 / O(1) Performance Verification Tests ──

    #[test]
    fn test_cache_large_capacity_lru_correctness() {
        // 验证大容量下 LRU 正确性 / Verify LRU correctness at scale
        let n = 1000;
        let mut cache = SemanticCache::new(n);

        // 填满缓存 / Fill cache to capacity
        for i in 0..n {
            cache.set(format!("q{}", i), vec![(i as u64, 0.9)]);
        }
        assert_eq!(cache.len(), n);

        // 访问前半部分，使其变为最近使用 / Access first half to make them recently used
        for i in 0..n / 2 {
            let result = cache.get(&format!("q{}", i));
            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].0, i as u64);
        }

        // 插入 n/2 个新条目，应淘汰后半部分（最久未用）
        // Insert n/2 new entries, should evict second half (least recently used)
        for i in 0..n / 2 {
            cache.set(format!("new{}", i), vec![(1000 + i as u64, 0.8)]);
        }

        // 前半部分应仍在缓存中 / First half should still be cached
        for i in 0..n / 2 {
            assert!(
                cache.get(&format!("q{}", i)).is_some(),
                "q{} should still be cached",
                i
            );
        }

        // 后半部分应已被淘汰 / Second half should be evicted
        for i in n / 2..n {
            assert!(
                cache.get(&format!("q{}", i)).is_none(),
                "q{} should have been evicted",
                i
            );
        }
    }

    #[test]
    fn test_cache_zero_capacity() {
        // 零容量缓存不应 panic / Zero-capacity cache should not panic
        let mut cache = SemanticCache::new(0);
        cache.set("q".into(), vec![(1, 1.0)]);
        assert_eq!(cache.len(), 0);
        assert!(cache.get("q").is_none());
    }

    #[test]
    fn test_cache_batch_preserves_lru_order() {
        // 批量写入后 LRU 顺序正确 / LRU order correct after batch insert
        let mut cache = SemanticCache::new(4);
        cache.set("a".into(), vec![(1, 1.0)]);
        cache.set("b".into(), vec![(2, 1.0)]);

        // 批量写入 c, d, e — 应淘汰 a, b, c 中最久的
        // Batch insert c, d, e — should evict oldest among a, b, c
        cache.set_batch(vec![
            ("c".into(), vec![(3, 1.0)]),
            ("d".into(), vec![(4, 1.0)]),
            ("e".into(), vec![(5, 1.0)]),
        ]);

        assert_eq!(cache.len(), 4);
        // a 和 b 应被淘汰 / a and b should be evicted
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_none());
        // c, d, e 应在缓存中 / c, d, e should be cached
        assert!(cache.get("c").is_some());
        assert!(cache.get("d").is_some());
        assert!(cache.get("e").is_some());
    }

    #[test]
    fn test_cache_repeated_get_keeps_entry() {
        // 反复 get 不应导致条目丢失 / Repeated get should not lose entry
        let mut cache = SemanticCache::new(2);
        cache.set("a".into(), vec![(1, 1.0)]);
        cache.set("b".into(), vec![(2, 1.0)]);

        // 反复访问 a / Repeatedly access a
        for _ in 0..100 {
            assert!(cache.get("a").is_some());
        }

        // 插入 c，应淘汰 b（最久未用）而非 a
        // Insert c, should evict b (LRU) not a
        cache.set("c".into(), vec![(3, 1.0)]);
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }

    // ── SemanticRecallEngine 测试 / SemanticRecallEngine Tests ──
    // 仅在 embedding feature 开启时编译 / Only compiled when embedding feature is enabled

    #[cfg(feature = "embedding")]
    #[test]
    fn test_semantic_recall_engine_new() {
        // 模型可能无法在 CI 下载——无论成功失败都不应 panic
        // Model may not be downloadable in CI — should not panic regardless of success/failure
        let engine = SemanticRecallEngine::new();
        if engine.is_some() {
            tracing::info!("SemanticRecallEngine 模型加载成功 / model loaded successfully");
        } else {
            tracing::info!(
                "SemanticRecallEngine 模型加载失败（CI 环境可能无网络）/ model load failed (CI may have no network)"
            );
        }
    }

    #[cfg(feature = "embedding")]
    #[test]
    fn test_semantic_recall_index_and_search() {
        // 仅在模型可用时测试往返 / Only test roundtrip when model is available
        let mut engine = match SemanticRecallEngine::new() {
            Some(e) => e,
            None => return, // 模型不可用，跳过 / Model unavailable, skip
        };
        // 索引几条文本 / Index a few texts
        engine.index_text("主人|喜欢|编程", "主人 喜欢 编程");
        engine.index_text("主人|喜欢|rust", "主人 喜欢 Rust");
        engine.index_text("主人|在|杭州", "主人 在 杭州");

        // 搜索与编程相关的查询 / Search with a programming-related query
        let results = engine.search("编程语言", 5);
        // 应有结果返回 / Should return results
        assert!(
            !results.is_empty(),
            "语义搜索应返回结果 / semantic search should return results"
        );
    }
}
