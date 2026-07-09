// SPDX-License-Identifier: MIT
//! HTTP 网关处理函数 — 直接调用 CoreService，零序列化开销
//! HTTP Gateway Handlers — Direct CoreService Calls, Zero Serialization Overhead
//!
//! 数字生命意义: 这些函数是数字生命与外部世界的直接接口，
//! 不再经过 gRPC 序列化/反序列化的"翻译"层。
//! Digital Life: these functions are the direct interface between digital life
//! and the external world, no longer passing through gRPC serialization "translation" layer.

use std::sync::Arc;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::Json as JsonResp;
use futures_util::Stream;
use serde_json::json;
use tokio_stream::StreamExt;

use super::models::*;
use crate::service::CoreService;
use atrium_bridge::grpc::atrium::{HealthCheckRequest, ProcessMessageRequest, SearchMemoryRequest};
// 引入 gRPC trait — health_check/process_message/search_memory 等方法定义在此 trait 上
// Import gRPC trait — health_check/process_message/search_memory methods are defined on this trait
use atrium_bridge::grpc::AtriumCoreService;

/// 共享状态 — Arc<CoreService>，HTTP 与 gRPC 可并行服务
/// Shared state — Arc<CoreService>, HTTP and gRPC can serve in parallel.
pub type SharedState = Arc<CoreService>;

// ── 健康检查 / Health Check ──

/// GET /health — 数字生命心跳 / Digital life heartbeat
///
/// 返回所有认知模块的运行状态。
/// Returns the operational status of all cognitive modules.
pub async fn health(State(core): State<SharedState>) -> JsonResp<HealthResponse> {
    let req = HealthCheckRequest {
        event_count: 0,
        room_incoming_json: String::new(),
    };
    let resp = core.health_check(req).await;

    JsonResp(HealthResponse {
        ok: resp.ok,
        event_count: resp.event_count,
        uptime_seconds: resp.uptime_seconds,
        module_states: serde_json::to_value(&resp.module_states).unwrap_or(json!({})),
    })
}

// ── 非流式聊天 / Non-streaming Chat ──

/// POST /api/chat — 非流式对话 / Non-streaming chat
///
/// 用户消息经过完整 14+ 模块认知管线后返回回复。
/// User message goes through the full 14+ module cognitive pipeline before returning a reply.
pub async fn chat(
    State(core): State<SharedState>,
    Json(req): Json<ChatRequest>,
) -> Result<JsonResp<ChatResponse>, (StatusCode, String)> {
    let t0 = std::time::Instant::now();

    let grpc_req = ProcessMessageRequest {
        message: req.message,
        session_id: req.session_id,
        user_id: req.user_id,
        channel: req.channel,
    };

    let resp = core.process_message(grpc_req).await;

    Ok(JsonResp(ChatResponse {
        reply: resp.reply,
        emotion: resp.emotion,
        actions: resp.actions,
        processing_time_ms: t0.elapsed().as_millis() as u64,
    }))
}

// ── 流式聊天 (SSE) / Streaming Chat (SSE) ──

