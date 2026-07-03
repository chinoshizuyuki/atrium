// SPDX-License-Identifier: MIT
//! 风格记忆 — 用户级风格偏移的 sled 持久化
//! StyleMemory — Per-user style offset persisted via sled.
//!
//! 每个用户对 Atrium 的表达风格有偏好偏移。
//! 通过用户反馈（点赞/修正）学习偏移向量，持久化到 sled。
//! 下次对话时，StyleEmbedding + 用户偏移 → 个性化风格。

use serde::{Deserialize, Serialize};

use crate::style_modulator::{StyleEmbedding, STYLE_DIM};

// ════════════════════════════════════════════════════════════════════
// StyleOffset — 用户风格偏移向量
// ════════════════════════════════════════════════════════════════════

/// 用户风格偏移向量 — 128维，与 StyleEmbedding 同空间
///
/// 通过用户反馈学习：
/// - 用户点赞某回复 → 偏移向该回复的风格方向微调
/// - 用户修正某回复 → 偏移远离该回复的风格方向
/// - 偏移量随反馈次数累积，但有上限防止发散
#[derive(Clone, Debug)]
pub struct StyleOffset {
    /// 偏移向量
    pub offset: [f32; STYLE_DIM],
    /// 累计反馈次数
    pub feedback_count: u32,
    /// 最后更新时间（unix timestamp）
    pub last_updated: u64,
}

// serde 默认不支持 [T; 128]，手动实现序列化（通过 Vec 中转）
impl Serialize for StyleOffset {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("StyleOffset", 3)?;
        s.serialize_field("offset", &self.offset.to_vec())?;
        s.serialize_field("feedback_count", &self.feedback_count)?;
        s.serialize_field("last_updated", &self.last_updated)?;
        s.end()
    }
}
impl<'de> Deserialize<'de> for StyleOffset {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            offset: Vec<f32>,
            feedback_count: u32,
            last_updated: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        let mut arr = [0.0f32; STYLE_DIM];
        let len = h.offset.len().min(STYLE_DIM);
        arr[..len].copy_from_slice(&h.offset[..len]);
        Ok(StyleOffset {
            offset: arr,
            feedback_count: h.feedback_count,
            last_updated: h.last_updated,
        })
    }
}

impl StyleOffset {
    /// 零偏移
    pub fn zero() -> Self {
        Self {
            offset: [0.0; STYLE_DIM],
            feedback_count: 0,
            last_updated: 0,
        }
    }

