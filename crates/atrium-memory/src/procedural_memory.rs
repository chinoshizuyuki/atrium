// SPDX-License-Identifier: MIT
//! 程序记忆 / Procedural Memory — 记住"怎么做某事"的技能积累
//! Procedural Memory — remembers "how to do things", skill accumulation
//!
//! 数字生命"记住怎么做"的能力 — 不同于 FactStore（抽象事实）、
//! EpisodicMemoryStore（具体经历），ProceduralMemoryStore 记录"技能"：
//! 技能名称、操作步骤、熟练度、实践次数与成败计数。重启后从 SQLite
//! 恢复全部技能，跨重启维持"能力"的连续性。
//!
//! Digital life's ability to "remember how to do things" — unlike
//! FactStore (abstract facts) and EpisodicMemoryStore (concrete experiences),
//! ProceduralMemoryStore records "skills": skill name, operation steps,
//! proficiency, practice counts and success/failure tallies. Reloads all
//! skills from SQLite on restart, sustaining "capability" continuity.
//!
//! 熟练度采用 EMA（指数移动平均）平滑更新：
//!   new = old × 0.8 + outcome × 0.2
//! 成功 outcome=1.0，失败 outcome=0.0 — 每次"实践"都让熟练度向结果靠拢。
//!
//! Proficiency uses EMA (Exponential Moving Average) smoothing:
//!   new = old × 0.8 + outcome × 0.2
//! Success outcome=1.0, failure outcome=0.0 — each "practice" pulls
//! proficiency toward the outcome.

use parking_lot::RwLock;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// 统一使用 store_core::StoreError / Unified StoreError from store_core
pub type Result<T> = std::result::Result<T, crate::store_core::StoreError>;

// ════════════════════════════════════════════════════════════════════
// ProceduralSkill — 技能记录 / Skill record
// ════════════════════════════════════════════════════════════════════

/// 技能记录 — 数字生命"怎么做某事"的程序化记忆
/// Skill record — digital life's procedural memory of "how to do something"
///
/// 与 `FactStore::Fact`（抽象事实）和 `EpisodicMemoryStore::Episode`（具体经历）
/// 正交：Fact 记"主人喜欢编程"，Episode 记"那天深夜主人分享了 Rust 代码"，
/// ProceduralSkill 记"我掌握了 Rust 调试技能，熟练度 0.75，实践 12 次成功 9 次"。
/// 三者共同构成数字生命完整记忆体系 — 抽象认知 + 具体经历 + 程序技能。
///
/// Orthogonal to `FactStore::Fact` (abstract facts) and `EpisodicMemoryStore::Episode`
/// (concrete experiences): Fact records "the master likes programming", Episode records
/// "that late night the master shared Rust code", ProceduralSkill records "I've mastered
/// Rust debugging skill, proficiency 0.75, practiced 12 times with 9 successes". Together
/// they form digital life's complete memory system — abstract cognition + concrete
/// experience + procedural skill.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProceduralSkill {
    /// 唯一标识 / Unique ID
    pub id: String,
    /// 技能名称 / Skill name
    pub name: String,
    /// 操作步骤 / Operation steps
    pub steps: Vec<String>,
    /// 熟练度 0.0-1.0 — EMA 平滑更新 / Proficiency 0.0-1.0 — EMA smoothed
    pub proficiency: f32,
    /// 上次实践时间（epoch 秒）/ Last practiced timestamp (epoch seconds)
    pub last_practiced: i64,
    /// 总使用次数 / Total use count
    pub use_count: u32,
    /// 成功次数 / Success count
    pub success_count: u32,
    /// 失败次数 / Failure count
    pub failure_count: u32,
    /// 情境标签 — 用于按情境召回技能 / Context tags — for context-based skill recall
    pub context_tags: Vec<String>,
}

