// SPDX-License-Identifier: MIT
//! 物理存在感存储 — PhysicalPresenceEngine 的 sled 持久化
//! PhysicalPresenceStore — PhysicalPresenceEngine model persisted via sled.
//!
//! 将物理存在感引擎状态（体感快照 + 体感签名）持久化到 sled，
//! 消除重启失忆缺陷，支持跨会话的体感连续性。
//! 体感存储归入 limbic_db（情感中枢）——体感是情感的躯体映射。
//!
//! Persist the physical presence engine state (somatic snapshot + body signature)
//! to sled, eliminating restart amnesia and supporting cross-session somatic continuity.
//! Physical presence storage belongs to limbic_db — body sense is the somatic mapping of emotion.

use serde::{Deserialize, Serialize};

use crate::emotional_irrationality::BodyMemory;
use crate::physical_presence::{
    BodySignature, EnvironmentChannels, PhysicalPresenceConfig, PhysicalState,
    PhysiologicalChannels,
};

// ════════════════════════════════════════════════════════════════════
// PhysicalPresenceStoreError — 存储错误类型 / Storage Error Type
// ════════════════════════════════════════════════════════════════════

/// 物理存在感存储错误 / Physical presence store error
#[derive(Debug)]
pub enum PhysicalPresenceStoreError {
    /// sled 数据库错误 / Sled database error
    SledError(String),
    /// 序列化/反序列化错误 / Codec (de)serialization error
    CodecError(String),
}

impl std::fmt::Display for PhysicalPresenceStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SledError(e) => write!(f, "physical_presence sled error: {}", e),
            Self::CodecError(e) => write!(f, "physical_presence codec error: {}", e),
        }
    }
}

impl std::error::Error for PhysicalPresenceStoreError {}

// ════════════════════════════════════════════════════════════════════
// SerializablePhysicalPresence — 可序列化的引擎快照
// Serializable PhysicalPresence snapshot for sled bincode persistence
// ════════════════════════════════════════════════════════════════════

/// 可序列化的物理存在感快照 — 用于 sled bincode 持久化
///
/// PhysicalPresenceEngine 的 1:1 镜像，确保 bincode 稳定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializablePhysicalPresence {
    // ── 体感状态 / Physical state ──
    /// 基础体感通道 / Base somatic channels
    pub body_breath_offset: f64,
    pub body_tension: f64,
    pub body_heaviness: f64,
    pub body_warmth: f64,
    /// 生理通道 / Physiological channels
    pub physio_fatigue: f64,
    pub physio_hunger: f64,
    pub physio_drowsiness: f64,
    pub physio_discomfort: f64,
    /// 环境感知通道 / Environment channels
    pub env_temperature_perception: f64,
    pub env_posture: f64,
    /// 最后更新时间戳 / Last update timestamp
    pub updated_at: i64,

    // ── 体感签名 / Body signature ──
    pub sig_baseline_tension: f64,
    pub sig_baseline_warmth: f64,
    pub sig_fatigue_proneness: f64,
    pub sig_signature_label: String,

    // ── 配置 / Config ──
    pub config_enabled: bool,
    pub config_fatigue_half_life_secs: f64,
    pub config_circadian_enabled: bool,
    pub config_interaction_fatigue_enabled: bool,
    pub config_body_to_emotion_enabled: bool,
    pub config_prompt_budget: usize,
    pub config_signature_ema_alpha: f64,

    // ── 引擎内部状态 / Engine internal state ──
    pub last_tick_at: i64,
}

