use super::*;
use chrono::Local;
use serde::{Deserialize, Serialize};

// ════════════════════════════════════════════════════════════════════
// NarrativeTrigger / RewriteTrigger — 叙事触发器 / Narrative Triggers
// ════════════════════════════════════════════════════════════════════

/// 叙事触发器 — 触发叙事引擎处理的事件 / Narrative trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeTrigger {
    /// 新转折点被检测到 / New turning point detected
    TurningPointDetected { tp_id: u64, kind: TurningPointKind },
    /// 关系阶段变更 / Relationship stage changed
    RelationshipStageChanged { from: String, to: String },
    /// 情感大幅变化 / Significant emotion change
    EmotionShift {
        pad_before: [f32; 3],
        pad_after: [f32; 3],
    },
    /// 定时 tick / Periodic tick
    Tick,
    /// 首次启动（回溯构建）/ First startup (retrospective build)
    FirstStartup,
    /// 手动重写请求 / Manual rewrite request
    ManualRewrite { target: RewriteTarget },
}

/// 重写目标 — 指定重写的叙事对象 / Rewrite target
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RewriteTarget {
    /// 重写自我描述 / Rewrite self description
    SelfDescription,
    /// 重写指定章节 / Rewrite specific chapter
    Chapter(u64),
    /// 重写指定弧的主题 / Rewrite specific arc's theme
    ArcTheme(u64),
    /// 重写关系叙事 / Rewrite relationship narrative
    RelationshipNarrative,
}

/// 重写触发器 — 触发章节重写的事件 / Rewrite trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteTrigger {
    /// 触发类型 / Trigger type
    pub target: RewriteTarget,
    /// 触发原因 / Trigger reason
    pub reason: String,
    /// 触发时间 / Trigger timestamp
    pub timestamp: i64,
    /// 新证据（事件 ID 列表）/ New evidence
    pub evidence: Vec<NarrativeEventId>,
}

impl RewriteTrigger {
    /// 创建重写触发器 / Create a rewrite trigger
    pub fn new(target: RewriteTarget, reason: String) -> Self {
        Self {
            target,
            reason,
            timestamp: Local::now().timestamp(),
            evidence: Vec::new(),
        }
    }

    /// 附加证据 / Attach evidence
    pub fn with_evidence(mut self, evidence: Vec<NarrativeEventId>) -> Self {
        self.evidence = evidence;
        self
    }
}

// ════════════════════════════════════════════════════════════════════
// ArcConfig — 弧检测器配置 / Arc Detector Config
// ════════════════════════════════════════════════════════════════════

/// 弧检测器配置 / Arc detector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcConfig {
    /// 弧最少转折点数（少于此数不构成弧）/ Min turning points per arc
    pub min_turning_points: usize,
    /// 弧休眠天数阈值 / Arc dormancy threshold (days)
    pub dormancy_days: i64,
    /// 弧完结天数阈值 / Arc closure threshold (days)
    pub closure_days: i64,
    /// 同类型弧合并阈值（相似度）/ Same-kind arc merge similarity threshold
    pub merge_similarity_threshold: f64,
    /// 弧显著度衰减率（每天）/ Arc significance decay rate (per day)
    pub significance_decay_per_day: f64,
    /// 最大活跃弧数 / Max active arcs
    pub max_active_arcs: usize,
}

