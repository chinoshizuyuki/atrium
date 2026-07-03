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

use serde::{Deserialize, Serialize};

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

// ── 公历转农历 / Solar to Lunar Conversion ──

/// 公历转农历 / Convert solar date to lunar date
///
/// # 参数 / Parameters
/// - `solar_year`: 公历年份 (1900-2100) / Solar year
/// - `solar_month`: 公历月份 (1-12) / Solar month
/// - `solar_day`: 公历日期 (1-31) / Solar day
///
/// # 返回 / Returns
/// - `Some((lunar_year, lunar_month, lunar_day, is_leap))` 转换成功 / Conversion success
/// - `None` 输入无效 / Invalid input
///
/// # 示例 / Example
/// ```no_run
/// use atrium_memory::lunar::solar_to_lunar;
/// // 2026-02-17 → 农历丙午年正月初一
/// let result = solar_to_lunar(2026, 2, 17);
/// assert_eq!(result, Some((2026, 1, 1, false)));
/// ```
pub fn solar_to_lunar(
    solar_year: u32,
    solar_month: u32,
    solar_day: u32,
) -> Option<(u32, u32, u32, bool)> {
    // 参数校验 / Parameter validation
    if !(1900..=2100).contains(&solar_year) {
        return None;
    }
    if !(1..=12).contains(&solar_month) {
        return None;
    }
    if solar_day == 0 || solar_day > 31 {
        return None;
    }

    // 日数校验 / Day-of-month validation
    let md = month_days_solar(solar_year);
    if solar_day as i64 > md[(solar_month - 1) as usize] {
        return None;
    }

    // 计算公历日期距 1900-01-31 的天数偏移 / Compute day offset from 1900-01-31
    let mut offset = 0i64;

    // 累加整年天数 / Accumulate full year days from 1900
    for y in 1900..solar_year {
        offset += if is_leap_solar(y) { 366i64 } else { 365i64 };
    }

    // 累加当年月天数 / Accumulate month days in current year
    for &days in md.iter().take((solar_month - 1) as usize) {
        offset += days;
    }

    // 加上当月天数偏移（1-based → 0-based）/ Add day offset (1-based → 0-based)
    offset += (solar_day - 1) as i64;

    // 1900-01-31 是农历 1900-01-01 的公历对应日
    // 1900-01-31 is the solar date of lunar 1900-01-01
    // 需要减去 30 天（1月1日到1月31日差 30 天）
    // Subtract 30 days (Jan 1 to Jan 31 = 30 days)
    offset -= 30;

    if offset < 0 {
        return None; // 1900-01-31 之前无法转换 / Cannot convert before 1900-01-31
    }

    // 遍历农历年，定位年份 / Walk lunar years to find the right year
    let mut lunar_year = 1900u32;
    loop {
        let yd = year_days(lunar_year) as i64;
        if offset < yd {
            break;
        }
        offset -= yd;
        lunar_year += 1;
    }

    // 遍历农历月，定位月份 / Walk lunar months to find the right month
    let leap = leap_month(lunar_year);
    let mut lunar_month = 1u32;
    let mut is_leap_month = false;

    for m in 1..=12u32 {
        let mdays = month_days(lunar_year, m) as i64;
        if offset < mdays {
            lunar_month = m;
            is_leap_month = false;
            break;
        }
        offset -= mdays;

        // 闰月紧跟正常月之后 / Leap month follows its normal month
        if m == leap {
            let ld = leap_days(lunar_year) as i64;
            if offset < ld {
                lunar_month = m;
                is_leap_month = true;
                break;
            }
            offset -= ld;
        }

        // 12月兜底 / December fallback
        if m == 12 {
            lunar_month = 12;
            is_leap_month = false;
        }
    }

    let lunar_day = (offset + 1) as u32;
    Some((lunar_year, lunar_month, lunar_day, is_leap_month))
}

// ── 农历日期 / Lunar Date ──

/// 农历日期 / Lunar date
///
/// 零依赖天文农历日期结构体，支持公历↔农历互转、中文格式化、节日检测。
/// Zero-dependency astronomical lunar date struct with bidirectional conversion,
/// Chinese formatting, and festival detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LunarDate {
    /// 农历年 (1900-2100) / Lunar year
    pub year: u32,
    /// 农历月 (1-12) / Lunar month
    pub month: u32,
    /// 农历日 (1-30) / Lunar day
    pub day: u32,
    /// 是否闰月 / Whether this is a leap month
    pub is_leap: bool,
}

