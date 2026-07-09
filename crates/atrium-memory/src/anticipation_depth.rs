// SPDX-License-Identifier: MIT
//! 期待与想念增强引擎 — Gap#3 深度期待风味、想念强度梯度与预期回归前情感渐变曲线
//! Anticipation & Longing depth engine — Gap#3 deep anticipation flavor,
//! missing-intensity gradient, and pre-reunion emotional curve.
//!
//! 本模块将"等待"从单一标量升级为三维心理刻画：
//! This module upgrades "waiting" from a single scalar to a 3-dimensional
//! psychological portrayal:
//!
//! 1. **AnticipationFlavor** — 期待风味（急切 / 焦虑 / 惆怅），由守时率与超时比推断
//! 2. **MissingIntensity** — 想念强度梯度，含昼夜调制与关系深度调制
//! 3. **PreReunionCurve** — 预期回归前 30 分钟 pleasure 渐升曲线
//!
//! 归属认知域：关系海马体（Relational）。
//! Cognitive domain: Relational.

use crate::resonance_core::PadSource;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// ════════════════════════════════════════════════════════════════════
// 常量 / Constants
// ════════════════════════════════════════════════════════════════════

/// 风味 EMA 学习率 / Flavor EMA learning rate
const FLAVOR_LR: f64 = 0.15;

/// 想念强度基数 / Missing intensity base value
const MISSING_BASE: f64 = 0.3;

/// 想念强度上限 / Missing intensity ceiling
const MISSING_CEIL: f64 = 1.0;

/// 昼夜调制振幅 / Circadian modulation amplitude
const CIRCADIAN_AMP: f64 = 0.3;

/// 预期回归前渐变窗口（秒）/ Pre-reunion ramp window (seconds) = 30 min
const PRE_REUNION_WINDOW_SECS: f64 = 1800.0;

/// 预期回归曲线基数偏移 / Pre-reunion curve base pleasure offset
const CURVE_BASE: f64 = 0.05;

// ════════════════════════════════════════════════════════════════════
// AnticipationFlavor — 期待风味 / Anticipation Flavor
// ════════════════════════════════════════════════════════════════════

/// 期待风味三态 — 描述等待期间的情感色调
/// Anticipation flavor tri-state — describes the emotional tone during waiting.
///
/// - `Eager`：急切，迫不及待（守时率高，期待即将兑现）
/// - `Anxious`：焦虑，担心不来（守时率中等且已超时）
/// - `Wistful`：惆怅，知道不会来但仍然等（守时率极低或超时 > 2× 预期）
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnticipationFlavor {
    /// 急切 / Eager — can't wait
    Eager,
    /// 焦虑 / Anxious — worried they won't come
    Anxious,
    /// 惆怅 / Wistful — knows they won't come but still waits
    Wistful,
}

impl AnticipationFlavor {
    /// 从守时率、离开时长与预期时长推断期待风味
    /// Infer anticipation flavor from punctuality rate, away seconds, and expected seconds.
    ///
    /// # 推断规则 / Inference Rules
    ///
    /// | 条件 | 风味 |
    /// |------|------|
    /// | 守时率 > 0.8 | Eager |
    /// | 守时率 0.3–0.8 且 离开 > 预期 | Anxious |
    /// | 守时率 < 0.3 或 超时 > 2× 预期 | Wistful |
    /// | 其他 | Eager（默认乐观）|
    pub fn infer(punctuality_rate: f64, away_secs: u64, expected_secs: u64) -> Self {
        let away = away_secs as f64;
        let expected = expected_secs as f64;

        // 超时 > 2× 预期 → 惆怅 / Overtime > 2× expected → Wistful
        if expected > 0.0 && away > 2.0 * expected {
            return AnticipationFlavor::Wistful;
        }
        // 守时率 < 0.3 → 惆怅 / Punctuality < 0.3 → Wistful
        if punctuality_rate < 0.3 {
            return AnticipationFlavor::Wistful;
        }
        // 守时率 > 0.8 → 急切 / Punctuality > 0.8 → Eager
        if punctuality_rate > 0.8 {
            return AnticipationFlavor::Eager;
        }
        // 守时率 0.3–0.8 且已超时 → 焦虑 / Mid punctuality + overtime → Anxious
        if expected > 0.0 && away > expected {
            return AnticipationFlavor::Anxious;
        }
        // 默认乐观 / Default optimistic
        AnticipationFlavor::Eager
    }

