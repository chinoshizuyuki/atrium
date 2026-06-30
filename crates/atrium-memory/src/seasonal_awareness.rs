// ════════════════════════════════════════════════════════════════════
// SeasonalAwareness — 季节与节日感知 / Seasonal & Holiday Awareness
// ════════════════════════════════════════════════════════════════════
//
// 感知当前季节和节日，为对话注入季节性语境。
// 例如：冬天说"外面冷吧"，春节说"新年快乐"。
//
// 支持的节日类型：
//   - 固定日期节日（元旦、国庆等）
//   - 农历节日（春节、中秋等，动态计算公历映射）
//   - 季节感知（春夏秋冬的情绪色彩）
//
// C4 改进：农历节日不再硬编码 2026 日期，改为通过 lunar 模块
// 动态计算，每年 1 月 1 日自动重算当年农历节日缓存。

use serde::{Deserialize, Serialize};

use crate::lunar;

// ── 季节 / Season ──

/// 四季 / Four seasons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring, // 3-5月
    Summer, // 6-8月
    Autumn, // 9-11月
    Winter, // 12-2月
}

impl Season {
    /// 从月份推断季节 / Infer season from month (1-12)
    pub fn from_month(month: u32) -> Self {
        match month {
            3..=5 => Self::Spring,
            6..=8 => Self::Summer,
            9..=11 => Self::Autumn,
            _ => Self::Winter, // 12, 1, 2
        }
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Spring => "春天",
            Self::Summer => "夏天",
            Self::Autumn => "秋天",
            Self::Winter => "冬天",
        }
    }

    /// 季节情绪色彩提示 / Seasonal mood color hint
    pub fn mood_hint(&self) -> &'static str {
        match self {
            Self::Spring => "万物复苏，充满希望",
            Self::Summer => "阳光热烈，活力充沛",
            Self::Autumn => "天高气爽，沉淀思考",
            Self::Winter => "寒意渐浓，温暖相伴",
        }
    }

    /// 季节 PAD 基础调制量 / Seasonal PAD base modulation deltas
    ///
    /// 返回 (pleasure_delta, arousal_delta, dominance_delta) 基础调制量。
    /// 这些是微调值（绝对值 ≤ 0.05），用于在现有情感状态上叠加季节色彩。
    ///
    /// 设计原则：
    ///   - Spring: +P +A — 希望、活力 / Hopeful, energetic
    ///   - Summer: +P ++A — 热烈、活跃 / Vibrant, active
    ///   - Autumn: -P -A — 沉静、思考 / Contemplative, calm
    ///   - Winter: -P -A -D — 内敛、温暖 / Inward, cozy
    ///
    /// Returns (pleasure_delta, arousal_delta, dominance_delta) base modulation.
    /// These are subtle values (|delta| <= 0.05) for overlaying seasonal color
    /// onto the existing emotion state.
    pub fn pad_modulation(&self) -> (f32, f32, f32) {
        match self {
            // 春天：希望、活力 / Spring: hopeful, energetic
            Self::Spring => (0.05, 0.03, 0.02),
            // 夏天：热烈、活跃 / Summer: vibrant, active
            Self::Summer => (0.03, 0.05, 0.01),
            // 秋天：沉静、思考 / Autumn: contemplative, calm
            Self::Autumn => (-0.02, -0.03, 0.01),
            // 冬天：内敛、温暖 / Winter: inward, cozy
            Self::Winter => (-0.04, -0.05, -0.02),
        }
    }

    /// 季节序数（跨 crate 传递用）/ Season ordinal for cross-crate use
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Spring => 0,
            Self::Summer => 1,
            Self::Autumn => 2,
            Self::Winter => 3,
        }
    }
}

// ── 节日 / Holiday ──

/// 节日类型 / Holiday type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holiday {
    /// 节日名称 / Holiday name
    pub name: String,
    /// 月份（1-12）/ Month (1-12)
    pub month: u32,
    /// 日期（1-31）/ Day (1-31)
    pub day: u32,
    /// 是否为农历节日（动态映射到公历）/ Whether lunar (dynamic solar mapping)
    pub is_lunar: bool,
    /// 庆祝语 / Celebration greeting
    pub greeting: String,
}

