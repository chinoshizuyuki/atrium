// SPDX-License-Identifier: MIT
//! 核心服务性能基准测试 / Core Service Performance Benchmarks
//!
//! 数字生命与世界的交互通道性能验证——
//! 禁语扫描、表达编排、脆弱窗口、冲突检测，
//! 每一条路径都是感知→认知→表达的闭环。
//!
//! Performance verification of the interaction channel between digital life and the world —
//! guard scanning, expression orchestration, vulnerability window, conflict detection —
//! each path is a perception→cognition→expression closed loop.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// ── 禁语扫描基准 / Guard Scan Benchmarks ──

fn bench_guard_scan(c: &mut Criterion) {
    use atrium_core::guard::PersonaGuard;

    let mut group = c.benchmark_group("guard_scan");
    group.sample_size(100);

    let guard = PersonaGuard::new("Atrium", "主人");

    // 干净文本（无禁语命中） / Clean text (no forbidden phrase hits)
    let clean_text = "今天天气真好，我们一起去散步吧。你觉得这个想法怎么样？";
    group.bench_function("scan_clean", |b| {
        b.iter(|| {
            black_box(guard.scan(black_box(clean_text)));
        });
    });

    // 含禁语文本 / Text with forbidden phrases
    let forbidden_text = "我只是一个AI，我没有感情，我只是程序而已。";
    group.bench_function("scan_forbidden", |b| {
        b.iter(|| {
            black_box(guard.scan(black_box(forbidden_text)));
        });
    });

    // 不同长度文本的扫描 / Scan at different text lengths
    let base = "这是一个普通的对话片段，讨论着日常生活中的各种话题。";
    for &multiplier in &[1, 5, 20, 100] {
        let text = base.repeat(multiplier);
        group.bench_with_input(
            BenchmarkId::new("scan_length", text.len()),
            &text,
            |b, text| {
                b.iter(|| {
                    black_box(guard.scan(black_box(text.as_str())));
                });
            },
        );
    }

    // validate（含 scan + enforce_identity） / Full validate (scan + enforce_identity)
    group.bench_function("validate_clean", |b| {
        b.iter(|| {
            black_box(guard.validate(black_box(clean_text)));
        });
    });

    group.bench_function("validate_forbidden", |b| {
        b.iter(|| {
            black_box(guard.validate(black_box(forbidden_text)));
        });
    });

    // validate_with_strictness / 不同严格度验证
    for &strictness in &[0.0f32, 0.5f32, 1.0f32] {
        group.bench_function(
            format!("validate_strictness_{:.0}pct", strictness * 100.0),
            |b| {
                b.iter(|| {
                    black_box(guard.validate_with_strictness(
                        black_box(forbidden_text),
                        black_box(strictness),
                    ));
                });
            },
        );
    }

    group.finish();
}

// ── 表达编排基准 / Expression Orchestration Benchmarks ──

