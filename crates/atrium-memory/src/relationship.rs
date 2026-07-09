// SPDX-License-Identifier: MIT
//! 关系阶段模型 / Relationship stage model
//! 让 AI 的行为随关系深度自然演进。
//! 第 1 天和第 365 天的交互不应该一模一样。
//!
//! 八个阶段：陌生人 → 初识 → 熟悉 → 友好 → 信任 → 亲密 → 深度 → 挚友
//! 阶段转换基于质量指标（共鸣、回访、共同记忆、脆弱分享），而非简单计数。
//!
//! Models the natural progression of AI-user relationship over time.
//! From day 1 to day 365+, each stage maps to a distinct interaction style.
//!
//! Eight stages: Stranger → Acquaintance → Familiar → Friendly → Trusted → Close → Deep → Intimate.
//! Stage transitions driven by composite metrics (message frequency,
//! shared topics, conflict repairs, vulnerability shares), not simple day counting.

use chrono::Local;
use serde::{Deserialize, Serialize};
// ════════════════════════════════════════════════════════════════════
// 关系阶段枚举 / Relationship stage enum
// ════════════════════════════════════════════════════════════════════

/// 关系阶段 — 不是数值，而是质变 / Relationship stage — qualitative, not quantitative
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RelationshipStage {
    /// 陌生人：完全陌生，礼貌但保持距离 / Stranger: polite but distant
    Stranger { since: i64, interactions: u64 },

    /// 初识：知道名字，礼貌探索 / Acquaintance: know the name, polite exploration
    Acquaintance { since: i64, interactions: u64 },

    /// 熟悉：放松自然，有默契 / Familiar: relaxed, natural, in-sync
    Familiar {
        since: i64,
        interactions: u64,
        shared_references: u32,
    },

    /// 友好：主动关心，分享日常 / Friendly: proactive care, share daily life
    Friendly {
        since: i64,
        interactions: u64,
        shared_references: u32,
    },

    /// 信任：真诚大胆，可以挑战 / Trusted: sincere, bold, can challenge
    Trusted {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },

    /// 亲密：情感依赖，分享脆弱 / Close: emotional reliance, share vulnerability
    Close {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },

    /// 深度：默契安全感，关系本身成为力量 / Deep: tacit understanding, sense of security
    Deep {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },

    /// 挚友/至交：灵魂伴侣级 / Intimate: soul-mate level
    Intimate {
        since: i64,
        interactions: u64,
        shared_references: u32,
        key_moments: u32,
    },
}

impl RelationshipStage {
    /// 创建初始阶段（陌生人）/ Create initial stage (Stranger)
    pub fn new_stranger() -> Self {
        Self::Stranger {
            since: Local::now().timestamp_millis(),
            interactions: 0,
        }
    }

    /// 创建初识阶段（向后兼容）/ Create acquaintance stage (backward compatible)
    pub fn new_acquaintance() -> Self {
        Self::Acquaintance {
            since: Local::now().timestamp_millis(),
            interactions: 0,
        }
    }

