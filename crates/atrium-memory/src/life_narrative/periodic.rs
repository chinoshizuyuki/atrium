use super::*;
use crate::maturity::EmotionContext;

// ════════════════════════════════════════════════════════════════════
// NarrativeEventRecorder — 叙事事件记录器 / Narrative Event Recorder
// ════════════════════════════════════════════════════════════════════

/// 叙事事件记录器 — 处理消息管线中的叙事事件检测与记录
/// Narrative event recorder — handles narrative event detection and recording
/// in the message processing pipeline.
///
/// 对应 CoreService 管线中的：
/// - Step 0.9: 叙事事件检测（TurningPointDetector.detect）
/// - Step 9.5: 叙事事件记录（更新情感轨迹 + 标记待处理）
pub struct NarrativeEventRecorder {
    /// 转折点检测器 / Turning point detector
    detector: TurningPointDetector,
    /// 弧检测器 / Arc detector
    arc_detector: ArcDetector,
    /// 是否有未处理的转折点 / Whether there are unprocessed turning points
    has_pending: bool,
}

impl Default for NarrativeEventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl NarrativeEventRecorder {
    /// 创建默认记录器 / Create default recorder
    pub fn new() -> Self {
        Self {
            detector: TurningPointDetector::default_new(),
            arc_detector: ArcDetector::default_new(),
            has_pending: false,
        }
    }

    /// 使用配置创建记录器 / Create recorder with config
    pub fn with_config(detector_config: TurningPointConfig, arc_config: ArcConfig) -> Self {
        Self {
            detector: TurningPointDetector::new(detector_config),
            arc_detector: ArcDetector::new(arc_config),
            has_pending: false,
        }
    }

    /// Step 0.9: 叙事事件检测 — 在消息处理管线中检测转折点
    /// Step 0.9: Narrative event detection — detect turning points in message pipeline.
    ///
    /// 返回检测到的转折点（如有），并标记待处理状态。
    /// Returns detected turning point (if any) and marks pending state.
    pub fn detect_event(
        &mut self,
        event: &NarrativeEvent,
        context: &DetectionContext,
    ) -> Option<TurningPoint> {
        let tp = self.detector.detect(event, context);
        if tp.is_some() {
            self.has_pending = true;
        }
        tp
    }

    /// Step 9.5: 叙事事件记录 — 将转折点集成到叙事模型
    /// Step 9.5: Narrative event recording — integrate turning point into narrative model.
    ///
    /// 处理流程：
    /// 1. 将转折点添加到模型
    /// 2. 通过 ArcDetector 处理，可能创建/更新弧
    /// 3. 返回弧更新列表
    pub fn record_event(&mut self, model: &mut NarrativeSelf, tp: &TurningPoint) -> Vec<ArcUpdate> {
        model.add_turning_point(tp.clone());
        let updates = self.arc_detector.process_turning_point(model, tp);
        self.has_pending = false;
        updates
    }

    /// 是否有待处理的转折点 / Whether there are pending turning points
    pub fn has_pending(&self) -> bool {
        self.has_pending
    }

    /// 从成长里程碑构建叙事事件 / Build narrative event from growth milestone
    pub fn milestone_to_event(
        kind: &str,
        description: &str,
        timestamp: i64,
        emotion: Option<EmotionContext>,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::Milestone {
                kind: kind.to_string(),
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion,
            tags: Vec::new(),
        }
    }

    /// 从关系变更构建叙事事件 / Build narrative event from relationship change
    pub fn relationship_to_event(
        from: &str,
        to: &str,
        description: &str,
        timestamp: i64,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::RelationshipChange {
                from: from.to_string(),
                to: to.to_string(),
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion: None,
            tags: Vec::new(),
        }
    }

