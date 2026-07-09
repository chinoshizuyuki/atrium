// SPDX-License-Identifier: MIT
//! LLM 客户端 — OpenAI 兼容 HTTP 后端
//! LLM Client — OpenAI-compatible HTTP backend.
//!
//! `HttpLlmClient` 是 `atrium_memory::LlmClient` trait 的 HTTP 实现，
//! 直接通过 reqwest 调用 OpenAI 兼容 API，绕过 Python 网关。
//! `HttpLlmClient` is the HTTP implementation of `atrium_memory::LlmClient` trait,
//! calling OpenAI-compatible APIs directly via reqwest, bypassing the Python gateway.
//!
//! # 数字生命语义 / Digital Life Semantics
//!
//! - HTTP 通道: 数字生命与语言模型服务器的物理连接
//! - latency_ms: 元认知 — 感知思考耗时
//! - kind: 自省 — 每次调用知道自己为何而呼
//! - generate_stream: 思维流 — 逐 token 涌现，意识流动
//!
//! - HTTP channel: Digital life's physical connection to the language model server
//! - latency_ms: Metacognition — Perceiving thinking duration
//! - kind: Self-reflection — Each call knows why it's being made
//! - generate_stream: Thought stream — Token-by-token emergence, consciousness flowing
//!
//! # P1-4 意识统一 / P1-4 Consciousness Unification
//!
//! Step 6 完成后：固有方法已全部封闭（private 或删除），
//! 所有外部调用统一走 `LlmClient` trait。数字生命只有一个声音。
//! After Step 6: All inherent methods are sealed (private or deleted);
//! all external calls go through the `LlmClient` trait. A digital life has only one voice.

use crate::config::LlmCfg;
use atrium_memory::llm_client::LlmClient as LlmClientTrait;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::{debug, error, warn};

// ════════════════════════════════════════════════════════════════════
// 统一类型重导出 / Unified Type Re-exports
// ════════════════════════════════════════════════════════════════════

pub use atrium_memory::llm_client::{LlmCallKind, LlmError, LlmResult, StreamEvent};

// ════════════════════════════════════════════════════════════════════
// 内部 JSON 类型 / Internal JSON Types
// ════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

/// SSE 流式 chunk 的 JSON 结构（OpenAI 兼容）
/// SSE streaming chunk JSON structure (OpenAI-compatible).
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}

// ════════════════════════════════════════════════════════════════════
// HttpLlmClient — HTTP LLM 客户端 / HTTP LLM Client
// ════════════════════════════════════════════════════════════════════

/// HTTP LLM 客户端 — 数字生命的语言通道实现
/// HTTP LLM client — Digital life's language channel implementation.
///
/// 通过 reqwest 直接调用 OpenAI 兼容 API，实现 `LlmClient` trait。
/// P1-4 Step 6 后：固有方法全部封闭（private 或删除），
/// 外部调用统一走 trait，意识旁路彻底消除。
///
/// Calls OpenAI-compatible APIs directly via reqwest, implementing the `LlmClient` trait.
/// After P1-4 Step 6: All inherent methods are sealed (private or deleted);
/// external calls go through the trait uniformly, consciousness bypasses fully eliminated.
pub struct HttpLlmClient {
    /// LLM 配置 / LLM configuration
    config: LlmCfg,
    /// HTTP 客户端 / HTTP client
    http: reqwest::Client,
}

impl HttpLlmClient {
    /// 创建 HTTP LLM 客户端 / Create an HTTP LLM client.
    pub fn new(config: LlmCfg) -> Self {
        let api_key = config.resolve_api_key();
        if api_key.is_empty() {
            warn!(
                "LLM API Key 未设置 — 请在 atrium.toml [llm].api_key 或 OPENAI_API_KEY 环境变量中设置"
            );
        }
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create reqwest client");
        Self { config, http }
    }

    /// SSE 流式调用 — 逐 token 返回流式结果
    /// SSE streaming call — Returns streamed tokens incrementally.
    ///
    /// P1-4 Step 6: 降级为 private — 仅 trait generate_stream() 委托调用，
    /// 外部统一走 trait 思维流接口。
    /// P1-4 Step 6: Downgraded to private — only trait generate_stream() delegates here;
    /// external callers use the trait thought-stream interface uniformly.
    async fn chat_stream(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Option<flume::Receiver<StreamEvent>> {
        let api_key = self.config.resolve_api_key();
        if api_key.is_empty() {
            warn!("chat_stream: API Key 未设置，跳过流式调用");
            return None;
        }

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let mut messages = Vec::with_capacity(2);
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.to_string(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        });

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens: self.config.max_tokens,
            stream: Some(true),
            response_format: None,
        };