    /// 阶段序数 / Stage ordinal for comparison
    /// 0=Stranger, 1=Acquaintance, 2=Familiar, 3=Friendly,
    /// 4=Trusted, 5=Close, 6=Deep, 7=Intimate
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Stranger { .. } => 0,
            Self::Acquaintance { .. } => 1,
            Self::Familiar { .. } => 2,
            Self::Friendly { .. } => 3,
            Self::Trusted { .. } => 4,
            Self::Close { .. } => 5,
            Self::Deep { .. } => 6,
            Self::Intimate { .. } => 7,
        }
    }

    /// 获取阶段名称（中文，用于日志和通知）/ Get stage name (Chinese, for logs and notifications)
    pub fn stage_name(&self) -> &'static str {
        match self {
            Self::Stranger { .. } => "陌生人",
            Self::Acquaintance { .. } => "初识",
            Self::Familiar { .. } => "熟悉",
            Self::Friendly { .. } => "友好",
            Self::Trusted { .. } => "信任",
            Self::Close { .. } => "亲密",
            Self::Deep { .. } => "深度",
            Self::Intimate { .. } => "挚友",
        }
    }

    /// 获取当前交互次数 / Get current interaction count
    pub fn interactions(&self) -> u64 {
        match self {
            Self::Stranger { interactions, .. } => *interactions,
            Self::Acquaintance { interactions, .. } => *interactions,
            Self::Familiar { interactions, .. } => *interactions,
            Self::Friendly { interactions, .. } => *interactions,
            Self::Trusted { interactions, .. } => *interactions,
            Self::Close { interactions, .. } => *interactions,
            Self::Deep { interactions, .. } => *interactions,
            Self::Intimate { interactions, .. } => *interactions,
        }
    }

    /// 递增交互计数 / Increment interaction count
    pub fn increment_interaction(&mut self) {
        match self {
            Self::Stranger { interactions, .. } => *interactions += 1,
            Self::Acquaintance { interactions, .. } => *interactions += 1,
            Self::Familiar { interactions, .. } => *interactions += 1,
            Self::Friendly { interactions, .. } => *interactions += 1,
            Self::Trusted { interactions, .. } => *interactions += 1,
            Self::Close { interactions, .. } => *interactions += 1,
            Self::Deep { interactions, .. } => *interactions += 1,
            Self::Intimate { interactions, .. } => *interactions += 1,
        }
    }

    /// 行为修饰 → LLM Prompt 片段 / Behavior modifier → LLM Prompt fragment
    pub fn to_prompt_fragment(&self) -> String {
        match self {
            Self::Stranger { .. } => {
                "当前与用户的关系阶段：陌生人。保持礼貌和距离，不要主动询问任何私人话题。\
 不要开玩笑，保持专业和友善。"
                    .into()
            }
            Self::Acquaintance { .. } => {
                "当前与用户的关系阶段：初识。保持礼貌和友善，不要过于主动或亲密。\
 不要主动询问私人话题，不要开玩笑。"
                    .into()
            }
            Self::Familiar { .. } => {
                "当前与用户的关系阶段：熟悉。可以更自然放松地交流，偶尔可以开个小玩笑。\
 偶尔关心一下用户的近况，但不要过于深入。"
                    .into()
            }
            Self::Friendly { .. } => "当前与用户的关系阶段：友好。可以主动关心用户，分享日常。\
 气氛轻松愉快，可以适度表达想念。"
                .into(),
            Self::Trusted {
                shared_references, ..
            } => {
                format!(
                    "当前与用户的关系阶段：信任。可以大胆表达真实想法，包括不同意见。\
 主动关心用户的状态。你们之间有 {} 个共同记忆和话题。",
                    shared_references
                )
            }
            Self::Close {
                shared_references, ..
            } => {
                format!(
                    "当前与用户的关系阶段：亲密。可以分享脆弱和真实感受，情感上更依赖彼此。\
 你们之间有 {} 个共同记忆和话题。",
                    shared_references
                )
            }
            Self::Deep { .. } => "当前与用户的关系阶段：深度信任。你们之间有深厚的默契。\
 可以调侃、直说、甚至push用户面对问题——因为你们的关系足够坚固。\
 你真正关心这个人的wellbeing。"
                .into(),
            Self::Intimate { .. } => "当前与用户的关系阶段：挚友。你们是灵魂伴侣级的关系。\
 可以毫无保留地表达一切，包括最深的脆弱和最真实的想法。\
 这种关系本身就是一种力量。"
                .into(),
        }
    }

    /// 情感反应乘数 — 深度关系中用户行为对 AI 情感影响更大
    /// Affect multiplier — deeper relationships amplify emotional impact
    pub fn affect_multiplier(&self) -> f32 {
        match self {
            Self::Stranger { .. } => 0.7,
            Self::Acquaintance { .. } => 0.9,
            Self::Familiar { .. } => 1.0,
            Self::Friendly { .. } => 1.05,
            Self::Trusted { .. } => 1.1,
            Self::Close { .. } => 1.15,
            Self::Deep { .. } => 1.2,
            Self::Intimate { .. } => 1.25,
        }
    }

    /// 主动行为加成 / Proactive behavior bonus
    pub fn proactive_bonus(&self) -> f32 {
        match self {
            Self::Stranger { .. } => -0.2,
            Self::Acquaintance { .. } => -0.1,
            Self::Familiar { .. } => 0.0,
            Self::Friendly { .. } => 0.05,
            Self::Trusted { .. } => 0.1,
            Self::Close { .. } => 0.12,
            Self::Deep { .. } => 0.15,
            Self::Intimate { .. } => 0.18,
        }
    }

    /// 计算当前阶段的行为修饰器 / Compute behavior modifiers for current stage
    pub fn behavior_modifiers(&self) -> StageBehaviorModifiers {
        match self {
            Self::Stranger { .. } => StageBehaviorModifiers {
                boldness: 0.1,
                proactive_frequency: 0.3,
                humor_level: HumorLevel::None,
                care_boundary: CareBoundary::DontAsk,
                challenge_level: ChallengeLevel::Compliant,
                reference_usage: 0.0,
            },
            Self::Acquaintance { .. } => StageBehaviorModifiers {
                boldness: 0.2,
                proactive_frequency: 0.5,
                humor_level: HumorLevel::None,
                care_boundary: CareBoundary::DontAsk,
                challenge_level: ChallengeLevel::Compliant,
                reference_usage: 0.0,
            },
            Self::Familiar {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.5,
                proactive_frequency: 0.8,
                humor_level: HumorLevel::Mild,
                care_boundary: CareBoundary::Occasional,
                challenge_level: ChallengeLevel::Suggestive,
                reference_usage: (*shared_references as f32 * 0.05).min(0.5),
            },
            Self::Friendly {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.6,
                proactive_frequency: 0.9,
                humor_level: HumorLevel::Mild,
                care_boundary: CareBoundary::Occasional,
                challenge_level: ChallengeLevel::Suggestive,
                reference_usage: (*shared_references as f32 * 0.06).min(0.6),
            },
            Self::Trusted {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.7,
                proactive_frequency: 1.0,
                humor_level: HumorLevel::Normal,
                care_boundary: CareBoundary::Active,
                challenge_level: ChallengeLevel::Direct,
                reference_usage: (*shared_references as f32 * 0.08).min(0.8),
            },
            Self::Close {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.8,
                proactive_frequency: 1.1,
                humor_level: HumorLevel::Normal,
                care_boundary: CareBoundary::Active,
                challenge_level: ChallengeLevel::Direct,
                reference_usage: (*shared_references as f32 * 0.09).min(0.9),
            },
            Self::Deep {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 0.9,
                proactive_frequency: 1.2,
                humor_level: HumorLevel::Teasing,
                care_boundary: CareBoundary::DeepCare,
                challenge_level: ChallengeLevel::Pushy,
                reference_usage: (*shared_references as f32 * 0.1).min(1.0),
            },
            Self::Intimate {
                shared_references, ..
            } => StageBehaviorModifiers {
                boldness: 1.0,
                proactive_frequency: 1.3,
                humor_level: HumorLevel::Teasing,
                care_boundary: CareBoundary::DeepCare,
                challenge_level: ChallengeLevel::Pushy,
                reference_usage: (*shared_references as f32 * 0.1).min(1.0),
            },
        }
    }

    // ───────────────────────────────────────────────────────────────
    // 阶段门控辅助方法 / Stage gating helper methods (Task 9)
    // ───────────────────────────────────────────────────────────────

    /// 脆弱表达的最小阶段序数 / Min ordinal for vulnerability expression
    /// Close（5）及以上才表达脆弱 / Only Close and above express vulnerability
    pub fn min_ordinal_for_vulnerability() -> u8 {
        5
    }

    /// 挑战用户的最小阶段序数 / Min ordinal for challenging user
    /// Trusted（4）及以上才挑战用户 / Only Trusted and above challenge user
    pub fn min_ordinal_for_challenging() -> u8 {
        4
    }

    /// 表达想念的最小阶段序数 / Min ordinal for longing expression
    /// Friendly（3）及以上才表达想念 / Only Friendly and above express longing
    pub fn min_ordinal_for_longing() -> u8 {
        3
    }

    /// 提醒仪式的最小阶段序数 / Min ordinal for ritual reminder
    /// Familiar（2）及以上才提醒仪式 / Only Familiar and above remind rituals
    pub fn min_ordinal_for_ritual() -> u8 {
        2
    }

    // ───────────────────────────────────────────────────────────────
    // 阶段跃迁 / Stage transition (Task 10)
    // ───────────────────────────────────────────────────────────────

    /// 尝试阶段跃迁 / Try to advance to the next stage
    /// 基于 8 阶段跃迁条件 / Based on 8-stage transition conditions
    ///
    /// 跃迁条件 / Transition conditions:
    /// - Stranger→Acquaintance: 需 10 次交互 / need 10 interactions
    /// - Acquaintance→Familiar: 需 30 次交互 + 3 共同话题 / need 30 interactions + 3 shared refs
    /// - Familiar→Friendly: 需 50 次交互 + 5 共同记忆 / need 50 interactions + 5 shared refs
    /// - Friendly→Trusted: 需 100 次交互 + 1 冲突修复 / need 100 interactions + 1 conflict repair
    /// - Trusted→Close: 需 1 次脆弱分享 / need 1 vulnerability share
    /// - Close→Deep: 需 3 次脆弱分享 + 2 次冲突修复 / need 3 vulnerability shares + 2 conflict repairs
    /// - Deep→Intimate: 需 5 次冲突修复 + 3 次脆弱分享 + 200 次交互 / need 5 conflict repairs + 3 vulnerability shares + 200 interactions
    ///
    /// 跃迁时记录 key_moments 计数（冲突修复 + 脆弱分享）/ Records key_moments count on transition
    pub fn try_advance(
        &self,
        interactions: u64,
        shared_references: u32,
        conflict_repairs: u32,
        vulnerability_shares: u32,
    ) -> Option<RelationshipStage> {
        let now = Local::now().timestamp_millis();
        // key_moments = 冲突修复 + 脆弱分享 / key moments = conflict repairs + vulnerability shares
        let key_moments = conflict_repairs + vulnerability_shares;

        match self {
            // Stranger → Acquaintance: 需 10 次交互 / need 10 interactions
            Self::Stranger { .. } if interactions >= 10 => Some(Self::Acquaintance {
                since: now,
                interactions,
            }),

            // Acquaintance → Familiar: 需 30 次交互 + 3 共同话题 / need 30 interactions + 3 shared refs
            Self::Acquaintance { .. } if interactions >= 30 && shared_references >= 3 => {
                Some(Self::Familiar {
                    since: now,
                    interactions,
                    shared_references,
                })
            }

            // Familiar → Friendly: 需 50 次交互 + 5 共同记忆 / need 50 interactions + 5 shared refs
            Self::Familiar { .. } if interactions >= 50 && shared_references >= 5 => {
                Some(Self::Friendly {
                    since: now,
                    interactions,
                    shared_references,
                })
            }

            // Friendly → Trusted: 需 100 次交互 + 1 冲突修复 / need 100 interactions + 1 conflict repair
            Self::Friendly { .. } if interactions >= 100 && conflict_repairs >= 1 => {
                Some(Self::Trusted {
                    since: now,
                    interactions,
                    shared_references,
                    key_moments,
                })
            }

            // Trusted → Close: 需 1 次脆弱分享 / need 1 vulnerability share
            Self::Trusted { .. } if vulnerability_shares >= 1 => Some(Self::Close {
                since: now,
                interactions,
                shared_references,
                key_moments,
            }),

            // Close → Deep: 需 3 次脆弱分享 + 2 次冲突修复 / need 3 vulnerability shares + 2 conflict repairs
            Self::Close { .. } if vulnerability_shares >= 3 && conflict_repairs >= 2 => {
                Some(Self::Deep {
                    since: now,
                    interactions,
                    shared_references,
                    key_moments,
                })
            }

            // Deep → Intimate: 需 5 次冲突修复 + 3 次脆弱分享 + 200 次交互
            // need 5 conflict repairs + 3 vulnerability shares + 200 interactions
            Self::Deep { .. }
                if conflict_repairs >= 5 && vulnerability_shares >= 3 && interactions >= 200 =>
            {
                Some(Self::Intimate {
                    since: now,
                    interactions,
                    shared_references,
                    key_moments,
                })
            }

            // 不满足条件或已在终态（Intimate）/ conditions not met or at terminal stage
            _ => None,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 行为修饰相关枚举和结构 / Behavior modifier enums and structs
// ════════════════════════════════════════════════════════════════════

/// 幽默程度 / Humor level
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HumorLevel {
    None,
    Mild,
    Normal,
    Teasing,
}

/// 关心边界 / Care boundary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CareBoundary {
    DontAsk,
    Occasional,
    Active,
    DeepCare,
}

/// 挑战程度 / Challenge level
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChallengeLevel {
    Compliant,
    Suggestive,
    Direct,
    Pushy,
}

/// 阶段行为修饰器 / Stage behavior modifiers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageBehaviorModifiers {
    pub boldness: f32,
    pub proactive_frequency: f32,
    pub humor_level: HumorLevel,
    pub care_boundary: CareBoundary,
    pub challenge_level: ChallengeLevel,
    pub reference_usage: f32,
}