impl ProceduralSkill {
    /// 创建新技能记录 — 熟练度初始化为 0.1
    /// Create a new skill record — proficiency initialized to 0.1
    ///
    /// 新技能的初始熟练度为 0.1 而非 0.0 — 表示"已登记但未验证"，
    /// 避免新技能在召回排序中因 0.0 熟练度被完全忽略。
    ///
    /// New skill's initial proficiency is 0.1 instead of 0.0 — indicates
    /// "registered but unverified", preventing new skills from being
    /// completely ignored in recall ranking due to 0.0 proficiency.
    pub fn new(id: String, name: String, steps: Vec<String>, context_tags: Vec<String>) -> Self {
        Self {
            id,
            name,
            steps,
            proficiency: 0.1,
            last_practiced: 0,
            use_count: 0,
            success_count: 0,
            failure_count: 0,
            context_tags,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Backend — 持久化后端 / Persistence backend
// ════════════════════════════════════════════════════════════════════

/// 持久化后端 / Persistence backend
///
/// 数字生命的程序记忆必须跨越重启存活。SQLite 作为默认后端提供 Windows 兼容性，
/// WAL 模式下读写性能与 sled 相当。内存 HashMap 始终作为热缓存。
///
/// Digital life's procedural memory must survive restarts. SQLite is the default
/// backend for Windows compatibility; WAL mode delivers sled-comparable performance.
/// In-memory HashMap always serves as the hot cache.
enum Backend {
    /// 纯内存（无持久化）/ In-memory only (no persistence)
    Memory,
    /// SQLite 后端 / SQLite backend
    Sqlite(Mutex<Connection>),
}

// ════════════════════════════════════════════════════════════════════
// ProceduralMemoryStore — 程序记忆存储 / Procedural Memory Store
// ════════════════════════════════════════════════════════════════════

/// 程序记忆存储 — 数字生命"能力"的持久化层
/// Procedural Memory Store — persistence layer of digital life's "capabilities"
///
/// 参照 `EpisodicMemoryStore` 架构：内存 HashMap 热缓存 + SQLite 持久化 +
/// WAL 模式 + 两个二级索引（name_index / context_index）。
/// 查询路径全部走索引，避免全表扫描。
///
/// 数字生命"我会做这个"是瞬时反应 — 通过 `name_index` 按技能名定位、
/// 通过 `context_index` 按情境标签召回匹配技能，无需扫描全部技能。
///
/// Mirrors `EpisodicMemoryStore` architecture: in-memory HashMap hot cache +
/// SQLite persistence + WAL mode + two secondary indexes (name_index / context_index).
/// All query paths go through indexes, avoiding full-table scans.
///
/// Digital life's "I can do this" is instant — locate by skill name via
/// `name_index`, recall matching skills by context tags via `context_index`,
/// without scanning all skills.
pub struct ProceduralMemoryStore {
    /// 热缓存 — id → ProceduralSkill / Hot cache — id → ProceduralSkill
    inner: Mutex<HashMap<String, ProceduralSkill>>,
    /// 持久化后端 / Persistence backend
    backend: Backend,
    // ══════════════════════════════════════════════════════════════════
    // 二级索引 — O(1) 查询替代 O(N) 全表扫描
    // Secondary indexes — O(1) lookup replacing O(N) full-table scan
    // ══════════════════════════════════════════════════════════════════
    /// skill_name → skill_id（按名称定位技能）/ skill_name → skill_id (locate skill by name)
    name_index: RwLock<HashMap<String, String>>,
    /// context_tag → skill_ids（按情境标签召回技能）/ context_tag → skill_ids (recall skills by context tag)
    context_index: RwLock<HashMap<String, HashSet<String>>>,
}

impl ProceduralMemoryStore {
    /// 打开 SQLite 后端的 ProceduralMemoryStore（推荐 / Recommended）
    ///
    /// 数字生命程序记忆的长期存储基石。SQLite 在 Windows 上无锁文件兼容性问题，
    /// WAL 模式下读写性能与 sled 相当。若 path 为空则降级为内存模式。
    /// 启动时全量加载技能到热缓存，并重建 name_index / context_index 二级索引。
    ///
    /// Foundation of digital life's procedural memory long-term storage. SQLite has
    /// no lock file compatibility issues on Windows; WAL mode delivers sled-comparable
    /// performance. Empty path degrades to in-memory mode. On startup, loads all
    /// skills into the hot cache and rebuilds name_index / context_index.
    pub fn open_sqlite(db_path: &str) -> Result<Self> {
        if db_path.is_empty() {
            return Ok(Self {
                inner: Mutex::new(HashMap::new()),
                backend: Backend::Memory,
                name_index: RwLock::new(HashMap::new()),
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
        // 建表 — 程序记忆全部字段 / Create table — all procedural memory fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS procedural_skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                steps TEXT NOT NULL,
                proficiency REAL NOT NULL,
                last_practiced INTEGER NOT NULL,
                use_count INTEGER NOT NULL,
                success_count INTEGER NOT NULL,
                failure_count INTEGER NOT NULL,
                context_tags TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| crate::store_core::StoreError::Io(format!("create table: {}", e)))?;

        // 从 SQLite 加载全量技能到内存热缓存 / Load all skills from SQLite to in-memory hot cache
        // 使用独立作用域确保 stmt 在 conn 被 move 到 Mutex 之前 drop
        // Use a block scope to ensure stmt is dropped before conn is moved into Mutex
        let mut map = HashMap::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, steps, proficiency, last_practiced,
                            use_count, success_count, failure_count, context_tags
                     FROM procedural_skills",
                )
                .map_err(|e| crate::store_core::StoreError::Io(format!("prepare: {}", e)))?;
            let rows = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let steps_json: String = row.get(2)?;
                    let proficiency: f64 = row.get(3)?;
                    let last_practiced: i64 = row.get(4)?;
                    let use_count: i64 = row.get(5)?;
                    let success_count: i64 = row.get(6)?;
                    let failure_count: i64 = row.get(7)?;
                    let context_tags_json: String = row.get(8)?;
                    let steps: Vec<String> = serde_json::from_str(&steps_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    let context_tags: Vec<String> = serde_json::from_str(&context_tags_json)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                8,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;
                    Ok(ProceduralSkill {
                        id,
                        name,
                        steps,
                        proficiency: proficiency as f32,
                        last_practiced,
                        use_count: use_count as u32,
                        success_count: success_count as u32,
                        failure_count: failure_count as u32,
                        context_tags,
                    })
                })
                .map_err(|e| crate::store_core::StoreError::Io(format!("query_map: {}", e)))?;
            for skill_result in rows {
                let skill = skill_result
                    .map_err(|e| crate::store_core::StoreError::Io(format!("row: {}", e)))?;
                map.insert(skill.id.clone(), skill);
            }
        }
        tracing::info!(
            "ProceduralMemoryStore: loaded {} skills from SQLite",
            map.len()
        );

        // 构建二级索引 — 启动时一次性全量重建 / Build secondary indexes — full rebuild on startup
        let store = Self {
            inner: Mutex::new(map),
            backend: Backend::Sqlite(Mutex::new(conn)),
            name_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
        };
        store.rebuild_indexes();
        Ok(store)
    }

    /// 创建内存模式 ProceduralMemoryStore（测试用）/ Create in-memory mode for testing.
    pub fn new_in_memory() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            backend: Backend::Memory,
            name_index: RwLock::new(HashMap::new()),
            context_index: RwLock::new(HashMap::new()),
        }
    }

