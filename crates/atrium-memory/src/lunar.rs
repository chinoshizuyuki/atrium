// ════════════════════════════════════════════════════════════════════
// Lunar — 农历公历互转 / Chinese Lunar-Solar Calendar Conversion
// ════════════════════════════════════════════════════════════════════
//
// 基于 1900-2100 年农历信息表的查表算法。
// 数据来源：calendar.js (jiangjiazhi)，经微软 ChineseLunisolarCalendar 验证。
//
// 编码格式（每个 u32 条目）：
//   - 位 0-3:  闰月月份 (0=无闰月, 1-12=闰月)
//   - 位 4-15: 1-12 月大小月标记 (1=30天/大月, 0=29天/小月)
//              位 (16-m) 表示第 m 月 (m=1..12)
//   - 位 16:   闰月天数 (1=30天, 0=29天)

/// 农历信息表 (1900-2100) / Lunar calendar data table
///
/// 每个条目编码一年的农历信息，详见模块头部注释。
const LUNAR_INFO: [u32; 201] = [
    // 1900-1909
    0x04bd8, 0x04ae0, 0x0a570, 0x054d5, 0x0d260, 0x0d950, 0x16554, 0x056a0, 0x09ad0, 0x055d2,
    // 1910-1919
    0x04ae0, 0x0a5b6, 0x0a4d0, 0x0d250, 0x1d255, 0x0b540, 0x0d6a0, 0x0ada2, 0x095b0, 0x14977,
    // 1920-1929
    0x04970, 0x0a4b0, 0x0b4b5, 0x06a50, 0x06d40, 0x1ab54, 0x02b60, 0x09570, 0x052f2, 0x04970,
    // 1930-1939
    0x06566, 0x0d4a0, 0x0ea50, 0x06e95, 0x05ad0, 0x02b60, 0x186e3, 0x092e0, 0x1c8d7, 0x0c950,
    // 1940-1949
    0x0d4a0, 0x1d8a6, 0x0b550, 0x056a0, 0x1a5b4, 0x025d0, 0x092d0, 0x0d2b2, 0x0a950, 0x0b557,
    // 1950-1959
    0x06ca0, 0x0b550, 0x15355, 0x04da0, 0x0a5b0, 0x14573, 0x052b0, 0x0a9a8, 0x0e950, 0x06aa0,
    // 1960-1969
    0x0aea6, 0x0ab50, 0x04b60, 0x0aae4, 0x0a570, 0x05260, 0x0f263, 0x0d950, 0x05b57, 0x056a0,
    // 1970-1979
    0x096d0, 0x04dd5, 0x04ad0, 0x0a4d0, 0x0d4d4, 0x0d250, 0x0d558, 0x0b540, 0x0b6a0, 0x195a6,
    // 1980-1989
    0x095b0, 0x049b0, 0x0a974, 0x0a4b0, 0x0b27a, 0x06a50, 0x06d40, 0x0af46, 0x0ab60, 0x09570,
    // 1990-1999
    0x04af5, 0x04970, 0x064b0, 0x074a3, 0x0ea50, 0x06b58, 0x05ac0, 0x0ab60, 0x096d5, 0x092e0,
    // 2000-2009
    0x0c960, 0x0d954, 0x0d4a0, 0x0da50, 0x07552, 0x056a0, 0x0abb7, 0x025d0, 0x092d0, 0x0cab5,
    // 2010-2019
    0x0a950, 0x0b4a0, 0x0baa4, 0x0ad50, 0x055d9, 0x04ba0, 0x0a5b0, 0x15176, 0x052b0, 0x0a930,
    // 2020-2029
    0x07954, 0x06aa0, 0x0ad50, 0x05b52, 0x04b60, 0x0a6e6, 0x0a4e0, 0x0d260, 0x0ea65, 0x0d530,
    // 2030-2039
    0x05aa0, 0x076a3, 0x096d0, 0x04afb, 0x04ad0, 0x0a4d0, 0x1d0b6, 0x0d250, 0x0d520, 0x0dd45,
    // 2040-2049
    0x0b5a0, 0x056d0, 0x055b2, 0x049b0, 0x0a577, 0x0a4b0, 0x0aa50, 0x1b255, 0x06d20, 0x0ada0,
    // 2050-2059
    0x14b63, 0x09370, 0x049f8, 0x04970, 0x064b0, 0x168a6, 0x0ea50, 0x06b20, 0x1a6c4, 0x0aae0,
    // 2060-2069
    0x0a2e0, 0x0d2e3, 0x0c960, 0x0d557, 0x0d4a0, 0x0da50, 0x05d55, 0x056a0, 0x0a6d0, 0x055d4,
    // 2070-2079
    0x052d0, 0x0a9b8, 0x0a950, 0x0b4a0, 0x0b6a6, 0x0ad50, 0x055a0, 0x0aba4, 0x0a5b0, 0x052b0,
    // 2080-2089
    0x0b273, 0x06930, 0x07337, 0x06aa0, 0x0ad50, 0x14b55, 0x04b60, 0x0a570, 0x054e4, 0x0d160,
    // 2090-2099
    0x0e968, 0x0d520, 0x0daa0, 0x16aa6, 0x056d0, 0x04ae0, 0x0a9d4, 0x0a2d0, 0x0d150, 0x0f252,
    // 2100
    0x0d520,
];

