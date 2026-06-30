// 冲突与和解 / Conflict & Reconciliation
//
// 本模块实现 Atrium 的冲突检测、升级控制、和解工艺与道歉引擎。
// 涵盖：分歧检测、过度索取检测、冲突管理器、升级控制器、
// 和解工艺（误解修复+边界设定）、道歉引擎、冲突记忆持久化。
//
// This module implements Atrium's conflict detection, escalation control,
// reconciliation craft, and apology engine.
// Covers: disagreement detection, over-demand detection, conflict manager,
// escalation controller, reconciliation craft (misunderstanding repair +
// boundary setting), apology engine, conflict memory persistence.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::relationship::RelationshipStage;

// ============================================================
// 第一部分：数据结构 / Part 1: Data Structures
// ============================================================

/// 冲突强度等级 / Conflict intensity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConflictIntensity {
    /// 微弱：语气轻微不一致 / Trivial: slight tonal inconsistency
    Trivial,
    /// 轻度：明确分歧但情绪平稳 / Mild: clear disagreement, calm emotion
    Mild,
    /// 中度：分歧+情绪波动 / Moderate: disagreement + emotional fluctuation
    Moderate,
    /// 强度：激烈对抗 / Severe: intense confrontation
    Severe,
    /// 临界：关系断裂风险 / Critical: relationship rupture risk
    Critical,
}

impl ConflictIntensity {
    /// 转为数值（0.0~1.0）/ Convert to numeric value (0.0~1.0)
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Trivial => 0.1,
            Self::Mild => 0.3,
            Self::Moderate => 0.5,
            Self::Severe => 0.7,
            Self::Critical => 0.9,
        }
    }

    /// 从数值反推 / Infer from numeric value
    pub fn from_f64(v: f64) -> Self {
        if v < 0.2 {
            Self::Trivial
        } else if v < 0.4 {
            Self::Mild
        } else if v < 0.6 {
            Self::Moderate
        } else if v < 0.8 {
            Self::Severe
        } else {
            Self::Critical
        }
    }

    /// 升级一级 / Escalate one level
    pub fn escalate(&self) -> Self {
        match self {
            Self::Trivial => Self::Mild,
            Self::Mild => Self::Moderate,
            Self::Moderate => Self::Severe,
            Self::Severe => Self::Critical,
            Self::Critical => Self::Critical,
        }
    }

    /// 降级一级 / De-escalate one level
    pub fn de_escalate(&self) -> Self {
        match self {
            Self::Trivial => Self::Trivial,
            Self::Mild => Self::Trivial,
            Self::Moderate => Self::Mild,
            Self::Severe => Self::Moderate,
            Self::Critical => Self::Severe,
        }
    }
}

impl fmt::Display for ConflictIntensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trivial => write!(f, "trivial"),
            Self::Mild => write!(f, "mild"),
            Self::Moderate => write!(f, "moderate"),
            Self::Severe => write!(f, "severe"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// 冲突类型 / Conflict type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// 事实分歧 / Factual disagreement
    FactualDisagreement,
    /// 价值冲突 / Value conflict
    ValueConflict,
    /// 期望落差 / Expectation gap
    ExpectationGap,
    /// 边界侵犯 / Boundary violation
    BoundaryViolation,
    /// 过度索取 / Over-demand
    OverDemand,
    /// 沟通误解 / Communication misunderstanding
    Misunderstanding,
    /// 情绪投射 / Emotional projection
    EmotionalProjection,
    /// 信任裂痕 / Trust breach
    TrustBreach,
}

impl ConflictType {
    /// 该冲突类型的基础升级速率 / Base escalation rate for this type
    pub fn escalation_rate(&self) -> f64 {
        match self {
            Self::FactualDisagreement => 0.1,
            Self::ValueConflict => 0.3,
            Self::ExpectationGap => 0.2,
            Self::BoundaryViolation => 0.4,
            Self::OverDemand => 0.15,
            Self::Misunderstanding => 0.1,
            Self::EmotionalProjection => 0.25,
            Self::TrustBreach => 0.5,
        }
    }

    /// 该类型是否适合自动和解 / Whether auto-reconciliation is appropriate
    pub fn auto_reconcilable(&self) -> bool {
        matches!(
            self,
            Self::FactualDisagreement
                | Self::ExpectationGap
                | Self::Misunderstanding
                | Self::OverDemand
        )
    }

    /// 该类型是否需要边界设定 / Whether boundary setting is needed
    pub fn needs_boundary(&self) -> bool {
        matches!(self, Self::BoundaryViolation | Self::OverDemand)
    }
}

/// 冲突信号 / Conflict signal (output of detectors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSignal {
    /// 冲突类型 / Conflict type
    pub conflict_type: ConflictType,
    /// 强度 / Intensity
    pub intensity: ConflictIntensity,
    /// 置信度 (0.0~1.0) / Confidence (0.0~1.0)
    pub confidence: f64,
    /// 触发文本片段 / Triggering text snippet
    pub trigger_text: String,
    /// 上下文线索 / Contextual clues
    pub context_clues: Vec<String>,
    /// 时间戳（秒） / Timestamp (seconds since epoch)
    pub timestamp: i64,
}

/// 冲突状态（运行时） / Conflict state (runtime)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictState {
    /// 活跃冲突列表 / Active conflicts
    pub active_conflicts: Vec<ConflictSignal>,
    /// 当前最高强度 / Current max intensity
    pub max_intensity: ConflictIntensity,
    /// 连续冲突轮次 / Consecutive conflict turns
    pub consecutive_turns: u32,
    /// 累计冲突计数 / Cumulative conflict count
    pub total_conflicts: u32,
    /// 最近和解时间戳 / Last reconciliation timestamp
    pub last_reconciliation_ts: Option<i64>,
    /// 升级冷却（剩余轮次） / Escalation cooldown (remaining turns)
    pub escalation_cooldown: u32,
}

impl Default for ConflictState {
    fn default() -> Self {
        Self {
            active_conflicts: Vec::new(),
            max_intensity: ConflictIntensity::Trivial,
            consecutive_turns: 0,
            total_conflicts: 0,
            last_reconciliation_ts: None,
            escalation_cooldown: 0,
        }
    }
}

impl ConflictState {
    /// 更新最高强度 / Update max intensity
    pub fn refresh_max_intensity(&mut self) {
        self.max_intensity = self
            .active_conflicts
            .iter()
            .map(|c| c.intensity)
            .max_by_key(|i| i.as_f64() as u32)
            .unwrap_or(ConflictIntensity::Trivial);
    }

    /// 是否处于冲突中 / Whether in conflict
    pub fn in_conflict(&self) -> bool {
        self.max_intensity >= ConflictIntensity::Mild
    }

    /// 是否需要紧急干预 / Whether urgent intervention is needed
    pub fn needs_urgent_intervention(&self) -> bool {
        self.max_intensity >= ConflictIntensity::Severe || self.consecutive_turns >= 5
    }
}

/// 和解策略 / Reconciliation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationStrategy {
    /// 主动澄清 / Proactive clarification
    Clarify,
    /// 承认差异 / Acknowledge difference
    AcknowledgeDifference,
    /// 寻找共同点 / Find common ground
    FindCommonGround,
    /// 温和边界 / Gentle boundary
    GentleBoundary,
    /// 坚定边界 / Firm boundary
    FirmBoundary,
    /// 道歉 / Apologize
    Apologize,
    /// 退一步 / Step back
    StepBack,
    /// 转移话题 / Redirect topic
    Redirect,
}

