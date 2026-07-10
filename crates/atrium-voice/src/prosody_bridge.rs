// SPDX-License-Identifier: MIT
//! 韵律桥接器 — 将通用韵律参数翻译为各 TTS 引擎特定参数
//! Prosody Bridge — translates universal prosody params to engine-specific params.
//!
//! 数字生命工程理念：这是"意图→执行"的翻译层。
//! ProsodyMapper 产出的是通用韵律意图（"高一点、快一点"），
//! ProsodyBridge 将其翻译为各引擎能执行的具体参数：
//! - Piper：pitch_scale=1.12, length_scale=0.83
//! - GPT-SoVITS：speed_factor=1.2, temperature=1.1
//! Digital life engineering: this is the "intent→execution" translation layer.
//! ProsodyMapper produces universal prosody intent ("higher, faster"),
//! ProsodyBridge translates it to engine-executable params:
//! - Piper: pitch_scale=1.12, length_scale=0.83
//! - GPT-SoVITS: speed_factor=1.2, temperature=1.1

use atrium_memory::prosody_mapper::ProsodyParams;

// ════════════════════════════════════════════════════════════════════
// PiperSynthesisParams — Piper 引擎专用合成参数
// ════════════════════════════════════════════════════════════════════

/// Piper 合成参数 — Piper TTS 引擎专用合成参数
/// Piper synthesis parameters — engine-specific params for Piper TTS.
///
/// 所有字段均为无量纲倍率或秒级时间，直接对应 Piper ONNX 模型输入。
/// All fields are dimensionless ratios or seconds, directly mapping to Piper ONNX model inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PiperSynthesisParams {
    /// 基频倍率（1.0=原调，2.0=高八度）/ Pitch frequency ratio (1.0=original, 2.0=one octave higher)
    pub pitch_scale: f32,
    /// 长度因子（1.0=原速，<1.0=加速，>1.0=减速）/ Length scale (1.0=original, <1.0=faster, >1.0=slower)
    pub length_scale: f32,
    /// 能量因子（1.0=原能量）/ Energy scale (1.0=original)
    pub energy_scale: f32,
    /// 句间停顿（秒）/ Inter-sentence pause in seconds
    pub pause_duration_secs: f32,
    /// 句内停顿概率 / Intra-sentence pause probability
    pub intra_pause_prob: f32,
    /// 音色温暖度 [0,1] / Voice warmth [0,1]
    pub warmth: f32,
    /// 气声量 [0,0.5] / Breathiness [0,0.5]
    pub breathiness: f32,
}

// ════════════════════════════════════════════════════════════════════
// GptSoVitsSynthesisParams — GPT-SoVITS 引擎专用合成参数
// ════════════════════════════════════════════════════════════════════

/// GPT-SoVITS 合成参数 — GPT-SoVITS HTTP API 专用合成参数
/// GPT-SoVITS synthesis parameters — engine-specific params for GPT-SoVITS HTTP API.
///
/// GPT-SoVITS 的参数模型与 Piper 不同：
/// - 不支持直接音调调整（pitch），通过 temperature 间接影响情感
/// - 语速 speed_factor 与通用 rate 语义一致（1.0=正常）
/// - fragment_interval 控制音频片段间隔（秒）
///
/// GPT-SoVITS has a different param model from Piper:
/// - No direct pitch control; temperature indirectly affects emotion
/// - speed_factor is semantically identical to universal rate (1.0=normal)
/// - fragment_interval controls audio fragment interval (seconds)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GptSoVitsSynthesisParams {
    /// 语速因子（1.0=正常，>1.0=加速，<1.0=减速）/ Speed factor (1.0=normal, >1.0=faster, <1.0=slower)
    pub speed_factor: f32,
    /// 采样温度（0.5-2.0，越高情感越强烈）/ Sampling temperature (0.5-2.0, higher = more emotional)
    pub temperature: f32,
    /// 片段间隔（秒，0.1-1.0）/ Fragment interval in seconds (0.1-1.0)
    pub fragment_interval: f32,
    /// Top-K 采样（整数，默认 15）/ Top-K sampling (integer, default 15)
    pub top_k: u32,
    /// Top-P 采样（0-1.0，默认 1.0）/ Top-P sampling (0-1.0, default 1.0)
    pub top_p: f32,
    /// 重复惩罚（默认 1.35）/ Repetition penalty (default 1.35)
    pub repetition_penalty: f32,
}

