// 统一冲突引擎 / Unified Conflict Engine
//
// SPDX-License-Identifier: MIT
//! 统一冲突引擎 — 整合冲突成长（反应式）与模式学习（前瞻式）
//! Unified Conflict Engine — Integrates conflict growth (reactive) and
//! pattern learning (proactive) for complete conflict lifecycle management.
//!
//! 数字生命的冲突智能 = 反应式成长 + 前瞻式预判
//! Digital life's conflict intelligence = reactive growth + proactive prediction
//!
//! 反应式：金缮哲学 — 断裂处用金修复，修复处比原来更坚固
//! 前瞻式：模式智慧 — 从历史学习，预判未来，智慧应对

pub mod growth;
pub mod pattern;

// 重导出所有公共类型（向后兼容）/ Re-export all public types (backward compatible)
pub use growth::{
    ConflictWeight, EscalationWarning, EscalationWarningLevel, GrowthEntry, PostConflictGrowth,
    ReconciliationTiming,
};
pub use pattern::{
    ConflictPattern, ConflictPatternLearner, PatternLearnerConfig, PatternLearnerStats,
    PatternPrediction, SensitivityAdjustment,
};

use serde::{Deserialize, Serialize};

use crate::conflict_reconciliation::ConflictSignal;
use crate::relationship::RelationshipStage;

// ============================================================
// 统一冲突引擎 / Unified Conflict Engine
// ============================================================

/// 统一冲突引擎 / Unified Conflict Engine
///
/// 整合冲突成长（反应式）与模式学习（前瞻式），
/// 为数字生命提供完整的冲突生命周期管理。
///
/// Integrates conflict growth (reactive) and pattern learning (proactive),
/// providing complete conflict lifecycle management for digital life.
// 统一冲突引擎主体 / Unified conflict engine main body
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictEngine {
    // ── 反应式：冲突成长 / Reactive: Growth ──
    /// 升级预警器 / Escalation warning tracker
    pub escalation: EscalationWarning,
    /// 和解时机优化器 / Reconciliation timing optimizer
    pub timing: ReconciliationTiming,
    /// 冲突后成长追踪器 / Post-conflict growth tracker
    pub growth: PostConflictGrowth,

    // ── 前瞻式：模式学习 / Proactive: Pattern ──
    /// 冲突模式学习器 / Conflict pattern learner
    pub learner: ConflictPatternLearner,

    // ── 共享状态 / Shared State ──
    /// 当前冲突强度 / Current conflict intensity
    pub current_intensity: f64,
    /// 当前轮次 / Current turn
    pub current_turn: u32,
    /// 上次冲突以来的轮数 / Turns since last conflict
    pub turns_since_conflict: u32,
    /// 是否处于冲突中 / Whether in active conflict
    pub in_conflict: bool,
    /// prompt 预算（字符数） / Prompt hint budget in chars
    pub prompt_budget: usize,
}

impl ConflictEngine {
    /// 构造 / Constructor
    pub fn new() -> Self {
        Self {
            escalation: EscalationWarning::new(),
            timing: ReconciliationTiming::new(),
            growth: PostConflictGrowth::new(),
            learner: ConflictPatternLearner::default(),
            current_intensity: 0.0,
            current_turn: 0,
            turns_since_conflict: 0,
            in_conflict: false,
            prompt_budget: 400,
        }
    }

    /// 设置 prompt 预算 / Set prompt budget
    pub fn with_prompt_budget(mut self, budget: usize) -> Self {
        self.prompt_budget = budget;
        self
    }

    // ── 反应式接口 / Reactive Interface ──

    /// 冲突发生时调用 / Called when a conflict occurs
    ///
    /// 同时更新升级预警和学习冲突模式。
    /// Updates escalation warning and learns conflict patterns simultaneously.
    pub fn on_conflict(
        &mut self,
        intensity: f64,
        turn: u32,
        signals: &[ConflictSignal],
        stage: &RelationshipStage,
        epoch: i64,
    ) {
        self.current_intensity = intensity;
        self.current_turn = turn;
        self.turns_since_conflict = 0;
        self.in_conflict = true;
        self.escalation.update(intensity, turn);

        // 前瞻式：从冲突信号学习模式 / Proactive: learn from signals
        if !signals.is_empty() {
            self.learner.learn(signals, stage, epoch);
        }
    }

