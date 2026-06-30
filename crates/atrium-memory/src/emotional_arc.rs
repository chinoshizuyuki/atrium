// SPDX-License-Identifier: MIT
//! 情绪轨迹 — 回复内部的情感时间序列
//! EmotionalArc — Intra-reply emotion trajectory.
//!
//! 当前系统把回复当作一个情感快照，但真实表达中一句话内部情绪就在变化：
//! "我...其实有点难过，但没关系啦，你不用担心的。"
//!   ↑沉默    ↑悲伤浮现      ↑自我安慰    ↑转向坚强

use serde::{Deserialize, Serialize};

use crate::relationship::RelationshipStage;

// ════════════════════════════════════════════════════════════════════
// ExpressionStrategy — 表达策略
// ════════════════════════════════════════════════════════════════════

/// 表达策略 — 回复中每个语义段的表达方式
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExpressionStrategy {
    /// 直接表达 — "我很开心"
    Direct,
    /// 含蓄暗示 — "今天的阳光真好..."
    Implicit,
    /// 自我修复 — "不对，应该说..."（犹豫→修正）
    SelfRepair,
    /// 战略性沉默 — "..."（留白让用户感受）
    StrategicSilence,
    /// 情绪转向 — 从悲伤转向坚强（保护性转换）
    ProtectiveShift,
    /// 反向表达 — 嘴上说"没事"但语气暴露（仅在深度关系阶段）
    Incongruent,
}

// ════════════════════════════════════════════════════════════════════
// ArcTrend — 轨迹整体趋势
// ════════════════════════════════════════════════════════════════════

/// 回复内情绪轨迹的整体趋势
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ArcTrend {
    /// 情绪升温（悲伤→愤怒，平静→兴奋）
    Escalating,
    /// 情绪降温（兴奋→平静，愤怒→冷静）
    DeEscalating,
    /// 平稳
    Steady,
    /// 波动（内心矛盾）
    Oscillating,
    /// 从强烈情绪恢复（悲伤→平静）
    Recovering,
}

// ════════════════════════════════════════════════════════════════════
// ArcWaypoint — 轨迹节点
// ════════════════════════════════════════════════════════════════════

/// 轨迹节点 — 对应回复中的一个语义段
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArcWaypoint {
    /// 该段的 PAD 状态
    pub pad: [f32; 3],
    /// 在回复中的相对位置 0.0-1.0
    pub position: f32,
    /// 该段的表达策略
    pub strategy: ExpressionStrategy,
}

// ════════════════════════════════════════════════════════════════════
// EmotionalArc — 回复内情绪轨迹
// ════════════════════════════════════════════════════════════════════

/// 一条回复内部的情感轨迹
///
/// 不是单一 PAD 点，而是一个时间序列。
/// 让回复内部有情绪变化，而非全程恒定。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmotionalArc {
    /// 轨迹节点 — 按位置排序
    pub waypoints: Vec<ArcWaypoint>,
    /// 整体趋势
    pub trend: ArcTrend,
}

/// 相邻轨迹节点的 PAD 欧氏距离上限
///
/// 防止回复内部情绪跳变。
pub const ARC_INERTIA_THRESHOLD: f32 = 0.3;

/// 最大轨迹节点数
pub const MAX_WAYPOINTS: usize = 8;

impl EmotionalArc {
    /// 空轨迹（单点）
    pub fn single(pad: [f32; 3]) -> Self {
        Self {
            waypoints: vec![ArcWaypoint {
                pad,
                position: 0.0,
                strategy: ExpressionStrategy::Direct,
            }],
            trend: ArcTrend::Steady,
        }
    }

    /// 生成情绪轨迹
    ///
    /// 从 current_pad 沿趋势方向插值到 target_pad，
    /// 生成多个轨迹节点，每个节点有独立的表达策略。
    pub fn generate(
        current_pad: [f32; 3],
        target_pad: [f32; 3],
        relationship: &RelationshipStage,
        reply_length_estimate: usize,
    ) -> Self {
        // 1. 确定整体趋势
        let trend = Self::determine_trend(&current_pad, &target_pad);

        // 2. 确定节点数量
        //    短回复（<50字）：1-2个节点
        //    中回复（50-200字）：3-5个节点
        //    长回复（>200字）：5-8个节点
        let num_waypoints = if reply_length_estimate < 50 {
            1
        } else if reply_length_estimate < 200 {
            ((reply_length_estimate as f32 / 50.0).ceil() as usize).clamp(2, 5)
        } else {
            ((reply_length_estimate as f32 / 40.0).ceil() as usize).clamp(3, MAX_WAYPOINTS)
        };

        // 3. 生成轨迹节点
        let mut waypoints = Vec::with_capacity(num_waypoints);
        for i in 0..num_waypoints {
            let t = if num_waypoints == 1 {
                0.0
            } else {
                i as f32 / (num_waypoints - 1) as f32
            };

            // 沿趋势方向插值
            let pad = Self::interpolate_pad(&current_pad, &target_pad, t);

            // 确定表达策略
            let strategy = Self::determine_strategy(&pad, &current_pad, relationship, t);

            waypoints.push(ArcWaypoint {
                pad,
                position: t,
                strategy,
            });
        }

        // 4. 惯性约束：确保相邻节点 PAD 距离不超过阈值
        Self::enforce_inertia(&mut waypoints);

        Self { waypoints, trend }
    }