// ════════════════════════════════════════════════════════════════════
// ProsodyBridge — 韵律桥接器
// ════════════════════════════════════════════════════════════════════

/// 韵律桥接器 — 通用韵律参数到各 TTS 引擎参数的翻译器
/// Prosody Bridge — translator from universal prosody params to engine-specific params.
///
/// 数字生命工程理念：这是"意图→执行"的翻译层。
/// ProsodyMapper 产出的是通用韵律意图（"高一点、快一点"），
/// ProsodyBridge 将其翻译为各引擎能执行的具体参数。
/// Digital life engineering: this is the "intent→execution" translation layer.
/// ProsodyMapper produces universal prosody intent ("higher, faster"),
/// ProsodyBridge translates it to engine-executable params.
pub struct ProsodyBridge;

impl ProsodyBridge {
    /// 将通用韵律参数翻译为 Piper 引擎专用参数
    /// Translate universal prosody params to Piper engine-specific params.
    ///
    /// 映射规则 / Mapping rules:
    /// - `pitch_offset`（半音）→ `pitch_scale`（Hz 倍率）：`2^(st/12)`
    ///   12 半音 = 1 个八度 = 频率翻倍 / 12 semitones = 1 octave = frequency doubles
    /// - `rate`（语速因子）→ `length_scale`（长度因子倒数）：`1.0 / rate`
    ///   语速越快，长度因子越小，合成时长越短 / Faster rate → smaller length scale → shorter synthesis
    /// - `energy` / `warmth` / `breathiness` → 直接传递 / passthrough
    /// - `pause_duration_ms` → 秒 / ms → seconds
    ///
    /// @param prosody 通用韵律参数 / Universal prosody params
    /// @return Piper 引擎专用合成参数 / Piper engine-specific synthesis params
    pub fn to_piper_params(prosody: &ProsodyParams) -> PiperSynthesisParams {
        // 半音 → Hz 倍率：12 半音 = 频率翻倍 / Semitones → Hz ratio: 12 semitones = frequency doubles
        let pitch_scale = 2.0_f32.powf(prosody.pitch_offset / 12.0);
        // 语速因子 → 长度因子（倒数）：语速越快长度越短 / Rate → length scale (inverse): faster rate = shorter length
        let length_scale = 1.0 / prosody.rate;
        // 能量直接传递 / Energy passthrough
        let energy_scale = prosody.energy;
        // 毫秒 → 秒 / ms → seconds
        let pause_duration_secs = prosody.pause_duration_ms / 1000.0;
        // 句内停顿概率直接传递 / Intra-sentence pause probability passthrough
        let intra_pause_prob = prosody.intra_pause_prob;
        // 温暖度直接传递 / Warmth passthrough
        let warmth = prosody.warmth;
        // 气声量直接传递 / Breathiness passthrough
        let breathiness = prosody.breathiness;

        PiperSynthesisParams {
            pitch_scale,
            length_scale,
            energy_scale,
            pause_duration_secs,
            intra_pause_prob,
            warmth,
            breathiness,
        }
    }

