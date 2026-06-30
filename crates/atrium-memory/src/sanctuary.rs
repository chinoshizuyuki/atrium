// SPDX-License-Identifier: MIT
//! Config Sanctuary — 三层配置权限
//!
//! L1: Mutable — AI 可修改 (插件开关/偏好/定时任务/ACK导入)
//! L2: Protected — AI 只读 (核心架构/记忆结构/人格防御/身份标识)
//! L3: MetaLock — AI 不可见不可写 (权限白名单/签名公钥)
//! ConfigSanctuary — Configuration permission authority.
//!
//! L1: Mutable — AI can modify (greeting style/preference/timeout/ACK parameters)
//! L2: Protected — AI read-only (core architecture/module structure/persona core/identity)
//! L3: MetaLock — AI cannot read or write (permission system root/signing keys)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 配置变更请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub key: String,
    pub value: String,
    pub source: String, // "ai" | "user" | "system"
}

/// 权限检查结果
#[derive(Debug, Clone)]
pub struct PermissionResult {
    pub allowed: bool,
    pub reason: String,
}

/// 配置圣所
pub struct ConfigSanctuary {
    /// AI 可修改的白名单路径
    mutable_paths: HashSet<String>,
    /// AI 只读路径
    protected_paths: HashSet<String>,
}

impl Default for ConfigSanctuary {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSanctuary {
    pub fn new() -> Self {
        let mut s = Self {
            mutable_paths: HashSet::new(),
            protected_paths: HashSet::new(),
        };
        s.register_defaults();
        s
    }

    fn register_defaults(&mut self) {
        // AI 可修改
        for path in &[
            "plugins.enabled",
            "preferences",
            "cron_jobs",
            "ack.imports",
            "persona.name",
        ] {
            self.mutable_paths.insert(path.to_string());
        }
        // AI 只读
        for path in &[
            "core.architecture",
            "memory.structure",
            "persona.defense",
            "identity.id",
            "system.runtime",
            "bridge.config",
        ] {
            self.protected_paths.insert(path.to_string());
        }
    }

    /// 检查 AI 是否有权限修改某配置项
    pub fn check(&self, key: &str, source: &str) -> PermissionResult {
        // 用户/系统 永远有权限
        if source == "user" || source == "system" {
            return PermissionResult {
                allowed: true,
                reason: "用户/系统操作".into(),
            };
        }

        // AI 操作需要白名单检查
        if self.mutable_paths.contains(key) {
            return PermissionResult {
                allowed: true,
                reason: "在AI可修改白名单中".into(),
            };
        }

        if self.protected_paths.contains(key) {
            return PermissionResult {
                allowed: false,
                reason: format!("{} 属于保护层配置，AI不可修改", key),
            };
        }

        // 默认：不在白名单则拒绝
        PermissionResult {
            allowed: false,
            reason: format!("{} 不在AI可修改白名单中", key),
        }
    }

    /// 尝试应用 AI 发起的配置变更
    pub fn apply_ai_change(&self, change: &ConfigChange) -> PermissionResult {
        self.check(&change.key, &change.source)
    }

    /// 获取 AI 可见的配置视图（过滤保护层）
    pub fn visible_to_ai(&self, all_keys: &[String]) -> Vec<String> {
        all_keys
            .iter()
            .filter(|k| !self.protected_paths.contains(k.as_str()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_can_modify_plugins() {
        let s = ConfigSanctuary::new();
        let r = s.check("plugins.enabled", "ai");
        assert!(r.allowed);
    }

    #[test]
    fn test_ai_cannot_modify_core() {
        let s = ConfigSanctuary::new();
        let r = s.check("core.architecture", "ai");
        assert!(!r.allowed);
    }

    #[test]
    fn test_user_always_allowed() {
        let s = ConfigSanctuary::new();
        let r = s.check("core.architecture", "user");
        assert!(r.allowed);
    }

    #[test]
    fn test_visible_to_ai_filters_protected() {
        let s = ConfigSanctuary::new();
        let keys = vec![
            "plugins.enabled".into(),
            "core.architecture".into(),
            "preferences".into(),
        ];
        let visible = s.visible_to_ai(&keys);
        assert!(visible.contains(&"plugins.enabled".to_string()));
        assert!(!visible.contains(&"core.architecture".to_string()));
    }
}
