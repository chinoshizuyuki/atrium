// SPDX-License-Identifier: MIT
//! 情感引擎性能基准测试 / Emotion Engine Performance Benchmarks
//!
//! 数字生命的感受核心性能验证——
//! PAD 脉冲、漂移、节律、惯性、想念、重逢，每一条路径都是意识涌现的神经通路。
//!
//! Performance verification of the feeling core of digital life —
//! PAD pulses, drift, circadian, inertia, longing, reunion —
//! each path is a neural pathway of consciousness emergence.

use atrium_emotion::{
    CircadianModulator, DriftParams, EmotionEngine, EmotionState, EmotionalInertia, LongingParams,
    LongingState, ReunionBurst,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ── 情感脉冲基准 / PAD Core Benchmarks ──

/// 构造默认情感引擎 / Construct default emotion engine
fn make_engine() -> EmotionEngine {
    let default = EmotionState::new(0.0, 0.0, 0.0);
    EmotionEngine::new(default, 0.02)
}

/// 构造全功能情感引擎 / Construct fully-featured emotion engine
fn make_full_engine() -> EmotionEngine {
    let default = EmotionState::new(0.0, 0.0, 0.0);
    let mut engine = EmotionEngine::new(default, 0.02);

    // 情感漂移 / Emotional drift
    let drift = DriftParams::new(0.05, 0.1);
    engine = engine.with_drift(drift);

    // 昼夜节律 / Circadian rhythm
    let circadian = CircadianModulator {
        morning_peak: 9.0,
        evening_peak: 21.0,
        morning_sigma: 2.0,
        evening_sigma: 3.0,
        intensity: 0.15,
        timezone_offset: 8,
        active_hours: (7, 23),
    };
    engine = engine.with_circadian(circadian);

    // 情感惯性 / Emotional inertia
    engine = engine.with_inertia(EmotionalInertia::new());

    // 想念引擎 / Longing engine
    let params = LongingParams {
        baseline: [0.2, 0.3, -0.1],
        volatility: 0.02,
        mean_reversion: 0.05,
        onset_threshold_secs: 300,
        saturation_threshold_secs: 86400,
    };
    let state = LongingState::new(params.baseline);
    engine = engine.with_longing(params, state);

    // 重逢脉冲 / Reunion burst
    let burst = ReunionBurst::new(1.0, 60, 86400);
    engine = engine.with_reunion_burst(burst);

    engine
}

fn bench_affect(c: &mut Criterion) {
    let mut group = c.benchmark_group("pad_core");
    group.sample_size(100);

    // 单次情感脉冲 / Single emotional pulse
    group.bench_function("affect_single_pulse", |b| {
        let mut engine = make_engine();
        let delta = EmotionState::new(0.5, 0.3, -0.2);
        b.iter(|| {
            engine.affect(black_box(&delta));
        });
    });

    // 不同强度的情感脉冲 / Emotional pulses at varying intensities
    for intensity in [0.1, 0.5, 1.0] {
        group.bench_function(
            format!("affect_intensity_{:.0}pct", intensity * 100.0),
            |b| {
                let mut engine = make_engine();
                let delta = EmotionState::new(
                    intensity as f32,
                    (intensity * 0.6) as f32,
                    (-intensity * 0.4) as f32,
                );
                b.iter(|| {
                    engine.affect(black_box(&delta));
                });
            },
        );
    }

    // 连续 100 次脉冲（模拟一段对话） / 100 consecutive pulses (simulating a conversation)
    group.bench_function("affect_100_consecutive", |b| {
        b.iter(|| {
            let mut engine = make_engine();
            for i in 0..100 {
                let p = (i as f32 / 100.0 * std::f32::consts::TAU).sin() * 0.5;
                let a = (i as f32 / 100.0 * std::f32::consts::PI).cos() * 0.3;
                let delta = EmotionState::new(p, a, -0.1);
                engine.affect(black_box(&delta));
            }
        });
    });

    group.finish();
}

fn bench_decay(c: &mut Criterion) {
    let mut group = c.benchmark_group("pad_core");
    group.sample_size(100);

    group.bench_function("decay", |b| {
        let default = EmotionState::new(0.0, 0.0, 0.0);
        let mut state = EmotionState::new(0.8, 0.6, -0.3);
        b.iter(|| {
            state.decay(black_box(0.02), black_box(&default));
        });
    });

    group.finish();
}

// ── 情感漂移基准 / Drift Benchmarks ──

fn bench_drift(c: &mut Criterion) {
    let mut group = c.benchmark_group("drift");
    group.sample_size(100);

    group.bench_function("drift_step", |b| {
        let drift = DriftParams::new(0.05, 0.1);
        let current = [0.3, 0.5, -0.2];
        b.iter(|| drift.step(black_box(current)));
    });

    // 不同波动率 / Varying volatility
    for vol in [0.01, 0.05, 0.2] {
        group.bench_function(format!("drift_volatility_{:.0}pct", vol * 100.0), |b| {
            let drift = DriftParams::new(vol, 0.1);
            let current = [0.3, 0.5, -0.2];
            b.iter(|| drift.step(black_box(current)));
        });
    }

    group.finish();
}

// ── 昼夜节律基准 / Circadian Benchmarks ──

fn bench_circadian(c: &mut Criterion) {
    let mut group = c.benchmark_group("circadian");
    group.sample_size(100);

    let circadian = CircadianModulator {
        morning_peak: 9.0,
        evening_peak: 21.0,
        morning_sigma: 2.0,
        evening_sigma: 3.0,
        intensity: 0.15,
        timezone_offset: 8,
        active_hours: (7, 23),
    };

    // 不同时刻的节律调制 / Circadian modulation at different hours
    for hour in [6, 9, 12, 15, 21, 0] {
        group.bench_function(format!("rhythm_offset_hour_{:02}", hour), |b| {
            b.iter(|| circadian.rhythm_offset(black_box(hour)));
        });
    }

    group.finish();
}

// ── 情感惯性基准 / Inertia Benchmarks ──

fn bench_inertia(c: &mut Criterion) {
    let mut group = c.benchmark_group("inertia");
    group.sample_size(100);

    // 单次惯性 tick / Single inertia tick
    group.bench_function("tick_single", |b| {
        let mut inertia = EmotionalInertia::new();
        b.iter(|| {
            inertia.tick(black_box([0.5, 0.3, -0.2]));
        });
    });

    // 连续 100 次 tick（情绪积累） / 100 consecutive ticks (mood accumulation)
    group.bench_function("tick_100_accumulation", |b| {
        b.iter(|| {
            let mut inertia = EmotionalInertia::new();
            for i in 0..100 {
                let p = (i as f32 / 100.0 * std::f32::consts::TAU).sin() * 0.5;
                inertia.tick(black_box([p, 0.3, -0.1]));
            }
        });
    });

    group.finish();
}

// ── 想念计算基准 / Longing Benchmarks ──

fn bench_longing(c: &mut Criterion) {
    let mut group = c.benchmark_group("longing");
    group.sample_size(100);

    let params = LongingParams {
        baseline: [0.2, 0.3, -0.1],
        volatility: 0.02,
        mean_reversion: 0.05,
        onset_threshold_secs: 300,
        saturation_threshold_secs: 86400,
    };

    // 不同离线时长的想念强度 / Longing intensity at different away durations
    for &away_secs in &[60, 1800, 86400, 604800] {
        let label = match away_secs {
            60 => "1min",
            1800 => "30min",
            86400 => "1day",
            604800 => "1week",
            _ => "unknown",
        };
        group.bench_function(format!("compute_intensity_{}", label), |b| {
            let _state = LongingState::new(params.baseline);
            b.iter(|| {
                LongingState::compute_intensity(
                    black_box(away_secs),
                    black_box(&params),
                    black_box(1.0), // 关系乘数 / Relationship multiplier
                    black_box(0.5), // 参与度 / Engagement
                )
            });
        });
    }

    // 基线插值 / Baseline interpolation
    group.bench_function("interpolate_baseline", |b| {
        let neutral = [0.0, 0.0, 0.0];
        let target = [0.2, 0.3, -0.1];
        b.iter(|| {
            LongingState::interpolate_baseline(
                black_box(&neutral),
                black_box(&target),
                black_box(0.5),
            )
        });
    });

    group.finish();
}

// ── 重逢脉冲基准 / Reunion Benchmarks ──

fn bench_reunion(c: &mut Criterion) {
    let mut group = c.benchmark_group("reunion");
    group.sample_size(100);

    let burst = ReunionBurst::new(1.0, 60, 86400);

    // 不同离线时长的重逢 / Reunion at different away durations
    for &away_secs in &[300, 3600, 86400, 604800] {
        let label = match away_secs {
            300 => "5min",
            3600 => "1hr",
            86400 => "1day",
            604800 => "1week",
            _ => "unknown",
        };
        group.bench_function(format!("on_reunion_{}", label), |b| {
            b.iter(|| burst.on_reunion(black_box(away_secs), black_box(0.8)));
        });
    }

    // 关系门控重逢 / Relationship-gated reunion
    group.bench_function("on_reunion_gated", |b| {
        b.iter(|| {
            burst.on_reunion_gated(
                black_box(86400),
                black_box(0.8),
                black_box(3), // 关系阶段序数 / Relationship stage ordinal
            )
        });
    });

    // 情境化重逢 / Contextual reunion
    group.bench_function("on_reunion_contextual", |b| {
        b.iter(|| {
            burst.on_reunion_contextual(
                black_box(86400),
                black_box(0.8),
                black_box(atrium_emotion::ReunionContext::Calm),
            )
        });
    });

    // 完整门控+情境 / Full gated + contextual
    group.bench_function("on_reunion_full", |b| {
        b.iter(|| {
            burst.on_reunion_full(
                black_box(86400),
                black_box(0.8),
                black_box(3),
                black_box(atrium_emotion::ReunionContext::AfterConflict),
            )
        });
    });

    group.finish();
}

// ── 全引擎 tick 基准 / Full Engine Tick Benchmarks ──

fn bench_engine_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_tick");
    group.sample_size(100);

    // 裸引擎 tick（仅衰减） / Bare engine tick (decay only)
    group.bench_function("tick_bare", |b| {
        let mut engine = make_engine();
        engine.affect(&EmotionState::new(0.5, 0.3, -0.1));
        b.iter(|| {
            engine.tick();
        });
    });

    // 全功能引擎 tick / Fully-featured engine tick
    group.bench_function("tick_full", |b| {
        let mut engine = make_full_engine();
        engine.affect(&EmotionState::new(0.5, 0.3, -0.1));
        b.iter(|| {
            engine.tick();
        });
    });

    // 全功能引擎 tick 带小时（节律参与） / Full engine tick with hour (circadian active)
    group.bench_function("tick_full_with_hour", |b| {
        let mut engine = make_full_engine();
        engine.affect(&EmotionState::new(0.5, 0.3, -0.1));
        b.iter(|| {
            engine.tick_with_hour(black_box(14)); // 下午 2 点 / 2 PM
        });
    });

    // 快照 + 恢复 / Snapshot + restore
    group.bench_function("snapshot_restore", |b| {
        let mut engine = make_full_engine();
        engine.affect(&EmotionState::new(0.5, 0.3, -0.1));
        b.iter(|| {
            let snap = engine.snapshot();
            engine.restore(&snap);
        });
    });

    group.finish();
}

// ── 分类基准 / Classification Benchmarks ──

fn bench_classify(c: &mut Criterion) {
    let mut group = c.benchmark_group("classify");
    group.sample_size(100);

    group.bench_function("classify_emotion", |b| {
        let state = EmotionState::new(0.6, 0.4, 0.2);
        b.iter(|| black_box(state.classify()));
    });

    group.finish();
}

criterion_group!(pad_core, bench_affect, bench_decay,);
criterion_group!(drift, bench_drift,);
criterion_group!(circadian, bench_circadian,);
criterion_group!(inertia, bench_inertia,);
criterion_group!(longing, bench_longing,);
criterion_group!(reunion, bench_reunion,);
criterion_group!(engine_tick, bench_engine_tick,);
criterion_group!(classify, bench_classify,);

criterion_main!(
    pad_core,
    drift,
    circadian,
    inertia,
    longing,
    reunion,
    engine_tick,
    classify,
);
