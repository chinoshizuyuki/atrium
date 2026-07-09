// SPDX-License-Identifier: MIT
//! 关键信息缓存
//!
//! 存储永不丢弃的用户偏好、习惯、身份信息和重要约定。
//! 独立于对话历史，任何 session 都能读取。
//!
//! 双后端持久化：SQLite（默认，Windows 兼容）+ sled（旧数据兼容）。
//! upsert 时同步写入磁盘，启动时从持久化后端恢复全量数据到内存缓存。
//! KeyFactCache — Key information cache.
//!
//! Stores high-value user preferences, habits, and key information for fast retrieval.
//! Available for recall in any session at any time.
//!
//! Dual-backend persistence: SQLite (default, Windows-compatible) + sled (legacy compat).
//! Writes are synchronously flushed on upsert; on startup, full data is restored
//! from the persistent backend into the in-memory cache.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

/// 关键信息类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyFactCategory {
    Preference,
    Identity,
    Commitment,
    Todo,
    Relationship,
}

impl KeyFactCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "偏好",
            Self::Identity => "身份",
            Self::Commitment => "约定",
            Self::Todo => "待办",
            Self::Relationship => "关系",
        }
    }
}

/// 关键信息条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFact {
    pub content: String,
    pub category: KeyFactCategory,
    pub confidence: f64,
    pub source: String,
    pub first_seen: i64,
    pub last_confirmed: i64,
    pub confirmed_count: u32,
}

impl KeyFact {
    pub fn new(content: &str, category: KeyFactCategory, confidence: f64, source: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            content: content.to_string(),
            category,
            confidence: confidence.clamp(0.0, 1.0),
            source: source.to_string(),
            first_seen: now,
            last_confirmed: now,
            confirmed_count: 1,
        }
    }

    pub fn confirm(&mut self) {
        self.confirmed_count += 1;
        self.last_confirmed = chrono::Utc::now().timestamp();
        self.confidence = self.confidence + (1.0 - self.confidence) * 0.3;
    }

    /// sled 存储键：category|content
    fn storage_key(&self) -> Vec<u8> {
        format!("{}|{}", self.category.as_str(), self.content).into_bytes()
    }
}

/// 持久化后端 / Persistence backend
enum Backend {
    /// 纯内存（无持久化）/ In-memory only (no persistence)
    Memory,
    /// sled 后端（旧数据兼容）/ Sled backend (legacy compat)
    Sled(sled::Db),
    /// SQLite 后端（新默认）/ SQLite backend (new default)
    Sqlite(Mutex<Connection>),
}

/// 关键信息缓存（内存 + 双后端持久化）/ Key info cache (in-memory + dual-backend persistence)
pub struct KeyFactCache {
    facts: RwLock<HashMap<String, Vec<KeyFact>>>,
    backend: Backend,
}

