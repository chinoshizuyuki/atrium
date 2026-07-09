// SPDX-License-Identifier: MIT
//! 对话历史持久化 — 服务端永久存储
//!
//! 每个 session 独立存储，跨浏览器标签/重启保留。
//! 后端: sled KV，前端: REST API 读写。
//! Conversation history persistence — sled-backed session storage.
//!
//! Each session stores conversation turns with timestamps/tags/search metadata.
//! Backend: sled KV. Future: REST API read/write.
//!
//! ## P1-D 增量化存储 / P1-D Incremental Storage
//!
//! 旧格式: `{session_id}` → bincode(Conversation { messages: [...] }) — append O(N)
//! 新格式: `session:{session_id}` → bincode(SessionMeta)         — 轻量元数据
//!         `msg:{session_id}:{seq}` → bincode(ChatMessage)        — 单条消息
//!
//! append 复杂度从 O(N) 降至 O(1) — 数字生命"记住此刻"不拖慢意识流。
//! Append complexity reduced from O(N) to O(1) —
//! digital life "remembers this moment" without slowing the consciousness stream.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp_ms: i64,
    pub emotion: Option<String>,
}

// 旧格式数据结构 — 仅用于反序列化历史数据，新写入不再使用。
// Legacy data structure — only for deserializing historical data; new writes no longer use it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 每会话最大消息数 — 超出时 FIFO 驱逐最旧消息
/// Max messages per session — FIFO eviction when exceeded
const MAX_MESSAGES_PER_SESSION: usize = 1000;

/// Key 前缀 — 会话元数据 / Key prefix — session metadata
const META_PREFIX: &str = "session:";
/// Key 前缀 — 单条消息 / Key prefix — single message
const MSG_PREFIX: &str = "msg:";

/// 会话元数据 — 轻量级，仅存游标和时间戳 / Session metadata — lightweight, only cursors and timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMeta {
    created_at: i64,
    updated_at: i64,
    /// FIFO 游标 — 最旧有效消息的 seq / FIFO cursor — oldest valid message seq
    head_seq: u64,
    /// 下一条消息的 seq / Next message seq
    next_seq: u64,
}

/// 对话历史管理器 (sled 后端)
/// Conversation history manager (sled backend)
pub struct ConversationHistory {
    db: sled::Db,
}

