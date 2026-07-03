// SPDX-License-Identifier: MIT
//! 记忆系统性能基准测试 / Memory System Performance Benchmarks
//!
//! 数字生命的认知核心性能验证——
//! 关联扩散、犯错决策、自纠闭环、非理性涌现、记忆存取，
//! 每一条路径都是记忆→联想→涌现的神经通路。
//!
//! Performance verification of the cognitive core of digital life —
//! associative spread, imperfection decision, self-correction loop,
//! irrationality emergence, memory I/O —
//! each path is a memory→association→emergence neural pathway.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Instant;

// ── 关联图扩散基准 / Associative Graph Spread Benchmarks ──

fn bench_graph_spread(c: &mut Criterion) {
    use atrium_memory::associative::AssociativeGraph;
    use atrium_memory::fact_store::Fact;

    let mut group = c.benchmark_group("graph_spread");
    group.sample_size(50);

    // 不同图规模的扩散 / Spread at different graph sizes
    for &n_facts in &[10, 50, 200] {
        let mut graph = AssociativeGraph::new();

        // 构建图：n_facts 条事实 + 关系 / Build graph: n_facts facts + relations
        for i in 0..n_facts {
            let fact = Fact::new(
                format!("entity_{}", i),
                format!("rel_{}", i % 5),
                format!("target_{}", i),
            )
            .with_confidence(0.8);
            graph.add_fact(&fact);
        }

        group.bench_with_input(
            BenchmarkId::new("spread_activation", n_facts),
            &n_facts,
            |b, &_| {
                b.iter(|| {
                    graph.spread_activation(
                        black_box("entity_0"),
                        black_box(0.7), // 衰减率 / Decay rate
                        black_box(3),   // 最大跳数 / Max hops
                    );
                });
            },
        );
    }

    // 不同跳数的扩散 / Spread at different hop depths
    let mut graph = AssociativeGraph::new();
    for i in 0..100 {
        let fact = Fact::new(
            format!("s_{}", i),
            format!("p_{}", i % 10),
            format!("o_{}", i),
        );
        graph.add_fact(&fact);
    }

    for &max_hops in &[1, 2, 3, 5] {
        group.bench_with_input(
            BenchmarkId::new("spread_hops", max_hops),
            &max_hops,
            |b, &hops| {
                b.iter(|| {
                    graph.spread_activation(black_box("s_0"), black_box(0.7), black_box(hops));
                });
            },
        );
    }

    group.finish();
}

// ── ImperfectionEngine 犯错决策基准 / Imperfection Decision Benchmarks ──

fn bench_imperfection_decision(c: &mut Criterion) {
    use atrium_memory::imperfection_engine::{ImperfectionConfig, ImperfectionEngine};

    let mut group = c.benchmark_group("imperfection");
    group.sample_size(100);

    // 门控检查 / Gate check
    group.bench_function("check_gate", |b| {
        let config = ImperfectionConfig::default();
        let engine = ImperfectionEngine::new_deterministic(config, 42);
        let now = Instant::now();
        b.iter(|| {
            engine.check_gate(black_box(now));
        });
    });

    // 概率计算 / Probability computation
    group.bench_function("compute_probability", |b| {
        let config = ImperfectionConfig::default();
        let engine = ImperfectionEngine::new_deterministic(config, 42);
        b.iter(|| {
            // 遍历所有 5 种犯错类型 / Iterate all 5 mistake kinds
            use atrium_memory::imperfection_engine::MistakeKind;
            for kind in [
                MistakeKind::MemoryDrift,
                MistakeKind::ReasoningLeap,
                MistakeKind::OverSimplification,
                MistakeKind::IntentionalVagueness,
                MistakeKind::KnowledgeBoundary,
            ] {
                black_box(engine.compute_probability(black_box(kind), black_box("编程")));
            }
        });
    });

    // 犯错决策（含门控+概率+随机） / Full mistake decision (gate + probability + random)
    group.bench_function("decide_mistake", |b| {
        let config = ImperfectionConfig::default();
        let mut engine = ImperfectionEngine::new_deterministic(config, 42);
        let now = Instant::now();
        b.iter(|| {
            black_box(engine.decide_mistake(black_box("编程"), black_box(now)));
        });
    });

    // 不同认知域的犯错决策 / Decision across different domains
    for domain in ["编程", "情感", "哲学", "日常", "未知领域"] {
        group.bench_function(format!("decide_mistake_{}", domain), |b| {
            let config = ImperfectionConfig::default();
            let mut engine = ImperfectionEngine::new_deterministic(config, 42);
            let now = Instant::now();
            b.iter(|| {
                black_box(engine.decide_mistake(black_box(domain), black_box(now)));
            });
        });
    }

    group.finish();
}

