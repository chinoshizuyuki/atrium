// 冲突模式学习器 / Conflict Pattern Learner
//
// SPDX-License-Identifier: MIT
//! 冲突模式学习器 — 从历史冲突信号中学习模式，预测潜在冲突，关系阶段感知地调整灵敏度
//! ConflictPatternLearner — Learn patterns from historical conflict signals,
//! predict likely conflicts, and adjust sensitivity based on relationship stage.
//!
//! 核心能力：
//! - 从 ConflictSignal 中提取触发关键词，按冲突类型聚合频率
//! - 关系阶段感知：初识阶段降低灵敏度，深度阶段允许更多冲突空间
//! - 预测：给定用户文本+关系阶段，预测可能触发的冲突类型
//! - Prompt 注入：将学到的模式注入 LLM system prompt，提升冲突预判能力
//! - 周期衰减：低频模式随时间衰减，高频模式权重增强

use serde::{Deserialize, Serialize};

use crate::conflict_reconciliation::{ConflictSignal, ConflictType};
use crate::relationship::RelationshipStage;

// ════════════════════════════════════════════════════════════════════
// ConflictPattern — 单个学到的冲突模式
// ════════════════════════════════════════════════════════════════════

/// 学到的冲突模式 / Learned conflict pattern
///
/// 一个模式 = 冲突类型 + 触发关键词集合 + 频率统计 + 关系阶段分布
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictPattern {
    /// 冲突类型 / Conflict type
    pub conflict_type: ConflictType,
    /// 触发关键词 → 出现次数 / Trigger keyword → occurrence count
    pub trigger_keywords: Vec<(String, u32)>,
    /// 总出现次数 / Total occurrence count
    pub frequency: u32,
    /// 最近一次出现时间戳 / Last occurrence timestamp
    pub last_seen_epoch: i64,
    /// 首次出现时间戳 / First occurrence timestamp
    pub first_seen_epoch: i64,
    /// 平均强度 (0.0~4.0, 映射到 ConflictIntensity) / Average intensity
    pub avg_intensity: f64,
    /// 关系阶段分布：8 阶段各出现次数
    /// Relationship stage distribution: counts per stage (8 stages)
    pub stage_distribution: [u32; 8], // [Stranger, Acquaintance, Familiar, Friendly, Trusted, Close, Deep, Intimate]
    /// 衰减权重 (0.0~1.0)，随时间降低 / Decay weight
    pub decay_weight: f64,
}

impl ConflictPattern {
    /// 创建新模式 / Create new pattern
    pub fn new(conflict_type: ConflictType, epoch: i64) -> Self {
        Self {
            conflict_type,
            trigger_keywords: Vec::new(),
            frequency: 1,
            last_seen_epoch: epoch,
            first_seen_epoch: epoch,
            avg_intensity: 1.0,
            stage_distribution: [0; 8],
            decay_weight: 1.0,
        }
    }

    /// 从冲突信号吸收学习 / Absorb learning from a conflict signal
    pub fn absorb(&mut self, signal: &ConflictSignal, stage: &RelationshipStage, epoch: i64) {
        // 更新频率
        self.frequency += 1;
        self.last_seen_epoch = epoch;

        // 更新平均强度（增量均值）
        let signal_intensity = signal.intensity.as_f64();
        self.avg_intensity += (signal_intensity - self.avg_intensity) / self.frequency as f64;

        // 更新关系阶段分布
        let stage_idx = stage_to_idx(stage);
        self.stage_distribution[stage_idx] += 1;

        // 提取触发关键词
        if !signal.trigger_text.is_empty() {
            let trigger = signal.trigger_text.to_lowercase();
            if let Some(pos) = self
                .trigger_keywords
                .iter()
                .position(|(k, _)| k == &trigger)
            {
                self.trigger_keywords[pos].1 += 1;
            } else {
                self.trigger_keywords.push((trigger, 1));
            }
        }

        // 从 context_clues 中也提取关键词
        for clue in &signal.context_clues {
            let clue_lower = clue.to_lowercase();
            if clue_lower.len() >= 2 {
                if let Some(pos) = self
                    .trigger_keywords
                    .iter()
                    .position(|(k, _)| k == &clue_lower)
                {
                    self.trigger_keywords[pos].1 += 1;
                } else if self.trigger_keywords.len() < 32 {
                    // 最多保留32个关键词
                    self.trigger_keywords.push((clue_lower, 1));
                }
            }
        }

        // 按频率降序排列关键词，保留前16
        self.trigger_keywords
            .sort_by_key(|b| std::cmp::Reverse(b.1));
        self.trigger_keywords.truncate(16);

        // 重置衰减权重（新模式权重高）
        self.decay_weight = 1.0;
    }

