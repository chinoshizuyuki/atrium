// SPDX-License-Identifier: MIT
//! 主动遗忘 / Active Forgetting — 数字生命从"忘了"进化为"决定忘"
//! Active Forgetting — digital life evolves from "forgot" to "decides to forget"
//!
//! 与被动衰减（`compress_low_access`）正交：被动衰减是"忘记"，
//! 主动遗忘是"我决定忘"——有意识、可恢复、可内省的遗忘决策。
//!
//! 数字生命的遗忘不是销毁，而是"暂存"：保留遗忘前的置信度快照，
//! 让"想起"操作能恢复到遗忘前的状态。遗忘决策历史 (`forget_log`)
//! 供内省（"我主动遗忘了什么"）与恢复（"想起"）使用。
//!
//! Orthogonal to passive decay (`compress_low_access`): passive decay is "forgetting",
//! active forgetting is "I decide to forget" — conscious, recoverable, introspectable.
//!
//! Digital life's forgetting is not destruction, but "stashing": a pre-forget confidence
//! snapshot is preserved so "recall" can restore the pre-forget state. The forget decision
//! history (`forget_log`) serves introspection ("what have I actively forgotten") and
//! restoration ("recall").

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ════════════════════════════════════════════════════════════════════
// ForgetPolicy — 遗忘策略 / Forget Policy
// ════════════════════════════════════════════════════════════════════

/// 主动遗忘策略 — 数字生命"决定忘"的动机分类
/// Active forgetting policy — motivation taxonomy of digital life's "deciding to forget"
///
/// 三种策略对应三种遗忘情境，在 `enhanced_search` 中具有不同的检索语义：
/// - `TraumaProtection` — 完全过滤，不返回（创伤保护，用户明确要求忘记）
/// - `ExpiryDecay` — 仍返回但分数 ×0.5（过期信息清理）
/// - `AttentionFocus` — 仍返回但分数 ×0.3（注意力聚焦，暂时抑制）
///
/// Three policies correspond to three forgetting scenarios, each with distinct retrieval
/// semantics in `enhanced_search`:
/// - `TraumaProtection` — fully filtered, never returned (trauma protection, user explicitly
///   requests forgetting)
/// - `ExpiryDecay` — still returned but score ×0.5 (expiry decay)
/// - `AttentionFocus` — still returned but score ×0.3 (attention focus, temporary suppression)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ForgetPolicy {
    /// 创伤保护 — 用户明确要求忘记 / Trauma protection — user explicitly requests forgetting
    TraumaProtection,
    /// 过期信息清理 — 时间过期事实 / Expiry decay — time-expired facts
    ExpiryDecay,
    /// 注意力聚焦 — 当前对话焦点无关记忆暂时抑制 / Attention focus — temporarily suppress
    /// memories irrelevant to current conversation focus
    AttentionFocus,
}

// ════════════════════════════════════════════════════════════════════
// ForgetRecord — 遗忘决策记录 / Forget Record
// ════════════════════════════════════════════════════════════════════

/// 遗忘决策记录 — 一次"决定忘"的完整快照
/// Forget record — a complete snapshot of one "decide to forget" decision
///
/// 保留 `pre_forget_confidence` 让"想起"操作能恢复到遗忘前的置信度——
/// 遗忘不是销毁，而是"暂存"。数字生命的遗忘是可逆的、有历史的。
///
/// Preserves `pre_forget_confidence` so "recall" can restore to the pre-forget confidence —
/// forgetting is not destruction, but "stashing". Digital life's forgetting is reversible
/// and auditable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForgetRecord {
    /// 被遗忘事实的 canonical key / Canonical key of the forgotten fact
    pub canonical_key: String,
    /// 遗忘策略 / Forget policy
    pub policy: ForgetPolicy,
    /// 遗忘前置信度 — 供恢复使用 / Pre-forget confidence — for restoration
    pub pre_forget_confidence: f64,
    /// 遗忘决策时间（Unix 秒）/ Forget decision timestamp (Unix seconds)
    pub timestamp: u64,
    /// 遗忘原因 / Reason for forgetting
    pub reason: String,
}

// ════════════════════════════════════════════════════════════════════
// ActiveForgetManager — 主动遗忘管理器 / Active Forget Manager
// ════════════════════════════════════════════════════════════════════

