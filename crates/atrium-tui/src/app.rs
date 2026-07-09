// SPDX-License-Identifier: MIT
//! TUI 应用状态机 / TUI application state

use anyhow::{anyhow, Result};
use ratatui::widgets::ListState;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Atrium,
    System,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub text: String,
    pub ts: chrono::DateTime<chrono::Local>,
}

/// TUI 事件
pub enum AppEvent {
    /// 用户发送消息 (触发 SSE 流)
    #[allow(dead_code)]
    SendMessage(String),
    /// 流开始
    StreamStart,
    /// 流式 token
    StreamToken(String, String),
    /// 流结束 (元数据 JSON)
    StreamDone(String),
    /// 流错误
    StreamError(String),
    /// 状态刷新 (情绪)
    Emotion(EmotionSnapshot),
    /// 状态刷新 (人格)
    Persona(PersonaSnapshot),
    /// 状态刷新 (健康)
    Health(HealthSnapshot),
    /// 状态刷新出错
    StatusError(String),
}

#[derive(Debug, Clone, Default)]
pub struct EmotionSnapshot {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct PersonaSnapshot {
    pub name: String,
    pub master_name: String,
    pub version: String,
    pub relationship_stage: String,
    pub maturity_stage: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct HealthSnapshot {
    pub ok: bool,
    pub event_count: u64,
    pub uptime_seconds: u64,
    pub module_states: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ChatState {
    pub list_state: ListState,
}

pub struct App {
    pub gateway: String,
    pub session_id: String,
    pub user_id: String,
    pub http: Client,
    pub input: String,
    pub messages: Vec<Message>,
    pub streaming: bool,
    pub chat_state: ChatState,

    // 状态面板数据
    pub emotion_label: String,
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub relationship_stage: String,
    pub maturity_stage: String,
    pub module_states: Vec<(String, String)>,
    pub module_count: usize,
}

impl App {
    pub fn new(gateway: String, session: String, user: String, http: Client) -> Self {
        let mut list_state = ListState::default();
        list_state.select(None);
        Self {
            gateway,
            session_id: session,
            user_id: user,
            http,
            input: String::new(),
            messages: Vec::new(),
            streaming: false,
            chat_state: ChatState { list_state },
            emotion_label: "—".into(),
            pleasure: 0.0,
            arousal: 0.0,
            dominance: 0.0,
            relationship_stage: "—".into(),
            maturity_stage: "—".into(),
            module_states: Vec::new(),
            module_count: 0,
        }
    }

    /// 处理事件，返回 true 表示应退出主循环
    pub fn handle_event(&mut self, ev: AppEvent) -> bool {
        match ev {
            AppEvent::SendMessage(_) => {
                // 由调用方处理
            }
            AppEvent::StreamStart => {
                self.streaming = true;
                self.messages.push(Message {
                    role: MessageRole::Atrium,
                    text: String::new(),
                    ts: chrono::Local::now(),
                });
                self.scroll_to_bottom();
            }
            AppEvent::StreamToken(token, emotion) => {
                self.streaming = true;
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Atrium {
                        last.text.push_str(&token);
                    }
                }
                if !emotion.is_empty() {
                    self.emotion_label = emotion;
                }
                self.scroll_to_bottom();
            }
            AppEvent::StreamDone(meta) => {
                self.streaming = false;
                if !meta.is_empty() {
                    if let Some(last) = self.messages.last_mut() {
                        if last.role == MessageRole::Atrium && last.text.is_empty() {
                            last.text = format!("(流结束，无内容: {})", meta);
                        }
                    }
                }
                self.scroll_to_bottom();
            }
            AppEvent::StreamError(msg) => {
                self.streaming = false;
                self.messages.push(Message {
                    role: MessageRole::System,
                    text: format!("✗ {}", msg),
                    ts: chrono::Local::now(),
                });
                self.scroll_to_bottom();
            }
            AppEvent::Emotion(e) => {
                self.emotion_label = e.label;
                self.pleasure = e.pleasure;
                self.arousal = e.arousal;
                self.dominance = e.dominance;
            }
            AppEvent::Persona(p) => {
                self.relationship_stage = p.relationship_stage;
                self.maturity_stage = p.maturity_stage;
            }
            AppEvent::Health(h) => {
                self.module_states = h.module_states;
                self.module_count = self.module_states.len();
            }
            AppEvent::StatusError(msg) => {
                // 静默: 状态刷新失败不弹消息，避免刷屏
                let _ = msg;
            }
        }
        false
    }

    pub fn scroll_up(&mut self) {
        let len = self.messages.len();
        if len == 0 {
            return;
        }
        let cur = self.chat_state.list_state.selected().unwrap_or(len);
        if cur > 0 {
            self.chat_state.list_state.select(Some(cur - 1));
        }
    }

    pub fn scroll_down(&mut self) {
        let len = self.messages.len();
        if len == 0 {
            return;
        }
        let cur = self.chat_state.list_state.selected().unwrap_or(0);
        if cur < len {
            self.chat_state.list_state.select(Some(cur + 1));
        } else {
            self.chat_state.list_state.select(None);
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.chat_state.list_state.select(None);
    }
}

/// 后台定时刷新状态
pub async fn refresh_status(
    http: &Client,
    gateway: &str,
    tx: &mpsc::Sender<AppEvent>,
) -> Result<()> {
    // 并发拉取 emotion / persona / health
    let emo_url = format!("{}/api/emotion", gateway);
    let per_url = format!("{}/api/persona", gateway);
    let hlt_url = format!("{}/health", gateway);
    let (emo, per, hlt) = tokio::join!(
        fetch_json::<EmotionResp>(http, &emo_url),
        fetch_json::<PersonaResp>(http, &per_url),
        fetch_json::<HealthResp>(http, &hlt_url),
    );

    if let Ok(e) = emo {
        let _ = tx
            .send(AppEvent::Emotion(EmotionSnapshot {
                pleasure: e.pleasure,
                arousal: e.arousal,
                dominance: e.dominance,
                label: e.label,
            }))
            .await;
    }
    if let Ok(p) = per {
        let _ = tx
            .send(AppEvent::Persona(PersonaSnapshot {
                name: p.name,
                master_name: p.master_name,
                version: p.version,
                relationship_stage: p.relationship_stage,
                maturity_stage: p.maturity_stage,
            }))
            .await;
    }
    if let Ok(h) = hlt {
        let mut modules = Vec::new();
        if let serde_json::Value::Object(map) = &h.module_states {
            for (k, v) in map {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                modules.push((k.clone(), s));
            }
        }
        let _ = tx
            .send(AppEvent::Health(HealthSnapshot {
                ok: h.ok,
                event_count: h.event_count,
                uptime_seconds: h.uptime_seconds,
                module_states: modules,
            }))
            .await;
    }
    Ok(())
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(http: &Client, url: &str) -> Result<T> {
    let resp = http.get(url).send().await.map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!("{} 返回 {}", url, resp.status()));
    }
    let body = resp.json::<T>().await.map_err(|e| anyhow!(e))?;
    Ok(body)
}

#[derive(Debug, Deserialize)]
struct EmotionResp {
    pleasure: f32,
    arousal: f32,
    dominance: f32,
    label: String,
}

#[derive(Debug, Deserialize)]
struct PersonaResp {
    name: String,
    master_name: String,
    version: String,
    relationship_stage: String,
    maturity_stage: String,
}

#[derive(Debug, Deserialize)]
struct HealthResp {
    ok: bool,
    event_count: u64,
    uptime_seconds: u64,
    module_states: serde_json::Value,
}
