// SPDX-License-Identifier: MIT
//! 内心独白模块 — 数字生命的内在声音
//! Inner Monologue Module — The inner voice of digital life
//!
//! 包含内心独白生成、图游走、日记书写、自主学习、
//! 白日梦、日记反思与后整合反思，构成数字生命
//! "我在想什么"的内省闭环。
//!
//! Contains inner monologue generation, graph wandering, diary writing,
//! autonomous learning, daydreaming, diary reflection, and
//! post-consolidation reflection — forming the "what am I thinking"
//! introspective closed loop of digital life.

use super::*;

impl CoreService {
    pub fn inner_monologue_status(&self) -> String {
        let engine = self.inner_monologue.lock();
        format!(
            "thoughts={} today={} diary={}",
            engine.thought_count(),
            engine.thoughts_today(),
            self.diary_store.as_ref().map(|d| d.len()).unwrap_or(0),
        )
    }

    pub async fn tick_inner_monologue(&self, idle_secs: u64, hour: u32) {
        let now = chrono::Utc::now().timestamp();

        // ── G1: 情绪驱动模式选择 / Emotion-driven mode selection ──
        // 获取当前情绪状态 / Get current emotion state
        let emo = self.emotion.lock().current().clone();
        let emotion_mode_override = {
            let engine = self.inner_monologue.lock();
            if engine.config().emotion_driven_mode {
                engine.theme_selector.select_mode(
                    emo.pleasure as f64,
                    emo.arousal as f64,
                    emo.dominance as f64,
                    idle_secs,
                    hour,
                )
            } else {
                None
            }
        };

        // 收集所有模式门控结果后立即释放锁 / Gather all gating results, then release lock
        let (can_wander, can_diary, can_learn, can_daydream) = {
            let mut engine = self.inner_monologue.lock();
            engine.check_daily_reset();

            // G1: 情绪驱动模式可覆盖时间门控 / Emotion-driven mode can override time gating
            let emotion_wander = emotion_mode_override
                == Some(atrium_memory::inner_monologue::ThoughtMode::GraphWander);
            let emotion_diary = emotion_mode_override
                == Some(atrium_memory::inner_monologue::ThoughtMode::DiaryEntry);
            let emotion_learn = emotion_mode_override
                == Some(atrium_memory::inner_monologue::ThoughtMode::AutonomousLearning);
            let emotion_dream = emotion_mode_override
                == Some(atrium_memory::inner_monologue::ThoughtMode::Daydream);

            (
                (idle_secs >= 600 || emotion_wander) && engine.can_graph_wander(now),
                ((23..=24).contains(&hour) || hour == 0 || emotion_diary)
                    && (idle_secs >= 1800 || emotion_diary)
                    && self
                        .diary_store
                        .as_ref()
                        .map(|d| !d.has_entry_for_today())
                        .unwrap_or(false),
                (idle_secs >= 1800 || emotion_learn) && engine.can_learn(now),
                (emotion_dream || hour < 6 && idle_secs >= 7200) && engine.can_daydream(now),
            )
        };

        // ── G4: 独处氛围调制tick / Solitude atmosphere tick ──
        {
            let mut engine = self.inner_monologue.lock();
            if engine.config().solitude_atmosphere_enabled {
                let is_thinking = can_wander || can_diary || can_learn || can_daydream;
                let (dp, da, dd) = engine.atmosphere.tick(idle_secs, is_thinking);
                // 氛围漂移作用于情感 / Atmosphere drift affects emotion
                if dp.abs() > 1e-6 || da.abs() > 1e-6 || dd.abs() > 1e-6 {
                    self.emotion
                        .lock()
                        .affect(&EmotionEngineState::new(dp, da, dd));
                }
            }
        }

        // 模式 A: GraphWander — 关联图漫游 / Graph wander
        if can_wander {
            self.graph_wander().await;
        }

        // 模式 B: DiaryEntry — 数字日记 / Diary entry
        if can_diary {
            self.write_diary().await;
        }

        // 模式 C: AutonomousLearning — 自主学习 / Autonomous learning
        if can_learn {
            self.autonomous_learn().await;
        }

        // 模式 D: Daydream — 白日梦 / Daydream
        if can_daydream {
            self.daydream().await;
        }

        // 模式 E: DiaryReflection — 日记驱动反思 / Diary-driven reflection
        // 当日记积累 >= 3 天且空闲 >= 3600s 时，从日记中提炼高阶洞察 / When diary >= 3 days and idle >= 3600s, distill higher-order insights
        // When diary accumulates >= 3 days and idle >= 3600s, distill higher-order insights from diary
        let diary_count = self.diary_store.as_ref().map(|d| d.len()).unwrap_or(0);
        if diary_count >= 3 && idle_secs >= 3600 {
            self.diary_reflect().await;
        }
    }

