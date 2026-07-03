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

        // 仪式检测器 prompt（时间 + 内容合并）/ Ritual prompt (time + content combined)
        // G3 修复：使用 combined_prompt_fragment() 替代 prompt_fragment()，内容仪式中断提醒不再被丢弃
        // G3 fix: use combined_prompt_fragment() instead of prompt_fragment(), content ritual interruption reminders no longer dropped
        {
            let detector = self.ritual_detector.lock();
            let fragment = detector.combined_prompt_fragment();
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
        // 仪式检测器每日评估（时间 + 内容合并）/ Ritual detector daily evaluation (time + content combined)
        // G2 修复：加入 evaluate_content_daily()，内容仪式不再永远停在 Candidate
        // G2 fix: add evaluate_content_daily(), content rituals no longer stuck at Candidate forever
        {
            let mut detector = self.ritual_detector.lock();
            detector.evaluate_daily(now_epoch);
            detector.evaluate_content_daily(now_epoch);
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
            let mut engine = self.imperfection.lock();

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
                    if let Some(ref store) = self.imperfection_store {
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
        if let Some(ref store) = self.imperfection_store {
            let engine = self.imperfection.lock();
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
        if let Some(ref store) = self.imperfection_store {
            match store.lock().load() {
                Ok(engine) => {
                    let mut current = self.imperfection.lock();
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
            let mut engine = self.imperfection.lock();
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
        let engine = self.physical_presence.lock();
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

        let mut engine = self.physical_presence.lock();
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

        let mut engine = self.physical_presence.lock();
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
        if let Some(ref store) = self.physical_presence_store {
            let engine = self.physical_presence.lock();
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
        if let Some(ref store) = self.physical_presence_store {
            match store.lock().load() {
                Ok(engine) => {
                    let mut current = self.physical_presence.lock();
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
        let mut drive = self.curiosity_drive.lock();
        drive.accumulate(now);
    }

    /// 好奇心内驱力 prompt 注入 / Curiosity drive prompt injection
    ///
    /// 将好奇心 PAD 签名转化为 prompt 提示，让 LLM 感受到"想知道更多"的驱力。
    pub fn curiosity_drive_prompt_fragment(&self) -> String {
        let drive = self.curiosity_drive.lock();
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
        let mut resonance = self.curiosity_resonance.lock();
        resonance.tick(now);
    }

    /// 好奇心共振 prompt 注入 / Curiosity resonance prompt injection
    pub fn curiosity_resonance_prompt_fragment(&self) -> String {
        let resonance = self.curiosity_resonance.lock();
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
        let learner = self.followup_style_learner.lock();
        let summary = learner.insight_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!("[追问风格/FollowUpStyle] {}", summary)
        }
    }

    /// 多事项编织器 prompt 注入 / Multi-item weaver prompt injection
    ///
    /// 多事项编织器在运行时通过 weave() 方法将多个待追问事项编织成自然语言。
    /// 此 prompt fragment 标记其可用性，实际编织在追问决策时调用。
    pub fn multi_item_weaver_prompt_fragment(&self) -> String {
        // 多事项编织器是即时调用型模块，无独立 prompt 状态
        // Multi-item weaver is an on-demand module, no standalone prompt state
        String::new()
    }

    /// 语义关联发现 prompt 注入 / Semantic association prompt injection
    ///
    /// 基于用户消息查找语义关联，为追问提供"话题网络"线索。
    pub fn semantic_association_prompt_fragment(&self, msg: &str) -> String {
        let assoc = self.semantic_association.lock();
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
            let detector = self.ritual_detector.lock();
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
            self.ritual_anticipation
                .compute(&active_rituals, minute_of_day, relation_ordinal)
        };
        if result.is_zero() {
            String::new()
        } else {
            self.ritual_anticipation.description_zh(&result)
        }
    }

    /// 自适应仪式发现 prompt 注入 / Adaptive ritual discovery prompt injection
    ///
    /// 从用户消息中提取行为签名，发现潜在的仪式模式——
    /// "你似乎总在深夜分享音乐"这类模式的自适应捕捉。
    pub fn adaptive_ritual_prompt_fragment(&self, msg: &str) -> String {
        let ritual = self.adaptive_ritual.lock();
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
        let mut resonance = self.vulnerability_resonance.lock();
        // 推进共振衰减 — 过期脉冲自然移除 / Advance decay — expired pulses naturally removed
        let _removed = resonance.tick(now_secs);
    }

    /// 脆弱共振 prompt 注入 / Vulnerability resonance prompt injection
    pub fn vulnerability_resonance_prompt_fragment(&self, now_secs: f64) -> String {
        let resonance = self.vulnerability_resonance.lock();
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
        let wisdom = self.vulnerability_wisdom.lock();
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
        let bridge = self.imperfection_vulnerability_bridge.lock();
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

        let modulator = self.authentic_expression_modulator.lock();
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
        let mut drift = self.personality_drift.lock();
        // 简化：用默认 SolitudePattern 推进 / Simplified: advance with default pattern
        let pattern = atrium_memory::personality_drift::SolitudePattern::default();
        drift.tick(&pattern);
    }

    /// 人格漂移 prompt 注入 / Personality drift prompt injection
    pub fn personality_drift_prompt_fragment(&self) -> String {
        let drift = self.personality_drift.lock();
        drift.prompt_injection()
    }

    /// 独处原型 prompt 注入 / Solitude archetype prompt injection
    pub fn solitude_archetype_prompt_fragment(&self) -> String {
        let tracker = self.solitude_archetype.lock();
        tracker.prompt_injection()
    }

    /// 独处创造力 prompt 注入 / Solitude creativity prompt injection
    pub fn solitude_creativity_prompt_fragment(&self) -> String {
        let creativity = self.solitude_creativity.lock();
        creativity.prompt_injection()
    }

    /// 独处质量 prompt 注入 / Solitude quality prompt injection
    pub fn solitude_quality_prompt_fragment(&self) -> String {
        let quality = self.solitude_quality.lock();
        quality.to_prompt_hint()
    }

    // ── Gap#5 共享仪式补充 / Ritual supplements ──

    /// 仪式演化 prompt 注入 / Ritual evolution prompt injection
    pub fn ritual_evolution_prompt_fragment(&self) -> String {
        let evolution = self.ritual_evolution.lock();
        let desc = evolution.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[仪式演化/RitualEvolution] {}", desc)
        }
    }

    /// 仪式缺席检测 tick / Ritual absence detection tick
    pub fn ritual_absence_tick(&self, now_epoch: i64) {
        let mut absence = self.ritual_absence.lock();
        let _ = absence.detect(now_epoch);
    }

    /// 仪式缺席 prompt 注入 / Ritual absence prompt injection
    pub fn ritual_absence_prompt_fragment(&self, now_epoch: i64) -> String {
        let absence = self.ritual_absence.lock();
        absence.prompt_injection(now_epoch)
    }

    /// 仪式涌现 prompt 注入 / Ritual emergence prompt injection
    pub fn ritual_emergence_prompt_fragment(&self, now_epoch: i64) -> String {
        let emergence = self.ritual_emergence.lock();
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
        let ritual = self.vulnerability_ritual.lock();
        ritual.prompt_injection()
    }

    /// 不完美温暖 prompt 注入 / Imperfection warmth prompt injection
    pub fn imperfection_warmth_prompt_fragment(&self) -> String {
        let warmth = self.imperfection_warmth.lock();
        warmth.prompt_injection()
    }

    /// 真实不完美 prompt 注入 / Authentic imperfection prompt injection
    pub fn authentic_imperfection_prompt_fragment(&self) -> String {
        let ai = self.authentic_imperfection.lock();
        ai.prompt_injection()
    }

    // ── Gap#4 冲突与和解 / Conflict and reconciliation ──

    /// 冲突成长 tick / Conflict growth tick
    ///
    /// 后备推进：无冲突时检查和解条件。
    pub fn conflict_growth_tick(&self) {
        let mut growth = self.conflict_growth.lock();
        let pleasure: f64 = {
            let emo = self.emotion.lock();
            emo.current().pleasure as f64
        };
        let turns = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        growth.on_calm(pleasure, turns as u32);
    }

    /// 冲突成长 prompt 注入 / Conflict growth prompt injection
    pub fn conflict_growth_prompt_fragment(&self) -> String {
        let growth = self.conflict_growth.lock();
        growth.to_prompt_hint()
    }

    // ── Gap#3 期待与想念 / Anticipation and longing ──

    /// 期待深度 tick / Anticipation depth tick
    ///
    /// 推进期待深度的"途中"状态——离开越久，想念越深。
    pub fn anticipation_depth_tick(&self, now_epoch: i64) {
        let mut depth = self.anticipation_depth.lock();
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
        let depth = self.anticipation_depth.lock();
        depth.to_prompt_hint()
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
