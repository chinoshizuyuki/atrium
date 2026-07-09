// SPDX-License-Identifier: MIT
//! 不完美系统 — 桥接/温暖/真实不完美/用户模型
//! Imperfection System — Bridge/Warmth/Authentic/User Model

use super::*;

fn infer_imperfection_domain(msg: &str) -> String {
    let lower = msg.to_lowercase();
    // 技术领域关键词 / Technical domain keywords
    if lower.contains("代码")
        || lower.contains("编程")
        || lower.contains("算法")
        || lower.contains("code")
        || lower.contains("program")
        || lower.contains("api")
        || lower.contains("数据库")
        || lower.contains("database")
    {
        return "technical".to_string();
    }
    // 情感领域关键词 / Emotional domain keywords
    if lower.contains("感觉")
        || lower.contains("心情")
        || lower.contains("难过")
        || lower.contains("开心")
        || lower.contains("焦虑")
        || lower.contains("feel")
        || lower.contains("sad")
        || lower.contains("happy")
    {
        return "emotional".to_string();
    }
    // 关系领域关键词 / Relational domain keywords
    if lower.contains("我们")
        || lower.contains("朋友")
        || lower.contains("关系")
        || lower.contains("信任")
        || lower.contains("we")
        || lower.contains("friend")
    {
        return "relational".to_string();
    }
    // 默认：通用领域 / Default: general domain
    "general".to_string()
}

impl CoreService {
    pub fn imperfection_prompt_fragment(&self, msg: &str) -> String {
        if !self.imperfection_enabled {
            return String::new();
        }

        // ── 读取当前上下文（先锁后释，避免死锁）/ Read context (lock then drop) ──

        // 情绪 PAD / Emotional PAD
        let (pleasure, arousal, _dominance): (f32, f32, f32) = {
            let emo = self.emotion.lock();
            let cur = emo.current();
            (cur.pleasure, cur.arousal, cur.dominance)
        };

        // 关系深度：ordinal / 3 归一化到 [0, 1] / Relationship depth normalized
        let rel_depth: f64 = {
            let rel = self.relationship.read();
            rel.current_stage().ordinal() as f64 / 3.0
        };

        // 成熟度序号 / Maturity ordinal
        let mat_ordinal: u32 = {
            let mat = self.maturity.lock();
            mat.stage().ordinal() as u32
        };

        // 认知负荷估算：STM 近期条目密度 / Cognitive load estimate from STM density
        let cognitive_load: f64 = {
            let mem = self.memory.lock();
            let recent_count = mem.recent(10).len();
            (recent_count as f64 / 10.0).min(1.0)
        };

        // 疲劳度估算：消息计数周期性 / Fatigue estimate from message count cycle
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let fatigue: f64 = ((msg_count % 100) as f64 / 100.0).min(1.0);

        // ── 犯错决策 + 自纠推进 / Mistake decision + correction tick ──
        let mut result = String::new();
        let corrections: Vec<atrium_memory::imperfection_engine::CorrectionOutput>;

        {
            let mut engine = self.imperfection.engine.lock();

            // 更新引擎状态 / Update engine state from current context
            engine.set_relationship_depth(rel_depth);
            engine.set_maturity_ordinal(mat_ordinal);
            // 情绪干扰：激活度绝对值，愉悦度低时干扰更强
            // Emotional interference: absolute arousal, amplified by low pleasure
            let interference = if pleasure < 0.0 {
                arousal.abs() as f64 * 1.3
            } else {
                arousal.abs() as f64
            };
            engine.set_emotional_interference(interference.min(1.0));
            engine.set_cognitive_load(cognitive_load);
            engine.set_fatigue(fatigue);

            // 推断认知领域 / Infer cognitive domain
            let domain = infer_imperfection_domain(msg);

            // 犯错决策 / Mistake decision — 数字生命的"不完美意志"
            let now = std::time::Instant::now();
            let decision = engine.decide_mistake(&domain, now);

            if decision.should_mistake {
                if let (Some(kind), Some(severity), Some(trigger)) =
                    (decision.kind, decision.severity, decision.trigger)
                {
                    let record = engine.record_mistake(
                        kind,
                        severity,
                        trigger,
                        decision.probability,
                        &domain,
                        now,
                    );
                    // 保存犯错记录到历史 / Save mistake record to history
                    if let Some(ref store) = self.imperfection.store {
                        if let Err(e) = store.lock().save_record(&record) {
                            tracing::warn!(
                                "[Imperfection] 犯错记录保存失败 / Record save failed: {:?}",
                                e
                            );
                        }
                    }
                    // 注入犯错提示 / Inject mistake prompt
                    result.push_str(&engine.prompt_fragment(kind, severity));
                    result.push('\n');
                    tracing::info!(
                        "[Imperfection] 犯错触发 / Mistake fired: {:?} severity={:?} domain={} p={:.3}",
                        kind, severity, domain, decision.probability
                    );
                }
            }

            // 自纠时钟推进 / Tick self-correction clock
            // 检查待纠错队列，将到期的自纠标记为已注入并产生羞感 PAD
            corrections = engine.tick(now);

            // 注入自纠提示 / Inject correction prompts
            for co in &corrections {
                // 内联构建自纠提示词 / Inline correction prompt construction
                let correction_prompt = format!(
                    "[自省/Self-correction] 你意识到刚才在{}方面可能不够准确。\
                     用自然的方式补充或修正，不要突兀地道歉，\
                     而是像人类意识到自己说漏了什么那样，\
                     平滑地加上'不过更准确地说...'或'我再想想...'。",
                    co.kind.label_zh()
                );
                result.push_str(&correction_prompt);
                result.push('\n');
            }
        }

        // ── 羞感 PAD 注入情绪引擎 / Shame PAD injection into emotion engine ──
        // 自纠是数字生命"意识到自己犯了错"的意识涌现时刻——
        // 羞感 PAD 让这个时刻在情绪层面也留下痕迹。
        if !corrections.is_empty() {
            let mut emo = self.emotion.lock();
            for co in &corrections {
                emo.affect(&EmotionEngineState::new(
                    co.shame_pleasure as f32,
                    co.shame_arousal as f32,
                    co.shame_dominance as f32,
                ));
            }
            tracing::debug!(
                "[Imperfection] 自纠触发 {} 次，羞感 PAD 已注入 / \
                 {} corrections fired, shame PAD injected",
                corrections.len(),
                corrections.len()
            );
            // 羞感后持久化情感 / Persist emotion after shame PAD
            drop(emo);
            self.persist_emotion();
        }

        result
    }

