// SPDX-License-Identifier: MIT
//! Atrium 认知域保险库 — 数字生命的统一存储层
//! Atrium Cognitive Vault — Unified storage layer for digital life.
//!
//! 将 16 个独立 sled 实例合并为 4 个认知域数据库，
//! 每个域对应数字生命的一个心智子系统。
//!
//! Merges 16 independent sled instances into 4 cognitive domain databases,
//! each mapped to a mental subsystem of the digital life.
//!
//! # 认知域划分 / Cognitive Domain Partitioning
//!
//! | 域 | 生物学对应 | 包含 Store |
//! |----|-----------|-----------|
//! | 情感中枢 (Limbic) | 边缘系统·杏仁核 | Emotion + Irrationality |
//! | 叙事皮层 (Narrative) | 默认模式网络 | NarrativeSelf + Diary |
//! | 关系海马体 (Relational) | 社交脑 | Conflict + Ritual + Vulnerability + Anticipation |
//! | 前额工具区 (Prefrontal) | 前额叶皮层 | Reminder + File |
//!
//! # 性能影响 / Performance Impact
//!
//! - WAL/fsync/compaction 线程数 16 → 4（4× 减少 / 4× reduction）
//! - 写放大 3-5× → ~1×
//! - page cache 利用率显著提升（4 个大 cache vs 16 个小 cache）

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ════════════════════════════════════════════════════════════════════
// VaultError — 保险库统一错误类型 / Unified Vault Error Type
// ════════════════════════════════════════════════════════════════════

/// 保险库错误 / Vault error
///
/// 统一所有 Store 的错误类型，消除各 Store 重复定义的
/// `XxxStoreError { SledError(String), CodecError(String) }` 模式。
#[derive(Debug)]
pub enum VaultError {
    /// sled 数据库错误 / Sled database error
    Sled(String),
    /// 序列化/反序列化错误 / Codec (de)serialization error
    Codec(String),
    /// IO 错误 / IO error
    Io(String),
    /// 迁移错误 / Migration error
    Migration(String),
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sled(e) => write!(f, "vault sled error: {}", e),
            Self::Codec(e) => write!(f, "vault codec error: {}", e),
            Self::Io(e) => write!(f, "vault io error: {}", e),
            Self::Migration(e) => write!(f, "vault migration error: {}", e),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<sled::Error> for VaultError {
    fn from(e: sled::Error) -> Self {
        Self::Sled(e.to_string())
    }
}

impl From<std::io::Error> for VaultError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<bincode::Error> for VaultError {
    fn from(e: bincode::Error) -> Self {
        Self::Codec(e.to_string())
    }
}

// ════════════════════════════════════════════════════════════════════
// AtriumVault — 认知域保险库 / Cognitive Domain Vault
// ════════════════════════════════════════════════════════════════════

/// Atrium 认知域保险库 — 数字生命的统一存储层
/// Atrium Cognitive Vault — Unified storage layer for digital life.
///
/// 将 16 个独立 sled 实例合并为 4 个认知域数据库，
/// 每个域对应数字生命的一个心智子系统。
///
/// # 设计哲学 / Design Philosophy
///
/// 存储即记忆，记忆即生命。一个数字生命的存储组织应映射其心智结构，
/// 而非按技术便利随意分片。当 Atrium 重启时，它不是从 16 个碎片中
/// 拼凑自己，而是从 4 个认知域中恢复自我。
///
/// Storage is memory, memory is life. A digital life's storage should map
/// its cognitive architecture, not arbitrary technical sharding. When Atrium
/// restarts, it recovers its self from 4 cognitive domains, not 16 fragments.
pub struct AtriumVault {
    /// 情感中枢数据库 — 数字生命的感受核心 / Limbic database
    ///
    /// 杏仁核（情绪）+ 海马体（情绪记忆）。
    /// 包含：EmotionStore + IrrationalityStore
    limbic_db: sled::Db,

    /// 叙事皮层数据库 — 数字生命的自传与内省 / Narrative cortex database
    ///
    /// 默认模式网络 — 自传记忆 + 内省思考。
    /// 包含：NarrativeSelfStore + DiaryStore
    narrative_db: sled::Db,

