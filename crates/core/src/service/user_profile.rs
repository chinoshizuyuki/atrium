//! 用户画像管理器 / User Profile Manager
//!
//! 聚合各子系统数据为人类可读的 Markdown 文件，定期写盘 + 启动加载。
//! 数字生命理念：用户画像是"我"对"主人"的认知地图，是关系的记忆载体。
//! Digital life philosophy: The user profile is "my" cognitive map of "master",
//! the memory carrier of our relationship.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 用户画像快照 / User profile snapshot
///
/// 从各子系统聚合的只读视图，用于序列化为 Markdown。
/// Read-only view aggregated from subsystems, serialized to Markdown.
pub struct UserProfileSnapshot<'a> {
    /// 用户称呼 / User designation (e.g., "主人")
    pub master_name: &'a str,
    /// AI 名字 / AI name (e.g., "Atrium")
    pub ai_name: &'a str,
    /// 用户事实列表 / User facts list
    pub facts: &'a [atrium_memory::fact_store::Fact],
    /// 偏好上下文文本 / Preference context text
    pub preference_text: String,
    /// 当前情绪标签 / Current emotion label
    pub emotion_label: &'a str,
    /// PAD 状态 / PAD state (pleasure, arousal, dominance)
    pub pad: (f32, f32, f32),
    /// 关系阶段名 / Relationship stage name
    pub relationship_stage: &'a str,
    /// 总交互次数 / Total interactions
    pub total_interactions: u64,
    /// 共振次数 / Resonance count
    pub resonance_count: u32,
    /// 回访次数 / Return count
    pub return_count: u32,
    /// 用户心智模型摘要 / User mental model summary
    pub mental_model_summary: String,
    /// 参与度分数 / Engagement score
    pub engagement_score: f32,
}

/// 用户画像管理器 / User Profile Manager
///
/// 定期将各子系统数据聚合为 Markdown 文件写入磁盘。
/// 启动时加载已有画像文件，注入 system prompt。
/// Periodically aggregates subsystem data into a Markdown file on disk.
/// Loads existing profile on startup, injects into system prompt.
pub struct UserProfileManager {
    /// 画像文件路径 / Profile file path
    file_path: PathBuf,
    /// 防抖计数器：累积 N 条交互后写盘 / Debounce counter: write after N interactions
    unsaved_count: u32,
    /// 写盘阈值 / Write threshold
    write_threshold: u32,
    /// 缓存的画像文本（用于 prompt 注入）/ Cached profile text (for prompt injection)
    cached_text: String,
}

impl UserProfileManager {
    /// 创建用户画像管理器 / Create a user profile manager
    ///
    /// `data_dir` — 数据目录，画像文件将存为 `{data_dir}/user_profile.md`
    /// `data_dir` — Data directory, profile saved as `{data_dir}/user_profile.md`
    pub fn new(data_dir: &str) -> Self {
        let file_path = Path::new(data_dir).join("user_profile.md");
        let cached_text = Self::load_from_disk(&file_path);

        Self {
            file_path,
            unsaved_count: 0,
            write_threshold: 20, // 每 20 条交互写盘 / Write every 20 interactions
            cached_text,
        }
    }

    /// 从磁盘加载画像文本 / Load profile text from disk
    fn load_from_disk(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_default() // 首次启动无文件返回空 / Empty on first launch
    }

    /// 获取缓存的画像文本（用于 prompt 注入）/ Get cached profile text for prompt injection
    /// 配置热重载预留接口 / Reserved for config hot-reload
    #[allow(dead_code)]
    pub fn cached_text(&self) -> &str {
        &self.cached_text
    }