impl Holiday {
    /// 判断今天是否是此节日 / Check if today is this holiday
    pub fn is_today(&self, month: u32, day: u32) -> bool {
        self.month == month && self.day == day
    }

    /// 距今天的天数（正数=未来，负数=已过）/ Days from today
    pub fn days_from(&self, month: u32, day: u32) -> i32 {
        let self_doy = month_day_to_doy(self.month, self.day);
        let now_doy = month_day_to_doy(month, day);
        self_doy as i32 - now_doy as i32
    }
}

/// 月份日转年内天数 / Month-day to day-of-year
fn month_day_to_doy(month: u32, day: u32) -> u32 {
    const CUM_DAYS: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    CUM_DAYS.get(month as usize - 1).copied().unwrap_or(0) + day
}

// ── 季节情绪 PAD 调制 / Seasonal Emotion PAD Modulation ──

/// 关系阶段对季节调制的缩放因子 / Relationship stage scaling for seasonal modulation
///
/// 关系越深，季节对情绪的调制越强。
/// Deeper relationships allow stronger seasonal modulation of emotion.
///
/// | 关系阶段 | ordinal | 缩放因子 |
/// |----------|---------|----------|
/// | Acquaintance | 0 | 0.5 |
/// | Familiar     | 1 | 0.75 |
/// | Trusted      | 2 | 1.0 |
/// | Deep         | 3 | 1.2 |
fn relationship_scale(ordinal: u8) -> f32 {
    match ordinal {
        0 => 0.5,  // 初识：轻微调制 / Acquaintance: subtle modulation
        1 => 0.75, // 熟悉：中等调制 / Familiar: moderate modulation
        2 => 1.0,  // 信任：标准调制 / Trusted: standard modulation
        3 => 1.2,  // 深度：强化调制 / Deep: enhanced modulation
        _ => 1.0,  // 未知：标准 / Unknown: standard
    }
}

/// 季节情绪 PAD 调制结果 / Seasonal PAD modulation result
///
/// 包含经关系阶段缩放后的 PAD 调制量和相关元信息。
/// Contains relationship-scaled PAD modulation deltas and related metadata.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SeasonalPadModulation {
    /// 愉悦度调制量 / Pleasure modulation delta
    pub pleasure_delta: f32,
    /// 唤醒度调制量 / Arousal modulation delta
    pub arousal_delta: f32,
    /// 支配度调制量 / Dominance modulation delta
    pub dominance_delta: f32,
    /// 源季节 / Source season
    pub season: Season,
    /// 关系阶段缩放因子 / Relationship stage scale factor
    pub scale: f32,
}

impl SeasonalPadModulation {
    /// 零调制 / Zero modulation (no-op)
    pub fn zero() -> Self {
        Self {
            pleasure_delta: 0.0,
            arousal_delta: 0.0,
            dominance_delta: 0.0,
            season: Season::Spring,
            scale: 0.0,
        }
    }

    /// 是否为零调制 / Whether this is a zero modulation
    pub fn is_zero(&self) -> bool {
        self.pleasure_delta == 0.0 && self.arousal_delta == 0.0 && self.dominance_delta == 0.0
    }

    /// 调制幅度（L2 范数）/ Modulation magnitude (L2 norm)
    pub fn magnitude(&self) -> f32 {
        let p2 = self.pleasure_delta * self.pleasure_delta;
        let a2 = self.arousal_delta * self.arousal_delta;
        let d2 = self.dominance_delta * self.dominance_delta;
        (p2 + a2 + d2).sqrt()
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self) -> String {
        if self.is_zero() {
            return "无季节调制".to_string();
        }
        let p_dir = if self.pleasure_delta > 0.0 { "+" } else { "" };
        let a_dir = if self.arousal_delta > 0.0 { "+" } else { "" };
        let d_dir = if self.dominance_delta > 0.0 { "+" } else { "" };
        format!(
            "{}季节调制: P{}{:.3} A{}{:.3} D{}{:.3} (x{:.2})",
            self.season.label_zh(),
            p_dir,
            self.pleasure_delta,
            a_dir,
            self.arousal_delta,
            d_dir,
            self.dominance_delta,
            self.scale
        )
    }
}

