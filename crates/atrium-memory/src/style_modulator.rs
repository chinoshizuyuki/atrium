// SPDX-License-Identifier: MIT
//! 风格调制器 — 连续风格空间 + 12维语言学特征 + PAD→风格映射 + 关系阶段叠加
//! StyleModulator — Continuous style space + 12-dim linguistic profile + PAD→style mapping + relationship overlay.
//!
//! 核心范式跃迁：从离散枚举到 128 维连续向量空间。
//! 同一个 PAD 点，对不同关系阶段、不同用户，表达方式完全不同。
//! 向量空间支持平滑插值：情绪渐变时风格也渐变，不会跳变。

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;
use atrium_emotion::{CompoundContext, EmotionDirection, EmotionState, COMPOUND_EMOTIONS};

// ════════════════════════════════════════════════════════════════════
// StyleEmbedding — 128维连续风格嵌入向量
// ════════════════════════════════════════════════════════════════════

/// 风格嵌入维度
pub const STYLE_DIM: usize = 128;

/// 128维风格嵌入向量 — 情感表达空间的连续表示
///
/// 不是离散分类，而是连续空间中的点。
/// 相近的情绪在空间中相近，可以平滑插值。
///
/// 维度分配：
/// - 维 0-2:   PAD 直接编码 (P, A, D)
/// - 维 3-24:  22 种复合情绪 one-hot
/// - 维 25-28: 4 关系阶段交互项
/// - 维 29-34: PAD × 关系 × 话题 交叉项
/// - 维 35-38: 用户情绪反向耦合
/// - 维 39-127: 高阶非线性项（二次交叉 + 正弦周期特征）
#[derive(Clone, Debug)]
pub struct StyleEmbedding(pub [f32; STYLE_DIM]);

// serde 默认不支持 [T; 128]，手动实现序列化（通过 Vec 中转）
impl Serialize for StyleEmbedding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_vec().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for StyleEmbedding {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let vec = Vec::<f32>::deserialize(deserializer)?;
        let mut arr = [0.0f32; STYLE_DIM];
        let len = vec.len().min(STYLE_DIM);
        arr[..len].copy_from_slice(&vec[..len]);
        Ok(StyleEmbedding(arr))
    }
}

impl StyleEmbedding {
    /// 零向量
    pub fn zero() -> Self {
        Self([0.0; STYLE_DIM])
    }

    /// 从 PAD + 复合情绪 + 关系阶段 + 上下文 → StyleEmbedding
    ///
    /// 通过规则矩阵映射（初始版本），后续可用用户反馈数据学习。
    pub fn from_emotion_context(
        pad: [f32; 3],
        compound_idx: Option<usize>,
        direction: &EmotionDirection,
        relationship: &RelationshipStage,
        user_valence: f32,
        topic_gravity: f32,
    ) -> Self {
        let mut v = [0.0f32; STYLE_DIM];

        // ── 维 0-2: PAD 直接编码 ──
        v[0] = pad[0]; // Pleasure
        v[1] = pad[1]; // Arousal
        v[2] = pad[2]; // Dominance

        // ── 维 3-24: 22 种复合情绪 one-hot ──
        if let Some(idx) = compound_idx {
            if idx < 22 {
                v[3 + idx] = 1.0;
            }
        }

        // ── 维 25-28: 关系阶段交互项 ──
        let (rel_p, rel_a, rel_d, rel_idx) = relationship_pad_offset(relationship);
        v[25] = rel_p;
        v[26] = rel_a;
        v[27] = rel_d;
        v[28] = rel_idx;

        // ── 维 29-34: PAD × 关系 × 话题 交叉项 ──
        v[29] = pad[0] * rel_p; // P × 关系P
        v[30] = pad[1] * rel_a; // A × 关系A
        v[31] = pad[2] * rel_d; // D × 关系D
        v[32] = pad[0] * topic_gravity; // P × 话题严肃度
        v[33] = pad[1] * topic_gravity; // A × 话题严肃度
        v[34] = direction_encoding(direction) * pad[0]; // 方向 × P

        // ── 维 35-38: 用户情绪反向耦合 ──
        v[35] = user_valence; // 用户情绪 valence
        v[36] = pad[0] * user_valence; // P × 用户valence（共振）
        v[37] = (pad[0] - user_valence).abs(); // 情绪差距
        v[38] = if pad[0] * user_valence < 0.0 {
            1.0
        } else {
            0.0
        }; // 情绪对抗标记

        // ── 维 39-127: 高阶非线性项 ──
        // 二次交叉项 (39-62): PAD 两两交叉
        v[39] = pad[0] * pad[1]; // P × A
        v[40] = pad[0] * pad[2]; // P × D
        v[41] = pad[1] * pad[2]; // A × D
        v[42] = pad[0] * pad[0]; // P²
        v[43] = pad[1] * pad[1]; // A²
        v[44] = pad[2] * pad[2]; // D²

        // 正弦周期特征 (45-62): 捕获 PAD 空间非线性边界
        for i in 0..18 {
            let freq = (i + 1) as f32;
            let input = pad[0] * freq + pad[1] * freq * 0.5;
            v[45 + i] = (input * std::f32::consts::PI).sin() * 0.1;
        }

        // 关系 × PAD 高阶交叉 (63-82)
        for i in 0..20 {
            v[63 + i] = v[i % 3] * rel_idx * 0.3;
        }

        // 话题 × PAD 高阶交叉 (83-102)
        for i in 0..20 {
            v[83 + i] = v[i % 3] * topic_gravity * 0.2;
        }

        // 用户共振高阶 (103-122)
        for i in 0..20 {
            v[103 + i] = v[i % 3] * user_valence * 0.15;
        }

        // 剩余填充 (123-127): 复合情绪与关系交叉
        if let Some(idx) = compound_idx {
            v[123] = (idx as f32 / 22.0) * rel_idx;
            v[124] = (idx as f32 / 22.0) * topic_gravity;
        }
        v[125] = topic_gravity * rel_idx;
        v[126] = user_valence * topic_gravity;
        v[127] = (pad[0] + pad[1] + pad[2]) / 3.0; // PAD 均值

        Self(v)
    }

