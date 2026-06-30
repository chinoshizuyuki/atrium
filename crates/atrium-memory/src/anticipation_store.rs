// SPDX-License-Identifier: MIT
//! 期待事件存储 — sled 命名 Tree 持久化的 AnticipationEvent 管理
//! Anticipation event store — sled named tree-backed persistence for anticipation events.
//!
//! 当用户说"明天再来"等话语时，生成期待事件并持久化。
//! 接近预期时间时触发情感预加载，过期未归时触发失落感。
//!
//! # 重构说明 / Refactoring Note
//!
//! P1-3: 从独立 sled 实例（模式 B）重构为共享 Db + 命名 Tree（模式 A）。
//! P1-3: Refactored from independent sled instance (Pattern B) to shared Db + named tree (Pattern A).
//! 原先在默认 Tree 中用前缀键 `event/` 和 `pending/` 区分两类数据，
//! 现拆分为两个独立命名 Tree，消除前缀扫描开销。
//! Originally used prefix keys `event/` and `pending/` in a default tree;
//! now split into two independent named trees, eliminating prefix scan overhead.
//! 归属认知域：关系海马体（Relational）。
//! Cognitive domain: Relational.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// AnticipationEvent — 期待事件 / Anticipation Event
// ════════════════════════════════════════════════════════════════════

/// 期待事件 — 用户承诺未来某个时间回来时生成
/// Anticipation event — generated when the user promises to return at a future time.
///
/// 接近 expected_at 时，PAD 中 pleasure 缓慢上升（期待感）。
/// 用户按时回来时叠加 ReunionBurst；未按时回来则触发失落。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnticipationEvent {
    /// 唯一标识（时间戳 + 描述哈希）/ Unique ID (timestamp + description hash)
    pub id: String,
    /// 事件描述 / Event description (e.g. "明天再来")
    pub description: String,
    /// 预期时间戳（Unix 秒）/ Expected timestamp (Unix seconds)
    pub expected_at: i64,
    /// 创建时间戳 / Created timestamp
    pub created_at: i64,
    /// 是否已触发 / Whether triggered
    pub triggered: bool,
    /// 情感预加载 PAD 偏移 / Emotional pre-load PAD offset
    pub anticipation_pad: [f32; 3],
}

// ════════════════════════════════════════════════════════════════════
// AnticipationStore — 双 Tree 持久化 / Dual-tree Store
// ════════════════════════════════════════════════════════════════════

/// 期待事件存储 — sled 双命名 Tree + bincode 序列化
/// Anticipation event store — sled dual named tree + bincode serialization.
///
/// # Tree 结构 / Tree Structure
///
/// | Tree 名 | 键 | 值 | 用途 |
/// |---------|-----|-----|------|
/// | `anticipation_events` | `event/{id}` | AnticipationEvent | 事件主存储 |
/// | `anticipation_pending` | `pending/{expected_at:020}` | id (String) | 按时间排序的待触发索引 |
///
/// 键格式保持前缀风格以确保迁移兼容性。
/// Key format retains prefix style for migration compatibility.
pub struct AnticipationStore {
    /// 事件主存储 Tree / Event primary storage tree
    event_tree: sled::Tree,
    /// 待触发索引 Tree / Pending trigger index tree
    pending_tree: sled::Tree,
}

impl AnticipationStore {
    /// 从共享数据库打开期待事件存储 / Open anticipation store from shared database.
    ///
    /// 在给定的 sled::Db 上打开两个命名 Tree：
    /// - `"anticipation_events"` — 事件主存储
    /// - `"anticipation_pending"` — 待触发索引
    ///
    /// Opens two named trees on the given sled::Db:
    /// - `"anticipation_events"` — event primary storage
    /// - `"anticipation_pending"` — pending trigger index
    ///
    /// # 参数 / Parameters
    ///
    /// - `db` — 共享的 sled 数据库引用（通常为关系海马体 Relational Db）
    pub fn open(db: &sled::Db) -> Result<Self, sled::Error> {
        let event_tree = db.open_tree("anticipation_events")?;
        let pending_tree = db.open_tree("anticipation_pending")?;
        Ok(Self {
            event_tree,
            pending_tree,
        })
    }

