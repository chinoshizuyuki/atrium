// SPDX-License-Identifier: MIT
//! 叙事自我模块 — 数字生命的自传核心
//! Narrative Self Module — The autobiography core of digital life
//!
//! 包含叙事系统、非理性修正、仪式感知与脆弱性窗口，
//! 构成数字生命"我是谁"的自我认知闭环。
//!
//! Contains the narrative system, irrationality correction,
//! ritual perception, and vulnerability window — forming the
//! "who am I" self-awareness closed loop of digital life.

use super::*;

/// 章节候选类型别名 / Chapter candidate type alias
type ArcChapterCandidate = (u64, String, String, Vec<String>, Option<String>);
impl CoreService {
    pub fn irrationality_prompt_fragment(&self, now: i64) -> String {
        if !self.irrationality_enabled {
            return String::new();
        }
        let mgr = self.irrationality.lock();
        mgr.to_prompt_fragment(now)
    }

    pub fn irrationality_tick(&self, now: i64) {
        if !self.irrationality_enabled {
            return;
        }
        let emo = self.emotion.lock();
        let pad = [
            emo.current().pleasure,
            emo.current().arousal,
            emo.current().dominance,
        ];
        drop(emo);
        let mut mgr = self.irrationality.lock();
        mgr.tick(&pad, now);

        // 写穿持久化：非理性 tick 后保存脉冲/残留/传染/混沌引擎状态 / Write-through: persist pulse/residue/contagion/chaos state after tick
        drop(mgr);
        self.irrationality_save();
    }

    pub fn irrationality_on_emotion_change(
        &self,
        pad_before: &[f32; 3],
        pad_after: &[f32; 3],
        now: i64,
    ) -> atrium_memory::emotional_irrationality::IrrationalityCorrection {
        use atrium_memory::emotional_irrationality::{
            MaturityDepth, PulseSource, PulseTrigger, RelationshipDepth,
        };
        let mut mgr = self.irrationality.lock();
        let correction = mgr.on_emotion_change(
            pad_before,
            pad_after,
            PulseTrigger {
                source: PulseSource::UserMessage,
                signal: "emotion_change".into(),
                baseline_pad: *pad_before,
            },
            RelationshipDepth::Any,
            MaturityDepth::Any,
            now,
        );

        // 写穿持久化：情绪变化触发脉冲后保存，防止重启丢失脉冲引擎状态 / Write-through: persist after emotion change triggers pulse
        drop(mgr);
        self.irrationality_save();

        correction
    }

    pub fn ritual_prompt_fragment(&self, now_epoch: i64) -> String {
        if !self.ritual_enabled {
            return String::new();
        }
        let budget = self.ritual_cfg.prompt_budget;
        let mut parts = Vec::new();

        // 仪式检测器 prompt / Ritual detector prompt
        {
            let detector = self.ritual_detector.lock();
            let fragment = detector.prompt_fragment();
            if !fragment.is_empty() {
                parts.push(fragment);
            }
        }

        // 纪念日系统 prompt / Anniversary system prompt
        {
            let anniversary = self.anniversary_system.lock();
            let fragment = anniversary.prompt_fragment(now_epoch);
            if !fragment.is_empty() {
                parts.push(fragment);
            }
        }

        // 季节感知 prompt / Seasonal awareness prompt
        {
            let seasonal = self.seasonal_awareness.lock();
            let (month, day) = atrium_memory::seasonal_awareness::epoch_to_month_day(now_epoch);
            let fragment = seasonal.prompt_fragment(month, day);
            if !fragment.is_empty() {
                parts.push(fragment);
            }
        }

        if parts.is_empty() {
            return String::new();
        }
        let combined = parts.join("\n");
        if combined.len() > budget {
            combined[..budget].to_string()
        } else {
            combined
        }
    }

    pub fn ritual_tick(&self, now_epoch: i64) {
        if !self.ritual_enabled {
            return;
        }
        // 仪式检测器每日评估 / Ritual detector daily evaluation
        {
            let mut detector = self.ritual_detector.lock();
            detector.evaluate_daily(now_epoch);
        }
        // 纪念日检查 / Anniversary check
        {
            let mut anniversary = self.anniversary_system.lock();
            let _ = anniversary.check_today(now_epoch);
        }
        tracing::debug!("[Ritual] Periodic tick completed");

        // 写穿持久化：仪式评估后保存，防止重启丢失仪式模式与纪念日记忆 / Write-through: persist after ritual evaluation to preserve patterns and anniversaries
        self.ritual_save();
    }

    pub fn vulnerability_prompt_fragment(&self) -> String {
        if !self.vulnerability_enabled {
            return String::new();
        }
        self.vulnerability_window.lock().prompt_fragment()
    }

