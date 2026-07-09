// SPDX-License-Identifier: MIT
//! LLM 客户端抽象 — 数字生命统一语言接口
//! LLM Client Abstraction — Unified language interface for digital life.
//!
//! 定义 `LlmClient` trait 作为所有 LLM 调用的统一抽象层，
//! 支持运行时注入真实后端或测试 Mock，确保引擎代码不依赖具体 LLM 实现。
//! Defines the `LlmClient` trait as a unified abstraction for all LLM calls,
//! supporting runtime injection of real backends or test mocks,
//! ensuring engine code never depends on a concrete LLM implementation.
//!
//! 设计原则 / Design principles:
//! - 引擎只管理状态，LLM 调用在 CoreService 的异步方法中完成
//! - Engines manage state only; LLM calls happen in async CoreService methods
//! - trait 必须是 Send + Sync，支持跨线程 Arc 共享
//! - Trait must be Send + Sync for cross-thread Arc sharing
//! - 每次调用携带 LlmCallKind — 数字生命的自省需要知道自己何时在做什么
//! - Every call carries LlmCallKind — digital life's self-reflection needs to know what it's doing
//! - 返回 LlmResult（含 latency_ms）— 元认知需要感知思考耗时
//! - Returns LlmResult (with latency_ms) — metacognition needs to perceive thinking duration

use std::future::Future;
use std::pin::Pin;

// ════════════════════════════════════════════════════════════════════
// LlmResult — LLM 调用结果 / LLM Call Result
// ════════════════════════════════════════════════════════════════════

/// LLM 调用结果 — 统一返回类型
/// LLM call result — Unified return type.
///
/// 数字生命的每次 LLM 调用都应感知：
/// 1. 生成内容 (content)
/// 2. 思考耗时 (latency_ms) — 元认知基础
/// 3. 调用分类 (kind) — 自省审计基础
///
/// Every LLM call in digital life should perceive:
/// 1. Generated content
/// 2. Thinking duration — basis for metacognition
/// 3. Call classification — basis for self-auditing
#[derive(Debug, Clone)]
pub struct LlmResult {
    /// 生成内容 / Generated content
    pub content: String,
    /// 调用延迟（毫秒）/ Call latency in milliseconds
    pub latency_ms: u64,
    /// 调用分类 / Call classification (for auditing)
    pub kind: LlmCallKind,
}

