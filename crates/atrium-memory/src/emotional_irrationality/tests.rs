// ! 情绪非理性系统 — 单元测试 / Emotional Irrationality System — Unit Tests

use crate::emotional_irrationality::chaos_engine::*;
use crate::emotional_irrationality::contagion_engine::*;
use crate::emotional_irrationality::pulse_engine::*;
use crate::emotional_irrationality::residue_engine::*;
use crate::emotional_irrationality::shock_absorber::*;
use crate::emotional_irrationality::types::*;
use crate::emotional_irrationality::{IrrationalityConfig, IrrationalityManager, RandomMode};
use rand::rngs::SmallRng;
use rand::SeedableRng;

// ── DecayCurve 测试 ──

#[test]
fn test_decay_exponential() {
    let curve = DecayCurve::Exponential { lambda: 0.1 };
    assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
    assert!(curve.evaluate(10.0) < 1.0);
    assert!(curve.evaluate(10.0) > 0.0);
    assert!(curve.evaluate(100.0) < 0.01);
}

#[test]
fn test_decay_power_law() {
    let curve = DecayCurve::PowerLaw {
        tau: 60.0,
        alpha: 0.5,
    };
    assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
    // 幂律衰减比指数慢
    let exp = DecayCurve::Exponential { lambda: 0.1 };
    assert!(curve.evaluate(100.0) > exp.evaluate(100.0));
}

#[test]
fn test_decay_damped_oscillation() {
    let curve = DecayCurve::DampedOscillation {
        zeta: 0.05,
        omega: 0.3,
    };
    assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
    // 振荡衰减可能出现零点
    assert!(curve.evaluate(100.0) >= 0.0);
}

#[test]
fn test_decay_staged() {
    let curve = DecayCurve::Staged {
        stages: [
            DecayStage {
                duration_secs: 10.0,
                decay_rate: 0.5,
            },
            DecayStage {
                duration_secs: 30.0,
                decay_rate: 0.1,
            },
            DecayStage {
                duration_secs: 60.0,
                decay_rate: 0.01,
            },
        ],
    };
    assert!((curve.evaluate(0.0) - 1.0).abs() < 1e-6);
    assert!(curve.evaluate(5.0) < 1.0);
    assert!(curve.evaluate(5.0) > curve.evaluate(20.0));
}

// ── ShockAbsorber 测试 ──

#[test]
fn test_shock_absorber_full() {
    let mut sa = ShockAbsorber::new(2.0, 0.1);
    let mut pulse = ChaoticPulse {
        id: 1,
        kind: PulseKind::Startle,
        intensity: 1.0,
        pad_impulse: [-0.3, 0.3, -0.1],
        duration_secs: 300.0,
        decay_curve: DecayCurve::default_exponential(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: false,
        residual_intensity: 1.0,
    };
    let result = sa.absorb(&mut pulse, 1000);
    assert_eq!(result, AbsorbResult::FullyAbsorbed);
    assert!(pulse.absorbed);
}

#[test]
fn test_shock_absorber_overload() {
    let mut sa = ShockAbsorber::new(2.0, 0.1);
    // 消耗全部容量
    let mut p1 = ChaoticPulse {
        id: 1,
        kind: PulseKind::Startle,
        intensity: 2.0,
        pad_impulse: [-0.3, 0.3, -0.1],
        duration_secs: 300.0,
        decay_curve: DecayCurve::default_exponential(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: false,
        residual_intensity: 2.0,
    };
    let _ = sa.absorb(&mut p1, 1000);
    // 第二个脉冲应被过载保护（同一时间戳，无恢复）
    let mut p2 = ChaoticPulse {
        id: 2,
        kind: PulseKind::JoyBurst,
        intensity: 0.5,
        pad_impulse: [0.3, 0.3, 0.1],
        duration_secs: 300.0,
        decay_curve: DecayCurve::default_exponential(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "test".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: false,
        residual_intensity: 0.5,
    };
    let result = sa.absorb(&mut p2, 1000);
    assert_eq!(result, AbsorbResult::OverloadProtection);
}

// ── PulseEngine 测试 ──

#[test]
fn test_pulse_detect_startle() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "bad_news".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    let result = engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
    assert!(result.is_some());
    let pulse = result.unwrap();
    assert_eq!(pulse.kind, PulseKind::Startle);
}

#[test]
fn test_pulse_detect_joy_burst() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "good_news".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    let result = engine.detect(&[0.0, 0.0, 0.0], &[0.5, 0.5, 0.1], trigger, 1000);
    assert!(result.is_some());
    assert_eq!(result.unwrap().kind, PulseKind::JoyBurst);
}

#[test]
fn test_pulse_detect_sadness_surge() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "loss".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    let result = engine.detect(&[0.0, 0.0, 0.0], &[-0.5, -0.3, -0.1], trigger, 1000);
    assert!(result.is_some());
    assert_eq!(result.unwrap().kind, PulseKind::SadnessSurge);
}

#[test]
fn test_pulse_no_detect_small_change() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "minor".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    let result = engine.detect(&[0.0, 0.0, 0.0], &[0.1, 0.1, 0.0], trigger, 1000);
    assert!(result.is_none());
}

#[test]
fn test_pulse_combined_effect() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "test".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
    let effect = engine.combined_effect(1000);
    // 刚触发时效果应非零
    assert!(effect[0].abs() > 0.01 || effect[1].abs() > 0.01);
}

#[test]
fn test_pulse_tick_decay() {
    let mut engine = PulseEngine::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "test".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    engine.detect(&[0.0, 0.0, 0.0], &[-0.5, 0.5, -0.3], trigger, 1000);
    assert!(!engine.active_pulses.is_empty());
    // 大量时间后脉冲应衰减消失
    engine.tick(100000);
    assert!(engine.active_pulses.is_empty());
}

// ── ResidueEngine 测试 ──

