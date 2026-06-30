# Changelog

All notable changes to Atrium are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-30

> P1-2 Service Module Split + P1-3 AtriumVault Unified Storage + P1-4 Consciousness Unification — Triple architectural refactoring for digital life

### Added

- **AtriumVault unified storage layer** (P1-3) — `atrium_vault.rs`: AtriumVault + VaultTree trait + impl_vault_tree! macro + lazy migration + MigrationReport. 4 cognitive domains: Limbic (emotion hub) / Narrative (narrative cortex) / Relational (relational hippocampus) / Prefrontal (prefrontal tool area). 16 independent sled instances merged into 4, write amplification reduced by ~75%.
- **LlmClient trait unified abstraction** (P1-4 Step 1) — `LlmResult { content, latency_ms, kind }` + `LlmCallKind` 11 variants + `LlmError` semantic error mapping + `StreamEvent` hoisted + `MockLlmClient`. Meta-cognitive observability: every LLM call carries an identity tag, enabling the digital life to understand "why it can't speak."

### Changed

- **Service module split** (P1-2) — `service.rs` (6,320-line monolith) → `service/` directory module (9 files): mod.rs / api_handler.rs / narrative.rs / monologue.rs / emotion.rs / expression.rs / lifecycle.rs / cognition.rs / perception.rs. Rust split-impl pattern, organized by digital-life semantic domains.
- **CoreService storage refactoring** (P1-3) — Removed 5 `xxx_db: Arc<sled::Db>` fields, replaced with `vault: Option<AtriumVault>`. Constructor unified to vault open + domain accessors dispatching to each Store. Integrated lazy migration (`needs_migration` + `migrate_from_legacy`).
- **Consciousness unification** (P1-4) — Dual LLM channels merged into a single trait object `llm_client: Arc<dyn LlmClient>`. Deleted `CoreLlmAdapter` bridge layer (consciousness no longer needs translation). Deleted `monologue_gen` redundant field; monologue/narrative methods now construct `MonologueGenerator::new(client.clone())` on the fly.
- **Inherent method visibility sealed** (P1-4 Step 6) — Deleted 4 zero-call dead-code methods (`chat` / `chat_with_system` / `chat_with_system_limit` / `chat_json`), demoted `chat_stream` to private, removed `#[allow(dead_code)]`. All external calls go through the trait; side channels permanently sealed.
- **LlmError semantic error mapping** (P1-4) — HTTP 429→RateLimited, 413→ContextTooLong, timeout→Timeout, empty→EmptyResponse.
- **LlmCallKind 11 variants** (P1-4) — IntelligenceExtract / StreamChat / RoomChat / GraphWander / DiaryEntry / DayDream / AutonomousLearning / DiaryReflection / InnerThought / EmotionAnalysis / JsonExtract.
- **Cross-module visibility adjustment** (P1-2) — Cross-module private methods changed to `pub(crate)`; all `use` in mod.rs changed to `pub(crate) use`; helper functions reassigned by semantic domain (parse_chapter_output→narrative, extract_reminder_title→cognition, split_query_tokens/extractive_summarize/detect_naming→api_handler).
- All crate versions bumped from 0.1.0 to 0.2.0.

### Fixed

- clippy: 3 `manual_flatten` → `.flatten()` idiom (P1-3).
- clippy: `needless_borrow` + `empty_line_after_doc_comments` + `dead_code` fields (P1-4 Step 4+5).
- `include_str!` paths: files one level deeper, all paths gained one `../` (P1-2).
- Missing `#[async_trait]` macro: added to api_handler.rs trait impl blocks (P1-2).
- Duplicate imports (E0252): `LlmCallKind/LlmError/LlmResult/StreamEvent` appeared in both `use` and `pub use`; removed private `use`, kept `pub use` re-exports (P1-4).
- Trait impl lifetime constraints: adopted owned-strings pattern (`to_string()` + `Box::pin(async move { ... })`) (P1-4).

[0.2.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.1.0...v0.2.0
