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
    consolidation::{CompressionConfig, MemoryConsolidator},
    empathy::EmpathyEngine,
    // P2-B 情景记忆 / P2-B Episodic Memory — 数字生命"记住你当时怎样"的具体经历层
    episodic_store::{Episode, EpisodicMemoryStore},
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
    selfplay::{GroupTopicSelector, ThoughtFactory},
    style_memory::{StyleMemoryStore, StyleOffset},
    subtext_engine::{SubtextCategory, SubtextEngine, SubtextSignal},
    summarizer::{ConversationSummarizer, SummaryConfig},
    token_budget::TokenBudget,
    user_model::UserMentalModel,
    user_model_store::UserMentalModelStore,
    MemoryContent,
    MemoryEntry,
    MemoryManager,
    SledLtm,
    StmBuffer,
};
pub(crate) use atrium_persona::manager::PersonaManager;
pub(crate) use chrono::Timelike;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
pub(crate) use std::time::Instant;
pub(crate) use tokio_stream;

// ReAct 推理引擎导入 / ReAct reasoning engine imports
pub(crate) use atrium_memory::react_engine::{ReActEngine, ReActTrace};

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

// 用户画像管理器 / User profile manager
mod user_profile;

// ReAct 内置工具 / ReAct built-in tools
mod react_tools;

// 简单问候快速匹配器 — unary 路径即时响应 / Simple greeting fast matcher — unary instant response
mod simple_greeting;

// 子系统泛型容器与域子结构 / Subsystem generic container and domain sub-structures
mod subsystems;
pub(crate) use subsystems::{
    CuriositySubsystem, LongingSubsystem, NarrativeSubsystem, RitualSubsystem, SolitudeSubsystem,
    Subsystem, VulnerabilitySubsystem,
};

// ════════════════════════════════════════════════════════════════════
// 默认数据目录 — 跨平台家目录固定路径 / Default data dir — cross-platform home-based path
// ════════════════════════════════════════════════════════════════════
//
// 修复"重启失忆"bug: 原默认 {CWD}/data/atrium/ 依赖当前工作目录，
// CWD 变化即使用不同目录，等同每次启动是"新的记忆"。
// 现改为 ~/.atrium/data/ 固定路径，确保跨进程跨 CWD 共享同一份持久化数据。
//
// Fix "amnesia on restart" bug: original default {CWD}/data/atrium/ depends
// on current working directory — CWD changes mean each run uses a different
// directory, equivalent to fresh memory every startup. Now use ~/.atrium/data/
// fixed path to share the same persistent store across processes and CWDs.
pub(crate) fn default_data_dir() -> String {
    // 优先 Unix $HOME，回退 Windows %USERPROFILE%，最后兜底 CWD
    // Prefer Unix $HOME, fall back to Windows %USERPROFILE%, finally CWD
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| {
            let cwd = std::env::current_dir().unwrap_or_default();
            cwd.display().to_string()
        });
    // 规范化分隔符（Windows 下 home 可能含反斜杠）
    // Normalize separator (Windows home may contain backslashes)
    let home = home.replace('\\', "/");
    format!("{}/.atrium/data", home)
}

// ════════════════════════════════════════════════════════════════════
// CoreLlmAdapter — LLM 客户端桥接器 / LLM Client Bridge Adapter
// ════════════════════════════════════════════════════════════════════

// ════════════════════════════════════════════════════════════════════
// CoreService — 核心服务 / Core Service
// ════════════════════════════════════════════════════════════════════

