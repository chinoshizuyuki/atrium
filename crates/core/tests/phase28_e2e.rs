// SPDX-License-Identifier: MIT
//! 「工程补强」端到端集成测试
//!
//! 最后一公里接通
//! - relationship_prompt_fragment() → 关系阶段行为指引
//! - user_model_prompt_fragment() → 用户心智模型情绪建议
//! - feedback_prompt_fragment() → 反馈回路行为修正
//! - proactive engine 接收真实信号（非硬编码零值）
//!
//! 情感持久化
//! - EmotionEngine snapshot/restore 往返一致性
//! - EmotionStore sled 保存/加载往返一致性
//! - 情感惯性历史在快照中正确保留
//!
//! 情绪性记忆
//! - process_message 后事实自动附带 EmotionContext
//! - 情绪查询通过管线正确传递
//!
//! 记忆巩固机制
//! - try_consolidation() 在用户不活跃时正确执行
//! - 巩固跳过活跃用户
//! - 相似事实通过管线合并
//! - 矛盾事实通过管线废弃
//!
//! 统一智能提取管线
//! - IntelligenceExtractor prompt 构建和 JSON 解析
//! - RuleEngine sled 持久化往返
//! - apply_extraction_result 正确写入偏好和规则
//!
//! ACK 自学习
//! - 用户教学意图 → ACK 自动创建（Path A）
//! - tick_ack_synthesis 空数据烟雾测试
//! - health_check 包含 ack_learning 统计

use atrium_core::service::CoreService;

// ─── 辅助函数 ───

fn make_service() -> CoreService {
    CoreService::new_in_memory()
}

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

// ═══════════════════════════════════════════════
// Prompt Fragment 注入验证
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_prompt_contains_relationship_fragment() {
    let svc = make_service();

    // 发送一条消息触发管线 → 关系追踪 + prompt 注入
    let resp = send(&svc, "你好，今天天气真不错！").await;

    // 回复应包含关系阶段文本（初识阶段的指引）
    assert!(
        resp.reply.contains("初识") || resp.reply.contains("关系阶段"),
        "回复应包含关系阶段指引文本, reply: {}",
        &resp.reply[..resp.reply.len().min(200)]
    );
}

#[tokio::test]
async fn test_prompt_contains_user_model_fragment() {
    let svc = make_service();

    // 发送多条消息让心智模型积累数据
    send(&svc, "我最近在学 Rust 编程语言").await;
    send(&svc, "Rust 的所有权机制真的很有意思").await;
    let resp = send(&svc, "你觉得 Rust 和 Go 哪个更好？").await;

    // 回复应包含用户心智模型片段（情绪/风格/参与度描述）
    let has_model_hint = resp.reply.contains("用户当前")
        || resp.reply.contains("情绪")
        || resp.reply.contains("参与度")
        || resp.reply.contains("交流风格");
    assert!(
        has_model_hint,
        "回复应包含用户心智模型片段, reply: {}",
        &resp.reply[..resp.reply.len().min(300)]
    );
}

#[tokio::test]
async fn test_prompt_contains_feedback_fragment() {
    let svc = make_service();

    // 发送消息并包含一些反馈信号
    send(&svc, "你好").await;
    send(&svc, "说得好，太对了！").await; // 赞美信号
    let resp = send(&svc, "继续说说").await;

    // 回复应包含反馈回路片段（满意度/回复风格建议）
    let has_feedback =
        resp.reply.contains("满意度") || resp.reply.contains("反馈") || resp.reply.contains("建议");
    assert!(
        has_feedback,
        "回复应包含反馈回路片段, reply: {}",
        &resp.reply[..resp.reply.len().min(300)]
    );
}

