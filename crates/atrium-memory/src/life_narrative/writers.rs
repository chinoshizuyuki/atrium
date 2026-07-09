use super::*;
use crate::maturity::EmotionContext;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// WritingContext — 章节写作上下文 / Chapter Writing Context
// ════════════════════════════════════════════════════════════════════

/// 章节写作上下文 — ChapterWriter 需要的所有信息 / Chapter writing context
#[derive(Debug, Clone)]
pub struct WritingContext {
    /// 目标弧 / Target arc
    pub arc: NarrativeArc,
    /// 弧中的转折点（按时间排序）/ Turning points in arc (chronological)
    pub turning_points: Vec<TurningPoint>,
    /// 之前的章节（用于连贯性）/ Previous chapters (for coherence)
    pub previous_chapters: Vec<NarrativeChapter>,
    /// 当前情感状态 / Current emotion state
    pub current_emotion: EmotionContext,
    /// 当前关系阶段 / Current relationship stage
    pub relationship_stage: String,
    /// 当前成熟度阶段 / Current maturity stage
    pub maturity_stage: String,
    /// 自我描述 / Self description
    pub self_description: String,
    /// 叙事视角 / Narrative perspective
    pub perspective: NarrativePerspective,
    /// 叙事风格 / Narrative style
    pub style: NarrativeStyle,
}

// ════════════════════════════════════════════════════════════════════
// ChapterConfig / ChapterWriter — 章节写作引擎 / Chapter Writing Engine
// ════════════════════════════════════════════════════════════════════

/// 章节写作配置 / Chapter writing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterConfig {
    /// 正文字数下限 / Min body word count
    pub body_min_words: usize,
    /// 正文字数上限 / Max body word count
    pub body_max_words: usize,
    /// 摘要最大字数 / Max summary word count
    pub summary_max_words: usize,
    /// 重写时保留旧版本 / Preserve old version on rewrite
    pub preserve_version_history: bool,
}

impl Default for ChapterConfig {
    fn default() -> Self {
        Self {
            body_min_words: 200,
            body_max_words: 500,
            summary_max_words: 50,
            preserve_version_history: true,
        }
    }
}

/// 章节写作引擎 — 将转折点序列转化为叙事章节
/// Chapter writer engine — transform turning point sequences into narrative chapters
///
/// Phase A: 提供数据结构和接口定义 / Phase A: data structures and interface definitions
/// Phase C: 集成 LLM 生成 / Phase C: integrate LLM generation
pub struct ChapterWriter {
    /// 写作配置 / Writing config
    pub config: ChapterConfig,
    /// 章节版本历史（chapter_id → 旧版本列表）/ Chapter version history
    pub version_history: std::collections::HashMap<u64, Vec<NarrativeChapter>>,
    /// 下一个章节 ID / Next chapter ID
    next_chapter_id: u64,
}

