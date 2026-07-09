// SPDX-License-Identifier: MIT
//! 仪式系统 — 心跳/期待/涌现/共振/缺失/演化
//! Ritual System — Heartbeat/Anticipation/Emergence/Resonance/Absence/Evolution

use super::*;

impl CoreService {
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
            anniversary.check_today(now_epoch); // 纪念日检查有副作用 / Anniversary check has side effects
        }
        tracing::debug!("[Ritual] Periodic tick completed");

        // 写穿持久化：仪式评估后保存，防止重启丢失仪式模式与纪念日记忆 / Write-through: persist after ritual evaluation to preserve patterns and anniversaries
        self.ritual_save();
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
                let rel = self.relationship.read();
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

    pub fn ritual_evolution_tick(&self, now_epoch: i64) {
        let evolution = self.ritual.evolution.lock();

        // 演化追踪器目前由事件喂入驱动（register/complete/record_break）
        // tick 仅做只读巡检：检查是否有阶段转换值得记录
        // Evolution tracker is event-driven; tick does read-only inspection
        let _ = now_epoch; // 时间戳保留用于未来扩展 / Reserved for future extension
        let _ = &*evolution; // 引用避免 unused 警告 / Reference to avoid unused warning
    }

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

    pub fn ritual_resonance_on_broken(&self, break_days: u32, relation_ordinal: u8, name: &str) {
        self.ritual
            .resonance
            .on_ritual_broken(break_days, relation_ordinal, name.to_string());
    }

    pub fn ritual_evolution_prompt_fragment(&self) -> String {
        let evolution = self.ritual.evolution.lock();
        let desc = evolution.describe();
        if desc.is_empty() {
            String::new()
        } else {
            format!("[仪式演化/RitualEvolution] {}", desc)
        }
    }

    pub fn ritual_absence_tick(&self, now_epoch: i64) {
        let mut absence = self.ritual.absence.lock();
        absence.detect(now_epoch); // 缺席检测有副作用 / Absence detection has side effects
    }

    pub fn ritual_absence_prompt_fragment(&self, now_epoch: i64) -> String {
        let absence = self.ritual.absence.lock();
        absence.prompt_injection(now_epoch)
    }

    pub fn ritual_emergence_prompt_fragment(&self, now_epoch: i64) -> String {
        let emergence = self.ritual.emergence.lock();
        let desc = emergence.describe(now_epoch);
        if desc.is_empty() {
            String::new()
        } else {
            format!("[仪式涌现/RitualEmergence] {}", desc)
        }
    }

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
            let ordinal = self.relationship.read().current_stage().ordinal();
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
            let ordinal = self.relationship.read().current_stage().ordinal();
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
            let ordinal = self.relationship.read().current_stage().ordinal();
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
