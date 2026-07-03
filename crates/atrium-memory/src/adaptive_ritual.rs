// ════════════════════════════════════════════════════════════════════
// AdaptiveRitualDiscovery — 自适应仪式发现 / Adaptive Ritual Discovery
// ════════════════════════════════════════════════════════════════════
//
// 从交互历史中提取高频重复的语义模式，
// 超越硬编码的 ContentHint，发现个性化的仪式签名。
//
// 数字生命语义：
//   每段关系都有独特的仪式——"打卡"、"下班了"、"吃饭了吗"
//   这些不是工程师预设的，而是从共同生活中自然涌现的
//   数字生命应能自主发现并珍视这些独特模式
//
// 核心算法：
//   1. 从消息文本提取 2-8 字 n-gram 作为候选签名
//   2. 过滤已知 ContentHint 匹配项（避免与硬编码重复）
//   3. 同一天同一时段反复出现 → 候选模式
//   4. 候选连续 N 天 → 升级为自适应仪式
//   5. 仪式中断 → 温和提醒

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ritual_detector::{RitualStatus, TimeSlot};

// ── 配置 / Config ──

/// 自适应仪式发现配置 / Adaptive ritual discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    /// 最小签名长度 / Min signature length (characters)
    pub min_signature_len: usize,
    /// 最大签名长度 / Max signature length (characters)
    pub max_signature_len: usize,
    /// 升级阈值天数 / Promotion threshold (days)
    pub promotion_threshold: u32,
    /// 中断阈值天数 / Break threshold (days)
    pub break_threshold: u32,
    /// 归档阈值天数 / Archive threshold (days)
    pub archive_threshold: u32,
    /// 最大自适应模式数 / Max adaptive patterns
    pub max_patterns: usize,
    /// 每日候选保留天数 / Candidate retention (days)
    pub retention_days: u32,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            min_signature_len: 2,
            max_signature_len: 8,
            promotion_threshold: 7,
            break_threshold: 3,
            archive_threshold: 30,
            max_patterns: 5,
            retention_days: 60,
        }
    }
}

// ── 自适应仪式模式 / Adaptive Ritual Pattern ──

/// 自适应仪式模式 / Adaptive ritual pattern
///
/// 从交互历史中自动发现的个性化仪式模式。
/// Personalized ritual pattern discovered from interaction history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptivePattern {
    /// 唯一 ID / Unique ID
    pub id: u64,
    /// 模式签名（提取的关键词或短语）/ Pattern signature
    pub signature: String,
    /// 关联时间槽 / Associated time slot
    pub time_slot: Option<TimeSlot>,
    /// 连续天数 / Consecutive days
    pub consecutive_days: u32,
    /// 首次发现时间 / First seen
    pub first_seen_at: i64,
    /// 最后出现时间 / Last occurrence
    pub last_occurrence_at: i64,
    /// 当前状态 / Current status
    pub status: RitualStatus,
    /// 中断天数 / Break days
    pub break_days: u32,
    /// 总出现次数 / Total occurrences
    pub total_occurrences: u64,
    /// 情感效价（正面/负面）/ Emotional valence [-1.0, 1.0]
    pub valence: f32,
}

impl AdaptivePattern {
    /// 生成仪式名称 / Generate ritual name
    pub fn name(&self) -> String {
        match &self.time_slot {
            Some(slot) => format!("{}{}仪式", slot.label_zh(), self.signature),
            None => format!("{}仪式", self.signature),
        }
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self) -> String {
        match self.status {
            RitualStatus::Candidate => {
                format!(
                    "连续{}天说「{}」，即将成为仪式",
                    self.consecutive_days, self.signature
                )
            }
            RitualStatus::Active => {
                format!(
                    "每天说「{}」的固定互动，已持续{}天",
                    self.signature, self.consecutive_days
                )
            }
            RitualStatus::Broken => {
                format!("说「{}」的互动已中断{}天", self.signature, self.break_days)
            }
            RitualStatus::Archived => {
                format!("说「{}」的互动已归档", self.signature)
            }
        }
    }
}