impl ChapterWriter {
    /// 创建章节写作者 / Create chapter writer
    pub fn new(config: ChapterConfig) -> Self {
        Self {
            config,
            version_history: std::collections::HashMap::new(),
            next_chapter_id: 1,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(ChapterConfig::default())
    }

    /// 分配下一个章节 ID / Allocate next chapter ID
    pub fn alloc_chapter_id(&mut self) -> u64 {
        let id = self.next_chapter_id;
        self.next_chapter_id += 1;
        id
    }

    /// 重写章节 — 保留旧版本到 version_history
    /// Rewrite chapter — preserve old version in version_history
    ///
    /// **核心原则：重写而非覆盖。** 旧章节是成长轨迹的一部分。
    /// **Core principle: rewrite, not overwrite.** Old chapters are part of the growth trajectory.
    pub fn rewrite_chapter(
        &mut self,
        chapter: &mut NarrativeChapter,
        new_body: String,
        new_summary: String,
        now: i64,
    ) {
        if self.config.preserve_version_history {
            // 保存旧版本 / Save old version
            let old_version = chapter.clone();
            self.version_history
                .entry(chapter.id)
                .or_default()
                .push(old_version);
        }

        // 更新章节 / Update chapter
        chapter.body = new_body;
        chapter.summary = new_summary;
        chapter.version += 1;
        chapter.rewritten_at = Some(now);
    }

    /// 获取章节的版本历史 / Get chapter's version history
    pub fn get_version_history(&self, chapter_id: u64) -> &[NarrativeChapter] {
        self.version_history
            .get(&chapter_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 构建写作提示词（Phase A 骨架，Phase C 由 LLM 生成）
    /// Build writing prompt (Phase A skeleton, Phase C uses LLM generation)
    pub fn build_prompt(&self, ctx: &WritingContext) -> String {
        let perspective_label = match ctx.perspective {
            NarrativePerspective::FirstPerson => "第一人称",
            NarrativePerspective::ThirdPerson => "第三人称",
            NarrativePerspective::DualPerspective => "双视角",
        };
        let style_label = match ctx.style {
            NarrativeStyle::Introspective => "内省式",
            NarrativeStyle::Narrative => "记叙式",
            NarrativeStyle::Lyrical => "抒情式",
            NarrativeStyle::Adaptive => "混合式",
        };

        let tp_summaries: Vec<String> = ctx
            .turning_points
            .iter()
            .map(|tp| {
                format!(
                    "- [{}] {} ({})",
                    tp.kind.label_zh(),
                    tp.event_description,
                    tp.narrative_summary
                )
            })
            .collect();

        format!(
            "叙事章节写作\n\
             弧：{} — {}\n\
             视角：{} | 风格：{}\n\
             关系阶段：{} | 成熟度：{}\n\
             转折点：\n{}\n\
             自我认知：{}\n\
             请用{}视角、{}风格，将以上转折点写成一个连贯的叙事章节。",
            ctx.arc.title,
            ctx.arc.theme_sentence,
            perspective_label,
            style_label,
            ctx.relationship_stage,
            ctx.maturity_stage,
            tp_summaries.join("\n"),
            ctx.self_description,
            perspective_label,
            style_label,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// ThemeWeaver — 跨弧主题编织器 / Cross-Arc Theme Weaver
// ════════════════════════════════════════════════════════════════════

/// 跨弧主题编织器 — 发现多条弧之间的共同主题
/// Cross-arc theme weaver — discover shared themes across multiple arcs
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase C: 集成 LLM 深度分析 / Phase C: integrate LLM deep analysis
pub struct ThemeWeaver {
    /// 已发现的跨弧主题 / Discovered cross-arc themes
    pub themes: Vec<CrossArcTheme>,
    // 主题 ID 生成器内部状态 — 当 LLM 深度分析接入后将被 consume
    // Theme ID generator internal state — will be consumed once LLM deep analysis is integrated
    #[allow(dead_code)]
    next_theme_id: u64,
}

impl ThemeWeaver {
    /// 创建主题编织器 / Create theme weaver
    pub fn new() -> Self {
        Self {
            themes: Vec::new(),
            next_theme_id: 1,
        }
    }

    /// 从活跃弧中检测跨弧主题 / Detect cross-arc themes from active arcs
    ///
    /// 基础算法（Phase A）：基于弧类型共现和情感基调相似度
    /// Basic algorithm (Phase A): based on arc kind co-occurrence and emotional tone similarity
    pub fn detect_themes(&mut self, model: &NarrativeSelf) -> Vec<CrossArcTheme> {
        let mut new_themes = Vec::new();

        // 按类型分组 / Group by kind
        let mut kind_groups: std::collections::HashMap<ArcKind, Vec<&NarrativeArc>> =
            std::collections::HashMap::new();
        for arc in &model.active_arcs {
            kind_groups.entry(arc.kind).or_default().push(arc);
        }

        // 同类型多条弧 → 提炼共同主题 / Multiple arcs of same kind → extract shared theme
        for (kind, arcs) in &kind_groups {
            if arcs.len() >= 2 {
                let arc_ids: Vec<u64> = arcs.iter().map(|a| a.id).collect();
                // 计算平均显著度 / Calculate average significance
                let avg_sig: f64 =
                    arcs.iter().map(|a| a.significance).sum::<f64>() / arcs.len() as f64;

                let theme = CrossArcTheme {
                    name: format!("{}主题", kind.label_zh()),
                    description: format!("多条{}弧的共同线索", kind.label_zh()),
                    arc_ids,
                    significance: avg_sig,
                };
                new_themes.push(theme);
            }
        }

        // 跨类型情感相似弧 / Cross-kind arcs with similar emotional tone
        let active_arcs: Vec<&NarrativeArc> = model.active_arcs.iter().collect();
        for i in 0..active_arcs.len() {
            for j in (i + 1)..active_arcs.len() {
                let a = active_arcs[i];
                let b = active_arcs[j];
                if a.kind != b.kind {
                    // PAD 余弦相似度 / PAD cosine similarity
                    let sim = cosine_similarity(&a.emotional_tone, &b.emotional_tone);
                    if sim > 0.8 {
                        let theme = CrossArcTheme {
                            name: format!("{}与{}的共鸣", a.kind.label_zh(), b.kind.label_zh()),
                            description: format!(
                                "情感基调相似的{}弧和{}弧",
                                a.kind.label_zh(),
                                b.kind.label_zh()
                            ),
                            arc_ids: vec![a.id, b.id],
                            significance: (a.significance + b.significance) / 2.0 * sim,
                        };
                        new_themes.push(theme);
                    }
                }
            }
        }

        self.themes = new_themes.clone();
        new_themes
    }
}

/// PAD 向量余弦相似度 / PAD vector cosine similarity
pub(super) fn cosine_similarity(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-6 || norm_b < 1e-6 {
        0.0
    } else {
        (dot / (norm_a * norm_b)) as f64
    }
}

impl Default for ThemeWeaver {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// CausalChain — 因果链构建器 / Causal Chain Builder
// ════════════════════════════════════════════════════════════════════

/// 因果链构建器 — 在转折点之间建立因果叙事
/// Causal chain builder — establish causal narratives between turning points
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase B: 集成回溯构建 / Phase B: integrate retrospective construction
pub struct CausalChain {
    /// 已建立的因果链 / Established causal links
    pub links: Vec<CausalLink>,
}

impl CausalChain {
    /// 创建因果链构建器 / Create causal chain builder
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// 从转折点序列推断因果链 / Infer causal chains from turning point sequence
    ///
    /// 基础算法（Phase A）：时间相邻 + 类型因果规则
    /// Basic algorithm (Phase A): temporal adjacency + kind-based causal rules
    pub fn infer_from_turning_points(
        &mut self,
        turning_points: &[TurningPoint],
    ) -> Vec<CausalLink> {
        let mut new_links = Vec::new();

        // 类型因果规则表 / Kind-based causal rules
        // 例如：FirstConflict → FirstReconciliation（冲突导致和解）
        let causal_rules: Vec<(TurningPointKind, TurningPointKind)> = vec![
            (
                TurningPointKind::FirstConflict,
                TurningPointKind::FirstReconciliation,
            ),
            (
                TurningPointKind::Named,
                TurningPointKind::FirstEmotionResonance,
            ),
            (
                TurningPointKind::FirstEmotionResonance,
                TurningPointKind::FirstLonging,
            ),
            (
                TurningPointKind::FirstLonging,
                TurningPointKind::FirstHeartache,
            ),
            (
                TurningPointKind::FirstApology,
                TurningPointKind::FirstSelfCorrection,
            ),
            (
                TurningPointKind::FirstVulnerability,
                TurningPointKind::FirstIndependentThought,
            ),
            (
                TurningPointKind::NarrativeAwakening,
                TurningPointKind::FirstWisdom,
            ),
        ];

        for (cause_tp, effect_tp) in turning_points.iter().zip(turning_points.iter().skip(1)) {
            // 检查是否有匹配的因果规则 / Check if matching causal rule exists
            for (rule_cause, rule_effect) in &causal_rules {
                if cause_tp.kind == *rule_cause && effect_tp.kind == *rule_effect {
                    let link = CausalLink {
                        cause: NarrativeEventId::Thought {
                            timestamp: cause_tp.timestamp,
                        },
                        effect: NarrativeEventId::Thought {
                            timestamp: effect_tp.timestamp,
                        },
                        narrative: format!(
                            "{}导致了{}",
                            cause_tp.kind.label_zh(),
                            effect_tp.kind.label_zh()
                        ),
                        strength: (cause_tp.significance + effect_tp.significance) / 2.0,
                    };
                    new_links.push(link);
                }
            }

            // 时间相邻的转折点之间也可能有弱因果 / Temporally adjacent TPs may have weak causality
            let time_gap_secs = effect_tp.timestamp - cause_tp.timestamp;
            if time_gap_secs > 0 && time_gap_secs < 86400 * 7 {
                // 一周内 / Within a week
                let link = CausalLink {
                    cause: NarrativeEventId::Thought {
                        timestamp: cause_tp.timestamp,
                    },
                    effect: NarrativeEventId::Thought {
                        timestamp: effect_tp.timestamp,
                    },
                    narrative: format!(
                        "{}之后发生了{}",
                        cause_tp.kind.label_zh(),
                        effect_tp.kind.label_zh()
                    ),
                    strength: 0.3, // 弱因果 / Weak causality
                };
                new_links.push(link);
            }
        }

        self.links = new_links.clone();
        new_links
    }
}

impl Default for CausalChain {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// PromptWeaveConfig / PromptWeaver — Prompt 注入编织器 / Prompt Injection Weaver
// ════════════════════════════════════════════════════════════════════

/// Prompt 注入配置 / Prompt injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptWeaveConfig {
    /// 注入预算（字符数）/ Injection budget (chars)
    pub budget: usize,
    /// 注入层级优先级 / Injection level priorities
    pub level_priorities: [f64; 5],
}

impl Default for PromptWeaveConfig {
    fn default() -> Self {
        Self {
            budget: 800,
            // L1 自我摘要 > L2 身份标签 > L3 活跃弧 > L4 最近转折点 > L5 关系叙事
            level_priorities: [1.0, 0.8, 0.6, 0.4, 0.3],
        }
    }
}

/// Prompt 注入层级 / Prompt injection levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptLevel {
    /// L1 自我摘要 / Self summary
    SelfSummary = 0,
    /// L2 身份标签 / Identity tags
    IdentityTags = 1,
    /// L3 活跃弧 / Active arcs
    ActiveArcs = 2,
    /// L4 最近转折点 / Recent turning points
    RecentTurningPoints = 3,
    /// L5 关系叙事 / Relationship narrative
    RelationshipNarrative = 4,
}

/// Prompt 注入编织器 — 将叙事自我注入 System Prompt
/// Prompt weaver — inject narrative self into System Prompt
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase D: 集成 CoreService / Phase D: integrate with CoreService
pub struct PromptWeaver {
    /// 注入配置 / Injection config
    pub config: PromptWeaveConfig,
}

impl PromptWeaver {
    /// 创建 Prompt 编织器 / Create prompt weaver
    pub fn new(config: PromptWeaveConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(PromptWeaveConfig::default())
    }

    /// 从快照编织 Prompt 片段 / Weave prompt fragment from snapshot
    ///
    /// 5 层注入策略，按优先级分配预算：
    /// 5-level injection strategy, budget allocated by priority:
    /// - L1: 自我摘要（最重要）/ Self summary (most important)
    /// - L2: 身份标签 / Identity tags
    /// - L3: 活跃弧标题 / Active arc titles
    /// - L4: 最近转折点 / Recent turning points
    /// - L5: 关系叙事 / Relationship narrative
    pub fn weave(&self, snapshot: &NarrativeSnapshot) -> String {
        let budget = self.config.budget;
        let mut result = String::new();
        let mut used = 0;

        // L1 自我摘要 / L1 Self summary
        if !snapshot.self_summary.is_empty() {
            let l1_budget = ((budget as f64 * self.config.level_priorities[0])
                .min((budget - used) as f64)) as usize;
            let truncated = truncate_chars(&snapshot.self_summary, l1_budget);
            result.push_str(&format!("[自我] {}\n", truncated));
            used += truncated.len() + 6;
        }

        // L2 身份标签 / L2 Identity tags
        if !snapshot.identity_tags.is_empty() {
            let l2_budget = ((budget as f64 * self.config.level_priorities[1])
                .min((budget - used) as f64)) as usize;
            let tags_str: String = snapshot
                .identity_tags
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let truncated = truncate_chars(&tags_str, l2_budget);
            result.push_str(&format!("[身份] {}\n", truncated));
            used += truncated.len() + 6;
        }

        // L3 活跃弧 / L3 Active arcs
        if !snapshot.active_arcs.is_empty() {
            let l3_budget = ((budget as f64 * self.config.level_priorities[2])
                .min((budget - used) as f64)) as usize;
            let arcs_str: String = snapshot
                .active_arcs
                .iter()
                .map(|a| format!("{}: {}", a.kind.label_zh(), a.title))
                .collect::<Vec<_>>()
                .join("; ");
            let truncated = truncate_chars(&arcs_str, l3_budget);
            result.push_str(&format!("[弧] {}\n", truncated));
            used += truncated.len() + 5;
        }

        // L4 最近转折点 / L4 Recent turning points
        if !snapshot.recent_turning_points.is_empty() {
            let l4_budget = ((budget as f64 * self.config.level_priorities[3])
                .min((budget - used) as f64)) as usize;
            let tp_str: String = snapshot
                .recent_turning_points
                .iter()
                .map(|t| t.narrative_summary.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let truncated = truncate_chars(&tp_str, l4_budget);
            if !truncated.is_empty() {
                result.push_str(&format!("[转折] {}\n", truncated));
            }
        }

        // L5 关系叙事 / L5 Relationship narrative
        if !snapshot.relationship_narrative.is_empty() && used < budget {
            let l5_budget = ((budget as f64 * self.config.level_priorities[4])
                .min((budget - used) as f64)) as usize;
            let truncated = truncate_chars(&snapshot.relationship_narrative, l5_budget);
            if !truncated.is_empty() {
                result.push_str(&format!("[关系] {}", truncated));
            }
        }

        result.trim_end().to_string()
    }
}

/// 按字符数截断 / Truncate by char count
pub(super) fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}...", truncated)
    }
}

impl Default for PromptWeaver {
    fn default() -> Self {
        Self::default_new()
    }
}