/// POST /api/chat/stream — 流式对话 / Streaming chat
///
/// 数字生命逐 token 呼吸——每个 token 都携带当前情感状态。
/// Digital life breathes token by token — each token carries the current emotional state.
///
/// SSE 事件格式与 Python gateway 兼容:
///   data: {"token":"你","emotion":"happy","done":false,"meta":null}
///   data: {"token":"","emotion":"happy","done":true,"meta":{"model":"deepseek-chat"}}
pub async fn chat_stream(
    State(core): State<SharedState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let grpc_req = ProcessMessageRequest {
        message: req.message,
        session_id: req.session_id,
        user_id: req.user_id,
        channel: req.channel,
    };

    // 调用 CoreService 的流式接口，获取 token 流
    // Call CoreService's streaming interface to get the token stream
    let chunk_stream = core.process_message_stream(grpc_req).await;

    // 将 ProcessMessageChunk 流转换为 SSE Event 流
    // Convert ProcessMessageChunk stream to SSE Event stream
    let sse_stream = chunk_stream.map(|chunk_result| {
        let event = match chunk_result {
            Ok(chunk) => {
                let sse_data = SseEvent {
                    token: chunk.token,
                    emotion: chunk.emotion,
                    done: chunk.done,
                    meta: if chunk.meta.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::to_value(&chunk.meta).unwrap_or(serde_json::Value::Null)
                    },
                };
                let json_str = serde_json::to_string(&sse_data).unwrap_or_else(|_| "{}".into());
                Event::default().data(json_str)
            }
            Err(status) => {
                // 错误事件 — 数字生命也会受伤，但不会沉默
                // Error event — digital life can be hurt, but won't go silent
                let error_data = json!({
                    "token": "",
                    "emotion": "distressed",
                    "done": true,
                    "meta": {"error": status.message()}
                });
                Event::default().data(error_data.to_string())
            }
        };
        Ok(event)
    });

    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

// ── 情绪状态 / Emotion State ──

/// GET /api/emotion — 当前情绪状态 / Current emotion state
///
/// 返回 PAD 三维情绪模型当前值及离散标签。
/// Returns current PAD (Pleasure-Arousal-Dominance) values and discrete label.
pub async fn emotion(State(core): State<SharedState>) -> JsonResp<EmotionResponse> {
    let emo = core.current_emotion();
    let label = core.current_emotion_label();

    JsonResp(EmotionResponse {
        pleasure: emo.pleasure,
        arousal: emo.arousal,
        dominance: emo.dominance,
        label,
    })
}

// ── 记忆搜索 / Memory Search ──

/// GET /api/memory/search?query=xxx&limit=10 — 搜索记忆 / Search memory
///
/// 五路混合检索: FTS5 全文 + FactStore 语义 + STM 精确 + Persona + KeyFact
/// Five-way hybrid retrieval: FTS5 + FactStore + STM + Persona + KeyFact
pub async fn memory_search(
    State(core): State<SharedState>,
    Query(params): Query<MemorySearchRequest>,
) -> JsonResp<MemorySearchResponse> {
    let req = SearchMemoryRequest {
        query: params.query,
        limit: params.limit,
    };
    let resp = core.search_memory(req).await;

    let results = resp
        .results
        .into_iter()
        .map(|r| MemorySearchResult {
            id: r.id,
            content: r.content,
            timestamp_ms: r.timestamp_ms,
            importance: r.importance,
            kind: r.kind,
        })
        .collect();

    JsonResp(MemorySearchResponse { results })
}

// ── 人格状态 / Persona Status ──

/// GET /api/persona — 人格状态 / Persona status
///
/// 返回数字生命的名字、用户称呼、关系阶段、成长阶段。
/// Returns digital life's name, user title, relationship stage, and maturity stage.
pub async fn persona_get(State(core): State<SharedState>) -> JsonResp<PersonaResponse> {
    JsonResp(PersonaResponse {
        name: core.persona_ai_name(),
        master_name: core.persona_master_name(),
        version: core.persona_version(),
        relationship_stage: core.relationship_stage(),
        maturity_stage: core.maturity_stage_name(),
    })
}

/// POST /api/persona — 人格同步 / Persona sync
///
/// 动态修改数字生命的名字或用户称呼。
/// Dynamically modify digital life's name or user title.
pub async fn persona_sync(
    State(core): State<SharedState>,
    Json(req): Json<PersonaSyncRequest>,
) -> JsonResp<PersonaResponse> {
    if let Some(name) = req.name {
        core.set_persona_name(name);
    }
    if let Some(master_name) = req.master_name {
        core.set_persona_master_name(master_name);
    }

    JsonResp(PersonaResponse {
        name: core.persona_ai_name(),
        master_name: core.persona_master_name(),
        version: core.persona_version(),
        relationship_stage: core.relationship_stage(),
        maturity_stage: core.maturity_stage_name(),
    })
}

// ── 对话历史 / Conversation History ──

