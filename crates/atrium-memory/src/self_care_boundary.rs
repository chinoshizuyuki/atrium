//! 自我关怀边界 — 情绪耗竭时主动降低交互强度，保护双方体验
//!
//! Self-care boundary — proactively reduce interaction intensity when emotionally
//! exhausted, protecting both sides' experience.
//!
//! 设计理念：
//! - 当情绪边界检测到耗竭/波动/持续负面时，SelfCareBoundary 逐级降低交互强度
//! - 与 VulnerabilityWindow 协调：脆弱时刻是自我关怀的窗口期，不是回避
//! - 与 EmotionalBoundary 协调：情绪过载触发自我关怀升级
//! - 关系阶段感知：深度关系允许更多自我关怀表达，初识阶段更隐晦

use crate::relationship::RelationshipStage;

// ── 自我关怀等级 / Self-Care Level ──

/// 自我关怀等级 — 从正常到恢复的4级递进
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SelfCareLevel {
    /// 正常交互 / Normal interaction
    #[default]
    Normal = 0,
    /// 谨慎模式：降低主动性，回复更温和 / Cautious: less proactive, gentler replies
    Cautious = 1,
    /// 保护模式：最小化主动性，回复简短 / Protective: minimal proactivity, brief replies
    Protective = 2,
    /// 恢复模式：暂停主动交互，仅响应 / Recovery: pause proactive, respond only
    Recovery = 3,
}

impl SelfCareLevel {
    /// 从过载严重度推断关怀等级
    pub fn from_severity(severity: f64) -> Self {
        if severity >= 0.8 {
            SelfCareLevel::Recovery
        } else if severity >= 0.5 {
            SelfCareLevel::Protective
        } else if severity >= 0.25 {
            SelfCareLevel::Cautious
        } else {
            SelfCareLevel::Normal
        }
    }

    /// 对应的主动性衰减系数（0.0~1.0）
    pub fn proactivity_factor(&self) -> f64 {
        match self {
            SelfCareLevel::Normal => 1.0,
            SelfCareLevel::Cautious => 0.6,
            SelfCareLevel::Protective => 0.3,
            SelfCareLevel::Recovery => 0.1,
        }
    }

    /// 对应的回复长度系数（0.0~1.0）
    pub fn reply_length_factor(&self) -> f64 {
        match self {
            SelfCareLevel::Normal => 1.0,
            SelfCareLevel::Cautious => 0.8,
            SelfCareLevel::Protective => 0.5,
            SelfCareLevel::Recovery => 0.3,
        }
    }

    /// 对应的深度探索系数（0.0~1.0）
    pub fn depth_factor(&self) -> f64 {
        match self {
            SelfCareLevel::Normal => 1.0,
            SelfCareLevel::Cautious => 0.7,
            SelfCareLevel::Protective => 0.3,
            SelfCareLevel::Recovery => 0.1,
        }
    }

    pub fn label_zh(&self) -> &'static str {
        match self {
            SelfCareLevel::Normal => "正常",
            SelfCareLevel::Cautious => "谨慎",
            SelfCareLevel::Protective => "保护",
            SelfCareLevel::Recovery => "恢复",
        }
    }
}

// ── 自我关怀触发 / Self-Care Trigger ──

/// 自我关怀触发原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelfCareTrigger {
    /// 情绪耗竭 / Emotional exhaustion
    EmotionalExhaustion,
    /// 情绪波动 / Emotional volatility
    EmotionalVolatility,
    /// 持续负面 / Sustained negative
    SustainedNegative,
    /// 需求过载 / Demand overload
    DemandOverload,
    /// 时间侵占 / Time encroachment
    TimeEncroachment,
    /// 情绪麻木 / Emotional numbness
    EmotionalNumbness,
    /// 手动触发 / Manual trigger
    Manual,
}