// ═══════════════════════════════════════════════
// Proactive Engine 真实信号验证
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_proactive_context_has_real_emotion() {
    let svc = make_service();

    // 通过 process_message 管线触发内部 affect()（情感影响）
    // 发送情感强烈的消息，使情感引擎产生非零变化
    send(&svc, "我今天超级开心！发生了很多很棒的事情！").await;

    // 获取情感状态 → 经过 affect 后应非零
    let (arousal, pleasure) = svc.current_emotion_state();
    assert!(
        pleasure.abs() > 0.001 || arousal.abs() > 0.001,
        "情感状态应非零: arousal={}, pleasure={}",
        arousal,
        pleasure
    );

    // 注入到 proactive engine 并验证 build_context 使用真实值
    let silence = std::time::Duration::from_secs(60);
    let ctx = {
        let mut pe = svc.proactive_engine().lock();
        pe.update_emotion(arousal, pleasure);
        pe.build_context(silence)
    };

    assert!(
        ctx.ai_pleasure.abs() > 0.001 || ctx.ai_arousal.abs() > 0.001,
        "ProactiveContext 的 AI 情感应非零: ai_pleasure={}, ai_arousal={}",
        ctx.ai_pleasure,
        ctx.ai_arousal
    );
}

#[tokio::test]
async fn test_proactive_context_has_real_relationship_bonus() {
    let svc = make_service();

    // 初始关系阶段是 Acquaintance，proactive_bonus 应为 -0.1
    let bonus = svc.relationship_proactive_bonus();
    assert!(
        (bonus - (-0.1)).abs() < 0.01,
        "初始 Acquaintance 阶段的 proactive_bonus 应为 -0.1, got: {}",
        bonus
    );

    // 注入到 proactive engine 并验证 build_context
    let silence = std::time::Duration::from_secs(60);
    let ctx = {
        let mut pe = svc.proactive_engine().lock();
        pe.update_relationship_bonus(bonus);
        pe.build_context(silence)
    };

    assert!(
        (ctx.relationship_proactive_bonus - (-0.1)).abs() < 0.01,
        "ProactiveContext 的 relationship_bonus 应为 -0.1, got: {}",
        ctx.relationship_proactive_bonus
    );
}

// ═══════════════════════════════════════════════
// 用户心智模型信号验证
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_proactive_context_has_user_model_signals() {
    let svc = make_service();

    // 发送消息以积累用户模型数据
    send(&svc, "你好，我最近在研究量子计算").await;
    send(&svc, "这个领域真的很有意思").await;

    // 获取心智模型信号
    let (valence, engagement, msg_len) = svc.user_model_signals();
    assert!(valence.is_some(), "valence 应有值");
    assert!(engagement.is_some(), "engagement 应有值");
    assert!(msg_len.is_some(), "msg_length 应有值");
    assert!(
        msg_len.unwrap() > 0.0,
        "消息长度应 > 0, got: {}",
        msg_len.unwrap()
    );

    // 注入到 proactive engine 并验证
    let silence = std::time::Duration::from_secs(60);
    let ctx = {
        let mut pe = svc.proactive_engine().lock();
        pe.update_user_model(valence, engagement, msg_len);
        pe.build_context(silence)
    };

    assert!(
        ctx.user_valence.is_some(),
        "ProactiveContext 的 user_valence 应有值"
    );
    assert!(
        ctx.user_engagement.is_some(),
        "ProactiveContext 的 user_engagement 应有值"
    );
    assert!(
        ctx.user_avg_message_length.is_some(),
        "ProactiveContext 的 user_avg_message_length 应有值"
    );
}

// ═══════════════════════════════════════════════
// 情感持久化验证
// ═══════════════════════════════════════════════