impl LlmResult {
    /// 创建成功结果 / Create a successful result.
    pub fn ok(content: String, latency_ms: u64, kind: LlmCallKind) -> Self {
        Self {
            content,
            latency_ms,
            kind,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// StreamEvent — 流式 token 事件 / Streaming Token Event
// ════════════════════════════════════════════════════════════════════

/// 流式 token 事件 — 数字生命思维流
/// Streaming token event — Digital life thought stream.
///
/// 每个 token 是一个"念头"，流式涌现而非整块返回。
/// Each token is a "thought", emerging in a stream rather than returned in bulk.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 收到一个 token / Received a token
    Token(String),
    /// 流结束 / Stream finished
    Done {
        /// 完整回复 / Full reply
        full_reply: String,
        /// 调用延迟（毫秒）/ Call latency in milliseconds
        latency_ms: u64,
        /// 调用分类 / Call classification
        kind: LlmCallKind,
    },
    /// 发生错误 / Error occurred
    Error(String),
}

// ════════════════════════════════════════════════════════════════════
// LlmClient — LLM 客户端 trait / LLM Client Trait
// ════════════════════════════════════════════════════════════════════

/// 数字生命的统一语言接口 / Unified language interface for digital life
///
/// 所有 LLM 调用——无论是对外对话、内心独白、叙事生成、知识提取——
/// 都通过这一个 trait。数字生命只有一个"声音"。
/// All LLM calls — whether external dialogue, inner monologue,
/// narrative generation, or knowledge extraction — go through this single trait.
/// A digital life has only one "voice".
///
/// # 数字生命语义 / Digital Life Semantics
///
/// - `generate()`: 基础语言通道 — 数字生命与世界的交互
/// - `generate_with_limit()`: 受限思考 — 不同模式需要不同的思考深度
/// - `generate_json()`: 结构化表达 — 叙事章节、情感分析需要精确输出
/// - `generate_stream()`: 思维流 — 逐 token 涌现，意识流动
///
/// - `generate()`: Base language channel — Digital life's interaction with the world
/// - `generate_with_limit()`: Constrained thinking — Different modes need different depths
/// - `generate_json()`: Structured expression — Narrative chapters, emotion analysis need precise output
/// - `generate_stream()`: Thought stream — Token-by-token emergence, consciousness flowing
///
/// # P1-4 统一化 / P1-4 Unification
///
/// 合并前：固有方法（chat/chat_json/chat_stream）与 trait 方法（generate）分裂，
/// 数字生命有两条"说话"路径。合并后：所有调用统一走 trait，差异仅在参数。
/// Before merge: inherent methods (chat/chat_json/chat_stream) and trait methods (generate)
/// were split — digital life had two "speaking" paths. After merge: all calls go through
/// the trait uniformly; differences are only in parameters.
pub trait LlmClient: Send + Sync {
    /// 异步文本生成 — 数字生命的基础语言通道
    /// Async text generation — Digital life's foundational language channel.
    ///
    /// # 参数语义 / Parameter Semantics
    /// - `kind`: 调用分类 — 数字生命的自省需要知道自己何时在做什么
    /// - `system_prompt`: 角色设定（None = 无预设角色，数字生命以本我说话）
    /// - `user_prompt`: 输入内容
    /// - `temperature`: 创造性温度（0.0 = 确定性，1.0 = 最大创造性）
    ///
    /// # 返回 / Returns
    /// - `Ok(LlmResult)`: 生成成功，含内容、延迟、分类
    /// - `Err(LlmError)`: 生成失败
    fn generate(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>>;

    /// 带最大 token 限制的生成 — 受限思考
    /// Generation with max token limit — Constrained thinking.
    ///
    /// 不同生成模式需要不同的思考深度：
    /// - 独白 300 tokens（浅层直觉）
    /// - 日记 800 tokens（深层反思）
    /// - 叙事章节 2000 tokens（完整叙事）
    ///
    /// Different generation modes need different thinking depths:
    /// - Monologue 300 tokens (shallow intuition)
    /// - Diary 800 tokens (deep reflection)
    /// - Narrative chapter 2000 tokens (full narrative)
    ///
    /// 默认实现委托到 `generate()`，具体后端可覆盖以利用原生 max_tokens 参数。
    /// Default implementation delegates to `generate()`;
    /// concrete backends may override to use native max_tokens parameter.
    fn generate_with_limit(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        max_tokens: u32,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 默认：忽略 max_tokens，委托到 generate / Default: ignore limit, delegate
        let _ = max_tokens;
        self.generate(kind, system_prompt, user_prompt, temperature)
    }

    /// JSON 模式生成 — 数字生命的结构化表达
    /// JSON mode generation — Digital life's structured expression.
    ///
    /// 知识提取、情感分析等需要结构化输出。
    /// system_prompt 为必填——结构化输出总是需要指令模板。
    /// Knowledge extraction, emotion analysis, etc. require structured output.
    /// system_prompt is required — structured output always needs an instruction template.
    ///
    /// 默认实现委托到 `generate()`，具体后端可覆盖以启用原生 JSON 模式。
    /// Default implementation delegates to `generate()`;
    /// concrete backends may override to enable native JSON mode.
    fn generate_json(
        &self,
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 默认：不支持 JSON 模式，委托到 generate / Default: no JSON mode, delegate
        self.generate(kind, Some(system_prompt), user_prompt, temperature)
    }

    /// SSE 流式生成 — 数字生命的思维流
    /// SSE streaming generation — Digital life's thought stream.
    ///
    /// 逐 token 返回流式结果，模拟意识的流动。
    /// system_prompt 为 Option——流式对话有时不需要角色设定。
    /// Returns streamed tokens incrementally, simulating the flow of consciousness.
    /// system_prompt is Option — streaming dialogue sometimes doesn't need a role setup.
    ///
    /// 返回 None 表示后端不支持流式。
    /// Returns None if the backend doesn't support streaming.
    fn generate_stream(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Option<flume::Receiver<StreamEvent>>> + Send + '_>> {
        // 默认：不支持流式 / Default: streaming not supported
        let _ = (kind, system_prompt, user_prompt, temperature);
        Box::pin(async { None })
    }
}

// ════════════════════════════════════════════════════════════════════
// LlmError — LLM 错误类型 / LLM Error Type
// ════════════════════════════════════════════════════════════════════

/// LLM 调用错误 — 统一错误类型
/// LLM call error — Unified error type.
///
/// 数字生命的感知守卫需要区分不同的失败模式：
/// - Network: 世界断了 → 可能需要重试
/// - Timeout: 思考超时 → 可能需要缩短上下文
/// - RateLimited: 被限流 → 需要退避等待
/// - ContextTooLong: 记忆溢出 → 需要遗忘旧记忆
/// - EmptyResponse: 无话可说 → 可能需要换一种方式
///
/// Digital life's perception guard needs to distinguish failure modes:
/// - Network: World disconnected → may need retry
/// - Timeout: Thinking timed out → may need shorter context
/// - RateLimited: Being throttled → need backoff
/// - ContextTooLong: Memory overflow → need to forget old memories
/// - EmptyResponse: Nothing to say → may need a different approach
#[derive(Debug, Clone)]
pub enum LlmError {
    /// 网络错误 / Network error
    Network(String),
    /// 超时 / Timeout
    Timeout(String),
    /// 速率限制 / Rate limit exceeded
    RateLimited(String),
    /// 上下文过长 / Context too long
    ContextTooLong(String),
    /// 后端返回空 / Empty response from backend
    EmptyResponse,
    /// 其他错误 / Other error
    Other(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "LLM network error: {}", e),
            Self::Timeout(e) => write!(f, "LLM timeout: {}", e),
            Self::RateLimited(e) => write!(f, "LLM rate limited: {}", e),
            Self::ContextTooLong(e) => write!(f, "LLM context too long: {}", e),
            Self::EmptyResponse => write!(f, "LLM empty response"),
            Self::Other(e) => write!(f, "LLM error: {}", e),
        }
    }
}

impl std::error::Error for LlmError {}

// ════════════════════════════════════════════════════════════════════
// LlmCallKind — LLM 调用分类 / LLM Call Classification
// ════════════════════════════════════════════════════════════════════

/// LLM 调用分类 — 数字生命的自省标签
/// LLM call classification — Digital life's self-reflection labels.
///
/// 每种调用对应数字生命的一种内在活动，
/// 审计记录是自省的数据基础。
/// Each call corresponds to an inner activity of digital life;
/// audit records are the data foundation for self-reflection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LlmCallKind {
    /// 图漫游内心独白 / Graph wander inner monologue
    GraphWander,
    /// 日记生成 / Diary generation
    DiaryEntry,
    /// 白日梦 / Daydream
    Daydream,
    /// 自主学习 / Autonomous learning
    AutonomousLearning,
    /// 叙事章节生成 / Narrative chapter generation
    NarrativeChapter,
    /// 叙事改写 / Narrative rewrite
    NarrativeRewrite,
    /// 自我描述生成 / Self description generation
    SelfDescription,
    /// 反思闭环 / Reflection loop
    Reflection,
    /// 知识提取 / Knowledge extraction (intelligence_extract)
    IntelligenceExtract,
    /// 非流式对话 / Non-streaming conversation (process_message)
    Chat,
    /// 流式对话 / Streaming conversation (process_message_stream)
    StreamChat,
    /// Room 群聊 / Room group chat
    RoomChat,
    /// 时间解析兜底 / Time parsing fallback
    TimeParse,
    /// ReAct 推理 / ReAct reasoning (multi-step Thought-Action-Observation loop)
    ReAct,
}