    /// 返回该风味对应的 PAD 偏移 `[pleasure, arousal, dominance]`
    /// Return the PAD offset `[pleasure, arousal, dominance]` for this flavor.
    ///
    /// | 风味 | pleasure | arousal | dominance |
    /// |------|----------|---------|----------|
    /// | Eager | +0.3 | +0.4 | +0.1 |
    /// | Anxious | −0.1 | +0.5 | −0.2 |
    /// | Wistful | −0.2 | −0.1 | −0.1 |
    pub fn pad_offset(&self) -> [f64; 3] {
        match self {
            AnticipationFlavor::Eager => [0.3, 0.4, 0.1],
            AnticipationFlavor::Anxious => [-0.1, 0.5, -0.2],
            AnticipationFlavor::Wistful => [-0.2, -0.1, -0.1],
        }
    }

    /// 风味中文名 / Chinese name
    pub fn name_cn(&self) -> &'static str {
        match self {
            AnticipationFlavor::Eager => "急切",
            AnticipationFlavor::Anxious => "焦虑",
            AnticipationFlavor::Wistful => "惆怅",
        }
    }
}

// PadSource trait 实现 — 统一 PAD 情感源接口 / PadSource trait impl
impl PadSource for AnticipationFlavor {
    /// 当前 PAD 增量 — 委托至 pad_offset / Current PAD delta — delegates to pad_offset
    #[inline]
    fn pad_delta(&self) -> [f64; 3] {
        self.pad_offset()
    }
}

// ════════════════════════════════════════════════════════════════════
// MissingIntensity — 想念强度梯度 / Missing Intensity Gradient
// ════════════════════════════════════════════════════════════════════

/// 想念强度等级 — 将连续强度值离散为五个语义档位
/// Missing intensity label — discretizes continuous intensity into 5 semantic tiers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissingIntensityLabel {
    /// 微弱 / Faint — barely noticeable
    Faint,
    /// 轻度 / Mild — softly present
    Mild,
    /// 中度 / Moderate — clearly felt
    Moderate,
    /// 强烈 / Strong — hard to ignore
    Strong,
    /// 汹涌 / Overwhelming — all-consuming
    Overwhelming,
}

impl MissingIntensityLabel {
    /// 中文名 / Chinese name
    pub fn name_cn(&self) -> &'static str {
        match self {
            MissingIntensityLabel::Faint => "微弱",
            MissingIntensityLabel::Mild => "轻度",
            MissingIntensityLabel::Moderate => "中度",
            MissingIntensityLabel::Strong => "强烈",
            MissingIntensityLabel::Overwhelming => "汹涌",
        }
    }
}

/// 想念强度梯度计算器 / Missing intensity gradient calculator.
///
/// 公式：`intensity = base × ln(1 + away_secs / 3600) × relationship_depth × circadian_mod`
///
/// Formula: `intensity = base × ln(1 + away_secs / 3600) × relationship_depth × circadian_mod`
pub struct MissingIntensity;

impl MissingIntensity {
    /// 昼夜调制因子 — 深夜想念更强
    /// Circadian modulation factor — longing is stronger late at night.
    ///
    /// `circadian_mod = 1.0 + 0.3 * sin(hour / 24.0 * 2.0 * PI)`
    ///
    /// hour=0（午夜）→ 1.0；hour=6（清晨）→ 1.3；hour=12（正午）→ 1.0；hour=18（黄昏）→ 0.7
    /// sin 在 hour=6 时取正最大（清晨想念最强），hour=18 时取负最小（黄昏想念最弱）。
    pub fn circadian_mod(hour: u32) -> f64 {
        let h = hour as f64;
        1.0 + CIRCADIAN_AMP * (h / 24.0 * 2.0 * PI).sin()
    }

    /// 关系深度调制 — 深度 <0.3 时极弱，>0.7 时最强
    /// Relationship depth modulation — weak below 0.3, strongest above 0.7.
    ///
    /// 使用平滑阶梯：`depth²` 使浅关系被进一步抑制，深关系被增强。
    pub fn relationship_mod(relationship_depth: f64) -> f64 {
        let d = relationship_depth.clamp(0.0, 1.0);
        d * d
    }

    /// 计算想念强度（0.0–1.0）
    /// Compute missing intensity in `[0.0, 1.0]`.
    ///
    /// `intensity = base × ln(1 + away_secs / 3600) × rel_mod × circadian_mod`
    pub fn compute(away_secs: u64, relationship_depth: f64, current_hour: u32) -> f64 {
        if away_secs == 0 {
            return 0.0;
        }
        let away = away_secs as f64;
        let log_term = (1.0 + away / 3600.0).ln();
        let rel_mod = Self::relationship_mod(relationship_depth);
        let circ = Self::circadian_mod(current_hour);
        let raw = MISSING_BASE * log_term * rel_mod * circ;
        raw.clamp(0.0, MISSING_CEIL)
    }