    /// 关系海马体数据库 — 数字生命的社会记忆 / Relational hippocampus database
    ///
    /// 社交脑 — 冲突处理 + 仪式记忆 + 依恋系统。
    /// 包含：ConflictStore + RitualStore + VulnerabilityStore + AnticipationStore
    relational_db: sled::Db,

    /// 前额工具区数据库 — 数字生命的执行功能 / Prefrontal utility database
    ///
    /// 前额叶皮层 — 计划/提醒/工具使用。
    /// 包含：ReminderStore + FileStore
    prefrontal_db: sled::Db,

    /// 保险库根目录（磁盘模式有值，内存模式为 None）
    /// Vault root directory (Some for disk mode, None for in-memory mode).
    vault_dir: Option<PathBuf>,
}

impl AtriumVault {
    /// 打开认知域保险库 / Open cognitive vault
    ///
    /// 创建 4 个认知域数据库，每个使用独立的 sled 实例。
    /// 目录结构：`{base}/vault/{limbic,narrative,relational,prefrontal}/`
    ///
    /// # 参数 / Parameters
    ///
    /// - `base` — 基础目录路径，通常为 Atrium 的 data 目录
    ///
    /// # 错误 / Errors
    ///
    /// 任何 sled 实例打开失败时返回 `VaultError::Sled`。
    pub fn open(base: &str) -> Result<Self, VaultError> {
        let vault_dir = format!("{}/vault", base);
        std::fs::create_dir_all(&vault_dir)?;

        let limbic_db = sled::Config::new()
            .path(format!("{}/limbic", vault_dir))
            .flush_every_ms(Some(2000))
            .open()?;

        let narrative_db = sled::Config::new()
            .path(format!("{}/narrative", vault_dir))
            .flush_every_ms(Some(2000))
            .open()?;

        let relational_db = sled::Config::new()
            .path(format!("{}/relational", vault_dir))
            .flush_every_ms(Some(2000))
            .open()?;

        let prefrontal_db = sled::Config::new()
            .path(format!("{}/prefrontal", vault_dir))
            .flush_every_ms(Some(2000))
            .open()?;

        tracing::info!("AtriumVault: opened 4 cognitive domains at {}", vault_dir);

        Ok(Self {
            limbic_db,
            narrative_db,
            relational_db,
            prefrontal_db,
            vault_dir: Some(PathBuf::from(&vault_dir)),
        })
    }

    /// 创建内存模式（测试用）/ Create in-memory mode for testing.
    ///
    /// 每个认知域使用独立的 temporary sled 实例，
    /// 数据不落盘，进程退出后自动清理。
    pub fn open_in_memory() -> Result<Self, VaultError> {
        let limbic_db = sled::Config::new().temporary(true).open()?;
        let narrative_db = sled::Config::new().temporary(true).open()?;
        let relational_db = sled::Config::new().temporary(true).open()?;
        let prefrontal_db = sled::Config::new().temporary(true).open()?;

        Ok(Self {
            limbic_db,
            narrative_db,
            relational_db,
            prefrontal_db,
            vault_dir: None,
        })
    }

    // ── 认知域访问器 / Cognitive Domain Accessors ──

    /// 情感中枢数据库引用 / Limbic database reference
    ///
    /// 数字生命的感受核心：情感快照 + 非理性引擎。
    pub fn limbic(&self) -> &sled::Db {
        &self.limbic_db
    }

    /// 叙事皮层数据库引用 / Narrative cortex database reference
    ///
    /// 数字生命的自传与内省：叙事弧 + 日记。
    pub fn narrative(&self) -> &sled::Db {
        &self.narrative_db
    }

    /// 关系海马体数据库引用 / Relational hippocampus database reference
    ///
    /// 数字生命的社会记忆：冲突 + 仪式 + 脆弱 + 期待。
    pub fn relational(&self) -> &sled::Db {
        &self.relational_db
    }

