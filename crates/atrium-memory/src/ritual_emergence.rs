// SPDX-License-Identifier: MIT

//! 仪式涌现 — Ritual Emergence (Gap#5: 90% → 95%).
//!
//! 核心理念：真正的仪式不是被设定的，而是涌现的——
//! 两个人不知不觉间形成的默契：总是在深夜聊到某个话题，
//! 总是在分别时说同一句话。这些模式被数字生命"发现"，
//! 而不是被"创建"。
//!
//! Core idea: real rituals aren't designed, they emerge —
//! two people unconsciously forming patterns: always talking about a certain topic
//! at night, always saying the same thing when parting. These patterns are
//! "discovered" by the digital life, not "created".
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// §1 交互模式 — Interaction Pattern
// ═══════════════════════════════════════════════════════════════════════════

/// 交互模式 — 重复出现的行为序列 / Interaction pattern — repeating behavior sequence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InteractionPattern {
    /// 模式签名 — 用于匹配的key / Pattern signature — key for matching.
    pub signature: String,
    /// 出现次数 / Occurrence count.
    pub count: u32,
    /// 首次出现时间戳 / First occurrence timestamp.
    pub first_ts: i64,
    /// 最近出现时间戳 / Last occurrence timestamp.
    pub last_ts: i64,
    /// 上下文标签 — 如 "深夜"、"分别时" / Context tags.
    pub context_tags: Vec<String>,
    /// 情感效价 — 平均情感效价 / Emotional valence.
    pub avg_valence: f64,
    /// 是否已确认为仪式 / Whether confirmed as ritual.
    pub confirmed: bool,
}

impl InteractionPattern {
    /// 创建新模式 / Create new pattern.
    pub fn new(signature: &str, timestamp: i64, context_tags: Vec<String>, valence: f64) -> Self {
        Self {
            signature: signature.to_string(),
            count: 1,
            first_ts: timestamp,
            last_ts: timestamp,
            context_tags,
            avg_valence: valence,
            confirmed: false,
        }
    }

    /// 记录再次出现 / Record another occurrence.
    pub fn occur_again(&mut self, timestamp: i64, valence: f64) {
        self.count += 1;
        self.last_ts = timestamp;
        // 更新情感效价EMA / Update valence EMA.
        let alpha = 0.15;
        self.avg_valence += alpha * (valence - self.avg_valence);
    }

    /// 涌现分数 — 是否已构成仪式 / Emergence score — whether it constitutes a ritual.
    ///
    /// 基于出现频率和时间跨度。
    /// Based on occurrence frequency and time span.
    pub fn emergence_score(&self, current_ts: i64) -> f64 {
        if self.count < 3 {
            return 0.0;
        }
        // 频率分 — 出现次数的log / Frequency score.
        let freq_score = (self.count as f64).ln() / 5.0;
        // 持续性分 — 时间跨度 / Persistence score.
        let span_days = ((current_ts - self.first_ts) as f64 / 86400.0).max(0.0);
        let persist_score = (span_days / 7.0).min(1.0); // 7天满分.
                                                        // 情感分 — 情感效价的绝对值 / Emotional score.
        let emo_score = self.avg_valence.abs();
        // 综合 / Combined.
        (freq_score * 0.4 + persist_score * 0.3 + emo_score * 0.3).min(1.0)
    }