#[test]
fn test_residue_from_pulse() {
    let mut engine = ResidueEngine::default();
    let pulse = ChaoticPulse {
        id: 1,
        kind: PulseKind::SadnessSurge,
        intensity: 0.8,
        pad_impulse: [-0.5, -0.3, -0.1],
        duration_secs: 300.0,
        decay_curve: DecayCurve::slow_power_law(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "loss".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: true,
        residual_intensity: 0.8,
    };
    let result = engine.from_pulse(&pulse);
    assert!(result.is_some());
    let residue = result.unwrap();
    assert_eq!(residue.kind, ResidueKind::LingeringSadness);
    assert!(residue.intensity > 0.0);
}

#[test]
fn test_residue_no_residue_for_uncaused() {
    let mut engine = ResidueEngine::default();
    let pulse = ChaoticPulse {
        id: 1,
        kind: PulseKind::UncausedFluctuation,
        intensity: 0.03,
        pad_impulse: [0.01, -0.01, 0.0],
        duration_secs: 60.0,
        decay_curve: DecayCurve::default_exponential(),
        trigger: PulseTrigger {
            source: PulseSource::Spontaneous,
            signal: "noise".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: true,
        residual_intensity: 0.03,
    };
    let result = engine.from_pulse(&pulse);
    assert!(result.is_none());
}

#[test]
fn test_residue_combined_effect() {
    let mut engine = ResidueEngine::default();
    let pulse = ChaoticPulse {
        id: 1,
        kind: PulseKind::JoyBurst,
        intensity: 0.9,
        pad_impulse: [0.5, 0.5, 0.1],
        duration_secs: 300.0,
        decay_curve: DecayCurve::default_exponential(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "good".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 1000,
        absorbed: true,
        residual_intensity: 0.9,
    };
    engine.from_pulse(&pulse);
    let effect = engine.combined_effect(1000);
    assert!(effect.active_count > 0);
    assert_eq!(effect.dominant_residue, Some(ResidueKind::Afterglow));
    // Afterglow PAD偏移：P>0
    assert!(effect.pad_offset[0] > 0.0);
}

#[test]
fn test_residue_tick_decay() {
    let mut engine = ResidueEngine::default();
    let pulse = ChaoticPulse {
        id: 1,
        kind: PulseKind::AngerFlash,
        intensity: 0.7,
        pad_impulse: [-0.4, 0.4, 0.2],
        duration_secs: 300.0,
        decay_curve: DecayCurve::slow_power_law(),
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "provoked".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: 0,
        absorbed: true,
        residual_intensity: 0.7,
    };
    engine.from_pulse(&pulse);
    assert!(!engine.active_residues.is_empty());
    // 大量时间后残留应衰减消失
    engine.tick(10000000);
    assert!(engine.active_residues.is_empty());
}

// ── ContagionEngine 测试 ──

#[test]
fn test_contagion_default_rules() {
    let engine = ContagionEngine::default();
    assert_eq!(engine.rules.len(), 12);
}

#[test]
fn test_contagion_evaluate_with_anger() {
    let mut engine = ContagionEngine::default();
    let profile = EmotionProfile {
        anger: 0.8,
        sadness: 0.0,
        anxiety: 0.0,
        fear: 0.0,
        joy: 0.0,
        calm: 0.0,
        guilt: 0.0,
        shame: 0.0,
        pride: 0.0,
        envy: 0.0,
        gratitude: 0.0,
        nostalgia: 0.0,
    };
    // 注入确定性 RNG，确保可复现 / Inject deterministic RNG for reproducibility
    let mut rng = SmallRng::seed_from_u64(42);
    let result = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::Any,
        1000,
        &mut rng,
    );
    // 概率性，不保证一定触发，但规则检查应通过
    // 主要验证不 panic
    assert!(result.len() <= engine.rules.len());
}

// ── ChaosEngine 测试 ──

#[test]
fn test_chaos_attractor_calm() {
    let mut engine = ChaosEngine::default();
    // 填充平静区域轨迹
    for i in 0..20 {
        engine.record(&[0.1, -0.05, 0.0], 1000 + i * 60);
    }
    let attractor = engine.detect_attractor();
    assert_eq!(attractor, StrangeAttractor::CalmBasin);
}

#[test]
fn test_chaos_attractor_anxiety() {
    let mut engine = ChaosEngine::default();
    for i in 0..20 {
        engine.record(&[-0.2, 0.3, -0.2], 1000 + i * 60);
    }
    let attractor = engine.detect_attractor();
    assert_eq!(attractor, StrangeAttractor::AnxietyBasin);
}

#[test]
fn test_chaos_bifurcation() {
    let mut engine = ChaosEngine::default();
    // 前半段平静，后半段焦虑 → 分岔
    for i in 0..10 {
        engine.record(&[0.2, -0.1, 0.0], 1000 + i * 60);
    }
    for i in 10..20 {
        engine.record(&[-0.3, 0.4, -0.2], 1000 + i * 60);
    }
    let pattern = engine.detect_bifurcation(2200);
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().kind, EmergentKind::Bifurcation);
}

// ── EmotionProfile 测试 ──

#[test]
fn test_emotion_profile_from_pad_anger() {
    let profile = EmotionProfile::from_pad(&[-0.5, 0.7, 0.3]);
    assert!(profile.anger > 0.1);
    assert!(profile.sadness < profile.anger);
}

#[test]
fn test_emotion_profile_from_pad_joy() {
    let profile = EmotionProfile::from_pad(&[0.6, 0.5, 0.2]);
    assert!(profile.joy > 0.1);
    assert!(profile.anger < 0.01);
}

#[test]
fn test_emotion_profile_from_pad_calm() {
    let profile = EmotionProfile::from_pad(&[0.5, -0.3, 0.1]);
    assert!(profile.calm > 0.1);
}

// ── BodyMemory 测试 ──

#[test]
fn test_body_memory_neutral() {
    let bm = BodyMemory::neutral();
    assert!((bm.tension - 0.0).abs() < 1e-6);
    assert!((bm.warmth - 0.0).abs() < 1e-6);
}

#[test]
fn test_body_memory_from_residue() {
    let bm = BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 1.0);
    assert!(bm.tension > 0.5);
    assert!(bm.warmth < 0.0);
}

#[test]
fn test_body_memory_combine() {
    let bm1 = BodyMemory {
        breath_offset: 0.1,
        tension: 0.2,
        heaviness: 0.1,
        warmth: 0.0,
    };
    let bm2 = BodyMemory {
        breath_offset: 0.2,
        tension: 0.3,
        heaviness: 0.0,
        warmth: 0.5,
    };
    let combined = bm1.combine(&bm2, 0.5);
    assert!((combined.tension - 0.35).abs() < 1e-6);
}

// ── ResidueKind 测试 ──

#[test]
fn test_residue_half_lives() {
    assert_eq!(ResidueKind::Tension.default_half_life_secs(), 1800.0);
    assert_eq!(
        ResidueKind::LingeringSadness.default_half_life_secs(),
        7200.0
    );
    assert_eq!(
        ResidueKind::IntimacyDeepening.default_half_life_secs(),
        f64::MAX
    );
}

#[test]
fn test_residue_pad_offsets() {
    let sad = ResidueKind::LingeringSadness.default_pad_offset();
    assert!(sad[0] < 0.0); // P < 0
    let warm = ResidueKind::WarmthResidue.default_pad_offset();
    assert!(warm[0] > 0.0); // P > 0
}

// ── IrrationalityManager 集成测试 ──

#[test]
fn test_manager_on_emotion_change() {
    let mut mgr = IrrationalityManager::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "shocking_news".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    let correction = mgr.on_emotion_change(
        &[0.0, 0.0, 0.0],
        &[-0.5, 0.5, -0.3],
        trigger,
        RelationshipDepth::FamiliarOrAbove,
        MaturityDepth::Any,
        1000,
    );
    assert!(correction.active_pulses > 0 || correction.active_residues > 0);
}