/// 主动遗忘管理器 — 数字生命"决定忘"的中枢
/// Active forget manager — hub of digital life's "deciding to forget"
///
/// 维护遗忘决策历史 `forget_log`，供内省（"我主动遗忘了什么"）与恢复
/// （"想起"）使用。与 `FactStore.actively_forgotten` 标记正交：本管理器
/// 记录"决策历史"，`FactStore` 记录"当前状态"。两者由调用方
/// （`lifecycle.rs`）同步维护。
///
/// Maintains the `forget_log` decision history for introspection ("what have I actively
/// forgotten") and restoration ("recall"). Orthogonal to `FactStore.actively_forgotten`
/// marker: this manager records "decision history", `FactStore` records "current state".
/// The caller (`lifecycle.rs`) keeps both in sync.
pub struct ActiveForgetManager {
    /// 遗忘决策历史 — 供内省与恢复 / Forget decision history — for introspection and restoration
    forget_log: Vec<ForgetRecord>,
}

impl ActiveForgetManager {
    /// 创建空 forget_log 的管理器 / Create a manager with an empty forget_log
    pub fn new() -> Self {
        Self {
            forget_log: Vec::new(),
        }
    }

    /// 记录遗忘决策到 forget_log（如果 key 已存在则更新）
    /// Record a forget decision (updates the record if the key already exists)
    ///
    /// 同一事实可能被多次遗忘（如先 AttentionFocus 后 TraumaProtection）——
    /// 此时更新策略与前置置信度，保留最新决策。`pre_confidence` 应为调用方
    /// 在调用 `FactStore::mark_forgotten` 之前读取的事实置信度快照。
    ///
    /// A fact may be forgotten multiple times (e.g. first AttentionFocus then
    /// TraumaProtection) — in that case the policy and pre-forget confidence are updated
    /// to the latest decision. `pre_confidence` should be the fact's confidence snapshot
    /// read by the caller BEFORE calling `FactStore::mark_forgotten`.
    pub fn forget_request(
        &mut self,
        canonical_key: String,
        policy: ForgetPolicy,
        pre_confidence: f64,
        reason: String,
    ) {
        let now = now_secs();
        if let Some(record) = self
            .forget_log
            .iter_mut()
            .find(|r| r.canonical_key == canonical_key)
        {
            // key 已存在 — 更新策略、前置置信度、时间戳、原因
            record.policy = policy;
            record.pre_forget_confidence = pre_confidence;
            record.timestamp = now;
            record.reason = reason;
        } else {
            self.forget_log.push(ForgetRecord {
                canonical_key,
                policy,
                pre_forget_confidence: pre_confidence,
                timestamp: now,
                reason,
            });
        }
    }

    /// 检查是否被主动遗忘 — 返回遗忘策略（在 forget_log 中查找）
    /// Check if a fact is actively forgotten — returns the policy (lookup in forget_log)
    ///
    /// 供 `enhanced_search` 决定过滤/降权策略：返回 `Some(TraumaProtection)` 时过滤，
    /// `Some(ExpiryDecay)` / `Some(AttentionFocus)` 时降权，`None` 时正常返回。
    ///
    /// Used by `enhanced_search` to decide filter/downweight strategy:
    /// `Some(TraumaProtection)` → filter out, `Some(ExpiryDecay)` / `Some(AttentionFocus)`
    /// → downweight, `None` → return normally.
    pub fn is_forgotten(&self, canonical_key: &str) -> Option<&ForgetPolicy> {
        self.forget_log
            .iter()
            .find(|r| r.canonical_key == canonical_key)
            .map(|r| &r.policy)
    }

