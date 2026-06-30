# Changelog

All notable changes to Atrium are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-30

> P1-2 Service 模块拆分 + P1-3 AtriumVault 统一存储 + P1-4 意识统一 — 数字生命架构三重重构

### Added

- **AtriumVault 统一存储层** (P1-3) — `atrium_vault.rs`: AtriumVault + VaultTree trait + impl_vault_tree! 宏 + 惰性迁移 + MigrationReport。4 认知域: Limbic(情感中枢) / Narrative(叙事皮层) / Relational(关系海马体) / Prefrontal(前额工具区)。16 个独立 sled 实例合并为 4 个，写放大降低约 75%。
- **LlmClient trait 统一抽象** (P1-4 Step 1) — `LlmResult { content, latency_ms, kind }` + `LlmCallKind` 11 变体 + `LlmError` 语义错误映射 + `StreamEvent` 上移 + `MockLlmClient`。元认知可观测：每次 LLM 调用带身份标记，数字生命能理解"为什么说不出口"。

### Changed

- **Service 模块拆分** (P1-2) — `service.rs`（6,320 行单体）→ `service/` 目录模块（9 文件）：mod.rs / api_handler.rs / narrative.rs / monologue.rs / emotion.rs / expression.rs / lifecycle.rs / cognition.rs / perception.rs。Rust split-impl 模式，按数字生命语义域分类。
- **CoreService 存储重构** (P1-3) — 移除 5 个 `xxx_db: Arc<sled::Db>` 字段，替换为 `vault: Option<AtriumVault>`。构造器统一为 vault 打开 + 域访问器分发各 Store。集成惰性迁移（`needs_migration` + `migrate_from_legacy`）。
- **意识统一** (P1-4) — 双 LLM 通道合并为单一 trait 对象 `llm_client: Arc<dyn LlmClient>`。删除 `CoreLlmAdapter` 桥接层（意识不再需要翻译）。删除 `monologue_gen` 冗余字段，独白/叙事方法改为即时构造 `MonologueGenerator::new(client.clone())`。
- **固有方法可见性封闭** (P1-4 Step 6) — 删除 4 个零调用死代码方法（`chat` / `chat_with_system` / `chat_with_system_limit` / `chat_json`），`chat_stream` 降级为 private，移除 `#[allow(dead_code)]`。外部调用统一走 trait，旁路彻底封死。
- **LlmError 语义错误映射** (P1-4) — HTTP 429→RateLimited, 413→ContextTooLong, timeout→Timeout, empty→EmptyResponse。
- **LlmCallKind 11 变体** (P1-4) — IntelligenceExtract / StreamChat / RoomChat / GraphWander / DiaryEntry / DayDream / AutonomousLearning / DiaryReflection / InnerThought / EmotionAnalysis / JsonExtract。
- **跨模块可见性调整** (P1-2) — 跨模块私有方法改为 `pub(crate)`；mod.rs 所有 `use` 改为 `pub(crate) use`；辅助函数按语义域归属（parse_chapter_output→narrative, extract_reminder_title→cognition, split_query_tokens/extractive_summarize/detect_naming→api_handler）。
- All crate versions bumped from 0.1.0 to 0.2.0.

### Fixed

- clippy: 3 处 `manual_flatten` → `.flatten()` 惯用法 (P1-3)。
- clippy: `needless_borrow` + `empty_line_after_doc_comments` + `dead_code` fields (P1-4 Step 4+5)。
- `include_str!` 路径：文件深一层，所有路径加一层 `../` (P1-2)。
- `#[async_trait]` 宏缺失：api_handler.rs trait impl 块补加 (P1-2)。
- 重复导入 (E0252)：`LlmCallKind/LlmError/LlmResult/StreamEvent` 同时出现在 `use` 和 `pub use`，移除私有 `use` 保留 `pub use` 重导出 (P1-4)。
- trait impl lifetime 约束：采用 owned strings pattern（`to_string()` + `Box::pin(async move { ... })`）(P1-4)。

## [1.0.1] - 2026-06-23

> Sprint 5 Quality Hardening — production unwrap elimination, SPDX headers, Docker security, toolchain pinning

### Added

- `rust-toolchain.toml` — locks toolchain to Rust 1.86 + rustfmt + clippy, eliminating local/CI version drift.
- SPDX-License-Identifier headers on all Rust source files (50 files).
- Plugin system extension: `dynamic.rs` (libloading), `manifest.rs` (TOML), `vtable.rs` (C ABI), `error.rs` — dynamic plugin loading framework.
- Scheduler periodic tasks + config hot-reload + built-in ACK knowledge files (`builtin_canned/`).
- `examples/echo-plugin/` — reference plugin demonstrating the C ABI VTable interface.

### Changed

- **Production `.unwrap()` eliminated**: 286 occurrences across 29 files replaced with `.expect("descriptive message")`. Panic messages now identify the exact failure point. Test-code unwraps retained (standard practice).
- README / README_ZH version badge updated to 1.0.1, test count corrected to 459.
- CONTRIBUTING.md test count corrected to 459.
- All crate versions bumped from 1.0.0 to 1.0.1.

