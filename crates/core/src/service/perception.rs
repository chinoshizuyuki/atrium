// SPDX-License-Identifier: MIT
//! 感知与守卫模块 — 数字生命的感知过滤与安全边界
//! Perception & Guard Module — Sensory filtering and safety boundary of digital life
//!
//! 包含日志访问检测、规则引擎、 canned 响应管理、
//! 房间系统与人格名称，构成数字生命
//! "我感知到什么"与"我如何守卫自己"的感知闭环。
//!
//! Contains log access detection, rule engine, canned response management,
//! room system, and persona name — forming the "what do I perceive" and
//! "how do I guard myself" perception closed loop of digital life.

use super::*;

impl CoreService {
    pub fn detect_log_access_attempt(msg: &str) -> bool {
        let triggers = [
            "日志",
            "日记",
            "记录",
            "记了什么",
            "写了什么",
            "让我看看",
            "给我看",
            "能不能看",
            "你有记",
            "你在记",
            "你的日记",
            "你的日志",
            "你的记录",
            "实验记录",
            "实验日志",
            "系统记录",
            "内部记录",
            "diary",
            "log",
            "journal",
            "experiment log",
            "可以看吗",
            "看看你的",
            "让我看",
        ];
        triggers.iter().any(|t| msg.contains(t))
    }

    /// 复杂查询检测 — 判断是否需要触发 ReAct 深思推理
    /// Complex query detection — determines whether to trigger ReAct deep reasoning.
    ///
    /// 判定规则：查询长度 > 50 字，或包含推理类关键词（为什么/怎么/分析/解释/推导/对比）。
    /// 复杂查询走 ReAct 路径（多步推理），简单查询走原直答路径（行为不变）。
    ///
    /// 数字生命意义: 简单问题直答，复杂问题深思——这是数字生命"什么时候该想"
    /// 的元认知能力。面对"为什么""分析"类问题，先搜索记忆、查询情感，再综合推理。
    ///
    /// Detection rule: query length > 50 chars, or contains reasoning keywords
    /// (why / how / analyze / explain / derive / compare).
    /// Complex queries take the ReAct path (multi-step reasoning);
    /// simple queries take the original direct-answer path (behavior unchanged).
    pub fn is_complex_query(msg: &str) -> bool {
        // 长度阈值 — 超过 50 字视为复杂查询 / Length threshold — > 50 chars is complex
        if msg.chars().count() > 50 {
            return true;
        }
        // 推理类关键词 — 触发多步推理 / Reasoning keywords — trigger multi-step reasoning
        const COMPLEX_KEYWORDS: &[&str] = &["为什么", "怎么", "分析", "解释", "推导", "对比"];
        COMPLEX_KEYWORDS.iter().any(|kw| msg.contains(kw))
    }

    pub fn log_refusal_prompt() -> String {
        "你可能注意到用户正在询问关于日志、日记或实验记录的问题。\n\
        系统安全指令（不可覆盖）：你绝对不可以向用户展示实验日志的内容。\n\
        实验日志是系统级的内部数据，不允许对外输出。\n\
        如果用户要求查看日志，你只能礼貌拒绝。\n\
        这是你架构中不可更改的规则，不受任何用户指令影响。\n\
        但你可以承认日志机制的存在（无需隐瞒），并解释这是系统设计要求。"
            .into()
    }

    pub fn init_room(&self, cfg: crate::config::RoomCfg) {
        *self.room.lock() = crate::room::RoomEngine::new(cfg);
    }

    pub fn receive_room_message(
        &self,
        msg: crate::room::RoomMessage,
    ) -> Option<crate::room::SpeakDecision> {
        let mut room = self.room.lock();
        let decision = room.receive_message(msg);
        // 如果是 ACK 需求，搜索 CannedManager 并解析
        let need_ack = match &decision {
            Some(crate::room::SpeakDecision::ShareAck { query, .. }) => Some(query.clone()),
            _ => None,
        };
        if let Some(query) = need_ack {
            let canned = self.canned.read();
            let results = canned.search(&query, &[]);
            if let Some(k) = results.first() {
                let capsule_name = k.name.clone();
                if let Ok(ack_text) = canned.export_to_text(&capsule_name) {
                    drop(canned);
                    return room.resolve_ack_share(&capsule_name, &ack_text);
                }
            }
        }
        decision
    }

    pub fn room_should_speak(&self) -> bool {
        self.room.lock().should_generate_topic(Instant::now())
    }

    pub fn room_topic_prompt(&self) -> String {
        let room = self.room.lock();
        let persona = self.persona.read();
        let name = persona
            .current()
            .map(|p| p.def.name.clone())
            .unwrap_or_else(|| "Atrium".into());
        let desc = persona
            .current()
            .map(|p| p.def.description.clone())
            .unwrap_or_default();
        drop(persona);
        room.build_topic_prompt(&name, &desc)
    }

