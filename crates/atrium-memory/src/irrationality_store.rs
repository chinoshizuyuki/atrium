// SPDX-License-Identifier: MIT
//! 情绪非理性存储 — IrrationalityManager 的 sled 持久化
//! IrrationalityStore — IrrationalityManager model persisted via sled.
//!
//! 将非理性管理器（脉冲引擎、残留引擎、传染引擎、混沌引擎）持久化到 sled，
//! 消除重启失忆缺陷，支持跨会话的情绪连续性。

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::emotional_irrationality::{
    ChaosConfig, ChaoticPulse, ContagionConfig, ContagionRule, ContagionRuleEntry, CrossContagion,
    EmotionChaos, EmotionResidue, PulseConfig, ResidueConfig, ShockAbsorber,
};

// ════════════════════════════════════════════════════════════════════
// IrrationalityStoreError — 存储错误类型 / Storage Error Type
// ════════════════════════════════════════════════════════════════════

/// 非理性存储错误 / Irrationality store error
#[derive(Debug)]
pub enum IrrationalityStoreError {
    /// sled 数据库错误 / Sled database error
    SledError(String),
    /// 序列化/反序列化错误 / Codec (de)serialization error
    CodecError(String),
}

impl std::fmt::Display for IrrationalityStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SledError(e) => write!(f, "irrationality sled error: {}", e),
            Self::CodecError(e) => write!(f, "irrationality codec error: {}", e),
        }
    }
}

impl std::error::Error for IrrationalityStoreError {}

// ════════════════════════════════════════════════════════════════════
// SerializableIrrationalityManager — 可序列化的非理性管理器快照
// Serializable IrrationalityManager snapshot for sled bincode persistence
// ════════════════════════════════════════════════════════════════════

/// 可序列化的非理性管理器快照 — 用于 sled bincode 持久化
///
/// IrrationalityManager 的 1:1 镜像，确保 bincode 稳定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableIrrationalityManager {
    // ── 脉冲引擎状态 / Pulse engine state ──
    /// 脉冲引擎配置 / Pulse engine config
    pub pulse_config: PulseConfig,
    /// 活跃脉冲列表 / Active pulses
    pub active_pulses: Vec<ChaoticPulse>,
    /// 冲击吸收器 / Shock absorber
    pub shock_absorber: ShockAbsorber,
    /// 脉冲引擎自增ID / Pulse engine next ID
    pub pulse_next_id: u64,

    // ── 残留引擎状态 / Residue engine state ──
    /// 残留引擎配置 / Residue engine config
    pub residue_config: ResidueConfig,
    /// 活跃残留列表 / Active residues
    pub active_residues: Vec<EmotionResidue>,
    /// 残留引擎自增ID / Residue engine next ID
    pub residue_next_id: u64,

    // ── 传染引擎状态 / Contagion engine state ──
    /// 传染引擎配置 / Contagion engine config
    pub contagion_config: ContagionConfig,
    /// 传染规则表 / Contagion rule table
    pub contagion_rules: Vec<ContagionRuleEntry>,
    /// 近期传染记录 / Recent contagions
    pub recent_contagions: Vec<CrossContagion>,
    /// 冷却索引 / Cooldown index: rule → last trigger timestamp
    pub last_trigger: HashMap<ContagionRule, i64>,
    /// 传染引擎自增ID / Contagion engine next ID
    pub contagion_next_id: u64,

    // ── 混沌引擎状态 / Chaos engine state ──
    /// 混沌引擎配置 / Chaos engine config
    pub chaos_config: ChaosConfig,
    /// 混沌状态 / Emotion chaos state
    pub chaos_state: EmotionChaos,

    // ── 全局配置 / Global config ──
    /// 是否启用 / Enabled flag
    pub enabled: bool,
    /// Prompt 预算 / Prompt budget
    pub prompt_budget: usize,
}

impl From<&crate::emotional_irrationality::IrrationalityManager>
    for SerializableIrrationalityManager
{
    fn from(mgr: &crate::emotional_irrationality::IrrationalityManager) -> Self {
        Self {
            pulse_config: mgr.pulse.config.clone(),
            active_pulses: mgr.pulse.active_pulses.clone(),
            shock_absorber: mgr.pulse.shock_absorber.clone(),
            pulse_next_id: mgr.pulse.next_id,

            residue_config: mgr.residue.config.clone(),
            active_residues: mgr.residue.active_residues.clone(),
            residue_next_id: mgr.residue.next_id,

            contagion_config: mgr.contagion.config.clone(),
            contagion_rules: mgr.contagion.rules.clone(),
            recent_contagions: mgr.contagion.recent_contagions.clone(),
            last_trigger: mgr.contagion.last_trigger.clone(),
            contagion_next_id: mgr.contagion.next_id,

            chaos_config: mgr.chaos.config.clone(),
            chaos_state: mgr.chaos.state.clone(),

            enabled: mgr.config.enabled,
            prompt_budget: mgr.config.prompt_budget,
        }
    }
}

