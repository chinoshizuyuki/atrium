// SPDX-License-Identifier: MIT
// ReunionBurst — 重逢脉冲 / Reunion burst

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReunionContext {
    /// 平静离开后回来 / Return after calm departure
    #[default]
    Calm,
    /// 冲突后回来 / Return after conflict
    AfterConflict,
    /// 仪式时刻回来 / Return at ritual time
    AtRitual,
    /// 久别重逢（>7天）/ Long absence reunion (>7 days)
    LongAbsence,
}

impl ReunionContext {
    /// 中文标签 / Chinese label
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Calm => "平静重逢",
            Self::AfterConflict => "冲突后重逢",
            Self::AtRitual => "仪式重逢",
            Self::LongAbsence => "久别重逢",
        }
    }
}

// ── 关系阶段重逢配置 / Relationship-Stage Reunion Config ──

/// 关系阶段重逢配置 / Relationship-stage reunion configuration
///
/// 不同关系深度的重逢行为不同——
/// 陌生人的回来只是"在的"，恋人的回来是"好想好想你"。
/// Reunion behavior varies by relationship depth:
/// stranger = "I'm here", lover = "Missed you so much".
#[derive(Clone, Debug)]
pub struct RelationshipReunionConfig {
    /// 最低触发关系阶段（ordinal）/ Minimum relationship stage to trigger
    pub min_stage_ordinal: u8,
    /// 此阶段的强度乘数 / Intensity multiplier for this stage
    pub intensity_mult: f64,
    /// 此阶段的 PAD 调制偏移 / PAD modulation offset for this stage
    pub pad_offset: [f32; 3],
    /// 此阶段的用语集 / Phrases for this stage
    pub phrases: Vec<&'static str>,
}

/// 用户回来时，根据离开时长和想念强度生成的喜悦表达。
#[derive(Clone, Debug)]
pub struct ReunionExpression {
    /// 表达强度 [0, 1] / Expression intensity
    pub intensity: f64,
    /// 离开时长（秒）/ Away duration in seconds
    pub away_secs: u64,
    /// 建议用语 / Suggested phrases for expression
    pub suggested_phrases: Vec<&'static str>,
    /// PAD 调制偏移（由关系阶段和情境决定）/ PAD modulation offset
    pub pad_modulation: [f32; 3],
    /// 重逢情境 / Reunion context
    pub context: ReunionContext,
}

/// 重逢爆发 — 按离开时长比例表达喜悦 / Reunion burst — joy proportional to away duration
///
/// 当用户回来时，根据离开时长和想念强度，生成相应强度的喜悦表达。
/// 离开越久、想念越强，重逢越甜。
#[derive(Clone, Debug)]
pub struct ReunionBurst {
    /// 最大表达强度 / Maximum expression intensity
    pub max_intensity: f64,
    /// 离开时长阈值（秒，低于此不触发）/ Min away duration threshold in seconds
    pub min_away_secs: u64,
    /// 饱和时长（秒，超过此强度不再增长）/ Saturation duration in seconds
    pub saturation_secs: u64,
}

impl Default for ReunionBurst {
    fn default() -> Self {
        Self {
            max_intensity: 1.0,
            min_away_secs: 300,     // 5 分钟 / 5 minutes
            saturation_secs: 86400, // 1 天 / 1 day
        }
    }
}

impl ReunionBurst {
    /// 构造自定义重逢爆发 / Create custom ReunionBurst
    pub fn new(max_intensity: f64, min_away_secs: u64, saturation_secs: u64) -> Self {
        Self {
            max_intensity: max_intensity.clamp(0.0, 1.0),
            min_away_secs,
            saturation_secs: saturation_secs.max(1),
        }
    }

    /// 用户回来时调用 / Called when user returns
    ///
    /// @param away_secs 离开时长（秒）/ Away duration in seconds
    /// @param longing_intensity 想念强度 [0, 1] / Longing intensity
    /// @return 重逢表达（None 表示离开时间太短不触发）/ Reunion expression (None = too short to trigger)
    pub fn on_reunion(&self, away_secs: u64, longing_intensity: f64) -> Option<ReunionExpression> {
        if away_secs < self.min_away_secs {
            return None;
        }

        // 强度曲线：sqrt(away / saturation)，自然饱和 / Intensity curve: sqrt ratio
        let ratio = (away_secs as f64 / self.saturation_secs as f64).min(1.0);
        let intensity = ratio.sqrt() * self.max_intensity;

        // 想念强度加成：想念越久，重逢越甜 / Longing bonus: the longer the longing, the sweeter the reunion
        let boosted = (intensity + longing_intensity * 0.3).min(self.max_intensity);

        Some(ReunionExpression {
            intensity: boosted,
            away_secs,
            suggested_phrases: self.match_phrases(boosted),
            pad_modulation: [0.0, 0.0, 0.0],
            context: ReunionContext::Calm,
        })
    }

