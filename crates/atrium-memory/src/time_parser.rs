// SPDX-License-Identifier: MIT
//! 时间解析器 — 中文自然语言 → RRULE + next_trigger_at
//! Time parser — Chinese NL → RRULE + next trigger timestamp.
//!
//! 纯规则解析，零 LLM 依赖，<1μs。
//! Pure rule-based parsing, zero LLM dependency, <1μs.

use chrono::{Datelike, Local, NaiveTime, Timelike};

/// 解析结果
#[derive(Clone, Debug)]
pub struct TimeParseResult {
    /// RRULE 字符串 (RFC 5545)，一次性提醒为空
    pub rrule: String,
    /// 下次触发时间 (epoch seconds)
    pub next_trigger_at: i64,
    /// 是否为一次性（触发后删除）
    pub one_shot: bool,
}

/// 解析中文自然语言时间表达式
///
/// # Examples
/// - "每天早上8点提醒我开会" → FREQ=DAILY;BYHOUR=8
/// - "每周三下午3点" → FREQ=WEEKLY;BYDAY=WE;BYHOUR=15
/// - "明天下午2点" → 一次性, +1 day at 14:00
pub fn parse_time(msg: &str) -> Option<TimeParseResult> {
    let now = Local::now();

    // 尝试提取小时:分钟，失败则用默认时间 9:00
    let time = extract_time(msg).unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap());

    // 检测重复模式
    if msg.contains("每天") || msg.contains("每日") || msg.contains("天天") {
        let rrule = format!(
            "FREQ=DAILY;BYHOUR={};BYMINUTE={}",
            time.hour(),
            time.minute()
        );
        let next = next_occurrence_daily(now, time);
        return Some(TimeParseResult {
            rrule,
            next_trigger_at: next,
            one_shot: false,
        });
    }

    if msg.contains("每周") || msg.contains("每个星期") {
        let weekday = extract_weekday(msg)?;
        let rrule = format!(
            "FREQ=WEEKLY;BYDAY={};BYHOUR={};BYMINUTE={}",
            weekday,
            time.hour(),
            time.minute()
        );
        let next = next_occurrence_weekly(now, weekday, time);
        return Some(TimeParseResult {
            rrule,
            next_trigger_at: next,
            one_shot: false,
        });
    }

    if msg.contains("每月") || msg.contains("每个月") {
        let day = extract_month_day(msg).unwrap_or(now.day());
        let rrule = format!(
            "FREQ=MONTHLY;BYMONTHDAY={};BYHOUR={};BYMINUTE={}",
            day,
            time.hour(),
            time.minute()
        );
        let next = next_occurrence_monthly(now, day, time);
        return Some(TimeParseResult {
            rrule,
            next_trigger_at: next,
            one_shot: false,
        });
    }

    // 一次性
    let days_offset = if msg.contains("明天") {
        1
    } else if msg.contains("后天") {
        2
    } else if msg.contains("大后天") {
        3
    } else {
        0
    };

    let target = now.date_naive() + chrono::Duration::days(days_offset);
    let dt = target
        .and_time(time)
        .and_local_timezone(Local)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| now.timestamp());

    Some(TimeParseResult {
        rrule: String::new(),
        next_trigger_at: dt,
        one_shot: true,
    })
}

fn extract_time(msg: &str) -> Option<NaiveTime> {
    // 1. Try "H:MM" or "H：MM" format
    // 取首个匹配即可 / First match suffices
    if let Some(cap) = find_time_patterns(msg).into_iter().next() {
        let mut hour = cap.hour;
        let minute = cap.minute;
        hour = apply_period_offset(msg, hour);
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }

    // 2. Try "N点MM分" / "N点MM" / "N点" format
    if let Some((hour, minute)) = extract_hour_minute(msg) {
        let h = apply_period_offset(msg, hour);
        return NaiveTime::from_hms_opt(h, minute, 0);
    }

    None
}

fn apply_period_offset(msg: &str, hour: u32) -> u32 {
    let mut h = hour;
    if msg.contains("下午") || msg.contains("晚上") || msg.contains("傍晚") {
        if h < 12 {
            h += 12;
        }
    } else if (msg.contains("凌晨") || msg.contains("深夜")) && h == 12 {
        h = 0;
    }
    if h >= 24 {
        h = 23;
    }
    h
}

