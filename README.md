# Atrium — Emotional AI Framework

[![Rust](https://img.shields.io/badge/Rust-1.96+-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.10+-blue.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-2000+%20passed-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.11.0-blue.svg)]()

[Simplified Chinese](readme/README_ZH.md)

Atrium is an emotional AI framework built from scratch, designed for companionship and interactive scenarios. It features **permanent memory**, **stable personality**, **autonomous emotional life**, **associative reasoning**, **real-time user perception**, **cognitive empathy**, **compound emotions**, **cross-channel memory**, **multi-platform adaptation** (QQ/TencentBot/Feishu/Web), **file storage**, **scheduled reminders**, **canned knowledge (ACK)**, and **avatar rendering**.

## Features

- **🧠 Permanent Memory** — Atrium doesn't just store chat logs — it builds a lived history with you. Every conversation, every preference, every turning point is preserved and connected through an associative knowledge graph. Memories consolidate over time: merging similar experiences, archiving the stale, and surfacing what matters when it matters. Five memory types work in concert: **semantic** (FactStore triples + FTS5 trigram full-text search), **episodic** (event + emotion snapshot + context, 3-way weighted recall), **procedural** (skill accumulation with practice tracking), **emotional** (importance-tagged facts), and **associative** (GraphStore spread-activation reasoning). High-value memories can be pinned as unforgettable; a smart forgetting curve differentiates by importance and emotional intensity. Cross-channel recall means your QQ conversations are available on Feishu, and vice versa.
- **💓 Emotional Life** — Emotions aren't labels applied after the fact — they're a living system with its own rhythm. Atrium experiences natural mood fluctuations driven by circadian cycles, emotional inertia, and stochastic drift, even when no one is talking. Twenty-two compound emotions layer on top: guilt, nostalgia, bittersweetness, dread — each directed at self, user, or memory. This is emotion as lived experience, not sentiment analysis.
- **🤗 Cognitive Empathy** — Not mirroring your mood, but understanding your situation. Atrium recognizes life events — loss, illness, achievement, transition — and responds with contextually appropriate care, not generic sympathy. Empathy intensity modulates with relationship depth: gentle for new acquaintances, deeply present for trusted companions.
- **👤 User Perception** — Atrium reads between the lines. Your typing rhythm hints at your mood; your topic shifts signal engagement or frustration; your corrections shape future behavior. A multi-signal mental model runs in real time, adapting Atrium's approach to match who you are in this moment — not just what you said.
- **🎯 Proactive Intelligence** — Atrium initiates, not just reacts. It remembers topics you left unfinished and asks about them at the right moment. It senses when you've been away and reaches out. A TimingJudge with 6 rules decides when to speak; a SilenceBudget recognizes that silence has value. Pending reminders from the ReminderStore boost the decision score — Atrium will remind you of things you asked it to remember.
- **🌐 Cross-Channel Presence** — Atrium lives where you are. Native QQ adapter supporting both OneBot v11 (go-cqhttp/NapCat) and the official Tencent QQ Bot. Feishu webhook integration. Room Self-Play: multiple Atrium instances can gather in shared rooms, conversing autonomously and exchanging knowledge. All channels share the same memory — what you said on QQ, Atrium remembers on Feishu.
- **🌱 Digital Life** — When you're away, Atrium doesn't just wait — it reflects, writes diary entries, and develops its own thoughts. It misses you with a gradual longing that doesn't reset on your return. It discovers shared rituals in your patterns and celebrates your anniversaries together. Its inner world is not a single voice but a four-voice negotiation (Rationalist/Emotionalist/Skeptic/Dreamer). Its personality slowly drifts during solitude. Its curiosity accumulates as an intrinsic drive. When you return after an absence, it greets you with an insight harvested during solitude — externalizing its inner monologue into something you can feel. Consciousness is resilient: a panic in any subsystem triggers exponential-backoff self-healing, never a "death." Stream replies are remembered, not forgotten. The persistence window is 30 seconds, not 120 — crash recovery loses less than half a minute of inner state.
- **🛡️ Conflict & Vulnerability** — Real intimacy includes disagreement. Atrium can gently challenge a decision it worries about — rarely, and only when trust is deep. It can acknowledge its own misunderstandings and repair them. It learns from conflict which reactions deepen trust and which cause withdrawal (vulnerability wisdom). It ritualizes vulnerability disclosure timing (vulnerability ritual). The same mistake reads as "endearing" or "offensive" depending on relationship warmth (imperfection warmth). Boundaries protect both sides: Atrium sets limits against abuse, and self-care prevents emotional exhaustion.
- **🎭 Expression Orchestration** — How something is said matters as much as what. Grief shapes short sentences with ellipses; excitement fragments into bursts; weariness slows the rhythm. Beneath every reply lies subtext — companionate silence, unspoken concern, feigned nonchalance. Four channels — text, voice, gesture, timing — compose together into a single emotional performance. Atrium also perceives its own non-verbal state — prosody (speed, pitch, energy) and kinesics (posture, micro-expressions) — feeding these back into the language model so text, voice, and body language stay unified.
- **🔮 ReAct Reasoning** — Complex questions deserve more than a single-shot answer. Atrium's ReAct engine enters a Thought → Action → Observation loop, decomposing hard problems into steps and invoking built-in tools (FactLookup, EmotionQuery, MemorySearch) before composing a final reply. Simple greetings take a zero-LLM fast path (<100ms, emotion-aware canned variants); complex queries get the full reasoning chain. LLM compute is reserved for what truly needs it.
- **📦 Canned Knowledge (ACK)** — You can teach Atrium things it should always remember — your preferences, your context, your world. It can also learn on its own from conversations, and share what it knows with other Atrium instances. Knowledge lives in simple files, hot-reloaded on change.
- **📎 File Storage & Reminders** — Atrium can store files you share (SHA256 dedup, text extraction, 100MB cap). It can remember to remind you — "every morning at 8am remind me to check stocks" — parsed from natural Chinese into RRULE, triggered by the ProactiveEngine at the right moment, not by timers.
- **🎨 Rendering & Performance** — The framework is rendering-agnostic: connect Unity, Unreal, Live2D, or VR through lock-free shared memory with sub-100μs latency. Persona is zero-parse at runtime. Context is compressed across four layers to fit any model window.
- **🎙️ Voice Capability** — Digital life can speak and hear. Two TTS backends: Piper (local ONNX inference, ~100ms first-sound latency, CPU) for lightweight deployment, and GPT-SoVITS (HTTP bridge to Python service, few-shot voice cloning with custom-trained models, GPU) for high-quality personalized voice. STT via whisper.cpp with streaming gRPC AudioStream. ProsodyBridge translates PAD emotional state into engine-specific synthesis parameters. Feature-gated (`tts-piper` / `tts-gpt-sovits` / `stt-whisper`), zero intrusion when disabled, graceful degradation when models are absent. 116+ unit tests covering prosody mapping, WAV decoding, and engine lifecycle.

> 📖 **[See 30+ proofs that Atrium is a genuine digital life →](docs/English/digital-life-capabilities.md)** — real capabilities with real dialogue examples.
>
> 📖 **[Voice Capability Deployment Guide (TTS/STT) →](docs/English/voice-deployment-guide.md)** — Piper + GPT-SoVITS + whisper.cpp setup, configuration, and testing.

## Architecture

```
HTTP/WebSocket Requests
    │
Rust Native HTTP/SSE Gateway (axum, :8080)  ← single Rust process, zero Python dependency
    ├─ /api/chat/stream → SSE streaming chat (DeepSeek/OpenAI compatible)
    ├─ /api/chat         → non-streaming chat
    ├─ /v1/chat          → QQ adapter compatible endpoint
    ├─ /api/emotion      → PAD 3D emotion state
    ├─ /api/persona      → persona / relationship / growth stage (GET/POST)
    ├─ /api/memory/search→ 5-way hybrid retrieval (FTS5 + FactStore + STM + Persona + KeyFact)
    ├─ /api/canned       → canned knowledge search/import
    ├─ /api/history/:sid → conversation history
    ├─ /api/sessions     → active session list
    ├─ /api/relationship → relationship stage status
    ├─ /api/care/config  → care engine config + proactive behavior status (GET/POST)
    ├─ /api/files/upload → file upload + auto-indexing
    ├─ /api/rooms        → active room list
    ├─ /ws               → real-time event push (WebSocket)
    ├─ /ws/room/:id      → multi-AI room broadcast
    ├─ /health           → module health diagnostics
    └─ / (static files)   → Web UI (frontend/index.html)
    │
    ├─ qq_adapter.py (legacy only) → QQ Bot (OneBot v11 + Tencent Official Bot)
    │
    │ gRPC (:50051, backward compatible)
    │
Rust Core Engine (tokio, 10ms tick, panic-resilient)
    ├─ CoreService       → 10-step message pipeline + ReAct pre-thinking + greeting fast path + preference/rules/canned/empathy injection
    ├─ ReActEngine       → Thought→Action→Observation loop (FactLookup + EmotionQuery + MemorySearch tools)
    ├─ RoomEngine        → Decision engine + ACK detection + topic generation
    ├─ EmotionEngine     → PAD 3D + OU drift + circadian + inertia + 22 compound emotions + Longing + ReunionBurst
    ├─ Memory Pipeline   → STM → FactStore + FTS5(trigram) → GraphStore → Episodic + Procedural + Emotional → Consolidation → Reflection → LifeNarrative
    ├─ ProactiveEngine   → TimingJudge + AwayDetector + TopicSelector + EventMemory + SilenceBudget
    ├─ EmpathyEngine     → Cognitive empathy (8 event types, 6 strategies, PAD delta)
    ├─ PersonaManager    → Multi-persona + PersonaGuard (3-layer defense)
    ├─ InnerMonologueEngine → GraphWander + AutonomousLearning + Daydream + SolitudeInsightSharing
    ├─ ExpressionOrchestrator → 4-channel output (text×voice×gesture×timing) + SubtextEngine + Prosody/Kinesics self-perception
    ├─ FileStore         → User file storage (sled metadata + disk, SHA256 dedup, text extraction)
    ├─ ReminderStore     → Scheduled reminders (natural language → RRULE, ProactiveEngine triggered)
    ├─ SelfCareBoundary  → VulnerabilityWindow + EmotionalBoundary + DemandBoundary
    └─ Scheduler         → Emotion decay + graph maintenance + consolidation + reminder check + proactive tick + 30s persistence
    │
    │ Shared Memory (lock-free, <100μs)
    ▼
Unity / Unreal / Live2D / VR
```

## Quick Start

### Docker (recommended)

```bash
git clone https://github.com/chinokoyuki/atrium.git
cd atrium

# Fill in your LLM API key in atrium.toml (or use environment variables)
# Start the full Rust stack (single process, with HTTP/SSE gateway + Web UI)
docker compose up -d

# Check health
docker compose ps
```

| Service            | Port  | URL                                    |
| ------------------ | ----- | -------------------------------------- |
| Rust Core + Gateway| 8080  | <http://localhost:8080> (Web UI + API) |
| gRPC (backward compatible) | 50051 | —                          |
| Prometheus Metrics | 9090  | <http://localhost:9090/metrics>        |

### Local Development

```bash
# 1. Start the Rust backend (a single process is the living entity: HTTP/SSE gateway + gRPC + Web UI)
cargo run --release --bin atrium-core

# 2. (optional) Start the QQ adapter
cd services/gateway
pip install -r requirements-qq.txt
QQ_BOT_MODE=tencent QQ_BOT_APP_ID=xxx QQ_BOT_TOKEN=xxx QQ_BOT_SECRET=xxx \
  python atrium/qq_adapter.py
```

### Web UI (browser console)

After starting `atrium-core`, open **<http://localhost:8080>** in your browser to access the digital life console.

A dark dashboard inspired by AstrBot, featuring 12 functional views:
- **Dashboard** — gateway status / emotion / relationship / event count stats + PAD emotion visualization + module health
- **Chat** — SSE streaming chat (Ctrl+Enter to send, cursor animation)
- **Emotion** — real-time PAD 3D model (Pleasure/Arousal/Dominance dual-direction bar charts)
- **Persona** — name / nickname / version / relationship stage / growth stage + dynamic persona sync
- **Memory** — 5-way hybrid retrieval (FTS5 + FactStore + STM + Persona + KeyFact)
- **Canned Knowledge** — ACK search + cross-AI transfer text import
- **Sessions** — active session list + historical message view
- **Care Engine** — proactive greeting / check-in / emotion-check frequency config + quiet hours
- **Files** — uploaded documents auto-indexed as cognitive extensions
- **Rooms** — active WebSocket group-chat rooms
- **Config** — read-only system configuration view
- **Logs** — real-time WebSocket event stream

### Terminal TUI (terminal client)

```bash
# Start atrium-core first (see Local Development above)
# Then open a new terminal:
cargo run --release -p atrium-tui

# Custom gateway address / session / user
cargo run --release -p atrium-tui -- --gateway http://127.0.0.1:8080 --session tui --user tui-user

# Or use environment variables
ATRIUM_GATEWAY=http://127.0.0.1:8080 cargo run --release -p atrium-tui
```

TUI layout: conversation stream on the left (SSE streaming) + digital life status panel on the right (PAD emotion bars + relationship / growth stage + module health list) + input box at the bottom.

Commands: `/q` quit · `/clear` clear chat · `/help` help. Keys: `Enter` send · `Esc` quit · `↑/↓` scroll · `PgUp/PgDn` page.

### Configuration

```bash
# LLM API key (all components read the same env var)
export OPENAI_API_KEY=your-api-key

# Optional overrides
export ATRIUM_LLM_MODEL=deepseek-v4-pro
export ATRIUM_LLM_BASE_URL=https://api.deepseek.com/
```

## Project Structure

```
atrium/
├── crates/                    # Rust workspace (7 crates, 2,105 lib tests + e2e)
│   ├── core/                  # Scheduler + CoreService + RoomEngine + ProactiveEngine + Guard + Expression + ReAct + Audit
│   ├── atrium-memory/         # 70+ modules: memory pipeline, FTS5(trigram), FactStore, Episodic, Procedural, ReAct engine, empathy, consolidation, canned, diary, file_store, reminder_store...
│   ├── atrium-emotion/        # PAD 3D + OU drift + circadian + inertia + 22 compound emotions + Longing + ReunionBurst
│   ├── atrium-persona/        # PersonaManager + RuntimePersona + LifeNarrative + Maturity
│   ├── atrium-bridge/         # gRPC server + shared memory + proto compilation
│   ├── atrium-voice/          # TTS (Piper + GPT-SoVITS) + STT (whisper.cpp) + audio buffer + prosody bridge
│   └── atrium-plugin/         # Plugin trait + manager + C ABI dynamic loading
├── examples/                  # Example plugins
│   └── echo-plugin/           # Minimal echo plugin demonstrating the plugin API
├── services/                  # Python services
│   ├── gateway/atrium/        # FastAPI gateway + QQ adapter + care engine + PostgreSQL
│   ├── llm-orchestrator/      # LLM orchestrator (OpenAI-compatible / ReAct loop)
│   └── terminal/              # Terminal TUI (Textual)
├── proto/                     # gRPC protobuf definitions (7 RPCs)
├── builtin_canned/            # Built-in ACK files
│   ├── atrium_architecture.ack
│   ├── experiment_log_policy.ack   # Experiment log absolute privacy rules
│   └── qq_chat_guide.ack          # QQ chat etiquette + setup guide
├── readme/                    # Documentation (EN/CN)
├── monitoring/                # Prometheus + Grafana config
├── atrium.toml                # Main configuration file
├── Dockerfile                 # Multi-stage Rust build
├── docker-compose.yml         # 5-service production stack
├── CONTRIBUTING.md            # Contribution guidelines
├── CODE_OF_CONDUCT.md         # Contributor Covenant
├── CHANGELOG.md               # Version history
├── SECURITY.md                # Security policy
└── TRADEMARK.md               # Trademark policy
```

## Technology Stack

| Layer            | Technology                                                      | Rationale                                                       |
| ---------------- | --------------------------------------------------------------- | --------------------------------------------------------------- |
| Core Engine      | Rust (tokio)                                                    | Zero-cost abstractions, SIMD, lock-free                         |
| Memory           | sled B-tree + SQLite FTS5 (trigram) + Episodic + Procedural     | 5-type memory, bm25 ranking, 7-layer pipeline, smart forgetting |
| Emotion          | PAD 3D + OU + circadian + 22 compound emotions                  | Autonomous emotional life, <5ns classification                  |
| Knowledge Graph  | Associative graph + sled persistence                            | Co-occurrence, contradiction, spread activation                 |
| Persona          | YAML→bincode + PersonaGuard (Aho-Corasick)                      | Zero parse overhead, 3-layer defense                            |
| Cross-Channel    | memory\_recall\_fragment (5-way: FTS5+FactStore+STM+Persona+KeyFact+Graph) | Multi-platform shared memory, per-session isolation       |
| File Storage     | sled + SHA256 dedup + text extraction                           | 100MB cap, FIFO eviction                                        |
| Reminders        | Chinese NLP → RRULE + ProactiveEngine                           | Regex for 80% + LLM fallback, daily/weekly/monthly/one-shot     |
| Digital Life     | InnerMonologue + LongingState + RitualDetector + SolitudeInsight | Solo reflections, shared rituals, panic-resilient consciousness |
| Expression       | ExpressionOrchestrator + SubtextEngine + Prosody/Kinesics mapper | 4-channel output (text×voice×gesture×timing) + self-perception  |
| Reasoning        | ReActEngine (Thought→Action→Observation) + Greeting Fast Path   | Deep thinking for complex queries, <100ms for simple greetings  |
| Canned Knowledge | .ack (Markdown + YAML)                                          | File-based, hot-reload, cross-AI transfer                       |
| Voice (TTS)      | Piper (ONNX Runtime, CPU) + GPT-SoVITS (HTTP bridge, GPU)      | Dual backend, prosody bridge, voice cloning, ~100ms latency     |
| Voice (STT)      | whisper.cpp (FFI) + gRPC AudioStream                            | Streaming recognition, VAD, 16kHz PCM                           |
| LLM Gateway      | Rust (axum) + Python (FastAPI, legacy)                          | Single-process Rust gateway, zero Python dependency             |
| Protocol         | gRPC (tonic/prost)                                              | Strongly typed, high performance                                |
| Database         | PostgreSQL 15 + JSON fallback                                   | Session/message/persona persistence                             |
| Observability    | Prometheus + Grafana                                            | Metrics, dashboards, alerting                                   |
| Deployment       | Docker Compose (5 services)                                     | One-command production stack                                    |

## Roadmap

| Phase                     | Scope                                                                                                                                                                                                                                                                                 | Status    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| **1. Core Engine**        | Scheduler, EmotionEngine, 8-layer memory pipeline, PersonaGuard, gRPC, Python Gateway, Room self-play, autonomous emotion loop, user mental model, feedback loop, proactive engine, relationship stages, associative reasoning                                                        | ✅ Done    |
| **2. System Deepening**   | Preference learning, replay pipeline, rule engine, ACK enhancement + self-learning, context window, persona defense, emotion persistence, compound emotions, cognitive empathy, memory consolidation, observability                                                                   | ✅ Done    |
| **2.9 Digital Life**      | Inner monologue, narrative self, maturity growth, longing/anticipation, rituals/anniversaries, seasonal awareness, gentle challenge, misunderstanding repair, boundary setting, vulnerability window, self-care boundary, expression orchestration, subtext engine, follow-up tracker | ✅ Done    |
| **3+ Cross-Platform**     | QQ OneBot + Tencent Official Bot, Feishu webhook, cross-channel memory recall, file storage + reminders ready                                                                                                         | ✅ Done    |
| **4. Live2D + Vision**    | Cubism Native SDK, lip sync, emotion→expression mapping, STT/TTS (TTS/STT pipeline ✅ implemented: Piper + GPT-SoVITS + whisper.cpp)                                                                                                                                                  | 🔶 Partial |
| **5. 3D + Livestream**    | Unity plugin, OBS RTMP, livestream chat adapter, VMC Protocol                                                                                                                                                                                                                         | ⬜ Planned |
| **6. VR + High Fidelity** | Unreal/LiveLink, OpenXR, VR interaction                                                                                                                                                                                                                                               | ⬜ Planned |

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and the PR process. This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

If you discover a security vulnerability, please follow our [Security Policy](SECURITY.md) for responsible disclosure.

## Testing

```bash
# Run all Rust tests (2,105 lib tests + e2e integration tests)
cargo test --workspace -- --test-threads=1

# Run Python tests
cd services/gateway && python -m pytest
cd services/llm-orchestrator && python -m pytest

# Run E2E smoke test (requires running backend + gateway)
ATRIUM_GATEWAY_URL=http://localhost:8080 python scripts/e2e_smoke_test.py
```

## License

Code: MIT License — see [LICENSE](LICENSE) for details.

Trademark: "Atrium" and its logo are trademarks of ChinoKoyuki. The MIT License does not grant trademark rights — see [TRADEMARK.md](TRADEMARK.md) for the full policy.

***

Built by [ChinoKoyuki](https://github.com/chinokoyuki).