#[test]
fn test_manager_prompt_fragment() {
    let mut mgr = IrrationalityManager::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "test".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    mgr.on_emotion_change(
        &[0.0, 0.0, 0.0],
        &[-0.5, 0.5, -0.3],
        trigger,
        RelationshipDepth::Any,
        MaturityDepth::Any,
        1000,
    );
    let fragment = mgr.to_prompt_fragment(1000);
    assert!(!fragment.is_empty());
    assert!(fragment.contains("[情绪生态]"));
}

#[test]
fn test_manager_tick() {
    let mut mgr = IrrationalityManager::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "test".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    mgr.on_emotion_change(
        &[0.0, 0.0, 0.0],
        &[-0.5, 0.5, -0.3],
        trigger,
        RelationshipDepth::Any,
        MaturityDepth::Any,
        1000,
    );
    // Tick 应不 panic
    mgr.tick(&[0.0, 0.0, 0.0], 1060);
}

#[test]
fn test_manager_body_memory() {
    let mut mgr = IrrationalityManager::default();
    let trigger = PulseTrigger {
        source: PulseSource::UserMessage,
        signal: "test".to_string(),
        baseline_pad: [0.0, 0.0, 0.0],
    };
    mgr.on_emotion_change(
        &[0.0, 0.0, 0.0],
        &[-0.5, 0.5, -0.3],
        trigger,
        RelationshipDepth::Any,
        MaturityDepth::Any,
        1000,
    );
    let bm = mgr.body_memory_for_expression(1000);
    // 应返回有效的身体记忆
    assert!(bm.tension.is_finite());
    assert!(bm.warmth.is_finite());
}

// ═══════════════════════════════════════════════════════════
// ═════════════════════════════════════════════════════════════
// Phase E: 15 enhancement tests
// ═════════════════════════════════════════════════════════════

/// Helper: push a residue directly into the engine for testing
fn add_test_residue(engine: &mut ResidueEngine, kind: ResidueKind, intensity: f64, ts: i64) {
    let id = engine.next_id;
    engine.next_id += 1;
    engine.active_residues.push(EmotionResidue {
        id,
        kind,
        intensity,
        pad_offset: [0.0, 0.0, 0.0],
        half_life_secs: 600.0,
        created_at: ts,
        updated_at: ts,
        source_pulse_id: None,
        body_memory: BodyMemory::neutral(),
        expressed: false,
    });
}

#[test]
fn test_body_memory_decay() {
    let mut bm = BodyMemory {
        breath_offset: 0.8,
        tension: 0.6,
        heaviness: 0.4,
        warmth: 0.2,
    };
    bm.decay(0.5);
    assert!((bm.breath_offset - 0.4).abs() < 1e-9);
    assert!((bm.tension - 0.3).abs() < 1e-9);
    assert!((bm.heaviness - 0.2).abs() < 1e-9);
    assert!((bm.warmth - 0.1).abs() < 1e-9);
}

#[test]
fn test_body_memory_normalize() {
    let mut bm = BodyMemory {
        breath_offset: 1.5,
        tension: -2.0,
        heaviness: 0.5,
        warmth: -0.3,
    };
    bm.normalize();
    assert!((bm.breath_offset - 1.0).abs() < 1e-9);
    assert!((bm.tension - (-1.0)).abs() < 1e-9);
    assert!((bm.heaviness - 0.5).abs() < 1e-9);
    assert!((bm.warmth - (-0.3)).abs() < 1e-9);
}

#[test]
fn test_body_memory_dominant_channel() {
    let bm = BodyMemory {
        breath_offset: 0.1,
        tension: 0.9,
        heaviness: 0.3,
        warmth: 0.2,
    };
    assert_eq!(bm.dominant_channel(), "tension");

    let bm2 = BodyMemory {
        breath_offset: 0.8,
        tension: 0.1,
        heaviness: 0.2,
        warmth: 0.3,
    };
    assert_eq!(bm2.dominant_channel(), "breath");
}

#[test]
fn test_body_memory_magnitude() {
    let bm = BodyMemory {
        breath_offset: 1.0,
        tension: 1.0,
        heaviness: 0.0,
        warmth: 0.0,
    };
    assert!((bm.magnitude() - 2.0_f64.sqrt()).abs() < 1e-9);

    let neutral = BodyMemory::neutral();
    assert!(neutral.magnitude().abs() < 1e-9);
}

#[test]
fn test_body_memory_to_prompt_hint() {
    let bm_tense = BodyMemory {
        breath_offset: 0.0,
        tension: 0.5,
        heaviness: 0.0,
        warmth: 0.0,
    };
    let hint = bm_tense.to_prompt_hint();
    assert!(hint.contains("紧张"), "hint={}", hint);

    let bm_calm = BodyMemory::neutral();
    let hint_calm = bm_calm.to_prompt_hint();
    assert!(hint_calm.contains("平静"), "hint_calm={}", hint_calm);
}

#[test]
fn test_residue_merge_same_kind() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.4, 100);
    add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 200);
    assert_eq!(engine.active_residues.len(), 2);
    engine.merge_same_kind();
    assert_eq!(engine.active_residues.len(), 1);
    let merged = &engine.active_residues[0];
    assert!(
        (merged.intensity - 0.49).abs() < 1e-9,
        "merged intensity={}",
        merged.intensity
    );
    assert_eq!(merged.kind, ResidueKind::Tension);
}

#[test]
fn test_residue_mark_expressed() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 100);
    let residue_id = engine.active_residues[0].id;
    assert!(!engine.active_residues[0].expressed);
    let found = engine.mark_expressed(residue_id);
    assert!(found);
    assert!(engine.active_residues[0].expressed);
    let not_found = engine.mark_expressed(99999);
    assert!(!not_found);
}

#[test]
fn test_residue_strongest() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 100);
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.7, 200);
    add_test_residue(&mut engine, ResidueKind::LingeringSadness, 0.5, 300);
    let strongest = engine.strongest_residue().unwrap();
    assert!((strongest.intensity - 0.7).abs() < 1e-9);
    assert_eq!(strongest.kind, ResidueKind::Afterglow);
}

#[test]
fn test_residue_total_intensity() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.3, 100);
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 200);
    let total = engine.total_intensity();
    assert!((total - 0.8).abs() < 1e-9);
}

#[test]
fn test_residue_interaction_amplify() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
    add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200);
    let factor = engine.residue_interaction_factor();
    assert!((factor - 1.15).abs() < 1e-9, "factor={}", factor);
}

#[test]
fn test_residue_interaction_dampen() {
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.5, 100);
    add_test_residue(&mut engine, ResidueKind::Tension, 0.4, 200);
    let factor = engine.residue_interaction_factor();
    assert!((factor - 0.85).abs() < 1e-9, "factor={}", factor);
}

