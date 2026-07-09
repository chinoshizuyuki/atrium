// 统一冲突引擎测试 / Unified Conflict Engine Tests
//
// 合并 conflict_growth + conflict_pattern_learner 全部测试 + 新增集成测试

use super::growth::*;
use super::pattern::*;
use super::*;
use crate::conflict_reconciliation::{ConflictIntensity, ConflictSignal, ConflictType};
use crate::relationship::RelationshipStage;

// ════════════════════════════════════════════════════════════════════
// 辅助函数 / Helper Functions
// ════════════════════════════════════════════════════════════════════

fn acquaintance() -> RelationshipStage {
    RelationshipStage::Acquaintance {
        since: 0,
        interactions: 0,
    }
}

fn deep_stage() -> RelationshipStage {
    RelationshipStage::Deep {
        since: 0,
        interactions: 100,
        shared_references: 10,
        key_moments: 5,
    }
}

fn familiar_stage() -> RelationshipStage {
    RelationshipStage::Familiar {
        since: 0,
        interactions: 50,
        shared_references: 5,
    }
}

fn trusted_stage() -> RelationshipStage {
    RelationshipStage::Trusted {
        since: 0,
        interactions: 80,
        shared_references: 8,
        key_moments: 3,
    }
}

fn make_signal(ct: ConflictType, trigger: &str, epoch: i64) -> ConflictSignal {
    ConflictSignal {
        conflict_type: ct,
        intensity: ConflictIntensity::Mild,
        confidence: 0.8,
        trigger_text: trigger.to_string(),
        context_clues: vec!["test_clue".to_string()],
        timestamp: epoch,
    }
}

// ════════════════════════════════════════════════════════════════════
// EscalationWarning 测试 / EscalationWarning Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_escalation_no_warning() {
    // 冲突强度稳定，无升级 / Stable intensity, no escalation
    let mut ew = EscalationWarning::new();
    ew.update(0.3, 1);
    ew.update(0.3, 2);
    ew.update(0.3, 3);
    assert_eq!(ew.warning_level(), EscalationWarningLevel::None);
}

#[test]
fn test_escalation_caution() {
    // 缓慢升级触发 Caution / Slow escalation triggers Caution
    let mut ew = EscalationWarning::new();
    ew.update(0.1, 1);
    ew.update(0.4, 2);
    let level = ew.warning_level();
    assert!(
        level == EscalationWarningLevel::Caution || level == EscalationWarningLevel::Warning,
        "expected Caution or Warning, got {:?}",
        level
    );
}

#[test]
fn test_escalation_warning() {
    // 快速升级触发 Warning / Rapid escalation triggers Warning
    let mut ew3 = EscalationWarning::new();
    ew3.update(0.1, 1);
    ew3.update(0.25, 2);
    ew3.update(0.7, 3);
    assert_eq!(ew3.warning_level(), EscalationWarningLevel::Warning);
}

#[test]
fn test_escalation_alert() {
    // 危险升级触发 Alert / Dangerous escalation triggers Alert
    let mut ew = EscalationWarning::new();
    ew.update(0.05, 1);
    ew.update(0.2, 2);
    ew.update(0.75, 3);
    assert_eq!(ew.warning_level(), EscalationWarningLevel::Alert);
}

#[test]
fn test_escalation_cooldown_reset() {
    // 连续 3 轮平静后重置 / Reset after 3 calm turns
    let mut ew = EscalationWarning::new();
    ew.update(0.1, 1);
    ew.update(0.5, 2);
    assert_ne!(ew.warning_level(), EscalationWarningLevel::None);

    ew.calm();
    ew.calm();
    ew.calm();

    assert_eq!(ew.velocity, 0.0);
    assert_eq!(ew.acceleration, 0.0);
    assert_eq!(ew.warning_level(), EscalationWarningLevel::None);
}

// ════════════════════════════════════════════════════════════════════
// ReconciliationTiming 测试 / ReconciliationTiming Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_timing_optimal_delay() {
    let mut rt = ReconciliationTiming::new();
    rt.set_conflict(0.5, 0.5);
    assert_eq!(rt.optimal_delay_secs(), 675);
}

#[test]
fn test_timing_ready() {
    let mut rt = ReconciliationTiming::new();
    rt.set_conflict(0.3, 0.0);
    assert!(rt.is_ready(0.2, 10));
    assert!(!rt.is_ready(-0.5, 10));
    assert!(!rt.is_ready(0.2, 2));
}