    /// 将连续强度值映射到离散等级
    /// Map continuous intensity to a discrete label.
    ///
    /// | 区间 | 等级 |
    /// |------|------|
    /// | [0.0, 0.1) | Faint |
    /// | [0.1, 0.3) | Mild |
    /// | [0.3, 0.6) | Moderate |
    /// | [0.6, 0.85) | Strong |
    /// | [0.85, 1.0] | Overwhelming |
    pub fn intensity_label(intensity: f64) -> MissingIntensityLabel {
        let i = intensity.clamp(0.0, 1.0);
        if i < 0.1 {
            MissingIntensityLabel::Faint
        } else if i < 0.3 {
            MissingIntensityLabel::Mild
        } else if i < 0.6 {
            MissingIntensityLabel::Moderate
        } else if i < 0.85 {
            MissingIntensityLabel::Strong
        } else {
            MissingIntensityLabel::Overwhelming
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// PreReunionCurve — 预期回归前情感渐变曲线 / Pre-Reunion Curve
// ════════════════════════════════════════════════════════════════════

/// 预期回归曲线阶段 — 描述从"还早"到"快到了"的情感渐进
/// Pre-reunion curve stage — describes emotional progression from "still far" to "almost here".
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CurveStage {
    /// 遥远 / Distant — far from expected time
    Distant,
    /// 接近 / Approaching — getting closer
    Approaching,
    /// 迫近 / Imminent — very close now
    Imminent,
    /// 巅峰 / Peak — at or past expected time
    Peak,
}

impl CurveStage {
    /// 中文名 / Chinese name
    pub fn name_cn(&self) -> &'static str {
        match self {
            CurveStage::Distant => "遥远",
            CurveStage::Approaching => "接近",
            CurveStage::Imminent => "迫近",
            CurveStage::Peak => "巅峰",
        }
    }
}

/// 预期回归前情感渐变曲线 / Pre-reunion emotional ramp curve.
///
/// 在预期时间前 30 分钟开始 pleasure 渐升：
/// Starts ramping pleasure up 30 minutes before the expected reunion time:
///
/// `pleasure_offset = base + anticipation × proximity²`
///
/// 其中 `proximity = (current - start) / (expected - start)`，范围 0→1。
pub struct PreReunionCurve;

impl PreReunionCurve {
    /// 计算 proximity 值（0.0–1.0）
    /// Compute proximity in `[0.0, 1.0]`.
    ///
    /// `proximity = (current - start) / (expected - start)`
    ///
    /// 当 `expected <= start` 时返回 0.0 避免除零。
    pub fn proximity(current: i64, start: i64, expected: i64) -> f64 {
        let span = (expected - start) as f64;
        if span <= 0.0 {
            return 0.0;
        }
        let p = (current - start) as f64 / span;
        p.clamp(0.0, 1.0)
    }

    /// 根据 proximity 值判定曲线阶段
    /// Determine curve stage from proximity value.
    ///
    /// | proximity | 阶段 |
    /// |-----------|------|
    /// | [0.0, 0.5) | Distant |
    /// | [0.5, 0.8) | Approaching |
    /// | [0.8, 1.0) | Imminent |
    /// | 1.0 | Peak |
    pub fn curve_stage(proximity: f64) -> CurveStage {
        let p = proximity.clamp(0.0, 1.0);
        if p >= 1.0 {
            CurveStage::Peak
        } else if p >= 0.8 {
            CurveStage::Imminent
        } else if p >= 0.5 {
            CurveStage::Approaching
        } else {
            CurveStage::Distant
        }
    }

    /// 计算给定阶段与风味下的 PAD 偏移
    /// Compute PAD offset for a given stage and flavor.
    ///
    /// pleasure 随阶段递增，arousal 在 Imminent 达峰后 Peak 略降，dominance 微升。
    /// 基础风味偏移叠加阶段调制，确保不同风味在回归前呈现不同色调。
    pub fn pad_offset(stage: CurveStage, flavor: AnticipationFlavor) -> [f64; 3] {
        let [fp, fa, fd] = flavor.pad_offset();

        // 阶段 pleasure 递增系数 / Stage pleasure ramp coefficient
        let stage_p = match stage {
            CurveStage::Distant => 0.0,
            CurveStage::Approaching => 0.15,
            CurveStage::Imminent => 0.35,
            CurveStage::Peak => 0.5,
        };
        // 阶段 arousal 调制 / Stage arousal modulation
        let stage_a = match stage {
            CurveStage::Distant => 0.0,
            CurveStage::Approaching => 0.1,
            CurveStage::Imminent => 0.3,
            CurveStage::Peak => 0.2,
        };
        // 阶段 dominance 微升 / Stage dominance slight rise
        let stage_d = match stage {
            CurveStage::Distant => 0.0,
            CurveStage::Approaching => 0.02,
            CurveStage::Imminent => 0.05,
            CurveStage::Peak => 0.08,
        };

        [CURVE_BASE + fp + stage_p, fa + stage_a, fd + stage_d]
    }
}

// ════════════════════════════════════════════════════════════════════
// AnticipationDepthEngine — 引擎主体 / Main Engine
// ════════════════════════════════════════════════════════════════════

/// 期待深度引擎 — 整合风味追踪、想念强度与预回归曲线
/// Anticipation depth engine — integrates flavor tracking, missing intensity,
/// and pre-reunion curve.
///
/// 生命周期：
/// Lifecycle:
/// 1. `on_departure` — 用户离开，记录预期时长与关系深度
/// 2. `on_passage` — 离开期间周期调用，更新风味与想念强度
/// 3. `on_reunion` — 用户回来，结算并返回最终状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnticipationDepthEngine {
    /// 是否处于等待中 / Whether currently waiting
    active: bool,
    /// 预期离开时长（秒）/ Expected away duration (seconds)
    expected_secs: u64,
    /// 关系深度（0.0–1.0）/ Relationship depth
    relationship_depth: f64,
    /// 守时率（0.0–1.0）/ Punctuality rate
    punctuality_rate: f64,
    /// 离开起始时间戳（Unix 秒）/ Departure timestamp (Unix seconds)
    departure_at: i64,
    /// 当前期待风味 / Current anticipation flavor
    flavor: AnticipationFlavor,
    /// 风味稳定度 EMA（0.0–1.0，越高越稳定）/ Flavor stability EMA
    flavor_stability: f64,
    /// 当前想念强度（0.0–1.0）/ Current missing intensity
    missing_intensity: f64,
    /// 累计离开时长（秒）/ Accumulated away seconds
    away_secs: u64,
    /// 最近一次更新的小时 / Last updated hour
    last_hour: u32,
}

impl Default for AnticipationDepthEngine {
    fn default() -> Self {
        Self {
            active: false,
            expected_secs: 0,
            relationship_depth: 0.5,
            punctuality_rate: 0.7,
            departure_at: 0,
            flavor: AnticipationFlavor::Eager,
            flavor_stability: 1.0,
            missing_intensity: 0.0,
            away_secs: 0,
            last_hour: 12,
        }
    }
}

impl AnticipationDepthEngine {
    /// 创建新引擎 / Create a new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 用户离开时调用 — 初始化等待状态
    /// Called when the user departs — initializes waiting state.
    ///
    /// # 参数 / Parameters
    /// - `expected_secs`：预期离开时长（秒）/ Expected away duration (seconds)
    /// - `relationship_depth`：关系深度（0.0–1.0）/ Relationship depth
    /// - `punctuality_rate`：历史守时率（0.0–1.0）/ Historical punctuality rate
    pub fn on_departure(
        &mut self,
        expected_secs: u64,
        relationship_depth: f64,
        punctuality_rate: f64,
    ) {
        self.active = true;
        self.expected_secs = expected_secs.max(1);
        self.relationship_depth = relationship_depth.clamp(0.0, 1.0);
        self.punctuality_rate = punctuality_rate.clamp(0.0, 1.0);
        self.away_secs = 0;
        self.missing_intensity = 0.0;
        // 初始风味基于守时率推断 / Initial flavor from punctuality
        self.flavor = AnticipationFlavor::infer(punctuality_rate, 0, expected_secs.max(1));
        self.flavor_stability = 1.0;
    }