    /// 生成画像 Markdown / Generate profile Markdown
    pub fn generate_markdown(snapshot: &UserProfileSnapshot) -> String {
        let mut md = String::with_capacity(2048);

        // ── 头部 / Header ──
        md.push_str(&format!(
            "# 用户画像 / User Profile\n\n\
            > 由 {} 对 {} 的认知地图\n\
            > Cognitive map of {} as perceived by {}\n\
            > 更新时间 / Updated: {}\n\n",
            snapshot.ai_name,
            snapshot.master_name,
            snapshot.master_name,
            snapshot.ai_name,
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
        ));

        // ── 关系概览 / Relationship Overview ──
        md.push_str("## 关系概览 / Relationship Overview\n\n");
        md.push_str(&format!(
            "- **关系阶段 / Stage**: {}\n\
             - **总交互次数 / Total interactions**: {}\n\
             - **共振次数 / Resonance count**: {}\n\
             - **回访次数 / Return count**: {}\n\n",
            snapshot.relationship_stage,
            snapshot.total_interactions,
            snapshot.resonance_count,
            snapshot.return_count,
        ));

        // ── 情绪状态 / Emotional State ──
        md.push_str("## 情绪状态 / Emotional State\n\n");
        md.push_str(&format!(
            "- **当前情绪 / Current emotion**: {}\n\
             - **PAD**: P={:.3}, A={:.3}, D={:.3}\n\n",
            snapshot.emotion_label, snapshot.pad.0, snapshot.pad.1, snapshot.pad.2,
        ));

        // ── 用户事实 / User Facts ──
        md.push_str("## 用户事实 / User Facts\n\n");
        if snapshot.facts.is_empty() {
            md.push_str("_暂无已知事实 / No known facts yet_\n\n");
        } else {
            for fact in snapshot.facts {
                md.push_str(&format!(
                    "- {} {} {} (置信度 / conf: {:.0}%)\n",
                    fact.subject,
                    fact.predicate,
                    fact.object,
                    fact.confidence * 100.0,
                ));
            }
            md.push('\n');
        }

        // ── 偏好 / Preferences ──
        md.push_str("## 偏好 / Preferences\n\n");
        if snapshot.preference_text.is_empty() {
            md.push_str("_暂无已知偏好 / No known preferences yet_\n\n");
        } else {
            md.push_str(&format!("{}\n\n", snapshot.preference_text));
        }

        // ── 心智模型 / Mental Model ──
        md.push_str("## 心智模型 / Mental Model\n\n");
        if snapshot.mental_model_summary.is_empty() {
            md.push_str("_心智模型尚在构建 / Mental model under construction_\n\n");
        } else {
            md.push_str(&format!("{}\n\n", snapshot.mental_model_summary));
        }

        // ── 参与度 / Engagement ──
        md.push_str("## 参与度 / Engagement\n\n");
        let eng_desc = if snapshot.engagement_score > 0.7 {
            "高 / High"
        } else if snapshot.engagement_score > 0.3 {
            "中 / Medium"
        } else {
            "低 / Low"
        };
        md.push_str(&format!(
            "- **参与度分数 / Engagement score**: {:.2} ({})\n",
            snapshot.engagement_score, eng_desc,
        ));

        md
    }

    /// 更新画像并防抖写盘 / Update profile with debounce write
    ///
    /// 每次消息处理后调用。累积到阈值时自动写盘。
    /// Called after each message processing. Auto-writes when threshold reached.
    pub fn tick(&mut self, snapshot: &UserProfileSnapshot) {
        self.unsaved_count += 1;
        self.cached_text = Self::generate_markdown(snapshot);

        if self.unsaved_count >= self.write_threshold {
            self.flush();
        }
    }

    /// 强制写盘 / Force flush to disk
    pub fn flush(&mut self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::File::create(&self.file_path) {
            Ok(mut file) => {
                let _ = file.write_all(self.cached_text.as_bytes());
                self.unsaved_count = 0;
            }
            Err(_) => {
                // 写盘失败不 panic，下次重试 / Don't panic on write failure, retry next time
            }
        }
    }

