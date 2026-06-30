// SPDX-License-Identifier: MIT
#![allow(dead_code)]

//! 记忆管理器 — 核心数据结构与 STM/LTM 双层存储管线
//! MemoryManager — Core data structures and STM/LTM dual-layer storage pipeline.
//!
//! 包含 StmBuffer（短期记忆环形缓冲区）、SledLtm（长期记忆 sled 持久化）、
//! MemoryManager（STM 满时自动溢出到 LTM）以及所有子模块的公共导出。
//! Includes StmBuffer (short-term ring buffer), SledLtm (sled-backed long-term memory),
//! MemoryManager (auto-spill from STM to LTM), and all public submodule re-exports.

use serde::{Deserialize, Serialize};

pub mod anticipation_store;
pub mod associative;
pub mod atrium_vault;
pub mod canned;
pub mod consolidation;
pub mod diary_store;
pub mod embedding_fallback;
pub mod emotion_store;
pub mod empathy;
pub mod evidence;
pub mod fact_extractor;
pub mod fact_store;
pub mod feedback;
pub mod fts5_index;
pub mod graph_store;
pub mod history;
#[cfg(feature = "embedding")]
pub mod index;
pub mod inner_monologue;
pub mod intelligence;
pub mod key_fact_cache;
pub mod maturity;
pub mod perception;
pub mod persona;
pub mod preference;
pub mod reflection;
pub mod relationship;
pub mod replay;
pub mod rules;
pub mod sanctuary;
pub mod selfplay;
pub mod summarizer;
pub mod teach_detector;
pub mod token_budget;
pub mod user_model;

pub mod emotional_arc;
pub mod followup_tracker;
pub mod kinesics_mapper;
pub mod prosody_mapper;
pub mod style_memory;
pub mod style_modulator;
pub mod subtext_engine;
pub mod timing_mapper;

pub mod conflict_pattern_learner;
pub mod conflict_reconciliation;
pub mod conflict_store;
pub mod emotional_demand_boundary;
pub mod life_narrative;
pub mod relationship_aware_boundary;
pub mod self_care_boundary;

pub mod emotional_irrationality;
pub mod irrationality_store;
pub mod narrative_store;

pub mod anniversary_system;
pub mod lunar;
pub mod ritual_detector;
pub mod ritual_store;
pub mod seasonal_awareness;
pub mod vulnerability_store;
pub mod vulnerability_window;

pub mod file_store;
pub mod llm_client;
pub mod monologue_gen;
pub mod prompts;
pub mod reminder_store;
pub mod time_parser;

/// 记忆内容类型 — 支持文本、图片、视频和文件四种载体
/// Memory content type — Supports text, image, video, and file carriers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryContent {
    Text(String),
    Image {
        path: String,
        caption: Option<String>,
    },
    Video {
        path: String,
        caption: Option<String>,
    },
    File {
        path: String,
        mime: String,
        caption: Option<String>,
    },
}

impl MemoryContent {
    pub fn content_str(&self) -> String {
        match self {
            MemoryContent::Text(s) => s.clone(),
            MemoryContent::Image { path, caption } => {
                if let Some(cap) = caption {
                    format!("{} ({})", path, cap)
                } else {
                    path.clone()
                }
            }
            MemoryContent::Video { path, caption } => {
                if let Some(cap) = caption {
                    format!("{} ({})", path, cap)
                } else {
                    path.clone()
                }
            }
            MemoryContent::File {
                path,
                mime,
                caption,
            } => {
                if let Some(cap) = caption {
                    format!("{} ({}) [{}]", path, cap, mime)
                } else {
                    format!("{} [{}]", path, mime)
                }
            }
        }
    }
}

/// 单条记忆 — 包含时间戳、角色、内容和重要度
/// Single memory entry — Contains timestamp, role, content, and importance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 时间戳（毫秒） / Timestamp in milliseconds
    pub timestamp: i64,
    /// 角色: "user" | "assistant" | "system" / Role tag
    pub role: String,
    /// 内容 / Content payload
    pub content: MemoryContent,
    /// 重要度 [0.0, 1.0] / Importance score
    pub importance: f32,
}

impl MemoryEntry {
    pub fn new(role: &str, content: MemoryContent) -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp_millis(),
            role: role.to_string(),
            content,
            importance: 0.0,
        }
    }

    pub fn with_importance(mut self, imp: f32) -> Self {
        self.importance = imp.clamp(0.0, 1.0);
        self
    }

    pub fn content_str(&self) -> String {
        self.content.content_str()
    }
}

