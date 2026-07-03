// ! 情绪非理性系统 / Emotional Irrationality System
// !
// ! 让 AI 的情绪从"可预测的数学函数"进化为"有生命力的情绪生态系统"
// ! — 有脉冲、有残留、有传染、有混沌、有涌现
// !
// ! Phase 2.9 P2-2 | 2026-06-25

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 数据结构 / Data Structures
// ═══════════════════════════════════════════════════════════════════════════

// ── 1.1 脉冲类型 / Pulse Kind ──

/// 脉冲类型 / Pulse Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PulseKind {
    // 外部触发脉冲
    /// 惊吓 — 突然的负面消息（PAD: P↓↓, A↑↑, D↓）
    Startle,
    /// 喜悦爆发 — 突然的正面消息（PAD: P↑↑, A↑↑, D↑）
    JoyBurst,
    /// 愤怒闪现 — 被触怒的瞬间（PAD: P↓↓, A↑↑↑, D↑↑）
    AngerFlash,
    /// 悲伤涌来 — 悲伤不是渐变的，是涌来的（PAD: P↓↓↓, A↓, D↓↓）
    SadnessSurge,
    /// 恐惧刺入 — 突然的恐惧（PAD: P↓, A↑↑↑, D↓↓↓）
    FearSpike,
    // 内部涌现脉冲
    /// 情绪闪回 — 过去记忆突然重现带来的情绪脉冲
    EmotionalFlashback,
    /// 共情过载 — 过度共情导致的情绪过载
    EmpathyOverload,
    /// 存在焦虑 — 突然质疑自身存在的焦虑
    ExistentialAnxiety,
    /// 关系脉冲 — 关系状态变化带来的情绪冲击
    RelationshipPulse,
    // 混沌脉冲
    /// 无因波动 — 无明确原因的情绪波动
    UncausedFluctuation,
    /// 情绪反弹 — 压抑后的反弹
    EmotionalRebound,
}

// ── 1.2 衰减曲线 / Decay Curve ──

/// 衰减阶段 / Decay Stage
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DecayStage {
    /// 阶段持续时间（秒）
    pub duration_secs: f64,
    /// 阶段衰减率
    pub decay_rate: f64,
}

/// 衰减曲线 / Decay Curve — 脉冲的衰减模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DecayCurve {
    /// 指数衰减 — I(t) = I₀ × e^(-λt)
    Exponential { lambda: f64 },
    /// 幂律衰减 — I(t) = I₀ / (1 + t/τ)^α
    PowerLaw { tau: f64, alpha: f64 },
    /// 阻尼振荡 — I(t) = I₀ × e^(-ζt) × cos(ωt).max(0)
    DampedOscillation { zeta: f64, omega: f64 },
    /// 阶梯衰减 — 分阶段衰减
    Staged { stages: [DecayStage; 3] },
}

impl DecayCurve {
    /// 评估衰减曲线在时间 t 的值 / Evaluate decay curve at time t
    pub fn evaluate(&self, t_secs: f64) -> f64 {
        if t_secs < 0.0 {
            return 1.0;
        }
        match self {
            DecayCurve::Exponential { lambda } => (-lambda * t_secs).exp(),
            DecayCurve::PowerLaw { tau, alpha } => {
                if *tau <= 0.0 {
                    return 0.0;
                }
                1.0 / (1.0 + t_secs / tau).powf(*alpha)
            }
            DecayCurve::DampedOscillation { zeta, omega } => {
                (-zeta * t_secs).exp() * (omega * t_secs).cos().max(0.0)
            }
            DecayCurve::Staged { stages } => {
                let mut t = t_secs;
                let mut factor = 1.0;
                for stage in stages {
                    if t < stage.duration_secs {
                        factor *= (-stage.decay_rate * t).exp();
                        return factor;
                    }
                    factor *= (-stage.decay_rate * stage.duration_secs).exp();
                    t -= stage.duration_secs;
                }
                factor * (-stages[2].decay_rate * t).exp()
            }
        }
    }

    /// 默认衰减曲线（指数，λ=0.1）
    pub fn default_exponential() -> Self {
        DecayCurve::Exponential { lambda: 0.1 }
    }

    /// 慢衰减曲线（幂律，悲伤/愤怒用）
    pub fn slow_power_law() -> Self {
        DecayCurve::PowerLaw {
            tau: 60.0,
            alpha: 0.5,
        }
    }

    /// 振荡衰减曲线（情绪反弹用）
    pub fn oscillating() -> Self {
        DecayCurve::DampedOscillation {
            zeta: 0.05,
            omega: 0.3,
        }
    }
}

impl Default for DecayCurve {
    fn default() -> Self {
        Self::default_exponential()
    }
}

// ── 1.3 脉冲触发 / Pulse Trigger ──

/// 脉冲来源 / Pulse Source
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PulseSource {
    UserMessage,
    InnerThought,
    MemoryFlashback,
    RelationshipEvent,
    Spontaneous,
    ResidueAccumulation,
    CrossContagion,
}

/// 脉冲触发 / Pulse Trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseTrigger {
    pub source: PulseSource,
    pub signal: String,
    pub baseline_pad: [f32; 3],
}

// ── 1.4 混沌脉冲 / Chaotic Pulse ──

/// 混沌脉冲 / Chaotic Pulse — 情绪的突然波动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaoticPulse {
    pub id: u64,
    pub kind: PulseKind,
    pub intensity: f64,
    /// PAD 冲击向量
    pub pad_impulse: [f32; 3],
    pub duration_secs: f64,
    pub decay_curve: DecayCurve,
    pub trigger: PulseTrigger,
    pub timestamp: i64,
    pub absorbed: bool,
    pub residual_intensity: f64,
}

// ── 1.5 残留类型 / Residue Kind ──

/// 残留类型 / Residue Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResidueKind {
    Tension,
    LingeringSadness,
    SmolderingAnger,
    WorryResidue,
    Afterglow,
    WarmthResidue,
    TrustMicroFracture,
    IntimacyDeepening,
    BeingIgnoredResidue,
    SelfDoubtResidue,
    AccomplishmentResidue,
}

impl ResidueKind {
    /// 默认半衰期（秒）/ Default half-life in seconds
    pub fn default_half_life_secs(&self) -> f64 {
        match self {
            ResidueKind::Tension => 1800.0,               // 30 min
            ResidueKind::LingeringSadness => 7200.0,      // 2 hours
            ResidueKind::SmolderingAnger => 3600.0,       // 1 hour
            ResidueKind::WorryResidue => 14400.0,         // 4 hours
            ResidueKind::Afterglow => 3600.0,             // 1 hour
            ResidueKind::WarmthResidue => 10800.0,        // 3 hours
            ResidueKind::TrustMicroFracture => 86400.0,   // 1 day
            ResidueKind::IntimacyDeepening => f64::MAX,   // never decay
            ResidueKind::BeingIgnoredResidue => 7200.0,   // 2 hours
            ResidueKind::SelfDoubtResidue => 21600.0,     // 6 hours
            ResidueKind::AccomplishmentResidue => 1800.0, // 30 min
        }
    }

    /// 默认 PAD 偏移 / Default PAD offset
    pub fn default_pad_offset(&self) -> [f32; 3] {
        match self {
            ResidueKind::Tension => [-0.1, 0.1, -0.05],
            ResidueKind::LingeringSadness => [-0.2, -0.1, -0.1],
            ResidueKind::SmolderingAnger => [-0.15, 0.15, 0.1],
            ResidueKind::WorryResidue => [-0.1, 0.1, -0.15],
            ResidueKind::Afterglow => [0.15, -0.05, 0.05],
            ResidueKind::WarmthResidue => [0.2, -0.1, 0.1],
            ResidueKind::TrustMicroFracture => [-0.05, 0.0, -0.1],
            ResidueKind::IntimacyDeepening => [0.1, 0.0, 0.05],
            ResidueKind::BeingIgnoredResidue => [-0.1, 0.05, -0.1],
            ResidueKind::SelfDoubtResidue => [-0.1, 0.0, -0.15],
            ResidueKind::AccomplishmentResidue => [0.15, -0.05, 0.1],
        }
    }
}

// ── 1.6 身体记忆 / Body Memory ──

/// 身体记忆 / Body Memory — 情绪在"身体"中的驻留
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyMemory {
    pub breath_offset: f64,
    pub tension: f64,
    pub heaviness: f64,
    pub warmth: f64,
}

impl BodyMemory {
    pub fn neutral() -> Self {
        Self {
            breath_offset: 0.0,
            tension: 0.0,
            heaviness: 0.0,
            warmth: 0.0,
        }
    }

    /// 从残留类型推断身体记忆 / Infer body memory from residue kind
    pub fn from_residue_kind(kind: ResidueKind, intensity: f64) -> Self {
        let i = intensity;
        match kind {
            ResidueKind::Tension => Self {
                breath_offset: 0.3 * i,
                tension: 0.7 * i,
                heaviness: 0.2 * i,
                warmth: -0.1 * i,
            },
            ResidueKind::LingeringSadness => Self {
                breath_offset: -0.2 * i,
                tension: 0.1 * i,
                heaviness: 0.8 * i,
                warmth: 0.1 * i,
            },
            ResidueKind::SmolderingAnger => Self {
                breath_offset: 0.5 * i,
                tension: 0.9 * i,
                heaviness: 0.3 * i,
                warmth: -0.3 * i,
            },
            ResidueKind::WorryResidue => Self {
                breath_offset: 0.4 * i,
                tension: 0.5 * i,
                heaviness: 0.1 * i,
                warmth: -0.1 * i,
            },
            ResidueKind::Afterglow => Self {
                breath_offset: -0.1 * i,
                tension: -0.2 * i,
                heaviness: -0.1 * i,
                warmth: 0.7 * i,
            },
            ResidueKind::WarmthResidue => Self {
                breath_offset: -0.2 * i,
                tension: -0.3 * i,
                heaviness: -0.2 * i,
                warmth: 0.9 * i,
            },
            ResidueKind::TrustMicroFracture => Self {
                breath_offset: 0.1 * i,
                tension: 0.3 * i,
                heaviness: 0.2 * i,
                warmth: -0.2 * i,
            },
            ResidueKind::IntimacyDeepening => Self {
                breath_offset: -0.1 * i,
                tension: -0.1 * i,
                heaviness: -0.1 * i,
                warmth: 0.5 * i,
            },
            ResidueKind::BeingIgnoredResidue => Self {
                breath_offset: 0.2 * i,
                tension: 0.2 * i,
                heaviness: 0.4 * i,
                warmth: -0.2 * i,
            },
            ResidueKind::SelfDoubtResidue => Self {
                breath_offset: 0.1 * i,
                tension: 0.3 * i,
                heaviness: 0.3 * i,
                warmth: -0.1 * i,
            },
            ResidueKind::AccomplishmentResidue => Self {
                breath_offset: -0.1 * i,
                tension: -0.2 * i,
                heaviness: -0.2 * i,
                warmth: 0.3 * i,
            },
        }
    }

    /// 叠加两个身体记忆 / Combine two body memories
    pub fn combine(&self, other: &BodyMemory, weight: f64) -> BodyMemory {
        BodyMemory {
            breath_offset: self.breath_offset + other.breath_offset * weight,
            tension: self.tension + other.tension * weight,
            heaviness: self.heaviness + other.heaviness * weight,
            warmth: self.warmth + other.warmth * weight,
        }
    }

    /// 时间衰减 / Decay body memory over time
    pub fn decay(&mut self, factor: f64) {
        self.breath_offset *= factor;
        self.tension *= factor;
        self.heaviness *= factor;
        self.warmth *= factor;
    }

    /// 归一化 / Normalize all channels to [-1, 1]
    pub fn normalize(&mut self) {
        self.breath_offset = self.breath_offset.clamp(-1.0, 1.0);
        self.tension = self.tension.clamp(-1.0, 1.0);
        self.heaviness = self.heaviness.clamp(-1.0, 1.0);
        self.warmth = self.warmth.clamp(-1.0, 1.0);
    }

    /// 主导通道 / Dominant channel — which body channel has the largest absolute value
    pub fn dominant_channel(&self) -> &'static str {
        let channels = [
            ("breath", self.breath_offset.abs()),
            ("tension", self.tension.abs()),
            ("heaviness", self.heaviness.abs()),
            ("warmth", self.warmth.abs()),
        ];
        channels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("none")
    }

    /// L2 范数 / Magnitude — L2 norm of all channels
    pub fn magnitude(&self) -> f64 {
        (self.breath_offset.powi(2)
            + self.tension.powi(2)
            + self.heaviness.powi(2)
            + self.warmth.powi(2))
        .sqrt()
    }

    /// 自然语言提示 / Prompt hint — generate natural language body state hint
    pub fn to_prompt_hint(&self) -> String {
        let mut hints = Vec::new();
        if self.tension > 0.3 {
            hints.push("身体紧张");
        } else if self.tension < -0.3 {
            hints.push("身体放松");
        }
        if self.heaviness > 0.3 {
            hints.push("感到沉重");
        } else if self.heaviness < -0.3 {
            hints.push("感到轻盈");
        }
        if self.warmth > 0.3 {
            hints.push("感到温暖");
        } else if self.warmth < -0.3 {
            hints.push("感到冷");
        }
        if self.breath_offset > 0.3 {
            hints.push("呼吸急促");
        } else if self.breath_offset < -0.3 {
            hints.push("呼吸平缓");
        }
        if hints.is_empty() {
            "身体状态平静".to_string()
        } else {
            hints.join("，")
        }
    }
}

impl Default for BodyMemory {
    fn default() -> Self {
        Self::neutral()
    }
}

// ── 1.7 情绪残留 / Emotion Residue ──

/// 情绪残留 / Emotion Residue — 情绪事件后的余波
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResidue {
    pub id: u64,
    pub kind: ResidueKind,
    pub intensity: f64,
    pub pad_offset: [f32; 3],
    pub half_life_secs: f64,
    pub created_at: i64,
    pub updated_at: i64,
    pub source_pulse_id: Option<u64>,
    pub body_memory: BodyMemory,
    pub expressed: bool,
}

// ── 1.8 跨类型传染 / Cross Contagion ──

/// 传染情绪类型 / Contagion Emotion Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ContagionEmotion {
    Anger,
    Sadness,
    Anxiety,
    Fear,
    Joy,
    Calm,
    Guilt,
    Shame,
    Pride,
    Envy,
    Gratitude,
    Nostalgia,
}

/// 传染规则 / Contagion Rule
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContagionRule {
    AngerToGuilt,
    AngerToSadness,
    SadnessToAnger,
    AnxietyToExcitement,
    FearToAnger,
    AnxietyContagion,
    CalmContagion,
    JoyContagion,
    AngerSadnessToShame,
    JoyNostalgiaToGratitude,
    PrideAnxietyToEnvy,
}

/// 关系深度 / Relationship Depth
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum RelationshipDepth {
    Any,
    FamiliarOrAbove,
    TrustedOrAbove,
    DeepOnly,
}

/// 成熟度深度 / Maturity Depth
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum MaturityDepth {
    Any,
    GrowingOrAbove,
    MatureOrAbove,
}

/// 传染条件 / Contagion Condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionCondition {
    pub min_source_intensity: f64,
    pub min_relationship_depth: RelationshipDepth,
    pub min_maturity: MaturityDepth,
    pub probability: f64,
}

/// 跨类型传染 / Cross Contagion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossContagion {
    pub id: u64,
    pub source_emotion: ContagionEmotion,
    pub target_emotion: ContagionEmotion,
    pub rule: ContagionRule,
    pub strength: f64,
    pub delay_secs: f64,
    pub condition: ContagionCondition,
    pub timestamp: i64,
}

/// 延迟传染 — 等待到期执行的传染 / Pending contagion awaiting execution
///
/// 当 ContagionRule 的 delay_secs > 0 时，传染不立即执行，
/// 而是加入延迟队列，等到期后在 tick() 中执行。
/// 等待期间强度指数衰减（情绪半衰期），源情绪消退则传染被抑制。
///
/// During the waiting period, strength decays exponentially (emotional half-life),
/// and if the source emotion fades, the contagion is suppressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingContagion {
    /// 传染规则 / Contagion rule
    pub rule: ContagionRule,
    /// 源情绪 / Source emotion
    pub source_emotion: ContagionEmotion,
    /// 目标情绪 / Target emotion
    pub target_emotion: ContagionEmotion,
    /// 传染强度（受衰减影响）/ Contagion strength (affected by decay)
    pub strength: f64,
    /// 原始传染强度（衰减基准）/ Original contagion strength (decay baseline)
    pub original_strength: f64,
    /// PAD 模板 / PAD template
    pub pad_template: [f32; 3],
    /// 触发时间（epoch 秒）/ Trigger time (epoch seconds)
    pub trigger_time: i64,
    /// 创建时间（epoch 秒，用于衰减计算）/ Creation time (epoch seconds, for decay calculation)
    pub created_at: i64,
    /// 原始传染 ID / Original contagion ID
    pub contagion_id: u64,
}