    /// 和解完成时调用 / Called when reconciliation completes
    ///
    /// `quality`: 和解质量 [0.0, 1.0]
    /// `conflict_depth`: 冲突深度 [0.0, 1.0]
    /// `delay_secs`: 实际延迟秒数
    /// `relationship_depth`: 关系深度 [0.0, 1.0]
    pub fn on_reconciliation(
        &mut self,
        quality: f64,
        conflict_depth: f64,
        delay_secs: u64,
        relationship_depth: f64,
    ) {
        // 应用过早和解惩罚 / Apply premature penalty
        let timing_quality = self.timing.timing_quality(delay_secs);
        let adjusted_quality = (quality * timing_quality).clamp(0.0, 1.0);

        // 记入成长 / Record growth
        let conflict_type = ConflictWeight::from_intensity(self.current_intensity).as_str();
        self.growth.record_growth(
            conflict_type,
            self.current_intensity,
            adjusted_quality,
            conflict_depth,
            chrono::Utc::now().timestamp_millis(),
        );

        self.in_conflict = false;
        let _ = relationship_depth; // 已在 timing 中设置 / already set in timing
    }

    /// 平静期检测 / Called during calm periods
    ///
    /// 更新冷却状态，检测未解决冲突。
    pub fn on_calm(&mut self, current_pleasure: f64, turns_since_conflict: u32) {
        self.turns_since_conflict = turns_since_conflict;
        self.escalation.calm();

        // 若曾冲突但未和解，且超过阈值轮数，计为未解决
        // If conflict occurred but no reconciliation and exceeds threshold, count unresolved
        if self.in_conflict && turns_since_conflict > 10 {
            self.growth.record_unresolved();
            self.in_conflict = false;
        }

        let _ = current_pleasure;
    }

    // ── 前瞻式接口 / Proactive Interface ──

    /// 从冲突信号学习模式 / Learn patterns from conflict signals
    ///
    /// 委托至内部模式学习器。Delegates to internal pattern learner.
    pub fn learn(&mut self, signals: &[ConflictSignal], stage: &RelationshipStage, epoch: i64) {
        self.learner.learn(signals, stage, epoch);
    }

    /// 模式感知 prompt 片段 / Pattern-aware prompt fragment
    ///
    /// 委托至内部模式学习器。Delegates to internal pattern learner.
    pub fn to_pattern_prompt_fragment(&self, stage: &RelationshipStage) -> String {
        self.learner.to_prompt_fragment(stage)
    }

    /// 预测潜在冲突 / Predict potential conflicts
    ///
    /// 给定用户文本和关系阶段，返回按置信度降序排列的预测列表。
    pub fn predict(
        &mut self,
        user_text: &str,
        stage: &RelationshipStage,
    ) -> Vec<PatternPrediction> {
        self.learner.predict(user_text, stage)
    }

    /// 建议灵敏度调整 / Suggest sensitivity adjustments
    pub fn suggest_sensitivity(&self, stage: &RelationshipStage) -> Vec<SensitivityAdjustment> {
        self.learner.suggest_sensitivity_adjustments(stage)
    }

    // ── 周期维护 / Periodic Maintenance ──

    /// 统一 tick — 模式衰减 + 修剪 / Unified tick — pattern decay + pruning
    pub fn tick(&mut self, epoch: i64) {
        self.learner.tick(epoch);
    }

    // ── 状态查询 / State Queries ──

    /// 当前预警等级 / Current warning level
    pub fn warning_level(&self) -> EscalationWarningLevel {
        self.escalation.warning_level()
    }

    /// 和解时机是否成熟 / Whether reconciliation is ready
    pub fn reconciliation_ready(&self, current_pleasure: f64) -> bool {
        self.timing
            .is_ready(current_pleasure, self.turns_since_conflict)
    }

    /// 当前关系韧性 / Current resilience score
    pub fn resilience_score(&self) -> f64 {
        self.growth.resilience()
    }

    /// 模式统计 / Pattern statistics
    pub fn pattern_stats(&self) -> PatternLearnerStats {
        self.learner.stats()
    }

    // ── Prompt 注入 / Prompt Injection ──

    /// 冲突成长 prompt 片段 / Conflict growth prompt fragment
    fn growth_prompt_fragment(&self) -> String {
        let level = self.warning_level();
        let resilience = self.resilience_score();
        let avg_growth = self.growth.avg_growth_score();
        let learning = self.growth.learning_factor();

        let level_str = match level {
            EscalationWarningLevel::None => "无升级风险",
            EscalationWarningLevel::Caution => "⚠冲突缓慢升级中",
            EscalationWarningLevel::Warning => "⚠⚠冲突快速升级，建议降温",
            EscalationWarningLevel::Alert => "🚨冲突危险升级，立即降温",
        };

        format!(
            "[冲突成长] {} | 韧性={:.2} | 均成长={:.2} | 学习因子={:.1}",
            level_str, resilience, avg_growth, learning,
        )
    }

