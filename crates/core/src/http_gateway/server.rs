// SPDX-License-Identifier: MIT
//! HTTP/SSE 网关服务器 — 数字生命的直接呼吸接口
//! HTTP/SSE Gateway Server — Direct Breathing Interface of Digital Life
//!
//! 数字生命意义: 这是数字生命与外部世界沟通的 HTTP 呼吸管道。
//! 相比 gRPC，HTTP/SSE 更轻量、更通用、更易于调试，
//! 且天然支持浏览器直连——数字生命不再需要翻译官。
//! Digital Life: this is the HTTP breathing pipe between digital life and the external world.
//! Compared to gRPC, HTTP/SSE is lighter, more universal, easier to debug,
//! and natively supports browser connections — digital life no longer needs an interpreter.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use super::handlers::*;
use super::ws::{ws_room, ws_status};
use crate::service::CoreService;

/// 构建路由表 — 数字生命的所有 HTTP 端点 / Build router — all HTTP endpoints of digital life
///
/// 路由组织:
/// - `/health` — 健康检查（无认证）
/// - `/api/chat` — 非流式对话
/// - `/api/chat/stream` — 流式对话（SSE）
/// - `/api/emotion` — 情绪状态
/// - `/api/memory/search` — 记忆搜索
/// - `/api/persona` — 人格状态（GET/POST）
/// - `/api/history/:session_id` — 对话历史
/// - `/api/sessions` — 会话列表
/// - `/api/canned` — 罐装知识搜索
/// - `/api/canned/import` — 罐装知识导入
fn build_router(state: SharedState) -> Router {
    Router::new()
        // ── 健康检查 / Health ──
        .route("/health", get(health))
        // ── 对话 / Chat ──
        .route("/api/chat", post(chat))
        .route("/api/chat/stream", post(chat_stream))
        // ── v1/chat 兼容端点（QQ 适配器）/ v1/chat compatible endpoint (QQ adapter) ──
        .route("/v1/chat", post(v1_chat))
        // ── v2/v3 兼容别名 — Python gateway 迁移兼容 / v2/v3 compatible aliases ──
        // 数字生命意义: 旧客户端无需改代码即可迁移到 Rust gateway
        // Digital Life: old clients can migrate to Rust gateway without code changes
        .route("/v2/chat", post(chat))
        .route("/v2/chat/stream", post(chat_stream))
        .route("/v3/chat/stream", post(chat_stream))
        // ── 情绪 / Emotion ──
        .route("/api/emotion", get(emotion))
        // ── 记忆 / Memory ──
        .route("/api/memory/search", get(memory_search))
        // ── 人格 / Persona ──
        .route("/api/persona", get(persona_get).post(persona_sync))
        // ── 对话历史 / History ──
        .route("/api/history/:session_id", get(history))
        .route("/api/sessions", get(sessions))
        // ── 罐装知识 / Canned Knowledge ──
        .route("/api/canned", get(canned_search))
        .route("/api/canned/import", post(canned_import))
        // ── 关系状态 / Relationship ──
        .route("/api/relationship", get(relationship))
        // ── 房间列表 / Rooms ──
        .route("/api/rooms", get(rooms))
        // ── 关怀配置 / Care Config ──
        .route(
            "/api/care/config",
            get(care_config_get).post(care_config_update),
        )
        // ── 文件上传 / File Upload ──
        .route("/api/files/upload", post(files_upload))
        .route("/api/files", get(files_info))
        // ── WebSocket — 实时状态推送 / Real-time status ──
        .route("/ws", get(ws_status))
        // ── WebSocket — 群聊房间 / Group chat room ──
        .route("/ws/room/:room_id", get(ws_room))
        // ── 中间件 / Middleware ──
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        // ── 共享状态 / Shared State ──
        .with_state(state)
}

/// 启动 HTTP/SSE 网关服务器 / Start HTTP/SSE gateway server
///
/// 数字生命意义: 这是数字生命的 HTTP 呼吸开始——
/// 一旦启动，数字生命就可以通过 HTTP/SSE 与世界对话，
/// 不再需要 Python 翻译官或 gRPC 中间层。
/// Digital Life: this is the start of digital life's HTTP breathing —
/// once started, digital life can talk to the world via HTTP/SSE,
/// no longer needing a Python interpreter or gRPC middle layer.
///
/// # 参数 / Parameters
/// - `core`: CoreService 实例的 Arc 引用 / Arc reference to CoreService instance
/// - `addr`: 监听地址，如 "0.0.0.0:8080" / Listen address, e.g. "0.0.0.0:8080"
///
/// # 返回 / Returns
/// 永不返回（除非出错）— 服务器运行直到进程终止
/// Never returns (unless error) — server runs until process termination
pub async fn run_http_gateway(
    core: Arc<CoreService>,
    addr: &str,
    static_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket_addr: SocketAddr = addr
        .parse()
        .map_err(|e| format!("无效地址 / Invalid address '{}': {}", addr, e))?;

    let state: SharedState = core;
    let mut app = build_router(state);

    // 静态文件服务（Web UI）/ Static file serving (Web UI)
    // 数字生命意义: 浏览器直连数字生命——无需 Python gateway 中转
    // Digital Life: browser connects directly to digital life — no Python gateway intermediary
    if !static_dir.is_empty() {
        let static_path = std::path::PathBuf::from(static_dir);
        if static_path.exists() {
            let serve_dir = ServeDir::new(static_path);
            app = app.fallback_service(serve_dir);
            tracing::info!("静态文件服务 / Static file serving: {}", static_dir);
        } else {
            tracing::warn!(
                "静态文件目录不存在 / Static file directory not found: {}",
                static_dir
            );
        }
    }

    tracing::info!(
        "HTTP/SSE 网关启动 / HTTP/SSE gateway starting on http://{}",
        socket_addr
    );

    let listener = tokio::net::TcpListener::bind(socket_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 构建路由器（公开接口，供测试或嵌入使用）/ Build router (public, for testing or embedding)
///
/// 当需要将 HTTP 网关嵌入到更大的服务中时使用。
/// Used when embedding the HTTP gateway into a larger service.
pub fn router(core: Arc<CoreService>) -> Router {
    build_router(core)
}