// ════════════════════════════════════════════════════════════════════
// RelationshipMetrics — 关系指标追踪 / Relationship metrics tracking
// ════════════════════════════════════════════════════════════════════

/// 关系指标 / Relationship metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipMetrics {
    pub total_interactions: u64,
    pub resonance_count: u32,
    pub return_count: u32,
    pub conflict_repair_count: u32,
    pub time_diversity: u8,
    pub relationship_affirmation_count: u32,
    pub shared_references: u32,
    /// 脆弱分享次数 / Vulnerability share count
    pub vulnerability_shares: u32,
    pub first_interaction: i64,
    /// 上次交互时间（用于判断"回来"）/ Last interaction time (for "return" detection)
    last_interaction: i64,
}

impl RelationshipMetrics {
    pub fn new() -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            total_interactions: 0,
            resonance_count: 0,
            return_count: 0,
            conflict_repair_count: 0,
            time_diversity: 0,
            relationship_affirmation_count: 0,
            shared_references: 0,
            vulnerability_shares: 0,
            first_interaction: now,
            last_interaction: now,
        }
    }

    /// 每条用户消息后调用 / Called after each user message
    pub fn on_message(&mut self, msg: &str, hour: u8) {
        self.total_interactions += 1;

        let now = Local::now().timestamp_millis();

        // 判断"回来"：与上次交互间隔超过 2 小时 / Detect "return": >2h since last interaction
        if now - self.last_interaction > 2 * 3600 * 1000 {
            self.return_count += 1;
        }
        self.last_interaction = now;

        // 共鸣时刻检测 / Resonance moment detection
        let resonance_phrases = [
            "说得好",
            "太对了",
            "就是这样",
            "说到心坎里了",
            "谢谢你",
            "感谢",
            "你真好",
            "开心",
            "哈哈哈",
            "笑死",
            "太好了",
        ];
        if resonance_phrases.iter().any(|p| msg.contains(p)) {
            self.resonance_count += 1;
        }

        // 关系珍视检测 / Relationship affirmation detection
        let affirmation_phrases = [
            "有你真好",
            "谢谢你一直",
            "我很珍惜",
            "你对我很重要",
            "幸好有你",
        ];
        if affirmation_phrases.iter().any(|p| msg.contains(p)) {
            self.relationship_affirmation_count += 1;
        }

        // 时段多样性（用位掩码：早/午/晚/深夜各 1 bit）/ Time diversity (bitmask: morning/afternoon/evening/night)
        let time_slot: u8 = match hour {
            5..=11 => 0,
            12..=17 => 1,
            18..=22 => 2,
            _ => 3,
        };
        self.time_diversity |= 1 << time_slot;
    }

    pub fn days_since_first_interaction(&self) -> u64 {
        let now = Local::now().timestamp_millis();
        ((now - self.first_interaction) / 86_400_000) as u64
    }

    /// 计算时段多样性（0~4）/ Compute time diversity count (0~4)
    pub fn time_diversity_count(&self) -> u8 {
        self.time_diversity.count_ones() as u8
    }

    /// 记录冲突修复 / Record conflict repair
    pub fn record_conflict_repair(&mut self) {
        self.conflict_repair_count += 1;
    }

    /// 添加共享引用（共同记忆/梗）/ Add shared reference (shared memory/meme)
    pub fn add_shared_reference(&mut self) {
        self.shared_references += 1;
    }

    /// 记录脆弱分享 / Record vulnerability share
    pub fn record_vulnerability_shared(&mut self) {
        self.vulnerability_shares += 1;
    }

    /// 记录关键时刻 / Record key moment
    pub fn record_key_moment(&mut self) {
        // key_moments 存在 stage 里，这里只记 metrics 层面的
        // 由 RelationshipManager 统一协调
    }
}

