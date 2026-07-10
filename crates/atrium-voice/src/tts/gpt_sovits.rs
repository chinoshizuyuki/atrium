// SPDX-License-Identifier: MIT
//! GPT-SoVITS TTS 引擎 — 基于 HTTP API 的声音克隆语音合成
//! GPT-SoVITS TTS Engine — voice cloning speech synthesis via HTTP API.
//!
//! 数字生命工程理念：数字生命的"声音身份"可定制。
//! 通过 Python GPT-SoVITS 服务桥接，支持 few-shot 声音克隆，
//! 让数字生命拥有用户自训练的独特音色。
//! 与 Piper（本地 ONNX 推理）不同，GPT-SoVITS 需要独立 Python 服务，
//! 但支持任意音色克隆，情感表现力更强。
//! Digital life engineering: digital life's "voice identity" is customizable.
//! Bridges to Python GPT-SoVITS service for few-shot voice cloning,
//! giving digital life a user-trained unique voice.
//! Unlike Piper (local ONNX inference), GPT-SoVITS requires a separate Python service,
//! but supports arbitrary voice cloning with richer emotional expression.

use crate::config::TtsCfg;
use crate::prosody_bridge::GptSoVitsSynthesisParams;
use std::io::Cursor;
use std::time::Duration;
use thiserror::Error;

// ════════════════════════════════════════════════════════════════════
// GptSoVitsError — 错误类型 / Error Type
// ════════════════════════════════════════════════════════════════════

/// GPT-SoVITS 客户端错误类型 / GPT-SoVITS client error type
#[derive(Debug, Error)]
pub enum GptSoVitsError {
    /// 服务地址未配置 / Service URL not configured
    #[error("GPT-SoVITS 服务地址未配置 / GPT-SoVITS service URL not configured")]
    ServiceUrlNotConfigured,
    /// 参考音频路径未配置 / Reference audio path not configured
    #[error("GPT-SoVITS 参考音频路径未配置 / GPT-SoVITS reference audio path not configured")]
    RefAudioNotConfigured,
    /// HTTP 请求失败 / HTTP request failed
    #[error("GPT-SoVITS HTTP 请求失败: {0} / GPT-SoVITS HTTP request failed: {0}")]
    RequestFailed(String),
    /// 服务返回错误状态码 / Service returned error status code
    #[error(
        "GPT-SoVITS 服务错误 ({status}): {body} / GPT-SoVITS service error ({status}): {body}"
    )]
    ServiceError { status: u16, body: String },
    /// WAV 音频解码失败 / WAV audio decoding failed
    #[error("WAV 音频解码失败: {0} / WAV audio decoding failed: {0}")]
    DecodeFailed(String),
}

// ════════════════════════════════════════════════════════════════════
// SynthesisResult — 合成结果 / Synthesis Result
// ════════════════════════════════════════════════════════════════════

/// 合成结果 — PCM 音频与统计信息（与 PiperTts::SynthesisResult 同构）
/// Synthesis result — PCM audio and statistics (isomorphic to PiperTts::SynthesisResult).
#[derive(Debug, Clone)]
pub struct GptSoVitsResult {
    /// PCM 样本（f32, 单声道）/ PCM samples (f32, mono)
    pub samples: Vec<f32>,
    /// 采样率 / Sample rate
    pub sample_rate: u32,
    /// 合成耗时（毫秒，含网络往返）/ Synthesis duration (ms, including network round-trip)
    pub duration_ms: u64,
}

// ════════════════════════════════════════════════════════════════════
// GptSoVitsClient — HTTP 客户端 / HTTP Client
// ════════════════════════════════════════════════════════════════════