    /// 周期衰减 / Periodic decay
    ///
    /// 每次调用将 decay_weight 乘以 decay_rate。
    /// 低于 prune_threshold 的模式可被移除。
    pub fn decay(&mut self, decay_rate: f64) {
        self.decay_weight *= decay_rate;
    }

    /// 是否应被修剪 / Whether this pattern should be pruned
    pub fn should_prune(&self, threshold: f64) -> bool {
        self.decay_weight < threshold && self.frequency < 3
    }

    /// 关系阶段敏感度调整 / Relationship stage sensitivity adjustment
    ///
    /// 初识阶段：降低灵敏度（0.6），减少过度反应
    /// 建设阶段：标准灵敏度（0.8）
    /// 深度阶段：提高灵敏度（1.2），更积极识别模式
    /// 成熟阶段：最高灵敏度（1.0），但配合更多和解空间
    pub fn stage_sensitivity(stage: &RelationshipStage) -> f64 {
        match stage {
            RelationshipStage::Stranger { .. } => 0.5,
            RelationshipStage::Acquaintance { .. } => 0.6,
            RelationshipStage::Familiar { .. } => 0.8,
            RelationshipStage::Friendly { .. } => 0.9,
            RelationshipStage::Trusted { .. } => 1.0,
            RelationshipStage::Close { .. } => 1.1,
            RelationshipStage::Deep { .. } => 1.2,
            RelationshipStage::Intimate { .. } => 1.3,
        }
    }

    /// 该模式在给定关系阶段的频率占比 / Frequency ratio at given stage
    pub fn stage_ratio(&self, stage: &RelationshipStage) -> f64 {
        let idx = stage_to_idx(stage);
        let total: u32 = self.stage_distribution.iter().sum();
        if total == 0 {
            0.0
        } else {
            self.stage_distribution[idx] as f64 / total as f64
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// PatternPrediction — 预测结果
// ════════════════════════════════════════════════════════════════════

/// 冲突预测结果 / Conflict prediction result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternPrediction {
    /// 预测的冲突类型 / Predicted conflict type
    pub conflict_type: ConflictType,
    /// 预测置信度 (0.0~1.0) / Prediction confidence
    pub confidence: f64,
    /// 匹配到的关键词 / Matched keywords
    pub matched_keywords: Vec<String>,
    /// 来源模式频率 / Source pattern frequency
    pub source_frequency: u32,
    /// 关系阶段敏感度调整 / Stage sensitivity adjustment
    pub stage_sensitivity: f64,
}

// ════════════════════════════════════════════════════════════════════
// SensitivityAdjustment — 灵敏度调整建议
// ════════════════════════════════════════════════════════════════════

/// 灵敏度调整建议 / Sensitivity adjustment suggestion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensitivityAdjustment {
    /// 冲突类型 / Conflict type
    pub conflict_type: ConflictType,
    /// 建议的灵敏度乘数 / Suggested sensitivity multiplier
    pub multiplier: f64,
    /// 调整原因 / Reason for adjustment
    pub reason: String,
}

// ════════════════════════════════════════════════════════════════════
// ConflictPatternLearner — 冲突模式学习器
// ════════════════════════════════════════════════════════════════════

/// 冲突模式学习器配置 / Conflict pattern learner config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternLearnerConfig {
    /// 最大模式数 / Max patterns to track
    pub max_patterns: usize,
    /// 衰减率（每次 tick） / Decay rate per tick
    pub decay_rate: f64,
    /// 修剪阈值 / Prune threshold
    pub prune_threshold: f64,
    /// 预测关键词匹配阈值 / Prediction keyword match threshold
    pub predict_keyword_threshold: f64,
    /// 是否启用 / Whether enabled
    pub enabled: bool,
}