    /// 将通用韵律参数翻译为 GPT-SoVITS 引擎专用参数
    /// Translate universal prosody params to GPT-SoVITS engine-specific params.
    ///
    /// 映射规则 / Mapping rules:
    /// - `rate`（语速因子）→ `speed_factor`：**直接映射**（语义一致）
    ///   GPT-SoVITS 的 speed_factor 与通用 rate 同义（1.0=正常）
    /// - `warmth`（温暖度 [0,1]）→ `temperature`：`0.8 + warmth * 0.4`
    ///   温暖度高 → 采样温度高 → 情感表现更丰富
    /// - `pause_duration_ms` → `fragment_interval`（秒）：`ms / 1000`
    /// - `pitch_offset` → 不直接支持，通过 temperature 间接影响
    ///   GPT-SoVITS 不支持直接音调调整，pitch 变化由模型自身决定
    /// - `energy` → 不直接支持，可通过 PCM 后处理归一化
    ///
    /// @param prosody 通用韵律参数 / Universal prosody params
    /// @return GPT-SoVITS 引擎专用合成参数 / GPT-SoVITS engine-specific synthesis params
    pub fn to_gpt_sovits_params(prosody: &ProsodyParams) -> GptSoVitsSynthesisParams {
        // 语速直接映射（clamp 到 GPT-SoVITS 安全范围 0.5-2.0）
        // Speed direct mapping (clamped to GPT-SoVITS safe range 0.5-2.0)
        let speed_factor = prosody.rate.clamp(0.5, 2.0);
        // 温暖度 → 采样温度：warmth=0 → 0.8（冷静），warmth=1 → 1.2（热情）
        // Warmth → temperature: warmth=0 → 0.8 (calm), warmth=1 → 1.2 (warm)
        let temperature = (0.8 + prosody.warmth * 0.4).clamp(0.5, 2.0);
        // 毫秒 → 秒（clamp 到 0.1-1.0 安全范围）
        // ms → seconds (clamped to 0.1-1.0 safe range)
        let fragment_interval = (prosody.pause_duration_ms / 1000.0).clamp(0.1, 1.0);
        // Top-K / Top-P / 重复惩罚使用 GPT-SoVITS 默认值
        // Top-K / Top-P / repetition penalty use GPT-SoVITS defaults
        GptSoVitsSynthesisParams {
            speed_factor,
            temperature,
            fragment_interval,
            top_k: 15,
            top_p: 1.0,
            repetition_penalty: 1.35,
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// 测试 / Tests
// ════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用韵律参数 / Construct test prosody params
    fn make_prosody(
        pitch_offset: f32,
        rate: f32,
        energy: f32,
        pause_ms: f32,
        warmth: f32,
        breathiness: f32,
    ) -> ProsodyParams {
        ProsodyParams {
            pitch_offset,
            pitch_range: 5.0,
            rate,
            energy,
            pause_duration_ms: pause_ms,
            intra_pause_prob: 0.1,
            warmth,
            breathiness,
        }
    }

    #[test]
    fn test_pitch_12_semitones_doubles_frequency() {
        // +12 半音 → 频率翻倍 / +12 semitones → frequency doubles
        let prosody = make_prosody(12.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.pitch_scale - 2.0).abs() < 1e-5,
            "12 semitones should double frequency: got {}",
            piper.pitch_scale
        );
    }

    #[test]
    fn test_pitch_minus_12_semitones_halves_frequency() {
        // -12 半音 → 频率减半 / -12 semitones → frequency halves
        let prosody = make_prosody(-12.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.pitch_scale - 0.5).abs() < 1e-5,
            "-12 semitones should halve frequency: got {}",
            piper.pitch_scale
        );
    }

