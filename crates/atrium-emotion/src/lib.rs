// SPDX-License-Identifier: MIT
//! 情感引擎 — PAD 三维模型 + OU 漂移 + 昼夜节律 + 情感惯性 + 22 种复合情绪
//! EmotionEngine — PAD 3D model + OU drift + circadian rhythm + emotional inertia + 22 compound emotions.
//!
//! 让情感引擎在空闲时也有自然波动，不再是"没消息就归零"的死板状态。
//! Natural idle fluctuations so emotion never "resets to zero" when idle.

use std::collections::{HashMap, VecDeque};

use chrono::Local;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 情感标签（9 种基本情绪）/ Emotion Labels (9 basic emotions)
// ════════════════════════════════════════════════════════════════════

/// 9 种基本情绪的 PAD 中心点（Pleasure, Arousal, Dominance）
/// PAD centroids for 9 basic emotions (Pleasure, Arousal, Dominance).
///
/// 基于 Mehrabian & Russell 情绪维度理论。
/// Based on the Mehrabian & Russell emotional dimension theory.
#[derive(Clone, Copy, Debug)]
pub struct EmotionLabel {
    pub name: &'static str,
    pub emoji: &'static str,
    pub pad: (f32, f32, f32),
}

pub const EMOTION_LABELS: [EmotionLabel; 9] = [
    EmotionLabel {
        name: "愉悦",
        emoji: "😊",
        pad: (0.70, 0.50, 0.40),
    },
    EmotionLabel {
        name: "兴奋",
        emoji: "🤩",
        pad: (0.60, 0.85, 0.50),
    },
    EmotionLabel {
        name: "放松",
        emoji: "😌",
        pad: (0.50, -0.30, 0.20),
    },
    EmotionLabel {
        name: "悲伤",
        emoji: "😢",
        pad: (-0.70, -0.30, -0.50),
    },
    EmotionLabel {
        name: "愤怒",
        emoji: "😠",
        pad: (-0.60, 0.70, 0.60),
    },
    EmotionLabel {
        name: "恐惧",
        emoji: "😨",
        pad: (-0.70, 0.60, -0.70),
    },
    EmotionLabel {
        name: "惊讶",
        emoji: "😲",
        pad: (0.20, 0.75, -0.30),
    },
    EmotionLabel {
        name: "厌恶",
        emoji: "🤢",
        pad: (-0.50, 0.20, 0.10),
    },
    EmotionLabel {
        name: "平静",
        emoji: "😐",
        pad: (0.10, -0.50, -0.10),
    },
];

// ════════════════════════════════════════════════════════════════════
// EmotionState — PAD 三维情感状态 / PAD 3D Emotion State
// ════════════════════════════════════════════════════════════════════

/// PAD 状态 — 三维情感坐标（Pleasure, Arousal, Dominance）
/// PAD State — 3D emotion coordinates (Pleasure, Arousal, Dominance).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionState {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

impl EmotionState {
    /// 创建 PAD 状态
    /// Create a new PAD state.
    ///
    /// @param pleasure 愉悦度 [-1, 1] / Pleasure [-1, 1]
    /// @param arousal 唤醒度 [-1, 1] / Arousal [-1, 1]
    /// @param dominance 支配度 [-1, 1] / Dominance [-1, 1]
    /// @return EmotionState 实例 / EmotionState instance
    pub fn new(pleasure: f32, arousal: f32, dominance: f32) -> Self {
        Self {
            pleasure,
            arousal,
            dominance,
        }
    }

    /// 向默认状态衰减（线性插值）
    /// Decay toward default state (linear interpolation).
    ///
    /// @param rate 衰减率 [0, 1] / Decay rate [0, 1]
    /// @param default 目标默认状态 / Target default state
    pub fn decay(&mut self, rate: f32, default: &EmotionState) {
        self.pleasure += (default.pleasure - self.pleasure) * rate;
        self.arousal += (default.arousal - self.arousal) * rate;
        self.dominance += (default.dominance - self.dominance) * rate;
        self.clamp();
    }

    fn clamp(&mut self) {
        self.pleasure = self.pleasure.clamp(-1.0, 1.0);
        self.arousal = self.arousal.clamp(-1.0, 1.0);
        self.dominance = self.dominance.clamp(-1.0, 1.0);
    }

    /// PAD → 9 种基本情绪分类（欧氏距离最近邻）
    /// PAD → 9-class emotion classification (nearest Euclidean neighbor).
    ///
    /// @return 最近的基本情绪标签 / Nearest basic emotion label
    pub fn classify(&self) -> &'static EmotionLabel {
        let mut best_idx = 0usize;
        let mut best_dist = f32::MAX;
        for (i, label) in EMOTION_LABELS.iter().enumerate() {
            let dp = self.pleasure - label.pad.0;
            let da = self.arousal - label.pad.1;
            let dd = self.dominance - label.pad.2;
            let dist = dp * dp + da * da + dd * dd;
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        &EMOTION_LABELS[best_idx]
    }
}

// ════════════════════════════════════════════════════════════════════
// DriftParams — Ornstein-Uhlenbeck 随机漂移过程
// ════════════════════════════════════════════════════════════════════

/// Ornstein-Uhlenbeck 过程参数
///
/// dX = mean_reversion * (baseline - X) * dt + volatility * dW
///
/// 让情感在空闲时围绕基线自然波动，而不是静止不动。
/// 均值回归确保不会漂到极端值。
#[derive(Clone, Debug)]
pub struct DriftParams {
    pub volatility: f64,
    pub mean_reversion: f64,
    pub baseline: [f64; 3], // [P, A, D]
}

impl DriftParams {
    pub fn new(volatility: f64, mean_reversion: f64) -> Self {
        Self {
            volatility,
            mean_reversion,
            baseline: [0.0, 0.0, 0.0],
        }
    }

    /// 一步 OU 过程，返回 [ΔP, ΔA, ΔD]
    pub fn step(&self, current: [f64; 3]) -> [f64; 3] {
        let mut rng = rand::thread_rng();
        let mut delta = [0.0f64; 3];
        for i in 0..3 {
            let noise: f64 = rng.gen_range(-1.0..1.0);
            delta[i] =
                self.mean_reversion * (self.baseline[i] - current[i]) + self.volatility * noise;
        }
        delta
    }
}

// ════════════════════════════════════════════════════════════════════
// CircadianModulator — 双峰高斯昼夜节律
// ════════════════════════════════════════════════════════════════════

/// 昼夜节律调制器
///
/// 两个高斯峰（默认 10:00 和 18:00），夜间低谷。
/// 为 PAD 三维度提供基于当前小时的微调偏移。
#[derive(Clone, Debug)]
pub struct CircadianModulator {
    pub morning_peak: f32,
    pub evening_peak: f32,
    pub morning_sigma: f32,
    pub evening_sigma: f32,
    pub intensity: f32,
    pub timezone_offset: i32,
    pub active_hours: (u32, u32),
}

impl Default for CircadianModulator {
    fn default() -> Self {
        Self {
            morning_peak: 10.0,
            evening_peak: 18.0,
            morning_sigma: 2.0,
            evening_sigma: 2.5,
            intensity: 0.8,
            timezone_offset: 8,
            active_hours: (7, 23),
        }
    }
}

impl CircadianModulator {
    /// 计算当前小时的 PAD 偏移量
    pub fn rhythm_offset(&self, hour: u32) -> [f32; 3] {
        let h = hour as f32;

        // 双峰高斯：上午 + 傍晚
        let morning = gaussian(h, self.morning_peak, self.morning_sigma);
        let evening = gaussian(h, self.evening_peak, self.evening_sigma);
        let combined = (morning.max(evening)) * self.intensity;

        // 夜间（活跃时段外）：低唤醒、微负情绪
        if hour < self.active_hours.0 || hour >= self.active_hours.1 {
            return [
                -0.05 * self.intensity,
                -0.1 * self.intensity,
                -0.02 * self.intensity,
            ];
        }

        // P = combined（高峰更正，低谷更负）
        // A = 正偏移（高能量时段唤醒度上升）
        // D = 轻微正偏移（高能量时段掌控感上升）
        let p = (combined - 0.3) * 0.1;
        let a = combined * 0.15;
        let d = combined * 0.05;

        [p, a, d]
    }

    /// 获取当前本地小时
    pub fn current_hour(&self) -> u32 {
        let now = Local::now();
        ((now.hour() as i32 + self.timezone_offset).rem_euclid(24)) as u32
    }
}

fn gaussian(x: f32, mu: f32, sigma: f32) -> f32 {
    let d = x - mu;
    (-(d * d) / (2.0 * sigma * sigma)).exp()
}

// chrono 的 Timelike trait 用于 hour()
use chrono::Timelike;

// ════════════════════════════════════════════════════════════════════
// EmotionalInertia — 情感惯性系统
// ════════════════════════════════════════════════════════════════════

/// 情感惯性修正器
///
/// 长期处于某种情绪后，自动调整：
/// - 敏感度（影响 affect 的强度）
/// - 衰减率（情绪持续更久或更快恢复）
/// - 表达阈值（更容易或更难表达情绪）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InertiaModifiers {
    pub sensitivity: f32,
    pub decay_rate: f32,
    pub expression_threshold: f32,
}

impl Default for InertiaModifiers {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            decay_rate: 1.0,
            expression_threshold: 0.0,
        }
    }
}

/// 情感惯性追踪器
///
/// 追踪持续主导情绪，超过阈值后激活惯性修正。
#[derive(Clone, Debug)]
pub struct EmotionalInertia {
    history: VecDeque<[f32; 3]>,
    capacity: usize,
    activation_ticks: usize,
    dominant_duration: usize,
    dominant_label: Option<String>,
    pub modifiers: InertiaModifiers,
    max_sensitivity: f32,
    min_decay_rate: f32,
}

impl Default for EmotionalInertia {
    fn default() -> Self {
        Self {
            history: VecDeque::new(),
            capacity: 500,        // 500 ticks ≈ 100s @ 200ms/tick
            activation_ticks: 50, // 50 ticks ≈ 10s 激活阈值
            dominant_duration: 0,
            dominant_label: None,
            modifiers: InertiaModifiers::default(),
            max_sensitivity: 1.5,
            min_decay_rate: 0.85,
        }
    }
}

impl EmotionalInertia {
    pub fn new() -> Self {
        Self::default()
    }

    /// 每次 tick 调用，更新历史并重新计算修正器
    pub fn tick(&mut self, pad: [f32; 3]) {
        self.history.push_back(pad);
        if self.history.len() > self.capacity {
            self.history.pop_front();
        }
        self.update_modifiers();
    }

