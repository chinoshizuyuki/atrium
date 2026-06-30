// SPDX-License-Identifier: MIT
//! EmpathyEngine — 共情推理引擎
//! EmpathyEngine — Cognitive empathy reasoning.
//!
//! 超越简单的 15% 情绪传染，实现认知共情——
//! "如果我处在对方的处境中会怎样"。
//! Goes beyond simple 15% sentiment contagion to achieve genuine perspective-taking —
//! "I understand you're in a tough spot right now."
//!
//! 纯规则实现，零 LLM 调用，延迟 <1μs。
//! Rule-based implementation (no LLM calls), latency <1μs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ════════════════════════════════════════════════════════════════════
// 配置
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, Deserialize)]
pub struct EmpathyCfg {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 共情推断权重（0=无共情, 1=完全共情）
    #[serde(default = "default_empathy_weight")]
    pub empathy_weight: f32,
    /// 同类事件冷却消息数（避免反复触发）
    #[serde(default = "default_cooldown")]
    pub cooldown_messages: u64,
}

fn default_true() -> bool {
    true
}
fn default_empathy_weight() -> f32 {
    0.6
}
fn default_cooldown() -> u64 {
    3
}

impl Default for EmpathyCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            empathy_weight: default_empathy_weight(),
            cooldown_messages: default_cooldown(),
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 事件类型
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeEventType {
    /// 失去：失业/失恋/被裁
    Loss,
    /// 生病/受伤
    Illness,
    /// 失败：考试失利/项目失败
    Failure,
    /// 成就：升职/获奖/考上
    Achievement,
    /// 转变：搬家/换工作/毕业
    Transition,
    /// 冲突：争吵/冷战/被误解
    Conflict,
    /// 悲伤：丧亲/重大失去
    Grief,
    /// 日常压力：加班/堵车/失眠
    DailyStress,
}

impl LifeEventType {
    /// 事件基础情感强度 (0..1)
    fn base_intensity(&self) -> f32 {
        match self {
            Self::Grief => 0.95,
            Self::Loss => 0.80,
            Self::Illness => 0.60,
            Self::Conflict => 0.55,
            Self::Failure => 0.50,
            Self::Transition => 0.35,
            Self::Achievement => 0.65,
            Self::DailyStress => 0.25,
        }
    }

