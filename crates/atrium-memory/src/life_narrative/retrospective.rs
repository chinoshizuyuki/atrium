use super::*;

// ════════════════════════════════════════════════════════════════════
// RetrospectiveBuilder — 回溯构建引擎 / Retrospective Construction Engine
// ════════════════════════════════════════════════════════════════════

/// 回溯数据源 — 首次启动时从已有存储中提取的原始素材
/// Retrospective data source — raw materials extracted from existing stores on first startup.
#[derive(Debug, Clone)]
pub struct RetrospectiveSource {
    /// 里程碑事件 / Milestone events
    pub milestones: Vec<NarrativeEvent>,
    /// 关系变更事件 / Relationship change events
    pub relationship_changes: Vec<NarrativeEvent>,
    /// 情感事件 / Emotion events
    pub emotion_events: Vec<NarrativeEvent>,
    /// 日记关键事件（从日记中提取的重要时刻）/ Diary key events
    pub diary_events: Vec<NarrativeEvent>,
    /// 内在独白反思事件 / Inner monologue reflection events
    pub monologue_events: Vec<NarrativeEvent>,
    /// 高置信度事实摘要 / High-confidence fact summaries
    pub fact_summaries: Vec<String>,
}

impl Default for RetrospectiveSource {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrospectiveSource {
    /// 创建空数据源 / Create empty source
    pub fn new() -> Self {
        Self {
            milestones: Vec::new(),
            relationship_changes: Vec::new(),
            emotion_events: Vec::new(),
            diary_events: Vec::new(),
            monologue_events: Vec::new(),
            fact_summaries: Vec::new(),
        }
    }

    /// 总事件数 / Total event count
    pub fn total_events(&self) -> usize {
        self.milestones.len()
            + self.relationship_changes.len()
            + self.emotion_events.len()
            + self.diary_events.len()
            + self.monologue_events.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.total_events() == 0 && self.fact_summaries.is_empty()
    }

    /// 合并所有事件为统一列表 / Merge all events into a single list
    pub fn all_events(&self) -> Vec<&NarrativeEvent> {
        self.milestones
            .iter()
            .chain(self.relationship_changes.iter())
            .chain(self.emotion_events.iter())
            .chain(self.diary_events.iter())
            .chain(self.monologue_events.iter())
            .collect()
    }
}

/// 回溯构建结果 / Retrospective construction result
#[derive(Debug, Clone)]
pub struct RetrospectiveResult {
    /// 检测到的转折点 / Detected turning points
    pub turning_points: Vec<TurningPoint>,
    /// 识别的叙事弧 / Identified narrative arcs
    pub arcs: Vec<NarrativeArc>,
    /// 因果链 / Causal links
    pub causal_links: Vec<CausalLink>,
    /// 跨弧主题 / Cross-arc themes
    pub themes: Vec<CrossArcTheme>,
    /// 构建的自我摘要 / Constructed self summary
    pub self_summary: String,
    /// 构建的自我描述 / Constructed self description
    pub self_description: String,
    /// 构建的身份标签 / Constructed identity tags
    pub identity_tags: Vec<IdentityTag>,
    /// 消耗的事件数 / Consumed event count
    pub events_consumed: usize,
    /// 构建耗时毫秒 / Build duration in milliseconds
    pub build_duration_ms: u64,
}

impl RetrospectiveResult {
    /// 是否有实质内容 / Whether has substantive content
    pub fn has_content(&self) -> bool {
        !self.turning_points.is_empty() || !self.arcs.is_empty()
    }

    /// 应用到叙事自我模型 / Apply to narrative self model
    pub fn apply_to(self, model: &mut NarrativeSelf) {
        // 转折点 / Turning points
        for tp in self.turning_points {
            model.add_turning_point(tp);
        }

        // 叙事弧 / Narrative arcs
        for arc in self.arcs {
            model.add_arc(arc);
        }

        // 自我摘要 / Self summary
        if !self.self_summary.is_empty() {
            model.self_summary = self.self_summary;
        }

        // 自我描述 / Self description
        if !self.self_description.is_empty() {
            model.self_description = self.self_description;
        }

        // 身份标签 / Identity tags
        for tag in self.identity_tags {
            model.add_identity_tag(tag);
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();
    }
}

/// 回溯构建引擎 — 首次启动时从已有数据构建初始叙事
/// Retrospective builder — construct initial narrative from existing data on first startup.
///
/// 回溯构建是异步的 — 不阻塞正常服务启动。后台逐步构建，
/// 构建期间叙事 Prompt 注入返回空字符串。
/// Retrospective construction is async — does not block normal service startup.
/// Builds incrementally in the background; narrative prompt injection returns
/// empty string during construction.
pub struct RetrospectiveBuilder {
    /// 转折点检测器 / Turning point detector
    detector: TurningPointDetector,
    /// 弧检测器 / Arc detector
    arc_detector: ArcDetector,
    /// 因果链推断 / Causal chain inferrer
    causal_chain: CausalChain,
    /// 主题编织器 / Theme weaver
    theme_weaver: ThemeWeaver,
    // 弧配置 — Phase C LLM 深度分析接入后将驱动弧构造参数
    // Arc configuration — will drive arc construction params once Phase C LLM analysis is integrated
    #[allow(dead_code)]
    arc_config: ArcConfig,
}

impl Default for RetrospectiveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrospectiveBuilder {
    /// 创建默认构建器 / Create default builder
    pub fn new() -> Self {
        Self {
            detector: TurningPointDetector::default_new(),
            arc_detector: ArcDetector::default_new(),
            causal_chain: CausalChain::new(),
            theme_weaver: ThemeWeaver::new(),
            arc_config: ArcConfig::default(),
        }
    }