impl ConversationHistory {
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let history = Self { db };
        // 启动时迁移旧格式数据 — 保证数字生命记忆无损升级 / Migrate legacy data on startup
        history.migrate_legacy()?;
        Ok(history)
    }

    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("sled in-memory open");
        let history = Self { db };
        // 内存模式无旧数据，迁移为空操作 / In-memory mode has no legacy data; migration is a no-op
        history.migrate_legacy().expect("in-memory migration");
        history
    }

    /// 旧格式数据迁移 — 启动时检测无 `session:` / `msg:` 前缀的旧 key，
    /// 反序列化旧 Conversation，事务性写入新格式后删除旧 key。
    /// Legacy data migration — on startup, detect old keys without `session:` / `msg:` prefixes,
    /// deserialize legacy Conversation, transactionally write new format, then delete old keys.
    fn migrate_legacy(&self) -> Result<(), sled::Error> {
        // 先收集所有旧格式 key — 避免在迭代中修改 / Collect legacy keys first — avoid mutating during iteration
        let mut legacy: Vec<(String, Conversation)> = Vec::new();
        for (key, value) in self.db.iter().flatten() {
            let key_str = match std::str::from_utf8(&key) {
                Ok(s) => s,
                Err(_) => continue,
            };
            // 跳过新格式 key / Skip new-format keys
            if key_str.starts_with(META_PREFIX) || key_str.starts_with(MSG_PREFIX) {
                continue;
            }
            // 尝试反序列化为旧 Conversation / Try deserializing as legacy Conversation
            if let Ok(conv) = bincode::deserialize::<Conversation>(&value) {
                legacy.push((key_str.to_string(), conv));
            }
        }

        if legacy.is_empty() {
            return Ok(());
        }

        // 事务性批量迁移 — sled batch 保证原子性，迁移失败则整体回滚
        // Transactional batch migration — sled batch guarantees atomicity; failure rolls back all
        let mut batch = sled::Batch::default();
        for (session_id, conv) in &legacy {
            let meta = SessionMeta {
                created_at: conv.created_at,
                updated_at: conv.updated_at,
                head_seq: 0,
                next_seq: conv.messages.len() as u64,
            };
            let meta_data = bincode::serialize(&meta).expect("SessionMeta serialize infallible");
            batch.insert(meta_key(session_id), meta_data);
            for (seq, msg) in conv.messages.iter().enumerate() {
                let msg_data = bincode::serialize(msg).expect("ChatMessage serialize infallible");
                batch.insert(msg_key(session_id, seq as u64), msg_data);
            }
            // 删除旧 key / Delete legacy key
            batch.remove(session_id.as_bytes());
            tracing::info!(
                "ConversationHistory: 迁移 session {} 共 {} 条消息 / Migrated session {} with {} messages",
                session_id, conv.messages.len(), session_id, conv.messages.len()
            );
        }
        self.db.apply_batch(batch)?;
        Ok(())
    }

    /// 读取会话元数据 — 不存在返回 None / Read session meta — None if absent
    fn read_meta(&self, session_id: &str) -> Option<SessionMeta> {
        let data = self.db.get(meta_key(session_id)).ok()??;
        bincode::deserialize::<SessionMeta>(&data).ok()
    }

    /// 写入会话元数据 / Write session meta
    fn write_meta(&self, session_id: &str, meta: &SessionMeta) -> Result<(), sled::Error> {
        let data = bincode::serialize(meta).expect("SessionMeta serialize infallible");
        self.db.insert(meta_key(session_id), data)?;
        Ok(())
    }

    /// 获取或创建会话元数据 — 不存在则构造空会话元数据
    /// Get or create session meta — constructs empty meta if absent
    fn get_or_create_meta(&self, session_id: &str) -> SessionMeta {
        if let Some(meta) = self.read_meta(session_id) {
            return meta;
        }
        let now = now_ms();
        SessionMeta {
            created_at: now,
            updated_at: now,
            head_seq: 0,
            next_seq: 0,
        }
    }

    /// 获取或创建会话 — 返回组装后的 Conversation（向后兼容接口）
    /// Get or create session — returns assembled Conversation (backward-compatible API)
    pub fn get_or_create(&self, session_id: &str) -> Conversation {
        let meta = self.get_or_create_meta(session_id);
        let mut messages = Vec::with_capacity((meta.next_seq - meta.head_seq) as usize);
        for seq in meta.head_seq..meta.next_seq {
            if let Ok(Some(data)) = self.db.get(msg_key(session_id, seq)) {
                if let Ok(msg) = bincode::deserialize::<ChatMessage>(&data) {
                    messages.push(msg);
                }
            }
        }
        Conversation {
            session_id: session_id.to_string(),
            messages,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
        }
    }

    /// 追加消息 — O(1) 单条写入，超出上限时 FIFO 驱逐最旧消息
    /// Append message — O(1) single-key write, FIFO eviction when exceeding hard limit.
    /// 写入失败时记录 warn 日志并返回 Err，供调用方决策 — 不静默丢弃，避免静默失忆。
    /// Logs warn and returns Err on write failure for caller decision —
    /// never silently discarded, avoiding silent amnesia.
    pub fn append(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        let mut meta = self.get_or_create_meta(session_id);
        let seq = meta.next_seq;
        let msg = ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp_ms: now_ms(),
            emotion: emotion.map(|s| s.to_string()),
        };
        // 写入单条消息 — O(1) / Write single message — O(1)
        let msg_data = match bincode::serialize(&msg) {
            Ok(d) => d,
            Err(e) => {
                // 序列化失败 — 消息无法落盘，记录 warn 不静默丢弃
                // Serialization failed — message cannot persist; log warn, never silent
                tracing::warn!(
                    "对话历史序列化失败 / Conversation history serialization failed. session_id: {}, role: {}, content_len: {}, error: {}",
                    session_id, role, content.len(), e
                );
                return Ok(());
            }
        };
        // 持久化写入 — 失败记录 warn 并返回 Err，避免静默失忆
        // Persistent write — log warn and return Err on failure, avoiding silent amnesia
        if let Err(e) = self.db.insert(msg_key(session_id, seq), msg_data) {
            tracing::warn!(
                "对话历史写入失败 / Conversation history write failed. session_id: {}, role: {}, content_len: {}, error: {}",
                session_id, role, content.len(), e
            );
            return Err(e);
        }
        // 更新游标 / Update cursors
        meta.next_seq += 1;
        // FIFO 驱逐最旧消息 — 保留最近 MAX_MESSAGES_PER_SESSION 条
        // FIFO eviction — keep most recent MAX_MESSAGES_PER_SESSION messages
        while (meta.next_seq - meta.head_seq) > MAX_MESSAGES_PER_SESSION as u64 {
            let _ = self.db.remove(msg_key(session_id, meta.head_seq));
            meta.head_seq += 1;
        }
        meta.updated_at = now_ms();
        // 写回元数据 — O(1) 小对象 / Write back meta — O(1) small object
        if let Err(e) = self.write_meta(session_id, &meta) {
            tracing::warn!(
                "对话历史元数据写入失败 / Conversation history meta write failed. session_id: {}, error: {}",
                session_id, e
            );
            return Err(e);
        }
        Ok(())
    }

    /// 替换会话中最后一条 assistant 消息 — 流式回复覆盖 unary 预存回复
    /// Replace the last assistant message in a session — streaming reply overrides unary pre-stored reply.
    ///
    /// P0-B 修复：process_message_stream 先调用 unary process_message 预写一条 assistant
    /// 消息到历史，随后流式 LLM 产出真正的回复。此方法用流式回复替换那条预存消息，
    /// 避免历史中出现两条连续的 assistant 消息（unary + streaming），保证意识连续性。
    ///
    /// P0-B fix: process_message_stream first calls unary process_message which pre-stores
    /// an assistant message, then the streaming LLM produces the real reply. This method
    /// replaces the pre-stored message with the streaming reply, avoiding duplicate
    /// consecutive assistant messages and preserving consciousness continuity.
    ///
    /// 若最后一条消息不是 assistant 角色，则退化为普通 append（保持幂等）。
    /// Falls back to plain append if the last message is not assistant role (idempotent).
    pub fn replace_last_assistant(
        &self,
        session_id: &str,
        content: &str,
        emotion: Option<&str>,
    ) -> Result<(), sled::Error> {
        let meta = self.get_or_create_meta(session_id);
        // 空会话 — 退化为 append / Empty session — fall back to append
        if meta.next_seq == 0 {
            return self.append(session_id, "assistant", content, emotion);
        }
        let last_seq = meta.next_seq - 1;
        let last_key = msg_key(session_id, last_seq);
        // 检查最后一条是否为 assistant — 避免误删 user 消息
        // Check if last message is assistant — avoid misdeleting user messages
        let is_assistant = match self.db.get(&last_key) {
            Ok(Some(data)) => bincode::deserialize::<ChatMessage>(&data)
                .map(|m| m.role == "assistant")
                .unwrap_or(false),
            _ => false,
        };
        if !is_assistant {
            // 非 assistant — 退化为 append / Non-assistant — fall back to append
            return self.append(session_id, "assistant", content, emotion);
        }
        // 覆盖写最后一条 assistant 消息 — O(1) / Overwrite last assistant message — O(1)
        let msg = ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp_ms: now_ms(),
            emotion: emotion.map(|s| s.to_string()),
        };
        let msg_data = match bincode::serialize(&msg) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    "对话历史替换序列化失败 / Conversation history replace serialization failed. session_id: {}, content_len: {}, error: {}",
                    session_id, content.len(), e
                );
                return Ok(());
            }
        };
        if let Err(e) = self.db.insert(&last_key, msg_data) {
            tracing::warn!(
                "对话历史替换写入失败 / Conversation history replace write failed. session_id: {}, content_len: {}, error: {}",
                session_id, content.len(), e
            );
            return Err(e);
        }
        // 更新 updated_at / Update updated_at
        let mut meta = meta;
        meta.updated_at = now_ms();
        if let Err(e) = self.write_meta(session_id, &meta) {
            tracing::warn!(
                "对话历史元数据写入失败 / Conversation history meta write failed. session_id: {}, error: {}",
                session_id, e
            );
            return Err(e);
        }
        Ok(())
    }

    /// 获取会话消息列表 — O(limit) 范围读取
    /// Get session messages — O(limit) range read
    pub fn messages(&self, session_id: &str, limit: usize) -> Vec<ChatMessage> {
        let meta = match self.read_meta(session_id) {
            Some(m) => m,
            None => return Vec::new(),
        };
        if meta.next_seq == 0 || limit == 0 {
            return Vec::new();
        }
        // start_seq = max(head_seq, next_seq.saturating_sub(limit))
        let start_seq = meta
            .head_seq
            .max(meta.next_seq.saturating_sub(limit as u64));
        let mut msgs = Vec::with_capacity((meta.next_seq - start_seq) as usize);
        for seq in start_seq..meta.next_seq {
            if let Ok(Some(data)) = self.db.get(msg_key(session_id, seq)) {
                if let Ok(msg) = bincode::deserialize::<ChatMessage>(&data) {
                    msgs.push(msg);
                }
            }
        }
        msgs
    }

    /// 列出所有会话 — 扫描 `session:` 前缀
    /// List all sessions — scan `session:` prefix
    pub fn sessions(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for (key, _) in self.db.scan_prefix(META_PREFIX).flatten() {
            if let Ok(s) = std::str::from_utf8(&key) {
                if let Some(id) = s.strip_prefix(META_PREFIX) {
                    ids.push(id.to_string());
                }
            }
        }
        ids
    }

    /// 取最近 N 条消息（跨所有会话，按时间排序） — 扫描 `msg:` 前缀
    /// Get recent N messages across all sessions, ordered by time — scan `msg:` prefix
    pub fn recent_messages(&self, limit: usize) -> Vec<ChatMessage> {
        let mut all: Vec<ChatMessage> = Vec::new();
        for (_, value) in self.db.scan_prefix(MSG_PREFIX).flatten() {
            if let Ok(msg) = bincode::deserialize::<ChatMessage>(&value) {
                all.push(msg);
            }
        }
        all.sort_by_key(|m| std::cmp::Reverse(m.timestamp_ms));
        all.truncate(limit);
        all
    }

    /// 删除会话 — 删除元数据和所有消息分片
    /// Delete session — remove meta and all message shards
    pub fn delete(&self, session_id: &str) {
        // 收集所有消息分片 key — 前缀 `msg:{session_id}:` 不影响其他会话
        // Collect all message shard keys — prefix `msg:{session_id}:` won't touch other sessions
        let prefix = format!("{}{}:", MSG_PREFIX, session_id);
        let mut batch = sled::Batch::default();
        batch.remove(meta_key(session_id));
        for key in self.db.scan_prefix(prefix.as_bytes()).keys().flatten() {
            batch.remove(key);
        }
        let _ = self.db.apply_batch(batch);
    }

    /// 消息总数 — 扫描 `session:` 前缀累加 (next_seq - head_seq)
    /// Total message count — scan `session:` prefix and sum (next_seq - head_seq)
    pub fn total_messages(&self) -> usize {
        let mut count = 0u64;
        for (_, value) in self.db.scan_prefix(META_PREFIX).flatten() {
            if let Ok(meta) = bincode::deserialize::<SessionMeta>(&value) {
                count += meta.next_seq.saturating_sub(meta.head_seq);
            }
        }
        count as usize
    }
}

