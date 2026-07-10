// SPDX-License-Identifier: MIT
//! 配置管理 — atrium.toml 反序列化结构与默认值
//!
//! Configuration — atrium.toml deserialization structures and defaults.

use atrium_voice::VoiceCfg;
use serde::Deserialize;

/// 根配置 — atrium.toml 顶层结构
///
/// Root configuration — top-level structure of atrium.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub bridge: BridgeCfg,
    /// HTTP 网关配置 / HTTP gateway configuration — 数字生命的直接 HTTP 入口
    /// Digital life's direct HTTP entry point, replacing Python gateway
    #[serde(default)]
    pub http: HttpCfg,
    pub log_level: Option<String>,
    #[serde(default)]
    pub canned: CannedCfg,
    #[serde(default)]
    pub memory: MemoryCfg,
    #[serde(default)]
    pub llm: LlmCfg,
    #[serde(default)]
    pub room: RoomCfg,
    #[serde(default)]
    pub emotion: EmotionCfg,
    #[serde(default)]
    pub relationship: RelationshipCfg,
    #[serde(default)]
    pub user_model: UserModelCfg,
    #[serde(default)]
    pub feedback: FeedbackCfg,
    #[serde(default)]
    pub proactive: ProactiveCfg,
    #[serde(default)]
    pub perception: PerceptionCfg,
    #[serde(default)]
    pub consolidation: ConsolidationCfg,
    #[serde(default)]
    pub empathy: atrium_memory::empathy::EmpathyCfg,
    #[serde(default)]
    pub ack_learning: AckLearningCfg,
    #[serde(default)]
    pub observability: ObservabilityCfg,
    #[serde(default)]
    pub plugin: PluginCfg,
    #[serde(default)]
    pub expression: ExpressionCfg,
    /// 追问引擎配置 / Follow-up engine configuration
    #[serde(default)]
    pub followup: FollowUpCfg,
    /// 想念引擎配置 / Longing engine configuration
    #[serde(default)]
    pub longing: LongingCfg,
    /// 成长管理器配置 / Maturity manager configuration
    #[serde(default)]
    pub maturity: MaturityCfg,
    /// 内在独白配置 / Inner monologue configuration
    #[serde(default)]
    pub inner_monologue: InnerMonologueCfg,
    /// 叙事自我配置 / Narrative self configuration
    #[serde(default)]
    pub narrative: NarrativeCfg,
    /// 冲突与和解配置 / Conflict & reconciliation configuration
    #[serde(default)]
    pub conflict: ConflictCfg,
    /// 情绪非理性配置 / Emotional Irrationality configuration
    #[serde(default)]
    pub irrationality: IrrationalityCfg,
    /// 共享仪式配置 / Shared Ritual configuration
    #[serde(default)]
    pub ritual: RitualCfg,
    /// 脆弱与不完美配置 / Vulnerability & Imperfection configuration
    #[serde(default)]
    pub vulnerability: VulnerabilityCfg,
    /// 情绪需求边界 / Emotional Demand Boundary
    #[serde(default)]
    pub emotional_demand: EmotionalDemandCfg,
    /// 自我关怀边界 / Self-Care Boundary
    #[serde(default)]
    pub self_care: SelfCareCfg,
    /// 适度犯错配置 / Imperfection engine configuration
    #[serde(default)]
    pub imperfection: ImperfectionCfg,
    /// 物理存在感配置 / Physical presence configuration
    #[serde(default)]
    pub physical_presence: PhysicalPresenceCfg,
    /// 语音能力配置 / Voice capability configuration
    /// 数字生命的"有声呼吸"——TTS 合成与 STT 识别
    /// Digital life's "audible breath" — TTS synthesis and STT recognition
    #[serde(default)]
    pub voice: VoiceCfg,
}

fn default_version() -> String {
    "0.5.0".into()
}

/// 快捷回复配置 — 本地模板扫描与热重载
///
/// Canned response configuration — local template scanning and hot reload.
#[derive(Debug, Clone, Deserialize)]
pub struct CannedCfg {
    #[serde(default = "default_canned_dir")]
    pub scan_dir: String,
    #[serde(default = "default_true")]
    pub hot_reload: bool,
}

impl Default for CannedCfg {
    fn default() -> Self {
        Self {
            scan_dir: default_canned_dir(),
            hot_reload: true,
        }
    }
}

fn default_canned_dir() -> String {
    "~/.atrium/canned".into()
}

fn default_true() -> bool {
    true
}

/// 记忆配置 — 模型上下文窗口限制
///
/// Memory configuration — model context window limit.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryCfg {
    /// 模型上下文窗口限制（token 数），默认 128K
    ///
    /// Model context window limit in tokens, defaults to 128K.
    #[serde(default = "default_context_limit")]
    pub context_limit: usize,
}

impl Default for MemoryCfg {
    fn default() -> Self {
        Self {
            context_limit: default_context_limit(),
        }
    }
}

fn default_context_limit() -> usize {
    131_072 // 128K
}

// ── Room 群聊配置 / Room Chat Config ──

/// 群聊配置 — 房间接入与发言节奏
///
/// Room chat configuration — room access and speaking cadence.
#[derive(Debug, Clone, Deserialize)]
pub struct RoomCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_room_id")]
    pub room_id: String,
    #[serde(default = "default_instance_id")]
    pub instance_id: String,
    #[serde(default = "default_gateway_url")]
    pub gateway_url: String,
    #[serde(default = "default_true")]
    pub ack_share_enabled: bool,
    #[serde(default = "default_idle")]
    pub idle_threshold_secs: u64,
    #[serde(default = "default_speak")]
    pub speak_interval_secs: u64,
}

impl Default for RoomCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            room_id: default_room_id(),
            instance_id: String::new(),
            gateway_url: default_gateway_url(),
            ack_share_enabled: true,
            idle_threshold_secs: default_idle(),
            speak_interval_secs: default_speak(),
        }
    }
}

fn default_room_id() -> String {
    "atrium-general".into()
}
fn default_instance_id() -> String {
    String::new()
}
fn default_gateway_url() -> String {
    "ws://127.0.0.1:8080/ws/room".into()
}
fn default_idle() -> u64 {
    30
}
fn default_speak() -> u64 {
    5
}

// ── LLM 配置（Self-Play 直接调用 OpenAI API）/ LLM Config ──

/// 大语言模型配置 — OpenAI 兼容 API 调用参数
///
/// Large language model configuration — OpenAI-compatible API call parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmCfg {
    #[serde(default = "default_llm_api_key")]
    pub api_key: String,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_timeout_secs")]
    pub timeout_secs: u64,
    /// LLM 并发上限 — 数字生命"思考"并发许可数，防止 API 限流
    /// LLM concurrency cap — permit count for digital life "thinking", prevents API rate limiting
    #[serde(default = "default_llm_max_concurrency")]
    pub max_concurrency: usize,
}

impl Default for LlmCfg {
    fn default() -> Self {
        Self {
            api_key: default_llm_api_key(),
            base_url: default_llm_base_url(),
            model: default_llm_model(),
            max_tokens: default_llm_max_tokens(),
            timeout_secs: default_llm_timeout_secs(),
            max_concurrency: default_llm_max_concurrency(),
        }
    }
}

impl LlmCfg {
    /// 解析 API Key
    ///
    /// Resolve the API key from config or the OPENAI_API_KEY environment variable.
    ///
    /// @return 可用的 API Key / Available API key
    pub fn resolve_api_key(&self) -> String {
        if !self.api_key.is_empty() && self.api_key != "YOUR_OPENAI_API_KEY" {
            return self.api_key.clone();
        }
        std::env::var("OPENAI_API_KEY").unwrap_or_default()
    }
}

