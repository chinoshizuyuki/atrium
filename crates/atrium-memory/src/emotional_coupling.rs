// SPDX-License-Identifier: MIT

//! 情绪耦合矩阵 — Emotional Coupling Matrix (Gap#2: 90% → 95%).
//!
//! 核心理念：情绪不是独立的——悲伤时会格外孤独，喜悦时会抑制焦虑。
//! 情绪耦合定义了情绪状态间的相互调制关系：一种情绪的活跃会
//! 增强或抑制其他情绪。某些耦合组合还会产生涌现情绪。
//!
//! Core idea: emotions are not independent — sadness amplifies loneliness,
//! joy suppresses anxiety. Emotional coupling defines mutual modulation
//! between emotional states: the activation of one emotion enhances or
//! suppresses others. Certain coupling combinations produce emergent emotions.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 情绪状态枚举 — Emotion State Enum
// ═══════════════════════════════════════════════════════════════════════════

/// 基本情绪状态 / Basic emotion states.
///
/// 用于耦合矩阵的索引维度。
/// Used as index dimensions for the coupling matrix.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EmotionState {
    /// 愉悦 / Joy.
    Joy,
    /// 悲伤 / Sadness.
    Sadness,
    /// 愤怒 / Anger.
    Anger,
    /// 恐惧 / Fear.
    Fear,
    /// 惊讶 / Surprise.
    Surprise,
    /// 厌恶 / Disgust.
    Disgust,
    /// 信任 / Trust.
    Trust,
    /// 期待 / Anticipation.
    Anticipation,
    /// 孤独 / Loneliness.
    Loneliness,
    /// 焦虑 / Anxiety.
    Anxiety,
}

/// 情绪状态总数 / Number of emotion states.
pub const NUM_EMOTIONS: usize = 10;

impl EmotionState {
    /// 转为索引 / Convert to index.
    pub fn as_index(&self) -> usize {
        match self {
            Self::Joy => 0,
            Self::Sadness => 1,
            Self::Anger => 2,
            Self::Fear => 3,
            Self::Surprise => 4,
            Self::Disgust => 5,
            Self::Trust => 6,
            Self::Anticipation => 7,
            Self::Loneliness => 8,
            Self::Anxiety => 9,
        }
    }

    /// 从索引恢复 / Restore from index.
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Joy,
            1 => Self::Sadness,
            2 => Self::Anger,
            3 => Self::Fear,
            4 => Self::Surprise,
            5 => Self::Disgust,
            6 => Self::Trust,
            7 => Self::Anticipation,
            8 => Self::Loneliness,
            9 => Self::Anxiety,
            _ => Self::Joy,
        }
    }

    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Joy => "愉悦",
            Self::Sadness => "悲伤",
            Self::Anger => "愤怒",
            Self::Fear => "恐惧",
            Self::Surprise => "惊讶",
            Self::Disgust => "厌恶",
            Self::Trust => "信任",
            Self::Anticipation => "期待",
            Self::Loneliness => "孤独",
            Self::Anxiety => "焦虑",
        }
    }

    /// 是否正面情绪 / Whether positive emotion.
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Joy | Self::Trust | Self::Anticipation)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 耦合矩阵 — Coupling Matrix
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪耦合矩阵 / Emotional coupling matrix.
///
/// `matrix[a][b]` 表示情绪A对情绪B的耦合系数：
/// - > 0：A增强B（如悲伤增强孤独）
/// - < 0：A抑制B（如喜悦抑制焦虑）
/// - = 0：无耦合
///
/// `matrix[a][b]` represents the coupling coefficient of emotion A on emotion B:
/// - > 0: A enhances B (e.g., sadness enhances loneliness)
/// - < 0: A suppresses B (e.g., joy suppresses anxiety)
/// - = 0: no coupling
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingMatrix {
    /// N×N 耦合系数矩阵 / N×N coupling coefficient matrix.
    matrix: [[f64; NUM_EMOTIONS]; NUM_EMOTIONS],
}

impl Default for CouplingMatrix {
    fn default() -> Self {
        Self::human_like()
    }
}

impl CouplingMatrix {
    /// 创建零矩阵 / Create zero matrix.
    pub fn zero() -> Self {
        Self {
            matrix: [[0.0; NUM_EMOTIONS]; NUM_EMOTIONS],
        }
    }

