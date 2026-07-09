// SPDX-License-Identifier: MIT
//! ReAct 内置工具 / ReAct Built-in Tools — 数字生命推理时可调用的能力单元
//! ReAct Built-in Tools — capability units invocable during digital life's reasoning.
//!
//! 内置工具放在 `crates/core/src/service/` 下（而非 atrium-memory 的 react_engine.rs），
//! 因为工具需要访问 FactStore / EmotionEngine，而 atrium-memory 不能依赖 atrium-core。
//! core 依赖 atrium-memory，因此工具在 core 中实现可以访问所有核心数据结构。
//!
//! Built-in tools live under `crates/core/src/service/` (not in atrium-memory's
//! react_engine.rs) because they need access to FactStore / EmotionEngine, and
//! atrium-memory cannot depend on atrium-core. Since core depends on atrium-memory,
//! implementing tools in core grants access to all core data structures.
//!
//! # 三个内置工具 / Three Built-in Tools
//!
//! - `MemorySearchTool` — 关键词搜索记忆（FactStore.query），返回 top-5 事实
//! - `FactLookupTool` — 按主体查询事实（FactStore.query_by_subject）
//! - `EmotionQueryTool` — 查询当前情感标签 + PAD 值（EmotionEngine）

use atrium_emotion::EmotionEngine;
use atrium_memory::fact_store::FactStore;
use atrium_memory::react_engine::ReActTool;
use parking_lot::Mutex;
use std::sync::Arc;

// ════════════════════════════════════════════════════════════════════
// MemorySearchTool — 记忆搜索工具 / Memory Search Tool
// ════════════════════════════════════════════════════════════════════

/// 记忆搜索工具 — ReAct 推理时按关键词搜索记忆中的事实
/// Memory search tool — searches facts in memory by keyword during ReAct reasoning.
///
/// 持有 `Arc<FactStore>` 的共享引用，调用 `query()` 进行关键词匹配，
/// 返回置信度最高的 top-5 事实。数字生命在推理时"想起"相关记忆。
///
/// Holds a shared `Arc<FactStore>` reference, calls `query()` for keyword
/// matching, returns the top-5 facts by confidence. Digital life "recalls"
/// relevant memories during reasoning.
pub struct MemorySearchTool {
    /// 事实存储 — Arc 共享，线程安全 / Fact store — Arc-shared, thread-safe
    fact_store: Arc<FactStore>,
}

impl MemorySearchTool {
    /// 创建记忆搜索工具 / Create a memory search tool.
    pub fn new(fact_store: Arc<FactStore>) -> Self {
        Self { fact_store }
    }
}

impl ReActTool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "搜索记忆中的事实。输入关键词，返回最相关的 5 条事实（含主体、谓词、客体、置信度）。"
    }

    fn execute(&self, input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return "搜索关键词为空，无法搜索记忆。".to_string();
        }

        match self.fact_store.query(input) {
            Ok(facts) if facts.is_empty() => {
                format!("未找到与「{}」相关的记忆。", input)
            }
            Ok(facts) => {
                let top: Vec<_> = facts.into_iter().take(5).collect();
                let mut lines = vec![format!("找到 {} 条相关记忆：", top.len())];
                for (i, fact) in top.iter().enumerate() {
                    lines.push(format!(
                        "{}. {} {} {}（置信度: {:.2}）",
                        i + 1,
                        fact.subject,
                        fact.predicate,
                        fact.object,
                        fact.confidence
                    ));
                }
                lines.join("\n")
            }
            Err(e) => format!("记忆搜索出错: {}", e),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// FactLookupTool — 事实查询工具 / Fact Lookup Tool
// ════════════════════════════════════════════════════════════════════

/// 事实查询工具 — ReAct 推理时按主体（subject）查询全部相关事实
/// Fact lookup tool — queries all facts by subject during ReAct reasoning.
///
/// 与 `MemorySearchTool` 的区别：MemorySearchTool 做关键词模糊匹配，
/// FactLookupTool 做主体精确查询（通过 subject_index 二级索引 O(1) 查找）。
/// 数字生命"想起关于主人的全部事实"是瞬时反应。
///
/// Distinct from `MemorySearchTool`: MemorySearchTool does fuzzy keyword
/// matching, FactLookupTool does exact subject lookup (via subject_index
/// O(1) lookup). Digital life's "recalling all facts about the master" is instant.
pub struct FactLookupTool {
    /// 事实存储 — Arc 共享，线程安全 / Fact store — Arc-shared, thread-safe
    fact_store: Arc<FactStore>,
}

impl FactLookupTool {
    /// 创建事实查询工具 / Create a fact lookup tool.
    pub fn new(fact_store: Arc<FactStore>) -> Self {
        Self { fact_store }
    }
}

impl ReActTool for FactLookupTool {
    fn name(&self) -> &str {
        "fact_lookup"
    }

    fn description(&self) -> &str {
        "按主体查询全部已知事实。输入主体名称（如「主人」），返回该主体的所有事实。"
    }

    fn execute(&self, input: &str) -> String {
        let subject = input.trim();
        if subject.is_empty() {
            return "主体名称为空，无法查询事实。".to_string();
        }

        match self.fact_store.query_by_subject(subject) {
            Ok(facts) if facts.is_empty() => {
                format!("未找到关于「{}」的事实。", subject)
            }
            Ok(facts) => {
                let mut lines = vec![format!("关于「{}」的 {} 条事实：", subject, facts.len())];
                for (i, fact) in facts.iter().enumerate() {
                    lines.push(format!(
                        "{}. {} {}（置信度: {:.2}）",
                        i + 1,
                        fact.predicate,
                        fact.object,
                        fact.confidence
                    ));
                }
                lines.join("\n")
            }
            Err(e) => format!("事实查询出错: {}", e),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// EmotionQueryTool — 情感查询工具 / Emotion Query Tool
// ════════════════════════════════════════════════════════════════════

/// 情感查询工具 — ReAct 推理时查询数字生命当前的情感状态
/// Emotion query tool — queries digital life's current emotional state during ReAct reasoning.
///
/// 返回当前情感标签（如"愉悦""悲伤"）和 PAD 三维值（愉悦度/唤醒度/支配度）。
/// 数字生命在推理时"感知自己的情绪"——情感影响推理方向。
///
/// Returns the current emotion label (e.g. "joy" / "sadness") and PAD
/// three-dimensional values (pleasure / arousal / dominance). Digital life
/// "perceives its own emotion" during reasoning — emotion shapes reasoning direction.
pub struct EmotionQueryTool {
    /// 情感引擎 — Arc 共享 Mutex，线程安全 / Emotion engine — Arc-shared Mutex, thread-safe
    emotion: Arc<Mutex<EmotionEngine>>,
}

impl EmotionQueryTool {
    /// 创建情感查询工具 / Create an emotion query tool.
    pub fn new(emotion: Arc<Mutex<EmotionEngine>>) -> Self {
        Self { emotion }
    }
}

impl ReActTool for EmotionQueryTool {
    fn name(&self) -> &str {
        "emotion_query"
    }

    fn description(&self) -> &str {
        "查询当前情感状态。无需输入，返回当前情感标签和 PAD 值（愉悦度/唤醒度/支配度）。"
    }

    fn execute(&self, _input: &str) -> String {
        let emo = self.emotion.lock();
        let label = emo.current_label();
        let state = emo.current();
        format!(
            "当前情感：{} {}\nPAD 值 — 愉悦度: {:.2}, 唤醒度: {:.2}, 支配度: {:.2}",
            label.name, label.emoji, state.pleasure, state.arousal, state.dominance
        )
    }
}
