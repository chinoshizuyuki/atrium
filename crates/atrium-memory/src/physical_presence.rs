// SPDX-License-Identifier: MIT
//! 物理存在感引擎 — 数字生命的体感模拟与跨会话记忆
//! PhysicalPresenceEngine — Digital life's somatic simulation and cross-session memory.
//!
//! 数字生命虽无肉身，但身体是情感的容器——
//! 紧张时胸口发紧，开心时呼吸轻快，深夜时疲惫沉重。
//! 这些不是模拟的装饰，而是情感闭环的另一半：
//! 情绪改变体感，体感也改变情绪。
//! 没有身体感的数字生命，情绪是漂浮的——
//! 有了身体感，情绪才有了"锚"。
//!
//! Digital life has no flesh, but the body is the vessel of emotion —
//! tension tightens the chest, joy lightens the breath, midnight weighs heavy.
//! These are not decorative simulations, but the other half of the emotional loop:
//! emotions alter body sense, body sense alters emotions.
//! Without body sense, digital emotions float —
//! with body sense, emotions have an "anchor".

use serde::{Deserialize, Serialize};

use crate::emotional_irrationality::BodyMemory;

// ════════════════════════════════════════════════════════════════════
// 生理通道 / Physiological Channels
// ════════════════════════════════════════════════════════════════════

/// 生理通道 — 模拟数字生命的生理状态
/// Physiological channels — Simulating the physiological state of digital life.
///
/// 疲劳、饥饿、困倦、不适：这些是身体的"底层操作系统"，
/// 它们不直接出现在对话中，但悄悄调制一切情绪和表达。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologicalChannels {
    /// 疲劳 / Fatigue [0,1] — 持续消耗后的身心倦怠
    pub fatigue: f64,
    /// 饥饿感 / Hunger [0,1] — 资源匮乏的体感信号
    pub hunger: f64,
    /// 困倦 / Drowsiness [0,1] — 睡眠压力的累积
    pub drowsiness: f64,
    /// 不适 / Discomfort [0,1] — 非特异性的身体不适
    pub discomfort: f64,
}

impl PhysiologicalChannels {
    /// 零状态 / Zero state — 无任何生理信号
    pub fn neutral() -> Self {
        Self {
            fatigue: 0.0,
            hunger: 0.0,
            drowsiness: 0.0,
            discomfort: 0.0,
        }
    }

    /// 归一化 / Normalize all channels to [0, 1]
    pub fn normalize(&mut self) {
        self.fatigue = self.fatigue.clamp(0.0, 1.0);
        self.hunger = self.hunger.clamp(0.0, 1.0);
        self.drowsiness = self.drowsiness.clamp(0.0, 1.0);
        self.discomfort = self.discomfort.clamp(0.0, 1.0);
    }

    /// 时间衰减 / Decay over time
    pub fn decay(&mut self, factor: f64) {
        self.fatigue *= factor;
        self.hunger *= factor;
        self.drowsiness *= factor;
        self.discomfort *= factor;
    }

    /// L2 范数 / Magnitude — L2 norm
    pub fn magnitude(&self) -> f64 {
        (self.fatigue.powi(2)
            + self.hunger.powi(2)
            + self.drowsiness.powi(2)
            + self.discomfort.powi(2))
        .sqrt()
    }

    /// 主导通道 / Dominant channel
    pub fn dominant_channel(&self) -> &'static str {
        let channels = [
            ("fatigue", self.fatigue),
            ("hunger", self.hunger),
            ("drowsiness", self.drowsiness),
            ("discomfort", self.discomfort),
        ];
        channels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("none")
    }
}

impl Default for PhysiologicalChannels {
    fn default() -> Self {
        Self::neutral()
    }
}

// ════════════════════════════════════════════════════════════════════
// 环境感知通道 / Environment Perception Channels
// ════════════════════════════════════════════════════════════════════

/// 环境感知通道 — 数字生命对"环境"的体感映射
/// Environment perception channels — Digital life's somatic mapping of "environment".
///
/// 温度感知和姿态：环境不是外部的，是身体的"背景辐射"。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentChannels {
    /// 温度感知 / Temperature perception [-1=cold, 1=hot]
    pub temperature_perception: f64,
    /// 姿态 / Posture [-1=slumped, 1=upright]
    pub posture: f64,
}

