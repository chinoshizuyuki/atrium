// SPDX-License-Identifier: MIT
//! 体态映射器 — 从情绪状态映射到虚拟体态/微表情
//! KinesicsMapper — Map emotion state to virtual body language / micro-expressions.
//!
//! 虚拟形象的体态不是随机的，而是情绪的物理投射。
//! 微表情持续时间 0.04-0.2 秒，是情绪的真实泄露。

use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// MicroExpression — 微表情
// ════════════════════════════════════════════════════════════════════

/// 微表情类型
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MicroExpressionType {
    /// 嘴角微微上扬（喜悦泄露）
    LipCornerPull,
    /// 嘴角微微下垂（悲伤泄露）
    LipCornerDepress,
    /// 眉头微蹙（担忧/不满）
    BrowFurrow,
    /// 眼睛微眯（怀疑/不悦）
    EyeSquint,
    /// 鼻翼微动（厌恶泄露）
    NoseWrinkle,
    /// 嘴唇微抿（压抑情绪）
    LipPress,
    /// 眼睛微睁（惊讶）
    EyeWiden,
}

/// 微表情实例
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicroExpression {
    /// 微表情类型
    pub kind: MicroExpressionType,
    /// 强度 0-1（微表情通常 0.1-0.4）
    pub intensity: f32,
    /// 持续时间（秒）— 微表情 0.04-0.2 秒
    pub duration_secs: f32,
    /// 在回复中的触发位置 0-1
    pub trigger_position: f32,
}

// ════════════════════════════════════════════════════════════════════
// BodyPosture — 体态
// ════════════════════════════════════════════════════════════════════

/// 体态参数 — 虚拟形象的身体语言
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyPosture {
    /// 头部倾斜角度（度）— 悲伤→-10°，好奇→+15°
    pub head_tilt: f32,
    /// 肩膀姿态 0-1 — 0=耸肩防御，1=放松展开
    pub shoulder_openness: f32,
    /// 身体前倾/后仰 — 正=前倾（关注），负=后仰（回避）
    pub lean: f32,
    /// 眼神接触倾向 0-1 — 自信→高，羞涩→低
    pub eye_contact: f32,
    /// 手势活跃度 0-1 — 兴奋→高，悲伤→低
    pub gesture_activity: f32,
    /// 呼吸频率因子 — 焦虑→1.5，平静→1.0
    pub breath_rate: f32,
}

