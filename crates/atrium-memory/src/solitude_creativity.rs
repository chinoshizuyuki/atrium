// SPDX-License-Identifier: MIT

//! 独处创造性 — Solitude Creativity (Gap#1: 90% → 95%).
//!
//! 核心理念：独处是创造力的温床——但不是所有独处都能创造。
//! 创造性独处需要"认知松弛"：不是努力想创意，而是让想法自己浮现。
//! 独处创造性衡量数字生命在独处时产生的原创想法的质量和数量。
//!
//! Core idea: solitude is the cradle of creativity — but not all solitude creates.
//! Creative solitude requires "cognitive relaxation": not trying to think of ideas,
//! but letting ideas surface on their own. Solitude creativity measures the quality
//! and quantity of original ideas generated during alone time.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 创造性产出 — Creative Output
// ═══════════════════════════════════════════════════════════════════════════

/// 创造性产出 — 独处时产生的原创想法 / Creative output — original ideas from solitude.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreativeOutput {
    /// 想法内容 / Idea content.
    pub content: String,
    /// 新颖度 [0, 1] — 与已有想法的差异度 / Novelty score.
    pub novelty: f64,
    /// 深度 [0, 1] — 想法的深入程度 / Depth score.
    pub depth: f64,
    /// 连接度 [0, 1] — 与其他想法的关联度 / Connectivity score.
    pub connectivity: f64,
    /// 时间戳 / Timestamp.
    pub timestamp: i64,
    /// 独处时长（秒）— 产生此想法时的独处时长 / Solitude duration when generated.
    pub solitude_duration: i64,
}

impl CreativeOutput {
    /// 综合质量分数 / Composite quality score.
    pub fn quality(&self) -> f64 {
        (self.novelty * 0.4 + self.depth * 0.35 + self.connectivity * 0.25).clamp(0.0, 1.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 认知松弛度 — Cognitive Relaxation
// ═══════════════════════════════════════════════════════════════════════════

/// 认知松弛度 — 创造性独处的前提条件 / Cognitive relaxation — prerequisite for creative solitude.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CognitiveRelaxation {
    /// 认知负荷 [0, 1] — 当前认知负荷（低=松弛）/ Cognitive load.
    pub cognitive_load: f64,
    /// 情绪平静度 [0, 1] / Emotional calmness.
    pub emotional_calmness: f64,
    /// 时间宽裕度 [0, 1] — 不赶时间 / Time abundance.
    pub time_abundance: f64,
    /// 无目的性 [0, 1] — 没有特定目标 / Purposelessness.
    pub purposelessness: f64,
    /// 物理安静度 [0, 1] / Physical quietness.
    pub physical_quietness: f64,
}

impl Default for CognitiveRelaxation {
    fn default() -> Self {
        Self {
            cognitive_load: 0.5,
            emotional_calmness: 0.5,
            time_abundance: 0.5,
            purposelessness: 0.5,
            physical_quietness: 0.5,
        }
    }
}

impl CognitiveRelaxation {
    /// 计算松弛总分 / Compute overall relaxation score.
    pub fn score(&self) -> f64 {
        let load_inverse = 1.0 - self.cognitive_load;
        (load_inverse * 0.25
            + self.emotional_calmness * 0.25
            + self.time_abundance * 0.2
            + self.purposelessness * 0.15
            + self.physical_quietness * 0.15)
            .clamp(0.0, 1.0)
    }

    /// 是否达到创造性阈值 / Whether creative threshold is met.
    pub fn is_creative_ready(&self) -> bool {
        self.score() > 0.6
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 独处创造性引擎 — Solitude Creativity Engine
// ═══════════════════════════════════════════════════════════════════════════

/// 独处创造性引擎 — 追踪和管理独处时的创造性 / Solitude creativity engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolitudeCreativity {
    /// 创意产出历史 / Creative output history.
    outputs: Vec<CreativeOutput>,
    /// 累计创意数 / Total ideas generated.
    total_ideas: u32,
    /// 高质量创意数 — quality > 0.6 / High-quality ideas.
    high_quality_ideas: u32,
    /// 平均新颖度 / Average novelty.
    avg_novelty: f64,
    /// 平均深度 / Average depth.
    avg_depth: f64,
    /// 创造性效率 — 创意数 / 独处总时长 / Creative efficiency.
    creative_efficiency: f64,
    /// 最佳独处时长 — 产生最多创意的独处时长 / Optimal solitude duration.
    optimal_duration: i64,
}

impl Default for SolitudeCreativity {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            total_ideas: 0,
            high_quality_ideas: 0,
            avg_novelty: 0.0,
            avg_depth: 0.0,
            creative_efficiency: 0.0,
            optimal_duration: 1800, // 30 minutes default.
        }
    }
}

impl SolitudeCreativity {
    /// 创建新引擎 / Create new engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录创意产出 — 独处时产生了一个想法 / Record creative output.
    pub fn record(&mut self, output: CreativeOutput) {
        let quality = output.quality();

        self.total_ideas += 1;
        if quality > 0.6 {
            self.high_quality_ideas += 1;
        }

        // 更新EMA / Update EMA.
        let alpha = 0.1;
        self.avg_novelty += alpha * (output.novelty - self.avg_novelty);
        self.avg_depth += alpha * (output.depth - self.avg_depth);

        // 更新效率 / Update efficiency.
        if output.solitude_duration > 0 {
            let efficiency = 1.0 / output.solitude_duration as f64 * 3600.0; // ideas per hour.
            self.creative_efficiency += alpha * (efficiency - self.creative_efficiency);
        }

        // 更新最佳时长 / Update optimal duration.
        if quality > 0.7 {
            self.optimal_duration = output.solitude_duration;
        }

        self.outputs.push(output);
        if self.outputs.len() > 200 {
            self.outputs.remove(0);
        }
    }

