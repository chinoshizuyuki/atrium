// SPDX-License-Identifier: MIT
//! 文件存储 — 用户上传文件管理（sled 命名 Tree + 磁盘文件）
//! File store — user-uploaded file management (sled named tree + disk files).
//!
//! 文件存储于磁盘，元数据存储于 sled 命名 Tree。
//! Files stored on disk, metadata in a sled named tree.
//!
//! Features:
//! - Upload with size/count limits (FIFO eviction)
//! - Text extraction for pdf/txt/md → FactStore + FTS5
//! - Cross-session persistence
//! - Hash-based deduplication
//!
//! # 重构说明 / Refactoring Note
//!
//! P1-3: 从独立 sled 实例（模式 B）重构为共享 Db + 命名 Tree（模式 A）。
//! P1-3: Refactored from independent sled instance (Pattern B) to shared Db + named tree (Pattern A).
//! 归属认知域：前额工具区（Prefrontal）。
//! Cognitive domain: Prefrontal.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 文件存储句柄 — 数字生命的工具记忆
/// File store handle — Tool memory of digital life.
///
/// 使用命名 Tree `"file_meta"` 存储文件元数据，
/// 共享前额工具区数据库实例，消除独立 sled 实例的开销。
pub struct FileStore {
    /// 文件元数据 Tree / File metadata tree
    tree: sled::Tree,
    /// 文件磁盘目录 / Disk directory for file storage
    files_dir: PathBuf,
    /// 总容量上限 (bytes) / Total capacity limit (bytes)
    max_total_size: u64,
    /// 单文件上限 (bytes) / Per-file size limit (bytes)
    max_file_size: u64,
    /// 最大文件数 / Maximum file count
    max_file_count: usize,
}

/// 文件元数据 / File metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMeta {
    /// 文件哈希 (SHA256 hex) / File hash (SHA256 hex)
    pub hash: String,
    /// 原始文件名 / Original filename
    pub original_name: String,
    /// MIME 类型 / MIME type
    pub mime_type: String,
    /// 文件大小 (bytes) / File size (bytes)
    pub size: u64,
    /// 磁盘相对路径 / Relative disk path
    pub disk_path: String,
    /// 是否已提取文本 / Whether text has been extracted
    pub text_extracted: bool,
    /// 提取的文本内容（仅文本类文件，截断至 4096 字节）
    /// Extracted text content (text files only, truncated to 4096 bytes)
    pub extracted_text: String,
    /// 上传时间戳 / Upload timestamp
    pub created_at: i64,
    /// 所属 session / Owning session ID
    pub session_id: String,
}

impl FileStore {
    /// 从共享数据库打开文件存储 / Open file store from shared database.
    ///
    /// 在给定的 sled::Db 上打开命名 Tree `"file_meta"`，
    /// 并在 `base_dir/files/` 下创建文件存储目录。
    ///
    /// Opens the named tree `"file_meta"` on the given sled::Db,
    /// and creates the file storage directory at `base_dir/files/`.
    ///
    /// # 参数 / Parameters
    ///
    /// - `db` — 共享的 sled 数据库引用（通常为前额工具区 Prefrontal Db）
    /// - `base_dir` — 基础目录路径，用于创建 `files/` 子目录
    pub fn open(db: &sled::Db, base_dir: &str) -> Result<Self, sled::Error> {
        let tree = db.open_tree("file_meta")?;
        let base = Path::new(base_dir);
        let files_dir = base.join("files");
        std::fs::create_dir_all(&files_dir).ok();

        Ok(Self {
            tree,
            files_dir,
            max_total_size: 100 * 1024 * 1024, // 100 MB
            max_file_size: 10 * 1024 * 1024,   // 10 MB per file
            max_file_count: 100,
        })
    }