// ── ImperfectionEngine 自纠闭环基准 / Self-Correction Loop Benchmarks ──

fn bench_imperfection_tick(c: &mut Criterion) {
    use atrium_memory::imperfection_engine::{
        ImperfectionConfig, ImperfectionEngine, MistakeKind, MistakeSeverity,
    };

    let mut group = c.benchmark_group("imperfection_tick");
    group.sample_size(100);

    // tick（无待纠错） / Tick with no pending corrections
    group.bench_function("tick_empty", |b| {
        let config = ImperfectionConfig::default();
        let mut engine = ImperfectionEngine::new_deterministic(config, 42);
        let now = Instant::now();
        b.iter(|| {
            black_box(engine.tick(black_box(now)));
        });
    });

    // tick（有待纠错） / Tick with pending corrections
    group.bench_function("tick_with_pending", |b| {
        b.iter(|| {
            let config = ImperfectionConfig::default();
            let mut engine = ImperfectionEngine::new_deterministic(config, 42);
            // 注入一条待纠错记录 / Inject a pending correction
            let now = Instant::now();
            let decision = engine.decide_mistake("编程", now);
            if decision.should_mistake {
                if let (Some(kind), Some(severity), Some(trigger)) =
                    (decision.kind, decision.severity, decision.trigger)
                {
                    engine.record_mistake(
                        kind,
                        severity,
                        trigger,
                        decision.probability,
                        "编程",
                        now,
                    );
                }
            }
            // 推进时间使自纠到期 / Advance time so correction becomes due
            let later = now + std::time::Duration::from_secs(20);
            black_box(engine.tick(black_box(later)));
        });
    });

    // next_correction_prompt / Correction prompt extraction
    group.bench_function("next_correction_prompt", |b| {
        let config = ImperfectionConfig::default();
        let mut engine = ImperfectionEngine::new_deterministic(config, 42);
        let now = Instant::now();
        // 注入犯错记录使 prompt 可用 / Inject mistake to make prompt available
        let decision = engine.decide_mistake("编程", now);
        if decision.should_mistake {
            if let (Some(kind), Some(severity), Some(trigger)) =
                (decision.kind, decision.severity, decision.trigger)
            {
                engine.record_mistake(kind, severity, trigger, decision.probability, "编程", now);
            }
        }
        let later = now + std::time::Duration::from_secs(20);
        engine.tick(later);
        b.iter(|| {
            black_box(engine.next_correction_prompt());
        });
    });

    // prompt_fragment / Prompt fragment generation
    group.bench_function("prompt_fragment", |b| {
        let config = ImperfectionConfig::default();
        let engine = ImperfectionEngine::new_deterministic(config, 42);
        b.iter(|| {
            black_box(engine.prompt_fragment(
                black_box(MistakeKind::KnowledgeBoundary),
                black_box(MistakeSeverity::Moderate),
            ));
        });
    });

    group.finish();
}

// ── ImperfectionEngine 状态调制基准 / State Modulation Benchmarks ──

fn bench_imperfection_modulation(c: &mut Criterion) {
    use atrium_memory::imperfection_engine::{ImperfectionConfig, ImperfectionEngine};

    let mut group = c.benchmark_group("imperfection_modulation");
    group.sample_size(100);

    // 不同关系深度的概率调制 / Probability modulation at different relationship depths
    for &depth in &[0.1, 0.3, 0.5, 0.7, 0.9] {
        group.bench_function(format!("relationship_depth_{:.0}pct", depth * 100.0), |b| {
            let config = ImperfectionConfig::default();
            let mut engine = ImperfectionEngine::new_deterministic(config, 42);
            engine.set_relationship_depth(depth);
            let now = Instant::now();
            b.iter(|| {
                black_box(engine.decide_mistake(black_box("编程"), black_box(now)));
            });
        });
    }

    // 不同成熟度的概率调制 / Probability modulation at different maturity levels
    for &(ordinal, label) in &[(0, "Naive"), (1, "Growing"), (2, "Mature"), (3, "Wise")] {
        group.bench_function(format!("maturity_{}", label), |b| {
            let config = ImperfectionConfig::default();
            let mut engine = ImperfectionEngine::new_deterministic(config, 42);
            engine.set_maturity_ordinal(ordinal);
            let now = Instant::now();
            b.iter(|| {
                black_box(engine.decide_mistake(black_box("编程"), black_box(now)));
            });
        });
    }

    group.finish();
}

