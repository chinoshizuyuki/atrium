// SPDX-License-Identifier: MIT
//! MemoryConsolidator — 记忆巩固机制
//!
//! 参考人类睡眠时海马体→新皮层的记忆转移机制，在用户不活跃时执行"巩固"：
//!
//! 1. **相似合并**：文本 Jaccard 相似度 ≥ 阈值的同主语事实 → 合并为一条
//! 2. **低频压缩**：age > N 天 + verify_count ≤ 1 → 降低置信度（模拟遗忘）
//! 3. **矛盾废弃**：与新事实矛盾的旧事实 → 删除
//!
//! 频率控制：最多每 24 小时执行一次，每次最多处理 N 条事实。
//! MemoryConsolidator — Memory consolidation pipeline.
//!
//! Inspired by the hippocampal-neocortical memory transfer mechanism during sleep,
//! executes "consolidation" when the user is less active.
//! 1. **Semantic merge**: text Jaccard similarity > threshold + same entity → merge
//! 2. **Confidence compression**: age > N days + verify_count == 1 → boost confidence (forgetting curve)
//! 3. **Contradiction removal**: mutually contradictory facts → delete both

use crate::fact_store::{Fact, FactStore};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

// ════════════════════════════════════════════════════════════════════
// 配置与结果
// ════════════════════════════════════════════════════════════════════

/// 巩固配置（由 CoreService 传入，来自 ConsolidationCfg）
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    pub enabled: bool,
    pub max_facts_per_run: usize,
    pub min_interval_hours: u64,
    pub similarity_threshold: f64,
    pub low_access_age_days: u64,
}

impl ConsolidationConfig {
    /// 从核心配置值构建（由 CoreService 调用）
    pub fn new(
        enabled: bool,
        max_facts_per_run: usize,
        min_interval_hours: u64,
        similarity_threshold: f64,
        low_access_age_days: u64,
    ) -> Self {
        Self {
            enabled,
            max_facts_per_run,
            min_interval_hours,
            similarity_threshold,
            low_access_age_days,
        }
    }
}

/// 单次巩固运行的结果统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub merged_pairs: usize,
    pub compressed_count: usize,
    pub deprecated_count: usize,
    pub facts_before: usize,
    pub facts_after: usize,
    pub timestamp: u64,
}

// ════════════════════════════════════════════════════════════════════
// MemoryConsolidator
// ════════════════════════════════════════════════════════════════════

pub struct MemoryConsolidator {
    config: ConsolidationConfig,
    last_run_at: Option<u64>,
    total_runs: u64,
    total_merged: u64,
    total_compressed: u64,
    total_deprecated: u64,
}

impl MemoryConsolidator {
    pub fn new(config: ConsolidationConfig) -> Self {
        Self {
            config,
            last_run_at: None,
            total_runs: 0,
            total_merged: 0,
            total_compressed: 0,
            total_deprecated: 0,
        }
    }

    /// 判断是否应该执行巩固
    ///
    /// 条件：
    /// 1. 已启用
    /// 2. 距离上次运行超过 min_interval_hours
    /// 3. 用户已不活跃超过 trigger_inactive_hours（由调用方判断）
    pub fn should_run(&self, inactive_seconds: u64, trigger_inactive_hours: u64) -> bool {
        if !self.config.enabled {
            return false;
        }

        // 用户不活跃时间不足
        if inactive_seconds < trigger_inactive_hours * 3600 {
            return false;
        }

        // 冷却时间检查
        if let Some(last) = self.last_run_at {
            let now = now_secs();
            let elapsed_hours = (now.saturating_sub(last)) / 3600;
            if elapsed_hours < self.config.min_interval_hours {
                return false;
            }
        }

        true
    }

    /// 执行一次完整的巩固运行
    pub fn run(&mut self, store: &FactStore) -> ConsolidationResult {
        let now = now_secs();
        let all_facts = store.all_facts();

        let mut result = ConsolidationResult {
            facts_before: all_facts.len(),
            timestamp: now,
            ..Default::default()
        };

        // 限制处理数量
        let facts: Vec<Fact> = all_facts
            .into_iter()
            .take(self.config.max_facts_per_run)
            .collect();

        // ── 阶段 1: 矛盾废弃 ──
        let deprecated = self.deprecate_contradictions(store, &facts);
        result.deprecated_count = deprecated;

        // ── 阶段 2: 相似合并 ──
        let merged = self.merge_similar(store, &facts);
        result.merged_pairs = merged;

        // ── 阶段 3: 低频压缩 ──
        let compressed = self.compress_low_access(store, now);
        result.compressed_count = compressed;

        result.facts_after = store.count();

        // 更新统计
        self.last_run_at = Some(now);
        self.total_runs += 1;
        self.total_merged += merged as u64;
        self.total_compressed += compressed as u64;
        self.total_deprecated += deprecated as u64;

        tracing::info!(
            "记忆巩固完成: 合并={} 压缩={} 废弃={} 事实 {} → {}",
            merged,
            compressed,
            deprecated,
            result.facts_before,
            result.facts_after,
        );

        result
    }

