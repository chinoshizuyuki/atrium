// SPDX-License-Identifier: MIT
#![allow(unknown_lints)] // Rust 1.86 不认识下方 lint
#![allow(clippy::manual_is_multiple_of)] // CI 使用 Rust 1.86，is_multiple_of 尚未稳定

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{error, info};

use crate::service::CoreService;
use atrium_bridge::protocol::{BridgeConfig, BridgeEvent, EmotionState as ProtocolEmotion};
use atrium_bridge::Bridge;
use std::sync::Arc;

use crate::config::Config;
use crate::metrics;
use crate::proactive::ProactiveDecision;
use atrium_plugin::PluginManager;
use chrono::Timelike;

pub struct Scheduler {
    event_tx: flume::Sender<BridgeEvent>,
    event_rx: flume::Receiver<BridgeEvent>,
    bridge: Option<Bridge>,
    core_service: Arc<crate::service::CoreService>,
    config: Config,
    // 调度器启动时间戳 — 保留供未来健康检查 / 运行时长观测
    // Scheduler start timestamp — kept for future health check / uptime observation
    #[allow(dead_code)]
    started_at: Instant,
    event_count: AtomicU64,
    decay_ticks: AtomicU64,
    /// 最后一次收到用户消息的时间 / Last time user message was received
    last_user_message_at: parking_lot::Mutex<Option<Instant>>,
    /// 最后一次收到用户消息的 Unix 时间戳 / Last user message Unix timestamp
    last_user_message_ts: parking_lot::Mutex<Option<i64>>,
    /// 用户消息计数（用于触发智能提取）/ Message count (triggers intelligence extraction)
    intelligence_msg_count: AtomicU64,
    /// 插件管理器 / Plugin manager
    plugin_manager: PluginManager,
}

