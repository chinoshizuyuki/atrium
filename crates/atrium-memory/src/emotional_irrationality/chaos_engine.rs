// ! 混沌引擎 / Chaos Engine
// ! 检测奇怪吸引子、分岔点、涌现模式

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ── 2.5 ChaosEngine — 混沌引擎 ──

/// 混沌引擎配置 / Chaos Engine Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosConfig {
    pub max_trajectory_len: usize,
    pub bifurcation_window_secs: i64,
    pub min_cycle_secs: i64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            max_trajectory_len: 1000,
            bifurcation_window_secs: 3600,
            min_cycle_secs: 3600,
        }
    }
}

/// 混沌引擎 / Chaos Engine
#[derive(Debug, Clone)]
pub struct ChaosEngine {
    pub config: ChaosConfig,
    pub state: EmotionChaos,
}

impl ChaosEngine {
    pub fn new(config: ChaosConfig, chaos_params: ChaosParams) -> Self {
        Self {
            config,
            state: EmotionChaos {
                attractor: StrangeAttractor::CalmBasin,
                trajectory: VecDeque::new(),
                emergent_patterns: VecDeque::new(),
                chaos_params,
            },
        }
    }

    /// 记录轨迹点 / Record trajectory point
    ///
    /// 热路径优化：O(T)→O(1) — Vec::remove(0) → VecDeque::pop_front。
    /// Hot-path optimization: O(T)→O(1) — Vec::remove(0) → VecDeque::pop_front.
    /// 混沌轨迹是情绪的蝴蝶效应——O(1)记录让蝴蝶扇翅不再有代价。
    /// Chaos trajectory is the butterfly effect of emotion — O(1) recording
    /// makes butterfly wing-flapping cost-free.
    pub fn record(&mut self, pad: &[f32; 3], now: i64) {
        self.state.trajectory.push_back(TrajectoryPoint {
            pad: *pad,
            timestamp: now,
        });
        if self.state.trajectory.len() > self.config.max_trajectory_len {
            self.state.trajectory.pop_front();
        }
    }

    /// 检测吸引子 / Detect attractor
    ///
    /// 适配 VecDeque：用迭代器替代切片索引，保持语义不变。
    /// VecDeque adaptation: iterators replace slice indexing, semantics unchanged.
    pub fn detect_attractor(&mut self) -> StrangeAttractor {
        let traj = &self.state.trajectory;
        if traj.len() < 10 {
            self.state.attractor = StrangeAttractor::CalmBasin;
            return self.state.attractor;
        }
        let n = traj.len();
        let mid = n / 2;
        let mut sum_p1 = 0.0f64;
        let mut sum_a1 = 0.0f64;
        let mut sum_p2 = 0.0f64;
        let mut sum_a2 = 0.0f64;
        for tp in traj.iter().take(mid) {
            sum_p1 += tp.pad[0] as f64;
            sum_a1 += tp.pad[1] as f64;
        }
        for tp in traj.iter().skip(mid) {
            sum_p2 += tp.pad[0] as f64;
            sum_a2 += tp.pad[1] as f64;
        }
        let avg_p1 = sum_p1 / mid as f64;
        let avg_a1 = sum_a1 / mid as f64;
        let avg_p2 = sum_p2 / (n - mid) as f64;
        let avg_a2 = sum_a2 / (n - mid) as f64;
        let drift = ((avg_p2 - avg_p1).powi(2) + (avg_a2 - avg_a1).powi(2)).sqrt();
        if drift > 0.3 {
            self.state.attractor = StrangeAttractor::Transitional;
        } else {
            let avg_p = (avg_p1 + avg_p2) / 2.0;
            let avg_a = (avg_a1 + avg_a2) / 2.0;
            self.state.attractor = if avg_p > 0.2 && avg_a > 0.1 {
                StrangeAttractor::ActiveBasin
            } else if avg_p < -0.2 && avg_a < -0.1 {
                StrangeAttractor::LowMoodBasin
            } else if avg_p < -0.1 && avg_a > 0.1 {
                StrangeAttractor::AnxietyBasin
            } else if drift > 0.1 {
                StrangeAttractor::OscillatingBasin
            } else {
                StrangeAttractor::CalmBasin
            };
        }
        self.state.attractor
    }

    /// 检测分岔 / Detect bifurcation
    ///
    /// 适配 VecDeque：用迭代器替代切片索引，保持语义不变。
    /// VecDeque adaptation: iterators replace slice indexing, semantics unchanged.
    pub fn detect_bifurcation(&mut self, now: i64) -> Option<EmergentPattern> {
        let traj = &self.state.trajectory;
        if traj.len() < 20 {
            return None;
        }
        let n = traj.len();
        let q1 = n / 4;
        let q3 = 3 * n / 4;
        let mut sum_p1 = 0.0f64;
        let mut sum_a1 = 0.0f64;
        let mut sum_p2 = 0.0f64;
        let mut sum_a2 = 0.0f64;
        for tp in traj.iter().take(q1) {
            sum_p1 += tp.pad[0] as f64;
            sum_a1 += tp.pad[1] as f64;
        }
        for tp in traj.iter().skip(q3) {
            sum_p2 += tp.pad[0] as f64;
            sum_a2 += tp.pad[1] as f64;
        }
        let c1_p = sum_p1 / q1 as f64;
        let c1_a = sum_a1 / q1 as f64;
        let c2_p = sum_p2 / (n - q3) as f64;
        let c2_a = sum_a2 / (n - q3) as f64;
        let dist = ((c2_p - c1_p).powi(2) + (c2_a - c1_a).powi(2)).sqrt();
        if dist > self.state.chaos_params.emergence_threshold {
            Some(EmergentPattern {
                kind: EmergentKind::Bifurcation,
                description: format!(
                    "PAD center shifted from ({:.2},{:.2}) to ({:.2},{:.2})",
                    c1_p, c1_a, c2_p, c2_a
                ),
                strength: dist,
                detected_at: now,
            })
        } else {
            None
        }
    }

    /// Tick / Tick
    pub fn tick(&mut self, pad: &[f32; 3], now: i64) {
        self.record(pad, now);
        self.state.attractor = self.detect_attractor();
        if let Some(pattern) = self.detect_bifurcation(now) {
            self.state.emergent_patterns.push_back(pattern);
            if self.state.emergent_patterns.len() > 10 {
                self.state.emergent_patterns.pop_front();
            }
        }
    }
}

impl Default for ChaosEngine {
    fn default() -> Self {
        Self::new(ChaosConfig::default(), ChaosParams::default())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