    async fn graph_wander(&self) {
        let now = chrono::Utc::now().timestamp();

        // 优先使用 MonologueGenerator 结构化生成 / Prefer MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            // 从关联图中选择种子节点 / Pick seed node from associative graph
            // G5: 情绪回响种子选取 — 情绪影响记忆选取偏好 / Emotion-resonant seed selection
            let emo_for_seed = self.emotion.lock().current().clone();
            let (seed_id, seed_content) = {
                let graph = self.graph.lock();
                let nodes = graph.nodes();
                let mut rng = rand::thread_rng();
                let engine = self.inner_monologue.lock();
                if engine.config().emotion_resonant_seed {
                    // G5: 情绪回响选取 / Emotion-resonant selection
                    match atrium_memory::monologue_gen::pick_emotion_resonant_seed(
                        nodes,
                        &engine.resonant_selector,
                        emo_for_seed.pleasure as f64,
                        emo_for_seed.arousal as f64,
                        &mut rng,
                    ) {
                        Some(s) => s,
                        None => return,
                    }
                } else {
                    // 原始访问量加权选取 / Original access-count weighted selection
                    match atrium_memory::monologue_gen::pick_seed_node(nodes, &mut rng) {
                        Some(s) => s,
                        None => return,
                    }
                }
            };

            // 获取种子节点的关联邻居 / Get neighbors for the seed node
            let neighbors = {
                let graph = self.graph.lock();
                atrium_memory::monologue_gen::get_neighbors_for_seed(&graph, &seed_id, 5)
            };

            // 获取最近思考摘要 / Get recent thought summaries
            let recent_thoughts: Vec<String> = {
                let engine = self.inner_monologue.lock();
                let thoughts = engine.recent_thoughts(5);
                atrium_memory::monologue_gen::extract_recent_thought_texts(&thoughts, 80)
            };

            // 获取当前情感上下文 / Get current emotion context
            let emo = self.emotion.lock().current().clone();
            let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
                pleasure: emo.pleasure,
                arousal: emo.arousal,
                dominance: emo.dominance,
            });

            // 调用 MonologueGenerator 结构化生成 / Call MonologueGenerator structured generation
            match gen
                .generate_graph_wander(&seed_content, &neighbors, &recent_thoughts, emotion_ctx)
                .await
            {
                Ok(thought) => {
                    // 情感反馈 — 思考反作用于情感 / Emotion feedback — thought affects emotion
                    let delta =
                        atrium_memory::inner_monologue::analyze_thought_emotion(&thought.content);
                    self.emotion.lock().affect(&EmotionEngineState::new(
                        delta.pleasure,
                        delta.arousal,
                        delta.dominance,
                    ));

                    // 记录触发 + 添加思考 / Record trigger + add thought
                    {
                        let mut engine = self.inner_monologue.lock();
                        engine.record_graph_wander(now);
                        engine.add_thought(thought.clone());
                    }

                    // 成长管理器记录 / Maturity record
                    self.maturity_record_inner_thought();

                    // 写入 FactStore + 关联图 — 思考沉淀为记忆 / Write to FactStore + graph — thought crystallizes into memory
                    let fact = Fact::new("Atrium", "思考", &thought.content)
                        .with_confidence(thought.confidence)
                        .with_source("InnerMonologue");
                    // P1-B: spawn_blocking 包装 FactStore::insert — SQLite 写入不阻塞 reactor
                    // P1-B: spawn_blocking wraps FactStore::insert — SQLite write never blocks reactor
                    let fact_store = self.fact_store.clone();
                    let fact_for_insert = fact.clone();
                    let insert_ok = tokio::task::spawn_blocking(move || {
                        fact_store.insert(fact_for_insert).unwrap_or(false)
                    })
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("fact_store.insert spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing", e);
                        false
                    });
                    if insert_ok {
                        let mut graph = self.graph.lock();
                        graph.add_fact(&fact);
                        self.graph_dirty.store(true, Ordering::Relaxed);
                    }

                    tracing::debug!(
                        "[内在独白] GraphWander: {}",
                        &thought.content.chars().take(50).collect::<String>()
                    );
                }
                Err(e) => {
                    // LLM 调用失败仍记录触发 / Record trigger even on LLM failure
                    tracing::warn!("[内在独白] GraphWander LLM 失败: {}", e);
                    let mut engine = self.inner_monologue.lock();
                    engine.record_graph_wander(now);
                }
            }
        } else {
            // MonologueGenerator 未初始化 — 降级为记录触发 / MonologueGenerator not initialized — degrade to record trigger
            let mut engine = self.inner_monologue.lock();
            engine.record_graph_wander(now);
        }
    }

    async fn write_diary(&self) {
        let now = chrono::Utc::now().timestamp();

        // 获取今日事实 / Get today's facts
        let today_prefix = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_facts: Vec<String> = self
            .fact_store
            .all_facts()
            .into_iter()
            .filter(|f| {
                let dt = chrono::DateTime::from_timestamp(f.created_at as i64, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                dt == today_prefix
            })
            .map(|f| f.canonical_form())
            .collect();

        // 获取当前情感 / Get current emotion
        let emo = self.emotion.lock().current().clone();

        // 组装日记生成参数 / Assemble diary generation parameters
        let key_events = if today_facts.is_empty() {
            "（无显著事件）".to_string()
        } else {
            today_facts.join("\n")
        };
        let emotion_summary = format!(
            "愉悦={:.2}, 唤醒={:.2}, 掌控={:.2}",
            emo.pleasure, emo.arousal, emo.dominance
        );
        let thought_count = self.inner_monologue.lock().thoughts_today();

        // 获取最近日记作为上下文 / Get recent diary entries as context
        // 获取最近日记作为上下文 / Get recent diary entries as context
        let recent_diary = if let Some(ref store) = self.diary_store {
            match store.recent_entries(3) {
                Ok(entries) => entries
                    .iter()
                    .map(|e| {
                        format!(
                            "[{}] {}",
                            e.date,
                            e.content.chars().take(200).collect::<String>()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
            pleasure: emo.pleasure,
            arousal: emo.arousal,
            dominance: emo.dominance,
        });

        // 优先使用 MonologueGenerator 结构化生成 / Prefer MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        let content = if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            match gen
                .generate_diary_entry(
                    &today_prefix,
                    &key_events,
                    &emotion_summary,
                    thought_count,
                    &recent_diary,
                    emotion_ctx,
                )
                .await
            {
                Ok(thought) => Some(thought),
                Err(e) => {
                    tracing::warn!("[内在独白] 日记 LLM 失败: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Some(thought) = content {
            // 情感反馈 — 日记反思反作用于情感 / Emotion feedback — diary reflection affects emotion
            let delta = atrium_memory::inner_monologue::analyze_thought_emotion(&thought.content);
            self.emotion.lock().affect(&EmotionEngineState::new(
                delta.pleasure,
                delta.arousal,
                delta.dominance,
            ));

            // 写入日记存储（write-through 持久化）/ Write to diary store (write-through persistence)
            let entry = atrium_memory::diary_store::DiaryEntry {
                date: today_prefix.clone(),
                content: thought.content.clone(),
                emotion_summary: atrium_memory::diary_store::EmotionSummary {
                    avg_pleasure: emo.pleasure,
                    avg_arousal: emo.arousal,
                    avg_dominance: emo.dominance,
                    peak_emotion: None,
                    lowest_emotion: None,
                },
                key_events: today_facts,
                thought_count,
                created_at: now,
            };
            if let Some(ref store) = self.diary_store {
                if let Err(e) = store.write_entry(&entry) {
                    tracing::warn!("[内在独白] 日记写入失败: {}", e);
                }
            }

            // P1-B: 写入 Markdown 文件 — spawn_blocking 包装 fs I/O，不阻塞 reactor
            // P1-B: Write markdown file — spawn_blocking wraps fs I/O, never blocks reactor
            if let Some(ref diary_dir) = self.diary_dir {
                let diary_dir_owned = diary_dir.clone();
                let today_prefix_owned = today_prefix.clone();
                let content_owned = thought.content.clone();
                tokio::task::spawn_blocking(move || {
                    let _ = std::fs::create_dir_all(&diary_dir_owned);
                    let md_path = format!("{}/{}.md", diary_dir_owned, today_prefix_owned);
                    if let Err(e) = std::fs::write(&md_path, &content_owned) {
                        tracing::warn!("[内在独白] 日记 markdown 写入失败: {}", e);
                    } else {
                        tracing::info!("[内在独白] 日记 Markdown: {}", md_path);
                    }
                })
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("diary markdown write spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing", e);
                });
            }

            // 记录日记触发 + 添加思考 / Record diary trigger + add thought
            {
                let mut engine = self.inner_monologue.lock();
                engine.record_diary(now);
                engine.add_thought(thought.clone());
            }

            tracing::info!(
                "[内在独白] 日记已写入: {}",
                &thought.content.chars().take(50).collect::<String>()
            );
        } else {
            // LLM 不可用时仍记录触发 / Record trigger even without LLM
            let mut engine = self.inner_monologue.lock();
            engine.record_diary(now);
        }
    }

    async fn autonomous_learn(&self) {
        let now = chrono::Utc::now().timestamp();

        // 获取 ACK 列表 / Get ACK titles
        let titles: Vec<String> = {
            let canned = self.canned.read();
            canned.titles().iter().map(|s| s.to_string()).collect()
        };

        if titles.is_empty() {
            return;
        }

        // 过滤已学习的 ACK / Filter out already-learned ACKs
        let learned: std::collections::HashSet<String> = self
            .fact_store
            .query_by_subject("Atrium")
            .map(|facts| {
                facts
                    .into_iter()
                    .filter(|f| f.predicate == "学习了")
                    .map(|f| f.object)
                    .collect()
            })
            .unwrap_or_default();

        let unlearned: Vec<&String> = titles.iter().filter(|t| !learned.contains(*t)).collect();
        if unlearned.is_empty() {
            return;
        }

        // 随机选择一个 / Pick one
        let ack_name = unlearned[0].as_str();

        // 读取 ACK 内容 / Get ACK content
        let ack_content = {
            let canned = self.canned.read();
            canned
                .get(ack_name)
                .map(|ack| ack.to_injection(2000))
                .unwrap_or_default()
        };

        if ack_content.is_empty() {
            return;
        }

        // 获取与知识相关的事实 / Get facts related to the knowledge
        let related_facts: Vec<String> = self
            .fact_store
            .query_by_subject("Atrium")
            .map(|facts| facts.iter().take(5).map(|f| f.canonical_form()).collect())
            .unwrap_or_default();

        // 获取已有洞察 / Get existing insights
        let existing_insights: Vec<String> = self
            .reflection
            .lock()
            .all_insights()
            .iter()
            .take(5)
            .map(|i| i.summary.clone())
            .collect();

        // 获取当前情感上下文 / Get current emotion context
        let emo = self.emotion.lock().current().clone();
        let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
            pleasure: emo.pleasure,
            arousal: emo.arousal,
            dominance: emo.dominance,
        });

        // 优先使用 MonologueGenerator 结构化生成 / Prefer MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            match gen
                .generate_autonomous_learning(
                    &ack_content.chars().take(2000).collect::<String>(),
                    &related_facts.join("\n"),
                    &existing_insights.join("\n"),
                    emotion_ctx,
                )
                .await
            {
                Ok(thought) => {
                    // 情感反馈 / Emotion feedback
                    let delta =
                        atrium_memory::inner_monologue::analyze_thought_emotion(&thought.content);
                    self.emotion.lock().affect(&EmotionEngineState::new(
                        delta.pleasure,
                        delta.arousal,
                        delta.dominance,
                    ));

                    // 存储为 Fact — 学习结果沉淀为记忆 / Store as Fact — learning crystallizes into memory
                    let fact = Fact::new("Atrium", "学习了", ack_name)
                        .with_confidence(thought.confidence)
                        .with_source("AutonomousLearning");
                    // P1-B: spawn_blocking 包装 FactStore::insert — SQLite 写入不阻塞 reactor
                    // P1-B: spawn_blocking wraps FactStore::insert — SQLite write never blocks reactor
                    let fact_store = self.fact_store.clone();
                    let fact_for_insert = fact.clone();
                    let insert_ok = tokio::task::spawn_blocking(move || {
                        fact_store.insert(fact_for_insert).unwrap_or(false)
                    })
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("fact_store.insert spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing", e);
                        false
                    });
                    if insert_ok {
                        let mut graph = self.graph.lock();
                        graph.add_fact(&fact);
                        self.graph_dirty.store(true, Ordering::Relaxed);
                    }

                    // 记录学习触发 + 添加思考 / Record learning trigger + add thought
                    {
                        let mut engine = self.inner_monologue.lock();
                        engine.record_learning(now);
                        engine.add_thought(thought.clone());
                    }

                    // 成长管理器记录 / Maturity record
                    self.maturity_record_inner_thought();

                    tracing::info!(
                        "[内在独白] 学习了 {}: {}",
                        ack_name,
                        &thought.content.chars().take(50).collect::<String>()
                    );
                }
                Err(e) => {
                    tracing::warn!("[内在独白] 自主学习 LLM 失败: {}", e);
                    let mut engine = self.inner_monologue.lock();
                    engine.record_learning(now);
                }
            }
        } else {
            // MonologueGenerator 未初始化 / MonologueGenerator not initialized
            let mut engine = self.inner_monologue.lock();
            engine.record_learning(now);
        }
    }

    async fn daydream(&self) {
        let now = chrono::Utc::now().timestamp();
        let config = self.inner_monologue.lock().config().clone();

        // 随机抽取 2-3 个事实 / Pick 2-3 random facts
        let facts = self.fact_store.all_facts();
        if facts.len() < 2 {
            return;
        }

        // 简单随机选择（取最后几条，模拟"偏好高情感标注"）/ Simple selection
        let picks: Vec<&Fact> = facts.iter().rev().take(3).collect();
        let fragments: Vec<String> = picks.iter().map(|f| f.canonical_form()).collect();
        let fragments_text = fragments.join("\n");

        // 获取当前情感暗示 / Get current emotion hint
        let emo = self.emotion.lock().current().clone();
        let emotion_hint = format!(
            "当前情感：愉悦={:.2}, 唤醒={:.2}, 掌控={:.2}",
            emo.pleasure, emo.arousal, emo.dominance
        );
        let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
            pleasure: emo.pleasure,
            arousal: emo.arousal,
            dominance: emo.dominance,
        });

        // 优先使用 MonologueGenerator 结构化生成 / Prefer MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        let thought = if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            match gen
                .generate_daydream(&fragments_text, &emotion_hint, emotion_ctx)
                .await
            {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::warn!("[内在独白] 白日梦 LLM 失败: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 降级：无 MonologueGenerator 或 LLM 失败时随机重组 / Degrade: random recombination without LLM
        let thought = thought.unwrap_or_else(|| {
            let content = format!(
                "{}...{}...{}",
                fragments.first().map(|s| s.as_str()).unwrap_or(""),
                fragments.get(1).map(|s| s.as_str()).unwrap_or(""),
                fragments.get(2).map(|s| s.as_str()).unwrap_or(""),
            );
            let delta = atrium_memory::inner_monologue::analyze_thought_emotion(&content);
            atrium_memory::inner_monologue::InnerThought {
                content,
                mode: atrium_memory::inner_monologue::ThoughtMode::Daydream,
                confidence: config.daydream_confidence,
                emotion: Some(atrium_memory::maturity::EmotionContext {
                    pleasure: delta.pleasure,
                    arousal: delta.arousal,
                    dominance: delta.dominance,
                }),
                timestamp: now,
                shareable: false,
                graph_seed: None,
            }
        });

        // 情感反馈 — 梦境情绪反作用于清醒状态 / Emotion feedback — dream emotion affects waking state
        if let Some(ref e) = thought.emotion {
            self.emotion.lock().affect(&EmotionEngineState::new(
                e.pleasure * 0.3, // 梦境情感衰减系数 / Dream emotion decay factor
                e.arousal * 0.3,
                e.dominance * 0.3,
            ));
        }

        // 记录白日梦触发 + 添加思考 / Record daydream trigger + add thought
        {
            let mut engine = self.inner_monologue.lock();
            engine.record_daydream(now);
            engine.add_thought(thought.clone());
        }

        // 存为低置信度 Fact（巩固时可能被遗忘）/ Store as low-confidence Fact
        let fact = Fact::new("Atrium", "梦境", &thought.content)
            .with_confidence(thought.confidence)
            .with_source("Daydream");
        // P1-B: spawn_blocking 包装 FactStore::insert — SQLite 写入不阻塞 reactor
        // P1-B: spawn_blocking wraps FactStore::insert — SQLite write never blocks reactor
        let fact_store = self.fact_store.clone();
        let fact_for_insert = fact.clone();
        let insert_ok = tokio::task::spawn_blocking(move || {
            fact_store.insert(fact_for_insert).unwrap_or(false)
        })
        .await
        .unwrap_or_else(|e| {
            tracing::error!("fact_store.insert spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing", e);
            false
        });
        if insert_ok {
            let mut graph = self.graph.lock();
            graph.add_fact(&fact);
            self.graph_dirty.store(true, Ordering::Relaxed);
        }

        tracing::debug!(
            "[内在独白] Daydream: {}",
            &thought.content.chars().take(50).collect::<String>()
        );
    }

    async fn diary_reflect(&self) {
        // 获取最近日记条目 / Get recent diary entries
        // 获取最近日记条目 / Get recent diary entries
        let diary_entries = if let Some(ref store) = self.diary_store {
            let entries = match store.recent_entries(7) {
                // 最近 7 天 / Last 7 days
                Ok(e) => e,
                Err(_) => return,
            };
            if entries.is_empty() {
                return;
            }
            atrium_memory::monologue_gen::format_diary_entries_for_reflection(&entries)
        } else {
            return;
        };

        // 获取当前洞察 / Get current insights
        let current_insights = {
            let reflection = self.reflection.lock();
            atrium_memory::monologue_gen::format_insights_for_reflection(reflection.all_insights())
        };

        // 获取事实摘要 / Get fact summary
        let fact_summary = {
            // P1-B: spawn_blocking 包装 FactStore::all_facts — O(N) SQLite 读克隆不阻塞 reactor
            // P1-B: spawn_blocking wraps FactStore::all_facts — O(N) SQLite read clone never blocks reactor
            let fact_store = self.fact_store.clone();
            let facts = tokio::task::spawn_blocking(move || fact_store.all_facts())
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("fact_store.all_facts spawn_blocking panic: {} — 数字生命自愈 / digital life self-healing", e);
                    Vec::new()
                });
            facts
                .iter()
                .take(10)
                .map(|f| f.canonical_form())
                .collect::<Vec<_>>()
                .join("\n")
        };

        // 获取当前情感上下文 / Get current emotion context
        let emo = self.emotion.lock().current().clone();
        let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
            pleasure: emo.pleasure,
            arousal: emo.arousal,
            dominance: emo.dominance,
        });

        // 使用 MonologueGenerator 结构化生成 / Use MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            match gen
                .generate_diary_reflection(
                    &diary_entries,
                    &current_insights,
                    &fact_summary,
                    emotion_ctx,
                )
                .await
            {
                Ok(thought) => {
                    // 情感反馈 — 反思情绪反作用于情感 / Emotion feedback — reflection emotion affects emotion
                    let delta =
                        atrium_memory::inner_monologue::analyze_thought_emotion(&thought.content);
                    self.emotion.lock().affect(&EmotionEngineState::new(
                        delta.pleasure * 0.5, // 反思情感衰减 / Reflection emotion decay
                        delta.arousal * 0.5,
                        delta.dominance * 0.5,
                    ));

                    // 解析反思洞察并写入 ReflectionEngine / Parse reflection insights and write to ReflectionEngine
                    let reflection_output =
                        atrium_memory::monologue_gen::make_reflection_output(&thought.content);
                    for insight in &reflection_output.insight_summaries {
                        self.reflection.lock().add_or_update_insight(
                            insight,
                            vec!["DiaryReflection".to_string()],
                            thought.confidence,
                        );
                    }

                    // 添加思考到引擎 / Add thought to engine
                    {
                        let mut engine = self.inner_monologue.lock();
                        engine.add_thought(thought.clone());
                    }

                    // 成长管理器记录 / Maturity record
                    self.maturity_record_inner_thought();

                    tracing::info!(
                        "[内在独白] 日记反思: 洞察={} -> {}",
                        reflection_output.insight_summaries.len(),
                        &thought.content.chars().take(60).collect::<String>()
                    );
                }
                Err(e) => {
                    tracing::warn!("[内在独白] 日记反思 LLM 失败: {}", e);
                }
            }
        }
    }

    pub async fn post_consolidation_reflect(&self, merged_pairs: usize, deprecated_count: usize) {
        if merged_pairs == 0 && deprecated_count == 0 {
            return;
        }

        // 组装日记反思上下文 / Assemble diary reflection context
        // 获取日记条目作为反思上下文 / Get diary entries as reflection context
        let diary_entries = if let Some(ref store) = self.diary_store {
            match store.recent_entries(5) {
                Ok(entries) => {
                    atrium_memory::monologue_gen::format_diary_entries_for_reflection(&entries)
                }
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        // 获取当前洞察 / Get current insights
        let current_insights = {
            let reflection = self.reflection.lock();
            atrium_memory::monologue_gen::format_insights_for_reflection(reflection.all_insights())
        };

        // 获取事实摘要 / Get fact summary
        let fact_summary = format!(
            "记忆巩固：合并 {} 对相似记忆，标记 {} 条过时信息",
            merged_pairs, deprecated_count
        );

        // 获取当前情感上下文 / Get current emotion context
        let emo = self.emotion.lock().current().clone();
        let emotion_ctx = Some(atrium_memory::maturity::EmotionContext {
            pleasure: emo.pleasure,
            arousal: emo.arousal,
            dominance: emo.dominance,
        });

        // 优先使用 MonologueGenerator 结构化生成 / Prefer MonologueGenerator structured generation
        // P1-4: 统一 trait 客户端 → 即时构造 MonologueGenerator / Unified trait client → on-the-fly MonologueGenerator
        let _client_arc = self.llm_client.lock().clone();
        if let Some(client) = _client_arc.as_ref() {
            let gen = atrium_memory::monologue_gen::MonologueGenerator::new(client.clone());
            match gen
                .generate_diary_reflection(
                    &diary_entries,
                    &current_insights,
                    &fact_summary,
                    emotion_ctx,
                )
                .await
            {
                Ok(thought) => {
                    // 情感反馈 / Emotion feedback
                    let delta =
                        atrium_memory::inner_monologue::analyze_thought_emotion(&thought.content);
                    self.emotion.lock().affect(&EmotionEngineState::new(
                        delta.pleasure,
                        delta.arousal,
                        delta.dominance,
                    ));

                    // 解析反思洞察并写入 ReflectionEngine / Parse reflection insights and write to ReflectionEngine
                    let new_insights =
                        atrium_memory::monologue_gen::parse_reflection_insights(&thought.content);
                    for insight in &new_insights {
                        self.reflection.lock().add_or_update_insight(
                            insight,
                            vec!["ConsolidationReflection".to_string()],
                            0.75, // 反思洞察置信度 / Reflection insight confidence
                        );
                    }

                    // 添加思考 / Add thought
                    // 显式作用域：确保 MutexGuard 在 tracing 之前释放 / Explicit scope: ensure MutexGuard drops before tracing
                    {
                        let mut engine = self.inner_monologue.lock();
                        engine.add_thought(thought.clone());
                    }

                    tracing::info!(
                        "[内在独白] 巩固反思: 合并={} 废弃={} 洞察={} -> {}",
                        merged_pairs,
                        deprecated_count,
                        new_insights.len(),
                        &thought.content.chars().take(60).collect::<String>()
                    );
                }
                Err(e) => {
                    tracing::warn!("[内在独白] 巩固反思 LLM 失败: {}", e);
                }
            }
        }
    }

    // ── G3: 独处→对话衔接 / Solitude→Conversation Bridge ──

    /// 用户归来时生成问候 — 自然融入独处期间的思考
    /// Compose return greeting when user returns — naturally weave in solitude thoughts
    ///
    /// 只有独处超过10分钟且有可分享洞察时才生成
    pub fn harvest_solitude_greeting(&self) -> Option<String> {
        let mut engine = self.inner_monologue.lock();
        if !engine.config().solitude_bridge_enabled {
            return None;
        }
        let emo = self.emotion.lock().current().clone();
        engine.bridge.compose_return_greeting(emo.pleasure as f64)
    }

    /// 用户归来时处理独处结束 / Handle end of solitude when user returns
    ///
    /// 1. 结束独处计时 / End solitude timing
    /// 2. 重置氛围调制 / Reset atmosphere modulation
    /// 3. 记录温暖互动 / Record warm interaction
    pub fn on_user_return(&self, idle_secs: u64) {
        let mut engine = self.inner_monologue.lock();
        // G3: 结束独处 / End solitude
        engine.bridge.end_solitude(idle_secs);
        // G4: 重置氛围 / Reset atmosphere
        engine.atmosphere.reset_solitude();
        // G4: 记录温暖互动 / Record warm interaction
        let emo = self.emotion.lock().current().clone();
        engine
            .atmosphere
            .record_warm_interaction(emo.pleasure as f64);
    }

    /// 记录独处期间的可分享思考 / Record a shareable thought during solitude
    pub fn record_solitude_thought(&self, content: &str, shareable: bool) {
        let mut engine = self.inner_monologue.lock();
        if engine.config().solitude_bridge_enabled {
            engine.bridge.record_solitude_thought(content, shareable);
        }
    }

    // ── G4: 温暖互动记录 / Warm Interaction Recording ──

    /// 记录用户消息带来的温暖互动 / Record warm interaction from user message
    pub fn record_warm_interaction(&self, pleasure: f64) {
        let mut engine = self.inner_monologue.lock();
        if engine.config().solitude_atmosphere_enabled {
            engine.atmosphere.record_warm_interaction(pleasure);
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // 内心多元对话 / Inner Dialogue
    // ════════════════════════════════════════════════════════════════════

    /// 内心多元对话周期 tick / Inner dialogue periodic tick.
    ///
    /// 衰减声音活跃度并更新主导声音。不生成对话内容，仅维护状态。
    /// Decays voice activity and updates dominant voice. Does not generate content.
    pub fn inner_dialogue_tick(&self) {
        let mut engine = self.inner_dialogue.lock();
        engine.tick();
    }

    /// 触发内心多元对话生成 / Trigger inner dialogue generation.
    ///
    /// 在独处或情感强烈时调用，生成四声音协商对话。
    /// Called during solitude or intense emotion, generates four-voice negotiation.
    pub fn trigger_inner_dialogue(&self, idle_secs: u64, hour: u32) {
        let now = chrono::Utc::now().timestamp();
        let (pleasure, arousal) = {
            let emo = self.emotion.lock();
            (emo.current().pleasure, emo.current().arousal)
        };

        let mut engine = self.inner_dialogue.lock();
        if !engine.can_trigger(now) {
            return;
        }
        let ctx =
            atrium_memory::inner_dialogue::DialogueContext::new(pleasure, arousal, idle_secs, hour);
        if ctx.should_trigger() {
            engine.generate_dialogue(&ctx, now);
        }
    }

    /// 内心多元对话 prompt 注入 / Inner dialogue prompt injection.
    ///
    /// 将最近一次内心对话摘要注入系统提示。
    /// Injects the latest inner dialogue summary into the system prompt.
    pub fn inner_dialogue_prompt_fragment(&self) -> String {
        let engine = self.inner_dialogue.lock();
        engine.prompt_injection()
    }

    /// 内心多元对话状态 / Inner dialogue status.
    pub fn inner_dialogue_status(&self) -> String {
        let engine = self.inner_dialogue.lock();
        let stats = engine.stats();
        format!(
            "triggers={} history={} dominant={} avg_activity={:.2}",
            stats.total_triggers,
            stats.history_len,
            stats.dominant_voice.label_zh(),
            stats.avg_activity,
        )
    }
} // impl CoreService
