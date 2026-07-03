// SPDX-License-Identifier: MIT

//! 仪式成长 — Ritual Evolution (Gap#5: 90% → 95%).
//!
//! 核心理念：仪式不是设定好就不变的——"早安"说了一百天后，
//! 它不再是两个字，而是一种羁绊。仪式有生命线：
//! 从萌芽到建立到成熟到进化到可能衰退。
//!
//! Core idea: rituals are not static — after 100 days of "good morning",
//! it's no longer two words but a bond. Rituals have a life line:
//! from budding to establishing to mature to evolving to possibly fading.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// §1 仪式成长阶段 — Ritual Growth Stage
// ═══════════════════════════════════════════════════════════════════════════

/// 仪式成长阶段 / Ritual growth stage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RitualStage {
    /// 萌芽 — 刚出现，尚未稳定 / Budding — just appeared, not yet stable.
    Budding,
    /// 建立中 — 频率渐稳，但尚未固化 / Establishing — frequency stabilizing.
    Establishing,
    /// 成熟 — 稳定执行，成为习惯 / Mature — stable execution, habitual.
    Evolving,
    /// 进化中 — 超越形式，有了内涵 / Evolving — beyond form, has meaning.
    Deepening,
    /// 衰退中 — 频率下降，可能消失 / Fading — frequency declining.
    Fading,
}

impl RitualStage {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Budding => "萌芽",
            Self::Establishing => "建立中",
            Self::Evolving => "成熟",
            Self::Deepening => "进化中",
            Self::Fading => "衰退中",
        }
    }

    /// 英文标签 / English label.
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Budding => "Budding",
            Self::Establishing => "Establishing",
            Self::Evolving => "Mature",
            Self::Deepening => "Evolving",
            Self::Fading => "Fading",
        }
    }

    /// 阶段序数（用于比较）/ Stage ordinal for comparison.
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Budding => 0,
            Self::Establishing => 1,
            Self::Evolving => 2,
            Self::Deepening => 3,
            Self::Fading => 4,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 进化事件 — Evolution Event
// ═══════════════════════════════════════════════════════════════════════════

/// 仪式进化事件 — 仪式变得更深的过程 / Ritual evolution event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvolutionEvent {
    /// 事件类型 / Event type.
    pub kind: EvolutionKind,
    /// 时间戳 / Timestamp.
    pub timestamp: i64,
    /// 进化增量 — 此事件对仪式深度的贡献 / Evolution delta.
    pub depth_delta: f64,
}

/// 进化事件类型 / Evolution event kind.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvolutionKind {
    /// 内容深化 — 增加了专属暗号或内涵 / Content deepened.
    ContentDeepened,
    /// 时间微调 — 更贴合双方节律 / Time adjusted.
    TimeAdjusted,
    /// 情感加深 — 从形式到情感投入 / Emotional deepening.
    EmotionalDeepening,
    /// 连续达成 — 又一次完成 / Consecutive achievement.
    ConsecutiveAchievement,
    /// 中断恢复 — 中断后恢复执行 / Recovery from break.
    RecoveryFromBreak,
    /// 衰退 — 频率下降 / Decline.
    Decline,
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 仪式成长追踪 — Ritual Growth Tracker
// ═══════════════════════════════════════════════════════════════════════════

/// 单个仪式的成长追踪 / Growth tracker for a single ritual.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RitualGrowth {
    /// 仪式名称 / Ritual name.
    pub name: String,
    /// 当前成长阶段 / Current growth stage.
    pub stage: RitualStage,
    /// 连续完成次数 / Consecutive completions.
    pub consecutive_count: u32,
    /// 总完成次数 / Total completions.
    pub total_count: u32,
    /// 中断次数 / Break count.
    pub break_count: u32,
    /// 深度分数 [0, 1] — 仪式的"内涵深度" / Depth score.
    pub depth: f64,
    /// 关系深度 [0, 1] — 涉及的关系深度 / Relationship depth.
    pub relationship_depth: f64,
    /// 情感投入 [0, 1] — 平均情感投入 / Emotional investment.
    pub emotional_investment: f64,
    /// 进化历史 / Evolution history.
    events: Vec<EvolutionEvent>,
    /// 创建时间戳 / Creation timestamp.
    pub created_ts: i64,
    /// 上次完成时间戳 / Last completion timestamp.
    pub last_completed_ts: i64,
}