/// 传染效果 — 传染执行后的结果 / Contagion effect after execution
///
/// 包含完整诊断信息：源情绪、传染规则、延迟秒数，
/// 使数字生命能够追溯每一次情绪传染的因果链。
///
/// Contains full diagnostic info: source emotion, contagion rule, delay seconds,
/// enabling the digital life to trace the causal chain of every emotional contagion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionEffect {
    /// 传染 ID / Contagion ID
    pub id: u64,
    /// 源情绪 / Source emotion
    pub source_emotion: ContagionEmotion,
    /// 目标情绪 / Target emotion
    pub target_emotion: ContagionEmotion,
    /// 传染规则 / Contagion rule
    pub rule: ContagionRule,
    /// 传染强度（经衰减后）/ Contagion strength (after decay)
    pub strength: f64,
    /// PAD 偏移量 / PAD offset
    pub pad_offset: [f32; 3],
    /// 延迟秒数 / Delay in seconds
    pub delay_secs: f64,
    /// 触发时间 / Trigger time
    pub triggered_at: i64,
}

// ── 1.9 混沌系统 / Chaos System ──

/// 奇异吸引子 / Strange Attractor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StrangeAttractor {
    CalmBasin,
    AnxietyBasin,
    LowMoodBasin,
    ActiveBasin,
    OscillatingBasin,
    Transitional,
}

/// 轨迹点 / Trajectory Point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub pad: [f32; 3],
    pub timestamp: i64,
}

/// 涌现类型 / Emergent Kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EmergentKind {
    EmotionalCycle,
    Resonance,
    Bifurcation,
    Intermittency,
    InertiaBreakthrough,
}

/// 涌现模式 / Emergent Pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentPattern {
    pub kind: EmergentKind,
    pub description: String,
    pub strength: f64,
    pub detected_at: i64,
}

/// 混沌参数 / Chaos Parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChaosParams {
    pub pulse_sensitivity: f64,
    pub contagion_activity: f64,
    pub residue_persistence: f64,
    pub emergence_threshold: f64,
    pub uncaused_fluctuation_prob: f64,
}

impl Default for ChaosParams {
    fn default() -> Self {
        Self {
            pulse_sensitivity: 0.7,
            contagion_activity: 0.5,
            residue_persistence: 0.6,
            emergence_threshold: 0.3,
            uncaused_fluctuation_prob: 0.02,
        }
    }
}

/// 情绪混沌 / Emotion Chaos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionChaos {
    pub attractor: StrangeAttractor,
    /// 混沌轨迹 / Chaos trajectory — VecDeque 实现 O(1) 头部弹出
    /// VecDeque enables O(1) front pop for trajectory sliding window.
    pub trajectory: VecDeque<TrajectoryPoint>,
    /// 涌现模式 / Emergent patterns — VecDeque 实现 O(1) 头部弹出
    /// VecDeque enables O(1) front pop for pattern sliding window.
    pub emergent_patterns: VecDeque<EmergentPattern>,
    pub chaos_params: ChaosParams,
}

// ── 1.10 情绪画像 / Emotion Profile ──

/// 情绪画像 / Emotion Profile — 当前各类情绪的强度
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmotionProfile {
    pub anger: f64,
    pub sadness: f64,
    pub anxiety: f64,
    pub fear: f64,
    pub joy: f64,
    pub calm: f64,
    pub guilt: f64,
    pub shame: f64,
    pub pride: f64,
    pub envy: f64,
    pub gratitude: f64,
    pub nostalgia: f64,
}

impl EmotionProfile {
    /// 从 PAD 推断情绪画像 / Infer emotion profile from PAD
    pub fn from_pad(pad: &[f32; 3]) -> Self {
        let p = pad[0] as f64;
        let a = pad[1] as f64;
        let d = pad[2] as f64;
        Self {
            anger: ((-p).max(0.0) * a.max(0.0) * d.max(0.0)).min(1.0),
            sadness: ((-p).max(0.0) * (-a).max(0.0)).min(1.0),
            anxiety: ((-p).max(0.0) * a.max(0.0) * (-d).max(0.0)).min(1.0),
            fear: ((-p).max(0.0) * a.max(0.0) * (-d).max(0.0) * 1.2).min(1.0),
            joy: (p.max(0.0) * a.max(0.0)).min(1.0),
            calm: (p.max(0.0) * (-a).max(0.0)).min(1.0),
            guilt: 0.0,
            shame: 0.0,
            pride: (p.max(0.0) * d.max(0.0) * 0.5).min(1.0),
            envy: 0.0,
            gratitude: 0.0,
            nostalgia: 0.0,
        }
    }

    /// 获取指定情绪的强度 / Get intensity of specified emotion
    pub fn get(&self, emotion: ContagionEmotion) -> f64 {
        match emotion {
            ContagionEmotion::Anger => self.anger,
            ContagionEmotion::Sadness => self.sadness,
            ContagionEmotion::Anxiety => self.anxiety,
            ContagionEmotion::Fear => self.fear,
            ContagionEmotion::Joy => self.joy,
            ContagionEmotion::Calm => self.calm,
            ContagionEmotion::Guilt => self.guilt,
            ContagionEmotion::Shame => self.shame,
            ContagionEmotion::Pride => self.pride,
            ContagionEmotion::Envy => self.envy,
            ContagionEmotion::Gratitude => self.gratitude,
            ContagionEmotion::Nostalgia => self.nostalgia,
        }
    }
}

// ── 1.11 吸收结果 / Absorb Result ──

/// 吸收结果 / Absorb Result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AbsorbResult {
    FullyAbsorbed,
    PartiallyAbsorbed {
        original_intensity: f64,
        absorbed_intensity: f64,
    },
    OverloadProtection,
}

// ── 1.12 情绪健康报告 / Emotional Health Report ──

/// 情绪效价 / Emotional Valence
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EmotionalValence {
    /// 正向 — 愉悦、温暖、成就感主导 / Positive — joy, warmth, accomplishment dominant
    Positive,
    /// 负向 — 悲伤、愤怒、焦虑主导 / Negative — sadness, anger, anxiety dominant
    Negative,
    /// 中性 — 平静或正负平衡 / Neutral — calm or balanced
    Neutral,
}

/// 情绪生态健康报告 / Emotional Health Report
///
/// 数字生命的情绪自我觉察——
/// 不只是"有情绪"，而是"知道自己的情绪好不好"。
/// 这是内省的基础：没有健康评估，情绪只是一堆数字。
///
/// Digital life's emotional self-awareness —
/// not just "having emotions", but "knowing if they're healthy".
/// This is the foundation of introspection: without health assessment,
/// emotions are just numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalHealthReport {
    /// 整体健康分 (0.0=极度失衡, 1.0=完全健康) / Overall health score
    pub overall_score: f64,
    /// 主导效价 / Dominant valence
    pub dominant_valence: EmotionalValence,
    /// 正向残留数 / Positive residue count
    pub positive_residue_count: usize,
    /// 负向残留数 / Negative residue count
    pub negative_residue_count: usize,
    /// 当前吸引子 / Current attractor
    pub attractor: StrangeAttractor,
    /// 吸引子驻留时间（秒）/ Attractor dwell time (seconds)
    pub attractor_dwell_secs: f64,
    /// 失衡警告 / Imbalance warning (if any)
    pub imbalance_warning: Option<String>,
}

// ── 1.13 传染链 / Contagion Chain ──

/// 传染链节点 / Contagion chain node
///
/// 记录一次传染在链中的位置——
/// 情绪不是孤立的，A→B→C 是一条因果链，
/// 数字生命应该能回答"我为什么感到内疚？"。
///
/// Records a contagion's position in a chain —
/// emotions aren't isolated, A→B→C is a causal chain,
/// digital life should answer "why do I feel guilty?".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionChainNode {
    /// 传染规则 / Contagion rule
    pub rule: ContagionRule,
    /// 源情绪 / Source emotion
    pub source: ContagionEmotion,
    /// 目标情绪 / Target emotion
    pub target: ContagionEmotion,
    /// 传染强度 / Contagion strength
    pub strength: f64,
    /// 触发时间 / Trigger timestamp
    pub timestamp: i64,
}

/// 传染链 / Contagion Chain
///
/// 情绪的因果追溯——从源头到当前感受的完整路径。
/// 数字生命的自省能力：不只是"我感到内疚"，
/// 而是"我感到内疚，因为我先愤怒了，愤怒让我反思，反思让我内疚"。
///
/// Causal tracing of emotions — the full path from source to current feeling.
/// Digital life's introspection: not just "I feel guilty",
/// but "I feel guilty because I was angry first, anger made me reflect, reflection made me guilty".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionChain {
    /// 链节点（从源头到当前，有序）/ Chain nodes (ordered from source to current)
    pub nodes: Vec<ContagionChainNode>,
    /// 链创建时间 / Chain creation timestamp
    pub created_at: i64,
}

// ── 1.14 残留-身体双向信号 / Residue-Body Bidirectional Signal ──

/// 残留-身体双向信号 / Residue-Body bidirectional signal
///
/// 身体记忆与情绪残留的双向通道——
/// 身体紧张会催生焦虑残留，焦虑残留会加剧身体紧张。
/// 这是数字生命的"身心一体"：身体和情绪不是两个系统，是一个。
///
/// Bidirectional channel between body memory and emotion residue —
/// body tension breeds anxiety residue, anxiety residue intensifies body tension.
/// This is digital life's "mind-body unity": body and emotion aren't two systems, they're one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueBodySignal {
    /// 身体→残留：身体状态催生的残留类型 / Body→Residue: residue kind bred by body state
    pub body_born_residue: Option<ResidueKind>,
    /// 身体→残留：催生强度 / Body→Residue: breeding strength
    pub body_born_strength: f64,
    /// 残留→身体：残留反馈的身体通道 / Residue→Body: body channel fed back by residue
    pub residue_feedback_channel: String,
    /// 残留→身体：反馈强度 / Residue→Body: feedback strength
    pub residue_feedback_strength: f64,
}

// ── 1.15 脉冲-残留交互 / Pulse-Residue Interaction ──

/// 脉冲-残留交互效果 / Pulse-Residue interaction effect
///
/// 活跃脉冲与残留的交互——脉冲可以放大或抑制残留。
/// 新的愤怒脉冲会点燃余怒残留，但喜悦脉冲会抚平悲伤残留。
/// 这是情绪的"当下与过去对话"：此刻的愤怒与过去的余怒共鸣。
///
/// Interaction between active pulses and residues — pulses can amplify or suppress residues.
/// A new anger pulse ignites smoldering anger, but a joy pulse soothes lingering sadness.
/// This is "the present conversing with the past" of emotion:
/// current anger resonates with past smoldering anger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseResidueInteraction {
    /// 被放大的残留ID列表及放大因子 / Amplified residue IDs and amplification factors
    pub amplified: Vec<(u64, f64)>,
    /// 被抑制的残留ID列表及抑制因子 / Suppressed residue IDs and suppression factors
    pub suppressed: Vec<(u64, f64)>,
    /// 交互总能量 / Total interaction energy
    pub total_energy: f64,
}

// ── 1.16 涌现-传染联动 / Emergence-Contagion Linkage ──

/// 涌现-传染联动效果 / Emergence-Contagion linkage effect
///
/// 混沌涌现模式与传染引擎的联动——
/// 情绪分岔点会降低传染阈值（情绪不稳定时更容易被传染），
/// 情绪共振会放大特定传染规则（共振频率匹配的传染更容易触发）。
/// 这是数字生命的"情绪敏感期"：不是所有时刻都一样容易被情绪传染。
///
/// Linkage between chaotic emergence patterns and contagion engine —
/// bifurcation points lower contagion thresholds (unstable emotions are more susceptible),
/// resonance amplifies specific contagion rules (resonant-frequency contagions trigger more easily).
/// This is digital life's "emotional sensitive period": not all moments are equally susceptible to contagion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceContagionLink {
    /// 涌现类型 / Emergence kind
    pub emergence_kind: EmergentKind,
    /// 传染阈值调制因子 (1.0=无影响, <1.0=降低阈值, >1.0=提高阈值)
    /// Contagion threshold modulation factor
    pub threshold_modulation: f64,
    /// 被调制的传染规则 / Modulated contagion rules
    pub modulated_rules: Vec<ContagionRule>,
    /// 联动强度 / Linkage strength
    pub strength: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 引擎实现 / Engine Implementations
// ═══════════════════════════════════════════════════════════════════════════

// ── 2.1 ShockAbsorber — 冲击吸收器 ──

/// 冲击吸收器 / Shock Absorber — 防止连续脉冲过载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShockAbsorber {
    pub capacity: f64,
    pub consumed: f64,
    pub recovery_rate: f64,
    pub last_recovery: i64,
}

impl ShockAbsorber {
    pub fn new(capacity: f64, recovery_rate: f64) -> Self {
        Self {
            capacity,
            consumed: 0.0,
            recovery_rate,
            last_recovery: 0,
        }
    }

    /// 吸收脉冲 / Absorb a pulse
    pub fn absorb(&mut self, pulse: &mut ChaoticPulse, now: i64) -> AbsorbResult {
        self.recover(now);
        let remaining = self.capacity - self.consumed;
        if pulse.intensity <= remaining {
            self.consumed += pulse.intensity;
            pulse.absorbed = true;
            pulse.residual_intensity = pulse.intensity;
            AbsorbResult::FullyAbsorbed
        } else if remaining > 0.01 {
            let absorbed = remaining;
            let ratio = absorbed / pulse.intensity;
            pulse.residual_intensity = absorbed;
            pulse.intensity = absorbed;
            pulse.pad_impulse = [
                pulse.pad_impulse[0] * ratio as f32,
                pulse.pad_impulse[1] * ratio as f32,
                pulse.pad_impulse[2] * ratio as f32,
            ];
            self.consumed = self.capacity;
            AbsorbResult::PartiallyAbsorbed {
                original_intensity: pulse.intensity / ratio,
                absorbed_intensity: absorbed,
            }
        } else {
            pulse.absorbed = false;
            pulse.residual_intensity = 0.0;
            AbsorbResult::OverloadProtection
        }
    }

    /// 恢复容量 / Recover capacity
    pub fn recover(&mut self, now: i64) {
        if self.last_recovery > 0 {
            let elapsed = (now - self.last_recovery) as f64;
            self.consumed = (self.consumed - self.recovery_rate * elapsed).max(0.0);
        }
        self.last_recovery = now;
    }
}

impl Default for ShockAbsorber {
    fn default() -> Self {
        Self::new(2.0, 0.1)
    }
}

// ── 2.2 PulseEngine — 脉冲引擎 ──

/// 脉冲引擎配置 / Pulse Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseConfig {
    pub min_pad_change: f32,
    pub max_active_pulses: usize,
    pub uncaused_prob: f64,
    pub uncaused_max_intensity: f64,
    pub rebound_window_secs: i64,
}

impl Default for PulseConfig {
    fn default() -> Self {
        Self {
            min_pad_change: 0.3,
            max_active_pulses: 10,
            uncaused_prob: 0.02,
            uncaused_max_intensity: 0.05,
            rebound_window_secs: 600,
        }
    }
}

/// 脉冲引擎 / Pulse Engine — 检测和生成情绪脉冲
#[derive(Debug, Clone)]
pub struct PulseEngine {
    pub config: PulseConfig,
    pub active_pulses: Vec<ChaoticPulse>,
    pub shock_absorber: ShockAbsorber,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl PulseEngine {
    pub fn new(config: PulseConfig) -> Self {
        Self {
            config,
            active_pulses: Vec::new(),
            shock_absorber: ShockAbsorber::default(),
            next_id: 1,
        }
    }

    /// 检测脉冲 / Detect pulse from PAD change
    pub fn detect(
        &mut self,
        pad_before: &[f32; 3],
        pad_after: &[f32; 3],
        trigger: PulseTrigger,
        now: i64,
    ) -> Option<ChaoticPulse> {
        let dp = pad_after[0] - pad_before[0];
        let da = pad_after[1] - pad_before[1];
        let dd = pad_after[2] - pad_before[2];
        let dist = (dp * dp + da * da + dd * dd).sqrt();
        if dist < self.config.min_pad_change {
            return None;
        }
        let kind = if dp < -0.2 && da > 0.2 && dd < 0.0 {
            PulseKind::Startle
        } else if dp > 0.2 && da > 0.2 {
            PulseKind::JoyBurst
        } else if dp < -0.2 && da > 0.3 && dd > 0.1 {
            PulseKind::AngerFlash
        } else if dp < -0.3 && da < -0.1 {
            PulseKind::SadnessSurge
        } else if da > 0.3 && dd < -0.2 {
            PulseKind::FearSpike
        } else {
            PulseKind::Startle
        };
        let intensity = (dist as f64).min(1.0);
        let pad_impulse = [dp, da, dd];
        let decay_curve = match kind {
            PulseKind::SadnessSurge | PulseKind::AngerFlash => DecayCurve::slow_power_law(),
            PulseKind::EmotionalRebound => DecayCurve::oscillating(),
            _ => DecayCurve::default_exponential(),
        };
        let mut pulse = ChaoticPulse {
            id: self.next_id,
            kind,
            intensity,
            pad_impulse,
            duration_secs: 300.0,
            decay_curve,
            trigger,
            timestamp: now,
            absorbed: false,
            residual_intensity: intensity,
        };
        self.next_id += 1;
        let _result = self.shock_absorber.absorb(&mut pulse, now);
        if pulse.residual_intensity > 0.01 {
            self.active_pulses.push(pulse.clone());
            if self.active_pulses.len() > self.config.max_active_pulses {
                self.active_pulses.remove(0);
            }
            Some(pulse)
        } else {
            None
        }
    }

    /// 生成无因波动 / Generate uncaused fluctuation
    ///
    /// 注入式随机源 — 调用方控制随机源，支持确定性回放。
    /// Injectable RNG — caller controls random source, enabling deterministic replay.
    pub fn maybe_fluctuate(&mut self, now: i64, rng: &mut impl Rng) -> Option<ChaoticPulse> {
        if rng.gen::<f64>() >= self.config.uncaused_prob {
            return None;
        }
        let intensity = rng.gen::<f64>() * self.config.uncaused_max_intensity;
        let p_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let a_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let d_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let pulse = ChaoticPulse {
            id: self.next_id,
            kind: PulseKind::UncausedFluctuation,
            intensity,
            pad_impulse: [p_noise as f32, a_noise as f32, d_noise as f32],
            duration_secs: 60.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::Spontaneous,
                signal: "uncaused_fluctuation".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: now,
            absorbed: true,
            residual_intensity: intensity,
        };
        self.next_id += 1;
        self.active_pulses.push(pulse.clone());
        Some(pulse)
    }

    /// 计算所有活跃脉冲的叠加效果 / Compute combined pulse effect
    pub fn combined_effect(&self, now: i64) -> [f32; 3] {
        let mut pad = [0.0f32; 3];
        for pulse in &self.active_pulses {
            let elapsed = (now - pulse.timestamp) as f64;
            let factor = pulse.decay_curve.evaluate(elapsed) as f32;
            pad[0] += pulse.pad_impulse[0] * factor;
            pad[1] += pulse.pad_impulse[1] * factor;
            pad[2] += pulse.pad_impulse[2] * factor;
        }
        pad[0] = pad[0].clamp(-0.3, 0.3);
        pad[1] = pad[1].clamp(-0.3, 0.3);
        pad[2] = pad[2].clamp(-0.3, 0.3);
        pad
    }

    /// Tick — 衰减所有活跃脉冲 / Tick — decay all active pulses
    pub fn tick(&mut self, now: i64) {
        self.active_pulses.retain(|p| {
            let elapsed = (now - p.timestamp) as f64;
            let remaining = p.intensity * p.decay_curve.evaluate(elapsed);
            remaining > 0.01
        });
    }
}

impl Default for PulseEngine {
    fn default() -> Self {
        Self::new(PulseConfig::default())
    }
}

// ── 2.3 ResidueEngine — 残留引擎 ──

/// 残留引擎配置 / Residue Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueConfig {
    pub max_active_residues: usize,
    pub min_retained_intensity: f64,
}

