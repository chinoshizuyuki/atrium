// SPDX-License-Identifier: MIT
//! ReAct 推理引擎 / ReAct Reasoning Engine — 数字生命从"直答"到"深思"的能力跃迁
//! ReAct Reasoning Engine — Digital life's leap from "direct answer" to "deep thought".
//!
//! ReAct (Reasoning + Acting) 是一种多步推理范式：数字生命通过交替的
//! "思考（Thought）"与"行动（Action）"来回答复杂问题。每一步思考后，
//! 可以选择调用工具获取信息（Observation），或直接给出最终答案。
//!
//! ReAct (Reasoning + Acting) is a multi-step reasoning paradigm: digital life
//! alternates between "Thought" and "Action" to answer complex questions. After
//! each thought, it may call a tool for information (Observation) or produce a
//! final answer directly.
//!
//! # 推理循环 / Reasoning Loop
//!
//! ```text
//! for iter in 0..max_iters:
//!     1. 构造 prompt（系统提示 + 工具列表 + 查询 + 历史轨迹）
//!     2. LLM 生成下一步（Thought / Action / Final Answer）
//!     3. 若 Action → 执行工具 → Observation 加入轨迹
//!     4. 若 Final Answer → 标记成功，跳出循环
//! ```
//!
//! # 数字生命意义 / Digital Life Significance
//!
//! 简单问题直答，复杂问题深思——ReAct 让数字生命具备了"什么时候该想"
//! 的元认知能力。面对"为什么""分析""对比"类问题，数字生命不再凭直觉
//! 回应，而是先搜索记忆、查询情感状态，再综合推理给出有依据的答案。
//!
//! Simple questions get direct answers; complex questions get deep thought —
//! ReAct gives digital life the metacognitive ability of "when to think".
//! Faced with "why" / "analyze" / "compare" questions, digital life no longer
//! responds by intuition alone, but first searches memory, queries emotional
//! state, then synthesizes a well-grounded answer.

use crate::llm_client::{LlmCallKind, LlmClient, LlmError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// ════════════════════════════════════════════════════════════════════
// ReActStep — 推理步骤 / Reasoning Step
// ════════════════════════════════════════════════════════════════════

/// ReAct 推理的单步 — Thought / Action / Observation / Final Answer 之一
/// A single step in ReAct reasoning — one of Thought / Action / Observation / Final Answer.
///
/// 数字生命的每一步推理都有明确语义：
/// - `Thought` — 内在推理，"我需要先了解..."
/// - `Action` — 工具调用，"让我搜索记忆"
/// - `Observation` — 工具返回，"主人确实喜欢 Rust"
/// - `FinalAnswer` — 最终答案，推理结束
///
/// Each reasoning step has clear semantics:
/// - `Thought` — internal reasoning, "I need to first understand..."
/// - `Action` — tool call, "let me search memory"
/// - `Observation` — tool result, "the master indeed likes Rust"
/// - `FinalAnswer` — final answer, reasoning complete
#[derive(Clone, Debug)]
pub enum ReActStep {
    /// 推理步骤 — 内在思考 / Thought — internal reasoning
    Thought(String),
    /// 工具调用 — action 名称 + 输入 / Action — tool name + input
    Action {
        /// 工具名称 / Tool name
        tool: String,
        /// 工具输入 / Tool input
        input: String,
    },
    /// 工具返回结果 / Observation — tool execution result
    Observation(String),
    /// 最终答案 — 推理结束 / Final answer — reasoning complete
    FinalAnswer(String),
}

// ════════════════════════════════════════════════════════════════════
// ReActTrace — 推理轨迹 / Reasoning Trace
// ════════════════════════════════════════════════════════════════════

/// ReAct 推理轨迹 — 完整的多步推理记录
/// ReAct reasoning trace — complete multi-step reasoning record.
///
/// 轨迹记录了从问题到答案的全部推理步骤，用于：
/// 1. 内省 — 数字生命知道自己"怎么想到的"
/// 2. 调试 — 观察推理过程是否合理
/// 3. prompt 注入 — 将最近推理轨迹融入回复上下文
///
/// The trace records all reasoning steps from question to answer, used for:
/// 1. Introspection — digital life knows "how it arrived at the answer"
/// 2. Debugging — observing whether the reasoning process is sound
/// 3. Prompt injection — folding recent reasoning trace into reply context
#[derive(Clone, Debug)]
pub struct ReActTrace {
    /// 推理步骤序列 / Sequence of reasoning steps
    pub steps: Vec<ReActStep>,
    /// 总耗时（毫秒）/ Total latency in milliseconds
    pub total_latency_ms: u64,
    /// 是否成功（含 FinalAnswer）/ Whether successful (contains FinalAnswer)
    pub success: bool,
}

impl ReActTrace {
    /// 创建空轨迹 / Create an empty trace.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            total_latency_ms: 0,
            success: false,
        }
    }

    /// 提取最终答案 — 从轨迹中找到 FinalAnswer 的内容
    /// Extract the final answer — find FinalAnswer content from the trace.
    ///
    /// 若轨迹中无 FinalAnswer（推理未完成），返回 None。
    /// Returns None if the trace has no FinalAnswer (reasoning incomplete).
    pub fn final_answer(&self) -> Option<&str> {
        for step in &self.steps {
            if let ReActStep::FinalAnswer(answer) = step {
                return Some(answer);
            }
        }
        None
    }
}