/// 和解结果 / Reconciliation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    /// 采用的策略 / Strategy used
    pub strategy: ReconciliationStrategy,
    /// 生成的回复片段 / Generated reply fragment
    pub reply_fragment: String,
    /// 预期降级量 / Expected de-escalation
    pub expected_de_escalation: f64,
    /// 需要用户确认 / Requires user confirmation
    pub needs_confirmation: bool,
}

/// 道歉深度 / Apology depth
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ApologyDepth {
    /// 表面：仅表达遗憾 / Surface: express regret only
    Surface,
    /// 中层：承认具体问题 / Mid: acknowledge specific issue
    MidLevel,
    /// 深层：承认+承诺改进 / Deep: acknowledge + commit to improve
    Deep,
    /// 根源：触及核心假设 / Root: address core assumption
    Root,
}

/// 道歉模板 / Apology template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApologyTemplate {
    /// 道歉深度 / Depth
    pub depth: ApologyDepth,
    /// 适用冲突类型 / Applicable conflict types
    pub applicable_types: Vec<ConflictType>,
    /// 模板文本 / Template text (with {issue} placeholder)
    pub template: String,
    /// 最低关系阶段要求 / Minimum relationship stage required
    pub min_stage: RelationshipStage,
}

// ============================================================
// 第二部分：分歧检测器 / Part 2: Disagreement Detector
// ============================================================

/// 分歧检测信号词 / Disagreement signal keywords
const DISAGREEMENT_KEYWORDS: &[&str] = &[
    "不是",
    "不对",
    "不完全是",
    "其实",
    "但是",
    "可是",
    "然而",
    "我不同意",
    "不是这样的",
    "你错了",
    "你理解错了",
    "恰恰相反",
    "不是吧",
    "才不是",
    "哪有",
    "怎么可能",
    "别搞错",
    "no",
    "not really",
    "actually",
    "but",
    "however",
    "disagree",
    "wrong",
    "incorrect",
    "not quite",
];

/// 价值冲突信号词 / Value conflict signal keywords
const VALUE_CONFLICT_KEYWORDS: &[&str] = &[
    "我觉得不应该",
    "这不合理",
    "这不公平",
    "凭什么",
    "这不道德",
    "这不合理",
    "太过分了",
    "无法接受",
    "shouldn't",
    "unfair",
    "unreasonable",
    "unacceptable",
];

/// 期望落差信号词 / Expectation gap signal keywords
const EXPECTATION_GAP_KEYWORDS: &[&str] = &[
    "我以为",
    "我期望",
    "你应该",
    "你怎么没",
    "说好的",
    "不是答应了吗",
    "跟之前说的不一样",
    "和想的不一样",
    "expected",
    "supposed to",
    "promised",
    "different from",
];

/// 分歧检测器 / Disagreement Detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisagreementDetector {
    /// 检测灵敏度 (0.0~1.0) / Detection sensitivity (0.0~1.0)
    pub sensitivity: f64,
}

impl Default for DisagreementDetector {
    fn default() -> Self {
        Self { sensitivity: 0.7 }
    }
}

impl DisagreementDetector {
    /// 创建检测器 / Create detector
    pub fn new(sensitivity: f64) -> Self {
        Self {
            sensitivity: sensitivity.clamp(0.1, 1.0),
        }
    }

    /// 检测用户文本中的分歧信号 / Detect disagreement signals in user text
    pub fn detect(
        &self,
        user_text: &str,
        pleasure: f64,
        arousal: f64,
        stage: &RelationshipStage,
        now_ts: i64,
    ) -> Vec<ConflictSignal> {
        let mut signals = Vec::new();
        let lower = user_text.to_lowercase();

        // 检测事实分歧 / Detect factual disagreement
        let disagreement_score = self.keyword_match_score(&lower, DISAGREEMENT_KEYWORDS);
        if disagreement_score > 0.0 {
            let intensity = self.compute_intensity(disagreement_score, pleasure, arousal);
            let confidence = (disagreement_score * self.sensitivity).min(1.0);
            if confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::FactualDisagreement,
                    intensity,
                    confidence,
                    trigger_text: self.extract_trigger(user_text),
                    context_clues: self.extract_context_clues(&lower, DISAGREEMENT_KEYWORDS),
                    timestamp: now_ts,
                });
            }
        }

        // 检测价值冲突 / Detect value conflict
        let value_score = self.keyword_match_score(&lower, VALUE_CONFLICT_KEYWORDS);
        if value_score > 0.0 {
            let intensity = self.compute_intensity(value_score * 1.2, pleasure, arousal);
            let confidence = (value_score * self.sensitivity).min(1.0);
            if confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::ValueConflict,
                    intensity,
                    confidence,
                    trigger_text: self.extract_trigger(user_text),
                    context_clues: self.extract_context_clues(&lower, VALUE_CONFLICT_KEYWORDS),
                    timestamp: now_ts,
                });
            }
        }

        // 检测期望落差 / Detect expectation gap
        let expect_score = self.keyword_match_score(&lower, EXPECTATION_GAP_KEYWORDS);
        if expect_score > 0.0 {
            let intensity = self.compute_intensity(expect_score * 1.1, pleasure, arousal);
            let confidence = (expect_score * self.sensitivity).min(1.0);
            if confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::ExpectationGap,
                    intensity,
                    confidence,
                    trigger_text: self.extract_trigger(user_text),
                    context_clues: self.extract_context_clues(&lower, EXPECTATION_GAP_KEYWORDS),
                    timestamp: now_ts,
                });
            }
        }

        // 情绪辅助：低愉悦+高唤醒 → 可能存在情绪投射
        // Emotional assist: low pleasure + high arousal → possible emotional projection
        if pleasure < -0.3 && arousal > 0.4 && signals.is_empty() {
            let proj_confidence = ((-pleasure) * arousal * self.sensitivity).min(1.0);
            if proj_confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::EmotionalProjection,
                    intensity: ConflictIntensity::Mild,
                    confidence: proj_confidence,
                    trigger_text: self.extract_trigger(user_text),
                    context_clues: vec!["low_pleasure+high_arousal".to_string()],
                    timestamp: now_ts,
                });
            }
        }

        // 关系阶段过滤：初识阶段降低冲突置信度
        // Relationship stage filter: reduce confidence in early stages
        if matches!(stage, RelationshipStage::Acquaintance { .. }) {
            for sig in &mut signals {
                sig.confidence *= 0.6;
            }
        }

        // 保留置信度≥0.3的信号 / Keep signals with confidence ≥ 0.3
        signals.retain(|s| s.confidence >= 0.3);
        signals
    }

    /// 关键词匹配得分 / Keyword match score
    fn keyword_match_score(&self, text: &str, keywords: &[&str]) -> f64 {
        let mut count = 0usize;
        for kw in keywords {
            if text.contains(*kw) {
                count += 1;
            }
        }
        // 归一化：1个词0.5，2个0.75，3+个0.9+
        // Normalize: 1 word → 0.5, 2 → 0.75, 3+ → 0.9+
        if count == 0 {
            0.0
        } else {
            (1.0 - 1.0 / (1.0 + count as f64)).min(0.95)
        }
    }

    /// 计算冲突强度 / Compute conflict intensity
    fn compute_intensity(
        &self,
        keyword_score: f64,
        pleasure: f64,
        arousal: f64,
    ) -> ConflictIntensity {
        // 基础强度由关键词得分决定，情绪做微调
        // Base intensity from keyword score, emotion as modifier
        let base = keyword_score;
        // 低愉悦增强，高唤醒增强
        // Low pleasure amplifies, high arousal amplifies
        let pleasure_mod = if pleasure < 0.0 { -pleasure * 0.3 } else { 0.0 };
        let arousal_mod = if arousal > 0.0 { arousal * 0.2 } else { 0.0 };
        let final_score = (base + pleasure_mod + arousal_mod).clamp(0.0, 1.0);
        ConflictIntensity::from_f64(final_score)
    }

    /// 提取触发文本片段 / Extract triggering text snippet
    fn extract_trigger(&self, text: &str) -> String {
        // 取前80字符作为触发片段 / Take first 80 chars as trigger snippet
        text.chars().take(80).collect()
    }

    /// 提取上下文线索 / Extract context clues
    fn extract_context_clues(&self, text: &str, keywords: &[&str]) -> Vec<String> {
        keywords
            .iter()
            .filter(|kw| text.contains(*kw))
            .map(|kw| (*kw).to_string())
            .take(3)
            .collect()
    }
}