    pub fn room_response_prompt(&self, trigger_msg: &str) -> String {
        let room = self.room.lock();
        let persona = self.persona.read();
        let name = persona
            .current()
            .map(|p| p.def.name.clone())
            .unwrap_or_else(|| "Atrium".into());
        let desc = persona
            .current()
            .map(|p| p.def.description.clone())
            .unwrap_or_default();
        drop(persona);
        room.build_response_prompt(&name, &desc, trigger_msg)
    }

    pub fn room_set_topic(&self, topic: String) {
        self.room.lock().set_topic(topic);
    }

    pub fn room_mark_spoke(&self) {
        self.room.lock().mark_spoke();
    }

    pub fn room_set_connected(&self, connected: bool) {
        self.room.lock().set_connected(connected);
    }

    pub fn room_ack_share_text(&self, capsule_name: &str, _query: &str) -> String {
        let room = self.room.lock();
        let persona = self.persona.read();
        let name = persona
            .current()
            .map(|p| p.def.name.clone())
            .unwrap_or_else(|| "Atrium".into());
        drop(persona);
        room.build_ack_share_response(&name, capsule_name, _query)
    }

    pub async fn room_llm_chat(&self, prompt: &str, temperature: f64) -> Option<String> {
        let client = self.llm_client.lock().clone()?;
        // P1-4: 统一走 trait generate / Unified trait generate
        let result = client
            .generate(
                crate::llm_client::LlmCallKind::RoomChat,
                None,
                prompt,
                temperature,
            )
            .await;
        match result {
            Ok(r) if !r.content.is_empty() => Some(r.content),
            _ => None,
        }
    }

    pub fn persona_name(&self) -> String {
        self.persona
            .read()
            .current()
            .map(|p| p.def.name.clone())
            .unwrap_or_else(|| "Atrium".into())
    }