    /// 确定整体趋势
    fn determine_trend(current: &[f32; 3], target: &[f32; 3]) -> ArcTrend {
        let pleasure_delta = target[0] - current[0];
        let arousal_delta = target[1] - current[1].abs();

        // 如果目标愉悦度明显高于当前 → 恢复
        if pleasure_delta > 0.2 {
            return ArcTrend::Recovering;
        }
        // 如果目标愉悦度明显低于当前 → 升温（情绪恶化）
        if pleasure_delta < -0.2 {
            return ArcTrend::Escalating;
        }
        // 如果唤醒度在下降 → 降温
        if arousal_delta < -0.2 {
            return ArcTrend::DeEscalating;
        }
        // 如果愉悦度变化不大但方向不一致 → 波动
        if (current[0] - target[0]).abs() < 0.1 && current[0].abs() > 0.3 {
            return ArcTrend::Oscillating;
        }

        ArcTrend::Steady
    }

    /// PAD 插值
    fn interpolate_pad(current: &[f32; 3], target: &[f32; 3], t: f32) -> [f32; 3] {
        [
            current[0] + (target[0] - current[0]) * t,
            current[1] + (target[1] - current[1]) * t,
            current[2] + (target[2] - current[2]) * t,
        ]
    }

    /// 确定表达策略
    fn determine_strategy(
        pad: &[f32; 3],
        start_pad: &[f32; 3],
        relationship: &RelationshipStage,
        position: f32,
    ) -> ExpressionStrategy {
        // 回复开头（position < 0.2）且悲伤 → 可能沉默
        if position < 0.2 && pad[0] < -0.3 && pad[1] < 0.0 {
            return ExpressionStrategy::StrategicSilence;
        }

        // 回复中段（0.3-0.6）且情绪在转向 → 自我修复
        if (0.3..0.6).contains(&position) {
            let pleasure_change = (pad[0] - start_pad[0]).abs();
            if pleasure_change > 0.2 {
                return ExpressionStrategy::SelfRepair;
            }
        }

        // 回复后段（>0.6）且从负情绪转向正 → 保护性转向
        if position > 0.6 && start_pad[0] < -0.2 && pad[0] > start_pad[0] {
            return ExpressionStrategy::ProtectiveShift;
        }

        // 深度关系 + 悲伤 → 允许反向表达（假装没事）
        if matches!(relationship, RelationshipStage::Deep { .. }) && pad[0] < -0.3 && position > 0.4
        {
            // 30% 概率选择 Incongruent
            if rand::random::<f32>() < 0.3 {
                return ExpressionStrategy::Incongruent;
            }
        }

        // 低支配 + 有情绪 → 含蓄暗示
        if pad[2] < -0.2 && pad[0].abs() > 0.2 {
            return ExpressionStrategy::Implicit;
        }

        ExpressionStrategy::Direct
    }

    /// 惯性约束 — 确保相邻节点 PAD 距离不超过阈值
    ///
    /// 如果跳变过大，在中间插入平滑节点。
    fn enforce_inertia(waypoints: &mut Vec<ArcWaypoint>) {
        if waypoints.len() < 2 {
            return;
        }

        let mut i = 0;
        while i < waypoints.len() - 1 {
            let dist = pad_distance(&waypoints[i].pad, &waypoints[i + 1].pad);
            if dist > ARC_INERTIA_THRESHOLD {
                // 在中间插入一个平滑节点
                let mid_pad = [
                    (waypoints[i].pad[0] + waypoints[i + 1].pad[0]) / 2.0,
                    (waypoints[i].pad[1] + waypoints[i + 1].pad[1]) / 2.0,
                    (waypoints[i].pad[2] + waypoints[i + 1].pad[2]) / 2.0,
                ];
                let mid_pos = (waypoints[i].position + waypoints[i + 1].position) / 2.0;

                let mid_waypoint = ArcWaypoint {
                    pad: mid_pad,
                    position: mid_pos,
                    strategy: ExpressionStrategy::Implicit,
                };

                waypoints.insert(i + 1, mid_waypoint);

                // 如果节点数已达上限，停止插入
                if waypoints.len() >= MAX_WAYPOINTS {
                    break;
                }
            }
            i += 1;
        }
    }