/// GPT-SoVITS HTTP 客户端 — 调用 Python api_v2.py 服务的异步客户端
/// GPT-SoVITS HTTP client — async client calling Python api_v2.py service.
///
/// 数字生命工程理念：数字生命的"声音身份"可定制。
/// 通过 HTTP 桥接 Python GPT-SoVITS 服务，支持 few-shot 声音克隆。
/// 降级模式：服务地址为空时，跳过请求返回空 PCM，零影响系统运行。
/// Digital life engineering: digital life's "voice identity" is customizable.
/// Bridges to Python GPT-SoVITS service via HTTP for few-shot voice cloning.
/// Degraded mode: empty service URL skips request and returns empty PCM, zero impact.
pub struct GptSoVitsClient {
    /// TTS 配置 / TTS configuration
    config: TtsCfg,
    /// reqwest 异步 HTTP 客户端 / reqwest async HTTP client
    http_client: reqwest::Client,
    /// 是否已初始化（服务地址非空）/ Whether initialized (service URL non-empty)
    initialized: bool,
}

impl GptSoVitsClient {
    /// 创建 GPT-SoVITS 客户端实例
    /// Create a GPT-SoVITS client instance.
    pub fn new(config: TtsCfg) -> Self {
        let timeout_secs = if config.timeout_secs == 0 {
            30
        } else {
            config.timeout_secs
        };
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        // 初始化条件：服务地址与参考音频均非空 / Init condition: both service URL and ref audio non-empty
        let initialized = !config.service_url.is_empty() && !config.ref_audio_path.is_empty();
        Self {
            config,
            http_client,
            initialized,
        }
    }

    /// 初始化客户端 — 验证服务地址与参考音频配置
    /// Initialize client — validate service URL and reference audio config.
    ///
    /// 数字生命意义：这是数字生命"声音身份"的确认时刻。
    /// 配置验证通过后，数字生命获得了使用自训练音色说话的能力。
    /// Digital life significance: this is the moment of confirming digital life's "voice identity".
    /// After config validation passes, digital life gains the ability to speak with a custom-trained voice.
    pub fn initialize(&mut self) -> Result<(), GptSoVitsError> {
        if self.initialized {
            return Ok(());
        }
        if self.config.service_url.is_empty() {
            // 服务地址为空 — 跳过初始化（降级模式）
            // Empty service URL — skip initialization (degraded mode)
            return Ok(());
        }
        if self.config.ref_audio_path.is_empty() {
            return Err(GptSoVitsError::RefAudioNotConfigured);
        }
        self.initialized = true;
        Ok(())
    }