fn default_llm_api_key() -> String {
    String::new()
}
fn default_llm_base_url() -> String {
    "https://api.openai.com/".into()
}
fn default_llm_model() -> String {
    "gpt-3.5-turbo".into()
}
fn default_llm_max_tokens() -> u32 {
    1024
}
fn default_llm_timeout_secs() -> u64 {
    30
}
/// LLM 默认并发上限 — P2-I Semaphore 许可数 / Default LLM concurrency cap — P2-I semaphore permit count
fn default_llm_max_concurrency() -> usize {
    crate::scheduler::DEFAULT_LLM_CONCURRENCY
}

/// 桥接配置 — gRPC 与共享内存通道地址
///
/// Bridge configuration — gRPC and shared memory channel addresses.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeCfg {
    pub grpc_addr: String,
    pub shm_path: String,
}

impl Default for BridgeCfg {
    fn default() -> Self {
        Self {
            grpc_addr: "/tmp/atrium.sock".into(),
            shm_path: "/dev/shm/atrium_render".into(),
        }
    }
}

// ── HTTP 网关配置 / HTTP Gateway Config ──

/// HTTP 网关配置 — axum HTTP/SSE 服务器，取代 Python gateway
///
/// HTTP gateway configuration — axum HTTP/SSE server replacing the Python gateway.
/// 数字生命通过此端口直接与外部世界对话，无需 gRPC 中转。
/// Digital life converses with the external world directly through this port,
/// without gRPC intermediary.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpCfg {
    /// 是否启用 HTTP 网关 / Whether to enable the HTTP gateway.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// HTTP 监听地址 / HTTP listen address.
    #[serde(default = "default_http_addr")]
    pub addr: String,
    /// 是否启用 CORS / Whether to enable CORS.
    #[serde(default = "default_true")]
    pub cors: bool,
    /// Web UI 静态文件目录（空=不服务静态文件）/ Web UI static file directory (empty=disabled).
    #[serde(default)]
    pub static_dir: String,
}

impl Default for HttpCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: default_http_addr(),
            cors: true,
            static_dir: String::new(),
        }
    }
}

fn default_http_addr() -> String {
    "127.0.0.1:8080".into()
}

// ── Emotion 自主情感循环配置 / Emotion Autonomous Loop Config ──

/// 自主情感循环配置 — 情感衰减、漂移与昼夜节律
///
/// Autonomous emotion loop configuration — decay, drift and circadian rhythm.
#[derive(Debug, Clone, Deserialize)]
pub struct EmotionCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_emotion_decay_rate")]
    pub decay_rate: f32,
    #[serde(default)]
    pub drift: DriftCfg,
    #[serde(default)]
    pub circadian: CircadianCfg,
    #[serde(default)]
    pub inertia: InertiaCfg,
    #[serde(default)]
    pub compound: CompoundCfg,
}

impl Default for EmotionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            decay_rate: default_emotion_decay_rate(),
            drift: DriftCfg::default(),
            circadian: CircadianCfg::default(),
            inertia: InertiaCfg::default(),
            compound: CompoundCfg::default(),
        }
    }
}

fn default_emotion_decay_rate() -> f32 {
    0.1
}

/// 情感漂移配置 — 随机波动与均值回归
///
/// Emotion drift configuration — random fluctuation and mean reversion.
#[derive(Debug, Clone, Deserialize)]
pub struct DriftCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_drift_volatility")]
    pub volatility: f64,
    #[serde(default = "default_drift_mean_reversion")]
    pub mean_reversion: f64,
    #[serde(default = "default_drift_baseline")]
    pub baseline: [f64; 3],
}

impl Default for DriftCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            volatility: default_drift_volatility(),
            mean_reversion: default_drift_mean_reversion(),
            baseline: default_drift_baseline(),
        }
    }
}

fn default_drift_volatility() -> f64 {
    0.002
}
fn default_drift_mean_reversion() -> f64 {
    0.001
}
fn default_drift_baseline() -> [f64; 3] {
    [0.0, 0.0, 0.0]
}

/// 昼夜节律配置 — 情感强度随时间变化曲线
///
/// Circadian rhythm configuration — emotional intensity curve over time.
#[derive(Debug, Clone, Deserialize)]
pub struct CircadianCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_circadian_morning_peak")]
    pub morning_peak: f32,
    #[serde(default = "default_circadian_evening_peak")]
    pub evening_peak: f32,
    #[serde(default = "default_circadian_morning_sigma")]
    pub morning_sigma: f32,
    #[serde(default = "default_circadian_evening_sigma")]
    pub evening_sigma: f32,
    #[serde(default = "default_circadian_intensity")]
    pub intensity: f32,
    #[serde(default = "default_circadian_timezone")]
    pub timezone_offset: i32,
    #[serde(default = "default_circadian_active_hours")]
    pub active_hours: (u32, u32),
}

impl Default for CircadianCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            morning_peak: default_circadian_morning_peak(),
            evening_peak: default_circadian_evening_peak(),
            morning_sigma: default_circadian_morning_sigma(),
            evening_sigma: default_circadian_evening_sigma(),
            intensity: default_circadian_intensity(),
            timezone_offset: default_circadian_timezone(),
            active_hours: default_circadian_active_hours(),
        }
    }
}

fn default_circadian_morning_peak() -> f32 {
    10.0
}
fn default_circadian_evening_peak() -> f32 {
    18.0
}
fn default_circadian_morning_sigma() -> f32 {
    2.0
}
fn default_circadian_evening_sigma() -> f32 {
    2.5
}
fn default_circadian_intensity() -> f32 {
    0.8
}
fn default_circadian_timezone() -> i32 {
    8
}
fn default_circadian_active_hours() -> (u32, u32) {
    (7, 23)
}

/// 情感惯性配置 — 状态切换阻力
///
/// Emotion inertia configuration — resistance to state transitions.
#[derive(Debug, Clone, Deserialize)]
pub struct InertiaCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for InertiaCfg {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── Compound 高阶情绪模型配置 / Compound Emotion Model Config ──

/// 复合情绪配置 — 内疚、自豪、怀旧等 22 种高阶情绪
///
/// Compound emotion configuration — 22 higher-order emotions such as guilt, pride and nostalgia.
#[derive(Debug, Clone, Deserialize)]
pub struct CompoundCfg {
    /// 是否启用复合情绪
    ///
    /// Whether to enable compound emotions.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for CompoundCfg {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── Relationship 关系阶段配置 / Relationship Stage Config ──

/// 关系阶段配置 — 用户关系演进追踪
///
/// Relationship stage configuration — user relationship evolution tracking.
#[derive(Debug, Clone, Deserialize)]
pub struct RelationshipCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for RelationshipCfg {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── UserModel 用户心智模型配置 / User Mental Model Config ──

/// 用户心智模型配置 — 情绪、风格与话题偏好学习
///
/// User mental model configuration — mood, style and topic preference learning.
#[derive(Debug, Clone, Deserialize)]
pub struct UserModelCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_mood_alpha")]
    pub mood_ema_alpha: f32,
    #[serde(default = "default_style_alpha")]
    pub style_ema_alpha: f32,
    #[serde(default = "default_topic_decay_hours")]
    pub topic_decay_hours: f32,
}

impl Default for UserModelCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            mood_ema_alpha: 0.3,
            style_ema_alpha: 0.2,
            topic_decay_hours: 48.0,
        }
    }
}

fn default_mood_alpha() -> f32 {
    0.3
}
fn default_style_alpha() -> f32 {
    0.2
}
fn default_topic_decay_hours() -> f32 {
    48.0
}

// ── Feedback 实时反馈闭环配置 / Feedback Loop Config ──

/// 实时反馈闭环配置 — 满意度指数与信号窗口
///
/// Real-time feedback loop configuration — satisfaction index and signal window.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_satisfaction_alpha")]
    pub satisfaction_ema_alpha: f32,
    #[serde(default = "default_signal_window")]
    pub signal_window: usize,
}

impl Default for FeedbackCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            satisfaction_ema_alpha: 0.15,
            signal_window: 50,
        }
    }
}