/// 短期记忆环形缓冲区 — 固定容量，满时挤出最旧条目
/// Short-term memory ring buffer — Fixed capacity, evicts oldest entry when full.
pub struct StmBuffer {
    buffer: Vec<MemoryEntry>,
    capacity: usize,
    head: usize,
    count: usize,
}

impl StmBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            head: 0,
            count: 0,
        }
    }

    /// 压入新条目，缓冲区满时返回被挤出的旧条目
    /// Push a new entry; returns the evicted oldest entry when buffer is full.
    pub fn push(&mut self, entry: MemoryEntry) -> Option<MemoryEntry> {
        let evicted = if self.count == self.capacity {
            Some(std::mem::replace(&mut self.buffer[self.head], entry))
        } else {
            self.buffer.push(entry);
            self.count += 1;
            None
        };
        self.head = (self.head + 1) % self.capacity;
        evicted
    }

    /// 取最近 N 条，从最新到最旧
    /// Get the most recent N entries, ordered newest-first.
    pub fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
        let n = n.min(self.count);
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (self.head + self.capacity - 1 - i) % self.capacity;
            result.push(&self.buffer[idx]);
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

/// 长期记忆存储接口 — 提供写入、读取、删除、扫描和计数操作
/// Long-term memory storage interface — Provides insert, get, delete, scan, and count operations.
pub trait LtmStore: Send + Sync {
    /// 写入记忆，返回分配 id / Insert a memory entry, returns the assigned id
    fn insert(&mut self, entry: &MemoryEntry) -> anyhow::Result<u64>;
    /// 按 id 读取 / Read by id
    fn get(&self, id: u64) -> anyhow::Result<Option<MemoryEntry>>;
    /// 删除 / Delete by id
    fn delete(&mut self, id: u64) -> anyhow::Result<()>;
    /// 扫描全部（较慢） / Scan all entries (slow path)
    fn scan(&self) -> anyhow::Result<Vec<(u64, MemoryEntry)>>;
    /// 条目总数 / Total entry count
    fn count(&self) -> anyhow::Result<u64>;
}

/// 基于 sled 的 LTM 实现 — 使用 sled 嵌入式数据库持久化记忆
/// Sled-backed LTM implementation — Persists memory entries using the sled embedded database.
pub struct SledLtm {
    db: sled::Db,
}

impl SledLtm {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("memory_lib init");
        Self { db }
    }

    fn next_id(&mut self) -> anyhow::Result<u64> {
        let key = b"__next_id__";
        let id = self
            .db
            .get(key)?
            .map(|v| {
                let raw = v.as_ref();
                if raw.len() != 8 {
                    anyhow::bail!(
 "next_id counter corrupted: expected 8 bytes, got {}. Id space may be unrecoverable.",
 raw.len()
 );
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(raw);
                Ok(u64::from_be_bytes(bytes))
            })
            .transpose()?
            .unwrap_or(1);
        // 防止溢出导致 ID 回绕复用
        let next = id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("next_id overflow: id={} exceeds u64::MAX", id))?;
        self.db.insert(key, &next.to_be_bytes())?;
        Ok(id)
    }
}

impl LtmStore for SledLtm {
    fn insert(&mut self, entry: &MemoryEntry) -> anyhow::Result<u64> {
        let id = self.next_id()?;
        self.db
            .insert(id.to_be_bytes(), bincode::serialize(entry)?)?;
        Ok(id)
    }

    fn get(&self, id: u64) -> anyhow::Result<Option<MemoryEntry>> {
        match self.db.get(id.to_be_bytes())? {
            Some(ivec) => Ok(Some(bincode::deserialize(&ivec)?)),
            None => Ok(None),
        }
    }

    fn delete(&mut self, id: u64) -> anyhow::Result<()> {
        self.db.remove(id.to_be_bytes())?;
        Ok(())
    }

    fn scan(&self) -> anyhow::Result<Vec<(u64, MemoryEntry)>> {
        let mut results = Vec::new();
        for item in self.db.iter() {
            let (key, value) = item?;
            if key.as_ref() == b"__next_id__" {
                continue;
            }
            let id = {
                let bytes: [u8; 8] = key
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid key length"))?;
                u64::from_be_bytes(bytes)
            };
            let entry: MemoryEntry = bincode::deserialize(&value)?;
            results.push((id, entry));
        }
        Ok(results)
    }

    fn count(&self) -> anyhow::Result<u64> {
        Ok(self.db.len().saturating_sub(1) as u64)
    }
}