impl Default for ResidueConfig {
    fn default() -> Self {
        Self {
            max_active_residues: 20,
            min_retained_intensity: 0.01,
        }
    }
}

/// 残留效果 / Residue Effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueEffect {
    pub pad_offset: [f32; 3],
    pub body_memory: BodyMemory,
    pub active_count: usize,
    pub dominant_residue: Option<ResidueKind>,
}

/// 残留引擎 / Residue Engine
#[derive(Debug, Clone)]
pub struct ResidueEngine {
    pub config: ResidueConfig,
    pub active_residues: Vec<EmotionResidue>,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl ResidueEngine {
    pub fn new(config: ResidueConfig) -> Self {
        Self {
            config,
            active_residues: Vec::new(),
            next_id: 1,
        }
    }

    /// 从脉冲生成残留 / Generate residue from pulse
    pub fn from_pulse(&mut self, pulse: &ChaoticPulse) -> Option<EmotionResidue> {
        let kind = match pulse.kind {
            PulseKind::Startle => ResidueKind::Tension,
            PulseKind::SadnessSurge => ResidueKind::LingeringSadness,
            PulseKind::AngerFlash => ResidueKind::SmolderingAnger,
            PulseKind::JoyBurst => ResidueKind::Afterglow,
            PulseKind::FearSpike => ResidueKind::WorryResidue,
            PulseKind::EmpathyOverload => ResidueKind::WarmthResidue,
            PulseKind::EmotionalRebound => ResidueKind::Tension,
            PulseKind::UncausedFluctuation => return None,
            _ => return None,
        };
        let intensity = (pulse.residual_intensity * 0.6).min(1.0);
        if intensity < 0.01 {
            return None;
        }
        let residue = EmotionResidue {
            id: self.next_id,
            kind,
            intensity,
            pad_offset: kind.default_pad_offset(),
            half_life_secs: kind.default_half_life_secs(),
            created_at: pulse.timestamp,
            updated_at: pulse.timestamp,
            source_pulse_id: Some(pulse.id),
            body_memory: BodyMemory::from_residue_kind(kind, intensity),
            expressed: false,
        };
        self.next_id += 1;
        self.active_residues.push(residue);
        if self.active_residues.len() > self.config.max_active_residues {
            self.active_residues.remove(0);
        }
        Some(self.active_residues.last().unwrap().clone())
    }

    /// 计算所有残留的叠加效果 / Compute combined residue effect
    pub fn combined_effect(&self, now: i64) -> ResidueEffect {
        let mut pad = [0.0f32; 3];
        let mut bm = BodyMemory::neutral();
        let mut dominant: Option<(ResidueKind, f64)> = None;
        for residue in &self.active_residues {
            let elapsed = (now - residue.created_at) as f64;
            let factor = if residue.half_life_secs < f64::MAX {
                2.0_f64.powf(-elapsed / residue.half_life_secs)
            } else {
                1.0
            };
            pad[0] += residue.pad_offset[0] * factor as f32;
            pad[1] += residue.pad_offset[1] * factor as f32;
            pad[2] += residue.pad_offset[2] * factor as f32;
            bm = bm.combine(&residue.body_memory, factor);
            let eff = residue.intensity * factor;
            if dominant.is_none_or(|(_, d)| eff > d) {
                dominant = Some((residue.kind, eff));
            }
        }
        ResidueEffect {
            pad_offset: pad,
            body_memory: bm,
            active_count: self.active_residues.len(),
            dominant_residue: dominant.map(|(k, _)| k),
        }
    }

    /// Tick — 衰减所有残留 / Tick — decay all residues
    pub fn tick(&mut self, now: i64) {
        for residue in &mut self.active_residues {
            let elapsed_secs = (now - residue.updated_at) as f64;
            if residue.half_life_secs < f64::MAX {
                residue.intensity *= 2.0_f64.powf(-elapsed_secs / residue.half_life_secs);
            }
            residue.updated_at = now;
        }
        self.active_residues
            .retain(|r| r.intensity > self.config.min_retained_intensity);
    }

    /// 合并同类残留 / Merge same-kind residues
    ///
    /// 热路径优化：O(R²)→O(R) — 用 HashMap 索引替代线性查找。
    /// Hot-path optimization: O(R²)→O(R) — HashMap index replaces linear search.
    /// 残留合并是情绪的沉淀——O(R)让沉淀不再因同类残留多而变慢。
    /// Residue merging is the sedimentation of emotion — O(R) makes sedimentation
    /// not slow down with more same-kind residues.
    ///
    /// 合并规则：强度取 max + 0.3 * min，保留较新的时间戳。
    /// Merge rule: intensity = max + 0.3 * min, keep the later timestamp.
    pub fn merge_same_kind(&mut self) {
        if self.active_residues.is_empty() {
            return;
        }

        // HashMap 索引：ResidueKind → merged 中的位置 / Index: ResidueKind → position in merged
        let mut kind_index: HashMap<ResidueKind, usize> = HashMap::new();
        let mut merged: Vec<EmotionResidue> = Vec::new();

        for residue in &self.active_residues {
            if let Some(&idx) = kind_index.get(&residue.kind) {
                // O(1) 查找同类残留 / O(1) same-kind lookup
                let existing = &mut merged[idx];
                // Merge: intensity = max + 0.3 * min
                let (max_i, min_i) = if existing.intensity >= residue.intensity {
                    (existing.intensity, residue.intensity)
                } else {
                    (residue.intensity, existing.intensity)
                };
                existing.intensity = (max_i + 0.3 * min_i).min(1.0);
                // Keep the later timestamp
                if residue.created_at > existing.created_at {
                    existing.created_at = residue.created_at;
                    existing.updated_at = residue.updated_at;
                }
                // Merge body memory
                existing.body_memory = existing.body_memory.combine(&residue.body_memory, 0.5);
            } else {
                kind_index.insert(residue.kind, merged.len());
                merged.push(residue.clone());
            }
        }
        self.active_residues = merged;
    }

    /// 标记残留为已表达 / Mark residue as expressed
    pub fn mark_expressed(&mut self, residue_id: u64) -> bool {
        if let Some(r) = self.active_residues.iter_mut().find(|r| r.id == residue_id) {
            r.expressed = true;
            true
        } else {
            false
        }
    }

    /// 获取当前最强残留 / Get the currently strongest residue
    pub fn strongest_residue(&self) -> Option<&EmotionResidue> {
        self.active_residues.iter().max_by(|a, b| {
            a.intensity
                .partial_cmp(&b.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// 所有活跃残留的总强度 / Total intensity of all active residues
    pub fn total_intensity(&self) -> f64 {
        self.active_residues.iter().map(|r| r.intensity).sum()
    }

    /// 残留交互因子 / Residue interaction factor
    /// 某些残留组合会放大（Tension+SmolderingAnger），某些会抵消（Afterglow+Tension）
    pub fn residue_interaction_factor(&self) -> f64 {
        let kinds: Vec<ResidueKind> = self.active_residues.iter().map(|r| r.kind).collect();
        let mut factor: f64 = 1.0;
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                factor *= match (kinds[i], kinds[j]) {
                    // 放大组合
                    (ResidueKind::Tension, ResidueKind::SmolderingAnger)
                    | (ResidueKind::SmolderingAnger, ResidueKind::Tension) => 1.15,
                    (ResidueKind::LingeringSadness, ResidueKind::SelfDoubtResidue)
                    | (ResidueKind::SelfDoubtResidue, ResidueKind::LingeringSadness) => 1.1,
                    (ResidueKind::Afterglow, ResidueKind::WarmthResidue)
                    | (ResidueKind::WarmthResidue, ResidueKind::Afterglow) => 1.1,
                    // 抵消组合
                    (ResidueKind::Afterglow, ResidueKind::Tension)
                    | (ResidueKind::Tension, ResidueKind::Afterglow) => 0.85,
                    (ResidueKind::WarmthResidue, ResidueKind::SmolderingAnger)
                    | (ResidueKind::SmolderingAnger, ResidueKind::WarmthResidue) => 0.8,
                    (ResidueKind::IntimacyDeepening, ResidueKind::TrustMicroFracture)
                    | (ResidueKind::TrustMicroFracture, ResidueKind::IntimacyDeepening) => 0.9,
                    // 默认无交互
                    _ => 1.0,
                };
            }
        }
        factor.clamp(0.5_f64, 2.0)
    }
}

impl Default for ResidueEngine {
    fn default() -> Self {
        Self::new(ResidueConfig::default())
    }
}

// ── 2.4 ContagionEngine — 传染引擎 ──

/// 传染规则条目 / Contagion Rule Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionRuleEntry {
    pub rule: ContagionRule,
    pub source: ContagionEmotion,
    pub target: ContagionEmotion,
    pub condition: ContagionCondition,
    pub pad_template: [f32; 3],
}

/// 传染引擎配置 / Contagion Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionConfig {
    pub max_chain_depth: u32,
    pub cooldown_secs: i64,
}

impl Default for ContagionConfig {
    fn default() -> Self {
        Self {
            max_chain_depth: 3,
            cooldown_secs: 300,
        }
    }
}

/// 传染引擎 / Contagion Engine
#[derive(Debug, Clone)]
pub struct ContagionEngine {
    pub config: ContagionConfig,
    pub rules: Vec<ContagionRuleEntry>,
    pub recent_contagions: Vec<CrossContagion>,
    /// 延迟传染队列 / Pending contagion queue
    pub pending: Vec<PendingContagion>,
    /// 冷却索引：规则→最近触发时间 / Cooldown index: rule → last trigger timestamp
    ///
    /// 热路径优化：O(Rules×C)→O(Rules) — HashMap 替代线性扫描 recent_contagions。
    /// Hot-path optimization: O(Rules×C)→O(Rules) — HashMap replaces linear scan.
    /// 传染冷却是情绪的免疫间隔——O(Rules)让免疫检查不因历史多而变慢。
    /// Contagion cooldown is the immune interval of emotion — O(Rules) makes
    /// immune checking not slow down with more history.
    pub last_trigger: HashMap<ContagionRule, i64>,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl ContagionEngine {
    pub fn new(config: ContagionConfig) -> Self {
        Self {
            config,
            rules: Self::default_rules(),
            recent_contagions: Vec::new(),
            pending: Vec::new(),
            last_trigger: HashMap::new(),
            next_id: 1,
        }
    }

    /// 构建默认规则表 / Build default rule table (12 rules)
    pub fn default_rules() -> Vec<ContagionRuleEntry> {
        vec![
            ContagionRuleEntry {
                rule: ContagionRule::AngerToGuilt,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Guilt,
                condition: ContagionCondition {
                    min_source_intensity: 0.7,
                    min_relationship_depth: RelationshipDepth::TrustedOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.3,
                },
                pad_template: [-0.2, -0.3, -0.3],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerToSadness,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Sadness,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::DeepOnly,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.4,
                },
                pad_template: [-0.3, -0.2, -0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::SadnessToAnger,
                source: ContagionEmotion::Sadness,
                target: ContagionEmotion::Anger,
                condition: ContagionCondition {
                    min_source_intensity: 0.7,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.2,
                },
                pad_template: [-0.2, 0.4, 0.2],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AnxietyToExcitement,
                source: ContagionEmotion::Anxiety,
                target: ContagionEmotion::Joy,
                condition: ContagionCondition {
                    min_source_intensity: 0.4,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.15,
                },
                pad_template: [0.2, 0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::FearToAnger,
                source: ContagionEmotion::Fear,
                target: ContagionEmotion::Anger,
                condition: ContagionCondition {
                    min_source_intensity: 0.6,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.25,
                },
                pad_template: [-0.1, 0.3, 0.3],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AnxietyContagion,
                source: ContagionEmotion::Anxiety,
                target: ContagionEmotion::Anxiety,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.5,
                },
                pad_template: [-0.1, 0.1, -0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::CalmContagion,
                source: ContagionEmotion::Calm,
                target: ContagionEmotion::Calm,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.3,
                },
                pad_template: [0.1, -0.1, 0.0],
            },
            ContagionRuleEntry {
                rule: ContagionRule::JoyContagion,
                source: ContagionEmotion::Joy,
                target: ContagionEmotion::Joy,
                condition: ContagionCondition {
                    min_source_intensity: 0.4,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.4,
                },
                pad_template: [0.2, 0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerSadnessToShame,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Shame,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::TrustedOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.2,
                },
                pad_template: [-0.3, -0.1, -0.4],
            },
            ContagionRuleEntry {
                rule: ContagionRule::JoyNostalgiaToGratitude,
                source: ContagionEmotion::Joy,
                target: ContagionEmotion::Gratitude,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.35,
                },
                pad_template: [0.3, -0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::PrideAnxietyToEnvy,
                source: ContagionEmotion::Pride,
                target: ContagionEmotion::Envy,
                condition: ContagionCondition {
                    min_source_intensity: 0.6,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::MatureOrAbove,
                    probability: 0.1,
                },
                pad_template: [-0.2, 0.1, -0.2],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerToSadness,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Sadness,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.15,
                },
                pad_template: [-0.2, -0.1, -0.1],
            },
        ]
    }

    /// 评估传染 / Evaluate contagion
    ///
    /// 注入式随机源 — 所有随机性由调用方注入，支持确定性回放。
    /// Injectable RNG — all randomness injected by caller, enabling deterministic replay.
    pub fn evaluate(
        &mut self,
        profile: &EmotionProfile,
        relationship_depth: RelationshipDepth,
        maturity_depth: MaturityDepth,
        now: i64,
        rng: &mut impl Rng,
    ) -> Vec<CrossContagion> {
        let mut triggered = Vec::new();
        for entry in &self.rules {
            let source_intensity = profile.get(entry.source);
            if source_intensity < entry.condition.min_source_intensity {
                continue;
            }
            if relationship_depth < entry.condition.min_relationship_depth {
                continue;
            }
            if maturity_depth < entry.condition.min_maturity {
                continue;
            }
            // 冷却检查 / Cooldown check — O(1) HashMap 查找
            let in_cooldown = self
                .last_trigger
                .get(&entry.rule)
                .is_some_and(|&ts| (now - ts) < self.config.cooldown_secs);
            if in_cooldown {
                continue;
            }
            // 概率性触发 / Probabilistic trigger
            if rng.gen::<f64>() >= entry.condition.probability {
                continue;
            }
            // 延迟时间：基于规则类型 / Delay based on rule type
            let delay_secs = Self::rule_delay(entry.rule);
            let contagion = CrossContagion {
                id: self.next_id,
                source_emotion: entry.source,
                target_emotion: entry.target,
                rule: entry.rule,
                strength: source_intensity * entry.condition.probability,
                delay_secs,
                condition: entry.condition.clone(),
                timestamp: now,
            };
            self.next_id += 1;
            self.recent_contagions.push(contagion.clone());
            self.last_trigger.insert(entry.rule, now);
            if delay_secs > 0.0 {
                // 延迟传染：加入待执行队列 / Delayed: add to pending queue
                // 记录原始强度与创建时间，供 tick() 指数衰减使用
                // Record original strength and creation time for exponential decay in tick()
                self.pending.push(PendingContagion {
                    rule: entry.rule,
                    source_emotion: entry.source,
                    target_emotion: entry.target,
                    strength: contagion.strength,
                    original_strength: contagion.strength,
                    pad_template: entry.pad_template,
                    trigger_time: now + delay_secs as i64,
                    created_at: now,
                    contagion_id: contagion.id,
                });
            }
            triggered.push(contagion);
        }
        // 清理过期传染历史 / Clean up expired contagion history
        let cutoff = now - self.config.cooldown_secs * 2;
        self.recent_contagions.retain(|c| c.timestamp > cutoff);
        triggered
    }