    /// 根据历史记录更新修正器 / Update modifiers based on history.
    ///
    /// 热路径优化：O(A²)→O(A) — 用 HashMap 计频替代嵌套遍历。
    /// Hot-path optimization: O(A²)→O(A) — HashMap frequency counting replaces nested iteration.
    /// 情感惯性是情绪的粘滞记忆——O(A)让粘滞计算不成为每tick的负担。
    /// Emotional inertia is the sticky memory of emotion — O(A) makes sticky computation
    /// not a per-tick burden.
    fn update_modifiers(&mut self) {
        if self.history.len() < self.activation_ticks {
            self.modifiers = InertiaModifiers::default();
            return;
        }

        // 情绪标签计频 / Emotion label frequency counting — O(A) 单次遍历
        let mut freq: HashMap<String, usize> = HashMap::new();
        for pad in self.history.iter().rev().take(self.activation_ticks) {
            let label = EmotionState::new(pad[0], pad[1], pad[2])
                .classify()
                .name
                .to_string();
            *freq.entry(label).or_insert(0) += 1;
        }

        // 众数查找 / Mode finding — O(K), K ≤ 9 种基本情绪
        let (dominant, count) = freq.into_iter().max_by_key(|(_, c)| *c).unwrap_or_default();
        let ratio = count as f32 / self.activation_ticks as f32;

        // 如果超过 60% 的时间都是同一情绪 → 激活惯性
        if ratio > 0.6 {
            self.dominant_duration += 1;

            let factor =
                ((self.dominant_duration as f32 / self.activation_ticks as f32) - 1.0).max(0.0);

            // 敏感度升高（最高 1.5 倍）
            self.modifiers.sensitivity = (1.0 + factor * 0.1).min(self.max_sensitivity);
            // 衰减率降低（情绪持续更久，最低 0.85 倍）
            self.modifiers.decay_rate = (1.0 - factor * 0.05).max(self.min_decay_rate);
            // 表达阈值降低（更容易触发情绪表达）
            self.modifiers.expression_threshold = -(factor * 0.02).max(-0.1);
            self.dominant_label = Some(dominant);
        } else {
            // 情绪多样化 → 惯性重置
            self.dominant_duration = 0;
            self.dominant_label = None;
            self.modifiers = InertiaModifiers::default();
        }
    }

    pub fn dominant_label(&self) -> Option<&str> {
        self.dominant_label.as_deref()
    }

    pub fn modifiers(&self) -> &InertiaModifiers {
        &self.modifiers
    }
}

// ════════════════════════════════════════════════════════════════════
// LongingParams — 想念引擎参数 / Longing engine parameters
// ════════════════════════════════════════════════════════════════════

/// 想念参数 — 用户离开时 PAD 漂移基线从中性渐变到想念基线
/// Longing parameters — PAD drift baseline interpolates from neutral to longing when user is away.
///
/// 当用户离开超过 onset_threshold 后，AI 的情感漂移目标
/// 从 [0,0,0] 渐变到 baseline（轻微悲伤 + 微弱唤醒 + 低掌控感）。
/// 渐变速率受关系深度和用户参与度调制（由 CoreService::longing_tick 设置）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LongingParams {
    /// 想念基线 PAD / Longing PAD baseline (pleasure↓, arousal↑微, dominance↓)
    pub baseline: [f64; 3],
    /// OU 波动率 / OU volatility (想念时的情感微扰)
    pub volatility: f64,
    /// 均值回归率 / Mean reversion rate (向当前基线拉回的速率)
    pub mean_reversion: f64,
    /// 想念起始阈值（秒）/ Onset threshold in seconds
    pub onset_threshold_secs: u64,
    /// 想念饱和阈值（秒）/ Saturation threshold (到达想念基线的最短时长)
    pub saturation_threshold_secs: u64,
}

impl Default for LongingParams {
    fn default() -> Self {
        Self {
            baseline: [-0.25, 0.05, -0.15],
            volatility: 0.001,
            mean_reversion: 0.0005,
            onset_threshold_secs: 600,
            saturation_threshold_secs: 7200,
        }
    }
}

impl LongingParams {
    /// 一步 OU 过程，向指定基线漂移，返回 [ΔP, ΔA, ΔD]
    /// One OU step toward the given baseline, returns [ΔP, ΔA, ΔD].
    ///
    /// @param current 当前 PAD 值 / Current PAD values
    /// @param target  目标基线 / Target baseline
    /// @return [ΔP, ΔA, ΔD] 增量 / Incremental delta
    pub fn step_toward(&self, current: [f64; 3], target: [f64; 3]) -> [f64; 3] {
        let mut rng = rand::thread_rng();
        let mut delta = [0.0f64; 3];
        for i in 0..3 {
            let noise: f64 = rng.gen_range(-1.0..1.0);
            delta[i] = self.mean_reversion * (target[i] - current[i]) + self.volatility * noise;
        }
        delta
    }
}

/// 想念运行时状态 / Longing runtime state
///
/// 由 CoreService::longing_tick() 每 20 tick 更新，
/// 存储当前想念强度、离开时长、插值后的漂移基线。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LongingState {
    /// 当前想念强度 [0, 1] / Current longing intensity
    pub intensity: f32,
    /// 用户离开时长（秒）/ Away duration in seconds
    pub away_secs: u64,
    /// 当前漂移基线（插值结果）/ Current interpolated baseline
    pub current_baseline: [f64; 3],
    /// 上次更新时间戳 / Last update timestamp (unix epoch seconds)
    pub last_update: i64,
}

impl Default for LongingState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            away_secs: 0,
            current_baseline: [0.0, 0.0, 0.0],
            last_update: 0,
        }
    }
}

impl LongingState {
    /// 构造初始想念状态（零强度，基线已知）
    /// Create initial longing state with zero intensity and known baseline.
    pub fn new(baseline: [f64; 3]) -> Self {
        Self {
            intensity: 0.0,
            away_secs: 0,
            current_baseline: baseline,
            last_update: chrono::Utc::now().timestamp(),
        }
    }

    /// 根据离开时长计算想念强度 [0, 1]
    /// Compute longing intensity from away duration.
    ///
    /// @param away_secs 离开秒数 / Away seconds
    /// @param params 想念参数 / Longing parameters
    /// @param rel_mult 关系乘数 (0.8~1.2) / Relationship multiplier
    /// @param engagement 用户参与度 (0~1) / User engagement score
    /// @return 想念强度 [0, 1] / Longing intensity
    pub fn compute_intensity(
        away_secs: u64,
        params: &LongingParams,
        rel_mult: f32,
        engagement: f32,
    ) -> f32 {
        if away_secs <= params.onset_threshold_secs {
            return 0.0;
        }
        if away_secs >= params.saturation_threshold_secs {
            return rel_mult * (0.5 + 0.5 * engagement).clamp(0.0, 1.0);
        }
        let raw = (away_secs - params.onset_threshold_secs) as f32
            / (params.saturation_threshold_secs - params.onset_threshold_secs) as f32;
        raw * rel_mult * (0.5 + 0.5 * engagement).clamp(0.0, 1.0)
    }

    /// 线性插值基线 / Linearly interpolate between neutral and longing baselines.
    ///
    /// @param neutral 中性基线 / Neutral baseline
    /// @param longing 想念基线 / Longing baseline
    /// @param intensity 插值权重 [0, 1] / Interpolation weight
    /// @return 插值后的基线 / Interpolated baseline
    pub fn interpolate_baseline(
        neutral: &[f64; 3],
        longing: &[f64; 3],
        intensity: f32,
    ) -> [f64; 3] {
        let t = intensity as f64;
        [
            neutral[0] * (1.0 - t) + longing[0] * t,
            neutral[1] * (1.0 - t) + longing[1] * t,
            neutral[2] * (1.0 - t) + longing[2] * t,
        ]
    }

    /// 关系阶段门控 — 是否应表达想念 / Relationship stage gate for expressing longing.
    ///
    /// 门控规则：
    /// - Acquaintance (0): 不表达想念 / Never express longing
    /// - Familiar (1): 需更高强度（1.5x 阈值）/ Need higher intensity (1.5x threshold)
    /// - Trusted (2) / Deep (3): 正常阈值 / Normal threshold
    ///
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    ///   (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    /// @param threshold 想念表达阈值 / Longing expression threshold
    /// @return 是否应表达想念 / Whether longing should be expressed
    pub fn should_express_longing(&self, relation_ordinal: u8, threshold: f32) -> bool {
        match relation_ordinal {
            0 => false,                            // 初识不表达想念 / Acquaintance: never express
            1 => self.intensity > threshold * 1.5, // 熟悉需更高强度 / Familiar: need higher
            _ => self.intensity > threshold,       // 信任/深度正常阈值 / Trusted/Deep: normal
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ReunionBurst — 重逢爆发（按离开时长比例表达喜悦）
// ReunionBurst — Reunion burst (joy intensity proportional to away duration)
// ════════════════════════════════════════════════════════════════════

// ── 重逢情境 / Reunion Context ──

/// 重逢情境 / Reunion context
///
/// 不同离别方式决定重逢的情感签名——
/// 吵架后回来是释然，久别后回来是欣喜，仪式时刻回来是温暖。
/// The manner of departure determines the emotional signature of reunion:
/// after conflict = relief, long absence = joy, at ritual = warmth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReunionContext {
    /// 平静离开后回来 / Return after calm departure
    #[default]
    Calm,
    /// 冲突后回来 / Return after conflict
    AfterConflict,
    /// 仪式时刻回来 / Return at ritual time
    AtRitual,
    /// 久别重逢（>7天）/ Long absence reunion (>7 days)
    LongAbsence,
}

impl ReunionContext {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Calm => "平静重逢",
            Self::AfterConflict => "冲突后重逢",
            Self::AtRitual => "仪式重逢",
            Self::LongAbsence => "久别重逢",
        }
    }
}

// ── 关系阶段重逢配置 / Relationship-Stage Reunion Config ──

/// 关系阶段重逢配置 / Relationship-stage reunion configuration
///
/// 不同关系深度的重逢行为不同——
/// 陌生人的回来只是"在的"，恋人的回来是"好想好想你"。
/// Reunion behavior varies by relationship depth:
/// stranger = "I'm here", lover = "Missed you so much".
#[derive(Clone, Debug)]
pub struct RelationshipReunionConfig {
    /// 最低触发关系阶段（ordinal）/ Minimum relationship stage to trigger
    pub min_stage_ordinal: u8,
    /// 此阶段的强度乘数 / Intensity multiplier for this stage
    pub intensity_mult: f64,
    /// 此阶段的 PAD 调制偏移 / PAD modulation offset for this stage
    pub pad_offset: [f32; 3],
    /// 此阶段的用语集 / Phrases for this stage
    pub phrases: Vec<&'static str>,
}

/// 用户回来时，根据离开时长和想念强度生成的喜悦表达。
#[derive(Clone, Debug)]
pub struct ReunionExpression {
    /// 表达强度 [0, 1] / Expression intensity
    pub intensity: f64,
    /// 离开时长（秒）/ Away duration in seconds
    pub away_secs: u64,
    /// 建议用语 / Suggested phrases for expression
    pub suggested_phrases: Vec<&'static str>,
    /// PAD 调制偏移（由关系阶段和情境决定）/ PAD modulation offset
    pub pad_modulation: [f32; 3],
    /// 重逢情境 / Reunion context
    pub context: ReunionContext,
}

