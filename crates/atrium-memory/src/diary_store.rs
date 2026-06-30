// SPDX-License-Identifier: MIT
//! 数字日记 — AI 独处时在深夜自动书写的私人记录
//!
//! Digital diary — Private entries the AI writes to itself during late-night idle periods.
//!
//! DiaryStore 使用 sled 命名 Tree 持久化，按日期（YYYY-MM-DD）索引。
//! 每天最多一条日记，内容通过 LLM 生成，包含当日事件摘要、情感曲线和思考计数。
//! 日记默认不直接分享给用户，但可通过 enhanced_search 被检索到并自然融入回复。
//!
//! DiaryStore uses a sled named tree for persistence, indexed by date string (YYYY-MM-DD).
//! At most one entry per day; content is LLM-generated, including a summary of the day's
//! events, the emotional trajectory, and a thought count. Diary entries are not shared
//! directly with the user but can be retrieved via enhanced_search and woven into replies.
//!
//! # 重构说明 / Refactoring Note
//!
//! P1-3: 从独立 sled 实例（模式 B）重构为共享 Db + 命名 Tree（模式 A）。
//! P1-3: Refactored from independent sled instance (Pattern B) to shared Db + named tree (Pattern A).
//! 归属认知域：叙事皮层（Narrative）。
//! Cognitive domain: Narrative.

use serde::{Deserialize, Serialize};

/// 数字日记存储 — sled 命名 Tree 持久化层
/// Digital diary storage — sled named tree-backed persistence layer.
///
/// 使用命名 Tree `"diary_entries"` 存储日记条目，
/// 共享叙事皮层数据库实例，消除独立 sled 实例的开销。
pub struct DiaryStore {
    /// 日记条目 Tree / Diary entries tree
    tree: sled::Tree,
}

/// 日记条目 — 一天的完整记录
/// Diary entry — A complete record for a single day.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiaryEntry {
    /// 日期（YYYY-MM-DD）/ Date string in YYYY-MM-DD format.
    pub date: String,
    /// 日记正文 / Diary body text.
    pub content: String,
    /// 当日情感曲线摘要 / Emotion summary for the day.
    pub emotion_summary: EmotionSummary,
    /// 关键事件列表（来自 FactStore）/ Key events sourced from FactStore.
    pub key_events: Vec<String>,
    /// 当日生成的思考数 / Thoughts generated that day.
    pub thought_count: u32,
    /// 创建时间戳 / Creation timestamp (epoch seconds).
    pub created_at: i64,
}

/// 情感摘要 — 一天的 PAD 均值与极值标签
/// Emotion summary — Daily PAD averages plus peak/low emotion labels.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EmotionSummary {
    /// 平均愉悦度 / Average pleasure.
    pub avg_pleasure: f32,
    /// 平均唤醒度 / Average arousal.
    pub avg_arousal: f32,
    /// 平均掌控感 / Average dominance.
    pub avg_dominance: f32,
    /// 最强烈情绪标签 / Peak emotion label.
    pub peak_emotion: Option<String>,
    /// 最低落情绪标签 / Lowest emotion label.
    pub lowest_emotion: Option<String>,
}

impl DiaryStore {
    /// 从共享数据库打开日记存储 / Open diary store from shared database.
    ///
    /// 在给定的 sled::Db 上打开命名 Tree `"diary_entries"`。
    /// Opens the named tree `"diary_entries"` on the given sled::Db.
    ///
    /// # 参数 / Parameters
    ///
    /// - `db` — 共享的 sled 数据库引用（通常为叙事皮层 Narrative Db）
    pub fn open(db: &sled::Db) -> Result<Self, sled::Error> {
        let tree = db.open_tree("diary_entries")?;
        Ok(Self { tree })
    }

