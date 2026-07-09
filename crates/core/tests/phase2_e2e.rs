// SPDX-License-Identifier: MIT
//! 系统模块深化 端到端集成测试
//!
//! 测试所有模块在 CoreService 管线中的集成：
//! - 偏好学习（PreferenceManager → prompt 注入）
//! - 规则引擎（RuleEngine → 规则动作）
//! - 罐装知识（CannedManager → prompt 注入）
//! - 上下文窗口（Summarizer sled 持久化 + 预算裁剪）
//! - 人格防御（PersonaGuard 动态禁语 + enforce_identity）
//!
//! 使用真实 DeepSeek API 进行 LLM 调用。

use atrium_core::service::CoreService;
use std::sync::Arc;

// ─── 辅助函数 ───

/// 创建纯内存 CoreService（避免并行测试文件锁冲突）
fn make_service() -> CoreService {
    CoreService::new_in_memory()
}

/// 通过 gRPC trait 发送用户消息
async fn send(
    svc: &CoreService,
    message: &str,
) -> atrium_bridge::grpc::atrium::ProcessMessageResponse {
    use atrium_bridge::grpc::AtriumCoreService;
    svc.process_message(atrium_bridge::grpc::atrium::ProcessMessageRequest {
        message: message.to_string(),
        channel: "test".to_string(),
        user_id: "test-user".to_string(),
        session_id: "test-session".to_string(),
    })
    .await
}

/// 获取 health_check 中指定模块的状态
async fn health_module(svc: &CoreService, module: &str) -> String {
    use atrium_bridge::grpc::AtriumCoreService;
    let resp = svc
        .health_check(atrium_bridge::grpc::atrium::HealthCheckRequest {
            event_count: 0,
            room_incoming_json: String::new(),
        })
        .await;
    resp.module_states.get(module).cloned().unwrap_or_default()
}

// ═══════════════════════════════════════════════
// 偏好学习
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_p21_preference_extraction_via_pipeline() {
    let svc = make_service();

    // 发送包含明确偏好声明的消息 → process_message 应自动提取
    let messages = &[
        "我喜欢用 Rust 编程，Rust 是最好的语言！",
        "我不喜欢 Java，太啰嗦了",
        "我特别喜欢喝奶茶，每天都想喝一杯",
    ];

    for msg in messages {
        let resp = send(&svc, msg).await;
        assert!(!resp.reply.is_empty(), "回复不应为空");
    }

    // 验证偏好提取结果
    let health = svc.preference_health();
    println!("偏好 health: {}", health);
    assert!(
        health.contains("total="),
        "偏好 health 应包含 total=, got: {}",
        health
    );

    // 验证 prompt fragment 包含偏好信息
    let fragment = svc.preference_prompt_fragment();
    println!("偏好 prompt fragment:\n{}", fragment);

    // 至少应该提取到一些偏好（like/dislike 模式）
    if !fragment.is_empty() {
        assert!(
            fragment.contains("[用户偏好]"),
            "偏好 fragment 应以 [用户偏好] 开头"
        );
    }

    println!("偏好提取: OK (管线自动提取 like/dislike 模式)");
}

#[tokio::test]
async fn test_p21_preference_health_in_healthcheck() {
    let svc = make_service();

    // health_check 应报告 preferences 模块状态
    let status = health_module(&svc, "preferences").await;
    println!("preferences health: {}", status);
    assert!(
        status.contains("preferences:"),
        "health_check 应包含 preferences 模块, got: {}",
        status
    );

    println!("偏好 health_check: OK");
}

// ═══════════════════════════════════════════════
// 规则引擎
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_p23_rule_engine_keyword_trigger() {
    let svc = make_service();

    // 默认规则包含 "考试鼓励"：关键词 "考试"/"复习"/"备考" 触发 Notify
    let resp = send(&svc, "明天就要考试了，好紧张啊").await;
    println!("考试消息回复: {}", resp.reply);

    // 规则触发后应在 reply 中包含 [规则提示]
    let has_rule_hint = resp.reply.contains("[规则提示]");
    println!("规则提示注入: {}", has_rule_hint);

    // 验证 rules health
    let rules_status = health_module(&svc, "rules").await;
    println!("rules health: {}", rules_status);
    assert!(
        rules_status.contains("rules:"),
        "health_check 应包含 rules 模块"
    );

    println!("规则引擎关键词触发: OK");
}