// ── 内部辅助函数 / Internal helpers ──

/// 获取农历年份的闰月月份 / Get leap month number for a lunar year
///
/// 返回 0 表示无闰月，1-12 表示闰月月份。
/// Returns 0 for no leap month, 1-12 for leap month number.
fn leap_month(year: u32) -> u32 {
    LUNAR_INFO[(year - 1900) as usize] & 0xF
}

/// 获取农历年份闰月的天数 / Get leap month days for a lunar year
///
/// 无闰月返回 0，有闰月返回 29 或 30。
/// Returns 0 if no leap month, 29 or 30 otherwise.
fn leap_days(year: u32) -> u32 {
    let lm = leap_month(year);
    if lm == 0 {
        0
    } else if (LUNAR_INFO[(year - 1900) as usize] & 0x10000) != 0 {
        30
    } else {
        29
    }
}

/// 获取农历年份某月的天数（非闰月）/ Get month days (non-leap) for a lunar year
///
/// 返回 29 或 30。月份范围 1-12。
/// Returns 29 or 30. Month range: 1-12.
fn month_days(year: u32, month: u32) -> u32 {
    if (LUNAR_INFO[(year - 1900) as usize] & (0x10000 >> month)) != 0 {
        30
    } else {
        29
    }
}

/// 获取农历年份的总天数 / Get total days in a lunar year
fn year_days(year: u32) -> u32 {
    // 348 = 12 * 29（每月至少 29 天）/ 348 = 12 * 29 (minimum 29 days per month)
    // 每个大月（30天）比小月（29天）多 1 天，统计大月数量
    // Each big month (30d) has 1 extra day vs small month (29d); count big months
    let mut sum = 348u32;
    let info = LUNAR_INFO[(year - 1900) as usize];
    // 从 0x8000 右移到 0x10，扫描 12 个月的大小月标记
    // Right-shift from 0x8000 to 0x10, scanning 12 months' big/small flags
    let mut i = 0x8000u32;
    while i > 0x8 {
        if (info & i) != 0 {
            sum += 1;
        }
        i >>= 1;
    }
    sum + leap_days(year)
}

