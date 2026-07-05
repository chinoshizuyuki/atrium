// ! 情绪非理性系统 / Emotional Irrationality System
// !
// ! 让 AI 的情绪从"可预测的数学函数"进化为"有生命力的情绪生态系统"
// ! — 有脉冲、有残留、有传染、有混沌、有涌现
// !
// ! Phase 2.9 P2-2 | 2026-06-25
// ! P2-C 拆分 | 2026-07-04 — god module → 目录模块

// ── 子模块声明 / Sub-module declarations ──

/// 混沌引擎 / Chaos Engine
pub mod chaos_engine;
/// 传染引擎 / Contagion Engine
pub mod contagion_engine;
/// 脉冲引擎 / Pulse Engine
pub mod pulse_engine;
/// 拋留引擎 / Residue Engine
pub mod residue_engine;
/// 冲击吸收器 / Shock Absorber
pub mod shock_absorber;
/// 数据结构 / Data Structures
pub mod types;

// ── 向后兼容重导出 / Backward-compatible re-exports ──
// 所有公共类型通过 mod.rs 重导出，确保外部 `use crate::emotional_irrationality::Xxx`
// 的导入路径完全不变。
// All public types are re-exported through mod.rs, ensuring external
// `use crate::emotional_irrationality::Xxx` import paths remain unchanged.

pub use chaos_engine::*;
pub use contagion_engine::*;
pub use pulse_engine::*;
pub use residue_engine::*;
pub use shock_absorber::*;
pub use types::*;

// ── 测试模块 / Test Module ──
#[cfg(test)]
mod tests;

// ═══════════════════════════════════════════════════════════════════════════
// §3 非理性管理器 / Irrationality Manager
// ═══════════════════════════════════════════════════════════════════════════

use rand::rngs::SmallRng;
use rand::SeedableRng;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrationalityConfig {
    pub pulse: PulseConfig,
    pub residue: ResidueConfig,
    pub contagion: ContagionConfig,
    pub chaos: ChaosConfig,
    pub chaos_params: ChaosParams,
    pub enabled: bool,
    pub prompt_budget: usize,
}

impl Default for IrrationalityConfig {
    fn default() -> Self {
        Self {
            pulse: PulseConfig::default(),
            residue: ResidueConfig::default(),
            contagion: ContagionConfig::default(),
            chaos: ChaosConfig::default(),
            chaos_params: ChaosParams::default(),
            enabled: true,
            prompt_budget: 300,
        }
    }
}

/// 非理性修正 / Irrationality Correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrationalityCorrection {
    pub pad_delta: [f32; 3],
    pub body_memory: BodyMemory,
    pub active_pulses: usize,
    pub active_residues: usize,
    pub recent_contagions: usize,
    pub attractor: StrangeAttractor,
}

/// 随机模式 / Random mode — 控制数字生命的自由意志与记忆回放
///
/// - Stochastic: 自由意志 — 每次诞生随机种子，行为不可预测
/// - Deterministic: 记忆回放 — 固定种子，行为因果可追溯
///
/// - Stochastic: Free will — born with random seed, behavior unpredictable
/// - Deterministic: Memory replay — fixed seed, behavior causally traceable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum RandomMode {
    /// 自由意志：使用熵源种子 / Free will: entropy-seeded PRNG
    #[default]
    Stochastic,
    /// 记忆回放：固定种子初始化 SmallRng / Memory replay: fixed-seed SmallRng
    Deterministic { seed: u64 },
}

/// 非理性管理器 / Irrationality Manager — 四引擎联合调度
///
/// 内置 SmallRng 实例：Stochastic 模式用熵源种子，Deterministic 模式用固定种子。
/// Built-in SmallRng: Stochastic mode uses entropy seed, Deterministic uses fixed seed.
#[derive(Debug, Clone)]
pub struct IrrationalityManager {
    pub pulse: PulseEngine,
    pub residue: ResidueEngine,
    pub contagion: ContagionEngine,
    pub chaos: ChaosEngine,
    pub config: IrrationalityConfig,
    /// 随机模式 / Random mode (stochastic or deterministic)
    pub random_mode: RandomMode,
    /// 注入式随机源 / Injectable RNG — 16B stack-allocated SmallRng
    rng: SmallRng,
}