    /// 验证轨迹的惯性约束
    pub fn validate_inertia(&self) -> bool {
        for window in self.waypoints.windows(2) {
            let dist = pad_distance(&window[0].pad, &window[1].pad);
            if dist > ARC_INERTIA_THRESHOLD * 1.1 {
                // 允许 10% 容差
                return false;
            }
        }
        true
    }

    /// 获取轨迹起点的 PAD
    pub fn start_pad(&self) -> Option<[f32; 3]> {
        self.waypoints.first().map(|w| w.pad)
    }

    /// 获取轨迹终点的 PAD
    pub fn end_pad(&self) -> Option<[f32; 3]> {
        self.waypoints.last().map(|w| w.pad)
    }

    /// 获取指定位置处的 PAD（线性插值）
    pub fn pad_at_position(&self, position: f32) -> [f32; 3] {
        if self.waypoints.is_empty() {
            return [0.0, 0.0, 0.0];
        }
        if self.waypoints.len() == 1 {
            return self.waypoints[0].pad;
        }

        // 找到包含 position 的区间
        for window in self.waypoints.windows(2) {
            if position >= window[0].position && position <= window[1].position {
                let span = window[1].position - window[0].position;
                if span < 1e-6 {
                    return window[0].pad;
                }
                let t = (position - window[0].position) / span;
                return Self::interpolate_pad(&window[0].pad, &window[1].pad, t);
            }
        }

        // 超出范围，返回最近的端点
        if position <= self.waypoints[0].position {
            self.waypoints[0].pad
        } else {
            self.waypoints.last().unwrap().pad
        }
    }

    /// 生成轨迹描述（用于 Prompt 注入）
    pub fn to_prompt_hint(&self) -> String {
        if self.waypoints.len() <= 1 {
            return String::new();
        }

        let trend_desc = match self.trend {
            ArcTrend::Escalating => "情绪在升温",
            ArcTrend::DeEscalating => "情绪在降温",
            ArcTrend::Steady => "情绪平稳",
            ArcTrend::Oscillating => "内心有些矛盾",
            ArcTrend::Recovering => "从强烈情绪慢慢恢复",
        };

        let strategy_hints: Vec<&str> = self
            .waypoints
            .iter()
            .map(|w| match w.strategy {
                ExpressionStrategy::Direct => "",
                ExpressionStrategy::Implicit => "含蓄地",
                ExpressionStrategy::SelfRepair => "犹豫后修正",
                ExpressionStrategy::StrategicSilence => "用沉默表达",
                ExpressionStrategy::ProtectiveShift => "从脆弱转向坚强",
                ExpressionStrategy::Incongruent => "嘴上说没事但语气暴露了",
            })
            .filter(|s| !s.is_empty())
            .collect();

        if strategy_hints.is_empty() {
            format!("[情绪轨迹] {}", trend_desc)
        } else {
            format!(
                "[情绪轨迹] {}，表达方式：{}",
                trend_desc,
                strategy_hints.join("→")
            )
        }
    }
}

/// PAD 欧氏距离
pub fn pad_distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dp = a[0] - b[0];
    let da = a[1] - b[1];
    let dd = a[2] - b[2];
    (dp * dp + da * da + dd * dd).sqrt()
}