    /// 从 StyleEmbedding 差值创建偏移
    pub fn from_diff(current: &StyleEmbedding, preferred: &StyleEmbedding) -> Self {
        let mut offset = [0.0f32; STYLE_DIM];
        for (o, (p, c)) in offset
            .iter_mut()
            .zip(preferred.0.iter().zip(current.0.iter()))
        {
            *o = p - c;
        }
        Self {
            offset,
            feedback_count: 1,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// 应用正向反馈（点赞）— 偏移向目标方向微调
    ///
    /// learning_rate 控制每次反馈的步长。
    /// 使用衰减学习率：随反馈次数增加，步长减小。
    pub fn apply_positive_feedback(
        &mut self,
        target_style: &StyleEmbedding,
        current_style: &StyleEmbedding,
    ) {
        let learning_rate = self.decayed_learning_rate();
        for i in 0..STYLE_DIM {
            let diff = target_style.0[i] - current_style.0[i];
            self.offset[i] += diff * learning_rate;
            // 裁剪到 [-1.0, 1.0] 防止发散
            self.offset[i] = self.offset[i].clamp(-1.0, 1.0);
        }
        self.feedback_count += 1;
        self.update_timestamp();
    }

    /// 应用负向反馈（修正）— 偏移远离该风格方向
    pub fn apply_negative_feedback(
        &mut self,
        rejected_style: &StyleEmbedding,
        current_style: &StyleEmbedding,
    ) {
        let learning_rate = self.decayed_learning_rate();
        for i in 0..STYLE_DIM {
            let diff = rejected_style.0[i] - current_style.0[i];
            // 远离 rejected 方向
            self.offset[i] -= diff * learning_rate * 0.5; // 负反馈步长更小
            self.offset[i] = self.offset[i].clamp(-1.0, 1.0);
        }
        self.feedback_count += 1;
        self.update_timestamp();
    }

    /// 衰减学习率 — 随反馈次数增加而减小
    fn decayed_learning_rate(&self) -> f32 {
        // 初始 0.1，衰减到 0.01
        0.1 / (1.0 + self.feedback_count as f32 * 0.1)
    }

    /// 应用偏移到 StyleEmbedding
    ///
    /// 最终风格 = 基础风格 + 用户偏移 × 权重
    pub fn apply_to(&self, style: &StyleEmbedding, weight: f32) -> StyleEmbedding {
        let mut result = [0.0f32; STYLE_DIM];
        for (r, (s, o)) in result
            .iter_mut()
            .zip(style.0.iter().zip(self.offset.iter()))
        {
            *r = s + o * weight;
        }
        StyleEmbedding(result)
    }

    /// 偏移范数 — 衡量偏移强度
    pub fn norm(&self) -> f32 {
        let sum: f32 = (0..STYLE_DIM)
            .map(|i| self.offset[i] * self.offset[i])
            .sum();
        sum.sqrt()
    }

    fn update_timestamp(&mut self) {
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

// ════════════════════════════════════════════════════════════════════
// StyleMemoryStore — sled 持久化存储
// ════════════════════════════════════════════════════════════════════

/// 风格记忆存储 — 基于 sled 的用户级风格偏移持久化
///
/// Key: user_id (String)
/// Value: StyleOffset (bincode serialized)
pub struct StyleMemoryStore {
    tree: sled::Tree,
}

impl StyleMemoryStore {
    /// 打开或创建风格记忆存储
    pub fn open(db: &sled::Db) -> Result<Self, StyleMemoryError> {
        let tree = db
            .open_tree("style_memory")
            .map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
        Ok(Self { tree })
    }

    /// 获取用户的风格偏移
    pub fn get(&self, user_id: &str) -> Result<StyleOffset, StyleMemoryError> {
        let key = user_id.as_bytes();
        match self.tree.get(key) {
            Ok(Some(value)) => bincode::deserialize(&value)
                .map_err(|e| StyleMemoryError::DeserializeError(e.to_string())),
            Ok(None) => Ok(StyleOffset::zero()),
            Err(e) => Err(StyleMemoryError::SledError(e.to_string())),
        }
    }

    /// 保存用户的风格偏移
    pub fn set(&self, user_id: &str, offset: &StyleOffset) -> Result<(), StyleMemoryError> {
        let key = user_id.as_bytes();
        let value = bincode::serialize(offset)
            .map_err(|e| StyleMemoryError::SerializeError(e.to_string()))?;
        self.tree
            .insert(key, value)
            .map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
        // sled 自动 flush，但可以显式 flush 确保持久化
        self.tree
            .flush()
            .map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
        Ok(())
    }

    /// 应用正向反馈并保存
    pub fn apply_positive_and_save(
        &self,
        user_id: &str,
        target_style: &StyleEmbedding,
        current_style: &StyleEmbedding,
    ) -> Result<StyleOffset, StyleMemoryError> {
        let mut offset = self.get(user_id)?;
        offset.apply_positive_feedback(target_style, current_style);
        self.set(user_id, &offset)?;
        Ok(offset)
    }

    /// 应用负向反馈并保存
    pub fn apply_negative_and_save(
        &self,
        user_id: &str,
        rejected_style: &StyleEmbedding,
        current_style: &StyleEmbedding,
    ) -> Result<StyleOffset, StyleMemoryError> {
        let mut offset = self.get(user_id)?;
        offset.apply_negative_feedback(rejected_style, current_style);
        self.set(user_id, &offset)?;
        Ok(offset)
    }

    /// 删除用户的风格偏移（重置）
    pub fn remove(&self, user_id: &str) -> Result<(), StyleMemoryError> {
        let key = user_id.as_bytes();
        self.tree
            .remove(key)
            .map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
        self.tree
            .flush()
            .map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
        Ok(())
    }

    /// 列出所有有风格偏移的用户
    pub fn list_users(&self) -> Result<Vec<String>, StyleMemoryError> {
        let mut users = Vec::new();
        for item in self.tree.iter() {
            let (key, _) = item.map_err(|e| StyleMemoryError::SledError(e.to_string()))?;
            let user_id = String::from_utf8(key.to_vec())
                .map_err(|e| StyleMemoryError::DeserializeError(e.to_string()))?;
            users.push(user_id);
        }
        Ok(users)
    }

    /// 获取存储中的用户数量
    pub fn count(&self) -> Result<usize, StyleMemoryError> {
        Ok(self.tree.len())
    }
}

// ════════════════════════════════════════════════════════════════════
// 错误类型
// ════════════════════════════════════════════════════════════════════

/// 风格记忆错误
#[derive(Debug)]
pub enum StyleMemoryError {
    /// sled 存储错误
    SledError(String),
    /// 序列化错误
    SerializeError(String),
    /// 反序列化错误
    DeserializeError(String),
}

impl std::fmt::Display for StyleMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StyleMemoryError::SledError(e) => write!(f, "sled error: {}", e),
            StyleMemoryError::SerializeError(e) => write!(f, "serialize error: {}", e),
            StyleMemoryError::DeserializeError(e) => write!(f, "deserialize error: {}", e),
        }
    }
}

impl std::error::Error for StyleMemoryError {}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_offset_zero() {
        let offset = StyleOffset::zero();
        assert_eq!(offset.norm(), 0.0);
        assert_eq!(offset.feedback_count, 0);
    }