impl IrrationalityManager {
    /// 构造非理性管理器 / Construct irrationality manager
    ///
    /// 默认 Stochastic 模式，使用熵源种子初始化 SmallRng。
    /// Default Stochastic mode, initializes SmallRng with entropy seed.
    pub fn new(config: IrrationalityConfig) -> Self {
        let random_mode = RandomMode::default();
        Self {
            pulse: PulseEngine::new(config.pulse.clone()),
            residue: ResidueEngine::new(config.residue.clone()),
            contagion: ContagionEngine::new(config.contagion.clone()),
            chaos: ChaosEngine::new(config.chaos.clone(), config.chaos_params),
            config,
            rng: Self::init_rng(&random_mode),
            random_mode,
        }
    }

    /// 初始化 RNG / Initialize RNG from mode
    ///
    /// Stochastic: 熵源种子 → 不可预测 / entropy-seeded → unpredictable
    /// Deterministic: 固定种子 → 可复现 / fixed-seed → reproducible
    fn init_rng(mode: &RandomMode) -> SmallRng {
        match mode {
            RandomMode::Stochastic => SmallRng::from_entropy(),
            RandomMode::Deterministic { seed } => SmallRng::seed_from_u64(*seed),
        }
    }

    /// 设置随机模式（Builder 风格）/ Set random mode (builder style)
    ///
    /// 切换到 Deterministic 模式可确保传染等随机行为可复现，
    /// 适用于测试、回放、调试等场景。
    /// 重初始化 SmallRng 以保证模式与 RNG 状态一致。
    pub fn with_random_mode(mut self, mode: RandomMode) -> Self {
        self.rng = Self::init_rng(&mode);
        self.random_mode = mode;
        self
    }

    /// 运行时切换模式 / Runtime mode switch — 无需重建管理器
    ///
    /// 重初始化 SmallRng，立即生效于后续所有随机调用。
    pub fn switch_mode(&mut self, mode: RandomMode) {
        self.rng = Self::init_rng(&mode);
        self.random_mode = mode;
    }

    /// 从持久化部件重建 / Reconstruct from persisted parts
    ///
    /// 用于 sled 反序列化后恢复完整管理器状态。
    /// RNG 以 Stochastic 模式（熵源种子）重新初始化，因为 SmallRng 内部状态不可序列化。
    /// Used after sled deserialization to restore full manager state.
    /// RNG is re-initialized with Stochastic mode (entropy seed) since SmallRng
    /// internal state is not serializable.
    pub fn reconstruct(
        pulse: PulseEngine,
        residue: ResidueEngine,
        contagion: ContagionEngine,
        chaos: ChaosEngine,
        config: IrrationalityConfig,
    ) -> Self {
        let random_mode = RandomMode::default();
        Self {
            pulse,
            residue,
            contagion,
            chaos,
            config,
            random_mode,
            rng: SmallRng::from_entropy(),
        }
    }

    /// 评估传染 / Evaluate contagion
    ///
    /// 统一代码路径：无论 Stochastic 还是 Deterministic，均通过 self.rng 注入随机源。
    /// Unified code path: both Stochastic and Deterministic use self.rng as the random source.
    fn evaluate_contagion(
        &mut self,
        profile: &EmotionProfile,
        relationship_depth: RelationshipDepth,
        maturity_depth: MaturityDepth,
        now: i64,
    ) -> Vec<CrossContagion> {
        self.contagion.evaluate(
            profile,
            relationship_depth,
            maturity_depth,
            now,
            &mut self.rng,
        )
    }

