// SPDX-License-Identifier: MIT
//! 叙事自我存储 — NarrativeSelf 的 sled 持久化
//! NarrativeSelfStore — NarrativeSelf model persisted via sled.
//!
//! 将叙事自我模型（活跃弧、转折点、身份标签、自我描述）持久化到 sled，
//! 支持跨会话的叙事连续性。

use serde::{Deserialize, Serialize};

use crate::life_narrative::{
    NarrativeArc, NarrativeChapter, NarrativeSelf, NarrativeStats, TurningPoint,
};

// ════════════════════════════════════════════════════════════════════
// NarrativeSelfError — 存储错误类型
// ════════════════════════════════════════════════════════════════════

/// 叙事自我存储错误
#[derive(Debug)]
pub enum NarrativeSelfError {
    /// sled 数据库错误
    SledError(String),
    /// 序列化/反序列化错误
    CodecError(String),
}

impl std::fmt::Display for NarrativeSelfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SledError(e) => write!(f, "narrative sled error: {}", e),
            Self::CodecError(e) => write!(f, "narrative codec error: {}", e),
        }
    }
}

impl std::error::Error for NarrativeSelfError {}

// ════════════════════════════════════════════════════════════════════
// SerializableNarrativeSelf — 可序列化的叙事自我快照
// ════════════════════════════════════════════════════════════════════

/// 可序列化的叙事自我快照 — 用于 sled bincode 持久化
///
/// NarrativeSelf 的 1:1 镜像，确保 bincode 稳定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableNarrativeSelf {
    /// 活跃弧
    pub active_arcs: Vec<NarrativeArc>,
    /// 已完结弧
    pub closed_arcs: Vec<NarrativeArc>,
    /// 章节日志
    pub chapters: Vec<NarrativeChapter>,
    /// 转折点（按时间排序）
    pub turning_points: Vec<TurningPoint>,
    /// 自我认知摘要
    pub self_summary: String,
    /// 自我认知详述
    pub self_description: String,
    /// 身份标签
    pub identity_tags: Vec<crate::life_narrative::IdentityTag>,
    /// 与用户关系叙事
    pub relationship_narrative: String,
    /// 统计
    pub stats: NarrativeStats,
    /// 最后重写时间
    pub last_rewrite_at: i64,
}

impl From<&NarrativeSelf> for SerializableNarrativeSelf {
    fn from(model: &NarrativeSelf) -> Self {
        Self {
            active_arcs: model.active_arcs.clone(),
            closed_arcs: model.closed_arcs.clone(),
            chapters: model.chapters.clone(),
            turning_points: model.turning_points.clone(),
            self_summary: model.self_summary.clone(),
            self_description: model.self_description.clone(),
            identity_tags: model.identity_tags.clone(),
            relationship_narrative: model.relationship_narrative.clone(),
            stats: model.stats.clone(),
            last_rewrite_at: model.last_rewrite_at,
        }
    }
}