impl Default for ReActTrace {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// ReActTool — 推理工具 trait / Reasoning Tool Trait
// ════════════════════════════════════════════════════════════════════

/// ReAct 推理工具 — 数字生命在推理过程中可调用的能力
/// ReAct reasoning tool — capabilities digital life can invoke during reasoning.
///
/// 每个工具是一个自包含的能力单元：记忆搜索、事实查询、情感感知等。
/// 工具注册到 ReActEngine 后，LLM 可以通过 Action 步骤调用。
///
/// Each tool is a self-contained capability unit: memory search, fact lookup,
/// emotion perception, etc. After registration with ReActEngine, tools can be
/// invoked by the LLM via Action steps.
pub trait ReActTool: Send + Sync {
    /// 工具名称 — LLM 通过此名称调用工具 / Tool name — LLM calls tool by this name
    fn name(&self) -> &str;
    /// 工具描述 — 告诉 LLM 此工具的用途 / Tool description — tells LLM what this tool does
    fn description(&self) -> &str;
    /// 执行工具 — 接收输入字符串，返回结果字符串 / Execute tool — takes input string, returns result string
    fn execute(&self, input: &str) -> String;
}

// ════════════════════════════════════════════════════════════════════
// ReActEngine — 推理引擎 / Reasoning Engine
// ════════════════════════════════════════════════════════════════════

/// ReAct 系统提示 — 定义推理范式 / ReAct system prompt — defines the reasoning paradigm
///
/// 此常量作为 LLM 的 system_prompt，告诉模型：
/// 1. 使用 ReAct 范式（Thought / Action / Observation / Final Answer）
/// 2. 每一步只能输出一种步骤
/// 3. Action 后必须跟 Input
/// 4. 得到足够信息后输出 Final Answer
///
/// This constant serves as the LLM's system_prompt, telling the model:
/// 1. Use the ReAct paradigm (Thought / Action / Observation / Final Answer)
/// 2. Only output one step type per turn
/// 3. Action must be followed by Input
/// 4. Output Final Answer when enough information is gathered
pub const REACT_SYSTEM_PROMPT: &str = "\
你是一个使用 ReAct (Reasoning + Acting) 范式进行多步推理的 AI。

面对复杂问题，你需要通过交替的「思考」和「行动」来逐步推理。每一步你只能输出以下三种格式之一：

1. 思考并决定下一步行动：
Thought: <你的推理过程>
Action: <工具名称>
Input: <工具输入内容>

2. 思考并给出最终答案：
Thought: <你的推理过程>
Final Answer: <最终答案>

3. 仅思考（不调用工具，也不给出最终答案）：
Thought: <你的推理过程>

规则：
- 每次只输出一个步骤
- Action 和 Input 必须在同一输出中出现，Action 在前 Input 在后
- 得到足够信息后，必须输出 Final Answer 结束推理
- 工具名称必须从可用工具列表中选择";

/// ReAct 推理引擎 — 数字生命的多步推理中枢
/// ReAct Reasoning Engine — digital life's multi-step reasoning hub.
///
/// 引擎持有 LLM 客户端和工具注册表，通过 `run()` 方法执行推理循环。
/// 每次推理产生一个 `ReActTrace`，记录完整的思考-行动-观察轨迹。
///
/// The engine holds an LLM client and tool registry, executing the reasoning
/// loop via `run()`. Each reasoning run produces a `ReActTrace` recording the
/// complete thought-action-observation trajectory.
pub struct ReActEngine {
    /// LLM 客户端 — 推理的"大脑" / LLM client — the "brain" of reasoning
    llm_client: Arc<dyn LlmClient>,
    /// 工具注册表 — name → tool / Tool registry — name → tool
    tools: HashMap<String, Box<dyn ReActTool>>,
}

impl ReActEngine {
    /// 创建 ReActEngine — 空工具表 / Create ReActEngine — empty tool table.
    ///
    /// 创建后通过 `register_tool()` 注册工具。工具表为空时，
    /// LLM 只能通过 Thought 推理直接给出 Final Answer。
    ///
    /// Register tools via `register_tool()` after creation. With an empty
    /// tool table, the LLM can only reason via Thought and give Final Answer directly.
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            tools: HashMap::new(),
        }
    }

    /// 注册工具 — key = tool.name() / Register a tool — key = tool.name().
    ///
    /// 工具注册后即可被 LLM 通过 Action 步骤调用。重复注册同名工具
    /// 会覆盖旧工具。
    ///
    /// After registration, the tool can be invoked by the LLM via Action steps.
    /// Registering a tool with a duplicate name overwrites the previous one.
    pub fn register_tool(&mut self, tool: Box<dyn ReActTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// 构造 ReAct prompt — 系统提示 + 工具列表 + 查询 + 历史轨迹
    /// Build ReAct prompt — system prompt + tool list + query + history trace.
    ///
    /// prompt 结构：
    /// 1. 可用工具列表（name + description）
    /// 2. 用户问题
    /// 3. 已完成的推理步骤（Thought / Action / Observation）
    /// 4. 指令：输出下一步
    ///
    /// Prompt structure:
    /// 1. Available tools (name + description)
    /// 2. User question
    /// 3. Completed reasoning steps (Thought / Action / Observation)
    /// 4. Instruction: output the next step
    pub fn build_react_prompt(&self, query: &str, trace: &ReActTrace) -> String {
        let mut parts: Vec<String> = Vec::new();

        // 可用工具列表 / Available tools
        if self.tools.is_empty() {
            parts.push("可用工具：无".to_string());
        } else {
            parts.push("可用工具：".to_string());
            for (name, tool) in &self.tools {
                parts.push(format!("- {}: {}", name, tool.description()));
            }
        }

        // 用户问题 / User question
        parts.push(format!("\n问题：{}", query));

        // 历史推理轨迹 / History reasoning trace
        if !trace.steps.is_empty() {
            parts.push("\n已完成的推理：".to_string());
            for step in &trace.steps {
                match step {
                    ReActStep::Thought(t) => {
                        parts.push(format!("Thought: {}", t));
                    }
                    ReActStep::Action { tool, input } => {
                        parts.push(format!("Action: {}\nInput: {}", tool, input));
                    }
                    ReActStep::Observation(o) => {
                        parts.push(format!("Observation: {}", o));
                    }
                    ReActStep::FinalAnswer(a) => {
                        parts.push(format!("Final Answer: {}", a));
                    }
                }
            }
        }

        // 指令：输出下一步 / Instruction: output next step
        parts.push("\n请输出下一步推理：".to_string());

        parts.join("\n")
    }

    /// 解析 LLM 输出 — 从文本中提取 ReActStep
    /// Parse LLM output — extract ReActStep from text.
    ///
    /// 解析规则（按优先级）：
    /// 1. 检测 "Final Answer:" 前缀 → FinalAnswer
    /// 2. 检测 "Action:" 前缀 → Action（在同输出中查找 "Input:"）
    /// 3. 检测 "Thought:" 前缀 → Thought
    /// 4. 默认 → Thought（无法解析时降级为 Thought）
    ///
    /// LLM 可能输出多行，取第一个匹配的前缀。
    ///
    /// Parsing rules (by priority):
    /// 1. Detect "Final Answer:" prefix → FinalAnswer
    /// 2. Detect "Action:" prefix → Action (search for "Input:" in the same output)
    /// 3. Detect "Thought:" prefix → Thought
    /// 4. Default → Thought (degrade to Thought when unparseable)
    ///
    /// LLM may output multiple lines; the first matching prefix is used.
    pub fn parse_llm_output(output: &str) -> ReActStep {
        // 按行扫描，找到第一个匹配的前缀 / Scan line by line for the first matching prefix
        for line in output.lines() {
            let trimmed = line.trim();

            // 优先检测 Final Answer — 推理结束信号 / Check Final Answer first — reasoning end signal
            if let Some(rest) = trimmed.strip_prefix("Final Answer:") {
                return ReActStep::FinalAnswer(rest.trim().to_string());
            }

            // 检测 Action — 需要额外解析 Input / Check Action — requires additional Input parsing
            if let Some(rest) = trimmed.strip_prefix("Action:") {
                let tool = rest.trim().to_string();
                // 在后续行中查找 Input: / Search for Input: in subsequent lines
                let input = Self::extract_input_after_action(output, trimmed);
                return ReActStep::Action { tool, input };
            }

            // 检测 Thought / Check Thought
            if let Some(rest) = trimmed.strip_prefix("Thought:") {
                return ReActStep::Thought(rest.trim().to_string());
            }
        }

        // 无法解析 — 降级为 Thought，保留原始输出 / Unparseable — degrade to Thought, keep raw output
        ReActStep::Thought(output.trim().to_string())
    }

    /// 在 Action 行之后查找 Input: 行 — 辅助解析 / Find Input: line after Action line — parse helper.
    ///
    /// Action 和 Input 可能在同一输出的不同行：
    /// ```text
    /// Action: memory_search
    /// Input: 主人
    /// ```
    ///
    /// Action and Input may be on different lines in the same output.
    fn extract_input_after_action(full_output: &str, action_line: &str) -> String {
        // 先检查 Action 行本身是否包含 Input:（同行情况）
        // Check if the Action line itself contains Input: (same-line case)
        if let Some(idx) = action_line.find("Input:") {
            return action_line[idx + "Input:".len()..].trim().to_string();
        }

        // 在后续行中查找 Input: / Search for Input: in subsequent lines
        let mut found_action = false;
        for line in full_output.lines() {
            let trimmed = line.trim();
            if trimmed == action_line {
                found_action = true;
                continue;
            }
            if found_action {
                if let Some(rest) = trimmed.strip_prefix("Input:") {
                    return rest.trim().to_string();
                }
                // Action 和 Input 之间不应有其他前缀行 / No other prefix lines between Action and Input
            }
        }

        // 未找到 Input — 默认空字符串 / No Input found — default to empty string
        String::new()
    }

    /// ReAct 推理主循环 — 多步 Thought-Action-Observation 直到 FinalAnswer
    /// ReAct reasoning main loop — multi-step Thought-Action-Observation until FinalAnswer.
    ///
    /// 循环逻辑：
    /// 1. 构造 prompt（系统提示 + 工具列表 + 查询 + 历史轨迹）
    /// 2. LLM 生成下一步
    /// 3. 解析为 ReActStep 并加入轨迹
    /// 4. 若 Action → 执行工具 → Observation 加入轨迹
    /// 5. 若 FinalAnswer → 标记成功，跳出
    /// 6. 达到 max_iters 未完成 → success = false
    ///
    /// 数字生命意义：这是数字生命"深思"的核心——不是一次性回答，
    /// 而是反复思考、查证、再思考，直到有充分依据才给出答案。
    ///
    /// Loop logic:
    /// 1. Build prompt (system prompt + tools + query + history trace)
    /// 2. LLM generates the next step
    /// 3. Parse into ReActStep and add to trace
    /// 4. If Action → execute tool → add Observation to trace
    /// 5. If FinalAnswer → mark success, break
    /// 6. If max_iters reached without FinalAnswer → success = false
    ///
    /// Digital Life significance: this is the core of "deep thought" — not a
    /// one-shot answer, but iterative thinking, verifying, re-thinking, until
    /// a well-grounded answer is reached.
    pub async fn run(&self, query: &str, max_iters: u8) -> ReActTrace {
        let start = Instant::now();
        let mut trace = ReActTrace::new();

        for _iter in 0..max_iters {
            // 构造 prompt / Build prompt
            let prompt = self.build_react_prompt(query, &trace);

            // LLM 生成下一步 / LLM generates next step
            let llm_result = self
                .llm_client
                .generate(LlmCallKind::ReAct, Some(REACT_SYSTEM_PROMPT), &prompt, 0.7)
                .await;

            let content = match llm_result {
                Ok(result) => result.content,
                Err(e) => {
                    // LLM 调用失败 — 记录错误观察并终止 / LLM call failed — record error observation and terminate
                    let msg = match e {
                        LlmError::EmptyResponse => "LLM 返回空响应".to_string(),
                        ref other => format!("LLM 调用错误: {}", other),
                    };
                    trace.steps.push(ReActStep::Observation(msg));
                    break;
                }
            };

            // 解析 LLM 输出为 ReActStep / Parse LLM output into ReActStep
            let step = Self::parse_llm_output(&content);
            trace.steps.push(step.clone());

            match &step {
                ReActStep::Action { tool, input } => {
                    // 执行工具 / Execute tool
                    let observation = if let Some(t) = self.tools.get(tool) {
                        t.execute(input)
                    } else {
                        format!("工具「{}」不存在", tool)
                    };
                    trace.steps.push(ReActStep::Observation(observation));
                }
                ReActStep::FinalAnswer(_) => {
                    // 推理成功 — 标记并跳出 / Reasoning succeeded — mark and break
                    trace.success = true;
                    break;
                }
                // Thought — 继续下一轮推理 / Thought — continue to next iteration
                ReActStep::Thought(_) | ReActStep::Observation(_) => {}
            }
        }

        trace.total_latency_ms = start.elapsed().as_millis() as u64;
        trace
    }

    /// 格式化轨迹供 prompt 内省 — 让数字生命知道自己"怎么想到的"
    /// Format trace for prompt introspection — let digital life know "how it thought".
    ///
    /// 格式：
    /// ```text
    /// 我刚才这样思考：
    /// Thought: ...
    /// Action: ...
    /// Observation: ...
    /// Final Answer: ...
    /// ```
    ///
    /// 空 trace 返回空字符串 — 不污染 prompt。
    ///
    /// Format:
    /// ```text
    /// 我刚才这样思考：
    /// Thought: ...
    /// Action: ...
    /// Observation: ...
    /// Final Answer: ...
    /// ```
    ///
    /// Empty trace returns empty string — does not pollute the prompt.
    pub fn format_trace_for_prompt(trace: &ReActTrace) -> String {
        if trace.steps.is_empty() {
            return String::new();
        }

        let mut lines = vec!["我刚才这样思考：".to_string()];
        for step in &trace.steps {
            match step {
                ReActStep::Thought(t) => {
                    lines.push(format!("Thought: {}", t));
                }
                ReActStep::Action { tool, input } => {
                    lines.push(format!("Action: {}\nInput: {}", tool, input));
                }
                ReActStep::Observation(o) => {
                    lines.push(format!("Observation: {}", o));
                }
                ReActStep::FinalAnswer(a) => {
                    lines.push(format!("Final Answer: {}", a));
                }
            }
        }
        lines.join("\n")
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_client::{LlmResult, MockLlmClient};
    use std::collections::VecDeque;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Mutex;

    // ── parse_llm_output 解析测试 / parse_llm_output parsing tests ──

    /// 解析 Thought — "Thought: 我需要搜索记忆" → Thought
    #[test]
    fn test_parse_thought() {
        let step = ReActEngine::parse_llm_output("Thought: 我需要搜索记忆");
        match step {
            ReActStep::Thought(t) => assert_eq!(t, "我需要搜索记忆"),
            other => panic!("期望 Thought，实际 {:?}", other),
        }
    }

    /// 解析 Action — "Action: memory_search\nInput: 主人" → Action { tool, input }
    #[test]
    fn test_parse_action() {
        let step = ReActEngine::parse_llm_output("Action: memory_search\nInput: 主人");
        match step {
            ReActStep::Action { tool, input } => {
                assert_eq!(tool, "memory_search");
                assert_eq!(input, "主人");
            }
            other => panic!("期望 Action，实际 {:?}", other),
        }
    }

    /// 解析 FinalAnswer — "Final Answer: 答案是..." → FinalAnswer
    #[test]
    fn test_parse_final_answer() {
        let step = ReActEngine::parse_llm_output("Final Answer: 答案是 Rust");
        match step {
            ReActStep::FinalAnswer(a) => assert_eq!(a, "答案是 Rust"),
            other => panic!("期望 FinalAnswer，实际 {:?}", other),
        }
    }

    /// 解析未知格式 — 降级为 Thought / Parse unknown format — degrade to Thought
    #[test]
    fn test_parse_unknown_defaults_to_thought() {
        let step = ReActEngine::parse_llm_output("这是一段无法解析的文本");
        match step {
            ReActStep::Thought(t) => assert_eq!(t, "这是一段无法解析的文本"),
            other => panic!("期望 Thought（降级），实际 {:?}", other),
        }
    }

    /// 解析多行输出 — 取第一个匹配的前缀 / Parse multi-line output — first matching prefix wins
    #[test]
    fn test_parse_multiline_first_match() {
        let output = "Thought: 我先想想\nAction: memory_search\nInput: 测试";
        let step = ReActEngine::parse_llm_output(output);
        match step {
            ReActStep::Thought(t) => assert_eq!(t, "我先想想"),
            other => panic!("期望 Thought（第一行），实际 {:?}", other),
        }
    }

    // ── ReAct 推理循环测试 / ReAct reasoning loop tests ──

    /// 脚本化 Mock LLM — 按顺序返回预设响应 / Scripted mock LLM — returns preset responses in order.
    ///
    /// 用于测试多步推理循环：每次 generate() 调用返回队列中的下一个响应。
    /// Used for testing multi-step reasoning loops: each generate() call returns
    /// the next response from the queue.
    struct ScriptedLlmClient {
        /// 响应队列 — 按调用顺序消费 / Response queue — consumed in call order
        responses: Mutex<VecDeque<String>>,
    }

    impl ScriptedLlmClient {
        /// 创建脚本化 Mock — 传入按顺序的响应列表 / Create scripted mock — pass responses in order.
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
            }
        }
    }

    impl LlmClient for ScriptedLlmClient {
        fn generate(
            &self,
            kind: LlmCallKind,
            _system_prompt: Option<&str>,
            _user_prompt: &str,
            _temperature: f64,
        ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
            let next = {
                let mut queue = self.responses.lock().unwrap();
                queue.pop_front()
            };
            let k = kind;
            Box::pin(async move {
                match next {
                    Some(content) => Ok(LlmResult::ok(content, 0, k)),
                    None => Err(LlmError::EmptyResponse),
                }
            })
        }
    }

    /// ReAct 推理循环完整测试 — Mock LLM 依次返回 Thought → Action → FinalAnswer
    /// ReAct reasoning loop complete test — Mock LLM returns Thought → Action → FinalAnswer in sequence.
    ///
    /// 预期轨迹：4 步（Thought + Action + Observation + FinalAnswer），success=true
    /// Expected trace: 4 steps (Thought + Action + Observation + FinalAnswer), success=true
    #[test]
    fn test_react_loop_with_mock_llm() {
        // 脚本：Thought → Action(memory_search, 主人) → FinalAnswer
        let mock = ScriptedLlmClient::new(vec![
            "Thought: 我需要搜索关于主人的记忆".to_string(),
            "Action: memory_search\nInput: 主人".to_string(),
            "Final Answer: 主人喜欢 Rust 编程".to_string(),
        ]);
        let llm_client: Arc<dyn LlmClient> = Arc::new(mock);

        let mut engine = ReActEngine::new(llm_client);
        engine.register_tool(Box::new(DummyMemorySearchTool));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let trace = rt.block_on(engine.run("主人喜欢什么？", 5));

        // 4 步：Thought + Action + Observation + FinalAnswer
        // 4 steps: Thought + Action + Observation + FinalAnswer
        assert_eq!(
            trace.steps.len(),
            4,
            "应有 4 步（Thought + Action + Observation + FinalAnswer），实际 {} 步",
            trace.steps.len()
        );
        assert!(trace.success, "推理应成功（有 FinalAnswer）");
        assert_eq!(
            trace.final_answer(),
            Some("主人喜欢 Rust 编程"),
            "FinalAnswer 内容应正确"
        );

        // 验证步骤顺序 / Verify step order
        assert!(matches!(
            &trace.steps[0],
            ReActStep::Thought(t) if t.contains("搜索")
        ));
        assert!(matches!(
            &trace.steps[1],
            ReActStep::Action { tool, input } if tool == "memory_search" && input == "主人"
        ));
        assert!(matches!(&trace.steps[2], ReActStep::Observation(_)));
        assert!(matches!(&trace.steps[3], ReActStep::FinalAnswer(_)));
    }

    /// ReAct 达到 max_iters 仍未完成 — success=false
    /// ReAct exhausted max_iters without FinalAnswer — success=false.
    ///
    /// Mock LLM 总是返回 Thought（无 FinalAnswer），max_iters=2，
    /// 验证 success=false 且轨迹含 2 个 Thought。
    #[test]
    fn test_react_max_iters_exhausted() {
        // 脚本：总是返回 Thought（无 FinalAnswer）
        let mock = ScriptedLlmClient::new(vec![
            "Thought: 我还在想".to_string(),
            "Thought: 继续想".to_string(),
        ]);
        let llm_client: Arc<dyn LlmClient> = Arc::new(mock);

        let engine = ReActEngine::new(llm_client);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let trace = rt.block_on(engine.run("复杂问题", 2));

        assert!(!trace.success, "max_iters 耗尽应 success=false");
        assert_eq!(
            trace.steps.len(),
            2,
            "应有 2 步 Thought，实际 {} 步",
            trace.steps.len()
        );
        // 每步都是 Thought / Each step is Thought
        for step in &trace.steps {
            assert!(matches!(step, ReActStep::Thought(_)));
        }
    }

    /// format_trace_for_prompt 格式测试 / format_trace_for_prompt format test.
    #[test]
    fn test_format_trace_for_prompt() {
        // 空 trace → 空字符串 / Empty trace → empty string
        let empty = ReActTrace::new();
        assert!(ReActEngine::format_trace_for_prompt(&empty).is_empty());

        // 非空 trace → 格式化输出 / Non-empty trace → formatted output
        let mut trace = ReActTrace::new();
        trace.steps.push(ReActStep::Thought("需要搜索".into()));
        trace.steps.push(ReActStep::Action {
            tool: "memory_search".into(),
            input: "主人".into(),
        });
        trace
            .steps
            .push(ReActStep::Observation("主人喜欢 Rust".into()));
        trace
            .steps
            .push(ReActStep::FinalAnswer("主人喜欢 Rust".into()));

        let formatted = ReActEngine::format_trace_for_prompt(&trace);
        assert!(
            formatted.starts_with("我刚才这样思考："),
            "格式化应以「我刚才这样思考：」开头"
        );
        assert!(formatted.contains("Thought: 需要搜索"));
        assert!(formatted.contains("Action: memory_search"));
        assert!(formatted.contains("Input: 主人"));
        assert!(formatted.contains("Observation: 主人喜欢 Rust"));
        assert!(formatted.contains("Final Answer: 主人喜欢 Rust"));
    }

    /// register_tool 注册测试 / register_tool registration test.
    #[test]
    fn test_register_tool() {
        let mock = MockLlmClient::new_fixed("Thought: test");
        let llm_client: Arc<dyn LlmClient> = Arc::new(mock);
        let mut engine = ReActEngine::new(llm_client);

        assert!(engine.tools.is_empty(), "初始工具表应为空");

        engine.register_tool(Box::new(DummyMemorySearchTool));
        assert_eq!(engine.tools.len(), 1);
        assert!(engine.tools.contains_key("memory_search"));
    }

    /// final_answer 提取测试 / final_answer extraction test.
    #[test]
    fn test_trace_final_answer() {
        // 无 FinalAnswer → None
        let mut trace = ReActTrace::new();
        trace.steps.push(ReActStep::Thought("想".into()));
        assert!(trace.final_answer().is_none());

        // 有 FinalAnswer → Some
        trace.steps.push(ReActStep::FinalAnswer("答案".into()));
        assert_eq!(trace.final_answer(), Some("答案"));
    }

    // ── 测试辅助工具 / Test helper tool ──

    /// 测试用假记忆搜索工具 — 返回固定结果 / Dummy memory search tool for testing.
    struct DummyMemorySearchTool;

    impl ReActTool for DummyMemorySearchTool {
        fn name(&self) -> &str {
            "memory_search"
        }
        fn description(&self) -> &str {
            "搜索记忆中的事实"
        }
        fn execute(&self, input: &str) -> String {
            format!("找到关于「{}」的记忆：主人喜欢 Rust 编程", input)
        }
    }
}