    // ════════════════════════════════════════════════════════════════
    // 阶段 1: 矛盾废弃
    // ════════════════════════════════════════════════════════════════

    /// 检测矛盾事实：同一主语 + 矛盾谓语 → 保留较新的事实，删除旧的
    fn deprecate_contradictions(&self, store: &FactStore, facts: &[Fact]) -> usize {
        let mut removed = HashSet::new();

        // 按主语分组
        let mut by_subject: HashMap<String, Vec<&Fact>> = HashMap::new();
        for f in facts {
            by_subject
                .entry(f.subject.to_lowercase())
                .or_default()
                .push(f);
        }

        for group in by_subject.values() {
            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    let a = group[i];
                    let b = group[j];

                    // 跳过已被标记删除的
                    let a_key = a.canonical_form();
                    let b_key = b.canonical_form();
                    if removed.contains(&a_key) || removed.contains(&b_key) {
                        continue;
                    }

                    if is_contradictory(&a.predicate, &b.predicate) {
                        // 保留较新的事实（verified_at 更大），删除旧的
                        let older = if a.verified_at <= b.verified_at { a } else { b };
                        let key = older.canonical_form();
                        if store.remove(&key) {
                            removed.insert(key);
                        }
                    }
                }
            }
        }

        removed.len()
    }

    // ════════════════════════════════════════════════════════════════
    // 阶段 2: 相似合并
    // ════════════════════════════════════════════════════════════════

    /// 合并文本高度相似的同主语事实
    fn merge_similar(&self, store: &FactStore, facts: &[Fact]) -> usize {
        let threshold = self.config.similarity_threshold;
        let mut merged = 0;
        let mut already_merged = HashSet::new();

        // 按主语分组
        let mut by_subject: HashMap<String, Vec<&Fact>> = HashMap::new();
        for f in facts {
            by_subject
                .entry(f.subject.to_lowercase())
                .or_default()
                .push(f);
        }

        for group in by_subject.values() {
            for i in 0..group.len() {
                let a_key = group[i].canonical_form();
                if already_merged.contains(&a_key) {
                    continue;
                }

                for j in (i + 1)..group.len() {
                    let b_key = group[j].canonical_form();
                    if already_merged.contains(&b_key) {
                        continue;
                    }

                    let sim = text_similarity(
                        &format!("{} {}", group[i].predicate, group[i].object),
                        &format!("{} {}", group[j].predicate, group[j].object),
                    );

                    if sim >= threshold {
                        // 合并：保留置信度更高的，删除另一条
                        let (keeper, loser) = if group[i].confidence >= group[j].confidence {
                            (group[i], group[j])
                        } else {
                            (group[j], group[i])
                        };

                        // 将 loser 的 verify_count 合并到 keeper
                        let loser_key = loser.canonical_form();
                        let mut merged_fact = keeper.clone();
                        merged_fact.merge_confidence(loser.confidence);

                        // 保留更强的情感上下文
                        if loser.emotion_context.is_some() && merged_fact.emotion_context.is_none()
                        {
                            merged_fact.emotion_context = loser.emotion_context.clone();
                        }

                        // 删除 loser，重新插入合并后的 keeper
                        store.remove(&loser_key);
                        let _ = store.insert(merged_fact);
                        already_merged.insert(loser_key);
                        merged += 1;
                    }
                }
            }
        }

        merged
    }

    // ════════════════════════════════════════════════════════════════
    // 阶段 3: 低频压缩
    // ════════════════════════════════════════════════════════════════

    /// 压缩长期未被再次验证的低置信度事实
    ///
    /// 条件：age > low_access_age_days 且 verify_count ≤ 1 且 confidence < 0.4
    /// 动作：降低置信度（模拟自然遗忘），不直接删除
    fn compress_low_access(&self, store: &FactStore, now: u64) -> usize {
        let age_threshold_secs = self.config.low_access_age_days * 86400;
        let all_facts = store.all_facts();
        let mut compressed = 0;

        for fact in &all_facts {
            let age = now.saturating_sub(fact.created_at);
            if age < age_threshold_secs {
                continue;
            }
            if fact.verify_count > 1 {
                continue;
            }
            if fact.confidence >= 0.4 {
                continue;
            }

            // 降低置信度（乘以 0.5 衰减因子）
            let new_confidence = fact.confidence * 0.5;
            let key = fact.canonical_form();

            // 删除旧事实，插入衰减后的版本
            store.remove(&key);
            let mut compressed_fact = fact.clone();
            compressed_fact.confidence = new_confidence;
            let _ = store.insert(compressed_fact);
            compressed += 1;
        }

        compressed
    }

    /// 获取巩固统计摘要
    pub fn health_status(&self) -> String {
        format!(
            "runs={} merged={} compressed={} deprecated={} last_run={}",
            self.total_runs,
            self.total_merged,
            self.total_compressed,
            self.total_deprecated,
            self.last_run_at
                .map(|t| t.to_string())
                .unwrap_or_else(|| "never".into()),
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数
// ════════════════════════════════════════════════════════════════════

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 基于字符 bigram 的 Jaccard 相似度（0.0~1.0）
///
/// 轻量级文本相似度，不依赖 embedding 模型。
/// 对中文和英文都有效。
fn text_similarity(a: &str, b: &str) -> f64 {
    let bigrams_a = char_bigrams(a);
    let bigrams_b = char_bigrams(b);

    if bigrams_a.is_empty() && bigrams_b.is_empty() {
        return 1.0;
    }
    if bigrams_a.is_empty() || bigrams_b.is_empty() {
        return 0.0;
    }

    let intersection: HashSet<_> = bigrams_a.intersection(&bigrams_b).collect();
    let union: HashSet<_> = bigrams_a.union(&bigrams_b).collect();

    intersection.len() as f64 / union.len() as f64
}

/// 提取字符串的字符 bigram 集合
fn char_bigrams(s: &str) -> HashSet<String> {
    let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut set = HashSet::new();
    for w in chars.windows(2) {
        set.insert(format!("{}{}", w[0], w[1]));
    }
    // 单字符文本退化为 unigram
    if chars.len() == 1 {
        set.insert(chars[0].to_string());
    }
    set
}

/// 检测谓语是否矛盾（简化版，复用 evidence 模块的逻辑）
fn is_contradictory(pred_a: &str, pred_b: &str) -> bool {
    let a = pred_a.to_lowercase();
    let b = pred_b.to_lowercase();

    // 直接矛盾对
    let contradiction_pairs: &[(&str, &str)] = &[
        ("喜欢", "讨厌"),
        ("讨厌", "喜欢"),
        ("不喜欢", "喜欢"),
        ("喜欢", "不喜欢"),
        ("爱", "恨"),
        ("恨", "爱"),
        ("是", "不是"),
        ("不是", "是"),
        ("有", "没有"),
        ("没有", "有"),
        ("likes", "dislikes"),
        ("dislikes", "likes"),
        ("is", "is not"),
        ("is not", "is"),
    ];

    for &(x, y) in contradiction_pairs {
        if a == x && b == y {
            return true;
        }
    }
    false
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ConsolidationConfig {
        ConsolidationConfig {
            enabled: true,
            max_facts_per_run: 100,
            min_interval_hours: 24,
            similarity_threshold: 0.7,
            low_access_age_days: 90,
        }
    }

    fn old_fact(subject: &str, predicate: &str, object: &str, days_old: u64) -> Fact {
        let mut f = Fact::new(subject, predicate, object);
        f.created_at = now_secs() - days_old * 86400;
        f.verified_at = f.created_at;
        f
    }

    #[test]
    fn test_text_similarity_identical() {
        let sim = text_similarity("喜欢编程", "喜欢编程");
        assert!(sim > 0.99, "相同文本应相似度 ≈ 1.0, got {}", sim);
    }

    #[test]
    fn test_text_similarity_different() {
        let sim = text_similarity("喜欢编程", "讨厌做饭");
        assert!(sim < 0.3, "完全不同的文本应低相似度, got {}", sim);
    }

    #[test]
    fn test_contradiction_detection() {
        assert!(is_contradictory("喜欢", "讨厌"));
        assert!(is_contradictory("讨厌", "喜欢"));
        assert!(is_contradictory("是", "不是"));
        assert!(!is_contradictory("喜欢", "喜欢"));
        assert!(!is_contradictory("喜欢", "学习"));
    }

    #[test]
    fn test_merge_similar_facts() {
        let store = FactStore::new_in_memory().unwrap();
        store
            .insert(Fact::new("主人", "喜欢", "Rust 编程语言").with_confidence(0.9))
            .unwrap();
        store
            .insert(Fact::new("主人", "喜欢", "Rust 编程").with_confidence(0.7))
            .unwrap();
        store
            .insert(Fact::new("主人", "讨厌", "Java").with_confidence(0.8))
            .unwrap();

        let mut consolidator = MemoryConsolidator::new(ConsolidationConfig {
            similarity_threshold: 0.5,
            ..default_config()
        });

        let result = consolidator.run(&store);
        assert!(result.merged_pairs >= 1, "应至少合并 1 对相似事实");
        // 合并后事实数应减少
        assert!(
            result.facts_after < result.facts_before,
            "合并后事实数应减少: {} → {}",
            result.facts_before,
            result.facts_after
        );
    }

    #[test]
    fn test_deprecate_contradictions() {
        let store = FactStore::new_in_memory().unwrap();
        // 旧事实：喜欢 Java
        let old = old_fact("主人", "喜欢", "Java", 30);
        store.insert(old).unwrap();
        // 新事实：讨厌 Java（矛盾）
        store
            .insert(Fact::new("主人", "讨厌", "Java").with_confidence(0.9))
            .unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);
        assert!(result.deprecated_count >= 1, "应至少废弃 1 条矛盾事实");
    }

    #[test]
    fn test_compress_low_access() {
        let store = FactStore::new_in_memory().unwrap();
        // 91 天前创建、只验证 1 次、低置信度
        let old_low = old_fact("主人", "提到", "某个不重要的事", 91);
        let mut f = old_low;
        f.confidence = 0.3;
        f.verify_count = 1;
        store.insert(f).unwrap();

        // 近期事实不应被压缩
        store
            .insert(Fact::new("主人", "喜欢", "Rust").with_confidence(0.9))
            .unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);
        assert!(result.compressed_count >= 1, "应至少压缩 1 条低频事实");
    }

    #[test]
    fn test_should_run_cooldown() {
        let mut consolidator = MemoryConsolidator::new(ConsolidationConfig {
            min_interval_hours: 24,
            ..default_config()
        });

        // 首次：无冷却，不活跃 7 小时 → 应该运行
        assert!(consolidator.should_run(7 * 3600, 6));

        // 模拟运行后
        consolidator.last_run_at = Some(now_secs());

        // 刚运行过 → 不应该运行
        assert!(!consolidator.should_run(7 * 3600, 6));
    }

    #[test]
    fn test_should_run_not_inactive_enough() {
        let consolidator = MemoryConsolidator::new(default_config());
        // 只不活跃 2 小时，阈值 6 小时 → 不应该运行
        assert!(!consolidator.should_run(2 * 3600, 6));
    }

    #[test]
    fn test_empty_store_no_panic() {
        let store = FactStore::new_in_memory().unwrap();
        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);
        assert_eq!(result.merged_pairs, 0);
        assert_eq!(result.compressed_count, 0);
        assert_eq!(result.deprecated_count, 0);
        assert_eq!(result.facts_before, 0);
        assert_eq!(result.facts_after, 0);
    }

    #[test]
    fn test_health_status() {
        let consolidator = MemoryConsolidator::new(default_config());
        let status = consolidator.health_status();
        assert!(status.contains("runs=0"));
        assert!(status.contains("last_run=never"));
    }

    #[test]
    fn test_max_facts_per_run_limit() {
        let store = FactStore::new_in_memory().unwrap();
        // 插入 200 条事实
        for i in 0..200 {
            store
                .insert(Fact::new("主人", "提到", &format!("事物{}", i)))
                .unwrap();
        }

        let mut consolidator = MemoryConsolidator::new(ConsolidationConfig {
            max_facts_per_run: 50,
            ..default_config()
        });

        let result = consolidator.run(&store);
        // 虽然 200 条事实，但只处理了 50 条
        // facts_before 应反映总事实数（200），但合并/压缩只在前 50 条上进行
        assert_eq!(result.facts_before, 200);
    }
}