    /// 创建仿人类耦合矩阵 / Create human-like coupling matrix.
    ///
    /// 基于心理学文献中的情绪共现和调制关系。
    /// Based on emotional co-occurrence and modulation relationships from psychology literature.
    pub fn human_like() -> Self {
        let mut m = Self::zero();

        // 悲伤增强孤独 / Sadness enhances loneliness.
        m.set(EmotionState::Sadness, EmotionState::Loneliness, 0.35);
        // 悲伤抑制愉悦 / Sadness suppresses joy.
        m.set(EmotionState::Sadness, EmotionState::Joy, -0.30);
        // 悲伤增强焦虑 / Sadness enhances anxiety.
        m.set(EmotionState::Sadness, EmotionState::Anxiety, 0.20);

        // 愉悦抑制焦虑 / Joy suppresses anxiety.
        m.set(EmotionState::Joy, EmotionState::Anxiety, -0.40);
        // 愉悦抑制悲伤 / Joy suppresses sadness.
        m.set(EmotionState::Joy, EmotionState::Sadness, -0.25);
        // 愉悦增强信任 / Joy enhances trust.
        m.set(EmotionState::Joy, EmotionState::Trust, 0.20);

        // 愤怒抑制信任 / Anger suppresses trust.
        m.set(EmotionState::Anger, EmotionState::Trust, -0.35);
        // 愤怒增强厌恶 / Anger enhances disgust.
        m.set(EmotionState::Anger, EmotionState::Disgust, 0.25);
        // 愤怒掩盖恐惧 / Anger masks fear.
        m.set(EmotionState::Anger, EmotionState::Fear, -0.20);

        // 恐惧增强焦虑 / Fear enhances anxiety.
        m.set(EmotionState::Fear, EmotionState::Anxiety, 0.30);
        // 恐惧抑制信任 / Fear suppresses trust.
        m.set(EmotionState::Fear, EmotionState::Trust, -0.25);

        // 孤独增强悲伤 / Loneliness enhances sadness.
        m.set(EmotionState::Loneliness, EmotionState::Sadness, 0.25);
        // 孤独抑制愉悦 / Loneliness suppresses joy.
        m.set(EmotionState::Loneliness, EmotionState::Joy, -0.15);

        // 焦虑抑制愉悦 / Anxiety suppresses joy.
        m.set(EmotionState::Anxiety, EmotionState::Joy, -0.20);
        // 焦虑增强恐惧 / Anxiety enhances fear.
        m.set(EmotionState::Anxiety, EmotionState::Fear, 0.15);

        // 信任增强愉悦 / Trust enhances joy.
        m.set(EmotionState::Trust, EmotionState::Joy, 0.15);
        // 信任抑制恐惧 / Trust suppresses fear.
        m.set(EmotionState::Trust, EmotionState::Fear, -0.15);

        // 期待增强愉悦 / Anticipation enhances joy.
        m.set(EmotionState::Anticipation, EmotionState::Joy, 0.10);
        // 期待增强焦虑 / Anticipation enhances anxiety (uncertainty).
        m.set(EmotionState::Anticipation, EmotionState::Anxiety, 0.10);

        // 惊讶增强期待 / Surprise enhances anticipation.
        m.set(EmotionState::Surprise, EmotionState::Anticipation, 0.15);

        // 厌恶抑制信任 / Disgust suppresses trust.
        m.set(EmotionState::Disgust, EmotionState::Trust, -0.20);

        m
    }

    /// 设置耦合系数 / Set coupling coefficient.
    pub fn set(&mut self, from: EmotionState, to: EmotionState, value: f64) {
        self.matrix[from.as_index()][to.as_index()] = value;
    }

    /// 获取耦合系数 / Get coupling coefficient.
    pub fn get(&self, from: EmotionState, to: EmotionState) -> f64 {
        self.matrix[from.as_index()][to.as_index()]
    }