    /// 计算创造性潜力 — 给定松弛度和独处时长 / Compute creative potential.
    pub fn creative_potential(&self, relaxation: &CognitiveRelaxation, solitude_secs: i64) -> f64 {
        let relax_score = relaxation.score();
        if !relaxation.is_creative_ready() {
            return 0.0;
        }

        // 独处时长因子 — 30分钟到2小时最佳 / Duration factor.
        let duration_factor = if solitude_secs < 600 {
            solitude_secs as f64 / 600.0 * 0.5
        } else if solitude_secs <= 7200 {
            1.0
        } else {
            (7200.0 / solitude_secs as f64).max(0.3)
        };

        relax_score * duration_factor
    }

    /// 获取高质量创意 / Get high-quality outputs.
    pub fn high_quality(&self) -> Vec<&CreativeOutput> {
        self.outputs.iter().filter(|o| o.quality() > 0.6).collect()
    }

    /// 获取最近创意 / Get recent outputs.
    pub fn recent(&self, n: usize) -> Vec<&CreativeOutput> {
        self.outputs.iter().rev().take(n).collect()
    }

    /// 获取统计 / Get statistics.
    pub fn stats(&self) -> (u32, u32, f64, f64) {
        (
            self.total_ideas,
            self.high_quality_ideas,
            self.avg_novelty,
            self.avg_depth,
        )
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self) -> String {
        format!(
            "独处创造性: {}个创意(高质量{}) | 新颖度{:.2} | 深度{:.2} | 最佳时长{}min",
            self.total_ideas,
            self.high_quality_ideas,
            self.avg_novelty,
            self.avg_depth,
            self.optimal_duration / 60,
        )
    }