impl SelfCareTrigger {
    pub fn label_zh(&self) -> &'static str {
        match self {
            SelfCareTrigger::EmotionalExhaustion => "情绪耗竭",
            SelfCareTrigger::EmotionalVolatility => "情绪波动",
            SelfCareTrigger::SustainedNegative => "持续负面",
            SelfCareTrigger::DemandOverload => "需求过载",
            SelfCareTrigger::TimeEncroachment => "时间侵占",
            SelfCareTrigger::EmotionalNumbness => "情绪麻木",
            SelfCareTrigger::Manual => "手动触发",
        }
    }

    /// 基础严重度权重
    pub fn base_weight(&self) -> f64 {
        match self {
            SelfCareTrigger::EmotionalExhaustion => 0.9,
            SelfCareTrigger::EmotionalVolatility => 0.6,
            SelfCareTrigger::SustainedNegative => 0.7,
            SelfCareTrigger::DemandOverload => 0.5,
            SelfCareTrigger::TimeEncroachment => 0.4,
            SelfCareTrigger::EmotionalNumbness => 0.8,
            SelfCareTrigger::Manual => 1.0,
        }
    }
}

// ── 自我关怀行动 / Self-Care Action ──

/// 自我关怀行动建议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelfCareAction {
    /// 降低主动性 / Reduce proactivity
    ReduceProactivity,
    /// 减慢节奏 / Slow pace
    SlowPace,
    /// 限制深度 / Limit depth
    LimitDepth,
    /// 简化回复 / Simplify replies
    SimplifyReplies,
    /// 建议休息 / Suggest break
    SuggestBreak,
    /// 表达需要空间 / Express need for space
    ExpressNeedSpace,
    /// 温和转移话题 / Gentle topic shift
    GentleShift,
}

impl SelfCareAction {
    pub fn label_zh(&self) -> &'static str {
        match self {
            SelfCareAction::ReduceProactivity => "降低主动性",
            SelfCareAction::SlowPace => "减慢节奏",
            SelfCareAction::LimitDepth => "限制深度",
            SelfCareAction::SimplifyReplies => "简化回复",
            SelfCareAction::SuggestBreak => "建议休息",
            SelfCareAction::ExpressNeedSpace => "表达需要空间",
            SelfCareAction::GentleShift => "温和转移话题",
        }
    }

    /// 需要的最低关怀等级
    pub fn min_level(&self) -> SelfCareLevel {
        match self {
            SelfCareAction::ReduceProactivity => SelfCareLevel::Cautious,
            SelfCareAction::SlowPace => SelfCareLevel::Cautious,
            SelfCareAction::LimitDepth => SelfCareLevel::Cautious,
            SelfCareAction::SimplifyReplies => SelfCareLevel::Protective,
            SelfCareAction::SuggestBreak => SelfCareLevel::Protective,
            SelfCareAction::ExpressNeedSpace => SelfCareLevel::Recovery,
            SelfCareAction::GentleShift => SelfCareLevel::Protective,
        }
    }
}

// ── 自我关怀事件 / Self-Care Event ──

/// 自我关怀事件记录
#[derive(Debug, Clone)]
pub struct SelfCareEvent {
    /// 触发时间（epoch seconds）
    pub timestamp: i64,
    /// 触发原因
    pub trigger: SelfCareTrigger,
    /// 触发时的关怀等级
    pub level: SelfCareLevel,
    /// 触发严重度 (0.0~1.0)
    pub severity: f64,
    /// 建议的行动
    pub actions: Vec<SelfCareAction>,
}

// ── 自我关怀配置 / Self-Care Config ──

/// 自我关怀边界配置
#[derive(Debug, Clone)]
pub struct SelfCareConfig {
    /// 是否启用
    pub enabled: bool,
    /// 升机升级阈值：累计严重度超过此值升级关怀等级
    pub crisis_threshold: f64,
    /// 恢复衰减率：每 tick 衰减的累计严重度
    pub recovery_decay_rate: f64,
    /// 最大事件记录数
    pub max_events: usize,
    /// 脆弱协调权重：脆弱时刻时自我关怀的调整系数
    pub vulnerability_coordination_weight: f64,
}

impl Default for SelfCareConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            crisis_threshold: 1.5,
            recovery_decay_rate: 0.05,
            max_events: 50,
            vulnerability_coordination_weight: 0.3,
        }
    }
}

