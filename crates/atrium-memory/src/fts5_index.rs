// SPDX-License-Identifier: MIT
//! FTS5 全文索引
//! FTS5 full-text search index.

use rusqlite::{params, Connection};
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
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
 content,
 source UNINDEXED,
 tokenize='unicode61'
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
        // 先尝试 FTS5 MATCH（对 ASCII/已分词文本有 bm25 排名）
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
                // MATCH 无结果时回退到 LIKE（处理 CJK 等 unicode61 不支持的场景）
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
    // 单字或无空格查询：不加引号，让 FTS5 做词条/前缀匹配
    // 含空格查询：用双引号包裹做精确短语搜索
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
}