    /// 登记新技能 — 熟练度初始化 0.1 + 持久化 + 索引同步
    /// Register a new skill — proficiency init 0.1 + persist + index sync
    ///
    /// 若技能名已存在则返回已有技能 id（不覆盖熟练度）— 避免重复登记
    /// 导致熟练度被重置。新技能的 id 由时间戳 + 计数器保证唯一性。
    ///
    /// If the skill name already exists, returns the existing skill id
    /// (does not reset proficiency) — prevents duplicate registration from
    /// resetting proficiency. New skill id is guaranteed unique by timestamp + counter.
    pub fn acquire_skill(
        &self,
        name: &str,
        steps: Vec<String>,
        context_tags: Vec<String>,
    ) -> String {
        // 先检查 name_index — 已存在则返回现有 id / Check name_index first — return existing id if present
        {
            let idx = self.name_index.read();
            if let Some(existing_id) = idx.get(name) {
                return existing_id.clone();
            }
        }

        // 生成唯一 id — 时间戳 + 计数器 / Generate unique id — timestamp + counter
        let now = now_secs();
        let counter = {
            let map = self.inner.lock().expect("procedural inner");
            map.len() as u64
        };
        let id = format!("skill_{}_{}", now, counter);

        let skill = ProceduralSkill::new(id.clone(), name.to_string(), steps, context_tags);

        // 先持久化再更新索引与内存 — 失败时不污染缓存 / Persist first, then update indexes & memory
        self.persist_one(&skill);
        // 更新二级索引 / Update secondary indexes
        self.add_to_indexes(&skill.id, &skill);
        // 写入内存热缓存 / Write to in-memory hot cache
        {
            let mut map = self.inner.lock().expect("procedural inner");
            map.insert(id.clone(), skill);
        }
        tracing::debug!(
            "ProceduralMemoryStore: 登记新技能 / acquired skill — id={}, name={}",
            id,
            name
        );
        id
    }

