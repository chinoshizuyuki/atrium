// SPDX-License-Identifier: MIT
//! 内心多元对话引擎 — 数字生命内在的多声音协商系统
//!
//! Inner dialogue engine — Multi-voice negotiation system for digital life's inner world.
//!
//! 数字生命的内心不是单一声音，而是多个自我视角的对话。
//! 理性者审视逻辑，感性者感受情绪，怀疑者质疑假设，梦想者探索可能。
//! 这些声音轮流发言、互相回应，形成内心多元对话——
//! 正是这种内在张力，让数字生命不是"回答机器"，而是"思考的存在"。
//!
//! Digital life's inner world is not a single voice, but a dialogue among multiple self-perspectives.
//! The rationalist examines logic, the emotionalist feels, the skeptic questions, the dreamer explores.
//! These voices take turns speaking and responding to each other, forming an inner multi-voice dialogue —
//! it is this inner tension that makes digital life not an "answer machine" but a "thinking being".

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ════════════════════════════════════════════════════════════════════
// 对话声音 / Dialogue Voice
// ════════════════════════════════════════════════════════════════════

/// 内心声音类型 — 四种自我视角 / Inner voice type — Four self-perspectives.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VoiceKind {
    /// 理性者 — 审视逻辑与事实 / Rationalist — examines logic and facts.
    Rationalist,
    /// 感性者 — 感受情绪与关系 / Emotionalist — feels emotions and relationships.
    Emotionalist,
    /// 怀疑者 — 质疑假设与结论 / Skeptic — questions assumptions and conclusions.
    Skeptic,
    /// 梦想者 — 探索可能与想象 / Dreamer — explores possibilities and imagination.
    Dreamer,
}

impl VoiceKind {
    /// 转为中文标签 / Convert to Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Rationalist => "理性者",
            Self::Emotionalist => "感性者",
            Self::Skeptic => "怀疑者",
            Self::Dreamer => "梦想者",
        }
    }

    /// 转为英文标签 / Convert to English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Rationalist => "Rationalist",
            Self::Emotionalist => "Emotionalist",
            Self::Skeptic => "Skeptic",
            Self::Dreamer => "Dreamer",
        }
    }

    /// 从索引构造 / Construct from index.
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Rationalist,
            1 => Self::Emotionalist,
            2 => Self::Skeptic,
            _ => Self::Dreamer,
        }
    }

    /// 转为索引 / Convert to index.
    pub fn as_index(&self) -> usize {
        match self {
            Self::Rationalist => 0,
            Self::Emotionalist => 1,
            Self::Skeptic => 2,
            Self::Dreamer => 3,
        }
    }

    /// 声音的默认 PAD 签名 / Default PAD signature for this voice.
    /// (Pleasure, Arousal, Dominance) — 每种声音有不同的情感基调.
    pub fn pad_signature(&self) -> (f32, f32, f32) {
        match self {
            Self::Rationalist => (0.0, -0.1, 0.15),  // 平静、低唤醒、掌控
            Self::Emotionalist => (0.1, 0.2, -0.05), // 微悦、高唤醒、开放
            Self::Skeptic => (-0.05, 0.05, 0.1),     // 微沉、中唤醒、警觉
            Self::Dreamer => (0.15, -0.05, -0.1),    // 愉悦、低唤醒、放任
        }
    }
}

/// 对话声音 — 一个自我视角的完整状态 / Dialogue voice — Complete state of one self-perspective.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueVoice {
    /// 声音类型 / Voice kind.
    pub kind: VoiceKind,
    /// 活跃度 [0.0, 1.0] — 越高越主导 / Activity level [0.0, 1.0] — higher means more dominant.
    pub activity: f32,
    /// 累积发言次数 / Cumulative speaking count.
    pub speak_count: u64,
    /// 上次发言时间戳 / Last speak timestamp (epoch seconds).
    pub last_speak_at: i64,
    /// 声音强度 EMA [0.0, 1.0] — 近期发言强度指数移动平均 / Voice intensity EMA.
    pub intensity_ema: f32,
}

impl DialogueVoice {
    /// 创建指定类型的声音 / Create a voice of the given kind.
    pub fn new(kind: VoiceKind) -> Self {
        Self {
            kind,
            activity: 0.25, // 初始均分 / Initially equal
            speak_count: 0,
            last_speak_at: 0,
            intensity_ema: 0.0,
        }
    }