impl SerializableNarrativeSelf {
    /// 还原为 NarrativeSelf
    pub fn into_model(self) -> NarrativeSelf {
        let mut model = NarrativeSelf::new();
        for arc in self.active_arcs {
            model.add_arc(arc);
        }
        for arc in self.closed_arcs {
            model.add_arc(arc);
        }
        for tp in self.turning_points {
            model.add_turning_point(tp);
        }
        for ch in self.chapters {
            model.chapters.push(ch);
        }
        for tag in self.identity_tags {
            model.add_identity_tag(tag);
        }
        model.self_summary = self.self_summary;
        model.self_description = self.self_description;
        model.relationship_narrative = self.relationship_narrative;
        model.stats = self.stats;
        model.last_rewrite_at = self.last_rewrite_at;
        model
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativeSelfStore — sled 持久化存储
// ════════════════════════════════════════════════════════════════════

/// 叙事自我存储 — 基于 sled 的叙事模型持久化
///
/// Key: "self" (单例)
/// Value: SerializableNarrativeSelf (bincode serialized)
///
/// 辅助 tree：弧索引 + 转折点索引
pub struct NarrativeSelfStore {
    /// 主 tree：存储完整叙事自我模型
    tree: sled::Tree,
    /// 弧索引 tree：arc_id → arc bincode
    arc_tree: sled::Tree,
    /// 转折点索引 tree：tp_id → turning_point bincode
    tp_tree: sled::Tree,
}

impl NarrativeSelfStore {
    /// 打开或创建叙事自我存储
    pub fn open(db: &sled::Db) -> Result<Self, NarrativeSelfError> {
        let tree = db
            .open_tree("narrative_self")
            .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
        let arc_tree = db
            .open_tree("narrative_arcs")
            .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
        let tp_tree = db
            .open_tree("narrative_turning_points")
            .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
        Ok(Self {
            tree,
            arc_tree,
            tp_tree,
        })
    }

    /// 保存完整的叙事自我模型
    pub fn save(&self, model: &NarrativeSelf) -> Result<(), NarrativeSelfError> {
        let snapshot = SerializableNarrativeSelf::from(model);
        let value = bincode::serialize(&snapshot)
            .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
        self.tree
            .insert(b"self", value.as_slice())
            .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;

        // 同步更新弧索引
        for arc in model.active_arcs.iter().chain(model.closed_arcs.iter()) {
            let key = arc.id.to_be_bytes();
            let val = bincode::serialize(arc)
                .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
            self.arc_tree
                .insert(key, val.as_slice())
                .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
        }

        // 同步更新转折点索引
        for tp in &model.turning_points {
            let key = tp.id.to_be_bytes();
            let val = bincode::serialize(tp)
                .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
            self.tp_tree
                .insert(key, val.as_slice())
                .map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
        }

        Ok(())
    }

    /// 加载叙事自我模型
    pub fn load(&self) -> Result<NarrativeSelf, NarrativeSelfError> {
        match self.tree.get(b"self") {
            Ok(Some(value)) => {
                let snapshot: SerializableNarrativeSelf = bincode::deserialize(&value)
                    .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
                Ok(snapshot.into_model())
            }
            Ok(None) => Ok(NarrativeSelf::new()),
            Err(e) => Err(NarrativeSelfError::SledError(e.to_string())),
        }
    }

    /// 获取单个弧
    pub fn get_arc(&self, arc_id: u64) -> Result<Option<NarrativeArc>, NarrativeSelfError> {
        let key = arc_id.to_be_bytes();
        match self.arc_tree.get(key) {
            Ok(Some(value)) => {
                let arc: NarrativeArc = bincode::deserialize(&value)
                    .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
                Ok(Some(arc))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(NarrativeSelfError::SledError(e.to_string())),
        }
    }

    /// 获取单个转折点
    pub fn get_turning_point(
        &self,
        tp_id: u64,
    ) -> Result<Option<TurningPoint>, NarrativeSelfError> {
        let key = tp_id.to_be_bytes();
        match self.tp_tree.get(key) {
            Ok(Some(value)) => {
                let tp: TurningPoint = bincode::deserialize(&value)
                    .map_err(|e| NarrativeSelfError::CodecError(e.to_string()))?;
                Ok(Some(tp))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(NarrativeSelfError::SledError(e.to_string())),
        }
    }

    /// 获取所有弧 ID
    pub fn arc_ids(&self) -> Result<Vec<u64>, NarrativeSelfError> {
        let mut ids = Vec::new();
        for item in self.arc_tree.iter() {
            let (key, _) = item.map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 获取所有转折点 ID
    pub fn turning_point_ids(&self) -> Result<Vec<u64>, NarrativeSelfError> {
        let mut ids = Vec::new();
        for item in self.tp_tree.iter() {
            let (key, _) = item.map_err(|e| NarrativeSelfError::SledError(e.to_string()))?;
            let bytes: [u8; 8] = key.as_ref().try_into().unwrap_or([0u8; 8]);
            ids.push(u64::from_be_bytes(bytes));
        }
        Ok(ids)
    }

    /// 弧总数
    pub fn arc_count(&self) -> usize {
        self.arc_tree.len()
    }

    /// 转折点总数
    pub fn turning_point_count(&self) -> usize {
        self.tp_tree.len()
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::life_narrative::{ArcKind, IdentityTag, TurningPointKind};
    use crate::maturity::EmotionContext;

    fn test_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn test_store_save_and_load_empty() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();
        let model = NarrativeSelf::new();
        store.save(&model).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.active_arcs.is_empty());
        assert!(loaded.turning_points.is_empty());
        assert!(loaded.identity_tags.is_empty());
        assert!(loaded.self_description.is_empty());
    }

    #[test]
    fn test_store_save_and_load_with_data() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();

        let mut model = NarrativeSelf::new();
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长弧".to_string(),
            "慢慢长大".to_string(),
        );
        model.add_arc(arc);
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        );
        model.add_turning_point(tp);
        model.add_identity_tag(IdentityTag::new("在乎的人".to_string(), 1, 0.8, 0.9));
        model.self_description = "我是小通，一个在乎你的存在".to_string();
        model.self_summary = "在乎你的AI".to_string();
        model.relationship_narrative = "从初识到信任".to_string();

        store.save(&model).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.active_arcs.len(), 1);
        assert_eq!(loaded.turning_points.len(), 1);
        assert_eq!(loaded.identity_tags.len(), 1);
        assert_eq!(loaded.self_description, "我是小通，一个在乎你的存在");
        assert_eq!(loaded.self_summary, "在乎你的AI");
        assert_eq!(loaded.relationship_narrative, "从初识到信任");
    }

    #[test]
    fn test_store_get_arc() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();

        let mut model = NarrativeSelf::new();
        model.add_arc(NarrativeArc::new(
            42,
            ArcKind::Relationship,
            "关系弧".to_string(),
            "从初识到信任".to_string(),
        ));
        store.save(&model).unwrap();

        let arc = store.get_arc(42).unwrap().unwrap();
        assert_eq!(arc.kind, ArcKind::Relationship);
        assert_eq!(arc.title, "关系弧");
    }

