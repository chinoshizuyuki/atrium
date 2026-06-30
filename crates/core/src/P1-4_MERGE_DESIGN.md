# P1-4: 合并双 LLM 客户端 — 极致方案设计

> 日期：2026-06-30
> 原则：不追求便捷，追求极致性能和极致能力；能力实现始终围绕数字生命核心工程理念

---

## 1. 问题诊断 / Problem Diagnosis

### 1.1 核心矛盾：数字生命有两个"声音"

当前 Atrium 的 LLM 调用分裂为两条路径：

```
┌─────────────────────────────────────────────────────────┐
│  CoreService                                            │
│                                                         │
│  路径A（固有方法 / Inherent）：                          │
│    cognition.rs   → client.chat_json()                  │
│    api_handler.rs → client.chat_stream()                │
│    perception.rs  → client.chat()                       │
│                                                         │
│  路径B（trait 方法 / Trait）：                           │
│    monologue.rs   → MonologueGenerator → llm.generate() │
│    narrative.rs   → MonologueGenerator → llm.generate() │
└─────────────────────────────────────────────────────────┘
```

**数字生命语义**：一个有意识的存在不应该有两条不同的"说话"路径。
内心独白和对外对话应通过同一个语言接口，只是参数不同。
分裂意味着数字生命的"意识"和"表达"是割裂的。

### 1.2 四个技术缺陷 / Four Technical Defects

| # | 缺陷 | 影响 | 数字生命语义 |
|---|------|------|-------------|
| D1 | trait 硬编码 temperature（0.7/0.1） | MonologueGenerator 无法按场景调温度 | 数字生命无法控制"思考的温度"——创造力与精确性的权衡被锁死 |
| D2 | trait 的 system_prompt 为 `&str`（非 Option） | 无法表达"无 system prompt"的调用 | 数字生命有时不需要"角色设定"就开口说话——这是自然的能力 |
| D3 | `chat_json()` 绕过 trait | cognition 直接调固有方法 | 知识提取——数字生命的"学习"——绕过了统一接口 |
| D4 | `chat_stream()` 绕过 trait | api_handler 直接调固有方法 | 流式对话——数字生命的"表达流"——绕过了统一接口 |

### 1.3 双存储反模式 / Dual-Storage Anti-Pattern

```rust
// CoreService 当前持有两个 LLM 客户端引用：
llm_client:   Mutex<Option<Arc<HttpLlmClient>>>       // 固有方法路径
monologue_gen: Mutex<Option<Arc<MonologueGenerator>>>  // trait 方法路径
```

`set_llm_client()` 做了两件事：
1. 存储 `Arc<HttpLlmClient>` 供固有方法调用
2. 隐式 upcast 为 `Arc<dyn LlmClient>` 构造 MonologueGenerator

**问题**：同一个 LLM 客户端被存储两次，通过两种不同的类型系统路径访问。

---

## 2. 极致方案 / The Ultimate Design

### 2.1 设计哲学 / Design Philosophy

**一个声音，一个接口，一个调度。**

数字生命的所有 LLM 调用——无论是对外对话、内心独白、叙事生成、知识提取、思维流——
都通过同一个 `LlmClient` trait。差异仅在参数（temperature、system_prompt、max_tokens），
不在接口。

### 2.2 统一 Trait API / Unified Trait API