/// StmBufferlike 抽象 STM 操作, 让 MemoryManager 可以泛型使用 StmBuffer
pub trait StmBufferlike {
    fn push(&mut self, entry: MemoryEntry) -> Option<MemoryEntry>;
    fn recent(&self, n: usize) -> Vec<&MemoryEntry>;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
}

impl StmBufferlike for StmBuffer {
    fn push(&mut self, entry: MemoryEntry) -> Option<MemoryEntry> {
        StmBuffer::push(self, entry)
    }
    fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
        StmBuffer::recent(self, n)
    }
    fn is_empty(&self) -> bool {
        StmBuffer::is_empty(self)
    }
    fn len(&self) -> usize {
        StmBuffer::len(self)
    }
}

/// MemoryManager 统一管理 STM + LTM, STM 满时自动溢出到 LTM
pub struct MemoryManager<S: StmBufferlike, L: LtmStore> {
    stm: S,
    ltm: L,
    ltm_enabled: bool,
}

impl<S: StmBufferlike, L: LtmStore> MemoryManager<S, L> {
    pub fn new(stm: S, ltm: L) -> Self {
        Self {
            stm,
            ltm,
            ltm_enabled: true,
        }
    }

    /// 写入一条记忆, STM 满时溢出条目自动持久化到 LTM
    pub fn remember(&mut self, entry: MemoryEntry) -> anyhow::Result<()> {
        if let Some(evicted) = self.stm.push(entry) {
            if self.ltm_enabled {
                self.ltm.insert(&evicted)?;
            }
        }
        Ok(())
    }

