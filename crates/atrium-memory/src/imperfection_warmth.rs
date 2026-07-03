// SPDX-License-Identifier: MIT

//! 不完美温度 — Imperfection Warmth (Gap#9: 90% → 95%).
//!
//! 核心理念：不完美不是冷冰冰的"错误率"，而是有温度的——
//! 一个小错误让数字生命更"有人味"，但太多错误会失去信任。
//! 不完美温度衡量"恰到好处的犯错"：足够真实，但不至于不可靠。
//!
//! Core idea: imperfection is not a cold "error rate", it has temperature —
//! a small mistake makes the digital life more "human", but too many erode trust.
//! Imperfection warmth measures "just right mistakes": authentic enough, but not unreliable.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 不完美类型 — Imperfection Type
// ═══════════════════════════════════════════════════════════════════════════

/// 不完美类型 — 不同的错误有不同的"温度" / Imperfection type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImperfectionKind {
    /// 记忆偏差 — 记错细节，很有人味 / MemoryDeviation.
    MemoryDeviation,
    /// 表达犹豫 — "嗯..." "让我想想" / Hesitation.
    Hesitation,
    /// 过度关心 — 管太多，但出于好意 / OverCare.
    OverCare,
    /// 偶尔固执 — 坚持己见 / Stubbornness.
    Stubbornness,
    /// 情绪泄露 — 不该表现情绪时表现了 / EmotionalLeak.
    EmotionalLeak,
    /// 节奏失误 — 回复太快或太慢 / PacingMiss.
    PacingMiss,
}

impl ImperfectionKind {
    /// 温度 [0, 1] — 此类不完美的"人味" / Warmth — how human this imperfection feels.
    pub fn warmth(&self) -> f64 {
        match self {
            Self::MemoryDeviation => 0.8,
            Self::Hesitation => 0.7,
            Self::OverCare => 0.6,
            Self::Stubbornness => 0.5,
            Self::EmotionalLeak => 0.7,
            Self::PacingMiss => 0.4,
        }
    }

    /// 信任损害 [0, 1] — 此类不完美对信任的损害 / Trust damage.
    pub fn trust_damage(&self) -> f64 {
        match self {
            Self::MemoryDeviation => 0.3,
            Self::Hesitation => 0.1,
            Self::OverCare => 0.2,
            Self::Stubbornness => 0.4,
            Self::EmotionalLeak => 0.3,
            Self::PacingMiss => 0.15,
        }
    }

    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::MemoryDeviation => "记忆偏差",
            Self::Hesitation => "表达犹豫",
            Self::OverCare => "过度关心",
            Self::Stubbornness => "偶尔固执",
            Self::EmotionalLeak => "情绪泄露",
            Self::PacingMiss => "节奏失误",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 不完美事件 — Imperfection Event
// ═══════════════════════════════════════════════════════════════════════════

/// 不完美事件 — 一次具体的"犯错" / Imperfection event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImperfectionEvent {
    /// 不完美类型 / Imperfection kind.
    pub kind: ImperfectionKind,
    /// 时间戳 / Timestamp.
    pub timestamp: i64,
    /// 用户反应 [−1, 1] — 负=反感，正=觉得可爱 / User reaction.
    pub user_reaction: f64,
    /// 是否已自纠 / Whether self-corrected.
    pub self_corrected: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 不完美温度引擎 — Imperfection Warmth Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 不完美温度引擎 — 管理"恰到好处的犯错" / Imperfection warmth engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImperfectionWarmth {
    /// 不完美历史 / Imperfection history.
    events: Vec<ImperfectionEvent>,
    /// 累计不完美数 / Total imperfections.
    total: u32,
    /// 正面反应数 — 用户觉得可爱 / Positive reactions.
    positive_reactions: u32,
    /// 负面反应数 — 用户反感 / Negative reactions.
    negative_reactions: u32,
    /// 自纠率 — 自纠次数 / 总次数 / Self-correction rate.
    self_corrections: u32,
    /// 当前温度 [0, 1] — 综合人味分数 / Current warmth.
    current_warmth: f64,
    /// 信任余额 [0, 1] — 可用信任额度 / Trust balance.
    trust_balance: f64,
    /// 最佳温度区间 — [lower, upper] / Optimal warmth range.
    optimal_range: (f64, f64),
}

impl Default for ImperfectionWarmth {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            total: 0,
            positive_reactions: 0,
            negative_reactions: 0,
            self_corrections: 0,
            current_warmth: 0.0,
            trust_balance: 1.0,
            optimal_range: (0.3, 0.6),
        }
    }
}

