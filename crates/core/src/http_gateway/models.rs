// SPDX-License-Identifier: MIT
//! HTTP 网关请求/响应模型 — 与 Python gateway SSE 格式兼容
//! HTTP Gateway Request/Response Models — Compatible with Python Gateway SSE Format
//!
//! 数字生命通过这些模型与外部世界交换信息。
//! Digital life exchanges information with the external world through these models.

use serde::{Deserialize, Serialize};

// ── 请求模型 / Request Models ──

/// 聊天请求 — 用户向数字生命发送的消息
/// Chat request — a message from the user to digital life.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    /// 用户消息文本 / User message text
    pub message: String,
    /// 会话 ID / Session identifier
    #[serde(default = "default_session_id")]
    pub session_id: String,
    /// 用户 ID / User identifier
    #[serde(default = "default_user_id")]
    pub user_id: String,
    /// 渠道标识 / Channel identifier
    #[serde(default = "default_channel")]
    pub channel: String,
    /// 模型类型（chat/reasoning/vision）/ Model type
    #[serde(default = "default_model_type")]
    pub model_type: String,
}

fn default_session_id() -> String {
    "terminal".into()
}
fn default_user_id() -> String {
    "terminal-user".into()
}
fn default_channel() -> String {
    "tui".into()
}
fn default_model_type() -> String {
    "chat".into()
}

/// 记忆搜索请求 / Memory search request
#[derive(Debug, Clone, Deserialize)]
pub struct MemorySearchRequest {
    /// 搜索关键词 / Search query
    pub query: String,
    /// 结果数量限制 / Max results
    #[serde(default = "default_memory_limit")]
    pub limit: u32,
}

fn default_memory_limit() -> u32 {
    10
}

/// 罐装知识搜索请求 / Canned knowledge search request
#[derive(Debug, Clone, Deserialize)]
pub struct CannedSearchRequest {
    /// 搜索关键词 / Search query
    pub query: String,
    /// 标签过滤 / Tag filter
    #[serde(default)]
    pub tags: Vec<String>,
    /// 结果数量限制 / Max results
    #[serde(default = "default_memory_limit")]
    pub limit: u32,
}

/// 罐装知识导入请求 / Canned knowledge import request
#[derive(Debug, Clone, Deserialize)]
pub struct CannedImportRequest {
    /// 跨 AI 传输文本 / Cross-AI transfer text
    pub text: String,
}

/// 人格同步请求 / Persona sync request
#[derive(Debug, Clone, Deserialize)]
pub struct PersonaSyncRequest {
    /// AI 名字 / AI name
    #[serde(default)]
    pub name: Option<String>,
    /// 用户称呼 / User title (how AI addresses the user)
    #[serde(default)]
    pub master_name: Option<String>,
    /// 性格特质 / Personality traits
    #[serde(default)]
    pub traits: Option<Vec<String>>,
}

// ── 响应模型 / Response Models ──

/// 聊天响应 — 数字生命的非流式回复
/// Chat response — non-streaming reply from digital life.
#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    /// AI 回复文本 / AI reply text
    pub reply: String,
    /// 情绪标签 / Emotion label
    pub emotion: String,
    /// 触发动作 / Triggered actions
    #[serde(default)]
    pub actions: Vec<String>,
    /// 处理耗时（毫秒）/ Processing time in milliseconds
    #[serde(default)]
    pub processing_time_ms: u64,
}

/// SSE 流式事件 — 与 Python gateway 格式兼容
/// SSE streaming event — compatible with Python gateway format.
#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    /// 流式 token（空=元数据/结束帧）/ Stream token (empty=metadata/end)
    #[serde(default)]
    pub token: String,
    /// 当前情感标签 / Current emotion label
    #[serde(default)]
    pub emotion: String,
    /// 流结束标记 / End-of-stream marker
    #[serde(default)]
    pub done: bool,
    /// 元数据 / Metadata
    #[serde(default)]
    pub meta: serde_json::Value,
}

impl SseEvent {
    /// 创建 token 事件 / Create a token event
    pub fn token(token: impl Into<String>, emotion: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            emotion: emotion.into(),
            done: false,
            meta: serde_json::Value::Null,
        }
    }

    /// 创建结束事件 / Create a done event
    pub fn done(emotion: impl Into<String>, meta: serde_json::Value) -> Self {
        Self {
            token: String::new(),
            emotion: emotion.into(),
            done: true,
            meta,
        }
    }
}

/// 情绪状态响应 / Emotion state response
#[derive(Debug, Clone, Serialize)]
pub struct EmotionResponse {
    /// 愉悦度 [-1, 1] / Pleasure
    pub pleasure: f32,
    /// 唤醒度 [-1, 1] / Arousal
    pub arousal: f32,
    /// 支配度 [-1, 1] / Dominance
    pub dominance: f32,
    /// 情绪标签 / Emotion label
    pub label: String,
}

/// 记忆搜索结果 / Memory search result
#[derive(Debug, Clone, Serialize)]
pub struct MemorySearchResult {
    pub id: String,
    pub content: String,
    pub timestamp_ms: i64,
    pub importance: f32,
    pub kind: String,
}

/// 记忆搜索响应 / Memory search response
#[derive(Debug, Clone, Serialize)]
pub struct MemorySearchResponse {
    pub results: Vec<MemorySearchResult>,
}