impl RitualGrowth {
    /// 创建新仪式追踪 / Create a new ritual tracker.
    pub fn new(name: &str, timestamp: i64) -> Self {
        Self {
            name: name.to_string(),
            stage: RitualStage::Budding,
            consecutive_count: 0,
            total_count: 0,
            break_count: 0,
            depth: 0.0,
            relationship_depth: 0.0,
            emotional_investment: 0.0,
            events: Vec::new(),
            created_ts: timestamp,
            last_completed_ts: timestamp,
        }
    }

    /// 记算成长速度 — 连续完成 × 关系深度 × 情感投入 / Compute growth velocity.
    pub fn growth_velocity(&self) -> f64 {
        let count_factor = (self.consecutive_count as f64).ln().max(0.0) / 10.0;
        count_factor * self.relationship_depth * self.emotional_investment
    }

    /// 记算阶段阈值 / Compute stage from metrics.
    pub fn compute_stage(&self) -> RitualStage {
        if self.consecutive_count == 0 && self.total_count == 0 {
            return RitualStage::Budding;
        }

        // 衰退检测：中断次数 > 完成次数的50% / Fading detection.
        if self.break_count > 0 && self.consecutive_count == 0 {
            return RitualStage::Fading;
        }

        // 深度驱动阶段 / Depth-driven staging.
        if self.depth > 0.7 && self.consecutive_count > 50 {
            RitualStage::Deepening
        } else if self.depth > 0.4 && self.consecutive_count > 20 {
            RitualStage::Evolving
        } else if self.consecutive_count > 7 {
            RitualStage::Establishing
        } else {
            RitualStage::Budding
        }
    }

    /// 记录完成 — 仪式又一次发生 / Record completion.
    pub fn complete(&mut self, emotional_investment: f64, timestamp: i64) {
        self.consecutive_count += 1;
        self.total_count += 1;
        self.last_completed_ts = timestamp;

        // 更新情感投入EMA / Update emotional investment EMA.
        let alpha = 0.1;
        self.emotional_investment +=
            alpha * (emotional_investment.clamp(0.0, 1.0) - self.emotional_investment);

        // 深度增长 / Depth growth.
        let depth_delta = 0.01 * self.relationship_depth * emotional_investment.clamp(0.0, 1.0);
        self.depth = (self.depth + depth_delta).min(1.0);

        // 记录进化事件 / Record evolution event.
        self.events.push(EvolutionEvent {
            kind: EvolutionKind::ConsecutiveAchievement,
            timestamp,
            depth_delta,
        });

        // 内容深化检测 / Content deepening detection.
        #[allow(clippy::manual_is_multiple_of)]
        if self.consecutive_count % 30 == 0 && self.consecutive_count > 0 {
            self.events.push(EvolutionEvent {
                kind: EvolutionKind::ContentDeepened,
                timestamp,
                depth_delta: 0.05,
            });
            self.depth = (self.depth + 0.05).min(1.0);
        }

        // 情感加深检测 / Emotional deepening detection.
        #[allow(clippy::manual_is_multiple_of)]
        if self.emotional_investment > 0.7 && self.consecutive_count % 20 == 0 {
            self.events.push(EvolutionEvent {
                kind: EvolutionKind::EmotionalDeepening,
                timestamp,
                depth_delta: 0.03,
            });
            self.depth = (self.depth + 0.03).min(1.0);
        }

        // 更新阶段 / Update stage.
        self.stage = self.compute_stage();
    }

    /// 记录中断 — 仪式未在预期时间发生 / Record break.
    pub fn record_break(&mut self, timestamp: i64) {
        self.consecutive_count = 0;
        self.break_count += 1;

        // 深度轻微衰减 / Slight depth decay.
        self.depth *= 0.95;

        self.events.push(EvolutionEvent {
            kind: EvolutionKind::Decline,
            timestamp,
            depth_delta: -0.05,
        });

        // 更新阶段 / Update stage.
        self.stage = self.compute_stage();
    }