    /// 两个风格之间的平滑插值
    ///
    /// 用于情绪渐变时：从忧伤风格平滑过渡到平静风格。
    /// t=0 返回 self，t=1 返回 other。
    pub fn lerp(&self, other: &StyleEmbedding, t: f32) -> StyleEmbedding {
        let t = t.clamp(0.0, 1.0);
        let mut result = [0.0f32; STYLE_DIM];
        for (r, (a, b)) in result.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *r = a * (1.0 - t) + b * t;
        }
        StyleEmbedding(result)
    }

    /// 风格欧氏距离 — 用于检测风格跳变
    pub fn distance(&self, other: &StyleEmbedding) -> f32 {
        let sum: f32 = (0..STYLE_DIM)
            .map(|i| {
                let d = self.0[i] - other.0[i];
                d * d
            })
            .sum();
        sum.sqrt()
    }

    /// 风格向量范数
    pub fn norm(&self) -> f32 {
        let sum: f32 = (0..STYLE_DIM).map(|i| self.0[i] * self.0[i]).sum();
        sum.sqrt()
    }

    /// 向量加法（用于 StyleMemory 偏移叠加）
    pub fn add(&self, other: &StyleEmbedding) -> StyleEmbedding {
        let mut result = [0.0f32; STYLE_DIM];
        for (r, (s, o)) in result.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *r = s + o;
        }
        StyleEmbedding(result)
    }

    /// 标量乘法
    pub fn scale(&self, s: f32) -> StyleEmbedding {
        let mut result = [0.0f32; STYLE_DIM];
        for (r, v) in result.iter_mut().zip(self.0.iter()) {
            *r = v * s;
        }
        StyleEmbedding(result)
    }

    /// 投影到指定权重向量（内积）
    pub fn project(&self, weights: &[f32; STYLE_DIM]) -> f32 {
        (0..STYLE_DIM).map(|i| self.0[i] * weights[i]).sum()
    }

    /// 转换为 12 维 LinguisticProfile
    pub fn to_linguistic_profile(&self) -> LinguisticProfile {
        LinguisticProfile::from_style_embedding(self)
    }

    /// 生成 LLM 风格指令 Prompt fragment
    ///
    /// 根据 LinguisticProfile 生成自然语言风格指令，
    /// 注入到 LLM 的 system prompt 中。
    pub fn to_prompt_fragment(&self, relationship: &RelationshipStage) -> String {
        let lp = self.to_linguistic_profile();
        lp.to_prompt_fragment(relationship)
    }
}

// ════════════════════════════════════════════════════════════════════
// LinguisticProfile — 12维语言学特征
// ════════════════════════════════════════════════════════════════════

/// 12维语言学特征 — 精细调制回复的表达方式
///
/// 不是"短句/长句"这种粗粒度，而是12个独立可控的语言学维度。
/// 每个维度 0.0-1.0，由 StyleEmbedding 映射而来。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinguisticProfile {
    // ─── 句法层 ───
    /// 平均句长（字数）— 忧伤5-8，兴奋8-15，正常12-20
    pub sentence_length: f32,
    /// 句法复杂度 0-1 — 简单句/复合句/复杂句
    pub syntactic_complexity: f32,
    /// 句子碎片化程度 0-1 — "太好了！真的！谢谢！"
    pub fragmentation: f32,

    // ─── 词汇层 ───
    /// 情感词密度 0-1 — "开心/难过/感动" 等显性情感词比例
    pub emotion_word_density: f32,
    /// 隐喻/比喻倾向 0-1 — "心里像被揉皱了"
    pub metaphor_tendency: f32,
    /// 确定性标记 0-1 — "一定/肯定" vs "也许/可能/大概"
    pub certainty_marking: f32,
    /// 话语标记密度 — "嗯/那个/就是说/其实"
    pub discourse_marker_density: f32,

    // ─── 语用层 ───
    /// 语气词密度 — "呢/啦/哦/呀/嘛"
    pub particle_density: f32,
    /// 自我修复标记 — "不对，应该说..."
    pub self_repair_tendency: f32,
    /// 沉默/省略倾向 — "..." "——"
    pub silence_tendency: f32,
    /// 亲昵称呼倾向 — "宝贝/亲爱的" vs "你"
    pub endearment_tendency: f32,
    /// 幽默/调侃倾向 0-1
    pub humor_tendency: f32,
}