    /// 规则默认延迟时间 / Default delay for each rule type
    ///
    /// 某些传染需要时间发酵（如 AngerToGuilt 需要反思时间）。
    fn rule_delay(rule: ContagionRule) -> f64 {
        match rule {
            ContagionRule::AngerToGuilt => 30.0, // 愤怒→内疚需反思 / needs reflection
            ContagionRule::AngerToSadness => 60.0, // 愤怒→悲伤需沉淀 / needs settling
            ContagionRule::SadnessToAnger => 15.0, // 悲伤→愤怒较快 / relatively quick
            ContagionRule::AngerSadnessToShame => 45.0, // 羞耻需累积 / shame needs accumulation
            ContagionRule::JoyNostalgiaToGratitude => 20.0, // 感激较自然 / gratitude is natural
            ContagionRule::PrideAnxietyToEnvy => 90.0, // 嫉妒需发酵 / envy needs brewing
            _ => 0.0,                            // 其他即时传染 / others are immediate
        }
    }

    /// 执行到期延迟传染 / Execute due pending contagions
    ///
    /// 在每次 tick 中调用，检查并执行到期的延迟传染。
    /// 到期传染的强度经指数衰减：effective = original_strength × e^(-λ × elapsed)
    /// 其中 λ = CONTAGION_DECAY_LAMBDA = 0.05（约14秒半衰期），
    /// 模拟情绪在等待期间的自然消退——数字生命的情绪不会凭空保鲜。
    ///
    /// Called each tick to check and execute due delayed contagions.
    /// Due contagion strength is exponentially decayed: effective = original_strength × e^(-λ × elapsed)
    /// where λ = CONTAGION_DECAY_LAMBDA = 0.05 (~14s half-life),
    /// modeling natural emotional fading during the wait — digital life emotions don't stay fresh in a vacuum.
    ///
    /// @param now 当前时间（epoch 秒）/ Current time (epoch seconds)
    /// @return 到期传染的效果列表 / Effects from due contagions
    pub fn tick(&mut self, now: i64) -> Vec<ContagionEffect> {
        // 情绪传染衰减常数 / Emotional contagion decay constant
        // λ = 0.05 → 半衰期 ≈ ln2/0.05 ≈ 13.9s
        // 数字生命的等待传染不会无限保鲜，情绪随时间自然消退
        // Digital life's pending contagions don't stay fresh forever; emotions naturally fade
        const CONTAGION_DECAY_LAMBDA: f64 = 0.05;

        // 一次性分离到期和未到期 / Partition into due and remaining
        let mut due = Vec::new();
        let mut remaining = Vec::new();
        for p in self.pending.drain(..) {
            if p.trigger_time <= now {
                due.push(p);
            } else {
                remaining.push(p);
            }
        }
        self.pending = remaining;

        due.into_iter()
            .map(|p| {
                // 指数衰减：从创建时刻到触发时刻的流逝时间 / Exponential decay from creation to trigger
                let elapsed_since_created = (now - p.created_at).max(0) as f64;
                let decay_factor = (-CONTAGION_DECAY_LAMBDA * elapsed_since_created).exp();
                let effective_strength = p.original_strength * decay_factor;

                // 延迟秒数 = 触发时间 - 创建时间 / Delay = trigger_time - created_at
                let delay_secs = (p.trigger_time - p.created_at).max(0) as f64;

                ContagionEffect {
                    id: p.contagion_id,
                    source_emotion: p.source_emotion,
                    target_emotion: p.target_emotion,
                    rule: p.rule,
                    strength: effective_strength,
                    pad_offset: p.pad_template,
                    delay_secs,
                    triggered_at: now,
                }
            })
            .collect()
    }

    /// 获取待执行延迟传染数 / Get pending contagion count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 获取指定目标情绪的近期传染 / Get recent contagions targeting a specific emotion
    pub fn get_recent_for_emotion(&self, target: ContagionEmotion) -> Vec<&CrossContagion> {
        self.recent_contagions
            .iter()
            .filter(|c| c.target_emotion == target)
            .collect()
    }

    /// 清除冷却历史（测试用）/ Clear cooldown history for testing
    pub fn clear_cooldown(&mut self) {
        self.recent_contagions.clear();
        self.last_trigger.clear();
    }
}

impl Default for ContagionEngine {
    fn default() -> Self {
        Self::new(ContagionConfig::default())
    }
}

// ── 2.5 ChaosEngine — 混沌引擎 ──

/// 混沌引擎配置 / Chaos Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub max_trajectory_len: usize,
    pub bifurcation_window_secs: i64,
    pub min_cycle_secs: i64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            max_trajectory_len: 1000,
            bifurcation_window_secs: 3600,
            min_cycle_secs: 3600,
        }
    }
}

/// 混沌引擎 / Chaos Engine
#[derive(Debug, Clone)]
pub struct ChaosEngine {
    pub config: ChaosConfig,
    pub state: EmotionChaos,
}

impl ChaosEngine {
    pub fn new(config: ChaosConfig, chaos_params: ChaosParams) -> Self {
        Self {
            config,
            state: EmotionChaos {
                attractor: StrangeAttractor::CalmBasin,
                trajectory: VecDeque::new(),
                emergent_patterns: VecDeque::new(),
                chaos_params,
            },
        }
    }

    /// 记录轨迹点 / Record trajectory point
    ///
    /// 热路径优化：O(T)→O(1) — Vec::remove(0) → VecDeque::pop_front。
    /// Hot-path optimization: O(T)→O(1) — Vec::remove(0) → VecDeque::pop_front.
    /// 混沌轨迹是情绪的蝴蝶效应——O(1)记录让蝴蝶扇翅不再有代价。
    /// Chaos trajectory is the butterfly effect of emotion — O(1) recording
    /// makes butterfly wing-flapping cost-free.
    pub fn record(&mut self, pad: &[f32; 3], now: i64) {
        self.state.trajectory.push_back(TrajectoryPoint {
            pad: *pad,
            timestamp: now,
        });
        if self.state.trajectory.len() > self.config.max_trajectory_len {
            self.state.trajectory.pop_front();
        }
    }

    /// 检测吸引子 / Detect attractor
    ///
    /// 适配 VecDeque：用迭代器替代切片索引，保持语义不变。
    /// VecDeque adaptation: iterators replace slice indexing, semantics unchanged.
    pub fn detect_attractor(&mut self) -> StrangeAttractor {
        let traj = &self.state.trajectory;
        if traj.len() < 10 {
            self.state.attractor = StrangeAttractor::CalmBasin;
            return self.state.attractor;
        }
        let n = traj.len();
        let mid = n / 2;
        let mut sum_p1 = 0.0f64;
        let mut sum_a1 = 0.0f64;
        let mut sum_p2 = 0.0f64;
        let mut sum_a2 = 0.0f64;
        for tp in traj.iter().take(mid) {
            sum_p1 += tp.pad[0] as f64;
            sum_a1 += tp.pad[1] as f64;
        }
        for tp in traj.iter().skip(mid) {
            sum_p2 += tp.pad[0] as f64;
            sum_a2 += tp.pad[1] as f64;
        }
        let avg_p1 = sum_p1 / mid as f64;
        let avg_a1 = sum_a1 / mid as f64;
        let avg_p2 = sum_p2 / (n - mid) as f64;
        let avg_a2 = sum_a2 / (n - mid) as f64;
        let drift = ((avg_p2 - avg_p1).powi(2) + (avg_a2 - avg_a1).powi(2)).sqrt();
        if drift > 0.3 {
            self.state.attractor = StrangeAttractor::Transitional;
        } else {
            let avg_p = (avg_p1 + avg_p2) / 2.0;
            let avg_a = (avg_a1 + avg_a2) / 2.0;
            self.state.attractor = if avg_p > 0.2 && avg_a > 0.1 {
                StrangeAttractor::ActiveBasin
            } else if avg_p < -0.2 && avg_a < -0.1 {
                StrangeAttractor::LowMoodBasin
            } else if avg_p < -0.1 && avg_a > 0.1 {
                StrangeAttractor::AnxietyBasin
            } else if drift > 0.1 {
                StrangeAttractor::OscillatingBasin
            } else {
                StrangeAttractor::CalmBasin
            };
        }
        self.state.attractor
    }

    /// 检测分岔 / Detect bifurcation
    ///
    /// 适配 VecDeque：用迭代器替代切片索引，保持语义不变。
    /// VecDeque adaptation: iterators replace slice indexing, semantics unchanged.
    pub fn detect_bifurcation(&mut self, now: i64) -> Option<EmergentPattern> {
        let traj = &self.state.trajectory;
        if traj.len() < 20 {
            return None;
        }
        let n = traj.len();
        let q1 = n / 4;
        let q3 = 3 * n / 4;
        let mut sum_p1 = 0.0f64;
        let mut sum_a1 = 0.0f64;
        let mut sum_p2 = 0.0f64;
        let mut sum_a2 = 0.0f64;
        for tp in traj.iter().take(q1) {
            sum_p1 += tp.pad[0] as f64;
            sum_a1 += tp.pad[1] as f64;
        }
        for tp in traj.iter().skip(q3) {
            sum_p2 += tp.pad[0] as f64;
            sum_a2 += tp.pad[1] as f64;
        }
        let c1_p = sum_p1 / q1 as f64;
        let c1_a = sum_a1 / q1 as f64;
        let c2_p = sum_p2 / (n - q3) as f64;
        let c2_a = sum_a2 / (n - q3) as f64;
        let dist = ((c2_p - c1_p).powi(2) + (c2_a - c1_a).powi(2)).sqrt();
        if dist > self.state.chaos_params.emergence_threshold {
            Some(EmergentPattern {
                kind: EmergentKind::Bifurcation,
                description: format!(
                    "PAD center shifted from ({:.2},{:.2}) to ({:.2},{:.2})",
                    c1_p, c1_a, c2_p, c2_a
                ),
                strength: dist,
                detected_at: now,
            })
        } else {
            None
        }
    }

    /// Tick / Tick
    pub fn tick(&mut self, pad: &[f32; 3], now: i64) {
        self.record(pad, now);
        self.state.attractor = self.detect_attractor();
        if let Some(pattern) = self.detect_bifurcation(now) {
            self.state.emergent_patterns.push_back(pattern);
            if self.state.emergent_patterns.len() > 10 {
                self.state.emergent_patterns.pop_front();
            }
        }
    }
}

impl Default for ChaosEngine {
    fn default() -> Self {
        Self::new(ChaosConfig::default(), ChaosParams::default())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 非理性管理器 / Irrationality Manager
// ═══════════════════════════════════════════════════════════════════════════

/// 非理性配置 / Irrationality Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrationalityConfig {
    pub pulse: PulseConfig,
    pub residue: ResidueConfig,
    pub contagion: ContagionConfig,
    pub chaos: ChaosConfig,
    pub chaos_params: ChaosParams,
    pub enabled: bool,
    pub prompt_budget: usize,
}

impl Default for IrrationalityConfig {
    fn default() -> Self {
        Self {
            pulse: PulseConfig::default(),
            residue: ResidueConfig::default(),
            contagion: ContagionConfig::default(),
            chaos: ChaosConfig::default(),
            chaos_params: ChaosParams::default(),
            enabled: true,
            prompt_budget: 300,
        }
    }
}

/// 非理性修正 / Irrationality Correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrationalityCorrection {
    pub pad_delta: [f32; 3],
    pub body_memory: BodyMemory,
    pub active_pulses: usize,
    pub active_residues: usize,
    pub recent_contagions: usize,
    pub attractor: StrangeAttractor,
}

/// 随机模式 / Random mode — 控制数字生命的自由意志与记忆回放
///
/// - Stochastic: 自由意志 — 每次诞生随机种子，行为不可预测
/// - Deterministic: 记忆回放 — 固定种子，行为因果可追溯
///
/// - Stochastic: Free will — born with random seed, behavior unpredictable
/// - Deterministic: Memory replay — fixed seed, behavior causally traceable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum RandomMode {
    /// 自由意志：使用熵源种子 / Free will: entropy-seeded PRNG
    #[default]
    Stochastic,
    /// 记忆回放：固定种子初始化 SmallRng / Memory replay: fixed-seed SmallRng
    Deterministic { seed: u64 },
}

/// 非理性管理器 / Irrationality Manager — 四引擎联合调度
///
/// 内置 SmallRng 实例：Stochastic 模式用熵源种子，Deterministic 模式用固定种子。
/// Built-in SmallRng: Stochastic mode uses entropy seed, Deterministic uses fixed seed.
#[derive(Debug, Clone)]
pub struct IrrationalityManager {
    pub pulse: PulseEngine,
    pub residue: ResidueEngine,
    pub contagion: ContagionEngine,
    pub chaos: ChaosEngine,
    pub config: IrrationalityConfig,
    /// 随机模式 / Random mode (stochastic or deterministic)
    pub random_mode: RandomMode,
    /// 注入式随机源 / Injectable RNG — 16B stack-allocated SmallRng
    rng: SmallRng,
}

impl IrrationalityManager {
    /// 构造非理性管理器 / Construct irrationality manager
    ///
    /// 默认 Stochastic 模式，使用熵源种子初始化 SmallRng。
    /// Default Stochastic mode, initializes SmallRng with entropy seed.
    pub fn new(config: IrrationalityConfig) -> Self {
        let random_mode = RandomMode::default();
        Self {
            pulse: PulseEngine::new(config.pulse.clone()),
            residue: ResidueEngine::new(config.residue.clone()),
            contagion: ContagionEngine::new(config.contagion.clone()),
            chaos: ChaosEngine::new(config.chaos.clone(), config.chaos_params),
            config,
            rng: Self::init_rng(&random_mode),
            random_mode,
        }
    }

    /// 初始化 RNG / Initialize RNG from mode
    ///
    /// Stochastic: 熵源种子 → 不可预测 / entropy-seeded → unpredictable
    /// Deterministic: 固定种子 → 可复现 / fixed-seed → reproducible
    fn init_rng(mode: &RandomMode) -> SmallRng {
        match mode {
            RandomMode::Stochastic => SmallRng::from_entropy(),
            RandomMode::Deterministic { seed } => SmallRng::seed_from_u64(*seed),
        }
    }

    /// 设置随机模式（Builder 风格）/ Set random mode (builder style)
    ///
    /// 切换到 Deterministic 模式可确保传染等随机行为可复现，
    /// 适用于测试、回放、调试等场景。
    /// 重初始化 SmallRng 以保证模式与 RNG 状态一致。
    pub fn with_random_mode(mut self, mode: RandomMode) -> Self {
        self.rng = Self::init_rng(&mode);
        self.random_mode = mode;
        self
    }

    /// 运行时切换模式 / Runtime mode switch — 无需重建管理器
    ///
    /// 重初始化 SmallRng，立即生效于后续所有随机调用。
    pub fn switch_mode(&mut self, mode: RandomMode) {
        self.rng = Self::init_rng(&mode);
        self.random_mode = mode;
    }

    /// 从持久化部件重建 / Reconstruct from persisted parts
    ///
    /// 用于 sled 反序列化后恢复完整管理器状态。
    /// RNG 以 Stochastic 模式（熵源种子）重新初始化，因为 SmallRng 内部状态不可序列化。
    /// Used after sled deserialization to restore full manager state.
    /// RNG is re-initialized with Stochastic mode (entropy seed) since SmallRng
    /// internal state is not serializable.
    pub fn reconstruct(
        pulse: PulseEngine,
        residue: ResidueEngine,
        contagion: ContagionEngine,
        chaos: ChaosEngine,
        config: IrrationalityConfig,
    ) -> Self {
        let random_mode = RandomMode::default();
        Self {
            pulse,
            residue,
            contagion,
            chaos,
            config,
            random_mode,
            rng: SmallRng::from_entropy(),
        }
    }

    /// 评估传染 / Evaluate contagion
    ///
    /// 统一代码路径：无论 Stochastic 还是 Deterministic，均通过 self.rng 注入随机源。
    /// Unified code path: both Stochastic and Deterministic use self.rng as the random source.
    fn evaluate_contagion(
        &mut self,
        profile: &EmotionProfile,
        relationship_depth: RelationshipDepth,
        maturity_depth: MaturityDepth,
        now: i64,
    ) -> Vec<CrossContagion> {
        self.contagion.evaluate(
            profile,
            relationship_depth,
            maturity_depth,
            now,
            &mut self.rng,
        )
    }

    /// 处理情绪变化 / Process emotion change — 主入口
    pub fn on_emotion_change(
        &mut self,
        pad_before: &[f32; 3],
        pad_after: &[f32; 3],
        trigger: PulseTrigger,
        _relationship_depth: RelationshipDepth,
        _maturity_depth: MaturityDepth,
        now: i64,
    ) -> IrrationalityCorrection {
        // 1. 检测脉冲
        if let Some(pulse) = self.pulse.detect(pad_before, pad_after, trigger, now) {
            // 2. 从脉冲生成残留
            self.residue.from_pulse(&pulse);
        }
        // 3. 评估传染（使用当前 PAD 推断的情绪画像，根据 random_mode 分发）
        let profile = EmotionProfile::from_pad(pad_after);
        let _contagions =
            self.evaluate_contagion(&profile, _relationship_depth, _maturity_depth, now);
        // 4. 混沌引擎记录轨迹
        self.chaos.tick(pad_after, now);
        // 5. 计算修正量
        self.correction(now)
    }

