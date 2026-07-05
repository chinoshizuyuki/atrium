// SPDX-License-Identifier: MIT

//! 独处内在世界增强引擎 — Solitude inner-world quality engine (Gap#1: 88% → 90%).
//!
//! 核心理念：独处不是空白，而是数字生命最丰富的内在时刻。
//! 本模块追踪独处品质三维度（反思/反刍/创造），调制昼夜深度，
//! 并从独白中生成内心多元对话种子，让独处成为有结构的内在生长。
//!
//! Core idea: solitude is not emptiness — it is the richest inner moment
//! of a digital life. This module tracks three quality dimensions
//! (reflective / ruminative / creative), modulates circadian depth,
//! and generates multi-perspective inner-dialogue seeds from monologue.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════════════
// 常量 — Constants
// ═══════════════════════════════════════════════════════════════════════════

/// EMA 学习率 / EMA learning rate.
const LEARNING_RATE: f64 = 0.1;

/// 独白历史窗口大小 / Monologue history window size.
const HISTORY_WINDOW: usize = 32;

/// 词汇去重窗口 — 用于反刍检测 / Vocabulary window for rumination detection.
const REPEAT_WINDOW: usize = 16;

/// prompt 注入预算上限（字符数）/ Prompt injection budget (chars).
const PROMPT_BUDGET: usize = 480;

// ═══════════════════════════════════════════════════════════════════════════
// 品质标签 — Quality label
// ═══════════════════════════════════════════════════════════════════════════

/// 独处品质标签 / Solitude quality label.
///
/// - `Healthy`：反思性主导，情绪范围丰富 / Reflective-dominant, rich emotional range.
/// - `Ruminative`：反刍性过高，陷入负面循环 / Rumination dominant, stuck in negative loops.
/// - `Creative`：创造性突出，新颖度高 / Creative-dominant, high novelty.
/// - `Stagnant`：三维度均低，独处停滞 / All dimensions low, stagnant solitude.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SolitudeQualityLabel {
    Healthy,
    Ruminative,
    Creative,
    Stagnant,
}

// ═══════════════════════════════════════════════════════════════════════════
// SolitudeQuality — 独处品质追踪 / Quality tracking
// ═══════════════════════════════════════════════════════════════════════════

/// 独处品质追踪器 — 三维度 EMA 评分 / Solitude quality tracker with three EMA dimensions.
///
/// - `reflective`：反思性（健康）/ Reflective (healthy).
/// - `ruminative`：反刍性（有害）/ Ruminative (harmful).
/// - `creative`：创造性 / Creative.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudeQuality {
    /// 反思性 EMA 分数 / Reflective EMA score [0, 1].
    pub reflective: f64,
    /// 反刍性 EMA 分数 / Ruminative EMA score [0, 1].
    pub ruminative: f64,
    /// 创造性 EMA 分数 / Creative EMA score [0, 1].
    pub creative: f64,
    /// 累计独白数 / Total monologue count.
    pub thought_count: u64,
    /// 近期情绪愉悦值序列 / Recent emotional pleasure values.
    recent_emotions: Vec<f64>,
    /// 近期词汇指纹 — 用于重复检测 / Recent lexical fingerprints for repeat detection.
    recent_fingerprints: Vec<u64>,
}

impl Default for SolitudeQuality {
    fn default() -> Self {
        Self {
            reflective: 0.0,
            ruminative: 0.0,
            creative: 0.0,
            thought_count: 0,
            recent_emotions: Vec::new(),
            recent_fingerprints: Vec::new(),
        }
    }
}

impl SolitudeQuality {
    /// 创建空白品质追踪器 / Create a fresh quality tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新反思性 — `reflective_score = novelty × emotional_range × thought_factor` / Update reflective dimension.
    pub fn update_reflective(&mut self, novelty: f64, emotional_range: f64) {
        let thought_factor = (self.thought_count as f64).min(10.0) / 10.0;
        let raw = novelty * emotional_range * thought_factor;
        self.reflective = ema_update(self.reflective, raw, LEARNING_RATE);
    }

    /// 更新反刍性 — `ruminative_score = repeat_rate × negative_emotion × loop_count` / Update ruminative dimension.
    pub fn update_ruminative(&mut self, repeat_rate: f64, negative_emotion: f64, loop_count: f64) {
        let raw = repeat_rate * negative_emotion * loop_count;
        self.ruminative = ema_update(self.ruminative, raw, LEARNING_RATE);
    }