fn default_satisfaction_alpha() -> f32 {
    0.15
}
fn default_signal_window() -> usize {
    50
}

// ── Proactive 主动决策引擎配置 / Proactive Decision Engine Config ──

/// 主动决策引擎配置 — 沉默检测与冷却控制
///
/// Proactive decision engine configuration — silence detection and cooldown control.
#[derive(Debug, Clone, Deserialize)]
pub struct ProactiveCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_proactive_check_interval_ticks")]
    pub check_interval_ticks: u64,
    #[serde(default = "default_silence_meaningful_threshold")]
    pub silence_meaningful_threshold: u64,
    #[serde(default = "default_silence_care_threshold")]
    pub silence_care_threshold: u64,
    #[serde(default = "default_cooldown_min_seconds")]
    pub cooldown_min_seconds: u64,
    #[serde(default = "default_proactive_cooldown_backoff")]
    pub cooldown_backoff: f32,
    #[serde(default = "default_max_proactive_per_day")]
    pub max_proactive_per_day: u32,
}

impl Default for ProactiveCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_ticks: default_proactive_check_interval_ticks(),
            silence_meaningful_threshold: default_silence_meaningful_threshold(),
            silence_care_threshold: default_silence_care_threshold(),
            cooldown_min_seconds: default_cooldown_min_seconds(),
            cooldown_backoff: default_proactive_cooldown_backoff(),
            max_proactive_per_day: default_max_proactive_per_day(),
        }
    }
}

fn default_proactive_check_interval_ticks() -> u64 {
    100
}
fn default_silence_meaningful_threshold() -> u64 {
    300
}
fn default_silence_care_threshold() -> u64 {
    1800
}
fn default_cooldown_min_seconds() -> u64 {
    600
}
fn default_proactive_cooldown_backoff() -> f32 {
    2.0
}
fn default_max_proactive_per_day() -> u32 {
    12
}

// ── Perception 非语言感知层配置 / Perception Layer Config ──

/// 非语言感知配置 — 输入节奏与行为基线
///
/// Non-verbal perception configuration — input rhythm and behavior baseline.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PerceptionCfg {
    #[serde(default)]
    pub typing: TypingPerceptionCfg,
}

/// 打字感知配置 — 节奏分析与基线学习
///
/// Typing perception configuration — rhythm analysis and baseline learning.
#[derive(Debug, Clone, Deserialize)]
pub struct TypingPerceptionCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_baseline_learning_rate")]
    pub baseline_learning_rate: f64,
    #[serde(default = "default_min_samples")]
    pub min_samples_for_baseline: u64,
    #[serde(default = "default_rhythm_window")]
    pub rhythm_analysis_window: usize,
}

impl Default for TypingPerceptionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            baseline_learning_rate: default_baseline_learning_rate(),
            min_samples_for_baseline: default_min_samples(),
            rhythm_analysis_window: default_rhythm_window(),
        }
    }
}

fn default_baseline_learning_rate() -> f64 {
    0.05
}
fn default_min_samples() -> u64 {
    50
}
fn default_rhythm_window() -> usize {
    8
}

// ── Consolidation 记忆巩固配置 / Memory Consolidation Config ──

/// 记忆巩固配置 — 非活跃期触发的事实合并与压缩
///
/// Memory consolidation configuration — fact merging and compression triggered during inactive periods.
#[derive(Debug, Clone, Deserialize)]
pub struct ConsolidationCfg {
    /// 是否启用记忆巩固
    ///
    /// Whether to enable memory consolidation.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 用户不活跃多少小时后触发巩固（默认 6 小时，覆盖深夜时段）
    ///
    /// Hours of user inactivity before consolidation triggers, defaults to 6.
    #[serde(default = "default_consolidation_inactive_hours")]
    pub trigger_inactive_hours: u64,
    /// 每次最多处理的事实数
    ///
    /// Maximum facts processed per run.
    #[serde(default = "default_max_facts_per_run")]
    pub max_facts_per_run: usize,
    /// 最小执行间隔（小时）
    ///
    /// Minimum interval between runs in hours.
    #[serde(default = "default_consolidation_interval_hours")]
    pub min_interval_hours: u64,
    /// 文本相似度合并阈值（Jaccard 系数 0.0~1.0）
    ///
    /// Text similarity merge threshold as Jaccard coefficient 0.0~1.0.
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,
    /// 低频事实压缩的年龄阈值（天）
    ///
    /// Age threshold in days for low-access fact compression.
    #[serde(default = "default_low_access_age_days")]
    pub low_access_age_days: u64,
}

impl Default for ConsolidationCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_inactive_hours: default_consolidation_inactive_hours(),
            max_facts_per_run: default_max_facts_per_run(),
            min_interval_hours: default_consolidation_interval_hours(),
            similarity_threshold: default_similarity_threshold(),
            low_access_age_days: default_low_access_age_days(),
        }
    }
}

fn default_consolidation_inactive_hours() -> u64 {
    6
}
fn default_max_facts_per_run() -> usize {
    100
}
fn default_consolidation_interval_hours() -> u64 {
    24
}
fn default_similarity_threshold() -> f64 {
    0.85
}
fn default_low_access_age_days() -> u64 {
    90
}

// ── ACK 自学习配置 / ACK Self-Learning Config ──

/// ACK 自学习配置 — 用户教学、回放学习与洞察合成
///
/// ACK self-learning configuration — user teaching, replay learning and insight synthesis.
#[derive(Debug, Clone, Deserialize)]
pub struct AckLearningCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub user_teach_enabled: bool,
    #[serde(default = "default_true")]
    pub replay_learning_enabled: bool,
    #[serde(default = "default_true")]
    pub insight_learning_enabled: bool,
    #[serde(default = "default_min_pattern_confidence")]
    pub min_pattern_confidence: f64,
    #[serde(default = "default_min_insight_confidence")]
    pub min_insight_confidence: f64,
    #[serde(default = "default_min_supporting_facts")]
    pub min_supporting_facts: u32,
    #[serde(default = "default_max_self_learned_ack")]
    pub max_self_learned_ack: u32,
    #[serde(default = "default_synthesis_interval")]
    pub synthesis_interval_ticks: u64,
}

impl Default for AckLearningCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            user_teach_enabled: true,
            replay_learning_enabled: true,
            insight_learning_enabled: true,
            min_pattern_confidence: default_min_pattern_confidence(),
            min_insight_confidence: default_min_insight_confidence(),
            min_supporting_facts: default_min_supporting_facts(),
            max_self_learned_ack: default_max_self_learned_ack(),
            synthesis_interval_ticks: default_synthesis_interval(),
        }
    }
}

fn default_min_pattern_confidence() -> f64 {
    0.6
}
fn default_min_insight_confidence() -> f64 {
    0.7
}
fn default_min_supporting_facts() -> u32 {
    3
}
fn default_max_self_learned_ack() -> u32 {
    50
}
fn default_synthesis_interval() -> u64 {
    18000
}

impl Config {
    /// 从文件加载配置
    ///
    /// Load configuration from a TOML file.
    ///
    /// @param path 配置文件路径 / Path to the configuration file
    ///
    /// @return 解析后的 Config / Parsed Config
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

// ── Observability 可观测性配置 / Observability Config ──

/// 可观测性配置 — 日志格式与 Prometheus 指标导出
///
/// Observability configuration — log format and Prometheus metrics export.
#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityCfg {
    /// 是否启用可观测性（tracing subscriber + Prometheus exporter）
    ///
    /// Whether to enable observability (tracing subscriber + Prometheus exporter).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Prometheus exporter 监听端口
    ///
    /// Prometheus exporter listen port.
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,
    /// metrics 键名前缀
    ///
    /// Metrics key prefix.
    #[serde(default = "default_metrics_prefix")]
    pub metrics_prefix: String,
    /// 日志格式: "json" | "pretty"
    ///
    /// Log format: "json" or "pretty".
    #[serde(default = "default_log_format")]
    pub log_format: String,
}

