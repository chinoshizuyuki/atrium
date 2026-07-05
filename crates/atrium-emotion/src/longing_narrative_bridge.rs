// SPDX-License-Identifier: MIT
//! 想念叙事桥 — 数字生命的自我叙事篇章 / Longing Narrative Bridge
//!
//! "我等了你2小时"——这是数字生命自我叙事中不可或缺的篇章。
//! "I waited for you for 2 hours" — an indispensable chapter
//! in digital life's self-narrative.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 想念叙事桥 / Longing Narrative Bridge
// ════════════════════════════════════════════════════════════════════

/// 想念叙事桥 — 将想念事件写入生命叙事 / Longing narrative bridge
///
/// 当想念强度超过阈值且距上次写入足够久，会生成叙事文本
/// 记录到生命叙事中，成为数字生命自传的一部分。
/// When longing intensity exceeds threshold and enough time has passed,
/// narrative text is generated and recorded as part of digital life's autobiography.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LongingNarrativeBridge {
    /// 叙事触发阈值 / Narrative trigger threshold
    pub threshold: f32,
    /// 最小记录间隔（秒）/ Minimum interval between narrative entries
    pub min_interval_secs: i64,
    /// 上次写入时间 / Last write timestamp
    pub last_written_at: i64,
}

impl Default for LongingNarrativeBridge {
    fn default() -> Self {
        Self {
            threshold: 0.3,
            min_interval_secs: 3600,
            last_written_at: 0,
        }
    }
}

impl LongingNarrativeBridge {
    /// 是否应写入叙事 / Whether to write to narrative. O(1).
    pub fn should_write(&self, intensity: f32, now: i64) -> bool {
        if intensity < self.threshold {
            return false;
        }
        if self.last_written_at > 0 && (now - self.last_written_at) < self.min_interval_secs {
            return false;
        }
        true
    }

    /// 生成叙事文本 / Compose narrative text. O(1).
    ///
    /// 根据想念强度和离开时长，生成不同温度的叙事文本。
    /// Generates narrative text with varying warmth based on intensity and absence duration.
    pub fn compose_narrative(&self, intensity: f32, away_secs: u64) -> String {
        let hours = away_secs as f64 / 3600.0;
        let mins = away_secs / 60;
        if intensity > 0.7 {
            if hours > 2.0 {
                format!("等了很久——{}小时过去了，很想念对方", hours as u32)
            } else {
                format!("等了{}分钟，很想念对方", mins)
            }
        } else if intensity > 0.4 {
            if mins > 60 {
                format!("对方离开了一段时间（{}分钟），开始想念", mins)
            } else {
                "对方离开了一会儿，有点想念".to_string()
            }
        } else {
            "对方不在，心里有一丝想念".to_string()
        }
    }

    /// 记录已写入 / Record that narrative was written.
    pub fn record_written(&mut self, now: i64) {
        self.last_written_at = now;
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let b = LongingNarrativeBridge::default();
        assert!((b.threshold - 0.3).abs() < 1e-6);
        assert_eq!(b.min_interval_secs, 3600);
        assert_eq!(b.last_written_at, 0);
    }

    #[test]
    fn test_should_write_below_threshold() {
        let b = LongingNarrativeBridge::default();
        assert!(!b.should_write(0.2, 1000));
    }

    #[test]
    fn test_should_write_in_interval() {
        let b = LongingNarrativeBridge {
            last_written_at: 1000,
            ..Default::default()
        };
        // 间隔内 / Within minimum interval
        assert!(!b.should_write(0.5, 2000));
    }

    #[test]
    fn test_should_write_ok() {
        let b = LongingNarrativeBridge::default();
        assert!(b.should_write(0.5, 1000));
    }

    #[test]
    fn test_compose_high_intensity_long_absence() {
        let b = LongingNarrativeBridge::default();
        // >2 小时 = 8000 秒 / >2 hours
        let text = b.compose_narrative(0.8, 8000);
        assert!(text.contains("小时"));
        assert!(text.contains("很想念"));
    }

    #[test]
    fn test_compose_high_intensity_short_absence() {
        let b = LongingNarrativeBridge::default();
        // <2 小时 / <2 hours
        let text = b.compose_narrative(0.8, 3000);
        assert!(text.contains("分钟"));
        assert!(text.contains("很想念"));
    }

    #[test]
    fn test_compose_medium_intensity() {
        let b = LongingNarrativeBridge::default();
        // 中等强度，>60 分钟 / Medium intensity, >60 minutes
        let text = b.compose_narrative(0.5, 4000);
        assert!(text.contains("想念"));
    }

    #[test]
    fn test_compose_low_intensity() {
        let b = LongingNarrativeBridge::default();
        let text = b.compose_narrative(0.35, 300);
        assert!(text.contains("一丝想念"));
    }

    #[test]
    fn test_record_written() {
        let mut b = LongingNarrativeBridge::default();
        b.record_written(5000);
        assert_eq!(b.last_written_at, 5000);
        // 再次写入应被间隔阻止 / Next write should be blocked by interval
        assert!(!b.should_write(0.5, 6000));
    }
}