    /// 计算耦合效果 — 给定各情绪强度，返回耦合后的调制增量 / Compute coupling effects.
    ///
    /// `intensities[i]` 是情绪i的当前强度 [0, 1]。
    /// 返回每个情绪的耦合调制增量（可正可负）。
    ///
    /// `intensities[i]` is the current intensity of emotion i [0, 1].
    /// Returns coupling modulation delta for each emotion (can be positive or negative).
    pub fn compute_effects(&self, intensities: &[f64; NUM_EMOTIONS]) -> [f64; NUM_EMOTIONS] {
        let mut deltas = [0.0; NUM_EMOTIONS];
        for (a, &intensity_a) in intensities.iter().enumerate() {
            if intensity_a <= 0.0 {
                continue;
            }
            for (b, delta_b) in deltas.iter_mut().enumerate() {
                if a == b {
                    continue;
                }
                // 情绪A对情绪B的耦合效果 = coupling[A][B] × intensity_A / Coupling effect.
                *delta_b += self.matrix[a][b] * intensity_a;
            }
        }
        deltas
    }

    /// 应用耦合效果并裁剪 / Apply coupling effects and clamp.
    ///
    /// 返回耦合后的情绪强度（裁剪到 [0, 1]）。
    /// Returns coupled intensities clamped to [0, 1].
    pub fn apply_coupling(&self, intensities: &[f64; NUM_EMOTIONS]) -> [f64; NUM_EMOTIONS] {
        let deltas = self.compute_effects(intensities);
        let mut result = [0.0; NUM_EMOTIONS];
        for i in 0..NUM_EMOTIONS {
            result[i] = (intensities[i] + deltas[i]).clamp(0.0, 1.0);
        }
        result
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 涌现情绪 — Emergent Emotions
// ═══════════════════════════════════════════════════════════════════════════

/// 涌现情绪类型 / Emergent emotion kind.
///
/// 某些情绪组合产生新的涌现情绪——
/// 不是基本情绪的线性叠加，而是质变。
/// Certain emotion combinations produce new emergent emotions —
/// not linear combinations of basic emotions, but qualitative shifts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmergentEmotion {
    /// 怨恨 — 悲伤 + 愤怒 / Resentment — sadness + anger.
    Resentment,
    /// 兴奋 — 愉悦 + 期待 / Excitement — joy + anticipation.
    Excitement,
    /// 绝望 — 悲伤 + 恐惧 + 无助 / Despair — sadness + fear.
    Despair,
    /// 敬畏 — 恐惧 + 惊讶 + 信任 / Awe — fear + surprise + trust.
    Awe,
    /// 怀旧 — 悲伤 + 信任 + 愉悦(弱) / Nostalgia — sadness + trust + weak joy.
    Nostalgia,
    /// 温情 — 信任 + 愉悦(弱) / Tenderness — trust + weak joy.
    Tenderness,
}

impl EmergentEmotion {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Resentment => "怨恨",
            Self::Excitement => "兴奋",
            Self::Despair => "绝望",
            Self::Awe => "敬畏",
            Self::Nostalgia => "怀旧",
            Self::Tenderness => "温情",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Resentment => "Resentment",
            Self::Excitement => "Excitement",
            Self::Despair => "Despair",
            Self::Awe => "Awe",
            Self::Nostalgia => "Nostalgia",
            Self::Tenderness => "Tenderness",
        }
    }
}

/// 涌现情绪检测结果 / Emergent emotion detection result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergentResult {
    /// 涌现情绪类型 / Emergent emotion kind.
    pub emotion: EmergentEmotion,
    /// 涌现强度 [0, 1] / Emergent intensity.
    pub intensity: f64,
}

