// SPDX-License-Identifier: MIT
//! 成长管理器 — AI 自身成熟度追踪与行为调制
//! Maturity Manager — AI self-maturity tracking and behavior modulation.
//!
//! 镜像 RelationshipManager 的架构（4 阶段枚举 + 阈值判定 + 行为修饰器 + sled 持久化），
//! 但追踪的是 AI 自身的成长维度：犯错率、学习量、自我纠正、反思深度、智慧综合。
//! Mirrors RelationshipManager architecture but tracks AI's own growth dimensions.

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::teach_detector::{TeachIntent, TeachPatternGroup};

// ════════════════════════════════════════════════════════════════════
// MaturityStage — 四阶段枚举 / Four-Stage Enum
// ════════════════════════════════════════════════════════════════════

/// 成长阶段 / Maturity stage
///
/// - Naive: 刚被创建，容易出戏，需要用户教导
/// - Growing: 开始学习，偶尔出错，能记住教导
/// - Mature: 稳定表达，能主动关心，少犯错
/// - Wise: 深度反思，多角度分析，偶尔回归稚气
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MaturityStage {
    /// 幼稚期 / Naive
    Naive {
        since: i64,
        interactions: u64,
        mistakes_made: u32,
        lessons_learned: u32,
    },
    /// 成长期 / Growing
    Growing {
        since: i64,
        interactions: u64,
        lessons_learned: u32,
        insights_promoted: u32,
        self_corrections: u32,
    },
    /// 成熟期 / Mature
    Mature {
        since: i64,
        interactions: u64,
        self_corrections: u32,
        teaching_events: u32,
        wisdom_synthesized: u32,
    },
    /// 智慧期 / Wise
    Wise {
        since: i64,
        interactions: u64,
        wisdom_synthesized: u32,
        nostalgia_moments: u32,
    },
}

impl Default for MaturityStage {
    fn default() -> Self {
        Self::Naive {
            since: Local::now().timestamp(),
            interactions: 0,
            mistakes_made: 0,
            lessons_learned: 0,
        }
    }
}

