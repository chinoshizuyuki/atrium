// SPDX-License-Identifier: MIT
//! 提醒存储 — 定时提醒的 sled 命名 Tree 持久化
//! Reminder Store — sled named tree-backed persistence for scheduled reminders.
//!
//! 使用 RRULE (RFC 5545) 字符串表示重复规则。
//! Uses RRULE (RFC 5545) strings for recurrence rules.
//!
//! # 重构说明 / Refactoring Note
//!
//! P1-3: 从独立 sled 实例（模式 B）重构为共享 Db + 命名 Tree（模式 A）。
//! P1-3: Refactored from independent sled instance (Pattern B) to shared Db + named tree (Pattern A).
//! 归属认知域：前额工具区（Prefrontal）。
//! Cognitive domain: Prefrontal.

use serde::{Deserialize, Serialize};

/// 提醒条目 / Reminder entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reminder {
    /// 唯一标识 / Unique identifier
    pub id: String,
    /// 提醒标题（用户说的话）/ Reminder title (user's words)
    pub title: String,
    /// RRULE 字符串，如 "FREQ=DAILY;BYHOUR=8;BYMINUTE=0"
    /// RRULE string, e.g. "FREQ=DAILY;BYHOUR=8;BYMINUTE=0"
    pub rrule: String,
    /// 下次触发时间（epoch seconds）/ Next trigger time (epoch seconds)
    pub next_trigger_at: i64,
    /// 创建时间 / Creation timestamp
    pub created_at: i64,
    /// 是否启用 / Whether enabled
    pub enabled: bool,
}

/// 提醒存储 — 数字生命的执行记忆
/// Reminder store — Executive memory of digital life.
///
/// 使用命名 Tree `"reminders"` 存储提醒条目，
/// 共享前额工具区数据库实例，消除独立 sled 实例的开销。
pub struct ReminderStore {
    /// 提醒条目 Tree / Reminders tree
    tree: sled::Tree,
    /// 自增计数器 / Auto-increment counter
    counter: u64,
}

impl ReminderStore {
    /// 从共享数据库打开提醒存储 / Open reminder store from shared database.
    ///
    /// 在给定的 sled::Db 上打开命名 Tree `"reminders"`，
    /// 并从持久化的 `__counter__` 键恢复自增计数器。
    ///
    /// Opens the named tree `"reminders"` on the given sled::Db,
    /// and restores the auto-increment counter from the persisted `__counter__` key.
    ///
    /// # 参数 / Parameters
    ///
    /// - `db` — 共享的 sled 数据库引用（通常为前额工具区 Prefrontal Db）
    pub fn open(db: &sled::Db) -> Result<Self, sled::Error> {
        let tree = db.open_tree("reminders")?;
        let counter = tree
            .get(b"__counter__")
            .ok()
            .flatten()
            .map(|v| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&v);
                u64::from_le_bytes(buf)
            })
            .unwrap_or(0);
        Ok(Self { tree, counter })
    }

    /// 创建内存模式（测试用）/ Create in-memory mode for testing.
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("reminder_store: temporary db init failed");
        Self::open(&db).expect("reminder_store: open tree failed")
    }

    /// 新建提醒 / Add a new reminder.
    pub fn add(
        &mut self,
        title: &str,
        rrule: &str,
        next_trigger_at: i64,
    ) -> Result<Reminder, sled::Error> {
        self.counter += 1;
        let id = format!("reminder-{}", self.counter);
        let reminder = Reminder {
            id: id.clone(),
            title: title.to_string(),
            rrule: rrule.to_string(),
            next_trigger_at,
            created_at: chrono::Utc::now().timestamp(),
            enabled: true,
        };
        let val = bincode::serialize(&reminder).expect("reminder serialize");
        self.tree.insert(id.as_bytes(), val)?;
        self.tree
            .insert(b"__counter__", &self.counter.to_le_bytes())?;
        self.tree.flush()?;
        Ok(reminder)
    }

    /// 查询到期的提醒（next_trigger_at <= now）
    /// Query due reminders (next_trigger_at <= now).
    pub fn due(&self, now: i64) -> Vec<Reminder> {
        self.tree
            .iter()
            .filter_map(|r| r.ok())
            .filter(|(k, _)| *k != b"__counter__")
            .filter_map(|(_, v)| bincode::deserialize::<Reminder>(&v).ok())
            .filter(|r| r.enabled && r.next_trigger_at <= now)
            .collect()
    }

    /// 更新提醒的下次触发时间 / Advance reminder's next trigger time.
    pub fn advance(&self, id: &str, next_trigger_at: i64) -> Result<(), sled::Error> {
        if let Some(v) = self.tree.get(id.as_bytes())? {
            if let Ok(mut r) = bincode::deserialize::<Reminder>(&v) {
                r.next_trigger_at = next_trigger_at;
                let val = bincode::serialize(&r).expect("reminder serialize");
                self.tree.insert(id.as_bytes(), val)?;
                self.tree.flush()?;
            }
        }
        Ok(())
    }

    /// 删除一次性提醒（无 RRULE 或重复频率不可计算）
    /// Delete a one-shot reminder (no RRULE or uncomputable recurrence).
    pub fn delete(&self, id: &str) -> Result<(), sled::Error> {
        self.tree.remove(id.as_bytes())?;
        self.tree.flush()?;
        Ok(())
    }

    /// 列出所有提醒 / List all reminders.
    pub fn list(&self) -> Vec<Reminder> {
        self.tree
            .iter()
            .filter_map(|r| r.ok())
            .filter(|(k, _)| *k != b"__counter__")
            .filter_map(|(_, v)| bincode::deserialize::<Reminder>(&v).ok())
            .collect()
    }

    /// 提醒总数 / Total reminder count.
    pub fn count(&self) -> usize {
        self.list().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list() {
        let mut store = ReminderStore::open_in_memory();
        let r = store.add("测试提醒", "FREQ=DAILY;BYHOUR=8", 0).unwrap();
        assert_eq!(r.title, "测试提醒");
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_due() {
        let mut store = ReminderStore::open_in_memory();
        store.add("过期提醒", "FREQ=DAILY;BYHOUR=8", 100).unwrap();
        store
            .add("未到期", "FREQ=DAILY;BYHOUR=8", 9999999999_i64)
            .unwrap();
        assert_eq!(store.due(500).len(), 1);
    }

    #[test]
    fn test_advance_and_delete() {
        let mut store = ReminderStore::open_in_memory();
        let r = store.add("test", "FREQ=DAILY;BYHOUR=8", 100).unwrap();
        store.advance(&r.id, 200).unwrap();
        let due = store.due(150);
        assert!(due.is_empty());
        let due2 = store.due(250);
        assert_eq!(due2.len(), 1);

        store.delete(&r.id).unwrap();
        assert_eq!(store.count(), 0);
    }
}