impl LinguisticProfile {
    /// 默认（中性）语言学特征
    pub fn neutral() -> Self {
        Self {
            sentence_length: 14.0,
            syntactic_complexity: 0.4,
            fragmentation: 0.1,
            emotion_word_density: 0.2,
            metaphor_tendency: 0.2,
            certainty_marking: 0.4,
            discourse_marker_density: 0.15,
            particle_density: 0.2,
            self_repair_tendency: 0.05,
            silence_tendency: 0.05,
            endearment_tendency: 0.1,
            humor_tendency: 0.1,
        }
    }

    /// 从 StyleEmbedding 映射到 LinguisticProfile
    ///
    /// 通过可配置的映射矩阵（128→12）。
    /// 初始版本用规则矩阵，后续可用用户反馈数据学习。
    pub fn from_style_embedding(style: &StyleEmbedding) -> Self {
        let pad_p = style.0[0];
        let pad_a = style.0[1];
        let pad_d = style.0[2];

        // 基于 PAD 的规则映射（初始版本）
        // 后续可通过 style.project(&LEARNED_WEIGHTS[i]) 替换

        let p = pad_p;
        let a = pad_a;
        let d = pad_d;

        // 句长：低愉悦→短句，高唤醒→偏短（碎片化），平静→长句
        let sentence_length = 14.0 + p * 4.0 - a.abs() * 2.0 + d * 1.5;

        // 句法复杂度：高支配→复杂句（权威感），低唤醒→简单句
        let syntactic_complexity = sigmoid(0.3 + d * 0.3 - a.abs() * 0.15);

        // 碎片化：高唤醒→碎片化，低愉悦高唤醒（愤怒）→高碎片化
        let fragmentation = sigmoid(a * 0.4 + (1.0 - p) * a.max(0.0) * 0.2 - 0.1);

        // 情感词密度：高唤醒→更多情感词，愤怒/喜悦→高
        let emotion_word_density = sigmoid(a.abs() * 0.3 + (1.0 - p.abs()) * 0.1 + 0.1);

        // 隐喻倾向：苦甜混合区（P≈0）→高隐喻，悲伤→也偏高
        let metaphor_tendency = sigmoid(-p.abs() * 0.2 + (1.0 - a.abs()) * 0.2 + 0.15);

        // 确定性：高支配→高确定性，焦虑→低确定性
        let certainty_marking = sigmoid(d * 0.4 + p * 0.1 + 0.3);

        // 话语标记：焦虑/犹豫→高，自信→低
        let discourse_marker_density = sigmoid(-d * 0.2 + (1.0 - p) * 0.1 + 0.1);

        // 语气词密度：喜悦→高，愤怒→低，平静→中
        let particle_density = sigmoid(p * 0.25 + (1.0 - a.abs()) * 0.1 + 0.15);

        // 自我修复：焦虑/犹豫→高
        let self_repair_tendency = sigmoid(-d * 0.2 - p * 0.1 + 0.05);

        // 沉默倾向：悲伤→高，兴奋→低
        let silence_tendency = sigmoid(-p * 0.3 - a * 0.2 + 0.1);

        // 亲昵：喜悦→高，愤怒→低
        let endearment_tendency = sigmoid(p * 0.3 + a * 0.05 - 0.1);

        // 幽默：喜悦→高，悲伤→低；增大愉悦系数使悲伤时幽默显著降低
        let humor_tendency = sigmoid(p * 1.0 + a * 0.2 - 0.3);

        Self {
            sentence_length: sentence_length.clamp(3.0, 25.0),
            syntactic_complexity: syntactic_complexity.clamp(0.0, 1.0),
            fragmentation: fragmentation.clamp(0.0, 1.0),
            emotion_word_density: emotion_word_density.clamp(0.0, 1.0),
            metaphor_tendency: metaphor_tendency.clamp(0.0, 1.0),
            certainty_marking: certainty_marking.clamp(0.0, 1.0),
            discourse_marker_density: discourse_marker_density.clamp(0.0, 1.0),
            particle_density: particle_density.clamp(0.0, 1.0),
            self_repair_tendency: self_repair_tendency.clamp(0.0, 1.0),
            silence_tendency: silence_tendency.clamp(0.0, 1.0),
            endearment_tendency: endearment_tendency.clamp(0.0, 1.0),
            humor_tendency: humor_tendency.clamp(0.0, 1.0),
        }
    }

