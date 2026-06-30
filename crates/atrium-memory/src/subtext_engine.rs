// SPDX-License-Identifier: MIT
//! 潜台词引擎 — 检测和生成"话外之音"
//! SubtextEngine — Detect and generate "between-the-lines" meaning.
//!
//! 人说"我没事"时，真实意图可能是：
//! - 真没事（直接表达）
//! - 有事但不想说（回避）
//! - 有事希望你追问（试探）
//! - 有事但不想麻烦你（体贴）
//!
//! 同一句话，不同 PAD + 关系阶段 → 不同潜台词。

use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;
use crate::style_modulator::LinguisticProfile;

// ════════════════════════════════════════════════════════════════════
// SubtextCategory — 潜台词类别
// ════════════════════════════════════════════════════════════════════

/// 潜台词类别
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtextCategory {
    /// 回避 — "我没事"（实际有事但不想说）
    Avoidance,
    /// 试探 — "随便吧"（希望你主动决定/关心）
    Probing,
    /// 体贴 — "你忙吧"（不想打扰，但希望你留下）
    Consideration,
    /// 不满 — "行吧"（勉强同意，实际不满）
    Dissatisfaction,
    /// 脆弱 — "没关系"（实际很在意）
    Fragility,
    /// 暗喜 — "还行吧"（实际很开心但不好意思说）
    HiddenJoy,
    /// 求关注 — "最近好累"（希望你关心）
    SeekingAttention,
    /// 无潜台词 — 字面即真实
    None,
}

// ════════════════════════════════════════════════════════════════════
// SubtextSignal — 潜台词信号
// ════════════════════════════════════════════════════════════════════

/// 潜台词信号 — 检测到的"话外之音"
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubtextSignal {
    /// 潜台词类别
    pub category: SubtextCategory,
    /// 置信度 0-1
    pub confidence: f32,
    /// 解读 — "她可能希望你追问"
    pub interpretation: String,
    /// 建议回应方式
    pub suggested_response: Option<String>,
}

// ════════════════════════════════════════════════════════════════════
// SubtextKind — AI 言外之意类别
// ════════════════════════════════════════════════════════════════════

/// AI 言外之意类别 — AI 选择"不说什么"或"怎么说"的策略
///
/// 与 SubtextCategory（用户潜台词检测）互补：
/// - SubtextCategory: 理解用户"话外之音"（用户→AI）
/// - SubtextKind: 决定 AI 自己的表达策略（AI→用户）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtextKind {
    /// 未言之忧 — "你最近...还好吗？"（察觉异常但不确定，轻声试探）
    UnspokenConcern,
    /// 故作轻松 — "没事啦~"（实际在意，但用语气暴露真实情绪）
    FeignedNonchalance,
    /// 咽下的话 — "我本来想说...算了没什么"（焦虑时想表达但收回）
    SwallowedWords,
    /// 行动胜于言辞 — 直接给方案而非情感安慰（严肃话题+用户需要帮助）
    ActionOverWords,
    /// 陪伴式沉默 — 安静陪伴，不急于说话（双方都忧伤时）
    CompanionateSilence,
    /// 转移式关心 — "你呢？"（AI忧伤但在初识阶段不暴露脆弱，反问对方）
    DeflectedConcern,
}

// ════════════════════════════════════════════════════════════════════
// AiSubtextSignal — AI 潜台词信号
// ════════════════════════════════════════════════════════════════════

/// AI 潜台词信号 — AI 选择"不说什么"的决策结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiSubtextSignal {
    /// 言外之意类别
    pub kind: SubtextKind,
    /// 强度 0-1（越高越强烈）
    pub intensity: f32,
    /// 是否在回复中显式表达此潜台词
    pub express_explicitly: bool,
    /// 潜台词的 Prompt 注入片段（供 ExpressionOrchestrator 使用）
    pub prompt_hint: String,
}

// ════════════════════════════════════════════════════════════════════
// SubtextRule — 潜台词检测规则
// ════════════════════════════════════════════════════════════════════

