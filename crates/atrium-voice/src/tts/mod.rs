// SPDX-License-Identifier: MIT
//! TTS 子模块 — 语音合成引擎
//! TTS submodule — speech synthesis engine.
//!
//! 支持两种后端，通过 feature gate 切换：
//! - `tts-piper`：Piper ONNX 本地推理（CPU，低延迟）
//! - `tts-gpt-sovits`：GPT-SoVITS HTTP 桥接（GPU，声音克隆）
//! Supports two backends, switched via feature gates:
//! - `tts-piper`: Piper ONNX local inference (CPU, low latency)
//! - `tts-gpt-sovits`: GPT-SoVITS HTTP bridge (GPU, voice cloning)

/// Piper TTS 引擎实现 / Piper TTS engine implementation
#[cfg(feature = "tts-piper")]
pub mod piper;

/// GPT-SoVITS TTS 引擎实现 — 声音克隆 HTTP 后端
/// GPT-SoVITS TTS engine implementation — voice cloning HTTP backend
#[cfg(feature = "tts-gpt-sovits")]
pub mod gpt_sovits;

/// VoiceEngine 统一 TTS 接口 / VoiceEngine unified TTS interface
///
/// 任一 TTS feature 启用时编译 / Compiled when any TTS feature is enabled
#[cfg(any(feature = "tts-piper", feature = "tts-gpt-sovits"))]
pub mod engine;

#[cfg(feature = "tts-gpt-sovits")]
pub use gpt_sovits::GptSoVitsClient;
#[cfg(feature = "tts-piper")]
pub use piper::PiperTts;