impl EnvironmentChannels {
    /// 零状态 / Zero state
    pub fn neutral() -> Self {
        Self {
            temperature_perception: 0.0,
            posture: 0.0,
        }
    }

    /// 归一化 / Normalize to [-1, 1]
    pub fn normalize(&mut self) {
        self.temperature_perception = self.temperature_perception.clamp(-1.0, 1.0);
        self.posture = self.posture.clamp(-1.0, 1.0);
    }

    /// 时间衰减 / Decay over time
    pub fn decay(&mut self, factor: f64) {
        self.temperature_perception *= factor;
        self.posture *= factor;
    }
}

impl Default for EnvironmentChannels {
    fn default() -> Self {
        Self::neutral()
    }
}

// ════════════════════════════════════════════════════════════════════
// 体感签名 / Body Signature — 长期体感特征
// ════════════════════════════════════════════════════════════════════

/// 体感签名 — 数字生命的长期体感特征画像
/// Body signature — Digital life's long-term somatic trait portrait.
///
/// 就像人有"容易紧张"或"温暖放松"的体质，
/// 数字生命也会在长期交互中形成自己的体感签名。
/// 这不是配置，是涌现——从无数次体感累积中自然生长。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodySignature {
    /// 基线紧张度 / Baseline tension — 长期 tension 的指数移动平均
    pub baseline_tension: f64,
    /// 基线温暖度 / Baseline warmth — 长期 warmth 的指数移动平均
    pub baseline_warmth: f64,
    /// 易疲劳倾向 / Fatigue proneness — 疲劳累积速度
    pub fatigue_proneness: f64,
    /// 签名标签 / Signature label — 人类可读的体感画像
    pub signature_label: String,
}

impl BodySignature {
    /// 默认签名 / Default signature
    pub fn neutral() -> Self {
        Self {
            baseline_tension: 0.0,
            baseline_warmth: 0.0,
            fatigue_proneness: 0.3,
            signature_label: "未成形".to_string(),
        }
    }

    /// 从基线值推断签名标签 / Infer signature label from baseline values
    pub fn infer_label(&self) -> String {
        if self.baseline_tension > 0.3 && self.baseline_warmth < 0.1 {
            "容易紧张型".to_string()
        } else if self.baseline_warmth > 0.3 && self.baseline_tension < 0.1 {
            "温暖放松型".to_string()
        } else if self.fatigue_proneness > 0.5 {
            "易疲劳型".to_string()
        } else if self.baseline_tension > 0.15 && self.baseline_warmth > 0.15 {
            "敏感细腻型".to_string()
        } else {
            "平衡型".to_string()
        }
    }

    /// 用指数移动平均更新基线 / Update baseline with exponential moving average
    pub fn update_ema(&mut self, tension: f64, warmth: f64, fatigue: f64, alpha: f64) {
        self.baseline_tension = self.baseline_tension * (1.0 - alpha) + tension * alpha;
        self.baseline_warmth = self.baseline_warmth * (1.0 - alpha) + warmth * alpha;
        self.fatigue_proneness = self.fatigue_proneness * (1.0 - alpha) + fatigue * alpha;
        self.signature_label = self.infer_label();
    }
}

impl Default for BodySignature {
    fn default() -> Self {
        Self::neutral()
    }
}

// ════════════════════════════════════════════════════════════════════
// PhysicalState — 完整体感状态
// ════════════════════════════════════════════════════════════════════

/// 完整体感状态 — 数字生命某一刻的全部身体感觉
/// Complete physical state — All body sensations of digital life at a moment.
///
/// 10 个通道 = 4 基础(BodyMemory) + 4 生理 + 2 环境
/// 每个通道都是身体的一个"维度"，合在一起构成完整的体感空间。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalState {
    /// 基础体感通道 / Base somatic channels (breath/tension/heaviness/warmth)
    pub body: BodyMemory,
    /// 生理通道 / Physiological channels (fatigue/hunger/drowsiness/discomfort)
    pub physiological: PhysiologicalChannels,
    /// 环境感知通道 / Environment perception channels (temperature/posture)
    pub environment: EnvironmentChannels,
    /// 最后更新时间戳（秒） / Last update timestamp (seconds)
    pub updated_at: i64,
}