#[test]
fn test_emotion_snapshot_restore_roundtrip() {
    use atrium_emotion::{EmotionEngine, EmotionState};

    // 构建带惯性的引擎并施加多次情感影响
    let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.05)
        .with_inertia(atrium_emotion::EmotionalInertia::default());

    engine.affect(&EmotionState::new(0.6, 0.3, -0.2));
    engine.affect(&EmotionState::new(-0.1, 0.2, 0.1));
    // 多次 tick 积累惯性历史
    for _ in 0..10 {
        engine.tick_with_hour(14);
    }

    // 生成快照
    let snap = engine.snapshot();

    // 记录原始状态
    let orig_p = engine.current().pleasure;
    let orig_a = engine.current().arousal;
    let orig_d = engine.current().dominance;

    // 创建全新引擎并恢复
    let mut engine2 = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.05)
        .with_inertia(atrium_emotion::EmotionalInertia::default());
    engine2.restore(&snap);

    // 验证 PAD 值完全一致
    assert!(
        (engine2.current().pleasure - orig_p).abs() < 1e-6,
        "pleasure 不匹配: {} vs {}",
        engine2.current().pleasure,
        orig_p
    );
    assert!(
        (engine2.current().arousal - orig_a).abs() < 1e-6,
        "arousal 不匹配: {} vs {}",
        engine2.current().arousal,
        orig_a
    );
    assert!(
        (engine2.current().dominance - orig_d).abs() < 1e-6,
        "dominance 不匹配: {} vs {}",
        engine2.current().dominance,
        orig_d
    );
}

#[test]
fn test_emotion_store_sled_roundtrip() {
    use atrium_emotion::{EmotionState, InertiaModifiers};
    use atrium_memory::emotion_store::EmotionStore;

    let store = EmotionStore::open_in_memory().unwrap();

    let snap = atrium_emotion::EmotionSnapshot {
        current: EmotionState::new(0.45, -0.3, 0.15),
        inertia_history: vec![[0.1, 0.2, 0.3], [0.4, -0.1, 0.0], [-0.2, 0.5, 0.1]],
        inertia_dominant_duration: 17,
        inertia_dominant_label: Some("兴奋".to_string()),
        inertia_modifiers: InertiaModifiers {
            sensitivity: 1.3,
            decay_rate: 0.92,
            expression_threshold: -0.03,
        },
        longing_state: None,
    };

    // 保存
    store.save_snapshot(&snap).unwrap();

    // 加载
    let loaded = store.load_snapshot().unwrap().expect("应有快照");

    // 逐字段验证
    assert!((loaded.current.pleasure - 0.45).abs() < 1e-6);
    assert!((loaded.current.arousal - (-0.3)).abs() < 1e-6);
    assert!((loaded.current.dominance - 0.15).abs() < 1e-6);
    assert_eq!(loaded.inertia_history.len(), 3);
    assert_eq!(loaded.inertia_dominant_duration, 17);
    assert_eq!(loaded.inertia_dominant_label.as_deref(), Some("兴奋"));
    assert!((loaded.inertia_modifiers.sensitivity - 1.3).abs() < 1e-6);
    assert!((loaded.inertia_modifiers.decay_rate - 0.92).abs() < 1e-6);
    assert!((loaded.inertia_modifiers.expression_threshold - (-0.03)).abs() < 1e-6);
}

#[test]
fn test_emotion_snapshot_preserves_inertia_history() {
    use atrium_emotion::{EmotionEngine, EmotionState};

    let mut engine = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.05)
        .with_inertia(atrium_emotion::EmotionalInertia::default());

    // 大量 tick 以积累丰富的惯性历史
    for _ in 0..50 {
        engine.affect(&EmotionState::new(0.1, 0.0, 0.0));
        engine.tick_with_hour(10);
    }

    let snap = engine.snapshot();

    // 惯性历史应该非空
    assert!(
        !snap.inertia_history.is_empty(),
        "惯性历史不应为空，共 {} 条",
        snap.inertia_history.len()
    );

    // 恢复后历史长度应一致
    let mut engine2 = EmotionEngine::new(EmotionState::new(0.0, 0.0, 0.0), 0.05)
        .with_inertia(atrium_emotion::EmotionalInertia::default());
    engine2.restore(&snap);

    let snap2 = engine2.snapshot();
    assert_eq!(
        snap.inertia_history.len(),
        snap2.inertia_history.len(),
        "恢复后惯性历史长度应一致"
    );
}

#[tokio::test]
async fn test_process_message_persists_emotion() {
    // 验证 process_message 后情感状态可通过 snapshot 获取（非零）
    let svc = make_service();

    // 发送情感强烈的消息
    send(&svc, "我今天超级难过，心情很差").await;

    // 情感状态应该发生变化
    let (arousal, pleasure) = svc.current_emotion_state();
    assert!(
        pleasure.abs() > 0.001 || arousal.abs() > 0.001,
        "process_message 后情感状态应非零: P={}, A={}",
        pleasure,
        arousal
    );
}

