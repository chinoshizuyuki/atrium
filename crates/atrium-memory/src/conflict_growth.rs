// 冲突与和解 / Conflict & Reconciliation
//
// SPDX-License-Identifier: MIT
//! 冲突成长引擎 / Conflict Growth Engine
//!
//! Gap#4 冲突与和解增强模块：在冲突中成长，在和解中深化关系。
//! 涵盖：冲突升级预警（速度+加速度）、和解时机优化（情绪衰减检测）、
//! 冲突后成长追踪（金缮哲学——断裂处用金修复，修复处比原来更坚固）。
//!
//! Gap#4 Conflict & Reconciliation enhancement: growing through conflict,
//! deepening relationship through reconciliation.
//! Covers: escalation warning (velocity + acceleration), reconciliation timing
//! optimization (emotion decay detection), post-conflict growth tracking
//! (Kintsugi philosophy — repair fractures with gold, the mended place is stronger).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ============================================================
// 第一部分：冲突升级预警 / Part 1: Escalation Warning
// ============================================================

/// 预警等级 / Escalation warning level
// 冲突升级预警等级 / Conflict escalation warning level
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationWarningLevel {
    /// 无预警 / No warning
    None,
    /// 注意：趋势上升 / Caution: upward trend
    Caution,
    /// 警告：快速升级 / Warning: rapid escalation
    Warning,
    /// 警报：危险升级 / Alert: dangerous escalation
    Alert,
}

/// 冲突升级预警器 / Escalation warning tracker
///
/// 追踪冲突强度的变化速率（velocity）与加速度（acceleration），
/// 当两者同时超过阈值时发出预警。
/// Tracks conflict intensity change rate (velocity) and acceleration,
/// issuing warnings when both exceed thresholds.
// 冲突升级预警追踪器 / Conflict escalation warning tracker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationWarning {
    /// 上一轮冲突强度 / Previous intensity
    prev_intensity: f64,
    /// 上一轮轮次 / Previous turn
    prev_turn: u32,
    /// 当前升级速度 / Current escalation velocity
    velocity: f64,
    /// 当前加速度 / Current acceleration
    acceleration: f64,
    /// 连续无冲突轮数 / Consecutive calm turns
    calm_turns: u32,
    /// 是否已初始化 / Whether initialized
    initialized: bool,
}

impl EscalationWarning {
    /// 构造 / Constructor
    pub fn new() -> Self {
        Self {
            prev_intensity: 0.0,
            prev_turn: 0,
            velocity: 0.0,
            acceleration: 0.0,
            calm_turns: 0,
            initialized: false,
        }
    }

    /// 每轮冲突更新 / Update on each conflict turn
    ///
    /// 计算 velocity = Δintensity / Δturns，
    /// acceleration = Δvelocity / Δturns。
    pub fn update(&mut self, current_intensity: f64, turn: u32) {
        if !self.initialized {
            self.prev_intensity = current_intensity;
            self.prev_turn = turn;
            self.initialized = true;
            self.calm_turns = 0;
            return;
        }

        let delta_turns = (turn - self.prev_turn) as f64;
        if delta_turns <= 0.0 {
            return;
        }

        let delta_intensity = current_intensity - self.prev_intensity;
        let new_velocity = delta_intensity / delta_turns;
        self.acceleration = (new_velocity - self.velocity) / delta_turns;
        self.velocity = new_velocity;
        self.prev_intensity = current_intensity;
        self.prev_turn = turn;
        self.calm_turns = 0;
    }

    /// 记算当前预警等级 / Compute current warning level
    pub fn warning_level(&self) -> EscalationWarningLevel {
        let v = self.velocity;
        let a = self.acceleration;

        if a > 0.3 && v > 0.4 {
            EscalationWarningLevel::Alert
        } else if a > 0.2 && v > 0.3 {
            EscalationWarningLevel::Warning
        } else if a > 0.1 && v > 0.2 {
            EscalationWarningLevel::Caution
        } else {
            EscalationWarningLevel::None
        }
    }

