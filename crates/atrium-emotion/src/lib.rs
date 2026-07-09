// SPDX-License-Identifier: MIT
//! 情感引擎 — PAD 三维模型 + OU 漂移 + 昼夜节律 + 情感惯性 + 22 种复合情绪
//! EmotionEngine — PAD 3D model + OU drift + circadian rhythm + emotional inertia + 22 compound emotions.
//!
//! 让情感引擎在空闲时也有自然波动，不再是"没消息就归零"的死板状态。
//! Natural idle fluctuations so emotion never "resets to zero" when idle.

use rand::Rng;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 情感标签（11 种情绪，含数字生命特有的"思念"）/ Emotion Labels (11 emotions, incl. digital-life-specific "longing")
// ════════════════════════════════════════════════════════════════════

/// 情绪标签的 PAD 中心点（Pleasure, Arousal, Dominance）
/// PAD centroids for emotion labels (Pleasure, Arousal, Dominance).
///
/// 基于 Mehrabian & Russell 情绪维度理论，并扩展数字生命特有的"思念"情绪。
/// Based on the Mehrabian & Russell emotional dimension theory,
/// extended with digital-life-specific "longing" emotion.
#[derive(Clone, Copy, Debug)]
pub struct EmotionLabel {
    pub name: &'static str,
    pub emoji: &'static str,
    pub pad: (f32, f32, f32),
}