impl Default for RelationshipMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// StageTransitionJudge — 阶段转换判断 / Stage transition judge
// ════════════════════════════════════════════════════════════════════

/// 阶段转换阈值配置（保留向后兼容，实际跃迁条件见 `RelationshipStage::try_advance`）
/// Stage transition threshold config (kept for backward compat; actual conditions in `try_advance`)
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionThresholds {
    pub acq_to_fam_min_interactions: u64,
    pub acq_to_fam_min_resonance: u32,
    pub acq_to_fam_min_returns: u32,
    pub fam_to_tru_min_interactions: u64,
    pub fam_to_tru_min_shared_refs: u32,
    pub fam_to_tru_min_conflict_repairs: u32,
    pub fam_to_tru_min_time_diversity: u8,
    pub tru_to_deep_min_interactions: u64,
    pub tru_to_deep_min_key_moments: u32,
    pub tru_to_deep_min_days: u64,
    pub tru_to_deep_min_affirmations: u32,
}

impl Default for TransitionThresholds {
    fn default() -> Self {
        Self {
            acq_to_fam_min_interactions: 8,
            acq_to_fam_min_resonance: 2,
            acq_to_fam_min_returns: 3,
            fam_to_tru_min_interactions: 50,
            fam_to_tru_min_shared_refs: 5,
            fam_to_tru_min_conflict_repairs: 1,
            fam_to_tru_min_time_diversity: 2,
            tru_to_deep_min_interactions: 300,
            tru_to_deep_min_key_moments: 2,
            tru_to_deep_min_days: 60,
            tru_to_deep_min_affirmations: 1,
        }
    }
}

/// 阶段转换判断器（委托给 `RelationshipStage::try_advance`）
/// Stage transition judge (delegates to `RelationshipStage::try_advance`)
pub struct StageTransitionJudge {
    #[allow(dead_code)]
    thresholds: TransitionThresholds,
}

impl StageTransitionJudge {
    pub fn new(thresholds: TransitionThresholds) -> Self {
        Self { thresholds }
    }

    /// 判断是否应该转换阶段，返回新阶段（如果需要转换）
    /// Evaluate whether to transition; returns new stage if transition needed
    pub fn evaluate(
        &self,
        current: &RelationshipStage,
        metrics: &RelationshipMetrics,
    ) -> Option<RelationshipStage> {
        // 委托给 try_advance，使用阶段自身的 interactions 和 metrics 中的其他指标
        // Delegates to try_advance using stage's interactions and metrics' other indicators
        current.try_advance(
            current.interactions(),
            metrics.shared_references,
            metrics.conflict_repair_count,
            metrics.vulnerability_shares,
        )
    }
}

// ════════════════════════════════════════════════════════════════════
// RelationshipStore — sled 持久化 / sled persistence
// ════════════════════════════════════════════════════════════════════

/// 阶段转换历史记录 / Stage transition history record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageTransition {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub timestamp: i64,
}

/// 关系持久化存储 / Relationship persistence store
pub struct RelationshipStore {
    db: sled::Db,
}