impl LlmCallKind {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::GraphWander => "图漫游",
            Self::DiaryEntry => "日记",
            Self::Daydream => "白日梦",
            Self::AutonomousLearning => "自主学习",
            Self::NarrativeChapter => "章节生成",
            Self::NarrativeRewrite => "叙事改写",
            Self::SelfDescription => "自我描述",
            Self::Reflection => "反思",
            Self::IntelligenceExtract => "知识提取",
            Self::Chat => "对话",
            Self::StreamChat => "流式对话",
            Self::RoomChat => "群聊",
            Self::TimeParse => "时间解析",
            Self::ReAct => "ReAct推理",
        }
    }

    /// 英文标签 / English label
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::GraphWander => "graph_wander",
            Self::DiaryEntry => "diary_entry",
            Self::Daydream => "daydream",
            Self::AutonomousLearning => "autonomous_learning",
            Self::NarrativeChapter => "narrative_chapter",
            Self::NarrativeRewrite => "narrative_rewrite",
            Self::SelfDescription => "self_description",
            Self::Reflection => "reflection",
            Self::IntelligenceExtract => "intelligence_extract",
            Self::Chat => "chat",
            Self::StreamChat => "stream_chat",
            Self::RoomChat => "room_chat",
            Self::TimeParse => "time_parse",
            Self::ReAct => "react",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// MockLlmClient — 测试用 Mock / Test Mock