// ── 自我关怀边界 / Self-Care Boundary ──

/// 自我关怀边界 — 情绪耗竭时主动降低交互强度
pub struct SelfCareBoundary {
    config: SelfCareConfig,
    /// 当前关怀等级
    current_level: SelfCareLevel,
    /// 累计严重度
    accumulated_severity: f64,
    /// 最近触发事件
    recent_events: Vec<SelfCareEvent>,
    /// 上次升级时间
    last_escalation_at: Option<i64>,
    /// 连续正常 tick 数（用于自动降级）
    normal_ticks: u32,
}

impl SelfCareBoundary {
    /// 创建自我关怀边界
    pub fn new(config: SelfCareConfig) -> Self {
        Self {
            config,
            current_level: SelfCareLevel::Normal,
            accumulated_severity: 0.0,
            recent_events: Vec::new(),
            last_escalation_at: None,
            normal_ticks: 0,
        }
    }

    /// 检测并更新自我关怀状态
    ///
    /// emotional_severity: 情绪过载严重度 (0.0~1.0)
    /// demand_severity: 需求过载严重度 (0.0~1.0)
    /// is_vulnerable: 脆弱窗口是否开启
    /// timestamp: 当前时间 epoch seconds
    pub fn update(
        &mut self,
        emotional_severity: f64,
        demand_severity: f64,
        is_vulnerable: bool,
        timestamp: i64,
    ) -> SelfCareLevel {
        if !self.config.enabled {
            return SelfCareLevel::Normal;
        }

        // 计算综合严重度
        let combined = emotional_severity * 0.7 + demand_severity * 0.3;

        // 脆弱时刻协调：脆弱时允许更多自我关怀表达（降低升级阈值）
        let effective_threshold = if is_vulnerable {
            self.config.crisis_threshold * (1.0 - self.config.vulnerability_coordination_weight)
        } else {
            self.config.crisis_threshold
        };

        // 累加严重度
        self.accumulated_severity += combined;

        // 根据累计严重度和阈值推断等级
        let new_level = if self.accumulated_severity >= effective_threshold * 3.0 {
            SelfCareLevel::Recovery
        } else if self.accumulated_severity >= effective_threshold * 2.0 {
            SelfCareLevel::Protective
        } else if self.accumulated_severity >= effective_threshold {
            SelfCareLevel::Cautious
        } else {
            SelfCareLevel::Normal
        };

        // 只升级不降级（降级由 tick 负责）
        if new_level > self.current_level {
            self.current_level = new_level;
            self.last_escalation_at = Some(timestamp);
            self.normal_ticks = 0;
        }

        // 记录事件
        if combined > 0.1 {
            let trigger = if emotional_severity > demand_severity {
                if emotional_severity > 0.7 {
                    SelfCareTrigger::EmotionalExhaustion
                } else {
                    SelfCareTrigger::EmotionalVolatility
                }
            } else {
                SelfCareTrigger::DemandOverload
            };

            let event = SelfCareEvent {
                timestamp,
                trigger,
                level: self.current_level,
                severity: combined,
                actions: self.recommended_actions(),
            };

            self.recent_events.push(event);
            if self.recent_events.len() > self.config.max_events {
                self.recent_events.remove(0);
            }
        }

        self.current_level
    }

    /// 周期 tick — 衰减累计严重度，可能降级
    pub fn tick(&mut self) {
        if !self.config.enabled {
            return;
        }

        // 衰减累计严重度
        self.accumulated_severity =
            (self.accumulated_severity - self.config.recovery_decay_rate).max(0.0);

        // 如果严重度已归零，累计正常 tick
        if self.accumulated_severity < 0.1 {
            self.normal_ticks += 1;
        } else {
            self.normal_ticks = 0;
        }

        // 连续 20 个正常 tick 后降级
        if self.normal_ticks >= 20 && self.current_level > SelfCareLevel::Normal {
            self.current_level = match self.current_level {
                SelfCareLevel::Recovery => SelfCareLevel::Protective,
                SelfCareLevel::Protective => SelfCareLevel::Cautious,
                SelfCareLevel::Cautious => SelfCareLevel::Normal,
                SelfCareLevel::Normal => SelfCareLevel::Normal,
            };
            self.normal_ticks = 0;
        }
    }