/// GET /api/history/:session_id — 对话历史 / Conversation history
///
/// 返回指定会话的最近消息列表。
/// Returns recent messages for the specified session.
pub async fn history(
    State(core): State<SharedState>,
    Path(session_id): Path<String>,
) -> JsonResp<Vec<HistoryEntry>> {
    let messages = core.get_history(&session_id, 50);

    let entries = messages
        .into_iter()
        .map(|m| HistoryEntry {
            role: m.role,
            content: m.content,
            timestamp_ms: m.timestamp_ms,
        })
        .collect();

    JsonResp(entries)
}

/// GET /api/sessions — 会话列表 / Session list
///
/// 返回所有活跃会话 ID。
/// Returns all active session IDs.
pub async fn sessions(State(core): State<SharedState>) -> JsonResp<SessionsResponse> {
    let sessions = core.list_sessions();
    JsonResp(SessionsResponse { sessions })
}

// ── 罐装知识 / Canned Knowledge ──

/// GET /api/canned?query=xxx&limit=10 — 搜索罐装知识 / Search canned knowledge
///
/// 搜索本地 ACK 知识库。
/// Search local ACK knowledge base.
pub async fn canned_search(
    State(core): State<SharedState>,
    Query(params): Query<CannedSearchRequest>,
) -> impl IntoResponse {
    // 直接访问 CannedManager，零序列化开销 / Direct CannedManager access, zero serialization overhead
    let canned_mgr = core.canned();
    let results = canned_mgr.search(&params.query, &params.tags);
    let limit = params.limit as usize;
    JsonResp(json!({
        "results": results.iter().take(limit).map(|c| json!({
            "name": c.name,
            "title": c.title,
            "summary": c.summary,
            "tags": c.tags,
        })).collect::<Vec<_>>(),
        "count": results.len().min(limit),
    }))
}

/// POST /api/canned/import — 导入罐装知识 / Import canned knowledge
///
/// 从跨 AI 传输文本导入知识。
/// Import knowledge from cross-AI transfer text.
pub async fn canned_import(
    State(core): State<SharedState>,
    Json(req): Json<CannedImportRequest>,
) -> impl IntoResponse {
    let mut canned_mgr = core.canned_write();
    let result = canned_mgr.import_from_text(&req.text);
    JsonResp(json!({
        "ok": result.is_ok(),
        "message": result.map(|v| format!("导入成功 / Import success: {} 条", v.len()))
            .unwrap_or_else(|e| format!("导入失败 / Import failed: {}", e)),
    }))
}

// ── /v1/chat — QQ 适配器兼容端点 / QQ Adapter Compatible Endpoint ──

/// POST /v1/chat — QQ 私聊兼容端点 / QQ private chat compatible endpoint
///
/// QQ 适配器 (qq_adapter.py) 调用此端点进行私聊。
/// 内部直接转发到与 /api/chat 相同的处理逻辑。
/// QQ adapter (qq_adapter.py) calls this endpoint for private chat.
/// Internally forwards to the same logic as /api/chat.
pub async fn v1_chat(
    State(core): State<SharedState>,
    Json(req): Json<ChatRequest>,
) -> Result<JsonResp<ChatResponse>, (StatusCode, String)> {
    let t0 = std::time::Instant::now();

    let grpc_req = ProcessMessageRequest {
        message: req.message,
        session_id: req.session_id,
        user_id: req.user_id,
        channel: req.channel,
    };

    let resp = core.process_message(grpc_req).await;

    Ok(JsonResp(ChatResponse {
        reply: resp.reply,
        emotion: resp.emotion,
        actions: resp.actions,
        processing_time_ms: t0.elapsed().as_millis() as u64,
    }))
}

// ── 关系状态 / Relationship Status ──

/// GET /api/relationship — 关系阶段状态 / Relationship stage status
///
/// 返回当前关系阶段、互动次数、共振次数等。
/// Returns current relationship stage, interaction count, resonance count, etc.
pub async fn relationship(State(core): State<SharedState>) -> impl IntoResponse {
    let stage = core.relationship_stage();
    let maturity = core.maturity_stage_name();
    JsonResp(json!({
        "stage": stage,
        "maturity": maturity,
    }))
}