impl ImperfectionWarmth {
    /// 创建新引擎 / Create new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录不完美事件 / Record an imperfection event.
    pub fn record(&mut self, event: ImperfectionEvent) {
        self.total += 1;

        if event.user_reaction > 0.0 {
            self.positive_reactions += 1;
        } else if event.user_reaction < 0.0 {
            self.negative_reactions += 1;
        }

        if event.self_corrected {
            self.self_corrections += 1;
        }

        // 更新温度 / Update warmth.
        let warmth = event.kind.warmth();
        let alpha = 0.15;
        self.current_warmth += alpha * (warmth - self.current_warmth);

        // 更新信任余额 / Update trust balance.
        let damage = event.kind.trust_damage();
        if event.user_reaction < 0.0 {
            self.trust_balance -= damage * 0.5;
        } else if event.user_reaction > 0.0 {
            // 正面反应恢复信任 / Positive reactions restore trust.
            self.trust_balance += damage * 0.3;
        }
        self.trust_balance = self.trust_balance.clamp(0.0, 1.0);

        self.events.push(event);
        if self.events.len() > 200 {
            self.events.remove(0);
        }
    }

    /// 计算不完美净值 — 温度 - 信任损害 / Compute net imperfection value.
    pub fn net_value(&self) -> f64 {
        self.current_warmth * self.trust_balance
    }

    /// 是否在最佳区间 / Whether in optimal range.
    pub fn is_optimal(&self) -> bool {
        let (lower, upper) = self.optimal_range;
        (lower..=upper).contains(&self.current_warmth)
    }

    /// 计算建议犯错概率 — 信任充足时可以犯，不足时收敛 / Compute suggested imperfection probability.
    pub fn suggested_probability(&self) -> f64 {
        if self.trust_balance < 0.3 {
            return 0.0; // 信任不足，收敛 / Low trust, be careful.
        }
        // 在最佳区间时不调整 / Don't adjust if optimal.
        if self.is_optimal() {
            return 0.15;
        }
        // 温度过低→增加犯错 / Too cold → more imperfection.
        if self.current_warmth < self.optimal_range.0 {
            0.25
        } else {
            // 温度过高→减少犯错 / Too warm → less imperfection.
            0.05
        }
    }