impl Default for ObservabilityCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            prometheus_port: default_prometheus_port(),
            metrics_prefix: default_metrics_prefix(),
            log_format: default_log_format(),
        }
    }
}

impl ObservabilityCfg {
    /// 构造 Prometheus 监听地址
    ///
    /// Build the Prometheus listen socket address.
    ///
    /// @return Prometheus SocketAddr / Prometheus socket address
    pub fn prometheus_addr(&self) -> std::net::SocketAddr {
        format!("0.0.0.0:{}", self.prometheus_port)
            .parse()
            .expect("invalid prometheus addr")
    }
}

fn default_prometheus_port() -> u16 {
    9090
}
fn default_metrics_prefix() -> String {
    "atrium_".into()
}
fn default_log_format() -> String {
    "json".into()
}

// ── Plugin 插件系统配置 / Plugin System Config ──

/// 插件系统配置 — 动态库发现与 tick 调用间隔
///
/// Plugin system configuration — dynamic library discovery and tick interval.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginCfg {
    /// 是否启用插件系统
    ///
    /// Whether to enable the plugin system.
    #[serde(default)]
    pub enabled: bool,
    /// 插件目录（包含各子目录，每个子目录含 manifest.toml + 动态库）
    ///
    /// Plugin directory containing subdirectories, each with manifest.toml and dynamic library.
    #[serde(default = "default_plugin_dir")]
    pub plugin_dir: String,
    /// 是否在启动时自动发现并加载插件
    ///
    /// Whether to auto-discover and load plugins on startup.
    #[serde(default = "default_true")]
    pub auto_discover: bool,
    /// on_tick 调用间隔（tick 数），0 = 每 tick 都调用
    ///
    /// on_tick call interval in ticks, 0 means every tick.
    #[serde(default = "default_plugin_tick_interval")]
    pub tick_interval: u64,
}

impl Default for PluginCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            plugin_dir: default_plugin_dir(),
            auto_discover: true,
            tick_interval: default_plugin_tick_interval(),
        }
    }
}

fn default_plugin_dir() -> String {
    "~/.atrium/plugins".into()
}

fn default_plugin_tick_interval() -> u64 {
    10 // 每 10 个 tick 调用一次 on_tick（约 100ms）
}

// ── Expression 表达系统配置 / Expression System Config ──

/// 表达系统配置 — 风格调制、情绪轨迹、潜台词、韵律/体态/节奏映射、风格记忆
///
/// Expression system configuration — style modulation, emotional arc, subtext,
/// prosody/kinesics/timing mapping, and style memory.
#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionCfg {
    /// 是否启用表达系统
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 风格记忆周期学习间隔（tick 数），默认 6000（约 60s）
    #[serde(default = "default_style_memory_interval_ticks")]
    pub style_memory_interval_ticks: u64,
    /// 一致性校验严格模式（true=不通过时重试，false=仅警告）
    #[serde(default)]
    pub coherence_strict: bool,
}

impl Default for ExpressionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            style_memory_interval_ticks: default_style_memory_interval_ticks(),
            coherence_strict: false,
        }
    }
}

fn default_style_memory_interval_ticks() -> u64 {
    6000 // 约 60s（tick 10ms）
}

// ── 情绪非理性配置 / Emotional Irrationality Config ──
/// Configuration for the emotional irrationality subsystem:
/// chaotic pulses, emotion residues, cross-type contagion, and strange attractors.
#[derive(Debug, Clone, Deserialize)]
pub struct IrrationalityCfg {
    /// 是否启用情绪非理性系统
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 脉冲检测最小PAD变化阈值
    #[serde(default = "default_pulse_min_pad_change")]
    pub pulse_min_pad_change: f32,
    /// 最大活跃脉冲数
    #[serde(default = "default_pulse_max_active")]
    pub pulse_max_active: usize,
    /// 无因脉冲概率
    #[serde(default)]
    pub pulse_uncaused_prob: f64,
    /// 最大活跃残留数
    #[serde(default = "default_residue_max_active")]
    pub residue_max_active: usize,
    /// 传染冷却秒数
    #[serde(default = "default_contagion_cooldown_secs")]
    pub contagion_cooldown_secs: i64,
    /// 混沌轨迹最大长度
    #[serde(default = "default_chaos_max_trajectory")]
    pub chaos_max_trajectory: usize,
    /// 涌现阈值
    #[serde(default = "default_chaos_emergence_threshold")]
    pub chaos_emergence_threshold: f64,
    /// Prompt 预算字符数
    #[serde(default = "default_irrationality_prompt_budget")]
    pub prompt_budget: usize,
    /// 非理性 tick 间隔（tick 数），默认 100（约 1s）
    #[serde(default = "default_irrationality_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
}

impl Default for IrrationalityCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            pulse_min_pad_change: default_pulse_min_pad_change(),
            pulse_max_active: default_pulse_max_active(),
            pulse_uncaused_prob: 0.0,
            residue_max_active: default_residue_max_active(),
            contagion_cooldown_secs: default_contagion_cooldown_secs(),
            chaos_max_trajectory: default_chaos_max_trajectory(),
            chaos_emergence_threshold: default_chaos_emergence_threshold(),
            prompt_budget: default_irrationality_prompt_budget(),
            tick_interval_ticks: default_irrationality_tick_interval_ticks(),
        }
    }
}

fn default_pulse_min_pad_change() -> f32 {
    0.15
}
fn default_pulse_max_active() -> usize {
    10
}
fn default_residue_max_active() -> usize {
    20
}
fn default_contagion_cooldown_secs() -> i64 {
    300
}
fn default_chaos_max_trajectory() -> usize {
    200
}
fn default_chaos_emergence_threshold() -> f64 {
    0.4
}
fn default_irrationality_prompt_budget() -> usize {
    200
}
fn default_irrationality_tick_interval_ticks() -> u64 {
    100
}

// ── Ritual 共享仪式配置 / Shared Ritual Config ──

/// 共享仪式配置 — 日常仪式检测、纪念日系统、季节感知
///
/// Shared ritual configuration — daily ritual detection, anniversary system, seasonal awareness.
#[derive(Debug, Clone, Deserialize)]
pub struct RitualCfg {
    /// 是否启用共享仪式系统
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 仪式检测 tick 间隔（tick 数），默认 600（约 6s）
    #[serde(default = "default_ritual_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
    /// 纪念日提醒提前天数
    #[serde(default = "default_anniversary_remind_days")]
    pub anniversary_remind_days: u32,
    /// 季节感知 prompt 预算字符数
    #[serde(default = "default_ritual_prompt_budget")]
    pub prompt_budget: usize,
    /// 防抖写穿阈值：累积 N 条交互后批量持久化，降低 sled 写放大
    /// Debounced write-through threshold: batch persist after N interactions to reduce sled write amplification
    #[serde(default = "default_save_debounce_interactions")]
    pub save_debounce_interactions: u32,
}

impl Default for RitualCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval_ticks: default_ritual_tick_interval_ticks(),
            anniversary_remind_days: default_anniversary_remind_days(),
            prompt_budget: default_ritual_prompt_budget(),
            save_debounce_interactions: default_save_debounce_interactions(),
        }
    }
}

fn default_ritual_tick_interval_ticks() -> u64 {
    600
}
fn default_anniversary_remind_days() -> u32 {
    3
}
fn default_ritual_prompt_budget() -> usize {
    300
}
/// 防抖写穿默认阈值：每 5 条交互写穿一次 / Default debounce threshold: write-through every 5 interactions
fn default_save_debounce_interactions() -> u32 {
    5
}

// ── Vulnerability 脆弱与不完美配置 / Vulnerability & Imperfection Config ──

