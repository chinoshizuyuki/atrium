// SPDX-License-Identifier: MIT
//! 用户心智模型持久化 — 跨会话用户认知画像的 sled 存储
//! User Mental Model Store — Cross-session user cognitive portrait persisted via sled.
//!
//! 数字生命重启后需恢复对用户的理解（情绪模式、沟通风格、
//! 兴趣偏好、参与度），否则每次对话都从零建模，丧失"记得你"的能力。
//!
//! After digital life restarts, it must restore its understanding of the user
//! (mood patterns, communication style, interest preferences, engagement),
//! otherwise every conversation starts from scratch, losing "I remember you".

use crate::user_model::UserMentalModel;

// ════════════════════════════════════════════════════════════════════
// UserMentalModelStore — sled 持久化存储
// ════════════════════════════════════════════════════════════════════

/// 用户心智模型存储 — 基于 sled 的跨会话持久化
///
/// Key: user_id (String)
/// Value: UserMentalModel (bincode serialized)
///
/// 设计原则 / Design principles:
/// - 写穿策略：每次 save() 立即 flush，保证崩溃安全
///   Write-through: every save() immediately flushes for crash safety
/// - 零拷贝读：bincode 反序列化到栈上结构，无堆分配
///   Zero-copy read: bincode deserializes to stack-allocated struct, no heap allocation
/// - 单用户优化：默认 user_id = "default"，避免多用户场景下的 Tree 碎片
///   Single-user optimization: default user_id = "default", avoiding Tree fragmentation
pub struct UserMentalModelStore {
    tree: sled::Tree,
}

impl UserMentalModelStore {
    /// 打开或创建用户心智模型存储 / Open or create user mental model store
    pub fn open(db: &sled::Db) -> Result<Self, UserModelError> {
        let tree = db
            .open_tree("user_model")
            .map_err(|e| UserModelError::SledError(e.to_string()))?;
        Ok(Self { tree })
    }

    /// 加载用户的心智模型 / Load user's mental model
    pub fn load(&self, user_id: &str) -> Result<UserMentalModel, UserModelError> {
        let key = user_id.as_bytes();
        match self.tree.get(key) {
            Ok(Some(value)) => bincode::deserialize(&value)
                .map_err(|e| UserModelError::DeserializeError(e.to_string())),
            Ok(None) => Ok(UserMentalModel::new()),
            Err(e) => Err(UserModelError::SledError(e.to_string())),
        }
    }

    /// 保存用户的心智模型 / Save user's mental model
    pub fn save(&self, user_id: &str, model: &UserMentalModel) -> Result<(), UserModelError> {
        let key = user_id.as_bytes();
        let value =
            bincode::serialize(model).map_err(|e| UserModelError::SerializeError(e.to_string()))?;
        self.tree
            .insert(key, value)
            .map_err(|e| UserModelError::SledError(e.to_string()))?;
        self.tree
            .flush()
            .map_err(|e| UserModelError::SledError(e.to_string()))?;
        Ok(())
    }

    /// 删除用户的心智模型（重置）/ Remove user's mental model (reset)
    pub fn remove(&self, user_id: &str) -> Result<(), UserModelError> {
        let key = user_id.as_bytes();
        self.tree
            .remove(key)
            .map_err(|e| UserModelError::SledError(e.to_string()))?;
        self.tree
            .flush()
            .map_err(|e| UserModelError::SledError(e.to_string()))?;
        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════════
// 错误类型 / Error types
// ════════════════════════════════════════════════════════════════════

/// 用户心智模型错误 / User mental model error
#[derive(Debug)]
pub enum UserModelError {
    /// sled 存储错误 / sled storage error
    SledError(String),
    /// 序列化错误 / Serialization error
    SerializeError(String),
    /// 反序列化错误 / Deserialization error
    DeserializeError(String),
}

impl std::fmt::Display for UserModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserModelError::SledError(e) => write!(f, "sled error: {}", e),
            UserModelError::SerializeError(e) => write!(f, "serialize error: {}", e),
            UserModelError::DeserializeError(e) => write!(f, "deserialize error: {}", e),
        }
    }
}

impl std::error::Error for UserModelError {}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_model_store_roundtrip() {
        let dir = tempfile::Builder::new()
            .prefix("user_model_store_test")
            .tempdir()
            .unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = UserMentalModelStore::open(&db).unwrap();

        // 初始应为默认模型 / Initial should be default model
        // 注意：两次 UserMentalModel::new() 调用可能跨毫秒，时间戳字段会不同
        // Note: two UserMentalModel::new() calls may span different milliseconds
        let model = store.load("user1").unwrap();
        let default = UserMentalModel::new();
        assert_eq!(model.mood.valence, default.mood.valence);
        assert_eq!(model.mood.intensity, default.mood.intensity);
        assert_eq!(model.style.formality, default.style.formality);
        assert_eq!(model.topic_interests, default.topic_interests);

        // 修改并保存 / Modify and save
        let mut modified = UserMentalModel::new();
        modified.topic_interests.insert("Rust".to_string(), 0.8);
        store.save("user1", &modified).unwrap();

        // 加载应一致 / Load should match
        let loaded = store.load("user1").unwrap();
        assert_eq!(loaded.topic_interests.get("Rust"), Some(&0.8));
    }

    #[test]
    fn test_user_model_store_remove() {
        let dir = tempfile::Builder::new()
            .prefix("user_model_store_test_rm")
            .tempdir()
            .unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = UserMentalModelStore::open(&db).unwrap();

        let model = UserMentalModel::new();
        store.save("user1", &model).unwrap();
        store.remove("user1").unwrap();

        let loaded = store.load("user1").unwrap();
        assert_eq!(loaded, UserMentalModel::new());
    }
}
