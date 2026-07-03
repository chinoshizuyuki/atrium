// SPDX-License-Identifier: MIT
//! 冲突与和解存储 — ConflictManager 的 sled 持久化
//! ConflictStore — ConflictManager model persisted via sled.
//!
//! 将冲突管理器（分歧检测、过度索取检测、升级控制、和解工艺、道歉引擎、冲突状态）
//! 持久化到 sled，消除重启失忆缺陷，支持跨会话的冲突记忆连续性。

use serde::{Deserialize, Serialize};

use crate::conflict_reconciliation::{
    ApologyTemplate, ConflictConfig, ConflictIntensity, ConflictPadBridge, ConflictSignal,
    ConflictState, EscalationConfig, ProactiveReconcilerConfig, ReconciliationConfig,
    ReconciliationRitual, RecoveryCurve,
};

// ════════════════════════════════════════════════════════════════════
// ConflictStoreError — 存储错误类型 / Storage Error Type
// ════════════════════════════════════════════════════════════════════

/// 冲突存储错误 / Conflict store error
#[derive(Debug)]
pub enum ConflictStoreError {
    /// sled 数据库错误 / Sled database error
    SledError(String),
    /// 序列化/反序列化错误 / Codec (de)serialization error
    CodecError(String),
}

impl std::fmt::Display for ConflictStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SledError(e) => write!(f, "conflict sled error: {}", e),
            Self::CodecError(e) => write!(f, "conflict codec error: {}", e),
        }
    }
}

impl std::error::Error for ConflictStoreError {}

// ════════════════════════════════════════════════════════════════════
// SerializableConflictManager — 可序列化的冲突管理器快照
// Serializable ConflictManager snapshot for sled bincode persistence
// ════════════════════════════════════════════════════════════════════

/// 可序列化的冲突管理器快照 — 用于 sled bincode 持久化
///
/// ConflictManager 的 1:1 镜像，确保 bincode 稳定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableConflictManager {
    // ── 分歧检测器状态 / Disagreement detector state ──
    /// 检测灵敏度 / Detection sensitivity
    pub disagreement_sensitivity: f64,

    // ── 过度索取检测器状态 / Over-demand detector state ──
    /// 累计索取计数 / Cumulative demand count
    pub demand_count: u32,
    /// 索取窗口 / Demand window
    pub demand_window: Vec<f64>,
    /// 窗口大小 / Window size
    pub window_size: usize,
    /// 高频索取阈值 / High-frequency demand threshold
    pub high_freq_threshold: f64,

    // ── 升级控制器状态 / Escalation controller state ──
    /// 升级控制器配置 / Escalation config
    pub escalation_config: EscalationConfig,
    /// 当前升级级别 / Current escalation level
    pub current_escalation_level: ConflictIntensity,
    /// 无冲突轮次计数 / Calm turns count
    pub calm_turns: u32,

    // ── 和解工艺配置 / Reconciliation craft config ──
    /// 和解工艺配置 / Reconciliation config
    pub reconciliation_config: ReconciliationConfig,

    // ── 道歉引擎状态 / Apology engine state ──
    /// 道歉模板列表 / Apology templates
    pub apology_templates: Vec<ApologyTemplate>,

    // ── 冲突状态 / Conflict state ──
    /// 冲突状态 / Conflict state
    pub state: ConflictState,

    // ── G1: 主动和解管线状态 / G1: Proactive reconciler state ──
    /// 主动和解配置 / Proactive reconciler config
    pub proactive_config: ProactiveReconcilerConfig,
    /// 本会话主动和解次数 / Proactive reconciliation count this session
    pub proactive_count: u32,
    /// 上次主动和解时间戳 / Last proactive reconciliation timestamp
    pub last_proactive_ts: i64,

    // ── G2: 冲突↔情绪双向闭环 / G2: Conflict↔emotion PAD bridge ──
    /// PAD桥接参数 / PAD bridge parameters
    pub pad_bridge: ConflictPadBridge,

    // ── G4: 冲突恢复曲线 / G4: Conflict recovery curve ──
    /// 恢复曲线 / Recovery curve
    pub recovery_curve: RecoveryCurve,

    // ── G5: 和解仪式 / G5: Reconciliation ritual ──
    /// 和解仪式（可能为空）/ Reconciliation ritual (optional)
    pub ritual: Option<ReconciliationRitual>,

    // ── 全局配置 / Global config ──
    /// 冲突管理器配置 / Conflict manager config
    pub config: ConflictConfig,
}