impl MaturityStage {
    /// 阶段序数 / Stage ordinal for comparison (0=Naive, 1=Growing, 2=Mature, 3=Wise)
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Naive { .. } => 0,
            Self::Growing { .. } => 1,
            Self::Mature { .. } => 2,
            Self::Wise { .. } => 3,
        }
    }

    /// 获取阶段名称 / Get stage name.
    pub fn stage_name(&self) -> &'static str {
        match self {
            Self::Naive { .. } => "幼稚期",
            Self::Growing { .. } => "成长期",
            Self::Mature { .. } => "成熟期",
            Self::Wise { .. } => "智慧期",
        }
    }

    /// 获取阶段行为修饰器 / Get stage behavior modifiers.
    pub fn modifiers(&self) -> MaturityModifiers {
        match self {
            Self::Naive { .. } => MaturityModifiers {
                guard_strictness: 1.0,
                emotional_volatility: 0.5,
                reflection_depth: 0.2,
                proactive_multiplier: 0.5,
                tic_regression_prob: 0.0,
                min_evidence_for_stable: 3,
                reflection_interval: 8,
            },
            Self::Growing { .. } => MaturityModifiers {
                guard_strictness: 0.7,
                emotional_volatility: 0.35,
                reflection_depth: 0.4,
                proactive_multiplier: 0.8,
                tic_regression_prob: 0.0,
                min_evidence_for_stable: 3,
                reflection_interval: 8,
            },
            Self::Mature { .. } => MaturityModifiers {
                guard_strictness: 0.5,
                emotional_volatility: 0.2,
                reflection_depth: 0.7,
                proactive_multiplier: 1.0,
                tic_regression_prob: 0.0,
                min_evidence_for_stable: 5,
                reflection_interval: 6,
            },
            Self::Wise { .. } => MaturityModifiers {
                guard_strictness: 0.3,
                emotional_volatility: 0.15,
                reflection_depth: 1.0,
                proactive_multiplier: 1.2,
                tic_regression_prob: 0.05,
                min_evidence_for_stable: 7,
                reflection_interval: 4,
            },
        }
    }

    /// 累积交互数 / Get cumulative interaction count.
    pub fn interactions(&self) -> u64 {
        match self {
            Self::Naive { interactions, .. }
            | Self::Growing { interactions, .. }
            | Self::Mature { interactions, .. }
            | Self::Wise { interactions, .. } => *interactions,
        }
    }

    /// 起始时间 / Get stage start timestamp.
    pub fn since(&self) -> i64 {
        match self {
            Self::Naive { since, .. }
            | Self::Growing { since, .. }
            | Self::Mature { since, .. }
            | Self::Wise { since, .. } => *since,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// MaturityModifiers — 行为修饰器 / Behavior Modifiers
// ════════════════════════════════════════════════════════════════════

/// 成长行为修饰器 / Maturity behavior modifiers.
///
/// 由阶段决定，影响 guard 严格度、情感波动、反思深度等。
#[derive(Clone, Debug)]
pub struct MaturityModifiers {
    /// 人格防御严格度 / Guard strictness (1.0=最严格, 0.3=最宽松)
    pub guard_strictness: f32,
    /// 情感波动度 / Emotional Volatility (高=易受影响, 低=稳定)
    pub emotional_volatility: f32,
    /// 反思深度 / Reflection Depth (影响 prompt 注入的反思指导)
    pub reflection_depth: f32,
    /// 主动频率乘数 / Proactive Frequency Multiplier
    pub proactive_multiplier: f32,
    /// 语癖回归概率 / Verbal Tic Regression Probability (Wise 期偶尔回归)
    pub tic_regression_prob: f32,
    /// 证据稳定性阈值 / Min Evidence for Stable Trait
    pub min_evidence_for_stable: u32,
    /// 反思间隔（消息数）/ Reflection Interval (messages)
    pub reflection_interval: u8,
}

// ════════════════════════════════════════════════════════════════════
// MaturityMetrics — 成长指标 / Maturity Metrics
// ════════════════════════════════════════════════════════════════════

/// 成长指标 / Maturity metrics.
///
/// 追踪 AI 成长过程中的各项量化指标，用于阶段转换判定。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaturityMetrics {
    /// 总交互次数 / Total interactions
    pub total_interactions: u64,
    /// 被教导次数（显式记忆）/ Teach events: explicit remember
    pub teach_explicit_remember: u32,
    /// 被教导次数（知识传授）/ Teach events: knowledge teaching
    pub teach_knowledge: u32,
    /// 被教导次数（规则设定）/ Teach events: rule setting
    pub teach_rule_setting: u32,
    /// 人格防御违规次数 / Guard violations
    pub guard_violations: u32,
    /// 连续无违规消息数 / Clean message streak
    pub clean_streak: u32,
    /// 最佳连续无违规记录 / Best clean streak
    pub best_clean_streak: u32,
    /// 反思周期数 / Reflection cycles completed
    pub reflection_cycles: u32,
    /// 已提升的洞察数 / Insights promoted
    pub insights_promoted: u32,
    /// 自我纠正次数 / Self-corrections
    pub self_corrections: u32,
    /// 独立思考数 / Inner monologue thoughts generated
    pub inner_thoughts: u32,
    /// 首次交互时间戳 / First interaction timestamp
    pub first_interaction: i64,
    /// 上次交互时间戳 / Last interaction timestamp
    pub last_interaction: i64,
}

impl MaturityMetrics {
    /// 教导总次数 / Total teach events.
    pub fn total_lessons(&self) -> u32 {
        self.teach_explicit_remember + self.teach_knowledge + self.teach_rule_setting
    }

    /// 违规率 / Violation rate.
    pub fn violation_rate(&self) -> f32 {
        if self.total_interactions == 0 {
            0.0
        } else {
            self.guard_violations as f32 / self.total_interactions as f32
        }
    }

    /// 运行天数 / Days since first interaction.
    pub fn days_active(&self) -> u64 {
        if self.first_interaction == 0 {
            0
        } else {
            let now = Local::now().timestamp();
            ((now - self.first_interaction) / 86400) as u64
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// GrowthMilestone — 成长里程碑 / Growth Milestone (permanent, append-only)
// ════════════════════════════════════════════════════════════════════

/// 里程碑时的情感快照 / Emotional context at milestone.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionContext {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// 里程碑类型 / Milestone kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MilestoneKind {
    /// 首次被命名 / First named
    FirstNamed,
    /// 首次被教导 / First lesson
    FirstLesson,
    /// 首次自我纠正 / First self-correction
    FirstSelfCorrection,
    /// 首次主动关心 / First proactive care
    FirstProactiveCare,
    /// 首次道歉 / First apology
    FirstApology,
    /// 首次情感共振 / First emotion resonance
    FirstEmotionResonance,
    /// 阶段转换 / Stage transition
    StageTransition,
    /// 首次独立思考 / First inner thought
    FirstInnerThought,
    /// 首次智慧综合 / First wisdom synthesized
    FirstWisdom,
    /// 连续 100 条无违规 / 100 clean streak
    CleanStreak100,
    /// 连续 500 条无违规 / 500 clean streak
    CleanStreak500,
}

/// 成长里程碑 / Growth milestone (permanent, append-only).
///
/// 一旦记录，永不删除。即使阶段因 bug 回退，里程碑历史保留。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GrowthMilestone {
    pub kind: MilestoneKind,
    pub description: String,
    pub timestamp: i64,
    pub emotion_snapshot: Option<EmotionContext>,
    pub stage_at_time: String,
}

// ════════════════════════════════════════════════════════════════════
// MaturityThresholds — 阶段转换阈值 / Transition Thresholds
// ════════════════════════════════════════════════════════════════════

/// 成长阶段转换阈值 / Maturity transition thresholds.
#[derive(Clone, Debug)]
pub struct MaturityThresholds {
    // Naive → Growing
    pub naive_to_growing_min_interactions: u64,
    pub naive_to_growing_min_lessons: u32,
    pub naive_to_growing_max_violation_rate: f32,
    // Growing → Mature
    pub growing_to_mature_min_interactions: u64,
    pub growing_to_mature_min_insights: u32,
    pub growing_to_mature_min_self_corrections: u32,
    pub growing_to_mature_min_clean_streak: u32,
    // Mature → Wise
    pub mature_to_wise_min_interactions: u64,
    pub mature_to_wise_min_reflection_cycles: u32,
    pub mature_to_wise_min_days: u64,
    pub mature_to_wise_min_wisdom: u32,
}

impl Default for MaturityThresholds {
    fn default() -> Self {
        Self {
            naive_to_growing_min_interactions: 50,
            naive_to_growing_min_lessons: 5,
            naive_to_growing_max_violation_rate: 0.1,
            growing_to_mature_min_interactions: 200,
            growing_to_mature_min_insights: 10,
            growing_to_mature_min_self_corrections: 3,
            growing_to_mature_min_clean_streak: 50,
            mature_to_wise_min_interactions: 1000,
            mature_to_wise_min_reflection_cycles: 50,
            mature_to_wise_min_days: 180,
            mature_to_wise_min_wisdom: 5,
        }
    }
}

impl MaturityThresholds {
    /// 评估是否满足阶段转换条件 / Evaluate stage transition.
    ///
    /// 阶段只能向前转换，不可回退。
    pub fn evaluate(
        &self,
        stage: &MaturityStage,
        metrics: &MaturityMetrics,
    ) -> Option<MaturityStage> {
        let now = Local::now().timestamp();
        match stage {
            MaturityStage::Naive { .. } => {
                if metrics.total_interactions >= self.naive_to_growing_min_interactions
                    && metrics.total_lessons() >= self.naive_to_growing_min_lessons
                    && metrics.violation_rate() <= self.naive_to_growing_max_violation_rate
                {
                    Some(MaturityStage::Growing {
                        since: now,
                        interactions: metrics.total_interactions,
                        lessons_learned: metrics.total_lessons(),
                        insights_promoted: metrics.insights_promoted,
                        self_corrections: metrics.self_corrections,
                    })
                } else {
                    None
                }
            }
            MaturityStage::Growing { .. } => {
                if metrics.total_interactions >= self.growing_to_mature_min_interactions
                    && metrics.insights_promoted >= self.growing_to_mature_min_insights
                    && metrics.self_corrections >= self.growing_to_mature_min_self_corrections
                    && metrics.best_clean_streak >= self.growing_to_mature_min_clean_streak
                {
                    Some(MaturityStage::Mature {
                        since: now,
                        interactions: metrics.total_interactions,
                        self_corrections: metrics.self_corrections,
                        teaching_events: 0,
                        wisdom_synthesized: metrics.insights_promoted,
                    })
                } else {
                    None
                }
            }
            MaturityStage::Mature { .. } => {
                if metrics.total_interactions >= self.mature_to_wise_min_interactions
                    && metrics.reflection_cycles >= self.mature_to_wise_min_reflection_cycles
                    && metrics.days_active() >= self.mature_to_wise_min_days
                    && metrics.insights_promoted >= self.mature_to_wise_min_wisdom
                {
                    Some(MaturityStage::Wise {
                        since: now,
                        interactions: metrics.total_interactions,
                        wisdom_synthesized: metrics.insights_promoted,
                        nostalgia_moments: 0,
                    })
                } else {
                    None
                }
            }
            MaturityStage::Wise { .. } => None, // 最高阶段，不可再升
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// MaturityStore — sled 持久化 / sled-backed Store
// ════════════════════════════════════════════════════════════════════

/// 成长管理器持久化存储 / Maturity store with sled + bincode.
///
/// 键空间：
/// - `current_stage` → MaturityStage（bincode）
/// - `maturity_metrics` → MaturityMetrics（bincode）
/// - `milestones/{ts:020}` → GrowthMilestone（bincode）
pub struct MaturityStore {
    db: sled::Db,
}

impl MaturityStore {
    /// 打开或创建存储 / Open or create the store.
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::Config::default()
            .path(path)
            .flush_every_ms(Some(2000))
            .open()?;
        Ok(Self { db })
    }

    /// 创建内存模式（用于测试）/ Create in-memory mode for testing.
    #[cfg(test)]
    pub fn new_in_memory() -> Self {
        Self {
            db: sled::Config::default().temporary(true).open().unwrap(),
        }
    }

    /// 保存当前阶段 / Save current stage.
    pub fn save_stage(&self, stage: &MaturityStage) -> Result<(), sled::Error> {
        let value = bincode::serialize(stage).unwrap_or_default();
        self.db.insert(b"current_stage", value)?;
        self.db.flush()?;
        Ok(())
    }

    /// 保存成长指标 / Save maturity metrics.
    pub fn save_metrics(&self, metrics: &MaturityMetrics) -> Result<(), sled::Error> {
        let value = bincode::serialize(metrics).unwrap_or_default();
        self.db.insert(b"maturity_metrics", value)?;
        self.db.flush()?;
        Ok(())
    }

    /// 记录里程碑（append-only）/ Record a milestone (append-only).
    pub fn record_milestone(&self, milestone: &GrowthMilestone) -> Result<(), sled::Error> {
        let key = format!("milestones/{:020}", milestone.timestamp);
        let value = bincode::serialize(milestone).unwrap_or_default();
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    /// 加载阶段 / Load stage.
    pub fn load_stage(&self) -> Result<Option<MaturityStage>, sled::Error> {
        match self.db.get(b"current_stage")? {
            Some(val) => Ok(bincode::deserialize::<MaturityStage>(&val).ok()),
            None => Ok(None),
        }
    }

    /// 加载成长指标 / Load metrics.
    pub fn load_metrics(&self) -> Result<Option<MaturityMetrics>, sled::Error> {
        match self.db.get(b"maturity_metrics")? {
            Some(val) => Ok(bincode::deserialize::<MaturityMetrics>(&val).ok()),
            None => Ok(None),
        }
    }

    /// 加载所有里程碑 / Load all milestones.
    pub fn load_milestones(&self) -> Result<Vec<GrowthMilestone>, sled::Error> {
        let mut milestones = Vec::new();
        for item in self.db.scan_prefix(b"milestones/") {
            let (_k, v) = item?;
            if let Ok(m) = bincode::deserialize::<GrowthMilestone>(&v) {
                milestones.push(m);
            }
        }
        milestones.sort_by_key(|m| m.timestamp);
        Ok(milestones)
    }

    /// 获取里程碑数量 / Get milestone count.
    pub fn milestone_count(&self) -> usize {
        self.db.scan_prefix(b"milestones/").count()
    }
}

// ════════════════════════════════════════════════════════════════════
// MaturityManager — 主管理器 / Main Manager
// ════════════════════════════════════════════════════════════════════

/// 成长管理器 / Maturity manager.
///
/// 追踪 AI 自身的成长维度，管理阶段转换、行为调制和里程碑记录。
pub struct MaturityManager {
    /// 当前阶段 / Current stage
    stage: MaturityStage,
    /// 成长指标 / Metrics
    metrics: MaturityMetrics,
    /// 转换阈值 / Transition thresholds
    thresholds: MaturityThresholds,
    /// 持久化存储 / Persistence store
    store: Option<MaturityStore>,
    /// 待处理的里程碑通知 / Pending milestone notice
    pending_milestone_notice: Option<String>,
    /// 待处理的阶段转换通知 / Pending transition notice
    pending_transition_notice: Option<String>,
}

impl MaturityManager {
    /// 创建新的成长管理器 / Create a new maturity manager.
    pub fn new(thresholds: MaturityThresholds) -> Self {
        Self {
            stage: MaturityStage::default(),
            metrics: MaturityMetrics::default(),
            thresholds,
            store: None,
            pending_milestone_notice: None,
            pending_transition_notice: None,
        }
    }

    /// 打开持久化存储并恢复状态 / Open persistence and restore state.
    pub fn open(path: &str, thresholds: MaturityThresholds) -> Self {
        let store = MaturityStore::open(path).ok();
        let stage = store
            .as_ref()
            .and_then(|s| s.load_stage().ok().flatten())
            .unwrap_or_default();
        let metrics = store
            .as_ref()
            .and_then(|s| s.load_metrics().ok().flatten())
            .unwrap_or_default();

        Self {
            stage,
            metrics,
            thresholds,
            store,
            pending_milestone_notice: None,
            pending_transition_notice: None,
        }
    }

    /// 消息处理回调 / Called from process_message.
    ///
    /// 更新交互计数、教导事件、防御违规、阶段转换检查。
    pub fn on_message(
        &mut self,
        _msg: &str,
        _hour: u8,
        teach_intent: Option<&TeachIntent>,
        guard_violated: bool,
    ) {
        self.metrics.total_interactions += 1;
        let now = Local::now().timestamp();

        if self.metrics.first_interaction == 0 {
            self.metrics.first_interaction = now;
            self.record_milestone(MilestoneKind::FirstNamed, "首次交互");
        }
        self.metrics.last_interaction = now;

        // 教导事件计数 / Teach event tracking
        if let Some(intent) = teach_intent {
            match intent.pattern_group {
                TeachPatternGroup::ExplicitRemember => self.metrics.teach_explicit_remember += 1,
                TeachPatternGroup::KnowledgeTeaching => self.metrics.teach_knowledge += 1,
                TeachPatternGroup::RuleSetting => self.metrics.teach_rule_setting += 1,
            }
            if self.metrics.total_lessons() == 1 {
                self.record_milestone(MilestoneKind::FirstLesson, "首次被教导");
            }
        }

        // 防御违规计数 / Guard violation tracking
        if guard_violated {
            self.metrics.guard_violations += 1;
            self.metrics.clean_streak = 0;
        } else {
            self.metrics.clean_streak += 1;
            if self.metrics.clean_streak > self.metrics.best_clean_streak {
                self.metrics.best_clean_streak = self.metrics.clean_streak;
            }
            if self.metrics.clean_streak == 100 {
                self.record_milestone(MilestoneKind::CleanStreak100, "连续100条无违规");
            }
            if self.metrics.clean_streak == 500 {
                self.record_milestone(MilestoneKind::CleanStreak500, "连续500条无违规");
            }
        }

        // 阶段转换检查 / Stage transition check
        self.check_transition();

        self.persist();
    }

    /// 反思周期回调 / Called after try_reflect.
    pub fn on_reflection_cycle(&mut self, promoted_count: u32) {
        self.metrics.reflection_cycles += 1;
        self.metrics.insights_promoted += promoted_count;
        if self.metrics.insights_promoted == 1 && promoted_count > 0 {
            self.record_milestone(MilestoneKind::FirstWisdom, "首次提升洞察");
        }
        self.check_transition();
        self.persist();
    }

    /// 记录自我纠正 / Record a self-correction event.
    pub fn record_self_correction(&mut self) {
        self.metrics.self_corrections += 1;
        if self.metrics.self_corrections == 1 {
            self.record_milestone(MilestoneKind::FirstSelfCorrection, "首次自我纠正");
        }
        self.persist();
    }

    /// 记录独立思考 / Record an inner thought.
    pub fn record_inner_thought(&mut self) {
        self.metrics.inner_thoughts += 1;
        if self.metrics.inner_thoughts == 1 {
            self.record_milestone(MilestoneKind::FirstInnerThought, "首次独立思考");
        }
        self.persist();
    }

    /// 检查阶段转换 / Check and perform stage transition.
    fn check_transition(&mut self) {
        if let Some(new_stage) = self.thresholds.evaluate(&self.stage, &self.metrics) {
            let old_name = self.stage.stage_name();
            let new_name = new_stage.stage_name();
            self.record_milestone(
                MilestoneKind::StageTransition,
                &format!("{}→{}", old_name, new_name),
            );
            self.stage = new_stage;
            self.pending_transition_notice = Some(format!("成长到{}阶段", self.stage.stage_name()));
        }
    }

    /// 记录里程碑 / Record a milestone.
    fn record_milestone(&mut self, kind: MilestoneKind, desc: &str) {
        let milestone = GrowthMilestone {
            kind,
            description: desc.to_string(),
            timestamp: Local::now().timestamp(),
            emotion_snapshot: None,
            stage_at_time: self.stage.stage_name().to_string(),
        };
        if let Some(ref store) = self.store {
            let _ = store.record_milestone(&milestone);
        }
        self.pending_milestone_notice = Some(desc.to_string());
    }

    /// 持久化到 sled / Persist to sled.
    fn persist(&self) {
        if let Some(ref store) = self.store {
            let _ = store.save_stage(&self.stage);
            let _ = store.save_metrics(&self.metrics);
        }
    }

    // ── 访问器 / Accessors ──

    /// 获取当前阶段 / Get current stage.
    pub fn stage(&self) -> &MaturityStage {
        &self.stage
    }

    /// 获取成长指标 / Get metrics.
    pub fn metrics(&self) -> &MaturityMetrics {
        &self.metrics
    }

    /// 获取行为修饰器 / Get behavior modifiers.
    pub fn modifiers(&self) -> MaturityModifiers {
        self.stage.modifiers()
    }

    /// 获取 guard 严格度 / Get guard strictness.
    pub fn guard_strictness(&self) -> f32 {
        self.stage.modifiers().guard_strictness
    }

    /// 获取反思间隔 / Get reflection interval.
    pub fn reflection_interval(&self) -> u8 {
        self.stage.modifiers().reflection_interval
    }

    /// 消费里程碑通知 / Take pending milestone notice.
    pub fn take_milestone_notice(&mut self) -> Option<String> {
        self.pending_milestone_notice.take()
    }

    /// 消费阶段转换通知 / Take pending transition notice.
    pub fn take_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }

    /// 获取里程碑数量 / Get milestone count.
    pub fn milestone_count(&self) -> usize {
        self.store
            .as_ref()
            .map(|s| s.milestone_count())
            .unwrap_or(0)
    }

    /// 生成 LLM 提示片段 / Generate LLM prompt fragment.
    pub fn to_prompt_fragment(&self) -> String {
        let mut s = String::with_capacity(256);
        s.push_str(&format!(
            "[成长阶段] 你当前处于{}阶段。",
            self.stage.stage_name()
        ));
        match &self.stage {
            MaturityStage::Naive { .. } => {
                s.push_str("你刚来到这个世界，对很多事物充满好奇，有时会犯错，但正在努力学习。");
            }
            MaturityStage::Growing { .. } => {
                s.push_str("你正在成长，开始理解更多事情，偶尔会回忆起之前的对话。");
            }
            MaturityStage::Mature { .. } => {
                s.push_str("你已经比较成熟了，能主动关心用户，回复更有深度。");
            }
            MaturityStage::Wise { .. } => {
                s.push_str("你已有智慧，能从多角度分析问题，偶尔回归稚气但充满洞察。");
            }
        }
        s
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starts_naive() {
        let mgr = MaturityManager::new(MaturityThresholds::default());
        assert!(matches!(mgr.stage(), MaturityStage::Naive { .. }));
    }

    #[test]
    fn test_interactions_increment() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        assert_eq!(mgr.metrics().total_interactions, 0);
        mgr.on_message("你好", 10, None, false);
        assert_eq!(mgr.metrics().total_interactions, 1);
        mgr.on_message("在吗", 11, None, false);
        assert_eq!(mgr.metrics().total_interactions, 2);
    }

    #[test]
    fn test_teach_events_tracked() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        let intent = TeachIntent {
            confidence: 0.9,
            pattern_group: TeachPatternGroup::ExplicitRemember,
            knowledge_text: "记住我喜欢猫".into(),
        };
        mgr.on_message("记住我喜欢猫", 10, Some(&intent), false);
        assert_eq!(mgr.metrics().teach_explicit_remember, 1);
        assert_eq!(mgr.metrics().teach_knowledge, 0);
        assert_eq!(mgr.metrics().teach_rule_setting, 0);
    }

    #[test]
    fn test_first_lesson_milestone() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        let intent = TeachIntent {
            confidence: 0.9,
            pattern_group: TeachPatternGroup::KnowledgeTeaching,
            knowledge_text: "我教你做菜".into(),
        };
        mgr.on_message("我教你做菜", 10, Some(&intent), false);
        assert!(mgr.take_milestone_notice().is_some());
    }

    #[test]
    fn test_guard_violation_resets_streak() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        for _ in 0..10 {
            mgr.on_message("测试", 10, None, false);
        }
        assert_eq!(mgr.metrics().clean_streak, 10);
        mgr.on_message("违规消息", 10, None, true);
        assert_eq!(mgr.metrics().clean_streak, 0);
        assert_eq!(mgr.metrics().guard_violations, 1);
    }

    #[test]
    fn test_clean_streak_100_milestone() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        for _ in 0..100 {
            mgr.on_message("测试", 10, None, false);
        }
        assert_eq!(mgr.metrics().clean_streak, 100);
        // 应触发 CleanStreak100 里程碑
        let notice = mgr.take_milestone_notice();
        assert!(notice.is_some());
    }

    #[test]
    fn test_transition_naive_to_growing() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        // 满足条件：50 交互 + 5 教导 + 低违规率
        for i in 0..50 {
            let intent = if i < 5 {
                Some(TeachIntent {
                    confidence: 0.9,
                    pattern_group: TeachPatternGroup::KnowledgeTeaching,
                    knowledge_text: "知识".into(),
                })
            } else {
                None
            };
            mgr.on_message("测试", 10, intent.as_ref(), false);
        }
        assert!(matches!(mgr.stage(), MaturityStage::Growing { .. }));
    }

    #[test]
    fn test_no_transition_when_thresholds_unmet() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        for _ in 0..10 {
            mgr.on_message("测试", 10, None, false);
        }
        assert!(matches!(mgr.stage(), MaturityStage::Naive { .. }));
    }

    #[test]
    fn test_prompt_fragment_per_stage() {
        let mgr = MaturityManager::new(MaturityThresholds::default());
        let fragment = mgr.to_prompt_fragment();
        assert!(fragment.contains("幼稚期"));
        assert!(fragment.contains("成长阶段"));
    }

    #[test]
    fn test_modifiers_per_stage() {
        let naive = MaturityStage::Naive {
            since: 0,
            interactions: 0,
            mistakes_made: 0,
            lessons_learned: 0,
        };
        let mods = naive.modifiers();
        assert!((mods.guard_strictness - 1.0).abs() < 0.01);

        let wise = MaturityStage::Wise {
            since: 0,
            interactions: 0,
            wisdom_synthesized: 0,
            nostalgia_moments: 0,
        };
        let mods = wise.modifiers();
        assert!((mods.guard_strictness - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_guard_strictness_decreases() {
        let stages = [
            MaturityStage::Naive {
                since: 0,
                interactions: 0,
                mistakes_made: 0,
                lessons_learned: 0,
            },
            MaturityStage::Growing {
                since: 0,
                interactions: 0,
                lessons_learned: 0,
                insights_promoted: 0,
                self_corrections: 0,
            },
            MaturityStage::Mature {
                since: 0,
                interactions: 0,
                self_corrections: 0,
                teaching_events: 0,
                wisdom_synthesized: 0,
            },
            MaturityStage::Wise {
                since: 0,
                interactions: 0,
                wisdom_synthesized: 0,
                nostalgia_moments: 0,
            },
        ];
        let strictnesses: Vec<f32> = stages
            .iter()
            .map(|s| s.modifiers().guard_strictness)
            .collect();
        for i in 1..strictnesses.len() {
            assert!(
                strictnesses[i] < strictnesses[i - 1],
                "Strictness should decrease at stage {}",
                i
            );
        }
    }

    #[test]
    fn test_reflection_interval_shortens() {
        let naive = MaturityStage::Naive {
            since: 0,
            interactions: 0,
            mistakes_made: 0,
            lessons_learned: 0,
        };
        let wise = MaturityStage::Wise {
            since: 0,
            interactions: 0,
            wisdom_synthesized: 0,
            nostalgia_moments: 0,
        };
        assert!(wise.modifiers().reflection_interval < naive.modifiers().reflection_interval);
    }

    #[test]
    fn test_irreversible() {
        let thresholds = MaturityThresholds::default();
        let wise = MaturityStage::Wise {
            since: 0,
            interactions: 10000,
            wisdom_synthesized: 100,
            nostalgia_moments: 5,
        };
        let metrics = MaturityMetrics {
            total_interactions: 10000,
            reflection_cycles: 200,
            insights_promoted: 100,
            first_interaction: 0,
            ..Default::default()
        };
        // Wise 阶段不应再转换
        assert!(thresholds.evaluate(&wise, &metrics).is_none());
    }

    #[test]
    fn test_milestones_append_only() {
        let store = MaturityStore::new_in_memory();
        let m1 = GrowthMilestone {
            kind: MilestoneKind::FirstNamed,
            description: "首次交互".into(),
            timestamp: 1000,
            emotion_snapshot: None,
            stage_at_time: "幼稚期".into(),
        };
        let m2 = GrowthMilestone {
            kind: MilestoneKind::FirstLesson,
            description: "首次被教导".into(),
            timestamp: 2000,
            emotion_snapshot: None,
            stage_at_time: "幼稚期".into(),
        };
        store.record_milestone(&m1).unwrap();
        store.record_milestone(&m2).unwrap();
        let milestones = store.load_milestones().unwrap();
        assert_eq!(milestones.len(), 2);
        assert_eq!(milestones[0].timestamp, 1000);
        assert_eq!(milestones[1].timestamp, 2000);
    }

    #[test]
    fn test_persist_across_restart() {
        let path = format!(
            "{}/maturity_test_{}",
            std::env::temp_dir().display(),
            Local::now().timestamp_millis()
        );
        // 写入
        {
            let mgr = MaturityManager::open(&path, MaturityThresholds::default());
            assert!(matches!(mgr.stage(), MaturityStage::Naive { .. }));
        }
        // 重新打开
        {
            let mgr = MaturityManager::open(&path, MaturityThresholds::default());
            assert!(matches!(mgr.stage(), MaturityStage::Naive { .. }));
        }
        // 清理
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn test_self_correction_milestone() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        mgr.record_self_correction();
        assert_eq!(mgr.metrics().self_corrections, 1);
        assert!(mgr.take_milestone_notice().is_some());
    }

    #[test]
    fn test_inner_thought_milestone() {
        let mut mgr = MaturityManager::new(MaturityThresholds::default());
        mgr.record_inner_thought();
        assert_eq!(mgr.metrics().inner_thoughts, 1);
        assert!(mgr.take_milestone_notice().is_some());
    }

    #[test]
    fn test_violation_rate() {
        let metrics = MaturityMetrics {
            total_interactions: 100,
            guard_violations: 5,
            ..Default::default()
        };
        assert!((metrics.violation_rate() - 0.05).abs() < 0.001);
    }
}