    #[test]
    fn test_style_offset_from_diff() {
        let a = StyleEmbedding::zero();
        let mut b_data = [0.0f32; STYLE_DIM];
        b_data[0] = 1.0;
        let b = StyleEmbedding(b_data);

        let offset = StyleOffset::from_diff(&a, &b);
        assert!((offset.offset[0] - 1.0).abs() < 1e-6);
        assert_eq!(offset.feedback_count, 1);
    }

    #[test]
    fn test_style_offset_positive_feedback() {
        let current = StyleEmbedding::zero();
        let mut target_data = [0.0f32; STYLE_DIM];
        target_data[0] = 0.5;
        let target = StyleEmbedding(target_data);

        let mut offset = StyleOffset::zero();
        offset.apply_positive_feedback(&target, &current);

        // 偏移应向目标方向移动
        assert!(
            offset.offset[0] > 0.0,
            "positive feedback should move toward target"
        );
        assert_eq!(offset.feedback_count, 1);
    }

    #[test]
    fn test_style_offset_negative_feedback() {
        let current = StyleEmbedding::zero();
        let mut rejected_data = [0.0f32; STYLE_DIM];
        rejected_data[0] = 0.5;
        let rejected = StyleEmbedding(rejected_data);

        let mut offset = StyleOffset::zero();
        offset.apply_negative_feedback(&rejected, &current);

        // 偏移应远离 rejected 方向
        assert!(
            offset.offset[0] < 0.0,
            "negative feedback should move away from rejected"
        );
        assert_eq!(offset.feedback_count, 1);
    }

