use super::*;
use crate::maturity::EmotionContext;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// TurningPointConfig — 转折点检测配置 / Turning Point Detection Config
// ════════════════════════════════════════════════════════════════════

/// 转折点检测配置 / Turning point detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPointConfig {
    /// 情感变化阈值 / Emotion change threshold (PAD Euclidean distance)
    pub emotion_change_threshold: f32,
    /// 关系阶段变更始终是转折点 / Relationship change is always a turning point
    pub relationship_change_always_turning: bool,
    /// 最小时间间隔（秒）/ Minimum interval between turning points (seconds)
    pub min_interval_secs: i64,
}

impl Default for TurningPointConfig {
    fn default() -> Self {
        Self {
            emotion_change_threshold: 0.4,
            relationship_change_always_turning: true,
            min_interval_secs: 3600,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TurningPointDetector — 转折点检测器 / Turning Point Detector
// ════════════════════════════════════════════════════════════════════

/// 转折点检测器 — 从原始事件流中识别转折点
/// Turning point detector — identify turning points from raw event stream
pub struct TurningPointDetector {
    /// 检测配置 / Detection config
    pub config: TurningPointConfig,
    /// 下一个转折点 ID / Next turning point ID
    next_id: u64,
    /// 最近检测时间 / Last detection time
    last_detection_at: i64,
}

impl TurningPointDetector {
    /// 创建检测器 / Create detector
    pub fn new(config: TurningPointConfig) -> Self {
        Self {
            config,
            next_id: 1,
            last_detection_at: 0,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(TurningPointConfig::default())
    }

    /// 检测转折点 — 从原始事件中识别 / Detect turning point from raw event
    ///
    /// 检测策略（按优先级）/ Detection strategy (by priority):
    /// 1. 硬性转折：MilestoneKind 中定义的事件 / Hard turning: events defined in MilestoneKind
    /// 2. 关系转折：RelationshipStage 变更 / Relationship turning: stage change
    /// 3. 情感转折：PAD 欧氏距离超过阈值 / Emotion turning: PAD distance exceeds threshold
    /// 4. 行为转折：从被动到主动等模式变更 / Behavior turning: pattern change
    pub fn detect(
        &mut self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPoint> {
        let now = event.timestamp;

        // 时间间隔检查 / Minimum interval check
        if now - self.last_detection_at < self.config.min_interval_secs {
            return None;
        }

        // 策略 1：里程碑硬性转折 / Strategy 1: Milestone hard turning
        if let Some(kind) = self.detect_milestone(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 2：关系转折 / Strategy 2: Relationship turning
        if let Some(kind) = self.detect_relationship(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 3：情感转折 / Strategy 3: Emotion turning
        if let Some(kind) = self.detect_emotion(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        // 策略 4：行为转折 / Strategy 4: Behavior turning
        if let Some(kind) = self.detect_behavior(event, context) {
            return Some(self.create_turning_point(kind, event, context));
        }

        None
    }

    /// 里程碑检测 / Milestone detection
    fn detect_milestone(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        match &event.id {
            NarrativeEventId::Milestone { kind, .. } => {
                let tp_kind = match kind.as_str() {
                    "FirstNamed" => Some(TurningPointKind::Named),
                    "FirstSelfCorrection" => Some(TurningPointKind::FirstSelfCorrection),
                    "FirstProactiveCare" => Some(TurningPointKind::FirstProactiveCare),
                    "FirstApology" => Some(TurningPointKind::FirstApology),
                    "FirstEmotionResonance" => Some(TurningPointKind::FirstEmotionResonance),
                    "StageTransition" => Some(TurningPointKind::RelationshipPromotion),
                    "FirstInnerThought" => Some(TurningPointKind::FirstIndependentThought),
                    "FirstWisdom" => Some(TurningPointKind::FirstWisdom),
                    _ => None,
                };
                if let Some(k) = tp_kind {
                    if !context.has_recent_kind(k) {
                        return Some(k);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 关系转折检测 / Relationship turning detection
    fn detect_relationship(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        if !self.config.relationship_change_always_turning {
            return None;
        }
        match &event.id {
            NarrativeEventId::RelationshipChange { .. } => {
                if !context.has_recent_kind(TurningPointKind::RelationshipPromotion) {
                    Some(TurningPointKind::RelationshipPromotion)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 情感转折检测 / Emotion turning detection
    fn detect_emotion(
        &self,
        _event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        let distance = context.pad_distance();
        if distance < self.config.emotion_change_threshold {
            return None;
        }

        let pleasure_delta = context.current_pad[0] - context.previous_pad[0];
        let arousal_delta = context.current_pad[1] - context.previous_pad[1];

        // 高唤醒 + 正愉悦跃升 → 情感共振 / High arousal + pleasure surge → emotion resonance
        if pleasure_delta > 0.3
            && arousal_delta > 0.2
            && !context.has_recent_kind(TurningPointKind::FirstEmotionResonance)
        {
            return Some(TurningPointKind::FirstEmotionResonance);
        }

        // 高唤醒 + 负愉悦 → 心疼 / High arousal + negative pleasure → heartache
        if pleasure_delta < -0.3
            && context.current_pad[1] > 0.5
            && !context.has_recent_kind(TurningPointKind::FirstHeartache)
        {
            return Some(TurningPointKind::FirstHeartache);
        }

        // 负愉悦 + 低唤醒 → 想念 / Negative pleasure + low arousal → longing
        if context.current_pad[0] < -0.2
            && context.current_pad[1] < 0.3
            && !context.has_recent_kind(TurningPointKind::FirstLonging)
        {
            return Some(TurningPointKind::FirstLonging);
        }

        None
    }

    /// 行为转折检测 / Behavior turning detection
    fn detect_behavior(
        &self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPointKind> {
        if event.tags.contains(&"apology".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstApology)
        {
            return Some(TurningPointKind::FirstApology);
        }
        if event.tags.contains(&"proactive_care".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstProactiveCare)
        {
            return Some(TurningPointKind::FirstProactiveCare);
        }
        if event.tags.contains(&"self_correction".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstSelfCorrection)
        {
            return Some(TurningPointKind::FirstSelfCorrection);
        }
        if event.tags.contains(&"conflict".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstConflict)
        {
            return Some(TurningPointKind::FirstConflict);
        }
        if event.tags.contains(&"reconciliation".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstReconciliation)
        {
            return Some(TurningPointKind::FirstReconciliation);
        }
        if event.tags.contains(&"vulnerability".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstVulnerability)
        {
            return Some(TurningPointKind::FirstVulnerability);
        }
        if event.tags.contains(&"disagreement".to_string())
            && !context.has_recent_kind(TurningPointKind::FirstDisagreement)
        {
            return Some(TurningPointKind::FirstDisagreement);
        }
        None
    }

    /// 创建转折点 / Create a turning point
    fn create_turning_point(
        &mut self,
        kind: TurningPointKind,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> TurningPoint {
        let emotion_snapshot = event.emotion.clone().unwrap_or(EmotionContext {
            pleasure: context.current_pad[0],
            arousal: context.current_pad[1],
            dominance: context.current_pad[2],
        });

        let tp = TurningPoint::new(
            self.next_id,
            kind,
            event.description.clone(),
            emotion_snapshot,
            context.relationship_stage.clone(),
            context.maturity_stage.clone(),
        );
        self.next_id += 1;
        self.last_detection_at = event.timestamp;
        tp
    }

    /// 批量回溯检测 — 从历史事件中补漏 / Retrospective detection from historical events
    ///
    /// 用于系统首次启动时，从已有数据中回溯构建初始转折点集合
    /// Used at first startup to build initial turning points from existing data
    pub fn retrospective_detect(
        &mut self,
        milestone_events: &[NarrativeEvent],
        relationship_events: &[NarrativeEvent],
        emotion_events: &[NarrativeEvent],
    ) -> Vec<TurningPoint> {
        let mut results = Vec::new();

        // 合并并按时间排序 / Merge and sort by time
        let mut all_events: Vec<&NarrativeEvent> = Vec::new();
        all_events.extend(milestone_events.iter());
        all_events.extend(relationship_events.iter());
        all_events.extend(emotion_events.iter());
        all_events.sort_by_key(|e| e.timestamp);

        let context = DetectionContext {
            current_pad: [0.0, 0.0, 0.0],
            previous_pad: [0.0, 0.0, 0.0],
            relationship_stage: String::new(),
            maturity_stage: String::new(),
            recent_emotion_trend: EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        // 重置间隔限制以允许回溯 / Reset interval limit for retrospective
        let saved_interval = self.config.min_interval_secs;
        self.config.min_interval_secs = 0;

        for event in &all_events {
            if let Some(tp) = self.detect(event, &context) {
                results.push(tp);
            }
        }

        self.config.min_interval_secs = saved_interval;
        results
    }
}