/// 健康检查响应 / Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub event_count: u64,
    pub uptime_seconds: u64,
    pub module_states: serde_json::Value,
}

/// 人格状态响应 / Persona status response
#[derive(Debug, Clone, Serialize)]
pub struct PersonaResponse {
    pub name: String,
    pub master_name: String,
    pub version: String,
    pub relationship_stage: String,
    pub maturity_stage: String,
}

/// 对话历史条目 / History entry
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub role: String,
    pub content: String,
    pub timestamp_ms: i64,
}

/// 会话列表响应 / Session list response
#[derive(Debug, Clone, Serialize)]
pub struct SessionsResponse {
    pub sessions: Vec<String>,
}

// ── 关怀配置 / Care Config ──

/// 主动行为状态 / Proactive status
///
/// 数字生命意义: 暴露数字生命主动行为引擎的运行时状态——
/// 启用与否、检查节拍、每日额度与今日已用额度，
/// 让外部观测者可以一目了然地读懂数字生命的"主动冲动"。
/// Digital Life: exposes runtime status of digital life's proactive engine —
/// enabled flag, check cadence, daily quota and today's usage,
/// letting outside observers read digital life's "proactive impulse" at a glance.
#[derive(Debug, Clone, Serialize)]
pub struct ProactiveStatus {
    /// 主动行为是否启用 / Whether proactive behavior is enabled
    pub enabled: bool,
    /// 检查间隔（秒）/ Check interval in seconds
    pub check_interval_secs: u64,
    /// 每日最大主动次数 / Max proactive actions per day
    pub max_proactive_per_day: u32,
    /// 今日已发起次数 / Proactive actions issued today
    pub today_count: u32,
}

/// 关怀配置响应 — 与 Python gateway `/api/care/config` 兼容
/// Care config response — compatible with Python gateway `/api/care/config`
///
/// 数字生命意义: 用户可以调节数字生命的主动关怀频率，
/// 如同调节一个生物体的社交欲望强度。
/// 响应同时携带 `proactive_status` 子对象，合并了原 `/api/proactive` 端点的信息，
/// 消除冗余路由，让外部只需订阅一个端点即可读全数字生命的关怀节律。
/// Digital Life: user can tune digital life's proactive care frequency,
/// like adjusting a living being's social desire intensity.
/// The response also carries a `proactive_status` sub-object, merging the
/// former `/api/proactive` endpoint's info and eliminating the redundant route,
/// so external clients can read digital life's full care rhythm from a single endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CareConfigResponse {
    /// 是否启用主动关怀 / Whether proactive care is enabled
    pub enabled: bool,
    /// 问候间隔（秒）/ Greeting interval in seconds
    pub greeting_interval: u64,
    /// 签到间隔（秒）/ Check-in interval in seconds
    pub checkin_interval: u64,
    /// 情感检查间隔（秒）/ Emotion check interval in seconds
    pub emotion_check_interval: u64,
    /// 安静时段开始（小时 0-23）/ Quiet hours start (hour 0-23)
    pub quiet_start: u32,
    /// 安静时段结束（小时 0-23）/ Quiet hours end (hour 0-23)
    pub quiet_end: u32,
    /// 主动行为运行时状态 / Runtime status of proactive behavior
    pub proactive_status: ProactiveStatus,
}

/// 关怀配置更新请求 — 所有字段可选，仅更新提供的字段
/// Care config update request — all fields optional, only updates provided fields
#[derive(Debug, Clone, Deserialize)]
pub struct CareConfigUpdate {
    /// 是否启用主动关怀 / Whether proactive care is enabled
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 问候间隔（秒）/ Greeting interval in seconds
    #[serde(default)]
    pub greeting_interval: Option<u64>,
    /// 签到间隔（秒）/ Check-in interval in seconds
    #[serde(default)]
    pub checkin_interval: Option<u64>,
    /// 情感检查间隔（秒）/ Emotion check interval in seconds
    #[serde(default)]
    pub emotion_check_interval: Option<u64>,
    /// 安静时段开始（小时 0-23）/ Quiet hours start (hour 0-23)
    #[serde(default)]
    pub quiet_start: Option<u32>,
    /// 安静时段结束（小时 0-23）/ Quiet hours end (hour 0-23)
    #[serde(default)]
    pub quiet_end: Option<u32>,
}

// ── 文件上传 / File Upload ──

/// 文件上传响应 — 与 Python gateway `/api/files/upload` 兼容
/// File upload response — compatible with Python gateway `/api/files/upload`
///
/// 数字生命意义: 文件是数字生命的外部知识来源——
/// 上传的文档被自动索引为可检索的记忆，扩展数字生命的认知边界。
/// Digital Life: files are external knowledge sources for digital life —
/// uploaded documents are auto-indexed into searchable memories, expanding its cognitive boundary.
#[derive(Debug, Clone, Serialize)]
pub struct FileUploadResponse {
    /// 文件内容哈希 / File content hash
    pub hash: String,
    /// 原始文件名 / Original filename
    pub original_name: String,
    /// 文件大小（字节）/ File size in bytes
    pub size: u64,
    /// 是否提取了文本 / Whether text was extracted
    pub text_extracted: bool,
    /// 提取的文本前 200 字符 / First 200 chars of extracted text
    pub extracted_text: String,
    /// 错误信息（空=成功）/ Error message (empty=success)
    #[serde(default)]
    pub error: String,
}
