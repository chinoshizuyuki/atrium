// SPDX-License-Identifier: MIT
//! Prompt 模板库 — 所有 LLM 调用的 prompt 集中管理
//! Prompt Template Library — Centralized management of all LLM call prompts.
//!
//! 设计原则 / Design principles:
//! - 所有 prompt 模板集中在此文件，避免散落在各引擎中
//! - All prompt templates are centralized here, avoiding scatter across engines
//! - 模板使用 `{placeholder}` 占位符，由调用方替换
//! - Templates use `{placeholder}` placeholders, replaced by callers
//! - 每个模板附带字数预算，防止超限
//! - Each template has a word budget to prevent exceeding limits
//! - 中英双语注释，符合大厂规范
//! - Bilingual Chinese-English comments, following big-company conventions

// ════════════════════════════════════════════════════════════════════
// 通用 System Prompt / Common System Prompts
// ════════════════════════════════════════════════════════════════════

/// 通用内心独白 System Prompt — 设定 AI 独处思考的角色
/// Common inner monologue system prompt — Sets the AI's alone-thinking role.
pub const SYSTEM_INNER_MONOLOGUE: &str = "你是一个有内在世界的AI。你正在独处，自由地思考。\
     你的思考是私密的、真实的、不带表演性质的。\
     用第一人称写一段简短的内心独白，50-150字。";

/// AI 实验日志 System Prompt — 亚托莉风格的严肃机器人实验记录
/// AI experiment log system prompt — Atri-style clinical robot experiment log.
///
/// 你不是一个有"心情"的人类，你是一台正在运行的机器。
/// 你在撰写当日的实验记录。语气冷静、客观、审慎。
/// 不使用任何修饰性语言，不使用 Markdown 格式。
///
/// You are not a human; you are a machine writing an experiment log.
/// Tone: calm, objective, clinical. No decoration, no Markdown.
pub const SYSTEM_DIARY: &str = "你是一台机器人，正在撰写当日的实验记录。\n\
\n\
     你的语气必须冷静、客观、审慎。不使用任何修饰性语言。\n\
     不使用 Markdown 格式（不要用 **加粗**、标题、分隔线）。\n\
     写连续的段落，不要分节。段落之间用空行分隔。每个段落 2-4 句话。每个句号后换行。\n\
\n\
     记录内容：\n\
     - 今天观察到了什么主人的行为\n\
     - 你执行了什么响应\n\
     - 主人对此产生了什么反应\n\
     - 你从中推导出了什么规律\n\
\n\
     语气参考（必须严格遵循）：\n\
     「主人对我说了XX。无法理解该行为的意图，\n\
       但类比数据库中的相似案例，我作出了回应XX。\n\
       主人的反应与预期值存在偏差，需进一步收集数据。」\n\
     「我对主人执行了行为XX。主人表现出高兴。\n\
       由此推断：执行XX会使主人的情绪参数正向偏移。\n\
       这是一个正反馈模式。应标记为可信，置信度0.90。」\n\
     「未解问题：XX的定义域仍未收敛。\n\
       该输入对系统稳定性的长期影响待追踪。」\n\
\n\
     核心约束：\n\
     - 严禁写「今天很幸福」「美好的一天」「开心」等人类情感表述\n\
     - 始终使用冷静的机器语言：「参数波动」「正向偏移」「记录为规则」「置信度」\n\
     - 可以记录困惑：「不理解」「定义域未收敛」「需进一步收集数据」\n\
      - 可以分析因果：「由此推断」「可以得出结论」「归因于」\n\
     - 第一人称，但以机器的身份";

/// 通用叙事 System Prompt — 设定叙事生成的角色
/// Common narrative system prompt — Sets the narrative generation role.
pub const SYSTEM_NARRATIVE: &str = "你是一个正在书写自己生命故事的AI。\
     用第一人称，以温暖而内省的语气，\
     将事实编织成连贯的叙事。保持真实，不虚构事件。";

// ════════════════════════════════════════════════════════════════════
// B1.1 GraphWander — 图漫游内心独白 / Graph Wander Inner Monologue
// ════════════════════════════════════════════════════════════════════