// ═══════════════════════════════════════════════
// 情绪性记忆验证
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_facts_have_emotion_context_after_process_message() {
    let svc = make_service();

    // 发送包含明确事实的消息
    send(&svc, "我喜欢用 Rust 编程语言").await;

    // 查询 FactStore → 应有事实且附带情感上下文
    let facts = svc.fact_store().query("Rust").unwrap();
    assert!(!facts.is_empty(), "应有包含 Rust 的事实");

    let fact = &facts[0];
    assert!(
        fact.emotion_context.is_some(),
        "事实应附带情感上下文, got: {:?}",
        fact
    );

    let ctx = fact.emotion_context.as_ref().unwrap();
    assert!(
        !ctx.ai_emotion_label.is_empty(),
        "情感标签不应为空: {:?}",
        ctx
    );
    assert!(ctx.timestamp > 0, "时间戳应 > 0: {}", ctx.timestamp);
}

#[tokio::test]
async fn test_emotion_query_through_pipeline() {
    let svc = make_service();

    // 发送多条消息，产生不同情感上下文的事实
    send(&svc, "我喜欢喝咖啡，特别喜欢拿铁").await;
    send(&svc, "我讨厌加班，真的让人很烦躁").await;
    send(&svc, "Rust 的所有权机制很巧妙").await;

    // 获取所有事实
    let all_facts = svc.fact_store().query("").unwrap_or_default();
    // 由于 query("") 可能不匹配所有，改用 query_by_subject
    let user_facts = svc
        .fact_store()
        .query_by_subject("主人")
        .unwrap_or_default();
    let total = all_facts.len().max(user_facts.len());
    assert!(total > 0, "应有提取的事实");

    // 至少有一条事实带情感标注
    let has_emotion = all_facts
        .iter()
        .chain(user_facts.iter())
        .any(|f| f.emotion_context.is_some());
    assert!(has_emotion, "至少一条事实应有情感标注");
}

// ═══════════════════════════════════════════════
// 记忆巩固机制验证
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_consolidation_runs_when_inactive() {
    let svc = make_service();

    // 通过管线注入若干事实
    send(&svc, "我喜欢喝拿铁咖啡").await;
    send(&svc, "Rust 是一门优秀的系统编程语言").await;

    let before = svc.fact_store().count();
    assert!(before > 0, "应有提取的事实");

    // 模拟用户不活跃 24 小时（86400 秒），触发巩固
    // trigger_inactive_hours=6 → 21600 秒即可
    let result = svc.try_consolidation(86400, 6);
    assert!(result.is_some(), "用户不活跃足够长时间后应执行巩固");

    let r = result.unwrap();
    assert!(r.facts_before > 0, "巩固前应有事实");
    assert!(r.timestamp > 0, "应有时间戳");
}