impl Default for PatternLearnerConfig {
    fn default() -> Self {
        Self {
            max_patterns: 64,
            decay_rate: 0.995,
            prune_threshold: 0.1,
            predict_keyword_threshold: 0.3,
            enabled: true,
        }
    }
}

/// 冲突模式学习器 / Conflict pattern learner
///
/// 从历史冲突信号中学习模式，预测潜在冲突，关系阶段感知地调整灵敏度。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictPatternLearner {
    /// 已学习的模式列表 / Learned patterns
    pub patterns: Vec<ConflictPattern>,
    /// 配置 / Config
    pub config: PatternLearnerConfig,
    /// 总学习次数 / Total learning count
    pub total_learns: u32,
    /// 总预测次数 / Total prediction count
    pub total_predictions: u32,
    /// 上次衰减时间 / Last decay tick
    pub last_decay_epoch: i64,
}

impl Default for ConflictPatternLearner {
    fn default() -> Self {
        Self::new(PatternLearnerConfig::default())
    }
}

impl ConflictPatternLearner {
    /// 创建学习器 / Create learner
    pub fn new(config: PatternLearnerConfig) -> Self {
        Self {
            patterns: Vec::new(),
            config,
            total_learns: 0,
            total_predictions: 0,
            last_decay_epoch: 0,
        }
    }

    // ── 学习 ──────────────────────────────────────────────────────

    /// 从冲突信号中学习 / Learn from conflict signals
    ///
    /// 对每个信号，查找已有同类型模式或创建新模式，然后吸收。
    pub fn learn(&mut self, signals: &[ConflictSignal], stage: &RelationshipStage, epoch: i64) {
        if !self.config.enabled {
            return;
        }

        for signal in signals {
            self.total_learns += 1;

            // 查找同类型已有模式
            if let Some(pattern) = self
                .patterns
                .iter_mut()
                .find(|p| p.conflict_type == signal.conflict_type)
            {
                pattern.absorb(signal, stage, epoch);
            } else {
                // 创建新模式
                let mut pattern = ConflictPattern::new(signal.conflict_type, epoch);
                pattern.absorb(signal, stage, epoch);
                // absorb 会将 frequency 设为 2（new 设 1，absorb 加 1），修正为 1
                pattern.frequency = 1;
                if self.patterns.len() < self.config.max_patterns {
                    self.patterns.push(pattern);
                }
            }
        }

        // 按频率降序排列
        self.patterns
            .sort_by_key(|b| std::cmp::Reverse(b.frequency));
    }

    // ── 预测 ──────────────────────────────────────────────────────