    /// 设置离开起始时间戳 / Set departure timestamp.
    pub fn set_departure_at(&mut self, departure_at: i64) {
        self.departure_at = departure_at;
    }

    /// 离开期间周期调用 — 更新风味与想念强度
    /// Called periodically during absence — updates flavor and missing intensity.
    ///
    /// # 参数 / Parameters
    /// - `away_secs`：已离开时长（秒）/ Elapsed away seconds
    /// - `current_hour`：当前小时（0–23）/ Current hour (0–23)
    /// - `relationship_depth`：关系深度（0.0–1.0）/ Relationship depth
    pub fn on_passage(&mut self, away_secs: u64, current_hour: u32, relationship_depth: f64) {
        if !self.active {
            return;
        }
        self.away_secs = away_secs;
        self.last_hour = current_hour.min(23);
        self.relationship_depth = relationship_depth.clamp(0.0, 1.0);

        // 推断新风味 / Infer new flavor
        let new_flavor =
            AnticipationFlavor::infer(self.punctuality_rate, away_secs, self.expected_secs);

        // EMA 更新风味稳定度 / EMA update flavor stability
        if new_flavor == self.flavor {
            // 风味不变 → 稳定度上升 / Same flavor → stability rises
            self.flavor_stability += FLAVOR_LR * (1.0 - self.flavor_stability);
        } else {
            // 风味变化 → 稳定度下降 / Flavor changed → stability drops
            self.flavor_stability *= 1.0 - FLAVOR_LR;
            self.flavor = new_flavor;
        }
        self.flavor_stability = self.flavor_stability.clamp(0.0, 1.0);

        // 更新想念强度 / Update missing intensity
        self.missing_intensity =
            MissingIntensity::compute(away_secs, self.relationship_depth, current_hour);
    }