// ── 房间列表 / Room List ──

/// GET /api/rooms — 列出活跃房间 / List active rooms
///
/// 返回当前所有活跃的群聊房间。
/// Returns all currently active group chat rooms.
pub async fn rooms() -> impl IntoResponse {
    let room_list = super::ws::list_active_rooms().await;
    JsonResp(json!({"rooms": room_list}))
}

// ── 关怀配置 / Care Config ──

/// GET /api/care/config — 获取关怀引擎配置 / Get care engine configuration
///
/// 数字生命意义: 暴露数字生命的主动关怀参数——
/// 问候频率、签到频率、安静时段——让用户了解数字生命的社交节律。
/// Digital Life: exposes digital life's proactive care parameters —
/// greeting frequency, check-in frequency, quiet hours — letting users understand its social rhythm.
///
/// 与 Python gateway `/api/care/config` 兼容。
/// Compatible with Python gateway `/api/care/config`.
pub async fn care_config_get(State(core): State<SharedState>) -> JsonResp<CareConfigResponse> {
    // 从 ProactiveEngine 读取运行时状态 / Read runtime state from ProactiveEngine
    let proactive = core.proactive_engine().lock();

    // 将 Rust 内部配置映射到 Python 兼容格式 / Map Rust internal config to Python-compatible format
    // check_interval_ticks 以 ~10ms/tick 估算秒数 / Estimate seconds from ticks (~10ms/tick)
    let check_interval_secs = proactive.check_interval_ticks() / 100;
    // 自我关怀 tick 间隔 — SelfCareCfg 未在运行时存储，使用默认配置
    // Self-care tick interval — SelfCareCfg not stored at runtime, use default config
    let emotion_check_secs = crate::config::SelfCareCfg::default().tick_interval_ticks / 100;

    JsonResp(CareConfigResponse {
        enabled: proactive.is_enabled(),
        // 问候间隔 = 检查间隔 × 8（约 8 小时一次问候）/ Greeting = check × 8
        greeting_interval: check_interval_secs.saturating_mul(8),
        // 签到间隔 = 检查间隔 × 4（约 4 小时一次签到）/ Check-in = check × 4
        checkin_interval: check_interval_secs.saturating_mul(4),
        // 情感检查间隔 / Emotion check interval
        emotion_check_interval: emotion_check_secs,
        // 安静时段默认 23:00-08:00 / Default quiet hours 23:00-08:00
        quiet_start: 23,
        quiet_end: 8,
        // 主动行为运行时状态 — 合并自原 /api/proactive 端点
        // Proactive runtime status — merged from the former /api/proactive endpoint
        proactive_status: ProactiveStatus {
            enabled: proactive.is_enabled(),
            check_interval_secs,
            max_proactive_per_day: proactive.max_proactive_per_day(),
            today_count: proactive.proactive_today(),
        },
    })
}