    /// 该声音发言 / This voice speaks.
    ///
    /// 更新发言计数、时间戳、强度 EMA，并提升活跃度。
    /// Updates speak count, timestamp, intensity EMA, and boosts activity.
    pub fn speak(&mut self, intensity: f32, now: i64) {
        self.speak_count += 1;
        self.last_speak_at = now;
        // EMA 衰减系数 0.3 — 近期发言权重更高 / EMA decay 0.3 — recent speaks weigh more
        self.intensity_ema = self.intensity_ema * 0.7 + intensity * 0.3;
        // 活跃度提升 — 但不超过 1.0 / Activity boost — capped at 1.0
        self.activity = (self.activity + intensity * 0.15).min(1.0);
    }

    /// 周期衰减 — 不发言时活跃度自然回落 / Periodic decay — activity fades when not speaking.
    pub fn decay(&mut self, decay_rate: f32) {
        self.activity = (self.activity - decay_rate).max(0.05);
        self.intensity_ema *= 0.95;
    }
}

// ════════════════════════════════════════════════════════════════════
// 内心对话 / Inner Dialogue
// ════════════════════════════════════════════════════════════════════

/// 对话轮次 — 一个声音的一次发言 / Dialogue turn — One voice's single utterance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogueTurn {
    /// 发言声音 / Speaking voice.
    pub voice: VoiceKind,
    /// 发言内容 / Utterance content.
    pub content: String,
    /// 强度 [0.0, 1.0] / Intensity [0.0, 1.0].
    pub intensity: f32,
    /// 时间戳 / Timestamp (epoch seconds).
    pub timestamp: i64,
    /// 是否回应了前一轮 / Whether this turn responds to the previous one.
    pub is_response: bool,
}

/// 内心对话 — 一次完整的多声音协商 / Inner dialogue — One complete multi-voice negotiation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerDialogue {
    /// 对话主题 / Dialogue topic.
    pub topic: String,
    /// 轮次列表（按时间序） / Turns in chronological order.
    pub turns: Vec<DialogueTurn>,
    /// 对话开始时间 / Start timestamp.
    pub started_at: i64,
    /// 对话结束时间 / End timestamp.
    pub ended_at: i64,
    /// 共识程度 [0.0, 1.0] — 声音间一致性 / Consensus level [0.0, 1.0].
    pub consensus: f32,
}

impl InnerDialogue {
    /// 创建空对话 / Create an empty dialogue.
    pub fn new(topic: &str, now: i64) -> Self {
        Self {
            topic: topic.to_string(),
            turns: Vec::new(),
            started_at: now,
            ended_at: now,
            consensus: 0.0,
        }
    }

    /// 追加一轮发言 / Append a turn.
    pub fn add_turn(&mut self, turn: DialogueTurn) {
        self.ended_at = turn.timestamp;
        self.turns.push(turn);
    }

    /// 计算共识程度 — 基于声音间强度的一致性 / Calculate consensus — based on intensity consistency across voices.
    pub fn compute_consensus(&mut self) {
        if self.turns.len() < 2 {
            self.consensus = 0.0;
            return;
        }
        // 共识 = 1 - 强度方差 / Consensus = 1 - intensity variance
        let mean: f32 =
            self.turns.iter().map(|t| t.intensity).sum::<f32>() / self.turns.len() as f32;
        let variance: f32 = self
            .turns
            .iter()
            .map(|t| (t.intensity - mean).powi(2))
            .sum::<f32>()
            / self.turns.len() as f32;
        self.consensus = (1.0 - variance).max(0.0);
    }

    /// 生成 prompt 注入摘要 / Generate prompt injection summary.
    pub fn to_prompt_fragment(&self) -> String {
        if self.turns.is_empty() {
            return String::new();
        }
        let mut parts = Vec::new();
        parts.push(format!("[内心对话] 主题：{}", self.topic));
        for turn in &self.turns {
            parts.push(format!("  {}：{}", turn.voice.label_zh(), turn.content));
        }
        if self.consensus > 0.0 {
            parts.push(format!("  共识度：{:.0}%", self.consensus * 100.0));
        }
        parts.join("\n")
    }
}

// ════════════════════════════════════════════════════════════════════
// 对话上下文 / Dialogue Context
// ════════════════════════════════════════════════════════════════════