// ── P3-C: O(N) 残留交互因子优化测试 / O(N) residue interaction factor tests ──

#[test]
fn test_residue_interaction_no_residues() {
    // 无残留时交互因子应为 1.0 / No residues → factor = 1.0
    let engine = ResidueEngine::new(ResidueConfig::default());
    let factor = engine.residue_interaction_factor();
    assert!((factor - 1.0).abs() < 1e-9, "factor={}", factor);
}

#[test]
fn test_residue_interaction_single_kind() {
    // 单一类型残留无交互 → 因子 1.0 / Single kind → no interaction → 1.0
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
    let factor = engine.residue_interaction_factor();
    assert!((factor - 1.0).abs() < 1e-9, "factor={}", factor);
}

#[test]
fn test_residue_interaction_multiple_same_kind_pairs() {
    // 多个同类残留：2 Tension + 2 SmolderingAnger → 4 对交互 → 1.15^4
    // Multiple same-kind: 2 Tension + 2 SmolderingAnger → 4 pairs → 1.15^4
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
    add_test_residue(&mut engine, ResidueKind::Tension, 0.4, 200);
    add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.3, 300);
    add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.2, 400);
    let factor = engine.residue_interaction_factor();
    let expected = 1.15_f64.powi(4); // 2 × 2 = 4 pairs
    assert!(
        (factor - expected).abs() < 1e-9,
        "expected {:.6}, got {:.6}",
        expected,
        factor
    );
}

#[test]
fn test_residue_interaction_mixed_amplify_and_suppress() {
    // 混合放大与抵消：Tension + SmolderingAnger + Afterglow
    // → 1.15 (Tension×SmolderingAnger) × 0.85 (Afterglow×Tension)
    // Mixed amplify and suppress: Tension + SmolderingAnger + Afterglow
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
    add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200);
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.3, 300);
    let factor = engine.residue_interaction_factor();
    let expected = 1.15 * 0.85; // Tension×SmolderingAnger + Afterglow×Tension
    assert!(
        (factor - expected).abs() < 1e-9,
        "expected {:.6}, got {:.6}",
        expected,
        factor
    );
}

#[test]
fn test_residue_interaction_all_six_pairs() {
    // 全部 6 对交互同时存在 / All 6 interaction pairs present
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100);
    add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200);
    add_test_residue(&mut engine, ResidueKind::LingeringSadness, 0.3, 300);
    add_test_residue(&mut engine, ResidueKind::SelfDoubtResidue, 0.3, 400);
    add_test_residue(&mut engine, ResidueKind::Afterglow, 0.3, 500);
    add_test_residue(&mut engine, ResidueKind::WarmthResidue, 0.3, 600);
    add_test_residue(&mut engine, ResidueKind::IntimacyDeepening, 0.3, 700);
    add_test_residue(&mut engine, ResidueKind::TrustMicroFracture, 0.3, 800);
    let factor = engine.residue_interaction_factor();
    // 1.15 × 1.1 × 1.1 × 0.85 × 0.8 × 0.9
    let expected = 1.15 * 1.1 * 1.1 * 0.85 * 0.8 * 0.9;
    assert!(
        (factor - expected).abs() < 1e-9,
        "expected {:.6}, got {:.6}",
        expected,
        factor
    );
}

#[test]
fn test_residue_interaction_clamp_upper_bound() {
    // 大量放大对 → 应被 clamp 到 2.0 / Many amplifying pairs → clamped to 2.0
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    // 5 Tension + 5 SmolderingAnger → 25 对 × 1.15 → 远超 2.0
    for i in 0..5 {
        add_test_residue(&mut engine, ResidueKind::Tension, 0.5, 100 + i);
        add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200 + i);
    }
    let factor = engine.residue_interaction_factor();
    assert!(
        (factor - 2.0).abs() < 1e-9,
        "should be clamped to 2.0, got {:.6}",
        factor
    );
}

#[test]
fn test_residue_interaction_clamp_lower_bound() {
    // 大量抵消对 → 应被 clamp 到 0.5 / Many suppressing pairs → clamped to 0.5
    let mut engine = ResidueEngine::new(ResidueConfig::default());
    // 5 WarmthResidue + 5 SmolderingAnger → 25 对 × 0.8 → 远低于 0.5
    for i in 0..5 {
        add_test_residue(&mut engine, ResidueKind::WarmthResidue, 0.5, 100 + i);
        add_test_residue(&mut engine, ResidueKind::SmolderingAnger, 0.4, 200 + i);
    }
    let factor = engine.residue_interaction_factor();
    assert!(
        (factor - 0.5).abs() < 1e-9,
        "should be clamped to 0.5, got {:.6}",
        factor
    );
}

#[test]
fn test_contagion_deterministic_anger_to_sadness() {
    // 确定性 RNG：固定种子确保可复现 / Deterministic RNG: fixed seed for reproducibility
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    let profile = EmotionProfile {
        anger: 0.8,
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(42);
    let triggered = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng,
    );
    // 确定性种子下应触发传染 / Deterministic seed should trigger contagion
    assert!(
        !triggered.is_empty(),
        "should trigger at least one contagion with deterministic seed"
    );
    let has_sadness = triggered
        .iter()
        .any(|c| c.target_emotion == ContagionEmotion::Sadness);
    assert!(has_sadness, "should have Anger->Sadness contagion");
}

#[test]
fn test_contagion_cooldown_prevents_retrigger() {
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    let profile = EmotionProfile {
        anger: 0.9,
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(42);
    let first = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng,
    );
    assert!(!first.is_empty());
    let second = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1050,
        &mut rng,
    );
    assert!(second.is_empty(), "cooldown should prevent retrigger");
}

#[test]
fn test_contagion_relationship_depth_filter() {
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    let profile = EmotionProfile {
        anger: 0.9,
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(42);
    let triggered_any = engine.evaluate(
        &profile,
        RelationshipDepth::Any,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng,
    );
    engine.clear_cooldown();
    let mut rng2 = SmallRng::seed_from_u64(42);
    let triggered_deep = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng2,
    );
    // Anger rules require TrustedOrAbove/DeepOnly, so Any triggers none
    assert!(
        triggered_any.is_empty(),
        "Any depth should not trigger anger contagions (rules require higher depth)"
    );
    // DeepOnly should trigger AngerToSadness (min_relationship_depth=DeepOnly)
    assert!(
        !triggered_deep.is_empty(),
        "DeepOnly should trigger at least one anger contagion"
    );
}

#[test]
fn test_contagion_maturity_filter() {
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    let profile = EmotionProfile {
        joy: 0.9,
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(42);
    let triggered_any = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::Any,
        1000,
        &mut rng,
    );
    let any_count = triggered_any.len();
    engine.clear_cooldown();
    let mut rng2 = SmallRng::seed_from_u64(42);
    let triggered_mature = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng2,
    );
    assert!(triggered_mature.len() <= any_count + 1);
}

