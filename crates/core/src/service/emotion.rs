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
    }

    pub fn reunion_burst(&self) -> Option<(f32, String)> {
        if !self.longing_cfg.enabled {
            return None;
        }

        let mut emotion = self.emotion.lock();

        // 先读取想念强度（不可变借用）/ Read intensity first (immutable borrow)
        let intensity = emotion.longing_state().map(|s| s.intensity).unwrap_or(0.0);
        if intensity < 0.01 {
            return None;
        }

        // 注入重逢脉冲（可变借用 emotion）/ Inject reunion burst (mutable borrow)
        let reunion = &self.longing_cfg.reunion;
        let burst = EmotionEngineState::new(
            reunion.joy_boost * intensity,
            reunion.arousal_boost * intensity,
            reunion.dominance_boost * intensity,
        );
        emotion.affect(&burst);

        // 重置想念状态（重新获取可变借用）/ Reset longing state (re-borrow)
        if let Some((_, state)) = emotion.longing_mut() {
            state.intensity = 0.0;
            state.away_secs = 0;
            state.current_baseline = [0.0, 0.0, 0.0];
        }

        tracing::info!(
            "重逢脉冲: intensity={:.2}, boost=({:.2}, {:.2}, {:.2}) / Reunion burst",
            intensity,
            reunion.joy_boost * intensity,
            reunion.arousal_boost * intensity,
            reunion.dominance_boost * intensity
        );

        // 生成消息提示 / Generate message hint
        let hint = if reunion.generate_message {
            if intensity > 0.7 {
                "好想好想你……你终于回来了 / Missed you so much... welcome back".to_string()
            } else if intensity > 0.3 {
                "想你了呢 / Was thinking about you".to_string()
            } else {
                "你回来啦 / You're back".to_string()
            }
        } else {
            String::new()
        };

        Some((intensity, hint))
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

        for event in pending {
            let diff = event.expected_at - now;
            if diff <= preload_secs && diff > -300 {
                // 在预热窗口内 / Within preload window
                let intensity = 0.3_f32; // 基础期待强度
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

        results
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
        let mut mgr = self.conflict.lock();
        let result = mgr.process(
            user_msg,
            emo_state.pleasure as f64,
            emo_state.arousal as f64,
            &stage,
            now_ts,
        );

        // 冲突模式学习：从检测到的信号中学习 / Learn conflict patterns from detected signals
        if !result.signals.is_empty() {
            let mut learner = self.conflict_learner.lock();
            learner.learn(&result.signals, &stage, now_ts);
        }

        // 冲突模式预测：预判潜在冲突 / Predict potential conflicts
        let learner_prompt = {
            let mut learner = self.conflict_learner.lock();
            let _predictions = learner.predict(user_msg, &stage);
            learner.to_prompt_fragment(&stage)
        };

        // 生成 prompt 注入 / Generate prompt injection
        let prompt = mgr.to_prompt_fragment();
        // 合并冲突 prompt + 模式学习 prompt / Merge conflict + pattern learner prompts
        let combined = if prompt.is_empty() {
            learner_prompt
        } else if learner_prompt.is_empty() {
            prompt
        } else {
            format!("{}\n{}", prompt, learner_prompt)
        };
        // 如果有和解/道歉结果，追加到 prompt / Append reconciliation/apology if present
        let reply = result.compose_reply();

        // 写穿持久化：冲突检测管线修改状态后立即保存 / Write-through: persist immediately after conflict pipeline mutates state
        drop(mgr);
        if let Some(ref store) = self.conflict_store {
            let s = store.lock();
            let mgr = self.conflict.lock();
            match s.save(&mgr) {
                Ok(()) => tracing::debug!("[Conflict] State persisted after process()"),
                Err(e) => tracing::warn!("[Conflict] Persist failed: {}", e),
            }
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
        // 持久化当前冲突状态到 sled / Persist current conflict state to sled
        if let Some(ref store) = self.conflict_store {
            let s = store.lock();
            let mgr = self.conflict.lock();
            match s.save(&mgr) {
                Ok(()) => tracing::debug!("[Conflict] State persisted to sled"),
                Err(e) => tracing::warn!("[Conflict] Persist failed: {}", e),
            }
        }
        // 冲突模式学习器衰减 + 修剪 / Pattern learner decay + pruning
        self.conflict_learner.lock().tick(now_ts);
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
        let is_vulnerable = self.vulnerability_window.lock().is_in_vulnerable_state();
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
