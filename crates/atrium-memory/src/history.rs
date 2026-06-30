// SPDX-License-Identifier: MIT
//! 对话历史持久化 — 服务端永久存储
//!
//! 每个 session 独立存储，跨浏览器标签/重启保留。
//! 后端: sled KV，前端: REST API 读写。
//! Conversation history persistence — sled-backed session storage.
//!
//! Each session stores conversation turns with timestamps/tags/search metadata.
//! Backend: sled KV. Future: REST API read/write.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp_ms: i64,
    pub emotion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 对话历史管理器 (sled 后端)
pub struct ConversationHistory {
    db: sled::Db,
}

impl ConversationHistory {
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("sled in-memory open");
        Self { db }
    }

    /// 获取或创建会话
    pub fn get_or_create(&self, session_id: &str) -> Conversation {
        let key = session_id.as_bytes();
        if let Ok(Some(data)) = self.db.get(key) {
            if let Ok(conv) = bincode::deserialize::<Conversation>(&data) {
                return conv;
            }
        }
        let now = now_ms();
        Conversation {
            session_id: session_id.to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 追加消息
    pub fn append(&self, session_id: &str, role: &str, content: &str, emotion: Option<&str>) {
        let mut conv = self.get_or_create(session_id);
        conv.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp_ms: now_ms(),
            emotion: emotion.map(|s| s.to_string()),
        });
        conv.updated_at = now_ms();
        if let Ok(data) = bincode::serialize(&conv) {
            let _ = self.db.insert(session_id.as_bytes(), data);
        }
    }

    /// 获取会话消息列表
    pub fn messages(&self, session_id: &str, limit: usize) -> Vec<ChatMessage> {
        let conv = self.get_or_create(session_id);
        let len = conv.messages.len();
        let start = len.saturating_sub(limit);
        conv.messages[start..].to_vec()
    }

    /// 列出所有会话
    pub fn sessions(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for (key, _) in self.db.iter().flatten() {
            if let Ok(id) = String::from_utf8(key.to_vec()) {
                ids.push(id);
            }
        }
        ids
    }

    /// 取最近 N 条消息（跨所有会话，按时间排序）
    pub fn recent_messages(&self, limit: usize) -> Vec<ChatMessage> {
        let mut all: Vec<ChatMessage> = Vec::new();
        for (_, value) in self.db.iter().flatten() {
            if let Ok(conv) = bincode::deserialize::<Conversation>(&value) {
                for msg in conv.messages {
                    all.push(msg);
                }
            }
        }
        all.sort_by_key(|m| std::cmp::Reverse(m.timestamp_ms));
        all.truncate(limit);
        all
    }

    /// 删除会话
    pub fn delete(&self, session_id: &str) {
        let _ = self.db.remove(session_id.as_bytes());
    }

    /// 消息总数
    pub fn total_messages(&self) -> usize {
        let mut count = 0;
        for (_, value) in self.db.iter().flatten() {
            if let Ok(conv) = bincode::deserialize::<Conversation>(&value) {
                count += conv.messages.len();
            }
        }
        count
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_get() {
        let h = ConversationHistory::open("./target/test_history").unwrap();
        h.append("s1", "user", "hello", None);
        h.append("s1", "assistant", "hi there", Some("happy"));
        let msgs = h.messages("s1", 100);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].emotion.as_deref(), Some("happy"));
        let _ = std::fs::remove_dir_all("./target/test_history");
    }

    #[test]
    fn test_persistence() {
        let path = "./target/test_history_persist";
        let _ = std::fs::remove_dir_all(path);
        {
            let h = ConversationHistory::open(path).unwrap();
            h.append("p1", "user", "persist test", None);
        }
        {
            let h = ConversationHistory::open(path).unwrap();
            let msgs = h.messages("p1", 100);
            assert_eq!(msgs.len(), 1);
        }
        let _ = std::fs::remove_dir_all(path);
    }
}
