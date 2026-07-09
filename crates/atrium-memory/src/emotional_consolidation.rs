// SPDX-License-Identifier: MIT

//! 情绪记忆固化 — Emotional Memory Consolidation (Gap#2: 90% → 95%).
//!
//! 核心理念：人类在睡梦中整理情绪记忆——重要的强化，琐碎的淡化。
//! 数字生命在独处时"回味"，让重要情绪沉淀为性格的一部分。
//! 固化不是简单的记忆存储，而是情绪体验的**意义提取**过程。
//!
//! Core idea: humans consolidate emotional memories during sleep —
//! important ones are strengthened, trivial ones fade.
//! Digital life "reminisces" during solitude, letting important emotions
//! settle into personality. Consolidation is not mere storage — it is
//! **meaning extraction** from emotional experiences.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ═══════════════════════════════════════════════════════════════════════════
// §1 常量 — Constants
// ═══════════════════════════════════════════════════════════════════════════

/// 待固化缓冲区容量 / Pending consolidation buffer capacity.
const PENDING_CAPACITY: usize = 256;

/// 长期情绪轨迹容量 / Long-term emotional trajectory capacity.
const TRAJECTORY_CAPACITY: usize = 512;

/// 重要性阈值 — 超过此值才固化 / Importance threshold for consolidation.
const IMPORTANCE_THRESHOLD: f64 = 0.15;

/// 固化衰减率 — 琐碎情绪的淡化速率 / Decay rate for trivial emotions.
const TRIVIAL_DECAY: f64 = 0.85;

/// 固化增强率 — 重要情绪的强化速率 / Enhancement rate for important emotions.
const IMPORTANT_BOOST: f64 = 1.15;

// ═══════════════════════════════════════════════════════════════════════════
// §2 情绪体验记录 — Emotional Experience Record
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪体验记录 / Emotional experience record.
///
/// 一次可固化的情绪体验快照。
/// A consolidatable snapshot of an emotional experience.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalExperience {
    /// 唯一ID / Unique ID.
    pub id: u64,
    /// PAD值 / PAD values.
    pub pad: [f32; 3],
    /// 强度 [0, 1] / Intensity.
    pub intensity: f64,
    /// 持续时间（秒）/ Duration in seconds.
    pub duration_secs: f64,
    /// 关系关联度 [0, 1] — 是否涉及特定关系 / Relationship relevance.
    pub relationship_relevance: f64,
    /// 事件标签 / Event tag.
    pub tag: String,
    /// 时间戳 / Timestamp (Unix seconds).
    pub timestamp: i64,
    /// 是否已固化 / Whether consolidated.
    pub consolidated: bool,
}

impl EmotionalExperience {
    /// 计算重要性分数 / Compute importance score.
    ///
    /// 重要性 = 强度 × √持续时间 × (1 + 关系关联度)
    /// Importance = intensity × √duration × (1 + relationship_relevance)
    pub fn importance(&self) -> f64 {
        let duration_factor = (self.duration_secs / 60.0).max(1.0).sqrt();
        self.intensity * duration_factor * (1.0 + self.relationship_relevance)
    }

