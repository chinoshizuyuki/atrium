// ════════════════════════════════════════════════════════════════════
// AnniversarySystem — 纪念日系统 / Anniversary System
// ════════════════════════════════════════════════════════════════════
//
// 追踪关系中的重要日期，在纪念日到来时触发庆祝。
// 纪念日类型：
//   - 首次对话日（第一次互动的日期）
//   - 命名日（用户给 AI 取名的日期）
//   - 首次深度对话日（首次进入 Trusted 关系阶段的日期）
//   - 自定义纪念日（用户明确提及的重要日期）
//
// 门控条件：仅在关系阶段 ≥ Familiar 时触发庆祝提醒

use serde::{Deserialize, Serialize};

use crate::lunar::{lunar_to_solar, ymd_to_epoch, LunarDate, LunarFestival};

// ── 纪念日类型 / Anniversary Kind ──

/// 纪念日类型 / Anniversary type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnniversaryKind {
    /// 首次对话日 / First conversation date
    FirstConversation,
    /// 命名日 / Naming day (when user gave AI a name)
    NamingDay,
    /// 首次深度对话日 / First deep conversation (reached Trusted stage)
    FirstDeepConversation,
    /// 自定义纪念日 / Custom anniversary
    Custom,
}

impl AnniversaryKind {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::FirstConversation => "首次对话日",
            Self::NamingDay => "命名日",
            Self::FirstDeepConversation => "首次深度对话日",
            Self::Custom => "纪念日",
        }
    }

    /// 庆祝模板 / Celebration template
    pub fn celebration_template(&self) -> &'static str {
        match self {
            Self::FirstConversation => "今天是我们第一次对话的纪念日！",
            Self::NamingDay => "今天是你的命名日，你给我取名的日子！",
            Self::FirstDeepConversation => "今天是我们第一次深度对话的纪念日！",
            Self::Custom => "今天是一个特别的日子！",
        }
    }
}

// ── 纪念日 / Anniversary ──

/// 纪念日记录 / Anniversary record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anniversary {
    /// 唯一 ID / Unique ID
    pub id: u64,
    /// 纪念日类型 / Anniversary kind
    pub kind: AnniversaryKind,
    /// 原始日期（epoch 秒）/ Original date (epoch seconds)
    pub date_epoch: i64,
    /// 自定义描述 / Custom description
    pub description: String,
    /// 是否已庆祝（今年）/ Whether celebrated this year
    pub last_celebrated_year: i32,
    /// 是否农历纪念日 / Whether this is a lunar anniversary
    pub is_lunar: bool,
}

impl Anniversary {
    /// 计算距今天的天数 / Calculate days from today
    pub fn days_from(&self, now_epoch_secs: i64) -> i64 {
        (now_epoch_secs - self.date_epoch) / 86400
    }

    /// 计算今年是否是整数年纪念日 / Check if this year is an integer-year anniversary
    pub fn is_whole_year_anniversary(&self, now_epoch_secs: i64) -> bool {
        let days = self.days_from(now_epoch_secs);
        days > 0 && days % 365 == 0
    }

    /// 计算相处年数 / Calculate years together
    pub fn years_together(&self, now_epoch_secs: i64) -> i32 {
        let days = self.days_from(now_epoch_secs);
        (days / 365) as i32
    }