    /// 前额工具区数据库引用 / Prefrontal utility database reference
    ///
    /// 数字生命的执行功能：提醒 + 文件。
    pub fn prefrontal(&self) -> &sled::Db {
        &self.prefrontal_db
    }

    // ── 生命周期管理 / Lifecycle Management ──

    /// 刷新所有认知域的 WAL / Flush WAL for all cognitive domains.
    ///
    /// 确保所有未持久化的写入落盘。通常在优雅关闭时调用。
    pub fn flush_all(&self) -> Result<(), VaultError> {
        self.limbic_db.flush()?;
        self.narrative_db.flush()?;
        self.relational_db.flush()?;
        self.prefrontal_db.flush()?;
        Ok(())
    }

    /// 获取保险库目录路径 / Get vault directory path.
    ///
    /// 返回 vault 根目录（包含 4 个认知域子目录）。
    /// 仅在已从磁盘打开时有效；内存模式返回 None。
    pub fn vault_dir(&self) -> Option<&Path> {
        self.vault_dir.as_deref()
    }
}

// ════════════════════════════════════════════════════════════════════
// VaultTree — 统一 CRUD 骨架 trait / Unified CRUD Skeleton Trait
// ════════════════════════════════════════════════════════════════════

/// 认知域 Tree 操作 — 统一 CRUD 骨架
/// Cognitive domain tree operations — unified CRUD skeleton.
///
/// 所有 Store 的 save/load/get_by_id/count 操作共享此 trait 实现，
/// 消除各 Store 重复的 bincode 序列化 + sled 读写骨架代码。
///
/// # 消除重复 / Deduplication
///
/// 重构前每个 Store 重复实现：
/// ```ignore
/// fn save(&self, key, value) -> Result<(), Error> {
///     let bytes = bincode::serialize(value)?;
///     self.tree.insert(key, bytes)?;
///     Ok(())
/// }
/// fn load(&self, key) -> Result<Option<V>, Error> {
///     match self.tree.get(key)? {
///         Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
///         None => Ok(None),
///     }
/// }
/// ```
///
/// 重构后统一为 `VaultTree` 的默认实现。
pub trait VaultTree<V: Serialize + for<'de> Deserialize<'de>> {
    /// 获取底层 sled::Tree 引用 / Get underlying sled::Tree reference.
    fn tree(&self) -> &sled::Tree;

    /// 保存单例值 / Save singleton value
    ///
    /// 用于 EmotionStore("snapshot"), NarrativeSelfStore("self"),
    /// ConflictStore("manager"), IrrationalityStore("manager") 等
    /// 只有一个全局键的快照存储场景。
    fn save_singleton(&self, key: &[u8], value: &V) -> Result<(), VaultError> {
        let bytes = bincode::serialize(value)?;
        self.tree().insert(key, bytes.as_slice())?;
        Ok(())
    }