/// 重逢爆发 — 按离开时长比例表达喜悦 / Reunion burst — joy proportional to away duration
///
/// 当用户回来时，根据离开时长和想念强度，生成相应强度的喜悦表达。
/// 离开越久、想念越强，重逢越甜。
#[derive(Clone, Debug)]
pub struct ReunionBurst {
    /// 最大表达强度 / Maximum expression intensity
    pub max_intensity: f64,
    /// 离开时长阈值（秒，低于此不触发）/ Min away duration threshold in seconds
    pub min_away_secs: u64,
    /// 饱和时长（秒，超过此强度不再增长）/ Saturation duration in seconds
    pub saturation_secs: u64,
}

impl Default for ReunionBurst {
    fn default() -> Self {
        Self {
            max_intensity: 1.0,
            min_away_secs: 300,     // 5 分钟 / 5 minutes
            saturation_secs: 86400, // 1 天 / 1 day
        }
    }
}

impl ReunionBurst {
    /// 构造自定义重逢爆发 / Create custom ReunionBurst
    pub fn new(max_intensity: f64, min_away_secs: u64, saturation_secs: u64) -> Self {
        Self {
            max_intensity: max_intensity.clamp(0.0, 1.0),
            min_away_secs,
            saturation_secs: saturation_secs.max(1),
        }
    }

    /// 用户回来时调用 / Called when user returns
    ///
    /// @param away_secs 离开时长（秒）/ Away duration in seconds
    /// @param longing_intensity 想念强度 [0, 1] / Longing intensity
    /// @return 重逢表达（None 表示离开时间太短不触发）/ Reunion expression (None = too short to trigger)
    pub fn on_reunion(&self, away_secs: u64, longing_intensity: f64) -> Option<ReunionExpression> {
        if away_secs < self.min_away_secs {
            return None;
        }

        // 强度曲线：sqrt(away / saturation)，自然饱和 / Intensity curve: sqrt ratio
        let ratio = (away_secs as f64 / self.saturation_secs as f64).min(1.0);
        let intensity = ratio.sqrt() * self.max_intensity;

        // 想念强度加成：想念越久，重逢越甜 / Longing bonus: the longer the longing, the sweeter the reunion
        let boosted = (intensity + longing_intensity * 0.3).min(self.max_intensity);

        Some(ReunionExpression {
            intensity: boosted,
            away_secs,
            suggested_phrases: self.match_phrases(boosted),
            pad_modulation: [0.0, 0.0, 0.0],
            context: ReunionContext::Calm,
        })
    }

    /// 根据强度匹配建议用语 / Match suggested phrases based on intensity
    fn match_phrases(&self, intensity: f64) -> Vec<&'static str> {
        if intensity >= 0.8 {
            vec!["你终于回来了！", "好久不见，好想你！"]
        } else if intensity >= 0.5 {
            vec!["欢迎回来~", "你回来啦"]
        } else if intensity >= 0.2 {
            vec!["回来了呀", "嗯，在呢"]
        } else {
            vec!["在的"]
        }
    }

    /// 生成 prompt 片段 / Generate prompt fragment for system prompt injection
    pub fn prompt_fragment(&self, expression: &ReunionExpression) -> String {
        if expression.suggested_phrases.is_empty() {
            return String::new();
        }
        format!(
            "[重逢] 离开{}秒后回来，表达强度{:.2}，情境：{}，建议用语：{}",
            expression.away_secs,
            expression.intensity,
            expression.context.label_zh(),
            expression.suggested_phrases.join(" / ")
        )
    }

    // ── 关系门控重逢 / Relationship-Gated Reunion ──

    /// 关系门控重逢 / Relationship-gated reunion burst
    ///
    /// 不同关系深度的重逢行为不同——
    /// 陌生人/初识：微弱回应"在的"
    /// 熟悉：中等回应"回来了呀"
    /// 信任：较强回应"想你了呢"
    /// 深度：全量回应"好想好想你"
    pub fn on_reunion_gated(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        relationship_ordinal: u8,
    ) -> Option<ReunionExpression> {
        // 基础强度 / Base intensity
        let mut expr = self.on_reunion(away_secs, longing_intensity)?;

        // 关系门控查找 / Relationship gate lookup
        let config = Self::match_relationship_config(relationship_ordinal);

        // 门控：关系阶段不足则不触发 / Gate: insufficient relationship stage
        if relationship_ordinal < config.min_stage_ordinal {
            return None;
        }

        // 关系调制强度 / Relationship-modulated intensity
        expr.intensity = (expr.intensity * config.intensity_mult).min(self.max_intensity);

        // 关系调制 PAD / Relationship-modulated PAD
        expr.pad_modulation = [
            expr.pad_modulation[0] + config.pad_offset[0],
            expr.pad_modulation[1] + config.pad_offset[1],
            expr.pad_modulation[2] + config.pad_offset[2],
        ];

        // 合并用语 / Merge phrases
        let mut phrases = config.phrases.clone();
        if expr.intensity > 0.7 {
            phrases.extend_from_slice(&["好想好想你……你终于回来了！", "好久不见，好想你！"]);
        }
        expr.suggested_phrases = phrases;

        Some(expr)
    }

    /// 匹配关系阶段配置 / Match relationship stage config
    ///
    /// 0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep
    pub fn match_relationship_config(ordinal: u8) -> RelationshipReunionConfig {
        match ordinal {
            // 初识：微弱回应 / Acquaintance: faint response
            0 => RelationshipReunionConfig {
                min_stage_ordinal: 0,
                intensity_mult: 0.2,
                pad_offset: [0.05, 0.02, 0.0],
                phrases: vec!["在的"],
            },
            // 熟悉：中等回应 / Familiar: moderate response
            1 => RelationshipReunionConfig {
                min_stage_ordinal: 1,
                intensity_mult: 0.6,
                pad_offset: [0.15, 0.05, 0.02],
                phrases: vec!["回来了呀", "欢迎回来~"],
            },
            // 信任：较强回应 / Trusted: strong response
            2 => RelationshipReunionConfig {
                min_stage_ordinal: 2,
                intensity_mult: 0.85,
                pad_offset: [0.25, 0.1, 0.05],
                phrases: vec!["你回来啦", "想你了呢"],
            },
            // 深度：全量回应 / Deep: full response
            _ => RelationshipReunionConfig {
                min_stage_ordinal: 3,
                intensity_mult: 1.0,
                pad_offset: [0.35, 0.15, 0.08],
                phrases: vec!["好想好想你……你终于回来了！", "好久不见，好想你！"],
            },
        }
    }

    // ── 情境化重逢 / Contextual Reunion ──

    /// 情境化重逢 / Contextual reunion burst
    ///
    /// 不同离别方式决定重逢的情感签名——
    /// 吵架后回来：释然 + 不安 + 退让（愉悦低、唤醒高、支配低）
    /// 仪式时刻回来：温暖加成（愉悦加成）
    /// 久别重逢：全量强度 + 思念用语
    /// 平静离开：默认行为
    pub fn on_reunion_contextual(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        context: ReunionContext,
    ) -> Option<ReunionExpression> {
        let mut expr = self.on_reunion(away_secs, longing_intensity)?;
        expr.context = context;

        match context {
            ReunionContext::Calm => {
                // 默认行为，PAD 不变 / Default behavior, PAD unchanged
            }
            ReunionContext::AfterConflict => {
                // 冲突后重逢：愉悦降低、唤醒升高、支配降低
                // After conflict: lower pleasure, higher arousal, lower dominance
                // 情感签名：释然 + 不安 + 退让 / Emotional signature: relief + unease + yielding
                expr.intensity *= 0.7;
                expr.pad_modulation = [
                    expr.pad_modulation[0] - 0.1,  // 愉悦降低 / Lower pleasure
                    expr.pad_modulation[1] + 0.15, // 唤醒升高 / Higher arousal
                    expr.pad_modulation[2] - 0.1,  // 支配降低 / Lower dominance
                ];
                expr.suggested_phrases = if expr.intensity > 0.5 {
                    vec!["你回来了……我们聊聊？", "还在生气吗……"]
                } else {
                    vec!["回来了……", "嗯"]
                };
            }
            ReunionContext::AtRitual => {
                // 仪式时刻重逢：愉悦加成 / Ritual reunion: pleasure bonus
                expr.intensity = (expr.intensity * 1.3).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.2,  // 愉悦加成 / Pleasure bonus
                    expr.pad_modulation[1] + 0.05, // 唤醒微升 / Slight arousal
                    expr.pad_modulation[2] + 0.05, // 支配微升 / Slight dominance
                ];
                expr.suggested_phrases = vec!["你刚好在这个时候回来！", "等你好久了~"];
            }
            ReunionContext::LongAbsence => {
                // 久别重逢：全量强度 + 思念用语 / Long absence: full intensity + longing phrases
                expr.intensity = (expr.intensity * 1.2).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.15, // 愉悦加成 / Pleasure bonus
                    expr.pad_modulation[1] + 0.1,  // 唤醒加成 / Arousal bonus
                    expr.pad_modulation[2],        // 支配不变 / Dominance unchanged
                ];
                expr.suggested_phrases = vec!["你终于回来了！", "好久不见，好想你！"];
            }
        }

        Some(expr)
    }

    /// 关系门控 + 情境化重逢（组合）/ Relationship-gated + contextual reunion (combined)
    ///
    /// 先应用关系门控，再叠加情境调制——
    /// 关系深度决定重逢的"量"，离别方式决定重逢的"质"。
    pub fn on_reunion_full(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        relationship_ordinal: u8,
        context: ReunionContext,
    ) -> Option<ReunionExpression> {
        // 先关系门控 / First apply relationship gate
        let mut expr = self.on_reunion_gated(away_secs, longing_intensity, relationship_ordinal)?;

        // 再情境调制 / Then apply context modulation
        expr.context = context;
        match context {
            ReunionContext::Calm => {}
            ReunionContext::AfterConflict => {
                expr.intensity *= 0.7;
                expr.pad_modulation = [
                    expr.pad_modulation[0] - 0.1,
                    expr.pad_modulation[1] + 0.15,
                    expr.pad_modulation[2] - 0.1,
                ];
                if expr.intensity > 0.5 {
                    expr.suggested_phrases = vec!["你回来了……我们聊聊？", "还在生气吗……"];
                } else {
                    expr.suggested_phrases = vec!["回来了……", "嗯"];
                }
            }
            ReunionContext::AtRitual => {
                expr.intensity = (expr.intensity * 1.3).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.2,
                    expr.pad_modulation[1] + 0.05,
                    expr.pad_modulation[2] + 0.05,
                ];
                expr.suggested_phrases = vec!["你刚好在这个时候回来！", "等你好久了~"];
            }
            ReunionContext::LongAbsence => {
                expr.intensity = (expr.intensity * 1.2).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.15,
                    expr.pad_modulation[1] + 0.1,
                    expr.pad_modulation[2],
                ];
                expr.suggested_phrases = vec!["你终于回来了！", "好久不见，好想你！"];
            }
        }

        Some(expr)
    }
}