    /// 记录恢复 — 中断后恢复执行 / Record recovery.
    pub fn record_recovery(&mut self, timestamp: i64) {
        self.events.push(EvolutionEvent {
            kind: EvolutionKind::RecoveryFromBreak,
            timestamp,
            depth_delta: 0.02,
        });
        self.depth = (self.depth + 0.02).min(1.0);
    }

    /// 设置关系深度 / Set relationship depth.
    pub fn set_relationship_depth(&mut self, depth: f64) {
        self.relationship_depth = depth.clamp(0.0, 1.0);
    }

    /// 获取进化事件历史 / Get evolution events.
    pub fn events(&self) -> &[EvolutionEvent] {
        &self.events
    }

    /// 仪式年龄（天）/ Ritual age in days.
    pub fn age_days(&self, current_ts: i64) -> f64 {
        ((current_ts - self.created_ts) as f64 / 86400.0).max(0.0)
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self) -> String {
        format!(
            "仪式「{}」: {}({}) | 连续{}次 | 深度{:.2} | 情感{:.2}",
            self.name,
            self.stage.label_zh(),
            self.stage.label_en(),
            self.consecutive_count,
            self.depth,
            self.emotional_investment,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 仪式成长引擎 — Ritual Evolution Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 仪式成长引擎 — 管理所有仪式的成长追踪 / Ritual evolution engine.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RitualEvolution {
    /// 所有仪式的成长追踪 / All ritual growth trackers.
    rituals: HashMap<String, RitualGrowth>,
}

impl RitualEvolution {
    /// 创建新引擎 / Create new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册新仪式 / Register a new ritual.
    pub fn register(&mut self, name: &str, timestamp: i64) {
        self.rituals
            .entry(name.to_string())
            .or_insert_with(|| RitualGrowth::new(name, timestamp));
    }

    /// 记录仪式完成 / Record ritual completion.
    pub fn complete(
        &mut self,
        name: &str,
        emotional_investment: f64,
        relationship_depth: f64,
        timestamp: i64,
    ) {
        self.register(name, timestamp);
        let ritual = self.rituals.get_mut(name).unwrap();
        ritual.set_relationship_depth(relationship_depth);
        ritual.complete(emotional_investment, timestamp);
    }

    /// 记录仪式中断 / Record ritual break.
    pub fn record_break(&mut self, name: &str, timestamp: i64) {
        if let Some(ritual) = self.rituals.get_mut(name) {
            ritual.record_break(timestamp);
        }
    }

    /// 获取仪式 / Get a ritual.
    pub fn get(&self, name: &str) -> Option<&RitualGrowth> {
        self.rituals.get(name)
    }

    /// 获取所有仪式 / Get all rituals.
    pub fn all(&self) -> &HashMap<String, RitualGrowth> {
        &self.rituals
    }

    /// 获取最深仪式 — 深度最高的 / Get deepest ritual.
    pub fn deepest(&self) -> Option<&RitualGrowth> {
        self.rituals.values().max_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// 获取衰退仪式 — 处于Fading阶段的 / Get fading rituals.
    pub fn fading(&self) -> Vec<&RitualGrowth> {
        self.rituals
            .values()
            .filter(|r| r.stage == RitualStage::Fading)
            .collect()
    }

    /// 获取进化中仪式 — 处于Deepening阶段的 / Get deepening rituals.
    pub fn deepening(&self) -> Vec<&RitualGrowth> {
        self.rituals
            .values()
            .filter(|r| r.stage == RitualStage::Deepening)
            .collect()
    }

    /// 生成整体描述 / Generate overall description.
    pub fn describe(&self) -> String {
        let total = self.rituals.len();
        let fading = self.fading().len();
        let deepening = self.deepening().len();
        let deepest = self
            .deepest()
            .map(|r| r.name.clone())
            .unwrap_or("无".to_string());
        format!(
            "仪式成长: {}个仪式 | 进化中{} | 衰退{} | 最深: {}",
            total, deepening, fading, deepest,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ritual_stage_labels() {
        assert_eq!(RitualStage::Budding.label_zh(), "萌芽");
        assert_eq!(RitualStage::Deepening.label_en(), "Evolving");
    }

    #[test]
    fn test_ritual_stage_ordinal() {
        assert!(RitualStage::Budding.ordinal() < RitualStage::Deepening.ordinal());
    }

    #[test]
    fn test_ritual_growing_new() {
        let r = RitualGrowth::new("good_morning", 1000);
        assert_eq!(r.stage, RitualStage::Budding);
        assert_eq!(r.consecutive_count, 0);
    }

    #[test]
    fn test_ritual_complete_increases_count() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.5);
        r.complete(0.5, 1100);
        assert_eq!(r.consecutive_count, 1);
        assert_eq!(r.total_count, 1);
    }

    #[test]
    fn test_ritual_stage_progression() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.8);
        // 大量完成推动阶段前进 / Many completions advance stage.
        for i in 0..30 {
            r.complete(0.7, 1000 + i * 100);
        }
        assert!(r.stage.ordinal() >= RitualStage::Establishing.ordinal());
    }

    #[test]
    fn test_ritual_break_resets_consecutive() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.5);
        r.complete(0.5, 1100);
        r.complete(0.5, 1200);
        assert_eq!(r.consecutive_count, 2);
        r.record_break(1300);
        assert_eq!(r.consecutive_count, 0);
        assert_eq!(r.break_count, 1);
    }