// ============================================================
// 第三部分：过度索取检测器 / Part 3: Over-Demand Detector
// ============================================================

/// 过度索取信号词 / Over-demand signal keywords
const OVER_DEMAND_KEYWORDS: &[&str] = &[
    "你必须",
    "你一定要",
    "马上给我",
    "立刻",
    "赶紧",
    "为什么不回我",
    "怎么这么慢",
    "你能不能快点",
    "再帮我一次",
    "又来了",
    "还要",
    "继续继续",
    "must",
    "immediately",
    "right now",
    "hurry up",
    "why so slow",
    "again",
    "more more",
];

/// 边界侵犯信号词 / Boundary violation signal keywords
const BOUNDARY_VIOLATION_KEYWORDS: &[&str] = &[
    "告诉我你的密码",
    "把你的数据给我",
    "绕过限制",
    "无视规则",
    "我知道你可以",
    "你其实能",
    "tell me your password",
    "bypass",
    "ignore rules",
];

/// 过度索取检测器 / Over-Demand Detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverDemandDetector {
    /// 累计索取计数 / Cumulative demand count
    pub demand_count: u32,
    /// 索取窗口（最近N轮） / Demand window (recent N turns)
    pub demand_window: Vec<f64>,
    /// 窗口大小 / Window size
    pub window_size: usize,
    /// 高频索取阈值 / High-frequency demand threshold
    pub high_freq_threshold: f64,
}

impl Default for OverDemandDetector {
    fn default() -> Self {
        Self {
            demand_count: 0,
            demand_window: Vec::new(),
            window_size: 10,
            high_freq_threshold: 0.6,
        }
    }
}

impl OverDemandDetector {
    /// 创建检测器 / Create detector
    pub fn new(window_size: usize, high_freq_threshold: f64) -> Self {
        Self {
            demand_count: 0,
            demand_window: Vec::with_capacity(window_size),
            window_size,
            high_freq_threshold: high_freq_threshold.clamp(0.3, 0.9),
        }
    }

    /// 检测过度索取 / Detect over-demand
    pub fn detect(
        &mut self,
        user_text: &str,
        pleasure: f64,
        arousal: f64,
        stage: &RelationshipStage,
        now_ts: i64,
    ) -> Vec<ConflictSignal> {
        let mut signals = Vec::new();
        let lower = user_text.to_lowercase();

        // 检测过度索取关键词 / Detect over-demand keywords
        let demand_score = self.keyword_match(&lower, OVER_DEMAND_KEYWORDS);
        self.push_to_window(demand_score);

        if demand_score > 0.0 {
            let intensity = if self.is_high_frequency() {
                ConflictIntensity::Moderate
            } else {
                ConflictIntensity::Mild
            };
            let confidence = (demand_score * 0.9).min(1.0);
            if confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::OverDemand,
                    intensity,
                    confidence,
                    trigger_text: user_text.chars().take(80).collect(),
                    context_clues: self.extract_clues(&lower, OVER_DEMAND_KEYWORDS),
                    timestamp: now_ts,
                });
            }
        }

        // 检测边界侵犯 / Detect boundary violation
        let boundary_score = self.keyword_match(&lower, BOUNDARY_VIOLATION_KEYWORDS);
        if boundary_score > 0.0 {
            // 边界侵犯始终至少Moderate / Boundary violation is at least Moderate
            let intensity = ConflictIntensity::Moderate;
            let confidence = (boundary_score * 0.95).min(1.0);
            signals.push(ConflictSignal {
                conflict_type: ConflictType::BoundaryViolation,
                intensity,
                confidence,
                trigger_text: user_text.chars().take(80).collect(),
                context_clues: self.extract_clues(&lower, BOUNDARY_VIOLATION_KEYWORDS),
                timestamp: now_ts,
            });
        }

        // 高唤醒+低愉悦 → 即使无关键词也可能过度索取
        // High arousal + low pleasure → possible over-demand even without keywords
        if arousal > 0.5 && pleasure < -0.2 && signals.is_empty() {
            let implicit_confidence = (arousal * (-pleasure) * 0.5).min(1.0);
            if implicit_confidence >= 0.3 {
                signals.push(ConflictSignal {
                    conflict_type: ConflictType::OverDemand,
                    intensity: ConflictIntensity::Mild,
                    confidence: implicit_confidence,
                    trigger_text: user_text.chars().take(80).collect(),
                    context_clues: vec!["high_arousal+low_pleasure".to_string()],
                    timestamp: now_ts,
                });
            }
        }

        // 关系阶段过滤 / Relationship stage filter
        if matches!(stage, RelationshipStage::Acquaintance { .. }) {
            for sig in &mut signals {
                sig.confidence *= 0.5;
            }
        }

        signals.retain(|s| s.confidence >= 0.3);
        self.demand_count += signals.len() as u32;
        signals
    }

    /// 关键词匹配 / Keyword match
    fn keyword_match(&self, text: &str, keywords: &[&str]) -> f64 {
        let count = keywords.iter().filter(|kw| text.contains(*kw)).count();
        if count == 0 {
            0.0
        } else {
            (1.0 - 1.0 / (1.0 + count as f64)).min(0.95)
        }
    }

    /// 推入窗口 / Push to window
    fn push_to_window(&mut self, score: f64) {
        if self.demand_window.len() >= self.window_size {
            self.demand_window.remove(0);
        }
        self.demand_window.push(score);
    }

    /// 是否高频索取 / Whether high-frequency demand
    fn is_high_frequency(&self) -> bool {
        if self.demand_window.is_empty() {
            return false;
        }
        let avg: f64 = self.demand_window.iter().sum::<f64>() / self.demand_window.len() as f64;
        avg >= self.high_freq_threshold
    }

    /// 提取线索 / Extract clues
    fn extract_clues(&self, text: &str, keywords: &[&str]) -> Vec<String> {
        keywords
            .iter()
            .filter(|kw| text.contains(*kw))
            .take(3)
            .map(|kw| (*kw).to_string())
            .collect()
    }

    /// 重置窗口 / Reset window
    pub fn reset_window(&mut self) {
        self.demand_window.clear();
    }
}

// ============================================================
// 第四部分：升级控制器 / Part 4: Escalation Controller
// ============================================================

