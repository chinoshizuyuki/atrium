// SPDX-License-Identifier: MIT
//! 仪式存储 — RitualDetector + AnniversarySystem + SeasonalAwareness 的 sled 持久化
//! RitualStore — Ritual/Anniversary/Seasonal models persisted via sled.
//!
//! 将仪式检测器、纪念日系统、季节感知系统持久化到 sled，
//! 消除重启失忆缺陷，支持跨会话的仪式连续性与纪念日记忆。

use serde::{Deserialize, Serialize};

use crate::anniversary_system::AnniversarySystem;
use crate::ritual_detector::RitualDetector;
use crate::seasonal_awareness::SeasonalAwareness;

// ════════════════════════════════════════════════════════════════════
// 统一使用 store_core::StoreError / Unified StoreError from store_core

// ════════════════════════════════════════════════════════════════════
// SerializableRitualSnapshot — 可序列化的仪式系统快照
// Serializable ritual systems snapshot for sled bincode persistence
// ════════════════════════════════════════════════════════════════════

/// 可序列化的仪式系统快照 — 用于 sled bincode 持久化
///
/// 包含三个子系统的完整状态，确保 bincode 稳定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableRitualSnapshot {
    /// 仪式检测器 / Ritual detector
    pub ritual_detector: RitualDetector,
    /// 纪念日系统 / Anniversary system
    pub anniversary_system: AnniversarySystem,
    /// 季节感知系统 / Seasonal awareness system
    pub seasonal_awareness: SeasonalAwareness,
}