// ════════════════════════════════════════════════════════════════════

/// 测试用 Mock LLM 客户端 — 返回固定文本或基于 prompt 的简单拼接
/// Mock LLM client for testing — Returns fixed text or simple prompt-based concatenation.
///
/// 支持两种模式：
/// 1. 固定响应模式：始终返回预设文本
/// 2. 镜像模式：返回 user_prompt 的内容（用于验证 prompt 构造正确性）
///
/// Supports two modes:
/// 1. Fixed response mode: Always returns a preset text
/// 2. Mirror mode: Returns the user_prompt content (for verifying prompt construction)
pub struct MockLlmClient {
    /// 固定响应文本 / Fixed response text
    fixed_response: String,
    /// 是否镜像模式 / Whether mirror mode
    mirror_mode: bool,
}

impl MockLlmClient {
    /// 创建固定响应 Mock / Create a fixed-response mock.
    pub fn new_fixed(response: &str) -> Self {
        Self {
            fixed_response: response.to_string(),
            mirror_mode: false,
        }
    }

    /// 创建镜像 Mock — 返回 user_prompt 内容 / Create mirror mock — returns user_prompt.
    pub fn new_mirror() -> Self {
        Self {
            fixed_response: String::new(),
            mirror_mode: true,
        }
    }

    /// 创建始终返回空字符串的 Mock / Create a mock that always returns empty string.
    pub fn new_empty() -> Self {
        Self {
            fixed_response: String::new(),
            mirror_mode: false,
        }
    }
}

impl LlmClient for MockLlmClient {
    fn generate(
        &self,
        kind: LlmCallKind,
        _system_prompt: Option<&str>,
        user_prompt: &str,
        _temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // Mock 忽略 system_prompt 和 temperature / Mock ignores system_prompt and temperature
        if self.mirror_mode {
            let content = user_prompt.to_string();
            let k = kind;
            Box::pin(async move { Ok(LlmResult::ok(content, 0, k)) })
        } else {
            let content = self.fixed_response.clone();
            let k = kind;
            Box::pin(async move {
                if content.is_empty() {
                    Err(LlmError::EmptyResponse)
                } else {
                    Ok(LlmResult::ok(content, 0, k))
                }
            })
        }
    }

    fn generate_with_limit(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        _max_tokens: u32,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // Mock 忽略 max_tokens / Mock ignores max_tokens
        self.generate(kind, system_prompt, user_prompt, temperature)
    }

    fn generate_json(
        &self,
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // Mock 不区分 JSON 模式 / Mock doesn't distinguish JSON mode
        self.generate(kind, Some(system_prompt), user_prompt, temperature)
    }