    /// 创建内存模式（测试用）/ Create in-memory mode for testing.
    ///
    /// 使用临时目录存储文件，临时 sled 实例存储元数据。
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("file_store: temporary db init failed");
        let tmp_dir = tempfile::tempdir().expect("file_store: temp dir failed");
        // 将临时目录路径持久化到结构体中（tempdir 被 leak 以保持目录存活）
        let files_dir = tmp_dir.path().join("files");
        std::fs::create_dir_all(&files_dir).ok();
        let tree = db
            .open_tree("file_meta")
            .expect("file_store: open tree failed");
        std::mem::forget(tmp_dir); // 防止临时目录被过早清理 / Prevent premature cleanup
        Self {
            tree,
            files_dir,
            max_total_size: 100 * 1024 * 1024,
            max_file_size: 10 * 1024 * 1024,
            max_file_count: 100,
        }
    }

    /// 查询总数 / Total file count.
    pub fn count(&self) -> usize {
        self.tree.len()
    }

    /// 查询总大小 / Total storage size in bytes.
    pub fn total_size(&self) -> u64 {
        self.tree
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| bincode::deserialize::<FileMeta>(&v).ok())
            .map(|m| m.size)
            .sum()
    }

    /// 存储文件 / Store a file.
    pub fn store(
        &self,
        data: &[u8],
        original_name: &str,
        mime_type: &str,
        session_id: &str,
    ) -> Result<FileMeta, crate::store_core::StoreError> {
        use sha2::{Digest, Sha256};

        if data.len() as u64 > self.max_file_size {
            return Err(crate::store_core::StoreError::Io(format!(
                "file too large: {} bytes",
                data.len()
            )));
        }

        // 哈希 / Hash computation
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hex::encode(hasher.finalize())
        };

        // 检查重复 / Dedup check
        if let Some(existing) = self.get_meta(&hash) {
            return Ok(existing);
        }

        // 容量控制：FIFO 淘汰 / Capacity control: FIFO eviction
        self.evict_if_needed(data.len() as u64);

        // 写磁盘 / Write to disk
        let ext = Path::new(original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let disk_name = format!("{}.{}", &hash[..16], ext);
        let disk_path = self.files_dir.join(&disk_name);
        std::fs::write(&disk_path, data)
            .map_err(|e| crate::store_core::StoreError::Io(e.to_string()))?;

        // 文本提取（仅文本类）/ Text extraction (text files only)
        let (text_extracted, extracted_text) =
            Self::try_extract_text(data, original_name, mime_type);

        let meta = FileMeta {
            hash: hash.clone(),
            original_name: original_name.to_string(),
            mime_type: mime_type.to_string(),
            size: data.len() as u64,
            disk_path: disk_name,
            text_extracted,
            extracted_text,
            created_at: chrono::Utc::now().timestamp(),
            session_id: session_id.to_string(),
        };

        let val = bincode::serialize(&meta).expect("file meta serialize");
        self.tree
            .insert(hash.as_bytes(), val)
            .map_err(|e| crate::store_core::StoreError::Sled(e.to_string()))?;
        self.tree.flush().ok();

        Ok(meta)
    }

    /// 获取元数据 / Get file metadata by hash.
    pub fn get_meta(&self, hash: &str) -> Option<FileMeta> {
        self.tree
            .get(hash.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| bincode::deserialize::<FileMeta>(&v).ok())
    }

    /// 列出所有文件 / List all files.
    pub fn list(&self) -> Vec<FileMeta> {
        self.tree
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_, v)| bincode::deserialize::<FileMeta>(&v).ok())
            .collect()
    }

    /// 按 session 列出 / List files by session.
    pub fn list_by_session(&self, session_id: &str) -> Vec<FileMeta> {
        self.list()
            .into_iter()
            .filter(|m| m.session_id == session_id)
            .collect()
    }

    /// FIFO 淘汰：移除最旧文件直到满足容量和数量限制
    /// FIFO eviction: remove oldest files until capacity and count limits are met.
    fn evict_if_needed(&self, incoming_size: u64) {
        let mut metas: Vec<FileMeta> = self.list();
        metas.sort_by_key(|m| m.created_at);

        while self.count() >= self.max_file_count
            || self.total_size() + incoming_size > self.max_total_size
        {
            if let Some(oldest) = metas.first() {
                // 删除磁盘文件 / Delete disk file
                let disk_path = self.files_dir.join(&oldest.disk_path);
                std::fs::remove_file(&disk_path).ok();
                // 删除元数据 / Delete metadata
                self.tree.remove(oldest.hash.as_bytes()).ok();
                metas.remove(0);
            } else {
                break;
            }
        }
    }

    /// 尝试提取文本内容 / Try to extract text content from file data.
    fn try_extract_text(data: &[u8], name: &str, mime: &str) -> (bool, String) {
        let is_text = mime.starts_with("text/")
            || name.ends_with(".txt")
            || name.ends_with(".md")
            || name.ends_with(".json")
            || name.ends_with(".csv")
            || name.ends_with(".rs")
            || name.ends_with(".py")
            || name.ends_with(".js")
            || name.ends_with(".toml")
            || name.ends_with(".yaml")
            || name.ends_with(".log");

        if !is_text {
            return (false, String::new());
        }

        match std::str::from_utf8(data) {
            Ok(s) => {
                let truncated: String = s.chars().take(4096).collect();
                (!truncated.is_empty(), truncated)
            }
            Err(_) => (false, String::new()),
        }
    }
}

// 统一使用 store_core::StoreError / Unified StoreError from store_core

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let store = FileStore::open_in_memory();

        let meta = store
            .store(b"hello world", "test.txt", "text/plain", "s1")
            .unwrap();
        assert_eq!(meta.original_name, "test.txt");
        assert!(meta.text_extracted);
        assert_eq!(meta.extracted_text, "hello world");
        assert_eq!(store.count(), 1);

        let loaded = store.get_meta(&meta.hash).unwrap();
        assert_eq!(loaded.original_name, "test.txt");
    }

    #[test]
    fn test_dedup() {
        let store = FileStore::open_in_memory();

        let m1 = store.store(b"abc", "a.txt", "text/plain", "s1").unwrap();
        let m2 = store.store(b"abc", "b.txt", "text/plain", "s1").unwrap();
        assert_eq!(m1.hash, m2.hash);
        assert_eq!(store.count(), 1); // 去重
    }

    #[test]
    fn test_text_extraction() {
        let store = FileStore::open_in_memory();

        let meta = store
            .store(b"fn main() {}", "hello.rs", "text/plain", "s1")
            .unwrap();
        assert!(meta.text_extracted);
        assert!(meta.extracted_text.contains("fn main()"));
    }

    #[test]
    fn test_non_text_skipped() {
        let store = FileStore::open_in_memory();

        let meta = store
            .store(&[0u8; 100], "data.bin", "application/octet-stream", "s1")
            .unwrap();
        assert!(!meta.text_extracted);
        assert!(meta.extracted_text.is_empty());
    }
}
