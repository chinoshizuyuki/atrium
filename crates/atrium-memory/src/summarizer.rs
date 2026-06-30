// SPDX-License-Identifier: MIT
//! 对话摘要管理器
//!
//! 当 token 预算低于阈值时，自动触发 LLM 摘要压缩。
//! 摘要存储在 sled 中，跨 session 持久化。
//! DialogSummarizer — Conversation summarizer.
//!
//! Automatically triggers LLM summary compression when token budget
//! exceeds the configured threshold. Summaries persisted to sled for
//! cross-session continuity.

use serde::{Deserialize, Serialize};

/// 对话摘要条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    /// 摘要文本
    pub text: String,
    /// 涵盖的起始消息序号
    pub start_id: u64,
    /// 涵盖的结束消息序号
    pub end_id: u64,
    /// 生成时间戳
    pub created_at: i64,
    /// 摘要的 token 估算
    pub token_estimate: usize,
}

/// 摘要触发器配置
#[derive(Debug, Clone)]
pub struct SummaryConfig {
    /// token 使用率超此阈值触发摘要
    pub trigger_ratio: f64,
    /// 至少 N 条消息后才触发
    pub min_messages: u64,
    /// 每轮摘要覆盖的最近消息数
    pub window_size: usize,
    /// 保留的最近轮次（不参与摘要，保持在近程上下文）
    pub keep_recent: usize,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            trigger_ratio: 0.70,
            min_messages: 10,
            window_size: 20,
            keep_recent: 5,
        }
    }
}

/// 对话摘要管理器
pub struct ConversationSummarizer {
    /// 已生成的摘要列表（按时间顺序）
    summaries: Vec<ConversationSummary>,
    /// 配置
    config: SummaryConfig,
    /// 当前累计消息数
    message_count: u64,
    /// 上次摘要后的消息数
    messages_since_summary: u64,
    /// 待 LLM 处理的摘要文本（Rust 侧存储，Python 网关拉取并调用 LLM）
    pub pending_llm_text: Option<String>,
    /// sled 持久化数据库（可选）
    db: Option<sled::Db>,
}

impl ConversationSummarizer {
    pub fn new(config: SummaryConfig) -> Self {
        Self {
            summaries: Vec::new(),
            config,
            message_count: 0,
            messages_since_summary: 0,
            pending_llm_text: None,
            db: None,
        }
    }

    /// 从 sled 加载历史摘要
    pub fn open(db_path: &str, config: SummaryConfig) -> Self {
        let db = sled::open(db_path).ok();
        let mut summaries = Vec::new();
        if let Some(ref db) = db {
            for item in db.iter().flatten() {
                let (_, value) = item;
                if let Ok(summary) = bincode::deserialize::<ConversationSummary>(&value) {
                    summaries.push(summary);
                }
            }
            summaries.sort_by_key(|s| s.start_id);
            tracing::info!("Summarizer: loaded {} summaries from sled", summaries.len());
        }
        Self {
            summaries,
            config,
            message_count: 0,
            messages_since_summary: 0,
            pending_llm_text: None,
            db,
        }
    }

    /// 内存模式别名
    pub fn new_in_memory(config: SummaryConfig) -> Self {
        Self::new(config)
    }

    /// 将单条摘要持久化到 sled
    fn persist_one(&self, summary: &ConversationSummary) {
        if let Some(ref db) = self.db {
            let key = format!("summary_{}_{}", summary.start_id, summary.end_id);
            if let Ok(data) = bincode::serialize(summary) {
                let _ = db.insert(key.as_bytes(), data);
                let _ = db.flush();
            }
        }
    }

    /// 记录新消息，返回是否需要触发摘要
    pub fn record_message(&mut self) -> bool {
        self.message_count += 1;
        self.messages_since_summary += 1;

        self.message_count >= self.config.min_messages
            && self.messages_since_summary >= self.config.window_size as u64
    }

    /// 存储新生成的摘要
    pub fn store_summary(&mut self, text: String, start_id: u64, end_id: u64) {
        let summary = ConversationSummary {
            token_estimate: super::token_budget::TokenBudget::estimate(&text),
            text,
            start_id,
            end_id,
            created_at: chrono::Utc::now().timestamp(),
        };
        self.persist_one(&summary);
        self.summaries.push(summary);
        self.messages_since_summary = 0;
    }