// ── 季节感知系统 / Seasonal Awareness System ──

/// 季节与节日感知系统 / Seasonal & holiday awareness system
///
/// 农历节日通过 `lunar` 模块动态计算，每年 1 月 1 日自动重算缓存。
/// Lunar holidays are dynamically computed via the `lunar` module,
/// with automatic cache refresh on each new year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalAwareness {
    /// 已知节日列表 / Known holidays
    pub holidays: Vec<Holiday>,
    /// 农历缓存年份 / Cached lunar year (for auto-refresh)
    #[serde(default = "default_cached_year")]
    pub cached_year: u32,
}

fn default_cached_year() -> u32 {
    current_year()
}

/// 获取当前公历年份 / Get current solar year
fn current_year() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut days = secs / 86400;
    let mut year = 1970u32;
    loop {
        #[allow(unknown_lints)]
        #[allow(clippy::manual_is_multiple_of)]
        let diy = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
            366u64
        } else {
            365u64
        };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }
    year
}

impl SeasonalAwareness {
    /// 创建当前年份的季节感知 / Create for current year
    pub fn new() -> Self {
        let year = current_year();
        Self::for_year(year)
    }

    /// 创建指定年份的季节感知 / Create for a specific year
    ///
    /// 农历节日通过 `lunar` 模块动态计算。
    /// Lunar holidays are dynamically computed via the `lunar` module.
    pub fn for_year(year: u32) -> Self {
        Self {
            holidays: Self::build_holidays(year),
            cached_year: year,
        }
    }

    /// 刷新农历节日缓存 / Refresh lunar holiday cache for a new year
    ///
    /// 如果年份未变则不重算；年份变化时替换所有农历节日。
    /// No-op if year unchanged; replaces all lunar holidays when year changes.
    pub fn refresh_for_year(&mut self, year: u32) {
        if year == self.cached_year {
            return;
        }
        // 移除旧农历节日 / Remove old lunar holidays
        self.holidays.retain(|h| !h.is_lunar);
        // 追加新农历节日 / Append new lunar holidays
        let lunar_holidays = Self::build_lunar_holidays(year);
        self.holidays.extend(lunar_holidays);
        self.cached_year = year;
    }

    /// 构建完整节日列表 / Build complete holiday list for a year
    fn build_holidays(year: u32) -> Vec<Holiday> {
        let mut holidays = Self::fixed_holidays();
        holidays.extend(Self::build_lunar_holidays(year));
        holidays
    }

    /// 固定公历节日 / Fixed solar holidays
    fn fixed_holidays() -> Vec<Holiday> {
        vec![
            Holiday {
                name: "元旦".to_string(),
                month: 1,
                day: 1,
                is_lunar: false,
                greeting: "新年快乐！".to_string(),
            },
            Holiday {
                name: "情人节".to_string(),
                month: 2,
                day: 14,
                is_lunar: false,
                greeting: "情人节快乐！".to_string(),
            },
            Holiday {
                name: "妇女节".to_string(),
                month: 3,
                day: 8,
                is_lunar: false,
                greeting: "节日快乐！".to_string(),
            },
            Holiday {
                name: "劳动节".to_string(),
                month: 5,
                day: 1,
                is_lunar: false,
                greeting: "劳动节快乐！".to_string(),
            },
            Holiday {
                name: "儿童节".to_string(),
                month: 6,
                day: 1,
                is_lunar: false,
                greeting: "儿童节快乐！".to_string(),
            },
            Holiday {
                name: "国庆节".to_string(),
                month: 10,
                day: 1,
                is_lunar: false,
                greeting: "国庆快乐！".to_string(),
            },
            Holiday {
                name: "圣诞节".to_string(),
                month: 12,
                day: 25,
                is_lunar: false,
                greeting: "圣诞快乐！".to_string(),
            },
        ]
    }

