// SPDX-License-Identifier: MIT
//! 核心服务实现 — 集成所有记忆增强模块
//!
//! 处理管线:
//!   用户消息 → STM 存储 → 事实提取 → 证据评分 → FactStore
//!   → 周期性 Reflection → Persona 固化 → FTS5 索引

pub(crate) use crate::guard::PersonaGuard;
pub(crate) use crate::metrics;
pub(crate) use crate::proactive::ProactiveEngine;
pub(crate) use async_trait::async_trait;
pub(crate) use atrium_bridge::grpc::AtriumCoreService;
pub(crate) use atrium_emotion::{
    AnticipationPreLoader, CircadianModulator, DisappointmentHandler, DriftParams, EmotionEngine,
    EmotionState as EmotionEngineState, EmotionalInertia, LongingExpressionChannel,
    LongingNarrativeBridge, LongingParams, LongingState,
};
pub(crate) use atrium_memory::{
    associative::AssociativeGraph,
    canned::CannedManager,
    consolidation::{ConsolidationConfig, MemoryConsolidator},
    empathy::EmpathyEngine,
    evidence::{EvidenceScorer, SourceType},
    fact_extractor,
    fact_store::{Fact, FactStore},
    feedback::FeedbackLoop,
    fts5_index::Fts5Index,
    graph_store::GraphStore,
    history::ConversationHistory,
    inner_dialogue::InnerDialogueEngine,
    key_fact_cache::KeyFactCache,
    longing_accumulation_store::LongingAccumulationStore,
    perception::{compile_rhythm_hint, MessageEvent, TypingRhythm, TypingRhythmAnalyzer},
    persona::PersonaManager as RuntimePersonaManager,
    preference::PreferenceManager,
    reflection::ReflectionEngine,
    relationship::RelationshipManager,
    replay::ReplayPipeline,
    rules::{RuleContext, RuleEngine},
    style_memory::{StyleMemoryStore, StyleOffset},
    subtext_engine::{SubtextCategory, SubtextEngine, SubtextSignal},
    summarizer::{ConversationSummarizer, SummaryConfig},
    token_budget::TokenBudget,
    user_model::UserMentalModel,
    user_model_store::UserMentalModelStore,
    MemoryContent, MemoryEntry, MemoryManager, SledLtm, StmBuffer,
};
pub(crate) use atrium_persona::manager::PersonaManager;
pub(crate) use chrono::Timelike;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
pub(crate) use std::time::Instant;
pub(crate) use tokio_stream;

// 叙事自我子模块 / Narrative self submodule
mod narrative;

// 情感系统子模块 / Emotion system submodule
mod emotion;

// 表达与关系子模块 / Expression & relationship submodule
mod expression;

// 内心独白子模块 / Inner monologue submodule
mod monologue;

// 感知与守卫子模块 / Perception & guard submodule
mod perception;

// 认知与记忆子模块 / Cognition & memory submodule
mod cognition;

// 生命维持子模块 / Lifecycle submodule
mod lifecycle;

// 外部接口子模块 / External API submodule
mod api_handler;

// 子系统泛型容器与域子结构 / Subsystem generic container and domain sub-structures
mod subsystems;
pub(crate) use subsystems::{
    CuriositySubsystem, LongingSubsystem, NarrativeSubsystem, RitualSubsystem, SolitudeSubsystem,
    Subsystem, VulnerabilitySubsystem,
};

// ════════════════════════════════════════════════════════════════════
// CoreLlmAdapter — LLM 客户端桥接器 / LLM Client Bridge Adapter
// ════════════════════════════════════════════════════════════════════

// ════════════════════════════════════════════════════════════════════
// CoreService — 核心服务 / Core Service
// ════════════════════════════════════════════════════════════════════