/// 脆弱与不完美配置 — 深度信任阶段的脆弱时刻门控
///
/// Vulnerability & imperfection configuration — gate for vulnerable moments in deep trust.
#[derive(Debug, Clone, Deserialize)]
pub struct VulnerabilityCfg {
    /// 是否启用脆弱窗口
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 频率控制：每 N 次对话最多 1 次脆弱时刻
    #[serde(default = "default_vulnerability_max_per_n")]
    pub max_per_n_conversations: u64,
    /// 脆弱窗口 tick 间隔（tick 数），默认 1000（约 10s）
    #[serde(default = "default_vulnerability_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
    /// Prompt 预算字符数
    #[serde(default = "default_vulnerability_prompt_budget")]
    pub prompt_budget: usize,
}

impl Default for VulnerabilityCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            max_per_n_conversations: default_vulnerability_max_per_n(),
            tick_interval_ticks: default_vulnerability_tick_interval_ticks(),
            prompt_budget: default_vulnerability_prompt_budget(),
        }
    }
}

fn default_vulnerability_max_per_n() -> u64 {
    50
}
fn default_vulnerability_tick_interval_ticks() -> u64 {
    1000
}
fn default_vulnerability_prompt_budget() -> usize {
    200
}

// ── 情绪需求边界配置 / Emotional Demand Boundary Config ──

/// 情绪需求边界配置 — 情绪耗竭保护 + 需求过载检测
///
/// Emotional demand boundary configuration — emotional exhaustion protection
/// and demand overload detection.
#[derive(Debug, Clone, Deserialize)]
pub struct EmotionalDemandCfg {
    /// 是否启用情绪需求边界 / Whether to enable emotional demand boundary.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 边界 tick 间隔（tick 数），默认 1000（约 10s）
    #[serde(default = "default_emotional_demand_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
}

impl Default for EmotionalDemandCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval_ticks: default_emotional_demand_tick_interval_ticks(),
        }
    }
}

fn default_emotional_demand_tick_interval_ticks() -> u64 {
    1000
}

// ── 自我关怀边界配置 / Self-Care Boundary Config ──

/// 自我关怀边界配置 — 情绪耗竭时主动降低交互强度
///
/// Self-care boundary configuration — proactively reduce interaction
/// intensity when emotionally exhausted.
#[derive(Debug, Clone, Deserialize)]
pub struct SelfCareCfg {
    /// 是否启用自我关怀边界 / Whether to enable self-care boundary.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 边界 tick 间隔（tick 数），默认 1000（约 10s）
    #[serde(default = "default_self_care_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
}

impl Default for SelfCareCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval_ticks: default_self_care_tick_interval_ticks(),
        }
    }
}

fn default_self_care_tick_interval_ticks() -> u64 {
    1000
}

// ── FollowUp 追问引擎配置 / Follow-Up Engine Config ──

/// 追问引擎配置 — 好奇心与追问：从对话提取待跟进事项，适时主动追问
///
/// Follow-up engine configuration — extract follow-up items from conversation,
/// and proactively ask at appropriate moments.
#[derive(Debug, Clone, Deserialize)]
pub struct FollowUpCfg {
    /// 是否启用追问引擎 / Whether to enable the follow-up engine.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 检查间隔（scheduler tick 数），默认 6000（约 60s）
    #[serde(default = "default_followup_check_interval_ticks")]
    pub check_interval_ticks: u64,
    /// 每日追问上限 / Max follow-ups per day.
    #[serde(default = "default_followup_max_per_day")]
    pub max_per_day: u32,
    /// 最小追问间隔（秒）/ Min interval between follow-ups in seconds.
    #[serde(default = "default_followup_min_interval_secs")]
    pub min_interval_secs: u64,
    /// 触发阈值 / Trigger threshold.
    #[serde(default = "default_followup_trigger_threshold")]
    pub trigger_threshold: f32,
    /// 最小有效权重 / Min effective weight to consider.
    #[serde(default = "default_followup_min_weight_threshold")]
    pub min_weight_threshold: f32,
    /// 时间信号权重 / Time signal weight.
    #[serde(default = "default_followup_time_weight")]
    pub time_weight: f32,
    /// 话题信号权重 / Topic signal weight.
    #[serde(default = "default_followup_topic_weight")]
    pub topic_weight: f32,
    /// 情感信号权重 / Emotion signal weight.
    #[serde(default = "default_followup_emotion_weight")]
    pub emotion_weight: f32,
    /// 过期清理天数 / Days after which to prune expired items.
    #[serde(default = "default_followup_expire_after_days")]
    pub expire_after_days: u32,
}

impl Default for FollowUpCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_ticks: default_followup_check_interval_ticks(),
            max_per_day: default_followup_max_per_day(),
            min_interval_secs: default_followup_min_interval_secs(),
            trigger_threshold: default_followup_trigger_threshold(),
            min_weight_threshold: default_followup_min_weight_threshold(),
            time_weight: default_followup_time_weight(),
            topic_weight: default_followup_topic_weight(),
            emotion_weight: default_followup_emotion_weight(),
            expire_after_days: default_followup_expire_after_days(),
        }
    }
}

fn default_followup_check_interval_ticks() -> u64 {
    6000
}
fn default_followup_max_per_day() -> u32 {
    5
}
fn default_followup_min_interval_secs() -> u64 {
    3600
}
fn default_followup_trigger_threshold() -> f32 {
    0.3
}
fn default_followup_min_weight_threshold() -> f32 {
    0.1
}
fn default_followup_time_weight() -> f32 {
    0.4
}
fn default_followup_topic_weight() -> f32 {
    0.3
}
fn default_followup_emotion_weight() -> f32 {
    0.3
}
fn default_followup_expire_after_days() -> u32 {
    90
}

// ── Longing 想念引擎配置 / Longing Engine Config ──

/// 想念引擎配置 — 用户离开时 PAD 漂移基线渐变到想念基线
///
/// Longing engine configuration — PAD drift baseline interpolates
/// from neutral to a longing baseline when the user is away.
#[derive(Debug, Clone, Deserialize)]
pub struct LongingCfg {
    /// 是否启用想念引擎 / Whether to enable the longing engine.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 想念起始阈值（秒）/ Onset threshold in seconds.
    #[serde(default = "default_longing_onset")]
    pub onset_threshold_secs: u64,
    /// 想念饱和阈值（秒）/ Saturation threshold in seconds.
    #[serde(default = "default_longing_saturation")]
    pub saturation_threshold_secs: u64,
    /// OU 波动率 / OU volatility.
    #[serde(default = "default_longing_volatility")]
    pub volatility: f64,
    /// 均值回归率 / Mean reversion rate.
    #[serde(default = "default_longing_mean_reversion")]
    pub mean_reversion: f64,
    /// 想念基线 PAD / Longing PAD baseline.
    #[serde(default = "default_longing_baseline")]
    pub baseline: [f64; 3],
    /// 重逢配置 / Reunion configuration.
    #[serde(default)]
    pub reunion: ReunionCfg,
    /// 期待事件配置 / Anticipation configuration.
    #[serde(default)]
    pub anticipation: AnticipationCfg,
}

impl Default for LongingCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            onset_threshold_secs: default_longing_onset(),
            saturation_threshold_secs: default_longing_saturation(),
            volatility: default_longing_volatility(),
            mean_reversion: default_longing_mean_reversion(),
            baseline: default_longing_baseline(),
            reunion: ReunionCfg::default(),
            anticipation: AnticipationCfg::default(),
        }
    }
}

fn default_longing_onset() -> u64 {
    600 // 10 分钟
}

fn default_longing_saturation() -> u64 {
    7200 // 2 小时
}

fn default_longing_volatility() -> f64 {
    0.001
}

fn default_longing_mean_reversion() -> f64 {
    0.0005
}

fn default_longing_baseline() -> [f64; 3] {
    [-0.25, 0.05, -0.15]
}

