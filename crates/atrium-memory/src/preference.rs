// SPDX-License-Identifier: MIT
//! 偏好学习系统 — 4 层置信度衰减 + sled 持久化
//! Preference learning system — 4-tier confidence decay + sled persistence.
//!
//! 层次（从高到低）/ Tiers (high to low):
//! L0: ExplicitDeclaration — 用户明确说"我喜欢X" / user explicitly says "I like X" (base=0.90)
//! L1: BehaviorInference — 行为推断"主人连续3天用Rust" / behavioral inference (base=0.70)
//! L2: LLMExtraction — LLM 从对话中提取 / LLM extraction from conversation (base=0.55)
//! L3: SystemDefault — 系统预设默认 / system preset default (base=0.20)
//!
//! 置信度衰减 / Confidence decay: sigmoid(time_since_last_confirm / half_life)
//! 反复确认 → 衰减变慢; 不再提及 → 逐渐遗忘
//! Repeated confirmations → slower decay; no longer mentioned → gradual forgetting

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 偏好来源层次
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PreferenceLayer {
    ExplicitDeclaration = 0,
    BehaviorInference = 1,
    LLMExtraction = 2,
    SystemDefault = 3,
}

impl PreferenceLayer {
    pub fn base_confidence(&self) -> f64 {
        match self {
            Self::ExplicitDeclaration => 0.90,
            Self::BehaviorInference => 0.70,
            Self::LLMExtraction => 0.55,
            Self::SystemDefault => 0.20,
        }
    }
}

/// 单条偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub key: String,
    pub value: String,
    pub layer: PreferenceLayer,
    pub confidence: f64,
    pub first_seen: u64,
    pub last_confirmed: u64,
    pub confirm_count: u32,
}

impl Preference {
    pub fn new(key: &str, value: &str, layer: PreferenceLayer) -> Self {
        let now = now_secs();
        Self {
            key: key.to_string(),
            value: value.to_string(),
            confidence: layer.base_confidence(),
            layer,
            first_seen: now,
            last_confirmed: now,
            confirm_count: 1,
        }
    }

    pub fn confirm(&mut self) {
        self.confirm_count += 1;
        self.last_confirmed = now_secs();
        self.confidence += (1.0 - self.confidence) * 0.25;
        if self.confirm_count == 5 && self.layer as u8 > 0 {
            self.layer = match self.layer {
                PreferenceLayer::LLMExtraction => PreferenceLayer::BehaviorInference,
                PreferenceLayer::BehaviorInference => PreferenceLayer::ExplicitDeclaration,
                _ => self.layer,
            };
            self.confidence = self.confidence.max(self.layer.base_confidence());
        }
    }

    pub fn effective_confidence(&self) -> f64 {
        let now = now_secs();
        let elapsed = now.saturating_sub(self.last_confirmed) as f64;
        let half_life = self.half_life_seconds();
        if half_life == 0.0 {
            return self.confidence;
        }
        let k = 5.0 / half_life;
        let decay = 1.0 / (1.0 + (k * (elapsed - half_life)).exp());
        (self.confidence * decay).max(0.0)
    }

    fn half_life_seconds(&self) -> f64 {
        match self.layer {
            PreferenceLayer::ExplicitDeclaration => 30.0 * 24.0 * 3600.0,
            PreferenceLayer::BehaviorInference => 14.0 * 24.0 * 3600.0,
            PreferenceLayer::LLMExtraction => 7.0 * 24.0 * 3600.0,
            PreferenceLayer::SystemDefault => 3.0 * 24.0 * 3600.0,
        }
    }

    pub fn is_active(&self, threshold: f64) -> bool {
        self.effective_confidence() > threshold
    }
}

/// 偏好管理器 — sled 持久化
pub struct PreferenceManager {
    prefs: Vec<Preference>,
    db: Option<sled::Db>,
}

impl Default for PreferenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferenceManager {
    pub fn new() -> Self {
        Self {
            prefs: Vec::new(),
            db: None,
        }
    }

    pub fn open(db_path: &str) -> Self {
        let db = sled::open(db_path).ok();
        let mut prefs = Vec::new();
        if let Some(ref db) = db {
            for item in db.iter().flatten() {
                let (_, value) = item;
                if let Ok(pref) = bincode::deserialize::<Preference>(&value) {
                    prefs.push(pref);
                }
            }
            tracing::info!("PreferenceManager: loaded {} preferences", prefs.len());
        }
        Self { prefs, db }
    }

    pub fn new_in_memory() -> Self {
        Self {
            prefs: Vec::new(),
            db: None,
        }
    }