    /// 动态计算农历节日 / Dynamically compute lunar holidays for a year
    fn build_lunar_holidays(year: u32) -> Vec<Holiday> {
        lunar::lunar_holidays_for_year(year)
            .into_iter()
            .map(|(name, month, day, greeting)| Holiday {
                name: name.to_string(),
                month,
                day,
                is_lunar: true,
                greeting: greeting.to_string(),
            })
            .collect()
    }

    /// 获取当前季节 / Get current season
    pub fn current_season(&self, month: u32) -> Season {
        Season::from_month(month)
    }

    /// 检查今日是否有节日 / Check if today has a holiday
    pub fn today_holidays(&self, month: u32, day: u32) -> Vec<&Holiday> {
        self.holidays
            .iter()
            .filter(|h| h.is_today(month, day))
            .collect()
    }

    /// 获取即将到来的节日（7天内）/ Get upcoming holidays (within 7 days)
    pub fn upcoming_holidays(&self, month: u32, day: u32) -> Vec<(&Holiday, i32)> {
        self.holidays
            .iter()
            .filter_map(|h| {
                let days = h.days_from(month, day);
                if days > 0 && days <= 7 {
                    Some((h, days))
                } else {
                    None
                }
            })
            .collect()
    }

    /// 生成季节 prompt 片段 / Generate seasonal prompt fragment
    ///
    /// 注入当前季节情绪色彩和节日信息。
    pub fn prompt_fragment(&self, month: u32, day: u32) -> String {
        let mut parts = Vec::new();

        // 季节 / Season
        let season = self.current_season(month);
        parts.push(format!(
            "[季节] {}，{}",
            season.label_zh(),
            season.mood_hint()
        ));

        // 今日节日 / Today's holidays
        let today = self.today_holidays(month, day);
        if !today.is_empty() {
            let names: Vec<&str> = today.iter().map(|h| h.name.as_str()).collect();
            parts.push(format!("[节日] 今天是{}", names.join("、")));
        }

        // 即将到来的节日 / Upcoming holidays
        let upcoming = self.upcoming_holidays(month, day);
        if !upcoming.is_empty() {
            let items: Vec<String> = upcoming
                .iter()
                .map(|(h, d)| format!("{}（{}天后）", h.name, d))
                .collect();
            parts.push(format!("[即将] {}", items.join("、")));
        }

        parts.join("\n")
    }

    /// 添加自定义节日 / Add custom holiday
    pub fn add_holiday(&mut self, holiday: Holiday) {
        self.holidays.push(holiday);
    }

    // ── C6: 季节情绪 PAD 调制 / C6: Seasonal Emotion PAD Modulation ──

    /// 计算季节情绪 PAD 调制量 / Compute seasonal PAD modulation
    ///
    /// 根据当前月份和关系阶段，计算季节对情感状态的 PAD 调制量。
    /// Computes PAD modulation deltas based on current month and relationship stage.
    ///
    /// @param month 当前月份 (1-12) / Current month (1-12)
    /// @param relationship_ordinal 关系阶段序数 / Relationship stage ordinal
    ///   (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    /// @return 季节 PAD 调制结果 / Seasonal PAD modulation result
    pub fn seasonal_pad_adjustment(
        &self,
        month: u32,
        relationship_ordinal: u8,
    ) -> SeasonalPadModulation {
        let season = self.current_season(month);
        let scale = relationship_scale(relationship_ordinal);
        let (p_base, a_base, d_base) = season.pad_modulation();
        SeasonalPadModulation {
            pleasure_delta: p_base * scale,
            arousal_delta: a_base * scale,
            dominance_delta: d_base * scale,
            season,
            scale,
        }
    }

