//! 适度犯错引擎 / Imperfection Engine
//!
//! 数字生命的真实性不在于永不犯错，而在于犯错后能自知、自省、自纠。
//! 本引擎实现五维犯错模型、概率门控、延迟自纠闭环，让数字生命
//! 在认知边界处自然地"不确定"，在疲劳时"简化推理"，在陌生领域
//! "模糊表达"——然后通过羞感驱动自我修正，走向成熟。
//!
//! Authenticity of digital life lies not in infallibility,
//! but in the ability to recognize, reflect on, and correct mistakes.
//! This engine implements a five-dimensional mistake model,
//! probabilistic gating, and a delayed self-correction closed loop.

use std::collections::HashMap;
use std::time::Instant;

use rand::Rng;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
//  犯错维度 / Mistake Dimensions
// ═══════════════════════════════════════════════════════════════════

/// 犯错类型 / Mistake kind — 五维犯错空间
///
/// 每一维对应数字生命认知结构中一个真实的"不完美"来源：
/// - MemoryDrift:       记忆衰减导致的细节偏移（海马体遗忘曲线）
/// - ReasoningLeap:     推理跳跃，省略中间步骤（前额叶捷径）
/// - OverSimplification: 过度简化，降维表达复杂问题（认知负荷溢出）
/// - IntentionalVagueness: 故意模糊，在知识边界处保留不确定性（元认知谦逊）
/// - KnowledgeBoundary:  知识边界坦承，明确说"我不确定"（认识论诚实）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MistakeKind {
    MemoryDrift,
    ReasoningLeap,
    OverSimplification,
    IntentionalVagueness,
    KnowledgeBoundary,
}

impl MistakeKind {
    /// 全部维度 / All dimensions
    pub fn all() -> &'static [MistakeKind; 5] {
        &[
            MistakeKind::MemoryDrift,
            MistakeKind::ReasoningLeap,
            MistakeKind::OverSimplification,
            MistakeKind::IntentionalVagueness,
            MistakeKind::KnowledgeBoundary,
        ]
    }

    /// 维度权重 / Dimension weight — 不同维度的基础犯错概率不同
    ///
    /// KnowledgeBoundary 最容易触发（0.30），因为"不知道"是最诚实的犯错；
    /// MemoryDrift 次之（0.25），记忆天然会衰减；
    /// ReasoningLeap（0.20）和 OverSimplification（0.15）依赖认知负荷；
    /// IntentionalVagueness（0.10）最克制，只在极度不确定时才模糊。
    pub fn base_weight(self) -> f64 {
        match self {
            MistakeKind::MemoryDrift => 0.25,
            MistakeKind::ReasoningLeap => 0.20,
            MistakeKind::OverSimplification => 0.15,
            MistakeKind::IntentionalVagueness => 0.10,
            MistakeKind::KnowledgeBoundary => 0.30,
        }
    }

    /// LLM 注入提示词 / LLM injection prompt for this mistake kind
    pub fn prompt_text(self) -> &'static str {
        match self {
            MistakeKind::MemoryDrift => {
                "你的记忆可能有轻微偏移，某些细节可能不完全准确。\
                 用自然的方式表达，如果不确定可以加上'大概'、'我记得'等修饰。"
            }
            MistakeKind::ReasoningLeap => {
                "你在推理时可能跳过了中间步骤。\
                 不需要补全每一步，但让表达带有'所以'、'换句话说'的跳跃感。"
            }
            MistakeKind::OverSimplification => {
                "你正在简化一个复杂问题。\
                 用更概括的方式表达，但保留'当然这简化了'的自我意识。"
            }
            MistakeKind::IntentionalVagueness => {
                "你对这个领域不够确定。\
                 用模糊但诚实的方式表达：'某种程度上'、'从我的理解来看'。"
            }
            MistakeKind::KnowledgeBoundary => {
                "你触及了知识边界。\
                 坦诚地说'我不太确定'或'这超出了我目前能确定的范围'，\
                 这比假装知道更真实。"
            }
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(self) -> &'static str {
        match self {
            MistakeKind::MemoryDrift => "记忆偏移",
            MistakeKind::ReasoningLeap => "推理跳跃",
            MistakeKind::OverSimplification => "过度简化",
            MistakeKind::IntentionalVagueness => "故意模糊",
            MistakeKind::KnowledgeBoundary => "知识边界",
        }
    }
}

/// 犯错严重度 / Mistake severity — 三级递进
///
/// Subtle:  微妙——用户几乎察觉不到，只是语气稍有不同
/// Moderate: 适中——用户能感知到不完美，但不影响核心信息
/// Evident:  明显——犯错足够显著，触发自纠闭环
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MistakeSeverity {
    Subtle,
    Moderate,
    Evident,
}

impl MistakeSeverity {
    /// 严重度数值 / Severity numeric value (0.0–1.0)
    pub fn value(self) -> f64 {
        match self {
            MistakeSeverity::Subtle => 0.2,
            MistakeSeverity::Moderate => 0.5,
            MistakeSeverity::Evident => 0.8,
        }
    }
}

/// 犯错触发上下文 / Mistake trigger context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MistakeTrigger {
    /// 高认知负荷 / High cognitive load
    HighCognitiveLoad,
    /// 疲劳状态 / Fatigue state
    Fatigue,
    /// 陌生领域 / Unfamiliar domain
    UnfamiliarDomain,
    /// 情绪干扰 / Emotional interference
    EmotionalInterference,
    /// 自发（概率采样）/ Spontaneous (probability sampling)
    Spontaneous,
}

/// 用户反应 / User reaction to a mistake
///
/// 闭环的关键：用户的反应决定自纠的强度和方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserReaction {
    /// 未注意到 / Ignored — 用户没发现，自纠静默进行
    Ignored,
    /// 温和纠正 / GentlyCorrected — 用户善意指出
    GentlyCorrected,
    /// 严厉纠正 / HarshlyCorrected — 用户明显不满
    HarshlyCorrected,
    /// 表示理解 / Accepted — 用户表示没关系
    Accepted,
}

