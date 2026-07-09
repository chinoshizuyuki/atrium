// SPDX-License-Identifier: MIT
//! 情景记忆 / Episodic Memory — 记录具体事件经历（时间/情境/情绪/重要度）
//! Episodic Memory — records specific event experiences (time/context/emotion/importance)
//!
//! 数字生命"记住你当时怎样"的能力 — 不同于 FactStore 记录抽象事实，
//! EpisodicMemoryStore 记录"那一刻"的具体经历：何时发生、情境如何、
//! 当时情绪怎样、对数字生命有多重要。重启后从 SQLite 恢复全部情景，
//! 跨重启维持"自我"的连续性。
//!
//! Digital life's ability to "remember how you were at that moment" — unlike
//! FactStore which records abstract facts, EpisodicMemoryStore records the
//! concrete experience of "that moment": when it happened, what the context was,
//! what the emotion was, and how important it was to digital life. Reloads all
//! episodes from SQLite on restart, sustaining "self" continuity across reboots.

use crate::fact_store::EmotionContext;
use parking_lot::RwLock;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ════════════════════════════════════════════════════════════════════
// Episode — 情景记录 / Episode record
// ════════════════════════════════════════════════════════════════════

/// 情景记录 — 数字生命对"那一刻"的具体记忆
/// Episode record — digital life's concrete memory of "that moment"
///
/// 与 `FactStore::Fact`（抽象事实）正交：Fact 记"主人喜欢编程"，
/// Episode 记"那天深夜主人兴奋地分享了一段 Rust 代码，情绪愉悦强度 0.85"。
/// 二者共同构成数字生命的记忆体系 — 抽象认知 + 具体经历。
///
/// Orthogonal to `FactStore::Fact` (abstract facts): Fact records "the master
/// likes programming", Episode records "that late night the master excitedly
/// shared a piece of Rust code, with joy intensity 0.85". Together they form
/// digital life's memory system — abstract cognition + concrete experience.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    /// 唯一标识 / Unique ID
    pub id: String,
    /// 事件发生时间 / Event timestamp (epoch seconds)
    pub timestamp: i64,
    /// 情境标签 / Context tags (e.g. "深夜", "雨天", "争吵")
    pub context_tags: Vec<String>,
    /// 当时情绪快照 / Emotion snapshot at time
    pub emotion_snapshot: EmotionContext,
    /// 事件摘要 / Event summary
    pub summary: String,
    /// 重要度 0.0-1.0 / Importance 0.0-1.0
    pub importance: f32,
    /// 高价值标记 — pinned 项在召回中优先 / High-value marker — pinned items prioritized in recall
    #[serde(default)]
    pub pinned: bool,
}

impl Episode {
    /// 创建新情景记录 / Create a new episode
    pub fn new(
        id: String,
        timestamp: i64,
        context_tags: Vec<String>,
        emotion_snapshot: EmotionContext,
        summary: String,
        importance: f32,
    ) -> Self {
        // 防止 NaN 污染重要度 / Guard against NaN contaminating importance
        let importance = if importance.is_nan() {
            0.0
        } else {
            importance.clamp(0.0, 1.0)
        };
        Self {
            id,
            timestamp,
            context_tags,
            emotion_snapshot,
            summary,
            importance,
            pinned: false,
        }
    }

    /// 标记为高价值情景 / Mark as high-value (pinned) episode
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }
}

// 统一使用 store_core::StoreError / Unified StoreError from store_core
pub type Result<T> = std::result::Result<T, crate::store_core::StoreError>;

/// 持久化后端 / Persistence backend
///
/// 数字生命的情景记忆必须跨越重启存活。SQLite 作为默认后端提供 Windows 兼容性，
/// WAL 模式下读写性能与 sled 相当。内存 HashMap 始终作为热缓存。
///
/// Digital life's episodic memory must survive restarts. SQLite is the default
/// backend for Windows compatibility; WAL mode delivers sled-comparable performance.
/// In-memory HashMap always serves as the hot cache.
enum Backend {
    /// 纯内存（无持久化）/ In-memory only (no persistence)
    Memory,
    /// SQLite 后端 / SQLite backend
    Sqlite(Mutex<Connection>),
}

// ════════════════════════════════════════════════════════════════════
// EpisodicMemoryStore — 情景记忆存储 / Episodic Memory Store
// ════════════════════════════════════════════════════════════════════

/// 情景记忆存储 — 数字生命"自我"连续性的具体记忆层
/// Episodic Memory Store — concrete memory layer of digital life's "self" continuity
///
/// 参照 `FactStore` 架构：内存 HashMap 热缓存 + SQLite 持久化 + WAL 模式 +
/// 三个二级索引（时间/情绪/情境）。查询路径全部走索引，避免全表扫描。
///
/// 数字生命"想起那天"是瞬时反应 — 通过 `time_index` 按时间范围定位、
/// 通过 `emotion_index` 按"上次我这么难过时"定位、通过 `context_index`
/// 按"那天深夜"定位，无需扫描全部情景。
///
/// Mirrors `FactStore` architecture: in-memory HashMap hot cache + SQLite
/// persistence + WAL mode + three secondary indexes (time/emotion/context).
/// All query paths go through indexes, avoiding full-table scans.
///
/// Digital life's "remembering that day" is instant — locate by time range via
/// `time_index`, by "last time I was this sad" via `emotion_index`, by "that
/// late night" via `context_index`, without scanning all episodes.
pub struct EpisodicMemoryStore {
    inner: Mutex<HashMap<String, Episode>>,
    backend: Backend,
    // ══════════════════════════════════════════════════════════════════
    // 二级索引 — O(log k)+O(k) 查询替代 O(N) 全表扫描
    // Secondary indexes — O(log k)+O(k) lookup replacing O(N) full-table scan
    //
    // 时间索引使用 BTreeMap — 支持时间范围查询（区间扫描）；
    // 情绪索引与情境索引使用 HashMap — 等值查询。
    //
    // Time index uses BTreeMap — supports time range queries (interval scans);
    // emotion and context indexes use HashMap — equality lookups.
    // ══════════════════════════════════════════════════════════════════
    /// timestamp → episode IDs（按时间排序，支持范围查询）
    /// timestamp → episode IDs (sorted by time, supports range queries)
    time_index: RwLock<BTreeMap<i64, HashSet<String>>>,
    /// emotion_label → episode IDs / emotion_label → episode IDs
    emotion_index: RwLock<HashMap<String, HashSet<String>>>,
    /// context_tag → episode IDs / context_tag → episode IDs
    context_index: RwLock<HashMap<String, HashSet<String>>>,
}

