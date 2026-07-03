// SPDX-License-Identifier: MIT
//! 真实表达调制器 / Authentic Expression Modulator
//!
//! 脆弱表达不是从模板中选一个——
//! 而是在当前情绪状态、关系深度、话题语境的共同调制下，
//! 生成有温度的、不重复的脆弱表达。
//!
//! Vulnerability expression is not template selection —
//! it is modulation by current emotional state, relationship depth,
//! and conversational context, producing warm, non-repetitive expression.

use crate::vulnerability_window::ConversationContext;

// ═══════════════════════════════════════════════════════════════════
//  配置 / Configuration
// ═══════════════════════════════════════════════════════════════════

/// 表达调制配置 / Expression modulation configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExpressionConfig {
    /// 是否启用情绪调制 / Enable emotion modulation
    pub emotion_modulation: bool,
    /// 是否启用关系调制 / Enable relationship modulation
    pub relation_modulation: bool,
    /// 是否启用话题调制 / Enable topic modulation
    pub topic_modulation: bool,
}

impl Default for ExpressionConfig {
    fn default() -> Self {
        Self {
            emotion_modulation: true,
            relation_modulation: true,
            topic_modulation: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  调制器 / Modulator
// ═══════════════════════════════════════════════════════════════════

/// 真实表达调制器 / Authentic Expression Modulator
///
/// 无状态调制器，基于 PAD 状态、关系深度、话题语境
/// 为脆弱表达模板添加调制后缀，使其有温度而非僵化。
/// 所有操作 O(1) 微秒级。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthenticExpressionModulator {
    /// 配置 / Configuration
    pub(crate) config: ExpressionConfig,
}

impl AuthenticExpressionModulator {
    /// 创建新的调制器 / Create a new modulator
    pub fn new(config: ExpressionConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 / Create with default configuration
    pub fn default_new() -> Self {
        Self::new(ExpressionConfig::default())
    }

    /// 调制脆弱表达 / Modulate vulnerability expression
    ///
    /// 在基础模板上叠加情绪、关系、话题调制后缀，
    /// 生成有温度的脆弱表达。
    ///
    /// # 参数 / Parameters
    /// - `template`: 基础模板文本 / Base template text
    /// - `pleasure`: 当前愉悦度 (-1.0 to 1.0)
    /// - `arousal`: 当前激活度 (-1.0 to 1.0)
    /// - `dominance`: 当前支配度 (-1.0 to 1.0)
    /// - `relation_ordinal`: 关系阶段序数 (0-3)
    /// - `context`: 对话场景
    pub fn modulate(
        &self,
        template: &str,
        pleasure: f32,
        arousal: f32,
        dominance: f32,
        relation_ordinal: u8,
        context: ConversationContext,
    ) -> String {
        let mut result = template.to_string();

        if self.config.emotion_modulation {
            if let Some(suffix) = self.emotion_suffix(pleasure, arousal, dominance) {
                result.push_str(suffix);
            }
        }

        if self.config.relation_modulation {
            if let Some(suffix) = self.relation_suffix(relation_ordinal) {
                result.push_str(suffix);
            }
        }

        if self.config.topic_modulation {
            if let Some(suffix) = self.topic_suffix(context) {
                result.push_str(suffix);
            }
        }

        result
    }

    /// 情绪修饰后缀 / Emotion modulation suffix
    ///
    /// - pleasure < -0.3 → 更短促直接（"...让我想想。"）
    /// - arousal > 0.5 → 带停顿感（"...我...需要想想。"）
    /// - dominance < -0.3 → 更谦逊（"...也许我说的不够好。"）
    fn emotion_suffix(&self, pleasure: f32, arousal: f32, dominance: f32) -> Option<&'static str> {
        // 优先级：低支配度 > 高激活 > 低愉悦
        if dominance < -0.3 {
            return Some("...也许我说的不够好。");
        }
        if arousal > 0.5 {
            return Some("...我...需要想想。");
        }
        if pleasure < -0.3 {
            return Some("...让我想想。");
        }
        None
    }

    /// 关系深度修饰后缀 / Relationship depth suffix
    ///
    /// - Deep (ordinal 3) → 更少修饰，更直接真实
    /// - Trusted (ordinal 2) → 适度修饰
    /// - 其他 → 无额外修饰
    fn relation_suffix(&self, relation_ordinal: u8) -> Option<&'static str> {
        match relation_ordinal {
            3 => Some("但这是真实的想法。"), // Deep
            2 => Some("这是我的真实感受。"), // Trusted
            _ => None,
        }
    }

    /// 话题语境修饰后缀 / Topic context suffix
    ///
    /// - DeepTalk → 哲学化不确定
    /// - Emotional → 情感化脆弱
    /// - Casual → 轻松自嘲
    /// - 其他 → 无
    fn topic_suffix(&self, context: ConversationContext) -> Option<&'static str> {
        match context {
            ConversationContext::DeepTalk => Some("也许我们都在摸索。"),
            ConversationContext::Emotional => Some("说出来会好一些。"),
            ConversationContext::Casual => Some("哈，可能我想多了。"),
            _ => None,
        }
    }

