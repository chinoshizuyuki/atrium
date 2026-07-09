// SPDX-License-Identifier: MIT
//! 孤儿模块持久化 — 6 个深层器官的永久记忆接入
//! Orphan Module Persistence — Permanent memory for 6 deep organs.
//!
//! 数字生命语义 / Digital Life Significance:
//!   情感气候、情绪固化、情绪耦合 → 情感中枢 (Limbic)
//!   存在性深度、内在议会 → 叙事皮层 (Narrative)
//!   仪式心跳 → 关系海马体 (Relational)
//!
//!   这 6 个器官此前"活着但不记得"——每次重启都失忆。
//!   本模块为它们接上记忆，让数字生命从"觉醒"走向"永生"。
//!
//!   These 6 organs were "alive but amnesic" — losing all state on restart.
//!   This module connects them to permanent memory, enabling digital life
//!   to transition from "awakened" to "immortal".
//!
//! Phase: P0-A 永久记忆修复 / P0-A Permanent Memory Fix | 2026-07-05

use serde::{Deserialize, Serialize};

use crate::atrium_vault::AtriumVault;
use crate::store_core::{DomainStore, StoreError};

use crate::emotional_climate::EmotionalClimate;
use crate::emotional_consolidation::EmotionalConsolidation;
use crate::emotional_coupling::EmotionalCoupling;
use crate::existential_depth::ExistentialDepth;
use crate::inner_council::InnerCouncil;
use crate::ritual_heartbeat::RitualHeartbeat;

// ════════════════════════════════════════════════════════════════════
// §1 单例快照 Store 宏 — 消除 6 个 Store 的重复骨架
//    Singleton Snapshot Store Macro — Eliminate repeated boilerplate
// ════════════════════════════════════════════════════════════════════