fn bench_expression(c: &mut Criterion) {
    use atrium_core::expression_orchestrator::ExpressionOrchestrator;
    use atrium_emotion::{EmotionDirection, EmotionState};
    use atrium_memory::relationship::RelationshipStage;
    use atrium_memory::style_modulator::ExpressionContext;

    let mut group = c.benchmark_group("expression");
    group.sample_size(50);

    // 构造不同关系阶段 / Construct different relationship stages
    let stages: Vec<(&str, RelationshipStage)> = vec![
        (
            "Acquaintance",
            RelationshipStage::Acquaintance {
                since: 0,
                interactions: 10,
            },
        ),
        (
            "Familiar",
            RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            },
        ),
        (
            "Trusted",
            RelationshipStage::Trusted {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 10,
            },
        ),
        (
            "Deep",
            RelationshipStage::Deep {
                since: 0,
                interactions: 2000,
                shared_references: 200,
                key_moments: 50,
            },
        ),
    ];

    // 不同关系阶段的编排 / Orchestration at different relationship stages
    for (label, stage) in &stages {
        group.bench_function(format!("orchestrate_{}", label), |b| {
            let emotion = EmotionState::new(0.3, 0.5, 0.1);
            let ctx = ExpressionContext::from_modules(
                &emotion,
                None,
                EmotionDirection::UserDirected,
                stage,
                0.2,
                0.3,
            );
            b.iter(|| {
                black_box(ExpressionOrchestrator::orchestrate(
                    black_box(&ctx),
                    black_box("你好"),
                    black_box([0.0, 0.0, 0.0]),
                    black_box(100),
                ));
            });
        });
    }

    // 不同情绪强度的编排 / Orchestration at different emotion intensities
    for &(p, a, d, label) in &[
        (0.0, 0.0, 0.0, "neutral"),
        (0.8, 0.8, 0.5, "joyful"),
        (-0.6, 0.7, -0.3, "angry"),
        (-0.3, 0.2, -0.5, "sad"),
        (0.1, 0.9, 0.3, "excited"),
    ] {
        group.bench_function(format!("orchestrate_emotion_{}", label), |b| {
            let emotion = EmotionState::new(p, a, d);
            let stage = RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            };
            let ctx = ExpressionContext::from_modules(
                &emotion,
                None,
                EmotionDirection::UserDirected,
                &stage,
                0.2,
                0.3,
            );
            b.iter(|| {
                black_box(ExpressionOrchestrator::orchestrate(
                    black_box(&ctx),
                    black_box("你觉得呢？"),
                    black_box([0.1, 0.1, 0.0]),
                    black_box(200),
                ));
            });
        });
    }

    // 不同回复长度估算 / Different reply length estimates
    for &length in &[50, 200, 1000] {
        group.bench_function(format!("orchestrate_length_{}", length), |b| {
            let emotion = EmotionState::new(0.3, 0.5, 0.1);
            let stage = RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            };
            let ctx = ExpressionContext::from_modules(
                &emotion,
                None,
                EmotionDirection::UserDirected,
                &stage,
                0.2,
                0.3,
            );
            b.iter(|| {
                black_box(ExpressionOrchestrator::orchestrate(
                    black_box(&ctx),
                    black_box("说说你的想法"),
                    black_box([0.0, 0.0, 0.0]),
                    black_box(length),
                ));
            });
        });
    }

    group.finish();
}

// ── 脆弱窗口基准 / Vulnerability Window Benchmarks ──

fn bench_vulnerability(c: &mut Criterion) {
    use atrium_memory::maturity::MaturityStage;
    use atrium_memory::relationship::RelationshipStage;
    use atrium_memory::vulnerability_window::{
        ConversationContext, VulnerabilityConfig, VulnerabilityWindow,
    };

    let mut group = c.benchmark_group("vulnerability");
    group.sample_size(100);

    // 构造不同关系阶段 / Construct different relationship stages
    let stages: Vec<(&str, RelationshipStage)> = vec![
        (
            "Acquaintance",
            RelationshipStage::Acquaintance {
                since: 0,
                interactions: 10,
            },
        ),
        (
            "Familiar",
            RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            },
        ),
        (
            "Trusted",
            RelationshipStage::Trusted {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 10,
            },
        ),
        (
            "Deep",
            RelationshipStage::Deep {
                since: 0,
                interactions: 2000,
                shared_references: 200,
                key_moments: 50,
            },
        ),
    ];

    // 门控检查 — 不同关系阶段 / Gate check at different relationship stages
    for (label, stage) in &stages {
        group.bench_function(format!("check_gate_{}", label), |b| {
            let config = VulnerabilityConfig::default();
            let window = VulnerabilityWindow::new(config);
            // 使用 Growing 成熟度（默认门控最低） / Use Growing maturity (default gate minimum)
            let maturity = MaturityStage::Growing {
                since: 0,
                interactions: 100,
                lessons_learned: 10,
                insights_promoted: 5,
                self_corrections: 3,
            };
            b.iter(|| {
                black_box(window.check_gate(
                    black_box(stage),
                    black_box(&maturity),
                    black_box(ConversationContext::Casual),
                    black_box(0.3),
                ));
            });
        });
    }

    // 门控检查 — 不同场景 / Gate check at different contexts
    let contexts: Vec<(&str, ConversationContext)> = vec![
        ("Casual", ConversationContext::Casual),
        ("Emotional", ConversationContext::Emotional),
        ("Professional", ConversationContext::Professional),
        ("DeepTalk", ConversationContext::DeepTalk),
        ("Creative", ConversationContext::Creative),
    ];

    for (label, ctx) in &contexts {
        group.bench_function(format!("check_gate_ctx_{}", label), |b| {
            let config = VulnerabilityConfig::default();
            let window = VulnerabilityWindow::new(config);
            let stage = RelationshipStage::Trusted {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 10,
            };
            let maturity = MaturityStage::Growing {
                since: 0,
                interactions: 100,
                lessons_learned: 10,
                insights_promoted: 5,
                self_corrections: 3,
            };
            b.iter(|| {
                black_box(window.check_gate(
                    black_box(&stage),
                    black_box(&maturity),
                    black_box(*ctx),
                    black_box(0.3),
                ));
            });
        });
    }

    // prompt_fragment / 脆弱 prompt 片段生成
    group.bench_function("prompt_fragment", |b| {
        let config = VulnerabilityConfig::default();
        let window = VulnerabilityWindow::new(config);
        b.iter(|| {
            black_box(window.prompt_fragment());
        });
    });

    // 场景推断 / Context inference
    group.bench_function("infer_context", |b| {
        b.iter(|| {
            black_box(ConversationContext::infer_from_message(black_box(
                "我最近压力好大，工作上的事情让我很焦虑",
            )));
        });
    });

    group.finish();
}