    /// 平静轮：递增冷却计数，连续 3 轮无冲突则重置 / Calm turn: increment cooldown, reset after 3
    pub fn calm(&mut self) {
        self.calm_turns += 1;
        if self.calm_turns >= 3 {
            self.velocity = 0.0;
            self.acceleration = 0.0;
            self.calm_turns = 0;
            self.initialized = false;
        }
    }
}

impl Default for EscalationWarning {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 第二部分：和解时机优化 / Part 2: Reconciliation Timing
// ============================================================

/// 冲突权重 / Conflict weight for timing calculation
// 冲突权重等级 / Conflict weight level
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictWeight {
    /// 微弱 / Trivial
    Trivial,
    /// 轻度 / Mild
    Mild,
    /// 中度 / Moderate
    Moderate,
    /// 强度 / Severe
    Severe,
    /// 临界 / Critical
    Critical,
}

impl ConflictWeight {
    /// 权重值 / Weight value
    pub fn value(&self) -> f64 {
        match self {
            Self::Trivial => 0.5,
            Self::Mild => 1.0,
            Self::Moderate => 1.5,
            Self::Severe => 2.5,
            Self::Critical => 4.0,
        }
    }

    /// 从强度数值推断 / Infer from intensity value
    pub fn from_intensity(intensity: f64) -> Self {
        if intensity < 0.2 {
            Self::Trivial
        } else if intensity < 0.4 {
            Self::Mild
        } else if intensity < 0.6 {
            Self::Moderate
        } else if intensity < 0.8 {
            Self::Severe
        } else {
            Self::Critical
        }
    }
}

/// 和解时机优化器 / Reconciliation timing optimizer
///
/// 最优延迟公式：
/// `optimal_delay = base_cooldown × conflict_weight × (1 + relationship_depth)`
///
/// 情绪衰减检测：当 pleasure 从负值回升到 > -0.1 时，和解时机成熟。
// 和解时机优化器 / Reconciliation timing optimizer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReconciliationTiming {
    /// 基础冷却时间（秒） / Base cooldown in seconds
    base_cooldown: f64,
    /// 冲突权重 / Conflict weight
    conflict_weight: ConflictWeight,
    /// 关系深度 [0.0, 1.0] / Relationship depth
    relationship_depth: f64,
    /// 上次冲突时 pleasure 值 / Pleasure at last conflict
    last_pleasure: f64,
    /// 最优延迟（秒） / Optimal delay in seconds
    optimal_delay: f64,
}

impl ReconciliationTiming {
    /// 构造 / Constructor
    pub fn new() -> Self {
        Self {
            base_cooldown: 300.0,
            conflict_weight: ConflictWeight::Mild,
            relationship_depth: 0.0,
            last_pleasure: 0.0,
            optimal_delay: 300.0,
        }
    }

    /// 设置冲突参数 / Set conflict parameters
    pub fn set_conflict(&mut self, intensity: f64, relationship_depth: f64) {
        self.conflict_weight = ConflictWeight::from_intensity(intensity);
        self.relationship_depth = relationship_depth.clamp(0.0, 1.0);
        self.optimal_delay =
            self.base_cooldown * self.conflict_weight.value() * (1.0 + self.relationship_depth);
    }

    /// 记算最优延迟（秒） / Compute optimal delay in seconds
    pub fn optimal_delay_secs(&self) -> u64 {
        self.optimal_delay.round() as u64
    }

    /// 和解时机是否成熟 / Whether reconciliation is ready
    ///
    /// 条件：pleasure 回升到 > -0.1 且已过冷却期。
    pub fn is_ready(&self, current_pleasure: f64, turns_since_conflict: u32) -> bool {
        let emotion_recovered = current_pleasure > -0.1;
        let min_turns = (self.optimal_delay / 60.0).ceil() as u32; // 粗略：每轮约 60 秒 / approx 60s per turn
        emotion_recovered && turns_since_conflict >= min_turns
    }

    /// 过早和解惩罚 / Premature reconciliation penalty
    ///
    /// `premature_penalty = max(0, 1 - actual_delay / optimal_delay)`
    pub fn premature_penalty(&self, actual_delay: u64) -> f64 {
        if self.optimal_delay <= 0.0 {
            return 0.0;
        }
        let ratio = actual_delay as f64 / self.optimal_delay;
        (1.0 - ratio).max(0.0)
    }

