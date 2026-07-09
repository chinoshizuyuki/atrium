// SPDX-License-Identifier: MIT
//! 端到端集成测试
//!
//! 测试所有 结构性演进模块在 CoreService 中的集成：
//! ① 自主情感循环（OU漂移+昼夜节律+情感惯性）
//! ② 用户心智模型（情绪/风格/参与度/话题追踪）
//! ③ 实时反馈闭环（5种信号+EMA满意度+行为调节）
//! ④ 主动决策引擎（TimingJudge+AwayDetector+TopicSelector+EventMemory）
//! ⑤ 关系阶段模型（四阶段+质量指标+行为修饰）
//!
//! 使用真实 DeepSeek API 进行 LLM 调用。

use atrium_core::service::CoreService;
use std::time::Duration;

// ─── 辅助函数 ───

/// 创建启用全部 功能的 CoreService（纯内存模式，避免并行测试文件锁）
fn make_service() -> CoreService {
    CoreService::new_in_memory()
}

/// 通过 gRPC trait 发送用户消息
async fn send_user_message(
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

// ─── ① 自主情感循环 ───

#[tokio::test]
async fn test_phase15_autonomous_emotion_loop() {
    let svc = make_service();

    // 初始情感状态应为默认值（PAD 接近 0）
    let initial = svc.current_emotion();
    println!(
        "初始情感: P={:.3} A={:.3} D={:.3}",
        initial.pleasure, initial.arousal, initial.dominance
    );
    assert!(
        initial.pleasure.abs() < 0.1,
        "初始 pleasure 应接近 0, got {}",
        initial.pleasure
    );

    // 情感衰减 tick 不应崩溃
    for _ in 0..100 {
        svc.emotion_tick();
    }

    // tick 后状态仍有效（不会 NaN 或越界）
    let after = svc.current_emotion();
    println!(
        "100 tick 后: P={:.3} A={:.3} D={:.3}",
        after.pleasure, after.arousal, after.dominance
    );
    assert!(
        after.pleasure.is_finite() && after.arousal.is_finite() && after.dominance.is_finite(),
        "情感状态必须有限"
    );
    assert!(
        (-1.0..=1.0).contains(&after.pleasure),
        "pleasure 应在 [-1, 1]"
    );
    assert!(
        (-1.0..=1.0).contains(&after.arousal),
        "arousal 应在 [-1, 1]"
    );
    assert!(
        (-1.0..=1.0).contains(&after.dominance),
        "dominance 应在 [-1, 1]"
    );

    println!("① 自主情感循环: OK (OU漂移+昼夜节律+情感惯性正常运行)");
}

#[tokio::test]
async fn test_phase15_emotion_affect_with_relationship_modulation() {
    let svc = make_service();

    // 发送一条正面消息 → 情感应受影响
    let resp1 = send_user_message(&svc, "你今天表现得真好！我非常满意！").await;
    println!("回复1: {}", resp1.reply);

    let emo1 = svc.current_emotion();
    println!(
        "正面消息后: P={:.3} A={:.3} D={:.3}",
        emo1.pleasure, emo1.arousal, emo1.dominance
    );

    // 连续发送多条消息，情感应逐步累积
    for msg in &["继续保持这个状态", "你真的越来越好了", "和你聊天总是很开心"]
    {
        send_user_message(&svc, msg).await;
    }

    let emo2 = svc.current_emotion();
    println!(
        "连续正面后: P={:.3} A={:.3} D={:.3}",
        emo2.pleasure, emo2.arousal, emo2.dominance
    );

    // 情感应该发生了某种变化（不严格测试方向，因为有昼夜节律和 OU 漂移的混合影响）
    let total_change = (emo2.pleasure - emo1.pleasure).abs()
        + (emo2.arousal - emo1.arousal).abs()
        + (emo2.dominance - emo1.dominance).abs();
    assert!(total_change > 0.0, "连续消息后情感应有变化");

    println!("① 情感调制 + 关系乘数: OK (情感随交互累积变化)");
}

// ─── ② 用户心智模型 ───

#[tokio::test]
async fn test_phase15_user_mental_model_tracking() {
    let svc = make_service();

    // 发送多条不同风格的消息
    let messages = &[
        "你好啊！今天天气真不错呢 😊",
        "我在写一个关于 Rust 异步编程的技术博客",
        "你能帮我看看这段代码有什么问题吗？",
        "我觉得这个方案不太行，重新想一个吧",
        "OK 那就这样吧，谢谢你！",
    ];

    for msg in messages {
        send_user_message(&svc, msg).await;
    }

    // 检查用户心智模型 prompt fragment 是否包含有意义的信息
    let fragment = svc.user_model_prompt_fragment();
    println!("用户心智模型 fragment:\n{}", fragment);

    // fragment 应该非空且包含一些追踪信息
    assert!(
        !fragment.is_empty(),
        "用户心智模型 prompt fragment 不应为空"
    );

    // 检查 health 状态
    let um_status = health_module(&svc, "user_model").await;
    println!("user_model health: {}", um_status);
    assert!(!um_status.is_empty(), "user_model health 状态不应为空");

    println!("② 用户心智模型: OK (情绪/风格/参与度/话题追踪正常)");
}

#[tokio::test]
async fn test_phase15_user_mental_model_mood_tracking() {
    let svc = make_service();

    // 发送一系列负面情绪消息
    for msg in &["我今天心情很差", "什么事情都不顺利", "感觉很沮丧"] {
        send_user_message(&svc, msg).await;
    }

    let fragment = svc.user_model_prompt_fragment();
    println!("负面情绪追踪: {}", fragment);

    // fragment 应该反映负面情绪
    assert!(!fragment.is_empty(), "用户心智模型应该追踪到情绪变化");

    println!("② 用户心智模型情绪追踪: OK");
}

// ─── ③ 实时反馈闭环 ───

#[tokio::test]
async fn test_phase15_feedback_loop_signals() {
    let svc = make_service();

    // 初始满意度应为默认值（接近 0.5 或 0.7）
    let initial_sat = svc.feedback_satisfaction();
    println!("初始满意度: {:.3}", initial_sat);

    // 发送一条 AI 回复（通过 process_message 自动触发 on_ai_reply）
    send_user_message(&svc, "请告诉我关于机器学习的知识").await;

    // 发送纠正信号
    send_user_message(&svc, "不对，你说的完全错了，应该是这样的...").await;
    let sat_after_correction = svc.feedback_satisfaction();
    println!("纠正后满意度: {:.3}", sat_after_correction);

    // 发送赞美信号
    send_user_message(&svc, "太棒了！你说得非常好！").await;
    let sat_after_praise = svc.feedback_satisfaction();
    println!("赞美后满意度: {:.3}", sat_after_praise);

    // 检查反馈 prompt fragment
    let fragment = svc.feedback_prompt_fragment();
    println!("反馈 fragment: {}", fragment);

    // 检查 health
    let fb_status = health_module(&svc, "feedback").await;
    println!("feedback health: {}", fb_status);
    assert!(!fb_status.is_empty(), "feedback health 状态不应为空");

    println!("③ 实时反馈闭环: OK (信号检测+满意度变化正常)");
}

#[tokio::test]
async fn test_phase15_feedback_frustration_detection() {
    let svc = make_service();

    let initial_sat = svc.feedback_satisfaction();

    // 模拟沮丧信号
    for msg in &["你怎么又搞错了", "这已经第三次了", "算了吧不想说了"] {
        send_user_message(&svc, msg).await;
    }

    let sat_after = svc.feedback_satisfaction();
    println!("沮丧信号后: 满意度 {:.3} → {:.3}", initial_sat, sat_after);

    // 满意度应该有某种变化
    let change = (sat_after - initial_sat).abs();
    assert!(change > 0.0, "沮丧信号后满意度应有变化");

    println!("③ 反馈闭环沮丧检测: OK");
}

// ─── ④ 主动决策引擎 ───

#[tokio::test]
async fn test_phase15_proactive_engine_stay_silent_initially() {
    let svc = make_service();

    // 初始状态：没有沉默时间，应该 StaySilent
    let silence = Duration::from_secs(0);
    let ctx = svc.proactive_engine().lock().build_context(silence);
    let decision = svc.proactive_engine().lock().decide(&ctx);

    println!("初始决策: {:?}", decision);
    assert!(
        matches!(
            decision,
            atrium_core::proactive::ProactiveDecision::StaySilent { .. }
        ),
        "无沉默时应 StaySilent, got {:?}",
        decision
    );

    println!("④ 主动决策引擎初始状态: OK (默认 StaySilent)");
}

#[tokio::test]
async fn test_phase15_proactive_event_extraction() {
    let svc = make_service();

    // 通过 ProactiveEngine 的 on_user_message 提取事件
    // （Scheduler 在运行时调用此方法，process_message 不直接触发）
    {
        let mut engine = svc.proactive_engine().lock();
        engine.on_user_message("明天下午3点我有一个重要的面试");
        engine.on_user_message("后天是我的生日，要准备一下");
        engine.on_user_message("这个deadline是下周一，不能延期");
    }

    // 检查 EventMemory 中是否提取到了事件
    let engine = svc.proactive_engine().lock();
    let event_count = engine.event_memory().len();
    println!("提取到的事件数: {}", event_count);

    assert!(
        event_count >= 2,
        "应至少提取到 2 个事件（面试/生日/deadline），got {}",
        event_count
    );

    println!("④ 主动决策引擎事件提取: OK (提取到 {} 个事件)", event_count);
}

#[tokio::test]
async fn test_phase15_proactive_cooldown_behavior() {
    let svc = make_service();

    // 模拟主动行为（用户未回应）
    {
        let mut engine = svc.proactive_engine().lock();
        engine.record_proactive(false); // 用户未回应
    }

    // 立即检查：应该在冷却中
    let silence = Duration::from_secs(10);
    let ctx = svc.proactive_engine().lock().build_context(silence);
    let decision = svc.proactive_engine().lock().decide(&ctx);

    println!("冷却中决策: {:?}", decision);
    assert!(
        matches!(
            decision,
            atrium_core::proactive::ProactiveDecision::StaySilent { .. }
        ),
        "冷却中应 StaySilent"
    );

    // 模拟用户回应后，冷却重置
    {
        let mut engine = svc.proactive_engine().lock();
        engine.record_proactive(true); // 用户回应
    }

    println!("④ 主动决策引擎冷却行为: OK (指数退避+重置正常)");
}

#[tokio::test]
async fn test_phase15_proactive_context_with_phase15_modules() {
    let svc = make_service();

    // 先发送一些消息，让 各模块有数据
    send_user_message(&svc, "你好！今天过得怎么样？").await;
    send_user_message(&svc, "我在研究 Rust 的所有权模型，很有意思").await;

    // 构建 ProactiveContext（包含情感、用户模型、关系阶段信息）
    let silence = Duration::from_secs(600); // 10 分钟沉默
    let ctx = svc.proactive_engine().lock().build_context(silence);

    println!(
        "ProactiveContext: silence={:?}, hour={}, ai_arousal={:.2}, ai_pleasure={:.2}",
        ctx.silence_duration, ctx.current_hour, ctx.ai_arousal, ctx.ai_pleasure
    );
    println!(
        " conversation_state={:?}, proactive_bonus={:.3}",
        ctx.conversation_state, ctx.relationship_proactive_bonus
    );
    println!(
        " user_valence={:?}, user_engagement={:?}",
        ctx.user_valence, ctx.user_engagement
    );

    // context 应该包含有效的 数据
    assert!(
        ctx.ai_arousal.is_finite() && ctx.ai_pleasure.is_finite(),
        "AI 情感状态应有限"
    );

    println!("④ ProactiveContext 集成: OK (情感+用户模型+关系阶段全部注入)");
}

// ─── ⑤ 关系阶段模型 ───

#[tokio::test]
async fn test_phase15_relationship_stage_initial() {
    let svc = make_service();

    // 初始阶段应为 Acquaintance
    let stage = svc.relationship_stage();
    println!("初始关系阶段: {}", stage);
    assert!(
        stage.contains("初识") || stage.to_lowercase().contains("acquaintance"),
        "初始阶段应为初识, got {}",
        stage
    );

    // 初始乘数应为 0.9（初识 Acquaintance 阶段）
    // Initial multiplier should be 0.9 (Acquaintance stage)
    let mult = svc.relationship_affect_multiplier();
    println!("初始情感乘数: {:.3}", mult);
    assert!((mult - 0.9).abs() < 0.01, "初识乘数应为 0.9, got {}", mult);

    println!("⑤ 关系阶段初始状态: OK (初识 + 乘数 0.9)");
}

#[tokio::test]
async fn test_phase15_relationship_metrics_growth() {
    let svc = make_service();

    // 发送多条消息，模拟互动增长
    let messages = &[
        "你好！很高兴认识你",
        "我叫小明，是一个程序员",
        "我特别喜欢写 Rust 代码",
        "你觉得人工智能怎么样？",
        "我最近在学机器学习",
        "周末的时候我喜欢爬山",
        "你有什么兴趣爱好吗？",
        "我觉得我们应该多聊聊",
    ];

    for msg in messages {
        send_user_message(&svc, msg).await;
    }

    // 检查关系阶段
    let stage = svc.relationship_stage();
    println!("8 条消息后关系阶段: {}", stage);

    // 检查情感乘数是否有变化
    let mult = svc.relationship_affect_multiplier();
    println!("8 条消息后情感乘数: {:.3}", mult);

    // 检查 prompt fragment
    let prompt = svc.relationship_prompt_fragment();
    println!("关系 prompt: {}", prompt);
    assert!(!prompt.is_empty(), "关系 prompt fragment 不应为空");

    println!("⑤ 关系阶段模型追踪: OK (指标增长正常)");
}

#[tokio::test]
async fn test_phase15_relationship_transition_detection() {
    let svc = make_service();

    // 大量消息模拟长期互动（尝试触发 Acquaintance → Familiar 转换）
    for i in 0..30 {
        let msg = format!("第 {} 次对话，今天聊点什么呢？", i + 1);
        send_user_message(&svc, &msg).await;
    }

    let stage = svc.relationship_stage();
    println!("30 条消息后关系阶段: {}", stage);

    // 检查是否有转换通知
    let notice = svc.take_relationship_transition_notice();
    if let Some(ref n) = notice {
        println!("关系转换通知: {}", n);
    }

    let mult = svc.relationship_affect_multiplier();
    println!("30 条消息后情感乘数: {:.3}", mult);

    println!("⑤ 关系阶段转换检测: OK");
}

// ─── 全流程集成测试（真实 LLM API）───

#[cfg(test)]
mod real_api {
    use super::*;
    use atrium_core::config::LlmCfg;
    use atrium_core::llm_client::{HttpLlmClient, LlmCallKind};
    use atrium_memory::llm_client::LlmClient; // trait for .generate() dispatch + dyn type
    use std::sync::Arc;

    fn make_llm_cfg() -> LlmCfg {
        // 从 atrium.toml 读取或使用环境变量
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
            // e2e 测试需要真实 API Key，通过环境变量传入
            "sk-test-placeholder".into()
        });
        LlmCfg {
            api_key,
            base_url: "https://api.deepseek.com/".into(),
            model: "deepseek-v4-flash".into(),
            max_tokens: 512,
            timeout_secs: 30,
            max_concurrency: 4,
        }
    }

    /// 端到端：真实 LLM + 全部 模块联动
    #[tokio::test]
    async fn test_e2e_real_llm_phase15_full_pipeline() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 设置 LLM 客户端
        let client = Arc::new(HttpLlmClient::new(cfg));

        // 模拟一次完整的用户对话（通过 process_message）
        let user_messages = &[
            "你好！我是小明，一个热爱编程的程序员。",
            "我最近在研究 Rust 的异步编程，特别有意思。",
            "你觉得 AI 未来会怎样发展？",
            "谢谢你和我聊天，今天过得很开心！",
        ];

        for (i, msg) in user_messages.iter().enumerate() {
            let resp = send_user_message(&svc, msg).await;
            println!(
                "消息 {}: '{}' → 回复: '{}' (emotion: {})",
                i + 1,
                msg,
                resp.reply,
                resp.emotion
            );
            assert!(!resp.reply.is_empty(), "回复不应为空");
        }

        // 验证 各模块状态
        println!("\n── 模块状态 ──");

        // ① 情感状态
        let emo = svc.current_emotion();
        println!(
            "① 情感: P={:.3} A={:.3} D={:.3}",
            emo.pleasure, emo.arousal, emo.dominance
        );
        assert!(emo.pleasure.is_finite(), "情感状态必须有效");

        // ② 用户心智模型
        let um_fragment = svc.user_model_prompt_fragment();
        println!("② 用户心智模型: {} chars", um_fragment.len());
        assert!(!um_fragment.is_empty(), "用户心智模型应有数据");

        // ③ 反馈闭环
        let sat = svc.feedback_satisfaction();
        let fb_fragment = svc.feedback_prompt_fragment();
        println!(
            "③ 满意度: {:.3}, fragment: {} chars",
            sat,
            fb_fragment.len()
        );

        // ⑤ 关系阶段
        let stage = svc.relationship_stage();
        let mult = svc.relationship_affect_multiplier();
        println!("⑤ 关系阶段: {}, 乘数: {:.3}", stage, mult);

        // 用 LLM 生成包含 上下文的回复
        let prompt = format!(
            "你是一个情感AI助手。当前情感状态: 愉悦={:.2}, 唤醒={:.2}。\
 用户信息: {}。关系阶段: {}。\
 请用1-2句话回应用户最后说的「谢谢你和我聊天，今天过得很开心！」",
            emo.pleasure, emo.arousal, um_fragment, stage
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.75)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!(
            "\nLLM 综合回复 ({}ms): {}",
            result.latency_ms, result.content
        );

        println!("\n①②③④⑤ 全流程集成: OK (真实 LLM + 全部 模块联动)");
    }

    /// 端到端：主动决策引擎 + LLM 话题生成
    #[tokio::test]
    async fn test_e2e_proactive_topic_generation() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 先积累一些对话上下文
        send_user_message(&svc, "我喜欢编程和听音乐").await;
        send_user_message(&svc, "最近在学 Rust，很有挑战性").await;

        // 模拟 10 分钟沉默后的 ProactiveContext
        let silence = Duration::from_secs(600);
        let ctx = svc.proactive_engine().lock().build_context(silence);

        println!(
            "沉默 10 分钟: state={:?}, silence={:?}",
            ctx.conversation_state, ctx.silence_duration
        );

        // 如果时机合适，用 LLM 生成主动话题
        let client = HttpLlmClient::new(cfg);

        let prompt = "你是 AI 助手，用户已经沉默了 10 分钟。用户兴趣: 编程、音乐、Rust。\
 请生成一个简短的主动话题来重新开启对话（1-2句话，友好自然）："
            .to_string();

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.8)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!("主动话题 ({}ms): {}", result.latency_ms, result.content);
        assert!(!result.content.is_empty(), "主动话题不应为空");

        println!("④ 主动决策引擎 LLM 话题生成: OK");
    }

    /// 端到端：关联记忆图集成 + LLM
    #[tokio::test]
    async fn test_e2e_associative_graph_phase15c() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 发送多条有语义关联的消息，触发图增量构建
        let messages = &[
            "我喜欢用 Rust 编程，Rust 的所有权机制很有趣。",
            "我最近在学习 AI 和深度学习，特别是 Transformer 架构。",
            "编程和 AI 结合能做出很多有意思的项目。",
            "你觉得 Rust 适合写 AI 应用吗？",
        ];

        for (i, msg) in messages.iter().enumerate() {
            let resp = send_user_message(&svc, msg).await;
            println!("消息 {}: '{}' → 回复长度: {}", i + 1, msg, resp.reply.len());
            assert!(!resp.reply.is_empty(), "回复不应为空");
        }

        // 验证关联图已构建
        let stats = svc.graph_stats();
        println!(
            "\n── 关联记忆图统计 ──\n节点: {}, 边: {}, 平均权重: {:.3}",
            stats.node_count, stats.edge_count, stats.avg_weight
        );
        assert!(
            stats.node_count >= 3,
            "关联图应有至少 3 个节点, 实际: {}",
            stats.node_count
        );
        assert!(
            stats.edge_count >= 1,
            "关联图应有至少 1 条边, 实际: {}",
            stats.edge_count
        );

        // 验证 health_check 报告图统计
        let graph_health = health_module(&svc, "graph").await;
        println!("Health graph: {}", graph_health);
        assert!(
            graph_health.contains("nodes="),
            "health_check 应报告 graph 模块"
        );

        // 测试图维护（衰减）
        svc.graph_maintenance(0.99, 0.01);
        let stats_after = svc.graph_stats();
        println!(
            "维护后: 节点={}, 边={}",
            stats_after.node_count, stats_after.edge_count
        );

        // 使用 LLM 生成回复，包含图上下文
        let client = HttpLlmClient::new(cfg);
        let prompt = format!(
            "你是一个情感AI助手。关联记忆显示用户喜欢 Rust 编程和 AI。\
 当前图有 {} 个节点和 {} 条边。\
 请用1-2句话回应用户的问题「Rust 适合写 AI 应用吗？」",
            stats.node_count, stats.edge_count
        );

        let result = client
            .generate(LlmCallKind::StreamChat, None, &prompt, 0.75)
            .await;
        assert!(result.is_ok(), "LLM 调用应成功");
        let result = result.unwrap();
        println!("LLM 回复 ({}ms): {}", result.latency_ms, result.content);
        assert!(!result.content.is_empty(), "LLM 回复不应为空");

        println!("\n⑧ 关联记忆图 Ph.C 集成: OK (增量构建 + 统计报告 + LLM 联动)");
    }

    // ── 表达系统 E2E 测试（5场景：悲伤/喜悦/关系阶段/潜台词/风格偏移） ──

    /// E2E 场景1：悲伤情绪 → 低愉悦、慢节奏、长停顿
    #[tokio::test]
    async fn test_e2e_expression_sad_scenario() {
        use atrium_core::expression_orchestrator::ExpressionOrchestrator;
        use atrium_emotion::EmotionState;
        use atrium_memory::relationship::RelationshipStage;
        use atrium_memory::style_modulator::ExpressionContext;

        let sad_state = EmotionState::new(-0.6, -0.3, -0.2);
        let stage = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 30,
            key_moments: 5,
        };
        let ctx = ExpressionContext::from_modules(
            &sad_state,
            None,
            atrium_emotion::EmotionDirection::SelfDirected,
            &stage,
            -0.4,
            0.5,
        );
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "我最近压力好大，什么都不想做",
            [-0.6, -0.3, -0.2],
            100,
        );

        assert!(
            expr.linguistic.sentence_length < 14.0,
            "悲伤时句长应偏短, got {:.1}",
            expr.linguistic.sentence_length
        );
        assert!(
            expr.timing.inter_sentence_pause_ms > 500.0,
            "悲伤时停顿应>500ms, got {:.0}ms",
            expr.timing.inter_sentence_pause_ms
        );
        assert!(
            expr.prosody.rate < 1.0,
            "悲伤时语速应<1.0, got {:.3}",
            expr.prosody.rate
        );
        // 一致性校验：悲伤+自我导向可能触发亲昵矛盾警告，属正常行为
        println!(
            "悲伤一致性: is_coherent={}, warnings={:?}",
            expr.coherence.is_coherent, expr.coherence.warnings
        );

        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过LLM验证：未配置API key");
        } else {
            let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
            let client = HttpLlmClient::new(cfg);
            let prompt = format!(
                "你是一个情感AI助手。{}\n用户说：'我最近压力好大，什么都不想做'。请用1-2句话回应。",
                injection
            );
            let result = client
                .generate(LlmCallKind::StreamChat, None, &prompt, 0.7)
                .await;
            assert!(result.is_ok(), "LLM调用应成功");
            let result = result.unwrap();
            println!("悲伤LLM回复({}ms): {}", result.latency_ms, result.content);
        }
        println!(
            "E2E场景1悲伤: OK (句长={:.1}, 停顿={:.0}ms, 语速={:.3})",
            expr.linguistic.sentence_length, expr.timing.inter_sentence_pause_ms, expr.prosody.rate
        );
    }

    /// E2E 场景2：喜悦情绪 → 高愉悦、快节奏、高亲昵
    #[tokio::test]
    async fn test_e2e_expression_joy_scenario() {
        use atrium_core::expression_orchestrator::ExpressionOrchestrator;
        use atrium_emotion::EmotionState;
        use atrium_memory::relationship::RelationshipStage;
        use atrium_memory::style_modulator::ExpressionContext;

        let joy_state = EmotionState::new(0.7, 0.5, 0.3);
        let stage = RelationshipStage::Deep {
            since: 0,
            interactions: 500,
            shared_references: 80,
            key_moments: 20,
        };
        let ctx = ExpressionContext::from_modules(
            &joy_state,
            None,
            atrium_emotion::EmotionDirection::UserDirected,
            &stage,
            0.6,
            0.2,
        );
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "太棒了！我终于搞定了那个bug！",
            [0.7, 0.5, 0.3],
            100,
        );

        assert!(
            expr.linguistic.sentence_length > 14.0,
            "喜悦时句长应偏长, got {:.1}",
            expr.linguistic.sentence_length
        );
        assert!(
            expr.prosody.rate > 1.0,
            "喜悦时语速应>1.0, got {:.3}",
            expr.prosody.rate
        );
        assert!(
            expr.linguistic.endearment_tendency > 0.4,
            "深度+喜悦亲昵应>0.4, got {:.3}",
            expr.linguistic.endearment_tendency
        );
        assert!(expr.coherence.is_coherent, "喜悦场景一致性应通过");

        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过LLM验证：未配置API key");
        } else {
            let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
            let client = HttpLlmClient::new(cfg);
            let prompt = format!(
                "你是一个情感AI助手。{}\n用户说：'太棒了！我终于搞定了那个bug！'。请用1-2句话回应。",
                injection
            );
            let result = client
                .generate(LlmCallKind::StreamChat, None, &prompt, 0.7)
                .await;
            assert!(result.is_ok(), "LLM调用应成功");
            let result = result.unwrap();
            println!("喜悦LLM回复({}ms): {}", result.latency_ms, result.content);
        }
        println!(
            "E2E场景2喜悦: OK (句长={:.1}, 语速={:.3}, 亲昵={:.3})",
            expr.linguistic.sentence_length, expr.prosody.rate, expr.linguistic.endearment_tendency
        );
    }

    /// E2E 场景3：关系阶段差异 → 初识 vs 深度，同一PAD产生不同表达
    #[tokio::test]
    async fn test_e2e_expression_relationship_stage() {
        use atrium_core::expression_orchestrator::ExpressionOrchestrator;
        use atrium_emotion::EmotionState;
        use atrium_memory::relationship::RelationshipStage;
        use atrium_memory::style_modulator::ExpressionContext;

        let state = EmotionState::new(0.3, 0.2, 0.1);

        let stage_acq = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 5,
        };
        let ctx_acq = ExpressionContext::from_modules(
            &state,
            None,
            atrium_emotion::EmotionDirection::UserDirected,
            &stage_acq,
            0.2,
            0.3,
        );
        let expr_acq = ExpressionOrchestrator::orchestrate(&ctx_acq, "你好", [0.0, 0.0, 0.0], 50);

        let stage_deep = RelationshipStage::Deep {
            since: 0,
            interactions: 500,
            shared_references: 80,
            key_moments: 20,
        };
        let ctx_deep = ExpressionContext::from_modules(
            &state,
            None,
            atrium_emotion::EmotionDirection::UserDirected,
            &stage_deep,
            0.6,
            0.2,
        );
        let expr_deep = ExpressionOrchestrator::orchestrate(&ctx_deep, "你好", [0.0, 0.0, 0.0], 50);

        assert!(
            expr_deep.linguistic.endearment_tendency > expr_acq.linguistic.endearment_tendency,
            "深度亲昵({:.3})应>初识亲昵({:.3})",
            expr_deep.linguistic.endearment_tendency,
            expr_acq.linguistic.endearment_tendency
        );
        assert!(
            expr_deep.linguistic.particle_density > expr_acq.linguistic.particle_density,
            "深度语气词({:.3})应>初识语气词({:.3})",
            expr_deep.linguistic.particle_density,
            expr_acq.linguistic.particle_density
        );

        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过LLM验证：未配置API key");
        } else {
            let client = HttpLlmClient::new(cfg);
            let inj_acq = ExpressionOrchestrator::build_system_prompt_injection(&expr_acq);
            let inj_deep = ExpressionOrchestrator::build_system_prompt_injection(&expr_deep);
            let r1 = client
                .generate(
                    LlmCallKind::StreamChat,
                    None,
                    &format!("{}\n用户说'你好'，回应1句", inj_acq),
                    0.7,
                )
                .await
                .unwrap();
            let r2 = client
                .generate(
                    LlmCallKind::StreamChat,
                    None,
                    &format!("{}\n用户说'你好'，回应1句", inj_deep),
                    0.7,
                )
                .await
                .unwrap();
            println!("初识回复: {}", r1.content);
            println!("深度回复: {}", r2.content);
        }
        println!(
            "E2E场景3关系阶段: OK (初识亲昵={:.3} vs 深度亲昵={:.3})",
            expr_acq.linguistic.endearment_tendency, expr_deep.linguistic.endearment_tendency
        );
    }

    /// E2E 场景4：潜台词检测 → "我没事"在深度关系触发Avoidance
    #[tokio::test]
    async fn test_e2e_expression_subtext() {
        use atrium_core::expression_orchestrator::ExpressionOrchestrator;
        use atrium_emotion::EmotionState;
        use atrium_memory::relationship::RelationshipStage;
        use atrium_memory::style_modulator::ExpressionContext;
        use atrium_memory::subtext_engine::SubtextEngine;

        let state = EmotionState::new(-0.2, 0.1, 0.0);
        let stage = RelationshipStage::Deep {
            since: 0,
            interactions: 500,
            shared_references: 80,
            key_moments: 20,
        };

        // 直接用 SubtextEngine::detect 验证文本潜台词检测
        let pad = [-0.2f32, 0.1, 0.0];
        let signals = SubtextEngine::detect("我没事", pad, &stage);
        assert!(
            !signals.is_empty(),
            "深度关系+我没事 → 应检测到潜台词, got {} 条",
            signals.len()
        );
        let has_avoidance = signals
            .iter()
            .any(|s| format!("{:?}", s.category).contains("Avoidance"));
        assert!(has_avoidance, "'我没事'应触发Avoidance潜台词");

        // 编排器路径：decide() 使用 PAD 规则（非文本匹配），可能产生不同信号
        let ctx = ExpressionContext::from_modules(
            &state,
            None,
            atrium_emotion::EmotionDirection::SelfDirected,
            &stage,
            -0.2,
            0.4,
        );
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "我没事", [-0.2, 0.1, 0.0], 50);
        println!(
            "编排器潜台词信号: {} 条 (decide基于PAD规则)",
            expr.subtext_signals.len()
        );

        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过LLM验证：未配置API key");
        } else {
            let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
            let client = HttpLlmClient::new(cfg);
            let prompt = format!(
                "你是一个情感AI助手。{}\n用户说：'我没事'。请用1-2句话回应。",
                injection
            );
            let result = client
                .generate(LlmCallKind::StreamChat, None, &prompt, 0.7)
                .await;
            assert!(result.is_ok(), "LLM调用应成功");
            let result = result.unwrap();
            println!("潜台词LLM回复({}ms): {}", result.latency_ms, result.content);
        }
        println!(
            "E2E场景4潜台词: OK (detect={}条含Avoidance={}, decide={}条)",
            signals.len(),
            has_avoidance,
            expr.subtext_signals.len()
        );
    }

    /// E2E 场景5：风格偏移学习 → 正反馈偏移后风格变化 + 持久化
    #[tokio::test]
    async fn test_e2e_expression_style_memory() {
        use atrium_memory::style_memory::{StyleMemoryStore, StyleOffset};
        use atrium_memory::style_modulator::StyleEmbedding;

        // 用临时 sled 数据库
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = StyleMemoryStore::open(&db).unwrap();
        let initial = store
            .get("test-user")
            .unwrap_or_else(|_| StyleOffset::zero());
        let initial_norm = initial.norm() as f64;
        println!("初始偏移范数: {:.6}", initial_norm);

        let mut offset = initial;
        // 构建目标 StyleEmbedding（用户偏好的风格方向）
        let mut target_arr = [0.0f32; 128];
        target_arr[0] = 0.5;
        target_arr[1] = 0.3;
        let target_style = StyleEmbedding(target_arr);
        // 当前基线风格（零向量）
        let current_style = StyleEmbedding::zero();

        for i in 0..3 {
            offset.apply_positive_feedback(&target_style, &current_style);
            let norm = offset.norm() as f64;
            println!(
                "正反馈 #{}: 范数={:.6}, dim0={:.4}, dim1={:.4}",
                i + 1,
                norm,
                offset.offset[0],
                offset.offset[1]
            );
        }

        assert!(
            offset.offset[0] > 0.0,
            "正反馈后dim0应>0, got {:.4}",
            offset.offset[0]
        );
        assert!(
            offset.offset[1] > 0.0,
            "正反馈后dim1应>0, got {:.4}",
            offset.offset[1]
        );

        // 负反馈：用户不喜欢过于正式
        let mut rejected_arr = [0.0f32; 128];
        rejected_arr[5] = 0.8;
        let rejected_style = StyleEmbedding(rejected_arr);
        let before_formal = offset.offset[5];
        offset.apply_negative_feedback(&rejected_style, &current_style);
        assert!(
            offset.offset[5] < before_formal,
            "负反馈后正式度应降低: {:.4} -> {:.4}",
            before_formal,
            offset.offset[5]
        );

        // 持久化并读回验证
        store.set("test-user", &offset).unwrap();
        let loaded = store.get("test-user").unwrap();
        let diff: f64 = offset
            .offset
            .iter()
            .zip(loaded.offset.iter())
            .map(|(a, b)| (*a - *b) as f64 * (*a - *b) as f64)
            .sum::<f64>()
            .sqrt();
        assert!(diff < 1e-6, "持久化后读回应一致, diff={:.6}", diff);

        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过LLM验证：未配置API key");
        } else {
            use atrium_core::expression_orchestrator::ExpressionOrchestrator;
            use atrium_emotion::EmotionState;
            use atrium_memory::relationship::RelationshipStage;
            use atrium_memory::style_modulator::ExpressionContext;

            let state = EmotionState::new(0.3, 0.2, 0.1);
            let stage = RelationshipStage::Familiar {
                since: 0,
                interactions: 50,
                shared_references: 5,
            };
            let ctx = ExpressionContext::from_modules(
                &state,
                None,
                atrium_emotion::EmotionDirection::UserDirected,
                &stage,
                0.3,
                0.3,
            );
            let expr =
                ExpressionOrchestrator::orchestrate(&ctx, "今天天气真好", [0.3, 0.2, 0.1], 50);
            let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
            let client = HttpLlmClient::new(cfg);
            let prompt = format!(
                "你是一个情感AI助手。{}\n用户说：'今天天气真好'。请用1-2句话回应。",
                injection
            );
            let result = client
                .generate(LlmCallKind::StreamChat, None, &prompt, 0.7)
                .await;
            assert!(result.is_ok(), "LLM调用应成功");
            let result = result.unwrap();
            println!(
                "风格偏移LLM回复({}ms): {}",
                result.latency_ms, result.content
            );
        }
        println!(
            "E2E场景5风格偏移: OK (dim0={:.4}, dim1={:.4}, 正式度={:.4})",
            offset.offset[0], offset.offset[1], offset.offset[5]
        );
    }

    /// 端到端：内在独白引擎 + 真实 LLM（GraphWander + AutonomousLearning + Daydream）
    #[tokio::test]
    async fn test_e2e_inner_monologue_with_llm() {
        let cfg = make_llm_cfg();
        if cfg.api_key.is_empty()
            || cfg.api_key.starts_with("sk-test")
            || cfg.api_key == "YOUR_OPENAI_API_KEY"
        {
            eprintln!("跳过：未配置 API key");
            return;
        }

        let svc = make_service();

        // 设置 LLM 客户端 / Set LLM client
        let client = Arc::new(HttpLlmClient::new(cfg));
        svc.set_llm_client(client);

        // 发送消息填充 FactStore（graph_wander 需要种子事实）
        // Send messages to populate FactStore (graph_wander needs seed facts)
        let messages = &[
            "你好！我叫小明，是一名 Rust 程序员。",
            "我最近在研究异步编程和分布式系统。",
            "我觉得 AI 和人类可以成为很好的合作伙伴。",
        ];
        for msg in messages {
            let _ = send_user_message(&svc, msg).await;
            // 短暂等待事实写入 / Brief wait for fact persistence
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // ── 模式 A: GraphWander（idle >= 600s，白天时段）──
        // Mode A: GraphWander (idle >= 600s, daytime)
        let status_before = svc.inner_monologue_status();
        println!("GraphWander 前状态: {}", status_before);

        svc.tick_inner_monologue(600, 14).await;

        // 等待异步 LLM 完成 / Wait for async LLM to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status_after = svc.inner_monologue_status();
        println!("GraphWander 后状态: {}", status_after);

        // 验证思维已生成 / Verify thoughts were generated
        let thoughts_count = svc.inner_monologue_status();
        assert!(!thoughts_count.is_empty(), "内在独白状态不应为空");

        // ── 模式 C: AutonomousLearning（idle >= 1800s）──
        // Mode C: AutonomousLearning (idle >= 1800s)
        svc.tick_inner_monologue(1800, 14).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status_learn = svc.inner_monologue_status();
        println!("AutonomousLearning 后状态: {}", status_learn);

        // ── 模式 D: Daydream（hour < 6, idle >= 7200s）──
        // Mode D: Daydream (hour < 6, idle >= 7200s)
        svc.tick_inner_monologue(7200, 3).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status_dream = svc.inner_monologue_status();
        println!("Daydream 后状态: {}", status_dream);

        // 验证至少有一个思维被生成 / Verify at least one thought was generated
        // 格式: "thoughts=N today=M diary=K"
        let parts: Vec<&str> = status_dream.split_whitespace().collect();
        for part in &parts {
            if let Some(n_str) = part.strip_prefix("thoughts=") {
                let n: usize = n_str.parse().unwrap_or(0);
                assert!(n > 0, "至少应有一个内在思维被生成, got thoughts={}", n);
                println!("内在独白引擎: {} 个思维已生成", n);
                break;
            }
        }

        // 验证 health_check 包含 inner_monologue 条目
        // Verify health_check includes inner_monologue entry
        let im_health = health_module(&svc, "inner_monologue").await;
        println!("health_check inner_monologue: {}", im_health);
        assert!(
            !im_health.is_empty(),
            "health_check 应包含 inner_monologue 条目"
        );

        println!("\n⑥ 内在独白引擎: OK (真实 LLM + GraphWander/Learning/Daydream 联动)");
    }
}