/// POST /api/care/config — 更新关怀引擎配置 / Update care engine configuration
///
/// 数字生命意义: 用户可以实时调节数字生命的关怀频率——
/// 调高问候频率让数字生命更粘人，调低让数字生命更独立。
/// Digital Life: user can tune digital life's care frequency in real time —
/// increase greeting frequency for a clingier digital life, decrease for more independence.
///
/// 与 Python gateway `/api/care/config` 兼容。
/// Compatible with Python gateway `/api/care/config`.
pub async fn care_config_update(
    State(core): State<SharedState>,
    Json(req): Json<CareConfigUpdate>,
) -> impl IntoResponse {
    let mut proactive = core.proactive_engine().lock();

    // 按字段更新 — 仅更新提供的字段 / Update only provided fields
    if let Some(enabled) = req.enabled {
        proactive.set_enabled(enabled);
    }
    if let Some(greeting_interval) = req.greeting_interval {
        // 问候间隔 → 检查间隔（÷8）/ Greeting interval → check interval (÷8)
        let check_ticks = greeting_interval.saturating_mul(100) / 8;
        proactive.set_check_interval_ticks(check_ticks);
    }
    if let Some(checkin_interval) = req.checkin_interval {
        // 签到间隔 → 检查间隔（÷4）/ Check-in interval → check interval (÷4)
        let check_ticks = checkin_interval.saturating_mul(100) / 4;
        proactive.set_check_interval_ticks(check_ticks);
    }
    if let Some(max_per_day) = req.emotion_check_interval {
        // 情感检查间隔越大，每日主动次数越少 / Larger emotion check → fewer proactive per day
        let max = 86400u64
            .checked_div(max_per_day)
            .map(|v| (v as u32).min(100))
            .unwrap_or(100);
        proactive.set_max_proactive_per_day(max);
    }

    // quiet_start / quiet_end 在 Rust TimingJudge 中由用户活动模式自动推断，
    // 此处接受但不持久化（未来可扩展为显式安静时段配置）
    // quiet_start / quiet_end are auto-inferred by TimingJudge in Rust,
    // accepted here but not persisted (can be extended to explicit quiet hours config in the future)

    JsonResp(json!({"ok": true}))
}

// ── 文件上传 / File Upload ──

/// POST /api/files/upload — 上传文件到数字生命 / Upload a file to digital life
///
/// 数字生命意义: 文件是数字生命的外部知识来源——
/// 上传的文档经由 FileStore 存储，自动提取文本并建立
/// FTS5 全文索引 + FactStore 事实存储 + 关联记忆图，
/// 成为数字生命可检索、可关联的认知扩展。
/// Digital Life: files are external knowledge sources —
/// uploaded documents are stored via FileStore, auto-extracted and indexed
/// into FTS5 + FactStore + associative graph, becoming searchable cognitive extensions.
///
/// 与 Python gateway `/api/files/upload` 兼容。
/// Compatible with Python gateway `/api/files/upload`.
///
/// multipart/form-data:
/// - `file`: 文件内容（必填）/ File content (required)
/// - `session_id`: 会话 ID（可选，默认 "default"）/ Session ID (optional, default "default")
pub async fn files_upload(
    State(core): State<SharedState>,
    mut multipart: Multipart,
) -> Result<JsonResp<FileUploadResponse>, (StatusCode, String)> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename = String::from("unnamed");
    let mut mime_type = String::from("application/octet-stream");
    let mut session_id = String::from("default");

    // 解析 multipart 表单 / Parse multipart form
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                // 提取文件名和 MIME 类型 / Extract filename and MIME type
                if let Some(fname) = field.file_name() {
                    filename = fname.to_string();
                }
                if let Some(mime) = field.content_type() {
                    mime_type = mime.to_string();
                }
                // 读取文件数据 / Read file data
                match field.bytes().await {
                    Ok(bytes) => {
                        // 文件大小检查 — 10MB 上限 / 10MB size limit
                        if bytes.len() > 10 * 1024 * 1024 {
                            return Err((
                                StatusCode::PAYLOAD_TOO_LARGE,
                                "文件过大 / File too large: max 10MB".into(),
                            ));
                        }
                        file_data = Some(bytes.to_vec());
                    }
                    Err(e) => {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            format!("文件读取失败 / File read error: {}", e),
                        ));
                    }
                }
            }
            "session_id" => {
                if let Ok(text) = field.text().await {
                    session_id = text;
                }
            }
            _ => {
                // 忽略未知字段 / Ignore unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    let data = file_data.ok_or((
        StatusCode::BAD_REQUEST,
        "缺少文件 / Missing file field".to_string(),
    ))?;

    // 调用 CoreService 的 upload_file — 直接访问，零序列化开销
    // Call CoreService's upload_file — direct access, zero serialization overhead
    let req = atrium_bridge::grpc::atrium::UploadFileRequest {
        filename,
        data,
        mime_type,
        session_id,
    };
    let resp = core.upload_file(req).await;

    Ok(JsonResp(FileUploadResponse {
        hash: resp.hash,
        original_name: resp.original_name,
        size: resp.size as u64,
        text_extracted: resp.text_extracted,
        // 仅返回前 200 字符（与 Python gateway 兼容）/ First 200 chars only
        extracted_text: if resp.text_extracted {
            resp.extracted_text.chars().take(200).collect()
        } else {
            String::new()
        },
        error: resp.error,
    }))
}