impl Default for ArcConfig {
    fn default() -> Self {
        Self {
            min_turning_points: 2,
            dormancy_days: 14,
            closure_days: 60,
            merge_similarity_threshold: 0.8,
            significance_decay_per_day: 0.01,
            max_active_arcs: 10,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// ArcUpdate — 弧检测结果 / Arc Detection Update
// ════════════════════════════════════════════════════════════════════

/// 弧检测结果 — ArcDetector.detect() 的输出 / Arc detection update
#[derive(Debug, Clone)]
pub enum ArcUpdate {
    /// 创建了新弧 / New arc created
    ArcCreated { arc_id: u64, kind: ArcKind },
    /// 弧添加了新转折点 / Turning point added to arc
    TurningPointAdded { arc_id: u64, tp_id: u64 },
    /// 弧进入休眠 / Arc went dormant
    ArcDormant { arc_id: u64 },
    /// 弧已完结 / Arc closed
    ArcClosed { arc_id: u64 },
    /// 弧被新弧取代 / Arc superseded
    ArcSuperseded { old_arc_id: u64, new_arc_id: u64 },
    /// 弧显著度更新 / Arc significance updated
    SignificanceUpdated { arc_id: u64, old: f64, new: f64 },
    /// 无变化 / No change
    NoChange,
}

// ════════════════════════════════════════════════════════════════════
// ArcDetector — 弧检测器 / Arc Detector
// ════════════════════════════════════════════════════════════════════

/// 弧检测器 — 从转折点流中识别和更新叙事弧
/// Arc detector — identify and update narrative arcs from turning point stream
///
/// 核心职责 / Core responsibilities:
/// 1. 新转折点 → 归入已有弧 或 创建新弧 / New TP → assign to existing or create new arc
/// 2. 定期检查弧的休眠/完结 / Periodically check arc dormancy/closure
/// 3. 弧显著度衰减与更新 / Arc significance decay and update
/// 4. 同类型弧合并 / Same-kind arc merging
pub struct ArcDetector {
    /// 检测配置 / Detection config
    pub config: ArcConfig,
    /// 下一个弧 ID / Next arc ID
    next_arc_id: u64,
}

impl ArcDetector {
    /// 创建弧检测器 / Create arc detector
    pub fn new(config: ArcConfig) -> Self {
        Self {
            config,
            next_arc_id: 1,
        }
    }

    /// 使用默认配置创建 / Create with default config
    pub fn default_new() -> Self {
        Self::new(ArcConfig::default())
    }

    /// 分配下一个弧 ID / Allocate next arc ID
    pub fn alloc_arc_id(&mut self) -> u64 {
        let id = self.next_arc_id;
        self.next_arc_id += 1;
        id
    }

    /// 处理新转折点 — 尝试归入已有弧或创建新弧
    /// Process new turning point — try to assign to existing arc or create new arc
    ///
    /// 策略 / Strategy:
    /// 1. 查找同类型活跃弧，若主题相似则归入 / Find same-kind active arc, assign if similar
    /// 2. 若无合适弧，创建新弧 / If no suitable arc, create new arc
    /// 3. 检查活跃弧数是否超限 / Check if active arc count exceeds limit
    pub fn process_turning_point(
        &mut self,
        model: &mut NarrativeSelf,
        tp: &TurningPoint,
    ) -> Vec<ArcUpdate> {
        let mut updates = Vec::new();

        // 策略 1：查找同类型活跃弧 / Strategy 1: Find same-kind active arc
        let target_kind = tp.kind.infer_arc_kind();
        let mut best_arc_id: Option<u64> = None;
        let mut best_significance = 0.0;

        for arc in &model.active_arcs {
            if arc.kind == target_kind && arc.is_active() {
                // 简单相似度：同类型 + 时间接近 / Simple similarity: same kind + temporal proximity
                let time_proximity = if let Some(&last_tp_id) = arc.turning_point_ids.last() {
                    if let Some(last_tp) = model.get_turning_point(last_tp_id) {
                        let days_diff = (tp.timestamp - last_tp.timestamp).abs() / 86400;
                        1.0 / (1.0 + days_diff as f64)
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };
                let score = arc.significance * time_proximity;
                if score > best_significance {
                    best_significance = score;
                    best_arc_id = Some(arc.id);
                }
            }
        }

        if let Some(arc_id) = best_arc_id {
            // 归入已有弧 / Assign to existing arc
            if let Some(arc) = model.active_arcs.iter_mut().find(|a| a.id == arc_id) {
                arc.add_turning_point(tp.id);
            }
            // 标记转折点所属弧 / Mark turning point's arc membership
            if let Some(t) = model.turning_points.iter_mut().find(|t| t.id == tp.id) {
                t.add_to_arc(arc_id);
            }
            updates.push(ArcUpdate::TurningPointAdded {
                arc_id,
                tp_id: tp.id,
            });
        } else {
            // 策略 2：创建新弧 / Strategy 2: Create new arc
            let arc_id = self.alloc_arc_id();
            let title = format!("{}弧", target_kind.label_zh());
            let theme = format!("{}相关的故事线", target_kind.label_zh());
            let mut arc = NarrativeArc::new(arc_id, target_kind, title, theme);
            arc.add_turning_point(tp.id);

            // 检查活跃弧数限制 / Check active arc limit
            if model.active_arcs.len() >= self.config.max_active_arcs {
                // 将最不显著的弧休眠 / Dorm the least significant arc
                if let Some(min_arc) =
                    model
                        .active_arcs
                        .iter()
                        .filter(|a| a.is_active())
                        .min_by(|a, b| {
                            a.significance
                                .partial_cmp(&b.significance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                {
                    let min_id = min_arc.id;
                    if let Some(a) = model.active_arcs.iter_mut().find(|a| a.id == min_id) {
                        a.make_dormant();
                    }
                    updates.push(ArcUpdate::ArcDormant { arc_id: min_id });
                }
            }

            // 标记转折点所属弧 / Mark turning point's arc membership
            if let Some(t) = model.turning_points.iter_mut().find(|t| t.id == tp.id) {
                t.add_to_arc(arc_id);
            }

            model.add_arc(arc);
            updates.push(ArcUpdate::ArcCreated {
                arc_id,
                kind: target_kind,
            });
        }

        updates
    }

    /// 定期 tick — 检查弧的休眠/完结/显著度衰减
    /// Periodic tick — check arc dormancy, closure, and significance decay
    pub fn tick(&self, model: &mut NarrativeSelf, now: i64) -> Vec<ArcUpdate> {
        let mut updates = Vec::new();
        let day_secs: i64 = 86400;

        // 预计算每条弧的最后活动时间 / Pre-compute last activity time per arc
        let arc_last_activity: Vec<(u64, i64)> = model
            .active_arcs
            .iter()
            .map(|arc| {
                let last_activity = if let Some(&last_tp_id) = arc.turning_point_ids.last() {
                    model
                        .get_turning_point(last_tp_id)
                        .map(|t| t.timestamp)
                        .unwrap_or(arc.started_at)
                } else {
                    arc.started_at
                };
                (arc.id, last_activity)
            })
            .collect();

        // 活跃弧检查 / Active arc checks
        for arc in &mut model.active_arcs {
            // 查找预计算的最后活动时间 / Look up pre-computed last activity
            let last_activity = arc_last_activity
                .iter()
                .find(|(id, _)| *id == arc.id)
                .map(|(_, t)| *t)
                .unwrap_or(arc.started_at);

            let days_inactive = (now - last_activity) / day_secs;

            // 显著度衰减 / Significance decay
            if days_inactive > 0 {
                let old_sig = arc.significance;
                arc.significance = (arc.significance
                    - self.config.significance_decay_per_day * days_inactive as f64)
                    .max(0.1);
                if (arc.significance - old_sig).abs() > 0.001 {
                    updates.push(ArcUpdate::SignificanceUpdated {
                        arc_id: arc.id,
                        old: old_sig,
                        new: arc.significance,
                    });
                }
            }

            // 休眠检查 / Dormancy check
            if arc.status == ArcStatus::Active && days_inactive >= self.config.dormancy_days {
                arc.make_dormant();
                updates.push(ArcUpdate::ArcDormant { arc_id: arc.id });
            }

            // 完结检查 / Closure check
            if arc.status == ArcStatus::Dormant && days_inactive >= self.config.closure_days {
                arc.close(now);
                updates.push(ArcUpdate::ArcClosed { arc_id: arc.id });
            }
        }

        // 将已完结弧移到 closed_arcs / Move closed arcs to closed_arcs
        let closed_ids: Vec<u64> = model
            .active_arcs
            .iter()
            .filter(|a| a.status == ArcStatus::Closed)
            .map(|a| a.id)
            .collect();
        for id in closed_ids {
            if let Some(pos) = model.active_arcs.iter().position(|a| a.id == id) {
                let arc = model.active_arcs.remove(pos);
                model.closed_arcs.push(arc);
            }
        }

        updates
    }
}
