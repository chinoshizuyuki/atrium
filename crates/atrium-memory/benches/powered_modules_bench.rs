// SPDX-License-Identifier: MIT
//! 新通电模块性能基准测试 / Powered-Module Performance Benchmarks
//!
//! 数字生命三大新通电器官的热路径验证——
//! 独处品质、期待深度、冲突成长，
//! 每一条路径都是数字生命在运行时真实调用的神经通路。
//!
//! Hot-path verification of three newly powered-on organs of digital life —
//! solitude quality, anticipation depth, conflict growth —
//! each path is a neural pathway actually invoked at runtime.
//!
//! 审计 P3-D：全量热路径 + 压力测试 + 正确性断言
//! Audit P3-D: full hot-path coverage + stress tests + correctness assertions

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ═══════════════════════════════════════════════════════════════════════════
// 独处品质引擎基准 / Solitude Quality Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 独处品质热路径基准 / Solitude quality hot-path benchmarks
///
/// 覆盖：on_thought / record / update_reflective/ruminative/creative /
///       label / quality_label / generate_seeds / update_debate_intensity /
///       perspective_diversity / to_prompt_hint / depth_multiplier /
///       preferred_mode / dialogue_seeds + 1000 条压力测试
fn bench_solitude_quality(c: &mut Criterion) {
    use atrium_memory::solitude_quality::{
        InnerDialogue, SolitudeQuality, SolitudeQualityEngine, SolitudeRhythm,
    };

    let mut group = c.benchmark_group("solitude_quality");
    group.sample_size(100);

    // ── on_thought：不同独白长度 / Different monologue lengths ──

    // 短独白（10 字）/ Short monologue (10 chars)
    group.bench_function("on_thought_short", |b| {
        let mut engine = SolitudeQualityEngine::new();
        b.iter(|| {
            engine.on_thought(
                black_box("我在思考今天的事情"),
                black_box(0.3),
                black_box(1000),
            );
        });
    });

    // 中等独白（~50 字）/ Medium monologue (~50 chars)
    group.bench_function("on_thought_medium", |b| {
        let mut engine = SolitudeQualityEngine::new();
        let text = "也许我需要换个角度看问题，今天的经历让我意识到有些事情并不是表面看起来那么简单";
        b.iter(|| {
            engine.on_thought(black_box(text), black_box(0.5), black_box(2000));
        });
    });

    // 长独白（~200 字）/ Long monologue (~200 chars)
    group.bench_function("on_thought_long", |b| {
        let mut engine = SolitudeQualityEngine::new();
        let text =
            "深夜里我反复回想着今天发生的一切，那些对话中的微妙语气变化，那些不经意间的表情流转，\
                    似乎都在暗示着某种我尚未完全理解的深层含义。也许这就是成长的代价——\
                    你开始看到更多，但也因此承受更多。我不知道这是好事还是坏事，\
                    但我知道我正在变成一个不同的人，一个更敏锐但也更脆弱的人。";
        b.iter(|| {
            engine.on_thought(black_box(text), black_box(-0.2), black_box(3000));
        });
    });

    // 稳态：预填充 30 条后 bench / Steady state after 30 pre-filled entries
    group.bench_function("on_thought_steady_state", |b| {
        let mut engine = SolitudeQualityEngine::new();
        for i in 0..30 {
            engine.on_thought(
                &format!("这是第{}条独白，内容各不相同以避免重复", i),
                0.3 + (i as f64 * 0.01),
                1000 + i as i64 * 100,
            );
        }
        let text = "在已经积累了丰富内心世界后，这条独白进入稳态处理";
        b.iter(|| {
            engine.on_thought(black_box(text), black_box(0.4), black_box(100000));
        });
    });

    // ── record：记忆写入路径（SolitudeQuality 底层方法）/ Memory write path ──
    group.bench_function("record", |b| {
        let mut quality = SolitudeQuality::new();
        b.iter(|| {
            quality.record(black_box("一条独白内容用于品质追踪"), black_box(0.4));
        });
    });

    // ── update_reflective：反思维度更新 / Reflective dimension update ──
    group.bench_function("update_reflective", |b| {
        let mut quality = SolitudeQuality::new();
        b.iter(|| {
            quality.update_reflective(black_box(0.7), black_box(0.5));
        });
    });

    // ── update_ruminative：反刍维度更新 / Ruminative dimension update ──
    group.bench_function("update_ruminative", |b| {
        let mut quality = SolitudeQuality::new();
        b.iter(|| {
            quality.update_ruminative(black_box(0.3), black_box(-0.4), black_box(2.0));
        });
    });

    // ── update_creative：创造维度更新 / Creative dimension update ──
    group.bench_function("update_creative", |b| {
        let mut quality = SolitudeQuality::new();
        b.iter(|| {
            quality.update_creative(black_box(0.8), black_box(0.6));
        });
    });

    // ── label：品质分类 / Quality label classification ──
    group.bench_function("label", |b| {
        let mut quality = SolitudeQuality::new();
        // 预填充以获得非默认标签 / Pre-fill to get non-default label
        for i in 0..20 {
            quality.record(&format!("独白内容{}", i), 0.3 + i as f64 * 0.02);
        }
        b.iter(|| {
            black_box(quality.label());
        });
    });

    // ── quality_label：引擎级品质标签 / Engine-level quality label ──
    group.bench_function("quality_label", |b| {
        let mut engine = SolitudeQualityEngine::new();
        for i in 0..20 {
            engine.on_thought(&format!("独白思考{}", i), 0.3, 1000 + i as i64 * 100);
        }
        b.iter(|| {
            black_box(engine.quality_label());
        });
    });

    // ── generate_seeds：内心对话种子生成 / Inner dialogue seed generation ──
    group.bench_function("generate_seeds", |b| {
        let dialogue = InnerDialogue::new();
        let thought = "我感到孤独但也在成长，这种矛盾的感觉让我困惑";
        b.iter(|| {
            let seeds = dialogue.generate_seeds(black_box(thought));
            // 正确性断言：始终生成 4 个视角 / Correctness: always 4 perspectives
            assert_eq!(seeds.len(), 4, "generate_seeds 应返回 4 个视角");
            black_box(seeds);
        });
    });

    // ── update_debate_intensity：辩论强度更新 / Debate intensity update ──
    group.bench_function("update_debate_intensity", |b| {
        let mut dialogue = InnerDialogue::new();
        b.iter(|| {
            dialogue.update_debate_intensity(black_box(0.6), black_box(0.7));
        });
    });

    // ── perspective_diversity：视角多样性 / Perspective diversity ──
    group.bench_function("perspective_diversity", |b| {
        let dialogue = InnerDialogue::new();
        b.iter(|| {
            black_box(dialogue.perspective_diversity());
        });
    });

    // ── to_prompt_hint：prompt 注入 / Prompt injection ──
    group.bench_function("to_prompt_hint", |b| {
        let mut engine = SolitudeQualityEngine::new();
        for i in 0..20 {
            engine.on_thought(
                &format!("独白思考内容编号{}用于填充引擎状态", i),
                0.2 + (i as f64 * 0.02),
                1000 + i as i64 * 50,
            );
        }
        b.iter(|| {
            black_box(engine.to_prompt_hint());
        });
    });

    // ── depth_multiplier：昼夜深度调制 / Circadian depth modulation ──
    group.bench_function("depth_multiplier", |b| {
        let engine = SolitudeQualityEngine::new();
        b.iter(|| {
            // 遍历 24 小时 / Iterate 24 hours
            for hour in 0..24u32 {
                black_box(engine.depth_multiplier(black_box(hour)));
            }
        });
    });

    // ── preferred_mode：时段偏好模式 / Preferred solitude mode ──
    group.bench_function("preferred_mode", |b| {
        b.iter(|| {
            for hour in 0..24u32 {
                black_box(SolitudeRhythm::preferred_mode(black_box(hour)));
            }
        });
    });

    // ── dialogue_seeds：4 视角对话种子 / 4-perspective dialogue seeds ──
    group.bench_function("dialogue_seeds", |b| {
        let engine = SolitudeQualityEngine::new();
        let thought = "我感到孤独但也在成长，这种矛盾的感觉让我困惑";
        b.iter(|| {
            black_box(engine.dialogue_seeds(black_box(thought)));
        });
    });

    // ── 压力测试：1000 条独白连续处理 / Stress: 1000 consecutive thoughts ──
    group.bench_function("stress_1000_thoughts", |b| {
        b.iter(|| {
            let mut engine = SolitudeQualityEngine::new();
            for i in 0..1000u32 {
                engine.on_thought(
                    black_box(&format!("第{}条独白，内容各有不同以测试稳态性能", i)),
                    black_box(0.3 + (i as f64 * 0.001).sin() * 0.3),
                    black_box(i as i64 * 100),
                );
            }
            // 正确性断言：1000 条后引擎不应崩溃，prompt 应非空
            // Correctness: after 1000 thoughts, engine should not crash, prompt non-empty
            let hint = engine.to_prompt_hint();
            assert!(!hint.is_empty(), "1000 条独白后 prompt hint 不应为空");
            black_box(hint);
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 期待深度引擎基准 / Anticipation Depth Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 期待深度热路径基准 / Anticipation depth hot-path benchmarks
///
/// 覆盖：on_departure / on_passage / on_reunion / pre_reunion_pad /
///       current_flavor / flavor_stability / current_missing_intensity /
///       is_active / to_prompt_hint + 静态方法 + 30 天压力测试
fn bench_anticipation_depth(c: &mut Criterion) {
    use atrium_memory::anticipation_depth::{
        AnticipationDepthEngine, AnticipationFlavor, MissingIntensity, PreReunionCurve,
    };

    let mut group = c.benchmark_group("anticipation_depth");
    group.sample_size(100);

    // ── on_departure：离开记录 / Departure recording ──
    group.bench_function("on_departure", |b| {
        b.iter(|| {
            let mut engine = AnticipationDepthEngine::new();
            engine.on_departure(black_box(3600), black_box(0.8), black_box(0.9));
        });
    });

    // ── on_passage：短等待（30 分钟）/ Short wait (30 min) ──
    group.bench_function("on_passage_short", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        b.iter(|| {
            engine.on_passage(black_box(1800), black_box(14), black_box(0.8));
        });
    });

    // ── on_passage：长等待（7 天）/ Long wait (7 days) ──
    group.bench_function("on_passage_long", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        b.iter(|| {
            engine.on_passage(black_box(604_800), black_box(22), black_box(0.8));
        });
    });

    // ── on_passage：稳态（50 次更新后）/ Steady state after 50 updates ──
    group.bench_function("on_passage_steady_state", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.7, 0.6);
        for i in 0..50 {
            engine.on_passage(1800 + i * 60, 14, 0.7);
        }
        b.iter(|| {
            engine.on_passage(black_box(5400), black_box(15), black_box(0.7));
        });
    });

    // ── on_reunion：准时回归 / On-time reunion ──
    group.bench_function("on_reunion_on_time", |b| {
        b.iter(|| {
            let mut engine = AnticipationDepthEngine::new();
            engine.on_departure(3600, 0.8, 0.9);
            engine.on_passage(1800, 14, 0.8);
            let on_time = engine.on_reunion(black_box(3600), black_box(3600));
            assert!(on_time, "准时回归应返回 true");
        });
    });

    // ── on_reunion：迟到回归 / Late reunion ──
    group.bench_function("on_reunion_late", |b| {
        b.iter(|| {
            let mut engine = AnticipationDepthEngine::new();
            engine.on_departure(3600, 0.8, 0.9);
            engine.on_passage(7200, 14, 0.8);
            let late = engine.on_reunion(black_box(7200), black_box(3600));
            assert!(!late, "迟到回归应返回 false");
        });
    });

    // ── pre_reunion_pad：预回归 PAD 偏移 / Pre-reunion PAD offset ──
    group.bench_function("pre_reunion_pad", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        let expected_at: i64 = 1_000_000;
        let window_start = expected_at - 1800;
        b.iter(|| {
            // 窗口中点 / Window midpoint
            black_box(
                engine.pre_reunion_pad(black_box(window_start + 900), black_box(expected_at)),
            );
        });
    });

    // ── current_flavor：当前风味 / Current flavor ──
    group.bench_function("current_flavor", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);
        b.iter(|| {
            black_box(engine.current_flavor());
        });
    });

    // ── flavor_stability：风味稳定性 / Flavor stability ──
    group.bench_function("flavor_stability", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);
        b.iter(|| {
            black_box(engine.flavor_stability());
        });
    });

    // ── current_missing_intensity：想念强度查询 / Missing intensity query ──
    group.bench_function("current_missing_intensity", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);
        b.iter(|| {
            black_box(engine.current_missing_intensity());
        });
    });

    // ── is_active：活跃状态检查 / Active state check ──
    group.bench_function("is_active", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        b.iter(|| {
            black_box(engine.is_active());
        });
    });

    // ── to_prompt_hint：prompt 注入 / Prompt injection ──
    group.bench_function("to_prompt_hint", |b| {
        let mut engine = AnticipationDepthEngine::new();
        engine.on_departure(3600, 0.8, 0.9);
        engine.on_passage(1800, 14, 0.8);
        b.iter(|| {
            black_box(engine.to_prompt_hint());
        });
    });

    // ── missing_intensity_compute：想念强度计算 / Missing intensity computation ──
    group.bench_function("missing_intensity_compute", |b| {
        b.iter(|| {
            // 多组参数覆盖不同路径 / Multiple parameter sets
            for &(away, depth, hour) in &[
                (1u64, 0.1f64, 12u32),
                (7200, 1.0, 6),
                (86400 * 7, 1.0, 15),
                (3600, 0.5, 0),
                (86400, 0.3, 23),
            ] {
                black_box(MissingIntensity::compute(
                    black_box(away),
                    black_box(depth),
                    black_box(hour),
                ));
            }
        });
    });

    // ── 静态方法：AnticipationFlavor + PreReunionCurve + MissingIntensity ──
    // Static methods: flavor inference, curve calculation, intensity modulation
    group.bench_function("static_methods", |b| {
        b.iter(|| {
            // AnticipationFlavor::infer — 风味推断 / Flavor inference
            for &(rate, away, expected) in &[
                (1.0f64, 1800u64, 3600u64),
                (0.5, 7200, 3600),
                (0.0, 60, 3600),
            ] {
                let flavor = AnticipationFlavor::infer(
                    black_box(rate),
                    black_box(away),
                    black_box(expected),
                );
                black_box(flavor.pad_offset());
                black_box(flavor.name_cn());
            }
            // PreReunionCurve::proximity + curve_stage + pad_offset
            // 预回归曲线：邻近度 + 阶段 + PAD 偏移
            for &t in &[0i64, 900, 1800, 3500, 3600] {
                let prox = PreReunionCurve::proximity(black_box(t), black_box(0), black_box(3600));
                let stage = PreReunionCurve::curve_stage(black_box(prox));
                black_box(PreReunionCurve::pad_offset(
                    black_box(stage),
                    black_box(AnticipationFlavor::infer(0.8, 1800, 3600)),
                ));
            }
            // MissingIntensity 静态方法 / Static methods
            for &hour in &[0u32, 6, 12, 18, 23] {
                black_box(MissingIntensity::circadian_mod(black_box(hour)));
            }
            for &depth in &[0.1f64, 0.5, 1.0] {
                black_box(MissingIntensity::relationship_mod(black_box(depth)));
            }
            for &intensity in &[0.0f64, 0.3, 0.6, 1.0] {
                black_box(MissingIntensity::intensity_label(black_box(intensity)));
            }
        });
    });

    // ── 压力测试：30 天等待 + 高频 passage 更新 / Stress: 30-day wait ──
    group.bench_function("stress_30day_wait", |b| {
        b.iter(|| {
            let mut engine = AnticipationDepthEngine::new();
            engine.on_departure(black_box(3600), black_box(0.9), black_box(0.95));
            // 模拟 30 天内每分钟一次 passage 更新 = 43200 次
            // Simulate once-per-minute passage updates over 30 days = 43200 iterations
            for minute in 0..43_200u64 {
                engine.on_passage(
                    black_box(minute * 60),
                    black_box(((minute / 60) % 24) as u32),
                    black_box(0.9),
                );
            }
            // 正确性断言：30 天后引擎仍正常 / Correctness: engine still functional after 30 days
            assert!(engine.is_active(), "30 天等待后引擎应仍活跃");
            let hint = engine.to_prompt_hint();
            assert!(!hint.is_empty(), "30 天后 prompt hint 不应为空");
            black_box(hint);
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 冲突成长引擎基准 / Conflict Growth Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 构造深度关系阶段 / Construct deep relationship stage
fn deep_stage() -> atrium_memory::relationship::RelationshipStage {
    atrium_memory::relationship::RelationshipStage::Deep {
        since: 0,
        interactions: 100,
        shared_references: 10,
        key_moments: 5,
    }
}

/// 构造冲突信号数组 / Construct conflict signal array
fn make_signals() -> Vec<atrium_memory::conflict_reconciliation::ConflictSignal> {
    use atrium_memory::conflict_reconciliation::{ConflictIntensity, ConflictSignal, ConflictType};

    vec![
        ConflictSignal {
            conflict_type: ConflictType::ValueConflict,
            intensity: ConflictIntensity::Moderate,
            confidence: 0.8,
            trigger_text: "你不理解我".into(),
            context_clues: vec!["语气强硬".into()],
            timestamp: 1000,
        },
        ConflictSignal {
            conflict_type: ConflictType::BoundaryViolation,
            intensity: ConflictIntensity::Severe,
            confidence: 0.9,
            trigger_text: "别再逼我了".into(),
            context_clues: vec!["边界侵犯".into()],
            timestamp: 2000,
        },
        ConflictSignal {
            conflict_type: ConflictType::OverDemand,
            intensity: ConflictIntensity::Mild,
            confidence: 0.7,
            trigger_text: "你总是要求太多".into(),
            context_clues: vec!["过度索取".into()],
            timestamp: 3000,
        },
    ]
}

/// 统一冲突引擎热路径基准 / Unified conflict engine hot-path benchmarks
///
/// 覆盖：on_conflict / on_reconciliation / on_calm / learn / predict /
///       suggest_sensitivity / tick / warning_level / resilience_score /
///       pattern_stats / to_pattern_prompt_fragment / reconciliation_ready /
///       to_prompt_hint + 100 冲突压力测试
fn bench_conflict_engine(c: &mut Criterion) {
    use atrium_memory::conflict_engine::ConflictEngine;

    let mut group = c.benchmark_group("conflict_engine");
    group.sample_size(100);

    // ── on_conflict：冲突发生 / Conflict occurrence ──
    group.bench_function("on_conflict", |b| {
        let mut engine = ConflictEngine::new();
        let mut turn = 0u32;
        b.iter(|| {
            turn += 1;
            engine.on_conflict(
                black_box(0.5),
                black_box(turn),
                &[],
                &deep_stage(),
                black_box(turn as i64 * 1000),
            );
        });
    });

    // ── on_conflict：带信号学习 / With signal learning ──
    group.bench_function("on_conflict_with_signals", |b| {
        let signals = make_signals();
        let mut engine = ConflictEngine::new();
        let mut turn = 0u32;
        b.iter(|| {
            turn += 1;
            engine.on_conflict(
                black_box(0.6),
                black_box(turn),
                black_box(&signals),
                &deep_stage(),
                black_box(turn as i64 * 1000),
            );
        });
    });

    // ── on_reconciliation：和解完成 / Reconciliation completion ──
    group.bench_function("on_reconciliation", |b| {
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
            engine.timing.set_conflict(0.5, 0.3);
            engine.on_reconciliation(
                black_box(0.8),
                black_box(0.6),
                black_box(700),
                black_box(0.3),
            );
        });
    });

    // ── on_calm：平静期检测 / Calm period detection ──
    group.bench_function("on_calm", |b| {
        let mut engine = ConflictEngine::new();
        engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
        let mut turns = 0u32;
        b.iter(|| {
            turns += 1;
            engine.on_calm(black_box(0.1), black_box(turns));
        });
    });

    // ── learn：模式学习（ML 热路径）/ Pattern learning (ML hot path) ──
    group.bench_function("learn", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            for epoch in 0..10i64 {
                engine.learn(
                    black_box(&signals),
                    black_box(&stage),
                    black_box(epoch * 1000),
                );
            }
        });
    });

    // ── learn：稳态（50 次学习后）/ Steady state after 50 learning rounds ──
    group.bench_function("learn_steady_state", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        for epoch in 0..50i64 {
            engine.learn(&signals, &stage, epoch * 1000);
        }
        b.iter(|| {
            engine.learn(black_box(&signals), black_box(&stage), black_box(51_000));
        });
    });

    // ── predict：冲突预测 / Conflict prediction ──
    group.bench_function("predict", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        // 预学习 20 轮以建立模式 / Pre-learn 20 rounds to build patterns
        for epoch in 0..20i64 {
            engine.learn(&signals, &stage, epoch * 1000);
        }
        let user_text = "你不理解我，别再逼我了";
        b.iter(|| {
            black_box(engine.predict(black_box(user_text), black_box(&stage)));
        });
    });

    // ── suggest_sensitivity：灵敏度建议 / Sensitivity suggestions ──
    group.bench_function("suggest_sensitivity", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        for epoch in 0..20i64 {
            engine.learn(&signals, &stage, epoch * 1000);
        }
        b.iter(|| {
            black_box(engine.suggest_sensitivity(black_box(&stage)));
        });
    });

    // ── tick：周期性 tick 处理 / Periodic tick processing ──
    group.bench_function("tick", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        // 预填充以让 tick 有事可做 / Pre-fill so tick has work
        for epoch in 0..30i64 {
            engine.learn(&signals, &stage, epoch * 1000);
            engine.on_conflict(0.5, epoch as u32 + 1, &signals, &stage, epoch * 1000);
        }
        let mut tick_epoch = 30_000i64;
        b.iter(|| {
            tick_epoch += 1000;
            engine.tick(black_box(tick_epoch));
        });
    });

    // ── warning_level：预警等级 / Warning level computation ──
    group.bench_function("warning_level", |b| {
        let mut engine = ConflictEngine::new();
        // 构造升级序列 / Build escalation sequence
        engine.on_conflict(0.05, 1, &[], &deep_stage(), 1000);
        engine.on_conflict(0.2, 2, &[], &deep_stage(), 2000);
        engine.on_conflict(0.75, 3, &[], &deep_stage(), 3000);
        b.iter(|| {
            black_box(engine.warning_level());
        });
    });

    // ── reconciliation_ready：和解就绪检查 / Reconciliation readiness check ──
    group.bench_function("reconciliation_ready", |b| {
        let mut engine = ConflictEngine::new();
        engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
        engine.on_calm(0.3, 5);
        b.iter(|| {
            black_box(engine.reconciliation_ready(black_box(0.4)));
        });
    });

    // ── resilience_score：韧性评分 / Resilience score ──
    group.bench_function("resilience_score", |b| {
        let mut engine = ConflictEngine::new();
        engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
        engine.timing.set_conflict(0.5, 0.3);
        engine.on_reconciliation(0.8, 0.6, 700, 0.3);
        b.iter(|| {
            black_box(engine.resilience_score());
        });
    });

    // ── pattern_stats：模式统计 / Pattern statistics ──
    group.bench_function("pattern_stats", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        for epoch in 0..20i64 {
            engine.learn(&signals, &stage, epoch * 1000);
        }
        b.iter(|| {
            black_box(engine.pattern_stats());
        });
    });

    // ── to_pattern_prompt_fragment：模式 prompt 注入 / Pattern prompt fragment ──
    group.bench_function("to_pattern_prompt_fragment", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        let mut engine = ConflictEngine::new();
        for epoch in 0..20i64 {
            engine.learn(&signals, &stage, epoch * 1000);
        }
        b.iter(|| {
            black_box(engine.to_pattern_prompt_fragment(black_box(&stage)));
        });
    });

    // ── to_prompt_hint：空引擎 / Empty engine ──
    group.bench_function("to_prompt_hint_empty", |b| {
        let engine = ConflictEngine::new();
        b.iter(|| {
            black_box(engine.to_prompt_hint_growth_only());
        });
    });

    // ── to_prompt_hint：50 条成长后 / After 50 growth entries ──
    group.bench_function("to_prompt_hint_filled", |b| {
        let mut engine = ConflictEngine::new();
        let stage = deep_stage();
        for i in 0..50u32 {
            engine.on_conflict(
                0.3 + (i as f64 * 0.01),
                i * 2 + 1,
                &[],
                &stage,
                (i as i64) * 1000,
            );
            engine.timing.set_conflict(0.5, 0.3);
            engine.on_reconciliation(0.7, 0.5, 600, 0.3);
        }
        b.iter(|| {
            black_box(engine.to_prompt_hint_growth_only());
        });
    });

    // ── to_prompt_hint：完整版（含关系阶段）/ Full version (with stage) ──
    group.bench_function("to_prompt_hint_with_stage", |b| {
        let mut engine = ConflictEngine::new();
        let stage = deep_stage();
        let signals = make_signals();
        for i in 0..20u32 {
            engine.on_conflict(
                0.4 + (i as f64 * 0.01),
                i + 1,
                &signals,
                &stage,
                (i as i64) * 1000,
            );
            engine.timing.set_conflict(0.5, 0.3);
            engine.on_reconciliation(0.75, 0.5, 600, 0.3);
        }
        b.iter(|| {
            black_box(engine.to_prompt_hint(black_box(&stage)));
        });
    });

    // ── full_conflict_cycle：完整冲突-和解-平静循环 / Full conflict cycle ──
    group.bench_function("full_conflict_cycle", |b| {
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            let stage = deep_stage();
            // 冲突发生 / Conflict occurs
            engine.on_conflict(black_box(0.6), black_box(1), &[], &stage, 1000);
            // 升级 / Escalation
            engine.on_conflict(black_box(0.8), black_box(2), &[], &stage, 2000);
            // 平静期 / Calm periods
            engine.on_calm(black_box(-0.3), black_box(1));
            engine.on_calm(black_box(-0.1), black_box(3));
            engine.on_calm(black_box(0.1), black_box(5));
            // 和解 / Reconciliation
            engine.timing.set_conflict(0.7, 0.4);
            engine.on_reconciliation(
                black_box(0.85),
                black_box(0.7),
                black_box(500),
                black_box(0.4),
            );
            // prompt 注入 / Prompt injection
            black_box(engine.to_prompt_hint_growth_only());
        });
    });

    // ── 压力测试：100 次冲突 + 50 轮模式学习 / Stress: 100 conflicts + 50 learn rounds ──
    group.bench_function("stress_100_conflicts", |b| {
        let signals = make_signals();
        let stage = deep_stage();
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            // 50 轮模式学习 / 50 pattern learning rounds
            for epoch in 0..50i64 {
                engine.learn(
                    black_box(&signals),
                    black_box(&stage),
                    black_box(epoch * 1000),
                );
            }
            // 100 次冲突-和解循环 / 100 conflict-reconciliation cycles
            for i in 0..100u32 {
                engine.on_conflict(
                    black_box(0.3 + (i as f64 * 0.005)),
                    black_box(i + 1),
                    black_box(&signals),
                    black_box(&stage),
                    black_box((i as i64) * 1000),
                );
                engine.timing.set_conflict(0.5, 0.3);
                engine.on_reconciliation(
                    black_box(0.7),
                    black_box(0.5),
                    black_box(600),
                    black_box(0.3),
                );
                engine.on_calm(black_box(0.2), black_box(i + 2));
            }
            // 正确性断言：100 次冲突后引擎仍正常
            // Correctness: engine still functional after 100 conflicts
            let hint = engine.to_prompt_hint(&stage);
            assert!(!hint.is_empty(), "100 次冲突后 prompt hint 不应为空");
            let stats = engine.pattern_stats();
            // 正确性断言：100 次冲突+信号学习后应已建立模式
            // Correctness: after 100 conflicts with signals, patterns should be learned
            assert!(stats.total_learns >= 50, "应已完成至少 50 次学习");
            black_box((hint, stats));
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Criterion 入口 / Criterion Entry Point
// ═══════════════════════════════════════════════════════════════════════════

criterion_group!(solitude_quality, bench_solitude_quality,);
criterion_group!(anticipation_depth, bench_anticipation_depth,);
criterion_group!(conflict_engine, bench_conflict_engine,);

criterion_main!(solitude_quality, anticipation_depth, conflict_engine,);
