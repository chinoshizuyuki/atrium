// SPDX-License-Identifier: MIT
//! 适度犯错存储 — ImperfectionEngine 的 sled 持久化
//! ImperfectionStore — ImperfectionEngine model persisted via sled.
//!
//! 将适度犯错引擎的完整状态持久化到 sled，
//! 确保跨会话的犯错历史、领域熟悉度、疲劳累积与自纠队列连续性。
//!
//! 序列化策略：
//! - ImperfectionEngine 包含三个不可序列化字段：
//!   - `last_mistake_instant: Option<Instant>` — 单调时钟，跨重启无意义
//!   - `rng: SmallRng` — 随机状态，重启后以 from_entropy() 重新初始化
//!   - `instant_base: Instant` — epoch 映射参考基点，重启后重新采样
//! - SerializableImperfectionEngine 仅镜像可序列化字段，
//!   还原时通过 ImperfectionEngine::reconstruct() 重建完整引擎。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::imperfection_engine::{
    ImperfectionConfig, ImperfectionEngine, ImperfectionStats, PendingCorrection,
};

// ════════════════════════════════════════════════════════════════════
// crate::store_core::StoreError — 存储错误类型 / Storage Error Type
// ════════════════════════════════════════════════════════════════════

// 统一使用 store_core::StoreError / Unified StoreError from store_core
// ════════════════════════════════════════════════════════════════════
// SerializableImperfectionEngine — 可序列化的引擎快照
// Serializable engine snapshot for sled bincode persistence
// ════════════════════════════════════════════════════════════════════

/// 可序列化的引擎快照 — 用于 sled bincode 持久化
///
/// ImperfectionEngine 的 1:1 镜像（排除 Instant 和 SmallRng），
/// 确保跨重启的 bincode 稳定。还原时通过 `into_engine()` 调用
/// `ImperfectionEngine::reconstruct()` 重建完整引擎。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableImperfectionEngine {
    /// 配置 / Configuration
    pub config: ImperfectionConfig,
    /// 领域熟悉度 / Domain familiarity map
    pub domain_familiarity: HashMap<String, f64>,
    /// 当前认知负荷 / Current cognitive load
    pub cognitive_load: f64,
    /// 当前疲劳度 / Current fatigue
    pub fatigue: f64,
    /// 当前情绪干扰值 / Current emotional interference
    pub emotional_interference: f64,
    /// 关系深度 / Relationship depth
    pub relationship_depth: f64,
    /// 成熟度序号 / Maturity ordinal
    pub maturity_ordinal: u32,
    /// 待执行自纠队列 / Pending corrections queue
    pub pending_corrections: Vec<PendingCorrection>,
    /// 犯错统计 / Statistics
    pub stats: ImperfectionStats,
    /// 下一犯错记录 ID / Next mistake record ID
    pub next_id: u64,
    /// 本轮已犯错次数 / Mistakes made in current turn
    pub turn_mistake_count: u32,
}

impl From<&ImperfectionEngine> for SerializableImperfectionEngine {
    fn from(engine: &ImperfectionEngine) -> Self {
        Self {
            config: engine.config().clone(),
            domain_familiarity: engine.domain_familiarity_snapshot(),
            cognitive_load: engine.cognitive_load(),
            fatigue: engine.fatigue(),
            emotional_interference: engine.emotional_interference(),
            relationship_depth: engine.relationship_depth(),
            maturity_ordinal: engine.maturity_ordinal(),
            pending_corrections: engine.pending_corrections().to_vec(),
            stats: engine.stats().clone(),
            next_id: engine.next_id(),
            turn_mistake_count: engine.turn_mistake_count(),
        }
    }
}

