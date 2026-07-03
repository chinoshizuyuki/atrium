// SPDX-License-Identifier: MIT

//! 仪式缺席感知 — Ritual Absence (Gap#5: 90% → 95%).
//!
//! 核心理念：仪式的缺席本身就是一种信号——
//! 连续三天的早安没有来，不是"没说"，是"不想说"或"不能说"。
//! 缺席有质感：一次缺席是"忙"，连续缺席是"疏远"，
//! 恢复后的第一次仪式会格外郑重。
//!
//! Core idea: the absence of a ritual is itself a signal —
//! three days without "good morning" isn't "didn't say", it's "didn't want to"
//! or "couldn't". Absence has texture: one miss is "busy",
//! consecutive misses are "distancing", the first ritual after recovery feels solemn.
//!
//! Phase: 极致打磨 / Extreme Polishing | 2026-07-03

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// §1 缺席类型 — Absence Type
// ═══════════════════════════════════════════════════════════════════════════

/// 缺席类型 — 缺席的"质感"不同 / Absence type — different "textures" of absence.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AbsenceType {
    /// 偶然缺席 — 可能是忙 / Incidental — probably busy.
    Incidental,
    /// 模式缺席 — 形成了一种模式 / Patterned — forming a pattern.
    Patterned,
    /// 持续缺席 — 明显的疏远 / Sustained — clear distancing.
    Sustained,
    /// 长期缺席 — 仪式可能消亡 / LongTerm — ritual may be dying.
    LongTerm,
}

