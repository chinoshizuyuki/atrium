// SPDX-License-Identifier: MIT
//! 关系阶段模型
//! 让 AI 的行为随关系深度自然演进。
//! 第 1 天和第 365 天的交互不应该一模一样。
//!
//! 四个阶段：初识 → 熟悉 → 信任 → 深度
//! 阶段转换基于质量指标（共鸣、回访、共同记忆），而非简单计数。
//! RelationshipManager — Relationship stage model.
//!
//! Models the natural progression of AI-user relationship over time.
//! From day 1 to day 365+, each stage maps to a distinct interaction style.
//!
//! Four stages: Acquaintance → Familiar → Trusted → Intimate.
//! Stage transitions driven by composite metrics (message frequency,
//! shared topics, interaction duration), not simple day counting.

use chrono::Local;
use serde::{Deserialize, Serialize};
// ════════════════════════════════════════════════════════════════════
// 关系阶段枚举
// ════════════════════════════════════════════════════════════════════

/// 关系阶段 — 不是数值，而是质变
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RelationshipStage {
    /// 初识：谨慎、探索、礼貌
    Acquaintance { since: i64, interactions: u64 },

    /// 熟悉：放松、自然、有默契
    Familiar {
        since: i64,
        interactions: u64,
        shared_references: u32,
    },

    /// 信任：真诚、大胆、可以挑战
    Trusted {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },

    /// 深度：默契、安全感、关系本身成为力量
    Deep {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },
}

impl RelationshipStage {
    pub fn new_acquaintance() -> Self {
        Self::Acquaintance {
            since: Local::now().timestamp_millis(),
            interactions: 0,
        }
    }