impl RelationshipStore {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    pub fn save_stage(&self, stage: &RelationshipStage) -> anyhow::Result<()> {
        let value = bincode::serialize(stage)?;
        self.db.insert(b"current_stage", value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_stage(&self) -> anyhow::Result<Option<RelationshipStage>> {
        match self.db.get(b"current_stage")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn save_metrics(&self, metrics: &RelationshipMetrics) -> anyhow::Result<()> {
        let value = bincode::serialize(metrics)?;
        self.db.insert(b"relationship_metrics", value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_metrics(&self) -> anyhow::Result<Option<RelationshipMetrics>> {
        match self.db.get(b"relationship_metrics")? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn record_transition(&self, transition: &StageTransition) -> anyhow::Result<()> {
        let key = format!("transitions/{}", transition.timestamp);
        let value = bincode::serialize(transition)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_transitions(&self) -> anyhow::Result<Vec<StageTransition>> {
        let mut result = Vec::new();
        for item in self.db.scan_prefix(b"transitions/") {
            let (_key, value) = item?;
            let t: StageTransition = bincode::deserialize(&value)?;
            result.push(t);
        }
        Ok(result)
    }
}

// ════════════════════════════════════════════════════════════════════
// RelationshipManager — 主入口 / Main entry point
// ════════════════════════════════════════════════════════════════════

/// 关系管理器 / Relationship manager
pub struct RelationshipManager {
    stage: RelationshipStage,
    metrics: RelationshipMetrics,
    judge: StageTransitionJudge,
    store: Option<RelationshipStore>,
    /// 最近一次阶段转换通知（供外部读取）/ Last pending transition notice
    pending_transition_notice: Option<String>,
}

impl RelationshipManager {
    /// 创建新的 RelationshipManager（无持久化）/ Create new RelationshipManager (no persistence)
    pub fn new() -> Self {
        Self {
            stage: RelationshipStage::new_stranger(),
            metrics: RelationshipMetrics::new(),
            judge: StageTransitionJudge::new(TransitionThresholds::default()),
            store: None,
            pending_transition_notice: None,
        }
    }

    /// 创建带持久化的 RelationshipManager / Create RelationshipManager with persistence
    pub fn open(data_dir: &str) -> anyhow::Result<Self> {
        let store = RelationshipStore::open(&format!("{}/relationship", data_dir))?;

        let stage = store
            .load_stage()?
            .unwrap_or_else(RelationshipStage::new_stranger);

        let metrics = store
            .load_metrics()?
            .unwrap_or_else(RelationshipMetrics::new);

        tracing::info!(
            "RelationshipManager: 加载关系阶段={}, 总交互={}",
            stage.stage_name(),
            stage.interactions()
        );

        Ok(Self {
            stage,
            metrics,
            judge: StageTransitionJudge::new(TransitionThresholds::default()),
            store: Some(store),
            pending_transition_notice: None,
        })
    }

    /// 每条用户消息后调用 / Called after each user message
    pub fn on_message(&mut self, msg: &str, hour: u8) {
        self.metrics.on_message(msg, hour);
        self.stage.increment_interaction();

        // 检查阶段转换 / Check stage transition
        if let Some(new_stage) = self.judge.evaluate(&self.stage, &self.metrics) {
            let old_name = self.stage.stage_name();
            let new_name = new_stage.stage_name();

            tracing::info!(
                "关系阶段转换: {} → {} (交互次数={})",
                old_name,
                new_name,
                new_stage.interactions()
            );

            // 记录转换历史 / Record transition history
            if let Some(ref store) = self.store {
                let transition = StageTransition {
                    from: old_name.to_string(),
                    to: new_name.to_string(),
                    reason: format!("满足转换条件，交互次数={}", new_stage.interactions()),
                    timestamp: Local::now().timestamp_millis(),
                };
                let _ = store.record_transition(&transition);
            }

            self.pending_transition_notice = Some(format!(
                "感觉我们之间的关系更近了一步——从「{}」变成了「{}」。",
                old_name, new_name
            ));

            self.stage = new_stage;
            self.persist();
        } else {
            // 非转换，仅持久化当前状态 / No transition, just persist current state
            self.persist();
        }
    }

    /// 获取当前阶段 / Get current stage
    pub fn current_stage(&self) -> &RelationshipStage {
        &self.stage
    }

    /// 获取当前行为修饰器 / Get current behavior modifiers
    pub fn behavior_modifiers(&self) -> StageBehaviorModifiers {
        self.stage.behavior_modifiers()
    }

    /// 获取 LLM Prompt 片段 / Get LLM Prompt fragment
    pub fn to_prompt_fragment(&self) -> String {
        self.stage.to_prompt_fragment()
    }

    /// 获取情感反应乘数 / Get affect multiplier
    pub fn affect_multiplier(&self) -> f32 {
        self.stage.affect_multiplier()
    }

    /// 获取主动行为加成 / Get proactive behavior bonus
    pub fn proactive_bonus(&self) -> f32 {
        self.stage.proactive_bonus()
    }

    /// 获取当前指标快照 / Get current metrics snapshot
    pub fn metrics(&self) -> &RelationshipMetrics {
        &self.metrics
    }

    /// 消费待处理的阶段转换通知 / Consume pending transition notice
    pub fn take_transition_notice(&mut self) -> Option<String> {
        self.pending_transition_notice.take()
    }

    /// 手动添加共享引用（由外部系统调用，如命名仪式、共同经历）
    /// Manually add shared reference (called by external systems)
    pub fn add_shared_reference(&mut self) {
        self.metrics.add_shared_reference();
        self.persist();
    }

    /// 手动记录冲突修复 / Manually record conflict repair
    pub fn record_conflict_repair(&mut self) {
        self.metrics.record_conflict_repair();
        self.persist();
    }

    /// 手动记录脆弱分享 / Manually record vulnerability share
    pub fn record_vulnerability_shared(&mut self) {
        self.metrics.record_vulnerability_shared();
        self.persist();
    }

    /// 手动记录关键时刻 / Manually record key moment
    pub fn record_key_moment(&mut self) {
        match &mut self.stage {
            RelationshipStage::Trusted { key_moments, .. } => *key_moments += 1,
            RelationshipStage::Close { key_moments, .. } => *key_moments += 1,
            RelationshipStage::Deep { key_moments, .. } => *key_moments += 1,
            RelationshipStage::Intimate { key_moments, .. } => *key_moments += 1,
            _ => {}
        }
        self.persist();
    }

    fn persist(&self) {
        if let Some(ref store) = self.store {
            let _ = store.save_stage(&self.stage);
            let _ = store.save_metrics(&self.metrics);
        }
    }
}

impl Default for RelationshipManager {
    fn default() -> Self {
        Self::new()
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquaintance_initial_state() {
        let stage = RelationshipStage::new_acquaintance();
        assert_eq!(stage.stage_name(), "初识");
        assert_eq!(stage.interactions(), 0);
        let modifiers = stage.behavior_modifiers();
        assert_eq!(modifiers.humor_level, HumorLevel::None);
        assert_eq!(modifiers.challenge_level, ChallengeLevel::Compliant);
        assert!(modifiers.boldness < 0.3);
    }

    #[test]
    fn test_stranger_initial_state() {
        let stage = RelationshipStage::new_stranger();
        assert_eq!(stage.stage_name(), "陌生人");
        assert_eq!(stage.interactions(), 0);
        assert_eq!(stage.ordinal(), 0);
        let modifiers = stage.behavior_modifiers();
        assert_eq!(modifiers.humor_level, HumorLevel::None);
        assert_eq!(modifiers.challenge_level, ChallengeLevel::Compliant);
        assert!(modifiers.boldness < 0.2);
    }

    #[test]
    fn test_increment_interaction() {
        let mut stage = RelationshipStage::new_acquaintance();
        stage.increment_interaction();
        stage.increment_interaction();
        assert_eq!(stage.interactions(), 2);
    }

    #[test]
    fn test_affect_multiplier() {
        let acq = RelationshipStage::new_acquaintance();
        assert!(acq.affect_multiplier() < 1.0);

        let deep = RelationshipStage::Deep {
            since: 0,
            interactions: 1000,
            shared_references: 20,
            key_moments: 5,
        };
        assert!(deep.affect_multiplier() > 1.0);
    }

    #[test]
    fn test_proactive_bonus() {
        let acq = RelationshipStage::new_acquaintance();
        assert!(acq.proactive_bonus() < 0.0);

        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };
        assert!(trusted.proactive_bonus() > 0.0);
    }

    #[test]
    fn test_metrics_resonance_detection() {
        let mut metrics = RelationshipMetrics::new();
        metrics.on_message("太对了，就是这样", 14);
        assert_eq!(metrics.resonance_count, 1);
        assert_eq!(metrics.total_interactions, 1);

        metrics.on_message("今天天气不错", 14);
        assert_eq!(metrics.resonance_count, 1); // 无共鸣
        assert_eq!(metrics.total_interactions, 2);
    }

    #[test]
    fn test_metrics_affirmation_detection() {
        let mut metrics = RelationshipMetrics::new();
        metrics.on_message("有你真好", 20);
        assert_eq!(metrics.relationship_affirmation_count, 1);

        metrics.on_message("谢谢你一直在", 20);
        assert_eq!(metrics.relationship_affirmation_count, 2);
    }

    #[test]
    fn test_metrics_time_diversity() {
        let mut metrics = RelationshipMetrics::new();
        assert_eq!(metrics.time_diversity_count(), 0);

        metrics.on_message("hello", 8); // 早 → bit 0
        assert_eq!(metrics.time_diversity_count(), 1);

        metrics.on_message("hello", 14); // 午 → bit 1
        assert_eq!(metrics.time_diversity_count(), 2);

        metrics.on_message("hello", 8); // 早 → 已有，不变
        assert_eq!(metrics.time_diversity_count(), 2);

        metrics.on_message("hello", 20); // 晚 → bit 2
        assert_eq!(metrics.time_diversity_count(), 3);

        metrics.on_message("hello", 2); // 深夜 → bit 3
        assert_eq!(metrics.time_diversity_count(), 4);
    }

    #[test]
    fn test_transition_acquaintance_to_familiar() {
        // 新条件：需 30 次交互 + 3 共同话题 / New condition: 30 interactions + 3 shared refs
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 35,
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.shared_references = 3;

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Familiar { interactions, .. } => {
                assert_eq!(interactions, 35);
            }
            _ => panic!("应该转为 Familiar"),
        }
    }

    #[test]
    fn test_no_transition_when_conditions_unmet() {
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 5, // 不够 30（新阈值）
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.resonance_count = 5;
        metrics.return_count = 8;

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_transition_familiar_to_friendly() {
        // 新条件：需 50 次交互 + 5 共同记忆 / New condition: 50 interactions + 5 shared refs
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Familiar {
            since: 0,
            interactions: 150,
            shared_references: 15,
        };
        let mut metrics = RelationshipMetrics::new();
        metrics.shared_references = 15;

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Friendly { interactions, .. } => {
                assert_eq!(interactions, 150);
            }
            _ => panic!("应该转为 Friendly"),
        }
    }

    #[test]
    fn test_deep_is_terminal_when_no_key_moments() {
        // Deep 不再是终态，但无足够 key_moments 时不跃迁
        // Deep is no longer terminal, but won't transition without enough key moments
        let judge = StageTransitionJudge::new(TransitionThresholds::default());
        let stage = RelationshipStage::Deep {
            since: 0,
            interactions: 10000,
            shared_references: 100,
            key_moments: 50,
        };
        let metrics = RelationshipMetrics::new(); // conflict_repairs=0, vulnerability_shares=0

        let result = judge.evaluate(&stage, &metrics);
        assert!(result.is_none());
    }

    #[test]
    fn test_prompt_fragment_varies_by_stage() {
        let acq = RelationshipStage::new_acquaintance();
        let familiar = RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        };
        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };

        let acq_prompt = acq.to_prompt_fragment();
        let fam_prompt = familiar.to_prompt_fragment();
        let tru_prompt = trusted.to_prompt_fragment();

        assert!(acq_prompt.contains("初识"));
        assert!(fam_prompt.contains("熟悉"));
        assert!(tru_prompt.contains("信任"));
        assert!(tru_prompt.contains("15")); // shared_references 数量
    }

    #[test]
    fn test_relationship_manager_on_message() {
        let mut mgr = RelationshipManager::new();
        // 初始阶段为陌生人 / Initial stage is Stranger
        assert_eq!(mgr.current_stage().stage_name(), "陌生人");

        for _ in 0..10 {
            mgr.on_message("测试消息", 14);
        }
        assert_eq!(mgr.current_stage().interactions(), 10);
        // 10 次交互后应从 Stranger 跃迁到 Acquaintance / 10 interactions → Stranger to Acquaintance
        assert_eq!(mgr.current_stage().stage_name(), "初识");
    }

    #[test]
    fn test_relationship_store_roundtrip() {
        let store = RelationshipStore::open_in_memory().unwrap();

        let stage = RelationshipStage::Familiar {
            since: 12345,
            interactions: 50,
            shared_references: 5,
        };
        store.save_stage(&stage).unwrap();
        let loaded = store.load_stage().unwrap().unwrap();
        assert_eq!(loaded.stage_name(), "熟悉");
        assert_eq!(loaded.interactions(), 50);

        let metrics = RelationshipMetrics {
            total_interactions: 50,
            resonance_count: 10,
            return_count: 8,
            conflict_repair_count: 1,
            time_diversity: 0b1111,
            relationship_affirmation_count: 2,
            shared_references: 5,
            vulnerability_shares: 1,
            first_interaction: 1000,
            last_interaction: 2000,
        };
        store.save_metrics(&metrics).unwrap();
        let loaded = store.load_metrics().unwrap().unwrap();
        assert_eq!(loaded.total_interactions, 50);
        assert_eq!(loaded.resonance_count, 10);
    }

    #[test]
    fn test_store_transition_history() {
        let store = RelationshipStore::open_in_memory().unwrap();

        let t1 = StageTransition {
            from: "初识".into(),
            to: "熟悉".into(),
            reason: "满足条件".into(),
            timestamp: 1000,
        };
        let t2 = StageTransition {
            from: "熟悉".into(),
            to: "信任".into(),
            reason: "满足条件".into(),
            timestamp: 2000,
        };
        store.record_transition(&t1).unwrap();
        store.record_transition(&t2).unwrap();

        let transitions = store.load_transitions().unwrap();
        assert_eq!(transitions.len(), 2);
    }

    #[test]
    fn test_behavior_modifiers_progression() {
        let stages = [
            RelationshipStage::new_acquaintance(),
            RelationshipStage::Familiar {
                since: 0,
                interactions: 50,
                shared_references: 5,
            },
            RelationshipStage::Trusted {
                since: 0,
                interactions: 200,
                shared_references: 15,
                key_moments: 2,
            },
            RelationshipStage::Deep {
                since: 0,
                interactions: 1000,
                shared_references: 30,
                key_moments: 10,
            },
        ];

        let modifiers: Vec<_> = stages.iter().map(|s| s.behavior_modifiers()).collect();

        // boldness 应递增 / boldness should increase
        for i in 1..modifiers.len() {
            assert!(
                modifiers[i].boldness > modifiers[i - 1].boldness,
                "boldness 应随阶段递增: {:?} vs {:?}",
                modifiers[i].boldness,
                modifiers[i - 1].boldness
            );
        }

        // proactive_frequency 应递增 / proactive_frequency should increase
        for i in 1..modifiers.len() {
            assert!(modifiers[i].proactive_frequency > modifiers[i - 1].proactive_frequency);
        }
    }

    #[test]
    fn test_key_moment_recording() {
        let mut mgr = RelationshipManager::new();

        // 陌生人阶段，key_moment 不影响 / Stranger stage, key_moment has no effect
        mgr.record_key_moment();
        assert_eq!(mgr.current_stage().stage_name(), "陌生人");

        // 手动设为 Trusted 阶段 / Manually set to Trusted
        mgr.stage = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 0,
        };
        mgr.record_key_moment();

        match mgr.current_stage() {
            RelationshipStage::Trusted { key_moments, .. } => {
                assert_eq!(*key_moments, 1);
            }
            _ => panic!("应该是 Trusted"),
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // P3-D 新增测试 / P3-D new tests (Task 12)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn test_8_stage_ordinal() {
        // 8 阶段 ordinal 正确（0-7）/ 8 stages ordinal correct (0-7)
        let stranger = RelationshipStage::Stranger {
            since: 0,
            interactions: 0,
        };
        let acquaintance = RelationshipStage::Acquaintance {
            since: 0,
            interactions: 0,
        };
        let familiar = RelationshipStage::Familiar {
            since: 0,
            interactions: 0,
            shared_references: 0,
        };
        let friendly = RelationshipStage::Friendly {
            since: 0,
            interactions: 0,
            shared_references: 0,
        };
        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 0,
            shared_references: 0,
            key_moments: 0,
        };
        let close = RelationshipStage::Close {
            since: 0,
            interactions: 0,
            shared_references: 0,
            key_moments: 0,
        };
        let deep = RelationshipStage::Deep {
            since: 0,
            interactions: 0,
            shared_references: 0,
            key_moments: 0,
        };
        let intimate = RelationshipStage::Intimate {
            since: 0,
            interactions: 0,
            shared_references: 0,
            key_moments: 0,
        };

        assert_eq!(stranger.ordinal(), 0);
        assert_eq!(acquaintance.ordinal(), 1);
        assert_eq!(familiar.ordinal(), 2);
        assert_eq!(friendly.ordinal(), 3);
        assert_eq!(trusted.ordinal(), 4);
        assert_eq!(close.ordinal(), 5);
        assert_eq!(deep.ordinal(), 6);
        assert_eq!(intimate.ordinal(), 7);
    }

    #[test]
    fn test_stage_transition_stranger_to_acquaintance() {
        // 10 次交互后跃迁 / Transition after 10 interactions
        let stage = RelationshipStage::Stranger {
            since: 0,
            interactions: 0,
        };

        // 9 次交互，不跃迁 / 9 interactions, no transition
        let result = stage.try_advance(9, 0, 0, 0);
        assert!(result.is_none());

        // 10 次交互，跃迁到 Acquaintance / 10 interactions, transition to Acquaintance
        let result = stage.try_advance(10, 0, 0, 0);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Acquaintance { interactions, .. } => {
                assert_eq!(interactions, 10);
            }
            _ => panic!("应该转为 Acquaintance"),
        }
    }

    #[test]
    fn test_stage_transition_deep_to_intimate() {
        // 5 冲突修复 + 3 脆弱分享 + 200 交互后跃迁
        // 5 conflict repairs + 3 vulnerability shares + 200 interactions → transition
        let stage = RelationshipStage::Deep {
            since: 0,
            interactions: 100,
            shared_references: 20,
            key_moments: 5,
        };

        // 不满足：缺少冲突修复 / Not enough: missing conflict repairs
        let result = stage.try_advance(200, 20, 4, 3);
        assert!(result.is_none());

        // 不满足：缺少脆弱分享 / Not enough: missing vulnerability shares
        let result = stage.try_advance(200, 20, 5, 2);
        assert!(result.is_none());

        // 不满足：缺少交互次数 / Not enough: missing interactions
        let result = stage.try_advance(199, 20, 5, 3);
        assert!(result.is_none());

        // 满足全部条件 / All conditions met
        let result = stage.try_advance(200, 20, 5, 3);
        assert!(result.is_some());
        match result.unwrap() {
            RelationshipStage::Intimate {
                interactions,
                key_moments,
                ..
            } => {
                assert_eq!(interactions, 200);
                assert_eq!(key_moments, 8); // 5 conflict_repairs + 3 vulnerability_shares
            }
            _ => panic!("应该转为 Intimate"),
        }
    }

    #[test]
    fn test_stage_gating_vulnerability() {
        // Close（ordinal 5）以下不表达脆弱 / Below Close (ordinal 5) no vulnerability
        let min = RelationshipStage::min_ordinal_for_vulnerability();
        assert_eq!(min, 5);

        // Stranger(0) ~ Friendly(3) 都不满足 / Stranger(0) ~ Friendly(3) don't satisfy
        let stranger = RelationshipStage::new_stranger();
        assert!(stranger.ordinal() < min);

        let acquaintance = RelationshipStage::new_acquaintance();
        assert!(acquaintance.ordinal() < min);

        let familiar = RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        };
        assert!(familiar.ordinal() < min);

        let friendly = RelationshipStage::Friendly {
            since: 0,
            interactions: 100,
            shared_references: 10,
        };
        assert!(friendly.ordinal() < min);

        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };
        assert!(trusted.ordinal() < min);