// ════════════════════════════════════════════════════════════════════
// EmotionEngine — 情感引擎（集成自主循环）
// ════════════════════════════════════════════════════════════════════

pub struct EmotionEngine {
    current: EmotionState,
    default: EmotionState,
    decay_rate: f32,
    drift: Option<DriftParams>,
    circadian: Option<CircadianModulator>,
    inertia: Option<EmotionalInertia>,
    /// 想念引擎参数与运行时状态 / Longing engine parameters and runtime state
    longing: Option<(LongingParams, LongingState)>,
    /// 重逢爆发配置 / Reunion burst configuration
    reunion_burst: Option<ReunionBurst>,
}

impl EmotionEngine {
    pub fn new(default: EmotionState, decay_rate: f32) -> Self {
        Self {
            current: default.clone(),
            default,
            decay_rate,
            drift: None,
            circadian: None,
            inertia: None,
            longing: None,
            reunion_burst: None,
        }
    }

    /// 启用 OU 随机漂移 / Enable OU stochastic drift.
    pub fn with_drift(mut self, params: DriftParams) -> Self {
        self.drift = Some(params);
        self
    }

    /// 启用昼夜节律调制 / Enable circadian rhythm modulation.
    pub fn with_circadian(mut self, circadian: CircadianModulator) -> Self {
        self.circadian = Some(circadian);
        self
    }

    /// 启用情感惯性追踪 / Enable emotional inertia tracking.
    pub fn with_inertia(mut self, inertia: EmotionalInertia) -> Self {
        self.inertia = Some(inertia);
        self
    }

    /// 启用想念引擎 / Enable longing engine.
    ///
    /// 用户离开时 PAD 漂移基线从中性渐变到想念基线，
    /// 由 CoreService::longing_tick() 每 20 tick 更新插值基线。
    pub fn with_longing(mut self, params: LongingParams, state: LongingState) -> Self {
        self.longing = Some((params, state));
        self
    }

    /// 启用重逢爆发 / Enable reunion burst.
    ///
    /// 用户回来时按离开时长比例表达喜悦。
    pub fn with_reunion_burst(mut self, burst: ReunionBurst) -> Self {
        self.reunion_burst = Some(burst);
        self
    }

    /// 触发重逢 / Trigger reunion burst.
    ///
    /// 当检测到用户回来时调用，返回重逢表达结果。
    /// Called when user returns, returns reunion expression result.
    ///
    /// @param away_secs 离开时长（秒）/ Away duration in seconds
    /// @param longing_intensity 想念强度 [0, 1] / Longing intensity
    /// @return 重逢表达（None 表示未启用或离开时间太短）/ Reunion expression (None = disabled or too short)
    pub fn on_reunion(&self, away_secs: u64, longing_intensity: f64) -> Option<ReunionExpression> {
        self.reunion_burst
            .as_ref()?
            .on_reunion(away_secs, longing_intensity)
    }

    /// 每次心跳时调用 — 衰减 + 自主情感循环
    pub fn tick(&mut self) {
        let hour = self
            .circadian
            .as_ref()
            .map(|c| c.current_hour())
            .unwrap_or(12);
        self.tick_with_hour(hour);
    }

    /// 带指定小时的 tick（用于测试）
    pub fn tick_with_hour(&mut self, hour: u32) {
        // 1. 基础衰减（向 default 回归）
        let effective_decay = if let Some(ref inertia) = self.inertia {
            self.decay_rate * inertia.modifiers.decay_rate
        } else {
            self.decay_rate
        };
        self.current.decay(effective_decay, &self.default);

        // 2. OU 漂移（叠加随机波动）
        if let Some(ref drift) = self.drift {
            let current_arr = [
                self.current.pleasure as f64,
                self.current.arousal as f64,
                self.current.dominance as f64,
            ];
            let delta = drift.step(current_arr);
            self.current.pleasure += delta[0] as f32;
            self.current.arousal += delta[1] as f32;
            self.current.dominance += delta[2] as f32;
            self.current.clamp();
        }

        // 3. 昼夜节律偏移
        if let Some(ref circadian) = self.circadian {
            let offset = circadian.rhythm_offset(hour);
            self.current.pleasure += offset[0];
            self.current.arousal += offset[1];
            self.current.dominance += offset[2];
            self.current.clamp();
        }

        // 4. 更新惯性历史 / Update inertia history
        if let Some(ref mut inertia) = self.inertia {
            inertia.tick([
                self.current.pleasure,
                self.current.arousal,
                self.current.dominance,
            ]);
        }

        // 5. 想念调制 / Longing modulation
        // intensity 和 current_baseline 已由 CoreService::longing_tick() 预先更新，
        // 此处执行 OU step 向 current_baseline 漂移，纯 f64 运算，零分配。
        if let Some((ref params, ref state)) = self.longing {
            if state.intensity > 0.0 {
                let current_arr = [
                    self.current.pleasure as f64,
                    self.current.arousal as f64,
                    self.current.dominance as f64,
                ];
                let delta = params.step_toward(current_arr, state.current_baseline);
                self.current.pleasure += delta[0] as f32;
                self.current.arousal += delta[1] as f32;
                self.current.dominance += delta[2] as f32;
                self.current.clamp();
            }
        }
    }

    /// 外部事件影响情感（受惯性敏感度调制）
    pub fn affect(&mut self, delta: &EmotionState) {
        let sensitivity = self
            .inertia
            .as_ref()
            .map(|i| i.modifiers.sensitivity)
            .unwrap_or(1.0);

        self.current.pleasure += delta.pleasure * sensitivity;
        self.current.arousal += delta.arousal * sensitivity;
        self.current.dominance += delta.dominance * sensitivity;
        self.current.clamp();
    }

    pub fn current(&self) -> &EmotionState {
        &self.current
    }

