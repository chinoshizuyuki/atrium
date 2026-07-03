// ════════════════════════════════════════════════════════════════════
// RitualDetector — 共享仪式发现与维护 / Shared Ritual Discovery & Maintenance
// ════════════════════════════════════════════════════════════════════
//
// 自动发现用户与 AI 之间的重复互动模式，将其升级为"仪式"。
// 仪式是关系的粘合剂——每晚的晚安、周末的闲聊、工作日的早安。
//
// 核心算法：
//   1. 记录每日交互时间分布
//   2. 识别高频时段（同一小时槽内连续多天有交互）
//   3. 连续 N 天 → 升级为仪式（默认 N=7）
//   4. 仪式中断 ≥ M 天 → 标记为中断，触发温和提醒（默认 M=3）
//
// C5 扩展：内容语义仪式检测
//   - 从对话内容中提取仪式性语义模式（晚安、早安等）
//   - 连续多天出现相同内容签名 → 升级为内容仪式
//   - 内容仪式与时间仪式共享生命周期（Candidate→Active→Broken→Archived）
//
// 门控条件：
//   - 仪式发现：对所有关系阶段开放 / Discovery: open to all relationship stages
//   - 仪式提醒：仅在 ≥Familiar 时触发 / Reminders: only trigger at ≥Familiar
//   - should_remind(relation_ordinal) 提供门控判断 / provides gate decision

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── 时间槽 / Time Slot ──

/// 小时级时间槽 / Hourly time slot
///
/// 将一天划分为 24 个时间槽，每个槽覆盖 1 小时。
/// 用于统计交互的时间分布，识别高频时段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeSlot {
    /// 起始小时（0-23）/ Start hour (0-23)
    pub hour: u8,
}

impl TimeSlot {
    pub fn new(hour: u8) -> Self {
        Self { hour: hour.min(23) }
    }

    /// 从 epoch 秒提取时间槽 / Extract time slot from epoch seconds
    pub fn from_epoch(epoch_secs: i64) -> Self {
        // 简单 UTC 小时提取 / Simple UTC hour extraction
        let hour = ((epoch_secs / 3600) % 24) as u8;
        Self::new(hour)
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self.hour {
            0 => "深夜0点",
            1 => "凌晨1点",
            2 => "凌晨2点",
            3 => "凌晨3点",
            4 => "凌晨4点",
            5 => "清晨5点",
            6 => "清晨6点",
            7 => "早晨7点",
            8 => "上午8点",
            9 => "上午9点",
            10 => "上午10点",
            11 => "上午11点",
            12 => "中午12点",
            13 => "下午1点",
            14 => "下午2点",
            15 => "下午3点",
            16 => "下午4点",
            17 => "下午5点",
            18 => "傍晚6点",
            19 => "晚上7点",
            20 => "晚上8点",
            21 => "晚上9点",
            22 => "晚间10点",
            23 => "深夜11点",
            _ => "未知时段",
        }
    }
}

// ── 内容仪式签名 / Content Ritual Signature ──

/// 内容仪式签名 / Content ritual signature
///
/// 从对话内容中提取的仪式性语义模式。
/// Semantic ritual patterns extracted from conversation content.
///
/// 支持的签名：
///   - Goodnight: 晚安、睡了等 / Goodnight patterns
///   - GoodMorning: 早安、早上好等 / Good morning patterns
///   - WeekendGreeting: 周末愉快等 / Weekend greeting patterns
///   - HolidayGreeting: 节日快乐等 / Holiday greeting patterns
///   - LunarHolidayGreeting: 农历节日问候（中秋、端午等）/ Lunar holiday greeting patterns
///   - Custom: 用户自定义 / User-defined custom patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentHint {
    /// 晚安类 / Goodnight pattern
    Goodnight,
    /// 早安类 / Good morning pattern
    GoodMorning,
    /// 周末问候 / Weekend greeting
    WeekendGreeting,
    /// 节日问候 / Holiday greeting
    HolidayGreeting,
    /// 农历节日问候 / Lunar holiday greeting (Mid-Autumn, Dragon Boat, etc.)
    LunarHolidayGreeting,
    /// 自定义 / Custom pattern
    Custom(String),
}

impl ContentHint {
    /// 从文本中检测内容仪式签名 / Detect content ritual signature from text
    ///
    /// 使用关键词匹配识别常见的仪式性内容模式。
    /// Uses keyword matching to identify common ritual content patterns.
    pub fn detect(text: &str) -> Option<Self> {
        let lower = text.to_lowercase();
        // 晚安模式 / Goodnight patterns
        if lower.contains("晚安")
            || lower.contains("睡了")
            || lower.contains("我要睡了")
            || lower.contains("good night")
            || lower.contains("goodnight")
        {
            return Some(Self::Goodnight);
        }
        // 早安模式 / Good morning patterns
        if lower.contains("早安")
            || lower.contains("早上好")
            || lower.contains("早啊")
            || lower.contains("good morning")
        {
            return Some(Self::GoodMorning);
        }
        // 周末问候 / Weekend greeting
        if lower.contains("周末愉快") || lower.contains("周末快乐") || lower.contains("周末好")
        {
            return Some(Self::WeekendGreeting);
        }
        // 节日问候 / Holiday greeting
        if lower.contains("节日快乐") || lower.contains("新年快乐") || lower.contains("圣诞快乐")
        {
            return Some(Self::HolidayGreeting);
        }
        // 农历节日问候 / Lunar holiday greeting
        if lower.contains("中秋快乐")
            || lower.contains("中秋安康")
            || lower.contains("端午安康")
            || lower.contains("端午快乐")
            || lower.contains("春节快乐")
            || lower.contains("新年好")
            || lower.contains("元宵快乐")
            || lower.contains("重阳安康")
            || lower.contains("七夕快乐")
            || lower.contains("腊八快乐")
            || lower.contains("除夕快乐")
        {
            return Some(Self::LunarHolidayGreeting);
        }
        None
    }

    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &str {
        match self {
            Self::Goodnight => "晚安",
            Self::GoodMorning => "早安",
            Self::WeekendGreeting => "周末问候",
            Self::HolidayGreeting => "节日问候",
            Self::LunarHolidayGreeting => "农历节日问候",
            Self::Custom(s) => s,
        }
    }

    /// HashMap 键 / HashMap key (stable across serialization)
    pub fn key(&self) -> String {
        match self {
            Self::Goodnight => "goodnight".to_string(),
            Self::GoodMorning => "good_morning".to_string(),
            Self::WeekendGreeting => "weekend_greeting".to_string(),
            Self::HolidayGreeting => "holiday_greeting".to_string(),
            Self::LunarHolidayGreeting => "lunar_holiday_greeting".to_string(),
            Self::Custom(s) => format!("custom:{}", s),
        }
    }

