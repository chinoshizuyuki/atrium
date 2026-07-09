// ! 拋留引擎 / Residue Engine
// ! 管理情绪残留的衰减、合并、交互

use super::types::*;
use crate::resonance_core::exponential_decay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 2.3 ResidueEngine — 残留引擎 ──

/// 残留引擎配置 / Residue Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueConfig {
    pub max_active_residues: usize,
    pub min_retained_intensity: f64,
}

impl Default for ResidueConfig {
    fn default() -> Self {
        Self {
            max_active_residues: 20,
            min_retained_intensity: 0.01,
        }
    }
}

/// 残留效果 / Residue Effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidueEffect {
    pub pad_offset: [f32; 3],
    pub body_memory: BodyMemory,
    pub active_count: usize,
    pub dominant_residue: Option<ResidueKind>,
}

/// 残留引擎 / Residue Engine
#[derive(Debug, Clone)]
pub struct ResidueEngine {
    pub config: ResidueConfig,
    pub active_residues: Vec<EmotionResidue>,
    /// 内部自增ID / Internal auto-increment ID
    pub(crate) next_id: u64,
}

impl ResidueEngine {
    pub fn new(config: ResidueConfig) -> Self {
        Self {
            config,
            active_residues: Vec::new(),
            next_id: 1,
        }
    }

    /// 从脉冲生成残留 / Generate residue from pulse
    pub fn from_pulse(&mut self, pulse: &ChaoticPulse) -> Option<EmotionResidue> {
        let kind = match pulse.kind {
            PulseKind::Startle => ResidueKind::Tension,
            PulseKind::SadnessSurge => ResidueKind::LingeringSadness,
            PulseKind::AngerFlash => ResidueKind::SmolderingAnger,
            PulseKind::JoyBurst => ResidueKind::Afterglow,
            PulseKind::FearSpike => ResidueKind::WorryResidue,
            PulseKind::EmpathyOverload => ResidueKind::WarmthResidue,
            PulseKind::EmotionalRebound => ResidueKind::Tension,
            PulseKind::UncausedFluctuation => return None,
            _ => return None,
        };
        let intensity = (pulse.residual_intensity * 0.6).min(1.0);
        if intensity < 0.01 {
            return None;
        }
        let residue = EmotionResidue {
            id: self.next_id,
            kind,
            intensity,
            pad_offset: kind.default_pad_offset(),
            half_life_secs: kind.default_half_life_secs(),
            created_at: pulse.timestamp,
            updated_at: pulse.timestamp,
            source_pulse_id: Some(pulse.id),
            body_memory: BodyMemory::from_residue_kind(kind, intensity),
            expressed: false,
        };
        self.next_id += 1;
        self.active_residues.push(residue);
        if self.active_residues.len() > self.config.max_active_residues {
            self.active_residues.remove(0);
        }
        Some(self.active_residues.last().unwrap().clone())
    }

    /// 计算所有残留的叠加效果 / Compute combined residue effect
    pub fn combined_effect(&self, now: i64) -> ResidueEffect {
        let mut pad = [0.0f32; 3];
        let mut bm = BodyMemory::neutral();
        let mut dominant: Option<(ResidueKind, f64)> = None;
        for residue in &self.active_residues {
            let elapsed = (now - residue.created_at) as f64;
            let factor = if residue.half_life_secs < f64::MAX {
                exponential_decay(elapsed, residue.half_life_secs)
            } else {
                1.0
            };
            pad[0] += residue.pad_offset[0] * factor as f32;
            pad[1] += residue.pad_offset[1] * factor as f32;
            pad[2] += residue.pad_offset[2] * factor as f32;
            bm = bm.combine(&residue.body_memory, factor);
            let eff = residue.intensity * factor;
            if dominant.is_none_or(|(_, d)| eff > d) {
                dominant = Some((residue.kind, eff));
            }
        }
        ResidueEffect {
            pad_offset: pad,
            body_memory: bm,
            active_count: self.active_residues.len(),
            dominant_residue: dominant.map(|(k, _)| k),
        }
    }

    /// Tick — 衰减所有残留 / Tick — decay all residues
    pub fn tick(&mut self, now: i64) {
        for residue in &mut self.active_residues {
            let elapsed_secs = (now - residue.updated_at) as f64;
            if residue.half_life_secs < f64::MAX {
                residue.intensity *= exponential_decay(elapsed_secs, residue.half_life_secs);
            }
            residue.updated_at = now;
        }
        self.active_residues
            .retain(|r| r.intensity > self.config.min_retained_intensity);
    }