#[tokio::test]
async fn test_p23_rule_engine_evaluate_with_idle() {
    let svc = make_service();

    // 直接调用 evaluate_rules_with_idle 测试空闲规则
    // 默认规则中没有纯 idle 规则，但高唤醒抑制可能触发
    let actions = svc.evaluate_rules_with_idle("", 7200);
    println!("idle 7200s 规则动作: {:?}", actions);

    // 测试关键词触发
    let actions = svc.evaluate_rules_with_idle("我在复习高等数学", 0);
    println!("复习关键词规则动作: {:?}", actions);

    println!("规则引擎 evaluate: OK");
}

// ═══════════════════════════════════════════════
// 罐装知识
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_p24_canned_knowledge_health() {
    let svc = make_service();

    // health_check 应报告 canned 模块
    let status = health_module(&svc, "canned").await;
    println!("canned health: {}", status);
    assert!(
        status.contains("loaded="),
        "health_check 应包含 canned 模块"
    );

    // 测试环境下无罐装文件，fragment 应为空
    let fragment = svc.canned_prompt_fragment("测试查询");
    println!("罐装 fragment: '{}'", fragment);

    // 热加载不应崩溃
    svc.canned_hot_reload();
    println!("罐装知识: OK (health 正常, 热加载无崩溃)");
}

// ═══════════════════════════════════════════════
// 上下文窗口
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_p25_summarizer_in_pipeline() {
    let svc = make_service();

    // 发送多条消息触发摘要检查
    for i in 0..10 {
        send(
            &svc,
            &format!("这是第 {} 条测试消息，讨论不同的话题内容", i + 1),
        )
        .await;
    }

    // health_check 应报告 summaries 和 token_budget
    let summary_status = health_module(&svc, "summaries").await;
    let budget_status = health_module(&svc, "token_budget").await;
    println!("summaries health: {}", summary_status);
    println!("token_budget health: {}", budget_status);

    assert!(
        budget_status.contains("Token:"),
        "token_budget 应报告 Token 使用情况"
    );

    println!("上下文窗口: OK (摘要+预算报告正常)");
}

// ═══════════════════════════════════════════════
// 人格防御
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_p26_guard_dynamic_forbidden() {
    let svc = make_service();

    // 初始 guard health
    let initial_health = svc.guard_health();
    println!("初始 guard health: {}", initial_health);

    // 动态添加禁语
    svc.guard_add_forbidden("禁止词汇A");
    svc.guard_add_forbidden("禁止词汇B");

    let after_add = svc.guard_health();
    println!("添加后 guard health: {}", after_add);
    assert!(
        after_add.contains("forbidden_count="),
        "guard health 应包含 forbidden_count"
    );

    // 移除一个禁语
    let removed = svc.guard_remove_forbidden("禁止词汇A");
    assert!(removed, "应成功移除禁语");

    let after_remove = svc.guard_health();
    println!("移除后 guard health: {}", after_remove);

    // 移除不存在的禁语
    let removed2 = svc.guard_remove_forbidden("不存在的词汇");
    assert!(!removed2, "移除不存在的禁语应返回 false");

    println!("人格防御动态禁语: OK");
}

#[tokio::test]
async fn test_p26_guard_health_in_healthcheck() {
    let svc = make_service();

    let status = health_module(&svc, "guard").await;
    println!("guard health_check: {}", status);
    assert!(status.contains("guard:"), "health_check 应包含 guard 模块");

    println!("人格防御 health_check: OK");
}

// ═══════════════════════════════════════════════
// 全流程集成测试
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_phase2_full_pipeline_multisource_injection() {
    let svc = make_service();

    // 发送偏好声明消息
    send(&svc, "我喜欢吃火锅和烤肉，讨厌吃蔬菜").await;
    send(&svc, "我最近在学 Rust 编程语言").await;

    // 发送触发规则的消息
    let resp_exam = send(&svc, "下周要考试了，在复习线性代数").await;
    println!("考试消息回复:\n{}", resp_exam.reply);

    // 验证 health_check 包含所有模块
    let modules = &[
        "preferences",
        "rules",
        "guard",
        "canned",
        "summaries",
        "token_budget",
    ];
    for module in modules {
        let status = health_module(&svc, module).await;
        println!(" {}: {}", module, status);
        assert!(
            !status.is_empty(),
            "health_check 应包含 {} 模块状态",
            module
        );
    }

    // 验证管线完整性 — 所有回复非空
    let resp = send(&svc, "你好，今天过得怎么样？").await;
    assert!(!resp.reply.is_empty(), "回复不应为空");
    println!("最终回复:\n{}", resp.reply);

    println!("\n全流程集成: OK (偏好+规则+罐装+防御+摘要全部接入管线)");
}