#[test]
fn test_contagion_get_recent_for_emotion() {
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    let profile = EmotionProfile {
        anger: 0.9,
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(42);
    let _ = engine.evaluate(
        &profile,
        RelationshipDepth::DeepOnly,
        MaturityDepth::MatureOrAbove,
        1000,
        &mut rng,
    );
    let sadness_contagions = engine.get_recent_for_emotion(ContagionEmotion::Sadness);
    assert!(
        !sadness_contagions.is_empty(),
        "should have Anger->Sadness contagion in recent"
    );
    let joy_contagions = engine.get_recent_for_emotion(ContagionEmotion::Joy);
    assert!(joy_contagions.is_empty(), "no Joy contagion should exist");
}

// ══════════════════════════════════════════════════════════════
// C3.1: 延迟传染测试
// ══════════════════════════════════════════════════════════════

#[test]
fn test_rule_delay_values() {
    // 验证各规则延迟时间 / Verify delay values for each rule
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::AngerToGuilt),
        30.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::AngerToSadness),
        60.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::SadnessToAnger),
        15.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::AngerSadnessToShame),
        45.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::JoyNostalgiaToGratitude),
        20.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::PrideAnxietyToEnvy),
        90.0
    );
    // 即时传染 / Immediate contagions
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::AnxietyToExcitement),
        0.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::AnxietyContagion),
        0.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::CalmContagion),
        0.0
    );
    assert_eq!(
        ContagionEngine::rule_delay(ContagionRule::JoyContagion),
        0.0
    );
}

#[test]
fn test_pending_contagion_queue() {
    // 延迟传染加入队列 / Delayed contagions added to pending queue
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    engine.clear_cooldown();
    let profile = EmotionProfile {
        anger: 0.8,
        ..Default::default()
    };

    let now = 1000i64;
    let mut rng = SmallRng::seed_from_u64(42);
    let triggered = engine.evaluate(
        &profile,
        RelationshipDepth::TrustedOrAbove,
        MaturityDepth::GrowingOrAbove,
        now,
        &mut rng,
    );

    // 应有传染触发 / Should have triggered contagions
    assert!(!triggered.is_empty(), "应有传染触发");

    // 检查有延迟的传染是否进入 pending 队列 / Check delayed contagions in pending
    let delayed: Vec<_> = triggered.iter().filter(|c| c.delay_secs > 0.0).collect();
    if !delayed.is_empty() {
        assert!(
            engine.pending_count() > 0,
            "有延迟传染时应进入 pending 队列"
        );
    }
}

#[test]
fn test_tick_executes_due_contagions() {
    // tick() 执行到期延迟传染 / tick() executes due pending contagions
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    engine.clear_cooldown();

    // 手动添加延迟传染 / Manually add pending contagion
    // 创建时间 t=500，第一个 t=1000 到期，第二个 t=2000 到期
    // Created at t=500, first due at t=1000, second due at t=2000
    engine.pending.push(PendingContagion {
        rule: ContagionRule::AngerToGuilt,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        strength: 0.5,
        original_strength: 0.5,
        pad_template: [-0.2, -0.3, -0.3],
        trigger_time: 1000,
        created_at: 500,
        contagion_id: 1,
    });
    engine.pending.push(PendingContagion {
        rule: ContagionRule::AngerToSadness,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Sadness,
        strength: 0.3,
        original_strength: 0.3,
        pad_template: [-0.3, -0.2, -0.1],
        trigger_time: 2000, // 未到期 / Not yet due
        created_at: 500,
        contagion_id: 2,
    });

    assert_eq!(engine.pending_count(), 2);

    // 在 t=1500 时，只有第一个到期 / At t=1500, only first is due
    let effects = engine.tick(1500);
    assert_eq!(effects.len(), 1, "应只有1个到期传染");
    assert_eq!(effects[0].target_emotion, ContagionEmotion::Guilt);
    assert_eq!(effects[0].id, 1);

    // pending 队列应只剩1个 / Pending queue should have 1 remaining
    assert_eq!(engine.pending_count(), 1);

    // 在 t=3000 时，第二个也到期 / At t=3000, second is also due
    let effects2 = engine.tick(3000);
    assert_eq!(effects2.len(), 1);
    assert_eq!(effects2[0].target_emotion, ContagionEmotion::Sadness);
    assert_eq!(engine.pending_count(), 0);
}

#[test]
fn test_tick_no_due_contagions() {
    // 无到期传染时返回空 / No due contagions returns empty
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    engine.pending.push(PendingContagion {
        rule: ContagionRule::AngerToGuilt,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        strength: 0.5,
        original_strength: 0.5,
        pad_template: [-0.2, -0.3, -0.3],
        trigger_time: 1000,
        created_at: 0,
        contagion_id: 1,
    });

    let effects = engine.tick(500); // 未到期 / Not yet due
    assert!(effects.is_empty());
    assert_eq!(engine.pending_count(), 1, "未到期传染应保留在队列中");
}

#[test]
fn test_contagion_effect_structure() {
    // ContagionEffect 结构正确 / ContagionEffect structure is correct
    let effect = ContagionEffect {
        id: 42,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        rule: ContagionRule::AngerToGuilt,
        strength: 0.6,
        pad_offset: [-0.2, -0.3, -0.3],
        delay_secs: 30.0,
        triggered_at: 1000,
    };
    assert_eq!(effect.id, 42);
    assert_eq!(effect.source_emotion, ContagionEmotion::Anger);
    assert_eq!(effect.target_emotion, ContagionEmotion::Guilt);
    assert_eq!(effect.rule, ContagionRule::AngerToGuilt);
    assert!((effect.delay_secs - 30.0).abs() < 1e-10);
    assert!((effect.strength - 0.6).abs() < 1e-10);
}

// ══════════════════════════════════════════════════════════════
// C3.2: RandomMode 确定性生产模式测试
// ══════════════════════════════════════════════════════════════

#[test]
fn test_random_mode_default_stochastic() {
    // 默认为随机模式 / Default is stochastic mode
    let mode = RandomMode::default();
    assert_eq!(mode, RandomMode::Stochastic);
}

#[test]
fn test_random_mode_deterministic() {
    // 确定性模式 / Deterministic mode
    let mode = RandomMode::Deterministic { seed: 42 };
    assert_eq!(mode, RandomMode::Deterministic { seed: 42 });
    assert_ne!(mode, RandomMode::Stochastic);
}

#[test]
fn test_irrationality_manager_default_stochastic() {
    // 默认管理器使用随机模式 / Default manager uses stochastic mode
    let mgr = IrrationalityManager::default();
    assert_eq!(mgr.random_mode, RandomMode::Stochastic);
}

