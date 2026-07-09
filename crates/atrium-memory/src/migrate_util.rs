// SPDX-License-Identifier: MIT
//! 旧 sled 迁移工具 — 三阶段强制清理 / Legacy Sled Migration Utility — Three-Stage Forced Cleanup
//!
//! 数字生命记忆持久化保障 — 旧 sled 目录锁文件残留时，三阶段强制清理确保
//! SQLite 迁移不被阻塞，记忆跨越重启存活。
//!
//! Digital life memory persistence safeguard — when legacy sled directory lockfiles
//! remain, three-stage forced cleanup ensures SQLite migration is not blocked
//! and memory survives restarts.

use std::path::Path;

/// 三阶段强制打开旧 sled 目录 / Three-stage forced open of legacy sled directory
///
/// 数字生命记忆迁移的核心工具 — 当 `sled::open` 因锁文件冲突失败（Windows os error 5）时，
/// 执行三阶段清理确保旧目录不再阻塞后续启动。
///
/// Digital life memory migration core utility — when `sled::open` fails due to
/// lockfile conflict (Windows os error 5), executes three-stage cleanup to ensure
/// the legacy directory no longer blocks subsequent startups.
///
/// # 三阶段清理策略 / Three-Stage Cleanup Strategy
///
/// 1. **阶段 1 — 重命名**：`fs::rename(sled_path, "{sled_path}.sled.bak")`
///    - 成功 → 旧目录已隔离，返回 None（跳过迁移，数据保留在 .bak）
///    - 失败 → 进入阶段 2
///
/// 2. **阶段 2 — 锁文件删除 + 重试**：删除旧目录内 `db`/`db.lock` 锁文件，重试 `sled::open`
///    - 重试成功 → 返回 Some(db)，调用方执行迁移后应调用 `finalize_sled_migration` 重命名
///    - 重试失败 → 进入阶段 3
///
/// 3. **阶段 3 — 强制删除**：`fs::remove_dir_all(sled_path)`
///    - 旧 sled 数据已不可读，清理优于残留（避免每次启动重复失败）
///    - 返回 None（跳过迁移）
///
/// # 参数 / Parameters
/// - `sled_path`: 旧 sled 目录路径 / Legacy sled directory path
///
/// # 返回 / Returns
/// - `Some(sled::Db)`: 成功打开，调用方负责数据迁移 + 调用 `finalize_sled_migration`
/// - `None`: 跳过迁移（旧目录不存在 / 已重命名 / 已强制清理）
pub fn try_open_legacy_sled(sled_path: &str) -> Option<sled::Db> {
    // 旧目录不存在 → 无需迁移 / No legacy directory → no migration needed
    if !Path::new(sled_path).exists() {
        return None;
    }

    // 首次尝试打开 — 可能因锁文件失败 / First attempt — may fail due to lockfile
    match sled::open(sled_path) {
        Ok(db) => {
            tracing::info!(
                "旧 sled 目录打开成功，开始迁移 / Legacy sled opened successfully, starting migration: {}",
                sled_path
            );
            Some(db)
        }
        Err(e) => {
            tracing::warn!(
                "旧 sled 目录打开失败 — 启动三阶段强制清理 / Legacy sled open failed — starting three-stage forced cleanup: {} | error: {}",
                sled_path, e
            );
            stage1_rename_or_stage2_retry_or_stage3_remove(sled_path)
        }
    }
}

/// 阶段 1-3 依次执行 / Execute stages 1-3 in sequence
fn stage1_rename_or_stage2_retry_or_stage3_remove(sled_path: &str) -> Option<sled::Db> {
    // ── 阶段 1：重命名为 .sled.bak / Stage 1: rename to .sled.bak ──
    let bak_path = format!("{}.sled.bak", sled_path);
    if std::fs::rename(sled_path, &bak_path).is_ok() {
        tracing::warn!(
            "阶段 1 成功 — 旧 sled 目录已重命名为 {}，跳过迁移（数据保留在 .bak）/ Stage 1 success — legacy sled renamed to {}, skip migration (data preserved in .bak)",
            bak_path, bak_path
        );
        return None;
    }
    tracing::warn!(
        "阶段 1 失败 — 重命名失败，进入阶段 2（锁文件删除 + 重试）/ Stage 1 failed — rename failed, entering stage 2 (lockfile removal + retry)"
    );

    // ── 阶段 2：删除锁文件 + 重试 / Stage 2: remove lockfiles + retry ──
    // sled 锁文件通常为 `db` 和 `db.lock` / sled lockfiles are typically `db` and `db.lock`
    for lockfile in &["db", "db.lock"] {
        let lock_path = format!("{}/{}", sled_path, lockfile);
        if Path::new(&lock_path).exists() {
            if let Err(e) = std::fs::remove_file(&lock_path) {
                tracing::warn!(
                    "阶段 2 — 锁文件删除失败 / Stage 2 — lockfile removal failed: {} | error: {}",
                    lock_path,
                    e
                );
            } else {
                tracing::info!(
                    "阶段 2 — 锁文件已删除 / Stage 2 — lockfile removed: {}",
                    lock_path
                );
            }
        }
    }
    match sled::open(sled_path) {
        Ok(db) => {
            tracing::info!(
                "阶段 2 成功 — 锁文件删除后重试打开成功，继续迁移 / Stage 2 success — retry after lockfile removal succeeded, continuing migration"
            );
            Some(db)
        }
        Err(e) => {
            tracing::warn!(
                "阶段 2 失败 — 重试仍失败，进入阶段 3（强制删除）/ Stage 2 failed — retry still fails, entering stage 3 (forced removal) | error: {}",
                e
            );
            // ── 阶段 3：强制删除 / Stage 3: forced removal ──
            match std::fs::remove_dir_all(sled_path) {
                Ok(()) => {
                    tracing::warn!(
                        "阶段 3 — 旧 sled 目录已强制删除（数据不可读，清理优于残留）/ Stage 3 — legacy sled directory force-removed (data unreadable, cleanup preferred over residue): {}",
                        sled_path
                    );
                }
                Err(rm_err) => {
                    tracing::error!(
                        "阶段 3 失败 — 强制删除失败，旧目录残留（可能需手动清理）/ Stage 3 failed — forced removal failed, legacy directory remains (may need manual cleanup): {} | error: {}",
                        sled_path, rm_err
                    );
                }
            }
            None
        }
    }
}