    /// 从情感变化构建叙事事件 / Build narrative event from emotion change
    pub fn emotion_to_event(
        pad_before: [f32; 3],
        pad_after: [f32; 3],
        description: &str,
        timestamp: i64,
    ) -> NarrativeEvent {
        NarrativeEvent {
            id: NarrativeEventId::EmotionEvent {
                pad_before,
                pad_after,
                timestamp,
            },
            description: description.to_string(),
            timestamp,
            emotion: Some(EmotionContext {
                pleasure: pad_after[0],
                arousal: pad_after[1],
                dominance: pad_after[2],
            }),
            tags: Vec::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// NarrativePeriodicTask — 叙事周期任务 / Narrative Periodic Tasks
// ════════════════════════════════════════════════════════════════════

/// 叙事日终报告 / Narrative daily report
#[derive(Debug, Clone, Default)]
pub struct NarrativeDailyReport {
    /// 今日检测到的遗漏转折点数 / Missed turning points detected today
    pub missed_turning_points: usize,
    /// 自我描述是否更新 / Whether self description was updated
    pub self_description_updated: bool,
    /// 今日叙事摘要 / Today's narrative summary
    pub daily_summary: String,
    /// 是否触发了旧章节重写 / Whether old chapter rewrite was triggered
    pub rewrite_triggered: bool,
}

/// 叙事周终报告 / Narrative weekly report
#[derive(Debug, Clone, Default)]
pub struct NarrativeWeeklyReport {
    /// 全面弧检测新增弧数 / New arcs from full arc detection
    pub new_arcs: usize,
    /// 跨弧主题数 / Cross-arc theme count
    pub cross_arc_themes: usize,
    /// 自我描述是否重写 / Whether self description was rewritten
    pub self_description_rewritten: bool,
    /// 身份标签更新数 / Identity tag updates
    pub identity_tag_updates: usize,
    /// 叙事快照是否保存 / Whether narrative snapshot was saved
    pub snapshot_saved: bool,
}

/// 叙事周期任务执行器 — 实现 tick / daily / weekly 三级周期任务
/// Narrative periodic task executor — implements tick / daily / weekly three-level periodic tasks.
pub struct NarrativePeriodicTask {
    // 弧检测器 — 周/月级弧演化检测时将被 consume
    // Arc detector — will be consumed during weekly/monthly arc evolution detection
    #[allow(dead_code)]
    arc_detector: ArcDetector,
    /// 主题编织器 / Theme weaver
    theme_weaver: ThemeWeaver,
    /// 因果链 / Causal chain
    causal_chain: CausalChain,
    /// 上次日终执行时间 / Last daily execution time
    last_daily_at: i64,
    /// 上次周终执行时间 / Last weekly execution time
    last_weekly_at: i64,
}
impl Default for NarrativePeriodicTask {
    fn default() -> Self {
        Self::new()
    }
}

impl NarrativePeriodicTask {
    /// 创建默认执行器 / Create default executor
    pub fn new() -> Self {
        Self {
            arc_detector: ArcDetector::default_new(),
            theme_weaver: ThemeWeaver::new(),
            causal_chain: CausalChain::new(),
            last_daily_at: 0,
            last_weekly_at: 0,
        }
    }

    /// tick_narrative: 叙事周期评估（每 1000 tick ≈ 10s）
    /// tick_narrative: Narrative periodic evaluation (every 1000 tick ≈ 10s).
    ///
    /// 执行：
    /// - 检查未处理的转折点 → 触发章节撰写
    /// - 检查弧状态 → 休眠/完结判定
    /// - 情感轨迹更新 → 活跃章节的情感轨迹追加
    pub fn tick(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
        dormancy_secs: i64,
        closure_secs: i64,
    ) {
        // 弧休眠/完结检测 / Arc dormancy/closure detection
        for arc in &mut model.active_arcs {
            // 计算弧的最后活动时间 / Calculate arc's last activity time
            let last_activity = model
                .turning_points
                .iter()
                .filter(|tp| arc.turning_point_ids.contains(&tp.id))
                .map(|tp| tp.timestamp)
                .max()
                .unwrap_or(arc.started_at);

            let inactive_secs = now_epoch_secs - last_activity;

            if inactive_secs > closure_secs && arc.is_active() {
                arc.close(now_epoch_secs);
            } else if inactive_secs > dormancy_secs && arc.is_active() {
                arc.make_dormant();
            }
        }

        // 将已休眠超时的弧移到 closed / Move dormant-expired arcs to closed
        let should_close: Vec<u64> = model
            .active_arcs
            .iter()
            .filter(|a| a.status == ArcStatus::Dormant)
            .filter_map(|a| {
                let last_activity = model
                    .turning_points
                    .iter()
                    .filter(|tp| a.turning_point_ids.contains(&tp.id))
                    .map(|tp| tp.timestamp)
                    .max()
                    .unwrap_or(a.started_at);
                if now_epoch_secs - last_activity > closure_secs {
                    Some(a.id)
                } else {
                    None
                }
            })
            .collect();

        for arc_id in should_close {
            if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                arc.close(now_epoch_secs);
            }
        }

        // 标记未处理转折点为已处理 / Mark unprocessed turning points as processed
        for tp in &mut model.turning_points {
            if !tp.integrated {
                tp.integrated = true;
            }
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();
    }

    /// daily_narrative: 叙事日终任务（每天一次）
    /// daily_narrative: Narrative daily task (once per day).
    ///
    /// 执行：
    /// - 从今日事件中检测遗漏的转折点
    /// - 更新自我描述（如果今日有重要事件）
    /// - 生成今日叙事摘要
    /// - 检查是否需要重写旧章节
    pub fn daily(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
        today_turning_points: &[TurningPoint],
    ) -> NarrativeDailyReport {
        let mut report = NarrativeDailyReport::default();

        // 检测遗漏的转折点 / Detect missed turning points
        let unprocessed = model.unintegrated_turning_points();
        report.missed_turning_points = unprocessed.len();

        // 更新自我描述 / Update self description
        if !today_turning_points.is_empty() {
            let significant: Vec<_> = today_turning_points
                .iter()
                .filter(|tp| tp.significance > 0.7)
                .collect();
            if !significant.is_empty() {
                report.self_description_updated = true;
                let summaries: Vec<&str> = significant
                    .iter()
                    .map(|tp| tp.narrative_summary.as_str())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !summaries.is_empty() {
                    report.daily_summary = summaries.join("；");
                }
            }
        }

        // 检查是否需要重写旧章节 / Check if old chapter rewrite is needed
        if now_epoch_secs - model.last_rewrite_at > 86400 * 30 {
            report.rewrite_triggered = true;
            model.last_rewrite_at = now_epoch_secs;
        }

        self.last_daily_at = now_epoch_secs;
        report
    }

    /// weekly_narrative: 叙事周终任务（每周一次）
    /// weekly_narrative: Narrative weekly task (once per week).
    ///
    /// 执行：
    /// - 全面弧检测（回溯一周事件）
    /// - 跨弧主题识别
    /// - 自我描述重写
    /// - 叙事快照保存
    /// - 身份标签更新
    pub fn weekly(
        &mut self,
        model: &mut NarrativeSelf,
        now_epoch_secs: i64,
    ) -> NarrativeWeeklyReport {
        let mut report = NarrativeWeeklyReport::default();

        // 跨弧主题识别 / Cross-arc theme detection
        let themes = self.theme_weaver.detect_themes(model);
        report.cross_arc_themes = themes.len();

        // 因果链推断 / Causal chain inference
        let _links = self
            .causal_chain
            .infer_from_turning_points(&model.turning_points);

        // 自我描述重写 / Self description rewrite
        // 仅在有足够素材时重写 / Only rewrite when there's enough material
        if model.turning_points.len() >= 3 && !model.active_arcs.is_empty() {
            report.self_description_rewritten = true;

            // 从弧标题构建新的自我描述 / Build new self description from arc titles
            let arc_titles: Vec<&str> = model
                .active_arcs
                .iter()
                .take(5)
                .map(|a| a.title.as_str())
                .collect();
            if !arc_titles.is_empty() {
                model.self_description = format!("我的故事围绕着{}展开", arc_titles.join("、"));
            }
        }

        // 身份标签更新 / Identity tag updates
        // 从最近转折点中提取新标签 / Extract new tags from recent turning points
        // 先收集待添加的标签，避免同时不可变借用和可变借用 model
        // Collect tags to add first to avoid simultaneous immutable and mutable borrows
        let new_tags: Vec<IdentityTag> = model
            .turning_points
            .iter()
            .rev()
            .take(5)
            .filter(|tp| tp.significance > 0.8)
            .filter_map(|tp| {
                let label = format!("{}经历者", tp.kind.label_zh());
                if model.identity_tags.iter().any(|t| t.label == label) {
                    None
                } else {
                    Some(IdentityTag::new(
                        label,
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ))
                }
            })
            .collect();
        for tag in new_tags {
            model.add_identity_tag(tag);
            report.identity_tag_updates += 1;
        }

        // 叙事快照保存标记 / Narrative snapshot save marker
        report.snapshot_saved = true;
        model.refresh_stats();

        self.last_weekly_at = now_epoch_secs;
        report
    }

    /// 是否应该执行日终任务 / Whether daily task should execute
    pub fn should_run_daily(&self, now_epoch_secs: i64) -> bool {
        // 至少间隔 20 小时 / At least 20 hours apart
        now_epoch_secs - self.last_daily_at >= 86400 - 14400
    }

    /// 是否应该执行周终任务 / Whether weekly task should execute
    pub fn should_run_weekly(&self, now_epoch_secs: i64) -> bool {
        // 至少间隔 6 天 / At least 6 days apart
        now_epoch_secs - self.last_weekly_at >= 86400 * 6
    }
}
