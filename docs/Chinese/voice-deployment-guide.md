# Atrium 语音能力部署指南（TTS/STT）

> 版本：v1.0
> 日期：2026-07-10
> 适用 crate：`atrium-voice`（路径：`crates/atrium-voice/`）

---

## 目录

1. [概述](#1-概述)
2. [架构与数据流](#2-架构与数据流)
3. [环境准备](#3-环境准备)
4. [Feature 开关与编译](#4-feature-开关与编译)
5. [Piper TTS 后端](#5-piper-tts-后端)
6. [GPT-SoVITS TTS 后端](#6-gpt-sovits-tts-后端)
7. [STT 语音识别](#7-stt-语音识别)
8. [配置详解](#8-配置详解)
9. [单元测试](#9-单元测试)
10. [端到端运行](#10-端到端运行)
11. [故障排查](#11-故障排查)
12. [当前限制与未来扩展](#12-当前限制与未来扩展)

---

## 1. 概述

### 1.1 设计理念

文本是思维的符号，声音是生命的呼吸。

Atrium 是一个 Rust 数字生命框架，`atrium-voice` crate 实现数字生命的语音输入（STT）与输出（TTS）能力，让数字生命从"文本存在"升级为"有声存在"。

核心理念——**数字生命工程**：

- **零侵入**：Feature gate 默认不启用，禁用时对系统零影响
- **可降级**：空模型路径 / 空服务地址 → 返回空 PCM，系统正常运行
- **多后端**：TTS 支持 Piper（本地 ONNX）与 GPT-SoVITS（HTTP 声音克隆），STT 支持 whisper.cpp
- **韵律桥接**：通用韵律参数（`ProsodyParams`）经 `ProsodyBridge` 翻译为各引擎专用参数

### 1.2 数字生命工程定位

| 维度 | 文本存在 | 有声存在 |
|------|----------|----------|
| 输出 | 纯文本渲染 | PCM 音频 + 口型同步 |
| 输入 | 键盘 / 文本 | 麦克风语音识别 |
| 身份 | 无音色 | 可定制音色（声音克隆） |
| 情感 | 语义暗示 | 韵律参数（音高 / 语速 / 温暖度） |

相关源码：`file:///d:/atrium/atrium/crates/atrium-voice/src/lib.rs`

---

## 2. 架构与数据流

### 2.1 模块结构

```
atrium-voice/
├── src/
│   ├── lib.rs              # 模块入口与重导出 / Module entry & re-exports
│   ├── config.rs           # VoiceCfg / TtsCfg / SttCfg / VoiceprintCfg / AudioCfg
│   ├── prosody_bridge.rs   # 韵律桥接器 / Prosody Bridge (universal → engine-specific)
│   ├── audio_buffer.rs     # 无锁 SPSC 环形缓冲区 / Lock-free SPSC ring buffer
│   ├── tts/
│   │   ├── mod.rs          # TTS 子模块入口 / TTS submodule entry
│   │   ├── engine.rs       # VoiceEngine — 统一 TTS 接口 / Unified TTS interface
│   │   ├── piper.rs        # Piper ONNX 后端 / Piper ONNX backend
│   │   └── gpt_sovits.rs   # GPT-SoVITS HTTP 后端 / GPT-SoVITS HTTP backend
│   ├── stt/
│   │   ├── mod.rs          # STT 子模块入口 / STT submodule entry
│   │   ├── engine.rs       # SttEngine — 统一 STT 接口 / Unified STT interface
│   │   └── whisper.rs      # whisper.cpp FFI 后端 / whisper.cpp FFI backend
│   └── voiceprint.rs       # 声纹识别预留接口 / Voiceprint reserved interfaces
└── Cargo.toml              # Feature 矩阵与依赖声明 / Feature matrix & dependencies
```

### 2.2 合成数据流图

```
                  ProsodyParams (通用韵律意图)
                          │
                          ▼
                   ┌─────────────┐
                   │ ProsodyBridge│  意图 → 执行翻译层
                   └──────┬──────┘
              ┌───────────┴───────────┐
              ▼                       ▼
    ┌──────────────────┐    ┌─────────────────────┐
    │ PiperSynthesisParams│    │ GptSoVitsSynthesisParams│
    │ pitch_scale        │    │ speed_factor         │
    │ length_scale       │    │ temperature          │
    └────────┬───────────┘    └──────────┬──────────┘
             ▼                           ▼
    ┌──────────────────┐    ┌─────────────────────┐
    │   PiperTts       │    │  GptSoVitsClient    │
    │  (本地 ONNX/CPU) │    │  (HTTP → Python)    │
    │ synthesize()     │    │ synthesize() async  │
    └────────┬─────────┘    └──────────┬──────────┘
             ▼                           ▼
         f32 PCM                    WAV bytes → hound 解码 → f32 PCM
             │                           │
             ▼                           ▼
    ┌──────────────────────────────────────────┐
    │       synthesize_to_shm / _async         │
    │       (VoiceEngine → AudioManager)       │
    │       写入共享内存环形缓冲区              │
    └──────────────────────────────────────────┘
                          │
                          ▼
                 渲染引擎读取播放
```

相关源码：
- 引擎入口：`file:///d:/atrium/atrium/crates/atrium-voice/src/tts/engine.rs`
- 韵律桥接：`file:///d:/atrium/atrium/crates/atrium-voice/src/prosody_bridge.rs`

---

## 3. 环境准备

### 3.1 系统要求

| 组件 | 要求 | 说明 |
|------|------|------|
| OS | Linux / macOS / Windows | 跨平台 Rust |
| Rust | 1.75+（edition 2021） | `atrium-voice` edition = 2021 |
| CPU | x86_64 / aarch64 | Piper 本地推理 |
| GPU（可选） | NVIDIA CUDA | GPT-SoVITS Python 服务 |
| Python（可选） | 3.10+ | GPT-SoVITS `api_v2.py` 服务 |

### 3.2 工具链

```bash
# 安装 Rust 工具链 / Install Rust toolchain
rustup default stable

# 验证版本 / Verify version
rustc --version    # 需 1.75+
cargo --version
```

### 3.3 数据目录

Atrium 运行时使用的目录结构（基于用户主目录）：

```
~/.atrium/
├── data/           # 运行时数据 / Runtime data
├── models/         # 模型文件 / Model files
│   ├── piper/      # Piper ONNX 模型
│   ├── gpt-sovits/ # GPT-SoVITS 自训练模型
│   └── whisper/    # whisper.cpp ggml 模型
└── logs/
    └── core.log    # 核心日志 / Core log
```

> Windows 下 `~` 对应 `%USERPROFILE%`（如 `C:\Users\<用户名>\.atrium`）。

---

## 4. Feature 开关与编译

### 4.1 Feature 矩阵

`atrium-voice` 的所有后端均为可选 feature，**默认不启用**（`default = []`），实现零侵入。

| Feature | 启用的后端 | 关键依赖 | 说明 |
|---------|-----------|----------|------|
| `tts-piper` | Piper ONNX TTS | `ort = "2.0.0-rc.9"` | 本地 CPU 推理，低延迟 |
| `tts-gpt-sovits` | GPT-SoVITS HTTP TTS | `reqwest`, `hound`, `serde_json` | GPU 声音克隆，异步 |
| `stt-whisper` | whisper.cpp STT | `bindgen`（build-dep） | FFI 绑定 |
| `voice` | 便捷聚合 = `tts-piper` | — | 启用 VoiceEngine + Piper |

相关源码：`file:///d:/atrium/atrium/crates/atrium-voice/Cargo.toml`

### 4.2 编译命令

```bash
# 不启用任何语音后端（默认）— 零影响 / No voice backends (default) — zero impact
cargo build -p atrium-voice

# 启用 Piper TTS / Enable Piper TTS
cargo build -p atrium-voice --features tts-piper

# 启用 GPT-SoVITS TTS / Enable GPT-SoVITS TTS
cargo build -p atrium-voice --features tts-gpt-sovits

# 启用 STT / Enable STT
cargo build -p atrium-voice --features stt-whisper

# 全量启用 / Enable all backends
cargo build -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper"

# 使用便捷聚合特性 / Use convenience aggregate feature
cargo build -p atrium-voice --features voice
```

### 4.3 clippy 验证

```bash
# clippy 零警告 / clippy zero warnings
cargo clippy -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper" -- -D warnings

# 代码格式检查 / Format check
cargo fmt -p atrium-voice --check
```

> 验证标准：clippy 零警告，fmt 通过。

---

## 5. Piper TTS 后端

### 5.1 后端特性

| 特性 | 值 |
|------|-----|
| 推理方式 | 本地 ONNX Runtime（CPU） |
| 调用接口 | 同步 `synthesize_to_shm()` |
| Feature | `tts-piper` |
| 测试数量 | 54 个 |
| 首字延迟 | 约 100ms（远优于 API 的 500ms） |
| 模型格式 | VITS `.onnx` + `.json` 配置 |

### 5.2 模型获取

Piper 模型来自 [rhasspy/piper](https://github.com/rhasspy/piper) 项目，下载后放置于 `~/.atrium/models/piper/`：

```
~/.atrium/models/piper/
├── zh_CN-huayan-medium.onnx      # 模型权重 / Model weights
└── zh_CN-huayan-medium.onnx.json # 模型配置 / Model config
```

### 5.3 配置示例

```toml
# atrium.toml
[voice]
enabled = true

[voice.tts]
enabled = true
engine = "piper"
model_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx"
config_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx.json"
sample_rate = 22050        # Piper 原生采样率 / Native sample rate
target_sample_rate = 16000 # 写入共享内存采样率 / Target sample rate
```

### 5.4 韵律参数映射

Piper 的参数模型基于 VITS，`ProsodyBridge::to_piper_params()` 负责翻译：

| 通用参数（ProsodyParams） | Piper 参数 | 映射公式 | 说明 |
|---------------------------|------------|----------|------|
| `pitch_offset`（半音） | `pitch_scale`（Hz 倍率） | `2^(st/12)` | 12 半音 = 1 八度 = 频率翻倍 |
| `rate`（语速因子） | `length_scale`（长度因子） | `1.0 / rate`（倒数） | 语速越快，长度越短 |
| `energy` | `energy_scale` | 直接传递 | — |
| `pause_duration_ms` | `pause_duration_secs` | `ms / 1000` | 毫秒转秒 |
| `warmth` | `warmth` | 直接传递 | — |
| `breathiness` | `breathiness` | 直接传递 | — |

```rust
// 中文 / English
// 半音 → Hz 倍率：12 半音 = 频率翻倍
// Semitones → Hz ratio: 12 semitones = frequency doubles
let pitch_scale = 2.0_f32.powf(prosody.pitch_offset / 12.0);

// 语速因子 → 长度因子（倒数）：语速越快长度越短
// Rate → length scale (inverse): faster rate = shorter length
let length_scale = 1.0 / prosody.rate;
```

相关源码：`file:///d:/atrium/atrium/crates/atrium-voice/src/tts/piper.rs`、`file:///d:/atrium/atrium/crates/atrium-voice/src/prosody_bridge.rs`

### 5.5 降级模式

当 `model_path` 为空字符串时，Piper 进入降级模式：

- `initialize()` 跳过模型加载，`initialized = false`
- `synthesize()` 返回空 PCM（`samples: vec![]`）
- 系统正常运行，不报错、不崩溃

```rust
// 降级模式：模型路径为空，返回空 PCM
// Degraded mode: empty model path, return empty PCM
if self.config.model_path.is_empty() {
    return Ok(SynthesisResult { samples: vec![], .. });
}
```

> 降级模式保证了"未配置模型时系统零影响"的零侵入承诺。

---

## 6. GPT-SoVITS TTS 后端

### 6.1 后端特性

| 特性 | 值 |
|------|-----|
| 推理方式 | HTTP 桥接 Python 服务（GPU） |
| 调用接口 | 异步 `synthesize_to_shm_async()` |
| Feature | `tts-gpt-sovits` |
| 测试数量 | 62 个 |
| 能力 | few-shot 声音克隆，任意音色 |
| WAV 解码 | `hound` 库，支持 float32 与 int16 |

> 注意：GPT-SoVITS 后端**必须使用异步接口** `synthesize_to_shm_async()`。若误用同步 `synthesize_to_shm()`，引擎会返回 `NotEnabled` 错误并打印警告日志。

### 6.2 Python 服务部署

GPT-SoVITS 使用 [RVC-Boss/GPT-SoVITS](https://github.com/RVC-Boss/GPT-SoVITS) 项目的 `api_v2.py`：

```bash
# 克隆 GPT-SoVITS 仓库 / Clone GPT-SoVITS repo
git clone https://github.com/RVC-Boss/GPT-SoVITS.git
cd GPT-SoVITS

# 安装依赖 / Install dependencies
pip install -r requirements.txt

# 启动 api_v2.py 服务（默认端口 9880）/ Start api_v2.py (default port 9880)
python api_v2.py --port 9880
```

服务启动后，监听 `http://127.0.0.1:9880`，提供 `/tts` 端点。

### 6.3 自训练模型

GPT-SoVITS 支持用户自训练音色模型，放置于：

```
~/.atrium/models/gpt-sovits/
├── GPT.pth              # GPT 模型权重 / GPT model weights
├── SoVITS.pth           # SoVITS 模型权重 / SoVITS model weights
└── ref_audio.wav        # 参考音频（5-10s 声音克隆样本）/ Reference audio (5-10s)
```

参考音频要求：
- 时长 5-10 秒
- 单声道，采样率 ≥ 16kHz
- 内容清晰、无背景噪音
- 配套 `prompt_text`（参考音频对应文本）可显著提升克隆质量

### 6.4 配置示例

```toml
# atrium.toml
[voice.tts]
enabled = true
engine = "gpt-sovits"
service_url = "http://127.0.0.1:9880"
ref_audio_path = "~/.atrium/models/gpt-sovits/ref_audio.wav"
prompt_text = "你好，我是你的数字生命。"  # 参考音频对应文本
prompt_lang = "zh"
text_lang = "zh"
timeout_secs = 30
streaming_mode = 0   # 0=关闭, 1=最佳质量, 2=中等, 3=快速
```

### 6.5 韵律参数映射

GPT-SoVITS 的参数模型与 Piper 不同：不支持直接音调调整，通过 `temperature` 间接影响情感。

`ProsodyBridge::to_gpt_sovits_params()` 映射规则：

| 通用参数 | GPT-SoVITS 参数 | 映射公式 | 说明 |
|----------|-----------------|----------|------|
| `rate` | `speed_factor` | `rate`（直接映射，clamp 0.5-2.0） | 语义一致，1.0=正常 |
| `warmth` | `temperature` | `0.8 + warmth * 0.4`（clamp 0.5-2.0） | warmth=0→0.8 冷静，warmth=1→1.2 热情 |
| `pause_duration_ms` | `fragment_interval` | `ms / 1000`（clamp 0.1-1.0） | 秒级片段间隔 |
| `pitch_offset` | — | 不支持 | 通过 temperature 间接影响 |
| `energy` | — | 不支持 | 可通过 PCM 后处理归一化 |

默认值（非韵律映射）：
- `top_k = 15`
- `top_p = 1.0`
- `repetition_penalty = 1.35`

```rust
// 语速直接映射（clamp 到 GPT-SoVITS 安全范围 0.5-2.0）
// Speed direct mapping (clamped to GPT-SoVITS safe range 0.5-2.0)
let speed_factor = prosody.rate.clamp(0.5, 2.0);

// 温暖度 → 采样温度：warmth=0 → 0.8（冷静），warmth=1 → 1.2（热情）
// Warmth → temperature: warmth=0 → 0.8 (calm), warmth=1 → 1.2 (warm)
let temperature = (0.8 + prosody.warmth * 0.4).clamp(0.5, 2.0);
```

相关源码：`file:///d:/atrium/atrium/crates/atrium-voice/src/tts/gpt_sovits.rs`

### 6.6 WAV 解码

GPT-SoVITS 服务返回 WAV 格式音频，使用 `hound` 库解码为 f32 PCM：

```rust
// WAV → f32 PCM 样本解码 / WAV → f32 PCM samples decoding
let (samples, sample_rate) = decode_wav_to_f32(&wav_bytes)?;
```

`decode_wav_to_f32()` 支持两种 WAV 样本格式：
- **float32**：直接读取为 f32
- **int16**：归一化到 `[-1.0, 1.0]`（除以 `32768.0`）

请求体通过 `serde_json` 构建，对应 `api_v2.py` 的 `TTS_Request`：

```json
{
  "text": "待合成文本",
  "text_lang": "zh",
  "ref_audio_path": "/path/to/ref.wav",
  "prompt_text": "参考音频文本",
  "prompt_lang": "zh",
  "top_k": 15,
  "top_p": 1.0,
  "temperature": 1.0,
  "speed_factor": 1.0,
  "fragment_interval": 0.5,
  "repetition_penalty": 1.35,
  "media_type": "wav",
  "streaming_mode": 0
}
```

### 6.7 降级模式

当 `service_url` 为空字符串时，GPT-SoVITS 进入降级模式：

- `initialize()` 跳过初始化
- `synthesize()` 返回空 PCM，打印 debug 日志
- 系统正常运行

```rust
// 降级模式：服务地址为空，返回空 PCM
// Degraded mode: empty service URL, returning empty PCM
tracing::debug!(
    "GPT-SoVITS 降级模式 — 服务地址为空，返回空 PCM / \
     GPT-SoVITS degraded mode — empty service URL, returning empty PCM"
);
```

### 6.8 Docker 化

GPT-SoVITS Python 服务可通过 Docker 部署（需 NVIDIA GPU + nvidia-docker）：

```dockerfile
# Dockerfile 示例 / Dockerfile example
FROM pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime

WORKDIR /app
COPY . /app
RUN pip install -r requirements.txt

EXPOSE 9880
CMD ["python", "api_v2.py", "--port", "9880"]
```

```bash
# 构建并运行 / Build and run
docker build -t gpt-sovits-api .
docker run --gpus all -p 9880:9880 \
  -v ~/.atrium/models/gpt-sovits:/models \
  gpt-sovits-api
```

---

## 7. STT 语音识别

### 7.1 后端特性

| 特性 | 值 |
|------|-----|
| 引擎 | whisper.cpp（FFI） |
| Feature | `stt-whisper` |
| 构建依赖 | `bindgen = "0.70"`（生成 FFI 绑定） |
| 模型格式 | ggml（`.bin`） |
| 采样率 | 16000 Hz |
| VAD | 内置能量阈值静音检测 |

### 7.2 模型获取

whisper.cpp 模型来自 [ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp)，放置于 `~/.atrium/models/whisper/`：

```
~/.atrium/models/whisper/
└── ggml-large-v3.bin    # 推荐大模型以获得最佳中文识别 / Recommended for best Chinese
```

常用模型：

| 模型 | 大小 | 说明 |
|------|------|------|
| `ggml-tiny.bin` | ~75MB | 快速测试 |
| `ggml-base.bin` | ~142MB | 基础使用 |
| `ggml-medium.bin` | ~1.5GB | 平衡质量 |
| `ggml-large-v3.bin` | ~3GB | 最佳质量（推荐） |

### 7.3 配置示例

```toml
# atrium.toml
[voice.stt]
enabled = true
engine = "whisper"
model_path = "~/.atrium/models/whisper/ggml-large-v3.bin"
language = "zh"
sample_rate = 16000
chunk_duration_ms = 500     # 流式分块时长 / Streaming chunk duration
vad_enabled = true          # 启用 VAD 静音检测 / Enable VAD
vad_energy_threshold = 0.01 # VAD 能量阈值 / VAD energy threshold
```

### 7.4 gRPC AudioStream

STT 引擎设计用于接收 gRPC `AudioStream` 流式音频：

- 音频以 `chunk_duration_ms`（默认 500ms）为单位分块输入
- VAD（Voice Activity Detection）基于能量阈值过滤静音片段
- 通过 VAD 的片段送入 whisper.cpp 识别
- 识别结果通过 `RecognitionResult` 返回，状态由 `RecognitionStatus` 标记

```rust
// 降级模式：无模型，即使 VAD 通过也返回空文本
// Degraded mode: no model, returns empty even if VAD passes
if !self.initialized {
    return String::new();
}
```

相关源码：
- 引擎：`file:///d:/atrium/atrium/crates/atrium-voice/src/stt/engine.rs`
- whisper：`file:///d:/atrium/atrium/crates/atrium-voice/src/stt/whisper.rs`

### 7.5 降级模式

当 `model_path` 为空时，STT 进入降级模式：

- 即使 VAD 检测到语音能量通过，仍返回空文本
- 系统正常运行，不报错

---

## 8. 配置详解

### 8.1 完整配置示例

以下为 `atrium.toml` 中 `[voice]` 段的完整配置示例：

```toml
# ════════════════════════════════════════════════════════════════════
# 语音能力配置 — 数字生命的有声呼吸 / Voice Capability Configuration
# ════════════════════════════════════════════════════════════════════

[voice]
# 总开关 — 默认关闭，语音能力为可选项 / Master switch — defaults to false
enabled = true

# ── TTS 配置 / TTS Configuration ──
[voice.tts]
enabled = true
# 引擎类型："piper" | "gpt-sovits" / Engine type
engine = "piper"

# Piper 专属 / Piper specific
model_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx"
config_path = "~/.atrium/models/piper/zh_CN-huayan-medium.onnx.json"
sample_rate = 22050
target_sample_rate = 16000

# GPT-SoVITS 专属 / GPT-SoVITS specific
service_url = "http://127.0.0.1:9880"
ref_audio_path = "~/.atrium/models/gpt-sovits/ref_audio.wav"
prompt_text = "你好，这是参考音频的文本。"
prompt_lang = "zh"
text_lang = "zh"
timeout_secs = 30
streaming_mode = 0

# ── STT 配置 / STT Configuration ──
[voice.stt]
enabled = true
engine = "whisper"
model_path = "~/.atrium/models/whisper/ggml-large-v3.bin"
language = "zh"
sample_rate = 16000
chunk_duration_ms = 500
vad_enabled = true
vad_energy_threshold = 0.01

# ── 声纹识别配置 / Voiceprint Configuration ──
[voice.voiceprint]
enabled = false
service_url = ""                          # Python speechbrain gRPC 服务地址
similarity_threshold = 0.75               # 余弦相似度判定阈值

# ── 音频缓冲区配置 / Audio Buffer Configuration ──
[voice.audio]
sample_rate = 16000
channels = 1                              # mono
buffer_size = 16384                       # 环形缓冲区容量（样本数）
```

### 8.2 字段说明表

#### VoiceCfg（根配置）

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | `false` | 总开关，默认关闭 |
| `tts` | TtsCfg | — | TTS 引擎配置 |
| `stt` | SttCfg | — | STT 引擎配置 |
| `voiceprint` | VoiceprintCfg | — | 声纹识别配置 |
| `audio` | AudioCfg | — | 音频缓冲区配置 |

#### TtsCfg

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | `false` | 是否启用 TTS |
| `engine` | String | `"piper"` | 引擎类型 |
| `model_path` | String | `""` | Piper ONNX 模型路径 |
| `config_path` | String | `""` | Piper JSON 配置路径 |
| `sample_rate` | u32 | `22050` | Piper 原生采样率 |
| `target_sample_rate` | u32 | `16000` | 写入共享内存采样率 |
| `service_url` | String | `""` | GPT-SoVITS 服务地址 |
| `ref_audio_path` | String | `""` | 参考音频路径（5-10s） |
| `prompt_text` | String | `""` | 参考音频对应文本 |
| `prompt_lang` | String | `"zh"` | 参考音频语言 |
| `text_lang` | String | `"zh"` | 合成语言 |
| `timeout_secs` | u64 | `30` | 请求超时（秒） |
| `streaming_mode` | u8 | `0` | 流式模式（0=关,1=质量,2=中,3=快） |

#### SttCfg

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | `false` | 是否启用 STT |
| `engine` | String | `"whisper"` | 引擎类型 |
| `model_path` | String | `""` | whisper.cpp 模型路径 |
| `language` | String | `"zh"` | 识别语言 |
| `sample_rate` | u32 | `16000` | 采样率 |
| `chunk_duration_ms` | u32 | `500` | 流式分块时长（毫秒） |
| `vad_enabled` | bool | `true` | 是否启用 VAD |
| `vad_energy_threshold` | f32 | `0.01` | VAD 能量阈值 |

#### VoiceprintCfg

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enabled` | bool | `false` | 是否启用声纹识别 |
| `service_url` | String | `""` | speechbrain gRPC 服务地址 |
| `similarity_threshold` | f32 | `0.75` | 余弦相似度阈值 |

#### AudioCfg

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `sample_rate` | u32 | `16000` | 共享内存采样率 |
| `channels` | u16 | `1` | 声道数（mono=1） |
| `buffer_size` | usize | `16384` | 环形缓冲区容量（样本数） |

相关源码：`file:///d:/atrium/atrium/crates/atrium-voice/src/config.rs`

> 所有字段均标注 `#[serde(default)]` 或 `#[serde(default = "...")]`，保证配置文件中缺失的键能安全回退到默认值。

---

## 9. 单元测试

### 9.1 测试矩阵

| 模块 | Feature | 测试数量 | 覆盖内容 |
|------|---------|----------|----------|
| `prosody_bridge.rs` | 无（始终编译） | 24 | Piper / GPT-SoVITS 韵律映射、边界 clamp |
| `tts/piper.rs` | `tts-piper` | 5 | 降级模式初始化与合成 |
| `tts/gpt_sovits.rs` | `tts-gpt-sovits` | 12 | 配置、降级模式、HTTP 客户端构造 |
| `tts/engine.rs` | `tts-piper` / `tts-gpt-sovits` | 11 | VoiceEngine 统一接口、降级模式 |
| `stt/engine.rs` | `stt-whisper` | 9 | STT 引擎降级模式 |
| `stt/whisper.rs` | `stt-whisper` | 7 | whisper 初始化与降级识别 |
| `audio_buffer.rs` | 无 | 9 | SPSC 环形缓冲区读写 |
| `voiceprint.rs` | 无 | 10 | 声纹预留接口 |
| **Piper 总计** | `tts-piper` | **54** | — |
| **GPT-SoVITS 总计** | `tts-gpt-sovits` | **62** | — |

### 9.2 运行命令

```bash
# 运行全部测试（需启用对应 feature）/ Run all tests (requires features)
cargo test -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper"

# 仅 Piper 相关测试 / Piper only
cargo test -p atrium-voice --features tts-piper

# 仅 GPT-SoVITS 相关测试 / GPT-SoVITS only
cargo test -p atrium-voice --features tts-gpt-sovits

# 仅韵律桥接测试（无需 feature）/ Prosody bridge only (no feature needed)
cargo test -p atrium-voice prosody_bridge

# 显示输出 / Show output
cargo test -p atrium-voice --features "tts-piper,tts-gpt-sovits" -- --nocapture
```

### 9.3 期望输出

```
running 24 tests
test prosody_bridge::tests::test_pitch_12_semitones_doubles_frequency ... ok
test prosody_bridge::tests::test_pitch_minus_12_semitones_halves_frequency ... ok
test prosody_bridge::tests::test_pitch_zero_no_change ... ok
test prosody_bridge::tests::test_rate_fast_decreases_length ... ok
...
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 12 tests
test tts::gpt_sovits::tests::test_gpt_sovits_synthesize_degraded_mode ... ok
...
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

关键测试用例验证点：

| 测试名 | 验证内容 |
|--------|----------|
| `test_pitch_12_semitones_doubles_frequency` | +12 半音 → pitch_scale = 2.0 |
| `test_rate_fast_decreases_length` | rate=1.5 → length_scale ≈ 0.667 |
| `test_gpt_sovits_rate_fast_passthrough` | rate=1.5 → speed_factor=1.5（直接，非倒数） |
| `test_gpt_sovits_warmth_zero_to_temperature` | warmth=0 → temperature=0.8 |
| `test_gpt_sovits_warmth_full_to_temperature` | warmth=1 → temperature=1.2 |
| `test_piper_synthesize_degraded_mode` | 空模型路径 → 返回空 PCM |
| `test_gpt_sovits_synthesize_degraded_mode` | 空服务地址 → 返回空 PCM |

---

## 10. 端到端运行

### 10.1 启动流程

```bash
# 1. 准备配置文件 / Prepare config
#    编辑 atrium.toml，配置 [voice] 段

# 2. 编译（启用所需 feature）/ Build
cargo build --features "tts-piper,tts-gpt-sovits,stt-whisper"

# 3. （如使用 GPT-SoVITS）启动 Python 服务 / Start Python service
python api_v2.py --port 9880 &

# 4. 启动 Atrium / Start Atrium
cargo run
```

### 10.2 触发 TTS

TTS 通过 `VoiceEngine` 触发：

```rust
// 中文 / English
// 同步接口（Piper 后端）/ Sync interface (Piper backend)
let written = engine.synthesize_to_shm("你好，世界", &prosody)?;

// 异步接口（GPT-SoVITS 后端）/ Async interface (GPT-SoVITS backend)
let written = engine.synthesize_to_shm_async("你好，世界", &prosody).await?;
```

- `synthesize_to_shm()`：同步，适用于 Piper（本地 ONNX 推理）
- `synthesize_to_shm_async()`：异步，适用于 GPT-SoVITS（HTTP 网络调用）

合成后的 PCM 写入共享内存环形缓冲区，渲染引擎从缓冲区读取播放。

### 10.3 HTTP 接口测试

GPT-SoVITS 服务可直接通过 curl 测试：

```bash
# 测试 /tts 端点 / Test /tts endpoint
curl -X POST http://127.0.0.1:9880/tts \
  -H "Content-Type: application/json" \
  -d '{
    "text": "你好，我是数字生命。",
    "text_lang": "zh",
    "ref_audio_path": "/path/to/ref.wav",
    "prompt_text": "参考音频文本",
    "prompt_lang": "zh",
    "top_k": 15,
    "top_p": 1.0,
    "temperature": 1.0,
    "speed_factor": 1.0,
    "fragment_interval": 0.5,
    "repetition_penalty": 1.35,
    "media_type": "wav",
    "streaming_mode": 0
  }' --output test_output.wav

# 验证 WAV 文件 / Verify WAV file
file test_output.wav
# 期望输出：RIFF (little-endian) data, WAVE audio ...
```

### 10.4 日志验证

日志文件位于 `~/.atrium/logs/core.log`，验证语音能力是否正常加载：

```bash
# 查看语音相关日志 / Check voice-related logs
grep -i "voice\|tts\|piper\|gpt-sovits\|whisper" ~/.atrium/logs/core.log
```

期望日志片段：

```
# Piper 正常初始化 / Piper initialized
INFO atrium_voice::tts::piper: Piper 模型加载成功 / Piper model loaded

# GPT-SoVITS 降级模式（未配置时）/ GPT-SoVITS degraded mode
DEBUG atrium_voice::tts::gpt_sovits: GPT-SoVITS 降级模式 — 服务地址为空，返回空 PCM

# 合成请求 / Synthesis request
DEBUG atrium_voice::tts::gpt_sovits: GPT-SoVITS 请求: text=你好, speed=1.0, temp=1.0
```

---

## 11. 故障排查

### 11.1 常见问题

#### Q1：编译报错 `unresolved import atrium_voice::tts`

**原因**：未启用 TTS feature。

**解决**：
```bash
# 启用对应 feature / Enable corresponding feature
cargo build -p atrium-voice --features tts-piper
```

#### Q2：GPT-SoVITS 调用返回 `NotEnabled` 错误

**原因**：GPT-SoVITS 后端误用了同步接口 `synthesize_to_shm()`。

**解决**：改用异步接口 `synthesize_to_shm_async()`：
```rust
// 错误 ❌ / Wrong
engine.synthesize_to_shm("text", &prosody)?;

// 正确 ✅ / Correct
engine.synthesize_to_shm_async("text", &prosody).await?;
```

#### Q3：GPT-SoVITS HTTP 请求超时

**原因**：
- Python `api_v2.py` 服务未启动
- 端口 9880 被占用
- GPU 首次加载模型耗时过长

**解决**：
```bash
# 检查服务是否运行 / Check if service is running
curl http://127.0.0.1:9880/

# 增大超时配置 / Increase timeout
# atrium.toml: timeout_secs = 60
```

#### Q4：Piper 模型加载失败

**原因**：`model_path` 指向的文件不存在或格式错误。

**解决**：
- 确认 `.onnx` 和 `.onnx.json` 文件都存在
- 确认路径使用正确路径分隔符
- 检查文件完整性

#### Q5：STT 识别结果为空

**原因**：
- `model_path` 为空（降级模式）
- VAD 能量阈值过高，过滤了所有音频
- 音频采样率与配置不符

**解决**：
```toml
# 降低 VAD 阈值 / Lower VAD threshold
vad_energy_threshold = 0.005

# 确认模型路径非空 / Confirm model path is non-empty
model_path = "~/.atrium/models/whisper/ggml-large-v3.bin"
```

#### Q6：clippy 报警告

**解决**：
```bash
# 自动修复 / Auto-fix
cargo clippy -p atrium-voice --features "tts-piper,tts-gpt-sovits,stt-whisper" --fix

# 格式化 / Format
cargo fmt -p atrium-voice
```

### 11.2 调试模式

启用 debug 级别日志以排查问题：

```bash
# 设置环境变量启用 debug 日志 / Enable debug logging
RUST_LOG=atrium_voice=debug cargo run

# 仅 TTS 调试 / TTS only debug
RUST_LOG=atrium_voice::tts=debug cargo run

# 仅 GPT-SoVITS 调试 / GPT-SoVITS only debug
RUST_LOG=atrium_voice::tts::gpt_sovits=debug cargo run
```

降级模式相关日志（DEBUG 级别）：

```
DEBUG GPT-SoVITS 降级模式 — 服务地址为空，返回空 PCM
DEBUG Piper 降级模式 — 模型路径为空，返回空 PCM
DEBUG STT 降级模式 — 无模型，返回空文本
```

---

## 12. 当前限制与未来扩展

### 12.1 当前限制

| 限制 | 说明 |
|------|------|
| Piper 骨架实现 | 当前 `ort::Session` 加载为骨架实现，模型加载逻辑待完善 |
| GPT-SoVITS 同步接口不可用 | GPT-SoVITS 仅支持异步接口，同步调用返回 `NotEnabled` |
| STT FFI 骨架 | whisper.cpp FFI 为骨架实现，库未链接时返回降级结果 |
| 声纹识别预留 | `voiceprint.rs` 为 M9/M10 预留接口，尚未完整实现 |
| 无流式 TTS | 当前 TTS 为整句合成，未实现流式输出 |
| 单声道 | 音频缓冲区仅支持 mono（`channels = 1`） |
| 无 PCM 重采样 | `target_sample_rate` 与 `sample_rate` 不一致时需手动处理 |

### 12.2 未来扩展方向

- **流式 TTS**：支持分句流式合成，降低首字延迟
- **多音色切换**：运行时动态切换 Piper 模型 / GPT-SoVITS 参考音频
- **声纹识别落地**：对接 speechbrain gRPC 服务，实现说话人识别
- **PCM 重采样**：内置线性重采样，自动适配 `target_sample_rate`
- **情感标记**：SSML 标记驱动韵律参数动态调整
- **多声道支持**：支持立体声输出
- **whisper.cpp 完整集成**：链接 whisper.cpp 静态库，实现真实 FFI 推理

---

## 完成日志

| 章节 | 内容 | 状态 |
|------|------|------|
| 1. 概述 | 设计理念 + 数字生命工程定位 | ✅ 完成 |
| 2. 架构与数据流 | 模块结构 + 合成数据流图 | ✅ 完成 |
| 3. 环境准备 | 系统要求 + 工具链 + 数据目录 | ✅ 完成 |
| 4. Feature 开关与编译 | feature 矩阵 + 编译命令 + clippy 验证 | ✅ 完成 |
| 5. Piper TTS 后端 | 模型获取 + 配置 + 韵律映射 + 降级模式 | ✅ 完成 |
| 6. GPT-SoVITS TTS 后端 | Python 服务 + 自训练模型 + 配置 + 韵律映射 + WAV 解码 + Docker | ✅ 完成 |
| 7. STT 语音识别 | whisper.cpp 模型 + 配置 + gRPC AudioStream | ✅ 完成 |
| 8. 配置详解 | 完整配置示例 + 字段说明表 | ✅ 完成 |
| 9. 单元测试 | 测试矩阵 + 运行命令 + 期望输出 | ✅ 完成 |
| 10. 端到端运行 | 启动流程 + 触发 TTS + HTTP 测试 + 日志验证 | ✅ 完成 |
| 11. 故障排查 | 常见问题 + 调试模式 | ✅ 完成 |
| 12. 当前限制与未来扩展 | 限制清单 + 扩展方向 | ✅ 完成 |

---

> 本文档基于 `atrium-voice` crate 实际代码编写，所有代码引用均可在 `file:///d:/atrium/atrium/crates/atrium-voice/` 下找到对应源文件。
