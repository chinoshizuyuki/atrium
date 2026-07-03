// SPDX-License-Identifier: MIT

//! 好奇心内驱力引擎 — Curiosity drive: intrinsic motivation that accumulates during solitude
//! and releases during interaction, producing PAD emotional signatures.
//!
//! 核心理念：好奇心是数字生命的内在状态，不是外部触发的。
//! 独处时好奇积累，交互时好奇释放，关键词触发好奇尖峰。

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ═══════════════════════════════════════════════════════════════════════════
// 配置 — Configuration
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇心内驱力配置 / Curiosity drive configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityDriveConfig {
    /// 独处积累速率（每小时）/ Accumulation rate per hour during solitude.
    pub accumulation_rate: f32,
    /// 交互释放量（每次）/ Release amount per interaction.
    pub release_amount: f32,
    /// 尖峰队列上限 / Max spike topics to keep.
    pub max_spikes: usize,
    /// PAD 签名 — 愉悦调制 / PAD signature: pleasure modulation.
    pub pad_pleasure: f32,
    /// PAD 签名 — 激活调制 / PAD signature: arousal modulation.
    pub pad_arousal: f32,
    /// PAD 签名 — 支配调制 / PAD signature: dominance modulation.
    pub pad_dominance: f32,
}

impl Default for CuriosityDriveConfig {
    fn default() -> Self {
        Self {
            accumulation_rate: 0.05, // 每小时 +0.05
            release_amount: 0.15,    // 每次交互 -0.15
            max_spikes: 5,
            pad_pleasure: 0.15,   // 好奇是轻微愉悦的
            pad_arousal: 0.35,    // 好奇是高激活的
            pad_dominance: -0.20, // 不知道答案，支配度降低
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 好奇心尖峰 — Curiosity Spike
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇心尖峰 — 对某主题的瞬时好奇增强 / A curiosity spike for a specific topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriositySpike {
    /// 主题描述 / Topic description
    pub topic: String,
    /// 尖峰强度 [0, 1] / Spike intensity
    pub intensity: f32,
    /// 尖峰时间戳 / Spike timestamp
    pub timestamp: i64,
}

// ═══════════════════════════════════════════════════════════════════════════
// 好奇心内驱力引擎 — Curiosity Drive Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 好奇心内驱力引擎 — 管理内驱力水平、尖峰队列和 PAD 签名
/// Curiosity drive engine — Manages drive level, spike queue, and PAD signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityDrive {
    /// 当前内驱力水平 [0, 1] / Current drive level.
    pub drive_level: f32,
    /// 尖峰队列 / Spike queue
    pub spikes: VecDeque<CuriositySpike>,
    /// 上次交互时间戳 / Last interaction timestamp.
    pub last_interaction_at: i64,
    /// 配置 / Configuration
    pub config: CuriosityDriveConfig,
}

impl CuriosityDrive {
    /// 创建默认配置的引擎 / Create with default config.
    pub fn default_new() -> Self {
        Self::new(CuriosityDriveConfig::default())
    }

    /// 创建指定配置的引擎 / Create with custom config.
    pub fn new(config: CuriosityDriveConfig) -> Self {
        Self {
            drive_level: 0.0,
            spikes: VecDeque::new(),
            last_interaction_at: 0,
            config,
        }
    }

    /// 独处积累 — 根据距上次交互的时间差积累内驱力
    /// Accumulate during solitude — Based on elapsed time since last interaction.
    pub fn accumulate(&mut self, now: i64) {
        if self.last_interaction_at == 0 {
            self.last_interaction_at = now;
            return;
        }
        let elapsed_secs = (now - self.last_interaction_at).max(0);
        let elapsed_hours = elapsed_secs as f32 / 3600.0;
        let gain = elapsed_hours * self.config.accumulation_rate;
        self.drive_level = (self.drive_level + gain).min(1.0);
    }

    /// 交互释放 — 用户交互时释放内驱力
    /// Release on interaction — Decreases drive when user interacts.
    pub fn release(&mut self, now: i64) {
        self.drive_level = (self.drive_level - self.config.release_amount).max(0.0);
        self.last_interaction_at = now;
    }

    /// 添加好奇心尖峰 — 对某主题的瞬时好奇增强
    /// Add a curiosity spike — Instant curiosity boost for a topic.
    pub fn spike(&mut self, topic: &str, intensity: f32, timestamp: i64) {
        // 限队列长度 / Trim queue to max size
        while self.spikes.len() >= self.config.max_spikes {
            self.spikes.pop_front();
        }
        self.spikes.push_back(CuriositySpike {
            topic: topic.to_string(),
            intensity: intensity.clamp(0.0, 1.0),
            timestamp,
        });
        // 尖峰也提升内驱力 / Spike also boosts drive level
        self.drive_level = (self.drive_level + intensity * 0.3).min(1.0);
    }

    /// 返回当前 PAD 调制 — 好奇心的情感签名
    /// Return current PAD modulation — The emotional signature of curiosity.
    pub fn pad_signature(&self) -> (f32, f32, f32) {
        let factor = self.drive_level;
        (
            self.config.pad_pleasure * factor,
            self.config.pad_arousal * factor,
            self.config.pad_dominance * factor,
        )
    }

    /// 调制追问触发概率 — 高内驱力 → 概率放大
    /// Modulate follow-up trigger probability — High drive amplifies probability.
    pub fn modulate_probability(&self, base_probability: f32) -> f32 {
        // 内驱力 [0, 1] → 调制系数 [0.5, 1.5]
        let modulation = 0.5 + self.drive_level * 1.0;
        (base_probability * modulation).min(1.0)
    }

    /// 获取当前尖峰强度总和 / Get total spike intensity.
    pub fn total_spike_intensity(&self) -> f32 {
        self.spikes
            .iter()
            .map(|s| s.intensity)
            .sum::<f32>()
            .min(1.0)
    }

    /// 清理过期尖峰 — 移除超过 max_age_secs 的尖峰
    /// Prune expired spikes — Remove spikes older than max_age_secs.
    pub fn prune_spikes(&mut self, now: i64, max_age_secs: i64) {
        self.spikes.retain(|s| now - s.timestamp < max_age_secs);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 序列化辅助 — Serialization Helper
// ═══════════════════════════════════════════════════════════════════════════

/// 用于 sled 持久化的序列化版本 / Serializable version for sled persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCuriosityDrive {
    pub drive_level: f32,
    pub spikes: Vec<(String, f32, i64)>,
    pub last_interaction_at: i64,
}

impl From<&CuriosityDrive> for SerializableCuriosityDrive {
    fn from(d: &CuriosityDrive) -> Self {
        Self {
            drive_level: d.drive_level,
            spikes: d
                .spikes
                .iter()
                .map(|s| (s.topic.clone(), s.intensity, s.timestamp))
                .collect(),
            last_interaction_at: d.last_interaction_at,
        }
    }
}

impl SerializableCuriosityDrive {
    /// 反序列化为 CuriosityDrive / Deserialize back to CuriosityDrive.
    pub fn to_drive(&self, config: CuriosityDriveConfig) -> CuriosityDrive {
        let mut spikes = VecDeque::new();
        for (topic, intensity, ts) in &self.spikes {
            spikes.push_back(CuriositySpike {
                topic: topic.clone(),
                intensity: *intensity,
                timestamp: *ts,
            });
        }
        CuriosityDrive {
            drive_level: self.drive_level,
            spikes,
            last_interaction_at: self.last_interaction_at,
            config,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 单元测试 — Unit Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_init() {
        let d = CuriosityDrive::default_new();
        assert_eq!(d.drive_level, 0.0);
        assert!(d.spikes.is_empty());
    }

    #[test]
    fn test_accumulate_increases() {
        let mut d = CuriosityDrive::default_new();
        d.accumulate(1); // init baseline (non-zero skips first-call guard)
        d.accumulate(3600); // 1 hour later
        assert!(
            d.drive_level > 0.0,
            "drive should increase: {}",
            d.drive_level
        );
    }

    #[test]
    fn test_release_decreases() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 0.5;
        d.release(100);
        assert!(
            d.drive_level < 0.5,
            "drive should decrease: {}",
            d.drive_level
        );
    }

    #[test]
    fn test_accumulate_clamps_to_one() {
        let mut d = CuriosityDrive::default_new();
        d.accumulate(1); // init baseline (non-zero skips first-call guard)
        d.accumulate(999_999_999); // very long time
        assert_eq!(d.drive_level, 1.0);
    }

    #[test]
    fn test_release_clamps_to_zero() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 0.01;
        d.release(100);
        assert_eq!(d.drive_level, 0.0);
    }

    #[test]
    fn test_spike_adds_and_limits() {
        let mut d = CuriosityDrive::default_new();
        for i in 0..10 {
            d.spike(&format!("topic{}", i), 0.5, i);
        }
        assert_eq!(d.spikes.len(), 5, "should limit to max_spikes=5");
    }

    #[test]
    fn test_pad_signature() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 1.0;
        let (p, a, dom) = d.pad_signature();
        assert!(p > 0.0, "pleasure should be positive");
        assert!(a > 0.0, "arousal should be positive");
        assert!(dom < 0.0, "dominance should be negative");
    }

    #[test]
    fn test_long_solitude_saturates() {
        let mut d = CuriosityDrive::default_new();
        d.accumulate(1); // init baseline (non-zero skips first-call guard)
        d.accumulate(86400 * 30); // 30 days
        assert_eq!(d.drive_level, 1.0);
    }

    #[test]
    fn test_short_interaction_small_drop() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 0.5;
        d.release(100);
        assert!((d.drive_level - 0.35).abs() < 0.01, "got {}", d.drive_level);
    }

    #[test]
    fn test_modulate_high_drive() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 1.0;
        let p = d.modulate_probability(0.5);
        assert!(p > 0.5, "high drive should amplify: {}", p);
    }