impl From<&crate::conflict_reconciliation::ConflictManager> for SerializableConflictManager {
    fn from(mgr: &crate::conflict_reconciliation::ConflictManager) -> Self {
        Self {
            disagreement_sensitivity: mgr.disagreement.sensitivity,
            demand_count: mgr.over_demand.demand_count,
            demand_window: mgr.over_demand.demand_window.clone(),
            window_size: mgr.over_demand.window_size,
            high_freq_threshold: mgr.over_demand.high_freq_threshold,
            escalation_config: mgr.escalation.config.clone(),
            current_escalation_level: mgr.escalation.current_level,
            calm_turns: mgr.escalation.calm_turns,
            reconciliation_config: mgr.reconciliation.config.clone(),
            apology_templates: mgr.apology.templates.clone(),
            state: mgr.state.clone(),
            // G1: 主动和解管线 / G1: Proactive reconciler
            proactive_config: mgr.proactive_reconciler.config.clone(),
            proactive_count: mgr.proactive_reconciler.proactive_count,
            last_proactive_ts: mgr.proactive_reconciler.last_proactive_ts,
            // G2: PAD桥接 / G2: PAD bridge
            pad_bridge: mgr.pad_bridge.clone(),
            // G4: 恢复曲线 / G4: Recovery curve
            recovery_curve: mgr.recovery_curve.clone(),
            // G5: 和解仪式 / G5: Reconciliation ritual
            ritual: mgr.ritual.clone(),
            config: ConflictConfig {
                disagreement_sensitivity: mgr.disagreement.sensitivity,
                over_demand_window: mgr.over_demand.window_size,
                over_demand_threshold: mgr.over_demand.high_freq_threshold,
                escalation: mgr.escalation.config.clone(),
                reconciliation: mgr.reconciliation.config.clone(),
            },
        }
    }
}

impl SerializableConflictManager {
    /// 还原为 ConflictManager / Restore into ConflictManager
    pub fn into_manager(self) -> crate::conflict_reconciliation::ConflictManager {
        use crate::conflict_reconciliation::ConflictManager;

        let mut mgr = ConflictManager::new(self.config);

        // 恢复分歧检测器灵敏度 / Restore disagreement sensitivity
        mgr.disagreement.sensitivity = self.disagreement_sensitivity;

        // 恢复过度索取检测器状态 / Restore over-demand detector state
        mgr.over_demand.demand_count = self.demand_count;
        mgr.over_demand.demand_window = self.demand_window;
        mgr.over_demand.window_size = self.window_size;
        mgr.over_demand.high_freq_threshold = self.high_freq_threshold;

        // 恢复升级控制器状态 / Restore escalation controller state
        mgr.escalation.config = self.escalation_config;
        mgr.escalation.current_level = self.current_escalation_level;
        mgr.escalation.calm_turns = self.calm_turns;

        // 恢复和解工艺配置 / Restore reconciliation config
        mgr.reconciliation.config = self.reconciliation_config;

        // 恢复道歉引擎模板 / Restore apology templates
        mgr.apology.templates = self.apology_templates;

        // 恢复冲突状态 / Restore conflict state
        mgr.state = self.state;

        // G1: 恢复主动和解管线 / G1: Restore proactive reconciler
        mgr.proactive_reconciler.config = self.proactive_config;
        mgr.proactive_reconciler.proactive_count = self.proactive_count;
        mgr.proactive_reconciler.last_proactive_ts = self.last_proactive_ts;

        // G2: 恢复PAD桥接 / G2: Restore PAD bridge
        mgr.pad_bridge = self.pad_bridge;

        // G4: 恢复恢复曲线 / G4: Restore recovery curve
        mgr.recovery_curve = self.recovery_curve;

        // G5: 恢复和解仪式 / G5: Restore reconciliation ritual
        mgr.ritual = self.ritual;

        mgr
    }
}

// ════════════════════════════════════════════════════════════════════
// ConflictStore — sled 持久化存储 / Sled persistent store
// ════════════════════════════════════════════════════════════════════

/// 冲突存储 — 基于 sled 的冲突管理器持久化
///
/// Key: "manager" (单例)
/// Value: SerializableConflictManager (bincode serialized)
///
/// 辅助 tree：冲突信号索引
pub struct ConflictStore {
    /// 主 tree：存储完整冲突管理器快照 / Main tree: full manager snapshot
    tree: sled::Tree,
    /// 冲突信号索引 tree：timestamp → signal bincode / Conflict signal index tree
    signal_tree: sled::Tree,
}

impl ConflictStore {
    /// 打开或创建冲突存储 / Open or create conflict store
    pub fn open(db: &sled::Db) -> Result<Self, ConflictStoreError> {
        let tree = db
            .open_tree("conflict_manager")
            .map_err(|e| ConflictStoreError::SledError(e.to_string()))?;
        let signal_tree = db
            .open_tree("conflict_signals")
            .map_err(|e| ConflictStoreError::SledError(e.to_string()))?;
        Ok(Self { tree, signal_tree })
    }

    /// 保存完整的冲突管理器 / Save full conflict manager
    pub fn save(
        &self,
        mgr: &crate::conflict_reconciliation::ConflictManager,
    ) -> Result<(), ConflictStoreError> {
        let snapshot = SerializableConflictManager::from(mgr);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| ConflictStoreError::CodecError(e.to_string()))?;
        self.tree
            .insert(b"manager", value.as_slice())
            .map_err(|e| ConflictStoreError::SledError(e.to_string()))?;

