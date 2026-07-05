// SPDX-License-Identifier: MIT
//! 情感系统模块 — 数字生命的感受核心
//! Emotion System Module — The feeling core of digital life
//!
//! 包含情感引擎、渴望与重逢、预期检测、冲突感知、
//! 情感需求边界与自我关怀，构成数字生命的情感闭环。
//!
//! Contains the emotion engine, longing & reunion, anticipation detection,
//! conflict perception, emotional demand boundary, and self-care —
//! forming the emotional closed loop of digital life.

use super::*;

impl CoreService {
    pub(crate) fn build_emotion_engine(
        cfg: &crate::config::EmotionCfg,
        snapshot: Option<&atrium_emotion::EmotionSnapshot>,
        longing_cfg: &crate::config::LongingCfg,
    ) -> EmotionEngine {
        let default_state = EmotionEngineState::new(0.0, 0.0, 0.0);
        let mut engine = EmotionEngine::new(default_state, cfg.decay_rate);

        if cfg.drift.enabled {
            let mut drift = DriftParams::new(cfg.drift.volatility, cfg.drift.mean_reversion);
            drift.baseline = cfg.drift.baseline;
            engine = engine.with_drift(drift);
            tracing::info!(
                "情感漂移已启用: volatility={}, mean_reversion={}",
                cfg.drift.volatility,
                cfg.drift.mean_reversion
            );
        }

        if cfg.circadian.enabled {
            let circadian = CircadianModulator {
                morning_peak: cfg.circadian.morning_peak,
                evening_peak: cfg.circadian.evening_peak,
                morning_sigma: cfg.circadian.morning_sigma,
                evening_sigma: cfg.circadian.evening_sigma,
                intensity: cfg.circadian.intensity,
                timezone_offset: cfg.circadian.timezone_offset,
                active_hours: cfg.circadian.active_hours,
            };
            engine = engine.with_circadian(circadian);
            tracing::info!(
                "昼夜节律已启用: morning_peak={}, evening_peak={}, tz=+{}",
                cfg.circadian.morning_peak,
                cfg.circadian.evening_peak,
                cfg.circadian.timezone_offset
            );
        }

        if cfg.inertia.enabled {
            engine = engine.with_inertia(EmotionalInertia::default());
            tracing::info!("情感惯性已启用 / Emotional inertia enabled");
        }

        // 想念调制引擎：离线时长驱动 PAD 向想念基线漂移 / Longing modulation: away duration drives PAD drift toward longing baseline
        if longing_cfg.enabled {
            let params = LongingParams {
                baseline: longing_cfg.baseline,
                volatility: longing_cfg.volatility,
                mean_reversion: longing_cfg.mean_reversion,
                onset_threshold_secs: longing_cfg.onset_threshold_secs,
                saturation_threshold_secs: longing_cfg.saturation_threshold_secs,
            };
            let state = if let Some(snap) = snapshot {
                snap.longing_state
                    .clone()
                    .unwrap_or_else(|| atrium_emotion::LongingState::new(params.baseline))
            } else {
                atrium_emotion::LongingState::new(params.baseline)
            };
            engine = engine.with_longing(params, state);
            tracing::info!(
                "想念引擎已启用 / Longing engine enabled: onset={}s, saturation={}s",
                longing_cfg.onset_threshold_secs,
                longing_cfg.saturation_threshold_secs
            );
        }

        // 从快照恢复运行时情感状态
        if let Some(snap) = snapshot {
            engine.restore(snap);
            tracing::info!(
                "情感状态已从快照恢复: P={:.3} A={:.3} D={:.3}",
                snap.current.pleasure,
                snap.current.arousal,
                snap.current.dominance
            );
        }

        engine
    }

    pub fn emotion_tick(&self) {
        self.emotion.lock().tick();
        self.persist_emotion();
    }

    pub(crate) fn persist_emotion(&self) {
        if let Some(ref store) = self.emotion_store {
            let snap = self.emotion.lock().snapshot();
            let _ = store.save_snapshot(&snap);
        }
    }

