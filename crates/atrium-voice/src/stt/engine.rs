// SPDX-License-Identifier: MIT
//! SttEngine — 统一 STT 接口，编排音频分块→识别→文本输出
//! SttEngine — unified STT interface, orchestrates audio chunking→recognition→text output.
//!
//! 数字生命工程理念：这是数字生命"听到声音"的入口。
//! 麦克风音频流经过分块、VAD 静音检测、whisper.cpp 识别，
//! 最终转为文本送入 process_message 管线——与文字输入完全同构。
//! Digital life engineering: this is the entry point of digital life "hearing".
//! Microphone audio stream is chunked, VAD-filtered, recognized by whisper.cpp,
//! and finally converted to text for the process_message pipeline —
//! fully isomorphic with text input.

use crate::config::SttCfg;
use crate::stt::whisper::{WhisperError, WhisperStt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// SttEngine 错误类型 / SttEngine error type
#[derive(Debug, Error)]
pub enum SttEngineError {
    /// Whisper 引擎错误 / Whisper engine error
    #[error("Whisper 引擎错误: {0} / Whisper engine error: {0}")]
    WhisperError(#[from] WhisperError),
    /// STT 未启用 / STT not enabled
    #[error("STT 未启用 / STT not enabled")]
    NotEnabled,
}

/// 识别状态 — 流式识别的阶段性反馈 / Recognition status — staged feedback for streaming recognition
#[derive(Debug, Clone, PartialEq)]
pub enum RecognitionStatus {
    /// 正在识别 — 已接收部分音频 / Recognizing — partial audio received
    Partial,
    /// 识别完成 — 句子边界或静音触发 / Recognition complete — sentence boundary or silence triggered
    Final,
    /// 静音 — VAD 检测到无语音 / Silence — VAD detected no speech
    Silence,
}

/// 识别结果 — 文本 + 状态 + 时间戳 / Recognition result — text + status + timestamp
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// 识别文本 / Recognized text
    pub text: String,
    /// 识别状态 / Recognition status
    pub status: RecognitionStatus,
    /// 识别耗时（毫秒）/ Recognition duration (ms)
    pub duration_ms: u64,
    /// 是否为最终结果（可送入 process_message）/ Whether this is a final result (can be sent to process_message)
    pub is_final: bool,
}

/// SttEngine — 统一 STT 接口
/// SttEngine — unified STT interface.
///
/// 编排流程 / Orchestration flow:
/// 1. 接收 PCM 音频块（从 gRPC AudioStream 或本地麦克风）
/// 2. 按 chunk_duration_ms 分块
/// 3. VAD 静音检测（可选）
/// 4. WhisperStt 识别
/// 5. 返回 RecognitionResult（Partial/Final/Silence）
pub struct SttEngine {
    /// STT 配置 / STT configuration
    config: SttCfg,
    /// Whisper STT 引擎 / Whisper STT engine
    whisper: Option<WhisperStt>,
    /// 音频缓冲区 — 积累到 chunk_size 后触发识别
    /// Audio buffer — accumulates until chunk_size then triggers recognition
    audio_buffer: Vec<f32>,
    /// 单块样本数 = sample_rate * chunk_duration_ms / 1000
    /// Samples per chunk = sample_rate * chunk_duration_ms / 1000
    chunk_sample_count: usize,
    /// 是否正在识别（原子标志）/ Is recognizing (atomic flag)
    is_recognizing: Arc<AtomicBool>,
}

impl SttEngine {
    /// 创建 SttEngine — 从配置初始化
    /// Create SttEngine — initialize from config.
    pub fn new(config: SttCfg) -> Self {
        let chunk_sample_count =
            (config.sample_rate as usize * config.chunk_duration_ms as usize) / 1000;
        let whisper = if config.enabled {
            Some(WhisperStt::new(config.clone()))
        } else {
            None
        };
        Self {
            config,
            whisper,
            audio_buffer: Vec::with_capacity(chunk_sample_count * 2),
            chunk_sample_count,
            is_recognizing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 推入音频块 — 积累到 chunk_sample_count 后触发识别
    /// Push audio chunk — accumulates until chunk_sample_count then triggers recognition.
    ///
    /// @param samples PCM 样本 / PCM samples
    /// @return 识别结果（如果触发了识别）/ Recognition result (if recognition was triggered)
    pub fn push_audio(
        &mut self,
        samples: &[f32],
    ) -> Result<Option<RecognitionResult>, SttEngineError> {
        if !self.config.enabled {
            return Err(SttEngineError::NotEnabled);
        }
        self.audio_buffer.extend_from_slice(samples);

        // 积累不足一块 — 等待更多音频 / Not enough for a chunk — wait for more audio
        if self.audio_buffer.len() < self.chunk_sample_count {
            return Ok(None);
        }

        // 取出一块进行识别 / Extract one chunk for recognition
        let chunk: Vec<f32> = self.audio_buffer.drain(..self.chunk_sample_count).collect();
        self.recognize_chunk(&chunk)
    }

    /// 识别一块音频 — 内部方法
    /// Recognize a chunk of audio — internal method.
    fn recognize_chunk(
        &mut self,
        samples: &[f32],
    ) -> Result<Option<RecognitionResult>, SttEngineError> {
        let whisper = self.whisper.as_mut().ok_or(SttEngineError::NotEnabled)?;

        // 标记正在识别 / Mark as recognizing
        self.is_recognizing.store(true, Ordering::Release);

        let start = std::time::Instant::now();
        let text = whisper.recognize(samples)?;
        let duration_ms = start.elapsed().as_millis() as u64;

        self.is_recognizing.store(false, Ordering::Release);

        // 判断识别状态 / Determine recognition status
        let (status, is_final) = if text.is_empty() {
            // 空文本 — 静音或识别失败 / Empty text — silence or recognition failure
            (RecognitionStatus::Silence, false)
        } else {
            // 非空文本 — 最终结果（句号/问号/感叹号结尾时为 Final）
            // Non-empty text — final result (Final when ending with sentence-ending punctuation)
            let is_sentence_end = text.ends_with('。')
                || text.ends_with('？')
                || text.ends_with('！')
                || text.ends_with('.')
                || text.ends_with('?')
                || text.ends_with('!');
            (RecognitionStatus::Final, is_sentence_end)
        };

        Ok(Some(RecognitionResult {
            text,
            status,
            duration_ms,
            is_final,
        }))
    }

    /// 强制刷新 — 将缓冲区剩余音频送入识别
    /// Force flush — send remaining buffered audio to recognition.
    pub fn flush(&mut self) -> Result<Option<RecognitionResult>, SttEngineError> {
        if !self.config.enabled {
            return Err(SttEngineError::NotEnabled);
        }
        if self.audio_buffer.is_empty() {
            return Ok(None);
        }
        let chunk: Vec<f32> = self.audio_buffer.drain(..).collect();
        self.recognize_chunk(&chunk)
    }

    /// 是否正在识别 / Whether currently recognizing
    pub fn is_recognizing(&self) -> bool {
        self.is_recognizing.load(Ordering::Acquire)
    }

    /// 获取 is_recognizing 原子引用 / Get is_recognizing atomic reference
    pub fn is_recognizing_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_recognizing)
    }

    /// STT 是否启用 / Whether STT is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 单块样本数 / Samples per chunk
    pub fn chunk_sample_count(&self) -> usize {
        self.chunk_sample_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stt_cfg() -> SttCfg {
        SttCfg {
            enabled: true,
            engine: "whisper".to_string(),
            model_path: String::new(),
            language: "zh".to_string(),
            sample_rate: 16000,
            chunk_duration_ms: 500,
            vad_enabled: true,
            vad_energy_threshold: 0.01,
        }
    }

    #[test]
    fn test_stt_engine_create() {
        // 创建引擎 / Create engine
        let engine = SttEngine::new(test_stt_cfg());
        assert!(engine.is_enabled());
        assert!(!engine.is_recognizing());
        // chunk_sample_count = 16000 * 500 / 1000 = 8000
        assert_eq!(engine.chunk_sample_count(), 8000);
    }

    #[test]
    fn test_stt_engine_not_enabled() {
        // 未启用时返回错误 / Returns error when not enabled
        let mut cfg = test_stt_cfg();
        cfg.enabled = false;
        let mut engine = SttEngine::new(cfg);
        let result = engine.push_audio(&[0.0; 100]);
        assert!(matches!(result, Err(SttEngineError::NotEnabled)));
    }

    #[test]
    fn test_stt_engine_accumulate_audio() {
        // 积累不足一块 — 返回 None / Not enough for a chunk — returns None
        let mut engine = SttEngine::new(test_stt_cfg());
        let result = engine.push_audio(&[0.0; 100]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_stt_engine_recognize_silence() {
        // 识别静音 — 返回 Silence 状态 / Recognize silence — returns Silence status
        let mut engine = SttEngine::new(test_stt_cfg());
        let silence = vec![0.0f32; 8000]; // 500ms of silence
        let result = engine.push_audio(&silence).unwrap().unwrap();
        assert_eq!(result.status, RecognitionStatus::Silence);
        assert!(!result.is_final);
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_stt_engine_recognize_speech_degraded() {
        // 降级模式识别语音 — 仍返回空文本（无模型）/ Degraded mode speech recognition — still empty (no model)
        let mut engine = SttEngine::new(test_stt_cfg());
        let speech = vec![0.5f32; 8000]; // 500ms of speech-like audio
        let result = engine.push_audio(&speech).unwrap().unwrap();
        // 降级模式：无模型，即使 VAD 通过也返回空文本 / Degraded mode: no model, returns empty even if VAD passes
        assert!(result.text.is_empty());
    }

    #[test]
    fn test_stt_engine_flush() {
        // flush 将剩余音频送入识别 / Flush sends remaining audio to recognition
        let mut engine = SttEngine::new(test_stt_cfg());
        engine.push_audio(&[0.0; 100]).unwrap(); // 积累少量 / Accumulate small amount
        let result = engine.flush().unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_stt_engine_flush_empty() {
        // flush 空缓冲区 — 返回 None / Flush empty buffer — returns None
        let mut engine = SttEngine::new(test_stt_cfg());
        let result = engine.flush().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_stt_engine_is_recognizing_flag() {
        // is_recognizing_flag 可跨线程查询 / is_recognizing_flag queryable across threads
        let engine = SttEngine::new(test_stt_cfg());
        let flag = engine.is_recognizing_flag();
        assert!(!flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_stt_engine_multiple_chunks() {
        // 多块连续识别 / Multiple chunks sequential recognition
        let mut engine = SttEngine::new(test_stt_cfg());
        // 推入 16000 样本 = 2 个块 / Push 16000 samples = 2 chunks
        let audio = vec![0.0f32; 16000];
        let r1 = engine.push_audio(&audio).unwrap();
        assert!(r1.is_some());
        // 第二块已在缓冲区 / Second chunk already in buffer
        let r2 = engine.flush().unwrap();
        assert!(r2.is_some());
    }
}