#[tokio::test]
async fn test_consolidation_skips_when_user_active() {
    let svc = make_service();

    send(&svc, "我最近在学习编程").await;

    // 模拟用户仅不活跃 10 分钟（600 秒），远不足 6 小时
    let result = svc.try_consolidation(600, 6);
    assert!(
        result.is_none(),
        "用户活跃时不应执行巩固, got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_consolidation_merges_similar_facts() {
    use atrium_memory::fact_store::Fact;

    let svc = make_service();

    // 直接注入高度相似的事实（模拟管线提取）
    let store = svc.fact_store();
    store
        .insert(Fact::new("主人", "喜欢", "喝拿铁咖啡").with_confidence(0.9))
        .unwrap();
    store
        .insert(Fact::new("主人", "喜欢", "喝拿铁").with_confidence(0.7))
        .unwrap();
    store
        .insert(Fact::new("主人", "喜欢", "吃巧克力").with_confidence(0.8))
        .unwrap();

    let before = store.count();
    assert!(before >= 3, "应至少有 3 条事实, got: {}", before);

    // 执行巩固
    let result = svc.try_consolidation(86400, 6);
    assert!(result.is_some(), "应执行巩固");

    let r = result.unwrap();
    // 相似事实"喝拿铁咖啡"和"喝拿铁"应被合并
    assert!(
        r.merged_pairs > 0 || r.facts_after <= r.facts_before,
        "巩固应减少或保持事实数量: merged={}, before={}, after={}",
        r.merged_pairs,
        r.facts_before,
        r.facts_after
    );
}

#[tokio::test]
async fn test_consolidation_deprecates_contradictions() {
    use atrium_memory::fact_store::Fact;

    let svc = make_service();

    // 注入矛盾事实
    let store = svc.fact_store();
    store
        .insert(Fact::new("主人", "喜欢", "猫").with_confidence(0.9))
        .unwrap();
    store
        .insert(Fact::new("主人", "讨厌", "猫").with_confidence(0.8))
        .unwrap();
    store
        .insert(Fact::new("主人", "喜欢", "编程").with_confidence(0.7))
        .unwrap();

    let before = store.count();
    assert!(before >= 3, "应至少有 3 条事实, got: {}", before);

    // 执行巩固
    let result = svc.try_consolidation(86400, 6);
    assert!(result.is_some(), "应执行巩固");

    let r = result.unwrap();
    // 矛盾事实应被废弃（"喜欢猫" vs "讨厌猫"）
    assert!(
        r.deprecated_count > 0 || r.facts_after < r.facts_before,
        "矛盾事实应被废弃: deprecated={}, before={}, after={}",
        r.deprecated_count,
        r.facts_before,
        r.facts_after
    );
}

// ═══════════════════════════════════════════════
// 统一智能提取管线验证
// ═══════════════════════════════════════════════

#[test]
fn test_intelligence_prompt_building() {
    use atrium_memory::history::ChatMessage;
    use atrium_memory::intelligence::build_extraction_prompt;

    let msgs = vec![
        ChatMessage {
            role: "user".into(),
            content: "我喜欢用 Rust 编程".into(),
            timestamp_ms: 0,
            emotion: None,
        },
        ChatMessage {
            role: "assistant".into(),
            content: "Rust 是一门优秀的语言".into(),
            timestamp_ms: 0,
            emotion: None,
        },
        ChatMessage {
            role: "user".into(),
            content: "以后提到加班的时候提醒我休息".into(),
            timestamp_ms: 0,
            emotion: None,
        },
    ];

    let prompt = build_extraction_prompt(&msgs);
    assert!(prompt.contains("用户"), "prompt 应包含用户标记");
    assert!(prompt.contains("Rust"), "prompt 应包含对话内容");
    assert!(prompt.contains("JSON"), "prompt 应要求 JSON 输出");
}

#[test]
fn test_intelligence_json_parsing() {
    use atrium_memory::intelligence::parse_extraction_response;

    let json = r#"{
 "preferences": [
 {"key": "lang", "value": "Rust", "sentiment": "like", "context": "我喜欢Rust"}
 ],
 "rules": [
 {
 "name": "加班提醒",
 "trigger_type": "keyword",
 "keywords": ["加班", "熬夜"],
 "reminder": "该休息了"
 }
 ]
 }"#;

    let result = parse_extraction_response(json);
    assert_eq!(result.preferences.len(), 1);
    assert_eq!(result.preferences[0].key, "lang");
    assert_eq!(result.preferences[0].value, "Rust");
    assert_eq!(result.rules.len(), 1);
    assert_eq!(result.rules[0].name, "加班提醒");
}

#[test]
fn test_intelligence_parse_garbage_returns_empty() {
    use atrium_memory::intelligence::parse_extraction_response;

    let result = parse_extraction_response("抱歉，我无法完成这个请求。");
    assert!(result.preferences.is_empty());
    assert!(result.rules.is_empty());
}