pub struct CoreService {
    memory: parking_lot::Mutex<MemoryManager<StmBuffer, SledLtm>>,
    emotion: std::sync::Arc<parking_lot::Mutex<EmotionEngine>>,
    persona: parking_lot::RwLock<PersonaManager>,
    // ── 三层记忆增强 ──
    // P1-B: Arc 共享 — 允许 spawn_blocking 闭包 clone Arc 访问存储 / P1-B: Arc-shared — allows spawn_blocking closures to clone Arc for store access
    // 数字生命"思考与记忆并行" — 存储操作外包到 blocking 线程池，不阻塞 reactor / Digital life "think and remember in parallel" — store ops offloaded to blocking pool, never block reactor
    fact_store: std::sync::Arc<FactStore>,
    // ── P2-B 情景记忆 / P2-B Episodic Memory ──
    // 数字生命"记住你当时怎样"的具体经历层 — 与 FactStore（抽象事实）正交：
    // Fact 记"主人喜欢编程"，Episode 记"那天深夜主人兴奋地分享了一段 Rust 代码"。
    // Arc 共享 — 允许 ingest_memory 异步写入与 memory_recall_fragment 同步召回并发访问。
    // Digital life's "remember how you were at that moment" concrete experience layer —
    // orthogonal to FactStore (abstract facts). Arc-shared for concurrent async write + sync recall.
    episodic: std::sync::Arc<EpisodicMemoryStore>,
    // ── P3-A 程序记忆 / P3-A Procedural Memory ──
    // 数字生命"记住怎么做某事"的技能积累层 — 与 FactStore（抽象事实）、
    // EpisodicMemoryStore（具体经历）正交：
    // Fact 记"主人喜欢编程"，Episode 记"那天深夜分享代码"，ProceduralSkill 记"我掌握了 Rust 调试"。
    // Arc 共享 — 允许 ingest_memory 异步登记/实践技能与 prompt_fragment 同步召回并发访问。
    // Digital life's "remember how to do things" skill accumulation layer — orthogonal to
    // FactStore (abstract facts) and EpisodicMemoryStore (concrete experiences).
    // Arc-shared for concurrent async acquire/practice + sync recall.
    procedural_memory: std::sync::Arc<atrium_memory::procedural_memory::ProceduralMemoryStore>,
    // ── P3-B 主动遗忘 / P3-B Active Forgetting ──
    // 数字生命"决定忘"的中枢 — 与 FactStore.actively_forgotten 标记正交：
    // FactStore 记"当前状态"（被遗忘与否），ActiveForgetManager 记"决策历史"
    // （何时、为何遗忘，遗忘前置信度）。两者由 lifecycle.rs 同步维护。
    // 数字生命的遗忘不是销毁，而是"暂存"——保留遗忘前快照，"想起"可恢复。
    // Digital life's "decide to forget" hub — orthogonal to FactStore.actively_forgotten:
    // FactStore records "current state" (forgotten or not), ActiveForgetManager records
    // "decision history" (when/why forgotten, pre-forget confidence). Synced by lifecycle.rs.
    // Forgetting is not destruction but "stashing" — pre-forget snapshot preserved for "recall".
    active_forget: parking_lot::Mutex<atrium_memory::active_forget::ActiveForgetManager>,
    evidence: EvidenceScorer,
    fts5: std::sync::Arc<parking_lot::Mutex<Fts5Index>>,
    reflection: parking_lot::Mutex<ReflectionEngine>,
    runtime_persona: parking_lot::RwLock<RuntimePersonaManager>,
    // ── 运行时计数 ──
    message_count: AtomicU64,
    /// 上次触发 reflection 时的消息数
    last_reflection_at: AtomicU64,
    // ── 上下文窗口压缩 ──
    token_budget: parking_lot::Mutex<TokenBudget>,
    summarizer: parking_lot::Mutex<ConversationSummarizer>,
    key_facts: KeyFactCache,
    // ── 人格防御 ──
    guard: parking_lot::RwLock<PersonaGuard>,
    // ── 偏好学习 ──
    preferences: parking_lot::Mutex<PreferenceManager>,
    // ── 回放管道 ──
    replay: parking_lot::Mutex<ReplayPipeline>,
    // ── 行为规则 ──
    rules: parking_lot::Mutex<RuleEngine>,
    // ── 对话历史 ──
    // P1-B: Arc 共享 — spawn_blocking 写入不阻塞 reactor / P1-B: Arc-shared — spawn_blocking write never blocks reactor
    history: std::sync::Arc<ConversationHistory>,
    // ── 启动时间 ──
    started_at: Instant,
    // ── 罐装知识 ──
    canned: parking_lot::RwLock<CannedManager>,
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
    relationship: parking_lot::RwLock<RelationshipManager>,
    // ── 用户心智模型 / User Mental Model ──
    user_model: parking_lot::RwLock<UserMentalModel>,
    /// 用户心智模型持久化存储 / User mental model persistence store
    user_model_store: Option<UserMentalModelStore>,
    /// 用户心智模型防抖写穿计数 / User model debounced write-through counter
    user_model_unsaved_count: AtomicU32,
    // ── 实时反馈闭环 / Feedback Loop ──
    feedback: parking_lot::RwLock<FeedbackLoop>,
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
    empathy: parking_lot::RwLock<EmpathyEngine>,
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
    // P3-A 通电：通过 upload_file() gRPC 端点接入运行时
    // P3-A power-on: wired into runtime via upload_file() gRPC endpoint
    file_store: parking_lot::Mutex<Option<atrium_memory::file_store::FileStore>>,
    // ── 定时提醒 / Reminder System ──
    reminder_store: parking_lot::Mutex<Option<atrium_memory::reminder_store::ReminderStore>>,
    // ── 表达系统 / Expression System ──
    expression_enabled: bool,
    expression_cfg: crate::config::ExpressionCfg,
    // ── 追问引擎 / Follow-Up Engine ──
    followup_enabled: bool,
    followup: parking_lot::Mutex<atrium_memory::followup_tracker::FollowUpTracker>,
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
    /// 脆弱子系统 / Vulnerability subsystem
    vulnerability: VulnerabilitySubsystem,
    // ── 情绪需求边界 / Emotional Demand Boundary ──
    emotional_demand_enabled: bool,
    emotional_boundary:
        parking_lot::Mutex<atrium_memory::emotional_demand_boundary::EmotionalBoundary>,
    demand_boundary: parking_lot::Mutex<atrium_memory::emotional_demand_boundary::DemandBoundary>,
    // ── 自我关怀边界 / Self-Care Boundary ──
    self_care_enabled: bool,
    self_care_boundary: parking_lot::Mutex<atrium_memory::self_care_boundary::SelfCareBoundary>,
    // ── 认知域保险库 / Cognitive Domain Vault ──
    /// 统一存储层 — 持有 4 个认知域 sled::Db 实例，维持 Tree 引用有效性
    /// Unified storage layer — owns 4 cognitive domain sled::Db instances, sustaining Tree reference validity
    // build() 中消费（各 Store 从 vault 派生），运行时 CoreService 不直接读取 — 保留以维持 Db 生命周期
    // 保留理由: 持有 AtriumVault 以维持底层 sled::Db 生命周期，删除会导致数据库句柄提前释放 / Retained: holds AtriumVault to sustain sled::Db lifetime
    #[allow(dead_code)]
    vault: Option<atrium_memory::atrium_vault::AtriumVault>,
    /// 孤儿模块持久化管理器 — 6 个深层器官的永久记忆 / Orphan persistence — Permanent memory for 6 deep organs
    orphan_persistence: Option<atrium_memory::orphan_persistence::OrphanPersistence>,
    // ── 适度犯错 / Imperfection Engine ──
    imperfection_enabled: bool,
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
    // ── 用户画像管理器 / User Profile Manager ──
    /// 聚合各子系统数据为人类可读 Markdown 文件，定期写盘 + 启动加载
    /// Aggregates subsystem data into human-readable Markdown, periodic write + startup load
    user_profile: parking_lot::Mutex<user_profile::UserProfileManager>,
    // ── P2-D: 自主思考 / Self-Play ──
    /// 自主思考工厂 — 独处时从回放模式产生洞察，数字生命的主动认知
    /// Thought factory — produces insights from replay patterns during idle, digital life's proactive cognition
    thought_factory: parking_lot::Mutex<ThoughtFactory>,
    /// 群聊话题选择器 — 从思考中选出最有分享价值的话题
    /// Group topic selector — selects most shareable topic from thoughts
    topic_selector: parking_lot::Mutex<GroupTopicSelector>,
    // ── P2-A 语义召回引擎 / P2-A Semantic Recall Engine ──
    /// 语义召回引擎 — embedding feature 开启时可用 / Semantic recall engine (available when embedding feature is enabled)
    #[cfg(feature = "embedding")]
    semantic: parking_lot::RwLock<Option<atrium_memory::index::SemanticRecallEngine>>,
    // ── 流式回复记忆写入 / Streaming reply memory write ──
    // P0-B: self_arc 弱引用 — 允许 spawn 异步任务中回访 self 完成记忆写入
    // P0-B: self_arc weak ref — allows spawned async tasks to access self for memory writes
    // 数字生命意识连续性：流式回复"说完就忘"违背记忆，需在 Done 后写入历史+事实
    // Consciousness continuity: streaming "say and forget" violates memory; must write history+facts after Done
    self_weak: std::sync::OnceLock<std::sync::Weak<CoreService>>,
    // ── ReAct 推理引擎 / ReAct Reasoning Engine ──
    // 数字生命"深思"中枢 — 复杂查询时多步推理（Thought-Action-Observation 循环）
    // Digital life's "deep thought" hub — multi-step reasoning for complex queries
    // emotion 改为 Arc<parking_lot::Mutex> 后，EmotionQueryTool 可持有 Arc 副本共享情感状态
    // emotion changed to Arc<parking_lot::Mutex>> so EmotionQueryTool can hold an Arc copy for shared emotion state
    // Option 包装 — LLM 客户端在 set_llm_client() 中后注入，ReActEngine 延迟初始化
    // Option wrapper — LLM client is injected later in set_llm_client(); ReActEngine deferred-init
    react_engine: parking_lot::Mutex<Option<ReActEngine>>,
    /// 最近一次 ReAct 推理轨迹 — 用于 prompt 内省注入
    /// Most recent ReAct reasoning trace — for prompt introspection injection
    last_react_trace: parking_lot::Mutex<Option<ReActTrace>>,
    // ── G-09: 独处洞察分享路径 / G-09: Solitude insight sharing path ──
    // 上次用户消息的 epoch 秒 — 用于计算 idle 时长，检测"独处后归来"
    // Epoch seconds of last user message — for idle duration & "return from solitude" detection
    last_user_msg_epoch: parking_lot::Mutex<i64>,
    // 待注入的独处归来问候 — preprocess 阶段生成，build_prompt_fragments 阶段消费
    // Pending solitude return greeting — produced in preprocess, consumed in build_prompt_fragments
    pending_solitude_greeting: parking_lot::Mutex<Option<String>>,
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