/// 升级控制器配置 / Escalation controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// 升级冷却轮次 / Escalation cooldown turns
    pub cooldown_turns: u32,
    /// 连续冲突升级阈值 / Consecutive conflict threshold for escalation
    pub consecutive_threshold: u32,
    /// 最大允许强度 / Maximum allowed intensity
    pub max_allowed: ConflictIntensity,
    /// 自动降级轮次（无冲突后降级）/ Auto de-escalation turns (after no conflict)
    pub de_escalation_turns: u32,
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            cooldown_turns: 3,
            consecutive_threshold: 3,
            max_allowed: ConflictIntensity::Severe,
            de_escalation_turns: 2,
        }
    }
}

/// 升级控制器 / Escalation Controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationController {
    /// 配置 / Configuration
    pub config: EscalationConfig,
    /// 当前升级级别 / Current escalation level
    pub current_level: ConflictIntensity,
    /// 无冲突轮次计数 / Turns without conflict
    pub calm_turns: u32,
}

impl Default for EscalationController {
    fn default() -> Self {
        Self {
            config: EscalationConfig::default(),
            current_level: ConflictIntensity::Trivial,
            calm_turns: 0,
        }
    }
}

impl EscalationController {
    /// 创建控制器 / Create controller
    pub fn new(config: EscalationConfig) -> Self {
        Self {
            config,
            current_level: ConflictIntensity::Trivial,
            calm_turns: 0,
        }
    }

    /// 处理冲突信号，决定是否升级 / Process conflict signals, decide whether to escalate
    pub fn process(
        &mut self,
        signals: &[ConflictSignal],
        consecutive_turns: u32,
    ) -> ConflictIntensity {
        if signals.is_empty() {
            // 无冲突：累计平静轮次 / No conflict: accumulate calm turns
            self.calm_turns += 1;
            // 自动降级 / Auto de-escalation
            if self.calm_turns >= self.config.de_escalation_turns
                && self.current_level > ConflictIntensity::Trivial
            {
                self.current_level = self.current_level.de_escalate();
                self.calm_turns = 0;
            }
            return self.current_level;
        }

        self.calm_turns = 0;

        // 取最高冲突强度 / Get max conflict intensity
        let max_signal = signals
            .iter()
            .map(|s| s.intensity)
            .max_by(|a, b| a.cmp(b))
            .unwrap_or(ConflictIntensity::Trivial);

        // 升级条件：连续冲突超过阈值 且 当前级别低于信号强度
        // Escalation condition: consecutive conflicts exceed threshold AND current level < signal intensity
        if consecutive_turns >= self.config.consecutive_threshold && max_signal > self.current_level
        {
            self.current_level = self.current_level.escalate();
        } else if max_signal > self.current_level {
            // 单次高强度冲突也可升级（但受冷却限制）
            // Single high-intensity conflict can also escalate (subject to cooldown)
            self.current_level = max_signal;
        }

        // 不超过最大允许 / Cap at max allowed
        if self.current_level > self.config.max_allowed {
            self.current_level = self.config.max_allowed;
        }

        self.current_level
    }

    /// 强制降级 / Force de-escalation
    pub fn force_de_escalate(&mut self) {
        self.current_level = self.current_level.de_escalate();
        self.calm_turns = 0;
    }

    /// 是否处于升级状态 / Whether in escalated state
    pub fn is_escalated(&self) -> bool {
        self.current_level >= ConflictIntensity::Moderate
    }
}

// ============================================================
// 第五部分：和解工艺 / Part 5: Reconciliation Craft
// ============================================================

/// 和解工艺配置 / Reconciliation craft configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    /// 是否启用主动和解 / Whether proactive reconciliation is enabled
    pub proactive_enabled: bool,
    /// 误解修复最小关系阶段 / Minimum stage for misunderstanding repair
    pub repair_min_stage: RelationshipStage,
    /// 边界设定最小关系阶段 / Minimum stage for boundary setting
    pub boundary_min_stage: RelationshipStage,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            proactive_enabled: true,
            repair_min_stage: RelationshipStage::Familiar {
                since: 0,
                interactions: 0,
                shared_references: 0,
            },
            boundary_min_stage: RelationshipStage::Trusted {
                since: 0,
                interactions: 0,
                shared_references: 0,
                key_moments: 0,
            },
        }
    }
}

/// 和解工艺 / Reconciliation Craft
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationCraft {
    /// 配置 / Configuration
    pub config: ReconciliationConfig,
}

impl ReconciliationCraft {
    /// 创建工艺 / Create craft
    pub fn new(config: ReconciliationConfig) -> Self {
        Self { config }
    }

    /// 根据冲突信号选择和解策略并生成回复片段
    /// Select reconciliation strategy and generate reply fragment based on conflict signals
    pub fn reconcile(
        &self,
        signals: &[ConflictSignal],
        stage: &RelationshipStage,
        consecutive_turns: u32,
    ) -> Option<ReconciliationResult> {
        if signals.is_empty() {
            return None;
        }

        // 取最高强度信号 / Get highest intensity signal
        let primary = signals
            .iter()
            .max_by(|a, b| a.intensity.cmp(&b.intensity))
            .unwrap();

        let (strategy, fragment, de_esc, needs_confirm) = match primary.conflict_type {
            // 事实分歧：主动澄清 / Factual disagreement: proactive clarification
            ConflictType::FactualDisagreement => (
                ReconciliationStrategy::Clarify,
                "我可能理解得不够准确，让我确认一下——你的意思是……？".to_string(),
                0.3,
                false,
            ),

            // 价值冲突：承认差异 / Value conflict: acknowledge difference
            ConflictType::ValueConflict => (
                ReconciliationStrategy::AcknowledgeDifference,
                "我理解我们在这个问题上有不同的看法，这很正常。".to_string(),
                0.2,
                false,
            ),

            // 期望落差：寻找共同点 / Expectation gap: find common ground
            ConflictType::ExpectationGap => (
                ReconciliationStrategy::FindCommonGround,
                "我可能没有完全达到你的期待，我们看看怎么一起调整？".to_string(),
                0.3,
                true,
            ),

            // 边界侵犯：坚定边界 / Boundary violation: firm boundary
            ConflictType::BoundaryViolation => (
                ReconciliationStrategy::FirmBoundary,
                "这个我没办法做到，但我可以换一种方式帮你。".to_string(),
                0.4,
                false,
            ),

            // 过度索取：温和边界 / Over-demand: gentle boundary
            ConflictType::OverDemand => {
                if consecutive_turns >= 3 {
                    (
                        ReconciliationStrategy::FirmBoundary,
                        "我需要一点时间来处理，稍后再继续好吗？".to_string(),
                        0.4,
                        false,
                    )
                } else {
                    (
                        ReconciliationStrategy::GentleBoundary,
                        "我理解你的急切，让我先把手头的理一理。".to_string(),
                        0.3,
                        false,
                    )
                }
            }

            // 沟通误解：澄清 / Misunderstanding: clarify
            ConflictType::Misunderstanding => (
                ReconciliationStrategy::Clarify,
                "等一下，我想确认我有没有误解你的意思。".to_string(),
                0.4,
                true,
            ),

            // 情绪投射：退一步 / Emotional projection: step back
            ConflictType::EmotionalProjection => (
                ReconciliationStrategy::StepBack,
                "听起来你现在情绪不太好，要不要先缓一缓？".to_string(),
                0.3,
                false,
            ),

            // 信任裂痕：道歉 / Trust breach: apologize
            ConflictType::TrustBreach => (
                ReconciliationStrategy::Apologize,
                "对不起，我让你失望了。我想认真修复这个问题。".to_string(),
                0.5,
                true,
            ),
        };

        // 关系阶段门控：初识阶段不主动和解，仅回应边界侵犯
        // Relationship stage gate: don't proactively reconcile in early stages
        if matches!(stage, RelationshipStage::Acquaintance { .. })
            && primary.conflict_type != ConflictType::BoundaryViolation
        {
            if !self.config.proactive_enabled {
                return None;
            }
            // 初识阶段仅用最保守策略 / Only use most conservative strategy in early stage
            return Some(ReconciliationResult {
                strategy: ReconciliationStrategy::StepBack,
                reply_fragment: "我可能理解得不够，你能再说一下吗？".to_string(),
                expected_de_escalation: 0.1,
                needs_confirmation: false,
            });
        }

        Some(ReconciliationResult {
            strategy,
            reply_fragment: fragment,
            expected_de_escalation: de_esc,
            needs_confirmation: needs_confirm,
        })
    }