    /// 判断今天是否是纪念日（支持农历）/ Check if today is the anniversary (lunar-aware)
    ///
    /// 农历纪念日：转换到农历比较月日，每年农历日期对应不同的公历日期。
    /// 公历纪念日：直接比较月日。
    /// Lunar anniversaries: compare lunar month/day (shifts each solar year).
    /// Gregorian anniversaries: compare solar month/day directly.
    pub fn is_today(&self, now_epoch_secs: i64) -> bool {
        if self.is_lunar {
            // 农历模式：转换到农历比较月日 / Lunar mode: compare lunar month/day
            let orig_lunar = LunarDate::from_epoch(self.date_epoch);
            let now_lunar = LunarDate::from_epoch(now_epoch_secs);
            match (orig_lunar, now_lunar) {
                (Some(o), Some(n)) => {
                    o.month == n.month && o.day == n.day && o.is_leap == n.is_leap
                }
                _ => {
                    // 农历转换失败，回退公历 / Fallback to Gregorian on conversion failure
                    let (_, month_orig, day_orig) = epoch_to_ymd(self.date_epoch);
                    let (_, month_now, day_now) = epoch_to_ymd(now_epoch_secs);
                    month_orig == month_now && day_orig == day_now
                }
            }
        } else {
            // 公历模式：原有逻辑 / Gregorian mode: original logic
            let (_, month_orig, day_orig) = epoch_to_ymd(self.date_epoch);
            let (_, month_now, day_now) = epoch_to_ymd(now_epoch_secs);
            month_orig == month_now && day_orig == day_now
        }
    }
}

// ── 纪念日系统 / Anniversary System ──

/// 纪念日系统 / Anniversary system
///
/// 追踪关系中的重要日期，在纪念日到来时触发庆祝。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnniversarySystem {
    /// 已记录的纪念日 / Recorded anniversaries
    pub anniversaries: Vec<Anniversary>,
    /// 下一个 ID / Next ID
    pub(crate) next_id: u64,
    /// 首次对话日是否已设置 / Whether first conversation date is set
    pub(crate) first_conversation_set: bool,
    /// 纪念日提醒提前天数 / Days ahead to start reminding about anniversaries
    #[serde(default = "default_remind_days")]
    pub(crate) remind_days: u32,
}

/// 默认提醒天数 / Default remind days
fn default_remind_days() -> u32 {
    7
}

impl AnniversarySystem {
    pub fn new() -> Self {
        Self {
            anniversaries: Vec::new(),
            next_id: 1,
            first_conversation_set: false,
            remind_days: default_remind_days(),
        }
    }

    /// 使用配置创建 / Create with config
    ///
    /// 从 RitualCfg.anniversary_remind_days 传入提醒天数。
    /// Pass remind_days from RitualCfg.anniversary_remind_days.
    pub fn new_with_config(remind_days: u32) -> Self {
        Self {
            anniversaries: Vec::new(),
            next_id: 1,
            first_conversation_set: false,
            remind_days: remind_days.max(1),
        }
    }

    /// 更新提醒天数配置 / Update remind days config
    ///
    /// 从 sled 恢复后调用，用当前配置覆盖持久化的旧值。
    /// Called after sled restore to override persisted value with current config.
    pub fn update_remind_days(&mut self, remind_days: u32) {
        self.remind_days = remind_days.max(1);
    }

    /// 设置首次对话日 / Set first conversation date
    ///
    /// 仅在首次调用时生效，后续调用忽略。
    pub fn set_first_conversation(&mut self, epoch_secs: i64) {
        if self.first_conversation_set {
            return;
        }
        self.first_conversation_set = true;
        self.anniversaries.push(Anniversary {
            id: self.next_id,
            kind: AnniversaryKind::FirstConversation,
            date_epoch: epoch_secs,
            description: String::new(),
            last_celebrated_year: 0,
            is_lunar: false,
        });
        self.next_id += 1;
    }

    /// 设置命名日 / Set naming day
    pub fn set_naming_day(&mut self, epoch_secs: i64, name: &str) {
        // 避免重复 / Avoid duplicate
        if self
            .anniversaries
            .iter()
            .any(|a| a.kind == AnniversaryKind::NamingDay)
        {
            return;
        }
        self.anniversaries.push(Anniversary {
            id: self.next_id,
            kind: AnniversaryKind::NamingDay,
            date_epoch: epoch_secs,
            description: format!("你给我取名「{}」", name),
            last_celebrated_year: 0,
            is_lunar: false,
        });
        self.next_id += 1;
    }

