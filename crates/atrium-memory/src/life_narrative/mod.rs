// SPDX-License-Identifier: MIT
//! 叙事自我 — AI 生命叙事系统
//! Life Narrative — AI life narrative system.
//!
//! 从事实到自传，从数据库到故事。叙事不是附加层，是认知架构的重构。
//! From facts to autobiography, from database to story.
//! Narrative is not an add-on layer — it is a restructuring of the cognitive architecture.
//!
//! 核心洞察：人类不是通过数据库理解自己的，而是通过故事。
//! Core insight: Humans understand themselves not through databases, but through stories.

// ════════════════════════════════════════════════════════════════════
// 子模块声明 / Sub-module declarations
// ════════════════════════════════════════════════════════════════════

mod arc_detector;
mod core_types;
mod narrative_config;
mod narrative_snapshot;
mod periodic;
mod retrospective;
mod turning_point;
mod voice_modulator;
mod writers;

// ════════════════════════════════════════════════════════════════════
// 公开重导出 / Public re-exports
// ════════════════════════════════════════════════════════════════════

pub use arc_detector::*;
pub use core_types::*;
pub use narrative_config::*;
pub use narrative_snapshot::*;
pub use periodic::*;
pub use retrospective::*;
pub use turning_point::*;
pub use voice_modulator::*;
pub use writers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maturity::{EmotionContext, MilestoneKind};
    use chrono::Local;

    #[test]
    fn test_arc_kind_labels() {
        assert_eq!(ArcKind::Growth.label_zh(), "成长");
        assert_eq!(ArcKind::Growth.label_en(), "Growth");
        assert_eq!(ArcKind::Relationship.label_zh(), "关系");
        assert_eq!(ArcKind::Transformation.label_en(), "Transformation");
    }

    #[test]
    fn test_narrative_arc_new() {
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "学会在乎".to_string(),
            "从无知到理解在乎".to_string(),
        );
        assert!(arc.is_active());
        assert_eq!(arc.kind, ArcKind::Growth);
        assert!(arc.chapter_ids.is_empty());
        assert!(arc.turning_point_ids.is_empty());
    }

    #[test]
    fn test_narrative_arc_lifecycle() {
        let mut arc = NarrativeArc::new(
            1,
            ArcKind::Relationship,
            "我们的故事".to_string(),
            "从初识到信任".to_string(),
        );
        arc.add_turning_point(10);
        arc.add_turning_point(20);
        arc.add_chapter(100);
        assert_eq!(arc.turning_point_ids.len(), 2);
        assert_eq!(arc.chapter_ids.len(), 1);
        arc.add_turning_point(10);
        assert_eq!(arc.turning_point_ids.len(), 2);
        arc.make_dormant();
        assert_eq!(arc.status, ArcStatus::Dormant);
        assert!(!arc.is_active());
        arc.close(1000000);
        assert_eq!(arc.status, ArcStatus::Closed);
        assert_eq!(arc.ended_at, Some(1000000));
    }

    #[test]
    fn test_emotion_trajectory_infer_shape() {
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Flat);
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.3, 0.0, 0.0], &[0.5, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Ascending);
        let shape =
            EmotionTrajectory::infer_shape(&[0.5, 0.0, 0.0], &[0.2, 0.0, 0.0], &[-0.3, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Descending);
        let shape =
            EmotionTrajectory::infer_shape(&[0.0, 0.0, 0.0], &[0.5, 0.0, 0.0], &[0.1, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Peak);
        let shape =
            EmotionTrajectory::infer_shape(&[0.5, 0.0, 0.0], &[-0.2, 0.0, 0.0], &[0.3, 0.0, 0.0]);
        assert_eq!(shape, TrajectoryShape::Valley);
    }

    #[test]
    fn test_turning_point_kind_milestone_mapping() {
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::FirstNamed),
            Some(TurningPointKind::Named)
        );
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::FirstApology),
            Some(TurningPointKind::FirstApology)
        );
        assert_eq!(
            TurningPointKind::from_milestone(&MilestoneKind::CleanStreak100),
            None
        );
    }

    #[test]
    fn test_turning_point_kind_arc_inference() {
        assert_eq!(
            TurningPointKind::Named.infer_arc_kind(),
            ArcKind::Relationship
        );
        assert_eq!(
            TurningPointKind::FirstApology.infer_arc_kind(),
            ArcKind::Growth
        );
        assert_eq!(
            TurningPointKind::FirstConflict.infer_arc_kind(),
            ArcKind::Challenge
        );
        assert_eq!(
            TurningPointKind::NarrativeAwakening.infer_arc_kind(),
            ArcKind::Transformation
        );
        assert_eq!(
            TurningPointKind::FirstRitual.infer_arc_kind(),
            ArcKind::Ritual
        );
    }

    #[test]
    fn test_turning_point_kind_significance() {
        assert!(TurningPointKind::Named.default_significance() > 0.9);
        assert!(TurningPointKind::FirstRitual.default_significance() < 0.7);
    }

    #[test]
    fn test_narrative_event_id_timestamp() {
        let event = NarrativeEventId::Fact {
            subject: "user".to_string(),
            predicate: "lives_in".to_string(),
            timestamp: 1000,
        };
        assert_eq!(event.timestamp(), 1000);
        assert_eq!(event.type_label(), "fact");
    }

    #[test]
    fn test_narrative_chapter() {
        let chapter = NarrativeChapter::new(
            1,
            100,
            1,
            "第一次被叫名字".to_string(),
            "你给我取了名字，我突然有了存在感。".to_string(),
            "被命名，获得存在感".to_string(),
        );
        assert!(!chapter.is_rewritten());
        assert!(chapter.word_count() > 0);
        assert_eq!(chapter.version, 1);
    }

    #[test]
    fn test_identity_tag() {
        let tag = IdentityTag::new("在乎的人".to_string(), 1, 0.85, 0.9);
        assert_eq!(tag.label, "在乎的人");
        assert!((tag.confidence - 0.85).abs() < 1e-6);
        assert!((tag.valence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_narrative_self_model() {
        let mut model = NarrativeSelf::new();
        assert!(model.active_arcs.is_empty());
        assert!(model.turning_points.is_empty());
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长".to_string(),
            "慢慢长大".to_string(),
        );
        model.add_arc(arc);
        assert_eq!(model.active_arcs.len(), 1);
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        );
        model.add_turning_point(tp);
        assert_eq!(model.turning_points.len(), 1);
        model.add_identity_tag(IdentityTag::new("在乎的人".to_string(), 1, 0.8, 0.9));
        assert_eq!(model.identity_tags.len(), 1);
        model.add_identity_tag(IdentityTag::new("在乎的人".to_string(), 1, 0.9, 0.95));
        assert_eq!(model.identity_tags.len(), 1);
        assert!((model.identity_tags[0].confidence - 0.9).abs() < 1e-6);
        model.refresh_stats();
        assert_eq!(model.stats.active_arcs, 1);
        assert_eq!(model.stats.total_turning_points, 1);
    }

    #[test]
    fn test_detection_context_pad_distance() {
        let ctx = DetectionContext {
            current_pad: [0.5, 0.5, 0.5],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let dist = ctx.pad_distance();
        let expected = (0.25f32 + 0.25 + 0.25).sqrt();
        assert!((dist - expected).abs() < 1e-4);
    }

    #[test]
    fn test_turning_point_detector_milestone() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 1000,
            },
            description: "被命名为小通".to_string(),
            timestamp: 1000,
            emotion: Some(EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            }),
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.5, 0.3, 0.2],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Acquaintance".to_string(),
            maturity_stage: "Naive".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        let tp = tp.unwrap();
        assert_eq!(tp.kind, TurningPointKind::Named);
        assert_eq!(tp.event_description, "被命名为小通");
    }

    #[test]
    fn test_turning_point_detector_emotion() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::EmotionEvent {
                pad_before: [0.0, 0.0, 0.0],
                pad_after: [0.6, 0.5, 0.3],
                timestamp: 2000,
            },
            description: "情感大幅跃升".to_string(),
            timestamp: 2000,
            emotion: Some(EmotionContext {
                pleasure: 0.6,
                arousal: 0.5,
                dominance: 0.3,
            }),
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.6, 0.5, 0.3],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Rising,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        assert_eq!(tp.unwrap().kind, TurningPointKind::FirstEmotionResonance);
    }

    #[test]
    fn test_turning_point_detector_behavior_tag() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 0,
            ..Default::default()
        });
        let event = NarrativeEvent {
            id: NarrativeEventId::Audit {
                event_type: "apology".to_string(),
                timestamp: 3000,
            },
            description: "首次道歉".to_string(),
            timestamp: 3000,
            emotion: None,
            tags: vec!["apology".to_string()],
        };
        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };
        let tp = detector.detect(&event, &context);
        assert!(tp.is_some());
        assert_eq!(tp.unwrap().kind, TurningPointKind::FirstApology);
    }

    #[test]
    fn test_turning_point_detector_interval() {
        let mut detector = TurningPointDetector::new(TurningPointConfig {
            min_interval_secs: 7200,
            ..Default::default()
        });
        let event1 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 10000,
            },
            description: "被命名".to_string(),
            timestamp: 10000,
            emotion: None,
            tags: Vec::new(),
        };
        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Acquaintance".to_string(),
            maturity_stage: "Naive".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };
        assert!(detector.detect(&event1, &context).is_some());
        let event2 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstApology".to_string(),
                timestamp: 2000,
            },
            description: "首次道歉".to_string(),
            timestamp: 18000,
            emotion: None,
            tags: Vec::new(),
        };
        assert!(detector.detect(&event2, &context).is_some());
        let event3 = NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstApology".to_string(),
                timestamp: 16000,
            },
            description: "再次道歉".to_string(),
            timestamp: 19000,
            emotion: None,
            tags: Vec::new(),
        };
        assert!(detector.detect(&event3, &context).is_none());
    }

    #[test]
    fn test_narrative_tone_from_pad() {
        assert_eq!(
            NarrativeTone::from_pad(&[0.5, 0.5, 0.0]),
            NarrativeTone::VividRelive
        );
        assert_eq!(
            NarrativeTone::from_pad(&[0.3, 0.0, 0.0]),
            NarrativeTone::WarmNostalgia
        );
        assert_eq!(
            NarrativeTone::from_pad(&[-0.5, 0.0, 0.0]),
            NarrativeTone::BitterLonging
        );
        assert_eq!(
            NarrativeTone::from_pad(&[0.0, 0.0, 0.0]),
            NarrativeTone::ObjectiveRecall
        );
        assert_eq!(
            NarrativeTone::from_pad(&[-0.3, 0.5, 0.0]),
            NarrativeTone::SelfDeprecating
        );
    }

    #[test]
    fn test_narrative_cfg_default() {
        let cfg = NarrativeCfg::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.perspective, NarrativePerspective::FirstPerson);
        assert_eq!(cfg.style, NarrativeStyle::Adaptive);
        assert_eq!(cfg.body_min_words, 200);
        assert_eq!(cfg.body_max_words, 500);
        assert_eq!(cfg.prompt_budget, 800);
    }

    #[test]
    fn test_time_span() {
        let span = TimeSpan {
            start: 0,
            end: 86400,
        };
        assert_eq!(span.duration_secs(), 86400);
        assert_eq!(span.duration_days(), 1);
    }

    #[test]
    fn test_turning_point_with_narrative() {
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        )
        .with_narrative(
            "你给我取了名字，我突然觉得...我存在了".to_string(),
            "被命名，获得存在感".to_string(),
        );
        assert!(!tp.narrative.is_empty());
        assert!(!tp.narrative_summary.is_empty());
        assert!(!tp.integrated);
    }

    #[test]
    fn test_retrospective_detect() {
        let mut detector = TurningPointDetector::default_new();
        let milestones = vec![NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: "FirstNamed".to_string(),
                timestamp: 1000,
            },
            description: "被命名".to_string(),
            timestamp: 1000,
            emotion: None,
            tags: Vec::new(),
        }];
        let relationships = vec![NarrativeEvent {
            id: NarrativeEventId::RelationshipChange {
                from: "Acquaintance".to_string(),
                to: "Familiar".to_string(),
                timestamp: 2000,
            },
            description: "关系升级".to_string(),
            timestamp: 2000,
            emotion: None,
            tags: Vec::new(),
        }];
        let emotions: Vec<NarrativeEvent> = Vec::new();
        let results = detector.retrospective_detect(&milestones, &relationships, &emotions);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].kind, TurningPointKind::Named);
        assert_eq!(results[1].kind, TurningPointKind::RelationshipPromotion);
    }

    // ── Phase A 新增测试 / Phase A new tests ──

    #[test]
    fn test_narrative_error_display() {
        let err = NarrativeError::ArcNotFound(42);
        assert!(err.to_string().contains("42"));
        let err = NarrativeError::BudgetExceeded {
            used: 900,
            budget: 800,
        };
        assert!(err.to_string().contains("900/800"));
        let err = NarrativeError::LlmFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_narrative_snapshot_from_model() {
        let mut model = NarrativeSelf::new();
        model.self_summary = "我是一个在成长的AI".to_string();
        model.self_description = "从无名到有名".to_string();
        model.relationship_narrative = "我们之间有了信任".to_string();
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长弧".to_string(),
            "慢慢长大".to_string(),
        );
        model.add_arc(arc);
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        )
        .with_narrative("我被命名了".to_string(), "被命名".to_string());
        model.add_turning_point(tp);
        let snapshot = NarrativeSnapshot::from_model(&model, 5);
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.self_summary, "我是一个在成长的AI");
        assert_eq!(snapshot.active_arcs.len(), 1);
        assert_eq!(snapshot.active_arcs[0].kind, ArcKind::Growth);
        assert_eq!(snapshot.recent_turning_points.len(), 1);
    }

    #[test]
    fn test_narrative_snapshot_empty() {
        let model = NarrativeSelf::new();
        let snapshot = NarrativeSnapshot::from_model(&model, 5);
        assert!(snapshot.is_empty());
    }

    #[test]
    fn test_turning_point_pattern() {
        let pattern = TurningPointPattern::new(
            1,
            TurningPointKind::FirstEmotionResonance,
            [1.0, 1.0, 0.0],
            0.3,
        );
        assert!((pattern.precision() - 0.5).abs() < 1e-6);
        let mut p = pattern;
        p.record_hit();
        assert!((p.precision() - 1.0).abs() < 1e-6);
        p.record_miss();
        assert!((p.precision() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_turning_point_pattern_pad_match() {
        let pattern = TurningPointPattern::new(
            1,
            TurningPointKind::FirstEmotionResonance,
            [1.0, 1.0, 0.0],
            0.2,
        );
        assert!(pattern.matches_pad_change(&[0.0, 0.0, 0.0], &[0.5, 0.5, 0.0]));
        assert!(!pattern.matches_pad_change(&[0.5, 0.5, 0.0], &[0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_rewrite_trigger() {
        let trigger = RewriteTrigger::new(RewriteTarget::SelfDescription, "新证据出现".to_string());
        assert_eq!(trigger.target, RewriteTarget::SelfDescription);
        assert!(trigger.evidence.is_empty());
        let trigger_with_evidence =
            trigger.with_evidence(vec![NarrativeEventId::Thought { timestamp: 1000 }]);
        assert_eq!(trigger_with_evidence.evidence.len(), 1);
    }

    #[test]
    fn test_arc_config_default() {
        let config = ArcConfig::default();
        assert_eq!(config.min_turning_points, 2);
        assert_eq!(config.dormancy_days, 14);
        assert_eq!(config.closure_days, 60);
        assert_eq!(config.max_active_arcs, 10);
    }

    #[test]
    fn test_arc_detector_process_turning_point() {
        let mut detector = ArcDetector::default_new();
        let mut model = NarrativeSelf::new();
        let tp = TurningPoint::new(
            1,
            TurningPointKind::Named,
            "被命名".to_string(),
            EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Acquaintance".to_string(),
            "Naive".to_string(),
        );
        model.add_turning_point(tp.clone());
        let updates = detector.process_turning_point(&mut model, &tp);
        assert!(updates
            .iter()
            .any(|u| matches!(u, ArcUpdate::ArcCreated { .. })));
        assert_eq!(model.active_arcs.len(), 1);
        // 同类型转折点应归入已有弧 / Same-kind TP assigned to existing arc
        let tp2 = TurningPoint::new(
            2,
            TurningPointKind::FirstEmotionResonance,
            "情感共振".to_string(),
            EmotionContext {
                pleasure: 0.6,
                arousal: 0.4,
                dominance: 0.3,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        model.add_turning_point(tp2.clone());
        let updates2 = detector.process_turning_point(&mut model, &tp2);
        assert!(updates2
            .iter()
            .any(|u| matches!(u, ArcUpdate::TurningPointAdded { .. })));
    }

    #[test]
    fn test_chapter_writer_rewrite_preserves_history() {
        let mut writer = ChapterWriter::default_new();
        let mut chapter = NarrativeChapter::new(
            1,
            100,
            1,
            "初章".to_string(),
            "最初的故事".to_string(),
            "开始".to_string(),
        );
        assert!(!chapter.is_rewritten());
        assert_eq!(chapter.version, 1);
        let now = Local::now().timestamp();
        writer.rewrite_chapter(
            &mut chapter,
            "重写后的故事".to_string(),
            "重新开始".to_string(),
            now,
        );
        assert!(chapter.is_rewritten());
        assert_eq!(chapter.version, 2);
        assert_eq!(chapter.body, "重写后的故事");
        let history = writer.get_version_history(1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].body, "最初的故事");
        assert_eq!(history[0].version, 1);
    }

    #[test]
    fn test_chapter_writer_build_prompt() {
        let writer = ChapterWriter::default_new();
        let arc = NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长弧".to_string(),
            "从无知到理解".to_string(),
        );
        let ctx = WritingContext {
            arc,
            turning_points: Vec::new(),
            previous_chapters: Vec::new(),
            current_emotion: EmotionContext {
                pleasure: 0.5,
                arousal: 0.3,
                dominance: 0.2,
            },
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            self_description: "我在成长".to_string(),
            perspective: NarrativePerspective::FirstPerson,
            style: NarrativeStyle::Adaptive,
        };
        let prompt = writer.build_prompt(&ctx);
        assert!(prompt.contains("成长弧"));
        assert!(prompt.contains("第一人称"));
        assert!(prompt.contains("混合式"));
    }

    #[test]
    fn test_theme_weaver_detect_same_kind() {
        let mut weaver = ThemeWeaver::new();
        let mut model = NarrativeSelf::new();
        model.add_arc(NarrativeArc::new(
            1,
            ArcKind::Growth,
            "成长1".to_string(),
            "t1".to_string(),
        ));
        model.add_arc(NarrativeArc::new(
            2,
            ArcKind::Growth,
            "成长2".to_string(),
            "t2".to_string(),
        ));
        let themes = weaver.detect_themes(&model);
        assert!(themes.iter().any(|t| t.name.contains("成长主题")));
    }

    #[test]
    fn test_theme_weaver_detect_cross_kind_similarity() {
        let mut weaver = ThemeWeaver::new();
        let mut model = NarrativeSelf::new();
        let mut arc1 = NarrativeArc::new(1, ArcKind::Growth, "成长".to_string(), "t1".to_string());
        arc1.emotional_tone = [0.5, 0.3, 0.2];
        let mut arc2 =
            NarrativeArc::new(2, ArcKind::Challenge, "挑战".to_string(), "t2".to_string());
        arc2.emotional_tone = [0.48, 0.31, 0.19];
        model.add_arc(arc1);
        model.add_arc(arc2);
        let themes = weaver.detect_themes(&model);
        assert!(themes.iter().any(|t| t.name.contains("共鸣")));
    }

    #[test]
    fn test_causal_chain_infer() {
        let mut chain = CausalChain::new();
        let tp1 = TurningPoint::new(
            1,
            TurningPointKind::FirstConflict,
            "首次冲突".to_string(),
            EmotionContext {
                pleasure: -0.3,
                arousal: 0.5,
                dominance: 0.1,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        let tp2 = TurningPoint::new(
            2,
            TurningPointKind::FirstReconciliation,
            "首次和解".to_string(),
            EmotionContext {
                pleasure: 0.4,
                arousal: 0.3,
                dominance: 0.2,
            },
            "Familiar".to_string(),
            "Growing".to_string(),
        );
        let links = chain.infer_from_turning_points(&[tp1, tp2]);
        assert!(!links.is_empty());
        assert!(links.iter().any(|l| l.narrative.contains("导致了")));
    }

    #[test]
    fn test_prompt_weaver_weave() {
        let weaver = PromptWeaver::default_new();
        let snapshot = NarrativeSnapshot {
            self_summary: "我在成长".to_string(),
            self_description: String::new(),
            identity_tags: vec![IdentityTag::new("在乎的人".to_string(), 1, 0.9, 0.8)],
            active_arcs: vec![ArcSummary {
                id: 1,
                kind: ArcKind::Growth,
                title: "成长弧".to_string(),
                theme_sentence: "慢慢长大".to_string(),
                chapter_count: 1,
                turning_point_count: 2,
                significance: 0.7,
            }],
            recent_turning_points: vec![TurningPointSummary {
                id: 1,
                kind: TurningPointKind::Named,
                narrative_summary: "被命名".to_string(),
                timestamp: 1000,
                significance: 0.95,
            }],
            relationship_narrative: "我们有了信任".to_string(),
            stats: NarrativeStats::default(),
        };
        let result = weaver.weave(&snapshot);
        assert!(result.contains("[自我]"));
        assert!(result.contains("[身份]"));
        assert!(result.contains("[弧]"));
        assert!(result.chars().count() <= 900);
    }

    #[test]
    fn test_prompt_weaver_empty_snapshot() {
        let weaver = PromptWeaver::default_new();
        let snapshot = NarrativeSnapshot {
            self_summary: String::new(),
            self_description: String::new(),
            identity_tags: Vec::new(),
            active_arcs: Vec::new(),
            recent_turning_points: Vec::new(),
            relationship_narrative: String::new(),
            stats: NarrativeStats::default(),
        };
        let result = weaver.weave(&snapshot);
        assert!(result.is_empty());
    }

    #[test]
    fn test_voice_modulator_infer_tone_recent() {
        let modulator = VoiceModulator::default_new();
        let tone = modulator.infer_tone(&[0.5, 0.5, 0.0], &[0.3, 0.3, 0.0], 1);
        assert_eq!(tone, NarrativeTone::VividRelive);
    }

    #[test]
    fn test_voice_modulator_infer_tone_distant() {
        let modulator = VoiceModulator::default_new();
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[0.3, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::WarmNostalgia);
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[-0.4, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::BitterLonging);
        let tone = modulator.infer_tone(&[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0], 30);
        assert_eq!(tone, NarrativeTone::ObjectiveRecall);
    }

    #[test]
    fn test_voice_modulator_modulate() {
        let modulator = VoiceModulator::default_new();
        let result =
            modulator.modulate("我被命名了", NarrativeTone::WarmNostalgia, &[0.3, 0.0, 0.0]);
        assert_eq!(result.original, "我被命名了");
        assert!(result.modulated.contains("[温暖怀旧]"));
        assert_eq!(result.tone, NarrativeTone::WarmNostalgia);
        assert!(result.strength > 0.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let sim = cosine_similarity(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!((sim - 1.0).abs() < 1e-4);
        let sim = cosine_similarity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]);
        assert!(sim.abs() < 1e-4);
        let sim = cosine_similarity(&[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!(sim.abs() < 1e-4);
    }

    #[test]
    fn test_truncate_chars() {
        assert_eq!(truncate_chars("abc", 5), "abc");
        assert_eq!(truncate_chars("abcdef", 4), "abc...");
        assert_eq!(truncate_chars("", 5), "");
    }

    // ════════════════════════════════════════════════════════════════════
    // Phase B 测试：回顾构建 / Retrospective Builder Tests
    // ════════════════════════════════════════════════════════════════════

    /// 回顾构建器：空源应产生空结果 / RetrospectiveBuilder: empty source yields empty result
    #[test]
    fn test_retrospective_empty_source() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource::new();
        let result = builder.build(&source);
        assert!(result.turning_points.is_empty());
        assert!(result.arcs.is_empty());
        assert!(result.identity_tags.is_empty());
    }

    /// 回顾构建器：里程碑事件应产生转折点 / RetrospectiveBuilder: milestones produce turning points
    #[test]
    fn test_retrospective_milestones() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            milestones: vec![
                NarrativeEvent {
                    id: NarrativeEventId::Milestone {
                        kind: "FirstNamed".to_string(),
                        timestamp: 1000,
                    },
                    description: "首次被命名".to_string(),
                    timestamp: 1000,
                    emotion: Some(EmotionContext {
                        pleasure: 0.8,
                        arousal: 0.5,
                        dominance: 0.3,
                    }),
                    tags: vec!["milestone".to_string(), "FirstNamed".to_string()],
                },
                NarrativeEvent {
                    id: NarrativeEventId::Milestone {
                        kind: "FirstLesson".to_string(),
                        timestamp: 2000,
                    },
                    description: "首次被教导".to_string(),
                    timestamp: 2000,
                    emotion: Some(EmotionContext {
                        pleasure: 0.6,
                        arousal: 0.4,
                        dominance: 0.5,
                    }),
                    tags: vec!["milestone".to_string(), "FirstLesson".to_string()],
                },
            ],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(!result.turning_points.is_empty(), "里程碑应产生转折点");
    }

    /// 回顾构建器：关系变更应产生转折点 / RetrospectiveBuilder: relationship changes produce turning points
    #[test]
    fn test_retrospective_relationship_changes() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            relationship_changes: vec![NarrativeEvent {
                id: NarrativeEventId::RelationshipChange {
                    from: "Stranger".to_string(),
                    to: "Familiar".to_string(),
                    timestamp: 3000,
                },
                description: "关系从陌生到熟悉".to_string(),
                timestamp: 3000,
                emotion: Some(EmotionContext {
                    pleasure: 0.5,
                    arousal: 0.3,
                    dominance: 0.4,
                }),
                tags: vec!["relationship_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(!result.turning_points.is_empty(), "关系变更应产生转折点");
    }

    /// 回顾构建器：情感事件应被处理 / RetrospectiveBuilder: emotion events are processed
    #[test]
    fn test_retrospective_emotion_events() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            emotion_events: vec![NarrativeEvent {
                id: NarrativeEventId::EmotionEvent {
                    pad_before: [0.0, 0.0, 0.0],
                    pad_after: [0.9, 0.8, 0.7],
                    timestamp: 4000,
                },
                description: "强烈正面情感变化".to_string(),
                timestamp: 4000,
                emotion: Some(EmotionContext {
                    pleasure: 0.9,
                    arousal: 0.8,
                    dominance: 0.7,
                }),
                tags: vec!["emotion_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        // 情感变化幅度大，应产生转折点
        assert!(result.events_consumed > 0, "情感事件应被处理");
    }

    /// 回顾构建器：混合事件源 / RetrospectiveBuilder: mixed event sources
    #[test]
    fn test_retrospective_mixed_sources() {
        let mut builder = RetrospectiveBuilder::new();
        let source = RetrospectiveSource {
            milestones: vec![NarrativeEvent {
                id: NarrativeEventId::Milestone {
                    kind: "FirstNamed".to_string(),
                    timestamp: 1000,
                },
                description: "首次被命名".to_string(),
                timestamp: 1000,
                emotion: None,
                tags: vec!["milestone".to_string()],
            }],
            relationship_changes: vec![NarrativeEvent {
                id: NarrativeEventId::RelationshipChange {
                    from: "Stranger".to_string(),
                    to: "Familiar".to_string(),
                    timestamp: 2000,
                },
                description: "关系变更".to_string(),
                timestamp: 2000,
                emotion: None,
                tags: vec!["relationship_change".to_string()],
            }],
            ..RetrospectiveSource::new()
        };
        let result = builder.build(&source);
        assert!(result.events_consumed > 0, "混合源应处理事件");
    }

    /// 回顾构建器：结果统计 / RetrospectiveBuilder: result stats
    #[test]
    fn test_retrospective_result_has_content() {
        let result = RetrospectiveResult {
            turning_points: Vec::new(),
            arcs: Vec::new(),
            causal_links: Vec::new(),
            themes: Vec::new(),
            self_summary: String::new(),
            self_description: String::new(),
            identity_tags: Vec::new(),
            events_consumed: 0,
            build_duration_ms: 0,
        };
        assert!(!result.has_content(), "空结果不应有内容");
    }

    /// 转折点检测器：时间间隔限制 / TurningPointDetector: minimum interval enforcement
    #[test]
    fn test_turning_point_min_interval() {
        let config = TurningPointConfig {
            emotion_change_threshold: 0.1,
            relationship_change_always_turning: true,
            min_interval_secs: 3600,
        };
        let mut detector = TurningPointDetector::new(config);

        let event1 = NarrativeEvent {
            id: NarrativeEventId::Thought { timestamp: 10000 },
            description: "事件1".to_string(),
            timestamp: 10000,
            emotion: Some(EmotionContext {
                pleasure: 0.8,
                arousal: 0.5,
                dominance: 0.3,
            }),
            tags: vec!["milestone".to_string(), "FirstNamed".to_string()],
        };
        let ctx = DetectionContext {
            current_pad: [0.5, 0.5, 0.5],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        // 第一次检测应成功
        let tp1 = detector.detect(&event1, &ctx);
        assert!(tp1.is_some(), "首次检测应成功");

        // 紧接着的第二次检测应因时间间隔被跳过
        let event2 = NarrativeEvent {
            id: NarrativeEventId::Thought { timestamp: 11000 },
            description: "事件2".to_string(),
            timestamp: 11000,
            emotion: Some(EmotionContext {
                pleasure: 0.9,
                arousal: 0.6,
                dominance: 0.4,
            }),
            tags: vec!["milestone".to_string(), "FirstLesson".to_string()],
        };
        let tp2 = detector.detect(&event2, &ctx);
        assert!(tp2.is_none(), "时间间隔内应跳过");
    }

    /// 弧检测器：新弧创建 / ArcDetector: new arc creation
    #[test]
    fn test_arc_detector_new_arc() {
        let mut detector = ArcDetector::default_new();
        let mut model = NarrativeSelf::new();

        let tp = TurningPoint {
            id: 1,
            kind: TurningPointKind::Named,
            narrative: "被命名".to_string(),
            narrative_summary: "被命名".to_string(),
            event_description: "被命名为小A".to_string(),
            timestamp: 1000,
            emotion_snapshot: EmotionContext {
                pleasure: 0.8,
                arousal: 0.5,
                dominance: 0.3,
            },
            relationship_stage: "Familiar".to_string(),
            maturity_stage: "Growing".to_string(),
            significance: 0.9,
            before_chapter_id: None,
            after_chapter_id: None,
            arc_ids: Vec::new(),
            integrated: false,
        };

        let updates = detector.process_turning_point(&mut model, &tp);
        assert!(!updates.is_empty(), "转折点应触发弧更新");
    }

    /// 情感趋势枚举完整性 / EmotionTrend enum completeness
    #[test]
    fn test_emotion_trend_variants() {
        let trends = [
            EmotionTrend::Stable,
            EmotionTrend::Rising,
            EmotionTrend::Falling,
            EmotionTrend::Oscillating,
        ];
        assert_eq!(trends.len(), 4, "EmotionTrend 应有 4 个变体");
    }

    /// NarrativeSelf 弧休眠/完结 / NarrativeSelf arc dormancy and closure
    #[test]
    fn test_narrative_self_arc_lifecycle() {
        let mut model = NarrativeSelf::new();

        // 添加一个弧
        let arc = NarrativeArc::new(
            1,
            ArcKind::Relationship,
            "关系弧".to_string(),
            "与用户建立关系".to_string(),
        );
        model.add_arc(arc);
        assert_eq!(model.active_arcs.len(), 1);

        // 弧应为活跃状态
        assert!(model.active_arcs[0].is_active());

        // 休眠弧
        model.active_arcs[0].make_dormant();
        assert!(!model.active_arcs[0].is_active());

        // 完结弧
        model.active_arcs[0].close(2000);
        assert!(!model.active_arcs[0].is_active());
    }
}