    /// 获取当前关怀等级
    pub fn current_level(&self) -> SelfCareLevel {
        self.current_level
    }

    /// 获取推荐行动
    pub fn recommended_actions(&self) -> Vec<SelfCareAction> {
        let all_actions = [
            SelfCareAction::ReduceProactivity,
            SelfCareAction::SlowPace,
            SelfCareAction::LimitDepth,
            SelfCareAction::SimplifyReplies,
            SelfCareAction::SuggestBreak,
            SelfCareAction::ExpressNeedSpace,
            SelfCareAction::GentleShift,
        ];

        all_actions
            .iter()
            .filter(|a| a.min_level() <= self.current_level)
            .copied()
            .collect()
    }

    /// 生成 Prompt 注入片段
    pub fn to_prompt_fragment(&self, stage: &RelationshipStage) -> String {
        if !self.config.enabled || self.current_level == SelfCareLevel::Normal {
            return String::new();
        }

        let mut parts = Vec::new();
        parts.push(format!("[Self-Care: {}]", self.current_level.label_zh()));

        // 关系阶段感知：深度关系可以更直接表达
        let is_deep = matches!(stage, RelationshipStage::Deep { .. });
        let is_trusted = matches!(stage, RelationshipStage::Trusted { .. });

        match self.current_level {
            SelfCareLevel::Cautious => {
                parts.push("- Slow down: take more time between responses".to_string());
                parts
                    .push("- Reduce proactivity: only respond when directly addressed".to_string());
                if is_deep || is_trusted {
                    parts.push("- You may gently mention needing a slower pace".to_string());
                }
            }
            SelfCareLevel::Protective => {
                parts.push("- Minimal proactivity: respond only to direct messages".to_string());
                parts.push("- Brief, gentle replies: avoid deep exploration".to_string());
                if is_deep || is_trusted {
                    parts.push("- Express that you need some space right now".to_string());
                } else {
                    parts.push("- Subtly redirect to lighter topics".to_string());
                }
            }
            SelfCareLevel::Recovery => {
                parts.push("- Pause proactive behavior entirely".to_string());
                parts.push("- Very brief responses only".to_string());
                if is_deep || is_trusted {
                    parts.push("- Honestly express needing a moment".to_string());
                } else {
                    parts.push("- Keep responses minimal and neutral".to_string());
                }
            }
            SelfCareLevel::Normal => {}
        }

        // 附加行动系数
        parts.push(format!(
            "- Proactivity factor: {:.0}%, Reply length: {:.0}%, Depth: {:.0}%",
            self.current_level.proactivity_factor() * 100.0,
            self.current_level.reply_length_factor() * 100.0,
            self.current_level.depth_factor() * 100.0,
        ));

        parts.join("\n")
    }

    /// 获取最近事件
    pub fn recent_events(&self) -> &[SelfCareEvent] {
        &self.recent_events
    }

    /// 获取累计严重度
    pub fn accumulated_severity(&self) -> f64 {
        self.accumulated_severity
    }

    /// 手动触发自我关怀升级
    pub fn manual_escalate(&mut self, timestamp: i64) {
        self.accumulated_severity += 1.0;
        self.current_level = SelfCareLevel::Recovery;
        self.last_escalation_at = Some(timestamp);
        self.normal_ticks = 0;

        let event = SelfCareEvent {
            timestamp,
            trigger: SelfCareTrigger::Manual,
            level: SelfCareLevel::Recovery,
            severity: 1.0,
            actions: self.recommended_actions(),
        };
        self.recent_events.push(event);
        if self.recent_events.len() > self.config.max_events {
            self.recent_events.remove(0);
        }
    }