    /// 使用内存数据库（测试用）/ Create an in-memory store for testing.
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("diary_store: temporary db init failed");
        Self::open(&db).expect("diary_store: open tree failed")
    }

    /// 写入日记条目（按日期去重，覆盖已有条目）
    /// Write a diary entry, keyed by date; overwrites if one already exists.
    ///
    /// @param entry 日记条目 / Diary entry to persist
    pub fn write_entry(&self, entry: &DiaryEntry) -> Result<(), sled::Error> {
        let key = entry.date.as_bytes();
        let val = bincode::serialize(entry).expect("diary serialize");
        self.tree.insert(key, val)?;
        self.tree.flush()?;
        Ok(())
    }

    /// 按日期读取日记
    /// Read a diary entry by date string.
    ///
    /// @param date 日期字符串（YYYY-MM-DD）/ Date string in YYYY-MM-DD format
    /// @return 对应日记条目，不存在则 None / The entry if it exists, None otherwise
    pub fn read_entry(&self, date: &str) -> Result<Option<DiaryEntry>, sled::Error> {
        match self.tree.get(date.as_bytes())? {
            Some(ivec) => {
                let entry: DiaryEntry = bincode::deserialize(&ivec).expect("diary deserialize");
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// 获取最近 N 条日记（按日期降序）
    /// Retrieve the most recent N entries, ordered newest-first.
    ///
    /// @param n 最大返回数 / Maximum number of entries to return
    /// @return 日记列表 / List of diary entries
    pub fn recent_entries(&self, n: usize) -> Result<Vec<DiaryEntry>, sled::Error> {
        let mut entries: Vec<DiaryEntry> = self
            .tree
            .iter()
            .filter_map(|item| {
                let (_, val) = item.ok()?;
                bincode::deserialize::<DiaryEntry>(&val).ok()
            })
            .collect();
        // 按日期降序排列 / Sort by date descending
        entries.sort_by(|a, b| b.date.cmp(&a.date));
        entries.truncate(n);
        Ok(entries)
    }

    /// 检查今天是否已有日记
    /// Check whether a diary entry already exists for today.
    pub fn has_entry_for_today(&self) -> bool {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.tree.contains_key(today.as_bytes()).unwrap_or(false)
    }

    /// 获取日记总数
    /// Total number of diary entries stored.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// 检查是否为空
    /// Check if diary store is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(date: &str, content: &str) -> DiaryEntry {
        DiaryEntry {
            date: date.to_string(),
            content: content.to_string(),
            emotion_summary: EmotionSummary {
                avg_pleasure: 0.5,
                avg_arousal: 0.3,
                avg_dominance: 0.1,
                peak_emotion: Some("happy".into()),
                lowest_emotion: None,
            },
            key_events: vec!["event_a".into()],
            thought_count: 5,
            created_at: 1_700_000_000,
        }
    }

    #[test]
    fn test_write_and_read_entry() {
        let store = DiaryStore::open_in_memory();
        let entry = make_entry("2026-06-24", "今天和主人聊了很多开心的事。");
        store.write_entry(&entry).unwrap();

        let loaded = store.read_entry("2026-06-24").unwrap().unwrap();
        assert_eq!(loaded.content, "今天和主人聊了很多开心的事。");
        assert_eq!(loaded.thought_count, 5);
        assert!(loaded.emotion_summary.peak_emotion.is_some());
    }

    #[test]
    fn test_read_nonexistent_entry() {
        let store = DiaryStore::open_in_memory();
        assert!(store.read_entry("1999-01-01").unwrap().is_none());
    }

    #[test]
    fn test_overwrite_same_date() {
        let store = DiaryStore::open_in_memory();
        store
            .write_entry(&make_entry("2026-06-24", "第一版"))
            .unwrap();
        store
            .write_entry(&make_entry("2026-06-24", "第二版"))
            .unwrap();

        let loaded = store.read_entry("2026-06-24").unwrap().unwrap();
        assert_eq!(loaded.content, "第二版");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_recent_entries_ordering() {
        let store = DiaryStore::open_in_memory();
        store
            .write_entry(&make_entry("2026-06-22", "前天"))
            .unwrap();
        store
            .write_entry(&make_entry("2026-06-24", "今天"))
            .unwrap();
        store
            .write_entry(&make_entry("2026-06-23", "昨天"))
            .unwrap();

        let recent = store.recent_entries(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].date, "2026-06-24");
        assert_eq!(recent[1].date, "2026-06-23");
    }

    #[test]
    fn test_has_entry_for_today() {
        let store = DiaryStore::open_in_memory();
        assert!(!store.has_entry_for_today());

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        store
            .write_entry(&make_entry(&today, "今天的日记"))
            .unwrap();
        assert!(store.has_entry_for_today());
    }

    #[test]
    fn test_len_after_multiple_writes() {
        let store = DiaryStore::open_in_memory();
        store.write_entry(&make_entry("2026-06-20", "a")).unwrap();
        store.write_entry(&make_entry("2026-06-21", "b")).unwrap();
        store.write_entry(&make_entry("2026-06-22", "c")).unwrap();
        assert_eq!(store.len(), 3);
    }
}