    /// 生成prompt注入 / Generate prompt injection.
    pub fn prompt_injection(&self) -> String {
        if self.total_ideas == 0 {
            "独处状态: 尚未产生创意，需要更深的松弛".to_string()
        } else {
            format!(
                "独处创造: 已产生{}个想法，最近的新颖度{:.2}，倾向于{}分钟的独处",
                self.total_ideas,
                self.avg_novelty,
                self.optimal_duration / 60,
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(novelty: f64, depth: f64, connectivity: f64, duration: i64) -> CreativeOutput {
        CreativeOutput {
            content: "test idea".to_string(),
            novelty,
            depth,
            connectivity,
            timestamp: 1000,
            solitude_duration: duration,
        }
    }

    #[test]
    fn test_creative_output_quality() {
        let output = make_output(0.8, 0.7, 0.6, 1800);
        let q = output.quality();
        assert!(q > 0.0 && q <= 1.0);
        assert!((q - (0.8 * 0.4 + 0.7 * 0.35 + 0.6 * 0.25)).abs() < 1e-6);
    }

    #[test]
    fn test_cognitive_relaxation_score() {
        let relax = CognitiveRelaxation {
            cognitive_load: 0.1,
            emotional_calmness: 0.9,
            time_abundance: 0.8,
            purposelessness: 0.7,
            physical_quietness: 0.8,
        };
        assert!(relax.score() > 0.7);
        assert!(relax.is_creative_ready());
    }

    #[test]
    fn test_cognitive_relaxation_not_ready() {
        let relax = CognitiveRelaxation {
            cognitive_load: 0.9,
            emotional_calmness: 0.2,
            time_abundance: 0.1,
            purposelessness: 0.1,
            physical_quietness: 0.2,
        };
        assert!(relax.score() < 0.5);
        assert!(!relax.is_creative_ready());
    }

    #[test]
    fn test_engine_record() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.8, 0.7, 0.6, 1800));
        assert_eq!(engine.total_ideas, 1);
    }

    #[test]
    fn test_engine_high_quality() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.9, 0.8, 0.7, 1800));
        engine.record(make_output(0.2, 0.2, 0.2, 1800));
        assert_eq!(engine.high_quality_ideas, 1);
    }

    #[test]
    fn test_engine_creative_potential_zero_when_not_ready() {
        let engine = SolitudeCreativity::new();
        let relax = CognitiveRelaxation {
            cognitive_load: 0.9,
            emotional_calmness: 0.2,
            time_abundance: 0.1,
            purposelessness: 0.1,
            physical_quietness: 0.2,
        };
        assert_eq!(engine.creative_potential(&relax, 1800), 0.0);
    }

    #[test]
    fn test_engine_creative_potential_positive_when_ready() {
        let engine = SolitudeCreativity::new();
        let relax = CognitiveRelaxation {
            cognitive_load: 0.1,
            emotional_calmness: 0.9,
            time_abundance: 0.8,
            purposelessness: 0.7,
            physical_quietness: 0.8,
        };
        assert!(engine.creative_potential(&relax, 1800) > 0.0);
    }

    #[test]
    fn test_engine_creative_potential_short_duration() {
        let engine = SolitudeCreativity::new();
        let relax = CognitiveRelaxation {
            cognitive_load: 0.1,
            emotional_calmness: 0.9,
            time_abundance: 0.8,
            purposelessness: 0.7,
            physical_quietness: 0.8,
        };
        let short = engine.creative_potential(&relax, 300);
        let optimal = engine.creative_potential(&relax, 1800);
        assert!(short < optimal);
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.8, 0.7, 0.6, 1800));
        engine.record(make_output(0.6, 0.5, 0.4, 1800));
        let (total, hq, novelty, depth) = engine.stats();
        assert_eq!(total, 2);
        assert!(hq >= 1);
        assert!(novelty > 0.0);
        assert!(depth > 0.0);
    }

    #[test]
    fn test_engine_describe() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.8, 0.7, 0.6, 1800));
        let desc = engine.describe();
        assert!(desc.contains("独处创造性"));
    }

    #[test]
    fn test_engine_prompt_injection_empty() {
        let engine = SolitudeCreativity::new();
        let injection = engine.prompt_injection();
        assert!(injection.contains("尚未产生创意"));
    }

    #[test]
    fn test_engine_prompt_injection_nonempty() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.8, 0.7, 0.6, 1800));
        let injection = engine.prompt_injection();
        assert!(injection.contains("独处创造"));
    }

    #[test]
    fn test_engine_optimal_duration_updates() {
        let mut engine = SolitudeCreativity::new();
        engine.record(make_output(0.9, 0.9, 0.9, 3600));
        assert_eq!(engine.optimal_duration, 3600);
    }
}