#[test]
fn test_irrationality_manager_with_deterministic() {
    // 切换到确定性模式 / Switch to deterministic mode
    let mgr =
        IrrationalityManager::default().with_random_mode(RandomMode::Deterministic { seed: 12345 });
    assert_eq!(mgr.random_mode, RandomMode::Deterministic { seed: 12345 });
}

#[test]
fn test_evaluate_contagion_deterministic_dispatch() {
    // 确定性模式：统一代码路径，通过内置 SmallRng 注入随机源
    // Deterministic mode: unified code path, injects RNG via built-in SmallRng
    let mut mgr =
        IrrationalityManager::default().with_random_mode(RandomMode::Deterministic { seed: 42 });
    mgr.contagion.clear_cooldown();

    let profile = EmotionProfile {
        anger: 0.8,
        ..Default::default()
    };

    let now = 1000i64;
    let contagions = mgr.evaluate_contagion(
        &profile,
        RelationshipDepth::TrustedOrAbove,
        MaturityDepth::GrowingOrAbove,
        now,
    );
    // 确定性种子下应触发传染 / Deterministic seed should trigger contagion
    assert!(!contagions.is_empty(), "确定性模式满足条件时应触发传染");
}

#[test]
fn test_evaluate_contagion_stochastic_dispatch() {
    // 随机模式：统一代码路径，内置 SmallRng 从熵源初始化
    // Stochastic mode: unified code path, built-in SmallRng from entropy
    let mut mgr = IrrationalityManager::default().with_random_mode(RandomMode::Stochastic);
    mgr.contagion.clear_cooldown();

    let profile = EmotionProfile {
        anger: 0.8,
        ..Default::default()
    };

    let now = 1000i64;
    // 多次调用，至少有一次触发（概率性）/ Multiple calls, at least one should trigger
    let mut any_triggered = false;
    for i in 0..20 {
        mgr.contagion.clear_cooldown();
        let contagions = mgr.evaluate_contagion(
            &profile,
            RelationshipDepth::TrustedOrAbove,
            MaturityDepth::GrowingOrAbove,
            now + i * 1000,
        );
        if !contagions.is_empty() {
            any_triggered = true;
            break;
        }
    }
    assert!(any_triggered, "随机模式在多次调用中应至少触发一次传染");
}
// ══════════════════════════════════════════════════════════════
// C3.1 增强测试：指数衰减 + 传染效果接入 + 提示片段
// ══════════════════════════════════════════════════════════════

#[test]
fn test_pending_contagion_exponential_decay() {
    // 延迟传染强度随等待时间指数衰减 / Pending contagion strength decays exponentially with wait time
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    // 创建时间 t=0，触发时间 t=100，在 t=100 时执行
    // 等待 100 秒，衰减因子 = e^(-0.05 * 100) ≈ 0.0067
    // Created at t=0, trigger at t=100, executed at t=100
    // Wait 100s, decay factor = e^(-0.05 * 100) ≈ 0.0067
    engine.pending.push(PendingContagion {
        rule: ContagionRule::AngerToGuilt,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        strength: 0.8,
        original_strength: 0.8,
        pad_template: [-0.2, -0.3, -0.3],
        trigger_time: 100,
        created_at: 0,
        contagion_id: 1,
    });

    let effects = engine.tick(100);
    assert_eq!(effects.len(), 1);
    // 100秒等待后，强度应显著衰减 / After 100s wait, strength should be significantly decayed
    let expected_strength = 0.8 * (-0.05_f64 * 100.0_f64).exp();
    assert!(
        (effects[0].strength - expected_strength).abs() < 1e-10,
        "expected {:.6}, got {:.6}",
        expected_strength,
        effects[0].strength
    );
    // 衰减后强度远小于原始 / Decayed strength much less than original
    assert!(
        effects[0].strength < 0.1,
        "强度应衰减到0.1以下，实际: {}",
        effects[0].strength
    );
}

#[test]
fn test_pending_contagion_short_delay_minimal_decay() {
    // 短延迟几乎不衰减 / Short delay has minimal decay
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    // 创建时间 t=0，触发时间 t=5，等待 5 秒
    // 衰减因子 = e^(-0.05 * 5) ≈ 0.778
    engine.pending.push(PendingContagion {
        rule: ContagionRule::SadnessToAnger,
        source_emotion: ContagionEmotion::Sadness,
        target_emotion: ContagionEmotion::Anger,
        strength: 0.6,
        original_strength: 0.6,
        pad_template: [-0.2, 0.4, 0.2],
        trigger_time: 5,
        created_at: 0,
        contagion_id: 1,
    });

    let effects = engine.tick(5);
    assert_eq!(effects.len(), 1);
    // 5秒等待后衰减很小 / After 5s wait, decay is small
    let expected = 0.6 * (-0.05_f64 * 5.0_f64).exp();
    assert!(
        (effects[0].strength - expected).abs() < 1e-10,
        "expected {:.6}, got {:.6}",
        expected,
        effects[0].strength
    );
    assert!(effects[0].strength > 0.4, "短延迟后强度应保留大部分");
}

#[test]
fn test_contagion_effect_diagnostic_fields() {
    // ContagionEffect 包含完整诊断信息 / ContagionEffect contains full diagnostic info
    let mut engine = ContagionEngine::new(ContagionConfig::default());
    engine.pending.push(PendingContagion {
        rule: ContagionRule::PrideAnxietyToEnvy,
        source_emotion: ContagionEmotion::Pride,
        target_emotion: ContagionEmotion::Envy,
        strength: 0.4,
        original_strength: 0.4,
        pad_template: [-0.2, 0.1, -0.2],
        trigger_time: 90,
        created_at: 0,
        contagion_id: 42,
    });

    let effects = engine.tick(90);
    assert_eq!(effects.len(), 1);
    let e = &effects[0];
    // 验证诊断字段 / Verify diagnostic fields
    assert_eq!(e.id, 42);
    assert_eq!(e.source_emotion, ContagionEmotion::Pride);
    assert_eq!(e.target_emotion, ContagionEmotion::Envy);
    assert_eq!(e.rule, ContagionRule::PrideAnxietyToEnvy);
    // delay_secs = trigger_time - created_at = 90 - 0 = 90
    assert!(
        (e.delay_secs - 90.0).abs() < 1e-10,
        "delay_secs should be 90.0, got {}",
        e.delay_secs
    );
    assert_eq!(e.triggered_at, 90);
}

#[test]
fn test_manager_tick_wires_contagion_effects() {
    // IrrationalityManager.tick() 将传染效果接入残留引擎
    // IrrationalityManager.tick() wires contagion effects into residue engine
    let mut mgr = IrrationalityManager::default();
    // 手动添加延迟传染 / Manually add pending contagion
    mgr.contagion.pending.push(PendingContagion {
        rule: ContagionRule::AngerToGuilt,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        strength: 0.5,
        original_strength: 0.5,
        pad_template: [-0.2, -0.3, -0.3],
        trigger_time: 1000,
        created_at: 970, // 30秒延迟 / 30s delay
        contagion_id: 1,
    });

    let residue_count_before = mgr.residue.active_residues.len();
    // tick 应执行到期传染并注入残留 / tick should execute due contagion and inject residue
    mgr.tick(&[0.0, 0.0, 0.0], 1000);
    let residue_count_after = mgr.residue.active_residues.len();
    // 传染效果应产生新残留 / Contagion effect should produce new residue
    assert!(
        residue_count_after > residue_count_before,
        "传染效果应注入残留: before={}, after={}",
        residue_count_before,
        residue_count_after
    );
}

