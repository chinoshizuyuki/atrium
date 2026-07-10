// SPDX-License-Identifier: MIT
//! 语音配置 — TTS/STT/声纹/音频参数的统一配置结构
//! Voice configuration — unified configuration structures for TTS/STT/voiceprint/audio.
//!
//! 所有字段均标注 `#[serde(default)]` 或 `#[serde(default = "...")]`，
//! 保证配置文件中缺失的键能安全回退到默认值。
//! All fields are annotated with `#[serde(default)]` or `#[serde(default = "...")]`,
//! ensuring missing keys in config files safely fall back to defaults.

use serde::Deserialize;

// ════════════════════════════════════════════════════════════════════
// VoiceCfg — 语音能力根配置 / Voice Capability Root Config
// ════════════════════════════════════════════════════════════════════

/// 语音能力根配置 — 数字生命"有声呼吸"的总开关与子模块配置
/// Voice capability root configuration — master switch and submodule configs
/// for digital life's "audible breath".
///
/// 默认 `enabled = false`：语音能力为可选项，禁用时对系统零影响。
/// Defaults to `enabled = false`: voice capability is opt-in and zero-impact when disabled.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct VoiceCfg {
    /// 总开关 — 默认关闭，语音能力为可选项 / Master switch — defaults to false, voice is opt-in
    #[serde(default)]
    pub enabled: bool,
    /// TTS 配置 / TTS configuration
    #[serde(default)]
    pub tts: TtsCfg,
    /// STT 配置 / STT configuration
    #[serde(default)]
    pub stt: SttCfg,
    /// 声纹配置 / Voiceprint configuration
    #[serde(default)]
    pub voiceprint: VoiceprintCfg,
    /// 音频配置 / Audio configuration
    #[serde(default)]
    pub audio: AudioCfg,
}

// ════════════════════════════════════════════════════════════════════
// TtsCfg — TTS 引擎配置 / TTS Engine Config
// ════════════════════════════════════════════════════════════════════

/// TTS 引擎配置 — 支持 Piper ONNX 与 GPT-SoVITS HTTP 两种后端
/// TTS engine configuration — supports both Piper ONNX and GPT-SoVITS HTTP backends.
///
/// 通过 `engine` 字段切换后端：
/// - `"piper"`：本地 ONNX 推理（CPU，低延迟）
/// - `"gpt-sovits"`：HTTP 桥接 Python GPT-SoVITS 服务（GPU，高质量声音克隆）
///
/// Switch backend via the `engine` field:
/// - `"piper"`: local ONNX inference (CPU, low latency)
/// - `"gpt-sovits"`: HTTP bridge to Python GPT-SoVITS service (GPU, high-quality voice cloning)
#[derive(Debug, Clone, Deserialize)]
pub struct TtsCfg {
    /// 是否启用 TTS / Whether to enable TTS
    #[serde(default)]
    pub enabled: bool,
    /// 引擎类型：`"piper"` | `"gpt-sovits"` / Engine type: `"piper"` | `"gpt-sovits"`
    #[serde(default = "default_tts_engine")]
    pub engine: String,
    /// Piper ONNX 模型路径 / Piper ONNX model path
    #[serde(default)]
    pub model_path: String,
    /// Piper JSON 配置路径 / Piper JSON config path
    #[serde(default)]
    pub config_path: String,
    /// Piper 原生采样率 / Piper native sample rate
    #[serde(default = "default_tts_sample_rate")]
    pub sample_rate: u32,
    /// 目标采样率（写入共享内存）/ Target sample rate (written to shared memory)
    #[serde(default = "default_target_sample_rate")]
    pub target_sample_rate: u32,
    // ── GPT-SoVITS HTTP 后端专属配置 / GPT-SoVITS HTTP backend specific config ──
    /// GPT-SoVITS API 服务地址（如 `http://127.0.0.1:9880`）/ GPT-SoVITS API service URL
    #[serde(default)]
    pub service_url: String,
    /// 参考音频路径（声音克隆样本，5-10s）/ Reference audio path (voice clone sample, 5-10s)
    #[serde(default)]
    pub ref_audio_path: String,
    /// 参考音频对应文本（提升克隆质量）/ Reference audio prompt text (improves clone quality)
    #[serde(default)]
    pub prompt_text: String,
    /// 参考音频语言 / Reference audio prompt language
    #[serde(default = "default_gpt_sovits_prompt_lang")]
    pub prompt_lang: String,
    /// 合成语言 / Synthesis text language
    #[serde(default = "default_gpt_sovits_text_lang")]
    pub text_lang: String,
    /// 请求超时（秒）/ Request timeout in seconds
    #[serde(default = "default_gpt_sovits_timeout_secs")]
    pub timeout_secs: u64,
    /// 是否启用流式模式（0=关闭,1=最佳质量,2=中等,3=快速）/ Streaming mode
    #[serde(default)]
    pub streaming_mode: u8,
}