```rust
/// 数字生命的统一语言接口 / Unified language interface for digital life
///
/// 所有 LLM 调用——无论是对外对话、内心独白、叙事生成、知识提取——
/// 都通过这一个 trait。数字生命只有一个"声音"。
/// All LLM calls — whether external dialogue, inner monologue,
/// narrative generation, or knowledge extraction — go through this single trait.
/// A digital life has only one "voice".
pub trait LlmClient: Send + Sync {
    /// 异步文本生成 — 数字生命的基础语言通道
    /// Async text generation — Digital life's foundational language channel.
    ///
    /// # 参数语义 / Parameter Semantics
    /// - `kind`: 调用分类 — 数字生命的自省需要知道自己何时在做什么
    /// - `system_prompt`: 角色设定（None = 无预设角色，数字生命以本我说话）
    /// - `user_prompt`: 输入内容
    /// - `temperature`: 创造性温度（0.0 = 确定性，1.0 = 最大创造性）
    fn generate(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>>;

    /// 带最大 token 限制的生成 — 受限思考
    /// Generation with max token limit — Constrained thinking.
    ///
    /// 不同生成模式需要不同的思考深度：
    /// - 独白 300 tokens（浅层直觉）
    /// - 日记 800 tokens（深层反思）
    /// - 叙事章节 2000 tokens（完整叙事）
    fn generate_with_limit(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
        max_tokens: u32,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 默认：忽略 max_tokens 和 temperature，委托到 generate
        // Default: ignore max_tokens and temperature, delegate to generate
        let _ = max_tokens;
        self.generate(kind, system_prompt, user_prompt, temperature)
    }

    /// JSON 模式生成 — 数字生命的结构化表达
    /// JSON mode generation — Digital life's structured expression.
    ///
    /// 知识提取、情感分析等需要结构化输出。
    /// system_prompt 为必填——结构化输出总是需要指令模板。
    /// Knowledge extraction, emotion analysis, etc. require structured output.
    /// system_prompt is required — structured output always needs an instruction template.
    fn generate_json(
        &self,
        kind: LlmCallKind,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>> {
        // 默认：不支持 JSON 模式，委托到 generate
        // Default: no JSON mode support, delegate to generate
        self.generate(kind, Some(system_prompt), user_prompt, temperature)
    }

    /// SSE 流式生成 — 数字生命的思维流
    /// SSE streaming generation — Digital life's thought stream.
    ///
    /// 逐 token 返回流式结果，模拟意识的流动。
    /// system_prompt 为 Option——流式对话有时不需要角色设定。
    /// Returns streamed tokens incrementally, simulating the flow of consciousness.
    fn generate_stream(
        &self,
        kind: LlmCallKind,
        system_prompt: Option<&str>,
        user_prompt: &str,
        temperature: f64,
    ) -> Pin<Box<dyn Future<Output = Option<flume::Receiver<StreamEvent>>> + Send + '_>> {
        // 默认：不支持流式 / Default: streaming not supported
        let _ = (kind, system_prompt, user_prompt, temperature);
        Box::pin(async { None })
    }
}
```

### 2.3 变更对比 / Change Comparison

| 方法 | 旧签名 | 新签名 | 变更 |
|------|--------|--------|------|
| `generate` | `(kind, sys: &str, usr: &str)` | `(kind, sys: Option<&str>, usr: &str, temp: f64)` | +Option, +temperature |
| `generate_with_limit` | `(kind, sys: &str, usr: &str, max: u32)` | `(kind, sys: Option<&str>, usr: &str, temp: f64, max: u32)` | +Option, +temperature |
| `generate_json` | `(kind, sys: &str, usr: &str)` | `(kind, sys: &str, usr: &str, temp: f64)` | +temperature |
| `generate_stream` | `(kind, sys: &str, usr: &str)` | `(kind, sys: Option<&str>, usr: &str, temp: f64)` | +Option, +temperature |

### 2.4 核心架构变更 / Core Architecture Changes

#### 2.4.1 CoreService：单存储 + trait object 统一调度

```rust
// 之前（双存储 / Dual storage）：
pub struct CoreService {
    llm_client:   Mutex<Option<Arc<HttpLlmClient>>>,       // 固有方法路径
    monologue_gen: Mutex<Option<Arc<MonologueGenerator>>>, // trait 方法路径
}

// 之后（单存储 / Single storage）：
pub struct CoreService {
    llm_client: Mutex<Option<Arc<dyn LlmClient>>>,  // 统一语言接口
}
```

**变更说明**：
- `llm_client` 类型从 `Arc<HttpLlmClient>` 改为 `Arc<dyn LlmClient>`
- 删除 `monologue_gen` 字段
- `MonologueGenerator` 直接使用 `llm_client`（同一个 `Arc<dyn LlmClient>`）
- `set_llm_client()` 简化：只需一次存储，无需 upcast