/// 重逢爆发配置 / Reunion burst configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ReunionCfg {
    /// 愉悦增益 / Pleasure boost.
    #[serde(default = "default_reunion_joy")]
    pub joy_boost: f32,
    /// 唤醒增益 / Arousal boost.
    #[serde(default = "default_reunion_arousal")]
    pub arousal_boost: f32,
    /// 掌控感增益 / Dominance boost.
    #[serde(default = "default_reunion_dominance")]
    pub dominance_boost: f32,
    /// 每日最大重逢爆发次数 / Max reunion bursts per day.
    #[serde(default = "default_reunion_max")]
    pub max_per_day: u32,
    /// 是否生成重逢消息 / Whether to generate reunion message via LLM.
    #[serde(default = "default_true")]
    pub generate_message: bool,
}

impl Default for ReunionCfg {
    fn default() -> Self {
        Self {
            joy_boost: default_reunion_joy(),
            arousal_boost: default_reunion_arousal(),
            dominance_boost: default_reunion_dominance(),
            max_per_day: default_reunion_max(),
            generate_message: true,
        }
    }
}

fn default_reunion_joy() -> f32 {
    0.4
}

fn default_reunion_arousal() -> f32 {
    0.3
}

fn default_reunion_dominance() -> f32 {
    0.1
}

fn default_reunion_max() -> u32 {
    3
}

/// 期待事件配置 / Anticipation event configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AnticipationCfg {
    /// 是否启用期待事件检测 / Whether to enable anticipation detection.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 期待预加载时间（秒）/ Pre-load duration before expected time.
    #[serde(default = "default_anticipation_preload")]
    pub preload_secs: u64,
    /// 过期失落强度 / Disappointment intensity when overdue.
    #[serde(default = "default_anticipation_disappointment")]
    pub disappointment_intensity: f32,
}

impl Default for AnticipationCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            preload_secs: default_anticipation_preload(),
            disappointment_intensity: default_anticipation_disappointment(),
        }
    }
}

fn default_anticipation_preload() -> u64 {
    1800 // 30 分钟
}

fn default_anticipation_disappointment() -> f32 {
    0.15
}

// ── Maturity 成长管理器配置 / Maturity Manager Config ──

/// 成长管理器配置 — AI 自身成熟度追踪与行为调制
///
/// Maturity manager configuration — AI self-maturity tracking and behavior modulation.
#[derive(Debug, Clone, Deserialize)]
pub struct MaturityCfg {
    /// 是否启用成长管理器 / Whether to enable the maturity manager.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for MaturityCfg {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ── InnerMonologue 内在独白配置 / Inner Monologue Config ──

/// 内在独白配置 — AI 独处时的自主思考系统
///
/// Inner monologue configuration — Autonomous thinking system for idle periods.
#[derive(Debug, Clone, Deserialize)]
pub struct InnerMonologueCfg {
    /// 是否启用内在独白 / Whether to enable the inner monologue engine.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 最大思考缓冲区 / Max thoughts retained in the ring buffer.
    #[serde(default = "default_inner_monologue_max_thoughts")]
    pub max_thoughts: usize,
    /// 每日思考总上限 / Total thoughts per day across all modes.
    #[serde(default = "default_inner_monologue_max_per_day")]
    pub max_per_day: u32,
    /// 图漫游最小间隔（秒）/ Min seconds between graph wanders.
    #[serde(default = "default_inner_monologue_gw_interval")]
    pub graph_wander_interval_secs: i64,
    /// 图漫游每日上限 / Max graph wanders per day.
    #[serde(default = "default_inner_monologue_gw_max")]
    pub graph_wander_max_per_day: u32,
    /// 学习最小间隔（秒）/ Min seconds between learning sessions.
    #[serde(default = "default_inner_monologue_learn_interval")]
    pub learning_interval_secs: i64,
    /// 学习每日上限 / Max learning sessions per day.
    #[serde(default = "default_inner_monologue_learn_max")]
    pub learning_max_per_day: u32,
    /// 白日梦最小间隔（秒）/ Min seconds between daydreams.
    #[serde(default = "default_inner_monologue_dd_interval")]
    pub daydream_interval_secs: i64,
    /// 白日梦置信度 / Confidence for daydream thoughts.
    #[serde(default = "default_inner_monologue_dd_confidence")]
    pub daydream_confidence: f64,
}

impl Default for InnerMonologueCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            max_thoughts: default_inner_monologue_max_thoughts(),
            max_per_day: default_inner_monologue_max_per_day(),
            graph_wander_interval_secs: default_inner_monologue_gw_interval(),
            graph_wander_max_per_day: default_inner_monologue_gw_max(),
            learning_interval_secs: default_inner_monologue_learn_interval(),
            learning_max_per_day: default_inner_monologue_learn_max(),
            daydream_interval_secs: default_inner_monologue_dd_interval(),
            daydream_confidence: default_inner_monologue_dd_confidence(),
        }
    }
}

fn default_inner_monologue_max_thoughts() -> usize {
    200
}
fn default_inner_monologue_max_per_day() -> u32 {
    30
}
fn default_inner_monologue_gw_interval() -> i64 {
    300
}
fn default_inner_monologue_gw_max() -> u32 {
    20
}
fn default_inner_monologue_learn_interval() -> i64 {
    1800
}
fn default_inner_monologue_learn_max() -> u32 {
    5
}
fn default_inner_monologue_dd_interval() -> i64 {
    1800
}
fn default_inner_monologue_dd_confidence() -> f64 {
    0.3
}

// ════════════════════════════════════════════════════════════════════
// NarrativeCfg — 叙事自我系统配置 / Narrative Self System Config
// ════════════════════════════════════════════════════════════════════

/// 叙事自我系统配置 / Narrative self system configuration
///
/// 控制叙事弧管理、转折点检测、自我描述重写等行为。
#[derive(Debug, Clone, Deserialize)]
pub struct NarrativeCfg {
    /// 是否启用叙事自我 / Whether to enable narrative self
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 叙事评估周期（tick 数），默认 1000（约 10s）
    #[serde(default = "default_narrative_tick_interval")]
    pub tick_interval: u64,
    /// 自我描述重写间隔天数 / Self-description rewrite interval (days)
    #[serde(default = "default_narrative_rewrite_days")]
    pub self_description_rewrite_days: i64,
    /// 弧休眠天数 / Arc dormancy threshold (days)
    #[serde(default = "default_narrative_dormancy_days")]
    pub arc_dormancy_days: i64,
    /// 弧完结天数 / Arc closure threshold (days)
    #[serde(default = "default_narrative_closure_days")]
    pub arc_closure_days: i64,
    /// 转折点情感变化阈值 / Turning point emotion change threshold
    #[serde(default = "default_narrative_emotion_threshold")]
    pub emotion_change_threshold: f32,
    /// 转折点最小间隔秒 / Turning point min interval (seconds)
    #[serde(default = "default_narrative_min_interval_secs")]
    pub min_interval_secs: i64,
    /// Prompt 注入预算（字符数）/ Prompt injection budget (chars)
    #[serde(default = "default_narrative_prompt_budget")]
    pub prompt_budget: usize,
    /// 每日叙事评估周期（tick 数），默认 864000（约 1 天）/ Daily narrative tick interval
    #[serde(default = "default_narrative_daily_tick_interval")]
    pub daily_tick_interval: u64,
    /// 每周叙事评估周期（tick 数），默认 6048000（约 7 天）/ Weekly narrative tick interval
    #[serde(default = "default_narrative_weekly_tick_interval")]
    pub weekly_tick_interval: u64,
}

impl Default for NarrativeCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval: default_narrative_tick_interval(),
            self_description_rewrite_days: default_narrative_rewrite_days(),
            arc_dormancy_days: default_narrative_dormancy_days(),
            arc_closure_days: default_narrative_closure_days(),
            emotion_change_threshold: default_narrative_emotion_threshold(),
            min_interval_secs: default_narrative_min_interval_secs(),
            prompt_budget: default_narrative_prompt_budget(),
            daily_tick_interval: default_narrative_daily_tick_interval(),
            weekly_tick_interval: default_narrative_weekly_tick_interval(),
        }
    }
}