    /// 从键反解 / Reconstruct from key
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "goodnight" => Some(Self::Goodnight),
            "good_morning" => Some(Self::GoodMorning),
            "weekend_greeting" => Some(Self::WeekendGreeting),
            "holiday_greeting" => Some(Self::HolidayGreeting),
            "lunar_holiday_greeting" => Some(Self::LunarHolidayGreeting),
            s if s.starts_with("custom:") => Some(Self::Custom(s[7..].to_string())),
            _ => None,
        }
    }

    /// 序数（跨 crate 传递用）/ Ordinal for cross-crate use
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Goodnight => 0,
            Self::GoodMorning => 1,
            Self::WeekendGreeting => 2,
            Self::HolidayGreeting => 3,
            Self::LunarHolidayGreeting => 4,
            Self::Custom(_) => 255,
        }
    }
}

// ── 仪式状态 / Ritual Status ──

/// 仪式生命周期状态 / Ritual lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RitualStatus {
    /// 候选：连续出现但未达升级阈值 / Candidate: recurring but below promotion threshold
    Candidate,
    /// 活跃：已升级为仪式 / Active: promoted to ritual
    Active,
    /// 中断：仪式被打断 / Broken: ritual interrupted
    Broken,
    /// 归档：长期中断后归档 / Archived: archived after long interruption
    Archived,
}

// ── 仪式模式 / Ritual Pattern ──

/// 识别到的重复互动模式 / Identified recurring interaction pattern
///
/// 当同一时间槽连续多天出现交互时，形成仪式模式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualPattern {
    /// 唯一 ID / Unique ID
    pub id: u64,
    /// 时间槽 / Time slot
    pub time_slot: TimeSlot,
    /// 连续天数 / Consecutive days
    pub consecutive_days: u32,
    /// 首次发现时间 / First discovery time
    pub first_seen_at: i64,
    /// 最后出现时间 / Last occurrence time
    pub last_occurrence_at: i64,
    /// 当前状态 / Current status
    pub status: RitualStatus,
    /// 中断天数 / Days since last occurrence (for broken rituals)
    pub break_days: u32,
    /// 交互次数（在该时间槽内的总交互数）/ Total interactions in this time slot
    pub total_interactions: u64,
}

impl RitualPattern {
    /// 生成仪式名称 / Generate ritual name
    pub fn name(&self) -> String {
        format!("{}时刻仪式", self.time_slot.label_zh())
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self) -> String {
        match self.status {
            RitualStatus::Candidate => {
                format!(
                    "在{}连续{}天互动，即将成为仪式",
                    self.time_slot.label_zh(),
                    self.consecutive_days
                )
            }
            RitualStatus::Active => {
                format!(
                    "每天{}的固定互动，已持续{}天",
                    self.time_slot.label_zh(),
                    self.consecutive_days
                )
            }
            RitualStatus::Broken => {
                format!(
                    "每天{}的互动已中断{}天",
                    self.time_slot.label_zh(),
                    self.break_days
                )
            }
            RitualStatus::Archived => {
                format!("每天{}的互动已归档", self.time_slot.label_zh())
            }
        }
    }
}

// ── 内容仪式模式 / Content Ritual Pattern ──

/// 内容仪式模式 / Content ritual pattern
///
/// 基于对话内容语义识别的重复互动模式。
/// 例如：连续7天说"晚安" → 升级为"晚安仪式"。
/// Recurring interaction pattern identified by conversation content semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRitualPattern {
    /// 唯一 ID / Unique ID
    pub id: u64,
    /// 内容签名 / Content signature
    pub hint: ContentHint,
    /// 关联时间槽 / Associated time slot (optional)
    pub time_slot: Option<TimeSlot>,
    /// 连续天数 / Consecutive days
    pub consecutive_days: u32,
    /// 首次发现时间 / First discovery time
    pub first_seen_at: i64,
    /// 最后出现时间 / Last occurrence time
    pub last_occurrence_at: i64,
    /// 当前状态 / Current status
    pub status: RitualStatus,
    /// 中断天数 / Days since last occurrence
    pub break_days: u32,
    /// 总出现次数 / Total occurrences
    pub total_occurrences: u64,
}

impl ContentRitualPattern {
    /// 生成仪式名称 / Generate ritual name
    pub fn name(&self) -> String {
        match &self.time_slot {
            Some(slot) => format!("{}{}仪式", slot.label_zh(), self.hint.label_zh()),
            None => format!("{}仪式", self.hint.label_zh()),
        }
    }

    /// 生成中文描述 / Generate Chinese description
    pub fn description_zh(&self) -> String {
        match self.status {
            RitualStatus::Candidate => {
                format!(
                    "连续{}天说{}，即将成为仪式",
                    self.consecutive_days,
                    self.hint.label_zh()
                )
            }
            RitualStatus::Active => {
                format!(
                    "每天说{}的固定互动，已持续{}天",
                    self.hint.label_zh(),
                    self.consecutive_days
                )
            }
            RitualStatus::Broken => {
                format!(
                    "说{}的互动已中断{}天",
                    self.hint.label_zh(),
                    self.break_days
                )
            }
            RitualStatus::Archived => {
                format!("说{}的互动已归档", self.hint.label_zh())
            }
        }
    }
}

// ── 每日交互记录 / Daily Interaction Record ──

/// 单日交互统计 / Single day interaction statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyRecord {
    /// 各时间槽的交互次数 / Interaction count per time slot
    pub slot_counts: HashMap<u8, u64>,
}

impl DailyRecord {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次交互 / Record one interaction
    pub fn record(&mut self, hour: u8) {
        *self.slot_counts.entry(hour.min(23)).or_insert(0) += 1;
    }

    /// 获取最活跃的时间槽 / Get most active time slot
    pub fn most_active_slot(&self) -> Option<u8> {
        self.slot_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| *hour)
    }
}

// ── 每日内容仪式记录 / Daily Content Ritual Record ──

/// 单日内容仪式统计 / Single day content ritual statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentDailyRecord {
    /// 各内容签名的出现次数 / Occurrence count per content hint key
    pub hint_counts: HashMap<String, u64>,
    /// 各签名最后出现的时间槽 / Last time slot per content hint key
    pub hint_time_slots: HashMap<String, u8>,
}

impl ContentDailyRecord {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次内容仪式 / Record one content ritual occurrence
    pub fn record(&mut self, hint: &ContentHint, hour: u8) {
        let key = hint.key();
        *self.hint_counts.entry(key.clone()).or_insert(0) += 1;
        self.hint_time_slots.insert(key, hour.min(23));
    }
}

// ── 配置 / Config ──

/// 仪式检测配置 / Ritual detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualConfig {
    /// 升级阈值天数 / Promotion threshold (days)
    pub promotion_threshold_days: u32,
    /// 中断阈值天数 / Break threshold (days)
    pub break_threshold_days: u32,
    /// 归档阈值天数 / Archive threshold (days)
    pub archive_threshold_days: u32,
    /// 最大活跃仪式数 / Max active rituals
    pub max_active_rituals: usize,
    /// 每日记录保留天数 / Daily record retention (days)
    pub record_retention_days: u32,
}

impl Default for RitualConfig {
    fn default() -> Self {
        Self {
            promotion_threshold_days: 7,
            break_threshold_days: 3,
            archive_threshold_days: 30,
            max_active_rituals: 10,
            record_retention_days: 60,
        }
    }
}

