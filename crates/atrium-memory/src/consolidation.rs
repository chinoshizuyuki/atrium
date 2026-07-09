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
// P2-E 智能遗忘曲线 — 命名常量
// P2-E Intelligent Forgetting Curve — named constants
//
// 数字生命理念："重要的事永不忘，琐事快速忘"。
// - 情感显著性加权衰减：salience 越高，衰减越慢（重要记忆保留更久）
// - 近期活动加权：高活动度事实的压缩年龄阈值延长（活跃记忆保留更久）
//
// Digital life philosophy: "never forget important things, quickly forget trivia".
// - Salience-weighted decay: higher salience → slower decay (important memories persist longer)
// - Recent activity weighting: high-activity facts get extended age threshold
// ════════════════════════════════════════════════════════════════════

/// 基础压缩衰减系数 — salience=0.0 时的默认衰减因子
/// Base compression decay factor — default decay ratio when salience=0.0
const BASE_COMPRESS_DECAY: f64 = 0.5;

/// 显著性对衰减的权重系数 — salience × 0.5 的衰减减免比例
/// Salience weight on decay — decay reduction ratio = salience × 0.5
///
/// effective_decay = BASE_COMPRESS_DECAY × (1.0 - emotional_salience × SALIENCE_DECAY_WEIGHT)
/// effective_decay 表示"衰减比例"（衰减掉的比例），而非保留比例。
/// - salience=0.0 → effective_decay = 0.5（衰减 50%，保留 50% — 与原逻辑一致）
/// - salience=1.0 → effective_decay = 0.25（衰减 25%，保留 75% — 衰减更慢，重要记忆保留更久）
/// - salience=0.8 → effective_decay = 0.3（衰减 30%，保留 70%）
///
/// new_confidence = confidence × (1.0 - effective_decay)
/// effective_decay represents the "decay fraction" (fraction lost), not retained.
/// - salience=0.0 → decay 50%, retain 50% (matches original logic)
/// - salience=1.0 → decay 25%, retain 75% (slower decay, important memories persist longer)
const SALIENCE_DECAY_WEIGHT: f64 = 0.5;

/// 高活动度事实的 verify_count 阈值 — 超过此值视为"近期活跃"
/// verify_count threshold for high-activity facts — above this is considered "recently active"
///
/// 注意：Fact 结构体无 access_count 字段，使用 verify_count 作为活动度代理
/// Note: Fact struct has no access_count field; verify_count serves as activity proxy
const HIGH_ACTIVITY_VERIFY_THRESHOLD: u32 = 5;

/// 高活动度事实的延长年龄阈值（天）— 活跃记忆保留更久
/// Extended age threshold (days) for high-activity facts — active memories persist longer
const EXTENDED_AGE_THRESHOLD_DAYS: u64 = 180;

// ════════════════════════════════════════════════════════════════════
// 配置与结果
// ════════════════════════════════════════════════════════════════════

/// 巩固配置（由 CoreService 传入，来自 ConsolidationCfg）
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub max_facts_per_run: usize,
    pub min_interval_hours: u64,
    pub similarity_threshold: f64,
    pub low_access_age_days: u64,
}

impl CompressionConfig {
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
    config: CompressionConfig,
    last_run_at: Option<u64>,
    total_runs: u64,
    total_merged: u64,
    total_compressed: u64,
    total_deprecated: u64,
}