impl From<&crate::physical_presence::PhysicalPresenceEngine> for SerializablePhysicalPresence {
    fn from(engine: &crate::physical_presence::PhysicalPresenceEngine) -> Self {
        Self {
            // 体感状态 / Physical state
            body_breath_offset: engine.state.body.breath_offset,
            body_tension: engine.state.body.tension,
            body_heaviness: engine.state.body.heaviness,
            body_warmth: engine.state.body.warmth,
            physio_fatigue: engine.state.physiological.fatigue,
            physio_hunger: engine.state.physiological.hunger,
            physio_drowsiness: engine.state.physiological.drowsiness,
            physio_discomfort: engine.state.physiological.discomfort,
            env_temperature_perception: engine.state.environment.temperature_perception,
            env_posture: engine.state.environment.posture,
            updated_at: engine.state.updated_at,

            // 体感签名 / Body signature
            sig_baseline_tension: engine.signature.baseline_tension,
            sig_baseline_warmth: engine.signature.baseline_warmth,
            sig_fatigue_proneness: engine.signature.fatigue_proneness,
            sig_signature_label: engine.signature.signature_label.clone(),

            // 配置 / Config
            config_enabled: engine.config.enabled,
            config_fatigue_half_life_secs: engine.config.fatigue_half_life_secs,
            config_circadian_enabled: engine.config.circadian_enabled,
            config_interaction_fatigue_enabled: engine.config.interaction_fatigue_enabled,
            config_body_to_emotion_enabled: engine.config.body_to_emotion_enabled,
            config_prompt_budget: engine.config.prompt_budget,
            config_signature_ema_alpha: engine.config.signature_ema_alpha,

            // 引擎内部状态 / Engine internal state
            last_tick_at: engine.last_tick_at,
        }
    }
}