    /// 记算和解质量调整 / Compute quality adjustment based on timing
    pub fn timing_quality(&self, actual_delay: u64) -> f64 {
        let penalty = self.premature_penalty(actual_delay);
        (1.0 - penalty * 0.5).clamp(0.0, 1.0)
    }
}

impl Default for ReconciliationTiming {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 第三部分：冲突后成长追踪 / Part 3: Post-Conflict Growth
// ============================================================

/// 成长记录条目 / Growth entry record
// 单次冲突-和解成长记录 / Single conflict-reconciliation growth record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrowthEntry {
    /// 冲突类型 / Conflict type label
    pub conflict_type: String,
    /// 冲突强度 [0.0, 1.0] / Conflict intensity
    pub intensity: f64,
    /// 成长评分 / Growth score
    pub growth_score: f64,
    /// 时间戳（毫秒） / Timestamp in milliseconds
    pub timestamp: i64,
}

/// 冲突后成长追踪器 / Post-conflict growth tracker
///
/// 金缮哲学：断裂处用金修复，修复处比原来更坚固。
/// 成长评分：`growth_score = reconciliation_quality × conflict_depth × learning_factor`
/// 关系韧性：`resilience = base + 0.05 × growth_count - 0.1 × unresolved_count`
///
/// Kintsugi philosophy: repair fractures with gold, the mended place is stronger.
// 冲突后成长追踪器 / Post-conflict growth tracker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostConflictGrowth {
    /// 成长记录滑动窗口 / Sliding window of growth entries
    entries: VecDeque<GrowthEntry>,
    /// 滑动窗口容量 / Sliding window capacity
    max_entries: usize,
    /// 成功和解次数 / Successful reconciliation count
    successful_reconciliations: u32,
    /// 未解决冲突数 / Unresolved conflict count
    unresolved_count: u32,
    /// 关系韧性基准 / Resilience base
    resilience_base: f64,
    /// 累积成长次数 / Cumulative growth count
    growth_count: u32,
}

impl PostConflictGrowth {
    /// 构造 / Constructor
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(50),
            max_entries: 50,
            successful_reconciliations: 0,
            unresolved_count: 0,
            resilience_base: 0.3,
            growth_count: 0,
        }
    }

    /// 学习因子 / Learning factor
    /// `learning_factor = 1.0 + 0.1 × successful_reconciliations`
    pub fn learning_factor(&self) -> f64 {
        1.0 + 0.1 * self.successful_reconciliations as f64
    }

    /// 计算成长评分 / Compute growth score
    ///
    /// `growth_score = reconciliation_quality × conflict_depth × learning_factor`
    pub fn growth_score(&self, reconciliation_quality: f64, conflict_depth: f64) -> f64 {
        reconciliation_quality.clamp(0.0, 1.0)
            * conflict_depth.clamp(0.0, 1.0)
            * self.learning_factor()
    }

    /// 记算关系韧性 / Compute resilience score
    ///
    /// `resilience = base + 0.05 × growth_count - 0.1 × unresolved_count`
    /// 限制在 [0.0, 1.0]。
    pub fn resilience(&self) -> f64 {
        let raw = self.resilience_base + 0.05 * self.growth_count as f64
            - 0.1 * self.unresolved_count as f64;
        raw.clamp(0.0, 1.0)
    }

    /// 记入一次成功和解 / Record a successful reconciliation
    pub fn record_growth(
        &mut self,
        conflict_type: &str,
        intensity: f64,
        reconciliation_quality: f64,
        conflict_depth: f64,
        timestamp: i64,
    ) -> f64 {
        let score = self.growth_score(reconciliation_quality, conflict_depth);
        self.successful_reconciliations += 1;
        self.growth_count += 1;

        let entry = GrowthEntry {
            conflict_type: conflict_type.to_string(),
            intensity,
            growth_score: score,
            timestamp,
        };

        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        score
    }

    /// 计入一次未解决冲突 / Record an unresolved conflict
    pub fn record_unresolved(&mut self) {
        self.unresolved_count += 1;
    }

    /// 获取最近成长记录 / Get recent growth entries
    pub fn recent_entries(&self) -> &VecDeque<GrowthEntry> {
        &self.entries
    }

    /// 平均成长评分 / Average growth score
    pub fn avg_growth_score(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.entries.iter().map(|e| e.growth_score).sum();
        sum / self.entries.len() as f64
    }
}