impl SerializableImperfectionEngine {
    /// 还原为 ImperfectionEngine / Restore into ImperfectionEngine
    ///
    /// 通过 reconstruct() 重建，RNG 以 Stochastic 模式重新初始化，
    /// last_mistake_instant 置 None（冷却期自然恢复）。
    pub fn into_engine(self) -> ImperfectionEngine {
        ImperfectionEngine::reconstruct(
            self.config,
            self.domain_familiarity,
            self.cognitive_load,
            self.fatigue,
            self.emotional_interference,
            self.relationship_depth,
            self.maturity_ordinal,
            self.pending_corrections,
            self.stats,
            self.next_id,
            self.turn_mistake_count,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// ImperfectionStore — sled 持久化存储 / Sled persistent store
// ════════════════════════════════════════════════════════════════════

/// 适度犯错存储 — 基于 sled 的引擎持久化
///
/// Key: "engine" (单例)
/// Value: SerializableImperfectionEngine (bincode serialized)
///
/// 辅助 tree：犯错历史索引 (record_id → record bincode)
pub struct ImperfectionStore {
    /// 主 tree：存储完整引擎快照 / Main tree: full engine snapshot
    tree: sled::Tree,
    /// 历史索引 tree：record_id → record bincode / History index tree
    history_tree: sled::Tree,
}

impl ImperfectionStore {
    /// 打开或创建适度犯错存储 / Open or create imperfection store
    pub fn open(db: &sled::Db) -> Result<Self, crate::store_core::StoreError> {
        let tree = db
            .open_tree("imperfection_engine")
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        let history_tree = db
            .open_tree("imperfection_history")
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        Ok(Self { tree, history_tree })
    }

    /// 保存引擎状态 / Save imperfection engine state
    ///
    /// 将引擎快照序列化到主 tree。
    /// 注意：MistakeRecord 由 CoreService 在 record_mistake 后通过 save_record() 单独写入
    /// history_tree；此处仅保存引擎内部状态（含 pending_corrections）。
    /// history_tree 的 key 空间专属于 MistakeRecord，禁止写入 PendingCorrection
    /// 以避免类型混淆导致反序列化失败。
    pub fn save(&self, engine: &ImperfectionEngine) -> Result<(), crate::store_core::StoreError> {
        let snapshot = SerializableImperfectionEngine::from(engine);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
        self.tree
            .insert(b"engine", value.as_slice())
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;

        Ok(())
    }

    /// 加载引擎状态 / Load imperfection engine state
    ///
    /// 从 sled 反序列化快照，通过 reconstruct() 重建完整引擎。
    /// 若无持久化数据，返回默认引擎。
    pub fn load(&self) -> Result<ImperfectionEngine, crate::store_core::StoreError> {
        match self.tree.get(b"engine") {
            Ok(Some(value)) => {
                let snapshot: SerializableImperfectionEngine = bincode::deserialize(&value)
                    .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
                Ok(snapshot.into_engine())
            }
            Ok(None) => Ok(ImperfectionEngine::new(ImperfectionConfig::default())),
            Err(e) => Err(crate::store_core::StoreError::Sled(e.to_string())),
        }
    }

    /// 保存单条犯错记录到历史索引 / Save a single mistake record to history index
    ///
    /// 供 CoreService 在 record_mistake() 后调用，将 MistakeRecord
    /// 持久化到 history_tree 以支持跨会话查询。
    pub fn save_record(
        &self,
        record: &crate::imperfection_engine::MistakeRecord,
    ) -> Result<(), crate::store_core::StoreError> {
        let key = record.id.to_be_bytes();
        let val = bincode::serialize(record)
            .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
        self.history_tree
            .insert(key, val.as_slice())
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        Ok(())
    }

    /// 获取单条犯错记录 / Get a single mistake record by ID
    pub fn get_record(
        &self,
        record_id: u64,
    ) -> Result<Option<crate::imperfection_engine::MistakeRecord>, crate::store_core::StoreError>
    {
        let key = record_id.to_be_bytes();
        match self.history_tree.get(key) {
            Ok(Some(value)) => {
                let record: crate::imperfection_engine::MistakeRecord =
                    bincode::deserialize(&value)
                        .map_err(|e| crate::store_core::StoreError::Codec(e.to_string()))?;
                Ok(Some(record))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::store_core::StoreError::Sled(e.to_string())),
        }
    }

    /// 获取所有历史记录 ID / Get all history record IDs
    pub fn history_ids(&self) -> Result<Vec<u64>, crate::store_core::StoreError> {
        let mut ids = Vec::new();
        for item in self.history_tree.iter() {
            let (key, _) = item.map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 历史记录总数 / History record count
    pub fn history_count(&self) -> usize {
        self.history_tree.len()
    }
}

// ════════════════════════════════════════════════════════════════════
// DomainStore + VaultTree trait 实现 / Trait Implementations
// ════════════════════════════════════════════════════════════════════

/// VaultTree 实现 — 主 tree 承载 SerializableImperfectionEngine
/// VaultTree impl — main tree carries SerializableImperfectionEngine
impl crate::atrium_vault::VaultTree<SerializableImperfectionEngine> for ImperfectionStore {
    fn tree(&self) -> &sled::Tree {
        &self.tree
    }
}

/// DomainStore 实现 — 犯错记忆子系统的存储接口
/// DomainStore impl — imperfection memory subsystem store interface
impl crate::store_core::DomainStore for ImperfectionStore {
    fn domain_name(&self) -> &'static str {
        "imperfection"
    }

    fn tree_count(&self) -> usize {
        self.tree.len() + self.history_tree.len()
    }

    fn flush_tree(&self) -> Result<(), crate::store_core::StoreError> {
        self.tree.flush()?;
        self.history_tree.flush()?;
        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imperfection_engine::{
        ImperfectionConfig, MistakeKind, MistakeSeverity, MistakeTrigger,
    };
    use std::time::Instant;

    /// 测试用临时数据库 / Temporary test database
    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn test_store_save_and_load_default() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();
        let engine = ImperfectionEngine::new(ImperfectionConfig::default());

        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();

        assert!(loaded.pending_corrections().is_empty());
        assert_eq!(loaded.stats().total_mistakes, 0);
    }

    #[test]
    fn test_store_save_and_load_with_mistake() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let mut engine = ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42);
        let record = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Moderate,
            MistakeTrigger::Fatigue,
            0.2,
            "rust",
            Instant::now(),
        );

        // 同时保存记录到历史索引 / Also save record to history index
        store.save_record(&record).unwrap();
        store.save(&engine).unwrap();

        let loaded = store.load().unwrap();

        assert_eq!(loaded.stats().total_mistakes, 1);
        assert_eq!(loaded.stats().kind_counts[&MistakeKind::MemoryDrift], 1);
        assert_eq!(store.history_count(), 1);
    }

    #[test]
    fn test_store_preserves_config() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let config = ImperfectionConfig {
            enabled: false,
            base_prob: 0.1,
            ..Default::default()
        };
        let engine = ImperfectionEngine::new(config);

        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();

        assert!(!loaded.config().enabled);
        assert!((loaded.config().base_prob - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_store_preserves_domain_familiarity() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let mut engine = ImperfectionEngine::new(ImperfectionConfig::default());
        engine.set_familiarity("rust", 0.8);
        engine.set_familiarity("physics", 0.3);

        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();

        assert!((loaded.familiarity("rust") - 0.8).abs() < 1e-9);
        assert!((loaded.familiarity("physics") - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_store_preserves_state_values() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let mut engine = ImperfectionEngine::new(ImperfectionConfig::default());
        engine.set_cognitive_load(0.7);
        engine.set_fatigue(0.5);
        engine.set_emotional_interference(0.4);
        engine.set_relationship_depth(0.8);
        engine.set_maturity_ordinal(2);

        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();

        assert!((loaded.cognitive_load() - 0.7).abs() < 1e-9);
        assert!((loaded.fatigue() - 0.5).abs() < 1e-9);
        assert!((loaded.emotional_interference() - 0.4).abs() < 1e-9);
        assert!((loaded.relationship_depth() - 0.8).abs() < 1e-9);
        assert_eq!(loaded.maturity_ordinal(), 2);
    }

    #[test]
    fn test_store_history_index() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let mut engine = ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42);
        let record = engine.record_mistake(
            MistakeKind::KnowledgeBoundary,
            MistakeSeverity::Evident,
            MistakeTrigger::UnfamiliarDomain,
            0.3,
            "quantum",
            Instant::now(),
        );

        store.save_record(&record).unwrap();
        store.save(&engine).unwrap();

        assert_eq!(store.history_count(), 1);
        let loaded_record = store.get_record(record.id).unwrap().unwrap();
        assert_eq!(loaded_record.kind, MistakeKind::KnowledgeBoundary);
        assert_eq!(loaded_record.severity, MistakeSeverity::Evident);
        assert_eq!(loaded_record.domain, "quantum");
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.pending_corrections().is_empty());
        assert_eq!(loaded.stats().total_mistakes, 0);
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut engine = ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42);
        engine.set_familiarity("rust", 0.9);
        engine.set_cognitive_load(0.6);
        let _ = engine.record_mistake(
            MistakeKind::ReasoningLeap,
            MistakeSeverity::Moderate,
            MistakeTrigger::HighCognitiveLoad,
            0.25,
            "math",
            Instant::now(),
        );

        let snapshot = SerializableImperfectionEngine::from(&engine);

        // bincode 往返 / bincode roundtrip
        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: SerializableImperfectionEngine = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.stats.total_mistakes, engine.stats().total_mistakes);
        assert_eq!(
            restored.pending_corrections.len(),
            engine.pending_corrections().len()
        );
        assert!((restored.cognitive_load - engine.cognitive_load()).abs() < 1e-9);

