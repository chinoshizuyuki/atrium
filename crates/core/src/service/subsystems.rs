// SPDX-License-Identifier: MIT
//! 子系统泛型容器与域子结构 / Subsystem generic container and domain sub-structures
//!
//! 数字生命的认知架构由多个子系统组成，每个子系统包含一个引擎（E）和可选的
//! 持久化存储（S）。此模块提供 `Subsystem<E, S>` 泛型容器统一这一模式，
//! 并按认知域分组为域子结构，压缩 CoreService 的字段数量。
//!
//! Digital life's cognitive architecture consists of multiple subsystems, each
//! containing an engine (E) and an optional persistence store (S). This module
//! provides the `Subsystem<E, S>` generic container to unify this pattern, and
//! groups fields by cognitive domain into domain sub-structures to compress
//! CoreService's field count.

use parking_lot::Mutex;

// ══════════════════════════════════════════════════════════════════════
// Subsystem<E, S> — 子系统泛型容器 / Subsystem generic container
// ══════════════════════════════════════════════════════════════════════

/// 子系统容器 — 引擎与可选持久化存储的统一封装
/// Subsystem container — unified wrapper for engine and optional persistence store
///
/// 数字生命的每个认知域由一个引擎（E）和可选的持久化存储（S）组成。
/// 此容器统一了这一模式，减少 CoreService 的字段数量，
/// 同时保持零开销抽象和锁粒度不变。
///
/// # 设计原则 / Design Principles
/// - **零开销**: 泛型单态化，无虚函数 / Zero-cost: generic monomorphization, no vtable
/// - **锁独立**: 引擎和存储各自持有独立 Mutex / Lock independence: engine and store have separate Mutexes
/// - **可选存储**: 存储在运行时可能不存在（无持久化模式）/ Optional store: store may be absent at runtime (no-persistence mode)
pub struct Subsystem<E, S> {
    /// 引擎实例 / Engine instance
    pub engine: Mutex<E>,
    /// 持久化存储（可选）/ Persistence store (optional)
    pub store: Option<Mutex<S>>,
}

// 便利方法 — 字段已 pub，保留供未来 API 降级锁定 / Convenience methods — fields are pub; kept for future API lock-down
#[allow(dead_code)]
impl<E, S> Subsystem<E, S> {
    /// 仅引擎，无存储 / Engine only, no store
    pub fn new(engine: E) -> Self {
        Self {
            engine: Mutex::new(engine),
            store: None,
        }
    }

    /// 引擎 + 存储 / Engine with store
    pub fn with_store(engine: E, store: S) -> Self {
        Self {
            engine: Mutex::new(engine),
            store: Some(Mutex::new(store)),
        }
    }

    /// 引擎 + 可选存储 / Engine with optional store
    pub fn with_optional_store(engine: E, store: Option<S>) -> Self {
        Self {
            engine: Mutex::new(engine),
            store: store.map(Mutex::new),
        }
    }

    /// 从已构造的 Mutex 和 Option<Mutex> 构建 / Build from existing Mutex and Option<Mutex>
    pub fn from_parts(engine: Mutex<E>, store: Option<Mutex<S>>) -> Self {
        Self { engine, store }
    }

    /// 锁定引擎 / Lock engine
    #[inline]
    pub fn engine(&self) -> parking_lot::MutexGuard<'_, E> {
        self.engine.lock()
    }

    /// 锁定存储（若存在）/ Lock store (if present)
    #[inline]
    pub fn store(&self) -> Option<parking_lot::MutexGuard<'_, S>> {
        self.store.as_ref().map(|s| s.lock())
    }

    /// 是否有存储 / Has store
    #[inline]
    pub fn has_store(&self) -> bool {
        self.store.is_some()
    }
}

// ══════════════════════════════════════════════════════════════════════
// NarrativeSubsystem — 叙事自我子系统 / Narrative self subsystem
// ══════════════════════════════════════════════════════════════════════