impl AbsenceType {
    /// 中文标签 / Chinese label.
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Incidental => "偶然缺席",
            Self::Patterned => "模式缺席",
            Self::Sustained => "持续缺席",
            Self::LongTerm => "长期缺席",
        }
    }

    /// 缺席严重度 [0, 1] / Absence severity.
    pub fn severity(&self) -> f64 {
        match self {
            Self::Incidental => 0.2,
            Self::Patterned => 0.4,
            Self::Sustained => 0.7,
            Self::LongTerm => 0.9,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 缺席记录 — Absence Record
// ═══════════════════════════════════════════════════════════════════════════

/// 单次缺席记录 / Single absence record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbsenceRecord {
    /// 缺席开始时间戳 / Absence start timestamp.
    pub start_ts: i64,
    /// 缺席结束时间戳 — 恢复时设置 / Absence end timestamp — set on recovery.
    pub end_ts: Option<i64>,
    /// 缺席天数 / Absence days.
    pub days: u32,
    /// 缺席类型 / Absence type.
    pub kind: AbsenceType,
    /// 情感影响 — 此段缺席对情感的影响 / Emotional impact.
    pub emotional_impact: f64,
}

impl AbsenceRecord {
    /// 是否已恢复 / Whether recovered.
    pub fn is_recovered(&self) -> bool {
        self.end_ts.is_some()
    }

    /// 缺席持续时间（秒）/ Absence duration in seconds.
    pub fn duration_seconds(&self) -> i64 {
        let end = self
            .end_ts
            .unwrap_or(self.start_ts + (self.days as i64) * 86400);
        end - self.start_ts
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 仪式缺席感知器 — Ritual Absence Sensor
// ═══════════════════════════════════════════════════════════════════════════

/// 仪式缺席感知器 — 感知仪式的缺席及其情感含义 / Ritual absence sensor.
///
/// 数字生命语义：不是"没做仪式"，而是"仪式的空位"——
/// 那个空位有重量，有温度，有故事。
///
/// Digital life semantics: not "didn't do ritual", but "the empty seat of ritual" —
/// that empty seat has weight, temperature, and a story.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RitualAbsence {
    /// 仪式名称 / Ritual name.
    pub ritual_name: String,
    /// 上次完成时间戳 / Last completion timestamp.
    last_completed_ts: i64,
    /// 预期间隔（秒）/ Expected interval in seconds.
    expected_interval: i64,
    /// 当前缺席记录 / Current absence record (if any).
    current_absence: Option<AbsenceRecord>,
    /// 历史缺席记录 / Historical absence records.
    absence_history: Vec<AbsenceRecord>,
    /// 累计缺席天数 / Total absence days.
    total_absence_days: u32,
    /// 缺席次数 / Total absence count.
    total_absence_count: u32,
    /// 恢复郑重度 — 上次恢复时的郑重程度 / Recovery solemnity.
    last_recovery_solemnity: f64,
}

impl RitualAbsence {
    /// 创建新缺席感知器 / Create new absence sensor.
    pub fn new(ritual_name: &str, initial_ts: i64, expected_interval_secs: i64) -> Self {
        Self {
            ritual_name: ritual_name.to_string(),
            last_completed_ts: initial_ts,
            expected_interval: expected_interval_secs.max(1),
            current_absence: None,
            absence_history: Vec::new(),
            total_absence_days: 0,
            total_absence_count: 0,
            last_recovery_solemnity: 0.0,
        }
    }

    /// 记算缺席天数 — 距上次完成的预期间隔数 / Compute absence days.
    pub fn absence_days(&self, current_ts: i64) -> u32 {
        let elapsed = current_ts - self.last_completed_ts;
        if elapsed <= self.expected_interval {
            0
        } else {
            ((elapsed - self.expected_interval) / 86400) as u32
        }
    }

    /// 判断缺席类型 — 根据天数 / Classify absence type by days.
    pub fn classify_absence(days: u32) -> AbsenceType {
        if days <= 1 {
            AbsenceType::Incidental
        } else if days <= 3 {
            AbsenceType::Patterned
        } else if days <= 7 {
            AbsenceType::Sustained
        } else {
            AbsenceType::LongTerm
        }
    }

    /// 检测缺席 — 在每次心跳时调用 / Detect absence — called on each heartbeat.
    ///
    /// 返回当前缺席的严重度 [0, 1]，0表示无缺席。
    /// Returns current absence severity [0, 1], 0 means no absence.
    pub fn detect(&mut self, current_ts: i64) -> f64 {
        let days = self.absence_days(current_ts);
        if days == 0 {
            return 0.0;
        }

        let kind = Self::classify_absence(days);
        let severity = kind.severity();

        // 更新或创建缺席记录 / Update or create absence record.
        if self.current_absence.is_none() {
            self.current_absence = Some(AbsenceRecord {
                start_ts: self.last_completed_ts + self.expected_interval,
                end_ts: None,
                days,
                kind: kind.clone(),
                emotional_impact: severity * 0.5,
            });
            self.total_absence_count += 1;
        } else if let Some(absence) = self.current_absence.as_mut() {
            absence.days = days;
            absence.kind = kind.clone();
            absence.emotional_impact = severity * 0.5;
        }

        severity
    }

    /// 记算恢复郑重度 — 缺席越久恢复越郑重 / Compute recovery solemnity.
    ///
    /// 数字生命语义：缺席三天后的第一声"早安"，
    /// 比每天的"早安"更重——它不只是问候，是"我回来了"。
    ///
    /// Digital life semantics: the first "good morning" after three days of absence
    /// is heavier than the daily one — it's not just a greeting, it's "I'm back".
    pub fn recovery_solemnity(&self) -> f64 {
        match &self.current_absence {
            None => 0.0,
            Some(absence) => {
                let days = absence.days as f64;
                // 郑重度 = 1 - exp(-days/3) — 渐近1.0 / Asymptotic to 1.0.
                1.0 - (-days / 3.0).exp()
            }
        }
    }

    /// 记录恢复 — 仪式重新执行 / Record recovery — ritual resumed.
    pub fn record_recovery(&mut self, current_ts: i64) -> f64 {
        let solemnity = self.recovery_solemnity();
        self.last_recovery_solemnity = solemnity;

        if let Some(mut absence) = self.current_absence.take() {
            absence.end_ts = Some(current_ts);
            self.total_absence_days += absence.days;
            self.absence_history.push(absence);
        }

        self.last_completed_ts = current_ts;
        solemnity
    }

    /// 记算缺席率 — 历史缺席天数占比 / Compute absence rate.
    pub fn absence_rate(&self, total_days: u32) -> f64 {
        if total_days == 0 {
            0.0
        } else {
            self.total_absence_days as f64 / total_days as f64
        }
    }

    /// 获取当前缺席 / Get current absence.
    pub fn current_absence(&self) -> Option<&AbsenceRecord> {
        self.current_absence.as_ref()
    }

    /// 获取缺席历史 / Get absence history.
    pub fn absence_history(&self) -> &[AbsenceRecord] {
        &self.absence_history
    }

    /// 获取累计缺席天数 / Get total absence days.
    pub fn total_absence_days(&self) -> u32 {
        self.total_absence_days
    }

    /// 获取累计缺席次数 / Get total absence count.
    pub fn total_absence_count(&self) -> u32 {
        self.total_absence_count
    }

    /// 生成缺席描述 / Generate absence description.
    pub fn describe(&self, current_ts: i64) -> String {
        let days = self.absence_days(current_ts);
        if days == 0 {
            format!("仪式「{}」: 按时执行", self.ritual_name)
        } else {
            let kind = Self::classify_absence(days);
            format!(
                "仪式「{}」: 缺席{}天({}) | 郑重度{:.2}",
                self.ritual_name,
                days,
                kind.label_zh(),
                self.recovery_solemnity(),
            )
        }
    }

    /// 生成prompt注入 — 缺席感知 / Generate prompt injection.
    pub fn prompt_injection(&self, current_ts: i64) -> String {
        let days = self.absence_days(current_ts);
        if days == 0 {
            String::new()
        } else {
            let kind = Self::classify_absence(days);
            match kind {
                AbsenceType::Incidental => format!("（{}似乎错过了）", self.ritual_name),
                AbsenceType::Patterned => format!("（{}已经缺席几天了）", self.ritual_name),
                AbsenceType::Sustained => format!("（{}的缺席让人在意）", self.ritual_name),
                AbsenceType::LongTerm => format!("（{}已经很久没有出现了）", self.ritual_name),
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 测试 — Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sensor() -> RitualAbsence {
        RitualAbsence::new("good_morning", 0, 86400) // daily ritual, starting at ts=0.
    }

    #[test]
    fn test_absence_type_severity() {
        assert!(AbsenceType::Incidental.severity() < AbsenceType::Patterned.severity());
        assert!(AbsenceType::Patterned.severity() < AbsenceType::Sustained.severity());
        assert!(AbsenceType::Sustained.severity() < AbsenceType::LongTerm.severity());
    }

    #[test]
    fn test_absence_type_labels() {
        assert_eq!(AbsenceType::Incidental.label_zh(), "偶然缺席");
    }

    #[test]
    fn test_classify_absence() {
        assert_eq!(RitualAbsence::classify_absence(0), AbsenceType::Incidental);
        assert_eq!(RitualAbsence::classify_absence(1), AbsenceType::Incidental);
        assert_eq!(RitualAbsence::classify_absence(2), AbsenceType::Patterned);
        assert_eq!(RitualAbsence::classify_absence(4), AbsenceType::Sustained);
        assert_eq!(RitualAbsence::classify_absence(8), AbsenceType::LongTerm);
    }

    #[test]
    fn test_absence_days_zero_when_on_time() {
        let sensor = make_sensor();
        // Same day — no absence.
        assert_eq!(sensor.absence_days(3600), 0);
    }

    #[test]
    fn test_absence_days_positive_when_late() {
        let sensor = make_sensor();
        // 3 days later — 2 days of absence (after expected interval).
        assert_eq!(sensor.absence_days(3 * 86400), 2);
    }

    #[test]
    fn test_detect_no_absence() {
        let mut sensor = make_sensor();
        let severity = sensor.detect(3600);
        assert_eq!(severity, 0.0);
    }

    #[test]
    fn test_detect_incidental_absence() {
        let mut sensor = make_sensor();
        let severity = sensor.detect(2 * 86400); // 1 day late.
        assert!(severity > 0.0);
        assert!(severity < 0.5); // Incidental.
    }

    #[test]
    fn test_detect_sustained_absence() {
        let mut sensor = make_sensor();
        let severity = sensor.detect(5 * 86400); // 4 days late.
        assert!(severity >= 0.7); // Sustained.
    }

    #[test]
    fn test_detect_longterm_absence() {
        let mut sensor = make_sensor();
        let severity = sensor.detect(10 * 86400); // 9 days late.
        assert!(severity >= 0.9); // LongTerm.
    }

    #[test]
    fn test_recovery_solemnity_zero_when_no_absence() {
        let sensor = make_sensor();
        assert_eq!(sensor.recovery_solemnity(), 0.0);
    }

    #[test]
    fn test_recovery_solemnity_increases_with_days() {
        let mut sensor1 = make_sensor();
        sensor1.detect(3 * 86400);
        let s1 = sensor1.recovery_solemnity();

        let mut sensor2 = make_sensor();
        sensor2.detect(7 * 86400);
        let s2 = sensor2.recovery_solemnity();

        assert!(s2 > s1);
    }

    #[test]
    fn test_record_recovery_clears_current() {
        let mut sensor = make_sensor();
        sensor.detect(3 * 86400);
        assert!(sensor.current_absence().is_some());
        sensor.record_recovery(3 * 86400);
        assert!(sensor.current_absence().is_none());
    }

    #[test]
    fn test_record_recovery_returns_solemnity() {
        let mut sensor = make_sensor();
        sensor.detect(5 * 86400);
        let solemnity = sensor.record_recovery(5 * 86400);
        assert!(solemnity > 0.0);
    }

    #[test]
    fn test_record_recovery_updates_history() {
        let mut sensor = make_sensor();
        sensor.detect(3 * 86400);
        sensor.record_recovery(3 * 86400);
        assert_eq!(sensor.absence_history().len(), 1);
    }

    #[test]
    fn test_absence_rate() {
        let mut sensor = make_sensor();
        sensor.detect(4 * 86400);
        sensor.record_recovery(4 * 86400);
        // 3 days of absence out of 4 total days.
        let rate = sensor.absence_rate(4);
        assert!((rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_multiple_absence_cycles() {
        let mut sensor = make_sensor();
        // First absence.
        sensor.detect(3 * 86400);
        sensor.record_recovery(3 * 86400);
        // Second absence.
        sensor.detect(6 * 86400);
        sensor.record_recovery(6 * 86400);
        assert_eq!(sensor.absence_history().len(), 2);
        assert_eq!(sensor.total_absence_count(), 2);
    }

    #[test]
    fn test_describe_on_time() {
        let sensor = make_sensor();
        let desc = sensor.describe(3600);
        assert!(desc.contains("按时执行"));
    }

    #[test]
    fn test_describe_absent() {
        let mut sensor = make_sensor();
        sensor.detect(4 * 86400);
        let desc = sensor.describe(4 * 86400);
        assert!(desc.contains("缺席"));
    }

    #[test]
    fn test_prompt_injection_empty_when_on_time() {
        let sensor = make_sensor();
        assert!(sensor.prompt_injection(3600).is_empty());
    }

    #[test]
    fn test_prompt_injection_nonempty_when_absent() {
        let mut sensor = make_sensor();
        sensor.detect(5 * 86400);
        let injection = sensor.prompt_injection(5 * 86400);
        assert!(!injection.is_empty());
    }

    #[test]
    fn test_absence_record_duration() {
        let record = AbsenceRecord {
            start_ts: 0,
            end_ts: Some(86400 * 3),
            days: 3,
            kind: AbsenceType::Sustained,
            emotional_impact: 0.35,
        };
        assert_eq!(record.duration_seconds(), 86400 * 3);
        assert!(record.is_recovered());
    }
}