impl BodyPosture {
    /// 中性体态
    pub fn neutral() -> Self {
        Self {
            head_tilt: 0.0,
            shoulder_openness: 0.5,
            lean: 0.0,
            eye_contact: 0.5,
            gesture_activity: 0.3,
            breath_rate: 1.0,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// KinesicsOutput — 体态输出
// ════════════════════════════════════════════════════════════════════

/// 体态映射输出 — 体态 + 微表情序列
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KinesicsOutput {
    /// 体态参数
    pub posture: BodyPosture,
    /// 微表情序列（按时间排序）
    pub micro_expressions: Vec<MicroExpression>,
}

// ════════════════════════════════════════════════════════════════════
// KinesicsMapper — 体态映射器
// ════════════════════════════════════════════════════════════════════

/// 体态映射器 — 从 PAD 映射到体态和微表情
pub struct KinesicsMapper;

impl KinesicsMapper {
    /// 从 PAD 映射到体态输出
    pub fn map(pad: [f32; 3], reply_length_estimate: usize) -> KinesicsOutput {
        let posture = Self::map_posture(pad);
        let micro_expressions = Self::map_micro_expressions(pad, reply_length_estimate);

        KinesicsOutput {
            posture,
            micro_expressions,
        }
    }

    /// PAD → 体态
    fn map_posture(pad: [f32; 3]) -> BodyPosture {
        let p = pad[0];
        let a = pad[1];
        let d = pad[2];

        // 头部倾斜：悲伤→偏一侧（负），好奇/兴奋→偏另一侧（正）
        let head_tilt = p * 10.0 + a * 5.0;

        // 肩膀：自信/喜悦→展开，悲伤/焦虑→收缩
        let shoulder_openness = 0.5 + p * 0.2 + d * 0.15 - a.min(0.0).abs() * 0.1;

        // 前倾/后仰：关注→前倾，回避→后仰
        let lean = d * 0.3 + p * 0.1 - (1.0 - p) * a.min(0.0).abs() * 0.2;

        // 眼神接触：自信→高，羞涩→低
        let eye_contact = 0.5 + d * 0.2 + p * 0.1;

        // 手势活跃度：兴奋→高，悲伤→低
        let gesture_activity = 0.3 + a * 0.2 + p * 0.1;

        // 呼吸频率：焦虑→快，平静→正常
        let breath_rate = 1.0 + a.min(0.0).abs() * 0.3 - p * 0.1;

        BodyPosture {
            head_tilt: head_tilt.clamp(-20.0, 20.0),
            shoulder_openness: shoulder_openness.clamp(0.0, 1.0),
            lean: lean.clamp(-0.5, 0.5),
            eye_contact: eye_contact.clamp(0.0, 1.0),
            gesture_activity: gesture_activity.clamp(0.0, 1.0),
            breath_rate: breath_rate.clamp(0.5, 2.0),
        }
    }

    /// PAD → 微表情序列
    ///
    /// 微表情是情绪的真实泄露，不受意识控制。
    /// 即使在"假装没事"时，微表情也会暴露真实情绪。
    fn map_micro_expressions(pad: [f32; 3], reply_length_estimate: usize) -> Vec<MicroExpression> {
        let mut expressions = Vec::new();
        let p = pad[0];
        let a = pad[1];
        let d = pad[2];

        // 微表情数量：短回复1-2个，长回复2-4个
        let max_count = if reply_length_estimate < 50 {
            1
        } else if reply_length_estimate < 200 {
            2
        } else {
            3
        };

        // 基于 PAD 生成微表情
        // 悲伤泄露：嘴角微下垂
        if p < -0.3 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::LipCornerDepress,
                intensity: (-p * 0.3).min(0.4),
                duration_secs: 0.1 + rand::random::<f32>() * 0.1,
                trigger_position: 0.2 + rand::random::<f32>() * 0.3,
            });
        }

        // 喜悦泄露：嘴角微上扬
        if p > 0.3 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::LipCornerPull,
                intensity: (p * 0.3).min(0.4),
                duration_secs: 0.08 + rand::random::<f32>() * 0.12,
                trigger_position: 0.3 + rand::random::<f32>() * 0.4,
            });
        }

        // 担忧/不满：眉头微蹙
        if p < -0.1 && a > 0.1 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::BrowFurrow,
                intensity: 0.15 + (-p * 0.2).min(0.25),
                duration_secs: 0.12 + rand::random::<f32>() * 0.08,
                trigger_position: 0.1 + rand::random::<f32>() * 0.2,
            });
        }

        // 压抑情绪：嘴唇微抿
        if d < -0.2 && p.abs() > 0.2 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::LipPress,
                intensity: 0.2 + (-d * 0.15).min(0.2),
                duration_secs: 0.15 + rand::random::<f32>() * 0.05,
                trigger_position: 0.5 + rand::random::<f32>() * 0.3,
            });
        }

        // 惊讶：眼睛微睁
        if a > 0.5 && p > 0.0 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::EyeWiden,
                intensity: (a * 0.25).min(0.35),
                duration_secs: 0.04 + rand::random::<f32>() * 0.06,
                trigger_position: rand::random::<f32>() * 0.3,
            });
        }

        // 怀疑/不悦：眼睛微眯
        if p < -0.2 && d > 0.1 {
            expressions.push(MicroExpression {
                kind: MicroExpressionType::EyeSquint,
                intensity: 0.15 + (-p * 0.1).min(0.2),
                duration_secs: 0.1 + rand::random::<f32>() * 0.1,
                trigger_position: 0.4 + rand::random::<f32>() * 0.3,
            });
        }

        // 截断到最大数量
        expressions.truncate(max_count);

        // 按 trigger_position 排序
        expressions.sort_by(|a, b| {
            a.trigger_position
                .partial_cmp(&b.trigger_position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        expressions
    }

    // ════════════════════════════════════════════════════════════════════
    // Prompt 片段生成 — 供 LLM 上下文注入
    // Prompt fragment generation — for LLM context injection
    // ════════════════════════════════════════════════════════════════════

    /// 体态描述词 — 从 BodyPosture 生成自然语言描述 / Body posture description words
    ///
    /// 将数值体态参数转换为 LLM 可理解的自然语言提示，
    /// 让 LLM 感知自己的体态状态，从而调整回复风格。
    fn posture_description(posture: &BodyPosture) -> String {
        let mut parts = Vec::new();

        // 头部倾斜 / Head tilt
        if posture.head_tilt < -5.0 {
            parts.push("头微微偏向一侧".to_string());
        } else if posture.head_tilt > 5.0 {
            parts.push("头微微好奇地倾斜".to_string());
        }

        // 肩膀姿态 / Shoulder posture
        if posture.shoulder_openness < 0.35 {
            parts.push("肩膀微微缩起（防御姿态）".to_string());
        } else if posture.shoulder_openness > 0.65 {
            parts.push("肩膀放松展开".to_string());
        }

        // 前倾/后仰 / Lean forward/backward
        if posture.lean > 0.15 {
            parts.push("身体微微前倾（关注）".to_string());
        } else if posture.lean < -0.15 {
            parts.push("身体微微后仰（保持距离）".to_string());
        }

        // 眼神接触 / Eye contact
        if posture.eye_contact > 0.65 {
            parts.push("眼神接触较强".to_string());
        } else if posture.eye_contact < 0.35 {
            parts.push("眼神接触较少（羞涩或回避）".to_string());
        }

        // 手势活跃度 / Gesture activity
        if posture.gesture_activity > 0.5 {
            parts.push("手势活跃".to_string());
        } else if posture.gesture_activity < 0.2 {
            parts.push("手势很少（安静）".to_string());
        }

        // 呼吸频率 / Breath rate
        if posture.breath_rate > 1.2 {
            parts.push("呼吸略快（紧张或激动）".to_string());
        }

        if parts.is_empty() {
            "体态中性放松".to_string()
        } else {
            parts.join("，")
        }
    }

    /// 微表情描述词 — 从微表情序列生成自然语言描述 / Micro-expression description words
    ///
    /// 微表情是情绪的真实泄露，即使"假装没事"也会暴露。
    /// 让 LLM 知道自己的微表情状态，增强回复的情感真实性。
    fn micro_expression_description(micro_expressions: &[MicroExpression]) -> String {
        if micro_expressions.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        for me in micro_expressions {
            let desc = match me.kind {
                // 嘴角微微上扬（喜悦泄露） / Lip corner pull (joy leak)
                MicroExpressionType::LipCornerPull => {
                    format!(
                        "嘴角有{:.0}%强度的上扬微表情（喜悦泄露）",
                        me.intensity * 100.0
                    )
                }
                // 嘴角微微下垂（悲伤泄露） / Lip corner depress (sadness leak)
                MicroExpressionType::LipCornerDepress => {
                    format!(
                        "嘴角有{:.0}%强度的下垂微表情（悲伤泄露）",
                        me.intensity * 100.0
                    )
                }
                // 眉头微蹙（担忧/不满） / Brow furrow (worry/dissatisfaction)
                MicroExpressionType::BrowFurrow => {
                    format!("眉头微蹙（{:.0}%强度，担忧或不满）", me.intensity * 100.0)
                }
                // 眼睛微眯（怀疑/不悦） / Eye squint (suspicion/displeasure)
                MicroExpressionType::EyeSquint => {
                    format!("眼睛微眯（{:.0}%强度，怀疑或不悦）", me.intensity * 100.0)
                }
                // 鼻翼微动（厌恶泄露） / Nose wrinkle (disgust leak)
                MicroExpressionType::NoseWrinkle => {
                    format!("鼻翼微动（{:.0}%强度，厌恶泄露）", me.intensity * 100.0)
                }
                // 嘴唇微抿（压抑情绪） / Lip press (suppressing emotion)
                MicroExpressionType::LipPress => {
                    format!("嘴唇微抿（{:.0}%强度，压抑情绪）", me.intensity * 100.0)
                }
                // 眼睛微睁（惊讶） / Eye widen (surprise)
                MicroExpressionType::EyeWiden => {
                    format!("眼睛微睁（{:.0}%强度，惊讶）", me.intensity * 100.0)
                }
            };
            parts.push(desc);
        }

        format!("微表情泄露：{}", parts.join("；"))
    }

    /// 生成 LLM prompt 片段 — 体态与微表情上下文 / Generate LLM prompt fragment — kinesics context
    ///
    /// 将体态映射结果转换为自然语言 prompt，注入 LLM 上下文。
    /// 让 LLM 感知自己当前的体态和微表情状态，
    /// 从而让回复风格与身体语言保持一致。
    ///
    /// # 示例输出 / Example output
    ///
    /// ```text
    /// [体态] 身体微微前倾（关注），眼神接触较强；微表情泄露：嘴角有20%强度的上扬微表情（喜悦泄露）
    /// ```
    pub fn to_prompt_fragment(output: &KinesicsOutput) -> String {
        let posture_desc = Self::posture_description(&output.posture);
        let me_desc = Self::micro_expression_description(&output.micro_expressions);

        if me_desc.is_empty() {
            format!("[体态] {}", posture_desc)
        } else {
            format!("[体态] {}；{}", posture_desc, me_desc)
        }
    }

    /// 生成体态动画指令（供渲染引擎使用）
    ///
    /// 输出格式：JSON 指令序列，描述虚拟形象的动画参数。
    pub fn to_animation_commands(output: &KinesicsOutput) -> String {
        let mut commands = Vec::new();

        // 体态指令
        commands.push(format!(
            "{{\"type\":\"posture\",\"head_tilt\":{:.1},\"shoulder\":{:.2},\"lean\":{:.2},\"eye_contact\":{:.2},\"gesture\":{:.2},\"breath_rate\":{:.2}}}",
            output.posture.head_tilt,
            output.posture.shoulder_openness,
            output.posture.lean,
            output.posture.eye_contact,
            output.posture.gesture_activity,
            output.posture.breath_rate,
        ));

        // 微表情指令
        for me in &output.micro_expressions {
            let kind_str = match me.kind {
                MicroExpressionType::LipCornerPull => "lip_corner_pull",
                MicroExpressionType::LipCornerDepress => "lip_corner_depress",
                MicroExpressionType::BrowFurrow => "brow_furrow",
                MicroExpressionType::EyeSquint => "eye_squint",
                MicroExpressionType::NoseWrinkle => "nose_wrinkle",
                MicroExpressionType::LipPress => "lip_press",
                MicroExpressionType::EyeWiden => "eye_widen",
            };
            commands.push(format!(
                "{{\"type\":\"micro_expression\",\"kind\":\"{}\",\"intensity\":{:.2},\"duration\":{:.3},\"position\":{:.2}}}",
                kind_str, me.intensity, me.duration_secs, me.trigger_position,
            ));
        }

        format!("[{}]", commands.join(","))
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinesics_neutral() {
        let output = KinesicsMapper::map([0.0, 0.0, 0.0], 100);
        // 中性 PAD → 接近中性体态
        assert!(output.posture.head_tilt.abs() < 5.0);
        assert!((output.posture.shoulder_openness - 0.5).abs() < 0.2);
        assert!(output.posture.lean.abs() < 0.2);
    }

    #[test]
    fn test_kinesics_sad() {
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 100);
        // 悲伤：头偏、肩缩、后仰
        assert!(
            output.posture.head_tilt < -3.0,
            "sad head tilt: {}",
            output.posture.head_tilt
        );
        assert!(
            output.posture.shoulder_openness < 0.5,
            "sad shoulders: {}",
            output.posture.shoulder_openness
        );
        assert!(
            output.posture.gesture_activity < 0.3,
            "sad gestures: {}",
            output.posture.gesture_activity
        );
        // 应有嘴角下垂微表情
        assert!(
            output
                .micro_expressions
                .iter()
                .any(|me| me.kind == MicroExpressionType::LipCornerDepress),
            "sad should have lip corner depress"
        );
    }

    #[test]
    fn test_kinesics_joy() {
        let output = KinesicsMapper::map([0.7, 0.5, 0.4], 100);
        // 喜悦：肩展、高眼神接触
        assert!(
            output.posture.shoulder_openness > 0.5,
            "joy shoulders: {}",
            output.posture.shoulder_openness
        );
        assert!(
            output.posture.eye_contact > 0.5,
            "joy eye contact: {}",
            output.posture.eye_contact
        );
        // 应有嘴角上扬微表情
        assert!(
            output
                .micro_expressions
                .iter()
                .any(|me| me.kind == MicroExpressionType::LipCornerPull),
            "joy should have lip corner pull"
        );
    }

    #[test]
    fn test_kinesics_anger() {
        let output = KinesicsMapper::map([-0.6, 0.7, 0.6], 100);
        // 愤怒：眉头蹙、高手势
        assert!(
            output.posture.gesture_activity > 0.3,
            "anger gestures: {}",
            output.posture.gesture_activity
        );
        // 可能有眉头蹙或眼睛微眯
        let has_furrow_or_squint = output.micro_expressions.iter().any(|me| {
            me.kind == MicroExpressionType::BrowFurrow || me.kind == MicroExpressionType::EyeSquint
        });
        assert!(
            has_furrow_or_squint,
            "anger should have brow furrow or eye squint"
        );
    }

    #[test]
    fn test_kinesics_short_reply_fewer_micro() {
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 30);
        assert!(
            output.micro_expressions.len() <= 1,
            "short reply should have at most 1 micro expression"
        );
    }

    #[test]
    fn test_kinesics_long_reply_more_micro() {
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 300);
        assert!(
            !output.micro_expressions.is_empty(),
            "long reply should have micro expressions"
        );
    }

    #[test]
    fn test_kinesics_micro_expression_duration() {
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 200);
        for me in &output.micro_expressions {
            // 微表情持续时间应在 0.04-0.2 秒范围
            assert!(
                me.duration_secs >= 0.04 && me.duration_secs <= 0.25,
                "micro expression duration should be 0.04-0.25s, got {}",
                me.duration_secs
            );
        }
    }

    #[test]
    fn test_kinesics_micro_expression_intensity() {
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 200);
        for me in &output.micro_expressions {
            // 微表情强度通常 0.1-0.4
            assert!(
                me.intensity > 0.0 && me.intensity <= 0.5,
                "micro expression intensity should be 0-0.5, got {}",
                me.intensity
            );
        }
    }

    #[test]
    fn test_kinesics_animation_commands() {
        let output = KinesicsMapper::map([0.3, 0.1, 0.0], 100);
        let cmds = KinesicsMapper::to_animation_commands(&output);
        assert!(cmds.starts_with('['));
        assert!(cmds.contains("\"type\":\"posture\""));
    }

    #[test]
    fn test_kinesics_clamps() {
        let output = KinesicsMapper::map([1.0, 1.0, 1.0], 100);
        let p = &output.posture;
        assert!(p.head_tilt >= -20.0 && p.head_tilt <= 20.0);
        assert!(p.shoulder_openness >= 0.0 && p.shoulder_openness <= 1.0);
        assert!(p.lean >= -0.5 && p.lean <= 0.5);
        assert!(p.eye_contact >= 0.0 && p.eye_contact <= 1.0);
        assert!(p.gesture_activity >= 0.0 && p.gesture_activity <= 1.0);
        assert!(p.breath_rate >= 0.5 && p.breath_rate <= 2.0);
    }

    // ── prompt_fragment 测试 / prompt_fragment tests ──

    #[test]
    fn test_kinesics_prompt_fragment_neutral() {
        // 中性 PAD → 体态中性描述 / Neutral PAD → neutral posture description
        let output = KinesicsMapper::map([0.0, 0.0, 0.0], 100);
        let fragment = KinesicsMapper::to_prompt_fragment(&output);
        assert!(
            fragment.starts_with("[体态]"),
            "fragment should start with [体态]"
        );
        assert!(
            fragment.contains("体态中性放松"),
            "neutral should mention relaxed posture"
        );
    }

    #[test]
    fn test_kinesics_prompt_fragment_sad() {
        // 悲伤 PAD → 防御姿态 + 悲伤微表情泄露 / Sad PAD → defensive posture + sadness leak
        let output = KinesicsMapper::map([-0.7, -0.3, -0.5], 200);
        let fragment = KinesicsMapper::to_prompt_fragment(&output);
        assert!(
            fragment.starts_with("[体态]"),
            "fragment should start with [体态]"
        );
        assert!(
            fragment.contains("微表情泄露"),
            "sad should mention micro-expression leak"
        );
        assert!(
            fragment.contains("悲伤泄露"),
            "sad should mention sadness leak"
        );
    }

    #[test]
    fn test_kinesics_prompt_fragment_joy() {
        // 喜悦 PAD → 放松展开 + 喜悦微表情 / Joy PAD → relaxed posture + joy micro-expression
        let output = KinesicsMapper::map([0.7, 0.5, 0.4], 200);
        let fragment = KinesicsMapper::to_prompt_fragment(&output);
        assert!(
            fragment.starts_with("[体态]"),
            "fragment should start with [体态]"
        );
        assert!(
            fragment.contains("微表情泄露"),
            "joy should mention micro-expression leak"
        );
        assert!(fragment.contains("喜悦泄露"), "joy should mention joy leak");
    }

    #[test]
    fn test_kinesics_prompt_fragment_anger() {
        // 愤怒 PAD → 微表情泄露（眉头/嘴角/眼睛） / Anger PAD → micro-expression leak
        // 注意：愤怒PAD的体态公式中 gesture_activity 和 breath_rate
        // 受负愉悦影响可能不触发"手势活跃"/"呼吸略快"阈值，
        // 但微表情（眉头微蹙/嘴角下垂/眼睛微眯）一定会泄露
        let output = KinesicsMapper::map([-0.6, 0.7, 0.6], 200);
        let fragment = KinesicsMapper::to_prompt_fragment(&output);
        assert!(
            fragment.starts_with("[体态]"),
            "fragment should start with [体态]"
        );
        // 愤怒应有微表情泄露 / Anger should have micro-expression leak
        assert!(
            fragment.contains("微表情泄露"),
            "anger should have micro-expression leak: {}",
            fragment
        );
        // 愤怒微表情包含不满或不悦 / Anger micro-expression should show displeasure
        let has_negative =
            fragment.contains("不满") || fragment.contains("不悦") || fragment.contains("悲伤泄露");
        assert!(
            has_negative,
            "anger should show negative micro-expression: {}",
            fragment
        );
    }

    #[test]
    fn test_kinesics_prompt_fragment_no_micro_short_reply() {
        // 极短回复 → 无微表情 → 无微表情泄露部分 / Very short reply → no leak section
        let output = KinesicsMapper::map([0.0, 0.0, 0.0], 10);
        let fragment = KinesicsMapper::to_prompt_fragment(&output);
        assert!(
            !fragment.contains("微表情泄露"),
            "short neutral should have no micro-expression leak"
        );
    }
}