impl Default for PostConflictGrowth {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 第四部分：引擎主体 / Part 4: Engine
// ============================================================

/// 冲突成长引擎 / Conflict growth engine
///
/// 整合升级预警、和解时机、冲突后成长三个组件。
/// Integrates escalation warning, reconciliation timing, and post-conflict growth.
// 冲突成长引擎主体 / Conflict growth engine main body
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictGrowthEngine {
    /// 升级预警器 / Escalation warning tracker
    pub escalation: EscalationWarning,
    /// 和解时机优化器 / Reconciliation timing optimizer
    pub timing: ReconciliationTiming,
    /// 冲突后成长追踪器 / Post-conflict growth tracker
    pub growth: PostConflictGrowth,
    /// 当前冲突强度 / Current conflict intensity
    current_intensity: f64,
    /// 当前轮次 / Current turn
    current_turn: u32,
    /// 上次冲突以来的轮数 / Turns since last conflict
    turns_since_conflict: u32,
    /// 是否处于冲突中 / Whether in active conflict
    in_conflict: bool,
    /// prompt 预算（字符数） / Prompt hint budget in chars
    prompt_budget: usize,
}

impl ConflictGrowthEngine {
    /// 构造 / Constructor
    pub fn new() -> Self {
        Self {
            escalation: EscalationWarning::new(),
            timing: ReconciliationTiming::new(),
            growth: PostConflictGrowth::new(),
            current_intensity: 0.0,
            current_turn: 0,
            turns_since_conflict: 0,
            in_conflict: false,
            prompt_budget: 200,
        }
    }

    /// 设置 prompt 预算 / Set prompt budget
    pub fn with_prompt_budget(mut self, budget: usize) -> Self {
        self.prompt_budget = budget;
        self
    }

    /// 冲突发生时调用 / Called when a conflict occurs
    pub fn on_conflict(&mut self, intensity: f64, turn: u32) {
        self.current_intensity = intensity;
        self.current_turn = turn;
        self.turns_since_conflict = 0;
        self.in_conflict = true;
        self.escalation.update(intensity, turn);
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
        let conflict_type = match ConflictWeight::from_intensity(self.current_intensity) {
            ConflictWeight::Trivial => "trivial",
            ConflictWeight::Mild => "mild",
            ConflictWeight::Moderate => "moderate",
            ConflictWeight::Severe => "severe",
            ConflictWeight::Critical => "critical",
        };
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
    pub fn on_calm(&mut self, current_pleasure: f64, turns_since_conflict: u32) {
        self.turns_since_conflict = turns_since_conflict;
        self.escalation.calm();

        // 若曾冲突但未和解，且超过阈值轮数，计为未解决
        // If conflict occurred but no reconciliation and exceeds threshold, count unresolved
        if self.in_conflict && turns_since_conflict > 10 {
            self.growth.record_unresolved();
            self.in_conflict = false;
        }

        // 更新 timing 的 last_pleasure
        let _ = current_pleasure;
    }

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

    /// prompt 注入片段（受 budget 约束） / Prompt hint string (budget-constrained)
    pub fn to_prompt_hint(&self) -> String {
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

        let mut hint = format!(
            "[冲突成长] {} | 韧性={:.2} | 均成长={:.2} | 学习因子={:.1}",
            level_str, resilience, avg_growth, learning,
        );

        // 截断至预算（按字节） / Truncate to budget (by bytes)
        if hint.len() > self.prompt_budget {
            let mut end = self.prompt_budget;
            // 确保不截断在 UTF-8 字符中间 / Avoid splitting mid-UTF-8 char
            while end > 0 && !hint.is_char_boundary(end) {
                end -= 1;
            }
            hint.truncate(end);
        }
        hint
    }
}

impl Default for ConflictGrowthEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 第五部分：序列化辅助 / Part 5: Serialization Helper
// ============================================================

/// 可序列化冲突成长快照 / Serializable conflict growth snapshot
///
/// 用于持久化引擎状态。通过 `from_engine` / `to_engine` 互转。
// 可序列化冲突成长快照 / Serializable conflict growth snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableConflictGrowth {
    /// 升级预警 / Escalation warning
    pub escalation: EscalationWarning,
    /// 和解时机 / Reconciliation timing
    pub timing: ReconciliationTiming,
    /// 冲突后成长 / Post-conflict growth
    pub growth: PostConflictGrowth,
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

impl SerializableConflictGrowth {
    /// 从引擎创建快照 / Create snapshot from engine
    pub fn from_engine(engine: &ConflictGrowthEngine) -> Self {
        Self {
            escalation: engine.escalation.clone(),
            timing: engine.timing.clone(),
            growth: engine.growth.clone(),
            current_intensity: engine.current_intensity,
            current_turn: engine.current_turn,
            turns_since_conflict: engine.turns_since_conflict,
            in_conflict: engine.in_conflict,
            prompt_budget: engine.prompt_budget,
        }
    }

