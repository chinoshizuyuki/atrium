// ! 脉冲引擎 / Pulse Engine
// ! 检测情绪脉冲、生成混沌脉冲、计算组合效应

use super::shock_absorber::ShockAbsorber;
use super::types::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ── 2.2 PulseEngine — 脉冲引擎 ──

/// 脉冲引擎配置 / Pulse Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulseConfig {
    pub min_pad_change: f32,
    pub max_active_pulses: usize,
    pub uncaused_prob: f64,
    pub uncaused_max_intensity: f64,
    pub rebound_window_secs: i64,
}

impl Default for PulseConfig {
    fn default() -> Self {
        Self {
            min_pad_change: 0.3,
            max_active_pulses: 10,
            uncaused_prob: 0.02,
            uncaused_max_intensity: 0.05,
            rebound_window_secs: 600,
        }
    }
}

/// 脉冲引擎 / Pulse Engine — 检测和生成情绪脉冲
#[derive(Debug, Clone)]
pub struct PulseEngine {
    pub config: PulseConfig,
    pub active_pulses: Vec<ChaoticPulse>,
    pub shock_absorber: ShockAbsorber,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl PulseEngine {
    pub fn new(config: PulseConfig) -> Self {
        Self {
            config,
            active_pulses: Vec::new(),
            shock_absorber: ShockAbsorber::default(),
            next_id: 1,
        }
    }

    /// 检测脉冲 / Detect pulse from PAD change
    pub fn detect(
        &mut self,
        pad_before: &[f32; 3],
        pad_after: &[f32; 3],
        trigger: PulseTrigger,
        now: i64,
    ) -> Option<ChaoticPulse> {
        let dp = pad_after[0] - pad_before[0];
        let da = pad_after[1] - pad_before[1];
        let dd = pad_after[2] - pad_before[2];
        let dist = (dp * dp + da * da + dd * dd).sqrt();
        if dist < self.config.min_pad_change {
            return None;
        }
        let kind = if dp < -0.2 && da > 0.2 && dd < 0.0 {
            PulseKind::Startle
        } else if dp > 0.2 && da > 0.2 {
            PulseKind::JoyBurst
        } else if dp < -0.2 && da > 0.3 && dd > 0.1 {
            PulseKind::AngerFlash
        } else if dp < -0.3 && da < -0.1 {
            PulseKind::SadnessSurge
        } else if da > 0.3 && dd < -0.2 {
            PulseKind::FearSpike
        } else {
            PulseKind::Startle
        };
        let intensity = (dist as f64).min(1.0);
        let pad_impulse = [dp, da, dd];
        let decay_curve = match kind {
            PulseKind::SadnessSurge | PulseKind::AngerFlash => DecayCurve::slow_power_law(),
            PulseKind::EmotionalRebound => DecayCurve::oscillating(),
            _ => DecayCurve::default_exponential(),
        };
        let mut pulse = ChaoticPulse {
            id: self.next_id,
            kind,
            intensity,
            pad_impulse,
            duration_secs: 300.0,
            decay_curve,
            trigger,
            timestamp: now,
            absorbed: false,
            residual_intensity: intensity,
        };
        self.next_id += 1;
        let _result = self.shock_absorber.absorb(&mut pulse, now);
        if pulse.residual_intensity > 0.01 {
            self.active_pulses.push(pulse.clone());
            if self.active_pulses.len() > self.config.max_active_pulses {
                self.active_pulses.remove(0);
            }
            Some(pulse)
        } else {
            None
        }
    }

    /// 生成无因波动 / Generate uncaused fluctuation
    ///
    /// 注入式随机源 — 调用方控制随机源，支持确定性回放。
    /// Injectable RNG — caller controls random source, enabling deterministic replay.
    pub fn maybe_fluctuate(&mut self, now: i64, rng: &mut impl Rng) -> Option<ChaoticPulse> {
        if rng.gen::<f64>() >= self.config.uncaused_prob {
            return None;
        }
        let intensity = rng.gen::<f64>() * self.config.uncaused_max_intensity;
        let p_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let a_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let d_noise = (rng.gen::<f64>() * 2.0 - 1.0) * 0.05;
        let pulse = ChaoticPulse {
            id: self.next_id,
            kind: PulseKind::UncausedFluctuation,
            intensity,
            pad_impulse: [p_noise as f32, a_noise as f32, d_noise as f32],
            duration_secs: 60.0,
            decay_curve: DecayCurve::default_exponential(),
            trigger: PulseTrigger {
                source: PulseSource::Spontaneous,
                signal: "uncaused_fluctuation".to_string(),
                baseline_pad: [0.0, 0.0, 0.0],
            },
            timestamp: now,
            absorbed: true,
            residual_intensity: intensity,
        };
        self.next_id += 1;
        self.active_pulses.push(pulse.clone());
        Some(pulse)
    }

    /// 计算所有活跃脉冲的叠加效果 / Compute combined pulse effect
    pub fn combined_effect(&self, now: i64) -> [f32; 3] {
        let mut pad = [0.0f32; 3];
        for pulse in &self.active_pulses {
            let elapsed = (now - pulse.timestamp) as f64;
            let factor = pulse.decay_curve.evaluate(elapsed) as f32;
            pad[0] += pulse.pad_impulse[0] * factor;
            pad[1] += pulse.pad_impulse[1] * factor;
            pad[2] += pulse.pad_impulse[2] * factor;
        }
        pad[0] = pad[0].clamp(-0.3, 0.3);
        pad[1] = pad[1].clamp(-0.3, 0.3);
        pad[2] = pad[2].clamp(-0.3, 0.3);
        pad
    }

    /// Tick — 衰减所有活跃脉冲 / Tick — decay all active pulses
    pub fn tick(&mut self, now: i64) {
        self.active_pulses.retain(|p| {
            let elapsed = (now - p.timestamp) as f64;
            let remaining = p.intensity * p.decay_curve.evaluate(elapsed);
            remaining > 0.01
        });
    }
}

impl Default for PulseEngine {
    fn default() -> Self {
        Self::new(PulseConfig::default())
    }
}
