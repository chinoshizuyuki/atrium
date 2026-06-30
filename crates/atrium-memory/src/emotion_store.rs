// SPDX-License-Identifier: MIT
//! EmotionStore — 情感状态持久化（sled named tree + bincode）
//! EmotionStore — Emotion state persistence (sled named tree + bincode).
//!
//! 保存/加载 EmotionSnapshot 到 sled 命名 Tree，让情感状态在进程重启后得以保留。
//! Saves/loads EmotionSnapshot to a sled named tree, preserving emotional state across restarts.
//!
//! # 重构说明 / Refactoring Note
//!
//! P1-3: 从独立 sled 实例（模式 B）重构为共享 Db + 命名 Tree（模式 A）。
//! P1-3: Refactored from independent sled instance (Pattern B) to shared Db + named tree (Pattern A).
//! 归属认知域：情感中枢（Limbic）。
//! Cognitive domain: Limbic.

use atrium_emotion::EmotionSnapshot;

/// 情感存储 — 数字生命的感受持久化层
/// Emotion store — Persistence layer for digital life's feelings.
///
/// 使用命名 Tree `"emotion_snapshot"` 存储情感快照，
/// 共享情感中枢数据库实例，消除独立 sled 实例的开销。
pub struct EmotionStore {
    /// 情感快照 Tree / Emotion snapshot tree
    tree: sled::Tree,
}

impl EmotionStore {
    /// 从共享数据库打开情感存储 / Open emotion store from shared database.
    ///
    /// 在给定的 sled::Db 上打开命名 Tree `"emotion_snapshot"`。
    /// Opens the named tree `"emotion_snapshot"` on the given sled::Db.
    ///
    /// # 参数 / Parameters
    ///
    /// - `db` — 共享的 sled 数据库引用（通常为情感中枢 Limbic Db）
    pub fn open(db: &sled::Db) -> anyhow::Result<Self> {
        let tree = db.open_tree("emotion_snapshot")?;
        Ok(Self { tree })
    }

    /// 创建内存模式（测试用）/ Create in-memory mode for testing.
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
        Self::open(&db)
    }

    /// 保存情感快照 / Save emotion snapshot.
    ///
    /// 将当前情感状态序列化后写入命名 Tree 的单例键。
    /// Serializes the current emotional state and writes to the named tree's singleton key.
    pub fn save_snapshot(&self, snap: &EmotionSnapshot) -> anyhow::Result<()> {
        let value = bincode::serialize(snap)?;
        self.tree.insert(b"emotion_snapshot", value)?;
        self.tree.flush()?;
        Ok(())
    }

    /// 加载情感快照 / Load emotion snapshot.
    ///
    /// 从命名 Tree 读取并反序列化情感状态。
    /// Reads and deserializes the emotional state from the named tree.
    pub fn load_snapshot(&self) -> anyhow::Result<Option<EmotionSnapshot>> {
        match self.tree.get(b"emotion_snapshot")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_emotion::{EmotionState, InertiaModifiers};

    #[test]
    fn test_save_and_load_snapshot() {
        let store = EmotionStore::open_in_memory().unwrap();

        let snap = EmotionSnapshot {
            current: EmotionState::new(0.5, 0.3, -0.2),
            inertia_history: vec![[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]],
            inertia_dominant_duration: 42,
            inertia_dominant_label: Some("愉悦".to_string()),
            inertia_modifiers: InertiaModifiers {
                sensitivity: 1.2,
                decay_rate: 0.9,
                expression_threshold: -0.05,
            },
            longing_state: None,
        };

        store.save_snapshot(&snap).unwrap();
        let loaded = store.load_snapshot().unwrap().unwrap();

        assert!((loaded.current.pleasure - 0.5).abs() < 1e-6);
        assert!((loaded.current.arousal - 0.3).abs() < 1e-6);
        assert!((loaded.current.dominance - (-0.2)).abs() < 1e-6);
        assert_eq!(loaded.inertia_history.len(), 2);
        assert_eq!(loaded.inertia_dominant_duration, 42);
        assert_eq!(loaded.inertia_dominant_label.as_deref(), Some("愉悦"));
        assert!((loaded.inertia_modifiers.sensitivity - 1.2).abs() < 1e-6);
    }

    #[test]
    fn test_load_empty_returns_none() {
        let store = EmotionStore::open_in_memory().unwrap();
        assert!(store.load_snapshot().unwrap().is_none());
    }
}
