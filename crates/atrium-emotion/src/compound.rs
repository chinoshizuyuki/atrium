// SPDX-License-Identifier: MIT
// CompoundEmotion — 复合情绪分析 / Compound emotion analysis

use serde::{Deserialize, Serialize};

use crate::{EmotionState, InertiaModifiers, LongingState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    pub current: EmotionState,
    pub inertia_history: Vec<[f32; 3]>,
    pub inertia_dominant_duration: usize,
    pub inertia_dominant_label: Option<String>,
    pub inertia_modifiers: InertiaModifiers,
    /// 想念引擎运行时状态 / Longing engine runtime state
    #[serde(default)]
    pub longing_state: Option<LongingState>,
}

// ════════════════════════════════════════════════════════════════════
// 高阶情绪模型 — 复合情绪层（20+ 种）
// ════════════════════════════════════════════════════════════════════

/// 情绪方向性：标记情绪的指向对象
///
/// 人类情绪不仅由 PAD 值决定，还取决于"对谁/对什么"产生的：
/// - Self-directed: 自豪/羞耻/内疚（指向自身）
/// - User-directed: 感激/心疼/嫉妒（指向对话对象）
/// - Memory-directed: 怀旧/释然（指向过去的记忆）
/// - Neutral: 无特定方向（敬畏/孤独等）
#[derive(Clone, Debug, PartialEq)]
pub enum EmotionDirection {
    SelfDirected,
    UserDirected,
    MemoryDirected,
    Neutral,
}

/// 复合情绪标签
///
/// 在 9 种基本情绪之上，叠加 22 种高阶复合情绪。
/// 每种情绪由 PAD 区域 + 方向性约束共同决定。
#[derive(Clone, Debug)]
pub struct CompoundEmotion {
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    /// PAD 区域的中心点
    pub pad_center: (f32, f32, f32),
    /// PAD 区域的半径（容差）
    pub pad_radius: f32,
    /// 必须匹配的方向性（None 表示任意方向均可）
    pub direction: Option<EmotionDirection>,
}