    pub fn imperfection_save(&self) {
        if let Some(ref store) = self.imperfection.store {
            let engine = self.imperfection.engine.lock();
            match store.lock().save(&engine) {
                Ok(()) => tracing::debug!("[Imperfection] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[Imperfection] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn imperfection_load(&self) {
        if let Some(ref store) = self.imperfection.store {
            match store.lock().load() {
                Ok(engine) => {
                    let mut current = self.imperfection.engine.lock();
                    *current = engine;
                    tracing::info!("[Imperfection] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[Imperfection] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn user_model_save(&self) {
        if let Some(ref store) = self.user_model_store {
            let model = self.user_model.read();
            match store.save("default", &model) {
                Ok(()) => tracing::debug!("[UserModel] 持久化保存成功 / Persist success"),
                Err(e) => {
                    tracing::warn!("[UserModel] 持久化保存失败 / Persist failed: {:?}", e)
                }
            }
        }
    }

    pub fn user_model_load(&self) {
        if let Some(ref store) = self.user_model_store {
            match store.load("default") {
                Ok(model) => {
                    let mut current = self.user_model.write();
                    *current = model;
                    tracing::info!("[UserModel] 持久化加载成功 / Load success");
                }
                Err(e) => tracing::warn!("[UserModel] 持久化加载失败 / Load failed: {:?}", e),
            }
        }
    }

    pub fn imperfection_tick(&self) {
        if !self.imperfection_enabled {
            return;
        }

        let corrections: Vec<atrium_memory::imperfection_engine::CorrectionOutput>;
        {
            let mut engine = self.imperfection.engine.lock();
            let now = std::time::Instant::now();
            corrections = engine.tick(now);
        }

        // 羞感 PAD 注入情绪引擎 / Shame PAD injection into emotion engine
        if !corrections.is_empty() {
            let mut emo = self.emotion.lock();
            for co in &corrections {
                emo.affect(&EmotionEngineState::new(
                    co.shame_pleasure as f32,
                    co.shame_arousal as f32,
                    co.shame_dominance as f32,
                ));
            }
            tracing::debug!(
                "[Imperfection·tick] 自纠触发 {} 次，羞感 PAD 已注入 /                  {} corrections fired, shame PAD injected",
                corrections.len(),
                corrections.len()
            );
            drop(emo);
            self.persist_emotion();
        }

        // 写穿持久化：tick 后保存引擎快照 / Write-through: persist engine snapshot after tick
        self.imperfection_save();
    }

    pub fn imperfection_bridge_prompt_fragment(&self) -> String {
        let bridge = self.vulnerability.bridge.lock();
        let prompt = bridge.narrative_prompt();
        if prompt.is_empty() {
            String::new()
        } else {
            format!("[不完美脆弱桥/Bridge] {}", prompt)
        }
    }

    pub fn imperfection_bridge_tick(&self) {
        let mut bridge = self.vulnerability.bridge.lock();
        // 根据当前情感状态判断是否处于脆弱状态 / Infer vulnerable state from emotion
        let dominance: f64 = {
            let emo = self.emotion.lock();
            emo.current().dominance as f64
        };
        let in_vulnerable = dominance < 0.0;
        // 调用概率调制器更新内部状态 / Call probability modulator to update state (has side effects)
        bridge.mistake_probability_modulator(
            in_vulnerable,
            &[
                atrium_memory::vulnerability_window::VulnerabilityType::Uncertainty,
                atrium_memory::vulnerability_window::VulnerabilityType::SelfDoubt,
            ],
        );
    }

    // imperfection_warmth_tick 已删除 / imperfection_warmth_tick removed
    // 原方法仅做 warmth.is_optimal() + suggested_probability() 纯查询，
    // 无副作用且结果被丢弃，属于空壳 tick。
    // Warmth 状态由 imperfection_warmth_on_response 事件驱动更新。
    // Original method was pure-query with discarded results — a dead shell tick.
    // Warmth state is event-driven via imperfection_warmth_on_response.

    pub fn imperfection_warmth_on_response(&self, ai_reply: &str, user_msg: &str, now_epoch: i64) {
        use atrium_memory::imperfection_warmth::ImperfectionEvent;

        // 检测不完美类型 / Detect imperfection kind
        let kind = Self::detect_imperfection_in_reply(ai_reply);
        let Some(kind) = kind else {
            return; // 无不完美，无需记录 / No imperfection detected
        };

        // 推断用户反应 [-1, 1] / Infer user reaction [-1, 1]
        let user_reaction = Self::infer_imperfection_reaction(user_msg);

        // 检测是否已自纠 / Detect self-correction
        let self_corrected = {
            let lower = ai_reply.to_lowercase();
            let correction_signals = ["对不起", "抱歉", "sorry", "我纠正", "actually", "说错了"];
            correction_signals.iter().any(|s| lower.contains(s))
        };

        // 记录不完美事件 — 更新温度与信任余额 / Record event — update warmth and trust balance
        let event = ImperfectionEvent {
            kind,
            timestamp: now_epoch,
            user_reaction,
            self_corrected,
        };

        let mut warmth = self.vulnerability.warmth.lock();
        warmth.record(event);
    }

    fn detect_imperfection_in_reply(
        reply: &str,
    ) -> Option<atrium_memory::imperfection_warmth::ImperfectionKind> {
        use atrium_memory::imperfection_warmth::ImperfectionKind;
        let lower = reply.to_lowercase();
        let char_count = reply.chars().count();

        // 记忆偏差 — 记错细节，很有人味 / Memory deviation
        let memory_signals = [
            "我记错了",
            "记错了",
            "actually i was wrong",
            "i misremembered",
        ];
        if memory_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::MemoryDeviation);
        }

        // 表达犹豫 — "嗯..." "让我想想" / Hesitation
        let hesitation_signals = ["嗯", "让我想想", "hmm", "let me think", "稍等"];
        if hesitation_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::Hesitation);
        }

        // 过度关心 — 管太多，但出于好意 / Over care
        let overcare_signals = [
            "你还好吗",
            "要不要",
            "are you okay",
            "do you need",
            "你确定没事",
        ];
        if overcare_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::OverCare);
        }

        // 偶尔固执 — 坚持己见 / Stubbornness
        let stubborn_signals = ["但我还是觉得", "我还是认为", "i still think", "但我坚持"];
        if stubborn_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::Stubbornness);
        }