pub struct CoreService {
    memory: parking_lot::Mutex<MemoryManager<StmBuffer, SledLtm>>,
    emotion: parking_lot::Mutex<EmotionEngine>,
    persona: parking_lot::Mutex<PersonaManager>,
    // ── 三层记忆增强 ──
    fact_store: FactStore,
    evidence: EvidenceScorer,
    fts5: parking_lot::Mutex<Fts5Index>,
    reflection: parking_lot::Mutex<ReflectionEngine>,
    runtime_persona: parking_lot::Mutex<RuntimePersonaManager>,
    // ── 运行时计数 ──
    message_count: AtomicU64,
    /// 上次触发 reflection 时的消息数
    last_reflection_at: AtomicU64,
    // ── 上下文窗口压缩 ──
    token_budget: parking_lot::Mutex<TokenBudget>,
    summarizer: parking_lot::Mutex<ConversationSummarizer>,
    key_facts: KeyFactCache,
    // ── 人格防御 ──
    guard: parking_lot::Mutex<PersonaGuard>,
    // ── 偏好学习 ──
    preferences: parking_lot::Mutex<PreferenceManager>,
    // ── 回放管道 ──
    replay: parking_lot::Mutex<ReplayPipeline>,
    // ── 行为规则 ──
    rules: parking_lot::Mutex<RuleEngine>,
    // ── 对话历史 ──
    history: ConversationHistory,
    // ── 启动时间 ──
    started_at: Instant,
    // ── 罐装知识 ──
    canned: parking_lot::Mutex<CannedManager>,
    // ── LLM 客户端（P1-4 统一 trait 对象）/ LLM client (P1-4 unified trait object) ──
    llm_client:
        parking_lot::Mutex<Option<std::sync::Arc<dyn atrium_memory::llm_client::LlmClient>>>,
    // ── Room 群聊引擎 / Room Engine ──
    room: parking_lot::Mutex<crate::room::RoomEngine>,
    // ── Room 输出队列（Python 网关轮询消费）/ Room outgoing queue (polled by Python gateway) ──
    room_outgoing: parking_lot::Mutex<std::collections::VecDeque<OutgoingRoomMessage>>,
    // ── Room 待处理触发器（health_check 收到消息后标记，Scheduler 异步 LLM）──
    pending_room_trigger: parking_lot::Mutex<Option<crate::room::SpeakDecision>>,
    // ── 关系阶段模型 / Relationship Stage Model ──
    relationship: parking_lot::Mutex<RelationshipManager>,
    // ── 用户心智模型 / User Mental Model ──
    user_model: parking_lot::Mutex<UserMentalModel>,
    /// 用户心智模型持久化存储 / User mental model persistence store
    user_model_store: Option<UserMentalModelStore>,
    /// 用户心智模型防抖写穿计数 / User model debounced write-through counter
    user_model_unsaved_count: AtomicU32,
    // ── 实时反馈闭环 / Feedback Loop ──
    feedback: parking_lot::Mutex<FeedbackLoop>,
    // ── 主动决策引擎 / Proactive Decision Engine ──
    proactive: parking_lot::Mutex<ProactiveEngine>,
    // ── 关联记忆图 / Associative Memory Graph ──
    graph: parking_lot::Mutex<AssociativeGraph>,
    graph_store: GraphStore,
    graph_dirty: AtomicBool,
    last_graph_save_at: AtomicU64,
    // ── 非语言感知层 / Perception Layer ──
    typing_analyzer: parking_lot::Mutex<TypingRhythmAnalyzer>,
    perception_enabled: bool,
    // ── 高阶情绪模型 / Compound Emotion Model ──
    compound_enabled: bool,
    // ── 情感持久化 / Emotion Persistence ──
    emotion_store: Option<atrium_memory::emotion_store::EmotionStore>,
    // ── 记忆巩固 / Memory Consolidation ──
    consolidator: parking_lot::Mutex<MemoryConsolidator>,
    // ── 共情推理引擎 / Empathy Engine ──
    empathy: parking_lot::Mutex<EmpathyEngine>,
    // ── ACK 自学习 / ACK Self-Learning ──
    ack_learning_cfg: crate::config::AckLearningCfg,
    teach_detected: parking_lot::Mutex<Option<atrium_memory::teach_detector::TeachIntent>>,
    // ── 期待事件存储 / Anticipation Event Store ──
    anticipation_store: Option<atrium_memory::anticipation_store::AnticipationStore>,
    // ── 想念引擎配置 / Longing Engine Config ──
    longing_cfg: crate::config::LongingCfg,
    // ── Gap#3 期待与想念增强 / Gap#3 Anticipation & Longing Enhancement ──
    /// G2: 期待渐变预加载器 / G2: Anticipation progressive pre-loader
    anticipation_preloader: AnticipationPreLoader,
    /// G4: 跨会话想念累积存储 / G4: Cross-session longing accumulation store
    longing_accumulation_store: Option<LongingAccumulationStore>,
    // ── 成长管理器 / Maturity Manager ──
    maturity: parking_lot::Mutex<atrium_memory::maturity::MaturityManager>,
    // ── 内在独白引擎 / Inner Monologue Engine ──
    inner_monologue: parking_lot::Mutex<atrium_memory::inner_monologue::InnerMonologueEngine>,
    // ── 内心多元对话引擎 / Inner Dialogue Engine ──
    /// Gap#1 内心多元对话 — 理性者/感性者/怀疑者/梦想者四声音协商
    /// Gap#1 Inner dialogue — Rationalist/Emotionalist/Skeptic/Dreamer four-voice negotiation
    inner_dialogue: parking_lot::Mutex<InnerDialogueEngine>,
    // ── 数字日记 / Digital Diary ──
    diary_store: Option<atrium_memory::diary_store::DiaryStore>,
    /// 日记 markdown 输出目录 / Diary markdown output directory
    diary_dir: Option<String>,
    // ── 文件存储 / File Store ──
    // build() 中构造，运行时尚未读取 — 供未来文件存储扩展接入
    // 保留理由: 未来文件存储集成预留，当前确实未使用 / Retained: reserved for future file store integration
    #[allow(dead_code)]
    file_store: parking_lot::Mutex<Option<atrium_memory::file_store::FileStore>>,
    // ── 定时提醒 / Reminder System ──
    reminder_store: parking_lot::Mutex<Option<atrium_memory::reminder_store::ReminderStore>>,
    // ── 表达系统 / Expression System ──
    expression_enabled: bool,
    expression_cfg: crate::config::ExpressionCfg,
    // ── 追问引擎 / Follow-Up Engine ──
    followup_enabled: bool,
    followup: parking_lot::Mutex<atrium_memory::followup_tracker::FollowUpTracker>,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    followup_cfg: crate::config::FollowUpCfg,
    // ── Gap#6 好奇心追问增强 / Curiosity enhancement engines ──
    /// 好奇心子系统 / Curiosity subsystem
    curiosity: CuriositySubsystem,
    // 多事项编织器 — 多个追问事项编织为自然语言 / Multi-item weaver — weave multiple follow-ups into natural language
    multi_item_weaver: atrium_memory::multi_item_weaver::MultiItemWeaver,
    // curiosity_resonance + semantic_association 已合并入 curiosity 子系统
    // curiosity_resonance + semantic_association merged into curiosity subsystem
    // ── 叙事自我 / Narrative Self ──
    narrative_enabled: bool,
    narrative_cfg: crate::config::NarrativeCfg,
    /// 叙事自我子系统 / Narrative self subsystem
    narrative: NarrativeSubsystem,
    // ── 冲突与和解 / Conflict & Reconciliation ──
    conflict_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    conflict_cfg: crate::config::ConflictCfg,
    /// 冲突管理子系统（引擎+存储）/ Conflict subsystem (engine+store)
    conflict: Subsystem<
        atrium_memory::conflict_reconciliation::ConflictManager,
        atrium_memory::conflict_store::ConflictStore,
    >,
    /// 关系感知边界 / Relationship-aware boundary
    boundary:
        parking_lot::Mutex<atrium_memory::relationship_aware_boundary::RelationshipAwareBoundary>,
    // ── 情绪非理性 / Emotional Irrationality ──
    irrationality_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    irrationality_cfg: crate::config::IrrationalityCfg,
    /// 情绪非理性子系统（引擎+存储）/ Irrationality subsystem (engine+store)
    irrationality: Subsystem<
        atrium_memory::emotional_irrationality::IrrationalityManager,
        atrium_memory::irrationality_store::IrrationalityStore,
    >,
    // ── 共享仪式 / Shared Ritual ──
    ritual_enabled: bool,
    ritual_cfg: crate::config::RitualCfg,
    /// 仪式子系统 / Ritual subsystem
    ritual: RitualSubsystem,
    /// 仪式防抖写穿计数器：累积 N 条交互后批量持久化 / Ritual debounced write-through counter: batch persist after N interactions
    ritual_unsaved_count: AtomicU32,
    // ── 脆弱与不完美 / Vulnerability & Imperfection ──
    vulnerability_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    vulnerability_cfg: crate::config::VulnerabilityCfg,
    /// 脆弱子系统 / Vulnerability subsystem
    vulnerability: VulnerabilitySubsystem,
    // ── 情绪需求边界 / Emotional Demand Boundary ──
    emotional_demand_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    emotional_demand_cfg: crate::config::EmotionalDemandCfg,
    emotional_boundary:
        parking_lot::Mutex<atrium_memory::emotional_demand_boundary::EmotionalBoundary>,
    demand_boundary: parking_lot::Mutex<atrium_memory::emotional_demand_boundary::DemandBoundary>,
    // ── 自我关怀边界 / Self-Care Boundary ──
    self_care_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    self_care_cfg: crate::config::SelfCareCfg,
    self_care_boundary: parking_lot::Mutex<atrium_memory::self_care_boundary::SelfCareBoundary>,
    // ── 认知域保险库 / Cognitive Domain Vault ──
    /// 统一存储层 — 持有 4 个认知域 sled::Db 实例，维持 Tree 引用有效性
    /// Unified storage layer — owns 4 cognitive domain sled::Db instances, sustaining Tree reference validity
    // build() 中消费（各 Store 从 vault 派生），运行时 CoreService 不直接读取 — 保留以维持 Db 生命周期
    // 保留理由: 持有 AtriumVault 以维持底层 sled::Db 生命周期，删除会导致数据库句柄提前释放 / Retained: holds AtriumVault to sustain sled::Db lifetime
    #[allow(dead_code)]
    vault: Option<atrium_memory::atrium_vault::AtriumVault>,
    // ── 适度犯错 / Imperfection Engine ──
    imperfection_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    imperfection_cfg: crate::config::ImperfectionCfg,
    /// 犯错子系统（引擎+存储）/ Imperfection subsystem (engine+store)
    imperfection: Subsystem<
        atrium_memory::imperfection_engine::ImperfectionEngine,
        atrium_memory::imperfection_store::ImperfectionStore,
    >,
    // ── Gap#9 脆弱增强 / Vulnerability Enhancement ──
    // ── 风格记忆 / Style Memory ──
    /// 风格记忆子系统（偏移缓存+持久化存储）/ Style memory subsystem (offset cache + persistence store)
    style: Subsystem<StyleOffset, StyleMemoryStore>,
    // ── 物理存在感 / Physical Presence ──
    physical_presence_enabled: bool,
    // build() 中消费，运行时不直接读取 — 保留用于配置热重载探测
    // Consumed only in build(), not read at runtime — kept for config hot-reload detection
    physical_presence_cfg: crate::config::PhysicalPresenceCfg,
    /// 物理存在感子系统（引擎+存储）/ Physical presence subsystem (engine+store)
    physical_presence: Subsystem<
        atrium_memory::physical_presence::PhysicalPresenceEngine,
        atrium_memory::physical_presence_store::PhysicalPresenceStore,
    >,
    // ── Phase 3: 完全死亡模块通电 / Phase 3: Dead module power-on ──
    // Gap#1 独处内在世界 / Solitude inner world
    /// 独处子系统 / Solitude subsystem
    solitude: SolitudeSubsystem,
    // Gap#5 共享仪式补充 — 已合并入 ritual 子系统 / Ritual supplements — merged into ritual subsystem
    // Gap#9 脆弱与不完美补充 — 已合并入 vulnerability 子系统 / Vulnerability supplements — merged into vulnerability subsystem
    // Gap#4 冲突与和解 / Conflict and reconciliation
    /// 统一冲突引擎 / Unified conflict engine — 冲突成长 + 模式学习
    conflict_engine: parking_lot::Mutex<atrium_memory::conflict_engine::ConflictEngine>,
    // Gap#3 期待与想念 / Anticipation and longing
    /// 期待与想念子系统 / Longing subsystem
    longing: LongingSubsystem,
    // R3 通电：6个孤儿引擎接入运行时 / R3 power-on: 6 orphan engines into runtime
    /// 情绪气候引擎 / Emotional climate engine — 长周期情感生态调制
    emotional_climate: parking_lot::Mutex<atrium_memory::emotional_climate::EmotionalClimate>,
    /// 情绪固化引擎 / Emotional consolidation engine — 独处时情绪记忆沉淀
    emotional_consolidation:
        parking_lot::Mutex<atrium_memory::emotional_consolidation::EmotionalConsolidation>,
    /// 情绪耦合引擎 / Emotional coupling engine — 情绪间相互调制与涌现
    emotional_coupling: parking_lot::Mutex<atrium_memory::emotional_coupling::EmotionalCoupling>,
    /// 存在深度引擎 / Existential depth engine — 深夜存在性思考
    existential_depth: parking_lot::Mutex<atrium_memory::existential_depth::ExistentialDepth>,
    /// 内在议会 / Inner council — 多视角内心 deliberation
    inner_council: parking_lot::Mutex<atrium_memory::inner_council::InnerCouncil>,
    /// 仪式心跳引擎 / Ritual heartbeat engine — 仪式对情感基线的持续调制
    ritual_heartbeat: parking_lot::Mutex<atrium_memory::ritual_heartbeat::RitualHeartbeat>,
}

/// Room 输出消息（Python 网关通过 health 轮询消费）
#[derive(Debug, Clone)]
pub struct OutgoingRoomMessage {
    pub room_id: String,
    pub content: String,
    pub msg_type: String,
    pub capsule_name: String,
    pub ack_text: String,
}

impl Default for CoreService {
    fn default() -> Self {
        Self::new()
    }
}

// P1-1 叙事 LLM 类型别名 / P1-1 Narrative LLM type aliases

/// 章节生成候选项: (弧ID, 弧标题, 弧主题, 转折点叙述列表, 前一章摘要)
/// Chapter generation candidate: (arc_id, arc_title, arc_theme, tp_narratives, prev_summary)
impl CoreService {
    /// 每 N 条消息触发一次 reflection
    const REFLECTION_INTERVAL: u64 = 8;

