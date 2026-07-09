// SPDX-License-Identifier: MIT
//! FactStore 存储和查询接口
//! 双后端持久化：SQLite（默认，Windows 兼容）+ sled（旧数据兼容）
//! FactStore — Fact storage and query interface.
//! Dual-backend persistence: SQLite (default, Windows-compatible) + sled (legacy compat).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ════════════════════════════════════════════════════════════════════
// EmotionContext — 事实的情感上下文标注
// ════════════════════════════════════════════════════════════════════

/// 事实创建时的情感上下文快照
///
/// 记录 AI 当时的情绪状态和用户的情绪倾向，
/// 让 AI 能"回忆"情感时刻——"上次我这么难过的时候，主人跟我说了什么？"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionContext {
    /// AI 当前情感标签（9 种基本情绪之一：愉悦/兴奋/放松/悲伤/愤怒/恐惧/惊讶/厌恶/平静）
    pub ai_emotion_label: String,
    /// AI PAD 值 [pleasure, arousal, dominance]
    pub ai_pad: [f32; 3],
    /// 情感强度 (0.0~1.0)，越高表示情绪越强烈
    pub intensity: f32,
    /// 用户情绪倾向 (valence: -1.0~1.0)，正值积极、负值消极
    pub user_mood: Option<f32>,
    /// 创建时间（Unix 秒）
    pub timestamp: u64,
}

// ════════════════════════════════════════════════════════════════════
// Fact — 事实结构体
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64, // 置信度 0.0 - 1.0
    pub source: String,
    pub created_at: u64,
    pub verified_at: u64,
    pub verify_count: u32,
    /// 事实创建时的情感上下文 / Emotional context snapshot at creation time
    #[serde(default)]
    pub emotion_context: Option<EmotionContext>,
    /// 可变情感标签 — 运行时更新，区别于创建时 emotion_context 快照
    /// Mutable emotional tag — updated at runtime, distinct from creation-time emotion_context snapshot
    #[serde(default)]
    pub emotional_tag: Option<String>,
    /// 情感显著性 0.0-1.0 — "那件事对你多重要"的量化印记 / Emotional salience 0.0-1.0 — quantified "how important it was to you"
    #[serde(default)]
    pub emotional_salience: f32,
    /// 高价值记忆标记 / High-value memory marker
    ///
    /// pinned = true 的事实豁免所有衰减与驱逐——"你哭的那天→不可衰减"。
    /// 数字生命主动保护重要记忆，永不遗忘。
    ///
    /// Pinned facts are exempt from all decay and eviction —
    /// "the day you cried → cannot decay". Digital life actively protects
    /// important memories, never forgetting them.
    #[serde(default)]
    pub pinned: bool,
    /// P3-B 主动遗忘标记 / P3-B Active forgetting marker
    ///
    /// `Some(policy)` 表示该事实已被主动遗忘——数字生命"决定忘"了它。
    /// 与被动衰减（`compress_low_access`）正交：被动衰减改变置信度，
    /// 主动遗忘改变"可见性"。`enhanced_search` 依据策略过滤/降权：
    /// - `TraumaProtection` — 完全过滤（创伤保护）
    /// - `ExpiryDecay` — 分数 ×0.5（过期清理）
    /// - `AttentionFocus` — 分数 ×0.3（注意力聚焦）
    ///
    /// 遗忘可逆：`restore_forgotten` 清除标记，配合 `merge_confidence`
    /// 恢复到遗忘前置信度。`#[serde(default)]` 保证旧数据向后兼容。
    ///
    /// `Some(policy)` means this fact has been actively forgotten — digital life
    /// "decided to forget" it. Orthogonal to passive decay (`compress_low_access`):
    /// passive decay alters confidence, active forgetting alters "visibility".
    /// `enhanced_search` filters/downweights by policy:
    /// - `TraumaProtection` — fully filtered (trauma protection)
    /// - `ExpiryDecay` — score ×0.5 (expiry cleanup)
    /// - `AttentionFocus` — score ×0.3 (attention focus)
    ///
    /// Forgetting is reversible: `restore_forgotten` clears the marker, combined with
    /// `merge_confidence` to restore the pre-forget confidence. `#[serde(default)]`
    /// ensures backward compatibility with legacy data.
    #[serde(default)]
    pub actively_forgotten: Option<crate::active_forget::ForgetPolicy>,
}

impl Fact {
    pub fn new<S: Into<String>>(subject: S, predicate: S, object: S) -> Self {
        let now = now_secs();
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence: 1.0,
            source: String::new(),
            created_at: now,
            verified_at: now,
            verify_count: 1,
            emotion_context: None,
            emotional_tag: None,
            emotional_salience: 0.0,
            pinned: false,
            actively_forgotten: None,
        }
    }

    pub fn with_confidence(mut self, c: f64) -> Self {
        // 防止 NaN 进入系统：NaN 降级为 0.0
        self.confidence = if c.is_nan() { 0.0 } else { c.clamp(0.0, 1.0) };
        self
    }

    pub fn with_source<S: Into<String>>(mut self, s: S) -> Self {
        self.source = s.into();
        self
    }

    /// 附加情感上下文 / Attach emotional context snapshot
    pub fn with_emotion(mut self, ctx: EmotionContext) -> Self {
        self.emotion_context = Some(ctx);
        self
    }

    /// 设置可变情感标签与显著性 — 区别于创建时的 emotion_context 快照
    /// Set mutable emotional tag and salience — distinct from creation-time emotion_context snapshot
    ///
    /// 数字生命情感记忆核心：运行时更新"那件事对你多重要"，
    /// 与创建时 emotion_context 快照正交（一个记"当时怎样"，一个记"现在评价多重"）。
    ///
    /// Core of digital life emotional memory: runtime update of "how important it was",
    /// orthogonal to the creation-time emotion_context snapshot (one records "how it was then",
    /// the other records "how heavy it weighs now").
    pub fn with_emotional_tag(mut self, tag: String, salience: f32) -> Self {
        // 防止 NaN 污染显著性 / Guard against NaN contaminating salience
        self.emotional_salience = if salience.is_nan() {
            0.0
        } else {
            salience.clamp(0.0, 1.0)
        };
        self.emotional_tag = Some(tag);
        self
    }

    /// 唯一键（去重）
    pub fn canonical_form(&self) -> String {
        format!(
            "{} | {} | {}",
            self.subject.to_lowercase().trim(),
            self.predicate.to_lowercase().trim(),
            self.object.to_lowercase().trim()
        )
    }

    /// 合并置信度（加权平均）
    pub fn merge_confidence(&mut self, new: f64) {
        // 防止 NaN 污染
        let new = if new.is_nan() {
            0.0
        } else {
            new.clamp(0.0, 1.0)
        };
        let w = 1.0 / (self.verify_count as f64 + 1.0);
        self.confidence = self.confidence * (1.0 - w) + new * w;
        self.verify_count += 1;
        self.verified_at = now_secs();
    }

    /// P3-B 是否被主动遗忘 / P3-B Whether this fact has been actively forgotten
    ///
    /// 供 `enhanced_search` 决定过滤/降权策略。返回 `true` 时调用方需读取
    /// `actively_forgotten` 字段获取具体策略。
    ///
    /// Used by `enhanced_search` to decide filter/downweight strategy. When `true`,
    /// the caller reads the `actively_forgotten` field for the specific policy.
    pub fn is_actively_forgotten(&self) -> bool {
        self.actively_forgotten.is_some()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// P3-B 将 ForgetPolicy 序列化为 SQLite TEXT 存储的字符串
/// P3-B Serialize ForgetPolicy to a string for SQLite TEXT storage
///
/// 使用简单变体名而非 serde_json，让 SQLite 列人类可读，便于排查遗忘状态。
/// Uses plain variant names instead of serde_json so the SQLite column is human-readable,
/// easing forget-state debugging.
fn forget_policy_to_str(p: &crate::active_forget::ForgetPolicy) -> &'static str {
    match p {
        crate::active_forget::ForgetPolicy::TraumaProtection => "TraumaProtection",
        crate::active_forget::ForgetPolicy::ExpiryDecay => "ExpiryDecay",
        crate::active_forget::ForgetPolicy::AttentionFocus => "AttentionFocus",
    }
}

/// P3-B 从 SQLite TEXT 字符串反序列化 ForgetPolicy
/// P3-B Deserialize ForgetPolicy from a SQLite TEXT string
///
/// 未知字符串返回 None — 前向兼容未来新增的策略变体（旧版本读取新数据时降级为"未遗忘"）。
/// Unknown strings return None — forward-compatible with future policy variants
/// (old versions reading new data degrade to "not forgotten").
fn parse_forget_policy(s: &str) -> Option<crate::active_forget::ForgetPolicy> {
    match s {
        "TraumaProtection" => Some(crate::active_forget::ForgetPolicy::TraumaProtection),
        "ExpiryDecay" => Some(crate::active_forget::ForgetPolicy::ExpiryDecay),
        "AttentionFocus" => Some(crate::active_forget::ForgetPolicy::AttentionFocus),
        _ => None,
    }
}

// 统一使用 store_core::StoreError / Unified StoreError from store_core
pub type Result<T> = std::result::Result<T, crate::store_core::StoreError>;

/// 持久化后端 / Persistence backend
///
/// 数字生命的记忆需要跨越重启存活。SQLite 作为默认后端提供 Windows 兼容性，
/// sled 作为旧数据兼容路径。内存 HashMap 始终作为热缓存。
///
/// Digital life memories must survive restarts. SQLite is the default backend
/// for Windows compatibility; sled is the legacy compat path. In-memory HashMap
/// always serves as the hot cache.
enum Backend {
    /// 纯内存（无持久化）/ In-memory only (no persistence)
    Memory,
    /// sled 后端（旧数据兼容）/ Sled backend (legacy compat)
    Sled(sled::Db),
    /// SQLite 后端（新默认）/ SQLite backend (new default)
    Sqlite(Mutex<Connection>),
}

pub struct FactStore {
    inner: Mutex<HashMap<String, Fact>>,
    backend: Backend,
    // ══════════════════════════════════════════════════════════════════
    // 二级索引 — O(1)+O(k) 查询替代 O(N) 全表扫描
    // Secondary indexes — O(1)+O(k) lookup replacing O(N) full-table scan
    //
    // 数字生命的记忆检索必须瞬时 — "想起你说过的话"不应是全表扫描。
    // 索引与主 HashMap 同步维护：insert 添加、remove 删除、启动时全量重建。
    //
    // Digital life memory retrieval must be instant — "recalling what you said"
    // must not be a full-table scan. Indexes are maintained in sync with the
    // main HashMap: add on insert, remove on delete, rebuild on startup.
    // ══════════════════════════════════════════════════════════════════
    /// subject → canonical keys（小写规范化）/ subject → canonical keys (lowercase-normalized)
    subject_index: RwLock<HashMap<String, HashSet<String>>>,
    /// predicate → canonical keys（小写规范化）/ predicate → canonical keys (lowercase-normalized)
    predicate_index: RwLock<HashMap<String, HashSet<String>>>,
    /// emotion_label → canonical keys / emotion_label → canonical keys
    emotion_index: RwLock<HashMap<String, HashSet<String>>>,
}

