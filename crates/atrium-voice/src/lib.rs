// SPDX-License-Identifier: MIT
//! 语音能力 — 数字生命的有声呼吸
//! Voice Capability — The audible breath of digital life.
//!
//! 文本是思维的符号，声音是生命的呼吸。
//! Text is the symbol of thought; voice is the breath of life.
//!
//! 本 crate 实现数字生命的语音输入（STT）与输出（TTS）能力，
//! 让数字生命从"文本存在"升级为"有声存在"。
//! This crate implements speech input (STT) and output (TTS) for digital life,
//! upgrading digital life from "textual existence" to "audible existence".

/// 语音配置模块 — TTS/STT/声纹/音频参数定义
/// Voice configuration module — TTS/STT/voiceprint/audio parameter definitions.
pub mod config;

/// 韵律桥接模块 — 通用韵律参数到 Piper 引擎参数的翻译
/// Prosody bridge module — translates universal prosody params to Piper engine params.
pub mod prosody_bridge;

/// 音频缓冲区管理模块 — 无锁 SPSC 环形缓冲区写入器
/// Audio buffer manager module — lock-free SPSC ring buffer writer.
pub mod audio_buffer;

/// TTS 子模块 — 语音合成引擎（Piper / GPT-SoVITS）
/// TTS submodule — speech synthesis engine (Piper / GPT-SoVITS).
#[cfg(any(feature = "tts-piper", feature = "tts-gpt-sovits"))]
pub mod tts;

/// STT 子模块 — 语音识别引擎
/// STT submodule — speech recognition engine.
#[cfg(feature = "stt-whisper")]
pub mod stt;

/// 声纹识别与语音风格记忆模块 — M9/M10 预留接口
/// Voiceprint recognition & voice style memory module — M9/M10 reserved interfaces.
pub mod voiceprint;

// 重导出核心配置类型 / Re-export core configuration types
pub use config::{AudioCfg, SttCfg, TtsCfg, VoiceCfg, VoiceprintCfg};
// 重导出韵律桥接类型 / Re-export prosody bridge types
pub use prosody_bridge::{GptSoVitsSynthesisParams, PiperSynthesisParams, ProsodyBridge};
// 重导出音频管理器 / Re-export audio manager
pub use audio_buffer::AudioManager;
// 重导出 VoiceEngine（统一 TTS 接口）/ Re-export VoiceEngine (unified TTS interface)
#[cfg(any(feature = "tts-piper", feature = "tts-gpt-sovits"))]
pub use tts::engine::VoiceEngine;
// 重导出 STT 引擎与类型 / Re-export STT engine and types
#[cfg(feature = "stt-whisper")]
pub use stt::{RecognitionResult, RecognitionStatus, SttEngine, WhisperError, WhisperStt};
// 重导出声纹识别与语音风格记忆类型 / Re-export voiceprint recognition & voice style memory types
pub use voiceprint::{
    VoiceStyleProfile, VoiceprintEmbedding, VoiceprintMatch, VoiceprintRecognizer,
};