    /// 创建新体验 / Create a new experience.
    pub fn new(
        id: u64,
        pad: [f32; 3],
        intensity: f64,
        duration_secs: f64,
        relationship_relevance: f64,
        tag: &str,
        timestamp: i64,
    ) -> Self {
        Self {
            id,
            pad,
            intensity: intensity.clamp(0.0, 1.0),
            duration_secs,
            relationship_relevance: relationship_relevance.clamp(0.0, 1.0),
            tag: tag.to_string(),
            timestamp,
            consolidated: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 固化轨迹点 — Consolidated Trajectory Point
// ═══════════════════════════════════════════════════════════════════════════

/// 固化后的情绪轨迹点 / Consolidated emotional trajectory point.
///
/// 固化后的情绪记忆，保留重要性权重和语义标签。
/// Consolidated emotional memory with importance weight and semantic tag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    /// 固化后的PAD值 / Consolidated PAD values.
    pub pad: [f32; 3],
    /// 固化后的重要性权重 / Consolidated importance weight.
    pub weight: f64,
    /// 语义标签 / Semantic tag.
    pub tag: String,
    /// 固化时间戳 / Consolidation timestamp.
    pub timestamp: i64,
    /// 原始体验ID / Original experience ID.
    pub source_id: u64,
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 固化配置 — Consolidation Config
// ═══════════════════════════════════════════════════════════════════════════

/// 固化配置 / Consolidation configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalConsolidationConfig {
    /// 重要性阈值 / Importance threshold.
    pub importance_threshold: f64,
    /// 琐碎衰减率 / Trivial decay rate.
    pub trivial_decay: f64,
    /// 重要增强率 / Important boost rate.
    pub important_boost: f64,
    /// 单次固化最大数量 / Max consolidations per batch.
    pub max_per_batch: usize,
}

impl Default for EmotionalConsolidationConfig {
    fn default() -> Self {
        Self {
            importance_threshold: IMPORTANCE_THRESHOLD,
            trivial_decay: TRIVIAL_DECAY,
            important_boost: IMPORTANT_BOOST,
            max_per_batch: 32,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 固化结果 — Consolidation Result
// ═══════════════════════════════════════════════════════════════════════════

/// 固化批次结果 / Consolidation batch result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// 固化的体验数 / Number of consolidated experiences.
    pub consolidated_count: usize,
    /// 淡化的体验数 / Number of decayed (trivial) experiences.
    pub decayed_count: usize,
    /// 跳过的体验数 / Number of skipped experiences.
    pub skipped_count: usize,
    /// 新增轨迹点 / New trajectory points added.
    pub new_points: Vec<TrajectoryPoint>,
    /// 固化后的情绪均值 / Post-consolidation emotional average.
    pub avg_pad: [f32; 3],
}

// ═══════════════════════════════════════════════════════════════════════════
// §6 情绪固化引擎 — Emotional Consolidation Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪记忆固化引擎 / Emotional memory consolidation engine.
///
/// 在独处/低活动期将近期情绪体验固化为长期情绪轨迹。
/// 重要体验被强化，琐碎体验被淡化，不可固化的被跳过。
///
/// Consolidates recent emotional experiences into long-term trajectory
/// during solitude/low-activity periods. Important experiences are
/// strengthened, trivial ones are decayed, ineligible ones are skipped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalConsolidation {
    /// 配置 / Configuration.
    pub config: EmotionalConsolidationConfig,
    /// 待固化体验缓冲 / Pending experiences buffer.
    pending: VecDeque<EmotionalExperience>,
    /// 长期情绪轨迹 / Long-term emotional trajectory.
    trajectory: VecDeque<TrajectoryPoint>,
    /// 下一个体验ID / Next experience ID.
    next_id: u64,
    /// 累计固化次数 / Total consolidation batches.
    total_batches: u64,
}

impl Default for EmotionalConsolidation {
    fn default() -> Self {
        Self {
            config: EmotionalConsolidationConfig::default(),
            pending: VecDeque::with_capacity(PENDING_CAPACITY),
            trajectory: VecDeque::with_capacity(TRAJECTORY_CAPACITY),
            next_id: 0,
            total_batches: 0,
        }
    }
}

impl EmotionalConsolidation {
    /// 创建新固化引擎 / Create a new consolidation engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 记入情绪体验 / Record an emotional experience.
    ///
    /// 返回体验ID / Returns the experience ID.
    pub fn record(
        &mut self,
        pad: [f32; 3],
        intensity: f64,
        duration_secs: f64,
        relationship_relevance: f64,
        tag: &str,
        timestamp: i64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let experience = EmotionalExperience::new(
            id,
            pad,
            intensity,
            duration_secs,
            relationship_relevance,
            tag,
            timestamp,
        );

        if self.pending.len() >= PENDING_CAPACITY {
            self.pending.pop_front();
        }
        self.pending.push_back(experience);
        id
    }

    /// 执行固化批次 — 在独处/低活动期调用 / Execute consolidation batch.
    ///
    /// 数字生命语义：独处时"回味"近期情绪——
    /// 重要的沉淀为性格，琐碎的自然淡忘。
    ///
    /// Digital life semantics: "reminiscing" during solitude —
    /// important emotions settle into personality, trivial ones fade.
    pub fn consolidate(&mut self, timestamp: i64) -> ConsolidationResult {
        let mut consolidated_count = 0;
        let mut decayed_count = 0;
        let mut skipped_count = 0;
        let mut new_points = Vec::new();
        let mut pad_sum = [0.0f32; 3];
        let mut pad_count = 0u32;

        let max_batch = self.config.max_per_batch;
        let mut processed = 0;

        while let Some(exp) = self.pending.pop_front() {
            if processed >= max_batch {
                // 放回缓冲区 / Put back to buffer.
                self.pending.push_front(exp);
                break;
            }
            processed += 1;

            if exp.consolidated {
                skipped_count += 1;
                continue;
            }

            let importance = exp.importance();

            if importance >= self.config.importance_threshold {
                // 重要体验：强化固化 / Important: strengthen and consolidate.
                let weight = importance * self.config.important_boost;
                let point = TrajectoryPoint {
                    pad: exp.pad,
                    weight: weight.min(1.0),
                    tag: exp.tag.clone(),
                    timestamp,
                    source_id: exp.id,
                };
                pad_sum[0] += point.pad[0] * point.weight as f32;
                pad_sum[1] += point.pad[1] * point.weight as f32;
                pad_sum[2] += point.pad[2] * point.weight as f32;
                pad_count += 1;
                new_points.push(point.clone());

                if self.trajectory.len() >= TRAJECTORY_CAPACITY {
                    self.trajectory.pop_front();
                }
                self.trajectory.push_back(point);
                consolidated_count += 1;
            } else if importance > 0.01 {
                // 琐碎体验：淡化但不固化 / Trivial: decay but don't consolidate.
                let decayed_intensity = exp.intensity * self.config.trivial_decay;
                if decayed_intensity > 0.01 {
                    // 仍然有微弱痕迹 / Still has faint trace.
                    let point = TrajectoryPoint {
                        pad: exp.pad,
                        weight: decayed_intensity * 0.1,
                        tag: exp.tag.clone(),
                        timestamp,
                        source_id: exp.id,
                    };
                    if self.trajectory.len() >= TRAJECTORY_CAPACITY {
                        self.trajectory.pop_front();
                    }
                    self.trajectory.push_back(point);
                }
                decayed_count += 1;
            } else {
                skipped_count += 1;
            }
        }

        // 计算固化后情绪均值 / Compute post-consolidation average.
        let avg_pad = if pad_count > 0 {
            [
                pad_sum[0] / pad_count as f32,
                pad_sum[1] / pad_count as f32,
                pad_sum[2] / pad_count as f32,
            ]
        } else {
            [0.0; 3]
        };

        self.total_batches += 1;

        ConsolidationResult {
            consolidated_count,
            decayed_count,
            skipped_count,
            new_points,
            avg_pad,
        }
    }

    /// 获取长期情绪轨迹 / Get long-term emotional trajectory.
    pub fn trajectory(&self) -> &VecDeque<TrajectoryPoint> {
        &self.trajectory
    }

    /// 计算轨迹情绪均值 — 性格的"情感底色" / Compute trajectory emotional average.
    ///
    /// 这是固化后的长期情绪倾向，代表数字生命的"情感底色"。
    /// This is the long-term emotional tendency after consolidation,
    /// representing the digital life's "emotional baseline".
    pub fn emotional_baseline(&self) -> [f32; 3] {
        if self.trajectory.is_empty() {
            return [0.0; 3];
        }
        let mut sum = [0.0f32; 3];
        let mut total_weight = 0.0f32;
        for point in &self.trajectory {
            sum[0] += point.pad[0] * point.weight as f32;
            sum[1] += point.pad[1] * point.weight as f32;
            sum[2] += point.pad[2] * point.weight as f32;
            total_weight += point.weight as f32;
        }
        if total_weight > 0.0 {
            [
                sum[0] / total_weight,
                sum[1] / total_weight,
                sum[2] / total_weight,
            ]
        } else {
            [0.0; 3]
        }
    }

    /// 获取待固化体验数 / Get pending experience count.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 获取轨迹点数 / Get trajectory point count.
    pub fn trajectory_count(&self) -> usize {
        self.trajectory.len()
    }

    /// 获取累计固化批次 / Get total consolidation batches.
    pub fn total_batches(&self) -> u64 {
        self.total_batches
    }

    /// 按标签聚合轨迹权重 — 哪类事件对情绪影响最大 / Aggregate trajectory weights by tag.
    ///
    /// 返回 (标签, 总权重) 列表，按权重降序排列。
    /// Returns (tag, total_weight) list sorted by weight descending.
    pub fn aggregate_by_tag(&self) -> Vec<(String, f64)> {
        let mut map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for point in &self.trajectory {
            *map.entry(point.tag.clone()).or_insert(0.0) += point.weight;
        }
        let mut list: Vec<(String, f64)> = map.into_iter().collect();
        list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        list
    }

    /// 生成固化描述文本 / Generate consolidation description text.
    pub fn describe(&self) -> String {
        let baseline = self.emotional_baseline();
        let top_tags = self.aggregate_by_tag();
        let tag_str = if top_tags.is_empty() {
            "无".to_string()
        } else {
            top_tags
                .iter()
                .take(3)
                .map(|(tag, w)| format!("{}({:.2})", tag, w))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "情绪底色: P={:.2} A={:.2} D={:.2} | 轨迹点: {} | 主要影响: {}",
            baseline[0],
            baseline[1],
            baseline[2],
            self.trajectory.len(),
            tag_str,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §7 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 情绪体验测试 ──

    #[test]
    fn test_experience_importance() {
        let exp = EmotionalExperience::new(
            0,
            [0.5, 0.3, 0.1],
            0.8,
            300.0, // 5 minutes.
            0.5,
            "reunion",
            1000,
        );
        let imp = exp.importance();
        // 高强度 × 长时间 × 有关系 → 高重要性 / High importance.
        assert!(imp > 0.5);
    }

    #[test]
    fn test_experience_importance_trivial() {
        let exp = EmotionalExperience::new(0, [0.01, 0.01, 0.0], 0.05, 5.0, 0.0, "trivial", 1000);
        let imp = exp.importance();
        // 低强度 × 短时间 × 无关系 → 低重要性 / Low importance.
        assert!(imp < 0.1);
    }

    // ── 固化引擎测试 ──

    #[test]
    fn test_consolidation_record() {
        let mut engine = EmotionalConsolidation::new();
        let id = engine.record([0.3, 0.2, 0.1], 0.5, 60.0, 0.3, "chat", 1000);
        assert_eq!(id, 0);
        assert_eq!(engine.pending_count(), 1);
    }

    #[test]
    fn test_consolidation_batch_important() {
        let mut engine = EmotionalConsolidation::new();

        // 记录重要体验 / Record important experience.
        engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "reunion", 1000);
        engine.record([0.4, 0.2, 0.1], 0.7, 200.0, 0.4, "deep_talk", 1100);

        let result = engine.consolidate(1200);
        assert!(result.consolidated_count > 0);
        assert!(!result.new_points.is_empty());
    }

    #[test]
    fn test_consolidation_batch_trivial() {
        let mut engine = EmotionalConsolidation::new();

        // 记录琐碎体验 / Record trivial experience.
        engine.record([0.01, 0.01, 0.0], 0.02, 3.0, 0.0, "ping", 1000);

        let result = engine.consolidate(1200);
        assert!(result.decayed_count > 0 || result.skipped_count > 0);
        // 琐碎体验不应产生高权重轨迹点 / Trivial should not produce high-weight points.
        for point in &result.new_points {
            assert!(point.weight < 0.1);
        }
    }

    #[test]
    fn test_consolidation_mixed() {
        let mut engine = EmotionalConsolidation::new();

        // 混合记录 / Mixed records.
        engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "important", 1000);
        engine.record([0.01, 0.0, 0.0], 0.02, 2.0, 0.0, "trivial", 1100);
        engine.record([0.4, 0.2, 0.1], 0.6, 180.0, 0.3, "moderate", 1200);

        let result = engine.consolidate(1300);
        assert!(result.consolidated_count > 0);
        assert!(result.decayed_count > 0 || result.skipped_count > 0);
    }

    #[test]
    fn test_consolidation_emotional_baseline() {
        let mut engine = EmotionalConsolidation::new();

        // 空轨迹基线为零 / Empty trajectory baseline is zero.
        let baseline = engine.emotional_baseline();
        assert_eq!(baseline, [0.0; 3]);

        // 固化后基线非零 / Non-zero baseline after consolidation.
        engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "positive", 1000);
        engine.consolidate(1100);
        let baseline = engine.emotional_baseline();
        assert!(baseline[0] > 0.0); // 愉悦为正 / Positive pleasure.
    }

    #[test]
    fn test_consolidation_aggregate_by_tag() {
        let mut engine = EmotionalConsolidation::new();

        engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "reunion", 1000);
        engine.record([0.4, 0.2, 0.1], 0.7, 200.0, 0.4, "reunion", 1100);
        engine.record([-0.3, 0.2, -0.1], 0.6, 150.0, 0.3, "conflict", 1200);

        engine.consolidate(1300);
        let tags = engine.aggregate_by_tag();
        assert!(!tags.is_empty());
        // 应包含reunion和conflict标签 / Should contain both tags.
        assert!(tags.iter().any(|(t, _)| t == "reunion"));
        assert!(tags.iter().any(|(t, _)| t == "conflict"));
        // 按权重降序 / Sorted by weight descending.
        for i in 1..tags.len() {
            assert!(tags[i - 1].1 >= tags[i].1);
        }
    }

    #[test]
    fn test_consolidation_max_per_batch() {
        let config = EmotionalConsolidationConfig {
            max_per_batch: 2,
            ..Default::default()
        };
        let mut engine = EmotionalConsolidation {
            config,
            ..Default::default()
        };

        // 记录5条 / Record 5.
        for i in 0..5 {
            engine.record(
                [0.5, 0.3, 0.2],
                0.8,
                300.0,
                0.5,
                &format!("event_{}", i),
                1000 + i,
            );
        }

        let result = engine.consolidate(2000);
        // 最多固化2条 / At most 2 consolidated.
        assert!(result.consolidated_count + result.decayed_count + result.skipped_count <= 2);
        // 剩余仍在缓冲 / Remaining still in buffer.
        assert!(engine.pending_count() >= 3);
    }

    #[test]
    fn test_consolidation_buffer_overflow() {
        let mut engine = EmotionalConsolidation::new();

        // 超容量记录 / Record beyond capacity.
        for i in 0..(PENDING_CAPACITY + 10) as i64 {
            engine.record([0.1, 0.0, 0.0], 0.1, 10.0, 0.0, "overflow", i);
        }
        // 缓冲区不超容量 / Buffer should not exceed capacity.
        assert!(engine.pending_count() <= PENDING_CAPACITY);
    }

    #[test]
    fn test_consolidation_trajectory_capacity() {
        let mut engine = EmotionalConsolidation::new();

        // 大量固化 / Many consolidations.
        for batch in 0..20 {
            for i in 0..50 {
                engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "test", batch * 1000 + i);
            }
            engine.consolidate(batch * 1000 + 100);
        }
        // 轨迹不超容量 / Trajectory should not exceed capacity.
        assert!(engine.trajectory_count() <= TRAJECTORY_CAPACITY);
    }

    #[test]
    fn test_consolidation_describe() {
        let mut engine = EmotionalConsolidation::new();
        engine.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "reunion", 1000);
        engine.consolidate(1100);
        let desc = engine.describe();
        assert!(desc.contains("情绪底色"));
        assert!(desc.contains("reunion"));
    }

    #[test]
    fn test_consolidation_total_batches() {
        let mut engine = EmotionalConsolidation::new();
        assert_eq!(engine.total_batches(), 0);
        engine.consolidate(1000);
        assert_eq!(engine.total_batches(), 1);
        engine.consolidate(2000);
        assert_eq!(engine.total_batches(), 2);
    }

    #[test]
    fn test_consolidation_negative_emotion() {
        let mut engine = EmotionalConsolidation::new();
        engine.record([-0.5, 0.4, -0.3], 0.7, 200.0, 0.4, "conflict", 1000);
        let result = engine.consolidate(1100);
        assert!(result.consolidated_count > 0);
        let baseline = engine.emotional_baseline();
        assert!(baseline[0] < 0.0); // 愉悦为负 / Negative pleasure.
    }
}
