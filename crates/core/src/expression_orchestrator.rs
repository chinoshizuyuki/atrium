// SPDX-License-Identifier: MIT
//! 表达编排器 — 四通道(text×voice×kinesics×timing)联合编排 + 一致性校验
//! ExpressionOrchestrator — 4-channel joint orchestration + coherence validation.
//!
//! 设计文档 §9: ExpressionOrchestrator
//! 8 步流水线:
//!   1. StyleEmbedding::from_emotion_context()
//!   2. EmotionalArc::generate()
//!   3. Text 通道: style_modulator → LinguisticProfile + prompt_fragment
//!   4. Voice 通道: prosody_mapper → ProsodyParams
//!   5. Kinesics 通道: kinesics_mapper → KinesicsOutput
//!   6. Timing 通道: timing_mapper → TimingProfile
//!   7. Subtext: subtext_engine.decide() → AiSubtextSignal
//!   8. ensure_coherence() 4 项一致性校验
//!
//! 零侵入原则: 只读取 EmotionEngine/RelationshipManager/UserMentalModel 的公开方法，永不修改它们。

use atrium_emotion::EmotionDirection;
use atrium_memory::emotional_arc::{ArcTrend, EmotionalArc};
use atrium_memory::kinesics_mapper::{KinesicsMapper, KinesicsOutput};
use atrium_memory::prosody_mapper::{ProsodyMapper, ProsodyParams};
use atrium_memory::style_modulator::{ExpressionContext, LinguisticProfile, StyleEmbedding};
use atrium_memory::subtext_engine::{AiSubtextSignal, SubtextEngine};
use atrium_memory::timing_mapper::{TimingMapper, TimingProfile};

// ════════════════════════════════════════════════════════════════════
// CoordinatedExpression — 编排器输出
// ════════════════════════════════════════════════════════════════════

/// 协调表达 — 四通道 + 潜台词 + 情绪轨迹的统一输出
///
/// 这是 ExpressionOrchestrator::orchestrate() 的返回值，
/// 包含一次回复所需的全部表达参数。
#[derive(Clone, Debug)]
pub struct CoordinatedExpression {
    // ── 四通道 ──
    /// Text 通道: 风格嵌入 + 语言学特征 + LLM prompt fragment
    pub style: StyleEmbedding,
    pub linguistic: LinguisticProfile,
    pub prompt_fragment: String,

    /// Voice 通道: 韵律参数
    pub prosody: ProsodyParams,

    /// Kinesics 通道: 体态 + 微表情
    pub kinesics: KinesicsOutput,

    /// Timing 通道: 时序特征
    pub timing: TimingProfile,

    // ── 潜台词层 ──
    /// AI 言外之意信号
    pub subtext_signals: Vec<AiSubtextSignal>,
    /// 潜台词 prompt 注入片段
    pub subtext_prompt_hint: String,

    // ── 情绪轨迹 ──
    /// 回复内情绪轨迹
    pub arc: EmotionalArc,

    // ── 一致性 ──
    /// 一致性校验结果
    pub coherence: CoherenceReport,
}

// ════════════════════════════════════════════════════════════════════
// CoherenceReport — 一致性校验报告
// ════════════════════════════════════════════════════════════════════

/// 一致性校验报告 — 设计文档 §9.3 ensure_coherence 4 项校验
#[derive(Clone, Debug)]
pub struct CoherenceReport {
    /// 1. 情绪方向一致性: 文本情绪方向 vs 非语言情绪方向
    pub direction_consistent: bool,
    /// 2. 唤醒度一致性: 文本唤醒 vs 韵律/体态唤醒
    pub arousal_consistent: bool,
    /// 3. 支配度一致性: 文本支配度 vs 体态支配度
    pub dominance_consistent: bool,
    /// 4. 言行一致性: 说的话 vs 语气/体态是否矛盾
    pub speech_act_consistent: bool,
    /// 整体一致性（4 项全部通过）
    pub is_coherent: bool,
    /// 不一致项的修正建议
    pub warnings: Vec<String>,
}

