// SPDX-License-Identifier: MIT
//! 时序映射器 — 回复节奏与时间感
//! TimingMapper — Reply rhythm and temporal feel.
//!
//! 回复不是瞬间弹出的，而是有节奏的：
//! - 悲伤时回复慢，像在慢慢组织语言
//! - 兴奋时回复快，想到什么说什么
//! - 犹豫时有停顿，像在斟酌
//! - 初识阶段回复更正式、节奏更稳

use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;
use crate::style_modulator::LinguisticProfile;

// ════════════════════════════════════════════════════════════════════
// TimingProfile — 时序特征
// ════════════════════════════════════════════════════════════════════

/// 时序特征 — 控制回复的时间节奏
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimingProfile {
    /// 打字延迟因子 — 1.0=正常速度，0.5=两倍速，2.0=半速
    /// 悲伤→1.5-2.0，兴奋→0.5-0.8，平静→1.0
    pub typing_delay_factor: f32,

    /// 句间停顿（ms）— 句与句之间的思考时间
    /// 悲伤→1200ms，兴奋→300ms，平静→600ms
    pub inter_sentence_pause_ms: f32,

    /// 思考停顿概率 — 回复前"思考"一下的概率
    /// 复杂问题→高，简单寒暄→低
    pub thinking_pause_prob: f32,

    /// 思考停顿时长（ms）
    pub thinking_pause_duration_ms: f32,

    /// 犹豫标记概率 — "嗯..." "让我想想..."
    pub hesitation_prob: f32,

    /// 分段发送概率 — 长回复拆成多条消息的概率
    pub segmented_send_prob: f32,

    /// 消息段间隔（ms）— 分段发送时各段之间的间隔
    pub segment_interval_ms: f32,

    /// 回复紧迫感 0-1 — 高紧迫→快速回复，低紧迫→可以等
    pub urgency: f32,
}