impl SerializableIrrationalityManager {
    /// 还原为 IrrationalityManager / Restore into IrrationalityManager
    pub fn into_manager(self) -> crate::emotional_irrationality::IrrationalityManager {
        use crate::emotional_irrationality::{
            ChaosEngine, ContagionEngine, IrrationalityConfig, IrrationalityManager, PulseEngine,
            ResidueEngine,
        };

        // 重建脉冲引擎 / Reconstruct pulse engine
        let mut pulse = PulseEngine::new(self.pulse_config);
        pulse.active_pulses = self.active_pulses;
        pulse.shock_absorber = self.shock_absorber;
        pulse.next_id = self.pulse_next_id;

        // 重建残留引擎 / Reconstruct residue engine
        let mut residue = ResidueEngine::new(self.residue_config);
        residue.active_residues = self.active_residues;
        residue.next_id = self.residue_next_id;

        // 重建传染引擎 / Reconstruct contagion engine
        let mut contagion = ContagionEngine::new(self.contagion_config);
        contagion.rules = self.contagion_rules;
        contagion.recent_contagions = self.recent_contagions;
        contagion.last_trigger = self.last_trigger;
        contagion.next_id = self.contagion_next_id;

        // 重建混沌引擎 / Reconstruct chaos engine
        let chaos = ChaosEngine {
            config: self.chaos_config,
            state: self.chaos_state,
        };

        // 重建配置 / Reconstruct config
        let config = IrrationalityConfig {
            pulse: pulse.config.clone(),
            residue: residue.config.clone(),
            contagion: contagion.config.clone(),
            chaos: chaos.config.clone(),
            chaos_params: chaos.state.chaos_params,
            enabled: self.enabled,
            prompt_budget: self.prompt_budget,
        };

        // 从持久化部件重建，RNG 以 Stochastic 模式重新初始化
        // Reconstruct from persisted parts, RNG re-initialized in Stochastic mode
        IrrationalityManager::reconstruct(pulse, residue, contagion, chaos, config)
    }
}

// ════════════════════════════════════════════════════════════════════
// IrrationalityStore — sled 持久化存储 / Sled persistent store
// ════════════════════════════════════════════════════════════════════

/// 非理性存储 — 基于 sled 的情绪非理性模型持久化
///
/// Key: "manager" (单例)
/// Value: SerializableIrrationalityManager (bincode serialized)
///
/// 辅助 tree：脉冲索引 + 残留索引
pub struct IrrationalityStore {
    /// 主 tree：存储完整非理性管理器快照 / Main tree: full manager snapshot
    tree: sled::Tree,
    /// 脉冲索引 tree：pulse_id → pulse bincode / Pulse index tree
    pulse_tree: sled::Tree,
    /// 残留索引 tree：residue_id → residue bincode / Residue index tree
    residue_tree: sled::Tree,
}

impl IrrationalityStore {
    /// 打开或创建非理性存储 / Open or create irrationality store
    pub fn open(db: &sled::Db) -> Result<Self, IrrationalityStoreError> {
        let tree = db
            .open_tree("irrationality_manager")
            .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
        let pulse_tree = db
            .open_tree("irrationality_pulses")
            .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
        let residue_tree = db
            .open_tree("irrationality_residues")
            .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
        Ok(Self {
            tree,
            pulse_tree,
            residue_tree,
        })
    }

    /// 保存完整的非理性管理器 / Save full irrationality manager
    pub fn save(
        &self,
        mgr: &crate::emotional_irrationality::IrrationalityManager,
    ) -> Result<(), IrrationalityStoreError> {
        let snapshot = SerializableIrrationalityManager::from(mgr);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
        self.tree
            .insert(b"manager", value.as_slice())
            .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;

        // 同步更新脉冲索引 / Sync pulse index
        for pulse in &mgr.pulse.active_pulses {
            let key = pulse.id.to_be_bytes();
            let val = bincode::serialize(pulse)
                .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
            self.pulse_tree
                .insert(key, val.as_slice())
                .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
        }

        // 同步更新残留索引 / Sync residue index
        for residue in &mgr.residue.active_residues {
            let key = residue.id.to_be_bytes();
            let val = bincode::serialize(residue)
                .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
            self.residue_tree
                .insert(key, val.as_slice())
                .map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
        }

        Ok(())
    }

    /// 加载非理性管理器 / Load irrationality manager
    pub fn load(
        &self,
    ) -> Result<crate::emotional_irrationality::IrrationalityManager, IrrationalityStoreError> {
        match self.tree.get(b"manager") {
            Ok(Some(value)) => {
                let snapshot: SerializableIrrationalityManager = bincode::deserialize(&value)
                    .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
                Ok(snapshot.into_manager())
            }
            Ok(None) => Ok(crate::emotional_irrationality::IrrationalityManager::default()),
            Err(e) => Err(IrrationalityStoreError::SledError(e.to_string())),
        }
    }