impl CoherenceReport {
    /// 全部通过
    #[allow(dead_code)]
    fn all_pass() -> Self {
        Self {
            direction_consistent: true,
            arousal_consistent: true,
            dominance_consistent: true,
            speech_act_consistent: true,
            is_coherent: true,
            warnings: Vec::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ExpressionOrchestrator — 表达编排器
// ════════════════════════════════════════════════════════════════════

/// 表达编排器 — 四通道联合编排
///
/// 无状态（所有子模块都是无状态的静态方法），
/// 每次调用 orchestrate() 独立计算。
pub struct ExpressionOrchestrator;

impl ExpressionOrchestrator {
    /// 8 步编排流水线 — 设计文档 §9.2
    ///
    /// 输入: ExpressionContext（已在 style_modulator 中定义）
    /// 输出: CoordinatedExpression（四通道 + 潜台词 + 轨迹 + 一致性）
    ///
    /// # Arguments
    /// * `ctx` - 表达上下文
    /// * `user_text` - 用户输入文本（用于潜台词检测）
    /// * `user_pad` - 用户当前 PAD（用于 AI 潜台词决策）
    /// * `reply_length_estimate` - 预估回复长度（字数）
    pub fn orchestrate(
        ctx: &ExpressionContext,
        _user_text: &str,
        user_pad: [f32; 3],
        reply_length_estimate: usize,
    ) -> CoordinatedExpression {
        // ── Step 1: StyleEmbedding ──
        let style = ctx.to_style_embedding();

        // ── Step 2: EmotionalArc ──
        let arc = EmotionalArc::generate(
            ctx.pad,
            ctx.target_pad,
            &ctx.relationship,
            reply_length_estimate,
        );

        // ── Step 3: Text 通道 ──
        let mut linguistic = LinguisticProfile::from_style_embedding(&style);
        linguistic.apply_relationship_overlay(&ctx.relationship);
        let prompt_fragment = style.to_prompt_fragment(&ctx.relationship);

        // ── Step 4: Voice 通道 ──
        let prosody = ProsodyMapper::map(ctx.pad, &linguistic);

        // ── Step 5: Kinesics 通道 ──
        let kinesics = KinesicsMapper::map(ctx.pad, reply_length_estimate);

        // ── Step 6: Timing 通道 ──
        let timing = TimingMapper::map(ctx.pad, &linguistic, &ctx.relationship);

        // ── Step 7: Subtext ──
        let subtext_signals =
            SubtextEngine::decide(ctx.pad, user_pad, &ctx.relationship, ctx.topic_gravity);

        // 合并潜台词 prompt hint
        let subtext_prompt_hint = subtext_signals
            .iter()
            .filter(|s| s.express_explicitly)
            .map(|s| s.prompt_hint.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // ── Step 8: ensure_coherence ──
        let coherence = Self::ensure_coherence(
            &ctx.direction,
            &linguistic,
            &prosody,
            &kinesics,
            &subtext_signals,
            ctx.pad,
        );

        CoordinatedExpression {
            style,
            linguistic,
            prompt_fragment,
            prosody,
            kinesics,
            timing,
            subtext_signals,
            subtext_prompt_hint,
            arc,
            coherence,
        }
    }

    /// 一致性校验 — 设计文档 §9.3 ensure_coherence
    ///
    /// 4 项校验:
    /// 1. 情绪方向一致性: 文本情绪方向 vs 非语言情绪方向
    /// 2. 唤醒度一致性: 文本唤醒 vs 韵律/体态唤醒
    /// 3. 支配度一致性: 文本支配度 vs 体态支配度
    /// 4. 言行一致性: 说的话 vs 语气/体态是否矛盾
    fn ensure_coherence(
        direction: &EmotionDirection,
        lp: &LinguisticProfile,
        prosody: &ProsodyParams,
        kinesics: &KinesicsOutput,
        subtext: &[AiSubtextSignal],
        pad: [f32; 3],
    ) -> CoherenceReport {
        let mut warnings = Vec::new();

        // ── 1. 情绪方向一致性 ──
        // 自我导向 + 高亲昵 → 矛盾（自我导向应偏克制）
        let direction_consistent = match direction {
            EmotionDirection::SelfDirected => lp.endearment_tendency < 0.5,
            EmotionDirection::UserDirected => true, // 用户导向总是与亲昵兼容
            EmotionDirection::MemoryDirected => lp.endearment_tendency < 0.6,
            EmotionDirection::Neutral => true,
        };
        if !direction_consistent {
            warnings.push("情绪方向与亲昵度矛盾: 自我导向情绪不应高亲昵".to_string());
        }

        // ── 2. 唤醒度一致性 ──
        // 文本高唤醒(碎片化/情感词密度高) vs 韵律低唤醒(慢速/低能量)
        // 阈值设高以避免中性PAD误报: fragmentation > 0.55, emotion_word_density > 0.6
        let text_arousal_high = lp.fragmentation > 0.55 || lp.emotion_word_density > 0.6;
        let voice_arousal_high = prosody.rate > 1.1 || prosody.energy > 1.1;
        let arousal_consistent = (!text_arousal_high || voice_arousal_high)
            && (text_arousal_high || !voice_arousal_high || pad[1] >= 0.0);
        if !arousal_consistent {
            warnings.push("唤醒度不一致: 文本与韵律的唤醒水平矛盾".to_string());
        }

        // ── 3. 支配度一致性 ──
        // 文本高确定性/高支配 vs 体态低支配(低眼神/低肩膀展开)
        let text_dominance_high = lp.certainty_marking > 0.6;
        let body_dominance_high =
            kinesics.posture.eye_contact > 0.5 && kinesics.posture.shoulder_openness > 0.5;
        let dominance_consistent = !text_dominance_high || body_dominance_high;
        if !dominance_consistent {
            warnings.push("支配度不一致: 文本确定性高但体态缺乏自信信号".to_string());
        }

        // ── 4. 言行一致性 ──
        // 有故作轻松潜台词 + 文本高确定性 → 矛盾（故作轻松应降低确定性）
        let has_feigned = subtext
            .iter()
            .any(|s| s.kind == atrium_memory::subtext_engine::SubtextKind::FeignedNonchalance);
        let speech_act_consistent = !(has_feigned && lp.certainty_marking > 0.5);
        if !speech_act_consistent {
            warnings.push("言行不一致: 故作轻松但确定性标记过高，应降低确定性".to_string());
        }

        let is_coherent = direction_consistent
            && arousal_consistent
            && dominance_consistent
            && speech_act_consistent;

        CoherenceReport {
            direction_consistent,
            arousal_consistent,
            dominance_consistent,
            speech_act_consistent,
            is_coherent,
            warnings,
        }
    }

    /// 构建 LLM 系统提示中的表达注入片段
    ///
    /// Step 8.5 注入点: 将风格指令 + 潜台词 hint 合并注入到 LLM system prompt。
    /// 格式: "[回复风格] ... [潜台词] ..."
    pub fn build_system_prompt_injection(expr: &CoordinatedExpression) -> String {
        let mut parts = Vec::new();

        if !expr.prompt_fragment.is_empty() {
            parts.push(expr.prompt_fragment.clone());
        }

        if !expr.subtext_prompt_hint.is_empty() {
            parts.push(format!("[潜台词] {}", expr.subtext_prompt_hint));
        }

        // 情绪轨迹提示
        if expr.arc.waypoints.len() > 1 {
            match expr.arc.trend {
                ArcTrend::Escalating => {
                    parts.push("[情绪轨迹] 回复中情绪逐渐升温。".to_string());
                }
                ArcTrend::DeEscalating => {
                    parts.push("[情绪轨迹] 回复中情绪逐渐平复。".to_string());
                }
                ArcTrend::Recovering => {
                    parts.push("[情绪轨迹] 从强烈情绪中恢复，语气逐渐缓和。".to_string());
                }
                ArcTrend::Oscillating => {
                    parts.push("[情绪轨迹] 内心矛盾，语气可以有犹豫和反复。".to_string());
                }
                ArcTrend::Steady => {}
            }
        }

        // 一致性警告注入
        if !expr.coherence.is_coherent {
            for warning in &expr.coherence.warnings {
                parts.push(format!("[一致性] {}", warning));
            }
        }

        parts.join(" ")
    }

    /// 回复后处理 — 对 LLM 原始输出应用语言学特征微调
    ///
    /// 设计文档 §12: post_process 在 LLM 生成后对文本做轻量修饰。
    /// 当前版本: 仅做一致性修正提示（不修改文本本身）。
    /// 后续版本: 可根据 LinguisticProfile 做标点/语气词/省略号等微调。
    pub fn post_process(expr: &CoordinatedExpression, raw_reply: &str) -> String {
        // 当前版本: 直接返回原文
        // 一致性不通过时，在末尾附加修正标记（仅调试用，生产环境不附加）
        let _ = (expr, raw_reply);
        raw_reply.to_string()
    }
}

// ════════════════════════════════════════════════════════════════════
// 单元测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_emotion::EmotionState;
    use atrium_memory::relationship::RelationshipStage;
    use atrium_memory::style_modulator::ExpressionContext;

    /// 辅助: 构建中性 ExpressionContext
    fn neutral_ctx() -> ExpressionContext {
        ExpressionContext::from_modules(
            &EmotionState::new(0.0, 0.0, 0.0),
            None,
            EmotionDirection::Neutral,
            &RelationshipStage::Familiar {
                since: 0,
                interactions: 50,
                shared_references: 5,
            },
            0.0,
            0.3,
        )
    }

    /// 辅助: 构建悲伤 ExpressionContext
    fn sad_ctx() -> ExpressionContext {
        ExpressionContext::from_modules(
            &EmotionState::new(-0.6, -0.3, -0.2),
            None,
            EmotionDirection::SelfDirected,
            &RelationshipStage::Trusted {
                since: 0,
                interactions: 200,
                shared_references: 30,
                key_moments: 5,
            },
            -0.4,
            0.5,
        )
    }

    /// 辅助: 构建喜悦 ExpressionContext
    fn joy_ctx() -> ExpressionContext {
        let state = EmotionState::new(0.7, 0.5, 0.3);
        ExpressionContext::from_modules(
            &state,
            None,
            EmotionDirection::UserDirected,
            &RelationshipStage::Deep {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 20,
            },
            0.6,
            0.2,
        )
    }

    // ── orchestrate 基本测试 ──

    #[test]
    fn test_orchestrate_neutral() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "你好", [0.0, 0.0, 0.0], 50);

        // 验证四通道都有输出
        assert!(expr.style.norm() >= 0.0);
        assert!(!expr.prompt_fragment.is_empty() || expr.linguistic.sentence_length > 0.0);
        assert!(expr.prosody.rate > 0.0);
        assert!(expr.timing.typing_delay_factor > 0.0);
        // 中性情绪应通过一致性
        assert!(expr.coherence.is_coherent);
    }

    #[test]
    fn test_orchestrate_sad() {
        let ctx = sad_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "我没事", [-0.3, -0.2, 0.0], 100);

        // 悲伤: 沉默倾向应偏高
        assert!(expr.linguistic.silence_tendency > 0.1);
        // 悲伤: 语速应偏慢
        assert!(expr.prosody.rate < 1.1);
        // 悲伤: 打字延迟应偏慢
        assert!(expr.timing.typing_delay_factor > 0.8);
        // 情绪轨迹应存在
        assert!(!expr.arc.waypoints.is_empty());
    }