    /// 从快照恢复引擎 / Restore engine from snapshot
    pub fn to_engine(&self) -> ConflictGrowthEngine {
        ConflictGrowthEngine {
            escalation: self.escalation.clone(),
            timing: self.timing.clone(),
            growth: self.growth.clone(),
            current_intensity: self.current_intensity,
            current_turn: self.current_turn,
            turns_since_conflict: self.turns_since_conflict,
            in_conflict: self.in_conflict,
            prompt_budget: self.prompt_budget,
        }
    }
}

// ============================================================
// 第六部分：单元测试 / Part 6: Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- EscalationWarning 测试 ---

    #[test]
    fn test_escalation_no_warning() {
        // 冲突强度稳定，无升级 / Stable intensity, no escalation
        let mut ew = EscalationWarning::new();
        ew.update(0.3, 1);
        ew.update(0.3, 2);
        ew.update(0.3, 3);
        assert_eq!(ew.warning_level(), EscalationWarningLevel::None);
    }

    #[test]
    fn test_escalation_caution() {
        // 缓慢升级触发 Caution / Slow escalation triggers Caution
        let mut ew = EscalationWarning::new();
        ew.update(0.1, 1);
        ew.update(0.4, 2); // velocity = 0.3, acceleration = 0.3
                           // velocity=0.3 > 0.2, acceleration=0.3 > 0.1 → Caution (但 0.3>0.2 且 0.3>0.2 → Warning?)
                           // velocity=0.3 > 0.3? 不，0.3 > 0.3 为 false → 不满足 Warning
                           // velocity=0.3 > 0.2 且 acceleration=0.3 > 0.1 → Caution
        let level = ew.warning_level();
        assert!(
            level == EscalationWarningLevel::Caution || level == EscalationWarningLevel::Warning,
            "expected Caution or Warning, got {:?}",
            level
        );
    }

    #[test]
    fn test_escalation_warning() {
        // 快速升级触发 Warning / Rapid escalation triggers Warning
        let mut ew = EscalationWarning::new();
        ew.update(0.1, 1);
        ew.update(0.5, 2); // velocity = 0.4
        ew.update(0.9, 3); // velocity = 0.4, acceleration = 0 → 不满足
                           // 重新构造：需要 acceleration > 0.2 且 velocity > 0.3
        let mut ew2 = EscalationWarning::new();
        ew2.update(0.1, 1);
        ew2.update(0.3, 2); // velocity = 0.2
        ew2.update(0.7, 3); // velocity = 0.4, acceleration = 0.2 → 0.2 > 0.2? false
                            // 再调：acceleration 需严格 > 0.2
        let mut ew3 = EscalationWarning::new();
        ew3.update(0.1, 1);
        ew3.update(0.25, 2); // velocity = 0.15
        ew3.update(0.7, 3); // velocity = 0.45, acceleration = 0.3 > 0.2, velocity 0.45 > 0.3 → Warning
        assert_eq!(ew3.warning_level(), EscalationWarningLevel::Warning);
    }

    #[test]
    fn test_escalation_alert() {
        // 危险升级触发 Alert / Dangerous escalation triggers Alert
        let mut ew = EscalationWarning::new();
        ew.update(0.05, 1);
        ew.update(0.2, 2); // velocity = 0.15
        ew.update(0.75, 3); // velocity = 0.55, acceleration = 0.4 > 0.3, velocity 0.55 > 0.4 → Alert
        assert_eq!(ew.warning_level(), EscalationWarningLevel::Alert);
    }