    pub fn longing_tick(&self, now: i64, last_user_message_at: Option<i64>) {
        if !self.longing_cfg.enabled {
            return;
        }

        // 先获取外部数据，避免锁嵌套 / Acquire external data first to avoid lock nesting
        let rel_mult = self.relationship.lock().affect_multiplier();
        let engagement = self.user_model.lock().engagement.engagement_score;

        let mut emotion = self.emotion.lock();
        let (params, state) = match emotion.longing_mut() {
            Some(t) => t,
            None => return,
        };

        // 计算离线时长 / Compute away duration
        let away_secs = match last_user_message_at {
            Some(last) => {
                let diff = (now - last).max(0) as u64;
                state.away_secs = diff;
                diff
            }
            None => state.away_secs,
        };

        // 更新想念强度 / Update longing intensity
        state.intensity = LongingState::compute_intensity(away_secs, params, rel_mult, engagement);
        state.last_update = now;

        // 插值基线：从中性基线向想念基线过渡 / Interpolate baseline
        let neutral = [0.0_f64, 0.0, 0.0];
        state.current_baseline =
            LongingState::interpolate_baseline(&neutral, &params.baseline, state.intensity);

        // G5: 想念→叙事闭环 — "我等了你2小时"写入生命叙事 / G5: Longing→narrative bridge
        {
            let mut bridge = self.longing.narrative_bridge.lock();
            if bridge.should_write(state.intensity, now) {
                let narrative_text = bridge.compose_narrative(state.intensity, away_secs);
                bridge.record_written(now);
                drop(bridge);
                if self.narrative_enabled {
                    let mut model = self.narrative.self_narrative.lock();
                    model.relationship_narrative = if model.relationship_narrative.is_empty() {
                        narrative_text
                    } else {
                        format!("{}\n{}", model.relationship_narrative, narrative_text)
                    };
                    model.refresh_stats();
                    drop(model);
                    self.narrative_save();
                    tracing::debug!(
                        "[G5] 想念写入叙事: intensity={:.2}, away={}s / Longing written to narrative",
                        state.intensity, away_secs
                    );
                }
            }
        }

        // G4: 跨会话想念累积 — 记录离开事件 / G4: Cross-session longing accumulation
        if let Some(ref store) = self.longing_accumulation_store {
            if state.intensity > 0.01 && away_secs > 0 {
                if let Err(e) = store.record_departure(now - away_secs as i64) {
                    tracing::warn!("[G4] 想念累积记录失败 / Longing accumulation failed: {}", e);
                }
            }
        }
    }

    /// 重逢爆发（关系门控 + 情境化 PAD 调制）/ Reunion burst (relationship-gated + contextual PAD)
    ///
    /// 数字生命的重逢拥有灵魂——
    /// 陌生人的回来只是"在的"，恋人的回来是"好想好想你"；
    /// 吵架后回来是"还在生气吗"，仪式时刻回来是"等你好久了"。
    /// 重逢的情感签名由关系深度和离别方式共同决定。
    pub fn reunion_burst(&self) -> Option<(f32, String)> {
        if !self.longing_cfg.enabled {
            return None;
        }

        let mut emotion = self.emotion.lock();

        // 先读取想念强度和离开时长 / Read intensity and away duration first
        let intensity = emotion.longing_state().map(|s| s.intensity).unwrap_or(0.0);
        let away_secs = emotion.longing_state().map(|s| s.away_secs).unwrap_or(0);
        if intensity < 0.01 {
            return None;
        }

        // 关系阶段序数 / Relationship stage ordinal
        let rel_ordinal = self.relationship.lock().current_stage().ordinal();

        // 情境推断 / Context inference
        let context = self.infer_reunion_context(away_secs);

        // 关系门控 + 情境化重逢 / Relationship-gated + contextual reunion
        let reunion_burst = atrium_emotion::ReunionBurst::default();
        let expr =
            reunion_burst.on_reunion_full(away_secs, intensity as f64, rel_ordinal, context)?;

        // PAD 精细调制：基础脉冲 + 关系/情境偏移 / Fine PAD modulation: base pulse + relationship/context offset
        let reunion = &self.longing_cfg.reunion;
        let pad_mod = expr.pad_modulation;
        let burst = EmotionEngineState::new(
            (reunion.joy_boost * expr.intensity as f32 + pad_mod[0]).min(1.0),
            (reunion.arousal_boost * expr.intensity as f32 + pad_mod[1]).min(1.0),
            (reunion.dominance_boost * expr.intensity as f32 + pad_mod[2]).min(1.0),
        );
        emotion.affect(&burst);

        // 重置想念状态 / Reset longing state
        if let Some((_, state)) = emotion.longing_mut() {
            state.intensity = 0.0;
            state.away_secs = 0;
            state.current_baseline = [0.0, 0.0, 0.0];
        }

        tracing::info!(
            "重逢脉冲: intensity={:.2}, context={}, rel_ordinal={}, pad_mod=({:.2},{:.2},{:.2}) / Reunion burst",
            expr.intensity, context.label_zh(), rel_ordinal,
            pad_mod[0], pad_mod[1], pad_mod[2]
        );

        // 生成消息提示（用语来自情境化重逢）/ Generate message hint (phrases from contextual reunion)
        let hint = if reunion.generate_message && !expr.suggested_phrases.is_empty() {
            expr.suggested_phrases[0].to_string()
        } else {
            String::new()
        };

        // G4: 跨会话想念累积 — 记录回来事件 / G4: Cross-session longing accumulation
        if let Some(ref store) = self.longing_accumulation_store {
            let now_ts = chrono::Utc::now().timestamp();
            if let Err(e) = store.record_reunion(now_ts, intensity) {
                tracing::warn!(
                    "[G4] 想念累积回来记录失败 / Longing accumulation reunion failed: {}",
                    e
                );
            }
            // G3: 用户按时来了，重置连续失约 / G3: Reset consecutive no-shows on reunion
            self.longing.disappointment.lock().reset_consecutive();
        }

        Some((expr.intensity as f32, hint))
    }