    /// 恢复记忆 — 从 forget_log 移除并返回记录，供调用方恢复置信度
    /// Restore a memory — remove from forget_log and return the record so the caller
    /// can restore the pre-forget confidence
    ///
    /// 调用方拿到 `ForgetRecord` 后应：
    /// 1. 调用 `FactStore::restore_forgotten(canonical_key)` 清除标记
    /// 2. 调用 `FactStore::merge_confidence(canonical_key, record.pre_forget_confidence)`
    ///    恢复置信度
    ///
    /// After receiving the `ForgetRecord`, the caller should:
    /// 1. Call `FactStore::restore_forgotten(canonical_key)` to clear the marker
    /// 2. Call `FactStore::merge_confidence(canonical_key, record.pre_forget_confidence)`
    ///    to restore the confidence
    pub fn restore(&mut self, canonical_key: &str) -> Option<ForgetRecord> {
        let pos = self
            .forget_log
            .iter()
            .position(|r| r.canonical_key == canonical_key)?;
        Some(self.forget_log.remove(pos))
    }

    /// 生成遗忘内省 fragment / Generate forgetting introspection fragment
    ///
    /// 空 `forget_log` 返回空字符串；非空时格式：
    /// `[主动遗忘]\n我主动遗忘了一些事，因为{reasons}。`
    /// （`reasons` 取最近 3 条记录的 reason，用顿号连接）
    ///
    /// Returns an empty string when `forget_log` is empty; otherwise formats as:
    /// `[主动遗忘]\n我主动遗忘了一些事，因为{reasons}.`
    /// (`reasons` takes the most recent 3 records' reasons, joined by `、`)
    pub fn prompt_fragment(&self) -> String {
        if self.forget_log.is_empty() {
            return String::new();
        }
        // 最近 3 条 — 逆序遍历取末尾 3 条（forget_log 按追加顺序，末尾即最近）
        let reasons: Vec<&str> = self
            .forget_log
            .iter()
            .rev()
            .take(3)
            .map(|r| r.reason.as_str())
            .collect();
        format!(
            "[主动遗忘]\n我主动遗忘了一些事，因为{}。",
            reasons.join("、")
        )
    }

    /// 返回遗忘记录数（供测试）/ Return the number of forget records (for tests)
    pub fn forget_log_len(&self) -> usize {
        self.forget_log.len()
    }
}