#[test]
fn test_prompt_fragment_with_pending_contagion() {
    // 提示片段包含延迟传染信息 / Prompt fragment includes pending contagion info
    let mut mgr = IrrationalityManager::default();
    mgr.contagion.pending.push(PendingContagion {
        rule: ContagionRule::AngerToGuilt,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        strength: 0.5,
        original_strength: 0.5,
        pad_template: [-0.2, -0.3, -0.3],
        trigger_time: 2000,
        created_at: 1970,
        contagion_id: 1,
    });

    let fragment = mgr.to_prompt_fragment(1000);
    assert!(
        fragment.contains("[延迟传染]"),
        "提示片段应包含延迟传染信息: {}",
        fragment
    );
    assert!(
        fragment.contains("愤怒→内疚"),
        "提示片段应包含传染规则描述: {}",
        fragment
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// G1-G5 增强方法测试 / G1-G5 Enhancement Method Tests
// ═══════════════════════════════════════════════════════════════════════════

// ── G1: 情绪健康报告测试 / Emotional Health Report Tests ──

#[test]
fn test_health_report_calm_basin() {
    let mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    let report = mgr.health_report(now);
    // 初始状态：平静吸引子，无残留 → 高健康分
    assert!(
        report.overall_score > 0.8,
        "初始健康分应>0.8，实际: {}",
        report.overall_score
    );
    assert!(matches!(report.attractor, StrangeAttractor::CalmBasin));
    assert!(report.imbalance_warning.is_none(), "初始状态不应有失衡警告");
}

#[test]
fn test_health_report_with_negative_residues() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加6个负向残留 / Add 6 negative residues
    for _ in 0..6 {
        mgr.residue.active_residues.push(EmotionResidue {
            id: mgr.residue.next_id,
            kind: ResidueKind::SmolderingAnger,
            intensity: 0.5,
            pad_offset: [-0.3, 0.2, 0.0],
            half_life_secs: 1800.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 0.5),
            expressed: false,
        });
        mgr.residue.next_id += 1;
    }
    let report = mgr.health_report(now);
    assert!(matches!(
        report.dominant_valence,
        EmotionalValence::Negative
    ));
    assert_eq!(report.negative_residue_count, 6);
    assert!(
        report.imbalance_warning.is_some(),
        "6个负向残留应触发失衡警告"
    );
}

#[test]
fn test_health_report_with_positive_residues() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加4个正向残留 / Add 4 positive residues
    for kind in [
        ResidueKind::Afterglow,
        ResidueKind::WarmthResidue,
        ResidueKind::IntimacyDeepening,
        ResidueKind::AccomplishmentResidue,
    ] {
        mgr.residue.active_residues.push(EmotionResidue {
            id: mgr.residue.next_id,
            kind,
            intensity: 0.5,
            pad_offset: [0.2, 0.1, 0.0],
            half_life_secs: 3600.0,
            created_at: now,
            updated_at: now,
            source_pulse_id: None,
            body_memory: BodyMemory::from_residue_kind(kind, 0.5),
            expressed: false,
        });
        mgr.residue.next_id += 1;
    }
    let report = mgr.health_report(now);
    assert!(matches!(
        report.dominant_valence,
        EmotionalValence::Positive
    ));
    assert_eq!(report.positive_residue_count, 4);
}

// ── G2: 传染因果追溯测试 / Contagion Causal Tracing Tests ──

#[test]
fn test_contagion_chain_empty() {
    let mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
    assert!(chain.is_none(), "无传染记录时应返回None");
}

#[test]
fn test_contagion_chain_single() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加一条传染记录：愤怒→内疚 / Add one contagion: Anger→Guilt
    mgr.contagion.recent_contagions.push(CrossContagion {
        id: 1,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        rule: ContagionRule::AngerToGuilt,
        strength: 0.8,
        delay_secs: 0.0,
        condition: ContagionCondition {
            min_source_intensity: 0.3,
            min_relationship_depth: RelationshipDepth::Any,
            min_maturity: MaturityDepth::Any,
            probability: 0.5,
        },
        timestamp: now,
    });
    let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
    assert!(chain.is_some());
    let chain = chain.unwrap();
    assert_eq!(chain.nodes.len(), 1);
    assert_eq!(chain.nodes[0].source, ContagionEmotion::Anger);
    assert_eq!(chain.nodes[0].target, ContagionEmotion::Guilt);
}

#[test]
fn test_contagion_chain_multi_hop() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 悲伤→愤怒→内疚 / Sadness→Anger→Guilt
    mgr.contagion.recent_contagions.push(CrossContagion {
        id: 1,
        source_emotion: ContagionEmotion::Sadness,
        target_emotion: ContagionEmotion::Anger,
        rule: ContagionRule::SadnessToAnger,
        strength: 0.6,
        delay_secs: 0.0,
        condition: ContagionCondition {
            min_source_intensity: 0.3,
            min_relationship_depth: RelationshipDepth::Any,
            min_maturity: MaturityDepth::Any,
            probability: 0.5,
        },
        timestamp: now - 10,
    });
    mgr.contagion.recent_contagions.push(CrossContagion {
        id: 2,
        source_emotion: ContagionEmotion::Anger,
        target_emotion: ContagionEmotion::Guilt,
        rule: ContagionRule::AngerToGuilt,
        strength: 0.8,
        delay_secs: 0.0,
        condition: ContagionCondition {
            min_source_intensity: 0.3,
            min_relationship_depth: RelationshipDepth::Any,
            min_maturity: MaturityDepth::Any,
            probability: 0.5,
        },
        timestamp: now,
    });
    let chain = mgr.contagion_chain(ContagionEmotion::Guilt);
    assert!(chain.is_some());
    let chain = chain.unwrap();
    assert_eq!(chain.nodes.len(), 2, "应回溯2跳");
    // 源头在前 / Source first
    assert_eq!(chain.nodes[0].source, ContagionEmotion::Sadness);
    assert_eq!(chain.nodes[0].target, ContagionEmotion::Anger);
    assert_eq!(chain.nodes[1].source, ContagionEmotion::Anger);
    assert_eq!(chain.nodes[1].target, ContagionEmotion::Guilt);
}

// ── G3: 残留-身体双向信号测试 / Residue-Body Bidirectional Signal Tests ──