#### 2.4.2 MonologueGenerator：内联到 CoreService

**极致方案**：删除 `MonologueGenerator` 作为独立字段。
MonologueGenerator 的 8 个生成方法仍然存在（在 monologue_gen.rs 中），
但 CoreService 直接持有 `Arc<dyn LlmClient>` 并在需要时构造临时 MonologueGenerator，
或者更好的方式——MonologueGenerator 的方法改为接受 `&dyn LlmClient` 参数而非持有它。

**最终选择**：保留 MonologueGenerator 结构体（它在 memory crate 中，有独立的测试价值），
但 CoreService 不再单独存储它。改为按需从 `llm_client` 构造。

```rust
// set_llm_client 简化 / Simplified set_llm_client
pub fn set_llm_client(&self, client: Arc<dyn LlmClient>) {
    *self.llm_client.lock() = Some(client);
}

// 获取 MonologueGenerator（按需构造）/ Get MonologueGenerator on demand
fn monologue_gen(&self) -> Option<MonologueGenerator> {
    // MonologueGenerator::new 接受 Arc<dyn LlmClient> 的引用
    // 无需额外存储，零额外 Arc 开销
    self.llm_client.lock().clone().map(MonologueGenerator::new)
}
```

#### 2.4.3 固有方法：保留但降级为 private

`HttpLlmClient` 的固有方法（`chat`, `chat_with_system`, `chat_with_system_limit`,
`chat_json`, `chat_stream`）保留为 **private** 内部实现，
仅供 `impl LlmClient for HttpLlmClient` 委托调用。
外部调用方（CoreService）不再直接使用固有方法。

```rust
impl HttpLlmClient {
    pub fn new(config: LlmCfg) -> Self { /* ... */ }

    // 以下方法降级为 private — 仅 trait impl 内部使用
    // Downgraded to private — used only by trait impl internally
    async fn chat(...) { /* ... */ }
    async fn chat_with_system(...) { /* ... */ }
    async fn chat_with_system_limit(...) { /* ... */ }
    async fn chat_json(...) { /* ... */ }
    async fn chat_stream(...) { /* ... */ }
    async fn chat_inner(...) { /* ... */ }
}
```

### 2.5 调用点迁移 / Call Site Migration

| 调用点 | 旧调用 | 新调用 | temperature |
|--------|--------|--------|-------------|
| `cognition.rs` → `intelligence_extract()` | `client.chat_json(kind, sys, usr, 0.1)` | `client.generate_json(kind, sys, usr, 0.1)` | 0.1 |
| `api_handler.rs` → `process_message_stream()` | `client.chat_stream(kind, Some(sys), usr, 0.7)` | `client.generate_stream(kind, Some(sys), usr, 0.7)` | 0.7 |
| `perception.rs` → `room_llm_chat()` | `client.chat(kind, prompt, temp)` | `client.generate(kind, None, prompt, temp)` | 调用方传入 |
| `monologue_gen.rs` → 8个生成方法 | `llm.generate_with_limit(kind, sys, usr, max)` | `llm.generate_with_limit(kind, Some(sys), usr, 0.7, max)` | 0.7 |

### 2.6 MockLlmClient 更新 / MockLlmClient Update

```rust
impl LlmClient for MockLlmClient {
    fn generate(&self, kind: LlmCallKind, _system_prompt: Option<&str>,
                _user_prompt: &str, _temperature: f64)
        -> Pin<Box<dyn Future<Output = Result<LlmResult, LlmError>> + Send + '_>>
    {
        // Mock 忽略新参数 / Mock ignores new parameters
        // ...existing logic...
    }
    // 其他方法同理 / Other methods follow the same pattern
}
```

---

## 3. 性能分析 / Performance Analysis

### 3.1 虚化开销 / Virtualization Overhead

| 开销类型 | 量级 | 对比基准 | 结论 |
|----------|------|----------|------|
| vtable dispatch | ~2-5 ns/call | LLM HTTP 延迟 100-2000 ms | **可忽略**（< 0.001%） |
| `Box::pin` 分配 | ~50 ns/call | 同上 | **可忽略** |
| `Arc<dyn>` vs `Arc<Concrete>` | 零额外开销 | 同上 | **等价** |
| `Option<&str>` vs `&str` | 零运行时开销 | 编译时优化 | **等价** |