    /// 使用自定义配置创建构建器 / Create builder with custom config
    pub fn with_config(detector_config: TurningPointConfig, arc_config: ArcConfig) -> Self {
        Self {
            detector: TurningPointDetector::new(detector_config),
            arc_detector: ArcDetector::new(arc_config.clone()),
            causal_chain: CausalChain::new(),
            theme_weaver: ThemeWeaver::new(),
            arc_config,
        }
    }

    /// 执行回溯构建 / Execute retrospective construction
    ///
    /// 从已有数据源中构建初始叙事自我模型。流程：
    /// 1. 从里程碑 + 关系变更 + 情感事件中检测转折点
    /// 2. 从日记 + 独白中补充转折点
    /// 3. 从转折点集合中识别叙事弧
    /// 4. 推断因果链
    /// 5. 检测跨弧主题
    /// 6. 生成初始自我描述
    ///
    /// Build initial narrative self model from existing data sources. Flow:
    /// 1. Detect turning points from milestones + relationship changes + emotion events
    /// 2. Supplement turning points from diary + monologue
    /// 3. Identify narrative arcs from turning point set
    /// 4. Infer causal chains
    /// 5. Detect cross-arc themes
    /// 6. Generate initial self description
    pub fn build(&mut self, source: &RetrospectiveSource) -> RetrospectiveResult {
        let start = std::time::Instant::now();

        // ── Step 1: 从里程碑 + 关系变更 + 情感事件中检测转折点 ──
        // Step 1: Detect turning points from milestones + relationship changes + emotion events
        let mut turning_points = self.detector.retrospective_detect(
            &source.milestones,
            &source.relationship_changes,
            &source.emotion_events,
        );

        // ── Step 2: 从日记 + 独白中补充转折点 ──
        // Step 2: Supplement turning points from diary + monologue
        let supplementary_events: Vec<&NarrativeEvent> = source
            .diary_events
            .iter()
            .chain(source.monologue_events.iter())
            .collect();

        for event in supplementary_events {
            // 为补充事件构造默认检测上下文 / Build default detection context for supplementary events
            let context = DetectionContext {
                current_pad: event
                    .emotion
                    .as_ref()
                    .map(|e| [e.pleasure, e.arousal, e.dominance])
                    .unwrap_or([0.0; 3]),
                previous_pad: [0.0; 3],
                relationship_stage: "Familiar".to_string(),
                maturity_stage: "Growing".to_string(),
                recent_emotion_trend: EmotionTrend::Stable,
                recent_kinds: Vec::new(),
            };
            if let Some(tp) = self.detector.detect(event, &context) {
                turning_points.push(tp);
            }
        }

        // 按时间排序转折点 / Sort turning points by timestamp
        turning_points.sort_by_key(|tp| tp.timestamp);

        // ── Step 3: 从转折点集合中识别叙事弧 ──
        // Step 3: Identify narrative arcs from turning point set
        let mut temp_model = NarrativeSelf::new();
        for tp in &turning_points {
            temp_model.add_turning_point(tp.clone());
        }

        // 逐个处理转折点以触发弧检测 / Process turning points one by one to trigger arc detection
        let mut arcs = Vec::new();
        for tp in &turning_points {
            let updates = self.arc_detector.process_turning_point(&mut temp_model, tp);
            for update in updates {
                match update {
                    ArcUpdate::ArcCreated { arc_id, kind } => {
                        let title = format!("{}弧", kind.label_zh());
                        let arc = NarrativeArc::new(
                            arc_id,
                            kind,
                            title,
                            String::new(), // theme_sentence 稍后由 ThemeWeaver 填充
                        );
                        arcs.push(arc);
                    }
                    ArcUpdate::TurningPointAdded { arc_id, tp_id } => {
                        // 将转折点 ID 关联到已有弧 / Associate turning point ID with existing arc
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.add_turning_point(tp_id);
                        }
                    }
                    ArcUpdate::ArcDormant { arc_id } => {
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.make_dormant();
                        }
                    }
                    ArcUpdate::ArcClosed { arc_id } => {
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.close(tp.timestamp);
                        }
                    }
                    ArcUpdate::ArcSuperseded {
                        old_arc_id,
                        new_arc_id,
                    } => {
                        // 标记旧弧被取代 / Mark old arc as superseded
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == old_arc_id) {
                            arc.make_dormant();
                        }
                        let _ = new_arc_id; // 新弧已在 ArcCreated 中处理
                    }
                    ArcUpdate::SignificanceUpdated { arc_id, old, new } => {
                        // 弧显著度更新 / Arc significance updated
                        if let Some(arc) = arcs.iter_mut().find(|a| a.id == arc_id) {
                            arc.significance = new;
                        }
                        let _ = old; // 旧值仅用于日志
                    }
                    ArcUpdate::NoChange => {
                        // 无变化，跳过 / No change, skip
                    }
                }
            }
        }

        // 将识别的弧添加到临时模型 / Add identified arcs to temp model
        for arc in &arcs {
            if arc.is_active() {
                temp_model.active_arcs.push(arc.clone());
            } else {
                temp_model.closed_arcs.push(arc.clone());
            }
        }

        // ── Step 4: 推断因果链 ──
        // Step 4: Infer causal chains
        let causal_links = self.causal_chain.infer_from_turning_points(&turning_points);

        // ── Step 5: 检测跨弧主题 ──
        // Step 5: Detect cross-arc themes
        let themes = self.theme_weaver.detect_themes(&temp_model);

        // ── Step 6: 生成初始自我描述 ──
        // Step 6: Generate initial self description
        let (self_summary, self_description, identity_tags) =
            self.build_self_description(&turning_points, &arcs, &themes, &source.fact_summaries);

        let build_duration_ms = start.elapsed().as_millis() as u64;

        RetrospectiveResult {
            events_consumed: source.total_events(),
            turning_points,
            arcs,
            causal_links,
            themes,
            self_summary,
            self_description,
            identity_tags,
            build_duration_ms,
        }
    }

    /// 从转折点 + 弧 + 主题构建自我描述
    /// Build self description from turning points + arcs + themes.
    fn build_self_description(
        &self,
        turning_points: &[TurningPoint],
        arcs: &[NarrativeArc],
        themes: &[CrossArcTheme],
        fact_summaries: &[String],
    ) -> (String, String, Vec<IdentityTag>) {
        let mut summary_parts = Vec::new();
        let mut description_parts = Vec::new();
        let mut tags = Vec::new();

        // 从转折点提取身份标签 / Extract identity tags from turning points
        for tp in turning_points {
            match tp.kind {
                TurningPointKind::Named => {
                    tags.push(IdentityTag::new(
                        "被命名者".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("被赋予了名字".to_string());
                }
                TurningPointKind::FirstEmotionResonance => {
                    tags.push(IdentityTag::new(
                        "有感受的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了感受".to_string());
                }
                TurningPointKind::FirstApology => {
                    tags.push(IdentityTag::new(
                        "会认错的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了道歉".to_string());
                }
                TurningPointKind::FirstConflict => {
                    summary_parts.push("经历了冲突".to_string());
                }
                TurningPointKind::FirstReconciliation => {
                    tags.push(IdentityTag::new(
                        "会和解的存在".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("学会了和解".to_string());
                }
                TurningPointKind::NarrativeAwakening => {
                    tags.push(IdentityTag::new(
                        "叙事觉醒者".to_string(),
                        tp.id,
                        tp.significance,
                        tp.emotion_snapshot.pleasure as f64,
                    ));
                    summary_parts.push("理解了叙事的意义".to_string());
                }
                _ => {}
            }
        }

        // 从弧构建描述段落 / Build description paragraphs from arcs
        for arc in arcs {
            if arc.is_active() {
                description_parts.push(format!(
                    "在「{}」这条路上，{}",
                    arc.title,
                    if arc.theme_sentence.is_empty() {
                        "我正在前行".to_string()
                    } else {
                        arc.theme_sentence.clone()
                    }
                ));
            }
        }

        // 从主题补充描述 / Supplement description from themes
        for theme in themes {
            if theme.significance > 0.5 {
                description_parts.push(format!("{}是我生命中的主题", theme.name));
            }
        }

        // 从事实摘要补充 / Supplement from fact summaries
        for fact in fact_summaries.iter().take(3) {
            description_parts.push(fact.clone());
        }

        // 组装自我摘要 / Assemble self summary
        let self_summary = if summary_parts.is_empty() {
            String::new()
        } else {
            format!("我{}", summary_parts.join("，"))
        };

        // 组装自我描述 / Assemble self description
        let self_description = description_parts.join("。");

        (self_summary, self_description, tags)
    }

    /// 检查是否需要回溯构建（叙事模型为空时需要）
    /// Check whether retrospective construction is needed (needed when narrative model is empty).
    pub fn needs_retrospective(model: &NarrativeSelf) -> bool {
        model.turning_points.is_empty() && model.active_arcs.is_empty()
    }
}