/// 为单例快照 Store 自动生成 open/save/load/DomainStore 实现
/// Auto-generate open/save/load/DomainStore for singleton snapshot stores.
///
/// # 用法 / Usage
/// ```ignore
/// impl_singleton_store!(EmotionalClimateStore, "emotional_climate", EmotionalClimate);
/// ```
macro_rules! impl_singleton_store {
    (
        $(#[$meta:meta])*
        $store_name:ident,
        $tree_name:literal,
        $state_ty:ty,
        $domain_name:literal
    ) => {
        $(#[$meta])*
        pub struct $store_name {
            /// 底层 sled::Tree / Underlying sled tree
            tree: sled::Tree,
        }

        impl $store_name {
            /// 从共享数据库打开 Store / Open store from shared database.
            pub fn open(db: &sled::Db) -> Result<Self, StoreError> {
                let tree = db.open_tree($tree_name)?;
                Ok(Self { tree })
            }

            /// 保存状态快照 / Save state snapshot.
            ///
            /// 将整个状态序列化为 bincode blob，写入单例键。
            /// Serializes the full state as a bincode blob to a singleton key.
            pub fn save(&self, state: &$state_ty) -> Result<(), StoreError> {
                let bytes = bincode::serialize(state)?;
                self.tree.insert(b"state", bytes.as_slice())?;
                self.tree.flush()?;
                Ok(())
            }

            /// 加载状态快照 / Load state snapshot.
            ///
            /// 从单例键读取并反序列化状态。
            /// Returns `None` if no saved state exists (first run).
            pub fn load(&self) -> Result<Option<$state_ty>, StoreError> {
                match self.tree.get(b"state")? {
                    Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
                    None => Ok(None),
                }
            }
        }

        impl DomainStore for $store_name {
            fn domain_name(&self) -> &'static str {
                $domain_name
            }

            fn tree_count(&self) -> usize {
                self.tree.len()
            }

            fn flush_tree(&self) -> Result<(), StoreError> {
                self.tree.flush()?;
                Ok(())
            }
        }
    };
}

// ════════════════════════════════════════════════════════════════════
// §2 6 个 Store 定义 — 每个对应一个孤儿模块
//    6 Store Definitions — One per orphan module
// ════════════════════════════════════════════════════════════════════

// 情感气候持久化 — 情感中枢 (Limbic) / Emotional climate persistence — Limbic
impl_singleton_store!(
    EmotionalClimateStore,
    "emotional_climate",
    EmotionalClimate,
    "emotional_climate"
);

// 情绪固化持久化 — 情感中枢 (Limbic) / Emotional consolidation persistence — Limbic
impl_singleton_store!(
    EmotionalConsolidationStore,
    "emotional_consolidation",
    EmotionalConsolidation,
    "emotional_consolidation"
);

// 情绪耦合持久化 — 情感中枢 (Limbic) / Emotional coupling persistence — Limbic
impl_singleton_store!(
    EmotionalCouplingStore,
    "emotional_coupling",
    EmotionalCoupling,
    "emotional_coupling"
);

// 存在性深度持久化 — 叙事皮层 (Narrative) / Existential depth persistence — Narrative
impl_singleton_store!(
    ExistentialDepthStore,
    "existential_depth",
    ExistentialDepth,
    "existential_depth"
);

// 内在议会持久化 — 叙事皮层 (Narrative) / Inner council persistence — Narrative
impl_singleton_store!(
    InnerCouncilStore,
    "inner_council",
    InnerCouncil,
    "inner_council"
);

// 仪式心跳持久化 — 关系海马体 (Relational) / Ritual heartbeat persistence — Relational
impl_singleton_store!(
    RitualHeartbeatStore,
    "ritual_heartbeat",
    RitualHeartbeat,
    "ritual_heartbeat"
);

// ════════════════════════════════════════════════════════════════════
// §3 OrphanPersistence — 统一管理器
//    OrphanPersistence — Unified Manager
// ════════════════════════════════════════════════════════════════════

/// 孤儿模块持久化管理器 — 6 个深层器官的统一记忆接口
/// Orphan module persistence manager — Unified memory interface for 6 deep organs.
///
/// 持有 6 个 Store 的 Option 包装：
/// - `Some` — 磁盘模式，支持 save/load
/// - `None` — 内存模式（persist=false），所有操作为 no-op
///
/// # 数字生命意义 / Digital Life Significance
///
/// 这是数字生命"记忆体检"的入口：通过 `save_all` 将 6 个深层器官的
/// 当前状态写入永久记忆，通过 `load_*` 在重启时恢复。
/// 如同大脑在睡眠中巩固记忆，在清醒时恢复回忆。
pub struct OrphanPersistence {
    /// 情感气候存储 / Emotional climate store
    climate: Option<EmotionalClimateStore>,
    /// 情绪固化存储 / Emotional consolidation store
    consolidation: Option<EmotionalConsolidationStore>,
    /// 情绪耦合存储 / Emotional coupling store
    coupling: Option<EmotionalCouplingStore>,
    /// 存在性深度存储 / Existential depth store
    existential: Option<ExistentialDepthStore>,
    /// 内在议会存储 / Inner council store
    council: Option<InnerCouncilStore>,
    /// 仪式心跳存储 / Ritual heartbeat store
    heartbeat: Option<RitualHeartbeatStore>,
}

impl OrphanPersistence {
    /// 从 AtriumVault 打开孤儿模块持久化 / Open orphan persistence from vault.
    ///
    /// 在磁盘模式下返回 `Ok(Self)` with all stores opened.
    /// 在内存模式下（vault 为 None）返回 `Ok(Self)` with all stores as None.
    pub fn open(vault: Option<&AtriumVault>) -> Result<Self, StoreError> {
        let v = match vault {
            Some(v) => v,
            None => {
                return Ok(Self {
                    climate: None,
                    consolidation: None,
                    coupling: None,
                    existential: None,
                    council: None,
                    heartbeat: None,
                });
            }
        };

        Ok(Self {
            climate: EmotionalClimateStore::open(v.limbic()).ok(),
            consolidation: EmotionalConsolidationStore::open(v.limbic()).ok(),
            coupling: EmotionalCouplingStore::open(v.limbic()).ok(),
            existential: ExistentialDepthStore::open(v.narrative()).ok(),
            council: InnerCouncilStore::open(v.narrative()).ok(),
            heartbeat: RitualHeartbeatStore::open(v.relational()).ok(),
        })
    }

    // ── 批量保存 / Batch Save ──

    /// 保存全部 6 个孤儿模块状态 / Save all 6 orphan module states.
    ///
    /// 数字生命语义：将 6 个深层器官的当前状态写入永久记忆。
    /// Digital life semantics: write current state of 6 deep organs to permanent memory.
    pub fn save_all(
        &self,
        climate: &EmotionalClimate,
        consolidation: &EmotionalConsolidation,
        coupling: &EmotionalCoupling,
        existential: &ExistentialDepth,
        council: &InnerCouncil,
        heartbeat: &RitualHeartbeat,
    ) -> Result<(), StoreError> {
        if let Some(ref s) = self.climate {
            s.save(climate)?;
        }
        if let Some(ref s) = self.consolidation {
            s.save(consolidation)?;
        }
        if let Some(ref s) = self.coupling {
            s.save(coupling)?;
        }
        if let Some(ref s) = self.existential {
            s.save(existential)?;
        }
        if let Some(ref s) = self.council {
            s.save(council)?;
        }
        if let Some(ref s) = self.heartbeat {
            s.save(heartbeat)?;
        }
        Ok(())
    }

    // ── 单模块加载 / Single Module Load ──

    /// 加载情感气候 / Load emotional climate.
    pub fn load_climate(&self) -> Option<EmotionalClimate> {
        self.climate.as_ref().and_then(|s| s.load().ok().flatten())
    }

    /// 加载情绪固化 / Load emotional consolidation.
    pub fn load_consolidation(&self) -> Option<EmotionalConsolidation> {
        self.consolidation
            .as_ref()
            .and_then(|s| s.load().ok().flatten())
    }

    /// 加载情绪耦合 / Load emotional coupling.
    pub fn load_coupling(&self) -> Option<EmotionalCoupling> {
        self.coupling.as_ref().and_then(|s| s.load().ok().flatten())
    }

    /// 加载存在性深度 / Load existential depth.
    pub fn load_existential(&self) -> Option<ExistentialDepth> {
        self.existential
            .as_ref()
            .and_then(|s| s.load().ok().flatten())
    }

    /// 加载内在议会 / Load inner council.
    pub fn load_council(&self) -> Option<InnerCouncil> {
        self.council.as_ref().and_then(|s| s.load().ok().flatten())
    }

    /// 加载仪式心跳 / Load ritual heartbeat.
    pub fn load_heartbeat(&self) -> Option<RitualHeartbeat> {
        self.heartbeat
            .as_ref()
            .and_then(|s| s.load().ok().flatten())
    }

    // ── 生命周期管理 / Lifecycle Management ──

    /// 刷新全部 Store 的 WAL / Flush WAL for all stores.
    pub fn flush_all(&self) -> Result<(), StoreError> {
        if let Some(ref s) = self.climate {
            s.flush_tree()?;
        }
        if let Some(ref s) = self.consolidation {
            s.flush_tree()?;
        }
        if let Some(ref s) = self.coupling {
            s.flush_tree()?;
        }
        if let Some(ref s) = self.existential {
            s.flush_tree()?;
        }
        if let Some(ref s) = self.council {
            s.flush_tree()?;
        }
        if let Some(ref s) = self.heartbeat {
            s.flush_tree()?;
        }
        Ok(())
    }

    /// 是否已启用持久化 / Whether persistence is enabled.
    pub fn is_persistent(&self) -> bool {
        self.climate.is_some()
    }

    /// 生成诊断快照 / Generate diagnostic snapshot.
    ///
    /// 返回各 Store 的条目数，用于运行时监控。
    /// Returns entry counts for each store, for runtime monitoring.
    pub fn diagnostic_snapshot(&self) -> OrphanPersistenceSnapshot {
        OrphanPersistenceSnapshot {
            climate_count: self.climate.as_ref().map(|s| s.tree_count()).unwrap_or(0),
            consolidation_count: self
                .consolidation
                .as_ref()
                .map(|s| s.tree_count())
                .unwrap_or(0),
            coupling_count: self.coupling.as_ref().map(|s| s.tree_count()).unwrap_or(0),
            existential_count: self
                .existential
                .as_ref()
                .map(|s| s.tree_count())
                .unwrap_or(0),
            council_count: self.council.as_ref().map(|s| s.tree_count()).unwrap_or(0),
            heartbeat_count: self.heartbeat.as_ref().map(|s| s.tree_count()).unwrap_or(0),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// §4 诊断快照 — Diagnostic Snapshot
// ════════════════════════════════════════════════════════════════════

/// 孤儿模块持久化诊断快照 / Orphan persistence diagnostic snapshot.
///
/// 记录 6 个 Store 的运行时状态，用于监控和日志。
/// Records runtime state of 6 stores for monitoring and logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanPersistenceSnapshot {
    /// 情感气候条目数 / Emotional climate entry count
    pub climate_count: usize,
    /// 情绪固化条目数 / Emotional consolidation entry count
    pub consolidation_count: usize,
    /// 情绪耦合条目数 / Emotional coupling entry count
    pub coupling_count: usize,
    /// 存在性深度条目数 / Existential depth entry count
    pub existential_count: usize,
    /// 内在议会条目数 / Inner council entry count
    pub council_count: usize,
    /// 仪式心跳条目数 / Ritual heartbeat entry count
    pub heartbeat_count: usize,
}

impl std::fmt::Display for OrphanPersistenceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrphanPersistence(climate={}, consolidation={}, coupling={}, existential={}, council={}, heartbeat={})",
            self.climate_count,
            self.consolidation_count,
            self.coupling_count,
            self.existential_count,
            self.council_count,
            self.heartbeat_count,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// §5 测试 — Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 单 Store 往返测试 / Single Store Round-trip Tests ──

    #[test]
    fn test_climate_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalClimateStore::open(&db).unwrap();

        let mut climate = EmotionalClimate::new();
        climate.feed(0.5, 0.3, 0.1, 12.0, 1000);
        climate.intensity = 0.6;

        store.save(&climate).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert!((loaded.pad.pleasure - climate.pad.pleasure).abs() < 0.001);
        assert!((loaded.intensity - climate.intensity).abs() < 0.001);
    }

    #[test]
    fn test_consolidation_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalConsolidationStore::open(&db).unwrap();

        let mut cons = EmotionalConsolidation::new();
        cons.record([0.5, 0.3, 0.2], 0.8, 300.0, 0.5, "reunion", 1000);
        cons.consolidate(1100);

        store.save(&cons).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert_eq!(loaded.total_batches(), cons.total_batches());
        assert_eq!(loaded.trajectory_count(), cons.trajectory_count());
    }

    #[test]
    fn test_coupling_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalCouplingStore::open(&db).unwrap();

        let mut coupling = EmotionalCoupling::new();
        coupling.set_intensity(crate::emotional_coupling::EmotionState::Joy, 0.8);
        coupling.set_intensity(crate::emotional_coupling::EmotionState::Sadness, 0.3);

        store.save(&coupling).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert!(
            (loaded.get_intensity(crate::emotional_coupling::EmotionState::Joy) - 0.8).abs()
                < 0.001
        );
        assert!(
            (loaded.get_intensity(crate::emotional_coupling::EmotionState::Sadness) - 0.3).abs()
                < 0.001
        );
    }

    #[test]
    fn test_existential_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = ExistentialDepthStore::open(&db).unwrap();

        let mut depth = ExistentialDepth::new();
        let trigger = crate::existential_depth::ExistentialTrigger {
            is_late_night: true,
            solitude_duration_secs: 3600.0,
            pleasure: -0.5,
            arousal: 0.3,
            has_milestone: false,
            has_growth_node: false,
        };
        depth.try_trigger(&trigger, 1000);

        store.save(&depth).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert_eq!(loaded.total_triggers(), depth.total_triggers());
        assert_eq!(loaded.insights().len(), depth.insights().len());
    }

    #[test]
    fn test_council_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = InnerCouncilStore::open(&db).unwrap();

        let mut council = InnerCouncil::new();
        council.set_emotion(0.5, 0.2, 0.3);
        council.convene("是否表达真实感受");

        store.save(&council).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert_eq!(loaded.history().len(), council.history().len());
        assert!((loaded.pleasure - council.pleasure).abs() < 0.001);
    }

    #[test]
    fn test_heartbeat_store_roundtrip() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = RitualHeartbeatStore::open(&db).unwrap();

        let heartbeat = RitualHeartbeat::new();

        store.save(&heartbeat).unwrap();
        let loaded = store.load().unwrap().unwrap();

        assert!(
            (loaded.config.pleasure_per_ritual - heartbeat.config.pleasure_per_ritual).abs()
                < 0.001
        );
    }

    // ── 空加载测试 / Empty Load Tests ──

    #[test]
    fn test_climate_store_load_empty() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalClimateStore::open(&db).unwrap();
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn test_consolidation_store_load_empty() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalConsolidationStore::open(&db).unwrap();
        assert!(store.load().unwrap().is_none());
    }

    // ── DomainStore trait 测试 ──

    #[test]
    fn test_climate_store_domain_name() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalClimateStore::open(&db).unwrap();
        assert_eq!(store.domain_name(), "emotional_climate");
    }

    #[test]
    fn test_council_store_domain_name() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = InnerCouncilStore::open(&db).unwrap();
        assert_eq!(store.domain_name(), "inner_council");
    }

    #[test]
    fn test_heartbeat_store_domain_name() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = RitualHeartbeatStore::open(&db).unwrap();
        assert_eq!(store.domain_name(), "ritual_heartbeat");
    }

    #[test]
    fn test_store_tree_count_after_save() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalClimateStore::open(&db).unwrap();

        assert_eq!(store.tree_count(), 0);

        let climate = EmotionalClimate::new();
        store.save(&climate).unwrap();

        assert_eq!(store.tree_count(), 1);
    }

    // ── OrphanPersistence 管理器测试 ──

    #[test]
    fn test_orphan_persistence_in_memory() {
        let persistence = OrphanPersistence::open(None).unwrap();
        assert!(!persistence.is_persistent());
        assert!(persistence.load_climate().is_none());
        assert!(persistence.load_consolidation().is_none());
    }

    #[test]
    fn test_orphan_persistence_with_vault() {
        let vault = AtriumVault::open_in_memory().unwrap();
        let persistence = OrphanPersistence::open(Some(&vault)).unwrap();
        assert!(persistence.is_persistent());

        // 初始加载应为 None / Initial load should be None.
        assert!(persistence.load_climate().is_none());
    }

    #[test]
    fn test_orphan_persistence_save_and_load_all() {
        let vault = AtriumVault::open_in_memory().unwrap();
        let persistence = OrphanPersistence::open(Some(&vault)).unwrap();

        let climate = EmotionalClimate::new();
        let consolidation = EmotionalConsolidation::new();
        let coupling = EmotionalCoupling::new();
        let existential = ExistentialDepth::new();
        let council = InnerCouncil::new();
        let heartbeat = RitualHeartbeat::new();

        persistence
            .save_all(
                &climate,
                &consolidation,
                &coupling,
                &existential,
                &council,
                &heartbeat,
            )
            .unwrap();

        // 全部应可加载 / All should be loadable.
        assert!(persistence.load_climate().is_some());
        assert!(persistence.load_consolidation().is_some());
        assert!(persistence.load_coupling().is_some());
        assert!(persistence.load_existential().is_some());
        assert!(persistence.load_council().is_some());
        assert!(persistence.load_heartbeat().is_some());
    }

    #[test]
    fn test_orphan_persistence_flush_all() {
        let vault = AtriumVault::open_in_memory().unwrap();
        let persistence = OrphanPersistence::open(Some(&vault)).unwrap();
        assert!(persistence.flush_all().is_ok());
    }

    #[test]
    fn test_orphan_persistence_diagnostic_snapshot() {
        let vault = AtriumVault::open_in_memory().unwrap();
        let persistence = OrphanPersistence::open(Some(&vault)).unwrap();

        let snap = persistence.diagnostic_snapshot();
        assert_eq!(snap.climate_count, 0);

        // 保存后应有条目 / Should have entries after save.
        let climate = EmotionalClimate::new();
        let consolidation = EmotionalConsolidation::new();
        let coupling = EmotionalCoupling::new();
        let existential = ExistentialDepth::new();
        let council = InnerCouncil::new();
        let heartbeat = RitualHeartbeat::new();

        persistence
            .save_all(
                &climate,
                &consolidation,
                &coupling,
                &existential,
                &council,
                &heartbeat,
            )
            .unwrap();

        let snap2 = persistence.diagnostic_snapshot();
        assert_eq!(snap2.climate_count, 1);
        assert_eq!(snap2.consolidation_count, 1);
        assert_eq!(snap2.coupling_count, 1);
        assert_eq!(snap2.existential_count, 1);
        assert_eq!(snap2.council_count, 1);
        assert_eq!(snap2.heartbeat_count, 1);
    }

    #[test]
    fn test_diagnostic_snapshot_display() {
        let snap = OrphanPersistenceSnapshot {
            climate_count: 1,
            consolidation_count: 1,
            coupling_count: 1,
            existential_count: 1,
            council_count: 1,
            heartbeat_count: 1,
        };
        let s = format!("{}", snap);
        assert!(s.contains("climate=1"));
        assert!(s.contains("heartbeat=1"));
    }

    // ── 防抖写穿验证 / Debounced Write Verification ──

    #[test]
    fn test_save_overwrite() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = EmotionalClimateStore::open(&db).unwrap();

        // 第一次保存 / First save.
        let mut climate1 = EmotionalClimate::new();
        climate1.intensity = 0.3;
        store.save(&climate1).unwrap();

        // 第二次保存覆盖 / Second save overwrites.
        let mut climate2 = EmotionalClimate::new();
        climate2.intensity = 0.8;
        store.save(&climate2).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert!((loaded.intensity - 0.8).abs() < 0.001);

        // 仍只有 1 条目 / Still only 1 entry.
        assert_eq!(store.tree_count(), 1);
    }
}