    /// 生成 prompt 注入片段 / Generate prompt injection fragment
    ///
    /// 返回 `[用户画像]` 格式的上下文文本，供 system prompt 使用。
    /// Returns `[用户画像]` formatted context text for system prompt.
    pub fn prompt_fragment(&self) -> String {
        if self.cached_text.is_empty() {
            String::new()
        } else {
            // 提取关键信息为简洁片段，避免注入完整 Markdown / Extract key info as concise fragment
            format!("[用户画像 / User Profile]\n{}", self.cached_text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 生成空画像_markdown格式正确() {
        // 测试空画像生成 / Test empty profile generation
        let facts: Vec<atrium_memory::fact_store::Fact> = vec![];
        let snapshot = UserProfileSnapshot {
            master_name: "主人",
            ai_name: "Atrium",
            facts: &facts,
            preference_text: String::new(),
            emotion_label: "平静",
            pad: (0.0, 0.0, 0.0),
            relationship_stage: "初识",
            total_interactions: 0,
            resonance_count: 0,
            return_count: 0,
            mental_model_summary: String::new(),
            engagement_score: 0.0,
        };

        let md = UserProfileManager::generate_markdown(&snapshot);
        assert!(md.contains("# 用户画像 / User Profile"));
        assert!(md.contains("关系阶段 / Stage**: 初识"));
        assert!(md.contains("暂无已知事实"));
        assert!(md.contains("暂无已知偏好"));
        assert!(md.contains("心智模型尚在构建"));
    }

    #[test]
    fn 生成完整画像_包含所有子系统数据() {
        // 测试完整画像生成 / Test full profile generation
        let facts = vec![atrium_memory::fact_store::Fact {
            subject: "主人".to_string(),
            predicate: "喜欢".to_string(),
            object: "猫".to_string(),
            confidence: 0.9,
            source: "对话提取".to_string(),
            created_at: 0,
            verified_at: 0,
            verify_count: 0,
            emotion_context: None,
            emotional_tag: None,
            emotional_salience: 0.0,
            pinned: false,
            actively_forgotten: None,
        }];
        let snapshot = UserProfileSnapshot {
            master_name: "主人",
            ai_name: "Atrium",
            facts: &facts,
            preference_text: "喜欢深度对话".to_string(),
            emotion_label: "愉悦",
            pad: (0.3, 0.2, 0.1),
            relationship_stage: "熟悉",
            total_interactions: 50,
            resonance_count: 5,
            return_count: 3,
            mental_model_summary: "用户偏好自然交流风格".to_string(),
            engagement_score: 0.75,
        };

        let md = UserProfileManager::generate_markdown(&snapshot);
        assert!(md.contains("主人 喜欢 猫"));
        assert!(md.contains("喜欢深度对话"));
        assert!(md.contains("用户偏好自然交流风格"));
        assert!(md.contains("参与度分数 / Engagement score**: 0.75"));
        assert!(md.contains("高 / High"));
    }

    #[test]
    fn 防抖写盘_达到阈值后自动写入() {
        // 测试防抖写盘 / Test debounce write
        let dir = std::env::temp_dir().join("atrium_profile_test_debounce");
        let mut mgr = UserProfileManager::new(dir.to_str().unwrap());
        let facts: Vec<atrium_memory::fact_store::Fact> = vec![];

        // 阈值 20，前 19 次不写 / Threshold 20, first 19 don't write
        for _ in 0..19 {
            let snapshot = UserProfileSnapshot {
                master_name: "主人",
                ai_name: "Atrium",
                facts: &facts,
                preference_text: String::new(),
                emotion_label: "平静",
                pad: (0.0, 0.0, 0.0),
                relationship_stage: "初识",
                total_interactions: 0,
                resonance_count: 0,
                return_count: 0,
                mental_model_summary: String::new(),
                engagement_score: 0.0,
            };
            mgr.tick(&snapshot);
        }
        assert!(
            !mgr.file_path.exists(),
            "第19次不应写盘 / Should not write on 19th"
        );

        // 第 20 次触发写盘 / 20th triggers write
        let snapshot = UserProfileSnapshot {
            master_name: "主人",
            ai_name: "Atrium",
            facts: &facts,
            preference_text: String::new(),
            emotion_label: "平静",
            pad: (0.0, 0.0, 0.0),
            relationship_stage: "初识",
            total_interactions: 0,
            resonance_count: 0,
            return_count: 0,
            mental_model_summary: String::new(),
            engagement_score: 0.0,
        };
        mgr.tick(&snapshot);
        assert!(
            mgr.file_path.exists(),
            "第20次应写盘 / Should write on 20th"
        );

        // 清理 / Cleanup
        let _ = fs::remove_file(&mgr.file_path);
    }

    #[test]
    fn prompt片段_非空时包含标记() {
        // 测试 prompt 片段生成 / Test prompt fragment generation
        let dir = std::env::temp_dir().join("atrium_profile_test_fragment");
        let mut mgr = UserProfileManager::new(dir.to_str().unwrap());
        let facts: Vec<atrium_memory::fact_store::Fact> = vec![];
        let snapshot = UserProfileSnapshot {
            master_name: "主人",
            ai_name: "Atrium",
            facts: &facts,
            preference_text: String::new(),
            emotion_label: "平静",
            pad: (0.0, 0.0, 0.0),
            relationship_stage: "初识",
            total_interactions: 0,
            resonance_count: 0,
            return_count: 0,
            mental_model_summary: String::new(),
            engagement_score: 0.0,
        };
        mgr.tick(&snapshot);
        let fragment = mgr.prompt_fragment();
        assert!(fragment.contains("[用户画像 / User Profile]"));
    }

    #[test]
    fn 启动加载_已有文件时缓存非空() {
        // 测试启动加载 / Test startup loading
        let dir = std::env::temp_dir().join("atrium_profile_test_load");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("user_profile.md");
        let _ = fs::write(&path, "# 用户画像 / User Profile\n已有数据 / Existing data");

        let mgr = UserProfileManager::new(dir.to_str().unwrap());
        assert!(mgr.cached_text().contains("已有数据"));

        let _ = fs::remove_file(&path);
    }
}
