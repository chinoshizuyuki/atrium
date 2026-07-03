// SPDX-License-Identifier: MIT

//! 语义关联发现 — Semantic association discovery: finds co-occurrence patterns
//! between terms to enable cross-topic curiosity linking.
//!
//! 核心理念：关键词提取太机械。数字生命应该能发现"图书馆"和"考研"之间的
//! 关联——通过共现频率和类别传播。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        }
    }

    /// 观察文本 — 提取关键词并更新共现计数
    /// Observe text — Extract keywords and update co-occurrence.
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
                let count = self.co_occurrence.entry(pair).or_insert(0);
                *count += 1;

                // 限制 HashMap 大小 / Limit HashMap size
                if self.co_occurrence.len() > self.config.max_entries {
                    // 移除计数最低的条目 / Remove lowest-count entry
                    if let Some(min_key) = self
                        .co_occurrence
                        .iter()
                        .min_by_key(|(_, v)| *v)
                        .map(|(k, _)| k.clone())
                    {
                        self.co_occurrence.remove(&min_key);
                    }
                }
            }
        }
    }

    /// 发现关联事项 — 给定文本，找出关联的已知事项
    /// Find related items — Given text, find associated known items.
    pub fn find_related(&self, text: &str) -> Vec<AssociationLink> {
        let keywords = self.extract_keywords(text);
        let mut links = Vec::new();

        for kw in &keywords {
            for (pair, count) in &self.co_occurrence {
                if *count < self.config.association_threshold {
                    continue;
                }
                let other = if &pair.0 == kw {
                    Some(&pair.1)
                } else if &pair.1 == kw {
                    Some(&pair.0)
                } else {
                    None
                };
                if let Some(term) = other {
                    let strength = self.association_strength(kw, term);
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
        let sa2: SemanticAssociation = bincode::deserialize(&bytes).unwrap();
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
}
