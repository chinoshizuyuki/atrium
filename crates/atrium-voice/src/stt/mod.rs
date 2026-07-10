// SPDX-License-Identifier: MIT
//! STT 子模块 — 语音识别引擎
//! STT submodule — speech recognition engine.

/// whisper.cpp FFI 绑定 / whisper.cpp FFI bindings
#[cfg(feature = "stt-whisper")]
pub mod whisper;

/// SttEngine 流式识别接口 / SttEngine streaming recognition interface
#[cfg(feature = "stt-whisper")]
pub mod engine;

#[cfg(feature = "stt-whisper")]
pub use engine::{RecognitionResult, RecognitionStatus, SttEngine};
#[cfg(feature = "stt-whisper")]
pub use whisper::{WhisperError, WhisperStt};