    /// 获取单个脉冲 / Get a single pulse by ID
    pub fn get_pulse(
        &self,
        pulse_id: u64,
    ) -> Result<Option<ChaoticPulse>, IrrationalityStoreError> {
        let key = pulse_id.to_be_bytes();
        match self.pulse_tree.get(key) {
            Ok(Some(value)) => {
                let pulse: ChaoticPulse = bincode::deserialize(&value)
                    .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
                Ok(Some(pulse))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(IrrationalityStoreError::SledError(e.to_string())),
        }
    }

    /// 获取单个残留 / Get a single residue by ID
    pub fn get_residue(
        &self,
        residue_id: u64,
    ) -> Result<Option<EmotionResidue>, IrrationalityStoreError> {
        let key = residue_id.to_be_bytes();
        match self.residue_tree.get(key) {
            Ok(Some(value)) => {
                let residue: EmotionResidue = bincode::deserialize(&value)
                    .map_err(|e| IrrationalityStoreError::CodecError(e.to_string()))?;
                Ok(Some(residue))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(IrrationalityStoreError::SledError(e.to_string())),
        }
    }

    /// 获取所有脉冲 ID / Get all pulse IDs
    pub fn pulse_ids(&self) -> Result<Vec<u64>, IrrationalityStoreError> {
        let mut ids = Vec::new();
        for item in self.pulse_tree.iter() {
            let (key, _) = item.map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 获取所有残留 ID / Get all residue IDs
    pub fn residue_ids(&self) -> Result<Vec<u64>, IrrationalityStoreError> {
        let mut ids = Vec::new();
        for item in self.residue_tree.iter() {
            let (key, _) = item.map_err(|e| IrrationalityStoreError::SledError(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 脉冲总数 / Pulse count
    pub fn pulse_count(&self) -> usize {
        self.pulse_tree.len()
    }

    /// 残留总数 / Residue count
    pub fn residue_count(&self) -> usize {
        self.residue_tree.len()
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emotional_irrationality::{
        IrrationalityManager, PulseKind, PulseSource, PulseTrigger,
    };

    /// 测试用临时数据库 / Temporary test database
    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn test_store_save_and_load_default() {
        let db = test_db();
        let store = IrrationalityStore::open(&db).unwrap();
        let mgr = IrrationalityManager::default();
        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.pulse.active_pulses.is_empty());
        assert!(loaded.residue.active_residues.is_empty());
        assert!(loaded.contagion.recent_contagions.is_empty());
    }

    #[test]
    fn test_store_save_and_load_with_pulses() {
        let db = test_db();
        let store = IrrationalityStore::open(&db).unwrap();

        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "bad_news".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        // 触发一个脉冲 / Trigger a pulse
        let _ = mgr
            .pulse
            .detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);

        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(
            loaded.pulse.active_pulses.len(),
            mgr.pulse.active_pulses.len()
        );
        assert_eq!(loaded.pulse.next_id, mgr.pulse.next_id);
        if !mgr.pulse.active_pulses.is_empty() {
            assert_eq!(loaded.pulse.active_pulses[0].kind, PulseKind::Startle);
        }
    }

    #[test]
    fn test_store_save_and_load_with_residues() {
        let db = test_db();
        let store = IrrationalityStore::open(&db).unwrap();

        let mut mgr = IrrationalityManager::default();
        // 先触发脉冲，再从脉冲生成残留 / Trigger pulse then generate residue
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "sad_event".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        if let Some(pulse) = mgr
            .pulse
            .detect(&[0.0, 0.0, 0.0], &[-0.5, -0.3, -0.1], trigger, 1000)
        {
            mgr.residue.from_pulse(&pulse);
        }

        store.save(&mgr).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(
            loaded.residue.active_residues.len(),
            mgr.residue.active_residues.len()
        );
        assert_eq!(loaded.residue.next_id, mgr.residue.next_id);
    }

    #[test]
    fn test_store_pulse_index() {
        let db = test_db();
        let store = IrrationalityStore::open(&db).unwrap();

        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        let _ = mgr
            .pulse
            .detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);

        store.save(&mgr).unwrap();
        assert_eq!(store.pulse_count(), mgr.pulse.active_pulses.len());

        if let Some(first_pulse) = mgr.pulse.active_pulses.first() {
            let loaded = store.get_pulse(first_pulse.id).unwrap().unwrap();
            assert_eq!(loaded.kind, first_pulse.kind);
        }
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = IrrationalityStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.pulse.active_pulses.is_empty());
        assert!(loaded.residue.active_residues.is_empty());
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut mgr = IrrationalityManager::default();
        let trigger = PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        };
        if let Some(pulse) = mgr
            .pulse
            .detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000)
        {
            mgr.residue.from_pulse(&pulse);
        }

        let snapshot = SerializableIrrationalityManager::from(&mgr);
        let restored = snapshot.into_manager();

        assert_eq!(
            restored.pulse.active_pulses.len(),
            mgr.pulse.active_pulses.len()
        );
        assert_eq!(
            restored.residue.active_residues.len(),
            mgr.residue.active_residues.len()
        );
        assert_eq!(restored.pulse.next_id, mgr.pulse.next_id);
        assert_eq!(restored.residue.next_id, mgr.residue.next_id);
        assert_eq!(restored.contagion.next_id, mgr.contagion.next_id);
    }
}
