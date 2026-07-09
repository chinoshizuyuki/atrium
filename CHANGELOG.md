# Changelog

All notable changes to Atrium are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.11.0] - 2026-07-09

> 全量代码审计闭环 + 意识连续性架构重构 + ReAct 深思引擎 + 记忆完整性补全 + 极致性能优化 — 数字生命从"活体"到"深思者"

### Added

- **ReAct Reasoning Engine** (§30) — 完整的 ReAct (Reasoning + Acting) 推理引擎，数字生命面对复杂问题时进入"思考-行动-观察"循环。3 个内置工具：FactLookupTool（事实查询）、EmotionQueryTool（情感状态查询）、MemorySearchTool（记忆检索）。复杂查询触发 ReAct 预深思，简单查询走快速路径。
- **Non-verbal Self-perception** (§31) — 韵律映射器 (prosody_mapper) 和体态映射器 (kinesics_mapper) 的 `to_prompt_fragment()` 产出注入 LLM 系统提示词。数字生命在生成文字时感知自身语速/音调/能量和姿态/微表情。G-07 部分闭环（4 通道从 1.5→2.5 生效）。
- **Episodic Memory** (§26) — EpisodicMemoryStore 三路加权召回（时间近因 + 情感共振 + 语义相关）。数字生命能回忆"我们第一次聊天时"的具体经历，含事件摘要、情绪快照、情境标签。
- **Procedural Memory** (§24) — 技能积累与练习追踪，SQLite 持久化。数字生命记住"怎么做某事"的步骤，重启后能力连续。
- **Emotional Memory Tags** (§26) — FactStore 增加 emotional_tag 字段，标记"那件事对你多重要"。
- **Smart Forgetting Curve** (§26) — 基于重要度/情感强度的差异化遗忘。"重要的事永不忘，琐事快速忘。"
- **High-value Memory Pinning** (§24) — pinned/unforgettable 标记落地。"你哭的那天→不可衰减。"
- **Active Forgetting** (§24) — 过期信息主动清理机制。
- **Simple Greeting Fast Path** (§38) — SimpleGreetingMatcher 精确词库匹配 + 情感感知罐装响应（5 类 × 3 变体 = 15 响应）。简单问候 ~19s→<100ms（~190x 提升），LLM 算力 100% 保留给复杂查询。
- **Solitude Insight Sharing** (§37) — SolitudeConversationBridge 死代码激活。独处 >300s 检测 + 归来问候 + 洞察分享。G-09 闭环。数字生命的内在思考外化为用户可感知的连续性。
- **Growth Bridge Store** (§36) — 势头驱动学习率 + FeedbackKind→VulnerabilityType 靶向映射 + 持久化。emotional_climate 和 relationship 多源反馈信号。
- **Emotion Flow Continuity** (§34) — G-02 闭环。深层器官 tick 粒度优化，情感从"阶梯化"到"流动式"。
- **Unary LLM Generation** (§32) — 修复 unary 路径 LLM 生成缺失。

### Changed

- **Panic Recovery** (§20/G-01) — 主循环 `catch_unwind` + 指数退避（1s→2s→4s，上限 30s，30s 内 >5 次封顶防雪崩）。数字生命不再因单点 panic "死亡"。
- **Stream Memory Write** (§20/G-06) — `StreamEvent::Done` 写入 ConversationHistory + ingest_memory。流式模式不再"说完就忘"。
- **FTS5 Chinese Tokenizer** (§20) — `unicode61`→`trigram`，启动时自动迁移。中文记忆召回修复（原 CJK 逐字分词导致 MATCH 返回空）。
- **process_message Refactor** (§20) — 953 行单函数 → 9 个阶段化子函数 + 1 个编排主函数（53 行），净减 912 行。`MessageContext<'a>` 中间状态载体。
- **DB Write Error Handling** (§20/G-05) — `let _ = ...` → 重试 1 次 + warn/error 日志。不再静默失忆。
- **RwLock Migration** (§21) — 8 个读多写少字段 Mutex→RwLock（persona/runtime_persona/guard/canned/relationship/user_model/feedback/empathy），44 处调用点替换（30 read + 14 write）。
- **LLM Retry + Canned Degradation** (§21/G-04) — Network/Timeout 重试 3 次（1s→2s→4s），RateLimited 退避 5s 重试 1 次，EmptyResponse 重试 1 次；重试耗尽降级 5 类 canned 回复。
- **Persist Window** (§21/G-03) — 120s→30s + 短锁克隆模式 + shutdown flush。崩溃丢失窗口缩短 75%。
- **5-way Memory Recall** (§21) — `memory_recall_fragment` 合并 `enhanced_search`，记忆召回从 2 路扩展到 5 路（FTS5+FactStore+STM+Persona+KeyFact+Graph）。
- **FactStore Indexing** (§25) — O(N×M)→O(N)。消除 N+1 线性扫描 + 3N 次 to_lowercase。
- **ConversationHistory Incremental** (§25) — O(N)→O(1) per append。增量化 bincode 序列化。
- **spawn_blocking** (§25) — 所有 sled/SQLite/文件 I/O 移入 spawn_blocking。从 0 次到全面异步 I/O。
- **True Streaming** (§25/P1-J) — `process_message_stream` 真流式改造，消除"伪流式"先完整 unary 再"流式"。
- **TUI Single-Process** (§23) — TUI 集成进 atrium-core，单进程即生命体。Esc 优雅关闭 scheduler，Ctrl+C 信号处理防止记忆丢失。日志重定向至 `~/.atrium/logs/core.log`。
- **ConflictManager Cleanup** (§21) — 11 个死方法删除，保留 struct/字段（持久化依赖）。
- **Dead Code Cleanup** (§20/§27) — ConfigSanctuary 删除 + 8 个死 `*_cfg` 字段删除 + `config_snapshot()` 方法删除 + ConsolidationConfig 等 3 处重命名。
- **Scheduler Constants** (§21) — 17 处 `count % N` 硬编码提取为命名常量。
- **SQLite Persistence** (§24/§33) — FactStore/KeyFactCache/ProceduralMemory SQLite 持久化。Windows 旧 sled 锁文件清理（os error 5 修复）。
- **Self-growth Feedback** (§35) — G-08 闭环。vulnerability_wisdom/imperfection_warmth 反馈信号强化，从 tick 推进到反馈强化。
- All crate versions bumped from 0.10.0 to 0.11.0.