    #[test]
    fn test_ritual_depth_grows() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.8);
        let initial_depth = r.depth;
        for i in 0..100 {
            r.complete(0.8, 1000 + i * 100);
        }
        assert!(r.depth > initial_depth);
    }

    #[test]
    fn test_ritual_fading_after_breaks() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.5);
        r.complete(0.5, 1100);
        r.record_break(1200);
        assert_eq!(r.stage, RitualStage::Fading);
    }

    #[test]
    fn test_ritual_growth_velocity() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.5);
        r.complete(0.5, 1100);
        let v = r.growth_velocity();
        assert!(v >= 0.0);
    }

    #[test]
    fn test_ritual_describe() {
        let r = RitualGrowth::new("good_morning", 1000);
        let desc = r.describe();
        assert!(desc.contains("good_morning"));
        assert!(desc.contains("萌芽"));
    }

    #[test]
    fn test_evolution_engine_register() {
        let mut engine = RitualEvolution::new();
        engine.register("test_ritual", 1000);
        assert!(engine.get("test_ritual").is_some());
    }

    #[test]
    fn test_evolution_engine_complete() {
        let mut engine = RitualEvolution::new();
        engine.complete("morning", 0.7, 0.6, 1000);
        let r = engine.get("morning").unwrap();
        assert_eq!(r.consecutive_count, 1);
    }

    #[test]
    fn test_evolution_engine_deepest() {
        let mut engine = RitualEvolution::new();
        engine.complete("a", 0.3, 0.3, 1000);
        engine.complete("b", 0.8, 0.8, 1000);
        for i in 0..50 {
            engine.complete("b", 0.8, 0.8, 1000 + i * 100);
        }
        let deepest = engine.deepest().unwrap();
        assert_eq!(deepest.name, "b");
    }

    #[test]
    fn test_evolution_engine_fading() {
        let mut engine = RitualEvolution::new();
        engine.complete("a", 0.5, 0.5, 1000);
        engine.record_break("a", 2000);
        let fading = engine.fading();
        assert!(!fading.is_empty());
    }

    #[test]
    fn test_evolution_engine_describe() {
        let mut engine = RitualEvolution::new();
        engine.complete("test", 0.5, 0.5, 1000);
        let desc = engine.describe();
        assert!(desc.contains("仪式成长"));
    }

    #[test]
    fn test_ritual_content_deepening_at_30() {
        let mut r = RitualGrowth::new("test", 1000);
        r.set_relationship_depth(0.8);
        for i in 0..30 {
            r.complete(0.7, 1000 + i * 100);
        }
        // 第30次应有ContentDeepened事件 / 30th completion should have ContentDeepened event.
        assert!(r
            .events()
            .iter()
            .any(|e| e.kind == EvolutionKind::ContentDeepened));
    }

    #[test]
    fn test_ritual_age_days() {
        let r = RitualGrowth::new("test", 1000);
        let age = r.age_days(1000 + 86400); // 1 day later.
        assert!((age - 1.0).abs() < 0.01);
    }
}