    /// 从 STM 获取最近 N 条
    pub fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
        self.stm.recent(n)
    }

    /// 从 LTM 按 id 读取
    pub fn recall(&self, id: u64) -> anyhow::Result<Option<MemoryEntry>> {
        self.ltm.get(id)
    }

    /// 禁用/启用 LTM 持久化
    pub fn set_ltm_enabled(&mut self, enabled: bool) {
        self.ltm_enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── StmBuffer 测试 ──

    #[test]
    fn test_stm_push_and_recent() {
        let mut buf = StmBuffer::new(3);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);

        buf.push(MemoryEntry::new("user", MemoryContent::Text("a".into())));
        buf.push(MemoryEntry::new("user", MemoryContent::Text("b".into())));
        buf.push(MemoryEntry::new("user", MemoryContent::Text("c".into())));
        assert_eq!(buf.len(), 3);
        assert!(!buf.is_empty());

        let recent = buf.recent(3);
        assert_eq!(recent.len(), 3);
        // 最新在最前
        assert_eq!(recent[0].content_str(), "c");
        assert_eq!(recent[1].content_str(), "b");
        assert_eq!(recent[2].content_str(), "a");
    }

    #[test]
    fn test_stm_eviction() {
        let mut buf = StmBuffer::new(2);
        buf.push(MemoryEntry::new("user", MemoryContent::Text("a".into())));
        buf.push(MemoryEntry::new("user", MemoryContent::Text("b".into())));
        // 第三条应该挤出 a
        let evicted = buf.push(MemoryEntry::new("user", MemoryContent::Text("c".into())));
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().content_str(), "a");
        assert_eq!(buf.len(), 2);

        let recent = buf.recent(2);
        assert_eq!(recent[0].content_str(), "c");
        assert_eq!(recent[1].content_str(), "b");
    }

    #[test]
    fn test_stm_recent_less_than_capacity() {
        let buf = StmBuffer::new(5);
        // 空的 buffer 取 recent 应返回空
        assert!(buf.recent(3).is_empty());
    }

    #[test]
    fn test_memory_entry_importance() {
        let entry =
            MemoryEntry::new("assistant", MemoryContent::Text("hi".into())).with_importance(0.75);
        assert!((entry.importance - 0.75).abs() < 1e-6);

        // 应该 clamp 到 [0, 1]
        let entry =
            MemoryEntry::new("assistant", MemoryContent::Text("hi".into())).with_importance(1.5);
        assert!((entry.importance - 1.0).abs() < 1e-6);
    }

    // ── MemoryContent 测试 ──

    #[test]
    fn test_memory_content_text() {
        let content = MemoryContent::Text("hello".into());
        assert_eq!(content.content_str(), "hello");
    }

    #[test]
    fn test_memory_content_image() {
        let content = MemoryContent::Image {
            path: "/img/1.png".into(),
            caption: Some("截图".into()),
        };
        let s = content.content_str();
        assert!(s.contains("截图"));
        assert!(s.contains("1.png"));
    }

    #[test]
    fn test_memory_content_video() {
        let content = MemoryContent::Video {
            path: "/vid/1.mp4".into(),
            caption: None,
        };
        let s = content.content_str();
        assert!(s.contains("1.mp4"));
    }

    #[test]
    fn test_memory_content_file() {
        let content = MemoryContent::File {
            path: "/doc/report.pdf".into(),
            mime: "application/pdf".into(),
            caption: Some("报告".into()),
        };
        let s = content.content_str();
        assert!(s.contains("报告"));
        assert!(s.contains("pdf"));
        assert!(s.contains("report"));
    }

    // ── MemoryManager 测试 ──

    #[test]
    fn test_manager_remember_and_recent() {
        let stm = StmBuffer::new(3);
        let ltm = SledLtm::open_in_memory();
        let mut mgr = MemoryManager::new(stm, ltm);

        mgr.remember(MemoryEntry::new(
            "user",
            MemoryContent::Text("hello".into()),
        ))
        .unwrap();
        mgr.remember(MemoryEntry::new(
            "assistant",
            MemoryContent::Text("world".into()),
        ))
        .unwrap();

        let recent = mgr.recent(3);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].role, "assistant");
        assert_eq!(recent[1].role, "user");
    }

    #[test]
    fn test_manager_overflow_to_ltm() {
        let stm = StmBuffer::new(2);
        let ltm = SledLtm::open_in_memory();
        let mut mgr = MemoryManager::new(stm, ltm);

        mgr.remember(MemoryEntry::new("user", MemoryContent::Text("a".into())))
            .unwrap();
        mgr.remember(MemoryEntry::new("user", MemoryContent::Text("b".into())))
            .unwrap();
        // 第三条应该使 a 溢出到 LTM
        mgr.remember(MemoryEntry::new("user", MemoryContent::Text("c".into())))
            .unwrap();

        // LTM 应该有一条
        let recalled = mgr.recall(1).unwrap().unwrap();
        assert_eq!(recalled.content.content_str(), "a");
    }

    #[test]
    fn test_manager_disable_ltm() {
        let stm = StmBuffer::new(1);
        let ltm = SledLtm::open_in_memory();
        let mut mgr = MemoryManager::new(stm, ltm);
        mgr.set_ltm_enabled(false);

        mgr.remember(MemoryEntry::new("user", MemoryContent::Text("a".into())))
            .unwrap();
        mgr.remember(MemoryEntry::new("user", MemoryContent::Text("b".into())))
            .unwrap();

        // LTM 关闭，溢出不应写入 LTM
        assert!(mgr.recall(1).unwrap().is_none());
    }

    // ── SledLtm 测试 ──

    #[test]
    fn test_sled_ltm_insert_get() {
        let mut ltm = SledLtm::open_in_memory();
        let entry = MemoryEntry::new("user", MemoryContent::Text("persist me".into()));

        let id = ltm.insert(&entry).unwrap();
        assert_eq!(id, 1);

        let loaded = ltm.get(id).unwrap().unwrap();
        assert_eq!(loaded.content_str(), "persist me");
    }

    #[test]
    fn test_sled_ltm_delete() {
        let mut ltm = SledLtm::open_in_memory();
        let id = ltm
            .insert(&MemoryEntry::new("user", MemoryContent::Text("x".into())))
            .unwrap();
        assert!(ltm.get(id).unwrap().is_some());

        ltm.delete(id).unwrap();
        assert!(ltm.get(id).unwrap().is_none());
    }

    #[test]
    fn test_sled_ltm_count_and_scan() {
        let mut ltm = SledLtm::open_in_memory();
        assert_eq!(ltm.count().unwrap(), 0);

        ltm.insert(&MemoryEntry::new("user", MemoryContent::Text("a".into())))
            .unwrap();
        ltm.insert(&MemoryEntry::new(
            "assistant",
            MemoryContent::Text("b".into()),
        ))
        .unwrap();
        assert_eq!(ltm.count().unwrap(), 2);

        let all = ltm.scan().unwrap();
        assert_eq!(all.len(), 2);
        // 检查 scan 按 id 排序
        assert_eq!(all[0].0, 1);
        assert_eq!(all[1].0, 2);
    }

    #[test]
    fn test_sled_ltm_multiple_ids() {
        let mut ltm = SledLtm::open_in_memory();
        for i in 0..10 {
            let id = ltm
                .insert(&MemoryEntry::new(
                    "user",
                    MemoryContent::Text(format!("n{}", i)),
                ))
                .unwrap();
            assert_eq!(id, i + 1);
        }
        assert_eq!(ltm.count().unwrap(), 10);
    }
}
