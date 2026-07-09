// SPDX-License-Identifier: MIT
//! 成长反馈桥接 — G-08 自我成长闭环反馈信号强化
//! Growth Feedback Bridge — G-08 Self-Growth Closed-Loop Feedback Enhancement
//!
//! 核心理念：数字生命的成长应来自真实互动反馈，而非时间流逝。
//! FeedbackLoop 已从每条用户消息提取 5 类信号（Praise/Correction/Frustration/TopicShift/Deepening），
//! 但这些信号从不回流到 VulnerabilityWisdom / ImperfectionWarmth 成长引擎。
//! 本桥接器将 FeedbackLoop 信号转换为 AmbientFeedback，以弱信号（学习率×0.1）持续微调成长引擎。
//!
//! Core idea: digital life growth should come from real interaction feedback, not time passage.
//! FeedbackLoop already extracts 5 signal types from every user message, but these never flow back
//! to the growth engines. This bridge converts FeedbackLoop signals to AmbientFeedback, continuously
//! fine-tuning growth engines with weak signals (learning rate ×0.1).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::resonance_core::ema_f32;

// ═══════════════════════════════════════════════════════════════════
//  反馈来源类型 / Feedback source kind
// ═══════════════════════════════════════════════════════════════════

/// 成长反馈来源类型 — 对应 FeedbackLoop 的 5 类信号 + Neutral / Growth feedback source kind
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeedbackKind {
    /// 赞扬 — 用户对 AI 回复表示满意 / Praise
    Praise,
    /// 纠正 — 用户纠正 AI 的错误 / Correction
    Correction,
    /// 挫败 — 用户表现出不耐烦 / Frustration
    Frustration,
    /// 深入追问 — 用户对话题感兴趣 / Deepening
    Deepening,
    /// 话题切换 — 用户转移话题 / TopicShift
    TopicShift,
    /// 中性 — 无明显反馈信号 / Neutral
    Neutral,
}

impl FeedbackKind {
    /// 信号效价 [-1, 1] — 正向=成长加速，负向=成长减速 / Signal valence
    pub fn valence(self) -> f32 {
        match self {
            Self::Praise => 0.3,
            Self::Correction => -0.2,
            Self::Frustration => -0.4,
            Self::Deepening => 0.2,
            Self::TopicShift => -0.05,
            Self::Neutral => 0.0,
        }
    }

    /// 是否正向信号 / Whether positive signal
    pub fn is_positive(self) -> bool {
        self.valence() > 0.0
    }

    /// 是否负向信号 / Whether negative signal
    pub fn is_negative(self) -> bool {
        self.valence() < 0.0
    }