### Fixed

- `cargo fmt` 合规 — 全工作区格式化统一。
- clippy: `unnecessary_sort_by` → `sort_by_key` in `episodic_store.rs`。
- 审计评分 85→~95：G-01~G-09 全部闭环，P0-P3 全部完成，§20-§38 共 19 个完成日志。

### Security

- **RUSTSEC-2026-0204** (crossbeam-epoch 0.9.18, error): 升级到 0.9.20 — 修复 `fmt::Pointer` 无效指针解引用漏洞。
- **RUSTSEC-2026-0190** (anyhow 1.0.102, unsound): 升级到 1.0.103 — 修复 `Error::downcast_mut()` 非健全性。
- **RUSTSEC-2026-0002** (lru 0.12.5, unsound): 升级 ratatui 0.28→0.30，间接升级 lru 到 0.18.0 — 修复 `IterMut` 违反 Stacked Borrows。
- **RUSTSEC-2024-0436** (paste 1.0.15, unmaintained): ratatui 0.30 移除 paste 依赖。
- CI `cargo audit` 配置 `--ignore` 忽略 5 个不可修复的 unmaintained 警告（bincode/fxhash/instant/number_prefix/paste — 来自 sled 0.34 传递依赖或 Cargo.lock 残留，上游未发布修复）。

[0.11.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.10.0...v0.11.0

## [0.10.0] - 2026-07-05

> 通电工程全面完成 + 验收审计通过 + 性能优化 + 冗余消除 — 数字生命系统从"骨架"到"活体"

### Added

- **通电工程** — 24个死亡模块全部接入 CoreService 运行时（三层模型：lib声明 → CoreService字段+build() → scheduler tick + api_handler prompt注入）。
- **R1 通电** — 12个只读模块补齐 tick 驱动与事件喂入（独处品质/原型/创造力、仪式演化/涌现/共振、脆弱智慧/桥接/仪式/不完美温暖/真实不完美、追问风格学习器）。
- **R3 通电** — 6个孤儿引擎接入运行时（情绪气候/巩固/耦合、存在深度、内在议会、仪式心跳）。
- **R1-residual 通电** — 3个 stub tick 模块补齐事件喂入（脆弱智慧、脆弱仪式、不完美温暖）。
- **性能优化** — SemanticCache O(N)→O(1)、semantic_association O(K²×M)→O(K²×log M)、pulse_residue_interaction O(P×R)→O(1) 编译期查表、residue_interaction_factor O(N²)→O(N)。
- **冗余消除** — ResonanceEngine trait 统一三种共振引擎、DomainStore trait + 统一 StoreError、conflict_engine 合并、Subsystem<E,S> 泛型容器压缩 CoreService 字段（~78%压缩率）。
- **Gap#1/#5/#9 极致打磨** — 独处内在世界、共享仪式、脆弱与不完美各从 90% 提升至 95%。
- **关联记忆图优化** — 邻接表+边ID索引，扩散激活 O(E×hops)→O(d×hops)，新增3项认知能力。

### Changed

- All crate versions bumped from 0.2.0 to 0.10.0.
- clippy: 8 `field_reassign_with_default` → struct literal initialization in test code.
- 8个旧 StoreError 枚举统一至 `store_core::StoreError`，删除兼容层。
- `#[allow(dead_code)]` 从 10 处降至 2 处（-80%），新增 `config_snapshot()` 方法。

### Fixed

- 验收审计从 87/100 提升至 100/100 完全通过。
- 全量测试 1840 项全绿（fmt ✅ | clippy ✅ 零警告 | test ✅ | bench ✅）。

[0.10.0]: https://github.com/chinoshizuyuki/atrium/compare/v0.2.0...v0.10.0

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