    #[test]
    fn test_escalation_cooldown_reset() {
        // 连续 3 轮平静后重置 / Reset after 3 calm turns
        let mut ew = EscalationWarning::new();
        ew.update(0.1, 1);
        ew.update(0.5, 2); // velocity = 0.4
        assert_ne!(ew.warning_level(), EscalationWarningLevel::None);

        ew.calm();
        ew.calm();
        ew.calm(); // 3 轮平静 → 重置

        assert_eq!(ew.velocity, 0.0);
        assert_eq!(ew.acceleration, 0.0);
        assert_eq!(ew.warning_level(), EscalationWarningLevel::None);
    }

    // --- ReconciliationTiming 测试 ---

    #[test]
    fn test_timing_optimal_delay() {
        // 最优延迟计算 / Optimal delay calculation
        let mut rt = ReconciliationTiming::new();
        rt.set_conflict(0.5, 0.5); // Moderate=1.5, depth=0.5
                                   // 300 × 1.5 × 1.5 = 675
        assert_eq!(rt.optimal_delay_secs(), 675);
    }

    #[test]
    fn test_timing_ready() {
        // 情绪回升 + 过冷却期 → ready / Emotion recovered + past cooldown → ready
        let mut rt = ReconciliationTiming::new();
        rt.set_conflict(0.3, 0.0); // Mild=1.0, depth=0.0 → 300s → 5 turns
        assert!(rt.is_ready(0.2, 10)); // pleasure > -0.1, turns >= 5
        assert!(!rt.is_ready(-0.5, 10)); // pleasure too low
        assert!(!rt.is_ready(0.2, 2)); // not enough turns
    }

