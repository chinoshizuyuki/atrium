// SPDX-License-Identifier: MIT
//! 协议定义
//! Protocol definitions — Bridge event types and serialization contracts.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 协议版本号 — 每次不兼容变更时递增
pub const BRIDGE_PROTOCOL_VERSION: u32 = 1;

// 基础类型

/// PAD 情感状态 (Pleasure-Arousal-Dominance)
///
/// 与 atrium-emotion crate 共享定义
/// 所有值范围 [-1.0, 1.0]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct EmotionState {
    /// 愉悦度 — 正面/负面
    pub pleasure: f32,
    /// 唤醒度 — 兴奋/平静
    pub arousal: f32,
    /// 支配度 — 控制/顺从
    pub dominance: f32,
}

impl EmotionState {
    pub const fn new(pleasure: f32, arousal: f32, dominance: f32) -> Self {
        Self {
            pleasure,
            arousal,
            dominance,
        }
    }

    /// 中性情感（默认状态）
    pub const fn neutral() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// 转换为情感标签文本
    pub fn label(&self) -> &'static str {
        match (self.pleasure, self.arousal, self.dominance) {
            (p, _, _) if p > 0.3 => "happy",
            (p, _, _) if p < -0.3 => "sad",
            (_, a, _) if a > 0.3 => "excited",
            (_, a, _) if a < -0.3 => "calm",
            (_, _, d) if d > 0.3 => "confident",
            (_, _, d) if d < -0.3 => "shy",
            _ => "neutral",
        }
    }
}

impl Default for EmotionState {
    fn default() -> Self {
        Self::neutral()
    }
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,
    pub timestamp_ms: i64,
    pub emotion_at_time: EmotionState,
    pub importance: f32,
    pub kind: MemoryKind,
}

impl MemoryEntry {
    pub fn new(content: String, emotion: EmotionState, kind: MemoryKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            timestamp_ms: Utc::now().timestamp_millis(),
            emotion_at_time: emotion,
            importance: 0.5,
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Conversation,
    Fact,
    Reflection,
    Experience,
    System,
}

// 核心请求/响应

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageRequest {
    pub message: String,
    pub channel: String,
    pub user_id: String,
    pub session_id: String,
}

/// 表达元数据 — 韵律/体态/节奏参数，供前端渲染和 TTS 使用
/// Expression metadata — prosody/kinesics/timing params for frontend rendering and TTS
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpressionMetadata {
    /// 韵律参数（SSML 兼容）
    pub prosody: ProsodyMeta,
    /// 体态参数（动画指令）
    pub kinesics: KinesicsMeta,
    /// 节奏参数（回复节奏控制）
    pub timing: TimingMeta,
}

/// 韵律元数据 — 供 TTS 引擎使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProsodyMeta {
    /// 基频偏移（半音）
    pub pitch_offset: f32,
    /// 语速因子（1.0=正常）
    pub rate: f32,
    /// 能量因子（1.0=正常）
    pub energy: f32,
    /// 句间停顿（ms）
    pub pause_duration_ms: f32,
    /// 音色温暖度 [0,1]
    pub warmth: f32,
    /// SSML 属性字符串
    pub ssml_attrs: String,
}

/// 体态元数据 — 供渲染引擎动画使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinesicsMeta {
    /// 头部倾斜角度
    pub head_tilt: f32,
    /// 肩膀展开度 [0,1]
    pub shoulder_openness: f32,
    /// 前倾/后仰 [-0.5,0.5]
    pub lean: f32,
    /// 眼神接触强度 [0,1]
    pub eye_contact: f32,
    /// 手势活跃度 [0,1]
    pub gesture_activity: f32,
    /// 呼吸频率因子
    pub breath_rate: f32,
    /// 动画指令 JSON
    pub animation_commands: String,
}

/// 节奏元数据 — 供回复调度使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMeta {
    /// 打字延迟因子（1.0=正常）
    pub typing_delay_factor: f32,
    /// 句间停顿（ms）
    pub inter_sentence_pause_ms: f32,
    /// 犹豫概率 [0,1]
    pub hesitation_prob: f32,
    /// 分段发送概率 [0,1]
    pub segmented_send_prob: f32,
    /// 紧迫感 [0,1]
    pub urgency: f32,
}

impl Default for ProsodyMeta {
    fn default() -> Self {
        Self {
            pitch_offset: 0.0,
            rate: 1.0,
            energy: 1.0,
            pause_duration_ms: 400.0,
            warmth: 0.5,
            ssml_attrs: String::new(),
        }
    }
}

impl Default for KinesicsMeta {
    fn default() -> Self {
        Self {
            head_tilt: 0.0,
            shoulder_openness: 0.5,
            lean: 0.0,
            eye_contact: 0.5,
            gesture_activity: 0.3,
            breath_rate: 1.0,
            animation_commands: String::new(),
        }
    }
}

impl Default for TimingMeta {
    fn default() -> Self {
        Self {
            typing_delay_factor: 1.0,
            inter_sentence_pause_ms: 500.0,
            hesitation_prob: 0.1,
            segmented_send_prob: 0.3,
            urgency: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageResponse {
    pub reply: String,
    pub emotion: String,
    pub actions: Vec<String>,
    /// 表达元数据（韵律/体态/节奏），expression 启用时填充
    #[serde(default)]
    pub expression: Option<ExpressionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryRequest {
    pub query: String,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMemoryResponse {
    pub results: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub ok: bool,
    pub event_count: u64,
    pub uptime_seconds: u64,
    pub module_states: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeProtocolConfig {
    pub grpc_addr: String,
    pub shm_path: String,
    pub shm_size: usize,
}

impl Default for BridgeProtocolConfig {
    fn default() -> Self {
        Self {
            grpc_addr: "/tmp/atrium.sock".into(),
            shm_path: "/dev/shm/atrium_render".into(),
            shm_size: 65536,
        }
    }
}

// 事件定义

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEvent {
    UserMessage {
        channel: String,
        content: String,
        user_id: String,
        timestamp_ms: i64,
    },
    LlmResponse {
        content: String,
        timestamp_ms: i64,
        emotion: EmotionState,
        /// 表达元数据（韵律/体态/节奏）
        #[serde(default)]
        expression: Option<ExpressionMetadata>,
    },
    ExternalEvent {
        event_type: String,
        payload: String,
    },
    EmotionUpdate(EmotionState),
    RenderAction {
        action: String,
        intensity: f32,
    },
    Heartbeat,
    SystemCommand(String),
    /// — Room 群聊：远程消息到达
    RoomIncoming {
        room_id: String,
        sender_instance: String,
        sender_name: String,
        content: String,
        msg_type: String, // chat/topic/ack_share/system
        timestamp_ms: u64,
        capsule_name: String,
        ack_text: String,
    },
    /// — Room 群聊：本地 AI 发言（由 Gateway 消费广播到房间）
    RoomOutgoing {
        room_id: String,
        sender_name: String,
        content: String,
        msg_type: String, // chat/topic/ack_share
        capsule_name: String,
        ack_text: String,
    },
}