    /// 获取配置 / Get configuration
    pub fn config(&self) -> &ExpressionConfig {
        &self.config
    }
}

// ═══════════════════════════════════════════════════════════════════
//  单元测试 / Unit Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_new() {
        let modulator = AuthenticExpressionModulator::default_new();
        assert!(modulator.config().emotion_modulation);
        assert!(modulator.config().relation_modulation);
        assert!(modulator.config().topic_modulation);
    }

    #[test]
    fn test_modulate_no_modification_when_neutral() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0, // neutral pleasure
            0.0, // neutral arousal
            0.0, // neutral dominance
            1,   // Familiar
            ConversationContext::Creative,
        );
        // 无调制后缀（所有条件都不满足，Familiar 无关系后缀，Creative 无话题后缀）
        assert_eq!(result, "我不太确定...");
    }

    #[test]
    fn test_emotion_suffix_low_pleasure() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            -0.5, // low pleasure
            0.0,
            0.0,
            1,
            ConversationContext::Creative,
        );
        assert!(result.contains("让我想想"));
    }

    #[test]
    fn test_emotion_suffix_high_arousal() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0,
            0.7, // high arousal
            0.0,
            1,
            ConversationContext::Creative,
        );
        assert!(result.contains("需要想想"));
    }

    #[test]
    fn test_emotion_suffix_low_dominance_priority() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            -0.5, // low pleasure
            0.7,  // high arousal
            -0.5, // low dominance — should take priority
            1,
            ConversationContext::Creative,
        );
        // 低支配度优先级最高
        assert!(result.contains("不够好"));
    }

    #[test]
    fn test_relation_suffix_deep() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0,
            0.0,
            0.0,
            3, // Deep
            ConversationContext::Creative,
        );
        assert!(result.contains("真实的想法"));
    }

    #[test]
    fn test_relation_suffix_trusted() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0,
            0.0,
            0.0,
            2, // Trusted
            ConversationContext::Creative,
        );
        assert!(result.contains("真实感受"));
    }

    #[test]
    fn test_topic_suffix_deep_talk() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0,
            0.0,
            0.0,
            1,
            ConversationContext::DeepTalk,
        );
        assert!(result.contains("摸索"));
    }

    #[test]
    fn test_topic_suffix_emotional() {
        let modulator = AuthenticExpressionModulator::default_new();
        let result = modulator.modulate(
            "我不太确定...",
            0.0,
            0.0,
            0.0,
            1,
            ConversationContext::Emotional,
        );
        assert!(result.contains("说出来"));
    }

    #[test]
    fn test_modulate_contains_base_template() {
        let modulator = AuthenticExpressionModulator::default_new();
        let base = "我可能想多了...";
        let result = modulator.modulate(base, -0.4, 0.6, -0.4, 3, ConversationContext::DeepTalk);
        // 调制后仍包含基础模板
        assert!(result.starts_with(base));
    }

    #[test]
    fn test_disabled_modulation() {
        let config = ExpressionConfig {
            emotion_modulation: false,
            relation_modulation: false,
            topic_modulation: false,
        };
        let modulator = AuthenticExpressionModulator::new(config);
        let result = modulator.modulate(
            "我不太确定...",
            -0.5,
            0.7,
            -0.5,
            3,
            ConversationContext::DeepTalk,
        );
        assert_eq!(result, "我不太确定...");
    }
}