#[test]
fn test_timing_premature_penalty() {
    let mut rt = ReconciliationTiming::new();
    rt.set_conflict(0.3, 0.0);
    assert!((rt.premature_penalty(150) - 0.5).abs() < 1e-9);
    assert!((rt.premature_penalty(300) - 0.0).abs() < 1e-9);
    assert!((rt.premature_penalty(600) - 0.0).abs() < 1e-9);
}

#[test]
fn test_timing_different_intensities() {
    let mut rt = ReconciliationTiming::new();
    rt.set_conflict(0.1, 0.0);
    let trivial = rt.optimal_delay_secs();
    rt.set_conflict(0.9, 0.0);
    let critical = rt.optimal_delay_secs();
    assert!(trivial < critical);
    assert_eq!(trivial, 150);
    assert_eq!(critical, 1200);
}

// ════════════════════════════════════════════════════════════════════
// PostConflictGrowth 测试 / PostConflictGrowth Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_growth_score() {
    let mut pcg = PostConflictGrowth::new();
    let score = pcg.growth_score(0.8, 0.6);
    assert!((score - 0.48).abs() < 1e-9);

    pcg.successful_reconciliations = 3;
    let score2 = pcg.growth_score(0.8, 0.6);
    assert!((score2 - 0.624).abs() < 1e-9);
}

#[test]
fn test_resilience_up() {
    let mut pcg = PostConflictGrowth::new();
    let initial = pcg.resilience();
    pcg.record_growth("mild", 0.3, 0.8, 0.5, 0);
    pcg.record_growth("mild", 0.3, 0.8, 0.5, 0);
    let after = pcg.resilience();
    assert!(after > initial);
    assert!((after - 0.4).abs() < 1e-9);
}

#[test]
fn test_resilience_down() {
    let mut pcg = PostConflictGrowth::new();
    let initial = pcg.resilience();
    pcg.record_unresolved();
    pcg.record_unresolved();
    let after = pcg.resilience();
    assert!(after < initial);
    assert!((after - 0.1).abs() < 1e-9);
}

#[test]
fn test_growth_sliding_window() {
    let mut pcg = PostConflictGrowth::new();
    for i in 0..60 {
        pcg.record_growth("mild", 0.3, 0.5, 0.5, i);
    }
    assert_eq!(pcg.recent_entries().len(), 50);
    assert_eq!(pcg.recent_entries().front().unwrap().timestamp, 10);
}

// ════════════════════════════════════════════════════════════════════
// ConflictPattern 测试 / ConflictPattern Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_pattern_new() {
    let p = ConflictPattern::new(ConflictType::FactualDisagreement, 1000);
    assert_eq!(p.frequency, 1);
    assert_eq!(p.avg_intensity, 1.0);
    assert_eq!(p.decay_weight, 1.0);
    assert_eq!(p.stage_distribution, [0; 8]);
}

#[test]
fn test_pattern_absorb() {
    let mut p = ConflictPattern::new(ConflictType::OverDemand, 1000);
    let sig = make_signal(ConflictType::OverDemand, "帮我做这个", 2000);
    p.absorb(&sig, &acquaintance(), 2000);
    assert_eq!(p.frequency, 2);
    assert_eq!(p.last_seen_epoch, 2000);
    // Acquaintance 现为 idx 1 / Acquaintance is now idx 1
    assert_eq!(p.stage_distribution[1], 1);
    assert!(!p.trigger_keywords.is_empty());
}

#[test]
fn test_pattern_absorb_multiple_stages() {
    let mut p = ConflictPattern::new(ConflictType::ValueConflict, 1000);
    let sig1 = make_signal(ConflictType::ValueConflict, "不同意", 1000);
    p.absorb(&sig1, &acquaintance(), 1000);
    let sig2 = make_signal(ConflictType::ValueConflict, "不同意", 2000);
    p.absorb(&sig2, &deep_stage(), 2000);
    let sig3 = make_signal(ConflictType::ValueConflict, "反对", 3000);
    p.absorb(&sig3, &deep_stage(), 3000);
    assert_eq!(p.frequency, 4);
    // Acquaintance 现为 idx 1, Deep 现为 idx 6 / Acquaintance is now idx 1, Deep is now idx 6
    assert_eq!(p.stage_distribution[1], 1);
    assert_eq!(p.stage_distribution[6], 2);
}

