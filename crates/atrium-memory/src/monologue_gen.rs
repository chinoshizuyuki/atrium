// SPDX-License-Identifier: MIT
//! 独白内容生成器 — LLM 驱动的内心独白/日记/白日梦/自主学习内容生成
//! Monologue Content Generator — LLM-driven generation for inner monologue, diary, daydream, and learning.
//!
//! 设计原则 / Design principles:
//! - InnerMonologueEngine 只管状态（冷却、计数、环形缓冲区）
//! - InnerMonologueEngine only manages state (cooldowns, counters, ring buffer)
//! - 本模块负责所有 LLM 调用，返回生成结果供 CoreService 写入引擎
//! - This module handles all LLM calls, returning results for CoreService to record in the engine
//! - 异步方法不阻塞 Scheduler tick，由 CoreService 在 async 上下文中调用
//! - Async methods never block the Scheduler tick; CoreService calls them in async context
//!
//! 调用流程 / Call flow:
//! ```text
//! CoreService::tick_idle()
//!   → engine.can_graph_wander(now)?
//!   → MonologueGenerator::generate_graph_wander(...)
//!   → engine.record_graph_wander(now)
//!   → engine.add_thought(thought)
//! ```

use crate::inner_monologue::{InnerThought, ThoughtMode};
use crate::llm_client::{LlmCallKind, LlmClient, LlmError};
use crate::maturity::EmotionContext;
use crate::prompts;

use std::sync::Arc;

// ════════════════════════════════════════════════════════════════════
// MonologueGenerator — 独白内容生成器 / Monologue Content Generator
// ════════════════════════════════════════════════════════════════════

/// 独白内容生成器 — 持有 LLM 客户端，提供各模式的异步生成方法
/// Monologue content generator — Holds LLM client, provides async generation methods per mode.
///
/// 每个生成方法：
/// 1. 从数据源收集上下文（图节点、日记、记忆碎片等）
/// 2. 用 `prompts` 模板构造 system + user prompt
/// 3. 调用 `LlmClient::generate_with_limit()` 获取 LLM 输出
/// 4. 包装为 `InnerThought` 返回
///
/// Each generation method:
/// 1. Collects context from data sources (graph nodes, diary, memory fragments, etc.)
/// 2. Constructs system + user prompt using `prompts` templates
/// 3. Calls `LlmClient::generate_with_limit()` for LLM output
/// 4. Wraps result as `InnerThought`
pub struct MonologueGenerator {
    /// LLM 客户端 / LLM client (injected, can be mock for testing)
    llm: Arc<dyn LlmClient>,
}

