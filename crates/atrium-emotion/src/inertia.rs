// SPDX-License-Identifier: MIT
// EmotionalInertia — 情绪惯性 / Emotional inertia

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::EmotionState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InertiaModifiers {
    pub sensitivity: f32,
    pub decay_rate: f32,
    pub expression_threshold: f32,
}

impl Default for InertiaModifiers {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            decay_rate: 1.0,
            expression_threshold: 0.0,
        }
    }
}

/// 情感惯性追踪器
///
/// 追踪持续主导情绪，超过阈值后激活惯性修正。
#[derive(Clone, Debug)]
pub struct EmotionalInertia {
    pub(crate) history: VecDeque<[f32; 3]>,
    capacity: usize,
    activation_ticks: usize,
    pub(crate) dominant_duration: usize,
    pub(crate) dominant_label: Option<String>,
    pub modifiers: InertiaModifiers,
    max_sensitivity: f32,
    min_decay_rate: f32,
}

impl Default for EmotionalInertia {
    fn default() -> Self {
        Self {
            history: VecDeque::new(),
            capacity: 500,        // 500 ticks ≈ 100s @ 200ms/tick
            activation_ticks: 50, // 50 ticks ≈ 10s 激活阈值
            dominant_duration: 0,
            dominant_label: None,
            modifiers: InertiaModifiers::default(),
            max_sensitivity: 1.5,
            min_decay_rate: 0.85,
        }
    }
}

impl EmotionalInertia {
    pub fn new() -> Self {
        Self::default()
    }

    /// 每次 tick 调用，更新历史并重新计算修正器
    pub fn tick(&mut self, pad: [f32; 3]) {
        self.history.push_back(pad);
        if self.history.len() > self.capacity {
            self.history.pop_front();
        }
        self.update_modifiers();
    }

    /// 根据历史记录更新修正器 / Update modifiers based on history.
    ///
    /// 热路径优化：O(A²)→O(A) — 用 HashMap 计频替代嵌套遍历。
    /// Hot-path optimization: O(A²)→O(A) — HashMap frequency counting replaces nested iteration.
    /// 情感惯性是情绪的粘滞记忆——O(A)让粘滞计算不成为每tick的负担。
    /// Emotional inertia is the sticky memory of emotion — O(A) makes sticky computation
    /// not a per-tick burden.
    fn update_modifiers(&mut self) {
        if self.history.len() < self.activation_ticks {
            self.modifiers = InertiaModifiers::default();
            return;
        }

        // 情绪标签计频 / Emotion label frequency counting — O(A) 单次遍历
        let mut freq: HashMap<String, usize> = HashMap::new();
        for pad in self.history.iter().rev().take(self.activation_ticks) {
            let label = EmotionState::new(pad[0], pad[1], pad[2])
                .classify()
                .name
                .to_string();
            *freq.entry(label).or_insert(0) += 1;
        }

        // 众数查找 / Mode finding — O(K), K ≤ 9 种基本情绪
        let (dominant, count) = freq.into_iter().max_by_key(|(_, c)| *c).unwrap_or_default();
        let ratio = count as f32 / self.activation_ticks as f32;

        // 如果超过 60% 的时间都是同一情绪 → 激活惯性
        if ratio > 0.6 {
            self.dominant_duration += 1;

            let factor =
                ((self.dominant_duration as f32 / self.activation_ticks as f32) - 1.0).max(0.0);

            // 敏感度升高（最高 1.5 倍）
            self.modifiers.sensitivity = (1.0 + factor * 0.1).min(self.max_sensitivity);
            // 衰减率降低（情绪持续更久，最低 0.85 倍）
            self.modifiers.decay_rate = (1.0 - factor * 0.05).max(self.min_decay_rate);
            // 表达阈值降低（更容易触发情绪表达）
            self.modifiers.expression_threshold = -(factor * 0.02).max(-0.1);
            self.dominant_label = Some(dominant);
        } else {
            // 情绪多样化 → 惯性重置
            self.dominant_duration = 0;
            self.dominant_label = None;
            self.modifiers = InertiaModifiers::default();
        }
    }

    pub fn dominant_label(&self) -> Option<&str> {
        self.dominant_label.as_deref()
    }

    pub fn modifiers(&self) -> &InertiaModifiers {
        &self.modifiers
    }
}