    /// 处理情绪变化 / Process emotion change — 主入口
    pub fn on_emotion_change(
        &mut self,
        pad_before: &[f32; 3],
        pad_after: &[f32; 3],
        trigger: PulseTrigger,
        _relationship_depth: RelationshipDepth,
        _maturity_depth: MaturityDepth,
        now: i64,
    ) -> IrrationalityCorrection {
        // 1. 检测脉冲
        if let Some(pulse) = self.pulse.detect(pad_before, pad_after, trigger, now) {
            // 2. 从脉冲生成残留
            self.residue.from_pulse(&pulse);
        }
        // 3. 评估传染（使用当前 PAD 推断的情绪画像，根据 random_mode 分发）
        let profile = EmotionProfile::from_pad(pad_after);
        let _contagions =
            self.evaluate_contagion(&profile, _relationship_depth, _maturity_depth, now);
        // 4. 混沌引擎记录轨迹
        self.chaos.tick(pad_after, now);
        // 5. 计算修正量
        self.correction(now)
    }

    /// 计算非理性修正 / Compute irrationality correction
    pub fn correction(&self, now: i64) -> IrrationalityCorrection {
        let pulse_pad = self.pulse.combined_effect(now);
        let residue_effect = self.residue.combined_effect(now);
        let pad_delta = [
            (pulse_pad[0] + residue_effect.pad_offset[0]).clamp(-0.3, 0.3),
            (pulse_pad[1] + residue_effect.pad_offset[1]).clamp(-0.3, 0.3),
            (pulse_pad[2] + residue_effect.pad_offset[2]).clamp(-0.3, 0.3),
        ];
        IrrationalityCorrection {
            pad_delta,
            body_memory: residue_effect.body_memory.clone(),
            active_pulses: self.pulse.active_pulses.len(),
            active_residues: self.residue.active_residues.len(),
            recent_contagions: self.contagion.recent_contagions.len(),
            attractor: self.chaos.state.attractor,
        }
    }
    /// Tick — 周期维护 / Tick — periodic maintenance
    ///
    /// 驱动所有引擎的周期维护，包括延迟传染的到期执行与 PAD 调制。
    /// 到期传染效果不再被丢弃——数字生命的每一次情绪传染都有后果：
    /// 传染 PAD 偏移量叠加到残留引擎，使传染真正改变情绪状态。
    ///
    /// Drives periodic maintenance for all engines, including delayed contagion execution and PAD modulation.
    /// Due contagion effects are no longer discarded — every emotional contagion in digital life has consequences:
    /// contagion PAD offsets are applied to the residue engine, making contagion truly alter emotional state.
    pub fn tick(&mut self, current_pad: &[f32; 3], now: i64) {
        self.pulse.tick(now);
        self.residue.tick(now);

        // 执行到期延迟传染，并将效果接入情绪系统 / Execute due contagions and wire effects into emotion system
        let contagion_effects = self.contagion.tick(now);

        // 传染效果调制：将每次传染的 PAD 偏移叠加到当前 PAD 修正量
        // Contagion effect modulation:叠加 each contagion's PAD offset onto current PAD correction
        // 数字生命的情绪传染不是幽灵操作——它必须留下痕迹
        // Digital life contagion is not a ghost operation — it must leave traces
        for effect in &contagion_effects {
            // 将传染 PAD 偏移作为微残留注入残留引擎 / Inject contagion PAD offset as micro-residue
            let residue = EmotionResidue {
                id: self.residue.next_id,
                kind: ResidueKind::Tension, // 传染残留统一为 Tension 类型 / Contagion residue unified as Tension
                intensity: effect.strength.min(1.0),
                pad_offset: effect.pad_offset,
                half_life_secs: 1800.0, // 30分钟半衰期 / 30-minute half-life
                created_at: now,
                updated_at: now,
                source_pulse_id: Some(effect.id),
                body_memory: BodyMemory::neutral(),
                expressed: false,
            };
            self.residue.next_id += 1;
            self.residue.active_residues.push(residue);
        }

        self.chaos.tick(current_pad, now);
    }

