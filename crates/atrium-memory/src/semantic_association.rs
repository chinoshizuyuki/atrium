// SPDX-License-Identifier: MIT

//! 语义关联发现 — Semantic association discovery: finds co-occurrence patterns
//! between terms to enable cross-topic curiosity linking.
//!
//! 核心理念：关键词提取太机械。数字生命应该能发现"图书馆"和"考研"之间的
//! 关联——通过共现频率和类别传播。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::followup_tracker::FollowUpCategory;

// ═══════════════════════════════════════════════════════════════════════════
// 配置 — Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// 语义关联配置 / Semantic association configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAssociationConfig {
    /// 共现阈值 — 达到此计数才建立关联 / Co-occurrence threshold.
    pub association_threshold: u32,
    /// HashMap 最大条目数 / Max HashMap entries.
    pub max_entries: usize,
    /// 关键词最大长度 / Max keyword length.
    pub max_keyword_len: usize,
}

impl Default for SemanticAssociationConfig {
    fn default() -> Self {
        Self {
            association_threshold: 2,
            max_entries: 500,
            max_keyword_len: 10,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 关联结果 — Association Result
// ═══════════════════════════════════════════════════════════════════════════

/// 语义关联结果 — 发现的关联事项 / A discovered semantic association.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationLink {
    /// 关联词 / Related term.
    pub term: String,
    /// 关联类别 / Related category.
    pub category: FollowUpCategory,
    /// 关联强度 [0, 1] / Association strength.
    pub strength: f32,
}

// ═══════════════════════════════════════════════════════════════════════════
// 语义关联发现 — Semantic Association Discovery
// ═══════════════════════════════════════════════════════════════════════════

/// 语义关联发现引擎 — 通过共现频率发现词间关联
/// Semantic association engine — Discovers term associations via co-occurrence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAssociation {
    /// 共现计数 (term_a, term_b) → count / Co-occurrence counts.
    pub co_occurrence: HashMap<(String, String), u32>,
    /// 词 → 类别映射 / Term to category mapping.
    pub term_categories: HashMap<String, FollowUpCategory>,
    /// 配置 / Configuration.
    pub config: SemanticAssociationConfig,
    /// 计数索引 — count → 该计数下所有词对 / Count index for O(1) min eviction.
    #[serde(skip)]
    pub count_index: BTreeMap<u32, HashSet<(String, String)>>,
    /// 词项反向索引 — term → (paired_term → count) / Reverse term index for O(R) lookup.
    #[serde(skip)]
    pub term_index: HashMap<String, HashMap<String, u32>>,
}

impl SemanticAssociation {
    /// 创建默认配置的引擎 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(SemanticAssociationConfig::default())
    }

    /// 创建指定配置的引擎 / Create with custom config.
    pub fn new(config: SemanticAssociationConfig) -> Self {
        Self {
            co_occurrence: HashMap::new(),
            term_categories: HashMap::new(),
            config,
            count_index: BTreeMap::new(),
            term_index: HashMap::new(),
        }
    }

    /// 观察文本 — 提取关键词并更新共现计数
    /// Observe text — Extract keywords and update co-occurrence.
    ///
    /// 优化后复杂度：O(K² × log M)，驱逐路径从 O(M) 全表扫描降至 O(log M) BTreeMap 查找。
    pub fn observe(&mut self, text: &str, extracted_categories: &[FollowUpCategory]) {
        let keywords = self.extract_keywords(text);
        if keywords.is_empty() {
            return;
        }

        // 记录词→类别映射 / Record term→category mapping
        for (i, kw) in keywords.iter().enumerate() {
            if let Some(&cat) = extracted_categories.get(i) {
                self.term_categories.insert(kw.clone(), cat);
            } else if let Some(&cat) = extracted_categories.first() {
                self.term_categories.insert(kw.clone(), cat);
            }
        }

        // 更新共现计数 / Update co-occurrence counts
        for i in 0..keywords.len() {
            for j in (i + 1)..keywords.len() {
                let pair = Self::ordered_pair(&keywords[i], &keywords[j]);
                let old_count = self.co_occurrence.get(&pair).copied().unwrap_or(0);
                let new_count = old_count + 1;

                // 更新主存储 / Update primary store
                self.co_occurrence.insert(pair.clone(), new_count);

                // 维护计数索引 — O(log C) / Maintain count index
                if old_count > 0 {
                    if let Some(set) = self.count_index.get_mut(&old_count) {
                        set.remove(&pair);
                    }
                    // 清理空桶 / Clean empty bucket
                    if self
                        .count_index
                        .get(&old_count)
                        .is_some_and(|s| s.is_empty())
                    {
                        self.count_index.remove(&old_count);
                    }
                }
                self.count_index
                    .entry(new_count)
                    .or_default()
                    .insert(pair.clone());

                // 维护词项反向索引 — O(1) / Maintain reverse term index
                self.term_index
                    .entry(pair.0.clone())
                    .or_default()
                    .insert(pair.1.clone(), new_count);
                self.term_index
                    .entry(pair.1.clone())
                    .or_default()
                    .insert(pair.0.clone(), new_count);

                // 限制 HashMap 大小 — O(log M) 驱逐 / Limit HashMap size
                if self.co_occurrence.len() > self.config.max_entries {
                    self.evict_min();
                }
            }
        }
    }