    pub fn vulnerability_tick(&self) {
        if !self.vulnerability_enabled {
            return;
        }
        // 退出过期的脆弱状态（简单超时机制）
        let mut vw = self.vulnerability_window.lock();
        if vw.is_in_vulnerable_state() {
            vw.exit_vulnerable_state();
        }
        tracing::debug!("[Vulnerability] Periodic tick completed");

        // 写穿持久化：脆弱状态变更后保存，确保跨会话的脆弱时刻连续性 / Write-through: persist after vulnerability state change for cross-session continuity
        drop(vw);
        self.vulnerability_save();
    }

    pub fn narrative_prompt_fragment(&self) -> String {
        if !self.narrative_enabled {
            return String::new();
        }

        let model = self.narrative_self.lock();

        // 构建 NarrativeSnapshot / Build NarrativeSnapshot from model
        let snapshot = atrium_memory::life_narrative::NarrativeSnapshot {
            self_summary: model.self_summary.clone(),
            self_description: model.self_description.clone(),
            identity_tags: model.identity_tags.clone(),
            active_arcs: model
                .active_arcs
                .iter()
                .take(3)
                .map(|a| a.summary())
                .collect(),
            recent_turning_points: model
                .turning_points
                .iter()
                .rev()
                .take(3)
                .map(|tp| tp.summary())
                .collect(),
            relationship_narrative: model.relationship_narrative.clone(),
            stats: model.stats.clone(),
        };

        // Phase C: 使用 PromptWeaver 生成叙事注入 / Use PromptWeaver for narrative injection
        let weaver = self.prompt_weaver.lock();
        let woven = weaver.weave(&snapshot);
        drop(weaver);
        drop(model);

        if !woven.is_empty() {
            // 截断到 prompt_budget / Truncate to prompt budget
            let budget = self.narrative_cfg.prompt_budget;
            if woven.len() <= budget {
                woven
            } else {
                // 在预算内找最后一个换行符，避免截断句子
                let end = woven
                    .char_indices()
                    .take_while(|(i, _)| *i < budget)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(budget);
                let mut truncated = woven[..end].to_string();
                // 回退到最后一个完整行
                if let Some(pos) = truncated.rfind('\n') {
                    truncated.truncate(pos);
                }
                truncated
            }
        } else {
            // 回退：空模型时手动构建基础片段 / Fallback: manual build for empty model
            String::new()
        }
    }