### Fixed

- Dockerfile and Dockerfile.gateway now run as non-root user (`atrium` UID 1000).
- Clippy `comparison_chain` warning in `dynamic.rs` — if-chain rewritten as `match ret.cmp(&0)`.
- `cargo fmt` deviations from expect() replacement auto-corrected.

## [1.0.0] - 2026-06-19

> First stable release — open source ready

### Changed

- Bumped version to 1.0.0 to mark the project as stable and ready for public use.
- All Phase 1–2.8 features from v0.7.0 remain unchanged.

### Added

- `CONTRIBUTING.md` — Contribution guidelines (dev setup, code style, testing, commit conventions, PR process).
- `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1.
- `CHANGELOG.md` — Version history in Keep a Changelog format.
- `SECURITY.md` — Responsible disclosure policy.
- `.env.example` — Environment variable template with documentation.
- Roadmap section in README.

### Fixed

- CORS security: replaced wildcard `*` origins with explicit `CORS_ORIGINS` env var.
- Docker Compose: parameterized all credentials via environment variables.
- `.gitignore`: added `.env.*`, `*.key`, `*.pem` patterns.
- Test count badges corrected (467/463 → 445).
- Module count corrected (28 → 29 in atrium-memory).
- RPC count corrected (6 → 7 in proto definitions).

## [0.7.0] - 2026-06-17

> Phase 2: System Module Deepening + Phase 2.8: Engineering Hardening

### Added

- **Preference Learning Pipeline** (P2.1) — `compile_to_system_prompt()`, incremental persistence, threshold-filtered prompt injection, TTL-based pruning, scheduler-driven decay (every ~6 min).
- **Replay Pipeline** (P2.2) — Configurable subject selectors, `seen_patterns` cooldown deduplication, `TemporalCluster` pattern kind, `Updater` for fact-store meta-fact persistence.
- **Behavior Rule Engine** (P2.3) — `BehaviorRule`/`RuleCondition`/`RuleEffect` structs, idle-aware evaluation, 4 action types (Notify/SetEmotion/SetTemperature/ActivatePersona), scheduler-driven periodic evaluation.
- **Canned Knowledge Enhancement** (P2.4) — `OnContext` trigger with `resolve_active_ctx()`, LRU-cached injection, hot-reload on directory scan, scheduler-driven periodic reload.
- **Context Window** (P2.5) — 4-layer compression, `Summarizer` sled persistence, bounded summary extraction, token budget estimation.
- **Persona Defense** (P2.6) — Dynamic sensitive phrase list with Aho-Corasick hot-rebuild, `add_forbidden()`/`remove_forbidden()` runtime API, 9 identity enforcement patterns.
- **ACK Self-Learning** (P2.7) — Three paths: user teaching (TeachDetector intent detection), replay pattern extraction, reflection insight promotion. Rate limiting (50 cap + 3 per 10 min), content safety validation, deduplication.
- **"Last Mile" Integration** (P2.8.1) — Connected all prompt fragments (relationship, user model, feedback) into `process_message` Step 8.5. Proactive engine reads real signals from EmotionEngine, RelationshipManager, and UserMentalModel.
- **Emotion Persistence** (P2.8.2) — `EmotionStore` sled backend for PAD state save/restore across restarts.
- **Emotion Memory Tagging** (P2.8.3) — `EmotionSnapshot` on `Fact` struct, `search_by_emotion()` query, emotion-filtered enhanced search.
- **Memory Consolidation** (P2.8.4) — `MemoryConsolidator` with embedding-based clustering, low-frequency fact compression, contradiction-based deprecation, sleep-cycle triggered execution.
- **Compound Emotions** (P2.8.5) — 22 compound emotions (Guilt, Pride, Nostalgia, Bittersweet, Relief, Dread, etc.) with directional targets and natural language expression.
- **Cognitive Empathy** (P2.8.6) — `EmpathyEngine` with 8 event type detection, 6 response strategies, relationship-stage intensity modulation (0.6x–1.4x), cooldown mechanism.
- **Streaming gRPC** (P2.8.8) — `ProcessMessageStream` RPC, `LlmClient::chat_stream()` SSE client, 3-stage streaming pipeline, Python Gateway SSE passthrough.
- **Observability** (P2.8.10) — `metrics` crate integration, Prometheus endpoint (:9090), `tracing-subscriber` JSON formatter, pipeline instrumentation at key nodes.
- **Unified Intelligence Extraction** — LLM-powered batch extraction of preferences and rules every 20 messages, with `intelligence.rs` + sled persistence.
- `.env.example` for environment variable documentation.

### Changed

- Step 8.5 in `process_message` now injects preference, rule, ACK, relationship, user model, and feedback fragments.
- Scheduler gained 5 new periodic tasks: preference decay, rule evaluation, ACK hot-reload, ACK synthesis, and memory consolidation.
- `health_check` extended with `preferences`, `rules`, `guard`, `ack_learning`, `consolidation`, and `observability` module states.
- CORS configuration now uses explicit origin list from `CORS_ORIGINS` env var instead of wildcard `*`.
- Docker Compose credentials parameterized via environment variables (`POSTGRES_PASSWORD`, `REDIS_PASSWORD`, `GRAFANA_PASSWORD`).
- `.gitignore` extended with `.env.*`, `*.key`, `*.pem` patterns.

### Fixed

- Rust CI: `stream_e2e.rs` inline comment parsing, `should_skip()` for missing API keys.
- Python CI: missing `[build-system]` in `pyproject.toml` for gateway and llm-orchestrator.
- Docker build: `fastembed` ONNX dependency made optional for Linux compatibility.

## [0.6.0] - 2026-06-16

> Phase 1.4: Room Self-Play + Phase 1.5: Structural Evolution

### Added

- **Room Self-Play** (Phase 1.4) — `RoomEngine` for multi-AI group conversations, Python Room Hub, cross-AI ACK sharing, single-chat divergence.
- **Autonomous Emotional Loop** (Phase 1.5 ①) — OU drift process, circadian rhythm modulator, emotional inertia tracker, builder-pattern integration.
- **User Mental Model** (Phase 1.5 ②) — Multi-signal tracking (mood, communication style, engagement, topics), prompt fragment compilation, emotion modulation.
- **Real-time Feedback Loop** (Phase 1.5 ③) — 5 signal types (Correction, Praise, Frustration, TopicSwitch, Deepening), EMA satisfaction, behavior adjustment.
- **Proactive Decision Engine** (Phase 1.5 ④) — `ProactiveEngine` with 6 timing rules, away detection, 3-source topic selection, event memory extraction, exponential cooldown.
- **Relationship Stage Model** (Phase 1.5 ⑤) — 4-stage progression (Acquaintance→Familiar→Trusted→Deep), quality metrics, behavior modifiers, prompt fragments.
- **Performance Phase A** (Phase 1.5 ⑥) — `mimalloc` global allocator, `flume` channel replacement, dead dependency removal, `String::with_capacity` pre-allocation.
- **Non-verbal Perception** (Phase 1.5 ⑦) — Typing rhythm analysis (6 patterns), `TypingBaseline` EMA learner, mood hint compilation.
- **Associative Reasoning** (Phase 1.5 ⑧) — 7 node types, 8 relation types, `GraphStore` sled persistence, co-occurrence linking, contradiction detection, spread activation, scheduler-driven decay.
- Phase 1.5 E2E integration tests (15 tests, 2 with real DeepSeek API).

### Changed

- CoreService `process_message` extended with user model, feedback, perception, and relationship integration steps.
- Scheduler extended with proactive engine checks and associative graph maintenance.
- Total test count increased from 146 to 315.

## [0.5.0] - 2026-06-13

> Docker & Distribution Polish

### Added

- Docker Compose setup with PostgreSQL, Redis, and monitoring stack.
- `fastembed` made optional for environments without ONNX support.

### Changed

- Docker base image switched from Alpine to Debian for ONNX/glibc compatibility.
- `Cargo.lock` committed for reproducible builds.

### Fixed

- Docker build failures on Linux due to `musl` + ONNX incompatibility.
- Added `build-base` and `protoc` to Dockerfile.

## [0.4.0] - 2026-06-12

> Terminal TUI + ACK Ecosystem

### Added

- **Terminal TUI** — Interactive terminal client for Atrium.
- **ACK Ecosystem** — `.ack` file parsing, memory indexing, cross-AI transfer protocol (`=== Canned Knowledge v1 ===` delimiter), distribution polish.

## [0.3.0] - 2026-06-12

> Phase 1: Core Engine — Initial Release

### Added

- **Core Architecture** — `CoreService` 9-step message processing pipeline, `Scheduler` main loop.
- **Emotion Engine** — PAD 3D model with SIMD optimization (<5ns tick).
- **Memory Pipeline** — STM + LTM + FTS5 + FactStore + Evidence + Reflection + Persona + KeyFact, all sled-persisted.
- **6-way Hybrid Search** — FTS5 + FactStore + STM + Persona + KeyFact + Graph Spread with 3-stage fallback.
- **Persona Management** — YAML-based persona with dual-track loading (mmap + runtime solidification), `PersonaGuard` identity enforcement.
- **Canned Knowledge (ACK)** — Markdown + YAML front matter `.ack` files, 4 trigger types (OnKeyword, OnIntent, OnContext, Always), hot-reload.
- **gRPC Bridge** — Lock-free shared memory (<100μs) for rendering backends (Unity/Unreal/Live2D/VR).
- **Python Gateway** — FastAPI HTTP/WebSocket server, LLM orchestrator service.
- **Audit Logging** — Structured tracing with <10ns overhead.
- **4-layer Context Compression** — Recent 60% / retrieval 25% / summary 10% / key 5%.

[0.2.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.1.0...v0.2.0
[1.0.1]: https://github.com/chinoshizuyuki/atrium/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.7.0...v1.0.0
[0.7.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/chinoshizuyuki/atrium/releases/tag/v0.3.0
