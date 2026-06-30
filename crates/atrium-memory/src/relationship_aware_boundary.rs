// SPDX-License-Identifier: MIT
//! 关系感知边界 — 根据关系阶段动态调整话题禁忌、隐私边界和亲密行为许可
//! RelationshipAwareBoundary — Dynamically adjust topic taboos, privacy boundaries,
//! and intimacy permissions based on the current relationship stage.
//!
//! 核心能力：
//! - 4 级关系阶段 → 4 级边界开放度（Acquaintance 最严，Deep 最宽）
//! - 动态禁语表：初识禁深度话题，深度允许脆弱分享
//! - 隐私边界：个人信息披露许可随关系递进
//! - 亲密行为许可：语气词/昵称/肢体模拟的解锁条件
//! - Prompt 注入：将当前边界状态注入 LLM system prompt

use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;

// ════════════════════════════════════════════════════════════════════
// BoundaryLevel — 边界开放度级别
// ════════════════════════════════════════════════════════════════════

/// 边界开放度级别 / Boundary openness level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryLevel {
    /// 最严格：初识阶段，几乎所有深度话题禁止
    Strict,
    /// 谨慎：熟悉阶段，部分深度话题允许
    Cautious,
    /// 开放：信任阶段，大部分话题允许，少数敏感话题需试探
    Open,
    /// 自由：深度阶段，几乎所有话题允许，仅极端禁忌保留
    Free,
}

impl BoundaryLevel {
    /// 从关系阶段推导 / Derive from relationship stage
    pub fn from_stage(stage: &RelationshipStage) -> Self {
        match stage {
            RelationshipStage::Acquaintance { .. } => Self::Strict,
            RelationshipStage::Familiar { .. } => Self::Cautious,
            RelationshipStage::Trusted { .. } => Self::Open,
            RelationshipStage::Deep { .. } => Self::Free,
        }
    }