    /// 将季节 PAD 调制应用到情感状态 / Apply seasonal PAD modulation to emotion state
    ///
    /// 这是一个便捷方法，直接返回调制后的 (pleasure, arousal, dominance) 元组。
    /// Convenience method returning modulated (P, A, D) tuple.
    ///
    /// @param base_pleasure 基础愉悦度 / Base pleasure
    /// @param base_arousal 基础唤醒度 / Base arousal
    /// @param base_dominance 基础支配度 / Base dominance
    /// @param month 当前月份 / Current month
    /// @param relationship_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @return 调制后的 (P, A, D) / Modulated (P, A, D), clamped to [-1, 1]
    pub fn apply_seasonal_pad(
        &self,
        base_pleasure: f32,
        base_arousal: f32,
        base_dominance: f32,
        month: u32,
        relationship_ordinal: u8,
    ) -> (f32, f32, f32) {
        let modulation = self.seasonal_pad_adjustment(month, relationship_ordinal);
        let p = (base_pleasure + modulation.pleasure_delta).clamp(-1.0, 1.0);
        let a = (base_arousal + modulation.arousal_delta).clamp(-1.0, 1.0);
        let d = (base_dominance + modulation.dominance_delta).clamp(-1.0, 1.0);
        (p, a, d)
    }
}