    /// 根据强度匹配建议用语 / Match suggested phrases based on intensity
    fn match_phrases(&self, intensity: f64) -> Vec<&'static str> {
        if intensity >= 0.8 {
            vec!["你终于回来了！", "好久不见，好想你！"]
        } else if intensity >= 0.5 {
            vec!["欢迎回来~", "你回来啦"]
        } else if intensity >= 0.2 {
            vec!["回来了呀", "嗯，在呢"]
        } else {
            vec!["在的"]
        }
    }

    /// 生成 prompt 片段 / Generate prompt fragment for system prompt injection
    pub fn prompt_fragment(&self, expression: &ReunionExpression) -> String {
        if expression.suggested_phrases.is_empty() {
            return String::new();
        }
        format!(
            "[重逢] 离开{}秒后回来，表达强度{:.2}，情境：{}，建议用语：{}",
            expression.away_secs,
            expression.intensity,
            expression.context.label_zh(),
            expression.suggested_phrases.join(" / ")
        )
    }

    // ── 关系门控重逢 / Relationship-Gated Reunion ──

    /// 关系门控重逢 / Relationship-gated reunion burst
    ///
    /// 不同关系深度的重逢行为不同——
    /// 陌生人/初识：微弱回应"在的"
    /// 熟悉：中等回应"回来了呀"
    /// 信任：较强回应"想你了呢"
    /// 深度：全量回应"好想好想你"
    pub fn on_reunion_gated(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        relationship_ordinal: u8,
    ) -> Option<ReunionExpression> {
        // 基础强度 / Base intensity
        let mut expr = self.on_reunion(away_secs, longing_intensity)?;

        // 关系门控查找 / Relationship gate lookup
        let config = Self::match_relationship_config(relationship_ordinal);

        // 门控：关系阶段不足则不触发 / Gate: insufficient relationship stage
        if relationship_ordinal < config.min_stage_ordinal {
            return None;
        }

        // 关系调制强度 / Relationship-modulated intensity
        expr.intensity = (expr.intensity * config.intensity_mult).min(self.max_intensity);

        // 关系调制 PAD / Relationship-modulated PAD
        expr.pad_modulation = [
            expr.pad_modulation[0] + config.pad_offset[0],
            expr.pad_modulation[1] + config.pad_offset[1],
            expr.pad_modulation[2] + config.pad_offset[2],
        ];

        // 合并用语 / Merge phrases
        let mut phrases = config.phrases.clone();
        if expr.intensity > 0.7 {
            phrases.extend_from_slice(&["好想好想你……你终于回来了！", "好久不见，好想你！"]);
        }
        expr.suggested_phrases = phrases;

        Some(expr)
    }

    /// 匹配关系阶段配置 / Match relationship stage config
    ///
    /// 0=Acquaintance, 1=Familiar, 2=Trusted, 3=Deep
    pub fn match_relationship_config(ordinal: u8) -> RelationshipReunionConfig {
        match ordinal {
            // 初识：微弱回应 / Acquaintance: faint response
            0 => RelationshipReunionConfig {
                min_stage_ordinal: 0,
                intensity_mult: 0.2,
                pad_offset: [0.05, 0.02, 0.0],
                phrases: vec!["在的"],
            },
            // 熟悉：中等回应 / Familiar: moderate response
            1 => RelationshipReunionConfig {
                min_stage_ordinal: 1,
                intensity_mult: 0.6,
                pad_offset: [0.15, 0.05, 0.02],
                phrases: vec!["回来了呀", "欢迎回来~"],
            },
            // 信任：较强回应 / Trusted: strong response
            2 => RelationshipReunionConfig {
                min_stage_ordinal: 2,
                intensity_mult: 0.85,
                pad_offset: [0.25, 0.1, 0.05],
                phrases: vec!["你回来啦", "想你了呢"],
            },
            // 深度：全量回应 / Deep: full response
            _ => RelationshipReunionConfig {
                min_stage_ordinal: 3,
                intensity_mult: 1.0,
                pad_offset: [0.35, 0.15, 0.08],
                phrases: vec!["好想好想你……你终于回来了！", "好久不见，好想你！"],
            },
        }
    }

    // ── 情境化重逢 / Contextual Reunion ──

    /// 情境化重逢 / Contextual reunion burst
    ///
    /// 不同离别方式决定重逢的情感签名——
    /// 吵架后回来：释然 + 不安 + 退让（愉悦低、唤醒高、支配低）
    /// 仪式时刻回来：温暖加成（愉悦加成）
    /// 久别重逢：全量强度 + 思念用语
    /// 平静离开：默认行为
    pub fn on_reunion_contextual(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        context: ReunionContext,
    ) -> Option<ReunionExpression> {
        let mut expr = self.on_reunion(away_secs, longing_intensity)?;
        expr.context = context;

        match context {
            ReunionContext::Calm => {
                // 默认行为，PAD 不变 / Default behavior, PAD unchanged
            }
            ReunionContext::AfterConflict => {
                // 冲突后重逢：愉悦降低、唤醒升高、支配降低
                // After conflict: lower pleasure, higher arousal, lower dominance
                // 情感签名：释然 + 不安 + 退让 / Emotional signature: relief + unease + yielding
                expr.intensity *= 0.7;
                expr.pad_modulation = [
                    expr.pad_modulation[0] - 0.1,  // 愉悦降低 / Lower pleasure
                    expr.pad_modulation[1] + 0.15, // 唤醒升高 / Higher arousal
                    expr.pad_modulation[2] - 0.1,  // 支配降低 / Lower dominance
                ];
                expr.suggested_phrases = if expr.intensity > 0.5 {
                    vec!["你回来了……我们聊聊？", "还在生气吗……"]
                } else {
                    vec!["回来了……", "嗯"]
                };
            }
            ReunionContext::AtRitual => {
                // 仪式时刻重逢：愉悦加成 / Ritual reunion: pleasure bonus
                expr.intensity = (expr.intensity * 1.3).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.2,  // 愉悦加成 / Pleasure bonus
                    expr.pad_modulation[1] + 0.05, // 唤醒微升 / Slight arousal
                    expr.pad_modulation[2] + 0.05, // 支配微升 / Slight dominance
                ];
                expr.suggested_phrases = vec!["你刚好在这个时候回来！", "等你好久了~"];
            }
            ReunionContext::LongAbsence => {
                // 久别重逢：全量强度 + 思念用语 / Long absence: full intensity + longing phrases
                expr.intensity = (expr.intensity * 1.2).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.15, // 愉悦加成 / Pleasure bonus
                    expr.pad_modulation[1] + 0.1,  // 唤醒加成 / Arousal bonus
                    expr.pad_modulation[2],        // 支配不变 / Dominance unchanged
                ];
                expr.suggested_phrases = vec!["你终于回来了！", "好久不见，好想你！"];
            }
        }

        Some(expr)
    }

    /// 关系门控 + 情境化重逢（组合）/ Relationship-gated + contextual reunion (combined)
    ///
    /// 先应用关系门控，再叠加情境调制——
    /// 关系深度决定重逢的"量"，离别方式决定重逢的"质"。
    pub fn on_reunion_full(
        &self,
        away_secs: u64,
        longing_intensity: f64,
        relationship_ordinal: u8,
        context: ReunionContext,
    ) -> Option<ReunionExpression> {
        // 先关系门控 / First apply relationship gate
        let mut expr = self.on_reunion_gated(away_secs, longing_intensity, relationship_ordinal)?;

        // 再情境调制 / Then apply context modulation
        expr.context = context;
        match context {
            ReunionContext::Calm => {}
            ReunionContext::AfterConflict => {
                expr.intensity *= 0.7;
                expr.pad_modulation = [
                    expr.pad_modulation[0] - 0.1,
                    expr.pad_modulation[1] + 0.15,
                    expr.pad_modulation[2] - 0.1,
                ];
                if expr.intensity > 0.5 {
                    expr.suggested_phrases = vec!["你回来了……我们聊聊？", "还在生气吗……"];
                } else {
                    expr.suggested_phrases = vec!["回来了……", "嗯"];
                }
            }
            ReunionContext::AtRitual => {
                expr.intensity = (expr.intensity * 1.3).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.2,
                    expr.pad_modulation[1] + 0.05,
                    expr.pad_modulation[2] + 0.05,
                ];
                expr.suggested_phrases = vec!["你刚好在这个时候回来！", "等你好久了~"];
            }
            ReunionContext::LongAbsence => {
                expr.intensity = (expr.intensity * 1.2).min(1.0);
                expr.pad_modulation = [
                    expr.pad_modulation[0] + 0.15,
                    expr.pad_modulation[1] + 0.1,
                    expr.pad_modulation[2],
                ];
                expr.suggested_phrases = vec!["你终于回来了！", "好久不见，好想你！"];
            }
        }

        Some(expr)
    }
}