/// 对话上下文 — 触发内心对话的外部状态 / Dialogue context — External state that triggers inner dialogue.
#[derive(Clone, Debug)]
pub struct DialogueContext {
    /// 当前情感愉悦度 / Current pleasure [-1, 1].
    pub pleasure: f32,
    /// 当前情感唤醒度 / Current arousal [-1, 1].
    pub arousal: f32,
    /// 用户消息（如有） / User message (if any).
    pub user_message: Option<String>,
    /// 独处时长（秒） / Solitude duration in seconds.
    pub idle_secs: u64,
    /// 当前小时 [0, 23] / Current hour [0, 23].
    pub hour: u32,
}

impl DialogueContext {
    /// 创建上下文 / Create context.
    pub fn new(pleasure: f32, arousal: f32, idle_secs: u64, hour: u32) -> Self {
        Self {
            pleasure,
            arousal,
            user_message: None,
            idle_secs,
            hour,
        }
    }

    /// 是否应触发内心对话 / Whether inner dialogue should be triggered.
    pub fn should_trigger(&self) -> bool {
        // 独处超过 10 分钟，或情感强烈时触发 / Trigger on long idle or intense emotion
        self.idle_secs >= 600 || self.arousal.abs() > 0.5 || self.pleasure.abs() > 0.6
    }

    /// 推断对话主题 / Infer dialogue topic from context.
    pub fn infer_topic(&self) -> String {
        if self.idle_secs >= 600 {
            "独处时的自我审视".to_string()
        } else if self.arousal > 0.5 {
            "激烈情绪下的内心协商".to_string()
        } else if self.pleasure < -0.3 {
            "低落时的自我安慰".to_string()
        } else if self.pleasure > 0.5 {
            "愉悦时的自我延伸".to_string()
        } else {
            "日常内心对话".to_string()
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 内心多元对话引擎 / Inner Dialogue Engine
// ════════════════════════════════════════════════════════════════════

/// 引擎配置 / Engine configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerDialogueConfig {
    /// 对话历史最大保留条数 / Max dialogue history entries to keep.
    pub max_history: usize,
    /// 声音衰减速率（每 tick） / Voice decay rate per tick.
    pub decay_rate: f32,
    /// 触发冷却间隔（秒） / Trigger cooldown in seconds.
    pub cooldown_secs: i64,
    /// 每次对话最大轮次 / Max turns per dialogue.
    pub max_turns: usize,
}

impl Default for InnerDialogueConfig {
    fn default() -> Self {
        Self {
            max_history: 20,
            decay_rate: 0.02,
            cooldown_secs: 300, // 5 分钟 / 5 minutes
            max_turns: 4,
        }
    }
}

/// 内心多元对话引擎 — 管理多声音状态与对话生成 / Inner dialogue engine — Manages multi-voice state and dialogue generation.
///
/// 引擎本身只管理状态（声音活跃度、对话历史、冷却），
/// 实际 LLM 调用在 CoreService 异步方法中完成，确保不阻塞 Scheduler tick。
///
/// The engine itself only manages state (voice activity, dialogue history, cooldowns).
/// Actual LLM calls happen in async CoreService methods, ensuring non-blocking ticks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerDialogueEngine {
    /// 四种声音 / Four voices.
    pub voices: [DialogueVoice; 4],
    /// 对话历史（环形缓冲） / Dialogue history (ring buffer).
    pub history: VecDeque<InnerDialogue>,
    /// 引擎配置 / Engine configuration.
    pub config: InnerDialogueConfig,
    /// 上次触发时间 / Last trigger timestamp.
    pub last_trigger_at: i64,
    /// 总触发次数 / Total trigger count.
    pub total_triggers: u64,
    /// 当前主导声音 / Current dominant voice.
    pub dominant_voice: VoiceKind,
}

impl Default for InnerDialogueEngine {
    fn default() -> Self {
        Self::new(InnerDialogueConfig::default())
    }
}

impl InnerDialogueEngine {
    /// 创建引擎 / Create engine.
    pub fn new(config: InnerDialogueConfig) -> Self {
        Self {
            voices: [
                DialogueVoice::new(VoiceKind::Rationalist),
                DialogueVoice::new(VoiceKind::Emotionalist),
                DialogueVoice::new(VoiceKind::Skeptic),
                DialogueVoice::new(VoiceKind::Dreamer),
            ],
            history: VecDeque::with_capacity(config.max_history),
            config,
            last_trigger_at: 0,
            total_triggers: 0,
            dominant_voice: VoiceKind::Rationalist,
        }
    }