    /// 预测潜在冲突 / Predict potential conflicts
    ///
    /// 给定用户文本和关系阶段，返回按置信度降序排列的预测列表。
    pub fn predict(
        &mut self,
        user_text: &str,
        stage: &RelationshipStage,
    ) -> Vec<PatternPrediction> {
        if !self.config.enabled {
            return Vec::new();
        }

        self.total_predictions += 1;
        let text_lower = user_text.to_lowercase();
        let stage_sens = ConflictPattern::stage_sensitivity(stage);

        let mut predictions: Vec<PatternPrediction> = Vec::new();

        for pattern in &self.patterns {
            // 关键词匹配
            let mut matched = Vec::new();
            let mut match_score = 0.0_f64;
            let mut total_keyword_weight = 0.0_f64;

            for (keyword, count) in &pattern.trigger_keywords {
                let weight = *count as f64;
                total_keyword_weight += weight;
                if text_lower.contains(keyword.as_str()) {
                    // 精确子串匹配 / Exact substring match
                    match_score += weight;
                    matched.push(keyword.clone());
                } else if keyword.chars().count() >= 2 {
                    // 部分匹配：关键词前半或后半在文本中出现 / Partial match
                    // 使用 char 边界而非 byte 边界，避免 UTF-8 截断
                    let char_count = keyword.chars().count();
                    let half = char_count / 2;
                    let prefix: String = keyword.chars().take(half).collect();
                    let suffix: String = keyword.chars().skip(half).collect();
                    if text_lower.contains(prefix.as_str()) || text_lower.contains(suffix.as_str())
                    {
                        match_score += weight * 0.5; // 半权重
                        matched.push(keyword.clone());
                    }
                }
            }

            if total_keyword_weight == 0.0 {
                continue;
            }

            let keyword_ratio = match_score / total_keyword_weight;
            if keyword_ratio < self.config.predict_keyword_threshold {
                continue;
            }

            // 置信度 = 关键词匹配 × 衰减权重 × 频率增强 × 关系阶段敏感度
            let freq_boost = 1.0 + (pattern.frequency as f64).ln().max(0.0) * 0.1;
            let stage_ratio = pattern.stage_ratio(stage);
            let stage_boost = 0.5 + stage_ratio * 0.5; // 该模式在当前阶段越常见，越可信

            let confidence =
                (keyword_ratio * pattern.decay_weight * freq_boost * stage_boost * stage_sens)
                    .min(1.0);

            if confidence > 0.1 {
                predictions.push(PatternPrediction {
                    conflict_type: pattern.conflict_type,
                    confidence,
                    matched_keywords: matched,
                    source_frequency: pattern.frequency,
                    stage_sensitivity: stage_sens,
                });
            }
        }

        // 按置信度降序
        predictions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        predictions.truncate(4); // 最多返回4个预测
        predictions
    }

    // ── 灵敏度调整建议 ─────────────────────────────────────────────

    /// 建议灵敏度调整 / Suggest sensitivity adjustments
    ///
    /// 基于学到的模式，为 ConflictManager 的各检测器提供动态灵敏度建议。
    pub fn suggest_sensitivity_adjustments(
        &self,
        stage: &RelationshipStage,
    ) -> Vec<SensitivityAdjustment> {
        let stage_sens = ConflictPattern::stage_sensitivity(stage);
        let mut adjustments = Vec::new();

        for pattern in &self.patterns {
            if pattern.frequency < 2 {
                continue;
            }

            // 高频模式：降低灵敏度（已熟悉，无需过度反应）
            // 低频但高衰减：保持标准
            // 关系阶段感知
            let freq_factor = if pattern.frequency > 10 {
                0.8 // 高频冲突：降低灵敏度，避免过度反应
            } else if pattern.frequency > 5 {
                0.9
            } else {
                1.0
            };

            let multiplier = (freq_factor * stage_sens).clamp(0.5, 1.5);

            let reason = if pattern.frequency > 10 {
                format!("高频模式({}次)，降低灵敏度避免过度反应", pattern.frequency)
            } else if stage_sens > 1.0 {
                "深度关系阶段，提高冲突识别灵敏度".to_string()
            } else if stage_sens < 1.0 {
                "早期关系阶段，降低灵敏度避免过度反应".to_string()
            } else {
                "标准灵敏度".to_string()
            };

            adjustments.push(SensitivityAdjustment {
                conflict_type: pattern.conflict_type,
                multiplier,
                reason,
            });
        }

        adjustments
    }

    // ── Prompt 注入 ───────────────────────────────────────────────