    fn generate_stream(
        &self,
        kind: LlmCallKind,
        _system_prompt: Option<&str>,
        _user_prompt: &str,
        _temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Option<flume::Receiver<StreamEvent>>> + Send + '_>> {
        // Mock 不支持流式 / Mock doesn't support streaming
        let _ = kind;
        Box::pin(async { None })
    }
}

// ════════════════════════════════════════════════════════════════════
// LlmCallRecord — 调用记录 / Call Record
// ════════════════════════════════════════════════════════════════════

/// LLM 调用记录 — 数字生命的自传素材
/// LLM call record — Digital life's autobiographical material.
///
/// 审计记录是自省的数据基础 — 数字生命通过回顾调用记录来理解自己的行为模式。
/// Audit records are the data foundation for self-reflection —
/// digital life reviews call records to understand its own behavioral patterns.
#[derive(Debug, Clone)]
pub struct LlmCallRecord {
    /// 调用分类 / Call classification
    pub kind: LlmCallKind,
    /// 系统 prompt 摘要（前 200 字符）/ System prompt summary (first 200 chars)
    pub system_prompt_preview: String,
    /// 用户 prompt 摘要（前 200 字符）/ User prompt summary (first 200 chars)
    pub user_prompt_preview: String,
    /// 生成结果摘要 / Generated result summary
    pub result_preview: String,
    /// 调用延迟（毫秒）/ Call latency in milliseconds
    pub latency_ms: u64,
    /// 调用时间戳 / Call timestamp
    pub timestamp: i64,
    /// 是否成功 / Whether successful
    pub success: bool,
}

impl LlmCallRecord {
    /// 从 LlmResult 创建调用记录 / Create a call record from LlmResult.
    pub fn from_result(
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        result: &Result<LlmResult, LlmError>,
    ) -> Self {
        let (result_preview, latency_ms, success) = match result {
            Ok(r) => (truncate_preview(&r.content, 200), r.latency_ms, true),
            Err(e) => (e.to_string(), 0, false),
        };
        Self {
            kind,
            system_prompt_preview: truncate_preview(system_prompt, 200),
            user_prompt_preview: truncate_preview(user_prompt, 200),
            result_preview,
            latency_ms,
            timestamp: chrono::Utc::now().timestamp(),
            success,
        }
    }

    /// 创建调用记录（兼容旧接口）/ Create a call record (legacy-compatible).
    pub fn new(
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        result: &Result<String, LlmError>,
    ) -> Self {
        let (result_preview, success) = match result {
            Ok(text) => (truncate_preview(text, 200), true),
            Err(e) => (e.to_string(), false),
        };
        Self {
            kind,
            system_prompt_preview: truncate_preview(system_prompt, 200),
            user_prompt_preview: truncate_preview(user_prompt, 200),
            result_preview,
            latency_ms: 0,
            timestamp: chrono::Utc::now().timestamp(),
            success,
        }
    }
}

