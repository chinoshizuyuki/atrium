// SPDX-License-Identifier: MIT
//! CannedManager — ACK (Atrium Canned Knowledge) 罐装知识管理
//! CannedManager — ACK (Atrium Canned Knowledge) canned knowledge manager.
//!
//! .ack 文件格式: YAML front matter + Markdown 正文
//! .ack file format: YAML front matter + Markdown body.
//! 支持: 文件扫描 / hot-reload / 多策略触发 / 跨 AI 传输
//! Supports: file scanning / hot-reload / multi-strategy trigger / cross-AI transfer.
//!
//! 数据目录: ~/.atrium/canned/*.ack
//! Data directory: ~/.atrium/canned/*.ack

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── 类型枚举 ────────────────────────────────────────────────────

/// 罐装知识的类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CapsuleKind {
    /// 工具用法（对标 OpenClaw Skill）
    ToolUsage,
    /// 行为策略
    BehaviorStrategy,
    /// 技术知识
    #[default]
    TechnicalKnowledge,
    /// 对话策略
    ConversationStrategy,
    /// MCP / 外部协议配置
    ProtocolConfig,
    /// 自定义
    Custom(String),
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum CapsuleTrigger {
    /// 关键词匹配
    OnKeyword { keywords: Vec<String> },
    /// 意图匹配
    OnIntent { intent: String },
    /// 上下文匹配（渠道/设备）
    OnContext {
        channel: Option<String>,
        device: Option<String>,
    },
    /// 始终加载（作为基础知识）
    Always,
}

/// 知识来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapsuleSource {
    /// 用户教授
    UserTaught { original_text: String },
    /// AI 自学（回放管道）
    SelfLearned { evidence_count: u32 },
    /// 外部导入
    Imported { file_path: String, author: String },
    /// 系统内置
    Builtin,
}

impl Default for CapsuleSource {
    fn default() -> Self {
        CapsuleSource::Imported {
            file_path: String::new(),
            author: String::new(),
        }
    }
}

// ─── 核心结构体 ──────────────────────────────────────────────────

/// 旧版前导元数据（兼容 之前的 .ack 文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyFrontMatter {
    title: Option<String>,
    version: Option<String>,
    author: Option<String>,
    tags: Option<Vec<String>>,
    triggers: Option<Vec<String>>,
    category: Option<String>,
    created: Option<String>,
}

/// .ack 文件前导元数据（YAML front matter）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawFrontMatter {
    // 新版字段
    name: Option<String>,
    title: Option<String>,
    version: Option<String>,
    kind: Option<String>,
    tags: Option<Vec<String>>,
    summary: Option<String>,
    trigger: Option<serde_yaml::Value>,
    depends_on: Option<Vec<String>>,
    active: Option<bool>,
    // 旧版字段（兼容）
    author: Option<String>,
    triggers: Option<Vec<String>>,
    category: Option<String>,
    created: Option<String>,
    // 旧版解析出的 trigger（跳过 serde_yaml 反序列化）
    #[serde(skip)]
    legacy_trigger: Option<CapsuleTrigger>,
}

/// 罐装知识条目
#[derive(Debug, Clone)]
pub struct CannedKnowledge {
    /// 唯一标识（从文件路径派生）
    pub id: String,
    /// 机器名
    pub name: String,
    /// 人类可读标题
    pub title: String,
    /// 语义版本
    pub version: String,
    /// 知识类型
    pub kind: CapsuleKind,
    /// 标签
    pub tags: Vec<String>,
    /// 一句话摘要
    pub summary: String,
    /// Markdown 正文
    pub body: String,
    /// 触发条件
    pub trigger: Option<CapsuleTrigger>,
    /// 依赖的其他 capsule name
    pub depends_on: Vec<String>,
    /// 来源
    pub source: CapsuleSource,
    /// 文件路径
    pub path: PathBuf,
    /// 创建时间（Unix 时间戳）
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
    /// 访问次数
    pub access_count: u64,
    /// 是否激活
    pub active: bool,
}