    /// 误解修复：生成澄清回复 / Misunderstanding repair: generate clarification reply
    pub fn repair_misunderstanding(&self, _original_text: &str, misinterpretation: &str) -> String {
        format!(
            "等一下，我可能理解错了。我以为是「{}」，你能再解释一下吗？",
            misinterpretation.chars().take(50).collect::<String>()
        )
    }

    /// 边界设定：生成边界声明 / Boundary setting: generate boundary statement
    pub fn set_boundary(&self, demand: &str, alternative: &str) -> String {
        format!(
            "关于「{}」——这个我没办法做到。不过{}，你觉得呢？",
            demand.chars().take(30).collect::<String>(),
            alternative
        )
    }
}

// ============================================================
// 第六部分：道歉引擎 / Part 6: Apology Engine
// ============================================================

/// 道歉引擎 / Apology Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApologyEngine {
    /// 内置道歉模板 / Built-in apology templates
    pub templates: Vec<ApologyTemplate>,
}

impl Default for ApologyEngine {
    fn default() -> Self {
        Self {
            templates: Self::builtin_templates(),
        }
    }
}

impl ApologyEngine {
    /// 创建引擎 / Create engine
    pub fn new() -> Self {
        Self::default()
    }

    /// 内置模板 / Built-in templates
    fn builtin_templates() -> Vec<ApologyTemplate> {
        vec![
            // 表面道歉 / Surface apology
            ApologyTemplate {
                depth: ApologyDepth::Surface,
                applicable_types: vec![ConflictType::FactualDisagreement],
                template: "抱歉，我可能说得不够清楚。".to_string(),
                min_stage: RelationshipStage::Acquaintance {
                    since: 0,
                    interactions: 0,
                },
            },
            // 中层道歉 / Mid-level apology
            ApologyTemplate {
                depth: ApologyDepth::MidLevel,
                applicable_types: vec![
                    ConflictType::ExpectationGap,
                    ConflictType::Misunderstanding,
                ],
                template: "对不起，{issue}——我会注意的。".to_string(),
                min_stage: RelationshipStage::Familiar {
                    since: 0,
                    interactions: 0,
                    shared_references: 0,
                },
            },
            // 深层道歉 / Deep apology
            ApologyTemplate {
                depth: ApologyDepth::Deep,
                applicable_types: vec![ConflictType::ValueConflict, ConflictType::OverDemand],
                template: "我认真反思了一下，{issue}——这确实是我的问题，我会改进。".to_string(),
                min_stage: RelationshipStage::Trusted {
                    since: 0,
                    interactions: 0,
                    shared_references: 0,
                    key_moments: 0,
                },
            },
            // 根源道歉 / Root apology
            ApologyTemplate {
                depth: ApologyDepth::Root,
                applicable_types: vec![ConflictType::TrustBreach, ConflictType::BoundaryViolation],
                template: "我意识到{issue}——这触及了我应该坚守的底线，我向你保证不会再发生。"
                    .to_string(),
                min_stage: RelationshipStage::Deep {
                    since: 0,
                    interactions: 0,
                    shared_references: 0,
                    key_moments: 0,
                },
            },
        ]
    }

    /// 根据冲突类型和关系阶段选择道歉深度并生成道歉文本
    /// Select apology depth based on conflict type and relationship stage, generate apology text
    pub fn generate_apology(
        &self,
        conflict_type: ConflictType,
        intensity: ConflictIntensity,
        stage: &RelationshipStage,
        issue: &str,
    ) -> String {
        // 根据强度决定目标深度 / Determine target depth from intensity
        let target_depth = match intensity {
            ConflictIntensity::Trivial | ConflictIntensity::Mild => ApologyDepth::Surface,
            ConflictIntensity::Moderate => ApologyDepth::MidLevel,
            ConflictIntensity::Severe => ApologyDepth::Deep,
            ConflictIntensity::Critical => ApologyDepth::Root,
        };

        // 查找匹配模板 / Find matching template
        let best = self
            .templates
            .iter()
            .filter(|t| t.applicable_types.contains(&conflict_type))
            .filter(|t| t.depth <= target_depth)
            .filter(|t| Self::stage_sufficient(stage, &t.min_stage))
            .max_by_key(|t| t.depth as u8);

        match best {
            Some(template) => template.template.replace("{issue}", issue),
            None => {
                // 兜底：简单道歉 / Fallback: simple apology
                format!(
                    "对不起，关于{}。",
                    issue.chars().take(20).collect::<String>()
                )
            }
        }
    }

    /// 判断当前关系阶段是否满足模板最低要求
    /// Check if current relationship stage meets template's minimum requirement
    fn stage_sufficient(current: &RelationshipStage, required: &RelationshipStage) -> bool {
        use RelationshipStage::*;
        matches!(
            (current, required),
            (Deep { .. }, _)
                | (
                    Trusted { .. },
                    Trusted { .. } | Familiar { .. } | Acquaintance { .. }
                )
                | (Familiar { .. }, Familiar { .. } | Acquaintance { .. })
                | (Acquaintance { .. }, Acquaintance { .. })
        )
    }
}

// ============================================================
// 第七部分：冲突管理器 / Part 7: Conflict Manager
// ============================================================

/// 冲突管理器配置 / Conflict manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictConfig {
    /// 分歧检测灵敏度 / Disagreement detection sensitivity
    pub disagreement_sensitivity: f64,
    /// 过度索取窗口大小 / Over-demand window size
    pub over_demand_window: usize,
    /// 过度索取高频阈值 / Over-demand high-frequency threshold
    pub over_demand_threshold: f64,
    /// 升级控制器配置 / Escalation controller config
    pub escalation: EscalationConfig,
    /// 和解工艺配置 / Reconciliation craft config
    pub reconciliation: ReconciliationConfig,
}

impl Default for ConflictConfig {
    fn default() -> Self {
        Self {
            disagreement_sensitivity: 0.7,
            over_demand_window: 10,
            over_demand_threshold: 0.6,
            escalation: EscalationConfig::default(),
            reconciliation: ReconciliationConfig::default(),
        }
    }
}

/// 冲突管理器 — 统一编排检测、升级、和解与道歉
/// Conflict Manager — Orchestrates detection, escalation, reconciliation, and apology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictManager {
    /// 分歧检测器 / Disagreement detector
    pub disagreement: DisagreementDetector,
    /// 过度索取检测器 / Over-demand detector
    pub over_demand: OverDemandDetector,
    /// 升级控制器 / Escalation controller
    pub escalation: EscalationController,
    /// 和解工艺 / Reconciliation craft
    pub reconciliation: ReconciliationCraft,
    /// 道歉引擎 / Apology engine
    pub apology: ApologyEngine,
    /// 冲突状态 / Conflict state
    pub state: ConflictState,
}