    /// 事件 valence 方向: 负=消极, 正=积极
    fn base_valence(&self) -> f32 {
        match self {
            Self::Grief => -0.9,
            Self::Loss => -0.7,
            Self::Illness => -0.5,
            Self::Conflict => -0.5,
            Self::Failure => -0.4,
            Self::DailyStress => -0.2,
            Self::Transition => 0.0, // 中性偏复杂
            Self::Achievement => 0.7,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 共情回应策略
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmpathyStrategy {
    /// 安静陪伴 — 适用于 grief/loss，不过度追问
    QuietCompany,
    /// 情感支持 — 适用于 illness/conflict
    EmotionalSupport,
    /// 鼓励 — 适用于 failure/daily_stress
    Encouragement,
    /// 庆祝 — 适用于 achievement
    Celebration,
    /// 实际帮助 — 适用于 transition
    PracticalHelp,
    /// 认可感受 — 通用兜底
    Validation,
}

impl EmpathyStrategy {
    /// 根据事件类型选择默认策略
    fn for_event(event: &LifeEventType) -> Self {
        match event {
            LifeEventType::Grief => Self::QuietCompany,
            LifeEventType::Loss => Self::QuietCompany,
            LifeEventType::Illness => Self::EmotionalSupport,
            LifeEventType::Conflict => Self::EmotionalSupport,
            LifeEventType::Failure => Self::Encouragement,
            LifeEventType::DailyStress => Self::Encouragement,
            LifeEventType::Achievement => Self::Celebration,
            LifeEventType::Transition => Self::PracticalHelp,
        }
    }

    /// 策略的自然语言指引（注入 LLM prompt）
    fn guidance_text(&self) -> &'static str {
        match self {
 Self::QuietCompany => "此刻不需要说太多大道理，安静地陪在对方身边，让对方感受到你在就好。避免说\"会好的\"之类空洞安慰。",
 Self::EmotionalSupport => "表达关心和理解，承认对方的感受是合理的。可以说\"我理解你的感受\"、\"你不必一个人扛\"。",
 Self::Encouragement => "肯定对方的努力和付出，传递信心。注意先认可情绪再鼓励，不要一上来就\"加油\"。",
 Self::Celebration => "真心为对方高兴！表达兴奋和祝贺，可以问问细节表示感兴趣。",
 Self::PracticalHelp => "提供具体的帮助建议，而非泛泛的\"有什么需要尽管说\"。",
 Self::Validation => "认可对方的感受，不评判不否定。\"你的感受很正常\"、\"换做谁都会这样\"。",
 }
    }
}

// ════════════════════════════════════════════════════════════════════
// 推断感受
// ════════════════════════════════════════════════════════════════════

/// 每种事件类型对应的推断感受词
fn inferred_feelings_for(event: &LifeEventType) -> Vec<&'static str> {
    match event {
        LifeEventType::Grief => vec!["悲痛", "空虚", "不舍"],
        LifeEventType::Loss => vec!["难过", "迷茫", "不安"],
        LifeEventType::Illness => vec!["难受", "疲惫", "担心"],
        LifeEventType::Conflict => vec!["委屈", "愤怒", "无助"],
        LifeEventType::Failure => vec!["沮丧", "自责", "不甘"],
        LifeEventType::DailyStress => vec!["疲惫", "烦躁", "无奈"],
        LifeEventType::Achievement => vec!["开心", "自豪", "兴奋"],
        LifeEventType::Transition => vec!["期待", "忐忑", "不舍"],
    }
}

// ════════════════════════════════════════════════════════════════════
// 事件检测器
// ════════════════════════════════════════════════════════════════════

struct EventDetector {
    event_type: LifeEventType,
    /// (关键词, 权重) — 多关键词命中时权重累加
    keywords: Vec<(&'static str, f32)>,
}

/// 置信度阈值——低于此值不认为检测到事件
const CONFIDENCE_THRESHOLD: f32 = 0.4;

fn build_detectors() -> Vec<EventDetector> {
    vec![
        EventDetector {
            event_type: LifeEventType::Grief,
            keywords: vec![
                ("去世", 0.5),
                ("离世", 0.5),
                ("过世", 0.5),
                ("走了", 0.3),
                ("丧", 0.4),
                ("追悼", 0.5),
                ("葬礼", 0.5),
                ("亲人", 0.2),
                ("永远离开", 0.5),
                ("再也见不到", 0.5),
                ("不在了", 0.3),
                ("天堂", 0.3),
                ("追思", 0.4),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Loss,
            keywords: vec![
                ("裁员", 0.5),
                ("失业", 0.5),
                ("被辞", 0.5),
                ("开除", 0.5),
                ("分手", 0.5),
                ("离婚", 0.5),
                ("失恋", 0.5),
                ("丢了", 0.2),
                ("失去", 0.3),
                ("没保住", 0.3),
                ("被甩", 0.4),
                ("下岗", 0.4),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Illness,
            keywords: vec![
                ("生病", 0.4),
                ("感冒", 0.3),
                ("发烧", 0.3),
                ("住院", 0.5),
                ("手术", 0.5),
                ("受伤", 0.4),
                ("骨折", 0.4),
                ("头疼", 0.2),
                ("胃疼", 0.2),
                ("不舒服", 0.2),
                ("难受", 0.2),
                ("去医院", 0.3),
                ("吃药", 0.2),
                ("过敏", 0.3),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Failure,
            keywords: vec![
                ("考砸", 0.5),
                ("挂科", 0.5),
                ("没考上", 0.5),
                ("落榜", 0.5),
                ("失败", 0.3),
                ("搞砸", 0.4),
                ("出错了", 0.3),
                ("没通过", 0.4),
                ("被拒", 0.4),
                ("不及格", 0.4),
                ("失利", 0.4),
                ("泡汤", 0.3),
                ("白费", 0.3),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Achievement,
            keywords: vec![
                ("考上", 0.5),
                ("录取", 0.5),
                ("升职", 0.5),
                ("加薪", 0.4),
                ("获奖", 0.5),
                ("得奖", 0.5),
                ("第一名", 0.5),
                ("冠军", 0.5),
                ("通过", 0.3),
                ("成功了", 0.4),
                ("做到了", 0.3),
                ("offer", 0.4),
                ("拿到", 0.2),
                ("被选", 0.3),
                ("恭喜", 0.3),
                ("庆祝", 0.3),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Transition,
            keywords: vec![
                ("搬家", 0.4),
                ("换工作", 0.4),
                ("跳槽", 0.4),
                ("毕业", 0.4),
                ("新城市", 0.4),
                ("新公司", 0.3),
                ("出国", 0.4),
                ("入职", 0.3),
                ("离职", 0.3),
                ("转学", 0.3),
                ("新环境", 0.3),
                ("重新开始", 0.3),
            ],
        },
        EventDetector {
            event_type: LifeEventType::Conflict,
            keywords: vec![
                ("吵架", 0.5),
                ("吵了一架", 0.5),
                ("冷战", 0.4),
                ("闹翻", 0.5),
                ("被误解", 0.4),
                ("误会", 0.3),
                ("生气", 0.2),
                ("不和", 0.3),
                ("矛盾", 0.3),
                ("闹别扭", 0.4),
                ("翻脸", 0.4),
                ("绝交", 0.5),
                ("被骂", 0.4),
                ("委屈", 0.3),
                ("欺负", 0.4),
            ],
        },
        EventDetector {
            event_type: LifeEventType::DailyStress,
            keywords: vec![
                ("加班", 0.4),
                ("熬夜", 0.3),
                ("失眠", 0.3),
                ("堵车", 0.3),
                ("累", 0.2),
                ("好烦", 0.3),
                ("烦死了", 0.3),
                ("压力大", 0.4),
                ("忙死了", 0.3),
                ("赶 deadline", 0.4),
                ("睡不好", 0.3),
                ("头疼", 0.2),
                ("挤地铁", 0.2),
                ("996", 0.4),
                ("内卷", 0.2),
            ],
        },
    ]
}

/// 检测消息中的生活事件，返回 (事件类型, 置信度)
fn detect_event(message: &str, detectors: &[EventDetector]) -> Option<(LifeEventType, f32)> {
    let mut best: Option<(LifeEventType, f32)> = None;

    for detector in detectors {
        let mut score: f32 = 0.0;
        let mut hits = 0u32;

        for &(keyword, weight) in &detector.keywords {
            if message.contains(keyword) {
                score += weight;
                hits += 1;
            }
        }

        // 多关键词命中奖励
        if hits >= 2 {
            score *= 1.2;
        }

        // 归一化到 [0, 1]：以 0.8 作为满分参考
        let confidence = (score / 0.8).min(1.0);

        if confidence >= CONFIDENCE_THRESHOLD {
            let dominated = match best.as_ref() {
                Some((_, best_c)) => confidence > *best_c,
                None => true,
            };
            if dominated {
                best = Some((detector.event_type.clone(), confidence));
            }
        }
    }

    best
}

// ════════════════════════════════════════════════════════════════════
// 关系阶段共情强度
// ════════════════════════════════════════════════════════════════════

/// 关系阶段名称 → 共情强度乘数
/// 越深的关系，共情共鸣越强
fn relationship_empathy_multiplier(stage_name: &str) -> f32 {
    match stage_name {
        "初识" => 0.6,
        "熟悉" => 0.85,
        "信任" => 1.1,
        "深度" => 1.4,
        _ => 0.8, // fallback
    }
}

// ════════════════════════════════════════════════════════════════════
// 共情分析结果
// ════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
pub struct EmpathyResult {
    pub event_type: LifeEventType,
    pub confidence: f32,
    pub inferred_feelings: Vec<String>,
    pub strategy: EmpathyStrategy,
    /// PAD delta 用于注入情感引擎 (pleasure, arousal, dominance)
    pub pad_delta: (f32, f32, f32),
    /// 注入 LLM prompt 的回应指引
    pub response_guidance: String,
}

// ════════════════════════════════════════════════════════════════════
// EmpathyEngine 主结构
// ════════════════════════════════════════════════════════════════════

pub struct EmpathyEngine {
    config: EmpathyCfg,
    detectors: Vec<EventDetector>,
    /// (事件类型, 消息序号) — 用于冷却判断
    last_event: HashMap<LifeEventType, u64>,
    /// 最近一次分析结果（供 prompt_fragment 读取）
    last_result: Option<EmpathyResult>,
}

impl EmpathyEngine {
    pub fn new(config: EmpathyCfg) -> Self {
        Self {
            config,
            detectors: build_detectors(),
            last_event: HashMap::new(),
            last_result: None,
        }
    }

    /// 分析用户消息，尝试识别生活事件并生成共情推理结果。
    ///
    /// - `message`: 用户消息文本
    /// - `relationship_stage_name`: 关系阶段名称（"初识"/"熟悉"/"信任"/"深度"）
    /// - `msg_index`: 当前消息序号（用于冷却判断）
    pub fn analyze(
        &mut self,
        message: &str,
        relationship_stage_name: &str,
        msg_index: u64,
    ) -> Option<EmpathyResult> {
        if !self.config.enabled {
            return None;
        }

        // 1. 事件检测
        let (event_type, confidence) = detect_event(message, &self.detectors)?;

        // 2. 冷却检查
        if let Some(&last_idx) = self.last_event.get(&event_type) {
            if msg_index.saturating_sub(last_idx) < self.config.cooldown_messages {
                return None;
            }
        }

        // 3. 记录事件时间
        self.last_event.insert(event_type.clone(), msg_index);

        // 4. 关系阶段调制
        let rel_mult = relationship_empathy_multiplier(relationship_stage_name);
        let intensity = event_type.base_intensity() * self.config.empathy_weight * rel_mult;

        // 5. 计算 PAD delta
        let valence = event_type.base_valence() * intensity;
        let arousal = match event_type {
            LifeEventType::Achievement => 0.3 * intensity,
            LifeEventType::Grief => -0.2 * intensity, // 悲伤时 arousal 偏低
            LifeEventType::Conflict => 0.4 * intensity, // 冲突时 arousal 偏高
            LifeEventType::DailyStress => 0.1 * intensity,
            _ => 0.15 * intensity,
        };
        let dominance = match event_type {
            LifeEventType::Achievement => 0.3 * intensity,
            LifeEventType::Grief | LifeEventType::Loss => -0.3 * intensity,
            LifeEventType::Failure => -0.2 * intensity,
            _ => -0.1 * intensity,
        };

        // 6. 选择回应策略
        let strategy = EmpathyStrategy::for_event(&event_type);

        // 7. 推断感受
        let inferred: Vec<String> = inferred_feelings_for(&event_type)
            .iter()
            .map(|s| s.to_string())
            .collect();

        // 8. 生成回应指引
        let guidance = format!(
            "[共情推理] 检测到「{}」事件（置信度 {:.0}%）。对方可能感到{}。{}",
            Self::event_type_label(&event_type),
            confidence * 100.0,
            inferred.join("、"),
            strategy.guidance_text(),
        );

        let result = EmpathyResult {
            event_type,
            confidence,
            inferred_feelings: inferred,
            strategy,
            pad_delta: (valence, arousal, dominance),
            response_guidance: guidance,
        };

        self.last_result = Some(result.clone());
        Some(result)
    }

    /// 获取最近一次共情结果的 prompt fragment
    pub fn prompt_fragment(&self) -> String {
        self.last_result
            .as_ref()
            .map(|r| r.response_guidance.clone())
            .unwrap_or_default()
    }

    /// 清除上一次结果（消息处理完毕后调用）
    pub fn clear_last_result(&mut self) {
        self.last_result = None;
    }

    /// 当前健康状态
    pub fn health_status(&self) -> String {
        format!(
            "empathy: enabled={}, cooldown={}, tracked_events={}",
            self.config.enabled,
            self.config.cooldown_messages,
            self.last_event.len()
        )
    }

    fn event_type_label(event: &LifeEventType) -> &'static str {
        match event {
            LifeEventType::Grief => "重大悲伤",
            LifeEventType::Loss => "失去",
            LifeEventType::Illness => "生病",
            LifeEventType::Conflict => "冲突",
            LifeEventType::Failure => "挫折",
            LifeEventType::DailyStress => "日常压力",
            LifeEventType::Achievement => "成就",
            LifeEventType::Transition => "人生转变",
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> EmpathyEngine {
        EmpathyEngine::new(EmpathyCfg::default())
    }

    #[test]
    fn test_detect_loss_event() {
        let mut engine = make_engine();
        let result = engine.analyze("我被公司裁员了，突然就失业了", "初识", 1);
        assert!(result.is_some(), "应检测到 Loss 事件");
        let r = result.unwrap();
        assert_eq!(r.event_type, LifeEventType::Loss);
        assert!(r.confidence >= CONFIDENCE_THRESHOLD);
        assert!(r.confidence > 0.5, "双关键词命中应有高置信度");
    }

    #[test]
    fn test_detect_achievement_event() {
        let mut engine = make_engine();
        let result = engine.analyze("我考上研究生了！终于拿到录取通知了！", "熟悉", 1);
        assert!(result.is_some(), "应检测到 Achievement 事件");
        let r = result.unwrap();
        assert_eq!(r.event_type, LifeEventType::Achievement);
        assert_eq!(r.strategy, EmpathyStrategy::Celebration);
    }

    #[test]
    fn test_detect_daily_stress() {
        let mut engine = make_engine();
        let result = engine.analyze("今天加班到好晚，压力大好累", "信任", 1);
        assert!(result.is_some(), "应检测到 DailyStress 事件");
        let r = result.unwrap();
        assert_eq!(r.event_type, LifeEventType::DailyStress);
        assert_eq!(r.strategy, EmpathyStrategy::Encouragement);
    }

    #[test]
    fn test_empathy_strategy_mapping() {
        // Grief → QuietCompany
        let mut engine = make_engine();
        let r = engine
            .analyze("奶奶今天去世了，再也见不到她了", "深度", 1)
            .unwrap();
        assert_eq!(r.event_type, LifeEventType::Grief);
        assert_eq!(r.strategy, EmpathyStrategy::QuietCompany);

        // Achievement → Celebration
        engine.last_event.clear();
        let r = engine.analyze("我得奖了！第一名！", "信任", 2).unwrap();
        assert_eq!(r.strategy, EmpathyStrategy::Celebration);

        // Failure → Encouragement
        engine.last_event.clear();
        let r = engine.analyze("考试考砸了，没通过", "熟悉", 3).unwrap();
        assert_eq!(r.event_type, LifeEventType::Failure);
        assert_eq!(r.strategy, EmpathyStrategy::Encouragement);
    }

    #[test]
    fn test_relationship_modulates_intensity() {
        let msg = "我被公司裁员了";

        let mut engine = make_engine();
        let r_acquaintance = engine.analyze(msg, "初识", 1).unwrap();

        engine.last_event.clear();
        let r_deep = engine.analyze(msg, "深度", 2).unwrap();

        // 深度关系的 PAD delta 绝对值应更大
        let abs_pleasure_acq = r_acquaintance.pad_delta.0.abs();
        let abs_pleasure_deep = r_deep.pad_delta.0.abs();
        assert!(
            abs_pleasure_deep > abs_pleasure_acq,
            "深度关系共情应更强: deep={}, acq={}",
            abs_pleasure_deep,
            abs_pleasure_acq
        );
    }

    #[test]
    fn test_cooldown_prevents_retrigger() {
        let mut engine = make_engine();

        // 第一次检测
        let r1 = engine.analyze("加班好累", "熟悉", 1);
        assert!(r1.is_some());

        // 冷却期内再次提到 → 应被抑制
        let r2 = engine.analyze("又加班了", "熟悉", 2);
        assert!(r2.is_none(), "冷却期内同类型事件不应重复触发");

        // 冷却期过后 → 可以再次触发
        let r3 = engine.analyze("加班太多了", "熟悉", 100);
        assert!(r3.is_some(), "冷却期过后应重新触发");
    }

    #[test]
    fn test_no_event_for_normal_message() {
        let mut engine = make_engine();
        let r = engine.analyze("今天天气不错，出去散步了", "熟悉", 1);
        assert!(r.is_none(), "普通消息不应触发事件");
    }

    #[test]
    fn test_prompt_fragment_contains_guidance() {
        let mut engine = make_engine();
        engine.analyze("我和朋友吵架了", "信任", 1);
        let fragment = engine.prompt_fragment();
        assert!(!fragment.is_empty());
        assert!(fragment.contains("共情推理"));
        assert!(fragment.contains("冲突"));
    }

    #[test]
    fn test_disabled_engine_returns_none() {
        let mut engine = EmpathyEngine::new(EmpathyCfg {
            enabled: false,
            empathy_weight: 0.6,
            cooldown_messages: 3,
        });
        let r = engine.analyze("我被裁员了", "熟悉", 1);
        assert!(r.is_none(), "禁用时不应返回结果");
    }

    #[test]
    fn test_pad_delta_direction() {
        let mut engine = make_engine();

        // 悲伤事件 → pleasure 应为负
        let r = engine.analyze("奶奶去世了", "信任", 1).unwrap();
        assert!(r.pad_delta.0 < 0.0, "Grief 应产生负 pleasure");

        // 成就事件 → pleasure 应为正
        engine.last_event.clear();
        let r = engine
            .analyze("考上研究生了！拿到录取通知！", "信任", 2)
            .unwrap();
        assert!(r.pad_delta.0 > 0.0, "Achievement 应产生正 pleasure");
    }
}
