// SPDX-License-Identifier: MIT
// CircadianModulator — 昼夜节律调制器 / Circadian rhythm modulator

use chrono::{Local, Timelike};

#[derive(Clone, Debug)]
pub struct CircadianModulator {
    pub morning_peak: f32,
    pub evening_peak: f32,
    pub morning_sigma: f32,
    pub evening_sigma: f32,
    pub intensity: f32,
    pub timezone_offset: i32,
    pub active_hours: (u32, u32),
}

impl Default for CircadianModulator {
    fn default() -> Self {
        Self {
            morning_peak: 10.0,
            evening_peak: 18.0,
            morning_sigma: 2.0,
            evening_sigma: 2.5,
            intensity: 0.8,
            timezone_offset: 8,
            active_hours: (7, 23),
        }
    }
}

impl CircadianModulator {
    /// 计算当前小时的 PAD 偏移量
    pub fn rhythm_offset(&self, hour: u32) -> [f32; 3] {
        let h = hour as f32;

        // 双峰高斯：上午 + 傍晚
        let morning = gaussian(h, self.morning_peak, self.morning_sigma);
        let evening = gaussian(h, self.evening_peak, self.evening_sigma);
        let combined = (morning.max(evening)) * self.intensity;

        // 夜间（活跃时段外）：低唤醒、微负情绪
        if hour < self.active_hours.0 || hour >= self.active_hours.1 {
            return [
                -0.05 * self.intensity,
                -0.1 * self.intensity,
                -0.02 * self.intensity,
            ];
        }

        // P = combined（高峰更正，低谷更负）
        // A = 正偏移（高能量时段唤醒度上升）
        // D = 轻微正偏移（高能量时段掌控感上升）
        let p = (combined - 0.3) * 0.1;
        let a = combined * 0.15;
        let d = combined * 0.05;

        [p, a, d]
    }

    /// 获取当前本地小时
    pub fn current_hour(&self) -> u32 {
        let now = Local::now();
        ((now.hour() as i32 + self.timezone_offset).rem_euclid(24)) as u32
    }
}

pub fn gaussian(x: f32, mu: f32, sigma: f32) -> f32 {
    let d = x - mu;
    (-(d * d) / (2.0 * sigma * sigma)).exp()
}