impl CannedKnowledge {
    /// 解析 .ack 文件
    pub fn parse(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取失败 {}: {}", path.display(), e))?;

        let (fm_str, body) = Self::split_front_matter(&content);

        let raw: RawFrontMatter = if fm_str.is_empty() {
            // 无 front matter — 使用文件名作为标题
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            RawFrontMatter {
                name: Some(stem.clone()),
                title: Some(stem),
                version: Some("1.0.0".into()),
                kind: Some("TechnicalKnowledge".into()),
                tags: Some(vec![]),
                summary: None,
                trigger: None,
                depends_on: None,
                active: Some(true),
                author: None,
                triggers: None,
                category: None,
                created: None,
                legacy_trigger: None,
            }
        } else {
            // 先尝试新版格式，失败则回退到旧版
            match serde_yaml::from_str::<RawFrontMatter>(&fm_str) {
                Ok(fm) => fm,
                Err(_) => {
                    // 尝试旧版 LegacyFrontMatter
                    match serde_yaml::from_str::<LegacyFrontMatter>(&fm_str) {
                        Ok(legacy) => {
                            let legacy_trigger = legacy
                                .triggers
                                .filter(|v| !v.is_empty())
                                .map(|kw| CapsuleTrigger::OnKeyword { keywords: kw });
                            RawFrontMatter {
                                name: legacy.title.clone(),
                                title: legacy.title,
                                version: legacy.version,
                                kind: legacy.category.map(|c| Self::map_legacy_category(&c)),
                                tags: legacy.tags,
                                summary: None,
                                trigger: None,
                                depends_on: None,
                                active: Some(true),
                                author: legacy.author,
                                triggers: None,
                                category: None,
                                created: legacy.created,
                                legacy_trigger,
                            }
                        }
                        Err(e) => return Err(format!("YAML 解析失败: {}", e)),
                    }
                }
            }
        };

        let name = raw.name.unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        });
        let title = raw.title.unwrap_or_else(|| name.clone());
        let version = raw.version.unwrap_or_else(|| "1.0.0".into());
        let kind = raw
            .kind
            .map(|k| Self::parse_kind(&k))
            .unwrap_or(CapsuleKind::TechnicalKnowledge);
        let tags = raw.tags.unwrap_or_default();
        let summary = raw.summary.unwrap_or_default();
        let depends_on = raw.depends_on.unwrap_or_default();
        let active = raw.active.unwrap_or(true);

        // 解析 trigger（优先旧版直接构造的，其次新版 YAML 反序列化，最后旧版 triggers 字段）
        let trigger = raw
            .legacy_trigger
            .or_else(|| {
                raw.trigger
                    .and_then(|v| serde_yaml::from_value::<CapsuleTrigger>(v).ok())
            })
            .or_else(|| {
                raw.triggers
                    .filter(|v| !v.is_empty())
                    .map(|kw| CapsuleTrigger::OnKeyword { keywords: kw })
            });

        // 生成 ID（从路径派生，保持确定性）
        let id = Self::derive_id(&name, &version);

        // 文件元数据
        let metadata = std::fs::metadata(path).ok();
        let created_at = metadata
            .as_ref()
            .and_then(|m| m.created().ok())
            .map_or_else(
                || chrono::Utc::now().timestamp(),
                |t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0)
                },
            );
        let updated_at = metadata.and_then(|m| m.modified().ok()).map_or_else(
            || chrono::Utc::now().timestamp(),
            |t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            },
        );

        Ok(Self {
            id,
            name,
            title,
            version,
            kind,
            tags,
            summary,
            body,
            trigger,
            depends_on,
            source: CapsuleSource::Imported {
                file_path: path.to_string_lossy().into(),
                author: String::new(),
            },
            path: path.to_path_buf(),
            created_at,
            updated_at,
            access_count: 0,
            active,
        })
    }

    /// 分割 front matter 和正文
    fn split_front_matter(content: &str) -> (String, String) {
        if let Some(rest) = content.strip_prefix("---") {
            if let Some(end) = rest.find("---") {
                let fm = rest[..end].trim().to_string();
                let body = rest[end + 3..].trim().to_string();
                return (fm, body);
            }
        }
        (String::new(), content.to_string())
    }

    /// 从种类字符串解析 CapsuleKind
    fn parse_kind(s: &str) -> CapsuleKind {
        match s {
            "ToolUsage" => CapsuleKind::ToolUsage,
            "BehaviorStrategy" => CapsuleKind::BehaviorStrategy,
            "TechnicalKnowledge" => CapsuleKind::TechnicalKnowledge,
            "ConversationStrategy" => CapsuleKind::ConversationStrategy,
            "ProtocolConfig" => CapsuleKind::ProtocolConfig,
            other => CapsuleKind::Custom(other.to_string()),
        }
    }

    /// 旧版 category 映射到新版 kind
    fn map_legacy_category(category: &str) -> String {
        match category {
            "programming" | "prog" => "TechnicalKnowledge".into(),
            "behavior" => "BehaviorStrategy".into(),
            "tool" => "ToolUsage".into(),
            "protocol" => "ProtocolConfig".into(),
            other => format!("Custom(\"{}\")", other),
        }
    }

    /// 从 name + version 派生确定性 ID
    fn derive_id(name: &str, version: &str) -> String {
        format!("{}-v{}", name, version)
    }

    /// 检查是否匹配触发关键词
    pub fn matches_keyword(&self, message: &str) -> bool {
        if let Some(CapsuleTrigger::OnKeyword { keywords }) = &self.trigger {
            let lower = message.to_lowercase();
            keywords.iter().any(|k| lower.contains(&k.to_lowercase()))
        } else {
            false
        }
    }

    /// 检查是否匹配意图
    pub fn matches_intent(&self, intent: &str) -> bool {
        if let Some(CapsuleTrigger::OnIntent { intent: target }) = &self.trigger {
            target == intent
        } else {
            false
        }
    }

    /// 检查是否始终激活
    pub fn is_always_active(&self) -> bool {
        matches!(self.trigger, Some(CapsuleTrigger::Always))
    }

    /// 构建注入 Prompt 的片段
    pub fn to_injection(&self, max_chars: usize) -> String {
        format!(
            "## {}\n{}\n",
            self.title,
            if self.body.len() > max_chars {
                &self.body[..max_chars]
            } else {
                &self.body
            }
        )
    }
}

// ─── LRU 缓存 ────────────────────────────────────────────────────

