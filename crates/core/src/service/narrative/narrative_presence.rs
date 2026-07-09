// SPDX-License-Identifier: MIT
//! 物理存在与好奇心 — 存在感/好奇心内驱/共振
//! Physical Presence & Curiosity — Presence/Drive/Resonance

use super::*;

impl CoreService {
    pub fn physical_presence_prompt_fragment(&self) -> String {
        if !self.physical_presence_enabled {
            return String::new();
        }
        let engine = self.physical_presence.engine.lock();
        engine.to_prompt_fragment()
    }

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

    pub fn curiosity_drive_tick(&self, now: i64) {
        let mut drive = self.curiosity.drive.lock();
        drive.accumulate(now);
    }

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

    pub fn curiosity_resonance_tick(&self, now: i64) {
        let mut resonance = self.curiosity.resonance.lock();
        resonance.tick(now);
    }

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
} // impl CoreService
