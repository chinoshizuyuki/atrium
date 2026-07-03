// SPDX-License-Identifier: MIT
//! 自我学习回放管道 — Selector → Replayer → Analyzer → Updater
//! Self-learning replay pipeline — Selector → Replayer → Analyzer → Updater.
//!
//! Atrium 在后台持续运行此管道，从历史对话中发现模式、提炼洞察。
//! Atrium runs this pipeline in the background, discovering patterns and
//! extracting insights from historical conversations.
//! 不需要 LLM 参与，纯 Rust 规则驱动，延迟 < 100μs/次。
//! No LLM required, pure Rust rule-driven, latency < 100μs/cycle.
//!
//! Selector 支持全量扫描 + 冷却去重 + Updater 持久化回写。
//! Selector supports full-scan + cooldown dedup; Updater persists results back to FactStore.

use crate::fact_store::{Fact, FactStore};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 回放管道阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Selecting,
    Replaying,
    Analyzing,
    Updating,
}

/// 回放管道
pub struct ReplayPipeline {
    stage: Stage,
    /// 上次运行时间戳
    last_run: u64,
    /// 运行间隔（秒）
    interval_secs: u64,
    /// 累计运行次数
    run_count: u64,
    /// 发现的模式数
    patterns_found: u64,
    /// 已发现模式的摘要 → 最后报告时间（冷却去重）
    seen_patterns: HashMap<String, u64>,
    /// 冷却期（秒），同一模式在此时间内不重复报告
    cooldown_secs: u64,
    /// Selector 使用的主体列表（默认: ["主人"]）
    selector_subjects: Vec<String>,
    /// 最近一次运行发现的模式（供 ACK 合成消费）
    last_patterns: Vec<DiscoveredPattern>,
    /// 已转化为 ACK 的模式摘要集合（防重复）
    consumed_patterns: std::collections::HashSet<String>,
}

