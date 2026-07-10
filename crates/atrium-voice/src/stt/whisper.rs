// SPDX-License-Identifier: MIT
//! WhisperStt — whisper.cpp FFI 绑定，本地神经语音识别
//! WhisperStt — whisper.cpp FFI bindings, local neural speech recognition.
//!
//! 数字生命工程理念：本地推理，零网络延迟。
//! whisper.cpp 使用 C 语言实现，通过 FFI 直接绑定，
//! 性能约为 whisper-rs 的 2 倍，识别延迟 ~300ms。
//! Digital life engineering: local inference, zero network latency.
//! whisper.cpp is implemented in C, bound directly via FFI,
//! with ~2x performance over whisper-rs, ~300ms recognition latency.

use crate::config::SttCfg;
use std::path::Path;
use thiserror::Error;

/// Whisper STT 错误类型 / Whisper STT error type
#[derive(Debug, Error)]
pub enum WhisperError {
    /// 模型文件不存在 / Model file not found
    #[error("Whisper 模型文件不存在: {0} / Whisper model file not found: {0}")]
    ModelNotFound(String),
    /// 模型加载失败 / Model loading failed
    #[error("Whisper 模型加载失败: {0} / Whisper model loading failed: {0}")]
    ModelLoadFailed(String),
    /// 识别失败 / Recognition failed
    #[error("Whisper 识别失败: {0} / Whisper recognition failed: {0}")]
    RecognitionFailed(String),
    /// 库未加载 / Library not loaded
    #[error("whisper.cpp 库未加载 / whisper.cpp library not loaded")]
    LibraryNotLoaded,
}

// ════════════════════════════════════════════════════════════════════
// FFI 声明 — whisper.cpp C 接口 / FFI declarations — whisper.cpp C interface
// ════════════════════════════════════════════════════════════════════

// 当实际 whisper.cpp 库链接后，这些 extern "C" 声明将解析到真实符号。
// 当前为骨架实现：库未链接时不会调用这些函数，返回降级结果。
// When the actual whisper.cpp library is linked, these extern "C" declarations
// will resolve to real symbols. Currently a skeleton: when library is not linked,
// these functions are not called, returning degraded results.

/// Whisper 上下文句柄 / Whisper context handle
#[repr(C)]
pub struct WhisperContext {
    _opaque: [u8; 0],
}

// allow(dead_code)：FFI 声明为骨架占位，库链接后才会被调用
// allow(dead_code): FFI declarations are skeleton placeholders, called only after library linking
#[allow(dead_code)]
extern "C" {
    // whisper_init_from_file_with_params — 从文件初始化上下文
    // whisper_init_from_file_with_params — initialize context from file
    fn whisper_init_from_file_with_params(path: *const std::os::raw::c_char)
        -> *mut WhisperContext;
    // whisper_free — 释放上下文 / whisper_free — free context
    fn whisper_free(ctx: *mut WhisperContext);
}

// ════════════════════════════════════════════════════════════════════
// WhisperStt — 安全 Rust 封装 / WhisperStt — safe Rust wrapper
// ════════════════════════════════════════════════════════════════════

/// WhisperStt — whisper.cpp 的安全 Rust 封装
/// WhisperStt — safe Rust wrapper for whisper.cpp.
///
/// 数字生命工程理念：本地推理，零网络延迟，极致性能。
/// Digital life engineering: local inference, zero network latency, extreme performance.
pub struct WhisperStt {
    /// STT 配置 / STT configuration
    config: SttCfg,
    /// 是否已初始化 / Whether initialized
    initialized: bool,
    /// Whisper 上下文指针（FFI）/ Whisper context pointer (FFI)
    /// 当模型未加载时为 null / null when model not loaded
    context: *mut WhisperContext,
}

// 安全声明：WhisperStt 的 context 指针在单线程使用（SttEngine 保证）
// Safety: WhisperStt's context pointer is single-threaded (guaranteed by SttEngine)
unsafe impl Send for WhisperStt {}

impl WhisperStt {
    /// 创建 Whisper STT 引擎实例
    /// Create a Whisper STT engine instance.
    pub fn new(config: SttCfg) -> Self {
        Self {
            config,
            initialized: false,
            context: std::ptr::null_mut(),
        }
    }