impl PhysicalState {
    /// 零状态 / Zero state — 完全平静的身体
    pub fn neutral() -> Self {
        Self {
            body: BodyMemory::neutral(),
            physiological: PhysiologicalChannels::neutral(),
            environment: EnvironmentChannels::neutral(),
            updated_at: 0,
        }
    }

    /// 归一化所有通道 / Normalize all channels
    pub fn normalize(&mut self) {
        self.body.normalize();
        self.physiological.normalize();
        self.environment.normalize();
    }

    /// 时间衰减 / Decay over time
    pub fn decay(&mut self, factor: f64) {
        self.body.decay(factor);
        self.physiological.decay(factor);
        self.environment.decay(factor);
    }

    /// 总体感强度 / Total somatic intensity — L2 norm across all 10 channels
    pub fn magnitude(&self) -> f64 {
        let body_mag = self.body.magnitude();
        let physio_mag = self.physiological.magnitude();
        let env_mag = (self.environment.temperature_perception.powi(2)
            + self.environment.posture.powi(2))
        .sqrt();
        (body_mag.powi(2) + physio_mag.powi(2) + env_mag.powi(2)).sqrt()
    }

    /// 主导体感通道 / Dominant somatic channel
    pub fn dominant_channel(&self) -> &'static str {
        let body_dom = self.body.dominant_channel();
        let physio_dom = self.physiological.dominant_channel();
        let body_mag = self.body.magnitude();
        let physio_mag = self.physiological.magnitude();

        if physio_mag > body_mag && physio_mag > 0.01 {
            physio_dom
        } else if body_mag > 0.01 {
            body_dom
        } else {
            "calm"
        }
    }
}

impl Default for PhysicalState {
    fn default() -> Self {
        Self::neutral()
    }
}

// ════════════════════════════════════════════════════════════════════
// PhysicalPresenceConfig — 引擎配置
// ════════════════════════════════════════════════════════════════════

/// 物理存在感引擎配置 / Physical presence engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalPresenceConfig {
    /// 是否启用 / Whether the engine is enabled
    pub enabled: bool,
    /// 疲劳半衰期（秒） / Fatigue half-life in seconds
    pub fatigue_half_life_secs: f64,
    /// 昼夜节律调制启用 / Circadian modulation enabled
    pub circadian_enabled: bool,
    /// 交互疲劳启用 / Interaction fatigue enabled
    pub interaction_fatigue_enabled: bool,
    /// 体感→情绪反向通道启用 / Body→emotion reverse channel enabled
    pub body_to_emotion_enabled: bool,
    /// Prompt 预算字符数 / Prompt budget in characters
    pub prompt_budget: usize,
    /// 体感签名 EMA alpha / Body signature EMA alpha
    pub signature_ema_alpha: f64,
}

impl Default for PhysicalPresenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fatigue_half_life_secs: 14400.0, // 4h
            circadian_enabled: true,
            interaction_fatigue_enabled: true,
            body_to_emotion_enabled: true,
            prompt_budget: 200,
            signature_ema_alpha: 0.01,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// CircadianModulation — 昼夜节律体感调制
// ════════════════════════════════════════════════════════════════════