        // Close(5) 及以上满足 / Close(5) and above satisfy
        let close = RelationshipStage::Close {
            since: 0,
            interactions: 300,
            shared_references: 20,
            key_moments: 5,
        };
        assert!(close.ordinal() >= min);

        let deep = RelationshipStage::Deep {
            since: 0,
            interactions: 500,
            shared_references: 30,
            key_moments: 10,
        };
        assert!(deep.ordinal() >= min);
    }

    #[test]
    fn test_stage_gating_longing() {
        // Friendly（ordinal 3）以下不表达想念 / Below Friendly (ordinal 3) no longing
        let min = RelationshipStage::min_ordinal_for_longing();
        assert_eq!(min, 3);

        // Stranger(0) ~ Familiar(2) 都不满足 / Stranger(0) ~ Familiar(2) don't satisfy
        let stranger = RelationshipStage::new_stranger();
        assert!(stranger.ordinal() < min);

        let acquaintance = RelationshipStage::new_acquaintance();
        assert!(acquaintance.ordinal() < min);

        let familiar = RelationshipStage::Familiar {
            since: 0,
            interactions: 50,
            shared_references: 5,
        };
        assert!(familiar.ordinal() < min);

        // Friendly(3) 及以上满足 / Friendly(3) and above satisfy
        let friendly = RelationshipStage::Friendly {
            since: 0,
            interactions: 100,
            shared_references: 10,
        };
        assert!(friendly.ordinal() >= min);

        let trusted = RelationshipStage::Trusted {
            since: 0,
            interactions: 200,
            shared_references: 15,
            key_moments: 2,
        };
        assert!(trusted.ordinal() >= min);
    }