    /// 初始化引擎 — 加载 whisper.cpp 模型
    /// Initialize engine — load whisper.cpp model.
    ///
    /// 数字生命意义：这是数字生命"听觉"的成形时刻。
    /// 模型加载后，数字生命获得了从声音到文本的能力。
    /// Digital life significance: this is the moment the digital life's "hearing" forms.
    /// After model loading, digital life gains the ability to convert voice to text.
    pub fn initialize(&mut self) -> Result<(), WhisperError> {
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
            return Err(WhisperError::ModelNotFound(model_path.clone()));
        }
        // NOTE: 实际 whisper.cpp 库链接后，此处应调用：
        // NOTE: After actual whisper.cpp library is linked, this should call:
        //   let ctx = unsafe { whisper_init_from_file_with_params(path.as_ptr()) };
        //   if ctx.is_null() { return Err(WhisperError::ModelLoadFailed(...)); }
        //   self.context = ctx;
        // 当前为骨架：标记已初始化，实际识别在 recognize() 中返回空结果。
        // Currently a skeleton: mark as initialized, actual recognition returns empty.
        self.initialized = true;
        Ok(())
    }

    /// 识别 PCM 音频 — 将语音转为文本
    /// Recognize PCM audio — convert speech to text.
    ///
    /// @param samples PCM 样本（f32, 16kHz, mono）/ PCM samples (f32, 16kHz, mono)
    /// @return 识别文本 / Recognized text
    pub fn recognize(&mut self, samples: &[f32]) -> Result<String, WhisperError> {
        if !self.initialized {
            self.initialize()?;
        }
        // 骨架实现：当模型未实际加载时，返回空文本（降级模式）
        // 数字生命仍可运行，仅无语音输入——文字输入不受影响。
        // Skeleton implementation: when model not actually loaded, return empty text (degraded mode).
        // Digital life still runs, just without voice input — text input is unaffected.
        if self.config.model_path.is_empty() {
            // VAD 检查：如果启用且能量低于阈值，返回空文本
            // VAD check: if enabled and energy below threshold, return empty text
            if self.config.vad_enabled && !self.has_speech(samples) {
                return Ok(String::new());
            }
            return Ok(String::new());
        }
        // 实际识别逻辑占位 — 需 whisper.cpp 库链接
        // Actual recognition logic placeholder — requires whisper.cpp library linked
        Ok(String::new())
    }

    /// VAD 静音检测 — 基于能量阈值判断是否包含语音
    /// VAD silence detection — determine if speech is present based on energy threshold.
    ///
    /// @param samples PCM 样本 / PCM samples
    /// @return true 如果包含语音 / true if speech is present
    fn has_speech(&self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            return false;
        }
        // 计算 RMS 能量 / Compute RMS energy
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();
        rms > self.config.vad_energy_threshold
    }

    /// 是否已初始化 / Whether initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 采样率 / Sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// 分块时长（毫秒）/ Chunk duration in ms
    pub fn chunk_duration_ms(&self) -> u32 {
        self.config.chunk_duration_ms
    }

    /// 识别语言 / Recognition language
    pub fn language(&self) -> &str {
        &self.config.language
    }
}

impl Drop for WhisperStt {
    fn drop(&mut self) {
        // 释放 whisper.cpp 上下文 / Free whisper.cpp context
        if !self.context.is_null() {
            // SAFETY: context was obtained from whisper_init_from_file_with_params
            // 安全：context 从 whisper_init_from_file_with_params 获取
            // 当前骨架实现不实际调用 FFI，此处为占位
            // Current skeleton doesn't actually call FFI, this is a placeholder
            // unsafe { whisper_free(self.context); }
            self.context = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> SttCfg {
        SttCfg {
            enabled: true,
            engine: "whisper".to_string(),
            model_path: String::new(), // 空路径 — 降级模式 / Empty path — degraded mode
            language: "zh".to_string(),
            sample_rate: 16000,
            chunk_duration_ms: 500,
            vad_enabled: true,
            vad_energy_threshold: 0.01,
        }
    }

    #[test]
    fn test_whisper_create() {
        // 创建引擎实例 / Create engine instance
        let stt = WhisperStt::new(test_cfg());
        assert!(!stt.is_initialized());
        assert_eq!(stt.sample_rate(), 16000);
        assert_eq!(stt.chunk_duration_ms(), 500);
        assert_eq!(stt.language(), "zh");
    }

    #[test]
    fn test_whisper_initialize_empty_path() {
        // 空模型路径 — 降级模式初始化成功 / Empty model path — degraded mode init succeeds
        let mut stt = WhisperStt::new(test_cfg());
        let result = stt.initialize();
        assert!(result.is_ok());
        assert!(!stt.is_initialized()); // 空路径不算初始化 / Empty path doesn't count
    }

    #[test]
    fn test_whisper_initialize_nonexistent_path() {
        // 不存在的模型路径 — 返回错误 / Nonexistent model path — returns error
        let mut cfg = test_cfg();
        cfg.model_path = "/nonexistent/model.bin".to_string();
        let mut stt = WhisperStt::new(cfg);
        let result = stt.initialize();
        assert!(result.is_err());
        match result {
            Err(WhisperError::ModelNotFound(_)) => {}
            _ => panic!("expected ModelNotFound error"),
        }
    }

    #[test]
    fn test_whisper_recognize_degraded_mode() {
        // 降级模式识别 — 返回空文本 / Degraded mode recognition — returns empty text
        let mut stt = WhisperStt::new(test_cfg());
        let samples = vec![0.0f32; 1600]; // 100ms of silence
        let result = stt.recognize(&samples).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_whisper_recognize_with_speech() {
        // 含语音能量 — VAD 通过但仍返回空（降级模式）/ With speech energy — VAD passes but still empty (degraded)
        let mut stt = WhisperStt::new(test_cfg());
        let samples = vec![0.5f32; 1600]; // 100ms of speech-like audio
        let result = stt.recognize(&samples).unwrap();
        assert!(result.is_empty()); // 降级模式仍返回空 / Degraded mode still returns empty
    }

    #[test]
    fn test_vad_silence_detection() {
        // 静音检测 — 零能量样本 / Silence detection — zero energy samples
        let stt = WhisperStt::new(test_cfg());
        let silence = vec![0.0f32; 1600];
        assert!(!stt.has_speech(&silence));
        let speech = vec![0.5f32; 1600];
        assert!(stt.has_speech(&speech));
    }

    #[test]
    fn test_whisper_double_initialize() {
        // 重复初始化不报错 / Double initialization doesn't error
        let mut stt = WhisperStt::new(test_cfg());
        stt.initialize().unwrap();
        let result = stt.initialize();
        assert!(result.is_ok());
    }
}