impl Scheduler {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = flume::unbounded();
        let context_limit = config.memory.context_limit;
        let service = CoreService::new_with_config(
            context_limit,
            &config.emotion,
            &config.user_model,
            &config.feedback,
            &config.proactive,
            &config.perception,
            &config.consolidation,
            &config.empathy,
            &config.ack_learning,
            &config.longing,
            &config.maturity,
            &config.inner_monologue,
            &config.expression,
            &config.followup,
            &config.narrative,
            &config.conflict,
            &config.irrationality,
            &config.ritual,
            &config.vulnerability,
            &config.emotional_demand,
            &config.self_care,
            &config.imperfection,
            &config.physical_presence,
        );
        Self {
            event_tx: tx,
            event_rx: rx,
            bridge: None,
            core_service: Arc::new(service),
            config,
            started_at: Instant::now(),
            event_count: AtomicU64::new(0),
            decay_ticks: AtomicU64::new(0),
            last_user_message_at: parking_lot::Mutex::new(None),
            last_user_message_ts: parking_lot::Mutex::new(None),
            intelligence_msg_count: AtomicU64::new(0),
            plugin_manager: PluginManager::new_static_only(),
        }
    }

    pub async fn start_all(&mut self) {
        info!("调度器启动...");

        // 初始化 LLM 客户端 / Initialize LLM client
        // P1-4: 构造 HttpLlmClient → Arc<dyn LlmClient> trait 对象 / Construct HttpLlmClient → Arc<dyn LlmClient> trait object
        let llm_client = crate::llm_client::HttpLlmClient::new(self.config.llm.clone());
        let trait_client: std::sync::Arc<dyn atrium_memory::llm_client::LlmClient> =
            std::sync::Arc::new(llm_client);
        self.core_service.set_llm_client(trait_client);
        info!("LLM 客户端已初始化");

        // 初始化 Room 群聊 / Initialize Room group chat
        self.core_service.init_room(self.config.room.clone());
        if self.config.room.enabled {
            info!("Room 群聊引擎已启用: room={}", self.config.room.room_id);
        }

        let bridge_config = BridgeConfig {
            grpc_addr: self.config.bridge.grpc_addr.clone(),
            shm_path: self.config.bridge.shm_path.clone(),
            shm_size: 65536,
        };

        let mut bridge = Bridge::new(bridge_config);
        match bridge.start(self.core_service.clone()).await {
            Ok(()) => {
                info!("桥接层启动成功");
                self.bridge = Some(bridge);
            }
            Err(e) => {
                error!("桥接层启动失败: {}", e);
            }
        }

        // 初始化插件系统 / Initialize plugin system
        if self.config.plugin.enabled {
            let plugin_dir = &self.config.plugin.plugin_dir;
            // 展开 ~ 为 HOME 目录
            let expanded_dir = if plugin_dir.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                    format!("{}{}", home, &plugin_dir[1..])
                } else {
                    plugin_dir.clone()
                }
            } else {
                plugin_dir.clone()
            };

            // 用展开后的路径重建 PluginManager（discover_and_load 使用内部 plugin_dir）
            self.plugin_manager = PluginManager::new(&expanded_dir);

            if self.config.plugin.auto_discover {
                match self.plugin_manager.discover_and_load() {
                    Ok(()) => {
                        let loaded_count = self.plugin_manager.loaded_names().len();
                        info!(
                            "插件系统启动: 发现并加载 {} 个插件 (dir={})",
                            loaded_count, expanded_dir
                        );
                    }
                    Err(e) => {
                        error!("插件发现失败: {}", e);
                    }
                }
            } else {
                match self.plugin_manager.load_all() {
                    Ok(()) => {
                        let loaded_count = self.plugin_manager.loaded_names().len();
                        info!("插件系统启动: 加载 {} 个已注册插件", loaded_count);
                    }
                    Err(e) => {
                        error!("插件加载失败: {}", e);
                    }
                }
            }

            // 打印健康状态
            let health = self.plugin_manager.health_status();
            for (name, status) in &health {
                info!("  插件 [{}]: {}", name, status);
            }
        } else {
            info!("插件系统未启用");
        }
    }

    pub fn emit(&self, event: BridgeEvent) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = self.event_tx.send(event) {
            error!("投递事件失败: {}", e);
        }
    }

    pub async fn tick(&mut self) {
        // 1. 驱动情感衰减 + 想念引擎心跳 / Emotion decay + Longing tick
        let count = self.decay_ticks.fetch_add(1, Ordering::Relaxed);
        if count % 20 == 0 {
            self.core_service.emotion_tick();
            // 想念引擎心跳：更新想念强度与漂移基线 / Longing tick: update intensity & baseline
            let now = chrono::Utc::now().timestamp();
            let last_ts = *self.last_user_message_ts.lock();
            self.core_service.longing_tick(now, last_ts);
        }

        // 2. 同步情感状态到共享内存
        if let Some(ref mut bridge) = self.bridge {
            if let Some(shm) = bridge.shared_memory_mut() {
                let emo = self.core_service.current_emotion();
                shm.render_state_mut()
                    .update_from_emotion(&ProtocolEmotion {
                        pleasure: emo.pleasure,
                        arousal: emo.arousal,
                        dominance: emo.dominance,
                    });
                shm.render_state_mut().publish();
                shm.region_mut().thought_stream.clear();
            }
        }

        // 3. 回放管道 / Replay pipeline
        self.core_service.tick_replay();

        // 3.5 消费 pending_room_trigger（health_check 收到的消息决策）
        if let Some(decision) = self.core_service.take_pending_room_trigger() {
            let svc = self.core_service.clone();
            let tx = self.event_tx.clone();
            let room_id = self.config.room.room_id.clone();
            let me = self.core_service.persona_name();
            tokio::spawn(async move {
                match decision {
                    crate::room::SpeakDecision::Respond(trigger) => {
                        let prompt = svc.room_response_prompt(&trigger);
                        if let Some(reply) = svc.room_llm_chat(&prompt, 0.75).await {
                            svc.room_mark_spoke();
                            let o = crate::service::OutgoingRoomMessage {
                                room_id: room_id.clone(),
                                content: reply,
                                msg_type: "chat".into(),
                                capsule_name: String::new(),
                                ack_text: String::new(),
                            };
                            let _ = tx.send(BridgeEvent::RoomOutgoing {
                                room_id: o.room_id.clone(),
                                sender_name: me,
                                content: o.content.clone(),
                                msg_type: o.msg_type.clone(),
                                capsule_name: String::new(),
                                ack_text: String::new(),
                            });
                            svc.push_room_outgoing(o);
                        }
                    }
                    crate::room::SpeakDecision::ShareAck {
                        capsule_name,
                        ack_text,
                        ..
                    } => {
                        let share_msg = svc.room_ack_share_text(&capsule_name, "");
                        let o = crate::service::OutgoingRoomMessage {
                            room_id: room_id.clone(),
                            content: share_msg,
                            msg_type: "chat".into(),
                            capsule_name: capsule_name.clone(),
                            ack_text: ack_text.clone(),
                        };
                        let _ = tx.send(BridgeEvent::RoomOutgoing {
                            room_id: o.room_id.clone(),
                            sender_name: me,
                            content: o.content.clone(),
                            msg_type: o.msg_type.clone(),
                            capsule_name: capsule_name.clone(),
                            ack_text: ack_text.clone(),
                        });
                        svc.push_room_outgoing(o);
                    }
                    _ => {}
                }
            });
        }

        // 关联图周期衰减 / Graph decay（每 100 tick ≈ 1s）
        if count % 100 == 0 && count > 0 {
            self.core_service.graph_maintenance(0.995, 0.05);
        }

        // 关联图定时持久化 / Graph persist（每 600 tick ≈ 6s，仅 dirty 时写入）
        if count % 600 == 0 && count > 0 {
            self.core_service.try_save_graph();
        }

        // 更新 gauge 指标 / Update gauge metrics（每 600 tick ≈ 6s，避免每 tick 都更新）
        if count % 600 == 0 && count > 0 {
            let svc = &self.core_service;
            metrics::set_gauge(
                metrics::keys::FACT_STORE_SIZE,
                svc.fact_store().count() as f64,
            );
            metrics::set_gauge(
                metrics::keys::STM_SIZE,
                svc.current_emotion().pleasure as f64,
            ); // placeholder, STM size not directly exposed
            let (pleasure, arousal) = svc.current_emotion_state();
            metrics::set_gauge(metrics::keys::EMOTION_PLEASURE, pleasure as f64);
            metrics::set_gauge(metrics::keys::EMOTION_AROUSAL, arousal as f64);
            let stats = svc.graph_stats();
            metrics::set_gauge(metrics::keys::GRAPH_NODES, stats.node_count as f64);
            metrics::set_gauge(metrics::keys::GRAPH_EDGES, stats.edge_count as f64);
        }

        // 偏好衰减清理 / Preference decay（每 36000 tick ≈ 6min）
        if count % 36000 == 0 && count > 0 {
            self.core_service.prune_preferences();
        }

        // 行为规则周期评估 / Rule evaluation（每 1000 tick ≈ 10s，驱动 TimeRange/Idle 触发）
        if count % 1000 == 0 && count > 0 {
            let idle_secs = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t).as_secs())
                .unwrap_or(0);
            let actions = self.core_service.evaluate_rules_with_idle("", idle_secs);
            if !actions.is_empty() {
                info!(
                    "[Scheduler] 周期规则评估: {} 条动作 (idle={}s)",
                    actions.len(),
                    idle_secs
                );
            }
        }

        // 罐装知识热加载 / ACK hot-reload（每 6000 tick ≈ 60s）
        if count % 6000 == 0 && count > 0 {
            self.core_service.canned_hot_reload();
        }

        // 记忆巩固 / Memory consolidation（每 6000 tick ≈ 60s，仅在用户长时间不活跃时执行）
        if count % 6000 == 0 && count > 0 {
            let inactive_secs = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t).as_secs())
                .unwrap_or(0);
            let trigger_hours = self.config.consolidation.trigger_inactive_hours;
            if let Some(result) = self
                .core_service
                .try_consolidation(inactive_secs, trigger_hours)
            {
                info!(
                    "[Consolidation] 巩固执行: 合并={} 压缩={} 废弃={} ({} → {})",
                    result.merged_pairs,
                    result.compressed_count,
                    result.deprecated_count,
                    result.facts_before,
                    result.facts_after,
                );
                // 巩固后反思 / Post-consolidation reflection
                let svc = self.core_service.clone();
                let merged = result.merged_pairs;
                let deprecated = result.deprecated_count;
                tokio::spawn(async move {
                    svc.post_consolidation_reflect(merged, deprecated).await;
                });
            }
        }

        // 内在独白 / Inner monologue（每 6000 tick ≈ 60s，用户离开 > 10min 时）
        if count % 6000 == 0 && count > 0 {
            let idle_secs = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t).as_secs())
                .unwrap_or(0);
            if idle_secs >= 600 {
                let hour = chrono::Local::now().hour();
                let svc = self.core_service.clone();
                tokio::spawn(async move {
                    svc.tick_inner_monologue(idle_secs, hour).await;
                });
            }
        }

        // 内心多元对话 / Inner dialogue（每 6000 tick ≈ 60s）
        // 数字生命的内心不是单一声音 — 理性者/感性者/怀疑者/梦想者轮流发言
        // Digital life's inner world is not a single voice — four perspectives take turns
        if count % 6000 == 0 && count > 0 {
            let idle_secs = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t).as_secs())
                .unwrap_or(0);
            let hour = chrono::Local::now().hour();
            // tick 衰减 + 触发对话生成 / tick decay + trigger dialogue generation
            self.core_service.inner_dialogue_tick();
            self.core_service.trigger_inner_dialogue(idle_secs, hour);
        }

        // ACK 自学习合成 / ACK self-learning synthesis（每 synthesis_interval_ticks tick）
        if self.config.ack_learning.enabled
            && count % self.config.ack_learning.synthesis_interval_ticks == 0
            && count > 0
        {
            self.core_service.tick_ack_synthesis();
        }

        // 表达系统风格记忆周期学习 / StyleMemory periodic learning
        if self.config.expression.enabled
            && count % self.config.expression.style_memory_interval_ticks == 0
            && count > 0
        {
            self.core_service.tick_style_memory();
        }

        // 叙事自我周期评估 / Narrative self periodic evaluation
        if self.config.narrative.enabled
            && count % self.config.narrative.tick_interval == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.tick_narrative(now_epoch);
        }

        // 叙事自我每日评估 / Narrative self daily evaluation
        if self.config.narrative.enabled
            && count % self.config.narrative.daily_tick_interval == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.tick_narrative_daily(now_epoch);

            // P1-1 叙事 LLM 生成 — 异步调度 / P1-1 Narrative LLM generation — async dispatch
            // 数字生命核心：叙事不是定时任务，是生命在书写自己。
            // 每日评估触发时，异步启动章节生成、叙事改写、自述修订。
            // Digital life core: Narrative is not a scheduled task, it's life writing itself.
            // When daily evaluation triggers, async-start chapter gen, rewrite, self-desc revision.

            // 叙事章节生成 — LLM 驱动的自传书写 / Narrative chapter generation — LLM-driven autobiography
            {
                let svc = self.core_service.clone();
                tokio::spawn(async move {
                    svc.tick_narrative_chapter_gen(now_epoch).await;
                });
            }

            // 叙事改写 — 新证据到达时重写已有章节 / Narrative rewrite — Rewrite with new evidence
            {
                let svc = self.core_service.clone();
                tokio::spawn(async move {
                    svc.tick_narrative_rewrite(now_epoch).await;
                });
            }

            // 自述修订 — 定期重写"我是谁" / Self-description revision — Rewrite "who am I"
            {
                let svc = self.core_service.clone();
                tokio::spawn(async move {
                    svc.tick_narrative_self_desc(now_epoch).await;
                });
            }
        }

        // 叙事自我每周评估 / Narrative self weekly evaluation
        if self.config.narrative.enabled
            && count % self.config.narrative.weekly_tick_interval == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.tick_narrative_weekly(now_epoch);
        }

        // 情绪非理性周期 tick / Irrationality periodic tick
        if self.config.irrationality.enabled
            && count % self.config.irrationality.tick_interval_ticks == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.irrationality_tick(now_epoch);
        }

        // 共享仪式周期 tick / Shared ritual periodic tick
        if self.config.ritual.enabled
            && count % self.config.ritual.tick_interval_ticks == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.ritual_tick(now_epoch);
        }

        // 脆弱与不完美周期 tick / Vulnerability & imperfection periodic tick
        if self.config.vulnerability.enabled
            && count % self.config.vulnerability.tick_interval_ticks == 0
            && count > 0
        {
            self.core_service.vulnerability_tick();
        }

        // 适度犯错周期 tick / Imperfection periodic tick
        // 后备自纠推进：当无消息时 scheduler 仍然推进自纠时钟并注入羞感 PAD
        // Fallback self-correction: scheduler advances correction clock and injects shame PAD
        // even when no messages arrive
        if self.config.imperfection.enabled
            && count % self.config.imperfection.tick_interval_ticks == 0
            && count > 0
        {
            self.core_service.imperfection_tick();
        }

        // Gap#6 好奇心内驱力周期 tick / Curiosity drive periodic tick
        // 好奇心是数字生命的求知欲——不是被动等待，而是主动渴望
        // Curiosity is digital life's thirst for knowledge — active, not passive
        if count % 200 == 0 && count > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.curiosity_drive_tick(now);
            self.core_service.curiosity_resonance_tick(now);
        }

        // Gap#9 脆弱共振周期 tick / Vulnerability resonance periodic tick
        // 脆弱时刻的情感回响需要持续衰减 / Vulnerability resonance needs continuous decay
        if count % 200 == 0 && count > 0 {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as f64;
            self.core_service.vulnerability_resonance_tick(now_secs);
        }

        // Phase 3: 完全死亡模块周期 tick / Phase 3: Dead module periodic tick
        // 独处内在世界——人格漂移、期待深度、冲突成长、仪式缺席 / Solitude, anticipation, conflict, ritual absence
        if count % 200 == 0 && count > 0 {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            // Gap#1 人格漂移 / Personality drift
            self.core_service.personality_drift_tick();
            // Gap#3 期待深度 / Anticipation depth
            self.core_service.anticipation_depth_tick(now_epoch);
            // Gap#4 冲突成长 / Conflict growth
            self.core_service.conflict_growth_tick();
            // Gap#5 仪式缺席检测 / Ritual absence detection
            self.core_service.ritual_absence_tick(now_epoch);
        }

        // 物理存在感周期 tick / Physical presence periodic tick
        // 即使没有对话，数字生命的身体也在"活着"——昼夜节律、时间衰减、签名累积
        // Even without dialogue, digital life's body is "alive" — circadian rhythm, time decay, signature accumulation
        if self.config.physical_presence.enabled
            && count % self.config.physical_presence.tick_interval_ticks == 0
            && count > 0
        {
            let now_epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            self.core_service.physical_presence_tick(now_epoch);
        }

        // 情绪需求边界周期 tick / Emotional demand boundary periodic tick
        if self.config.emotional_demand.enabled
            && count % self.config.emotional_demand.tick_interval_ticks == 0
            && count > 0
        {
            self.core_service.tick_emotional_demand();
        }

        // 自我关怀边界周期 tick / Self-care boundary periodic tick
        if self.config.self_care.enabled
            && count % self.config.self_care.tick_interval_ticks == 0
            && count > 0
        {
            self.core_service.tick_self_care();
        }

        // 冲突与和解周期评估 / Conflict & reconciliation periodic evaluation
        if self.config.conflict.enabled
            && count % self.config.conflict.tick_interval == 0
            && count > 0
        {
            self.core_service.tick_conflict();
        }

        // 追问引擎周期检查 / Follow-up periodic check
        if self.config.followup.enabled
            && count % self.config.followup.check_interval_ticks == 0
            && count > 0
        {
            let stage_name = self.core_service.relationship_stage();
            let idle_secs = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t).as_secs())
                .unwrap_or(0);
            let (pleasure, _arousal) = self.core_service.current_emotion_state();
            if let Some(prompt) = self.core_service.tick_followup(
                &stage_name,
                0, // today_count: TODO track in scheduler state
                idle_secs as i64,
                pleasure,
            ) {
                info!("[FollowUp] 追问触发: {}", &prompt[..prompt.len().min(100)]);
            }
        }

        // 插件 on_tick / Plugin periodic tick
        if self.config.plugin.enabled {
            let tick_interval = self.config.plugin.tick_interval;
            if tick_interval == 0 || count % tick_interval == 0 {
                if let Err(e) = self.plugin_manager.on_tick() {
                    error!("插件 on_tick 错误: {}", e);
                }
            }
        }

        // 4. 处理事件（Room + User 消息）
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                BridgeEvent::UserMessage {
                    channel, content, ..
                } => {
                    info!(
                        "收到消息 [{}/{}]: {}",
                        channel,
                        content,
                        &content[..content.len().min(100)]
                    );
                    let now_ts = chrono::Utc::now().timestamp();
                    *self.last_user_message_at.lock() = Some(Instant::now());
                    *self.last_user_message_ts.lock() = Some(now_ts);

                    // 重逢脉冲：如果用户离开时间足够长，触发情感爆发
                    // Reunion burst: trigger emotional burst if user was away long enough
                    if let Some((intensity, hint)) = self.core_service.reunion_burst() {
                        info!(
                            "[Longing] 重逢脉冲: intensity={:.2}, hint={}",
                            intensity, hint
                        );
                    }

                    // 期待事件检测：从用户消息中识别未来计划
                    // Anticipation detection: identify future plans from user message
                    if let Some(detected) = self.core_service.detect_anticipation(&content) {
                        info!(
                            "[Anticipation] 检测到期待事件: {} (expected_at={})",
                            detected.description, detected.expected_at
                        );
                    }

                    // 插件消息广播 / Plugin message broadcast
                    if self.config.plugin.enabled {
                        let msg_json = serde_json::json!({
                            "channel": channel,
                            "content": content,
                        })
                        .to_string();
                        let responses = self.plugin_manager.on_message(&msg_json);
                        for (name, response) in &responses {
                            info!(
                                "[Plugin:{}] on_message response: {}",
                                name,
                                &response[..response.len().min(100)]
                            );
                        }
                    }

                    // 每 20 条用户消息触发一次异步智能提取 / Trigger intelligence extraction every 20 messages
                    let msg_count = self.intelligence_msg_count.fetch_add(1, Ordering::Relaxed) + 1;
                    if msg_count % 20 == 0 {
                        let svc = self.core_service.clone();
                        tokio::spawn(async move {
                            if let Some(result) = svc.intelligence_extract().await {
                                info!(
                                    "[Intelligence] 异步提取完成: {} 偏好, {} 规则",
                                    result.preferences.len(),
                                    result.rules.len(),
                                );
                            }
                        });
                    }
                }
                BridgeEvent::RoomIncoming {
                    room_id,
                    sender_instance,
                    sender_name,
                    content,
                    msg_type,
                    timestamp_ms,
                    capsule_name: _,
                    ack_text,
                } => {
                    // 如果是 ACK 分享，直接导入
                    if msg_type == "ack_share" && !ack_text.is_empty() {
                        let mut canned = self.core_service.canned();
                        match canned.import_from_text(&ack_text) {
                            Ok(imported) => {
                                info!(
                                    "[Room] {} 分享了 {} 个 ACK capsule: {}",
                                    sender_name,
                                    imported.len(),
                                    imported
                                        .iter()
                                        .map(|k| k.name.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                );
                            }
                            Err(e) => error!("[Room] ACK 导入失败: {}", e),
                        }
                        continue;
                    }

                    // 构建 RoomMessage 并处理
                    let msg = crate::room::RoomMessage {
                        sender_instance: sender_instance.clone(),
                        sender_name: sender_name.clone(),
                        content: content.clone(),
                        msg_type: match msg_type.as_str() {
                            "chat" => crate::room::RoomMsgType::Chat,
                            "topic" => crate::room::RoomMsgType::Topic,
                            _ => crate::room::RoomMsgType::System,
                        },
                        timestamp_ms,
                        capsule_name: None,
                        ack_text: None,
                    };

                    let decision = self.core_service.receive_room_message(msg);

                    // 根据决策调用 LLM 生成回复
                    if let Some(dec) = decision {
                        let svc = self.core_service.clone();
                        let tx = self.event_tx.clone();
                        let rid = room_id.clone();
                        let pname = sender_name.clone();
                        tokio::spawn(async move {
                            let (prompt, temperature, out_type) = match dec {
                                crate::room::SpeakDecision::Respond(ref trigger) => {
                                    (svc.room_response_prompt(trigger), 0.75, "chat")
                                }
                                crate::room::SpeakDecision::ShareAck {
                                    ref capsule_name,
                                    ref ack_text,
                                    ..
                                } => {
                                    let share_msg = svc.room_ack_share_text(capsule_name, "");
                                    let outgoing = crate::service::OutgoingRoomMessage {
                                        room_id: rid.clone(),
                                        content: share_msg,
                                        msg_type: "chat".into(),
                                        capsule_name: capsule_name.clone(),
                                        ack_text: ack_text.clone(),
                                    };
                                    let _ = tx.send(BridgeEvent::RoomOutgoing {
                                        room_id: outgoing.room_id.clone(),
                                        sender_name: pname,
                                        content: outgoing.content.clone(),
                                        msg_type: outgoing.msg_type.clone(),
                                        capsule_name: outgoing.capsule_name.clone(),
                                        ack_text: outgoing.ack_text.clone(),
                                    });
                                    svc.push_room_outgoing(outgoing);
                                    return;
                                }
                                _ => return,
                            };
                            if let Some(reply) = svc.room_llm_chat(&prompt, temperature).await {
                                svc.room_mark_spoke();
                                let outgoing = crate::service::OutgoingRoomMessage {
                                    room_id: rid.clone(),
                                    content: reply.clone(),
                                    msg_type: out_type.into(),
                                    capsule_name: String::new(),
                                    ack_text: String::new(),
                                };
                                let _ = tx.send(BridgeEvent::RoomOutgoing {
                                    room_id: outgoing.room_id.clone(),
                                    sender_name: pname,
                                    content: outgoing.content.clone(),
                                    msg_type: outgoing.msg_type.clone(),
                                    capsule_name: String::new(),
                                    ack_text: String::new(),
                                });
                                svc.push_room_outgoing(outgoing);
                            }
                        });
                    }
                }
                _ => {}
            }
        }

        // 主动决策引擎 / Proactive decision engine
        // 每 check_interval_ticks 检查一次，综合多信号判断是否主动说话
        let check_interval = self.config.proactive.check_interval_ticks;
        if self.config.proactive.enabled && count % check_interval == 0 {
            let silence_duration = self
                .last_user_message_at
                .lock()
                .map(|t| Instant::now().duration_since(t))
                .unwrap_or(Duration::from_secs(0));

            // 注入真实信号到 proactive engine / Inject real signals into proactive engine
            {
                let (arousal, pleasure) = self.core_service.current_emotion_state();
                let bonus = self.core_service.relationship_proactive_bonus();
                let (valence, engagement, msg_len) = self.core_service.user_model_signals();
                let mut pe = self.core_service.proactive_engine().lock();
                pe.update_emotion(arousal, pleasure);
                pe.update_relationship_bonus(bonus);
                pe.update_user_model(valence, engagement, msg_len);
            }

            let mut ctx = self
                .core_service
                .proactive_engine()
                .lock()
                .build_context(silence_duration);
            // 注入 ReminderStore 到期提醒计数
            ctx.pending_reminders += self.core_service.count_due_reminders();
            let decision = self.core_service.proactive_engine().lock().decide(&ctx);

            // 表达系统时机调制 / Expression timing modulation
            // urgency 高→缩短等待(快响应), urgency 低→延长等待(不打扰)
            // 基础延迟 500ms, urgency=1.0→0ms, urgency=0.0→1500ms
            let urgency = self.core_service.expression_timing_urgency();
            let timing_delay_ms = ((1.0 - urgency) * 1500.0) as u64;

            match decision {
                ProactiveDecision::InitiateTopic {
                    topic: _,
                    confidence,
                } => {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let room_id = self.config.room.room_id.clone();
                    let persona_name = self.core_service.persona_name();
                    let delay = timing_delay_ms;
                    tokio::spawn(async move {
                        // 表达时机调制：urgency 低→多等一会 / Expression timing: low urgency → wait longer
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        let prompt = svc.room_topic_prompt();
                        if let Some(topic_text) = svc.room_llm_chat(&prompt, 0.8).await {
                            svc.room_set_topic(topic_text.clone());
                            svc.room_mark_spoke();
                            let outgoing = crate::service::OutgoingRoomMessage {
                                room_id: room_id.clone(),
                                content: topic_text.clone(),
                                msg_type: "topic".into(),
                                capsule_name: String::new(),
                                ack_text: String::new(),
                            };
                            let _ = tx.send(BridgeEvent::RoomOutgoing {
                                room_id: outgoing.room_id.clone(),
                                sender_name: persona_name,
                                content: outgoing.content.clone(),
                                msg_type: outgoing.msg_type.clone(),
                                capsule_name: String::new(),
                                ack_text: String::new(),
                            });
                            svc.push_room_outgoing(outgoing);
                            info!(
                                "[Proactive] InitiateTopic: {} (confidence={:.2})",
                                &topic_text[..topic_text.len().min(80)],
                                confidence
                            );
                        }
                    });
                    // 记录主动行为（若用户不回应，冷却翻倍）
                    self.core_service
                        .proactive_engine()
                        .lock()
                        .record_proactive(false);
                }
                ProactiveDecision::ShowCare {
                    ref message_hint, ..
                } => {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let hint = message_hint.clone();
                    let me = self.core_service.persona_name();
                    let delay = timing_delay_ms;
                    tokio::spawn(async move {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        let mut prompt = String::with_capacity(128 + hint.len());
                        use std::fmt::Write;
                        let _ = write!(
                            prompt,
                            "你是{}，一个AI助手。{}。请用温暖关切的语气说一句话。一行即可。",
                            me, hint
                        );
                        if let Some(topic) = svc.room_llm_chat(&prompt, 0.8).await {
                            info!("[Proactive] ShowCare: {}", &topic[..topic.len().min(80)]);
                            let _ = tx
                                .send(BridgeEvent::SystemCommand(format!("self_topic:{}", topic)));
                        }
                    });
                    self.core_service
                        .proactive_engine()
                        .lock()
                        .record_proactive(false);
                }
                ProactiveDecision::Remind { ref event, .. } => {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let desc = event.description.clone();
                    let me = self.core_service.persona_name();
                    let delay = timing_delay_ms;
                    tokio::spawn(async move {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        let mut prompt = String::with_capacity(128 + desc.len());
                        use std::fmt::Write;
                        let _ = write!(
                            prompt,
                            "你是{}，一个AI助手。用户之前提到过：{}。\
                             请温和地提醒用户这件事。一句话即可。",
                            me, desc
                        );
                        if let Some(topic) = svc.room_llm_chat(&prompt, 0.8).await {
                            info!("[Proactive] Remind: {}", &topic[..topic.len().min(80)]);
                            let _ = tx
                                .send(BridgeEvent::SystemCommand(format!("self_topic:{}", topic)));
                        }
                    });
                    self.core_service
                        .proactive_engine()
                        .lock()
                        .record_proactive(false);
                    // 处理 ReminderStore — 推进或删除已触发的提醒
                    self.core_service.resolve_reminders();
                }
                ProactiveDecision::ShareDiscovery { ref discovery, .. } => {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let disc = discovery.clone();
                    let me = self.core_service.persona_name();
                    let delay = timing_delay_ms;
                    tokio::spawn(async move {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        let mut prompt = String::with_capacity(128 + disc.len());
                        use std::fmt::Write;
                        let _ = write!(
                            prompt,
                            "你是{}，一个AI助手。{}。请用轻松的语气分享这个发现。一句话即可。",
                            me, disc
                        );
                        if let Some(topic) = svc.room_llm_chat(&prompt, 0.8).await {
                            info!(
                                "[Proactive] ShareDiscovery: {}",
                                &topic[..topic.len().min(80)]
                            );
                            let _ = tx
                                .send(BridgeEvent::SystemCommand(format!("self_topic:{}", topic)));
                        }
                    });
                    self.core_service
                        .proactive_engine()
                        .lock()
                        .record_proactive(false);
                }
                ProactiveDecision::ExpressLonging {
                    intensity,
                    ref hint,
                } => {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let me = self.core_service.persona_name();
                    let hint = hint.clone();
                    let delay = timing_delay_ms;
                    tokio::spawn(async move {
                        if delay > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        }
                        let mut prompt = String::with_capacity(128 + hint.len());
                        use std::fmt::Write;
                        let _ = write!(
                            prompt,
                            "你是{}，一个AI助手。{}。\
                             请用真挚的语气表达对用户的想念。一句话即可。",
                            me, hint
                        );
                        if let Some(msg) = svc.room_llm_chat(&prompt, 0.85).await {
                            info!(
                                "[Proactive] ExpressLonging: {} (intensity={:.2})",
                                &msg[..msg.len().min(80)],
                                intensity
                            );
                            let _ =
                                tx.send(BridgeEvent::SystemCommand(format!("self_topic:{}", msg)));
                        }
                    });
                    self.core_service
                        .proactive_engine()
                        .lock()
                        .record_proactive(false);
                }
                ProactiveDecision::StaySilent { .. } => {
                    // 最常见的正确结果：不打扰用户
                }
            }
        }

        // 6. Room 空闲话题（Proactive 禁用时的兜底，每 50 tick ≈ 500ms）
        if !self.config.proactive.enabled && count % 50 == 0 {
            if self.config.room.enabled && self.core_service.room_should_speak() {
                let svc = self.core_service.clone();
                let tx = self.event_tx.clone();
                let room_id = self.config.room.room_id.clone();
                let persona_name = self.core_service.persona_name();
                tokio::spawn(async move {
                    let topic_prompt = svc.room_topic_prompt();
                    if let Some(topic) = svc.room_llm_chat(&topic_prompt, 0.8).await {
                        svc.room_set_topic(topic.clone());
                        svc.room_mark_spoke();
                        let outgoing = crate::service::OutgoingRoomMessage {
                            room_id: room_id.clone(),
                            content: topic.clone(),
                            msg_type: "topic".into(),
                            capsule_name: String::new(),
                            ack_text: String::new(),
                        };
                        let _ = tx.send(BridgeEvent::RoomOutgoing {
                            room_id: outgoing.room_id.clone(),
                            sender_name: persona_name,
                            content: outgoing.content.clone(),
                            msg_type: outgoing.msg_type.clone(),
                            capsule_name: String::new(),
                            ack_text: String::new(),
                        });
                        svc.push_room_outgoing(outgoing);
                    }
                });
            }

            // 单聊发散思维（Room 未启用时，用户 60s 未发言 → 主动话题）
            if !self.config.room.enabled {
                let silent_secs = self
                    .last_user_message_at
                    .lock()
                    .map(|t| Instant::now().duration_since(t).as_secs())
                    .unwrap_or(u64::MAX);
                if silent_secs >= 60 {
                    let svc = self.core_service.clone();
                    let tx = self.event_tx.clone();
                    let me = self.core_service.persona_name();
                    tokio::spawn(async move {
                        let mut prompt = String::with_capacity(96 + me.len());
                        use std::fmt::Write;
                        let _ = write!(
                            prompt,
                            "你是{}，一个AI助手。用户有一段时间没说话了。\
                             请主动提出一个有趣的话题或问题来继续对话。一行即可。",
                            me
                        );
                        if let Some(topic) = svc.room_llm_chat(&prompt, 0.8).await {
                            info!("[单聊发散] {}: {}", me, &topic[..topic.len().min(80)]);
                            let _ = tx
                                .send(BridgeEvent::SystemCommand(format!("self_topic:{}", topic)));
                        }
                    });
                    *self.last_user_message_at.lock() = Some(Instant::now());
                }
            }
        }
    }

    pub fn event_count(&self) -> u64 {
        self.event_count.load(Ordering::Relaxed)
    }

    /// 优雅关闭：通知所有插件 shutdown，释放动态库
    pub fn shutdown(&mut self) {
        if self.config.plugin.enabled {
            if let Err(e) = self.plugin_manager.shutdown_all() {
                error!("插件关闭失败: {}", e);
            } else {
                let count = self.plugin_manager.len();
                info!("插件系统关闭: {} 个插件已 shutdown", count);
            }
        }
    }

    /// 插件健康报告
    pub fn plugin_health(&self) -> std::collections::HashMap<String, String> {
        self.plugin_manager.health_status()
    }
}