impl Default for ReplayPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayPipeline {
    /// 默认每 300 秒（5 分钟）运行一次
    pub fn new() -> Self {
        Self {
            stage: Stage::Idle,
            last_run: 0,
            interval_secs: 300,
            run_count: 0,
            patterns_found: 0,
            seen_patterns: HashMap::new(),
            cooldown_secs: 3600,
            selector_subjects: vec!["主人".into()],
            last_patterns: Vec::new(),
            consumed_patterns: std::collections::HashSet::new(),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs.max(1);
        self
    }

    /// 设置 Selector 关注的主体列表
    pub fn with_subjects(mut self, subjects: Vec<String>) -> Self {
        self.selector_subjects = subjects;
        self
    }

    /// 设置模式冷却时间（秒）
    pub fn with_cooldown(mut self, secs: u64) -> Self {
        self.cooldown_secs = secs;
        self
    }

    /// 检查是否应该运行
    pub fn should_run(&self) -> bool {
        let elapsed = now_secs().saturating_sub(self.last_run);
        elapsed >= self.interval_secs
    }

    /// 运行完整四段式管道
    /// 返回新发现的洞察
    pub fn run(&mut self, fact_store: &FactStore) -> Vec<DiscoveredPattern> {
        self.stage = Stage::Selecting;
        self.last_run = now_secs();
        self.run_count += 1;

        // 清理过期的冷却记录
        let now = now_secs();
        self.seen_patterns
            .retain(|_, last_seen| now.saturating_sub(*last_seen) < self.cooldown_secs * 2);

        // S1: Select — 选取近期事实
        let recent = self.select(fact_store);
        if recent.is_empty() {
            self.stage = Stage::Idle;
            return Vec::new();
        }

        // S2: Replay — 按时间线重放
        self.stage = Stage::Replaying;
        let timeline = self.replay(&recent);

        // S3: Analyze — 发现模式
        self.stage = Stage::Analyzing;
        let patterns = self.analyze(&timeline);
        self.patterns_found += patterns.len() as u64;

        // S4: Update — 过滤已报告的模式 + 持久化回写
        self.stage = Stage::Updating;
        let insights = self.update(&patterns, fact_store);

        // 存储最新发现供 ACK 合成消费
        self.last_patterns = insights.clone();

        self.stage = Stage::Idle;
        insights
    }

    /// S1: Select — : 按主体列表全量扫描（替代 4 个硬编码关键词）
    fn select(&self, fact_store: &FactStore) -> Vec<Fact> {
        let now = now_secs();
        let cutoff = now.saturating_sub(7 * 24 * 3600);
        let mut all = Vec::new();
        let mut seen: HashMap<String, bool> = HashMap::new();

        for subject in &self.selector_subjects {
            if let Ok(facts) = fact_store.query_by_subject(subject) {
                for f in facts {
                    if f.created_at >= cutoff && seen.insert(f.canonical_form(), true).is_none() {
                        all.push(f);
                    }
                }
            }
        }
        all
    }

    /// S2: Replay — 按时间线排列
    fn replay(&self, facts: &[Fact]) -> Vec<TimelineEntry> {
        let mut timeline: Vec<TimelineEntry> = facts
            .iter()
            .map(|f| TimelineEntry {
                timestamp: f.created_at,
                subject: f.subject.clone(),
                predicate: f.predicate.clone(),
                object: f.object.clone(),
                confidence: f.confidence,
            })
            .collect();
        timeline.sort_by_key(|e| e.timestamp);
        timeline
    }

    /// S3: Analyze — 发现模式
    fn analyze(&self, timeline: &[TimelineEntry]) -> Vec<AnalyzedPattern> {
        let mut patterns = Vec::new();

        // 模式 1: 高频主语-谓语对（同一 SPO 出现 ≥3 次）
        let mut spo_counts: HashMap<String, u32> = HashMap::new();
        for entry in timeline {
            let key = format!("{}|{}|{}", entry.subject, entry.predicate, entry.object);
            *spo_counts.entry(key).or_default() += 1;
        }
        for (key, count) in &spo_counts {
            if *count >= 3 {
                patterns.push(AnalyzedPattern {
                    kind: PatternKind::FrequentFact,
                    description: format!("高频事实(出现{}次): {}", count, key),
                    confidence: (*count as f64 / timeline.len() as f64).min(1.0),
                });
            }
        }

        // 模式 2: 同主语的多谓语聚类（同一实体有 ≥4 种不同谓语）
        let mut subject_predicates: HashMap<String, Vec<String>> = HashMap::new();
        for entry in timeline {
            subject_predicates
                .entry(entry.subject.clone())
                .or_default()
                .push(entry.predicate.clone());
        }
        for (subject, preds) in &subject_predicates {
            let unique: std::collections::HashSet<&String> = preds.iter().collect();
            if unique.len() >= 4 {
                let mut sorted: Vec<&&String> = unique.iter().collect();
                sorted.sort();
                patterns.push(AnalyzedPattern {
                    kind: PatternKind::EntityCluster,
                    description: format!(
                        "实体 {} 涉及{}种关系: {:?}",
                        subject,
                        sorted.len(),
                        sorted
                    ),
                    confidence: 0.7,
                });
            }
        }

        // 模式 3: 情感趋势（同类谓语置信度变化）
        if timeline.len() >= 5 {
            let recent_avg: f64 = timeline
                .iter()
                .rev()
                .take(5)
                .map(|e| e.confidence)
                .sum::<f64>()
                / 5.0;
            let early_avg: f64 = timeline.iter().take(5).map(|e| e.confidence).sum::<f64>() / 5.0;
            if (recent_avg - early_avg).abs() > 0.2 {
                let trend = if recent_avg > early_avg {
                    "上升"
                } else {
                    "下降"
                };
                patterns.push(AnalyzedPattern {
                    kind: PatternKind::ConfidenceTrend,
                    description: format!(
                        "置信度趋势{}: {:.2} → {:.2}",
                        trend, early_avg, recent_avg
                    ),
                    confidence: 0.6,
                });
            }
        }

        // 模式 4: 时间聚集（短时间内密集出现的事实）
        if timeline.len() >= 4 {
            let mut window_start = 0;
            for i in 1..timeline.len() {
                let gap = timeline[i]
                    .timestamp
                    .saturating_sub(timeline[window_start].timestamp);
                if gap > 3600 {
                    // 超过 1 小时，重置窗口
                    window_start = i;
                } else if i - window_start >= 3 {
                    // 1 小时内 ≥ 4 条事实 → 密集对话
                    patterns.push(AnalyzedPattern {
                        kind: PatternKind::TemporalCluster,
                        description: format!(
                            "密集对话: {}分钟内{}条相关事实",
                            gap / 60,
                            i - window_start + 1
                        ),
                        confidence: 0.65,
                    });
                    window_start = i + 1;
                }
            }
        }

        patterns
    }

    /// S4: Update — : 过滤已报告模式 + 持久化回写到 FactStore
    fn update(
        &mut self,
        patterns: &[AnalyzedPattern],
        fact_store: &FactStore,
    ) -> Vec<DiscoveredPattern> {
        let now = now_secs();
        let mut new_insights = Vec::new();

        for p in patterns.iter().filter(|p| p.confidence > 0.5) {
            // 冷却去重：同一摘要在冷却期内不重复报告
            if let Some(last_seen) = self.seen_patterns.get(&p.description) {
                if now.saturating_sub(*last_seen) < self.cooldown_secs {
                    continue;
                }
            }

            self.seen_patterns.insert(p.description.clone(), now);

            // 将发现的模式作为 meta-fact 持久化到 FactStore
            let meta_subject = match p.kind {
                PatternKind::FrequentFact => "replay:freq",
                PatternKind::EntityCluster => "replay:cluster",
                PatternKind::ConfidenceTrend => "replay:trend",
                PatternKind::TemporalCluster => "replay:temporal",
            };
            let meta_fact = Fact::new(meta_subject, "discovered", &p.description)
                .with_confidence(p.confidence)
                .with_source("replay_pipeline");
            let _ = fact_store.insert(meta_fact);

            new_insights.push(DiscoveredPattern {
                summary: p.description.clone(),
                kind: p.kind,
                confidence: p.confidence,
                discovered_at: now,
            });
        }

        new_insights
    }

    pub fn stats(&self) -> PipelineStats {
        PipelineStats {
            stage: self.stage,
            run_count: self.run_count,
            patterns_found: self.patterns_found,
            last_run: self.last_run,
            seen_count: self.seen_patterns.len(),
            cooldown_secs: self.cooldown_secs,
        }
    }

    /// 获取最近一次运行发现的模式（供 ACK 合成消费）
    pub fn recent_patterns(&self) -> &[DiscoveredPattern] {
        &self.last_patterns
    }

    /// 标记某个模式已被转化为 ACK（防止重复转化）
    pub fn mark_consumed(&mut self, summary: &str) {
        self.consumed_patterns.insert(summary.to_string());
    }

    /// 检查某个模式是否已被消费
    pub fn is_consumed(&self, summary: &str) -> bool {
        self.consumed_patterns.contains(summary)
    }
}

/// 时间线条目
#[derive(Debug, Clone)]
struct TimelineEntry {
    timestamp: u64,
    subject: String,
    predicate: String,
    object: String,
    confidence: f64,
}

/// 模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    FrequentFact,
    EntityCluster,
    ConfidenceTrend,
    /// 时间聚集模式
    TemporalCluster,
}