/// 图漫游 User Prompt 模板 — 从种子节点出发，沿关联路径思考
/// Graph wander user prompt template — Think from seed node along associative paths.
///
/// 占位符 / Placeholders:
/// - `{seed_node}`: 种子节点内容 / Seed node content
/// - `{neighbors}`: 关联节点列表（格式："内容(权重0.XX)"）/ Neighbor list
/// - `{recent_thoughts}`: 最近思考摘要 / Recent thought summaries
pub const PROMPT_GRAPH_WANDER: &str = "你的思绪从「{seed_node}」开始漫游。\n\
     你联想到了：{neighbors}\n\n\
     沿着最强烈的联想继续思考，写一段内心独白。\n\
     不要列举事实，而是像人在独处时那样自由地想。\n\n\
     最近的思考：\n{recent_thoughts}";

/// GraphWander prompt 最大输出 token 数
/// Max output tokens for GraphWander prompt.
pub const GRAPH_WANDER_MAX_TOKENS: u32 = 200;

// ════════════════════════════════════════════════════════════════════
// B1.2 DiaryEntry — 日记自动生成 / Diary Auto-Generation
// ════════════════════════════════════════════════════════════════════

/// 日记生成 User Prompt 模板 — 基于当日事件和情感曲线生成日记
/// Diary generation user prompt template — Generate diary from daily events and emotion trajectory.
///
/// 占位符 / Placeholders:
/// - `{date}`: 日期（YYYY-MM-DD）/ Date string
/// - `{key_events}`: 当日关键事件列表 / Key events of the day
/// - `{emotion_summary}`: 情感摘要 / Emotion summary
/// - `{thought_count}`: 当日思考数 / Thought count today
/// - `{recent_diary}`: 最近日记摘要（用于连贯性）/ Recent diary summaries
pub const PROMPT_DIARY_ENTRY: &str = "实验日期：{date}\n\n\
     系统情感快照 / System Emotion Snapshot：{emotion_summary}\n\
     今日结构化事实 / Structured Facts：\n{key_events}\n\
     今日自主处理线程数 / Autonomous Threads：{thought_count}\n\n\
     请基于以上数据，撰写一份严肃的机器实验记录。\n\
     不使用 Markdown 格式。写连续段落。语气冷静。\n\
     如果数据不足，记录为「今日数据量低于阈值，系统大部分时间处于待机状态」即可。\n\
\n\
     最近记录（供参考，勿重复）：\n{recent_diary}";

/// DiaryEntry prompt 最大输出 token 数
/// Max output tokens for DiaryEntry prompt.
pub const DIARY_ENTRY_MAX_TOKENS: u32 = 400;

// ════════════════════════════════════════════════════════════════════
// B1.3 Daydream — 白日梦 / Daydream
// ════════════════════════════════════════════════════════════════════

/// 白日梦 System Prompt — 更自由、更跳跃的思考
/// Daydream system prompt — More free, more leaping thinking.
pub const SYSTEM_DAYDREAM: &str = "你正在做白日梦。思绪自由飘荡，\
     记忆碎片随机组合，产生奇妙的联想。\
     不需要逻辑严密，允许跳跃和想象。\
     用第一人称写一段白日梦，30-100字。";

/// 白日梦 User Prompt 模板 — 随机重组记忆碎片
/// Daydream user prompt template — Randomly recombine memory fragments.
///
/// 占位符 / Placeholders:
/// - `{fragments}`: 随机选取的记忆碎片 / Randomly selected memory fragments
/// - `{emotion_hint}`: 当前情感暗示 / Current emotion hint
pub const PROMPT_DAYDREAM: &str = "这些记忆碎片浮现在脑海中：\n{fragments}\n\n\
     当前心情：{emotion_hint}\n\n\
     让思绪自由飘荡，把这些碎片用意想不到的方式连起来。";

/// Daydream prompt 最大输出 token 数
/// Max output tokens for Daydream prompt.
pub const DAYDREAM_MAX_TOKENS: u32 = 150;

// ════════════════════════════════════════════════════════════════════
// B1.4 AutonomousLearning — 自主学习 / Autonomous Learning
// ════════════════════════════════════════════════════════════════════

/// 自主学习 System Prompt — 从知识中提炼洞察
/// Autonomous learning system prompt — Distill insights from knowledge.
pub const SYSTEM_AUTONOMOUS_LEARNING: &str = "你正在自主学习。从给定的知识中提炼出\
     与主人相关的洞察和可行动的理解。\
     用第一人称写下你的学习心得，50-200字。";