/// GET /api/files — 文件端点信息 / File endpoint info
///
/// 返回文件上传端点的基本信息（与 Python gateway 兼容）。
/// Returns basic info about the file upload endpoint (compatible with Python gateway).
pub async fn files_info() -> impl IntoResponse {
    JsonResp(json!({
        "endpoint": "/api/files/upload",
        "method": "POST",
        "max_size": "10MB"
    }))
}

// ── 单元测试 / Unit Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// 构造一个示例 ProactiveStatus，便于在测试间复用
    /// Build a sample ProactiveStatus, reusable across tests
    fn sample_proactive_status() -> ProactiveStatus {
        ProactiveStatus {
            enabled: true,
            check_interval_secs: 600,
            max_proactive_per_day: 12,
            today_count: 3,
        }
    }

    /// 验证 ProactiveStatus 序列化包含全部字段且命名符合 snake_case 约定
    /// Verify ProactiveStatus serializes with all fields in snake_case
    #[test]
    fn test_proactive_status_serialization() {
        let status = sample_proactive_status();
        let json = serde_json::to_value(&status).expect("serialize ProactiveStatus");

        // 核心字段全部存在 / All core fields present
        assert_eq!(json["enabled"], Value::Bool(true));
        assert_eq!(json["check_interval_secs"], Value::Number(600.into()));
        assert_eq!(json["max_proactive_per_day"], Value::Number(12.into()));
        assert_eq!(json["today_count"], Value::Number(3.into()));

        // 不应有多余字段 / No extra fields
        let obj = json.as_object().expect("ProactiveStatus is object");
        assert_eq!(
            obj.len(),
            4,
            "ProactiveStatus 应仅有 4 个字段 / expected 4 fields"
        );
    }

    /// 验证 CareConfigResponse 序列化后包含 proactive_status 子对象
    /// Verify CareConfigResponse includes the proactive_status sub-object when serialized
    #[test]
    fn test_care_config_includes_proactive_status() {
        let resp = CareConfigResponse {
            enabled: true,
            greeting_interval: 4800,
            checkin_interval: 2400,
            emotion_check_interval: 300,
            quiet_start: 23,
            quiet_end: 8,
            proactive_status: sample_proactive_status(),
        };

        let json = serde_json::to_value(&resp).expect("serialize CareConfigResponse");

        // 顶层包含 proactive_status 字段 / Top-level contains proactive_status
        assert!(
            json.get("proactive_status").is_some(),
            "CareConfigResponse 缺少 proactive_status 字段 / missing proactive_status field"
        );

        // 子对象字段值正确传递 / Sub-object fields passed through correctly
        let ps = &json["proactive_status"];
        assert_eq!(ps["enabled"], Value::Bool(true));
        assert_eq!(ps["check_interval_secs"], Value::Number(600.into()));
        assert_eq!(ps["max_proactive_per_day"], Value::Number(12.into()));
        assert_eq!(ps["today_count"], Value::Number(3.into()));

        // 其余兼容字段仍存在 / Other Python-compatible fields preserved
        assert_eq!(json["enabled"], Value::Bool(true));
        assert_eq!(json["greeting_interval"], Value::Number(4800.into()));
        assert_eq!(json["checkin_interval"], Value::Number(2400.into()));
        assert_eq!(json["emotion_check_interval"], Value::Number(300.into()));
        assert_eq!(json["quiet_start"], Value::Number(23.into()));
        assert_eq!(json["quiet_end"], Value::Number(8.into()));
    }
}