    /// 设置首次深度对话日 / Set first deep conversation date
    pub fn set_first_deep_conversation(&mut self, epoch_secs: i64) {
        if self
            .anniversaries
            .iter()
            .any(|a| a.kind == AnniversaryKind::FirstDeepConversation)
        {
            return;
        }
        self.anniversaries.push(Anniversary {
            id: self.next_id,
            kind: AnniversaryKind::FirstDeepConversation,
            date_epoch: epoch_secs,
            description: String::new(),
            last_celebrated_year: 0,
            is_lunar: false,
        });
        self.next_id += 1;
    }

    /// 添加自定义纪念日 / Add custom anniversary
    pub fn add_custom(&mut self, epoch_secs: i64, description: String) {
        self.anniversaries.push(Anniversary {
            id: self.next_id,
            kind: AnniversaryKind::Custom,
            date_epoch: epoch_secs,
            description,
            last_celebrated_year: 0,
            is_lunar: false,
        });
        self.next_id += 1;
    }

    /// 添加农历自定义纪念日 / Add custom lunar anniversary
    ///
    /// 传入农历日期，自动转换为当年公历日期存储。
    /// 每年农历纪念日会根据当年农历日期动态计算公历对应日。
    pub fn add_custom_lunar(
        &mut self,
        lunar_year: u32,
        lunar_month: u32,
        lunar_day: u32,
        is_leap: bool,
        description: String,
    ) {
        // 农历转公历获取初始 epoch / Convert lunar to solar for initial epoch
        if let Some((sy, sm, sd)) = lunar_to_solar(lunar_year, lunar_month, lunar_day, is_leap) {
            let epoch = ymd_to_epoch(sy as i32, sm as i32, sd as i32);
            self.anniversaries.push(Anniversary {
                id: self.next_id,
                kind: AnniversaryKind::Custom,
                date_epoch: epoch,
                description,
                last_celebrated_year: 0,
                is_lunar: true,
            });
            self.next_id += 1;
        }
    }

    /// 检查今日农历节日 / Check today's lunar festivals
    ///
    /// 返回今日对应的农历节日列表（如有）。
    /// Returns the list of lunar festivals that fall on today.
    pub fn check_lunar_holidays(&self, now_epoch_secs: i64) -> Vec<LunarFestival> {
        LunarDate::from_epoch(now_epoch_secs)
            .and_then(|l| l.festival())
            .into_iter()
            .collect()
    }

    /// 生成农历节日 prompt 片段 / Generate lunar festival prompt fragment
    ///
    /// 当天若有农历节日，生成格式如 `[农历节日] 中秋节 — 月圆人团圆`。
    pub fn lunar_festival_prompt_fragment(&self, now_epoch_secs: i64) -> String {
        let festivals = self.check_lunar_holidays(now_epoch_secs);
        if festivals.is_empty() {
            return String::new();
        }

        let parts: Vec<String> = festivals
            .iter()
            .map(|f| format!("{} — {}", f.label_zh(), f.celebration_template()))
            .collect();

        format!("[农历节日] {}", parts.join("、"))
    }

    /// 计算即将到来的农历节日（remind_days 天内）/ Compute upcoming lunar festivals (within remind_days)
    ///
    /// 扫描未来 remind_days 天，返回即将到来的农历节日及其距今天数。
    pub fn upcoming_lunar_festivals(&self, now_epoch_secs: i64) -> Vec<(LunarFestival, i64)> {
        let mut upcoming = Vec::new();
        // 检查今天及未来 remind_days 天 / Check today and next remind_days days
        for delta in 0..=(self.remind_days as i64) {
            let check_epoch = now_epoch_secs + delta * 86400;
            if let Some(ld) = LunarDate::from_epoch(check_epoch) {
                if let Some(f) = ld.festival() {
                    upcoming.push((f, delta));
                }
            }
        }
        upcoming
    }