impl LunarDate {
    /// 从公历日期转换 / Convert from Gregorian date
    pub fn from_gregorian(year: u32, month: u32, day: u32) -> Option<Self> {
        solar_to_lunar(year, month, day).map(|(y, m, d, leap)| Self {
            year: y,
            month: m,
            day: d,
            is_leap: leap,
        })
    }

    /// 转换为公历日期 / Convert to Gregorian date
    pub fn to_gregorian(&self) -> Option<(u32, u32, u32)> {
        lunar_to_solar(self.year, self.month, self.day, self.is_leap)
    }

    /// 从 epoch 秒转换 / Convert from epoch seconds
    pub fn from_epoch(epoch_secs: i64) -> Option<Self> {
        let (y, m, d) = epoch_to_ymd_internal(epoch_secs);
        Self::from_gregorian(y as u32, m as u32, d as u32)
    }

    /// 中文月名 / Chinese month name
    ///
    /// 正月、二月、...、腊月，闰月前加"闰"。
    pub fn month_name_zh(&self) -> &'static str {
        /// 农历月名表 / Lunar month name table
        const MONTH_NAMES: [&str; 12] = [
            "正月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "冬月",
            "腊月",
        ];
        // 闰月用特殊命名 / Leap month uses special naming
        // 由于返回 &'static str，闰月用 "闰X" 格式需要运行时构造
        // 这里返回基础月名，闰月标记由调用方通过 is_leap 判断
        MONTH_NAMES.get((self.month - 1) as usize).unwrap_or(&"?")
    }

    /// 中文月名（含闰月标记）/ Chinese month name with leap marker
    pub fn month_name_zh_full(&self) -> String {
        let base = self.month_name_zh();
        if self.is_leap {
            format!("闰{}", base)
        } else {
            base.to_string()
        }
    }

    /// 中文日名（初一、初二...三十）/ Chinese day name
    pub fn day_name_zh(&self) -> &'static str {
        /// 农历日名表 / Lunar day name table
        const DAY_NAMES: [&str; 30] = [
            "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十", "十一",
            "十二", "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十", "廿一", "廿二",
            "廿三", "廿四", "廿五", "廿六", "廿七", "廿八", "廿九", "三十",
        ];
        DAY_NAMES.get((self.day - 1) as usize).unwrap_or(&"?")
    }

    /// 完整中文日期 / Full Chinese date string
    ///
    /// 格式：丙午年正月初一
    pub fn full_name_zh(&self) -> String {
        format!(
            "{}年{}{}",
            self.year,
            self.month_name_zh_full(),
            self.day_name_zh()
        )
    }

    /// 检测是否为农历节日 / Detect if this is a lunar festival
    pub fn festival(&self) -> Option<LunarFestival> {
        // 先查固定月日节日 / Check fixed month/day festivals first
        if let Some(f) = LunarFestival::from_lunar(self.month, self.day, self.is_leap) {
            return Some(f);
        }
        // 除夕需要年份信息 / New Year's Eve needs year info
        LunarFestival::detect_new_years_eve(self.year, self.month, self.day, self.is_leap)
    }
}

/// epoch 秒转公历年月日（内部辅助）/ Epoch seconds to Gregorian (year, month, day) — internal helper
fn epoch_to_ymd_internal(epoch_secs: i64) -> (i32, i32, i32) {
    let mut days = epoch_secs / 86400;
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap_solar(year as u32) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_solar(year as u32);
    let month_days: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1i64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year as i32, month as i32, (days + 1) as i32)
}

// ── 农历节日枚举 / Lunar Festival Enum ──

/// 农历节日 / Lunar festival
///
/// 九大传统农历节日，覆盖春节、元宵、端午、七夕、中元、中秋、重阳、腊八、除夕。
/// Nine traditional lunar festivals covering the major Chinese cultural dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LunarFestival {
    /// 春节（正月初一）/ Spring Festival
    SpringFestival,
    /// 元宵节（正月十五）/ Lantern Festival
    LanternFestival,
    /// 端午节（五月初五）/ Dragon Boat Festival
    DragonBoat,
    /// 七夕（七月初七）/ Qixi Festival
    Qixi,
    /// 中元节（七月十五）/ Ghost Festival
    GhostFestival,
    /// 中秋节（八月十五）/ Mid-Autumn Festival
    MidAutumn,
    /// 重阳节（九月初九）/ Double Ninth Festival
    DoubleNinth,
    /// 腊八节（十二月初八）/ Laba Festival
    Laba,
    /// 除夕（腊月最后一天）/ Lunar New Year's Eve
    NewYearsEve,
}