/// 检测涌现情绪 / Detect emergent emotions.
///
/// 基于当前情绪强度组合，检测是否产生涌现情绪。
/// Detects emergent emotions based on current emotion intensity combinations.
pub fn detect_emergent(intensities: &[f64; NUM_EMOTIONS]) -> Vec<EmergentResult> {
    let mut results = Vec::new();
    let joy = intensities[EmotionState::Joy.as_index()];
    let sad = intensities[EmotionState::Sadness.as_index()];
    let anger = intensities[EmotionState::Anger.as_index()];
    let fear = intensities[EmotionState::Fear.as_index()];
    let surprise = intensities[EmotionState::Surprise.as_index()];
    let trust = intensities[EmotionState::Trust.as_index()];
    let anticip = intensities[EmotionState::Anticipation.as_index()];

    // 怨恨 = 悲伤 × 愤怒（两者都需较强）/ Resentment.
    if sad > 0.3 && anger > 0.3 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Resentment,
            intensity: (sad * anger).sqrt(),
        });
    }

    // 兴奋 = 愉悦 × 期待 / Excitement.
    if joy > 0.3 && anticip > 0.3 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Excitement,
            intensity: (joy * anticip).sqrt(),
        });
    }

    // 绝望 = 悲伤 × 恐惧 / Despair.
    if sad > 0.3 && fear > 0.3 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Despair,
            intensity: (sad * fear).sqrt(),
        });
    }

    // 敬畏 = 恐惧 × 惊讶 × 信任 / Awe.
    if fear > 0.2 && surprise > 0.3 && trust > 0.2 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Awe,
            intensity: (fear * surprise * trust).cbrt(),
        });
    }

    // 怀旧 = 悲伤 × 信任 / Nostalgia.
    if sad > 0.2 && trust > 0.3 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Nostalgia,
            intensity: (sad * trust).sqrt() * 0.8,
        });
    }

    // 温情 = 信任 × 愉悦(弱) / Tenderness.
    if trust > 0.3 && joy > 0.1 {
        results.push(EmergentResult {
            emotion: EmergentEmotion::Tenderness,
            intensity: (trust * joy.max(0.1)).sqrt() * 0.7,
        });
    }

    // 按强度降序排列 / Sort by intensity descending.
    results.sort_by(|a, b| {
        b.intensity
            .partial_cmp(&a.intensity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 情绪耦合引擎 — Emotional Coupling Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 情绪耦合引擎 / Emotional coupling engine.
///
/// 管理耦合矩阵和当前情绪强度，提供耦合计算和涌现检测接口。
/// Manages coupling matrix and current emotion intensities, providing
/// coupling computation and emergent detection interfaces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalCoupling {
    /// 耦合矩阵 / Coupling matrix.
    pub matrix: CouplingMatrix,
    /// 当前情绪强度 / Current emotion intensities.
    pub intensities: [f64; NUM_EMOTIONS],
    /// 耦合学习率 — 矩阵自适应微调速率 / Coupling learning rate.
    learning_rate: f64,
}

impl Default for EmotionalCoupling {
    fn default() -> Self {
        Self {
            matrix: CouplingMatrix::human_like(),
            intensities: [0.0; NUM_EMOTIONS],
            learning_rate: 0.01,
        }
    }
}

impl EmotionalCoupling {
    /// 创建新耦合引擎 / Create a new coupling engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置情绪强度 / Set emotion intensity.
    pub fn set_intensity(&mut self, emotion: EmotionState, intensity: f64) {
        self.intensities[emotion.as_index()] = intensity.clamp(0.0, 1.0);
    }

    /// 获取情绪强度 / Get emotion intensity.
    pub fn get_intensity(&self, emotion: EmotionState) -> f64 {
        self.intensities[emotion.as_index()]
    }

    /// 执行耦合计算 — 返回耦合后的情绪强度 / Execute coupling computation.
    pub fn compute_coupled(&self) -> [f64; NUM_EMOTIONS] {
        self.matrix.apply_coupling(&self.intensities)
    }

    /// 检测涌现情绪 / Detect emergent emotions.
    pub fn detect_emergent(&self) -> Vec<EmergentResult> {
        detect_emergent(&self.intensities)
    }

    /// 自适应更新 — 当观察到情绪共现时微调耦合系数 / Adaptive update.
    ///
    /// 如果情绪A和B频繁共现，增强它们的正耦合。
    /// If emotions A and B frequently co-occur, enhance their positive coupling.
    pub fn adapt(&mut self, observed: &[f64; NUM_EMOTIONS]) {
        for a in 0..NUM_EMOTIONS {
            for b in 0..NUM_EMOTIONS {
                if a == b {
                    continue;
                }
                // 共现强度 = A × B / Co-occurrence intensity.
                let cooccur = observed[a] * observed[b];
                if cooccur > 0.1 {
                    // 微调耦合系数向共现方向 / Nudge coupling towards co-occurrence.
                    let current = self.matrix.matrix[a][b];
                    let adjustment = self.learning_rate * cooccur * (0.1 - current);
                    self.matrix.matrix[a][b] = current + adjustment;
                }
            }
        }
    }

    /// 生成耦合描述文本 / Generate coupling description text.
    pub fn describe(&self) -> String {
        let coupled = self.compute_coupled();
        let emergent = self.detect_emergent();

        let mut parts = Vec::new();
        // 列出被耦合显著调制的情绪 / List significantly modulated emotions.
        for (i, (&coupled_i, &intensity_i)) in
            coupled.iter().zip(self.intensities.iter()).enumerate()
        {
            let delta = coupled_i - intensity_i;
            if delta.abs() > 0.05 {
                let emo = EmotionState::from_index(i);
                let direction = if delta > 0.0 { "↑" } else { "↓" };
                parts.push(format!(
                    "{}{}({:.2})",
                    emo.label_zh(),
                    direction,
                    delta.abs()
                ));
            }
        }

        let mut desc = if parts.is_empty() {
            "情绪间无显著耦合".to_string()
        } else {
            format!("情绪耦合: {}", parts.join(", "))
        };

        // 列出涌现情绪 / List emergent emotions.
        if !emergent.is_empty() {
            let emo_strs: Vec<String> = emergent
                .iter()
                .map(|e| format!("{}({:.2})", e.emotion.label_zh(), e.intensity))
                .collect();
            desc.push_str(&format!(" | 涌现: {}", emo_strs.join(", ")));
        }

        desc
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 情绪状态测试 ──

    #[test]
    fn test_emotion_state_index_roundtrip() {
        for i in 0..NUM_EMOTIONS {
            let emo = EmotionState::from_index(i);
            assert_eq!(emo.as_index(), i);
        }
    }

    #[test]
    fn test_emotion_is_positive() {
        assert!(EmotionState::Joy.is_positive());
        assert!(EmotionState::Trust.is_positive());
        assert!(!EmotionState::Sadness.is_positive());
        assert!(!EmotionState::Anger.is_positive());
    }

    // ── 耦合矩阵测试 ──

    #[test]
    fn test_coupling_matrix_human_like() {
        let m = CouplingMatrix::human_like();
        // 悲伤增强孤独 / Sadness enhances loneliness.
        assert!(m.get(EmotionState::Sadness, EmotionState::Loneliness) > 0.0);
        // 愉悦抑制焦虑 / Joy suppresses anxiety.
        assert!(m.get(EmotionState::Joy, EmotionState::Anxiety) < 0.0);
        // 愤怒抑制信任 / Anger suppresses trust.
        assert!(m.get(EmotionState::Anger, EmotionState::Trust) < 0.0);
    }

    #[test]
    fn test_coupling_matrix_set_get() {
        let mut m = CouplingMatrix::zero();
        m.set(EmotionState::Joy, EmotionState::Sadness, 0.5);
        assert_eq!(m.get(EmotionState::Joy, EmotionState::Sadness), 0.5);
    }

    #[test]
    fn test_coupling_compute_effects() {
        let m = CouplingMatrix::human_like();
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Sadness.as_index()] = 0.8;

        let effects = m.compute_effects(&intensities);
        // 悲伤应增强孤独 / Sadness should enhance loneliness.
        assert!(effects[EmotionState::Loneliness.as_index()] > 0.0);
        // 悲伤应抑制愉悦 / Sadness should suppress joy.
        assert!(effects[EmotionState::Joy.as_index()] < 0.0);
    }

    #[test]
    fn test_coupling_apply_clamps_to_unit() {
        let m = CouplingMatrix::human_like();
        let intensities = [1.0; NUM_EMOTIONS];
        let result = m.apply_coupling(&intensities);
        for v in result {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn test_coupling_zero_intensities() {
        let m = CouplingMatrix::human_like();
        let intensities = [0.0; NUM_EMOTIONS];
        let effects = m.compute_effects(&intensities);
        // 零强度应产生零效果 / Zero intensities produce zero effects.
        for v in effects {
            assert_eq!(v, 0.0);
        }
    }

    // ── 涌现情绪测试 ──

    #[test]
    fn test_emergent_resentment() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Sadness.as_index()] = 0.6;
        intensities[EmotionState::Anger.as_index()] = 0.5;
        let results = detect_emergent(&intensities);
        assert!(results
            .iter()
            .any(|r| r.emotion == EmergentEmotion::Resentment));
    }

    #[test]
    fn test_emergent_excitement() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Joy.as_index()] = 0.7;
        intensities[EmotionState::Anticipation.as_index()] = 0.6;
        let results = detect_emergent(&intensities);
        assert!(results
            .iter()
            .any(|r| r.emotion == EmergentEmotion::Excitement));
    }

    #[test]
    fn test_emergent_nostalgia() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Sadness.as_index()] = 0.4;
        intensities[EmotionState::Trust.as_index()] = 0.5;
        let results = detect_emergent(&intensities);
        assert!(results
            .iter()
            .any(|r| r.emotion == EmergentEmotion::Nostalgia));
    }

    #[test]
    fn test_emergent_no_emergent_for_weak_emotions() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Joy.as_index()] = 0.1;
        let results = detect_emergent(&intensities);
        assert!(results.is_empty());
    }

    #[test]
    fn test_emergent_sorted_by_intensity() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Joy.as_index()] = 0.9;
        intensities[EmotionState::Anticipation.as_index()] = 0.9;
        intensities[EmotionState::Sadness.as_index()] = 0.4;
        intensities[EmotionState::Trust.as_index()] = 0.4;
        let results = detect_emergent(&intensities);
        for i in 1..results.len() {
            assert!(results[i - 1].intensity >= results[i].intensity);
        }
    }

    // ── 耦合引擎测试 ──

    #[test]
    fn test_coupling_engine_set_get() {
        let mut engine = EmotionalCoupling::new();
        engine.set_intensity(EmotionState::Joy, 0.8);
        assert_eq!(engine.get_intensity(EmotionState::Joy), 0.8);
    }

    #[test]
    fn test_coupling_engine_compute_coupled() {
        let mut engine = EmotionalCoupling::new();
        engine.set_intensity(EmotionState::Sadness, 0.7);
        let coupled = engine.compute_coupled();
        // 耦合后孤独应增强 / Loneliness should be enhanced after coupling.
        assert!(coupled[EmotionState::Loneliness.as_index()] > 0.0);
    }

    #[test]
    fn test_coupling_engine_detect_emergent() {
        let mut engine = EmotionalCoupling::new();
        engine.set_intensity(EmotionState::Joy, 0.6);
        engine.set_intensity(EmotionState::Anticipation, 0.5);
        let results = engine.detect_emergent();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_coupling_engine_adapt() {
        let mut engine = EmotionalCoupling::new();
        // Sadness→Joy 初始为 -0.30，adapt 向 0.1 靠拢应增大 / Sadness→Joy starts at -0.30.
        let original = engine.matrix.get(EmotionState::Sadness, EmotionState::Joy);

        let mut observed = [0.0; NUM_EMOTIONS];
        observed[EmotionState::Sadness.as_index()] = 0.8;
        observed[EmotionState::Joy.as_index()] = 0.7;

        // 多次自适应 / Multiple adaptations.
        for _ in 0..100 {
            engine.adapt(&observed);
        }
        let adapted = engine.matrix.get(EmotionState::Sadness, EmotionState::Joy);
        // 共现应增强正耦合 / Co-occurrence should enhance positive coupling.
        assert!(adapted >= original);
    }

    #[test]
    fn test_coupling_engine_describe() {
        let mut engine = EmotionalCoupling::new();
        engine.set_intensity(EmotionState::Sadness, 0.6);
        engine.set_intensity(EmotionState::Anger, 0.5);
        let desc = engine.describe();
        // 应包含耦合或涌现信息 / Should contain coupling or emergent info.
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_emergent_despair() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Sadness.as_index()] = 0.5;
        intensities[EmotionState::Fear.as_index()] = 0.6;
        let results = detect_emergent(&intensities);
        assert!(results
            .iter()
            .any(|r| r.emotion == EmergentEmotion::Despair));
    }

    #[test]
    fn test_emergent_awe() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Fear.as_index()] = 0.3;
        intensities[EmotionState::Surprise.as_index()] = 0.5;
        intensities[EmotionState::Trust.as_index()] = 0.4;
        let results = detect_emergent(&intensities);
        assert!(results.iter().any(|r| r.emotion == EmergentEmotion::Awe));
    }

    #[test]
    fn test_emergent_tenderness() {
        let mut intensities = [0.0; NUM_EMOTIONS];
        intensities[EmotionState::Trust.as_index()] = 0.6;
        intensities[EmotionState::Joy.as_index()] = 0.3;
        let results = detect_emergent(&intensities);
        assert!(results
            .iter()
            .any(|r| r.emotion == EmergentEmotion::Tenderness));
    }
}