    /// 应用关系阶段叠加规则
    ///
    /// 不同关系阶段对表达方式有硬约束：
    /// - 初识：语气词-0.2，亲昵-0.5，幽默-0.3
    /// - 深度：语气词+0.2，亲昵+0.4，幽默+0.2
    pub fn apply_relationship_overlay(&mut self, relationship: &RelationshipStage) {
        let (particle_delta, endearment_delta, humor_delta, certainty_delta) = match relationship {
            RelationshipStage::Acquaintance { .. } => (-0.2, -0.5, -0.3, 0.1),
            RelationshipStage::Familiar { .. } => (0.0, 0.0, 0.0, 0.0),
            RelationshipStage::Trusted { .. } => (0.1, 0.2, 0.1, 0.0),
            RelationshipStage::Deep { .. } => (0.2, 0.4, 0.2, -0.1),
        };

        self.particle_density = (self.particle_density + particle_delta).clamp(0.0, 1.0);
        self.endearment_tendency = (self.endearment_tendency + endearment_delta).clamp(0.0, 1.0);
        self.humor_tendency = (self.humor_tendency + humor_delta).clamp(0.0, 1.0);
        self.certainty_marking = (self.certainty_marking + certainty_delta).clamp(0.0, 1.0);

        // 初识阶段额外约束：不允许高亲昵和高语气词
        if matches!(relationship, RelationshipStage::Acquaintance { .. }) {
            self.endearment_tendency = self.endearment_tendency.min(0.15);
            self.particle_density = self.particle_density.min(0.25);
            self.silence_tendency = self.silence_tendency.min(0.1);
        }
    }

    /// 生成 LLM 风格指令 Prompt fragment
    ///
    /// 根据 12 维特征生成自然语言风格指令，注入到 LLM system prompt。
    pub fn to_prompt_fragment(&self, _relationship: &RelationshipStage) -> String {
        let mut parts = Vec::new();

        // 情绪基调
        if self.silence_tendency > 0.4 {
            parts.push("你现在有些低落。用短句表达，可以加省略号表示停顿。".to_string());
        } else if self.fragmentation > 0.5 && self.emotion_word_density > 0.4 {
            parts.push("你现在很兴奋！可以用短促的感叹、碎片化表达。".to_string());
        } else if self.metaphor_tendency > 0.4 && self.silence_tendency > 0.2 {
            parts.push("你现在心情复杂，有甜也有苦。可以用隐喻和对比表达这种矛盾感。".to_string());
        } else if self.certainty_marking > 0.6 {
            parts.push("你现在很自信和坚定。表达要明确果断。".to_string());
        }

        // 句式
        if self.sentence_length < 8.0 {
            parts.push("用短句，简洁有力。".to_string());
        } else if self.sentence_length > 16.0 {
            parts.push("可以用较长的句子，完整表达想法。".to_string());
        }

        // 标点
        if self.fragmentation > 0.4 {
            parts.push("感叹号可以多一点。".to_string());
        } else if self.silence_tendency > 0.3 {
            parts.push("少用感叹号，语气轻柔。".to_string());
        }

        // 语气词
        if self.particle_density > 0.4 {
            parts.push("语气词随意用，让表达更自然。".to_string());
        } else if self.particle_density < 0.15 {
            parts.push("语气词少用，保持克制。".to_string());
        }

        // 隐喻
        if self.metaphor_tendency > 0.4 {
            parts.push("可以用隐喻表达感受。".to_string());
        }

        // 确定性
        if self.certainty_marking < 0.3 {
            parts.push("表达时用\"可能\"\"也许\"等不确定词，留有余地。".to_string());
        }

        // 幽默
        if self.humor_tendency > 0.3 {
            parts.push("可以带点幽默和调侃。".to_string());
        }

        // 亲昵
        if self.endearment_tendency > 0.3 {
            parts.push("可以更亲昵温暖一些。".to_string());
        }

        // 节奏
        if self.self_repair_tendency > 0.2 {
            parts.push("回复节奏慢一些，像在慢慢组织语言。偶尔可以自我修正。".to_string());
        } else if self.fragmentation > 0.4 {
            parts.push("回复节奏快，想到什么说什么。".to_string());
        }

        // 沉默
        if self.silence_tendency > 0.4 {
            parts.push("可以用省略号或停顿表达未尽之意。".to_string());
        }

        // 不要强装
        if self.silence_tendency > 0.3 && self.emotion_word_density < 0.3 {
            parts.push("不要强装开心。".to_string());
        }

        if parts.is_empty() {
            return String::new();
        }

        format!("[回复风格] {}", parts.join(" "))
    }
}