    /// 增量持久化：只写入指定 key 的偏好（替代全量重写）
    fn persist_key(&self, key: &str) {
        if let Some(ref db) = self.db {
            let matching: Vec<&Preference> = self.prefs.iter().filter(|p| p.key == key).collect();
            // 先删除该 key 的旧条目
            let prefix = format!("pref_{}_", key);
            for db_key in db.scan_prefix(prefix.as_bytes()).keys().flatten() {
                let _ = db.remove(db_key);
            }
            // 写入当前条目
            for (i, pref) in matching.iter().enumerate() {
                if let Ok(data) = bincode::serialize(pref) {
                    let _ = db.insert(format!("pref_{}_{}", key, i).as_bytes(), data);
                }
            }
            let _ = db.flush();
        }
    }

    /// 全量持久化（仅启动时调用，或 prune/delete 后调用）
    fn persist_all(&self) {
        if let Some(ref db) = self.db {
            for key in db.iter().keys().flatten() {
                let _ = db.remove(key);
            }
            for (i, pref) in self.prefs.iter().enumerate() {
                if let Ok(data) = bincode::serialize(pref) {
                    let _ = db.insert(format!("pref_{}", i).as_bytes(), data);
                }
            }
            let _ = db.flush();
        }
    }

    /// 添加或确认偏好
    pub fn upsert(&mut self, key: &str, value: &str, layer: PreferenceLayer) {
        if let Some(existing) = self
            .prefs
            .iter_mut()
            .find(|p| p.key == key && p.value == value)
        {
            existing.confirm();
            if existing.layer as u8 > layer as u8 {
                existing.layer = layer;
            }
        } else {
            self.prefs.push(Preference::new(key, value, layer));
        }
        self.persist_key(key);
    }