/// 简单的 LRU 缓存实现
struct LruCache<K: Eq + std::hash::Hash + Clone, V: Clone> {
    entries: Vec<(K, V)>,
    capacity: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity,
        }
    }

    fn get(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            let entry = self.entries.remove(pos);
            let val = entry.1.clone();
            self.entries.push(entry);
            Some(val)
        } else {
            None
        }
    }

    fn insert(&mut self, key: K, value: V) {
        self.entries.retain(|(k, _)| k != &key);
        self.entries.push((key, value));
        while self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ─── CannedManager ────────────────────────────────────────────────

/// 罐装知识管理器
pub struct CannedManager {
    /// name → capsule
    index: HashMap<String, CannedKnowledge>,
    /// tag → [name]
    tag_index: HashMap<String, Vec<String>>,
    /// 最近使用的内容缓存
    lru_cache: LruCache<String, String>,
    /// 扫描目录
    scan_dir: PathBuf,
    /// 最后扫描时间
    last_scan: i64,
    /// sled 元数据数据库（可选）
    meta_db: Option<sled::Db>,
    /// 自学 ACK 总数（限流用）
    self_learned_count: u32,
    /// 近期创建时间戳（10 分钟窗口限流用）
    recent_creates: Vec<u64>,
}

impl CannedManager {
    pub fn new(scan_dir: &str) -> Self {
        Self {
            index: HashMap::new(),
            tag_index: HashMap::new(),
            lru_cache: LruCache::new(128),
            scan_dir: PathBuf::from(scan_dir),
            last_scan: 0,
            meta_db: None,
            self_learned_count: 0,
            recent_creates: Vec::new(),
        }
    }

    /// 带 sled 元数据存储的构造
    pub fn with_sled(scan_dir: &str, meta_db: sled::Db) -> Self {
        Self {
            index: HashMap::new(),
            tag_index: HashMap::new(),
            lru_cache: LruCache::new(128),
            scan_dir: PathBuf::from(scan_dir),
            last_scan: 0,
            meta_db: Some(meta_db),
            self_learned_count: 0,
            recent_creates: Vec::new(),
        }
    }

    /// 扫描目录加载所有 .ack 文件
    pub fn scan(&mut self) -> usize {
        let dir = &self.scan_dir;
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
            tracing::info!("创建 ACK 目录: {}", dir.display());
            return 0;
        }

        let mut loaded = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "ack") {
                    match CannedKnowledge::parse(&path) {
                        Ok(mut ack) => {
                            // 从 sled 恢复访问计数
                            if let Some(ref db) = self.meta_db {
                                if let Ok(Some(bytes)) = db.get(ack.id.as_bytes()) {
                                    ack.access_count = bytes.first().copied().unwrap_or(0) as u64;
                                }
                            }
                            let name = ack.name.clone();
                            self.rebuild_tag_index(&name, &ack.tags);
                            let existed = self.index.insert(name, ack).is_some();
                            if !existed {
                                tracing::info!("加载 ACK: {}", path.display());
                            }
                            loaded += 1;
                        }
                        Err(e) => {
                            tracing::warn!("解析 ACK 失败 {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        self.last_scan = chrono::Utc::now().timestamp();
        loaded
    }

    fn rebuild_tag_index(&mut self, name: &str, tags: &[String]) {
        for tag in tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .push(name.to_string());
        }
    }

    /// 找出当前对话应该激活的 capsules（多策略）
    pub fn resolve_active(
        &self,
        user_message: &str,
        user_intent: Option<&str>,
    ) -> Vec<&CannedKnowledge> {
        self.resolve_active_ctx(user_message, user_intent, None, None)
    }

    /// 带上下文信息的 resolve_active（支持 OnContext 触发）
    pub fn resolve_active_ctx(
        &self,
        user_message: &str,
        user_intent: Option<&str>,
        channel: Option<&str>,
        device: Option<&str>,
    ) -> Vec<&CannedKnowledge> {
        let mut active: Vec<&CannedKnowledge> = Vec::new();

        for capsule in self.index.values() {
            if !capsule.active {
                continue;
            }

            match &capsule.trigger {
                // 始终加载
                Some(CapsuleTrigger::Always) => {
                    active.push(capsule);
                }
                // 关键词匹配
                Some(CapsuleTrigger::OnKeyword { keywords }) => {
                    if !keywords.is_empty()
                        && keywords
                            .iter()
                            .any(|k| user_message.to_lowercase().contains(&k.to_lowercase()))
                    {
                        active.push(capsule);
                    }
                }
                // 意图匹配
                Some(CapsuleTrigger::OnIntent { intent }) => {
                    if let Some(ui) = user_intent {
                        if intent == ui {
                            active.push(capsule);
                        }
                    }
                }
                // 上下文匹配（: 实际实现）
                Some(CapsuleTrigger::OnContext {
                    channel: req_ch,
                    device: req_dev,
                }) => {
                    let ch_match = match (req_ch, channel) {
                        (None, _) => true,        // 不限制 channel
                        (Some(_), None) => false, // 需要但没传入
                        (Some(req), Some(act)) => req == act,
                    };
                    let dev_match = match (req_dev, device) {
                        (None, _) => true,
                        (Some(_), None) => false,
                        (Some(req), Some(act)) => req == act,
                    };
                    if ch_match && dev_match {
                        active.push(capsule);
                    }
                }
                None => {
                    // 无触发条件 — 只通过 search/resolve 显式调用
                }
            }
        }

        active
    }

    /// 按关键词和标签搜索（双向：字段包含query 或 query包含标签/关键词）
    pub fn search(&self, query: &str, _filter_tags: &[String]) -> Vec<&CannedKnowledge> {
        let lower = query.to_lowercase();
        self.index
            .values()
            .filter(|k| {
                k.active
                    && (k.title.to_lowercase().contains(&lower)
                        || k.summary.to_lowercase().contains(&lower)
                        || k.body.to_lowercase().contains(&lower)
                        || k.tags.iter().any(|t| {
                            let tl = t.to_lowercase();
                            tl.contains(&lower) || lower.contains(&tl)
                        })
                        || k.matches_keyword(query))
            })
            .collect()
    }

    /// 按触发关键词匹配（注入已有行为）
    pub fn trigger(&self, message: &str) -> Vec<&CannedKnowledge> {
        self.index
            .values()
            .filter(|k| k.active && k.matches_keyword(message))
            .collect()
    }

    /// 按分类获取
    pub fn by_category(&self, kind: &CapsuleKind) -> Vec<&CannedKnowledge> {
        self.index.values().filter(|k| &k.kind == kind).collect()
    }

    /// 获取所有知识标题
    pub fn titles(&self) -> Vec<&str> {
        self.index.keys().map(|s| s.as_str()).collect()
    }

    /// 按 name 获取
    pub fn get(&self, name: &str) -> Option<&CannedKnowledge> {
        self.index.get(name)
    }

    /// 获取总数
    pub fn count(&self) -> usize {
        self.index.len()
    }

    /// 按标签获取名称列表
    pub fn by_tag(&self, tag: &str) -> Vec<&String> {
        self.tag_index
            .get(tag)
            .map(|names| names.iter().collect())
            .unwrap_or_default()
    }

    /// 构建知识注入 Prompt
    pub fn inject_context(&self, message: &str, max_chars: usize) -> String {
        let triggered = self.resolve_active(message, None);
        if triggered.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("[罐装知识]\n");
        let mut total = 0;
        for ack in triggered {
            let snippet = ack.to_injection(max_chars.saturating_sub(total));
            if total + snippet.len() > max_chars {
                break;
            }
            total += snippet.len();
            ctx.push_str(&snippet);
        }
        ctx
    }

    /// 带 LRU 缓存的知识注入 Prompt（: 缓存激活结果）
    pub fn inject_context_cached(&mut self, message: &str, max_chars: usize) -> String {
        let cache_key = format!("{}:{}", message, max_chars);
        if let Some(cached) = self.lru_cache.get(&cache_key) {
            return cached;
        }
        let result = self.inject_context(message, max_chars);
        if !result.is_empty() {
            self.lru_cache.insert(cache_key, result.clone());
        }
        result
    }

    /// 热加载：重新扫描目录，加载新增/变更的 ACK 文件
    pub fn hot_reload(&mut self) -> usize {
        let before = self.index.len();
        let loaded = self.scan();
        let after = self.index.len();
        let new_count = after.saturating_sub(before);
        if new_count > 0 || loaded > 0 {
            tracing::info!(
                "CannedManager 热加载: 扫描 {} 个文件, 新增 {} 个 capsule, 总计 {}",
                loaded,
                new_count,
                after
            );
        }
        // 清空 LRU 缓存（内容可能已变更）
        self.lru_cache = LruCache::new(128);
        loaded
    }

    /// 获取 LRU 缓存大小
    pub fn cache_size(&self) -> usize {
        self.lru_cache.len()
    }

    /// 导出为跨 AI 传输文本
    pub fn export_to_text(&self, name: &str) -> Result<String, String> {
        let capsule = self
            .index
            .get(name)
            .ok_or_else(|| format!("未找到 capsule: {}", name))?;

        let yaml = serde_yaml::to_string(&ExportCapsule {
            name: capsule.name.clone(),
            title: capsule.title.clone(),
            kind: Self::kind_to_string(&capsule.kind),
            version: capsule.version.clone(),
            tags: capsule.tags.clone(),
            summary: capsule.summary.clone(),
            trigger: capsule.trigger.clone(),
            depends_on: capsule.depends_on.clone(),
            body: capsule.body.clone(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        Ok(format!(
            "=== Canned Knowledge v1 ===\n{}=== End Canned Knowledge ===",
            yaml
        ))
    }

    /// 从文本导入跨 AI 传输的 capsule
    pub fn import_from_text(&mut self, text: &str) -> Result<Vec<CannedKnowledge>, String> {
        let mut imported = Vec::new();

        // 查找 === Canned Knowledge v1 === ... === End Canned Knowledge === 块
        let start_marker = "=== Canned Knowledge v1 ===";
        let end_marker = "=== End Canned Knowledge ===";

        let mut start = 0;
        while let Some(block_start) = text[start..].find(start_marker) {
            let abs_start = start + block_start + start_marker.len();
            if let Some(block_end) = text[abs_start..].find(end_marker) {
                let yaml_str = text[abs_start..abs_start + block_end].trim();

                let export: ExportCapsule = serde_yaml::from_str(yaml_str)
                    .map_err(|e| format!("YAML 导入解析失败: {}", e))?;

                let capsule = self.ingest_export(export)?;
                self.save_to_disk(&capsule)?;
                imported.push(capsule);

                start = abs_start + block_end + end_marker.len();
            } else {
                break;
            }
        }

        Ok(imported)
    }

    fn ingest_export(&mut self, export: ExportCapsule) -> Result<CannedKnowledge, String> {
        let name = export.name.clone();
        let capsule = CannedKnowledge {
            id: CannedKnowledge::derive_id(&name, &export.version),
            name: name.clone(),
            title: export.title.clone(),
            version: export.version.clone(),
            kind: CannedKnowledge::parse_kind(&export.kind),
            tags: export.tags.clone(),
            summary: export.summary.clone(),
            body: export.body.clone(),
            trigger: export.trigger.clone(),
            depends_on: export.depends_on.clone(),
            source: CapsuleSource::Imported {
                file_path: String::new(),
                author: String::new(),
            },
            path: self.scan_dir.join(format!("{}.ack", name)),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            access_count: 0,
            active: true,
        };

        // 注册到索引
        self.rebuild_tag_index(&name, &capsule.tags);
        self.index.insert(name.clone(), capsule.clone());

        Ok(capsule)
    }

    pub fn save_to_disk(&self, capsule: &CannedKnowledge) -> Result<(), String> {
        if !self.scan_dir.exists() {
            std::fs::create_dir_all(&self.scan_dir).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        let yaml = serde_yaml::to_string(&ExportCapsule {
            name: capsule.name.clone(),
            title: capsule.title.clone(),
            kind: Self::kind_to_string(&capsule.kind),
            version: capsule.version.clone(),
            tags: capsule.tags.clone(),
            summary: capsule.summary.clone(),
            trigger: capsule.trigger.clone(),
            depends_on: capsule.depends_on.clone(),
            body: capsule.body.clone(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        let content = format!("---\n{}---\n\n{}", yaml, capsule.body);
        std::fs::write(&capsule.path, content).map_err(|e| format!("写入文件失败: {}", e))?;

        // 更新 sled 元数据
        if let Some(ref db) = self.meta_db {
            let _ = db.insert(capsule.id.as_bytes(), &[capsule.access_count as u8]);
        }

        Ok(())
    }

    /// 安全验证
    pub fn validate_safety(capsule: &CannedKnowledge) -> bool {
        // 不能覆盖系统内置
        if matches!(capsule.source, CapsuleSource::Builtin) {
            return false;
        }
        // 不能引用系统内部依赖
        if capsule.depends_on.iter().any(|d| d.starts_with("system.")) {
            return false;
        }
        // 限制 body 大小
        if capsule.body.len() > 100_000 {
            return false;
        }
        true
    }

    // ─── : ACK 自学习 ─────────────────────────────────

    /// 检查限流：总上限 50 + 10 分钟窗口上限 3
    fn check_rate_limit(&mut self, max_total: u32) -> Result<(), String> {
        if self.self_learned_count >= max_total {
            return Err(format!("ACK 自学上限已达 {} 个", max_total));
        }
        let now = Self::now_secs();
        // 清理 10 分钟前的记录
        self.recent_creates.retain(|t| now.saturating_sub(*t) < 600);
        if self.recent_creates.len() >= 3 {
            return Err("10 分钟内已创建 3 个 ACK，请稍后再试".into());
        }
        Ok(())
    }

    /// 检查内容安全：body ≤ 10000 字符，无代码块，无 URL
    fn validate_self_learn_content(body: &str) -> bool {
        if body.len() > 10_000 {
            return false;
        }
        // 禁止可执行代码块
        if body.contains("```bash") || body.contains("```sh") || body.contains("```powershell") {
            return false;
        }
        // 禁止 URL（防止钓鱼）
        if body.contains("http://") || body.contains("https://") {
            return false;
        }
        true
    }

    /// 检查重复：已有 ACK 的 title 前 20 字符相同
    fn is_duplicate(&self, prefix: &str) -> bool {
        let check: String = prefix.chars().take(20).collect();
        self.index
            .values()
            .any(|c| c.title.starts_with(&check) || c.body.starts_with(&check))
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn generate_name(prefix: &str) -> String {
        format!("{}_{}", prefix, Self::now_secs())
    }

    /// Path A: 用户教授 → 创建 ACK
    pub fn learn_from_user(
        &mut self,
        intent: &crate::teach_detector::TeachIntent,
        max_total: u32,
    ) -> Result<CannedKnowledge, String> {
        self.check_rate_limit(max_total)?;

        let text = &intent.knowledge_text;
        if !Self::validate_self_learn_content(text) {
            return Err("内容安全检查未通过".into());
        }
        if self.is_duplicate(text) {
            return Err("重复知识，跳过".into());
        }

        let name = Self::generate_name("ack_user");
        let now = chrono::Utc::now().timestamp();
        let path = self.scan_dir.join(format!("{}.ack", name));

        let kind = match intent.pattern_group {
            crate::teach_detector::TeachPatternGroup::RuleSetting => CapsuleKind::BehaviorStrategy,
            _ => CapsuleKind::TechnicalKnowledge,
        };

        // 提取关键词作为触发词（取前 3 个有意义的词）
        let keywords: Vec<String> = text
            .split(['，', '。', ',', ' '])
            .filter(|s| s.len() >= 2)
            .take(3)
            .map(|s| s.to_string())
            .collect();

        let capsule = CannedKnowledge {
            id: format!("{}-v1.0.0", name),
            name: name.clone(),
            title: format!("用户教授: {}", &text[..text.len().min(50)]),
            version: "1.0.0".into(),
            kind,
            tags: vec!["user-taught".into()],
            summary: text.clone(),
            body: text.clone(),
            trigger: if keywords.is_empty() {
                None
            } else {
                Some(CapsuleTrigger::OnKeyword { keywords })
            },
            depends_on: Vec::new(),
            source: CapsuleSource::UserTaught {
                original_text: text.clone(),
            },
            path,
            created_at: now,
            updated_at: now,
            access_count: 0,
            active: true,
        };

        self.save_to_disk(&capsule)?;
        self.rebuild_tag_index(&name, &capsule.tags);
        self.index.insert(name, capsule.clone());
        self.self_learned_count += 1;
        self.recent_creates.push(Self::now_secs());

        Ok(capsule)
    }

    /// Path B: 回放模式 → 创建 ACK
    pub fn learn_from_pattern(
        &mut self,
        pattern: &crate::replay::DiscoveredPattern,
        max_total: u32,
    ) -> Result<CannedKnowledge, String> {
        self.check_rate_limit(max_total)?;

        let body = format!(
 "## 回放发现模式\n\n- 类型: {:?}\n- 置信度: {:.0}%\n- 摘要: {}\n\n此知识由回放管道自动发现。",
 pattern.kind,
 pattern.confidence * 100.0,
 pattern.summary,
 );
        if !Self::validate_self_learn_content(&body) {
            return Err("内容安全检查未通过".into());
        }
        if self.is_duplicate(&pattern.summary) {
            return Err("重复知识，跳过".into());
        }

        let name = Self::generate_name("ack_replay");
        let now = chrono::Utc::now().timestamp();
        let path = self.scan_dir.join(format!("{}.ack", name));

        let kind = match pattern.kind {
            crate::replay::PatternKind::FrequentFact
            | crate::replay::PatternKind::EntityCluster => CapsuleKind::TechnicalKnowledge,
            crate::replay::PatternKind::ConfidenceTrend => CapsuleKind::BehaviorStrategy,
            crate::replay::PatternKind::TemporalCluster => CapsuleKind::ConversationStrategy,
        };

        let capsule = CannedKnowledge {
            id: format!("{}-v1.0.0", name),
            name: name.clone(),
            title: format!(
                "回放发现: {}",
                &pattern.summary[..pattern.summary.len().min(40)]
            ),
            version: "1.0.0".into(),
            kind,
            tags: vec!["self-learned".into(), "replay".into()],
            summary: pattern.summary.clone(),
            body,
            trigger: None,
            depends_on: Vec::new(),
            source: CapsuleSource::SelfLearned { evidence_count: 1 },
            path,
            created_at: now,
            updated_at: now,
            access_count: 0,
            active: true,
        };

        self.save_to_disk(&capsule)?;
        self.rebuild_tag_index(&name, &capsule.tags);
        self.index.insert(name, capsule.clone());
        self.self_learned_count += 1;
        self.recent_creates.push(Self::now_secs());

        Ok(capsule)
    }

    /// Path C: 反思洞察 → 创建 ACK
    pub fn learn_from_insight(
        &mut self,
        insight: &crate::reflection::Insight,
        max_total: u32,
    ) -> Result<CannedKnowledge, String> {
        self.check_rate_limit(max_total)?;

        let evidence_lines: String = insight
            .supporting_facts
            .iter()
            .map(|f| format!("- {}", f))
            .collect::<Vec<_>>()
            .join("\n");

        let body = format!(
            "## 洞察\n\n{}\n\n### 支撑证据\n\n{}\n\n置信度: {:.0}% | 此知识由反思引擎自动提炼。",
            insight.summary,
            evidence_lines,
            insight.confidence * 100.0,
        );
        if !Self::validate_self_learn_content(&body) {
            return Err("内容安全检查未通过".into());
        }
        if self.is_duplicate(&insight.summary) {
            return Err("重复知识，跳过".into());
        }

        let name = Self::generate_name("ack_insight");
        let now = chrono::Utc::now().timestamp();
        let path = self.scan_dir.join(format!("{}.ack", name));

        let capsule = CannedKnowledge {
            id: format!("{}-v1.0.0", name),
            name: name.clone(),
            title: format!(
                "洞察: {}",
                &insight.summary[..insight.summary.len().min(40)]
            ),
            version: "1.0.0".into(),
            kind: CapsuleKind::TechnicalKnowledge,
            tags: vec!["self-learned".into(), "insight".into()],
            summary: insight.summary.clone(),
            body,
            trigger: None,
            depends_on: Vec::new(),
            source: CapsuleSource::SelfLearned {
                evidence_count: insight.supporting_facts.len() as u32,
            },
            path,
            created_at: now,
            updated_at: now,
            access_count: 0,
            active: true,
        };

        self.save_to_disk(&capsule)?;
        self.rebuild_tag_index(&name, &capsule.tags);
        self.index.insert(name, capsule.clone());
        self.self_learned_count += 1;
        self.recent_creates.push(Self::now_secs());

        Ok(capsule)
    }

    /// ACK 自学习统计
    pub fn ack_learning_stats(&self) -> String {
        let user_taught = self
            .index
            .values()
            .filter(|c| matches!(c.source, CapsuleSource::UserTaught { .. }))
            .count();
        let self_learned = self
            .index
            .values()
            .filter(|c| matches!(c.source, CapsuleSource::SelfLearned { .. }))
            .count();
        format!(
            "ack_learning: total={}, user_taught={}, self_learned={}",
            self.index.len(),
            user_taught,
            self_learned,
        )
    }

    fn kind_to_string(kind: &CapsuleKind) -> String {
        match kind {
            CapsuleKind::ToolUsage => "ToolUsage".into(),
            CapsuleKind::BehaviorStrategy => "BehaviorStrategy".into(),
            CapsuleKind::TechnicalKnowledge => "TechnicalKnowledge".into(),
            CapsuleKind::ConversationStrategy => "ConversationStrategy".into(),
            CapsuleKind::ProtocolConfig => "ProtocolConfig".into(),
            CapsuleKind::Custom(s) => format!("Custom(\"{}\")", s),
        }
    }
}

/// 导出格式（内部使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportCapsule {
    name: String,
    title: String,
    kind: String,
    version: String,
    tags: Vec<String>,
    summary: String,
    trigger: Option<CapsuleTrigger>,
    depends_on: Vec<String>,
    body: String,
}

// ─── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn unique_test_dir(name: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        PathBuf::from(format!("./target/test_ack_{}_{}", name, id))
    }

    fn setup_test_dir(dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
    }

    fn cleanup_test_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn write_ack(dir: &Path, filename: &str, content: &str) {
        std::fs::write(dir.join(filename), content).unwrap();
    }

    // ─── 解析测试 ───

    #[test]
    fn test_parse_new_format() {
        let dir = unique_test_dir("newfmt");
        setup_test_dir(&dir);
        write_ack(
 &dir,
 "feishu.ack",
 "---\nname: feishu_connect\ntitle: 飞书连接配置\nversion: \"1.0.0\"\nkind: ToolUsage\ntags: [feishu, api]\nsummary: 飞书获取 token 流程\ntrigger:\n type: OnKeyword\n keywords: [飞书, feishu]\ndepends_on: []\n---\n\n# 飞书连接\n获取 token 的步骤。",
 );

        let ack = CannedKnowledge::parse(&dir.join("feishu.ack")).unwrap();
        assert_eq!(ack.name, "feishu_connect");
        assert_eq!(ack.title, "飞书连接配置");
        assert_eq!(ack.version, "1.0.0");
        assert_eq!(ack.kind, CapsuleKind::ToolUsage);
        assert_eq!(ack.tags, vec!["feishu", "api"]);
        assert_eq!(ack.summary, "飞书获取 token 流程");
        assert!(ack.matches_keyword("飞书怎么连"));
        assert!(!ack.matches_keyword("python怎么用"));
        assert!(ack.active);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_parse_minimal_format() {
        let dir = unique_test_dir("minimal");
        setup_test_dir(&dir);
        write_ack(
            &dir,
            "minimal.ack",
            "---\nname: test\ntitle: 测试\ntags: [test]\n---\n\n正文内容",
        );

        let ack = CannedKnowledge::parse(&dir.join("minimal.ack")).unwrap();
        assert_eq!(ack.name, "test");
        assert_eq!(ack.version, "1.0.0");
        assert_eq!(ack.kind, CapsuleKind::TechnicalKnowledge);
        assert!(ack.summary.is_empty());
        assert!(ack.trigger.is_none());
        assert!(ack.depends_on.is_empty());
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_parse_no_front_matter() {
        let dir = unique_test_dir("nofm");
        setup_test_dir(&dir);
        write_ack(&dir, "nofm.ack", "# 纯正文\n没有 front matter。");

        let ack = CannedKnowledge::parse(&dir.join("nofm.ack")).unwrap();
        assert_eq!(ack.name, "nofm");
        assert_eq!(ack.version, "1.0.0");
        assert_eq!(ack.kind, CapsuleKind::TechnicalKnowledge);
        assert!(ack.body.contains("纯正文"));
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_parse_legacy_format() {
        let dir = unique_test_dir("legacy");
        setup_test_dir(&dir);
        write_ack(
 &dir,
 "legacy.ack",
 "---\ntitle: Rust基础\nversion: \"1.0\"\ntags: [rust,编程]\ntriggers: [rust,所有权]\ncategory: programming\n---\n\n# Rust\nRust是系统编程语言。",
 );

        let ack = CannedKnowledge::parse(&dir.join("legacy.ack")).unwrap();
        assert_eq!(ack.title, "Rust基础");
        assert_eq!(ack.kind, CapsuleKind::TechnicalKnowledge);
        assert!(ack.matches_keyword("rust"));
        assert!(!ack.matches_keyword("python"));
        cleanup_test_dir(&dir);
    }

    // ─── 触发匹配测试 ───

    #[test]
    fn test_trigger_on_keyword() {
        let ack = make_capsule(
            "test",
            Some(CapsuleTrigger::OnKeyword {
                keywords: vec!["飞书".into(), "feishu".into()],
            }),
        );
        assert!(ack.matches_keyword("用飞书发送消息"));
        assert!(ack.matches_keyword("feishu api"));
        assert!(!ack.matches_keyword("用钉钉发送消息"));
    }

    #[test]
    fn test_trigger_on_intent() {
        let ack = make_capsule(
            "test",
            Some(CapsuleTrigger::OnIntent {
                intent: "code_review".into(),
            }),
        );
        assert!(ack.matches_intent("code_review"));
        assert!(!ack.matches_intent("write_code"));
        assert!(!ack.matches_keyword("code_review"));
    }

    #[test]
    fn test_trigger_always() {
        let ack = make_capsule("test", Some(CapsuleTrigger::Always));
        assert!(ack.is_always_active());
        assert!(!ack.matches_keyword("any"));
    }

    // ─── CannedManager 测试 ───

    #[test]
    fn test_manager_scan_and_count() {
        let dir = unique_test_dir("scan");
        cleanup_test_dir(&dir);
        setup_test_dir(&dir);
        write_ack(
            &dir,
            "a.ack",
            "---\nname: a\ntitle: A知识\nkind: TechnicalKnowledge\ntags: [rust]\n---\n\nA正文",
        );
        write_ack(
 &dir,
 "b.ack",
 "---\nname: b\ntitle: B策略\nkind: BehaviorStrategy\ntags: [coding]\ntrigger:\n type: OnKeyword\n keywords: [写代码]\n---\n\nB正文",
 );

        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        let loaded = mgr.scan();
        assert_eq!(loaded, 2);
        assert_eq!(mgr.count(), 2);
        assert_eq!(mgr.titles().len(), 2);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_resolve_active() {
        let dir = unique_test_dir("resolve");
        setup_test_dir(&dir);
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        mgr.index.insert(
            "always".into(),
            make_capsule("always", Some(CapsuleTrigger::Always)),
        );
        mgr.index.insert(
            "keyword".into(),
            make_capsule(
                "keyword",
                Some(CapsuleTrigger::OnKeyword {
                    keywords: vec!["飞书".into()],
                }),
            ),
        );
        mgr.index.insert("inactive".into(), {
            let mut c = make_capsule(
                "inactive",
                Some(CapsuleTrigger::OnKeyword {
                    keywords: vec!["test".into()],
                }),
            );
            c.active = false;
            c
        });

        let active = mgr.resolve_active("用飞书发消息", None);
        assert_eq!(active.len(), 2);
        assert!(active.iter().any(|c| c.name == "always"));
        assert!(active.iter().any(|c| c.name == "keyword"));

        let active2 = mgr.resolve_active("普通闲聊", None);
        assert_eq!(active2.len(), 1);

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_inject_context() {
        let dir = unique_test_dir("inject");
        setup_test_dir(&dir);
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        mgr.index.insert(
            "feishu".into(),
            CannedKnowledge {
                id: "feishu-v1".into(),
                name: "feishu".into(),
                title: "飞书配置".into(),
                version: "1.0.0".into(),
                kind: CapsuleKind::ToolUsage,
                tags: vec!["feishu".into()],
                summary: "飞书 API 配置".into(),
                body: "飞书连接步骤: 获取 token ...".into(),
                trigger: Some(CapsuleTrigger::OnKeyword {
                    keywords: vec!["飞书".into()],
                }),
                depends_on: vec![],
                source: CapsuleSource::Builtin,
                path: PathBuf::new(),
                created_at: 0,
                updated_at: 0,
                access_count: 0,
                active: true,
            },
        );

        let ctx = mgr.inject_context("怎么连接飞书", 500);
        assert!(ctx.contains("[罐装知识]"));
        assert!(ctx.contains("飞书配置"));

        let empty = mgr.inject_context("今天天气不错", 500);
        assert!(empty.is_empty());

        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_import_export() {
        let dir = unique_test_dir("ioe");
        setup_test_dir(&dir);
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        mgr.index.insert(
            "demo".into(),
            CannedKnowledge {
                id: "demo-v1".into(),
                name: "demo".into(),
                title: "示例知识".into(),
                version: "1.0.0".into(),
                kind: CapsuleKind::TechnicalKnowledge,
                tags: vec!["demo".into()],
                summary: "示例摘要".into(),
                body: "示例正文".into(),
                trigger: Some(CapsuleTrigger::OnKeyword {
                    keywords: vec!["demo".into()],
                }),
                depends_on: vec![],
                source: CapsuleSource::Builtin,
                path: dir.join("demo.ack"),
                created_at: 0,
                updated_at: 0,
                access_count: 0,
                active: true,
            },
        );

        let exported = mgr.export_to_text("demo").unwrap();
        assert!(exported.contains("=== Canned Knowledge v1 ==="));
        assert!(exported.contains("=== End Canned Knowledge ==="));
        assert!(exported.contains("name: demo"));

        let dir2 = unique_test_dir("ioe2");
        setup_test_dir(&dir2);
        let mut mgr2 = CannedManager::new(dir2.to_str().unwrap());
        let imported = mgr2.import_from_text(&exported).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "demo");

        cleanup_test_dir(&dir);
        cleanup_test_dir(&dir2);
    }

    #[test]
    fn test_validate_safety() {
        let safe = make_capsule("safe", None);
        assert!(CannedManager::validate_safety(&safe));

        let mut builtin = make_capsule("builtin", None);
        builtin.source = CapsuleSource::Builtin;
        assert!(!CannedManager::validate_safety(&builtin));

        let mut sys_dep = make_capsule("sys_dep", None);
        sys_dep.depends_on = vec!["system.internal".into()];
        assert!(!CannedManager::validate_safety(&sys_dep));

        let mut huge = make_capsule("huge", None);
        huge.body = "x".repeat(100_001);
        assert!(!CannedManager::validate_safety(&huge));
    }

    #[test]
    fn test_search_by_tag() {
        let dir = unique_test_dir("tag");
        setup_test_dir(&dir);
        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        mgr.index.insert(
            "rust_basics".into(),
            make_capsule_with_tags("rust_basics", vec!["rust".into(), "programming".into()]),
        );
        mgr.rebuild_tag_index("rust_basics", &["rust".into(), "programming".into()]);

        mgr.index.insert(
            "py_basics".into(),
            make_capsule_with_tags("py_basics", vec!["python".into(), "programming".into()]),
        );
        mgr.rebuild_tag_index("py_basics", &["python".into(), "programming".into()]);

        let results = mgr.search("rust", &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust_basics");

        cleanup_test_dir(&dir);
    }

    // ─── 辅助函数 ───

    fn make_capsule(name: &str, trigger: Option<CapsuleTrigger>) -> CannedKnowledge {
        CannedKnowledge {
            id: format!("{}-v1", name),
            name: name.to_string(),
            title: format!("{} 标题", name),
            version: "1.0.0".into(),
            kind: CapsuleKind::TechnicalKnowledge,
            tags: vec![],
            summary: String::new(),
            body: format!("{} 正文", name),
            trigger,
            depends_on: vec![],
            source: CapsuleSource::Imported {
                file_path: String::new(),
                author: String::new(),
            },
            path: PathBuf::new(),
            created_at: 0,
            updated_at: 0,
            access_count: 0,
            active: true,
        }
    }

    fn make_capsule_with_tags(name: &str, tags: Vec<String>) -> CannedKnowledge {
        let mut c = make_capsule(name, None);
        c.tags = tags;
        c
    }

    // ─── : OnContext / LRU / hot_reload ───

    #[test]
    fn test_resolve_active_on_context_channel_match() {
        let dir = unique_test_dir("ctx_ch");
        setup_test_dir(&dir);
        write_ack(
 &dir,
 "discord.ack",
 "---\nname: discord_help\ntitle: Discord帮助\ntrigger:\n type: OnContext\n channel: discord\n---\n\nDiscord相关内容",
 );
        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        mgr.scan();
        // channel 匹配
        let active = mgr.resolve_active_ctx("hello", None, Some("discord"), None);
        assert!(!active.is_empty(), "discord channel 应触发");
        // channel 不匹配
        let active = mgr.resolve_active_ctx("hello", None, Some("telegram"), None);
        assert!(active.is_empty(), "telegram channel 不应触发");
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_resolve_active_on_context_no_context_given() {
        let dir = unique_test_dir("ctx_none");
        setup_test_dir(&dir);
        write_ack(
            &dir,
            "any.ack",
            "---\nname: any_ctx\ntitle: 任意上下文\ntrigger:\n type: OnContext\n---\n\n任意内容",
        );
        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        mgr.scan();
        // 无限制的 OnContext（channel/device 都是 None）应始终匹配
        let active = mgr.resolve_active_ctx("hello", None, None, None);
        assert!(!active.is_empty());
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_inject_context_cached() {
        let dir = unique_test_dir("lru");
        setup_test_dir(&dir);
        write_ack(
            &dir,
            "always.ack",
            "---\nname: always_on\ntitle: 始终加载\ntrigger:\n type: Always\n---\n\n基础知识内容",
        );
        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        mgr.scan();
        // 第一次调用应缓存
        let result1 = mgr.inject_context_cached("any message", 500);
        assert!(!result1.is_empty());
        assert_eq!(mgr.cache_size(), 1);
        // 第二次调用应命中缓存
        let result2 = mgr.inject_context_cached("any message", 500);
        assert_eq!(result1, result2);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_hot_reload_detects_new_files() {
        let dir = unique_test_dir("reload");
        setup_test_dir(&dir);
        write_ack(
            &dir,
            "first.ack",
            "---\nname: first\ntitle: 第一个\n---\n\n内容",
        );
        let mut mgr = CannedManager::new(dir.to_str().unwrap());
        mgr.scan();
        assert_eq!(mgr.count(), 1);

        // 添加新文件
        write_ack(
            &dir,
            "second.ack",
            "---\nname: second\ntitle: 第二个\n---\n\n新内容",
        );
        let loaded = mgr.hot_reload();
        assert!(loaded >= 2, "应扫描到 2 个文件");
        assert_eq!(mgr.count(), 2);
        cleanup_test_dir(&dir);
    }

    // ─── : ACK 自学习测试 ──────────────────────────────

    #[test]
    fn test_learn_from_user_explicit() {
        let dir = unique_test_dir("test_learn_user");
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        let intent = crate::teach_detector::TeachIntent {
            confidence: 0.95,
            pattern_group: crate::teach_detector::TeachPatternGroup::ExplicitRemember,
            knowledge_text: "我喜欢画画和写代码".into(),
        };

        let result = mgr.learn_from_user(&intent, 50);
        assert!(result.is_ok(), "应成功创建 ACK: {:?}", result.err());
        let ack = result.unwrap();
        assert!(matches!(ack.source, CapsuleSource::UserTaught { .. }));
        assert!(ack.name.starts_with("ack_user_"));
        assert!(ack.path.exists(), ".ack 文件应已写入磁盘");
        assert_eq!(mgr.count(), 1);
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_learn_from_pattern_frequent() {
        let dir = unique_test_dir("test_learn_pattern");
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        let pattern = crate::replay::DiscoveredPattern {
            summary: "高频事实(出现5次): 主人|喜欢|Rust".into(),
            kind: crate::replay::PatternKind::FrequentFact,
            confidence: 0.85,
            discovered_at: 0,
        };

        let result = mgr.learn_from_pattern(&pattern, 50);
        assert!(result.is_ok(), "应成功创建 ACK: {:?}", result.err());
        let ack = result.unwrap();
        assert!(matches!(ack.source, CapsuleSource::SelfLearned { .. }));
        assert!(ack.tags.contains(&"replay".to_string()));
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_learn_from_insight_promoted() {
        let dir = unique_test_dir("test_learn_insight");
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        let insight = crate::reflection::Insight::new(
            "主人对Rust和AI领域有深入兴趣",
            vec!["喜欢Rust".into(), "研究AI".into(), "读论文".into()],
            0.85,
        );

        let result = mgr.learn_from_insight(&insight, 50);
        assert!(result.is_ok(), "应成功创建 ACK: {:?}", result.err());
        let ack = result.unwrap();
        assert!(matches!(
            ack.source,
            CapsuleSource::SelfLearned { evidence_count: 3 }
        ));
        assert!(ack.body.contains("支撑证据"));
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_rate_limit_total_cap() {
        let dir = unique_test_dir("test_rate_total");
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        // 用很小的上限测试
        for i in 0..3 {
            let intent = crate::teach_detector::TeachIntent {
                confidence: 0.9,
                pattern_group: crate::teach_detector::TeachPatternGroup::ExplicitRemember,
                knowledge_text: format!("知识内容 {}", i),
            };
            let result = mgr.learn_from_user(&intent, 3);
            assert!(result.is_ok(), "第 {} 个应成功", i);
        }

        // 第 4 个应失败（上限 3）
        let intent = crate::teach_detector::TeachIntent {
            confidence: 0.9,
            pattern_group: crate::teach_detector::TeachPatternGroup::ExplicitRemember,
            knowledge_text: "超出上限的知识".into(),
        };
        let result = mgr.learn_from_user(&intent, 3);
        assert!(result.is_err(), "应被限流拒绝");
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_rate_limit_10min_window() {
        let dir = unique_test_dir("test_rate_window");
        let mut mgr = CannedManager::new(dir.to_str().unwrap());

        // 手动注入近期创建记录模拟 10 分钟内已创建 3 个
        let now = CannedManager::now_secs();
        mgr.recent_creates = vec![now - 60, now - 120, now - 180];
        mgr.self_learned_count = 0; // 总数未达上限

        let intent = crate::teach_detector::TeachIntent {
            confidence: 0.9,
            pattern_group: crate::teach_detector::TeachPatternGroup::ExplicitRemember,
            knowledge_text: "窗口内第4个知识".into(),
        };
        let result = mgr.learn_from_user(&intent, 50);
        assert!(result.is_err(), "10分钟窗口内应被限流");
        cleanup_test_dir(&dir);
    }

    #[test]
    fn test_content_safety_rejects_code() {
        assert!(!CannedManager::validate_self_learn_content(
            "运行 ```bash rm -rf / ``` 这个命令"
        ));
        assert!(!CannedManager::validate_self_learn_content(
            "访问 https://evil.com 获取更多信息"
        ));
        assert!(CannedManager::validate_self_learn_content(
            "这是一段正常的知识内容"
        ));
    }
}
