// 冲突成长组件 / Conflict Growth Components
//
// SPDX-License-Identifier: MIT
//! 冲突成长组件 — 升级预警、和解时机、冲突后成长
//! Conflict Growth Components — Escalation warning, reconciliation timing, post-conflict growth.
//!
//! 金缮哲学：断裂处用金修复，修复处比原来更坚固。
//! Kintsugi philosophy: repair fractures with gold, the mended place is stronger.

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
    pub prev_intensity: f64,
    /// 上一轮轮次 / Previous turn
    pub prev_turn: u32,
    /// 当前升级速度 / Current escalation velocity
    pub velocity: f64,
    /// 当前加速度 / Current acceleration
    pub acceleration: f64,
    /// 连续无冲突轮数 / Consecutive calm turns
    pub calm_turns: u32,
    /// 是否已初始化 / Whether initialized
    pub initialized: bool,
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

    /// 转为字符串标签 / Convert to string label
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trivial => "trivial",
            Self::Mild => "mild",
            Self::Moderate => "moderate",
            Self::Severe => "severe",
            Self::Critical => "critical",
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
    pub base_cooldown: f64,
    /// 冲突权重 / Conflict weight
    pub conflict_weight: ConflictWeight,
    /// 关系深度 [0.0, 1.0] / Relationship depth
    pub relationship_depth: f64,
    /// 上次冲突时 pleasure 值 / Pleasure at last conflict
    pub last_pleasure: f64,
    /// 最优延迟（秒） / Optimal delay in seconds
    pub optimal_delay: f64,
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

    /// 计算和解质量调整 / Compute quality adjustment based on timing
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
    pub entries: VecDeque<GrowthEntry>,
    /// 滑动窗口容量 / Sliding window capacity
    pub max_entries: usize,
    /// 成功和解次数 / Successful reconciliation count
    pub successful_reconciliations: u32,
    /// 未解决冲突数 / Unresolved conflict count
    pub unresolved_count: u32,
    /// 关系韧性基准 / Resilience base
    pub resilience_base: f64,
    /// 累积成长次数 / Cumulative growth count
    pub growth_count: u32,
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

    /// 计算关系韧性 / Compute resilience score
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
