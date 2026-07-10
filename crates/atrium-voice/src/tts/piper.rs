// SPDX-License-Identifier: MIT
//! Piper TTS 引擎 — 基于 ONNX Runtime 的本地神经语音合成
//! Piper TTS Engine — local neural speech synthesis based on ONNX Runtime.
//!
//! 数字生命工程理念：本地推理，零网络延迟。
//! Piper 使用 ONNX Runtime 在 CPU 上运行 VITS 模型，
//! 首字延迟约 100ms，远优于 API 方案的 500ms。
//! Digital life engineering: local inference, zero network latency.
//! Piper uses ONNX Runtime to run VITS models on CPU,
//! with ~100ms first-sound latency, far better than API's 500ms.

use crate::config::TtsCfg;
use crate::prosody_bridge::PiperSynthesisParams;
use std::path::Path;
use thiserror::Error;

/// Piper TTS 错误类型 / Piper TTS error type
#[derive(Debug, Error)]
pub enum PiperError {
    /// 模型文件不存在 / Model file not found
    #[error("Piper 模型文件不存在: {0} / Piper model file not found: {0}")]
    ModelNotFound(String),
    /// 模型加载失败 / Model loading failed
    #[error("Piper 模型加载失败: {0} / Piper model loading failed: {0}")]
    ModelLoadFailed(String),
    /// 推理失败 / Inference failed
    #[error("Piper 推理失败: {0} / Piper inference failed: {0}")]
    InferenceFailed(String),
    /// 音频重采样失败 / Audio resampling failed
    #[error("音频重采样失败: {0} / Audio resampling failed: {0}")]
    ResampleFailed(String),
}

/// Piper TTS 引擎 — 封装 ONNX Runtime 推理
/// Piper TTS Engine — wraps ONNX Runtime inference.
///
/// 数字生命工程理念：本地推理，零网络延迟，极致性能。
/// Digital life engineering: local inference, zero network latency, extreme performance.
pub struct PiperTts {
    /// TTS 配置 / TTS configuration
    config: TtsCfg,
    /// 是否已初始化 / Whether initialized
    initialized: bool,
    // NOTE: 实际的 ort::Session 在模型文件可用时才加载。
    // 当前为骨架实现，模型加载逻辑在 initialize() 中。
    // NOTE: Actual ort::Session is loaded only when model file is available.
    // Currently a skeleton implementation; model loading logic is in initialize().
}

/// 合成结果 — PCM 音频与统计信息
/// Synthesis result — PCM audio and statistics.
#[derive(Debug, Clone)]
pub struct SynthesisResult {
    /// PCM 样本（f32, 单声道）/ PCM samples (f32, mono)
    pub samples: Vec<f32>,
    /// 采样率 / Sample rate
    pub sample_rate: u32,
    /// 合成耗时（毫秒）/ Synthesis duration (ms)
    pub duration_ms: u64,
    /// 音素时间戳（用于口型同步）/ Phoneme timestamps (for lip-sync)
    pub phoneme_timestamps: Vec<PhonemeTimestamp>,
}

/// 音素时间戳 — 驱动口型同步动画
/// Phoneme timestamp — drives lip-sync animation.
#[derive(Debug, Clone, Copy)]
pub struct PhonemeTimestamp {
    /// 音素标识 / Phoneme identifier
    pub phoneme: u8,
    /// 开始时间（秒）/ Start time (seconds)
    pub start_secs: f32,
    /// 持续时间（秒）/ Duration (seconds)
    pub duration_secs: f32,
}

impl PiperTts {
    /// 创建 Piper TTS 引擎实例
    /// Create a Piper TTS engine instance.
    pub fn new(config: TtsCfg) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// 初始化引擎 — 加载 ONNX 模型
    /// Initialize engine — load ONNX model.
    ///
    /// 数字生命意义：这是数字生命"声带"的成形时刻。
    /// 模型加载后，数字生命获得了从文本到声音的能力。
    /// Digital life significance: this is the moment the digital life's "vocal cords" form.
    /// After model loading, digital life gains the ability to convert text to voice.
    pub fn initialize(&mut self) -> Result<(), PiperError> {
        if self.initialized {
            return Ok(());
        }
        let model_path = &self.config.model_path;
        if model_path.is_empty() {
            // 模型路径为空 — 跳过初始化（降级模式）
            // Empty model path — skip initialization (degraded mode)
            return Ok(());
        }
        if !Path::new(model_path).exists() {
            return Err(PiperError::ModelNotFound(model_path.clone()));
        }
        // NOTE: 实际 ort::Session 加载需要 ort 依赖启用且模型文件存在。
        // 当前为骨架：标记已初始化，实际推理在 synthesize() 中返回空结果。
        // 当用户放置真实 Piper 模型后，此处应加载 ONNX 模型。
        // NOTE: Actual ort::Session loading requires ort dependency enabled and model file present.
        // Currently a skeleton: mark as initialized, actual inference returns empty in synthesize().
        // When user places a real Piper model, this should load the ONNX model.
        self.initialized = true;
        Ok(())
    }