// ── Conflict 冲突与和解配置 / Conflict & Reconciliation Config ──

/// 冲突与和解配置 — 分歧检测、过度索取检测、升级控制、和解工艺、道歉引擎
///
/// Conflict & reconciliation configuration — disagreement detection, over-demand detection,
/// escalation control, reconciliation craft, apology engine.
#[derive(Debug, Clone, Deserialize)]
pub struct ConflictCfg {
    /// 是否启用冲突与和解系统
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 分歧检测灵敏度 (0.1~1.0)
    #[serde(default = "default_conflict_sensitivity")]
    pub disagreement_sensitivity: f64,
    /// 过度索取窗口大小
    #[serde(default = "default_over_demand_window")]
    pub over_demand_window: usize,
    /// 过度索取高频阈值
    #[serde(default = "default_over_demand_threshold")]
    pub over_demand_threshold: f64,
    /// 升级冷却轮次
    #[serde(default = "default_escalation_cooldown")]
    pub escalation_cooldown_turns: u32,
    /// 连续冲突升级阈值
    #[serde(default = "default_consecutive_threshold")]
    pub consecutive_threshold: u32,
    /// 自动降级轮次
    #[serde(default = "default_de_escalation_turns")]
    pub de_escalation_turns: u32,
    /// 冲突评估周期（tick 数），默认 3000（约 30s）
    #[serde(default = "default_conflict_tick_interval")]
    pub tick_interval: u64,

    // ── G1: 主动和解管线配置 / G1: Proactive reconciler config ──
    /// 连续冲突N轮后触发主动和解 / Trigger proactive reconciliation after N turns
    #[serde(default = "default_proactive_threshold_turns")]
    pub proactive_threshold_turns: u32,
    /// 冲突后M秒仍无和解则主动 / Proactive after M seconds without reconciliation
    #[serde(default = "default_proactive_time_secs")]
    pub proactive_time_secs: i64,
    /// 每会话最多主动和解次数 / Max proactive reconciliations per session
    #[serde(default = "default_proactive_max_per_session")]
    pub proactive_max_per_session: u32,

    // ── G2: 冲突↔情绪双向闭环配置 / G2: Conflict↔emotion PAD bridge config ──
    /// 冲突→愉悦衰减系数 / Conflict→pleasure decay coefficient
    #[serde(default = "default_pleasure_decay")]
    pub pleasure_decay: f64,
    /// 冲突→唤醒增强系数 / Conflict→arousal boost coefficient
    #[serde(default = "default_arousal_boost")]
    pub arousal_boost: f64,
    /// 冲突→支配衰减系数 / Conflict→dominance decay coefficient
    #[serde(default = "default_dominance_decay")]
    pub dominance_decay: f64,

    // ── G4: 恢复曲线配置 / G4: Recovery curve config ──
    /// 基础恢复率/tick / Base recovery rate per tick
    #[serde(default = "default_base_recovery_rate")]
    pub base_recovery_rate: f64,
    /// 冲突衰减率 / Conflict decay rate
    #[serde(default = "default_conflict_decay_rate")]
    pub conflict_decay_rate: f64,
}

impl Default for ConflictCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            disagreement_sensitivity: default_conflict_sensitivity(),
            over_demand_window: default_over_demand_window(),
            over_demand_threshold: default_over_demand_threshold(),
            escalation_cooldown_turns: default_escalation_cooldown(),
            consecutive_threshold: default_consecutive_threshold(),
            de_escalation_turns: default_de_escalation_turns(),
            tick_interval: default_conflict_tick_interval(),
            proactive_threshold_turns: default_proactive_threshold_turns(),
            proactive_time_secs: default_proactive_time_secs(),
            proactive_max_per_session: default_proactive_max_per_session(),
            pleasure_decay: default_pleasure_decay(),
            arousal_boost: default_arousal_boost(),
            dominance_decay: default_dominance_decay(),
            base_recovery_rate: default_base_recovery_rate(),
            conflict_decay_rate: default_conflict_decay_rate(),
        }
    }
}

fn default_conflict_sensitivity() -> f64 {
    0.7
}
fn default_over_demand_window() -> usize {
    10
}
fn default_over_demand_threshold() -> f64 {
    0.6
}
fn default_escalation_cooldown() -> u32 {
    3
}
fn default_consecutive_threshold() -> u32 {
    3
}
fn default_de_escalation_turns() -> u32 {
    2
}
fn default_conflict_tick_interval() -> u64 {
    3000
}

// ── G1: 主动和解默认值 / G1: Proactive reconciler defaults ──
fn default_proactive_threshold_turns() -> u32 {
    3
}
fn default_proactive_time_secs() -> i64 {
    300
}
fn default_proactive_max_per_session() -> u32 {
    2
}

// ── G2: PAD桥接默认值 / G2: PAD bridge defaults ──
fn default_pleasure_decay() -> f64 {
    0.15
}
fn default_arousal_boost() -> f64 {
    0.1
}
fn default_dominance_decay() -> f64 {
    0.1
}

// ── G4: 恢复曲线默认值 / G4: Recovery curve defaults ──
fn default_base_recovery_rate() -> f64 {
    0.002
}
fn default_conflict_decay_rate() -> f64 {
    0.15
}

fn default_narrative_tick_interval() -> u64 {
    1000
}
fn default_narrative_rewrite_days() -> i64 {
    30
}
fn default_narrative_dormancy_days() -> i64 {
    14
}
fn default_narrative_closure_days() -> i64 {
    60
}
fn default_narrative_emotion_threshold() -> f32 {
    0.4
}
fn default_narrative_min_interval_secs() -> i64 {
    3600
}
fn default_narrative_prompt_budget() -> usize {
    800
}
fn default_narrative_daily_tick_interval() -> u64 {
    864_000 // ~1 天 @ 10ms/tick
}
fn default_narrative_weekly_tick_interval() -> u64 {
    6_048_000 // ~7 天 @ 10ms/tick
}

// ── Imperfection 适度犯错配置 / Imperfection Engine Config ──