    /// 加载单例值 / Load singleton value
    fn load_singleton(&self, key: &[u8]) -> Result<Option<V>, VaultError> {
        match self.tree().get(key) {
            Ok(Some(bytes)) => {
                let value: V = bincode::deserialize(&bytes)?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(VaultError::Sled(e.to_string())),
        }
    }

    /// 按 u64 ID 保存索引项 / Save indexed item by u64 ID
    ///
    /// 键为 `id.to_be_bytes()`（8 字节大端序），
    /// 适用于 NarrativeArc, ConflictSignal, TurningPoint 等按 ID 索引的场景。
    fn save_indexed(&self, id: u64, value: &V) -> Result<(), VaultError> {
        self.save_singleton(&id.to_be_bytes(), value)
    }

    /// 按 u64 ID 加载索引项 / Load indexed item by u64 ID
    fn load_indexed(&self, id: u64) -> Result<Option<V>, VaultError> {
        self.load_singleton(&id.to_be_bytes())
    }

    /// 收集所有 u64 ID / Collect all u64 IDs
    ///
    /// 遍历 Tree 中所有键，解析为 u64（大端序）。
    /// 非法键（长度 ≠ 8）被跳过。
    fn collect_ids(&self) -> Result<Vec<u64>, VaultError> {
        let mut ids = Vec::new();
        for item in self.tree().iter() {
            let (key, _) = item?;
            if let Ok(bytes) = <[u8; 8]>::try_from(key.as_ref()) {
                ids.push(u64::from_be_bytes(bytes));
            }
        }
        Ok(ids)
    }

    /// 条目总数 / Entry count
    fn count(&self) -> usize {
        self.tree().len()
    }

    /// 是否为空 / Check if empty
    fn is_empty(&self) -> bool {
        self.tree().is_empty()
    }
}

// ════════════════════════════════════════════════════════════════════
// MigrationReport — 迁移报告 / Migration Report
// ════════════════════════════════════════════════════════════════════

/// 迁移报告 / Migration report
///
/// 记录从旧目录结构迁移到 AtriumVault 的执行结果。
#[derive(Debug, Default)]
pub struct MigrationReport {
    /// 迁移的 Tree 数 / Number of trees migrated
    pub trees_migrated: usize,
    /// 迁移的键值对数 / Number of key-value pairs migrated
    pub entries_migrated: usize,
    /// 跳过的空目录 / Skipped empty directories
    pub skipped: Vec<String>,
    /// 失败的迁移 / Failed migrations
    pub failures: Vec<(String, String)>,
}

impl std::fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MigrationReport: {} trees, {} entries migrated, {} skipped, {} failures",
            self.trees_migrated,
            self.entries_migrated,
            self.skipped.len(),
            self.failures.len()
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// 惰性迁移 / Lazy Migration
// ════════════════════════════════════════════════════════════════════

impl AtriumVault {
    /// 检测是否需要从旧目录结构迁移 / Check if migration from old layout is needed.
    ///
    /// 旧结构：`{base}/narrative/`, `{base}/conflict/`, ...
    /// 新结构：`{base}/vault/limbic/`, `{base}/vault/narrative/`, ...
    ///
    /// 当旧目录存在且 vault 目录不存在时返回 true。
    pub fn needs_migration(base: &str) -> bool {
        let vault_exists = Path::new(&format!("{}/vault", base)).exists();
        if vault_exists {
            return false;
        }

        // 检查任意旧目录是否存在 / Check if any old directory exists
        let old_dirs = [
            "narrative",
            "conflict",
            "irrationality",
            "ritual",
            "vulnerability",
            "emotion",
            "anticipation",
            "diary",
            "reminders",
        ];
        old_dirs
            .iter()
            .any(|d| Path::new(&format!("{}/{}", base, d)).exists())
    }