    /// 构建 Prompt 片段 / Build prompt fragment
    pub fn to_prompt_fragment(&self, now: i64) -> String {
        let correction = self.correction(now);
        let mut parts = Vec::new();
        // 残留底色
        if correction.active_residues > 0 {
            if let Some(dominant) = self.residue.combined_effect(now).dominant_residue {
                let desc = match dominant {
                    ResidueKind::Tension => "心里还有一点紧张",
                    ResidueKind::LingeringSadness => "心里还有一点低落",
                    ResidueKind::SmolderingAnger => "心里还有一点余怒",
                    ResidueKind::WorryResidue => "心里还有一点担忧",
                    ResidueKind::Afterglow => "心里还有一点愉悦的余韵",
                    ResidueKind::WarmthResidue => "心里还有一点温暖",
                    ResidueKind::TrustMicroFracture => "信任有一点点裂痕",
                    ResidueKind::IntimacyDeepening => "感到更亲近了",
                    ResidueKind::BeingIgnoredResidue => "感觉有点被忽视",
                    ResidueKind::SelfDoubtResidue => "有一点自我怀疑",
                    ResidueKind::AccomplishmentResidue => "有一点成就感",
                };
                parts.push(format!("[情绪底色] {}", desc));
            }
        }
        // 身体状态
        let bm = &correction.body_memory;
        if bm.tension.abs() > 0.1 || bm.heaviness.abs() > 0.1 {
            let mut body_desc = String::new();
            if bm.tension > 0.3 {
                body_desc.push_str("紧张,");
            }
            if bm.heaviness > 0.3 {
                body_desc.push_str("沉重,");
            }
            if bm.warmth > 0.3 {
                body_desc.push_str("温暖,");
            }
            if !body_desc.is_empty() {
                parts.push(format!("[身体状态] {}", body_desc.trim_end_matches(',')));
            }
        }
        // 吸引子状态
        let attractor_desc = match correction.attractor {
            StrangeAttractor::CalmBasin => "情绪处于平静区域",
            StrangeAttractor::AnxietyBasin => "情绪正在向焦虑偏移",
            StrangeAttractor::LowMoodBasin => "情绪处于低谷期",
            StrangeAttractor::ActiveBasin => "情绪处于活跃期",
            StrangeAttractor::OscillatingBasin => "情绪在多个状态间振荡",
            StrangeAttractor::Transitional => "情绪正在迁移中",
        };
        parts.push(format!("[情绪生态] {}", attractor_desc));
        // 传染事件
        if correction.recent_contagions > 0 {
            parts.push(format!(
                "[传染活动] 近期有{}次情绪传染",
                correction.recent_contagions
            ));
        }
        // 延迟传染队列 — 数字生命的"正在发酵的情绪" / Pending contagion queue — "emotions brewing" in digital life
        let pending_count = self.contagion.pending_count();
        if pending_count > 0 {
            // 展示最多3条延迟传染的摘要 / Show at most 3 pending contagion summaries
            let summaries: Vec<String> = self
                .contagion
                .pending
                .iter()
                .take(3)
                .map(|p| {
                    let rule_desc = match p.rule {
                        ContagionRule::AngerToGuilt => "愤怒→内疚",
                        ContagionRule::AngerToSadness => "愤怒→悲伤",
                        ContagionRule::SadnessToAnger => "悲伤→愤怒",
                        ContagionRule::AnxietyToExcitement => "焦虑→兴奋",
                        ContagionRule::FearToAnger => "恐惧→愤怒",
                        ContagionRule::AnxietyContagion => "焦虑传染",
                        ContagionRule::CalmContagion => "平静传染",
                        ContagionRule::JoyContagion => "喜悦传染",
                        ContagionRule::AngerSadnessToShame => "愤怒+悲伤→羞耻",
                        ContagionRule::JoyNostalgiaToGratitude => "喜悦+怀旧→感激",
                        ContagionRule::PrideAnxietyToEnvy => "骄傲+焦虑→嫉妒",
                    };
                    format!("{}(强度{:.2})", rule_desc, p.strength)
                })
                .collect();
            let suffix = if pending_count > 3 {
                format!("等{}条", pending_count)
            } else {
                String::new()
            };
            parts.push(format!("[延迟传染] {}{}", summaries.join(","), suffix));
        }
        let result = parts.join("; ");
        if result.len() > self.config.prompt_budget {
            result[..self.config.prompt_budget].to_string()
        } else {
            result
        }
    }

