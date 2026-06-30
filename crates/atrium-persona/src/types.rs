// SPDX-License-Identifier: MIT
//! 人格类型定义 — 角色卡数据结构
//! Persona type definitions — Character card data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 完整角色卡定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDef {
    /// 人格名称
    pub name: String,
    /// 描述（角色设定）
    pub description: String,
    /// 核心特质：特质名 → 强度 [0.0, 1.0]
    pub traits: HashMap<String, f32>,
    /// 默认情绪基准
    pub mood_defaults: MoodParams,
    /// 对话风格
    pub speaking_style: SpeakingStyle,
    /// 知识领域：领域名 → 熟练度 [0.0, 1.0]
    pub knowledge_areas: HashMap<String, f32>,
}

/// PAD 情绪基准参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodParams {
    /// 愉悦度基准 [-1, 1]
    pub base_pleasure: f32,
    /// 激活度基准 [-1, 1]
    pub base_arousal: f32,
    /// 支配度基准 [-1, 1]
    pub base_dominance: f32,
    /// 情绪波动性 [0, 1] — 越高越容易受外界影响
    pub volatility: f32,
}

/// 说话风格参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakingStyle {
    /// 正式度 [0, 1] — 0=随意, 1=严谨
    pub formality: f32,
    /// 健谈度 [0, 1] — 0=沉默, 1=话多
    pub verbosity: f32,
    /// 共情度 [0, 1] — 0=冷漠, 1=体贴
    pub empathy: f32,
    /// 幽默感 [0, 1] — 0=严肃, 1=爱开玩笑
    pub humor: f32,
}

impl Default for SpeakingStyle {
    fn default() -> Self {
        Self {
            formality: 0.3,
            verbosity: 0.6,
            empathy: 0.8,
            humor: 0.4,
        }
    }
}

impl Default for MoodParams {
    fn default() -> Self {
        Self {
            base_pleasure: 0.3,
            base_arousal: 0.2,
            base_dominance: -0.1,
            volatility: 0.3,
        }
    }
}

/// 运行时人格实例
#[derive(Debug, Clone)]
pub struct PersonaInstance {
    pub def: PersonaDef,
    /// 当前情绪偏移（对话中累积）
    pub mood_offset: MoodParams,
    /// 交互计数
    pub interaction_count: u64,
}

impl PersonaInstance {
    pub fn new(def: PersonaDef) -> Self {
        Self {
            mood_offset: MoodParams {
                base_pleasure: 0.0,
                base_arousal: 0.0,
                base_dominance: 0.0,
                volatility: 0.0,
            },
            def,
            interaction_count: 0,
        }
    }

    /// 获取当前情绪状态（基准 + 偏移，clamp 到 [-1, 1]）
    pub fn current_mood(&self) -> (f32, f32, f32) {
        (
            (self.def.mood_defaults.base_pleasure + self.mood_offset.base_pleasure)
                .clamp(-1.0, 1.0),
            (self.def.mood_defaults.base_arousal + self.mood_offset.base_arousal).clamp(-1.0, 1.0),
            (self.def.mood_defaults.base_dominance + self.mood_offset.base_dominance)
                .clamp(-1.0, 1.0),
        )
    }

    /// 应用情绪偏移（由情感引擎触发）
    pub fn apply_mood_shift(
        &mut self,
        delta_pleasure: f32,
        delta_arousal: f32,
        delta_dominance: f32,
    ) {
        self.mood_offset.base_pleasure += delta_pleasure * self.def.mood_defaults.volatility;
        self.mood_offset.base_arousal += delta_arousal * self.def.mood_defaults.volatility;
        self.mood_offset.base_dominance += delta_dominance * self.def.mood_defaults.volatility;
        self.interaction_count += 1;
    }
}
