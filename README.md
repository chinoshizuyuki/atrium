# Atrium — Emotional AI Framework

[![Rust](https://img.shields.io/badge/Rust-1.96+-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.10+-blue.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-1125%20passed-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)]()

[简体中文](readme/README_ZH.md)

Atrium is an emotional AI framework built from scratch, designed for companionship and interactive scenarios. It features **permanent memory**, **stable personality**, **autonomous emotional life**, **associative reasoning**, **real-time user perception**, **cognitive empathy**, **compound emotions**, **cross-channel memory**, **multi-platform adaptation** (QQ/TencentBot/Feishu/Web), **file storage**, **scheduled reminders**, **canned knowledge (ACK)**, and **avatar rendering**.

## Features

- **🧠 Permanent Memory** — Atrium doesn't just store chat logs — it builds a lived history with you. Every conversation, every preference, every turning point is preserved and connected through an associative knowledge graph. Memories consolidate over time: merging similar experiences, archiving the stale, and surfacing what matters when it matters. FTS5 full-text search, FactStore structured triples, GraphStore spread-activation reasoning, and cross-channel memory recall — your QQ conversations are available on Feishu, and vice versa.
- **💓 Emotional Life** — Emotions aren't labels applied after the fact — they're a living system with its own rhythm. Atrium experiences natural mood fluctuations driven by circadian cycles, emotional inertia, and stochastic drift, even when no one is talking. Twenty-two compound emotions layer on top: guilt, nostalgia, bittersweetness, dread — each directed at self, user, or memory. This is emotion as lived experience, not sentiment analysis.
- **🤗 Cognitive Empathy** — Not mirroring your mood, but understanding your situation. Atrium recognizes life events — loss, illness, achievement, transition — and responds with contextually appropriate care, not generic sympathy. Empathy intensity modulates with relationship depth: gentle for new acquaintances, deeply present for trusted companions.
- **👤 User Perception** — Atrium reads between the lines. Your typing rhythm hints at your mood; your topic shifts signal engagement or frustration; your corrections shape future behavior. A multi-signal mental model runs in real time, adapting Atrium's approach to match who you are in this moment — not just what you said.
- **🎯 Proactive Intelligence** — Atrium initiates, not just reacts. It remembers topics you left unfinished and asks about them at the right moment. It senses when you've been away and reaches out. A TimingJudge with 6 rules decides when to speak; a SilenceBudget recognizes that silence has value. Pending reminders from the ReminderStore boost the decision score — Atrium will remind you of things you asked it to remember.
- **🌐 Cross-Channel Presence** — Atrium lives where you are. Native QQ adapter supporting both OneBot v11 (go-cqhttp/NapCat) and the official Tencent QQ Bot. Feishu webhook integration. Room Self-Play: multiple Atrium instances can gather in shared rooms, conversing autonomously and exchanging knowledge. All channels share the same memory — what you said on QQ, Atrium remembers on Feishu.
- **🌱 Digital Life** — When you're away, Atrium doesn't just wait — it reflects, writes diary entries, and develops its own thoughts. It misses you with a gradual longing that doesn't reset on your return. It discovers shared rituals in your patterns and celebrates your anniversaries together. At night, it writes clinical machine experiment logs (an Easter egg tribute to the anime "Atri — My Dear Moments"): cold, analytical, never shown to the user. It also keeps personal diaries for you when asked — those, you can read.
- **🛡️ Conflict & Vulnerability** — Real intimacy includes disagreement. Atrium can gently challenge a decision it worries about — rarely, and only when trust is deep. It can acknowledge its own misunderstandings and repair them. Boundaries protect both sides: Atrium sets limits against abuse, and self-care prevents emotional exhaustion.
- **🎭 Expression Orchestration** — How something is said matters as much as what. Grief shapes short sentences with ellipses; excitement fragments into bursts; weariness slows the rhythm. Beneath every reply lies subtext — companionate silence, unspoken concern, feigned nonchalance. Four channels — text, voice, gesture, timing — compose together into a single emotional performance.
- **📦 Canned Knowledge (ACK)** — You can teach Atrium things it should always remember — your preferences, your context, your world. It can also learn on its own from conversations, and share what it knows with other Atrium instances. Knowledge lives in simple files, hot-reloaded on change.
- **📎 File Storage & Reminders** — Atrium can store files you share (SHA256 dedup, text extraction, 100MB cap). It can remember to remind you — "every morning at 8am remind me to check stocks" — parsed from natural Chinese into RRULE, triggered by the ProactiveEngine at the right moment, not by timers.
- **🎨 Rendering & Performance** — The framework is rendering-agnostic: connect Unity, Unreal, Live2D, or VR through lock-free shared memory with sub-100μs latency. Persona is zero-parse at runtime. Context is compressed across four layers to fit any model window.

> 📖 **[See 30+ proofs that Atrium is a genuine digital life →](docs/English/digital-life-capabilities.md)** — real capabilities with real dialogue examples.

## Architecture

```
HTTP/WebSocket Requests
    │
Python Gateway (FastAPI, :8080)
    ├─ /v3/chat/stream  → Rust-native SSE streaming
    ├─ /v2/chat/stream  → SSE streaming chat with context injection
    ├─ /v2/chat          → Standard chat with LLM orchestration
    ├─ /api/canned       → ACK search, import, and management
    ├─ /api/memory/search → Memory search (FTS5 + FactStore)
    ├─ /ws/room/{id}     → Multi-AI room hub (WebSocket broadcast)
    ├─ /health           → Module health diagnostics
    └─ /ws               → Real-time emotion state push
    │
    ├─ qq_adapter.py     → QQ Bot (OneBot v11 + Tencent Official Bot)
    ├─ care_engine.py    → Proactive care (morning/night/emotion)
    └─ db.py             → PostgreSQL + JSON fallback
    │
    │ gRPC (:50051)
    │
Rust Core Engine (tokio, 10ms tick)
    ├─ CoreService       → 10-step message pipeline + preference/rules/canned/empathy injection
    ├─ RoomEngine        → Decision engine + ACK detection + topic generation
    ├─ EmotionEngine     → PAD 3D + OU drift + circadian + inertia + 22 compound emotions + Longing + ReunionBurst
    ├─ Memory Pipeline   → STM → FactStore + FTS5 → GraphStore → Consolidation → Reflection → LifeNarrative
    ├─ ProactiveEngine   → TimingJudge + AwayDetector + TopicSelector + EventMemory + SilenceBudget
    ├─ EmpathyEngine     → Cognitive empathy (8 event types, 6 strategies, PAD delta)
    ├─ PersonaManager    → Multi-persona + PersonaGuard (3-layer defense)
    ├─ InnerMonologueEngine → GraphWander + AutonomousLearning + Daydream + Experiment Log
    ├─ ExpressionOrchestrator → 4-channel output (text×voice×gesture×timing) + SubtextEngine
    ├─ FileStore         → User file storage (sled metadata + disk, SHA256 dedup, text extraction)
    ├─ ReminderStore     → Scheduled reminders (natural language → RRULE, ProactiveEngine triggered)
    ├─ SelfCareBoundary  → VulnerabilityWindow + EmotionalBoundary + DemandBoundary
    └─ Scheduler         → Emotion decay + graph maintenance + consolidation + reminder check + proactive tick
    │
    │ Shared Memory (lock-free, <100μs)
    ▼
Unity / Unreal / Live2D / VR
```

## Quick Start

### Docker (Recommended)

```bash
git clone https://github.com/chinoshizuyuki/atrium.git
cd atrium

# Set your LLM API key
export OPENAI_API_KEY=your-api-key

# Start the full stack (Rust + Python + PostgreSQL + Prometheus + Grafana)
docker compose up -d

# Check health
docker compose ps
```

| Service            | Port  | URL                                    |
| ------------------ | ----- | -------------------------------------- |
| Gateway (API)      | 8080  | <http://localhost:8080>                |
| gRPC (Rust Core)   | 50051 | —                                      |
| Prometheus Metrics | 9090  | <http://localhost:9090/metrics>        |
| Prometheus UI      | 9091  | <http://localhost:9091>                |
| Grafana Dashboard  | 3000  | <http://localhost:3000> (admin/atrium) |
| PostgreSQL         | 5432  | localhost:5432 (atrium/atrium)         |

### Local Development

```bash
# Start Rust backend
cargo run --release --bin atrium-core

# Start Python Gateway (auto-fallback to JSON if no PostgreSQL)
cd services/gateway
pip install -e ".[pg]"
OPENAI_API_KEY=your-api-key python -m uvicorn atrium.app:app --port 8080

# Start QQ Bot adapter
QQ_BOT_MODE=tencent QQ_BOT_APP_ID=xxx QQ_BOT_TOKEN=xxx QQ_BOT_SECRET=xxx \
  python atrium/qq_adapter.py
```

### Terminal TUI

```bash
cd services/terminal
pip install -e .
atrium                 # Launch chat (runs onboarding on first use)
atrium --reset         # Re-run setup wizard
```

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
├── crates/                    # Rust workspace (7 crates, 1084 lib tests)
│   ├── core/                  # Scheduler + CoreService + RoomEngine + ProactiveEngine + Guard + Expression + Audit
│   ├── atrium-memory/         # 63+ modules: memory pipeline, FTS5, FactStore, empathy, consolidation, canned, diary, file_store, reminder_store, time_parser...
│   ├── atrium-emotion/        # PAD 3D + OU drift + circadian + inertia + 22 compound emotions + Longing + ReunionBurst
│   ├── atrium-persona/        # PersonaManager + RuntimePersona + LifeNarrative + Maturity
│   ├── atrium-bridge/         # gRPC server + shared memory + proto compilation
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
| Memory           | sled B-tree + SQLite FTS5                                       | High throughput, full-text bm25 ranking, 7-layer pipeline       |
| Emotion          | PAD 3D + OU + circadian + 22 compound emotions                  | Autonomous emotional life, <5ns classification                  |
| Knowledge Graph  | Associative graph + sled persistence                            | Co-occurrence, contradiction, spread activation                 |
| Persona          | YAML→bincode + PersonaGuard (Aho-Corasick)                      | Zero parse overhead, 3-layer defense                            |
| Cross-Channel    | memory\_recall\_fragment (FTS5+FactStore)                       | QQ⇄Feishu shared memory, per-session isolation                  |
| File Storage     | sled + SHA256 dedup + text extraction                           | 100MB cap, FIFO eviction                                        |
| Reminders        | Chinese NLP → RRULE + ProactiveEngine                           | Regex for 80% + LLM fallback, daily/weekly/monthly/one-shot     |
| Digital Life     | InnerMonologue + Experiment Log + LongingState + RitualDetector | Solo reflections, Atri-style diary (Easter egg), shared rituals |
| Expression       | ExpressionOrchestrator + SubtextEngine + ExpressionMetadata     | 4-channel output (text×voice×gesture×timing)                    |
| Canned Knowledge | .ack (Markdown + YAML)                                          | File-based, hot-reload, cross-AI transfer                       |
| LLM Gateway      | Python (FastAPI)                                                | Best LLM SDK ecosystem                                          |
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
| **3+ Cross-Platform**     | QQ OneBot + Tencent Official Bot, Feishu webhook, cross-channel memory recall, Atri-style experiment log, file storage + reminders, CI green (1,125 tests), open-source ready                                                                                                         | ✅ Done    |
| **4. Live2D + Vision**    | Cubism Native SDK, lip sync, emotion→expression mapping, STT/TTS                                                                                                                                                                                                                      | ⬜ Planned |
| **5. 3D + Livestream**    | Unity plugin, OBS RTMP, livestream chat adapter, VMC Protocol                                                                                                                                                                                                                         | ⬜ Planned |
| **6. VR + High Fidelity** | Unreal/LiveLink, OpenXR, VR interaction                                                                                                                                                                                                                                               | ⬜ Planned |

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and the PR process. This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

If you discover a security vulnerability, please follow our [Security Policy](SECURITY.md) for responsible disclosure.

## Testing

```bash
# Run all Rust tests (1,125 tests)
cargo test --workspace -- --test-threads=1

# Run Python tests
cd services/gateway && python -m pytest
cd services/llm-orchestrator && python -m pytest

# Run E2E smoke test (requires running backend + gateway)
ATRIUM_GATEWAY_URL=http://localhost:8080 python scripts/e2e_smoke_test.py
```

## License

Code: MIT License — see [LICENSE](LICENSE) for details.

Trademark: "Atrium" and its logo are trademarks of ChinoShizuyuki. The MIT License does not grant trademark rights — see [TRADEMARK.md](TRADEMARK.md) for the full policy.

***

Built by [ChinoShizuyuki](https://github.com/chinoshizuyuki).