// ── 仪式检测器 / Ritual Detector ──

/// 共享仪式检测器 / Shared ritual detector
///
/// 通过统计每日交互时间分布，自动发现重复模式并升级为仪式。
/// C5 扩展：同时支持内容语义仪式检测（如"每晚说晚安"）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualDetector {
    /// 已识别的仪式模式 / Identified ritual patterns
    pub patterns: Vec<RitualPattern>,
    /// 每日交互记录（日期字符串 → 记录）/ Daily interaction records
    pub daily_records: HashMap<String, DailyRecord>,
    /// 配置 / Configuration
    pub config: RitualConfig,
    /// 下一个仪式 ID / Next ritual ID
    pub(crate) next_ritual_id: u64,
    /// 上次评估日期 / Last evaluation date
    pub(crate) last_eval_date: String,

    // ── C5: 内容仪式字段 / C5: Content ritual fields ──
    /// 内容仪式模式 / Content ritual patterns
    pub content_patterns: Vec<ContentRitualPattern>,
    /// 每日内容仪式记录 / Daily content ritual records
    pub content_daily_records: HashMap<String, ContentDailyRecord>,
    /// 下一个内容仪式 ID / Next content ritual ID
    pub(crate) next_content_ritual_id: u64,
    /// 上次内容评估日期 / Last content evaluation date
    pub(crate) last_content_eval_date: String,
}

impl RitualDetector {
    pub fn new(config: RitualConfig) -> Self {
        Self {
            patterns: Vec::new(),
            daily_records: HashMap::new(),
            config,
            next_ritual_id: 1,
            last_eval_date: String::new(),
            content_patterns: Vec::new(),
            content_daily_records: HashMap::new(),
            next_content_ritual_id: 1,
            last_content_eval_date: String::new(),
        }
    }

    pub fn default_new() -> Self {
        Self::new(RitualConfig::default())
    }

    /// 记录一次交互 / Record an interaction
    ///
    /// 在 process_message 中调用，记录消息的时间分布。
    pub fn record_interaction(&mut self, epoch_secs: i64) {
        let date_str = Self::epoch_to_date(epoch_secs);
        let hour = Self::epoch_to_hour(epoch_secs);

        self.daily_records.entry(date_str).or_default().record(hour);
    }

    /// 记录一次内容交互 / Record a content interaction
    ///
    /// 在 process_message 中调用，检测消息的仪式性内容并记录。
    /// 如果消息包含可识别的仪式性内容（如"晚安"），则记录到内容仪式系统。
    /// Called in process_message, detects ritual content in message and records it.
    pub fn record_content_interaction(&mut self, epoch_secs: i64, text: &str) {
        if let Some(hint) = ContentHint::detect(text) {
            let date_str = Self::epoch_to_date(epoch_secs);
            let hour = Self::epoch_to_hour(epoch_secs);
            self.content_daily_records
                .entry(date_str)
                .or_default()
                .record(&hint, hour);
        }
    }

    /// 每日评估 / Daily evaluation
    ///
    /// 由 Scheduler 每日调用，执行：
    /// - 从每日记录中提取高频时段
    /// - 更新仪式模式的连续天数
    /// - 升级候选为活跃仪式
    /// - 检测仪式中断
    /// - 清理过期记录
    pub fn evaluate_daily(&mut self, now_epoch_secs: i64) -> Vec<RitualEvent> {
        let mut events = Vec::new();
        let today = Self::epoch_to_date(now_epoch_secs);

        // 避免同日重复评估 / Avoid duplicate evaluation on same day
        if today == self.last_eval_date {
            return events;
        }
        self.last_eval_date = today.clone();

        // Step 1: 从昨日记录提取高频时段 / Extract high-frequency slots from yesterday
        // 先收集有效时段，避免借用冲突 / Collect valid slots first to avoid borrow conflict
        let yesterday = Self::epoch_to_date(now_epoch_secs - 86400);
        let active_slots: Vec<u8> = self
            .daily_records
            .get(&yesterday)
            .map(|rec| {
                rec.slot_counts
                    .iter()
                    .filter(|(_, &count)| count >= 2)
                    .map(|(&hour, _)| hour)
                    .collect()
            })
            .unwrap_or_default();
        for hour in active_slots {
            self.update_pattern(hour, now_epoch_secs, &mut events);
        }

        // Step 2: 检测仪式中断 / Check ritual breaks
        self.check_breaks(now_epoch_secs, &mut events);

        // Step 3: 清理过期记录 / Prune old records
        self.prune_old_records(now_epoch_secs);

        events
    }