impl Default for TtsCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: default_tts_engine(),
            model_path: String::new(),
            config_path: String::new(),
            sample_rate: default_tts_sample_rate(),
            target_sample_rate: default_target_sample_rate(),
            service_url: String::new(),
            ref_audio_path: String::new(),
            prompt_text: String::new(),
            prompt_lang: default_gpt_sovits_prompt_lang(),
            text_lang: default_gpt_sovits_text_lang(),
            timeout_secs: default_gpt_sovits_timeout_secs(),
            streaming_mode: 0,
        }
    }
}

fn default_tts_engine() -> String {
    "piper".into()
}
fn default_tts_sample_rate() -> u32 {
    22050
}
fn default_target_sample_rate() -> u32 {
    16000
}
fn default_gpt_sovits_prompt_lang() -> String {
    "zh".into()
}
fn default_gpt_sovits_text_lang() -> String {
    "zh".into()
}
fn default_gpt_sovits_timeout_secs() -> u64 {
    30
}

// ════════════════════════════════════════════════════════════════════
// SttCfg — STT 引擎配置 / STT Engine Config
// ════════════════════════════════════════════════════════════════════

/// STT 引擎配置 — whisper.cpp 语音识别后端参数
/// STT engine configuration — whisper.cpp speech recognition backend parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct SttCfg {
    /// 是否启用 STT / Whether to enable STT
    #[serde(default)]
    pub enabled: bool,
    /// 引擎类型 / Engine type
    #[serde(default = "default_stt_engine")]
    pub engine: String,
    /// whisper.cpp 模型路径 / whisper.cpp model path
    #[serde(default)]
    pub model_path: String,
    /// 识别语言 / Recognition language
    #[serde(default = "default_stt_language")]
    pub language: String,
    /// 采样率 / Sample rate
    #[serde(default = "default_stt_sample_rate")]
    pub sample_rate: u32,
    /// 流式分块时长（毫秒）/ Streaming chunk duration in ms
    #[serde(default = "default_stt_chunk_duration_ms")]
    pub chunk_duration_ms: u32,
    /// 是否启用 VAD 静音检测 / Whether to enable VAD silence detection
    #[serde(default = "default_true")]
    pub vad_enabled: bool,
    /// VAD 能量阈值 / VAD energy threshold
    #[serde(default = "default_vad_energy_threshold")]
    pub vad_energy_threshold: f32,
}

impl Default for SttCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: default_stt_engine(),
            model_path: String::new(),
            language: default_stt_language(),
            sample_rate: default_stt_sample_rate(),
            chunk_duration_ms: default_stt_chunk_duration_ms(),
            vad_enabled: true,
            vad_energy_threshold: default_vad_energy_threshold(),
        }
    }
}

fn default_stt_engine() -> String {
    "whisper".into()
}
fn default_stt_language() -> String {
    "zh".into()
}
fn default_stt_sample_rate() -> u32 {
    16000
}
fn default_stt_chunk_duration_ms() -> u32 {
    500
}
fn default_true() -> bool {
    true
}
fn default_vad_energy_threshold() -> f32 {
    0.01
}

// ════════════════════════════════════════════════════════════════════
// VoiceprintCfg — 声纹识别配置 / Voiceprint Recognition Config
// ════════════════════════════════════════════════════════════════════

/// 声纹识别配置 — Python speechbrain gRPC 服务对接参数
/// Voiceprint recognition configuration — Python speechbrain gRPC service parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceprintCfg {
    /// 是否启用声纹识别 / Whether to enable voiceprint recognition
    #[serde(default)]
    pub enabled: bool,
    /// Python speechbrain gRPC 服务地址 / Python speechbrain gRPC service URL
    #[serde(default)]
    pub service_url: String,
    /// 余弦相似度判定阈值 / Cosine similarity threshold
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

impl Default for VoiceprintCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            service_url: String::new(),
            similarity_threshold: default_similarity_threshold(),
        }
    }
}

fn default_similarity_threshold() -> f32 {
    0.75
}

// ════════════════════════════════════════════════════════════════════
// AudioCfg — 音频缓冲区配置 / Audio Buffer Config
// ════════════════════════════════════════════════════════════════════

/// 音频缓冲区配置 — 共享内存音频通道参数
/// Audio buffer configuration — shared memory audio channel parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioCfg {
    /// 共享内存音频采样率 / Shared memory audio sample rate
    #[serde(default = "default_audio_sample_rate")]
    pub sample_rate: u32,
    /// 声道数（mono=1）/ Channel count (mono=1)
    #[serde(default = "default_audio_channels")]
    pub channels: u16,
    /// 环形缓冲区容量（样本数）/ Ring buffer capacity (in samples)
    #[serde(default = "default_audio_buffer_size")]
    pub buffer_size: usize,
}

impl Default for AudioCfg {
    fn default() -> Self {
        Self {
            sample_rate: default_audio_sample_rate(),
            channels: default_audio_channels(),
            buffer_size: default_audio_buffer_size(),
        }
    }
}

fn default_audio_sample_rate() -> u32 {
    16000
}
fn default_audio_channels() -> u16 {
    1
}
fn default_audio_buffer_size() -> usize {
    16384
}