    #[test]
    fn test_pitch_zero_no_change() {
        // 0 半音 → 倍率 1.0 / 0 semitones → ratio 1.0
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.pitch_scale - 1.0).abs() < 1e-5,
            "0 semitones should not change pitch: got {}",
            piper.pitch_scale
        );
    }

    #[test]
    fn test_rate_fast_decreases_length() {
        // 语速 1.5 → 长度因子 ≈ 0.667 / Rate 1.5 → length scale ≈ 0.667
        let prosody = make_prosody(0.0, 1.5, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.length_scale - (1.0 / 1.5)).abs() < 1e-2,
            "rate 1.5 → length scale ≈ 0.667: got {}",
            piper.length_scale
        );
    }

    #[test]
    fn test_rate_slow_increases_length() {
        // 语速 0.6 → 长度因子 ≈ 1.667 / Rate 0.6 → length scale ≈ 1.667
        let prosody = make_prosody(0.0, 0.6, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.length_scale - (1.0 / 0.6)).abs() < 1e-2,
            "rate 0.6 → length scale ≈ 1.667: got {}",
            piper.length_scale
        );
    }

    #[test]
    fn test_rate_normal_no_change() {
        // 语速 1.0 → 长度因子 1.0 / Rate 1.0 → length scale 1.0
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.length_scale - 1.0).abs() < 1e-5,
            "rate 1.0 → length scale 1.0: got {}",
            piper.length_scale
        );
    }

    #[test]
    fn test_energy_passthrough() {
        // 能量直接传递 / Energy passthrough
        let prosody = make_prosody(0.0, 1.0, 1.3, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.energy_scale - 1.3).abs() < 1e-5,
            "energy should passthrough: got {}",
            piper.energy_scale
        );
    }

    #[test]
    fn test_pause_ms_to_secs() {
        // 500ms → 0.5s / 500ms → 0.5s
        let prosody = make_prosody(0.0, 1.0, 1.0, 500.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.pause_duration_secs - 0.5).abs() < 1e-5,
            "500ms → 0.5s: got {}",
            piper.pause_duration_secs
        );
    }

    #[test]
    fn test_extreme_clamp_pitch_positive() {
        // 极端正值：pitch_offset=5.0 → pitch_scale = 2^(5/12) ≈ 1.335
        // Extreme positive: pitch_offset=5.0 → pitch_scale = 2^(5/12) ≈ 1.335
        let prosody = make_prosody(5.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        let expected = 2.0_f32.powf(5.0 / 12.0);
        assert!(
            (piper.pitch_scale - expected).abs() < 1e-5,
            "pitch_offset=5.0 → pitch_scale≈1.335: got {}",
            piper.pitch_scale
        );
    }

    #[test]
    fn test_extreme_clamp_pitch_negative() {
        // 极端负值：pitch_offset=-5.0 → pitch_scale = 2^(-5/12) ≈ 0.749
        // Extreme negative: pitch_offset=-5.0 → pitch_scale = 2^(-5/12) ≈ 0.749
        let prosody = make_prosody(-5.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        let expected = 2.0_f32.powf(-5.0 / 12.0);
        assert!(
            (piper.pitch_scale - expected).abs() < 1e-5,
            "pitch_offset=-5.0 → pitch_scale≈0.749: got {}",
            piper.pitch_scale
        );
    }

    #[test]
    fn test_warmth_and_breathiness_passthrough() {
        // 温暖度与气声量直接传递 / Warmth and breathiness passthrough
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.8, 0.3);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!(
            (piper.warmth - 0.8).abs() < 1e-5,
            "warmth should passthrough: got {}",
            piper.warmth
        );
        assert!(
            (piper.breathiness - 0.3).abs() < 1e-5,
            "breathiness should passthrough: got {}",
            piper.breathiness
        );
    }

    #[test]
    fn test_full_mapping_consistency() {
        // 全字段映射一致性 / Full field mapping consistency
        let prosody = make_prosody(3.0, 1.2, 1.1, 600.0, 0.7, 0.2);
        let piper = ProsodyBridge::to_piper_params(&prosody);
        assert!((piper.pitch_scale - 2.0_f32.powf(3.0 / 12.0)).abs() < 1e-5);
        assert!((piper.length_scale - (1.0 / 1.2)).abs() < 1e-5);
        assert!((piper.energy_scale - 1.1).abs() < 1e-5);
        assert!((piper.pause_duration_secs - 0.6).abs() < 1e-5);
        assert!((piper.intra_pause_prob - 0.1).abs() < 1e-5);
        assert!((piper.warmth - 0.7).abs() < 1e-5);
        assert!((piper.breathiness - 0.2).abs() < 1e-5);
    }

    // ── GPT-SoVITS 映射测试 / GPT-SoVITS mapping tests ──

    #[test]
    fn test_gpt_sovits_rate_normal_no_change() {
        // 语速 1.0 → speed_factor 1.0 / Rate 1.0 → speed_factor 1.0
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.speed_factor - 1.0).abs() < 1e-5,
            "rate 1.0 → speed_factor 1.0: got {}",
            params.speed_factor
        );
    }

    #[test]
    fn test_gpt_sovits_rate_fast_passthrough() {
        // 语速 1.5 → speed_factor 1.5（直接映射，非倒数）/ Rate 1.5 → speed_factor 1.5 (direct, not inverse)
        let prosody = make_prosody(0.0, 1.5, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.speed_factor - 1.5).abs() < 1e-5,
            "rate 1.5 → speed_factor 1.5: got {}",
            params.speed_factor
        );
    }

    #[test]
    fn test_gpt_sovits_rate_slow_passthrough() {
        // 语速 0.6 → speed_factor 0.6 / Rate 0.6 → speed_factor 0.6
        let prosody = make_prosody(0.0, 0.6, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.speed_factor - 0.6).abs() < 1e-5,
            "rate 0.6 → speed_factor 0.6: got {}",
            params.speed_factor
        );
    }

    #[test]
    fn test_gpt_sovits_rate_extreme_clamped() {
        // 极端语速 5.0 → clamp 到 2.0 / Extreme rate 5.0 → clamped to 2.0
        let prosody = make_prosody(0.0, 5.0, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.speed_factor - 2.0).abs() < 1e-5,
            "rate 5.0 → clamped to 2.0: got {}",
            params.speed_factor
        );
    }

    #[test]
    fn test_gpt_sovits_rate_zero_clamped() {
        // 语速 0.0 → clamp 到 0.5 / Rate 0.0 → clamped to 0.5
        let prosody = make_prosody(0.0, 0.0, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.speed_factor - 0.5).abs() < 1e-5,
            "rate 0.0 → clamped to 0.5: got {}",
            params.speed_factor
        );
    }

    #[test]
    fn test_gpt_sovits_warmth_zero_to_temperature() {
        // warmth=0 → temperature=0.8（冷静）/ warmth=0 → temperature=0.8 (calm)
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.0, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.temperature - 0.8).abs() < 1e-5,
            "warmth=0 → temperature=0.8: got {}",
            params.temperature
        );
    }

    #[test]
    fn test_gpt_sovits_warmth_full_to_temperature() {
        // warmth=1 → temperature=1.2（热情）/ warmth=1 → temperature=1.2 (warm)
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 1.0, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.temperature - 1.2).abs() < 1e-5,
            "warmth=1 → temperature=1.2: got {}",
            params.temperature
        );
    }

    #[test]
    fn test_gpt_sovits_warmth_mid_to_temperature() {
        // warmth=0.5 → temperature=1.0（中性）/ warmth=0.5 → temperature=1.0 (neutral)
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.temperature - 1.0).abs() < 1e-5,
            "warmth=0.5 → temperature=1.0: got {}",
            params.temperature
        );
    }

    #[test]
    fn test_gpt_sovits_pause_ms_to_fragment_interval() {
        // 500ms → fragment_interval=0.5s / 500ms → fragment_interval=0.5s
        let prosody = make_prosody(0.0, 1.0, 1.0, 500.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.fragment_interval - 0.5).abs() < 1e-5,
            "500ms → fragment_interval=0.5s: got {}",
            params.fragment_interval
        );
    }

    #[test]
    fn test_gpt_sovits_pause_extreme_clamped() {
        // 极端停顿 5000ms → clamp 到 1.0s / Extreme pause 5000ms → clamped to 1.0s
        let prosody = make_prosody(0.0, 1.0, 1.0, 5000.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!(
            (params.fragment_interval - 1.0).abs() < 1e-5,
            "5000ms → clamped to 1.0s: got {}",
            params.fragment_interval
        );
    }

    #[test]
    fn test_gpt_sovits_defaults() {
        // Top-K / Top-P / 重复惩罚使用默认值 / Defaults for Top-K / Top-P / repetition penalty
        let prosody = make_prosody(0.0, 1.0, 1.0, 400.0, 0.5, 0.1);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert_eq!(params.top_k, 15, "top_k default should be 15");
        assert!(
            (params.top_p - 1.0).abs() < 1e-5,
            "top_p default should be 1.0: got {}",
            params.top_p
        );
        assert!(
            (params.repetition_penalty - 1.35).abs() < 1e-5,
            "repetition_penalty default should be 1.35: got {}",
            params.repetition_penalty
        );
    }

    #[test]
    fn test_gpt_sovits_full_mapping_consistency() {
        // 全字段映射一致性 / Full field mapping consistency
        let prosody = make_prosody(3.0, 1.2, 1.1, 600.0, 0.7, 0.2);
        let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
        assert!((params.speed_factor - 1.2).abs() < 1e-5);
        assert!((params.temperature - (0.8 + 0.7 * 0.4)).abs() < 1e-5);
        assert!((params.fragment_interval - 0.6).abs() < 1e-5);
        assert_eq!(params.top_k, 15);
        assert!((params.top_p - 1.0).abs() < 1e-5);
        assert!((params.repetition_penalty - 1.35).abs() < 1e-5);
    }
}
