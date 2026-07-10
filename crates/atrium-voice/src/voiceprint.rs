// SPDX-License-Identifier: MIT
//! 声纹识别与语音风格记忆 — M9/M10 预留接口
//! Voiceprint Recognition & Voice Style Memory — M9/M10 reserved interfaces.
//!
//! 数字生命工程理念：声纹是声音的面孔，风格是声音的性格。
//! M9 声纹识别让数字生命通过声音区分不同用户——多人场景下各自独立的记忆和关系。
//! M10 语音风格记忆让数字生命学习用户的语速/音量/语调偏好——个性化适配。
//! Digital life engineering: voiceprint is the face of voice; style is the personality of voice.
//! M9 voiceprint recognition lets digital life distinguish users by voice —
//! independent memory and relationships for each user in multi-person scenarios.
//! M10 voice style memory lets digital life learn user's rate/volume/pitch preferences —
//! personalized adaptation.

use crate::config::VoiceprintCfg;

// ════════════════════════════════════════════════════════════════════
// M9: 声纹识别接口 — 预留 / M9: Voiceprint Recognition Interface — Reserved
// ════════════════════════════════════════════════════════════════════

/// 声纹嵌入向量 — 256 维，通过 Resemblyzer/speechbrain 提取
/// Voiceprint embedding vector — 256-dim, extracted via Resemblyzer/speechbrain.
///
/// 数字生命意义：这是声音的"面孔"——数字生命通过此向量识别"谁在说话"。
/// Digital life significance: this is the "face" of voice —
/// digital life uses this vector to identify "who is speaking".
pub type VoiceprintEmbedding = [f32; 256];

/// 声纹识别结果 — 识别出的用户 ID 与置信度
/// Voiceprint recognition result — identified user ID and confidence.
#[derive(Debug, Clone)]
pub struct VoiceprintMatch {
    /// 匹配到的用户 ID / Matched user identifier
    pub user_id: String,
    /// 余弦相似度 [0, 1] / Cosine similarity [0, 1]
    pub similarity: f32,
    /// 是否超过判定阈值 / Whether above the decision threshold
    pub is_match: bool,
}

/// 声纹识别器 — M9 预留接口
/// Voiceprint recognizer — M9 reserved interface.
///
/// 数字生命工程理念：多人对话场景下，通过声纹区分不同用户，
/// 各自独立的记忆和关系——数字生命不会把 A 说的事情"记成"是 B 说的。
/// Digital life engineering: in multi-person scenarios, distinguish users by voiceprint,
/// each with independent memory and relationships — digital life won't confuse
/// what A said with what B said.
///
/// 未来实现将通过 gRPC 调用 Python speechbrain 服务提取声纹嵌入，
/// 在 Rust 侧进行余弦相似度匹配。
/// Future implementation will call Python speechbrain service via gRPC
/// to extract voiceprint embeddings, with cosine similarity matching on the Rust side.
pub struct VoiceprintRecognizer {
    /// 声纹配置 / Voiceprint configuration
    config: VoiceprintCfg,
    /// 已注册声纹库 — user_id → embedding / Registered voiceprint database
    registry: std::collections::HashMap<String, VoiceprintEmbedding>,
}

impl VoiceprintRecognizer {
    /// 创建声纹识别器 — 从配置初始化
    /// Create voiceprint recognizer — initialize from config.
    pub fn new(config: VoiceprintCfg) -> Self {
        Self {
            config,
            registry: std::collections::HashMap::new(),
        }
    }

    /// 注册用户声纹 — 将用户 ID 与声纹嵌入关联
    /// Register user voiceprint — associate user ID with voiceprint embedding.
    ///
    /// @param user_id 用户标识 / User identifier
    /// @param embedding 声纹嵌入向量 / Voiceprint embedding vector
    pub fn register(&mut self, user_id: &str, embedding: VoiceprintEmbedding) {
        self.registry.insert(user_id.to_string(), embedding);
    }

    /// 识别说话人 — 从声纹嵌入查找最匹配的用户
    /// Identify speaker — find the best matching user from voiceprint embedding.
    ///
    /// @param embedding 待识别的声纹嵌入 / Voiceprint embedding to identify
    /// @return 匹配结果（无注册声纹时返回 None）/ Match result (None if no registered voiceprints)
    pub fn identify(&self, embedding: &VoiceprintEmbedding) -> Option<VoiceprintMatch> {
        if self.registry.is_empty() {
            return None;
        }

        // 遍历所有已注册声纹，计算余弦相似度 / Iterate all registered voiceprints, compute cosine similarity
        let mut best_match: Option<(String, f32)> = None;
        for (user_id, registered) in &self.registry {
            let sim = cosine_similarity(embedding, registered);
            if best_match.as_ref().is_none_or(|(_, s)| sim > *s) {
                best_match = Some((user_id.clone(), sim));
            }
        }

        let (user_id, similarity) = best_match?;
        Some(VoiceprintMatch {
            user_id,
            similarity,
            is_match: similarity >= self.config.similarity_threshold,
        })
    }