    /// 获取所有摘要拼接的上下文（用于注入 Prompt）
    pub fn summary_context(&self) -> String {
        self.summary_context_bounded(usize::MAX)
    }

    /// 获取预算约束下的摘要上下文
    pub fn summary_context_bounded(&self, max_tokens: usize) -> String {
        if self.summaries.is_empty() {
            return String::new();
        }
        let mut ctx = String::from("[对话摘要]\n");
        let mut tokens_used = 0;
        for (i, s) in self.summaries.iter().enumerate() {
            if tokens_used + s.token_estimate > max_tokens {
                break;
            }
            let line = format!(
                "第{}段(消息#{}-#{}): {}\n",
                i + 1,
                s.start_id,
                s.end_id,
                s.text
            );
            tokens_used += s.token_estimate;
            ctx.push_str(&line);
        }
        ctx
    }

    /// 获取摘要总数
    pub fn summary_count(&self) -> usize {
        self.summaries.len()
    }

    /// 近期保留的消息轮数
    pub fn keep_recent(&self) -> usize {
        self.config.keep_recent
    }

    /// 窗口大小
    pub fn window_size(&self) -> usize {
        self.config.window_size
    }

    /// 总消息计数
    pub fn message_count(&self) -> u64 {
        self.message_count
    }

    /// 取出待 LLM 处理的摘要文本（消费式，取出后清空）
    pub fn take_pending_text(&mut self) -> Option<String> {
        self.pending_llm_text.take()
    }

    /// 提交 LLM 生成的摘要（替换最近一条抽取式摘要，或新增）
    pub fn submit_llm_summary(&mut self, text: String, start_id: u64, end_id: u64) {
        // 找到最近一条相同范围的摘要进行替换
        if let Some(last) = self.summaries.last_mut() {
            if last.end_id == end_id {
                last.text = text;
                last.token_estimate = super::token_budget::TokenBudget::estimate(&last.text);
                return;
            }
        }
        self.store_summary(text, start_id, end_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_no_trigger() {
        let mut s = ConversationSummarizer::new(SummaryConfig::default());
        for _ in 0..5 {
            let trigger = s.record_message();
            assert!(!trigger);
        }
    }

    #[test]
    fn test_trigger_after_window() {
        let mut s = ConversationSummarizer::new(SummaryConfig {
            min_messages: 5,
            window_size: 5,
            ..Default::default()
        });
        for _i in 0..5 {
            s.record_message();
        }
        // 第 6 条触发
        assert!(s.record_message());
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut s = ConversationSummarizer::new(SummaryConfig::default());
        s.store_summary("用户喜欢Rust和AI".into(), 1, 10);
        assert_eq!(s.summary_count(), 1);
        let ctx = s.summary_context();
        assert!(ctx.contains("Rust"));
    }

    #[test]
    fn test_summary_context_bounded() {
        let mut s = ConversationSummarizer::new(SummaryConfig::default());
        s.store_summary("摘要一：用户喜欢Rust编程".into(), 1, 5);
        s.store_summary("摘要二：用户在杭州工作，每天通勤上班".into(), 6, 10);
        // 预算只够第一条摘要（约 30 tokens）
        let bounded = s.summary_context_bounded(30);
        assert!(bounded.contains("Rust"), "第一条应在预算内");
        assert!(!bounded.contains("杭州"), "第二条应超出预算");
        // 无预算限制，两条都应包含
        let full = s.summary_context_bounded(usize::MAX);
        assert!(full.contains("Rust"));
        assert!(full.contains("杭州"));
    }

    #[test]
    fn test_sled_persistence_roundtrip() {
        let dir = format!("./target/test_summarizer_{}", std::process::id());
        let _ = std::fs::remove_dir_all(&dir);

        // 写入
        {
            let mut s = ConversationSummarizer::open(&dir, SummaryConfig::default());
            s.store_summary("持久化测试摘要".into(), 1, 10);
            assert_eq!(s.summary_count(), 1);
        }
        // 重新加载
        {
            let s = ConversationSummarizer::open(&dir, SummaryConfig::default());
            assert_eq!(s.summary_count(), 1);
            let ctx = s.summary_context();
            assert!(ctx.contains("持久化测试"));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