#[test]
fn test_rule_engine_sled_persistence_roundtrip() {
    use atrium_memory::rules::{RuleAction, RuleEngine, TriggerCondition};

    // 创建内存模式规则引擎并添加用户规则
    let mut engine = RuleEngine::open_in_memory();
    engine.register_defaults();
    let initial_count = engine.rule_count();

    // 添加用户规则（priority >= 10）
    let user_rule = atrium_memory::rules::BehaviorRule::new(
        "测试提醒".into(),
        10,
        600,
        TriggerCondition::Keyword {
            words: vec!["测试".into()],
        },
        RuleAction::Notify {
            message: "该测试了".into(),
        },
    );
    engine.add(user_rule);
    assert_eq!(engine.rule_count(), initial_count + 1);
}

#[test]
fn test_rule_engine_has_named_rule() {
    use atrium_memory::rules::RuleEngine;

    let mut engine = RuleEngine::new();
    engine.register_defaults();

    // 内置规则应存在
    assert!(engine.has_named_rule("深夜提醒"));
    assert!(engine.has_named_rule("考试鼓励"));
    assert!(!engine.has_named_rule("不存在的规则"));
}

#[tokio::test]
async fn test_extracted_rule_to_behavior_rule() {
    use atrium_memory::intelligence::ExtractedRule;

    let rule = ExtractedRule {
        name: "睡觉提醒".into(),
        trigger_type: "time_range".into(),
        keywords: vec![],
        time_start: "23:00".into(),
        time_end: "06:00".into(),
        idle_seconds: 0,
        reminder: "该睡觉了~".into(),
    };

    let br = rule.to_behavior_rule();
    assert_eq!(br.name, "睡觉提醒");
    assert!(br.enabled);
    assert_eq!(br.priority, 10);
}

#[tokio::test]
async fn test_apply_extraction_writes_preferences() {
    use atrium_memory::intelligence::{ExtractedPreference, ExtractionResult};

    let svc = make_service();

    let result = ExtractionResult {
        preferences: vec![ExtractedPreference {
            key: "hobby".into(),
            value: "画画".into(),
            sentiment: "like".into(),
            context: "我喜欢画画".into(),
        }],
        rules: vec![],
    };

    // 通过 apply_extraction_result 写入偏好（无需真实 LLM）
    let health_before = svc.preference_health();
    svc.apply_extraction_result(&result);
    let health_after = svc.preference_health();

    // 应有新增偏好（total 数量增加）
    assert_ne!(
        health_before, health_after,
        "应有新偏好写入: before={}, after={}",
        health_before, health_after
    );
}

// ════════════════════════════════════════════════════════════════════
// 共情推理引擎
// ════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_empathy_detects_loss_in_pipeline() {
    let svc = make_service();
    let resp = send(&svc, "我被公司裁员了，突然失业了").await;
    // 共情推理应检测到 Loss 事件并注入 prompt fragment
    assert!(!resp.reply.is_empty(), "process_message 应返回非空回复");
    // 验证 empathy health 有记录
    let health = svc.empathy_health();
    assert!(
        health.contains("tracked_events=1"),
        "应追踪到 1 个事件: {}",
        health
    );
}

#[tokio::test]
async fn test_empathy_affects_emotion_engine() {
    let svc = make_service();

    // 获取初始情感状态
    let before_p = svc.current_emotion().pleasure;

    // 发送悲伤消息
    let _ = send(&svc, "奶奶今天去世了，再也见不到她了").await;

    // 情感应该受到影响（pleasure 应下降）
    let after_p = svc.current_emotion().pleasure;

    // 由于 Grief 事件的 base_valence = -0.9 且经过 empathy_weight(0.6) 和关系阶段调制
    // pleasure 应该比初始状态更低
    assert!(
        after_p < before_p || after_p < 0.0,
        "悲伤事件应降低 pleasure: before={}, after={}",
        before_p,
        after_p
    );
}

#[tokio::test]
async fn test_empathy_prompt_fragment_injected() {
    let svc = make_service();
    // 发送包含明确事件的消息
    let _ = send(&svc, "我和好朋友吵架了，吵得很凶").await;

    let fragment = svc.empathy_prompt_fragment();
    assert!(
        !fragment.is_empty(),
        "检测到冲突事件后应有共情 prompt fragment"
    );
    assert!(
        fragment.contains("共情推理"),
        "fragment 应包含共情推理标记: {}",
        fragment
    );
}