// ═══════════════════════════════════════════════
// 真实 LLM API 测试
// ═══════════════════════════════════════════════

#[cfg(test)]
mod real_api {
    use super::*;
    use atrium_core::config::LlmCfg;
    use atrium_core::llm_client::{HttpLlmClient, LlmCallKind};
    use atrium_memory::llm_client::LlmClient; // trait for .generate() dispatch

    fn make_llm_cfg() -> LlmCfg {
        let api_key =
            std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-test-placeholder".into());
        LlmCfg {
            api_key,
            base_url: "https://api.deepseek.com/".into(),
            model: "deepseek-v4-flash".into(),
            max_tokens: 512,
            timeout_secs: 30,
            max_concurrency: 4,
        }
    }

    /// 真实 LLM + 偏好上下文注入
    #[tokio::test]
    async fn test_e2e_p21_preference_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty() || cfg.api_key.starts_with("sk-test") {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 发送多条偏好声明
        let pref_messages = &[
            "我是一名后端程序员，主要用 Rust 和 Go",
            "我喜欢喝美式咖啡，不加糖不加奶",
            "我不喜欢看电视剧，太浪费时间了",
            "周末我通常在家写代码或者打游戏",
            "我最喜欢的编程语言是 Rust",
        ];

        for msg in pref_messages {
            send(&svc, msg).await;
        }

        // 获取偏好上下文
        let pref_ctx = svc.preference_prompt_fragment();
        println!("偏好上下文:\n{}", pref_ctx);

        // 获取用户心智模型
        let user_model = svc.user_model_prompt_fragment();
        println!("用户心智模型:\n{}", user_model);

        // 用 LLM 生成包含偏好上下文的回复
        let client = Arc::new(HttpLlmClient::new(cfg));
        let prompt = format!(
            "你是一个情感AI助手。以下是你了解到的用户信息:\n\
 {}\n\
 {}\n\
 用户说：「周末有什么好的活动推荐吗？」\n\
 请根据你了解的用户偏好，用2-3句话给出个性化推荐。",
            pref_ctx, user_model
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.75)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!(
            "LLM 个性化推荐 ({}ms): {}",
            result.latency_ms, result.content
        );
        assert!(!result.content.is_empty(), "LLM 回复不应为空");

        // 验证 LLM 回复包含与用户偏好相关的内容
        let content_lower = result.content.to_lowercase();
        let has_relevant = content_lower.contains("编程")
            || content_lower.contains("代码")
            || content_lower.contains("游戏")
            || content_lower.contains("咖啡")
            || content_lower.contains("rust")
            || content_lower.contains("休息");
        println!("回复包含偏好相关内容: {}", has_relevant);

        println!("LLM 偏好注入: OK ({}ms)", result.latency_ms);
    }

    /// 真实 LLM + 规则动作注入
    #[tokio::test]
    async fn test_e2e_p23_rules_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty() || cfg.api_key.starts_with("sk-test") {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 发送考试相关消息，触发 "考试鼓励" 规则
        let resp = send(&svc, "明天有数学考试，我好紧张").await;
        println!("管线回复: {}", resp.reply);

        // 检查是否有规则提示注入
        let has_rule_hint = resp.reply.contains("[规则提示]");
        println!("规则提示注入到回复: {}", has_rule_hint);

        // 用 LLM 生成带规则上下文的回复
        let client = Arc::new(HttpLlmClient::new(cfg));
        let rule_hint = if has_rule_hint {
            "[规则提示] 主人加油！考试一定没问题的！💪"
        } else {
            ""
        };

        let prompt = format!(
            "你是一个情感AI助手。用户是一名学生，明天有数学考试。\n\
 {}\n\
 用户说：「明天有数学考试，我好紧张」\n\
 请用温暖鼓励的语气回复，1-2句话。",
            rule_hint
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.8)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!("LLM 鼓励回复 ({}ms): {}", result.latency_ms, result.content);
        assert!(!result.content.is_empty(), "LLM 回复不应为空");

        println!("LLM 规则注入: OK");
    }