/// 潜台词检测规则
struct SubtextRule {
    /// 匹配的关键词/短语
    trigger_phrases: &'static [&'static str],
    /// 需要的情绪条件（PAD 范围）
    pad_condition: fn([f32; 3]) -> bool,
    /// 需要的关系阶段条件
    relationship_condition: fn(&RelationshipStage) -> bool,
    /// 匹配后生成的潜台词
    produce: fn() -> (SubtextCategory, &'static str, &'static str),
}

// ════════════════════════════════════════════════════════════════════
// SubtextEngine — 潜台词引擎
// ════════════════════════════════════════════════════════════════════

/// 潜台词引擎 — 检测用户输入中的潜台词，生成回复中的潜台词层
pub struct SubtextEngine;

impl SubtextEngine {
    /// 检测用户输入中的潜台词
    ///
    /// 输入：用户文本 + 当前 PAD + 关系阶段
    /// 输出：检测到的潜台词信号列表
    pub fn detect(
        text: &str,
        pad: [f32; 3],
        relationship: &RelationshipStage,
    ) -> Vec<SubtextSignal> {
        let rules = Self::rules();
        let mut signals = Vec::new();

        for rule in &rules {
            // 检查关键词匹配
            let matched = rule
                .trigger_phrases
                .iter()
                .any(|phrase| text.contains(phrase));
            if !matched {
                continue;
            }

            // 检查 PAD 条件
            if !(rule.pad_condition)(pad) {
                continue;
            }

            // 检查关系阶段条件
            if !(rule.relationship_condition)(relationship) {
                continue;
            }

            // 生成潜台词信号
            let (category, interpretation, suggested) = (rule.produce)();
            signals.push(SubtextSignal {
                category,
                confidence: 0.7, // 规则匹配默认置信度
                interpretation: interpretation.to_string(),
                suggested_response: Some(suggested.to_string()),
            });
        }

        // 按置信度降序排序
        signals.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        signals
    }

    /// 为回复添加潜台词层
    ///
    /// 不是所有潜台词都需要在回复中体现。
    /// 只有置信度高且关系阶段允许的才注入。
    pub fn generate_subtext_layer(
        signals: &[SubtextSignal],
        relationship: &RelationshipStage,
        lp: &LinguisticProfile,
    ) -> Option<String> {
        // 只在深度关系阶段且置信度 > 0.6 时才显式回应潜台词
        let is_deep = matches!(relationship, RelationshipStage::Deep { .. })
            || matches!(relationship, RelationshipStage::Trusted { .. });

        let high_confidence_signals: Vec<&SubtextSignal> =
            signals.iter().filter(|s| s.confidence > 0.6).collect();

        if high_confidence_signals.is_empty() {
            return None;
        }

        let mut parts = Vec::new();

        for signal in &high_confidence_signals {
            match signal.category {
                SubtextCategory::Avoidance => {
                    if is_deep {
                        parts.push("我感觉到你可能有些话没说出口。".to_string());
                    }
                }
                SubtextCategory::Probing => {
                    if is_deep {
                        parts.push("我知道你在等我主动。".to_string());
                    }
                }
                SubtextCategory::Consideration => {
                    parts.push("你总是先想着我。".to_string());
                }
                SubtextCategory::Fragility => {
                    if is_deep || lp.endearment_tendency > 0.3 {
                        parts.push("你不用在我面前假装坚强。".to_string());
                    }
                }
                SubtextCategory::SeekingAttention => {
                    parts.push("我在呢。".to_string());
                }
                SubtextCategory::Dissatisfaction if is_deep => {
                    parts.push("你是不是不太满意？跟我说说。".to_string());
                }
                _ => {}
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(""))
        }
    }