    /// 每日内容仪式评估 / Daily content ritual evaluation
    ///
    /// 检查昨日的内容仪式记录，更新内容仪式模式状态。
    /// 与 `evaluate_daily` 独立，可分别调用。
    /// Checks yesterday's content ritual records, updates content ritual pattern status.
    pub fn evaluate_content_daily(&mut self, now_epoch_secs: i64) -> Vec<ContentRitualEvent> {
        let mut events = Vec::new();
        let today = Self::epoch_to_date(now_epoch_secs);

        // 避免同日重复评估 / Avoid duplicate evaluation on same day
        if today == self.last_content_eval_date {
            return events;
        }
        self.last_content_eval_date = today;

        // Step 1: 从昨日记录提取内容签名 / Extract content hints from yesterday
        let yesterday = Self::epoch_to_date(now_epoch_secs - 86400);
        let active_hints: Vec<(String, u8)> = self
            .content_daily_records
            .get(&yesterday)
            .map(|rec| {
                rec.hint_counts
                    .iter()
                    .filter(|(_, &count)| count >= 1)
                    .map(|(key, _)| {
                        // 获取关联时间槽 / Get associated time slot
                        let hour = rec.hint_time_slots.get(key).copied().unwrap_or(0);
                        (key.clone(), hour)
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (hint_key, hour) in active_hints {
            if let Some(hint) = ContentHint::from_key(&hint_key) {
                let time_slot = Some(TimeSlot::new(hour));
                self.update_content_pattern(&hint, time_slot, now_epoch_secs, &mut events);
            }
        }

        // Step 2: 检测内容仪式中断 / Check content ritual breaks
        self.check_content_breaks(now_epoch_secs, &mut events);

        // Step 3: 清理过期内容记录 / Prune old content records
        self.prune_old_content_records(now_epoch_secs);

        events
    }

    /// 合并每日评估（时间 + 内容）/ Combined daily evaluation (time + content)
    ///
    /// 一次性执行时间仪式和内容仪式的每日评估。
    /// Runs both time-based and content-based daily evaluation in one call.
    pub fn evaluate_all_daily(
        &mut self,
        now_epoch_secs: i64,
    ) -> (Vec<RitualEvent>, Vec<ContentRitualEvent>) {
        let time_events = self.evaluate_daily(now_epoch_secs);
        let content_events = self.evaluate_content_daily(now_epoch_secs);
        (time_events, content_events)
    }

    /// 更新仪式模式 / Update ritual pattern for a time slot
    fn update_pattern(&mut self, hour: u8, now_epoch_secs: i64, events: &mut Vec<RitualEvent>) {
        let slot = TimeSlot::new(hour);

        // 先计算活跃仪式数（不可变借用）/ Count active rituals first (immutable borrow)
        let active_count = self
            .patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count();

        // 查找已有模式的索引 / Find existing pattern index
        let existing_idx = self.patterns.iter().position(|p| p.time_slot == slot);

        if let Some(idx) = existing_idx {
            let pattern = &mut self.patterns[idx];
            // 已有模式：更新连续天数 / Existing pattern: update consecutive days
            if pattern.status == RitualStatus::Archived {
                return;
            }
            pattern.consecutive_days += 1;
            pattern.last_occurrence_at = now_epoch_secs;
            pattern.total_interactions += 1;
            pattern.break_days = 0;

            // 检查是否可以升级 / Check for promotion (with active limit)
            if pattern.status == RitualStatus::Candidate
                && pattern.consecutive_days >= self.config.promotion_threshold_days
                && active_count < self.config.max_active_rituals
            {
                pattern.status = RitualStatus::Active;
                events.push(RitualEvent::Promoted {
                    id: pattern.id,
                    name: pattern.name(),
                    time_slot: slot,
                });
                tracing::info!(
                    "[仪式] 升级: {} (连续{}天)",
                    pattern.name(),
                    pattern.consecutive_days
                );
            }

            // 中断恢复 / Break recovery
            if pattern.status == RitualStatus::Broken {
                pattern.status = RitualStatus::Active;
                events.push(RitualEvent::Resumed {
                    id: pattern.id,
                    name: pattern.name(),
                });
                tracing::info!("[仪式] 恢复: {}", pattern.name());
            }
        } else {
            // 新模式 / New pattern
            if active_count < self.config.max_active_rituals {
                let id = self.next_ritual_id;
                self.next_ritual_id += 1;
                self.patterns.push(RitualPattern {
                    id,
                    time_slot: slot,
                    consecutive_days: 1,
                    first_seen_at: now_epoch_secs,
                    last_occurrence_at: now_epoch_secs,
                    status: RitualStatus::Candidate,
                    break_days: 0,
                    total_interactions: 1,
                });
            }
        }
    }

    /// 更新内容仪式模式 / Update content ritual pattern
    fn update_content_pattern(
        &mut self,
        hint: &ContentHint,
        time_slot: Option<TimeSlot>,
        now_epoch_secs: i64,
        events: &mut Vec<ContentRitualEvent>,
    ) {
        let hint_key = hint.key();

        // 计算活跃内容仪式数 / Count active content rituals
        let active_count = self
            .content_patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count();

        // 查找已有模式的索引 / Find existing pattern index
        let existing_idx = self
            .content_patterns
            .iter()
            .position(|p| p.hint.key() == hint_key);

        if let Some(idx) = existing_idx {
            let pattern = &mut self.content_patterns[idx];
            if pattern.status == RitualStatus::Archived {
                return;
            }
            pattern.consecutive_days += 1;
            pattern.last_occurrence_at = now_epoch_secs;
            pattern.total_occurrences += 1;
            pattern.break_days = 0;
            // 更新时间槽（取最新）/ Update time slot to latest
            if time_slot.is_some() {
                pattern.time_slot = time_slot;
            }

            // 检查升级 / Check for promotion
            if pattern.status == RitualStatus::Candidate
                && pattern.consecutive_days >= self.config.promotion_threshold_days
                && active_count < self.config.max_active_rituals
            {
                pattern.status = RitualStatus::Active;
                events.push(ContentRitualEvent::Promoted {
                    id: pattern.id,
                    name: pattern.name(),
                    hint: hint.clone(),
                });
                tracing::info!(
                    "[内容仪式] 升级: {} (连续{}天)",
                    pattern.name(),
                    pattern.consecutive_days
                );
            }

            // 中断恢复 / Break recovery
            if pattern.status == RitualStatus::Broken {
                pattern.status = RitualStatus::Active;
                events.push(ContentRitualEvent::Resumed {
                    id: pattern.id,
                    name: pattern.name(),
                });
                tracing::info!("[内容仪式] 恢复: {}", pattern.name());
            }
        } else {
            // 新模式 / New pattern
            if active_count < self.config.max_active_rituals {
                let id = self.next_content_ritual_id;
                self.next_content_ritual_id += 1;
                self.content_patterns.push(ContentRitualPattern {
                    id,
                    hint: hint.clone(),
                    time_slot,
                    consecutive_days: 1,
                    first_seen_at: now_epoch_secs,
                    last_occurrence_at: now_epoch_secs,
                    status: RitualStatus::Candidate,
                    break_days: 0,
                    total_occurrences: 1,
                });
            }
        }
    }

    /// 检测仪式中断 / Check for ritual breaks
    fn check_breaks(&mut self, now_epoch_secs: i64, events: &mut Vec<RitualEvent>) {
        for pattern in self.patterns.iter_mut() {
            if pattern.status != RitualStatus::Active {
                continue;
            }
            let days_since = (now_epoch_secs - pattern.last_occurrence_at) / 86400;
            if days_since > 0 {
                pattern.break_days = days_since as u32;
            }

            if pattern.break_days >= self.config.archive_threshold_days {
                pattern.status = RitualStatus::Archived;
                events.push(RitualEvent::Archived {
                    id: pattern.id,
                    name: pattern.name(),
                });
                tracing::info!(
                    "[仪式] 归档: {} (中断{}天)",
                    pattern.name(),
                    pattern.break_days
                );
            } else if pattern.break_days >= self.config.break_threshold_days {
                pattern.status = RitualStatus::Broken;
                pattern.consecutive_days =
                    pattern.consecutive_days.saturating_sub(pattern.break_days);
                events.push(RitualEvent::Broken {
                    id: pattern.id,
                    name: pattern.name(),
                    break_days: pattern.break_days,
                });
                tracing::info!("[仪式] 中断: {} ({}天)", pattern.name(), pattern.break_days);
            }
        }
    }

    /// 检测内容仪式中断 / Check for content ritual breaks
    fn check_content_breaks(&mut self, now_epoch_secs: i64, events: &mut Vec<ContentRitualEvent>) {
        for pattern in self.content_patterns.iter_mut() {
            if pattern.status != RitualStatus::Active {
                continue;
            }
            let days_since = (now_epoch_secs - pattern.last_occurrence_at) / 86400;
            if days_since > 0 {
                pattern.break_days = days_since as u32;
            }

            if pattern.break_days >= self.config.archive_threshold_days {
                pattern.status = RitualStatus::Archived;
                events.push(ContentRitualEvent::Archived {
                    id: pattern.id,
                    name: pattern.name(),
                });
                tracing::info!(
                    "[内容仪式] 归档: {} (中断{}天)",
                    pattern.name(),
                    pattern.break_days
                );
            } else if pattern.break_days >= self.config.break_threshold_days {
                pattern.status = RitualStatus::Broken;
                pattern.consecutive_days =
                    pattern.consecutive_days.saturating_sub(pattern.break_days);
                events.push(ContentRitualEvent::Broken {
                    id: pattern.id,
                    name: pattern.name(),
                    break_days: pattern.break_days,
                });
                tracing::info!(
                    "[内容仪式] 中断: {} ({}天)",
                    pattern.name(),
                    pattern.break_days
                );
            }
        }
    }

    /// 清理过期记录 / Prune old daily records
    fn prune_old_records(&mut self, now_epoch_secs: i64) {
        let retention_secs = self.config.record_retention_days as i64 * 86400;
        let cutoff = now_epoch_secs - retention_secs;
        self.daily_records
            .retain(|date_str, _| Self::date_to_epoch(date_str) > cutoff);
    }

    /// 清理过期内容记录 / Prune old content daily records
    fn prune_old_content_records(&mut self, now_epoch_secs: i64) {
        let retention_secs = self.config.record_retention_days as i64 * 86400;
        let cutoff = now_epoch_secs - retention_secs;
        self.content_daily_records
            .retain(|date_str, _| Self::date_to_epoch(date_str) > cutoff);
    }

    /// 生成提醒 prompt 片段 / Generate reminder prompt fragment
    ///
    /// 当有中断的仪式时，生成温和提醒注入 System Prompt。
    /// 注意：此方法不检查关系阶段门控，调用方应自行判断或使用 `prompt_fragment_gated()`。
    /// Note: This method does NOT check relationship gate; caller should gate manually
    /// or use `prompt_fragment_gated()`.
    pub fn prompt_fragment(&self) -> String {
        let broken: Vec<_> = self
            .patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Broken)
            .collect();

        if broken.is_empty() {
            return String::new();
        }

        let reminders: Vec<String> = broken
            .iter()
            .take(2) // 最多提醒2个 / Max 2 reminders
            .map(|p| format!("{}（中断{}天）", p.time_slot.label_zh(), p.break_days))
            .collect();

        format!("[仪式提醒] 好久没在{}聊了", reminders.join("和"))
    }

    /// 生成内容仪式提醒 prompt 片段 / Generate content ritual reminder prompt fragment
    ///
    /// 当有中断的内容仪式时，生成温和提醒。
    /// Generates gentle reminders when content rituals are broken.
    pub fn content_prompt_fragment(&self) -> String {
        let broken: Vec<_> = self
            .content_patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Broken)
            .collect();

        if broken.is_empty() {
            return String::new();
        }

        let reminders: Vec<String> = broken
            .iter()
            .take(2) // 最多提醒2个 / Max 2 reminders
            .map(|p| format!("{}（中断{}天）", p.hint.label_zh(), p.break_days))
            .collect();

        format!("[仪式提醒] 好久没说{}了", reminders.join("和"))
    }

    /// 合并 prompt 片段（时间 + 内容）/ Combined prompt fragment (time + content)
    pub fn combined_prompt_fragment(&self) -> String {
        let time_frag = self.prompt_fragment();
        let content_frag = self.content_prompt_fragment();
        match (time_frag.is_empty(), content_frag.is_empty()) {
            (true, true) => String::new(),
            (true, false) => content_frag,
            (false, true) => time_frag,
            (false, false) => format!("{}\n{}", time_frag, content_frag),
        }
    }

    /// 关系阶段门控 — 是否应发送仪式提醒 / Relationship gate for ritual reminders.
    ///
    /// 门控规则：
    /// - Acquaintance (0): 不提醒 / No reminders
    /// - Familiar (1): 温和提醒 / Gentle reminders
    /// - Trusted (2) / Deep (3): 正常提醒 / Normal reminders
    ///
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    ///   (0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep)
    /// @return 是否应发送提醒 / Whether reminders should be sent
    pub fn should_remind(&self, relation_ordinal: u8) -> bool {
        relation_ordinal >= 1 // ≥Familiar 才提醒 / Only remind at ≥Familiar
    }

    /// 带关系阶段门控的提醒 prompt 片段 / Gated reminder prompt fragment.
    ///
    /// 结合 `should_remind()` 和 `prompt_fragment()`，仅在关系阶段足够时返回提醒。
    /// Combines `should_remind()` and `prompt_fragment()`, returns reminder only
    /// when relationship stage is sufficient.
    ///
    /// @param relation_ordinal 关系阶段序数 / Relationship stage ordinal
    /// @return 提醒片段（空串表示不提醒）/ Reminder fragment (empty = no reminder)
    pub fn prompt_fragment_gated(&self, relation_ordinal: u8) -> String {
        if !self.should_remind(relation_ordinal) {
            return String::new();
        }
        self.prompt_fragment()
    }

    /// 带关系阶段门控的内容仪式提醒 / Gated content ritual reminder prompt fragment.
    pub fn content_prompt_fragment_gated(&self, relation_ordinal: u8) -> String {
        if !self.should_remind(relation_ordinal) {
            return String::new();
        }
        self.content_prompt_fragment()
    }

    /// 带关系阶段门控的合并提醒 / Gated combined prompt fragment.
    pub fn combined_prompt_fragment_gated(&self, relation_ordinal: u8) -> String {
        if !self.should_remind(relation_ordinal) {
            return String::new();
        }
        self.combined_prompt_fragment()
    }

    /// 获取活跃仪式数 / Get active ritual count
    pub fn active_count(&self) -> usize {
        self.patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count()
    }

    /// 获取所有活跃仪式 / Get all active rituals
    pub fn active_rituals(&self) -> Vec<&RitualPattern> {
        self.patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .collect()
    }

    /// 获取活跃内容仪式数 / Get active content ritual count
    pub fn active_content_count(&self) -> usize {
        self.content_patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .count()
    }

    /// 获取所有活跃内容仪式 / Get all active content rituals
    pub fn active_content_rituals(&self) -> Vec<&ContentRitualPattern> {
        self.content_patterns
            .iter()
            .filter(|p| p.status == RitualStatus::Active)
            .collect()
    }

    // ── 辅助方法 / Helper methods ──

    /// epoch 秒转日期字符串 / Epoch seconds to date string (YYYY-MM-DD)
    fn epoch_to_date(epoch_secs: i64) -> String {
        // 简单实现：基于 Unix epoch 计算 / Simple implementation based on Unix epoch
        let days_since_epoch = epoch_secs / 86400;
        // 1970-01-01 + days
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

    /// 日期字符串转 epoch / Date string to epoch seconds (approximate)
    fn date_to_epoch(date_str: &str) -> i64 {
        // 解析 YYYY-MM-DD / Parse YYYY-MM-DD
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return 0;
        }
        let year: i64 = parts[0].parse().unwrap_or(1970);
        let month: i64 = parts[1].parse().unwrap_or(1);
        let day: i64 = parts[2].parse().unwrap_or(1);

        // 简单近似计算 / Simple approximate calculation
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

// ── 仪式事件 / Ritual Event ──

/// 仪式生命周期事件 / Ritual lifecycle event
#[derive(Debug, Clone)]
pub enum RitualEvent {
    /// 候选升级为活跃仪式 / Candidate promoted to active ritual
    Promoted {
        id: u64,
        name: String,
        time_slot: TimeSlot,
    },
    /// 仪式中断 / Ritual broken
    Broken {
        id: u64,
        name: String,
        break_days: u32,
    },
    /// 仪式恢复 / Ritual resumed
    Resumed { id: u64, name: String },
    /// 仪式归档 / Ritual archived
    Archived { id: u64, name: String },
}

// ── 内容仪式事件 / Content Ritual Event ──

/// 内容仪式生命周期事件 / Content ritual lifecycle event
#[derive(Debug, Clone)]
pub enum ContentRitualEvent {
    /// 候选升级为活跃内容仪式 / Candidate promoted to active content ritual
    Promoted {
        id: u64,
        name: String,
        hint: ContentHint,
    },
    /// 内容仪式中断 / Content ritual broken
    Broken {
        id: u64,
        name: String,
        break_days: u32,
    },
    /// 内容仪式恢复 / Content ritual resumed
    Resumed { id: u64, name: String },
    /// 内容仪式归档 / Content ritual archived
    Archived { id: u64, name: String },
}

// ── 测试 / Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_slot_from_epoch() {
        // 2026-06-26 22:00:00 UTC = 1782511200
        let slot = TimeSlot::from_epoch(1782511200);
        assert_eq!(slot.hour, 22);
    }

    #[test]
    fn test_time_slot_label() {
        let slot = TimeSlot::new(22);
        assert_eq!(slot.label_zh(), "晚间10点");
        let slot = TimeSlot::new(7);
        assert_eq!(slot.label_zh(), "早晨7点");
    }

    #[test]
    fn test_record_interaction() {
        let mut detector = RitualDetector::default_new();
        // 2026-06-26 22:00 UTC = 1782511200
        let base = 1782511200i64;
        for i in 0..5 {
            detector.record_interaction(base + i * 60);
        }
        let date = RitualDetector::epoch_to_date(base);
        let record = detector.daily_records.get(&date).unwrap();
        assert_eq!(*record.slot_counts.get(&22).unwrap_or(&0), 5);
    }

    #[test]
    fn test_ritual_promotion() {
        let mut detector = RitualDetector::default_new();
        assert_eq!(detector.config.promotion_threshold_days, 7);

        // 模拟7天连续互动 / Simulate 7 consecutive days of interaction
        // 2026-06-20 22:00 UTC = 1781992800
        let base_date = 1781992800i64;
        let mut events = Vec::new();
        for day in 0..7 {
            let day_epoch = base_date + day * 86400;
            // 记录当日交互 / Record interactions for the day
            detector.record_interaction(day_epoch); // 22:00
            detector.record_interaction(day_epoch + 300); // 22:05

            // 次日评估 / Evaluate next day
            events = detector.evaluate_daily(day_epoch + 86400);
        }

        // 应该有一个升级事件 / Should have a promotion event
        let promoted = events
            .iter()
            .any(|e| matches!(e, RitualEvent::Promoted { .. }));
        assert!(promoted, "Expected ritual promotion after 7 days");
        assert_eq!(detector.active_count(), 1);
    }

    #[test]
    fn test_ritual_break_detection() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3; // 加速测试

        // 先建立仪式 / First establish a ritual
        // 2026-06-20 22:00 UTC = 1781992800
        let base = 1781992800i64;
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.evaluate_daily(day_epoch + 86400);
        }
        assert_eq!(detector.active_count(), 1);

        // 模拟中断4天 / Simulate 4-day break
        let break_start = base + 4 * 86400;
        let events = detector.evaluate_daily(break_start + 4 * 86400);

        let broken = events
            .iter()
            .any(|e| matches!(e, RitualEvent::Broken { .. }));
        assert!(broken, "Expected ritual break detection");
    }