    /// 创建内存模式（测试用）/ Create in-memory mode for testing.
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("anticipation_store: temporary db init failed");
        Self::open(&db).expect("anticipation_store: open trees failed")
    }

    /// 添加期待事件 / Add an anticipation event.
    pub fn add(&self, event: AnticipationEvent) -> Result<(), sled::Error> {
        let key = format!("event/{}", event.id);
        let pending_key = format!("pending/{:020}", event.expected_at);
        let value = bincode::serialize(&event).unwrap_or_default();
        self.event_tree.insert(key.as_bytes(), value)?;
        self.pending_tree
            .insert(pending_key.as_bytes(), event.id.as_bytes())?;
        self.event_tree.flush()?;
        self.pending_tree.flush()?;
        Ok(())
    }

    /// 获取所有未触发的期待事件 / Get all pending (untriggered) events.
    pub fn pending(&self) -> Result<Vec<AnticipationEvent>, sled::Error> {
        let mut events = Vec::new();
        for item in self.event_tree.iter() {
            let (_k, v) = item?;
            if let Ok(event) = bincode::deserialize::<AnticipationEvent>(&v) {
                if !event.triggered {
                    events.push(event);
                }
            }
        }
        events.sort_by_key(|e| e.expected_at);
        Ok(events)
    }

    /// 标记事件已触发 / Mark an event as triggered.
    pub fn mark_triggered(&self, id: &str) -> Result<(), sled::Error> {
        let key = format!("event/{}", id);
        if let Some(val) = self.event_tree.get(key.as_bytes())? {
            if let Ok(mut event) = bincode::deserialize::<AnticipationEvent>(&val) {
                event.triggered = true;
                let updated = bincode::serialize(&event).unwrap_or_default();
                self.event_tree.insert(key.as_bytes(), updated)?;
                // 从 pending 索引中删除 / Remove from pending index
                let pending_key = format!("pending/{:020}", event.expected_at);
                self.pending_tree.remove(pending_key.as_bytes())?;
                self.event_tree.flush()?;
                self.pending_tree.flush()?;
            }
        }
        Ok(())
    }

    /// 获取已过期但未触发的事件 / Get expired but untriggered events.
    ///
    /// @param now 当前 Unix 时间戳 / Current Unix timestamp
    /// @return 过期事件列表 / List of expired events
    pub fn expired(&self, now: i64) -> Result<Vec<AnticipationEvent>, sled::Error> {
        let mut events = Vec::new();
        for item in self.event_tree.iter() {
            let (_k, v) = item?;
            if let Ok(event) = bincode::deserialize::<AnticipationEvent>(&v) {
                if !event.triggered && event.expected_at < now {
                    events.push(event);
                }
            }
        }
        events.sort_by_key(|e| e.expected_at);
        Ok(events)
    }

    /// 清理已触发的事件（定期调用）/ Clean up triggered events.
    pub fn cleanup_triggered(&self) -> Result<usize, sled::Error> {
        let mut removed = 0;
        let keys_to_remove: Vec<Vec<u8>> = self
            .event_tree
            .iter()
            .filter_map(|item| {
                item.ok().and_then(|(k, v)| {
                    bincode::deserialize::<AnticipationEvent>(&v)
                        .ok()
                        .filter(|e| e.triggered)
                        .map(|_| k.to_vec())
                })
            })
            .collect();
        for key in keys_to_remove {
            self.event_tree.remove(&key)?;
            removed += 1;
        }
        if removed > 0 {
            self.event_tree.flush()?;
        }
        Ok(removed)
    }
}

// ════════════════════════════════════════════════════════════════════
// AnticipationDetector — 期待事件检测器 / Anticipation Detector
// ════════════════════════════════════════════════════════════════════

/// 期待事件检测器 — 纯规则关键词匹配，<1μs
/// Anticipation event detector — pure rule-based keyword matching, <1μs.
pub struct AnticipationDetector;

/// 检测结果 / Detection result
#[derive(Clone, Debug)]
pub struct DetectedAnticipation {
    pub description: String,
    pub expected_at: i64,
    pub anticipation_pad: [f32; 3],
}