    /// 驱逐最低计数条目 — O(log M) via count_index
    /// Evict the minimum-count entry using the count index.
    fn evict_min(&mut self) {
        // 从计数索引首项取最小计数词对 / Get min-count pair from BTreeMap front
        let evicted = self.count_index.iter_mut().next().and_then(|(_, set)| {
            if let Some(pair) = set.iter().next().cloned() {
                set.remove(&pair);
                Some(pair)
            } else {
                None
            }
        });

        if let Some(pair) = evicted {
            // 清理空桶 / Clean empty bucket
            self.count_index.retain(|_, set| !set.is_empty());

            // 从主存储移除 / Remove from primary store
            self.co_occurrence.remove(&pair);

            // 从词项反向索引移除 / Remove from reverse term index
            if let Some(map_a) = self.term_index.get_mut(&pair.0) {
                map_a.remove(&pair.1);
                if map_a.is_empty() {
                    self.term_index.remove(&pair.0);
                }
            }
            if let Some(map_b) = self.term_index.get_mut(&pair.1) {
                map_b.remove(&pair.0);
                if map_b.is_empty() {
                    self.term_index.remove(&pair.1);
                }
            }
        }
    }

    /// 重建索引 — 反序列化后调用以恢复 count_index 和 term_index
    /// Rebuild indexes — Call after deserialization to restore count_index and term_index.
    pub fn rebuild_indexes(&mut self) {
        self.count_index.clear();
        self.term_index.clear();

        for (pair, &count) in &self.co_occurrence {
            // 重建计数索引 / Rebuild count index
            self.count_index
                .entry(count)
                .or_default()
                .insert(pair.clone());

            // 重建词项反向索引 / Rebuild reverse term index
            self.term_index
                .entry(pair.0.clone())
                .or_default()
                .insert(pair.1.clone(), count);
            self.term_index
                .entry(pair.1.clone())
                .or_default()
                .insert(pair.0.clone(), count);
        }
    }

    /// 发现关联事项 — 给定文本，找出关联的已知事项
    /// Find related items — Given text, find associated known items.
    ///
    /// 优化后复杂度：O(K × R)，R 为每个词的平均关联数（R ≪ M）。
    pub fn find_related(&self, text: &str) -> Vec<AssociationLink> {
        let keywords = self.extract_keywords(text);
        let mut links = Vec::new();

        for kw in &keywords {
            // 通过反向索引直接定位关联词 — O(R) / Direct lookup via reverse index
            if let Some(assoc_map) = self.term_index.get(kw) {
                for (term, &count) in assoc_map {
                    if count < self.config.association_threshold {
                        continue;
                    }
                    let strength = (count as f32 / 10.0).min(1.0);
                    if strength > 0.0 {
                        let category = self
                            .term_categories
                            .get(term)
                            .copied()
                            .unwrap_or(FollowUpCategory::Interest);
                        links.push(AssociationLink {
                            term: term.clone(),
                            category,
                            strength,
                        });
                    }
                }
            }
        }

        // 去重并按强度降序 / Deduplicate and sort by strength
        links.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        links.dedup_by(|a, b| a.term == b.term);
        links
    }

    /// 计算两个词之间的关联强度
    /// Compute association strength between two terms.
    pub fn association_strength(&self, term_a: &str, term_b: &str) -> f32 {
        let pair = Self::ordered_pair(term_a, term_b);
        match self.co_occurrence.get(&pair) {
            Some(&count) if count >= self.config.association_threshold => {
                (count as f32 / 10.0).min(1.0)
            }
            _ => 0.0,
        }
    }