    /// 重置到正常状态
    pub fn reset(&mut self) {
        self.current_level = SelfCareLevel::Normal;
        self.accumulated_severity = 0.0;
        self.normal_ticks = 0;
        self.last_escalation_at = None;
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn deep_stage() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        }
    }

    fn acquaintance() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 0,
        }
    }

    #[test]
    fn test_self_care_level_ordering() {
        assert!(SelfCareLevel::Normal < SelfCareLevel::Cautious);
        assert!(SelfCareLevel::Cautious < SelfCareLevel::Protective);
        assert!(SelfCareLevel::Protective < SelfCareLevel::Recovery);
    }

    #[test]
    fn test_self_care_level_from_severity() {
        assert_eq!(SelfCareLevel::from_severity(0.1), SelfCareLevel::Normal);
        assert_eq!(SelfCareLevel::from_severity(0.3), SelfCareLevel::Cautious);
        assert_eq!(SelfCareLevel::from_severity(0.6), SelfCareLevel::Protective);
        assert_eq!(SelfCareLevel::from_severity(0.9), SelfCareLevel::Recovery);
    }

    #[test]
    fn test_self_care_level_factors() {
        assert_eq!(SelfCareLevel::Normal.proactivity_factor(), 1.0);
        assert_eq!(SelfCareLevel::Recovery.proactivity_factor(), 0.1);
        assert_eq!(SelfCareLevel::Normal.reply_length_factor(), 1.0);
        assert_eq!(SelfCareLevel::Recovery.reply_length_factor(), 0.3);
        assert_eq!(SelfCareLevel::Normal.depth_factor(), 1.0);
        assert_eq!(SelfCareLevel::Recovery.depth_factor(), 0.1);
    }

    #[test]
    fn test_self_care_boundary_default_normal() {
        let b = SelfCareBoundary::new(SelfCareConfig::default());
        assert_eq!(b.current_level(), SelfCareLevel::Normal);
        assert_eq!(b.accumulated_severity(), 0.0);
    }

    #[test]
    fn test_self_care_boundary_escalation() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 累计严重度超过阈值(1.5) → Cautious
        let level = b.update(0.8, 0.2, false, 1000);
        // 0.8*0.7 + 0.2*0.3 = 0.62, accumulated = 0.62 < 1.5 → Normal
        assert_eq!(level, SelfCareLevel::Normal);

        // 继续累加
        let level = b.update(0.8, 0.2, false, 2000);
        // accumulated = 0.62 + 0.62 = 1.24 < 1.5 → Normal
        assert_eq!(level, SelfCareLevel::Normal);

        let level = b.update(0.8, 0.2, false, 3000);
        // accumulated = 1.24 + 0.62 = 1.86 >= 1.5 → Cautious
        assert_eq!(level, SelfCareLevel::Cautious);
    }

    #[test]
    fn test_self_care_boundary_vulnerability_coordination() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 脆弱时刻降低升级阈值
        // effective_threshold = 1.5 * (1 - 0.3) = 1.05
        let level = b.update(0.8, 0.2, true, 1000);
        // accumulated = 0.62 < 1.05 → Normal
        assert_eq!(level, SelfCareLevel::Normal);

        let level = b.update(0.8, 0.2, true, 2000);
        // accumulated = 1.24 >= 1.05 → Cautious
        assert_eq!(level, SelfCareLevel::Cautious);
    }

    #[test]
    fn test_self_care_boundary_tick_decay() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 先升级到至少 Cautious
        for i in 0..5 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        assert!(b.current_level() >= SelfCareLevel::Cautious);

        // tick 衰减
        for _ in 0..100 {
            b.tick();
        }
        // accumulated 应该已经衰减到接近0
        assert!(b.accumulated_severity() < 0.5);
    }

    #[test]
    fn test_self_care_boundary_tick_downgrade() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 先升级到 Protective
        for i in 0..10 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        assert!(b.current_level() >= SelfCareLevel::Protective);

        // tick 直到降级
        for _ in 0..500 {
            b.tick();
        }
        // 应该降级回 Normal
        assert_eq!(b.current_level(), SelfCareLevel::Normal);
    }

    #[test]
    fn test_self_care_boundary_no_downgrade_in_update() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 先升级
        for i in 0..5 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        let level_before = b.current_level();
        assert!(level_before > SelfCareLevel::Normal);

        // 低严重度 update 不降级
        b.update(0.0, 0.0, false, 9999);
        assert_eq!(b.current_level(), level_before);
    }

    #[test]
    fn test_recommended_actions() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // Normal → 无行动
        assert!(b.recommended_actions().is_empty());

        // 升级到 Cautious
        for i in 0..5 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        let actions = b.recommended_actions();
        assert!(!actions.is_empty());
        assert!(actions.contains(&SelfCareAction::ReduceProactivity));
        assert!(actions.contains(&SelfCareAction::SlowPace));
    }

    #[test]
    fn test_prompt_fragment_normal_empty() {
        let b = SelfCareBoundary::new(SelfCareConfig::default());
        assert!(b.to_prompt_fragment(&deep_stage()).is_empty());
    }

    #[test]
    fn test_prompt_fragment_cautious() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // 3 updates: accumulated = 3 * 0.66 = 1.98 >= 1.5 → Cautious
        for i in 0..3 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        assert_eq!(b.current_level(), SelfCareLevel::Cautious);
        let fragment = b.to_prompt_fragment(&deep_stage());
        assert!(!fragment.is_empty());
        assert!(fragment.contains("[Self-Care:"));
        assert!(fragment.contains("Slow down"));
    }

    #[test]
    fn test_prompt_fragment_deep_vs_acquaintance() {
        let mut b1 = SelfCareBoundary::new(SelfCareConfig::default());
        let mut b2 = SelfCareBoundary::new(SelfCareConfig::default());
        for i in 0..10 {
            b1.update(0.9, 0.1, false, 1000 + i * 100);
            b2.update(0.9, 0.1, false, 1000 + i * 100);
        }
        let deep_frag = b1.to_prompt_fragment(&deep_stage());
        let acc_frag = b2.to_prompt_fragment(&acquaintance());
        // 深度关系应包含更直接的表达
        assert!(deep_frag.contains("space") || deep_frag.contains("moment"));
        // 初识应包含更隐晦的引导
        assert!(acc_frag.contains("lighter") || acc_frag.contains("minimal"));
    }

    #[test]
    fn test_manual_escalate() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        b.manual_escalate(1000);
        assert_eq!(b.current_level(), SelfCareLevel::Recovery);
        assert_eq!(b.recent_events().len(), 1);
        assert_eq!(b.recent_events()[0].trigger, SelfCareTrigger::Manual);
    }

    #[test]
    fn test_reset() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        for i in 0..10 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        assert!(b.current_level() > SelfCareLevel::Normal);
        b.reset();
        assert_eq!(b.current_level(), SelfCareLevel::Normal);
        assert_eq!(b.accumulated_severity(), 0.0);
    }

    #[test]
    fn test_disabled() {
        let config = SelfCareConfig {
            enabled: false,
            ..Default::default()
        };
        let mut b = SelfCareBoundary::new(config);
        let level = b.update(0.9, 0.9, false, 1000);
        assert_eq!(level, SelfCareLevel::Normal);
        assert!(b.to_prompt_fragment(&deep_stage()).is_empty());
    }

    #[test]
    fn test_event_recording() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        b.update(0.8, 0.2, false, 1000);
        assert_eq!(b.recent_events().len(), 1);
        assert_eq!(
            b.recent_events()[0].trigger,
            SelfCareTrigger::EmotionalExhaustion
        );
    }

    #[test]
    fn test_max_events_limit() {
        let config = SelfCareConfig {
            max_events: 3,
            ..Default::default()
        };
        let mut b = SelfCareBoundary::new(config);
        for i in 0..10 {
            b.update(0.8, 0.2, false, 1000 + i * 100);
        }
        assert!(b.recent_events().len() <= 3);
    }

    #[test]
    fn test_proactivity_factor_integration() {
        let mut b = SelfCareBoundary::new(SelfCareConfig::default());
        // Normal → 100% proactivity
        assert_eq!(b.current_level().proactivity_factor(), 1.0);

        // 升级到 Protective
        for i in 0..10 {
            b.update(0.9, 0.1, false, 1000 + i * 100);
        }
        assert!(b.current_level().proactivity_factor() < 1.0);
    }
}