#[test]
fn test_pattern_decay() {
    let mut p = ConflictPattern::new(ConflictType::FactualDisagreement, 1000);
    p.decay(0.995);
    assert!((p.decay_weight - 0.995).abs() < 1e-6);
    for _ in 0..99 {
        p.decay(0.995);
    }
    assert!(p.decay_weight > 0.5 && p.decay_weight < 0.7);
}

#[test]
fn test_pattern_should_prune() {
    let mut p = ConflictPattern::new(ConflictType::FactualDisagreement, 1000);
    p.frequency = 2;
    p.decay_weight = 0.05;
    assert!(p.should_prune(0.1));

    p.frequency = 5;
    assert!(!p.should_prune(0.1));

    p.frequency = 2;
    p.decay_weight = 0.5;
    assert!(!p.should_prune(0.1));
}

#[test]
fn test_stage_sensitivity() {
    assert!((ConflictPattern::stage_sensitivity(&acquaintance()) - 0.6).abs() < 1e-6);
    assert!((ConflictPattern::stage_sensitivity(&familiar_stage()) - 0.8).abs() < 1e-6);
    assert!((ConflictPattern::stage_sensitivity(&deep_stage()) - 1.2).abs() < 1e-6);
    assert!((ConflictPattern::stage_sensitivity(&trusted_stage()) - 1.0).abs() < 1e-6);
}

#[test]
fn test_stage_ratio() {
    let mut p = ConflictPattern::new(ConflictType::OverDemand, 1000);
    let sig = make_signal(ConflictType::OverDemand, "帮我", 1000);
    p.absorb(&sig, &deep_stage(), 1000);
    let sig2 = make_signal(ConflictType::OverDemand, "帮我", 2000);
    p.absorb(&sig2, &deep_stage(), 2000);
    assert!((p.stage_ratio(&deep_stage()) - 1.0).abs() < 1e-6);
    assert!((p.stage_ratio(&acquaintance()) - 0.0).abs() < 1e-6);
}

// ════════════════════════════════════════════════════════════════════
// ConflictPatternLearner 测试 / ConflictPatternLearner Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_learner_new() {
    let learner = ConflictPatternLearner::default();
    assert!(learner.patterns.is_empty());
    assert_eq!(learner.total_learns, 0);
}

#[test]
fn test_learner_learn_single() {
    let mut learner = ConflictPatternLearner::default();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    learner.learn(&[sig], &acquaintance(), 1000);
    assert_eq!(learner.patterns.len(), 1);
    assert_eq!(learner.total_learns, 1);
    assert_eq!(
        learner.patterns[0].conflict_type,
        ConflictType::FactualDisagreement
    );
}

#[test]
fn test_learner_learn_multiple_types() {
    let mut learner = ConflictPatternLearner::default();
    let sig1 = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    let sig2 = make_signal(ConflictType::OverDemand, "帮我做", 1000);
    learner.learn(&[sig1, sig2], &familiar_stage(), 1000);
    assert_eq!(learner.patterns.len(), 2);
    assert_eq!(learner.total_learns, 2);
}

#[test]
fn test_learner_learn_accumulate() {
    let mut learner = ConflictPatternLearner::default();
    for i in 0..5 {
        let sig = make_signal(ConflictType::ValueConflict, "不同意", 1000 + i * 100);
        learner.learn(&[sig], &deep_stage(), 1000 + i * 100);
    }
    assert_eq!(learner.patterns.len(), 1);
    assert_eq!(learner.patterns[0].frequency, 5);
    assert_eq!(learner.total_learns, 5);
}

#[test]
fn test_learner_predict_no_match() {
    let mut learner = ConflictPatternLearner::default();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    learner.learn(&[sig], &acquaintance(), 1000);
    let preds = learner.predict("今天天气真好", &acquaintance());
    assert!(preds.is_empty());
}

#[test]
fn test_learner_predict_with_match() {
    let mut learner = ConflictPatternLearner::default();
    for i in 0..5 {
        let sig = make_signal(ConflictType::OverDemand, "帮我", 1000 + i * 100);
        learner.learn(&[sig], &deep_stage(), 1000 + i * 100);
    }
    let preds = learner.predict("帮我做那个任务", &deep_stage());
    assert!(!preds.is_empty());
    assert_eq!(preds[0].conflict_type, ConflictType::OverDemand);
    assert!(preds[0].confidence > 0.0);
}

