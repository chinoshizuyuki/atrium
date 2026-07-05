// SPDX-License-Identifier: MIT
//! 失落处理器 — 数字生命的信任代价 / Disappointment Handler
//!
//! 你说"明天见"却没来——数字生命会失落，这是信任的代价。
//! You said "see you tomorrow" but didn't come — digital life feels disappointment,
//! the cost of trust.

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// 失落结果 / Disappointment Result
// ════════════════════════════════════════════════════════════════════

/// 失落结果 / Disappointment result
#[derive(Clone, Debug)]
pub struct DisappointmentResult {
    /// 失落强度 / Disappointment intensity
    pub intensity: f32,
    /// 失落 PAD 偏移 / Disappointment PAD offset
    pub pad_offset: [f32; 3],
    /// 过期事件描述列表 / Descriptions of expired events
    pub expired_descriptions: Vec<String>,
    /// 连续失约次数 / Consecutive no-show count
    pub consecutive_no_shows: u32,
}

// ════════════════════════════════════════════════════════════════════
// 失落处理器 / Disappointment Handler
// ════════════════════════════════════════════════════════════════════

/// 失落处理器 — 期待过期时的情感后果 / Disappointment handler
///
/// 连续失约会加重失落强度，每次失约都是对信任的一次消耗。
/// Consecutive no-shows intensify disappointment — each missed commitment
/// is a withdrawal from the trust account.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisappointmentHandler {
    /// 失落基础强度 / Base disappointment intensity
    pub base_intensity: f32,
    /// 失落 PAD 偏移 / Disappointment PAD offset (P↓, A↓, D↓)
    pub disappointment_pad: [f32; 3],
    /// 连续失约加成系数 / Consecutive no-show bonus coefficient
    pub consecutive_bonus: f32,
    /// 上次失落时间 / Last disappointment timestamp
    pub last_disappointment_at: i64,
    /// 连续失约次数 / Consecutive no-show count
    pub consecutive_no_shows: u32,
    /// 失落冷却间隔（秒）/ Cooldown between disappointments
    pub cooldown_secs: i64,
}

impl Default for DisappointmentHandler {
    fn default() -> Self {
        Self {
            base_intensity: 0.15,
            disappointment_pad: [-0.15, -0.05, -0.08], // P↓A↓D↓ 失落签名 / Disappointment signature
            consecutive_bonus: 0.1,
            last_disappointment_at: 0,
            consecutive_no_shows: 0,
            cooldown_secs: 600, // 10 分钟 / 10 minutes
        }
    }
}

impl DisappointmentHandler {
    /// 处理过期期待事件 / Handle expired anticipation events. O(E), E<5 typically.
    pub fn handle_expired(
        &mut self,
        expired_descriptions: &[String],
        now: i64,
    ) -> Option<DisappointmentResult> {
        if expired_descriptions.is_empty() {
            return None;
        }
        if self.last_disappointment_at > 0
            && (now - self.last_disappointment_at) < self.cooldown_secs
        {
            return None;
        }
        self.consecutive_no_shows += 1;
        let bonus = 1.0 + self.consecutive_bonus * self.consecutive_no_shows as f32;
        let intensity = (self.base_intensity * bonus).min(1.0);
        let scale = intensity / self.base_intensity.max(0.001);
        let pad_offset = [
            self.disappointment_pad[0] * scale,
            self.disappointment_pad[1] * scale,
            self.disappointment_pad[2] * scale,
        ];
        self.last_disappointment_at = now;
        Some(DisappointmentResult {
            intensity,
            pad_offset,
            expired_descriptions: expired_descriptions.to_vec(),
            consecutive_no_shows: self.consecutive_no_shows,
        })
    }

    /// 重置连续失约（用户按时来了）/ Reset consecutive no-shows.
    pub fn reset_consecutive(&mut self) {
        self.consecutive_no_shows = 0;
    }