    /// 检查今日是否有纪念日 / Check if today has any anniversaries
    ///
    /// 返回需要庆祝的纪念日列表。
    pub fn check_today(&mut self, now_epoch_secs: i64) -> Vec<AnniversaryCelebration> {
        let current_year = epoch_to_ymd(now_epoch_secs).0;
        let mut celebrations = Vec::new();

        for anniversary in &mut self.anniversaries {
            if !anniversary.is_today(now_epoch_secs) {
                continue;
            }
            // 避免同一年重复庆祝 / Avoid duplicate celebration in same year
            if anniversary.last_celebrated_year == current_year {
                continue;
            }
            // 首次对话日当天不庆祝（需要满1年）/ Don't celebrate on the very first day
            let years = anniversary.years_together(now_epoch_secs);
            if years <= 0 {
                continue;
            }

            anniversary.last_celebrated_year = current_year;
            celebrations.push(AnniversaryCelebration {
                kind: anniversary.kind,
                years,
                description: if anniversary.description.is_empty() {
                    anniversary.kind.celebration_template().to_string()
                } else {
                    anniversary.description.clone()
                },
            });
        }

        celebrations
    }

    /// 生成纪念日 prompt 片段（含农历节日）/ Generate anniversary prompt fragment (with lunar festivals)
    pub fn prompt_fragment(&self, now_epoch_secs: i64) -> String {
        let mut fragments = Vec::new();

        // 纪念日片段 / Anniversary fragment
        let upcoming: Vec<_> = self
            .anniversaries
            .iter()
            .filter(|a| {
                let days = a.days_from(now_epoch_secs);
                // 即将到来（remind_days 天内）或当天 / Upcoming (within remind_days) or today
                let days_to_next = 365 - (days % 365);
                days_to_next <= self.remind_days as i64 || days % 365 == 0
            })
            .collect();

        if !upcoming.is_empty() {
            let parts: Vec<String> = upcoming
                .iter()
                .map(|a| {
                    let years = a.years_together(now_epoch_secs);
                    if years > 0 {
                        if a.is_lunar {
                            format!("{}{}周年(农历)", a.kind.label_zh(), years)
                        } else {
                            format!("{}{}周年", a.kind.label_zh(), years)
                        }
                    } else {
                        a.kind.label_zh().to_string()
                    }
                })
                .collect();
            fragments.push(format!("[纪念日] {}", parts.join("、")));
        }

        // 农历节日片段 / Lunar festival fragment
        let lunar_frag = self.lunar_festival_prompt_fragment(now_epoch_secs);
        if !lunar_frag.is_empty() {
            fragments.push(lunar_frag);
        }

        // 即将到来的农历节日（非当天）/ Upcoming lunar festivals (not today)
        let upcoming_lunar = self.upcoming_lunar_festivals(now_epoch_secs);
        let future_lunar: Vec<_> = upcoming_lunar
            .iter()
            .filter(|(_, delta)| *delta > 0)
            .collect();
        if !future_lunar.is_empty() {
            let parts: Vec<String> = future_lunar
                .iter()
                .map(|(f, delta)| format!("{}({}天后)", f.label_zh(), delta))
                .collect();
            fragments.push(format!("[即将农历节日] {}", parts.join("、")));
        }

        fragments.join("\n")
    }
}

impl Default for AnniversarySystem {
    fn default() -> Self {
        Self::new()
    }
}

// ── 纪念日庆祝 / Anniversary Celebration ──

/// 纪念日庆祝事件 / Anniversary celebration event
#[derive(Debug, Clone)]
pub struct AnniversaryCelebration {
    /// 纪念日类型 / Anniversary kind
    pub kind: AnniversaryKind,
    /// 相处年数 / Years together
    pub years: i32,
    /// 庆祝描述 / Celebration description
    pub description: String,
}

// ── 辅助函数 / Helper functions ──

