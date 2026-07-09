// SPDX-License-Identifier: MIT
//! FTS5 全文索引
//! FTS5 full-text search index.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug)]
pub enum Fts5Error {
    Db(String),
    Query(String),
}

impl std::fmt::Display for Fts5Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(msg) => write!(f, "Database error: {}", msg),
            Self::Query(msg) => write!(f, "Query Error: {}", msg),
        }
    }
}

impl std::error::Error for Fts5Error {}

impl From<rusqlite::Error> for Fts5Error {
    fn from(e: rusqlite::Error) -> Self {
        Fts5Error::Db(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Fts5Error>;

#[derive(Debug, Clone)]
pub struct Fts5Result {
    pub rowid: i64,
    pub content: String,
    pub rank: f64,
}

pub struct Fts5Index {
    conn: Connection,
}

impl Fts5Index {
    pub fn open(db_path: &str) -> Result<Self> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Fts5Error::Db(format!("Failed to create directory: {}", e)))?;
        }
        let conn = Connection::open(db_path)?;
        // 迁移旧 unicode61 表到 trigram 分词器 / Migrate old unicode61 table to trigram tokenizer
        migrate_from_unicode61(&conn)?;
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
 content,
 source UNINDEXED,
 tokenize='trigram'
 );",
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, content: &str, source: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO memory_fts (content, source) VALUES (?1, ?2)",
            params![content, source],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_batch(&self, entries: &[(&str, &str)]) -> Result<Vec<i64>> {
        let tx = self.conn.unchecked_transaction()?;
        let mut ids = Vec::new();
        for (content, source) in entries {
            tx.execute(
                "INSERT INTO memory_fts (content, source) VALUES (?1, ?2)",
                params![content, source],
            )?;
            ids.push(tx.last_insert_rowid());
        }
        tx.commit()?;
        Ok(ids)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Fts5Result>> {
        let sanitized = sanitize_fts5_query(query);
        // FTS5 MATCH 主路径 — trigram 支持中文 CJK 子串匹配（>= 3 字符走 bm25 排名）
        // FTS5 MATCH main path — trigram supports CJK substring matching (>= 3 chars via bm25 ranking)
        let match_sql = "SELECT rowid, content, bm25(memory_fts) as rank FROM memory_fts
 WHERE memory_fts MATCH ?1
 ORDER BY rank
 LIMIT ?2";
        let results = self.conn.prepare(match_sql).and_then(|mut stmt| {
            stmt.query_map(params![&sanitized, limit as i64], |row| {
                Ok(Fts5Result {
                    rowid: row.get(0)?,
                    content: row.get(1)?,
                    rank: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
        });
        match results {
            Ok(ref r) if !r.is_empty() => Ok(r.clone()),
            _ => {
                // 最终兜底：trigram 对 < 3 字符查询无法分词，LIKE 兜底（trigram 索引优化 LIKE，非全表扫描）
                // Final fallback: trigram cannot tokenize < 3 char queries; LIKE is optimized by trigram index, not a full scan
                let pattern = format!("%{}%", query);
                let mut stmt = self.conn.prepare(
                    "SELECT rowid, content, 0.0 as rank FROM memory_fts
 WHERE content LIKE ?1
 LIMIT ?2",
                )?;
                let results = stmt
                    .query_map(params![pattern, limit as i64], |row| {
                        Ok(Fts5Result {
                            rowid: row.get(0)?,
                            content: row.get(1)?,
                            rank: row.get(2)?,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(results)
            }
        }
    }

    pub fn delete(&self, rowid: i64) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM memory_fts WHERE rowid = ?1", params![rowid])?
            > 0)
    }

    pub fn clear(&self) -> Result<()> {
        self.conn.execute_batch("DELETE FROM memory_fts")?;
        Ok(())
    }

    pub fn count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM memory_fts", [], |r| r.get(0))?)
    }
}

/// 迁移旧 unicode61 FTS5 表到 trigram 分词器
/// Migrate old unicode61 FTS5 table to trigram tokenizer.
///
/// 检测现有 memory_fts 表的建表 SQL，若包含 `unicode61` 则：
///
/// 1. 备份全部记忆内容（rowid + content + source）
/// 2. DROP 旧表
/// 3. CREATE 新表（trigram 分词器）
/// 4. 重新插入全部记忆
///
/// 整个迁移在单事务中执行，失败则回滚保留旧表。
///
/// Detects the CREATE SQL of the existing memory_fts table. If it contains `unicode61`:
///
/// 1. Backs up all memory content (rowid + content + source)
/// 2. Drops the old table
/// 3. Creates a new table with trigram tokenizer
/// 4. Reinserts all memory content
///
/// The entire migration runs in a single transaction; on failure, rollback preserves the old table.
fn migrate_from_unicode61(conn: &Connection) -> Result<()> {
    // 查询现有表的建表 SQL / Query the CREATE statement of the existing table
    let existing_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='memory_fts'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    // 检测是否为旧 unicode61 分词器 / Detect if old unicode61 tokenizer is used
    let needs_migration = existing_sql
        .as_ref()
        .map(|sql| sql.to_lowercase().contains("unicode61"))
        .unwrap_or(false);

    if !needs_migration {
        return Ok(());
    }

    tracing::info!(
        target: "atrium_memory::fts5",
        "检测到旧 unicode61 FTS5 表，开始迁移到 trigram / Detected old unicode61 FTS5 table, migrating to trigram"
    );

    // 备份所有记忆内容（保留 rowid 以维持外部引用一致性）
    // Back up all memory content (preserve rowid for external reference consistency)
    let backup: Vec<(i64, String, String)> = {
        let mut stmt = conn.prepare("SELECT rowid, content, source FROM memory_fts")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    let backup_count = backup.len();

    // 整个迁移在单事务中执行，失败则回滚保留旧表
    // Entire migration in a single transaction; on failure, rollback preserves the old table
    {
        let tx = conn.unchecked_transaction()?;
        // DROP 旧表（含影子表自动清理）/ Drop old table (shadow tables auto-cleaned)
        tx.execute("DROP TABLE memory_fts", [])?;
        // CREATE 新表（trigram 分词器）/ Create new table with trigram tokenizer
        tx.execute_batch(
            "CREATE VIRTUAL TABLE memory_fts USING fts5(
                content,
                source UNINDEXED,
                tokenize='trigram'
            );",
        )?;
        // 重新插入全部记忆内容 / Reinsert all memory content
        if !backup.is_empty() {
            let mut stmt =
                tx.prepare("INSERT INTO memory_fts (rowid, content, source) VALUES (?1, ?2, ?3)")?;
            for (rowid, content, source) in &backup {
                stmt.execute(params![rowid, content, source])?;
            }
        }
        tx.commit()?;
    }

    tracing::info!(
        target: "atrium_memory::fts5",
        backup_count,
        "FTS5 表迁移完成，已重新索引 {} 条记忆 / FTS5 table migration complete, reindexed {} memories",
        backup_count, backup_count
    );

    Ok(())
}

/// 转义 FTS5 特殊字符，构建安全的 MATCH 查询字符串
fn sanitize_fts5_query(query: &str) -> String {
    // 移除 FTS5 特殊字符: * " ^ ( ) - : + [ ] ~ \
    let sanitized: String = query
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '*' | '"' | '^' | '(' | ')' | '-' | ':' | '+' | '[' | ']' | '~' | '\\'
            )
        })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        return "\"noresults\"".to_string();
    }
    // 单字或无空格查询：不加引号，trigram 做子串匹配（>= 3 字符有效）
    // 含空格查询：用双引号包裹做精确短语子串搜索
    // Single-token queries: unquoted, trigram does substring matching (effective for >= 3 chars)
    // Multi-token queries: quoted for exact phrase substring search
    if trimmed.contains(' ') {
        format!("\"{}\"", trimmed)
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_index() -> Fts5Index {
        Fts5Index::open(":memory:").unwrap()
    }

    #[test]
    fn test_insert_and_count() {
        let idx = test_index();
        idx.insert("主人喜欢编程", "chat").unwrap();
        idx.insert("主人喜欢Rust", "chat").unwrap();
        assert_eq!(idx.count().unwrap(), 2);
    }

    #[test]
    fn test_search() {
        let idx = test_index();
        idx.insert_batch(&[
            ("主人喜欢编程", "chat"),
            ("主人喜欢Rust", "chat"),
            ("Atrium是AI", "system"),
        ])
        .unwrap();
        let r = idx.search("喜欢", 10).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_delete() {
        let idx = test_index();
        idx.insert("test", "src").unwrap();
        assert_eq!(idx.count().unwrap(), 1);
        idx.delete(1).unwrap();
        assert_eq!(idx.count().unwrap(), 0);
    }

    #[test]
    fn test_empty_search() {
        let idx = test_index();
        assert!(idx.search("nothing", 10).unwrap().is_empty());
    }

    #[test]
    fn test_search_chinese_trigram() {
        let idx = test_index();
        idx.insert_batch(&[
            ("我今天吃了火锅", "chat"),
            ("昨天吃了面条", "chat"),
            ("今天天气很好", "system"),
        ])
        .unwrap();
        // 4 字符中文查询走 trigram MATCH 路径 / 4-char Chinese query via trigram MATCH
        let r = idx.search("吃了火锅", 10).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "我今天吃了火锅");
        // 3 字符中文查询走 trigram MATCH / 3-char Chinese query via trigram MATCH
        let r = idx.search("吃了面", 10).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "昨天吃了面条");
        // 2 字符查询 trigram 无法分词，LIKE 兜底 / 2-char query: trigram can't tokenize, LIKE fallback
        let r = idx.search("火锅", 10).unwrap();
        assert_eq!(r.len(), 1);
        // 2 字符查询 LIKE 兜底命中多条 / 2-char query via LIKE fallback hits multiple
        let r = idx.search("今天", 10).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_migrate_from_unicode61() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test_fts_migrate.db");
        let db_path_str = db_path.to_str().unwrap();

        // 步骤 1：创建旧 unicode61 表并插入数据 / Step 1: Create old unicode61 table with data
        {
            let conn = Connection::open(db_path_str).unwrap();
            conn.execute_batch(
                "CREATE VIRTUAL TABLE memory_fts USING fts5(
                    content,
                    source UNINDEXED,
                    tokenize='unicode61'
                );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO memory_fts (content, source) VALUES (?1, ?2)",
                params!["我今天吃了火锅", "chat"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO memory_fts (content, source) VALUES (?1, ?2)",
                params!["昨天吃了面条", "chat"],
            )
            .unwrap();
        }

        // 步骤 2：打开 Fts5Index — 触发迁移 / Step 2: Open Fts5Index — triggers migration
        let idx = Fts5Index::open(db_path_str).unwrap();

        // 步骤 3：验证数据完整保留 / Step 3: Verify data integrity preserved
        assert_eq!(idx.count().unwrap(), 2);

        // 步骤 4：验证迁移后使用 trigram — 3 字符查询走 MATCH / Verify trigram after migration
        let r = idx.search("吃了火锅", 10).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "我今天吃了火锅");

        // 步骤 5：验证 2 字符查询走 LIKE 兜底 / Verify 2-char query via LIKE fallback
        let r = idx.search("火锅", 10).unwrap();
        assert_eq!(r.len(), 1);
    }
}