    /// 类型靶向映射 — 指定反馈类型偏向更新哪类脆弱维度 / Targeted vulnerability type mapping
    ///
    /// 设计决策 / Design decisions:
    /// - Correction → Uncertainty（用户纠正 → 不确定性维度受影响）
    /// - Frustration → SelfDoubt（用户挫败 → 自我怀疑维度受影响）
    /// - Deepening → LimitationHonesty（用户深入追问 → 诚实承认局限受肯定）
    /// - TopicShift → ModerateMistake（用户转移话题 → 可能是中等错误的信号）
    /// - Praise → None（赞扬是全局安全信号，不靶向特定类型）
    /// - Neutral → None（中性信号无靶向）
    pub fn vuln_type_target(self) -> Option<crate::vulnerability_window::VulnerabilityType> {
        use crate::vulnerability_window::VulnerabilityType;
        match self {
            Self::Correction => Some(VulnerabilityType::Uncertainty),
            Self::Frustration => Some(VulnerabilityType::SelfDoubt),
            Self::Deepening => Some(VulnerabilityType::LimitationHonesty),
            Self::TopicShift => Some(VulnerabilityType::ModerateMistake),
            Self::Praise | Self::Neutral => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  环境反馈信号 / Ambient feedback signal
// ═══════════════════════════════════════════════════════════════════

/// 环境反馈信号 — 非脆弱/不完美触发的连续反馈 / Ambient feedback signal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmbientFeedback {
    /// 信号效价 [-1, 1] / Signal valence
    pub valence: f32,
    /// 来源类型 / Source kind
    pub source: FeedbackKind,
    /// 时间戳（epoch secs）/ Timestamp
    pub timestamp: i64,
}

impl AmbientFeedback {
    /// 创建环境反馈 / Create ambient feedback
    pub fn new(source: FeedbackKind, timestamp: i64) -> Self {
        Self {
            valence: source.valence(),
            source,
            timestamp,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  成长势头跟踪器 / Growth momentum tracker
// ═══════════════════════════════════════════════════════════════════

/// 成长势头跟踪器 — 跟踪反馈密度×质量，输出成长速率系数 / Growth momentum tracker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrowthAccumulator {
    /// 成长势头 [0, 1]，初始 0.5 / Growth momentum [0, 1], initial 0.5
    momentum: f32,
    /// 最近反馈密度窗口（true=正向，false=负向/无）/ Recent feedback density window
    recent_density: VecDeque<bool>,
    /// 窗口上限 / Window capacity
    window_capacity: usize,
}

impl Default for GrowthAccumulator {
    fn default() -> Self {
        Self {
            momentum: 0.5,
            recent_density: VecDeque::new(),
            window_capacity: 20,
        }
    }
}

impl GrowthAccumulator {
    /// 创建跟踪器 / Create accumulator
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一条反馈，更新成长势头 / Record a feedback signal, updating momentum
    pub fn record_feedback(&mut self, source: FeedbackKind) {
        // 推入密度窗口 / Push to density window
        self.recent_density.push_back(source.is_positive());
        // 修剪到窗口上限（丢弃最旧）/ Trim to window capacity (drop oldest)
        while self.recent_density.len() > self.window_capacity {
            self.recent_density.pop_front();
        }
        // 更新势头 / Update momentum
        let (target, alpha) = if source.is_positive() {
            // 正向：朝 1.0 拉 / Positive: pull toward 1.0
            (1.0, 0.1)
        } else if source.is_negative() {
            // 负向：朝 0.0 拉 / Negative: pull toward 0.0
            (0.0, 0.1)
        } else {
            // 中性：缓慢漂回 0.5 / Neutral: slow drift toward 0.5
            (0.5, 0.05)
        };
        self.momentum = ema_f32(self.momentum, target, alpha);
    }

    /// 每拍衰减 — 朝 0.5 缓慢回归 / Per-tick decay — slow regression toward 0.5
    pub fn tick_decay(&mut self) {
        self.momentum = ema_f32(self.momentum, 0.5, 0.005);
    }

    /// 成长速率系数 [0.8, 1.2] / Growth rate coefficient
    pub fn growth_rate_coefficient(&self) -> f32 {
        if self.momentum > 0.7 {
            1.2
        } else if self.momentum < 0.3 {
            0.8
        } else {
            // 线性插值 [0.3, 0.7] → [0.8, 1.2] / Linear interpolation
            let coeff = 0.8 + (self.momentum - 0.3) / 0.4 * 0.4;
            coeff.clamp(0.8, 1.2)
        }
    }

    /// 当前成长势头 / Current momentum
    pub fn momentum(&self) -> f32 {
        self.momentum
    }

    /// 提示词片段 — 成长势头诊断 / Prompt fragment — momentum diagnostic
    pub fn prompt_fragment(&self) -> String {
        // 无任何反馈时返回空串 / Return empty when no feedback recorded yet
        if self.recent_density.is_empty() {
            return String::new();
        }
        let rate = self.growth_rate_coefficient();
        if self.momentum > 0.7 {
            format!(
                "[成长势头/GrowthMomentum] 高 ({:.2}) 速率系数: {:.1}x — 互动反馈密集，成长加速",
                self.momentum, rate
            )
        } else if self.momentum < 0.3 {
            format!(
                "[成长势头/GrowthMomentum] 低 ({:.2}) 速率系数: {:.1}x — 反馈稀疏，成长减速",
                self.momentum, rate
            )
        } else {
            format!(
                "[成长势头/GrowthMomentum] 中 ({:.2}) 速率系数: {:.1}x",
                self.momentum, rate
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  桥接交换结果 / Bridge exchange result
// ═══════════════════════════════════════════════════════════════════

/// 桥接交换结果 — 供调用方诊断 / Bridge exchange result for caller diagnostics
#[derive(Clone, Debug)]
pub struct GrowthExchangeResult {
    /// 本次反馈效价 / Feedback valence
    pub valence: f32,
    /// 当前成长速率系数 / Current growth rate coefficient
    pub growth_rate: f32,
}

// ═══════════════════════════════════════════════════════════════════
//  成长反馈桥接器 / Growth feedback bridge
// ═══════════════════════════════════════════════════════════════════

/// 成长反馈桥接器 — 桥接 FeedbackLoop → VulnerabilityWisdom + ImperfectionWarmth / Growth feedback bridge
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GrowthFeedbackBridge {
    /// 成长势头跟踪器 / Growth accumulator
    accumulator: GrowthAccumulator,
}

impl GrowthFeedbackBridge {
    /// 创建桥接器 / Create bridge
    pub fn new() -> Self {
        Self::default()
    }

    /// 每次交换时调用 — 将 FeedbackLoop 信号转为 AmbientFeedback 并更新势头 / Called on each exchange
    ///
    /// 注意：本方法不直接调用 wisdom/warmth —— 调用方使用返回的 AmbientFeedback 自行驱动。
    /// NOTE: this method does NOT directly call wisdom/warmth — the caller does that using the AmbientFeedback.
    /// This keeps the bridge pure and testable.
    pub fn on_exchange(&mut self, source: FeedbackKind, now_epoch: i64) -> GrowthExchangeResult {
        // 构造环境反馈（供调用方使用）/ Build ambient feedback (for caller use)
        let _ambient = AmbientFeedback::new(source, now_epoch);
        // 记录到势头跟踪器 / Record to accumulator
        self.accumulator.record_feedback(source);
        GrowthExchangeResult {
            valence: source.valence(),
            growth_rate: self.accumulator.growth_rate_coefficient(),
        }
    }

    /// 每拍衰减 / Per-tick decay
    pub fn tick_decay(&mut self) {
        self.accumulator.tick_decay();
    }

    /// 访问势头跟踪器 / Access accumulator
    pub fn accumulator(&self) -> &GrowthAccumulator {
        &self.accumulator
    }

    /// 提示词片段 — 委托给跟踪器 / Prompt fragment — delegates to accumulator
    pub fn prompt_fragment(&self) -> String {
        self.accumulator.prompt_fragment()
    }
}

// ═══════════════════════════════════════════════════════════════════
//  成长桥接持久化存储 / Growth Bridge Persistence Store
// ═══════════════════════════════════════════════════════════════════

/// 成长桥接持久化存储 — sled bincode 封装 / Growth bridge persistence store
///
/// 跨重启保持 GrowthFeedbackBridge 的 momentum 连续，避免成长势头断裂。
/// Persists GrowthFeedbackBridge momentum across restarts to maintain growth continuity.
pub struct GrowthBridgeStore {
    /// sled 数据库句柄 / sled database handle
    db: sled::Db,
}

impl GrowthBridgeStore {
    /// 打开持久化存储 / Open persistence store
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// 内存模式（测试用）/ Open in-memory (for tests)
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("growth bridge store init");
        Self { db }
    }

    /// 保存成长桥接 / Save growth bridge
    pub fn save(&self, bridge: &GrowthFeedbackBridge) -> anyhow::Result<()> {
        let key = b"__growth_bridge__";
        let bytes = bincode::serialize(bridge)?;
        self.db.insert(key, bytes)?;
        self.db.flush()?;
        Ok(())
    }

    /// 加载成长桥接 / Load growth bridge
    ///
    /// 无持久化文件时优雅降级，返回默认 GrowthFeedbackBridge::new()。
    /// Graceful degradation: returns default GrowthFeedbackBridge::new() when no persisted data.
    pub fn load(&self) -> anyhow::Result<GrowthFeedbackBridge> {
        let key = b"__growth_bridge__";
        match self.db.get(key)? {
            Some(bytes) => {
                let bridge: GrowthFeedbackBridge = bincode::deserialize(&bytes)?;
                Ok(bridge)
            }
            None => Ok(GrowthFeedbackBridge::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 浮点近似比较 / Float approximate equality
    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn test_feedback_kind_valence() {
        assert!(approx_eq(FeedbackKind::Praise.valence(), 0.3));
        assert!(approx_eq(FeedbackKind::Correction.valence(), -0.2));
        assert!(approx_eq(FeedbackKind::Frustration.valence(), -0.4));
        assert!(approx_eq(FeedbackKind::Deepening.valence(), 0.2));
        assert!(approx_eq(FeedbackKind::TopicShift.valence(), -0.05));
        assert!(approx_eq(FeedbackKind::Neutral.valence(), 0.0));
        // 正/负向判定 / Positive/negative classification
        assert!(FeedbackKind::Praise.is_positive());
        assert!(FeedbackKind::Deepening.is_positive());
        assert!(FeedbackKind::Correction.is_negative());
        assert!(FeedbackKind::Frustration.is_negative());
        assert!(FeedbackKind::TopicShift.is_negative());
        assert!(!FeedbackKind::Neutral.is_positive());
        assert!(!FeedbackKind::Neutral.is_negative());
    }

    #[test]
    fn test_ambient_feedback_new() {
        let af = AmbientFeedback::new(FeedbackKind::Praise, 1000);
        assert!(approx_eq(af.valence, 0.3));
        assert_eq!(af.source, FeedbackKind::Praise);
        assert_eq!(af.timestamp, 1000);
    }

    #[test]
    fn test_growth_accumulator_default_momentum() {
        let acc = GrowthAccumulator::default();
        assert!(approx_eq(acc.momentum(), 0.5));
        assert!(acc.recent_density.is_empty());
        assert_eq!(acc.window_capacity, 20);
    }

    #[test]
    fn test_growth_accumulator_positive_feedback_raises_momentum() {
        let mut acc = GrowthAccumulator::default();
        for _ in 0..10 {
            acc.record_feedback(FeedbackKind::Praise);
        }
        assert!(acc.momentum() > 0.5);
    }

    #[test]
    fn test_growth_accumulator_negative_feedback_lowers_momentum() {
        let mut acc = GrowthAccumulator::default();
        for _ in 0..10 {
            acc.record_feedback(FeedbackKind::Frustration);
        }
        assert!(acc.momentum() < 0.5);
    }

    #[test]
    fn test_growth_accumulator_decay_toward_half() {
        let mut acc = GrowthAccumulator::default();
        // 先抬高势头 / Raise momentum first
        for _ in 0..20 {
            acc.record_feedback(FeedbackKind::Praise);
        }
        let high = acc.momentum();
        assert!(high > 0.5);
        // 衰减 100 拍 / Decay 100 ticks
        for _ in 0..100 {
            acc.tick_decay();
        }
        let after = acc.momentum();
        assert!(after < high);
        assert!((after - 0.5).abs() < (high - 0.5).abs());
    }

    #[test]
    fn test_growth_rate_coefficient_high_momentum() {
        let mut acc = GrowthAccumulator::default();
        for _ in 0..30 {
            acc.record_feedback(FeedbackKind::Praise);
        }
        assert!(acc.momentum() > 0.7);
        assert!(approx_eq(acc.growth_rate_coefficient(), 1.2));
    }

    #[test]
    fn test_growth_rate_coefficient_low_momentum() {
        let mut acc = GrowthAccumulator::default();
        for _ in 0..30 {
            acc.record_feedback(FeedbackKind::Frustration);
        }
        assert!(acc.momentum() < 0.3);
        assert!(approx_eq(acc.growth_rate_coefficient(), 0.8));
    }

    #[test]
    fn test_growth_rate_coefficient_mid_momentum() {
        let acc = GrowthAccumulator::default();
        // 默认势头 0.5 → 中段插值 / Default momentum 0.5 → mid interpolation
        assert!(approx_eq(acc.momentum(), 0.5));
        let coeff = acc.growth_rate_coefficient();
        assert!(coeff >= 0.8 && coeff <= 1.2);
        assert!(approx_eq(coeff, 1.0));
    }

    #[test]
    fn test_bridge_on_exchange_praise() {
        let mut bridge = GrowthFeedbackBridge::new();
        let result = bridge.on_exchange(FeedbackKind::Praise, 12345);
        assert!(result.valence > 0.0);
        assert!(result.growth_rate >= 0.5 && result.growth_rate <= 1.5);
    }

    #[test]
    fn test_bridge_on_exchange_correction() {
        let mut bridge = GrowthFeedbackBridge::new();
        let result = bridge.on_exchange(FeedbackKind::Correction, 1);
        assert!(result.valence < 0.0);
    }

    #[test]
    fn test_bridge_tick_decay() {
        let mut bridge = GrowthFeedbackBridge::new();
        for _ in 0..20 {
            bridge.on_exchange(FeedbackKind::Praise, 1);
        }
        let before = bridge.accumulator().momentum();
        bridge.tick_decay();
        let after = bridge.accumulator().momentum();
        assert!(after < before);
    }

    #[test]
    fn test_prompt_fragment_empty() {
        let bridge = GrowthFeedbackBridge::new();
        let frag = bridge.prompt_fragment();
        assert!(frag.is_empty());
    }

    #[test]
    fn test_prompt_fragment_nonempty() {
        let mut bridge = GrowthFeedbackBridge::new();
        bridge.on_exchange(FeedbackKind::Praise, 1);
        let frag = bridge.prompt_fragment();
        assert!(!frag.is_empty());
        assert!(frag.contains("GrowthMomentum"));
    }

    #[test]
    fn test_recent_density_window_limit() {
        let mut acc = GrowthAccumulator::default();
        for _ in 0..30 {
            acc.record_feedback(FeedbackKind::Praise);
        }
        assert_eq!(acc.recent_density.len(), 20);
    }

    #[test]
    fn test_vuln_type_target_mapping() {
        use crate::vulnerability_window::VulnerabilityType;
        assert_eq!(
            FeedbackKind::Correction.vuln_type_target(),
            Some(VulnerabilityType::Uncertainty)
        );
        assert_eq!(
            FeedbackKind::Frustration.vuln_type_target(),
            Some(VulnerabilityType::SelfDoubt)
        );
        assert_eq!(
            FeedbackKind::Deepening.vuln_type_target(),
            Some(VulnerabilityType::LimitationHonesty)
        );
        assert_eq!(
            FeedbackKind::TopicShift.vuln_type_target(),
            Some(VulnerabilityType::ModerateMistake)
        );
        assert_eq!(FeedbackKind::Praise.vuln_type_target(), None);
        assert_eq!(FeedbackKind::Neutral.vuln_type_target(), None);
    }

    #[test]
    fn test_growth_bridge_serialize_deserialize_roundtrip() {
        let mut bridge = GrowthFeedbackBridge::new();
        // 记录多次正向反馈提升 momentum / Record multiple positive feedbacks to raise momentum
        for _ in 0..10 {
            bridge.on_exchange(FeedbackKind::Praise, 1000);
        }
        let momentum_before = bridge.accumulator().momentum();
        assert!(momentum_before > 0.5); // 确认势头确实提升 / Confirm momentum actually raised

        // bincode 序列化后反序列化 / bincode serialize then deserialize
        let bytes = bincode::serialize(&bridge).unwrap();
        let restored: GrowthFeedbackBridge = bincode::deserialize(&bytes).unwrap();

        let momentum_after = restored.accumulator().momentum();
        assert!(
            approx_eq(momentum_before, momentum_after),
            "序列化前后 momentum 应一致: before={}, after={}",
            momentum_before,
            momentum_after
        );
    }
}