struct TimeCap {
    hour: u32,
    minute: u32,
}

fn find_time_patterns(msg: &str) -> Vec<TimeCap> {
    let mut results = Vec::new();
    let chars: Vec<char> = msg.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        if chars[i].is_ascii_digit() || is_chinese_digit(chars[i]) {
            let mut j = i;
            while j < len && (chars[j].is_ascii_digit() || is_chinese_digit(chars[j])) {
                j += 1;
            }
            let hour_str: String = chars[i..j].iter().collect();
            let hour = parse_digit(&hour_str);
            if let Some(h) = hour {
                if j + 2 < len
                    && (chars[j] == ':' || chars[j] == '：')
                    && chars[j + 1].is_ascii_digit()
                {
                    let mut k = j + 1;
                    while k < len && chars[k].is_ascii_digit() && k - (j + 1) < 2 {
                        k += 1;
                    }
                    let min_str: String = chars[j + 1..k].iter().collect();
                    if let Ok(m) = min_str.parse::<u32>() {
                        results.push(TimeCap { hour: h, minute: m });
                    }
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    results
}

fn extract_hour_minute(msg: &str) -> Option<(u32, u32)> {
    let chars: Vec<char> = msg.chars().collect();
    for i in 0..chars.len() {
        if i + 1 >= chars.len() || chars[i + 1] != '点' {
            // "十二点": chars[i]=='二' && chars[i+1]=='点' && i>0 && chars[i-1]=='十'
            if chars[i] == '二'
                && i + 1 < chars.len()
                && chars[i + 1] == '点'
                && i > 0
                && chars[i - 1] == '十'
            {
                let minute = extract_minute_after_point(&chars, i + 1);
                return Some((12, minute));
            }
            continue;
        }

        // chars[i] is the char before '点'
        if let Some(h) = parse_digit(&chars[i].to_string()) {
            let minute = extract_minute_after_point(&chars, i + 1);
            return Some((h, minute));
        }
    }
    None
}

/// Extract minutes after "点" in a char slice — "点20分" "点20" "点半" → 20, 30, 0
fn extract_minute_after_point(chars: &[char], point_idx: usize) -> u32 {
    // chars[point_idx] is '点', check chars after it
    if point_idx + 1 >= chars.len() {
        return 0;
    }

    // "点半" → 30
    if chars[point_idx + 1] == '半' {
        return 30;
    }

    // Collect digits after "点"
    let mut digits = String::new();
    for &c in chars[point_idx + 1..].iter() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if !digits.is_empty() {
            break;
        }
    }

    if let Ok(m) = digits.parse::<u32>() {
        if m < 60 {
            m
        } else {
            0
        }
    } else {
        0
    }
}

fn is_chinese_digit(c: char) -> bool {
    matches!(
        c,
        '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
    )
}

fn parse_digit(s: &str) -> Option<u32> {
    if let Ok(n) = s.trim().parse::<u32>() {
        if n <= 9 {
            return Some(n);
        }
    }
    match s.trim() {
        "零" => Some(0),
        "一" => Some(1),
        "二" | "两" => Some(2),
        "三" => Some(3),
        "四" => Some(4),
        "五" => Some(5),
        "六" => Some(6),
        "七" => Some(7),
        "八" => Some(8),
        "九" => Some(9),
        "十" => Some(10),
        _ => None,
    }
}

fn extract_weekday(msg: &str) -> Option<&'static str> {
    let pairs = [
        ("周一", "MO"),
        ("星期一", "MO"),
        ("周二", "TU"),
        ("星期二", "TU"),
        ("周三", "WE"),
        ("星期三", "WE"),
        ("周四", "TH"),
        ("星期四", "TH"),
        ("周五", "FR"),
        ("星期五", "FR"),
        ("周六", "SA"),
        ("星期六", "SA"),
        ("周日", "SU"),
        ("星期天", "SU"),
        ("星期日", "SU"),
    ];
    for (keyword, rrule_day) in &pairs {
        if msg.contains(keyword) {
            return Some(rrule_day);
        }
    }
    None
}

fn extract_month_day(msg: &str) -> Option<u32> {
    // "每月1号" "每个月15日"
    for prefix in &["每月", "每个月"] {
        if let Some(pos) = msg.find(prefix) {
            let rest = &msg[pos + prefix.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(d) = digits.parse::<u32>() {
                if (1..=31).contains(&d) {
                    return Some(d);
                }
            }
        }
    }
    None
}

fn next_occurrence_daily(now: chrono::DateTime<Local>, time: NaiveTime) -> i64 {
    let today = now.date_naive().and_time(time);
    let today_ts = today
        .and_local_timezone(Local)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0);
    if today_ts > now.timestamp() {
        today_ts
    } else {
        (now + chrono::Duration::days(1))
            .date_naive()
            .and_time(time)
            .and_local_timezone(Local)
            .single()
            .map(|d| d.timestamp())
            .unwrap_or(0)
    }
}

fn next_occurrence_weekly(now: chrono::DateTime<Local>, target_day: &str, time: NaiveTime) -> i64 {
    let target_num = match target_day {
        "MO" => chrono::Weekday::Mon,
        "TU" => chrono::Weekday::Tue,
        "WE" => chrono::Weekday::Wed,
        "TH" => chrono::Weekday::Thu,
        "FR" => chrono::Weekday::Fri,
        "SA" => chrono::Weekday::Sat,
        "SU" => chrono::Weekday::Sun,
        _ => return now.timestamp(),
    };
    let today = now.date_naive();
    let current_weekday = today.weekday();
    let days_until = (target_num.num_days_from_monday() as i64
        - current_weekday.num_days_from_monday() as i64
        + 7)
        % 7;
    let target_date = if days_until == 0 && now.time() > time {
        today + chrono::Duration::days(7)
    } else if days_until == 0 {
        today
    } else {
        today + chrono::Duration::days(days_until)
    };
    target_date
        .and_time(time)
        .and_local_timezone(Local)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn next_occurrence_monthly(now: chrono::DateTime<Local>, target_day: u32, time: NaiveTime) -> i64 {
    let today = now.date_naive();
    let mut month = today.month();
    let mut year = today.year();
    if today.day() >= target_day && now.time() > time {
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }
    if let Some(d) = chrono::NaiveDate::from_ymd_opt(year, month, target_day) {
        d.and_time(time)
            .and_local_timezone(Local)
            .single()
            .map(|d| d.timestamp())
            .unwrap_or(0)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily() {
        let r = parse_time("每天早上8点提醒我开会").unwrap();
        assert!(r.rrule.contains("FREQ=DAILY"));
        assert!(r.rrule.contains("BYHOUR=8"));
        assert!(!r.one_shot);
    }

    #[test]
    fn test_weekly() {
        let r = parse_time("每周三下午3点开会").unwrap();
        assert!(r.rrule.contains("FREQ=WEEKLY"));
        assert!(r.rrule.contains("BYDAY=WE"));
        assert!(r.rrule.contains("BYHOUR=15"));
    }

    #[test]
    fn test_monthly() {
        let r = parse_time("每月1号还款").unwrap();
        assert!(r.rrule.contains("FREQ=MONTHLY"));
        assert!(r.rrule.contains("BYMONTHDAY=1"));
    }

    #[test]
    fn test_one_shot() {
        let r = parse_time("明天下午2点提醒我").unwrap();
        assert!(r.one_shot);
        assert!(r.rrule.is_empty());
    }

    #[test]
    fn test_evening() {
        let r = parse_time("每天晚上9点").unwrap();
        assert!(r.rrule.contains("BYHOUR=21"));
    }

    #[test]
    fn test_no_time() {
        assert!(parse_time("提醒我明天").is_some()); // default 9:00
    }

    #[test]
    fn test_precise_minute() {
        let r = parse_time("每天下午3点20提醒我").unwrap();
        assert!(r.rrule.contains("BYHOUR=15"));
        assert!(r.rrule.contains("BYMINUTE=20"));
    }

    #[test]
    fn test_half_hour() {
        let r = parse_time("每天3点半提醒我").unwrap();
        assert!(r.rrule.contains("BYMINUTE=30"));
    }

    #[test]
    fn test_precise_no_ampm() {
        let r = parse_time("每天早上8点05分").unwrap();
        assert!(r.rrule.contains("BYHOUR=8"));
    }
}
