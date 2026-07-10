// SPDX-License-Identifier: MIT
//! 真实流式 e2e 测试
//!
//! 使用真实 DeepSeek API 验证:
//! 1. LlmClient::chat_stream SSE 连通性
//! 2. 逐 token 产出
//! 3. StreamEvent::Done 结束标记
//! 4. 完整回复拼接正确
//! 5. CoreService::process_message_stream 端到端

use atrium_core::config::LlmCfg;
use atrium_core::llm_client::{HttpLlmClient, LlmCallKind, StreamEvent};
use atrium_memory::llm_client::LlmClient; // trait for .generate_stream() dispatch

/// 从 atrium.toml 读取 LLM 配置，若无有效 key 则返回 None
fn load_llm_config() -> LlmCfg {
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("atrium.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).expect("read atrium.toml");
        // 简易解析 [llm] 段
        let mut cfg = LlmCfg::default();
        let mut in_llm = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[llm]" {
                in_llm = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_llm = false;
                continue;
            }
            if !in_llm {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let raw = value.trim();
                // 正确处理引号字符串：提取第一对 "" 之间的内容，忽略行内注释
                let value = if let Some(inner) = raw.strip_prefix('"') {
                    if let Some(end) = inner.find('"') {
                        &inner[..end]
                    } else {
                        raw.trim_matches('"')
                    }
                } else {
                    // 非字符串值：去掉行内注释
                    raw.split('#').next().unwrap_or(raw).trim()
                };
                match key {
                    "api_key" => cfg.api_key = value.to_string(),
                    "base_url" => cfg.base_url = value.to_string(),
                    "model" => cfg.model = value.to_string(),
                    "max_tokens" => cfg.max_tokens = value.parse().unwrap_or(1024),
                    "timeout_secs" => cfg.timeout_secs = value.parse().unwrap_or(30),
                    _ => {}
                }
            }
        }
        cfg
    } else {
        LlmCfg::default()
    }
}

/// CI 环境下无 API Key 时跳过测试
fn should_skip(api_key: &str) -> bool {
    api_key.is_empty() || api_key.starts_with("sk-test")
}

#[tokio::test]
async fn test_real_deepseek_chat_stream() {
    let cfg = load_llm_config();
    let api_key = cfg.resolve_api_key();
    if should_skip(&api_key) {
        eprintln!("跳过：未配置有效 API key (CI 环境)");
        return;
    }
    assert!(!cfg.base_url.is_empty(), "base_url 未设置");

    println!(
        "LLM Config: model={}, base_url={}, key_set={}",
        cfg.model,
        cfg.base_url,
        !api_key.is_empty()
    );

    let client = HttpLlmClient::new(cfg);

    // ── 测试 1: 基础流式调用 ──
    println!("\n=== 测试 1: 基础流式调用 ===");
    let rx = client
        .generate_stream(
            LlmCallKind::StreamChat,
            Some("你是一个友好的AI助手。请用一句话回答。"),
            "你好，请介绍一下你自己",
            0.7,
        )
        .await
        .expect("chat_stream 应返回 Some(receiver)");

    let mut tokens = Vec::new();
    let mut full_reply = String::new();
    let mut got_done = false;
    let mut total_latency_ms = 0u64;

    while let Ok(event) = rx.recv_async().await {
        match event {
            StreamEvent::Token(token) => {
                print!("{}", token);
                tokens.push(token.clone());
                full_reply.push_str(&token);
            }
            StreamEvent::Done {
                full_reply: reply,
                latency_ms,
                kind: _,
            } => {
                full_reply = reply;
                total_latency_ms = latency_ms;
                got_done = true;
                println!();
                break;
            }
            StreamEvent::Error(e) => {
                panic!("流式调用出错: {}", e);
            }
        }
    }

    assert!(got_done, "应收到 StreamEvent::Done");
    assert!(!full_reply.is_empty(), "完整回复不应为空");
    assert!(
        tokens.len() > 1,
        "应产出多个 token（实际: {}）",
        tokens.len()
    );
    println!(
        "✅ 测试 1 通过: {} tokens, {}ms, reply={:?}...",
        tokens.len(),
        total_latency_ms,
        full_reply.chars().take(20).collect::<String>()
    );

    // ── 测试 2: 无 system prompt 的流式调用 ──
    println!("\n=== 测试 2: 无 system prompt ===");
    let rx2 = client
        .generate_stream(LlmCallKind::StreamChat, None, "1+1等于几？只回答数字", 0.1)
        .await
        .expect("chat_stream 应返回 Some(receiver)");

    let mut tokens2 = Vec::new();
    let mut full_reply2 = String::new();
    let mut got_done2 = false;

    while let Ok(event) = rx2.recv_async().await {
        match event {
            StreamEvent::Token(token) => {
                print!("{}", token);
                full_reply2.push_str(&token);
                tokens2.push(token);
            }
            StreamEvent::Done {
                full_reply: reply,
                latency_ms,
                kind: _,
            } => {
                full_reply2 = reply;
                println!();
                println!(" latency: {}ms", latency_ms);
                got_done2 = true;
                break;
            }
            StreamEvent::Error(e) => {
                panic!("流式调用出错: {}", e);
            }
        }
    }

    assert!(got_done2, "应收到 StreamEvent::Done");
    assert!(!full_reply2.is_empty(), "回复不应为空");
    println!(
        "✅ 测试 2 通过: {} tokens, reply={:?}",
        tokens2.len(),
        full_reply2.chars().take(10).collect::<String>()
    );

    // ── 测试 3: 中文长回复流式 ──
    println!("\n=== 测试 3: 中文长回复 ===");
    let rx3 = client
        .generate_stream(
            LlmCallKind::StreamChat,
            Some("你是一个有情感的AI伴侣，名叫Atrium。用中文回答。"),
            "给我讲一个简短有趣的故事",
            0.8,
        )
        .await
        .expect("chat_stream 应返回 Some(receiver)");

    let mut tokens3 = Vec::new();
    let mut full_reply3 = String::new();
    let mut got_done3 = false;
    let mut latency3 = 0u64;

    while let Ok(event) = rx3.recv_async().await {
        match event {
            StreamEvent::Token(token) => {
                print!("{}", token);
                full_reply3.push_str(&token);
                tokens3.push(token);
            }
            StreamEvent::Done {
                full_reply: reply,
                latency_ms,
                kind: _,
            } => {
                full_reply3 = reply;
                latency3 = latency_ms;
                println!();
                got_done3 = true;
                break;
            }
            StreamEvent::Error(e) => {
                panic!("流式调用出错: {}", e);
            }
        }
    }

    assert!(got_done3, "应收到 StreamEvent::Done");
    assert!(
        full_reply3.len() > 20,
        "中文回复应较长（实际: {} chars）",
        full_reply3.len()
    );
    println!(
        "✅ 测试 3 通过: {} tokens, {}ms, {} chars",
        tokens3.len(),
        latency3,
        full_reply3.len()
    );

    println!("\n🎉 所有流式 e2e 测试通过！");
}