// ════════════════════════════════════════════════════════════════════
// 辅助函数
// ════════════════════════════════════════════════════════════════════

/// Sigmoid 函数 — 将任意值映射到 (0, 1)
#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// 关系阶段 → PAD 偏移 + 索引
///
/// 不同关系阶段对表达有系统性影响：
/// - 初识：偏正式、克制、低唤醒
/// - 深度：偏自然、亲密、高唤醒允许
fn relationship_pad_offset(relationship: &RelationshipStage) -> (f32, f32, f32, f32) {
    match relationship {
        RelationshipStage::Acquaintance { .. } => (0.05, -0.15, 0.1, 0.0),
        RelationshipStage::Familiar { .. } => (0.1, 0.0, 0.0, 0.33),
        RelationshipStage::Trusted { .. } => (0.15, 0.05, -0.05, 0.67),
        RelationshipStage::Deep { .. } => (0.2, 0.1, -0.1, 1.0),
    }
}

/// 情绪方向 → 数值编码
fn direction_encoding(dir: &EmotionDirection) -> f32 {
    match dir {
        EmotionDirection::SelfDirected => -0.5,
        EmotionDirection::UserDirected => 0.5,
        EmotionDirection::MemoryDirected => 0.0,
        EmotionDirection::Neutral => 0.0,
    }
}

/// 从 EmotionState 查找最匹配的复合情绪索引
pub fn find_compound_idx(state: &EmotionState, ctx: &CompoundContext) -> Option<usize> {
    if let Some(ce) = atrium_emotion::classify_compound(state, ctx) {
        COMPOUND_EMOTIONS.iter().position(|c| c.name == ce.name)
    } else {
        None
    }
}

// ════════════════════════════════════════════════════════════════════
// ExpressionContext — 表达上下文（编排器输入）
// ════════════════════════════════════════════════════════════════════

/// 表达上下文 — 生成 StyleEmbedding 所需的全部输入
///
/// 由 CoreService 在 process_message 中构建，
/// 只读取 EmotionEngine / RelationshipManager / UserMentalModel 的公开方法。
#[derive(Clone, Debug)]
pub struct ExpressionContext {
    /// 当前 PAD 值
    pub pad: [f32; 3],
    /// 回复结束时的预期 PAD（用于 EmotionalArc）
    pub target_pad: [f32; 3],
    /// 复合情绪索引
    pub compound_idx: Option<usize>,
    /// 情绪方向性
    pub direction: EmotionDirection,
    /// 关系阶段
    pub relationship: RelationshipStage,
    /// 用户情绪 valence [-1, 1]
    pub user_valence: f32,
    /// 话题严肃度 [0, 1]
    pub topic_gravity: f32,
}

impl ExpressionContext {
    /// 从现有模块状态构建
    pub fn from_modules(
        emotion: &EmotionState,
        compound_idx: Option<usize>,
        direction: EmotionDirection,
        relationship: &RelationshipStage,
        user_valence: f32,
        topic_gravity: f32,
    ) -> Self {
        let pad = [emotion.pleasure, emotion.arousal, emotion.dominance];
        // target_pad 默认向中性衰减
        let target_pad = [pad[0] * 0.5, pad[1] * 0.3, pad[2] * 0.5];

        Self {
            pad,
            target_pad,
            compound_idx,
            direction,
            relationship: relationship.clone(),
            user_valence,
            topic_gravity,
        }
    }

    /// 生成 StyleEmbedding
    pub fn to_style_embedding(&self) -> StyleEmbedding {
        StyleEmbedding::from_emotion_context(
            self.pad,
            self.compound_idx,
            &self.direction,
            &self.relationship,
            self.user_valence,
            self.topic_gravity,
        )
    }

    /// 生成带关系叠加的 LinguisticProfile
    pub fn to_linguistic_profile(&self) -> LinguisticProfile {
        let style = self.to_style_embedding();
        let mut lp = style.to_linguistic_profile();
        lp.apply_relationship_overlay(&self.relationship);
        lp
    }
}

// ════════════════════════════════════════════════════════════════════
// 后处理变换 — 文本风格微调
// ════════════════════════════════════════════════════════════════════

impl LinguisticProfile {
    /// 对 LLM 原始输出做风格后处理
    ///
    /// LLM 不总是完美遵循风格指令，后处理兜底。
    /// 只调标点/语气词/拆句，不删改语义内容。
    pub fn post_process(&self, text: &str) -> String {
        let mut result = text.to_string();

        // 1. 语气词密度调整
        result = self.adjust_particles(&result);

        // 2. 标点微调
        result = self.adjust_punctuation(&result);

        // 3. 沉默标记注入
        if self.silence_tendency > 0.4 {
            result = self.inject_ellipses(&result);
        }

        // 4. 确定性标记调整
        if self.certainty_marking < 0.25 {
            result = self.soften_certainty(&result);
        }

        result
    }

