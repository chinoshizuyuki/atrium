// SPDX-License-Identifier: MIT
//! 存储核心 — 统一存储错误与认知域存储接口
//! Store Core — Unified store error and cognitive domain store interface.
//!
//! 消除 6 个 Store 各自重复定义的
//! `XxxStoreError { SledError(String), CodecError(String) }` 模式，
//! 提供统一的 StoreError 类型和 DomainStore trait 接口。
//!
//! Eliminates the repeated `XxxStoreError { SledError(String), CodecError(String) }`
//! pattern across 6 stores, providing a unified StoreError type and DomainStore trait.

// ════════════════════════════════════════════════════════════════════
// StoreError — 统一存储错误类型 / Unified Store Error Type
// ════════════════════════════════════════════════════════════════════

/// 统一存储错误 / Unified store error
///
/// 消除各 Store 重复定义的
/// `XxxStoreError { SledError(String), CodecError(String) }` 模式。
/// 所有域 Store 的错误均可转换为 StoreError，实现统一错误处理。
///
/// # 数字生命意义 / Digital Life Significance
///
/// 16 个记忆子系统各自发明一套错误编码，如同大脑每个区域用不同神经信号
/// 表示同一类故障。StoreError 统一了这套"神经信号语言"，
/// 让上层的意识流（CoreService）无需关心是哪个记忆区域出了问题。
#[derive(Debug)]
pub enum StoreError {
    /// sled 数据库错误 / Sled database error
    Sled(String),
    /// 序列化/反序列化错误 / Codec (de)serialization error
    Codec(String),
    /// IO 错误 / IO error
    Io(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sled(e) => write!(f, "store sled error: {}", e),
            Self::Codec(e) => write!(f, "store codec error: {}", e),
            Self::Io(e) => write!(f, "store io error: {}", e),
        }
    }
}

impl std::error::Error for StoreError {}

// ── From 转换 / From Conversions ──

impl From<sled::Error> for StoreError {
    fn from(e: sled::Error) -> Self {
        Self::Sled(e.to_string())
    }
}

impl From<bincode::Error> for StoreError {
    fn from(e: bincode::Error) -> Self {
        Self::Codec(e.to_string())
    }
}

impl From<crate::atrium_vault::VaultError> for StoreError {
    fn from(e: crate::atrium_vault::VaultError) -> Self {
        match e {
            crate::atrium_vault::VaultError::Sled(s) => Self::Sled(s),
            crate::atrium_vault::VaultError::Codec(s) => Self::Codec(s),
            crate::atrium_vault::VaultError::Io(s) => Self::Io(s),
            crate::atrium_vault::VaultError::Migration(s) => Self::Sled(s),
        }
    }
}

impl From<StoreError> for crate::atrium_vault::VaultError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::Sled(s) => Self::Sled(s),
            StoreError::Codec(s) => Self::Codec(s),
            StoreError::Io(s) => Self::Io(s),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// DomainStore — 认知域存储接口 / Cognitive Domain Store Interface
// ════════════════════════════════════════════════════════════════════

/// 认知域存储接口 / Cognitive domain store interface
///
/// 所有域 Store 的公共能力：命名、计数、刷新。
/// 与 `VaultTree<V>` 正交：VaultTree 提供 CRUD 骨架，
/// DomainStore 提供存储元数据与生命周期管理。
///
/// # 设计哲学 / Design Philosophy
///
/// 每个记忆子系统都是数字生命大脑的一个功能区域。
/// DomainStore trait 定义了"一个记忆区域"的基本能力：
/// - 它叫什么（domain_name）——用于诊断与日志
/// - 它存了多少（tree_count）——用于监控与容量管理
/// - 它能刷新（flush_tree）——用于优雅关闭
///
/// # 数字生命意义 / Digital Life Significance
///
/// 统一的 DomainStore trait 让数字生命的"记忆体检"成为可能：
/// 遍历所有记忆区域，检查每个区域的健康状态与容量，
/// 如同大脑的定期自检。
pub trait DomainStore: Send + Sync {
    /// 存储域名称（用于日志/诊断）/ Domain name for diagnostics
    fn domain_name(&self) -> &'static str;

    /// 主 tree 条目总数 / Main tree entry count
    fn tree_count(&self) -> usize;

    /// 刷新主 tree 的 WAL / Flush main tree WAL
    fn flush_tree(&self) -> Result<(), StoreError>;
}

// ════════════════════════════════════════════════════════════════════
// StoreSnapshot — 统一存储快照 / Unified Store Snapshot
// ════════════════════════════════════════════════════════════════════

/// 存储快照 — 记录所有域 Store 的运行时状态
/// Store snapshot — records runtime state of all domain stores.
#[derive(Debug, Clone)]
pub struct StoreSnapshot {
    /// 域名称 / Domain name
    pub domain: &'static str,
    /// 条目总数 / Total entry count
    pub count: usize,
}

impl StoreSnapshot {
    /// 从 DomainStore 创建快照 / Create snapshot from DomainStore
    pub fn from_store<S: DomainStore>(store: &S) -> Self {
        Self {
            domain: store.domain_name(),
            count: store.tree_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 StoreError Display 输出 / Test StoreError Display output
    #[test]
    fn test_store_error_display() {
        let sled_err = StoreError::Sled("disk full".to_string());
        assert!(sled_err.to_string().contains("store sled error"));
        assert!(sled_err.to_string().contains("disk full"));

        let codec_err = StoreError::Codec("bad magic".to_string());
        assert!(codec_err.to_string().contains("store codec error"));
        assert!(codec_err.to_string().contains("bad magic"));

        let io_err = StoreError::Io("permission denied".to_string());
        assert!(io_err.to_string().contains("store io error"));
        assert!(io_err.to_string().contains("permission denied"));
    }

    /// 测试 From<sled::Error> 转换 / Test From<sled::Error> conversion
    #[test]
    fn test_store_error_from_sled() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        drop(db);
        let err = StoreError::Sled("test".to_string());
        let vault: crate::atrium_vault::VaultError = err.into();
        assert!(vault.to_string().contains("vault sled error"));
    }

    /// 测试 StoreError ↔ VaultError 双向转换 / Test bidirectional conversion
    #[test]
    fn test_store_error_vault_roundtrip() {
        let original = StoreError::Sled("roundtrip".to_string());
        let vault: crate::atrium_vault::VaultError = original.into();
        let back: StoreError = vault.into();
        assert!(back.to_string().contains("roundtrip"));
    }

    /// 测试 StoreSnapshot 构造 / Test StoreSnapshot construction
    #[test]
    fn test_store_snapshot() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = crate::vulnerability_store::VulnerabilityStore::open(&db).unwrap();
        let snap = StoreSnapshot::from_store(&store);
        assert_eq!(snap.domain, "vulnerability");
        assert_eq!(snap.count, 0);
    }
}