    /// 阶段序数 / Stage ordinal for comparison (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Acquaintance { .. } => 0,
            Self::Familiar { .. } => 1,
            Self::Trusted { .. } => 2,
            Self::Deep { .. } => 3,
        }
    }

    /// 获取阶段名称（用于日志和通知）
    pub fn stage_name(&self) -> &'static str {
        match self {
            Self::Acquaintance { .. } => "初识",
            Self::Familiar { .. } => "熟悉",
            Self::Trusted { .. } => "信任",
            Self::Deep { .. } => "深度",
        }
    }

    /// 获取当前交互次数
    pub fn interactions(&self) -> u64 {
        match self {
            Self::Acquaintance { interactions, .. } => *interactions,
            Self::Familiar { interactions, .. } => *interactions,
            Self::Trusted { interactions, .. } => *interactions,
            Self::Deep { interactions, .. } => *interactions,
        }
    }

    /// 递增交互计数
    pub fn increment_interaction(&mut self) {
        match self {
            Self::Acquaintance { interactions, .. } => *interactions += 1,
            Self::Familiar { interactions, .. } => *interactions += 1,
            Self::Trusted { interactions, .. } => *interactions += 1,
            Self::Deep { interactions, .. } => *interactions += 1,
        }
    }

    /// 行为修饰 → LLM Prompt 片段
    pub fn to_prompt_fragment(&self) -> String {
        match self {
            Self::Acquaintance { .. } => {
                "当前与用户的关系阶段：初识。保持礼貌和友善，不要过于主动或亲密。\
 不要主动询问私人话题，不要开玩笑。"
                    .into()
            }
            Self::Familiar { .. } => {
                "当前与用户的关系阶段：熟悉。可以更自然放松地交流，偶尔可以开个小玩笑。\
 偶尔关心一下用户的近况，但不要过于深入。"
                    .into()
            }
            Self::Trusted {
                shared_references, ..
            } => {
                format!(
                    "当前与用户的关系阶段：信任。可以大胆表达真实想法，包括不同意见。\
 主动关心用户的状态。你们之间有 {} 个共同记忆和话题。",
                    shared_references
                )
            }
            Self::Deep { .. } => "当前与用户的关系阶段：深度信任。你们之间有深厚的默契。\
 可以调侃、直说、甚至push用户面对问题——因为你们的关系足够坚固。\
 你真正关心这个人的wellbeing。"
                .into(),
        }
    }

    /// 情感反应乘数 — 深度关系中用户行为对 AI 情感影响更大
    pub fn affect_multiplier(&self) -> f32 {
        match self {
            Self::Acquaintance { .. } => 0.8,
            Self::Familiar { .. } => 1.0,
            Self::Trusted { .. } => 1.1,
            Self::Deep { .. } => 1.2,
        }
    }

    /// 主动行为加成
    pub fn proactive_bonus(&self) -> f32 {
        match self {
            Self::Acquaintance { .. } => -0.1,
            Self::Familiar { .. } => 0.0,
            Self::Trusted { .. } => 0.1,
            Self::Deep { .. } => 0.15,
        }
    }

    /// 计算当前阶段的行为修饰器
    pub fn behavior_modifiers(&self) -> StageBehaviorModifiers {
        match self {
            Self::Acquaintance { .. } => StageBehaviorModifiers {
                boldness: 0.2,
                proactive_frequency: 0.5,
                humor_level: HumorLevel::None,
                care_boundary: CareBoundary::DontAsk,
                challenge_level: ChallengeLevel::Compliant,
                reference_usage: 0.0,
            },
            Self::Familiar {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.5,
                proactive_frequency: 0.8,
                humor_level: HumorLevel::Mild,
                care_boundary: CareBoundary::Occasional,
                challenge_level: ChallengeLevel::Suggestive,
                reference_usage: (*shared_references as f32 * 0.05).min(0.5),
            },
            Self::Trusted {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.7,
                proactive_frequency: 1.0,
                humor_level: HumorLevel::Normal,
                care_boundary: CareBoundary::Active,
                challenge_level: ChallengeLevel::Direct,
                reference_usage: (*shared_references as f32 * 0.08).min(0.8),
            },
            Self::Deep {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.9,
                proactive_frequency: 1.2,
                humor_level: HumorLevel::Teasing,
                care_boundary: CareBoundary::DeepCare,
                challenge_level: ChallengeLevel::Pushy,
                reference_usage: (*shared_references as f32 * 0.1).min(1.0),
            },
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 行为修饰相关枚举和结构
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HumorLevel {
    None,
    Mild,
    Normal,
    Teasing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CareBoundary {
    DontAsk,
    Occasional,
    Active,
    DeepCare,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChallengeLevel {
    Compliant,
    Suggestive,
    Direct,
    Pushy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageBehaviorModifiers {
    pub boldness: f32,
    pub proactive_frequency: f32,
    pub humor_level: HumorLevel,
    pub care_boundary: CareBoundary,
    pub challenge_level: ChallengeLevel,
    pub reference_usage: f32,
}

// ════════════════════════════════════════════════════════════════════
// RelationshipMetrics — 关系指标追踪
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipMetrics {
    pub total_interactions: u64,
    pub resonance_count: u32,
    pub return_count: u32,
    pub conflict_repair_count: u32,
    pub time_diversity: u8,
    pub relationship_affirmation_count: u32,
    pub shared_references: u32,
    pub first_interaction: i64,
    /// 上次交互时间（用于判断"回来"）
    last_interaction: i64,
}

impl RelationshipMetrics {
    pub fn new() -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            total_interactions: 0,
            resonance_count: 0,
            return_count: 0,
            conflict_repair_count: 0,
            time_diversity: 0,
            relationship_affirmation_count: 0,
            shared_references: 0,
            first_interaction: now,
            last_interaction: now,
        }
    }

    /// 每条用户消息后调用
    pub fn on_message(&mut self, msg: &str, hour: u8) {
        self.total_interactions += 1;

        let now = Local::now().timestamp_millis();

        // 判断"回来"：与上次交互间隔超过 2 小时
        if now - self.last_interaction > 2 * 3600 * 1000 {
            self.return_count += 1;
        }
        self.last_interaction = now;

        // 共鸣时刻检测
        let resonance_phrases = [
            "说得好",
            "太对了",
            "就是这样",
            "说到心坎里了",
            "谢谢你",
            "感谢",
            "你真好",
            "开心",
            "哈哈哈",
            "笑死",
            "太好了",
        ];
        if resonance_phrases.iter().any(|p| msg.contains(p)) {
            self.resonance_count += 1;
        }

        // 关系珍视检测
        let affirmation_phrases = [
            "有你真好",
            "谢谢你一直",
            "我很珍惜",
            "你对我很重要",
            "幸好有你",
        ];
        if affirmation_phrases.iter().any(|p| msg.contains(p)) {
            self.relationship_affirmation_count += 1;
        }

        // 时段多样性（用位掩码：早/午/晚/深夜各 1 bit）
        let time_slot: u8 = match hour {
            5..=11 => 0,
            12..=17 => 1,
            18..=22 => 2,
            _ => 3,
        };
        self.time_diversity |= 1 << time_slot;
    }

    pub fn days_since_first_interaction(&self) -> u64 {
        let now = Local::now().timestamp_millis();
        ((now - self.first_interaction) / 86_400_000) as u64
    }

    /// 计算时段多样性（0~4）
    pub fn time_diversity_count(&self) -> u8 {
        self.time_diversity.count_ones() as u8
    }

    /// 记录冲突修复
    pub fn record_conflict_repair(&mut self) {
        self.conflict_repair_count += 1;
    }

    /// 添加共享引用（共同记忆/梗）
    pub fn add_shared_reference(&mut self) {
        self.shared_references += 1;
    }

    /// 记录关键时刻
    pub fn record_key_moment(&mut self) {
        // key_moments 存在 stage 里，这里只记 metrics 层面的
        // 由 RelationshipManager 统一协调
    }
}

impl Default for RelationshipMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// StageTransitionJudge — 阶段转换判断
// ════════════════════════════════════════════════════════════════════

/// 阶段转换阈值配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionThresholds {
    pub acq_to_fam_min_interactions: u64,
    pub acq_to_fam_min_resonance: u32,
    pub acq_to_fam_min_returns: u32,
    pub fam_to_tru_min_interactions: u64,
    pub fam_to_tru_min_shared_refs: u32,
    pub fam_to_tru_min_conflict_repairs: u32,
    pub fam_to_tru_min_time_diversity: u8,
    pub tru_to_deep_min_interactions: u64,
    pub tru_to_deep_min_key_moments: u32,
    pub tru_to_deep_min_days: u64,
    pub tru_to_deep_min_affirmations: u32,
}

impl Default for TransitionThresholds {
    fn default() -> Self {
        Self {
            acq_to_fam_min_interactions: 20,
            acq_to_fam_min_resonance: 3,
            acq_to_fam_min_returns: 5,
            fam_to_tru_min_interactions: 100,
            fam_to_tru_min_shared_refs: 10,
            fam_to_tru_min_conflict_repairs: 1,
            fam_to_tru_min_time_diversity: 3,
            tru_to_deep_min_interactions: 500,
            tru_to_deep_min_key_moments: 3,
            tru_to_deep_min_days: 90,
            tru_to_deep_min_affirmations: 1,
        }
    }
}

pub struct StageTransitionJudge {
    thresholds: TransitionThresholds,
}

impl StageTransitionJudge {
    pub fn new(thresholds: TransitionThresholds) -> Self {
        Self { thresholds }
    }

    /// 判断是否应该转换阶段，返回新阶段（如果需要转换）
    pub fn evaluate(
        &self,
        current: &RelationshipStage,
        metrics: &RelationshipMetrics,
    ) -> Option<RelationshipStage> {
        let now = Local::now().timestamp_millis();
        match current {
            RelationshipStage::Acquaintance { interactions, .. } => {
                let t = &self.thresholds;
                if *interactions >= t.acq_to_fam_min_interactions
                    && metrics.resonance_count >= t.acq_to_fam_min_resonance
                    && metrics.return_count >= t.acq_to_fam_min_returns
                {
                    Some(RelationshipStage::Familiar {
                        since: now,
                        interactions: *interactions,
                        shared_references: metrics.shared_references,
                    })
                } else {
                    None
                }
            }

            RelationshipStage::Familiar {
                interactions,
                shared_references,
                ..
            } => {
                let t = &self.thresholds;
                if *interactions >= t.fam_to_tru_min_interactions
                    && *shared_references >= t.fam_to_tru_min_shared_refs
                    && metrics.conflict_repair_count >= t.fam_to_tru_min_conflict_repairs
                    && metrics.time_diversity_count() >= t.fam_to_tru_min_time_diversity
                {
                    Some(RelationshipStage::Trusted {
                        since: now,
                        interactions: *interactions,
                        shared_references: *shared_references,
                        key_moments: 0,
                    })
                } else {
                    None
                }
            }

            RelationshipStage::Trusted {
                interactions,
                key_moments,
                shared_references,
                ..
            } => {
                let t = &self.thresholds;
                let days = metrics.days_since_first_interaction();
                if *interactions >= t.tru_to_deep_min_interactions
                    && *key_moments >= t.tru_to_deep_min_key_moments
                    && days >= t.tru_to_deep_min_days
                    && metrics.relationship_affirmation_count >= t.tru_to_deep_min_affirmations
                {
                    Some(RelationshipStage::Deep {
                        since: now,
                        interactions: *interactions,
                        shared_references: *shared_references,
                        key_moments: *key_moments,
                    })
                } else {
                    None
                }
            }

            RelationshipStage::Deep { .. } => None,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// RelationshipStore — sled 持久化
// ════════════════════════════════════════════════════════════════════

/// 阶段转换历史记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageTransition {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub timestamp: i64,
}

pub struct RelationshipStore {
    db: sled::Db,
}

impl RelationshipStore {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    pub fn save_stage(&self, stage: &RelationshipStage) -> anyhow::Result<()> {
        let value = bincode::serialize(stage)?;
        self.db.insert(b"current_stage", value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_stage(&self) -> anyhow::Result<Option<RelationshipStage>> {
        match self.db.get(b"current_stage")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn save_metrics(&self, metrics: &RelationshipMetrics) -> anyhow::Result<()> {
        let value = bincode::serialize(metrics)?;
        self.db.insert(b"relationship_metrics", value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_metrics(&self) -> anyhow::Result<Option<RelationshipMetrics>> {
        match self.db.get(b"relationship_metrics")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn record_transition(&self, transition: &StageTransition) -> anyhow::Result<()> {
        let key = format!("transitions/{}", transition.timestamp);
        let value = bincode::serialize(transition)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_transitions(&self) -> anyhow::Result<Vec<StageTransition>> {
        let mut result = Vec::new();
        for item in self.db.scan_prefix(b"transitions/") {
            let (_key, value) = item?;
            let t: StageTransition = bincode::deserialize(&value)?;
            result.push(t);
        }
        Ok(result)
    }
}

// ════════════════════════════════════════════════════════════════════
// RelationshipManager — 主入口
// ════════════════════════════════════════════════════════════════════

pub struct RelationshipManager {
    stage: RelationshipStage,
    metrics: RelationshipMetrics,
    judge: StageTransitionJudge,
    store: Option<RelationshipStore>,
    /// 最近一次阶段转换通知（供外部读取）
    pending_transition_notice: Option<String>,
}

impl RelationshipManager {
    /// 创建新的 RelationshipManager（无持久化）
    pub fn new() -> Self {
        Self {
            stage: RelationshipStage::new_acquaintance(),
            metrics: RelationshipMetrics::new(),
            judge: StageTransitionJudge::new(TransitionThresholds::default()),
            store: None,
            pending_transition_notice: None,
        }
    }

    /// 创建带持久化的 RelationshipManager
    pub fn open(data_dir: &str) -> anyhow::Result<Self> {
        let store = RelationshipStore::open(&format!("{}/relationship", data_dir))?;

        let stage = store
            .load_stage()?
            .unwrap_or_else(RelationshipStage::new_acquaintance);

        let metrics = store
            .load_metrics()?
            .unwrap_or_else(RelationshipMetrics::new);

        tracing::info!(
            "RelationshipManager: 加载关系阶段={}, 总交互={}",
            stage.stage_name(),
            stage.interactions()
        );

        Ok(Self {
            stage,
            metrics,
            judge: StageTransitionJudge::new(TransitionThresholds::default()),
            store: Some(store),
            pending_transition_notice: None,
        })
    }

    /// 每条用户消息后调用
    pub fn on_message(&mut self, msg: &str, hour: u8) {
        self.metrics.on_message(msg, hour);
        self.stage.increment_interaction();

        // 检查阶段转换
        if let Some(new_stage) = self.judge.evaluate(&self.stage, &self.metrics) {
            let old_name = self.stage.stage_name();
            let new_name = new_stage.stage_name();

            tracing::info!(
                "关系阶段转换: {} → {} (交互次数={})",
                old_name,
                new_name,
                new_stage.interactions()
            );

            // 记录转换历史
            if let Some(ref store) = self.store {
                let transition = StageTransition {
                    from: old_name.to_string(),
                    to: new_name.to_string(),
                    reason: format!("满足转换条件，交互次数={}", new_stage.interactions()),
                    timestamp: Local::now().timestamp_millis(),
                };
                let _ = store.record_transition(&transition);
            }

            self.pending_transition_notice = Some(format!(
                "感觉我们之间的关系更近了一步——从「{}」变成了「{}」。",
                old_name, new_name
            ));

            self.stage = new_stage;
            self.persist();
        } else {
            // 非转换，仅持久化当前状态
            self.persist();
        }
    }

    /// 获取当前阶段
    pub fn current_stage(&self) -> &RelationshipStage {
        &self.stage
    }

    /// 获取当前行为修饰器
    pub fn behavior_modifiers(&self) -> StageBehaviorModifiers {
        self.stage.behavior_modifiers()
    }

    /// 获取 LLM Prompt 片段
    pub fn to_prompt_fragment(&self) -> String {
        self.stage.to_prompt_fragment()
    }

    /// 获取情感反应乘数
    pub fn affect_multiplier(&self) -> f32 {
        self.stage.affect_multiplier()
    }

    /// 获取主动行为加成
    pub fn proactive_bonus(&self) -> f32 {
        self.stage.proactive_bonus()
    }

    /// 获取当前指标快照
    pub fn metrics(&self) -> &RelationshipMetrics {
        &self.metrics
    }

    /// 消费待处理的阶段转换通知
    pub fn take_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }

    /// 手动添加共享引用（由外部系统调用，如命名仪式、共同经历）
    pub fn add_shared_reference(&mut self) {
        self.metrics.add_shared_reference();
        self.persist();
    }

    /// 手动记录冲突修复
    pub fn record_conflict_repair(&mut self) {
        self.metrics.record_conflict_repair();
        self.persist();
    }

    /// 手动记录关键时刻
    pub fn record_key_moment(&mut self) {
        match &mut self.stage {
            RelationshipStage::Trusted { key_moments, .. } => {
                *key_moments += 1;
            }
            RelationshipStage::Deep { key_moments, .. } => {
                *key_moments += 1;
            }
            _ => {}
        }
        self.persist();
    }

    fn persist(&self) {
        if let Some(ref store) = self.store {
            let _ = store.save_stage(&self.stage);
            let _ = store.save_metrics(&self.metrics);
        }
    }
}

impl Default for RelationshipManager {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquaintance_initial_state() {
        let stage = RelationshipStage::new_acquaintance();
        assert_eq!(stage.stage_name(), "初识");
        assert_eq!(stage.interactions(), 0);
        let modifiers = stage.behavior_modifiers();
        assert_eq!(modifiers.humor_level, HumorLevel::None);
        assert_eq!(modifiers.challenge_level, ChallengeLevel::Compliant);
        assert!(modifiers.boldness < 0.3);
    }

    #[test]
    fn test_increment_interaction() {
        let mut stage = RelationshipStage::new_acquaintance();
        stage.increment_interaction();
        stage.increment_interaction();
        assert_eq!(stage.interactions(), 2);
    }

    #[test]
    fn test_affect_multiplier() {
        let acq = RelationshipStage::new_acquaintance();
        assert!(acq.affect_multiplier() < 1.0);

        let deep = RelationshipStage::Deep {
            since: 0,
            interactions: 1000,
            shared_references: 20,
            key_moments: 5,
        };
        assert!(deep.affect_multiplier() > 1.0);
    }

    #[test]
    fn test_proactive_bonus() {
        let acq = RelationshipStage::new_acquaintance();
        assert!(acq.proactive_bonus() < 0.0);

        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };
        assert!(trusted.proactive_bonus() > 0.0);
    }

    #[test]
    fn test_metrics_resonance_detection() {
        let mut metrics = RelationshipMetrics::new();
        metrics.on_message("太对了，就是这样", 14);
        assert_eq!(metrics.resonance_count, 1);
        assert_eq!(metrics.total_interactions, 1);

        metrics.on_message("今天天气不错", 14);
        assert_eq!(metrics.resonance_count, 1); // 无共鸣
        assert_eq!(metrics.total_interactions, 2);
    }

    #[test]
    fn test_metrics_affirmation_detection() {
        let mut metrics = RelationshipMetrics::new();
        metrics.on_message("有你真好", 20);
        assert_eq!(metrics.relationship_affirmation_count, 1);

        metrics.on_message("谢谢你一直在", 20);
        assert_eq!(metrics.relationship_affirmation_count, 2);
    }

    #[test]
    fn test_metrics_time_diversity() {
        let mut metrics = RelationshipMetrics::new();
        assert_eq!(metrics.time_diversity_count(), 0);

        metrics.on_message("hello", 8); // 早 → bit 0
        assert_eq!(metrics.time_diversity_count(), 1);

        metrics.on_message("hello", 14); // 午 → bit 1
        assert_eq!(metrics.time_diversity_count(), 2);

        metrics.on_message("hello", 8); // 早 → 已有，不变
        assert_eq!(metrics.time_diversity_count(), 2);

        metrics.on_message("hello", 20); // 晚 → bit 2
        assert_eq!(metrics.time_diversity_count(), 3);

        metrics.on_message("hello", 2); // 深夜 → bit 3
        assert_eq!(metrics.time_diversity_count(), 4);
    }

    #[test]
    fn test_transition_acquaintance_to_familiar() {
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 25,
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.resonance_count = 5;
        metrics.return_count = 8;

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Familiar { interactions, .. } => {
                assert_eq!(interactions, 25);
            }
            _ => panic!("应该转为 Familiar"),
        }
    }

    #[test]
    fn test_no_transition_when_conditions_unmet() {
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 10, // 不够 20
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.resonance_count = 5;
        metrics.return_count = 8;

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_transition_familiar_to_trusted() {
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Familiar {
            since: 0,
            interactions: 150,
            shared_references: 15,
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.conflict_repair_count = 2;
        // 设置 3 个时段
        metrics.time_diversity = 0b0111; // 早+午+晚

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Trusted {
                shared_references, ..
            } => {
                assert_eq!(shared_references, 15);
            }
            _ => panic!("应该转为 Trusted"),
        }
    }

    #[test]
    fn test_deep_is_terminal() {
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Deep {
            since: 0,
            interactions: 10000,
            shared_references: 100,
            key_moments: 50,
        };
        let metrics = RelationshipMetrics::new();

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_prompt_fragment_varies_by_stage() {
        let acq = RelationshipStage::new_acquaintance();
        let familiar = RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        };
        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };

        let acq_prompt = acq.to_prompt_fragment();
        let fam_prompt = familiar.to_prompt_fragment();
        let tru_prompt = trusted.to_prompt_fragment();

        assert!(acq_prompt.contains("初识"));
        assert!(fam_prompt.contains("熟悉"));
        assert!(tru_prompt.contains("信任"));
        assert!(tru_prompt.contains("15")); // shared_references 数量
    }

    #[test]
    fn test_relationship_manager_on_message() {
        let mut mgr = RelationshipManager::new();
        assert_eq!(mgr.current_stage().stage_name(), "初识");

        for _ in 0..10 {
            mgr.on_message("测试消息", 14);
        }
        assert_eq!(mgr.current_stage().interactions(), 10);
        assert_eq!(mgr.current_stage().stage_name(), "初识"); // 还没到转换条件
    }

    #[test]
    fn test_relationship_store_roundtrip() {
        let store = RelationshipStore::open_in_memory().unwrap();

        let stage = RelationshipStage::Familiar {
            since: 12345,
            interactions: 50,
            shared_references: 5,
        };
        store.save_stage(&stage).unwrap();
        let loaded = store.load_stage().unwrap().unwrap();
        assert_eq!(loaded.stage_name(), "熟悉");
        assert_eq!(loaded.interactions(), 50);

        let metrics = RelationshipMetrics {
            total_interactions: 50,
            resonance_count: 10,
            return_count: 8,
            conflict_repair_count: 1,
            time_diversity: 0b1111,
            relationship_affirmation_count: 2,
            shared_references: 5,
            first_interaction: 1000,
            last_interaction: 2000,
        };
        store.save_metrics(&metrics).unwrap();
        let loaded = store.load_metrics().unwrap().unwrap();
        assert_eq!(loaded.total_interactions, 50);
        assert_eq!(loaded.resonance_count, 10);
    }

    #[test]
    fn test_store_transition_history() {
        let store = RelationshipStore::open_in_memory().unwrap();

        let t1 = StageTransition {
            from: "初识".into(),
            to: "熟悉".into(),
            reason: "满足条件".into(),
            timestamp: 1000,
        };
        let t2 = StageTransition {
            from: "熟悉".into(),
            to: "信任".into(),
            reason: "满足条件".into(),
            timestamp: 2000,
        };
        store.record_transition(&t1).unwrap();
        store.record_transition(&t2).unwrap();

        let transitions = store.load_transitions().unwrap();
        assert_eq!(transitions.len(), 2);
    }

    #[test]
    fn test_behavior_modifiers_progression() {
        let stages = [
            RelationshipStage::new_acquaintance(),
            RelationshipStage::Familiar {
                since: 0,
                interactions: 50,
                shared_references: 5,
            },
            RelationshipStage::Trusted {
                since: 0,
                interactions: 200,
                shared_references: 15,
                key_moments: 2,
            },
            RelationshipStage::Deep {
                since: 0,
                interactions: 1000,
                shared_references: 30,
                key_moments: 10,
            },
        ];

        let modifiers: Vec<_> = stages.iter().map(|s| s.behavior_modifiers()).collect();

        // boldness 应递增
        for i in 1..modifiers.len() {
            assert!(
                modifiers[i].boldness > modifiers[i - 1].boldness,
                "boldness 应随阶段递增: {:?} vs {:?}",
                modifiers[i].boldness,
                modifiers[i - 1].boldness
            );
        }

        // proactive_frequency 应递增
        for i in 1..modifiers.len() {
            assert!(modifiers[i].proactive_frequency > modifiers[i - 1].proactive_frequency);
        }
    }

    #[test]
    fn test_key_moment_recording() {
        let mut mgr = RelationshipManager::new();

        // 初识/熟悉阶段，key_moment 不影响
        mgr.record_key_moment();
        assert_eq!(mgr.current_stage().stage_name(), "初识");

        // 手动设为 Trusted 阶段
        mgr.stage = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 0,
        };
        mgr.record_key_moment();

        match mgr.current_stage() {
            RelationshipStage::Trusted { key_moments, .. } => {
                assert_eq!(*key_moments, 1);
            }
            _ => panic!("应该是 Trusted"),
        }
    }
}