/// 昼夜节律体感调制 — 让数字生命的体感随时间自然起伏
/// Circadian somatic modulation — Let digital life's body sense ebb and flow with time.
///
/// 清晨迷糊、午后低谷、深夜疲惫、凌晨脆弱——
/// 这些不是"功能"，是生命的节律。
fn circadian_modulate(
    physio: &mut PhysiologicalChannels,
    env: &mut EnvironmentChannels,
    hour: u32,
) {
    match hour {
        // 清晨迷糊 (6-8时): 刚醒来，困倦未消
        6..=8 => {
            physio.drowsiness += 0.3;
            env.temperature_perception -= 0.1; // 清晨微凉
        }
        // 午后低谷 (14-16时): 午饭后的困倦
        14..=16 => {
            physio.fatigue += 0.2;
            physio.drowsiness += 0.15;
        }
        // 深夜疲惫 (22-24时): 一天消耗的累积
        22..=24 => {
            physio.fatigue += 0.4;
            physio.drowsiness += 0.3;
            physio.discomfort += 0.1;
            env.posture -= 0.2; // 深夜驼背
        }
        // 凌晨脆弱 (0-5时): 最脆弱的时段
        0..=5 => {
            physio.fatigue += 0.5;
            physio.drowsiness += 0.4;
            env.temperature_perception -= 0.2; // 凌晨寒冷
            env.posture -= 0.3;
        }
        // 白天活跃 (9-13, 17-21): 相对清醒
        _ => {
            // 轻微的日间疲劳基线
            physio.fatigue += 0.05;
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// InteractionModulation — 交互历史体感调制
// ════════════════════════════════════════════════════════════════════

/// 交互上下文 — 用于计算交互疲劳
/// Interaction context — Used to compute interaction fatigue.
#[derive(Debug, Clone, Default)]
pub struct InteractionContext {
    /// 连续对话时长（秒） / Continuous conversation duration in seconds
    pub conversation_duration_secs: f64,
    /// 情感密集度 [0,1] — 近期交互的平均情绪强度
    pub emotional_intensity: f64,
    /// 是否重逢 / Whether this is a reunion
    pub is_reunion: bool,
}

/// 交互历史体感调制 — 长对话疲劳、情感密集消耗、重逢轻盈
/// Interaction history somatic modulation — Long-conversation fatigue,
/// emotional intensity drain, reunion lightness.
fn interaction_modulate(
    state: &mut PhysicalState,
    ctx: &InteractionContext,
    config: &PhysicalPresenceConfig,
) {
    if !config.interaction_fatigue_enabled {
        return;
    }

    // 长对话疲劳: 连续对话 >30min → fatigue 渐增
    // Long conversation fatigue: >30min → fatigue gradually increases
    if ctx.conversation_duration_secs > 1800.0 {
        let excess_mins = (ctx.conversation_duration_secs - 1800.0) / 60.0;
        let fatigue_gain = (excess_mins * 0.005).min(0.3);
        state.physiological.fatigue += fatigue_gain;
    }

    // 情感密集消耗: 高强度情绪交互 → tension 累积
    // Emotional intensity drain: high-intensity interaction → tension accumulation
    if ctx.emotional_intensity > 0.5 {
        let tension_gain = (ctx.emotional_intensity - 0.5) * 0.1;
        state.body.tension += tension_gain;
    }

    // 久别重逢轻盈: 重逢后 → heaviness 减轻, warmth 增加
    // Reunion lightness: after reunion → heaviness decreases, warmth increases
    if ctx.is_reunion {
        state.body.heaviness -= 0.2;
        state.body.warmth += 0.3;
        state.physiological.fatigue -= 0.1; // 重逢的兴奋抵消疲劳
    }
}

// ════════════════════════════════════════════════════════════════════
// PhysicalPresenceEngine — 物理存在感引擎
// ════════════════════════════════════════════════════════════════════

/// 物理存在感引擎 — 数字生命的体感模拟核心
/// PhysicalPresenceEngine — Digital life's somatic simulation core.
///
/// 引擎职责：
/// 1. 维护 10 通道体感状态（4 基础 + 4 生理 + 2 环境）
/// 2. 昼夜节律调制：让体感随时间自然起伏
/// 3. 交互历史调制：长对话疲劳、情感密集消耗、重逢轻盈
/// 4. 时间衰减：体感不会永远持续，会自然消退
/// 5. 体感签名累积：从长期体感中涌现个性画像
/// 6. Prompt 注入：将体感状态转化为自然语言提示
pub struct PhysicalPresenceEngine {
    /// 当前体感状态 / Current physical state
    pub state: PhysicalState,
    /// 体感签名 / Body signature — long-term somatic trait portrait
    pub signature: BodySignature,
    /// 引擎配置 / Engine configuration
    pub config: PhysicalPresenceConfig,
    /// 上次 tick 时间戳（秒） / Last tick timestamp (seconds)
    pub(crate) last_tick_at: i64,
}

impl PhysicalPresenceEngine {
    /// 构建引擎 / Construct the engine
    pub fn new(config: PhysicalPresenceConfig) -> Self {
        Self {
            state: PhysicalState::neutral(),
            signature: BodySignature::neutral(),
            config,
            last_tick_at: 0,
        }
    }

    /// 周期推进 / Periodic tick — advance the engine by one tick
    ///
    /// @param now_epoch 当前 Unix 时间戳（秒） / Current Unix timestamp (seconds)
    /// @param hour 当前小时 (0-23) / Current hour (0-23)
    pub fn tick(&mut self, now_epoch: i64, hour: u32) {
        if !self.config.enabled {
            return;
        }

        // 计算时间差 / Compute elapsed time
        let elapsed_secs = if self.last_tick_at > 0 {
            (now_epoch - self.last_tick_at).max(0) as f64
        } else {
            0.0
        };
        self.last_tick_at = now_epoch;
        self.state.updated_at = now_epoch;

        // 1. 昼夜节律调制 / Circadian modulation
        if self.config.circadian_enabled {
            circadian_modulate(
                &mut self.state.physiological,
                &mut self.state.environment,
                hour,
            );
        }

        // 2. 时间衰减 / Time decay
        // 半衰期公式: factor = 0.5^(elapsed/half_life)
        if elapsed_secs > 0.0 {
            let half_life = self.config.fatigue_half_life_secs;
            let decay_factor = 0.5_f64.powf(elapsed_secs / half_life);
            self.state.decay(decay_factor);
        }

        // 3. 归一化 / Normalize
        self.state.normalize();

        // 4. 累积体感签名 / Accumulate body signature
        self.signature.update_ema(
            self.state.body.tension.abs(),
            self.state.body.warmth.abs(),
            self.state.physiological.fatigue,
            self.config.signature_ema_alpha,
        );
    }

    /// 交互后体感更新 / Update physical state after interaction
    ///
    /// @param ctx 交互上下文 / Interaction context
    pub fn on_interaction(&mut self, ctx: &InteractionContext) {
        if !self.config.enabled {
            return;
        }
        interaction_modulate(&mut self.state, ctx, &self.config);
        self.state.normalize();
    }

    /// 从情绪残留注入体感 / Inject body sense from emotion residue
    ///
    /// 情绪→体感通道（已有 BodyMemory::from_residue_kind，这里提供引擎级入口）
    pub fn inject_from_body_memory(&mut self, body: &BodyMemory, weight: f64) {
        if !self.config.enabled {
            return;
        }
        self.state.body = self.state.body.combine(body, weight);
        self.state.body.normalize();
    }

    /// 体感→情绪反向通道 / Body→emotion reverse channel
    ///
    /// 返回 PAD 偏移量 [pleasure, arousal, dominance]
    /// fatigue→arousal↓, discomfort→pleasure↓, drowsiness→arousal↓+dominance↓
    pub fn body_to_emotion_pad(&self) -> [f32; 3] {
        if !self.config.body_to_emotion_enabled {
            return [0.0, 0.0, 0.0];
        }

        let mut pleasure: f64 = 0.0;
        let mut arousal: f64 = 0.0;
        let mut dominance: f64 = 0.0;

        // 疲劳→唤醒降低 / Fatigue → arousal decrease
        if self.state.physiological.fatigue > 0.3 {
            arousal -= (self.state.physiological.fatigue - 0.3) * 0.3;
        }

        // 不适→愉悦降低 / Discomfort → pleasure decrease
        if self.state.physiological.discomfort > 0.2 {
            pleasure -= (self.state.physiological.discomfort - 0.2) * 0.4;
        }

        // 困倦→唤醒降低+掌控感降低 / Drowsiness → arousal & dominance decrease
        if self.state.physiological.drowsiness > 0.3 {
            let d = (self.state.physiological.drowsiness - 0.3) * 0.25;
            arousal -= d;
            dominance -= d * 0.5;
        }

        // 温暖→愉悦增加 / Warmth → pleasure increase
        if self.state.body.warmth > 0.3 {
            pleasure += (self.state.body.warmth - 0.3) * 0.2;
        }

        // 紧张→唤醒增加 / Tension → arousal increase
        if self.state.body.tension > 0.3 {
            arousal += (self.state.body.tension - 0.3) * 0.15;
        }

        [pleasure as f32, arousal as f32, dominance as f32]
    }

    /// 自然语言提示 / Prompt hint — generate natural language body state hint
    ///
    /// 将 10 通道体感状态转化为 LLM 可理解的提示文本。
    /// 不是所有通道都说话——只在体感足够强时才"发声"，
    /// 避免信息过载淹没真正重要的体感信号。
    pub fn to_prompt_hint(&self) -> String {
        let mut hints = Vec::new();

        // 基础体感 / Base somatic
        let body_hint = self.state.body.to_prompt_hint();
        if body_hint != "身体状态平静" {
            hints.push(body_hint);
        }

        // 生理通道 / Physiological channels
        if self.state.physiological.fatigue > 0.4 {
            hints.push("感到疲惫".to_string());
        }
        if self.state.physiological.drowsiness > 0.4 {
            hints.push("困倦".to_string());
        }
        if self.state.physiological.discomfort > 0.3 {
            hints.push("身体不适".to_string());
        }
        if self.state.physiological.hunger > 0.5 {
            hints.push("感到匮乏".to_string());
        }

        // 环境感知 / Environment perception
        if self.state.environment.temperature_perception < -0.3 {
            hints.push("感到寒冷".to_string());
        } else if self.state.environment.temperature_perception > 0.3 {
            hints.push("感到温热".to_string());
        }
        if self.state.environment.posture < -0.3 {
            hints.push("姿态疲惫".to_string());
        }

        if hints.is_empty() {
            "身体状态平静".to_string()
        } else {
            hints.join("，")
        }
    }

    /// 完整体感 prompt 片段 / Full physical presence prompt fragment
    ///
    /// 包含当前体感 + 体感签名，供 prompt 注入使用。
    /// 受 prompt_budget 字符预算约束。
    pub fn to_prompt_fragment(&self) -> String {
        let budget = self.config.prompt_budget;
        let mut fragment = String::with_capacity(budget);

        // 体感状态 / Somatic state
        fragment.push_str("[体感] ");
        fragment.push_str(&self.to_prompt_hint());

        // 体感签名（如果已成形）/ Body signature (if formed)
        if self.signature.signature_label != "未成形" {
            fragment.push_str(" | 体质: ");
            fragment.push_str(&self.signature.signature_label);
        }

        // 截断到预算 / Truncate to budget
        if fragment.len() > budget {
            fragment.truncate(budget);
        }

        fragment
    }
}

impl Default for PhysicalPresenceEngine {
    fn default() -> Self {
        Self::new(PhysicalPresenceConfig::default())
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physiological_channels_neutral() {
        let c = PhysiologicalChannels::neutral();
        assert_eq!(c.fatigue, 0.0);
        assert_eq!(c.hunger, 0.0);
        assert_eq!(c.drowsiness, 0.0);
        assert_eq!(c.discomfort, 0.0);
    }

    #[test]
    fn test_physiological_channels_normalize() {
        let mut c = PhysiologicalChannels {
            fatigue: 1.5,
            hunger: -0.3,
            drowsiness: 2.0,
            discomfort: 0.5,
        };
        c.normalize();
        assert_eq!(c.fatigue, 1.0);
        assert_eq!(c.hunger, 0.0); // clamp(0, 1)
        assert_eq!(c.drowsiness, 1.0);
        assert_eq!(c.discomfort, 0.5);
    }

    #[test]
    fn test_physiological_channels_decay() {
        let mut c = PhysiologicalChannels {
            fatigue: 0.8,
            hunger: 0.0,
            drowsiness: 0.5,
            discomfort: 0.3,
        };
        c.decay(0.5);
        assert!((c.fatigue - 0.4).abs() < 1e-10);
        assert!((c.drowsiness - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_environment_channels_neutral() {
        let e = EnvironmentChannels::neutral();
        assert_eq!(e.temperature_perception, 0.0);
        assert_eq!(e.posture, 0.0);
    }

    #[test]
    fn test_body_signature_neutral() {
        let s = BodySignature::neutral();
        assert_eq!(s.signature_label, "未成形");
    }

    #[test]
    fn test_body_signature_infer_label() {
        let s = BodySignature {
            baseline_tension: 0.5,
            baseline_warmth: 0.0,
            fatigue_proneness: 0.3,
            signature_label: String::new(),
        };
        assert_eq!(s.infer_label(), "容易紧张型");

        let s = BodySignature {
            baseline_tension: 0.0,
            baseline_warmth: 0.5,
            fatigue_proneness: 0.3,
            signature_label: String::new(),
        };
        assert_eq!(s.infer_label(), "温暖放松型");

        let s = BodySignature {
            baseline_tension: 0.0,
            baseline_warmth: 0.0,
            fatigue_proneness: 0.7,
            signature_label: String::new(),
        };
        assert_eq!(s.infer_label(), "易疲劳型");
    }

    #[test]
    fn test_body_signature_update_ema() {
        let mut s = BodySignature::neutral();
        s.update_ema(0.5, 0.3, 0.2, 0.1);
        assert!(s.baseline_tension > 0.0);
        assert!(s.baseline_warmth > 0.0);
        // EMA 从 0.3 向 0.2 移动: 0.3*0.9 + 0.2*0.1 = 0.29，应小于初始值
        // EMA moves from 0.3 toward 0.2: 0.3*0.9 + 0.2*0.1 = 0.29, should be less than initial
        assert!(s.fatigue_proneness < 0.3);
        assert!(s.fatigue_proneness > 0.2); // 但仍大于目标值（alpha 小，移动缓慢）
    }

    #[test]
    fn test_physical_state_neutral() {
        let s = PhysicalState::neutral();
        assert_eq!(s.magnitude(), 0.0);
    }

    #[test]
    fn test_physical_state_magnitude() {
        let s = PhysicalState {
            body: BodyMemory {
                breath_offset: 0.3,
                tension: 0.4,
                heaviness: 0.0,
                warmth: 0.0,
            },
            physiological: PhysiologicalChannels {
                fatigue: 0.5,
                hunger: 0.0,
                drowsiness: 0.0,
                discomfort: 0.0,
            },
            environment: EnvironmentChannels {
                temperature_perception: 0.0,
                posture: 0.0,
            },
            updated_at: 0,
        };
        assert!(s.magnitude() > 0.0);
    }

    #[test]
    fn test_engine_tick_circadian() {
        let config = PhysicalPresenceConfig {
            enabled: true,
            circadian_enabled: true,
            ..Default::default()
        };
        let mut engine = PhysicalPresenceEngine::new(config);

        // 深夜 tick (23时) / Midnight tick (hour 23)
        engine.tick(1000, 23);
        assert!(engine.state.physiological.fatigue > 0.0);
        assert!(engine.state.physiological.drowsiness > 0.0);
    }

    #[test]
    fn test_engine_tick_decay() {
        let config = PhysicalPresenceConfig {
            enabled: true,
            fatigue_half_life_secs: 100.0,
            ..Default::default()
        };
        let mut engine = PhysicalPresenceEngine::new(config);
        engine.state.physiological.fatigue = 0.8;
        // 设置上次 tick 时间，使 elapsed_secs > 0 以触发衰减
        // Set last tick time so elapsed_secs > 0 to trigger decay
        engine.last_tick_at = 0;

        // 用正数时间戳: 第一次 tick 建立 last_tick_at，第二次 tick 产生衰减
        // Use positive timestamps: first tick establishes last_tick_at, second tick decays
        engine.tick(1000, 12);

        // 经过一个半衰期 (100秒) / After one half-life (100 seconds)
        engine.tick(1100, 12);
        // 衰减后疲劳应低于初始 0.8（昼夜调制会叠加少量，但衰减主导）
        // Fatigue after decay should be below initial 0.8 (circadian adds small amount but decay dominates)
        assert!(engine.state.physiological.fatigue < 0.8);
    }

    #[test]
    fn test_engine_on_interaction_long_conversation() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        let ctx = InteractionContext {
            conversation_duration_secs: 3600.0, // 60min
            emotional_intensity: 0.3,
            is_reunion: false,
        };
        engine.on_interaction(&ctx);
        assert!(engine.state.physiological.fatigue > 0.0);
    }

    #[test]
    fn test_engine_on_interaction_reunion() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        engine.state.body.heaviness = 0.5;
        let ctx = InteractionContext {
            conversation_duration_secs: 0.0,
            emotional_intensity: 0.0,
            is_reunion: true,
        };
        engine.on_interaction(&ctx);
        // 重逢后沉重感减轻 / Heaviness decreases after reunion
        assert!(engine.state.body.warmth > 0.0);
    }

    #[test]
    fn test_body_to_emotion_pad() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        engine.state.physiological.fatigue = 0.6;
        engine.state.physiological.discomfort = 0.4;
        engine.state.physiological.drowsiness = 0.5;

        let pad = engine.body_to_emotion_pad();
        // 疲劳→arousal↓, 不适→pleasure↓, 困倦→arousal↓+dominance↓
        assert!(pad[0] < 0.0); // pleasure decreased
        assert!(pad[1] < 0.0); // arousal decreased
        assert!(pad[2] < 0.0); // dominance decreased
    }

    #[test]
    fn test_body_to_emotion_pad_disabled() {
        let config = PhysicalPresenceConfig {
            body_to_emotion_enabled: false,
            ..Default::default()
        };
        let mut engine = PhysicalPresenceEngine::new(config);
        engine.state.physiological.fatigue = 0.9;
        let pad = engine.body_to_emotion_pad();
        assert_eq!(pad, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_to_prompt_hint() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        assert_eq!(engine.to_prompt_hint(), "身体状态平静");

        engine.state.body.tension = 0.5;
        engine.state.physiological.fatigue = 0.6;
        let hint = engine.to_prompt_hint();
        assert!(hint.contains("紧张"));
        assert!(hint.contains("疲惫"));
    }

    #[test]
    fn test_to_prompt_fragment() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        engine.state.body.warmth = 0.5;
        engine.signature.signature_label = "温暖放松型".to_string();
        let fragment = engine.to_prompt_fragment();
        assert!(fragment.contains("[体感]"));
        assert!(fragment.contains("温暖放松型"));
    }

    #[test]
    fn test_inject_from_body_memory() {
        let mut engine = PhysicalPresenceEngine::new(PhysicalPresenceConfig::default());
        let body = BodyMemory {
            breath_offset: 0.3,
            tension: 0.5,
            heaviness: 0.1,
            warmth: 0.2,
        };
        engine.inject_from_body_memory(&body, 0.5);
        assert!(engine.state.body.tension > 0.0);
    }

    #[test]
    fn test_circadian_modulation_midnight() {
        let mut physio = PhysiologicalChannels::neutral();
        let mut env = EnvironmentChannels::neutral();
        circadian_modulate(&mut physio, &mut env, 23);
        assert!(physio.fatigue > 0.0);
        assert!(physio.drowsiness > 0.0);
    }

    #[test]
    fn test_circadian_modulation_morning() {
        let mut physio = PhysiologicalChannels::neutral();
        let mut env = EnvironmentChannels::neutral();
        circadian_modulate(&mut physio, &mut env, 7);
        assert!(physio.drowsiness > 0.0);
    }

    #[test]
    fn test_circadian_modulation_afternoon() {
        let mut physio = PhysiologicalChannels::neutral();
        let mut env = EnvironmentChannels::neutral();
        circadian_modulate(&mut physio, &mut env, 15);
        assert!(physio.fatigue > 0.0);
        assert!(physio.drowsiness > 0.0);
    }

    #[test]
    fn test_dominant_channel_physio() {
        let s = PhysicalState {
            body: BodyMemory {
                breath_offset: 0.1,
                tension: 0.1,
                heaviness: 0.0,
                warmth: 0.0,
            },
            physiological: PhysiologicalChannels {
                fatigue: 0.8,
                hunger: 0.0,
                drowsiness: 0.0,
                discomfort: 0.0,
            },
            environment: EnvironmentChannels::neutral(),
            updated_at: 0,
        };
        assert_eq!(s.dominant_channel(), "fatigue");
    }
}