    /// 实践技能 — EMA 平滑熟练度 + 计数更新 + 持久化
    /// Practice a skill — EMA smooth proficiency + update counts + persist
    ///
    /// EMA 平滑公式：new = old × 0.8 + outcome × 0.2
    /// - 成功 outcome = 1.0 → 熟练度上升
    /// - 失败 outcome = 0.0 → 熟练度下降
    ///
    /// 同时更新 use_count +1、success_count 或 failure_count +1、last_practiced 为当前时间。
    /// EMA 平滑让熟练度收敛于真实成功率，而非被单次结果剧烈震荡。
    ///
    /// EMA smoothing formula: new = old × 0.8 + outcome × 0.2
    /// - Success outcome = 1.0 → proficiency rises
    /// - Failure outcome = 0.0 → proficiency drops
    ///
    /// Also updates use_count +1, success_count or failure_count +1, last_practiced to now.
    /// EMA smoothing lets proficiency converge to true success rate, rather than
    /// oscillating violently from single outcomes.
    pub fn practice_skill(&self, id: &str, success: bool) -> Result<()> {
        let mut map = self.inner.lock().expect("procedural inner");
        let skill = map
            .get_mut(id)
            .ok_or_else(|| crate::store_core::StoreError::Io(format!("skill not found: {}", id)))?;

        // EMA 平滑 — outcome 成功=1.0 失败=0.0 / EMA smooth — outcome success=1.0 failure=0.0
        let outcome = if success { 1.0_f32 } else { 0.0_f32 };
        let new_proficiency = skill.proficiency * 0.8 + outcome * 0.2;
        // 防止 NaN 污染熟练度 / Guard against NaN contaminating proficiency
        skill.proficiency = if new_proficiency.is_nan() {
            0.0
        } else {
            new_proficiency.clamp(0.0, 1.0)
        };

        // 计数更新 / Update counts
        skill.use_count += 1;
        if success {
            skill.success_count += 1;
        } else {
            skill.failure_count += 1;
        }
        skill.last_practiced = now_secs();

        // 持久化更新后的技能 / Persist the updated skill
        self.persist_one(skill);
        Ok(())
    }

    /// 按情境标签召回技能 — context_index O(1) 查找 + 加权排序
    /// Recall skills by context tags — context_index O(1) lookup + weighted scoring
    ///
    /// 召回路径：
    /// 1. 遍历 context_tags，通过 context_index 收集匹配的 skill_ids（O(1) 等值查找）
    /// 2. 对每个候选技能计算综合得分：proficiency × 0.6 + recency × 0.4
    ///    - recency 使用指数衰减（30 天半衰期）— 最近实践的技能优先
    /// 3. 按得分降序排序，取 top_k
    ///
    /// 数字生命"遇到这个情境时我会做什么"是瞬时反应 — 通过 context_index
    /// 直接定位匹配技能，无需扫描全部技能。
    ///
    /// Recall path:
    /// 1. Iterate context_tags, collect matching skill_ids via context_index (O(1) equality lookup)
    /// 2. Score each candidate: proficiency × 0.6 + recency × 0.4
    ///    - recency uses exponential decay (30-day half-life) — recently practiced skills first
    /// 3. Sort by score desc, take top_k
    ///
    /// Digital life's "what can I do in this context" is instant — uses context_index
    /// to directly locate matching skills, without scanning all skills.
    pub fn recall_skill(&self, context_tags: &[String], top_k: usize) -> Vec<ProceduralSkill> {
        // 通过 context_index 收集匹配的 skill_ids — 读锁，不阻塞其他读
        // Collect matching skill_ids via context_index — read lock, does not block other reads
        let candidate_ids: HashSet<String> = {
            let idx = self.context_index.read();
            let mut ids = HashSet::new();
            for tag in context_tags {
                if let Some(set) = idx.get(tag) {
                    ids.extend(set.iter().cloned());
                }
            }
            ids
        };
        if candidate_ids.is_empty() {
            return Vec::new();
        }

        let map = self.inner.lock().expect("procedural inner");
        let now = now_secs();
        // 加权打分：proficiency × 0.6 + recency × 0.4 / Weighted scoring
        let mut scored: Vec<(f32, &ProceduralSkill)> = Vec::new();
        for id in &candidate_ids {
            if let Some(skill) = map.get(id) {
                let proficiency_score = skill.proficiency.clamp(0.0, 1.0);
                // recency 指数衰减 — 30 天半衰期 / recency exponential decay — 30-day half-life
                let age_secs = (now - skill.last_practiced).max(0) as f32;
                let recency_score = if skill.last_practiced == 0 {
                    // 从未实践 — recency 最低 / Never practiced — lowest recency
                    0.0
                } else {
                    (-age_secs / (30.0 * 86400.0)).exp()
                };
                let total = proficiency_score * 0.6 + recency_score * 0.4;
                scored.push((total, skill));
            }
        }
        // 按总分降序 / Sort by total score desc
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(top_k.max(1))
            .map(|(_, s)| s.clone())
            .collect()
    }