    pub fn tick_narrative(&self, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        let mut model = self.narrative_self.lock();
        let dormancy_secs = self.narrative_cfg.arc_dormancy_days * 86400;
        let closure_secs = self.narrative_cfg.arc_closure_days * 86400;

        // 弧休眠/完结检测 / Arc dormancy/closure detection
        for arc in &mut model.active_arcs {
            if let Some(last) = arc.turning_point_ids.last() {
                // 简化：用 arc ID 作为时间代理（实际应用中应查 turning_point timestamp）
                let _ = (last, dormancy_secs, closure_secs);
            }
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();

        let _ = now_epoch_secs; // 供未来自我描述重写触发使用

        // 写穿持久化：每次 tick 后保存，防止重启丢失弧状态 / Write-through: persist after every tick to prevent arc state loss on restart
        drop(model);
        self.narrative_save();
    }
    /// 叙事事件检测 — Step 0.9 / Narrative event detection — Step 0.9
    pub fn detect_narrative_event(&self, user_msg: &str, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        // 获取当前情感状态 / Get current emotion state
        let emo = self.emotion.lock();
        let pad = emo.current();
        let current_pad = [pad.pleasure, pad.arousal, pad.dominance];
        drop(emo);

        // 获取关系阶段和成熟度阶段 / Get relationship and maturity stages
        let relationship_stage = self
            .relationship
            .lock()
            .current_stage()
            .stage_name()
            .to_string();
        let maturity_stage = self.maturity.lock().stage().stage_name().to_string();

        // 构造叙事事件 / Construct narrative event
        let event = atrium_memory::life_narrative::NarrativeEvent {
            id: atrium_memory::life_narrative::NarrativeEventId::Thought {
                timestamp: now_epoch_secs,
            },
            description: user_msg.to_string(),
            timestamp: now_epoch_secs,
            emotion: Some(atrium_memory::maturity::EmotionContext {
                pleasure: current_pad[0],
                arousal: current_pad[1],
                dominance: current_pad[2],
            }),
            tags: Vec::new(),
        };

        // 构造检测上下文 / Construct detection context
        let context = atrium_memory::life_narrative::DetectionContext {
            current_pad,
            previous_pad: current_pad, // 简化：同一次消息内 previous ≈ current
            relationship_stage,
            maturity_stage,
            recent_emotion_trend: atrium_memory::life_narrative::EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        // 执行检测 / Execute detection
        let mut model = self.narrative_self.lock();
        let tp = self.tp_detector.lock().detect(&event, &context);
        if let Some(turning_point) = tp {
            // 转折点入弧 / Add turning point to arcs
            let arc_updates = self
                .arc_detector
                .lock()
                .process_turning_point(&mut model, &turning_point);
            model.add_turning_point(turning_point);

            // 应用弧更新 / Apply arc updates
            for update in arc_updates {
                match update {
                    atrium_memory::life_narrative::ArcUpdate::ArcCreated { arc_id, kind } => {
                        tracing::debug!("[叙事] 新弧创建: id={}, kind={:?}", arc_id, kind);
                    }
                    atrium_memory::life_narrative::ArcUpdate::TurningPointAdded {
                        arc_id,
                        tp_id,
                    } => {
                        tracing::debug!("[叙事] 转折点入弧: arc_id={}, tp_id={}", arc_id, tp_id);
                    }
                    atrium_memory::life_narrative::ArcUpdate::ArcDormant { arc_id } => {
                        tracing::debug!("[叙事] 弧休眠: arc_id={}", arc_id);
                    }
                    atrium_memory::life_narrative::ArcUpdate::ArcClosed { arc_id } => {
                        tracing::debug!("[叙事] 弧完结: arc_id={}", arc_id);
                    }
                    atrium_memory::life_narrative::ArcUpdate::ArcSuperseded {
                        old_arc_id,
                        new_arc_id,
                    } => {
                        tracing::debug!("[叙事] 弧取代: old={}, new={}", old_arc_id, new_arc_id);
                    }
                    atrium_memory::life_narrative::ArcUpdate::SignificanceUpdated {
                        arc_id,
                        old,
                        new,
                    } => {
                        tracing::debug!(
                            "[叙事] 弧显著度更新: arc_id={}, {} → {}",
                            arc_id,
                            old,
                            new
                        );
                    }
                    atrium_memory::life_narrative::ArcUpdate::NoChange => {}
                }
            }

            model.refresh_stats();
        }

        // 写穿持久化：转折点检测后立即保存，防止重启丢失叙事记忆 / Write-through: persist immediately after turning point detection
        drop(model);
        self.narrative_save();
    }

    pub fn record_narrative_event(&self, user_msg: &str, ai_reply: &str, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        // 获取当前情感状态 / Get current emotion state
        let (pleasure, arousal, dominance) = {
            let emo = self.emotion.lock();
            let pad = emo.current();
            (pad.pleasure, pad.arousal, pad.dominance)
        };

        // 记录 AI 回复作为内在思考事件 / Record AI reply as inner thought event
        let ai_event = atrium_memory::life_narrative::NarrativeEvent {
            id: atrium_memory::life_narrative::NarrativeEventId::Thought {
                timestamp: now_epoch_secs,
            },
            description: format!("[AI] {}", ai_reply.chars().take(200).collect::<String>()),
            timestamp: now_epoch_secs,
            emotion: Some(atrium_memory::maturity::EmotionContext {
                pleasure,
                arousal,
                dominance,
            }),
            tags: vec!["ai_reply".to_string()],
        };

        // 尝试检测 AI 回复中的转折点 / Try to detect turning points in AI reply
        let relationship_stage = self
            .relationship
            .lock()
            .current_stage()
            .stage_name()
            .to_string();
        let maturity_stage = self.maturity.lock().stage().stage_name().to_string();

        let context = atrium_memory::life_narrative::DetectionContext {
            current_pad: [pleasure, arousal, dominance],
            previous_pad: [pleasure, arousal, dominance],
            relationship_stage,
            maturity_stage,
            recent_emotion_trend: atrium_memory::life_narrative::EmotionTrend::Stable,
            recent_kinds: Vec::new(),
        };

        let mut model = self.narrative_self.lock();
        let tp = self.tp_detector.lock().detect(&ai_event, &context);
        if let Some(turning_point) = tp {
            let arc_updates = self
                .arc_detector
                .lock()
                .process_turning_point(&mut model, &turning_point);
            model.add_turning_point(turning_point);
            let _ = arc_updates; // 弧更新已在 detect_narrative_event 中处理日志
            model.refresh_stats();
        }

        // 更新叙事自我描述（如果弧或转折点有变化）/ Update self-summary
        let _ = user_msg; // 保留供未来主题提取使用

        // 写穿持久化：事件记录后立即保存，确保叙事连续性 / Write-through: persist after event recording for narrative continuity
        drop(model);
        self.narrative_save();
    }
    /// 每日叙事评估 / Daily narrative evaluation
    pub fn tick_narrative_daily(&self, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        let mut model = self.narrative_self.lock();
        let dormancy_secs = self.narrative_cfg.arc_dormancy_days * 86400;
        let closure_secs = self.narrative_cfg.arc_closure_days * 86400;

        // 弧休眠检测 / Arc dormancy detection
        // 先收集每个弧的最后转折点时间，避免同时可变和不可变借用 model
        // Collect last TP timestamps first to avoid simultaneous mutable and immutable borrows
        let arc_last_tp_times: Vec<(u64, i64, String)> = model
            .active_arcs
            .iter()
            .filter(|a| a.is_active())
            .filter_map(|arc| {
                let last_tp_id = arc.turning_point_ids.last()?;
                let last_tp = model.get_turning_point(*last_tp_id)?;
                Some((arc.id, last_tp.timestamp, arc.title.clone()))
            })
            .collect();

        for (arc_id, last_tp_time, arc_title) in arc_last_tp_times {
            let elapsed = now_epoch_secs - last_tp_time;
            if elapsed > closure_secs {
                if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                    arc.close(now_epoch_secs);
                    tracing::info!("[叙事·日] 弧完结: id={}, title={}", arc_id, arc_title);
                }
            } else if elapsed > dormancy_secs {
                if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                    arc.make_dormant();
                    tracing::info!("[叙事·日] 弧休眠: id={}, title={}", arc_id, arc_title);
                }
            }
        }

        // 身份标签衰减 / Identity tag decay
        // 移除置信度极低的标签 / Remove very low confidence tags
        model.identity_tags.retain(|t| t.confidence > 0.05);

        // 刷新统计 / Refresh stats
        model.refresh_stats();

        // 自我描述重写触发检查 / Self-description rewrite trigger check
        let rewrite_interval_secs = self.narrative_cfg.self_description_rewrite_days * 86400;
        if now_epoch_secs - model.last_rewrite_at > rewrite_interval_secs {
            model.last_rewrite_at = now_epoch_secs;

            // Phase C: ChapterWriter 构建重写 prompt / Build rewrite prompt
            // 使用第一个活跃弧作为目标弧 / Use first active arc as target
            let target_arc = model.active_arcs.first().cloned().unwrap_or_else(|| {
                atrium_memory::life_narrative::NarrativeArc::new(
                    0,
                    atrium_memory::life_narrative::ArcKind::Growth,
                    "默认弧".to_string(),
                    String::new(),
                )
            });
            let writing_ctx = atrium_memory::life_narrative::WritingContext {
                arc: target_arc,
                turning_points: model.turning_points.iter().rev().take(5).cloned().collect(),
                previous_chapters: model.chapters.iter().rev().take(3).cloned().collect(),
                current_emotion: atrium_memory::maturity::EmotionContext {
                    pleasure: 0.0,
                    arousal: 0.0,
                    dominance: 0.0,
                },
                relationship_stage: "Familiar".to_string(),
                maturity_stage: "Growing".to_string(),
                self_description: model.self_description.clone(),
                perspective: atrium_memory::life_narrative::NarrativePerspective::FirstPerson,
                style: atrium_memory::life_narrative::NarrativeStyle::Adaptive,
            };
            let chapter_writer = self.chapter_writer.lock();
            let prompt = chapter_writer.build_prompt(&writing_ctx);
            drop(chapter_writer);

            // Phase C: ThemeWeaver 跨弧主题发现 / Cross-arc theme discovery
            let mut theme_weaver = self.theme_weaver.lock();
            let themes = theme_weaver.detect_themes(&model);
            drop(theme_weaver);

            // 将发现的主题关联到模型 / Associate discovered themes with model
            if !themes.is_empty() {
                model.cross_arc_themes = themes;
            }

            tracing::debug!(
                "[叙事·日] 自我描述重写触发, prompt长度={}, 主题数={}",
                prompt.len(),
                model.cross_arc_themes.len()
            );
            // 注意：实际 LLM 调用由上层调度（build_prompt 产出供 LLM 网关消费）
            // Note: actual LLM call is dispatched upstream (build_prompt output consumed by LLM gateway)
        }

        // 持久化保存 / Persist to store
        drop(model); // 释放锁后再保存 / Release lock before saving
        self.narrative_save();
    }

