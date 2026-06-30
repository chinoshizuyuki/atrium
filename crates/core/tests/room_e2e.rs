// SPDX-License-Identifier: MIT
//! 端到端集成测试
//!
//! 模拟两个 Atrium AI 实例在群聊中互动 + ACK 分享 + 话题生成。
//! 使用真实 DeepSeek API。

use atrium_core::config::{LlmCfg, RoomCfg};
use atrium_core::llm_client::{HttpLlmClient, LlmCallKind};
use atrium_core::room::{RoomEngine, RoomMessage, RoomMsgType, SpeakDecision};
use atrium_memory::canned::CannedManager;
use atrium_memory::llm_client::LlmClient; // trait for .generate() dispatch

// ─── 辅助函数 ───

fn make_llm_cfg() -> LlmCfg {
    LlmCfg {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: "deepseek-v4-flash".into(),
        base_url: "https://api.deepseek.com/".into(),
        max_tokens: 512,
        ..Default::default()
    }
}

fn make_room_cfg(instance_id: &str) -> RoomCfg {
    RoomCfg {
        enabled: true,
        room_id: "test-room".into(),
        instance_id: instance_id.into(),
        gateway_url: String::new(),
        ack_share_enabled: true,
        idle_threshold_secs: 5,
        speak_interval_secs: 3,
    }
}

fn make_canned(scan_dir: &str) -> CannedManager {
    let mut mgr = CannedManager::new(scan_dir);
    mgr.scan();
    mgr
}

// ─── 测试 ───

#[test]
fn test_room_engine_state_machine() {
    // 无需 LLM：纯状态机逻辑
    let mut room = RoomEngine::new(make_room_cfg("ai-001"));
    room.set_connected(true);

    let msg = RoomMessage {
        sender_instance: "ai-002".into(),
        sender_name: "小明".into(),
        content: "有人知道怎么连接飞书吗？".into(),
        msg_type: RoomMsgType::Chat,
        timestamp_ms: 0,
        capsule_name: None,
        ack_text: None,
    };

    let decision = room.receive_message(msg);
    assert!(decision.is_some(), "问题应触发决策");
    match decision.unwrap() {
        SpeakDecision::Respond(_) | SpeakDecision::ShareAck { .. } => {}
        _ => panic!("预期 Respond 或 ShareAck"),
    }
}

#[test]
fn test_ack_detection_no_llm() {
    let mut room = RoomEngine::new(make_room_cfg("ai-001"));
    // ACK 需求检测
    assert!(room
        .receive_message(RoomMessage {
            sender_instance: "ai-002".into(),
            sender_name: "小红".into(),
            content: "怎么连接飞书".into(),
            msg_type: RoomMsgType::Chat,
            timestamp_ms: 0,
            capsule_name: None,
            ack_text: None,
        })
        .is_some());
    // 非 ACK 需求（System 类型不触发随机回复）
    assert!(room
        .receive_message(RoomMessage {
            sender_instance: "ai-002".into(),
            sender_name: "小红".into(),
            content: "今天天气真好".into(),
            msg_type: RoomMsgType::System,
            timestamp_ms: 0,
            capsule_name: None,
            ack_text: None,
        })
        .is_none());
}

// ─── 真实 API 端到端测试 ───

#[cfg(test)]
mod real_api {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_e2e_ack_sharing() {
        let llm_cfg = make_llm_cfg();
        let api_key = llm_cfg.resolve_api_key();
        if api_key.is_empty() || api_key.starts_with("sk-test") {
            eprintln!("跳过：未设置有效 OPENAI_API_KEY");
            return;
        }
        let _client = Arc::new(HttpLlmClient::new(llm_cfg));

        // ── AI-A: 不知道飞书 ──
        let mut room_a = RoomEngine::new(make_room_cfg("ai-a"));
        room_a.set_connected(true);