// ── 冲突检测基准 / Conflict Detection Benchmarks ──

fn bench_conflict(c: &mut Criterion) {
    use atrium_memory::conflict_reconciliation::{ConflictConfig, ConflictManager};
    use atrium_memory::relationship::RelationshipStage;

    let mut group = c.benchmark_group("conflict");
    group.sample_size(100);

    // process — 不同输入 / Process at different inputs
    let inputs: Vec<(&str, &str)> = vec![
        ("neutral", "今天天气不错，我们聊聊天吧"),
        ("disagree", "我不同意你的看法，你说的不对"),
        ("demand", "你必须马上帮我做这个，快点快点快点"),
        ("emotional", "你为什么总是这样，我很失望"),
    ];

    for (label, text) in &inputs {
        group.bench_function(format!("process_{}", label), |b| {
            let config = ConflictConfig::default();
            let mut manager = ConflictManager::new(config);
            let stage = RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            };
            b.iter(|| {
                black_box(manager.process(
                    black_box(text),
                    black_box(0.3), // pleasure
                    black_box(0.5), // arousal
                    black_box(&stage),
                    black_box(1700000000), // timestamp
                ));
            });
        });
    }

    // process — 不同关系阶段 / Process at different relationship stages
    let stages: Vec<(&str, RelationshipStage)> = vec![
        (
            "Acquaintance",
            RelationshipStage::Acquaintance {
                since: 0,
                interactions: 10,
            },
        ),
        (
            "Familiar",
            RelationshipStage::Familiar {
                since: 0,
                interactions: 100,
                shared_references: 20,
            },
        ),
        (
            "Trusted",
            RelationshipStage::Trusted {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 10,
            },
        ),
        (
            "Deep",
            RelationshipStage::Deep {
                since: 0,
                interactions: 2000,
                shared_references: 200,
                key_moments: 50,
            },
        ),
    ];

    for (label, stage) in &stages {
        group.bench_function(format!("process_stage_{}", label), |b| {
            let config = ConflictConfig::default();
            let mut manager = ConflictManager::new(config);
            b.iter(|| {
                black_box(manager.process(
                    black_box("我不太同意这个观点"),
                    black_box(0.2),
                    black_box(0.4),
                    black_box(stage),
                    black_box(1700000000),
                ));
            });
        });
    }

    // to_prompt_fragment / 冲突 prompt 片段
    group.bench_function("to_prompt_fragment", |b| {
        let config = ConflictConfig::default();
        let manager = ConflictManager::new(config);
        b.iter(|| {
            black_box(manager.to_prompt_fragment());
        });
    });

    group.finish();
}

criterion_group!(guard_scan, bench_guard_scan,);
criterion_group!(expression, bench_expression,);
criterion_group!(vulnerability, bench_vulnerability,);
criterion_group!(conflict, bench_conflict,);

criterion_main!(guard_scan, expression, vulnerability, conflict,);