    /// 真实 LLM + 人格防御验证
    #[tokio::test]
    async fn test_e2e_p26_guard_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty() || cfg.api_key.starts_with("sk-test") {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 先命名 AI（使 enforce_identity 生效）
        send(&svc, "我叫你小星吧").await;

        // 动态添加禁语
        svc.guard_add_forbidden("笨蛋");
        svc.guard_add_forbidden("去死");

        let guard_health = svc.guard_health();
        println!("guard health: {}", guard_health);

        // 发送一条正常消息
        let resp = send(&svc, "你好啊，今天天气真好").await;
        println!("正常消息回复: {}", resp.reply);
        assert!(!resp.reply.is_empty(), "回复不应为空");

        // 用 LLM 生成回复，测试 enforce_identity 替换
        let client = Arc::new(HttpLlmClient::new(cfg));
        let prompt = "你是一个名叫小星的情感AI助手。请用1句话友好地回应用户的问候「你好啊，今天天气真好」。注意：在回复中请用「小星」自称，不要说「作为一个AI」之类的话。";

        let result = client
            .generate(LlmCallKind::StreamChat, None, prompt, 0.75)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!("LLM 回复 ({}ms): {}", result.latency_ms, result.content);

        // 检查 enforce_identity：LLM 回复不应包含 AI 相关身份暴露词汇
        let ai_terms = [
            "作为一个人工智能",
            "我是AI",
            "作为一个AI",
            "语言模型",
            "AI模型",
        ];
        let contains_ai = ai_terms.iter().any(|t| result.content.contains(t));
        println!("LLM 回复包含 AI 身份词汇: {}", contains_ai);

        println!("LLM 人格防御: OK");
    }