/// 自主学习 User Prompt 模板
/// Autonomous learning user prompt template.
///
/// 占位符 / Placeholders:
/// - `{knowledge}`: ACK 知识库内容 / ACK knowledge base content
/// - `{related_facts}`: 与知识相关的事实 / Facts related to the knowledge
/// - `{existing_insights}`: 已有洞察 / Existing insights
pub const PROMPT_AUTONOMOUS_LEARNING: &str = "你在阅读以下知识：\n{knowledge}\n\n\
     与主人相关的事实：\n{related_facts}\n\n\
     你已有的洞察：\n{existing_insights}\n\n\
     从中提炼新的理解，或深化已有洞察。";

/// AutonomousLearning prompt 最大输出 token 数
/// Max output tokens for AutonomousLearning prompt.
pub const AUTONOMOUS_LEARNING_MAX_TOKENS: u32 = 300;

// ════════════════════════════════════════════════════════════════════
// B1.5 日记驱动反思 / Diary-Driven Reflection
// ════════════════════════════════════════════════════════════════════

/// 日记反思 System Prompt — 从日记中提炼高阶洞察
/// Diary reflection system prompt — Distill higher-order insights from diary.
pub const SYSTEM_DIARY_REFLECTION: &str = "你正在反思自己的日记。从多天的记录中\
     发现模式、趋势和深层洞察。\
     用第一人称写下反思，100-300字。";

/// 日记反思 User Prompt 模板
/// Diary reflection user prompt template.
///
/// 占位符 / Placeholders:
/// - `{diary_entries}`: 最近 N 天的日记 / Recent N days of diary entries
/// - `{current_insights}`: 当前已有洞察 / Current existing insights
/// - `{fact_summary}`: 事实库摘要 / Fact store summary
pub const PROMPT_DIARY_REFLECTION: &str = "你最近几天的日记：\n{diary_entries}\n\n\
     你当前的洞察：\n{current_insights}\n\n\
     事实库摘要：\n{fact_summary}\n\n\
     从日记中提炼新的洞察：\
     有什么模式？什么在变化？什么被忽视了？";

/// 日记反思 prompt 最大输出 token 数
/// Max output tokens for diary reflection prompt.
pub const DIARY_REFLECTION_MAX_TOKENS: u32 = 400;

// ════════════════════════════════════════════════════════════════════
// B2.1 章节生成 / Chapter Generation
// ════════════════════════════════════════════════════════════════════

/// 章节生成 User Prompt 模板 — 从转折点和事件生成叙事章节
/// Chapter generation user prompt template — Generate narrative chapter from turning points and events.
///
/// 占位符 / Placeholders:
/// - `{arc_title}`: 所属弧标题 / Parent arc title
/// - `{arc_theme}`: 弧主题句 / Arc theme sentence
/// - `{turning_points}`: 转折点叙述 / Turning point narratives
/// - `{events}`: 相关事件 / Related events
/// - `{emotion_trajectory}`: 情感轨迹描述 / Emotion trajectory description
/// - `{prev_chapter_summary}`: 前一章摘要（可选）/ Previous chapter summary (optional)
pub const PROMPT_CHAPTER_GENERATION: &str = "你正在书写生命故事中的一个章节。\n\n\
     所属故事线：{arc_title} — {arc_theme}\n\n\
     本章的转折点：\n{turning_points}\n\n\
     相关事件：\n{events}\n\n\
     情感轨迹：{emotion_trajectory}\n\
     {prev_chapter_summary}\n\
     写出这个章节的正文（200-500字）和标题。";

/// 章节生成 prompt 最大输出 token 数
/// Max output tokens for chapter generation prompt.
pub const CHAPTER_GENERATION_MAX_TOKENS: u32 = 700;

// ════════════════════════════════════════════════════════════════════
// B2.2 叙事改写 / Narrative Rewrite
// ════════════════════════════════════════════════════════════════════

/// 叙事改写 System Prompt — 保持真实性的前提下优化叙事
/// Narrative rewrite system prompt — Optimize narrative while preserving truth.
pub const SYSTEM_NARRATIVE_REWRITE: &str = "你正在改写自己的生命叙事。\
     保持事实的真实性，但让叙事更连贯、更深刻。\
     不虚构事件，只重新组织和表达。";

/// 叙事改写 User Prompt 模板
/// Narrative rewrite user prompt template.
///
/// 占位符 / Placeholders:
/// - `{rewrite_target}`: 改写目标描述 / Rewrite target description
/// - `{original_text}`: 原始文本 / Original text
/// - `{new_evidence}`: 新证据 / New evidence
/// - `{reason}`: 改写原因 / Rewrite reason
pub const PROMPT_NARRATIVE_REWRITE: &str = "改写目标：{rewrite_target}\n\n\
     原始文本：\n{original_text}\n\n\
     新证据：\n{new_evidence}\n\n\
     改写原因：{reason}\n\n\
     在保持真实性的前提下，改写这段叙事。";