impl KeyFactCache {
    /// 打开 SQLite 后端的缓存（推荐 / Recommended）
    ///
    /// 数字生命的关键信息（用户身份、偏好、约定）必须跨重启存活。
    /// SQLite 在 Windows 上无锁文件兼容性问题。若检测到旧 sled 目录，自动迁移。
    ///
    /// Digital life's key info (user identity, preferences, commitments) must survive
    /// restarts. SQLite has no lock file issues on Windows. Auto-migrates from sled.
    pub fn open_sqlite(path: &str) -> Result<Self, rusqlite::Error> {
        // 确保父目录存在 / Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Connection::open 失败时记录诊断上下文（路径/父目录权限/目标状态）/ Log diagnostics on open failure
        let conn = Connection::open(path).map_err(|e| {
            tracing::error!(
                "KeyFactCache sqlite open failed: {} | path: {} | parent exists: {} | parent writable: {} | target exists: {}",
                e,
                path,
                std::path::Path::new(path).parent().map(|p| p.exists()).unwrap_or(false),
                std::path::Path::new(path).parent()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(false),
                std::path::Path::new(path).exists(),
            );
            e
        })?;
        // 启用 WAL 模式 — 失败时追加路径诊断 / Enable WAL mode — append path diagnostics on failure
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| {
                tracing::error!("KeyFactCache PRAGMA failed: {} | path: {}", e, path);
                e
            })?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS key_facts (
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                confidence REAL NOT NULL,
                source TEXT NOT NULL,
                first_seen INTEGER NOT NULL,
                last_confirmed INTEGER NOT NULL,
                confirmed_count INTEGER NOT NULL,
                PRIMARY KEY(category, content)
            )",
            [],
        )
        .map_err(|e| {
            tracing::error!("KeyFactCache CREATE TABLE failed: {} | path: {}", e, path);
            e
        })?;

        // 数据迁移：检测旧 sled 目录 / Migration: detect legacy sled directory
        let sled_path = path.replace(".db", "");
        if std::path::Path::new(&sled_path).exists() {
            Self::migrate_sled_to_sqlite(&sled_path, &conn)?;
        }

        let cache = Self {
            facts: RwLock::new(HashMap::new()),
            backend: Backend::Sqlite(Mutex::new(conn)),
        };
        cache.load_from_sqlite();
        Ok(cache)
    }

    /// 从旧 sled 目录迁移数据到 SQLite / Migrate from legacy sled to SQLite
    fn migrate_sled_to_sqlite(sled_path: &str, conn: &Connection) -> Result<(), rusqlite::Error> {
        // 检查 SQLite 是否已有数据 — 若有则跳过迁移 / Skip if SQLite already has data
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM key_facts", [], |row| row.get(0))?;
        if count > 0 {
            tracing::info!(
                "KeyFactCache: SQLite 已有 {} 条数据，跳过 sled 迁移 / SQLite already has {} facts, skip sled migration",
                count, count
            );
            return Ok(());
        }

        // 三阶段强制打开旧 sled 目录 — 通用迁移工具 / Three-stage forced open — generic migration utility
        let sled_db = match crate::migrate_util::try_open_legacy_sled(sled_path) {
            Some(db) => db,
            None => return Ok(()), // 已清理或无旧目录，跳过迁移 / Cleaned or no legacy dir, skip
        };

        // 迁移数据 — KeyFactCache 特有逻辑 / Migrate data — KeyFactCache-specific logic
        let mut migrated = 0u32;
        // 使用 unchecked_transaction — 只需 &self，避免 &mut Connection 借用冲突
        // Use unchecked_transaction — requires only &self, avoids &mut Connection borrow conflict
        let tx = conn.unchecked_transaction()?;
        for item in sled_db.iter() {
            let (_, value) = match item {
                Ok(kv) => kv,
                Err(_) => continue,
            };
            if let Ok(fact) = bincode::deserialize::<KeyFact>(&value) {
                let _ = tx.execute(
                    "INSERT OR REPLACE INTO key_facts
                     (category, content, confidence, source, first_seen, last_confirmed, confirmed_count)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        fact.category.as_str(),
                        &fact.content,
                        fact.confidence,
                        &fact.source,
                        fact.first_seen,
                        fact.last_confirmed,
                        fact.confirmed_count as i64,
                    ],
                );
                migrated += 1;
            }
        }
        tx.commit()?;

        // 关闭 sled Db 句柄 — Windows 上 open 的文件句柄会阻止目录重命名 / Close sled Db handle — on Windows, open file handles prevent directory rename
        drop(sled_db);
        // 迁移完成 — 通用函数重命名旧 sled 目录为 .sled.bak / Finalize — rename legacy dir via generic utility
        crate::migrate_util::finalize_sled_migration(sled_path);
        tracing::info!(
            "KeyFactCache: 迁移完成 {} 条 / Migration complete: {} facts",
            migrated,
            migrated
        );
        Ok(())
    }

    /// 从 SQLite 加载全部数据到内存 / Load all data from SQLite to in-memory cache
    fn load_from_sqlite(&self) {
        let conn = match &self.backend {
            Backend::Sqlite(conn_mutex) => match conn_mutex.lock() {
                Ok(c) => c,
                Err(_) => return,
            },
            _ => return,
        };
        let mut facts = self.facts.write().expect("key_facts rwlock write");
        let mut stmt = match conn.prepare(
            "SELECT category, content, confidence, source, first_seen, last_confirmed, confirmed_count
             FROM key_facts",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("KeyFactCache: SQLite load prepare failed: {}", e);
                return;
            }
        };
        let rows = stmt.query_map([], |row| {
            let category_str: String = row.get(0)?;
            let category = match category_str.as_str() {
                "偏好" => KeyFactCategory::Preference,
                "身份" => KeyFactCategory::Identity,
                "约定" => KeyFactCategory::Commitment,
                "待办" => KeyFactCategory::Todo,
                "关系" => KeyFactCategory::Relationship,
                _ => KeyFactCategory::Preference,
            };
            Ok(KeyFact {
                content: row.get(1)?,
                category,
                confidence: row.get(2)?,
                source: row.get(3)?,
                first_seen: row.get(4)?,
                last_confirmed: row.get(5)?,
                confirmed_count: row.get(6)?,
            })
        });
        match rows {
            Ok(rows) => {
                for fact in rows.flatten() {
                    let key = fact.category.as_str().to_string();
                    facts.entry(key).or_default().push(fact);
                }
            }
            Err(e) => {
                tracing::warn!("KeyFactCache: SQLite query_map failed: {}", e);
            }
        }
    }

    /// 创建带 sled 持久化的缓存（旧接口，保留兼容）/ Open sled-backed cache (legacy, kept for compat)
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let cache = Self {
            facts: RwLock::new(HashMap::new()),
            backend: Backend::Sled(db),
        };
        cache.load_from_sled();
        Ok(cache)
    }

    /// 创建纯内存缓存（测试用，不持久化）/ Create in-memory cache (for tests, no persistence)
    pub fn new_in_memory() -> Self {
        Self {
            facts: RwLock::new(HashMap::new()),
            backend: Backend::Memory,
        }
    }

    /// 从 sled 加载全部数据到内存 / Load all data from sled to in-memory cache
    fn load_from_sled(&self) {
        let db = match &self.backend {
            Backend::Sled(db) => db,
            _ => return,
        };
        let mut facts = self.facts.write().expect("key_facts rwlock write");
        for item in db.iter() {
            let (_, value) = match item {
                Ok(kv) => kv,
                Err(_) => continue,
            };
            if let Ok(fact) = bincode::deserialize::<KeyFact>(&value) {
                let key = fact.category.as_str().to_string();
                facts.entry(key).or_default().push(fact);
            }
        }
    }

    /// 添加或更新关键信息（内存 + 磁盘双写）
    pub fn upsert(&self, content: &str, category: KeyFactCategory, confidence: f64, source: &str) {
        let mut facts = self.facts.write().expect("key_facts rwlock write");
        let key = category.as_str().to_string();
        let entry = facts.entry(key).or_default();

        if let Some(existing) = entry.iter_mut().find(|f| f.content == content) {
            existing.confirm();
            existing.confidence = existing.confidence.max(confidence);
            // 同步写 sled
            self.persist(existing);
        } else {
            let fact = KeyFact::new(content, category, confidence, source);
            self.persist(&fact);
            entry.push(fact);
        }

        if entry.len() > 50 {
            entry.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            entry.truncate(50);
        }
    }

    /// 持久化单条 KeyFact 到后端（sled 或 SQLite）/ Persist single KeyFact to backend
    fn persist(&self, fact: &KeyFact) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                let key = fact.storage_key();
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(key, data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO key_facts
                         (category, content, confidence, source, first_seen, last_confirmed, confirmed_count)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            fact.category.as_str(),
                            &fact.content,
                            fact.confidence,
                            &fact.source,
                            fact.first_seen,
                            fact.last_confirmed,
                            fact.confirmed_count as i64,
                        ],
                    );
                }
            }
        }
    }

    /// 获取高置信度关键信息摘要（用于注入 Prompt）
    pub fn build_context(&self, min_confidence: f64) -> String {
        let facts = self.facts.read().expect("key_facts rwlock read");
        let mut ctx = String::from("[关键信息]\n");

        for category in &[
            KeyFactCategory::Identity,
            KeyFactCategory::Preference,
            KeyFactCategory::Commitment,
            KeyFactCategory::Relationship,
            KeyFactCategory::Todo,
        ] {
            let key = category.as_str().to_string();
            if let Some(entries) = facts.get(&key) {
                let high: Vec<&KeyFact> = entries
                    .iter()
                    .filter(|f| f.confidence >= min_confidence)
                    .collect();
                if !high.is_empty() {
                    ctx.push_str(&format!("{}:\n", category.as_str()));
                    for f in high {
                        ctx.push_str(&format!(
                            " - {} (置信度:{:.0}%)\n",
                            f.content,
                            f.confidence * 100.0
                        ));
                    }
                }
            }
        }

        if ctx == "[关键信息]\n" {
            return String::new();
        }
        ctx
    }

    pub fn total_count(&self) -> usize {
        self.facts
            .read()
            .expect("key_facts rwlock read")
            .values()
            .map(|v| v.len())
            .sum()
    }

    pub fn search(&self, query: &str) -> Vec<KeyFact> {
        let facts = self.facts.read().expect("key_facts rwlock read");
        let mut results = Vec::new();
        for entries in facts.values() {
            for f in entries {
                if f.content.contains(query) || f.source.contains(query) {
                    results.push(f.clone());
                }
            }
        }
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// 显式 flush 到磁盘（关闭前调用）/ Explicit flush to disk (call before shutdown)
    pub fn flush(&self) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                let _ = db.flush();
            }
            Backend::Sqlite(_) => {
                // SQLite WAL 模式下每次 commit 自动写穿 / SQLite WAL auto-flushes on commit
            }
        }
    }

    /// 是否启用持久化 / Whether persistence is enabled
    pub fn is_persistent(&self) -> bool {
        !matches!(self.backend, Backend::Memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cache() -> KeyFactCache {
        KeyFactCache::new_in_memory()
    }

    #[test]
    fn test_upsert_and_build_context() {
        let cache = test_cache();
        cache.upsert("主人喜欢Rust", KeyFactCategory::Preference, 0.85, "对话");
        cache.upsert("主人叫张三", KeyFactCategory::Identity, 0.95, "对话");

        let ctx = cache.build_context(0.5);
        assert!(ctx.contains("Rust"));
        assert!(ctx.contains("张三"));
    }

    #[test]
    fn test_confirm_increases_confidence() {
        let cache = test_cache();
        cache.upsert("关键约定", KeyFactCategory::Commitment, 0.6, "来源");
        cache.upsert("关键约定", KeyFactCategory::Commitment, 0.7, "来源2");

        let results = cache.search("约定");
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence > 0.7);
        assert_eq!(results[0].confirmed_count, 2);
    }

    #[test]
    fn test_min_confidence_filter() {
        let cache = test_cache();
        cache.upsert("高置信", KeyFactCategory::Preference, 0.9, "s");
        cache.upsert("低置信", KeyFactCategory::Preference, 0.3, "s");

        let ctx = cache.build_context(0.7);
        assert!(ctx.contains("高置信"));
        assert!(!ctx.contains("低置信"));
    }

    #[test]
    fn test_empty_cache() {
        let cache = test_cache();
        assert!(cache.build_context(0.5).is_empty());
        assert!(!cache.is_persistent());
    }

    #[test]
    fn test_persistent_roundtrip() {
        let path = "./target/atrium_kf_test_roundtrip";
        // 清理旧数据
        let _ = std::fs::remove_dir_all(path);

        // 写入
        {
            let cache = KeyFactCache::open(path).unwrap();
            assert!(cache.is_persistent());
            cache.upsert("持久化测试", KeyFactCategory::Identity, 0.99, "测试");
            cache.flush();
        }

        // 重新打开，应能恢复
        {
            let cache = KeyFactCache::open(path).unwrap();
            let ctx = cache.build_context(0.5);
            assert!(ctx.contains("持久化测试"), "重启后应能恢复: {}", ctx);
            assert_eq!(cache.total_count(), 1);
        }

        // 清理
        let _ = std::fs::remove_dir_all(path);
    }

    // ── SQLite 后端测试 / SQLite Backend Tests ──

    #[test]
    fn test_sqlite_persistent_roundtrip() {
        // 验证 SQLite 后端持久化 — 关键信息重启后保留
        // Verify SQLite backend persistence — key info survives restart.
        let path = "./target/test_key_facts_sqlite.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 写入 / Write
        {
            let cache = KeyFactCache::open_sqlite(path).unwrap();
            assert!(cache.is_persistent());
            cache.upsert("主人叫Aris", KeyFactCategory::Identity, 0.95, "对话");
            cache.upsert("喜欢弹钢琴", KeyFactCategory::Preference, 0.85, "对话");
            cache.flush();
        }

        // 重新打开 — 验证数据保留 / Reopen — verify data preserved
        {
            let cache = KeyFactCache::open_sqlite(path).unwrap();
            assert!(cache.is_persistent());
            assert_eq!(
                cache.total_count(),
                2,
                "重启后应保留 2 条 / should have 2 after restart"
            );
            let ctx = cache.build_context(0.5);
            assert!(ctx.contains("Aris"), "应包含 Aris: {}", ctx);
            assert!(ctx.contains("钢琴"), "应包含 钢琴: {}", ctx);
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_sqlite_migrate_from_sled() {
        // 验证 sled → SQLite 数据迁移 / Verify sled → SQLite data migration
        let sqlite_path = "./target/test_key_facts_migrate.db";
        let sled_path = "./target/test_key_facts_migrate";
        let bak_path = format!("{}.sled.bak", sled_path);

        // 清理 / Cleanup
        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(format!("{}-wal", sqlite_path));
        let _ = std::fs::remove_file(format!("{}-shm", sqlite_path));
        let _ = std::fs::remove_dir_all(sled_path);
        let _ = std::fs::remove_dir_all(&bak_path);

        // 1. 先用 sled 写入 / Write with sled first
        {
            let cache = KeyFactCache::open(sled_path).unwrap();
            cache.upsert("迁移测试", KeyFactCategory::Identity, 0.9, "sled");
            cache.flush();
        }

        // 2. 用 SQLite 打开 — 应自动迁移 / Open with SQLite — should auto-migrate
        {
            let cache = KeyFactCache::open_sqlite(sqlite_path).unwrap();
            assert_eq!(
                cache.total_count(),
                1,
                "迁移后应有 1 条 / should have 1 after migration"
            );
            let ctx = cache.build_context(0.5);
            assert!(ctx.contains("迁移测试"), "应包含迁移数据: {}", ctx);
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(format!("{}-wal", sqlite_path));
        let _ = std::fs::remove_file(format!("{}-shm", sqlite_path));
        let _ = std::fs::remove_dir_all(&bak_path);
    }

    #[test]
    fn test_sqlite_confirm_increases_confidence() {
        // 验证 SQLite 后端的 confirm 逻辑 / Verify confirm logic on SQLite backend
        let path = "./target/test_key_facts_confirm.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        {
            let cache = KeyFactCache::open_sqlite(path).unwrap();
            cache.upsert("确认测试", KeyFactCategory::Commitment, 0.6, "s1");
            cache.upsert("确认测试", KeyFactCategory::Commitment, 0.7, "s2");

            let results = cache.search("确认");
            assert_eq!(results.len(), 1);
            assert!(
                results[0].confidence > 0.7,
                "二次确认应提升置信度 / confirm should increase confidence"
            );
            assert_eq!(results[0].confirmed_count, 2);
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }
}