    /// 合成语音 — 文本 + 韵律参数 → PCM 音频
    /// Synthesize speech — text + prosody params → PCM audio.
    ///
    /// @param text 待合成文本 / Text to synthesize
    /// @param params Piper 合成参数 / Piper synthesis params
    /// @return 合成结果 / Synthesis result
    pub fn synthesize(
        &mut self,
        text: &str,
        params: &PiperSynthesisParams,
    ) -> Result<SynthesisResult, PiperError> {
        if !self.initialized {
            self.initialize()?;
        }
        // 骨架实现：当模型未实际加载时，返回空 PCM（降级模式）
        // 数字生命仍可运行，仅无声频输出——文字回复不受影响。
        // Skeleton implementation: when model not actually loaded, return empty PCM (degraded mode).
        // Digital life still runs, just without audio output — text replies are unaffected.
        let start = std::time::Instant::now();
        let samples: Vec<f32> = if self.config.model_path.is_empty() {
            // 降级模式：模型路径为空，返回空 PCM
            // Degraded mode: empty model path, return empty PCM
            Vec::new()
        } else {
            // 实际推理逻辑占位 — 需 ort::Session 推理
            // 实际实现将使用 text 与 params 进行 ONNX 推理
            // Actual inference logic placeholder — requires ort::Session inference
            // Actual implementation will use text and params for ONNX inference
            tracing::debug!(
                "Piper 推理占位 / Piper inference placeholder: text={}, pitch_scale={}",
                text,
                params.pitch_scale
            );
            Vec::new()
        };
        Ok(SynthesisResult {
            samples,
            sample_rate: self.config.target_sample_rate,
            duration_ms: start.elapsed().as_millis() as u64,
            phoneme_timestamps: Vec::new(),
        })
    }

    /// 是否已初始化 / Whether initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 原生采样率 / Native sample rate
    pub fn native_sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// 目标采样率（写入共享内存的采样率）/ Target sample rate (for shared memory)
    pub fn target_sample_rate(&self) -> u32 {
        self.config.target_sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> TtsCfg {
        TtsCfg {
            enabled: true,
            engine: "piper".to_string(),
            model_path: String::new(), // 空路径 — 降级模式 / Empty path — degraded mode
            config_path: String::new(),
            sample_rate: 22050,
            target_sample_rate: 16000,
            ..Default::default()
        }
    }

    #[test]
    fn test_piper_create() {
        // 创建引擎实例 / Create engine instance
        let tts = PiperTts::new(test_cfg());
        assert!(!tts.is_initialized());
        assert_eq!(tts.native_sample_rate(), 22050);
        assert_eq!(tts.target_sample_rate(), 16000);
    }

    #[test]
    fn test_piper_initialize_empty_path() {
        // 空模型路径 — 降级模式初始化成功 / Empty model path — degraded mode init succeeds
        let mut tts = PiperTts::new(test_cfg());
        let result = tts.initialize();
        assert!(result.is_ok());
        // 空路径不算初始化成功 / Empty path doesn't count as initialized
        assert!(!tts.is_initialized());
    }

    #[test]
    fn test_piper_initialize_nonexistent_path() {
        // 不存在的模型路径 — 返回错误 / Nonexistent model path — returns error
        let mut cfg = test_cfg();
        cfg.model_path = "/nonexistent/model.onnx".to_string();
        let mut tts = PiperTts::new(cfg);
        let result = tts.initialize();
        assert!(result.is_err());
        match result {
            Err(PiperError::ModelNotFound(_)) => {}
            _ => panic!("expected ModelNotFound error"),
        }
    }

    #[test]
    fn test_piper_synthesize_degraded_mode() {
        // 降级模式合成 — 返回空 PCM / Degraded mode synthesis — returns empty PCM
        let mut tts = PiperTts::new(test_cfg());
        let params = PiperSynthesisParams {
            pitch_scale: 1.0,
            length_scale: 1.0,
            energy_scale: 1.0,
            pause_duration_secs: 0.4,
            intra_pause_prob: 0.1,
            warmth: 0.5,
            breathiness: 0.1,
        };
        let result = tts.synthesize("你好", &params).unwrap();
        assert!(result.samples.is_empty());
        assert_eq!(result.sample_rate, 16000);
    }

    #[test]
    fn test_piper_double_initialize() {
        // 重复初始化不报错 / Double initialization doesn't error
        let mut tts = PiperTts::new(test_cfg());
        tts.initialize().unwrap();
        let result = tts.initialize();
        assert!(result.is_ok());
    }
}