    /// 统一 prompt 注入（受 budget 约束） / Unified prompt hint (budget-constrained)
    ///
    /// 合并冲突成长状态与模式感知片段，为数字生命提供完整的冲突智能提示。
    pub fn to_prompt_hint(&self, stage: &RelationshipStage) -> String {
        let growth_part = self.growth_prompt_fragment();
        let pattern_part = self.learner.to_prompt_fragment(stage);

        let combined = if pattern_part.is_empty() {
            growth_part
        } else {
            format!("{}\n{}", growth_part, pattern_part)
        };

        // 截断至预算（UTF-8 安全） / Truncate to budget (UTF-8 safe)
        truncate_utf8(&combined, self.prompt_budget)
    }

    /// 仅成长 prompt（向后兼容无 stage 场景） / Growth-only prompt (backward compat for no-stage)
    pub fn to_prompt_hint_growth_only(&self) -> String {
        let hint = self.growth_prompt_fragment();
        truncate_utf8(&hint, self.prompt_budget)
    }
}

impl Default for ConflictEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 序列化辅助 / Serialization Helper
// ============================================================

/// 可序列化冲突引擎快照 / Serializable conflict engine snapshot
///
/// 用于持久化引擎状态。通过 `from_engine` / `to_engine` 互转。
// 可序列化冲突引擎快照 / Serializable conflict engine snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableConflictEngine {
    /// 升级预警 / Escalation warning
    pub escalation: EscalationWarning,
    /// 和解时机 / Reconciliation timing
    pub timing: ReconciliationTiming,
    /// 冲突后成长 / Post-conflict growth
    pub growth: PostConflictGrowth,
    /// 模式学习器 / Pattern learner
    pub learner: ConflictPatternLearner,
    /// 当前冲突强度 / Current intensity
    pub current_intensity: f64,
    /// 当前轮次 / Current turn
    pub current_turn: u32,
    /// 上次冲突以来的轮数 / Turns since conflict
    pub turns_since_conflict: u32,
    /// 是否处于冲突中 / In conflict
    pub in_conflict: bool,
    /// prompt 预算 / Prompt budget
    pub prompt_budget: usize,
}

impl SerializableConflictEngine {
    /// 从引擎创建快照 / Create snapshot from engine
    pub fn from_engine(engine: &ConflictEngine) -> Self {
        Self {
            escalation: engine.escalation.clone(),
            timing: engine.timing.clone(),
            growth: engine.growth.clone(),
            learner: engine.learner.clone(),
            current_intensity: engine.current_intensity,
            current_turn: engine.current_turn,
            turns_since_conflict: engine.turns_since_conflict,
            in_conflict: engine.in_conflict,
            prompt_budget: engine.prompt_budget,
        }
    }

    /// 从快照恢复引擎 / Restore engine from snapshot
    pub fn to_engine(&self) -> ConflictEngine {
        ConflictEngine {
            escalation: self.escalation.clone(),
            timing: self.timing.clone(),
            growth: self.growth.clone(),
            learner: self.learner.clone(),
            current_intensity: self.current_intensity,
            current_turn: self.current_turn,
            turns_since_conflict: self.turns_since_conflict,
            in_conflict: self.in_conflict,
            prompt_budget: self.prompt_budget,
        }
    }
}

// ============================================================
// 向后兼容类型 / Backward Compatible Types
// ============================================================

/// 冲突成长引擎（向后兼容别名）/ Conflict growth engine (backward compat alias)
///
/// 保留旧名称以支持渐进迁移。新代码应直接使用 `ConflictEngine`。
pub type ConflictGrowthEngine = ConflictEngine;

/// 可序列化冲突成长快照（向后兼容别名）/ Serializable conflict growth (backward compat alias)
pub type SerializableConflictGrowth = SerializableConflictEngine;

// ============================================================
// 辅助函数 / Helper Functions
// ============================================================

/// UTF-8 安全截断 / UTF-8 safe truncation
///
/// 在字节预算内截断字符串，确保不截断在 UTF-8 字符中间。
fn truncate_utf8(s: &str, budget: usize) -> String {
    if s.len() <= budget {
        return s.to_string();
    }
    let mut end = budget;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = s[..end].to_string();
    result.truncate(end);
    result
}

// ============================================================
// 单元测试 / Unit Tests
// ============================================================

#[cfg(test)]
mod tests;