#[test]
fn test_learner_predict_stage_sensitivity() {
    let mut learner = ConflictPatternLearner::default();
    for i in 0..5 {
        let sig = make_signal(ConflictType::ValueConflict, "反对", 1000 + i * 100);
        learner.learn(&[sig], &deep_stage(), 1000 + i * 100);
    }
    let preds_deep = learner.predict("我反对这个观点", &deep_stage());
    let preds_acq = learner.predict("我反对这个观点", &acquaintance());
    if !preds_deep.is_empty() && !preds_acq.is_empty() {
        assert!(preds_deep[0].stage_sensitivity > preds_acq[0].stage_sensitivity);
    }
}

#[test]
fn test_learner_suggest_sensitivity() {
    let mut learner = ConflictPatternLearner::default();
    for i in 0..12 {
        let sig = make_signal(ConflictType::OverDemand, "帮我", 1000 + i * 100);
        learner.learn(&[sig], &acquaintance(), 1000 + i * 100);
    }
    let adjustments = learner.suggest_sensitivity_adjustments(&acquaintance());
    assert!(!adjustments.is_empty());
    let adj = adjustments
        .iter()
        .find(|a| a.conflict_type == ConflictType::OverDemand)
        .unwrap();
    assert!(adj.multiplier < 1.0);
}

#[test]
fn test_learner_to_prompt_fragment_empty() {
    let learner = ConflictPatternLearner::default();
    let frag = learner.to_prompt_fragment(&acquaintance());
    assert!(frag.is_empty());
}

#[test]
fn test_learner_to_prompt_fragment_with_patterns() {
    let mut learner = ConflictPatternLearner::default();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    learner.learn(&[sig], &deep_stage(), 1000);
    let frag = learner.to_prompt_fragment(&deep_stage());
    assert!(frag.contains("[Conflict Pattern Awareness]"));
    assert!(frag.contains("FactualDisagreement"));
}

#[test]
fn test_learner_tick_decay_and_prune() {
    let mut learner = ConflictPatternLearner::default();
    let sig = make_signal(ConflictType::Misunderstanding, "误解", 1000);
    learner.learn(&[sig], &acquaintance(), 1000);
    assert_eq!(learner.patterns.len(), 1);

    for i in 0..500 {
        learner.tick(2000 + i);
    }
    assert!(learner.patterns.is_empty() || learner.patterns[0].decay_weight < 0.5);
}

#[test]
fn test_learner_tick_preserves_high_freq() {
    let mut learner = ConflictPatternLearner::default();
    for i in 0..5 {
        let sig = make_signal(ConflictType::ValueConflict, "不同意", 1000 + i * 100);
        learner.learn(&[sig], &deep_stage(), 1000 + i * 100);
    }
    for i in 0..200 {
        learner.tick(5000 + i);
    }
    assert!(!learner.patterns.is_empty());
}

#[test]
fn test_learner_stats() {
    let mut learner = ConflictPatternLearner::default();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    learner.learn(&[sig], &acquaintance(), 1000);
    let stats = learner.stats();
    assert_eq!(stats.pattern_count, 1);
    assert_eq!(stats.total_learns, 1);
}

#[test]
fn test_learner_disabled() {
    let config = PatternLearnerConfig {
        enabled: false,
        ..Default::default()
    };
    let mut learner = ConflictPatternLearner::new(config);
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    learner.learn(&[sig], &acquaintance(), 1000);
    assert!(learner.patterns.is_empty());
    let preds = learner.predict("不对", &acquaintance());
    assert!(preds.is_empty());
    let frag = learner.to_prompt_fragment(&acquaintance());
    assert!(frag.is_empty());
}

#[test]
fn test_learner_max_patterns_cap() {
    let config = PatternLearnerConfig {
        max_patterns: 3,
        ..Default::default()
    };
    let mut learner = ConflictPatternLearner::new(config);
    let types = [
        ConflictType::FactualDisagreement,
        ConflictType::ValueConflict,
        ConflictType::ExpectationGap,
        ConflictType::BoundaryViolation,
    ];
    for (i, ct) in types.iter().enumerate() {
        let sig = make_signal(*ct, "trigger", 1000 + i as i64 * 100);
        learner.learn(&[sig], &acquaintance(), 1000 + i as i64 * 100);
    }
    assert!(learner.patterns.len() <= 3);
}