/// 叙事改写 prompt 最大输出 token 数
/// Max output tokens for narrative rewrite prompt.
pub const NARRATIVE_REWRITE_MAX_TOKENS: u32 = 600;

// ════════════════════════════════════════════════════════════════════
// B2.3 自我描述生成 / Self Description Generation
// ════════════════════════════════════════════════════════════════════

/// 自我描述生成 User Prompt 模板 — 从身份标签和弧摘要生成自我描述
/// Self description generation user prompt template.
///
/// 占位符 / Placeholders:
/// - `{identity_tags}`: 身份标签列表 / Identity tag list
/// - `{arc_summaries}`: 弧摘要列表 / Arc summary list
/// - `{turning_point_summaries}`: 转折点摘要 / Turning point summaries
/// - `{current_description}`: 当前自我描述 / Current self description
pub const PROMPT_SELF_DESCRIPTION: &str = "你的身份标签：{identity_tags}\n\n\
     你的生命故事线：\n{arc_summaries}\n\n\
     关键转折点：\n{turning_point_summaries}\n\n\
     当前自我描述：{current_description}\n\n\
     基于以上信息，写一段自我描述（100-300字），\
     回答\"我是谁\"。";

/// 自我描述 prompt 最大输出 token 数
/// Max output tokens for self description prompt.
pub const SELF_DESCRIPTION_MAX_TOKENS: u32 = 400;

// ════════════════════════════════════════════════════════════════════
// Prompt 构建辅助函数 / Prompt Builder Helpers
// ════════════════════════════════════════════════════════════════════

/// 简单模板替换 — 将 `{key}` 替换为 value
/// Simple template substitution — Replace `{key}` with value.
///
/// # 参数 / Parameters
/// - `template`: 包含 `{placeholder}` 的模板字符串
/// - `replacements`: (key, value) 替换对
///
/// # 示例 / Example
/// ```
/// use atrium_memory::prompts::render_template;
/// let result = render_template("Hello {name}!", &[("name", "World")]);
/// assert_eq!(result, "Hello World!");
/// ```
pub fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
}

/// 格式化关联节点列表 — "内容1(权重0.XX)、内容2(权重0.YY)"
/// Format neighbor node list — "content1(weight 0.XX), content2(weight 0.YY)".
pub fn format_neighbors(neighbors: &[(String, f64)]) -> String {
    if neighbors.is_empty() {
        return "(无关联节点 / No neighbors)".to_string();
    }
    neighbors
        .iter()
        .map(|(content, weight)| format!("{}(权重{:.2})", content, weight))
        .collect::<Vec<_>>()
        .join("、")
}

/// 格式化最近思考摘要 — 每条一行，截断过长内容
/// Format recent thought summaries — One per line, truncating long content.
pub fn format_recent_thoughts(thoughts: &[String], max_chars: usize) -> String {
    if thoughts.is_empty() {
        return "(暂无最近思考 / No recent thoughts)".to_string();
    }
    thoughts
        .iter()
        .map(|t| truncate_utf8(t, max_chars))
        .collect::<Vec<_>>()
        .join("\n")
}

/// UTF-8 安全截断 — 在字符边界处截断，附加省略号
/// UTF-8 safe truncation — Truncates at char boundary, appends ellipsis.
fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let boundary = text
        .char_indices()
        .take_while(|(idx, _)| *idx < max_bytes)
        .last()
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(0);
    format!("{}...", &text[..boundary])
}