impl LunarFestival {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::SpringFestival => "春节",
            Self::LanternFestival => "元宵节",
            Self::DragonBoat => "端午节",
            Self::Qixi => "七夕",
            Self::GhostFestival => "中元节",
            Self::MidAutumn => "中秋节",
            Self::DoubleNinth => "重阳节",
            Self::Laba => "腊八节",
            Self::NewYearsEve => "除夕",
        }
    }

    /// 庆祝模板 / Celebration template
    pub fn celebration_template(&self) -> &'static str {
        match self {
            Self::SpringFestival => "新年快乐！恭贺新禧！",
            Self::LanternFestival => "元宵节快乐！",
            Self::DragonBoat => "端午安康！",
            Self::Qixi => "七夕快乐！",
            Self::GhostFestival => "中元节，思念故人。",
            Self::MidAutumn => "中秋快乐！月圆人团圆。",
            Self::DoubleNinth => "重阳安康！",
            Self::Laba => "腊八节快乐！",
            Self::NewYearsEve => "除夕快乐！辞旧迎新！",
        }
    }

    /// 情感签名（PAD 偏移）/ Emotional signature (PAD offset)
    ///
    /// 不同节日对数字生命情绪的不同影响：
    /// - 春节：高愉悦 + 高唤醒（喜庆）/ Spring: high pleasure + high arousal
    /// - 中秋：高愉悦 + 低唤醒（温馨）/ Mid-Autumn: high pleasure + low arousal
    /// - 中元：低愉悦 + 低唤醒（肃穆）/ Ghost: low pleasure + low arousal
    pub fn pad_offset(&self) -> [f32; 3] {
        match self {
            Self::SpringFestival => [0.35, 0.20, 0.05],
            Self::LanternFestival => [0.25, 0.15, 0.02],
            Self::DragonBoat => [0.15, 0.20, 0.00],
            Self::Qixi => [0.20, 0.05, -0.05],
            Self::GhostFestival => [-0.15, -0.10, -0.05],
            Self::MidAutumn => [0.30, 0.05, 0.05],
            Self::DoubleNinth => [0.10, -0.05, 0.00],
            Self::Laba => [0.10, 0.00, 0.00],
            Self::NewYearsEve => [0.25, 0.15, 0.05],
        }
    }

    /// 从农历月日识别节日（固定日期）/ Detect from lunar month/day (fixed dates)
    ///
    /// 除夕需要年份信息，此方法不检测除夕。
    /// New Year's Eve needs year info; this method does not detect it.
    pub fn from_lunar(month: u32, day: u32, is_leap: bool) -> Option<Self> {
        if is_leap {
            return None; // 闰月无传统节日 / No traditional festivals in leap months
        }
        match (month, day) {
            (1, 1) => Some(Self::SpringFestival),
            (1, 15) => Some(Self::LanternFestival),
            (5, 5) => Some(Self::DragonBoat),
            (7, 7) => Some(Self::Qixi),
            (7, 15) => Some(Self::GhostFestival),
            (8, 15) => Some(Self::MidAutumn),
            (9, 9) => Some(Self::DoubleNinth),
            (12, 8) => Some(Self::Laba),
            _ => None,
        }
    }

    /// 检测除夕（需要年份信息）/ Detect New Year's Eve (needs year info)
    ///
    /// 除夕是腊月最后一天（大月三十，小月二十九）。
    pub fn detect_new_years_eve(
        lunar_year: u32,
        month: u32,
        day: u32,
        is_leap: bool,
    ) -> Option<Self> {
        if month == 12 && !is_leap {
            let last_day = month_days(lunar_year, 12);
            if day == last_day {
                return Some(Self::NewYearsEve);
            }
        }
        None
    }

    /// 计算指定年份此节日的公历日期 / Compute solar date for this festival in a given year
    pub fn solar_date_in_year(&self, lunar_year: u32) -> Option<(u32, u32, u32)> {
        match self {
            Self::NewYearsEve => {
                // 除夕：腊月最后一天 / Last day of 12th month
                let last_day = month_days(lunar_year, 12);
                lunar_to_solar(lunar_year, 12, last_day, false)
            }
            Self::SpringFestival => lunar_to_solar(lunar_year, 1, 1, false),
            Self::LanternFestival => lunar_to_solar(lunar_year, 1, 15, false),
            Self::DragonBoat => lunar_to_solar(lunar_year, 5, 5, false),
            Self::Qixi => lunar_to_solar(lunar_year, 7, 7, false),
            Self::GhostFestival => lunar_to_solar(lunar_year, 7, 15, false),
            Self::MidAutumn => lunar_to_solar(lunar_year, 8, 15, false),
            Self::DoubleNinth => lunar_to_solar(lunar_year, 9, 9, false),
            Self::Laba => lunar_to_solar(lunar_year, 12, 8, false),
        }
    }
}