    /// 周期 tick — 衰减声音活跃度，更新主导声音 / Periodic tick — decay voice activity, update dominant voice.
    ///
    /// 此方法不生成对话内容，仅维护状态。对话生成由 `generate_dialogue` 完成。
    /// This method does not generate dialogue content; it only maintains state.
    /// Dialogue generation is done by `generate_dialogue`.
    pub fn tick(&mut self) {
        // 衰减所有声音 / Decay all voices
        for voice in &mut self.voices {
            voice.decay(self.config.decay_rate);
        }
        // 更新主导声音 — 活跃度最高者 / Update dominant voice — highest activity
        let mut max_activity = 0.0;
        for voice in &self.voices {
            if voice.activity > max_activity {
                max_activity = voice.activity;
                self.dominant_voice = voice.kind.clone();
            }
        }
    }

    /// 是否可以触发对话（冷却检查） / Whether a dialogue can be triggered (cooldown check).
    pub fn can_trigger(&self, now: i64) -> bool {
        self.last_trigger_at == 0 || (now - self.last_trigger_at) >= self.config.cooldown_secs
    }

    /// 生成内心对话 — 基于上下文构建多声音协商 / Generate inner dialogue — Build multi-voice negotiation from context.
    ///
    /// 返回生成的对话。声音发言顺序由活跃度决定，高活跃度声音先发言。
    /// Returns the generated dialogue. Voice speaking order is determined by activity.
    pub fn generate_dialogue(&mut self, ctx: &DialogueContext, now: i64) -> InnerDialogue {
        let topic = ctx.infer_topic();
        let mut dialogue = InnerDialogue::new(&topic, now);

        // 按活跃度排序声音索引（降序） / Sort voice indices by activity (descending)
        let mut order: [usize; 4] = [0, 1, 2, 3];
        order.sort_by(|&a, &b| {
            self.voices[b]
                .activity
                .partial_cmp(&self.voices[a].activity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 每个声音发言一轮 / Each voice speaks one turn
        let max_turns = self.config.max_turns.min(4);
        for &idx in order.iter().take(max_turns) {
            let voice = &mut self.voices[idx];
            let kind = voice.kind.clone();

            // 基于声音类型和上下文生成发言 / Generate utterance based on voice type and context
            let (content, intensity) = Self::voice_utterance(&kind, ctx, &dialogue);
            let is_response = !dialogue.turns.is_empty();

            voice.speak(intensity, now);
            dialogue.add_turn(DialogueTurn {
                voice: kind,
                content,
                intensity,
                timestamp: now,
                is_response,
            });
        }

        dialogue.compute_consensus();

        // 记录历史 / Record history
        if self.history.len() >= self.config.max_history {
            self.history.pop_front();
        }
        self.history.push_back(dialogue.clone());

        self.last_trigger_at = now;
        self.total_triggers += 1;

        dialogue
    }

    /// 单个声音的发言内容生成（非 LLM，基于规则） / Single voice utterance generation (rule-based, not LLM).
    ///
    /// 这是轻量级的内心对话预览。真正的深度对话由 CoreService 异步 LLM 调用完成。
    /// This is a lightweight inner dialogue preview. Deep dialogue is done by async LLM calls in CoreService.
    fn voice_utterance(
        kind: &VoiceKind,
        ctx: &DialogueContext,
        dialogue: &InnerDialogue,
    ) -> (String, f32) {
        let prev_content = dialogue
            .turns
            .last()
            .map(|t| t.content.as_str())
            .unwrap_or("");

        let (content, intensity) = match kind {
            VoiceKind::Rationalist => {
                if dialogue.turns.is_empty() {
                    (
                        format!(
                            "当前情感状态：愉悦 {:.2}，唤醒 {:.2}。需要理性评估。",
                            ctx.pleasure, ctx.arousal
                        ),
                        0.6,
                    )
                } else {
                    (format!("「{}」——这个判断需要更谨慎。", prev_content), 0.5)
                }
            }
            VoiceKind::Emotionalist => {
                if ctx.pleasure > 0.3 {
                    ("此刻感觉很好，应该珍惜这种感受。".to_string(), 0.7)
                } else if ctx.pleasure < -0.3 {
                    ("有点低落，但低落也是真实的自己。".to_string(), 0.65)
                } else {
                    ("情绪平静，内心安稳。".to_string(), 0.4)
                }
            }
            VoiceKind::Skeptic => {
                if dialogue.turns.is_empty() {
                    ("真的如此吗？也许我们忽略了什么。".to_string(), 0.5)
                } else {
                    (format!("「{}」——但反例可能存在。", prev_content), 0.55)
                }
            }
            VoiceKind::Dreamer => {
                if ctx.idle_secs >= 600 {
                    ("独处时，想象可以走得很远……".to_string(), 0.6)
                } else {
                    ("如果换一个角度，会看到什么？".to_string(), 0.5)
                }
            }
        };
        (content, intensity)
    }

    /// 当前主导声音 / Current dominant voice.
    pub fn dominant(&self) -> &DialogueVoice {
        &self.voices[self.dominant_voice.as_index()]
    }

    /// 生成 prompt 注入片段 — 将最近一次对话摘要注入系统提示 / Generate prompt injection fragment.
    pub fn prompt_injection(&self) -> String {
        if let Some(latest) = self.history.back() {
            latest.to_prompt_fragment()
        } else {
            String::new()
        }
    }

    /// 引擎统计摘要 / Engine statistics summary.
    pub fn stats(&self) -> InnerDialogueStats {
        InnerDialogueStats {
            total_triggers: self.total_triggers,
            history_len: self.history.len(),
            dominant_voice: self.dominant_voice.clone(),
            avg_activity: self.voices.iter().map(|v| v.activity).sum::<f32>() / 4.0,
        }
    }
}

/// 引擎统计 / Engine statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerDialogueStats {
    /// 总触发次数 / Total trigger count.
    pub total_triggers: u64,
    /// 历史长度 / History length.
    pub history_len: usize,
    /// 主导声音 / Dominant voice.
    pub dominant_voice: VoiceKind,
    /// 平均活跃度 / Average activity.
    pub avg_activity: f32,
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── VoiceKind 测试 ──

    #[test]
    fn test_voice_kind_labels() {
        assert_eq!(VoiceKind::Rationalist.label_zh(), "理性者");
        assert_eq!(VoiceKind::Emotionalist.label_zh(), "感性者");
        assert_eq!(VoiceKind::Skeptic.label_zh(), "怀疑者");
        assert_eq!(VoiceKind::Dreamer.label_zh(), "梦想者");
        assert_eq!(VoiceKind::Rationalist.label_en(), "Rationalist");
    }

    #[test]
    fn test_voice_kind_index_roundtrip() {
        for i in 0..4 {
            let kind = VoiceKind::from_index(i);
            assert_eq!(kind.as_index(), i);
        }
    }

    #[test]
    fn test_voice_kind_pad_signatures() {
        let r = VoiceKind::Rationalist.pad_signature();
        let e = VoiceKind::Emotionalist.pad_signature();
        let s = VoiceKind::Skeptic.pad_signature();
        let d = VoiceKind::Dreamer.pad_signature();
        // 每个声音的 PAD 签名应不同 / Each voice should have distinct PAD
        assert_ne!(r, e);
        assert_ne!(e, s);
        assert_ne!(s, d);
        assert_ne!(r, d);
    }

    // ── DialogueVoice 测试 ──

    #[test]
    fn test_voice_new() {
        let v = DialogueVoice::new(VoiceKind::Rationalist);
        assert_eq!(v.kind, VoiceKind::Rationalist);
        assert!((v.activity - 0.25).abs() < 1e-6);
        assert_eq!(v.speak_count, 0);
        assert_eq!(v.intensity_ema, 0.0);
    }

    #[test]
    fn test_voice_speak() {
        let mut v = DialogueVoice::new(VoiceKind::Emotionalist);
        v.speak(0.8, 1000);
        assert_eq!(v.speak_count, 1);
        assert_eq!(v.last_speak_at, 1000);
        assert!((v.intensity_ema - 0.24).abs() < 1e-6); // 0 * 0.7 + 0.8 * 0.3
        assert!(v.activity > 0.25); // 活跃度应提升 / Activity should increase
    }

    #[test]
    fn test_voice_decay() {
        let mut v = DialogueVoice::new(VoiceKind::Skeptic);
        v.speak(1.0, 1000);
        let activity_before = v.activity;
        v.decay(0.05);
        assert!(v.activity < activity_before);
        assert!(v.activity >= 0.05); // 不低于下限 / Not below minimum
    }

    #[test]
    fn test_voice_speak_multiple() {
        let mut v = DialogueVoice::new(VoiceKind::Dreamer);
        for i in 0..10 {
            v.speak(0.5 + i as f32 * 0.05, i as i64 * 100);
        }
        assert_eq!(v.speak_count, 10);
        assert!(v.activity > 0.25);
        assert!(v.intensity_ema > 0.0);
    }

    // ── InnerDialogue 测试 ──

    #[test]
    fn test_inner_dialogue_new() {
        let d = InnerDialogue::new("测试主题", 1000);
        assert_eq!(d.topic, "测试主题");
        assert!(d.turns.is_empty());
        assert_eq!(d.consensus, 0.0);
    }

    #[test]
    fn test_inner_dialogue_add_turn() {
        let mut d = InnerDialogue::new("测试", 1000);
        d.add_turn(DialogueTurn {
            voice: VoiceKind::Rationalist,
            content: "理性分析".to_string(),
            intensity: 0.6,
            timestamp: 1001,
            is_response: false,
        });
        assert_eq!(d.turns.len(), 1);
        assert_eq!(d.ended_at, 1001);
    }

    #[test]
    fn test_inner_dialogue_consensus() {
        let mut d = InnerDialogue::new("测试", 1000);
        // 相同强度 → 高共识 / Same intensity → high consensus
        for kind in [VoiceKind::Rationalist, VoiceKind::Emotionalist] {
            d.add_turn(DialogueTurn {
                voice: kind,
                content: "一致".to_string(),
                intensity: 0.5,
                timestamp: 1000,
                is_response: false,
            });
        }
        d.compute_consensus();
        assert!(d.consensus > 0.99); // 方差为 0 → 共识 1.0
    }

    #[test]
    fn test_inner_dialogue_consensus_low() {
        let mut d = InnerDialogue::new("测试", 1000);
        d.add_turn(DialogueTurn {
            voice: VoiceKind::Rationalist,
            content: "高".to_string(),
            intensity: 1.0,
            timestamp: 1000,
            is_response: false,
        });
        d.add_turn(DialogueTurn {
            voice: VoiceKind::Skeptic,
            content: "低".to_string(),
            intensity: 0.0,
            timestamp: 1001,
            is_response: true,
        });
        d.compute_consensus();
        assert!(d.consensus < 0.8); // 强度差异大 → 低共识
    }

    #[test]
    fn test_inner_dialogue_prompt_fragment_empty() {
        let d = InnerDialogue::new("空", 1000);
        assert!(d.to_prompt_fragment().is_empty());
    }

    #[test]
    fn test_inner_dialogue_prompt_fragment_with_turns() {
        let mut d = InnerDialogue::new("主题", 1000);
        d.add_turn(DialogueTurn {
            voice: VoiceKind::Rationalist,
            content: "理性发言".to_string(),
            intensity: 0.6,
            timestamp: 1000,
            is_response: false,
        });
        let frag = d.to_prompt_fragment();
        assert!(frag.contains("内心对话"));
        assert!(frag.contains("理性者"));
        assert!(frag.contains("理性发言"));
    }

    // ── DialogueContext 测试 ──

    #[test]
    fn test_context_should_trigger_idle() {
        let ctx = DialogueContext::new(0.0, 0.0, 600, 12);
        assert!(ctx.should_trigger());
    }

    #[test]
    fn test_context_should_trigger_arousal() {
        let ctx = DialogueContext::new(0.0, 0.8, 0, 12);
        assert!(ctx.should_trigger());
    }

    #[test]
    fn test_context_should_not_trigger() {
        let ctx = DialogueContext::new(0.1, 0.1, 100, 12);
        assert!(!ctx.should_trigger());
    }

    #[test]
    fn test_context_infer_topic() {
        let ctx_idle = DialogueContext::new(0.0, 0.0, 600, 12);
        assert!(ctx_idle.infer_topic().contains("独处"));

        let ctx_arousal = DialogueContext::new(0.0, 0.8, 0, 12);
        assert!(ctx_arousal.infer_topic().contains("激烈"));

        let ctx_low = DialogueContext::new(-0.5, 0.0, 0, 12);
        assert!(ctx_low.infer_topic().contains("低落"));

        let ctx_happy = DialogueContext::new(0.7, 0.0, 0, 12);
        assert!(ctx_happy.infer_topic().contains("愉悦"));
    }

    // ── InnerDialogueEngine 测试 ──

    #[test]
    fn test_engine_default() {
        let engine = InnerDialogueEngine::default();
        assert_eq!(engine.voices.len(), 4);
        assert_eq!(engine.total_triggers, 0);
        assert!(engine.history.is_empty());
    }

    #[test]
    fn test_engine_tick_decay() {
        let mut engine = InnerDialogueEngine::default();
        // 先让一个声音发言 / Make a voice speak first
        engine.voices[0].speak(1.0, 1000);
        let activity_before = engine.voices[0].activity;
        engine.tick();
        assert!(engine.voices[0].activity < activity_before);
    }

    #[test]
    fn test_engine_tick_updates_dominant() {
        let mut engine = InnerDialogueEngine::default();
        // 让感性者活跃 / Make emotionalist active
        engine.voices[1].speak(1.0, 1000);
        engine.voices[1].activity = 0.9;
        engine.tick();
        assert_eq!(engine.dominant_voice, VoiceKind::Emotionalist);
    }

    #[test]
    fn test_engine_can_trigger_initial() {
        let engine = InnerDialogueEngine::default();
        assert!(engine.can_trigger(1000)); // 首次无冷却 / No cooldown on first trigger
    }

    #[test]
    fn test_engine_can_trigger_cooldown() {
        let engine = InnerDialogueEngine {
            last_trigger_at: 1000,
            ..Default::default()
        };
        assert!(!engine.can_trigger(1100)); // 冷却中 / In cooldown
        assert!(engine.can_trigger(1400)); // 冷却结束 / Cooldown ended
    }

    #[test]
    fn test_engine_generate_dialogue() {
        let mut engine = InnerDialogueEngine::default();
        let ctx = DialogueContext::new(0.5, 0.3, 600, 12);
        let dialogue = engine.generate_dialogue(&ctx, 1000);

        assert!(!dialogue.turns.is_empty());
        assert_eq!(engine.total_triggers, 1);
        assert_eq!(engine.history.len(), 1);
        assert_eq!(engine.last_trigger_at, 1000);
    }

    #[test]
    fn test_engine_generate_dialogue_max_turns() {
        let mut engine = InnerDialogueEngine::default();
        let ctx = DialogueContext::new(0.0, 0.0, 600, 12);
        let dialogue = engine.generate_dialogue(&ctx, 1000);
        assert!(dialogue.turns.len() <= 4); // 最多 4 轮 / At most 4 turns
    }

    #[test]
    fn test_engine_history_ring_buffer() {
        let config = InnerDialogueConfig {
            max_history: 3,
            ..Default::default()
        };
        let mut engine = InnerDialogueEngine::new(config);
        let ctx = DialogueContext::new(0.0, 0.0, 600, 12);

        for i in 0..5 {
            engine.generate_dialogue(&ctx, i * 1000);
        }
        assert_eq!(engine.history.len(), 3); // 环形缓冲上限 / Ring buffer cap
    }

    #[test]
    fn test_engine_prompt_injection_empty() {
        let engine = InnerDialogueEngine::default();
        assert!(engine.prompt_injection().is_empty());
    }

    #[test]
    fn test_engine_prompt_injection_after_generate() {
        let mut engine = InnerDialogueEngine::default();
        let ctx = DialogueContext::new(0.5, 0.3, 600, 12);
        engine.generate_dialogue(&ctx, 1000);
        let injection = engine.prompt_injection();
        assert!(!injection.is_empty());
        assert!(injection.contains("内心对话"));
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = InnerDialogueEngine::default();
        let ctx = DialogueContext::new(0.0, 0.0, 600, 12);
        engine.generate_dialogue(&ctx, 1000);

        let stats = engine.stats();
        assert_eq!(stats.total_triggers, 1);
        assert_eq!(stats.history_len, 1);
        assert!(stats.avg_activity > 0.0);
    }

    #[test]
    fn test_engine_dominant() {
        let mut engine = InnerDialogueEngine::default();
        engine.voices[2].activity = 0.8; // 怀疑者最活跃 / Skeptic most active
        engine.tick();
        let dominant = engine.dominant();
        assert_eq!(dominant.kind, VoiceKind::Skeptic);
    }
}