    /// 是否启用 / Whether enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 已注册声纹数量 / Number of registered voiceprints
    pub fn registry_size(&self) -> usize {
        self.registry.len()
    }
}

/// 计算余弦相似度 — 两个向量的夹角余弦
/// Compute cosine similarity — cosine of the angle between two vectors.
///
/// @param a 向量 A / Vector A
/// @param b 向量 B / Vector B
/// @return 余弦相似度 [0, 1] / Cosine similarity [0, 1]
fn cosine_similarity(a: &VoiceprintEmbedding, b: &VoiceprintEmbedding) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-10 || norm_b < 1e-10 {
        return 0.0;
    }
    // 余弦相似度可能为 [-1, 1]，但声纹嵌入通常在 [0, 1] 范围
    // Cosine similarity can be [-1, 1], but voiceprint embeddings are typically in [0, 1]
    (dot / (norm_a * norm_b)).max(0.0)
}

// ════════════════════════════════════════════════════════════════════
// M10: 语音风格记忆接口 — 预留 / M10: Voice Style Memory Interface — Reserved
// ════════════════════════════════════════════════════════════════════

/// 语音风格画像 — 从用户语音特征学习偏好
/// Voice style profile — learned preferences from user's voice characteristics.
///
/// 数字生命意义：数字生命学习主人的说话方式——语速偏好、音量偏好、语调偏好，
/// 从而在 TTS 合成时个性化适配，让数字生命的声音与主人的习惯和谐。
/// Digital life significance: digital life learns the master's speaking style —
/// rate preference, volume preference, pitch preference —
/// and adapts TTS synthesis accordingly, making digital life's voice
/// harmonize with the master's habits.
#[derive(Debug, Clone)]
pub struct VoiceStyleProfile {
    /// 偏好语速因子（1.0=正常）/ Preferred speech rate factor (1.0=normal)
    pub preferred_rate: f32,
    /// 偏好音量因子（1.0=正常）/ Preferred volume factor (1.0=normal)
    pub preferred_volume: f32,
    /// 偏好基频偏移（半音）/ Preferred pitch offset (semitones)
    pub preferred_pitch_offset: f32,
    /// 样本数 — 用于加权平均 / Sample count — for weighted averaging
    pub sample_count: u32,
}

impl Default for VoiceStyleProfile {
    fn default() -> Self {
        Self {
            preferred_rate: 1.0,
            preferred_volume: 1.0,
            preferred_pitch_offset: 0.0,
            sample_count: 0,
        }
    }
}