// ── 短期记忆存取基准 / STM I/O Benchmarks ──

fn bench_stm(c: &mut Criterion) {
    use atrium_memory::{MemoryContent, MemoryEntry, StmBuffer};

    let mut group = c.benchmark_group("stm");
    group.sample_size(100);

    // push / 压入
    group.bench_function("push", |b| {
        let mut buf = StmBuffer::new(1000);
        b.iter(|| {
            buf.push(MemoryEntry::new(
                "user",
                MemoryContent::Text("hello".into()),
            ));
        });
    });

    // recent / 取最近
    group.bench_function("recent_10", |b| {
        let mut buf = StmBuffer::new(1000);
        for i in 0..500 {
            buf.push(MemoryEntry::new(
                "user",
                MemoryContent::Text(format!("msg_{}", i)),
            ));
        }
        b.iter(|| {
            black_box(buf.recent(black_box(10)));
        });
    });

    group.finish();
}

// ── 长期记忆存取基准 / LTM I/O Benchmarks ──

fn bench_ltm(c: &mut Criterion) {
    use atrium_memory::{LtmStore, MemoryContent, MemoryEntry, SledLtm};

    let mut group = c.benchmark_group("ltm");
    group.sample_size(50); // sled I/O 较慢，减少采样 / sled I/O slower, reduce samples

    // insert / 写入
    group.bench_function("insert", |b| {
        let mut ltm = SledLtm::open_in_memory();
        b.iter(|| {
            ltm.insert(&MemoryEntry::new(
                "user",
                MemoryContent::Text("persist me".into()),
            ))
            .unwrap();
        });
    });

    // get / 读取
    group.bench_function("get", |b| {
        let mut ltm = SledLtm::open_in_memory();
        // 预写入 100 条 / Pre-write 100 entries
        for i in 0..100 {
            ltm.insert(&MemoryEntry::new(
                "user",
                MemoryContent::Text(format!("entry_{}", i)),
            ))
            .unwrap();
        }
        b.iter(|| {
            black_box(ltm.get(black_box(50)).unwrap());
        });
    });

    group.finish();
}

// ── 非理性涌现基准 / Irrationality Emergence Benchmarks ──

fn bench_irrationality(c: &mut Criterion) {
    use atrium_memory::emotional_irrationality::{
        IrrationalityConfig, IrrationalityManager, RandomMode,
    };

    let mut group = c.benchmark_group("irrationality");
    group.sample_size(50);

    // tick / 非理性 tick（四引擎联合）
    group.bench_function("tick", |b| {
        let config = IrrationalityConfig::default();
        let mut irr = IrrationalityManager::new(config)
            .with_random_mode(RandomMode::Deterministic { seed: 42 });
        let now = 1700000000; // 固定时间戳 / Fixed timestamp
        let pad = [0.3, 0.5, -0.2];
        b.iter(|| {
            irr.tick(black_box(&pad), black_box(now));
        });
    });

    // to_prompt_fragment / 非理性 prompt 片段
    group.bench_function("to_prompt_fragment", |b| {
        let config = IrrationalityConfig::default();
        let irr = IrrationalityManager::new(config)
            .with_random_mode(RandomMode::Deterministic { seed: 42 });
        let now = 1700000000;
        b.iter(|| {
            black_box(irr.to_prompt_fragment(black_box(now)));
        });
    });

    group.finish();
}

criterion_group!(graph_spread, bench_graph_spread,);
criterion_group!(imperfection, bench_imperfection_decision,);
criterion_group!(imperfection_tick, bench_imperfection_tick,);
criterion_group!(imperfection_modulation, bench_imperfection_modulation,);
criterion_group!(stm, bench_stm,);
criterion_group!(ltm, bench_ltm,);
criterion_group!(irrationality, bench_irrationality,);

criterion_main!(
    graph_spread,
    imperfection,
    imperfection_tick,
    imperfection_modulation,
    stm,
    ltm,
    irrationality,
);