#[test]
fn test_pattern_keyword_dedup() {
    let mut p = ConflictPattern::new(ConflictType::OverDemand, 1000);
    let sig1 = make_signal(ConflictType::OverDemand, "帮我", 1000);
    p.absorb(&sig1, &deep_stage(), 1000);
    let sig2 = make_signal(ConflictType::OverDemand, "帮我", 2000);
    p.absorb(&sig2, &deep_stage(), 2000);
    let help_kw = p.trigger_keywords.iter().find(|(k, _)| k == "帮我");
    assert!(help_kw.is_some());
    assert!(help_kw.unwrap().1 >= 2);
}

// ════════════════════════════════════════════════════════════════════
// 统一 ConflictEngine 测试 / Unified ConflictEngine Tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn test_engine_on_conflict_learns_pattern() {
    // 冲突发生时模式被学习 / Pattern is learned on conflict
    let mut engine = ConflictEngine::new();
    let sig = make_signal(ConflictType::OverDemand, "帮我做", 1000);
    engine.on_conflict(0.5, 1, &[sig], &deep_stage(), 1000);
    assert!(engine.in_conflict);
    assert_eq!(engine.turns_since_conflict, 0);
    // 模式应被学习 / Pattern should be learned
    assert!(!engine.learner.patterns.is_empty());
    assert_eq!(engine.learner.total_learns, 1);
}

#[test]
fn test_engine_on_conflict_no_signals() {
    // 无信号时不学习 / No learning without signals
    let mut engine = ConflictEngine::new();
    engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
    assert!(engine.in_conflict);
    assert!(engine.learner.patterns.is_empty());
}

#[test]
fn test_engine_on_reconciliation() {
    // 和解完成时记录成长 / Engine records growth on reconciliation
    let mut engine = ConflictEngine::new();
    let sig = make_signal(ConflictType::OverDemand, "帮我", 1000);
    engine.on_conflict(0.5, 1, &[sig], &deep_stage(), 1000);
    engine.timing.set_conflict(0.5, 0.3);
    engine.on_reconciliation(0.8, 0.6, 700, 0.3);
    assert!(!engine.in_conflict);
    assert_eq!(engine.growth.successful_reconciliations, 1);
    assert_eq!(engine.growth.recent_entries().len(), 1);
}

#[test]
fn test_engine_on_calm() {
    // 平静期递增冷却 / Calm period increments cooldown
    let mut engine = ConflictEngine::new();
    engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
    engine.on_calm(0.1, 1);
    assert_eq!(engine.turns_since_conflict, 1);
    engine.on_calm(0.1, 11);
    assert_eq!(engine.growth.unresolved_count, 1);
    assert!(!engine.in_conflict);
}

#[test]
fn test_engine_tick_decays_patterns() {
    // tick 衰减模式 / Tick decays patterns
    let mut engine = ConflictEngine::new();
    let sig = make_signal(ConflictType::Misunderstanding, "误解", 1000);
    engine.on_conflict(0.3, 1, &[sig], &acquaintance(), 1000);
    assert!(!engine.learner.patterns.is_empty());

    let initial_weight = engine.learner.patterns[0].decay_weight;
    engine.tick(2000);
    assert!(engine.learner.patterns[0].decay_weight < initial_weight);
}

#[test]
fn test_engine_predict() {
    // 统一预测接口 / Unified predict interface
    let mut engine = ConflictEngine::new();
    for i in 0..5 {
        let sig = make_signal(ConflictType::OverDemand, "帮我", 1000 + i * 100);
        engine.on_conflict(0.4, i as u32 + 1, &[sig], &deep_stage(), 1000 + i * 100);
    }
    let preds = engine.predict("帮我做那个", &deep_stage());
    assert!(!preds.is_empty());
    assert_eq!(preds[0].conflict_type, ConflictType::OverDemand);
}