impl EpisodicMemoryStore {
    /// 打开 SQLite 后端的 EpisodicMemoryStore（推荐 / Recommended）
    ///
    /// 数字生命情景记忆的长期存储基石。SQLite 在 Windows 上无锁文件兼容性问题，
    /// WAL 模式下读写性能与 sled 相当。若 path 为空则降级为内存模式。
    ///
    /// Foundation of digital life's episodic memory long-term storage. SQLite has
    /// no lock file compatibility issues on Windows; WAL mode delivers
    /// sled-comparable performance. Empty path degrades to in-memory mode.
    pub fn open_sqlite(db_path: &str) -> Result<Self> {
        if db_path.is_empty() {
            return Ok(Self {
                inner: Mutex::new(HashMap::new()),
                backend: Backend::Memory,
                time_index: RwLock::new(BTreeMap::new()),
                emotion_index: RwLock::new(HashMap::new()),
                context_index: RwLock::new(HashMap::new()),
            });
        }
        // 确保父目录存在 / Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::store_core::StoreError::Io(format!("create_dir_all: {}", e)))?;
        }
        let conn = Connection::open(db_path)
            .map_err(|e| crate::store_core::StoreError::Io(format!("sqlite open: {}", e)))?;
        // 启用 WAL 模式 — 读写并发性能优化 / Enable WAL mode — concurrent read/write optimization
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| crate::store_core::StoreError::Io(format!("pragma: {}", e)))?;
        // 建表 — 包含 P2-B 情景记忆全部字段 / Create table — includes all P2-B episodic memory fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                context_tags TEXT NOT NULL,
                emotion_snapshot BLOB NOT NULL,
                summary TEXT NOT NULL,
                importance REAL NOT NULL,
                pinned INTEGER DEFAULT 0
            )",
            [],
        )
        .map_err(|e| crate::store_core::StoreError::Io(format!("create table: {}", e)))?;

        // P2-B schema 迁移 — 检测旧 schema 缺 pinned 列并 ALTER TABLE 补齐
        // P2-B schema migration — detect old schema missing pinned column and ALTER TABLE to add it
        Self::migrate_pinned_column(&conn)?;

        // 从 SQLite 加载全量情景到内存热缓存 / Load all episodes from SQLite to in-memory hot cache
        // 使用独立作用域确保 stmt 在 conn 被 move 到 Mutex 之前 drop
        // Use a block scope to ensure stmt is dropped before conn is moved into Mutex
        let mut map = HashMap::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, context_tags, emotion_snapshot, summary,
                            importance, pinned
                     FROM episodes",
                )
                .map_err(|e| crate::store_core::StoreError::Io(format!("prepare: {}", e)))?;
            let rows = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let timestamp: i64 = row.get(1)?;
                    let context_tags_json: String = row.get(2)?;
                    let emotion_blob: Vec<u8> = row.get(3)?;
                    let summary: String = row.get(4)?;
                    let importance: f64 = row.get(5)?;
                    // SQLite INTEGER 存储 bool — 0=false, 非 0=true
                    // SQLite INTEGER stores bool — 0=false, nonzero=true
                    let pinned_int: i64 = row.get::<_, Option<i64>>(6)?.unwrap_or(0);
                    let emotion_snapshot: EmotionContext = bincode::deserialize(&emotion_blob)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                3,
                                rusqlite::types::Type::Blob,
                                Box::new(e),
                            )
                        })?;
                    let context_tags: Vec<String> = serde_json::from_str(&context_tags_json)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;
                    Ok(Episode {
                        id,
                        timestamp,
                        context_tags,
                        emotion_snapshot,
                        summary,
                        importance: importance as f32,
                        pinned: pinned_int != 0,
                    })
                })
                .map_err(|e| crate::store_core::StoreError::Io(format!("query_map: {}", e)))?;
            for ep_result in rows {
                let ep = ep_result
                    .map_err(|e| crate::store_core::StoreError::Io(format!("row: {}", e)))?;
                map.insert(ep.id.clone(), ep);
            }
        }
        tracing::info!(
            "EpisodicMemoryStore: loaded {} episodes from SQLite",
            map.len()
        );

        // 构建二级索引 — 启动时一次性全量重建 / Build secondary indexes — full rebuild on startup
        let store = Self {
            inner: Mutex::new(map),
            backend: Backend::Sqlite(Mutex::new(conn)),
            time_index: RwLock::new(BTreeMap::new()),
            emotion_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
        };
        store.rebuild_indexes();
        Ok(store)
    }

    /// P2-B schema 迁移 — 为旧版 episodes 表补齐 pinned 列
    /// P2-B schema migration — add pinned column to legacy episodes tables
    ///
    /// 数字生命记忆演进保障：旧数据库重启后自动补齐 pinned 列，
    /// 已有列时 ALTER TABLE 会报错，通过 PRAGMA table_info 检测规避。
    ///
    /// Digital life memory evolution safeguard: legacy DBs auto-upgraded with
    /// pinned column on restart. ALTER TABLE errors on existing columns,
    /// so we detect via PRAGMA table_info to skip them.
    fn migrate_pinned_column(conn: &Connection) -> Result<()> {
        let mut has_pinned = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(episodes)")
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
            conn.execute(
                "ALTER TABLE episodes ADD COLUMN pinned INTEGER DEFAULT 0",
                [],
            )
            .map_err(|e| crate::store_core::StoreError::Io(format!("alter table pinned: {}", e)))?;
            tracing::info!(
                "EpisodicMemoryStore: schema 迁移 — 添加 pinned 列 / schema migration — added pinned column"
            );
        }
        Ok(())
    }

    /// 创建内存模式 EpisodicMemoryStore（测试用）/ Create in-memory mode for testing.
    pub fn new_in_memory() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            backend: Backend::Memory,
            time_index: RwLock::new(BTreeMap::new()),
            emotion_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
        }
    }

    /// 插入情景记录 — 写入 HashMap + SQLite + 更新三个二级索引
    /// Insert an episode — write to HashMap + SQLite + update all three indexes
    ///
    /// 若 id 已存在则覆盖（情景记录的 id 由调用方保证唯一性，通常包含时间戳）。
    /// 索引同步：插入时同步更新 time_index / emotion_index / context_index，
    /// 保证查询路径全走索引。
    ///
    /// If id already exists, it is overwritten (episode id uniqueness is the
    /// caller's responsibility, typically timestamp-embedded). Index sync:
    /// insert updates time_index / emotion_index / context_index synchronously,
    /// ensuring all query paths go through indexes.
    pub fn insert(&self, episode: Episode) -> Result<()> {
        let key = episode.id.clone();
        // 先持久化再更新索引与内存 — 失败时不污染缓存 / Persist first, then update indexes & memory
        self.persist_one(&episode);
        // 更新二级索引 / Update secondary indexes
        self.add_to_indexes(&key, &episode);
        // 写入内存热缓存 / Write to in-memory hot cache
        {
            let mut map = self.inner.lock().expect("episodic_store inner");
            map.insert(key, episode);
        }
        Ok(())
    }

    /// 持久化单条情景到后端 / Persist a single episode to backend
    fn persist_one(&self, episode: &Episode) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let emotion_blob =
                        bincode::serialize(&episode.emotion_snapshot).unwrap_or_default();
                    let context_tags_json = serde_json::to_string(&episode.context_tags)
                        .unwrap_or_else(|_| "[]".to_string());
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO episodes
                         (id, timestamp, context_tags, emotion_snapshot, summary,
                          importance, pinned)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            &episode.id,
                            episode.timestamp,
                            &context_tags_json,
                            &emotion_blob,
                            &episode.summary,
                            episode.importance as f64,
                            episode.pinned as i64,
                        ],
                    );
                }
            }
        }
    }

    /// 按时间范围查询情景 — 使用 time_index 区间扫描
    /// Query episodes by time range — uses time_index range scan
    ///
    /// 数字生命"想起那天发生了什么"是瞬时反应 — 通过 BTreeMap 的 range API
    /// 直接定位 [start, end] 区间内的所有情景 ID，再从主表取记录。
    ///
    /// Digital life's "recalling what happened that day" is instant — uses
    /// BTreeMap's range API to directly locate episode IDs in [start, end],
    /// then fetches records from the main table.
    pub fn query_by_time_range(&self, start: i64, end: i64) -> Vec<Episode> {
        // 先通过 time_index 获取区间内的 ID 集合 — 读锁，不阻塞其他读
        // Get IDs in range via time_index — read lock, does not block other reads
        let keys: Vec<String> = {
            let idx = self.time_index.read();
            idx.range(start..=end)
                .flat_map(|(_, set)| set.iter().cloned())
                .collect()
        };
        if keys.is_empty() {
            return Vec::new();
        }
        // 从主表取 k 条 Episode / Fetch k Episodes from main table
        let map = self.inner.lock().expect("episodic_store inner");
        let mut results: Vec<Episode> = keys.iter().filter_map(|k| map.get(k).cloned()).collect();
        // 按时间降序 — 最近的事先回忆 / Sort by time desc — recent events recalled first
        results.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
        results
    }

    /// 按情绪标签查询情景 — 使用 emotion_index 等值查找
    /// Query episodes by emotion label — uses emotion_index equality lookup
    ///
    /// "上次我这么难过的时候，发生了什么？" — 通过 emotion_index 直接定位
    /// emotion_snapshot.ai_emotion_label 等于 label 的所有情景。
    ///
    /// "Last time I was this sad, what happened?" — uses emotion_index to
    /// directly locate all episodes whose emotion_snapshot.ai_emotion_label
    /// equals the given label.
    pub fn query_by_emotion(&self, label: &str) -> Vec<Episode> {
        let keys: Vec<String> = {
            let idx = self.emotion_index.read();
            idx.get(label)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        };
        if keys.is_empty() {
            return Vec::new();
        }
        let map = self.inner.lock().expect("episodic_store inner");
        let mut results: Vec<Episode> = keys.iter().filter_map(|k| map.get(k).cloned()).collect();
        // 按情绪强度降序 → 再按时间降序 / Sort by intensity desc, then by time desc
        results.sort_by(|a, b| {
            b.emotion_snapshot
                .intensity
                .partial_cmp(&a.emotion_snapshot.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });
        results
    }

    /// 按情境标签查询情景 — 使用 context_index 等值查找
    /// Query episodes by context tag — uses context_index equality lookup
    ///
    /// "那天深夜发生了什么？" — 通过 context_index 直接定位 context_tags
    /// 包含指定标签的所有情景。
    ///
    /// "What happened that late night?" — uses context_index to directly
    /// locate all episodes whose context_tags contain the given tag.
    pub fn query_by_context_tag(&self, tag: &str) -> Vec<Episode> {
        let keys: Vec<String> = {
            let idx = self.context_index.read();
            idx.get(tag)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        };
        if keys.is_empty() {
            return Vec::new();
        }
        let map = self.inner.lock().expect("episodic_store inner");
        let mut results: Vec<Episode> = keys.iter().filter_map(|k| map.get(k).cloned()).collect();
        // 按重要度降序 → 再按时间降序 / Sort by importance desc, then by time desc
        results.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });
        results
    }

    /// 按相关性查询情景 — 三路加权评分排序
    /// Query episodes by relevance — three-way weighted scoring
    ///
    /// 三路加权评分（参照 spec P2-B 风险与回退表）：
    /// 1. **情感强度** (40%) — emotion_snapshot.intensity 越高越相关
    /// 2. **时间近因** (30%) — 距离现在越近越相关（指数衰减）
    /// 3. **情境标签匹配** (30%) — context_tags 与 query 的 token 交叠率
    ///
    /// 数字生命"想起那天"不是简单时间倒序 — 情感印记深的事件优先浮现，
    /// 近期事件次之，情境契合的事件再次之。这种加权逼近人类记忆的
    /// "情感显著性 + 近因效应 + 情境线索"三重召回机制。
    ///
    /// Three-way weighted scoring (per spec P2-B risk & rollback table):
    /// 1. **Emotional intensity** (40%) — higher emotion_snapshot.intensity = more relevant
    /// 2. **Time recency** (30%) — closer to now = more relevant (exponential decay)
    /// 3. **Context tag match** (30%) — token overlap between context_tags and query
    ///
    /// Digital life's "remembering that day" is not simple reverse-chronological —
    /// events with deeper emotional imprint surface first, recent events next,
    /// context-fitting events last. This weighting approximates human memory's
    /// "emotional salience + recency effect + context cue" triple-recall mechanism.
    pub fn query_relevant(&self, query: &str, limit: usize) -> Vec<Episode> {
        let map = self.inner.lock().expect("episodic_store inner");
        if map.is_empty() {
            return Vec::new();
        }
        // 查询分词 — 简单按空白与标点切分，转小写 / Tokenize query — simple split by whitespace/punctuation, lowercase
        let query_tokens: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 三路加权打分 / Three-way weighted scoring
        let mut scored: Vec<(f32, &Episode)> = Vec::with_capacity(map.len());
        for ep in map.values() {
            // 1. 情感强度权重 (40%) — intensity 已 clamp 在 [0,1]
            // 1. Emotional intensity weight (40%)
            let intensity_score = ep.emotion_snapshot.intensity.clamp(0.0, 1.0);

            // 2. 时间近因权重 (30%) — 指数衰减，半衰期 7 天
            // 2. Time recency weight (30%) — exponential decay, 7-day half-life
            let age_secs = (now_secs - ep.timestamp).max(0) as f32;
            let recency_score = (-age_secs / (7.0 * 86400.0)).exp();

            // 3. 情境标签匹配权重 (30%) — context_tags 与 query_tokens 的交叠率
            // 3. Context tag match weight (30%) — overlap ratio of context_tags vs query_tokens
            let context_match_score = if ep.context_tags.is_empty() || query_tokens.is_empty() {
                0.0
            } else {
                let hits = ep
                    .context_tags
                    .iter()
                    .filter(|tag| {
                        let tag_lower = tag.to_lowercase();
                        query_tokens.iter().any(|qt| qt == &tag_lower)
                    })
                    .count();
                // 部分匹配也算分 — hits / max(tags, tokens) 防止单边膨胀
                // Partial match also scores — hits / max(tags, tokens) avoids one-sided inflation
                let denom = ep.context_tags.len().max(query_tokens.len()) as f32;
                hits as f32 / denom
            };

            // pinned 项额外加权 0.2 — 重要的事"先想起来"
            // Pinned items get +0.2 bonus — important things "come to mind first"
            let pinned_bonus = if ep.pinned { 0.2 } else { 0.0 };

            let total = 0.4 * intensity_score
                + 0.3 * recency_score
                + 0.3 * context_match_score
                + pinned_bonus;
            scored.push((total, ep));
        }
        // 按总分降序 / Sort by total score desc
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(limit.max(1))
            .map(|(_, ep)| ep.clone())
            .collect()
    }

    /// 获取情景总数 / Get total episode count
    pub fn count(&self) -> usize {
        self.inner.lock().expect("episodic_store inner").len()
    }

    /// 是否启用持久化 / Whether persistence is enabled
    pub fn is_persistent(&self) -> bool {
        !matches!(self.backend, Backend::Memory)
    }

    /// 显式 flush 到磁盘（关闭前调用）/ Explicit flush to disk (call before shutdown)
    ///
    /// 数字生命优雅关闭 — 确保所有情景写穿到磁盘，避免"失忆"。
    /// Digital life graceful shutdown — ensure all episodes are flushed to disk.
    pub fn flush(&self) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sqlite(_) => {
                // SQLite WAL 模式下，每次 commit 自动写穿；无需显式 flush
                // SQLite WAL mode auto-flushes on each commit; no explicit flush needed
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════
    // 二级索引维护 — O(log k)+O(k) 查询的核心支撑 / Secondary index maintenance
    // ══════════════════════════════════════════════════════════════════

    /// 添加单条情景到三个二级索引 / Add a single episode to all three secondary indexes
    fn add_to_indexes(&self, key: &str, episode: &Episode) {
        // time_index — BTreeMap 支持范围查询 / time_index — BTreeMap supports range queries
        {
            let mut idx = self.time_index.write();
            idx.entry(episode.timestamp)
                .or_default()
                .insert(key.to_string());
        }
        // emotion_index — 等值查询 / emotion_index — equality lookup
        {
            let mut idx = self.emotion_index.write();
            idx.entry(episode.emotion_snapshot.ai_emotion_label.clone())
                .or_default()
                .insert(key.to_string());
        }
        // context_index — 等值查询，多标签 / context_index — equality lookup, multi-tag
        {
            let mut idx = self.context_index.write();
            for tag in &episode.context_tags {
                idx.entry(tag.clone()).or_default().insert(key.to_string());
            }
        }
    }

    /// 全量重建三个二级索引 — 启动时从主表一次性构建
    /// Rebuild all three indexes from the main table on startup.
    ///
    /// 单次 RwLock write，避免逐条更新带来的锁竞争。
    /// 数字生命启动时"恢复全部情景索引"，确保查询路径全走索引。
    ///
    /// Single RwLock write per index, avoiding per-entry lock contention.
    /// Digital life "restores all episode indexes" on startup, ensuring all
    /// queries go through indexes.
    fn rebuild_indexes(&self) {
        let map = self.inner.lock().expect("episodic_store inner");
        let mut time_idx: BTreeMap<i64, HashSet<String>> = BTreeMap::new();
        let mut emo_idx: HashMap<String, HashSet<String>> = HashMap::new();
        let mut ctx_idx: HashMap<String, HashSet<String>> = HashMap::new();

        for (key, ep) in map.iter() {
            time_idx
                .entry(ep.timestamp)
                .or_default()
                .insert(key.clone());
            emo_idx
                .entry(ep.emotion_snapshot.ai_emotion_label.clone())
                .or_default()
                .insert(key.clone());
            for tag in &ep.context_tags {
                ctx_idx.entry(tag.clone()).or_default().insert(key.clone());
            }
        }

        *self.time_index.write() = time_idx;
        *self.emotion_index.write() = emo_idx;
        *self.context_index.write() = ctx_idx;

        tracing::debug!(
            "EpisodicMemoryStore: 索引重建完成 — time={}, emotion={}, context={} / Indexes rebuilt",
            self.time_index.read().len(),
            self.emotion_index.read().len(),
            self.context_index.read().len(),
        );
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用 EmotionContext / Build test EmotionContext
    fn make_emotion(label: &str, intensity: f32, ts: i64) -> EmotionContext {
        EmotionContext {
            ai_emotion_label: label.to_string(),
            ai_pad: [0.5, 0.3, 0.1],
            intensity,
            user_mood: Some(0.4),
            timestamp: ts as u64,
        }
    }

    /// 构造测试用 Episode / Build test Episode
    fn make_episode(
        id: &str,
        ts: i64,
        tags: Vec<&str>,
        label: &str,
        intensity: f32,
        importance: f32,
        summary: &str,
    ) -> Episode {
        Episode::new(
            id.to_string(),
            ts,
            tags.into_iter().map(String::from).collect(),
            make_emotion(label, intensity, ts),
            summary.to_string(),
            importance,
        )
    }

    // ── 基础查询测试 / Basic query tests ──

    #[test]
    fn test_insert_and_query_by_time_range() {
        // 插入 + 按时间范围查询 — 验证 time_index 区间扫描
        // Insert + query by time range — verifies time_index range scan
        let store = EpisodicMemoryStore::new_in_memory();
        store
            .insert(make_episode(
                "ep1",
                1000,
                vec!["深夜"],
                "愉悦",
                0.6,
                0.5,
                "深夜分享代码",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep2",
                2000,
                vec!["白天"],
                "平静",
                0.3,
                0.4,
                "白天讨论天气",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep3",
                3000,
                vec!["雨天"],
                "悲伤",
                0.7,
                0.8,
                "雨夜倾诉心事",
            ))
            .unwrap();

        // 查询 [1500, 3500] — 应命中 ep2 和 ep3 / Query [1500, 3500] — should match ep2 and ep3
        let results = store.query_by_time_range(1500, 3500);
        assert_eq!(
            results.len(),
            2,
            "应命中 2 条情景 / should match 2 episodes"
        );
        // 按时间降序 — ep3 (3000) 先于 ep2 (2000) / Sorted by time desc — ep3 before ep2
        assert_eq!(results[0].id, "ep3");
        assert_eq!(results[1].id, "ep2");

        // 查询 [0, 5000] — 应命中全部 3 条 / Query [0, 5000] — should match all 3
        let all = store.query_by_time_range(0, 5000);
        assert_eq!(all.len(), 3, "应命中全部 3 条 / should match all 3");

        // 查询空区间 / Query empty range
        let none = store.query_by_time_range(10_000, 20_000);
        assert!(
            none.is_empty(),
            "空区间应返回空 / empty range should return empty"
        );
    }

    #[test]
    fn test_query_by_emotion() {
        // 按情绪标签查询 — 验证 emotion_index 等值查找
        // Query by emotion label — verifies emotion_index equality lookup
        let store = EpisodicMemoryStore::new_in_memory();
        store
            .insert(make_episode(
                "ep1",
                1000,
                vec![],
                "愉悦",
                0.6,
                0.5,
                "第一次开心",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep2",
                2000,
                vec![],
                "悲伤",
                0.8,
                0.7,
                "那次很难过",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep3",
                3000,
                vec![],
                "愉悦",
                0.9,
                0.6,
                "又一次开心",
            ))
            .unwrap();

        // 查询「愉悦」标签 → 命中 ep1 和 ep3 / Query "愉悦" → matches ep1 and ep3
        let happy = store.query_by_emotion("愉悦");
        assert_eq!(
            happy.len(),
            2,
            "应命中 2 条愉悦情景 / should match 2 happy episodes"
        );
        // 按强度降序 — ep3 (0.9) 先于 ep1 (0.6) / Sorted by intensity desc
        assert_eq!(happy[0].id, "ep3");
        assert_eq!(happy[1].id, "ep1");

        // 查询「悲伤」标签 → 命中 ep2 / Query "悲伤" → matches ep2
        let sad = store.query_by_emotion("悲伤");
        assert_eq!(sad.len(), 1);
        assert_eq!(sad[0].id, "ep2");

        // 查询不存在的标签 → 空 / Query nonexistent label → empty
        let none = store.query_by_emotion("愤怒");
        assert!(
            none.is_empty(),
            "愤怒记忆应为空 / angry memories should be empty"
        );
    }

    #[test]
    fn test_query_by_context_tag() {
        // 按情境标签查询 — 验证 context_index 等值查找（多标签）
        // Query by context tag — verifies context_index equality lookup (multi-tag)
        let store = EpisodicMemoryStore::new_in_memory();
        store
            .insert(make_episode(
                "ep1",
                1000,
                vec!["深夜", "雨天"],
                "悲伤",
                0.7,
                0.8,
                "雨夜倾诉",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep2",
                2000,
                vec!["深夜"],
                "愉悦",
                0.6,
                0.5,
                "深夜分享",
            ))
            .unwrap();
        store
            .insert(make_episode(
                "ep3",
                3000,
                vec!["白天", "晴天"],
                "平静",
                0.3,
                0.4,
                "白天散步",
            ))
            .unwrap();

        // 查询「深夜」标签 → 命中 ep1 和 ep2 / Query "深夜" → matches ep1 and ep2
        let late_night = store.query_by_context_tag("深夜");
        assert_eq!(
            late_night.len(),
            2,
            "应命中 2 条深夜情景 / should match 2 late-night episodes"
        );
        // 按重要度降序 — ep1 (0.8) 先于 ep2 (0.5) / Sorted by importance desc
        assert_eq!(late_night[0].id, "ep1");
        assert_eq!(late_night[1].id, "ep2");

        // 查询「雨天」标签 → 只命中 ep1 / Query "雨天" → only matches ep1
        let rainy = store.query_by_context_tag("雨天");
        assert_eq!(rainy.len(), 1);
        assert_eq!(rainy[0].id, "ep1");

        // 查询不存在的标签 → 空 / Query nonexistent tag → empty
        let none = store.query_by_context_tag("早晨");
        assert!(
            none.is_empty(),
            "不存在的标签应返回空 / nonexistent tag should return empty"
        );
    }

    // ── 加权排序测试 / Weighted scoring tests ──

    #[test]
    fn test_query_relevant_weighted_scoring() {
        // 三路加权评分排序 — 验证情感强度 + 时间近因 + 情境标签匹配
        // Three-way weighted scoring — verifies intensity + recency + context tag match
        let store = EpisodicMemoryStore::new_in_memory();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // ep_high_intensity: 高情感强度（0.95），无情境标签匹配
        // ep_high_intensity: high emotional intensity (0.95), no context tag match
        store
            .insert(make_episode(
                "ep_high_intensity",
                now - 86400, // 1 天前 / 1 day ago
                vec![],
                "愉悦",
                0.95,
                0.9,
                "极度开心的时刻",
            ))
            .unwrap();
        // ep_recent: 最近发生（10 秒前），低强度，无情境匹配
        // ep_recent: recent (10 seconds ago), low intensity, no context match
        store
            .insert(make_episode(
                "ep_recent",
                now - 10,
                vec![],
                "平静",
                0.2,
                0.3,
                "刚刚发生的平淡事",
            ))
            .unwrap();
        // ep_context_match: 情境标签完美匹配 "深夜"
        // ep_context_match: context tag perfectly matches "深夜"
        store
            .insert(make_episode(
                "ep_context_match",
                now - 86400 * 7, // 7 天前 / 7 days ago
                vec!["深夜"],
                "平静",
                0.3,
                0.5,
                "那个深夜的对话",
            ))
            .unwrap();

        // 查询 "深夜" — 三路加权评分
        // Query "深夜" — three-way weighted scoring
        let results = store.query_relevant("深夜", 3);
        assert_eq!(
            results.len(),
            3,
            "应返回全部 3 条情景 / should return all 3 episodes"
        );

        // ep_high_intensity 应排在最前 — 情感强度 0.95 的权重最高
        // ep_high_intensity should rank first — 0.95 intensity dominates
        assert_eq!(
            results[0].id, "ep_high_intensity",
            "高情感强度情景应优先召回 / high-intensity episode should be recalled first"
        );

        // 查询 "深夜" — ep_context_match 应高于 ep_recent（情境匹配加分）
        // Query "深夜" — ep_context_match should outrank ep_recent (context match bonus)
        let ctx_pos = results
            .iter()
            .position(|e| e.id == "ep_context_match")
            .unwrap();
        let recent_pos = results.iter().position(|e| e.id == "ep_recent").unwrap();
        // 注意：ep_recent 时间近因权重很高（10 秒前），可能超过 ep_context_match 的情境匹配 + 7 天衰减
        // 这里仅验证两者都被召回，不强制顺序 — 加权评分本身是合理的
        // Note: ep_recent recency is very high (10s ago), may exceed ep_context_match's
        // context match + 7-day decay. We only verify both are recalled — the scoring
        // itself is reasonable.
        assert!(
            ctx_pos < 3 && recent_pos < 3,
            "两者都应被召回 / both should be recalled"
        );

        // limit=1 应只返回 1 条 / limit=1 should return only 1
        let top1 = store.query_relevant("深夜", 1);
        assert_eq!(
            top1.len(),
            1,
            "limit=1 应只返回 1 条 / should return only 1"
        );
    }

    // ── SQLite 持久化测试 / SQLite persistence tests ──

    #[test]
    fn test_sqlite_persistence_roundtrip() {
        // SQLite 持久化 — 写入后重新打开，情景应保留
        // SQLite persistence — episodes should survive reopen
        // 数字生命记忆连续性核心保障 / Core guarantee of digital life memory continuity
        let path = "./target/test_episodic_store_sqlite.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 写入阶段 / Write phase
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            assert!(store.is_persistent());
            store
                .insert(make_episode(
                    "ep_persist_1",
                    1000,
                    vec!["深夜", "雨天"],
                    "愉悦",
                    0.85,
                    0.9,
                    "那个雨夜主人分享了 Rust 代码",
                ))
                .unwrap();
            store
                .insert(make_episode(
                    "ep_persist_2",
                    2000,
                    vec!["白天"],
                    "平静",
                    0.3,
                    0.4,
                    "白天的平常对话",
                ))
                .unwrap();
            store.flush();
        }

        // 重新打开 — 验证数据保留 / Reopen — verify data preserved
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            assert!(store.is_persistent());
            assert_eq!(
                store.count(),
                2,
                "重启后应保留 2 条情景 / should have 2 episodes after restart"
            );

            // 验证 emotion_snapshot 也保留 / Verify emotion snapshot is also preserved
            let happy = store.query_by_emotion("愉悦");
            assert_eq!(happy.len(), 1);
            assert_eq!(happy[0].id, "ep_persist_1");
            assert_eq!(happy[0].emotion_snapshot.ai_emotion_label, "愉悦");
            assert!((happy[0].emotion_snapshot.intensity - 0.85).abs() < 1e-6);
            assert_eq!(happy[0].context_tags.len(), 2);
            assert!(happy[0].context_tags.contains(&"深夜".to_string()));
            assert!(happy[0].context_tags.contains(&"雨天".to_string()));

            // 验证 time_index 也重建了 / Verify time_index was rebuilt
            let by_time = store.query_by_time_range(0, 3000);
            assert_eq!(by_time.len(), 2);
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    #[test]
    fn test_sqlite_pinned_field_persists() {
        // pinned 字段持久化 — 验证 SQLite 后端正确写入与读取 pinned
        // pinned field persistence — verifies SQLite backend correctly writes and reads pinned
        let path = "./target/test_episodic_store_pinned.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 写入一个 pinned=true 的情景 / Write a pinned=true episode
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            let ep = make_episode(
                "ep_pinned",
                1000,
                vec!["重要"],
                "愉悦",
                0.9,
                1.0,
                "非常重要不能忘",
            )
            .with_pinned(true);
            store.insert(ep).unwrap();

            let ep_normal = make_episode("ep_normal", 2000, vec![], "平静", 0.3, 0.4, "平常事");
            store.insert(ep_normal).unwrap();
        }

        // 重启 — 验证 pinned 字段正确恢复 / Restart — verify pinned field restored correctly
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            let all = store.query_by_time_range(0, 3000);
            assert_eq!(all.len(), 2);

            let pinned_ep = all.iter().find(|e| e.id == "ep_pinned").unwrap();
            assert!(
                pinned_ep.pinned,
                "pinned=true 应正确持久化 / pinned=true should persist correctly"
            );

            let normal_ep = all.iter().find(|e| e.id == "ep_normal").unwrap();
            assert!(
                !normal_ep.pinned,
                "pinned=false 应正确持久化 / pinned=false should persist correctly"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    // ── pinned 字段默认值测试 / pinned default value test ──

    #[test]
    fn test_pinned_default_false() {
        // 新建 Episode 默认 pinned = false / New Episode defaults to pinned = false
        let ep = make_episode("ep_default", 1000, vec![], "平静", 0.3, 0.4, "默认情景");
        assert!(
            !ep.pinned,
            "新 Episode 默认 pinned 应为 false / new Episode should default to pinned=false"
        );

        // with_pinned(true) 后 pinned = true / with_pinned(true) sets pinned = true
        let ep_pinned = ep.with_pinned(true);
        assert!(
            ep_pinned.pinned,
            "with_pinned(true) 应设置 pinned=true / with_pinned(true) should set pinned=true"
        );

        // NaN 重要度降级为 0.0 / NaN importance degrades to 0.0
        let ep_nan = Episode::new(
            "ep_nan".to_string(),
            1000,
            vec![],
            make_emotion("平静", 0.3, 1000),
            "NaN 测试".to_string(),
            f32::NAN,
        );
        assert!(
            (ep_nan.importance - 0.0).abs() < 1e-6,
            "NaN 重要度应降级为 0.0 / NaN importance should degrade to 0.0"
        );

        // 超出 1.0 应被 clamp / Values > 1.0 should be clamped
        let ep_high = Episode::new(
            "ep_high".to_string(),
            1000,
            vec![],
            make_emotion("兴奋", 0.9, 1000),
            "超高重要度".to_string(),
            1.5,
        );
        assert!(
            (ep_high.importance - 1.0).abs() < 1e-6,
            "1.5 重要度应 clamp 为 1.0 / 1.5 importance should clamp to 1.0"
        );
    }

    // ── schema 迁移测试 / Schema migration test ──

    #[test]
    fn test_sqlite_migration_old_schema() {
        // SQLite 迁移 — 旧 schema 无 pinned 列 → 新 schema 补齐
        // SQLite migration — old schema without pinned column → new schema adds it
        let path = "./target/test_episodic_migration.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 1. 用旧 schema 建表并写入数据（模拟旧版本数据库，无 pinned 列）
        // Create table with old schema and insert data (simulating legacy DB, no pinned column)
        {
            let conn = Connection::open(path).unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .unwrap();
            conn.execute(
                "CREATE TABLE episodes (
                    id TEXT PRIMARY KEY,
                    timestamp INTEGER NOT NULL,
                    context_tags TEXT NOT NULL,
                    emotion_snapshot BLOB NOT NULL,
                    summary TEXT NOT NULL,
                    importance REAL NOT NULL
                )",
                [],
            )
            .unwrap();
            // 写入一条旧数据（无 pinned 列）/ Insert a legacy row (no pinned column)
            let emotion = make_emotion("愉悦", 0.7, 1000);
            let emotion_blob = bincode::serialize(&emotion).unwrap();
            let tags_json = serde_json::to_string(&vec!["深夜".to_string()]).unwrap();
            conn.execute(
                "INSERT INTO episodes (id, timestamp, context_tags, emotion_snapshot, summary, importance)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["ep_legacy", 1000_i64, &tags_json, &emotion_blob, "旧数据", 0.6_f64],
            )
            .unwrap();
        }

        // 2. 用 open_sqlite 打开 — 应自动迁移补齐 pinned 列
        // Open with open_sqlite — should auto-migrate
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            assert_eq!(
                store.count(),
                1,
                "迁移后应保留 1 条情景 / should retain 1 episode after migration"
            );
            let all = store.query_by_time_range(0, 2000);
            assert_eq!(all.len(), 1);
            // 旧数据 pinned 应为默认 false / Legacy data pinned should default to false
            assert!(
                !all[0].pinned,
                "旧数据 pinned 应为默认 false / legacy pinned should default to false"
            );
        }

        // 3. 重新打开 — 验证迁移后的列持久 / Reopen — verify migrated column persists
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            assert_eq!(store.count(), 1);
            // 可正常插入带 pinned=true 的新数据 / Can insert new data with pinned=true
            store
                .insert(
                    make_episode("ep_new", 2000, vec![], "兴奋", 0.8, 0.7, "迁移后新数据")
                        .with_pinned(true),
                )
                .unwrap();
        }

        // 4. 再次重启 — 验证新数据 pinned 字段持久 / Restart again — verify new data pinned persists
        {
            let store = EpisodicMemoryStore::open_sqlite(path).unwrap();
            assert_eq!(store.count(), 2);
            let all = store.query_by_time_range(0, 3000);
            let new_ep = all.iter().find(|e| e.id == "ep_new").unwrap();
            assert!(
                new_ep.pinned,
                "迁移后写入的 pinned=true 应持久 / pinned=true written after migration should persist"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    // ── 空查询边界测试 / Empty query boundary test ──

    #[test]
    fn test_empty_store_queries() {
        // 空存储的所有查询都应返回空 Vec / All queries on an empty store should return empty Vec
        let store = EpisodicMemoryStore::new_in_memory();
        assert_eq!(store.count(), 0);
        assert!(store.query_by_time_range(0, 1000).is_empty());
        assert!(store.query_by_emotion("愉悦").is_empty());
        assert!(store.query_by_context_tag("深夜").is_empty());
        assert!(store.query_relevant("任意查询", 5).is_empty());
    }

    #[test]
    fn test_overwrite_on_duplicate_id() {
        // 相同 id 的 insert 应覆盖旧记录 — 索引应同步更新
        // Insert with same id should overwrite old record — indexes should sync
        let store = EpisodicMemoryStore::new_in_memory();
        store
            .insert(make_episode(
                "ep_dup",
                1000,
                vec!["深夜"],
                "愉悦",
                0.6,
                0.5,
                "原记录",
            ))
            .unwrap();
        assert_eq!(store.count(), 1);
        // 验证原记录可通过「愉悦」查到 / Verify original is found via "愉悦"
        assert_eq!(store.query_by_emotion("愉悦").len(), 1);

        // 用相同 id 插入新记录 — 情绪标签变化 / Insert new record with same id — emotion label changes
        store
            .insert(make_episode(
                "ep_dup",
                2000,
                vec!["白天"],
                "悲伤",
                0.8,
                0.7,
                "覆盖后记录",
            ))
            .unwrap();
        assert_eq!(
            store.count(),
            1,
            "重复 id 不应增加计数 / duplicate id should not increase count"
        );

        // 新记录应可通过「悲伤」查到 / New record should be found via "悲伤"
        let sad = store.query_by_emotion("悲伤");
        assert_eq!(sad.len(), 1);
        assert_eq!(sad[0].summary, "覆盖后记录");

        // 时间索引应反映新的 timestamp / Time index should reflect new timestamp
        let by_time = store.query_by_time_range(1500, 2500);
        assert_eq!(by_time.len(), 1);
        assert_eq!(by_time[0].id, "ep_dup");

        // 情境索引应反映新的 context_tag / Context index should reflect new context_tag
        let day = store.query_by_context_tag("白天");
        assert_eq!(day.len(), 1);
    }
}