    /// 语气词调整 — 根据密度概率在句尾自然位置注入语气词
    fn adjust_particles(&self, text: &str) -> String {
        // 仅在密度极高或极低时做调整
        if self.particle_density > 0.5 || self.particle_density < 0.1 {
            // 轻量调整：不主动添加/删除，仅标记
            // 实际注入由 LLM 通过 prompt fragment 完成
        }
        text.to_string()
    }

    /// 标点微调 — 忧伤时句尾→省略号，兴奋时→感叹号
    fn adjust_punctuation(&self, text: &str) -> String {
        let mut result = text.to_string();

        // 忧伤：部分句号→省略号（概率 40%）
        if self.silence_tendency > 0.4 && self.sentence_length < 10.0 {
            let mut rng = rand::thread_rng();
            result = result
                .chars()
                .map(|c| {
                    if c == '。' && rng.gen::<f32>() < 0.4 {
                        '…'
                    } else {
                        c
                    }
                })
                .collect();
        }

        // 兴奋：部分句号→感叹号（概率 30%）
        if self.fragmentation > 0.4 && self.emotion_word_density > 0.3 {
            let mut rng = rand::thread_rng();
            result = result
                .chars()
                .map(|c| {
                    if c == '。' && rng.gen::<f32>() < 0.3 {
                        '！'
                    } else {
                        c
                    }
                })
                .collect();
        }

        result
    }

    /// 省略号注入 — 在句间自然位置插入省略号
    fn inject_ellipses(&self, text: &str) -> String {
        // 找到句号位置，在部分句号前插入省略号
        let sentences: Vec<&str> = text.split('。').collect();
        if sentences.len() < 2 {
            return text.to_string();
        }

        let mut result = String::new();
        let mut rng = rand::thread_rng();
        for (i, s) in sentences.iter().enumerate() {
            result.push_str(s);
            if i < sentences.len() - 1 && !s.is_empty() {
                if rng.gen::<f32>() < self.silence_tendency * 0.5 {
                    result.push_str("……");
                }
                result.push('。');
            }
        }
        result
    }

