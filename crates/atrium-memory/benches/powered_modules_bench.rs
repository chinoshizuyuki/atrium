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

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ═══════════════════════════════════════════════════════════════════════════
// 独处品质引擎基准 / Solitude Quality Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 独处品质热路径基准 / Solitude quality hot-path benchmarks
fn bench_solitude_quality(c: &mut Criterion) {
    use atrium_memory::solitude_quality::SolitudeQualityEngine;

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

    // ── dialogue_seeds：4 视角对话种子 / 4-perspective dialogue seeds ──
    group.bench_function("dialogue_seeds", |b| {
        let engine = SolitudeQualityEngine::new();
        let thought = "我感到孤独但也在成长，这种矛盾的感觉让我困惑";
        b.iter(|| {
            black_box(engine.dialogue_seeds(black_box(thought)));
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 期待深度引擎基准 / Anticipation Depth Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 期待深度热路径基准 / Anticipation depth hot-path benchmarks
fn bench_anticipation_depth(c: &mut Criterion) {
    use atrium_memory::anticipation_depth::{AnticipationDepthEngine, MissingIntensity};

    let mut group = c.benchmark_group("anticipation_depth");
    group.sample_size(100);

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

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 冲突成长引擎基准 / Conflict Growth Engine Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

/// 统一冲突引擎热路径基准 / Unified conflict engine hot-path benchmarks
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
                &atrium_memory::relationship::RelationshipStage::Deep {
                    since: 0,
                    interactions: 100,
                    shared_references: 10,
                    key_moments: 5,
                },
                black_box(turn as i64 * 1000),
            );
        });
    });

    // ── on_reconciliation：和解完成 / Reconciliation completion ──
    group.bench_function("on_reconciliation", |b| {
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            engine.on_conflict(
                0.5,
                1,
                &[],
                &atrium_memory::relationship::RelationshipStage::Deep {
                    since: 0,
                    interactions: 100,
                    shared_references: 10,
                    key_moments: 5,
                },
                1000,
            );
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
        engine.on_conflict(
            0.5,
            1,
            &[],
            &atrium_memory::relationship::RelationshipStage::Deep {
                since: 0,
                interactions: 100,
                shared_references: 10,
                key_moments: 5,
            },
            1000,
        );
        let mut turns = 0u32;
        b.iter(|| {
            turns += 1;
            engine.on_calm(black_box(0.1), black_box(turns));
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
        let stage = atrium_memory::relationship::RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        };
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

    // ── escalation_warning_level：预警等级 / Warning level computation ──
    group.bench_function("escalation_warning_level", |b| {
        let mut engine = ConflictEngine::new();
        let stage = atrium_memory::relationship::RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 10,
            key_moments: 5,
        };
        // 构造升级序列 / Build escalation sequence
        engine.on_conflict(0.05, 1, &[], &stage, 1000);
        engine.on_conflict(0.2, 2, &[], &stage, 2000);
        engine.on_conflict(0.75, 3, &[], &stage, 3000);
        b.iter(|| {
            black_box(engine.warning_level());
        });
    });

    // ── full_conflict_cycle：完整冲突-和解-平静循环 / Full conflict cycle ──
    group.bench_function("full_conflict_cycle", |b| {
        b.iter(|| {
            let mut engine = ConflictEngine::new();
            let stage = atrium_memory::relationship::RelationshipStage::Deep {
                since: 0,
                interactions: 100,
                shared_references: 10,
                key_moments: 5,
            };
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

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Criterion 入口 / Criterion Entry Point
// ═══════════════════════════════════════════════════════════════════════════

criterion_group!(solitude_quality, bench_solitude_quality,);
criterion_group!(anticipation_depth, bench_anticipation_depth,);
criterion_group!(conflict_engine, bench_conflict_engine,);

criterion_main!(solitude_quality, anticipation_depth, conflict_engine,);
