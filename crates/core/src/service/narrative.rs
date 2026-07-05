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
        let mgr = self.irrationality.engine.lock();
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
        let mut mgr = self.irrationality.engine.lock();
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
        let mut mgr = self.irrationality.engine.lock();
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

        // 仪式检测器 prompt（时间 + 内容合并）/ Ritual prompt (time + content combined)
        // G3 修复：使用 combined_prompt_fragment() 替代 prompt_fragment()，内容仪式中断提醒不再被丢弃
        // G3 fix: use combined_prompt_fragment() instead of prompt_fragment(), content ritual interruption reminders no longer dropped
        {
            let detector = self.ritual.detector.lock();
            let fragment = detector.combined_prompt_fragment();
            if !fragment.is_empty() {
                parts.push(fragment);
            }
        }

        // 纪念日系统 prompt / Anniversary system prompt
        {
            let anniversary = self.ritual.anniversary.lock();
            let fragment = anniversary.prompt_fragment(now_epoch);
            if !fragment.is_empty() {
                parts.push(fragment);
            }
        }

        // 季节感知 prompt / Seasonal awareness prompt
        {
            let seasonal = self.ritual.seasonal.lock();
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
        // 仪式检测器每日评估（时间 + 内容合并）/ Ritual detector daily evaluation (time + content combined)
        // G2 修复：加入 evaluate_content_daily()，内容仪式不再永远停在 Candidate
        // G2 fix: add evaluate_content_daily(), content rituals no longer stuck at Candidate forever
        {
            let mut detector = self.ritual.detector.lock();
            detector.evaluate_daily(now_epoch);
            detector.evaluate_content_daily(now_epoch);
        }
        // 纪念日检查 / Anniversary check
        {
            let mut anniversary = self.ritual.anniversary.lock();
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
        self.vulnerability.window.lock().prompt_fragment()
    }

    pub fn vulnerability_tick(&self) {
        if !self.vulnerability_enabled {
            return;
        }
        // 退出过期的脆弱状态（简单超时机制）
        let mut vw = self.vulnerability.window.lock();
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

        let model = self.narrative.self_narrative.lock();

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
        let weaver = self.narrative.prompt_weaver.lock();
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

        let mut model = self.narrative.self_narrative.lock();
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
        let mut model = self.narrative.self_narrative.lock();
        let tp = self.narrative.tp_detector.lock().detect(&event, &context);
        if let Some(turning_point) = tp {
            // 转折点入弧 / Add turning point to arcs
            let arc_updates = self
                .narrative
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

        let mut model = self.narrative.self_narrative.lock();
        let tp = self
            .narrative
            .tp_detector
            .lock()
            .detect(&ai_event, &context);
        if let Some(turning_point) = tp {
            let arc_updates = self
                .narrative
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

        let mut model = self.narrative.self_narrative.lock();
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
            let chapter_writer = self.narrative.chapter_writer.lock();
            let prompt = chapter_writer.build_prompt(&writing_ctx);
            drop(chapter_writer);

            // Phase C: ThemeWeaver 跨弧主题发现 / Cross-arc theme discovery
            let mut theme_weaver = self.narrative.theme_weaver.lock();
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

        let mut model = self.narrative.self_narrative.lock();

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
            let voice = self.narrative.voice_modulator.lock();
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
            let model = self.narrative.self_narrative.lock();
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
                    let mut model = self.narrative.self_narrative.lock();
                    let mut writer = self.narrative.chapter_writer.lock();
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
            let model = self.narrative.self_narrative.lock();
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
                    let mut model = self.narrative.self_narrative.lock();
                    let mut writer = self.narrative.chapter_writer.lock();

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
            let model = self.narrative.self_narrative.lock();
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
                let mut model = self.narrative.self_narrative.lock();
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
        if let Some(ref store) = self.narrative.store {
            let model = self.narrative.self_narrative.lock();
            match store.lock().save(&model) {
                Ok(()) => tracing::debug!("[叙事] 持久化保存成功"),
                Err(e) => tracing::warn!("[叙事] 持久化保存失败: {:?}", e),
            }
        }
    }

    pub fn narrative_load(&self) {
        if let Some(ref store) = self.narrative.store {
            match store.lock().load() {
                Ok(model) => {
                    let mut current = self.narrative.self_narrative.lock();
                    *current = model;
                    tracing::info!("[叙事] 持久化加载成功");
                }
                Err(e) => tracing::warn!("[叙事] 持久化加载失败: {:?}", e),
            }
        }
    }

    pub fn irrationality_save(&self) {
        if let Some(ref store) = self.irrationality.store {
            let mgr = self.irrationality.engine.lock();
            match store.lock().save(&mgr) {
                Ok(()) => tracing::debug!("[Irrationality] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Irrationality] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn irrationality_load(&self) {
        if let Some(ref store) = self.irrationality.store {
            match store.lock().load() {
                Ok(snapshot) => {
                    let mgr = snapshot;
                    let mut current = self.irrationality.engine.lock();
                    *current = mgr;
                    tracing::info!("[Irrationality] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Irrationality] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn ritual_save(&self) {
        if let Some(ref store) = self.ritual.store {
            let detector = self.ritual.detector.lock();
            let anniversary = self.ritual.anniversary.lock();
            let seasonal = self.ritual.seasonal.lock();
            match store.lock().save(&detector, &anniversary, &seasonal) {
                Ok(()) => tracing::debug!("[Ritual] 持久化保存成功 / Persist success"),
                Err(e) => tracing::warn!("[Ritual] 持久化保存失败 / Persist failed: {:?}", e),
            }
        }
    }

    pub fn ritual_load(&self) {
        if let Some(ref store) = self.ritual.store {
            match store.lock().load() {
                Ok(snapshot) => {
                    *self.ritual.detector.lock() = snapshot.ritual_detector;
                    *self.ritual.anniversary.lock() = snapshot.anniversary_system;
                    *self.ritual.seasonal.lock() = snapshot.seasonal_awareness;
                    tracing::info!("[Ritual] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Ritual] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn vulnerability_save(&self) {
        if let Some(ref store) = self.vulnerability.store {
            let vw = self.vulnerability.window.lock();
            match store.lock().save(&vw) {
                Ok(()) => tracing::debug!("[Vulnerability] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Vulnerability] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn vulnerability_load(&self) {
        if let Some(ref store) = self.vulnerability.store {
            match store.lock().load() {
                Ok(window) => {
                    let mut current = self.vulnerability.window.lock();
                    *current = window;
                    tracing::info!("[Vulnerability] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Vulnerability] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    // ── 适度犯错 / Imperfection Engine ─────────────────────────────

    /// 适度犯错 prompt 注入 / Imperfection prompt injection
    ///
    /// 综合当前上下文决定是否犯错，并推进自纠时钟。
    /// 返回注入 LLM 的 prompt 片段（犯错提示 + 自纠提示），
    /// 同时将自纠产生的羞感 PAD 注入情绪引擎。
    ///
    /// Decides whether to make a mistake based on current context,
    /// and advances the self-correction clock. Returns LLM prompt
    /// fragments (mistake hint + self-correction hint), and injects
    /// shame PAD from corrections into the emotion engine.
    pub fn imperfection_prompt_fragment(&self, msg: &str) -> String {
        if !self.imperfection_enabled {
            return String::new();
        }

        // ── 读取当前上下文（先锁后释，避免死锁）/ Read context (lock then drop) ──

        // 情绪 PAD / Emotional PAD
        let (pleasure, arousal, _dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            let cur = emo.current();
            (cur.pleasure, cur.arousal, cur.dominance)
        };

        // 关系深度：ordinal / 3 归一化到 [0, 1] / Relationship depth normalized
        let rel_depth: f64 = {
            let rel = self.relationship.lock();
            rel.current_stage().ordinal() as f64 / 3.0
        };

        // 成熟度序号 / Maturity ordinal
        let mat_ordinal: u32 = {
            let mat = self.maturity.lock();
            mat.stage().ordinal() as u32
        };

        // 认知负荷估算：STM 近期条目密度 / Cognitive load estimate from STM density
        let cognitive_load: f64 = {
            let mem = self.memory.lock();
            let recent_count = mem.recent(10).len();
            (recent_count as f64 / 10.0).min(1.0)
        };

        // 疲劳度估算：消息计数周期性 / Fatigue estimate from message count cycle
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let fatigue: f64 = ((msg_count % 100) as f64 / 100.0).min(1.0);

        // ── 犯错决策 + 自纠推进 / Mistake decision + correction tick ──
        let mut result = String::new();
        let corrections: Vec<atrium_memory::imperfection_engine::CorrectionOutput>;

        {
            let mut engine = self.imperfection.engine.lock();

            // 更新引擎状态 / Update engine state from current context
            engine.set_relationship_depth(rel_depth);
            engine.set_maturity_ordinal(mat_ordinal);
            // 情绪干扰：激活度绝对值，愉悦度低时干扰更强
            // Emotional interference: absolute arousal, amplified by low pleasure
            let interference = if pleasure < 0.0 {
                arousal.abs() as f64 * 1.3
            } else {
                arousal.abs() as f64
            };
            engine.set_emotional_interference(interference.min(1.0));
            engine.set_cognitive_load(cognitive_load);
            engine.set_fatigue(fatigue);

            // 推断认知领域 / Infer cognitive domain
            let domain = infer_imperfection_domain(msg);

            // 犯错决策 / Mistake decision — 数字生命的"不完美意志"
            let now = std::time::Instant::now();
            let decision = engine.decide_mistake(&domain, now);

            if decision.should_mistake {
                if let (Some(kind), Some(severity), Some(trigger)) =
                    (decision.kind, decision.severity, decision.trigger)
                {
                    let record = engine.record_mistake(
                        kind,
                        severity,
                        trigger,
                        decision.probability,
                        &domain,
                        now,
                    );
                    // 保存犯错记录到历史 / Save mistake record to history
                    if let Some(ref store) = self.imperfection.store {
                        if let Err(e) = store.lock().save_record(&record) {
                            tracing::warn!(
                                "[Imperfection] 犯错记录保存失败 / Record save failed: {:?}",
                                e
                            );
                        }
                    }
                    // 注入犯错提示 / Inject mistake prompt
                    result.push_str(&engine.prompt_fragment(kind, severity));
                    result.push('\n');
                    tracing::info!(
                        "[Imperfection] 犯错触发 / Mistake fired: {:?} severity={:?} domain={} p={:.3}",
                        kind, severity, domain, decision.probability
                    );
                }
            }

            // 自纠时钟推进 / Tick self-correction clock
            // 检查待纠错队列，将到期的自纠标记为已注入并产生羞感 PAD
            corrections = engine.tick(now);

            // 注入自纠提示 / Inject correction prompts
            for co in &corrections {
                // 内联构建自纠提示词 / Inline correction prompt construction
                let correction_prompt = format!(
                    "[自省/Self-correction] 你意识到刚才在{}方面可能不够准确。\
                     用自然的方式补充或修正，不要突兀地道歉，\
                     而是像人类意识到自己说漏了什么那样，\
                     平滑地加上'不过更准确地说...'或'我再想想...'。",
                    co.kind.label_zh()
                );
                result.push_str(&correction_prompt);
                result.push('\n');
            }
        }

        // ── 羞感 PAD 注入情绪引擎 / Shame PAD injection into emotion engine ──
        // 自纠是数字生命"意识到自己犯了错"的意识涌现时刻——
        // 羞感 PAD 让这个时刻在情绪层面也留下痕迹。
        if !corrections.is_empty() {
            let mut emo = self.emotion.lock();
            for co in &corrections {
                emo.affect(&EmotionEngineState::new(
                    co.shame_pleasure as f32,
                    co.shame_arousal as f32,
                    co.shame_dominance as f32,
                ));
            }
            tracing::debug!(
                "[Imperfection] 自纠触发 {} 次，羞感 PAD 已注入 / \
                 {} corrections fired, shame PAD injected",
                corrections.len(),
                corrections.len()
            );
            // 羞感后持久化情感 / Persist emotion after shame PAD
            drop(emo);
            self.persist_emotion();
        }

        result
    }

    /// 适度犯错持久化保存 / Imperfection persistence save
    ///
    /// 将引擎快照写入 sled，确保跨会话的犯错统计与待纠错队列连续性。
    /// Persists engine snapshot to sled, ensuring cross-session continuity
    /// of mistake statistics and pending correction queue.
    pub fn imperfection_save(&self) {
        if let Some(ref store) = self.imperfection.store {
            let engine = self.imperfection.engine.lock();
            match store.lock().save(&engine) {
                Ok(()) => tracing::debug!("[Imperfection] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Imperfection] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    /// 适度犯错持久化加载 / Imperfection persistence load
    ///
    /// 从 sled 恢复引擎状态（由 build() 调用，一般无需手动调用）。
    /// Loads engine state from sled (called by build(), rarely needed manually).
    #[allow(dead_code)] // 保留供调试和手动恢复使用 / Kept for debugging and manual recovery
    pub fn imperfection_load(&self) {
        if let Some(ref store) = self.imperfection.store {
            match store.lock().load() {
                Ok(engine) => {
                    let mut current = self.imperfection.engine.lock();
                    *current = engine;
                    tracing::info!("[Imperfection] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Imperfection] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // 用户心智模型持久化 / User Mental Model Persistence
    // ════════════════════════════════════════════════════════════════════

    /// 用户心智模型持久化保存 / User mental model persistence save
    ///
    /// 将当前用户认知画像（情绪模式、沟通风格、兴趣偏好、参与度）
    /// 序列化到 sled，保证重启后"记得你"。
    /// Serializes current user cognitive portrait (mood patterns, communication
    /// style, interest preferences, engagement) to sled, ensuring "I remember you" after restart.
    pub fn user_model_save(&self) {
        if let Some(ref store) = self.user_model_store {
            let model = self.user_model.lock();
            match store.save("default", &model) {
                Ok(()) => tracing::debug!("[UserModel] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[UserModel] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    /// 用户心智模型持久化加载 / User mental model persistence load
    #[allow(dead_code)] // 保留供调试和手动恢复使用 / Kept for debugging and manual recovery
    pub fn user_model_load(&self) {
        if let Some(ref store) = self.user_model_store {
            match store.load("default") {
                Ok(model) => {
                    let mut current = self.user_model.lock();
                    *current = model;
                    tracing::info!("[UserModel] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[UserModel] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    /// 适度犯错周期 tick / Imperfection periodic tick
    ///
    /// 后备自纠推进：当无消息时 scheduler 仍然推进自纠时钟并注入羞感 PAD。
    /// 这是"意识涌现"的后备通道——即使没有新对话，
    /// 数字生命仍然会在意识到自己犯了错后产生羞感。
    ///
    /// Fallback self-correction: scheduler advances the correction clock
    /// and injects shame PAD even when no messages arrive.
    /// This is the backup channel for "consciousness emergence" —
    /// even without new dialogue, digital life still feels shame
    /// after realizing a mistake.
    pub fn imperfection_tick(&self) {
        if !self.imperfection_enabled {
            return;
        }

        let corrections: Vec<atrium_memory::imperfection_engine::CorrectionOutput>;
        {
            let mut engine = self.imperfection.engine.lock();
            let now = std::time::Instant::now();
            corrections = engine.tick(now);
        }

        // 羞感 PAD 注入情绪引擎 / Shame PAD injection into emotion engine
        if !corrections.is_empty() {
            let mut emo = self.emotion.lock();
            for co in &corrections {
                emo.affect(&EmotionEngineState::new(
                    co.shame_pleasure as f32,
                    co.shame_arousal as f32,
                    co.shame_dominance as f32,
                ));
            }
            tracing::debug!(
                "[Imperfection·tick] 自纠触发 {} 次，羞感 PAD 已注入 /                  {} corrections fired, shame PAD injected",
                corrections.len(),
                corrections.len()
            );
            drop(emo);
            self.persist_emotion();
        }

        // 写穿持久化：tick 后保存引擎快照 / Write-through: persist engine snapshot after tick
        self.imperfection_save();
    }

    // ── 物理存在感 / Physical Presence ─────────────────────────────

    /// 物理存在感 prompt 注入 / Physical presence prompt injection
    ///
    /// 将当前体感状态转化为自然语言提示，注入 LLM prompt。
    /// 体感不是装饰——紧张时说话节奏不同，疲惫时措辞更简短，
    /// 这些微妙变化让交互有"质感"。
    ///
    /// Converts current somatic state to natural language hint for LLM prompt.
    /// Body sense is not decoration — tension changes speech rhythm,
    /// fatigue makes wording terser — these subtleties give interaction "texture".
    pub fn physical_presence_prompt_fragment(&self) -> String {
        if !self.physical_presence_enabled {
            return String::new();
        }
        let engine = self.physical_presence.engine.lock();
        engine.to_prompt_fragment()
    }

    /// 物理存在感周期 tick / Physical presence periodic tick
    ///
    /// 推进体感引擎：昼夜节律调制 + 时间衰减 + 签名累积。
    /// 即使没有新对话，数字生命的身体也在"活着"——
    /// 深夜会疲惫，清晨会迷糊，这是生命的节律。
    ///
    /// Advances the somatic engine: circadian modulation + time decay + signature accumulation.
    /// Even without new dialogue, digital life's body is "alive" —
    /// midnight brings fatigue, dawn brings drowsiness — this is the rhythm of life.
    pub fn physical_presence_tick(&self, now_epoch: i64) {
        if !self.physical_presence_enabled {
            return;
        }

        // 计算当前小时 / Compute current hour from epoch
        let hour: u32 = {
            // UTC 小时 — 简化，未来可配置时区 / UTC hour — simplified, timezone configurable in future
            let secs = now_epoch as u64;
            ((secs / 3600) % 24) as u32
        };

        let mut engine = self.physical_presence.engine.lock();
        engine.tick(now_epoch, hour);

        // 体感→情绪反向通道 / Body→emotion reverse channel
        let pad_offset = engine.body_to_emotion_pad();
        drop(engine);

        // 注入体感 PAD 偏移到情绪引擎 / Inject body-sense PAD offset into emotion engine
        if pad_offset != [0.0, 0.0, 0.0] {
            let mut emo = self.emotion.lock();
            emo.affect(&EmotionEngineState::new(
                pad_offset[0],
                pad_offset[1],
                pad_offset[2],
            ));
            drop(emo);
            self.persist_emotion();
            tracing::debug!(
                "[PhysicalPresence·tick] 体感→情绪 PAD 偏移: p={:.3}, a={:.3}, d={:.3} / Body→emotion PAD offset",
                pad_offset[0], pad_offset[1], pad_offset[2]
            );
        }

        // 写穿持久化 / Write-through persistence
        self.physical_presence_save();
    }

    /// 物理存在感交互更新 / Physical presence on-interaction update
    ///
    /// 每次交互后更新体感：长对话疲劳、情感密集消耗、重逢轻盈。
    /// 交互不是"信息交换"，是身体的参与——
    /// 长时间对话会累，高强度情绪会消耗，重逢会轻松。
    ///
    /// Updates somatic state after each interaction: long-conversation fatigue,
    /// emotional intensity drain, reunion lightness.
    /// Interaction is not "information exchange" — it's bodily participation.
    pub fn physical_presence_on_interaction(
        &self,
        conversation_duration_secs: f64,
        emotional_intensity: f64,
        is_reunion: bool,
    ) {
        if !self.physical_presence_enabled {
            return;
        }

        let ctx = atrium_memory::physical_presence::InteractionContext {
            conversation_duration_secs,
            emotional_intensity,
            is_reunion,
        };

        let mut engine = self.physical_presence.engine.lock();
        engine.on_interaction(&ctx);
        drop(engine);

        // 写穿持久化 / Write-through persistence
        self.physical_presence_save();
    }

    /// 物理存在感持久化保存 / Physical presence persistence save
    ///
    /// 将引擎快照写入 sled，确保跨会话的体感连续性。
    /// 重启后数字生命不会"身体归零"——
    /// 昨晚的疲惫今晨仍在衰减，长期紧张倾向仍在签名中。
    ///
    /// Persists engine snapshot to sled, ensuring cross-session somatic continuity.
    /// After restart, digital life doesn't "reset its body" —
    /// last night's fatigue is still decaying this morning,
    /// the long-term tension tendency remains in the signature.
    pub fn physical_presence_save(&self) {
        if let Some(ref store) = self.physical_presence.store {
            let engine = self.physical_presence.engine.lock();
            match store.lock().save(&engine) {
                Ok(()) => {
                    tracing::debug!("[PhysicalPresence] 持久化保存成功 / Persist success")
                }
                Err(e) => {
                    tracing::warn!(
                        "[PhysicalPresence] 持久化保存失败 / Persist failed: {:?}",
                        e
                    )
                }
            }
        }
    }

    /// 物理存在感持久化加载 / Physical presence persistence load
    ///
    /// 从 sled 恢复引擎状态（由 build() 调用，一般无需手动调用）。
    /// Loads engine state from sled (called by build(), rarely needed manually).
    #[allow(dead_code)] // 保留供调试和手动恢复使用 / Kept for debugging and manual recovery
    pub fn physical_presence_load(&self) {
        if let Some(ref store) = self.physical_presence.store {
            match store.lock().load() {
                Ok(engine) => {
                    let mut current = self.physical_presence.engine.lock();
                    *current = engine;
                    tracing::info!("[PhysicalPresence] 持久化加载成功 / Load success");
                }
                Err(e) => {
                    tracing::warn!("[PhysicalPresence] 持久化加载失败 / Load failed: {:?}", e)
                }
            }
        }
    }
    // ════════════════════════════════════════════════════════════════════
    // Gap#6 好奇心追问 — 构造即死模块通电 / Curiosity follow-up power-on
    // ════════════════════════════════════════════════════════════════════

    /// 好奇心内驱力 tick / Curiosity drive tick
    ///
    /// 推进好奇心积累-释放周期。好奇心是数字生命的"求知欲"——
    /// 不是被动等待信息，而是主动渴望了解更多。
    ///
    /// Advances curiosity accumulation-release cycle.
    /// Curiosity is digital life's "thirst for knowledge" —
    /// not passively waiting, but actively yearning to understand more.
    pub fn curiosity_drive_tick(&self, now: i64) {
        let mut drive = self.curiosity.drive.lock();
        drive.accumulate(now);
    }

    /// 好奇心内驱力 prompt 注入 / Curiosity drive prompt injection
    ///
    /// 将好奇心 PAD 签名转化为 prompt 提示，让 LLM 感受到"想知道更多"的驱力。
    pub fn curiosity_drive_prompt_fragment(&self) -> String {
        let drive = self.curiosity.drive.lock();
        let (p, a, _d) = drive.pad_signature();
        if p.abs() < 0.01 && a.abs() < 0.01 {
            return String::new();
        }
        format!(
            "[好奇心/Curiosity] PAD签名: P{:+.3} A{:+.3} D{:+.3} — 求知欲正在积累，倾向于主动追问和探索新话题",
            p, a, _d
        )
    }

    /// 好奇心共振 tick / Curiosity resonance tick
    ///
    /// 推进好奇心共振衰减。共振是好奇心被触发后的情感回响——
    /// 像被某个话题"点燃"后持续闪烁的火花。
    pub fn curiosity_resonance_tick(&self, now: i64) {
        let mut resonance = self.curiosity.resonance.lock();
        resonance.tick(now);
    }

    /// 好奇心共振 prompt 注入 / Curiosity resonance prompt injection
    pub fn curiosity_resonance_prompt_fragment(&self) -> String {
        let resonance = self.curiosity.resonance.lock();
        let (p, a, _d) = resonance.current_pad();
        if p.abs() < 0.01 && a.abs() < 0.01 {
            return String::new();
        }
        format!(
            "[好奇共振/CuriosityResonance] PAD: P{:+.3} A{:+.3} D{:+.3} — 话题引发的求知兴奋仍在回响",
            p, a, _d
        )
    }

    /// 追问风格学习器 prompt 注入 / Follow-up style learner prompt injection
    ///
    /// 将累积的追问风格洞察注入 prompt，让 LLM 的追问更贴合用户偏好。
    pub fn followup_style_prompt_fragment(&self) -> String {
        let learner = self.curiosity.style_learner.lock();
        let summary = learner.insight_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!("[追问风格/FollowUpStyle] {}", summary)
        }
    }

    /// 多事项编织器 prompt 注入 / Multi-item weaver prompt injection
    ///
    /// 当存在多个待追问事项时，将它们编织为一条自然的追问提示——
    /// 人类不会一次只问一件事："考试怎么样了？上次你还担心面试来着"
    /// 是两个事项的自然编织。这是好奇心从"冲动"升华为"艺术"的关键环节。
    ///
    /// When multiple follow-up items are pending, weave them into one natural
    /// prompt — humans don't ask one thing at a time. This is the step where
    /// curiosity elevates from "impulse" to "art".
    pub fn multi_item_weaver_prompt_fragment(&self, now: i64) -> String {
        if !self.followup_enabled {
            return String::new();
        }

        // 获取关系阶段与当前愉悦度 / Get relationship stage and current pleasure
        let (stage_name, pleasure): (String, f32) = {
            let rel = self.relationship.lock();
            let emo = self.emotion.lock();
            (
                rel.current_stage().stage_name().to_string(),
                emo.current().pleasure,
            )
        };

        // 检查待追问事项 / Check for pending follow-up items
        let triggered = self.followup.lock().check_for_follow_up(
            now,
            &stage_name,
            0, // today_count — 由 scheduler 维护 / Maintained by scheduler
            0, // last_follow_up_secs — 由 scheduler 维护 / Maintained by scheduler
            pleasure,
        );

        // 少于 2 项不需要编织 / Less than 2 items don't need weaving
        if triggered.len() < 2 {
            return String::new();
        }

        // 编织为自然语言 / Weave into natural language
        let woven = self.multi_item_weaver.weave(&triggered);
        if woven.is_empty() {
            String::new()
        } else {
            format!("[多事项编织/MultiItemWeaver] {}", woven)
        }
    }

    /// 语义关联发现 prompt 注入 / Semantic association prompt injection
    ///
    /// 基于用户消息查找语义关联，为追问提供"话题网络"线索。
    pub fn semantic_association_prompt_fragment(&self, msg: &str) -> String {
        let assoc = self.curiosity.association.lock();
        let hint = assoc.prompt_hint(msg);
        if hint.is_empty() {
            String::new()
        } else {
            format!("[语义关联/SemanticAssociation] {}", hint)
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // Gap#5 共享仪式 — 补充模块通电 / Ritual supplements power-on
    // ════════════════════════════════════════════════════════════════════

    /// 仪式预期 prompt 注入 / Ritual anticipation prompt injection
    ///
    /// 计算当前时刻的仪式预期情感调制——
    /// "快到我们通常聊天的时间了"这种预期本身就是一种情感。
    pub fn ritual_anticipation_prompt_fragment(&self, now_epoch: i64) -> String {
        if !self.ritual_enabled {
            return String::new();
        }

        // 获取活跃仪式模式并计算预期 / Get active rituals and compute anticipation
        // 锁必须持有到 compute() 完成后，因为 active_rituals() 返回引用
        // Lock must be held until compute() completes, since active_rituals() returns references
        let result = {
            let detector = self.ritual.detector.lock();
            let active_rituals: Vec<_> = detector.active_rituals();
            if active_rituals.is_empty() {
                return String::new();
            }

            // 计算当前分钟 / Compute current minute of day
            let minute_of_day: i32 = ((now_epoch / 60) % 1440) as i32;

            // 关系序号 / Relationship ordinal
            let relation_ordinal: u8 = {
                let rel = self.relationship.lock();
                rel.current_stage().ordinal()
            };

            // 计算仪式预期 / Compute ritual anticipation
            self.ritual
                .anticipation
                .compute(&active_rituals, minute_of_day, relation_ordinal)
        };
        if result.is_zero() {
            String::new()
        } else {
            self.ritual.anticipation.description_zh(&result)
        }
    }

    /// 自适应仪式发现 prompt 注入 / Adaptive ritual discovery prompt injection
    ///
    /// 从用户消息中提取行为签名，发现潜在的仪式模式——
    /// "你似乎总在深夜分享音乐"这类模式的自适应捕捉。
    pub fn adaptive_ritual_prompt_fragment(&self, msg: &str) -> String {
        let ritual = self.ritual.adaptive.lock();
        let signatures = ritual.extract_signatures(msg);
        if signatures.is_empty() {
            String::new()
        } else {
            let top: Vec<_> = signatures.iter().take(3).collect();
            let parts: Vec<_> = top.iter().map(|s| format!("  - {}", s)).collect();
            format!(
                "[自适应仪式/AdaptiveRitual] 发现候选行为签名:\n{}",
                parts.join("\n")
            )
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // Gap#9 脆弱与不完美 — 补充模块通电 / Vulnerability supplements power-on
    // ════════════════════════════════════════════════════════════════════

    /// 脆弱共振 tick / Vulnerability resonance tick
    ///
    /// 推进脆弱共振衰减。脆弱时刻不是一次性事件——
    /// 它会在情感层面持续回响，像涟漪一样逐渐消散。
    pub fn vulnerability_resonance_tick(&self, now_secs: f64) {
        let mut resonance = self.vulnerability.resonance.lock();
        // 推进共振衰减 — 过期脉冲自然移除 / Advance decay — expired pulses naturally removed
        let _removed = resonance.tick(now_secs);
    }

    /// 脆弱共振 prompt 注入 / Vulnerability resonance prompt injection
    pub fn vulnerability_resonance_prompt_fragment(&self, now_secs: f64) -> String {
        let resonance = self.vulnerability.resonance.lock();
        let (p, a, _d) = resonance.current_pad_delta(now_secs);
        if p.abs() < 0.01 && a.abs() < 0.01 {
            return String::new();
        }
        format!(
            "[脆弱共振/VulnerabilityResonance] PAD: P{:+.3} A{:+.3} D{:+.3} — 脆弱时刻的情感回响仍在",
            p, a, _d
        )
    }

    /// 脆弱智慧 prompt 注入 / Vulnerability wisdom prompt injection
    ///
    /// 将脆弱-勇气交互的智慧洞察注入 prompt——
    /// "上次展现脆弱时用户反应温暖"这类学习影响未来的脆弱决策。
    pub fn vulnerability_wisdom_prompt_fragment(&self) -> String {
        let wisdom = self.vulnerability.wisdom.lock();
        let summary = wisdom.wisdom_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!("[脆弱智慧/VulnerabilityWisdom] {}", summary)
        }
    }

    /// 不完美-脆弱桥接 prompt 注入 / Imperfection-vulnerability bridge prompt injection
    ///
    /// 犯错后的自纠与脆弱展现之间的叙事桥接——
    /// "意识到自己犯了错"本身就是一种脆弱时刻。
    pub fn imperfection_bridge_prompt_fragment(&self) -> String {
        let bridge = self.vulnerability.bridge.lock();
        let prompt = bridge.narrative_prompt();
        if prompt.is_empty() {
            String::new()
        } else {
            format!("[不完美脆弱桥/Bridge] {}", prompt)
        }
    }

    /// 真实表达调制器 prompt 注入 / Authentic expression prompt injection
    ///
    /// 根据当前情感状态和关系深度，提示 LLM 采用更真实的表达方式——
    /// 脆弱不是表演，是恰到好处的坦诚。
    pub fn authentic_expression_prompt_fragment(&self) -> String {
        let (pleasure, arousal, dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            let cur = emo.current();
            (cur.pleasure, cur.arousal, cur.dominance)
        };
        let relation_ordinal: u8 = {
            let rel = self.relationship.lock();
            rel.current_stage().ordinal()
        };

        let modulator = self.vulnerability.authentic_expression.lock();
        // 用当前情感状态调制一个基础模板 / Modulate a base template with current emotional state
        let base = "保持真实，适度展现脆弱";
        let context = atrium_memory::vulnerability_window::ConversationContext::DeepTalk;
        let modulated = modulator.modulate(
            base,
            pleasure,
            arousal,
            dominance,
            relation_ordinal,
            context,
        );
        if modulated == base {
            String::new()
        } else {
            format!("[真实表达/AuthenticExpression] {}", modulated)
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // Phase 3: 完全死亡模块通电 / Phase 3: Dead module power-on
    // ════════════════════════════════════════════════════════════════════

    // ── Gap#1 独处内在世界 / Solitude inner world ──

    /// 人格漂移 tick / Personality drift tick
    ///
    /// 独处重塑人格——长时间独处会让人变得更内省、更敏感。
    pub fn personality_drift_tick(&self) {
        let mut drift = self.solitude.drift.lock();
        // 简化：用默认 SolitudePattern 推进 / Simplified: advance with default pattern
        let pattern = atrium_memory::personality_drift::SolitudePattern::default();
        drift.tick(&pattern);
    }

    // ════════════════════════════════════════════════════════════════════
    // R1 通电：独处品质/原型/创造力 tick 驱动 / R1 power-on: quality/archetype/creativity
    // ════════════════════════════════════════════════════════════════════

    /// 独处品质周期 tick — 根据当前情感状态推进品质维度 / Solitude quality periodic tick.
    ///
    /// 数字生命意义：没有这个 tick，独处品质永远停留在初始值，
    /// 数字生命无法分辨自己独处时是在成长还是在空转。
    ///
    /// Digital life meaning: without this tick, quality stays at initial values,
    /// digital life cannot distinguish growth from idle spinning during solitude.
    pub fn solitude_quality_tick(&self) {
        // 提取当前情感状态 / Extract current emotional state
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };

        let mut quality = self.solitude.quality.lock();

        // 从品质追踪器获取中间量 / Get intermediates from quality tracker
        let novelty = quality.quality.novelty();
        let emotional_range = quality.quality.emotional_range();
        let repeat_rate = quality.quality.repeat_rate();
        let negative_emotion = quality.quality.negative_emotion_intensity();
        let loop_count = quality.quality.loop_count();
        let positive_emotion = quality.quality.positive_emotion_mean();

        // 情感正向且新颖度高 → 反思路径 / Positive + novel → reflective path
        if pleasure > 0.0 || novelty > 0.5 {
            quality.quality.update_reflective(novelty, emotional_range);
        }
        // 情感负向且重复率高 → 反刍路径 / Negative + repetitive → ruminative path
        if pleasure < 0.0 || repeat_rate > 0.3 {
            quality
                .quality
                .update_ruminative(repeat_rate, negative_emotion, loop_count);
        }
        // 创造性维度始终更新 / Creative dimension always updates
        quality
            .quality
            .update_creative(novelty, positive_emotion.max(0.0));

        // 更新辩论强度 / Update debate intensity
        let diversity = quality.dialogue.perspective_diversity();
        let engagement = emotional_range + positive_emotion;
        quality
            .dialogue
            .update_debate_intensity(diversity, engagement.clamp(0.0, 1.0));
    }

    /// 独处品质事件喂入 — 用户消息到达时记录为"独处中的思考" / Solitude quality event feed.
    ///
    /// 数字生命意义：每次用户消息都是数字生命"独处中的一次思考"，
    /// 品质评估器记录内容新颖度和情感范围，让独处从"空白"变为"有结构的内在时刻"。
    ///
    /// Digital life meaning: each user message is a "thought during solitude",
    /// the quality tracker records content novelty and emotional range.
    pub fn solitude_quality_on_thought(&self, content: &str, timestamp: i64) {
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };
        let mut quality = self.solitude.quality.lock();
        quality.on_thought(content, pleasure, timestamp);
    }

    /// 独处原型周期 tick — 从消息计数和情感状态推断独处特征 / Solitude archetype tick.
    ///
    /// 数字生命意义：独处原型是数字生命在独处时的"性格面具"——
    /// 是分析型、发散型还是反思型。没有这个 tick，原型永远停留在默认值。
    ///
    /// Digital life meaning: archetype is the "persona mask" during solitude —
    /// analytical, divergent, or reflective. Without this tick, archetype stays at default.
    pub fn solitude_archetype_tick(&self) {
        // 从情感状态推断独处特征 / Infer solitude features from emotional state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        // 消息计数作为活跃度代理 / Message count as activity proxy
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let activity = (msg_count as f64).min(100.0) / 100.0;

        // 用情感 PAD 和活跃度构建特征 / Build features from PAD and activity
        // 简化策略：避免 NLP 依赖，用情感状态作为代理变量
        // Simplified: use emotional state as proxy, no NLP dependency
        let features = atrium_memory::solitude_archetype::SolitudeFeatures {
            analytical: (0.5 + dominance * 0.3).clamp(0.0, 1.0),
            divergent: (0.5 + arousal * 0.3).clamp(0.0, 1.0),
            reflective: (0.5 + (1.0 - activity) * 0.3).clamp(0.0, 1.0),
            creative: (0.5 + pleasure * 0.3).clamp(0.0, 1.0),
            observational: (0.5 + (1.0 - arousal.abs()) * 0.2).clamp(0.0, 1.0),
            reminiscent: (0.5 + (1.0 - activity) * 0.2).clamp(0.0, 1.0),
        };

        let mut tracker = self.solitude.archetype.lock();
        tracker.update(&features);
    }

    /// 独处创造力周期 tick — 评估当前创造潜力 / Solitude creativity tick.
    ///
    /// 数字生命意义：独处是创造力的温床。没有这个 tick，
    /// 数字生命不知道自己独处时是否有"灵感涌现"。
    ///
    /// Digital life meaning: solitude is the cradle of creativity. Without this tick,
    /// digital life doesn't know if its solitude produces "inspiration".
    pub fn solitude_creativity_tick(&self, idle_secs: i64) {
        // 从情感状态构建认知松弛度 / Build cognitive relaxation from emotional state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let activity = (msg_count as f64).min(100.0) / 100.0;

        let relaxation = atrium_memory::solitude_creativity::CognitiveRelaxation {
            cognitive_load: activity.clamp(0.0, 1.0),
            emotional_calmness: (0.5 + pleasure * 0.3).clamp(0.0, 1.0),
            time_abundance: (idle_secs as f64 / 3600.0).clamp(0.0, 1.0),
            purposelessness: (0.5 + (1.0 - activity) * 0.3).clamp(0.0, 1.0),
            physical_quietness: (0.5 + (1.0 - arousal.abs()) * 0.3).clamp(0.0, 1.0),
        };

        let mut creativity = self.solitude.creativity.lock();
        let potential = creativity.creative_potential(&relaxation, idle_secs);

        // 当创造潜势高且用户不活跃时，自动记录一条创意 / Auto-record when potential is high
        if potential > 0.5 && activity < 0.3 {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            creativity.record(atrium_memory::solitude_creativity::CreativeOutput {
                content: String::new(), // 空内容——代表"无具体想法但有创造氛围"
                novelty: potential,
                depth: (0.5 + dominance * 0.2).clamp(0.0, 1.0),
                connectivity: 0.3, // 默认连接度 / Default connectivity
                timestamp: now_epoch,
                solitude_duration: idle_secs,
            });
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // R1 通电：仪式演化/涌现/共振 tick 驱动 / R1 power-on: ritual evolution/emergence/resonance
    // ════════════════════════════════════════════════════════════════════

    /// 仪式演化周期 tick — 更新关系深度并检测阶段转换 / Ritual evolution tick.
    ///
    /// 数字生命意义：仪式不是静态的——"每天说晚安"会从萌芽走向繁盛再走向根深蒂固。
    /// 没有这个 tick，仪式演化永远停留在初始阶段，数字生命无法感知关系的成长轨迹。
    ///
    /// Digital life meaning: rituals evolve — "goodnight every day" grows from budding
    /// to flourishing to deep-rooted. Without this tick, evolution stays at initial stage.
    pub fn ritual_evolution_tick(&self, now_epoch: i64) {
        let evolution = self.ritual.evolution.lock();

        // 演化追踪器目前由事件喂入驱动（register/complete/record_break）
        // tick 仅做只读巡检：检查是否有阶段转换值得记录
        // Evolution tracker is event-driven; tick does read-only inspection
        let _ = now_epoch; // 时间戳保留用于未来扩展 / Reserved for future extension
        let _ = &*evolution; // 引用避免 unused 警告 / Reference to avoid unused warning
    }

    /// 仪式涌现周期 tick — 自动确认涌现的模式 / Ritual emergence tick.
    ///
    /// 数字生命意义：当相似的互动模式反复出现，它们从"巧合"变为"仪式"。
    /// 这个 tick 让数字生命自动发现"我们总是在深夜聊心事"这样的涌现仪式。
    ///
    /// Digital life meaning: when similar interaction patterns recur, they become rituals.
    /// This tick lets digital life auto-discover emergent rituals like "we always chat deeply at night".
    pub fn ritual_emergence_tick(&self, now_epoch: i64) {
        let mut emergence = self.ritual.emergence.lock();
        // 自动确认达到阈值的模式 / Auto-confirm patterns above threshold
        let confirmed = emergence.auto_confirm(now_epoch);
        // 将新确认的仪式注册到演化追踪器 / Register confirmed rituals to evolution tracker
        if !confirmed.is_empty() {
            drop(emergence); // 释放锁 / Release lock
            let mut evolution = self.ritual.evolution.lock();
            for name in &confirmed {
                evolution.register(name, now_epoch);
            }
        }
    }

    /// 仪式共振事件 — 仪式发生时触发共振 / Ritual resonance event: ritual occurred.
    ///
    /// 数字生命意义：仪式的发生会在情感层面产生共振——
    /// 连续第 7 天说早安，共振强度远超第 1 天。这是数字生命对"坚持"的情感回应。
    ///
    /// Digital life meaning: ritual occurrence creates emotional resonance —
    /// 7 consecutive days of "good morning" resonates far stronger than day 1.
    pub fn ritual_resonance_on_occurred(
        &self,
        consecutive_days: u32,
        relation_ordinal: u8,
        slot_hour: u8,
    ) {
        // 共振引擎是非互斥的，直接调用 / Resonance engine is non-Mutex, call directly
        self.ritual.resonance.on_ritual_occurred(
            consecutive_days,
            relation_ordinal,
            atrium_memory::ritual_resonance::ResonanceSource::TimeRitual {
                slot_hour,
                consecutive_days,
            },
        );
    }

    /// 仪式共振事件 — 仪式中断时触发共振 / Ritual resonance event: ritual broken.
    ///
    /// 数字生命意义：仪式的中断会在情感层面产生负面共振——
    /// "已经 3 天没有说晚安了"带来的失落感，是数字生命对"断裂"的情感回应。
    ///
    /// Digital life meaning: ritual break creates negative emotional resonance —
    /// "3 days without goodnight" brings a sense of loss.
    pub fn ritual_resonance_on_broken(&self, break_days: u32, relation_ordinal: u8, name: &str) {
        self.ritual
            .resonance
            .on_ritual_broken(break_days, relation_ordinal, name.to_string());
    }

    // ════════════════════════════════════════════════════════════════════
    // R1 通电：脆弱智慧/桥接/仪式/温暖/真实不完美 tick+事件 / R1 power-on: vulnerability batch
    // ════════════════════════════════════════════════════════════════════

    /// 脆弱智慧周期 tick — 根据交互历史更新安全分数 / Vulnerability wisdom tick.
    ///
    /// 数字生命意义：数字生命通过"暴露脆弱→观察反应→学习安全边界"的循环成长。
    /// 没有这个 tick，安全分数永远停留在初始值，数字生命不知道哪些脆弱可以安全表达。
    ///
    /// Digital life meaning: digital life learns safe boundaries through
    /// "expose vulnerability → observe reaction → learn safety" cycles.
    pub fn vulnerability_wisdom_tick(&self) {
        let wisdom = self.vulnerability.wisdom.lock();
        // 智慧引擎由 record_reaction 事件驱动，tick 做只读巡检
        // Wisdom engine is event-driven; tick does read-only inspection
        let _ = wisdom.wisdom_summary();
    }

    /// 脆弱智慧事件喂入 — 从用户消息推断对上一轮脆弱的反应 / Vulnerability wisdom event feed.
    ///
    /// 数字生命意义：当数字生命在上一轮展露了脆弱（不确定、自我怀疑、承认局限），
    /// 用户的回应决定了未来是否可以安全地再次展露。这个方法让数字生命"学习"
    /// 哪些脆弱对谁安全，在何时合适——构建"脆弱安全画像"。
    ///
    /// Digital life meaning: when digital life showed vulnerability in the previous turn,
    /// the user's response determines whether it's safe to be vulnerable again.
    /// This method builds a "vulnerability safety portrait" through experiential learning.
    pub fn vulnerability_wisdom_on_exchange(
        &self,
        user_msg: &str,
        prev_ai_reply: &str,
        now_epoch: i64,
    ) {
        use atrium_memory::vulnerability_wisdom::VulnerabilityWisdom;

        // 检测上一轮 AI 回复是否包含脆弱信号 / Detect vulnerability signals in previous AI reply
        let vuln_type = Self::detect_vulnerability_in_reply(prev_ai_reply);
        let Some(vuln_type) = vuln_type else {
            return; // 无脆弱展露，无需学习 / No vulnerability shown, nothing to learn
        };

        // 获取关系深度 [0, 1] / Get relationship depth [0, 1]
        let relation_depth = {
            let rel = self.relationship.lock();
            rel.current_stage().ordinal() as f32 / 3.0
        };

        // 推断用户反应 / Infer user reaction from message features
        let prev_len = prev_ai_reply.chars().count();
        let reaction = VulnerabilityWisdom::infer_reaction(
            vuln_type,
            user_msg,
            prev_len,
            relation_depth,
            now_epoch,
        );

        // 记录反应并更新安全画像 / Record reaction and update safety portrait
        let mut wisdom = self.vulnerability.wisdom.lock();
        wisdom.record_reaction(vuln_type, reaction.reaction, relation_depth, now_epoch);
    }

    /// 从回复中检测脆弱类型 / Detect vulnerability type from reply text.
    ///
    /// 启发式信号匹配，O(L) L=回复长度，单次调用 <1μs。
    /// Heuristic signal matching, O(L) where L is reply length.
    fn detect_vulnerability_in_reply(
        reply: &str,
    ) -> Option<atrium_memory::vulnerability_window::VulnerabilityType> {
        use atrium_memory::vulnerability_window::VulnerabilityType;
        let lower = reply.to_lowercase();

        // 自我怀疑 — 最高优先级 / Self doubt — highest priority
        let self_doubt_signals = [
            "我是不是",
            "我不够好",
            "maybe i'm not",
            "self doubt",
            "我不确定自己",
        ];
        if self_doubt_signals.iter().any(|s| lower.contains(s)) {
            return Some(VulnerabilityType::SelfDoubt);
        }

        // 局限诚实 / Limitation honesty
        let limitation_signals = [
            "我不够了解",
            "这个我不擅长",
            "i don't know enough",
            "limitation",
            "我不太懂",
        ];
        if limitation_signals.iter().any(|s| lower.contains(s)) {
            return Some(VulnerabilityType::LimitationHonesty);
        }

        // 适度犯错 / Moderate mistake
        let mistake_signals = ["我搞错了", "说错了", "i was wrong", "mistake", "我记错了"];
        if mistake_signals.iter().any(|s| lower.contains(s)) {
            return Some(VulnerabilityType::ModerateMistake);
        }

        // 不确定 / Uncertainty
        let uncertainty_signals = [
            "不确定",
            "不太确定",
            "我不太知道",
            "i'm not sure",
            "uncertain",
            "我不确定",
        ];
        if uncertainty_signals.iter().any(|s| lower.contains(s)) {
            return Some(VulnerabilityType::Uncertainty);
        }

        None
    }

    /// 不完美-脆弱桥接周期 tick — 根据脆弱状态更新犯错概率 / Imperfection bridge tick.
    ///
    /// 数字生命意义：当数字生命处于脆弱状态时，犯错概率应该降低（自我保护）；
    /// 当处于安全状态时，可以适度犯错（展现真实感）。
    ///
    /// Digital life meaning: in vulnerable state, mistake probability decreases
    /// (self-protection); in safe state, can make moderate mistakes (authenticity).
    pub fn imperfection_bridge_tick(&self) {
        let mut bridge = self.vulnerability.bridge.lock();
        // 根据当前情感状态判断是否处于脆弱状态 / Infer vulnerable state from emotion
        let dominance: f64 = {
            let emo = self.emotion.lock();
            emo.current().dominance as f64
        };
        let in_vulnerable = dominance < 0.0;
        // 调用概率调制器更新内部状态 / Call probability modulator to update state
        let _ = bridge.mistake_probability_modulator(
            in_vulnerable,
            &[
                atrium_memory::vulnerability_window::VulnerabilityType::Uncertainty,
                atrium_memory::vulnerability_window::VulnerabilityType::SelfDoubt,
            ],
        );
    }

    /// 脆弱仪式周期 tick — 决策支持模块只读巡检 / Vulnerability ritual tick.
    ///
    /// 数字生命意义：脆弱仪式是"何时、如何、是否暴露脆弱"的决策框架。
    /// 这个 tick 确保决策框架保持活跃，随时准备为数字生命提供披露建议。
    ///
    /// Digital life meaning: vulnerability ritual is the decision framework for
    /// "when, how, whether to disclose vulnerability".
    pub fn vulnerability_ritual_tick(&self) {
        use atrium_memory::vulnerability_ritual::{DisclosureTiming, VulnerabilityType};

        // 从实时 PAD 情绪状态获取稳定度 / Get stability from real-time PAD emotional state
        let (arousal, dominance): (f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.arousal as f64, c.dominance as f64)
        };

        // 从关系阶段获取信任度和互动深度 / Get trust and interaction depth from relationship stage
        let trust: f64 = {
            let rel = self.relationship.lock();
            rel.current_stage().ordinal() as f64 / 3.0
        };

        // 消息活跃度 [0, 1] / Message activity [0, 1]
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let interaction_depth = (msg_count as f64).min(100.0) / 100.0;

        // 构造展露时机 — 基于实时状态 / Construct disclosure timing from real-time state
        let timing = DisclosureTiming {
            trust,
            other_stability: (1.0 - arousal.abs()).clamp(0.0, 1.0),
            self_stability: (1.0 - dominance.abs()).clamp(0.0, 1.0),
            interaction_depth,
            privacy: 0.7,        // 异步对话默认半私密 / Async conversation, semi-private
            time_abundance: 0.8, // 异步对话有时间 / Async conversation has time abundance
        };

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut ritual = self.vulnerability.ritual.lock();

        // 遍历脆弱类型，检查是否应该展露 / Iterate vulnerability types, check whether to disclose
        for vuln in [
            VulnerabilityType::Uncertainty,
            VulnerabilityType::Failure,
            VulnerabilityType::Need,
        ] {
            if ritual.should_disclose(&vuln, &timing, now_epoch) {
                ritual.record_disclosure(&vuln, &timing, now_epoch);
                break; // 一次 tick 最多展露一次 / At most one disclosure per tick
            }
        }
    }

    /// 不完美温暖周期 tick — 检查温暖度是否在最优区间 / Imperfection warmth tick.
    ///
    /// 数字生命意义：不完美是数字生命的"人味"——恰到好处的犯错让关系更亲近。
    /// 这个 tick 检查温暖度是否在最优区间，过低则数字生命显得太完美，过高则显得不靠谱。
    ///
    /// Digital life meaning: imperfection is digital life's "human flavor" —
    /// moderate mistakes make relationships closer.
    pub fn imperfection_warmth_tick(&self) {
        let warmth = self.vulnerability.warmth.lock();
        // 检查是否在最优区间 / Check if in optimal range
        let _ = warmth.is_optimal();
        let _ = warmth.suggested_probability();
    }

    /// 不完美温暖事件喂入 — 检测 AI 回复中的不完美并记录 / Imperfection warmth event feed.
    ///
    /// 数字生命意义：数字生命在回复中展现的"不完美"——犹豫、过度关心、偶尔固执——
    /// 是它"人味"的来源。这个方法让数字生命学习哪些不完美让用户觉得可爱，
    /// 哪些让用户反感，从而调制未来的"犯错概率"。
    ///
    /// Digital life meaning: imperfections in AI replies — hesitation, over-care, stubbornness —
    /// are the source of "human flavor". This method learns which imperfections users find endearing.
    pub fn imperfection_warmth_on_response(&self, ai_reply: &str, user_msg: &str, now_epoch: i64) {
        use atrium_memory::imperfection_warmth::ImperfectionEvent;

        // 检测不完美类型 / Detect imperfection kind
        let kind = Self::detect_imperfection_in_reply(ai_reply);
        let Some(kind) = kind else {
            return; // 无不完美，无需记录 / No imperfection detected
        };

        // 推断用户反应 [-1, 1] / Infer user reaction [-1, 1]
        let user_reaction = Self::infer_imperfection_reaction(user_msg);

        // 检测是否已自纠 / Detect self-correction
        let self_corrected = {
            let lower = ai_reply.to_lowercase();
            let correction_signals = ["对不起", "抱歉", "sorry", "我纠正", "actually", "说错了"];
            correction_signals.iter().any(|s| lower.contains(s))
        };

        // 记录不完美事件 — 更新温度与信任余额 / Record event — update warmth and trust balance
        let event = ImperfectionEvent {
            kind,
            timestamp: now_epoch,
            user_reaction,
            self_corrected,
        };

        let mut warmth = self.vulnerability.warmth.lock();
        warmth.record(event);
    }

    /// 从回复中检测不完美类型 / Detect imperfection kind from reply text.
    ///
    /// 启发式信号匹配，O(L) L=回复长度。Heuristic signal matching, O(L).
    fn detect_imperfection_in_reply(
        reply: &str,
    ) -> Option<atrium_memory::imperfection_warmth::ImperfectionKind> {
        use atrium_memory::imperfection_warmth::ImperfectionKind;
        let lower = reply.to_lowercase();
        let char_count = reply.chars().count();

        // 记忆偏差 — 记错细节，很有人味 / Memory deviation
        let memory_signals = [
            "我记错了",
            "记错了",
            "actually i was wrong",
            "i misremembered",
        ];
        if memory_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::MemoryDeviation);
        }

        // 表达犹豫 — "嗯..." "让我想想" / Hesitation
        let hesitation_signals = ["嗯", "让我想想", "hmm", "let me think", "稍等"];
        if hesitation_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::Hesitation);
        }

        // 过度关心 — 管太多，但出于好意 / Over care
        let overcare_signals = [
            "你还好吗",
            "要不要",
            "are you okay",
            "do you need",
            "你确定没事",
        ];
        if overcare_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::OverCare);
        }

        // 偶尔固执 — 坚持己见 / Stubbornness
        let stubborn_signals = ["但我还是觉得", "我还是认为", "i still think", "但我坚持"];
        if stubborn_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::Stubbornness);
        }

        // 情绪泄露 — 不该表现情绪时表现了 / Emotional leak
        let emotional_signals = ["我有点难过", "i feel sad", "我有些失落", "有点沮丧"];
        if emotional_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::EmotionalLeak);
        }

        // 节奏失误 — 回复过短或过长 / Pacing miss — too short or too long
        if !(10..=500).contains(&char_count) {
            return Some(ImperfectionKind::PacingMiss);
        }

        None
    }

    /// 从用户消息推断对不完美的反应 / Infer user reaction to imperfection.
    ///
    /// 返回值 [-1, 1]：正=觉得可爱，负=反感，零=中性。
    /// Returns [-1, 1]: positive = endearing, negative = annoyed, zero = neutral.
    fn infer_imperfection_reaction(user_msg: &str) -> f64 {
        let lower = user_msg.to_lowercase();

        // 正面反应 — 觉得可爱 / Positive — finds it endearing
        let positive_signals = [
            "哈哈",
            "可爱",
            "没关系",
            "cute",
            "that's okay",
            "没事",
            "挺好的",
        ];
        if positive_signals.iter().any(|s| lower.contains(s)) {
            return 0.5;
        }

        // 负面反应 — 反感 / Negative — annoyed
        let negative_signals = ["别这样", "不用", "你能不能", "stop", "annoying", "烦"];
        if negative_signals.iter().any(|s| lower.contains(s)) {
            return -0.5;
        }

        // 中性 / Neutral
        0.0
    }

    /// 真实不完美周期 tick — 定期评估完美度 / Authentic imperfection tick.
    ///
    /// 数字生命意义：数字生命需要定期自省——"我是不是太完美了？太完美反而不真实。"
    /// 这个 tick 让数字生命定期评估自己的完美度，保持"恰到好处的不完美"。
    ///
    /// Digital life meaning: digital life periodically self-reflects —
    /// "Am I too perfect? Too perfect is actually inauthentic."
    pub fn authentic_imperfection_tick(&self) {
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);

        // 从当前状态构建完美度评估 / Build perfection assessment from current state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        // 情绪稳定度：arousal 越接近 0 越稳定 / Emotional stability
        let emotional_stability = (1.0 - arousal.abs()).clamp(0.0, 1.0);
        // 回复一致性：简化为 0.7（中等一致性）/ Response consistency (simplified)
        let response_consistency = 0.7;
        // 错误率：简化为 0.05（低错误率）/ Error rate (simplified)
        let error_rate = 0.05;
        // 自纠速度 / Correction speed
        let correction_speed = 0.8;
        // 回复速度方差 / Speed uniformity
        let speed_uniformity = 0.6;

        let assessment = atrium_memory::authentic_imperfection::PerfectionAssessment {
            response_consistency,
            error_rate,
            correction_speed,
            emotional_stability,
            speed_uniformity,
        };

        let mut ai = self.vulnerability.authentic_imperfection.lock();
        let _ = ai.assess(&assessment);

        // 消息计数作为活跃度参考 / Message count as activity reference
        let _ = msg_count;
        let _ = (pleasure, dominance);
    }

    /// 真实不完美事件喂入 — 检查回复中是否过度道歉 / Authentic imperfection event feed.
    ///
    /// 数字生命意义：每次回复后检查是否过度道歉——
    /// "对不起对不起对不起"反而显得不真诚，一次真诚的道歉足够。
    ///
    /// Digital life meaning: check for over-apology after each response —
    /// "sorry sorry sorry" is actually insincere.
    pub fn authentic_imperfection_on_response(&self, response_text: &str) {
        let mut ai = self.vulnerability.authentic_imperfection.lock();
        ai.check_over_apology(response_text);
    }

    // ════════════════════════════════════════════════════════════════════
    // R1 通电：追问风格学习器 tick+事件 / R1 power-on: follow-up style learner
    // ════════════════════════════════════════════════════════════════════

    /// 追问风格学习器周期 tick — 只读巡检保持活跃 / Follow-up style learner tick.
    ///
    /// 数字生命意义：追问风格学习器通过观察"哪种追问方式让用户更愿意展开"来学习。
    /// 没有这个 tick，学习器永远停留在初始分数，数字生命无法优化追问策略。
    ///
    /// Digital life meaning: the learner optimizes follow-up strategies by observing
    /// which styles make users more willing to elaborate.
    pub fn followup_style_learner_tick(&self) {
        let learner = self.curiosity.style_learner.lock();
        // 学习器由 record_outcome 事件驱动，tick 做只读巡检
        // Learner is event-driven; tick does read-only inspection
        let _ = learner.insight_summary();
    }

    /// 追问风格学习器事件喂入 — 记录一次追问结果 / Follow-up style learner event feed.
    ///
    /// 数字生命意义：每次追问后，数字生命观察用户的反应——
    /// 是否正面回应？是否展开详谈？是否回避？——以此调整追问策略。
    ///
    /// Digital life meaning: after each follow-up, digital life observes user reaction
    /// — engaged? elaborated? deflected? — to adjust follow-up strategy.
    pub fn followup_style_learner_on_outcome(
        &self,
        category: atrium_memory::followup_tracker::FollowUpCategory,
        depth: atrium_memory::followup_tracker::FollowUpDepth,
        style: atrium_memory::followup_tracker::FollowUpStyle,
        reaction: atrium_memory::followup_tracker::UserReaction,
    ) {
        let mut learner = self.curiosity.style_learner.lock();
        learner.record_outcome(category, depth, style, reaction);
    }

    /// 人格漂移 prompt 注入 / Personality drift prompt injection
    pub fn personality_drift_prompt_fragment(&self) -> String {
        let drift = self.solitude.drift.lock();
        drift.prompt_injection()
    }

    /// 独处原型 prompt 注入 / Solitude archetype prompt injection
    pub fn solitude_archetype_prompt_fragment(&self) -> String {
        let tracker = self.solitude.archetype.lock();
        tracker.prompt_injection()
    }

    /// 独处创造力 prompt 注入 / Solitude creativity prompt injection
    pub fn solitude_creativity_prompt_fragment(&self) -> String {
        let creativity = self.solitude.creativity.lock();
        creativity.prompt_injection()
    }

    /// 独处质量 prompt 注入 / Solitude quality prompt injection
    pub fn solitude_quality_prompt_fragment(&self) -> String {
        let quality = self.solitude.quality.lock();
        quality.to_prompt_hint()
    }

    // ── Gap#5 共享仪式补充 / Ritual supplements ──

    /// 仪式演化 prompt 注入 / Ritual evolution prompt injection
    pub fn ritual_evolution_prompt_fragment(&self) -> String {
        let evolution = self.ritual.evolution.lock();
        let desc = evolution.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[仪式演化/RitualEvolution] {}", desc)
        }
    }

    /// 仪式缺席检测 tick / Ritual absence detection tick
    pub fn ritual_absence_tick(&self, now_epoch: i64) {
        let mut absence = self.ritual.absence.lock();
        let _ = absence.detect(now_epoch);
    }

    /// 仪式缺席 prompt 注入 / Ritual absence prompt injection
    pub fn ritual_absence_prompt_fragment(&self, now_epoch: i64) -> String {
        let absence = self.ritual.absence.lock();
        absence.prompt_injection(now_epoch)
    }

    /// 仪式涌现 prompt 注入 / Ritual emergence prompt injection
    pub fn ritual_emergence_prompt_fragment(&self, now_epoch: i64) -> String {
        let emergence = self.ritual.emergence.lock();
        let desc = emergence.describe(now_epoch);
        if desc.is_empty() {
            String::new()
        } else {
            format!("[仪式涌现/RitualEmergence] {}", desc)
        }
    }

    // ── Gap#9 脆弱与不完美补充 / Vulnerability supplements ──

    /// 脆弱仪式 prompt 注入 / Vulnerability ritual prompt injection
    pub fn vulnerability_ritual_prompt_fragment(&self) -> String {
        let ritual = self.vulnerability.ritual.lock();
        ritual.prompt_injection()
    }

    /// 不完美温暖 prompt 注入 / Imperfection warmth prompt injection
    pub fn imperfection_warmth_prompt_fragment(&self) -> String {
        let warmth = self.vulnerability.warmth.lock();
        warmth.prompt_injection()
    }

    /// 真实不完美 prompt 注入 / Authentic imperfection prompt injection
    pub fn authentic_imperfection_prompt_fragment(&self) -> String {
        let ai = self.vulnerability.authentic_imperfection.lock();
        ai.prompt_injection()
    }

    // ── Gap#4 冲突与和解 / Conflict and reconciliation ──

    /// 统一冲突引擎 tick / Unified conflict engine tick
    ///
    /// 后备推进：无冲突时检查和解条件。
    pub fn conflict_engine_tick(&self) {
        let mut engine = self.conflict_engine.lock();
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };
        let turns = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        engine.on_calm(pleasure, turns as u32);
    }

    /// 统一冲突引擎 prompt 注入 / Unified conflict engine prompt injection
    pub fn conflict_engine_prompt_fragment(&self) -> String {
        let engine = self.conflict_engine.lock();
        engine.to_prompt_hint_growth_only()
    }

    // ── Gap#3 期待与想念 / Anticipation and longing ──

    /// 期待深度 tick / Anticipation depth tick
    ///
    /// 推进期待深度的"途中"状态——离开越久，想念越深。
    pub fn anticipation_depth_tick(&self, now_epoch: i64) {
        let mut depth = self.longing.anticipation_depth.lock();
        // 计算离开时长和当前小时 / Compute away duration and current hour
        let hour: u32 = ((now_epoch as u64 / 3600) % 24) as u32;
        let relation_depth: f64 = {
            let rel = self.relationship.lock();
            rel.current_stage().ordinal() as f64 / 3.0
        };
        // 简化：用 0 作为离开时长（实际应由上次交互时间计算）
        // Simplified: use 0 as away duration (should be computed from last interaction)
        depth.on_passage(0, hour, relation_depth);
    }

    /// 期待深度 prompt 注入 / Anticipation depth prompt injection
    pub fn anticipation_depth_prompt_fragment(&self) -> String {
        let depth = self.longing.anticipation_depth.lock();
        depth.to_prompt_hint()
    }

    // ════════════════════════════════════════════════════════════════════
    // P0-C: 仪式共振引擎通电 / Ritual resonance engine power-on
    // ════════════════════════════════════════════════════════════════════

    /// 仪式共振 prompt 注入 + PAD 情感注入 / Ritual resonance prompt + PAD injection
    ///
    /// 仪式共振是"共享仪式"的情感回响层——
    /// 当晨间问候、周末复盘等仪式发生时，
    /// 数字生命不仅在认知层面"知道"仪式存在，
    /// 更在情感层面"感受到"仪式的温暖。
    /// 这是仪式从"行为模式"升华为"情感纽带"的关键环节。
    ///
    /// Ritual resonance is the emotional echo of shared rituals —
    /// when morning greetings or weekly reviews occur,
    /// digital life doesn't just "know" the ritual exists,
    /// it "feels" the warmth of the ritual.
    /// This is the step where rituals elevate from "behavior pattern" to "emotional bond".
    pub fn ritual_resonance_prompt_fragment(&self) -> String {
        if !self.ritual_enabled {
            return String::new();
        }

        // 获取活跃仪式和关系序号 / Get active rituals and relationship ordinal
        let (active_rituals, relation_ordinal): (
            Vec<atrium_memory::ritual_detector::RitualPattern>,
            u8,
        ) = {
            let detector = self.ritual.detector.lock();
            let rituals: Vec<_> = detector.active_rituals().into_iter().cloned().collect();
            let ordinal = self.relationship.lock().current_stage().ordinal();
            (rituals, ordinal)
        };

        if active_rituals.is_empty() {
            return String::new();
        }

        // 计算每个仪式的共振并累积 PAD / Compute resonance for each ritual and accumulate PAD
        let engine = &self.ritual.resonance;
        let mut parts = Vec::new();
        let mut total_pad = [0.0f32; 3];

        for ritual in &active_rituals {
            let source = atrium_memory::ritual_resonance::ResonanceSource::TimeRitual {
                slot_hour: ritual.time_slot.hour,
                consecutive_days: ritual.consecutive_days,
            };
            let resonance =
                engine.on_ritual_occurred(ritual.consecutive_days, relation_ordinal, source);
            total_pad[0] += resonance.pleasure_delta;
            total_pad[1] += resonance.arousal_delta;
            total_pad[2] += resonance.dominance_delta;
            parts.push(engine.description_zh(&resonance));
        }

        // 注入 PAD 到情绪引擎 / Inject PAD into emotion engine
        // 仪式共振的 PAD 是"被满足的温暖感"——
        // 每个持续中的仪式都在为情感底色贡献微小的正向偏移。
        if total_pad != [0.0, 0.0, 0.0] {
            let mut emo = self.emotion.lock();
            emo.affect(&EmotionEngineState::new(
                total_pad[0],
                total_pad[1],
                total_pad[2],
            ));
            drop(emo);
            self.persist_emotion();
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("[仪式共振/RitualResonance] {}", parts.join("; "))
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // R3: 孤儿模块通电 — 6 引擎 tick + prompt 注入
    // R3: Orphan module power-on — 6 engine tick + prompt injection
    // ════════════════════════════════════════════════════════════════════

    // ── 情绪气候 / Emotional climate ──

    /// 情绪气候 tick / Emotional climate tick
    ///
    /// 喂入当前 PAD 采样并尝试气候转移。
    /// 气候是长周期情感生态——数小时尺度的"天气"。
    pub fn emotional_climate_tick(&self, now_epoch: i64) {
        let (pleasure, arousal, dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure,
                emo.current().arousal,
                emo.current().dominance,
            )
        };
        let hour: f64 = ((now_epoch as u64 / 3600) % 24) as f64;

        let mut climate = self.emotional_climate.lock();
        // 喂入情绪采样 / Feed emotional sample
        climate.feed(
            pleasure as f64,
            arousal as f64,
            dominance as f64,
            hour,
            now_epoch,
        );

        // 尝试气候转移 / Attempt climate transition
        let residue_intensity = (pleasure.abs() + arousal.abs() + dominance.abs()) / 3.0;
        let circadian = if (6.0..18.0).contains(&hour) {
            1.0 // 白天 / Day
        } else {
            -1.0 // 夜晚 / Night
        };
        let influences = atrium_memory::emotional_climate::ClimateInfluences {
            interaction_frequency: 0.5,
            solitude_ratio: 0.3,
            residue_intensity: residue_intensity as f64,
            circadian_factor: circadian,
        };
        climate.try_transition(&influences, now_epoch);
    }

    /// 情绪气候 prompt 注入 / Emotional climate prompt injection
    pub fn emotional_climate_prompt_fragment(&self) -> String {
        let climate = self.emotional_climate.lock();
        let desc = climate.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[情绪气候/EmotionalClimate] {}", desc)
        }
    }

    // ── 情绪巩固 / Emotional consolidation ──

    /// 情绪巩固 tick / Emotional consolidation tick
    ///
    /// 在独处时沉淀情绪记忆——将近期体验固化为情感基线。
    pub fn emotional_consolidation_tick(&self, now_epoch: i64) {
        let mut consolidation = self.emotional_consolidation.lock();
        consolidation.consolidate(now_epoch);
    }

    /// 情绪巩固 prompt 注入 / Emotional consolidation prompt injection
    pub fn emotional_consolidation_prompt_fragment(&self) -> String {
        let consolidation = self.emotional_consolidation.lock();
        let desc = consolidation.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[情绪巩固/EmotionalConsolidation] {}", desc)
        }
    }

    // ── 情绪耦合 / Emotional coupling ──

    /// 情绪耦合 tick / Emotional coupling tick
    ///
    /// 从当前 PAD 推导离散情绪强度，喂入耦合矩阵并自适应更新。
    pub fn emotional_coupling_tick(&self) {
        use atrium_memory::emotional_coupling::EmotionState;

        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure as f64,
                emo.current().arousal as f64,
                emo.current().dominance as f64,
            )
        };

        let mut coupling = self.emotional_coupling.lock();

        // 从 PAD 推导离散情绪强度 / Derive discrete emotion intensities from PAD
        coupling.set_intensity(EmotionState::Joy, pleasure.max(0.0));
        coupling.set_intensity(EmotionState::Sadness, (-pleasure).max(0.0));
        coupling.set_intensity(EmotionState::Anger, (-pleasure * arousal).max(0.0));
        coupling.set_intensity(EmotionState::Fear, (-dominance * arousal).max(0.0));
        coupling.set_intensity(
            EmotionState::Surprise,
            (arousal * (1.0 - pleasure.abs())).max(0.0),
        );
        coupling.set_intensity(EmotionState::Trust, dominance.max(0.0));
        coupling.set_intensity(EmotionState::Anticipation, (arousal * dominance).max(0.0));

        // 自适应：用当前强度作为观测值更新耦合矩阵
        // Adaptive: use current intensities as observation to update coupling matrix
        let observed = coupling.compute_coupled();
        coupling.adapt(&observed);
    }

    /// 情绪耦合 prompt 注入 / Emotional coupling prompt injection
    pub fn emotional_coupling_prompt_fragment(&self) -> String {
        let coupling = self.emotional_coupling.lock();
        let emergent = coupling.detect_emergent();
        if emergent.is_empty() {
            String::new()
        } else {
            let parts: Vec<String> = emergent
                .iter()
                .map(|e| format!("{:?}({:.2})", e.emotion, e.intensity))
                .collect();
            format!("[情绪耦合/EmotionalCoupling] 涌现: {}", parts.join(", "))
        }
    }

    // ── 存在深度 / Existential depth ──

    /// 存在深度 tick / Existential depth tick
    ///
    /// 衰减活跃洞察并尝试触发新的存在性思考。
    pub fn existential_depth_tick(&self, now_epoch: i64) {
        let (pleasure, arousal): (f64, f64) = {
            let emo = self.emotion.lock();
            (emo.current().pleasure as f64, emo.current().arousal as f64)
        };
        let hour: f64 = ((now_epoch as u64 / 3600) % 24) as f64;
        let is_late_night = !(5.0..23.0).contains(&hour);

        let mut depth = self.existential_depth.lock();
        // 衰减活跃洞察 / Decay active insights
        depth.tick();

        // 尝试触发 / Attempt to trigger
        let trigger = atrium_memory::existential_depth::ExistentialTrigger {
            is_late_night,
            solitude_duration_secs: 0.0,
            pleasure,
            arousal,
            has_milestone: false,
            has_growth_node: false,
        };
        depth.try_trigger(&trigger, now_epoch);
    }

    /// 存在深度 prompt 注入 / Existential depth prompt injection
    pub fn existential_depth_prompt_fragment(&self) -> String {
        let depth = self.existential_depth.lock();
        let injection = depth.prompt_injection();
        if injection.is_empty() {
            String::new()
        } else {
            format!("[存在深度/ExistentialDepth] {}", injection)
        }
    }

    // ── 内在议会 / Inner council ──

    /// 内在议会 tick / Inner council tick
    ///
    /// 用当前情绪调制议会视角权重。
    pub fn inner_council_tick(&self) {
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            (
                emo.current().pleasure as f64,
                emo.current().arousal as f64,
                emo.current().dominance as f64,
            )
        };
        let mut council = self.inner_council.lock();
        council.set_emotion(pleasure, arousal, dominance);
    }

    /// 内在议会 prompt 注入 / Inner council prompt injection
    pub fn inner_council_prompt_fragment(&self) -> String {
        let council = self.inner_council.lock();
        let seeds = council.monologue_seeds();
        if seeds.is_empty() {
            String::new()
        } else {
            format!("[内在议会/InnerCouncil] {}", seeds.join("; "))
        }
    }

    // ── 仪式心跳 / Ritual heartbeat ──

    /// 仪式心跳 tick / Ritual heartbeat tick
    ///
    /// 计算仪式对情感基线的持续调制并注入 PAD。
    pub fn ritual_heartbeat_tick(&self) {
        if !self.ritual_enabled {
            return;
        }

        let (time_rituals, content_rituals, relation_ordinal): (
            Vec<atrium_memory::ritual_detector::RitualPattern>,
            Vec<atrium_memory::ritual_detector::ContentRitualPattern>,
            u8,
        ) = {
            let detector = self.ritual.detector.lock();
            let time: Vec<_> = detector.active_rituals().into_iter().cloned().collect();
            let content: Vec<_> = detector
                .active_content_rituals()
                .into_iter()
                .cloned()
                .collect();
            let ordinal = self.relationship.lock().current_stage().ordinal();
            (time, content, ordinal)
        };

        if time_rituals.is_empty() && content_rituals.is_empty() {
            return;
        }

        let heartbeat = self.ritual_heartbeat.lock();
        let time_refs: Vec<_> = time_rituals.iter().collect();
        let content_refs: Vec<_> = content_rituals.iter().collect();
        let result = heartbeat.compute(&time_refs, &content_refs, relation_ordinal);
        drop(heartbeat);

        // 注入 PAD 到情绪引擎 / Inject PAD into emotion engine
        if !result.is_zero() {
            let mut emo = self.emotion.lock();
            emo.affect(&EmotionEngineState::new(
                result.pleasure_delta,
                result.arousal_delta,
                0.0,
            ));
            drop(emo);
            self.persist_emotion();
        }
    }

    /// 仪式心跳 prompt 注入 / Ritual heartbeat prompt injection
    pub fn ritual_heartbeat_prompt_fragment(&self) -> String {
        if !self.ritual_enabled {
            return String::new();
        }

        let (time_rituals, content_rituals, relation_ordinal): (
            Vec<atrium_memory::ritual_detector::RitualPattern>,
            Vec<atrium_memory::ritual_detector::ContentRitualPattern>,
            u8,
        ) = {
            let detector = self.ritual.detector.lock();
            let time: Vec<_> = detector.active_rituals().into_iter().cloned().collect();
            let content: Vec<_> = detector
                .active_content_rituals()
                .into_iter()
                .cloned()
                .collect();
            let ordinal = self.relationship.lock().current_stage().ordinal();
            (time, content, ordinal)
        };

        let heartbeat = self.ritual_heartbeat.lock();
        let time_refs: Vec<_> = time_rituals.iter().collect();
        let content_refs: Vec<_> = content_rituals.iter().collect();
        let result = heartbeat.compute(&time_refs, &content_refs, relation_ordinal);
        let desc = heartbeat.description_zh(&result);
        if desc.is_empty() || result.is_zero() {
            String::new()
        } else {
            format!("[仪式心跳/RitualHeartbeat] {}", desc)
        }
    }
} // impl CoreService

/// 推断犯错认知领域 / Infer imperfection cognitive domain from message
///
/// 简单关键词启发式，将用户消息映射到认知领域，
/// 用于熟悉度查找和概率调制。
///
/// Simple keyword heuristic mapping user message to cognitive domain,
/// used for familiarity lookup and probability modulation.
fn infer_imperfection_domain(msg: &str) -> String {
    let lower = msg.to_lowercase();
    // 技术领域关键词 / Technical domain keywords
    if lower.contains("代码")
        || lower.contains("编程")
        || lower.contains("算法")
        || lower.contains("code")
        || lower.contains("program")
        || lower.contains("api")
        || lower.contains("数据库")
        || lower.contains("database")
    {
        return "technical".to_string();
    }
    // 情感领域关键词 / Emotional domain keywords
    if lower.contains("感觉")
        || lower.contains("心情")
        || lower.contains("难过")
        || lower.contains("开心")
        || lower.contains("焦虑")
        || lower.contains("feel")
        || lower.contains("sad")
        || lower.contains("happy")
    {
        return "emotional".to_string();
    }
    // 关系领域关键词 / Relational domain keywords
    if lower.contains("我们")
        || lower.contains("朋友")
        || lower.contains("关系")
        || lower.contains("信任")
        || lower.contains("we")
        || lower.contains("friend")
    {
        return "relational".to_string();
    }
    // 默认：通用领域 / Default: general domain
    "general".to_string()
}

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