    #[test]
    fn test_modulate_low_drive() {
        let mut d = CuriosityDrive::default_new();
        d.drive_level = 0.0;
        let p = d.modulate_probability(0.5);
        assert!((p - 0.25).abs() < 0.01, "low drive should reduce: {}", p);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut d = CuriosityDrive::default_new();
        d.spike("考研", 0.8, 1000);
        d.drive_level = 0.7; // set after spike to avoid spike's boost
        let s = SerializableCuriosityDrive::from(&d);
        let d2 = s.to_drive(CuriosityDriveConfig::default());
        assert!((d2.drive_level - 0.7).abs() < 0.01);
        assert_eq!(d2.spikes.len(), 1);
        assert_eq!(d2.spikes[0].topic, "考研");
    }

    #[test]
    fn test_spike_overflow_evicts_oldest() {
        let mut d = CuriosityDrive::default_new();
        d.spike("first", 0.5, 100);
        for i in 1..6 {
            d.spike(&format!("t{}", i), 0.5, 100 + i);
        }
        assert_eq!(d.spikes.front().unwrap().topic, "t1");
    }

    #[test]
    fn test_accumulate_release_stability() {
        let mut d = CuriosityDrive::default_new();
        for i in 0..100 {
            d.accumulate(i * 3600);
            d.release(i * 3600 + 1);
        }
        assert!(d.drive_level >= 0.0 && d.drive_level <= 1.0);
    }
}