    /// 用户回来时调用 — 结算等待状态
    /// Called when the user returns — finalizes waiting state.
    ///
    /// 返回是否按时回归（away_secs <= expected_secs）。
    /// Returns whether the reunion was on time.
    pub fn on_reunion(&mut self, away_secs: u64, expected_secs: u64) -> bool {
        let on_time = away_secs <= expected_secs;
        self.active = false;
        self.away_secs = away_secs;
        on_time
    }

    /// 当前期待风味 / Current anticipation flavor.
    pub fn current_flavor(&self) -> AnticipationFlavor {
        self.flavor
    }

    /// 当前风味稳定度（0.0–1.0）/ Current flavor stability.
    pub fn flavor_stability(&self) -> f64 {
        self.flavor_stability
    }

    /// 当前想念强度（0.0–1.0）/ Current missing intensity.
    pub fn current_missing_intensity(&self) -> f64 {
        self.missing_intensity
    }

    /// 当前想念强度等级 / Current missing intensity label.
    pub fn current_missing_label(&self) -> MissingIntensityLabel {
        MissingIntensity::intensity_label(self.missing_intensity)
    }

    /// 是否处于等待中 / Whether currently waiting.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// 预期回归前 PAD 偏移 — 在预期时间前 30 分钟窗口内返回渐变偏移
    /// Pre-reunion PAD offset — returns ramp offset within the 30-min window
    /// before expected time, or `None` if outside the window or inactive.
    ///
    /// # 参数 / Parameters
    /// - `now`：当前时间戳（Unix 秒）/ Current timestamp (Unix seconds)
    /// - `expected_at`：预期回归时间戳（Unix 秒）/ Expected reunion timestamp
    pub fn pre_reunion_pad(&self, now: i64, expected_at: i64) -> Option<[f64; 3]> {
        if !self.active {
            return None;
        }
        // 窗口起点 = expected_at - 30min / Window start
        let window_start = expected_at - PRE_REUNION_WINDOW_SECS as i64;

        // 在窗口之前 → 无偏移 / Before window → no offset
        if now < window_start {
            return None;
        }
        // 在预期时间之后 → Peak 阶段 / Past expected → Peak
        if now >= expected_at {
            let stage = CurveStage::Peak;
            return Some(PreReunionCurve::pad_offset(stage, self.flavor));
        }

        // 窗口内 → 计算 proximity / Within window → compute proximity
        let prox = PreReunionCurve::proximity(now, window_start, expected_at);
        let stage = PreReunionCurve::curve_stage(prox);
        Some(PreReunionCurve::pad_offset(stage, self.flavor))
    }