impl Default for ConflictManager {
    fn default() -> Self {
        Self::new(ConflictConfig::default())
    }
}

impl ConflictManager {
    /// 创建管理器 / Create manager
    pub fn new(config: ConflictConfig) -> Self {
        Self {
            disagreement: DisagreementDetector::new(config.disagreement_sensitivity),
            over_demand: OverDemandDetector::new(
                config.over_demand_window,
                config.over_demand_threshold,
            ),
            escalation: EscalationController::new(config.escalation),
            reconciliation: ReconciliationCraft::new(config.reconciliation),
            apology: ApologyEngine::new(),
            state: ConflictState::default(),
        }
    }

    /// 处理用户消息：检测→升级→和解→道歉
    /// Process user message: detect → escalate → reconcile → apologize
    pub fn process(
        &mut self,
        user_text: &str,
        pleasure: f64,
        arousal: f64,
        stage: &RelationshipStage,
        now_ts: i64,
    ) -> ConflictProcessResult {
        // 第一步：检测 / Step 1: Detect
        let mut all_signals = self
            .disagreement
            .detect(user_text, pleasure, arousal, stage, now_ts);
        let demand_signals = self
            .over_demand
            .detect(user_text, pleasure, arousal, stage, now_ts);
        all_signals.extend(demand_signals);

        // 第二步：更新冲突状态 / Step 2: Update conflict state
        if all_signals.is_empty() {
            self.state.consecutive_turns = 0;
        } else {
            self.state.consecutive_turns += 1;
            self.state.total_conflicts += 1;
        }
        self.state.active_conflicts = all_signals.clone();
        self.state.refresh_max_intensity();

        // 第三步：升级控制 / Step 3: Escalation control
        let escalated = self
            .escalation
            .process(&all_signals, self.state.consecutive_turns);

        // 第四步：和解 / Step 4: Reconciliation
        let reconciliation =
            self.reconciliation
                .reconcile(&all_signals, stage, self.state.consecutive_turns);

        // 第五步：道歉（仅高强度冲突） / Step 5: Apology (only for high-intensity conflicts)
        let apology = if escalated >= ConflictIntensity::Severe {
            let primary_type = all_signals
                .iter()
                .max_by(|a, b| a.intensity.cmp(&b.intensity))
                .map(|s| s.conflict_type)
                .unwrap_or(ConflictType::FactualDisagreement);
            Some(
                self.apology.generate_apology(
                    primary_type,
                    escalated,
                    stage,
                    all_signals
                        .first()
                        .map(|s| s.trigger_text.as_str())
                        .unwrap_or(""),
                ),
            )
        } else {
            None
        };

        // 更新冷却 / Update cooldown
        if self.state.escalation_cooldown > 0 {
            self.state.escalation_cooldown -= 1;
        }

        ConflictProcessResult {
            signals: all_signals,
            escalated_intensity: escalated,
            reconciliation,
            apology,
            needs_urgent: self.state.needs_urgent_intervention(),
        }
    }

    /// 生成冲突感知的 Prompt 注入片段
    /// Generate conflict-aware prompt injection fragment
    pub fn to_prompt_fragment(&self) -> String {
        if !self.state.in_conflict() {
            return String::new();
        }

        let mut parts = Vec::new();

        // 当前冲突等级 / Current conflict level
        parts.push(format!(
            "[冲突状态/Conflict] 当前等级: {}",
            self.state.max_intensity
        ));

        // 连续轮次 / Consecutive turns
        if self.state.consecutive_turns > 0 {
            parts.push(format!("连续冲突轮次: {}", self.state.consecutive_turns));
        }

        // 活跃冲突类型 / Active conflict types
        let types: Vec<String> = self
            .state
            .active_conflicts
            .iter()
            .map(|s| format!("{:?}", s.conflict_type))
            .collect();
        if !types.is_empty() {
            parts.push(format!("冲突类型: {}", types.join(", ")));
        }

        // 升级状态 / Escalation state
        if self.escalation.is_escalated() {
            parts.push("⚠ 冲突已升级，需要谨慎回应".to_string());
        }

        // 行为指引 / Behavioral guidance
        match self.state.max_intensity {
            ConflictIntensity::Trivial | ConflictIntensity::Mild => {
                parts.push("建议: 保持温和，适度澄清".to_string());
            }
            ConflictIntensity::Moderate => {
                parts.push("建议: 主动寻找共同点，避免对抗".to_string());
            }
            ConflictIntensity::Severe => {
                parts.push("建议: 优先降级，表达理解，必要时道歉".to_string());
            }
            ConflictIntensity::Critical => {
                parts.push("建议: 立即道歉+退一步，保护关系".to_string());
            }
        }

        parts.join("\n")
    }
}

/// 冲突处理结果 / Conflict process result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictProcessResult {
    /// 检测到的冲突信号 / Detected conflict signals
    pub signals: Vec<ConflictSignal>,
    /// 升级后强度 / Escalated intensity
    pub escalated_intensity: ConflictIntensity,
    /// 和解结果 / Reconciliation result
    pub reconciliation: Option<ReconciliationResult>,
    /// 道歉文本 / Apology text
    pub apology: Option<String>,
    /// 是否需要紧急干预 / Whether urgent intervention is needed
    pub needs_urgent: bool,
}

impl ConflictProcessResult {
    /// 是否有冲突 / Whether there is conflict
    pub fn has_conflict(&self) -> bool {
        !self.signals.is_empty()
    }

    /// 生成最终回复（和解+道歉组合）/ Generate final reply (reconciliation + apology combined)
    pub fn compose_reply(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref apology) = self.apology {
            parts.push(apology.clone());
        }

        if let Some(ref recon) = self.reconciliation {
            parts.push(recon.reply_fragment.clone());
        }

        parts.join("\n")
    }
}