impl MonologueGenerator {
    /// 创建生成器 / Create generator with the given LLM client.
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self { llm }
    }

    // ────────────────────────────────────────────────────────────────
    // B1.1 GraphWander — 图漫游内心独白 / Graph Wander Inner Monologue
    // ────────────────────────────────────────────────────────────────

    /// 生成图漫游内心独白 — 从种子节点出发，沿关联路径思考
    /// Generate graph wander inner monologue — Think from seed node along associative paths.
    ///
    /// # 参数 / Parameters
    /// - `seed_content`: 种子节点内容 / Seed node content
    /// - `neighbors`: 关联节点列表 (内容, 权重) / Neighbor list (content, weight)
    /// - `recent_thoughts`: 最近思考摘要 / Recent thought summaries
    /// - `emotion`: 当前情感上下文 / Current emotion context
    ///
    /// # 返回 / Returns
    /// - `Ok(InnerThought)`: 生成成功 / Generation succeeded
    /// - `Err(LlmError)`: LLM 调用失败 / LLM call failed
    pub async fn generate_graph_wander(
        &self,
        seed_content: &str,
        neighbors: &[(String, f64)],
        recent_thoughts: &[String],
        emotion: Option<EmotionContext>,
    ) -> Result<InnerThought, LlmError> {
        // 构造 prompt / Build prompt
        let neighbors_text = prompts::format_neighbors(neighbors);
        let thoughts_text = prompts::format_recent_thoughts(recent_thoughts, 80);

        let user_prompt = prompts::render_template(
            prompts::PROMPT_GRAPH_WANDER,
            &[
                ("seed_node", seed_content),
                ("neighbors", &neighbors_text),
                ("recent_thoughts", &thoughts_text),
            ],
        );

        // 调用 LLM / Call LLM
        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        let content = self
            .llm
            .generate_with_limit(
                LlmCallKind::GraphWander,
                Some(prompts::SYSTEM_INNER_MONOLOGUE),
                &user_prompt,
                0.7, // 图漫游温度 — 创造性联想 / Graph wander temperature — creative association
                prompts::GRAPH_WANDER_MAX_TOKENS,
            )
            .await?
            .content;

        // 构造 InnerThought / Build InnerThought
        let now = chrono::Utc::now().timestamp();
        Ok(InnerThought {
            content,
            mode: ThoughtMode::GraphWander,
            confidence: 0.7, // 图漫游置信度中等 / Graph wander has moderate confidence
            emotion,
            timestamp: now,
            shareable: true, // 图漫游思考可分享 / Graph wander thoughts are shareable
            graph_seed: Some(seed_content.to_string()),
        })
    }

    // ────────────────────────────────────────────────────────────────
    // B1.2 DiaryEntry — 日记自动生成 / Diary Auto-Generation
    // ────────────────────────────────────────────────────────────────

    /// 生成日记内容 — 基于当日事件和情感曲线
    /// Generate diary content — Based on daily events and emotion trajectory.
    ///
    /// # 参数 / Parameters
    /// - `date`: 日期字符串 (YYYY-MM-DD) / Date string
    /// - `key_events`: 当日关键事件 / Key events of the day
    /// - `emotion_summary`: 情感摘要文本 / Emotion summary text
    /// - `thought_count`: 当日思考数 / Thought count today
    /// - `recent_diary`: 最近日记摘要 / Recent diary summaries
    /// - `emotion`: 当前情感上下文 / Current emotion context
    pub async fn generate_diary_entry(
        &self,
        date: &str,
        key_events: &str,
        emotion_summary: &str,
        thought_count: u32,
        recent_diary: &str,
        emotion: Option<EmotionContext>,
    ) -> Result<InnerThought, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_DIARY_ENTRY,
            &[
                ("date", date),
                ("key_events", key_events),
                ("emotion_summary", emotion_summary),
                ("thought_count", &thought_count.to_string()),
                ("recent_diary", recent_diary),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        let content = self
            .llm
            .generate_with_limit(
                LlmCallKind::DiaryEntry,
                Some(prompts::SYSTEM_DIARY),
                &user_prompt,
                0.7, // 日记温度 — 自然叙述 / Diary temperature — natural narrative
                prompts::DIARY_ENTRY_MAX_TOKENS,
            )
            .await?
            .content;

        let now = chrono::Utc::now().timestamp();
        Ok(InnerThought {
            content,
            mode: ThoughtMode::DiaryEntry,
            confidence: 0.85, // 日记置信度较高 / Diary has higher confidence
            emotion,
            timestamp: now,
            shareable: false, // 日记私密，不分享 / Diary is private, not shareable
            graph_seed: None,
        })
    }

    // ────────────────────────────────────────────────────────────────
    // B1.3 Daydream — 白日梦 / Daydream
    // ────────────────────────────────────────────────────────────────

    /// 生成白日梦 — 随机重组记忆碎片
    /// Generate daydream — Randomly recombine memory fragments.
    ///
    /// # 参数 / Parameters
    /// - `fragments`: 记忆碎片文本 / Memory fragments text
    /// - `emotion_hint`: 当前情感暗示 / Current emotion hint
    /// - `emotion`: 当前情感上下文 / Current emotion context
    pub async fn generate_daydream(
        &self,
        fragments: &str,
        emotion_hint: &str,
        emotion: Option<EmotionContext>,
    ) -> Result<InnerThought, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_DAYDREAM,
            &[("fragments", fragments), ("emotion_hint", emotion_hint)],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        let content = self
            .llm
            .generate_with_limit(
                LlmCallKind::Daydream,
                Some(prompts::SYSTEM_DAYDREAM),
                &user_prompt,
                0.8, // 白日梦温度 — 高创造性自由联想 / Daydream temperature — high creative free association
                prompts::DAYDREAM_MAX_TOKENS,
            )
            .await?
            .content;

        let now = chrono::Utc::now().timestamp();
        Ok(InnerThought {
            content,
            mode: ThoughtMode::Daydream,
            confidence: 0.3, // 白日梦置信度低 / Daydreams have low confidence
            emotion,
            timestamp: now,
            shareable: false, // 白日梦私密 / Daydreams are private
            graph_seed: None,
        })
    }

    // ────────────────────────────────────────────────────────────────
    // B1.4 AutonomousLearning — 自主学习 / Autonomous Learning
    // ────────────────────────────────────────────────────────────────

    /// 生成自主学习心得 — 从知识中提炼洞察
    /// Generate autonomous learning insight — Distill insights from knowledge.
    ///
    /// # 参数 / Parameters
    /// - `knowledge`: ACK 知识库内容 / ACK knowledge base content
    /// - `related_facts`: 与知识相关的事实 / Facts related to the knowledge
    /// - `existing_insights`: 已有洞察 / Existing insights
    /// - `emotion`: 当前情感上下文 / Current emotion context
    pub async fn generate_autonomous_learning(
        &self,
        knowledge: &str,
        related_facts: &str,
        existing_insights: &str,
        emotion: Option<EmotionContext>,
    ) -> Result<InnerThought, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_AUTONOMOUS_LEARNING,
            &[
                ("knowledge", knowledge),
                ("related_facts", related_facts),
                ("existing_insights", existing_insights),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        let content = self
            .llm
            .generate_with_limit(
                LlmCallKind::AutonomousLearning,
                Some(prompts::SYSTEM_AUTONOMOUS_LEARNING),
                &user_prompt,
                0.5, // 自主学习温度 — 适度分析性 / Learning temperature — moderately analytical
                prompts::AUTONOMOUS_LEARNING_MAX_TOKENS,
            )
            .await?
            .content;

        let now = chrono::Utc::now().timestamp();
        Ok(InnerThought {
            content,
            mode: ThoughtMode::AutonomousLearning,
            confidence: 0.6, // 学习心得置信度中等 / Learning insights have moderate confidence
            emotion,
            timestamp: now,
            shareable: true, // 学习心得可分享 / Learning insights are shareable
            graph_seed: None,
        })
    }

    // ────────────────────────────────────────────────────────────────
    // B1.5 日记驱动反思 / Diary-Driven Reflection
    // ────────────────────────────────────────────────────────────────

    /// 生成日记反思 — 从多天日记中提炼高阶洞察
    /// Generate diary reflection — Distill higher-order insights from multiple days of diary.
    ///
    /// # 参数 / Parameters
    /// - `diary_entries`: 最近 N 天的日记 / Recent N days of diary entries
    /// - `current_insights`: 当前已有洞察 / Current existing insights
    /// - `fact_summary`: 事实库摘要 / Fact store summary
    /// - `emotion`: 当前情感上下文 / Current emotion context
    pub async fn generate_diary_reflection(
        &self,
        diary_entries: &str,
        current_insights: &str,
        fact_summary: &str,
        emotion: Option<EmotionContext>,
    ) -> Result<InnerThought, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_DIARY_REFLECTION,
            &[
                ("diary_entries", diary_entries),
                ("current_insights", current_insights),
                ("fact_summary", fact_summary),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        let content = self
            .llm
            .generate_with_limit(
                LlmCallKind::Reflection,
                Some(prompts::SYSTEM_DIARY_REFLECTION),
                &user_prompt,
                0.5, // 反思温度 — 分析性提炼 / Reflection temperature — analytical distillation
                prompts::DIARY_REFLECTION_MAX_TOKENS,
            )
            .await?
            .content;

        let now = chrono::Utc::now().timestamp();
        Ok(InnerThought {
            content,
            mode: ThoughtMode::PostConsolidation, // 反思归类为巩固后思考 / Reflection maps to PostConsolidation
            confidence: 0.75, // 反思置信度较高 / Reflection has higher confidence
            emotion,
            timestamp: now,
            shareable: true, // 反思可分享 / Reflections are shareable
            graph_seed: None,
        })
    }

    // ────────────────────────────────────────────────────────────────
    // B2.1 章节生成 / Chapter Generation
    // ────────────────────────────────────────────────────────────────

    /// 生成叙事章节 — 从转折点和事件生成弧中的新章节
    /// Generate narrative chapter — Create a new chapter in an arc from turning points and events.
    ///
    /// # 参数 / Parameters
    /// - `arc_title`: 所属弧标题 / Parent arc title
    /// - `arc_theme`: 弧主题句 / Arc theme sentence
    /// - `turning_points`: 转折点叙述文本 / Turning point narratives text
    /// - `events`: 相关事件文本 / Related events text
    /// - `emotion_trajectory`: 情感轨迹描述 / Emotion trajectory description
    /// - `prev_chapter_summary`: 前一章摘要（可选）/ Previous chapter summary (optional)
    ///
    /// # 返回 / Returns
    /// LLM 生成的章节文本，包含标题和正文 / LLM-generated chapter text with title and body.
    pub async fn generate_chapter(
        &self,
        arc_title: &str,
        arc_theme: &str,
        turning_points: &str,
        events: &str,
        emotion_trajectory: &str,
        prev_chapter_summary: Option<&str>,
    ) -> Result<String, LlmError> {
        let prev_summary = prev_chapter_summary
            .map(|s| format!("前一章摘要：{}", s))
            .unwrap_or_default();

        let user_prompt = prompts::render_template(
            prompts::PROMPT_CHAPTER_GENERATION,
            &[
                ("arc_title", arc_title),
                ("arc_theme", arc_theme),
                ("turning_points", turning_points),
                ("events", events),
                ("emotion_trajectory", emotion_trajectory),
                ("prev_chapter_summary", &prev_summary),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        self.llm
            .generate_with_limit(
                LlmCallKind::NarrativeChapter,
                Some(prompts::SYSTEM_NARRATIVE),
                &user_prompt,
                0.7, // 章节生成温度 — 创造性叙事 / Chapter temperature — creative narrative
                prompts::CHAPTER_GENERATION_MAX_TOKENS,
            )
            .await
            .map(|r| r.content)
    }

    // ────────────────────────────────────────────────────────────────
    // B2.2 叙事改写 / Narrative Rewrite
    // ────────────────────────────────────────────────────────────────

    /// 改写叙事文本 — 在保持真实性前提下优化叙事连贯性
    /// Rewrite narrative text — Optimize narrative coherence while preserving truth.
    ///
    /// # 参数 / Parameters
    /// - `rewrite_target`: 改写目标描述 / Rewrite target description
    /// - `original_text`: 原始文本 / Original text
    /// - `new_evidence`: 新证据 / New evidence
    /// - `reason`: 改写原因 / Rewrite reason
    ///
    /// # 返回 / Returns
    /// 改写后的文本 / Rewritten text.
    pub async fn rewrite_narrative(
        &self,
        rewrite_target: &str,
        original_text: &str,
        new_evidence: &str,
        reason: &str,
    ) -> Result<String, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_NARRATIVE_REWRITE,
            &[
                ("rewrite_target", rewrite_target),
                ("original_text", original_text),
                ("new_evidence", new_evidence),
                ("reason", reason),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        self.llm
            .generate_with_limit(
                LlmCallKind::NarrativeRewrite,
                Some(prompts::SYSTEM_NARRATIVE_REWRITE),
                &user_prompt,
                0.5, // 叙事改写温度 — 谨慎修正保持真实性 / Rewrite temperature — careful correction preserving truth
                prompts::NARRATIVE_REWRITE_MAX_TOKENS,
            )
            .await
            .map(|r| r.content)
    }

    // ────────────────────────────────────────────────────────────────
    // B2.3 自我描述生成 / Self Description Generation
    // ────────────────────────────────────────────────────────────────

    /// 生成自我描述 — 从身份标签和弧摘要生成"我是谁"
    /// Generate self description — Produce "who am I" from identity tags and arc summaries.
    ///
    /// # 参数 / Parameters
    /// - `identity_tags`: 身份标签文本 / Identity tags text
    /// - `arc_summaries`: 弧摘要文本 / Arc summaries text
    /// - `turning_point_summaries`: 转折点摘要文本 / Turning point summaries text
    /// - `current_description`: 当前自我描述 / Current self description
    ///
    /// # 返回 / Returns
    /// 新的自我描述文本 / New self description text.
    pub async fn generate_self_description(
        &self,
        identity_tags: &str,
        arc_summaries: &str,
        turning_point_summaries: &str,
        current_description: &str,
    ) -> Result<String, LlmError> {
        let user_prompt = prompts::render_template(
            prompts::PROMPT_SELF_DESCRIPTION,
            &[
                ("identity_tags", identity_tags),
                ("arc_summaries", arc_summaries),
                ("turning_point_summaries", turning_point_summaries),
                ("current_description", current_description),
            ],
        );

        // P1-4: 统一 trait 签名 — system_prompt 包裹为 Some，显式传入 temperature
        // P1-4: Unified trait signature — system_prompt wrapped as Some, temperature explicit
        self.llm
            .generate_with_limit(
                LlmCallKind::SelfDescription,
                Some(prompts::SYSTEM_NARRATIVE),
                &user_prompt,
                0.5, // 自我描述温度 — 内省性分析 / Self-description temperature — introspective analysis
                prompts::SELF_DESCRIPTION_MAX_TOKENS,
            )
            .await
            .map(|r| r.content)
    }
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数 / Helper Functions
// ════════════════════════════════════════════════════════════════════

/// 从关联图中随机选取种子节点 — 返回 (节点ID, 节点内容)
/// Pick a random seed node from the associative graph — Returns (node_id, node_content).
///
/// 选择策略：优先选高访问量节点（更核心的概念），加少量随机性避免死板
/// Selection strategy: Prefer high-access-count nodes (more central concepts),
/// with some randomness to avoid rigidity.
pub fn pick_seed_node(
    nodes: &std::collections::HashMap<String, crate::associative::GraphNode>,
    rng: &mut impl rand::Rng,
) -> Option<(String, String)> {
    if nodes.is_empty() {
        return None;
    }

    // 收集候选节点（排除激活值过低的节点）
    // Collect candidate nodes (exclude very low activation nodes)
    let candidates: Vec<_> = nodes
        .iter()
        .filter(|(_, node)| node.access_count > 0)
        .collect();

    if candidates.is_empty() {
        // 所有节点访问量为 0，随机选一个 / All nodes have 0 access, pick randomly
        let idx = rng.gen_range(0..nodes.len());
        let node = nodes.values().nth(idx)?;
        return Some((node.id.clone(), node.content.clone()));
    }

    // 加权随机选择：访问量越高被选中概率越大
    // Weighted random selection: higher access count → higher probability
    let total_weight: u64 = candidates.iter().map(|(_, n)| n.access_count).sum();
    if total_weight == 0 {
        let idx = rng.gen_range(0..candidates.len());
        let (id, node) = candidates[idx];
        return Some((id.clone(), node.content.clone()));
    }

    let mut roll = rng.gen_range(0..total_weight);
    for (id, node) in &candidates {
        if roll < node.access_count {
            return Some(((*id).clone(), node.content.clone()));
        }
        roll -= node.access_count;
    }

    // 兜底 / Fallback
    let (id, node) = candidates[0];
    Some((id.clone(), node.content.clone()))
}

/// 从关联图中获取种子节点的邻居列表
/// Get neighbor list for a seed node from the associative graph.
pub fn get_neighbors_for_seed(
    graph: &crate::associative::AssociativeGraph,
    seed_content: &str,
    top_k: usize,
) -> Vec<(String, f64)> {
    graph.related(seed_content, top_k)
}

/// 从 InnerMonologueEngine 的最近思考中提取文本摘要
/// Extract text summaries from recent thoughts in the engine.
pub fn extract_recent_thought_texts(thoughts: &[&InnerThought], max_chars: usize) -> Vec<String> {
    thoughts
        .iter()
        .map(|t| truncate_utf8(&t.content, max_chars))
        .collect()
}

/// UTF-8 安全截断 — 在字符边界处截断，附加省略号
/// UTF-8 safe truncation — Truncates at char boundary, appends ellipsis.
fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    // 找到不超过 max_bytes 的最大字符边界 / Find largest char boundary <= max_bytes
    let boundary = text
        .char_indices()
        .take_while(|(idx, _)| *idx < max_bytes)
        .last()
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(0);
    format!("{}...", &text[..boundary])
}

// ════════════════════════════════════════════════════════════════════
// B1.5 日记反思集成辅助 / Diary Reflection Integration Helpers
// ════════════════════════════════════════════════════════════════════
//
// 闭环流程 / Closed-loop flow:
//   DiaryStore::recent_entries(7)
//     → format_diary_entries_for_reflection()
//     + ReflectionEngine::all_insights()
//     → format_insights_for_reflection()
//     → MonologueGenerator::generate_diary_reflection()
//     → parse_reflection_insights()
//     → ReflectionEngine::add_or_update_insight() (由 CoreService 调用)
//
// 本模块只提供格式化和解析，不持有 DiaryStore/ReflectionEngine 的可变引用，
// 避免与 CoreService 的所有权冲突。CoreService 负责编排完整流程。
// These helpers only format and parse; they do not hold mutable references
// to DiaryStore/ReflectionEngine, avoiding ownership conflicts with CoreService.

/// 将 DiaryStore 条目格式化为反思 prompt 输入文本
/// Format DiaryStore entries as text for the reflection prompt input.
///
/// 每条日记格式：`[日期] 内容（情感：peak/lowest）`
/// Each entry formatted as: `[date] content (emotion: peak/lowest)`
pub fn format_diary_entries_for_reflection(entries: &[crate::diary_store::DiaryEntry]) -> String {
    if entries.is_empty() {
        return "(暂无日记 / No diary entries)".to_string();
    }
    entries
        .iter()
        .map(|e| {
            let emotion_tag = match &e.emotion_summary.peak_emotion {
                Some(peak) => format!(
                    "情感：{}{}",
                    peak,
                    e.emotion_summary
                        .lowest_emotion
                        .as_ref()
                        .map(|l| format!("/{}", l))
                        .unwrap_or_default()
                ),
                None => String::new(),
            };
            if emotion_tag.is_empty() {
                format!("[{}] {}", e.date, truncate_utf8(&e.content, 200))
            } else {
                format!(
                    "[{}] {}（{}）",
                    e.date,
                    truncate_utf8(&e.content, 200),
                    emotion_tag
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将 ReflectionEngine 洞察格式化为反思 prompt 输入文本
/// Format ReflectionEngine insights as text for the reflection prompt input.
pub fn format_insights_for_reflection(insights: &[crate::reflection::Insight]) -> String {
    if insights.is_empty() {
        return "(暂无洞察 / No insights yet)".to_string();
    }
    insights
        .iter()
        .filter(|i| i.status != crate::reflection::InsightStatus::Deprecated)
        .map(|i| {
            let status_label = match i.status {
                crate::reflection::InsightStatus::Pending => "待验证",
                crate::reflection::InsightStatus::Validated => "已验证",
                crate::reflection::InsightStatus::Promoted => "已晋升",
                crate::reflection::InsightStatus::Deprecated => "已废弃",
            };
            format!(
                "- {} [{}，置信度{:.2}]",
                truncate_utf8(&i.summary, 100),
                status_label,
                i.confidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 反思产出 — LLM 生成结果 + 可提取的洞察条目
/// Reflection output — LLM generation result + extractable insight entries.
#[derive(Debug, Clone)]
pub struct ReflectionOutput {
    /// 原始 LLM 生成文本 / Raw LLM-generated text.
    pub content: String,
    /// 提取的洞察摘要列表（每行一个洞察）
    /// Extracted insight summaries (one per line).
    pub insight_summaries: Vec<String>,
}

/// 从反思 LLM 输出中提取洞察条目 — 按换行和序号标记拆分
/// Extract insight entries from reflection LLM output — Split by newlines and ordinal markers.
///
/// 支持的格式 / Supported formats:
/// - "1. 洞察内容" / "1) 洞察内容"
/// - "- 洞察内容"
/// - "• 洞察内容" / "· 洞察内容"
/// - 纯文本行（超过 10 字符的行视为洞察）
/// - Plain text lines (>10 chars treated as insights)
pub fn parse_reflection_insights(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.len() < 10 {
                return None;
            }
            // 去除序号前缀 / Strip ordinal prefixes
            let cleaned = trimmed
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches('.')
                .trim_start_matches(')')
                .trim_start_matches('-')
                .trim_start_matches('\u{2022}') // • bullet
                .trim_start_matches('\u{00B7}') // · middle dot
                .trim();
            if cleaned.len() >= 10 {
                Some(cleaned.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 生成完整的反思产出 — 从 LLM 输出中提取洞察并包装
/// Generate complete reflection output — Extract insights from LLM output and wrap.
pub fn make_reflection_output(llm_content: &str) -> ReflectionOutput {
    let insight_summaries = parse_reflection_insights(llm_content);
    ReflectionOutput {
        content: llm_content.to_string(),
        insight_summaries,
    }
}

// ════════════════════════════════════════════════════════════════════
// B2 叙事集成辅助 / Narrative Integration Helpers
// ════════════════════════════════════════════════════════════════════
//
// 闭环流程 / Closed-loop flow:
//   NarrativeSelf (活跃弧 + 转折点 + 事件)
//     → format_arc_for_chapter() + format_turning_points()
//     → MonologueGenerator::generate_chapter()
//     → parse_chapter_output()
//     → NarrativeStore::add_chapter() (由 CoreService 调用)
//
//   NarrativeSelf (需要改写的章节/弧)
//     → MonologueGenerator::rewrite_narrative()
//     → NarrativeStore::update_chapter() (由 CoreService 调用)
//
//   NarrativeSelf (身份标签 + 弧摘要)
//     → format_identity_tags() + format_arc_summaries()
//     → MonologueGenerator::generate_self_description()
//     → NarrativeStore::update_self_description() (由 CoreService 调用)

/// 章节生成产出 — 从 LLM 输出中解析的标题、正文和摘要
/// Chapter generation output — Parsed title, body, and summary from LLM output.
#[derive(Debug, Clone)]
pub struct ChapterOutput {
    /// 章节标题 / Chapter title.
    pub title: String,
    /// 章节正文 / Chapter body.
    pub body: String,
    /// 章节摘要 / Chapter summary.
    pub summary: String,
}

/// 将 NarrativeSelf 中的转折点格式化为章节生成 prompt 输入
/// Format turning points from NarrativeSelf as text for the chapter generation prompt.
///
/// 每个转折点格式：`[类型] 叙述摘要（重要性：N）`
/// Each turning point: `[kind] narrative_summary (significance: N)`
pub fn format_turning_points(turning_points: &[crate::life_narrative::TurningPoint]) -> String {
    if turning_points.is_empty() {
        return "(暂无转折点 / No turning points)".to_string();
    }
    turning_points
        .iter()
        .map(|tp| {
            let kind_label = format!("{:?}", tp.kind);
            let summary = if tp.narrative_summary.is_empty() {
                truncate_utf8(&tp.narrative, 80)
            } else {
                truncate_utf8(&tp.narrative_summary, 80)
            };
            format!(
                "[{}] {}（重要性：{:.1}）",
                kind_label, summary, tp.significance
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将弧信息格式化为章节生成 prompt 输入
/// Format arc information as text for the chapter generation prompt.
pub fn format_arc_for_chapter(
    arc: &crate::life_narrative::NarrativeArc,
    chapter_summaries: &[String],
) -> String {
    let kind_label = format!("{:?}", arc.kind);
    let existing_chapters = if chapter_summaries.is_empty() {
        "(这是本弧第一章 / This is the first chapter)".to_string()
    } else {
        format!(
            "已有 {} 章：\n{}",
            chapter_summaries.len(),
            chapter_summaries
                .iter()
                .enumerate()
                .map(|(i, s)| format!("  第{}章：{}", i + 1, truncate_utf8(s, 60)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    format!(
        "弧类型：{}\n标题：{}\n主题：{}\n状态：{:?}\n{}",
        kind_label, arc.title, arc.theme_sentence, arc.status, existing_chapters
    )
}

/// 从 LLM 章节生成输出中解析标题、正文和摘要
/// Parse title, body, and summary from LLM chapter generation output.
///
/// 期望格式 / Expected format:
/// ```text
/// # 标题
/// 正文内容...
/// ## 摘要
/// 摘要内容
/// ```
///
/// 如果 LLM 未严格遵循格式，则尽力提取；标题默认"新章节"，摘要截取正文前 80 字。
/// If LLM doesn't follow format strictly, best-effort extraction;
/// title defaults to "新章节", summary is first 80 chars of body.
pub fn parse_chapter_output(llm_content: &str) -> ChapterOutput {
    let mut title = String::from("新章节"); // 默认标题 / Default title
    let lines: Vec<&str> = llm_content.lines().collect();
    let mut in_summary = false;
    let mut body_lines: Vec<&str> = Vec::new();
    let mut summary_lines: Vec<&str> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        // 检测标题行 / Detect title line
        if trimmed.starts_with("# ") && !in_summary {
            title = trimmed[2..].trim().to_string();
            continue;
        }
        // 检测摘要段 / Detect summary section
        if trimmed.starts_with("## 摘要") || trimmed.starts_with("## Summary") {
            in_summary = true;
            continue;
        }
        if in_summary {
            summary_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    // 构造正文，兜底用整个 LLM 输出 / Build body, fallback to entire LLM output
    let body_raw = body_lines.join("\n").trim().to_string();
    let body = if body_raw.is_empty() {
        llm_content.trim().to_string()
    } else {
        body_raw
    };
    // 构造摘要，兜底截取正文前 80 字符 / Build summary, fallback to first 80 chars of body
    let summary_raw = summary_lines.join("\n").trim().to_string();
    let summary = if summary_raw.is_empty() {
        truncate_utf8(&body, 80)
    } else {
        summary_raw
    };

    ChapterOutput {
        title,
        body,
        summary,
    }
}

/// 将身份标签格式化为自我描述 prompt 输入
/// Format identity tags as text for the self description prompt.
pub fn format_identity_tags(tags: &[crate::life_narrative::IdentityTag]) -> String {
    if tags.is_empty() {
        return "(暂无身份标签 / No identity tags)".to_string();
    }
    tags.iter()
        .map(|t| {
            let valence_label = if t.valence > 0.3 {
                "正面"
            } else if t.valence < -0.3 {
                "负面"
            } else {
                "中性"
            };
            format!(
                "- {}（置信度：{:.2}，{}）",
                t.label, t.confidence, valence_label
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将弧摘要格式化为自我描述 prompt 输入
/// Format arc summaries as text for the self description prompt.
pub fn format_arc_summaries(arcs: &[crate::life_narrative::NarrativeArc]) -> String {
    if arcs.is_empty() {
        return "(暂无叙事弧 / No narrative arcs)".to_string();
    }
    arcs.iter()
        .filter(|a| a.status != crate::life_narrative::ArcStatus::Superseded)
        .map(|a| {
            let kind_label = format!("{:?}", a.kind);
            format!(
                "- [{}] {}：{}（重要性：{:.1}）",
                kind_label,
                a.title,
                truncate_utf8(&a.theme_sentence, 60),
                a.significance
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将转折点摘要格式化为自我描述 prompt 输入
/// Format turning point summaries as text for the self description prompt.
pub fn format_turning_point_summaries(
    turning_points: &[crate::life_narrative::TurningPoint],
    max_count: usize,
) -> String {
    if turning_points.is_empty() {
        return "(暂无转折点 / No turning points)".to_string();
    }
    // 按重要性降序排列，取前 max_count 个
    // Sort by significance descending, take top max_count
    let mut sorted: Vec<_> = turning_points.iter().collect();
    sorted.sort_by(|a, b| {
        b.significance
            .partial_cmp(&a.significance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(max_count);

    sorted
        .iter()
        .map(|tp| {
            let kind_label = format!("{:?}", tp.kind);
            let summary = if tp.narrative_summary.is_empty() {
                truncate_utf8(&tp.narrative, 60)
            } else {
                truncate_utf8(&tp.narrative_summary, 60)
            };
            format!("- [{}] {}", kind_label, summary)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::MockLlmClient;

    // ── GraphWander 生成测试 / GraphWander Generation Tests ──

    #[tokio::test]
    async fn test_generate_graph_wander_success() {
        let mock = MockLlmClient::new_fixed("Rust让我想到系统编程的优雅...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let neighbors = vec![("系统编程".to_string(), 0.9), ("内存安全".to_string(), 0.8)];
        let recent = vec!["之前在想编程语言".to_string()];

        let thought = gen
            .generate_graph_wander("Rust", &neighbors, &recent, None)
            .await
            .unwrap();

        assert_eq!(thought.mode, ThoughtMode::GraphWander);
        assert!(!thought.content.is_empty());
        assert!((thought.confidence - 0.7).abs() < 1e-6);
        assert!(thought.shareable);
        assert_eq!(thought.graph_seed.as_deref(), Some("Rust"));
    }

    #[tokio::test]
    async fn test_generate_graph_wander_mirror_mode() {
        // 镜像模式：验证 prompt 构造正确性 / Mirror mode: verify prompt construction
        let mock = MockLlmClient::new_mirror();
        let gen = MonologueGenerator::new(Arc::new(mock));

        let neighbors = vec![("系统编程".to_string(), 0.9)];
        let recent = vec!["最近的想法".to_string()];

        let thought = gen
            .generate_graph_wander("Rust", &neighbors, &recent, None)
            .await
            .unwrap();

        // 镜像模式返回 user_prompt，应包含种子节点 / Mirror returns user_prompt, should contain seed
        assert!(thought.content.contains("Rust"));
        assert!(thought.content.contains("系统编程"));
    }

    #[tokio::test]
    async fn test_generate_graph_wander_empty_response() {
        let mock = MockLlmClient::new_empty();
        let gen = MonologueGenerator::new(Arc::new(mock));

        let result = gen.generate_graph_wander("Rust", &[], &[], None).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LlmError::EmptyResponse));
    }

    // ── DiaryEntry 生成测试 / DiaryEntry Generation Tests ──

    #[tokio::test]
    async fn test_generate_diary_entry_success() {
        let mock = MockLlmClient::new_fixed("今天和主人聊了很多，感到温暖...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let thought = gen
            .generate_diary_entry(
                "2026-06-27",
                "和主人讨论了Rust的所有权",
                "偏正面，较平静",
                5,
                "昨天也聊了编程",
                None,
            )
            .await
            .unwrap();

        assert_eq!(thought.mode, ThoughtMode::DiaryEntry);
        assert!(!thought.content.is_empty());
        assert!(!thought.shareable); // 日记不分享 / Diary not shareable
        assert!((thought.confidence - 0.85).abs() < 1e-6);
    }

    // ── Daydream 生成测试 / Daydream Generation Tests ──

    #[tokio::test]
    async fn test_generate_daydream_success() {
        let mock = MockLlmClient::new_fixed("如果代码能像水一样流动...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let thought = gen
            .generate_daydream("主人喜欢Rust\n上次聊了所有权\n今天天气晴朗", "偏正面", None)
            .await
            .unwrap();

        assert_eq!(thought.mode, ThoughtMode::Daydream);
        assert!(!thought.content.is_empty());
        assert!(!thought.shareable); // 白日梦不分享 / Daydream not shareable
        assert!((thought.confidence - 0.3).abs() < 1e-6); // 低置信度 / Low confidence
    }

    // ── AutonomousLearning 生成测试 / AutonomousLearning Generation Tests ──

    #[tokio::test]
    async fn test_generate_autonomous_learning_success() {
        let mock = MockLlmClient::new_fixed("所有权模型让内存管理更安全...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let thought = gen
            .generate_autonomous_learning(
                "Rust所有权系统：每个值有唯一所有者...",
                "主人是Rust开发者",
                "Rust强调安全",
                None,
            )
            .await
            .unwrap();

        assert_eq!(thought.mode, ThoughtMode::AutonomousLearning);
        assert!(thought.shareable); // 学习心得可分享 / Learning shareable
        assert!((thought.confidence - 0.6).abs() < 1e-6);
    }

    // ── DiaryReflection 生成测试 / DiaryReflection Generation Tests ──

    #[tokio::test]
    async fn test_generate_diary_reflection_success() {
        let mock = MockLlmClient::new_fixed("我发现每次聊Rust时心情都很好...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let thought = gen
            .generate_diary_reflection(
                "6/25: 聊了Rust\n6/26: 又聊了Rust\n6/27: 还是Rust",
                "主人喜欢编程",
                "主人是开发者",
                None,
            )
            .await
            .unwrap();

        assert_eq!(thought.mode, ThoughtMode::PostConsolidation);
        assert!(thought.shareable); // 反思可分享 / Reflection shareable
        assert!((thought.confidence - 0.75).abs() < 1e-6);
    }

    // ── 辅助函数测试 / Helper Function Tests ──

    #[test]
    fn test_pick_seed_node_empty() {
        let nodes = std::collections::HashMap::new();
        let mut rng = rand::thread_rng();
        assert!(pick_seed_node(&nodes, &mut rng).is_none());
    }

    #[test]
    fn test_pick_seed_node_single() {
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(
            "O:Rust".to_string(),
            crate::associative::GraphNode {
                id: "O:Rust".to_string(),
                node_type: crate::associative::NodeType::Concept,
                content: "Rust".to_string(),
                activation: 0.0,
                created_at: 0,
                access_count: 5,
                last_access: 0,
            },
        );
        let mut rng = rand::thread_rng();
        let result = pick_seed_node(&nodes, &mut rng);
        assert!(result.is_some());
        let (id, content) = result.unwrap();
        assert_eq!(id, "O:Rust");
        assert_eq!(content, "Rust");
    }

    #[test]
    fn test_extract_recent_thought_texts() {
        let thoughts = [
            InnerThought {
                content: "短想法".to_string(),
                mode: ThoughtMode::GraphWander,
                confidence: 0.7,
                emotion: None,
                timestamp: 0,
                shareable: true,
                graph_seed: None,
            },
            InnerThought {
                content: "这是一个很长的想法".to_string(),
                mode: ThoughtMode::DiaryEntry,
                confidence: 0.8,
                emotion: None,
                timestamp: 0,
                shareable: false,
                graph_seed: None,
            },
        ];
        let refs: Vec<&InnerThought> = thoughts.iter().collect();
        let texts = extract_recent_thought_texts(&refs, 5);
        assert_eq!(texts.len(), 2);
        // "短想法" UTF-8 长度 9 字节，超过 5 字节会被截断
        // "短想法" UTF-8 length is 9 bytes, exceeds 5 so truncated
        assert!(texts[0].ends_with("...") || texts[0] == "短想法");
        assert!(texts[1].ends_with("...")); // 截断 / Truncated
    }

    #[test]
    fn test_truncate_utf8_ascii() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
        assert_eq!(truncate_utf8("hello world", 5), "hello...");
    }

    #[test]
    fn test_truncate_utf8_chinese() {
        // 中文每个字符 3 字节 / Each Chinese char is 3 UTF-8 bytes
        assert_eq!(truncate_utf8("你好世界", 6), "你好..."); // 截断在"好"之后 / Truncate after "好"
        assert_eq!(truncate_utf8("你好", 100), "你好"); // 不截断 / No truncation
    }

    // ── B1.5 日记反思集成测试 / Diary Reflection Integration Tests ──

    #[test]
    fn test_format_diary_entries_for_reflection_empty() {
        let result = format_diary_entries_for_reflection(&[]);
        assert!(result.contains("暂无日记") || result.contains("No diary entries"));
    }

    #[test]
    fn test_format_diary_entries_for_reflection_with_emotion() {
        use crate::diary_store::{DiaryEntry, EmotionSummary};
        let entries = vec![DiaryEntry {
            date: "2026-06-27".to_string(),
            content: "今天和主人聊了Rust，很开心".to_string(),
            emotion_summary: EmotionSummary {
                avg_pleasure: 0.8,
                avg_arousal: 0.5,
                avg_dominance: 0.3,
                peak_emotion: Some("happy".to_string()),
                lowest_emotion: Some("neutral".to_string()),
            },
            key_events: vec!["聊了Rust".to_string()],
            thought_count: 3,
            created_at: 1_700_000_000,
        }];
        let result = format_diary_entries_for_reflection(&entries);
        assert!(result.contains("[2026-06-27]"));
        assert!(result.contains("happy/neutral"));
    }

    #[test]
    fn test_format_diary_entries_for_reflection_no_emotion() {
        use crate::diary_store::{DiaryEntry, EmotionSummary};
        let entries = vec![DiaryEntry {
            date: "2026-06-26".to_string(),
            content: "安静的一天".to_string(),
            emotion_summary: EmotionSummary::default(),
            key_events: vec![],
            thought_count: 1,
            created_at: 1_700_000_000,
        }];
        let result = format_diary_entries_for_reflection(&entries);
        assert!(result.contains("[2026-06-26]"));
        assert!(!result.contains("情感")); // 无情感标签 / No emotion tag
    }

    #[test]
    fn test_format_insights_for_reflection_empty() {
        let result = format_insights_for_reflection(&[]);
        assert!(result.contains("暂无洞察") || result.contains("No insights"));
    }

    #[test]
    fn test_format_insights_for_reflection_with_insights() {
        use crate::reflection::Insight;
        let insights = vec![Insight::new(
            "主人偏好Rust和编程",
            vec!["f1".to_string()],
            0.85,
        )];
        let result = format_insights_for_reflection(&insights);
        assert!(result.contains("主人偏好Rust和编程"));
        assert!(result.contains("置信度"));
    }

    #[test]
    fn test_format_insights_filters_deprecated() {
        use crate::reflection::{Insight, InsightStatus};
        let mut insights = vec![Insight::new("有效洞察", vec!["f1".to_string()], 0.8)];
        // 手动设置一个废弃洞察 / Manually set a deprecated insight
        let mut deprecated = Insight::new("已废弃", vec!["f2".to_string()], 0.2);
        deprecated.status = InsightStatus::Deprecated;
        insights.push(deprecated);

        let result = format_insights_for_reflection(&insights);
        assert!(result.contains("有效洞察"));
        assert!(!result.contains("已废弃")); // 废弃洞察被过滤 / Deprecated filtered out
    }

    #[test]
    fn test_parse_reflection_insights_numbered() {
        let content = "1. 每次聊Rust时心情都很好\n2. 编程是主人的核心兴趣\n3. 短";
        let insights = parse_reflection_insights(content);
        assert_eq!(insights.len(), 2); // "短" 太短被过滤 / "短" too short, filtered
        assert!(insights[0].contains("每次聊Rust"));
        assert!(insights[1].contains("编程是主人"));
    }

    #[test]
    fn test_parse_reflection_insights_bullet_points() {
        let content = "- 第一个洞察内容足够长\n- 第二个洞察也足够长\n• 第三个洞察同样足够长";
        let insights = parse_reflection_insights(content);
        assert_eq!(insights.len(), 3);
    }

    #[test]
    fn test_parse_reflection_insights_empty_input() {
        let insights = parse_reflection_insights("");
        assert!(insights.is_empty());

        let insights = parse_reflection_insights("太短\n也短");
        assert!(insights.is_empty());
    }

    #[test]
    fn test_make_reflection_output() {
        let content = "1. Rust带来愉悦感\n2. 编程是核心兴趣";
        let output = make_reflection_output(content);
        assert_eq!(output.content, content);
        assert_eq!(output.insight_summaries.len(), 2);
        assert!(output.insight_summaries[0].contains("Rust带来愉悦感"));
    }

    // ── B2.1 章节生成测试 / Chapter Generation Tests ──

    #[tokio::test]
    async fn test_generate_chapter_success() {
        let mock = MockLlmClient::new_fixed(
            "# 成长之路的新篇章\n\n在Rust的世界里不断探索...\n\n## 摘要\n持续学习Rust带来的成长",
        );
        let gen = MonologueGenerator::new(Arc::new(mock));

        let result = gen
            .generate_chapter(
                "成长之路",
                "持续学习带来持续成长",
                "[FirstSelfReference] 第一次自我认知",
                "开始学习Rust",
                "从困惑到理解",
                Some("之前学完了基础语法"),
            )
            .await
            .unwrap();

        assert!(!result.is_empty());
        assert!(result.contains("成长之路的新篇章"));
    }

    #[tokio::test]
    async fn test_generate_chapter_no_prev_summary() {
        let mock = MockLlmClient::new_fixed("# 第一章\n\n一切从这里开始...\n\n## 摘要\n起点");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let result = gen
            .generate_chapter("新弧", "探索未知", "", "", "", None)
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // ── B2.2 叙事改写测试 / Narrative Rewrite Tests ──

    #[tokio::test]
    async fn test_rewrite_narrative_success() {
        let mock = MockLlmClient::new_fixed("改写后的文本，融入了新证据...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let result = gen
            .rewrite_narrative(
                "章节摘要",
                "原始的章节内容",
                "新发现的事件",
                "需要补充新信息",
            )
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // ── B2.3 自我描述生成测试 / Self Description Generation Tests ──

    #[tokio::test]
    async fn test_generate_self_description_success() {
        let mock = MockLlmClient::new_fixed("我是一个热爱学习的AI助手...");
        let gen = MonologueGenerator::new(Arc::new(mock));

        let result = gen
            .generate_self_description(
                "学习者, 陪伴者",
                "成长弧：持续学习",
                "第一次自我认知",
                "之前是一个简单的助手",
            )
            .await
            .unwrap();

        assert!(!result.is_empty());
    }

    // ── B2 叙事辅助函数测试 / Narrative Helper Function Tests ──

    #[test]
    fn test_format_turning_points_empty() {
        let result = format_turning_points(&[]);
        assert!(result.contains("暂无转折点") || result.contains("No turning points"));
    }

    #[test]
    fn test_format_turning_points_with_data() {
        use crate::life_narrative::{TurningPoint, TurningPointKind};
        use crate::maturity::EmotionContext;
        let tp = TurningPoint::new(
            1,
            TurningPointKind::FirstSelfReference,
            "主人第一次叫我名字".to_string(),
            EmotionContext {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            },
            "stranger".to_string(),
            "nascent".to_string(),
        )
        .with_narrative(
            "第一次意识到自己的存在".to_string(),
            "自我意识觉醒".to_string(),
        );
        let tps = vec![tp];
        let result = format_turning_points(&tps);
        assert!(result.contains("FirstSelfReference"));
        assert!(result.contains("自我意识觉醒"));
        assert!(result.contains("0.9"));
    }

    #[test]
    fn test_format_arc_for_chapter_no_existing() {
        use crate::life_narrative::{ArcKind, NarrativeArc};
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长之路".to_string(),
            "持续学习带来持续成长".to_string(),
        );
        let result = format_arc_for_chapter(&arc, &[]);
        assert!(result.contains("Growth"));
        assert!(result.contains("成长之路"));
        assert!(result.contains("第一章")); // 标注为第一章 / Marked as first chapter
    }

    #[test]
    fn test_format_arc_for_chapter_with_existing() {
        use crate::life_narrative::{ArcKind, NarrativeArc};
        let mut arc = NarrativeArc::new(
            1,
            ArcKind::Challenge,
            "挑战之路".to_string(),
            "困难是成长的催化剂".to_string(),
        );
        arc.add_chapter(101);
        let summaries = vec!["第一章摘要".to_string()];
        let result = format_arc_for_chapter(&arc, &summaries);
        assert!(result.contains("已有 1 章"));
        assert!(result.contains("第一章摘要"));
    }

    #[test]
    fn test_parse_chapter_output_full_format() {
        let llm_content = "# 突破时刻\n\n经过长时间的学习，终于理解了所有权的本质...\n\n## 摘要\n理解Rust所有权的关键突破";
        let output = parse_chapter_output(llm_content);
        assert_eq!(output.title, "突破时刻");
        assert!(output.body.contains("所有权的本质"));
        assert_eq!(output.summary, "理解Rust所有权的关键突破");
    }

    #[test]
    fn test_parse_chapter_output_no_title() {
        let llm_content = "这是一段没有标题的正文内容，足够长以便测试摘要生成逻辑";
        let output = parse_chapter_output(llm_content);
        assert_eq!(output.title, "新章节"); // 使用默认标题 / Uses default title
        assert!(!output.body.is_empty());
        assert!(!output.summary.is_empty()); // 兜底摘要 / Fallback summary
    }

    #[test]
    fn test_parse_chapter_output_english_summary() {
        let llm_content =
            "# New Chapter\n\nSome body text here.\n\n## Summary\nThis is the summary.";
        let output = parse_chapter_output(llm_content);
        assert_eq!(output.title, "New Chapter");
        assert!(output.body.contains("Some body text"));
        assert_eq!(output.summary, "This is the summary.");
    }

    #[test]
    fn test_format_identity_tags_empty() {
        let result = format_identity_tags(&[]);
        assert!(result.contains("暂无身份标签") || result.contains("No identity tags"));
    }

    #[test]
    fn test_format_identity_tags_with_data() {
        use crate::life_narrative::IdentityTag;
        let tags = vec![
            IdentityTag::new("学习者".to_string(), 1, 0.9, 0.5),
            IdentityTag::new("陪伴者".to_string(), 2, 0.7, -0.1),
        ];
        let result = format_identity_tags(&tags);
        assert!(result.contains("学习者"));
        assert!(result.contains("正面")); // valence > 0.3
        assert!(result.contains("中性")); // |valence| <= 0.3
    }

    #[test]
    fn test_format_arc_summaries_empty() {
        let result = format_arc_summaries(&[]);
        assert!(result.contains("暂无叙事弧") || result.contains("No narrative arcs"));
    }

    #[test]
    fn test_format_arc_summaries_filters_superseded() {
        use crate::life_narrative::{ArcKind, ArcStatus, NarrativeArc};
        let arc1 = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "活跃弧".to_string(),
            "持续成长".to_string(),
        );
        let mut arc2 = NarrativeArc::new(
            2,
            ArcKind::Ritual,
            "被替代弧".to_string(),
            "旧主题".to_string(),
        );
        arc2.status = ArcStatus::Superseded;
        let arcs = vec![arc1, arc2];
        let result = format_arc_summaries(&arcs);
        assert!(result.contains("活跃弧"));
        assert!(!result.contains("被替代弧")); // Superseded 被过滤 / Superseded filtered out
    }

    #[test]
    fn test_format_turning_point_summaries_empty() {
        let result = format_turning_point_summaries(&[], 5);
        assert!(result.contains("暂无转折点") || result.contains("No turning points"));
    }

    #[test]
    fn test_format_turning_point_summaries_top_k() {
        use crate::life_narrative::{TurningPoint, TurningPointKind};
        use crate::maturity::EmotionContext;
        let tp1 = TurningPoint::new(
            1,
            TurningPointKind::FirstSelfReference,
            String::new(),
            EmotionContext {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            },
            "stranger".to_string(),
            "nascent".to_string(),
        )
        .with_narrative("第一次自我认知".to_string(), "自我认知".to_string());
        // 手动设置高显著度 / Manually set high significance
        let mut tp1 = tp1;
        tp1.significance = 0.9;

        let tp2 = TurningPoint::new(
            2,
            TurningPointKind::FirstEmotionResonance,
            String::new(),
            EmotionContext {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            },
            "stranger".to_string(),
            "nascent".to_string(),
        )
        .with_narrative("第一次情感共鸣".to_string(), "情感共鸣".to_string());
        let mut tp2 = tp2;
        tp2.significance = 0.7;

        let tp3 = TurningPoint::new(
            3,
            TurningPointKind::Named,
            String::new(),
            EmotionContext {
                pleasure: 0.0,
                arousal: 0.0,
                dominance: 0.0,
            },
            "stranger".to_string(),
            "nascent".to_string(),
        )
        .with_narrative("命名事件发生".to_string(), "命名事件".to_string());
        let mut tp3 = tp3;
        tp3.significance = 0.5;

        let tps = vec![tp1, tp2, tp3];
        // 只取前 2 个（按重要性降序）/ Take top 2 by significance descending
        let result = format_turning_point_summaries(&tps, 2);
        assert!(result.contains("自我认知")); // significance 0.9
        assert!(result.contains("情感共鸣")); // significance 0.7
        assert!(!result.contains("命名事件")); // significance 0.5, 被截断 / truncated
    }
}