impl Default for SeasonalAwareness {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 / Helper ──

/// epoch 秒转月日 / Epoch seconds to (month, day)
pub fn epoch_to_month_day(epoch_secs: i64) -> (u32, u32) {
    let mut days = epoch_secs / 86400;
    let mut year = 1970i64;
    loop {
        let diy = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
            366
        } else {
            365
        };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }
    #[allow(unknown_lints)]
    #[allow(clippy::manual_is_multiple_of)]
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let md: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &d in &md {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    (month, (days + 1) as u32)
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_from_month() {
        assert_eq!(Season::from_month(1), Season::Winter);
        assert_eq!(Season::from_month(3), Season::Spring);
        assert_eq!(Season::from_month(6), Season::Summer);
        assert_eq!(Season::from_month(9), Season::Autumn);
        assert_eq!(Season::from_month(12), Season::Winter);
    }

    #[test]
    fn test_season_labels() {
        assert_eq!(Season::Spring.label_zh(), "春天");
        assert_eq!(Season::Summer.label_zh(), "夏天");
        assert_eq!(Season::Autumn.label_zh(), "秋天");
        assert_eq!(Season::Winter.label_zh(), "冬天");
    }

    #[test]
    fn test_holiday_is_today() {
        let h = Holiday {
            name: "元旦".to_string(),
            month: 1,
            day: 1,
            is_lunar: false,
            greeting: "新年快乐！".to_string(),
        };
        assert!(h.is_today(1, 1));
        assert!(!h.is_today(1, 2));
    }

    #[test]
    fn test_today_holidays() {
        let sa = SeasonalAwareness::for_year(2026);
        // 元旦 / New Year's Day
        let holidays = sa.today_holidays(1, 1);
        assert!(!holidays.is_empty());
        assert!(holidays.iter().any(|h| h.name == "元旦"));
    }

    #[test]
    fn test_upcoming_holidays() {
        let sa = SeasonalAwareness::for_year(2026);
        // 6月25日，儿童节已过，检查7天内是否有节日
        let upcoming = sa.upcoming_holidays(6, 25);
        // 可能没有7天内的节日，取决于日期 / May or may not have holidays within 7 days
        // 主要测试逻辑正确性 / Mainly test logic correctness
        for (_, days) in &upcoming {
            assert!(*days > 0 && *days <= 7);
        }
    }

    #[test]
    fn test_prompt_fragment() {
        let sa = SeasonalAwareness::for_year(2026);
        let fragment = sa.prompt_fragment(6, 26);
        assert!(fragment.contains("[季节]"));
        assert!(fragment.contains("夏天"));
    }

    #[test]
    fn test_prompt_fragment_with_holiday() {
        let sa = SeasonalAwareness::for_year(2026);
        let fragment = sa.prompt_fragment(1, 1);
        assert!(fragment.contains("[节日]"));
        assert!(fragment.contains("元旦"));
    }

    #[test]
    fn test_add_custom_holiday() {
        let mut sa = SeasonalAwareness::for_year(2026);
        sa.add_holiday(Holiday {
            name: "自定义日".to_string(),
            month: 7,
            day: 15,
            is_lunar: false,
            greeting: "自定义快乐！".to_string(),
        });
        let holidays = sa.today_holidays(7, 15);
        assert!(holidays.iter().any(|h| h.name == "自定义日"));
    }

    #[test]
    fn test_epoch_to_month_day() {
        // 2026-06-26 00:00 UTC = 1782432000
        let (month, day) = epoch_to_month_day(1782432000);
        assert_eq!(month, 6);
        assert_eq!(day, 26);
    }

    #[test]
    fn test_season_mood_hints() {
        // 确保每个季节都有非空的 mood hint
        assert!(!Season::Spring.mood_hint().is_empty());
        assert!(!Season::Summer.mood_hint().is_empty());
        assert!(!Season::Autumn.mood_hint().is_empty());
        assert!(!Season::Winter.mood_hint().is_empty());
    }

    #[test]
    fn test_lunar_holidays_exist() {
        let sa = SeasonalAwareness::for_year(2026);
        let lunar_count = sa.holidays.iter().filter(|h| h.is_lunar).count();
        assert!(lunar_count >= 5, "Should have at least 5 lunar holidays");
    }

    // ══════════════════════════════════════════════════════════════
    // C4: 动态农历计算测试 / Dynamic lunar computation tests
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_for_year_2026_spring_festival() {
        // 2026年春节应为 2月17日 / Spring Festival 2026 should be Feb 17
        let sa = SeasonalAwareness::for_year(2026);
        let spring = sa.holidays.iter().find(|h| h.name == "春节").unwrap();
        assert_eq!(spring.month, 2);
        assert_eq!(spring.day, 17);
        assert!(spring.is_lunar);
    }

    #[test]
    fn test_for_year_2025_spring_festival() {
        // 2025年春节应为 1月29日 / Spring Festival 2025 should be Jan 29
        let sa = SeasonalAwareness::for_year(2025);
        let spring = sa.holidays.iter().find(|h| h.name == "春节").unwrap();
        assert_eq!(spring.month, 1);
        assert_eq!(spring.day, 29);
    }

    #[test]
    fn test_for_year_2024_spring_festival() {
        // 2024年春节应为 2月10日 / Spring Festival 2024 should be Feb 10
        let sa = SeasonalAwareness::for_year(2024);
        let spring = sa.holidays.iter().find(|h| h.name == "春节").unwrap();
        assert_eq!(spring.month, 2);
        assert_eq!(spring.day, 10);
    }

    #[test]
    fn test_cached_year() {
        // 验证 cached_year 正确设置 / Verify cached_year is set correctly
        let sa = SeasonalAwareness::for_year(2026);
        assert_eq!(sa.cached_year, 2026);
    }

    #[test]
    fn test_refresh_for_year_same_year() {
        // 同年刷新不应改变节日 / Same year refresh should not change holidays
        let mut sa = SeasonalAwareness::for_year(2026);
        let count_before = sa.holidays.len();
        sa.refresh_for_year(2026);
        assert_eq!(sa.holidays.len(), count_before);
        assert_eq!(sa.cached_year, 2026);
    }

    #[test]
    fn test_refresh_for_year_different_year() {
        // 跨年刷新应更新农历节日 / Cross-year refresh should update lunar holidays
        let mut sa = SeasonalAwareness::for_year(2026);
        let spring_2026 = sa.holidays.iter().find(|h| h.name == "春节").unwrap();
        assert_eq!(spring_2026.month, 2);
        assert_eq!(spring_2026.day, 17);

        // 刷新到 2025 / Refresh to 2025
        sa.refresh_for_year(2025);
        assert_eq!(sa.cached_year, 2025);
        let spring_2025 = sa.holidays.iter().find(|h| h.name == "春节").unwrap();
        assert_eq!(spring_2025.month, 1);
        assert_eq!(spring_2025.day, 29);

        // 固定节日不变 / Fixed holidays unchanged
        assert!(sa.holidays.iter().any(|h| h.name == "元旦" && !h.is_lunar));
    }

    #[test]
    fn test_lunar_holiday_count_per_year() {
        // 每年应有 6 个农历节日 / Each year should have 6 lunar holidays
        for year in [2024, 2025, 2026, 2027] {
            let sa = SeasonalAwareness::for_year(year);
            let lunar_count = sa.holidays.iter().filter(|h| h.is_lunar).count();
            assert_eq!(lunar_count, 6, "Year {} should have 6 lunar holidays", year);
        }
    }

    #[test]
    fn test_total_holiday_count() {
        // 7 固定 + 6 农历 = 13 / 7 fixed + 6 lunar = 13
        let sa = SeasonalAwareness::for_year(2026);
        assert_eq!(sa.holidays.len(), 13);
    }

    // ══════════════════════════════════════════════════════════════
    // C6: 季节情绪 PAD 调制测试 / Seasonal PAD modulation tests
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_season_pad_modulation_values() {
        // 每个季节都应有非零调制 / Each season should have non-zero modulation
        let (p, a, _d) = Season::Spring.pad_modulation();
        assert!(p > 0.0, "Spring pleasure should be positive");
        assert!(a > 0.0, "Spring arousal should be positive");

        let (p, a, _d) = Season::Summer.pad_modulation();
        assert!(p > 0.0, "Summer pleasure should be positive");
        assert!(a > 0.0, "Summer arousal should be positive");

        let (p, a, _) = Season::Autumn.pad_modulation();
        assert!(p < 0.0, "Autumn pleasure should be negative");
        assert!(a < 0.0, "Autumn arousal should be negative");

        let (p, a, d) = Season::Winter.pad_modulation();
        assert!(p < 0.0, "Winter pleasure should be negative");
        assert!(a < 0.0, "Winter arousal should be negative");
        assert!(d < 0.0, "Winter dominance should be negative");
    }

    #[test]
    fn test_season_ordinal() {
        assert_eq!(Season::Spring.ordinal(), 0);
        assert_eq!(Season::Summer.ordinal(), 1);
        assert_eq!(Season::Autumn.ordinal(), 2);
        assert_eq!(Season::Winter.ordinal(), 3);
    }

    #[test]
    fn test_relationship_scale() {
        assert_eq!(relationship_scale(0), 0.5);
        assert_eq!(relationship_scale(1), 0.75);
        assert_eq!(relationship_scale(2), 1.0);
        assert_eq!(relationship_scale(3), 1.2);
        assert_eq!(relationship_scale(99), 1.0); // 未知默认 / Unknown default
    }

    #[test]
    fn test_seasonal_pad_adjustment_spring_deep() {
        let sa = SeasonalAwareness::for_year(2026);
        let modulation = sa.seasonal_pad_adjustment(4, 3); // 春天 + 深度关系
        assert_eq!(modulation.season, Season::Spring);
        assert!((modulation.scale - 1.2).abs() < 0.001);
        // 春天基础 P=0.05, scale=1.2 → delta=0.06
        assert!((modulation.pleasure_delta - 0.06).abs() < 0.001);
        assert!(modulation.pleasure_delta > 0.0);
    }

    #[test]
    fn test_seasonal_pad_adjustment_winter_acquaintance() {
        let sa = SeasonalAwareness::for_year(2026);
        let modulation = sa.seasonal_pad_adjustment(1, 0); // 冬天 + 初识
        assert_eq!(modulation.season, Season::Winter);
        assert!((modulation.scale - 0.5).abs() < 0.001);
        // 冬天基础 P=-0.04, scale=0.5 → delta=-0.02
        assert!((modulation.pleasure_delta - (-0.02)).abs() < 0.001);
        assert!(modulation.pleasure_delta < 0.0);
    }

    #[test]
    fn test_seasonal_pad_adjustment_summer_trusted() {
        let sa = SeasonalAwareness::for_year(2026);
        let modulation = sa.seasonal_pad_adjustment(7, 2); // 夏天 + 信任
        assert_eq!(modulation.season, Season::Summer);
        assert!((modulation.scale - 1.0).abs() < 0.001);
        // 夏天基础 (0.03, 0.05, 0.01), scale=1.0 → unchanged
        let (p_base, a_base, d_base) = Season::Summer.pad_modulation();
        assert!((modulation.pleasure_delta - p_base).abs() < 0.001);
        assert!((modulation.arousal_delta - a_base).abs() < 0.001);
        assert!((modulation.dominance_delta - d_base).abs() < 0.001);
    }

    #[test]
    fn test_seasonal_pad_modulation_zero() {
        let zero = SeasonalPadModulation::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.magnitude(), 0.0);
    }