/// 从偏移天数计算公历日期 / Compute solar date from day offset
///
/// 偏移从 1900-01-31（农历 1900-01-01 对应的公历日期）开始。
/// Offset starts from 1900-01-31 (solar date of lunar 1900-01-01).
fn solar_from_offset(offset: i64) -> (u32, u32, u32) {
    // 1900-01-31 = 1900-01-01 + 30 天
    let mut total_days = offset + 30;

    let mut year = 1900u32;
    loop {
        let diy = if is_leap_solar(year) { 366i64 } else { 365i64 };
        if total_days < diy {
            break;
        }
        total_days -= diy;
        year += 1;
    }

    let md = month_days_solar(year);
    let mut month = 1u32;
    for &d in &md {
        if total_days < d {
            break;
        }
        total_days -= d;
        month += 1;
    }

    let day = (total_days + 1) as u32;
    (year, month, day)
}

/// 公历闰年判断 / Solar leap year check
#[allow(unknown_lints)]
#[allow(clippy::manual_is_multiple_of)]
fn is_leap_solar(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 公历每月天数 / Solar month days array
fn month_days_solar(year: u32) -> [i64; 12] {
    if is_leap_solar(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    }
}

// ── 公开 API / Public API ──

/// 农历转公历 / Convert lunar date to solar date
///
/// # 参数 / Parameters
/// - `lunar_year`: 农历年份 (1900-2100) / Lunar year
/// - `lunar_month`: 农历月份 (1-12) / Lunar month
/// - `lunar_day`: 农历日期 (1-30) / Lunar day
/// - `is_leap`: 是否为闰月 / Whether the month is a leap month
///
/// # 返回 / Returns
/// - `Some((solar_year, solar_month, solar_day))` 转换成功 / Conversion success
/// - `None` 输入无效 / Invalid input
///
/// # 示例 / Example
/// ```no_run
/// use atrium_memory::lunar::lunar_to_solar;
/// // 2026年春节：农历正月初一 → 公历 2026-02-17
/// let result = lunar_to_solar(2026, 1, 1, false);
/// assert_eq!(result, Some((2026, 2, 17)));
/// ```
pub fn lunar_to_solar(
    lunar_year: u32,
    lunar_month: u32,
    lunar_day: u32,
    is_leap: bool,
) -> Option<(u32, u32, u32)> {
    // 参数校验 / Parameter validation
    if !(1900..=2100).contains(&lunar_year) {
        return None;
    }
    if !(1..=12).contains(&lunar_month) {
        return None;
    }
    if !(1..=30).contains(&lunar_day) {
        return None;
    }

    let leap = leap_month(lunar_year);

    // 闰月校验 / Leap month validation
    if is_leap && (leap == 0 || lunar_month != leap) {
        return None; // 无效：该年无此闰月 / Invalid: no such leap month this year
    }

    // 日数校验 / Day validation
    let max_day = if is_leap {
        leap_days(lunar_year)
    } else {
        month_days(lunar_year, lunar_month)
    };
    if lunar_day > max_day {
        return None;
    }

    let mut offset = 0i64;

    // 累加整年天数 / Accumulate full year days
    for y in 1900..lunar_year {
        offset += year_days(y) as i64;
    }

    // 累加当年月天数 / Accumulate month days in current year
    for m in 1..lunar_month {
        offset += month_days(lunar_year, m) as i64;
        // 闰月紧跟正常月之后 / Leap month follows its normal month
        if m == leap {
            offset += leap_days(lunar_year) as i64;
        }
    }

    // 如果目标是闰月，先加上对应正常月天数 / If target is leap month, add normal month first
    if is_leap && lunar_month == leap {
        offset += month_days(lunar_year, lunar_month) as i64;
    }

    // 加上当月天数偏移（从 1 开始，所以减 1）/ Add day offset (1-based, so -1)
    offset += (lunar_day - 1) as i64;

    // 从公历 1900-01-31 开始计算 / Compute from solar 1900-01-31
    let (sy, sm, sd) = solar_from_offset(offset);
    Some((sy, sm, sd))
}

/// 农历节日定义 / Lunar holiday definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LunarHoliday {
    /// 节日名称 / Holiday name
    pub name: &'static str,
    /// 农历月份 / Lunar month
    pub month: u32,
    /// 农历日期 / Lunar day
    pub day: u32,
    /// 是否为闰月 / Whether leap month
    pub is_leap: bool,
    /// 庆祝语 / Celebration greeting
    pub greeting: &'static str,
}

