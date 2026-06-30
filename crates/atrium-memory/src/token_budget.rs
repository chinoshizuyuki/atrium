// SPDX-License-Identifier: MIT
//! Token 预算管理器
//!
//! 实时估算 token 使用量，按四层上下文策略分配预算：
//! 近程上下文 60% | 语义检索 25% | 摘要 10% | 关键信息 5%
//! TokenBudget — Token budget controller.
//!
//! Real-time tracking of token usage with strategic budget allocation:
//! Conversation history 60% | Canned knowledge 25% | Summaries 10% | Key facts 5%

/// Token 预算分配策略
#[derive(Debug, Clone)]
pub struct TokenBudget {
    /// 总 token 上限（如 128K = 131072）
    total_budget: usize,
    /// 当前已使用的 token 估算
    used: usize,
    /// 各层分配比例
    allocation: BudgetAllocation,
}

#[derive(Debug, Clone, Copy)]
pub struct BudgetAllocation {
    /// 近程上下文（最近 N 轮原文）
    pub recent_pct: f64,
    /// 语义检索记忆
    pub retrieval_pct: f64,
    /// 对话摘要
    pub summary_pct: f64,
    /// 关键信息缓存
    pub key_fact_pct: f64,
}

impl Default for BudgetAllocation {
    fn default() -> Self {
        Self {
            recent_pct: 0.60,
            retrieval_pct: 0.25,
            summary_pct: 0.10,
            key_fact_pct: 0.05,
        }
    }
}

impl TokenBudget {
    /// 创建新的预算管理器
    /// `model_limit` 为模型最大上下文长度（token 数）
    pub fn new(model_limit: usize) -> Self {
        Self {
            total_budget: model_limit,
            used: 0,
            allocation: BudgetAllocation::default(),
        }
    }

    /// 估算文本的 token 数
    /// 中文约 2 token/字，英文约 1.3 token/词（4 字符≈1 词）
    pub fn estimate(text: &str) -> usize {
        let mut token_count = 0usize;
        let mut in_cjk = false;
        let mut cjk_run = 0usize;

        for ch in text.chars() {
            if is_cjk(ch) {
                in_cjk = true;
                cjk_run += 1;
            } else {
                if in_cjk {
                    // CJK 运行结束，中文约 2 token/字
                    token_count += cjk_run * 2;
                    cjk_run = 0;
                    in_cjk = false;
                }
                // 英文按词估算，约 4 字符/词，1.3 token/词
                if ch.is_whitespace() || ch.is_ascii_punctuation() {
                    // 分隔符不占 token
                } else {
                    token_count += 1;
                }
            }
        }
        if in_cjk {
            token_count += cjk_run * 2;
        }

        token_count.max(1)
    }

    /// 各层 token 预算
    pub fn recent_budget(&self) -> usize {
        (self.total_budget as f64 * self.allocation.recent_pct) as usize
    }

    pub fn retrieval_budget(&self) -> usize {
        (self.total_budget as f64 * self.allocation.retrieval_pct) as usize
    }

    pub fn summary_budget(&self) -> usize {
        (self.total_budget as f64 * self.allocation.summary_pct) as usize
    }

    pub fn key_fact_budget(&self) -> usize {
        (self.total_budget as f64 * self.allocation.key_fact_pct) as usize
    }

    /// 剩余可用 token
    pub fn remaining(&self) -> usize {
        self.total_budget.saturating_sub(self.used)
    }

    /// 使用率
    pub fn usage_ratio(&self) -> f64 {
        if self.total_budget == 0 {
            return 0.0;
        }
        self.used as f64 / self.total_budget as f64
    }

    /// 是否需要触发摘要（使用率超 70%）
    pub fn should_summarize(&self) -> bool {
        self.usage_ratio() > 0.70
    }

    /// 更新已用 token 数
    pub fn update_used(&mut self, tokens: usize) {
        self.used = tokens;
    }

    pub fn total_budget(&self) -> usize {
        self.total_budget
    }

    pub fn reset(&mut self) {
        self.used = 0;
    }

    /// 为四层上下文构建综合预算报告
    pub fn report(&self) -> String {
        format!(
            "Token:{}/{} recent({}):{} retrieval:{} summary:{} key:{}",
            self.used,
            self.total_budget,
            self.allocation.recent_pct * 100.0,
            self.recent_budget(),
            self.retrieval_budget(),
            self.summary_budget(),
            self.key_fact_budget(),
        )
    }
}

/// 判断字符是否为 CJK（中日韩统一表意文字）
fn is_cjk(ch: char) -> bool {
    matches!(ch,
    '\u{4E00}'..='\u{9FFF}' // CJK 统一汉字
    | '\u{3400}'..='\u{4DBF}' // CJK 扩展 A
    | '\u{20000}'..='\u{2A6DF}' // CJK 扩展 B
    | '\u{3040}'..='\u{309F}' // 平假名
    | '\u{30A0}'..='\u{30FF}' // 片假名
    | '\u{AC00}'..='\u{D7AF}' // 韩文
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_chinese() {
        let tokens = TokenBudget::estimate("我喜欢编程");
        // 4 个 CJK 字 × 2 = 8 tokens
        assert!((6..=12).contains(&tokens));
    }

    #[test]
    fn test_estimate_english() {
        let tokens = TokenBudget::estimate("I love programming");
        assert!(tokens > 1);
    }

    #[test]
    fn test_estimate_mixed() {
        let tokens = TokenBudget::estimate("我喜欢Rust编程");
        // 4 CJK + "Rust" ≈ 8+1 = 9
        assert!(tokens >= 8);
    }

    #[test]
    fn test_budget_allocation() {
        let budget = TokenBudget::new(100_000);
        assert!(budget.recent_budget() > budget.retrieval_budget());
        assert!(budget.retrieval_budget() > budget.summary_budget());
    }

    #[test]
    fn test_should_summarize() {
        let mut budget = TokenBudget::new(100_000);
        assert!(!budget.should_summarize());
        budget.update_used(80_000);
        assert!(budget.should_summarize());
    }

    #[test]
    fn test_usage_ratio() {
        let mut budget = TokenBudget::new(100);
        budget.update_used(50);
        assert!((budget.usage_ratio() - 0.5).abs() < 0.01);
    }
}