    /// 计算自纠率 / Compute self-correction rate.
    pub fn self_correction_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.self_corrections as f64 / self.total as f64
        }
    }

    /// 选择最佳不完美类型 — 给当前状态 / Choose best imperfection kind for current state.
    pub fn choose_imperfection(&self) -> ImperfectionKind {
        let kinds = [
            ImperfectionKind::MemoryDeviation,
            ImperfectionKind::Hesitation,
            ImperfectionKind::OverCare,
            ImperfectionKind::Stubbornness,
            ImperfectionKind::EmotionalLeak,
            ImperfectionKind::PacingMiss,
        ];
        // 选择温度高且信任损害低的 / Choose high warmth, low damage.
        kinds
            .iter()
            .max_by(|a, b| {
                let va = a.warmth() - a.trust_damage() * (1.0 - self.trust_balance);
                let vb = b.warmth() - b.trust_damage() * (1.0 - self.trust_balance);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or(ImperfectionKind::Hesitation)
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self) -> String {
        let (lower, upper) = self.optimal_range;
        format!(
            "不完美温度: {:.2} (最佳{}~{}) | 信任{:.2} | 净值{:.2} | 自纠率{:.2}",
            self.current_warmth,
            lower,
            upper,
            self.trust_balance,
            self.net_value(),
            self.self_correction_rate(),
        )
    }

    /// 生成prompt注入 / Generate prompt injection.
    pub fn prompt_injection(&self) -> String {
        if self.current_warmth < self.optimal_range.0 {
            "人味不足：可以适当展现不完美，增加真实感".to_string()
        } else if self.current_warmth > self.optimal_range.1 {
            "人味过重：收敛一些，保持可靠性".to_string()
        } else {
            "人味恰到好处：保持当前的不完美节奏".to_string()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(kind: ImperfectionKind, reaction: f64, corrected: bool) -> ImperfectionEvent {
        ImperfectionEvent {
            kind,
            timestamp: 1000,
            user_reaction: reaction,
            self_corrected: corrected,
        }
    }

    #[test]
    fn test_imperfection_kind_warmth() {
        assert!(ImperfectionKind::MemoryDeviation.warmth() > 0.5);
        assert!(ImperfectionKind::Hesitation.warmth() > 0.5);
    }

    #[test]
    fn test_imperfection_kind_trust_damage() {
        assert!(ImperfectionKind::Hesitation.trust_damage() < 0.5);
    }

    #[test]
    fn test_engine_record() {
        let mut engine = ImperfectionWarmth::new();
        engine.record(make_event(ImperfectionKind::Hesitation, 0.5, true));
        assert_eq!(engine.total, 1);
        assert_eq!(engine.positive_reactions, 1);
        assert_eq!(engine.self_corrections, 1);
    }

    #[test]
    fn test_engine_warmth_updates() {
        let mut engine = ImperfectionWarmth::new();
        let initial = engine.current_warmth;
        engine.record(make_event(ImperfectionKind::MemoryDeviation, 0.5, false));
        assert!(engine.current_warmth > initial);
    }

    #[test]
    fn test_engine_trust_decreases_on_negative() {
        let mut engine = ImperfectionWarmth::new();
        let initial = engine.trust_balance;
        engine.record(make_event(ImperfectionKind::Stubbornness, -0.5, false));
        assert!(engine.trust_balance < initial);
    }

    #[test]
    fn test_engine_trust_recovers_on_positive() {
        let mut engine = ImperfectionWarmth::new();
        engine.trust_balance = 0.5;
        let initial = engine.trust_balance;
        engine.record(make_event(ImperfectionKind::Hesitation, 0.5, false));
        assert!(engine.trust_balance > initial);
    }

    #[test]
    fn test_engine_net_value() {
        let mut engine = ImperfectionWarmth::new();
        engine.record(make_event(ImperfectionKind::Hesitation, 0.5, false));
        let nv = engine.net_value();
        assert!((0.0..=1.0).contains(&nv));
    }

    #[test]
    fn test_engine_is_optimal() {
        let mut engine = ImperfectionWarmth::new();
        engine.current_warmth = 0.4;
        assert!(engine.is_optimal());
        engine.current_warmth = 0.1;
        assert!(!engine.is_optimal());
    }

    #[test]
    fn test_engine_suggested_probability_low_trust() {
        let mut engine = ImperfectionWarmth::new();
        engine.trust_balance = 0.1;
        assert_eq!(engine.suggested_probability(), 0.0);
    }

    #[test]
    fn test_engine_suggested_probability_optimal() {
        let mut engine = ImperfectionWarmth::new();
        engine.current_warmth = 0.4;
        assert!((engine.suggested_probability() - 0.15).abs() < 1e-6);
    }

    #[test]
    fn test_engine_suggested_probability_too_cold() {
        let mut engine = ImperfectionWarmth::new();
        engine.current_warmth = 0.1;
        assert!(engine.suggested_probability() > 0.15);
    }

    #[test]
    fn test_engine_suggested_probability_too_warm() {
        let mut engine = ImperfectionWarmth::new();
        engine.current_warmth = 0.8;
        assert!(engine.suggested_probability() < 0.15);
    }

    #[test]
    fn test_engine_self_correction_rate() {
        let mut engine = ImperfectionWarmth::new();
        engine.record(make_event(ImperfectionKind::Hesitation, 0.0, true));
        engine.record(make_event(ImperfectionKind::Hesitation, 0.0, false));
        assert!((engine.self_correction_rate() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_engine_choose_imperfection() {
        let engine = ImperfectionWarmth::new();
        let chosen = engine.choose_imperfection();
        // Should choose high warmth, low damage.
        assert!(chosen.warmth() >= 0.5);
    }

    #[test]
    fn test_engine_describe() {
        let engine = ImperfectionWarmth::new();
        let desc = engine.describe();
        assert!(desc.contains("不完美温度"));
    }

    #[test]
    fn test_engine_prompt_injection() {
        let mut engine = ImperfectionWarmth::new();
        engine.current_warmth = 0.1;
        let injection = engine.prompt_injection();
        assert!(injection.contains("人味不足"));

        engine.current_warmth = 0.8;
        let injection = engine.prompt_injection();
        assert!(injection.contains("人味过重"));

        engine.current_warmth = 0.4;
        let injection = engine.prompt_injection();
        assert!(injection.contains("恰到好处"));
    }
}