        // 情绪泄露 — 不该表现情绪时表现了 / Emotional leak
        let emotional_signals = ["我有点难过", "i feel sad", "我有些失落", "有点沮丧"];
        if emotional_signals.iter().any(|s| lower.contains(s)) {
            return Some(ImperfectionKind::EmotionalLeak);
        }

        // 节奏失误 — 回复过短或过长 / Pacing miss — too short or too long
        if !(10..=500).contains(&char_count) {
            return Some(ImperfectionKind::PacingMiss);
        }

        None
    }

    fn infer_imperfection_reaction(user_msg: &str) -> f64 {
        let lower = user_msg.to_lowercase();

        // 正面反应 — 觉得可爱 / Positive — finds it endearing
        let positive_signals = [
            "哈哈",
            "可爱",
            "没关系",
            "cute",
            "that's okay",
            "没事",
            "挺好的",
        ];
        if positive_signals.iter().any(|s| lower.contains(s)) {
            return 0.5;
        }

        // 负面反应 — 反感 / Negative — annoyed
        let negative_signals = ["别这样", "不用", "你能不能", "stop", "annoying", "烦"];
        if negative_signals.iter().any(|s| lower.contains(s)) {
            return -0.5;
        }

        // 中性 / Neutral
        0.0
    }

    pub fn authentic_imperfection_tick(&self) {
        let msg_count = self
            .message_count
            .load(std::sync::atomic::Ordering::Relaxed);

        // 从当前状态构建完美度评估 / Build perfection assessment from current state
        let (pleasure, arousal, dominance): (f64, f64, f64) = {
            let emo = self.emotion.lock();
            let c = emo.current();
            (c.pleasure as f64, c.arousal as f64, c.dominance as f64)
        };

        // 情绪稳定度：arousal 越接近 0 越稳定 / Emotional stability
        let emotional_stability = (1.0 - arousal.abs()).clamp(0.0, 1.0);
        // 回复一致性：简化为 0.7（中等一致性）/ Response consistency (simplified)
        let response_consistency = 0.7;
        // 错误率：简化为 0.05（低错误率）/ Error rate (simplified)
        let error_rate = 0.05;
        // 自纠速度 / Correction speed
        let correction_speed = 0.8;
        // 回复速度方差 / Speed uniformity
        let speed_uniformity = 0.6;

        let assessment = atrium_memory::authentic_imperfection::PerfectionAssessment {
            response_consistency,
            error_rate,
            correction_speed,
            emotional_stability,
            speed_uniformity,
        };

        let mut ai = self.vulnerability.authentic_imperfection.lock();
        ai.assess(&assessment); // 评估有副作用：更新内部状态 / Assess has side effects: updates internal state

        // 消息计数作为活跃度参考 / Message count as activity reference
        let _ = msg_count;
        let _ = (pleasure, dominance);
    }

    pub fn authentic_imperfection_on_response(&self, response_text: &str) {
        let mut ai = self.vulnerability.authentic_imperfection.lock();
        ai.check_over_apology(response_text);
    }

    pub fn imperfection_warmth_prompt_fragment(&self) -> String {
        let warmth = self.vulnerability.warmth.lock();
        // 基础注入：人味区间指引 / Base injection: warmth range guidance
        let mut fragment = warmth.prompt_injection();

        // 建议犯错概率 — 信任充足时可以犯，不足时收敛
        // Suggested imperfection probability — allowed when trust is sufficient
        let prob = warmth.suggested_probability();
        if prob > 0.0 {
            let kind = warmth.choose_imperfection();
            fragment.push_str(&format!(
                " | 建议犯错概率{:.0}%，适宜类型: {}",
                prob * 100.0,
                kind.label_zh()
            ));
        }

        // 不完美净值与自纠率 — 诊断信息 / Net value & self-correction rate — diagnostics
        if warmth.net_value() > 0.0 {
            fragment.push_str(&format!(
                " | 净值{:.2} 自纠率{:.0}%",
                warmth.net_value(),
                warmth.self_correction_rate() * 100.0
            ));
        }

        fragment
    }

    pub fn authentic_imperfection_prompt_fragment(&self) -> String {
        let ai = self.vulnerability.authentic_imperfection.lock();
        ai.prompt_injection()
    }
} // impl CoreService