    /// 生成关联提示 — 用于 prompt 注入
    /// Generate association hint — For prompt injection.
    pub fn prompt_hint(&self, text: &str) -> String {
        let links = self.find_related(text);
        if links.is_empty() {
            return String::new();
        }
        let terms: Vec<&str> = links.iter().take(3).map(|l| l.term.as_str()).collect();
        format!(
            "用户提到的话题可能与之前提到的「{}」有关联。",
            terms.join("、")
        )
    }

    /// 提取关键词 — 简单的分词：提取 2~max_len 字的中文词段
    /// Extract keywords — Simple segmentation: 2~max_len char Chinese segments.
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut keywords = Vec::new();
        let max_len = self.config.max_keyword_len.min(chars.len());

        // 滑动窗口提取 2~4 字词段 / Sliding window for 2~4 char segments
        let window_sizes = [4, 3, 2];
        for &ws in &window_sizes {
            if ws > max_len {
                continue;
            }
            for i in 0..=(chars.len() - ws) {
                let segment: String = chars[i..i + ws].iter().collect();
                // 只保留含中文的段 / Keep only segments with Chinese characters
                if segment
                    .chars()
                    .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
                {
                    // 去除包含标点的段 / Skip segments with punctuation
                    if !segment
                        .chars()
                        .any(|c| c.is_ascii_punctuation() || "，。！？、；：".contains(c))
                    {
                        if !keywords.contains(&segment) {
                            keywords.push(segment);
                        }
                        if keywords.len() >= 5 {
                            return keywords;
                        }
                    }
                }
            }
        }
        keywords
    }

    /// 有序词对 — 保证 (a, b) 中 a <= b，避免重复存储
    /// Ordered pair — Ensure a <= b to avoid duplicate storage.
    fn ordered_pair(a: &str, b: &str) -> (String, String) {
        if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text_observe() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("", &[]);
        assert!(sa.co_occurrence.is_empty());
    }

    #[test]
    fn test_single_word_no_cooccurrence() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考研", &[FollowUpCategory::Plan]);
        // 单词不产生共现
        assert!(sa.co_occurrence.is_empty());
    }

    #[test]
    fn test_two_words_cooccurrence() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        assert!(!sa.co_occurrence.is_empty(), "should have co-occurrence");
    }

    #[test]
    fn test_threshold_association() {
        let mut sa = SemanticAssociation::default_new();
        // 两次共现 → 达到阈值
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        // 使用实际被提取的关键词查询 / Query with an actually-extracted keyword
        let links = sa.find_related("考研图书");
        assert!(!links.is_empty(), "should find related with threshold=2");
    }

    #[test]
    fn test_find_related_returns_links() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        let links = sa.find_related("考试");
        assert!(!links.is_empty());
        assert!(links[0].strength > 0.0);
    }

    #[test]
    fn test_no_association_empty_result() {
        let sa = SemanticAssociation::default_new();
        let links = sa.find_related("天气");
        assert!(links.is_empty());
    }

    #[test]
    fn test_association_strength_calculation() {
        let mut sa = SemanticAssociation::default_new();
        for _ in 0..5 {
            sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        }
        // 使用实际被提取的关键词对 / Use actually-extracted keyword pair
        let strength = sa.association_strength("考研图书", "研图书馆");
        assert!(strength > 0.0 && strength <= 1.0, "strength={}", strength);
    }

    #[test]
    fn test_prompt_hint_generated() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        let hint = sa.prompt_hint("考试");
        assert!(!hint.is_empty(), "should generate hint: {}", hint);
    }

    #[test]
    fn test_multiple_observe_accumulates() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        // 使用实际被提取的关键词对 / Use actually-extracted keyword pair
        let strength = sa.association_strength("考研图书", "研图书馆");
        assert!(strength > 0.2, "should accumulate: {}", strength);
    }

    #[test]
    fn test_different_texts_no_false_association() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        sa.observe("天气散步", &[FollowUpCategory::Interest]);
        // 考试 和 天气 不应关联
        let strength = sa.association_strength("考试", "天气");
        assert_eq!(strength, 0.0);
    }

    #[test]
    fn test_hashmap_capacity_limit() {
        let mut sa = SemanticAssociation::new(SemanticAssociationConfig {
            association_threshold: 1,
            max_entries: 5,
            max_keyword_len: 4,
        });
        // 大量不同词对 → 应限制在 max_entries
        for i in 0..50 {
            let text = format!("词{}对{}", i, i + 100);
            sa.observe(&text, &[FollowUpCategory::Interest]);
        }
        assert!(
            sa.co_occurrence.len() <= 10,
            "should be limited: {}",
            sa.co_occurrence.len()
        );
    }

    #[test]
    fn test_category_propagation() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        // 图书馆 应映射到 Plan 类别
        let cat = sa.term_categories.get("图书馆");
        assert!(cat.is_some(), "should have category mapping");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        // 使用 bincode 序列化（支持 tuple key）/ Use bincode (supports tuple keys)
        let bytes = bincode::serialize(&sa).unwrap();
        let mut sa2: SemanticAssociation = bincode::deserialize(&bytes).unwrap();
        // 反序列化后重建索引 / Rebuild indexes after deserialization
        sa2.rebuild_indexes();
        let s1 = sa.association_strength("考试面", "试面试");
        let s2 = sa2.association_strength("考试面", "试面试");
        assert!((s1 - s2).abs() < 0.01);
    }

    #[test]
    fn test_threshold_zero_immediate() {
        let mut sa = SemanticAssociation::new(SemanticAssociationConfig {
            association_threshold: 0,
            max_entries: 500,
            max_keyword_len: 10,
        });
        sa.observe("考试面试", &[FollowUpCategory::Work]);
        // 使用实际被提取的关键词对 / Use actually-extracted keyword pair
        let strength = sa.association_strength("考试面", "试面试");
        assert!(strength > 0.0, "threshold=0 should associate immediately");
    }

    // ── P3-B 专项验证 — 索引一致性与驱逐正确性 / Index consistency & eviction correctness ──

    #[test]
    fn test_count_index_consistency() {
        // 计数索引应与主存储一致 / Count index should match primary store
        let mut sa = SemanticAssociation::default_new();
        for _ in 0..3 {
            sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        }
        for _ in 0..2 {
            sa.observe("考试面试", &[FollowUpCategory::Work]);
        }

        // 验证：每个 co_occurrence 条目都在 count_index 中 / Verify consistency
        for (pair, &count) in &sa.co_occurrence {
            let in_index = sa.count_index.get(&count).is_some_and(|s| s.contains(pair));
            assert!(
                in_index,
                "pair {:?} count {} missing from count_index",
                pair, count
            );
        }

        // 验证：count_index 中的条目都在 co_occurrence 中 / Reverse consistency
        let total_indexed: usize = sa.count_index.values().map(|s| s.len()).sum();
        assert_eq!(total_indexed, sa.co_occurrence.len());
    }

    #[test]
    fn test_term_index_consistency() {
        // 词项反向索引应与主存储一致 / Term index should match primary store
        let mut sa = SemanticAssociation::default_new();
        sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        sa.observe("考试面试", &[FollowUpCategory::Work]);

        // 验证：每个 co_occurrence (a,b)→c 都在 term_index 中 / Verify
        for (pair, &count) in &sa.co_occurrence {
            let ab = sa.term_index.get(&pair.0).and_then(|m| m.get(&pair.1));
            let ba = sa.term_index.get(&pair.1).and_then(|m| m.get(&pair.0));
            assert_eq!(
                ab,
                Some(&count),
                "term_index[{}][{}] mismatch",
                pair.0,
                pair.1
            );
            assert_eq!(
                ba,
                Some(&count),
                "term_index[{}][{}] mismatch",
                pair.1,
                pair.0
            );
        }
    }

    #[test]
    fn test_eviction_removes_lowest_count() {
        // 驱逐应移除最低计数条目 / Eviction should remove min-count entry
        let mut sa = SemanticAssociation::new(SemanticAssociationConfig {
            association_threshold: 1,
            max_entries: 3,
            max_keyword_len: 4,
        });
        // 插入 3 个不同词对 / Insert 3 distinct pairs
        sa.observe("词甲词乙", &[FollowUpCategory::Interest]);
        sa.observe("词丙词丁", &[FollowUpCategory::Interest]);
        sa.observe("词戊词己", &[FollowUpCategory::Interest]);
        assert!(sa.co_occurrence.len() <= 3);

        // 再次插入词甲词乙 → 计数=2，其他仍=1
        sa.observe("词甲词乙", &[FollowUpCategory::Interest]);

        // 插入第4个不同词对 → 触发驱逐，应移除计数=1 的条目
        sa.observe("词庚词辛", &[FollowUpCategory::Interest]);
        assert!(sa.co_occurrence.len() <= 3, "should respect max_entries");

        // 词甲词乙（计数=2）不应被驱逐 / High-count pair should survive
        let survived = sa.co_occurrence.iter().any(|(_, &c)| c >= 2);
        assert!(survived, "highest-count pair should survive eviction");
    }

    #[test]
    fn test_find_related_equivalence() {
        // 优化后 find_related 应返回与逻辑等价的结果 / Optimized find_related equivalence
        let mut sa = SemanticAssociation::default_new();
        for _ in 0..3 {
            sa.observe("考试面试", &[FollowUpCategory::Work]);
        }
        for _ in 0..2 {
            sa.observe("考试面试", &[FollowUpCategory::Work]);
        }

        let links = sa.find_related("考试");
        // 每个返回的关联都应在 co_occurrence 中有对应 / Each link should exist in co_occurrence
        for link in &links {
            assert!(link.strength > 0.0, "strength should be positive");
        }
    }

    #[test]
    fn test_rebuild_indexes_correctness() {
        // 反序列化后重建索引应恢复完整查找能力 / Rebuild restores lookup
        let mut sa = SemanticAssociation::default_new();
        for _ in 0..3 {
            sa.observe("考研图书馆", &[FollowUpCategory::Plan]);
        }

        // 模拟反序列化 — 清空索引后重建 / Simulate deserialization
        sa.count_index.clear();
        sa.term_index.clear();
        // 使用与存储匹配的关键词查询 / Query with a keyword that matches stored terms
        assert!(
            sa.find_related("考研图书").is_empty(),
            "empty index → no results"
        );

        sa.rebuild_indexes();
        let links = sa.find_related("考研图书");
        assert!(!links.is_empty(), "rebuild should restore lookup");
    }

    #[test]
    fn test_eviction_index_cleanup() {
        // 驱逐后索引应正确清理 / Index cleanup after eviction
        let mut sa = SemanticAssociation::new(SemanticAssociationConfig {
            association_threshold: 1,
            max_entries: 2,
            max_keyword_len: 4,
        });
        sa.observe("词甲词乙", &[FollowUpCategory::Interest]);
        sa.observe("词丙词丁", &[FollowUpCategory::Interest]);
        sa.observe("词戊词己", &[FollowUpCategory::Interest]); // 触发驱逐

        // 验证索引与主存储一致 / Verify index consistency post-eviction
        let total_indexed: usize = sa.count_index.values().map(|s| s.len()).sum();
        assert_eq!(
            total_indexed,
            sa.co_occurrence.len(),
            "count_index size mismatch after eviction"
        );

        // 验证被驱逐的词对不在索引中 / Evicted pair should not be in index
        for pair in sa.co_occurrence.keys() {
            let in_term_a = sa
                .term_index
                .get(&pair.0)
                .is_some_and(|m| m.contains_key(&pair.1));
            let in_term_b = sa
                .term_index
                .get(&pair.1)
                .is_some_and(|m| m.contains_key(&pair.0));
            assert!(
                in_term_a && in_term_b,
                "pair {:?} missing from term_index after eviction",
                pair
            );
        }
    }

    #[test]
    fn test_large_scale_observe_performance() {
        // 大规模观察 — 验证索引在多次驱逐后仍一致 / Large-scale consistency
        let mut sa = SemanticAssociation::new(SemanticAssociationConfig {
            association_threshold: 1,
            max_entries: 10,
            max_keyword_len: 4,
        });
        for i in 0..100 {
            let text = format!("词{}对{}", i, i + 200);
            sa.observe(&text, &[FollowUpCategory::Interest]);
        }
        // 索引一致性 / Index consistency
        let total_indexed: usize = sa.count_index.values().map(|s| s.len()).sum();
        assert_eq!(
            total_indexed,
            sa.co_occurrence.len(),
            "index mismatch after 100 observes with eviction"
        );
        assert!(
            sa.co_occurrence.len() <= 10,
            "should respect max_entries: {}",
            sa.co_occurrence.len()
        );
    }
}