    #[test]
    fn test_style_offset_decayed_learning_rate() {
        let mut offset = StyleOffset::zero();
        let lr0 = offset.decayed_learning_rate();
        assert!(
            (lr0 - 0.1).abs() < 1e-6,
            "initial learning rate should be 0.1"
        );

        offset.feedback_count = 10;
        let lr10 = offset.decayed_learning_rate();
        assert!(lr10 < lr0, "learning rate should decay");

        offset.feedback_count = 100;
        let lr100 = offset.decayed_learning_rate();
        assert!(lr100 < lr10, "learning rate should continue decaying");
    }

    #[test]
    fn test_style_offset_apply_to() {
        let mut offset = StyleOffset::zero();
        offset.offset[0] = 0.1;

        let style = StyleEmbedding::zero();
        let adjusted = offset.apply_to(&style, 1.0);
        assert!(
            (adjusted.0[0] - 0.1).abs() < 1e-6,
            "offset should be applied"
        );

        let adjusted_half = offset.apply_to(&style, 0.5);
        assert!(
            (adjusted_half.0[0] - 0.05).abs() < 1e-6,
            "offset weight should scale"
        );
    }

    #[test]
    fn test_style_offset_clamp() {
        let current = StyleEmbedding::zero();
        let mut target_data = [0.0f32; STYLE_DIM];
        target_data[0] = 100.0; // 极端值
        let target = StyleEmbedding(target_data);

        let mut offset = StyleOffset::zero();
        // 多次反馈后偏移应被裁剪
        for _ in 0..100 {
            offset.apply_positive_feedback(&target, &current);
        }
        // 偏移不应超过 1.0
        for i in 0..STYLE_DIM {
            assert!(
                offset.offset[i].abs() <= 1.0,
                "offset should be clamped to [-1, 1], got {} at dim {}",
                offset.offset[i],
                i
            );
        }
    }

    #[test]
    fn test_style_memory_store_crud() {
        // 使用临时目录
        let dir = tempfile::Builder::new()
            .prefix("style_memory_test")
            .tempdir()
            .unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = StyleMemoryStore::open(&db).unwrap();

        // 初始应为零偏移
        let offset = store.get("user1").unwrap();
        assert_eq!(offset.norm(), 0.0);

        // 保存偏移
        let mut new_offset = StyleOffset::zero();
        new_offset.offset[0] = 0.5;
        new_offset.feedback_count = 3;
        store.set("user1", &new_offset).unwrap();

        // 读取
        let loaded = store.get("user1").unwrap();
        assert!((loaded.offset[0] - 0.5).abs() < 1e-6);
        assert_eq!(loaded.feedback_count, 3);

        // 删除
        store.remove("user1").unwrap();
        let after_remove = store.get("user1").unwrap();
        assert_eq!(after_remove.norm(), 0.0);
    }

    #[test]
    fn test_style_memory_store_apply_positive() {
        let dir = tempfile::Builder::new()
            .prefix("style_memory_test_pos")
            .tempdir()
            .unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = StyleMemoryStore::open(&db).unwrap();

        let current = StyleEmbedding::zero();
        let mut target_data = [0.0f32; STYLE_DIM];
        target_data[0] = 0.5;
        let target = StyleEmbedding(target_data);

        let offset = store
            .apply_positive_and_save("user1", &target, &current)
            .unwrap();
        assert!(offset.offset[0] > 0.0);
        assert_eq!(offset.feedback_count, 1);

        // 再次读取应一致
        let loaded = store.get("user1").unwrap();
        assert!((loaded.offset[0] - offset.offset[0]).abs() < 1e-6);
    }

    #[test]
    fn test_style_memory_store_list_users() {
        let dir = tempfile::Builder::new()
            .prefix("style_memory_test_list")
            .tempdir()
            .unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = StyleMemoryStore::open(&db).unwrap();

        let offset = StyleOffset::zero();
        store.set("user1", &offset).unwrap();
        store.set("user2", &offset).unwrap();

        let users = store.list_users().unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.contains(&"user1".to_string()));
        assert!(users.contains(&"user2".to_string()));

        assert_eq!(store.count().unwrap(), 2);
    }
}
