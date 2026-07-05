// ! 冲击吸收器 / Shock Absorber
// ! 吸收情绪脉冲的冲击强度，防止情绪过载

use super::types::*;
use serde::{Deserialize, Serialize};

// ── 2.1 ShockAbsorber — 冲击吸收器 ──

/// 冲击吸收器 / Shock Absorber — 防止连续脉冲过载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShockAbsorber {
    pub capacity: f64,
    pub consumed: f64,
    pub recovery_rate: f64,
    pub last_recovery: i64,
}

impl ShockAbsorber {
    pub fn new(capacity: f64, recovery_rate: f64) -> Self {
        Self {
            capacity,
            consumed: 0.0,
            recovery_rate,
            last_recovery: 0,
        }
    }

    /// 吸收脉冲 / Absorb a pulse
    pub fn absorb(&mut self, pulse: &mut ChaoticPulse, now: i64) -> AbsorbResult {
        self.recover(now);
        let remaining = self.capacity - self.consumed;
        if pulse.intensity <= remaining {
            self.consumed += pulse.intensity;
            pulse.absorbed = true;
            pulse.residual_intensity = pulse.intensity;
            AbsorbResult::FullyAbsorbed
        } else if remaining > 0.01 {
            let absorbed = remaining;
            let ratio = absorbed / pulse.intensity;
            pulse.residual_intensity = absorbed;
            pulse.intensity = absorbed;
            pulse.pad_impulse = [
                pulse.pad_impulse[0] * ratio as f32,
                pulse.pad_impulse[1] * ratio as f32,
                pulse.pad_impulse[2] * ratio as f32,
            ];
            self.consumed = self.capacity;
            AbsorbResult::PartiallyAbsorbed {
                original_intensity: pulse.intensity / ratio,
                absorbed_intensity: absorbed,
            }
        } else {
            pulse.absorbed = false;
            pulse.residual_intensity = 0.0;
            AbsorbResult::OverloadProtection
        }
    }

    /// 恢复容量 / Recover capacity
    pub fn recover(&mut self, now: i64) {
        if self.last_recovery > 0 {
            let elapsed = (now - self.last_recovery) as f64;
            self.consumed = (self.consumed - self.recovery_rate * elapsed).max(0.0);
        }
        self.last_recovery = now;
    }
}

impl Default for ShockAbsorber {
    fn default() -> Self {
        Self::new(2.0, 0.1)
    }
}