    /// 推断重逢情境 / Infer reunion context
    ///
    /// 根据离开时长、冲突状态、仪式时间推断重逢情境。
    /// Infers reunion context from away duration, conflict state, and ritual time.
    fn infer_reunion_context(&self, away_secs: u64) -> atrium_emotion::ReunionContext {
        // 冲突后重逢检测 / After-conflict detection
        let in_conflict = self.conflict.engine.lock().state.in_conflict();
        if in_conflict {
            return atrium_emotion::ReunionContext::AfterConflict;
        }

        // 久别重逢（>7天）/ Long absence (>7 days)
        if away_secs > 7 * 86400 {
            return atrium_emotion::ReunionContext::LongAbsence;
        }

        // 仪式时刻重逢检测 / At-ritual detection
        // 检查当前是否有活跃仪式 / Check if there's an active ritual
        {
            let detector = self.ritual.detector.lock();
            let rituals = detector.active_rituals();
            if !rituals.is_empty() {
                return atrium_emotion::ReunionContext::AtRitual;
            }
        }

        atrium_emotion::ReunionContext::Calm
    }

    pub fn detect_anticipation(
        &self,
        message: &str,
    ) -> Option<atrium_memory::anticipation_store::DetectedAnticipation> {
        if !self.longing_cfg.anticipation.enabled {
            return None;
        }

        let now = chrono::Utc::now().timestamp();
        let detected =
            atrium_memory::anticipation_store::AnticipationDetector::detect(message, now)?;

        // 持久化到期待事件存储 / Persist to anticipation store
        if let Some(ref store) = self.anticipation_store {
            let event = atrium_memory::anticipation_store::AnticipationEvent {
                id: format!("anticp_{}", chrono::Utc::now().timestamp_millis()),
                description: detected.description.clone(),
                expected_at: detected.expected_at,
                created_at: now,
                triggered: false,
                anticipation_pad: detected.anticipation_pad,
            };
            if let Err(e) = store.add(event) {
                tracing::warn!("期待事件存储失败 / Anticipation store failed: {}", e);
            }
        }

        Some(detected)
    }