/// 迁移完成后重命名旧 sled 目录为 .sled.bak / Rename legacy sled dir to .sled.bak after migration
///
/// 数字生命记忆迁移的收尾步骤 — 迁移成功后隔离旧目录，避免下次启动重复迁移。
/// Finalization step for digital life memory migration — isolate legacy directory
/// after successful migration to prevent repeated migration on next startup.
///
/// # 参数 / Parameters
/// - `sled_path`: 旧 sled 目录路径 / Legacy sled directory path
///
/// # 返回 / Returns
/// - `true`: 重命名成功 / Rename succeeded
/// - `false`: 重命名失败（旧目录残留，下次启动会再次尝试迁移但因 SQLite 已有数据而跳过）/ Rename failed (legacy dir remains, next startup will skip migration since SQLite already has data)
pub fn finalize_sled_migration(sled_path: &str) -> bool {
    let bak_path = format!("{}.sled.bak", sled_path);
    match std::fs::rename(sled_path, &bak_path) {
        Ok(()) => {
            tracing::info!(
                "迁移完成 — 旧 sled 目录已重命名为 {} / Migration complete — legacy sled renamed to {}",
                bak_path, bak_path
            );
            true
        }
        Err(e) => {
            tracing::info!(
                "迁移完成 — 旧 sled 目录重命名失败，保留原目录（SQLite 已有数据，下次启动跳过迁移）/ Migration complete — legacy rename failed, dir preserved (SQLite has data, next startup skips migration) | error: {}",
                e
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1.1 — 阶段 1 正常路径：try_open_legacy_sled 成功打开已存在的 sled 目录
    /// Test 1.1 — Stage 1 normal path: try_open_legacy_sled opens an existing sled dir
    #[test]
    fn test_try_open_legacy_sled_normal_open() {
        // 创建临时 sled 目录 / Create temp sled directory
        let temp_dir =
            std::env::temp_dir().join(format!("atrium_test_sled_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        // 先用 sled::open 创建并写入数据 / Create and write data via sled::open
        {
            let db = sled::open(&temp_dir).unwrap();
            db.insert(b"test_key", b"test_value").unwrap();
        }

        // try_open_legacy_sled 应成功打开 / Should open successfully
        let result = try_open_legacy_sled(temp_dir.to_str().unwrap());
        assert!(
            result.is_some(),
            "阶段 1 正常路径应返回 Some(db) / Stage 1 normal path should return Some(db)"
        );

        // 清理 / Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// 测试 1.2 — 无旧目录返回 None：路径不存在时跳过迁移
    /// Test 1.2 — Returns None when legacy dir absent: skip migration when path missing
    #[test]
    fn test_try_open_legacy_sled_no_directory() {
        // 构造一个确定不存在的路径 / Construct a path that definitely does not exist
        let non_existent_path =
            std::env::temp_dir().join(format!("atrium_nonexistent_{}", std::process::id()));
        let result = try_open_legacy_sled(non_existent_path.to_str().unwrap());
        assert!(
            result.is_none(),
            "旧目录不存在时应返回 None / Should return None when legacy dir is absent"
        );
    }

    /// 测试 1.3 — finalize_sled_migration 重命名成功：迁移后将旧目录重命名为 .sled.bak
    /// Test 1.3 — finalize_sled_migration renames to .bak: rename legacy dir to .sled.bak after migration
    #[test]
    fn test_finalize_sled_migration_renames_to_bak() {
        let temp_dir =
            std::env::temp_dir().join(format!("atrium_test_finalize_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let result = finalize_sled_migration(temp_dir.to_str().unwrap());
        assert!(result, "重命名应成功 / Rename should succeed");

        let bak_path = format!("{}.sled.bak", temp_dir.display());
        assert!(
            std::path::Path::new(&bak_path).exists(),
            ".sled.bak 应存在 / .sled.bak should exist"
        );

        // 清理 / Cleanup
        let _ = std::fs::remove_dir_all(format!("{}.sled.bak", temp_dir.display()));
    }
}