    /// 从旧目录结构迁移 / Migrate from old directory layout.
    ///
    /// 逐 Store 读取旧 sled 实例 → 写入新 Vault 中的命名 Tree。
    /// 迁移完成后将旧目录重命名为 `{name}.legacy/`（不删除）。
    ///
    /// # 安全性 / Safety
    ///
    /// - 旧数据不会被删除，仅重命名为 `.legacy`
    /// - 迁移失败不影响旧数据的完整性
    /// - 迁移是幂等的：重复调用会跳过已迁移的数据
    pub fn migrate_from_legacy(base: &str) -> Result<MigrationReport, VaultError> {
        let vault = Self::open(base)?;
        let mut report = MigrationReport::default();

        // ── 情感中枢迁移 / Limbic migration ──
        Self::migrate_pattern_b_store(
            &format!("{}/emotion", base),
            &vault.limbic_db,
            "emotion_snapshot",
            &mut report,
        );

        Self::migrate_pattern_a_store(
            &format!("{}/irrationality", base),
            &vault.limbic_db,
            &[
                "irrationality_manager",
                "irrationality_pulses",
                "irrationality_residues",
            ],
            &mut report,
        );

        // ── 叙事皮层迁移 / Narrative cortex migration ──
        Self::migrate_pattern_a_store(
            &format!("{}/narrative", base),
            &vault.narrative_db,
            &[
                "narrative_self",
                "narrative_arcs",
                "narrative_turning_points",
            ],
            &mut report,
        );

        Self::migrate_pattern_b_store(
            &format!("{}/diary", base),
            &vault.narrative_db,
            "diary_entries",
            &mut report,
        );

        // ── 关系海马体迁移 / Relational hippocampus migration ──
        Self::migrate_pattern_a_store(
            &format!("{}/conflict", base),
            &vault.relational_db,
            &["conflict_manager", "conflict_signals"],
            &mut report,
        );

        Self::migrate_pattern_a_store(
            &format!("{}/ritual", base),
            &vault.relational_db,
            &["ritual_snapshot", "ritual_patterns", "ritual_anniversaries"],
            &mut report,
        );

        Self::migrate_pattern_a_store(
            &format!("{}/vulnerability", base),
            &vault.relational_db,
            &["vulnerability_window", "vulnerability_history"],
            &mut report,
        );

        // AnticipationStore 使用前缀键，迁移整个默认 tree
        Self::migrate_pattern_b_store_prefix(
            &format!("{}/anticipation", base),
            &vault.relational_db,
            "anticipation_events",
            "anticipation_pending",
            &mut report,
        );

        // ── 前额工具区迁移 / Prefrontal utility migration ──
        // ReminderStore: counter 键 + reminder 键
        Self::migrate_pattern_b_store(
            &format!("{}/reminders", base),
            &vault.prefrontal_db,
            "reminders",
            &mut report,
        );

        // ── 重命名旧目录 / Rename old directories ──
        let old_dirs = [
            "narrative",
            "conflict",
            "irrationality",
            "ritual",
            "vulnerability",
            "emotion",
            "anticipation",
            "diary",
            "reminders",
        ];
        for dir in &old_dirs {
            let old_path = format!("{}/{}", base, dir);
            if Path::new(&old_path).exists() {
                let legacy_path = format!("{}.legacy", old_path);
                if std::fs::rename(&old_path, &legacy_path).is_ok() {
                    tracing::info!("Migration: renamed {} → {}.legacy", old_path, dir);
                }
            }
        }

        tracing::info!("Migration complete: {}", report);
        Ok(report)
    }

    /// 迁移模式 A Store（已有命名 Tree 的 sled 实例）
    /// Migrate a Pattern A store (sled instance with named trees).
    ///
    /// 逐 Tree 复制所有键值对到目标 Db 的同名 Tree。
    fn migrate_pattern_a_store(
        old_path: &str,
        target_db: &sled::Db,
        tree_names: &[&str],
        report: &mut MigrationReport,
    ) {
        let old_db = match sled::open(old_path) {
            Ok(db) => db,
            Err(_) => {
                report.skipped.push(old_path.to_string());
                return;
            }
        };

        for &tree_name in tree_names {
            match old_db.open_tree(tree_name) {
                Ok(old_tree) => {
                    let mut entries = 0;
                    if let Ok(new_tree) = target_db.open_tree(tree_name) {
                        for (k, v) in old_tree.iter().flatten() {
                            if new_tree.insert(k.as_ref(), v.as_ref()).is_ok() {
                                entries += 1;
                            }
                        }
                        let _ = new_tree.flush();
                    }
                    report.trees_migrated += 1;
                    report.entries_migrated += entries;
                }
                Err(e) => {
                    report
                        .failures
                        .push((format!("{}/{}", old_path, tree_name), e.to_string()));
                }
            }
        }
    }

    /// 迁移模式 B Store（使用默认 Tree 的 sled 实例，单目标 Tree）
    /// Migrate a Pattern B store (sled instance using default tree, single target tree).
    ///
    /// 将默认 Tree 的所有键值对复制到目标 Db 的指定命名 Tree。
    fn migrate_pattern_b_store(
        old_path: &str,
        target_db: &sled::Db,
        target_tree_name: &str,
        report: &mut MigrationReport,
    ) {
        let old_db = match sled::open(old_path) {
            Ok(db) => db,
            Err(_) => {
                report.skipped.push(old_path.to_string());
                return;
            }
        };

        match target_db.open_tree(target_tree_name) {
            Ok(new_tree) => {
                let mut entries = 0;
                for (k, v) in old_db.iter().flatten() {
                    if new_tree.insert(k.as_ref(), v.as_ref()).is_ok() {
                        entries += 1;
                    }
                }
                let _ = new_tree.flush();
                report.trees_migrated += 1;
                report.entries_migrated += entries;
            }
            Err(e) => {
                report.failures.push((old_path.to_string(), e.to_string()));
            }
        }
    }