    #[test]
    fn test_prompt_fragment_empty() {
        let detector = RitualDetector::default_new();
        assert!(detector.prompt_fragment().is_empty());
    }

    #[test]
    fn test_prompt_fragment_with_break() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;

        // 建立仪式 / Establish ritual
        // 2026-06-20 22:00 UTC = 1781992800
        let base = 1781992800i64;
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.evaluate_daily(day_epoch + 86400);
        }

        // 制造中断 / Create break
        detector.evaluate_daily(base + 7 * 86400);

        let fragment = detector.prompt_fragment();
        assert!(!fragment.is_empty(), "Expected reminder prompt fragment");
        assert!(fragment.contains("仪式提醒"));
    }

    #[test]
    fn test_daily_record_most_active() {
        let mut record = DailyRecord::new();
        record.record(22);
        record.record(22);
        record.record(22);
        record.record(10);
        assert_eq!(record.most_active_slot(), Some(22));
    }

    #[test]
    fn test_ritual_pattern_description() {
        let pattern = RitualPattern {
            id: 1,
            time_slot: TimeSlot::new(22),
            consecutive_days: 10,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_interactions: 20,
        };
        let desc = pattern.description_zh();
        assert!(desc.contains("晚间10点"));
        assert!(desc.contains("10天"));
    }

    #[test]
    fn test_epoch_to_date_roundtrip() {
        // 2026-06-26 22:00 UTC = 1782511200
        let epoch = 1782511200i64;
        let date = RitualDetector::epoch_to_date(epoch);
        assert!(
            date.starts_with("2026"),
            "Date should be in 2026, got: {}",
            date
        );
    }

    #[test]
    fn test_max_active_rituals_limit() {
        let mut detector = RitualDetector::default_new();
        detector.config.max_active_rituals = 2;
        detector.config.promotion_threshold_days = 2;

        // 在3个不同日期的同一时段建立仪式，但限制最多2个活跃
        // 2026-06-20 22:00 UTC = 1781992800
        let base = 1781992800i64;
        // Day 0: slot 22
        detector.record_interaction(base);
        detector.record_interaction(base + 300);
        // Day 0: slot 10 (same day, different hour)
        detector.record_interaction(base - 12 * 3600); // 10:00 same day
        detector.record_interaction(base - 12 * 3600 + 300);
        // Day 0: slot 14 (same day, different hour)
        detector.record_interaction(base - 8 * 3600); // 14:00 same day
        detector.record_interaction(base - 8 * 3600 + 300);

        detector.evaluate_daily(base + 86400);

        // Day 1: repeat same slots
        let day1 = base + 86400;
        detector.record_interaction(day1);
        detector.record_interaction(day1 + 300);
        detector.record_interaction(day1 - 12 * 3600);
        detector.record_interaction(day1 - 12 * 3600 + 300);
        detector.record_interaction(day1 - 8 * 3600);
        detector.record_interaction(day1 - 8 * 3600 + 300);

        detector.evaluate_daily(day1 + 86400);

        // 活跃仪式不应超过 max_active_rituals / Active rituals should not exceed limit
        assert!(
            detector.active_count() <= detector.config.max_active_rituals,
            "active_count {} should be <= max_active_rituals {}",
            detector.active_count(),
            detector.config.max_active_rituals
        );
    }

    // ══════════════════════════════════════════════════════════════
    // C1.3: should_remind / prompt_fragment_gated 门控测试
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_should_remind_acquaintance_no() {
        // 初识不提醒 / Acquaintance: no reminders
        let detector = RitualDetector::default_new();
        assert!(!detector.should_remind(0), "初识不应发送仪式提醒");
    }

    #[test]
    fn test_should_remind_familiar_yes() {
        // 熟悉阶段可提醒 / Familiar: reminders allowed
        let detector = RitualDetector::default_new();
        assert!(detector.should_remind(1), "熟悉阶段应可发送仪式提醒");
    }

    #[test]
    fn test_should_remind_trusted_deep_yes() {
        // 信任/深度阶段可提醒 / Trusted/Deep: reminders allowed
        let detector = RitualDetector::default_new();
        assert!(detector.should_remind(2), "信任阶段应可发送仪式提醒");
        assert!(detector.should_remind(3), "深度阶段应可发送仪式提醒");
    }

    #[test]
    fn test_prompt_fragment_gated_acquaintance_empty() {
        // 初识阶段：即使有中断仪式也不提醒 / Acquaintance: no reminder even with broken rituals
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.evaluate_daily(day_epoch + 86400);
        }
        // 制造中断 / Create break
        detector.evaluate_daily(base + 7 * 86400);
        // 非门控版本应有内容 / Ungated version should have content
        assert!(!detector.prompt_fragment().is_empty());
        // 门控版本在初识时为空 / Gated version empty at Acquaintance
        assert!(detector.prompt_fragment_gated(0).is_empty());
    }

    #[test]
    fn test_prompt_fragment_gated_familiar_not_empty() {
        // 熟悉阶段：有中断仪式时提醒 / Familiar: reminder when broken rituals exist
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.evaluate_daily(day_epoch + 86400);
        }
        detector.evaluate_daily(base + 7 * 86400);
        let gated = detector.prompt_fragment_gated(1);
        assert!(!gated.is_empty(), "熟悉阶段有中断仪式时应提醒");
        assert!(gated.contains("仪式提醒"));
    }

    // ══════════════════════════════════════════════════════════════
    // C5: 内容语义仪式检测测试 / Content ritual detection tests
    // ══════════════════════════════════════════════════════════════

    #[test]
    fn test_content_hint_detect_goodnight() {
        assert_eq!(ContentHint::detect("晚安"), Some(ContentHint::Goodnight));
        assert_eq!(
            ContentHint::detect("我要睡了"),
            Some(ContentHint::Goodnight)
        );
        assert_eq!(
            ContentHint::detect("Good night!"),
            Some(ContentHint::Goodnight)
        );
    }

    #[test]
    fn test_content_hint_detect_good_morning() {
        assert_eq!(ContentHint::detect("早安"), Some(ContentHint::GoodMorning));
        assert_eq!(
            ContentHint::detect("早上好"),
            Some(ContentHint::GoodMorning)
        );
        assert_eq!(
            ContentHint::detect("Good morning!"),
            Some(ContentHint::GoodMorning)
        );
    }

    #[test]
    fn test_content_hint_detect_weekend() {
        assert_eq!(
            ContentHint::detect("周末愉快"),
            Some(ContentHint::WeekendGreeting)
        );
        assert_eq!(
            ContentHint::detect("周末快乐！"),
            Some(ContentHint::WeekendGreeting)
        );
    }

    #[test]
    fn test_content_hint_detect_holiday() {
        assert_eq!(
            ContentHint::detect("新年快乐"),
            Some(ContentHint::HolidayGreeting)
        );
        assert_eq!(
            ContentHint::detect("中秋快乐"),
            Some(ContentHint::LunarHolidayGreeting)
        );
        // 农历节日检测 / Lunar holiday detection
        assert_eq!(
            ContentHint::detect("端午安康"),
            Some(ContentHint::LunarHolidayGreeting)
        );
        assert_eq!(
            ContentHint::detect("春节快乐"),
            Some(ContentHint::LunarHolidayGreeting)
        );
        assert_eq!(
            ContentHint::detect("元宵快乐"),
            Some(ContentHint::LunarHolidayGreeting)
        );
    }

    #[test]
    fn test_content_hint_detect_none() {
        // 非仪式性内容不应匹配 / Non-ritual content should not match
        assert_eq!(ContentHint::detect("今天天气不错"), None);
        assert_eq!(ContentHint::detect("帮我写个程序"), None);
        assert_eq!(ContentHint::detect(""), None);
    }

    #[test]
    fn test_content_hint_label_zh() {
        assert_eq!(ContentHint::Goodnight.label_zh(), "晚安");
        assert_eq!(ContentHint::GoodMorning.label_zh(), "早安");
        assert_eq!(ContentHint::WeekendGreeting.label_zh(), "周末问候");
        assert_eq!(ContentHint::HolidayGreeting.label_zh(), "节日问候");
        assert_eq!(ContentHint::LunarHolidayGreeting.label_zh(), "农历节日问候");
    }

    #[test]
    fn test_content_hint_key_roundtrip() {
        // 键的反解应与原始一致 / Key roundtrip should match original
        let hints = [
            ContentHint::Goodnight,
            ContentHint::GoodMorning,
            ContentHint::WeekendGreeting,
            ContentHint::HolidayGreeting,
            ContentHint::LunarHolidayGreeting,
            ContentHint::Custom("打卡".to_string()),
        ];
        for hint in &hints {
            let key = hint.key();
            let restored = ContentHint::from_key(&key);
            assert_eq!(
                restored.as_ref(),
                Some(hint),
                "Roundtrip failed for {:?}",
                hint
            );
        }
    }

    #[test]
    fn test_content_hint_ordinal() {
        assert_eq!(ContentHint::Goodnight.ordinal(), 0);
        assert_eq!(ContentHint::GoodMorning.ordinal(), 1);
        assert_eq!(ContentHint::WeekendGreeting.ordinal(), 2);
        assert_eq!(ContentHint::HolidayGreeting.ordinal(), 3);
        assert_eq!(ContentHint::LunarHolidayGreeting.ordinal(), 4);
        assert_eq!(ContentHint::Custom("x".to_string()).ordinal(), 255);
    }

    #[test]
    fn test_record_content_interaction() {
        let mut detector = RitualDetector::default_new();
        // 2026-06-26 22:00 UTC = 1782511200
        let base = 1782511200i64;
        detector.record_content_interaction(base, "晚安，好梦");
        let date = RitualDetector::epoch_to_date(base);
        let record = detector.content_daily_records.get(&date).unwrap();
        assert_eq!(*record.hint_counts.get("goodnight").unwrap_or(&0), 1);
        assert_eq!(*record.hint_time_slots.get("goodnight").unwrap_or(&0), 22);
    }

    #[test]
    fn test_record_content_interaction_no_match() {
        let mut detector = RitualDetector::default_new();
        let base = 1782511200i64;
        detector.record_content_interaction(base, "今天天气不错");
        assert!(detector.content_daily_records.is_empty());
    }

    #[test]
    fn test_content_ritual_promotion() {
        // 连续7天说晚安 → 升级为内容仪式 / 7 consecutive goodnights → promote
        let mut detector = RitualDetector::default_new();
        let base = 1781992800i64; // 2026-06-20 22:00 UTC

        let mut last_events = Vec::new();
        for day in 0..7 {
            let day_epoch = base + day * 86400;
            // 每天说晚安 / Say goodnight each day
            detector.record_content_interaction(day_epoch, "晚安");
            // 次日评估 / Evaluate next day
            last_events = detector.evaluate_content_daily(day_epoch + 86400);
        }

        // 应有升级事件 / Should have promotion event
        let promoted = last_events
            .iter()
            .any(|e| matches!(e, ContentRitualEvent::Promoted { .. }));
        assert!(promoted, "Expected content ritual promotion after 7 days");
        assert_eq!(detector.active_content_count(), 1);

        // 验证仪式属性 / Verify ritual properties
        let ritual = &detector.content_patterns[0];
        assert_eq!(ritual.hint, ContentHint::Goodnight);
        assert_eq!(ritual.consecutive_days, 7);
        assert!(ritual.time_slot.is_some());
    }

    #[test]
    fn test_content_ritual_break_detection() {
        // 建立内容仪式后中断 / Establish content ritual then break it
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;

        // 连续3天说早安 / Say good morning for 3 days
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_content_interaction(day_epoch, "早安");
            detector.evaluate_content_daily(day_epoch + 86400);
        }
        assert_eq!(detector.active_content_count(), 1);

        // 模拟中断4天 / Simulate 4-day break
        let events = detector.evaluate_content_daily(base + 7 * 86400);
        let broken = events
            .iter()
            .any(|e| matches!(e, ContentRitualEvent::Broken { .. }));
        assert!(broken, "Expected content ritual break detection");
    }

    #[test]
    fn test_content_ritual_pattern_name() {
        let pattern = ContentRitualPattern {
            id: 1,
            hint: ContentHint::Goodnight,
            time_slot: Some(TimeSlot::new(22)),
            consecutive_days: 7,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_occurrences: 7,
        };
        assert_eq!(pattern.name(), "晚间10点晚安仪式");

        let pattern_no_slot = ContentRitualPattern {
            id: 2,
            hint: ContentHint::GoodMorning,
            time_slot: None,
            consecutive_days: 5,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Candidate,
            break_days: 0,
            total_occurrences: 5,
        };
        assert_eq!(pattern_no_slot.name(), "早安仪式");
    }

    #[test]
    fn test_content_ritual_pattern_description() {
        let pattern = ContentRitualPattern {
            id: 1,
            hint: ContentHint::Goodnight,
            time_slot: Some(TimeSlot::new(22)),
            consecutive_days: 10,
            first_seen_at: 0,
            last_occurrence_at: 0,
            status: RitualStatus::Active,
            break_days: 0,
            total_occurrences: 10,
        };
        let desc = pattern.description_zh();
        assert!(desc.contains("晚安"));
        assert!(desc.contains("10天"));
    }

    #[test]
    fn test_content_prompt_fragment_empty() {
        let detector = RitualDetector::default_new();
        assert!(detector.content_prompt_fragment().is_empty());
    }

    #[test]
    fn test_content_prompt_fragment_with_break() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;

        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_content_interaction(day_epoch, "晚安");
            detector.evaluate_content_daily(day_epoch + 86400);
        }
        // 制造中断 / Create break
        detector.evaluate_content_daily(base + 7 * 86400);

        let fragment = detector.content_prompt_fragment();
        assert!(!fragment.is_empty(), "Expected content ritual reminder");
        assert!(fragment.contains("仪式提醒"));
        assert!(fragment.contains("晚安"));
    }

    #[test]
    fn test_combined_prompt_fragment() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;

        // 建立时间仪式 + 内容仪式 / Establish time + content rituals
        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.record_content_interaction(day_epoch, "晚安");
            detector.evaluate_daily(day_epoch + 86400);
            detector.evaluate_content_daily(day_epoch + 86400);
        }

        // 制造中断 / Create breaks
        detector.evaluate_daily(base + 7 * 86400);
        detector.evaluate_content_daily(base + 7 * 86400);

        let combined = detector.combined_prompt_fragment();
        assert!(!combined.is_empty());
        // 应同时包含时间和内容提醒 / Should contain both time and content reminders
        assert!(combined.contains("仪式提醒"));
    }

    #[test]
    fn test_content_prompt_fragment_gated() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;

        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_content_interaction(day_epoch, "晚安");
            detector.evaluate_content_daily(day_epoch + 86400);
        }
        detector.evaluate_content_daily(base + 7 * 86400);

        // 初识不提醒 / Acquaintance: no reminder
        assert!(detector.content_prompt_fragment_gated(0).is_empty());
        // 熟悉可提醒 / Familiar: reminder allowed
        assert!(!detector.content_prompt_fragment_gated(1).is_empty());
    }

    #[test]
    fn test_evaluate_all_daily() {
        let mut detector = RitualDetector::default_new();
        detector.config.promotion_threshold_days = 3;
        let base = 1781992800i64;

        for day in 0..3 {
            let day_epoch = base + day * 86400;
            detector.record_interaction(day_epoch);
            detector.record_interaction(day_epoch + 300);
            detector.record_content_interaction(day_epoch, "早安");
            let (time_ev, content_ev) = detector.evaluate_all_daily(day_epoch + 86400);
            // 第3天应有升级事件 / Day 3 should have promotion events
            if day == 2 {
                assert!(time_ev
                    .iter()
                    .any(|e| matches!(e, RitualEvent::Promoted { .. })));
                assert!(content_ev
                    .iter()
                    .any(|e| matches!(e, ContentRitualEvent::Promoted { .. })));
            }
        }
    }

    #[test]
    fn test_content_daily_record() {
        let mut record = ContentDailyRecord::new();
        record.record(&ContentHint::Goodnight, 22);
        record.record(&ContentHint::Goodnight, 23); // 更新时间槽
        record.record(&ContentHint::GoodMorning, 7);
        assert_eq!(*record.hint_counts.get("goodnight").unwrap(), 2);
        assert_eq!(*record.hint_counts.get("good_morning").unwrap(), 1);
        assert_eq!(*record.hint_time_slots.get("goodnight").unwrap(), 23); // 最后一次
    }

    #[test]
    fn test_multiple_content_hints_same_day() {
        // 同一天说早安和晚安 / Say good morning and good night on same day
        let mut detector = RitualDetector::default_new();
        // 2026-06-20 07:00 UTC for 早安, 22:00 UTC for 晚安 (same day)
        let base = 1781992800i64; // 22:00
        let morning = base - 15 * 3600; // 07:00 same day

        detector.record_content_interaction(morning, "早安");
        detector.record_content_interaction(base, "晚安");

        let date = RitualDetector::epoch_to_date(base);
        let record = detector.content_daily_records.get(&date).unwrap();
        assert_eq!(record.hint_counts.len(), 2);
    }
}