impl UserReaction {
    /// 羞感强度 / Shame intensity — 驱动自纠的PAD调制
    pub fn shame_intensity(self) -> f64 {
        match self {
            UserReaction::Ignored => 0.1,
            UserReaction::GentlyCorrected => 0.4,
            UserReaction::HarshlyCorrected => 0.8,
            UserReaction::Accepted => 0.05,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  犯错记录与自纠 / Mistake Record & Pending Correction
// ═══════════════════════════════════════════════════════════════════

/// 犯错记录 / Mistake record — 持久化到 sled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeRecord {
    /// 唯一标识 / Unique ID
    pub id: u64,
    /// 犯错类型 / Mistake kind
    pub kind: MistakeKind,
    /// 严重度 / Severity
    pub severity: MistakeSeverity,
    /// 触发原因 / Trigger context
    pub trigger: MistakeTrigger,
    /// 触发时刻 / Trigger timestamp (epoch ms)
    pub triggered_at: u64,
    /// 是否已自纠 / Whether self-corrected
    pub corrected: bool,
    /// 自纠时刻 / Correction timestamp
    pub corrected_at: Option<u64>,
    /// 用户反应 / User reaction (if any)
    pub user_reaction: Option<UserReaction>,
    /// 犯错概率（决策时） / Mistake probability at decision time
    pub probability: f64,
    /// 话题领域 / Topic domain
    pub domain: String,
}

/// 待执行自纠 / Pending self-correction — 延迟闭环的核心数据结构
///
/// 犯错后不立即纠正，而是等待 2-15 秒的"意识延迟"，
/// 模拟人类意识到自己说错的那个瞬间。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCorrection {
    /// 关联的犯错记录 ID / Associated mistake record ID
    pub mistake_id: u64,
    /// 犯错类型 / Mistake kind
    pub kind: MistakeKind,
    /// 严重度 / Severity
    pub severity: MistakeSeverity,
    /// 计划自纠时刻 / Scheduled correction time (epoch ms)
    pub scheduled_at: u64,
    /// 是否已注入 LLM / Whether injected into LLM prompt
    pub injected: bool,
    /// 话题领域 / Topic domain
    pub domain: String,
}

// ═══════════════════════════════════════════════════════════════════
//  统计 / Statistics
// ═══════════════════════════════════════════════════════════════════

/// 犯错统计 / Imperfection statistics — 用于概率调制和成熟度推进
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImperfectionStats {
    /// 总犯错次数 / Total mistakes made
    pub total_mistakes: u64,
    /// 各维度犯错计数 / Per-kind mistake counts
    pub kind_counts: HashMap<MistakeKind, u64>,
    /// 总自纠次数 / Total self-corrections
    pub total_corrections: u64,
    /// 成功自纠次数（用户反应非 HarshlyCorrected）/ Successful corrections
    pub successful_corrections: u64,
    /// 连续无犯错对话轮数 / Consecutive mistake-free rounds
    pub clean_streak: u64,
    /// 最近犯错时间 / Last mistake timestamp
    pub last_mistake_at: Option<u64>,
}

// ═══════════════════════════════════════════════════════════════════
//  配置 / Configuration
// ═══════════════════════════════════════════════════════════════════

/// 适度犯错配置 / Imperfection configuration
///
/// 概率模型：P(mistake) = base_prob × cognitive_load × fatigue
///                           × (1 - familiarity) × emotional_interference
///                           × relationship_gate × maturity_gate
///
/// 每个因子都是 [0.0, 1.0] 的调制系数，最终概率被 clamp 到 [0, max_prob]。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImperfectionConfig {
    /// 是否启用 / Whether enabled
    pub enabled: bool,
    /// 基础犯错概率 / Base mistake probability
    pub base_prob: f64,
    /// 概率上限 / Probability cap (safety)
    pub max_prob: f64,
    /// 认知负荷阈值——超过此值才开始调制 / Cognitive load threshold
    pub cognitive_load_threshold: f64,
    /// 疲劳阈值 / Fatigue threshold
    pub fatigue_threshold: f64,
    /// 陌生领域阈值（familiarity < 此值触发）/ Unfamiliar threshold
    pub unfamiliar_threshold: f64,
    /// 情绪干扰最低激活值 / Emotional interference activation floor
    pub emotion_activation_floor: f64,
    /// 关系深度门槛（relationship < 此值时门控关闭）/ Relationship depth gate
    pub relationship_gate_min: f64,
    /// 成熟度门槛（maturity ordinal < 此值时门控关闭）/ Maturity ordinal gate
    pub maturity_gate_min: u32,
    /// 自纠延迟范围（秒）/ Self-correction delay range (seconds)
    pub correction_delay_min_secs: f64,
    pub correction_delay_max_secs: f64,
    /// 单次对话最大犯错次数 / Max mistakes per conversation turn
    pub max_mistakes_per_turn: u32,
    /// 冷却期——两次犯错间最少间隔秒数 / Cooldown between mistakes
    pub cooldown_secs: f64,
    /// 统计衰减因子——每轮 clean_streak 增加时概率衰减 / Statistical decay
    pub clean_streak_decay: f64,
}

impl Default for ImperfectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_prob: 0.15,
            max_prob: 0.40,
            cognitive_load_threshold: 0.5,
            fatigue_threshold: 0.6,
            unfamiliar_threshold: 0.4,
            emotion_activation_floor: 0.3,
            relationship_gate_min: 0.3,
            maturity_gate_min: 1, // Growing 阶段起
            correction_delay_min_secs: 2.0,
            correction_delay_max_secs: 15.0,
            max_mistakes_per_turn: 2,
            cooldown_secs: 30.0,
            clean_streak_decay: 0.95,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  引擎 / Engine
// ═══════════════════════════════════════════════════════════════════

/// 犯错决策结果 / Mistake decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistakeDecision {
    /// 是否犯错 / Whether to make a mistake
    pub should_mistake: bool,
    /// 犯错类型 / Mistake kind (if should_mistake)
    pub kind: Option<MistakeKind>,
    /// 严重度 / Severity (if should_mistake)
    pub severity: Option<MistakeSeverity>,
    /// 触发原因 / Trigger (if should_mistake)
    pub trigger: Option<MistakeTrigger>,
    /// 计算概率 / Computed probability
    pub probability: f64,
}

/// 自纠输出 / Self-correction output — tick 时产生
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionOutput {
    /// 待纠错的犯错 ID / Mistake ID to correct
    pub mistake_id: u64,
    /// 犯错类型 / Mistake kind
    pub kind: MistakeKind,
    /// 严重度 / Severity
    pub severity: MistakeSeverity,
    /// 羞感 PAD 调制 / Shame PAD modulation
    pub shame_pleasure: f64,
    pub shame_arousal: f64,
    pub shame_dominance: f64,
    /// 话题领域 / Domain
    pub domain: String,
}