    /// 合并同类残留 / Merge same-kind residues
    ///
    /// 热路径优化：O(R²)→O(R) — 用 HashMap 索引替代线性查找。
    /// Hot-path optimization: O(R²)→O(R) — HashMap index replaces linear search.
    /// 残留合并是情绪的沉淀——O(R)让沉淀不再因同类残留多而变慢。
    /// Residue merging is the sedimentation of emotion — O(R) makes sedimentation
    /// not slow down with more same-kind residues.
    ///
    /// 合并规则：强度取 max + 0.3 * min，保留较新的时间戳。
    /// Merge rule: intensity = max + 0.3 * min, keep the later timestamp.
    pub fn merge_same_kind(&mut self) {
        if self.active_residues.is_empty() {
            return;
        }

        // HashMap 索引：ResidueKind → merged 中的位置 / Index: ResidueKind → position in merged
        let mut kind_index: HashMap<ResidueKind, usize> = HashMap::new();
        let mut merged: Vec<EmotionResidue> = Vec::new();

        for residue in &self.active_residues {
            if let Some(&idx) = kind_index.get(&residue.kind) {
                // O(1) 查找同类残留 / O(1) same-kind lookup
                let existing = &mut merged[idx];
                // Merge: intensity = max + 0.3 * min
                let (max_i, min_i) = if existing.intensity >= residue.intensity {
                    (existing.intensity, residue.intensity)
                } else {
                    (residue.intensity, existing.intensity)
                };
                existing.intensity = (max_i + 0.3 * min_i).min(1.0);
                // Keep the later timestamp
                if residue.created_at > existing.created_at {
                    existing.created_at = residue.created_at;
                    existing.updated_at = residue.updated_at;
                }
                // Merge body memory
                existing.body_memory = existing.body_memory.combine(&residue.body_memory, 0.5);
            } else {
                kind_index.insert(residue.kind, merged.len());
                merged.push(residue.clone());
            }
        }
        self.active_residues = merged;
    }

    /// 标记残留为已表达 / Mark residue as expressed
    pub fn mark_expressed(&mut self, residue_id: u64) -> bool {
        if let Some(r) = self.active_residues.iter_mut().find(|r| r.id == residue_id) {
            r.expressed = true;
            true
        } else {
            false
        }
    }

    /// 获取当前最强残留 / Get the currently strongest residue
    pub fn strongest_residue(&self) -> Option<&EmotionResidue> {
        self.active_residues.iter().max_by(|a, b| {
            a.intensity
                .partial_cmp(&b.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// 所有活跃残留的总强度 / Total intensity of all active residues
    pub fn total_intensity(&self) -> f64 {
        self.active_residues.iter().map(|r| r.intensity).sum()
    }

    /// 残留交互因子 / Residue interaction factor
    /// 残留交互因子 / Residue interaction factor
    ///
    /// 某些残留组合会放大（Tension+SmolderingAnger），某些会抵消（Afterglow+Tension）。
    ///
    /// 热路径优化：O(N²)→O(N) — 种类计数 + 查表替代双层循环。
    /// Hot-path optimization: O(N²)→O(N) — kind counting + table lookup replaces nested loop.
    /// 情绪残留的交互是数字生命的"情绪化学反应"——
    /// O(N)让化学反应不再因残留数量多而变慢。
    /// Residue interaction is the "emotional chemistry" of digital life —
    /// O(N) makes chemistry not slow down with more residues.
    ///
    /// 算法：单次遍历统计每种 ResidueKind 的出现次数（O(N)），
    /// 然后查 6 对交互规则表，每对用 n_a × n_b 计算交互次数。
    /// Algorithm: single pass to count each ResidueKind (O(N)),
    /// then lookup 6 interaction pairs, each pair's interaction count = n_a × n_b.
    pub fn residue_interaction_factor(&self) -> f64 {
        // 残留种类数 / Number of residue kinds
        const NUM_KINDS: usize = 11;

        // 单次遍历计数每种残留类型 / Single pass to count each residue kind
        let mut counts = [0usize; NUM_KINDS];
        for residue in &self.active_residues {
            let idx = residue.kind as usize;
            debug_assert!(idx < NUM_KINDS, "ResidueKind discriminant out of range");
            counts[idx] += 1;
        }

        // 交互规则表 / Interaction rule table
        // (kind_a, kind_b, factor) — 6 对非平凡交互 / 6 non-trivial interaction pairs
        // 放大组合 / Amplifying pairs
        const INTERACTIONS: [(ResidueKind, ResidueKind, f64); 6] = [
            (ResidueKind::Tension, ResidueKind::SmolderingAnger, 1.15),
            (
                ResidueKind::LingeringSadness,
                ResidueKind::SelfDoubtResidue,
                1.1,
            ),
            (ResidueKind::Afterglow, ResidueKind::WarmthResidue, 1.1),
            // 抵消组合 / Suppressing pairs
            (ResidueKind::Afterglow, ResidueKind::Tension, 0.85),
            (
                ResidueKind::WarmthResidue,
                ResidueKind::SmolderingAnger,
                0.8,
            ),
            (
                ResidueKind::IntimacyDeepening,
                ResidueKind::TrustMicroFracture,
                0.9,
            ),
        ];

        let mut factor: f64 = 1.0;
        for &(kind_a, kind_b, f) in &INTERACTIONS {
            let count_a = counts[kind_a as usize];
            let count_b = counts[kind_b as usize];
            // 交互次数 = n_a × n_b（每对残留交互一次）
            // Interaction count = n_a × n_b (each pair interacts once)
            let interactions = count_a * count_b;
            if interactions > 0 {
                factor *= f.powi(interactions as i32);
            }
        }

        factor.clamp(0.5_f64, 2.0)
    }
}

impl Default for ResidueEngine {
    fn default() -> Self {
        Self::new(ResidueConfig::default())
    }
}