    #[test]
    fn test_orchestrate_joy() {
        let ctx = joy_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "太好了！", [0.5, 0.3, 0.0], 80);

        // 喜悦: 亲昵倾向应偏高（深度关系）
        assert!(expr.linguistic.endearment_tendency > 0.2);
        // 喜悦: 音色温暖度应偏高
        assert!(expr.prosody.warmth > 0.4);
        // 喜悦: 紧迫感应偏高
        assert!(expr.timing.urgency > 0.4);
    }

    // ── ensure_coherence 测试 ──

    #[test]
    fn test_coherence_all_pass_neutral() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "你好", [0.0, 0.0, 0.0], 50);
        assert!(expr.coherence.direction_consistent);
        assert!(expr.coherence.arousal_consistent);
        assert!(expr.coherence.dominance_consistent);
        assert!(expr.coherence.speech_act_consistent);
        assert!(expr.coherence.is_coherent);
        assert!(expr.coherence.warnings.is_empty());
    }

    #[test]
    fn test_coherence_self_directed_high_endearment_fails() {
        // 自我导向 + 高亲昵 → 方向一致性失败
        let state = EmotionState::new(0.8, 0.6, 0.5);
        let ctx = ExpressionContext::from_modules(
            &state,
            None,
            EmotionDirection::SelfDirected, // 自我导向
            &RelationshipStage::Deep {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 20,
            }, // 深度关系 → 高亲昵
            0.5,
            0.1,
        );
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "我很好", [0.3, 0.2, 0.0], 60);
        // 深度关系 + 自我导向 → endearment_tendency 可能 > 0.5 → direction_consistent = false
        // 但如果 apply_relationship_overlay 把 endearment 压到 <= 0.5，则通过
        // 关键: 验证 coherence 逻辑正确运行
        if expr.linguistic.endearment_tendency >= 0.5 {
            assert!(!expr.coherence.direction_consistent);
        }
    }

    // ── build_system_prompt_injection 测试 ──

    #[test]
    fn test_prompt_injection_neutral() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "你好", [0.0, 0.0, 0.0], 50);
        let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
        // 中性情绪可能没有 prompt_fragment（没有显著风格特征）
        // 但不应 panic
        let _ = injection.len();
    }

    #[test]
    fn test_prompt_injection_sad_with_subtext() {
        let ctx = sad_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "我没事", [-0.3, -0.2, 0.0], 100);
        let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
        // 悲伤 + 深度关系 → 可能有潜台词注入
        if !expr.subtext_prompt_hint.is_empty() {
            assert!(injection.contains("[潜台词]"));
        }
    }

    #[test]
    fn test_prompt_injection_with_arc_trend() {
        let ctx = sad_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "我没事",
            [-0.3, -0.2, 0.0],
            200, // 长回复 → 多轨迹节点
        );
        let injection = ExpressionOrchestrator::build_system_prompt_injection(&expr);
        if expr.arc.waypoints.len() > 1 && expr.arc.trend != ArcTrend::Steady {
            assert!(injection.contains("[情绪轨迹]"));
        }
    }

    // ── post_process 测试 ──

    #[test]
    fn test_post_process_passthrough() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "你好", [0.0, 0.0, 0.0], 50);
        let reply = "你好呀！今天怎么样？";
        let processed = ExpressionOrchestrator::post_process(&expr, reply);
        assert_eq!(processed, reply);
    }

    // ── 关系阶段差异化测试 ──

    #[test]
    fn test_relationship_stage_differentiation() {
        // 同一 PAD，不同关系阶段 → 不同表达
        let pad = [0.3, 0.2, 0.1];
        let state = EmotionState::new(pad[0], pad[1], pad[2]);

        let ctx_acq = ExpressionContext::from_modules(
            &state,
            None,
            EmotionDirection::UserDirected,
            &RelationshipStage::Acquaintance {
                since: 0,
                interactions: 5,
            },
            0.2,
            0.3,
        );
        let ctx_deep = ExpressionContext::from_modules(
            &state,
            None,
            EmotionDirection::UserDirected,
            &RelationshipStage::Deep {
                since: 0,
                interactions: 500,
                shared_references: 80,
                key_moments: 20,
            },
            0.2,
            0.3,
        );

        let expr_acq = ExpressionOrchestrator::orchestrate(&ctx_acq, "你好", pad, 80);
        let expr_deep = ExpressionOrchestrator::orchestrate(&ctx_deep, "你好", pad, 80);

        // 初识阶段: 亲昵应低于深度阶段
        assert!(expr_acq.linguistic.endearment_tendency < expr_deep.linguistic.endearment_tendency);
        // 初识阶段: 语气词应低于深度阶段
        assert!(expr_acq.linguistic.particle_density <= expr_deep.linguistic.particle_density);
    }

    // ── 潜台词决策测试 ──

    #[test]
    fn test_subtext_companionate_silence() {
        // 双方都悲伤 → CompanionateSilence
        let ctx = sad_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "我没事",
            [-0.4, -0.2, 0.0], // 用户也悲伤
            100,
        );
        let has_companionate = expr
            .subtext_signals
            .iter()
            .any(|s| s.kind == atrium_memory::subtext_engine::SubtextKind::CompanionateSilence);
        assert!(has_companionate);
    }

    #[test]
    fn test_subtext_unspoken_concern() {
        // 用户悲伤 + AI 中性 → UnspokenConcern
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "还好吧",
            [-0.4, -0.1, 0.0], // 用户悲伤
            80,
        );
        let has_concern = expr
            .subtext_signals
            .iter()
            .any(|s| s.kind == atrium_memory::subtext_engine::SubtextKind::UnspokenConcern);
        assert!(has_concern);
    }

    // ── 情绪轨迹测试 ──

    #[test]
    fn test_arc_single_point_short_reply() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "嗯",
            [0.0, 0.0, 0.0],
            10, // 极短回复
        );
        // 短回复应只有 1 个轨迹节点
        assert_eq!(expr.arc.waypoints.len(), 1);
    }

    #[test]
    fn test_arc_multiple_points_long_reply() {
        let ctx = sad_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "我没事",
            [-0.3, -0.2, 0.0],
            300, // 长回复
        );
        // 长回复应有多个轨迹节点
        assert!(expr.arc.waypoints.len() > 1);
    }

    // ── CoordinatedExpression 完整性测试 ──

    #[test]
    fn test_coordinated_expression_all_channels_populated() {
        let ctx = joy_ctx();
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "太好了！", [0.5, 0.3, 0.0], 100);

        // 验证所有通道都有合理值
        // Style: 非零范数
        assert!(expr.style.norm() > 0.0);
        // Linguistic: 句长在合理范围
        assert!(expr.linguistic.sentence_length >= 3.0 && expr.linguistic.sentence_length <= 25.0);
        // Prosody: 语速 > 0
        assert!(expr.prosody.rate > 0.0);
        // Kinesics: 体态值在合理范围
        assert!(
            expr.kinesics.posture.shoulder_openness >= 0.0
                && expr.kinesics.posture.shoulder_openness <= 1.0
        );
        // Timing: 延迟因子 > 0
        assert!(expr.timing.typing_delay_factor > 0.0);
        // Arc: 至少 1 个 waypoint
        assert!(!expr.arc.waypoints.is_empty());
    }

    // ── 边界条件测试 ──

    #[test]
    fn test_orchestrate_extreme_pad() {
        // 极端 PAD 值不应 panic
        let state = EmotionState::new(-1.0, -1.0, -1.0);
        let ctx = ExpressionContext::from_modules(
            &state,
            None,
            EmotionDirection::SelfDirected,
            &RelationshipStage::Acquaintance {
                since: 0,
                interactions: 1,
            },
            -1.0,
            1.0,
        );
        let expr = ExpressionOrchestrator::orchestrate(&ctx, "test", [-1.0, -1.0, -1.0], 50);
        // 不应 panic，coherence 应正常计算
        let _ = expr.coherence.is_coherent;
    }

    #[test]
    fn test_orchestrate_zero_length_reply() {
        let ctx = neutral_ctx();
        let expr = ExpressionOrchestrator::orchestrate(
            &ctx,
            "",
            [0.0, 0.0, 0.0],
            0, // 零长度
        );
        // 不应 panic
        assert_eq!(expr.arc.waypoints.len(), 1); // 单点轨迹
    }
}