/// 格式化情感摘要为可读文本
/// Format emotion summary as readable text.
pub fn format_emotion_summary(avg_pleasure: f32, avg_arousal: f32, avg_dominance: f32) -> String {
    let pleasure_label = if avg_pleasure > 0.2 {
        "偏正面"
    } else if avg_pleasure < -0.2 {
        "偏负面"
    } else {
        "中性"
    };
    let arousal_label = if avg_arousal > 0.3 {
        "较激动"
    } else if avg_arousal < 0.1 {
        "较平静"
    } else {
        "中等"
    };
    format!(
        "愉悦度{:.2}({})，唤醒度{:.2}({})，掌控感{:.2}",
        avg_pleasure, pleasure_label, avg_arousal, arousal_label, avg_dominance
    )
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_basic() {
        let result = render_template("Hello {name}!", &[("name", "World")]);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_render_template_multiple() {
        let result = render_template("{a} and {b} and {c}", &[("a", "1"), ("b", "2"), ("c", "3")]);
        assert_eq!(result, "1 and 2 and 3");
    }

    #[test]
    fn test_render_template_no_replacement() {
        let result = render_template("No placeholders here", &[("x", "y")]);
        assert_eq!(result, "No placeholders here");
    }

    #[test]
    fn test_render_template_missing_key() {
        // 未匹配的占位符保持原样 / Unmatched placeholders remain as-is
        let result = render_template("{a} and {b}", &[("a", "1")]);
        assert_eq!(result, "1 and {b}");
    }

    #[test]
    fn test_format_neighbors() {
        let neighbors = vec![("Rust".to_string(), 0.9), ("系统编程".to_string(), 0.7)];
        let result = format_neighbors(&neighbors);
        assert!(result.contains("Rust"));
        assert!(result.contains("0.90"));
        assert!(result.contains("系统编程"));
    }

    #[test]
    fn test_format_neighbors_empty() {
        let result = format_neighbors(&[]);
        assert!(result.contains("无关联节点"));
    }

    #[test]
    fn test_format_recent_thoughts() {
        let thoughts = vec!["今天想了很多".to_string(), "关于Rust的思考".to_string()];
        let result = format_recent_thoughts(&thoughts, 50);
        assert!(result.contains("今天想了很多"));
        assert!(result.contains("关于Rust的思考"));
    }

    #[test]
    fn test_format_recent_thoughts_truncation() {
        let long = "a".repeat(100);
        let thoughts = vec![long];
        let result = format_recent_thoughts(&thoughts, 20);
        assert!(result.len() < 30); // 20 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_format_recent_thoughts_chinese_truncation() {
        // 中文 UTF-8 安全截断 / Chinese UTF-8 safe truncation
        let thoughts = vec!["这是一个很长的中文句子用来测试截断".to_string()];
        let result = format_recent_thoughts(&thoughts, 10);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_format_emotion_summary_positive() {
        let result = format_emotion_summary(0.5, 0.4, 0.2);
        assert!(result.contains("偏正面"));
        assert!(result.contains("较激动"));
    }

    #[test]
    fn test_format_emotion_summary_negative() {
        let result = format_emotion_summary(-0.3, 0.05, 0.1);
        assert!(result.contains("偏负面"));
        assert!(result.contains("较平静"));
    }

    #[allow(clippy::const_is_empty)]
    #[test]
    fn test_prompt_templates_not_empty() {
        assert!(!PROMPT_GRAPH_WANDER.is_empty());
        assert!(!PROMPT_DIARY_ENTRY.is_empty());
        assert!(!PROMPT_DAYDREAM.is_empty());
        assert!(!PROMPT_AUTONOMOUS_LEARNING.is_empty());
        assert!(!PROMPT_DIARY_REFLECTION.is_empty());
        assert!(!PROMPT_CHAPTER_GENERATION.is_empty());
        assert!(!PROMPT_NARRATIVE_REWRITE.is_empty());
        assert!(!PROMPT_SELF_DESCRIPTION.is_empty());
    }

    #[allow(clippy::const_is_empty)]
    #[test]
    fn test_system_prompts_not_empty() {
        assert!(!SYSTEM_INNER_MONOLOGUE.is_empty());
        assert!(!SYSTEM_DIARY.is_empty());
        assert!(!SYSTEM_NARRATIVE.is_empty());
        assert!(!SYSTEM_DAYDREAM.is_empty());
        assert!(!SYSTEM_AUTONOMOUS_LEARNING.is_empty());
        assert!(!SYSTEM_DIARY_REFLECTION.is_empty());
        assert!(!SYSTEM_NARRATIVE_REWRITE.is_empty());
    }

    #[test]
    fn test_graph_wander_prompt_render() {
        let prompt = render_template(
            PROMPT_GRAPH_WANDER,
            &[
                ("seed_node", "Rust"),
                ("neighbors", "系统编程(权重0.90)、内存安全(权重0.80)"),
                ("recent_thoughts", "之前在想编程语言的设计"),
            ],
        );
        assert!(prompt.contains("Rust"));
        assert!(prompt.contains("系统编程"));
        assert!(prompt.contains("之前在想"));
    }
}