impl SerializablePhysicalPresence {
    /// 还原为 PhysicalPresenceEngine / Restore into PhysicalPresenceEngine
    pub fn into_engine(self) -> crate::physical_presence::PhysicalPresenceEngine {
        let state = PhysicalState {
            body: BodyMemory {
                breath_offset: self.body_breath_offset,
                tension: self.body_tension,
                heaviness: self.body_heaviness,
                warmth: self.body_warmth,
            },
            physiological: PhysiologicalChannels {
                fatigue: self.physio_fatigue,
                hunger: self.physio_hunger,
                drowsiness: self.physio_drowsiness,
                discomfort: self.physio_discomfort,
            },
            environment: EnvironmentChannels {
                temperature_perception: self.env_temperature_perception,
                posture: self.env_posture,
            },
            updated_at: self.updated_at,
        };

        let signature = BodySignature {
            baseline_tension: self.sig_baseline_tension,
            baseline_warmth: self.sig_baseline_warmth,
            fatigue_proneness: self.sig_fatigue_proneness,
            signature_label: self.sig_signature_label,
        };

        let config = PhysicalPresenceConfig {
            enabled: self.config_enabled,
            fatigue_half_life_secs: self.config_fatigue_half_life_secs,
            circadian_enabled: self.config_circadian_enabled,
            interaction_fatigue_enabled: self.config_interaction_fatigue_enabled,
            body_to_emotion_enabled: self.config_body_to_emotion_enabled,
            prompt_budget: self.config_prompt_budget,
            signature_ema_alpha: self.config_signature_ema_alpha,
        };

        crate::physical_presence::PhysicalPresenceEngine {
            state,
            signature,
            config,
            last_tick_at: self.last_tick_at,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// PhysicalPresenceStore — sled 持久化存储 / Sled persistent store
// ════════════════════════════════════════════════════════════════════

/// 物理存在感存储 — 基于 sled 的体感模型持久化
///
/// Key: "engine" (单例)
/// Value: SerializablePhysicalPresence (bincode serialized)
///
/// 归属认知域: Limbic（情感中枢）— 体感是情感的躯体映射
pub struct PhysicalPresenceStore {
    /// 主 tree：存储完整引擎快照 / Main tree: full engine snapshot
    tree: sled::Tree,
    /// 签名 tree：独立存储体感签名（高频读取）/ Signature tree (high-read)
    signature_tree: sled::Tree,
}

impl PhysicalPresenceStore {
    /// 打开或创建物理存在感存储 / Open or create physical presence store
    pub fn open(db: &sled::Db) -> Result<Self, PhysicalPresenceStoreError> {
        let tree = db
            .open_tree("physical_presence_engine")
            .map_err(|e| PhysicalPresenceStoreError::SledError(e.to_string()))?;
        let signature_tree = db
            .open_tree("physical_presence_signature")
            .map_err(|e| PhysicalPresenceStoreError::SledError(e.to_string()))?;
        Ok(Self {
            tree,
            signature_tree,
        })
    }

    /// 保存完整的物理存在感引擎 / Save full physical presence engine
    pub fn save(
        &self,
        engine: &crate::physical_presence::PhysicalPresenceEngine,
    ) -> Result<(), PhysicalPresenceStoreError> {
        let snapshot = SerializablePhysicalPresence::from(engine);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| PhysicalPresenceStoreError::CodecError(e.to_string()))?;
        self.tree
            .insert(b"engine", value.as_slice())
            .map_err(|e| PhysicalPresenceStoreError::SledError(e.to_string()))?;

        // 同步更新签名索引 / Sync signature index
        let sig_value = bincode::serialize(&engine.signature)
            .map_err(|e| PhysicalPresenceStoreError::CodecError(e.to_string()))?;
        self.signature_tree
            .insert(b"current", sig_value.as_slice())
            .map_err(|e| PhysicalPresenceStoreError::SledError(e.to_string()))?;

        Ok(())
    }

    /// 加载物理存在感引擎 / Load physical presence engine
    pub fn load(
        &self,
    ) -> Result<crate::physical_presence::PhysicalPresenceEngine, PhysicalPresenceStoreError> {
        match self.tree.get(b"engine") {
            Ok(Some(value)) => {
                let snapshot: SerializablePhysicalPresence = bincode::deserialize(&value)
                    .map_err(|e| PhysicalPresenceStoreError::CodecError(e.to_string()))?;
                Ok(snapshot.into_engine())
            }
            Ok(None) => Ok(crate::physical_presence::PhysicalPresenceEngine::default()),
            Err(e) => Err(PhysicalPresenceStoreError::SledError(e.to_string())),
        }
    }

    /// 获取体感签名 / Get body signature (high-read path)
    pub fn get_signature(&self) -> Result<Option<BodySignature>, PhysicalPresenceStoreError> {
        match self.signature_tree.get(b"current") {
            Ok(Some(value)) => {
                let sig: BodySignature = bincode::deserialize(&value)
                    .map_err(|e| PhysicalPresenceStoreError::CodecError(e.to_string()))?;
                Ok(Some(sig))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(PhysicalPresenceStoreError::SledError(e.to_string())),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_presence::PhysicalPresenceEngine;

    /// 测试用临时数据库 / Temporary test database
    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn test_store_save_and_load_default() {
        let db = test_db();
        let store = PhysicalPresenceStore::open(&db).unwrap();
        let engine = PhysicalPresenceEngine::default();
        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.state.magnitude(), 0.0);
        assert_eq!(loaded.signature.signature_label, "未成形");
    }

    #[test]
    fn test_store_save_and_load_with_state() {
        let db = test_db();
        let store = PhysicalPresenceStore::open(&db).unwrap();

        let mut engine = PhysicalPresenceEngine::default();
        engine.state.body.tension = 0.5;
        engine.state.physiological.fatigue = 0.6;
        engine.state.environment.temperature_perception = -0.3;
        engine.signature.signature_label = "容易紧张型".to_string();

        store.save(&engine).unwrap();
        let loaded = store.load().unwrap();

        assert!((loaded.state.body.tension - 0.5).abs() < 1e-10);
        assert!((loaded.state.physiological.fatigue - 0.6).abs() < 1e-10);
        assert!((loaded.state.environment.temperature_perception - (-0.3)).abs() < 1e-10);
        assert_eq!(loaded.signature.signature_label, "容易紧张型");
    }

    #[test]
    fn test_store_get_signature() {
        let db = test_db();
        let store = PhysicalPresenceStore::open(&db).unwrap();

        let mut engine = PhysicalPresenceEngine::default();
        engine.signature.baseline_tension = 0.5;
        engine.signature.signature_label = "容易紧张型".to_string();

        store.save(&engine).unwrap();
        let sig = store.get_signature().unwrap().unwrap();
        assert!((sig.baseline_tension - 0.5).abs() < 1e-10);
        assert_eq!(sig.signature_label, "容易紧张型");
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = PhysicalPresenceStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.state.magnitude(), 0.0);
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut engine = PhysicalPresenceEngine::default();
        engine.state.body.tension = 0.7;
        engine.state.physiological.fatigue = 0.4;
        engine.state.environment.posture = -0.5;
        engine.signature.baseline_warmth = 0.3;
        engine.signature.signature_label = "温暖放松型".to_string();

        let snapshot = SerializablePhysicalPresence::from(&engine);
        let restored = snapshot.into_engine();

        assert!((restored.state.body.tension - 0.7).abs() < 1e-10);
        assert!((restored.state.physiological.fatigue - 0.4).abs() < 1e-10);
        assert!((restored.state.environment.posture - (-0.5)).abs() < 1e-10);
        assert!((restored.signature.baseline_warmth - 0.3).abs() < 1e-10);
        assert_eq!(restored.signature.signature_label, "温暖放松型");
    }
}