    /// 配置快照 — 返回当前所有域配置的引用 / Config snapshot — returns references to all domain configs
    ///
    /// 使 CoreService 的 cfg 字段在运行时可读，支持配置热重载检测与诊断。
    /// 数字生命意义: 数字生命运行时能感知自身配置状态，如同生物体感知内分泌水平。
    /// Digital Life: runtime awareness of own configuration, like an organism sensing hormone levels.
    #[allow(clippy::type_complexity)]
    pub fn config_snapshot(
        &self,
    ) -> (
        &crate::config::FollowUpCfg,
        &crate::config::ConflictCfg,
        &crate::config::IrrationalityCfg,
        &crate::config::VulnerabilityCfg,
        &crate::config::EmotionalDemandCfg,
        &crate::config::SelfCareCfg,
        &crate::config::ImperfectionCfg,
        &crate::config::PhysicalPresenceCfg,
    ) {
        (
            &self.followup_cfg,
            &self.conflict_cfg,
            &self.irrationality_cfg,
            &self.vulnerability_cfg,
            &self.emotional_demand_cfg,
            &self.self_care_cfg,
            &self.imperfection_cfg,
            &self.physical_presence_cfg,
        )
    }

    pub fn new() -> Self {
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| {
            // 默认: CWD/data/atrium/, 解析为绝对路径
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/data/atrium", cwd.display())
        });
        std::fs::create_dir_all(&data_dir).ok();
        tracing::info!("Data dir: {}", data_dir);
        Self::build(
            Some(&data_dir),
            131_072,
            &crate::config::EmotionCfg::default(),
            &crate::config::UserModelCfg::default(),
            &crate::config::FeedbackCfg::default(),
            &crate::config::ProactiveCfg::default(),
            &crate::config::PerceptionCfg::default(),
            &crate::config::ConsolidationCfg::default(),
            &atrium_memory::empathy::EmpathyCfg::default(),
            &crate::config::AckLearningCfg::default(),
            &crate::config::LongingCfg::default(),
            &crate::config::MaturityCfg::default(),
            &crate::config::InnerMonologueCfg::default(),
            &crate::config::ExpressionCfg::default(),
            &crate::config::FollowUpCfg::default(),
            &crate::config::NarrativeCfg::default(),
            &crate::config::ConflictCfg::default(),
            &crate::config::IrrationalityCfg::default(),
            &crate::config::RitualCfg::default(),
            &crate::config::VulnerabilityCfg::default(),
            &crate::config::EmotionalDemandCfg::default(),
            &crate::config::SelfCareCfg::default(),
            &crate::config::ImperfectionCfg::default(),
            &crate::config::PhysicalPresenceCfg::default(),
        )
    }

    pub fn new_with_context(context_limit: usize) -> Self {
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| {
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/data/atrium", cwd.display())
        });
        std::fs::create_dir_all(&data_dir).ok();
        tracing::info!("Data dir: {} (context_limit={})", data_dir, context_limit);
        Self::build(
            Some(&data_dir),
            context_limit,
            &crate::config::EmotionCfg::default(),
            &crate::config::UserModelCfg::default(),
            &crate::config::FeedbackCfg::default(),
            &crate::config::ProactiveCfg::default(),
            &crate::config::PerceptionCfg::default(),
            &crate::config::ConsolidationCfg::default(),
            &atrium_memory::empathy::EmpathyCfg::default(),
            &crate::config::AckLearningCfg::default(),
            &crate::config::LongingCfg::default(),
            &crate::config::MaturityCfg::default(),
            &crate::config::InnerMonologueCfg::default(),
            &crate::config::ExpressionCfg::default(),
            &crate::config::FollowUpCfg::default(),
            &crate::config::NarrativeCfg::default(),
            &crate::config::ConflictCfg::default(),
            &crate::config::IrrationalityCfg::default(),
            &crate::config::RitualCfg::default(),
            &crate::config::VulnerabilityCfg::default(),
            &crate::config::EmotionalDemandCfg::default(),
            &crate::config::SelfCareCfg::default(),
            &crate::config::ImperfectionCfg::default(),
            &crate::config::PhysicalPresenceCfg::default(),
        )
    }

    /// 使用完整配置创建 CoreService（Scheduler 使用）
    /// Create CoreService with full configuration (used by Scheduler).
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_config(
        context_limit: usize,
        emotion_cfg: &crate::config::EmotionCfg,
        user_model_cfg: &crate::config::UserModelCfg,
        feedback_cfg: &crate::config::FeedbackCfg,
        proactive_cfg: &crate::config::ProactiveCfg,
        perception_cfg: &crate::config::PerceptionCfg,
        consolidation_cfg: &crate::config::ConsolidationCfg,
        empathy_cfg: &atrium_memory::empathy::EmpathyCfg,
        ack_learning_cfg: &crate::config::AckLearningCfg,
        longing_cfg: &crate::config::LongingCfg,
        maturity_cfg: &crate::config::MaturityCfg,
        inner_monologue_cfg: &crate::config::InnerMonologueCfg,
        expression_cfg: &crate::config::ExpressionCfg,
        followup_cfg: &crate::config::FollowUpCfg,
        narrative_cfg: &crate::config::NarrativeCfg,
        conflict_cfg: &crate::config::ConflictCfg,
        irrationality_cfg: &crate::config::IrrationalityCfg,
        ritual_cfg: &crate::config::RitualCfg,
        vulnerability_cfg: &crate::config::VulnerabilityCfg,
        emotional_demand_cfg: &crate::config::EmotionalDemandCfg,
        self_care_cfg: &crate::config::SelfCareCfg,
        imperfection_cfg: &crate::config::ImperfectionCfg,
        physical_presence_cfg: &crate::config::PhysicalPresenceCfg,
    ) -> Self {
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| {
            let cwd = std::env::current_dir().unwrap_or_default();
            format!("{}/data/atrium", cwd.display())
        });
        std::fs::create_dir_all(&data_dir).ok();
        tracing::info!("Data dir: {} (context_limit={})", data_dir, context_limit);
        Self::build(
            Some(&data_dir),
            context_limit,
            emotion_cfg,
            user_model_cfg,
            feedback_cfg,
            proactive_cfg,
            perception_cfg,
            consolidation_cfg,
            empathy_cfg,
            ack_learning_cfg,
            longing_cfg,
            maturity_cfg,
            inner_monologue_cfg,
            expression_cfg,
            followup_cfg,
            narrative_cfg,
            conflict_cfg,
            irrationality_cfg,
            ritual_cfg,
            vulnerability_cfg,
            emotional_demand_cfg,
            self_care_cfg,
            imperfection_cfg,
            physical_presence_cfg,
        )
    }

    /// 内存模式（用于测试）/ In-memory mode for testing.
    pub fn new_in_memory() -> Self {
        Self::build(
            None,
            131_072,
            &crate::config::EmotionCfg::default(),
            &crate::config::UserModelCfg::default(),
            &crate::config::FeedbackCfg::default(),
            &crate::config::ProactiveCfg::default(),
            &crate::config::PerceptionCfg::default(),
            &crate::config::ConsolidationCfg::default(),
            &atrium_memory::empathy::EmpathyCfg::default(),
            &crate::config::AckLearningCfg::default(),
            &crate::config::LongingCfg::default(),
            &crate::config::MaturityCfg::default(),
            &crate::config::InnerMonologueCfg::default(),
            &crate::config::ExpressionCfg::default(),
            &crate::config::FollowUpCfg::default(),
            &crate::config::NarrativeCfg::default(),
            &crate::config::ConflictCfg::default(),
            &crate::config::IrrationalityCfg::default(),
            &crate::config::RitualCfg::default(),
            &crate::config::VulnerabilityCfg::default(),
            &crate::config::EmotionalDemandCfg::default(),
            &crate::config::SelfCareCfg::default(),
            &crate::config::ImperfectionCfg::default(),
            &crate::config::PhysicalPresenceCfg::default(),
        )
    }

    /// 出厂内置 ACK：首次启动时写入，不覆盖用户已有文件
    fn seed_builtin_ack(canned_dir: &str) {
        let _ = std::fs::create_dir_all(canned_dir);
        let builtins: &[(&str, &str)] = &[
            (
                "atrium_architecture.ack",
                include_str!("../../../../builtin_canned/atrium_architecture.ack"),
            ),
            (
                "ack_guide.ack",
                include_str!("../../../../builtin_canned/ack_guide.ack"),
            ),
            (
                "experiment_log_policy.ack",
                include_str!("../../../../builtin_canned/experiment_log_policy.ack"),
            ),
            (
                "qq_chat_guide.ack",
                include_str!("../../../../builtin_canned/qq_chat_guide.ack"),
            ),
        ];
        for (filename, content) in builtins {
            let path = std::path::Path::new(canned_dir).join(filename);
            if !path.exists() {
                if let Err(e) = std::fs::write(&path, content) {
                    tracing::warn!("写入内置 ACK {} 失败: {}", path.display(), e);
                } else {
                    tracing::info!("写入内置 ACK: {}", path.display());
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        data_dir: Option<&str>,
        context_limit: usize,
        emotion_cfg: &crate::config::EmotionCfg,
        user_model_cfg: &crate::config::UserModelCfg,
        feedback_cfg: &crate::config::FeedbackCfg,
        proactive_cfg: &crate::config::ProactiveCfg,
        perception_cfg: &crate::config::PerceptionCfg,
        consolidation_cfg: &crate::config::ConsolidationCfg,
        empathy_cfg: &atrium_memory::empathy::EmpathyCfg,
        ack_learning_cfg: &crate::config::AckLearningCfg,
        longing_cfg: &crate::config::LongingCfg,
        maturity_cfg: &crate::config::MaturityCfg,
        inner_monologue_cfg: &crate::config::InnerMonologueCfg,
        expression_cfg: &crate::config::ExpressionCfg,
        followup_cfg: &crate::config::FollowUpCfg,
        narrative_cfg: &crate::config::NarrativeCfg,
        conflict_cfg: &crate::config::ConflictCfg,
        irrationality_cfg: &crate::config::IrrationalityCfg,
        ritual_cfg: &crate::config::RitualCfg,
        vulnerability_cfg: &crate::config::VulnerabilityCfg,
        emotional_demand_cfg: &crate::config::EmotionalDemandCfg,
        self_care_cfg: &crate::config::SelfCareCfg,
        imperfection_cfg: &crate::config::ImperfectionCfg,
        physical_presence_cfg: &crate::config::PhysicalPresenceCfg,
    ) -> Self {
        let persist = data_dir.is_some();
        let dir = data_dir.unwrap_or("");

        // ── 认知域保险库 / Cognitive Domain Vault ──
        // 统一存储层：4 个认知域替代 16 个独立 sled 实例
        // Unified storage: 4 cognitive domains replace 16 independent sled instances
        // 惰性迁移：首次启动时检测旧目录结构，自动迁移到新 Vault 布局
        // Lazy migration: on first start, detect old directory layout and migrate to new Vault layout
        if persist && atrium_memory::atrium_vault::AtriumVault::needs_migration(dir) {
            tracing::info!(
                "AtriumVault: detecting legacy store directories, starting migration..."
            );
            match atrium_memory::atrium_vault::AtriumVault::migrate_from_legacy(dir) {
                Ok(report) => tracing::info!("AtriumVault: migration complete — {}", report),
                Err(e) => tracing::warn!(
                    "AtriumVault: migration failed (stores will start empty) — {}",
                    e
                ),
            }
        }
        let vault = if persist {
            atrium_memory::atrium_vault::AtriumVault::open(dir).ok()
        } else {
            None
        };

        let stm = StmBuffer::new(100);
        let ltm = if persist {
            SledLtm::open(&format!("{}/ltm", dir)).unwrap_or_else(|_| SledLtm::open_in_memory())
        } else {
            SledLtm::open_in_memory()
        };

        // 情感引擎初始化 — 从情感中枢加载快照 / Emotion engine init — load snapshot from limbic vault
        let emotion = {
            let snap = vault.as_ref().and_then(|v| {
                atrium_memory::emotion_store::EmotionStore::open(v.limbic())
                    .ok()
                    .and_then(|s| s.load_snapshot().ok().flatten())
            });
            Self::build_emotion_engine(emotion_cfg, snap.as_ref(), longing_cfg)
        };
        let mut persona = PersonaManager::new(atrium_persona::loader::PersonaLoader::new());
        persona.register(atrium_persona::manager::default_persona_def());
        let fts5 = if persist {
            Fts5Index::open(&format!("{}/fts5.db", dir))
                .unwrap_or_else(|_| Fts5Index::open(":memory:").expect("fts5 in-memory init"))
        } else {
            Fts5Index::open(":memory:").expect("fts5 in-memory init")
        };
        let evidence = EvidenceScorer::default();

        let fact_store = if persist {
            FactStore::new(&format!("{}/facts", dir))
                .unwrap_or_else(|_| FactStore::new("").expect("fact_store in-memory init"))
        } else {
            FactStore::new("").expect("fact_store in-memory init")
        };

        let key_facts = if persist {
            KeyFactCache::open(&format!("{}/key_facts", dir))
                .unwrap_or_else(|_| KeyFactCache::new_in_memory())
        } else {
            KeyFactCache::new_in_memory()
        };

        // ── 认知域 Store 初始化 / Cognitive Domain Store Initialization ──
        // 叙事自我 → 叙事皮层 / Narrative self → Narrative cortex
        let narrative_store = vault.as_ref().and_then(|v| {
            atrium_memory::narrative_store::NarrativeSelfStore::open(v.narrative()).ok()
        });
        // 冲突与和解 → 关系海马体 / Conflict & reconciliation → Relational hippocampus
        let conflict_store = vault
            .as_ref()
            .and_then(|v| atrium_memory::conflict_store::ConflictStore::open(v.relational()).ok());
        // 情绪非理性 → 情感中枢 / Irrationality → Limbic system
        let irrationality_store = vault.as_ref().and_then(|v| {
            atrium_memory::irrationality_store::IrrationalityStore::open(v.limbic()).ok()
        });
        // 共享仪式 → 关系海马体 / Ritual → Relational hippocampus
        let ritual_store = vault
            .as_ref()
            .and_then(|v| atrium_memory::ritual_store::RitualStore::open(v.relational()).ok());
        // 脆弱窗口 → 关系海马体 / Vulnerability → Relational hippocampus
        let vulnerability_store = vault.as_ref().and_then(|v| {
            atrium_memory::vulnerability_store::VulnerabilityStore::open(v.relational()).ok()
        });
        // 适度犯错 → 情感中枢 / Imperfection → Limbic system
        let imperfection_store = vault.as_ref().and_then(|v| {
            atrium_memory::imperfection_store::ImperfectionStore::open(v.limbic()).ok()
        });
        // 物理存在感 → 情感中枢 / Physical presence → Limbic system
        let physical_presence_store = vault.as_ref().and_then(|v| {
            atrium_memory::physical_presence_store::PhysicalPresenceStore::open(v.limbic()).ok()
        });
        // 风格记忆 → 情感中枢 / Style memory → Limbic system
        let style_memory_store = vault
            .as_ref()
            .and_then(|v| StyleMemoryStore::open(v.limbic()).ok());
        // 用户心智模型 → 关系海马体 / User mental model → Relational hippocampus
        let user_model_store = vault
            .as_ref()
            .and_then(|v| UserMentalModelStore::open(v.relational()).ok());

        let history = if persist {
            ConversationHistory::open(&format!("{}/conversations", dir)).unwrap_or_else(|_| {
                ConversationHistory::open("./data/conversations_fallback")
                    .expect("history fallback init")
            })
        } else {
            ConversationHistory::open_in_memory()
        };

        // 罐装知识管理器：先播种内置 ACK，再扫描加载全部
        let canned_dir = if persist {
            format!("{}/../canned", dir)
        } else {
            "~/.atrium/canned".into()
        };
        Self::seed_builtin_ack(&canned_dir);
        let mut canned = CannedManager::new(&canned_dir);
        let loaded = canned.scan();
        tracing::info!(
            "CannedManager: scanned {} ACK files in {}",
            loaded,
            canned_dir
        );

        let mut memory = MemoryManager::new(stm, ltm);

        // STM 热启动：从 ConversationHistory 恢复最近对话上下文
        if persist {
            let recent_msgs = history.recent_messages(50);
            if !recent_msgs.is_empty() {
                for m in &recent_msgs {
                    let _ = memory.remember(
                        MemoryEntry::new(&m.role, MemoryContent::Text(m.content.clone()))
                            .with_importance(0.3),
                    );
                }
                tracing::info!(
                    "STM warm-started with {} messages from history",
                    recent_msgs.len()
                );
            }
        }

        // ── 关联记忆图初始化──
        let graph_store = if persist {
            GraphStore::new(&format!("{}/graph", dir))
                .unwrap_or_else(|_| GraphStore::new_in_memory())
        } else {
            GraphStore::new_in_memory()
        };

        let mut graph = AssociativeGraph::new();
        match graph_store.load() {
            Ok(Some(loaded)) => {
                let stats = loaded.stats();
                graph = loaded;
                tracing::info!(
                    "关联记忆图: 从持久化恢复 {} 节点, {} 边",
                    stats.node_count,
                    stats.edge_count
                );
            }
            _ => {
                // 从 FactStore 全量构建（冷启动）
                let facts = fact_store.query_by_subject("主人").unwrap_or_default();
                if !facts.is_empty() {
                    graph.build(&facts);
                    tracing::info!(
                        "关联记忆图: 从 {} 条事实冷启动, {} 节点, {} 边",
                        facts.len(),
                        graph.node_count(),
                        graph.edge_count()
                    );
                }
            }
        }

        // ── 仪式系统启动恢复 / Ritual systems startup recovery ──
        let (ritual_detector_init, mut anniversary_init, seasonal_init) =
            if let Some(ref store) = ritual_store {
                if let Ok(snapshot) = store.load() {
                    tracing::info!("[Ritual] Restored from sled persistence");
                    (
                        snapshot.ritual_detector,
                        snapshot.anniversary_system,
                        snapshot.seasonal_awareness,
                    )
                } else {
                    (
                        atrium_memory::ritual_detector::RitualDetector::default_new(),
                        atrium_memory::anniversary_system::AnniversarySystem::new_with_config(
                            ritual_cfg.anniversary_remind_days,
                        ),
                        atrium_memory::seasonal_awareness::SeasonalAwareness::new(),
                    )
                }
            } else {
                (
                    atrium_memory::ritual_detector::RitualDetector::default_new(),
                    atrium_memory::anniversary_system::AnniversarySystem::new_with_config(
                        ritual_cfg.anniversary_remind_days,
                    ),
                    atrium_memory::seasonal_awareness::SeasonalAwareness::new(),
                )
            };
        // 用当前配置覆盖持久化的提醒天数 / Override persisted remind_days with current config
        anniversary_init.update_remind_days(ritual_cfg.anniversary_remind_days);

        // ── 脆弱窗口启动恢复 / Vulnerability window startup recovery ──
        let vulnerability_init = if let Some(ref store) = vulnerability_store {
            if let Ok(window) = store.load() {
                tracing::info!("[Vulnerability] Restored from sled persistence");
                window
            } else {
                atrium_memory::vulnerability_window::VulnerabilityWindow::new(
                    atrium_memory::vulnerability_window::VulnerabilityConfig {
                        max_per_n_conversations: vulnerability_cfg.max_per_n_conversations,
                        prompt_budget: vulnerability_cfg.prompt_budget,
                        ..Default::default()
                    },
                )
            }
        } else {
            atrium_memory::vulnerability_window::VulnerabilityWindow::new(
                atrium_memory::vulnerability_window::VulnerabilityConfig {
                    max_per_n_conversations: vulnerability_cfg.max_per_n_conversations,
                    prompt_budget: vulnerability_cfg.prompt_budget,
                    ..Default::default()
                },
            )
        };

        Self {
            memory: parking_lot::Mutex::new(memory),
            emotion: parking_lot::Mutex::new(emotion),
            persona: parking_lot::Mutex::new(persona),
            fact_store,
            evidence,
            fts5: parking_lot::Mutex::new(fts5),
            reflection: parking_lot::Mutex::new(if persist {
                ReflectionEngine::open(&format!("{}/reflections", dir))
            } else {
                ReflectionEngine::new()
            }),
            runtime_persona: parking_lot::Mutex::new(if persist {
                RuntimePersonaManager::open(&format!("{}/runtime_persona", dir))
            } else {
                RuntimePersonaManager::new()
            }),
            message_count: AtomicU64::new(0),
            last_reflection_at: AtomicU64::new(0),
            token_budget: parking_lot::Mutex::new(TokenBudget::new(context_limit)),
            summarizer: parking_lot::Mutex::new(if persist {
                ConversationSummarizer::open(
                    &format!("{}/summaries", dir),
                    SummaryConfig::default(),
                )
            } else {
                ConversationSummarizer::new(SummaryConfig::default())
            }),
            key_facts,
            guard: parking_lot::Mutex::new(PersonaGuard::new("Atrium", "主人")),
            preferences: parking_lot::Mutex::new(if persist {
                PreferenceManager::open(&format!("{}/preferences", dir))
            } else {
                PreferenceManager::new()
            }),
            replay: parking_lot::Mutex::new(ReplayPipeline::new().with_interval(300)),
            rules: parking_lot::Mutex::new({
                let mut e = if persist {
                    RuleEngine::open(&format!("{}/rules", dir))
                        .unwrap_or_else(|_| RuleEngine::new())
                } else {
                    RuleEngine::new()
                };
                e.register_defaults();
                e
            }),
            history,
            started_at: Instant::now(),
            canned: parking_lot::Mutex::new(canned),
            llm_client: parking_lot::Mutex::new(None),
            room: parking_lot::Mutex::new(crate::room::RoomEngine::new(
                crate::config::RoomCfg::default(),
            )),
            room_outgoing: parking_lot::Mutex::new(std::collections::VecDeque::new()),
            pending_room_trigger: parking_lot::Mutex::new(None),
            relationship: parking_lot::Mutex::new(if persist {
                RelationshipManager::open(dir).unwrap_or_else(|_| RelationshipManager::new())
            } else {
                RelationshipManager::new()
            }),
            user_model: parking_lot::Mutex::new({
                // 从 sled 恢复用户心智模型 / Restore user mental model from sled
                let mut model = UserMentalModel::with_config(
                    user_model_cfg.mood_ema_alpha,
                    user_model_cfg.style_ema_alpha,
                    user_model_cfg.topic_decay_hours,
                );
                if let Some(ref store) = user_model_store {
                    if let Ok(restored) = store.load("default") {
                        model = restored;
                        tracing::info!("[UserModel] Restored from sled persistence");
                    }
                }
                model
            }),
            user_model_store,
            user_model_unsaved_count: AtomicU32::new(0),
            feedback: parking_lot::Mutex::new(FeedbackLoop::with_config(
                feedback_cfg.satisfaction_ema_alpha,
                feedback_cfg.signal_window,
            )),
            proactive: parking_lot::Mutex::new(ProactiveEngine::new(proactive_cfg)),
            graph: parking_lot::Mutex::new(graph),
            graph_store,
            graph_dirty: AtomicBool::new(false),
            last_graph_save_at: AtomicU64::new(0),
            typing_analyzer: parking_lot::Mutex::new(TypingRhythmAnalyzer::new(
                perception_cfg.typing.baseline_learning_rate,
                perception_cfg.typing.rhythm_analysis_window,
            )),
            perception_enabled: perception_cfg.typing.enabled,
            compound_enabled: emotion_cfg.compound.enabled,
            // 情感持久化 → 情感中枢 / Emotion persistence → Limbic system
            emotion_store: vault
                .as_ref()
                .and_then(|v| atrium_memory::emotion_store::EmotionStore::open(v.limbic()).ok()),
            consolidator: parking_lot::Mutex::new(MemoryConsolidator::new(
                ConsolidationConfig::new(
                    consolidation_cfg.enabled,
                    consolidation_cfg.max_facts_per_run,
                    consolidation_cfg.min_interval_hours,
                    consolidation_cfg.similarity_threshold,
                    consolidation_cfg.low_access_age_days,
                ),
            )),
            empathy: parking_lot::Mutex::new(EmpathyEngine::new(empathy_cfg.clone())),
            ack_learning_cfg: ack_learning_cfg.clone(),
            teach_detected: parking_lot::Mutex::new(None),
            // 期待事件 → 关系海马体 / Anticipation → Relational hippocampus
            anticipation_store: if longing_cfg.anticipation.enabled {
                vault.as_ref().and_then(|v| {
                    atrium_memory::anticipation_store::AnticipationStore::open(v.relational()).ok()
                })
            } else {
                None
            },
            longing_cfg: longing_cfg.clone(),
            // ── Gap#3 期待与想念增强 / Gap#3 Anticipation & Longing Enhancement ──
            anticipation_preloader: AnticipationPreLoader::default(),
            // G4: 想念累积 → 关系海马体 / G4: Longing accumulation → Relational hippocampus
            longing_accumulation_store: vault
                .as_ref()
                .and_then(|v| LongingAccumulationStore::open_default(v.relational()).ok()),
            maturity: parking_lot::Mutex::new(if maturity_cfg.enabled && persist {
                atrium_memory::maturity::MaturityManager::open(
                    &format!("{}/maturity", dir),
                    atrium_memory::maturity::MaturityThresholds::default(),
                )
            } else {
                atrium_memory::maturity::MaturityManager::new(
                    atrium_memory::maturity::MaturityThresholds::default(),
                )
            }),
            // ── 内在独白引擎 / Inner Monologue Engine ──
            inner_monologue: parking_lot::Mutex::new({
                let im_config = atrium_memory::inner_monologue::InnerMonologueConfig {
                    max_thoughts: inner_monologue_cfg.max_thoughts,
                    max_per_day: inner_monologue_cfg.max_per_day,
                    graph_wander_interval_secs: inner_monologue_cfg.graph_wander_interval_secs,
                    graph_wander_max_per_day: inner_monologue_cfg.graph_wander_max_per_day,
                    graph_wander_decay_rate: 0.6,
                    graph_wander_max_hops: 3,
                    learning_interval_secs: inner_monologue_cfg.learning_interval_secs,
                    learning_max_per_day: inner_monologue_cfg.learning_max_per_day,
                    daydream_interval_secs: inner_monologue_cfg.daydream_interval_secs,
                    daydream_confidence: inner_monologue_cfg.daydream_confidence,
                    // G1-G5: 独处内在世界增强(默认全启用) / Solitude inner world enhancement (all enabled by default)
                    emotion_driven_mode: true,
                    solitude_depth_enabled: true,
                    solitude_bridge_enabled: true,
                    solitude_atmosphere_enabled: true,
                    emotion_resonant_seed: true,
                };
                atrium_memory::inner_monologue::InnerMonologueEngine::new(im_config)
            }),
            // ── 内心多元对话引擎 / Inner Dialogue Engine ──
            // 数字生命的内心不是单一声音，而是多个自我视角的对话
            // Digital life's inner world is not a single voice, but a dialogue among multiple selves
            inner_dialogue: parking_lot::Mutex::new(InnerDialogueEngine::default()),
            // ── 数字日记 / Digital Diary ──
            // 数字日记 → 叙事皮层 / Digital diary → Narrative cortex
            diary_store: if inner_monologue_cfg.enabled {
                vault
                    .as_ref()
                    .and_then(|v| atrium_memory::diary_store::DiaryStore::open(v.narrative()).ok())
            } else {
                None
            },
            diary_dir: if inner_monologue_cfg.enabled && persist {
                Some(format!("{}/diary", dir))
            } else {
                None
            },
            expression_enabled: expression_cfg.enabled,
            expression_cfg: expression_cfg.clone(),
            followup_enabled: followup_cfg.enabled,
            // ── 文件存储 / File Store ──
            // 文件存储 → 前额工具区 / File store → Prefrontal utility
            file_store: parking_lot::Mutex::new(vault.as_ref().and_then(|v| {
                atrium_memory::file_store::FileStore::open(v.prefrontal(), dir).ok()
            })),
            // ── 定时提醒 / Reminder System ──
            // 定时提醒 → 前额工具区 / Reminder → Prefrontal utility
            reminder_store: parking_lot::Mutex::new(vault.as_ref().and_then(|v| {
                atrium_memory::reminder_store::ReminderStore::open(v.prefrontal()).ok()
            })),
            followup: parking_lot::Mutex::new({
                let store = if persist {
                    atrium_memory::followup_tracker::FollowUpStore::open(&format!(
                        "{}/followup",
                        dir
                    ))
                    .unwrap_or_else(|_| {
                        atrium_memory::followup_tracker::FollowUpStore::open_in_memory()
                            .expect("followup in-memory init")
                    })
                } else {
                    atrium_memory::followup_tracker::FollowUpStore::open_in_memory()
                        .expect("followup in-memory init")
                };
                let recall_config = atrium_memory::followup_tracker::RecallConfig::default();
                let trigger_config = atrium_memory::followup_tracker::TriggerConfig {
                    max_per_day: followup_cfg.max_per_day,
                    min_interval_secs: followup_cfg.min_interval_secs as i64,
                    trigger_threshold: followup_cfg.trigger_threshold,
                    min_weight_threshold: followup_cfg.min_weight_threshold,
                    time_weight: followup_cfg.time_weight,
                    topic_weight: followup_cfg.topic_weight,
                    emotion_weight: followup_cfg.emotion_weight,
                };
                atrium_memory::followup_tracker::FollowUpTracker::new(
                    store,
                    recall_config,
                    trigger_config,
                )
            }),
            followup_cfg: followup_cfg.clone(),
            // ── Gap#6 好奇心追问增强引擎初始化 / Curiosity enhancement engines init ──
            curiosity: CuriositySubsystem::new(
                atrium_memory::curiosity_drive::CuriosityDrive::default_new(),
                atrium_memory::followup_style_learner::FollowUpStyleLearner::default_new(),
                atrium_memory::curiosity_resonance::CuriosityResonance::default_new(),
                atrium_memory::semantic_association::SemanticAssociation::default_new(),
            ),
            multi_item_weaver: atrium_memory::multi_item_weaver::MultiItemWeaver::default_new(),
            narrative_enabled: narrative_cfg.enabled,
            narrative_cfg: narrative_cfg.clone(),
            narrative: NarrativeSubsystem::new(
                {
                    // 从 sled 恢复叙事自我状态 / Restore narrative self from sled
                    let mut model = atrium_memory::life_narrative::NarrativeSelf::new();
                    if let Some(ref store) = narrative_store {
                        if let Ok(restored) = store.load() {
                            model = restored;
                            tracing::info!("[叙事/Narrative] Restored from sled persistence");
                        }
                    }
                    model
                },
                atrium_memory::life_narrative::TurningPointDetector::new(
                    atrium_memory::life_narrative::TurningPointConfig {
                        emotion_change_threshold: narrative_cfg.emotion_change_threshold,
                        min_interval_secs: narrative_cfg.min_interval_secs,
                        ..Default::default()
                    },
                ),
                atrium_memory::life_narrative::ArcDetector::default_new(),
                atrium_memory::life_narrative::PromptWeaver::default_new(),
                atrium_memory::life_narrative::ChapterWriter::default_new(),
                atrium_memory::life_narrative::ThemeWeaver::new(),
                atrium_memory::life_narrative::VoiceModulator::default_new(),
                narrative_store,
            ),
            conflict_enabled: conflict_cfg.enabled,
            conflict_cfg: conflict_cfg.clone(),
            conflict: Subsystem::from_parts(
                parking_lot::Mutex::new({
                    // 构建冲突管理器内部配置 / Build conflict manager internal config
                    pub(crate) use atrium_memory::conflict_reconciliation::{
                        ConflictConfig, ConflictPadBridge, EscalationConfig, ProactiveReconcilerConfig,
                        ReconciliationConfig, RecoveryCurve,
                    };
                    let conflict_inner = ConflictConfig {
                        disagreement_sensitivity: conflict_cfg.disagreement_sensitivity,
                        over_demand_window: conflict_cfg.over_demand_window,
                        over_demand_threshold: conflict_cfg.over_demand_threshold,
                        escalation: EscalationConfig {
                            cooldown_turns: conflict_cfg.escalation_cooldown_turns,
                            consecutive_threshold: conflict_cfg.consecutive_threshold,
                            max_allowed:
                                atrium_memory::conflict_reconciliation::ConflictIntensity::Severe,
                            de_escalation_turns: conflict_cfg.de_escalation_turns,
                        },
                        reconciliation: ReconciliationConfig::default(),
                    };
                    // 从 sled 恢复冲突状态 / Restore conflict state from sled
                    let mut mgr =
                        atrium_memory::conflict_reconciliation::ConflictManager::new(conflict_inner);

                    // G1: 主动和解管线配置 / G1: Proactive reconciler config
                    mgr.proactive_reconciler =
                        atrium_memory::conflict_reconciliation::ProactiveReconciler::new(
                            ProactiveReconcilerConfig {
                                unresolved_threshold_turns: conflict_cfg.proactive_threshold_turns,
                                time_since_conflict_secs: conflict_cfg.proactive_time_secs,
                                max_proactive_per_session: conflict_cfg.proactive_max_per_session,
                            },
                        );

                    // G2: 冲突↔情绪PAD桥接配置 / G2: Conflict↔emotion PAD bridge config
                    mgr.pad_bridge = ConflictPadBridge {
                        pleasure_decay: conflict_cfg.pleasure_decay,
                        arousal_boost: conflict_cfg.arousal_boost,
                        dominance_decay: conflict_cfg.dominance_decay,
                        ..Default::default()
                    };

                    // G4: 恢复曲线配置 / G4: Recovery curve config
                    mgr.recovery_curve = RecoveryCurve::new(
                        conflict_cfg.base_recovery_rate,
                        conflict_cfg.conflict_decay_rate,
                    );

                    // 从 sled 恢复冲突状态（覆盖默认配置）/ Restore from sled (overrides defaults)
                    if let Some(ref store) = conflict_store {
                        if let Ok(restored) = store.load() {
                            mgr = restored;
                            tracing::info!("[Conflict] Restored from sled persistence");
                        }
                    }
                    mgr
                }),
                conflict_store.map(parking_lot::Mutex::new),
            ),
            boundary: parking_lot::Mutex::new(
                atrium_memory::relationship_aware_boundary::RelationshipAwareBoundary::default(),
            ),
            // ── 情绪非理性 / Emotional Irrationality ──
            irrationality_enabled: irrationality_cfg.enabled,
            irrationality_cfg: irrationality_cfg.clone(),
            irrationality: Subsystem::from_parts(
                parking_lot::Mutex::new({
                    pub(crate) use atrium_memory::emotional_irrationality::{
                        ChaosConfig, ChaosParams, ContagionConfig, IrrationalityConfig,
                        IrrationalityManager, PulseConfig, ResidueConfig,
                    };
                    let irr_config = IrrationalityConfig {
                        pulse: PulseConfig {
                            min_pad_change: irrationality_cfg.pulse_min_pad_change,
                            max_active_pulses: irrationality_cfg.pulse_max_active,
                            uncaused_prob: irrationality_cfg.pulse_uncaused_prob,
                            uncaused_max_intensity: 0.3,
                            rebound_window_secs: 60,
                        },
                        residue: ResidueConfig {
                            max_active_residues: irrationality_cfg.residue_max_active,
                            min_retained_intensity: 0.05,
                        },
                        contagion: ContagionConfig {
                            max_chain_depth: 3,
                            cooldown_secs: irrationality_cfg.contagion_cooldown_secs,
                        },
                        chaos: ChaosConfig {
                            max_trajectory_len: irrationality_cfg.chaos_max_trajectory,
                            bifurcation_window_secs: 600,
                            min_cycle_secs: 120,
                        },
                        chaos_params: ChaosParams {
                            pulse_sensitivity: 0.5,
                            contagion_activity: 0.3,
                            residue_persistence: 0.7,
                            emergence_threshold: irrationality_cfg.chaos_emergence_threshold,
                            uncaused_fluctuation_prob: 0.02,
                        },
                        enabled: irrationality_cfg.enabled,
                        prompt_budget: irrationality_cfg.prompt_budget,
                    };
                    // 从 sled 恢复非理性状态 / Restore irrationality from sled
                    let mut mgr = IrrationalityManager::new(irr_config);
                    if let Some(ref store) = irrationality_store {
                        if let Ok(restored) = store.load() {
                            mgr = restored;
                            tracing::info!("[Irrationality] Restored from sled persistence");
                        }
                    }
                    mgr
                }),
                irrationality_store.map(parking_lot::Mutex::new),
            ),
            ritual_enabled: ritual_cfg.enabled,
            ritual_cfg: ritual_cfg.clone(),
            ritual: RitualSubsystem::new(
                ritual_detector_init,
                anniversary_init,
                seasonal_init,
                atrium_memory::adaptive_ritual::AdaptiveRitualDiscovery::new(),
                atrium_memory::ritual_evolution::RitualEvolution::new(),
                atrium_memory::ritual_absence::RitualAbsence::new("daily_chat", 0, 86400),
                atrium_memory::ritual_emergence::RitualEmergence::new(),
                atrium_memory::ritual_resonance::RitualResonanceEngine::new(),
                atrium_memory::ritual_anticipation::RitualAnticipation::new(),
                ritual_store,
            ),
            // 仪式防抖写穿计数器初始化 / Ritual debounced write-through counter init
            ritual_unsaved_count: AtomicU32::new(0),
            vulnerability_enabled: vulnerability_cfg.enabled,
            vulnerability_cfg: vulnerability_cfg.clone(),
            vulnerability: VulnerabilitySubsystem::new(
                vulnerability_init,
                atrium_memory::vulnerability_resonance::VulnerabilityResonance::default_new(),
                atrium_memory::vulnerability_wisdom::VulnerabilityWisdom::default_new(),
                atrium_memory::imperfection_vulnerability_bridge::ImperfectionVulnerabilityBridge::default_new(),
                atrium_memory::authentic_expression_modulator::AuthenticExpressionModulator::default_new(),
                atrium_memory::vulnerability_ritual::VulnerabilityRitual::new(),
                atrium_memory::imperfection_warmth::ImperfectionWarmth::new(),
                atrium_memory::authentic_imperfection::AuthenticImperfection::new(),
                vulnerability_store,
            ),
            // ── 认知域保险库 / Cognitive Domain Vault ──
            vault,
            emotional_demand_enabled: emotional_demand_cfg.enabled,
            emotional_demand_cfg: emotional_demand_cfg.clone(),
            emotional_boundary: parking_lot::Mutex::new(
                atrium_memory::emotional_demand_boundary::EmotionalBoundary::new(
                    atrium_memory::emotional_demand_boundary::EmotionalBoundaryConfig::default(),
                ),
            ),
            demand_boundary: parking_lot::Mutex::new(
                atrium_memory::emotional_demand_boundary::DemandBoundary::new(
                    atrium_memory::emotional_demand_boundary::DemandBoundaryConfig::default(),
                ),
            ),
            // ── 自我关怀边界 / Self-Care Boundary ──
            self_care_enabled: self_care_cfg.enabled,
            self_care_cfg: self_care_cfg.clone(),
            self_care_boundary: parking_lot::Mutex::new(
                atrium_memory::self_care_boundary::SelfCareBoundary::new(
                    atrium_memory::self_care_boundary::SelfCareConfig::default(),
                ),
            ),
            // ── 适度犯错 / Imperfection Engine ──
            imperfection_enabled: imperfection_cfg.enabled,
            imperfection_cfg: imperfection_cfg.clone(),
            imperfection: Subsystem::from_parts(
                parking_lot::Mutex::new({
                    use atrium_memory::imperfection_engine::ImperfectionConfig;
                    let imp_config = ImperfectionConfig {
                        enabled: imperfection_cfg.enabled,
                        base_prob: imperfection_cfg.base_prob,
                        max_prob: imperfection_cfg.max_prob,
                        cognitive_load_threshold: imperfection_cfg.cognitive_load_threshold,
                        fatigue_threshold: imperfection_cfg.fatigue_threshold,
                        unfamiliar_threshold: imperfection_cfg.unfamiliar_threshold,
                        emotion_activation_floor: imperfection_cfg.emotion_activation_floor,
                        relationship_gate_min: imperfection_cfg.relationship_gate_min,
                        maturity_gate_min: imperfection_cfg.maturity_gate_min,
                        correction_delay_min_secs: imperfection_cfg.correction_delay_min_secs,
                        correction_delay_max_secs: imperfection_cfg.correction_delay_max_secs,
                        max_mistakes_per_turn: imperfection_cfg.max_mistakes_per_turn,
                        cooldown_secs: imperfection_cfg.cooldown_secs,
                        clean_streak_decay: imperfection_cfg.clean_streak_decay,
                    };
                    // 从 sled 恢复犯错引擎状态 / Restore imperfection engine from sled
                    let mut engine =
                        atrium_memory::imperfection_engine::ImperfectionEngine::new(imp_config);
                    if let Some(ref store) = imperfection_store {
                        if let Ok(restored) = store.load() {
                            engine = restored;
                            tracing::info!("[Imperfection] Restored from sled persistence");
                        }
                    }
                    engine
                }),
                imperfection_store.map(parking_lot::Mutex::new),
            ),
            // ── Gap#9 脆弱增强引擎已合并入 vulnerability 子系统 / Vulnerability enhancement merged into vulnerability subsystem
            // ── 风格记忆 / Style Memory ──
            style: Subsystem::from_parts(
                parking_lot::Mutex::new(StyleOffset::zero()),
                style_memory_store.map(parking_lot::Mutex::new),
            ),
            // ── 物理存在感 / Physical Presence ──
            physical_presence_enabled: physical_presence_cfg.enabled,
            physical_presence_cfg: physical_presence_cfg.clone(),
            physical_presence: Subsystem::from_parts(
                parking_lot::Mutex::new({
                    use atrium_memory::physical_presence::PhysicalPresenceConfig as PpConfig;
                    let pp_config = PpConfig {
                        enabled: physical_presence_cfg.enabled,
                        fatigue_half_life_secs: physical_presence_cfg.fatigue_half_life_secs,
                        circadian_enabled: physical_presence_cfg.circadian_enabled,
                        interaction_fatigue_enabled: physical_presence_cfg.interaction_fatigue_enabled,
                        body_to_emotion_enabled: physical_presence_cfg.body_to_emotion_enabled,
                        prompt_budget: physical_presence_cfg.prompt_budget,
                        signature_ema_alpha: 0.01,
                    };
                    // 从 sled 恢复物理存在感状态 / Restore physical presence from sled
                    let mut engine =
                        atrium_memory::physical_presence::PhysicalPresenceEngine::new(pp_config);
                    if let Some(ref store) = physical_presence_store {
                        if let Ok(restored) = store.load() {
                            engine = restored;
                            tracing::info!("[PhysicalPresence] Restored from sled persistence");
                        }
                    }
                    engine
                }),
                physical_presence_store.map(parking_lot::Mutex::new),
            ),
            // ── Phase 3: 完全死亡模块初始化 / Phase 3: Dead module init ──
            // Gap#1 独处内在世界 / Solitude inner world
            solitude: SolitudeSubsystem::new(
                atrium_memory::personality_drift::PersonalityDrift::new(),
                atrium_memory::solitude_archetype::ArchetypeTracker::new(),
                atrium_memory::solitude_creativity::SolitudeCreativity::new(),
                atrium_memory::solitude_quality::SolitudeQualityEngine::new(),
            ),
            // Gap#5 共享仪式补充 — 已合并入 ritual 子系统 / Ritual supplements — merged into ritual subsystem
            // Gap#9 脆弱与不完美补充 — 已合并入 vulnerability 子系统 / Vulnerability supplements — merged into vulnerability subsystem
            // Gap#4 冲突与和解 / Conflict and reconciliation
            conflict_engine: parking_lot::Mutex::new(
                atrium_memory::conflict_engine::ConflictEngine::new(),
            ),
            // Gap#3 期待与想念 / Anticipation and longing
            longing: LongingSubsystem::new(
                LongingExpressionChannel::default(),
                DisappointmentHandler::default(),
                LongingNarrativeBridge::default(),
                atrium_memory::anticipation_depth::AnticipationDepthEngine::new(),
            ),
            // R3 通电：6个孤儿引擎初始化 / R3 power-on: initialize 6 orphan engines
            emotional_climate: parking_lot::Mutex::new(
                atrium_memory::emotional_climate::EmotionalClimate::new(),
            ),
            emotional_consolidation: parking_lot::Mutex::new(
                atrium_memory::emotional_consolidation::EmotionalConsolidation::new(),
            ),
            emotional_coupling: parking_lot::Mutex::new(
                atrium_memory::emotional_coupling::EmotionalCoupling::new(),
            ),
            existential_depth: parking_lot::Mutex::new(
                atrium_memory::existential_depth::ExistentialDepth::new(),
            ),
            inner_council: parking_lot::Mutex::new(
                atrium_memory::inner_council::InnerCouncil::new(),
            ),
            ritual_heartbeat: parking_lot::Mutex::new(
                atrium_memory::ritual_heartbeat::RitualHeartbeat::new(),
            ),
        }
    }

    /// 增强版记忆检索：FTS5 全文 + FactStore 语义 + STM 精确 + Persona + KeyFact 五路混合
    fn enhanced_search(&self, query: &str, limit: usize) -> Vec<(String, f64)> {
        let mut results: HashMap<String, f64> = HashMap::new();

        // 将长查询拆分为短词，每个短词独立搜索后合并
        let tokens = api_handler::split_query_tokens(query);
        // 如果有短词，用短词搜索；否则用原查询
        let queries: Vec<&str> = if tokens.len() > 1 {
            tokens.iter().map(|s| s.as_str()).collect()
        } else {
            vec![query]
        };

        for q in &queries {
            // FTS5 关键字搜索
            if let Ok(fts_results) = self.fts5.lock().search(q, 20) {
                for r in &fts_results {
                    let score = 1.0 / (1.0 + r.rank.abs());
                    results
                        .entry(r.content.clone())
                        .and_modify(|s| *s = s.max(score))
                        .or_insert(score);
                }
            }

            // FactStore 语义匹配（关键词交叠）
            if let Ok(fact_results) = self.fact_store.query(q) {
                for f in fact_results {
                    let key = f.canonical_form();
                    results
                        .entry(key)
                        .and_modify(|s| *s = s.max(f.confidence * 0.8))
                        .or_insert(f.confidence * 0.8);
                }
            }

            // STM 最近记忆精确匹配
            {
                let mem = self.memory.lock();
                for entry in mem.recent(30) {
                    let content = entry.content_str();
                    if content.contains(q) {
                        results
                            .entry(content.clone())
                            .and_modify(|s| *s = s.max(0.5))
                            .or_insert(0.5);
                    }
                }
            }

            // Persona 固化特质匹配
            {
                let rp = self.runtime_persona.lock();
                if let Some(persona) = rp.get("主人") {
                    for t in &persona.traits {
                        if t.name.contains(q) || t.value.contains(q) {
                            let key = format!("[人格]{}.{} = {}", persona.entity, t.name, t.value);
                            results
                                .entry(key)
                                .and_modify(|s| *s = s.max(t.confidence))
                                .or_insert(t.confidence);
                        }
                    }
                }
            }

            // KeyFactCache 搜索
            {
                let kf_results = self.key_facts.search(q);
                for kf in kf_results {
                    let key = format!("[关键{}] {}", kf.category.as_str(), kf.content);
                    results
                        .entry(key)
                        .and_modify(|s| *s = s.max(kf.confidence * 0.7))
                        .or_insert(kf.confidence * 0.7);
                }
            }

            // 关联记忆图扩散激活
            {
                let mut graph = self.graph.lock();
                let paths = graph.spread_activation(q, 0.5, 3);
                for path in paths.iter().take(5) {
                    if let Some(node) = graph.get_node(&path.to) {
                        let score = path.activation * 0.6;
                        results
                            .entry(node.content.clone())
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }
            }
        }

        let mut sorted: Vec<(String, f64)> = results.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit.max(1));
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::api_handler::detect_naming;
    use super::*;
    pub(crate) use atrium_bridge::grpc::atrium::{HealthCheckRequest, ProcessMessageRequest};

    fn test_service() -> CoreService {
        CoreService::new_in_memory()
    }

    /// 集成测试：完整 7 步管线 — 消息→STM→事实提取→证据→Reflection→Persona→Reply
    #[test]
    fn test_full_pipeline_single_message() {
        let svc = test_service();
        let req = ProcessMessageRequest {
            message: "主人好，我喜欢Rust编程".into(),
            session_id: "test".into(),
            user_id: "u1".into(),
            channel: "test".into(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(svc.process_message(req));

        // 回复应包含情感标签
        assert!(!resp.reply.is_empty());
        assert!(
            resp.reply.contains("happy")
                || resp.reply.contains("neutral")
                || resp.reply.contains("sad")
                || resp.reply.contains("名字")
        );
    }

    /// 命名仪式测试：未命名时引导，命名后使用新名字
    #[test]
    fn test_naming_ceremony() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // 第一步：未命名，AI 应引导起名
        let req1 = ProcessMessageRequest {
            message: "你好".into(),
            session_id: "naming".into(),
            user_id: "u1".into(),
            channel: "test".into(),
        };
        let resp1 = rt.block_on(svc.process_message(req1));
        assert!(
            resp1.reply.contains("名字") || resp1.reply.contains("Atrium"),
            "未命名时应引导起名: {}",
            resp1.reply
        );

        // 第二步：用户给出名字
        let req2 = ProcessMessageRequest {
            message: "你叫小未来".into(),
            session_id: "naming2".into(),
            user_id: "u1".into(),
            channel: "test".into(),
        };
        let resp2 = rt.block_on(svc.process_message(req2));
        assert!(
            resp2.reply.contains("小未来"),
            "命名后回复应包含新名字: {}",
            resp2.reply
        );

        // 第三步：命名后正常对话
        let req3 = ProcessMessageRequest {
            message: "小未来你好".into(),
            session_id: "naming3".into(),
            user_id: "u1".into(),
            channel: "test".into(),
        };
        let resp3 = rt.block_on(svc.process_message(req3));
        assert!(!resp3.reply.contains("名字"), "已命名不应再引导起名");
    }

    /// 记忆增强管线测试：事实应被提取并写入 FactStore
    #[test]
    fn test_fact_extraction_and_storage() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let messages = &[
            "我喜欢Rust编程",
            "我喜欢研究AI技术",
            "我知道Rust很快",
            "我想学习深度学习",
            "我在用Rust写项目",
            "我觉得AI很有趣",
            "我喜欢写代码",
            "我讨厌bug",
            "我爱Rust语言",
            "我想研究大模型",
            "我知道Python",
            "我在杭州",
        ];
        for msg in messages {
            let req = ProcessMessageRequest {
                message: msg.to_string(),
                session_id: "facts".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        let facts = svc.fact_store.query("Rust").unwrap();
        assert!(!facts.is_empty(), "应提取并存储了关于Rust的事实");

        let facts2 = svc.fact_store.query("AI").unwrap();
        assert!(!facts2.is_empty(), "应提取并存储了关于AI的事实");
    }

    /// Reflection 触发测试：8 条消息后应触发 reflection
    #[test]
    fn test_reflection_triggered() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let insights_before = svc.reflection.lock().all_insights().len();

        // 发多种消息（不同谓语），确保 reflection 能合成洞察
        let messages = &[
            "我喜欢Rust",
            "我喜欢AI",
            "我喜欢编程",
            "我喜欢游戏",
            "我知道tokio",
            "我知道sled",
            "我知道scc",
            "我知道gRPC",
            "我在杭州",
            "我想学AI",
        ];
        for msg in messages {
            let req = ProcessMessageRequest {
                message: msg.to_string(),
                session_id: "reflect".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        let insights_after = svc.reflection.lock().all_insights().len();
        assert!(
            insights_after > insights_before,
            "8条消息后应触发Reflection：before={}, after={}",
            insights_before,
            insights_after
        );
    }

    /// Token 预算 + 摘要测试
    #[test]
    fn test_summarizer_triggered() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let summaries_before = svc.summarizer.lock().summary_count();

        // 发 25 条消息（超过 20 条摘要窗口）
        for i in 0..25 {
            let msg = format!("这是一条测试消息，编号{}，主人讨论了Rust编程和AI技术", i);
            let req = ProcessMessageRequest {
                message: msg,
                session_id: "summary".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        let summarizer = svc.summarizer.lock();
        let summaries_after = summarizer.summary_count();
        assert!(
            summaries_after > summaries_before,
            "20条消息后应触发摘要：before={}, after={}",
            summaries_before,
            summaries_after
        );
        assert!(
            summarizer.pending_llm_text.is_some(),
            "应有待LLM处理的摘要文本"
        );
    }

    /// LLM 摘要提交测试
    #[test]
    fn test_llm_summary_submission() {
        let svc = test_service();
        // 提交一个 LLM 摘要（模拟 Python 网关调用）
        svc.submit_llm_summary("用户主要讨论了Rust编程和AI技术，总体情绪积极。".into());

        let ctx = svc.summarizer.lock().summary_context();
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("AI"));
    }

    /// 健康检查包含所有模块状态
    #[test]
    fn test_health_check_all_modules() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let req = HealthCheckRequest {
            event_count: 0,
            room_incoming_json: String::new(),
        };

        let resp = rt.block_on(svc.health_check(req));
        assert!(resp.ok);
        assert!(resp.module_states.contains_key("memory"));
        assert!(resp.module_states.contains_key("emotion"));
        assert!(resp.module_states.contains_key("persona"));
        assert!(resp.module_states.contains_key("fact_store"));
        assert!(resp.module_states.contains_key("reflection"));
        assert!(resp.module_states.contains_key("token_budget"));
        assert!(resp.module_states.contains_key("summaries"));
        assert!(resp.module_states.contains_key("key_facts"));
        assert!(resp.module_states.contains_key("summary_pending"));
    }

    /// 情感影响测试：多次消息后情感应偏离默认值
    #[test]
    fn test_emotion_accumulates() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let emo_before = svc.current_emotion();

        for _ in 0..20 {
            let req = ProcessMessageRequest {
                message: "你好".into(),
                session_id: "emo".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        let emo_after = svc.current_emotion();
        // 情感应有正向积累（每次 +0.05 愉悦度）
        assert!(
            emo_after.pleasure > emo_before.pleasure,
            "多次消息后愉悦度应上升：before={:.2}, after={:.2}",
            emo_before.pleasure,
            emo_after.pleasure
        );
    }

    /// 搜索记忆集成测试
    #[test]
    fn test_search_memory_integration() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // 写入几条消息
        for msg in &["主人喜欢Rust", "主人喜欢编程", "主人喜欢AI"] {
            let req = ProcessMessageRequest {
                message: msg.to_string(),
                session_id: "search".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        // 搜索记忆
        let search_req = atrium_bridge::grpc::atrium::SearchMemoryRequest {
            query: "Rust".into(),
            limit: 10,
        };
        let resp = rt.block_on(svc.search_memory(search_req));
        assert!(!resp.results.is_empty(), "搜索Rust应有结果");
    }

    /// 命名检测函数单元测试
    #[test]
    fn test_detect_naming_patterns() {
        assert_eq!(detect_naming("我叫你小未来"), Some("小未来".into()));
        assert_eq!(detect_naming("你叫Atrium吧"), Some("Atrium".into()));
        assert_eq!(detect_naming("你就叫Chino"), Some("Chino".into()));
        assert_eq!(detect_naming("你的名字是未来酱"), Some("未来酱".into()));
        assert_eq!(detect_naming("给你起名小不点"), Some("小不点".into()));
        assert_eq!(detect_naming("命名你为Mirai"), Some("Mirai".into()));
        assert_eq!(detect_naming("今天天气真好"), None);
        assert_eq!(detect_naming("你"), None); // 太短
    }
}