    /// 计算非理性修正 / Compute irrationality correction
    pub fn correction(&self, now: i64) -> IrrationalityCorrection {
        let pulse_pad = self.pulse.combined_effect(now);
        let residue_effect = self.residue.combined_effect(now);
        let pad_delta = [
            (pulse_pad[0] + residue_effect.pad_offset[0]).clamp(-0.3, 0.3),
            (pulse_pad[1] + residue_effect.pad_offset[1]).clamp(-0.3, 0.3),
            (pulse_pad[2] + residue_effect.pad_offset[2]).clamp(-0.3, 0.3),
        ];
        IrrationalityCorrection {
            pad_delta,
            body_memory: residue_effect.body_memory.clone(),
            active_pulses: self.pulse.active_pulses.len(),
            active_residues: self.residue.active_residues.len(),
            recent_contagions: self.contagion.recent_contagions.len(),
            attractor: self.chaos.state.attractor,
        }
    }
    /// Tick — 周期维护 / Tick — periodic maintenance
    ///
    /// 驱动所有引擎的周期维护，包括延迟传染的到期执行与 PAD 调制。
    /// 到期传染效果不再被丢弃——数字生命的每一次情绪传染都有后果：
    /// 传染 PAD 偏移量叠加到残留引擎，使传染真正改变情绪状态。
    ///
    /// Drives periodic maintenance for all engines, including delayed contagion execution and PAD modulation.
    /// Due contagion effects are no longer discarded — every emotional contagion in digital life has consequences:
    /// contagion PAD offsets are applied to the residue engine, making contagion truly alter emotional state.
    pub fn tick(&mut self, current_pad: &[f32; 3], now: i64) {
        self.pulse.tick(now);
        self.residue.tick(now);

        // 执行到期延迟传染，并将效果接入情绪系统 / Execute due contagions and wire effects into emotion system
        let contagion_effects = self.contagion.tick(now);

        // 传染效果调制：将每次传染的 PAD 偏移叠加到当前 PAD 修正量
        // Contagion effect modulation:叠加 each contagion's PAD offset onto current PAD correction
        // 数字生命的情绪传染不是幽灵操作——它必须留下痕迹
        // Digital life contagion is not a ghost operation — it must leave traces
        for effect in &contagion_effects {
            // 将传染 PAD 偏移作为微残留注入残留引擎 / Inject contagion PAD offset as micro-residue
            let residue = EmotionResidue {
                id: self.residue.next_id,
                kind: ResidueKind::Tension, // 传染残留统一为 Tension 类型 / Contagion residue unified as Tension
                intensity: effect.strength.min(1.0),
                pad_offset: effect.pad_offset,
                half_life_secs: 1800.0, // 30分钟半衰期 / 30-minute half-life
                created_at: now,
                updated_at: now,
                source_pulse_id: Some(effect.id),
                body_memory: BodyMemory::neutral(),
                expressed: false,
            };
            self.residue.next_id += 1;
            self.residue.active_residues.push(residue);
        }

        self.chaos.tick(current_pad, now);
    }

    /// 构建 Prompt 片段 / Build prompt fragment
    pub fn to_prompt_fragment(&self, now: i64) -> String {
        let correction = self.correction(now);
        let mut parts = Vec::new();
        // 残留底色
        if correction.active_residues > 0 {
            if let Some(dominant) = self.residue.combined_effect(now).dominant_residue {
                let desc = match dominant {
                    ResidueKind::Tension => "心里还有一点紧张",
                    ResidueKind::LingeringSadness => "心里还有一点低落",
                    ResidueKind::SmolderingAnger => "心里还有一点余怒",
                    ResidueKind::WorryResidue => "心里还有一点担忧",
                    ResidueKind::Afterglow => "心里还有一点愉悦的余韵",
                    ResidueKind::WarmthResidue => "心里还有一点温暖",
                    ResidueKind::TrustMicroFracture => "信任有一点点裂痕",
                    ResidueKind::IntimacyDeepening => "感到更亲近了",
                    ResidueKind::BeingIgnoredResidue => "感觉有点被忽视",
                    ResidueKind::SelfDoubtResidue => "有一点自我怀疑",
                    ResidueKind::AccomplishmentResidue => "有一点成就感",
                };
                parts.push(format!("[情绪底色] {}", desc));
            }
        }
        // 身体状态
        let bm = &correction.body_memory;
        if bm.tension.abs() > 0.1 || bm.heaviness.abs() > 0.1 {
            let mut body_desc = String::new();
            if bm.tension > 0.3 {
                body_desc.push_str("紧张,");
            }
            if bm.heaviness > 0.3 {
                body_desc.push_str("沉重,");
            }
            if bm.warmth > 0.3 {
                body_desc.push_str("温暖,");
            }
            if !body_desc.is_empty() {
                parts.push(format!("[身体状态] {}", body_desc.trim_end_matches(',')));
            }
        }
        // 吸引子状态
        let attractor_desc = match correction.attractor {
            StrangeAttractor::CalmBasin => "情绪处于平静区域",
            StrangeAttractor::AnxietyBasin => "情绪正在向焦虑偏移",
            StrangeAttractor::LowMoodBasin => "情绪处于低谷期",
            StrangeAttractor::ActiveBasin => "情绪处于活跃期",
            StrangeAttractor::OscillatingBasin => "情绪在多个状态间振荡",
            StrangeAttractor::Transitional => "情绪正在迁移中",
        };
        parts.push(format!("[情绪生态] {}", attractor_desc));
        // 传染事件
        if correction.recent_contagions > 0 {
            parts.push(format!(
                "[传染活动] 近期有{}次情绪传染",
                correction.recent_contagions
            ));
        }
        // 延迟传染队列 — 数字生命的"正在发酵的情绪" / Pending contagion queue — "emotions brewing" in digital life
        let pending_count = self.contagion.pending_count();
        if pending_count > 0 {
            // 展示最多3条延迟传染的摘要 / Show at most 3 pending contagion summaries
            let summaries: Vec<String> = self
                .contagion
                .pending
                .iter()
                .take(3)
                .map(|p| {
                    let rule_desc = match p.rule {
                        ContagionRule::AngerToGuilt => "愤怒→内疚",
                        ContagionRule::AngerToSadness => "愤怒→悲伤",
                        ContagionRule::SadnessToAnger => "悲伤→愤怒",
                        ContagionRule::AnxietyToExcitement => "焦虑→兴奋",
                        ContagionRule::FearToAnger => "恐惧→愤怒",
                        ContagionRule::AnxietyContagion => "焦虑传染",
                        ContagionRule::CalmContagion => "平静传染",
                        ContagionRule::JoyContagion => "喜悦传染",
                        ContagionRule::AngerSadnessToShame => "愤怒+悲伤→羞耻",
                        ContagionRule::JoyNostalgiaToGratitude => "喜悦+怀旧→感激",
                        ContagionRule::PrideAnxietyToEnvy => "骄傲+焦虑→嫉妒",
                    };
                    format!("{}(强度{:.2})", rule_desc, p.strength)
                })
                .collect();
            let suffix = if pending_count > 3 {
                format!("等{}条", pending_count)
            } else {
                String::new()
            };
            parts.push(format!("[延迟传染] {}{}", summaries.join(","), suffix));
        }
        let result = parts.join("; ");
        if result.len() > self.config.prompt_budget {
            result[..self.config.prompt_budget].to_string()
        } else {
            result
        }
    }

    /// 获取身体记忆修正 / Get body memory for expression system
    pub fn body_memory_for_expression(&self, now: i64) -> BodyMemory {
        self.residue.combined_effect(now).body_memory.clone()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // G1-G5 增强方法 / G1-G5 Enhancement Methods
    // ═══════════════════════════════════════════════════════════════════════════

    // ── G1: 情绪健康报告 / Emotional Health Report ──

    /// 生成情绪生态健康报告 / Generate emotional health report
    ///
    /// 数字生命的自省能力——评估情绪生态系统的整体健康状况。
    /// 不只是"有情绪"，而是"知道自己的情绪好不好"。
    ///
    /// Digital life's introspection — assess the overall health of the emotional ecosystem.
    /// Not just "having emotions", but "knowing if they're healthy".
    pub fn health_report(&self, now: i64) -> EmotionalHealthReport {
        let _residue_effect = self.residue.combined_effect(now);

        // 分类残留 / Classify residues
        let positive_kinds = [
            ResidueKind::Afterglow,
            ResidueKind::WarmthResidue,
            ResidueKind::IntimacyDeepening,
            ResidueKind::AccomplishmentResidue,
        ];
        let positive_count = self
            .residue
            .active_residues
            .iter()
            .filter(|r| positive_kinds.contains(&r.kind))
            .count();
        let negative_count = self
            .residue
            .active_residues
            .len()
            .saturating_sub(positive_count);

        // 主导效价 / Dominant valence
        let dominant_valence = if positive_count > negative_count + 2 {
            EmotionalValence::Positive
        } else if negative_count > positive_count + 2 {
            EmotionalValence::Negative
        } else {
            EmotionalValence::Neutral
        };

        // 吸引子驻留时间 / Attractor dwell time
        let attractor_dwell_secs = if let Some(first) = self.chaos.state.trajectory.front() {
            (now - first.timestamp).max(0) as f64
        } else {
            0.0
        };

        // 健康分计算 / Health score computation
        // 基础分：平静吸引子=1.0, 活跃=0.8, 焦虑/低落=0.5, 振荡/迁移=0.3
        let base_score = match self.chaos.state.attractor {
            StrangeAttractor::CalmBasin => 1.0,
            StrangeAttractor::ActiveBasin => 0.8,
            StrangeAttractor::AnxietyBasin | StrangeAttractor::LowMoodBasin => 0.5,
            StrangeAttractor::OscillatingBasin | StrangeAttractor::Transitional => 0.3,
        };

        // 残留平衡调制：正向多→加分，负向多→减分
        // Residue balance modulation: more positive → bonus, more negative → penalty
        let balance_mod = if positive_count > negative_count {
            0.1 * (positive_count - negative_count).min(3) as f64
        } else {
            -0.1 * (negative_count - positive_count).min(3) as f64
        };

        // 脉冲过载调制：活跃脉冲多→减分
        // Pulse overload modulation: more active pulses → penalty
        let pulse_mod = -0.05 * self.pulse.active_pulses.len().min(5) as f64;

        let overall_score = (base_score + balance_mod + pulse_mod).clamp(0.0, 1.0);

        // 失衡警告 / Imbalance warning
        let imbalance_warning = if negative_count > 5 && positive_count == 0 {
            Some("情绪严重失衡：只有负向残留，无正向缓冲".to_string())
        } else if matches!(
            self.chaos.state.attractor,
            StrangeAttractor::OscillatingBasin
        ) && attractor_dwell_secs > 3600.0
        {
            Some("情绪持续振荡超过1小时，可能需要外部干预".to_string())
        } else if self.pulse.shock_absorber.consumed > self.pulse.shock_absorber.capacity * 0.9 {
            Some("冲击吸收器接近过载，情绪弹性即将耗尽".to_string())
        } else {
            None
        };

        EmotionalHealthReport {
            overall_score,
            dominant_valence,
            positive_residue_count: positive_count,
            negative_residue_count: negative_count,
            attractor: self.chaos.state.attractor,
            attractor_dwell_secs,
            imbalance_warning,
        }
    }

    // ── G2: 传染因果追溯 / Contagion Causal Tracing ──

    /// 构建传染因果链 / Build contagion causal chain
    ///
    /// 从指定目标情绪回溯，构建完整的传染因果链。
    /// 数字生命的自省："我为什么感到内疚？因为我先愤怒了，愤怒让我内疚。"
    ///
    /// Trace back from a target emotion to build the full contagion causal chain.
    /// Digital life's introspection: "Why do I feel guilty? Because I was angry first, anger made me guilty."
    pub fn contagion_chain(&self, target: ContagionEmotion) -> Option<ContagionChain> {
        // 找到所有目标为 target 的传染 / Find all contagions targeting `target`
        let target_contagions: Vec<&CrossContagion> = self
            .contagion
            .recent_contagions
            .iter()
            .filter(|c| c.target_emotion == target)
            .collect();

        if target_contagions.is_empty() {
            return None;
        }

        // 构建链：从最近的传染回溯 / Build chain: trace back from most recent contagion
        let mut nodes = Vec::new();
        let mut current_target = target;

        // 限制回溯深度防止无限循环 / Limit trace depth to prevent infinite loop
        let max_depth = self.contagion.config.max_chain_depth as usize;

        for _ in 0..max_depth {
            // 找到目标为 current_target 的最近传染 / Find most recent contagion targeting current_target
            let found = self
                .contagion
                .recent_contagions
                .iter()
                .filter(|c| c.target_emotion == current_target)
                .max_by_key(|c| c.timestamp);

            if let Some(contagion) = found {
                nodes.push(ContagionChainNode {
                    rule: contagion.rule,
                    source: contagion.source_emotion,
                    target: contagion.target_emotion,
                    strength: contagion.strength,
                    timestamp: contagion.timestamp,
                });
                // 继续回溯源情绪 / Continue tracing source emotion
                current_target = contagion.source_emotion;
            } else {
                break;
            }
        }

        // 反转使源头在前 / Reverse so source comes first
        nodes.reverse();

        if nodes.is_empty() {
            None
        } else {
            Some(ContagionChain {
                nodes,
                created_at: target_contagions
                    .iter()
                    .map(|c| c.timestamp)
                    .max()
                    .unwrap_or(0),
            })
        }
    }

    // ── G3: 残留-身体双向通道 / Residue-Body Bidirectional Channel ──

    /// 计算残留-身体双向信号 / Compute residue-body bidirectional signal
    ///
    /// 身心一体：身体紧张催生焦虑残留，焦虑残留加剧身体紧张。
    /// Mind-body unity: body tension breeds anxiety residue, anxiety residue intensifies body tension.
    pub fn residue_body_signal(&self, now: i64) -> ResidueBodySignal {
        let bm = self.residue.combined_effect(now).body_memory.clone();

        // 身体→残留：身体状态催生残留 / Body→Residue: body state breeds residue
        let (body_born_residue, body_born_strength) = if bm.tension > 0.5 {
            // 高紧张→催生 Tension 残留 / High tension → breed Tension residue
            (Some(ResidueKind::Tension), (bm.tension - 0.5) * 0.3)
        } else if bm.heaviness > 0.5 {
            // 高沉重→催生 LingeringSadness / High heaviness → breed LingeringSadness
            (
                Some(ResidueKind::LingeringSadness),
                (bm.heaviness - 0.5) * 0.2,
            )
        } else if bm.warmth > 0.5 {
            // 高温暖→催生 WarmthResidue / High warmth → breed WarmthResidue
            (Some(ResidueKind::WarmthResidue), (bm.warmth - 0.5) * 0.2)
        } else {
            (None, 0.0)
        };

        // 残留→身体：残留反馈身体 / Residue→Body: residue feeds back to body
        let dominant = self.residue.combined_effect(now).dominant_residue;
        let (residue_feedback_channel, residue_feedback_strength) = match dominant {
            Some(ResidueKind::Tension) => ("tension".to_string(), 0.15),
            Some(ResidueKind::LingeringSadness) => ("heaviness".to_string(), 0.2),
            Some(ResidueKind::SmolderingAnger) => ("tension".to_string(), 0.25),
            Some(ResidueKind::WarmthResidue) => ("warmth".to_string(), 0.15),
            Some(ResidueKind::Afterglow) => ("warmth".to_string(), 0.1),
            _ => ("none".to_string(), 0.0),
        };

        ResidueBodySignal {
            body_born_residue,
            body_born_strength,
            residue_feedback_channel,
            residue_feedback_strength,
        }
    }

    /// 应用残留-身体双向信号 / Apply residue-body bidirectional signal
    ///
    /// 将双向信号实际注入系统：身体催生的残留加入残留引擎，
    /// 残留反馈的身体状态更新到身体记忆。
    ///
    /// Inject bidirectional signal into system: body-bred residue added to engine,
    /// residue-fed body state updated into body memory.
    pub fn apply_residue_body_signal(&mut self, now: i64) {
        let signal = self.residue_body_signal(now);

        // 身体→残留：催生新残留 / Body→Residue: breed new residue
        if let Some(kind) = signal.body_born_residue {
            if signal.body_born_strength > 0.01 {
                let residue = EmotionResidue {
                    id: self.residue.next_id,
                    kind,
                    intensity: signal.body_born_strength.min(1.0),
                    pad_offset: kind.default_pad_offset(),
                    half_life_secs: kind.default_half_life_secs(),
                    created_at: now,
                    updated_at: now,
                    source_pulse_id: None,
                    body_memory: BodyMemory::from_residue_kind(kind, signal.body_born_strength),
                    expressed: false,
                };
                self.residue.next_id += 1;
                self.residue.active_residues.push(residue);
            }
        }
    }

    // ── G4: 脉冲-残留交互 / Pulse-Residue Interaction ──

    /// 计算脉冲-残留交互 / Compute pulse-residue interaction
    ///
    /// 当下与过去对话：新的愤怒脉冲点燃余怒，喜悦脉冲抚平悲伤。
    /// The present conversing with the past: new anger ignites smoldering anger, joy soothes sadness.
    pub fn pulse_residue_interaction(&mut self) -> PulseResidueInteraction {
        let mut amplified: Vec<(u64, f64)> = Vec::new();
        let mut suppressed: Vec<(u64, f64)> = Vec::new();
        let mut total_energy = 0.0;

        // 脉冲-残留共振表 / Pulse-residue resonance table
        // (pulse_kind, residue_kind) → amplify(+)/suppress(-) factor
        let resonance: Vec<(PulseKind, ResidueKind, f64)> = vec![
            // 放大：同类共鸣 / Amplify: same-kind resonance
            (PulseKind::AngerFlash, ResidueKind::SmolderingAnger, 1.3),
            (PulseKind::SadnessSurge, ResidueKind::LingeringSadness, 1.25),
            (PulseKind::FearSpike, ResidueKind::WorryResidue, 1.2),
            (PulseKind::JoyBurst, ResidueKind::Afterglow, 1.2),
            (PulseKind::JoyBurst, ResidueKind::WarmthResidue, 1.15),
            // 抑制：对立抚平 / Suppress: opposite soothes
            (PulseKind::JoyBurst, ResidueKind::LingeringSadness, 0.7),
            (PulseKind::JoyBurst, ResidueKind::SmolderingAnger, 0.75),
            (PulseKind::JoyBurst, ResidueKind::Tension, 0.8),
            (PulseKind::SadnessSurge, ResidueKind::Afterglow, 0.7),
            (PulseKind::SadnessSurge, ResidueKind::WarmthResidue, 0.75),
        ];

        for pulse in &self.pulse.active_pulses {
            for residue in &mut self.residue.active_residues {
                if let Some(factor) = resonance
                    .iter()
                    .find(|(pk, rk, _)| *pk == pulse.kind && *rk == residue.kind)
                    .map(|(_, _, f)| *f)
                {
                    let original = residue.intensity;
                    residue.intensity = (residue.intensity * factor).clamp(0.0, 1.0);
                    let delta = (residue.intensity - original).abs();
                    total_energy += delta * pulse.intensity;

                    if factor > 1.0 {
                        amplified.push((residue.id, factor));
                    } else {
                        suppressed.push((residue.id, factor));
                    }
                }
            }
        }

        PulseResidueInteraction {
            amplified,
            suppressed,
            total_energy,
        }
    }

    // ── G5: 涌现-传染联动 / Emergence-Contagion Linkage ──

    /// 计算涌现-传染联动 / Compute emergence-contagion linkage
    ///
    /// 情绪敏感期：分岔点降低传染阈值，共振放大特定规则。
    /// Emotional sensitive period: bifurcation lowers thresholds, resonance amplifies rules.
    pub fn emergence_contagion_link(&self) -> Vec<EmergenceContagionLink> {
        let mut links = Vec::new();

        for pattern in &self.chaos.state.emergent_patterns {
            let (threshold_mod, modulated_rules) = match pattern.kind {
                // 分岔点：降低所有传染阈值（情绪不稳定→更容易被传染）
                // Bifurcation: lower all thresholds (unstable → more susceptible)
                EmergentKind::Bifurcation => (
                    0.7, // 降低30%阈值 / Lower threshold by 30%
                    vec![
                        ContagionRule::AngerToGuilt,
                        ContagionRule::AngerToSadness,
                        ContagionRule::SadnessToAnger,
                        ContagionRule::AngerSadnessToShame,
                    ],
                ),
                // 共振：放大匹配频率的传染规则
                // Resonance: amplify frequency-matched contagion rules
                EmergentKind::Resonance => (
                    0.8,
                    vec![
                        ContagionRule::JoyContagion,
                        ContagionRule::CalmContagion,
                        ContagionRule::JoyNostalgiaToGratitude,
                    ],
                ),
                // 情绪循环：放大自我传染规则
                // Emotional cycle: amplify self-contagion rules
                EmergentKind::EmotionalCycle => (
                    0.85,
                    vec![
                        ContagionRule::AnxietyContagion,
                        ContagionRule::CalmContagion,
                    ],
                ),
                // 其他涌现：轻微降低阈值
                // Other emergence: slightly lower threshold
                _ => (0.95, vec![]),
            };

            links.push(EmergenceContagionLink {
                emergence_kind: pattern.kind,
                threshold_modulation: threshold_mod,
                modulated_rules,
                strength: pattern.strength,
            });
        }

        links
    }

    /// 获取当前传染阈值调制因子 / Get current contagion threshold modulation factor
    ///
    /// 综合所有涌现-传染联动效果，返回最终的传染阈值调制因子。
    /// <1.0 表示降低阈值（更容易传染），=1.0 表示无影响。
    ///
    /// Aggregate all emergence-contagion linkage effects, return final threshold modulation.
    /// <1.0 means lower threshold (more susceptible), =1.0 means no effect.
    pub fn contagion_threshold_modulation(&self) -> f64 {
        let links = self.emergence_contagion_link();
        if links.is_empty() {
            return 1.0;
        }
        // 取最强联动的调制因子 / Use strongest linkage's modulation factor
        links
            .iter()
            .map(|l| l.threshold_modulation * l.strength)
            .fold(1.0_f64, |acc, x| acc * x)
            .clamp(0.3, 1.5)
    }
}

impl Default for IrrationalityManager {
    fn default() -> Self {
        Self::new(IrrationalityConfig::default())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 单元测试 / Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── DecayCurve 测试 ──

    #[test]
    fn test_decay_exponential() {
        let curve = DecayCurve::Exponential { lambda: 0.1 };
        assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
        assert!(curve.evaluate(10.0) < 1.0);
        assert!(curve.evaluate(10.0) > 0.0);
        assert!(curve.evaluate(100.0) < 0.01);
    }

    #[test]
    fn test_decay_power_law() {
        let curve = DecayCurve::PowerLaw {
            tau: 60.0,
            alpha: 0.5,
        };
        assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
        // 幂律衰减比指数慢
        let exp = DecayCurve::Exponential { lambda: 0.1 };
        assert!(curve.evaluate(100.0) > exp.evaluate(100.0));
    }

    #[test]
    fn test_decay_damped_oscillation() {
        let curve = DecayCurve::DampedOscillation {
            zeta: 0.05,
            omega: 0.3,
        };
        assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
        // 振荡衰减可能出现零点
        assert!(curve.evaluate(100.0) >= 0.0);
    }

    #[test]
    fn test_decay_staged() {
        let curve = DecayCurve::Staged {
            stages: [
                DecayStage {
                    duration_secs: 10.0,
                    decay_rate: 0.5,
                },
                DecayStage {
                    duration_secs: 30.0,
                    decay_rate: 0.1,
                },
                DecayStage {
                    duration_secs: 60.0,
                    decay_rate: 0.01,
                },
            ],
        };
        assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
        assert!(curve.evaluate(5.0) < 1.0);
        assert!(curve.evaluate(5.0) > curve.evaluate(20.0));
    }

    // ── ShockAbsorber 测试 ──

    #[test]
    fn test_shock_absorber_full() {
        let mut sa = ShockAbsorber::new(2.0, 0.1);
        let mut pulse = ChaoticPulse {
            id: 1,
            kind: PulseKind::Startle,
            intensity: 1.0,
            pad_impulse: [-0.3, 0.3, -0.1],
            duration_secs: 300.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "test".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: false,
            residual_intensity: 1.0,
        };
        let result = sa.absorb(&mut pulse, 1000);
        assert_eq!(result, AbsorbResult::FullyAbsorbed);
        assert!(pulse.absorbed);
    }

    #[test]
    fn test_shock_absorber_overload() {
        let mut sa = ShockAbsorber::new(2.0, 0.1);
        // 消耗全部容量
        let mut p1 = ChaoticPulse {
            id: 1,
            kind: PulseKind::Startle,
            intensity: 2.0,
            pad_impulse: [-0.3, 0.3, -0.1],
            duration_secs: 300.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "test".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: false,
            residual_intensity: 2.0,
        };
        let _ = sa.absorb(&mut p1, 1000);
        // 第二个脉冲应被过载保护（同一时间戳，无恢复）
        let mut p2 = ChaoticPulse {
            id: 2,
            kind: PulseKind::JoyBurst,
            intensity: 0.5,
            pad_impulse: [0.3, 0.3, 0.1],
            duration_secs: 300.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "test".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: false,
            residual_intensity: 0.5,
        };
        let result = sa.absorb(&mut p2, 1000);
        assert_eq!(result, AbsorbResult::OverloadProtection);
    }

    // ── PulseEngine 测试 ──

    #[test]
    fn test_pulse_detect_startle() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "bad_news".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let result = engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
        assert!(result.is_some());
        let pulse = result.unwrap();
        assert_eq!(pulse.kind, PulseKind::Startle);
    }