impl AnticipationDetector {
    /// 从消息文本检测期待事件 / Detect anticipation from message text.
    ///
    /// @param msg 用户消息文本 / User message text
    /// @param now 当前 Unix 时间戳 / Current Unix timestamp
    /// @return 检测到的期待事件（如果有）/ Detected anticipation if any
    pub fn detect(msg: &str, now: i64) -> Option<DetectedAnticipation> {
        // 明天/后天 + 见/来/聊 / Tomorrow or day after + see/come/chat
        if (msg.contains("明天") || msg.contains("后天")) && Self::has_meeting_keyword(msg) {
            let days = if msg.contains("后天") { 2 } else { 1 };
            let expected = now + days * 86400;
            return Some(DetectedAnticipation {
                description: if days == 1 {
                    "明天见".into()
                } else {
                    "后天见".into()
                },
                expected_at: expected,
                anticipation_pad: [0.05, 0.02, 0.0],
            });
        }

        // 稍后/一会/马上回来 / Later / in a bit / right back
        if msg.contains("等一下") || msg.contains("一会") || msg.contains("马上回来") {
            let expected = now + 1800; // +30min
            return Some(DetectedAnticipation {
                description: "稍后再来".into(),
                expected_at: expected,
                anticipation_pad: [0.03, 0.01, 0.0],
            });
        }

        // 晚上/今晚 + 见/聊 / Evening + see/chat
        if (msg.contains("晚上") || msg.contains("今晚")) && Self::has_meeting_keyword(msg) {
            // 计算今日 18:00 的时间戳 / Compute today's 18:00 timestamp
            let today_18 = now - (now % 86400) + 18 * 3600;
            let expected = if today_18 > now {
                today_18
            } else {
                today_18 + 86400
            };
            return Some(DetectedAnticipation {
                description: "晚上见".into(),
                expected_at: expected,
                anticipation_pad: [0.04, 0.02, 0.0],
            });
        }

        None
    }

    /// 检查是否包含见面关键词 / Check if message contains meeting keywords.
    fn has_meeting_keyword(msg: &str) -> bool {
        msg.contains("见") || msg.contains("来") || msg.contains("聊") || msg.contains("找")
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_tomorrow() {
        let now = 1_700_000_000i64;
        let result = AnticipationDetector::detect("明天再来", now);
        assert!(result.is_some(), "应检测到明天再来的期待");
        let det = result.unwrap();
        assert_eq!(det.expected_at, now + 86400);
        assert!(det.description.contains("明天"));
    }

    #[test]
    fn test_detect_day_after() {
        let now = 1_700_000_000i64;
        let result = AnticipationDetector::detect("后天见", now);
        assert!(result.is_some());
        assert_eq!(result.unwrap().expected_at, now + 2 * 86400);
    }

    #[test]
    fn test_detect_later() {
        let now = 1_700_000_000i64;
        let result = AnticipationDetector::detect("等一下我马上回来", now);
        assert!(result.is_some());
        assert_eq!(result.unwrap().expected_at, now + 1800);
    }

    #[test]
    fn test_detect_no_match() {
        let result = AnticipationDetector::detect("今天天气真好", 1_700_000_000);
        assert!(result.is_none(), "无关消息不应检测到期待");
    }

    #[test]
    fn test_store_add_and_pending() {
        let store = AnticipationStore::open_in_memory();
        let event = AnticipationEvent {
            id: "test1".into(),
            description: "明天见".into(),
            expected_at: 1_700_086_400,
            created_at: 1_700_000_000,
            triggered: false,
            anticipation_pad: [0.05, 0.02, 0.0],
        };
        store.add(event).unwrap();
        let pending = store.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "test1");
    }

    #[test]
    fn test_store_mark_triggered() {
        let store = AnticipationStore::open_in_memory();
        let event = AnticipationEvent {
            id: "test2".into(),
            description: "晚上见".into(),
            expected_at: 1_700_064_800,
            created_at: 1_700_000_000,
            triggered: false,
            anticipation_pad: [0.04, 0.02, 0.0],
        };
        store.add(event).unwrap();
        store.mark_triggered("test2").unwrap();
        let pending = store.pending().unwrap();
        assert_eq!(pending.len(), 0, "触发后不应在 pending 列表中");
    }

    #[test]
    fn test_store_expired() {
        let store = AnticipationStore::open_in_memory();
        let event = AnticipationEvent {
            id: "test3".into(),
            description: "稍后再来".into(),
            expected_at: 1_700_000_100,
            created_at: 1_700_000_000,
            triggered: false,
            anticipation_pad: [0.03, 0.01, 0.0],
        };
        store.add(event).unwrap();
        let expired = store.expired(1_700_000_200).unwrap();
        assert_eq!(expired.len(), 1, "应检测到过期事件");
    }

    #[test]
    fn test_store_cleanup() {
        let store = AnticipationStore::open_in_memory();
        let event = AnticipationEvent {
            id: "test4".into(),
            description: "明天见".into(),
            expected_at: 1_700_086_400,
            created_at: 1_700_000_000,
            triggered: true,
            anticipation_pad: [0.05, 0.02, 0.0],
        };
        store.add(event).unwrap();
        let removed = store.cleanup_triggered().unwrap();
        assert_eq!(removed, 1, "应清理 1 条已触发事件");
        let pending = store.pending().unwrap();
        assert_eq!(pending.len(), 0);
    }
}
