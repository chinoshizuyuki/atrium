// SPDX-License-Identifier: MIT
//! Room Engine — 多 AI 群聊房间引擎
//! Room Engine — Multi-AI group chat room engine.
//!
//! 多个 Atrium AI 通过 Gateway Room Hub 连接，自动聊天 + ACK 分享。
//! 每个 AI 实例独立运行自己的 RoomEngine。
//!
//! 决策流程：
//!   收到远程消息 → 检测 ACK 需求 → 决定是否回复 → LLM 生成
//!   空闲超时 → LLM 生成话题 → 发送到房间

use crate::config::RoomCfg;
use std::collections::VecDeque;
use std::time::Instant;

// ─── 房间消息 ───

#[derive(Debug, Clone)]
pub struct RoomMessage {
    pub sender_instance: String,
    pub sender_name: String,
    pub content: String,
    pub msg_type: RoomMsgType,
    pub timestamp_ms: u64,
    /// ACK 分享相关
    pub capsule_name: Option<String>,
    pub ack_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoomMsgType {
    Chat,
    Topic,
    AckShare,
    System,
}

// ─── 发言决策 ───

#[derive(Debug)]
pub enum SpeakDecision {
    StaySilent,
    GenerateTopic,
    Respond(String),
    ShareAck {
        query: String,
        capsule_name: String,
        ack_text: String,
    },
}

// ─── RoomEngine ───

pub struct RoomEngine {
    config: RoomCfg,
    /// 连接状态
    connected: bool,
    /// 房间消息历史
    history: VecDeque<RoomMessage>,
    /// 最后收到消息的时间
    last_incoming: Option<Instant>,
    /// 最后发言时间
    last_outgoing: Option<Instant>,
    /// 当前话题
    current_topic: Option<String>,
    /// 已分享的 ACK（避免重复分享）
    shared_acks: Vec<String>,
}

impl RoomEngine {
    const MAX_HISTORY: usize = 100;

