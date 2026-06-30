// SPDX-License-Identifier: MIT
//! 韵律映射器 — 从情绪状态映射到语音韵律参数
//! ProsodyMapper — Map emotion state to speech prosody parameters.
//!
//! TTS 的韵律参数（基频、语速、能量、停顿）不是固定的，
//! 而是随情绪连续变化。同样的文字，不同情绪下韵律完全不同。

use serde::{Deserialize, Serialize};

use crate::style_modulator::LinguisticProfile;

// ════════════════════════════════════════════════════════════════════
// ProsodyParams — 语音韵律参数
// ════════════════════════════════════════════════════════════════════

/// 语音韵律参数 — 控制 TTS 输出的韵律
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProsodyParams {
    /// 基频偏移（半音）— 悲伤→-3st，喜悦→+2st，愤怒→+4st
    pub pitch_offset: f32,
    /// 基频范围（半音）— 悲伤→窄(3st)，兴奋→宽(8st)，平静→中(5st)
    pub pitch_range: f32,
    /// 语速因子 — 悲伤→0.8，兴奋→1.2，愤怒→1.3
    pub rate: f32,
    /// 能量因子 — 悲伤→0.6，愤怒→1.2，平静→1.0
    pub energy: f32,
    /// 句间停顿（ms）— 悲伤→800ms，兴奋→200ms，平静→400ms
    pub pause_duration_ms: f32,
    /// 句内停顿概率 — 犹豫时更高
    pub intra_pause_prob: f32,
    /// 音色温暖度 0-1 — 悲伤→0.3，喜悦→0.8
    pub warmth: f32,
    /// 气声量 0-1 — 亲密/悲伤→偏高
    pub breathiness: f32,
}