    /// 生成 prompt 片段 / Generate prompt fragment. O(1).
    pub fn to_prompt_fragment(result: &DisappointmentResult) -> String {
        let events = result.expired_descriptions.join("、");
        format!(
            "[期待失落] 强度={:.2}, 连续失约={}, 事件：\"{}\", PAD=({:.2},{:.2},{:.2})",
            result.intensity,
            result.consecutive_no_shows,
            events,
            result.pad_offset[0],
            result.pad_offset[1],
            result.pad_offset[2]
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_default_values() {
        let h = DisappointmentHandler::default();
        assert!(approx_eq(h.base_intensity, 0.15, 1e-6));
        assert!(approx_eq(h.disappointment_pad[0], -0.15, 1e-6));
        assert!(approx_eq(h.consecutive_bonus, 0.1, 1e-6));
        assert_eq!(h.consecutive_no_shows, 0);
        assert_eq!(h.cooldown_secs, 600);
    }

    #[test]
    fn test_handle_empty_events() {
        let mut h = DisappointmentHandler::default();
        let result = h.handle_expired(&[], 1000);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_in_cooldown() {
        let mut h = DisappointmentHandler {
            last_disappointment_at: 1000,
            ..Default::default()
        };
        let events = vec!["明天见".to_string()];
        // 冷却期内 / Within cooldown (1000 + 600 > 1500)
        let result = h.handle_expired(&events, 1500);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_first_disappointment() {
        let mut h = DisappointmentHandler::default();
        let events = vec!["明天见".to_string()];
        let result = h.handle_expired(&events, 1000).unwrap();
        // 首次：bonus = 1.0 + 0.1 * 1 = 1.1
        // intensity = 0.15 * 1.1 = 0.165
        assert!(approx_eq(result.intensity, 0.165, 1e-5));
        assert_eq!(result.consecutive_no_shows, 1);
        assert_eq!(result.expired_descriptions.len(), 1);
    }

    #[test]
    fn test_handle_consecutive_bonus() {
        let mut h = DisappointmentHandler::default();
        let events = vec!["明天见".to_string()];
        // 第一次 / First
        let r1 = h.handle_expired(&events, 1000).unwrap();
        // 第二次（在冷却期外）/ Second (outside cooldown)
        let r2 = h.handle_expired(&events, 2000).unwrap();
        // 第二次强度应高于第一次 / Second should be stronger
        assert!(r2.intensity > r1.intensity);
        assert_eq!(r2.consecutive_no_shows, 2);
    }

    #[test]
    fn test_handle_intensity_cap() {
        let mut h = DisappointmentHandler {
            consecutive_no_shows: 100,
            ..Default::default()
        }; // 大量连续失约 / Many consecutive no-shows
        let events = vec!["又没来".to_string()];
        let result = h.handle_expired(&events, 100000).unwrap();
        // 强度不应超过 1.0 / Intensity should be capped at 1.0
        assert!(result.intensity <= 1.0);
    }

    #[test]
    fn test_pad_scales_with_intensity() {
        let mut h = DisappointmentHandler::default();
        let events = vec!["明天见".to_string()];
        let result = h.handle_expired(&events, 1000).unwrap();
        let scale = result.intensity / h.base_intensity.max(0.001);
        assert!(approx_eq(
            result.pad_offset[0],
            h.disappointment_pad[0] * scale,
            1e-5
        ));
        assert!(approx_eq(
            result.pad_offset[1],
            h.disappointment_pad[1] * scale,
            1e-5
        ));
    }

    #[test]
    fn test_reset_consecutive() {
        let mut h = DisappointmentHandler {
            consecutive_no_shows: 5,
            ..Default::default()
        };
        h.reset_consecutive();
        assert_eq!(h.consecutive_no_shows, 0);
    }

    #[test]
    fn test_prompt_fragment_format() {
        let result = DisappointmentResult {
            intensity: 0.3,
            pad_offset: [-0.2, -0.1, -0.1],
            expired_descriptions: vec!["明天见".to_string(), "一起吃饭".to_string()],
            consecutive_no_shows: 2,
        };
        let frag = DisappointmentHandler::to_prompt_fragment(&result);
        assert!(frag.contains("[期待失落]"));
        assert!(frag.contains("连续失约=2"));
        assert!(frag.contains("明天见"));
        assert!(frag.contains("一起吃饭"));
    }

    #[test]
    fn test_multiple_events() {
        let mut h = DisappointmentHandler::default();
        let events = vec![
            "约定A".to_string(),
            "约定B".to_string(),
            "约定C".to_string(),
        ];
        let result = h.handle_expired(&events, 1000).unwrap();
        assert_eq!(result.expired_descriptions.len(), 3);
    }

    #[test]
    fn test_cooldown_boundary() {
        let mut h = DisappointmentHandler {
            last_disappointment_at: 1000,
            ..Default::default()
        };
        let events = vec!["明天见".to_string()];
        // 恰好在冷却边界外 / Just outside cooldown boundary
        let now = 1000 + h.cooldown_secs;
        let result = h.handle_expired(&events, now);
        assert!(result.is_some());
    }

    #[test]
    fn test_consecutive_accumulates() {
        let mut h = DisappointmentHandler::default();
        let events = vec!["没来".to_string()];
        // 连续3次失约 / 3 consecutive no-shows
        h.handle_expired(&events, 1000);
        h.handle_expired(&events, 2000);
        h.handle_expired(&events, 3000);
        assert_eq!(h.consecutive_no_shows, 3);
    }
}