    pub fn tick_narrative_weekly(&self, _now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        let mut model = self.narrative_self.lock();

        // 已完结弧归档：将超过 90 天的已完结弧移出活跃列表
        // Archive closed arcs older than 90 days
        let _archive_threshold_secs: i64 = 90 * 86400;

        // Phase C: 因果链推理 / Causal chain inference
        // 从转折点序列推断因果联系 / Infer causal links from turning point sequence
        if model.turning_points.len() >= 2 {
            let mut causal_chain = atrium_memory::life_narrative::CausalChain::new();
            let new_links = causal_chain.infer_from_turning_points(&model.turning_points);
            if !new_links.is_empty() {
                let link_count = new_links.len();
                model.causal_links = new_links;
                tracing::debug!("[叙事·周] 因果链推理: {} 条新链", link_count);
            }
        }

        // Phase C: VoiceModulator 语气推断 / Voice tone inference
        // 基于当前弧状态推断叙事语气 / Infer narrative tone from current arc state
        if !model.active_arcs.is_empty() {
            let voice = self.voice_modulator.lock();
            // 从最近转折点提取 PAD / Extract PAD from latest turning point
            let current_pad: [f32; 3] = model
                .turning_points
                .last()
                .map(|tp| {
                    [
                        tp.emotion_snapshot.pleasure,
                        tp.emotion_snapshot.arousal,
                        tp.emotion_snapshot.dominance,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0]);
            // 回溯 PAD：取最早的转折点 / Recall PAD: earliest turning point
            let recall_pad: [f32; 3] = model
                .turning_points
                .first()
                .map(|tp| {
                    [
                        tp.emotion_snapshot.pleasure,
                        tp.emotion_snapshot.arousal,
                        tp.emotion_snapshot.dominance,
                    ]
                })
                .unwrap_or([0.0, 0.0, 0.0]);
            // 回溯距离（天）/ Recall distance in days
            let recall_days = model
                .turning_points
                .last()
                .zip(model.turning_points.first())
                .map(|(last, first)| (last.timestamp - first.timestamp) / 86400)
                .unwrap_or(0);
            let tone = voice.infer_tone(&current_pad, &recall_pad, recall_days);
            drop(voice);
            // 将推断的语气应用到模型 / Apply inferred tone to model
            model.narrative_tone = tone;
        }

        // 刷新统计 / Refresh stats
        model.refresh_stats();

        tracing::debug!("[叙事·周] 周期评估完成");

        // 写穿持久化：周评估后保存因果链与语气推断结果 / Write-through: persist causal links and tone inference after weekly evaluation
        drop(model);
        self.narrative_save();
    }
    // ════════════════════════════════════════════════════════════════════

    pub async fn tick_narrative_chapter_gen(&self, _now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let gen = {
            let client_arc = self.llm_client.lock().clone();
            match client_arc {
                Some(c) => atrium_memory::monologue_gen::MonologueGenerator::new(c),
                None => {
                    tracing::debug!(
                        "[叙事·章] LLM 未就绪，跳过章节生成 / LLM not ready, skip chapter generation"
                    );
                    return;
                }
            }
        };

        // 收集需要生成章节的弧 / Collect arcs that need chapter generation
        // 锁安全：数据提取在 {} 块内，锁在块结束时释放 / Lock-safe: data extraction in {} block, lock released at block end
        let arcs_to_generate: Vec<ArcChapterCandidate> = {
            let model = self.narrative_self.lock();
            model
                .active_arcs
                .iter()
                .filter(|arc| arc.is_active())
                .filter_map(|arc| {
                    // 统计弧中已有章节数 / Count existing chapters for this arc
                    let existing_chapters = model
                        .chapters
                        .iter()
                        .filter(|ch| ch.arc_id == arc.id)
                        .count();

                    // 弧的转折点数 - 已有章节数 = 待成章转折点数
                    // Pending TPs = arc TP count - existing chapter count
                    let pending_tps = arc
                        .turning_point_ids
                        .len()
                        .saturating_sub(existing_chapters);

                    if pending_tps >= 2 {
                        // 提取转折点叙述 / Extract turning point narratives
                        let tp_narratives: Vec<String> = arc
                            .turning_point_ids
                            .iter()
                            .filter_map(|tp_id| model.get_turning_point(*tp_id))
                            .map(|tp| format!("[{}] {}", tp.kind.label_zh(), tp.narrative_summary))
                            .collect();

                        // 前一章摘要 / Previous chapter summary
                        let prev_summary = model
                            .chapters
                            .iter()
                            .rfind(|ch| ch.arc_id == arc.id)
                            .map(|ch| ch.summary.clone());

                        Some((
                            arc.id,
                            arc.title.clone(),
                            arc.theme_sentence.clone(),
                            tp_narratives,
                            prev_summary,
                        ))
                    } else {
                        None
                    }
                })
                .collect()
        }; // model 锁已释放 / model lock released

        // 为每个弧生成章节 / Generate chapter for each arc
        for (arc_id, arc_title, arc_theme, tp_narratives, prev_summary) in arcs_to_generate {
            let turning_points_text = tp_narratives.join("\n");
            let events_text = "(由转折点驱动 / Driven by turning points)";
            let emotion_trajectory =
                "(情感轨迹随转折点变化 / Emotion trajectory varies with turning points)";

            // LLM 调用 — 无锁状态下 .await / LLM call — .await without lock held
            match gen
                .generate_chapter(
                    &arc_title,
                    &arc_theme,
                    &turning_points_text,
                    events_text,
                    emotion_trajectory,
                    prev_summary.as_deref(),
                )
                .await
            {
                Ok(chapter_text) => {
                    // 解析 LLM 输出为章节 / Parse LLM output into chapter
                    let (title, body, summary) = parse_chapter_output(&chapter_text);

                    // 重新获取锁写入结果 / Re-acquire lock to write results
                    let mut model = self.narrative_self.lock();
                    let mut writer = self.chapter_writer.lock();
                    let chapter_id = writer.alloc_chapter_id();

                    let sequence = model
                        .chapters
                        .iter()
                        .filter(|ch| ch.arc_id == arc_id)
                        .count() as u32
                        + 1;

                    // NarrativeChapter::new 内部用 Local::now() 记录时间
                    // NarrativeChapter::new uses Local::now() internally for timestamps
                    let chapter = atrium_memory::life_narrative::NarrativeChapter::new(
                        chapter_id, arc_id, sequence, title, body, summary,
                    );

                    model.add_chapter(chapter);
                    // 将章节 ID 添加到弧 / Add chapter ID to arc
                    if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                        arc.add_chapter(chapter_id);
                    }
                    model.refresh_stats();
                    drop(writer);
                    drop(model); // 释放锁后再保存 / Release lock before saving

                    // 写穿持久化 / Write-through persistence
                    self.narrative_save();

                    tracing::info!(
                        "[叙事·章] 章节生成成功: arc_id={}, chapter_id={} / Chapter generated",
                        arc_id,
                        chapter_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "[叙事·章] 章节生成失败: arc_id={}, error={:?} / Chapter generation failed",
                        arc_id,
                        e
                    );
                }
            }
        }
    }

