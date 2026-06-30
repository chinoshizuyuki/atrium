// SPDX-License-Identifier: MIT
//! 条件行为规则系统 — 确定性条件匹配
//! RuleEngine — Deterministic conditional behavior system.
//!
//! 触发条件 / Trigger types: time_range, keyword, emotion, message_count, idle, compound
//! 规则引擎运行在 Rust 侧，与 LLM 无关，确保关键行为永远正确执行。
//! Runs entirely in Rust, LLM-independent, ensuring critical behaviors always execute correctly.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerCondition {
    TimeRange {
        start: String,
        end: String,
    },
    Keyword {
        words: Vec<String>,
    },
    Emotion {
        min_pleasure: f32,
        max_pleasure: f32,
        min_arousal: f32,
        max_arousal: f32,
    },
    MessageCount {
        threshold: u64,
    },
    Idle {
        seconds: u64,
    },
    Compound {
        op: LogicOp,
        conditions: Vec<TriggerCondition>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleAction {
    Notify {
        message: String,
    },
    SetEmotion {
        pleasure: f32,
        arousal: f32,
        dominance: f32,
    },
    ActivatePersona {
        name: String,
    },
    SetTemperature {
        value: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorRule {
    pub name: String,
    pub enabled: bool,
    pub priority: u32,
    pub condition: TriggerCondition,
    pub action: RuleAction,
    pub cooldown_secs: u64,
    last_triggered: u64,
}

impl BehaviorRule {
    /// 构造新规则（last_triggered 初始化为 0）
    pub fn new(
        name: String,
        priority: u32,
        cooldown_secs: u64,
        condition: TriggerCondition,
        action: RuleAction,
    ) -> Self {
        Self {
            name,
            enabled: true,
            priority,
            cooldown_secs,
            condition,
            action,
            last_triggered: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleContext {
    pub current_time: String,
    pub last_message: String,
    pub emotion_pleasure: f32,
    pub emotion_arousal: f32,
    pub emotion_dominance: f32,
    pub message_count: u64,
    pub idle_seconds: u64,
    pub extra: HashMap<String, String>,
}

impl Default for RuleContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleContext {
    pub fn new() -> Self {
        let now = chrono::Local::now();
        Self {
            current_time: now.format("%H:%M").to_string(),
            last_message: String::new(),
            emotion_pleasure: 0.0,
            emotion_arousal: 0.0,
            emotion_dominance: 0.0,
            message_count: 0,
            idle_seconds: 0,
            extra: HashMap::new(),
        }
    }
}

pub struct RuleEngine {
    rules: Vec<BehaviorRule>,
    fired_count: u64,
    store: Option<sled::Db>,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            fired_count: 0,
            store: None,
        }
    }

    /// 打开 sled 持久化规则引擎
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let mut engine = Self {
            rules: Vec::new(),
            fired_count: 0,
            store: Some(db),
        };
        engine.load();
        Ok(engine)
    }

    /// 内存模式（测试用）
    pub fn open_in_memory() -> Self {
        let db = sled::Config::new().temporary(true).open().ok();
        Self {
            rules: Vec::new(),
            fired_count: 0,
            store: db,
        }
    }

    /// 保存用户创建的规则到 sled
    fn save(&self) {
        if let Some(ref db) = self.store {
            // 只保存 priority >= 10 的用户规则（内置规则每次启动重新注册）
            let user_rules: Vec<&BehaviorRule> =
                self.rules.iter().filter(|r| r.priority >= 10).collect();
            if let Ok(data) = serde_json::to_vec(&user_rules) {
                let _ = db.insert("user_rules", data);
                let _ = db.flush();
            }
        }
    }

    /// 从 sled 加载用户规则
    fn load(&mut self) {
        if let Some(ref db) = self.store {
            if let Ok(Some(data)) = db.get("user_rules") {
                if let Ok(user_rules) = serde_json::from_slice::<Vec<BehaviorRule>>(&data) {
                    for rule in user_rules {
                        self.rules.push(rule);
                    }
                    self.rules.sort_by_key(|r| r.priority);
                    tracing::info!("RuleEngine: 从 sled 加载 {} 条用户规则", self.rules.len());
                }
            }
        }
    }

    pub fn register_defaults(&mut self) {
        self.add(BehaviorRule {
            name: "深夜提醒".into(),
            enabled: true,
            priority: 1,
            cooldown_secs: 3600,
            condition: TriggerCondition::TimeRange {
                start: "02:00".into(),
                end: "06:00".into(),
            },
            action: RuleAction::Notify {
                message: "主人，很晚了，该休息了哦~ 🌙".into(),
            },
            last_triggered: 0,
        });
        self.add(BehaviorRule {
            name: "考试鼓励".into(),
            enabled: true,
            priority: 2,
            cooldown_secs: 600,
            condition: TriggerCondition::Keyword {
                words: vec!["考试".into(), "复习".into(), "备考".into()],
            },
            action: RuleAction::Notify {
                message: "主人加油！考试一定没问题的！💪".into(),
            },
            last_triggered: 0,
        });
        self.add(BehaviorRule {
            name: "高唤醒抑制".into(),
            enabled: true,
            priority: 3,
            cooldown_secs: 300,
            condition: TriggerCondition::Emotion {
                min_pleasure: -1.0,
                max_pleasure: 1.0,
                min_arousal: 0.7,
                max_arousal: 1.0,
            },
            action: RuleAction::SetTemperature { value: 0.3 },
            last_triggered: 0,
        });
    }

    pub fn add(&mut self, rule: BehaviorRule) {
        let is_user_rule = rule.priority >= 10;
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
        if is_user_rule {
            self.save();
        }
    }

    pub fn evaluate(&mut self, ctx: &RuleContext) -> Vec<RuleAction> {
        let now = now_secs();
        let mut actions = Vec::new();
        for rule in &mut self.rules {
            if !rule.enabled {
                continue;
            }
            if now.saturating_sub(rule.last_triggered) < rule.cooldown_secs {
                continue;
            }
            if !eval_condition(&rule.condition, ctx) {
                continue;
            }
            rule.last_triggered = now;
            actions.push(rule.action.clone());
            self.fired_count += 1;
        }
        actions
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub fn fired_count(&self) -> u64 {
        self.fired_count
    }

    /// 检查是否已存在指定名称的规则
    pub fn has_named_rule(&self, name: &str) -> bool {
        self.rules.iter().any(|r| r.name == name)
    }
}

fn eval_condition(cond: &TriggerCondition, ctx: &RuleContext) -> bool {
    match cond {
        TriggerCondition::TimeRange { start, end } => {
            ctx.current_time >= *start && ctx.current_time <= *end
        }
        TriggerCondition::Keyword { words } => {
            let lower = ctx.last_message.to_lowercase();
            words.iter().any(|w| lower.contains(&w.to_lowercase()))
        }
        TriggerCondition::Emotion {
            min_pleasure,
            max_pleasure,
            min_arousal,
            max_arousal,
        } => {
            ctx.emotion_pleasure >= *min_pleasure
                && ctx.emotion_pleasure <= *max_pleasure
                && ctx.emotion_arousal >= *min_arousal
                && ctx.emotion_arousal <= *max_arousal
        }
        TriggerCondition::MessageCount { threshold } => ctx.message_count >= *threshold,
        TriggerCondition::Idle { seconds } => ctx.idle_seconds >= *seconds,
        TriggerCondition::Compound { op, conditions } => match op {
            LogicOp::And => conditions.iter().all(|c| eval_condition(c, ctx)),
            LogicOp::Or => conditions.iter().any(|c| eval_condition(c, ctx)),
        },
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range() {
        let mut engine = RuleEngine::new();
        engine.register_defaults();
        let mut ctx = RuleContext::new();
        ctx.current_time = "03:00".into();
        let actions = engine.evaluate(&ctx);
        assert!(actions
            .iter()
            .any(|a| matches!(a, RuleAction::Notify { .. })));
    }

    #[test]
    fn test_keyword_match() {
        let mut engine = RuleEngine::new();
        engine.register_defaults();
        let mut ctx = RuleContext::new();
        ctx.last_message = "明天要考试了".into();
        let actions = engine.evaluate(&ctx);
        assert!(actions
            .iter()
            .any(|a| matches!(a, RuleAction::Notify { .. })));
    }

    #[test]
    fn test_keyword_no_match() {
        let mut engine = RuleEngine::new();
        engine.register_defaults();
        let mut ctx = RuleContext::new();
        ctx.last_message = "今天天气真好".into();
        let actions = engine.evaluate(&ctx);
        let has_keyword = actions
            .iter()
            .any(|a| matches!(a, RuleAction::Notify { message } if message.contains("考试")));
        assert!(!has_keyword);
    }

    #[test]
    fn test_emotion_trigger() {
        let mut engine = RuleEngine::new();
        engine.register_defaults();
        let mut ctx = RuleContext::new();
        ctx.emotion_arousal = 0.85;
        let actions = engine.evaluate(&ctx);
        assert!(actions
            .iter()
            .any(|a| matches!(a, RuleAction::SetTemperature { .. })));
    }

    #[test]
    fn test_cooldown() {
        let mut engine = RuleEngine::new();
        engine.register_defaults();
        let mut ctx = RuleContext::new();
        ctx.current_time = "03:00".into();
        let first = engine.evaluate(&ctx);
        assert!(!first.is_empty());
        let second = engine.evaluate(&ctx);
        assert!(second.is_empty());
    }
}