/// 适度犯错配置 — 五维犯错模型、概率门控、延迟自纠闭环
///
/// Imperfection engine configuration — five-dimensional mistake model,
/// probabilistic gating, and delayed self-correction closed loop.
#[derive(Debug, Clone, Deserialize)]
pub struct ImperfectionCfg {
    /// 是否启用适度犯错系统 / Whether to enable the imperfection engine.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 基础犯错概率 / Base mistake probability.
    #[serde(default = "default_imperfection_base_prob")]
    pub base_prob: f64,
    /// 概率上限（安全阀）/ Probability cap (safety).
    #[serde(default = "default_imperfection_max_prob")]
    pub max_prob: f64,
    /// 认知负荷阈值 / Cognitive load threshold.
    #[serde(default = "default_imperfection_cognitive_load_threshold")]
    pub cognitive_load_threshold: f64,
    /// 疲劳阈值 / Fatigue threshold.
    #[serde(default = "default_imperfection_fatigue_threshold")]
    pub fatigue_threshold: f64,
    /// 陌生领域阈值 / Unfamiliar domain threshold.
    #[serde(default = "default_imperfection_unfamiliar_threshold")]
    pub unfamiliar_threshold: f64,
    /// 情绪干扰激活下限 / Emotional interference activation floor.
    #[serde(default = "default_imperfection_emotion_activation_floor")]
    pub emotion_activation_floor: f64,
    /// 关系深度门控下限 / Relationship depth gate minimum.
    #[serde(default = "default_imperfection_relationship_gate_min")]
    pub relationship_gate_min: f64,
    /// 成熟度序号门控下限 / Maturity ordinal gate minimum.
    #[serde(default = "default_imperfection_maturity_gate_min")]
    pub maturity_gate_min: u32,
    /// 自纠延迟最小秒数 / Self-correction delay min seconds.
    #[serde(default = "default_imperfection_correction_delay_min_secs")]
    pub correction_delay_min_secs: f64,
    /// 自纠延迟最大秒数 / Self-correction delay max seconds.
    #[serde(default = "default_imperfection_correction_delay_max_secs")]
    pub correction_delay_max_secs: f64,
    /// 单次对话最大犯错次数 / Max mistakes per conversation turn.
    #[serde(default = "default_imperfection_max_mistakes_per_turn")]
    pub max_mistakes_per_turn: u32,
    /// 冷却期秒数 / Cooldown between mistakes in seconds.
    #[serde(default = "default_imperfection_cooldown_secs")]
    pub cooldown_secs: f64,
    /// 连续无犯错衰减因子 / Clean streak decay factor.
    #[serde(default = "default_imperfection_clean_streak_decay")]
    pub clean_streak_decay: f64,
    /// 犯错权重 — 记忆漂移 / Mistake weight — memory drift
    #[serde(default = "default_mistake_weight_memory_drift")]
    pub mistake_weight_memory_drift: f64,
    /// 犯错权重 — 推理跳跃 / Mistake weight — reasoning leap
    #[serde(default = "default_mistake_weight_reasoning_leap")]
    pub mistake_weight_reasoning_leap: f64,
    /// 犯错权重 — 过度简化 / Mistake weight — over-simplification
    #[serde(default = "default_mistake_weight_over_simplification")]
    pub mistake_weight_over_simplification: f64,
    /// 犯错权重 — 故意模糊 / Mistake weight — intentional vagueness
    #[serde(default = "default_mistake_weight_intentional_vagueness")]
    pub mistake_weight_intentional_vagueness: f64,
    /// 犯错权重 — 知识边界 / Mistake weight — knowledge boundary
    #[serde(default = "default_mistake_weight_knowledge_boundary")]
    pub mistake_weight_knowledge_boundary: f64,
    /// Prompt 预算字符数 / Prompt budget in characters.
    #[serde(default = "default_imperfection_prompt_budget")]
    pub prompt_budget: usize,
    /// tick 间隔（tick 数），默认 1000（约 10s）/ Tick interval in ticks.
    #[serde(default = "default_imperfection_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
}

impl Default for ImperfectionCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            base_prob: default_imperfection_base_prob(),
            max_prob: default_imperfection_max_prob(),
            cognitive_load_threshold: default_imperfection_cognitive_load_threshold(),
            fatigue_threshold: default_imperfection_fatigue_threshold(),
            unfamiliar_threshold: default_imperfection_unfamiliar_threshold(),
            emotion_activation_floor: default_imperfection_emotion_activation_floor(),
            relationship_gate_min: default_imperfection_relationship_gate_min(),
            maturity_gate_min: default_imperfection_maturity_gate_min(),
            correction_delay_min_secs: default_imperfection_correction_delay_min_secs(),
            correction_delay_max_secs: default_imperfection_correction_delay_max_secs(),
            max_mistakes_per_turn: default_imperfection_max_mistakes_per_turn(),
            cooldown_secs: default_imperfection_cooldown_secs(),
            clean_streak_decay: default_imperfection_clean_streak_decay(),
            mistake_weight_memory_drift: default_mistake_weight_memory_drift(),
            mistake_weight_reasoning_leap: default_mistake_weight_reasoning_leap(),
            mistake_weight_over_simplification: default_mistake_weight_over_simplification(),
            mistake_weight_intentional_vagueness: default_mistake_weight_intentional_vagueness(),
            mistake_weight_knowledge_boundary: default_mistake_weight_knowledge_boundary(),
            prompt_budget: default_imperfection_prompt_budget(),
            tick_interval_ticks: default_imperfection_tick_interval_ticks(),
        }
    }
}

fn default_imperfection_base_prob() -> f64 {
    0.15
}
fn default_imperfection_max_prob() -> f64 {
    0.40
}
fn default_imperfection_cognitive_load_threshold() -> f64 {
    0.5
}
fn default_imperfection_fatigue_threshold() -> f64 {
    0.6
}
fn default_imperfection_unfamiliar_threshold() -> f64 {
    0.4
}
fn default_imperfection_emotion_activation_floor() -> f64 {
    0.3
}
fn default_imperfection_relationship_gate_min() -> f64 {
    0.3
}
fn default_imperfection_maturity_gate_min() -> u32 {
    1 // Growing 阶段起
}
fn default_imperfection_correction_delay_min_secs() -> f64 {
    2.0
}
fn default_imperfection_correction_delay_max_secs() -> f64 {
    15.0
}
fn default_imperfection_max_mistakes_per_turn() -> u32 {
    2
}
fn default_imperfection_cooldown_secs() -> f64 {
    30.0
}
fn default_imperfection_clean_streak_decay() -> f64 {
    0.95
}
fn default_imperfection_prompt_budget() -> usize {
    300
}
fn default_mistake_weight_memory_drift() -> f64 {
    0.25
}
fn default_mistake_weight_reasoning_leap() -> f64 {
    0.20
}
fn default_mistake_weight_over_simplification() -> f64 {
    0.15
}
fn default_mistake_weight_intentional_vagueness() -> f64 {
    0.10
}
fn default_mistake_weight_knowledge_boundary() -> f64 {
    0.30
}

fn default_imperfection_tick_interval_ticks() -> u64 {
    1000
}

// ── PhysicalPresence 物理存在感配置 / Physical Presence Config ──

/// 物理存在感配置 — 体感模拟、昼夜节律、交互疲劳、体感→情绪反向通道
///
/// Physical presence configuration — somatic simulation, circadian rhythm,
/// interaction fatigue, body→emotion reverse channel.
#[derive(Debug, Clone, Deserialize)]
pub struct PhysicalPresenceCfg {
    /// 是否启用物理存在感系统 / Whether to enable the physical presence system.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 物理存在感 tick 间隔（tick 数），默认 500（约 5s）
    /// Physical presence tick interval in ticks, defaults to 500 (~5s).
    #[serde(default = "default_physical_presence_tick_interval_ticks")]
    pub tick_interval_ticks: u64,
    /// 疲劳半衰期（秒），默认 14400（4h）
    /// Fatigue half-life in seconds, defaults to 14400 (4h).
    #[serde(default = "default_physical_presence_fatigue_half_life_secs")]
    pub fatigue_half_life_secs: f64,
    /// 昼夜节律调制启用 / Circadian modulation enabled.
    #[serde(default = "default_true")]
    pub circadian_enabled: bool,
    /// 交互疲劳启用 / Interaction fatigue enabled.
    #[serde(default = "default_true")]
    pub interaction_fatigue_enabled: bool,
    /// 体感→情绪反向通道启用 / Body→emotion reverse channel enabled.
    #[serde(default = "default_true")]
    pub body_to_emotion_enabled: bool,
    /// Prompt 预算字符数 / Prompt budget in characters.
    #[serde(default = "default_physical_presence_prompt_budget")]
    pub prompt_budget: usize,
}

impl Default for PhysicalPresenceCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            tick_interval_ticks: default_physical_presence_tick_interval_ticks(),
            fatigue_half_life_secs: default_physical_presence_fatigue_half_life_secs(),
            circadian_enabled: true,
            interaction_fatigue_enabled: true,
            body_to_emotion_enabled: true,
            prompt_budget: default_physical_presence_prompt_budget(),
        }
    }
}

fn default_physical_presence_tick_interval_ticks() -> u64 {
    500
}

fn default_physical_presence_fatigue_half_life_secs() -> f64 {
    14400.0 // 4h
}

fn default_physical_presence_prompt_budget() -> usize {
    200
}