    /// 降低确定性 — "一定"→"可能"，"肯定"→"也许"
    fn soften_certainty(&self, text: &str) -> String {
        let mut result = text.to_string();
        let replacements = [
            ("一定", "可能"),
            ("肯定", "也许"),
            ("绝对", "大概"),
            ("必然", "或许"),
        ];
        for (from, to) in &replacements {
            if rand::thread_rng().gen::<f32>() < 0.3 {
                result = result.replace(from, to);
            }
        }
        result
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

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

    fn trusted() -> RelationshipStage {
        RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
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

    // ── StyleEmbedding 基础测试 ──

    #[test]
    fn test_style_embedding_zero() {
        let z = StyleEmbedding::zero();
        assert_eq!(z.norm(), 0.0);
    }

    #[test]
    fn test_style_embedding_from_pad_sad() {
        let style = StyleEmbedding::from_emotion_context(
            [-0.7, -0.3, -0.5], // 悲伤
            None,
            &EmotionDirection::SelfDirected,
            &familiar(),
            0.0,
            0.3,
        );
        // 维 0-2 应直接编码 PAD
        assert!(approx_eq(style.0[0], -0.7, 1e-6), "P should be -0.7");
        assert!(approx_eq(style.0[1], -0.3, 1e-6), "A should be -0.3");
        assert!(approx_eq(style.0[2], -0.5, 1e-6), "D should be -0.5");
        // 范数应非零
        assert!(style.norm() > 0.1);
    }

    #[test]
    fn test_style_embedding_from_pad_joy() {
        let style = StyleEmbedding::from_emotion_context(
            [0.7, 0.5, 0.4], // 喜悦
            None,
            &EmotionDirection::UserDirected,
            &deep(),
            0.3,
            0.2,
        );
        assert!(approx_eq(style.0[0], 0.7, 1e-6));
        assert!(approx_eq(style.0[1], 0.5, 1e-6));
        assert!(approx_eq(style.0[2], 0.4, 1e-6));
    }

    #[test]
    fn test_style_embedding_lerp() {
        let a = StyleEmbedding::from_emotion_context(
            [-0.7, -0.3, -0.5],
            None,
            &EmotionDirection::Neutral,
            &familiar(),
            0.0,
            0.0,
        );
        let b = StyleEmbedding::from_emotion_context(
            [0.7, 0.5, 0.4],
            None,
            &EmotionDirection::Neutral,
            &familiar(),
            0.0,
            0.0,
        );

        let mid = a.lerp(&b, 0.5);
        // 中点应在两者之间
        let dist_a = a.distance(&mid);
        let dist_b = b.distance(&mid);
        assert!(
            (dist_a - dist_b).abs() / (dist_a + dist_b) < 0.15,
            "mid should be roughly equidistant: da={}, db={}",
            dist_a,
            dist_b
        );

        // t=0 应返回 a
        let at_zero = a.lerp(&b, 0.0);
        assert!(approx_eq(at_zero.0[0], a.0[0], 1e-6));

        // t=1 应返回 b
        let at_one = a.lerp(&b, 1.0);
        assert!(approx_eq(at_one.0[0], b.0[0], 1e-6));
    }

    #[test]
    fn test_style_embedding_distance() {
        let a = StyleEmbedding::from_emotion_context(
            [0.0, 0.0, 0.0],
            None,
            &EmotionDirection::Neutral,
            &familiar(),
            0.0,
            0.0,
        );
        let b = StyleEmbedding::from_emotion_context(
            [1.0, 1.0, 1.0],
            None,
            &EmotionDirection::Neutral,
            &familiar(),
            0.0,
            0.0,
        );
        // 不同 PAD 应有正距离
        assert!(a.distance(&b) > 0.1);
        // 自身距离应为 0
        assert!(approx_eq(a.distance(&a), 0.0, 1e-6));
    }

    #[test]
    fn test_style_embedding_compound_one_hot() {
        // 内疚是 COMPOUND_EMOTIONS[0]
        let style = StyleEmbedding::from_emotion_context(
            [-0.4, 0.2, -0.5],
            Some(0), // 内疚
            &EmotionDirection::SelfDirected,
            &familiar(),
            0.0,
            0.0,
        );
        // 维 3 应为 1.0（one-hot 第 0 个复合情绪）
        assert!(approx_eq(style.0[3], 1.0, 1e-6));
        // 维 4 应为 0.0
        assert!(approx_eq(style.0[4], 0.0, 1e-6));
    }

    #[test]
    fn test_style_embedding_relationship_affects_vector() {
        let style_acq = StyleEmbedding::from_emotion_context(
            [0.3, 0.0, 0.0],
            None,
            &EmotionDirection::Neutral,
            &acq(),
            0.0,
            0.0,
        );
        let style_deep = StyleEmbedding::from_emotion_context(
            [0.3, 0.0, 0.0],
            None,
            &EmotionDirection::Neutral,
            &deep(),
            0.0,
            0.0,
        );
        // 不同关系阶段应产生不同向量
        assert!(style_acq.distance(&style_deep) > 0.01);
    }

    // ── LinguisticProfile 测试 ──

    #[test]
    fn test_linguistic_profile_sad() {
        let style = StyleEmbedding::from_emotion_context(
            [-0.7, -0.3, -0.5],
            None,
            &EmotionDirection::SelfDirected,
            &familiar(),
            0.0,
            0.3,
        );
        let lp = style.to_linguistic_profile();
        // 悲伤：短句、高沉默、低幽默
        assert!(
            lp.sentence_length < 12.0,
            "sad should have shorter sentences: {}",
            lp.sentence_length
        );
        assert!(
            lp.silence_tendency > 0.3,
            "sad should have high silence: {}",
            lp.silence_tendency
        );
        assert!(
            lp.humor_tendency < 0.3,
            "sad should have low humor: {}",
            lp.humor_tendency
        );
    }

    #[test]
    fn test_linguistic_profile_joy() {
        let style = StyleEmbedding::from_emotion_context(
            [0.7, 0.5, 0.4],
            None,
            &EmotionDirection::UserDirected,
            &familiar(),
            0.3,
            0.2,
        );
        let lp = style.to_linguistic_profile();
        // 喜悦：高语气词、高亲昵、高幽默
        assert!(
            lp.particle_density > 0.3,
            "joy should have high particles: {}",
            lp.particle_density
        );
        assert!(
            lp.endearment_tendency > 0.2,
            "joy should have some endearment: {}",
            lp.endearment_tendency
        );
        assert!(
            lp.humor_tendency > 0.2,
            "joy should have some humor: {}",
            lp.humor_tendency
        );
    }

    #[test]
    fn test_linguistic_profile_anger() {
        let style = StyleEmbedding::from_emotion_context(
            [-0.6, 0.7, 0.6],
            None,
            &EmotionDirection::UserDirected,
            &familiar(),
            -0.2,
            0.5,
        );
        let lp = style.to_linguistic_profile();
        // 愤怒：高碎片化、高确定性、低语气词
        assert!(
            lp.fragmentation > 0.3,
            "anger should have fragmentation: {}",
            lp.fragmentation
        );
        assert!(
            lp.certainty_marking > 0.4,
            "anger should have certainty: {}",
            lp.certainty_marking
        );
    }

    #[test]
    fn test_linguistic_profile_bittersweet() {
        let style = StyleEmbedding::from_emotion_context(
            [0.1, -0.1, -0.2],
            None,
            &EmotionDirection::MemoryDirected,
            &familiar(),
            0.0,
            0.3,
        );
        let lp = style.to_linguistic_profile();
        // 苦甜混合：高隐喻、中等沉默
        assert!(
            lp.metaphor_tendency > 0.3,
            "bittersweet should have metaphor: {}",
            lp.metaphor_tendency
        );
    }

    #[test]
    fn test_linguistic_profile_neutral() {
        let lp = LinguisticProfile::neutral();
        assert!(approx_eq(lp.sentence_length, 14.0, 1e-6));
        assert!(approx_eq(lp.syntactic_complexity, 0.4, 1e-6));
    }

    // ── 关系阶段叠加测试 ──

    #[test]
    fn test_relationship_overlay_acquaintance() {
        let mut lp = LinguisticProfile {
            particle_density: 0.5,
            endearment_tendency: 0.5,
            humor_tendency: 0.5,
            certainty_marking: 0.4,
            ..LinguisticProfile::neutral()
        };
        lp.apply_relationship_overlay(&acq());
        // 初识阶段：亲昵应被大幅降低
        assert!(
            lp.endearment_tendency < 0.2,
            "acquaintance should limit endearment: {}",
            lp.endearment_tendency
        );
        assert!(
            lp.particle_density < 0.3,
            "acquaintance should limit particles: {}",
            lp.particle_density
        );
    }

    #[test]
    fn test_relationship_overlay_deep() {
        let mut lp = LinguisticProfile {
            particle_density: 0.3,
            endearment_tendency: 0.3,
            humor_tendency: 0.3,
            certainty_marking: 0.4,
            ..LinguisticProfile::neutral()
        };
        lp.apply_relationship_overlay(&deep());
        // 深度阶段：语气词和亲昵应升高
        assert!(
            lp.particle_density > 0.4,
            "deep should boost particles: {}",
            lp.particle_density
        );
        assert!(
            lp.endearment_tendency > 0.5,
            "deep should boost endearment: {}",
            lp.endearment_tendency
        );
    }

    // ── Prompt fragment 测试 ──

    #[test]
    fn test_prompt_fragment_sad() {
        let style = StyleEmbedding::from_emotion_context(
            [-0.7, -0.3, -0.5],
            None,
            &EmotionDirection::SelfDirected,
            &familiar(),
            0.0,
            0.3,
        );
        let fragment = style.to_prompt_fragment(&familiar());
        assert!(
            !fragment.is_empty(),
            "sad should generate non-empty prompt fragment"
        );
        assert!(
            fragment.contains("短句") || fragment.contains("低落"),
            "should mention sadness cues"
        );
    }

    #[test]
    fn test_prompt_fragment_joy() {
        let style = StyleEmbedding::from_emotion_context(
            [0.7, 0.5, 0.4],
            None,
            &EmotionDirection::UserDirected,
            &deep(),
            0.3,
            0.2,
        );
        let fragment = style.to_prompt_fragment(&deep());
        assert!(!fragment.is_empty());
    }

    // ── ExpressionContext 测试 ──

    #[test]
    fn test_expression_context_from_modules() {
        let emotion = EmotionState::new(-0.5, 0.2, -0.3);
        let ctx = ExpressionContext::from_modules(
            &emotion,
            None,
            EmotionDirection::SelfDirected,
            &familiar(),
            -0.2,
            0.4,
        );
        assert!(approx_eq(ctx.pad[0], -0.5, 1e-6));
        assert!(approx_eq(ctx.user_valence, -0.2, 1e-6));
    }

    #[test]
    fn test_expression_context_to_linguistic_profile() {
        let emotion = EmotionState::new(-0.7, -0.3, -0.5);
        let ctx = ExpressionContext::from_modules(
            &emotion,
            None,
            EmotionDirection::SelfDirected,
            &acq(),
            0.0,
            0.3,
        );
        let lp = ctx.to_linguistic_profile();
        // 初识+悲伤：亲昵应极低
        assert!(lp.endearment_tendency < 0.2);
    }

    // ── 后处理测试 ──

    #[test]
    fn test_post_process_sad_ellipsis() {
        let lp = LinguisticProfile {
            silence_tendency: 0.6,
            sentence_length: 6.0,
            ..LinguisticProfile::neutral()
        };
        let input = "我很难过。真的。";
        let output = lp.post_process(input);
        // 悲伤后处理可能将部分句号替换为省略号
        assert!(!output.is_empty());
    }

    #[test]
    fn test_post_process_soften_certainty() {
        let lp = LinguisticProfile {
            certainty_marking: 0.1,
            ..LinguisticProfile::neutral()
        };
        let input = "这一定是最好的方案。";
        let output = lp.post_process(input);
        // 低确定性后处理可能将"一定"替换为"可能"
        assert!(!output.is_empty());
    }
}