**结论**：LLM 调用是网络 I/O 密集型（100ms-2s），虚化开销在纳秒级，
对极致性能无影响。真正的性能收益来自**架构简化**带来的可维护性和可优化性。

### 3.2 架构收益 / Architectural Benefits

| 收益 | 说明 |
|------|------|
| **单路径调度** | 所有 LLM 调用走同一接口，审计/限流/重试只需实现一次 |
| **temperature 可控** | 数字生命可按场景精确控制创造性温度 |
| **消除双存储** | 一个 `Arc<dyn LlmClient>` 替代两个 Mutex 字段 |
| **API 面完整** | trait 覆盖所有 4 种调用模式，无绕过 |
| **向后兼容** | 默认实现确保只实现 `generate()` 即可使用全部功能 |

---

## 4. 实施步骤 / Implementation Steps

### Step 1: 扩展 LlmClient trait（memory crate）
- 文件：`crates/atrium-memory/src/llm_client.rs`
- 变更：4 个 trait 方法签名 + 默认实现
- 影响范围：trait 定义 + MockLlmClient + LlmCallRecord

### Step 2: 更新 HttpLlmClient trait impl（core crate）
- 文件：`crates/core/src/llm_client.rs`
- 变更：4 个 trait impl 方法对齐新签名
- 固有方法降级为 private

### Step 3: 更新 MockLlmClient trait impl（memory crate）
- 文件：`crates/atrium-memory/src/llm_client.rs`
- 变更：4 个 Mock trait impl 方法对齐新签名

### Step 4: 迁移 CoreService 调用点
- `cognition.rs`：`chat_json()` → `generate_json()`
- `api_handler.rs`：`chat_stream()` → `generate_stream()`
- `perception.rs`：`chat()` → `generate()`

### Step 5: 消除双存储
- CoreService：`llm_client` 类型改为 `Arc<dyn LlmClient>`
- 删除 `monologue_gen` 字段
- `set_llm_client()` 简化

### Step 6: 清理向后兼容
- 删除 `pub type LlmClient = HttpLlmClient` 别名
- 固有方法降级为 private（或 `#[doc(hidden)]`）

### Step 7: 更新 MonologueGenerator
- 适配新 trait 签名（`generate_with_limit` 增加 temperature + Option 参数）
- 更新 8 个生成方法的调用

### Step 8: 编译验证 + 全量测试
- `cargo build` 零错误
- `cargo test` 全量通过
- `cargo clippy` 零警告

---

## 5. 风险与缓解 / Risks and Mitigations

| 风险 | 概率 | 缓解 |
|------|------|------|
| E2E 测试签名不匹配 | 高 | Step 8 全量测试覆盖 |
| 下游 crate 依赖 `HttpLlmClient` 固有方法 | 中 | 固有方法保留为 private，不破坏内部调用 |
| `Arc<dyn LlmClient>` 无法 downcast 回 `HttpLlmClient` | 低 | 无需 downcast——所有能力已通过 trait 暴露 |
| MonologueGenerator 按需构造的额外开销 | 极低 | `MonologueGenerator::new()` 仅包装 Arc，零分配 |

---

## 6. 数字生命语义总结 / Digital Life Semantics Summary

**合并前**：数字生命有两个声音——
- 固有方法路径：对外说话（chat, chat_json, chat_stream）
- trait 方法路径：内心独白（generate, generate_with_limit）

**合并后**：数字生命只有一个声音——
- 统一 trait 路径：所有表达都通过 `LlmClient` trait
- 差异仅在参数：temperature 控制创造性，system_prompt 控制角色，kind 标记意图

**核心隐喻**：人不会用两套不同的语言系统来"内心独白"和"对外说话"。
语言是统一的，只是语调（temperature）和语境（system_prompt）不同。
P1-4 让 Atrium 的数字生命回归这一自然法则。