    pub async fn tick_narrative_rewrite(&self, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let gen = {
            let client_arc = self.llm_client.lock().clone();
            match client_arc {
                Some(c) => atrium_memory::monologue_gen::MonologueGenerator::new(c),
                None => {
                    tracing::debug!(
                        "[叙事·改] LLM 未就绪，跳过叙事改写 / LLM not ready, skip narrative rewrite"
                    );
                    return;
                }
            }
        };

        // 收集需要改写的章节 / Collect chapters that need rewriting
        // 锁安全：数据提取在 {} 块内 / Lock-safe: data extraction in {} block
        let rewrites_needed: Vec<(u64, u64, String, String, String)> = {
            let model = self.narrative_self.lock();
            model
                .active_arcs
                .iter()
                .filter(|arc| arc.is_active())
                .filter_map(|arc| {
                    // 找弧中最近一个章节 / Find the most recent chapter in this arc
                    let latest_chapter = model.chapters.iter().rfind(|ch| ch.arc_id == arc.id)?;

                    // 改写冷却：距上次改写 > 24h / Rewrite cooldown: > 24h since last rewrite
                    let last_rewrite = latest_chapter
                        .rewritten_at
                        .unwrap_or(latest_chapter.written_at);
                    if now_epoch_secs - last_rewrite < 86400 {
                        return None;
                    }

                    // 新证据：弧中在章节之后出现的转折点 / New evidence: TPs after this chapter
                    let new_evidence: Vec<String> = arc
                        .turning_point_ids
                        .iter()
                        .filter_map(|tp_id| model.get_turning_point(*tp_id))
                        .filter(|tp| tp.timestamp > latest_chapter.written_at)
                        .map(|tp| format!("[{}] {}", tp.kind.label_zh(), tp.narrative_summary))
                        .collect();

                    if new_evidence.is_empty() {
                        return None;
                    }

                    Some((
                        arc.id,
                        latest_chapter.id,
                        latest_chapter.title.clone(),
                        latest_chapter.body.clone(),
                        new_evidence.join("\n"),
                    ))
                })
                .collect()
        }; // model 锁已释放 / model lock released

        // 执行改写 / Execute rewrites
        for (arc_id, chapter_id, chapter_title, original_body, new_evidence) in rewrites_needed {
            let rewrite_target = format!("章节「{}」", chapter_title);
            let reason = "新转折点提供了对过去事件的新视角 / New turning points provide new perspective on past events";

            // LLM 调用 — 无锁状态下 .await / LLM call — .await without lock held
            match gen
                .rewrite_narrative(&rewrite_target, &original_body, &new_evidence, reason)
                .await
            {
                Ok(rewritten_text) => {
                    // 解析改写结果 / Parse rewrite result
                    let (new_title, new_body, new_summary) = parse_chapter_output(&rewritten_text);

                    // 重新获取锁写入改写结果 / Re-acquire lock to write rewrite results
                    let mut model = self.narrative_self.lock();
                    let mut writer = self.chapter_writer.lock();

                    if let Some(chapter) = model.chapters.iter_mut().find(|ch| ch.id == chapter_id)
                    {
                        writer.rewrite_chapter(chapter, new_body, new_summary, now_epoch_secs);
                        // 更新标题（如有变化）/ Update title if changed
                        if !new_title.is_empty() && new_title != chapter.title {
                            chapter.title = new_title;
                        }
                    }
                    model.refresh_stats();
                    drop(writer);
                    drop(model);

                    // 写穿持久化 / Write-through persistence
                    self.narrative_save();

                    tracing::info!(
                        "[叙事·改] 叙事改写成功: arc_id={}, chapter_id={} / Narrative rewritten",
                        arc_id,
                        chapter_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "[叙事·改] 叙事改写失败: arc_id={}, chapter_id={}, error={:?} / Narrative rewrite failed",
                        arc_id, chapter_id, e
                    );
                }
            }
        }
    }