impl ProsodyParams {
    /// 默认（中性）韵律参数
    pub fn neutral() -> Self {
        Self {
            pitch_offset: 0.0,
            pitch_range: 5.0,
            rate: 1.0,
            energy: 1.0,
            pause_duration_ms: 400.0,
            intra_pause_prob: 0.1,
            warmth: 0.5,
            breathiness: 0.1,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ProsodyMapper — 韵律映射器
// ════════════════════════════════════════════════════════════════════

/// 韵律映射器 — 从 PAD + LinguisticProfile → ProsodyParams
pub struct ProsodyMapper;

impl ProsodyMapper {
    /// 从 PAD 和 LinguisticProfile 映射到韵律参数
    ///
    /// PAD 是主要驱动，LinguisticProfile 提供微调。
    pub fn map(pad: [f32; 3], lp: &LinguisticProfile) -> ProsodyParams {
        let p = pad[0]; // Pleasure
        let a = pad[1]; // Arousal
        let d = pad[2]; // Dominance

        // ── 基频偏移 ──
        // 悲伤→低，喜悦→高，愤怒→高（高唤醒驱动基频上升）
        let pitch_offset = p * 1.5 + a * 2.5 - d * 0.3;

        // ── 基频范围 ──
        // 兴奋→宽，悲伤→窄，平静→中
        let pitch_range = 5.0 + a * 2.0 - (1.0 - p) * 0.5;

        // ── 语速 ──
        // 兴奋/愤怒→快，悲伤/疲惫→慢；唤醒主导，愉悦微调
        let rate = 1.0 + a * 0.3 - (1.0 - p) * 0.05;

        // ── 能量 ──
        // 愤怒→高，悲伤→低；唤醒主导，愉悦微调
        let energy = 1.0 + a * 0.25 + (1.0 - p) * a.max(0.0) * 0.1 - (0.5 - p).max(0.0) * 0.15;

        // ── 句间停顿 ──
        // 悲伤→长，兴奋→短
        let pause_duration_ms = 400.0 - a * 150.0 + (1.0 - p) * 200.0;

        // ── 句内停顿概率 ──
        // 犹豫/自我修复→高
        let intra_pause_prob = 0.1 + lp.self_repair_tendency * 0.3 + lp.silence_tendency * 0.2;

        // ── 音色温暖度 ──
        // 喜悦/关爱→高，愤怒→低
        let warmth = 0.5 + p * 0.25 - a.min(0.0).abs() * 0.1 + d * 0.05;

        // ── 气声量 ──
        // 亲密/悲伤→高，愤怒→低
        let breathiness = 0.1 + (1.0 - p) * 0.15 + lp.silence_tendency * 0.2 - a.max(0.0) * 0.05;

        ProsodyParams {
            pitch_offset: pitch_offset.clamp(-5.0, 5.0),
            pitch_range: pitch_range.clamp(2.0, 10.0),
            rate: rate.clamp(0.6, 1.5),
            energy: energy.clamp(0.4, 1.4),
            pause_duration_ms: pause_duration_ms.clamp(100.0, 1200.0),
            intra_pause_prob: intra_pause_prob.clamp(0.0, 0.6),
            warmth: warmth.clamp(0.0, 1.0),
            breathiness: breathiness.clamp(0.0, 0.5),
        }
    }
}

impl ProsodyParams {
    /// 生成 SSML 韵律标记
    ///
    /// 将 ProsodyParams 转换为 SSML <prosody> 属性，
    /// 供 TTS 引擎使用。
    pub fn to_ssml_attrs(&self) -> String {
        let rate_pct = (self.rate * 100.0) as i32;
        let pitch_st = if self.pitch_offset >= 0.0 {
            format!("+{:.1}st", self.pitch_offset)
        } else {
            format!("{:.1}st", self.pitch_offset)
        };
        let energy_pct = (self.energy * 100.0) as i32;

        format!(
            "rate=\"{}%\" pitch=\"{}\" volume=\"{}%\"",
            rate_pct, pitch_st, energy_pct
        )
    }

    /// 对文本应用韵律标记
    ///
    /// 在句间插入停顿标记，调整整体韵律。
    pub fn apply_to_text(&self, text: &str) -> String {
        let prosody_attrs = self.to_ssml_attrs();

        // 在句号/问号/感叹号后插入停顿
        let pause_ms = self.pause_duration_ms as u32;
        let break_mark = format!("<break time=\"{}ms\"/>", pause_ms);

        let mut result = format!("<prosody {}>", prosody_attrs);

        // 逐句处理
        for c in text.chars() {
            result.push(c);
            if c == '。' || c == '？' || c == '！' || c == '.' || c == '?' || c == '!' {
                result.push_str(&break_mark);
            }
            // 句内停顿（逗号后，概率性）
            if (c == '，' || c == ',') && rand::random::<f32>() < self.intra_pause_prob {
                result.push_str(&format!(
                    "<break time=\"{}ms\"/>",
                    (pause_ms as f32 * 0.3) as u32
                ));
            }
        }

        result.push_str("</prosody>");
        result
    }
}

// ════════════════════════════════════════════════════════════════════
// Prompt 片段生成 — 供 LLM 上下文注入
// Prompt fragment generation — for LLM context injection
// ════════════════════════════════════════════════════════════════════

impl ProsodyParams {
    /// 生成 LLM prompt 片段 — 语音韵律上下文 / Generate LLM prompt fragment — prosody context
    ///
    /// 将韵律参数转换为自然语言提示，注入 LLM 上下文。
    /// 让 LLM 感知自己当前的语音韵律状态，
    /// 从而让文字回复风格与语音韵律保持一致。
    ///
    /// # 示例输出 / Example output
    ///
    /// ```text
    /// [韵律] 语速偏快，音调略高，句间停顿短，音色温暖
    /// ```
    pub fn to_prompt_fragment(&self) -> String {
        let mut parts = Vec::new();

        // 语速描述 / Rate description
        if self.rate > 1.15 {
            parts.push("语速偏快".to_string());
        } else if self.rate < 0.85 {
            parts.push("语速偏慢".to_string());
        }

        // 音调描述 / Pitch description
        if self.pitch_offset > 1.0 {
            parts.push("音调略高".to_string());
        } else if self.pitch_offset < -1.0 {
            parts.push("音调偏低".to_string());
        }

        // 停顿描述 / Pause description
        if self.pause_duration_ms < 300.0 {
            parts.push("句间停顿短".to_string());
        } else if self.pause_duration_ms > 600.0 {
            parts.push("句间停顿较长".to_string());
        }

        // 能量描述 / Energy description
        if self.energy > 1.1 {
            parts.push("声音有力".to_string());
        } else if self.energy < 0.8 {
            parts.push("声音轻柔".to_string());
        }

        // 音色描述 / Warmth description
        if self.warmth > 0.65 {
            parts.push("音色温暖".to_string());
        } else if self.warmth < 0.35 {
            parts.push("音色冷淡".to_string());
        }

        // 气声描述 / Breathiness description
        if self.breathiness > 0.2 {
            parts.push("带轻微气声".to_string());
        }

        if parts.is_empty() {
            "[韵律] 语速适中，音调平稳".to_string()
        } else {
            format!("[韵律] {}", parts.join("，"))
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sad_lp() -> LinguisticProfile {
        LinguisticProfile {
            silence_tendency: 0.5,
            self_repair_tendency: 0.1,
            ..LinguisticProfile::neutral()
        }
    }

    fn joy_lp() -> LinguisticProfile {
        LinguisticProfile {
            silence_tendency: 0.05,
            self_repair_tendency: 0.02,
            ..LinguisticProfile::neutral()
        }
    }

    fn angry_lp() -> LinguisticProfile {
        LinguisticProfile {
            silence_tendency: 0.05,
            self_repair_tendency: 0.02,
            ..LinguisticProfile::neutral()
        }
    }

    #[test]
    fn test_prosody_neutral() {
        let lp = LinguisticProfile::neutral();
        let prosody = ProsodyMapper::map([0.0, 0.0, 0.0], &lp);
        // 中性 PAD → 接近默认值
        assert!(prosody.pitch_offset.abs() < 0.5);
        assert!((prosody.rate - 1.0).abs() < 0.2);
        assert!((prosody.energy - 1.0).abs() < 0.2);
    }

    #[test]
    fn test_prosody_sad() {
        let prosody = ProsodyMapper::map([-0.7, -0.3, -0.5], &sad_lp());
        // 悲伤：低基频、慢语速、低能量、长停顿
        assert!(
            prosody.pitch_offset < -1.0,
            "sad should have low pitch: {}",
            prosody.pitch_offset
        );
        assert!(prosody.rate < 1.0, "sad should be slow: {}", prosody.rate);
        assert!(
            prosody.energy < 1.0,
            "sad should have low energy: {}",
            prosody.energy
        );
        assert!(
            prosody.pause_duration_ms > 500.0,
            "sad should have long pauses: {}",
            prosody.pause_duration_ms
        );
        assert!(
            prosody.pitch_range < 5.0,
            "sad should have narrow pitch range: {}",
            prosody.pitch_range
        );
    }

    #[test]
    fn test_prosody_joy() {
        let prosody = ProsodyMapper::map([0.7, 0.5, 0.4], &joy_lp());
        // 喜悦：高基频、快语速、宽音域
        assert!(
            prosody.pitch_offset > 1.0,
            "joy should have high pitch: {}",
            prosody.pitch_offset
        );
        assert!(prosody.rate > 1.0, "joy should be fast: {}", prosody.rate);
        assert!(
            prosody.pitch_range > 5.0,
            "joy should have wide pitch range: {}",
            prosody.pitch_range
        );
        assert!(
            prosody.warmth > 0.5,
            "joy should have warm tone: {}",
            prosody.warmth
        );
    }

    #[test]
    fn test_prosody_anger() {
        let prosody = ProsodyMapper::map([-0.6, 0.7, 0.6], &angry_lp());
        // 愤怒：高基频、快语速、高能量
        assert!(
            prosody.pitch_offset > 0.0,
            "anger should have high pitch: {}",
            prosody.pitch_offset
        );
        assert!(prosody.rate > 1.0, "anger should be fast: {}", prosody.rate);
        assert!(
            prosody.energy > 1.0,
            "anger should have high energy: {}",
            prosody.energy
        );
    }

    #[test]
    fn test_prosody_ssml_attrs() {
        let prosody = ProsodyMapper::map([0.3, 0.1, 0.0], &LinguisticProfile::neutral());
        let attrs = prosody.to_ssml_attrs();
        assert!(attrs.contains("rate="));
        assert!(attrs.contains("pitch="));
        assert!(attrs.contains("volume="));
    }

    #[test]
    fn test_prosody_apply_to_text() {
        let prosody = ProsodyMapper::map([0.0, 0.0, 0.0], &LinguisticProfile::neutral());
        let result = prosody.apply_to_text("你好。我很开心。");
        assert!(result.contains("<prosody"));
        assert!(result.contains("</prosody>"));
        assert!(result.contains("<break"));
    }

    #[test]
    fn test_prosody_breathiness_sad() {
        let prosody = ProsodyMapper::map([-0.7, -0.3, -0.5], &sad_lp());
        // 悲伤→偏高气声
        assert!(
            prosody.breathiness > 0.15,
            "sad should have breathiness: {}",
            prosody.breathiness
        );
    }

    #[test]
    fn test_prosody_warmth_joy() {
        let prosody = ProsodyMapper::map([0.7, 0.5, 0.4], &joy_lp());
        // 喜悦→温暖音色
        assert!(
            prosody.warmth > 0.5,
            "joy should be warm: {}",
            prosody.warmth
        );
    }

    #[test]
    fn test_prosody_clamps() {
        // 极端 PAD 不应产生超范围值
        let lp = LinguisticProfile::neutral();
        let prosody = ProsodyMapper::map([1.0, 1.0, 1.0], &lp);
        assert!(prosody.pitch_offset >= -5.0 && prosody.pitch_offset <= 5.0);
        assert!(prosody.rate >= 0.6 && prosody.rate <= 1.5);
        assert!(prosody.energy >= 0.4 && prosody.energy <= 1.4);
        assert!(prosody.warmth >= 0.0 && prosody.warmth <= 1.0);
    }

    // ── to_prompt_fragment 测试 / to_prompt_fragment tests ──

    #[test]
    fn test_prosody_prompt_fragment_neutral() {
        // 中性 PAD → 韵律片段以[韵律]开头 / Neutral PAD → fragment starts with [韵律]
        // 注意：LinguisticProfile::neutral() 的 silence_tendency=0.3 贡献气声，
        // 所以中性 PAD 可能产生"带轻微气声"而非纯"语速适中"
        let params = ProsodyMapper::map([0.0, 0.0, 0.0], &LinguisticProfile::neutral());
        let frag = params.to_prompt_fragment();
        assert!(frag.starts_with("[韵律]"), "should start with [韵律]");
        // 中性PAD不应出现极端描述 / Neutral PAD should not have extreme descriptors
        assert!(
            !frag.contains("语速偏快"),
            "neutral should not be fast: {}",
            frag
        );
        assert!(
            !frag.contains("语速偏慢"),
            "neutral should not be slow: {}",
            frag
        );
    }

    #[test]
    fn test_prosody_prompt_fragment_fast() {
        // 高唤醒 → 语速偏快 / High arousal → fast rate
        let params = ProsodyMapper::map([0.5, 0.8, 0.3], &LinguisticProfile::neutral());
        let frag = params.to_prompt_fragment();
        assert!(frag.starts_with("[韵律]"), "should start with [韵律]");
        assert!(
            frag.contains("语速偏快"),
            "high arousal should mention fast rate: {}",
            frag
        );
    }

    #[test]
    fn test_prosody_prompt_fragment_slow() {
        // 低唤醒 → 语速偏慢 / Low arousal → slow rate
        let params = ProsodyMapper::map([0.0, -0.8, 0.0], &LinguisticProfile::neutral());
        let frag = params.to_prompt_fragment();
        assert!(frag.starts_with("[韵律]"), "should start with [韵律]");
        assert!(
            frag.contains("语速偏慢"),
            "low arousal should mention slow rate: {}",
            frag
        );
    }

    #[test]
    fn test_prosody_prompt_fragment_warm() {
        // 高愉悦 → 音色温暖 / High pleasure → warm tone
        let params = ProsodyMapper::map([0.8, 0.0, 0.0], &LinguisticProfile::neutral());
        let frag = params.to_prompt_fragment();
        assert!(
            frag.contains("音色温暖"),
            "high pleasure should mention warmth: {}",
            frag
        );
    }
}