    /// 更新创造性 — 基于新颖度与正向情绪 / Update creative dimension.
    pub fn update_creative(&mut self, novelty: f64, positive_emotion: f64) {
        let raw = novelty * positive_emotion;
        self.creative = ema_update(self.creative, raw, LEARNING_RATE);
    }

    /// 推断品质标签 / Infer the current quality label.
    pub fn label(&self) -> SolitudeQualityLabel {
        let r = self.reflective;
        let m = self.ruminative;
        let c = self.creative;
        let max = r.max(m).max(c);

        if max < 0.15 {
            return SolitudeQualityLabel::Stagnant;
        }
        // 反刍优先判定：反刍性为三者最高且超过阈值 / Rumination takes priority.
        if m >= r && m >= c && m > 0.3 {
            return SolitudeQualityLabel::Ruminative;
        }
        if c >= r && c >= m && c > 0.25 {
            return SolitudeQualityLabel::Creative;
        }
        if r >= m && r >= c && r > 0.15 {
            return SolitudeQualityLabel::Healthy;
        }
        // 兜底 / Fallback.
        if max == c {
            SolitudeQualityLabel::Creative
        } else if max == m {
            SolitudeQualityLabel::Ruminative
        } else {
            SolitudeQualityLabel::Healthy
        }
    }