impl SerializableRitualSnapshot {
    /// 从三个子系统构造 / Construct from three subsystems
    pub fn from_parts(
        detector: &RitualDetector,
        anniversary: &AnniversarySystem,
        seasonal: &SeasonalAwareness,
    ) -> Self {
        Self {
            ritual_detector: detector.clone(),
            anniversary_system: anniversary.clone(),
            seasonal_awareness: seasonal.clone(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// RitualStore — sled 持久化存储 / Sled persistent store
// ════════════════════════════════════════════════════════════════════

/// 仪式存储 — 基于 sled 的仪式系统持久化
///
/// Key: "snapshot" (单例)
/// Value: SerializableRitualSnapshot (bincode serialized)
///
/// 辅助 tree：仪式模式索引 + 纪念日索引
pub struct RitualStore {
    /// 主 tree：存储完整仪式系统快照 / Main tree: full snapshot
    tree: sled::Tree,
    /// 仪式模式索引 tree：ritual_id → pattern bincode / Ritual pattern index tree
    ritual_tree: sled::Tree,
    /// 纪念日索引 tree：anniversary_id → anniversary bincode / Anniversary index tree
    anniversary_tree: sled::Tree,
}

impl RitualStore {
    /// 打开或创建仪式存储 / Open or create ritual store
    pub fn open(db: &sled::Db) -> Result<Self, crate::store_core::StoreError> {
        let tree = db
            .open_tree("ritual_snapshot")
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        let ritual_tree = db
            .open_tree("ritual_patterns")
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        let anniversary_tree = db
            .open_tree("ritual_anniversaries")
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        Ok(Self {
            tree,
            ritual_tree,
            anniversary_tree,
        })
    }

    /// 保存完整的仪式系统快照 / Save full ritual systems snapshot
    pub fn save(
        &self,
        detector: &RitualDetector,
        anniversary: &AnniversarySystem,
        seasonal: &SeasonalAwareness,
    ) -> Result<(), crate::store_core::StoreError> {
        let snapshot = SerializableRitualSnapshot::from_parts(detector, anniversary, seasonal);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
        self.tree
            .insert(b"snapshot", value.as_slice())
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;

        // 同步更新仪式模式索引 / Sync ritual pattern index
        for pattern in &detector.patterns {
            let key = pattern.id.to_be_bytes();
            let val = bincode::serialize(pattern)
                .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
            self.ritual_tree
                .insert(key, val.as_slice())
                .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        }

        // 同步更新纪念日索引 / Sync anniversary index
        for ann in &anniversary.anniversaries {
            let key = ann.id.to_be_bytes();
            let val = bincode::serialize(ann)
                .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
            self.anniversary_tree
                .insert(key, val.as_slice())
                .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        }

        Ok(())
    }

    /// 加载仪式系统快照 / Load ritual systems snapshot
    pub fn load(&self) -> Result<SerializableRitualSnapshot, crate::store_core::StoreError> {
        match self.tree.get(b"snapshot") {
            Ok(Some(value)) => {
                let snapshot: SerializableRitualSnapshot = bincode::deserialize(&value)
                    .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
                Ok(snapshot)
            }
            Ok(None) => Ok(SerializableRitualSnapshot {
                ritual_detector: RitualDetector::default_new(),
                anniversary_system: AnniversarySystem::new(),
                seasonal_awareness: SeasonalAwareness::new(),
            }),
            Err(e) => Err(crate::store_core::StoreError::Sled(e.to_string())),
        }
    }

    /// 获取单个仪式模式 / Get a single ritual pattern by ID
    pub fn get_ritual_pattern(
        &self,
        ritual_id: u64,
    ) -> Result<Option<crate::ritual_detector::RitualPattern>, crate::store_core::StoreError> {
        let key = ritual_id.to_be_bytes();
        match self.ritual_tree.get(key) {
            Ok(Some(value)) => {
                let pattern: crate::ritual_detector::RitualPattern =
                    bincode::deserialize(&value)
                        .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
                Ok(Some(pattern))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::store_core::StoreError::Sled(e.to_string())),
        }
    }

    /// 获取单个纪念日 / Get a single anniversary by ID
    pub fn get_anniversary(
        &self,
        anniversary_id: u64,
    ) -> Result<Option<crate::anniversary_system::Anniversary>, crate::store_core::StoreError> {
        let key = anniversary_id.to_be_bytes();
        match self.anniversary_tree.get(key) {
            Ok(Some(value)) => {
                let ann: crate::anniversary_system::Anniversary = bincode::deserialize(&value)
                    .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
                Ok(Some(ann))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::store_core::StoreError::Sled(e.to_string())),
        }
    }

    /// 获取所有仪式模式 ID / Get all ritual pattern IDs
    pub fn ritual_pattern_ids(&self) -> Result<Vec<u64>, crate::store_core::StoreError> {
        let mut ids = Vec::new();
        for item in self.ritual_tree.iter() {
            let (key, _) = item.map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 获取所有纪念日 ID / Get all anniversary IDs
    pub fn anniversary_ids(&self) -> Result<Vec<u64>, crate::store_core::StoreError> {
        let mut ids = Vec::new();
        for item in self.anniversary_tree.iter() {
            let (key, _) = item.map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 仪式模式总数 / Ritual pattern count
    pub fn ritual_pattern_count(&self) -> usize {
        self.ritual_tree.len()
    }

    /// 纪念日总数 / Anniversary count
    pub fn anniversary_count(&self) -> usize {
        self.anniversary_tree.len()
    }
}

// ════════════════════════════════════════════════════════════════════
// DomainStore + VaultTree trait 实现 / Trait Implementations
// ════════════════════════════════════════════════════════════════════

/// VaultTree 实现 — 主 tree 承载 SerializableRitualSnapshot
/// VaultTree impl — main tree carries SerializableRitualSnapshot
impl crate::atrium_vault::VaultTree<SerializableRitualSnapshot> for RitualStore {
    fn tree(&self) -> &sled::Tree {
        &self.tree
    }
}

/// DomainStore 实现 — 仪式记忆子系统的存储接口
/// DomainStore impl — ritual memory subsystem store interface
impl crate::store_core::DomainStore for RitualStore {
    fn domain_name(&self) -> &'static str {
        "ritual"
    }

    fn tree_count(&self) -> usize {
        self.tree.len() + self.ritual_tree.len() + self.anniversary_tree.len()
    }

    fn flush_tree(&self) -> Result<(), crate::store_core::StoreError> {
        self.tree.flush()?;
        self.ritual_tree.flush()?;
        self.anniversary_tree.flush()?;
        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anniversary_system::AnniversaryKind;

    /// 测试用临时数据库 / Temporary test database
    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn test_store_save_and_load_default() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();
        let detector = RitualDetector::default_new();
        let anniversary = AnniversarySystem::new();
        let seasonal = SeasonalAwareness::new();

        store.save(&detector, &anniversary, &seasonal).unwrap();
        let loaded = store.load().unwrap();

        assert!(loaded.ritual_detector.patterns.is_empty());
        assert!(loaded.anniversary_system.anniversaries.is_empty());
    }

    #[test]
    fn test_store_save_and_load_with_rituals() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();

        let mut detector = RitualDetector::default_new();
        // 模拟交互记录 / Simulate interaction records
        let base = 1781992800i64; // 2026-06-20 22:00 UTC
        detector.record_interaction(base);
        detector.record_interaction(base + 300);

        let anniversary = AnniversarySystem::new();
        let seasonal = SeasonalAwareness::new();

        store.save(&detector, &anniversary, &seasonal).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(
            loaded.ritual_detector.daily_records.len(),
            detector.daily_records.len()
        );
    }

    #[test]
    fn test_store_save_and_load_with_anniversaries() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();

        let detector = RitualDetector::default_new();
        let mut anniversary = AnniversarySystem::new();
        anniversary.set_first_conversation(1000 * 86400);
        anniversary.set_naming_day(1001 * 86400, "Atrium");
        let seasonal = SeasonalAwareness::new();

        store.save(&detector, &anniversary, &seasonal).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.anniversary_system.anniversaries.len(), 2);
        assert_eq!(
            loaded.anniversary_system.anniversaries[0].kind,
            AnniversaryKind::FirstConversation
        );
    }

    #[test]
    fn test_store_preserves_seasonal_custom_holidays() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();

        let detector = RitualDetector::default_new();
        let anniversary = AnniversarySystem::new();
        let mut seasonal = SeasonalAwareness::new();
        // 添加自定义节日 / Add custom holiday
        seasonal.add_holiday(crate::seasonal_awareness::Holiday {
            name: "自定义日".to_string(),
            month: 7,
            day: 15,
            is_lunar: false,
            greeting: "快乐！".to_string(),
        });

        let custom_count = seasonal.holidays.len();
        store.save(&detector, &anniversary, &seasonal).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.seasonal_awareness.holidays.len(), custom_count);
    }

    #[test]
    fn test_store_ritual_pattern_index() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();

        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 2;
        // 建立仪式 / Establish a ritual
        let base = 1781992800i64;
        for day in 0..2 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.evaluate_daily(day_epoch + 86400);
        }

        let anniversary = AnniversarySystem::new();
        let seasonal = SeasonalAwareness::new();

        store.save(&detector, &anniversary, &seasonal).unwrap();

        if !detector.patterns.is_empty() {
            assert!(store.ritual_pattern_count() > 0);
            let first = &detector.patterns[0];
            let loaded = store.get_ritual_pattern(first.id).unwrap().unwrap();
            assert_eq!(loaded.id, first.id);
        }
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = RitualStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.ritual_detector.patterns.is_empty());
        assert!(loaded.anniversary_system.anniversaries.is_empty());
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut detector = RitualDetector::default_new();
        detector.record_interaction(1781992800);

        let mut anniversary = AnniversarySystem::new();
        anniversary.set_first_conversation(1000 * 86400);

        let seasonal = SeasonalAwareness::new();

        let snapshot = SerializableRitualSnapshot::from_parts(&detector, &anniversary, &seasonal);

        // bincode 往返 / bincode roundtrip
        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: SerializableRitualSnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(
            restored.ritual_detector.patterns.len(),
            detector.patterns.len()
        );
        assert_eq!(
            restored.anniversary_system.anniversaries.len(),
            anniversary.anniversaries.len()
        );
    }
}
