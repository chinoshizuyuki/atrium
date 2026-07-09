// SPDX-License-Identifier: MIT
//! 情绪非理性系统 — 脉冲/残留/传染/混沌修正
//! Emotional Irrationality System — Pulse/Residue/Contagion/Chaos correction

use super::*;

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
} // impl CoreService