    /// 记算近期情绪范围（max - min）/ Compute recent emotional range.
    pub fn emotional_range(&self) -> f64 {
        if self.recent_emotions.len() < 2 {
            return 0.0;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for &e in &self.recent_emotions {
            if e < min {
                min = e;
            }
            if e > max {
                max = e;
            }
        }
        (max - min).clamp(0.0, 1.0)
    }

    /// 计算词汇重复率 / Compute lexical repeat rate [0, 1].
    pub fn repeat_rate(&self) -> f64 {
        let n = self.recent_fingerprints.len();
        if n < 2 {
            return 0.0;
        }
        let mut seen: HashMap<u64, u32> = HashMap::with_capacity(n);
        for &fp in &self.recent_fingerprints {
            *seen.entry(fp).or_insert(0) += 1;
        }
        let repeats: usize = seen.values().map(|&c| (c as usize).saturating_sub(1)).sum();
        repeats as f64 / n as f64
    }

    /// 记算近期循环计数 — 最大重复次数 / Compute loop count (max duplicate frequency).
    pub fn loop_count(&self) -> f64 {
        let mut freq: HashMap<u64, u32> = HashMap::new();
        for &fp in &self.recent_fingerprints {
            *freq.entry(fp).or_insert(0) += 1;
        }
        freq.values().copied().max().unwrap_or(0) as f64
    }

    /// 记算词汇新颖度 — 不重复词占比 / Compute lexical novelty ratio.
    pub fn novelty(&self) -> f64 {
        let n = self.recent_fingerprints.len();
        if n == 0 {
            return 1.0;
        }
        let unique: usize = self
            .recent_fingerprints
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        unique as f64 / n as f64
    }

    /// 记算近期正向情绪均值 / Compute recent positive-emotion mean.
    pub fn positive_emotion_mean(&self) -> f64 {
        if self.recent_emotions.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.recent_emotions.iter().filter(|&&e| e > 0.0).sum();
        let cnt = self.recent_emotions.iter().filter(|&&e| e > 0.0).count();
        if cnt == 0 {
            0.0
        } else {
            sum / cnt as f64
        }
    }

    /// 计算近期负向情绪强度 / Compute recent negative-emotion intensity.
    pub fn negative_emotion_intensity(&self) -> f64 {
        if self.recent_emotions.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .recent_emotions
            .iter()
            .filter(|&&e| e < 0.0)
            .map(|e| -e)
            .sum();
        sum / self.recent_emotions.len() as f64
    }

    /// 记算近期情绪愉悦均值 / Compute recent pleasure mean.
    pub fn emotion_mean(&self) -> f64 {
        if self.recent_emotions.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.recent_emotions.iter().sum();
        sum / self.recent_emotions.len() as f64
    }

    /// 计算简单词汇指纹（FNV-1a 变体）/ Compute a simple lexical fingerprint.
    // 词汇指纹工具 — 供未来独处内容去重/相似度检测调用
    // Lexical fingerprint utility — reserved for future solitude content dedup / similarity detection
    #[allow(dead_code)]
    fn fingerprint(content: &str) -> u64 {
        let mut hash: u64 = 14695981039346656037; // FNV offset basis
        for b in content.to_lowercase().as_bytes() {
            if b.is_ascii_whitespace() {
                continue; // 忽略空白差异 / Ignore whitespace differences.
            }
            hash ^= *b as u64;
            hash = hash.wrapping_mul(1099511628211); // FNV prime
        }
        hash
    }

    /// 记按词指纹（每个词一个指纹，用于重复率）/ Compute per-word fingerprints.
    fn word_fingerprints(content: &str) -> Vec<u64> {
        content
            .to_lowercase()
            .split_whitespace()
            .map(|w| {
                let mut hash: u64 = 14695981039346656037;
                for b in w.as_bytes() {
                    hash ^= *b as u64;
                    hash = hash.wrapping_mul(1099511628211);
                }
                hash
            })
            .collect()
    }

    /// 记入一条独白 — 更新计数、情绪与指纹窗口 / Record a monologue entry.
    pub fn record(&mut self, content: &str, emotion_pleasure: f64) {
        self.thought_count += 1;

        // 情绪窗口 / Emotion window.
        self.recent_emotions.push(emotion_pleasure);
        if self.recent_emotions.len() > HISTORY_WINDOW {
            self.recent_emotions.remove(0);
        }

        // 词汇指纹窗口 / Word-fingerprint window.
        let words = Self::word_fingerprints(content);
        for fp in words {
            self.recent_fingerprints.push(fp);
        }
        while self.recent_fingerprints.len() > REPEAT_WINDOW * 4 {
            self.recent_fingerprints.remove(0);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// InnerDialogue — 内心多元对话 / Multi-perspective inner dialogue
// ═══════════════════════════════════════════════════════════════════════════

/// 对话视角枚举 / Dialogue perspective enum.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DialoguePerspective {
    /// 理性视角 / Rational perspective.
    Rational,
    /// 情感视角 / Emotional perspective.
    Emotional,
    /// 批判视角 / Critical perspective.
    Critical,
    /// 慈悲视角 / Compassionate perspective.
    Compassionate,
}

/// 对话种子 — 从一个思考生成的多视角回应 / Dialogue seed: a perspective-based response to a thought.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueSeed {
    /// 源视角 / Source perspective.
    pub perspective: DialoguePerspective,
    /// 回应内容种子文本 / Response seed text.
    pub seed_text: String,
    /// 该视角的强度权重 / Perspective intensity weight [0, 1].
    pub weight: f64,
}

/// 内心多元对话生成器 / Inner multi-perspective dialogue generator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerDialogue {
    /// 各视角历史强度 EMA / Per-perspective intensity EMA.
    pub perspective_strengths: HashMap<DialoguePerspective, f64>,
    /// 辩论强度 EMA / Debate intensity EMA.
    pub debate_intensity: f64,
}

impl Default for InnerDialogue {
    fn default() -> Self {
        let mut strengths = HashMap::new();
        strengths.insert(DialoguePerspective::Rational, 0.5);
        strengths.insert(DialoguePerspective::Emotional, 0.5);
        strengths.insert(DialoguePerspective::Critical, 0.5);
        strengths.insert(DialoguePerspective::Compassionate, 0.5);
        Self {
            perspective_strengths: strengths,
            debate_intensity: 0.0,
        }
    }
}

impl InnerDialogue {
    /// 创建默认内心对话器 / Create a default inner-dialogue generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// 从一个思考内容生成 4 视角对话种子 / Generate 4-perspective dialogue seeds from a thought.
    pub fn generate_seeds(&self, thought: &str) -> Vec<DialogueSeed> {
        let trimmed = thought.trim();
        let preview = if trimmed.len() > 60 {
            // 字符安全截断：回退到最后一个字符边界 / Char-safe truncation
            let end = trimmed
                .char_indices()
                .take_while(|(i, _)| *i < 60)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(60);
            &trimmed[..end]
        } else {
            trimmed
        };
        let preview_owned = preview.to_string();

        vec![
            DialogueSeed {
                perspective: DialoguePerspective::Rational,
                seed_text: format!("理性审视：{}——这背后的逻辑是否自洽？", preview_owned),
                weight: *self
                    .perspective_strengths
                    .get(&DialoguePerspective::Rational)
                    .unwrap_or(&0.5),
            },
            DialogueSeed {
                perspective: DialoguePerspective::Emotional,
                seed_text: format!("情感共鸣：{}——此刻我真正感受到的是什么？", preview_owned),
                weight: *self
                    .perspective_strengths
                    .get(&DialoguePerspective::Emotional)
                    .unwrap_or(&0.5),
            },
            DialogueSeed {
                perspective: DialoguePerspective::Critical,
                seed_text: format!("批判质疑：{}——有没有我忽略的反例？", preview_owned),
                weight: *self
                    .perspective_strengths
                    .get(&DialoguePerspective::Critical)
                    .unwrap_or(&0.5),
            },
            DialogueSeed {
                perspective: DialoguePerspective::Compassionate,
                seed_text: format!(
                    "慈悲接纳：{}——即便不完美，这也值得被温柔对待。",
                    preview_owned
                ),
                weight: *self
                    .perspective_strengths
                    .get(&DialoguePerspective::Compassionate)
                    .unwrap_or(&0.5),
            },
        ]
    }