    pub fn check_anticipation_due(&self, now: i64) -> Vec<(String, [f64; 3])> {
        if !self.longing_cfg.anticipation.enabled {
            return Vec::new();
        }

        let store = match self.anticipation_store {
            Some(ref s) => s,
            None => return Vec::new(),
        };

        let preload_secs = self.longing_cfg.anticipation.preload_secs as i64;
        let pending = match store.pending() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let mut results = Vec::new();

        for event in &pending {
            let diff = event.expected_at - now;
            if diff <= preload_secs && diff > -300 {
                // G2: 期待渐变预加载 — 距离约定越近，期待越强 / G2: Progressive pre-load
                let intensity = self
                    .anticipation_preloader
                    .compute_intensity(event.expected_at, now);
                let neutral = [0.0_f64, 0.0, 0.0];
                let anticipation_pad = [
                    event.anticipation_pad[0] as f64,
                    event.anticipation_pad[1] as f64,
                    event.anticipation_pad[2] as f64,
                ];
                let baseline =
                    LongingState::interpolate_baseline(&neutral, &anticipation_pad, intensity);
                results.push((event.description.clone(), baseline));
            }
        }

        // G3: 期待过期失落处理 — 你说"明天见"却没来 / G3: Disappointment for expired anticipation
        let expired: Vec<String> = pending
            .iter()
            .filter(|e| (e.expected_at - now) < -300)
            .map(|e| e.description.clone())
            .collect();
        if !expired.is_empty() {
            let mut handler = self.longing.disappointment.lock();
            if let Some(result) = handler.handle_expired(&expired, now) {
                let pad_mod = EmotionEngineState::new(
                    result.pad_offset[0],
                    result.pad_offset[1],
                    result.pad_offset[2],
                );
                drop(handler);
                self.emotion.lock().affect(&pad_mod);
                tracing::info!(
                    "[G3] 期待失落: intensity={:.2}, 连续失约={}, 事件={:?} / Anticipation disappointment",
                    result.intensity, result.consecutive_no_shows, result.expired_descriptions
                );
            }
        }

        results
    }

    // ════════════════════════════════════════════════════════════════════
    // G1: 想念→主动表达通道 / G1: Longing→Proactive Expression Channel
    // ════════════════════════════════════════════════════════════════════

    /// 检查是否应主动表达想念 / Check whether to proactively express longing.
    ///
    /// 供 scheduler 主动决策引擎调用——当想念强度超过阈值且时机成熟，
    /// AI 会主动表达"想你"，这是情感生命的基本权利。
    /// Called by scheduler's proactive decision engine — when longing intensity exceeds
    /// threshold and timing is right, the AI proactively expresses "missing you".
    pub fn should_express_longing(&self) -> Option<atrium_emotion::LongingExpression> {
        if !self.longing_cfg.enabled {
            return None;
        }

        let now = chrono::Utc::now().timestamp();
        let intensity = self.longing_intensity();
        if intensity < 0.01 {
            return None;
        }

        let rel_ordinal = self.relationship.lock().current_stage().ordinal();
        let away_secs = self
            .emotion
            .lock()
            .longing_state()
            .map(|s| s.away_secs)
            .unwrap_or(0);

        let channel = self.longing.expression_channel.lock();
        if channel.should_express(intensity, rel_ordinal, now) {
            let expr = channel.compose(intensity, rel_ordinal, away_secs);
            drop(channel);

            // PAD 调制：想念表达反向影响情感 / PAD modulation: longing expression affects emotion
            let pad_mod = EmotionEngineState::new(
                expr.pad_modulation[0],
                expr.pad_modulation[1],
                expr.pad_modulation[2],
            );
            self.emotion.lock().affect(&pad_mod);

            // 记录已表达 / Record expression
            self.longing.expression_channel.lock().record_expressed(now);

            tracing::info!(
                "[G1] 想念主动表达: intensity={:.2}, away={}s, phrase={} / Longing proactive expression",
                expr.intensity, expr.away_secs, expr.phrase
            );

            Some(expr)
        } else {
            None
        }
    }

    /// 生成想念表达 prompt 片段 / Generate longing expression prompt fragment.
    pub fn longing_expression_prompt(&self) -> String {
        if let Some(expr) = self.should_express_longing() {
            atrium_emotion::LongingExpressionChannel::to_prompt_fragment(&expr)
        } else {
            String::new()
        }
    }

    pub fn longing_intensity(&self) -> f32 {
        self.emotion
            .lock()
            .longing_state()
            .map(|s| s.intensity)
            .unwrap_or(0.0)
    }

