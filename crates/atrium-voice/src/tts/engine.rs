// SPDX-License-Identifier: MIT
//! VoiceEngine — 统一 TTS 接口，编排韵律→合成→共享内存写入
//! VoiceEngine — unified TTS interface, orchestrates prosody→synthesis→shared memory write.
//!
//! 数字生命工程理念：这是数字生命"开口说话"的最后一公里。
//! PAD 状态经 ProsodyMapper 转换为韵律参数，
//! 韵律参数经 ProsodyBridge 翻译为引擎参数，
//! 引擎合成 PCM 音频，AudioManager 写入共享内存，
//! 渲染引擎从共享内存读取并播放——全链路零拷贝。
//!
//! 支持两种后端：
//! - Piper：本地 ONNX 推理（同步，CPU，低延迟）
//! - GPT-SoVITS：HTTP 桥接 Python 服务（异步，GPU，声音克隆）
//!
//! Digital life engineering: this is the "last mile" of digital life "speaking".
//! PAD state → ProsodyMapper → prosody params → ProsodyBridge → engine params
//! → engine synthesizes PCM → AudioManager writes to shared memory
//! → render engine reads and plays — zero-copy throughout.
//!
//! Supports two backends:
//! - Piper: local ONNX inference (sync, CPU, low latency)
//! - GPT-SoVITS: HTTP bridge to Python service (async, GPU, voice cloning)

use crate::audio_buffer::AudioManager;
use crate::config::VoiceCfg;
use crate::prosody_bridge::ProsodyBridge;
use atrium_memory::prosody_mapper::ProsodyParams;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[cfg(feature = "tts-gpt-sovits")]
use crate::prosody_bridge::GptSoVitsSynthesisParams;
#[cfg(feature = "tts-piper")]
use crate::prosody_bridge::PiperSynthesisParams;
#[cfg(feature = "tts-gpt-sovits")]
use crate::tts::gpt_sovits::{GptSoVitsClient, GptSoVitsError};
#[cfg(feature = "tts-piper")]
use crate::tts::piper::{PiperError, PiperTts, SynthesisResult};