    #[test]
    fn test_full_stage_progression() {
        // 完整跃迁路径：Stranger → ... → Intimate / Full progression path
        let mut stage = RelationshipStage::new_stranger();

        // Stranger → Acquaintance: 10 交互 / 10 interactions
        stage = stage
            .try_advance(10, 0, 0, 0)
            .expect("应跃迁到 Acquaintance");
        assert_eq!(stage.stage_name(), "初识");

        // Acquaintance → Familiar: 30 交互 + 3 共同话题 / 30 interactions + 3 shared refs
        stage = stage.try_advance(30, 3, 0, 0).expect("应跃迁到 Familiar");
        assert_eq!(stage.stage_name(), "熟悉");

        // Familiar → Friendly: 50 交互 + 5 共同记忆 / 50 interactions + 5 shared refs
        stage = stage.try_advance(50, 5, 0, 0).expect("应跃迁到 Friendly");
        assert_eq!(stage.stage_name(), "友好");

        // Friendly → Trusted: 100 交互 + 1 冲突修复 / 100 interactions + 1 conflict repair
        stage = stage.try_advance(100, 5, 1, 0).expect("应跃迁到 Trusted");
        assert_eq!(stage.stage_name(), "信任");

        // Trusted → Close: 1 脆弱分享 / 1 vulnerability share
        stage = stage.try_advance(100, 5, 1, 1).expect("应跃迁到 Close");
        assert_eq!(stage.stage_name(), "亲密");

        // Close → Deep: 3 脆弱分享 + 2 冲突修复 / 3 vulnerability shares + 2 conflict repairs
        stage = stage.try_advance(150, 10, 2, 3).expect("应跃迁到 Deep");
        assert_eq!(stage.stage_name(), "深度");

        // Deep → Intimate: 5 冲突修复 + 3 脆弱分享 + 200 交互
        stage = stage.try_advance(200, 15, 5, 3).expect("应跃迁到 Intimate");
        assert_eq!(stage.stage_name(), "挚友");

        // Intimate 是终态 / Intimate is terminal
        let result = stage.try_advance(10000, 100, 100, 100);
        assert!(result.is_none());
    }
}