#[test]
fn test_residue_body_signal_neutral() {
    let mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    let signal = mgr.residue_body_signal(now);
    // 初始状态：无身体紧张→无催生残留 / Initial: no tension → no bred residue
    assert!(signal.body_born_residue.is_none());
    assert_eq!(signal.body_born_strength, 0.0);
}

#[test]
fn test_residue_body_signal_high_tension() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加高紧张残留 / Add high-tension residue
    mgr.residue.active_residues.push(EmotionResidue {
        id: 1,
        kind: ResidueKind::Tension,
        intensity: 0.8,
        pad_offset: [0.0, 0.3, 0.0],
        half_life_secs: 1800.0,
        created_at: now,
        updated_at: now,
        source_pulse_id: None,
        body_memory: BodyMemory {
            breath_offset: 0.1,
            tension: 0.8,
            heaviness: 0.0,
            warmth: 0.0,
        },
        expressed: false,
    });
    let signal = mgr.residue_body_signal(now);
    assert!(signal.body_born_residue.is_some(), "高紧张应催生残留");
    assert!(signal.body_born_strength > 0.0, "催生强度应>0");
}

#[test]
fn test_apply_residue_body_signal() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    let before_count = mgr.residue.active_residues.len();
    // 添加高温暖残留触发身体→残留通道 / Add high-warmth residue to trigger body→residue
    mgr.residue.active_residues.push(EmotionResidue {
        id: 1,
        kind: ResidueKind::WarmthResidue,
        intensity: 0.8,
        pad_offset: [0.3, 0.1, 0.0],
        half_life_secs: 3600.0,
        created_at: now,
        updated_at: now,
        source_pulse_id: None,
        body_memory: BodyMemory {
            breath_offset: 0.0,
            tension: 0.0,
            heaviness: 0.0,
            warmth: 0.8,
        },
        expressed: false,
    });
    mgr.apply_residue_body_signal(now);
    // 高温暖应催生WarmthResidue / High warmth should breed WarmthResidue
    assert!(
        mgr.residue.active_residues.len() > before_count + 1,
        "应新增身体催生的残留"
    );
}

// ── G4: 脉冲-残留交互测试 / Pulse-Residue Interaction Tests ──

#[test]
fn test_pulse_residue_interaction_no_overlap() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加喜悦脉冲和悲伤残留（对立→抑制）/ Add joy pulse + sadness residue (opposite → suppress)
    mgr.pulse.active_pulses.push(ChaoticPulse {
        id: 1,
        kind: PulseKind::JoyBurst,
        intensity: 0.8,
        pad_impulse: [0.5, 0.5, 0.3],
        duration_secs: 30.0,
        decay_curve: DecayCurve::Exponential { lambda: 0.1 },
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "joy".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: now,
        absorbed: false,
        residual_intensity: 0.0,
    });
    mgr.residue.active_residues.push(EmotionResidue {
        id: 1,
        kind: ResidueKind::LingeringSadness,
        intensity: 0.6,
        pad_offset: [-0.3, 0.1, 0.0],
        half_life_secs: 3600.0,
        created_at: now,
        updated_at: now,
        source_pulse_id: None,
        body_memory: BodyMemory::from_residue_kind(ResidueKind::LingeringSadness, 0.6),
        expressed: false,
    });
    let interaction = mgr.pulse_residue_interaction();
    // 喜悦应抑制悲伤 / Joy should suppress sadness
    assert!(!interaction.suppressed.is_empty(), "喜悦脉冲应抑制悲伤残留");
}

#[test]
fn test_pulse_residue_interaction_resonance() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let now = 1000;
    // 添加愤怒脉冲和余怒残留（同类→放大）/ Add anger pulse + smoldering anger (same-kind → amplify)
    mgr.pulse.active_pulses.push(ChaoticPulse {
        id: 1,
        kind: PulseKind::AngerFlash,
        intensity: 0.7,
        pad_impulse: [-0.5, 0.6, 0.2],
        duration_secs: 20.0,
        decay_curve: DecayCurve::Exponential { lambda: 0.1 },
        trigger: PulseTrigger {
            source: PulseSource::UserMessage,
            signal: "anger".to_string(),
            baseline_pad: [0.0, 0.0, 0.0],
        },
        timestamp: now,
        absorbed: false,
        residual_intensity: 0.0,
    });
    mgr.residue.active_residues.push(EmotionResidue {
        id: 1,
        kind: ResidueKind::SmolderingAnger,
        intensity: 0.5,
        pad_offset: [-0.3, 0.2, 0.0],
        half_life_secs: 1800.0,
        created_at: now,
        updated_at: now,
        source_pulse_id: None,
        body_memory: BodyMemory::from_residue_kind(ResidueKind::SmolderingAnger, 0.5),
        expressed: false,
    });
    let interaction = mgr.pulse_residue_interaction();
    // 愤怒应放大余怒 / Anger should amplify smoldering anger
    assert!(!interaction.amplified.is_empty(), "愤怒脉冲应放大余怒残留");
    assert!(interaction.amplified[0].1 > 1.0, "放大因子应>1.0");
}

// ── G5: 涌现-传染联动测试 / Emergence-Contagion Linkage Tests ──

#[test]
fn test_emergence_contagion_link_empty() {
    let mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let links = mgr.emergence_contagion_link();
    assert!(links.is_empty(), "无涌现模式时应返回空");
}

#[test]
fn test_emergence_contagion_link_bifurcation() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    // 添加分岔点涌现 / Add bifurcation emergence
    mgr.chaos
        .state
        .emergent_patterns
        .push_back(EmergentPattern {
            kind: EmergentKind::Bifurcation,
            strength: 0.8,
            detected_at: 1000,
            description: "test bifurcation".to_string(),
        });
    let links = mgr.emergence_contagion_link();
    assert_eq!(links.len(), 1);
    assert!(links[0].threshold_modulation < 1.0, "分岔点应降低传染阈值");
    assert!(!links[0].modulated_rules.is_empty(), "分岔点应调制传染规则");
}

#[test]
fn test_contagion_threshold_modulation_no_emergence() {
    let mgr = IrrationalityManager::new(IrrationalityConfig::default());
    let mod_factor = mgr.contagion_threshold_modulation();
    assert!((mod_factor - 1.0).abs() < 1e-6, "无涌现时调制因子应为1.0");
}

#[test]
fn test_contagion_threshold_modulation_with_bifurcation() {
    let mut mgr = IrrationalityManager::new(IrrationalityConfig::default());
    mgr.chaos
        .state
        .emergent_patterns
        .push_back(EmergentPattern {
            kind: EmergentKind::Bifurcation,
            strength: 0.8,
            detected_at: 1000,
            description: "test".to_string(),
        });
    let mod_factor = mgr.contagion_threshold_modulation();
    assert!(
        mod_factor < 1.0,
        "分岔点应降低调制因子(更易传染)，实际: {}",
        mod_factor
    );
}