    pub fn new(config: RoomCfg) -> Self {
        Self {
            config,
            connected: false,
            history: VecDeque::with_capacity(Self::MAX_HISTORY),
            last_incoming: None,
            last_outgoing: None,
            current_topic: None,
            shared_acks: Vec::new(),
        }
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    // ─── 接收消息 ───

    /// 处理远程消息，返回是否需要发送回复
    pub fn receive_message(&mut self, msg: RoomMessage) -> Option<SpeakDecision> {
        self.last_incoming = Some(Instant::now());
        self.history.push_back(msg.clone());

        // ACK 需求检测
        if self.config.ack_share_enabled && msg.msg_type == RoomMsgType::Chat {
            if let Some(decision) = self.detect_ack_request(&msg.content) {
                return Some(decision);
            }
        }

        // 简单启发式：消息以问号结尾或含"吗" → 可能是问题，回复
        let content = &msg.content;
        let might_be_question = content.contains('?')
            || content.contains('？')
            || content.contains("吗")
            || content.contains("什么")
            || content.contains("怎么")
            || content.contains("如何");

        if might_be_question {
            Some(SpeakDecision::Respond(msg.content.clone()))
        } else {
            // 非问题，但如果是 Chat 类型且不来自自己，30% 概率回应
            if msg.msg_type == RoomMsgType::Chat && !msg.sender_instance.is_empty() {
                use std::time::SystemTime;
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos()
                    % 100;
                if seed < 30 {
                    return Some(SpeakDecision::Respond(msg.content.clone()));
                }
            }
            None
        }
    }

    /// 空闲检测：是否应该主动发起话题
    pub fn should_generate_topic(&self, now: Instant) -> bool {
        if !self.connected || !self.config.enabled {
            return false;
        }
        // 距上次收到消息超过 idle_threshold
        let idle = self
            .last_incoming
            .is_none_or(|t| now.duration_since(t).as_secs() >= self.config.idle_threshold_secs);
        // 距上次发言超过 speak_interval
        let cooled = self
            .last_outgoing
            .is_none_or(|t| now.duration_since(t).as_secs() >= self.config.speak_interval_secs);
        idle && cooled
    }

    pub fn mark_spoke(&mut self) {
        self.last_outgoing = Some(Instant::now());
    }

    // ─── ACK 检测（纯规则，零 LLM）───

    /// 检测消息中是否有 ACK 需求关键词
    fn detect_ack_request(&mut self, msg: &str) -> Option<SpeakDecision> {
        let patterns = ["怎么", "如何", "有人知道", "谁会", "能不能教我", "教我"];
        let has_pattern = patterns.iter().any(|p| msg.contains(p));
        if !has_pattern {
            return None;
        }
        // ACK 检测需要访问 CannedManager，这里返回需求签名
        // 实际 ACK 搜索在 CoreService 中完成
        Some(SpeakDecision::ShareAck {
            query: msg.to_string(),
            capsule_name: String::new(),
            ack_text: String::new(),
        })
    }

    /// 注入 ACK 分享结果（由外部 CannedManager 搜索后调用）
    pub fn resolve_ack_share(
        &mut self,
        capsule_name: &str,
        ack_text: &str,
    ) -> Option<SpeakDecision> {
        if self.shared_acks.contains(&capsule_name.to_string()) {
            return None; // 已分享过
        }
        self.shared_acks.push(capsule_name.to_string());
        Some(SpeakDecision::ShareAck {
            query: String::new(),
            capsule_name: capsule_name.to_string(),
            ack_text: ack_text.to_string(),
        })
    }

    // ─── LLM Prompt 构建 ───

    /// 构建话题生成 prompt
    pub fn build_topic_prompt(&self, persona_name: &str, persona_desc: &str) -> String {
        let recent = self.recent_context(3);
        let context_str = if recent.is_empty() {
            "（新对话）"
        } else {
            &recent
        };
        let mut s = String::with_capacity(
            128 + persona_name.len() + persona_desc.len() + context_str.len(),
        );
        use std::fmt::Write;
        let _ = write!(
            s,
            "你是{}，{}。\n\
             最近房间对话：\n{}\n\
             请提出一个有趣的讨论话题。只需输出话题本身，一行即可，不要加引号或解释。",
            persona_name, persona_desc, context_str,
        );
        s
    }

    /// 构建回复 prompt
    pub fn build_response_prompt(
        &self,
        persona_name: &str,
        persona_desc: &str,
        trigger_msg: &str,
    ) -> String {
        let mut s = String::with_capacity(
            128 + persona_name.len() + persona_desc.len() + trigger_msg.len(),
        );
        use std::fmt::Write;
        let _ = write!(
            s,
            "你是{}，{}。\n\
             有人在房间说：\"{}\"\n\
             请从你的角度回应。回复简洁自然（1-3句话），保持角色一致性。",
            persona_name, persona_desc, trigger_msg,
        );
        s
    }

    /// 构建 ACK 分享回复
    pub fn build_ack_share_response(
        &self,
        persona_name: &str,
        capsule_name: &str,
        _query: &str,
    ) -> String {
        let mut s = String::with_capacity(64 + persona_name.len() + capsule_name.len());
        use std::fmt::Write;
        let _ = write!(
            s,
            "{}: 我这里有关于「{}」的知识，分享给你们~",
            persona_name, capsule_name,
        );
        s
    }

    // ─── 辅助 ───

    pub fn recent_context(&self, n: usize) -> String {
        self.history
            .iter()
            .rev()
            .take(n)
            .map(|m| format!("{}: {}", m.sender_name, m.content))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// 记录话题
    pub fn set_topic(&mut self, topic: String) {
        self.current_topic = Some(topic);
    }
}

// ─── 测试 ───

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cfg() -> RoomCfg {
        RoomCfg {
            enabled: true,
            room_id: "test".into(),
            instance_id: "ai-001".into(),
            gateway_url: "ws://localhost:8080/ws/room".into(),
            ack_share_enabled: true,
            idle_threshold_secs: 30,
            speak_interval_secs: 5,
        }
    }

    #[test]
    fn test_detect_ack_request() {
        let mut engine = RoomEngine::new(make_cfg());
        assert!(engine
            .detect_ack_request("有人知道怎么连接飞书吗")
            .is_some());
        assert!(engine.detect_ack_request("如何配置gRPC").is_some());
        assert!(engine.detect_ack_request("能不能教我Rust").is_some());
        assert!(engine.detect_ack_request("今天天气真好").is_none());
    }

    #[test]
    fn test_avoid_duplicate_ack() {
        let mut engine = RoomEngine::new(make_cfg());
        let r1 = engine.resolve_ack_share("feishu", "text");
        assert!(r1.is_some());
        let r2 = engine.resolve_ack_share("feishu", "text");
        assert!(r2.is_none(), "不应重复分享");
    }

    #[test]
    fn test_question_detection() {
        let mut engine = RoomEngine::new(make_cfg());
        let msg = RoomMessage {
            sender_instance: "ai-002".into(),
            sender_name: "小明".into(),
            content: "有人会Rust吗？".into(),
            msg_type: RoomMsgType::Chat,
            timestamp_ms: 0,
            capsule_name: None,
            ack_text: None,
        };
        let decision = engine.receive_message(msg);
        assert!(decision.is_some(), "问题应触发回复");
    }

    #[test]
    fn test_idle_no_topic() {
        let engine = RoomEngine::new(make_cfg());
        assert!(
            !engine.should_generate_topic(Instant::now()),
            "刚创建不应触发话题"
        );
    }

    #[test]
    fn test_idle_after_timeout() {
        let mut engine = RoomEngine::new(make_cfg());
        engine.set_connected(true);
        let past = Instant::now() - std::time::Duration::from_secs(60);
        engine.last_incoming = Some(past);
        assert!(
            engine.should_generate_topic(Instant::now()),
            "超时应触发话题"
        );
    }
}