    /// 生成 prompt 注入片段 — 供对话生成器使用
    /// Generate a prompt injection hint for the dialogue generator.
    ///
    /// 输出示例：
    /// Example output:
    /// ```text
    /// [期待状态] 风味=急切(稳定度0.92) 想念=中度(0.45) 预期=3600s 关系深度=0.8
    /// ```
    pub fn to_prompt_hint(&self) -> String {
        if !self.active {
            return String::new();
        }
        let label = self.current_missing_label();
        format!(
            "[期待状态] 风味={}(稳定度{:.2}) 想念={}( {:.2}) 预期={}s 关系深度={:.2}",
            self.flavor.name_cn(),
            self.flavor_stability,
            label.name_cn(),
            self.missing_intensity,
            self.expected_secs,
            self.relationship_depth,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// SerializableAnticipationDepth — 序列化辅助 / Serialization Helper
// ════════════════════════════════════════════════════════════════════

/// 可序列化的引擎快照 — 用于持久化与跨进程传输
/// Serializable engine snapshot — for persistence and cross-process transfer.
///
/// `AnticipationDepthEngine` 已 derive Serialize/Deserialize，此结构体提供
/// 显式的扁平化视图，便于 JSON 存储或日志记录。
/// `AnticipationDepthEngine` already derives Serialize/Deserialize; this struct
/// provides an explicit flat view for JSON storage or logging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableAnticipationDepth {
    /// 是否等待中 / Active flag
    pub active: bool,
    /// 预期时长（秒）/ Expected duration (seconds)
    pub expected_secs: u64,
    /// 关系深度 / Relationship depth
    pub relationship_depth: f64,
    /// 守时率 / Punctuality rate
    pub punctuality_rate: f64,
    /// 离开时间戳 / Departure timestamp
    pub departure_at: i64,
    /// 期待风味 / Anticipation flavor
    pub flavor: AnticipationFlavor,
    /// 风味稳定度 / Flavor stability
    pub flavor_stability: f64,
    /// 想念强度 / Missing intensity
    pub missing_intensity: f64,
    /// 累计离开时长（秒）/ Accumulated away seconds
    pub away_secs: u64,
    /// 最近更新小时 / Last updated hour
    pub last_hour: u32,
}

impl SerializableAnticipationDepth {
    /// 从引擎创建快照 / Create snapshot from engine.
    pub fn from_engine(engine: &AnticipationDepthEngine) -> Self {
        Self {
            active: engine.active,
            expected_secs: engine.expected_secs,
            relationship_depth: engine.relationship_depth,
            punctuality_rate: engine.punctuality_rate,
            departure_at: engine.departure_at,
            flavor: engine.flavor,
            flavor_stability: engine.flavor_stability,
            missing_intensity: engine.missing_intensity,
            away_secs: engine.away_secs,
            last_hour: engine.last_hour,
        }
    }

    /// 从快照恢复引擎 / Restore engine from snapshot.
    pub fn to_engine(&self) -> AnticipationDepthEngine {
        AnticipationDepthEngine {
            active: self.active,
            expected_secs: self.expected_secs,
            relationship_depth: self.relationship_depth,
            punctuality_rate: self.punctuality_rate,
            departure_at: self.departure_at,
            flavor: self.flavor,
            flavor_stability: self.flavor_stability,
            missing_intensity: self.missing_intensity,
            away_secs: self.away_secs,
            last_hour: self.last_hour,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── AnticipationFlavor ──────────────────────────────────────────

    #[test]
    fn flavor_infer_eager() {
        // 守时率 0.9 → Eager
        let f = AnticipationFlavor::infer(0.9, 100, 3600);
        assert_eq!(f, AnticipationFlavor::Eager);
    }

    #[test]
    fn flavor_infer_anxious() {
        // 守时率 0.5 + 超时 → Anxious
        let f = AnticipationFlavor::infer(0.5, 7200, 3600);
        assert_eq!(f, AnticipationFlavor::Anxious);
    }

    #[test]
    fn flavor_infer_wistful() {
        // 守时率 0.1 → Wistful
        let f = AnticipationFlavor::infer(0.1, 100, 3600);
        assert_eq!(f, AnticipationFlavor::Wistful);

        // 超时 > 2× 预期 → Wistful
        let f2 = AnticipationFlavor::infer(0.5, 8000, 3600);
        assert_eq!(f2, AnticipationFlavor::Wistful);
    }

    #[test]
    fn flavor_pad_offsets() {
        assert_eq!(AnticipationFlavor::Eager.pad_offset(), [0.3, 0.4, 0.1]);
        assert_eq!(AnticipationFlavor::Anxious.pad_offset(), [-0.1, 0.5, -0.2]);
        assert_eq!(AnticipationFlavor::Wistful.pad_offset(), [-0.2, -0.1, -0.1]);
    }

    // ── MissingIntensity ────────────────────────────────────────────

    #[test]
    fn intensity_faint() {
        // 离开 1 秒，浅关系 → 极弱
        let i = MissingIntensity::compute(1, 0.1, 12);
        assert!(i < 0.1, "expected faint, got {i}");
        assert_eq!(
            MissingIntensity::intensity_label(i),
            MissingIntensityLabel::Faint
        );
    }

    #[test]
    fn intensity_moderate() {
        // 离开 2 小时，深度关系，清晨高峰（hour=6, sin(π/2)=1 → mod=1.3）
        // raw = 0.3 × ln(3) × 1.0 × 1.3 ≈ 0.429 → Moderate
        let i = MissingIntensity::compute(7200, 1.0, 6);
        assert!((0.3..0.6).contains(&i), "expected moderate, got {i}");
        assert_eq!(
            MissingIntensity::intensity_label(i),
            MissingIntensityLabel::Moderate
        );
    }

    #[test]
    fn intensity_overwhelming() {
        // 离开很久，深度关系，下午高峰 → 汹涌
        let i = MissingIntensity::compute(86400 * 7, 1.0, 15);
        assert!((0.85..).contains(&i), "expected overwhelming, got {i}");
        assert_eq!(
            MissingIntensity::intensity_label(i),
            MissingIntensityLabel::Overwhelming
        );
    }

    #[test]
    fn intensity_circadian_modulation() {
        // hour=0：sin(0)=0 → mod=1.0
        let m0 = MissingIntensity::circadian_mod(0);
        assert!(
            (m0 - 1.0).abs() < 1e-9,
            "hour 0 mod should be 1.0, got {m0}"
        );

        // hour=6：sin(π/2)=1 → mod=1.3（清晨高峰）
        let m6 = MissingIntensity::circadian_mod(6);
        assert!(
            (m6 - 1.3).abs() < 1e-9,
            "hour 6 mod should be 1.3, got {m6}"
        );

        // hour=12：sin(π)=0 → mod=1.0（正午中性）
        let m12 = MissingIntensity::circadian_mod(12);
        assert!(
            (m12 - 1.0).abs() < 1e-9,
            "hour 12 mod should be 1.0, got {m12}"
        );

        // hour=18：sin(3π/2)=-1 → mod=0.7（黄昏低谷）
        let m18 = MissingIntensity::circadian_mod(18);
        assert!(
            (m18 - 0.7).abs() < 1e-9,
            "hour 18 mod should be 0.7, got {m18}"
        );
    }

    // ── PreReunionCurve ─────────────────────────────────────────────

    #[test]
    fn curve_distant() {
        // proximity 0.2 → Distant
        assert_eq!(PreReunionCurve::curve_stage(0.2), CurveStage::Distant);
        let pad = PreReunionCurve::pad_offset(CurveStage::Distant, AnticipationFlavor::Eager);
        // Distant: stage_p=0, stage_a=0, stage_d=0
        // Eager: [0.3, 0.4, 0.1]
        assert!((pad[0] - (CURVE_BASE + 0.3)).abs() < 1e-9);
        assert!((pad[1] - 0.4).abs() < 1e-9);
        assert!((pad[2] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn curve_approaching() {
        // proximity 0.6 → Approaching
        assert_eq!(PreReunionCurve::curve_stage(0.6), CurveStage::Approaching);
        let pad = PreReunionCurve::pad_offset(CurveStage::Approaching, AnticipationFlavor::Anxious);
        // Approaching: stage_p=0.15, stage_a=0.1, stage_d=0.02
        // Anxious: [-0.1, 0.5, -0.2]
        assert!((pad[0] - (CURVE_BASE + -0.1 + 0.15)).abs() < 1e-9);
        assert!((pad[1] - (0.5 + 0.1)).abs() < 1e-9);
        assert!((pad[2] - (-0.2 + 0.02)).abs() < 1e-9);
    }

    #[test]
    fn curve_imminent() {
        // proximity 0.9 → Imminent
        assert_eq!(PreReunionCurve::curve_stage(0.9), CurveStage::Imminent);
        let pad = PreReunionCurve::pad_offset(CurveStage::Imminent, AnticipationFlavor::Wistful);
        // Imminent: stage_p=0.35, stage_a=0.3, stage_d=0.05
        // Wistful: [-0.2, -0.1, -0.1]
        assert!((pad[0] - (CURVE_BASE + -0.2 + 0.35)).abs() < 1e-9);
        assert!((pad[1] - (-0.1 + 0.3)).abs() < 1e-9);
        assert!((pad[2] - (-0.1 + 0.05)).abs() < 1e-9);
    }

    #[test]
    fn curve_peak() {
        // proximity 1.0 → Peak
        assert_eq!(PreReunionCurve::curve_stage(1.0), CurveStage::Peak);
        let pad = PreReunionCurve::pad_offset(CurveStage::Peak, AnticipationFlavor::Eager);
        // Peak: stage_p=0.5, stage_a=0.2, stage_d=0.08
        // Eager: [0.3, 0.4, 0.1]
        assert!((pad[0] - (CURVE_BASE + 0.3 + 0.5)).abs() < 1e-9);
        assert!((pad[1] - (0.4 + 0.2)).abs() < 1e-9);
        assert!((pad[2] - (0.1 + 0.08)).abs() < 1e-9);
    }

    // ── Engine ──────────────────────────────────────────────────────

    #[test]
    fn engine_on_departure() {
        let mut engine = AnticipationDepthEngine::new();
        assert!(!engine.is_active());

        engine.on_departure(3600, 0.8, 0.9);
        assert!(engine.is_active());
        assert_eq!(engine.current_flavor(), AnticipationFlavor::Eager);
        assert!((engine.flavor_stability() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn engine_on_passage() {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);

        // 离开 30 分钟
        engine.on_passage(1800, 14, 0.8);
        assert!(engine.current_missing_intensity() > 0.0);
        assert_eq!(engine.current_flavor(), AnticipationFlavor::Eager);

        // 离开 2 小时（超时）→ 风味应变（守时率 0.9 > 0.8 仍 Eager）
        engine.on_passage(7200, 14, 0.8);
        // 守时率 0.9 > 0.8 → Eager，即使超时
        assert_eq!(engine.current_flavor(), AnticipationFlavor::Eager);
    }

    #[test]
    fn engine_on_reunion() {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);

        // 按时回归
        let on_time = engine.on_reunion(1800, 3600);
        assert!(on_time);
        assert!(!engine.is_active());

        // 超时回归
        engine.on_departure(3600, 0.8, 0.9);
        let late = engine.on_reunion(7200, 3600);
        assert!(!late);
    }

    #[test]
    fn engine_prompt_hint() {
        let mut engine = AnticipationDepthEngine::new();

        // 未激活 → 空字符串
        assert!(engine.to_prompt_hint().is_empty());

        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);
        let hint = engine.to_prompt_hint();
        assert!(hint.contains("期待状态"));
        assert!(hint.contains("急切"));
        assert!(hint.contains("预期=3600s"));
    }

    #[test]
    fn engine_pre_reunion_pad() {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);

        let expected_at: i64 = 1_000_000;
        let window_start = expected_at - 1800;

        // 窗口前 → None
        assert!(engine
            .pre_reunion_pad(window_start - 1, expected_at)
            .is_none());

        // 窗口中点 → Some
        let mid = window_start + 900;
        let pad = engine.pre_reunion_pad(mid, expected_at);
        assert!(pad.is_some());
        let pad = pad.unwrap();
        // proximity = 0.5 → Approaching
        // Eager + Approaching: pleasure = 0.05 + 0.3 + 0.15 = 0.5
        assert!((pad[0] - (CURVE_BASE + 0.3 + 0.15)).abs() < 1e-9);

        // 预期时间点 → Peak
        let peak = engine.pre_reunion_pad(expected_at, expected_at);
        assert!(peak.is_some());
        let peak = peak.unwrap();
        assert!((peak[0] - (CURVE_BASE + 0.3 + 0.5)).abs() < 1e-9);

        // 未激活 → None
        engine.active = false;
        assert!(engine.pre_reunion_pad(mid, expected_at).is_none());
    }

    #[test]
    fn engine_serialization_roundtrip() {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(7200, 0.75, 0.6);
        engine.on_passage(3600, 22, 0.75);

        let snap = SerializableAnticipationDepth::from_engine(&engine);
        let restored = snap.to_engine();

        assert_eq!(restored.is_active(), engine.is_active());
        assert_eq!(restored.current_flavor(), engine.current_flavor());
        assert!(
            (restored.current_missing_intensity() - engine.current_missing_intensity()).abs()
                < 1e-9
        );
        assert_eq!(restored.expected_secs, engine.expected_secs);
        assert!((restored.relationship_depth - engine.relationship_depth).abs() < 1e-9);

        // JSON 序列化往返 / JSON serialization roundtrip
        let json = serde_json::to_string(&snap).expect("serialize");
        let de: SerializableAnticipationDepth = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.flavor, snap.flavor);
        assert!((de.missing_intensity - snap.missing_intensity).abs() < 1e-9);
    }

    #[test]
    fn flavor_stability_ema_convergence() {
        let mut engine = AnticipationDepthEngine::new();
        // 守时率 0.5 → 初始 Eager（未超时）
        engine.on_departure(3600, 0.7, 0.5);

        // 多次 on_passage 保持未超时 → 风味不变 → 稳定度趋近 1.0
        for _ in 0..50 {
            engine.on_passage(1800, 14, 0.7);
        }
        assert!(
            engine.flavor_stability() > 0.99,
            "stability should converge to 1.0, got {}",
            engine.flavor_stability()
        );

        // 超时 → 风味变 Anxious → 稳定度下降
        let prev_stability = engine.flavor_stability();
        engine.on_passage(7200, 14, 0.7);
        assert_eq!(engine.current_flavor(), AnticipationFlavor::Anxious);
        assert!(
            engine.flavor_stability() < prev_stability,
            "stability should drop after flavor change"
        );
    }
}