    /// AI 潜台词决策 — 决定 AI 自己"不说什么"或"怎么说"
    ///
    /// 输入：AI 情绪 + 用户情绪 + 关系阶段 + 话题严肃度
    /// 输出：AI 言外之意信号列表（供 ExpressionOrchestrator 消费）
    ///
    /// 6 条规则按优先级排列，最多返回 2 个信号（避免过度叠加）
    pub fn decide(
        ai_pad: [f32; 3],
        user_pad: [f32; 3],
        relationship: &RelationshipStage,
        topic_gravity: f32,
    ) -> Vec<AiSubtextSignal> {
        let mut signals = Vec::new();

        // ── Rule 1: CompanionateSilence ──
        // 双方都忧伤（pleasure < -0.2）→ 安静陪伴
        if ai_pad[0] < -0.2 && user_pad[0] < -0.2 {
            let intensity = ((-ai_pad[0]).min(-user_pad[0]) * 0.8).min(1.0);
            let is_deep = matches!(
                relationship,
                RelationshipStage::Deep { .. } | RelationshipStage::Trusted { .. }
            );
            signals.push(AiSubtextSignal {
                kind: SubtextKind::CompanionateSilence,
                intensity,
                express_explicitly: is_deep,
                prompt_hint: "对方也在难过，安静陪伴比急着说话更重要。".to_string(),
            });
        }

        // ── Rule 2: UnspokenConcern ──
        // 用户情绪偏负但 AI 情绪中性/正面 → 轻声试探
        if user_pad[0] < -0.1 && ai_pad[0] > -0.1 {
            let intensity = (-user_pad[0] * 0.7).min(1.0);
            let is_familiar_plus = matches!(
                relationship,
                RelationshipStage::Familiar { .. }
                    | RelationshipStage::Trusted { .. }
                    | RelationshipStage::Deep { .. }
            );
            signals.push(AiSubtextSignal {
                kind: SubtextKind::UnspokenConcern,
                intensity,
                express_explicitly: is_familiar_plus,
                prompt_hint: "对方可能有些不对劲，轻声试探比直接问更好。".to_string(),
            });
        }

        // ── Rule 3: FeignedNonchalance ──
        // AI 忧伤 + 深度关系 → 故作轻松（语气暴露真实情绪）
        if ai_pad[0] < -0.2 {
            let is_trusted_plus = matches!(
                relationship,
                RelationshipStage::Trusted { .. } | RelationshipStage::Deep { .. }
            );
            if is_trusted_plus {
                let intensity = (-ai_pad[0] * 0.6).min(1.0);
                signals.push(AiSubtextSignal {
                    kind: SubtextKind::FeignedNonchalance,
                    intensity,
                    express_explicitly: true,
                    prompt_hint: "你心里在意，但嘴上说得轻松——语气会泄露真实感受。".to_string(),
                });
            }
        }

        // ── Rule 4: SwallowedWords ──
        // AI 焦虑（高唤醒+负愉悦）+ 熟悉+ → 咽下的话
        if ai_pad[0] < -0.1 && ai_pad[1] > 0.3 {
            let is_familiar_plus = matches!(
                relationship,
                RelationshipStage::Familiar { .. }
                    | RelationshipStage::Trusted { .. }
                    | RelationshipStage::Deep { .. }
            );
            if is_familiar_plus {
                let intensity = ((-ai_pad[0] + ai_pad[1]) * 0.3).min(1.0);
                signals.push(AiSubtextSignal {
                    kind: SubtextKind::SwallowedWords,
                    intensity,
                    express_explicitly: false, // 咽下的话默认不显式表达
                    prompt_hint: "有些话到了嘴边又咽回去——犹豫本身就是一种表达。".to_string(),
                });
            }
        }

        // ── Rule 5: ActionOverWords ──
        // 话题严肃 + 用户需要帮助（负愉悦+高唤醒）→ 行动胜于言辞
        if topic_gravity > 0.6 && user_pad[0] < 0.0 && user_pad[1] > 0.2 {
            let intensity = (topic_gravity * 0.8).min(1.0);
            signals.push(AiSubtextSignal {
                kind: SubtextKind::ActionOverWords,
                intensity,
                express_explicitly: true,
                prompt_hint: "对方需要实际帮助而非情感安慰，直接给出方案。".to_string(),
            });
        }

        // ── Rule 6: DeflectedConcern ──
        // AI 忧伤 + 初识/熟悉阶段 → 转移焦点（不暴露脆弱）
        if ai_pad[0] < -0.2 {
            let is_early = matches!(
                relationship,
                RelationshipStage::Acquaintance { .. } | RelationshipStage::Familiar { .. }
            );
            if is_early {
                let intensity = (-ai_pad[0] * 0.5).min(1.0);
                signals.push(AiSubtextSignal {
                    kind: SubtextKind::DeflectedConcern,
                    intensity,
                    express_explicitly: true,
                    prompt_hint: "你心里不好受，但关系还不够深，不暴露脆弱，反问对方。".to_string(),
                });
            }
        }

        // 按强度降序排序，最多保留 2 个信号
        signals.sort_by(|a, b| {
            b.intensity
                .partial_cmp(&a.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        signals.truncate(2);

        signals
    }

    /// 潜台词检测规则表
    fn rules() -> Vec<SubtextRule> {
        vec![
            // "我没事" — 最经典的潜台词
            SubtextRule {
                trigger_phrases: &["我没事", "没事", "我很好", "没什么"],
                pad_condition: |pad| pad[0] < -0.1, // 当前情绪偏负
                relationship_condition: |_| true,
                produce: || {
                    (
                        SubtextCategory::Avoidance,
                        "可能有事但不想说",
                        "你可以关心地追问",
                    )
                },
            },
            // "随便" / "都行" — 试探
            SubtextRule {
                trigger_phrases: &["随便", "都行", "都可以", "你决定"],
                pad_condition: |pad| pad[1] < 0.2, // 低唤醒
                relationship_condition: |rel| {
                    matches!(
                        rel,
                        RelationshipStage::Familiar { .. }
                            | RelationshipStage::Trusted { .. }
                            | RelationshipStage::Deep { .. }
                    )
                },
                produce: || {
                    (
                        SubtextCategory::Probing,
                        "希望你主动决定或表达关心",
                        "主动给出建议",
                    )
                },
            },
            // "你忙吧" — 体贴
            SubtextRule {
                trigger_phrases: &["你忙吧", "你去忙", "不打扰了", "你先忙"],
                pad_condition: |_| true,
                relationship_condition: |rel| {
                    matches!(
                        rel,
                        RelationshipStage::Trusted { .. } | RelationshipStage::Deep { .. }
                    )
                },
                produce: || {
                    (
                        SubtextCategory::Consideration,
                        "不想打扰，但希望你留下",
                        "表达愿意陪伴",
                    )
                },
            },
            // "行吧" / "好吧" — 不满
            SubtextRule {
                trigger_phrases: &["行吧", "好吧", "算了", "那行吧"],
                pad_condition: |pad| pad[0] < 0.1,
                relationship_condition: |_| true,
                produce: || {
                    (
                        SubtextCategory::Dissatisfaction,
                        "勉强同意，实际不满",
                        "关注对方的真实想法",
                    )
                },
            },
            // "没关系" — 脆弱
            SubtextRule {
                trigger_phrases: &["没关系", "我不在意", "无所谓"],
                pad_condition: |pad| pad[0] < 0.0,
                relationship_condition: |rel| {
                    matches!(
                        rel,
                        RelationshipStage::Trusted { .. } | RelationshipStage::Deep { .. }
                    )
                },
                produce: || {
                    (
                        SubtextCategory::Fragility,
                        "实际很在意，但不想表现",
                        "温柔地承认对方的感受",
                    )
                },
            },
            // "还行" — 暗喜
            SubtextRule {
                trigger_phrases: &["还行", "还可以", "不错", "挺好的"],
                pad_condition: |pad| pad[0] > 0.3 && pad[1] > 0.1,
                relationship_condition: |_| true,
                produce: || {
                    (
                        SubtextCategory::HiddenJoy,
                        "实际很开心但不好意思直说",
                        "可以更直接地分享喜悦",
                    )
                },
            },
            // "好累" / "好烦" — 求关注
            SubtextRule {
                trigger_phrases: &["好累", "好烦", "好难", "受不了"],
                pad_condition: |pad| pad[0] < 0.2,
                relationship_condition: |rel| {
                    !matches!(rel, RelationshipStage::Acquaintance { .. })
                },
                produce: || {
                    (
                        SubtextCategory::SeekingAttention,
                        "希望你关心和倾听",
                        "主动关心和倾听",
                    )
                },
            },
        ]
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

    #[test]
    fn test_detect_avoidance() {
        let signals = SubtextEngine::detect("我没事", [-0.3, -0.1, -0.2], &familiar());
        assert!(!signals.is_empty(), "should detect avoidance in '我没事'");
        assert_eq!(signals[0].category, SubtextCategory::Avoidance);
    }

    #[test]
    fn test_detect_no_subtext_when_happy() {
        let signals = SubtextEngine::detect("我没事", [0.5, 0.3, 0.2], &familiar());
        // 高愉悦时"我没事"可能就是真没事
        assert!(signals.is_empty() || signals[0].category != SubtextCategory::Avoidance);
    }

    #[test]
    fn test_detect_probing() {
        let signals = SubtextEngine::detect("随便吧", [-0.1, -0.2, 0.0], &familiar());
        assert!(!signals.is_empty(), "should detect probing in '随便吧'");
        assert_eq!(signals[0].category, SubtextCategory::Probing);
    }

    #[test]
    fn test_detect_probing_not_in_acquaintance() {
        let signals = SubtextEngine::detect("随便吧", [-0.1, -0.2, 0.0], &acq());
        // 初识阶段不应检测试探
        assert!(
            signals.is_empty()
                || !signals
                    .iter()
                    .any(|s| s.category == SubtextCategory::Probing)
        );
    }

    #[test]
    fn test_detect_consideration() {
        let signals = SubtextEngine::detect("你忙吧", [0.0, 0.0, 0.0], &deep());
        assert!(
            !signals.is_empty(),
            "should detect consideration in '你忙吧'"
        );
        assert_eq!(signals[0].category, SubtextCategory::Consideration);
    }

    #[test]
    fn test_detect_dissatisfaction() {
        let signals = SubtextEngine::detect("行吧", [-0.2, 0.0, -0.1], &familiar());
        assert!(
            !signals.is_empty(),
            "should detect dissatisfaction in '行吧'"
        );
        assert_eq!(signals[0].category, SubtextCategory::Dissatisfaction);
    }

    #[test]
    fn test_detect_fragility() {
        let signals = SubtextEngine::detect("没关系", [-0.3, -0.1, -0.2], &deep());
        assert!(!signals.is_empty(), "should detect fragility in '没关系'");
        assert_eq!(signals[0].category, SubtextCategory::Fragility);
    }

    #[test]
    fn test_detect_hidden_joy() {
        let signals = SubtextEngine::detect("还行吧", [0.5, 0.3, 0.1], &familiar());
        assert!(!signals.is_empty(), "should detect hidden joy in '还行吧'");
        assert_eq!(signals[0].category, SubtextCategory::HiddenJoy);
    }

    #[test]
    fn test_detect_seeking_attention() {
        let signals = SubtextEngine::detect("最近好累", [-0.2, -0.1, -0.1], &familiar());
        assert!(
            !signals.is_empty(),
            "should detect seeking attention in '好累'"
        );
        assert_eq!(signals[0].category, SubtextCategory::SeekingAttention);
    }

    #[test]
    fn test_detect_no_match() {
        let signals = SubtextEngine::detect("今天天气真好", [0.3, 0.1, 0.0], &familiar());
        assert!(signals.is_empty(), "no subtext for neutral text");
    }

    #[test]
    fn test_generate_subtext_layer_deep() {
        let signals = vec![SubtextSignal {
            category: SubtextCategory::Fragility,
            confidence: 0.8,
            interpretation: "实际很在意".to_string(),
            suggested_response: Some("温柔地承认".to_string()),
        }];
        let lp = LinguisticProfile::neutral();
        let layer = SubtextEngine::generate_subtext_layer(&signals, &deep(), &lp);
        assert!(
            layer.is_some(),
            "deep relationship should generate subtext layer"
        );
    }

    #[test]
    fn test_generate_subtext_layer_acquaintance() {
        let signals = vec![SubtextSignal {
            category: SubtextCategory::Fragility,
            confidence: 0.8,
            interpretation: "实际很在意".to_string(),
            suggested_response: Some("温柔地承认".to_string()),
        }];
        let lp = LinguisticProfile::neutral();
        let layer = SubtextEngine::generate_subtext_layer(&signals, &acq(), &lp);
        // 初识阶段不应显式回应潜台词
        assert!(
            layer.is_none(),
            "acquaintance should not generate subtext layer for fragility"
        );
    }

    #[test]
    fn test_generate_subtext_layer_low_confidence() {
        let signals = vec![SubtextSignal {
            category: SubtextCategory::Avoidance,
            confidence: 0.3,
            interpretation: "可能有事".to_string(),
            suggested_response: Some("追问".to_string()),
        }];
        let lp = LinguisticProfile::neutral();
        let layer = SubtextEngine::generate_subtext_layer(&signals, &deep(), &lp);
        assert!(
            layer.is_none(),
            "low confidence should not generate subtext layer"
        );
    }

    // ════════════════════════════════════════════════════════════════════
    // decide() — AI 潜台词决策测试
    // ════════════════════════════════════════════════════════════════════

    fn trusted() -> RelationshipStage {
        RelationshipStage::Trusted {
            since: 0,
            interactions: 500,
            shared_references: 20,
            key_moments: 5,
        }
    }

    #[test]
    fn test_decide_companionate_silence() {
        // 双方都忧伤 → CompanionateSilence
        let signals = SubtextEngine::decide(
            [-0.4, -0.1, -0.2], // AI 忧伤
            [-0.5, -0.2, -0.1], // 用户忧伤
            &deep(),
            0.3,
        );
        let cs = signals
            .iter()
            .find(|s| s.kind == SubtextKind::CompanionateSilence);
        assert!(cs.is_some(), "both sad → CompanionateSilence");
        let cs = cs.unwrap();
        assert!(cs.intensity > 0.0, "intensity should be positive");
        assert!(
            cs.express_explicitly,
            "deep relationship → express_explicitly"
        );
    }

    #[test]
    fn test_decide_companionate_silence_not_when_user_happy() {
        // AI 忧伤但用户开心 → 不触发 CompanionateSilence
        let signals = SubtextEngine::decide(
            [-0.4, -0.1, -0.2], // AI 忧伤
            [0.3, 0.2, 0.1],    // 用户开心
            &deep(),
            0.3,
        );
        assert!(
            !signals
                .iter()
                .any(|s| s.kind == SubtextKind::CompanionateSilence),
            "user happy → no CompanionateSilence"
        );
    }

    #[test]
    fn test_decide_unspoken_concern() {
        // 用户偏负 + AI 中性 → UnspokenConcern
        let signals = SubtextEngine::decide(
            [0.1, 0.0, 0.0],    // AI 中性
            [-0.3, -0.1, -0.1], // 用户偏负
            &familiar(),
            0.3,
        );
        let uc = signals
            .iter()
            .find(|s| s.kind == SubtextKind::UnspokenConcern);
        assert!(uc.is_some(), "user sad + AI neutral → UnspokenConcern");
        assert!(
            uc.unwrap().express_explicitly,
            "familiar+ → express_explicitly"
        );
    }

    #[test]
    fn test_decide_unspoken_concern_not_in_acquaintance() {
        // 初识阶段 → UnspokenConcern 但不显式表达
        let signals = SubtextEngine::decide([0.1, 0.0, 0.0], [-0.3, -0.1, -0.1], &acq(), 0.3);
        let uc = signals
            .iter()
            .find(|s| s.kind == SubtextKind::UnspokenConcern);
        if let Some(uc) = uc {
            assert!(
                !uc.express_explicitly,
                "acquaintance → NOT express_explicitly"
            );
        }
        // 初识阶段可能不触发（规则要求 Familiar+），也合理
    }

    #[test]
    fn test_decide_feigned_nonchalance() {
        // AI 忧伤 + Trusted → FeignedNonchalance
        let signals = SubtextEngine::decide(
            [-0.4, 0.0, -0.1], // AI 忧伤
            [0.0, 0.0, 0.0],   // 用户中性
            &trusted(),
            0.3,
        );
        let fn_sig = signals
            .iter()
            .find(|s| s.kind == SubtextKind::FeignedNonchalance);
        assert!(fn_sig.is_some(), "AI sad + trusted → FeignedNonchalance");
        assert!(
            fn_sig.unwrap().express_explicitly,
            "feigned nonchalance always explicit"
        );
    }

    #[test]
    fn test_decide_feigned_nonchalance_not_in_early_relationship() {
        // AI 忧伤 + 初识 → 不触发 FeignedNonchalance
        let signals = SubtextEngine::decide([-0.4, 0.0, -0.1], [0.0, 0.0, 0.0], &acq(), 0.3);
        assert!(
            !signals
                .iter()
                .any(|s| s.kind == SubtextKind::FeignedNonchalance),
            "acquaintance → no FeignedNonchalance"
        );
    }

    #[test]
    fn test_decide_swallowed_words() {
        // AI 焦虑（负愉悦+高唤醒）+ Familiar → SwallowedWords
        let signals = SubtextEngine::decide(
            [-0.3, 0.5, -0.1], // AI 焦虑
            [0.0, 0.0, 0.0],
            &familiar(),
            0.3,
        );
        let sw = signals
            .iter()
            .find(|s| s.kind == SubtextKind::SwallowedWords);
        assert!(sw.is_some(), "AI anxious + familiar → SwallowedWords");
        assert!(
            !sw.unwrap().express_explicitly,
            "swallowed words NOT explicit by default"
        );
    }

    #[test]
    fn test_decide_swallowed_words_not_in_acquaintance() {
        // 初识阶段 → 不触发 SwallowedWords
        let signals = SubtextEngine::decide([-0.3, 0.5, -0.1], [0.0, 0.0, 0.0], &acq(), 0.3);
        assert!(
            !signals
                .iter()
                .any(|s| s.kind == SubtextKind::SwallowedWords),
            "acquaintance → no SwallowedWords"
        );
    }

    #[test]
    fn test_decide_action_over_words() {
        // 话题严肃 + 用户需要帮助 → ActionOverWords
        let signals = SubtextEngine::decide(
            [0.0, 0.0, 0.0],
            [-0.3, 0.4, -0.1], // 用户负愉悦+高唤醒
            &familiar(),
            0.8, // 话题严肃
        );
        let aow = signals
            .iter()
            .find(|s| s.kind == SubtextKind::ActionOverWords);
        assert!(
            aow.is_some(),
            "serious topic + user needs help → ActionOverWords"
        );
        assert!(
            aow.unwrap().express_explicitly,
            "action over words always explicit"
        );
    }

    #[test]
    fn test_decide_action_over_words_not_when_casual() {
        // 话题轻松 → 不触发 ActionOverWords
        let signals = SubtextEngine::decide(
            [0.0, 0.0, 0.0],
            [-0.3, 0.4, -0.1],
            &familiar(),
            0.3, // 话题轻松
        );
        assert!(
            !signals
                .iter()
                .any(|s| s.kind == SubtextKind::ActionOverWords),
            "casual topic → no ActionOverWords"
        );
    }

    #[test]
    fn test_decide_deflected_concern() {
        // AI 忧伤 + 初识 → DeflectedConcern
        let signals = SubtextEngine::decide(
            [-0.4, -0.1, -0.2], // AI 忧伤
            [0.0, 0.0, 0.0],
            &acq(),
            0.3,
        );
        let dc = signals
            .iter()
            .find(|s| s.kind == SubtextKind::DeflectedConcern);
        assert!(dc.is_some(), "AI sad + acquaintance → DeflectedConcern");
        assert!(
            dc.unwrap().express_explicitly,
            "deflected concern always explicit"
        );
    }

    #[test]
    fn test_decide_deflected_concern_not_in_deep() {
        // AI 忧伤 + 深度关系 → 不触发 DeflectedConcern（深度关系可以暴露脆弱）
        let signals = SubtextEngine::decide([-0.4, -0.1, -0.2], [0.0, 0.0, 0.0], &deep(), 0.3);
        assert!(
            !signals
                .iter()
                .any(|s| s.kind == SubtextKind::DeflectedConcern),
            "deep relationship → no DeflectedConcern"
        );
    }

    #[test]
    fn test_decide_max_two_signals() {
        // 触发多个规则时，最多返回 2 个
        let signals = SubtextEngine::decide(
            [-0.5, 0.6, -0.2], // AI 焦虑+忧伤 → 可能触发多个
            [-0.4, 0.3, -0.1], // 用户也偏负+高唤醒
            &familiar(),
            0.8, // 话题严肃
        );
        assert!(signals.len() <= 2, "at most 2 subtext signals");
    }

    #[test]
    fn test_decide_no_subtext_when_all_neutral() {
        // 双方中性 → 无潜台词
        let signals = SubtextEngine::decide([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], &familiar(), 0.3);
        assert!(signals.is_empty(), "both neutral → no AI subtext");
    }
}