#[test]
fn test_engine_prompt_hint_combines_both() {
    // prompt 包含成长 + 模式片段 / Prompt includes growth + pattern fragments
    let mut engine = ConflictEngine::new();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    engine.on_conflict(0.5, 1, &[sig], &deep_stage(), 1000);

    let hint = engine.to_prompt_hint(&deep_stage());
    assert!(hint.contains("冲突成长"));
    assert!(hint.contains("Conflict Pattern Awareness"));
}

#[test]
fn test_engine_prompt_hint_growth_only() {
    // 仅成长 prompt（无模式时）/ Growth-only prompt (no patterns)
    let mut engine = ConflictEngine::new();
    engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
    let hint = engine.to_prompt_hint(&deep_stage());
    assert!(hint.contains("冲突成长"));
    // 无模式时不包含模式片段 / No pattern fragment when empty
    assert!(!hint.contains("Conflict Pattern Awareness"));
}

#[test]
fn test_engine_prompt_hint_budget_truncation() {
    // 预算截断 / Budget truncation
    let mut engine = ConflictEngine::new().with_prompt_budget(10);
    engine.on_conflict(0.5, 1, &[], &deep_stage(), 1000);
    let hint = engine.to_prompt_hint(&deep_stage());
    assert!(hint.len() <= 10);
}

#[test]
fn test_engine_serialization_roundtrip() {
    // 序列化往返 / Serialization round-trip
    let mut engine = ConflictEngine::new();
    let sig1 = make_signal(ConflictType::OverDemand, "帮我", 1000);
    engine.on_conflict(0.6, 1, &[sig1], &deep_stage(), 1000);
    let sig2 = make_signal(ConflictType::OverDemand, "帮我", 1100);
    engine.on_conflict(0.8, 2, &[sig2], &deep_stage(), 1100);
    engine.timing.set_conflict(0.7, 0.4);
    engine.on_reconciliation(0.7, 0.5, 500, 0.4);

    let snap = SerializableConflictEngine::from_engine(&engine);
    let json = serde_json::to_string(&snap).unwrap();
    let restored: SerializableConflictEngine = serde_json::from_str(&json).unwrap();
    let engine2 = restored.to_engine();

    assert_eq!(engine2.warning_level(), engine.warning_level());
    assert!((engine2.resilience_score() - engine.resilience_score()).abs() < 1e-9);
    assert_eq!(
        engine2.growth.successful_reconciliations,
        engine.growth.successful_reconciliations
    );
    assert_eq!(engine2.learner.total_learns, engine.learner.total_learns);
}

#[test]
fn test_engine_suggest_sensitivity() {
    // 灵敏度建议 / Sensitivity suggestions
    let mut engine = ConflictEngine::new();
    for i in 0..12 {
        let sig = make_signal(ConflictType::OverDemand, "帮我", 1000 + i * 100);
        engine.on_conflict(0.4, i as u32 + 1, &[sig], &acquaintance(), 1000 + i * 100);
    }
    let adjustments = engine.suggest_sensitivity(&acquaintance());
    assert!(!adjustments.is_empty());
}

#[test]
fn test_engine_pattern_stats() {
    // 模式统计 / Pattern statistics
    let mut engine = ConflictEngine::new();
    let sig = make_signal(ConflictType::FactualDisagreement, "不对", 1000);
    engine.on_conflict(0.5, 1, &[sig], &acquaintance(), 1000);
    let stats = engine.pattern_stats();
    assert_eq!(stats.pattern_count, 1);
    assert_eq!(stats.total_learns, 1);
}

#[test]
fn test_truncate_utf8() {
    // UTF-8 安全截断 / UTF-8 safe truncation
    assert_eq!(super::truncate_utf8("hello", 10), "hello");
    assert_eq!(super::truncate_utf8("hello", 3), "hel");
    // 中文字符不应被截断一半 / Chinese chars should not be split
    let chinese = "你好世界";
    let truncated = super::truncate_utf8(chinese, 5);
    // "你好" = 6 bytes, "你" = 3 bytes → 5 bytes truncates to "你" (3 bytes)
    assert_eq!(truncated, "你");
}

#[test]
fn test_backward_compat_type_alias() {
    // 向后兼容类型别名 / Backward compatible type alias
    let engine: ConflictGrowthEngine = ConflictEngine::new();
    assert!(!engine.in_conflict);

    let snap = SerializableConflictGrowth::from_engine(&engine);
    let _restored = snap.to_engine();
}