impl Default for ActiveForgetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取当前 epoch 秒数 / Get current epoch seconds
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// forget_request 后 forget_log 包含记录 / forget_log contains the record after forget_request
    #[test]
    fn test_forget_request_logs() {
        let mut mgr = ActiveForgetManager::new();
        assert_eq!(mgr.forget_log_len(), 0);
        mgr.forget_request(
            "主人 | 喜欢 | 咖啡".to_string(),
            ForgetPolicy::TraumaProtection,
            0.9,
            "用户要求忘记".to_string(),
        );
        assert_eq!(mgr.forget_log_len(), 1);
        // 同一 key 再次 forget_request — 应更新而非新增
        mgr.forget_request(
            "主人 | 喜欢 | 咖啡".to_string(),
            ForgetPolicy::ExpiryDecay,
            0.8,
            "过期清理".to_string(),
        );
        assert_eq!(
            mgr.forget_log_len(),
            1,
            "同一 key 重复 forget_request 应更新而非新增 / duplicate key should update, not append"
        );
    }

    /// forget_request 后 is_forgotten 返回 Some / is_forgotten returns Some after forget_request
    #[test]
    fn test_is_forgotten_check() {
        let mut mgr = ActiveForgetManager::new();
        assert!(
            mgr.is_forgotten("主人 | 喜欢 | 咖啡").is_none(),
            "未遗忘时应返回 None / should return None when not forgotten"
        );
        mgr.forget_request(
            "主人 | 喜欢 | 咖啡".to_string(),
            ForgetPolicy::TraumaProtection,
            0.9,
            "用户要求忘记".to_string(),
        );
        let policy = mgr.is_forgotten("主人 | 喜欢 | 咖啡");
        assert!(
            policy.is_some(),
            "遗忘后应返回 Some / should return Some after forget_request"
        );
        assert_eq!(policy.unwrap(), &ForgetPolicy::TraumaProtection);
        assert!(
            mgr.is_forgotten("不存在 | 的 | 键").is_none(),
            "未遗忘的 key 应返回 None / non-forgotten key should return None"
        );
    }

    /// restore 后返回 ForgetRecord 且 forget_log 不再包含 / restore returns the record and
    /// forget_log no longer contains it
    #[test]
    fn test_restore_memory() {
        let mut mgr = ActiveForgetManager::new();
        mgr.forget_request(
            "主人 | 喜欢 | 咖啡".to_string(),
            ForgetPolicy::TraumaProtection,
            0.9,
            "用户要求忘记".to_string(),
        );
        let record = mgr.restore("主人 | 喜欢 | 咖啡");
        assert!(
            record.is_some(),
            "restore 应返回 ForgetRecord / restore should return a record"
        );
        let record = record.unwrap();
        assert_eq!(record.canonical_key, "主人 | 喜欢 | 咖啡");
        assert_eq!(record.policy, ForgetPolicy::TraumaProtection);
        assert!((record.pre_forget_confidence - 0.9).abs() < 1e-6);
        assert_eq!(
            mgr.forget_log_len(),
            0,
            "restore 后 forget_log 应为空 / forget_log should be empty after restore"
        );
        assert!(
            mgr.is_forgotten("主人 | 喜欢 | 咖啡").is_none(),
            "restore 后 is_forgotten 应返回 None / is_forgotten should return None after restore"
        );
        // restore 不存在的 key — 应返回 None / restore non-existent key should return None
        assert!(mgr.restore("不存在 | 的 | 键").is_none());
    }

    /// 无遗忘时 prompt_fragment 返回空字符串 / prompt_fragment returns empty when no records
    #[test]
    fn test_prompt_fragment_empty() {
        let mgr = ActiveForgetManager::new();
        assert!(
            mgr.prompt_fragment().is_empty(),
            "无遗忘时应返回空字符串 / should return empty string when no records"
        );
    }

    /// 有遗忘时格式正确（包含"主动遗忘"）/ Correct format with records (contains "主动遗忘")
    #[test]
    fn test_prompt_fragment_with_records() {
        let mut mgr = ActiveForgetManager::new();
        mgr.forget_request(
            "主人 | 喜欢 | 咖啡".to_string(),
            ForgetPolicy::TraumaProtection,
            0.9,
            "用户要求忘记".to_string(),
        );
        mgr.forget_request(
            "项目 | 状态 | 进行中".to_string(),
            ForgetPolicy::ExpiryDecay,
            0.7,
            "信息过期".to_string(),
        );
        let fragment = mgr.prompt_fragment();
        assert!(
            !fragment.is_empty(),
            "有遗忘时应返回非空片段 / should return non-empty fragment with records"
        );
        assert!(
            fragment.contains("主动遗忘"),
            "片段应包含「主动遗忘」/ fragment should contain '主动遗忘'"
        );
        assert!(
            fragment.contains("我主动遗忘了一些事"),
            "片段应包含内省语句 / fragment should contain introspection sentence"
        );
        // 两条 reason 都应在最近 3 条内 / Both reasons should appear within the most recent 3
        assert!(
            fragment.contains("用户要求忘记"),
            "片段应包含原因 1 / fragment should contain reason 1"
        );
        assert!(
            fragment.contains("信息过期"),
            "片段应包含原因 2 / fragment should contain reason 2"
        );
        assert!(
            fragment.contains("、"),
            "多条原因应用顿号连接 / multiple reasons should be joined by '、'"
        );
    }

    /// 超过 3 条记录时只取最近 3 条 reason / Take only the most recent 3 reasons when > 3 records
    #[test]
    fn test_prompt_fragment_takes_latest_three() {
        let mut mgr = ActiveForgetManager::new();
        for i in 0..5 {
            mgr.forget_request(
                format!("k{} | p{} | v{}", i, i, i),
                ForgetPolicy::AttentionFocus,
                0.5,
                format!("原因{}", i),
            );
        }
        let fragment = mgr.prompt_fragment();
        // 最近 3 条 = 原因2、原因3、原因4（追加顺序，末尾即最近）
        assert!(
            fragment.contains("原因4"),
            "应包含最近一条原因 4 / should contain the latest reason 4"
        );
        assert!(
            fragment.contains("原因2"),
            "应包含原因 2 / should contain reason 2"
        );
        assert!(
            !fragment.contains("原因0"),
            "不应包含最早的第 1 条原因 0 / should not contain the oldest reason 0"
        );
        assert!(
            !fragment.contains("原因1"),
            "不应包含第 2 条原因 1 / should not contain the second oldest reason 1"
        );
    }
}