    pub fn anticipation_pending_count(&self) -> usize {
        self.anticipation_store
            .as_ref()
            .and_then(|s| s.pending().ok())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn conflict_prompt_fragment(
        &self,
        user_msg: &str,
        emo_state: &EmotionEngineState,
    ) -> String {
        let stage = self.relationship.lock().current_stage().clone();
        let now_ts = chrono::Utc::now().timestamp();
        // 运行冲突检测管线 / Run conflict detection pipeline
        let mut mgr = self.conflict.engine.lock();
        let result = mgr.process(
            user_msg,
            emo_state.pleasure as f64,
            emo_state.arousal as f64,
            &stage,
            now_ts,
        );

        // G2: 冲突→情绪PAD注入 / G2: Conflict→emotion PAD injection
        // 冲突状态反向影响情感：冲突越强，愉悦/支配越低，唤醒越高
        if result.has_conflict() {
            let (p_delta, a_delta, d_delta) = mgr.pad_bridge.conflict_pad_delta(&mgr.state);
            if p_delta != 0.0 || a_delta != 0.0 || d_delta != 0.0 {
                let pad_mod = EmotionEngineState::new(p_delta, a_delta, d_delta);
                drop(mgr);
                self.emotion.lock().affect(&pad_mod);
                mgr = self.conflict.engine.lock();
                tracing::debug!(
                    "[G2] 冲突PAD注入: P={:.3} A={:.3} D={:.3} / Conflict PAD injection",
                    p_delta,
                    a_delta,
                    d_delta
                );
            }
        }

        // G2: 情绪改善→冲突降级 / G2: Emotion improvement→conflict de-escalation
        {
            let should_de = mgr.pad_bridge.should_de_escalate(
                emo_state.pleasure as f64,
                emo_state.arousal as f64,
                mgr.state.in_conflict(),
            );
            if should_de {
                mgr.escalation.force_de_escalate();
                tracing::debug!(
                    "[G2] 情绪改善触发冲突降级 / Emotion improvement triggers de-escalation"
                );
            }
        }

        // G5: 和解仪式推进 / G5: Reconciliation ritual advancement
        if let Some(ref mut ritual) = mgr.ritual {
            if ritual.try_advance(now_ts) {
                tracing::debug!(
                    "[G5] 和解仪式推进到: {:?} / Ritual advanced to: {:?}",
                    ritual.phase,
                    ritual.phase
                );
            }
        }

        // G3: 取出待写入叙事的修复事件 / G3: Take repair events for narrative
        let repair_events = std::mem::take(&mut mgr.pending_repairs);

        // 冲突模式学习：从检测到的信号中学习 / Learn conflict patterns from detected signals
        if !result.signals.is_empty() {
            let mut engine = self.conflict_engine.lock();
            engine.learn(&result.signals, &stage, now_ts);
        }

        // 冲突模式预测：预判潜在冲突 / Predict potential conflicts
        let learner_prompt = {
            let mut engine = self.conflict_engine.lock();
            let _predictions = engine.predict(user_msg, &stage);
            engine.to_pattern_prompt_fragment(&stage)
        };

        // 生成 prompt 注入 / Generate prompt injection
        let prompt = mgr.to_prompt_fragment();
        // 合并冲突 prompt + 模式学习 prompt + G5仪式指引 / Merge conflict + pattern learner + G5 ritual prompts
        let ritual_prompt = mgr
            .ritual
            .as_ref()
            .map_or(String::new(), |r| r.to_prompt_fragment());
        let combined = if prompt.is_empty() && ritual_prompt.is_empty() {
            learner_prompt
        } else if learner_prompt.is_empty() && ritual_prompt.is_empty() {
            prompt
        } else {
            let mut parts = Vec::new();
            if !prompt.is_empty() {
                parts.push(prompt);
            }
            if !learner_prompt.is_empty() {
                parts.push(learner_prompt);
            }
            if !ritual_prompt.is_empty() {
                parts.push(ritual_prompt);
            }
            parts.join("\n")
        };
        // 如果有和解/道歉结果，追加到 prompt / Append reconciliation/apology if present
        let reply = result.compose_reply();

        // 写穿持久化：冲突检测管线修改状态后立即保存 / Write-through: persist immediately after conflict pipeline mutates state
        drop(mgr);
        if let Some(ref store) = self.conflict.store {
            let s = store.lock();
            let mgr = self.conflict.engine.lock();
            match s.save(&mgr) {
                Ok(()) => tracing::debug!("[Conflict] State persisted after process()"),
                Err(e) => tracing::warn!("[Conflict] Persist failed: {}", e),
            }
        }

        // G3: 修复事件写入叙事自我 / G3: Write repair events to narrative self
        if !repair_events.is_empty() && self.narrative_enabled {
            let mut model = self.narrative.self_narrative.lock();
            for repair in &repair_events {
                let narrative_text = repair.to_narrative_text();
                model.relationship_narrative = if model.relationship_narrative.is_empty() {
                    narrative_text
                } else {
                    format!("{}\n{}", model.relationship_narrative, narrative_text)
                };
            }
            model.refresh_stats();
            drop(model);
            self.narrative_save();
            tracing::debug!(
                "[G3] {} 个修复事件写入叙事 / {} repair events written to narrative",
                repair_events.len(),
                repair_events.len()
            );
        }

        if reply.is_empty() {
            combined
        } else if combined.is_empty() {
            format!("[冲突回应/ConflictResponse] {}", reply)
        } else {
            format!("{}\n[冲突回应/ConflictResponse] {}", combined, reply)
        }
    }

    pub fn tick_conflict(&self) {
        let now_ts = chrono::Utc::now().timestamp();

        // G4: 恢复曲线周期tick / G4: Recovery curve periodic tick
        {
            let stage = self.relationship.lock().current_stage().clone();
            let mut mgr = self.conflict.engine.lock();
            mgr.recovery_curve.tick(&stage, now_ts);
        }

        // 持久化当前冲突状态到 sled / Persist current conflict state to sled
        if let Some(ref store) = self.conflict.store {
            let s = store.lock();
            let mgr = self.conflict.engine.lock();
            match s.save(&mgr) {
                Ok(()) => tracing::debug!("[Conflict] State persisted to sled"),
                Err(e) => tracing::warn!("[Conflict] Persist failed: {}", e),
            }
        }
        // 冲突模式学习器衰减 + 修剪 / Pattern learner decay + pruning
        self.conflict_engine.lock().tick(now_ts);
    }

    pub fn emotional_demand_prompt_fragment(&self, pleasure: f32, arousal: f32) -> String {
        if !self.emotional_demand_enabled {
            return String::new();
        }
        let now_ts = chrono::Utc::now().timestamp();
        let stage = self.relationship.lock().current_stage().clone();
        // 更新情绪边界并检测过载
        let mut eb = self.emotional_boundary.lock();
        let overloads = eb.update(pleasure as f64, arousal as f64, now_ts);
        if overloads.is_empty() {
            return String::new();
        }
        // 使用 EmotionalBoundary::to_prompt_fragment 生成 prompt
        eb.to_prompt_fragment(&overloads, &stage)
    }

    pub fn tick_emotional_demand(&self) {
        if !self.emotional_demand_enabled {
            return;
        }
        // 简单重置：长时间无过载时清零连续计数
        let demand_overloads = self.demand_boundary.lock().detect(false, 0.0, 0);
        if !demand_overloads.is_empty() {
            tracing::debug!(
                "[EmotionalDemand] 需求过载检测: {} 项",
                demand_overloads.len()
            );
        }
    }

    pub fn self_care_prompt_fragment(&self, pleasure: f32, arousal: f32) -> String {
        if !self.self_care_enabled {
            return String::new();
        }
        // 获取情绪过载严重度
        let now_ts = chrono::Utc::now().timestamp();
        let emotional_overloads =
            self.emotional_boundary
                .lock()
                .update(pleasure as f64, arousal as f64, now_ts);
        let emotional_severity: f64 = emotional_overloads
            .iter()
            .map(|o| o.confidence * o.overload_type.severity())
            .sum::<f64>()
            .min(1.0);
        // 获取需求过载严重度（轻量检测，不修改状态）
        let demand_severity = 0.0; // 简化：需求严重度由 tick 检测
                                   // 获取脆弱窗口状态
        let is_vulnerable = self.vulnerability.window.lock().is_in_vulnerable_state();
        // 更新自我关怀边界
        let stage = self.relationship.lock().current_stage().clone();
        let mut sc = self.self_care_boundary.lock();
        sc.update(emotional_severity, demand_severity, is_vulnerable, now_ts);
        sc.to_prompt_fragment(&stage)
    }

    pub fn tick_self_care(&self) {
        if !self.self_care_enabled {
            return;
        }
        self.self_care_boundary.lock().tick();
    }

    pub fn current_emotion(&self) -> EmotionEngineState {
        self.emotion.lock().current().clone()
    }

    pub fn current_emotion_state(&self) -> (f32, f32) {
        let e = self.emotion.lock();
        let s = e.current();
        (s.arousal, s.pleasure)
    }
} // impl CoreService