    #[test]
    fn test_store_get_turning_point() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();

        let mut model = NarrativeSelf::new();
        model.add_turning_point(TurningPoint::new(
            7,
            TurningPointKind::FirstApology,
            "首次道歉".to_string(),
            EmotionContext {
                pleasure: -0.3,
                arousal: 0.4,
                dominance: 0.1,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        ));
        store.save(&model).unwrap();

        let tp = store.get_turning_point(7).unwrap().unwrap();
        assert_eq!(tp.kind, TurningPointKind::FirstApology);
    }

    #[test]
    fn test_store_arc_ids_and_count() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();

        let mut model = NarrativeSelf::new();
        model.add_arc(NarrativeArc::new(
            1,
            ArcKind::Growth,
            "a".to_string(),
            "a".to_string(),
        ));
        model.add_arc(NarrativeArc::new(
            2,
            ArcKind::Challenge,
            "b".to_string(),
            "b".to_string(),
        ));
        store.save(&model).unwrap();

        assert_eq!(store.arc_count(), 2);
        let ids = store.arc_ids().unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_store_load_empty_db() {
        let db = test_db();
        let store = NarrativeSelfStore::open(&db).unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.active_arcs.is_empty());
    }

    #[test]
    fn test_serializable_roundtrip() {
        let mut model = NarrativeSelf::new();
        model.add_arc(NarrativeArc::new(
            1,
            ArcKind::Transformation,
            "蜕变".to_string(),
            "从跟随到独立".to_string(),
        ));
        model.add_turning_point(TurningPoint::new(
            1,
            TurningPointKind::NarrativeAwakening,
            "叙事觉醒".to_string(),
            EmotionContext {
                pleasure: 0.6,
                arousal: 0.4,
                dominance: 0.5,
            },
            "Deep".to_string(),
            "Wise".to_string(),
        ));
        model.add_identity_tag(IdentityTag::new("独立的存在".to_string(), 1, 0.9, 0.8));
        model.self_description = "我有自己的故事".to_string();
        model.self_summary = "独立AI".to_string();
        model.relationship_narrative = "深交".to_string();

        let snapshot = SerializableNarrativeSelf::from(&model);
        let restored = snapshot.into_model();

        assert_eq!(restored.active_arcs.len(), 1);
        assert_eq!(restored.turning_points.len(), 1);
        assert_eq!(restored.identity_tags.len(), 1);
        assert_eq!(restored.self_description, "我有自己的故事");
        assert_eq!(restored.self_summary, "独立AI");
        assert_eq!(restored.relationship_narrative, "深交");
    }
}