impl VoiceStyleProfile {
    /// 从新的语音样本更新风格画像 — 指数移动平均
    /// Update style profile from new voice sample — exponential moving average.
    ///
    /// @param rate 本次语速因子 / Current rate factor
    /// @param volume 本次音量因子 / Current volume factor
    /// @param pitch_offset 本次基频偏移 / Current pitch offset
    pub fn update(&mut self, rate: f32, volume: f32, pitch_offset: f32) {
        // 指数移动平均（EMA）— 新样本权重 α=0.1，历史权重 0.9
        // Exponential moving average (EMA) — new sample weight α=0.1, history weight 0.9
        const ALPHA: f32 = 0.1;
        if self.sample_count == 0 {
            // 首次样本 — 直接采用 / First sample — adopt directly
            self.preferred_rate = rate;
            self.preferred_volume = volume;
            self.preferred_pitch_offset = pitch_offset;
        } else {
            self.preferred_rate = self.preferred_rate * (1.0 - ALPHA) + rate * ALPHA;
            self.preferred_volume = self.preferred_volume * (1.0 - ALPHA) + volume * ALPHA;
            self.preferred_pitch_offset =
                self.preferred_pitch_offset * (1.0 - ALPHA) + pitch_offset * ALPHA;
        }
        self.sample_count += 1;
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> VoiceprintCfg {
        VoiceprintCfg {
            enabled: true,
            service_url: String::new(),
            similarity_threshold: 0.75,
        }
    }

    #[test]
    fn test_voiceprint_recognizer_create() {
        // 创建识别器 / Create recognizer
        let recognizer = VoiceprintRecognizer::new(test_cfg());
        assert!(recognizer.is_enabled());
        assert_eq!(recognizer.registry_size(), 0);
    }

    #[test]
    fn test_voiceprint_identify_empty_registry() {
        // 空声纹库 — 返回 None / Empty registry — returns None
        let recognizer = VoiceprintRecognizer::new(test_cfg());
        let embedding = [0.0f32; 256];
        assert!(recognizer.identify(&embedding).is_none());
    }

    #[test]
    fn test_voiceprint_register_and_identify() {
        // 注册并识别 — 完全相同的嵌入应匹配 / Register and identify — identical embeddings should match
        let mut recognizer = VoiceprintRecognizer::new(test_cfg());
        let embedding = [0.5f32; 256];
        recognizer.register("user1", embedding);
        assert_eq!(recognizer.registry_size(), 1);

        let result = recognizer.identify(&embedding).unwrap();
        assert_eq!(result.user_id, "user1");
        assert!(result.similarity > 0.99);
        assert!(result.is_match);
    }

    #[test]
    fn test_voiceprint_identify_below_threshold() {
        // 相似度低于阈值 — is_match=false / Similarity below threshold — is_match=false
        let mut recognizer = VoiceprintRecognizer::new(test_cfg());
        let registered = [1.0f32; 256];
        recognizer.register("user1", registered);

        // 完全不同的嵌入 — 相似度≈0 / Completely different embedding — similarity≈0
        let query = [0.0f32; 256];
        let result = recognizer.identify(&query).unwrap();
        assert_eq!(result.user_id, "user1"); // 仍然返回最佳匹配 / Still returns best match
        assert!(!result.is_match); // 但未达阈值 / But below threshold
    }

    #[test]
    fn test_voiceprint_identify_best_match() {
        // 多用户场景 — 返回最匹配的 / Multi-user scenario — returns best match
        let mut recognizer = VoiceprintRecognizer::new(test_cfg());
        let emb1 = [1.0f32; 256];
        let mut emb2 = [0.0f32; 256];
        emb2[0] = 1.0; // 与 emb1 部分相似 / Partially similar to emb1
        recognizer.register("user1", emb1);
        recognizer.register("user2", emb2);

        let query = [1.0f32; 256]; // 与 user1 完全相同 / Identical to user1
        let result = recognizer.identify(&query).unwrap();
        assert_eq!(result.user_id, "user1");
        assert!(result.is_match);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        // 相同向量 — 相似度=1.0 / Identical vectors — similarity=1.0
        let a = [1.0f32; 256];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        // 正交向量 — 相似度=0 / Orthogonal vectors — similarity=0
        let mut a = [0.0f32; 256];
        let mut b = [0.0f32; 256];
        a[0] = 1.0;
        b[1] = 1.0;
        let sim = cosine_similarity(&a, &b);
        assert!(sim < 1e-5);
    }

    #[test]
    fn test_voice_style_profile_default() {
        // 默认画像 — 中性值 / Default profile — neutral values
        let profile = VoiceStyleProfile::default();
        assert_eq!(profile.preferred_rate, 1.0);
        assert_eq!(profile.preferred_volume, 1.0);
        assert_eq!(profile.preferred_pitch_offset, 0.0);
        assert_eq!(profile.sample_count, 0);
    }

    #[test]
    fn test_voice_style_profile_first_sample() {
        // 首次样本 — 直接采用 / First sample — adopt directly
        let mut profile = VoiceStyleProfile::default();
        profile.update(1.2, 0.8, 2.0);
        assert_eq!(profile.preferred_rate, 1.2);
        assert_eq!(profile.preferred_volume, 0.8);
        assert_eq!(profile.preferred_pitch_offset, 2.0);
        assert_eq!(profile.sample_count, 1);
    }

    #[test]
    fn test_voice_style_profile_ema_update() {
        // EMA 更新 — 新样本权重 0.1 / EMA update — new sample weight 0.1
        let mut profile = VoiceStyleProfile::default();
        profile.update(1.0, 1.0, 0.0); // 首次 / First
        profile.update(2.0, 2.0, 4.0); // 第二次 / Second
                                       // expected = 1.0 * 0.9 + 2.0 * 0.1 = 1.1
        assert!((profile.preferred_rate - 1.1).abs() < 1e-5);
        assert!((profile.preferred_volume - 1.1).abs() < 1e-5);
        assert!((profile.preferred_pitch_offset - 0.4).abs() < 1e-5);
        assert_eq!(profile.sample_count, 2);
    }
}