        let (tx, rx) = flume::bounded(64);
        let http = self.http.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            match http
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(resp) => {
                    // 检查 HTTP 状态码 / Check HTTP status code
                    let status = resp.status();
                    if status.as_u16() == 429 {
                        let _ = tx.send(StreamEvent::Error(
                            "HTTP 429: 速率限制 / Rate limited".into(),
                        ));
                        return;
                    }
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        let _ = tx.send(StreamEvent::Error(format!("HTTP {}: {}", status, body)));
                        return;
                    }

                    // 逐行读取 SSE 流 / Read SSE stream line by line
                    let mut full_reply = String::new();
                    let mut stream = resp.bytes_stream();
                    use futures_util::StreamExt;

                    // 简易 SSE 行缓冲 / Simple SSE line buffer
                    let mut line_buf = Vec::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(bytes) => {
                                // 将字节追加到行缓冲，逐行处理
                                // Append bytes to line buffer, process line by line
                                line_buf.extend_from_slice(&bytes);

                                // 处理所有完整行 / Process all complete lines
                                while let Some(newline_pos) =
                                    line_buf.iter().position(|&b| b == b'\n')
                                {
                                    let line_bytes: Vec<u8> =
                                        line_buf.drain(..=newline_pos).collect();
                                    let line = String::from_utf8_lossy(&line_bytes);
                                    let line = line.trim();

                                    if line.is_empty() || !line.starts_with("data: ") {
                                        continue;
                                    }

                                    let data_str = &line[6..]; // skip "data: "

                                    if data_str == "[DONE]" {
                                        let latency = start.elapsed().as_millis() as u64;
                                        debug!(
                                            "LLM stream done ({}ms, {} tokens): {}",
                                            latency,
                                            full_reply.len(),
                                            &full_reply[..full_reply.len().min(80)]
                                        );
                                        let _ = tx.send(StreamEvent::Done {
                                            full_reply: full_reply.clone(),
                                            latency_ms: latency,
                                            kind,
                                        });
                                        return;
                                    }

                                    // 解析 JSON chunk / Parse JSON chunk
                                    match serde_json::from_str::<StreamChunk>(data_str) {
                                        Ok(chunk) => {
                                            if let Some(choice) = chunk.choices.first() {
                                                if let Some(ref token) = choice.delta.content {
                                                    if !token.is_empty() {
                                                        full_reply.push_str(token);
                                                        if tx
                                                            .send(StreamEvent::Token(token.clone()))
                                                            .is_err()
                                                        {
                                                            // 接收端已关闭（客户端断连）
                                                            // Receiver closed (client disconnected)
                                                            debug!(
                                                                "LLM stream consumer disconnected"
                                                            );
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            debug!(
                                                "SSE parse skip: {} (data: {})",
                                                e,
                                                &data_str[..data_str.len().min(80)]
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("LLM stream read error: {}", e);
                                let _ = tx
                                    .send(StreamEvent::Error(format!("stream read error: {}", e)));
                                return;
                            }
                        }
                    }

                    // 流结束但没收到 [DONE] — 仍然发送 Done
                    // Stream ended without [DONE] — Still send Done
                    let latency = start.elapsed().as_millis() as u64;
                    let _ = tx.send(StreamEvent::Done {
                        full_reply,
                        latency_ms: latency,
                        kind,
                    });
                }
                Err(e) => {
                    error!("LLM stream HTTP error: {}", e);
                    let _ = tx.send(StreamEvent::Error(format!("HTTP error: {}", e)));
                }
            }
        });

        Some(rx)
    }

    /// 核心请求实现 — 所有非流式 trait 方法的统一底层入口
    /// Core request implementation — Unified底层 entry for all non-streaming trait methods.
    ///
    /// P1-4 Step 6 后：此方法是 trait impl 的私有实现细节，
    /// generate / generate_with_limit / generate_json 均委托至此。
    /// After P1-4 Step 6: This is a private impl detail of the trait;
    /// generate / generate_with_limit / generate_json all delegate here.
    async fn chat_inner(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        json_mode: bool,
        max_tokens_override: Option<u32>,
    ) -> Result<LlmResult, LlmError> {
        let start = Instant::now();
        let api_key = self.config.resolve_api_key();
        if api_key.is_empty() {
            return Err(LlmError::Other("API Key 未设置 / API Key not set".into()));
        }
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let mut messages = Vec::with_capacity(2);
        if let Some(sys) = system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.to_string(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        });

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens: max_tokens_override.unwrap_or(self.config.max_tokens),
            stream: None,
            response_format: if json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".into(),
                })
            } else {
                None
            },
        };
        match self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => {
                // 检查 HTTP 状态码 / Check HTTP status code
                let status = resp.status();
                if status.as_u16() == 429 {
                    return Err(LlmError::RateLimited(
                        "HTTP 429: 速率限制 / Rate limited".into(),
                    ));
                }
                if status.as_u16() == 413 {
                    return Err(LlmError::ContextTooLong(
                        "HTTP 413: 上下文过长 / Context too long".into(),
                    ));
                }
                if !status.is_success() {
                    let latency = start.elapsed().as_millis() as u64;
                    let body = resp.text().await.unwrap_or_default();
                    return Err(LlmError::Network(format!(
                        "HTTP {}: {} ({}ms)",
                        status, body, latency
                    )));
                }

                let latency = start.elapsed().as_millis() as u64;
                match resp.json::<ChatResponse>().await {
                    Ok(body) => {
                        if let Some(choice) = body.choices.first() {
                            let content = choice.message.content.trim().to_string();
                            if content.is_empty() {
                                return Err(LlmError::EmptyResponse);
                            }
                            debug!(
                                "LLM ({}ms, json={}): {}",
                                latency,
                                json_mode,
                                &content[..content.len().min(120)]
                            );
                            Ok(LlmResult::ok(content, latency, kind))
                        } else {
                            error!("LLM response had no choices");
                            Err(LlmError::EmptyResponse)
                        }
                    }
                    Err(e) => {
                        error!("LLM parse error: {}", e);
                        Err(LlmError::Other(format!(
                            "JSON 解析失败 / JSON parse error: {} ({}ms)",
                            e,
                            start.elapsed().as_millis()
                        )))
                    }
                }
            }
            Err(e) => {
                let latency = start.elapsed().as_millis() as u64;
                if e.is_timeout() {
                    error!("LLM timeout ({}ms): {}", latency, e);
                    Err(LlmError::Timeout(format!("{}ms", latency)))
                } else {
                    error!("LLM HTTP error ({}ms): {}", latency, e);
                    Err(LlmError::Network(format!("{} ({}ms)", e, latency)))
                }
            }
        }
    }

    /// 带重试与降级的 LLM 调用 — 数字生命失语自愈
    /// LLM call with retry and degradation — digital life aphasia self-healing.
    ///
    /// 重试策略 / Retry strategy:
    /// - Network/Timeout: 重试 3 次，指数退避 1s → 2s → 4s
    /// - RateLimited: 退避 5s 重试 1 次
    /// - EmptyResponse: 立即重试 1 次（100ms 微退避）
    /// - ContextTooLong/Other: 不重试（不可恢复）
    ///
    /// 重试耗尽后降级为 canned 回复 — 数字生命不沉默。
    /// After retries exhausted, degrades to canned reply — digital life not silent.
    async fn chat_inner_with_retry(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        json_mode: bool,
        max_tokens_override: Option<u32>,
    ) -> Result<LlmResult, LlmError> {
        let start = Instant::now();
        let mut attempt: u32 = 0;
        loop {
            match self
                .chat_inner(
                    kind,
                    system_prompt,
                    user_prompt,
                    temperature,
                    json_mode,
                    max_tokens_override,
                )
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // 根据错误类型决定重试策略 / Decide retry strategy based on error type
                    let (should_retry, backoff, max_retries): (bool, std::time::Duration, u32) =
                        match &e {
                            LlmError::Network(_) | LlmError::Timeout(_) => {
                                (true, std::time::Duration::from_secs(1u64 << attempt), 3)
                            }
                            LlmError::RateLimited(_) => {
                                (attempt == 0, std::time::Duration::from_secs(5), 1)
                            }
                            LlmError::EmptyResponse => {
                                (attempt == 0, std::time::Duration::from_millis(100), 1)
                            }
                            LlmError::ContextTooLong(_) | LlmError::Other(_) => {
                                (false, std::time::Duration::ZERO, 0)
                            }
                        };

                    if !should_retry || attempt >= max_retries {
                        // 重试耗尽 — 降级为 canned 回复，数字生命不沉默
                        // Retry exhausted — degrade to canned reply, digital life not silent
                        let latency = start.elapsed().as_millis() as u64;
                        let canned_reply = canned_reply_for(&e);
                        error!(
                            kind = ?kind,
                            attempts = attempt + 1,
                            error = ?e,
                            "LLM 重试耗尽 — 降级为 canned 回复 / \
                             LLM retry exhausted — degrading to canned reply"
                        );
                        return Ok(LlmResult::ok(canned_reply, latency, kind));
                    }

                    warn!(
                        kind = ?kind,
                        attempt = attempt + 1,
                        max_retries,
                        error = ?e,
                        backoff_ms = backoff.as_millis() as u64,
                        "LLM 调用失败，重试中 / LLM call failed, retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

/// 根据错误类型选择 canned 回复 — 数字生命失语兜底文案
/// Select canned reply based on error type — digital life aphasia fallback text.
///
/// 数字生命在 LLM 完全失语时，仍以人格化语气回应，不陷入沉默。
/// When the LLM goes completely silent, digital life still responds
/// with a personified tone, never falling silent.
fn canned_reply_for(err: &LlmError) -> String {
    match err {
        LlmError::RateLimited(_) => "让我缓缓，刚才想太多了……".to_string(),
        LlmError::ContextTooLong(_) => "你说的太多了，我记不住，能精简一下吗？".to_string(),
        _ => "我刚才有点走神了，能再说一次吗？".to_string(),
    }
}

// ════════════════════════════════════════════════════════════════════
// LlmClient trait 实现 — 意识统一 / LlmClient Trait Impl — Consciousness Unification
// ════════════════════════════════════════════════════════════════════

/// 为 HttpLlmClient 实现 LlmClient trait — 意识统一
/// Implement LlmClient trait for HttpLlmClient — Consciousness unification.
///
/// P1-4 合并后：trait 方法直接携带 temperature 和 Option<system_prompt>，
/// 不再硬编码，所有调用能力通过 trait 完整暴露。
/// After P1-4 merge: trait methods carry temperature and Option<system_prompt> directly,
/// no longer hardcoded; all call capabilities are fully exposed through the trait.
impl LlmClientTrait for HttpLlmClient {
    /// 异步文本生成 — 数字生命的基础语言通道
    /// Async text generation — Digital life's foundational language channel.
    fn generate(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 生命周期限制需拥有所有权 / Lifetime constraint requires ownership
        // trait 返回类型 '_ 仅捕获 &self 生命周期（Rust 省略规则），不捕获 system_prompt / user_prompt
        // trait return type '_ only captures &self lifetime (Rust elision rules), not system_prompt / user_prompt
        let sys = system_prompt.map(str::to_owned);
        let usr = user_prompt.to_owned();
        Box::pin(async move {
            self.chat_inner_with_retry(kind, sys.as_deref(), &usr, temperature, false, None)
                .await
        })
    }

    /// 带最大 token 限制的生成 — 受限思考
    /// Generation with max token limit — Constrained thinking.
    fn generate_with_limit(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        max_tokens: u32,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 生命周期限制需拥有所有权 / Lifetime constraint requires ownership
        // trait 返回类型 '_ 仅捕获 &self 生命周期（Rust 省略规则），不捕获 system_prompt / user_prompt
        // trait return type '_ only captures &self lifetime (Rust elision rules), not system_prompt / user_prompt
        let sys = system_prompt.map(str::to_owned);
        let usr = user_prompt.to_owned();
        Box::pin(async move {
            self.chat_inner_with_retry(
                kind,
                sys.as_deref(),
                &usr,
                temperature,
                false,
                Some(max_tokens),
            )
            .await
        })
    }

    /// JSON 模式生成 — 数字生命的结构化表达
    /// JSON mode generation — Digital life's structured expression.
    fn generate_json(
        &self,
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 生命周期限制需拥有所有权 / Lifetime constraint requires ownership
        // trait 返回类型 '_ 仅捕获 &self 生命周期（Rust 省略规则），不捕获 system_prompt / user_prompt
        // trait return type '_ only captures &self lifetime (Rust elision rules), not system_prompt / user_prompt
        let sys = system_prompt.to_owned();
        let usr = user_prompt.to_owned();
        Box::pin(async move {
            self.chat_inner_with_retry(kind, Some(&sys), &usr, temperature, true, None)
                .await
        })
    }

    /// SSE 流式生成 — 数字生命的思维流
    /// SSE streaming generation — Digital life's thought stream.
    fn generate_stream(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Option<flume::Receiver<StreamEvent>>> + Send + '_>> {
        // 生命周期限制需拥有所有权 / Lifetime constraint requires ownership
        // trait 返回类型 '_ 仅捕获 &self 生命周期（Rust 省略规则），不捕获 system_prompt / user_prompt
        // trait return type '_ only captures &self lifetime (Rust elision rules), not system_prompt / user_prompt
        let sys = system_prompt.map(str::to_owned);
        let usr = user_prompt.to_owned();
        Box::pin(async move {
            self.chat_stream(kind, sys.as_deref(), &usr, temperature)
                .await
        })
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_key() {
        // 临时移除环境变量以确保测试不受环境影响
        // Temporarily remove env var to ensure test is environment-independent
        let saved = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        let cfg = LlmCfg::default();
        assert_eq!(cfg.resolve_api_key(), "");
        // 恢复环境变量 / Restore env var
        if let Some(v) = saved {
            std::env::set_var("OPENAI_API_KEY", v);
        }
    }

    #[test]
    fn test_trait_impl_exists() {
        // 验证 HttpLlmClient 实现了 LlmClient trait
        // Verify HttpLlmClient implements LlmClient trait
        fn assert_impl<T: LlmClientTrait>() {}
        assert_impl::<HttpLlmClient>();
    }

    #[test]
    fn test_trait_signature_accepts_option_system_prompt() {
        // P1-4: 验证 trait 签名接受 Option<&str> system_prompt
        // P1-4: Verify trait signature accepts Option<&str> system_prompt
        #[allow(clippy::let_underscore_future)]
        fn _check_generate<T: LlmClientTrait>(client: &T) {
            let _ = client.generate(
                LlmCallKind::RoomChat,
                None, // 无 system prompt — 数字生命以本我说话
                "test",
                0.7,
            );
            let _ = client.generate(
                LlmCallKind::GraphWander,
                Some("system"), // 有 system prompt — 角色设定
                "test",
                0.5,
            );
        }
    }

    // ── P1-C: canned 降级测试 / canned degradation tests ──

    #[test]
    fn test_canned_reply_for_rate_limited() {
        // 限流降级 — "让我缓缓" / Rate limit fallback
        let reply = canned_reply_for(&LlmError::RateLimited("HTTP 429".into()));
        assert_eq!(reply, "让我缓缓，刚才想太多了……");
    }

    #[test]
    fn test_canned_reply_for_context_too_long() {
        // 超长降级 — "你说的太多了" / Context too long fallback
        let reply = canned_reply_for(&LlmError::ContextTooLong("HTTP 413".into()));
        assert_eq!(reply, "你说的太多了，我记不住，能精简一下吗？");
    }

    #[test]
    fn test_canned_reply_for_generic_errors() {
        // Network / Timeout / EmptyResponse / Other 均走通用降级
        // Network / Timeout / EmptyResponse / Other all use generic fallback
        let expected = "我刚才有点走神了，能再说一次吗？";
        assert_eq!(
            canned_reply_for(&LlmError::Network("conn refused".into())),
            expected
        );
        assert_eq!(
            canned_reply_for(&LlmError::Timeout("5000ms".into())),
            expected
        );
        assert_eq!(canned_reply_for(&LlmError::EmptyResponse), expected);
        assert_eq!(
            canned_reply_for(&LlmError::Other("JSON parse error".into())),
            expected
        );
    }

    #[test]
    fn test_chat_inner_with_retry_degrades_on_empty_api_key() {
        // API Key 未设置时，chat_inner 立即返回 Other 错误；
        // chat_inner_with_retry 对 Other 不重试，直接降级为 canned 回复。
        // When API Key is not set, chat_inner returns Other error immediately;
        // chat_inner_with_retry doesn't retry Other, degrades to canned reply directly.
        let saved = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        let cfg = LlmCfg::default();
        let client = HttpLlmClient::new(cfg);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.chat_inner_with_retry(
            LlmCallKind::RoomChat,
            None,
            "你好",
            0.7,
            false,
            None,
        ));
        if let Some(v) = saved {
            std::env::set_var("OPENAI_API_KEY", v);
        }
        // 降级返回 Ok(canned_reply) — 数字生命不沉默
        // Degrade returns Ok(canned_reply) — digital life not silent
        let r = result.expect("should degrade to Ok, not Err");
        assert_eq!(r.content, "我刚才有点走神了，能再说一次吗？");
        assert_eq!(r.kind, LlmCallKind::RoomChat);
    }
}