/// 分析出的模式
#[derive(Debug, Clone)]
struct AnalyzedPattern {
    kind: PatternKind,
    description: String,
    confidence: f64,
}

/// 发现的模式（对外输出）
#[derive(Debug, Clone)]
pub struct DiscoveredPattern {
    pub summary: String,
    pub kind: PatternKind,
    pub confidence: f64,
    pub discovered_at: u64,
}

/// 管道统计
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub stage: Stage,
    pub run_count: u64,
    pub patterns_found: u64,
    pub last_run: u64,
    /// 已冷却去重的模式数
    pub seen_count: usize,
    /// 冷却时间（秒）
    pub cooldown_secs: u64,
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

    fn test_store() -> FactStore {
        let s = FactStore::new("").unwrap();
        let facts = vec![
            Fact::new("主人", "喜欢", "Rust").with_confidence(0.9),
            Fact::new("主人", "喜欢", "AI").with_confidence(0.85),
            Fact::new("主人", "喜欢", "编程").with_confidence(0.8),
            Fact::new("主人", "知道", "tokio").with_confidence(0.7),
            Fact::new("主人", "在", "杭州").with_confidence(0.95),
            Fact::new("主人", "想", "学深度学习").with_confidence(0.75),
        ];
        for f in &facts {
            s.insert(f.clone()).unwrap();
        }
        s
    }

    #[test]
    fn test_pipeline_run() {
        let store = test_store();
        let mut pipeline = ReplayPipeline::new();
        let patterns = pipeline.run(&store);
        assert!(!patterns.is_empty(), "应发现至少1个模式");
    }

    #[test]
    fn test_should_run() {
        let mut p = ReplayPipeline::new().with_interval(1);
        assert!(p.should_run(), "初始应可运行");
        p.run(&FactStore::new("").unwrap());
        assert!(!p.should_run(), "刚跑完不应再跑");
    }

    #[test]
    fn test_stats() {
        let store = test_store();
        let mut pipeline = ReplayPipeline::new();
        pipeline.run(&store);
        let stats = pipeline.stats();
        assert_eq!(stats.stage, Stage::Idle);
        assert_eq!(stats.run_count, 1);
        assert!(stats.patterns_found > 0);
    }

    #[test]
    fn test_cooldown_dedup() {
        let store = test_store();
        let mut pipeline = ReplayPipeline::new().with_cooldown(3600);
        let first = pipeline.run(&store);
        let second = pipeline.run(&store);
        // 第二次运行应被冷却过滤，不重复报告
        assert!(
            second.is_empty(),
            "冷却期内不应重复报告: first={}, second={}",
            first.len(),
            second.len()
        );
        assert!(pipeline.stats().seen_count > 0, "应有去重记录");
    }

    #[test]
    fn test_selector_broad_scan() {
        let store = test_store();
        let pipeline = ReplayPipeline::new();
        // select 应使用主体列表而非硬编码关键词
        let facts = pipeline.select(&store);
        assert!(!facts.is_empty(), "主体 '主人' 应有事实");
    }

    #[test]
    fn test_updater_persists_to_factstore() {
        let store = test_store();
        let mut pipeline = ReplayPipeline::new();
        let patterns = pipeline.run(&store);
        if !patterns.is_empty() {
            // 检查 FactStore 中是否有 replay: 前缀的 meta-fact
            let meta = store
                .query_by_subject("replay:freq")
                .unwrap_or_default()
                .into_iter()
                .chain(store.query_by_subject("replay:cluster").unwrap_or_default())
                .chain(store.query_by_subject("replay:trend").unwrap_or_default())
                .chain(
                    store
                        .query_by_subject("replay:temporal")
                        .unwrap_or_default(),
                )
                .collect::<Vec<_>>();
            assert!(
                !meta.is_empty(),
                "发现的模式应作为 meta-fact 持久化到 FactStore"
            );
        }
    }

    #[test]
    fn test_with_subjects() {
        let store = test_store();
        let pipeline = ReplayPipeline::new().with_subjects(vec!["不存在".into()]);
        let facts = pipeline.select(&store);
        assert!(facts.is_empty(), "不存在的主体应返回空");
    }

    // ─── : ACK 自学习集成测试 ──────────────────────────

    #[test]
    fn test_recent_patterns_populated_after_run() {
        let store = test_store();
        let mut pipeline = ReplayPipeline::new();
        let _patterns = pipeline.run(&store);
        // run() 应将发现的模式存入 last_patterns
        let recent = pipeline.recent_patterns();
        assert!(!recent.is_empty(), "run() 后 recent_patterns 应非空");
    }

    #[test]
    fn test_mark_consumed() {
        let mut pipeline = ReplayPipeline::new();
        assert!(!pipeline.is_consumed("test_summary"));
        pipeline.mark_consumed("test_summary");
        assert!(pipeline.is_consumed("test_summary"));
        assert!(!pipeline.is_consumed("other_summary"));
    }
}