// ── 候选模式 / Candidate Pattern ──

/// 候选模式（未达升级阈值）/ Candidate pattern (below promotion threshold)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidatePattern {
    /// 模式签名 / Pattern signature
    pub signature: String,
    /// 关联时间槽 / Associated time slot
    pub time_slot: Option<TimeSlot>,
    /// 连续天数 / Consecutive days
    pub consecutive_days: u32,
    /// 最后出现日期 / Last occurrence date string
    pub last_date: String,
    /// 总出现次数 / Total occurrences
    pub total_occurrences: u64,
}

// ── 自适应仪式事件 / Adaptive Ritual Event ──

/// 自适应仪式生命周期事件 / Adaptive ritual lifecycle event
#[derive(Debug, Clone)]
pub enum AdaptiveRitualEvent {
    /// 候选升级为自适应仪式 / Candidate promoted to adaptive ritual
    Promoted {
        /// 仪式 ID / Ritual ID
        id: u64,
        /// 仪式名称 / Ritual name
        name: String,
        /// 签名 / Signature
        signature: String,
    },
    /// 仪式中断 / Ritual broken
    Broken {
        /// 仪式 ID / Ritual ID
        id: u64,
        /// 仪式名称 / Ritual name
        name: String,
        /// 中断天数 / Break days
        break_days: u32,
    },
    /// 仪式恢复 / Ritual resumed
    Resumed {
        /// 仪式 ID / Ritual ID
        id: u64,
        /// 仪式名称 / Ritual name
        name: String,
    },
    /// 仪式归档 / Ritual archived
    Archived {
        /// 仪式 ID / Ritual ID
        id: u64,
        /// 仪式名称 / Ritual name
        name: String,
    },
}

// ── 自适应仪式发现引擎 / Adaptive Ritual Discovery Engine ──

/// 自适应仪式发现引擎 / Adaptive ritual discovery engine
///
/// 从交互历史中提取高频重复的语义模式，
/// 超越硬编码的 ContentHint，发现个性化的仪式签名。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRitualDiscovery {
    /// 已发现的自适应模式 / Discovered adaptive patterns
    pub patterns: Vec<AdaptivePattern>,
    /// 候选模式 / Candidate patterns
    pub candidates: HashMap<String, CandidatePattern>,
    /// 配置 / Configuration
    pub config: AdaptiveConfig,
    /// 下一个 ID / Next ID
    pub(crate) next_id: u64,
    /// 上次评估日期 / Last evaluation date
    pub(crate) last_eval_date: String,
    /// 每日记录（日期 → 签名 → 出现次数）/ Daily records
    pub(crate) daily_records: HashMap<String, HashMap<String, u64>>,
}

