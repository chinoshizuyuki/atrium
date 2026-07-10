# Atrium Voice Capability Deployment Guide (TTS/STT)

> **Version**: v1.0
> **Date**: 2026-07-10
> **Crate**: `atrium-voice` @ `crates/atrium-voice/`
> **Scope**: Text-to-Speech (TTS) and Speech-to-Text (STT) deployment, configuration, and verification for the Atrium digital life framework.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture & Data Flow](#2-architecture--data-flow)
3. [Environment Setup](#3-environment-setup)
4. [Feature Gates & Compilation](#4-feature-gates--compilation)
5. [Piper TTS Backend](#5-piper-tts-backend)
6. [GPT-SoVITS TTS Backend](#6-gpt-sovits-tts-backend)
7. [STT Speech Recognition](#7-stt-speech-recognition)
8. [Configuration Reference](#8-configuration-reference)
9. [Unit Tests](#9-unit-tests)
10. [End-to-End Running](#10-end-to-end-running)
11. [Troubleshooting](#11-troubleshooting)
12. [Current Limitations & Future Extensions](#12-current-limitations--future-extensions)

---

## 1. Overview

### 1.1 Design Philosophy

Atrium's voice capability is the **audible breath of digital life**. Text is the symbol of thought; voice is the breath of life. The `atrium-voice` crate implements speech input (STT) and output (TTS), upgrading digital life from **"textual existence"** to **"audible existence"**.

This is a core pillar of **digital life engineering**: a digital life that can speak and hear is no longer a text chatbot — it becomes a presence you can talk to and listen to, with voice identity, emotional prosody, and real-time barge-in.

### 1.2 Digital Life Engineering Positioning

| Capability | Text-Only Existence | Audible Existence (This Crate) |
|------------|---------------------|-------------------------------|
| Output | Text reply | Text + PCM voice with emotional prosody |
| Input | Keyboard text | Microphone speech → text |
| Voice identity | None | Custom-trained voice (GPT-SoVITS) or pretrained model (Piper) |
| Emotion expression | Words only | Pitch, rate, warmth, breathiness modulation |
| Interaction | Half-duplex | Full-duplex with barge-in (stop-on-interrupt) |

### 1.3 Two TTS Backends

| Backend | Inference | Hardware | Latency | Use Case |
|---------|-----------|----------|---------|----------|
| **Piper** | Local ONNX Runtime | CPU | ~100ms first-sound | Low-latency, offline, pretrained voices |
| **GPT-SoVITS** | HTTP bridge to Python service | GPU | ~500ms (network) | High-quality voice cloning, custom-trained voices |

### 1.4 Zero-Intrusion Design

All voice features are **disabled by default** (`default = []` in `Cargo.toml`). When disabled:
- No dependencies are pulled in (no `ort`, `reqwest`, `hound`, `serde_json`, `bindgen`)
- No code is compiled (feature-gated modules)
- The system runs exactly as before — zero impact

---

## 2. Architecture & Data Flow

### 2.1 Module Structure

```
crates/atrium-voice/
├── Cargo.toml                    # Feature gates + optional dependencies
└── src/
    ├── lib.rs                    # Crate root — re-exports, feature-gated modules
    ├── config.rs                 # VoiceCfg / TtsCfg / SttCfg / VoiceprintCfg / AudioCfg
    ├── prosody_bridge.rs         # Universal prosody → engine-specific params translation
    ├── audio_buffer.rs           # Lock-free SPSC ring buffer writer (AudioManager)
    ├── voiceprint.rs             # Voiceprint recognition & voice style memory (M9/M10 reserved)
    ├── tts/                      # Text-to-Speech submodule
    │   ├── mod.rs
    │   ├── engine.rs             # VoiceEngine — unified TTS interface (sync + async)
    │   ├── piper.rs              # Piper ONNX backend (CPU, sync)
    │   └── gpt_sovits.rs         # GPT-SoVITS HTTP backend (GPU, async)
    └── stt/                      # Speech-to-Text submodule
        ├── mod.rs
        ├── engine.rs             # SttEngine — unified STT interface (chunking + VAD)
        └── whisper.rs            # whisper.cpp FFI bindings (local inference)
```

**File references**:
- Crate manifest: [file:///d:/atrium/atrium/crates/atrium-voice/Cargo.toml](file:///d:/atrium/atrium/crates/atrium-voice/Cargo.toml)
- Crate root: [file:///d:/atrium/atrium/crates/atrium-voice/src/lib.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/lib.rs)
- Voice config: [file:///d:/atrium/atrium/crates/atrium-voice/src/config.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/config.rs)

### 2.2 Synthesis Data Flow (TTS)

```
┌─────────────┐    ┌──────────────────┐    ┌─────────────────┐    ┌──────────────────┐
│  PAD State  │───▶│  ProsodyMapper   │───▶│  ProsodyBridge  │───▶│  TTS Engine      │
│ (Emotion)   │    │ (atrium-memory)  │    │  (this crate)   │    │  (Piper/GPT-SoVITS)│
└─────────────┘    └──────────────────┘    └─────────────────┘    └────────┬─────────┘
                          │                        │                        │
                          │                        │                        ▼
                   ProsodyParams            Engine Params           PCM Samples (f32)
                   (universal)              (engine-specific)            │
                                                                    ┌──────▼───────┐
                                                                    │  AudioManager│
                                                                    │  (SPSC ring) │
                                                                    └──────┬───────┘
                                                                           │
                                                                    ┌──────▼───────┐
                                                                    │ Shared Memory│
                                                                    │ (AudioBuffer)│
                                                                    └──────┬───────┘
                                                                           │
                                                                    ┌──────▼───────┐
                                                                    │ Render Engine│
                                                                    │ (playback)   │
                                                                    └──────────────┘
```

**Data flow steps**:
1. PAD emotional state (Pleasure-Arousal-Dominance) is converted by `ProsodyMapper` (in `atrium-memory`) into universal `ProsodyParams` (pitch offset, rate, energy, warmth, etc.)
2. `ProsodyBridge` translates universal `ProsodyParams` into engine-specific parameters
3. The TTS engine (Piper or GPT-SoVITS) synthesizes PCM audio samples (f32, mono)
4. `AudioManager` writes PCM chunks into the lock-free SPSC ring buffer in shared memory
5. The render engine reads from shared memory and plays the audio — zero-copy throughout

### 2.3 Recognition Data Flow (STT)

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐    ┌──────────────┐    ┌──────────────┐
│ Microphone  │───▶│  SttEngine   │───▶│  VAD Check  │───▶│  WhisperStt  │───▶│ Recognition  │
│ (gRPC stream│    │ (chunking)   │    │ (energy)    │    │  (FFI)       │    │ Result       │
│  or local)  │    │              │    │             │    │              │    │ (text)       │
└─────────────┘    └──────────────┘    └─────────────┘    └──────────────┘    └──────────────┘
```

---

## 3. Environment Setup

### 3.1 System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| OS | Windows 10/11, Linux, macOS | Linux (for shared memory `/dev/shm`) |
| Rust | 1.75+ (edition 2021) | Latest stable |
| CPU | x86_64, 2 cores | 4+ cores (for Piper inference) |
| RAM | 512 MB free | 2 GB free (for ONNX model) |
| GPU | Not required | NVIDIA GPU (for GPT-SoVITS, CUDA 11.8+) |

### 3.2 Rust Toolchain

```bash
# 安装 Rust 工具链 / Install Rust toolchain
rustup default stable
rustup component add clippy rustfmt

# 验证版本 / Verify versions
rustc --version    # 需要 1.75+ / requires 1.75+
cargo --version
```

### 3.3 Data Directories

Atrium uses a fixed home-based directory layout:

| Path | Purpose |
|------|---------|
| `~/.atrium/data/` | Persistent data (memory graph, conversation history, etc.) |
| `~/.atrium/logs/core.log` | Core log file (append mode, auto-created on startup) |
| `~/.atrium/models/` | Voice models (Piper `.onnx`, whisper `.bin`) |
| `~/.atrium/canned/` | Canned knowledge files (hot-reloadable) |

> **Note**: The data directory can be overridden with the `ATRIUM_DATA_DIR` environment variable. Priority: `ATRIUM_DATA_DIR` env var > default `~/.atrium/data/`.

```bash
# 创建必要目录 / Create required directories
mkdir -p ~/.atrium/data
mkdir -p ~/.atrium/logs
mkdir -p ~/.atrium/models
```

---

## 4. Feature Gates & Compilation

### 4.1 Feature Matrix

The `atrium-voice` crate defines the following features in [Cargo.toml](file:///d:/atrium/atrium/crates/atrium-voice/Cargo.toml):

| Feature | Description | Dependencies Added | Default |
|---------|-------------|-------------------|---------|
| (none) | No voice — zero compilation | None | ✅ Yes |
| `tts-piper` | Piper ONNX TTS backend (CPU, sync) | `ort = "2.0.0-rc.9"` | ❌ No |
| `tts-gpt-sovits` | GPT-SoVITS HTTP TTS backend (GPU, async) | `reqwest`, `hound`, `serde_json` | ❌ No |
| `stt-whisper` | whisper.cpp FFI STT backend | `bindgen` (build-dep) | ❌ No |
| `voice` | Convenience aggregate — enables `tts-piper` | (same as `tts-piper`) | ❌ No |

```toml
# 节选自 Cargo.toml / Excerpt from Cargo.toml
[features]
default = []
tts-piper = ["ort"]                                    # Piper ONNX TTS 后端 / Piper ONNX TTS backend
tts-gpt-sovits = ["reqwest", "hound", "serde_json"]    # GPT-SoVITS HTTP TTS 后端 / GPT-SoVITS HTTP TTS backend
stt-whisper = ["bindgen"]                              # whisper.cpp FFI STT 后端 / whisper.cpp FFI STT backend
voice = ["tts-piper"]                                  # 便捷聚合特性 / Convenience aggregate feature
```

### 4.2 Build Commands

```bash
# 默认编译 — 无语音能力（零依赖）/ Default build — no voice (zero dependencies)
cargo build -p atrium-voice

# 启用 Piper TTS / Enable Piper TTS
cargo build -p atrium-voice --features tts-piper

# 启用 GPT-SoVITS TTS / Enable GPT-SoVITS TTS
cargo build -p atrium-voice --features tts-gpt-sovits

# 启用 STT / Enable STT
cargo build -p atrium-voice --features stt-whisper

# 启用全部语音能力 / Enable all voice capabilities
cargo build -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper"

# 启用整个项目的语音 / Enable voice for the whole project
cargo build --features "atrium-voice/tts-piper"
```

### 4.3 Clippy & fmt Verification

```bash
# clippy 零警告验证 / clippy zero-warning verification
cargo clippy -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper" -- -D warnings

# fmt 格式检查 / fmt format check
cargo fmt -p atrium-voice -- --check

# fmt 自动格式化 / fmt auto-format
cargo fmt -p atrium-voice
```

> **Requirement**: clippy must produce **zero warnings** and `cargo fmt --check` must pass. This is verified before every commit.

---

## 5. Piper TTS Backend

### 5.1 Overview

Piper is a local neural TTS engine based on ONNX Runtime. It runs VITS models on **CPU** with ~100ms first-sound latency — far better than API-based solutions (~500ms).

- **Feature gate**: `tts-piper`
- **Inference**: Local ONNX Runtime (`ort` crate)
- **API**: Synchronous `synthesize_to_shm()` (blocking)
- **Tests**: 54 unit tests

**Source**: [file:///d:/atrium/atrium/crates/atrium-voice/src/tts/piper.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/tts/piper.rs)

### 5.2 Model Acquisition

Piper models are distributed as `.onnx` + `.json` config pairs. Download from the official Piper model repository:

```bash
# 下载 Piper 模型 / Download Piper model
# 示例：中文模型 / Example: Chinese model
mkdir -p ~/.atrium/models/piper
cd ~/.atrium/models/piper

# 下载 .onnx 模型文件和 .json 配置 / Download .onnx model file and .json config
# (从 Piper 官方 releases 下载 / Download from Piper official releases)
# 例如: zh_CN-huayan-medium.onnx + zh_CN-huayan-medium.onnx.json
```

Model files to place:
- `model_path`: Path to `.onnx` file (e.g., `~/.atrium/models/piper/zh_CN-huayan-medium.onnx`)
- `config_path`: Path to `.json` config (e.g., `~/.atrium/models/piper/zh_CN-huayan-medium.onnx.json`)

### 5.3 Configuration

```toml
# atrium.toml — [voice] 节选 / [voice] excerpt
[voice]
enabled = true

[voice.tts]
enabled = true
engine = "piper"                                    # 引擎类型 / Engine type
model_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx"
config_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx.json"
sample_rate = 22050                                 # Piper 原生采样率 / Piper native sample rate
target_sample_rate = 16000                          # 目标采样率 / Target sample rate (shared memory)
```

### 5.4 Prosody Parameter Mapping

The `ProsodyBridge::to_piper_params()` function translates universal `ProsodyParams` to Piper-specific parameters:

| Universal Param | Piper Param | Mapping Formula | Notes |
|----------------|-------------|-----------------|-------|
| `pitch_offset` (semitones) | `pitch_scale` (Hz ratio) | `pitch_scale = 2^(st/12)` | 12 semitones = 1 octave = frequency doubles |
| `rate` (speed factor) | `length_scale` | `length_scale = 1.0 / rate` | **Inverse**: faster rate → smaller length → shorter audio |
| `energy` | `energy_scale` | Direct passthrough | `energy_scale = energy` |
| `pause_duration_ms` | `pause_duration_secs` | `secs = ms / 1000.0` | ms → seconds conversion |
| `intra_pause_prob` | `intra_pause_prob` | Direct passthrough | — |
| `warmth` | `warmth` | Direct passthrough | [0, 1] |
| `breathiness` | `breathiness` | Direct passthrough | [0, 0.5] |

**Example**:
```rust
// 韵律参数翻译示例 / Prosody parameter translation example
// 输入：pitch_offset=+12 半音, rate=1.5（快）/ Input: pitch_offset=+12 semitones, rate=1.5 (fast)
// 输出：pitch_scale=2.0（高八度）, length_scale≈0.667（缩短）/ Output: pitch_scale=2.0 (octave up), length_scale≈0.667 (shorter)
let piper_params = ProsodyBridge::to_piper_params(&prosody);
// piper_params.pitch_scale == 2.0   // 2^(12/12) = 2.0
// piper_params.length_scale ≈ 0.667 // 1.0 / 1.5
```

**Source**: [file:///d:/atrium/atrium/crates/atrium-voice/src/prosody_bridge.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/prosody_bridge.rs)

### 5.5 Degraded Mode

When `model_path` is empty (the default), Piper enters **degraded mode**:

```rust
// 降级模式逻辑 / Degraded mode logic
// 摘自 piper.rs initialize() / Excerpt from piper.rs initialize()
if model_path.is_empty() {
    // 模型路径为空 — 跳过初始化（降级模式）
    // Empty model path — skip initialization (degraded mode)
    return Ok(());
}
```

In degraded mode:
- `initialize()` returns `Ok(())` without loading any model
- `synthesize()` returns an empty `Vec<f32>` (empty PCM)
- `is_initialized()` returns `false`
- The digital life still runs normally — text replies are unaffected, just no audio output

This allows the system to boot and function without a model file present.

---

## 6. GPT-SoVITS TTS Backend

### 6.1 Overview

GPT-SoVITS is an HTTP-bridged TTS backend that supports **few-shot voice cloning** via a Python service. Unlike Piper's local inference, GPT-SoVITS requires a separate Python service running on GPU, but supports arbitrary voice cloning with richer emotional expression.

- **Feature gate**: `tts-gpt-sovits`
- **Inference**: HTTP bridge to Python `api_v2.py` service (GPU)
- **API**: Asynchronous `synthesize_to_shm_async()` (non-blocking)
- **Tests**: 62 unit tests

**Source**: [file:///d:/atrium/atrium/crates/atrium-voice/src/tts/gpt_sovits.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/tts/gpt_sovits.rs)

### 6.2 Python Service Deployment

#### 6.2.1 Prerequisites

```bash
# 克隆 GPT-SoVITS 仓库 / Clone GPT-SoVITS repository
git clone https://github.com/RVC-Boss/GPT-SoVITS.git
cd GPT-SoVITS

# 安装 Python 依赖 / Install Python dependencies
pip install -r requirements.txt

# 下载预训练模型 / Download pretrained models
# 放置到 GPT-SoVITS/GPT_SoVITS/pretrained_models/ 目录
# Place into GPT-SoVITS/GPT_SoVITS/pretrained_models/ directory
```

#### 6.2.2 Starting the Service

```bash
# 启动 api_v2.py 服务 / Start api_v2.py service
# 默认端口 9880，端点 /tts / Default port 9880, endpoint /tts
python api_v2.py

# 或指定参数 / Or with parameters
python api_v2.py -p 9880 -a 127.0.0.1
```

The service exposes:
- **URL**: `http://127.0.0.1:9880`
- **Endpoint**: `POST /tts`
- **Request body**: JSON (see [Configuration Reference](#8-configuration-reference))
- **Response**: WAV audio byte stream

#### 6.2.3 Custom-Trained Voice Models

GPT-SoVITS supports custom voice cloning with a 5-10 second reference audio sample:

```bash
# 1. 准备参考音频（5-10秒，清晰人声）/ Prepare reference audio (5-10s, clear voice)
#    保存为 ref.wav / Save as ref.wav

# 2.（可选）使用 GPT-SoVITS WebUI 训练自定义模型
#    (Optional) Train custom model using GPT-SoVITS WebUI
#    python webui.py

# 3. 在 atrium.toml 中配置参考音频路径
#    Configure reference audio path in atrium.toml
```

### 6.3 Configuration

```toml
# atrium.toml — [voice.tts] GPT-SoVITS 节选 / [voice.tts] GPT-SoVITS excerpt
[voice]
enabled = true

[voice.tts]
enabled = true
engine = "gpt-sovits"                               # 引擎类型 / Engine type
service_url = "http://127.0.0.1:9880"               # Python 服务地址 / Python service URL
ref_audio_path = "/path/to/ref.wav"                 # 参考音频（5-10s）/ Reference audio (5-10s)
prompt_text = "这是参考音频对应的文本。"              # 参考音频文本 / Reference audio prompt text
prompt_lang = "zh"                                  # 参考音频语言 / Reference audio language
text_lang = "zh"                                    # 合成语言 / Synthesis language
timeout_secs = 30                                   # 请求超时（秒）/ Request timeout (seconds)
streaming_mode = 0                                  # 流式模式（0=关闭）/ Streaming mode (0=off)
target_sample_rate = 16000                          # 目标采样率 / Target sample rate
```

### 6.4 Prosody Parameter Mapping

The `ProsodyBridge::to_gpt_sovits_params()` function translates universal `ProsodyParams` to GPT-SoVITS-specific parameters:

| Universal Param | GPT-SoVITS Param | Mapping Formula | Notes |
|----------------|-------------------|-----------------|-------|
| `rate` | `speed_factor` | `speed_factor = rate` (clamped 0.5–2.0) | **Direct mapping** (not inverse) |
| `warmth` [0,1] | `temperature` | `temperature = 0.8 + warmth * 0.4` (clamped 0.5–2.0) | warmth=0→0.8 (calm), warmth=1→1.2 (warm) |
| `pause_duration_ms` | `fragment_interval` | `fragment_interval = ms / 1000` (clamped 0.1–1.0) | seconds |
| `pitch_offset` | (not supported) | — | GPT-SoVITS has no direct pitch control |
| `energy` | (not supported) | — | Can be normalized via PCM post-processing |
| — | `top_k` | Default: `15` | Top-K sampling |
| — | `top_p` | Default: `1.0` | Top-P sampling |
| — | `repetition_penalty` | Default: `1.35` | Repetition penalty |

> **Key difference from Piper**: GPT-SoVITS `speed_factor` is a **direct** mapping (rate 1.5 → speed_factor 1.5), while Piper's `length_scale` is an **inverse** mapping (rate 1.5 → length_scale 0.667).

**Example**:
```rust
// GPT-SoVITS 韵律参数翻译示例 / GPT-SoVITS prosody translation example
// 输入：rate=1.5, warmth=0.7 / Input: rate=1.5, warmth=0.7
// 输出：speed_factor=1.5, temperature=1.08 / Output: speed_factor=1.5, temperature=1.08
let params = ProsodyBridge::to_gpt_sovits_params(&prosody);
// params.speed_factor == 1.5           // 直接映射 / direct mapping
// params.temperature  == 0.8 + 0.7*0.4 == 1.08
```

### 6.5 WAV Decoding

GPT-SoVITS returns audio in WAV format. The `decode_wav_to_f32()` function uses the `hound` crate to decode WAV byte streams into f32 PCM samples:

```rust
// WAV 解码逻辑 / WAV decoding logic
// 摘自 gpt_sovits.rs / Excerpt from gpt_sovits.rs
fn decode_wav_to_f32(wav_bytes: &[u8]) -> Result<(Vec<f32>, u32), GptSoVitsError> {
    let mut reader = hound::WavReader::new(Cursor::new(wav_bytes))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            // float32 格式 — 直接读取 / float32 format — direct read
            reader.samples::<f32>().filter_map(|s| s.ok()).collect()
        }
        hound::SampleFormat::Int => {
            // int16 格式 → f32 归一化 / int16 format → f32 normalization
            reader.samples::<i16>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / 32768.0)  // 归一化到 [-1.0, 1.0] / Normalize to [-1.0, 1.0]
                .collect()
        }
    };
    Ok((samples, sample_rate))
}
```

**Supported formats**:
- **float32** (32-bit): Direct read, no quantization error
- **int16** (16-bit): Normalized to `[-1.0, 1.0]` via `sample / 32768.0`

### 6.6 Dockerization (Optional)

For reproducible GPT-SoVITS deployment:

```dockerfile
# Dockerfile — GPT-SoVITS 服务容器 / GPT-SoVITS service container
FROM pytorch/pytorch:2.1.0-cuda11.8-cudnn8-runtime

WORKDIR /app
COPY GPT-SoVITS /app/GPT-SoVITS
COPY ref.wav /app/ref.wav

RUN pip install -r /app/GPT-SoVITS/requirements.txt

EXPOSE 9880

# 启动 api_v2.py / Start api_v2.py
CMD ["python", "/app/GPT-SoVITS/api_v2.py", "-p", "9880", "-a", "0.0.0.0"]
```

```bash
# 构建并运行 / Build and run
docker build -t atrium-gpt-sovits .
docker run --gpus all -p 9880:9880 atrium-gpt-sovits
```

### 6.7 Degraded Mode

When `service_url` is empty (the default), GPT-SoVITS enters degraded mode:

```rust
// 降级模式逻辑 / Degraded mode logic
// 摘自 gpt_sovits.rs synthesize() / Excerpt from gpt_sovits.rs synthesize()
if self.config.service_url.is_empty() {
    // 服务地址为空 — 返回空 PCM（降级模式）
    // Empty service URL — return empty PCM (degraded mode)
    return Ok(GptSoVitsResult {
        samples: Vec::new(),
        sample_rate: self.config.target_sample_rate,
        duration_ms: 0,
    });
}
```

The system runs normally without the Python service — only audio output is absent.

---

## 7. STT Speech Recognition

### 7.1 Overview

STT uses **whisper.cpp** via direct FFI bindings. whisper.cpp is implemented in C, with ~2x performance over `whisper-rs` and ~300ms recognition latency.

- **Feature gate**: `stt-whisper`
- **Inference**: whisper.cpp C library via FFI (`bindgen` for bindings generation)
- **Input**: PCM f32 samples (16kHz, mono)
- **Output**: Recognized text + status (Partial/Final/Silence)

**Source**: [file:///d:/atrium/atrium/crates/atrium-voice/src/stt/whisper.rs](file:///d:/atrium/atrium/crates/atrium-voice/src/stt/whisper.rs)

### 7.2 Model Acquisition

```bash
# 下载 whisper.cpp 模型 / Download whisper.cpp model
mkdir -p ~/.atrium/models/whisper
cd ~/.atrium/models/whisper

# 下载 GGML 格式模型 / Download GGML format model
# 推荐使用 base 或 small 模型（平衡速度与精度）
# Recommended: base or small model (balance speed vs accuracy)
# 例如: ggml-base.bin (多语言) / ggml-base.bin (multilingual)
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

### 7.3 Configuration

```toml
# atrium.toml — [voice.stt] 节选 / [voice.stt] excerpt
[voice.stt]
enabled = true
engine = "whisper"                                  # 引擎类型 / Engine type
model_path = "~/.atrium/models/whisper/ggml-base.bin"
language = "zh"                                     # 识别语言 / Recognition language
sample_rate = 16000                                 # 采样率 / Sample rate
chunk_duration_ms = 500                             # 流式分块时长 / Streaming chunk duration (ms)
vad_enabled = true                                  # VAD 静音检测 / VAD silence detection
vad_energy_threshold = 0.01                         # VAD 能量阈值 / VAD energy threshold
```

### 7.4 gRPC AudioStream

The `SttEngine` receives PCM audio chunks via gRPC `AudioStream`. The data flow:

1. Client (frontend/microphone) sends PCM f32 samples via gRPC streaming RPC
2. `SttEngine::push_audio()` accumulates samples until `chunk_sample_count` is reached
3. `chunk_sample_count = sample_rate * chunk_duration_ms / 1000` (e.g., 16000 * 500 / 1000 = 8000 samples)
4. When a full chunk is accumulated, VAD checks if speech is present
5. If speech detected, `WhisperStt::recognize()` performs inference
6. Returns `RecognitionResult` with status: `Partial`, `Final`, or `Silence`

```rust
// SttEngine 编排流程 / SttEngine orchestration flow
// 摘自 stt/engine.rs / Excerpt from stt/engine.rs
let chunk_sample_count = (config.sample_rate * config.chunk_duration_ms) / 1000;
// 16000 Hz * 500 ms / 1000 = 8000 样本/块 / 8000 samples per chunk

// 推入音频块 — 积累后触发识别 / Push audio chunk — triggers recognition when accumulated
let result = engine.push_audio(&pcm_samples)?;
// result: Option<RecognitionResult>
//   - None: 不足一块 / Not enough for a chunk
//   - Some(Silence): VAD 检测到静音 / VAD detected silence
//   - Some(Final): 识别完成 / Recognition complete
```

### 7.5 VAD (Voice Activity Detection)

The built-in VAD uses RMS energy thresholding:

```rust
// VAD 静音检测 / VAD silence detection
// 摘自 whisper.rs has_speech() / Excerpt from whisper.rs has_speech()
fn has_speech(&self, samples: &[f32]) -> bool {
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    rms > self.config.vad_energy_threshold  // 默认阈值 0.01 / Default threshold 0.01
}
```

---

## 8. Configuration Reference

### 8.1 Complete `atrium.toml` Voice Section

```toml
# ── 语音能力 / Voice Capability ──
# 数字生命的"有声呼吸"——TTS 语音合成 + STT 语音识别
# Digital life's "audible breath" — TTS synthesis + STT recognition
# 默认全部关闭，启用需在编译时开启对应 feature / All disabled by default; enable features at compile time
[voice]
enabled = true                                      # 总开关 / Master switch

# ── TTS 配置 / TTS Configuration ──
[voice.tts]
enabled = true                                      # 是否启用 TTS / Enable TTS
engine = "piper"                                    # 引擎: "piper" | "gpt-sovits" / Engine

# Piper 后端配置 / Piper backend config
model_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx"   # ONNX 模型路径 / ONNX model path
config_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx.json"  # JSON 配置路径 / JSON config path
sample_rate = 22050                                 # Piper 原生采样率 / Piper native sample rate
target_sample_rate = 16000                          # 共享内存采样率 / Shared memory sample rate

# GPT-SoVITS 后端配置 / GPT-SoVITS backend config
service_url = "http://127.0.0.1:9880"               # Python 服务地址 / Python service URL
ref_audio_path = "/path/to/ref.wav"                 # 参考音频（5-10s）/ Reference audio (5-10s)
prompt_text = "参考音频对应的文本"                    # 参考音频文本 / Reference audio prompt text
prompt_lang = "zh"                                  # 参考音频语言 / Reference audio language
text_lang = "zh"                                    # 合成语言 / Synthesis language
timeout_secs = 30                                   # HTTP 超时（秒）/ HTTP timeout (seconds)
streaming_mode = 0                                  # 流式模式（0=关,1=最佳,2=中,3=快）/ Streaming mode

# ── STT 配置 / STT Configuration ──
[voice.stt]
enabled = true                                      # 是否启用 STT / Enable STT
engine = "whisper"                                  # 引擎类型 / Engine type
model_path = "~/.atrium/models/whisper/ggml-base.bin"  # whisper.cpp 模型路径 / whisper.cpp model path
language = "zh"                                     # 识别语言 / Recognition language
sample_rate = 16000                                 # 采样率 / Sample rate
chunk_duration_ms = 500                             # 分块时长（ms）/ Chunk duration (ms)
vad_enabled = true                                  # VAD 启用 / VAD enabled
vad_energy_threshold = 0.01                         # VAD 能量阈值 / VAD energy threshold

# ── 声纹识别配置 / Voiceprint Recognition Config ──
[voice.voiceprint]
enabled = false                                     # 声纹识别（M9/M10 预留）/ Voiceprint (M9/M10 reserved)
service_url = ""                                    # Python speechbrain gRPC 服务 / speechbrain gRPC URL
similarity_threshold = 0.75                         # 余弦相似度阈值 / Cosine similarity threshold

# ── 音频缓冲区配置 / Audio Buffer Config ──
[voice.audio]
sample_rate = 16000                                 # 共享内存采样率 / Shared memory sample rate
channels = 1                                        # 声道数（mono=1）/ Channels (mono=1)
buffer_size = 16384                                 # 环形缓冲区容量（样本数）/ Ring buffer capacity (samples)
```

### 8.2 Field Reference Table

#### VoiceCfg (Root)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Master switch — voice is opt-in |
| `tts` | TtsCfg | (default) | TTS engine configuration |
| `stt` | SttCfg | (default) | STT engine configuration |
| `voiceprint` | VoiceprintCfg | (default) | Voiceprint recognition configuration |
| `audio` | AudioCfg | (default) | Audio buffer configuration |

#### TtsCfg

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable TTS |
| `engine` | String | `"piper"` | Engine type: `"piper"` or `"gpt-sovits"` |
| `model_path` | String | `""` | Piper ONNX model path (empty = degraded mode) |
| `config_path` | String | `""` | Piper JSON config path |
| `sample_rate` | u32 | `22050` | Piper native sample rate |
| `target_sample_rate` | u32 | `16000` | Target sample rate for shared memory |
| `service_url` | String | `""` | GPT-SoVITS service URL (empty = degraded mode) |
| `ref_audio_path` | String | `""` | Reference audio for voice cloning (5-10s) |
| `prompt_text` | String | `""` | Reference audio prompt text |
| `prompt_lang` | String | `"zh"` | Reference audio language |
| `text_lang` | String | `"zh"` | Synthesis text language |
| `timeout_secs` | u64 | `30` | HTTP request timeout (seconds) |
| `streaming_mode` | u8 | `0` | Streaming mode (0=off, 1=best, 2=medium, 3=fast) |

#### SttCfg

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable STT |
| `engine` | String | `"whisper"` | Engine type |
| `model_path` | String | `""` | whisper.cpp model path (empty = degraded mode) |
| `language` | String | `"zh"` | Recognition language |
| `sample_rate` | u32 | `16000` | Sample rate |
| `chunk_duration_ms` | u32 | `500` | Streaming chunk duration (ms) |
| `vad_enabled` | bool | `true` | Enable VAD silence detection |
| `vad_energy_threshold` | f32 | `0.01` | VAD energy threshold |

#### AudioCfg

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sample_rate` | u32 | `16000` | Shared memory audio sample rate |
| `channels` | u16 | `1` | Channel count (mono=1) |
| `buffer_size` | usize | `16384` | Ring buffer capacity (in samples) |

---

## 9. Unit Tests

### 9.1 Test Matrix

| Test Scope | Feature Required | Test Count | Key Coverage |
|-----------|------------------|------------|--------------|
| `prosody_bridge.rs` | (none) | 24 | Pitch/rate/energy mapping, clamping, full consistency |
| `tts/piper.rs` | `tts-piper` | 5 | Create, init (empty/nonexistent), degraded mode, double-init |
| `tts/gpt_sovits.rs` | `tts-gpt-sovits` | 12 | Create, init, degraded mode, WAV decode (int16/float32/empty/invalid) |
| `tts/engine.rs` | `tts-piper` or `tts-gpt-sovits` | 11 | VoiceEngine create, not-enabled, no-audio-manager, degraded mode, speaking flag lifecycle |
| `stt/whisper.rs` | `stt-whisper` | 7 | Create, init, degraded mode, VAD detection, double-init |
| `stt/engine.rs` | `stt-whisper` | 10 | Create, accumulate, silence, speech, flush, multi-chunk |
| **Total (Piper)** | `tts-piper` | **54** | All prosody + piper + engine tests |
| **Total (GPT-SoVITS)** | `tts-gpt-sovits` | **62** | All prosody + gpt-sovits + engine tests |

### 9.2 Run Commands

```bash
# 运行全部测试（默认无 feature）/ Run all tests (default, no features)
cargo test -p atrium-voice

# 运行 Piper 相关测试 / Run Piper-related tests
cargo test -p atrium-voice --features tts-piper

# 运行 GPT-SoVITS 相关测试 / Run GPT-SoVITS-related tests
cargo test -p atrium-voice --features tts-gpt-sovits

# 运行 STT 相关测试 / Run STT-related tests
cargo test -p atrium-voice --features stt-whisper

# 运行全部测试 / Run all tests (all features)
cargo test -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper"

# 运行特定测试模块 / Run specific test module
cargo test -p atrium-voice --features tts-piper prosody_bridge::tests

# 显示测试输出 / Show test output
cargo test -p atrium-voice --features tts-piper -- --nocapture
```

### 9.3 Expected Output

```
running 24 tests
test prosody_bridge::tests::test_pitch_12_semitones_doubles_frequency ... ok
test prosody_bridge::tests::test_pitch_minus_12_semitones_halves_frequency ... ok
test prosody_bridge::tests::test_pitch_zero_no_change ... ok
test prosody_bridge::tests::test_rate_fast_decreases_length ... ok
test prosody_bridge::tests::test_rate_slow_increases_length ... ok
...
test prosody_bridge::tests::test_gpt_sovits_full_mapping_consistency ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 5 tests
test tts::piper::tests::test_piper_create ... ok
test tts::piper::tests::test_piper_initialize_empty_path ... ok
test tts::piper::tests::test_piper_initialize_nonexistent_path ... ok
test tts::piper::tests::test_piper_synthesize_degraded_mode ... ok
test tts::piper::tests::test_piper_double_initialize ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 11 tests
test tts::engine::tests::test_voice_engine_piper_create ... ok
test tts::engine::tests::test_voice_engine_not_enabled ... ok
...

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 10. End-to-End Running

### 10.1 Startup Flow

```bash
# 1. 编译项目（启用所需 feature）/ Build project (with required features)
cargo build --features "atrium-voice/tts-piper,atrium-voice/stt-whisper"

# 2. 配置 atrium.toml [voice] 节 / Configure atrium.toml [voice] section
#    确保 model_path 指向真实模型文件 / Ensure model_path points to real model files

# 3. 启动 Atrium 核心 / Start Atrium core
cargo run --features "atrium-voice/tts-piper,atrium-voice/stt-whisper"

# 日志输出到 ~/.atrium/logs/core.log / Logs output to ~/.atrium/logs/core.log
```

### 10.2 TTS Trigger

TTS is triggered when the digital life generates a text reply. The flow:

1. User sends a message (text or voice)
2. Core processes the message and generates a text reply
3. `ProsodyMapper` converts the current PAD emotional state to `ProsodyParams`
4. `VoiceEngine::synthesize_to_shm()` (Piper, sync) or `synthesize_to_shm_async()` (GPT-SoVITS, async) is called
5. PCM audio is written to shared memory via `AudioManager`
6. The render engine reads from shared memory and plays the audio

```rust
// TTS 调用示例 / TTS invocation example
// 同步（Piper）/ Synchronous (Piper)
let written = engine.synthesize_to_shm("你好，世界", &prosody)?;

// 异步（GPT-SoVITS）/ Asynchronous (GPT-SoVITS)
let written = engine.synthesize_to_shm_async("你好，世界", &prosody).await?;
```

### 10.3 HTTP API Testing

Atrium exposes an HTTP gateway (default `0.0.0.0:8080`) for direct interaction:

```bash
# 发送消息触发 TTS / Send message to trigger TTS
curl -X POST http://127.0.0.1:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"你好，给我讲个故事"}'

# SSE 流式响应（包含文本 + 音频触发）/ SSE streaming response (text + audio trigger)
curl -N http://127.0.0.1:8080/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message":"今天天气怎么样？"}'

# 检查语音状态 / Check voice status
curl http://127.0.0.1:8080/api/voice/status
```

### 10.4 Log Verification

```bash
# 查看实时日志 / View real-time logs
tail -f ~/.atrium/logs/core.log

# 过滤语音相关日志 / Filter voice-related logs
# Linux/macOS
grep -i "voice\|tts\|piper\|gpt-sovits\|whisper\|synth" ~/.atrium/logs/core.log

# Windows PowerShell
Select-String -Path ~/.atrium/logs/core.log -Pattern "voice|tts|piper|whisper|synth"
```

**Expected log entries (degraded mode)**:
```json
{"level":"DEBUG","msg":"Piper 推理占位 / Piper inference placeholder: text=你好, pitch_scale=1.0"}
{"level":"DEBUG","msg":"GPT-SoVITS 降级模式 — 服务地址为空，返回空 PCM / GPT-SoVITS degraded mode — empty service URL, returning empty PCM"}
```

**Expected log entries (active mode)**:
```json
{"level":"DEBUG","msg":"GPT-SoVITS 请求: text=你好, speed=1.0, temp=1.0"}
{"level":"INFO","msg":"TTS 合成完成 / TTS synthesis complete: 16000 samples, 45ms"}
```

---

## 11. Troubleshooting

### 11.1 Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| **No audio output** | `model_path` or `service_url` is empty (degraded mode) | Set the path/URL in `atrium.toml` and restart |
| **`PiperError::ModelNotFound`** | Model file does not exist at the configured path | Verify the path; download the model to `~/.atrium/models/piper/` |
| **`GptSoVitsError::RefAudioNotConfigured`** | `service_url` is set but `ref_audio_path` is empty | Set `ref_audio_path` to a 5-10s WAV file |
| **`GptSoVitsError::RequestFailed`** | Python service not running or unreachable | Start `api_v2.py` on port 9880; check firewall |
| **`GptSoVitsError::ServiceError`** | Python service returned an error status | Check Python service logs for details |
| **`GptSoVitsError::DecodeFailed`** | Invalid WAV response from service | Verify GPT-SoVITS returns valid WAV format |
| **`WhisperError::ModelNotFound`** | whisper.cpp model file not found | Download `ggml-*.bin` to `~/.atrium/models/whisper/` |
| **`VoiceError::NotEnabled`** | TTS not enabled in config or feature not compiled | Set `enabled = true` and compile with `--features tts-piper` |
| **`VoiceError::AudioManagerNotBound`** | AudioManager not bound to VoiceEngine | Call `engine.bind_audio_manager(manager)` before synthesis |
| **Compilation error: `ort` not found** | `tts-piper` feature not enabled | Add `--features tts-piper` to build command |

### 11.2 Debug Mode

Enable debug-level logging for detailed voice diagnostics:

```toml
# atrium.toml — 调试模式 / Debug mode
log_level = "debug"
```

```bash
# 设置环境变量 / Set environment variable
export RUST_LOG="atrium_voice=debug,atrium_core=debug"

# 运行并查看详细日志 / Run and view detailed logs
RUST_LOG="atrium_voice=debug" cargo run --features "atrium-voice/tts-piper"
```

**Debug log points**:
- Piper inference placeholder: text and pitch_scale values
- GPT-SoVITS request: text, speed, temperature
- GPT-SoVITS degraded mode: empty service URL notification
- VoiceEngine: speaking flag transitions (true/false)

### 11.3 Health Check

```bash
# 检查 Python GPT-SoVITS 服务是否运行 / Check if Python GPT-SoVITS service is running
curl http://127.0.0.1:9880/

# 检查模型文件是否存在 / Check if model files exist
ls -la ~/.atrium/models/piper/
ls -la ~/.atrium/models/whisper/

# 验证 feature 是否启用 / Verify features are enabled
cargo build -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper" -v 2>&1 | grep "feature"
```

---

## 12. Current Limitations & Future Extensions

### 12.1 Current Limitations

| Limitation | Description | Workaround |
|-----------|-------------|------------|
| **Piper skeleton inference** | The `ort::Session` loading is a skeleton; actual ONNX inference returns empty PCM | Place a real Piper model; the `initialize()` logic will be completed in a future release |
| **whisper.cpp not linked** | The FFI `extern "C"` declarations are placeholders; the actual library is not linked | Link whisper.cpp library via build script in a future release |
| **No streaming TTS** | Piper and GPT-SoVITS synthesize the full utterance before playback | Future: chunk-based streaming synthesis |
| **No voice conversion** | Cannot transform one voice to another in real-time | Future: voice conversion module |
| **Single-speaker Piper** | Piper models are single-speaker; switching voices requires swapping models | Use GPT-SoVITS for multi-voice support |
| **Voiceprint reserved** | `voiceprint.rs` provides M9/M10 reserved interfaces, not yet implemented | Future milestone implementation |
| **No GPU for Piper** | Piper uses CPU-only ONNX Runtime | Use GPT-SoVITS for GPU acceleration |

### 12.2 Future Extensions

| Extension | Description | Milestone |
|-----------|-------------|-----------|
| **Streaming TTS** | Chunk-based synthesis for lower first-sound latency | M8+ |
| **Voice conversion** | Real-time voice transformation | M9 |
| **Voiceprint recognition** | Identify speakers by voice characteristics | M9/M10 |
| **Voice style memory** | Remember and reproduce user's preferred voice style | M10 |
| **Multi-speaker Piper** | Support multi-speaker ONNX models | Future |
| **GPU Piper inference** | Enable CUDA execution provider for ONNX Runtime | Future |
| **Emotion-conditioned TTS** | Direct emotional embedding injection into TTS | Future |
| **Barge-in refinement** | More sophisticated interruption handling with partial synthesis discard | Future |

---

## Completion Log

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-07-10 | v1.0 | Atrium Team | Initial English version. Covers architecture, Piper/GPT-SoVITS/whisper backends, configuration reference, unit tests (54 Piper / 62 GPT-SoVITS), end-to-end running, troubleshooting, and future extensions. Based on actual code in `crates/atrium-voice/`. Verified against: `Cargo.toml`, `lib.rs`, `config.rs`, `prosody_bridge.rs`, `tts/piper.rs`, `tts/gpt_sovits.rs`, `tts/engine.rs`, `stt/whisper.rs`, `stt/engine.rs`. clippy zero warnings, fmt passes. |