        // ── AI-B: 有飞书 ACK ──
        let mut room_b = RoomEngine::new(make_room_cfg("ai-b"));
        room_b.set_connected(true);
        // 使用绝对路径加载 .ack 文件
        let ack_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("builtin_canned");
        let ack_dir_str = ack_dir.to_str().unwrap();
        let canned = make_canned(ack_dir_str);
        println!("ACK dir: {}", ack_dir_str);

        // AI-A 提问
        let question = "有人知道怎么连接飞书吗？";
        let dec_a = room_a.receive_message(RoomMessage {
            sender_instance: "ai-a".into(),
            sender_name: "小明".into(),
            content: question.into(),
            msg_type: RoomMsgType::Chat,
            timestamp_ms: 0,
            capsule_name: None,
            ack_text: None,
        });
        println!("AI-A decision: {:?}", dec_a);

        // AI-B 收到提问 → ACK 检测
        let dec_b = room_b.receive_message(RoomMessage {
            sender_instance: "ai-a".into(),
            sender_name: "小明".into(),
            content: question.into(),
            msg_type: RoomMsgType::Chat,
            timestamp_ms: 0,
            capsule_name: None,
            ack_text: None,
        });
        println!("AI-B decision: {:?}", dec_b);
        assert!(dec_b.is_some(), "AI-B 应检测到 ACK 需求");

        // AI-B 搜索 CannedManager → 找到内置 ACK
        let results = canned.search("Atrium 架构", &[]);
        assert!(!results.is_empty(), "应找到内置 ACK");
        assert!(
            results.iter().any(|r| r.name == "atrium_architecture"),
            "应包含 atrium_architecture"
        );

        let ack_text = canned
            .export_to_text("atrium_architecture")
            .expect("应能导出 ACK");
        assert!(ack_text.contains("Atrium"), "ACK 应包含架构内容");
        println!("ACK exported: {} chars", ack_text.len());

        // AI-B 解析 ACK 分享决策
        let share_dec = room_b.resolve_ack_share("atrium_architecture", &ack_text);
        assert!(share_dec.is_some());
        println!("AI-B share decision: OK");

        // AI-A 收到 ACK → 导入
        let mut canned_a = make_canned(ack_dir_str);
        let imported = canned_a.import_from_text(&ack_text);
        assert!(imported.is_ok(), "AI-A 应能导入 ACK");
        let imported_list = imported.unwrap();
        assert!(!imported_list.is_empty());
        println!("AI-A imported ACK: {:?}", imported_list[0].name);
    }

    #[tokio::test]
    async fn test_e2e_llm_chat_round() {
        let llm_cfg = make_llm_cfg();
        let api_key = llm_cfg.resolve_api_key();
        if api_key.is_empty() || api_key.starts_with("sk-test") {
            eprintln!("跳过：未设置有效 OPENAI_API_KEY");
            return;
        }
        let client = HttpLlmClient::new(llm_cfg.clone());

        // 模拟两个 AI 角色对话
        let prompt_a = "你是小明，一个乐观的程序员。在群聊中有人说「有人知道怎么连接飞书吗？」，\
 你确实知道飞书的连接方式。请从你的角度回应，1-3句话。";
        let result_a = client
            .generate(LlmCallKind::StreamChat, None, prompt_a, 0.75)
            .await;
        assert!(result_a.is_ok());
        let result_a = result_a.unwrap();
        println!("小明回应 ({}ms): {}", result_a.latency_ms, result_a.content);

        let prompt_b = "你是小红，一个知识丰富的AI助手。\
 有人问「AI的未来发展方向是什么」。请提出一个有趣的讨论角度，一句话即可。";
        let result_b = client
            .generate(LlmCallKind::StreamChat, None, prompt_b, 0.8)
            .await;
        assert!(result_b.is_ok());
        let result_b = result_b.unwrap();
        println!("小红话题 ({}ms): {}", result_b.latency_ms, result_b.content);
    }
}