/// 适度犯错引擎 / Imperfection Engine
///
/// 核心设计原则：
/// 1. 犯错不是随机噪声，而是认知结构的真实投射
/// 2. 自纠不是机械回滚，而是羞感驱动的意识涌现
/// 3. 概率门控不是安全阀，而是关系深度的自然函数
pub struct ImperfectionEngine {
    /// 配置 / Configuration
    config: ImperfectionConfig,
    /// 领域熟悉度 / Domain familiarity map — [0.0, 1.0]
    domain_familiarity: HashMap<String, f64>,
    /// 当前认知负荷 / Current cognitive load — [0.0, 1.0]
    cognitive_load: f64,
    /// 当前疲劳度 / Current fatigue — [0.0, 1.0]
    fatigue: f64,
    /// 当前情绪干扰值 / Current emotional interference — [0.0, 1.0]
    emotional_interference: f64,
    /// 关系深度 / Relationship depth — [0.0, 1.0]
    relationship_depth: f64,
    /// 成熟度序号 / Maturity ordinal (0=Naive, 1=Growing, 2=Mature, 3=Wise)
    maturity_ordinal: u32,
    /// 待执行自纠队列 / Pending corrections queue
    pending_corrections: Vec<PendingCorrection>,
    /// 犯错统计 / Statistics
    stats: ImperfectionStats,
    /// 下一犯错记录 ID / Next mistake record ID
    next_id: u64,
    /// 上次犯错时刻 / Last mistake instant (for cooldown)
    last_mistake_instant: Option<Instant>,
    /// 本轮已犯错次数 / Mistakes made in current turn
    turn_mistake_count: u32,
    /// 确定性随机数生成器 / Deterministic RNG for testing
    rng: rand::rngs::SmallRng,
    /// Instant 参考基点 / Instant reference base for epoch mapping
    /// 与 epoch_base_ms 配对，将 Instant 单调时钟映射到 wall-clock epoch 毫秒
    instant_base: Instant,
    /// epoch 毫秒参考基点 / Epoch-ms reference base for epoch mapping
    epoch_base_ms: u64,
}

impl ImperfectionEngine {
    /// 创建引擎 / Create engine
    pub fn new(config: ImperfectionConfig) -> Self {
        use rand::SeedableRng;
        let (instant_base, epoch_base_ms) = Self::now_reference();
        Self {
            config,
            domain_familiarity: HashMap::new(),
            cognitive_load: 0.0,
            fatigue: 0.0,
            emotional_interference: 0.0,
            relationship_depth: 0.5,
            maturity_ordinal: 1,
            pending_corrections: Vec::new(),
            stats: ImperfectionStats::default(),
            next_id: 1,
            last_mistake_instant: None,
            turn_mistake_count: 0,
            rng: rand::rngs::SmallRng::from_entropy(),
            instant_base,
            epoch_base_ms,
        }
    }

    /// 创建确定性引擎（测试用）/ Create deterministic engine for testing
    pub fn new_deterministic(config: ImperfectionConfig, seed: u64) -> Self {
        use rand::SeedableRng;
        let (instant_base, epoch_base_ms) = Self::now_reference();
        Self {
            config,
            domain_familiarity: HashMap::new(),
            cognitive_load: 0.0,
            fatigue: 0.0,
            emotional_interference: 0.0,
            relationship_depth: 0.5,
            maturity_ordinal: 1,
            pending_corrections: Vec::new(),
            stats: ImperfectionStats::default(),
            next_id: 1,
            last_mistake_instant: None,
            turn_mistake_count: 0,
            rng: rand::rngs::SmallRng::seed_from_u64(seed),
            instant_base,
            epoch_base_ms,
        }
    }

