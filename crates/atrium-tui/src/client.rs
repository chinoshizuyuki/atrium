// SPDX-License-Identifier: MIT
//! HTTP/SSE 客户端 — 直连 Rust Gateway

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::app::AppEvent;

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: String,
    pub user_id: String,
    pub channel: String,
    pub model_type: String,
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    token: Option<String>,
    emotion: Option<String>,
    done: Option<bool>,
    #[serde(default)]
    meta: serde_json::Value,
}

/// 发起 SSE 流式对话，返回事件接收端
pub async fn chat_stream(
    http: &Client,
    gateway: &str,
    req: ChatRequest,
) -> Result<mpsc::Receiver<AppEvent>> {
    let url = format!("{}/api/chat/stream", gateway);
    let resp = http.post(&url).json(&req).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("chat_stream HTTP {} : {}", status, body));
    }

    let (tx, rx) = mpsc::channel::<AppEvent>(64);
    let mut byte_stream = resp.bytes_stream();
    tokio::spawn(async move {
        let mut buf = String::new();
        use futures_util::StreamExt;
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buf.push_str(&String::from_utf8_lossy(&chunk));
                    // SSE 事件以空行分隔 / SSE events are separated by blank lines
                    while let Some(pos) = buf.find("\n\n") {
                        let event_str = buf[..pos].to_string();
                        buf = buf[pos + 2..].to_string();
                        if let Some(ev) = parse_sse_event(&event_str) {
                            let is_done = ev.done.unwrap_or(false);
                            let token = ev.token.unwrap_or_default();
                            let emotion = ev.emotion.unwrap_or_default();
                            if !token.is_empty() {
                                let _ = tx.send(AppEvent::StreamToken(token, emotion)).await;
                            }
                            if is_done {
                                let meta_str = if ev.meta.is_null() {
                                    String::new()
                                } else {
                                    ev.meta.to_string()
                                };
                                let _ = tx.send(AppEvent::StreamDone(meta_str)).await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(AppEvent::StreamError(format!("流读取失败: {}", e)))
                        .await;
                    return;
                }
            }
        }
        // 流自然结束 (无 done 事件)
        let _ = tx.send(AppEvent::StreamDone(String::new())).await;
    });

    Ok(rx)
}

fn parse_sse_event(raw: &str) -> Option<SseEvent> {
    // SSE 格式: 多行 "data: xxx"，取 data 字段拼接
    let mut data_parts: Vec<&str> = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            data_parts.push(rest.trim());
        }
    }
    if data_parts.is_empty() {
        return None;
    }
    let data = data_parts.join("\n");
    serde_json::from_str(&data).ok()
}
