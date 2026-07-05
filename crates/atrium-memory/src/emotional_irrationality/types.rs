// ! 情绪非理性系统 — 数据结构 / Emotional Irrationality System — Data Structures
// !
// ! 情绪非理性系统的全部类型定义：脉冲、残留、传染、混沌、涌现

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
///
/// `#[repr(u8)]` 确保判别值可安全转换为 `usize` 用于数组索引。
/// `#[repr(u8)]` ensures discriminant can be safely cast to `usize` for array indexing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u8)]
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