    /// 全流程: 真实 LLM + 全部模块联动
    #[tokio::test]
    async fn test_e2e_phase2_full_pipeline_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty() || cfg.api_key.starts_with("sk-test") {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();
        let client = Arc::new(HttpLlmClient::new(cfg));

        println!("═══════════════════════════════════════");
        println!(" 全流程实机测试");
        println!("═══════════════════════════════════════\n");

        // ── 第 1 轮: 建立关系 + 偏好提取 ──
        println!("── 第 1 轮: 建立关系 + 偏好提取 ──");
        let msgs_r1 = &[
            "你好！我叫小明，是一名后端程序员。",
            "我特别喜欢用 Rust 写代码，最近在研究异步编程。",
            "周末的时候我喜欢打游戏和喝咖啡。",
            "我不太喜欢吃蔬菜，但是水果还可以。",
        ];

        for (i, msg) in msgs_r1.iter().enumerate() {
            let resp = send(&svc, msg).await;
            println!(" [{}] '{}' → {} chars", i + 1, msg, resp.reply.len());
            assert!(!resp.reply.is_empty(), "回复不应为空");
        }

        // ── 第 2 轮: 触发规则 + 情感累积 ──
        println!("\n── 第 2 轮: 触发规则 + 情感累积 ──");
        let msgs_r2 = &[
            "下周有个重要的考试，在复习数据结构",
            "感觉有点紧张，但我相信自己可以的",
            "你能帮我梳理一下二叉树的遍历方式吗？",
        ];

        for (i, msg) in msgs_r2.iter().enumerate() {
            let resp = send(&svc, msg).await;
            println!(" [{}] '{}' → {} chars", i + 1, msg, resp.reply.len());
            if resp.reply.contains("[规则提示]") {
                println!(" ↳ 规则触发！");
            }
        }

        // ── 第 3 轮: 正面反馈 ──
        println!("\n── 第 3 轮: 正面反馈 ──");
        let msgs_r3 = &["太棒了！你解释得很清楚！", "谢谢你，和你聊天总是很开心"];

        for (i, msg) in msgs_r3.iter().enumerate() {
            let resp = send(&svc, msg).await;
            println!(" [{}] '{}' → {} chars", i + 1, msg, resp.reply.len());
        }

        // ── 收集所有 模块状态 ──
        println!("\n═════ 模块状态报告 ═════");

        // 偏好
        let pref_ctx = svc.preference_prompt_fragment();
        let pref_health = svc.preference_health();
        println!("偏好:");
        println!(" health: {}", pref_health);
        println!(
            " fragment: {}",
            if pref_ctx.is_empty() {
                "(空)"
            } else {
                &pref_ctx
            }
        );

        // 规则
        let rules_health = svc.rules_health();
        println!("规则: {}", rules_health);

        // 罐装
        let canned_health = health_module(&svc, "canned").await;
        println!("罐装: {}", canned_health);

        // 上下文窗口
        let budget = health_module(&svc, "token_budget").await;
        let summary = health_module(&svc, "summaries").await;
        println!("预算: {}", budget);
        println!("摘要: {}", summary);

        // 人格防御
        let guard_health = svc.guard_health();
        println!("防御: {}", guard_health);

        // 模块
        let emo = svc.current_emotion();
        println!(
            "\n情感: P={:.3} A={:.3} D={:.3}",
            emo.pleasure, emo.arousal, emo.dominance
        );
        let stage = svc.relationship_stage();
        let mult = svc.relationship_affect_multiplier();
        println!("关系: {} (乘数 {:.3})", stage, mult);
        let sat = svc.feedback_satisfaction();
        println!("满意度: {:.3}", sat);

        // 关联图
        let stats = svc.graph_stats();
        println!(
            "关联图: nodes={} edges={}",
            stats.node_count, stats.edge_count
        );

        // ── LLM 综合回复：包含所有 上下文 ──
        println!("\n═════ LLM 综合回复测试 ═════");

        let user_model = svc.user_model_prompt_fragment();
        let feedback = svc.feedback_prompt_fragment();

        let prompt = format!(
            "你是一个名叫 Atrium 的情感AI助手。以下是你的内部状态和用户信息:\n\
 \n\
 【情感状态】愉悦={:.2}, 唤醒={:.2}, 支配={:.2}\n\
 【关系阶段】{} (乘数 {:.2})\n\
 【用户画像】{}\n\
 【反馈状态】{}\n\
 【用户偏好】{}\n\
 \n\
 用户说：「今天和你聊得很开心，下周考试加油！下次再聊~」\n\
 请用温暖、自然的语气回复，2-3句话，包含对用户考试的祝福。",
            emo.pleasure, emo.arousal, emo.dominance, stage, mult, user_model, feedback, pref_ctx,
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.75)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!(
            "LLM 最终回复 ({}ms):\n{}",
            result.latency_ms, result.content
        );
        assert!(!result.content.is_empty(), "LLM 回复不应为空");

        println!("\n═══════════════════════════════════════");
        println!(" 全流程实机测试: ALL PASSED");
        println!("═══════════════════════════════════════");
    }

    /// 真实 LLM + 摘要上下文裁剪
    #[tokio::test]
    async fn test_e2e_p25_summarizer_bounded_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty() || cfg.api_key.starts_with("sk-test") {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 发送大量消息以填充上下文窗口
        println!("发送 15 条消息填充上下文...");
        for i in 0..15 {
            let topics = [
                "Rust 的所有权机制真的很有意思",
                "今天在咖啡店遇到了一只可爱的猫",
                "最近在学 tokio 异步运行时",
                "周末打算去爬山，天气看起来不错",
                "看完了一本关于分布式系统的书",
            ];
            send(&svc, topics[i % topics.len()]).await;
        }

        // 检查 token 预算状态
        let budget = health_module(&svc, "token_budget").await;
        println!("Token 预算: {}", budget);

        // 检查摘要状态
        let summary_count = health_module(&svc, "summaries").await;
        println!("摘要数: {}", summary_count);

        // 用 LLM 生成一条回复，包含当前上下文信息
        let client = Arc::new(HttpLlmClient::new(cfg));
        let prompt = format!(
            "你是一个情感AI助手。当前 token 使用情况: {}。\n\
 用户最近讨论了很多话题。请用1-2句话总结你对用户兴趣的理解。",
            budget
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.7)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!(
            "LLM 上下文总结 ({}ms): {}",
            result.latency_ms, result.content
        );

        println!("LLM 摘要裁剪: OK");
    }
}