        // 同步更新冲突信号索引 / Sync conflict signal index
        for signal in &mgr.state.active_conflicts {
            let key = signal.timestamp.to_be_bytes();
            let val = bincode::serialize(signal)
                .map_err(|e| ConflictStoreError::CodecError(e.to_string()))?;
            self.signal_tree
                .insert(key, val.as_slice())
                .map_err(|e| ConflictStoreError::SledError(e.to_string()))?;
        }

        Ok(())
    }

    /// 加载冲突管理器 / Load conflict manager
    pub fn load(
        &self,
    ) -> Result<crate::conflict_reconciliation::ConflictManager, ConflictStoreError> {
        match self.tree.get(b"manager") {
            Ok(Some(value)) => {
                let snapshot: SerializableConflictManager = bincode::deserialize(&value)
                    .map_err(|e| ConflictStoreError::CodecError(e.to_string()))?;
                Ok(snapshot.into_manager())
            }
            Ok(None) => Ok(crate::conflict_reconciliation::ConflictManager::default()),
            Err(e) => Err(ConflictStoreError::SledError(e.to_string())),
        }
    }

    /// 获取指定时间戳的冲突信号 / Get conflict signal by timestamp
    pub fn get_signal(&self, timestamp: i64) -> Result<Option<ConflictSignal>, ConflictStoreError> {
        let key = timestamp.to_be_bytes();
        match self.signal_tree.get(key) {
            Ok(Some(value)) => {
                let signal: ConflictSignal = bincode::deserialize(&value)
                    .map_err(|e| ConflictStoreError::CodecError(e.to_string()))?;
                Ok(Some(signal))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ConflictStoreError::SledError(e.to_string())),
        }
    }

    /// 获取所有冲突信号时间戳 / Get all conflict signal timestamps
    pub fn signal_timestamps(&self) -> Result<Vec<i64>, ConflictStoreError> {
        let mut timestamps = Vec::new();
        for item in self.signal_tree.iter() {
            let (key, _) = item.map_err(|e| ConflictStoreError::SledError(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            timestamps.push(i64::from_be_bytes(bytes));
        }
        Ok(timestamps)
    }

    /// 冲突信号总数 / Conflict signal count
    pub fn signal_count(&self) -> usize {
        self.signal_tree.len()
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict_reconciliation::ConflictManager;
    use crate::relationship::RelationshipStage;

    /// 测试用临时数据库 / Temporary test database
    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    /// 深层关系阶段 / Deep relationship stage
    fn deep_stage() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        }
    }

    #[test]
    fn test_store_save_and_load_default() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();
        let mgr = ConflictManager::default();
        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.state.active_conflicts.is_empty());
        assert_eq!(loaded.state.total_conflicts, 0);
    }

    #[test]
    fn test_store_save_and_load_with_conflict() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();

        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        // 触发一次冲突 / Trigger a conflict
        let _ = mgr.process("不是这样的，你理解错了", -0.2, 0.3, &stage, 1000);

        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.state.total_conflicts, mgr.state.total_conflicts);
        assert_eq!(loaded.state.consecutive_turns, mgr.state.consecutive_turns);
    }

    #[test]
    fn test_store_preserves_escalation_state() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();

        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        // 触发多次冲突以升级 / Trigger multiple conflicts to escalate
        for ts in 1000..1005 {
            let _ = mgr.process("你错了，不是这样的", -0.4, 0.5, &stage, ts);
        }

        let saved_level = mgr.escalation.current_level;
        let saved_calm = mgr.escalation.calm_turns;

        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.escalation.current_level, saved_level);
        assert_eq!(loaded.escalation.calm_turns, saved_calm);
    }

    #[test]
    fn test_store_preserves_over_demand_state() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();

        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        // 触发过度索取 / Trigger over-demand
        let _ = mgr.process("你必须马上给我结果", -0.2, 0.5, &stage, 1000);

        let saved_count = mgr.over_demand.demand_count;
        let saved_window_len = mgr.over_demand.demand_window.len();

        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.over_demand.demand_count, saved_count);
        assert_eq!(loaded.over_demand.demand_window.len(), saved_window_len);
    }

    #[test]
    fn test_store_signal_index() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();

        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let _ = mgr.process("不是这样的", -0.2, 0.3, &stage, 12345);

        store.save(&mgr).unwrap();

        if !mgr.state.active_conflicts.is_empty() {
            assert!(store.signal_count() > 0);
        }
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = ConflictStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.state.active_conflicts.is_empty());
        assert_eq!(loaded.state.total_conflicts, 0);
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut mgr = ConflictManager::default();
        let stage = deep_stage();
        let _ = mgr.process("你错了，不是这样的", -0.3, 0.4, &stage, 1000);

        let snapshot = SerializableConflictManager::from(&mgr);
        let restored = snapshot.into_manager();

        assert_eq!(restored.state.total_conflicts, mgr.state.total_conflicts);
        assert_eq!(
            restored.escalation.current_level,
            mgr.escalation.current_level
        );
        assert_eq!(
            restored.over_demand.demand_count,
            mgr.over_demand.demand_count
        );
        assert_eq!(
            restored.disagreement.sensitivity,
            mgr.disagreement.sensitivity
        );
    }
}
