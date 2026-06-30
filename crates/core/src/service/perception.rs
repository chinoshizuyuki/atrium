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
            let canned = self.canned.lock();
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
        let persona = self.persona.lock();
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
        let persona = self.persona.lock();
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
        let persona = self.persona.lock();
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
            .lock()
            .current()
            .map(|p| p.def.name.clone())
            .unwrap_or_else(|| "Atrium".into())
    }

    pub fn canned(&self) -> parking_lot::MutexGuard<'_, CannedManager> {
        self.canned.lock()
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
        self.guard.lock().add_forbidden(phrase);
    }

    pub fn guard_remove_forbidden(&self, phrase: &str) -> bool {
        self.guard.lock().remove_forbidden(phrase)
    }

    pub fn guard_health(&self) -> String {
        let guard = self.guard.lock();
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
        self.canned.lock().inject_context_cached(query, 500)
    }

    pub fn canned_hot_reload(&self) {
        let loaded = self.canned.lock().hot_reload();
        if loaded > 0 {
            tracing::info!("罐装知识热加载: 扫描了 {} 个文件", loaded);
        }
    }
} // impl CoreService