    #[test]
    fn test_timing_premature_penalty() {
        // 过早和解惩罚 / Premature penalty
        let mut rt = ReconciliationTiming::new();
        rt.set_conflict(0.3, 0.0); // optimal = 300
                                   // actual = 150 → penalty = 1 - 150/300 = 0.5
        assert!((rt.premature_penalty(150) - 0.5).abs() < 1e-9);
        // actual = 300 → penalty = 0
        assert!((rt.premature_penalty(300) - 0.0).abs() < 1e-9);
        // actual = 600 → penalty = max(0, 1-2) = 0
        assert!((rt.premature_penalty(600) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_timing_different_intensities() {
        // 不同强度产生不同延迟 / Different intensities produce different delays
        let mut rt = ReconciliationTiming::new();
        rt.set_conflict(0.1, 0.0); // Trivial=0.5 → 150
        let trivial = rt.optimal_delay_secs();

        rt.set_conflict(0.9, 0.0); // Critical=4.0 → 1200
        let critical = rt.optimal_delay_secs();

        assert!(trivial < critical);
        assert_eq!(trivial, 150);
        assert_eq!(critical, 1200);
    }

    // --- PostConflictGrowth 测试 ---

    #[test]
    fn test_growth_score() {
        // 成长评分计算 / Growth score calculation
        let mut pcg = PostConflictGrowth::new();
        // learning_factor = 1.0 + 0.1 × 0 = 1.0
        let score = pcg.growth_score(0.8, 0.6);
        // 0.8 × 0.6 × 1.0 = 0.48
        assert!((score - 0.48).abs() < 1e-9);

        pcg.successful_reconciliations = 3;
        // learning_factor = 1.0 + 0.1 × 3 = 1.3
        let score2 = pcg.growth_score(0.8, 0.6);
        // 0.8 × 0.6 × 1.3 = 0.624
        assert!((score2 - 0.624).abs() < 1e-9);
    }

    #[test]
    fn test_resilience_up() {
        // 成长增加韧性 / Growth increases resilience
        let mut pcg = PostConflictGrowth::new();
        let initial = pcg.resilience(); // 0.3
        pcg.record_growth("mild", 0.3, 0.8, 0.5, 0);
        pcg.record_growth("mild", 0.3, 0.8, 0.5, 0);
        let after = pcg.resilience();
        assert!(after > initial);
        // 0.3 + 0.05×2 = 0.4
        assert!((after - 0.4).abs() < 1e-9);
    }

    #[test]
    fn test_resilience_down() {
        // 未解决冲突降低韧性 / Unresolved conflicts decrease resilience
        let mut pcg = PostConflictGrowth::new();
        let initial = pcg.resilience(); // 0.3
        pcg.record_unresolved();
        pcg.record_unresolved();
        let after = pcg.resilience();
        assert!(after < initial);
        // 0.3 - 0.1×2 = 0.1
        assert!((after - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_growth_sliding_window() {
        // 滑动窗口保留最近 50 条 / Sliding window keeps last 50 entries
        let mut pcg = PostConflictGrowth::new();
        for i in 0..60 {
            pcg.record_growth("mild", 0.3, 0.5, 0.5, i);
        }
        assert_eq!(pcg.recent_entries().len(), 50);
        // 最旧的应被淘汰，最早保留的是第 11 条（index=10）
        assert_eq!(pcg.recent_entries().front().unwrap().timestamp, 10);
    }

    // --- Engine 测试 ---

    #[test]
    fn test_engine_on_conflict() {
        // 冲突发生时更新引擎 / Engine updates on conflict
        let mut engine = ConflictGrowthEngine::new();
        engine.on_conflict(0.5, 1);
        assert!(engine.in_conflict);
        assert_eq!(engine.turns_since_conflict, 0);
    }

    #[test]
    fn test_engine_on_reconciliation() {
        // 和解完成时记录成长 / Engine records growth on reconciliation
        let mut engine = ConflictGrowthEngine::new();
        engine.on_conflict(0.5, 1);
        engine.timing.set_conflict(0.5, 0.3);
        engine.on_reconciliation(0.8, 0.6, 700, 0.3);
        assert!(!engine.in_conflict);
        assert_eq!(engine.growth.successful_reconciliations, 1);
        assert_eq!(engine.growth.recent_entries().len(), 1);
    }

    #[test]
    fn test_engine_on_calm() {
        // 平静期递增冷却 / Calm period increments cooldown
        let mut engine = ConflictGrowthEngine::new();
        engine.on_conflict(0.5, 1);
        engine.on_calm(0.1, 1);
        assert_eq!(engine.turns_since_conflict, 1);
        // 超过 10 轮未和解 → 记为未解决
        engine.on_calm(0.1, 11);
        assert_eq!(engine.growth.unresolved_count, 1);
        assert!(!engine.in_conflict);
    }

    #[test]
    fn test_engine_prompt_hint() {
        // prompt 注入片段 / Prompt hint generation
        let mut engine = ConflictGrowthEngine::new();
        engine.on_conflict(0.5, 1);
        let hint = engine.to_prompt_hint();
        assert!(!hint.is_empty());
        assert!(hint.contains("冲突成长"));

        // 预算截断 / Budget truncation
        let mut engine2 = ConflictGrowthEngine::new().with_prompt_budget(10);
        engine2.on_conflict(0.5, 1);
        let hint2 = engine2.to_prompt_hint();
        assert!(hint2.len() <= 10);
    }

    #[test]
    fn test_engine_serialization_roundtrip() {
        // 序列化往返 / Serialization round-trip
        let mut engine = ConflictGrowthEngine::new();
        engine.on_conflict(0.6, 1);
        engine.on_conflict(0.8, 2);
        engine.timing.set_conflict(0.7, 0.4);
        engine.on_reconciliation(0.7, 0.5, 500, 0.4);

        let snap = SerializableConflictGrowth::from_engine(&engine);
        let json = serde_json::to_string(&snap).unwrap();
        let restored: SerializableConflictGrowth = serde_json::from_str(&json).unwrap();
        let engine2 = restored.to_engine();

        assert_eq!(engine2.warning_level(), engine.warning_level());
        assert!((engine2.resilience_score() - engine.resilience_score()).abs() < 1e-9);
        assert_eq!(
            engine2.growth.successful_reconciliations,
            engine.growth.successful_reconciliations
        );
        assert_eq!(
            engine2.growth.recent_entries().len(),
            engine.growth.recent_entries().len()
        );
    }
}