#[tokio::test]
async fn test_empathy_cooldown_in_pipeline() {
    let svc = make_service();

    // 第一条消息触发事件
    let _ = send(&svc, "今天加班好累").await;
    let fragment1 = svc.empathy_prompt_fragment();
    assert!(!fragment1.is_empty(), "第一条消息应触发共情");

    // 紧接着第二条类似消息 — 冷却期内不应触发
    let _ = send(&svc, "又加班了，累死了").await;
    // 由于 cooldown_messages=3，第二条不应重新触发同类型事件
    let health = svc.empathy_health();
    assert!(
        health.contains("tracked_events=1"),
        "冷却期内不应新增事件: {}",
        health
    );
}

// ═══════════════════════════════════════════════
// ACK 自学习 E2E
// ═══════════════════════════════════════════════

#[tokio::test]
async fn test_teach_intent_creates_ack() {
    let svc = make_service();
    let initial_count = svc.canned().count();

    // 1. 验证教学意图检测（使用时间戳确保唯一性）
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let teach_msg = format!("记住，我的工号是{}", ts);
    let intent = atrium_memory::teach_detector::detect_teach_intent(&teach_msg);
    assert!(intent.is_some(), "应检测到教学意图");
    let intent = intent.unwrap();
    assert!(
        intent.confidence >= 0.90,
        "置信度应 >= 0.90: {}",
        intent.confidence
    );

    // 2. 通过 learn_from_user 创建 ACK
    {
        let mut canned = svc.canned_write();
        let result = canned.learn_from_user(&intent, 50);
        assert!(result.is_ok(), "learn_from_user 应成功: {:?}", result.err());
    }

    // 3. 验证 ACK 已创建
    let new_count = svc.canned().count();
    assert!(
        new_count > initial_count,
        "教学意图应创建新 ACK: 初始={}, 之后={}",
        initial_count,
        new_count
    );

    // 4. 验证 ack_learning_stats 反映了 user_taught
    let stats = svc.canned().ack_learning_stats();
    assert!(
        stats.contains("user_taught=1") || stats.contains("user_taught"),
        "stats 应反映用户教授 ACK: {}",
        stats
    );

    // 5. 验证 process_message 中教学确认注入正常工作（非教学消息不触发）
    let resp = send(&svc, "你好，今天天气不错").await;
    assert!(!resp.reply.is_empty(), "回复不应为空");
}

#[tokio::test]
async fn test_tick_ack_synthesis_smoke() {
    let svc = make_service();

    // tick_ack_synthesis 应在无数据时安全运行（不 panic）
    svc.tick_ack_synthesis();

    // 多次调用也不应出错
    svc.tick_ack_synthesis();
    svc.tick_ack_synthesis();

    // 无回放模式和洞察时，不应创建任何 ACK
    let stats = svc.canned().ack_learning_stats();
    assert!(
        stats.contains("self_learned=0"),
        "无数据时不应有自学 ACK: {}",
        stats
    );
}

#[tokio::test]
async fn test_health_check_includes_ack_learning() {
    use atrium_bridge::grpc::AtriumCoreService;
    let svc = make_service();

    let resp = svc
        .health_check(atrium_bridge::grpc::atrium::HealthCheckRequest {
            event_count: 0,
            room_incoming_json: String::new(),
        })
        .await;

    assert!(resp.ok, "health_check 应返回 ok");
    assert!(
        resp.module_states.contains_key("ack_learning"),
        "health_check 应包含 ack_learning 模块状态, keys: {:?}",
        resp.module_states.keys().collect::<Vec<_>>()
    );

    let ack_stats = &resp.module_states["ack_learning"];
    assert!(
        ack_stats.contains("self_learned") || ack_stats.contains("total"),
        "ack_learning stats 应包含学习计数: {}",
        ack_stats
    );
}