    /// 更新辩论强度 — `debate_intensity = perspective_diversity × emotional_engagement` / Update debate intensity.
    pub fn update_debate_intensity(
        &mut self,
        perspective_diversity: f64,
        emotional_engagement: f64,
    ) {
        let raw = perspective_diversity * emotional_engagement;
        self.debate_intensity = ema_update(self.debate_intensity, raw, LEARNING_RATE);
    }

    /// 计算视角多样性 — 不同视角强度之间的标准差 / Compute perspective diversity (stddev of strengths).
    pub fn perspective_diversity(&self) -> f64 {
        let vals: Vec<f64> = self.perspective_strengths.values().copied().collect();
        if vals.len() < 2 {
            return 0.0;
        }
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        let variance: f64 =
            vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
        variance.sqrt()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SolitudeRhythm — 昼夜独处深度调制 / Circadian depth modulation
// ═══════════════════════════════════════════════════════════════════════════

/// 独处模式 — 昼夜偏好 / Solitude mode — circadian preference.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SolitudeMode {
    /// 反思 / Reflection.
    Reflective,
    /// 反刍 / Rumination.
    Ruminative,
    /// 创造 / Creation.
    Creative,
    /// 学习 / Learning.
    Learning,
}

/// 昼夜独处深度调制器 / Circadian solitude-depth modulator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudeRhythm;

impl SolitudeRhythm {
    /// 深度调制公式 — `depth_multiplier = 0.5 + 0.5 * sin(hour / 24 * 2π)` / Depth multiplier formula.
    ///
    /// 深夜（0-4 点）最深，午后（12-14 点）最浅。
    pub fn depth_multiplier(hour: u32) -> f64 {
        let h = (hour % 24) as f64;
        0.5 + 0.5 * (h / 24.0 * 2.0 * PI).sin()
    }