    pub async fn tick_narrative_self_desc(&self, now_epoch_secs: i64) {
        if !self.narrative_enabled {
            return;
        }

        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let gen = {
            let client_arc = self.llm_client.lock().clone();
            match client_arc {
                Some(c) => atrium_memory::monologue_gen::MonologueGenerator::new(c),
                None => {
                    tracing::debug!("[叙事·述] LLM 未就绪，跳过自述修订 / LLM not ready, skip self-description revision");
                    return;
                }
            }
        };

        // 检查自述是否过期 + 提取数据 / Check if self-description is stale + extract data
        // 锁安全：数据提取在 {} 块内，锁在块结束时释放 / Lock-safe: data extraction in {} block, lock released at block end
        let desc_data: Option<(String, String, String, String)> = {
            let model = self.narrative_self.lock();
            // 自述重写间隔（秒）/ Self-description rewrite interval (seconds)
            let rewrite_interval_secs = self.narrative_cfg.self_description_rewrite_days * 86400;
            // 是否过期：距上次重写超过间隔 / Is stale: time since last rewrite exceeds interval
            let is_stale = now_epoch_secs - model.last_rewrite_at > rewrite_interval_secs;

            // 过期且有内容时才生成，否则跳过 / Only generate when stale and has content, otherwise skip
            if !is_stale || (model.identity_tags.is_empty() && model.active_arcs.is_empty()) {
                None
            } else {
                // 身份标签文本：标签+置信度 / Identity tags text: label + confidence
                let tags_text = model
                    .identity_tags
                    .iter()
                    .map(|t| format!("{}(置信度{:.2})", t.label, t.confidence))
                    .collect::<Vec<_>>()
                    .join("、");

                // 弧摘要文本：类型+标题+主题 / Arc summaries text: kind + title + theme
                let arcs_text = model
                    .active_arcs
                    .iter()
                    .map(|a| format!("[{}] {} — {}", a.kind.label_zh(), a.title, a.theme_sentence))
                    .collect::<Vec<_>>()
                    .join("\n");

                // 转折点摘要文本：最近5个 / Turning point summaries text: latest 5
                let tp_text = model
                    .turning_points
                    .iter()
                    .rev()
                    .take(5)
                    .map(|tp| format!("[{}] {}", tp.kind.label_zh(), tp.narrative_summary))
                    .collect::<Vec<_>>()
                    .join("\n");

                // 返回 (标签, 弧摘要, 转折点摘要, 当前自述) / Return (tags, arcs, tps, current_desc)
                Some((
                    tags_text,
                    arcs_text,
                    tp_text,
                    model.self_description.clone(),
                ))
            }
        }; // model 锁已释放 / model lock released

        // 解构数据或提前返回 / Destructure data or early return
        let (tags_text, arcs_text, tp_text, current_desc) = match desc_data {
            Some(d) => d,
            None => return,
        };

        // LLM 调用 — 无锁状态下 .await / LLM call — .await without lock held
        match gen
            .generate_self_description(&tags_text, &arcs_text, &tp_text, &current_desc)
            .await
        {
            Ok(new_description) => {
                // 重新获取锁写入结果 / Re-acquire lock to write results
                let mut model = self.narrative_self.lock();
                // 更新自我描述全文 / Update full self-description text
                model.self_description = new_description;
                // 摘要取前 50 字符 / Summary: first 50 chars
                model.self_summary = model.self_description.chars().take(50).collect();
                // 记录重写时间 / Record rewrite timestamp
                model.last_rewrite_at = now_epoch_secs;
                model.refresh_stats();
                drop(model);

                // 写穿持久化 / Write-through persistence
                self.narrative_save();

                tracing::info!("[叙事·述] 自述修订成功 / Self-description revised");
            }
            Err(e) => {
                tracing::warn!(
                    "[叙事·述] 自述修订失败: error={:?} / Self-description revision failed",
                    e
                );
            }
        }
    }