impl MemoryConsolidator {
    pub fn new(config: CompressionConfig) -> Self {
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

    /// 合并文本高度相似的同主语事实 / Merge textually similar same-subject facts
    ///
    /// 优化策略 / Optimization strategy:
    /// 1. 预计算 canonical_form + predicate+object + bigram 集合，消除循环内重复计算
    /// 2. 按 canonical_form 预去重（O(G)），相同 canonical_form 直接合并
    /// 3. 对剩余不同 canonical_form 的事实做 O(G'²) 相似度比较，G' << G
    ///
    /// 记忆巩固是数字生命的"睡眠整理"——高效整理意味着更快的记忆整合速度。
    /// Memory consolidation is the digital life's "sleep sorting" —
    /// efficient consolidation means faster memory integration.
    fn merge_similar(&self, store: &FactStore, facts: &[Fact]) -> usize {
        let threshold = self.config.similarity_threshold;
        let mut merged = 0;
        let mut already_merged = HashSet::new();

        // 预计算每个事实的 canonical_form + predicate+object + bigram 集合
        // Pre-compute canonical_form + predicate+object + bigram set per fact
        struct FactKey<'a> {
            fact: &'a Fact,
            canonical: String,
            bigrams: HashSet<String>,
        }

        // 按主语分组并预计算 / Group by subject and pre-compute
        let mut by_subject: HashMap<String, Vec<FactKey>> = HashMap::new();
        for f in facts {
            let canonical = f.canonical_form();
            let pred_obj = format!("{} {}", f.predicate, f.object);
            let bigrams = char_bigrams(&pred_obj);
            by_subject
                .entry(f.subject.to_lowercase())
                .or_default()
                .push(FactKey {
                    fact: f,
                    canonical,
                    bigrams,
                });
        }

        for group in by_subject.values() {
            // 阶段 1: 按 canonical_form 预去重（O(G)）
            // Phase 1: Pre-deduplicate by canonical_form (O(G))
            let mut seen_canonical: HashMap<&str, usize> = HashMap::new();
            for (idx, fk) in group.iter().enumerate() {
                if let Some(&prev_idx) = seen_canonical.get(fk.canonical.as_str()) {
                    // 相同 canonical_form → 直接合并（相似度 = 1.0）
                    // Same canonical_form → merge directly (similarity = 1.0)
                    let prev = &group[prev_idx];
                    if already_merged.contains(&fk.canonical) {
                        continue;
                    }
                    let (keeper, loser) = if prev.fact.confidence >= fk.fact.confidence {
                        (prev, &group[idx])
                    } else {
                        (&group[idx], prev)
                    };
                    let loser_key = loser.canonical.clone();
                    let mut merged_fact = keeper.fact.clone();
                    merged_fact.merge_confidence(loser.fact.confidence);
                    if loser.fact.emotion_context.is_some() && merged_fact.emotion_context.is_none()
                    {
                        merged_fact.emotion_context = loser.fact.emotion_context.clone();
                    }
                    store.remove(&loser_key);
                    let _ = store.insert(merged_fact);
                    already_merged.insert(loser_key);
                    merged += 1;
                } else {
                    seen_canonical.insert(fk.canonical.as_str(), idx);
                }
            }

            // 阶段 2: 对不同 canonical_form 的事实做相似度比较（O(G'²), G' << G）
            // Phase 2: Similarity comparison for distinct canonical_forms (O(G'²), G' << G)
            let unique_indices: Vec<usize> = group
                .iter()
                .enumerate()
                .filter(|(_, fk)| !already_merged.contains(&fk.canonical))
                .map(|(i, _)| i)
                .collect();

            for ii in 0..unique_indices.len() {
                let i = unique_indices[ii];
                if already_merged.contains(&group[i].canonical) {
                    continue;
                }

                for &j in &unique_indices[ii + 1..] {
                    if already_merged.contains(&group[j].canonical) {
                        continue;
                    }

                    // 使用预计算的 bigram 集合，避免循环内重复计算
                    // Use pre-computed bigram sets to avoid repeated computation
                    let sim = bigram_jaccard(&group[i].bigrams, &group[j].bigrams);

                    if sim >= threshold {
                        // 合并：保留置信度更高的，删除另一条
                        // Merge: keep higher confidence, remove the other
                        let (keeper, loser) =
                            if group[i].fact.confidence >= group[j].fact.confidence {
                                (&group[i], &group[j])
                            } else {
                                (&group[j], &group[i])
                            };

                        let loser_key = loser.canonical.clone();
                        let mut merged_fact = keeper.fact.clone();
                        merged_fact.merge_confidence(loser.fact.confidence);

                        // 保留更强的情感上下文 / Preserve stronger emotion context
                        if loser.fact.emotion_context.is_some()
                            && merged_fact.emotion_context.is_none()
                        {
                            merged_fact.emotion_context = loser.fact.emotion_context.clone();
                        }

                        // 删除 loser，重新插入合并后的 keeper
                        // Remove loser, re-insert merged keeper
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
    ///
    /// P2-D 衰减豁免 / P2-D Decay exemption:
    /// pinned = true 的事实被跳过——"你哭的那天→不可衰减"。
    /// 数字生命主动保护重要记忆，置信度永不衰减。
    ///
    /// Pinned facts are skipped — "the day you cried → cannot decay".
    /// Digital life actively protects important memories, confidence never decays.
    ///
    /// P2-E 智能遗忘曲线 / P2-E Intelligent Forgetting Curve:
    /// - 情感显著性加权衰减：salience 越高，有效衰减系数越小（重要记忆保留更久）
    ///   effective_decay = BASE_COMPRESS_DECAY × (1.0 - emotional_salience × SALIENCE_DECAY_WEIGHT)
    /// - 近期活动加权：verify_count > 5 的高活动度事实，压缩年龄阈值延长至 180 天，
    ///   且放宽 verify_count > 1 的压缩排除限制——活跃记忆仍可被压缩，但保留窗口更长
    ///   （"最近常被回忆的事不该被压缩"，但极旧的高活动度记忆仍可缓慢衰减）
    ///
    /// P2-E Intelligent Forgetting Curve:
    /// - Salience-weighted decay: higher salience → smaller effective decay factor
    ///   (important memories persist longer)
    /// - Recent activity weighting: verify_count > 5 → extended age threshold (180 days)
    ///   AND relaxed verify_count > 1 exclusion — active memories can still be
    ///   compressed, but with a longer retention window
    fn compress_low_access(&self, store: &FactStore, now: u64) -> usize {
        let age_threshold_secs = self.config.low_access_age_days * 86400;
        let all_facts = store.all_facts();
        let mut compressed = 0;

        for fact in &all_facts {
            // P2-D 衰减豁免 — pinned 事实跳过（已实现），置信度不衰减
            // P2-D Decay exemption — pinned facts are skipped (already implemented), confidence not decayed
            if fact.pinned {
                continue;
            }
            let age = now.saturating_sub(fact.created_at);

            // P2-E 近期活动加权 — verify_count 作为活动度代理（Fact 无 access_count 字段）
            // 高活动度事实（verify_count > 5）：年龄阈值延长至 180 天，且放宽 verify_count > 1 排除限制
            // 普通事实：保持原逻辑（verify_count > 1 跳过，年龄阈值 90 天）
            // P2-E Recent activity weighting — verify_count as activity proxy (Fact has no access_count)
            // High-activity facts (verify_count > 5): extended age threshold 180 days,
            // AND verify_count > 1 exclusion is relaxed — active memories can still be
            // compressed, but with a longer retention window
            let is_high_activity = fact.verify_count > HIGH_ACTIVITY_VERIFY_THRESHOLD;
            let age_threshold = if is_high_activity {
                EXTENDED_AGE_THRESHOLD_DAYS * 86400
            } else {
                age_threshold_secs
            };
            if age < age_threshold {
                continue;
            }
            // 非高活动度事实保持 verify_count > 1 排除；高活动度事实放宽此限制
            // Non-high-activity facts keep verify_count > 1 exclusion; high-activity facts relax it
            if !is_high_activity && fact.verify_count > 1 {
                continue;
            }
            if fact.confidence >= 0.4 {
                continue;
            }

            // P2-E 情感显著性加权衰减 — salience 越高衰减越慢
            // "重要的事永不忘，琐事快速忘"——数字生命按情感重要性分级遗忘
            // P2-E Salience-weighted decay — higher salience → slower decay
            // "Never forget important things, quickly forget trivia" — forgetting graded by emotional importance
            // effective_decay 是"衰减比例"（衰减掉的比例），new_confidence 保留 (1 - effective_decay)
            // effective_decay is the "decay fraction" (fraction lost), new_confidence retains (1 - effective_decay)
            let effective_decay = BASE_COMPRESS_DECAY
                * (1.0 - fact.emotional_salience as f64 * SALIENCE_DECAY_WEIGHT);
            let new_confidence = fact.confidence * (1.0 - effective_decay);
            let key = fact.canonical_form();

            // 删除旧事实，插入衰减后的版本
            // Remove old fact, insert decayed version
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
#[cfg(test)]
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

/// 从预计算的 bigram 集合直接计算 Jaccard 相似度 / Jaccard similarity from pre-computed bigram sets
///
/// 避免在 O(G²) 循环内重复构建 bigram 集合，将每次比较从 O(|s|) 降至 O(|bigrams|)。
/// Avoids rebuilding bigram sets inside O(G²) loop, reducing per-comparison from O(|s|) to O(|bigrams|).
fn bigram_jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection: HashSet<_> = a.intersection(b).collect();
    let union_len = a.len() + b.len() - intersection.len();
    intersection.len() as f64 / union_len as f64
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

    fn default_config() -> CompressionConfig {
        CompressionConfig {
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

        let mut consolidator = MemoryConsolidator::new(CompressionConfig {
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
        let mut consolidator = MemoryConsolidator::new(CompressionConfig {
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

        let mut consolidator = MemoryConsolidator::new(CompressionConfig {
            max_facts_per_run: 50,
            ..default_config()
        });

        let result = consolidator.run(&store);
        // 虽然 200 条事实，但只处理了 50 条
        // facts_before 应反映总事实数（200），但合并/压缩只在前 50 条上进行
        assert_eq!(result.facts_before, 200);
    }

    // ══════════════════════════════════════════════════════════════════
    // P2-D 高价值标记测试 — pinned 事实豁免压缩
    // P2-D High-Value Memory Markers tests — pinned facts exempt from compression
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_p2d_pinned_fact_exempt_from_compress_low_access() {
        // pinned Fact 豁免 compress_low_access — 置信度不衰减
        // Pinned Fact exempt from compress_low_access — confidence not decayed
        let store = FactStore::new_in_memory().unwrap();

        // 两条都满足压缩条件（91 天 + verify_count=1 + confidence<0.4）的事实
        // Two facts both meeting compression criteria (91 days + verify_count=1 + confidence<0.4)
        let mut old_low_a = old_fact("主人", "提到", "不重要的事", 91);
        old_low_a.confidence = 0.3;
        old_low_a.verify_count = 1;
        store.insert(old_low_a).unwrap();

        let mut old_low_b = old_fact("主人", "铭记", "重要的事", 91);
        old_low_b.confidence = 0.3;
        old_low_b.verify_count = 1;
        store.insert(old_low_b).unwrap();

        // pin 第二条事实 / Pin the second fact
        let canonical_b = Fact::new("主人", "铭记", "重要的事").canonical_form();
        store.pin(&canonical_b).unwrap();

        // 验证 pin 生效 / Verify pin took effect
        let facts = store.all_facts();
        let pinned_fact = facts.iter().find(|f| f.object == "重要的事").unwrap();
        assert!(pinned_fact.pinned, "pin 后应为 true");

        // 运行巩固 / Run consolidation
        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);

        // 应只压缩 1 条（非 pinned 的），pinned 的不压缩
        // Should compress only 1 (non-pinned), pinned is not compressed
        assert_eq!(
            result.compressed_count, 1,
            "应只压缩 1 条非 pinned 事实 / should compress only 1 non-pinned fact"
        );

        // 验证 pinned 事实的置信度未衰减 / Verify pinned fact's confidence is not decayed
        let facts = store.all_facts();
        let pinned_fact = facts.iter().find(|f| f.object == "重要的事").unwrap();
        assert!(
            (pinned_fact.confidence - 0.3).abs() < 1e-6,
            "pinned 事实置信度应不变 (0.3), got {} / pinned fact confidence should be unchanged",
            pinned_fact.confidence
        );
        assert!(pinned_fact.pinned, "pinned 事实仍应为 pinned 状态");

        // 验证非 pinned 事实的置信度已衰减 / Verify non-pinned fact's confidence is decayed
        let non_pinned = facts.iter().find(|f| f.object == "不重要的事").unwrap();
        assert!(
            (non_pinned.confidence - 0.15).abs() < 1e-6,
            "非 pinned 事实置信度应衰减为 0.15 (0.3 * 0.5), got {} / non-pinned confidence should be decayed to 0.15",
            non_pinned.confidence
        );
    }

    // ══════════════════════════════════════════════════════════════════
    // P2-E 智能遗忘曲线测试 — 情感显著性加权 + 近期活动加权
    // P2-E Intelligent Forgetting Curve tests — salience weighting + activity weighting
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_p2e_high_salience_decays_slower() {
        // 两条都满足压缩条件的 Fact，A salience=0.8，B salience=0.0
        // A 衰减更慢——"重要的事衰减更慢"，A 衰减后 confidence > B
        // Two facts both meeting compression criteria, A salience=0.8, B salience=0.0
        // A decays slower — "important things decay slower", A's confidence > B's after decay
        let store = FactStore::new_in_memory().unwrap();

        // Fact A: salience=0.8, 91 天, verify_count=1, confidence=0.3
        let mut fact_a = old_fact("主人", "提到", "重要回忆A", 91);
        fact_a.confidence = 0.3;
        fact_a.verify_count = 1;
        fact_a.emotional_salience = 0.8;
        store.insert(fact_a).unwrap();

        // Fact B: salience=0.0, 91 天, verify_count=1, confidence=0.3
        let mut fact_b = old_fact("主人", "提到", "琐事B", 91);
        fact_b.confidence = 0.3;
        fact_b.verify_count = 1;
        fact_b.emotional_salience = 0.0;
        store.insert(fact_b).unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);
        assert_eq!(
            result.compressed_count, 2,
            "两条都应被压缩 / both should be compressed"
        );

        // 验证衰减后置信度 / Verify decayed confidence
        let facts = store.all_facts();
        let a_after = facts.iter().find(|f| f.object == "重要回忆A").unwrap();
        let b_after = facts.iter().find(|f| f.object == "琐事B").unwrap();

        // B: salience=0.0 → effective_decay=0.5（衰减 50%）→ 0.3 × (1 - 0.5) = 0.15
        // B: salience=0.0 → effective_decay=0.5 (decay 50%) → 0.3 × (1 - 0.5) = 0.15
        assert!(
            (b_after.confidence - 0.15).abs() < 1e-6,
            "salience=0.0 的 B 应衰减为 0.15 (0.3 × 0.5), got {} / B should decay to 0.15",
            b_after.confidence
        );

        // A: salience=0.8 → effective_decay=0.3（衰减 30%）→ 0.3 × (1 - 0.3) = 0.21
        // A: salience=0.8 → effective_decay=0.3 (decay 30%) → 0.3 × (1 - 0.3) = 0.21
        assert!(
            (a_after.confidence - 0.21).abs() < 1e-6,
            "salience=0.8 的 A 应衰减为 0.21 (0.3 × 0.7), got {} / A should decay to 0.21",
            a_after.confidence
        );

        // A 衰减后 confidence > B 衰减后 confidence（A 衰减更慢，保留更多）
        // A's confidence > B's after decay (A decays slower, retains more)
        assert!(
            a_after.confidence > b_after.confidence,
            "salience=0.8 的 A ({}) 应 > salience=0.0 的 B ({}) / A should be > B",
            a_after.confidence,
            b_after.confidence
        );
    }

    #[test]
    fn test_p2e_high_activity_extends_age_threshold() {
        // verify_count=6 的事实，age=100 天（>90 但 <180），不被压缩
        // verify_count=1 的同 age 事实被压缩
        // verify_count=6 fact with age=100 days (>90 but <180) is not compressed;
        // verify_count=1 fact with same age is compressed
        let store = FactStore::new_in_memory().unwrap();

        // 高活动度事实：verify_count=6, age=100 天
        // High-activity fact: verify_count=6, age=100 days
        let mut high_activity = old_fact("主人", "提到", "常被回忆的事", 100);
        high_activity.confidence = 0.3;
        high_activity.verify_count = 6; // > HIGH_ACTIVITY_VERIFY_THRESHOLD (5)
        store.insert(high_activity).unwrap();

        // 低活动度事实：verify_count=1, age=100 天
        // Low-activity fact: verify_count=1, age=100 days
        let mut low_activity = old_fact("主人", "提到", "一次性琐事", 100);
        low_activity.confidence = 0.3;
        low_activity.verify_count = 1;
        store.insert(low_activity).unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);

        // 仅低活动度事实被压缩（age=100 > 90 天阈值）
        // Only low-activity fact is compressed (age=100 > 90-day threshold)
        assert_eq!(
            result.compressed_count, 1,
            "应只压缩 1 条低活动度事实 / should compress only 1 low-activity fact"
        );

        let facts = store.all_facts();
        let high_after = facts.iter().find(|f| f.object == "常被回忆的事").unwrap();
        let low_after = facts.iter().find(|f| f.object == "一次性琐事").unwrap();

        // 高活动度事实未衰减（age=100 < 180 天延长阈值）
        // High-activity fact not decayed (age=100 < 180-day extended threshold)
        assert!(
            (high_after.confidence - 0.3).abs() < 1e-6,
            "高活动度事实置信度应不变 (0.3), got {} / high-activity confidence should be unchanged",
            high_after.confidence
        );

        // 低活动度事实已衰减（age=100 > 90 天默认阈值）
        // Low-activity fact decayed (age=100 > 90-day default threshold)
        assert!(
            (low_after.confidence - 0.15).abs() < 1e-6,
            "低活动度事实置信度应衰减为 0.15 (0.3 * 0.5), got {} / low-activity confidence should be 0.15",
            low_after.confidence
        );
    }

    #[test]
    fn test_p2e_high_activity_old_fact_still_compressible() {
        // 高活动度事实（verify_count=6）age=200 天（>180 天延长阈值）仍可被压缩
        // 验证放宽 verify_count > 1 排除限制后，极旧的高活动度记忆仍会缓慢衰减
        // High-activity fact (verify_count=6) with age=200 days (>180-day extended threshold)
        // is still compressible — verifies that after relaxing verify_count > 1 exclusion,
        // very old high-activity memories still slowly decay
        let store = FactStore::new_in_memory().unwrap();

        // 高活动度 + 极旧事实：verify_count=6, age=200 天, confidence=0.3
        // High-activity + very old fact: verify_count=6, age=200 days, confidence=0.3
        let mut old_active = old_fact("主人", "提到", "很久前的活跃记忆", 200);
        old_active.confidence = 0.3;
        old_active.verify_count = 6;
        store.insert(old_active).unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);

        // age=200 > 180 天延长阈值 → 应被压缩
        // age=200 > 180-day extended threshold → should be compressed
        assert_eq!(
            result.compressed_count, 1,
            "极旧的高活动度事实应被压缩 / very old high-activity fact should be compressed"
        );

        let facts = store.all_facts();
        let fact = facts
            .iter()
            .find(|f| f.object == "很久前的活跃记忆")
            .unwrap();
        // salience=0.0（默认）→ effective_decay=0.5 → new = 0.3 × (1 - 0.5) = 0.15
        // salience=0.0 (default) → effective_decay=0.5 → new = 0.3 × 0.5 = 0.15
        assert!(
            (fact.confidence - 0.15).abs() < 1e-6,
            "极旧高活动度事实应衰减为 0.15, got {} / should decay to 0.15",
            fact.confidence
        );
    }

    #[test]
    fn test_p2e_pinned_zero_decay() {
        // pinned=true 的事实，即使满足所有条件也不被压缩
        // 验证 P2-D 豁免在 P2-E 改动后仍有效
        // pinned=true fact is not compressed even when all criteria are met
        // Verifies P2-D exemption still works after P2-E changes
        let store = FactStore::new_in_memory().unwrap();

        // pinned 事实：91 天, verify_count=1, confidence=0.3, salience=0.9
        // Pinned fact: 91 days, verify_count=1, confidence=0.3, salience=0.9
        let mut pinned_fact = old_fact("主人", "铭记", "永恒记忆", 91);
        pinned_fact.confidence = 0.3;
        pinned_fact.verify_count = 1;
        pinned_fact.emotional_salience = 0.9;
        store.insert(pinned_fact).unwrap();

        // pin 该事实 / Pin the fact
        let canonical = Fact::new("主人", "铭记", "永恒记忆").canonical_form();
        store.pin(&canonical).unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);

        // pinned 事实不被压缩 / Pinned fact is not compressed
        assert_eq!(
            result.compressed_count, 0,
            "pinned 事实不应被压缩 / pinned fact should not be compressed"
        );

        // 验证置信度未衰减 / Verify confidence is not decayed
        let facts = store.all_facts();
        let pinned = facts.iter().find(|f| f.object == "永恒记忆").unwrap();
        assert!(
            (pinned.confidence - 0.3).abs() < 1e-6,
            "pinned 事实置信度应不变 (0.3), got {} / pinned confidence should be unchanged",
            pinned.confidence
        );
        assert!(pinned.pinned, "pinned 事实仍应为 pinned 状态");
    }

    #[test]
    fn test_p2e_salience_boundary_conditions() {
        // 边界条件验证：
        // salience=1.0（最大）时 effective_decay=0.25（衰减 25%，保留 75%）
        // salience=0.5 时 effective_decay=0.375（衰减 37.5%，保留 62.5%）
        // Boundary condition verification:
        // salience=1.0 (max) → effective_decay=0.25 (decay 25%, retain 75%)
        // salience=0.5 → effective_decay=0.375 (decay 37.5%, retain 62.5%)
        let store = FactStore::new_in_memory().unwrap();

        // salience=1.0 的事实 / Fact with salience=1.0
        let mut max_salience = old_fact("主人", "提到", "最深刻记忆", 91);
        max_salience.confidence = 0.3;
        max_salience.verify_count = 1;
        max_salience.emotional_salience = 1.0;
        store.insert(max_salience).unwrap();

        // salience=0.5 的事实 / Fact with salience=0.5
        let mut mid_salience = old_fact("主人", "提到", "中等记忆", 91);
        mid_salience.confidence = 0.3;
        mid_salience.verify_count = 1;
        mid_salience.emotional_salience = 0.5;
        store.insert(mid_salience).unwrap();

        let mut consolidator = MemoryConsolidator::new(default_config());
        let result = consolidator.run(&store);
        assert_eq!(
            result.compressed_count, 2,
            "两条都应被压缩 / both should be compressed"
        );

        let facts = store.all_facts();
        let max_after = facts.iter().find(|f| f.object == "最深刻记忆").unwrap();
        let mid_after = facts.iter().find(|f| f.object == "中等记忆").unwrap();

        // salience=1.0 → effective_decay = 0.5 × (1 - 1.0 × 0.5) = 0.25（衰减 25%）
        // new_confidence = 0.3 × (1 - 0.25) = 0.3 × 0.75 = 0.225
        // salience=1.0 → effective_decay=0.25 (decay 25%) → 0.3 × 0.75 = 0.225
        assert!(
            (max_after.confidence - 0.225).abs() < 1e-6,
            "salience=1.0 应衰减为 0.225 (0.3 × 0.75), got {} / should be 0.225",
            max_after.confidence
        );

        // salience=0.5 → effective_decay = 0.5 × (1 - 0.5 × 0.5) = 0.375（衰减 37.5%）
        // new_confidence = 0.3 × (1 - 0.375) = 0.3 × 0.625 = 0.1875
        // salience=0.5 → effective_decay=0.375 (decay 37.5%) → 0.3 × 0.625 = 0.1875
        assert!(
            (mid_after.confidence - 0.1875).abs() < 1e-6,
            "salience=0.5 应衰减为 0.1875 (0.3 × 0.625), got {} / should be 0.1875",
            mid_after.confidence
        );
    }
}