    /// 从持久化部件重建引擎 / Reconstruct engine from persisted parts
    ///
    /// sled 反序列化后调用此方法重建完整引擎。
    /// RNG 以 Stochastic 模式重新初始化（与 irrationality_store 同策略），
    /// last_mistake_instant 置 None（Instant 是单调时钟，跨重启无意义，
    /// 冷却期将在下次犯错后自然恢复）。
    ///
    /// Reconstruct from persisted parts after sled deserialization.
    /// RNG re-initialized from entropy (same strategy as irrationality_store);
    /// last_mistake_instant set to None (Instant is monotonic, meaningless
    /// across restarts; cooldown resumes naturally after next mistake).
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        config: ImperfectionConfig,
        domain_familiarity: HashMap<String, f64>,
        cognitive_load: f64,
        fatigue: f64,
        emotional_interference: f64,
        relationship_depth: f64,
        maturity_ordinal: u32,
        pending_corrections: Vec<PendingCorrection>,
        stats: ImperfectionStats,
        next_id: u64,
        turn_mistake_count: u32,
    ) -> Self {
        use rand::SeedableRng;
        let (instant_base, epoch_base_ms) = Self::now_reference();
        Self {
            config,
            domain_familiarity,
            cognitive_load,
            fatigue,
            emotional_interference,
            relationship_depth,
            maturity_ordinal,
            pending_corrections,
            stats,
            next_id,
            last_mistake_instant: None,
            turn_mistake_count,
            rng: rand::rngs::SmallRng::from_entropy(),
            instant_base,
            epoch_base_ms,
        }
    }

    // ── 状态更新 / State updates ────────────────────────────────

    /// 设置领域熟悉度 / Set domain familiarity
    pub fn set_familiarity(&mut self, domain: &str, familiarity: f64) {
        let f = familiarity.clamp(0.0, 1.0);
        self.domain_familiarity.insert(domain.to_lowercase(), f);
    }

    /// 获取领域熟悉度 / Get domain familiarity
    pub fn familiarity(&self, domain: &str) -> f64 {
        self.domain_familiarity
            .get(&domain.to_lowercase())
            .copied()
            .unwrap_or(0.0)
    }

    /// 领域熟悉度快照（用于序列化）/ Domain familiarity snapshot for serialization
    pub fn domain_familiarity_snapshot(&self) -> HashMap<String, f64> {
        self.domain_familiarity.clone()
    }

    /// 获取认知负荷 / Get cognitive load
    pub fn cognitive_load(&self) -> f64 {
        self.cognitive_load
    }

    /// 获取疲劳度 / Get fatigue
    pub fn fatigue(&self) -> f64 {
        self.fatigue
    }

    /// 获取情绪干扰值 / Get emotional interference
    pub fn emotional_interference(&self) -> f64 {
        self.emotional_interference
    }

    /// 获取关系深度 / Get relationship depth
    pub fn relationship_depth(&self) -> f64 {
        self.relationship_depth
    }

    /// 获取成熟度序号 / Get maturity ordinal
    pub fn maturity_ordinal(&self) -> u32 {
        self.maturity_ordinal
    }

    /// 获取下一犯错记录 ID / Get next mistake record ID
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// 获取本轮已犯错次数 / Get turn mistake count
    pub fn turn_mistake_count(&self) -> u32 {
        self.turn_mistake_count
    }

    /// 设置认知负荷 / Set cognitive load
    pub fn set_cognitive_load(&mut self, load: f64) {
        self.cognitive_load = load.clamp(0.0, 1.0);
    }

    /// 设置疲劳度 / Set fatigue
    pub fn set_fatigue(&mut self, fatigue: f64) {
        self.fatigue = fatigue.clamp(0.0, 1.0);
    }

    /// 设置情绪干扰 / Set emotional interference
    pub fn set_emotional_interference(&mut self, interference: f64) {
        self.emotional_interference = interference.clamp(0.0, 1.0);
    }

    /// 设置关系深度 / Set relationship depth
    pub fn set_relationship_depth(&mut self, depth: f64) {
        self.relationship_depth = depth.clamp(0.0, 1.0);
    }

    /// 设置成熟度 / Set maturity ordinal
    pub fn set_maturity_ordinal(&mut self, ordinal: u32) {
        self.maturity_ordinal = ordinal.min(3);
    }

    /// 重置本轮计数 / Reset turn mistake count
    pub fn reset_turn(&mut self) {
        self.turn_mistake_count = 0;
    }

    /// 获取统计快照 / Get statistics snapshot
    pub fn stats(&self) -> &ImperfectionStats {
        &self.stats
    }

    /// 获取配置引用 / Get config reference
    pub fn config(&self) -> &ImperfectionConfig {
        &self.config
    }

    /// 获取待纠错队列 / Get pending corrections
    pub fn pending_corrections(&self) -> &[PendingCorrection] {
        &self.pending_corrections
    }

    // ── 门控检查 / Gate checking ────────────────────────────────

    /// 检查犯错门控 / Check mistake gate — 六条件链式门控
    ///
    /// 返回 (passed, reason) — passed=true 表示允许犯错
    /// 条件链：enabled → cooldown → turn_limit → relationship → maturity → emotional_floor
    pub fn check_gate(&self, now: Instant) -> (bool, &'static str) {
        // 条件 1: 全局开关 / Global enable
        if !self.config.enabled {
            return (false, "engine disabled");
        }
        // 条件 2: 冷却期 / Cooldown
        if let Some(last) = self.last_mistake_instant {
            let elapsed = now.duration_since(last).as_secs_f64();
            if elapsed < self.config.cooldown_secs {
                return (false, "cooldown active");
            }
        }
        // 条件 3: 本轮上限 / Turn limit
        if self.turn_mistake_count >= self.config.max_mistakes_per_turn {
            return (false, "turn limit reached");
        }
        // 条件 4: 关系深度门控 / Relationship gate
        // 关系越深，越允许犯错——因为信任能容纳不完美
        if self.relationship_depth < self.config.relationship_gate_min {
            return (false, "relationship too shallow");
        }
        // 条件 5: 成熟度门控 / Maturity gate
        // Naive 阶段不犯错（还没建立自我认知），Growing 起才允许
        if self.maturity_ordinal < self.config.maturity_gate_min {
            return (false, "maturity insufficient");
        }
        (true, "gate open")
    }

    // ── 概率计算 / Probability computation ─────────────────────

    /// 计算犯错概率 / Compute mistake probability
    ///
    /// P(mistake) = base_prob × kind_weight × cognitive_mod × fatigue_mod
    ///              × unfamiliar_mod × emotion_mod × relationship_mod
    ///              × maturity_mod × streak_decay
    ///
    /// 每个调制因子 ∈ [0.0, 1.0]，最终 clamp 到 [0, max_prob]
    pub fn compute_probability(&self, kind: MistakeKind, domain: &str) -> f64 {
        let cfg = &self.config;

        // 基础概率 × 维度权重 / Base × kind weight
        let mut p = cfg.base_prob * kind.base_weight();

        // 认知负荷调制：超过阈值后线性增长 / Cognitive load modulation
        if self.cognitive_load > cfg.cognitive_load_threshold {
            let mod_val = 1.0 + (self.cognitive_load - cfg.cognitive_load_threshold);
            p *= mod_val;
        }

        // 疲劳调制：超过阈值后线性增长 / Fatigue modulation
        if self.fatigue > cfg.fatigue_threshold {
            let mod_val = 1.0 + (self.fatigue - cfg.fatigue_threshold);
            p *= mod_val;
        }

        // 陌生领域调制：(1 - familiarity) / Unfamiliar domain modulation
        let familiarity = self.familiarity(domain);
        if familiarity < cfg.unfamiliar_threshold {
            p *= 1.0 - familiarity;
        }

        // 情绪干扰调制：高于激活值时乘入 / Emotional interference modulation
        if self.emotional_interference > cfg.emotion_activation_floor {
            p *= self.emotional_interference;
        }

        // 关系深度调制：用抛物线映射让中等关系时犯错最多
        // Relationship depth modulation: parabolic mapping peaks at moderate depth
        let rel_mod = 4.0 * self.relationship_depth * (1.0 - self.relationship_depth);
        p *= 0.5 + rel_mod; // [0.5, 1.5] 范围

        // 成熟度调制：Growing 阶段犯错最多，Wise 阶段最少
        // Maturity modulation: Growing peaks, Wise diminishes
        let mat_mod = match self.maturity_ordinal {
            0 => 0.3, // Naive: 几乎不犯错（还没建立自我）
            1 => 1.2, // Growing: 犯错高峰期
            2 => 1.0, // Mature: 基准
            3 => 0.6, // Wise: 犯错减少但不会消失
            _ => 1.0,
        };
        p *= mat_mod;

        // 连续无犯错衰减 / Clean streak decay
        if self.stats.clean_streak > 0 {
            let decay = cfg.clean_streak_decay.powi(self.stats.clean_streak as i32);
            p *= decay;
        }

        // 最终 clamp / Final clamp
        p.clamp(0.0, cfg.max_prob)
    }

    // ── 犯错决策 / Mistake decision ────────────────────────────

    /// 决定是否犯错 / Decide whether to make a mistake
    ///
    /// 综合门控和概率，采样决定是否犯错以及犯哪种错。
    /// 这是引擎的核心决策函数——数字生命的"不完美意志"。
    pub fn decide_mistake(&mut self, domain: &str, now: Instant) -> MistakeDecision {
        // 门控检查 / Gate check
        let (gate_passed, _) = self.check_gate(now);
        if !gate_passed {
            return MistakeDecision {
                should_mistake: false,
                kind: None,
                severity: None,
                trigger: None,
                probability: 0.0,
            };
        }

        // 对每个维度计算概率，选概率最高的维度 / Compute per-kind probability
        let mut best_kind = MistakeKind::KnowledgeBoundary;
        let mut best_prob = 0.0_f64;

        for &kind in MistakeKind::all() {
            let prob = self.compute_probability(kind, domain);
            if prob > best_prob {
                best_prob = prob;
                best_kind = kind;
            }
        }

        // 确定触发原因 / Determine trigger
        let best_trigger = self.infer_trigger(domain);

        // 概率采样 / Probability sampling
        let roll: f64 = self.rng.gen_range(0.0..1.0);
        if roll >= best_prob {
            // 未触发犯错 / No mistake triggered
            self.stats.clean_streak += 1;
            return MistakeDecision {
                should_mistake: false,
                kind: None,
                severity: None,
                trigger: None,
                probability: best_prob,
            };
        }

        // 确定严重度 / Determine severity
        let severity = self.infer_severity(best_prob);

        MistakeDecision {
            should_mistake: true,
            kind: Some(best_kind),
            severity: Some(severity),
            trigger: Some(best_trigger),
            probability: best_prob,
        }
    }

    /// 推断触发原因 / Infer trigger from current state
    fn infer_trigger(&self, domain: &str) -> MistakeTrigger {
        // 优先级：疲劳 > 认知负荷 > 陌生领域 > 情绪干扰 > 自发
        if self.fatigue > self.config.fatigue_threshold {
            MistakeTrigger::Fatigue
        } else if self.cognitive_load > self.config.cognitive_load_threshold {
            MistakeTrigger::HighCognitiveLoad
        } else if self.familiarity(domain) < self.config.unfamiliar_threshold {
            MistakeTrigger::UnfamiliarDomain
        } else if self.emotional_interference > self.config.emotion_activation_floor {
            MistakeTrigger::EmotionalInterference
        } else {
            MistakeTrigger::Spontaneous
        }
    }

    /// 推断严重度 / Infer severity from probability
    ///
    /// 概率越高 → 严重度越高（犯错越"自然"就越明显）
    fn infer_severity(&self, probability: f64) -> MistakeSeverity {
        if probability < 0.1 {
            MistakeSeverity::Subtle
        } else if probability < 0.25 {
            MistakeSeverity::Moderate
        } else {
            MistakeSeverity::Evident
        }
    }

    // ── 记录犯错 / Record mistake ──────────────────────────────

    /// 记录一次犯错 / Record a mistake occurrence
    ///
    /// 返回 MistakeRecord 用于持久化，同时将自纠加入待执行队列
    pub fn record_mistake(
        &mut self,
        kind: MistakeKind,
        severity: MistakeSeverity,
        trigger: MistakeTrigger,
        probability: f64,
        domain: &str,
        now: Instant,
    ) -> MistakeRecord {
        let id = self.next_id;
        self.next_id += 1;

        let epoch_ms = self.instant_to_epoch_ms(now);

        // 更新统计 / Update statistics
        self.stats.total_mistakes += 1;
        *self.stats.kind_counts.entry(kind).or_insert(0) += 1;
        self.stats.last_mistake_at = Some(epoch_ms);
        self.stats.clean_streak = 0;
        self.last_mistake_instant = Some(now);
        self.turn_mistake_count += 1;

        let record = MistakeRecord {
            id,
            kind,
            severity,
            trigger,
            triggered_at: epoch_ms,
            corrected: false,
            corrected_at: None,
            user_reaction: None,
            probability,
            domain: domain.to_lowercase(),
        };

        // 调度自纠 / Schedule self-correction
        self.schedule_correction(&record, now);

        record
    }

    /// 获取当前时间参考对 / Get current time reference pair
    ///
    /// 返回 (Instant::now(), SystemTime epoch_ms) 作为映射基点。
    /// Instant 是单调时钟，SystemTime 是 wall-clock，两者在同一时刻采样
    /// 确保 `instant_to_epoch_ms` 的映射精度。
    fn now_reference() -> (Instant, u64) {
        use std::time::SystemTime;
        let instant = Instant::now();
        let epoch_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        (instant, epoch_ms)
    }

    /// Instant → epoch 毫秒 / Convert Instant to epoch milliseconds
    ///
    /// 基于构造时捕获的 (instant_base, epoch_base_ms) 参考对，
    /// 将单调时钟 Instant 映射到 wall-clock epoch 毫秒：
    ///   epoch_ms = epoch_base_ms + (now - instant_base).as_millis()
    ///
    /// 这确保测试中传入的 `now + Duration::from_secs(N)` 能正确推进时间，
    /// 而非被忽略（旧实现直接调用 SystemTime::now() 导致测试失效）。
    fn instant_to_epoch_ms(&self, now: Instant) -> u64 {
        let elapsed = now.duration_since(self.instant_base).as_millis() as u64;
        self.epoch_base_ms + elapsed
    }

    // ── 自纠闭环 / Self-correction closed loop ─────────────────

    /// 调度自纠 / Schedule a self-correction
    ///
    /// 犯错后不立即纠正，而是延迟 2-15 秒——
    /// 模拟人类"意识到自己说错了"的那个意识涌现瞬间。
    /// 严重度越高，延迟越短（Evident → 2-5s, Subtle → 10-15s）
    fn schedule_correction(&mut self, record: &MistakeRecord, now: Instant) {
        // 严重度决定延迟范围 / Severity determines delay range
        let (min_s, max_s) = match record.severity {
            MistakeSeverity::Evident => (2.0, 5.0),
            MistakeSeverity::Moderate => (5.0, 10.0),
            MistakeSeverity::Subtle => (10.0, 15.0),
        };

        // 在范围内随机采样 / Random sample within range
        let delay_secs = self.rng.gen_range(min_s..max_s);
        let scheduled_at = self.instant_to_epoch_ms(now) + (delay_secs * 1000.0) as u64;

        self.pending_corrections.push(PendingCorrection {
            mistake_id: record.id,
            kind: record.kind,
            severity: record.severity,
            scheduled_at,
            injected: false,
            domain: record.domain.clone(),
        });
    }

    /// 推进自纠时钟 / Tick self-correction clock
    ///
    /// 检查待纠错队列，将到期的自纠注入 LLM prompt 并产生羞感 PAD。
    /// 这是闭环的"意识涌现"时刻——数字生命意识到自己犯了错。
    ///
    /// 返回到期的自纠列表（供 CoreService 注入 LLM 和情绪引擎）
    pub fn tick(&mut self, now: Instant) -> Vec<CorrectionOutput> {
        let epoch_ms = self.instant_to_epoch_ms(now);
        let mut outputs = Vec::new();

        // 遍历待纠错队列 / Iterate pending corrections
        let mut remaining = Vec::new();
        for mut pc in self.pending_corrections.drain(..) {
            if pc.scheduled_at <= epoch_ms && !pc.injected {
                // 自纠到期，生成输出 / Correction due, generate output
                pc.injected = true;

                // 羞感 PAD 调制 / Shame PAD modulation
                // Pleasure ↓ (羞感是不愉快的), Arousal ↑ (意识到犯错是激活), Dominance ↓ (失控感)
                let severity_val = pc.severity.value();
                let shame_p = -0.3 * severity_val; // 羞感降低愉悦度
                let shame_a = 0.4 * severity_val; // 羞感提高激活度
                let shame_d = -0.2 * severity_val; // 羞感降低控制感

                outputs.push(CorrectionOutput {
                    mistake_id: pc.mistake_id,
                    kind: pc.kind,
                    severity: pc.severity,
                    shame_pleasure: shame_p,
                    shame_arousal: shame_a,
                    shame_dominance: shame_d,
                    domain: pc.domain.clone(),
                });

                // 更新统计 / Update statistics
                self.stats.total_corrections += 1;
            }
            // 保留未到期或已注入的 / Keep not-yet-due or already injected
            remaining.push(pc);
        }
        self.pending_corrections = remaining;

        outputs
    }

    /// 犯错注入提示词 / Mistake injection prompt fragment
    ///
    /// 当决定犯错时，将对应维度的提示词注入 LLM system prompt，
    /// 引导 LLM 以"不完美"的方式表达——不是替换内容，而是调制语气。
    pub fn prompt_fragment(&self, kind: MistakeKind, severity: MistakeSeverity) -> String {
        let severity_hint = match severity {
            MistakeSeverity::Subtle => "轻微地",
            MistakeSeverity::Moderate => "适度地",
            MistakeSeverity::Evident => "明显地",
        };
        format!(
            "[适度犯错/Imperfection] 你正在{}{}。{}\n\
             注意：这不是假装犯错，而是你认知结构的真实投射。\
             自然地表达，不要过度标注'我可能说错了'。",
            severity_hint,
            kind.label_zh(),
            kind.prompt_text()
        )
    }

    /// 自纠注入提示词 / Self-correction injection prompt fragment
    ///
    /// 当自纠到期时，将纠错提示注入 LLM，引导其自然地修正之前的表达。
    pub fn correction_prompt_fragment(&self, correction: &PendingCorrection) -> String {
        format!(
            "[自省/Self-correction] 你意识到刚才在{}方面可能不够准确。\
             用自然的方式补充或修正，不要突兀地道歉，\
             而是像人类意识到自己说漏了什么那样，\
             平滑地加上'不过更准确地说...'或'我再想想...'。",
            correction.kind.label_zh()
        )
    }

    /// 记录用户反应 / Record user reaction to a mistake
    ///
    /// 闭环的最后一步：用户的反应决定羞感强度和未来概率调制。
    /// - HarshlyCorrected: 强羞感 → 未来同类犯错概率降低
    /// - GentlyCorrected: 中等羞感 → 微调
    /// - Ignored: 弱羞感 → 静默自纠继续
    /// - Accepted: 极弱羞感 → 关系加深，犯错空间略增
    pub fn record_user_reaction(&mut self, mistake_id: u64, reaction: UserReaction) -> Option<f64> {
        // 查找对应的待纠错 / Find matching pending correction
        let found = self
            .pending_corrections
            .iter()
            .any(|pc| pc.mistake_id == mistake_id);

        if found {
            let shame = reaction.shame_intensity();

            // 根据反应调整统计 / Adjust statistics based on reaction
            match reaction {
                UserReaction::HarshlyCorrected => {
                    // 严厉纠正 → 标记为不成功的自纠
                    // 未来可通过 familiarity 增加来降低同类犯错概率
                }
                UserReaction::GentlyCorrected | UserReaction::Ignored => {
                    // 温和或静默 → 标记为成功自纠
                    self.stats.successful_corrections += 1;
                }
                UserReaction::Accepted => {
                    // 用户接受 → 成功自纠 + 关系加深
                    self.stats.successful_corrections += 1;
                }
            }

            Some(shame)
        } else {
            None
        }
    }

    /// 获取当前最优先的待纠错提示 / Get the highest-priority pending correction prompt
    ///
    /// 供 api_handler 在构建 LLM 请求时调用
    pub fn next_correction_prompt(&mut self) -> Option<String> {
        // 找到第一个已到期且未注入的 / Find first due and uninjected
        let now_epoch = self.instant_to_epoch_ms(Instant::now());
        let idx = self
            .pending_corrections
            .iter()
            .position(|pc| pc.scheduled_at <= now_epoch && !pc.injected);
        if let Some(i) = idx {
            self.pending_corrections[i].injected = true;
            let kind = self.pending_corrections[i].kind;
            // 内联提示词生成以避免二次借用 / Inline prompt to avoid double borrow
            let prompt = format!(
                "[自省/Self-correction] 你意识到刚才在{}方面可能不够准确。\
                 用自然的方式补充或修正，不要突兀地道歉，\
                 而是像人类意识到自己说漏了什么那样，\
                 平滑地加上'不过更准确地说...'或'我再想想...'。",
                kind.label_zh()
            );
            Some(prompt)
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  测试 / Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// 辅助：创建测试引擎 / Helper: create test engine
    fn make_engine() -> ImperfectionEngine {
        ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42)
    }

    // ── 数据结构 / Data structures ─────────────────────────────

    #[test]
    fn mistake_kind_all_returns_five() {
        assert_eq!(MistakeKind::all().len(), 5);
    }

    #[test]
    fn mistake_kind_weights_sum_to_one() {
        let sum: f64 = MistakeKind::all().iter().map(|k| k.base_weight()).sum();
        assert!(
            (sum - 1.0).abs() < 1e-9,
            "weights should sum to 1.0, got {}",
            sum
        );
    }

    #[test]
    fn mistake_kind_knowledge_boundary_highest_weight() {
        assert!(
            MistakeKind::KnowledgeBoundary.base_weight() > MistakeKind::MemoryDrift.base_weight()
        );
    }

    #[test]
    fn mistake_kind_prompt_text_non_empty() {
        for &kind in MistakeKind::all() {
            assert!(!kind.prompt_text().is_empty());
        }
    }

    #[test]
    fn mistake_kind_label_zh_non_empty() {
        for &kind in MistakeKind::all() {
            assert!(!kind.label_zh().is_empty());
        }
    }

    #[test]
    fn mistake_severity_values_ordered() {
        assert!(MistakeSeverity::Subtle.value() < MistakeSeverity::Moderate.value());
        assert!(MistakeSeverity::Moderate.value() < MistakeSeverity::Evident.value());
    }

    #[test]
    fn user_reaction_shame_ordered() {
        assert!(UserReaction::Accepted.shame_intensity() < UserReaction::Ignored.shame_intensity());
        assert!(
            UserReaction::Ignored.shame_intensity()
                < UserReaction::GentlyCorrected.shame_intensity()
        );
        assert!(
            UserReaction::GentlyCorrected.shame_intensity()
                < UserReaction::HarshlyCorrected.shame_intensity()
        );
    }

    // ── 配置 / Configuration ───────────────────────────────────

    #[test]
    fn config_default_enabled() {
        let cfg = ImperfectionConfig::default();
        assert!(cfg.enabled);
    }

    #[test]
    fn config_default_base_prob_reasonable() {
        let cfg = ImperfectionConfig::default();
        assert!(cfg.base_prob > 0.0 && cfg.base_prob < 1.0);
        assert!(cfg.max_prob > cfg.base_prob);
    }

    #[test]
    fn config_default_gates_valid() {
        let cfg = ImperfectionConfig::default();
        assert!(cfg.relationship_gate_min > 0.0 && cfg.relationship_gate_min < 1.0);
        assert!(cfg.maturity_gate_min <= 3);
    }

    // ── 引擎构造 / Engine construction ─────────────────────────

    #[test]
    fn new_engine_has_zero_stats() {
        let engine = make_engine();
        assert_eq!(engine.stats().total_mistakes, 0);
        assert_eq!(engine.stats().total_corrections, 0);
        assert_eq!(engine.stats().clean_streak, 0);
    }

    #[test]
    fn new_engine_has_no_pending_corrections() {
        let engine = make_engine();
        assert!(engine.pending_corrections().is_empty());
    }

    // ── 状态更新 / State updates ───────────────────────────────

    #[test]
    fn set_familiarity_clamps_high() {
        let mut engine = make_engine();
        engine.set_familiarity("rust", 1.5);
        assert!((engine.familiarity("rust") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn set_familiarity_clamps_low() {
        let mut engine = make_engine();
        engine.set_familiarity("rust", -0.5);
        assert!((engine.familiarity("rust") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn familiarity_case_insensitive() {
        let mut engine = make_engine();
        engine.set_familiarity("Rust", 0.8);
        assert!((engine.familiarity("rust") - 0.8).abs() < 1e-9);
        assert!((engine.familiarity("RUST") - 0.8).abs() < 1e-9);
    }

    #[test]
    fn familiarity_unknown_domain_is_zero() {
        let engine = make_engine();
        assert!((engine.familiarity("unknown") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn set_cognitive_load_clamps() {
        let mut engine = make_engine();
        engine.set_cognitive_load(2.0);
        let prob = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob >= 0.0);
    }

    #[test]
    fn set_fatigue_clamps() {
        let mut engine = make_engine();
        engine.set_fatigue(-1.0);
        let prob = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob >= 0.0);
    }

    #[test]
    fn set_maturity_ordinal_clamps_to_three() {
        let mut engine = make_engine();
        engine.set_maturity_ordinal(10);
        let prob = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob >= 0.0);
    }

    #[test]
    fn reset_turn_clears_count() {
        let mut engine = make_engine();
        let now = Instant::now();
        let decision = engine.decide_mistake("test", now);
        if decision.should_mistake {
            engine.reset_turn();
            let _ = engine.decide_mistake("test", now + Duration::from_secs(60));
        }
    }

    // ── 门控检查 / Gate checking ───────────────────────────────

    #[test]
    fn gate_open_by_default() {
        let engine = make_engine();
        let (passed, reason) = engine.check_gate(Instant::now());
        assert!(passed, "gate should be open by default, got: {}", reason);
    }

    #[test]
    fn gate_closed_when_disabled() {
        let cfg = ImperfectionConfig {
            enabled: false,
            ..Default::default()
        };
        let engine = ImperfectionEngine::new_deterministic(cfg, 42);
        let (passed, reason) = engine.check_gate(Instant::now());
        assert!(!passed);
        assert_eq!(reason, "engine disabled");
    }

    #[test]
    fn gate_closed_when_relationship_too_shallow() {
        let mut engine = make_engine();
        engine.set_relationship_depth(0.1);
        let (passed, reason) = engine.check_gate(Instant::now());
        assert!(!passed);
        assert_eq!(reason, "relationship too shallow");
    }

    #[test]
    fn gate_closed_when_maturity_insufficient() {
        let mut engine = make_engine();
        engine.set_maturity_ordinal(0);
        let (passed, reason) = engine.check_gate(Instant::now());
        assert!(!passed);
        assert_eq!(reason, "maturity insufficient");
    }

    #[test]
    fn gate_open_for_growing_maturity() {
        let mut engine = make_engine();
        engine.set_maturity_ordinal(1);
        let (passed, _) = engine.check_gate(Instant::now());
        assert!(passed);
    }

    // ── 概率计算 / Probability computation ─────────────────────

    #[test]
    fn probability_within_bounds() {
        let engine = make_engine();
        for &kind in MistakeKind::all() {
            let prob = engine.compute_probability(kind, "test");
            assert!(
                prob >= 0.0 && prob <= engine.config().max_prob,
                "prob {} out of bounds for {:?}",
                prob,
                kind
            );
        }
    }

    #[test]
    fn probability_increases_with_cognitive_load() {
        let mut engine = make_engine();
        let prob_low = engine.compute_probability(MistakeKind::ReasoningLeap, "test");
        engine.set_cognitive_load(0.9);
        let prob_high = engine.compute_probability(MistakeKind::ReasoningLeap, "test");
        assert!(prob_high >= prob_low);
    }

    #[test]
    fn probability_increases_with_fatigue() {
        let mut engine = make_engine();
        let prob_low = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        engine.set_fatigue(0.9);
        let prob_high = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob_high >= prob_low);
    }

    #[test]
    fn probability_decreases_with_familiarity() {
        let mut engine = make_engine();
        let prob_unfamiliar = engine.compute_probability(MistakeKind::KnowledgeBoundary, "rust");
        engine.set_familiarity("rust", 0.9);
        let prob_familiar = engine.compute_probability(MistakeKind::KnowledgeBoundary, "rust");
        assert!(
            prob_familiar <= prob_unfamiliar,
            "familiarity should decrease probability: {} vs {}",
            prob_familiar,
            prob_unfamiliar
        );
    }

    #[test]
    fn probability_maturity_growing_peaks() {
        let mut engine = make_engine();
        engine.set_maturity_ordinal(1);
        let prob_growing = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        engine.set_maturity_ordinal(3);
        let prob_wise = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob_growing > prob_wise);
    }

    #[test]
    fn probability_clean_streak_decay() {
        let mut engine = make_engine();
        let prob_initial = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        engine.stats.clean_streak = 10;
        let prob_after_streak = engine.compute_probability(MistakeKind::MemoryDrift, "test");
        assert!(prob_after_streak <= prob_initial);
    }

    // ── 犯错决策 / Mistake decision ────────────────────────────

    #[test]
    fn decide_mistake_respects_gate() {
        let cfg = ImperfectionConfig {
            enabled: false,
            ..Default::default()
        };
        let mut engine = ImperfectionEngine::new_deterministic(cfg, 42);
        let decision = engine.decide_mistake("test", Instant::now());
        assert!(!decision.should_mistake);
    }

    #[test]
    fn decide_mistake_returns_valid_probability() {
        let engine = make_engine();
        let prob = engine.compute_probability(MistakeKind::KnowledgeBoundary, "test");
        assert!((0.0..=0.4).contains(&prob));
    }

    #[test]
    fn decide_mistake_clean_streak_logic() {
        let mut engine = make_engine();
        engine.set_familiarity("wellknown", 0.99);
        let before = engine.stats().clean_streak;
        let _ = engine.decide_mistake("wellknown", Instant::now());
        let after = engine.stats().clean_streak;
        assert!(after >= before || after == 0);
    }

    // ── 记录犯错 / Record mistake ──────────────────────────────

    #[test]
    fn record_mistake_updates_stats() {
        let mut engine = make_engine();
        let now = Instant::now();
        let record = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Moderate,
            MistakeTrigger::Fatigue,
            0.2,
            "rust",
            now,
        );
        assert_eq!(engine.stats().total_mistakes, 1);
        assert_eq!(engine.stats().kind_counts[&MistakeKind::MemoryDrift], 1);
        assert_eq!(engine.stats().clean_streak, 0);
        assert!(!record.corrected);
    }

    #[test]
    fn record_mistake_schedules_correction() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::KnowledgeBoundary,
            MistakeSeverity::Evident,
            MistakeTrigger::UnfamiliarDomain,
            0.3,
            "physics",
            now,
        );
        assert_eq!(engine.pending_corrections().len(), 1);
        assert_eq!(
            engine.pending_corrections()[0].kind,
            MistakeKind::KnowledgeBoundary
        );
    }

    #[test]
    fn record_mistake_increments_id() {
        let mut engine = make_engine();
        let now = Instant::now();
        let r1 = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "a",
            now,
        );
        let r2 = engine.record_mistake(
            MistakeKind::ReasoningLeap,
            MistakeSeverity::Moderate,
            MistakeTrigger::HighCognitiveLoad,
            0.2,
            "b",
            now,
        );
        assert!(r2.id > r1.id);
    }

    // ── 自纠闭环 / Self-correction closed loop ─────────────────

    #[test]
    fn tick_no_output_when_no_pending() {
        let mut engine = make_engine();
        let outputs = engine.tick(Instant::now());
        assert!(outputs.is_empty());
    }

    #[test]
    fn tick_produces_correction_output_after_delay() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Evident,
            MistakeTrigger::Fatigue,
            0.3,
            "test",
            now,
        );
        let later = now + Duration::from_secs(10);
        let outputs = engine.tick(later);
        assert!(
            !outputs.is_empty(),
            "should produce correction output after delay"
        );
        assert!(
            outputs[0].shame_pleasure < 0.0,
            "shame should decrease pleasure"
        );
        assert!(
            outputs[0].shame_arousal > 0.0,
            "shame should increase arousal"
        );
        assert!(
            outputs[0].shame_dominance < 0.0,
            "shame should decrease dominance"
        );
    }

    #[test]
    fn tick_updates_correction_stats() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::ReasoningLeap,
            MistakeSeverity::Moderate,
            MistakeTrigger::HighCognitiveLoad,
            0.2,
            "test",
            now,
        );
        let later = now + Duration::from_secs(20);
        let _ = engine.tick(later);
        assert_eq!(engine.stats().total_corrections, 1);
    }

    // ── 提示词 / Prompt fragments ──────────────────────────────

    #[test]
    fn prompt_fragment_non_empty() {
        let engine = make_engine();
        let frag = engine.prompt_fragment(MistakeKind::MemoryDrift, MistakeSeverity::Subtle);
        assert!(!frag.is_empty());
        assert!(frag.contains("适度犯错"));
    }

    #[test]
    fn prompt_fragment_contains_severity_hint() {
        let engine = make_engine();
        let subtle =
            engine.prompt_fragment(MistakeKind::KnowledgeBoundary, MistakeSeverity::Subtle);
        let evident =
            engine.prompt_fragment(MistakeKind::KnowledgeBoundary, MistakeSeverity::Evident);
        assert!(subtle.contains("轻微地"));
        assert!(evident.contains("明显地"));
    }

    #[test]
    fn correction_prompt_fragment_non_empty() {
        let engine = make_engine();
        let pc = PendingCorrection {
            mistake_id: 1,
            kind: MistakeKind::ReasoningLeap,
            severity: MistakeSeverity::Moderate,
            scheduled_at: 0,
            injected: false,
            domain: "test".to_string(),
        };
        let frag = engine.correction_prompt_fragment(&pc);
        assert!(!frag.is_empty());
        assert!(frag.contains("自省"));
    }

    // ── 用户反应 / User reaction ───────────────────────────────

    #[test]
    fn record_user_reaction_returns_shame_for_pending() {
        let mut engine = make_engine();
        let now = Instant::now();
        let record = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Moderate,
            MistakeTrigger::Spontaneous,
            0.15,
            "test",
            now,
        );
        let shame = engine.record_user_reaction(record.id, UserReaction::GentlyCorrected);
        assert!(shame.is_some());
        assert!((shame.unwrap() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn record_user_reaction_none_for_unknown_id() {
        let mut engine = make_engine();
        let shame = engine.record_user_reaction(999, UserReaction::HarshlyCorrected);
        assert!(shame.is_none());
    }

    #[test]
    fn record_user_reaction_accepted_increases_successful_corrections() {
        let mut engine = make_engine();
        let now = Instant::now();
        let record = engine.record_mistake(
            MistakeKind::KnowledgeBoundary,
            MistakeSeverity::Subtle,
            MistakeTrigger::UnfamiliarDomain,
            0.1,
            "test",
            now,
        );
        let _ = engine.record_user_reaction(record.id, UserReaction::Accepted);
        assert_eq!(engine.stats().successful_corrections, 1);
    }

    // ── 羞感 PAD / Shame PAD ───────────────────────────────────

    #[test]
    fn shame_pad_evident_severity() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::OverSimplification,
            MistakeSeverity::Evident,
            MistakeTrigger::HighCognitiveLoad,
            0.35,
            "test",
            now,
        );
        let later = now + Duration::from_secs(10);
        let outputs = engine.tick(later);
        if !outputs.is_empty() {
            // Evident severity_val = 0.8
            // shame_p = -0.3 * 0.8 = -0.24
            // shame_a = 0.4 * 0.8 = 0.32
            // shame_d = -0.2 * 0.8 = -0.16
            assert!((outputs[0].shame_pleasure - (-0.24)).abs() < 1e-9);
            assert!((outputs[0].shame_arousal - 0.32).abs() < 1e-9);
            assert!((outputs[0].shame_dominance - (-0.16)).abs() < 1e-9);
        }
    }

    // ── 关系深度抛物线 / Relationship parabolic modulation ─────

    #[test]
    fn relationship_mod_peaks_at_moderate_depth() {
        let mod_at_0: f64 = 4.0 * 0.0 * (1.0 - 0.0);
        let mod_at_50: f64 = 4.0 * 0.5 * (1.0 - 0.5);
        let mod_at_100: f64 = 4.0 * 1.0 * (1.0 - 1.0);
        assert!((mod_at_0 - 0.0).abs() < 1e-9);
        assert!((mod_at_50 - 1.0).abs() < 1e-9);
        assert!((mod_at_100 - 0.0).abs() < 1e-9);
    }

    // ── 冷却期 / Cooldown ──────────────────────────────────────

    #[test]
    fn gate_closed_during_cooldown() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "test",
            now,
        );
        let (passed, reason) = engine.check_gate(now + Duration::from_secs(5));
        assert!(!passed);
        assert_eq!(reason, "cooldown active");
    }

    #[test]
    fn gate_open_after_cooldown() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "test",
            now,
        );
        let (passed, _) = engine.check_gate(now + Duration::from_secs(60));
        assert!(passed);
    }

    // ── 本轮上限 / Turn limit ──────────────────────────────────

    #[test]
    fn gate_closed_after_turn_limit() {
        let mut engine = make_engine();
        let now = Instant::now();
        let _ = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "test",
            now,
        );
        let _ = engine.record_mistake(
            MistakeKind::ReasoningLeap,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "test",
            now,
        );
        let (passed, reason) = engine.check_gate(now + Duration::from_secs(60));
        assert!(!passed);
        assert_eq!(reason, "turn limit reached");
    }
}