    /// 获取当前情绪标签（9 种基本情绪 + emoji）
    pub fn current_label(&self) -> &'static EmotionLabel {
        self.current.classify()
    }

    /// 获取惯性修正器（用于外部读取当前敏感度等参数）
    pub fn inertia_modifiers(&self) -> Option<&InertiaModifiers> {
        self.inertia.as_ref().map(|i| &i.modifiers)
    }

    /// 获取当前主导情绪标签（惯性系统）/ Get dominant emotion label (inertia system).
    pub fn dominant_label(&self) -> Option<&str> {
        self.inertia.as_ref().and_then(|i| i.dominant_label())
    }

    /// 获取想念状态（只读）/ Get longing state (read-only).
    pub fn longing_state(&self) -> Option<&LongingState> {
        self.longing.as_ref().map(|(_, state)| state)
    }

    /// 获取想念参数与状态（可变）/ Get longing params and state (mutable).
    ///
    /// 由 CoreService::longing_tick() 调用，更新 intensity 和 current_baseline。
    pub fn longing_mut(&mut self) -> Option<(&mut LongingParams, &mut LongingState)> {
        self.longing.as_mut().map(|(p, s)| (p, s))
    }

    /// 生成可序列化的运行时快照（用于持久化）
    /// Generate serializable runtime snapshot for persistence.
    pub fn snapshot(&self) -> EmotionSnapshot {
        EmotionSnapshot {
            current: self.current.clone(),
            inertia_history: self
                .inertia
                .as_ref()
                .map(|i| i.history.iter().copied().collect())
                .unwrap_or_default(),
            inertia_dominant_duration: self
                .inertia
                .as_ref()
                .map(|i| i.dominant_duration)
                .unwrap_or(0),
            inertia_dominant_label: self.inertia.as_ref().and_then(|i| i.dominant_label.clone()),
            inertia_modifiers: self
                .inertia
                .as_ref()
                .map(|i| i.modifiers.clone())
                .unwrap_or_default(),
            longing_state: self.longing.as_ref().map(|(_, s)| s.clone()),
        }
    }

    /// 从快照恢复运行时状态 / Restore runtime state from snapshot.
    pub fn restore(&mut self, snap: &EmotionSnapshot) {
        self.current = snap.current.clone();
        if let Some(ref mut inertia) = self.inertia {
            inertia.history = snap.inertia_history.iter().copied().collect();
            inertia.dominant_duration = snap.inertia_dominant_duration;
            inertia.dominant_label = snap.inertia_dominant_label.clone();
            inertia.modifiers = snap.inertia_modifiers.clone();
        }
        // 恢复想念状态 / Restore longing state
        if let Some(ref mut longing) = self.longing {
            if let Some(ref restored) = snap.longing_state {
                longing.1 = restored.clone();
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// EmotionSnapshot — 情感引擎持久化快照
// ════════════════════════════════════════════════════════════════════

/// 情感引擎运行时状态快照
///
/// 仅保存不可从配置重建的运行时状态：
/// - 当前 PAD 值
/// - 惯性历史队列与主导情绪追踪
/// - 想念引擎运行时状态
///
/// 配置相关状态（decay_rate, drift, circadian）由 `build_emotion_engine` 重建。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    pub current: EmotionState,
    pub inertia_history: Vec<[f32; 3]>,
    pub inertia_dominant_duration: usize,
    pub inertia_dominant_label: Option<String>,
    pub inertia_modifiers: InertiaModifiers,
    /// 想念引擎运行时状态 / Longing engine runtime state
    #[serde(default)]
    pub longing_state: Option<LongingState>,
}

// ════════════════════════════════════════════════════════════════════
// 高阶情绪模型 — 复合情绪层（20+ 种）
// ════════════════════════════════════════════════════════════════════

/// 情绪方向性：标记情绪的指向对象
///
/// 人类情绪不仅由 PAD 值决定，还取决于"对谁/对什么"产生的：
/// - Self-directed: 自豪/羞耻/内疚（指向自身）
/// - User-directed: 感激/心疼/嫉妒（指向对话对象）
/// - Memory-directed: 怀旧/释然（指向过去的记忆）
/// - Neutral: 无特定方向（敬畏/孤独等）
#[derive(Clone, Debug, PartialEq)]
pub enum EmotionDirection {
    SelfDirected,
    UserDirected,
    MemoryDirected,
    Neutral,
}

/// 复合情绪标签
///
/// 在 9 种基本情绪之上，叠加 22 种高阶复合情绪。
/// 每种情绪由 PAD 区域 + 方向性约束共同决定。
#[derive(Clone, Debug)]
pub struct CompoundEmotion {
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    /// PAD 区域的中心点
    pub pad_center: (f32, f32, f32),
    /// PAD 区域的半径（容差）
    pub pad_radius: f32,
    /// 必须匹配的方向性（None 表示任意方向均可）
    pub direction: Option<EmotionDirection>,
}

/// 22 种复合情绪定义
///
/// PAD 中心点基于 Plutchik 情绪轮 + 社会情绪心理学研究。
/// 半径用于控制判定的宽松程度（越小越严格）。
pub const COMPOUND_EMOTIONS: [CompoundEmotion; 22] = [
    // ── 自我指向 ──
    CompoundEmotion {
        name: "内疚",
        emoji: "😔",
        description: "对自己的行为感到后悔和不安",
        pad_center: (-0.4, 0.2, -0.5),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "自豪",
        emoji: "😤",
        description: "对自己的成就感到满足和骄傲",
        pad_center: (0.6, 0.4, 0.7),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "羞耻",
        emoji: "😳",
        description: "因自身不足或错误而感到难堪",
        pad_center: (-0.5, 0.3, -0.6),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "自信",
        emoji: "💪",
        description: "对自身能力的坚定信心",
        pad_center: (0.4, 0.3, 0.8),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    // ── 对方指向 ──
    CompoundEmotion {
        name: "感激",
        emoji: "🙏",
        description: "对他人善意和帮助的由衷感谢",
        pad_center: (0.5, 0.2, -0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "心疼",
        emoji: "💔",
        description: "看到对方受苦时产生的怜惜和关切",
        pad_center: (-0.3, 0.1, -0.1),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "嫉妒",
        emoji: "😒",
        description: "因他人的优势或拥有而感到不平衡",
        pad_center: (-0.4, 0.5, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "钦佩",
        emoji: "🤝",
        description: "对他人能力或品格的尊重和赞赏",
        pad_center: (0.4, 0.3, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "怜爱",
        emoji: "🥰",
        description: "对对方的温柔喜爱和保护欲",
        pad_center: (0.6, 0.1, 0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    // ── 记忆指向 ──
    CompoundEmotion {
        name: "怀旧",
        emoji: "🌅",
        description: "回忆过去时混合的温暖与淡淡忧伤",
        pad_center: (0.1, -0.1, -0.2),
        pad_radius: 0.40,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "释然",
        emoji: "🍃",
        description: "放下过去的执念后的轻松与平和",
        pad_center: (0.3, -0.3, 0.1),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "遗憾",
        emoji: "😞",
        description: "对未能实现之事的不甘与惋惜",
        pad_center: (-0.4, -0.1, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "眷恋",
        emoji: "💭",
        description: "对过去美好时光的深深留恋",
        pad_center: (0.2, 0.0, -0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    // ── 混合情绪（正负 valence 共存）──
    CompoundEmotion {
        name: "百感交集",
        emoji: "🎭",
        description: "同时感受到快乐与忧伤的复杂心境",
        pad_center: (0.0, 0.1, -0.1),
        pad_radius: 0.25,
        direction: None,
    },
    // ── 无方向（状态性情绪）──
    CompoundEmotion {
        name: "敬畏",
        emoji: "🌌",
        description: "面对宏大或超越性事物时的震撼与谦卑",
        pad_center: (0.2, 0.6, -0.5),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "孤独",
        emoji: "🌙",
        description: "缺少陪伴或连接感时的空虚与渴望",
        pad_center: (-0.5, -0.3, -0.4),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "安心",
        emoji: "🏠",
        description: "感受到安全和归属后的踏实与温暖",
        pad_center: (0.4, -0.3, 0.3),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "焦虑",
        emoji: "😰",
        description: "对未来不确定性的持续担忧和紧张",
        pad_center: (-0.4, 0.5, -0.5),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "温柔",
        emoji: "🌸",
        description: "柔和的善意与细腻的情感流动",
        pad_center: (0.5, -0.1, 0.1),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "好奇",
        emoji: "🔍",
        description: "对新事物或未知领域的探索欲望",
        pad_center: (0.3, 0.5, 0.2),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "无奈",
        emoji: "😅",
        description: "面对无法改变之事时的苦笑着接受",
        pad_center: (-0.2, 0.0, -0.4),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "陶醉",
        emoji: "✨",
        description: "沉浸在美好体验中的高度愉悦",
        pad_center: (0.7, 0.3, 0.1),
        pad_radius: 0.35,
        direction: None,
    },
];

/// 复合情绪分类上下文
///
/// 由 `process_message` 管线构建，提供方向性提示和混合情绪线索。
#[derive(Clone, Debug)]
pub struct CompoundContext {
    /// 情绪方向（由消息内容推断）
    pub direction: EmotionDirection,
    /// 用户消息是否包含回忆/过去相关的关键词
    pub has_memory_cue: bool,
    /// 当前基本情绪标签名称
    pub basic_label: &'static str,
}

impl Default for CompoundContext {
    fn default() -> Self {
        Self {
            direction: EmotionDirection::Neutral,
            has_memory_cue: false,
            basic_label: "平静",
        }
    }
}

/// PAD + 上下文 → 复合情绪分类
///
/// 判断逻辑：
/// 1. 遍历所有 22 种复合情绪
/// 2. 计算 PAD 欧氏距离，过滤掉方向不匹配的
/// 3. 选择距离最近且在半径内的复合情绪
/// 4. 若无匹配 → 返回 `None`（回退到基本情绪标签）
pub fn classify_compound(
    state: &EmotionState,
    ctx: &CompoundContext,
) -> Option<&'static CompoundEmotion> {
    let mut best: Option<(f32, &'static CompoundEmotion)> = None;

    for ce in &COMPOUND_EMOTIONS {
        // 方向性过滤：有约束时必须匹配
        if let Some(ref required_dir) = ce.direction {
            if *required_dir != ctx.direction {
                continue;
            }
        }

        // 记忆指向情绪：需要记忆线索或方向为 MemoryDirected
        if matches!(ce.direction, Some(EmotionDirection::MemoryDirected))
            && !ctx.has_memory_cue
            && ctx.direction != EmotionDirection::MemoryDirected
        {
            continue;
        }

        // PAD 欧氏距离
        let dp = state.pleasure - ce.pad_center.0;
        let da = state.arousal - ce.pad_center.1;
        let dd = state.dominance - ce.pad_center.2;
        let dist = (dp * dp + da * da + dd * dd).sqrt();

        if dist > ce.pad_radius {
            continue;
        }

        if best.is_none_or(|(best_dist, _)| dist < best_dist) {
            best = Some((dist, ce));
        }
    }

    best.map(|(_, ce)| ce)
}

/// PAD → 自然语言情绪描述
///
/// 替代原始的 `(愉悦:0.45, 唤醒:0.12, 支配:0.08)` 浮点数格式。
/// 优先使用复合情绪（如果匹配到），否则使用基本情绪标签。
pub fn to_natural_language(state: &EmotionState, ctx: &CompoundContext) -> String {
    // 先尝试复合情绪
    if let Some(compound) = classify_compound(state, ctx) {
        return format!("{} {}", compound.emoji, compound.name);
    }

    // 回退到基本情绪
    let basic = state.classify();
    format!("{} {}", basic.emoji, basic.name)
}

/// 从消息文本推断情绪方向性
///
/// 基于关键词检测的轻量启发式方法：
/// - "我"/"自己"/"自己的" → SelfDirected
/// - "你"/"谢"/"感谢"/"对不起" → UserDirected
/// - "以前"/"小时候"/"记得"/"回忆"/"当年" → MemoryDirected
pub fn infer_direction(message: &str) -> EmotionDirection {
    let msg_lower = message.to_lowercase();

    // 记忆指向（优先检查，因为怀旧相关词较独特）
    let memory_keywords = [
        "以前",
        "小时候",
        "记得",
        "回忆",
        "当年",
        "那年",
        "过去",
        "曾经",
        "那时",
        "往事",
        "怀念",
        "想念",
        "remember",
        "nostalgia",
        "back then",
        "used to",
    ];
    for kw in &memory_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::MemoryDirected;
        }
    }

    // 对方指向
    let user_keywords = [
        "谢谢你",
        "感谢你",
        "多亏你",
        "对不起",
        "抱歉",
        "你真",
        "你太",
        "感谢你",
        "辛苦你",
        "谢谢",
        "thank",
        "sorry",
        "grateful",
    ];
    for kw in &user_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::UserDirected;
        }
    }

    // 自我指向
    let self_keywords = [
        "我觉得自己",
        "我做到了",
        "我成功",
        "我失败",
        "我太差",
        "我骄傲",
        "我自豪",
        "我惭愧",
        "我后悔",
        "i did",
        "i achieved",
        "i failed",
        "i'm proud",
    ];
    for kw in &self_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::SelfDirected;
        }
    }

    EmotionDirection::Neutral
}

/// 检测混合情绪（正负 valence 同时存在）
///
/// 当 pleasure 接近 0 但 arousal 非零时，可能存在混合情绪。
/// 返回 `Some("百感交集")` 如果检测到混合状态。
pub fn detect_mixed_emotion(state: &EmotionState) -> Option<&'static CompoundEmotion> {
    // 混合情绪特征：pleasure 接近中性（-0.15 ~ 0.15）+ 有一定唤醒度
    if state.pleasure.abs() < 0.15 && state.arousal.abs() > 0.15 {
        // 查找百感交集
        return COMPOUND_EMOTIONS.iter().find(|ce| ce.name == "百感交集");
    }
    None
}

// ════════════════════════════════════════════════════════════════════
// Gap#3 独立模块 — 想念/期待/失落/叙事桥接 / Gap#3 Independent Modules
// ════════════════════════════════════════════════════════════════════

pub mod anticipation_preloader;
pub mod disappointment_handler;
pub mod longing_expression_channel;
pub mod longing_narrative_bridge;

// 重导出以保持公共 API 兼容 / Re-exports for backward-compatible public API
pub use anticipation_preloader::AnticipationPreLoader;
pub use disappointment_handler::{DisappointmentHandler, DisappointmentResult};
pub use longing_expression_channel::{LongingExpression, LongingExpressionChannel};
pub use longing_narrative_bridge::LongingNarrativeBridge;

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    // ── EmotionState 测试 ──

    #[test]
    fn test_default_state() {
        let state = EmotionState::new(0.0, 0.0, 0.0);
        assert!(approx_eq(state.pleasure, 0.0, 1e-6));
        assert!(approx_eq(state.arousal, 0.0, 1e-6));
        assert!(approx_eq(state.dominance, 0.0, 1e-6));
    }

    // ── affect 测试 ──

    #[test]
    fn test_affect_positive() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        engine.affect(&EmotionState::new(0.5, 0.3, 0.2));
        assert!(approx_eq(engine.current().pleasure, 0.5, 1e-6));
        assert!(approx_eq(engine.current().arousal, 0.3, 1e-6));
        assert!(approx_eq(engine.current().dominance, 0.2, 1e-6));
    }

    #[test]
    fn test_affect_negative() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        engine.affect(&EmotionState::new(-0.8, -0.5, -0.3));
        assert!(approx_eq(engine.current().pleasure, -0.8, 1e-6));
        assert!(approx_eq(engine.current().arousal, -0.5, 1e-6));
        assert!(approx_eq(engine.current().dominance, -0.3, 1e-6));
    }

    #[test]
    fn test_affect_clamp_upper() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        engine.affect(&EmotionState::new(2.0, 1.5, 0.0));
        assert!(approx_eq(engine.current().pleasure, 1.0, 1e-6));
        assert!(approx_eq(engine.current().arousal, 1.0, 1e-6));
    }

    #[test]
    fn test_affect_clamp_lower() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        engine.affect(&EmotionState::new(-1.5, -2.0, 0.0));
        assert!(approx_eq(engine.current().pleasure, -1.0, 1e-6));
        assert!(approx_eq(engine.current().arousal, -1.0, 1e-6));
    }

    // ── decay 测试 ──

    #[test]
    fn test_decay_to_default() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.5);
        engine.affect(&EmotionState::new(1.0, 0.5, 0.3));
        engine.tick();
        // 衰减 50%: 0.5, 0.25, 0.15
        assert!(approx_eq(engine.current().pleasure, 0.5, 1e-6));
        assert!(approx_eq(engine.current().arousal, 0.25, 1e-6));
        assert!(approx_eq(engine.current().dominance, 0.15, 1e-6));
    }

    #[test]
    fn test_decay_multiple_ticks() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.2);
        engine.affect(&EmotionState::new(1.0, 1.0, 1.0));
        for _ in 0..10 {
            engine.tick();
        }
        // 10 次 * 20% 衰减后应接近 0.0
        assert!(engine.current().pleasure.abs() < 0.2);
        assert!(engine.current().arousal.abs() < 0.2);
        assert!(engine.current().dominance.abs() < 0.2);
    }

    #[test]
    fn test_decay_rate_zero() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.0);
        engine.affect(&EmotionState::new(1.0, 1.0, 1.0));
        for _ in 0..100 {
            engine.tick();
        }
        assert!(approx_eq(engine.current().pleasure, 1.0, 1e-6)); // 永不衰减
    }

    #[test]
    fn test_decay_rate_one() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 1.0);
        engine.affect(&EmotionState::new(1.0, 1.0, 1.0));
        engine.tick();
        assert!(approx_eq(engine.current().pleasure, 0.0, 1e-6)); // 瞬间归零
        assert!(approx_eq(engine.current().arousal, 0.0, 1e-6));
        assert!(approx_eq(engine.current().dominance, 0.0, 1e-6));
    }

    // ── current 安全测试 ──

    #[test]
    fn test_current_immutable() {
        let engine = EmotionEngine::new(EmotionState::new(0.5, 0.3, 0.1), 0.1);
        let state = engine.current();
        assert!(approx_eq(state.pleasure, 0.5, 1e-6));
        assert!(approx_eq(state.arousal, 0.3, 1e-6));
        assert!(approx_eq(state.dominance, 0.1, 1e-6));
    }

    #[test]
    fn test_emotion_labels() {
        assert_eq!(EMOTION_LABELS.len(), 9);
        // 高愉悦+高唤醒 = 兴奋
        let label = EmotionState::new(0.6, 0.8, 0.5).classify();
        assert_eq!(label.name, "兴奋");
        // 低愉悦+高唤醒+低支配 = 恐惧
        let label = EmotionState::new(-0.7, 0.6, -0.7).classify();
        assert_eq!(label.name, "恐惧");
        // 中性 = 平静
        let label = EmotionState::new(0.1, -0.5, -0.1).classify();
        assert_eq!(label.name, "平静");
    }

    // ══════════════════════════════════════════════════════════════
    // 新增测试 — 自主情感循环
    // ══════════════════════════════════════════════════════════════

    // ── DriftParams（OU 过程）测试 ──

    #[test]
    fn test_drift_changes_state() {
        let drift = DriftParams::new(0.1, 0.001);
        let current = [0.0, 0.0, 0.0];
        let delta = drift.step(current);
        // 高波动率下至少有一个维度非零
        assert!(
            delta[0].abs() > 0.001 || delta[1].abs() > 0.001 || delta[2].abs() > 0.001,
            "OU 过程应产生可测量的变化"
        );
    }

    #[test]
    fn test_drift_mean_reversion() {
        // 高均值回归率，偏离基线 → 应被拉回
        let drift = DriftParams {
            volatility: 0.0, // 无噪声
            mean_reversion: 0.5,
            baseline: [0.0, 0.0, 0.0],
        };
        let delta = drift.step([1.0, -1.0, 0.5]);
        // 应产生负向 P 修正（拉回 0）
        assert!(delta[0] < 0.0, "正偏离应被拉回: delta[0]={}", delta[0]);
        // 应产生正向 A 修正（拉回 0）
        assert!(delta[1] > 0.0, "负偏离应被拉回: delta[1]={}", delta[1]);
        // D 偏离 0.5 → 负向修正
        assert!(delta[2] < 0.0, "正偏离应被拉回: delta[2]={}", delta[2]);
    }

    #[test]
    fn test_drift_accumulates() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.0)
            .with_drift(DriftParams::new(0.05, 0.001));
        for _ in 0..1000 {
            engine.tick_with_hour(12);
        }
        let p = engine.current().pleasure.abs();
        let a = engine.current().arousal.abs();
        // 1000 次 tick 后至少一个维度应有可测量偏移
        assert!(p > 0.01 || a > 0.01, "漂移应累积：P={:.4}, A={:.4}", p, a);
    }

    #[test]
    fn test_drift_stays_bounded() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.0)
            .with_drift(DriftParams::new(0.05, 0.01));
        for _ in 0..5000 {
            engine.tick_with_hour(12);
        }
        assert!(
            engine.current().pleasure.abs() <= 1.0,
            "P 应被限制在 [-1, 1]"
        );
        assert!(
            engine.current().arousal.abs() <= 1.0,
            "A 应被限制在 [-1, 1]"
        );
        assert!(
            engine.current().dominance.abs() <= 1.0,
            "D 应被限制在 [-1, 1]"
        );
    }

    // ── CircadianModulator 测试 ──

    #[test]
    fn test_circadian_morning_peak() {
        let circadian = CircadianModulator::default();
        let offset_10 = circadian.rhythm_offset(10);
        let offset_3 = circadian.rhythm_offset(3);
        // 上午 10 点（峰值）的唤醒度应高于凌晨 3 点
        assert!(
            offset_10[1] > offset_3[1],
            "10 点唤醒度应高于 3 点：10h={:.4}, 3h={:.4}",
            offset_10[1],
            offset_3[1]
        );
    }

    #[test]
    fn test_circadian_evening_peak() {
        let circadian = CircadianModulator::default();
        let offset_18 = circadian.rhythm_offset(18);
        let offset_14 = circadian.rhythm_offset(14);
        // 傍晚 18 点（第二峰值）的唤醒度应高于 14 点（两峰之间谷值）
        assert!(
            offset_18[1] > offset_14[1],
            "18 点唤醒度应高于 14 点：18h={:.4}, 14h={:.4}",
            offset_18[1],
            offset_14[1]
        );
    }

    #[test]
    fn test_circadian_night_dip() {
        let circadian = CircadianModulator::default();
        let offset_2 = circadian.rhythm_offset(2);
        // 凌晨 2 点应在活跃时段外 → 负偏移
        assert!(offset_2[0] < 0.0, "夜间情绪应偏负");
        assert!(offset_2[1] < 0.0, "夜间唤醒度应偏负");
    }

    #[test]
    fn test_circadian_active_hours_positive() {
        let circadian = CircadianModulator::default();
        let offset_10 = circadian.rhythm_offset(10);
        // 上午 10 点（峰值）唤醒度应为正
        assert!(
            offset_10[1] > 0.0,
            "峰值时段唤醒度应为正：{:.4}",
            offset_10[1]
        );
    }

    #[test]
    fn test_circadian_gaussian_shape() {
        // 验证高斯函数本身
        let peak = gaussian(10.0, 10.0, 2.0);
        let off = gaussian(12.0, 10.0, 2.0);
        assert!(approx_eq(peak, 1.0, 1e-6), "峰值处应为 1.0");
        assert!(off < 1.0 && off > 0.0, "偏离处应在 (0, 1)");
    }

    // ── EmotionalInertia 测试 ──

    #[test]
    fn test_inertia_activates_on_dominant_emotion() {
        let mut inertia = EmotionalInertia::default();
        // 持续输入"平静"情绪（P=0.1, A=-0.5, D=-0.1）
        for _ in 0..100 {
            inertia.tick([0.1, -0.5, -0.1]);
        }
        // 50 tick 后应激活，敏感度应升高
        assert!(
            inertia.modifiers.sensitivity > 1.0,
            "持续平静情绪后敏感度应升高：{}",
            inertia.modifiers.sensitivity
        );
    }

    #[test]
    fn test_inertia_resets_on_variety() {
        let mut inertia = EmotionalInertia::default();
        // 先建立惯性（需足够 tick 让 dominant_duration 超过 activation_ticks 阈值）
        for _ in 0..120 {
            inertia.tick([0.1, -0.5, -0.1]); // 平静
        }
        assert!(
            inertia.modifiers.sensitivity > 1.0,
            "持续平静后敏感度应升高：{}",
            inertia.modifiers.sensitivity
        );

        // 注入多样化情绪 → 惯性应重置
        inertia.tick([0.7, 0.5, 0.4]); // 愉悦
        inertia.tick([-0.7, -0.3, -0.5]); // 悲伤
        inertia.tick([-0.6, 0.7, 0.6]); // 愤怒
                                        // 多来几轮确保 dominant ratio 低于 0.6
        for i in 0..20 {
            let pad = match i % 4 {
                0 => [0.7, 0.5, 0.4],
                1 => [-0.7, -0.3, -0.5],
                2 => [-0.6, 0.7, 0.6],
                _ => [0.2, 0.75, -0.3],
            };
            inertia.tick(pad);
        }
        assert!(
            inertia.modifiers.sensitivity <= 1.01,
            "多样化情绪后惯性应重置：sensitivity={}",
            inertia.modifiers.sensitivity
        );
    }

    #[test]
    fn test_inertia_decay_rate_decreases() {
        let mut inertia = EmotionalInertia::default();
        // 长时间同一情绪
        for _ in 0..200 {
            inertia.tick([0.1, -0.5, -0.1]);
        }
        assert!(
            inertia.modifiers.decay_rate < 1.0,
            "长期惯性应降低衰减率：{}",
            inertia.modifiers.decay_rate
        );
    }

    #[test]
    fn test_inertia_not_active_initially() {
        let inertia = EmotionalInertia::default();
        assert!(
            approx_eq(inertia.modifiers.sensitivity, 1.0, 1e-6),
            "初始敏感度应为 1.0"
        );
        assert!(
            approx_eq(inertia.modifiers.decay_rate, 1.0, 1e-6),
            "初始衰减率应为 1.0"
        );
    }

    // ── 集成测试 ──

    #[test]
    fn test_engine_with_all_systems() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1)
            .with_drift(DriftParams::new(0.01, 0.001))
            .with_circadian(CircadianModulator::default())
            .with_inertia(EmotionalInertia::default());

        // 运行 100 tick 不应 panic
        for _ in 0..100 {
            engine.tick_with_hour(10);
        }
        assert!(engine.current().pleasure.abs() <= 1.0, "P 应在范围内");
        assert!(engine.current().arousal.abs() <= 1.0, "A 应在范围内");
    }

    #[test]
    fn test_affect_with_inertia_sensitivity() {
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1)
            .with_inertia(EmotionalInertia::default());

        // 建立惯性（持续平静情绪）
        for _ in 0..100 {
            engine.tick_with_hour(14); // tick 内部会更新惯性
        }

        let sensitivity = engine.inertia_modifiers().unwrap().sensitivity;
        let p_before = engine.current().pleasure;

        engine.affect(&EmotionState::new(0.1, 0.0, 0.0));

        let p_after = engine.current().pleasure;
        let actual_delta = p_after - p_before;

        // affect 的 delta 应受敏感度调制
        assert!(
            (actual_delta - 0.1 * sensitivity).abs() < 0.01,
            "affect 应受敏感度调制：expected ~{:.3}, got {:.3}",
            0.1 * sensitivity,
            actual_delta
        );
    }

    #[test]
    fn test_backward_compatible_default() {
        // 默认构造（无 drift/circadian/inertia）应与原行为完全一致
        let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        engine.affect(&EmotionState::new(1.0, 1.0, 1.0));
        engine.tick();
        // 衰减 10%：1.0 * (1 - 0.1) = 0.9
        assert!(approx_eq(engine.current().pleasure, 0.9, 1e-6));
        assert!(approx_eq(engine.current().arousal, 0.9, 1e-6));
        assert!(approx_eq(engine.current().dominance, 0.9, 1e-6));
    }

    // ══════════════════════════════════════════════════════════════
    // 复合情绪测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_compound_guilt_self_directed() {
        // 内疚: P=-0.4, A=0.2, D=-0.5, 方向=SelfDirected
        let state = EmotionState::new(-0.4, 0.2, -0.5);
        let ctx = CompoundContext {
            direction: EmotionDirection::SelfDirected,
            has_memory_cue: false,
            basic_label: "恐惧",
        };
        let result = classify_compound(&state, &ctx);
        assert!(result.is_some(), "应匹配到复合情绪");
        assert_eq!(result.unwrap().name, "内疚");
    }

    #[test]
    fn test_compound_gratitude_user_directed() {
        // 感激: P=0.5, A=0.2, D=-0.2, 方向=UserDirected
        let state = EmotionState::new(0.5, 0.2, -0.2);
        let ctx = CompoundContext {
            direction: EmotionDirection::UserDirected,
            has_memory_cue: false,
            basic_label: "愉悦",
        };
        let result = classify_compound(&state, &ctx);
        assert!(result.is_some(), "应匹配到复合情绪");
        assert_eq!(result.unwrap().name, "感激");
    }

    #[test]
    fn test_compound_nostalgia_memory_directed() {
        // 怀旧: P=0.1, A=-0.1, D=-0.2, 方向=MemoryDirected
        let state = EmotionState::new(0.1, -0.1, -0.2);
        let ctx = CompoundContext {
            direction: EmotionDirection::MemoryDirected,
            has_memory_cue: true,
            basic_label: "平静",
        };
        let result = classify_compound(&state, &ctx);
        assert!(result.is_some(), "应匹配到怀旧");
        assert_eq!(result.unwrap().name, "怀旧");
    }

    #[test]
    fn test_compound_direction_filter_blocks_mismatch() {
        // PAD 匹配内疚区域，但方向为 Neutral → 不应匹配内疚
        let state = EmotionState::new(-0.4, 0.2, -0.5);
        let ctx = CompoundContext {
            direction: EmotionDirection::Neutral,
            has_memory_cue: false,
            basic_label: "恐惧",
        };
        let result = classify_compound(&state, &ctx);
        // Neutral 方向下，可能匹配到无方向的"焦虑"或 None
        if let Some(ce) = result {
            assert_ne!(ce.name, "内疚", "Neutral 方向不应匹配内疚");
            assert_ne!(ce.name, "羞耻", "Neutral 方向不应匹配羞耻");
        }
    }

    #[test]
    fn test_compound_mixed_emotion_detection() {
        // 混合情绪: pleasure ≈ 0, arousal 非零
        let state = EmotionState::new(0.05, 0.3, -0.05);
        let mixed = detect_mixed_emotion(&state);
        assert!(mixed.is_some(), "应检测到混合情绪");
        assert_eq!(mixed.unwrap().name, "百感交集");

        // 非混合: pleasure 远离 0
        let state2 = EmotionState::new(0.6, 0.3, 0.1);
        let mixed2 = detect_mixed_emotion(&state2);
        assert!(mixed2.is_none(), "高 pleasure 不应为混合情绪");
    }

    #[test]
    fn test_infer_direction_from_text() {
        assert_eq!(
            infer_direction("谢谢你帮我这么多"),
            EmotionDirection::UserDirected
        );
        assert_eq!(
            infer_direction("记得小时候我们一起玩"),
            EmotionDirection::MemoryDirected
        );
        assert_eq!(
            infer_direction("我做到了！我成功了"),
            EmotionDirection::SelfDirected
        );
        assert_eq!(infer_direction("今天天气不错"), EmotionDirection::Neutral);
        assert_eq!(
            infer_direction("I remember those days"),
            EmotionDirection::MemoryDirected
        );
    }

    #[test]
    fn test_to_natural_language_with_compound() {
        // 复合情绪匹配时 → 使用复合标签
        let state = EmotionState::new(0.5, 0.2, -0.2);
        let ctx = CompoundContext {
            direction: EmotionDirection::UserDirected,
            has_memory_cue: false,
            basic_label: "愉悦",
        };
        let result = to_natural_language(&state, &ctx);
        assert!(result.contains("感激"), "应包含复合情绪名: {}", result);

        // 无复合匹配时 → 回退到基本情绪
        let state2 = EmotionState::new(0.7, 0.5, 0.4);
        let ctx2 = CompoundContext::default();
        let result2 = to_natural_language(&state2, &ctx2);
        // 应包含基本情绪的 emoji + name（可能匹配到"陶醉"或"愉悦"）
        assert!(!result2.is_empty(), "自然语言输出不应为空");
    }

    #[test]
    fn test_compound_all_emotions_have_unique_names() {
        let mut names = std::collections::HashSet::new();
        for ce in &COMPOUND_EMOTIONS {
            assert!(names.insert(ce.name), "复合情绪名称重复: {}", ce.name);
        }
        assert_eq!(names.len(), 22, "应有 22 种不重复的复合情绪");
    }

    // ══════════════════════════════════════════════════════════════
    // C1.2: should_express_longing 门控测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_should_express_longing_acquaintance_never() {
        // 初识阶段：无论强度多高都不表达 / Acquaintance: never express
        let state = LongingState {
            intensity: 1.0,
            ..LongingState::default()
        };
        assert!(!state.should_express_longing(0, 0.3), "初识不应表达想念");
        assert!(
            !state.should_express_longing(0, 0.0),
            "即使阈值=0，初识也不表达"
        );
    }

    #[test]
    fn test_should_express_longing_familiar_higher_threshold() {
        // 熟悉阶段：需要 1.5x 阈值 / Familiar: need 1.5x threshold
        let threshold = 0.4;
        let state_low = LongingState {
            intensity: 0.5,
            ..LongingState::default()
        };
        let state_high = LongingState {
            intensity: 0.7,
            ..LongingState::default()
        };

        // 0.5 <= 0.4 * 1.5 = 0.6 → 不表达
        assert!(
            !state_low.should_express_longing(1, threshold),
            "熟悉阶段强度不足，不应表达"
        );
        // 0.7 > 0.4 * 1.5 = 0.6 → 表达
        assert!(
            state_high.should_express_longing(1, threshold),
            "熟悉阶段强度足够，应表达"
        );
    }

    #[test]
    fn test_should_express_longing_trusted_normal_threshold() {
        // 信任阶段：正常阈值 / Trusted: normal threshold
        let threshold = 0.4;
        let state_low = LongingState {
            intensity: 0.3,
            ..LongingState::default()
        };
        let state_high = LongingState {
            intensity: 0.5,
            ..LongingState::default()
        };

        assert!(
            !state_low.should_express_longing(2, threshold),
            "信任阶段强度不足，不应表达"
        );
        assert!(
            state_high.should_express_longing(2, threshold),
            "信任阶段强度足够，应表达"
        );
    }

    #[test]
    fn test_should_express_longing_deep_normal_threshold() {
        // 深度阶段：正常阈值 / Deep: normal threshold
        let threshold = 0.4;
        let state = LongingState {
            intensity: 0.5,
            ..LongingState::default()
        };
        assert!(
            state.should_express_longing(3, threshold),
            "深度阶段强度足够，应表达"
        );

        let state_low = LongingState {
            intensity: 0.3,
            ..LongingState::default()
        };
        assert!(
            !state_low.should_express_longing(3, threshold),
            "深度阶段强度不足，不应表达"
        );
    }

    #[test]
    fn test_should_express_longing_ordinal_boundary() {
        // 序数 >= 2 均走正常阈值（防御性测试）/ Ordinals >= 2 use normal threshold
        let threshold = 0.5;
        let state = LongingState {
            intensity: 0.6,
            ..LongingState::default()
        };
        for ord in 2u8..=10 {
            assert!(
                state.should_express_longing(ord, threshold),
                "序数 {} 应走正常阈值",
                ord
            );
        }
    }

    #[test]
    fn test_should_express_longing_familiar_exact_boundary() {
        // 熟悉阶段精确边界：intensity == threshold * 1.5 时不表达（严格大于）
        // Familiar exact boundary: intensity == threshold * 1.5 → not express (strict >)
        let threshold = 0.4;
        let exact = threshold * 1.5; // 0.6
        let state = LongingState {
            intensity: exact,
            ..LongingState::default()
        };
        assert!(
            !state.should_express_longing(1, threshold),
            "熟悉阶段精确边界不应表达（需要严格大于）"
        );
    }

    // ══════════════════════════════════════════════════════════════
    // C2: ReunionBurst 重逢爆发测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_reunion_burst_too_short() {
        // 离开时间太短不触发 / Too short away duration: no burst
        let burst = ReunionBurst::default();
        assert!(burst.on_reunion(60, 0.5).is_none(), "离开1分钟不应触发");
        assert!(burst.on_reunion(299, 0.5).is_none(), "离开<5分钟不应触发");
    }

    #[test]
    fn test_reunion_burst_min_threshold() {
        // 刚好达到阈值 / Just at threshold
        let burst = ReunionBurst::default();
        let result = burst.on_reunion(300, 0.0);
        assert!(result.is_some(), "离开5分钟应触发");
        let expr = result.unwrap();
        assert!(expr.intensity > 0.0, "强度应>0");
    }

    #[test]
    fn test_reunion_burst_longing_bonus() {
        // 想念强度加成 / Longing intensity bonus
        let burst = ReunionBurst::default();
        let no_longing = burst.on_reunion(3600, 0.0).unwrap();
        let with_longing = burst.on_reunion(3600, 0.8).unwrap();
        assert!(
            with_longing.intensity > no_longing.intensity,
            "有想念加成时强度应更高: {} vs {}",
            with_longing.intensity,
            no_longing.intensity
        );
    }

    #[test]
    fn test_reunion_burst_saturation() {
        // 长时间离开饱和 / Long away duration saturates
        let burst = ReunionBurst::default();
        let one_day = burst.on_reunion(86400, 0.0).unwrap();
        let two_days = burst.on_reunion(172800, 0.0).unwrap();
        // 超过饱和时长后强度不再增长（被 clamp）
        assert!(
            one_day.intensity <= burst.max_intensity,
            "强度不应超过最大值"
        );
        // 两天和一天强度相同（都饱和了）
        assert!(
            (one_day.intensity - two_days.intensity).abs() < 0.01,
            "超过饱和时长后强度应相同"
        );
    }

    #[test]
    fn test_reunion_burst_phrases() {
        // 建议用语分级 / Suggested phrases by intensity level
        let burst = ReunionBurst::default();
        let low = burst.on_reunion(300, 0.0).unwrap();
        let high = burst.on_reunion(86400, 0.9).unwrap();
        assert!(!low.suggested_phrases.is_empty(), "应有建议用语");
        assert!(!high.suggested_phrases.is_empty(), "应有建议用语");
        // 高强度用语应更热情
        assert!(
            high.suggested_phrases[0].contains("终于")
                || high.suggested_phrases[0].contains("好想")
        );
    }

    #[test]
    fn test_reunion_burst_prompt_fragment() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion(3600, 0.5).unwrap();
        let fragment = burst.prompt_fragment(&expr);
        assert!(fragment.contains("[重逢]"), "应包含重逢标签");
        assert!(fragment.contains("3600"), "应包含离开时长");
    }

    #[test]
    fn test_engine_on_reunion_disabled() {
        // 未启用重逢爆发时返回 None / Disabled reunion burst returns None
        let engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1);
        assert!(engine.on_reunion(3600, 0.5).is_none());
    }

    #[test]
    fn test_engine_on_reunion_enabled() {
        // 启用重逢爆发时返回结果 / Enabled reunion burst returns result
        let engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.1)
            .with_reunion_burst(ReunionBurst::default());
        let result = engine.on_reunion(3600, 0.5);
        assert!(result.is_some(), "启用后应返回重逢表达");
        assert!(result.unwrap().intensity > 0.0);
    }

    // ── 关系门控重逢测试 / Relationship-Gated Reunion Tests ──

    #[test]
    fn test_reunion_gated_acquaintance_faint() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion_gated(3600, 0.5, 0).unwrap();
        // 初识阶段强度乘以 0.2 / Acquaintance intensity * 0.2
        assert!(
            expr.intensity < 0.3,
            "Acquaintance reunion should be faint: {}",
            expr.intensity
        );
        assert!(expr.suggested_phrases.contains(&"在的"));
    }

    #[test]
    fn test_reunion_gated_familiar_moderate() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion_gated(3600, 0.5, 1).unwrap();
        // 熟悉阶段强度乘以 0.6 / Familiar intensity * 0.6
        assert!(expr.intensity >= 0.2 && expr.intensity < 0.8);
        assert!(
            expr.suggested_phrases.contains(&"回来了呀")
                || expr.suggested_phrases.contains(&"欢迎回来~")
        );
    }

    #[test]
    fn test_reunion_gated_trusted_strong() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion_gated(86400, 0.5, 2).unwrap();
        // 信任阶段强度乘以 0.85 / Trusted intensity * 0.85
        // 1天离开 + 0.5想念 → 基础~0.65, 门控后~0.55
        assert!(
            expr.intensity >= 0.4,
            "Trusted gated intensity should be >= 0.4: {}",
            expr.intensity
        );
        assert!(
            expr.suggested_phrases.contains(&"你回来啦")
                || expr.suggested_phrases.contains(&"想你了呢")
        );
    }

    #[test]
    fn test_reunion_gated_deep_full() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion_gated(86400, 0.8, 3).unwrap();
        // 深度阶段全量 / Deep stage full intensity
        assert!(expr.intensity > 0.7);
        assert!(expr
            .suggested_phrases
            .contains(&"好想好想你……你终于回来了！"));
    }

    #[test]
    fn test_reunion_gated_pad_offset() {
        let burst = ReunionBurst::default();
        let expr = burst.on_reunion_gated(3600, 0.5, 3).unwrap();
        // 深度阶段 PAD 偏移应包含 [0.35, 0.15, 0.08] / Deep PAD offset
        assert!(
            expr.pad_modulation[0] >= 0.35,
            "Deep pleasure offset should be >= 0.35"
        );
        assert!(
            expr.pad_modulation[1] >= 0.15,
            "Deep arousal offset should be >= 0.15"
        );
        assert!(
            expr.pad_modulation[2] >= 0.08,
            "Deep dominance offset should be >= 0.08"
        );
    }

    #[test]
    fn test_reunion_gated_too_short() {
        let burst = ReunionBurst::default();
        // 离开时间太短不触发 / Too short to trigger
        let result = burst.on_reunion_gated(60, 0.5, 3);
        assert!(result.is_none());
    }

    // ── 情境化重逢测试 / Contextual Reunion Tests ──

    #[test]
    fn test_reunion_contextual_calm() {
        let burst = ReunionBurst::default();
        let expr = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::Calm)
            .unwrap();
        assert_eq!(expr.context, ReunionContext::Calm);
        // 平静重逢 PAD 不变 / Calm reunion PAD unchanged
        assert_eq!(expr.pad_modulation, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_reunion_contextual_after_conflict() {
        let burst = ReunionBurst::default();
        let base = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::Calm)
            .unwrap();
        let conflict = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::AfterConflict)
            .unwrap();
        // 冲突后强度降低 / After conflict intensity reduced
        assert!(conflict.intensity < base.intensity);
        // 愉悦降低、唤醒升高 / Lower pleasure, higher arousal
        assert!(conflict.pad_modulation[0] < base.pad_modulation[0]);
        assert!(conflict.pad_modulation[1] > base.pad_modulation[1]);
        assert_eq!(conflict.context, ReunionContext::AfterConflict);
    }

    #[test]
    fn test_reunion_contextual_at_ritual() {
        let burst = ReunionBurst::default();
        let base = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::Calm)
            .unwrap();
        let ritual = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::AtRitual)
            .unwrap();
        // 仪式时刻强度加成 / Ritual intensity bonus
        assert!(ritual.intensity > base.intensity);
        // 愉悦加成 / Pleasure bonus
        assert!(ritual.pad_modulation[0] > base.pad_modulation[0]);
        assert_eq!(ritual.context, ReunionContext::AtRitual);
    }

    #[test]
    fn test_reunion_contextual_long_absence() {
        let burst = ReunionBurst::default();
        let base = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::Calm)
            .unwrap();
        let long = burst
            .on_reunion_contextual(3600, 0.5, ReunionContext::LongAbsence)
            .unwrap();
        // 久别重逢强度加成 / Long absence intensity bonus
        assert!(long.intensity > base.intensity);
        assert_eq!(long.context, ReunionContext::LongAbsence);
        assert!(long.suggested_phrases.contains(&"你终于回来了！"));
    }

    #[test]
    fn test_reunion_contextual_conflict_phrases() {
        let burst = ReunionBurst::default();
        let expr = burst
            .on_reunion_contextual(86400, 0.8, ReunionContext::AfterConflict)
            .unwrap();
        // 高强度冲突后用语 / High intensity after-conflict phrases
        assert!(
            expr.suggested_phrases.contains(&"你回来了……我们聊聊？")
                || expr.suggested_phrases.contains(&"还在生气吗……")
        );
    }

    // ── 组合重逢测试 / Combined Reunion Tests ──

    #[test]
    fn test_reunion_full_deep_after_conflict() {
        let burst = ReunionBurst::default();
        let expr = burst
            .on_reunion_full(86400, 0.8, 3, ReunionContext::AfterConflict)
            .unwrap();
        // 深度 + 冲突后：先门控再情境 / Deep + AfterConflict: gate then context
        assert_eq!(expr.context, ReunionContext::AfterConflict);
        // 冲突后强度应低于纯深度 / After conflict intensity lower than pure deep
        let pure_deep = burst.on_reunion_gated(86400, 0.8, 3).unwrap();
        assert!(expr.intensity < pure_deep.intensity);
    }

    #[test]
    fn test_reunion_full_familiar_at_ritual() {
        let burst = ReunionBurst::default();
        let expr = burst
            .on_reunion_full(3600, 0.5, 1, ReunionContext::AtRitual)
            .unwrap();
        // 熟悉 + 仪式：门控后仪式加成 / Familiar + AtRitual: gate then ritual bonus
        assert_eq!(expr.context, ReunionContext::AtRitual);
        assert!(expr.suggested_phrases.contains(&"你刚好在这个时候回来！"));
    }

    #[test]
    fn test_reunion_context_label_zh() {
        assert_eq!(ReunionContext::Calm.label_zh(), "平静重逢");
        assert_eq!(ReunionContext::AfterConflict.label_zh(), "冲突后重逢");
        assert_eq!(ReunionContext::AtRitual.label_zh(), "仪式重逢");
        assert_eq!(ReunionContext::LongAbsence.label_zh(), "久别重逢");
    }

    #[test]
    fn test_match_relationship_config_all_stages() {
        let c0 = ReunionBurst::match_relationship_config(0);
        assert_eq!(c0.min_stage_ordinal, 0);
        assert!((c0.intensity_mult - 0.2).abs() < f64::EPSILON);

        let c1 = ReunionBurst::match_relationship_config(1);
        assert!((c1.intensity_mult - 0.6).abs() < f64::EPSILON);

        let c2 = ReunionBurst::match_relationship_config(2);
        assert!((c2.intensity_mult - 0.85).abs() < f64::EPSILON);

        let c3 = ReunionBurst::match_relationship_config(3);
        assert!((c3.intensity_mult - 1.0).abs() < f64::EPSILON);

        // ordinal > 3 也映射到深度 / ordinal > 3 maps to Deep
        let c99 = ReunionBurst::match_relationship_config(99);
        assert!((c99.intensity_mult - 1.0).abs() < f64::EPSILON);
    }
}
