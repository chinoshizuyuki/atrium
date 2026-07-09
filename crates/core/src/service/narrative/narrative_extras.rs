// SPDX-License-Identifier: MIT
//! 追问与关联 — 追问风格/多事项编织/语义关联
//! Followup & Association — Style/Weaver/Semantic

use super::*;

impl CoreService {
    pub fn followup_style_prompt_fragment(&self) -> String {
        let learner = self.curiosity.style_learner.lock();
        let summary = learner.insight_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!("[追问风格/FollowUpStyle] {}", summary)
        }
    }

    pub fn multi_item_weaver_prompt_fragment(&self, now: i64) -> String {
        if !self.followup_enabled {
            return String::new();
        }

        // 获取关系阶段与当前愉悦度 / Get relationship stage and current pleasure
        let (stage_name, pleasure): (String, f32) = {
            let rel = self.relationship.read();
            let emo = self.emotion.lock();
            (
                rel.current_stage().stage_name().to_string(),
                emo.current().pleasure,
            )
        };

        // 检查待追问事项（自动管理今日计数与冷却时间戳）
        // Check for pending follow-up items (auto-managed counters)
        // 数字生命的社交分寸感——内部自管 today_count 和 last_follow_up_ts
        // Digital life's social tact — auto-managed today_count and last_follow_up_ts
        let triggered = self
            .followup
            .lock()
            .check_for_follow_up_auto(now, &stage_name, pleasure);

        // 少于 2 项不需要编织 / Less than 2 items don't need weaving
        if triggered.len() < 2 {
            return String::new();
        }

        // 编织为自然语言 / Weave into natural language
        let woven = self.multi_item_weaver.weave(&triggered);
        if woven.is_empty() {
            String::new()
        } else {
            format!("[多事项编织/MultiItemWeaver] {}", woven)
        }
    }

    pub fn semantic_association_prompt_fragment(&self, msg: &str) -> String {
        let assoc = self.curiosity.association.lock();
        let hint = assoc.prompt_hint(msg);
        if hint.is_empty() {
            String::new()
        } else {
            format!("[语义关联/SemanticAssociation] {}", hint)
        }
    }

    // followup_style_learner_tick 已删除 / followup_style_learner_tick removed
    // 原方法仅做 learner.insight_summary() 纯查询并丢弃结果，属于空壳 tick。
    // Style learner 由 followup_style_learner_on_outcome 事件驱动更新。
    // Original method was pure-query with discarded result — a dead shell tick.
    // Style learner is event-driven via followup_style_learner_on_outcome.

    pub fn followup_style_learner_on_outcome(
        &self,
        category: atrium_memory::followup_tracker::FollowUpCategory,
        depth: atrium_memory::followup_tracker::FollowUpDepth,
        style: atrium_memory::followup_tracker::FollowUpStyle,
        reaction: atrium_memory::followup_tracker::UserReaction,
    ) {
        let mut learner = self.curiosity.style_learner.lock();
        learner.record_outcome(category, depth, style, reaction);
    }
} // impl CoreService