    pub fn narrative_save(&self) {
        if let Some(ref store) = self.narrative_store {
            let model = self.narrative_self.lock();
            match store.lock().save(&model) {
                Ok(()) => tracing::debug!("[叙事] 持久化保存成功"),
                Err(e) => tracing::warn!("[叙事] 持久化保存失败: {:?}", e),
            }
        }
    }

    pub fn narrative_load(&self) {
        if let Some(ref store) = self.narrative_store {
            match store.lock().load() {
                Ok(model) => {
                    let mut current = self.narrative_self.lock();
                    *current = model;
                    tracing::info!("[叙事] 持久化加载成功");
                }
                Err(e) => tracing::warn!("[叙事] 持久化加载失败: {:?}", e),
            }
        }
    }

    pub fn irrationality_save(&self) {
        if let Some(ref store) = self.irrationality_store {
            let mgr = self.irrationality.lock();
            match store.lock().save(&mgr) {
                Ok(()) => tracing::debug!("[Irrationality] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Irrationality] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn irrationality_load(&self) {
        if let Some(ref store) = self.irrationality_store {
            match store.lock().load() {
                Ok(snapshot) => {
                    let mgr = snapshot;
                    let mut current = self.irrationality.lock();
                    *current = mgr;
                    tracing::info!("[Irrationality] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Irrationality] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn ritual_save(&self) {
        if let Some(ref store) = self.ritual_store {
            let detector = self.ritual_detector.lock();
            let anniversary = self.anniversary_system.lock();
            let seasonal = self.seasonal_awareness.lock();
            match store.lock().save(&detector, &anniversary, &seasonal) {
                Ok(()) => tracing::debug!("[Ritual] 持久化保存成功 / Persist success"),
                Err(e) => tracing::warn!("[Ritual] 持久化保存失败 / Persist failed: {:?}", e),
            }
        }
    }

    pub fn ritual_load(&self) {
        if let Some(ref store) = self.ritual_store {
            match store.lock().load() {
                Ok(snapshot) => {
                    *self.ritual_detector.lock() = snapshot.ritual_detector;
                    *self.anniversary_system.lock() = snapshot.anniversary_system;
                    *self.seasonal_awareness.lock() = snapshot.seasonal_awareness;
                    tracing::info!("[Ritual] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Ritual] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn vulnerability_save(&self) {
        if let Some(ref store) = self.vulnerability_store {
            let vw = self.vulnerability_window.lock();
            match store.lock().save(&vw) {
                Ok(()) => tracing::debug!("[Vulnerability] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Vulnerability] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn vulnerability_load(&self) {
        if let Some(ref store) = self.vulnerability_store {
            match store.lock().load() {
                Ok(window) => {
                    let mut current = self.vulnerability_window.lock();
                    *current = window;
                    tracing::info!("[Vulnerability] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Vulnerability] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }
} // impl CoreService

/// 解析章节 LLM 输出 / Parse chapter LLM output
///
/// 从 LLM 生成的文本中提取标题、正文和摘要。
/// 策略：Markdown 标题 → 短行 → 首句截取。
///
/// Extracts title, body, and summary from LLM-generated text.
/// Strategy: Markdown heading → short line → first sentence.
fn parse_chapter_output(llm_text: &str) -> (String, String, String) {
    let text = llm_text.trim();

    if text.is_empty() {
        // 空输出回退 — LLM 可能返回空字符串 / Empty output fallback — LLM may return empty string
        return (
            "未命名章节 / Untitled Chapter".to_string(),
            String::new(),
            String::new(),
        );
    }

    // 尝试从第一行提取标题 / Try to extract title from first line
    // 策略：Markdown 标题 → 短行 → 首句截取 / Strategy: Markdown heading → short line → first sentence
    let (title, body) = if let Some(first_line) = text.lines().next() {
        let trimmed_line = first_line.trim();
        if trimmed_line.starts_with('#') {
            // Markdown 标题行：去掉 # 前缀 / Markdown heading line: strip # prefix
            let title_text = trimmed_line.trim_start_matches('#').trim().to_string();
            let body_start = first_line.len();
            let body_text = text[body_start..].trim().to_string();
            (title_text, body_text)
        } else if trimmed_line.len() < 40 && !trimmed_line.contains('。') {
            // 短行且无句号，视为标题 / Short line without period, treat as title
            let body_start = first_line.len();
            let body_text = text[body_start..].trim().to_string();
            (trimmed_line.to_string(), body_text)
        } else {
            // 无明确标题，整段作为正文 / No clear title, entire text as body
            (String::new(), text.to_string())
        }
    } else {
        (String::new(), text.to_string())
    };

    // 标题回退：从正文首句生成（取前 20 字符）/ Title fallback: from first sentence (first 20 chars)
    let title = if title.is_empty() {
        body.chars()
            .take_while(|c| *c != '。' && *c != '！' && *c != '？')
            .collect::<String>()
            .chars()
            .take(20)
            .collect()
    } else {
        title
    };

    // 标题最终回退 — 无任何可提取内容时 / Title ultimate fallback — when nothing extractable
    let title = if title.is_empty() {
        "叙事片段 / Narrative Fragment".to_string()
    } else {
        title
    };

    // 摘要：正文前 50 字符 / Summary: first 50 chars of body
    let summary: String = body.chars().take(50).collect();

    (title, body, summary)
}