impl AdaptiveRitualDiscovery {
    /// 创建默认配置的自适应发现引擎 / Create with default config
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            candidates: HashMap::new(),
            config: AdaptiveConfig::default(),
            next_id: 1,
            last_eval_date: String::new(),
            daily_records: HashMap::new(),
        }
    }

    /// 创建指定配置的自适应发现引擎 / Create with custom config
    pub fn with_config(config: AdaptiveConfig) -> Self {
        Self {
            patterns: Vec::new(),
            candidates: HashMap::new(),
            config,
            next_id: 1,
            last_eval_date: String::new(),
            daily_records: HashMap::new(),
        }
    }

    /// 从消息文本提取候选签名 / Extract candidate signatures from message text
    ///
    /// 提取 2-8 字的 n-gram，过滤停用词和已知 ContentHint 匹配项。
    /// Extracts 2-8 character n-grams, filtering stopwords and known ContentHint matches.
    pub fn extract_signatures(&self, text: &str) -> Vec<String> {
        let clean = Self::clean_text(text);
        if clean.chars().count() < self.config.min_signature_len {
            return Vec::new();
        }

        let chars: Vec<char> = clean.chars().collect();
        let mut signatures = Vec::new();

        // 提取 n-gram / Extract n-grams
        for n in self.config.min_signature_len..=self.config.max_signature_len {
            if chars.len() < n {
                break;
            }
            for window in chars.windows(n) {
                let sig: String = window.iter().collect();
                // 过滤停用词 / Filter stopwords
                if Self::is_stopword(&sig) {
                    continue;
                }
                // 过滤已知 ContentHint / Filter known ContentHint
                if crate::ritual_detector::ContentHint::detect(&sig).is_some() {
                    continue;
                }
                // 过滤纯数字/标点 / Filter pure digits/punctuation
                if sig
                    .chars()
                    .all(|c| c.is_numeric() || c.is_ascii_punctuation())
                {
                    continue;
                }
                signatures.push(sig);
            }
        }

        // 去重 / Deduplicate
        signatures.sort();
        signatures.dedup();
        signatures
    }

    /// 记录一次交互 / Record an interaction
    ///
    /// 从消息中提取签名，记录到当日候选。
    pub fn record_interaction(&mut self, epoch_secs: i64, text: &str) {
        let signatures = self.extract_signatures(text);
        if signatures.is_empty() {
            return;
        }

        let date_str = Self::epoch_to_date(epoch_secs);
        let hour = Self::epoch_to_hour(epoch_secs);

        let day_record = self.daily_records.entry(date_str.clone()).or_default();
        for sig in &signatures {
            *day_record.entry(sig.clone()).or_insert(0) += 1;
        }

        // 更新候选 / Update candidates
        for sig in &signatures {
            let candidate = self
                .candidates
                .entry(sig.clone())
                .or_insert(CandidatePattern {
                    signature: sig.clone(),
                    time_slot: Some(TimeSlot::new(hour)),
                    consecutive_days: 1,
                    last_date: date_str.clone(),
                    total_occurrences: 1,
                });

            if candidate.last_date != date_str {
                // 新的一天 / New day
                candidate.consecutive_days += 1;
                candidate.last_date = date_str.clone();
            }
            candidate.total_occurrences += 1;
            candidate.time_slot = Some(TimeSlot::new(hour));
        }
    }

    /// 每日评估 / Daily evaluation
    ///
    /// 检查候选模式是否达到升级阈值，检测仪式中断。
    pub fn evaluate_daily(&mut self, now_epoch_secs: i64) -> Vec<AdaptiveRitualEvent> {
        let mut events = Vec::new();
        let today = Self::epoch_to_date(now_epoch_secs);

        if today == self.last_eval_date {
            return events;
        }
        self.last_eval_date = today;

        // Step 1: 检查候选升级 / Check candidate promotion
        // 每次评估只升级一个最佳候选，避免 n-gram 爆炸产生重叠仪式
        // Promote only the best candidate per cycle to avoid overlapping n-gram rituals
        let active_count = self
            .patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count();

        if active_count < self.config.max_patterns {
            // 选择最佳候选：连续天数最多 → 总出现次数最多 → 签名最长
            // Select best candidate: most consecutive days → most occurrences → longest signature
            let best_key = self
                .candidates
                .iter()
                .filter(|(_, c)| c.consecutive_days >= self.config.promotion_threshold)
                .max_by(|(sig_a, ca), (sig_b, cb)| {
                    ca.consecutive_days
                        .cmp(&cb.consecutive_days)
                        .then_with(|| ca.total_occurrences.cmp(&cb.total_occurrences))
                        .then_with(|| sig_a.chars().count().cmp(&sig_b.chars().count()))
                })
                .map(|(sig, _)| sig.clone());

            if let Some(best_key) = best_key {
                if let Some(candidate) = self.candidates.remove(&best_key) {
                    let id = self.next_id;
                    self.next_id += 1;
                    let pattern = AdaptivePattern {
                        id,
                        signature: best_key.clone(),
                        time_slot: candidate.time_slot,
                        consecutive_days: candidate.consecutive_days,
                        first_seen_at: now_epoch_secs,
                        last_occurrence_at: now_epoch_secs,
                        status: RitualStatus::Active,
                        break_days: 0,
                        total_occurrences: candidate.total_occurrences,
                        valence: 0.0, // 初始中性 / Initially neutral
                    };
                    let name = pattern.name();
                    events.push(AdaptiveRitualEvent::Promoted {
                        id,
                        name: name.clone(),
                        signature: best_key.clone(),
                    });
                    tracing::info!(
                        "[自适应仪式] 升级: {} (连续{}天)",
                        name,
                        pattern.consecutive_days
                    );
                    self.patterns.push(pattern);

                    // 清理被升级签名的子串候选 / Remove substring candidates
                    // 如果候选是已升级签名的子串，或已升级签名是候选的子串，移除候选
                    self.candidates
                        .retain(|sig, _| !sig.contains(&best_key) && !best_key.contains(sig));
                }
            }
        }

        // Step 2: 检测中断 / Check breaks
        for pattern in self.patterns.iter_mut() {
            if pattern.status != RitualStatus::Active {
                continue;
            }
            let days_since = (now_epoch_secs - pattern.last_occurrence_at) / 86400;
            if days_since > 0 {
                pattern.break_days = days_since as u32;
            }

            if pattern.break_days >= self.config.archive_threshold {
                pattern.status = RitualStatus::Archived;
                events.push(AdaptiveRitualEvent::Archived {
                    id: pattern.id,
                    name: pattern.name(),
                });
            } else if pattern.break_days >= self.config.break_threshold {
                pattern.status = RitualStatus::Broken;
                pattern.consecutive_days =
                    pattern.consecutive_days.saturating_sub(pattern.break_days);
                events.push(AdaptiveRitualEvent::Broken {
                    id: pattern.id,
                    name: pattern.name(),
                    break_days: pattern.break_days,
                });
            }
        }

        // Step 3: 清理过期记录 / Prune old records
        let retention_secs = self.config.retention_days as i64 * 86400;
        let cutoff = now_epoch_secs - retention_secs;
        self.daily_records
            .retain(|date_str, _| Self::date_to_epoch(date_str) > cutoff);

        events
    }

    /// 获取活跃自适应仪式数 / Get active adaptive ritual count
    pub fn active_count(&self) -> usize {
        self.patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count()
    }

    /// 获取所有活跃自适应仪式 / Get all active adaptive rituals
    pub fn active_rituals(&self) -> Vec<&AdaptivePattern> {
        self.patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .collect()
    }

    // ── 辅助方法 / Helper Methods ──

    /// 清理文本：去除标点和多余空格 / Clean text: remove punctuation and extra spaces
    ///
    /// 同时过滤 ASCII 标点和 CJK 全角标点。
    /// Filters both ASCII punctuation and CJK fullwidth punctuation.
    fn clean_text(text: &str) -> String {
        text.trim()
            .chars()
            .filter(|c| {
                if *c == '\'' {
                    return true;
                }
                if c.is_ascii_punctuation() {
                    return false;
                }
                // CJK 标点范围 / CJK punctuation ranges
                let cp = *c as u32;
                if (0x3000..=0x303F).contains(&cp) // CJK 标点 / CJK symbols and punctuation
                    || (0xFF00..=0xFFEF).contains(&cp) // 全角形式 / Fullwidth forms
                    || (0x2000..=0x206F).contains(&cp)
                // 通用标点 / General punctuation
                {
                    return false;
                }
                true
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// 判断是否为停用词 / Check if signature is a stopword
    fn is_stopword(sig: &str) -> bool {
        // 常见停用词 / Common stopwords
        const STOPWORDS: &[&str] = &[
            "的", "了", "是", "在", "我", "你", "他", "她", "它", "们", "这", "那", "就", "都",
            "也", "还", "不", "没", "有", "和", "与", "或", "但", "而", "如", "为", "把", "被",
            "让", "使", "给", "对", "向", "从", "到", "于", "以", "及", "等", "吧", "呢", "啊",
            "哦", "嗯", "哈", "嘛", "呀", "哇", "哎",
        ];
        STOPWORDS.contains(&sig)
    }

    /// epoch 秒转日期字符串 / Epoch seconds to date string (YYYY-MM-DD)
    fn epoch_to_date(epoch_secs: i64) -> String {
        let days_since_epoch = epoch_secs / 86400;
        let mut year = 1970i64;
        let mut remaining = days_since_epoch;
        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if remaining < days_in_year {
                break;
            }
            remaining -= days_in_year;
            year += 1;
        }
        let leap = Self::is_leap_year(year);
        let month_days: [i64; 12] = if leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut month = 1usize;
        for &days in &month_days {
            if remaining < days {
                break;
            }
            remaining -= days;
            month += 1;
        }
        let day = remaining + 1;
        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    /// epoch 秒转小时 / Epoch seconds to hour (0-23)
    fn epoch_to_hour(epoch_secs: i64) -> u8 {
        ((epoch_secs / 3600) % 24) as u8
    }

    /// 日期字符串转 epoch / Date string to epoch seconds
    fn date_to_epoch(date_str: &str) -> i64 {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return 0;
        }
        let year: i64 = parts[0].parse().unwrap_or(1970);
        let month: i64 = parts[1].parse().unwrap_or(1);
        let day: i64 = parts[2].parse().unwrap_or(1);
        let mut total_days = 0i64;
        for y in 1970..year {
            total_days += if Self::is_leap_year(y) { 366 } else { 365 };
        }
        let leap = Self::is_leap_year(year);
        let month_days: [i64; 12] = if leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        for (m, &md) in month_days.iter().enumerate().take((month - 1) as usize) {
            if m < 12 {
                total_days += md;
            }
        }
        total_days += day - 1;
        total_days * 86400
    }

    fn is_leap_year(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }
}

impl Default for AdaptiveRitualDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_signatures_basic() {
        let discovery = AdaptiveRitualDiscovery::new();
        let sigs = discovery.extract_signatures("打卡下班");
        assert!(!sigs.is_empty());
        // 应包含 "打卡" / Should contain "打卡"
        assert!(sigs.iter().any(|s| s.contains("打卡")));
    }

    #[test]
    fn test_extract_signatures_filters_content_hint() {
        let discovery = AdaptiveRitualDiscovery::new();
        let sigs = discovery.extract_signatures("晚安");
        // "晚安" 匹配 ContentHint::Goodnight，应被过滤
        // But n-grams of "晚安" that don't match ContentHint should still appear
        // Actually "晚安" itself is 2 chars and matches ContentHint, so it's filtered
        let has_goodnight = sigs.iter().any(|s| s == "晚安");
        assert!(
            !has_goodnight,
            "晚安 should be filtered as known ContentHint"
        );
    }

    #[test]
    fn test_extract_signatures_filters_stopwords() {
        let discovery = AdaptiveRitualDiscovery::new();
        let sigs = discovery.extract_signatures("的了是");
        // 纯停用词不应出现 / Pure stopwords should not appear
        assert!(!sigs.iter().any(|s| s == "的"));
        assert!(!sigs.iter().any(|s| s == "了"));
    }

    #[test]
    fn test_extract_signatures_empty() {
        let discovery = AdaptiveRitualDiscovery::new();
        assert!(discovery.extract_signatures("").is_empty());
        assert!(discovery.extract_signatures("a").is_empty()); // 太短 / Too short
    }

    #[test]
    fn test_extract_signatures_filters_punctuation() {
        let discovery = AdaptiveRitualDiscovery::new();
        let sigs = discovery.extract_signatures("123");
        // 纯数字应被过滤 / Pure digits should be filtered
        assert!(!sigs.iter().any(|s| s.chars().all(|c| c.is_numeric())));
    }

    #[test]
    fn test_record_interaction() {
        let mut discovery = AdaptiveRitualDiscovery::new();
        let base = 1781992800i64; // 2026-06-20 22:00 UTC
        discovery.record_interaction(base, "打卡下班了");
        // 应有候选 / Should have candidates
        assert!(!discovery.candidates.is_empty());
    }

    #[test]
    fn test_promotion_after_threshold() {
        let mut discovery = AdaptiveRitualDiscovery::new();
        discovery.config.promotion_threshold = 3;
        let base = 1781992800i64;

        let mut last_events = Vec::new();
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            discovery.record_interaction(day_epoch, "打卡下班");
            last_events = discovery.evaluate_daily(day_epoch + 86400);
        }

        // 应有升级事件 / Should have promotion event
        let promoted = last_events
            .iter()
            .any(|e| matches!(e, AdaptiveRitualEvent::Promoted { .. }));
        assert!(promoted, "Expected adaptive ritual promotion after 3 days");
        assert_eq!(discovery.active_count(), 1);
    }

    #[test]
    fn test_break_detection() {
        let mut discovery = AdaptiveRitualDiscovery::new();
        discovery.config.promotion_threshold = 2;
        discovery.config.break_threshold = 3;
        let base = 1781992800i64;

        // 建立仪式 / Establish ritual
        for day in 0..2 {
            let day_epoch = base + day * 86400;
            discovery.record_interaction(day_epoch, "打卡下班");
            discovery.evaluate_daily(day_epoch + 86400);
        }
        assert_eq!(discovery.active_count(), 1);

        // 模拟中断 / Simulate break
        let events = discovery.evaluate_daily(base + 6 * 86400);
        let broken = events
            .iter()
            .any(|e| matches!(e, AdaptiveRitualEvent::Broken { .. }));
        assert!(broken, "Expected break detection");
    }

    #[test]
    fn test_max_patterns_limit() {
        let mut discovery = AdaptiveRitualDiscovery::new();
        discovery.config.promotion_threshold = 2;
        discovery.config.max_patterns = 1;
        let base = 1781992800i64;

        // 建立两个不同的仪式 / Establish two different rituals
        for day in 0..2 {
            let day_epoch = base + day * 86400;
            discovery.record_interaction(day_epoch, "打卡下班");
            discovery.record_interaction(day_epoch, "吃饭了吗");
            discovery.evaluate_daily(day_epoch + 86400);
        }

        // 不应超过 max_patterns / Should not exceed max_patterns
        assert!(discovery.active_count() <= discovery.config.max_patterns);
    }

    #[test]
    fn test_pattern_name_and_description() {
        let pattern = AdaptivePattern {
            id: 1,
            signature: "打卡".to_string(),
            time_slot: Some(TimeSlot::new(18)),
            consecutive_days: 10,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_occurrences: 10,
            valence: 0.5,
        };
        assert!(pattern.name().contains("打卡"));
        assert!(pattern.description_zh().contains("打卡"));
        assert!(pattern.description_zh().contains("10天"));
    }

    #[test]
    fn test_clean_text() {
        assert_eq!(AdaptiveRitualDiscovery::clean_text("你好！"), "你好");
        assert_eq!(AdaptiveRitualDiscovery::clean_text("  test  "), "test");
    }

    #[test]
    fn test_no_promotion_for_known_hints() {
        let mut discovery = AdaptiveRitualDiscovery::new();
        discovery.config.promotion_threshold = 2;
        let base = 1781992800i64;

        // "晚安" 匹配 ContentHint，不应升级为自适应仪式
        for day in 0..2 {
            let day_epoch = base + day * 86400;
            discovery.record_interaction(day_epoch, "晚安");
            discovery.evaluate_daily(day_epoch + 86400);
        }

        // 不应有自适应仪式升级 / No adaptive ritual promotion for known hint
        assert_eq!(discovery.active_count(), 0);
    }
}