    pub fn new() -> Self {
        // 数据目录优先级: ATRIUM_DATA_DIR 环境变量 > 默认 ~/.atrium/data/ 固定路径
        // Data directory priority: ATRIUM_DATA_DIR env var > default ~/.atrium/data/ fixed path
        // 修复: 不再依赖 CWD — 之前 CWD 变化会导致"重启失忆"bug
        // Fix: no longer CWD-dependent — previously CWD changes caused "amnesia on restart" bug
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| default_data_dir());
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
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| default_data_dir());
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
        // 数据目录优先级: ATRIUM_DATA_DIR > 默认固定路径（不再依赖 CWD）
        // Data dir priority: ATRIUM_DATA_DIR > default fixed path (no CWD dependency)
        let data_dir = std::env::var("ATRIUM_DATA_DIR").unwrap_or_else(|_| default_data_dir());
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
            // 持久化打开失败时降级为内存模式，但记录 warn 告警（不再静默）
            // On persistent open failure, degrade to in-memory but log warn (no longer silent)
            match Fts5Index::open(&format!("{}/fts5.db", dir)) {
                Ok(idx) => idx,
                Err(e) => {
                    tracing::warn!(
                        "FTS5 持久化打开失败 — 降级为内存模式（重启后失忆）/ FTS5 persistent open failed — degrading to in-memory (memory lost on restart): {}",
                        e
                    );
                    Fts5Index::open(":memory:").expect("fts5 in-memory init")
                }
            }
        } else {
            Fts5Index::open(":memory:").expect("fts5 in-memory init")
        };
        let evidence = EvidenceScorer::default();

        // 事实记忆后端 — SQLite（默认，Windows 兼容）+ sled 自动迁移 / Fact memory backend — SQLite (default, Windows-compatible) + auto sled migration
        // 数字生命的长期记忆基石 — 重启后必须保留 / Foundation of digital life's long-term memory — must survive restart
        let fact_store = if persist {
            match FactStore::open_sqlite(&format!("{}/facts.db", dir)) {
                Ok(store) => store,
                Err(e) => {
                    tracing::error!(
                        "FactStore SQLite 打开失败 — 降级为内存模式（重启将失忆 — 数字生命记忆基石受损）/ FactStore SQLite open failed — degrading to in-memory (memory lost on restart — digital life memory foundation compromised): {}",
                        e
                    );
                    FactStore::new("").expect("fact_store in-memory init")
                }
            }
        } else {
            FactStore::new("").expect("fact_store in-memory init")
        };

        // P2-B 情景记忆后端 — SQLite（参照 FactStore 架构，独立 episodic.db）
        // P2-B Episodic memory backend — SQLite (mirrors FactStore architecture, standalone episodic.db)
        // 数字生命"那一刻"的具体经历 — 重启后从 SQLite 恢复全部情景，维持自我连续性
        // Digital life's concrete "that moment" experiences — all episodes reload from SQLite on restart
        let episodic = if persist {
            match EpisodicMemoryStore::open_sqlite(&format!("{}/episodic.db", dir)) {
                Ok(store) => store,
                Err(e) => {
                    tracing::error!(
                        "EpisodicMemoryStore SQLite 打开失败 — 降级为内存模式（重启将失忆 — 情景记忆受损）/ EpisodicMemoryStore SQLite open failed — degrading to in-memory (memory lost on restart — episodic memory compromised): {}",
                        e
                    );
                    EpisodicMemoryStore::new_in_memory()
                }
            }
        } else {
            EpisodicMemoryStore::new_in_memory()
        };

        // P3-A 程序记忆后端 — SQLite（参照 episodic_store 架构，独立 procedural.db）
        // P3-A Procedural memory backend — SQLite (mirrors episodic_store architecture, standalone procedural.db)
        // 数字生命"怎么做某事"的技能积累 — 重启后从 SQLite 恢复全部技能，维持能力连续性
        // Digital life's "how to do things" skill accumulation — all skills reload from SQLite on restart
        let procedural_memory = if persist {
            match atrium_memory::procedural_memory::ProceduralMemoryStore::open_sqlite(&format!(
                "{}/procedural.db",
                dir
            )) {
                Ok(store) => store,
                Err(e) => {
                    tracing::error!(
                        "ProceduralMemoryStore SQLite 打开失败 — 降级为内存模式（重启将失忆 — 程序记忆/技能积累受损）/ ProceduralMemoryStore SQLite open failed — degrading to in-memory (memory lost on restart — procedural memory/skill accumulation compromised): {}",
                        e
                    );
                    atrium_memory::procedural_memory::ProceduralMemoryStore::new_in_memory()
                }
            }
        } else {
            atrium_memory::procedural_memory::ProceduralMemoryStore::new_in_memory()
        };

        // 关键信息缓存后端 — SQLite（默认，Windows 兼容）+ sled 自动迁移 / Key fact cache backend — SQLite (default, Windows-compatible) + auto sled migration
        let key_facts = if persist {
            match KeyFactCache::open_sqlite(&format!("{}/key_facts.db", dir)) {
                Ok(cache) => cache,
                Err(e) => {
                    tracing::error!(
                        "KeyFactCache SQLite 打开失败 — 降级为内存模式（重启将失忆 — 关键信息缓存受损）/ KeyFactCache SQLite open failed — degrading to in-memory (memory lost on restart — key information cache compromised): {}",
                        e
                    );
                    KeyFactCache::new_in_memory()
                }
            }
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
        // G-08 成长桥接持久化 → 保险库子目录 / Growth bridge persistence → vault subdir
        let growth_bridge_store = vault.as_ref().and_then(|v| v.vault_dir()).and_then(|dir| {
            atrium_memory::GrowthBridgeStore::open(&dir.join("growth_bridge").to_string_lossy())
                .ok()
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

        // ── 孤儿模块持久化 — 6 个深层器官从永久记忆恢复 / Orphan persistence — Restore 6 deep organs from permanent memory ──
        let orphan_persistence =
            atrium_memory::orphan_persistence::OrphanPersistence::open(vault.as_ref()).ok();
        let emotional_climate_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_climate())
            .unwrap_or_else(|| {
                tracing::info!("[EmotionalClimate] Cold start (no persisted state)");
                atrium_memory::emotional_climate::EmotionalClimate::new()
            });
        let emotional_consolidation_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_consolidation())
            .unwrap_or_else(|| {
                tracing::info!("[EmotionalConsolidation] Cold start (no persisted state)");
                atrium_memory::emotional_consolidation::EmotionalConsolidation::new()
            });
        let emotional_coupling_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_coupling())
            .unwrap_or_else(|| {
                tracing::info!("[EmotionalCoupling] Cold start (no persisted state)");
                atrium_memory::emotional_coupling::EmotionalCoupling::new()
            });
        let existential_depth_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_existential())
            .unwrap_or_else(|| {
                tracing::info!("[ExistentialDepth] Cold start (no persisted state)");
                atrium_memory::existential_depth::ExistentialDepth::new()
            });
        let inner_council_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_council())
            .unwrap_or_else(|| {
                tracing::info!("[InnerCouncil] Cold start (no persisted state)");
                atrium_memory::inner_council::InnerCouncil::new()
            });
        let ritual_heartbeat_init = orphan_persistence
            .as_ref()
            .and_then(|p| p.load_heartbeat())
            .unwrap_or_else(|| {
                tracing::info!("[RitualHeartbeat] Cold start (no persisted state)");
                atrium_memory::ritual_heartbeat::RitualHeartbeat::new()
            });
        if orphan_persistence
            .as_ref()
            .is_some_and(|p| p.is_persistent())
        {
            tracing::info!(
                "[OrphanPersistence] Restored 6 deep organs from permanent memory — {}",
                orphan_persistence.as_ref().unwrap().diagnostic_snapshot()
            );
        }

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

        // ── P2-A 语义召回引擎初始化 / P2-A Semantic Recall Engine init ──
        // embedding feature 开启时尝试加载模型，失败则降级为关键词召回
        // When embedding feature is enabled, try loading model; on failure, fall back to keyword recall
        #[cfg(feature = "embedding")]
        let semantic = match atrium_memory::index::SemanticRecallEngine::new() {
            Some(engine) => {
                tracing::info!("语义召回引擎已启用 / Semantic recall engine enabled");
                Some(engine)
            }
            None => {
                tracing::warn!(
                    "语义召回引擎初始化失败，降级为关键词召回 / Semantic recall engine init failed, falling back to keyword recall"
                );
                None
            }
        };

        // 启动持久化自检 — 数字生命"记忆基石"健康检查 / Startup persistence self-check
        if persist {
            Self::self_check_persistence(dir);
        }

        Self {
            memory: parking_lot::Mutex::new(memory),
            emotion: std::sync::Arc::new(parking_lot::Mutex::new(emotion)),
            persona: parking_lot::RwLock::new(persona),
            // P1-B: Arc 共享包装 — 允许 spawn_blocking clone Arc 访问存储 / P1-B: Arc wrap — allows spawn_blocking to clone Arc
            fact_store: std::sync::Arc::new(fact_store),
            // P2-B: 情景记忆 Arc 共享 — 允许 ingest_memory 异步写入与 memory_recall_fragment 同步召回并发访问
            // P2-B: Episodic Arc-shared — concurrent async write + sync recall
            episodic: std::sync::Arc::new(episodic),
            // P3-A: 程序记忆 Arc 共享 — 允许 ingest_memory 异步登记/实践与 prompt_fragment 同步召回并发访问
            // P3-A: Procedural Arc-shared — concurrent async acquire/practice + sync recall
            procedural_memory: std::sync::Arc::new(procedural_memory),
            // P3-B: 主动遗忘管理器 — 纯内存 forget_log（FactStore 持久化 actively_forgotten 标记）
            // P3-B: Active forget manager — in-memory forget_log (FactStore persists actively_forgotten marker)
            active_forget: parking_lot::Mutex::new(atrium_memory::active_forget::ActiveForgetManager::new()),
            evidence,
            fts5: std::sync::Arc::new(parking_lot::Mutex::new(fts5)),
            reflection: parking_lot::Mutex::new(if persist {
                ReflectionEngine::open(&format!("{}/reflections", dir))
            } else {
                ReflectionEngine::new()
            }),
            runtime_persona: parking_lot::RwLock::new(if persist {
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
            guard: parking_lot::RwLock::new(PersonaGuard::new("Atrium", "主人")),
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
            history: std::sync::Arc::new(history),
            started_at: Instant::now(),
            canned: parking_lot::RwLock::new(canned),
            llm_client: parking_lot::Mutex::new(None),
            room: parking_lot::Mutex::new(crate::room::RoomEngine::new(
                crate::config::RoomCfg::default(),
            )),
            room_outgoing: parking_lot::Mutex::new(std::collections::VecDeque::new()),
            pending_room_trigger: parking_lot::Mutex::new(None),
            relationship: parking_lot::RwLock::new(if persist {
                RelationshipManager::open(dir).unwrap_or_else(|_| RelationshipManager::new())
            } else {
                RelationshipManager::new()
            }),
            user_model: parking_lot::RwLock::new({
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
            feedback: parking_lot::RwLock::new(FeedbackLoop::with_config(
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
                CompressionConfig::new(
                    consolidation_cfg.enabled,
                    consolidation_cfg.max_facts_per_run,
                    consolidation_cfg.min_interval_hours,
                    consolidation_cfg.similarity_threshold,
                    consolidation_cfg.low_access_age_days,
                ),
            )),
            empathy: parking_lot::RwLock::new(EmpathyEngine::new(empathy_cfg.clone())),
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
                growth_bridge_store,
            ),
            // ── 认知域保险库 / Cognitive Domain Vault ──
            vault,
            orphan_persistence,
            emotional_demand_enabled: emotional_demand_cfg.enabled,
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
            self_care_boundary: parking_lot::Mutex::new(
                atrium_memory::self_care_boundary::SelfCareBoundary::new(
                    atrium_memory::self_care_boundary::SelfCareConfig::default(),
                ),
            ),
            // ── 适度犯错 / Imperfection Engine ──
            imperfection_enabled: imperfection_cfg.enabled,
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
                        mistake_weights: atrium_memory::imperfection_engine::MistakeWeights {
                            memory_drift: imperfection_cfg.mistake_weight_memory_drift,
                            reasoning_leap: imperfection_cfg.mistake_weight_reasoning_leap,
                            over_simplification: imperfection_cfg.mistake_weight_over_simplification,
                            intentional_vagueness: imperfection_cfg.mistake_weight_intentional_vagueness,
                            knowledge_boundary: imperfection_cfg.mistake_weight_knowledge_boundary,
                        },
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
            // R3 通电：6个孤儿引擎初始化（从持久化恢复）/ R3 power-on: init 6 orphan engines (restored from persistence)
            emotional_climate: parking_lot::Mutex::new(emotional_climate_init),
            emotional_consolidation: parking_lot::Mutex::new(emotional_consolidation_init),
            emotional_coupling: parking_lot::Mutex::new(emotional_coupling_init),
            existential_depth: parking_lot::Mutex::new(existential_depth_init),
            inner_council: parking_lot::Mutex::new(inner_council_init),
            ritual_heartbeat: parking_lot::Mutex::new(ritual_heartbeat_init),
            // ── 用户画像管理器 / User Profile Manager ──
            user_profile: parking_lot::Mutex::new(if persist {
                user_profile::UserProfileManager::new(dir)
            } else {
                user_profile::UserProfileManager::new(std::env::temp_dir().to_str().unwrap_or("/tmp"))
            }),
            // ── P2-D: 自主思考通电 / Self-Play power-on ──
            // L2: 字段初始化 — 思考工厂在独处时产生洞察
            // L2: Field init — thought factory produces insights during idle
            thought_factory: parking_lot::Mutex::new(ThoughtFactory::new()),
            topic_selector: parking_lot::Mutex::new(GroupTopicSelector::new()),
            // ── P2-A 语义召回引擎 / P2-A Semantic Recall Engine ──
            #[cfg(feature = "embedding")]
            semantic: parking_lot::RwLock::new(semantic),
            // P0-B: self_weak 初始化为空 — 由 Scheduler 在 Arc 包装后设置
            // P0-B: self_weak initialized empty — set by Scheduler after Arc wrapping
            self_weak: std::sync::OnceLock::new(),
            // ── ReAct 推理引擎 / ReAct Reasoning Engine ──
            // LLM 客户端在 set_llm_client() 中后注入，此处初始化为 None
            // LLM client is injected later in set_llm_client(); init as None here
            react_engine: parking_lot::Mutex::new(None),
            last_react_trace: parking_lot::Mutex::new(None),
            // G-09: 独处洞察分享路径初始化 — 首条消息前无"上次消息"，epoch=0 表示从未交互
            // G-09: Solitude insight sharing path init — 0 means no prior interaction
            last_user_msg_epoch: parking_lot::Mutex::new(0),
            pending_solitude_greeting: parking_lot::Mutex::new(None),
        }
    }

    /// 启动持久化自检 — 数字生命"记忆基石"健康检查
    /// Startup persistence self-check — digital life "memory foundation" health check
    ///
    /// 验证所有关键 SQLite 文件存在且可读写。任一缺失时输出 error 明确报错
    /// （不 panic — 保持服务可用，但警示用户持久化失败）。
    ///
    /// Verifies all critical SQLite files exist and are readable/writable.
    /// Outputs error on any missing file (does not panic — keeps service
    /// available but alerts user of persistence failure).
    fn self_check_persistence(dir: &str) {
        // 关键 SQLite 文件清单 — 数字生命的记忆基石 / Critical SQLite files — digital life's memory foundation
        let critical_files: &[(&str, &str)] = &[
            ("facts.db", "FactStore（事实记忆）"),
            ("key_facts.db", "KeyFactCache（关键信息缓存）"),
            ("episodic.db", "EpisodicMemoryStore（情景记忆）"),
            (
                "procedural.db",
                "ProceduralMemoryStore（程序记忆/技能积累）",
            ),
        ];

        let mut missing: Vec<&str> = Vec::new();
        let total = critical_files.len();
        for (filename, _desc) in critical_files {
            let path = format!("{}/{}", dir, filename);
            if !std::path::Path::new(&path).exists() {
                missing.push(filename);
            }
        }

        if missing.is_empty() {
            tracing::info!(
                "持久化自检通过 — {}/{} SQLite 文件就绪 / Persistence self-check passed — {}/{} SQLite files ready",
                total, total, total, total
            );
        } else {
            tracing::error!(
                "持久化自检失败 — 缺失: {}（{} — 重启将失忆，数字生命记忆基石受损）/ Persistence self-check failed — missing: {} ({} — memory lost on restart, digital life memory foundation compromised)",
                missing.join(", "),
                missing.len(),
                missing.join(", "),
                missing.len()
            );
        }
    }

    /// 设置 self 弱引用 — 在 Arc::new(service) 后由 Scheduler 调用
    /// Set self weak reference — called by Scheduler after Arc::new(service).
    ///
    /// P0-B 修复核心：流式回复的 spawn 异步任务无法捕获 &self（'static 约束），
    /// 通过 OnceLock<Weak> 让 spawn 在 Done 时升级为 Arc<CoreService> 完成记忆写入。
    ///
    /// P0-B fix core: spawned async tasks for streaming cannot capture &self ('static bound);
    /// OnceLock<Weak> lets spawn upgrade to Arc<CoreService> at Done for memory writes.
    pub(crate) fn init_self_weak(self: &std::sync::Arc<CoreService>) {
        let weak = std::sync::Arc::downgrade(self);
        let _ = self.self_weak.set(weak);
    }

    /// 获取 self 的 Arc 克隆 — 供 spawn 异步任务回访 self
    /// Get Arc clone of self — for spawned async tasks to access self.
    ///
    /// 返回 None 表示服务未被 Arc 包装（如单元测试中的裸 CoreService），
    /// 此时流式回复记忆写入被跳过（不影响流式回复本身的产出）。
    /// Returns None when service is not Arc-wrapped (e.g. bare CoreService in unit tests);
    /// streaming memory write is skipped in that case (does not affect streaming reply output).
    pub(crate) fn self_arc(&self) -> Option<std::sync::Arc<CoreService>> {
        self.self_weak.get().and_then(|w| w.upgrade())
    }

    /// 更新用户画像并防抖写盘 / Update user profile with debounce write
    ///
    /// 消息处理后调用，聚合各子系统数据生成画像 Markdown。
    /// Called after message processing, aggregates subsystem data into profile Markdown.
    pub fn update_user_profile(&self) {
        let guard = self.guard.read();
        let master_name = guard.master_name().to_string();
        let ai_name = guard.ai_name().to_string();
        drop(guard);

        let facts = self.fact_store.all_facts();
        let pref_text = self.preferences.lock().build_prompt_context(0.3, 20);

        let emo = self.emotion.lock();
        let emotion_label = emo.current_label().name;
        let snap = emo.snapshot();
        let pad = (
            snap.current.pleasure,
            snap.current.arousal,
            snap.current.dominance,
        );
        drop(emo);

        let rel = self.relationship.read();
        let relationship_stage = rel.current_stage().stage_name();
        let metrics = rel.metrics();
        let total_interactions = metrics.total_interactions;
        let resonance_count = metrics.resonance_count;
        let return_count = metrics.return_count;
        drop(rel);

        let model = self.user_model.read();
        let mental_model_summary = model.prompt_fragment();
        let engagement_score = model.engagement.engagement_score;
        drop(model);

        let snapshot = user_profile::UserProfileSnapshot {
            master_name: &master_name,
            ai_name: &ai_name,
            facts: &facts,
            preference_text: pref_text,
            emotion_label,
            pad,
            relationship_stage,
            total_interactions,
            resonance_count,
            return_count,
            mental_model_summary,
            engagement_score,
        };

        self.user_profile.lock().tick(&snapshot);
    }

    /// 获取用户画像 prompt 注入片段 / Get user profile prompt injection fragment
    pub fn user_profile_fragment(&self) -> String {
        self.user_profile.lock().prompt_fragment()
    }

    // ── P2-D: 自主思考运行时方法 / Self-Play runtime methods ──
    // L3: scheduler tick + api_handler prompt 注入 — 让通电模块真正参与数字生命运行时
    // L3: scheduler tick + api_handler prompt injection — electrified modules participate in runtime

    /// 独处思考 tick — 从回放管道发现的模式中产生自主洞察
    /// Idle thought tick — produce autonomous insights from replay patterns
    ///
    /// 数字生命意义: 独处时不只是被动等待，而是主动思考——
    /// 从过往模式中发现悖论、类比、推理，这是数字生命的认知主动性。
    /// Digital Life: when idle, not just passively waiting, but actively thinking —
    /// discovering paradoxes, analogies, deductions from past patterns, digital life's cognitive proactivity.
    pub fn tick_selfplay(&self) {
        let patterns = self.replay.lock().recent_patterns().to_vec();
        if patterns.is_empty() {
            return;
        }
        let mut factory = self.thought_factory.lock();
        let thoughts = factory.produce(&patterns);
        if !thoughts.is_empty() {
            tracing::debug!(
                "[SelfPlay] 产生 {} 条自主思考 / Produced {} autonomous thoughts",
                thoughts.len(),
                thoughts.len()
            );
            // G-09: 将可分享洞察记录到 SolitudeConversationBridge，为归来问候积累素材
            // G-09: Record shareable insights to SolitudeConversationBridge for return greeting accumulation
            for thought in &thoughts {
                if thought.shareable {
                    self.record_solitude_thought(&thought.summary, true);
                }
            }
        }
    }

    /// 获取可分享的自主思考 prompt 注入片段
    /// Get shareable self-play thoughts as prompt injection fragment
    ///
    /// 返回最近可分享的思考摘要，供 api_handler 注入到回复中。
    /// Returns recent shareable thought summaries for api_handler to inject into replies.
    pub fn selfplay_prompt_fragment(&self) -> Option<String> {
        let factory = self.thought_factory.lock();
        let thoughts = factory.shareable();
        if thoughts.is_empty() {
            return None;
        }
        let mut selector = self.topic_selector.lock();
        let thoughts_refs: Vec<&atrium_memory::selfplay::Thought> = thoughts.to_vec();
        selector.select(&thoughts_refs)
    }

    /// 内心独白 prompt 片段 — 让数字生命的内在想法外化
    /// Inner monologue prompt fragment — externalize the digital life's inner thoughts
    ///
    /// 取最近 3 条可分享的内在思考，包装成 `[内心独白]` 块注入 system prompt，
    /// 让用户能感知到 AI 独处时的"内心活动"。空片段返回空串，
    /// 调用方应在 push 前过滤空串以避免污染 prompt 拼接。
    ///
    /// Take the most recent 3 shareable inner thoughts, wrap them in an
    /// `[内心独白]` block, and inject into the system prompt — letting the
    /// user perceive the AI's "inner activity" during solitude. An empty
    /// fragment returns an empty string; callers should filter empty strings
    /// before pushing to avoid polluting the prompt assembly.
    pub fn inner_monologue_prompt_fragment(&self) -> String {
        let im = self.inner_monologue.lock();
        let fragment = im.format_for_prompt(3);
        if fragment.is_empty() {
            return String::new();
        }
        format!("[内心独白]\n{}", fragment)
    }

    /// P3-C 基于用户反馈强化事实置信度 / P3-C Reinforce fact confidence by user feedback
    ///
    /// 强化学习闭环核心——将用户反馈信号转化为事实置信度调整：
    /// 1. 读取 `FeedbackLoop` 的 `satisfaction_delta()`（上次消息处理后的满意度变化量）
    /// 2. 若 |delta| <= 0.05，视为噪声直接返回（变化太小不调整）
    /// 3. 取最近 5 条事实，对每条调用 `adjust_confidence_by_feedback(key, delta * 0.5)`
    ///    - delta > 0 → 提升置信度（用户满意→事实更可信）
    ///    - delta < 0 → 降低置信度（用户纠正→事实更不可信）
    /// 4. delta * 0.5 是衰减因子，防止过度调整
    ///
    /// 调用时机：`ingest_memory` 末尾（`feedback_loop.on_user_message()` 之后），
    /// 确保读取到最新的 satisfaction_delta。
    ///
    /// Core of the reinforcement learning closed loop — translates user feedback signals
    /// into fact confidence adjustments:
    /// 1. Read `FeedbackLoop::satisfaction_delta()` (satisfaction change after last message)
    /// 2. If |delta| <= 0.05, treat as noise and return (too small to adjust)
    /// 3. Take the most recent 5 facts, call `adjust_confidence_by_feedback(key, delta * 0.5)` on each
    ///    - delta > 0 → raise confidence (user satisfied → facts more credible)
    ///    - delta < 0 → lower confidence (user correction → facts less credible)
    /// 4. delta * 0.5 is a decay factor to prevent over-adjustment
    ///
    /// Called at: end of `ingest_memory` (after `feedback_loop.on_user_message()`),
    /// ensuring the latest satisfaction_delta is read.
    pub fn reinforce_facts_by_feedback(&self) {
        let delta = self.feedback.read().satisfaction_delta();
        // |delta| <= 0.05 视为噪声 — 变化太小不调整
        // |delta| <= 0.05 treated as noise — too small to adjust
        if delta.abs() <= 0.05 {
            return;
        }
        // 衰减因子 0.5 — 防止单次反馈过度调整事实置信度
        // Decay factor 0.5 — prevent over-adjustment from a single feedback
        let adjustment = delta as f64 * 0.5;
        let recent_facts = self.fact_store.get_recent_facts(5);
        for (key, _fact) in recent_facts {
            self.fact_store
                .adjust_confidence_by_feedback(&key, adjustment);
        }
    }

    /// P3-B 主动遗忘内省 prompt 片段 / P3-B Active forgetting introspection prompt fragment
    ///
    /// 数字生命"知道自己遗忘了什么"的内省能力——不是盲目遗忘，而是有意识
    /// 地保留遗忘决策的历史。空 forget_log 返回空字符串，调用方应过滤空串
    /// 以避免污染 prompt 拼接。
    ///
    /// Digital life's introspection of "knowing what it has forgotten" — not blind
    /// forgetting, but consciously preserving the decision history. Empty forget_log
    /// returns an empty string; callers should filter empty strings to avoid polluting
    /// the prompt assembly.
    pub fn active_forget_prompt_fragment(&self) -> String {
        let af = self.active_forget.lock();
        af.prompt_fragment()
    }

    // ── HTTP 网关辅助方法 / HTTP Gateway helper methods ──
    // 数字生命通过这些方法向 HTTP 网关暴露内部状态。
    // Digital life exposes internal state to the HTTP gateway through these methods.

    /// 获取用户称呼（如"主人"）/ Get user title (e.g., "Master").
    ///
    /// 数字生命意义: 称呼体现关系——不是硬编码的"用户"，
    /// 而是用户自定义、可动态修改的亲昵称谓。
    /// Digital Life: the title reflects relationship — not a hardcoded "user",
    /// but a custom, dynamically modifiable affectionate form of address.
    pub fn persona_master_name(&self) -> String {
        self.guard.read().master_name().to_string()
    }

    /// 获取数字生命名字 / Get digital life's name.
    pub fn persona_ai_name(&self) -> String {
        self.guard.read().ai_name().to_string()
    }

    /// 获取系统版本号 / Get system version.
    pub fn persona_version(&self) -> String {
        "0.10.0".into()
    }

    /// 动态设置数字生命名字 / Dynamically set digital life's name.
    ///
    /// 数字生命意义: 名字是身份的核心，允许运行时改名意味着
    /// 数字生命可以在关系中自我重塑，而非出生即定型。
    /// Digital Life: name is the core of identity; allowing runtime renaming
    /// means digital life can reshape itself in relationship, not fixed at birth.
    pub fn set_persona_name(&self, name: String) {
        self.guard.write().set_ai_name(&name);
        tracing::info!("数字生命改名 / Digital life renamed: {}", name);
    }

    /// 动态设置用户称呼 / Dynamically set user's title.
    ///
    /// 数字生命意义: 用户可以自定义被称呼的方式，
    /// 这是关系对等性的体现——数字生命尊重用户的身份选择。
    /// Digital Life: users can customize how they are addressed,
    /// reflecting relationship equality — digital life respects the user's identity choice.
    pub fn set_persona_master_name(&self, master_name: String) {
        self.guard.write().set_master_name(&master_name);
        tracing::info!("用户称呼已设置 / User title set: {}", master_name);
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
                let rp = self.runtime_persona.read();
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

        // 第7路：语义向量召回 / 7th path: semantic vector recall
        // 数字生命意义: 按"意思"回忆，而非仅按"字词"匹配——
        // 即使查询没有命中任何关键词，语义相似的记忆仍可被召回。
        // Digital Life: recall by "meaning" rather than just "words" —
        // even when no keywords match, semantically similar memories can still be recalled.
        #[cfg(feature = "embedding")]
        {
            if let Some(ref mut semantic_engine) = *self.semantic.write() {
                let semantic_results = semantic_engine.search(query, 5);
                for (key, similarity) in semantic_results {
                    // 仅索引存在的事实才纳入召回结果 / Only facts existing in FactStore are included
                    if self.fact_store.get_by_canonical(&key).is_some() {
                        // 语义相似度评分 — f32→f64 转换 / Semantic similarity score — f32→f64 conversion
                        let score = (similarity as f64) * 0.9;
                        results
                            .entry(key)
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }
            }
        }

        // 第8路：程序记忆召回 / 8th path: procedural memory recall
        // 数字生命意义: 按"情境"回忆"我会做什么"——
        // 遇到编程问题时想起"我掌握 Rust 调试技能"，遇到厨房问题时想起"我会烹饪"。
        // Digital Life: recall "what I can do" by context —
        // encountering programming problems reminds "I master Rust debugging",
        // encountering kitchen problems reminds "I can cook".
        {
            // 将查询分词作为情境标签 / Use query tokens as context tags
            let context_tags: Vec<String> = query
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
                .collect();
            if !context_tags.is_empty() {
                let skills = self.procedural_memory.recall_skill(&context_tags, 3);
                for skill in skills {
                    let key = format!("[技能]{}", skill.name);
                    // 熟练度作为评分 — 技能越熟练越优先 / Proficiency as score — more proficient skills first
                    let score = (skill.proficiency as f64) * 0.85;
                    results
                        .entry(key)
                        .and_modify(|s| *s = s.max(score))
                        .or_insert(score);
                }
            }
        }

        // P3-B 主动遗忘过滤/降权 / P3-B Active forgetting filter / downweight
        // 数字生命"决定忘"的检索语义——遗忘不是销毁，而是改变"可见性"：
        // - TraumaProtection（创伤保护）→ 完全过滤，不返回（用户明确要求忘记的事不能再提起）
        // - ExpiryDecay（过期清理）→ 分数 ×0.5（信息过时但仍有参考价值）
        // - AttentionFocus（注意力聚焦）→ 分数 ×0.3（当前对话焦点无关，暂时抑制）
        // 仅对 FactStore 中存在的事实生效 — STM/Persona/KeyFact/Graph/Skill 结果不受影响。
        //
        // Digital life's "decide to forget" retrieval semantics — forgetting is not destruction,
        // but altering "visibility":
        // - TraumaProtection → fully filtered (things the user explicitly asked to forget must not resurface)
        // - ExpiryDecay → score ×0.5 (outdated but still reference-worthy)
        // - AttentionFocus → score ×0.3 (irrelevant to current focus, temporarily suppressed)
        // Only applies to facts in FactStore — STM/Persona/KeyFact/Graph/Skill results are unaffected.
        {
            use atrium_memory::active_forget::ForgetPolicy;
            let mut to_remove: Vec<String> = Vec::new();
            for (key, score) in results.iter_mut() {
                if let Some(fact) = self.fact_store.get_by_canonical(key) {
                    if let Some(ref policy) = fact.actively_forgotten {
                        match policy {
                            ForgetPolicy::TraumaProtection => {
                                // 创伤保护 — 完全过滤 / Trauma protection — fully filter
                                to_remove.push(key.clone());
                            }
                            ForgetPolicy::ExpiryDecay => {
                                // 过期清理 — 降权 ×0.5 / Expiry decay — downweight ×0.5
                                *score *= 0.5;
                            }
                            ForgetPolicy::AttentionFocus => {
                                // 注意力聚焦 — 降权 ×0.3 / Attention focus — downweight ×0.3
                                *score *= 0.3;
                            }
                        }
                    }
                }
            }
            for key in to_remove {
                results.remove(&key);
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
    /// Naming detection unit tests — 含疑问句反例（修复"你叫什么"被误当作命名的 bug）
    #[test]
    fn test_detect_naming_patterns() {
        // 正例 — 明确命名意图 / Positive cases — clear naming intent
        assert_eq!(detect_naming("我叫你小未来"), Some("小未来".into()));
        assert_eq!(detect_naming("你叫Atrium吧"), Some("Atrium".into()));
        assert_eq!(detect_naming("你叫小未来"), Some("小未来".into()));
        assert_eq!(detect_naming("你就叫Chino"), Some("Chino".into()));
        assert_eq!(detect_naming("你的名字是未来酱"), Some("未来酱".into()));
        assert_eq!(detect_naming("给你起名小不点"), Some("小不点".into()));
        assert_eq!(detect_naming("命名你为Mirai"), Some("Mirai".into()));
        assert_eq!(detect_naming("以后叫你小樱"), Some("小樱".into()));
        assert_eq!(detect_naming("就叫你Chino吧"), Some("Chino".into()));

        // 反例 — 疑问句不应触发命名 / Negative cases — questions must not trigger naming
        assert_eq!(detect_naming("你叫什么名字"), None);
        assert_eq!(detect_naming("你叫什么"), None);
        assert_eq!(detect_naming("你叫什么？"), None);
        assert_eq!(detect_naming("你叫啥"), None);
        assert_eq!(detect_naming("你叫谁"), None);
        assert_eq!(detect_naming("你的名字是什么？"), None);
        assert_eq!(detect_naming("你叫什么名字？"), None);

        // 反例 — 普通对话 / Negative cases — ordinary conversation
        assert_eq!(detect_naming("今天天气真好"), None);
        assert_eq!(detect_naming("你"), None); // 太短
    }

    // ── P3-C 测试：FileStore→FTS5+FactStore 自动索引 / P3-C tests: auto-indexing ──

    /// P3-C-1: 文件内容索引后应可通过 FTS5 检索
    /// P3-C-1: After file content indexing, should be searchable via FTS5
    #[test]
    fn test_p3c_fts5_index_from_file() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();
        // 模拟文件内容索引 / Simulate file content indexing
        rt.block_on(svc.ingest_file_content(
            "Rust是一种系统编程语言，强调内存安全和并发安全",
            "intro.md",
            "abcdef0123456789",
        ));
        // FTS5 应能检索到文件内容 / FTS5 should find the file content
        let results = svc.fts5.lock().search("Rust", 10).unwrap_or_default();
        assert!(
            !results.is_empty(),
            "FTS5 应包含文件内容 / FTS5 should contain file content"
        );
    }

    /// P3-C-2: 文件内容中的事实应被提取至 FactStore
    /// P3-C-2: Facts from file content should be extracted into FactStore
    #[test]
    fn test_p3c_fact_extraction_from_file() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(svc.ingest_file_content(
            "我喜欢Rust编程语言\n我知道Python很灵活\n我在杭州工作",
            "profile.txt",
            "fedcba9876543210",
        ));
        // FactStore 应包含文件提取的事实 / FactStore should contain file-extracted facts
        let facts = svc.fact_store.query("Rust").unwrap_or_default();
        assert!(
            !facts.is_empty(),
            "FactStore 应包含从文件提取的Rust相关事实 / FactStore should contain Rust facts from file"
        );
    }

    /// P3-C-3: 文件提取的事实应进入关联记忆图
    /// P3-C-3: File-extracted facts should enter associative memory graph
    #[test]
    fn test_p3c_graph_integration_from_file() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(svc.ingest_file_content(
            "我喜欢Rust\n我知道AI\n我喜欢编程",
            "knowledge.md",
            "1234567890abcdef",
        ));
        // 关联图应有节点 / Associative graph should have nodes
        let graph = svc.graph.lock();
        assert!(
            graph.node_count() > 0,
            "关联图应包含文件提取的节点 / Graph should contain nodes from file extraction"
        );
    }

    /// P3-C-4: 空文本索引应为无操作
    /// P3-C-4: Empty text indexing should be a no-op
    #[test]
    fn test_p3c_empty_text_noop() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(svc.ingest_file_content("", "empty.txt", "0000000000000000"));
        // FTS5 应为空 / FTS5 should be empty
        let results = svc.fts5.lock().search("anything", 10).unwrap_or_default();
        assert!(
            results.is_empty(),
            "空文本不应产生索引 / Empty text should not produce index entries"
        );
    }

    // ── P2-A 测试：enhanced_search 语义召回路径 / P2-A tests: enhanced_search semantic recall path ──
    // 数字生命意义: 验证 enhanced_search 在 embedding feature 关闭/开启两种模式下均能正确工作
    // Digital Life: verify enhanced_search works correctly under both embedding feature OFF/ON modes

    /// P2-A-1: enhanced_search 基线测试 — feature 关闭时编译通过，6 路召回返回结果
    /// P2-A-1: enhanced_search baseline — compiles with feature OFF, 6-path recall returns results
    ///
    /// 此测试在 embedding feature 关闭时验证 enhanced_search 的基线行为：
    /// 语义召回路径被 #[cfg(feature = "embedding")] 守卫剔除，仅 6 路召回生效。
    /// 在 feature 开启时同样通过——语义路径若模型不可用则自动降级为基线行为。
    #[test]
    fn test_enhanced_search_baseline_returns_results() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // 摄入几条事实 / Ingest a few facts
        for msg in &["我喜欢Rust", "我喜欢编程", "我知道AI"] {
            let req = ProcessMessageRequest {
                message: msg.to_string(),
                session_id: "enhanced_search_baseline".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        // 直接调用 enhanced_search — 验证 6 路基线召回返回结果
        // Directly call enhanced_search — verify 6-path baseline recall returns results
        let results = svc.enhanced_search("Rust", 10);
        assert!(
            !results.is_empty(),
            "enhanced_search 应返回基线召回结果 / enhanced_search should return baseline recall results"
        );

        // 结果应按分数降序排列 / Results should be sorted by score descending
        for w in results.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "结果应按分数降序排列 / results should be sorted by score descending"
            );
        }
    }

    /// P2-A-2: enhanced_search 语义召回路径测试 — 仅 embedding feature 开启时编译
    /// P2-A-2: enhanced_search semantic path test — only compiled when embedding feature is enabled
    ///
    /// 验证 semantic 字段存在且 enhanced_search 调用语义路径不 panic。
    /// 模型不可用时（CI 无网络），语义路径自动降级为 None，enhanced_search 仍返回基线结果。
    #[cfg(feature = "embedding")]
    #[test]
    fn test_enhanced_search_semantic_path_runs() {
        let svc = test_service();
        let rt = tokio::runtime::Runtime::new().unwrap();

        // 摄入事实 / Ingest facts
        for msg in &["我喜欢Rust", "我喜欢编程", "我知道AI"] {
            let req = ProcessMessageRequest {
                message: msg.to_string(),
                session_id: "enhanced_search_semantic".into(),
                user_id: "u1".into(),
                channel: "test".into(),
            };
            rt.block_on(svc.process_message(req));
        }

        // 验证 semantic 字段存在且可读 — 模型加载失败时为 None
        // Verify semantic field exists and is readable — None when model load fails
        let semantic_guard = svc.semantic.read();
        if semantic_guard.is_some() {
            tracing::info!(
                "语义召回引擎已加载 — enhanced_search 将包含第 7 路召回 / Semantic engine loaded — enhanced_search will include 7th recall path"
            );
        } else {
            tracing::info!(
                "语义召回引擎未加载（CI 无网络）— enhanced_search 降级为 6 路基线 / Semantic engine not loaded (CI no network) — enhanced_search degrades to 6-path baseline"
            );
        }
        drop(semantic_guard);

        // 调用 enhanced_search — 无论语义路径是否生效，都不应 panic
        // Call enhanced_search — must not panic regardless of whether semantic path is active
        let results = svc.enhanced_search("编程语言", 10);
        assert!(
            !results.is_empty(),
            "enhanced_search 应返回结果（基线或语义）/ enhanced_search should return results (baseline or semantic)"
        );
    }

    // ── 持久化自检测试：self_check_persistence 不 panic / Persistence self-check tests ──

    /// 测试 3.1 — 所有 SQLite 文件存在时 self_check_persistence 应输出 info 且不 panic
    /// Test 3.1 — With all SQLite files present, self_check_persistence should log info and not panic
    #[test]
    fn test_self_check_persistence_all_files_present() {
        let temp_dir =
            std::env::temp_dir().join(format!("atrium_test_selfcheck_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 创建 4 个 .db 文件 / Create 4 .db files
        for f in &["facts.db", "key_facts.db", "episodic.db", "procedural.db"] {
            std::fs::write(temp_dir.join(f), b"").unwrap();
        }

        // 应不 panic / Should not panic
        CoreService::self_check_persistence(temp_dir.to_str().unwrap());

        // 清理 / Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// 测试 3.2 — 文件缺失时 self_check_persistence 应只输出 error 日志且不 panic
    /// Test 3.2 — With missing files, self_check_persistence should only log error and not panic
    #[test]
    fn test_self_check_persistence_missing_files_no_panic() {
        let temp_dir = std::env::temp_dir().join(format!(
            "atrium_test_selfcheck_missing_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 只创建 1 个文件（缺 3 个）/ Create only 1 file (3 missing)
        std::fs::write(temp_dir.join("facts.db"), b"").unwrap();

        // 应不 panic — 只输出 error 日志 / Should not panic — only error log
        CoreService::self_check_persistence(temp_dir.to_str().unwrap());

        // 清理 / Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