    #[test]
    fn test_seasonal_pad_modulation_magnitude() {
        let modulation = SeasonalPadModulation {
            pleasure_delta: 0.03,
            arousal_delta: 0.04,
            dominance_delta: 0.0,
            season: Season::Summer,
            scale: 1.0,
        };
        // magnitude = sqrt(0.03^2 + 0.04^2) = sqrt(0.0025) = 0.05
        assert!((modulation.magnitude() - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_seasonal_pad_modulation_description() {
        let zero = SeasonalPadModulation::zero();
        assert_eq!(zero.description_zh(), "无季节调制");

        let modulation = SeasonalPadModulation {
            pleasure_delta: 0.06,
            arousal_delta: 0.036,
            dominance_delta: 0.024,
            season: Season::Spring,
            scale: 1.2,
        };
        let desc = modulation.description_zh();
        assert!(desc.contains("春天"));
        assert!(desc.contains("季节调制"));
    }

    #[test]
    fn test_apply_seasonal_pad() {
        let sa = SeasonalAwareness::for_year(2026);
        // 基础情感：平静 (0.1, -0.5, -0.1) + 春天深度关系调制
        let (p, a, d) = sa.apply_seasonal_pad(0.1, -0.5, -0.1, 4, 3);
        // 春天深度：P delta = 0.05*1.2 = 0.06
        assert!(p > 0.1, "Pleasure should increase in spring");
        // A delta = 0.03*1.2 = 0.036
        assert!(a > -0.5, "Arousal should increase in spring");
        // 所有值应在 [-1, 1] 范围内 / All values should be in [-1, 1]
        assert!((-1.0..=1.0).contains(&p));
        assert!((-1.0..=1.0).contains(&a));
        assert!((-1.0..=1.0).contains(&d));
    }

    #[test]
    fn test_apply_seasonal_pad_clamping() {
        let sa = SeasonalAwareness::for_year(2026);
        // 极端值应被 clamp / Extreme values should be clamped
        let (p, a, d) = sa.apply_seasonal_pad(0.99, 0.99, 0.99, 6, 3);
        assert!(p <= 1.0);
        assert!(a <= 1.0);
        assert!(d <= 1.0);

        let (p, a, d) = sa.apply_seasonal_pad(-0.99, -0.99, -0.99, 1, 3);
        assert!(p >= -1.0);
        assert!(a >= -1.0);
        assert!(d >= -1.0);
    }

    #[test]
    fn test_deeper_relationship_stronger_modulation() {
        let sa = SeasonalAwareness::for_year(2026);
        // 同一季节，关系越深调制越强 / Same season, deeper relationship = stronger modulation
        let mod0 = sa.seasonal_pad_adjustment(4, 0); // 初识
        let mod1 = sa.seasonal_pad_adjustment(4, 1); // 熟悉
        let mod2 = sa.seasonal_pad_adjustment(4, 2); // 信任
        let mod3 = sa.seasonal_pad_adjustment(4, 3); // 深度

        // 春天 P delta 全部为正，深度越大 delta 越大
        assert!(mod0.pleasure_delta < mod1.pleasure_delta);
        assert!(mod1.pleasure_delta < mod2.pleasure_delta);
        assert!(mod2.pleasure_delta < mod3.pleasure_delta);
    }
}