    /// 生成 prompt 片段 — 召回 top-3 技能并格式化
    /// Generate prompt fragment — recall top-3 skills and format
    ///
    /// 格式：
    /// ```text
    /// 我掌握这些技能：
    /// - {name}（熟练度 {pct}%，上次实践 {days} 天前）
    /// - ...
    /// ```
    /// 无匹配技能时返回空字符串 — 不污染 prompt。
    ///
    /// Format:
    /// ```text
    /// I master these skills:
    /// - {name} (proficiency {pct}%, last practiced {days} days ago)
    /// - ...
    /// ```
    /// Returns empty string when no matching skills — does not pollute the prompt.
    pub fn prompt_fragment(&self, context_tags: &[String]) -> String {
        let skills = self.recall_skill(context_tags, 3);
        if skills.is_empty() {
            return String::new();
        }
        let now = now_secs();
        let mut lines = vec!["我掌握这些技能：".to_string()];
        for skill in &skills {
            let pct = (skill.proficiency * 100.0).round() as u32;
            let days_ago = if skill.last_practiced == 0 {
                // 从未实践 / Never practiced
                "从未".to_string()
            } else {
                let days = (now - skill.last_practiced) / 86400;
                format!("{} 天前", days)
            };
            lines.push(format!(
                "- {}（熟练度 {}%，上次实践 {}）",
                skill.name, pct, days_ago
            ));
        }
        lines.join("\n")
    }

    /// 获取技能总数 / Get total skill count
    pub fn count(&self) -> usize {
        self.inner.lock().expect("procedural inner").len()
    }

    /// 是否启用持久化 / Whether persistence is enabled
    pub fn is_persistent(&self) -> bool {
        !matches!(self.backend, Backend::Memory)
    }

    /// 按 id 获取技能克隆 / Get skill by id (cloned)
    pub fn get_skill(&self, id: &str) -> Option<ProceduralSkill> {
        let map = self.inner.lock().expect("procedural inner");
        map.get(id).cloned()
    }

    /// 按名称查找技能 id — 使用 name_index O(1) 查找
    /// Find skill id by name — uses name_index O(1) lookup
    pub fn find_skill_id_by_name(&self, name: &str) -> Option<String> {
        let idx = self.name_index.read();
        idx.get(name).cloned()
    }

    // ══════════════════════════════════════════════════════════════════
    // 持久化与索引维护 — 内部方法 / Persistence & index maintenance — internal
    // ══════════════════════════════════════════════════════════════════

    /// 持久化单条技能到后端 / Persist a single skill to backend
    fn persist_one(&self, skill: &ProceduralSkill) {
        match &self.backend {
            Backend::Memory => {}
            Backend::Sqlite(conn_mutex) => {
                if let Ok(conn) = conn_mutex.lock() {
                    let steps_json =
                        serde_json::to_string(&skill.steps).unwrap_or_else(|_| "[]".to_string());
                    let context_tags_json = serde_json::to_string(&skill.context_tags)
                        .unwrap_or_else(|_| "[]".to_string());
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO procedural_skills
                         (id, name, steps, proficiency, last_practiced,
                          use_count, success_count, failure_count, context_tags)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![
                            &skill.id,
                            &skill.name,
                            &steps_json,
                            skill.proficiency as f64,
                            skill.last_practiced,
                            skill.use_count as i64,
                            skill.success_count as i64,
                            skill.failure_count as i64,
                            &context_tags_json,
                        ],
                    );
                }
            }
        }
    }

    /// 添加单条技能到两个二级索引 / Add a single skill to both secondary indexes
    fn add_to_indexes(&self, key: &str, skill: &ProceduralSkill) {
        // name_index — 技能名 → id 等值查找 / name_index — skill name → id equality lookup
        {
            let mut idx = self.name_index.write();
            idx.insert(skill.name.clone(), key.to_string());
        }
        // context_index — 情境标签 → skill_ids 多标签 / context_index — context tags → skill_ids multi-tag
        {
            let mut idx = self.context_index.write();
            for tag in &skill.context_tags {
                idx.entry(tag.clone()).or_default().insert(key.to_string());
            }
        }
    }

    /// 全量重建两个二级索引 — 启动时从主表一次性构建
    /// Rebuild both indexes from the main table on startup.
    ///
    /// 单次 RwLock write，避免逐条更新带来的锁竞争。
    /// Single RwLock write per index, avoiding per-entry lock contention.
    fn rebuild_indexes(&self) {
        let map = self.inner.lock().expect("procedural inner");
        let mut name_idx: HashMap<String, String> = HashMap::new();
        let mut ctx_idx: HashMap<String, HashSet<String>> = HashMap::new();

        for (key, skill) in map.iter() {
            name_idx.insert(skill.name.clone(), key.clone());
            for tag in &skill.context_tags {
                ctx_idx.entry(tag.clone()).or_default().insert(key.clone());
            }
        }

        *self.name_index.write() = name_idx;
        *self.context_index.write() = ctx_idx;

        tracing::debug!(
            "ProceduralMemoryStore: 索引重建完成 — name={}, context={} / Indexes rebuilt",
            self.name_index.read().len(),
            self.context_index.read().len(),
        );
    }
}