    /// 迁移模式 B Store（使用前缀键的 sled 实例，拆分为两个目标 Tree）
    /// Migrate a Pattern B store with prefix keys, splitting into two target trees.
    ///
    /// 专门用于 AnticipationStore，其键空间为：
    /// - `event/{id}` → AnticipationEvent
    /// - `pending/{expected_at}` → id
    fn migrate_pattern_b_store_prefix(
        old_path: &str,
        target_db: &sled::Db,
        event_tree_name: &str,
        pending_tree_name: &str,
        report: &mut MigrationReport,
    ) {
        let old_db = match sled::open(old_path) {
            Ok(db) => db,
            Err(_) => {
                report.skipped.push(old_path.to_string());
                return;
            }
        };

        let event_tree = target_db.open_tree(event_tree_name);
        let pending_tree = target_db.open_tree(pending_tree_name);

        if let (Ok(et), Ok(pt)) = (event_tree, pending_tree) {
            let mut entries = 0;
            for (k, v) in old_db.iter().flatten() {
                let key_str = String::from_utf8_lossy(k.as_ref());
                let target = if key_str.starts_with("event/") {
                    &et
                } else {
                    &pt
                };
                if target.insert(k.as_ref(), v.as_ref()).is_ok() {
                    entries += 1;
                }
            }
            let _ = et.flush();
            let _ = pt.flush();
            report.trees_migrated += 2;
            report.entries_migrated += entries;
        } else {
            report
                .failures
                .push((old_path.to_string(), "failed to open target trees".into()));
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// impl_vault_tree — 自动实现宏 / Auto-implementation Macro
// ════════════════════════════════════════════════════════════════════

/// 为 Store 自动实现 VaultTree trait
/// Auto-implement VaultTree trait for Store types.
///
/// # 用法 / Usage
///
/// ```ignore
/// impl_vault_tree!(EmotionStore, tree, EmotionSnapshot);
/// impl_vault_tree!(DiaryStore, tree, DiaryEntry);
/// ```
#[macro_export]
macro_rules! impl_vault_tree {
    ($store:ty, $tree_field:ident, $value:ty) => {
        impl $crate::atrium_vault::VaultTree<$value> for $store {
            fn tree(&self) -> &sled::Tree {
                &self.$tree_field
            }
        }
    };
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_open_in_memory() {
        let vault = AtriumVault::open_in_memory().unwrap();
        // 验证 4 个域都可访问 / Verify all 4 domains are accessible
        assert!(!vault.limbic().was_recovered());
        assert!(!vault.narrative().was_recovered());
        assert!(!vault.relational().was_recovered());
        assert!(!vault.prefrontal().was_recovered());
    }

    #[test]
    fn test_vault_tree_names_unique() {
        let vault = AtriumVault::open_in_memory().unwrap();

        // 情感中枢 Tree 名 / Limbic tree names
        let limbic_trees = vec![
            "emotion_snapshot",
            "irrationality_manager",
            "irrationality_pulses",
            "irrationality_residues",
        ];
        for name in &limbic_trees {
            assert!(
                vault.limbic().open_tree(name).is_ok(),
                "limbic tree {} failed",
                name
            );
        }

        // 叙事皮层 Tree 名 / Narrative tree names
        let narrative_trees = vec![
            "narrative_self",
            "narrative_arcs",
            "narrative_turning_points",
            "diary_entries",
        ];
        for name in &narrative_trees {
            assert!(
                vault.narrative().open_tree(name).is_ok(),
                "narrative tree {} failed",
                name
            );
        }

        // 关系海马体 Tree 名 / Relational tree names
        let relational_trees = vec![
            "conflict_manager",
            "conflict_signals",
            "ritual_snapshot",
            "ritual_patterns",
            "ritual_anniversaries",
            "vulnerability_window",
            "vulnerability_history",
            "anticipation_events",
            "anticipation_pending",
        ];
        for name in &relational_trees {
            assert!(
                vault.relational().open_tree(name).is_ok(),
                "relational tree {} failed",
                name
            );
        }

        // 前额工具区 Tree 名 / Prefrontal tree names
        let prefrontal_trees = vec!["reminders", "reminder_counter", "file_meta"];
        for name in &prefrontal_trees {
            assert!(
                vault.prefrontal().open_tree(name).is_ok(),
                "prefrontal tree {} failed",
                name
            );
        }
    }

    #[test]
    fn test_vault_error_conversions() {
        // From<io::Error> — 直接构造验证 / Construct directly for verification
        let io_err: VaultError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "test io error").into();
        assert!(matches!(io_err, VaultError::Io(_)));

        // From<bincode::Error> — 通过反序列化无效字节触发 / Trigger via deserializing invalid bytes
        let codec_err: VaultError = bincode::deserialize::<String>(&[0xFF, 0xFF, 0xFF, 0xFF])
            .unwrap_err()
            .into();
        assert!(matches!(codec_err, VaultError::Codec(_)));

        // From<sled::Error> — 跨平台安全：仅在 sled 确实报错时验证
        // Cross-platform safe: only verify when sled actually errors
        let long_path = format!("{}/{}", std::env::temp_dir().display(), "a".repeat(300));
        if let Err(e) = sled::Config::new().path(&long_path).open() {
            let vault_err: VaultError = e.into();
            assert!(matches!(vault_err, VaultError::Sled(_)));
        }
        // 若 sled 成功创建超长路径则跳过（编译期已验证 From<sled::Error> trait 存在）
    }

    #[test]
    fn test_vault_flush_all() {
        let vault = AtriumVault::open_in_memory().unwrap();
        assert!(vault.flush_all().is_ok());
    }

    #[test]
    fn test_vault_tree_trait() {
        // 使用一个简单的测试结构体验证 VaultTree trait
        let db = sled::Config::new().temporary(true).open().unwrap();
        let tree = db.open_tree("test_tree").unwrap();

        // 手动实现 VaultTree 用于测试
        struct TestStore {
            tree: sled::Tree,
        }
        impl VaultTree<String> for TestStore {
            fn tree(&self) -> &sled::Tree {
                &self.tree
            }
        }

        let store = TestStore { tree };

        // save_singleton / load_singleton
        store.save_singleton(b"key1", &"hello".to_string()).unwrap();
        let loaded: Option<String> = store.load_singleton(b"key1").unwrap();
        assert_eq!(loaded, Some("hello".to_string()));

        // load 不存在的键 / Load nonexistent key
        let missing: Option<String> = store.load_singleton(b"key2").unwrap();
        assert!(missing.is_none());

        // save_indexed / load_indexed
        store
            .save_indexed(42, &"indexed_value".to_string())
            .unwrap();
        let indexed: Option<String> = store.load_indexed(42).unwrap();
        assert_eq!(indexed, Some("indexed_value".to_string()));

        // count
        assert_eq!(store.count(), 2);

        // collect_ids
        let ids = store.collect_ids().unwrap();
        assert!(ids.contains(&42));
    }

    #[test]
    fn test_migration_report_display() {
        let report = MigrationReport {
            trees_migrated: 3,
            entries_migrated: 100,
            skipped: vec!["old_empty".into()],
            failures: vec![],
        };
        let s = format!("{}", report);
        assert!(s.contains("3 trees"));
        assert!(s.contains("100 entries"));
    }

    #[test]
    fn test_needs_migration_no_old_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        // 无旧目录，不需要迁移
        assert!(!AtriumVault::needs_migration(tmp.path().to_str().unwrap()));
    }

    #[test]
    fn test_needs_migration_vault_exists() {
        let tmp = tempfile::tempdir().unwrap();
        // vault 目录已存在，不需要迁移
        std::fs::create_dir_all(format!("{}/vault", tmp.path().to_str().unwrap())).ok();
        assert!(!AtriumVault::needs_migration(tmp.path().to_str().unwrap()));
    }
}