    /// 查询最高置信度偏好的值
    pub fn get(&self, key: &str, threshold: f64) -> Option<&Preference> {
        self.prefs
            .iter()
            .filter(|p| p.key == key && p.is_active(threshold))
            .max_by(|a, b| {
                a.effective_confidence()
                    .partial_cmp(&b.effective_confidence())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// 获取所有活跃偏好
    pub fn active(&self, threshold: f64) -> Vec<&Preference> {
        self.prefs
            .iter()
            .filter(|p| p.is_active(threshold))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.prefs.len()
    }

    /// 构建 LLM Prompt 上下文片段 / Build LLM Prompt context fragment.
    ///
    /// 格式:
    /// ```text
    /// [用户偏好]
    /// - lang: Rust (置信度 0.90, L0)
    /// - food: 火锅 (置信度 0.72, L1)
    /// ```
    pub fn build_prompt_context(&self, threshold: f64, max_items: usize) -> String {
        let mut active = self.active(threshold);
        // 按 effective_confidence 降序排列
        active.sort_by(|a, b| {
            b.effective_confidence()
                .partial_cmp(&a.effective_confidence())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        active.truncate(max_items);

        if active.is_empty() {
            return String::new();
        }

        let mut out = String::from("[用户偏好]\n");
        for p in &active {
            let layer_label = match p.layer {
                PreferenceLayer::ExplicitDeclaration => "L0",
                PreferenceLayer::BehaviorInference => "L1",
                PreferenceLayer::LLMExtraction => "L2",
                PreferenceLayer::SystemDefault => "L3",
            };
            use std::fmt::Write;
            let _ = writeln!(
                out,
                "- {}: {} (置信度 {:.2}, {})",
                p.key,
                p.value,
                p.effective_confidence(),
                layer_label
            );
        }
        out
    }

    /// 清理过期偏好：移除 effective_confidence 低于 min_confidence
    /// 且超过 max_age_days 天未确认的条目
    pub fn prune(&mut self, min_confidence: f64, max_age_days: u64) -> usize {
        let now = now_secs();
        let max_age_secs = max_age_days * 86400;
        let before = self.prefs.len();
        self.prefs.retain(|p| {
            let eff = p.effective_confidence();
            let age = now.saturating_sub(p.last_confirmed);
            // 保留条件: 置信度足够 OR 还不够老
            eff >= min_confidence || age < max_age_secs
        });
        let removed = before - self.prefs.len();
        if removed > 0 {
            self.persist_all();
            tracing::info!("PreferenceManager: pruned {} expired preferences", removed);
        }
        removed
    }

    /// 删除指定 key+value 的偏好
    pub fn remove(&mut self, key: &str, value: &str) -> bool {
        let before = self.prefs.len();
        self.prefs.retain(|p| !(p.key == key && p.value == value));
        let removed = self.prefs.len() < before;
        if removed {
            self.persist_all();
        }
        removed
    }

    /// 删除指定 key 的所有偏好
    pub fn remove_key(&mut self, key: &str) -> usize {
        let before = self.prefs.len();
        self.prefs.retain(|p| p.key != key);
        let removed = before - self.prefs.len();
        if removed > 0 {
            self.persist_all();
        }
        removed
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_base_confidence() {
        assert!(
            PreferenceLayer::ExplicitDeclaration.base_confidence()
                > PreferenceLayer::BehaviorInference.base_confidence()
        );
        assert!(
            PreferenceLayer::BehaviorInference.base_confidence()
                > PreferenceLayer::LLMExtraction.base_confidence()
        );
    }

    #[test]
    fn test_confirm_increases_confidence() {
        let mut p = Preference::new("lang", "Rust", PreferenceLayer::LLMExtraction);
        let before = p.confidence;
        p.confirm();
        assert!(p.confidence > before);
    }

    #[test]
    fn test_effective_confidence_decays() {
        let mut p = Preference::new("lang", "Rust", PreferenceLayer::SystemDefault);
        // 模拟很久之前确认的
        p.last_confirmed = 0;
        let eff = p.effective_confidence();
        assert!(
            eff < p.confidence,
            "应随时间衰减: eff={} < conf={}",
            eff,
            p.confidence
        );
    }

    #[test]
    fn test_manager_upsert_and_get() {
        let mut mgr = PreferenceManager::new();
        mgr.upsert("lang", "Rust", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("lang", "Python", PreferenceLayer::LLMExtraction);
        let best = mgr.get("lang", 0.3).unwrap();
        assert_eq!(best.value, "Rust"); // ExplicitDeclaration 优先级更高
    }

    #[test]
    fn test_layer_promotion() {
        let mut p = Preference::new("lang", "Go", PreferenceLayer::LLMExtraction);
        assert_eq!(p.layer, PreferenceLayer::LLMExtraction);
        for _ in 0..5 {
            p.confirm();
        }
        assert_eq!(p.layer, PreferenceLayer::BehaviorInference); // 晋升
    }

    #[test]
    fn test_build_prompt_context_empty() {
        let mgr = PreferenceManager::new();
        let ctx = mgr.build_prompt_context(0.1, 10);
        assert!(ctx.is_empty(), "无偏好时应返回空字符串");
    }

    #[test]
    fn test_build_prompt_context_with_items() {
        let mut mgr = PreferenceManager::new();
        mgr.upsert("lang", "Rust", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("food", "火锅", PreferenceLayer::BehaviorInference);
        let ctx = mgr.build_prompt_context(0.1, 10);
        assert!(ctx.contains("[用户偏好]"));
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("火锅"));
    }

    #[test]
    fn test_build_prompt_context_max_items() {
        let mut mgr = PreferenceManager::new();
        mgr.upsert("a", "1", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("b", "2", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("c", "3", PreferenceLayer::ExplicitDeclaration);
        let ctx = mgr.build_prompt_context(0.1, 2);
        // 只应包含 2 条
        let line_count = ctx.lines().filter(|l| l.starts_with("- ")).count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn test_prune_removes_expired() {
        let mut mgr = PreferenceManager::new();
        let mut old_pref = Preference::new("old", "data", PreferenceLayer::SystemDefault);
        old_pref.last_confirmed = 0; // 很久以前确认
        old_pref.confidence = 0.01; // 极低置信度
        mgr.prefs.push(old_pref);
        mgr.upsert("fresh", "data", PreferenceLayer::ExplicitDeclaration);

        let removed = mgr.prune(0.1, 1); // 阈值 0.1, 超过 1 天
        assert_eq!(removed, 1);
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_remove_specific() {
        let mut mgr = PreferenceManager::new();
        mgr.upsert("lang", "Rust", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("lang", "Python", PreferenceLayer::LLMExtraction);
        assert!(mgr.remove("lang", "Rust"));
        assert_eq!(mgr.count(), 1);
        assert!(mgr.get("lang", 0.1).unwrap().value == "Python");
    }

    #[test]
    fn test_remove_key() {
        let mut mgr = PreferenceManager::new();
        mgr.upsert("lang", "Rust", PreferenceLayer::ExplicitDeclaration);
        mgr.upsert("lang", "Python", PreferenceLayer::LLMExtraction);
        mgr.upsert("food", "火锅", PreferenceLayer::BehaviorInference);
        let removed = mgr.remove_key("lang");
        assert_eq!(removed, 2);
        assert_eq!(mgr.count(), 1);
    }
}