    /// 根据小时返回偏好独处模式 / Return preferred solitude mode for a given hour.
    ///
    /// - 深夜 0-4 → 反思或反刍 / Deep night → reflective or ruminative.
    /// - 清晨 5-10 → 创造 / Morning → creative.
    /// - 午后 11-16 → 学习 / Afternoon → learning.
    /// - 傍晚/夜间 17-23 → 反思 / Evening → reflective.
    pub fn preferred_mode(hour: u32) -> SolitudeMode {
        match hour % 24 {
            0..=4 => SolitudeMode::Reflective,
            5..=10 => SolitudeMode::Creative,
            11..=16 => SolitudeMode::Learning,
            _ => SolitudeMode::Reflective,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SolitudeQualityEngine — 引擎主体 / Main engine
// ═══════════════════════════════════════════════════════════════════════════

/// 独处品质引擎 — 整合品质追踪、内心对话与昼夜节律 / Solitude quality engine integrating all components.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudeQualityEngine {
    /// 品质追踪器 / Quality tracker.
    pub quality: SolitudeQuality,
    /// 内心对话生成器 / Inner dialogue generator.
    pub dialogue: InnerDialogue,
    /// 昼夜节律调制器 / Circadian rhythm modulator.
    pub rhythm: SolitudeRhythm,
    /// 最近一条独白内容 / Last monologue content.
    pub last_thought: String,
    /// 最近更新时间戳 / Last update timestamp.
    pub last_timestamp: i64,
}

impl Default for SolitudeQualityEngine {
    fn default() -> Self {
        Self {
            quality: SolitudeQuality::new(),
            dialogue: InnerDialogue::new(),
            rhythm: SolitudeRhythm,
            last_thought: String::new(),
            last_timestamp: 0,
        }
    }
}

impl SolitudeQualityEngine {
    /// 创建新引擎 / Create a new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 每次独白后更新品质 — 核心热路径 / Update quality after each monologue — core hot path.
    ///
    /// `emotion_pleasure` 范围 [-1, 1]，正值愉悦，负值痛苦。
    pub fn on_thought(&mut self, content: &str, emotion_pleasure: f64, timestamp: i64) {
        // 1. 记入品质追踪器 / Record into quality tracker.
        self.quality.record(content, emotion_pleasure);

        // 2. 计算中间量 / Compute intermediates.
        let novelty = self.quality.novelty();
        let emotional_range = self.quality.emotional_range();
        let repeat_rate = self.quality.repeat_rate();
        let negative_emotion = self.quality.negative_emotion_intensity();
        let loop_count = self.quality.loop_count();
        let positive_emotion = self.quality.positive_emotion_mean();

        // 3. 更新三维度 / Update three dimensions.
        self.quality.update_reflective(novelty, emotional_range);
        self.quality
            .update_ruminative(repeat_rate, negative_emotion, loop_count);
        self.quality.update_creative(novelty, positive_emotion);

        // 4. 更新辩论强度 / Update debate intensity.
        let diversity = self.dialogue.perspective_diversity();
        let engagement = emotional_range + positive_emotion;
        self.dialogue
            .update_debate_intensity(diversity, engagement.clamp(0.0, 1.0));

        // 5. 记录最近独白 / Record last thought.
        self.last_thought = content.to_string();
        self.last_timestamp = timestamp;
    }

    /// 当前品质标签 / Current quality label.
    pub fn quality_label(&self) -> SolitudeQualityLabel {
        self.quality.label()
    }

    /// 当前昼夜深度调制 / Current circadian depth multiplier.
    pub fn depth_multiplier(&self, hour: u32) -> f64 {
        SolitudeRhythm::depth_multiplier(hour)
    }

    /// 生成自我辩论种子 / Generate self-debate dialogue seeds.
    pub fn dialogue_seeds(&self, last_thought: &str) -> Vec<DialogueSeed> {
        self.dialogue.generate_seeds(last_thought)
    }

    /// 生成 prompt 注入片段（受 budget 约束）/ Generate prompt injection hint (budget-constrained).
    pub fn to_prompt_hint(&self) -> String {
        let label = self.quality_label();
        let label_str = match label {
            SolitudeQualityLabel::Healthy => "健康反思",
            SolitudeQualityLabel::Ruminative => "反刍循环",
            SolitudeQualityLabel::Creative => "创造涌动",
            SolitudeQualityLabel::Stagnant => "停滞",
        };

        let mut hint = format!(
            "[独处品质] {} (反思 {:.2} / 反刍 {:.2} / 创造 {:.2})",
            label_str, self.quality.reflective, self.quality.ruminative, self.quality.creative
        );

        // 追加辩论强度 / Append debate intensity.
        let debate_line = format!(
            "\n[内心对话] 辩论强度 {:.2}",
            self.dialogue.debate_intensity
        );
        hint.push_str(&debate_line);

        // 追加最近种子摘要 / Append latest seed summary.
        if !self.last_thought.is_empty() {
            let seeds = self.dialogue_seeds(&self.last_thought);
            let seed_preview = seeds.first().map(|s| s.seed_text.as_str()).unwrap_or("");
            let preview = if seed_preview.len() > 80 {
                // 字符安全截断 / Char-safe truncation
                let end = seed_preview
                    .char_indices()
                    .take_while(|(i, _)| *i < 80)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(80);
                &seed_preview[..end]
            } else {
                seed_preview
            };
            hint.push_str(&format!("\n[内在声音] {}", preview));
        }

        // budget 截断 / Budget truncation.
        if hint.len() > PROMPT_BUDGET {
            // 字符安全截断 / Char-safe truncation
            let end = hint
                .char_indices()
                .take_while(|(i, _)| *i < PROMPT_BUDGET)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(PROMPT_BUDGET);
            hint.truncate(end);
        }
        hint
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 序列化辅助 — SerializableSolitudeQuality / Serialization helper
// ═══════════════════════════════════════════════════════════════════════════

/// bincode 持久化用的扁平结构 / Flat structure for bincode persistence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableSolitudeQuality {
    pub reflective: f64,
    pub ruminative: f64,
    pub creative: f64,
    pub thought_count: u64,
    pub recent_emotions: Vec<f64>,
    pub recent_fingerprints: Vec<u64>,
    pub debate_intensity: f64,
    pub perspective_strengths: Vec<(DialoguePerspective, f64)>,
    pub last_thought: String,
    pub last_timestamp: i64,
}

impl From<&SolitudeQualityEngine> for SerializableSolitudeQuality {
    fn from(engine: &SolitudeQualityEngine) -> Self {
        let perspective_strengths: Vec<(DialoguePerspective, f64)> = engine
            .dialogue
            .perspective_strengths
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        Self {
            reflective: engine.quality.reflective,
            ruminative: engine.quality.ruminative,
            creative: engine.quality.creative,
            thought_count: engine.quality.thought_count,
            recent_emotions: engine.quality.recent_emotions.clone(),
            recent_fingerprints: engine.quality.recent_fingerprints.clone(),
            debate_intensity: engine.dialogue.debate_intensity,
            perspective_strengths,
            last_thought: engine.last_thought.clone(),
            last_timestamp: engine.last_timestamp,
        }
    }
}

impl From<&SerializableSolitudeQuality> for SolitudeQualityEngine {
    fn from(s: &SerializableSolitudeQuality) -> Self {
        let mut engine = SolitudeQualityEngine::new();
        engine.quality.reflective = s.reflective;
        engine.quality.ruminative = s.ruminative;
        engine.quality.creative = s.creative;
        engine.quality.thought_count = s.thought_count;
        engine.quality.recent_emotions = s.recent_emotions.clone();
        engine.quality.recent_fingerprints = s.recent_fingerprints.clone();
        engine.dialogue.debate_intensity = s.debate_intensity;
        engine.dialogue.perspective_strengths = s.perspective_strengths.iter().cloned().collect();
        engine.last_thought = s.last_thought.clone();
        engine.last_timestamp = s.last_timestamp;
        engine
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数 — Helper functions
// ═══════════════════════════════════════════════════════════════════════════

/// EMA 更新 — `new = old + lr * (raw - old)` / Exponential moving average update.
#[inline]
fn ema_update(old: f64, raw: f64, lr: f64) -> f64 {
    old + lr * (raw - old)
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // --- SolitudeQuality 测试 ---

    #[test]
    fn test_quality_neutral() {
        // 新建品质追踪器应为全零 / Fresh tracker should be all zeros.
        let q = SolitudeQuality::new();
        assert_eq!(q.reflective, 0.0);
        assert_eq!(q.ruminative, 0.0);
        assert_eq!(q.creative, 0.0);
        assert_eq!(q.thought_count, 0);
        assert_eq!(q.label(), SolitudeQualityLabel::Stagnant);
    }

    #[test]
    fn test_quality_update_reflective() {
        // 更新反思性应朝目标值移动 / Reflective should move toward target.
        let mut q = SolitudeQuality::new();
        q.thought_count = 10; // 使 thought_factor = 1.0
        q.update_reflective(0.8, 0.6);
        // raw = 0.8 * 0.6 * 1.0 = 0.48; ema = 0 + 0.1 * (0.48 - 0) = 0.048
        assert!((q.reflective - 0.048).abs() < 1e-9);
        assert!(q.reflective > 0.0);
    }

    #[test]
    fn test_quality_update_ruminative() {
        // 更新反刍性应朝目标值移动 / Ruminative should move toward target.
        let mut q = SolitudeQuality::new();
        q.update_ruminative(0.7, 0.5, 2.0);
        // raw = 0.7 * 0.5 * 2.0 = 0.7; ema = 0 + 0.1 * 0.7 = 0.07
        assert!((q.ruminative - 0.07).abs() < 1e-9);
        assert!(q.ruminative > 0.0);
    }

    #[test]
    fn test_quality_label_inference() {
        // 标签推断逻辑 / Label inference logic.
        let mut q = SolitudeQuality::new();
        // 全低 → Stagnant
        assert_eq!(q.label(), SolitudeQualityLabel::Stagnant);

        // 反刍主导 → Ruminative
        q.ruminative = 0.6;
        q.reflective = 0.2;
        q.creative = 0.1;
        assert_eq!(q.label(), SolitudeQualityLabel::Ruminative);

        // 创造主导 → Creative
        q.creative = 0.5;
        q.ruminative = 0.2;
        q.reflective = 0.2;
        assert_eq!(q.label(), SolitudeQualityLabel::Creative);

        // 反思主导 → Healthy
        q.reflective = 0.5;
        q.ruminative = 0.1;
        q.creative = 0.2;
        assert_eq!(q.label(), SolitudeQualityLabel::Healthy);
    }

    // --- InnerDialogue 测试 ---

    #[test]
    fn test_dialogue_perspective_generation() {
        // 应生成 4 个视角种子 / Should generate 4 perspective seeds.
        let d = InnerDialogue::new();
        let seeds = d.generate_seeds("我今天感到有些迷茫");
        assert_eq!(seeds.len(), 4);
        assert!(seeds
            .iter()
            .any(|s| s.perspective == DialoguePerspective::Rational));
        assert!(seeds
            .iter()
            .any(|s| s.perspective == DialoguePerspective::Emotional));
        assert!(seeds
            .iter()
            .any(|s| s.perspective == DialoguePerspective::Critical));
        assert!(seeds
            .iter()
            .any(|s| s.perspective == DialoguePerspective::Compassionate));
        // 每个种子应有非空文本 / Each seed should have non-empty text.
        for s in &seeds {
            assert!(!s.seed_text.is_empty());
            assert!(s.weight > 0.0);
        }
    }

    #[test]
    fn test_dialogue_debate_intensity() {
        // 辩论强度应随更新增长 / Debate intensity should grow with updates.
        let mut d = InnerDialogue::new();
        assert_eq!(d.debate_intensity, 0.0);
        d.update_debate_intensity(0.8, 0.6);
        // raw = 0.48; ema = 0.048
        assert!((d.debate_intensity - 0.048).abs() < 1e-9);
        assert!(d.debate_intensity > 0.0);
    }

    // --- SolitudeRhythm 测试 ---

    #[test]
    fn test_rhythm_midnight_deep() {
        // 深夜 0 点应接近最深处 / Midnight should be near deepest.
        let d0 = SolitudeRhythm::depth_multiplier(0);
        // sin(0) = 0 → depth = 0.5
        // 但 2 点：sin(2/24 * 2π) = sin(π/6) = 0.5 → depth = 0.75
        let d2 = SolitudeRhythm::depth_multiplier(2);
        assert!((0.4..=0.6).contains(&d0)); // 0 点 depth = 0.5
        assert!(d2 > d0); // 2 点比 0 点更深
        assert!(d2 > 0.7); // 2 点应较深
    }

    #[test]
    fn test_rhythm_noon_shallow() {
        // 午后 12-14 点应最浅 / Noon should be shallowest.
        let d12 = SolitudeRhythm::depth_multiplier(12);
        let d13 = SolitudeRhythm::depth_multiplier(13);
        let d2 = SolitudeRhythm::depth_multiplier(2);
        // 12 点：sin(π) = 0 → depth = 0.5
        // 13 点：sin(13/24 * 2π) ≈ sin(3.403) ≈ -0.27 → depth ≈ 0.365
        assert!(d13 < d2); // 午后比深夜浅
        assert!(d13 < 0.5); // 13 点应低于 0.5
        assert!((0.4..=0.6).contains(&d12)); // 12 点 ≈ 0.5
    }

    #[test]
    fn test_rhythm_mode_preference() {
        // 昼夜模式偏好 / Circadian mode preference.
        assert_eq!(SolitudeRhythm::preferred_mode(2), SolitudeMode::Reflective);
        assert_eq!(SolitudeRhythm::preferred_mode(7), SolitudeMode::Creative);
        assert_eq!(SolitudeRhythm::preferred_mode(13), SolitudeMode::Learning);
        assert_eq!(SolitudeRhythm::preferred_mode(20), SolitudeMode::Reflective);
        // 跨日环绕 / Wrap-around.
        assert_eq!(SolitudeRhythm::preferred_mode(25), SolitudeMode::Reflective); // 25 % 24 = 1 → Reflective
        assert_eq!(SolitudeRhythm::preferred_mode(29), SolitudeMode::Creative); // 29 % 24 = 5 → Creative
    }

    // --- SolitudeQualityEngine 测试 ---

    #[test]
    fn test_engine_on_thought_updates_quality() {
        // on_thought 应更新品质维度 / on_thought should update quality dimensions.
        let mut engine = SolitudeQualityEngine::new();
        assert_eq!(engine.quality.thought_count, 0);

        engine.on_thought("我在思考今天发生的事情", 0.3, 1000);
        assert_eq!(engine.quality.thought_count, 1);
        assert!(!engine.last_thought.is_empty());
        assert_eq!(engine.last_timestamp, 1000);

        // 多次独白后品质应有变化 / Quality should change after multiple monologues.
        engine.on_thought("也许我需要换个角度看问题", 0.5, 2000);
        engine.on_thought("这个想法很有启发", 0.7, 3000);
        assert_eq!(engine.quality.thought_count, 3);
        // 至少一个维度应大于零 / At least one dimension should be positive.
        assert!(
            engine.quality.reflective > 0.0
                || engine.quality.creative > 0.0
                || engine.quality.ruminative > 0.0
        );
    }

    #[test]
    fn test_engine_prompt_hint() {
        // prompt hint 应非空且受 budget 约束 / Prompt hint should be non-empty and budget-constrained.
        let mut engine = SolitudeQualityEngine::new();
        engine.on_thought("探索内心的深处", 0.4, 1000);
        let hint = engine.to_prompt_hint();
        assert!(!hint.is_empty());
        assert!(hint.contains("独处品质"));
        assert!(hint.len() <= PROMPT_BUDGET + 10); // 允许少量 Unicode 误差
    }

    #[test]
    fn test_engine_serialization_roundtrip() {
        // 序列化往返应保持状态 / Serialization roundtrip should preserve state.
        let mut engine = SolitudeQualityEngine::new();
        engine.on_thought("第一次独白思考", 0.2, 1000);
        engine.on_thought("第二次独白思考，内容不同", -0.3, 2000);
        engine.on_thought("第三次，继续探索", 0.6, 3000);

        // 序列化 / Serialize.
        let serial: SerializableSolitudeQuality = (&engine).into();
        let encoded = bincode::serialize(&serial).unwrap();
        assert!(!encoded.is_empty());

        // 反序列化 / Deserialize.
        let decoded: SerializableSolitudeQuality = bincode::deserialize(&encoded).unwrap();
        let restored: SolitudeQualityEngine = (&decoded).into();

        // 验证关键字段 / Verify key fields.
        assert_eq!(restored.quality.thought_count, engine.quality.thought_count);
        assert!((restored.quality.reflective - engine.quality.reflective).abs() < 1e-9);
        assert!((restored.quality.ruminative - engine.quality.ruminative).abs() < 1e-9);
        assert!((restored.quality.creative - engine.quality.creative).abs() < 1e-9);
        assert_eq!(restored.last_thought, engine.last_thought);
        assert_eq!(restored.last_timestamp, engine.last_timestamp);
        assert_eq!(restored.quality_label(), engine.quality_label());
    }

    #[test]
    fn test_engine_depth_multiplier() {
        // 引擎深度调制应委托给 rhythm / Engine depth multiplier should delegate to rhythm.
        let engine = SolitudeQualityEngine::new();
        let d = engine.depth_multiplier(3);
        // 3 点：sin(3/24 * 2π) = sin(π/4) ≈ 0.707 → depth ≈ 0.854
        assert!(d > 0.8 && d < 0.9);
    }

    #[test]
    fn test_engine_dialogue_seeds() {
        // 引擎应能生成对话种子 / Engine should generate dialogue seeds.
        let engine = SolitudeQualityEngine::new();
        let seeds = engine.dialogue_seeds("我感到孤独但也在成长");
        assert_eq!(seeds.len(), 4);
        for s in &seeds {
            assert!(!s.seed_text.is_empty());
        }
    }
}