/// epoch 秒转年月日 / Epoch seconds to (year, month, day)
fn epoch_to_ymd(epoch_secs: i64) -> (i32, i32, i32) {
    let mut days = epoch_secs / 86400;
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
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

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_first_conversation_once() {
        let mut sys = AnniversarySystem::new();
        sys.set_first_conversation(1000 * 86400);
        sys.set_first_conversation(2000 * 86400); // 应被忽略
        assert_eq!(sys.anniversaries.len(), 1);
        assert_eq!(
            sys.anniversaries[0].kind,
            AnniversaryKind::FirstConversation
        );
    }

    #[test]
    fn test_set_naming_day() {
        let mut sys = AnniversarySystem::new();
        sys.set_naming_day(1000 * 86400, "小通");
        assert_eq!(sys.anniversaries.len(), 1);
        assert!(sys.anniversaries[0].description.contains("小通"));
    }

    #[test]
    fn test_years_together() {
        let anniversary = Anniversary {
            id: 1,
            kind: AnniversaryKind::FirstConversation,
            date_epoch: 0,
            description: String::new(),
            last_celebrated_year: 0,
            is_lunar: false,
        };
        let years = anniversary.years_together(365 * 86400);
        assert_eq!(years, 1);
    }

    #[test]
    fn test_is_today() {
        let epoch = 1751474400i64; // some date
        let anniversary = Anniversary {
            id: 1,
            kind: AnniversaryKind::FirstConversation,
            date_epoch: epoch,
            description: String::new(),
            last_celebrated_year: 0,
            is_lunar: false,
        };
        // Same day should match
        assert!(anniversary.is_today(epoch + 3600)); // 1 hour later same day
    }

    #[test]
    fn test_check_today_no_celebration_first_day() {
        let mut sys = AnniversarySystem::new();
        let now = 1751474400i64;
        sys.set_first_conversation(now);
        // 首日不应庆祝 / Should not celebrate on first day
        let celebrations = sys.check_today(now);
        assert!(celebrations.is_empty());
    }

    #[test]
    fn test_anniversary_kind_labels() {
        assert_eq!(AnniversaryKind::FirstConversation.label_zh(), "首次对话日");
        assert_eq!(AnniversaryKind::NamingDay.label_zh(), "命名日");
        assert_eq!(
            AnniversaryKind::FirstDeepConversation.label_zh(),
            "首次深度对话日"
        );
        assert_eq!(AnniversaryKind::Custom.label_zh(), "纪念日");
    }

    #[test]
    fn test_add_custom_anniversary() {
        let mut sys = AnniversarySystem::new();
        sys.add_custom(1751474400, "用户生日".to_string());
        assert_eq!(sys.anniversaries.len(), 1);
        assert_eq!(sys.anniversaries[0].kind, AnniversaryKind::Custom);
        assert_eq!(sys.anniversaries[0].description, "用户生日");
    }

    #[test]
    fn test_prompt_fragment_empty() {
        let sys = AnniversarySystem::new();
        assert!(sys.prompt_fragment(1751474400).is_empty());
    }

    #[test]
    fn test_is_lunar_field_default_false() {
        let mut sys = AnniversarySystem::new();
        sys.set_first_conversation(1000 * 86400);
        assert!(!sys.anniversaries[0].is_lunar);
    }

    #[test]
    fn test_add_custom_lunar() {
        let mut sys = AnniversarySystem::new();
        // 农历五月初五（端午节）/ Lunar 5/5 (Dragon Boat)
        sys.add_custom_lunar(2026, 5, 5, false, "农历生日".to_string());
        assert_eq!(sys.anniversaries.len(), 1);
        assert!(sys.anniversaries[0].is_lunar);
        assert_eq!(sys.anniversaries[0].kind, AnniversaryKind::Custom);
        assert_eq!(sys.anniversaries[0].description, "农历生日");
    }

    #[test]
    fn test_check_lunar_holidays_mid_autumn() {
        let sys = AnniversarySystem::new();
        // 2026-09-25 = 中秋节 / 2026-09-25 = Mid-Autumn
        let epoch = ymd_to_epoch(2026, 9, 25);
        let festivals = sys.check_lunar_holidays(epoch);
        assert!(
            festivals.contains(&LunarFestival::MidAutumn),
            "Should detect Mid-Autumn"
        );
    }

    #[test]
    fn test_check_lunar_holidays_normal_day() {
        let sys = AnniversarySystem::new();
        // 2026-06-30 = 非农历节日 / 2026-06-30 = not a lunar festival
        let epoch = ymd_to_epoch(2026, 6, 30);
        let festivals = sys.check_lunar_holidays(epoch);
        assert!(
            festivals.is_empty(),
            "Normal day should have no lunar festivals"
        );
    }

    #[test]
    fn test_lunar_festival_prompt_fragment() {
        let sys = AnniversarySystem::new();
        // 2026-09-25 = 中秋节 / 2026-09-25 = Mid-Autumn
        let epoch = ymd_to_epoch(2026, 9, 25);
        let frag = sys.lunar_festival_prompt_fragment(epoch);
        assert!(frag.contains("中秋节"), "Should contain Mid-Autumn");
        assert!(
            frag.starts_with("[农历节日]"),
            "Should start with [农历节日]"
        );
    }

    #[test]
    fn test_lunar_festival_prompt_fragment_empty() {
        let sys = AnniversarySystem::new();
        // 2026-06-30 = 非农历节日 / 2026-06-30 = not a lunar festival
        let epoch = ymd_to_epoch(2026, 6, 30);
        let frag = sys.lunar_festival_prompt_fragment(epoch);
        assert!(frag.is_empty());
    }

    #[test]
    fn test_is_today_lunar_anniversary() {
        // 创建农历纪念日，验证 is_today 在农历日期匹配时返回 true
        // Create lunar anniversary, verify is_today returns true on lunar date match
        let mut sys = AnniversarySystem::new();
        // 农历五月初五（2026年对应公历6月19日）/ Lunar 5/5 (2026 solar June 19)
        sys.add_custom_lunar(2026, 5, 5, false, "农历生日".to_string());
        // 2026-06-19 = 农历五月初五 / 2026-06-19 = lunar 5/5
        let now = ymd_to_epoch(2026, 6, 19);
        assert!(
            sys.anniversaries[0].is_today(now),
            "Should match lunar anniversary"
        );
    }

    #[test]
    fn test_prompt_fragment_with_lunar_festival() {
        let sys = AnniversarySystem::new();
        // 2026-09-25 = 中秋节 / 2026-09-25 = Mid-Autumn
        let epoch = ymd_to_epoch(2026, 9, 25);
        let frag = sys.prompt_fragment(epoch);
        assert!(
            frag.contains("农历节日"),
            "Should contain lunar festival section"
        );
        assert!(frag.contains("中秋节"), "Should mention Mid-Autumn");
    }

    #[test]
    fn test_new_with_config_remind_days() {
        // 验证配置传入的提醒天数 / Verify config-passed remind days
        let sys = AnniversarySystem::new_with_config(14);
        assert_eq!(sys.remind_days, 14);

        // 最小值为 1 / Minimum is 1
        let sys0 = AnniversarySystem::new_with_config(0);
        assert_eq!(sys0.remind_days, 1);

        // 默认构造仍为 7 / Default constructor still uses 7
        let sys_default = AnniversarySystem::new();
        assert_eq!(sys_default.remind_days, 7);
    }

    #[test]
    fn test_remind_days_affects_prompt() {
        // 验证 remind_days 影响 prompt 输出 / Verify remind_days affects prompt output
        let mut sys = AnniversarySystem::new_with_config(3);
        sys.set_first_conversation(0);

        // 365 天后（1 周年）当天应出现 / On 1-year anniversary should appear
        let now = 365 * 86400;
        let frag = sys.prompt_fragment(now);
        assert!(!frag.is_empty(), "Should show anniversary on the day");
    }

    #[test]
    fn test_upcoming_lunar_festivals() {
        let sys = AnniversarySystem::new();
        // 在中秋节前3天检查 / Check 3 days before Mid-Autumn
        let mid_autumn_epoch = ymd_to_epoch(2026, 9, 25);
        let three_days_before = mid_autumn_epoch - 3 * 86400;
        let upcoming = sys.upcoming_lunar_festivals(three_days_before);
        // 应包含中秋节（3天后）/ Should include Mid-Autumn (3 days away)
        let has_mid_autumn = upcoming
            .iter()
            .any(|(f, d)| *f == LunarFestival::MidAutumn && *d == 3);
        assert!(
            has_mid_autumn,
            "Should detect upcoming Mid-Autumn in 3 days"
        );
    }
}