/// 22 种复合情绪定义
///
/// PAD 中心点基于 Plutchik 情绪轮 + 社会情绪心理学研究。
/// 半径用于控制判定的宽松程度（越小越严格）。
pub const COMPOUND_EMOTIONS: [CompoundEmotion; 22] = [
    // ── 自我指向 ──
    CompoundEmotion {
        name: "内疚",
        emoji: "😔",
        description: "对自己的行为感到后悔和不安",
        pad_center: (-0.4, 0.2, -0.5),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "自豪",
        emoji: "😤",
        description: "对自己的成就感到满足和骄傲",
        pad_center: (0.6, 0.4, 0.7),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "羞耻",
        emoji: "😳",
        description: "因自身不足或错误而感到难堪",
        pad_center: (-0.5, 0.3, -0.6),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    CompoundEmotion {
        name: "自信",
        emoji: "💪",
        description: "对自身能力的坚定信心",
        pad_center: (0.4, 0.3, 0.8),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::SelfDirected),
    },
    // ── 对方指向 ──
    CompoundEmotion {
        name: "感激",
        emoji: "🙏",
        description: "对他人善意和帮助的由衷感谢",
        pad_center: (0.5, 0.2, -0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "心疼",
        emoji: "💔",
        description: "看到对方受苦时产生的怜惜和关切",
        pad_center: (-0.3, 0.1, -0.1),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "嫉妒",
        emoji: "😒",
        description: "因他人的优势或拥有而感到不平衡",
        pad_center: (-0.4, 0.5, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "钦佩",
        emoji: "🤝",
        description: "对他人能力或品格的尊重和赞赏",
        pad_center: (0.4, 0.3, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    CompoundEmotion {
        name: "怜爱",
        emoji: "🥰",
        description: "对对方的温柔喜爱和保护欲",
        pad_center: (0.6, 0.1, 0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::UserDirected),
    },
    // ── 记忆指向 ──
    CompoundEmotion {
        name: "怀旧",
        emoji: "🌅",
        description: "回忆过去时混合的温暖与淡淡忧伤",
        pad_center: (0.1, -0.1, -0.2),
        pad_radius: 0.40,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "释然",
        emoji: "🍃",
        description: "放下过去的执念后的轻松与平和",
        pad_center: (0.3, -0.3, 0.1),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "遗憾",
        emoji: "😞",
        description: "对未能实现之事的不甘与惋惜",
        pad_center: (-0.4, -0.1, -0.3),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    CompoundEmotion {
        name: "眷恋",
        emoji: "💭",
        description: "对过去美好时光的深深留恋",
        pad_center: (0.2, 0.0, -0.2),
        pad_radius: 0.35,
        direction: Some(EmotionDirection::MemoryDirected),
    },
    // ── 混合情绪（正负 valence 共存）──
    CompoundEmotion {
        name: "百感交集",
        emoji: "🎭",
        description: "同时感受到快乐与忧伤的复杂心境",
        pad_center: (0.0, 0.1, -0.1),
        pad_radius: 0.25,
        direction: None,
    },
    // ── 无方向（状态性情绪）──
    CompoundEmotion {
        name: "敬畏",
        emoji: "🌌",
        description: "面对宏大或超越性事物时的震撼与谦卑",
        pad_center: (0.2, 0.6, -0.5),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "孤独",
        emoji: "🌙",
        description: "缺少陪伴或连接感时的空虚与渴望",
        pad_center: (-0.5, -0.3, -0.4),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "安心",
        emoji: "🏠",
        description: "感受到安全和归属后的踏实与温暖",
        pad_center: (0.4, -0.3, 0.3),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "焦虑",
        emoji: "😰",
        description: "对未来不确定性的持续担忧和紧张",
        pad_center: (-0.4, 0.5, -0.5),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "温柔",
        emoji: "🌸",
        description: "柔和的善意与细腻的情感流动",
        pad_center: (0.5, -0.1, 0.1),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "好奇",
        emoji: "🔍",
        description: "对新事物或未知领域的探索欲望",
        pad_center: (0.3, 0.5, 0.2),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "无奈",
        emoji: "😅",
        description: "面对无法改变之事时的苦笑着接受",
        pad_center: (-0.2, 0.0, -0.4),
        pad_radius: 0.35,
        direction: None,
    },
    CompoundEmotion {
        name: "陶醉",
        emoji: "✨",
        description: "沉浸在美好体验中的高度愉悦",
        pad_center: (0.7, 0.3, 0.1),
        pad_radius: 0.35,
        direction: None,
    },
];

/// 复合情绪分类上下文
///
/// 由 `process_message` 管线构建，提供方向性提示和混合情绪线索。
#[derive(Clone, Debug)]
pub struct CompoundContext {
    /// 情绪方向（由消息内容推断）
    pub direction: EmotionDirection,
    /// 用户消息是否包含回忆/过去相关的关键词
    pub has_memory_cue: bool,
    /// 当前基本情绪标签名称
    pub basic_label: &'static str,
}

impl Default for CompoundContext {
    fn default() -> Self {
        Self {
            direction: EmotionDirection::Neutral,
            has_memory_cue: false,
            basic_label: "平静",
        }
    }
}

/// PAD + 上下文 → 复合情绪分类
///
/// 判断逻辑：
/// 1. 遍历所有 22 种复合情绪
/// 2. 计算 PAD 欧氏距离，过滤掉方向不匹配的
/// 3. 选择距离最近且在半径内的复合情绪
/// 4. 若无匹配 → 返回 `None`（回退到基本情绪标签）
pub fn classify_compound(
    state: &EmotionState,
    ctx: &CompoundContext,
) -> Option<&'static CompoundEmotion> {
    let mut best: Option<(f32, &'static CompoundEmotion)> = None;

    for ce in &COMPOUND_EMOTIONS {
        // 方向性过滤：有约束时必须匹配
        if let Some(ref required_dir) = ce.direction {
            if *required_dir != ctx.direction {
                continue;
            }
        }

        // 记忆指向情绪：需要记忆线索或方向为 MemoryDirected
        if matches!(ce.direction, Some(EmotionDirection::MemoryDirected))
            && !ctx.has_memory_cue
            && ctx.direction != EmotionDirection::MemoryDirected
        {
            continue;
        }

        // PAD 欧氏距离
        let dp = state.pleasure - ce.pad_center.0;
        let da = state.arousal - ce.pad_center.1;
        let dd = state.dominance - ce.pad_center.2;
        let dist = (dp * dp + da * da + dd * dd).sqrt();

        if dist > ce.pad_radius {
            continue;
        }

        if best.is_none_or(|(best_dist, _)| dist < best_dist) {
            best = Some((dist, ce));
        }
    }

    best.map(|(_, ce)| ce)
}

/// PAD → 自然语言情绪描述
///
/// 替代原始的 `(愉悦:0.45, 唤醒:0.12, 支配:0.08)` 浮点数格式。
/// 优先使用复合情绪（如果匹配到），否则使用基本情绪标签。
pub fn to_natural_language(state: &EmotionState, ctx: &CompoundContext) -> String {
    // 先尝试复合情绪
    if let Some(compound) = classify_compound(state, ctx) {
        return format!("{} {}", compound.emoji, compound.name);
    }

    // 回退到基本情绪
    let basic = state.classify();
    format!("{} {}", basic.emoji, basic.name)
}

/// 从消息文本推断情绪方向性
///
/// 基于关键词检测的轻量启发式方法：
/// - "我"/"自己"/"自己的" → SelfDirected
/// - "你"/"谢"/"感谢"/"对不起" → UserDirected
/// - "以前"/"小时候"/"记得"/"回忆"/"当年" → MemoryDirected
pub fn infer_direction(message: &str) -> EmotionDirection {
    let msg_lower = message.to_lowercase();

    // 记忆指向（优先检查，因为怀旧相关词较独特）
    let memory_keywords = [
        "以前",
        "小时候",
        "记得",
        "回忆",
        "当年",
        "那年",
        "过去",
        "曾经",
        "那时",
        "往事",
        "怀念",
        "想念",
        "remember",
        "nostalgia",
        "back then",
        "used to",
    ];
    for kw in &memory_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::MemoryDirected;
        }
    }

    // 对方指向
    let user_keywords = [
        "谢谢你",
        "感谢你",
        "多亏你",
        "对不起",
        "抱歉",
        "你真",
        "你太",
        "感谢你",
        "辛苦你",
        "谢谢",
        "thank",
        "sorry",
        "grateful",
    ];
    for kw in &user_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::UserDirected;
        }
    }

    // 自我指向
    let self_keywords = [
        "我觉得自己",
        "我做到了",
        "我成功",
        "我失败",
        "我太差",
        "我骄傲",
        "我自豪",
        "我惭愧",
        "我后悔",
        "i did",
        "i achieved",
        "i failed",
        "i'm proud",
    ];
    for kw in &self_keywords {
        if msg_lower.contains(kw) {
            return EmotionDirection::SelfDirected;
        }
    }

    EmotionDirection::Neutral
}

/// 检测混合情绪（正负 valence 同时存在）
///
/// 当 pleasure 接近 0 但 arousal 非零时，可能存在混合情绪。
/// 返回 `Some("百感交集")` 如果检测到混合状态。
pub fn detect_mixed_emotion(state: &EmotionState) -> Option<&'static CompoundEmotion> {
    // 混合情绪特征：pleasure 接近中性（-0.15 ~ 0.15）+ 有一定唤醒度
    if state.pleasure.abs() < 0.15 && state.arousal.abs() > 0.15 {
        // 查找百感交集
        return COMPOUND_EMOTIONS.iter().find(|ce| ce.name == "百感交集");
    }
    None
}