// ════════════════════════════════════════════════════════════════════
// 测试
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

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

    fn acq() -> RelationshipStage {
        RelationshipStage::Acquaintance {
            since: 0,
            interactions: 5,
        }
    }

    #[test]
    fn test_emotional_arc_single() {
        let arc = EmotionalArc::single([0.5, 0.3, 0.2]);
        assert_eq!(arc.waypoints.len(), 1);
        assert_eq!(arc.trend, ArcTrend::Steady);
    }

    #[test]
    fn test_emotional_arc_short_reply() {
        let arc = EmotionalArc::generate(
            [-0.7, -0.3, -0.5],
            [-0.3, -0.1, -0.2],
            &familiar(),
            30, // 短回复
        );
        assert_eq!(arc.waypoints.len(), 1, "short reply should have 1 waypoint");
    }

    #[test]
    fn test_emotional_arc_medium_reply() {
        let arc = EmotionalArc::generate(
            [-0.5, 0.2, -0.3],
            [-0.2, 0.0, -0.1],
            &familiar(),
            100, // 中回复
        );
        assert!(
            arc.waypoints.len() >= 2 && arc.waypoints.len() <= 5,
            "medium reply should have 2-5 waypoints, got {}",
            arc.waypoints.len()
        );
    }

    #[test]
    fn test_emotional_arc_long_reply() {
        let arc = EmotionalArc::generate(
            [-0.7, -0.3, -0.5],
            [0.1, -0.1, 0.0],
            &familiar(),
            300, // 长回复
        );
        assert!(
            arc.waypoints.len() >= 3,
            "long reply should have 3+ waypoints, got {}",
            arc.waypoints.len()
        );
        assert!(arc.waypoints.len() <= MAX_WAYPOINTS);
    }

    #[test]
    fn test_emotional_arc_inertia_constraint() {
        let arc = EmotionalArc::generate([-0.7, -0.3, -0.5], [0.5, 0.3, 0.2], &familiar(), 300);
        // 惯性约束后应通过验证
        assert!(
            arc.validate_inertia(),
            "arc should satisfy inertia constraint after enforcement"
        );
    }

    #[test]
    fn test_emotional_arc_trend_recovering() {
        let arc = EmotionalArc::generate([-0.7, -0.3, -0.5], [0.1, -0.1, 0.0], &familiar(), 100);
        // 从悲伤到平静 → Recovering
        assert_eq!(arc.trend, ArcTrend::Recovering);
    }

    #[test]
    fn test_emotional_arc_trend_escalating() {
        let arc = EmotionalArc::generate([0.1, 0.0, 0.0], [-0.5, 0.3, -0.2], &familiar(), 100);
        // 从平静到悲伤 → Escalating
        assert_eq!(arc.trend, ArcTrend::Escalating);
    }

    #[test]
    fn test_emotional_arc_trend_steady() {
        let arc = EmotionalArc::generate([0.1, 0.0, 0.0], [0.1, 0.0, 0.0], &familiar(), 100);
        assert_eq!(arc.trend, ArcTrend::Steady);
    }

    #[test]
    fn test_emotional_arc_start_end_pad() {
        let arc = EmotionalArc::generate([-0.5, 0.2, -0.3], [-0.2, 0.0, -0.1], &familiar(), 100);
        let start = arc.start_pad().unwrap();
        let end = arc.end_pad().unwrap();
        // 起点应接近 current_pad
        assert!((start[0] - (-0.5)).abs() < 0.01);
        // 终点应接近 target_pad
        assert!((end[0] - (-0.2)).abs() < 0.01);
    }

    #[test]
    fn test_emotional_arc_pad_at_position() {
        let arc = EmotionalArc::generate([-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], &familiar(), 200);
        // position=0 应返回起点
        let pad_start = arc.pad_at_position(0.0);
        assert!((pad_start[0] - (-0.5)).abs() < 0.01);
        // position=1 应返回终点
        let pad_end = arc.pad_at_position(1.0);
        assert!((pad_end[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_emotional_arc_strategy_silence() {
        // 悲伤开头 → StrategicSilence
        let arc = EmotionalArc::generate([-0.7, -0.3, -0.5], [-0.3, -0.1, -0.2], &familiar(), 200);
        // 第一个节点的策略可能是 StrategicSilence
        if let Some(first) = arc.waypoints.first() {
            if first.pad[0] < -0.3 && first.pad[1] < 0.0 && first.position < 0.2 {
                assert_eq!(first.strategy, ExpressionStrategy::StrategicSilence);
            }
        }
    }

    #[test]
    fn test_emotional_arc_incongruent_only_in_deep() {
        // 初识阶段不应出现 Incongruent
        let arc = EmotionalArc::generate([-0.7, -0.3, -0.5], [-0.3, -0.1, -0.2], &acq(), 200);
        for wp in &arc.waypoints {
            assert_ne!(
                wp.strategy,
                ExpressionStrategy::Incongruent,
                "Incongruent should not appear in Acquaintance stage"
            );
        }
    }

    #[test]
    fn test_emotional_arc_prompt_hint() {
        let arc = EmotionalArc::generate([-0.7, -0.3, -0.5], [0.1, -0.1, 0.0], &familiar(), 200);
        let hint = arc.to_prompt_hint();
        assert!(
            !hint.is_empty(),
            "multi-waypoint arc should have prompt hint"
        );
    }

    #[test]
    fn test_pad_distance() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        assert!((pad_distance(&a, &b) - 1.0).abs() < 1e-6);

        let c = [0.0, 0.0, 0.0];
        assert!((pad_distance(&a, &c)).abs() < 1e-6);
    }
}