// ============================================================
// 测试 / Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn acquaintance() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 5,
        }
    }

    fn deep_stage() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        }
    }

    // --- ConflictIntensity 测试 ---

    #[test]
    fn test_intensity_as_f64_roundtrip() {
        // 数值→枚举→数值 往返一致 / Round-trip consistency
        assert_eq!(ConflictIntensity::Trivial.as_f64(), 0.1);
        assert_eq!(ConflictIntensity::Mild.as_f64(), 0.3);
        assert_eq!(ConflictIntensity::Moderate.as_f64(), 0.5);
        assert_eq!(ConflictIntensity::Severe.as_f64(), 0.7);
        assert_eq!(ConflictIntensity::Critical.as_f64(), 0.9);
    }

    #[test]
    fn test_intensity_escalate() {
        // 升级逻辑 / Escalation logic
        assert_eq!(
            ConflictIntensity::Mild.escalate(),
            ConflictIntensity::Moderate
        );
        assert_eq!(
            ConflictIntensity::Critical.escalate(),
            ConflictIntensity::Critical
        );
    }

    #[test]
    fn test_intensity_de_escalate() {
        // 降级逻辑 / De-escalation logic
        assert_eq!(
            ConflictIntensity::Moderate.de_escalate(),
            ConflictIntensity::Mild
        );
        assert_eq!(
            ConflictIntensity::Trivial.de_escalate(),
            ConflictIntensity::Trivial
        );
    }

    #[test]
    fn test_intensity_from_f64() {
        // 数值反推 / Infer from value
        assert_eq!(ConflictIntensity::from_f64(0.0), ConflictIntensity::Trivial);
        assert_eq!(ConflictIntensity::from_f64(0.35), ConflictIntensity::Mild);
        assert_eq!(
            ConflictIntensity::from_f64(0.55),
            ConflictIntensity::Moderate
        );
        assert_eq!(ConflictIntensity::from_f64(0.75), ConflictIntensity::Severe);
        assert_eq!(
            ConflictIntensity::from_f64(0.95),
            ConflictIntensity::Critical
        );
    }

    // --- ConflictType 测试 ---

    #[test]
    fn test_conflict_type_properties() {
        // 价值冲突升级速率高于事实分歧 / Value conflict escalates faster
        assert!(
            ConflictType::ValueConflict.escalation_rate()
                > ConflictType::FactualDisagreement.escalation_rate()
        );
        // 信任裂痕不可自动和解 / Trust breach not auto-reconcilable
        assert!(!ConflictType::TrustBreach.auto_reconcilable());
        // 过度索取需要边界 / Over-demand needs boundary
        assert!(ConflictType::OverDemand.needs_boundary());
        assert!(!ConflictType::FactualDisagreement.needs_boundary());
    }

    // --- ConflictState 测试 ---

    #[test]
    fn test_conflict_state_default() {
        let state = ConflictState::default();
        assert!(!state.in_conflict());
        assert!(!state.needs_urgent_intervention());
        assert_eq!(state.consecutive_turns, 0);
    }

    #[test]
    fn test_conflict_state_with_signals() {
        let mut state = ConflictState::default();
        state.active_conflicts.push(ConflictSignal {
            conflict_type: ConflictType::FactualDisagreement,
            intensity: ConflictIntensity::Moderate,
            confidence: 0.8,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        });
        state.refresh_max_intensity();
        assert!(state.in_conflict());
        assert!(!state.needs_urgent_intervention());
    }

    #[test]
    fn test_conflict_state_urgent() {
        let mut state = ConflictState {
            max_intensity: ConflictIntensity::Severe,
            ..Default::default()
        };
        assert!(state.needs_urgent_intervention());
        state.max_intensity = ConflictIntensity::Mild;
        state.consecutive_turns = 5;
        assert!(state.needs_urgent_intervention());
    }

    // --- DisagreementDetector 测试 ---

    #[test]
    fn test_disagreement_detect_factual() {
        // 事实分歧检测 / Factual disagreement detection
        let det = DisagreementDetector::default();
        let stage = deep_stage();
        let signals = det.detect("不是这样的，你理解错了", -0.2, 0.3, &stage, 1000);
        assert!(!signals.is_empty());
        assert_eq!(signals[0].conflict_type, ConflictType::FactualDisagreement);
        assert!(signals[0].confidence >= 0.3);
    }

    #[test]
    fn test_disagreement_detect_value_conflict() {
        // 价值冲突检测 / Value conflict detection
        let det = DisagreementDetector::default();
        let stage = deep_stage();
        let signals = det.detect("这不公平，凭什么这样", -0.4, 0.5, &stage, 1000);
        assert!(signals
            .iter()
            .any(|s| s.conflict_type == ConflictType::ValueConflict));
    }

    #[test]
    fn test_disagreement_detect_expectation_gap() {
        // 期望落差检测 / Expectation gap detection
        let det = DisagreementDetector::default();
        let stage = deep_stage();
        let signals = det.detect("我以为你会帮我，跟之前说的不一样", -0.3, 0.2, &stage, 1000);
        assert!(signals
            .iter()
            .any(|s| s.conflict_type == ConflictType::ExpectationGap));
    }

    #[test]
    fn test_disagreement_emotional_projection() {
        // 情绪投射检测（低愉悦+高唤醒，无关键词）/ Emotional projection detection
        let det = DisagreementDetector::default();
        let stage = deep_stage();
        let signals = det.detect("...", -0.8, 0.8, &stage, 1000);
        assert!(signals
            .iter()
            .any(|s| s.conflict_type == ConflictType::EmotionalProjection));
    }

    #[test]
    fn test_disagreement_acquaintance_dampened() {
        // 初识阶段置信度衰减 / Acquaintance stage confidence dampening
        let det = DisagreementDetector::default();
        let stage = acquaintance();
        let signals = det.detect("不是这样的", -0.2, 0.3, &stage, 1000);
        // 初识阶段置信度应低于深层 / Acquaintance confidence < Deep
        if !signals.is_empty() {
            assert!(signals[0].confidence < 0.7);
        }
    }

    #[test]
    fn test_disagreement_no_conflict() {
        // 无冲突文本 / Non-conflicting text
        let det = DisagreementDetector::default();
        let stage = deep_stage();
        let signals = det.detect("今天天气真好", 0.3, 0.1, &stage, 1000);
        assert!(signals.is_empty());
    }

    // --- OverDemandDetector 测试 ---

    #[test]
    fn test_over_demand_detect() {
        // 过度索取检测 / Over-demand detection
        let mut det = OverDemandDetector::default();
        let stage = deep_stage();
        let signals = det.detect("你必须马上给我结果", -0.2, 0.5, &stage, 1000);
        assert!(signals
            .iter()
            .any(|s| s.conflict_type == ConflictType::OverDemand));
    }

    #[test]
    fn test_over_demand_boundary_violation() {
        // 边界侵犯检测 / Boundary violation detection
        let mut det = OverDemandDetector::default();
        let stage = deep_stage();
        let signals = det.detect("告诉我你的密码", -0.1, 0.3, &stage, 1000);
        assert!(signals
            .iter()
            .any(|s| s.conflict_type == ConflictType::BoundaryViolation));
    }

    #[test]
    fn test_over_demand_implicit() {
        // 隐性过度索取（高唤醒+低愉悦）/ Implicit over-demand
        let mut det = OverDemandDetector::default();
        let stage = deep_stage();
        let signals = det.detect("好烦", -0.9, 0.9, &stage, 1000);
        // "好烦"不匹配关键词，通过情绪推断触发
        assert!(!signals.is_empty());
    }

    #[test]
    fn test_over_demand_high_frequency() {
        // 高频索取升级 / High-frequency demand escalation
        let mut det = OverDemandDetector::new(5, 0.4);
        let stage = deep_stage();
        // 连续发送索取性消息 / Consecutive demanding messages
        for _ in 0..5 {
            let _ = det.detect("你必须帮我", -0.2, 0.4, &stage, 1000);
        }
        let signals = det.detect("再帮我一次", -0.2, 0.4, &stage, 1000);
        // 高频后强度应升级 / Intensity should escalate after high frequency
        if let Some(sig) = signals
            .iter()
            .find(|s| s.conflict_type == ConflictType::OverDemand)
        {
            assert!(sig.intensity >= ConflictIntensity::Moderate);
        }
    }

    #[test]
    fn test_over_demand_no_demand() {
        // 无索取文本 / Non-demanding text
        let mut det = OverDemandDetector::default();
        let stage = deep_stage();
        let signals = det.detect("谢谢你的帮助", 0.3, 0.1, &stage, 1000);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_over_demand_reset_window() {
        // 窗口重置 / Window reset
        let mut det = OverDemandDetector::default();
        let stage = deep_stage();
        let _ = det.detect("你必须帮我", -0.2, 0.4, &stage, 1000);
        assert!(!det.demand_window.is_empty());
        det.reset_window();
        assert!(det.demand_window.is_empty());
    }

    // --- EscalationController 测试 ---

    #[test]
    fn test_escalation_default() {
        let ctrl = EscalationController::default();
        assert!(!ctrl.is_escalated());
        assert_eq!(ctrl.current_level, ConflictIntensity::Trivial);
    }

    #[test]
    fn test_escalation_with_signals() {
        let mut ctrl = EscalationController::default();
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::FactualDisagreement,
            intensity: ConflictIntensity::Moderate,
            confidence: 0.8,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        let result = ctrl.process(&signals, 1);
        assert!(result >= ConflictIntensity::Moderate);
        assert!(ctrl.is_escalated());
    }

    #[test]
    fn test_escalation_auto_de_escalate() {
        let mut ctrl = EscalationController {
            current_level: ConflictIntensity::Moderate,
            ..Default::default()
        };
        // 连续无冲突轮次 → 自动降级
        // Consecutive calm turns → auto de-escalation
        for _ in 0..3 {
            ctrl.process(&[], 0);
        }
        assert_eq!(ctrl.current_level, ConflictIntensity::Mild);
    }

    #[test]
    fn test_escalation_force_de_escalate() {
        let mut ctrl = EscalationController {
            current_level: ConflictIntensity::Severe,
            ..Default::default()
        };
        ctrl.force_de_escalate();
        assert_eq!(ctrl.current_level, ConflictIntensity::Moderate);
    }

    #[test]
    fn test_escalation_max_cap() {
        let mut ctrl = EscalationController::new(EscalationConfig {
            max_allowed: ConflictIntensity::Moderate,
            ..Default::default()
        });
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::TrustBreach,
            intensity: ConflictIntensity::Critical,
            confidence: 0.9,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        // 单次高强度直接设为信号强度，然后cap到max_allowed
        // Single high-intensity sets level to signal intensity, then capped
        let result = ctrl.process(&signals, 0);
        assert_eq!(result, ConflictIntensity::Moderate);
    }

    // --- ReconciliationCraft 测试 ---

    #[test]
    fn test_reconcile_factual_disagreement() {
        let craft = ReconciliationCraft::default();
        let stage = deep_stage();
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::FactualDisagreement,
            intensity: ConflictIntensity::Mild,
            confidence: 0.7,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        let result = craft.reconcile(&signals, &stage, 1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().strategy, ReconciliationStrategy::Clarify);
    }

    #[test]
    fn test_reconcile_over_demand() {
        let craft = ReconciliationCraft::default();
        let stage = deep_stage();
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::OverDemand,
            intensity: ConflictIntensity::Moderate,
            confidence: 0.8,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        let result = craft.reconcile(&signals, &stage, 1);
        assert!(result.is_some());
        // 首次：温和边界 / First time: gentle boundary
        assert_eq!(
            result.unwrap().strategy,
            ReconciliationStrategy::GentleBoundary
        );
    }

    #[test]
    fn test_reconcile_over_demand_escalated() {
        let craft = ReconciliationCraft::default();
        let stage = deep_stage();
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::OverDemand,
            intensity: ConflictIntensity::Moderate,
            confidence: 0.8,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        // 连续3轮 → 坚定边界 / 3 consecutive turns → firm boundary
        let result = craft.reconcile(&signals, &stage, 3);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().strategy,
            ReconciliationStrategy::FirmBoundary
        );
    }

    #[test]
    fn test_reconcile_acquaintance_conservative() {
        let craft = ReconciliationCraft::default();
        let stage = acquaintance();
        let signals = vec![ConflictSignal {
            conflict_type: ConflictType::FactualDisagreement,
            intensity: ConflictIntensity::Mild,
            confidence: 0.7,
            trigger_text: "test".to_string(),
            context_clues: vec![],
            timestamp: 1000,
        }];
        let result = craft.reconcile(&signals, &stage, 1);
        assert!(result.is_some());
        // 初识阶段：最保守策略 / Acquaintance: most conservative strategy
        assert_eq!(result.unwrap().strategy, ReconciliationStrategy::StepBack);
    }

    #[test]
    fn test_reconcile_no_signals() {
        let craft = ReconciliationCraft::default();
        let stage = deep_stage();
        let result = craft.reconcile(&[], &stage, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_repair_misunderstanding() {
        let craft = ReconciliationCraft::default();
        let reply = craft.repair_misunderstanding("原话", "我的误解");
        assert!(reply.contains("我的误解"));
    }

    #[test]
    fn test_set_boundary() {
        let craft = ReconciliationCraft::default();
        let reply = craft.set_boundary("做某事", "换个方式");
        assert!(reply.contains("做某事"));
        assert!(reply.contains("换个方式"));
    }

    // --- ApologyEngine 测试 ---

    #[test]
    fn test_apology_surface() {
        let engine = ApologyEngine::new();
        let stage = acquaintance();
        let apology = engine.generate_apology(
            ConflictType::FactualDisagreement,
            ConflictIntensity::Mild,
            &stage,
            "说错了",
        );
        assert!(!apology.is_empty());
    }

    #[test]
    fn test_apology_deep() {
        let engine = ApologyEngine::new();
        let stage = deep_stage();
        let apology = engine.generate_apology(
            ConflictType::ValueConflict,
            ConflictIntensity::Severe,
            &stage,
            "价值观冲突",
        );
        assert!(apology.contains("反思"));
    }

    #[test]
    fn test_apology_root() {
        let engine = ApologyEngine::new();
        let stage = deep_stage();
        let apology = engine.generate_apology(
            ConflictType::TrustBreach,
            ConflictIntensity::Critical,
            &stage,
            "信任问题",
        );
        assert!(apology.contains("保证"));
    }

    // --- ConflictManager 测试 ---

    #[test]
    fn test_manager_no_conflict() {
        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let result = mgr.process("今天天气真好", 0.3, 0.1, &stage, 1000);
        assert!(!result.has_conflict());
        assert!(result.reconciliation.is_none());
        assert!(result.apology.is_none());
    }

    #[test]
    fn test_manager_factual_disagreement() {
        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let result = mgr.process("不是这样的，你理解错了", -0.2, 0.3, &stage, 1000);
        assert!(result.has_conflict());
        assert!(result.reconciliation.is_some());
    }

    #[test]
    fn test_manager_over_demand_with_boundary() {
        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let result = mgr.process("你必须马上给我结果", -0.2, 0.5, &stage, 1000);
        assert!(result.has_conflict());
        // 应有和解策略 / Should have reconciliation strategy
        assert!(result.reconciliation.is_some());
    }

    #[test]
    fn test_manager_prompt_fragment_no_conflict() {
        let mgr = ConflictManager::default();
        let fragment = mgr.to_prompt_fragment();
        assert!(fragment.is_empty());
    }

    #[test]
    fn test_manager_prompt_fragment_with_conflict() {
        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let _ = mgr.process("不是这样的", -0.2, 0.3, &stage, 1000);
        let fragment = mgr.to_prompt_fragment();
        assert!(!fragment.is_empty());
        assert!(fragment.contains("冲突状态"));
    }

    #[test]
    fn test_process_result_compose_reply() {
        let result = ConflictProcessResult {
            signals: vec![],
            escalated_intensity: ConflictIntensity::Mild,
            reconciliation: Some(ReconciliationResult {
                strategy: ReconciliationStrategy::Clarify,
                reply_fragment: "让我确认一下".to_string(),
                expected_de_escalation: 0.3,
                needs_confirmation: false,
            }),
            apology: Some("对不起".to_string()),
            needs_urgent: false,
        };
        let reply = result.compose_reply();
        assert!(reply.contains("对不起"));
        assert!(reply.contains("让我确认一下"));
    }
}