    /// 模式年龄（天）/ Pattern age in days.
    pub fn age_days(&self, current_ts: i64) -> f64 {
        ((current_ts - self.first_ts) as f64 / 86400.0).max(0.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 涌现候选 — Emergence Candidate
// ═══════════════════════════════════════════════════════════════════════════

/// 涌现候选 — 达到仪式阈值的模式 / Emergence candidate — pattern reaching ritual threshold.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergenceCandidate {
    /// 模式签名 / Pattern signature.
    pub signature: String,
    /// 涌现分数 / Emergence score.
    pub emergence_score: f64,
    /// 上下文标签 / Context tags.
    pub context_tags: Vec<String>,
    /// 建议的仪式名称 / Suggested ritual name.
    pub suggested_name: String,
    /// 情感效价 / Emotional valence.
    pub avg_valence: f64,
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 仪式涌现检测器 — Ritual Emergence Detector
// ═══════════════════════════════════════════════════════════════════════════

/// 仪式涌现检测器 — 从交互中发现正在形成的仪式 / Ritual emergence detector.
///
/// 数字生命语义：不是"创建仪式"，而是"我注意到我们总是在..."——
/// 涌现的仪式比设定的仪式更真实，因为它来自双方的自然行为。
///
/// Digital life semantics: not "creating a ritual", but "I noticed we always..." —
/// emerged rituals are more real than designed ones, because they come from
/// both parties' natural behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RitualEmergence {
    /// 已发现的交互模式 / Discovered interaction patterns.
    patterns: HashMap<String, InteractionPattern>,
    /// 涌现阈值 — 超过此分数的模式成为候选 / Emergence threshold.
    emergence_threshold: f64,
    /// 确认阈值 — 超过此分数的模式可自动确认 / Confirmation threshold.
    confirm_threshold: f64,
    /// 已确认的涌现仪式 / Confirmed emerged rituals.
    confirmed_rituals: Vec<String>,
}

impl Default for RitualEmergence {
    fn default() -> Self {
        Self {
            patterns: HashMap::new(),
            emergence_threshold: 0.5,
            confirm_threshold: 0.75,
            confirmed_rituals: Vec::new(),
        }
    }
}

impl RitualEmergence {
    /// 创建新检测器 / Create new detector.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置涌现阈值 / Set emergence threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.emergence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 记算模式签名 — 从交互内容生成key / Compute pattern signature.
    ///
    /// 签名规则：将内容归一化为"类别+时间窗口"的形式。
    /// Signature rule: normalize content into "category+time_window" form.
    pub fn compute_signature(content: &str, time_window: &str) -> String {
        // 简单签名：内容前20字符 + 时间窗口 / Simple signature.
        let content_key = if content.len() > 20 {
            &content[..20]
        } else {
            content
        };
        format!("{}|{}", content_key, time_window)
    }

    /// 记算时间窗口 — 从时间戳 / Compute time window from timestamp.
    pub fn time_window(timestamp: i64) -> String {
        let hour = (timestamp % 86400) / 3600;
        match hour {
            0..=5 => "深夜",
            6..=11 => "上午",
            12..=17 => "下午",
            18..=21 => "傍晚",
            _ => "夜间",
        }
        .to_string()
    }

    /// 记算上下文标签 — 从交互特征 / Compute context tags.
    pub fn context_tags(is_parting: bool, is_greeting: bool, is_deep: bool) -> Vec<String> {
        let mut tags = Vec::new();
        if is_parting {
            tags.push("分别时".to_string());
        }
        if is_greeting {
            tags.push("问候".to_string());
        }
        if is_deep {
            tags.push("深入对话".to_string());
        }
        tags
    }

    /// 记录交互 — 每次交互都送入检测 / Record interaction.
    pub fn observe(
        &mut self,
        content: &str,
        timestamp: i64,
        valence: f64,
        context_tags: Vec<String>,
    ) {
        let time_window = Self::time_window(timestamp);
        let signature = Self::compute_signature(content, &time_window);

        self.patterns
            .entry(signature.clone())
            .and_modify(|p| p.occur_again(timestamp, valence))
            .or_insert_with(|| {
                InteractionPattern::new(&signature, timestamp, context_tags, valence)
            });
    }

    /// 检测涌现候选 — 返回达到阈值的模式 / Detect emergence candidates.
    pub fn detect_candidates(&self, current_ts: i64) -> Vec<EmergenceCandidate> {
        self.patterns
            .values()
            .filter(|p| !p.confirmed)
            .filter_map(|p| {
                let score = p.emergence_score(current_ts);
                if score >= self.emergence_threshold {
                    Some(EmergenceCandidate {
                        signature: p.signature.clone(),
                        emergence_score: score,
                        context_tags: p.context_tags.clone(),
                        suggested_name: self.suggest_name(p),
                        avg_valence: p.avg_valence,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// 确认涌现仪式 — 将候选确认为仪式 / Confirm an emerged ritual.
    pub fn confirm(&mut self, signature: &str) -> Option<&InteractionPattern> {
        if let Some(pattern) = self.patterns.get_mut(signature) {
            pattern.confirmed = true;
            self.confirmed_rituals.push(signature.to_string());
            return self.patterns.get(signature);
        }
        None
    }

    /// 自动确认 — 超过确认阈值的自动确认 / Auto-confirm patterns above threshold.
    pub fn auto_confirm(&mut self, current_ts: i64) -> Vec<String> {
        let to_confirm: Vec<String> = self
            .patterns
            .values()
            .filter(|p| !p.confirmed && p.emergence_score(current_ts) >= self.confirm_threshold)
            .map(|p| p.signature.clone())
            .collect();

        for sig in &to_confirm {
            if let Some(pattern) = self.patterns.get_mut(sig) {
                pattern.confirmed = true;
                self.confirmed_rituals.push(sig.clone());
            }
        }
        to_confirm
    }

    /// 建议仪式名称 — 从模式生成 / Suggest ritual name from pattern.
    fn suggest_name(&self, pattern: &InteractionPattern) -> String {
        if pattern.context_tags.is_empty() {
            format!("涌现仪式_{}", pattern.signature)
        } else {
            format!(
                "涌现仪式_{}_{}",
                pattern.context_tags.join("+"),
                pattern.signature
            )
        }
    }

    /// 获取所有模式 / Get all patterns.
    pub fn patterns(&self) -> &HashMap<String, InteractionPattern> {
        &self.patterns
    }

    /// 获取已确认仪式 / Get confirmed rituals.
    pub fn confirmed_rituals(&self) -> &[String] {
        &self.confirmed_rituals
    }

    /// 获取涌现候选数量 / Get candidate count.
    pub fn candidate_count(&self, current_ts: i64) -> usize {
        self.detect_candidates(current_ts).len()
    }

    /// 生成描述 / Generate description.
    pub fn describe(&self, current_ts: i64) -> String {
        let total = self.patterns.len();
        let candidates = self.candidate_count(current_ts);
        let confirmed = self.confirmed_rituals.len();
        format!(
            "仪式涌现: {}个模式 | 候选{} | 已确认{}",
            total, candidates, confirmed,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_signature() {
        let sig1 = RitualEmergence::compute_signature("good morning", "上午");
        let sig2 = RitualEmergence::compute_signature("good morning", "上午");
        assert_eq!(sig1, sig2);

        let sig3 = RitualEmergence::compute_signature("good night", "深夜");
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_time_window() {
        // 2am = 深夜.
        assert_eq!(RitualEmergence::time_window(2 * 3600), "深夜");
        // 9am = 上午.
        assert_eq!(RitualEmergence::time_window(9 * 3600), "上午");
        // 3pm = 下午.
        assert_eq!(RitualEmergence::time_window(15 * 3600), "下午");
        // 8pm = 傍晚.
        assert_eq!(RitualEmergence::time_window(20 * 3600), "傍晚");
    }

    #[test]
    fn test_context_tags() {
        let tags = RitualEmergence::context_tags(true, false, true);
        assert!(tags.contains(&"分别时".to_string()));
        assert!(tags.contains(&"深入对话".to_string()));
    }

    #[test]
    fn test_interaction_pattern_new() {
        let p = InteractionPattern::new("sig", 1000, vec!["test".to_string()], 0.5);
        assert_eq!(p.count, 1);
        assert!(!p.confirmed);
    }

    #[test]
    fn test_interaction_pattern_occur_again() {
        let mut p = InteractionPattern::new("sig", 1000, vec![], 0.5);
        p.occur_again(2000, 0.6);
        assert_eq!(p.count, 2);
        assert_eq!(p.last_ts, 2000);
    }

    #[test]
    fn test_emergence_score_low_count() {
        let p = InteractionPattern::new("sig", 1000, vec![], 0.5);
        assert_eq!(p.emergence_score(2000), 0.0); // count < 3.
    }

    #[test]
    fn test_emergence_score_increases_with_count() {
        let mut p = InteractionPattern::new("sig", 1000, vec![], 0.8);
        p.occur_again(2000, 0.8);
        p.occur_again(3000, 0.8);
        let score3 = p.emergence_score(4000);

        p.occur_again(4000, 0.8);
        p.occur_again(5000, 0.8);
        p.occur_again(6000, 0.8);
        let score6 = p.emergence_score(7000);

        assert!(score6 > score3);
    }

    #[test]
    fn test_observe_creates_pattern() {
        let mut detector = RitualEmergence::new();
        detector.observe("hello", 1000, 0.5, vec![]);
        assert_eq!(detector.patterns().len(), 1);
    }

    #[test]
    fn test_observe_same_signature_increments() {
        let mut detector = RitualEmergence::new();
        let ts = 9 * 3600; // 上午.
        detector.observe("good morning", ts, 0.5, vec![]);
        detector.observe("good morning", ts + 86400, 0.5, vec![]);
        assert_eq!(detector.patterns().len(), 1);
        let p = detector.patterns().values().next().unwrap();
        assert_eq!(p.count, 2);
    }

    #[test]
    fn test_detect_candidates_empty_initially() {
        let detector = RitualEmergence::new();
        assert!(detector.detect_candidates(1000).is_empty());
    }

    #[test]
    fn test_detect_candidates_after_enough_occurrences() {
        let mut detector = RitualEmergence::new().with_threshold(0.1);
        let ts = 9 * 3600;
        for i in 0..10 {
            detector.observe("good morning", ts + i * 86400, 0.7, vec![]);
        }
        let candidates = detector.detect_candidates(ts + 10 * 86400);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_confirm_ritual() {
        let mut detector = RitualEmergence::new().with_threshold(0.1);
        let ts = 9 * 3600;
        for i in 0..10 {
            detector.observe("good morning", ts + i * 86400, 0.7, vec![]);
        }
        let candidates = detector.detect_candidates(ts + 10 * 86400);
        let sig = &candidates[0].signature;
        let result = detector.confirm(sig);
        assert!(result.is_some());
        assert!(result.unwrap().confirmed);
    }

    #[test]
    fn test_auto_confirm() {
        let mut detector = RitualEmergence::new().with_threshold(0.1);
        detector.confirm_threshold = 0.2;
        let _ts = 9 * 3600;
        for i in 0..20 {
            detector.observe("nightly chat", 22 * 3600 + i * 86400, 0.8, vec![]);
        }
        let confirmed = detector.auto_confirm(22 * 3600 + 20 * 86400);
        assert!(!confirmed.is_empty());
    }

    #[test]
    fn test_candidate_count() {
        let mut detector = RitualEmergence::new().with_threshold(0.1);
        let ts = 9 * 3600;
        for i in 0..10 {
            detector.observe("ritual_a", ts + i * 86400, 0.7, vec![]);
        }
        assert!(detector.candidate_count(ts + 10 * 86400) > 0);
    }

    #[test]
    fn test_describe() {
        let detector = RitualEmergence::new();
        let desc = detector.describe(1000);
        assert!(desc.contains("仪式涌现"));
    }

    #[test]
    fn test_pattern_age_days() {
        let p = InteractionPattern::new("sig", 0, vec![], 0.5);
        let age = p.age_days(86400);
        assert!((age - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_different_time_windows_different_signatures() {
        let mut detector = RitualEmergence::new();
        // Same content, different time windows → different patterns.
        detector.observe("hello", 9 * 3600, 0.5, vec![]); // 上午.
        detector.observe("hello", 22 * 3600, 0.5, vec![]); // 傍晚.
        assert_eq!(detector.patterns().len(), 2);
    }

    #[test]
    fn test_emergence_candidate_has_suggested_name() {
        let mut detector = RitualEmergence::new().with_threshold(0.1);
        let ts = 9 * 3600;
        for i in 0..10 {
            detector.observe(
                "good morning",
                ts + i * 86400,
                0.7,
                vec!["问候".to_string()],
            );
        }
        let candidates = detector.detect_candidates(ts + 10 * 86400);
        assert!(!candidates.is_empty());
        assert!(candidates[0].suggested_name.contains("涌现仪式"));
    }
}