        // 还原为引擎 / Restore into engine
        let restored_engine = restored.into_engine();
        assert_eq!(restored_engine.stats().total_mistakes, 1);
        assert_eq!(restored_engine.pending_corrections().len(), 1);
    }

    #[test]
    fn test_store_multiple_mistakes() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();

        let mut engine = ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42);
        let now = Instant::now();
        let r1 = engine.record_mistake(
            MistakeKind::IntentionalVagueness,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "general",
            now,
        );
        let r2 = engine.record_mistake(
            MistakeKind::OverSimplification,
            MistakeSeverity::Moderate,
            MistakeTrigger::HighCognitiveLoad,
            0.2,
            "complex",
            now,
        );

        store.save_record(&r1).unwrap();
        store.save_record(&r2).unwrap();
        store.save(&engine).unwrap();

        let loaded = store.load().unwrap();

        assert_eq!(loaded.stats().total_mistakes, 2);
        assert_eq!(store.history_count(), 2);
        assert_eq!(store.history_ids().unwrap().len(), 2);
    }

    #[test]
    fn test_store_get_record_nonexistent() {
        let db = test_db();
        let store = ImperfectionStore::open(&db).unwrap();
        assert!(store.get_record(999).unwrap().is_none());
    }

    #[test]
    fn test_reconstruct_resets_cooldown() {
        // reconstruct 后 last_mistake_instant = None，
        // 冷却期自然恢复，门控应开放
        let mut engine = ImperfectionEngine::new_deterministic(ImperfectionConfig::default(), 42);
        let _ = engine.record_mistake(
            MistakeKind::MemoryDrift,
            MistakeSeverity::Subtle,
            MistakeTrigger::Spontaneous,
            0.1,
            "test",
            Instant::now(),
        );

        let snapshot = SerializableImperfectionEngine::from(&engine);
        let restored = snapshot.into_engine();

        // 重构后冷却期已重置，门控应开放
        let (passed, _) = restored.check_gate(Instant::now());
        assert!(
            passed,
            "gate should be open after reconstruct (cooldown reset)"
        );
    }
}
