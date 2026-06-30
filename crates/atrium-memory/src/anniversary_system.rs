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

    /// 判断今天是否是纪念日 / Check if today is the anniversary
    pub fn is_today(&self, now_epoch_secs: i64) -> bool {
        // 比较月和日 / Compare month and day
        let (_, month_orig, day_orig) = epoch_to_ymd(self.date_epoch);
        let (_, month_now, day_now) = epoch_to_ymd(now_epoch_secs);
        month_orig == month_now && day_orig == day_now
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
}

impl AnniversarySystem {
    pub fn new() -> Self {
        Self {
            anniversaries: Vec::new(),
            next_id: 1,
            first_conversation_set: false,
        }
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
        });
        self.next_id += 1;
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

    /// 生成纪念日 prompt 片段 / Generate anniversary prompt fragment
    pub fn prompt_fragment(&self, now_epoch_secs: i64) -> String {
        let upcoming: Vec<_> = self
            .anniversaries
            .iter()
            .filter(|a| {
                let days = a.days_from(now_epoch_secs);
                // 即将到来（7天内）或刚过（1天内）/ Upcoming (within 7 days) or just passed (within 1 day)
                let days_to_next = 365 - (days % 365);
                days_to_next <= 7 || days % 365 == 0
            })
            .collect();

        if upcoming.is_empty() {
            return String::new();
        }

        let parts: Vec<String> = upcoming
            .iter()
            .map(|a| {
                let years = a.years_together(now_epoch_secs);
                if years > 0 {
                    format!("{}{}周年", a.kind.label_zh(), years)
                } else {
                    a.kind.label_zh().to_string()
                }
            })
            .collect();

        format!("[纪念日] {}", parts.join("、"))
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
}