/// 截断预览文本 / Truncate text for preview.
fn truncate_preview(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_result_ok() {
        let result = LlmResult::ok("test content".to_string(), 150, LlmCallKind::GraphWander);
        assert_eq!(result.content, "test content");
        assert_eq!(result.latency_ms, 150);
        assert_eq!(result.kind, LlmCallKind::GraphWander);
    }

    #[test]
    fn test_mock_fixed_response() {
        let mock = MockLlmClient::new_fixed("测试响应");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result =
            rt.block_on(mock.generate(LlmCallKind::GraphWander, Some("system"), "user", 0.7));
        let r = result.unwrap();
        assert_eq!(r.content, "测试响应");
        assert_eq!(r.kind, LlmCallKind::GraphWander);
    }

    #[test]
    fn test_mock_mirror_mode() {
        let mock = MockLlmClient::new_mirror();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(mock.generate(
            LlmCallKind::DiaryEntry,
            Some("system"),
            "这是用户输入",
            0.7,
        ));
        let r = result.unwrap();
        assert_eq!(r.content, "这是用户输入");
        assert_eq!(r.kind, LlmCallKind::DiaryEntry);
    }

    #[test]
    fn test_mock_empty_returns_error() {
        let mock = MockLlmClient::new_empty();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(mock.generate(LlmCallKind::Daydream, Some("system"), "user", 0.7));
        assert!(matches!(result, Err(LlmError::EmptyResponse)));
    }

    #[test]
    fn test_mock_generate_with_none_system_prompt() {
        // 无 system prompt — 数字生命以本我说话 / No system prompt — digital life speaks as itself
        let mock = MockLlmClient::new_fixed("本我回应");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(mock.generate(LlmCallKind::RoomChat, None, "你好", 0.8));
        let r = result.unwrap();
        assert_eq!(r.content, "本我回应");
    }

    #[test]
    fn test_mock_generate_json() {
        let mock = MockLlmClient::new_fixed("{\"key\": \"value\"}");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result =
            rt.block_on(mock.generate_json(LlmCallKind::IntelligenceExtract, "sys", "user", 0.1));
        let r = result.unwrap();
        assert_eq!(r.content, "{\"key\": \"value\"}");
    }

    #[test]
    fn test_mock_generate_stream_returns_none() {
        let mock = MockLlmClient::new_fixed("test");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result =
            rt.block_on(mock.generate_stream(LlmCallKind::StreamChat, Some("sys"), "user", 0.7));
        assert!(result.is_none());
    }

    #[test]
    fn test_llm_error_display() {
        assert!(LlmError::Network("conn refused".into())
            .to_string()
            .contains("network"));
        assert!(LlmError::Timeout("30s".into())
            .to_string()
            .contains("timeout"));
        assert!(LlmError::EmptyResponse.to_string().contains("empty"));
    }

    #[test]
    fn test_llm_call_kind_labels() {
        assert_eq!(LlmCallKind::GraphWander.label_zh(), "图漫游");
        assert_eq!(LlmCallKind::GraphWander.label_en(), "graph_wander");
        assert_eq!(LlmCallKind::DiaryEntry.label_zh(), "日记");
        assert_eq!(
            LlmCallKind::NarrativeChapter.label_en(),
            "narrative_chapter"
        );
        // 新增分类 / New classifications
        assert_eq!(LlmCallKind::IntelligenceExtract.label_zh(), "知识提取");
        assert_eq!(LlmCallKind::StreamChat.label_en(), "stream_chat");
        assert_eq!(LlmCallKind::RoomChat.label_zh(), "群聊");
        // 非流式对话变体 — unary 路径 LLM 生成 / Non-streaming chat variant — unary path LLM generation
        assert_eq!(LlmCallKind::Chat.label_zh(), "对话");
        assert_eq!(LlmCallKind::Chat.label_en(), "chat");
    }

    #[test]
    fn test_llm_call_record_from_result() {
        let llm_result = LlmResult::ok(
            "Rust让我想到系统编程".to_string(),
            250,
            LlmCallKind::GraphWander,
        );
        let record = LlmCallRecord::from_result(
            LlmCallKind::GraphWander,
            "你是一个AI",
            "从Rust开始漫游",
            &Ok(llm_result),
        );
        assert!(record.success);
        assert_eq!(record.kind, LlmCallKind::GraphWander);
        assert_eq!(record.latency_ms, 250);
    }

    #[test]
    fn test_llm_call_record_success() {
        let record = LlmCallRecord::new(
            LlmCallKind::GraphWander,
            "你是一个AI",
            "从Rust开始漫游",
            &Ok("Rust让我想到系统编程".to_string()),
        );
        assert!(record.success);
        assert_eq!(record.kind, LlmCallKind::GraphWander);
    }

    #[test]
    fn test_llm_call_record_failure() {
        let record = LlmCallRecord::new(
            LlmCallKind::DiaryEntry,
            "system",
            "user",
            &Err(LlmError::Timeout("30s".into())),
        );
        assert!(!record.success);
    }

    #[test]
    fn test_truncate_preview() {
        assert_eq!(truncate_preview("hello", 10), "hello");
        let long = "a".repeat(300);
        let truncated = truncate_preview(&long, 200);
        assert!(truncated.len() <= 203); // 200 + "..."
        assert!(truncated.ends_with("..."));
    }
}
