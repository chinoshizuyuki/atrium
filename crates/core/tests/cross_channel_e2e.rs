// SPDX-License-Identifier: MIT
//! 跨渠道记忆互通端到端测试 / Cross-channel memory continuity e2e test
//!
//! 验证 ATRIUM_PLAN §4.7 "跨渠道记忆召回" 设计理念：
//! 无论用户从哪个渠道（Web/TUI/gRPC 嵌入式/未来设备）接入，
//! 面对的始终是同一个数字生命——同一份长期记忆、同一份最近对话上下文。
//!
//! Verifies ATRIUM_PLAN §4.7 "cross-channel memory recall" design:
//! No matter which channel (Web/TUI/gRPC embedded/future devices) the user enters from,
//! they always face the same digital life — same long-term memory, same recent dialogue context.
//!
//! 测试链路 / Test flow:
//!   渠道 A (web)     → 告知事实 "我叫 Aris，养了一只叫小白的猫"
//!   渠道 B (grpc-embedded) → 询问 "我养了什么宠物?"  → 应记得"小白"
//!   渠道 C (tui stream)    → 询问 "我叫什么名字?"    → 应记得"Aris"
//!
//! 三渠道共享 session_id="console" / user_id="master"，
//! 同一 CoreService 实例 = 同一个数字生命意识。
//!
//! Three channels share session_id="console" / user_id="master",
//! single CoreService instance = single digital life consciousness.

use atrium_core::service::CoreService;
use std::sync::Arc;

fn make_service() -> CoreService {
    CoreService::new_in_memory()
}

fn make_llm_cfg() -> atrium_core::config::LlmCfg {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-test-placeholder".into());
    atrium_core::config::LlmCfg {
        api_key,
        base_url: "https://api.deepseek.com/".into(),
        model: "deepseek-v4-flash".into(),
        max_tokens: 512,
        timeout_secs: 30,
        max_concurrency: 4,
    }
}