/// 九大农历节日列表 / Nine major lunar festivals
pub const LUNAR_FESTIVALS: [LunarFestival; 9] = [
    LunarFestival::SpringFestival,
    LunarFestival::LanternFestival,
    LunarFestival::DragonBoat,
    LunarFestival::Qixi,
    LunarFestival::GhostFestival,
    LunarFestival::MidAutumn,
    LunarFestival::DoubleNinth,
    LunarFestival::Laba,
    LunarFestival::NewYearsEve,
];

/// 计算指定年份所有农历节日的公历日期 / Compute solar dates for all lunar festivals in a year
///
/// 返回 `Vec<(节日, 公历月, 公历日, 庆祝语)>`。
/// Returns `Vec<(festival, solar_month, solar_day, greeting)>`.
pub fn lunar_festivals_for_year(year: u32) -> Vec<(LunarFestival, u32, u32, &'static str)> {
    LUNAR_FESTIVALS
        .iter()
        .filter_map(|&f| {
            f.solar_date_in_year(year)
                .map(|(_, sm, sd)| (f, sm, sd, f.celebration_template()))
        })
        .collect()
}

/// 公历年月日转 epoch 秒 / Gregorian (year, month, day) to epoch seconds
///
/// 用于将农历转公历后的日期转回 epoch 秒。
/// Used to convert lunar→solar dates back to epoch seconds.
pub fn ymd_to_epoch(year: i32, month: i32, day: i32) -> i64 {
    let mut total_days = 0i64;
    for y in 1970..year {
        total_days += if is_leap_solar(y as u32) { 366 } else { 365 };
    }
    let leap = is_leap_solar(year as u32);
    let month_days: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for &days in month_days.iter().take((month - 1) as usize) {
        total_days += days;
    }
    total_days += (day - 1) as i64;
    total_days * 86400
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

    // ══════════════════════════════════════════════════════════════
    // P2-1: solar_to_lunar + LunarDate + LunarFestival 测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_solar_to_lunar_spring_2026() {
        // 2026-02-17 → 农历正月初一 / Spring Festival 2026
        let result = solar_to_lunar(2026, 2, 17);
        assert_eq!(result, Some((2026, 1, 1, false)));
    }

    #[test]
    fn test_solar_to_lunar_mid_autumn_2026() {
        // 2026-09-25 → 农历八月十五 / Mid-Autumn 2026
        let result = solar_to_lunar(2026, 9, 25);
        assert_eq!(result, Some((2026, 8, 15, false)));
    }

    #[test]
    fn test_solar_to_lunar_dragon_boat_2026() {
        // 2026-06-19 → 农历五月初五 / Dragon Boat 2026
        let result = solar_to_lunar(2026, 6, 19);
        assert_eq!(result, Some((2026, 5, 5, false)));
    }

    #[test]
    fn test_solar_to_lunar_qixi_2026() {
        // 2026-08-19 → 农历七月初七 / Qixi 2026
        let result = solar_to_lunar(2026, 8, 19);
        assert_eq!(result, Some((2026, 7, 7, false)));
    }

    #[test]
    fn test_solar_to_lunar_roundtrip_2026() {
        // 公历→农历→公历 往返一致性 / Roundtrip: solar→lunar→solar
        for (sm, sd) in [(1, 15), (3, 8), (6, 20), (9, 25), (12, 31)] {
            let lunar = solar_to_lunar(2026, sm, sd).expect("solar_to_lunar failed");
            let solar =
                lunar_to_solar(lunar.0, lunar.1, lunar.2, lunar.3).expect("lunar_to_solar failed");
            assert_eq!(solar, (2026, sm, sd), "roundtrip failed for {}/{}", sm, sd);
        }
    }

    #[test]
    fn test_solar_to_lunar_invalid() {
        assert_eq!(solar_to_lunar(1899, 1, 1), None);
        assert_eq!(solar_to_lunar(2101, 1, 1), None);
        assert_eq!(solar_to_lunar(2026, 0, 1), None);
        assert_eq!(solar_to_lunar(2026, 13, 1), None);
        assert_eq!(solar_to_lunar(2026, 2, 30), None); // 2026年2月无30日
    }

    #[test]
    fn test_lunar_date_from_gregorian() {
        let ld = LunarDate::from_gregorian(2026, 2, 17).unwrap();
        assert_eq!(ld.year, 2026);
        assert_eq!(ld.month, 1);
        assert_eq!(ld.day, 1);
        assert!(!ld.is_leap);
    }

    #[test]
    fn test_lunar_date_to_gregorian() {
        let ld = LunarDate {
            year: 2026,
            month: 8,
            day: 15,
            is_leap: false,
        };
        let (y, m, d) = ld.to_gregorian().unwrap();
        assert_eq!((y, m, d), (2026, 9, 25));
    }

    #[test]
    fn test_lunar_date_month_name_zh() {
        let ld = LunarDate {
            year: 2026,
            month: 1,
            day: 1,
            is_leap: false,
        };
        assert_eq!(ld.month_name_zh(), "正月");
        assert_eq!(ld.month_name_zh_full(), "正月");

        let ld_leap = LunarDate {
            year: 2023,
            month: 2,
            day: 1,
            is_leap: true,
        };
        assert_eq!(ld_leap.month_name_zh_full(), "闰二月");
    }

    #[test]
    fn test_lunar_date_day_name_zh() {
        let ld1 = LunarDate {
            year: 2026,
            month: 1,
            day: 1,
            is_leap: false,
        };
        assert_eq!(ld1.day_name_zh(), "初一");

        let ld15 = LunarDate {
            year: 2026,
            month: 1,
            day: 15,
            is_leap: false,
        };
        assert_eq!(ld15.day_name_zh(), "十五");

        let ld30 = LunarDate {
            year: 2026,
            month: 1,
            day: 30,
            is_leap: false,
        };
        assert_eq!(ld30.day_name_zh(), "三十");

        let ld21 = LunarDate {
            year: 2026,
            month: 1,
            day: 21,
            is_leap: false,
        };
        assert_eq!(ld21.day_name_zh(), "廿一");
    }

    #[test]
    fn test_lunar_date_full_name_zh() {
        let ld = LunarDate {
            year: 2026,
            month: 1,
            day: 1,
            is_leap: false,
        };
        assert_eq!(ld.full_name_zh(), "2026年正月初一");
    }

    #[test]
    fn test_lunar_date_festival() {
        let spring = LunarDate {
            year: 2026,
            month: 1,
            day: 1,
            is_leap: false,
        };
        assert_eq!(spring.festival(), Some(LunarFestival::SpringFestival));

        let mid_autumn = LunarDate {
            year: 2026,
            month: 8,
            day: 15,
            is_leap: false,
        };
        assert_eq!(mid_autumn.festival(), Some(LunarFestival::MidAutumn));

        let normal = LunarDate {
            year: 2026,
            month: 3,
            day: 10,
            is_leap: false,
        };
        assert_eq!(normal.festival(), None);
    }

    #[test]
    fn test_lunar_festival_label_zh() {
        assert_eq!(LunarFestival::SpringFestival.label_zh(), "春节");
        assert_eq!(LunarFestival::MidAutumn.label_zh(), "中秋节");
        assert_eq!(LunarFestival::NewYearsEve.label_zh(), "除夕");
        assert_eq!(LunarFestival::Laba.label_zh(), "腊八节");
        assert_eq!(LunarFestival::GhostFestival.label_zh(), "中元节");
    }

    #[test]
    fn test_lunar_festival_from_lunar() {
        assert_eq!(
            LunarFestival::from_lunar(1, 1, false),
            Some(LunarFestival::SpringFestival)
        );
        assert_eq!(
            LunarFestival::from_lunar(8, 15, false),
            Some(LunarFestival::MidAutumn)
        );
        assert_eq!(
            LunarFestival::from_lunar(12, 8, false),
            Some(LunarFestival::Laba)
        );
        assert_eq!(
            LunarFestival::from_lunar(7, 15, false),
            Some(LunarFestival::GhostFestival)
        );
        // 闰月无节日 / No festival in leap month
        assert_eq!(LunarFestival::from_lunar(1, 1, true), None);
    }

    #[test]
    fn test_lunar_festival_detect_new_years_eve() {
        // 2026年腊月：查表确认最后一天 / Check last day of 12th month in 2026
        let last_day_12 = month_days(2026, 12);
        let result = LunarFestival::detect_new_years_eve(2026, 12, last_day_12, false);
        assert_eq!(result, Some(LunarFestival::NewYearsEve));
        // 非最后一天不是除夕 / Not last day → not New Year's Eve
        assert_eq!(
            LunarFestival::detect_new_years_eve(2026, 12, 1, false),
            None
        );
    }

    #[test]
    fn test_lunar_festival_solar_date_in_year() {
        // 2026年春节公历日期 / Spring Festival 2026 solar date
        let (y, m, d) = LunarFestival::SpringFestival
            .solar_date_in_year(2026)
            .unwrap();
        assert_eq!((y, m, d), (2026, 2, 17));

        // 2026年中秋公历日期 / Mid-Autumn 2026 solar date
        let (y, m, d) = LunarFestival::MidAutumn.solar_date_in_year(2026).unwrap();
        assert_eq!((y, m, d), (2026, 9, 25));
    }

    #[test]
    fn test_lunar_festival_pad_offset() {
        // 春节应为正愉悦 / Spring Festival should have positive pleasure
        let pad = LunarFestival::SpringFestival.pad_offset();
        assert!(pad[0] > 0.0, "Spring pleasure should be positive");

        // 中元节应为负愉悦 / Ghost Festival should have negative pleasure
        let pad = LunarFestival::GhostFestival.pad_offset();
        assert!(pad[0] < 0.0, "Ghost pleasure should be negative");

        // 中秋应为正愉悦低唤醒 / Mid-Autumn should have positive pleasure, low arousal
        let pad = LunarFestival::MidAutumn.pad_offset();
        assert!(pad[0] > 0.0, "Mid-Autumn pleasure should be positive");
        assert!(
            pad[1] < LunarFestival::SpringFestival.pad_offset()[1],
            "Mid-Autumn arousal should be lower than Spring"
        );
    }

    #[test]
    fn test_lunar_festivals_for_year_2026() {
        let festivals = lunar_festivals_for_year(2026);
        assert_eq!(festivals.len(), 9, "Should have 9 lunar festivals");

        // 春节 / Spring Festival
        assert_eq!(festivals[0].0, LunarFestival::SpringFestival);
        assert_eq!(festivals[0].1, 2); // month
        assert_eq!(festivals[0].2, 17); // day

        // 中秋 / Mid-Autumn
        let mid_autumn = festivals
            .iter()
            .find(|f| f.0 == LunarFestival::MidAutumn)
            .unwrap();
        assert_eq!(mid_autumn.1, 9);
        assert_eq!(mid_autumn.2, 25);
    }

    #[test]
    fn test_ymd_to_epoch() {
        // 1970-01-01 = epoch 0
        assert_eq!(ymd_to_epoch(1970, 1, 1), 0);

        // 1970-01-02 = 86400
        assert_eq!(ymd_to_epoch(1970, 1, 2), 86400);

        // 2000-01-01 应为正数 / Should be positive
        assert!(ymd_to_epoch(2000, 1, 1) > 0);
    }

    #[test]
    fn test_lunar_date_from_epoch() {
        // 2026-02-17 = 春节 / Spring Festival 2026
        let epoch = ymd_to_epoch(2026, 2, 17);
        let ld = LunarDate::from_epoch(epoch).unwrap();
        assert_eq!(ld.month, 1);
        assert_eq!(ld.day, 1);
        assert!(!ld.is_leap);
    }

    #[test]
    fn test_solar_to_lunar_leap_month_2023() {
        // 2023年闰二月 / 2023 has leap 2nd month
        // 2023-03-22 = 闰二月初一 / 2023-03-22 = leap 2/1
        let result = solar_to_lunar(2023, 3, 22);
        assert!(result.is_some());
        let (_, m, d, is_leap) = result.unwrap();
        assert_eq!(m, 2);
        assert_eq!(d, 1);
        assert!(is_leap, "Should be leap month");
    }

    #[test]
    fn test_solar_to_lunar_spring_2024() {
        // 2024年春节：公历 2月10日 / Spring Festival 2024: solar Feb 10
        let result = solar_to_lunar(2024, 2, 10);
        assert_eq!(result, Some((2024, 1, 1, false)));
    }

    #[test]
    fn test_solar_to_lunar_spring_2025() {
        // 2025年春节：公历 1月29日 / Spring Festival 2025: solar Jan 29
        let result = solar_to_lunar(2025, 1, 29);
        assert_eq!(result, Some((2025, 1, 1, false)));
    }
}
