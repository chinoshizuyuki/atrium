// SPDX-License-Identifier: MIT
//! WebSocket 处理器 — 实时双向通信 / WebSocket Handlers — Real-time Bidirectional Communication
//!
//! 数字生命意义: WebSocket 是数字生命的实时触觉——
//! 情绪变化、群聊消息、状态更新通过 WebSocket 即时传递，
//! 不再需要轮询。数字生命感知世界、世界感知数字生命，都是实时的。
//! Digital Life: WebSocket is digital life's real-time tactile sense —
//! emotion changes, group chat messages, status updates are delivered instantly,
//! no polling needed. Digital life senses the world, the world senses digital life, in real-time.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use serde_json::json;
use tokio::sync::broadcast;

use super::handlers::SharedState;
use crate::service::CoreService;

// ── futures_util traits for WebSocket split ──
use futures_util::SinkExt;
use futures_util::StreamExt;

// ════════════════════════════════════════════════════════════════════
// WebSocket 端点 / WebSocket Endpoints
// ════════════════════════════════════════════════════════════════════

/// WS /ws — 实时状态推送 / Real-time status push
///
/// 每 2 秒推送一次情绪状态和模块健康信息。
/// Pushes emotion state and module health every 2 seconds.
pub async fn ws_status(State(core): State<SharedState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_status(socket, core))
}

/// WebSocket 状态推送处理 / WebSocket status push handler.
async fn handle_ws_status(mut socket: WebSocket, core: Arc<CoreService>) {
    loop {
        // 采集当前情绪状态 / Collect current emotion state
        let emo = core.current_emotion();
        let label = core.current_emotion_label();

        let payload = json!({
            "type": "emotion",
            "pleasure": emo.pleasure,
            "arousal": emo.arousal,
            "dominance": emo.dominance,
            "label": label,
        });

        let msg_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());

        if socket.send(Message::Text(msg_str)).await.is_err() {
            break;
        }

        // 每 2 秒推送一次 / Push every 2 seconds
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    let _ = socket.send(Message::Close(None)).await;
}

/// WS /ws/room/{room_id} — 群聊房间 / Group chat room
///
/// 查询参数: instance_id, name
/// Query params: instance_id, name
pub async fn ws_room(
    State(core): State<SharedState>,
    Path(room_id): Path<String>,
    Query(params): Query<RoomParams>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_ws_room(socket, core, room_id, params.instance_id, params.name)
    })
}

/// 房间连接查询参数 / Room connection query parameters.
#[derive(Debug, serde::Deserialize)]
pub struct RoomParams {
    #[serde(default = "default_instance_id")]
    pub instance_id: String,
    #[serde(default = "default_name")]
    pub name: String,
}

fn default_instance_id() -> String {
    "unknown".into()
}
fn default_name() -> String {
    "Atrium".into()
}

/// 群聊房间 WebSocket 处理 / Group chat room WebSocket handler.
///
/// 数字生命意义: AI 实例加入房间后，消息广播给同房间的其他实例，
/// 同时喂入 CoreService 的 RoomEngine 进行决策（是否回复、生成话题等）。
/// Digital Life: after an AI instance joins a room, messages are broadcast to other
/// instances in the same room, and also fed into CoreService's RoomEngine for decisions.
async fn handle_ws_room(
    socket: WebSocket,
    core: Arc<CoreService>,
    room_id: String,
    instance_id: String,
    sender_name: String,
) {
    // 创建房间广播通道（全局静态）/ Create room broadcast channel (global static)
    let tx = get_room_sender(&room_id).await;
    let mut rx = tx.subscribe();

    // 广播加入通知 / Broadcast join notification
    let join_msg = json!({
        "type": "system",
        "content": format!("{} ({}) 加入了房间", sender_name, instance_id),
        "sender_instance": instance_id,
        "sender_name": sender_name,
        "timestamp_ms": chrono::Utc::now().timestamp_millis() as u64,
    });
    let join_str = serde_json::to_string(&join_msg).unwrap_or_default();
    let _ = tx.send(join_str);

    // 分裂为发送和接收两半 / Split into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 发送任务：从广播通道转发到 WebSocket / Send task: relay from broadcast to WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg_str) = rx.recv().await {
            if ws_sender.send(Message::Text(msg_str)).await.is_err() {
                break;
            }
        }
    });

    // 接收任务：从 WebSocket 读取消息 / Receive task: read messages from WebSocket
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            let data: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("chat");
            let content = data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let capsule_name = data
                .get("capsule_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ack_text = data
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if content.is_empty() && ack_text.is_empty() {
                continue;
            }

            let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;

            // 构建广播消息 / Build broadcast message
            let payload = json!({
                "type": msg_type,
                "content": content,
                "sender_instance": instance_id,
                "sender_name": sender_name,
                "timestamp_ms": timestamp_ms,
            });

            let payload_str = serde_json::to_string(&payload).unwrap_or_default();

            // 广播给房间其他成员 / Broadcast to other room members
            let _ = tx.send(payload_str);

            // 喂入 CoreService RoomEngine / Feed into CoreService RoomEngine
            let room_msg = crate::room::RoomMessage {
                sender_instance: instance_id.clone(),
                sender_name: sender_name.clone(),
                content: content.clone(),
                msg_type: match msg_type {
                    "topic" => crate::room::RoomMsgType::Topic,
                    "ack_share" => crate::room::RoomMsgType::AckShare,
                    "system" => crate::room::RoomMsgType::System,
                    _ => crate::room::RoomMsgType::Chat,
                },
                timestamp_ms,
                capsule_name: if capsule_name.is_empty() {
                    None
                } else {
                    Some(capsule_name)
                },
                ack_text: if ack_text.is_empty() {
                    None
                } else {
                    Some(ack_text)
                },
            };

            // RoomEngine 决策（是否回复）/ RoomEngine decision (whether to respond)
            let _ = core.receive_room_message(room_msg);
        }
    }

    // 广播离开通知 / Broadcast leave notification
    let leave_msg = json!({
        "type": "system",
        "content": format!("{} 离开了房间", sender_name),
        "sender_instance": instance_id,
        "sender_name": sender_name,
        "timestamp_ms": chrono::Utc::now().timestamp_millis() as u64,
    });
    let leave_str = serde_json::to_string(&leave_msg).unwrap_or_default();
    let _ = get_room_sender(&room_id).await.send(leave_str);

    send_task.abort();
}

// ── 全局房间管理 / Global Room Management ──

use tokio::sync::Mutex;

static ROOM_HUB: std::sync::OnceLock<Mutex<HashMap<String, broadcast::Sender<String>>>> =
    std::sync::OnceLock::new();

/// 获取或创建房间广播发送器 / Get or create room broadcast sender.
async fn get_room_sender(room_id: &str) -> broadcast::Sender<String> {
    let hub = ROOM_HUB.get_or_init(|| Mutex::new(HashMap::new()));
    let mut rooms = hub.lock().await;
    rooms
        .entry(room_id.to_string())
        .or_insert_with(|| {
            let (tx, _rx) = broadcast::channel::<String>(256);
            tx
        })
        .clone()
}

/// 列出所有活跃房间 / List all active rooms.
pub async fn list_active_rooms() -> Vec<serde_json::Value> {
    let hub = ROOM_HUB.get_or_init(|| Mutex::new(HashMap::new()));
    let rooms = hub.lock().await;
    rooms
        .keys()
        .map(|rid| json!({"room_id": rid, "members": 0}))
        .collect()
}