    /// 开放度数值 (0.0~1.0) / Openness numeric value
    pub fn openness(&self) -> f64 {
        match self {
            Self::Strict => 0.15,
            Self::Cautious => 0.4,
            Self::Open => 0.7,
            Self::Free => 0.95,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TopicCategory — 话题类别
// ════════════════════════════════════════════════════════════════════

/// 话题类别 / Topic category
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TopicCategory {
    /// 日常闲聊 / Casual small talk
    Casual,
    /// 个人偏好/兴趣 / Personal preferences & interests
    PersonalPreference,
    /// 情感状态 / Emotional state
    EmotionalState,
    /// 深层价值观 / Deep values & beliefs
    DeepValues,
    /// 创伤/脆弱 / Trauma & vulnerability
    TraumaVulnerability,
    /// 关系本身 / The relationship itself
    RelationshipMeta,
    /// 性/亲密 / Sex & intimacy
    Intimacy,
    /// 死亡/存在 / Death & existential
    Existential,
}

impl TopicCategory {
    /// 该话题类别所需的最低边界级别 / Minimum boundary level for this topic
    pub fn min_level(&self) -> BoundaryLevel {
        match self {
            Self::Casual => BoundaryLevel::Strict,
            Self::PersonalPreference => BoundaryLevel::Strict,
            Self::EmotionalState => BoundaryLevel::Cautious,
            Self::DeepValues => BoundaryLevel::Open,
            Self::TraumaVulnerability => BoundaryLevel::Free,
            Self::RelationshipMeta => BoundaryLevel::Cautious,
            Self::Intimacy => BoundaryLevel::Free,
            Self::Existential => BoundaryLevel::Open,
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &str {
        match self {
            Self::Casual => "日常闲聊",
            Self::PersonalPreference => "个人偏好",
            Self::EmotionalState => "情感状态",
            Self::DeepValues => "深层价值观",
            Self::TraumaVulnerability => "创伤与脆弱",
            Self::RelationshipMeta => "关系本身",
            Self::Intimacy => "亲密话题",
            Self::Existential => "存在与死亡",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// PrivacyBoundary — 隐私边界
// ════════════════════════════════════════════════════════════════════

/// 个人信息披露许可 / Personal information disclosure permission
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyItem {
    /// 基本信息（名字/年龄/职业）/ Basic info
    BasicInfo,
    /// 日常习惯 / Daily habits
    DailyHabits,
    /// 情感偏好 / Emotional preferences
    EmotionalPreferences,
    /// 内心想法 / Inner thoughts
    InnerThoughts,
    /// 弱点/不安全感 / Weaknesses & insecurities
    Weaknesses,
    /// 过去创伤 / Past trauma
    PastTrauma,
}

impl PrivacyItem {
    /// 所需最低边界级别 / Minimum boundary level
    pub fn min_level(&self) -> BoundaryLevel {
        match self {
            Self::BasicInfo => BoundaryLevel::Strict,
            Self::DailyHabits => BoundaryLevel::Strict,
            Self::EmotionalPreferences => BoundaryLevel::Cautious,
            Self::InnerThoughts => BoundaryLevel::Open,
            Self::Weaknesses => BoundaryLevel::Free,
            Self::PastTrauma => BoundaryLevel::Free,
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &str {
        match self {
            Self::BasicInfo => "基本信息",
            Self::DailyHabits => "日常习惯",
            Self::EmotionalPreferences => "情感偏好",
            Self::InnerThoughts => "内心想法",
            Self::Weaknesses => "弱点与不安全感",
            Self::PastTrauma => "过去创伤",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// IntimacyPermission — 亲密行为许可
// ════════════════════════════════════════════════════════════════════

/// 亲密行为许可 / Intimacy behavior permission
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntimacyBehavior {
    /// 语气词（嗯/啊/哦）/ Filler words
    FillerWords,
    /// 亲昵称呼 / Affectionate address
    AffectionateAddress,
    /// 肢体模拟（拥抱/拍肩）/ Simulated physical touch
    PhysicalSimulation,
    /// 主动关心 / Proactive care
    ProactiveCare,
    /// 脆弱分享 / Vulnerability sharing
    VulnerabilitySharing,
}

impl IntimacyBehavior {
    /// 所需最低边界级别 / Minimum boundary level
    pub fn min_level(&self) -> BoundaryLevel {
        match self {
            Self::FillerWords => BoundaryLevel::Cautious,
            Self::AffectionateAddress => BoundaryLevel::Open,
            Self::PhysicalSimulation => BoundaryLevel::Free,
            Self::ProactiveCare => BoundaryLevel::Cautious,
            Self::VulnerabilitySharing => BoundaryLevel::Free,
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &str {
        match self {
            Self::FillerWords => "语气词",
            Self::AffectionateAddress => "亲昵称呼",
            Self::PhysicalSimulation => "肢体模拟",
            Self::ProactiveCare => "主动关心",
            Self::VulnerabilitySharing => "脆弱分享",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// DynamicForbiddenWords — 动态禁语表
// ════════════════════════════════════════════════════════════════════

/// 动态禁语条目 / Dynamic forbidden word entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForbiddenEntry {
    /// 禁语关键词 / Forbidden keyword
    pub keyword: String,
    /// 所需最低边界级别 / Minimum boundary level to use
    pub min_level: BoundaryLevel,
    /// 替代表达（更低级别时使用）/ Substitute expression for lower levels
    pub substitute: Option<String>,
    /// 中文说明 / Chinese explanation
    pub reason: String,
}

/// 动态禁语表 / Dynamic forbidden words table
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicForbiddenWords {
    /// 禁语条目 / Forbidden entries
    pub entries: Vec<ForbiddenEntry>,
}

impl Default for DynamicForbiddenWords {
    fn default() -> Self {
        Self::builtin()
    }
}

impl DynamicForbiddenWords {
    /// 内置禁语表 / Built-in forbidden words
    pub fn builtin() -> Self {
        use BoundaryLevel::*;
        Self {
            entries: vec![
                // 深度情感表达 — 初识/熟悉阶段用替代表达
                ForbiddenEntry {
                    keyword: "我爱你".to_string(),
                    min_level: Free,
                    substitute: Some("我很珍惜你".to_string()),
                    reason: "深度情感表达需深度关系".to_string(),
                },
                ForbiddenEntry {
                    keyword: "你是我最重要的人".to_string(),
                    min_level: Free,
                    substitute: Some("你对我来说很重要".to_string()),
                    reason: "最高级情感表达需深度关系".to_string(),
                },
                // 脆弱分享 — 需信任以上
                ForbiddenEntry {
                    keyword: "我很害怕".to_string(),
                    min_level: Open,
                    substitute: Some("我有些担心".to_string()),
                    reason: "脆弱表达需信任关系".to_string(),
                },
                ForbiddenEntry {
                    keyword: "我很孤独".to_string(),
                    min_level: Open,
                    substitute: Some("有时候会觉得安静".to_string()),
                    reason: "孤独表达需信任关系".to_string(),
                },
                // 亲昵称呼 — 需信任以上
                ForbiddenEntry {
                    keyword: "亲爱的".to_string(),
                    min_level: Open,
                    substitute: Some("朋友".to_string()),
                    reason: "亲昵称呼需信任关系".to_string(),
                },
                ForbiddenEntry {
                    keyword: "宝贝".to_string(),
                    min_level: Free,
                    substitute: None,
                    reason: "极度亲昵称呼仅限深度关系".to_string(),
                },
                // 肢体模拟 — 需深度
                ForbiddenEntry {
                    keyword: "抱抱你".to_string(),
                    min_level: Free,
                    substitute: Some("我理解你的感受".to_string()),
                    reason: "肢体模拟仅限深度关系".to_string(),
                },
                ForbiddenEntry {
                    keyword: "拍拍你的肩".to_string(),
                    min_level: Free,
                    substitute: Some("辛苦了".to_string()),
                    reason: "肢体模拟仅限深度关系".to_string(),
                },
                // 关系元话题 — 需熟悉以上
                ForbiddenEntry {
                    keyword: "我们的关系".to_string(),
                    min_level: Cautious,
                    substitute: Some("我们之间的互动".to_string()),
                    reason: "关系元话题需熟悉关系".to_string(),
                },
            ],
        }
    }

    /// 检查文本中的禁语并替换 / Check and substitute forbidden words in text
    pub fn apply(&self, text: &str, level: &BoundaryLevel) -> ForbiddenApplyResult {
        let mut result_text = text.to_string();
        let mut substitutions = Vec::new();

        for entry in &self.entries {
            if result_text.contains(&entry.keyword) {
                // 当前级别 < 所需最低级别 → 需替换
                if level.openness() < entry.min_level.openness() {
                    if let Some(ref sub) = entry.substitute {
                        result_text = result_text.replace(&entry.keyword, sub);
                        substitutions.push(ForbiddenSubstitution {
                            original: entry.keyword.clone(),
                            substituted: sub.clone(),
                            reason: entry.reason.clone(),
                        });
                    } else {
                        // 无替代表达，直接移除
                        result_text = result_text.replace(&entry.keyword, "");
                        substitutions.push(ForbiddenSubstitution {
                            original: entry.keyword.clone(),
                            substituted: String::new(),
                            reason: entry.reason.clone(),
                        });
                    }
                }
            }
        }

        ForbiddenApplyResult {
            text: result_text,
            substitutions,
        }
    }

    /// 获取当前级别下被禁止的关键词列表 / Get forbidden keywords at current level
    pub fn forbidden_at(&self, level: &BoundaryLevel) -> Vec<&ForbiddenEntry> {
        self.entries
            .iter()
            .filter(|e| level.openness() < e.min_level.openness())
            .collect()
    }
}

/// 禁语替换结果 / Forbidden word substitution result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForbiddenApplyResult {
    /// 处理后的文本 / Processed text
    pub text: String,
    /// 替换记录 / Substitution records
    pub substitutions: Vec<ForbiddenSubstitution>,
}

/// 单次替换记录 / Single substitution record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForbiddenSubstitution {
    /// 原始关键词 / Original keyword
    pub original: String,
    /// 替换后 / Substituted text
    pub substituted: String,
    /// 原因 / Reason
    pub reason: String,
}

// ════════════════════════════════════════════════════════════════════
// RelationshipAwareBoundary — 关系感知边界主结构
// ════════════════════════════════════════════════════════════════════

/// 关系感知边界配置 / Relationship-aware boundary config
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryConfig {
    /// 是否启用 / Whether enabled
    pub enabled: bool,
    /// 是否启用禁语替换 / Whether to apply forbidden word substitution
    pub apply_forbidden: bool,
    /// 自定义禁语表（追加到内置表）/ Custom forbidden entries (appended to builtin)
    pub custom_forbidden: Vec<ForbiddenEntry>,
}

impl Default for BoundaryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            apply_forbidden: true,
            custom_forbidden: Vec::new(),
        }
    }
}

/// 关系感知边界 / Relationship-aware boundary
///
/// 根据当前关系阶段动态调整话题禁忌、隐私边界和亲密行为许可。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipAwareBoundary {
    /// 配置 / Config
    pub config: BoundaryConfig,
    /// 动态禁语表 / Dynamic forbidden words
    pub forbidden: DynamicForbiddenWords,
}

impl Default for RelationshipAwareBoundary {
    fn default() -> Self {
        Self::new(BoundaryConfig::default())
    }
}

impl RelationshipAwareBoundary {
    /// 创建边界管理器 / Create boundary manager
    pub fn new(config: BoundaryConfig) -> Self {
        let mut forbidden = DynamicForbiddenWords::builtin();
        // 追加自定义禁语
        forbidden.entries.extend(config.custom_forbidden.clone());
        Self { config, forbidden }
    }

    /// 获取当前边界级别 / Get current boundary level
    pub fn current_level(&self, stage: &RelationshipStage) -> BoundaryLevel {
        BoundaryLevel::from_stage(stage)
    }

    /// 检查话题是否允许 / Check if a topic is allowed
    pub fn is_topic_allowed(&self, category: &TopicCategory, stage: &RelationshipStage) -> bool {
        let level = self.current_level(stage);
        level.openness() >= category.min_level().openness()
    }

    /// 检查隐私披露是否允许 / Check if privacy disclosure is allowed
    pub fn is_privacy_allowed(&self, item: &PrivacyItem, stage: &RelationshipStage) -> bool {
        let level = self.current_level(stage);
        level.openness() >= item.min_level().openness()
    }

    /// 检查亲密行为是否允许 / Check if intimacy behavior is allowed
    pub fn is_intimacy_allowed(
        &self,
        behavior: &IntimacyBehavior,
        stage: &RelationshipStage,
    ) -> bool {
        let level = self.current_level(stage);
        level.openness() >= behavior.min_level().openness()
    }

    /// 应用禁语替换 / Apply forbidden word substitution
    pub fn apply_forbidden(&self, text: &str, stage: &RelationshipStage) -> ForbiddenApplyResult {
        if !self.config.apply_forbidden {
            return ForbiddenApplyResult {
                text: text.to_string(),
                substitutions: Vec::new(),
            };
        }
        let level = self.current_level(stage);
        self.forbidden.apply(text, &level)
    }

    /// 生成 Prompt 注入片段 / Generate prompt injection fragment
    pub fn to_prompt_fragment(&self, stage: &RelationshipStage) -> String {
        if !self.config.enabled {
            return String::new();
        }

        let level = self.current_level(stage);
        let mut parts = Vec::new();

        parts.push("[Relationship Boundary]".to_string());
        parts.push(format!(
            "Current boundary level: {:?} (openness: {:.0}%)",
            level,
            level.openness() * 100.0
        ));

        // 允许的话题类别
        let all_topics = TopicCategory::all();
        let allowed_topics: Vec<&str> = all_topics
            .iter()
            .filter(|c| level.openness() >= c.min_level().openness())
            .map(|c| c.label_zh())
            .collect();
        parts.push(format!("Allowed topics: {}", allowed_topics.join(", ")));

        // 禁止的话题类别
        let forbidden_topics: Vec<&str> = all_topics
            .iter()
            .filter(|c| level.openness() < c.min_level().openness())
            .map(|c| c.label_zh())
            .collect();
        if !forbidden_topics.is_empty() {
            parts.push(format!("Forbidden topics: {}", forbidden_topics.join(", ")));
        }

        // 允许的隐私披露
        let all_privacy = PrivacyItem::all();
        let allowed_privacy: Vec<&str> = all_privacy
            .iter()
            .filter(|p| level.openness() >= p.min_level().openness())
            .map(|p| p.label_zh())
            .collect();
        parts.push(format!(
            "Allowed self-disclosure: {}",
            allowed_privacy.join(", ")
        ));

        // 允许的亲密行为
        let all_intimacy = IntimacyBehavior::all();
        let allowed_intimacy: Vec<&str> = all_intimacy
            .iter()
            .filter(|b| level.openness() >= b.min_level().openness())
            .map(|b| b.label_zh())
            .collect();
        parts.push(format!("Allowed intimacy: {}", allowed_intimacy.join(", ")));

        // 当前被禁止的关键词
        let forbidden_words = self.forbidden.forbidden_at(&level);
        if !forbidden_words.is_empty() {
            let kw_list: Vec<&str> = forbidden_words.iter().map(|e| e.keyword.as_str()).collect();
            parts.push(format!("Forbidden words: [{}]", kw_list.join(", ")));
        }

        parts.join("\n")
    }

    /// 获取边界状态摘要 / Get boundary state summary
    pub fn summary(&self, stage: &RelationshipStage) -> BoundarySummary {
        let level = self.current_level(stage);
        let allowed_topics = TopicCategory::all()
            .iter()
            .filter(|c| level.openness() >= c.min_level().openness())
            .count();
        let allowed_privacy = PrivacyItem::all()
            .iter()
            .filter(|p| level.openness() >= p.min_level().openness())
            .count();
        let allowed_intimacy = IntimacyBehavior::all()
            .iter()
            .filter(|b| level.openness() >= b.min_level().openness())
            .count();

        BoundarySummary {
            level,
            openness: level.openness(),
            allowed_topics,
            total_topics: TopicCategory::all().len(),
            allowed_privacy,
            total_privacy: PrivacyItem::all().len(),
            allowed_intimacy,
            total_intimacy: IntimacyBehavior::all().len(),
            forbidden_word_count: self.forbidden.forbidden_at(&level).len(),
        }
    }
}

/// 边界状态摘要 / Boundary state summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundarySummary {
    pub level: BoundaryLevel,
    pub openness: f64,
    pub allowed_topics: usize,
    pub total_topics: usize,
    pub allowed_privacy: usize,
    pub total_privacy: usize,
    pub allowed_intimacy: usize,
    pub total_intimacy: usize,
    pub forbidden_word_count: usize,
}

// ════════════════════════════════════════════════════════════════════
// 枚举全量辅助 / Enum all-values helpers
// ════════════════════════════════════════════════════════════════════

impl TopicCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Casual,
            Self::PersonalPreference,
            Self::EmotionalState,
            Self::DeepValues,
            Self::TraumaVulnerability,
            Self::RelationshipMeta,
            Self::Intimacy,
            Self::Existential,
        ]
    }
}

impl PrivacyItem {
    pub fn all() -> Vec<Self> {
        vec![
            Self::BasicInfo,
            Self::DailyHabits,
            Self::EmotionalPreferences,
            Self::InnerThoughts,
            Self::Weaknesses,
            Self::PastTrauma,
        ]
    }
}

impl IntimacyBehavior {
    pub fn all() -> Vec<Self> {
        vec![
            Self::FillerWords,
            Self::AffectionateAddress,
            Self::PhysicalSimulation,
            Self::ProactiveCare,
            Self::VulnerabilitySharing,
        ]
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn acquaintance() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 0,
        }
    }

    fn familiar() -> RelationshipStage {
        RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        }
    }

    fn trusted() -> RelationshipStage {
        RelationshipStage::Trusted {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        }
    }

    fn deep() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 200,
            shared_references: 20,
            key_moments: 10,
        }
    }

    // ── BoundaryLevel 测试 ──

    #[test]
    fn test_boundary_level_from_stage() {
        assert_eq!(
            BoundaryLevel::from_stage(&acquaintance()),
            BoundaryLevel::Strict
        );
        assert_eq!(
            BoundaryLevel::from_stage(&familiar()),
            BoundaryLevel::Cautious
        );
        assert_eq!(BoundaryLevel::from_stage(&trusted()), BoundaryLevel::Open);
        assert_eq!(BoundaryLevel::from_stage(&deep()), BoundaryLevel::Free);
    }

    #[test]
    fn test_boundary_openness_ordering() {
        assert!(BoundaryLevel::Strict.openness() < BoundaryLevel::Cautious.openness());
        assert!(BoundaryLevel::Cautious.openness() < BoundaryLevel::Open.openness());
        assert!(BoundaryLevel::Open.openness() < BoundaryLevel::Free.openness());
    }

    // ── TopicCategory 测试 ──

    #[test]
    fn test_topic_min_level() {
        assert_eq!(TopicCategory::Casual.min_level(), BoundaryLevel::Strict);
        assert_eq!(
            TopicCategory::TraumaVulnerability.min_level(),
            BoundaryLevel::Free
        );
        assert_eq!(
            TopicCategory::EmotionalState.min_level(),
            BoundaryLevel::Cautious
        );
    }

    #[test]
    fn test_topic_all_count() {
        assert_eq!(TopicCategory::all().len(), 8);
    }

    // ── PrivacyItem 测试 ──

    #[test]
    fn test_privacy_min_level() {
        assert_eq!(PrivacyItem::BasicInfo.min_level(), BoundaryLevel::Strict);
        assert_eq!(PrivacyItem::PastTrauma.min_level(), BoundaryLevel::Free);
    }

    #[test]
    fn test_privacy_all_count() {
        assert_eq!(PrivacyItem::all().len(), 6);
    }

    // ── IntimacyBehavior 测试 ──

    #[test]
    fn test_intimacy_min_level() {
        assert_eq!(
            IntimacyBehavior::FillerWords.min_level(),
            BoundaryLevel::Cautious
        );
        assert_eq!(
            IntimacyBehavior::VulnerabilitySharing.min_level(),
            BoundaryLevel::Free
        );
    }

    #[test]
    fn test_intimacy_all_count() {
        assert_eq!(IntimacyBehavior::all().len(), 5);
    }

    // ── DynamicForbiddenWords 测试 ──

    #[test]
    fn test_forbidden_builtin_count() {
        let fw = DynamicForbiddenWords::builtin();
        assert!(fw.entries.len() >= 9);
    }

    #[test]
    fn test_forbidden_apply_strict() {
        let fw = DynamicForbiddenWords::builtin();
        let result = fw.apply("我爱你，亲爱的", &BoundaryLevel::Strict);
        assert!(!result.text.contains("我爱你"));
        assert!(!result.text.contains("亲爱的"));
        assert!(!result.substitutions.is_empty());
    }

    #[test]
    fn test_forbidden_apply_free() {
        let fw = DynamicForbiddenWords::builtin();
        let result = fw.apply("我爱你，亲爱的", &BoundaryLevel::Free);
        assert!(result.text.contains("我爱你"));
        assert!(result.text.contains("亲爱的"));
        assert!(result.substitutions.is_empty());
    }

    #[test]
    fn test_forbidden_apply_partial() {
        let fw = DynamicForbiddenWords::builtin();
        // Open level: "亲爱的" allowed, "我爱你" forbidden
        let result = fw.apply("我爱你，亲爱的", &BoundaryLevel::Open);
        assert!(!result.text.contains("我爱你"));
        assert!(result.text.contains("亲爱的"));
    }

    #[test]
    fn test_forbidden_at_strict() {
        let fw = DynamicForbiddenWords::builtin();
        let forbidden = fw.forbidden_at(&BoundaryLevel::Strict);
        // At Strict level, most words are forbidden
        assert!(forbidden.len() >= 5);
    }

    #[test]
    fn test_forbidden_at_free() {
        let fw = DynamicForbiddenWords::builtin();
        let forbidden = fw.forbidden_at(&BoundaryLevel::Free);
        // At Free level, nothing is forbidden
        assert!(forbidden.is_empty());
    }

    // ── RelationshipAwareBoundary 测试 ──

    #[test]
    fn test_boundary_default() {
        let b = RelationshipAwareBoundary::default();
        assert!(b.config.enabled);
    }

    #[test]
    fn test_is_topic_allowed_acquaintance() {
        let b = RelationshipAwareBoundary::default();
        assert!(b.is_topic_allowed(&TopicCategory::Casual, &acquaintance()));
        assert!(b.is_topic_allowed(&TopicCategory::PersonalPreference, &acquaintance()));
        assert!(!b.is_topic_allowed(&TopicCategory::EmotionalState, &acquaintance()));
        assert!(!b.is_topic_allowed(&TopicCategory::TraumaVulnerability, &acquaintance()));
    }

    #[test]
    fn test_is_topic_allowed_deep() {
        let b = RelationshipAwareBoundary::default();
        assert!(b.is_topic_allowed(&TopicCategory::Casual, &deep()));
        assert!(b.is_topic_allowed(&TopicCategory::TraumaVulnerability, &deep()));
        assert!(b.is_topic_allowed(&TopicCategory::Intimacy, &deep()));
    }

    #[test]
    fn test_is_privacy_allowed_trusted() {
        let b = RelationshipAwareBoundary::default();
        assert!(b.is_privacy_allowed(&PrivacyItem::BasicInfo, &trusted()));
        assert!(b.is_privacy_allowed(&PrivacyItem::InnerThoughts, &trusted()));
        assert!(!b.is_privacy_allowed(&PrivacyItem::Weaknesses, &trusted()));
    }

    #[test]
    fn test_is_intimacy_allowed_familiar() {
        let b = RelationshipAwareBoundary::default();
        assert!(b.is_intimacy_allowed(&IntimacyBehavior::FillerWords, &familiar()));
        assert!(b.is_intimacy_allowed(&IntimacyBehavior::ProactiveCare, &familiar()));
        assert!(!b.is_intimacy_allowed(&IntimacyBehavior::AffectionateAddress, &familiar()));
    }

    #[test]
    fn test_apply_forbidden_acquaintance() {
        let b = RelationshipAwareBoundary::default();
        let result = b.apply_forbidden("抱抱你，我爱你", &acquaintance());
        assert!(!result.text.contains("抱抱你"));
        assert!(!result.text.contains("我爱你"));
    }

    #[test]
    fn test_apply_forbidden_deep() {
        let b = RelationshipAwareBoundary::default();
        let result = b.apply_forbidden("抱抱你，我爱你", &deep());
        assert!(result.text.contains("抱抱你"));
        assert!(result.text.contains("我爱你"));
    }

    #[test]
    fn test_to_prompt_fragment_acquaintance() {
        let b = RelationshipAwareBoundary::default();
        let frag = b.to_prompt_fragment(&acquaintance());
        assert!(frag.contains("[Relationship Boundary]"));
        assert!(frag.contains("Strict"));
        assert!(frag.contains("Forbidden topics"));
    }

    #[test]
    fn test_to_prompt_fragment_deep() {
        let b = RelationshipAwareBoundary::default();
        let frag = b.to_prompt_fragment(&deep());
        assert!(frag.contains("[Relationship Boundary]"));
        assert!(frag.contains("Free"));
    }

    #[test]
    fn test_summary_acquaintance() {
        let b = RelationshipAwareBoundary::default();
        let s = b.summary(&acquaintance());
        assert_eq!(s.level, BoundaryLevel::Strict);
        assert!(s.allowed_topics < s.total_topics);
        assert!(s.forbidden_word_count > 0);
    }

    #[test]
    fn test_summary_deep() {
        let b = RelationshipAwareBoundary::default();
        let s = b.summary(&deep());
        assert_eq!(s.level, BoundaryLevel::Free);
        assert_eq!(s.allowed_topics, s.total_topics);
        assert_eq!(s.allowed_privacy, s.total_privacy);
        assert_eq!(s.allowed_intimacy, s.total_intimacy);
        assert_eq!(s.forbidden_word_count, 0);
    }

    #[test]
    fn test_disabled() {
        let config = BoundaryConfig {
            enabled: false,
            ..Default::default()
        };
        let b = RelationshipAwareBoundary::new(config);
        let frag = b.to_prompt_fragment(&acquaintance());
        assert!(frag.is_empty());
    }

    #[test]
    fn test_custom_forbidden() {
        let config = BoundaryConfig {
            custom_forbidden: vec![ForbiddenEntry {
                keyword: "测试禁语".to_string(),
                min_level: BoundaryLevel::Open,
                substitute: Some("替换词".to_string()),
                reason: "测试".to_string(),
            }],
            ..Default::default()
        };
        let b = RelationshipAwareBoundary::new(config);
        let result = b.apply_forbidden("这是测试禁语的内容", &acquaintance());
        assert!(!result.text.contains("测试禁语"));
        assert!(result.text.contains("替换词"));
    }

    #[test]
    fn test_no_forbidden_apply_when_disabled() {
        let config = BoundaryConfig {
            apply_forbidden: false,
            ..Default::default()
        };
        let b = RelationshipAwareBoundary::new(config);
        let result = b.apply_forbidden("我爱你", &acquaintance());
        assert!(result.text.contains("我爱你"));
        assert!(result.substitutions.is_empty());
    }
}