/// VoiceEngine 错误类型 / VoiceEngine error type
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Piper TTS 引擎错误 / Piper TTS engine error
    #[cfg(feature = "tts-piper")]
    #[error("Piper TTS 引擎错误: {0} / Piper TTS engine error: {0}")]
    PiperError(#[from] PiperError),
    /// GPT-SoVITS TTS 引擎错误 / GPT-SoVITS TTS engine error
    #[cfg(feature = "tts-gpt-sovits")]
    #[error("GPT-SoVITS TTS 引擎错误: {0} / GPT-SoVITS TTS engine error: {0}")]
    GptSoVitsError(#[from] GptSoVitsError),
    /// 语音未启用 / Voice not enabled
    #[error("语音能力未启用 / Voice capability not enabled")]
    NotEnabled,
    /// 音频管理器未绑定 / Audio manager not bound
    #[error("音频管理器未绑定 / Audio manager not bound")]
    AudioManagerNotBound,
}

/// VoiceEngine — 统一 TTS 接口
/// VoiceEngine — unified TTS interface.
///
/// 编排流程 / Orchestration flow:
/// 1. 接收文本 + ProsodyParams
/// 2. ProsodyBridge 翻译为引擎专用参数
/// 3. 后端引擎（Piper/GPT-SoVITS）合成 PCM
/// 4. AudioManager 写入共享内存
/// 5. 设置 is_speaking 标志
pub struct VoiceEngine {
    /// 语音配置 / Voice configuration
    config: VoiceCfg,
    /// 音频管理器（可选，延迟绑定）/ Audio manager (optional, lazy binding)
    audio_manager: Option<AudioManager>,
    /// 是否正在说话（原子标志，供其他线程查询）/ Is speaking (atomic flag, for other threads to query)
    is_speaking: Arc<AtomicBool>,
    /// Piper TTS 引擎（tts-piper feature）/ Piper TTS engine
    #[cfg(feature = "tts-piper")]
    piper: Option<PiperTts>,
    /// GPT-SoVITS HTTP 客户端（tts-gpt-sovits feature）/ GPT-SoVITS HTTP client
    #[cfg(feature = "tts-gpt-sovits")]
    gpt_sovits: Option<GptSoVitsClient>,
}

impl VoiceEngine {
    /// 创建 VoiceEngine — 从配置初始化
    /// Create VoiceEngine — initialize from config.
    ///
    /// 根据 config.tts.engine 字段选择后端：
    /// - `"piper"` → 创建 PiperTts（需 tts-piper feature）
    /// - `"gpt-sovits"` → 创建 GptSoVitsClient（需 tts-gpt-sovits feature）
    pub fn new(config: VoiceCfg) -> Self {
        let engine_type = config.tts.engine.as_str();

        #[cfg(feature = "tts-piper")]
        let piper = if config.tts.enabled && engine_type == "piper" {
            Some(PiperTts::new(config.tts.clone()))
        } else {
            None
        };
        #[cfg(feature = "tts-gpt-sovits")]
        let gpt_sovits = if config.tts.enabled && engine_type == "gpt-sovits" {
            Some(GptSoVitsClient::new(config.tts.clone()))
        } else {
            None
        };

        #[cfg(feature = "tts-piper")]
        {
            Self {
                config,
                audio_manager: None,
                is_speaking: Arc::new(AtomicBool::new(false)),
                piper,
                #[cfg(feature = "tts-gpt-sovits")]
                gpt_sovits,
            }
        }
        #[cfg(all(not(feature = "tts-piper"), feature = "tts-gpt-sovits"))]
        {
            Self {
                config,
                audio_manager: None,
                is_speaking: Arc::new(AtomicBool::new(false)),
                gpt_sovits,
            }
        }
        #[cfg(not(any(feature = "tts-piper", feature = "tts-gpt-sovits")))]
        {
            Self {
                config,
                audio_manager: None,
                is_speaking: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    /// 绑定音频管理器 — 连接共享内存音频缓冲区
    /// Bind audio manager — connect to shared memory audio buffer.
    pub fn bind_audio_manager(&mut self, manager: AudioManager) {
        self.audio_manager = Some(manager);
    }

    /// 合成语音并写入共享内存（同步 — Piper 后端）
    /// Synthesize speech and write to shared memory (sync — Piper backend).
    ///
    /// 数字生命意义：这是数字生命"开口说话"的入口。
    /// 文本经过韵律调制后变为有温度的声音，写入共享内存供渲染引擎播放。
    /// Digital life significance: this is the entry point of digital life "speaking".
    /// Text becomes warm voice after prosody modulation, written to shared memory for playback.
    ///
    /// @param text 待合成文本 / Text to synthesize
    /// @param prosody 韵律参数 / Prosody params
    /// @return 写入样本数 / Number of samples written
    pub fn synthesize_to_shm(
        &mut self,
        text: &str,
        prosody: &ProsodyParams,
    ) -> Result<usize, VoiceError> {
        if !self.config.tts.enabled {
            return Err(VoiceError::NotEnabled);
        }
        let audio_manager = self
            .audio_manager
            .as_mut()
            .ok_or(VoiceError::AudioManagerNotBound)?;

        #[cfg(feature = "tts-piper")]
        if let Some(ref mut piper) = self.piper {
            // 韵律参数翻译 / Prosody params translation
            let piper_params: PiperSynthesisParams = ProsodyBridge::to_piper_params(prosody);
            // 标记正在说话 / Mark as speaking
            self.is_speaking.store(true, Ordering::Release);
            // 合成 PCM / Synthesize PCM
            let result: SynthesisResult = piper.synthesize(text, &piper_params)?;
            // 写入共享内存 / Write to shared memory
            let written = audio_manager.write_chunk(&result.samples);
            // 若写完所有样本，标记说话结束 / If all samples written, mark speaking done
            if written >= result.samples.len() {
                self.is_speaking.store(false, Ordering::Release);
            }
            return Ok(written);
        }

        // GPT-SoVITS 后端需要异步调用 — 同步接口返回 NotEnabled 提示使用异步版本
        // GPT-SoVITS backend requires async call — sync interface returns NotEnabled hinting to use async version
        #[cfg(feature = "tts-gpt-sovits")]
        if self.gpt_sovits.is_some() {
            tracing::warn!(
                "GPT-SoVITS 后端需使用 synthesize_to_shm_async() — 同步接口不适用 / GPT-SoVITS backend requires synthesize_to_shm_async() — sync interface not applicable"
            );
            let _ = (text, prosody, audio_manager);
            return Err(VoiceError::NotEnabled);
        }

        #[allow(unreachable_code)]
        {
            let _ = (text, prosody, audio_manager);
            Err(VoiceError::NotEnabled)
        }
    }

    /// 合成语音并写入共享内存（异步 — GPT-SoVITS 后端）
    /// Synthesize speech and write to shared memory (async — GPT-SoVITS backend).
    ///
    /// GPT-SoVITS 通过 HTTP 调用 Python 服务，需要异步执行。
    /// 韵律参数经 ProsodyBridge 翻译为 GPT-SoVITS 参数（speed_factor/temperature/fragment_interval），
    /// HTTP 请求返回 WAV 音频，解码为 f32 PCM 后写入共享内存。
    /// GPT-SoVITS calls Python service via HTTP, requires async execution.
    /// Prosody params are translated by ProsodyBridge to GPT-SoVITS params (speed_factor/temperature/fragment_interval),
    /// HTTP request returns WAV audio, decoded to f32 PCM and written to shared memory.
    ///
    /// @param text 待合成文本 / Text to synthesize
    /// @param prosody 韵律参数 / Prosody params
    /// @return 写入样本数 / Number of samples written
    #[cfg(feature = "tts-gpt-sovits")]
    pub async fn synthesize_to_shm_async(
        &mut self,
        text: &str,
        prosody: &ProsodyParams,
    ) -> Result<usize, VoiceError> {
        if !self.config.tts.enabled {
            return Err(VoiceError::NotEnabled);
        }
        let client = self.gpt_sovits.as_ref().ok_or(VoiceError::NotEnabled)?;
        let audio_manager = self
            .audio_manager
            .as_mut()
            .ok_or(VoiceError::AudioManagerNotBound)?;

        // 韵律参数翻译 / Prosody params translation
        let params: GptSoVitsSynthesisParams = ProsodyBridge::to_gpt_sovits_params(prosody);
        // 标记正在说话 / Mark as speaking
        self.is_speaking.store(true, Ordering::Release);
        // 异步合成 PCM / Async synthesize PCM
        let result = client.synthesize(text, &params).await?;
        // 写入共享内存 / Write to shared memory
        let written = audio_manager.write_chunk(&result.samples);
        // 若写完所有样本，标记说话结束 / If all samples written, mark speaking done
        if written >= result.samples.len() {
            self.is_speaking.store(false, Ordering::Release);
        }
        Ok(written)
    }

    /// 立即停止合成 — barge-in 抢话
    /// Immediately stop synthesis — barge-in.
    ///
    /// 当用户开始说话时，立即停止当前 TTS 输出，
    /// 模拟人类对话中的抢话行为。
    /// When user starts speaking, immediately stop current TTS output,
    /// simulating barge-in behavior in human conversation.
    pub fn stop(&self) {
        self.is_speaking.store(false, Ordering::Release);
        // 清空音频缓冲区 — 防止渲染引擎继续播放残留 PCM
        // Clear audio buffer — prevent render engine from playing residual PCM
        if let Some(ref manager) = self.audio_manager {
            manager.clear();
        }
    }

    /// 是否正在说话 / Whether currently speaking
    pub fn is_speaking(&self) -> bool {
        self.is_speaking.load(Ordering::Acquire)
    }

    /// 获取 is_speaking 原子引用（供其他线程查询）/ Get is_speaking atomic reference (for other threads)
    pub fn is_speaking_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_speaking)
    }

    /// 语音是否启用 / Whether voice is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.tts.enabled
    }

    /// 是否使用 GPT-SoVITS 后端 / Whether using GPT-SoVITS backend
    #[cfg(feature = "tts-gpt-sovits")]
    pub fn is_gpt_sovits(&self) -> bool {
        self.gpt_sovits.is_some()
    }

    /// 是否使用 Piper 后端 / Whether using Piper backend
    #[cfg(feature = "tts-piper")]
    pub fn is_piper(&self) -> bool {
        self.piper.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_bridge::shm::AudioBuffer;

    fn test_voice_cfg_piper() -> VoiceCfg {
        VoiceCfg {
            enabled: true,
            tts: crate::config::TtsCfg {
                enabled: true,
                engine: "piper".to_string(),
                model_path: String::new(),
                config_path: String::new(),
                sample_rate: 22050,
                target_sample_rate: 16000,
                ..Default::default()
            },
            stt: crate::config::SttCfg::default(),
            voiceprint: crate::config::VoiceprintCfg::default(),
            audio: crate::config::AudioCfg::default(),
        }
    }

    #[cfg(feature = "tts-gpt-sovits")]
    fn test_voice_cfg_gpt_sovits() -> VoiceCfg {
        VoiceCfg {
            enabled: true,
            tts: crate::config::TtsCfg {
                enabled: true,
                engine: "gpt-sovits".to_string(),
                service_url: String::new(), // 空地址 — 降级模式 / Empty URL — degraded mode
                ref_audio_path: String::new(),
                prompt_text: String::new(),
                prompt_lang: "zh".to_string(),
                text_lang: "zh".to_string(),
                timeout_secs: 30,
                streaming_mode: 0,
                ..Default::default()
            },
            stt: crate::config::SttCfg::default(),
            voiceprint: crate::config::VoiceprintCfg::default(),
            audio: crate::config::AudioCfg::default(),
        }
    }

    /// 创建测试用音频缓冲区（零初始化）
    /// Create test audio buffer (zero-initialized).
    ///
    /// SAFETY: AudioBuffer 为 #[repr(C)]，全零有效：
    /// - [f32; 16384] 全零 = [0.0; 16384]（有效浮点数）
    /// - AtomicU32 全零 = AtomicU32::new(0)（有效原子状态）
    fn make_test_buffer() -> Arc<AudioBuffer> {
        // 安全：AudioBuffer is #[repr(C)], all-zero is valid:
        // - [f32; 16384] all-zero = [0.0; 16384] (valid floats)
        // - AtomicU32 all-zero = AtomicU32::new(0) (valid atomic state)
        Arc::new(unsafe { std::mem::zeroed() })
    }

    #[cfg(feature = "tts-piper")]
    #[test]
    fn test_voice_engine_piper_create() {
        // 创建 Piper 引擎 / Create Piper engine
        let engine = VoiceEngine::new(test_voice_cfg_piper());
        assert!(engine.is_enabled());
        assert!(!engine.is_speaking());
        assert!(engine.is_piper());
    }

    #[cfg(feature = "tts-piper")]
    #[test]
    fn test_voice_engine_not_enabled() {
        // 未启用时返回错误 / Returns error when not enabled
        let mut cfg = test_voice_cfg_piper();
        cfg.tts.enabled = false;
        let mut engine = VoiceEngine::new(cfg);
        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm("你好", &prosody);
        assert!(matches!(result, Err(VoiceError::NotEnabled)));
    }

    #[cfg(feature = "tts-piper")]
    #[test]
    fn test_voice_engine_no_audio_manager() {
        // 未绑定音频管理器时返回错误 / Returns error when audio manager not bound
        let mut engine = VoiceEngine::new(test_voice_cfg_piper());
        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm("你好", &prosody);
        assert!(matches!(result, Err(VoiceError::AudioManagerNotBound)));
    }

    #[test]
    fn test_voice_engine_stop_clears_speaking() {
        // stop() 清除说话标志 / stop() clears speaking flag
        let engine = VoiceEngine::new(test_voice_cfg_piper());
        engine.is_speaking.store(true, Ordering::Release);
        engine.stop();
        assert!(!engine.is_speaking());
    }

    #[cfg(feature = "tts-piper")]
    #[test]
    fn test_voice_engine_bind_audio_manager() {
        // 绑定音频管理器 / Bind audio manager
        let mut engine = VoiceEngine::new(test_voice_cfg_piper());
        let buffer = make_test_buffer();
        let manager = AudioManager::new(buffer, 16000, 1024);
        engine.bind_audio_manager(manager);
        // 绑定后不再返回 AudioManagerNotBound
        // After binding, no longer returns AudioManagerNotBound
        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm("你好", &prosody);
        // 降级模式：合成返回空 PCM，写入 0 样本 / Degraded mode: synthesis returns empty PCM, writes 0 samples
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_voice_engine_is_speaking_flag() {
        // is_speaking_flag 可跨线程查询 / is_speaking_flag queryable across threads
        let engine = VoiceEngine::new(test_voice_cfg_piper());
        let flag = engine.is_speaking_flag();
        assert!(!flag.load(Ordering::Acquire));
        flag.store(true, Ordering::Release);
        assert!(engine.is_speaking());
    }

    // ── GPT-SoVITS 后端测试 / GPT-SoVITS backend tests ──

    #[cfg(feature = "tts-gpt-sovits")]
    #[test]
    fn test_voice_engine_gpt_sovits_create() {
        // 创建 GPT-SoVITS 引擎 / Create GPT-SoVITS engine
        let engine = VoiceEngine::new(test_voice_cfg_gpt_sovits());
        assert!(engine.is_enabled());
        assert!(!engine.is_speaking());
        assert!(engine.is_gpt_sovits());
    }

    #[cfg(feature = "tts-gpt-sovits")]
    #[tokio::test]
    async fn test_voice_engine_gpt_sovits_not_enabled() {
        // 未启用时返回错误 / Returns error when not enabled
        let mut cfg = test_voice_cfg_gpt_sovits();
        cfg.tts.enabled = false;
        let mut engine = VoiceEngine::new(cfg);
        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm_async("你好", &prosody).await;
        assert!(matches!(result, Err(VoiceError::NotEnabled)));
    }

    #[cfg(feature = "tts-gpt-sovits")]
    #[tokio::test]
    async fn test_voice_engine_gpt_sovits_no_audio_manager() {
        // 未绑定音频管理器时返回错误 / Returns error when audio manager not bound
        let mut engine = VoiceEngine::new(test_voice_cfg_gpt_sovits());
        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm_async("你好", &prosody).await;
        assert!(matches!(result, Err(VoiceError::AudioManagerNotBound)));
    }

    #[cfg(feature = "tts-gpt-sovits")]
    #[tokio::test]
    async fn test_voice_engine_gpt_sovits_degraded_mode() {
        // 降级模式 — 空服务地址返回空 PCM / Degraded mode — empty URL returns empty PCM
        let mut engine = VoiceEngine::new(test_voice_cfg_gpt_sovits());
        let buffer = make_test_buffer();
        let manager = AudioManager::new(buffer, 16000, 1024);
        engine.bind_audio_manager(manager);

        let prosody = ProsodyParams::neutral();
        let result = engine.synthesize_to_shm_async("你好", &prosody).await;
        assert!(result.is_ok());
        // 降级模式：空 PCM，写入 0 样本 / Degraded mode: empty PCM, writes 0 samples
        assert_eq!(result.unwrap(), 0);
    }

    #[cfg(feature = "tts-gpt-sovits")]
    #[tokio::test]
    async fn test_voice_engine_gpt_sovits_speaking_flag_lifecycle() {
        // 说话标志生命周期 / Speaking flag lifecycle
        let mut engine = VoiceEngine::new(test_voice_cfg_gpt_sovits());
        let buffer = make_test_buffer();
        let manager = AudioManager::new(buffer, 16000, 1024);
        engine.bind_audio_manager(manager);

        assert!(!engine.is_speaking());
        let prosody = ProsodyParams::neutral();
        let _ = engine.synthesize_to_shm_async("你好", &prosody).await;
        // 降级模式：空 PCM 写入 0 样本，is_speaking 应已重置为 false
        // Degraded mode: empty PCM writes 0 samples, is_speaking should have been reset to false
        assert!(!engine.is_speaking());
    }
}