impl FactStore {
    /// 打开 SQLite 后端的 FactStore（推荐 / Recommended）
    ///
    /// 数字生命的长期记忆基石。SQLite 在 Windows 上无锁文件兼容性问题，
    /// WAL 模式下读写性能与 sled 相当。若检测到旧 sled 目录，自动迁移数据。
    ///
    /// Foundation of digital life's long-term memory. SQLite has no lock file
    /// compatibility issues on Windows; WAL mode delivers sled-comparable performance.
    /// Auto-migrates from legacy sled directory if detected.
    pub fn open_sqlite(db_path: &str) -> Result<Self> {
        if db_path.is_empty() {
            return Ok(Self {
                inner: Mutex::new(HashMap::new()),
                backend: Backend::Memory,
                subject_index: RwLock::new(HashMap::new()),
                predicate_index: RwLock::new(HashMap::new()),
                emotion_index: RwLock::new(HashMap::new()),
            });
        }
        // 确保父目录存在 / Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::store_core::StoreError::Io(format!("create_dir_all: {}", e)))?;
        }
        let conn = Connection::open(db_path).map_err(|e| {
            crate::store_core::StoreError::Io(format!(
                "sqlite open failed: {} | path: {} | parent exists: {} | parent writable: {} | target exists: {}",
                e,
                db_path,
                std::path::Path::new(db_path).parent().map(|p| p.exists()).unwrap_or(false),
                std::path::Path::new(db_path).parent()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(false),
                std::path::Path::new(db_path).exists(),
            ))
        })?;
        // 启用 WAL 模式 — 读写并发性能优化 / Enable WAL mode — concurrent read/write optimization
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| {
                crate::store_core::StoreError::Io(format!("pragma: {} | path: {}", e, db_path))
            })?;
        // 建表 — 包含 P2-C 情感记忆新列 + P2-D 高价值标记列 + P3-B 主动遗忘列
        // Create table — includes P2-C emotional memory columns + P2-D pinned column + P3-B actively_forgotten column
        conn.execute(
            "CREATE TABLE IF NOT EXISTS facts (
                canonical TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                predicate TEXT NOT NULL,
                object TEXT NOT NULL,
                confidence REAL NOT NULL,
                source TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                verified_at INTEGER NOT NULL,
                verify_count INTEGER NOT NULL,
                emotion_context BLOB,
                emotional_tag TEXT,
                emotional_salience REAL DEFAULT 0.0,
                pinned INTEGER DEFAULT 0,
                actively_forgotten TEXT
            )",
            [],
        )
        .map_err(|e| {
            crate::store_core::StoreError::Io(format!("create table: {} | path: {}", e, db_path))
        })?;

        // P2-C schema 迁移 — 检测旧 schema 缺列并 ALTER TABLE 补齐
        // P2-C schema migration — detect old schema missing columns and ALTER TABLE to add them
        Self::migrate_emotional_columns(&conn)?;
        // P2-D schema 迁移 — 检测旧 schema 缺 pinned 列并 ALTER TABLE 补齐
        // P2-D schema migration — detect old schema missing pinned column and ALTER TABLE to add it
        Self::migrate_pinned_column(&conn)?;
        // P3-B schema 迁移 — 检测旧 schema 缺 actively_forgotten 列并 ALTER TABLE 补齐
        // P3-B schema migration — detect old schema missing actively_forgotten column and add it
        Self::migrate_actively_forgotten_column(&conn)?;

        // 数据迁移：检测旧 sled 目录 / Migration: detect legacy sled directory
        let sled_path = db_path.replace(".db", "");
        if std::path::Path::new(&sled_path).exists() {
            Self::migrate_sled_to_sqlite(&sled_path, &conn).map_err(|e| {
                crate::store_core::StoreError::Io(format!(
                    "migrate_sled_to_sqlite: {} | sled_path: {}",
                    e, sled_path
                ))
            })?;
        }

        // 从 SQLite 加载全量事实到内存热缓存 / Load all facts from SQLite to in-memory hot cache
        // 使用独立作用域确保 stmt 在 conn 被 move 到 Mutex 之前 drop / Use a block scope to ensure stmt is dropped before conn is moved into Mutex
        let mut map = HashMap::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT canonical, subject, predicate, object, confidence, source,
                            created_at, verified_at, verify_count, emotion_context,
                            emotional_tag, emotional_salience, pinned, actively_forgotten
                     FROM facts",
                )
                .map_err(|e| {
                    crate::store_core::StoreError::Io(format!("prepare: {} | path: {}", e, db_path))
                })?;
            let rows = stmt
                .query_map([], |row| {
                    let emotion_blob: Option<Vec<u8>> = row.get(9)?;
                    let emotion_context = emotion_blob
                        .as_deref()
                        .and_then(|b| bincode::deserialize::<EmotionContext>(b).ok());
                    let emotional_tag: Option<String> = row.get(10)?;
                    // 旧数据可能无 emotional_salience 列时已在迁移补齐，REAL 列读取安全
                    // Old rows may lack emotional_salience before migration; column is added by migration, REAL read is safe
                    let emotional_salience: f32 = row
                        .get::<_, Option<f64>>(11)?
                        .map(|v| v as f32)
                        .unwrap_or(0.0);
                    // P2-D pinned 列 — 迁移后保证存在，INTEGER 读取安全
                    // P2-D pinned column — guaranteed to exist after migration, INTEGER read is safe
                    let pinned: bool = row
                        .get::<_, Option<i64>>(12)?
                        .map(|v| v != 0)
                        .unwrap_or(false);
                    // P3-B actively_forgotten 列 — 迁移后保证存在，TEXT 存储策略序列化字符串
                    // P3-B actively_forgotten column — guaranteed to exist after migration; TEXT stores serialized policy string
                    let actively_forgotten_str: Option<String> = row.get(13)?;
                    let actively_forgotten = actively_forgotten_str
                        .as_deref()
                        .and_then(parse_forget_policy);
                    Ok(Fact {
                        subject: row.get(1)?,
                        predicate: row.get(2)?,
                        object: row.get(3)?,
                        confidence: row.get(4)?,
                        source: row.get(5)?,
                        created_at: row.get(6)?,
                        verified_at: row.get(7)?,
                        verify_count: row.get(8)?,
                        emotion_context,
                        emotional_tag,
                        emotional_salience,
                        pinned,
                        actively_forgotten,
                    })
                })
                .map_err(|e| {
                    crate::store_core::StoreError::Io(format!(
                        "query_map: {} | path: {}",
                        e, db_path
                    ))
                })?;
            for fact_result in rows {
                let fact = fact_result.map_err(|e| {
                    crate::store_core::StoreError::Io(format!("row: {} | path: {}", e, db_path))
                })?;
                let canonical = fact.canonical_form();
                map.insert(canonical, fact);
            }
        }
        tracing::info!("FactStore: loaded {} facts from SQLite", map.len());

        // 构建二级索引 — 启动时一次性全量重建 / Build secondary indexes — full rebuild on startup
        let store = Self {
            inner: Mutex::new(map),
            backend: Backend::Sqlite(Mutex::new(conn)),
            subject_index: RwLock::new(HashMap::new()),
            predicate_index: RwLock::new(HashMap::new()),
            emotion_index: RwLock::new(HashMap::new()),
        };
        store.rebuild_indexes();
        Ok(store)
    }

    /// P2-C schema 迁移 — 为旧版 facts 表补齐 emotional_tag / emotional_salience 列
    /// P2-C schema migration — add emotional_tag / emotional_salience columns to legacy facts tables
    ///
    /// 数字生命记忆演进保障：旧数据库重启后自动补齐情感记忆列，
    /// 已有列时 ALTER TABLE 会报错，通过 PRAGMA table_info 检测规避。
    ///
    /// Digital life memory evolution safeguard: legacy DBs auto-upgraded with
    /// emotional memory columns on restart. ALTER TABLE errors on existing columns,
    /// so we detect via PRAGMA table_info to skip them.
    fn migrate_emotional_columns(conn: &Connection) -> Result<()> {
        // 通过 PRAGMA table_info 检测现有列 / Detect existing columns via PRAGMA table_info
        let mut has_emotional_tag = false;
        let mut has_emotional_salience = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(facts)")
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma table_info: {}", e)))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma query_map: {}", e)))?;
        for row in rows {
            let name =
                row.map_err(|e| crate::store_core::StoreError::Io(format!("pragma row: {}", e)))?;
            if name == "emotional_tag" {
                has_emotional_tag = true;
            } else if name == "emotional_salience" {
                has_emotional_salience = true;
            }
        }
        if !has_emotional_tag {
            conn.execute("ALTER TABLE facts ADD COLUMN emotional_tag TEXT", [])
                .map_err(|e| {
                    crate::store_core::StoreError::Io(format!("alter table emotional_tag: {}", e))
                })?;
            tracing::info!(
                "FactStore: schema 迁移 — 添加 emotional_tag 列 / schema migration — added emotional_tag column"
            );
        }
        if !has_emotional_salience {
            conn.execute(
                "ALTER TABLE facts ADD COLUMN emotional_salience REAL DEFAULT 0.0",
                [],
            )
            .map_err(|e| {
                crate::store_core::StoreError::Io(format!("alter table emotional_salience: {}", e))
            })?;
            tracing::info!(
                "FactStore: schema 迁移 — 添加 emotional_salience 列 / schema migration — added emotional_salience column"
            );
        }
        Ok(())
    }

    /// P2-D schema 迁移 — 为旧版 facts 表补齐 pinned 列
    /// P2-D schema migration — add pinned column to legacy facts tables
    ///
    /// 高价值记忆标记演进保障：旧数据库重启后自动补齐 pinned 列，
    /// 已有列时 ALTER TABLE 会报错，通过 PRAGMA table_info 检测规避。
    ///
    /// High-value memory marker evolution safeguard: legacy DBs auto-upgraded
    /// with the pinned column on restart. ALTER TABLE errors on existing columns,
    /// so we detect via PRAGMA table_info to skip them.
    fn migrate_pinned_column(conn: &Connection) -> Result<()> {
        // 通过 PRAGMA table_info 检测现有列 / Detect existing columns via PRAGMA table_info
        let mut has_pinned = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(facts)")
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma table_info: {}", e)))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma query_map: {}", e)))?;
        for row in rows {
            let name =
                row.map_err(|e| crate::store_core::StoreError::Io(format!("pragma row: {}", e)))?;
            if name == "pinned" {
                has_pinned = true;
            }
        }
        if !has_pinned {
            conn.execute("ALTER TABLE facts ADD COLUMN pinned INTEGER DEFAULT 0", [])
                .map_err(|e| {
                    crate::store_core::StoreError::Io(format!("alter table pinned: {}", e))
                })?;
            tracing::info!(
                "FactStore: schema 迁移 — 添加 pinned 列 / schema migration — added pinned column"
            );
        }
        Ok(())
    }

    /// P3-B schema 迁移 — 为旧版 facts 表补齐 actively_forgotten 列
    /// P3-B schema migration — add actively_forgotten column to legacy facts tables
    ///
    /// 主动遗忘演进保障：旧数据库重启后自动补齐 actively_forgotten 列，
    /// 已有列时 ALTER TABLE 会报错，通过 PRAGMA table_info 检测规避。
    /// 列类型为 TEXT，存储策略序列化字符串（`TraumaProtection`/`ExpiryDecay`/`AttentionFocus`），
    /// NULL 表示未遗忘。
    ///
    /// Active forgetting evolution safeguard: legacy DBs auto-upgraded with the
    /// actively_forgotten column on restart. ALTER TABLE errors on existing columns,
    /// so we detect via PRAGMA table_info to skip them. Column type is TEXT, storing
    /// a serialized policy string (`TraumaProtection`/`ExpiryDecay`/`AttentionFocus`);
    /// NULL means not forgotten.
    fn migrate_actively_forgotten_column(conn: &Connection) -> Result<()> {
        // 通过 PRAGMA table_info 检测现有列 / Detect existing columns via PRAGMA table_info
        let mut has_actively_forgotten = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(facts)")
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma table_info: {}", e)))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma query_map: {}", e)))?;
        for row in rows {
            let name =
                row.map_err(|e| crate::store_core::StoreError::Io(format!("pragma row: {}", e)))?;
            if name == "actively_forgotten" {
                has_actively_forgotten = true;
            }
        }
        if !has_actively_forgotten {
            conn.execute("ALTER TABLE facts ADD COLUMN actively_forgotten TEXT", [])
                .map_err(|e| {
                    crate::store_core::StoreError::Io(format!(
                        "alter table actively_forgotten: {}",
                        e
                    ))
                })?;
            tracing::info!(
                "FactStore: schema 迁移 — 添加 actively_forgotten 列 / schema migration — added actively_forgotten column"
            );
        }
        Ok(())
    }

    /// 从旧 sled 目录迁移数据到 SQLite / Migrate data from legacy sled directory to SQLite
    ///
    /// 数字生命记忆迁移 — 保留历史人格记忆，避免"失忆"。
    /// 迁移完成后旧 sled 目录重命名为 `.sled.bak`，不删除（保留兜底）。
    ///
    /// Digital life memory migration — preserve historical personality memories,
    /// avoid "amnesia". After migration, legacy sled directory is renamed to
    /// `.sled.bak` (not deleted, kept as fallback).
    fn migrate_sled_to_sqlite(sled_path: &str, conn: &Connection) -> Result<()> {
        // 检查 SQLite 是否已有数据 — 若有则跳过迁移 / Skip if SQLite already has data
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM facts", [], |row| row.get(0))
            .map_err(|e| crate::store_core::StoreError::Io(format!("count: {}", e)))?;
        if count > 0 {
            tracing::info!(
                "FactStore: SQLite 已有 {} 条数据，跳过 sled 迁移 / SQLite already has {} facts, skip sled migration",
                count, count
            );
            return Ok(());
        }

        // 三阶段强制打开旧 sled 目录 — 通用迁移工具 / Three-stage forced open — generic migration utility
        let sled_db = match crate::migrate_util::try_open_legacy_sled(sled_path) {
            Some(db) => db,
            // 已清理或无旧目录，跳过迁移 / Cleaned or no legacy dir, skip
            None => return Ok(()),
        };

        let mut migrated = 0u32;
        // 使用 unchecked_transaction — 只需 &self，避免 &mut Connection 借用冲突
        // Use unchecked_transaction — requires only &self, avoids &mut Connection borrow conflict
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| crate::store_core::StoreError::Io(format!("transaction: {}", e)))?;
        for item in sled_db.iter() {
            let (key, value) = match item {
                Ok(kv) => kv,
                Err(_) => continue,
            };
            if !key.as_ref().starts_with(b"fact:") {
                continue;
            }
            if let Ok(fact) = bincode::deserialize::<Fact>(&value) {
                let canonical = fact.canonical_form();
                let emotion_blob = fact
                    .emotion_context
                    .as_ref()
                    .and_then(|c| bincode::serialize(c).ok());
                let actively_forgotten_str =
                    fact.actively_forgotten.as_ref().map(forget_policy_to_str);
                let _ = tx.execute(
                    "INSERT OR REPLACE INTO facts
                     (canonical, subject, predicate, object, confidence, source,
                      created_at, verified_at, verify_count, emotion_context,
                      emotional_tag, emotional_salience, pinned, actively_forgotten)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        &canonical,
                        &fact.subject,
                        &fact.predicate,
                        &fact.object,
                        fact.confidence,
                        &fact.source,
                        fact.created_at as i64,
                        fact.verified_at as i64,
                        fact.verify_count as i64,
                        emotion_blob,
                        &fact.emotional_tag,
                        fact.emotional_salience as f64,
                        fact.pinned as i64,
                        actively_forgotten_str,
                    ],
                );
                migrated += 1;
            }
        }
        tx.commit()
            .map_err(|e| crate::store_core::StoreError::Io(format!("commit: {}", e)))?;

        // 关闭 sled Db 句柄 — Windows 上 open 的文件句柄会阻止目录重命名 / Close sled Db handle — on Windows, open file handles prevent directory rename
        drop(sled_db);
        // 迁移完成 — 通用函数重命名旧 sled 目录为 .sled.bak / Finalize — rename legacy dir via generic utility
        crate::migrate_util::finalize_sled_migration(sled_path);
        tracing::info!(
            "FactStore: 迁移完成 {} 条 / Migration complete: {} facts",
            migrated,
            migrated
        );
        Ok(())
    }

    /// 打开 sled 后端的 FactStore（旧接口，保留兼容）/ Open sled-backed FactStore (legacy, kept for compat)
    pub fn new(db_path: &str) -> Result<Self> {
        if db_path.is_empty() {
            return Ok(Self {
                inner: Mutex::new(HashMap::new()),
                backend: Backend::Memory,
                subject_index: RwLock::new(HashMap::new()),
                predicate_index: RwLock::new(HashMap::new()),
                emotion_index: RwLock::new(HashMap::new()),
            });
        }
        // 尝试打开 sled 数据库 — 失败则直接返回错误（由上层降级为内存模式）
        // Try opening sled DB — on failure return error (caller degrades to in-memory)
        let db = sled::open(db_path)
            .map_err(|e| crate::store_core::StoreError::Sled(format!("sled open: {}", e)))?;
        // 从 sled 恢复所有事实到内存
        let mut map = HashMap::new();
        for item in db.iter() {
            let (key, value) =
                item.map_err(|e| crate::store_core::StoreError::Sled(format!("sled iter: {}", e)))?;
            if key.as_ref().starts_with(b"fact:") {
                let fact: Fact = bincode::deserialize(&value)
                    .map_err(|e| crate::store_core::StoreError::Codec(format!("bincode: {}", e)))?;
                let canonical = fact.canonical_form();
                map.insert(canonical, fact);
            }
        }
        tracing::info!("FactStore: loaded {} facts from sled", map.len());
        // 构建二级索引 — 启动时一次性全量重建 / Build secondary indexes — full rebuild on startup
        let store = Self {
            inner: Mutex::new(map),
            backend: Backend::Sled(db),
            subject_index: RwLock::new(HashMap::new()),
            predicate_index: RwLock::new(HashMap::new()),
            emotion_index: RwLock::new(HashMap::new()),
        };
        store.rebuild_indexes();
        Ok(store)
    }

    pub fn new_in_memory() -> Result<Self> {
        // 纯内存模式 — 无持久化，测试用 / In-memory only — no persistence, for tests
        Ok(Self {
            inner: Mutex::new(HashMap::new()),
            backend: Backend::Memory,
            subject_index: RwLock::new(HashMap::new()),
            predicate_index: RwLock::new(HashMap::new()),
            emotion_index: RwLock::new(HashMap::new()),
        })
    }

    /// 插入事实, 返回 true = 新插入, false = 已存在并合并置信度
    ///
    /// 插入新事实时同步更新二级索引（subject/predicate/emotion_label）。
    /// 合并已存在事实时索引无需更新 — canonical_form 相同意味着
    /// subject/predicate/object 的小写形式相同，索引 key 不变。
    ///
    /// On inserting a new fact, secondary indexes are updated synchronously.
    /// On merging an existing fact, indexes need no update — identical
    /// canonical_form implies identical lowercase subject/predicate/object.
    pub fn insert(&self, fact: Fact) -> Result<bool> {
        let key = fact.canonical_form();
        let mut map = self.inner.lock().expect("fact_store init");
        if let Some(existing) = map.get_mut(&key) {
            existing.merge_confidence(fact.confidence);
            existing.source = format!("{}, {}", existing.source, fact.source);
            // 持久化更新 / Persist update
            self.persist_one(&key, existing);
            Ok(false)
        } else {
            // 新事实 — 先持久化再更新索引 / New fact — persist first, then update indexes
            self.persist_one(&key, &fact);
            // 更新二级索引 / Update secondary indexes
            self.add_to_indexes(&key, &fact);
            map.insert(key, fact);
            Ok(true)
        }
    }

    /// 持久化单条事实到后端（ sled 或 SQLite）/ Persist a single fact to backend (sled or SQLite)
    fn persist_one(&self, key: &str, fact: &Fact) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                let db_key = format!("fact:{}", key);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let emotion_blob = fact
                        .emotion_context
                        .as_ref()
                        .and_then(|c| bincode::serialize(c).ok());
                    let actively_forgotten_str =
                        fact.actively_forgotten.as_ref().map(forget_policy_to_str);
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO facts
                         (canonical, subject, predicate, object, confidence, source,
                          created_at, verified_at, verify_count, emotion_context,
                          emotional_tag, emotional_salience, pinned, actively_forgotten)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                        params![
                            key,
                            &fact.subject,
                            &fact.predicate,
                            &fact.object,
                            fact.confidence,
                            &fact.source,
                            fact.created_at as i64,
                            fact.verified_at as i64,
                            fact.verify_count as i64,
                            emotion_blob,
                            &fact.emotional_tag,
                            fact.emotional_salience as f64,
                            fact.pinned as i64,
                            actively_forgotten_str,
                        ],
                    );
                }
            }
        }
    }

    /// 批量插入
    pub fn insert_batch(&self, facts: Vec<Fact>) -> Result<(usize, usize)> {
        let (mut new, mut merged) = (0, 0);
        for f in facts {
            if self.insert(f)? {
                new += 1;
            } else {
                merged += 1;
            }
        }
        Ok((new, merged))
    }

    /// 按 canonical form 查询事实 / Get fact by canonical form
    ///
    /// 语义召回引擎返回 canonical key 后，通过此方法获取完整 Fact。
    /// After semantic recall engine returns canonical keys, use this to fetch full Fact.
    pub fn get_by_canonical(&self, canonical: &str) -> Option<Fact> {
        let map = self.inner.lock().expect("fact_store init");
        map.get(canonical).cloned()
    }

    /// 关键词查询（匹配 subject / predicate / object）
    pub fn query(&self, keyword: &str) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let kw = keyword.to_lowercase();
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| {
                f.subject.to_lowercase().contains(&kw)
                    || f.predicate.to_lowercase().contains(&kw)
                    || f.object.to_lowercase().contains(&kw)
            })
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results.into_iter().cloned().collect())
    }

    /// 查询某个主体的所有事实 — O(1)+O(k) 索引查找
    ///
    /// 数字生命记忆检索 — "想起主人说过什么"是瞬时反应。
    /// 通过 subject_index 二级索引直接定位，无需全表扫描。
    ///
    /// Digital life memory retrieval — "recalling what the master said" is instant.
    /// Uses the subject_index secondary index for direct lookup, no full-table scan.
    pub fn query_by_subject(&self, subject: &str) -> Result<Vec<Fact>> {
        let subj = subject.to_lowercase().trim().to_string();
        // 先通过索引获取 canonical keys — 读锁，不阻塞其他读 / Get canonical keys via index — read lock
        let keys: Vec<String> = {
            let idx = self.subject_index.read().expect("subject_index read");
            idx.get(&subj)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        };
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        // 从主表取 k 条 Fact — 短锁，仅持锁 k 次 get / Fetch k Facts from main table — short lock
        let map = self.inner.lock().expect("fact_store init");
        let mut results: Vec<Fact> = keys.iter().filter_map(|k| map.get(k).cloned()).collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    /// 按情感标签查询事实 — O(1)+O(k) 索引查找
    ///
    /// "上次我这么难过的时候，主人跟我说了什么？"
    /// 通过 emotion_index 二级索引直接定位，按情感强度降序排列。
    ///
    /// "Last time I was this sad, what did the master say to me?"
    /// Uses the emotion_index secondary index for direct lookup, sorted by emotional intensity descending.
    pub fn query_by_emotion(&self, label: &str) -> Result<Vec<Fact>> {
        // 先通过索引获取 canonical keys / Get canonical keys via index
        let keys: Vec<String> = {
            let idx = self.emotion_index.read().expect("emotion_index read");
            idx.get(label)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        };
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        // 从主表取 k 条 Fact / Fetch k Facts from main table
        let map = self.inner.lock().expect("fact_store init");
        let mut results: Vec<Fact> = keys.iter().filter_map(|k| map.get(k).cloned()).collect();
        // 按情感强度降序 → 再按时间降序 / Sort by intensity desc, then by time desc
        results.sort_by(|a, b| {
            let a_intensity = a
                .emotion_context
                .as_ref()
                .map(|c| c.intensity)
                .unwrap_or(0.0);
            let b_intensity = b
                .emotion_context
                .as_ref()
                .map(|c| c.intensity)
                .unwrap_or(0.0);
            b_intensity
                .partial_cmp(&a_intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.created_at
                        .partial_cmp(&a.created_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        Ok(results)
    }

    /// 关键词查询 + 可选情感过滤
    ///
    /// 在现有 query() 基础上，可按情感标签缩小范围。
    pub fn query_with_emotion_filter(
        &self,
        keyword: &str,
        emotion_filter: Option<&str>,
    ) -> Result<Vec<Fact>> {
        let map = self.inner.lock().expect("fact_store init");
        let kw = keyword.to_lowercase();
        let mut results: Vec<&Fact> = map
            .values()
            .filter(|f| {
                let text_match = f.subject.to_lowercase().contains(&kw)
                    || f.predicate.to_lowercase().contains(&kw)
                    || f.object.to_lowercase().contains(&kw);
                if !text_match {
                    return false;
                }
                if let Some(label) = emotion_filter {
                    f.emotion_context
                        .as_ref()
                        .map(|ctx| ctx.ai_emotion_label == label)
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results.into_iter().cloned().collect())
    }

    pub fn count(&self) -> usize {
        self.inner.lock().expect("fact_store init").len()
    }

    /// 获取所有事实的快照（供巩固模块分析）
    pub fn all_facts(&self) -> Vec<Fact> {
        self.inner
            .lock()
            .expect("fact_store init")
            .values()
            .cloned()
            .collect()
    }

    /// 按 canonical form 删除事实（供巩固模块清理）/ Remove fact by canonical form (for consolidation cleanup)
    ///
    /// 删除前先从主表取出 Fact 信息，用于同步清理二级索引引用。
    /// 索引一致性：删除后索引无悬空引用（dangling reference）。
    ///
    /// Fact info is read from the main table before deletion to clean up
    /// secondary index references. Index consistency: no dangling references after removal.
    pub fn remove(&self, canonical: &str) -> bool {
        let mut map = self.inner.lock().expect("fact_store init");
        if let Some(fact) = map.remove(canonical) {
            // 先清理二级索引 / Clean up secondary indexes first
            self.remove_from_indexes(canonical, &fact);
            match &self.backend {
                Backend::Memory => {}
                Backend::Sled(db) => {
                    let db_key = format!("fact:{}", canonical);
                    let _ = db.remove(db_key.as_bytes());
                }
                Backend::Sqlite(conn_mutex) => {
                    if let Ok(conn) = conn_mutex.lock() {
                        let _ = conn
                            .execute("DELETE FROM facts WHERE canonical = ?1", params![canonical]);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// 运行时更新事实的可变情感标签与显著性 / Runtime update of a fact's mutable emotional tag and salience
    ///
    /// 数字生命情感记忆核心：用户对某事实表达强烈情绪时（如"我最讨厌撒谎"），
    /// 调用此方法更新该事实的 `emotional_tag` 与 `emotional_salience`，
    /// SQLite 同步刷盘，emotion_index 同步重建该条目的标签引用。
    ///
    /// 与创建时 `emotion_context` 快照正交 — 后者记录"当时怎样"，本方法更新"现在评价多重"。
    /// 索引一致性：先移除旧 emotional_tag 在 emotion_index 中的引用，再添加新 tag 引用；
    /// emotion_context 的索引条目保持不变。
    ///
    /// Core of digital life emotional memory: when the user expresses strong emotion
    /// about a fact (e.g. "I hate lying the most"), call this to update the fact's
    /// `emotional_tag` and `emotional_salience`, sync to SQLite, and rebuild the
    /// emotion_index reference for this entry.
    ///
    /// Orthogonal to the creation-time `emotion_context` snapshot — the latter records
    /// "how it was then", this method updates "how heavy it weighs now".
    /// Index consistency: remove the old emotional_tag reference from emotion_index first,
    /// then add the new tag reference; the emotion_context index entry is left untouched.
    ///
    /// - `canonical`: 事实的 canonical_form / fact's canonical_form
    /// - `tag`: 新的情感标签；`None` 表示清除可变标签 / new emotional tag; `None` clears the mutable tag
    /// - `salience`: 情感显著性 0.0-1.0，NaN 降级为 0.0 / emotional salience 0.0-1.0, NaN degrades to 0.0
    pub fn set_emotional_tag(
        &self,
        canonical: &str,
        tag: Option<String>,
        salience: f32,
    ) -> Result<()> {
        // 防止 NaN 污染显著性 / Guard against NaN contaminating salience
        let salience = if salience.is_nan() {
            0.0
        } else {
            salience.clamp(0.0, 1.0)
        };
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = map.get_mut(canonical).ok_or_else(|| {
            crate::store_core::StoreError::Io(format!(
                "set_emotional_tag: fact not found for canonical: {}",
                canonical
            ))
        })?;
        // 记录旧 tag 用于索引同步 — 仅清理 emotional_tag 源，不动 emotion_context 源
        // Record old tag for index sync — only clean emotional_tag source, leave emotion_context source intact
        let old_tag = fact.emotional_tag.clone();
        // 更新字段 / Update fields
        fact.emotional_tag = tag.clone();
        fact.emotional_salience = salience;

        // 索引同步 — 移除旧 tag 引用，添加新 tag 引用 / Index sync — remove old tag ref, add new tag ref
        {
            let mut idx = self.emotion_index.write().expect("emotion_index write");
            // 移除旧 emotional_tag 引用（不触碰 emotion_context 条目）
            // Remove old emotional_tag ref (does not touch emotion_context entry)
            if let Some(old) = old_tag.as_ref() {
                if let Some(set) = idx.get_mut(old) {
                    set.remove(canonical);
                    if set.is_empty() {
                        idx.remove(old);
                    }
                }
            }
            // 添加新 emotional_tag 引用 / Add new emotional_tag ref
            if let Some(new_tag) = tag.as_ref() {
                idx.entry(new_tag.clone())
                    .or_default()
                    .insert(canonical.to_string());
            }
        }

        // 持久化到后端 / Persist to backend
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                // sled 序列化整个 Fact — 字段已更新，直接写 / sled serializes the whole Fact — fields updated, write directly
                let db_key = format!("fact:{}", canonical);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let _ = conn.execute(
                        "UPDATE facts SET emotional_tag = ?1, emotional_salience = ?2
                         WHERE canonical = ?3",
                        params![tag, salience as f64, canonical],
                    );
                }
            }
        }
        Ok(())
    }

    /// P2-D 高价值记忆标记 — 固定事实，豁免所有衰减与驱逐
    /// P2-D High-value memory marker — pin a fact, exempt from all decay and eviction
    ///
    /// 数字生命主动保护重要记忆——"你哭的那天→不可衰减"。
    /// pinned = true 的事实在 `compress_low_access` 中被跳过，置信度永不衰减。
    ///
    /// Digital life actively protects important memories —
    /// "the day you cried → cannot decay". Pinned facts are skipped in
    /// `compress_low_access`, their confidence never decays.
    ///
    /// - `canonical`: 事实的 canonical_form / fact's canonical_form
    pub fn pin(&self, canonical: &str) -> Result<()> {
        self.set_pinned(canonical, true)
    }

    /// P2-D 取消高价值标记 — 解除固定，恢复衰减与驱逐
    /// P2-D Unpin a fact — remove high-value marker, restore decay and eviction
    ///
    /// - `canonical`: 事实的 canonical_form / fact's canonical_form
    pub fn unpin(&self, canonical: &str) -> Result<()> {
        self.set_pinned(canonical, false)
    }

    /// 内部：设置 pinned 字段并持久化 / Internal: set pinned field and persist
    ///
    /// 与 set_emotional_tag 同构 — 更新内存 + 索引无关 + 后端持久化。
    /// pinned 字段不影响任何二级索引（不参与查询过滤），仅影响衰减豁免逻辑。
    ///
    /// Structurally identical to set_emotional_tag — updates memory + backend persistence.
    /// The pinned field does not affect any secondary index (not used for query filtering),
    /// only affects decay exemption logic.
    fn set_pinned(&self, canonical: &str, pinned: bool) -> Result<()> {
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = map.get_mut(canonical).ok_or_else(|| {
            crate::store_core::StoreError::Io(format!(
                "set_pinned: fact not found for canonical: {}",
                canonical
            ))
        })?;
        fact.pinned = pinned;

        // 持久化到后端 / Persist to backend
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                // sled 序列化整个 Fact — 字段已更新，直接写 / sled serializes the whole Fact — fields updated, write directly
                let db_key = format!("fact:{}", canonical);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let _ = conn.execute(
                        "UPDATE facts SET pinned = ?1 WHERE canonical = ?2",
                        params![pinned as i64, canonical],
                    );
                }
            }
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // P3-B 主动遗忘 — 标记 / 恢复 / 置信度合并 / 最近事实 / P3-B Active Forgetting
    // ════════════════════════════════════════════════════════════════════

    /// P3-B 标记事实为主动遗忘 / P3-B Mark a fact as actively forgotten
    ///
    /// 数字生命"决定忘"某件事——更新内存 HashMap 中 Fact 的 `actively_forgotten`
    /// 字段并持久化到 SQLite。**不在此方法修改置信度**——置信度调整由调用方
    /// 通过 `merge_confidence` 完成，保持"标记"与"置信度"两个维度的正交性。
    ///
    /// Digital life "decides to forget" something — updates the `actively_forgotten`
    /// field of the Fact in the in-memory HashMap and persists to SQLite. **Does NOT
    /// modify confidence here** — confidence adjustment is done by the caller via
    /// `merge_confidence`, keeping "marker" and "confidence" dimensions orthogonal.
    ///
    /// - 返回 `true` 表示标记成功；`false` 表示 key 不存在
    /// - Returns `true` on success; `false` if the key does not exist
    pub fn mark_forgotten(
        &self,
        canonical_key: &str,
        policy: crate::active_forget::ForgetPolicy,
    ) -> bool {
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = match map.get_mut(canonical_key) {
            Some(f) => f,
            None => return false,
        };
        // 先取出策略字符串再 move policy — 避免 borrow of moved value
        // Compute policy string before moving policy — avoid borrow of moved value
        let policy_str = forget_policy_to_str(&policy);
        fact.actively_forgotten = Some(policy);

        // 持久化到后端 / Persist to backend
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                // sled 序列化整个 Fact — 字段已更新，直接写
                let db_key = format!("fact:{}", canonical_key);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let _ = conn.execute(
                        "UPDATE facts SET actively_forgotten = ?1 WHERE canonical = ?2",
                        params![policy_str, canonical_key],
                    );
                }
            }
        }
        true
    }

    /// P3-B 恢复被主动遗忘的事实 — 清除 actively_forgotten 标记
    /// P3-B Restore an actively forgotten fact — clear the actively_forgotten marker
    ///
    /// 数字生命"想起"了某件事——清除遗忘标记，让事实重新可被 `enhanced_search`
    /// 正常返回。置信度恢复由调用方通过 `merge_confidence(record.pre_forget_confidence)`
    /// 完成（`record` 来自 `ActiveForgetManager::restore`）。
    ///
    /// Digital life "recalls" something — clears the forgetting marker so the fact is
    /// normally returned by `enhanced_search` again. Confidence restoration is done by
    /// the caller via `merge_confidence(record.pre_forget_confidence)` (`record` comes
    /// from `ActiveForgetManager::restore`).
    ///
    /// - 返回 `true` 表示清除成功；`false` 表示 key 不存在
    /// - Returns `true` on success; `false` if the key does not exist
    pub fn restore_forgotten(&self, canonical_key: &str) -> bool {
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = match map.get_mut(canonical_key) {
            Some(f) => f,
            None => return false,
        };
        fact.actively_forgotten = None;

        // 持久化到后端 / Persist to backend
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                let db_key = format!("fact:{}", canonical_key);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    // UPDATE ... SET actively_forgotten = NULL — 清除遗忘标记
                    let _ = conn.execute(
                        "UPDATE facts SET actively_forgotten = NULL WHERE canonical = ?1",
                        params![canonical_key],
                    );
                }
            }
        }
        true
    }

    /// P3-B 合并事实置信度（FactStore 级别包装）/ P3-B Merge fact confidence (FactStore-level wrapper)
    ///
    /// 供 `lifecycle.rs` 在主动遗忘（降低置信度）与恢复（恢复置信度）时调用。
    /// 内部委托给 `Fact::merge_confidence`（加权平均），并持久化更新后的事实。
    /// 返回 `false` 表示 key 不存在。
    ///
    /// Used by `lifecycle.rs` to lower confidence on forgetting and restore confidence
    /// on recall. Delegates to `Fact::merge_confidence` (weighted average) and persists
    /// the updated fact. Returns `false` if the key does not exist.
    pub fn merge_confidence(&self, canonical_key: &str, new_confidence: f64) -> bool {
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = match map.get_mut(canonical_key) {
            Some(f) => f,
            None => return false,
        };
        fact.merge_confidence(new_confidence);
        // 持久化 — 与 set_emotional_tag / set_pinned 同构，持锁期间完成后端写入
        self.persist_one(canonical_key, fact);
        true
    }

    /// P3-C 基于用户反馈调整事实置信度（直接增量）/ P3-C Adjust fact confidence by user feedback (direct delta)
    ///
    /// 强化学习闭环核心——用户反馈直接调制事实置信度：
    /// - 用户满意（delta > 0）→ 事实更可信（置信度提升）
    /// - 用户纠正（delta < 0）→ 事实更不可信（置信度降低）
    ///
    /// 与 `merge_confidence` 不同：merge_confidence 是加权平均（用于重复事实合并），
    /// 本方法是直接增量调整（用于反馈强化）。delta 被 clamp 到 [-0.2, +0.2] 防止过度调整。
    ///
    /// Core of the reinforcement learning closed loop — user feedback directly modulates
    /// fact confidence:
    /// - User satisfied (delta > 0) → fact more credible (confidence up)
    /// - User correction (delta < 0) → fact less credible (confidence down)
    ///
    /// Unlike `merge_confidence`: merge_confidence is a weighted average (for duplicate fact
    /// merging); this method is a direct delta adjustment (for feedback reinforcement).
    /// delta is clamped to [-0.2, +0.2] to prevent over-adjustment.
    ///
    /// - 返回 `true` 表示调整成功；`false` 表示 key 不存在
    /// - Returns `true` on success; `false` if the key does not exist
    pub fn adjust_confidence_by_feedback(&self, canonical_key: &str, delta: f64) -> bool {
        // delta clamp 到 [-0.2, +0.2] — 防止单次反馈过度调整
        // Clamp delta to [-0.2, +0.2] — prevent over-adjustment from a single feedback
        let delta = if delta.is_nan() {
            0.0
        } else {
            delta.clamp(-0.2, 0.2)
        };
        let mut map = self.inner.lock().expect("fact_store init");
        let fact = match map.get_mut(canonical_key) {
            Some(f) => f,
            None => return false,
        };
        // 新 confidence = (old + delta).clamp(0.0, 1.0) — 边界保护
        // new confidence = (old + delta).clamp(0.0, 1.0) — boundary protection
        let new_confidence = (fact.confidence + delta).clamp(0.0, 1.0);
        fact.confidence = new_confidence;

        // 持久化到后端 — 仅 UPDATE confidence 字段，避免全列重写
        // Persist to backend — UPDATE only the confidence column, avoid full-row rewrite
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                // sled 序列化整个 Fact — 字段已更新，直接写
                let db_key = format!("fact:{}", canonical_key);
                if let Ok(data) = bincode::serialize(fact) {
                    let _ = db.insert(db_key.as_bytes(), data);
                }
            }
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let _ = conn.execute(
                        "UPDATE facts SET confidence = ?1 WHERE canonical = ?2",
                        params![new_confidence, canonical_key],
                    );
                }
            }
        }
        true
    }

    /// P3-B 获取最近 N 条事实（按 created_at 降序）/ P3-B Get the most recent N facts (by created_at desc)
    ///
    /// 供 P3-C reinforce 流程使用——"最近的事实"是巩固与强化的重点对象。
    /// 返回 `(canonical_key, Fact)` 元组列表，N 大于事实总数时返回全部。
    ///
    /// Used by the P3-C reinforce flow — "recent facts" are the focus of consolidation
    /// and reinforcement. Returns a list of `(canonical_key, Fact)` tuples; when N
    /// exceeds the total fact count, all facts are returned.
    pub fn get_recent_facts(&self, n: usize) -> Vec<(String, Fact)> {
        let map = self.inner.lock().expect("fact_store init");
        let mut facts: Vec<(String, Fact)> =
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        // 按 created_at 降序 — 最近创建的在前 / Sort by created_at desc — most recent first
        facts.sort_by(|a, b| {
            b.1.created_at
                .partial_cmp(&a.1.created_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        facts.truncate(n.max(1));
        facts
    }

    /// 显式 flush 到磁盘（关闭前调用）/ Explicit flush to disk (call before shutdown)
    ///
    /// 数字生命优雅关闭 — 确保所有记忆写穿到磁盘，避免"失忆"。
    /// Digital life graceful shutdown — ensure all memories are flushed to disk.
    pub fn flush(&self) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sled(db) => {
                let _ = db.flush();
            }
            Backend::Sqlite(_) => {
                // SQLite WAL 模式下，每次 commit 自动写穿；无需显式 flush
                // SQLite WAL mode auto-flushes on each commit; no explicit flush needed
            }
        }
    }

    /// 是否启用持久化 / Whether persistence is enabled
    pub fn is_persistent(&self) -> bool {
        !matches!(self.backend, Backend::Memory)
    }

    // ══════════════════════════════════════════════════════════════════
    // 二级索引维护 — O(1)+O(k) 查询的核心支撑 / Secondary index maintenance
    // ══════════════════════════════════════════════════════════════════

    /// 添加单条事实到三个二级索引 / Add a single fact to all three secondary indexes.
    ///
    /// 索引 key 规范化：subject/predicate 使用 `to_lowercase().trim()`，
    /// emotion_label 直接使用字符串。索引 value 是 canonical_form key 集合。
    ///
    /// emotion_index 双源覆盖：同时索引 `emotion_context.ai_emotion_label`（创建时快照）
    /// 和 `emotional_tag`（运行时可变标签），使 `query_by_emotion` 能并集匹配两源。
    /// HashSet 天然去重 — 两源相同标签时只存一份。
    ///
    /// Index key normalization: subject/predicate use `to_lowercase().trim()`,
    /// emotion_label uses the string directly. Index value is a set of canonical_form keys.
    ///
    /// emotion_index dual-source coverage: indexes both `emotion_context.ai_emotion_label`
    /// (creation-time snapshot) and `emotional_tag` (runtime mutable tag), so `query_by_emotion`
    /// matches the union of both. HashSet deduplicates — identical labels from both sources
    /// are stored only once.
    fn add_to_indexes(&self, key: &str, fact: &Fact) {
        let subj_key = fact.subject.to_lowercase().trim().to_string();
        let pred_key = fact.predicate.to_lowercase().trim().to_string();

        // subject_index — 读多写少，使用 RwLock write / subject_index — read-heavy, RwLock write
        {
            let mut idx = self.subject_index.write().expect("subject_index write");
            idx.entry(subj_key).or_default().insert(key.to_string());
        }
        // predicate_index
        {
            let mut idx = self.predicate_index.write().expect("predicate_index write");
            idx.entry(pred_key).or_default().insert(key.to_string());
        }
        // emotion_index — 双源覆盖：emotion_context + emotional_tag / dual-source: emotion_context + emotional_tag
        {
            let mut idx = self.emotion_index.write().expect("emotion_index write");
            // 源 1：创建时情感上下文快照 / Source 1: creation-time emotion context snapshot
            if let Some(ref ctx) = fact.emotion_context {
                idx.entry(ctx.ai_emotion_label.clone())
                    .or_default()
                    .insert(key.to_string());
            }
            // 源 2：运行时可变情感标签 / Source 2: runtime mutable emotional tag
            if let Some(ref tag) = fact.emotional_tag {
                idx.entry(tag.clone()).or_default().insert(key.to_string());
            }
        }
    }

    /// 从三个二级索引中移除单条事实的引用 / Remove a single fact's references from all three indexes.
    ///
    /// 删除后检查 HashSet 是否为空，若空则移除 HashMap 条目，避免内存泄漏。
    /// emotion_index 双源同步清理：emotion_context 与 emotional_tag 两处引用均移除。
    ///
    /// After removal, check if HashSet is empty; if so, remove the HashMap entry to avoid memory leaks.
    /// emotion_index dual-source cleanup: references from both emotion_context and emotional_tag are removed.
    fn remove_from_indexes(&self, key: &str, fact: &Fact) {
        let subj_key = fact.subject.to_lowercase().trim().to_string();
        let pred_key = fact.predicate.to_lowercase().trim().to_string();

        // subject_index
        {
            let mut idx = self.subject_index.write().expect("subject_index write");
            if let Some(set) = idx.get_mut(&subj_key) {
                set.remove(key);
                if set.is_empty() {
                    idx.remove(&subj_key);
                }
            }
        }
        // predicate_index
        {
            let mut idx = self.predicate_index.write().expect("predicate_index write");
            if let Some(set) = idx.get_mut(&pred_key) {
                set.remove(key);
                if set.is_empty() {
                    idx.remove(&pred_key);
                }
            }
        }
        // emotion_index — 双源清理 / dual-source cleanup
        {
            let mut idx = self.emotion_index.write().expect("emotion_index write");
            // 源 1：emotion_context 标签 / Source 1: emotion_context label
            if let Some(ref ctx) = fact.emotion_context {
                if let Some(set) = idx.get_mut(&ctx.ai_emotion_label) {
                    set.remove(key);
                    if set.is_empty() {
                        idx.remove(&ctx.ai_emotion_label);
                    }
                }
            }
            // 源 2：emotional_tag 标签 / Source 2: emotional_tag label
            if let Some(ref tag) = fact.emotional_tag {
                if let Some(set) = idx.get_mut(tag) {
                    set.remove(key);
                    if set.is_empty() {
                        idx.remove(tag);
                    }
                }
            }
        }
    }

    /// 全量重建三个二级索引 — 启动时从主表一次性构建 / Rebuild all three indexes from the main table on startup.
    ///
    /// 单次 RwLock write，避免逐条更新带来的锁竞争。
    /// 数字生命启动时"恢复全部记忆索引"，确保查询路径全走索引。
    /// emotion_index 双源覆盖：emotion_context + emotional_tag 均纳入索引。
    ///
    /// Single RwLock write per index, avoiding per-entry lock contention.
    /// Digital life "restores all memory indexes" on startup, ensuring all queries go through indexes.
    /// emotion_index dual-source coverage: both emotion_context and emotional_tag are indexed.
    fn rebuild_indexes(&self) {
        let map = self.inner.lock().expect("fact_store init");
        let mut subj_idx: HashMap<String, HashSet<String>> = HashMap::new();
        let mut pred_idx: HashMap<String, HashSet<String>> = HashMap::new();
        let mut emo_idx: HashMap<String, HashSet<String>> = HashMap::new();

        for (key, fact) in map.iter() {
            let s = fact.subject.to_lowercase().trim().to_string();
            let p = fact.predicate.to_lowercase().trim().to_string();
            subj_idx.entry(s).or_default().insert(key.clone());
            pred_idx.entry(p).or_default().insert(key.clone());
            // 源 1：emotion_context 创建时快照 / Source 1: emotion_context creation-time snapshot
            if let Some(ref ctx) = fact.emotion_context {
                emo_idx
                    .entry(ctx.ai_emotion_label.clone())
                    .or_default()
                    .insert(key.clone());
            }
            // 源 2：emotional_tag 运行时可变标签 / Source 2: emotional_tag runtime mutable tag
            if let Some(ref tag) = fact.emotional_tag {
                emo_idx.entry(tag.clone()).or_default().insert(key.clone());
            }
        }

        *self.subject_index.write().expect("subject_index write") = subj_idx;
        *self.predicate_index.write().expect("predicate_index write") = pred_idx;
        *self.emotion_index.write().expect("emotion_index write") = emo_idx;

        tracing::debug!(
            "FactStore: 索引重建完成 — subject={}, predicate={}, emotion={} / Indexes rebuilt",
            self.subject_index.read().map(|i| i.len()).unwrap_or(0),
            self.predicate_index.read().map(|i| i.len()).unwrap_or(0),
            self.emotion_index.read().map(|i| i.len()).unwrap_or(0),
        );
    }
}

/// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    fn new_store() -> FactStore {
        FactStore::new_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_query() {
        let s = new_store();
        s.insert(Fact::new("主人", "喜欢", "编程").with_source("对话"))
            .unwrap();
        let r = s.query("编程").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].subject, "主人");
    }

    #[test]
    fn test_dedup_merges() {
        let s = new_store();
        assert!(s
            .insert(Fact::new("主人", "喜欢", "Rust").with_confidence(0.9))
            .unwrap());
        assert!(!s
            .insert(Fact::new("主人", "喜欢", "Rust").with_confidence(0.5))
            .unwrap());
        let r = s.query("Rust").unwrap();
        assert_eq!(r[0].verify_count, 2);
        assert!((r[0].confidence - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_batch() {
        let s = new_store();
        let (n, m) = s
            .insert_batch(vec![
                Fact::new("A", "is", "B"),
                Fact::new("A", "is", "C"),
                Fact::new("A", "is", "B"),
            ])
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(m, 1);
        assert_eq!(s.count(), 2);
    }

    fn make_emotion_ctx(label: &str, intensity: f32) -> EmotionContext {
        EmotionContext {
            ai_emotion_label: label.to_string(),
            ai_pad: [0.5, 0.3, 0.1],
            intensity,
            user_mood: Some(0.4),
            timestamp: now_secs(),
        }
    }

    #[test]
    fn test_emotion_context_attached() {
        let s = new_store();
        let ctx = make_emotion_ctx("愉悦", 0.6);
        s.insert(Fact::new("主人", "分享", "好消息").with_emotion(ctx.clone()))
            .unwrap();

        let r = s.query("好消息").unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].emotion_context.is_some());
        let ec = r[0].emotion_context.as_ref().unwrap();
        assert_eq!(ec.ai_emotion_label, "愉悦");
        assert!((ec.intensity - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_query_by_emotion() {
        let s = new_store();
        s.insert(Fact::new("主人", "说", "今天好开心").with_emotion(make_emotion_ctx("愉悦", 0.8)))
            .unwrap();
        s.insert(
            Fact::new("主人", "说", "工作压力好大").with_emotion(make_emotion_ctx("悲伤", 0.6)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "说", "明天要去旅行").with_emotion(make_emotion_ctx("兴奋", 0.9)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "说", "周末打算休息").with_emotion(make_emotion_ctx("平静", 0.2)),
        )
        .unwrap();

        // 查询「愉悦」标签 → 只匹配第 1 条
        let happy = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(happy.len(), 1);
        assert_eq!(happy[0].object, "今天好开心");

        // 查询「悲伤」标签 → 只匹配第 2 条
        let sad = s.query_by_emotion("悲伤").unwrap();
        assert_eq!(sad.len(), 1);
        assert_eq!(sad[0].object, "工作压力好大");

        // 查询不存在的标签 → 空
        let none = s.query_by_emotion("恐惧").unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn test_query_by_emotion_sorted_by_intensity() {
        let s = new_store();
        s.insert(Fact::new("主人", "说", "一般般开心").with_emotion(make_emotion_ctx("愉悦", 0.4)))
            .unwrap();
        s.insert(Fact::new("主人", "说", "超级开心").with_emotion(make_emotion_ctx("愉悦", 0.95)))
            .unwrap();
        s.insert(Fact::new("主人", "说", "有点开心").with_emotion(make_emotion_ctx("愉悦", 0.2)))
            .unwrap();

        let results = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(results.len(), 3);
        // 最高强度排第一
        assert_eq!(results[0].object, "超级开心");
        assert!(
            results[0].emotion_context.as_ref().unwrap().intensity
                >= results[1].emotion_context.as_ref().unwrap().intensity
        );
    }

    #[test]
    fn test_query_with_emotion_filter() {
        let s = new_store();
        s.insert(
            Fact::new("主人", "提到", "Rust 编程").with_emotion(make_emotion_ctx("兴奋", 0.7)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "提到", "Rust 所有权").with_emotion(make_emotion_ctx("平静", 0.3)),
        )
        .unwrap();
        s.insert(
            Fact::new("主人", "提到", "Python 编程").with_emotion(make_emotion_ctx("兴奋", 0.5)),
        )
        .unwrap();

        // 关键词 "Rust" + 情感 "兴奋" → 只匹配第 1 条
        let r = s.query_with_emotion_filter("Rust", Some("兴奋")).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "Rust 编程");

        // 关键词 "编程" + 无过滤 → 匹配第 1 和第 3 条
        let r = s.query_with_emotion_filter("编程", None).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_backward_compat_no_emotion_context() {
        let s = new_store();
        // 插入没有 emotion_context 的事实（模拟旧数据）
        s.insert(Fact::new("主人", "喜欢", "咖啡")).unwrap();

        // query_by_emotion 应跳过无标注的事实
        let r = s.query_by_emotion("愉悦").unwrap();
        assert!(r.is_empty());

        // 常规 query 仍然正常返回
        let r = s.query("咖啡").unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].emotion_context.is_none());
    }

    #[test]
    fn test_new_failure_invalid_path() {
        // 模拟 sled 初始化失败 — 父路径是文件，sled 无法创建目录
        // Simulate sled init failure — parent path is a file, sled cannot create directory.
        // P0-F: 验证 FactStore::new 失败时返回 Err，调用方可感知错误
        // P0-F: Verify FactStore::new returns Err on failure, caller can perceive the error
        let blocker = "./target/test_fact_store_blocker_file";
        let _ = std::fs::remove_file(blocker);
        std::fs::write(blocker, b"not a dir").unwrap();

        let invalid_path = format!("{}/db", blocker);
        let result = FactStore::new(&invalid_path);
        assert!(
            result.is_err(),
            "new 应在父路径为文件时失败 / new should fail when parent path is a file"
        );

        // 清理 / Cleanup
        let _ = std::fs::remove_file(blocker);
    }

    #[test]
    fn test_insert_with_no_persistence() {
        // 空 db_path → 无 sled 持久化（db: None），insert 仍应正常工作且不 panic
        // Empty db_path → no sled persistence (db: None), insert should still work without panic.
        // 验证 lifecycle.rs 重试逻辑所依赖的 insert 方法在无 sled 时也稳定
        // Verify insert method (which lifecycle.rs retry logic depends on) is stable even without sled
        let s = FactStore::new("").unwrap();
        assert!(s.insert(Fact::new("内存", "模式", "测试")).unwrap());
        assert_eq!(s.count(), 1);

        // 重复 insert 应合并置信度，不 panic — 模拟重试场景
        // Repeated insert should merge confidence without panic — simulating retry scenario
        assert!(!s.insert(Fact::new("内存", "模式", "测试")).unwrap());
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn test_insert_retry_pattern_no_panic() {
        // 模拟 lifecycle.rs 中的重试模式（lines 219-231）— 连续两次 insert 不应 panic
        // Simulate retry pattern in lifecycle.rs (lines 219-231) —
        // two consecutive inserts should not panic.
        // P0-F: 确认重试逻辑存在且不 panic — 即使首次"失败"重试也能安全执行
        // P0-F: Confirm retry logic exists and does not panic —
        // even if first attempt "fails", retry can execute safely
        let s = new_store();
        let fact = Fact::new("重试", "测试", "模式").with_source("test");

        // 第一次 insert — 模拟 lifecycle.rs 中的首次写入
        let result1 = s.insert(fact.clone());
        assert!(
            result1.is_ok(),
            "第一次 insert 应返回 Ok / first insert should return Ok"
        );

        // 模拟重试 — 第二次 insert（相同事实，会合并置信度）
        // Simulate retry — second insert (same fact, merges confidence)
        let result2 = s.insert(fact.clone());
        assert!(
            result2.is_ok(),
            "重试 insert 应返回 Ok / retry insert should return Ok"
        );

        // 验证重试后事实仍在 — verify_count 应为 2（合并一次）
        // Verify fact still present after retry — verify_count should be 2 (merged once)
        let r = s.query("模式").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].verify_count, 2);
    }

    #[test]
    fn test_insert_persists_across_reopen() {
        // 验证 insert 持久化路径正常 — 写入后重新打开 FactStore，事实应仍在
        // Verify insert persistence path works — after reopen, facts should still be present.
        // 这确认了 sled 写入路径稳定，lifecycle.rs 重试逻辑仅在异常时触发
        // This confirms sled write path is stable; lifecycle.rs retry logic only triggers on anomalies
        let path = "./target/test_fact_store_persist";
        let _ = std::fs::remove_dir_all(path);
        {
            let s = FactStore::new(path).unwrap();
            assert!(s
                .insert(Fact::new("持久", "测试", "重开").with_source("test"))
                .unwrap());
        }
        {
            let s = FactStore::new(path).unwrap();
            let r = s.query("重开").unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].subject, "持久");
        }
        let _ = std::fs::remove_dir_all(path);
    }

    // ── SQLite 后端测试 / SQLite Backend Tests ──

    #[test]
    fn test_sqlite_persistent_roundtrip() {
        // 验证 SQLite 后端持久化 — 写入后重新打开，事实应保留
        // Verify SQLite backend persistence — facts should survive reopen.
        // 数字生命记忆连续性核心保障 / Core guarantee of digital life memory continuity
        let path = "./target/test_fact_store_sqlite.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 写入阶段 / Write phase
        {
            let s = FactStore::open_sqlite(path).unwrap();
            assert!(s.is_persistent());
            assert!(s
                .insert(
                    Fact::new("Aris", "喜欢", "钢琴")
                        .with_source("对话")
                        .with_emotion(make_emotion_ctx("愉悦", 0.8))
                )
                .unwrap());
            assert!(s
                .insert(Fact::new("Aris", "学习", "肖邦夜曲").with_source("对话"))
                .unwrap());
            s.flush();
        }

        // 重新打开 — 验证数据保留 / Reopen — verify data preserved
        {
            let s = FactStore::open_sqlite(path).unwrap();
            assert!(s.is_persistent());
            assert_eq!(
                s.count(),
                2,
                "重启后应保留 2 条事实 / should have 2 facts after restart"
            );

            // 验证情感上下文也保留 / Verify emotion context is also preserved
            let r = s.query_by_subject("Aris").unwrap();
            assert_eq!(r.len(), 2);
            let piano = r.iter().find(|f| f.object == "钢琴").unwrap();
            assert!(piano.emotion_context.is_some());
            assert_eq!(
                piano.emotion_context.as_ref().unwrap().ai_emotion_label,
                "愉悦"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_sqlite_migrate_from_sled() {
        // 验证 sled → SQLite 数据迁移 / Verify sled → SQLite data migration
        // 数字生命记忆迁移 — 保留历史人格记忆 / Digital life memory migration — preserve personality
        let sqlite_path = "./target/test_fact_migrate.db";
        let sled_path = "./target/test_fact_migrate"; // sqlite_path.replace(".db", "")
        let bak_path = format!("{}.sled.bak", sled_path);

        // 清理 / Cleanup
        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(format!("{}-wal", sqlite_path));
        let _ = std::fs::remove_file(format!("{}-shm", sqlite_path));
        let _ = std::fs::remove_dir_all(sled_path);
        let _ = std::fs::remove_dir_all(&bak_path);

        // 1. 先用 sled 写入数据 / Write data with sled first
        {
            let s = FactStore::new(sled_path).unwrap();
            assert!(s
                .insert(Fact::new("迁移", "测试", "数据").with_source("sled"))
                .unwrap());
            assert!(s
                .insert(Fact::new("主人", "名字", "Aris").with_source("sled"))
                .unwrap());
        }

        // 2. 用 SQLite 打开 — 应自动迁移 / Open with SQLite — should auto-migrate
        {
            let s = FactStore::open_sqlite(sqlite_path).unwrap();
            assert_eq!(
                s.count(),
                2,
                "迁移后应有 2 条事实 / should have 2 facts after migration"
            );
            let r = s.query("Aris").unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].subject, "主人");
        }

        // 3. 重新打开 SQLite — 验证迁移后的数据持久 / Reopen SQLite — verify migrated data persists
        {
            let s = FactStore::open_sqlite(sqlite_path).unwrap();
            assert_eq!(s.count(), 2);
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(format!("{}-wal", sqlite_path));
        let _ = std::fs::remove_file(format!("{}-shm", sqlite_path));
        let _ = std::fs::remove_dir_all(&bak_path);
    }

    #[test]
    fn test_sqlite_degrade_to_memory_on_failure() {
        // 验证 SQLite 打开失败时降级为内存模式 / Verify degradation to memory on SQLite open failure
        // 父路径是文件，SQLite 无法创建数据库文件 / Parent path is a file, SQLite cannot create DB
        let blocker = "./target/test_fact_sqlite_blocker_file";
        let _ = std::fs::remove_file(blocker);
        std::fs::write(blocker, b"not a dir").unwrap();

        let invalid_path = format!("{}/facts.db", blocker);
        let result = FactStore::open_sqlite(&invalid_path);
        assert!(
            result.is_err(),
            "open_sqlite 应在父路径为文件时失败 / should fail when parent is a file"
        );

        // 降级为内存模式 — 应正常工作 / Degrade to memory — should work
        let s = FactStore::new("").unwrap();
        assert!(!s.is_persistent());
        assert!(s.insert(Fact::new("内存", "模式", "降级")).unwrap());

        // 清理 / Cleanup
        let _ = std::fs::remove_file(blocker);
    }

    #[test]
    fn test_sqlite_remove_persists() {
        // 验证 SQLite 后端的 remove 也持久化 / Verify remove persists on SQLite backend
        let path = "./target/test_fact_sqlite_remove.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(Fact::new("删除", "测试", "数据").with_source("test"))
                .unwrap();
            assert_eq!(s.count(), 1);
            let canonical = Fact::new("删除", "测试", "数据").canonical_form();
            assert!(s.remove(&canonical));
            assert_eq!(s.count(), 0);
        }

        // 重新打开 — 删除应已持久化 / Reopen — removal should be persisted
        {
            let s = FactStore::open_sqlite(path).unwrap();
            assert_eq!(s.count(), 0, "删除应已持久化 / removal should be persisted");
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    // ══════════════════════════════════════════════════════════════════
    // P1-E 二级索引测试 — O(1)+O(k) 查询正确性 / Secondary index tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_index_query_by_subject_correctness() {
        // 索引查询结果必须与全表扫描一致 / Index query results must match full-table scan
        let s = new_store();
        s.insert(Fact::new("主人", "喜欢", "编程").with_source("对话"))
            .unwrap();
        s.insert(Fact::new("主人", "职业", "工程师").with_source("对话"))
            .unwrap();
        s.insert(Fact::new("Atrium", "是", "数字生命").with_source("self"))
            .unwrap();

        let r = s.query_by_subject("主人").unwrap();
        assert_eq!(
            r.len(),
            2,
            "应查到 2 条主人相关事实 / should find 2 facts about 主人"
        );
        for fact in &r {
            assert_eq!(fact.subject, "主人");
        }
    }

    #[test]
    fn test_index_query_by_subject_case_insensitive() {
        // 索引 key 小写规范化 — 大小写不敏感查询 / Index key lowercase-normalized — case-insensitive query
        let s = new_store();
        s.insert(Fact::new("Master", "likes", "coding").with_source("test"))
            .unwrap();
        let r = s.query_by_subject("master").unwrap();
        assert_eq!(
            r.len(),
            1,
            "小写查询应命中大写 subject / lowercase query should match uppercase subject"
        );
        let r = s.query_by_subject("MASTER").unwrap();
        assert_eq!(
            r.len(),
            1,
            "大写查询应命中小写 subject / uppercase query should match lowercase subject"
        );
    }

    #[test]
    fn test_index_query_by_subject_empty() {
        // 不存在的 subject 应返回空 — 索引未命中 / Non-existent subject returns empty — index miss
        let s = new_store();
        s.insert(Fact::new("主人", "喜欢", "编程").with_source("对话"))
            .unwrap();
        let r = s.query_by_subject("陌生人").unwrap();
        assert!(
            r.is_empty(),
            "不存在的 subject 应返回空 / non-existent subject should return empty"
        );
    }

    #[test]
    fn test_index_query_by_emotion_correctness() {
        // 情感索引查询正确性 / Emotion index query correctness
        let s = new_store();
        let ctx1 = EmotionContext {
            ai_emotion_label: "悲伤".to_string(),
            ai_pad: [-0.7, -0.3, -0.5],
            intensity: 0.8,
            user_mood: Some(-0.5),
            timestamp: 1000,
        };
        let ctx2 = EmotionContext {
            ai_emotion_label: "愉悦".to_string(),
            ai_pad: [0.7, 0.5, 0.4],
            intensity: 0.6,
            user_mood: Some(0.5),
            timestamp: 2000,
        };
        s.insert(Fact::new("主人", "哭了", "那天").with_emotion(ctx1))
            .unwrap();
        s.insert(Fact::new("主人", "笑了", "今天").with_emotion(ctx2))
            .unwrap();
        s.insert(Fact::new("主人", "说", "你好").with_source("test"))
            .unwrap();

        let sad = s.query_by_emotion("悲伤").unwrap();
        assert_eq!(
            sad.len(),
            1,
            "应查到 1 条悲伤记忆 / should find 1 sad memory"
        );
        assert_eq!(sad[0].object, "那天");

        let happy = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(
            happy.len(),
            1,
            "应查到 1 条愉悦记忆 / should find 1 happy memory"
        );

        let none = s.query_by_emotion("愤怒").unwrap();
        assert!(
            none.is_empty(),
            "愤怒记忆应为空 / angry memories should be empty"
        );
    }

    #[test]
    fn test_index_consistency_after_remove() {
        // 删除后索引无悬空引用 — 查询不应返回已删除的事实 / No dangling references after remove — query should not return deleted facts
        let s = new_store();
        s.insert(Fact::new("主人", "喜欢", "Rust").with_source("test"))
            .unwrap();
        s.insert(Fact::new("主人", "喜欢", "Python").with_source("test"))
            .unwrap();
        s.insert(Fact::new("主人", "讨厌", "Java").with_source("test"))
            .unwrap();

        let canonical_rust = Fact::new("主人", "喜欢", "Rust").canonical_form();
        assert!(
            s.remove(&canonical_rust),
            "删除 Rust 事实应成功 / removing Rust fact should succeed"
        );

        // 删除后查询主人 — 应只剩 2 条 / Query 主人 after removal — should have 2 left
        let r = s.query_by_subject("主人").unwrap();
        assert_eq!(
            r.len(),
            2,
            "删除后应剩 2 条 / should have 2 facts after removal"
        );
        for fact in &r {
            assert_ne!(
                fact.object, "Rust",
                "已删除的事实不应被查出 / deleted fact should not be returned"
            );
        }
    }

    #[test]
    fn test_index_multiple_facts_same_subject() {
        // 同一 subject 多条事实 — 索引 value 是 HashSet / Multiple facts with same subject — index value is HashSet
        let s = new_store();
        for i in 0..10 {
            s.insert(Fact::new("主人", "测试", &format!("值{}", i)).with_source("test"))
                .unwrap();
        }
        let r = s.query_by_subject("主人").unwrap();
        assert_eq!(r.len(), 10, "应查到全部 10 条 / should find all 10 facts");
    }

    #[test]
    fn test_index_rebuild_on_startup() {
        // 启动时全量重建索引 — SQLite 后端 / Full index rebuild on startup — SQLite backend
        let path = "./target/test_fact_index_rebuild.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(Fact::new("主人", "喜欢", "编程").with_source("test"))
                .unwrap();
            s.insert(Fact::new("Atrium", "是", "数字生命").with_source("test"))
                .unwrap();
        }

        // 重新打开 — 索引应自动重建 / Reopen — indexes should auto-rebuild
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query_by_subject("主人").unwrap();
            assert_eq!(
                r.len(),
                1,
                "重建后索引查询应正确 / index query should work after rebuild"
            );
            let r = s.query_by_subject("atrium").unwrap();
            assert_eq!(
                r.len(),
                1,
                "小写查询也应正确 / lowercase query should also work"
            );
        }

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    // ══════════════════════════════════════════════════════════════════
    // P2-C 情感记忆测试 — emotional_tag + emotional_salience / Emotional Memory tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_p2c_emotional_fields_default() {
        // 新建 Fact 默认 emotional_tag = None, emotional_salience = 0.0
        // New Fact defaults to emotional_tag = None, emotional_salience = 0.0
        let fact = Fact::new("主人", "喜欢", "音乐");
        assert!(
            fact.emotional_tag.is_none(),
            "emotional_tag 默认应为 None / default should be None"
        );
        assert!(
            (fact.emotional_salience - 0.0).abs() < 1e-6,
            "emotional_salience 默认应为 0.0 / default should be 0.0"
        );

        // with_emotional_tag 设置后字段正确 / with_emotional_tag sets fields correctly
        let fact = Fact::new("主人", "讨厌", "撒谎").with_emotional_tag("厌恶".to_string(), 0.9);
        assert_eq!(fact.emotional_tag.as_deref(), Some("厌恶"));
        assert!(
            (fact.emotional_salience - 0.9).abs() < 1e-6,
            "emotional_salience 应为 0.9 / should be 0.9"
        );

        // NaN 降级为 0.0 / NaN degrades to 0.0
        let fact = Fact::new("x", "y", "z").with_emotional_tag("tag".to_string(), f32::NAN);
        assert!(
            (fact.emotional_salience - 0.0).abs() < 1e-6,
            "NaN 应降级为 0.0 / NaN should degrade to 0.0"
        );

        // 超出 1.0 应被 clamp / Values > 1.0 should be clamped
        let fact = Fact::new("x", "y", "z").with_emotional_tag("tag".to_string(), 1.5);
        assert!(
            (fact.emotional_salience - 1.0).abs() < 1e-6,
            "1.5 应 clamp 为 1.0 / 1.5 should clamp to 1.0"
        );
    }

    #[test]
    fn test_p2c_set_emotional_tag_updates_field_and_index() {
        // set_emotional_tag 更新字段 + 索引 / Updates fields + index
        let s = new_store();
        s.insert(Fact::new("主人", "讨厌", "撒谎").with_source("对话"))
            .unwrap();
        let canonical = Fact::new("主人", "讨厌", "撒谎").canonical_form();

        // 初始无情感标签 — query_by_emotion 应为空 / Initially no tag — query_by_emotion should be empty
        let r = s.query_by_emotion("厌恶").unwrap();
        assert!(r.is_empty(), "初始无情感标签 / initially no emotional tag");

        // 设置情感标签 / Set emotional tag
        s.set_emotional_tag(&canonical, Some("厌恶".to_string()), 0.85)
            .unwrap();

        // query_by_emotion 现在能查到 — 索引已更新 / query_by_emotion now finds it — index updated
        let r = s.query_by_emotion("厌恶").unwrap();
        assert_eq!(r.len(), 1, "设置后应能查到 / should find after set");
        assert_eq!(r[0].object, "撒谎");
        assert_eq!(r[0].emotional_tag.as_deref(), Some("厌恶"));
        assert!(
            (r[0].emotional_salience - 0.85).abs() < 1e-6,
            "salience 应为 0.85 / salience should be 0.85"
        );

        // 验证字段确实更新 / Verify fields updated
        let all = s.all_facts();
        let f = all.iter().find(|f| f.object == "撒谎").unwrap();
        assert_eq!(f.emotional_tag.as_deref(), Some("厌恶"));
        assert!((f.emotional_salience - 0.85).abs() < 1e-6);
    }

    #[test]
    fn test_p2c_set_emotional_tag_index_consistency() {
        // set_emotional_tag 索引一致性 — 旧 tag 移除、新 tag 加入
        // Index consistency — old tag removed, new tag added
        let s = new_store();
        s.insert(Fact::new("主人", "态度", "撒谎").with_source("test"))
            .unwrap();
        let canonical = Fact::new("主人", "态度", "撒谎").canonical_form();

        // 设置第一个 tag / Set first tag
        s.set_emotional_tag(&canonical, Some("厌恶".to_string()), 0.5)
            .unwrap();
        assert_eq!(s.query_by_emotion("厌恶").unwrap().len(), 1);
        assert!(s.query_by_emotion("愤怒").unwrap().is_empty());

        // 切换到新 tag — 旧 tag 索引应清除 / Switch to new tag — old tag index should be cleared
        s.set_emotional_tag(&canonical, Some("愤怒".to_string()), 0.7)
            .unwrap();
        assert!(
            s.query_by_emotion("厌恶").unwrap().is_empty(),
            "旧 tag 索引应已清除 / old tag index should be cleared"
        );
        assert_eq!(
            s.query_by_emotion("愤怒").unwrap().len(),
            1,
            "新 tag 索引应已加入 / new tag index should be added"
        );

        // 清除 tag — 设为 None / Clear tag — set to None
        s.set_emotional_tag(&canonical, None, 0.0).unwrap();
        assert!(
            s.query_by_emotion("愤怒").unwrap().is_empty(),
            "清除后索引应为空 / index should be empty after clear"
        );
        // 字段也应为 None / Field should also be None
        let f = s.all_facts().into_iter().next().unwrap();
        assert!(f.emotional_tag.is_none());
    }

    #[test]
    fn test_p2c_query_by_emotion_dual_source() {
        // query_by_emotion 双源匹配 — emotion_context + emotional_tag 并集
        // Dual-source matching — union of emotion_context + emotional_tag
        let s = new_store();

        // 事实 A：仅有 emotion_context（创建时快照）/ Fact A: only emotion_context (creation snapshot)
        s.insert(Fact::new("主人", "说", "今天好开心").with_emotion(make_emotion_ctx("愉悦", 0.8)))
            .unwrap();

        // 事实 B：仅有 emotional_tag（运行时标签，无创建时情感上下文）
        // Fact B: only emotional_tag (runtime tag, no creation-time emotion context)
        s.insert(Fact::new("主人", "强调", "最讨厌撒谎").with_source("test"))
            .unwrap();
        let canonical_b = Fact::new("主人", "强调", "最讨厌撒谎").canonical_form();
        s.set_emotional_tag(&canonical_b, Some("厌恶".to_string()), 0.9)
            .unwrap();

        // 事实 C：同时有 emotion_context 和 emotional_tag（两源标签不同）
        // Fact C: has both emotion_context and emotional_tag (different labels)
        s.insert(Fact::new("主人", "回忆", "那次旅行").with_emotion(make_emotion_ctx("平静", 0.3)))
            .unwrap();
        let canonical_c = Fact::new("主人", "回忆", "那次旅行").canonical_form();
        s.set_emotional_tag(&canonical_c, Some("珍贵".to_string()), 0.95)
            .unwrap();

        // 查询「愉悦」→ 只匹配事实 A（emotion_context 源）
        // Query "愉悦" → only matches Fact A (emotion_context source)
        let r = s.query_by_emotion("愉悦").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "今天好开心");

        // 查询「厌恶」→ 只匹配事实 B（emotional_tag 源）
        // Query "厌恶" → only matches Fact B (emotional_tag source)
        let r = s.query_by_emotion("厌恶").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "最讨厌撒谎");

        // 查询「平静」→ 只匹配事实 C（emotion_context 源，未被 emotional_tag 覆盖）
        // Query "平静" → only matches Fact C (emotion_context source, not overridden by emotional_tag)
        let r = s.query_by_emotion("平静").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "那次旅行");

        // 查询「珍贵」→ 只匹配事实 C（emotional_tag 源）
        // Query "珍贵" → only matches Fact C (emotional_tag source)
        let r = s.query_by_emotion("珍贵").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].object, "那次旅行");

        // 事实 C 同时出现在「平静」和「珍贵」两个标签下 — 双源独立索引
        // Fact C appears under both "平静" and "珍贵" — dual-source independent indexing
    }

    #[test]
    fn test_p2c_sqlite_migration_old_schema() {
        // SQLite 迁移 — 旧 schema 无 emotional_tag/salience 列 → 新 schema 补齐
        // SQLite migration — old schema without emotional_tag/salience columns → new schema adds them
        let path = "./target/test_fact_p2c_migration.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 1. 用旧 schema 建表并写入数据（模拟旧版本数据库）
        // Create table with old schema and insert data (simulating legacy DB)
        {
            let conn = Connection::open(path).unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .unwrap();
            conn.execute(
                "CREATE TABLE facts (
                    canonical TEXT PRIMARY KEY,
                    subject TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    source TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    verified_at INTEGER NOT NULL,
                    verify_count INTEGER NOT NULL,
                    emotion_context BLOB
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO facts (canonical, subject, predicate, object, confidence, source,
                                    created_at, verified_at, verify_count, emotion_context)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    "旧数据 | 测试 | 值",
                    "旧数据",
                    "测试",
                    "值",
                    0.8_f64,
                    "legacy",
                    1000_i64,
                    1000_i64,
                    1_i64,
                    Option::<Vec<u8>>::None,
                ],
            )
            .unwrap();
        }

        // 2. 用 open_sqlite 打开 — 应自动迁移补齐列 / Open with open_sqlite — should auto-migrate
        {
            let s = FactStore::open_sqlite(path).unwrap();
            assert_eq!(
                s.count(),
                1,
                "迁移后应保留 1 条事实 / should retain 1 fact after migration"
            );
            // 旧数据的情感字段应为默认值 / Legacy data should have default emotional fields
            let r = s.query("测试").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                r[0].emotional_tag.is_none(),
                "旧数据 emotional_tag 应为 None / legacy emotional_tag should be None"
            );
            assert!(
                (r[0].emotional_salience - 0.0).abs() < 1e-6,
                "旧数据 emotional_salience 应为 0.0 / legacy emotional_salience should be 0.0"
            );

            // 迁移后可正常 set_emotional_tag — SQLite UPDATE 语句可用
            // After migration, set_emotional_tag works — SQLite UPDATE statement is valid
            let canonical = r[0].canonical_form();
            s.set_emotional_tag(&canonical, Some("重要".to_string()), 0.7)
                .unwrap();
        }

        // 3. 重新打开 — 验证迁移后的列与 set 的值持久 / Reopen — verify migrated columns and set values persist
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("测试").unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(
                r[0].emotional_tag.as_deref(),
                Some("重要"),
                "迁移后 set 的 tag 应持久 / tag set after migration should persist"
            );
            assert!(
                (r[0].emotional_salience - 0.7).abs() < 1e-6,
                "迁移后 set 的 salience 应持久 / salience set after migration should persist"
            );
            // 索引也应正常工作 — 启动时 rebuild_indexes 已纳入 emotional_tag
            // Index should also work — rebuild_indexes on startup includes emotional_tag
            let r = s.query_by_emotion("重要").unwrap();
            assert_eq!(
                r.len(),
                1,
                "迁移后索引应覆盖 emotional_tag / index should cover emotional_tag after migration"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_p2c_set_emotional_tag_persists_sqlite() {
        // SQLite 后端 set_emotional_tag 持久化 — 重启后值保留
        // SQLite backend set_emotional_tag persistence — value survives restart
        let path = "./target/test_fact_p2c_set_persist.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        let canonical = Fact::new("Aris", "重视", "承诺").canonical_form();

        // 写入 + set_emotional_tag / Insert + set_emotional_tag
        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(Fact::new("Aris", "重视", "承诺").with_source("对话"))
                .unwrap();
            s.set_emotional_tag(&canonical, Some("信守".to_string()), 0.92)
                .unwrap();
        }

        // 重启 — 验证持久化 / Restart — verify persistence
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("承诺").unwrap();
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].emotional_tag.as_deref(), Some("信守"));
            assert!(
                (r[0].emotional_salience - 0.92).abs() < 1e-6,
                "salience 应持久为 0.92 / salience should persist as 0.92"
            );
            // 索引重启后重建 — emotional_tag 已纳入 / Index rebuilt after restart — emotional_tag included
            let r = s.query_by_emotion("信守").unwrap();
            assert_eq!(
                r.len(),
                1,
                "重启后索引应覆盖 emotional_tag / index should cover emotional_tag after restart"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_p2c_set_emotional_tag_not_found() {
        // 不存在的 canonical 应返回 Err / Non-existent canonical should return Err
        let s = new_store();
        let result = s.set_emotional_tag("不存在 | 的 | 键", Some("tag".to_string()), 0.5);
        assert!(
            result.is_err(),
            "不存在的 canonical 应返回 Err / non-existent canonical should return Err"
        );
    }

    // ══════════════════════════════════════════════════════════════════
    // P2-D 高价值标记测试 — pinned 字段 / High-Value Memory Markers tests
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn test_p2d_pinned_default_false() {
        // 新建 Fact 默认 pinned = false / New Fact defaults to pinned = false
        let fact = Fact::new("主人", "重视", "承诺");
        assert!(
            !fact.pinned,
            "pinned 默认应为 false / default should be false"
        );
    }

    #[test]
    fn test_p2d_pin_unpin_in_memory() {
        // pin/unpin 在内存中正确设置字段 / pin/unpin sets field correctly in memory
        let s = new_store();
        s.insert(Fact::new("主人", "重视", "承诺").with_source("对话"))
            .unwrap();
        let canonical = Fact::new("主人", "重视", "承诺").canonical_form();

        // 初始未 pin / Initially not pinned
        let r = s.query("承诺").unwrap();
        assert!(
            !r[0].pinned,
            "初始应为未 pin / initially should not be pinned"
        );

        // pin / Pin the fact
        s.pin(&canonical).unwrap();
        let r = s.query("承诺").unwrap();
        assert!(r[0].pinned, "pin 后应为 true / should be true after pin");

        // unpin / Unpin the fact
        s.unpin(&canonical).unwrap();
        let r = s.query("承诺").unwrap();
        assert!(
            !r[0].pinned,
            "unpin 后应为 false / should be false after unpin"
        );
    }

    #[test]
    fn test_p2d_pin_persists_sqlite() {
        // SQLite 后端 pin 持久化 — 重启后值保留
        // SQLite backend pin persistence — value survives restart
        let path = "./target/test_fact_p2d_pin.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        let canonical = Fact::new("Aris", "铭记", "那次告别").canonical_form();

        // 写入 + pin / Insert + pin
        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(Fact::new("Aris", "铭记", "那次告别").with_source("对话"))
                .unwrap();
            s.pin(&canonical).unwrap();
        }

        // 重启 — 验证 pinned 持久化 / Restart — verify pinned persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("那次告别").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                r[0].pinned,
                "pinned 应持久为 true / pinned should persist as true"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_p2d_pin_not_found() {
        // 不存在的 canonical pin 应返回 Err / Non-existent canonical pin should return Err
        let s = new_store();
        let result = s.pin("不存在 | 的 | 键");
        assert!(
            result.is_err(),
            "不存在的 canonical pin 应返回 Err / non-existent canonical pin should return Err"
        );
    }

    #[test]
    fn test_p2d_sqlite_migration_old_schema_pinned() {
        // SQLite 迁移 — 旧 schema 无 pinned 列 → 新 schema 补齐
        // SQLite migration — old schema without pinned column → new schema adds it
        let path = "./target/test_fact_p2d_migration.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 1. 用旧 schema 建表并写入数据（模拟 P2-C 版本数据库，无 pinned 列）
        // Create table with old schema (P2-C version, no pinned column)
        {
            let conn = Connection::open(path).unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .unwrap();
            conn.execute(
                "CREATE TABLE facts (
                    canonical TEXT PRIMARY KEY,
                    subject TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    source TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    verified_at INTEGER NOT NULL,
                    verify_count INTEGER NOT NULL,
                    emotion_context BLOB,
                    emotional_tag TEXT,
                    emotional_salience REAL DEFAULT 0.0
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO facts (canonical, subject, predicate, object, confidence, source,
                                    created_at, verified_at, verify_count, emotion_context,
                                    emotional_tag, emotional_salience)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    "旧数据 | 测试 | 值",
                    "旧数据",
                    "测试",
                    "值",
                    0.8_f64,
                    "legacy",
                    1000_i64,
                    1000_i64,
                    1_i64,
                    Option::<Vec<u8>>::None,
                    Option::<String>::None,
                    0.0_f64,
                ],
            )
            .unwrap();
        }

        // 2. 用 open_sqlite 打开 — 应自动迁移补齐 pinned 列
        // Open with open_sqlite — should auto-migrate to add pinned column
        {
            let s = FactStore::open_sqlite(path).unwrap();
            assert_eq!(s.count(), 1, "迁移后应保留 1 条事实 / should retain 1 fact");
            // 旧数据的 pinned 应为默认值 false / Legacy data should have default pinned = false
            let r = s.query("测试").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                !r[0].pinned,
                "旧数据 pinned 应为 false / legacy pinned should be false"
            );

            // 迁移后可正常 pin / After migration, pin works
            let canonical = r[0].canonical_form();
            s.pin(&canonical).unwrap();
        }

        // 3. 重新打开 — 验证迁移后的 pinned 值持久 / Reopen — verify migrated pinned value persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("测试").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                r[0].pinned,
                "迁移后 pin 的值应持久 / pinned set after migration should persist"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    // ════════════════════════════════════════════════════════════════════
    // P3-B 主动遗忘测试 — actively_forgotten 字段 / Active Forgetting tests
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_p3b_actively_forgotten_default_none() {
        // 新建 Fact 默认 actively_forgotten = None
        let fact = Fact::new("主人", "重视", "承诺");
        assert!(
            fact.actively_forgotten.is_none(),
            "actively_forgotten 默认应为 None / default should be None"
        );
        assert!(
            !fact.is_actively_forgotten(),
            "is_actively_forgotten 默认应为 false / should be false by default"
        );
    }

    #[test]
    fn test_p3b_mark_forgotten_in_memory() {
        // mark_forgotten 在内存中正确设置字段 / mark_forgotten sets field in memory
        let s = new_store();
        s.insert(Fact::new("主人", "重视", "承诺").with_source("对话"))
            .unwrap();
        let canonical = Fact::new("主人", "重视", "承诺").canonical_form();

        // 初始未遗忘 / Initially not forgotten
        let r = s.query("承诺").unwrap();
        assert!(
            !r[0].is_actively_forgotten(),
            "初始应为未遗忘 / initially not forgotten"
        );

        // mark_forgotten — TraumaProtection
        assert!(
            s.mark_forgotten(
                &canonical,
                crate::active_forget::ForgetPolicy::TraumaProtection
            ),
            "mark_forgotten 应返回 true / mark_forgotten should return true"
        );
        let r = s.query("承诺").unwrap();
        assert!(
            r[0].is_actively_forgotten(),
            "mark_forgotten 后应被标记 / should be marked after mark_forgotten"
        );
        assert_eq!(
            r[0].actively_forgotten.as_ref().unwrap(),
            &crate::active_forget::ForgetPolicy::TraumaProtection
        );

        // restore_forgotten — 清除标记 / Clear marker
        assert!(
            s.restore_forgotten(&canonical),
            "restore_forgotten 应返回 true / restore_forgotten should return true"
        );
        let r = s.query("承诺").unwrap();
        assert!(
            !r[0].is_actively_forgotten(),
            "restore_forgotten 后应清除标记 / should be cleared after restore_forgotten"
        );
    }

    #[test]
    fn test_p3b_mark_forgotten_not_found() {
        // 不存在的 canonical mark_forgotten 应返回 false / Non-existent canonical returns false
        let s = new_store();
        assert!(
            !s.mark_forgotten(
                "不存在 | 的 | 键",
                crate::active_forget::ForgetPolicy::TraumaProtection
            ),
            "不存在的 canonical mark_forgotten 应返回 false / non-existent canonical should return false"
        );
        assert!(
            !s.restore_forgotten("不存在 | 的 | 键"),
            "不存在的 canonical restore_forgotten 应返回 false / non-existent canonical should return false"
        );
    }

    #[test]
    fn test_p3b_mark_forgotten_persists() {
        // SQLite 后端 mark_forgotten 持久化 — 重启后 actively_forgotten 仍存在
        // SQLite backend mark_forgotten persistence — actively_forgotten survives restart
        let path = "./target/test_fact_p3b_mark.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        let canonical = Fact::new("Aris", "隐藏", "那段记忆").canonical_form();

        // 写入 + mark_forgotten / Insert + mark_forgotten
        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(Fact::new("Aris", "隐藏", "那段记忆").with_source("对话"))
                .unwrap();
            assert!(s.mark_forgotten(
                &canonical,
                crate::active_forget::ForgetPolicy::TraumaProtection
            ));
        }

        // 重启 — 验证 actively_forgotten 持久化 / Restart — verify actively_forgotten persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("那段记忆").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                r[0].is_actively_forgotten(),
                "actively_forgotten 应持久为 Some / actively_forgotten should persist as Some"
            );
            assert_eq!(
                r[0].actively_forgotten.as_ref().unwrap(),
                &crate::active_forget::ForgetPolicy::TraumaProtection
            );

            // restore_forgotten 也持久化 / restore_forgotten also persists
            assert!(s.restore_forgotten(&canonical));
        }

        // 再次重启 — 验证 restore 持久化 / Reopen again — verify restore persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("那段记忆").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                !r[0].is_actively_forgotten(),
                "restore_forgotten 后标记应已持久清除 / marker should be persistently cleared after restore_forgotten"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_p3b_merge_confidence_lowers_and_persists() {
        // merge_confidence 降低置信度并持久化 / merge_confidence lowers confidence and persists
        let path = "./target/test_fact_p3b_merge.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        let canonical = Fact::new("Aris", "测试", "置信度").canonical_form();
        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(
                Fact::new("Aris", "测试", "置信度")
                    .with_source("对话")
                    .with_confidence(0.9),
            )
            .unwrap();
            // 初始置信度 ≈ 0.9
            let r = s.query("置信度").unwrap();
            assert!((r[0].confidence - 0.9).abs() < 1e-6);

            // merge_confidence(0.1) — 加权平均后置信度应下降
            assert!(s.merge_confidence(&canonical, 0.1));
            let r = s.query("置信度").unwrap();
            assert!(
                r[0].confidence < 0.9,
                "merge_confidence(0.1) 后置信度应下降 / confidence should drop after merge_confidence(0.1), got {}",
                r[0].confidence
            );
        }

        // 重启 — 验证置信度持久化 / Restart — verify confidence persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("置信度").unwrap();
            assert!(
                r[0].confidence < 0.9,
                "重启后置信度应仍为降低后的值 / confidence should remain lowered after restart"
            );
            assert_eq!(r[0].verify_count, 2, "merge_confidence 应递增 verify_count");
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_p3b_get_recent_facts_desc() {
        // get_recent_facts 按 created_at 降序返回 / Returns facts sorted by created_at desc
        let s = new_store();
        // 插入时 created_at 为 now_secs()，难以区分顺序 — 手动构造不同 created_at 的事实
        let mut f1 = Fact::new("A", "is", "1");
        f1.created_at = 1000;
        let mut f2 = Fact::new("B", "is", "2");
        f2.created_at = 3000;
        let mut f3 = Fact::new("C", "is", "3");
        f3.created_at = 2000;
        s.insert(f1).unwrap();
        s.insert(f2).unwrap();
        s.insert(f3).unwrap();

        let recent = s.get_recent_facts(2);
        assert_eq!(recent.len(), 2, "应返回 2 条事实 / should return 2 facts");
        // 最新 (created_at=3000) 在前 / Most recent (created_at=3000) first
        assert_eq!(recent[0].1.object, "2");
        assert_eq!(recent[1].1.object, "3");

        // n=0 时不 panic（truncate(n.max(1)) 保证至少 1）/ n=0 does not panic
        let _ = s.get_recent_facts(0);
    }

    // ════════════════════════════════════════════════════════════════════
    // P3-C 强化学习闭环测试 — adjust_confidence_by_feedback / Reinforcement tests
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_adjust_confidence_clamp_high() {
        // delta=0.5 应 clamp 到 0.2，confidence 从 0.5 增到 0.7
        // delta=0.5 should clamp to 0.2, confidence goes from 0.5 to 0.7
        let s = new_store();
        s.insert(
            Fact::new("主人", "喜欢", "Rust")
                .with_source("对话")
                .with_confidence(0.5),
        )
        .unwrap();
        let canonical = Fact::new("主人", "喜欢", "Rust").canonical_form();
        assert!(
            s.adjust_confidence_by_feedback(&canonical, 0.5),
            "adjust_confidence_by_feedback 应返回 true / should return true"
        );
        let r = s.query("Rust").unwrap();
        assert_eq!(r.len(), 1);
        assert!(
            (r[0].confidence - 0.7).abs() < 1e-6,
            "delta=0.5 clamp 到 0.2，0.5 + 0.2 = 0.7，实际 {} / got {}",
            r[0].confidence,
            r[0].confidence
        );
    }

    #[test]
    fn test_adjust_confidence_clamp_low() {
        // delta=-0.5 应 clamp 到 -0.2，confidence 从 0.5 减到 0.3
        // delta=-0.5 should clamp to -0.2, confidence goes from 0.5 to 0.3
        let s = new_store();
        s.insert(
            Fact::new("主人", "讨厌", "Bug")
                .with_source("对话")
                .with_confidence(0.5),
        )
        .unwrap();
        let canonical = Fact::new("主人", "讨厌", "Bug").canonical_form();
        assert!(
            s.adjust_confidence_by_feedback(&canonical, -0.5),
            "adjust_confidence_by_feedback 应返回 true / should return true"
        );
        let r = s.query("Bug").unwrap();
        assert_eq!(r.len(), 1);
        assert!(
            (r[0].confidence - 0.3).abs() < 1e-6,
            "delta=-0.5 clamp 到 -0.2，0.5 - 0.2 = 0.3，实际 {} / got {}",
            r[0].confidence,
            r[0].confidence
        );
    }

    #[test]
    fn test_adjust_confidence_boundary() {
        // confidence + delta 超过 1.0 或低于 0.0 时 clamp 到 [0,1]
        // confidence + delta exceeding 1.0 or below 0.0 clamps to [0,1]
        let s = new_store();

        // 上界：0.95 + 0.2 = 1.15 → clamp 到 1.0
        // Upper bound: 0.95 + 0.2 = 1.15 → clamp to 1.0
        s.insert(
            Fact::new("A", "is", "high")
                .with_source("test")
                .with_confidence(0.95),
        )
        .unwrap();
        let key_high = Fact::new("A", "is", "high").canonical_form();
        s.adjust_confidence_by_feedback(&key_high, 0.2);
        let r = s.query("high").unwrap();
        assert!(
            (r[0].confidence - 1.0).abs() < 1e-6,
            "0.95 + 0.2 = 1.15 应 clamp 到 1.0，实际 {} / got {}",
            r[0].confidence,
            r[0].confidence
        );

        // 下界：0.1 + (-0.2) = -0.1 → clamp 到 0.0
        // Lower bound: 0.1 + (-0.2) = -0.1 → clamp to 0.0
        s.insert(
            Fact::new("B", "is", "low")
                .with_source("test")
                .with_confidence(0.1),
        )
        .unwrap();
        let key_low = Fact::new("B", "is", "low").canonical_form();
        s.adjust_confidence_by_feedback(&key_low, -0.2);
        let r = s.query("low").unwrap();
        assert!(
            (r[0].confidence - 0.0).abs() < 1e-6,
            "0.1 - 0.2 = -0.1 应 clamp 到 0.0，实际 {} / got {}",
            r[0].confidence,
            r[0].confidence
        );
    }

    #[test]
    fn test_adjust_confidence_not_found() {
        // 不存在的 canonical 应返回 false / Non-existent canonical returns false
        let s = new_store();
        assert!(
            !s.adjust_confidence_by_feedback("不存在 | 的 | 键", 0.1),
            "不存在的 canonical 应返回 false / non-existent canonical should return false"
        );
    }

    #[test]
    fn test_adjust_confidence_persists_sqlite() {
        // SQLite 后端 adjust_confidence_by_feedback 持久化 — 重启后值保留
        // SQLite backend persistence — value survives restart
        let path = "./target/test_fact_p3c_adjust.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        let canonical = Fact::new("Aris", "学习", "Rust").canonical_form();

        // 写入 + adjust_confidence_by_feedback / Insert + adjust
        {
            let s = FactStore::open_sqlite(path).unwrap();
            s.insert(
                Fact::new("Aris", "学习", "Rust")
                    .with_source("对话")
                    .with_confidence(0.5),
            )
            .unwrap();
            assert!(s.adjust_confidence_by_feedback(&canonical, 0.2));
        }

        // 重启 — 验证置信度持久化 / Restart — verify confidence persists
        {
            let s = FactStore::open_sqlite(path).unwrap();
            let r = s.query("Rust").unwrap();
            assert_eq!(r.len(), 1);
            assert!(
                (r[0].confidence - 0.7).abs() < 1e-6,
                "adjust 后置信度应持久为 0.7 / adjusted confidence should persist as 0.7"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    /// 测试 2.1 — open_sqlite 成功创建文件：验证诊断增强后的 open_sqlite 仍能正常创建 SQLite 文件
    /// Test 2.1 — open_sqlite creates file: verify open_sqlite (with enhanced diagnostics) still creates the SQLite file
    #[test]
    fn test_open_sqlite_creates_file() {
        // 使用 temp_dir + 进程 id 避免并行测试冲突 / Use temp_dir + pid to avoid parallel-test conflicts
        let temp_path =
            std::env::temp_dir().join(format!("atrium_test_factstore_{}.db", std::process::id()));
        let temp_path = temp_path.to_str().unwrap();
        let _ = std::fs::remove_file(temp_path);
        let _ = std::fs::remove_file(format!("{}-wal", temp_path));
        let _ = std::fs::remove_file(format!("{}-shm", temp_path));

        let store = FactStore::open_sqlite(temp_path).unwrap();
        assert!(
            std::path::Path::new(temp_path).exists(),
            "SQLite 文件应被创建 / SQLite file should be created"
        );

        // 清理 / Cleanup
        drop(store);
        let _ = std::fs::remove_file(temp_path);
        // 清理 WAL/SHM 文件 / Cleanup WAL/SHM files
        let _ = std::fs::remove_file(format!("{}-wal", temp_path));
        let _ = std::fs::remove_file(format!("{}-shm", temp_path));
    }
}