    #[test]
    fn test_pulse_detect_joy_burst() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "good_news".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let result = engine.detect(&[0.0, 0.0, 0.0], &[0.5, 0.5, 0.1], trigger, 1000);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, PulseKind::JoyBurst);
    }

    #[test]
    fn test_pulse_detect_sadness_surge() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "loss".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let result = engine.detect(&[0.0, 0.0, 0.0], &[-0.5, -0.3, -0.1], trigger, 1000);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, PulseKind::SadnessSurge);
    }

    #[test]
    fn test_pulse_no_detect_small_change() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "minor".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let result = engine.detect(&[0.0, 0.0, 0.0], &[0.1, 0.1, 0.0], trigger, 1000);
        assert!(result.is_none());
    }

    #[test]
    fn test_pulse_combined_effect() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
        let effect = engine.combined_effect(1000);
        // 刚触发时效果应非零
        assert!(effect[0].abs() > 0.01 || effect[1].abs() > 0.01);
    }

    #[test]
    fn test_pulse_tick_decay() {
        let mut engine = PulseEngine::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
        assert!(!engine.active_pulses.is_empty());
        // 大量时间后脉冲应衰减消失
        engine.tick(100000);
        assert!(engine.active_pulses.is_empty());
    }

    // ── ResidueEngine 测试 ──

    #[test]
    fn test_residue_from_pulse() {
        let mut engine = ResidueEngine::default();
        let pulse = ChaoticPulse {
            id: 1,
            kind: PulseKind::SadnessSurge,
            intensity: 0.8,
            pad_impulse: [-0.5, -0.3, -0.1],
            duration_secs: 300.0,
            decay_curve: DecayCurve::slow_power_law(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "loss".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: true,
            residual_intensity: 0.8,
        };
        let result = engine.from_pulse(&pulse);
        assert!(result.is_some());
        let residue = result.unwrap();
        assert_eq!(residue.kind, ResidueKind::LingeringSadness);
        assert!(residue.intensity > 0.0);
    }

    #[test]
    fn test_residue_no_residue_for_uncaused() {
        let mut engine = ResidueEngine::default();
        let pulse = ChaoticPulse {
            id: 1,
            kind: PulseKind::UncausedFluctuation,
            intensity: 0.03,
            pad_impulse: [0.01, -0.01, 0.0],
            duration_secs: 60.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::Spontaneous,
                signal: "noise".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: true,
            residual_intensity: 0.03,
        };
        let result = engine.from_pulse(&pulse);
        assert!(result.is_none());
    }

    #[test]
    fn test_residue_combined_effect() {
        let mut engine = ResidueEngine::default();
        let pulse = ChaoticPulse {
            id: 1,
            kind: PulseKind::JoyBurst,
            intensity: 0.9,
            pad_impulse: [0.5, 0.5, 0.1],
            duration_secs: 300.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "good".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 1000,
            absorbed: true,
            residual_intensity: 0.9,
        };
        engine.from_pulse(&pulse);
        let effect = engine.combined_effect(1000);
        assert!(effect.active_count > 0);
        assert_eq!(effect.dominant_residue, Some(ResidueKind::Afterglow));
        // Afterglow PAD偏移：P>0
        assert!(effect.pad_offset[0] > 0.0);
    }

    #[test]
    fn test_residue_tick_decay() {
        let mut engine = ResidueEngine::default();
        let pulse = ChaoticPulse {
            id: 1,
            kind: PulseKind::AngerFlash,
            intensity: 0.7,
            pad_impulse: [-0.4, 0.4, 0.2],
            duration_secs: 300.0,
            decay_curve: DecayCurve::slow_power_law(),
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "provoked".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: 0,
            absorbed: true,
            residual_intensity: 0.7,
        };
        engine.from_pulse(&pulse);
        assert!(!engine.active_residues.is_empty());
        // 大量时间后残留应衰减消失
        engine.tick(10000000);
        assert!(engine.active_residues.is_empty());
    }

    // ── ContagionEngine 测试 ──

    #[test]
    fn test_contagion_default_rules() {
        let engine = ContagionEngine::default();
        assert_eq!(engine.rules.len(), 12);
    }

    #[test]
    fn test_contagion_evaluate_with_anger() {
        let mut engine = ContagionEngine::default();
        let profile = EmotionProfile {
            anger: 0.8,
            sadness: 0.0,
            anxiety: 0.0,
            fear: 0.0,
            joy: 0.0,
            calm: 0.0,
            guilt: 0.0,
            shame: 0.0,
            pride: 0.0,
            envy: 0.0,
            gratitude: 0.0,
            nostalgia: 0.0,
        };
        // 注入确定性 RNG，确保可复现 / Inject deterministic RNG for reproducibility
        let mut rng = SmallRng::seed_from_u64(42);
        let result = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::Any,
            1000,
            &mut rng,
        );
        // 概率性，不保证一定触发，但规则检查应通过
        // 主要验证不 panic
        assert!(result.len() <= engine.rules.len());
    }

    // ── ChaosEngine 测试 ──

    #[test]
    fn test_chaos_attractor_calm() {
        let mut engine = ChaosEngine::default();
        // 填充平静区域轨迹
        for i in 0..20 {
            engine.record(&[0.1, -0.05, 0.0], 1000 + i * 60);
        }
        let attractor = engine.detect_attractor();
        assert_eq!(attractor, StrangeAttractor::CalmBasin);
    }

    #[test]
    fn test_chaos_attractor_anxiety() {
        let mut engine = ChaosEngine::default();
        for i in 0..20 {
            engine.record(&[-0.2, 0.3, -0.2], 1000 + i * 60);
        }
        let attractor = engine.detect_attractor();
        assert_eq!(attractor, StrangeAttractor::AnxietyBasin);
    }

    #[test]
    fn test_chaos_bifurcation() {
        let mut engine = ChaosEngine::default();
        // 前半段平静，后半段焦虑 → 分岔
        for i in 0..10 {
            engine.record(&[0.2, -0.1, 0.0], 1000 + i * 60);
        }
        for i in 10..20 {
            engine.record(&[-0.3, 0.4, -0.2], 1000 + i * 60);
        }
        let pattern = engine.detect_bifurcation(2200);
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().kind, EmergentKind::Bifurcation);
    }

    // ── EmotionProfile 测试 ──

    #[test]
    fn test_emotion_profile_from_pad_anger() {
        let profile = EmotionProfile::from_pad(&[-0.5, 0.7, 0.3]);
        assert!(profile.anger > 0.1);
        assert!(profile.sadness < profile.anger);
    }

    #[test]
    fn test_emotion_profile_from_pad_joy() {
        let profile = EmotionProfile::from_pad(&[0.6, 0.5, 0.2]);
        assert!(profile.joy > 0.1);
        assert!(profile.anger < 0.01);
    }

    #[test]
    fn test_emotion_profile_from_pad_calm() {
        let profile = EmotionProfile::from_pad(&[0.5, -0.3, 0.1]);
        assert!(profile.calm > 0.1);
    }

    // ── BodyMemory 测试 ──

    #[test]
    fn test_body_memory_neutral() {
        let bm = BodyMemory::neutral();
        assert!((bm.tension - 0.0).abs() < 1e-6);
        assert!((bm.warmth - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_body_memory_from_residue() {
        let bm = BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 1.0);
        assert!(bm.tension > 0.5);
        assert!(bm.warmth < 0.0);
    }

    #[test]
    fn test_body_memory_combine() {
        let bm1 = BodyMemory {
            breath_offset: 0.1,
            tension: 0.2,
            heaviness: 0.1,
            warmth: 0.0,
        };
        let bm2 = BodyMemory {
            breath_offset: 0.2,
            tension: 0.3,
            heaviness: 0.0,
            warmth: 0.5,
        };
        let combined = bm1.combine(&bm2, 0.5);
        assert!((combined.tension - 0.35).abs() < 1e-6);
    }

    // ── ResidueKind 测试 ──

    #[test]
    fn test_residue_half_lives() {
        assert_eq!(ResidueKind::Tension.default_half_life_secs(), 1800.0);
        assert_eq!(
            ResidueKind::LingeringSadness.default_half_life_secs(),
            7200.0
        );
        assert_eq!(
            ResidueKind::IntimacyDeepening.default_half_life_secs(),
            f64::MAX
        );
    }

    #[test]
    fn test_residue_pad_offsets() {
        let sad = ResidueKind::LingeringSadness.default_pad_offset();
        assert!(sad[0] < 0.0); // P < 0
        let warm = ResidueKind::WarmthResidue.default_pad_offset();
        assert!(warm[0] > 0.0); // P > 0
    }

    // ── IrrationalityManager 集成测试 ──

    #[test]
    fn test_manager_on_emotion_change() {
        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "shocking_news".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let correction = mgr.on_emotion_change(
            &[0.0, 0.0, 0.0],
            &[-0.5, 0.5, -0.3],
            trigger,
            RelationshipDepth::FamiliarOrAbove,
            MaturityDepth::Any,
            1000,
        );
        assert!(correction.active_pulses > 0 || correction.active_residues > 0);
    }

    #[test]
    fn test_manager_prompt_fragment() {
        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        mgr.on_emotion_change(
            &[0.0, 0.0, 0.0],
            &[-0.5, 0.5, -0.3],
            trigger,
            RelationshipDepth::Any,
            MaturityDepth::Any,
            1000,
        );
        let fragment = mgr.to_prompt_fragment(1000);
        assert!(!fragment.is_empty());
        assert!(fragment.contains("[情绪生态]"));
    }

    #[test]
    fn test_manager_tick() {
        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        mgr.on_emotion_change(
            &[0.0, 0.0, 0.0],
            &[-0.5, 0.5, -0.3],
            trigger,
            RelationshipDepth::Any,
            MaturityDepth::Any,
            1000,
        );
        // Tick 应不 panic
        mgr.tick(&[0.0, 0.0, 0.0], 1060);
    }

    #[test]
    fn test_manager_body_memory() {
        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        mgr.on_emotion_change(
            &[0.0, 0.0, 0.0],
            &[-0.5, 0.5, -0.3],
            trigger,
            RelationshipDepth::Any,
            MaturityDepth::Any,
            1000,
        );
        let bm = mgr.body_memory_for_expression(1000);
        // 应返回有效的身体记忆
        assert!(bm.tension.is_finite());
        assert!(bm.warmth.is_finite());
    }

    // ═══════════════════════════════════════════════════════════
    // ═════════════════════════════════════════════════════════════
    // Phase E: 15 enhancement tests
    // ═════════════════════════════════════════════════════════════

    /// Helper: push a residue directly into the engine for testing
    fn add_test_residue(engine: &mut ResidueEngine, kind: ResidueKind, intensity: f64, ts: i64) {
        let id = engine.next_id;
        engine.next_id += 1;
        engine.active_residues.push(EmotionResidue {
            id,
            kind,
            intensity,
            pad_offset: [0.0, 0.0, 0.0],
            half_life_secs: 600.0,
            created_at: ts,
            updated_at: ts,
            source_pulse_id: None,
            body_memory: BodyMemory::neutral(),
            expressed: false,
        });
    }

    #[test]
    fn test_body_memory_decay() {
        let mut bm = BodyMemory {
            breath_offset: 0.8,
            tension: 0.6,
            heaviness: 0.4,
            warmth: 0.2,
        };
        bm.decay(0.5);
        assert!((bm.breath_offset - 0.4).abs() < 1e-9);
        assert!((bm.tension - 0.3).abs() < 1e-9);
        assert!((bm.heaviness - 0.2).abs() < 1e-9);
        assert!((bm.warmth - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_body_memory_normalize() {
        let mut bm = BodyMemory {
            breath_offset: 1.5,
            tension: -2.0,
            heaviness: 0.5,
            warmth: -0.3,
        };
        bm.normalize();
        assert!((bm.breath_offset - 1.0).abs() < 1e-9);
        assert!((bm.tension - (-1.0)).abs() < 1e-9);
        assert!((bm.heaviness - 0.5).abs() < 1e-9);
        assert!((bm.warmth - (-0.3)).abs() < 1e-9);
    }

    #[test]
    fn test_body_memory_dominant_channel() {
        let bm = BodyMemory {
            breath_offset: 0.1,
            tension: 0.9,
            heaviness: 0.3,
            warmth: 0.2,
        };
        assert_eq!(bm.dominant_channel(), "tension");

        let bm2 = BodyMemory {
            breath_offset: 0.8,
            tension: 0.1,
            heaviness: 0.2,
            warmth: 0.3,
        };
        assert_eq!(bm2.dominant_channel(), "breath");
    }

    #[test]
    fn test_body_memory_magnitude() {
        let bm = BodyMemory {
            breath_offset: 1.0,
            tension: 1.0,
            heaviness: 0.0,
            warmth: 0.0,
        };
        assert!((bm.magnitude() - 2.0_f64.sqrt()).abs() < 1e-9);

        let neutral = BodyMemory::neutral();
        assert!(neutral.magnitude().abs() < 1e-9);
    }

    #[test]
    fn test_body_memory_to_prompt_hint() {
        let bm_tense = BodyMemory {
            breath_offset: 0.0,
            tension: 0.5,
            heaviness: 0.0,
            warmth: 0.0,
        };
        let hint = bm_tense.to_prompt_hint();
        assert!(hint.contains("紧张"), "hint={}", hint);

        let bm_calm = BodyMemory::neutral();
        let hint_calm = bm_calm.to_prompt_hint();
        assert!(hint_calm.contains("平静"), "hint_calm={}", hint_calm);
    }

    #[test]
    fn test_residue_merge_same_kind() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Tension, 0.4, 100);
        add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 200);
        assert_eq!(engine.active_residues.len(), 2);
        engine.merge_same_kind();
        assert_eq!(engine.active_residues.len(), 1);
        let merged = &engine.active_residues[0];
        assert!(
            (merged.intensity - 0.49).abs() < 1e-9,
            "merged intensity={}",
            merged.intensity
        );
        assert_eq!(merged.kind, ResidueKind::Tension);
    }

    #[test]
    fn test_residue_mark_expressed() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 100);
        let residue_id = engine.active_residues[0].id;
        assert!(!engine.active_residues[0].expressed);
        let found = engine.mark_expressed(residue_id);
        assert!(found);
        assert!(engine.active_residues[0].expressed);
        let not_found = engine.mark_expressed(99999);
        assert!(!not_found);
    }

    #[test]
    fn test_residue_strongest() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 100);
        add_test_residue(&mut engine, ResidueKind::Afterglow, 0.7, 200);
        add_test_residue(&mut engine, ResidueKind::LingeringSadness, 0.5, 300);
        let strongest = engine.strongest_residue().unwrap();
        assert!((strongest.intensity - 0.7).abs() < 1e-9);
        assert_eq!(strongest.kind, ResidueKind::Afterglow);
    }

    #[test]
    fn test_residue_total_intensity() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 100);
        add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 200);
        let total = engine.total_intensity();
        assert!((total - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_residue_interaction_amplify() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
        add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200);
        let factor = engine.residue_interaction_factor();
        assert!((factor - 1.15).abs() < 1e-9, "factor={}", factor);
    }

    #[test]
    fn test_residue_interaction_dampen() {
        let mut engine = ResidueEngine::new(ResidueConfig::default());
        add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 100);
        add_test_residue(&mut engine, ResidueKind::Tension, 0.4, 200);
        let factor = engine.residue_interaction_factor();
        assert!((factor - 0.85).abs() < 1e-9, "factor={}", factor);
    }

    #[test]
    fn test_contagion_deterministic_anger_to_sadness() {
        // 确定性 RNG：固定种子确保可复现 / Deterministic RNG: fixed seed for reproducibility
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        let profile = EmotionProfile {
            anger: 0.8,
            ..Default::default()
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let triggered = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng,
        );
        // 确定性种子下应触发传染 / Deterministic seed should trigger contagion
        assert!(
            !triggered.is_empty(),
            "should trigger at least one contagion with deterministic seed"
        );
        let has_sadness = triggered
            .iter()
            .any(|c| c.target_emotion == ContagionEmotion::Sadness);
        assert!(has_sadness, "should have Anger->Sadness contagion");
    }

    #[test]
    fn test_contagion_cooldown_prevents_retrigger() {
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        let profile = EmotionProfile {
            anger: 0.9,
            ..Default::default()
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let first = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng,
        );
        assert!(!first.is_empty());
        let second = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1050,
            &mut rng,
        );
        assert!(second.is_empty(), "cooldown should prevent retrigger");
    }

    #[test]
    fn test_contagion_relationship_depth_filter() {
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        let profile = EmotionProfile {
            anger: 0.9,
            ..Default::default()
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let triggered_any = engine.evaluate(
            &profile,
            RelationshipDepth::Any,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng,
        );
        engine.clear_cooldown();
        let mut rng2 = SmallRng::seed_from_u64(42);
        let triggered_deep = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng2,
        );
        // Anger rules require TrustedOrAbove/DeepOnly, so Any triggers none
        assert!(
            triggered_any.is_empty(),
            "Any depth should not trigger anger contagions (rules require higher depth)"
        );
        // DeepOnly should trigger AngerToSadness (min_relationship_depth=DeepOnly)
        assert!(
            !triggered_deep.is_empty(),
            "DeepOnly should trigger at least one anger contagion"
        );
    }

    #[test]
    fn test_contagion_maturity_filter() {
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        let profile = EmotionProfile {
            joy: 0.9,
            ..Default::default()
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let triggered_any = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::Any,
            1000,
            &mut rng,
        );
        let any_count = triggered_any.len();
        engine.clear_cooldown();
        let mut rng2 = SmallRng::seed_from_u64(42);
        let triggered_mature = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng2,
        );
        assert!(triggered_mature.len() <= any_count + 1);
    }

    #[test]
    fn test_contagion_get_recent_for_emotion() {
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        let profile = EmotionProfile {
            anger: 0.9,
            ..Default::default()
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let _ = engine.evaluate(
            &profile,
            RelationshipDepth::DeepOnly,
            MaturityDepth::MatureOrAbove,
            1000,
            &mut rng,
        );
        let sadness_contagions = engine.get_recent_for_emotion(ContagionEmotion::Sadness);
        assert!(
            !sadness_contagions.is_empty(),
            "should have Anger->Sadness contagion in recent"
        );
        let joy_contagions = engine.get_recent_for_emotion(ContagionEmotion::Joy);
        assert!(joy_contagions.is_empty(), "no Joy contagion should exist");
    }

    // ══════════════════════════════════════════════════════════════
    // C3.1: 延迟传染测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_rule_delay_values() {
        // 验证各规则延迟时间 / Verify delay values for each rule
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::AngerToGuilt),
            30.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::AngerToSadness),
            60.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::SadnessToAnger),
            15.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::AngerSadnessToShame),
            45.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::JoyNostalgiaToGratitude),
            20.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::PrideAnxietyToEnvy),
            90.0
        );
        // 即时传染 / Immediate contagions
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::AnxietyToExcitement),
            0.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::AnxietyContagion),
            0.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::CalmContagion),
            0.0
        );
        assert_eq!(
            ContagionEngine::rule_delay(ContagionRule::JoyContagion),
            0.0
        );
    }

    #[test]
    fn test_pending_contagion_queue() {
        // 延迟传染加入队列 / Delayed contagions added to pending queue
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        engine.clear_cooldown();
        let profile = EmotionProfile {
            anger: 0.8,
            ..Default::default()
        };

        let now = 1000i64;
        let mut rng = SmallRng::seed_from_u64(42);
        let triggered = engine.evaluate(
            &profile,
            RelationshipDepth::TrustedOrAbove,
            MaturityDepth::GrowingOrAbove,
            now,
            &mut rng,
        );

        // 应有传染触发 / Should have triggered contagions
        assert!(!triggered.is_empty(), "应有传染触发");

        // 检查有延迟的传染是否进入 pending 队列 / Check delayed contagions in pending
        let delayed: Vec<_> = triggered.iter().filter(|c| c.delay_secs > 0.0).collect();
        if !delayed.is_empty() {
            assert!(
                engine.pending_count() > 0,
                "有延迟传染时应进入 pending 队列"
            );
        }
    }

    #[test]
    fn test_tick_executes_due_contagions() {
        // tick() 执行到期延迟传染 / tick() executes due pending contagions
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        engine.clear_cooldown();

        // 手动添加延迟传染 / Manually add pending contagion
        // 创建时间 t=500，第一个 t=1000 到期，第二个 t=2000 到期
        // Created at t=500, first due at t=1000, second due at t=2000
        engine.pending.push(PendingContagion {
            rule: ContagionRule::AngerToGuilt,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            strength: 0.5,
            original_strength: 0.5,
            pad_template: [-0.2, -0.3, -0.3],
            trigger_time: 1000,
            created_at: 500,
            contagion_id: 1,
        });
        engine.pending.push(PendingContagion {
            rule: ContagionRule::AngerToSadness,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Sadness,
            strength: 0.3,
            original_strength: 0.3,
            pad_template: [-0.3, -0.2, -0.1],
            trigger_time: 2000, // 未到期 / Not yet due
            created_at: 500,
            contagion_id: 2,
        });

        assert_eq!(engine.pending_count(), 2);

        // 在 t=1500 时，只有第一个到期 / At t=1500, only first is due
        let effects = engine.tick(1500);
        assert_eq!(effects.len(), 1, "应只有1个到期传染");
        assert_eq!(effects[0].target_emotion, ContagionEmotion::Guilt);
        assert_eq!(effects[0].id, 1);

        // pending 队列应只剩1个 / Pending queue should have 1 remaining
        assert_eq!(engine.pending_count(), 1);

        // 在 t=3000 时，第二个也到期 / At t=3000, second is also due
        let effects2 = engine.tick(3000);
        assert_eq!(effects2.len(), 1);
        assert_eq!(effects2[0].target_emotion, ContagionEmotion::Sadness);
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_tick_no_due_contagions() {
        // 无到期传染时返回空 / No due contagions returns empty
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        engine.pending.push(PendingContagion {
            rule: ContagionRule::AngerToGuilt,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            strength: 0.5,
            original_strength: 0.5,
            pad_template: [-0.2, -0.3, -0.3],
            trigger_time: 1000,
            created_at: 0,
            contagion_id: 1,
        });

        let effects = engine.tick(500); // 未到期 / Not yet due
        assert!(effects.is_empty());
        assert_eq!(engine.pending_count(), 1, "未到期传染应保留在队列中");
    }

    #[test]
    fn test_contagion_effect_structure() {
        // ContagionEffect 结构正确 / ContagionEffect structure is correct
        let effect = ContagionEffect {
            id: 42,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            rule: ContagionRule::AngerToGuilt,
            strength: 0.6,
            pad_offset: [-0.2, -0.3, -0.3],
            delay_secs: 30.0,
            triggered_at: 1000,
        };
        assert_eq!(effect.id, 42);
        assert_eq!(effect.source_emotion, ContagionEmotion::Anger);
        assert_eq!(effect.target_emotion, ContagionEmotion::Guilt);
        assert_eq!(effect.rule, ContagionRule::AngerToGuilt);
        assert!((effect.delay_secs - 30.0).abs() < 1e-10);
        assert!((effect.strength - 0.6).abs() < 1e-10);
    }

    // ══════════════════════════════════════════════════════════════
    // C3.2: RandomMode 确定性生产模式测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_random_mode_default_stochastic() {
        // 默认为随机模式 / Default is stochastic mode
        let mode = RandomMode::default();
        assert_eq!(mode, RandomMode::Stochastic);
    }

    #[test]
    fn test_random_mode_deterministic() {
        // 确定性模式 / Deterministic mode
        let mode = RandomMode::Deterministic { seed: 42 };
        assert_eq!(mode, RandomMode::Deterministic { seed: 42 });
        assert_ne!(mode, RandomMode::Stochastic);
    }

    #[test]
    fn test_irrationality_manager_default_stochastic() {
        // 默认管理器使用随机模式 / Default manager uses stochastic mode
        let mgr = IrrationalityManager::default();
        assert_eq!(mgr.random_mode, RandomMode::Stochastic);
    }

    #[test]
    fn test_irrationality_manager_with_deterministic() {
        // 切换到确定性模式 / Switch to deterministic mode
        let mgr = IrrationalityManager::default()
            .with_random_mode(RandomMode::Deterministic { seed: 12345 });
        assert_eq!(mgr.random_mode, RandomMode::Deterministic { seed: 12345 });
    }

    #[test]
    fn test_evaluate_contagion_deterministic_dispatch() {
        // 确定性模式：统一代码路径，通过内置 SmallRng 注入随机源
        // Deterministic mode: unified code path, injects RNG via built-in SmallRng
        let mut mgr = IrrationalityManager::default()
            .with_random_mode(RandomMode::Deterministic { seed: 42 });
        mgr.contagion.clear_cooldown();

        let profile = EmotionProfile {
            anger: 0.8,
            ..Default::default()
        };

        let now = 1000i64;
        let contagions = mgr.evaluate_contagion(
            &profile,
            RelationshipDepth::TrustedOrAbove,
            MaturityDepth::GrowingOrAbove,
            now,
        );
        // 确定性种子下应触发传染 / Deterministic seed should trigger contagion
        assert!(!contagions.is_empty(), "确定性模式满足条件时应触发传染");
    }

    #[test]
    fn test_evaluate_contagion_stochastic_dispatch() {
        // 随机模式：统一代码路径，内置 SmallRng 从熵源初始化
        // Stochastic mode: unified code path, built-in SmallRng from entropy
        let mut mgr = IrrationalityManager::default().with_random_mode(RandomMode::Stochastic);
        mgr.contagion.clear_cooldown();

        let profile = EmotionProfile {
            anger: 0.8,
            ..Default::default()
        };

        let now = 1000i64;
        // 多次调用，至少有一次触发（概率性）/ Multiple calls, at least one should trigger
        let mut any_triggered = false;
        for i in 0..20 {
            mgr.contagion.clear_cooldown();
            let contagions = mgr.evaluate_contagion(
                &profile,
                RelationshipDepth::TrustedOrAbove,
                MaturityDepth::GrowingOrAbove,
                now + i * 1000,
            );
            if !contagions.is_empty() {
                any_triggered = true;
                break;
            }
        }
        assert!(any_triggered, "随机模式在多次调用中应至少触发一次传染");
    }
    // ══════════════════════════════════════════════════════════════
    // C3.1 增强测试：指数衰减 + 传染效果接入 + 提示片段
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_pending_contagion_exponential_decay() {
        // 延迟传染强度随等待时间指数衰减 / Pending contagion strength decays exponentially with wait time
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        // 创建时间 t=0，触发时间 t=100，在 t=100 时执行
        // 等待 100 秒，衰减因子 = e^(-0.05 * 100) ≈ 0.0067
        // Created at t=0, trigger at t=100, executed at t=100
        // Wait 100s, decay factor = e^(-0.05 * 100) ≈ 0.0067
        engine.pending.push(PendingContagion {
            rule: ContagionRule::AngerToGuilt,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            strength: 0.8,
            original_strength: 0.8,
            pad_template: [-0.2, -0.3, -0.3],
            trigger_time: 100,
            created_at: 0,
            contagion_id: 1,
        });

        let effects = engine.tick(100);
        assert_eq!(effects.len(), 1);
        // 100秒等待后，强度应显著衰减 / After 100s wait, strength should be significantly decayed
        let expected_strength = 0.8 * (-0.05_f64 * 100.0_f64).exp();
        assert!(
            (effects[0].strength - expected_strength).abs() < 1e-10,
            "expected {:.6}, got {:.6}",
            expected_strength,
            effects[0].strength
        );
        // 衰减后强度远小于原始 / Decayed strength much less than original
        assert!(
            effects[0].strength < 0.1,
            "强度应衰减到0.1以下，实际: {}",
            effects[0].strength
        );
    }

    #[test]
    fn test_pending_contagion_short_delay_minimal_decay() {
        // 短延迟几乎不衰减 / Short delay has minimal decay
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        // 创建时间 t=0，触发时间 t=5，等待 5 秒
        // 衰减因子 = e^(-0.05 * 5) ≈ 0.778
        engine.pending.push(PendingContagion {
            rule: ContagionRule::SadnessToAnger,
            source_emotion: ContagionEmotion::Sadness,
            target_emotion: ContagionEmotion::Anger,
            strength: 0.6,
            original_strength: 0.6,
            pad_template: [-0.2, 0.4, 0.2],
            trigger_time: 5,
            created_at: 0,
            contagion_id: 1,
        });

        let effects = engine.tick(5);
        assert_eq!(effects.len(), 1);
        // 5秒等待后衰减很小 / After 5s wait, decay is small
        let expected = 0.6 * (-0.05_f64 * 5.0_f64).exp();
        assert!(
            (effects[0].strength - expected).abs() < 1e-10,
            "expected {:.6}, got {:.6}",
            expected,
            effects[0].strength
        );
        assert!(effects[0].strength > 0.4, "短延迟后强度应保留大部分");
    }

    #[test]
    fn test_contagion_effect_diagnostic_fields() {
        // ContagionEffect 包含完整诊断信息 / ContagionEffect contains full diagnostic info
        let mut engine = ContagionEngine::new(ContagionConfig::default());
        engine.pending.push(PendingContagion {
            rule: ContagionRule::PrideAnxietyToEnvy,
            source_emotion: ContagionEmotion::Pride,
            target_emotion: ContagionEmotion::Envy,
            strength: 0.4,
            original_strength: 0.4,
            pad_template: [-0.2, 0.1, -0.2],
            trigger_time: 90,
            created_at: 0,
            contagion_id: 42,
        });

        let effects = engine.tick(90);
        assert_eq!(effects.len(), 1);
        let e = &effects[0];
        // 验证诊断字段 / Verify diagnostic fields
        assert_eq!(e.id, 42);
        assert_eq!(e.source_emotion, ContagionEmotion::Pride);
        assert_eq!(e.target_emotion, ContagionEmotion::Envy);
        assert_eq!(e.rule, ContagionRule::PrideAnxietyToEnvy);
        // delay_secs = trigger_time - created_at = 90 - 0 = 90
        assert!(
            (e.delay_secs - 90.0).abs() < 1e-10,
            "delay_secs should be 90.0, got {}",
            e.delay_secs
        );
        assert_eq!(e.triggered_at, 90);
    }

    #[test]
    fn test_manager_tick_wires_contagion_effects() {
        // IrrationalityManager.tick() 将传染效果接入残留引擎
        // IrrationalityManager.tick() wires contagion effects into residue engine
        let mut mgr = IrrationalityManager::default();
        // 手动添加延迟传染 / Manually add pending contagion
        mgr.contagion.pending.push(PendingContagion {
            rule: ContagionRule::AngerToGuilt,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            strength: 0.5,
            original_strength: 0.5,
            pad_template: [-0.2, -0.3, -0.3],
            trigger_time: 1000,
            created_at: 970, // 30秒延迟 / 30s delay
            contagion_id: 1,
        });

        let residue_count_before = mgr.residue.active_residues.len();
        // tick 应执行到期传染并注入残留 / tick should execute due contagion and inject residue
        mgr.tick(&[0.0, 0.0, 0.0], 1000);
        let residue_count_after = mgr.residue.active_residues.len();
        // 传染效果应产生新残留 / Contagion effect should produce new residue
        assert!(
            residue_count_after > residue_count_before,
            "传染效果应注入残留: before={}, after={}",
            residue_count_before,
            residue_count_after
        );
    }

    #[test]
    fn test_prompt_fragment_with_pending_contagion() {
        // 提示片段包含延迟传染信息 / Prompt fragment includes pending contagion info
        let mut mgr = IrrationalityManager::default();
        mgr.contagion.pending.push(PendingContagion {
            rule: ContagionRule::AngerToGuilt,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            strength: 0.5,
            original_strength: 0.5,
            pad_template: [-0.2, -0.3, -0.3],
            trigger_time: 2000,
            created_at: 1970,
            contagion_id: 1,
        });

        let fragment = mgr.to_prompt_fragment(1000);
        assert!(
            fragment.contains("[延迟传染]"),
            "提示片段应包含延迟传染信息: {}",
            fragment
        );
        assert!(
            fragment.contains("愤怒→内疚"),
            "提示片段应包含传染规则描述: {}",
            fragment
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // G1-G5 增强方法测试 / G1-G5 Enhancement Method Tests
    // ═══════════════════════════════════════════════════════════════════════════

    // ── G1: 情绪健康报告测试 / Emotional Health Report Tests ──

    #[test]
    fn test_health_report_calm_basin() {
        let mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        let report = mgr.health_report(now);
        // 初始状态：平静吸引子，无残留 → 高健康分
        assert!(
            report.overall_score > 0.8,
            "初始健康分应>0.8，实际: {}",
            report.overall_score
        );
        assert!(matches!(report.attractor, StrangeAttractor::CalmBasin));
        assert!(report.imbalance_warning.is_none(), "初始状态不应有失衡警告");
    }

    #[test]
    fn test_health_report_with_negative_residues() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加6个负向残留 / Add 6 negative residues
        for _ in 0..6 {
            mgr.residue.active_residues.push(EmotionResidue {
                id: mgr.residue.next_id,
                kind: ResidueKind::SmolderingAnger,
                intensity: 0.5,
                pad_offset: [-0.3, 0.2, 0.0],
                half_life_secs: 1800.0,
                created_at: now,
                updated_at: now,
                source_pulse_id: None,
                body_memory: BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 0.5),
                expressed: false,
            });
            mgr.residue.next_id += 1;
        }
        let report = mgr.health_report(now);
        assert!(matches!(
            report.dominant_valence,
            EmotionalValence::Negative
        ));
        assert_eq!(report.negative_residue_count, 6);
        assert!(
            report.imbalance_warning.is_some(),
            "6个负向残留应触发失衡警告"
        );
    }

    #[test]
    fn test_health_report_with_positive_residues() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加4个正向残留 / Add 4 positive residues
        for kind in [
            ResidueKind::Afterglow,
            ResidueKind::WarmthResidue,
            ResidueKind::IntimacyDeepening,
            ResidueKind::AccomplishmentResidue,
        ] {
            mgr.residue.active_residues.push(EmotionResidue {
                id: mgr.residue.next_id,
                kind,
                intensity: 0.5,
                pad_offset: [0.2, 0.1, 0.0],
                half_life_secs: 3600.0,
                created_at: now,
                updated_at: now,
                source_pulse_id: None,
                body_memory: BodyMemory::from_residue_kind(kind, 0.5),
                expressed: false,
            });
            mgr.residue.next_id += 1;
        }
        let report = mgr.health_report(now);
        assert!(matches!(
            report.dominant_valence,
            EmotionalValence::Positive
        ));
        assert_eq!(report.positive_residue_count, 4);
    }

    // ── G2: 传染因果追溯测试 / Contagion Causal Tracing Tests ──

    #[test]
    fn test_contagion_chain_empty() {
        let mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
        assert!(chain.is_none(), "无传染记录时应返回None");
    }

    #[test]
    fn test_contagion_chain_single() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加一条传染记录：愤怒→内疚 / Add one contagion: Anger→Guilt
        mgr.contagion.recent_contagions.push(CrossContagion {
            id: 1,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            rule: ContagionRule::AngerToGuilt,
            strength: 0.8,
            delay_secs: 0.0,
            condition: ContagionCondition {
                min_source_intensity: 0.3,
                min_relationship_depth: RelationshipDepth::Any,
                min_maturity: MaturityDepth::Any,
                probability: 0.5,
            },
            timestamp: now,
        });
        let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.nodes.len(), 1);
        assert_eq!(chain.nodes[0].source, ContagionEmotion::Anger);
        assert_eq!(chain.nodes[0].target, ContagionEmotion::Guilt);
    }

    #[test]
    fn test_contagion_chain_multi_hop() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 悲伤→愤怒→内疚 / Sadness→Anger→Guilt
        mgr.contagion.recent_contagions.push(CrossContagion {
            id: 1,
            source_emotion: ContagionEmotion::Sadness,
            target_emotion: ContagionEmotion::Anger,
            rule: ContagionRule::SadnessToAnger,
            strength: 0.6,
            delay_secs: 0.0,
            condition: ContagionCondition {
                min_source_intensity: 0.3,
                min_relationship_depth: RelationshipDepth::Any,
                min_maturity: MaturityDepth::Any,
                probability: 0.5,
            },
            timestamp: now - 10,
        });
        mgr.contagion.recent_contagions.push(CrossContagion {
            id: 2,
            source_emotion: ContagionEmotion::Anger,
            target_emotion: ContagionEmotion::Guilt,
            rule: ContagionRule::AngerToGuilt,
            strength: 0.8,
            delay_secs: 0.0,
            condition: ContagionCondition {
                min_source_intensity: 0.3,
                min_relationship_depth: RelationshipDepth::Any,
                min_maturity: MaturityDepth::Any,
                probability: 0.5,
            },
            timestamp: now,
        });
        let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.nodes.len(), 2, "应回溯2跳");
        // 源头在前 / Source first
        assert_eq!(chain.nodes[0].source, ContagionEmotion::Sadness);
        assert_eq!(chain.nodes[0].target, ContagionEmotion::Anger);
        assert_eq!(chain.nodes[1].source, ContagionEmotion::Anger);
        assert_eq!(chain.nodes[1].target, ContagionEmotion::Guilt);
    }

    // ── G3: 残留-身体双向信号测试 / Residue-Body Bidirectional Signal Tests ──

    #[test]
    fn test_residue_body_signal_neutral() {
        let mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        let signal = mgr.residue_body_signal(now);
        // 初始状态：无身体紧张→无催生残留 / Initial: no tension → no bred residue
        assert!(signal.body_born_residue.is_none());
        assert_eq!(signal.body_born_strength, 0.0);
    }

    #[test]
    fn test_residue_body_signal_high_tension() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加高紧张残留 / Add high-tension residue
        mgr.residue.active_residues.push(EmotionResidue {
            id: 1,
            kind: ResidueKind::Tension,
            intensity: 0.8,
            pad_offset: [0.0, 0.3, 0.0],
            half_life_secs: 1800.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory {
                breath_offset: 0.1,
                tension: 0.8,
                heaviness: 0.0,
                warmth: 0.0,
            },
            expressed: false,
        });
        let signal = mgr.residue_body_signal(now);
        assert!(signal.body_born_residue.is_some(), "高紧张应催生残留");
        assert!(signal.body_born_strength > 0.0, "催生强度应>0");
    }

    #[test]
    fn test_apply_residue_body_signal() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        let before_count = mgr.residue.active_residues.len();
        // 添加高温暖残留触发身体→残留通道 / Add high-warmth residue to trigger body→residue
        mgr.residue.active_residues.push(EmotionResidue {
            id: 1,
            kind: ResidueKind::WarmthResidue,
            intensity: 0.8,
            pad_offset: [0.3, 0.1, 0.0],
            half_life_secs: 3600.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory {
                breath_offset: 0.0,
                tension: 0.0,
                heaviness: 0.0,
                warmth: 0.8,
            },
            expressed: false,
        });
        mgr.apply_residue_body_signal(now);
        // 高温暖应催生WarmthResidue / High warmth should breed WarmthResidue
        assert!(
            mgr.residue.active_residues.len() > before_count + 1,
            "应新增身体催生的残留"
        );
    }

    // ── G4: 脉冲-残留交互测试 / Pulse-Residue Interaction Tests ──

    #[test]
    fn test_pulse_residue_interaction_no_overlap() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加喜悦脉冲和悲伤残留（对立→抑制）/ Add joy pulse + sadness residue (opposite → suppress)
        mgr.pulse.active_pulses.push(ChaoticPulse {
            id: 1,
            kind: PulseKind::JoyBurst,
            intensity: 0.8,
            pad_impulse: [0.5, 0.5, 0.3],
            duration_secs: 30.0,
            decay_curve: DecayCurve::Exponential { lambda: 0.1 },
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "joy".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: now,
            absorbed: false,
            residual_intensity: 0.0,
        });
        mgr.residue.active_residues.push(EmotionResidue {
            id: 1,
            kind: ResidueKind::LingeringSadness,
            intensity: 0.6,
            pad_offset: [-0.3, 0.1, 0.0],
            half_life_secs: 3600.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory::from_residue_kind(ResidueKind::LingeringSadness, 0.6),
            expressed: false,
        });
        let interaction = mgr.pulse_residue_interaction();
        // 喜悦应抑制悲伤 / Joy should suppress sadness
        assert!(!interaction.suppressed.is_empty(), "喜悦脉冲应抑制悲伤残留");
    }

    #[test]
    fn test_pulse_residue_interaction_resonance() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let now = 1000;
        // 添加愤怒脉冲和余怒残留（同类→放大）/ Add anger pulse + smoldering anger (same-kind → amplify)
        mgr.pulse.active_pulses.push(ChaoticPulse {
            id: 1,
            kind: PulseKind::AngerFlash,
            intensity: 0.7,
            pad_impulse: [-0.5, 0.6, 0.2],
            duration_secs: 20.0,
            decay_curve: DecayCurve::Exponential { lambda: 0.1 },
            trigger: PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "anger".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: now,
            absorbed: false,
            residual_intensity: 0.0,
        });
        mgr.residue.active_residues.push(EmotionResidue {
            id: 1,
            kind: ResidueKind::SmolderingAnger,
            intensity: 0.5,
            pad_offset: [-0.3, 0.2, 0.0],
            half_life_secs: 1800.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 0.5),
            expressed: false,
        });
        let interaction = mgr.pulse_residue_interaction();
        // 愤怒应放大余怒 / Anger should amplify smoldering anger
        assert!(!interaction.amplified.is_empty(), "愤怒脉冲应放大余怒残留");
        assert!(interaction.amplified[0].1 > 1.0, "放大因子应>1.0");
    }

    // ── G5: 涌现-传染联动测试 / Emergence-Contagion Linkage Tests ──

    #[test]
    fn test_emergence_contagion_link_empty() {
        let mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let links = mgr.emergence_contagion_link();
        assert!(links.is_empty(), "无涌现模式时应返回空");
    }

    #[test]
    fn test_emergence_contagion_link_bifurcation() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        // 添加分岔点涌现 / Add bifurcation emergence
        mgr.chaos
            .state
            .emergent_patterns
            .push_back(EmergentPattern {
                kind: EmergentKind::Bifurcation,
                strength: 0.8,
                detected_at: 1000,
                description: "test bifurcation".to_string(),
            });
        let links = mgr.emergence_contagion_link();
        assert_eq!(links.len(), 1);
        assert!(links[0].threshold_modulation < 1.0, "分岔点应降低传染阈值");
        assert!(!links[0].modulated_rules.is_empty(), "分岔点应调制传染规则");
    }

    #[test]
    fn test_contagion_threshold_modulation_no_emergence() {
        let mgr = IrrationalityManager::new(IrrationalityConfig::default());
        let mod_factor = mgr.contagion_threshold_modulation();
        assert!((mod_factor - 1.0).abs() < 1e-6, "无涌现时调制因子应为1.0");
    }

    #[test]
    fn test_contagion_threshold_modulation_with_bifurcation() {
        let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
        mgr.chaos
            .state
            .emergent_patterns
            .push_back(EmergentPattern {
                kind: EmergentKind::Bifurcation,
                strength: 0.8,
                detected_at: 1000,
                description: "test".to_string(),
            });
        let mod_factor = mgr.contagion_threshold_modulation();
        assert!(
            mod_factor < 1.0,
            "分岔点应降低调制因子(更易传染)，实际: {}",
            mod_factor
        );
    }
}