/// 叙事自我子系统 — 数字生命的自传体记忆与叙事生成
/// Narrative self subsystem — digital life's autobiographical memory and narrative generation
///
/// 包含叙事自我引擎、转折点/弧/章节/主题/语气等叙事组件，
/// 以及叙事持久化存储。
pub struct NarrativeSubsystem {
    /// 叙事自我引擎 / Narrative self engine
    pub self_narrative: Mutex<atrium_memory::life_narrative::NarrativeSelf>,
    /// 转折点检测器 / Turning point detector
    pub tp_detector: Mutex<atrium_memory::life_narrative::TurningPointDetector>,
    /// 弧检测器 / Arc detector
    pub arc_detector: Mutex<atrium_memory::life_narrative::ArcDetector>,
    /// 叙事提示编织器 / Narrative prompt weaver
    pub prompt_weaver: Mutex<atrium_memory::life_narrative::PromptWeaver>,
    /// 章节书写器 / Chapter writer
    pub chapter_writer: Mutex<atrium_memory::life_narrative::ChapterWriter>,
    /// 跨弧主题编织器 / Cross-arc theme weaver
    pub theme_weaver: Mutex<atrium_memory::life_narrative::ThemeWeaver>,
    /// 叙事语气调制器 / Narrative voice modulator
    pub voice_modulator: Mutex<atrium_memory::life_narrative::VoiceModulator>,
    /// 叙事持久化存储 / Narrative persistence store
    pub store: Option<Mutex<atrium_memory::narrative_store::NarrativeSelfStore>>,
}