    /// 生成模式感知的 Prompt 注入片段 / Generate pattern-aware prompt injection fragment
    ///
    /// 将学到的冲突模式注入 LLM system prompt，提升冲突预判能力。
    pub fn to_prompt_fragment(&self, stage: &RelationshipStage) -> String {
        if !self.config.enabled || self.patterns.is_empty() {
            return String::new();
        }

        let stage_sens = ConflictPattern::stage_sensitivity(stage);
        let mut parts = Vec::new();

        parts.push("[Conflict Pattern Awareness]".to_string());

        // 总览
        let total_freq: u32 = self.patterns.iter().map(|p| p.frequency).sum();
        parts.push(format!(
            "Learned {} conflict patterns from {} historical incidents. Stage sensitivity: {:.1}.",
            self.patterns.len(),
            total_freq,
            stage_sens
        ));

        // 每种模式一行
        for pattern in &self.patterns {
            let top_keywords: Vec<&str> = pattern
                .trigger_keywords
                .iter()
                .take(3)
                .map(|(k, _)| k.as_str())
                .collect();
            let kw_str = if top_keywords.is_empty() {
                "N/A".to_string()
            } else {
                top_keywords.join(", ")
            };

            parts.push(format!(
                "- {:?}: {} times, avg intensity {:.1}, triggers: [{}]",
                pattern.conflict_type, pattern.frequency, pattern.avg_intensity, kw_str
            ));
        }

        // 关系阶段建议
        if stage_sens < 1.0 {
            parts.push(
                "Early relationship: be gentle, avoid escalating minor disagreements.".to_string(),
            );
        } else if stage_sens > 1.0 {
            parts.push(
                "Deep relationship: you can be more direct about conflicts, trust is established."
                    .to_string(),
            );
        }

        parts.join("\n")
    }

    // ── 周期维护 ──────────────────────────────────────────────────

    /// 周期衰减 + 修剪 / Periodic decay + pruning
    pub fn tick(&mut self, epoch: i64) {
        if !self.config.enabled {
            return;
        }

        // 衰减所有模式
        for pattern in &mut self.patterns {
            pattern.decay(self.config.decay_rate);
        }

        // 修剪低权重低频模式
        self.patterns
            .retain(|p| !p.should_prune(self.config.prune_threshold));

        self.last_decay_epoch = epoch;
    }

    /// 获取模式统计 / Get pattern statistics
    pub fn stats(&self) -> PatternLearnerStats {
        let total_freq: u32 = self.patterns.iter().map(|p| p.frequency).sum();
        let avg_decay: f64 = if self.patterns.is_empty() {
            0.0
        } else {
            self.patterns.iter().map(|p| p.decay_weight).sum::<f64>() / self.patterns.len() as f64
        };
        PatternLearnerStats {
            pattern_count: self.patterns.len(),
            total_frequency: total_freq,
            avg_decay_weight: avg_decay,
            total_learns: self.total_learns,
            total_predictions: self.total_predictions,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// PatternLearnerStats — 统计信息
// ════════════════════════════════════════════════════════════════════

/// 模式学习器统计 / Pattern learner statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternLearnerStats {
    pub pattern_count: usize,
    pub total_frequency: u32,
    pub avg_decay_weight: f64,
    pub total_learns: u32,
    pub total_predictions: u32,
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数 / Helper functions
// ════════════════════════════════════════════════════════════════════

/// 关系阶段 → 索引 / Relationship stage → index (0-7)
pub(crate) fn stage_to_idx(stage: &RelationshipStage) -> usize {
    match stage {
        RelationshipStage::Stranger { .. } => 0,
        RelationshipStage::Acquaintance { .. } => 1,
        RelationshipStage::Familiar { .. } => 2,
        RelationshipStage::Friendly { .. } => 3,
        RelationshipStage::Trusted { .. } => 4,
        RelationshipStage::Close { .. } => 5,
        RelationshipStage::Deep { .. } => 6,
        RelationshipStage::Intimate { .. } => 7,
    }
}