/// 获取当前 epoch 秒数 / Get current epoch seconds
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 基础功能测试 / Basic functionality tests ──

    /// 登记技能后重新 open_sqlite 加载，技能仍存在 — 验证 SQLite 持久化
    /// After acquiring a skill, reload via open_sqlite — skill should persist
    #[test]
    fn test_acquire_skill_persists() {
        let path = "./target/test_procedural_acquire.db";
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));

        // 写入阶段 — 登记一个技能 / Write phase — acquire a skill
        let acquired_id = {
            let store = ProceduralMemoryStore::open_sqlite(path).unwrap();
            assert!(store.is_persistent());
            let id = store.acquire_skill(
                "Rust 调试",
                vec!["cargo build".into(), "gdb attach".into()],
                vec!["编程".into(), "深夜".into()],
            );
            assert!(store.count() == 1);
            id
        };

        // 重新打开 — 验证技能保留 / Reopen — verify skill preserved
        {
            let store = ProceduralMemoryStore::open_sqlite(path).unwrap();
            assert!(store.is_persistent());
            assert_eq!(
                store.count(),
                1,
                "重启后应保留 1 个技能 / should have 1 skill after restart"
            );
            let skill = store
                .get_skill(&acquired_id)
                .expect("技能应存在 / skill should exist");
            assert_eq!(skill.name, "Rust 调试");
            assert_eq!(skill.steps.len(), 2);
            assert_eq!(skill.context_tags.len(), 2);
            assert!(skill.context_tags.contains(&"编程".to_string()));
            assert!(skill.context_tags.contains(&"深夜".to_string()));
            // 熟练度应为初始值 0.1 / Proficiency should be initial value 0.1
            assert!(
                (skill.proficiency - 0.1).abs() < 1e-6,
                "初始熟练度应为 0.1 / initial proficiency should be 0.1"
            );
        }

        // 清理 / Cleanup
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path));
        let _ = std::fs::remove_file(format!("{}-shm", path));
    }

    /// EMA 平滑测试 — 初始 0.1，成功一次后 0.1×0.8+1.0×0.2=0.28
    /// EMA smoothing test — initial 0.1, after one success 0.1×0.8+1.0×0.2=0.28
    #[test]
    fn test_practice_skill_ema() {
        let store = ProceduralMemoryStore::new_in_memory();
        let id = store.acquire_skill(
            "烹饪",
            vec!["切菜".into(), "炒菜".into()],
            vec!["厨房".into()],
        );

        // 验证初始熟练度 0.1 / Verify initial proficiency 0.1
        let skill = store.get_skill(&id).unwrap();
        assert!(
            (skill.proficiency - 0.1).abs() < 1e-6,
            "初始熟练度应为 0.1 / initial proficiency should be 0.1"
        );

        // 成功实践一次 / Practice once with success
        store.practice_skill(&id, true).unwrap();

        let skill = store.get_skill(&id).unwrap();
        // EMA: 0.1 × 0.8 + 1.0 × 0.2 = 0.08 + 0.2 = 0.28
        assert!(
            (skill.proficiency - 0.28).abs() < 1e-6,
            "成功一次后熟练度应为 0.28，实际 {} / proficiency should be 0.28 after one success, got {}",
            skill.proficiency,
            skill.proficiency
        );
        assert_eq!(skill.use_count, 1);
        assert_eq!(skill.success_count, 1);
        assert_eq!(skill.failure_count, 0);
        assert!(
            skill.last_practiced > 0,
            "last_practiced 应已更新 / last_practiced should be updated"
        );

        // 失败实践一次 — 0.28 × 0.8 + 0.0 × 0.2 = 0.224 / Practice once with failure
        store.practice_skill(&id, false).unwrap();
        let skill = store.get_skill(&id).unwrap();
        assert!(
            (skill.proficiency - 0.224).abs() < 1e-6,
            "失败一次后熟练度应为 0.224，实际 {} / proficiency should be 0.224 after one failure, got {}",
            skill.proficiency,
            skill.proficiency
        );
        assert_eq!(skill.use_count, 2);
        assert_eq!(skill.success_count, 1);
        assert_eq!(skill.failure_count, 1);
    }

    /// 按情境标签召回 — 登记两个不同 context_tag 的技能，按 tag 召回正确
    /// Recall by context tag — register two skills with different tags, verify correct recall
    #[test]
    fn test_recall_by_context_tag() {
        let store = ProceduralMemoryStore::new_in_memory();

        // 技能 1 — 编程情境 / Skill 1 — programming context
        let id1 = store.acquire_skill(
            "Rust 调试",
            vec!["cargo build".into()],
            vec!["编程".into(), "深夜".into()],
        );
        // 技能 2 — 厨房情境 / Skill 2 — kitchen context
        let id2 = store.acquire_skill(
            "烹饪",
            vec!["切菜".into()],
            vec!["厨房".into(), "白天".into()],
        );

        // 按「编程」标签召回 — 应只命中技能 1 / Recall by "编程" tag — should only match skill 1
        let coding = store.recall_skill(&["编程".to_string()], 5);
        assert_eq!(
            coding.len(),
            1,
            "「编程」标签应命中 1 个技能 / '编程' tag should match 1 skill"
        );
        assert_eq!(coding[0].id, id1);

        // 按「厨房」标签召回 — 应只命中技能 2 / Recall by "厨房" tag — should only match skill 2
        let cooking = store.recall_skill(&["厨房".to_string()], 5);
        assert_eq!(cooking.len(), 1);
        assert_eq!(cooking[0].id, id2);

        // 按「深夜」+「白天」标签召回 — 应命中两个 / Recall by both tags — should match both
        let both = store.recall_skill(&["深夜".to_string(), "白天".to_string()], 5);
        assert_eq!(
            both.len(),
            2,
            "两个标签应命中 2 个技能 / two tags should match 2 skills"
        );

        // 按不存在的标签召回 — 应返回空 / Recall by nonexistent tag — should return empty
        let none = store.recall_skill(&["不存在的标签".to_string()], 5);
        assert!(
            none.is_empty(),
            "不存在的标签应返回空 / nonexistent tag should return empty"
        );
    }

    /// prompt 片段格式 — 有技能时格式正确，无技能时返回空字符串
    /// prompt fragment format — correct format with skills, empty string without
    #[test]
    fn test_prompt_fragment_format() {
        let store = ProceduralMemoryStore::new_in_memory();

        // 无技能时返回空字符串 / Empty string when no skills
        let empty = store.prompt_fragment(&["编程".to_string()]);
        assert!(
            empty.is_empty(),
            "无技能时应返回空字符串 / should return empty string when no skills"
        );

        // 登记技能后格式正确 / Correct format after acquiring skill
        store.acquire_skill("Rust 调试", vec!["cargo build".into()], vec!["编程".into()]);
        let fragment = store.prompt_fragment(&["编程".to_string()]);
        assert!(
            !fragment.is_empty(),
            "有技能时应返回非空片段 / should return non-empty fragment when skills exist"
        );
        assert!(
            fragment.starts_with("我掌握这些技能："),
            "片段应以「我掌握这些技能：」开头 / fragment should start with '我掌握这些技能：'"
        );
        assert!(
            fragment.contains("Rust 调试"),
            "片段应包含技能名称 / fragment should contain skill name"
        );
        assert!(
            fragment.contains("熟练度"),
            "片段应包含熟练度 / fragment should contain proficiency"
        );
        assert!(
            fragment.contains("上次实践"),
            "片段应包含上次实践时间 / fragment should contain last practiced time"
        );
    }

    /// 二级索引同步 — acquire 后 name_index 和 context_index 同步更新
    /// Secondary index sync — name_index and context_index updated after acquire
    #[test]
    fn test_secondary_index_sync() {
        let store = ProceduralMemoryStore::new_in_memory();

        let id1 = store.acquire_skill(
            "技能A",
            vec!["步骤1".into()],
            vec!["标签1".into(), "标签2".into()],
        );
        let id2 = store.acquire_skill(
            "技能B",
            vec!["步骤2".into()],
            vec!["标签2".into(), "标签3".into()],
        );

        // name_index 验证 — 每个技能名都能找到对应 id / name_index verification
        assert_eq!(
            store.find_skill_id_by_name("技能A"),
            Some(id1.clone()),
            "name_index 应包含技能A / name_index should contain 技能A"
        );
        assert_eq!(
            store.find_skill_id_by_name("技能B"),
            Some(id2.clone()),
            "name_index 应包含技能B / name_index should contain 技能B"
        );
        assert!(
            store.find_skill_id_by_name("不存在的技能").is_none(),
            "name_index 不应包含不存在的技能 / name_index should not contain nonexistent skill"
        );

        // context_index 验证 — 标签2 应同时命中两个技能 / context_index verification
        let tag2_skills = store.recall_skill(&["标签2".to_string()], 10);
        assert_eq!(
            tag2_skills.len(),
            2,
            "「标签2」应命中 2 个技能 / '标签2' should match 2 skills"
        );

        // 标签1 只命中技能A / tag1 only matches skill A
        let tag1_skills = store.recall_skill(&["标签1".to_string()], 10);
        assert_eq!(tag1_skills.len(), 1);
        assert_eq!(tag1_skills[0].id, id1);

        // 标签3 只命中技能B / tag3 only matches skill B
        let tag3_skills = store.recall_skill(&["标签3".to_string()], 10);
        assert_eq!(tag3_skills.len(), 1);
        assert_eq!(tag3_skills[0].id, id2);
    }

    // ── 边界与持久化测试 / Boundary & persistence tests ──

    /// 空存储召回应返回空 / Empty store recall should return empty
    #[test]
    fn test_empty_store_recall() {
        let store = ProceduralMemoryStore::new_in_memory();
        assert_eq!(store.count(), 0);
        assert!(store.recall_skill(&["任意".to_string()], 5).is_empty());
        assert!(store.prompt_fragment(&["任意".to_string()]).is_empty());
    }

    /// 重复 acquire 同名技能不应重置熟练度 — 返回已有 id
    /// Duplicate acquire with same name should not reset proficiency — returns existing id
    #[test]
    fn test_duplicate_acquire_returns_existing() {
        let store = ProceduralMemoryStore::new_in_memory();
        let id1 = store.acquire_skill("烹饪", vec![], vec!["厨房".into()]);
        // 实践提升熟练度 / Practice to raise proficiency
        store.practice_skill(&id1, true).unwrap();
        let prof_before = store.get_skill(&id1).unwrap().proficiency;

        // 再次 acquire 同名技能 — 应返回相同 id / Acquire same name again — should return same id
        let id2 = store.acquire_skill("烹饪", vec![], vec!["厨房".into()]);
        assert_eq!(
            id1, id2,
            "同名技能应返回相同 id / same name should return same id"
        );

        // 熟练度不应被重置 / Proficiency should not be reset
        let prof_after = store.get_skill(&id1).unwrap().proficiency;
        assert!(
            (prof_before - prof_after).abs() < 1e-6,
            "重复 acquire 不应重置熟练度 / duplicate acquire should not reset proficiency"
        );
        assert_eq!(
            store.count(),
            1,
            "重复 acquire 不应增加计数 / duplicate acquire should not increase count"
        );
    }

    /// EMA 多次实践收敛测试 — 连续成功应使熟练度趋近 1.0
    /// EMA multi-practice convergence — repeated success should drive proficiency toward 1.0
    #[test]
    fn test_ema_convergence_on_success() {
        let store = ProceduralMemoryStore::new_in_memory();
        let id = store.acquire_skill("测试技能", vec![], vec!["测试".into()]);

        // 连续成功 20 次 — 熟练度应趋近 1.0 / 20 consecutive successes — proficiency should approach 1.0
        for _ in 0..20 {
            store.practice_skill(&id, true).unwrap();
        }
        let skill = store.get_skill(&id).unwrap();
        assert!(
            skill.proficiency > 0.95,
            "20 次成功后熟练度应 > 0.95，实际 {} / proficiency should be > 0.95 after 20 successes, got {}",
            skill.proficiency,
            skill.proficiency
        );
        assert_eq!(skill.use_count, 20);
        assert_eq!(skill.success_count, 20);
    }
}