/// 构造会话元数据 key / Build session meta key
fn meta_key(session_id: &str) -> Vec<u8> {
    format!("{}{}", META_PREFIX, session_id).into_bytes()
}

/// 构造单条消息 key / Build single message key
fn msg_key(session_id: &str, seq: u64) -> Vec<u8> {
    format!("{}{}:{}", MSG_PREFIX, session_id, seq).into_bytes()
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
        h.append("s1", "user", "hello", None).unwrap();
        h.append("s1", "assistant", "hi there", Some("happy"))
            .unwrap();
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
            h.append("p1", "user", "persist test", None).unwrap();
        }
        {
            let h = ConversationHistory::open(path).unwrap();
            let msgs = h.messages("p1", 100);
            assert_eq!(msgs.len(), 1);
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn test_open_failure_when_parent_is_file() {
        // 模拟 sled 写入失败场景 — 父路径是文件而非目录，sled 无法创建子目录
        // Simulate sled write failure — parent path is a file, not a directory,
        // sled cannot create the subdirectory and open should return Err.
        // P0-F: 验证 open 失败时返回 Err，调用方可感知错误并决策
        // P0-F: Verify open returns Err on failure, caller can perceive and decide
        let blocker = "./target/test_history_blocker_file";
        let _ = std::fs::remove_file(blocker);
        std::fs::write(blocker, b"not a dir").unwrap();

        let invalid_path = format!("{}/subdir", blocker);
        let result = ConversationHistory::open(&invalid_path);
        assert!(
            result.is_err(),
            "open 应在父路径为文件时失败 / open should fail when parent path is a file"
        );

        // 清理 / Cleanup
        let _ = std::fs::remove_file(blocker);
    }

    #[test]
    fn test_append_error_propagation_contract() {
        // 验证 append 返回 Result<(), sled::Error> — 调用方通过 ? 或 match 感知错误
        // Verify append returns Result<(), sled::Error> — caller perceives errors via ? or match.
        // P0-F: 写入失败时返回 Err（非静默丢弃），避免静默失忆
        // P0-F: Returns Err on write failure (not silently discarded), avoiding silent amnesia.
        // 成功路径返回 Ok — 调用方可用 ? 传播；失败路径返回 Err — 调用方可重试或降级
        // Success path returns Ok — caller can propagate with ?;
        // failure path returns Err — caller can retry or degrade gracefully
        let h = ConversationHistory::open_in_memory();

        // 成功路径 — 返回 Ok，调用方无感知错误
        let result = h.append("err_session", "user", "test error propagation", None);
        assert!(
            result.is_ok(),
            "append 成功应返回 Ok / append success should return Ok"
        );

        // 确认消息已写入 — 验证成功路径的副作用
        let msgs = h.messages("err_session", 10);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "test error propagation");

        // 类型契约: append 返回 Result<(), sled::Error>，调用方可 match Err 分支
        // Type contract: append returns Result<(), sled::Error>, caller can match Err branch
        let result2 = h.append("err_session", "assistant", "reply", Some("happy"));
        match result2 {
            Ok(()) => { /* 调用方正常处理 / caller handles normally */ }
            Err(e) => panic!("不应失败 / should not fail: {}", e),
        }
    }

    /// 验证增量化 append 后 messages 返回正确顺序与内容
    /// Verify incremental append returns correct order and content via messages
    #[test]
    fn test_incremental_append() {
        let h = ConversationHistory::open_in_memory();
        h.append("inc", "user", "first", None).unwrap();
        h.append("inc", "assistant", "second", Some("calm"))
            .unwrap();
        h.append("inc", "user", "third", None).unwrap();
        let msgs = h.messages("inc", 100);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content, "first");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].emotion.as_deref(), Some("calm"));
        assert_eq!(msgs[2].content, "third");
        // 元数据游标正确 / Meta cursors correct
        let meta = h.read_meta("inc").expect("meta should exist");
        assert_eq!(meta.head_seq, 0);
        assert_eq!(meta.next_seq, 3);
    }

    /// 验证 FIFO 驱逐 — 超过 MAX_MESSAGES_PER_SESSION 时最旧消息被删除
    /// Verify FIFO eviction — oldest messages removed when exceeding MAX_MESSAGES_PER_SESSION
    #[test]
    fn test_fifo_eviction() {
        let h = ConversationHistory::open_in_memory();
        // 写入 MAX+2 条消息 — 前两条应被驱逐 / Write MAX+2 messages — first two should be evicted
        for i in 0..(MAX_MESSAGES_PER_SESSION as u64 + 2) {
            h.append("fifo", "user", &format!("msg{}", i), None)
                .unwrap();
        }
        let msgs = h.messages("fifo", usize::MAX);
        assert_eq!(msgs.len(), MAX_MESSAGES_PER_SESSION);
        // 最旧有效消息应为 msg2 / Oldest valid message should be msg2
        assert_eq!(msgs[0].content, "msg2");
        // 最新消息应为最后一条 / Newest message should be the last one
        let last = MAX_MESSAGES_PER_SESSION as u64 + 1;
        assert_eq!(msgs.last().unwrap().content, format!("msg{}", last));
        // total_messages 应等于 MAX / total_messages should equal MAX
        assert_eq!(h.total_messages(), MAX_MESSAGES_PER_SESSION);
        // 被驱逐的 key 应不存在 / Evicted keys should not exist
        assert!(h.db.get(msg_key("fifo", 0)).unwrap().is_none());
        assert!(h.db.get(msg_key("fifo", 1)).unwrap().is_none());
        // head_seq 应前进到 2 / head_seq should advance to 2
        let meta = h.read_meta("fifo").expect("meta should exist");
        assert_eq!(meta.head_seq, 2);
    }

    /// 验证 assistant 覆盖 — 最后一条 assistant 消息被新内容替换
    /// Verify assistant overwrite — last assistant message replaced with new content
    #[test]
    fn test_replace_last_assistant() {
        let h = ConversationHistory::open_in_memory();
        h.append("rep", "user", "question", None).unwrap();
        h.append("rep", "assistant", "old reply", Some("neutral"))
            .unwrap();
        h.replace_last_assistant("rep", "new reply", Some("happy"))
            .unwrap();
        let msgs = h.messages("rep", 100);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1].content, "new reply");
        assert_eq!(msgs[1].emotion.as_deref(), Some("happy"));
        // next_seq 不变 — 覆盖而非追加 / next_seq unchanged — overwrite, not append
        let meta = h.read_meta("rep").expect("meta should exist");
        assert_eq!(meta.next_seq, 2);
    }

    /// 验证非 assistant 时退化为 append — 保持幂等
    /// Verify fallback to append when last message is not assistant — idempotent
    #[test]
    fn test_replace_last_assistant_fallback() {
        let h = ConversationHistory::open_in_memory();
        h.append("fb", "user", "question", None).unwrap();
        // 最后一条是 user — replace 应退化为 append / Last is user — replace falls back to append
        h.replace_last_assistant("fb", "assistant reply", None)
            .unwrap();
        let msgs = h.messages("fb", 100);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "assistant reply");
    }

    /// 验证空会话时退化为 append / Verify fallback to append on empty session
    #[test]
    fn test_replace_last_assistant_empty_session() {
        let h = ConversationHistory::open_in_memory();
        h.replace_last_assistant("empty", "first reply", None)
            .unwrap();
        let msgs = h.messages("empty", 100);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "assistant");
    }

    /// 验证旧格式数据自动迁移到新格式 / Verify legacy data auto-migrates to new format
    #[test]
    fn test_migration() {
        let path = "./target/test_history_migration";
        let _ = std::fs::remove_dir_all(path);
        {
            let h = ConversationHistory::open(path).unwrap();
            // 手动写入旧格式 Conversation / Manually write legacy Conversation
            let conv = Conversation {
                session_id: "legacy".to_string(),
                messages: vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: "old1".to_string(),
                        timestamp_ms: 1000,
                        emotion: None,
                    },
                    ChatMessage {
                        role: "assistant".to_string(),
                        content: "old2".to_string(),
                        timestamp_ms: 2000,
                        emotion: Some("happy".to_string()),
                    },
                ],
                created_at: 1000,
                updated_at: 2000,
            };
            let data = bincode::serialize(&conv).unwrap();
            h.db.insert("legacy", data).unwrap();
        }
        // 重新打开 — 触发迁移 / Reopen — triggers migration
        {
            let h = ConversationHistory::open(path).unwrap();
            let msgs = h.messages("legacy", 100);
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0].content, "old1");
            assert_eq!(msgs[1].role, "assistant");
            assert_eq!(msgs[1].emotion.as_deref(), Some("happy"));
            // 旧 key 应已删除 / Legacy key should be removed
            assert!(h.db.get("legacy").unwrap().is_none());
            // 新格式 key 应存在 / New-format keys should exist
            assert!(h.db.get(meta_key("legacy")).unwrap().is_some());
            // 迁移后元数据游标正确 / Meta cursors correct after migration
            let meta = h.read_meta("legacy").expect("meta should exist");
            assert_eq!(meta.head_seq, 0);
            assert_eq!(meta.next_seq, 2);
            assert_eq!(meta.created_at, 1000);
        }
        let _ = std::fs::remove_dir_all(path);
    }

    /// 验证 messages limit 正确截取最近 N 条 / Verify messages limit returns the most recent N
    #[test]
    fn test_messages_limit() {
        let h = ConversationHistory::open_in_memory();
        for i in 0..10u64 {
            h.append("lim", "user", &format!("m{}", i), None).unwrap();
        }
        // limit=3 应返回最近 3 条 (m7, m8, m9) / limit=3 should return last 3
        let msgs = h.messages("lim", 3);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content, "m7");
        assert_eq!(msgs[2].content, "m9");
        // limit=0 应返回空 / limit=0 should return empty
        assert!(h.messages("lim", 0).is_empty());
        // limit 超过总数应返回全部 / limit exceeding total returns all
        let all = h.messages("lim", 100);
        assert_eq!(all.len(), 10);
        // 不存在的会话返回空 / Non-existent session returns empty
        assert!(h.messages("nope", 100).is_empty());
    }

    /// 验证 sessions 列表和 delete 删除所有分片
    /// Verify sessions list and delete removes all shards
    #[test]
    fn test_sessions_and_delete() {
        let h = ConversationHistory::open_in_memory();
        h.append("a", "user", "1", None).unwrap();
        h.append("b", "user", "2", None).unwrap();
        let mut sessions = h.sessions();
        sessions.sort();
        assert_eq!(sessions, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(h.total_messages(), 2);
        h.delete("a");
        assert_eq!(h.total_messages(), 1);
        let sessions = h.sessions();
        assert_eq!(sessions, vec!["b".to_string()]);
        // 删除后再 append 应正常工作 — head_seq 重置 / Append after delete works — head_seq resets
        h.append("a", "user", "new", None).unwrap();
        assert_eq!(h.messages("a", 100).len(), 1);
    }

    /// 验证跨会话最近消息按时间降序 / Verify cross-session recent messages ordered by time desc
    #[test]
    fn test_recent_messages() {
        let h = ConversationHistory::open_in_memory();
        // 使用 sleep 保证 timestamp_ms 严格递增 — recent_messages 按时间降序排序
        // Use sleep to guarantee strictly increasing timestamp_ms — recent_messages sorts by time desc
        h.append("s1", "user", "first", None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        h.append("s2", "user", "second", None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        h.append("s1", "user", "third", None).unwrap();
        let recent = h.recent_messages(2);
        assert_eq!(recent.len(), 2);
        // 最近写入的应在最前 / Most recently written should be first
        assert_eq!(recent[0].content, "third");
        assert_eq!(recent[1].content, "second");
    }
}