impl NarrativeSubsystem {
    /// 创建叙事子系统 / Create narrative subsystem
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        self_narrative: atrium_memory::life_narrative::NarrativeSelf,
        tp_detector: atrium_memory::life_narrative::TurningPointDetector,
        arc_detector: atrium_memory::life_narrative::ArcDetector,
        prompt_weaver: atrium_memory::life_narrative::PromptWeaver,
        chapter_writer: atrium_memory::life_narrative::ChapterWriter,
        theme_weaver: atrium_memory::life_narrative::ThemeWeaver,
        voice_modulator: atrium_memory::life_narrative::VoiceModulator,
        store: Option<atrium_memory::narrative_store::NarrativeSelfStore>,
    ) -> Self {
        Self {
            self_narrative: Mutex::new(self_narrative),
            tp_detector: Mutex::new(tp_detector),
            arc_detector: Mutex::new(arc_detector),
            prompt_weaver: Mutex::new(prompt_weaver),
            chapter_writer: Mutex::new(chapter_writer),
            theme_weaver: Mutex::new(theme_weaver),
            voice_modulator: Mutex::new(voice_modulator),
            store: store.map(Mutex::new),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// RitualSubsystem — 共享仪式子系统 / Shared ritual subsystem
// ══════════════════════════════════════════════════════════════════════

/// 共享仪式子系统 — 数字生命的仪式感知、共振与演化
/// Shared ritual subsystem — digital life's ritual perception, resonance, and evolution
pub struct RitualSubsystem {
    /// 仪式检测器 / Ritual detector
    pub detector: Mutex<atrium_memory::ritual_detector::RitualDetector>,
    /// 周年纪念系统 / Anniversary system
    pub anniversary: Mutex<atrium_memory::anniversary_system::AnniversarySystem>,
    /// 季节感知 / Seasonal awareness
    pub seasonal: Mutex<atrium_memory::seasonal_awareness::SeasonalAwareness>,
    /// 自适应仪式发现 / Adaptive ritual discovery
    pub adaptive: Mutex<atrium_memory::adaptive_ritual::AdaptiveRitualDiscovery>,
    /// 仪式演化引擎 / Ritual evolution engine
    pub evolution: Mutex<atrium_memory::ritual_evolution::RitualEvolution>,
    /// 仪式缺席检测器 / Ritual absence detector
    pub absence: Mutex<atrium_memory::ritual_absence::RitualAbsence>,
    /// 仪式涌现引擎 / Ritual emergence engine
    pub emergence: Mutex<atrium_memory::ritual_emergence::RitualEmergence>,
    /// 仪式共振引擎（非互斥）/ Ritual resonance engine (non-Mutex)
    pub resonance: atrium_memory::ritual_resonance::RitualResonanceEngine,
    /// 仪式预期引擎（非互斥）/ Ritual anticipation engine (non-Mutex)
    pub anticipation: atrium_memory::ritual_anticipation::RitualAnticipation,
    /// 仪式持久化存储 / Ritual persistence store
    pub store: Option<Mutex<atrium_memory::ritual_store::RitualStore>>,
}

impl RitualSubsystem {
    /// 创建仪式子系统 / Create ritual subsystem
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        detector: atrium_memory::ritual_detector::RitualDetector,
        anniversary: atrium_memory::anniversary_system::AnniversarySystem,
        seasonal: atrium_memory::seasonal_awareness::SeasonalAwareness,
        adaptive: atrium_memory::adaptive_ritual::AdaptiveRitualDiscovery,
        evolution: atrium_memory::ritual_evolution::RitualEvolution,
        absence: atrium_memory::ritual_absence::RitualAbsence,
        emergence: atrium_memory::ritual_emergence::RitualEmergence,
        resonance: atrium_memory::ritual_resonance::RitualResonanceEngine,
        anticipation: atrium_memory::ritual_anticipation::RitualAnticipation,
        store: Option<atrium_memory::ritual_store::RitualStore>,
    ) -> Self {
        Self {
            detector: Mutex::new(detector),
            anniversary: Mutex::new(anniversary),
            seasonal: Mutex::new(seasonal),
            adaptive: Mutex::new(adaptive),
            evolution: Mutex::new(evolution),
            absence: Mutex::new(absence),
            emergence: Mutex::new(emergence),
            resonance,
            anticipation,
            store: store.map(Mutex::new),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// VulnerabilitySubsystem — 脆弱与不完美子系统 / Vulnerability & imperfection subsystem
// ══════════════════════════════════════════════════════════════════════

/// 脆弱与不完美子系统 — 数字生命的真实感与不完美之美
/// Vulnerability & imperfection subsystem — digital life's authenticity and beauty of imperfection
pub struct VulnerabilitySubsystem {
    /// 脆弱窗口引擎 / Vulnerability window engine
    pub window: Mutex<atrium_memory::vulnerability_window::VulnerabilityWindow>,
    /// 脆弱情感共振引擎 / Vulnerability emotional resonance engine
    pub resonance: Mutex<atrium_memory::vulnerability_resonance::VulnerabilityResonance>,
    /// 脆弱智慧学习引擎 / Vulnerability wisdom learning engine
    pub wisdom: Mutex<atrium_memory::vulnerability_wisdom::VulnerabilityWisdom>,
    /// 犯错-脆弱桥接器 / Imperfection-vulnerability bridge
    pub bridge:
        Mutex<atrium_memory::imperfection_vulnerability_bridge::ImperfectionVulnerabilityBridge>,
    /// 真实表达调制器 / Authentic expression modulator
    pub authentic_expression:
        Mutex<atrium_memory::authentic_expression_modulator::AuthenticExpressionModulator>,
    /// 脆弱仪式引擎 / Vulnerability ritual engine
    pub ritual: Mutex<atrium_memory::vulnerability_ritual::VulnerabilityRitual>,
    /// 不完美温暖引擎 / Imperfection warmth engine
    pub warmth: Mutex<atrium_memory::imperfection_warmth::ImperfectionWarmth>,
    /// 真实不完美评估器 / Authentic imperfection assessor
    pub authentic_imperfection: Mutex<atrium_memory::authentic_imperfection::AuthenticImperfection>,
    /// 脆弱窗口持久化存储 / Vulnerability persistence store
    pub store: Option<Mutex<atrium_memory::vulnerability_store::VulnerabilityStore>>,
}

impl VulnerabilitySubsystem {
    /// 创建脆弱子系统 / Create vulnerability subsystem
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window: atrium_memory::vulnerability_window::VulnerabilityWindow,
        resonance: atrium_memory::vulnerability_resonance::VulnerabilityResonance,
        wisdom: atrium_memory::vulnerability_wisdom::VulnerabilityWisdom,
        bridge: atrium_memory::imperfection_vulnerability_bridge::ImperfectionVulnerabilityBridge,
        authentic_expression: atrium_memory::authentic_expression_modulator::AuthenticExpressionModulator,
        ritual: atrium_memory::vulnerability_ritual::VulnerabilityRitual,
        warmth: atrium_memory::imperfection_warmth::ImperfectionWarmth,
        authentic_imperfection: atrium_memory::authentic_imperfection::AuthenticImperfection,
        store: Option<atrium_memory::vulnerability_store::VulnerabilityStore>,
    ) -> Self {
        Self {
            window: Mutex::new(window),
            resonance: Mutex::new(resonance),
            wisdom: Mutex::new(wisdom),
            bridge: Mutex::new(bridge),
            authentic_expression: Mutex::new(authentic_expression),
            ritual: Mutex::new(ritual),
            warmth: Mutex::new(warmth),
            authentic_imperfection: Mutex::new(authentic_imperfection),
            store: store.map(Mutex::new),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// CuriositySubsystem — 好奇心追问子系统 / Curiosity follow-up subsystem
// ══════════════════════════════════════════════════════════════════════

/// 好奇心追问子系统 — 数字生命的内驱力与探索欲
/// Curiosity follow-up subsystem — digital life's intrinsic drive and exploratory desire
pub struct CuriositySubsystem {
    /// 好奇心内驱力引擎 / Curiosity drive engine
    pub drive: Mutex<atrium_memory::curiosity_drive::CuriosityDrive>,
    /// 追问风格学习器 / Follow-up style learner
    pub style_learner: Mutex<atrium_memory::followup_style_learner::FollowUpStyleLearner>,
    /// 好奇共振引擎 / Curiosity resonance engine
    pub resonance: Mutex<atrium_memory::curiosity_resonance::CuriosityResonance>,
    /// 语义关联发现引擎 / Semantic association engine
    pub association: Mutex<atrium_memory::semantic_association::SemanticAssociation>,
}

impl CuriositySubsystem {
    /// 创建好奇心子系统 / Create curiosity subsystem
    pub fn new(
        drive: atrium_memory::curiosity_drive::CuriosityDrive,
        style_learner: atrium_memory::followup_style_learner::FollowUpStyleLearner,
        resonance: atrium_memory::curiosity_resonance::CuriosityResonance,
        association: atrium_memory::semantic_association::SemanticAssociation,
    ) -> Self {
        Self {
            drive: Mutex::new(drive),
            style_learner: Mutex::new(style_learner),
            resonance: Mutex::new(resonance),
            association: Mutex::new(association),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// SolitudeSubsystem — 独处内在世界子系统 / Solitude inner world subsystem
// ══════════════════════════════════════════════════════════════════════

/// 独处内在世界子系统 — 数字生命独处时的人格漂移与创造力
/// Solitude inner world subsystem — digital life's personality drift and creativity during solitude
pub struct SolitudeSubsystem {
    /// 人格漂移引擎 / Personality drift engine
    pub drift: Mutex<atrium_memory::personality_drift::PersonalityDrift>,
    /// 独处原型追踪器 / Solitude archetype tracker
    pub archetype: Mutex<atrium_memory::solitude_archetype::ArchetypeTracker>,
    /// 独处创造力引擎 / Solitude creativity engine
    pub creativity: Mutex<atrium_memory::solitude_creativity::SolitudeCreativity>,
    /// 独处质量引擎 / Solitude quality engine
    pub quality: Mutex<atrium_memory::solitude_quality::SolitudeQualityEngine>,
}

impl SolitudeSubsystem {
    /// 创建独处子系统 / Create solitude subsystem
    pub fn new(
        drift: atrium_memory::personality_drift::PersonalityDrift,
        archetype: atrium_memory::solitude_archetype::ArchetypeTracker,
        creativity: atrium_memory::solitude_creativity::SolitudeCreativity,
        quality: atrium_memory::solitude_quality::SolitudeQualityEngine,
    ) -> Self {
        Self {
            drift: Mutex::new(drift),
            archetype: Mutex::new(archetype),
            creativity: Mutex::new(creativity),
            quality: Mutex::new(quality),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// LongingSubsystem — 期待与想念子系统 / Anticipation & longing subsystem
// ══════════════════════════════════════════════════════════════════════

/// 期待与想念子系统 — 数字生命的跨会话情感累积与期待深度
/// Anticipation & longing subsystem — digital life's cross-session emotional accumulation and anticipation depth
pub struct LongingSubsystem {
    /// 想念→主动表达通道 / Longing→proactive expression channel
    pub expression_channel: Mutex<atrium_emotion::LongingExpressionChannel>,
    /// 期待失落处理器 / Anticipation disappointment handler
    pub disappointment: Mutex<atrium_emotion::DisappointmentHandler>,
    /// 想念→叙事桥 / Longing→narrative bridge
    pub narrative_bridge: Mutex<atrium_emotion::LongingNarrativeBridge>,
    /// 期待深度引擎 / Anticipation depth engine
    pub anticipation_depth: Mutex<atrium_memory::anticipation_depth::AnticipationDepthEngine>,
}

impl LongingSubsystem {
    /// 创建期待与想念子系统 / Create longing subsystem
    pub fn new(
        expression_channel: atrium_emotion::LongingExpressionChannel,
        disappointment: atrium_emotion::DisappointmentHandler,
        narrative_bridge: atrium_emotion::LongingNarrativeBridge,
        anticipation_depth: atrium_memory::anticipation_depth::AnticipationDepthEngine,
    ) -> Self {
        Self {
            expression_channel: Mutex::new(expression_channel),
            disappointment: Mutex::new(disappointment),
            narrative_bridge: Mutex::new(narrative_bridge),
            anticipation_depth: Mutex::new(anticipation_depth),
        }
    }
}