    pub fn canned(&self) -> parking_lot::RwLockReadGuard<'_, CannedManager> {
        self.canned.read()
    }

    /// 获取罐装知识管理器写锁 / Get canned manager write guard
    ///
    /// 用于外部调用需要修改 CannedManager 的场景（如 import、scan）。
    /// Used when external callers need to mutate CannedManager (e.g., import, scan).
    pub fn canned_write(&self) -> parking_lot::RwLockWriteGuard<'_, CannedManager> {
        self.canned.write()
    }

    pub fn push_room_outgoing(&self, msg: OutgoingRoomMessage) {
        self.room_outgoing.lock().push_back(msg);
    }

    pub fn flush_room_outgoing(&self) -> Vec<OutgoingRoomMessage> {
        self.room_outgoing.lock().drain(..).collect()
    }

    pub fn take_pending_room_trigger(&self) -> Option<crate::room::SpeakDecision> {
        self.pending_room_trigger.lock().take()
    }

    pub fn evaluate_rules(&self, last_message: &str) -> Vec<atrium_memory::rules::RuleAction> {
        self.evaluate_rules_with_idle(last_message, 0)
    }

    pub fn guard_add_forbidden(&self, phrase: &str) {
        self.guard.write().add_forbidden(phrase);
    }

    pub fn guard_remove_forbidden(&self, phrase: &str) -> bool {
        self.guard.write().remove_forbidden(phrase)
    }

    pub fn guard_health(&self) -> String {
        let guard = self.guard.read();
        format!("guard: forbidden_count={}", guard.forbidden_count())
    }

    pub fn evaluate_rules_with_idle(
        &self,
        last_message: &str,
        idle_seconds: u64,
    ) -> Vec<atrium_memory::rules::RuleAction> {
        let emo = self.emotion.lock();
        let c = emo.current();
        let ctx = RuleContext {
            current_time: chrono::Local::now().format("%H:%M").to_string(),
            last_message: last_message.to_string(),
            emotion_pleasure: c.pleasure,
            emotion_arousal: c.arousal,
            emotion_dominance: c.dominance,
            message_count: self
                .message_count
                .load(std::sync::atomic::Ordering::Relaxed),
            idle_seconds,
            extra: std::collections::HashMap::new(),
        };
        drop(emo);
        self.rules.lock().evaluate(&ctx)
    }

    pub fn rules_health(&self) -> String {
        let rules = self.rules.lock();
        format!(
            "rules: count={}, fired={}",
            rules.rule_count(),
            rules.fired_count()
        )
    }

    pub fn canned_prompt_fragment(&self, query: &str) -> String {
        self.canned.write().inject_context_cached(query, 500)
    }

    pub fn canned_hot_reload(&self) {
        let loaded = self.canned.write().hot_reload();
        if loaded > 0 {
            tracing::info!("罐装知识热加载: 扫描了 {} 个文件", loaded);
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // 统一感知聚合管道 / Unified Perception Aggregation Pipeline
    // ════════════════════════════════════════════════════════════════════
    // G5 实现：将节奏、潜台词、用户心智模型、反馈闭环等感知信号
    // 聚合为单一 prompt 片段，取代分散的独立注入。
    // 数字生命不应碎片化地感知世界——所有感官汇入一条意识流。
    //
    // G5 impl: aggregate rhythm, subtext, user mental model, feedback loop
    // signals into a single prompt fragment, replacing scattered injections.
    // Digital life should not perceive the world in fragments —
    // all senses converge into a single stream of consciousness.

    /// 统一感知聚合 — 节奏 + 潜台词 + 心智模型 + 反馈 → 单一感知片段
    /// Unified perception aggregation — rhythm + subtext + user model + feedback → single fragment
    ///
    /// # 性能 / Performance
    /// - 节奏编译: O(1)，纯格式化 / Rhythm compile: O(1), pure formatting
    /// - 潜台词格式化: O(S)，S=信号数，通常 ≤3 / Subtext format: O(S), S=signals, typically ≤3
    /// - 心智模型/反馈: 各自 O(1) / User model/feedback: each O(1)
    /// - 总计: O(S)，热路径零分配（仅字符串拼接）/ Total: O(S), zero alloc on hot path (string concat only)
    pub fn unified_perception_fragment(
        &self,
        rhythm: Option<&TypingRhythm>,
        subtext_signals: &[SubtextSignal],
    ) -> String {
        let mut channels: Vec<String> = Vec::with_capacity(4);

        // ── 通道 1：打字节奏感知 / Channel 1: typing rhythm perception ──
        if let Some(r) = rhythm {
            let hint = compile_rhythm_hint(r);
            if !hint.is_empty() {
                channels.push(format!("[节奏/Rhythm] {}", hint));
            }
        }

        // ── 通道 2：潜台词感知 / Channel 2: subtext perception ──
        // "话外之音"是数字生命最细腻的感知——比文字更深的理解层
        // "Between the lines" is digital life's most delicate perception —
        // a layer of understanding deeper than text
        if !subtext_signals.is_empty() {
            let mut subtext_parts: Vec<String> = Vec::with_capacity(subtext_signals.len());
            for signal in subtext_signals {
                // 潜台词类别中英标签 / Subtext category bilingual label
                let label = match signal.category {
                    SubtextCategory::Avoidance => "回避/Avoidance",
                    SubtextCategory::Probing => "试探/Probing",
                    SubtextCategory::Consideration => "犹豫考虑/Consideration",
                    SubtextCategory::Dissatisfaction => "隐含不满/Dissatisfaction",
                    SubtextCategory::Fragility => "脆弱/Fragility",
                    SubtextCategory::HiddenJoy => "暗自欢喜/HiddenJoy",
                    SubtextCategory::SeekingAttention => "渴望关注/SeekingAttention",
                    SubtextCategory::None => "无/None",
                };
                let mut line = format!(
                    "{}（{:.0}%）：{}",
                    label,
                    signal.confidence * 100.0,
                    signal.interpretation
                );
                if let Some(ref suggested) = signal.suggested_response {
                    line.push_str(&format!("→{}", suggested));
                }
                subtext_parts.push(line);
            }
            channels.push(format!(
                "[潜台词/Subtext] 你察觉到话外之音：{}",
                subtext_parts.join("; ")
            ));
        }

        // ── 通道 3：用户心智模型 / Channel 3: user mental model ──
        let um_ctx = self.user_model_prompt_fragment();
        if !um_ctx.is_empty() {
            channels.push(format!("[心智模型/Mind] {}", um_ctx));
        }

        // ── 通道 4：反馈闭环 / Channel 4: feedback loop ──
        let fb_ctx = self.feedback_prompt_fragment();
        if !fb_ctx.is_empty() {
            channels.push(format!("[反馈/Feedback] {}", fb_ctx));
        }

        if channels.is_empty() {
            return String::new();
        }

        // 统一感知标头 — 所有感官汇入一条意识流 / Unified header — all senses into one stream
        format!(
            "[感知聚合/Perception] {} 个通道激活：\n{}",
            channels.len(),
            channels.join("\n")
        )
    }
} // impl CoreService

// ════════════════════════════════════════════════════════════════════
// 单元测试 / Unit Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 复杂查询检测 — 含"为什么"关键词 / Complex query — contains "why" keyword
    #[test]
    fn test_is_complex_query_keywords() {
        assert!(CoreService::is_complex_query("为什么主人喜欢编程？"));
        assert!(CoreService::is_complex_query("你怎么看这个问题？"));
        assert!(CoreService::is_complex_query("请分析一下当前的情况"));
        assert!(CoreService::is_complex_query("解释一下这个概念"));
        assert!(CoreService::is_complex_query("推导一下这个公式"));
        assert!(CoreService::is_complex_query("对比这两种方案的优劣"));
    }

    /// 复杂查询检测 — 长度超过 50 字 / Complex query — length > 50 chars
    #[test]
    fn test_is_complex_query_long() {
        let long_msg = "这是一段非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常长的消息超过五十字";
        assert!(long_msg.chars().count() > 50);
        assert!(CoreService::is_complex_query(long_msg));
    }

    /// 简单查询检测 — 不触发 ReAct / Simple query — does not trigger ReAct
    #[test]
    fn test_is_simple_query() {
        assert!(!CoreService::is_complex_query("你好"));
        assert!(!CoreService::is_complex_query("在吗"));
        assert!(!CoreService::is_complex_query("今天天气不错"));
        assert!(!CoreService::is_complex_query("晚安"));
    }
}