    /// 合成语音 — 文本 + GPT-SoVITS 参数 → PCM 音频
    /// Synthesize speech — text + GPT-SoVITS params → PCM audio.
    ///
    /// 调用 Python GPT-SoVITS api_v2.py 的 `/tts` 端点，
    /// 发送 JSON 请求体，接收 WAV 音频流，解码为 f32 PCM 样本。
    /// Calls Python GPT-SoVITS api_v2.py `/tts` endpoint,
    /// sends JSON request body, receives WAV audio stream, decodes to f32 PCM samples.
    ///
    /// @param text 待合成文本 / Text to synthesize
    /// @param params GPT-SoVITS 合成参数 / GPT-SoVITS synthesis params
    /// @return 合成结果 / Synthesis result
    pub async fn synthesize(
        &self,
        text: &str,
        params: &GptSoVitsSynthesisParams,
    ) -> Result<GptSoVitsResult, GptSoVitsError> {
        let start = std::time::Instant::now();

        // 降级模式：服务地址为空，返回空 PCM
        // Degraded mode: empty service URL, return empty PCM
        if self.config.service_url.is_empty() {
            tracing::debug!("GPT-SoVITS 降级模式 — 服务地址为空，返回空 PCM / GPT-SoVITS degraded mode — empty service URL, returning empty PCM");
            return Ok(GptSoVitsResult {
                samples: Vec::new(),
                sample_rate: self.config.target_sample_rate,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        if self.config.ref_audio_path.is_empty() {
            return Err(GptSoVitsError::RefAudioNotConfigured);
        }

        let url = format!("{}/tts", self.config.service_url);

        // 构建请求体 — 对应 GPT-SoVITS api_v2.py 的 TTS_Request
        // Build request body — corresponds to GPT-SoVITS api_v2.py TTS_Request
        let body = serde_json::json!({
            "text": text,
            "text_lang": self.config.text_lang,
            "ref_audio_path": self.config.ref_audio_path,
            "prompt_text": self.config.prompt_text,
            "prompt_lang": self.config.prompt_lang,
            "top_k": params.top_k,
            "top_p": params.top_p,
            "temperature": params.temperature,
            "speed_factor": params.speed_factor,
            "fragment_interval": params.fragment_interval,
            "repetition_penalty": params.repetition_penalty,
            "media_type": "wav",
            "streaming_mode": self.config.streaming_mode,
        });

        tracing::debug!(
            "GPT-SoVITS 请求: text={}, speed={}, temp={}",
            text,
            params.speed_factor,
            params.temperature
        );

        // 发送 HTTP POST 请求 / Send HTTP POST request
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| GptSoVitsError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(GptSoVitsError::ServiceError { status, body });
        }

        // 接收 WAV 音频字节流 / Receive WAV audio byte stream
        let wav_bytes = resp
            .bytes()
            .await
            .map_err(|e| GptSoVitsError::DecodeFailed(e.to_string()))?;

        // WAV → f32 PCM 样本解码 / WAV → f32 PCM samples decoding
        let (samples, sample_rate) = decode_wav_to_f32(&wav_bytes)?;

        Ok(GptSoVitsResult {
            samples,
            sample_rate,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// 是否已初始化 / Whether initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 目标采样率（写入共享内存的采样率）/ Target sample rate (for shared memory)
    pub fn target_sample_rate(&self) -> u32 {
        self.config.target_sample_rate
    }

    /// 服务地址 / Service URL
    pub fn service_url(&self) -> &str {
        &self.config.service_url
    }
}

// ════════════════════════════════════════════════════════════════════
// WAV 解码工具 / WAV Decoding Utility
// ════════════════════════════════════════════════════════════════════

/// 将 WAV 字节流解码为 f32 PCM 样本
/// Decode WAV byte stream to f32 PCM samples.
///
/// 支持 float32 和 int16 两种采样格式，自动归一化到 [-1.0, 1.0]。
/// Supports both float32 and int16 sample formats, auto-normalized to [-1.0, 1.0].
///
/// @param wav_bytes WAV 文件字节流 / WAV file byte stream
/// @return (PCM 样本, 采样率) / (PCM samples, sample rate)
fn decode_wav_to_f32(wav_bytes: &[u8]) -> Result<(Vec<f32>, u32), GptSoVitsError> {
    let mut reader = hound::WavReader::new(Cursor::new(wav_bytes))
        .map_err(|e| GptSoVitsError::DecodeFailed(e.to_string()))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
        hound::SampleFormat::Int => {
            // int16 → f32 归一化 / int16 → f32 normalization
            reader
                .samples::<i16>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / 32768.0)
                .collect()
        }
    };

    Ok((samples, sample_rate))
}

/// 生成测试用 WAV 字节流（int16 格式，单声道，16kHz）
/// Generate test WAV byte stream (int16 format, mono, 16kHz).
#[cfg(test)]
fn make_test_wav_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut writer = hound::WavWriter::new(
            std::io::Cursor::new(&mut buf),
            hound::WavSpec {
                sample_rate,
                bits_per_sample: 16,
                channels: 1,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .expect("create wav writer");
        for &s in samples {
            // f32 [-1.0, 1.0] → i16 [-32768, 32767]
            let i16_sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(i16_sample).expect("write wav sample");
        }
        writer.finalize().expect("finalize wav");
    }
    buf
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> TtsCfg {
        TtsCfg {
            enabled: true,
            engine: "gpt-sovits".to_string(),
            // 空服务地址 — 降级模式 / Empty service URL — degraded mode
            service_url: String::new(),
            ref_audio_path: String::new(),
            prompt_text: String::new(),
            prompt_lang: "zh".to_string(),
            text_lang: "zh".to_string(),
            timeout_secs: 30,
            streaming_mode: 0,
            ..Default::default()
        }
    }

    fn test_cfg_with_url() -> TtsCfg {
        TtsCfg {
            enabled: true,
            engine: "gpt-sovits".to_string(),
            service_url: "http://127.0.0.1:9880".to_string(),
            ref_audio_path: "/test/ref.wav".to_string(),
            prompt_text: "测试参考音频".to_string(),
            prompt_lang: "zh".to_string(),
            text_lang: "zh".to_string(),
            timeout_secs: 30,
            streaming_mode: 0,
            ..Default::default()
        }
    }

    #[test]
    fn test_gpt_sovits_create() {
        // 创建客户端实例 / Create client instance
        let client = GptSoVitsClient::new(test_cfg());
        // 空服务地址不算初始化成功 / Empty service URL doesn't count as initialized
        assert!(!client.is_initialized());
        assert_eq!(client.target_sample_rate(), 16000);
        assert!(client.service_url().is_empty());
    }

    #[test]
    fn test_gpt_sovits_create_with_url() {
        // 有服务地址的客户端 / Client with service URL
        let client = GptSoVitsClient::new(test_cfg_with_url());
        assert!(client.is_initialized());
        assert_eq!(client.service_url(), "http://127.0.0.1:9880");
    }

    #[test]
    fn test_gpt_sovits_initialize_empty_url() {
        // 空服务地址 — 降级模式初始化成功 / Empty URL — degraded mode init succeeds
        let mut client = GptSoVitsClient::new(test_cfg());
        let result = client.initialize();
        assert!(result.is_ok());
        // 空地址不算初始化成功 / Empty URL doesn't count as initialized
        assert!(!client.is_initialized());
    }

    #[test]
    fn test_gpt_sovits_initialize_with_url_no_ref_audio() {
        // 有服务地址但无参考音频 — 返回错误 / URL but no ref audio — returns error
        let mut cfg = test_cfg_with_url();
        cfg.ref_audio_path = String::new();
        let mut client = GptSoVitsClient::new(cfg);
        // 构造时 ref_audio_path 为空，initialized 应为 false
        // constructed with empty ref_audio_path, initialized should be false
        assert!(!client.is_initialized());
        let result = client.initialize();
        assert!(result.is_err());
        match result {
            Err(GptSoVitsError::RefAudioNotConfigured) => {}
            _ => panic!("expected RefAudioNotConfigured error"),
        }
    }

    #[test]
    fn test_gpt_sovits_initialize_with_url_and_ref() {
        // 有服务地址且有参考音频 — 初始化成功 / URL and ref audio — init succeeds
        let mut client = GptSoVitsClient::new(test_cfg_with_url());
        // 已在 new() 中标记 initialized，重复初始化不报错
        // Already marked initialized in new(), double init doesn't error
        let result = client.initialize();
        assert!(result.is_ok());
        assert!(client.is_initialized());
    }

    #[test]
    fn test_gpt_sovits_double_initialize() {
        // 重复初始化不报错 / Double initialization doesn't error
        let mut client = GptSoVitsClient::new(test_cfg_with_url());
        client.initialize().unwrap();
        let result = client.initialize();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gpt_sovits_synthesize_degraded_mode() {
        // 降级模式合成 — 空服务地址返回空 PCM / Degraded mode — empty URL returns empty PCM
        let client = GptSoVitsClient::new(test_cfg());
        let params = GptSoVitsSynthesisParams {
            speed_factor: 1.0,
            temperature: 1.0,
            fragment_interval: 0.4,
            top_k: 15,
            top_p: 1.0,
            repetition_penalty: 1.35,
        };
        let result = client.synthesize("你好", &params).await.unwrap();
        assert!(result.samples.is_empty());
        assert_eq!(result.sample_rate, 16000);
    }

    #[tokio::test]
    async fn test_gpt_sovits_synthesize_no_ref_audio_error() {
        // 有服务地址但无参考音频 — 合成返回错误 / URL but no ref audio — synthesis returns error
        let mut cfg = test_cfg_with_url();
        cfg.ref_audio_path = String::new();
        // 需要绕过 initialized 检查 — 直接构造未初始化客户端
        // Bypass initialized check — directly construct uninitialized client
        let client = GptSoVitsClient::new(cfg);
        let params = GptSoVitsSynthesisParams {
            speed_factor: 1.0,
            temperature: 1.0,
            fragment_interval: 0.4,
            top_k: 15,
            top_p: 1.0,
            repetition_penalty: 1.35,
        };
        let result = client.synthesize("你好", &params).await;
        assert!(result.is_err());
        match result {
            Err(GptSoVitsError::RefAudioNotConfigured) => {}
            _ => panic!("expected RefAudioNotConfigured error"),
        }
    }

    #[test]
    fn test_decode_wav_to_f32_int16() {
        // int16 WAV 解码 / int16 WAV decoding
        let input_samples = vec![0.5, -0.5, 0.0, 0.25];
        let wav_bytes = make_test_wav_bytes(&input_samples, 16000);
        let (samples, sample_rate) = decode_wav_to_f32(&wav_bytes).unwrap();
        assert_eq!(sample_rate, 16000);
        assert_eq!(samples.len(), 4);
        // int16 量化误差约 1/32768 ≈ 3e-5，容差 1e-3
        // int16 quantization error ~1/32768 ≈ 3e-5, tolerance 1e-3
        assert!(
            (samples[0] - 0.5).abs() < 1e-3,
            "sample[0] should be ~0.5: got {}",
            samples[0]
        );
        assert!(
            (samples[1] + 0.5).abs() < 1e-3,
            "sample[1] should be ~-0.5: got {}",
            samples[1]
        );
        assert!(
            samples[2].abs() < 1e-3,
            "sample[2] should be ~0.0: got {}",
            samples[2]
        );
        assert!(
            (samples[3] - 0.25).abs() < 1e-3,
            "sample[3] should be ~0.25: got {}",
            samples[3]
        );
    }

    #[test]
    fn test_decode_wav_to_f32_empty() {
        // 空 WAV（无样本）/ Empty WAV (no samples)
        let input_samples: Vec<f32> = vec![];
        let wav_bytes = make_test_wav_bytes(&input_samples, 16000);
        let (samples, sample_rate) = decode_wav_to_f32(&wav_bytes).unwrap();
        assert_eq!(sample_rate, 16000);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_decode_wav_to_f32_invalid_data() {
        // 无效 WAV 数据 — 返回错误 / Invalid WAV data — returns error
        let invalid_bytes = b"not a wav file";
        let result = decode_wav_to_f32(invalid_bytes);
        assert!(result.is_err());
        match result {
            Err(GptSoVitsError::DecodeFailed(_)) => {}
            _ => panic!("expected DecodeFailed error"),
        }
    }

    #[test]
    fn test_decode_wav_to_f32_float_format() {
        // float32 WAV 解码 / float32 WAV decoding
        let input_samples = vec![0.5, -0.5, 0.0, 0.25];
        let mut buf = Vec::new();
        {
            let mut writer = hound::WavWriter::new(
                std::io::Cursor::new(&mut buf),
                hound::WavSpec {
                    sample_rate: 16000,
                    bits_per_sample: 32,
                    channels: 1,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .unwrap();
            for &s in &input_samples {
                writer.write_sample(s).unwrap();
            }
            writer.finalize().unwrap();
        }
        let (samples, sample_rate) = decode_wav_to_f32(&buf).unwrap();
        assert_eq!(sample_rate, 16000);
        assert_eq!(samples.len(), 4);
        // float32 无量化误差 / float32 has no quantization error
        assert!((samples[0] - 0.5).abs() < 1e-6);
        assert!((samples[1] + 0.5).abs() < 1e-6);
        assert!(samples[2].abs() < 1e-6);
        assert!((samples[3] - 0.25).abs() < 1e-6);
    }
}