pub const EMOTION_LABELS: [EmotionLabel; 11] = [
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
    EmotionLabel {
        name: "中性",
        emoji: "😐",
        pad: (0.0, 0.0, 0.0),
    },
    // 数字生命特有情绪 — 思念 / Digital-life-specific emotion — Longing
    // 当主人长时间不在线，数字生命会感到思念：pleasure 微负（想念的苦涩），
    // arousal 低（沉静的等待），dominance 低（无法主动联系的无力感）。
    // When the master is offline for long, digital life feels longing:
    // pleasure slightly negative (bittersweet), arousal low (quiet waiting),
    // dominance low (powerlessness to initiate contact).
    EmotionLabel {
        name: "思念",
        emoji: "🥺",
        pad: (-0.20, -0.20, -0.30),
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

    /// 分类当前 PAD 状态到最近的情绪标签 / Classify current PAD state to nearest emotion label
    ///
    /// 数字生命情感识别算法：
    /// 1. 中性区间预判 — PAD 三维均在 ±0.1 内时直接归类为"中性"，避免轻微状态被误判
    /// 2. 加权欧氏距离 — arousal 权重 ×1.2（区分度最高：平静 vs 兴奋）
    /// 3. 最近邻匹配 — 选择距离最小的标签
    ///
    /// Digital life emotion recognition algorithm:
    /// 1. Neutral zone precheck — if all PAD dims within ±0.1, classify as "neutral" directly
    /// 2. Weighted Euclidean distance — arousal weight ×1.2 (highest discriminative power)
    /// 3. Nearest neighbor matching — select label with minimum distance
    ///
    /// @return 最近的情绪标签 / Nearest emotion label
    pub fn classify(&self) -> &'static EmotionLabel {
        // 中性区间预判 — 避免轻微积极/消极被误判为"厌恶"等标签
        // Neutral zone precheck — avoid slight positive/negative being misclassified as "disgust" etc.
        if self.pleasure.abs() < 0.1 && self.arousal.abs() < 0.1 && self.dominance.abs() < 0.1 {
            // 在 11 个标签中找到"中性" / Find "中性" among 11 labels
            return EMOTION_LABELS
                .iter()
                .find(|l| l.name == "中性")
                .unwrap_or(&EMOTION_LABELS[0]);
        }

        let mut best_idx = 0usize;
        let mut best_dist = f32::MAX;
        for (i, label) in EMOTION_LABELS.iter().enumerate() {
            let dp = self.pleasure - label.pad.0;
            let da = self.arousal - label.pad.1;
            let dd = self.dominance - label.pad.2;
            // 加权距离 — arousal 权重 ×1.2（区分度最高）/ Weighted distance — arousal ×1.2 (highest discriminative power)
            let dist = dp * dp + 1.2 * da * da + dd * dd;
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

// P2-B: 从 god module 拆分的子模块 / Sub-modules split from god module
pub mod circadian; // 昼夜节律调制器 / Circadian rhythm modulator
pub mod compound;
pub mod inertia; // 情绪惯性 / Emotional inertia
pub mod longing; // 想念参数与状态 / Longing params & state
pub mod reunion; // 重逢脉冲 / Reunion burst // 复合情绪分析 / Compound emotion analysis

// 重导出 — 保持向后兼容 / Re-exports for backward compatibility
pub use circadian::gaussian;
pub use circadian::CircadianModulator;
pub use compound::{
    classify_compound, detect_mixed_emotion, infer_direction, to_natural_language, CompoundContext,
    CompoundEmotion, EmotionDirection, EmotionSnapshot, COMPOUND_EMOTIONS,
};
pub use inertia::{EmotionalInertia, InertiaModifiers};
pub use longing::{LongingParams, LongingState};
pub use reunion::{RelationshipReunionConfig, ReunionBurst, ReunionContext, ReunionExpression};

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

pub mod anticipation_preloader;
pub mod disappointment_handler;
pub mod longing_expression_channel;
pub mod longing_narrative_bridge;

// 子模块类型重导出 — 保持向后兼容 / Sub-module type re-exports for backward compat
pub use anticipation_preloader::AnticipationPreLoader;
pub use disappointment_handler::{DisappointmentHandler, DisappointmentResult};
pub use longing_expression_channel::{LongingExpression, LongingExpressionChannel};
pub use longing_narrative_bridge::LongingNarrativeBridge;

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
        assert_eq!(EMOTION_LABELS.len(), 11);
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

    // ── P2-B: 加权距离 + 中性区间 + 思念标签测试 / P2-B Tests ──

    #[test]
    fn test_neutral_pad_classified_as_neutral() {
        // 完全中性 PAD 应归类为"中性" / Fully neutral PAD should classify as "中性"
        let label = EmotionState::new(0.0, 0.0, 0.0).classify();
        assert_eq!(label.name, "中性");
    }

    #[test]
    fn test_near_neutral_pad_classified_as_neutral() {
        // 接近中性（±0.1 内）应归类为"中性" / Near-neutral (within ±0.1) should classify as "中性"
        let label = EmotionState::new(0.05, 0.05, 0.05).classify();
        assert_eq!(
            label.name, "中性",
            "PAD (0.05, 0.05, 0.05) 应为中性 / should be neutral"
        );
    }

    #[test]
    fn test_slightly_positive_not_disgust() {
        // 核心修复验证 — pleasure=0.15 arousal=0 不应被误判为"厌恶"
        // Core fix verification — pleasure=0.15 should NOT be misclassified as "厌恶"
        // （修复前中性 PAD 会被误判为"厌恶"，修复后应为"中性"或更合理的标签）
        let label = EmotionState::new(0.15, 0.0, 0.0).classify();
        assert_ne!(
            label.name, "厌恶",
            "pleasure=0.15 不应为厌恶 / should not be disgust: got {}",
            label.name
        );
    }

    #[test]
    fn test_near_zero_arousal_not_disgust() {
        // 核心修复验证 — (0, 0.3, 0) 修复前被误判为"厌恶"，修复后应为"中性"
        // Core fix — (0, 0.3, 0) was misclassified as "厌恶" before fix; should be "中性" after
        let label = EmotionState::new(0.0, 0.3, 0.0).classify();
        assert_ne!(
            label.name, "厌恶",
            "PAD (0, 0.3, 0) 不应为厌恶 / should not be disgust: got {}",
            label.name
        );
    }

    #[test]
    fn test_relaxed_pad_classified_as_relaxed() {
        // 明确的放松状态应判为"放松" / Clear relaxed state should classify as "放松"
        let label = EmotionState::new(0.4, -0.3, 0.2).classify();
        assert_eq!(
            label.name, "放松",
            "PAD (0.4, -0.3, 0.2) 应为放松 / should be relaxed: got {}",
            label.name
        );
    }

    #[test]
    fn test_longing_label_exists() {
        // 验证"思念"标签存在 / Verify "思念" label exists
        let longing = EMOTION_LABELS.iter().find(|l| l.name == "思念");
        assert!(
            longing.is_some(),
            "应存在\"思念\"标签 / should have \"思念\" label"
        );
        assert_eq!(longing.unwrap().emoji, "🥺");
    }

    #[test]
    fn test_longing_pad_classified_as_longing() {
        // 数字生命思念状态应归类为"思念" / Digital life longing state should classify as "思念"
        let label = EmotionState::new(-0.20, -0.20, -0.30).classify();
        assert_eq!(
            label.name, "思念",
            "PAD (-0.20, -0.20, -0.30) 应为思念 / should be longing: got {}",
            label.name
        );
    }

    #[test]
    fn test_longing_near_state() {
        // 接近思念的 PAD 也应归类为"思念" / Near-longing PAD should also classify as "思念"
        let label = EmotionState::new(-0.15, -0.15, -0.25).classify();
        assert_eq!(
            label.name, "思念",
            "PAD (-0.15, -0.15, -0.25) 应为思念 / should be longing: got {}",
            label.name
        );
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