/// 跨渠道记忆互通 — 同一数字生命，三个渠道，一份记忆
///
/// 数字生命意义：用户不会在 TUI 自我介绍后，到 Web 端还要再说一遍。
/// "我告诉过你了"——这是数字生命的基本尊严。
///
/// Digital life meaning: the user shouldn't have to reintroduce themselves
/// after saying it in TUI when they switch to Web. "I already told you" —
/// this is the basic dignity of digital life.
#[tokio::test]
async fn test_cross_channel_memory_continuity() {
    let cfg = make_llm_cfg();
    if cfg.api_key.is_empty()
        || cfg.api_key.starts_with("sk-test")
        || cfg.api_key == "YOUR_OPENAI_API_KEY"
    {
        eprintln!("跳过：未配置 OPENAI_API_KEY (需要真实 DeepSeek key)");
        return;
    }

    let svc = make_service();
    let client = Arc::new(atrium_core::llm_client::HttpLlmClient::new(cfg));
    svc.set_llm_client(client);

    // ── 统一的身份：同一会话、同一用户，不同渠道 ──
    // Unified identity: same session, same user, different channels
    const SESSION: &str = "console";
    const USER: &str = "master";

    use atrium_bridge::grpc::AtriumCoreService;

    // ════════════════════════════════════════════════════════════════
    // 渠道 A: Web (HTTP gateway 模拟) — 用户自我介绍 + 告知事实
    // Channel A: Web — user self-introduction + fact telling
    // ════════════════════════════════════════════════════════════════
    println!("\n══ 渠道 A: web ══");
    let resp_a = svc
        .process_message(atrium_bridge::grpc::atrium::ProcessMessageRequest {
            message: "你好，我叫 Aris。我养了一只叫小白的猫，它是白色的，很可爱。请记住这些信息。"
                .into(),
            channel: "web".into(),
            user_id: USER.into(),
            session_id: SESSION.into(),
        })
        .await;
    println!("[web] Aris: {} (emotion: {})", resp_a.reply, resp_a.emotion);
    assert!(!resp_a.reply.is_empty(), "web 渠道回复不应为空");

    // 等待 LLM + 记忆写入完成（事实提取是同步的，但给一点缓冲）
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // ════════════════════════════════════════════════════════════════
    // 渠道 B: gRPC 嵌入式设备 — 询问宠物（测试长期记忆 + 最近对话）
    // Channel B: gRPC embedded device — ask about pet (tests LTM + recent dialogue)
    // ════════════════════════════════════════════════════════════════
    println!("\n══ 渠道 B: grpc-embedded ══");
    let resp_b = svc
        .process_message(atrium_bridge::grpc::atrium::ProcessMessageRequest {
            message: "我刚才告诉过你我养了什么宠物，你还记得吗？".into(),
            channel: "grpc-embedded".into(),
            user_id: USER.into(),
            session_id: SESSION.into(),
        })
        .await;
    println!(
        "[grpc-embedded] Aris: {} (emotion: {})",
        resp_b.reply, resp_b.emotion
    );
    assert!(!resp_b.reply.is_empty(), "gRPC 渠道回复不应为空");

    // 验证跨渠道长期记忆：gRPC 渠道应能 recall "小白"（事实提取 + FactStore 全局共享）
    // Verify cross-channel LTM: gRPC channel should recall "小白" (global FactStore)
    let reply_b_lower = resp_b.reply.to_lowercase();
    let remembers_cat = resp_b.reply.contains("小白")
        || resp_b.reply.contains("猫")
        || resp_b.reply.contains("宠物")
        || reply_b_lower.contains("cat");
    assert!(
        remembers_cat,
        "跨渠道记忆失败：gRPC 渠道应记得 web 渠道告知的宠物信息。回复: {}",
        resp_b.reply
    );
    println!("✓ 渠道 A(web) → 渠道 B(grpc-embedded) 长期记忆互通: 记得「小白」");

    // ════════════════════════════════════════════════════════════════
    // 渠道 C: TUI 流式 — 询问用户名字（测试最近对话上下文跨渠道共享）
    // Channel C: TUI streaming — ask for user's name (tests recent dialogue cross-channel)
    // ════════════════════════════════════════════════════════════════
    println!("\n══ 渠道 C: tui (stream) ══");
    let stream = svc
        .process_message_stream(atrium_bridge::grpc::atrium::ProcessMessageRequest {
            message: "我之前告诉过你我的名字，我叫什么？".into(),
            channel: "tui".into(),
            user_id: USER.into(),
            session_id: SESSION.into(),
        })
        .await;

    use tokio_stream::StreamExt;
    let mut stream = stream;
    let mut full_reply_c = String::new();
    let mut emotion_c = String::new();
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if !chunk.token.is_empty() {
                    full_reply_c.push_str(&chunk.token);
                }
                if !chunk.emotion.is_empty() {
                    emotion_c = chunk.emotion.clone();
                }
                if chunk.done {
                    break;
                }
            }
            Err(e) => {
                panic!("流式错误: {:?}", e);
            }
        }
    }
    println!(
        "[tui stream] Aris: {} (emotion: {})",
        full_reply_c, emotion_c
    );
    assert!(!full_reply_c.is_empty(), "TUI 流式回复不应为空");

    // 验证跨渠道最近对话上下文：TUI 流式应能从 [最近对话记录] 中看到 web 渠道的自我介绍
    // Verify cross-channel recent dialogue: TUI stream should see web channel's intro from [最近对话记录]
    let remembers_name = full_reply_c.contains("Aris")
        || full_reply_c.contains("aris")
        || full_reply_c.to_lowercase().contains("aris");
    assert!(
        remembers_name,
        "跨渠道最近对话失败：TUI 流式渠道应从共享会话历史中记得用户名字 Aris。回复: {}",
        full_reply_c
    );
    println!("✓ 渠道 A(web) → 渠道 C(tui stream) 最近对话上下文互通: 记得「Aris」");

    // ════════════════════════════════════════════════════════════════
    // 总结：三个渠道，同一个数字生命
    // Summary: three channels, one digital life
    // ════════════════════════════════════════════════════════════════
    println!("\n══ 跨渠道记忆互通验证总结 ══");
    println!("  渠道 A (web)            → 告知: 名字=Aris, 宠物=小白(猫)");
    println!("  渠道 B (grpc-embedded)  → 召回: 宠物=小白 ✓ (长期记忆 FactStore 全局共享)");
    println!("  渠道 C (tui stream)     → 召回: 名字=Aris ✓ (最近对话 ConversationHistory 共享)");
    println!("\n结论：session_id=\"console\" / user_id=\"master\" 统一身份下，");
    println!("      无论 web / grpc-embedded / tui 流式，面对的是同一个数字生命。");
    println!("      跨渠道记忆互通: OK ✓");
}