/// 六大农历节日 / Six major lunar holidays
pub const LUNAR_HOLIDAYS: [LunarHoliday; 6] = [
    LunarHoliday {
        name: "春节",
        month: 1,
        day: 1,
        is_leap: false,
        greeting: "新年快乐！恭贺新禧！",
    },
    LunarHoliday {
        name: "元宵节",
        month: 1,
        day: 15,
        is_leap: false,
        greeting: "元宵节快乐！",
    },
    LunarHoliday {
        name: "端午节",
        month: 5,
        day: 5,
        is_leap: false,
        greeting: "端午安康！",
    },
    LunarHoliday {
        name: "七夕",
        month: 7,
        day: 7,
        is_leap: false,
        greeting: "七夕快乐！",
    },
    LunarHoliday {
        name: "中秋节",
        month: 8,
        day: 15,
        is_leap: false,
        greeting: "中秋快乐！",
    },
    LunarHoliday {
        name: "重阳节",
        month: 9,
        day: 9,
        is_leap: false,
        greeting: "重阳安康！",
    },
];

/// 计算指定年份所有农历节日的公历日期 / Compute solar dates for all lunar holidays in a year
///
/// 返回 `Vec<(节日名, 公历月, 公历日, 庆祝语)>`。
/// Returns `Vec<(name, solar_month, solar_day, greeting)>`.
pub fn lunar_holidays_for_year(year: u32) -> Vec<(&'static str, u32, u32, &'static str)> {
    LUNAR_HOLIDAYS
        .iter()
        .filter_map(|h| {
            lunar_to_solar(year, h.month, h.day, h.is_leap)
                .map(|(_, sm, sd)| (h.name, sm, sd, h.greeting))
        })
        .collect()
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leap_month_2025() {
        // 2025年闰六月 / 2025 has leap 6th month
        assert_eq!(leap_month(2025), 6);
    }

    #[test]
    fn test_leap_month_2026() {
        // 2026年无闰月 / 2026 has no leap month
        assert_eq!(leap_month(2026), 0);
    }

    #[test]
    fn test_leap_month_2023() {
        // 2023年闰二月 / 2023 has leap 2nd month
        assert_eq!(leap_month(2023), 2);
    }

    #[test]
    fn test_year_days_2026() {
        // 2026年无闰月，354天 / 2026 no leap, 354 days
        assert_eq!(year_days(2026), 354);
    }

    #[test]
    fn test_lunar_to_solar_spring_2026() {
        // 2026年春节：农历正月初一 → 公历 2月17日
        // Spring Festival 2026: lunar 1/1 -> solar 2/17
        let result = lunar_to_solar(2026, 1, 1, false);
        assert_eq!(result, Some((2026, 2, 17)));
    }

    #[test]
    fn test_lunar_to_solar_lantern_2026() {
        // 2026年元宵节：农历正月十五 → 公历 3月3日
        // Lantern Festival 2026: lunar 1/15 -> solar 3/3
        let result = lunar_to_solar(2026, 1, 15, false);
        assert_eq!(result, Some((2026, 3, 3)));
    }

    #[test]
    fn test_lunar_to_solar_dragon_boat_2026() {
        // 2026年端午节：农历五月初五 → 公历 5月31日
        // Dragon Boat 2026: lunar 5/5 -> solar 5/31
        let result = lunar_to_solar(2026, 5, 5, false);
        assert_eq!(result, Some((2026, 6, 19)));
    }

    #[test]
    fn test_lunar_to_solar_qixi_2026() {
        // 2026年七夕：农历七月初七 → 公历 8月19日
        // Qixi 2026: lunar 7/7 -> solar 8/19
        let result = lunar_to_solar(2026, 7, 7, false);
        assert_eq!(result, Some((2026, 8, 19)));
    }

    #[test]
    fn test_lunar_to_solar_mid_autumn_2026() {
        // 2026年中秋节：农历八月十五 → 公历 9月25日
        // Mid-Autumn 2026: lunar 8/15 -> solar 9/25
        let result = lunar_to_solar(2026, 8, 15, false);
        assert_eq!(result, Some((2026, 9, 25)));
    }

    #[test]
    fn test_lunar_to_solar_double_ninth_2026() {
        // 2026年重阳节：农历九月初九 → 公历 10月21日
        // Double Ninth 2026: lunar 9/9 -> solar 10/21
        let result = lunar_to_solar(2026, 9, 9, false);
        assert_eq!(result, Some((2026, 10, 18)));
    }

    #[test]
    fn test_lunar_to_solar_spring_2024() {
        // 2024年春节：农历正月初一 → 公历 2月10日
        // Spring Festival 2024: lunar 1/1 -> solar 2/10
        let result = lunar_to_solar(2024, 1, 1, false);
        assert_eq!(result, Some((2024, 2, 10)));
    }

    #[test]
    fn test_lunar_to_solar_spring_2025() {
        // 2025年春节：农历正月初一 → 公历 1月29日
        // Spring Festival 2025: lunar 1/1 -> solar 1/29
        let result = lunar_to_solar(2025, 1, 1, false);
        assert_eq!(result, Some((2025, 1, 29)));
    }

    #[test]
    fn test_lunar_to_solar_invalid_year() {
        // 超出范围 / Out of range
        assert_eq!(lunar_to_solar(1899, 1, 1, false), None);
        assert_eq!(lunar_to_solar(2101, 1, 1, false), None);
    }

    #[test]
    fn test_lunar_to_solar_invalid_month() {
        assert_eq!(lunar_to_solar(2026, 0, 1, false), None);
        assert_eq!(lunar_to_solar(2026, 13, 1, false), None);
    }

    #[test]
    fn test_lunar_to_solar_invalid_leap() {
        // 2026年无闰月，请求闰月应返回 None
        // 2026 has no leap month, requesting leap should return None
        assert_eq!(lunar_to_solar(2026, 1, 1, true), None);
    }

    #[test]
    fn test_lunar_holidays_for_year_2026() {
        // 验证 2026 年全部 6 个农历节日 / Verify all 6 lunar holidays for 2026
        let holidays = lunar_holidays_for_year(2026);
        assert_eq!(holidays.len(), 6);

        // 春节 / Spring Festival
        assert_eq!(holidays[0], ("春节", 2, 17, "新年快乐！恭贺新禧！"));
        // 元宵 / Lantern
        assert_eq!(holidays[1], ("元宵节", 3, 3, "元宵节快乐！"));
        // 端午 / Dragon Boat
        assert_eq!(holidays[2], ("端午节", 6, 19, "端午安康！"));
        // 七夕 / Qixi
        assert_eq!(holidays[3], ("七夕", 8, 19, "七夕快乐！"));
        // 中秋 / Mid-Autumn
        assert_eq!(holidays[4], ("中秋节", 9, 25, "中秋快乐！"));
        // 重阳 / Double Ninth
        assert_eq!(holidays[5], ("重阳节", 10, 18, "重阳安康！"));
    }

    #[test]
    fn test_lunar_holidays_for_year_2025() {
        // 2025年春节应为 1月29日 / Spring Festival 2025 should be Jan 29
        let holidays = lunar_holidays_for_year(2025);
        assert_eq!(holidays.len(), 6);
        assert_eq!(holidays[0].0, "春节");
        assert_eq!(holidays[0].1, 1); // 月份 / month
        assert_eq!(holidays[0].2, 29); // 日期 / day
    }
}
