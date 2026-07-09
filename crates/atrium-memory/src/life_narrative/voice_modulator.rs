use super::*;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// VoiceModulator / ModulatedNarrative — 叙事语气调制器 / Narrative Voice Modulator
// ════════════════════════════════════════════════════════════════════

/// 调制后的叙事 / Modulated narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulatedNarrative {
    /// 原始文本 / Original text
    pub original: String,
    /// 调制后文本 / Modulated text
    pub modulated: String,
    /// 应用的语气 / Applied tone
    pub tone: NarrativeTone,
    /// 应用的视角 / Applied perspective
    pub perspective: NarrativePerspective,
    /// 调制强度 (0.0~1.0) / Modulation strength
    pub strength: f64,
}

/// 叙事语气调制器 — 根据情感状态和回忆距离调整叙事语气
/// Voice modulator — adjust narrative tone based on emotion state and recall distance
///
/// Phase A: 提供数据结构和基础算法 / Phase A: data structures and basic algorithm
/// Phase D: 集成 CoreService / Phase D: integrate with CoreService
pub struct VoiceModulator {
    /// 默认视角 / Default perspective
    pub default_perspective: NarrativePerspective,
    /// 默认风格 / Default style
    pub default_style: NarrativeStyle,
}

impl VoiceModulator {
    /// 创建语气调制器 / Create voice modulator
    pub fn new(perspective: NarrativePerspective, style: NarrativeStyle) -> Self {
        Self {
            default_perspective: perspective,
            default_style: style,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(NarrativePerspective::FirstPerson, NarrativeStyle::Adaptive)
    }

    /// 推断语气 — 从当前情感和回忆距离 / Infer tone from current emotion and recall distance
    ///
    /// 回忆距离越远，语气越趋向怀旧/客观；
    /// The more distant the recall, the more nostalgic/objective the tone.
    pub fn infer_tone(
        &self,
        current_pad: &[f32; 3],
        recall_pad: &[f32; 3],
        recall_distance_days: i64,
    ) -> NarrativeTone {
        // 近期回忆：直接用当前 PAD / Recent recall: use current PAD directly
        if recall_distance_days < 3 {
            NarrativeTone::from_pad(current_pad)
        } else if recall_distance_days < 14 {
            // 中期回忆：混合当前和回忆 PAD / Medium recall: blend current and recall PAD
            let blended = [
                (current_pad[0] + recall_pad[0]) / 2.0,
                (current_pad[1] + recall_pad[1]) / 2.0,
                (current_pad[2] + recall_pad[2]) / 2.0,
            ];
            NarrativeTone::from_pad(&blended)
        } else {
            // 远期回忆：趋向客观/怀旧 / Distant recall: tend toward objective/nostalgic
            let p = recall_pad[0];
            if p > 0.1 {
                NarrativeTone::WarmNostalgia
            } else if p < -0.2 {
                NarrativeTone::BitterLonging
            } else {
                NarrativeTone::ObjectiveRecall
            }
        }
    }

    /// 调制叙事文本 / Modulate narrative text
    ///
    /// Phase A: 返回语气标记的文本（不做 LLM 改写）
    /// Phase A: return tone-annotated text (no LLM rewrite)
    pub fn modulate(
        &self,
        text: &str,
        tone: NarrativeTone,
        current_pad: &[f32; 3],
    ) -> ModulatedNarrative {
        // 计算调制强度：情感越强烈，调制越强 / Stronger emotion → stronger modulation
        let emotion_intensity =
            (current_pad[0].powi(2) + current_pad[1].powi(2) + current_pad[2].powi(2)).sqrt()
                as f64;
        let strength = (emotion_intensity / 1.0).min(1.0);

        // Phase A: 仅添加语气标记前缀 / Phase A: only add tone marker prefix
        let tone_marker = format!("[{}]", tone.label_zh());
        let modulated = format!("{} {}", tone_marker, text);

        ModulatedNarrative {
            original: text.to_string(),
            modulated,
            tone,
            perspective: self.default_perspective,
            strength,
        }
    }
}

impl Default for VoiceModulator {
    fn default() -> Self {
        Self::default_new()
    }
}
