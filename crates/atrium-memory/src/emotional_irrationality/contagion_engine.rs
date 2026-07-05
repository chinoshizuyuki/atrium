// ! 传染引擎 / Contagion Engine
// ! 跨情境情绪传染、规则评估、冷却管理

use super::types::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 2.4 ContagionEngine — 传染引擎 ──

/// 传染规则条目 / Contagion Rule Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionRuleEntry {
    pub rule: ContagionRule,
    pub source: ContagionEmotion,
    pub target: ContagionEmotion,
    pub condition: ContagionCondition,
    pub pad_template: [f32; 3],
}

/// 传染引擎配置 / Contagion Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContagionConfig {
    pub max_chain_depth: u32,
    pub cooldown_secs: i64,
}

impl Default for ContagionConfig {
    fn default() -> Self {
        Self {
            max_chain_depth: 3,
            cooldown_secs: 300,
        }
    }
}

/// 传染引擎 / Contagion Engine
#[derive(Debug, Clone)]
pub struct ContagionEngine {
    pub config: ContagionConfig,
    pub rules: Vec<ContagionRuleEntry>,
    pub recent_contagions: Vec<CrossContagion>,
    /// 延迟传染队列 / Pending contagion queue
    pub pending: Vec<PendingContagion>,
    /// 冷却索引：规则→最近触发时间 / Cooldown index: rule → last trigger timestamp
    ///
    /// 热路径优化：O(Rules×C)→O(Rules) — HashMap 替代线性扫描 recent_contagions。
    /// Hot-path optimization: O(Rules×C)→O(Rules) — HashMap replaces linear scan.
    /// 传染冷却是情绪的免疫间隔——O(Rules)让免疫检查不因历史多而变慢。
    /// Contagion cooldown is the immune interval of emotion — O(Rules) makes
    /// immune checking not slow down with more history.
    pub last_trigger: HashMap<ContagionRule, i64>,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl ContagionEngine {
    pub fn new(config: ContagionConfig) -> Self {
        Self {
            config,
            rules: Self::default_rules(),
            recent_contagions: Vec::new(),
            pending: Vec::new(),
            last_trigger: HashMap::new(),
            next_id: 1,
        }
    }

    /// 构建默认规则表 / Build default rule table (12 rules)
    pub fn default_rules() -> Vec<ContagionRuleEntry> {
        vec![
            ContagionRuleEntry {
                rule: ContagionRule::AngerToGuilt,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Guilt,
                condition: ContagionCondition {
                    min_source_intensity: 0.7,
                    min_relationship_depth: RelationshipDepth::TrustedOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.3,
                },
                pad_template: [-0.2, -0.3, -0.3],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerToSadness,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Sadness,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::DeepOnly,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.4,
                },
                pad_template: [-0.3, -0.2, -0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::SadnessToAnger,
                source: ContagionEmotion::Sadness,
                target: ContagionEmotion::Anger,
                condition: ContagionCondition {
                    min_source_intensity: 0.7,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.2,
                },
                pad_template: [-0.2, 0.4, 0.2],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AnxietyToExcitement,
                source: ContagionEmotion::Anxiety,
                target: ContagionEmotion::Joy,
                condition: ContagionCondition {
                    min_source_intensity: 0.4,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.15,
                },
                pad_template: [0.2, 0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::FearToAnger,
                source: ContagionEmotion::Fear,
                target: ContagionEmotion::Anger,
                condition: ContagionCondition {
                    min_source_intensity: 0.6,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.25,
                },
                pad_template: [-0.1, 0.3, 0.3],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AnxietyContagion,
                source: ContagionEmotion::Anxiety,
                target: ContagionEmotion::Anxiety,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.5,
                },
                pad_template: [-0.1, 0.1, -0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::CalmContagion,
                source: ContagionEmotion::Calm,
                target: ContagionEmotion::Calm,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.3,
                },
                pad_template: [0.1, -0.1, 0.0],
            },
            ContagionRuleEntry {
                rule: ContagionRule::JoyContagion,
                source: ContagionEmotion::Joy,
                target: ContagionEmotion::Joy,
                condition: ContagionCondition {
                    min_source_intensity: 0.4,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.4,
                },
                pad_template: [0.2, 0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerSadnessToShame,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Shame,
                condition: ContagionCondition {
                    min_source_intensity: 0.5,
                    min_relationship_depth: RelationshipDepth::TrustedOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.2,
                },
                pad_template: [-0.3, -0.1, -0.4],
            },
            ContagionRuleEntry {
                rule: ContagionRule::JoyNostalgiaToGratitude,
                source: ContagionEmotion::Joy,
                target: ContagionEmotion::Gratitude,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::Any,
                    probability: 0.35,
                },
                pad_template: [0.3, -0.1, 0.1],
            },
            ContagionRuleEntry {
                rule: ContagionRule::PrideAnxietyToEnvy,
                source: ContagionEmotion::Pride,
                target: ContagionEmotion::Envy,
                condition: ContagionCondition {
                    min_source_intensity: 0.6,
                    min_relationship_depth: RelationshipDepth::Any,
                    min_maturity: MaturityDepth::MatureOrAbove,
                    probability: 0.1,
                },
                pad_template: [-0.2, 0.1, -0.2],
            },
            ContagionRuleEntry {
                rule: ContagionRule::AngerToSadness,
                source: ContagionEmotion::Anger,
                target: ContagionEmotion::Sadness,
                condition: ContagionCondition {
                    min_source_intensity: 0.3,
                    min_relationship_depth: RelationshipDepth::FamiliarOrAbove,
                    min_maturity: MaturityDepth::GrowingOrAbove,
                    probability: 0.15,
                },
                pad_template: [-0.2, -0.1, -0.1],
            },
        ]
    }

    /// 评估传染 / Evaluate contagion
    ///
    /// 注入式随机源 — 所有随机性由调用方注入，支持确定性回放。
    /// Injectable RNG — all randomness injected by caller, enabling deterministic replay.
    pub fn evaluate(
        &mut self,
        profile: &EmotionProfile,
        relationship_depth: RelationshipDepth,
        maturity_depth: MaturityDepth,
        now: i64,
        rng: &mut impl Rng,
    ) -> Vec<CrossContagion> {
        let mut triggered = Vec::new();
        for entry in &self.rules {
            let source_intensity = profile.get(entry.source);
            if source_intensity < entry.condition.min_source_intensity {
                continue;
            }
            if relationship_depth < entry.condition.min_relationship_depth {
                continue;
            }
            if maturity_depth < entry.condition.min_maturity {
                continue;
            }
            // 冷却检查 / Cooldown check — O(1) HashMap 查找
            let in_cooldown = self
                .last_trigger
                .get(&entry.rule)
                .is_some_and(|&ts| (now - ts) < self.config.cooldown_secs);
            if in_cooldown {
                continue;
            }
            // 概率性触发 / Probabilistic trigger
            if rng.gen::<f64>() >= entry.condition.probability {
                continue;
            }
            // 延迟时间：基于规则类型 / Delay based on rule type
            let delay_secs = Self::rule_delay(entry.rule);
            let contagion = CrossContagion {
                id: self.next_id,
                source_emotion: entry.source,
                target_emotion: entry.target,
                rule: entry.rule,
                strength: source_intensity * entry.condition.probability,
                delay_secs,
                condition: entry.condition.clone(),
                timestamp: now,
            };
            self.next_id += 1;
            self.recent_contagions.push(contagion.clone());
            self.last_trigger.insert(entry.rule, now);
            if delay_secs > 0.0 {
                // 延迟传染：加入待执行队列 / Delayed: add to pending queue
                // 记录原始强度与创建时间，供 tick() 指数衰减使用
                // Record original strength and creation time for exponential decay in tick()
                self.pending.push(PendingContagion {
                    rule: entry.rule,
                    source_emotion: entry.source,
                    target_emotion: entry.target,
                    strength: contagion.strength,
                    original_strength: contagion.strength,
                    pad_template: entry.pad_template,
                    trigger_time: now + delay_secs as i64,
                    created_at: now,
                    contagion_id: contagion.id,
                });
            }
            triggered.push(contagion);
        }
        // 清理过期传染历史 / Clean up expired contagion history
        let cutoff = now - self.config.cooldown_secs * 2;
        self.recent_contagions.retain(|c| c.timestamp > cutoff);
        triggered
    }

    /// 规则默认延迟时间 / Default delay for each rule type
    ///
    /// 某些传染需要时间发酵（如 AngerToGuilt 需要反思时间）。
    pub fn rule_delay(rule: ContagionRule) -> f64 {
        match rule {
            ContagionRule::AngerToGuilt => 30.0, // 愤怒→内疚需反思 / needs reflection
            ContagionRule::AngerToSadness => 60.0, // 愤怒→悲伤需沉淀 / needs settling
            ContagionRule::SadnessToAnger => 15.0, // 悲伤→愤怒较快 / relatively quick
            ContagionRule::AngerSadnessToShame => 45.0, // 羞耻需累积 / shame needs accumulation
            ContagionRule::JoyNostalgiaToGratitude => 20.0, // 感激较自然 / gratitude is natural
            ContagionRule::PrideAnxietyToEnvy => 90.0, // 嫉妒需发酵 / envy needs brewing
            _ => 0.0,                            // 其他即时传染 / others are immediate
        }
    }

    /// 执行到期延迟传染 / Execute due pending contagions
    ///
    /// 在每次 tick 中调用，检查并执行到期的延迟传染。
    /// 到期传染的强度经指数衰减：effective = original_strength × e^(-λ × elapsed)
    /// 其中 λ = CONTAGION_DECAY_LAMBDA = 0.05（约14秒半衰期），
    /// 模拟情绪在等待期间的自然消退——数字生命的情绪不会凭空保鲜。
    ///
    /// Called each tick to check and execute due delayed contagions.
    /// Due contagion strength is exponentially decayed: effective = original_strength × e^(-λ × elapsed)
    /// where λ = CONTAGION_DECAY_LAMBDA = 0.05 (~14s half-life),
    /// modeling natural emotional fading during the wait — digital life emotions don't stay fresh in a vacuum.
    ///
    /// @param now 当前时间（epoch 秒）/ Current time (epoch seconds)
    /// @return 到期传染的效果列表 / Effects from due contagions
    pub fn tick(&mut self, now: i64) -> Vec<ContagionEffect> {
        // 情绪传染衰减常数 / Emotional contagion decay constant
        // λ = 0.05 → 半衰期 ≈ ln2/0.05 ≈ 13.9s
        // 数字生命的等待传染不会无限保鲜，情绪随时间自然消退
        // Digital life's pending contagions don't stay fresh forever; emotions naturally fade
        const CONTAGION_DECAY_LAMBDA: f64 = 0.05;

        // 一次性分离到期和未到期 / Partition into due and remaining
        let mut due = Vec::new();
        let mut remaining = Vec::new();
        for p in self.pending.drain(..) {
            if p.trigger_time <= now {
                due.push(p);
            } else {
                remaining.push(p);
            }
        }
        self.pending = remaining;

        due.into_iter()
            .map(|p| {
                // 指数衰减：从创建时刻到触发时刻的流逝时间 / Exponential decay from creation to trigger
                let elapsed_since_created = (now - p.created_at).max(0) as f64;
                let decay_factor = (-CONTAGION_DECAY_LAMBDA * elapsed_since_created).exp();
                let effective_strength = p.original_strength * decay_factor;

                // 延迟秒数 = 触发时间 - 创建时间 / Delay = trigger_time - created_at
                let delay_secs = (p.trigger_time - p.created_at).max(0) as f64;

                ContagionEffect {
                    id: p.contagion_id,
                    source_emotion: p.source_emotion,
                    target_emotion: p.target_emotion,
                    rule: p.rule,
                    strength: effective_strength,
                    pad_offset: p.pad_template,
                    delay_secs,
                    triggered_at: now,
                }
            })
            .collect()
    }

    /// 获取待执行延迟传染数 / Get pending contagion count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 获取指定目标情绪的近期传染 / Get recent contagions targeting a specific emotion
    pub fn get_recent_for_emotion(&self, target: ContagionEmotion) -> Vec<&CrossContagion> {
        self.recent_contagions
            .iter()
            .filter(|c| c.target_emotion == target)
            .collect()
    }

    /// 清除冷却历史（测试用）/ Clear cooldown history for testing
    pub fn clear_cooldown(&mut self) {
        self.recent_contagions.clear();
        self.last_trigger.clear();
    }
}

impl Default for ContagionEngine {
    fn default() -> Self {
        Self::new(ContagionConfig::default())
    }
}