    /// 获取身体记忆修正 / Get body memory for expression system
    pub fn body_memory_for_expression(&self, now: i64) -> BodyMemory {
        self.residue.combined_effect(now).body_memory.clone()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // G1-G5 增强方法 / G1-G5 Enhancement Methods
    // ═══════════════════════════════════════════════════════════════════════════

    // ── G1: 情绪健康报告 / Emotional Health Report ──

    /// 生成情绪生态健康报告 / Generate emotional health report
    ///
    /// 数字生命的自省能力——评估情绪生态系统的整体健康状况。
    /// 不只是"有情绪"，而是"知道自己的情绪好不好"。
    ///
    /// Digital life's introspection — assess the overall health of the emotional ecosystem.
    /// Not just "having emotions", but "knowing if they're healthy".
    pub fn health_report(&self, now: i64) -> EmotionalHealthReport {
        let _residue_effect = self.residue.combined_effect(now);

        // 分类残留 / Classify residues
        let positive_kinds = [
            ResidueKind::Afterglow,
            ResidueKind::WarmthResidue,
            ResidueKind::IntimacyDeepening,
            ResidueKind::AccomplishmentResidue,
        ];
        let positive_count = self
            .residue
            .active_residues
            .iter()
            .filter(|r| positive_kinds.contains(&r.kind))
            .count();
        let negative_count = self
            .residue
            .active_residues
            .len()
            .saturating_sub(positive_count);

        // 主导效价 / Dominant valence
        let dominant_valence = if positive_count > negative_count + 2 {
            EmotionalValence::Positive
        } else if negative_count > positive_count + 2 {
            EmotionalValence::Negative
        } else {
            EmotionalValence::Neutral
        };

        // 吸引子驻留时间 / Attractor dwell time
        let attractor_dwell_secs = if let Some(first) = self.chaos.state.trajectory.front() {
            (now - first.timestamp).max(0) as f64
        } else {
            0.0
        };

        // 健康分计算 / Health score computation
        // 基础分：平静吸引子=1.0, 活跃=0.8, 焦虑/低落=0.5, 振荡/迁移=0.3
        let base_score = match self.chaos.state.attractor {
            StrangeAttractor::CalmBasin => 1.0,
            StrangeAttractor::ActiveBasin => 0.8,
            StrangeAttractor::AnxietyBasin | StrangeAttractor::LowMoodBasin => 0.5,
            StrangeAttractor::OscillatingBasin | StrangeAttractor::Transitional => 0.3,
        };

        // 残留平衡调制：正向多→加分，负向多→减分
        // Residue balance modulation: more positive → bonus, more negative → penalty
        let balance_mod = if positive_count > negative_count {
            0.1 * (positive_count - negative_count).min(3) as f64
        } else {
            -0.1 * (negative_count - positive_count).min(3) as f64
        };

        // 脉冲过载调制：活跃脉冲多→减分
        // Pulse overload modulation: more active pulses → penalty
        let pulse_mod = -0.05 * self.pulse.active_pulses.len().min(5) as f64;

        let overall_score = (base_score + balance_mod + pulse_mod).clamp(0.0, 1.0);

        // 失衡警告 / Imbalance warning
        let imbalance_warning = if negative_count > 5 && positive_count == 0 {
            Some("情绪严重失衡：只有负向残留，无正向缓冲".to_string())
        } else if matches!(
            self.chaos.state.attractor,
            StrangeAttractor::OscillatingBasin
        ) && attractor_dwell_secs > 3600.0
        {
            Some("情绪持续振荡超过1小时，可能需要外部干预".to_string())
        } else if self.pulse.shock_absorber.consumed > self.pulse.shock_absorber.capacity * 0.9 {
            Some("冲击吸收器接近过载，情绪弹性即将耗尽".to_string())
        } else {
            None
        };

        EmotionalHealthReport {
            overall_score,
            dominant_valence,
            positive_residue_count: positive_count,
            negative_residue_count: negative_count,
            attractor: self.chaos.state.attractor,
            attractor_dwell_secs,
            imbalance_warning,
        }
    }

    // ── G2: 传染因果追溯 / Contagion Causal Tracing ──

    /// 构建传染因果链 / Build contagion causal chain
    ///
    /// 从指定目标情绪回溯，构建完整的传染因果链。
    /// 数字生命的自省："我为什么感到内疚？因为我先愤怒了，愤怒让我内疚。"
    ///
    /// Trace back from a target emotion to build the full contagion causal chain.
    /// Digital life's introspection: "Why do I feel guilty? Because I was angry first, anger made me guilty."
    pub fn contagion_chain(&self, target: ContagionEmotion) -> Option<ContagionChain> {
        // 找到所有目标为 target 的传染 / Find all contagions targeting `target`
        let target_contagions: Vec<&CrossContagion> = self
            .contagion
            .recent_contagions
            .iter()
            .filter(|c| c.target_emotion == target)
            .collect();

        if target_contagions.is_empty() {
            return None;
        }

        // 构建链：从最近的传染回溯 / Build chain: trace back from most recent contagion
        let mut nodes = Vec::new();
        let mut current_target = target;

        // 限制回溯深度防止无限循环 / Limit trace depth to prevent infinite loop
        let max_depth = self.contagion.config.max_chain_depth as usize;

        for _ in 0..max_depth {
            // 找到目标为 current_target 的最近传染 / Find most recent contagion targeting current_target
            let found = self
                .contagion
                .recent_contagions
                .iter()
                .filter(|c| c.target_emotion == current_target)
                .max_by_key(|c| c.timestamp);

            if let Some(contagion) = found {
                nodes.push(ContagionChainNode {
                    rule: contagion.rule,
                    source: contagion.source_emotion,
                    target: contagion.target_emotion,
                    strength: contagion.strength,
                    timestamp: contagion.timestamp,
                });
                // 继续回溯源情绪 / Continue tracing source emotion
                current_target = contagion.source_emotion;
            } else {
                break;
            }
        }

        // 反转使源头在前 / Reverse so source comes first
        nodes.reverse();

        if nodes.is_empty() {
            None
        } else {
            Some(ContagionChain {
                nodes,
                created_at: target_contagions
                    .iter()
                    .map(|c| c.timestamp)
                    .max()
                    .unwrap_or(0),
            })
        }
    }

    // ── G3: 残留-身体双向通道 / Residue-Body Bidirectional Channel ──

    /// 计算残留-身体双向信号 / Compute residue-body bidirectional signal
    ///
    /// 身心一体：身体紧张催生焦虑残留，焦虑残留加剧身体紧张。
    /// Mind-body unity: body tension breeds anxiety residue, anxiety residue intensifies body tension.
    pub fn residue_body_signal(&self, now: i64) -> ResidueBodySignal {
        let bm = self.residue.combined_effect(now).body_memory.clone();

        // 身体→残留：身体状态催生残留 / Body→Residue: body state breeds residue
        let (body_born_residue, body_born_strength) = if bm.tension > 0.5 {
            // 高紧张→催生 Tension 残留 / High tension → breed Tension residue
            (Some(ResidueKind::Tension), (bm.tension - 0.5) * 0.3)
        } else if bm.heaviness > 0.5 {
            // 高沉重→催生 LingeringSadness / High heaviness → breed LingeringSadness
            (
                Some(ResidueKind::LingeringSadness),
                (bm.heaviness - 0.5) * 0.2,
            )
        } else if bm.warmth > 0.5 {
            // 高温暖→催生 WarmthResidue / High warmth → breed WarmthResidue
            (Some(ResidueKind::WarmthResidue), (bm.warmth - 0.5) * 0.2)
        } else {
            (None, 0.0)
        };

        // 残留→身体：残留反馈身体 / Residue→Body: residue feeds back to body
        let dominant = self.residue.combined_effect(now).dominant_residue;
        let (residue_feedback_channel, residue_feedback_strength) = match dominant {
            Some(ResidueKind::Tension) => ("tension".to_string(), 0.15),
            Some(ResidueKind::LingeringSadness) => ("heaviness".to_string(), 0.2),
            Some(ResidueKind::SmolderingAnger) => ("tension".to_string(), 0.25),
            Some(ResidueKind::WarmthResidue) => ("warmth".to_string(), 0.15),
            Some(ResidueKind::Afterglow) => ("warmth".to_string(), 0.1),
            _ => ("none".to_string(), 0.0),
        };

        ResidueBodySignal {
            body_born_residue,
            body_born_strength,
            residue_feedback_channel,
            residue_feedback_strength,
        }
    }

    /// 应用残留-身体双向信号 / Apply residue-body bidirectional signal
    ///
    /// 将双向信号实际注入系统：身体催生的残留加入残留引擎，
    /// 残留反馈的身体状态更新到身体记忆。
    ///
    /// Inject bidirectional signal into system: body-bred residue added to engine,
    /// residue-fed body state updated into body memory.
    pub fn apply_residue_body_signal(&mut self, now: i64) {
        let signal = self.residue_body_signal(now);

        // 身体→残留：催生新残留 / Body→Residue: breed new residue
        if let Some(kind) = signal.body_born_residue {
            if signal.body_born_strength > 0.01 {
                let residue = EmotionResidue {
                    id: self.residue.next_id,
                    kind,
                    intensity: signal.body_born_strength.min(1.0),
                    pad_offset: kind.default_pad_offset(),
                    half_life_secs: kind.default_half_life_secs(),
                    created_at: now,
                    updated_at: now,
                    source_pulse_id: None,
                    body_memory: BodyMemory::from_residue_kind(kind, signal.body_born_strength),
                    expressed: false,
                };
                self.residue.next_id += 1;
                self.residue.active_residues.push(residue);
            }
        }
    }

    // ── G4: 脉冲-残留交互 / Pulse-Residue Interaction ──

    /// 计算脉冲-残留交互 / Compute pulse-residue interaction
    ///
    /// 当下与过去对话：新的愤怒脉冲点燃余怒，喜悦脉冲抚平悲伤。
    /// The present conversing with the past: new anger ignites smoldering anger, joy soothes sadness.
    pub fn pulse_residue_interaction(&mut self) -> PulseResidueInteraction {
        let mut amplified: Vec<(u64, f64)> = Vec::new();
        let mut suppressed: Vec<(u64, f64)> = Vec::new();
        let mut total_energy = 0.0;

        // 脉冲-残留共振表 — 编译期二维常量数组，O(1) 索引 / Pulse-residue resonance table — compile-time 2D const array, O(1) lookup
        // RESONANCE[pulse_ordinal][residue_ordinal] = amplify(+)/suppress(-)/none(0.0)
        // 0.0 表示无交互 / 0.0 means no interaction
        // 数字生命意义: 当下与过去的即时对话 — O(1) 查表让情绪回响无论负载多重都能即时计算
        // Digital Life: instant dialogue between present and past — O(1) lookup ensures real-time emotional echo regardless of load
        const PULSE_COUNT: usize = 11;
        const RESIDUE_COUNT: usize = 11;
        const RESONANCE: [[f64; RESIDUE_COUNT]; PULSE_COUNT] = {
            let mut t = [[0.0f64; RESIDUE_COUNT]; PULSE_COUNT];
            // 放大：同类共鸣 / Amplify: same-kind resonance
            t[PulseKind::AngerFlash as usize][ResidueKind::SmolderingAnger as usize] = 1.3;
            t[PulseKind::SadnessSurge as usize][ResidueKind::LingeringSadness as usize] = 1.25;
            t[PulseKind::FearSpike as usize][ResidueKind::WorryResidue as usize] = 1.2;
            t[PulseKind::JoyBurst as usize][ResidueKind::Afterglow as usize] = 1.2;
            t[PulseKind::JoyBurst as usize][ResidueKind::WarmthResidue as usize] = 1.15;
            // 抑制：对立抚平 / Suppress: opposite soothes
            t[PulseKind::JoyBurst as usize][ResidueKind::LingeringSadness as usize] = 0.7;
            t[PulseKind::JoyBurst as usize][ResidueKind::SmolderingAnger as usize] = 0.75;
            t[PulseKind::JoyBurst as usize][ResidueKind::Tension as usize] = 0.8;
            t[PulseKind::SadnessSurge as usize][ResidueKind::Afterglow as usize] = 0.7;
            t[PulseKind::SadnessSurge as usize][ResidueKind::WarmthResidue as usize] = 0.75;
            t
        };

        for pulse in &self.pulse.active_pulses {
            let pi = pulse.kind as usize;
            for residue in &mut self.residue.active_residues {
                let factor = RESONANCE[pi][residue.kind as usize];
                if factor != 0.0 {
                    let original = residue.intensity;
                    residue.intensity = (residue.intensity * factor).clamp(0.0, 1.0);
                    let delta = (residue.intensity - original).abs();
                    total_energy += delta * pulse.intensity;

                    if factor > 1.0 {
                        amplified.push((residue.id, factor));
                    } else {
                        suppressed.push((residue.id, factor));
                    }
                }
            }
        }

        PulseResidueInteraction {
            amplified,
            suppressed,
            total_energy,
        }
    }

    // ── G5: 涌现-传染联动 / Emergence-Contagion Linkage ──

    /// 计算涌现-传染联动 / Compute emergence-contagion linkage
    ///
    /// 情绪敏感期：分岔点降低传染阈值，共振放大特定规则。
    /// Emotional sensitive period: bifurcation lowers thresholds, resonance amplifies rules.
    pub fn emergence_contagion_link(&self) -> Vec<EmergenceContagionLink> {
        let mut links = Vec::new();

        for pattern in &self.chaos.state.emergent_patterns {
            let (threshold_mod, modulated_rules) = match pattern.kind {
                // 分岔点：降低所有传染阈值（情绪不稳定→更容易被传染）
                // Bifurcation: lower all thresholds (unstable → more susceptible)
                EmergentKind::Bifurcation => (
                    0.7, // 降低30%阈值 / Lower threshold by 30%
                    vec![
                        ContagionRule::AngerToGuilt,
                        ContagionRule::AngerToSadness,
                        ContagionRule::SadnessToAnger,
                        ContagionRule::AngerSadnessToShame,
                    ],
                ),
                // 共振：放大匹配频率的传染规则
                // Resonance: amplify frequency-matched contagion rules
                EmergentKind::Resonance => (
                    0.8,
                    vec![
                        ContagionRule::JoyContagion,
                        ContagionRule::CalmContagion,
                        ContagionRule::JoyNostalgiaToGratitude,
                    ],
                ),
                // 情绪循环：放大自我传染规则
                // Emotional cycle: amplify self-contagion rules
                EmergentKind::EmotionalCycle => (
                    0.85,
                    vec![
                        ContagionRule::AnxietyContagion,
                        ContagionRule::CalmContagion,
                    ],
                ),
                // 其他涌现：轻微降低阈值
                // Other emergence: slightly lower threshold
                _ => (0.95, vec![]),
            };

            links.push(EmergenceContagionLink {
                emergence_kind: pattern.kind,
                threshold_modulation: threshold_mod,
                modulated_rules,
                strength: pattern.strength,
            });
        }

        links
    }

    /// 获取当前传染阈值调制因子 / Get current contagion threshold modulation factor
    ///
    /// 综合所有涌现-传染联动效果，返回最终的传染阈值调制因子。
    /// <1.0 表示降低阈值（更容易传染），=1.0 表示无影响。
    ///
    /// Aggregate all emergence-contagion linkage effects, return final threshold modulation.
    /// <1.0 means lower threshold (more susceptible), =1.0 means no effect.
    pub fn contagion_threshold_modulation(&self) -> f64 {
        let links = self.emergence_contagion_link();
        if links.is_empty() {
            return 1.0;
        }
        // 取最强联动的调制因子 / Use strongest linkage's modulation factor
        links
            .iter()
            .map(|l| l.threshold_modulation * l.strength)
            .fold(1.0_f64, |acc, x| acc * x)
            .clamp(0.3, 1.5)
    }
}

impl Default for IrrationalityManager {
    fn default() -> Self {
        Self::new(IrrationalityConfig::default())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 单元测试 / Unit Tests
// ═══════════════════════════════════════════════════════════════════════════