impl TimingProfile {
    /// 默认（中性）时序特征
    pub fn neutral() -> Self {
        Self {
            typing_delay_factor: 1.0,
            inter_sentence_pause_ms: 600.0,
            thinking_pause_prob: 0.3,
            thinking_pause_duration_ms: 1500.0,
            hesitation_prob: 0.05,
            segmented_send_prob: 0.2,
            segment_interval_ms: 2000.0,
            urgency: 0.5,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// TimingMapper — 时序映射器
// ════════════════════════════════════════════════════════════════════

/// 时序映射器 — 从 PAD + LinguisticProfile + 关系阶段 → TimingProfile
pub struct TimingMapper;

impl TimingMapper {
    /// 从 PAD、LinguisticProfile 和关系阶段映射到时序特征
    pub fn map(
        pad: [f32; 3],
        lp: &LinguisticProfile,
        relationship: &RelationshipStage,
    ) -> TimingProfile {
        let p = pad[0];
        let a = pad[1];
        let _d = pad[2];

        // ── 打字延迟 ──
        // 悲伤→慢，兴奋→快，焦虑→偏快但犹豫
        let typing_delay_factor = 1.0 - p * 0.3 - a * 0.2 + lp.silence_tendency * 0.5;

        // ── 句间停顿 ──
        // 悲伤→长（在组织语言），兴奋→短，中性→约600ms
        let inter_sentence_pause_ms =
            500.0 - a * 200.0 + (0.5 - p) * 200.0 + lp.self_repair_tendency * 500.0;

        // ── 思考停顿概率 ──
        // 复杂问题/低确定性→高，简单寒暄→低
        let thinking_pause_prob =
            0.3 + (1.0 - lp.certainty_marking) * 0.2 + lp.self_repair_tendency * 0.3;

        // ── 思考停顿时长 ──
        // 悲伤/犹豫→长
        let thinking_pause_duration_ms =
            1500.0 + (1.0 - p) * 500.0 + lp.self_repair_tendency * 1000.0;

        // ── 犹豫标记概率 ──
        // 低确定性/自我修复→高
        let hesitation_prob =
            0.05 + (1.0 - lp.certainty_marking) * 0.15 + lp.self_repair_tendency * 0.3;

        // ── 分段发送 ──
        // 长回复+悲伤/犹豫→分段发送（像在慢慢说）
        let segmented_send_prob = 0.2 + lp.silence_tendency * 0.3 + lp.self_repair_tendency * 0.2;

        // ── 消息段间隔 ──
        // 悲伤→间隔长，兴奋→间隔短
        let segment_interval_ms = 2000.0 - a * 500.0 + (1.0 - p) * 800.0;

        // ── 紧迫感 ──
        // 用户在等/兴奋→高紧迫，悲伤/思考→低紧迫
        let urgency = 0.5 + a * 0.2 + p * 0.1 - lp.silence_tendency * 0.2;

        // ── 关系阶段修正 ──
        let (typing_mod, pause_mod, urgency_mod) = match relationship {
            RelationshipStage::Acquaintance { .. } => (1.1, 1.2, -0.1), // 初识：更慢更稳
            RelationshipStage::Familiar { .. } => (1.0, 1.0, 0.0),
            RelationshipStage::Trusted { .. } => (0.95, 0.9, 0.05),
            RelationshipStage::Deep { .. } => (0.9, 0.8, 0.1), // 深度：更自然更快
        };

        TimingProfile {
            typing_delay_factor: (typing_delay_factor * typing_mod).clamp(0.3, 3.0),
            inter_sentence_pause_ms: (inter_sentence_pause_ms * pause_mod).clamp(100.0, 3000.0),
            thinking_pause_prob: (thinking_pause_prob).clamp(0.0, 0.8),
            thinking_pause_duration_ms: thinking_pause_duration_ms.clamp(500.0, 5000.0),
            hesitation_prob: hesitation_prob.clamp(0.0, 0.5),
            segmented_send_prob: segmented_send_prob.clamp(0.0, 0.8),
            segment_interval_ms: segment_interval_ms.clamp(500.0, 5000.0),
            urgency: (urgency + urgency_mod).clamp(0.0, 1.0),
        }
    }
}

impl TimingProfile {
    /// 计算模拟打字延迟（ms）
    ///
    /// 用于模拟"正在输入..."的效果。
    /// 不是固定延迟，而是基于回复长度和情绪状态。
    pub fn compute_typing_delay_ms(&self, text_length: usize) -> u32 {
        // 基础延迟：每个字符 50ms（正常打字速度）
        let base_delay = text_length as f32 * 50.0;

        // 乘以情绪延迟因子
        let emotional_delay = base_delay * self.typing_delay_factor;

        // 加上思考停顿
        let thinking_delay = if rand::random::<f32>() < self.thinking_pause_prob {
            self.thinking_pause_duration_ms
        } else {
            0.0
        };

        // 加上句间停顿
        let sentence_count = (text_length as f32 / 15.0).max(1.0);
        let sentence_pauses = (sentence_count - 1.0) * self.inter_sentence_pause_ms;

        let total = emotional_delay + thinking_delay + sentence_pauses;

        // 最少 300ms，最多 15 秒
        total.clamp(300.0, 15000.0) as u32
    }

    /// 决定是否分段发送
    ///
    /// 长回复可以拆成多条消息，更自然。
    pub fn should_segment(&self, text_length: usize) -> bool {
        // 只对较长回复考虑分段
        if text_length < 50 {
            return false;
        }
        rand::random::<f32>() < self.segmented_send_prob
    }

    /// 计算分段点
    ///
    /// 在句号/问号/感叹号处自然分段。
    pub fn compute_segments(&self, text: &str) -> Vec<String> {
        if !self.should_segment(text.len()) {
            return vec![text.to_string()];
        }

        // 在标点处分段
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut char_count = 0;
        let target_segment_len = (text.len() as f32 / 2.5) as usize; // 每段约 40% 长度

        for c in text.chars() {
            current.push(c);
            char_count += 1;

            if char_count >= target_segment_len
                && (c == '。' || c == '？' || c == '！' || c == '.' || c == '?' || c == '!')
            {
                segments.push(current.trim().to_string());
                current = String::new();
                char_count = 0;
            }
        }

        if !current.trim().is_empty() {
            segments.push(current.trim().to_string());
        }

        // 如果只分出一段，不分段
        if segments.len() <= 1 {
            vec![text.to_string()]
        } else {
            segments
        }
    }

    /// 生成犹豫前缀
    ///
    /// 如 "嗯..."、"让我想想..."、"这个嘛..."
    pub fn generate_hesitation_prefix(&self) -> Option<String> {
        if rand::random::<f32>() > self.hesitation_prob {
            return None;
        }

        let prefixes = [
            "嗯...",
            "让我想想...",
            "这个嘛...",
            "怎么说呢...",
            "其实...",
        ];
        let idx = (rand::random::<f32>() * prefixes.len() as f32).floor() as usize;
        let idx = idx.min(prefixes.len() - 1);
        Some(prefixes[idx].to_string())
    }
}

// ════════════════════════════════════════════════════════════════════
// Prompt 片段生成 — 供 LLM 上下文注入
// Prompt fragment generation — for LLM context injection
// ════════════════════════════════════════════════════════════════════

impl TimingProfile {
    /// 生成 LLM prompt 片段 — 回复节奏上下文 / Generate LLM prompt fragment — timing context
    ///
    /// 将节奏参数转换为自然语言提示，注入 LLM 上下文。
    /// 让 LLM 感知自己当前的回复节奏状态，
    /// 从而让文字回复的节奏感与设计一致。
    ///
    /// # 示例输出 / Example output
    ///
    /// ```text
    /// [节奏] 打字偏慢，句间有停顿，回复分段，带犹豫前缀
    /// ```
    pub fn to_prompt_fragment(&self) -> String {
        let mut parts = Vec::new();

        // 打字速度描述 / Typing speed description
        // typing_delay_factor: 1.0=正常, >1.0=慢, <1.0=快
        if self.typing_delay_factor > 1.3 {
            parts.push("打字偏慢".to_string());
        } else if self.typing_delay_factor < 0.7 {
            parts.push("打字较快".to_string());
        }

        // 句间停顿描述 / Inter-sentence pause description
        if self.inter_sentence_pause_ms > 800.0 {
            parts.push("句间有停顿".to_string());
        } else if self.inter_sentence_pause_ms < 400.0 {
            parts.push("句间紧凑".to_string());
        }

        // 犹豫描述 / Hesitation description
        if self.hesitation_prob > 0.3 {
            parts.push("带犹豫前缀".to_string());
        }

        // 分段描述 / Segmentation description
        if self.segmented_send_prob > 0.5 {
            parts.push("回复分段".to_string());
        }

        // 紧迫感描述 / Urgency description
        if self.urgency > 0.6 {
            parts.push("语气紧迫".to_string());
        }

        // 思考停顿描述 / Thinking pause description
        if self.thinking_pause_prob > 0.4 {
            parts.push("先思考再回复".to_string());
        }

        if parts.is_empty() {
            "[节奏] 节奏平稳自然".to_string()
        } else {
            format!("[节奏] {}", parts.join("，"))
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn acq() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 5,
        }
    }

    fn familiar() -> RelationshipStage {
        RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        }
    }

    fn deep() -> RelationshipStage {
        RelationshipStage::Deep {
            since: 0,
            interactions: 1000,
            shared_references: 30,
            key_moments: 10,
        }
    }

    fn sad_lp() -> LinguisticProfile {
        LinguisticProfile {
            silence_tendency: 0.5,
            self_repair_tendency: 0.15,
            certainty_marking: 0.3,
            ..LinguisticProfile::neutral()
        }
    }

    fn joy_lp() -> LinguisticProfile {
        LinguisticProfile {
            silence_tendency: 0.05,
            self_repair_tendency: 0.02,
            certainty_marking: 0.6,
            ..LinguisticProfile::neutral()
        }
    }

    #[test]
    fn test_timing_neutral() {
        let lp = LinguisticProfile::neutral();
        let timing = TimingMapper::map([0.0, 0.0, 0.0], &lp, &familiar());
        assert!((timing.typing_delay_factor - 1.0).abs() < 0.3);
        assert!((timing.inter_sentence_pause_ms - 600.0).abs() < 200.0);
    }

    #[test]
    fn test_timing_sad_slow() {
        let timing = TimingMapper::map([-0.7, -0.3, -0.5], &sad_lp(), &familiar());
        // 悲伤→慢打字、长停顿
        assert!(
            timing.typing_delay_factor > 1.0,
            "sad should be slow: {}",
            timing.typing_delay_factor
        );
        assert!(
            timing.inter_sentence_pause_ms > 600.0,
            "sad should have long pauses: {}",
            timing.inter_sentence_pause_ms
        );
        assert!(
            timing.segmented_send_prob > 0.3,
            "sad should tend to segment: {}",
            timing.segmented_send_prob
        );
    }

    #[test]
    fn test_timing_joy_fast() {
        let timing = TimingMapper::map([0.7, 0.5, 0.4], &joy_lp(), &familiar());
        // 喜悦→快打字、短停顿
        assert!(
            timing.typing_delay_factor < 1.0,
            "joy should be fast: {}",
            timing.typing_delay_factor
        );
        assert!(
            timing.inter_sentence_pause_ms < 600.0,
            "joy should have short pauses: {}",
            timing.inter_sentence_pause_ms
        );
    }

    #[test]
    fn test_timing_acquaintance_slower() {
        let lp = LinguisticProfile::neutral();
        let timing_acq = TimingMapper::map([0.0, 0.0, 0.0], &lp, &acq());
        let timing_deep = TimingMapper::map([0.0, 0.0, 0.0], &lp, &deep());
        // 初识应比深度更慢更稳
        assert!(
            timing_acq.typing_delay_factor >= timing_deep.typing_delay_factor,
            "acquaintance should be slower than deep"
        );
    }

    #[test]
    fn test_timing_compute_typing_delay() {
        let timing = TimingProfile::neutral();
        let delay = timing.compute_typing_delay_ms(50);
        // 50 字符 × 50ms = 2500ms 基础，加句间停顿
        assert!(delay >= 300, "delay should be at least 300ms");
        assert!(delay <= 15000, "delay should be at most 15000ms");
    }

    #[test]
    fn test_timing_should_segment_short() {
        let timing = TimingProfile {
            segmented_send_prob: 0.8,
            ..TimingProfile::neutral()
        };
        // 短回复不应分段
        assert!(!timing.should_segment(30));
    }

    #[test]
    fn test_timing_compute_segments() {
        let timing = TimingProfile {
            segmented_send_prob: 1.0, // 强制分段
            ..TimingProfile::neutral()
        };
        let text = "今天天气真好。我想出去走走。你觉得呢？我们可以去公园。";
        let segments = timing.compute_segments(text);
        // 长文本应被分段
        assert!(!segments.is_empty());
        // 合并后应等于原文（去除空格差异）
        let rejoined = segments.join("");
        assert_eq!(rejoined, text);
    }

    #[test]
    fn test_timing_hesitation_prefix() {
        let timing = TimingProfile {
            hesitation_prob: 1.0, // 强制犹豫
            ..TimingProfile::neutral()
        };
        let prefix = timing.generate_hesitation_prefix();
        assert!(prefix.is_some(), "should generate hesitation prefix");
        assert!(!prefix.unwrap().is_empty());
    }

    #[test]
    fn test_timing_no_hesitation_when_low() {
        let timing = TimingProfile {
            hesitation_prob: 0.0, // 不犹豫
            ..TimingProfile::neutral()
        };
        let prefix = timing.generate_hesitation_prefix();
        assert!(prefix.is_none(), "should not hesitate when prob is 0");
    }

    #[test]
    fn test_timing_clamps() {
        let lp = LinguisticProfile::neutral();
        let timing = TimingMapper::map([1.0, 1.0, 1.0], &lp, &deep());
        assert!(timing.typing_delay_factor >= 0.3 && timing.typing_delay_factor <= 3.0);
        assert!(
            timing.inter_sentence_pause_ms >= 100.0 && timing.inter_sentence_pause_ms <= 3000.0
        );
        assert!(timing.thinking_pause_prob >= 0.0 && timing.thinking_pause_prob <= 0.8);
        assert!(timing.urgency >= 0.0 && timing.urgency <= 1.0);
    }

    // ── to_prompt_fragment 测试 / to_prompt_fragment tests ──

    #[test]
    fn test_timing_prompt_fragment_neutral() {
        // 中性 → 节奏平稳自然 / Neutral → steady natural rhythm
        let timing = TimingProfile::neutral();
        let frag = timing.to_prompt_fragment();
        assert!(
            frag.starts_with("[节奏]"),
            "should start with [节奏]: {}",
            frag
        );
    }

    #[test]
    fn test_timing_prompt_fragment_slow_hesitant() {
        // 慢节奏 + 高犹豫 → 打字偏慢 + 带犹豫前缀 / Slow + hesitant → slow typing + hesitation
        let timing = TimingProfile {
            typing_delay_factor: 1.8,
            inter_sentence_pause_ms: 1000.0,
            hesitation_prob: 0.5,
            segmented_send_prob: 0.8,
            urgency: 0.2,
            ..TimingProfile::neutral()
        };
        let frag = timing.to_prompt_fragment();
        assert!(frag.starts_with("[节奏]"), "should start with [节奏]");
        assert!(
            frag.contains("打字偏慢"),
            "should mention slow typing: {}",
            frag
        );
        assert!(
            frag.contains("句间有停顿"),
            "should mention pause: {}",
            frag
        );
        assert!(
            frag.contains("带犹豫前缀"),
            "should mention hesitation: {}",
            frag
        );
        assert!(
            frag.contains("回复分段"),
            "should mention segmentation: {}",
            frag
        );
    }

    #[test]
    fn test_timing_prompt_fragment_urgent() {
        // 紧迫 → 语气紧迫 / Urgent → urgent tone
        let timing = TimingProfile {
            urgency: 0.8,
            ..TimingProfile::neutral()
        };
        let frag = timing.to_prompt_fragment();
        assert!(
            frag.contains("语气紧迫"),
            "should mention urgency: {}",
            frag
        );
    }

    #[test]
    fn test_timing_prompt_fragment_thinking() {
        // 先思考再回复 / Thinking before reply
        let timing = TimingProfile {
            thinking_pause_prob: 0.6,
            ..TimingProfile::neutral()
        };
        let frag = timing.to_prompt_fragment();
        assert!(
            frag.contains("先思考再回复"),
            "should mention thinking: {}",
            frag
        );
    }
}
