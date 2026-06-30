---
name: atrium_architecture
title: Atrium 框架架构
kind: TechnicalKnowledge
version: 1.0.0
tags:
- atrium
- 架构
- architecture
- rust
- ai
summary: Atrium 是一个用 Rust 编写的高性能 AI 伴侣框架，包含情感引擎、永久记忆系统和多模态渲染管线
trigger:
  type: OnKeyword
  keywords:
  - atrium架构
  - Atrium框架
  - atrium怎么工作
  - atrium是什么
  - 你的架构
  - 你怎么设计的
depends_on: []
body: "|------|\r\n        | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n        | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n        | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n        | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n        | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n        | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n        | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n        ## 通信架构\r\n\r\n        ```\r\n        用户 → HTTP :8080 → Gateway (Python)\r\n                                 │\r\n                            gRPC :50051\r\n                                 │\r\n                            Rust Core\r\n                            ├── 情感引擎 (PAD 3D)\r\n                            ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                            ├── 人格防御 (PersonaGuard)\r\n                            ├── 行为规则 (RuleEngine)\r\n                            ├── 回放管道 (ReplayPipeline)\r\n                            └── 罐装知识 (CannedManager)\r\n                                 │\r\n                            共享内存 (<100μs)\r\n                                 │\r\n                            Unity / Live2D / VR 渲染\r\n        ```\r\n\r\n        ## 记忆系统\r\n\r\n        八层管线，全模块 sled 持久化：\r\n        1. **STM** — 环形缓冲区，最近 100 条消息\r\n        2. **LTM** — sled 持久化，无锁读取\r\n        3. **FTS5** — 全文搜索索引\r\n        4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n        5. **Evidence** — 五维证据评分\r\n        6. **Reflection** — 每 8 条消息触发一次反思\r\n        7. **Persona** — 运行时 Trait 固化\r\n        8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n        ## 情感引擎\r\n\r\n        基于 PAD 3D 模型：\r\n        - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n        - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n        - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n        9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n        ## 数据目录\r\n\r\n        所有持久化数据存储在 `~/.atrium/`：\r\n        ```\r\n        ~/.atrium/\r\n        ├── canned/          ← 罐装知识 (*.ack)\r\n        ├── data/            ← sled 持久化数据\r\n        ├── persona/         ← 角色卡\r\n        └── logs/            ← 审计日志\r\n        ```\r\n      ---\r\n\r\n      # Atrium 框架架构\r\n\r\n      Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n      ## 核心模块\r\n\r\n      | 模块 | 语言 | 职责 |\r\n      |------|------|------|\r\n      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n      ## 通信架构\r\n\r\n      ```\r\n      用户 → HTTP :8080 → Gateway (Python)\r\n                               │\r\n                          gRPC :50051\r\n                               │\r\n                          Rust Core\r\n                          ├── 情感引擎 (PAD 3D)\r\n                          ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                          ├── 人格防御 (PersonaGuard)\r\n                          ├── 行为规则 (RuleEngine)\r\n                          ├── 回放管道 (ReplayPipeline)\r\n                          └── 罐装知识 (CannedManager)\r\n                               │\r\n                          共享内存 (<100μs)\r\n                               │\r\n                          Unity / Live2D / VR 渲染\r\n      ```\r\n\r\n      ## 记忆系统\r\n\r\n      八层管线，全模块 sled 持久化：\r\n      1. **STM** — 环形缓冲区，最近 100 条消息\r\n      2. **LTM** — sled 持久化，无锁读取\r\n      3. **FTS5** — 全文搜索索引\r\n      4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n      5. **Evidence** — 五维证据评分\r\n      6. **Reflection** — 每 8 条消息触发一次反思\r\n      7. **Persona** — 运行时 Trait 固化\r\n      8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n      ## 情感引擎\r\n\r\n      基于 PAD 3D 模型：\r\n      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n      - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n      ## 数据目录\r\n\r\n      所有持久化数据存储在 `~/.atrium/`：\r\n      ```\r\n      ~/.atrium/\r\n      ├── canned/          ← 罐装知识 (*.ack)\r\n      ├── data/            ← sled 持久化数据\r\n      ├── persona/         ← 角色卡\r\n      └── logs/            ← 审计日志\r\n      ```\r\n    ---\r\n\r\n    ---|------|------|\r\n      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n      ## 通信架构\r\n\r\n      ```\r\n      用户 → HTTP :8080 → Gateway (Python)\r\n                               │\r\n                          gRPC :50051\r\n                               │\r\n                          Rust Core\r\n                          ├── 情感引擎 (PAD 3D)\r\n                          ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                          ├── 人格防御 (PersonaGuard)\r\n                          ├── 行为规则 (RuleEngine)\r\n                          ├── 回放管道 (ReplayPipeline)\r\n                          └── 罐装知识 (CannedManager)\r\n                               │\r\n                          共享内存 (<100μs)\r\n                               │\r\n                          Unity / Live2D / VR 渲染\r\n      ```\r\n\r\n      ## 记忆系统\r\n\r\n      八层管线，全模块 sled 持久化：\r\n      1. **STM** — 环形缓冲区，最近 100 条消息\r\n      2. **LTM** — sled 持久化，无锁读取\r\n      3. **FTS5** — 全文搜索索引\r\n      4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n      5. **Evidence** — 五维证据评分\r\n      6. **Reflection** — 每 8 条消息触发一次反思\r\n      7. **Persona** — 运行时 Trait 固化\r\n      8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n      ## 情感引擎\r\n\r\n      基于 PAD 3D 模型：\r\n      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n      - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n      ## 数据目录\r\n\r\n      所有持久化数据存储在 `~/.atrium/`：\r\n      ```\r\n      ~/.atrium/\r\n      ├── canned/          ← 罐装知识 (*.ack)\r\n      ├── data/            ← sled 持久化数据\r\n      ├── persona/         ← 角色卡\r\n      └── logs/            ← 审计日志\r\n      ```\r\n    ---\r\n\r\n    # Atrium 框架架构\r\n\r\n    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n    ## 核心模块\r\n\r\n    | 模块 | 语言 | 职责 |\r\n    |------|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  |------|------|\r\n      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n      ## 通信架构\r\n\r\n      ```\r\n      用户 → HTTP :8080 → Gateway (Python)\r\n                               │\r\n                          gRPC :50051\r\n                               │\r\n                          Rust Core\r\n                          ├── 情感引擎 (PAD 3D)\r\n                          ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                          ├── 人格防御 (PersonaGuard)\r\n                          ├── 行为规则 (RuleEngine)\r\n                          ├── 回放管道 (ReplayPipeline)\r\n                          └── 罐装知识 (CannedManager)\r\n                               │\r\n                          共享内存 (<100μs)\r\n                               │\r\n                          Unity / Live2D / VR 渲染\r\n      ```\r\n\r\n      ## 记忆系统\r\n\r\n      八层管线，全模块 sled 持久化：\r\n      1. **STM** — 环形缓冲区，最近 100 条消息\r\n      2. **LTM** — sled 持久化，无锁读取\r\n      3. **FTS5** — 全文搜索索引\r\n      4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n      5. **Evidence** — 五维证据评分\r\n      6. **Reflection** — 每 8 条消息触发一次反思\r\n      7. **Persona** — 运行时 Trait 固化\r\n      8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n      ## 情感引擎\r\n\r\n      基于 PAD 3D 模型：\r\n      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n      - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n      ## 数据目录\r\n\r\n      所有持久化数据存储在 `~/.atrium/`：\r\n      ```\r\n      ~/.atrium/\r\n      ├── canned/          ← 罐装知识 (*.ack)\r\n      ├── data/            ← sled 持久化数据\r\n      ├── persona/         ← 角色卡\r\n      └── logs/            ← 审计日志\r\n      ```\r\n    ---\r\n\r\n    # Atrium 框架架构\r\n\r\n    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n    ## 核心模块\r\n\r\n    | 模块 | 语言 | 职责 |\r\n    |------|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  ---|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  # Atrium 框架架构\r\n\r\n  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n  ## 核心模块\r\n\r\n  | 模块 | 语言 | 职责 |\r\n  |------|------|------|\r\n  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n  ## 通信架构\r\n\r\n  ```\r\n  用户 → HTTP :8080 → Gateway (Python)\r\n                           │\r\n                      gRPC :50051\r\n                           │\r\n                      Rust Core\r\n                      ├── 情感引擎 (PAD 3D)\r\n                      ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                      ├── 人格防御 (PersonaGuard)\r\n                      ├── 行为规则 (RuleEngine)\r\n                      ├── 回放管道 (ReplayPipeline)\r\n                      └── 罐装知识 (CannedManager)\r\n                           │\r\n                      共享内存 (<100μs)\r\n                           │\r\n                      Unity / Live2D / VR 渲染\r\n  ```\r\n\r\n  ## 记忆系统\r\n\r\n  八层管线，全模块 sled 持久化：\r\n  1. **STM** — 环形缓冲区，最近 100 条消息\r\n  2. **LTM** — sled 持久化，无锁读取\r\n  3. **FTS5** — 全文搜索索引\r\n  4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n  5. **Evidence** — 五维证据评分\r\n  6. **Reflection** — 每 8 条消息触发一次反思\r\n  7. **Persona** — 运行时 Trait 固化\r\n  8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n  ## 情感引擎\r\n\r\n  基于 PAD 3D 模型：\r\n  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n  - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n  ## 数据目录\r\n\r\n  所有持久化数据存储在 `~/.atrium/`：\r\n  ```\r\n  ~/.atrium/\r\n  ├── canned/          ← 罐装知识 (*.ack)\r\n  ├── data/            ← sled 持久化数据\r\n  ├── persona/         ← 角色卡\r\n  └── logs/            ← 审计日志\r\n  ```\r\n---\r\n\r\n---|------|\r\n      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n      ## 通信架构\r\n\r\n      ```\r\n      用户 → HTTP :8080 → Gateway (Python)\r\n                               │\r\n                          gRPC :50051\r\n                               │\r\n                          Rust Core\r\n                          ├── 情感引擎 (PAD 3D)\r\n                          ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                          ├── 人格防御 (PersonaGuard)\r\n                          ├── 行为规则 (RuleEngine)\r\n                          ├── 回放管道 (ReplayPipeline)\r\n                          └── 罐装知识 (CannedManager)\r\n                               │\r\n                          共享内存 (<100μs)\r\n                               │\r\n                          Unity / Live2D / VR 渲染\r\n      ```\r\n\r\n      ## 记忆系统\r\n\r\n      八层管线，全模块 sled 持久化：\r\n      1. **STM** — 环形缓冲区，最近 100 条消息\r\n      2. **LTM** — sled 持久化，无锁读取\r\n      3. **FTS5** — 全文搜索索引\r\n      4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n      5. **Evidence** — 五维证据评分\r\n      6. **Reflection** — 每 8 条消息触发一次反思\r\n      7. **Persona** — 运行时 Trait 固化\r\n      8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n      ## 情感引擎\r\n\r\n      基于 PAD 3D 模型：\r\n      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n      - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n      ## 数据目录\r\n\r\n      所有持久化数据存储在 `~/.atrium/`：\r\n      ```\r\n      ~/.atrium/\r\n      ├── canned/          ← 罐装知识 (*.ack)\r\n      ├── data/            ← sled 持久化数据\r\n      ├── persona/         ← 角色卡\r\n      └── logs/            ← 审计日志\r\n      ```\r\n    ---\r\n\r\n    # Atrium 框架架构\r\n\r\n    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n    ## 核心模块\r\n\r\n    | 模块 | 语言 | 职责 |\r\n    |------|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  ---|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  # Atrium 框架架构\r\n\r\n  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n  ## 核心模块\r\n\r\n  | 模块 | 语言 | 职责 |\r\n  |------|------|------|\r\n  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n  ## 通信架构\r\n\r\n  ```\r\n  用户 → HTTP :8080 → Gateway (Python)\r\n                           │\r\n                      gRPC :50051\r\n                           │\r\n                      Rust Core\r\n                      ├── 情感引擎 (PAD 3D)\r\n                      ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                      ├── 人格防御 (PersonaGuard)\r\n                      ├── 行为规则 (RuleEngine)\r\n                      ├── 回放管道 (ReplayPipeline)\r\n                      └── 罐装知识 (CannedManager)\r\n                           │\r\n                      共享内存 (<100μs)\r\n                           │\r\n                      Unity / Live2D / VR 渲染\r\n  ```\r\n\r\n  ## 记忆系统\r\n\r\n  八层管线，全模块 sled 持久化：\r\n  1. **STM** — 环形缓冲区，最近 100 条消息\r\n  2. **LTM** — sled 持久化，无锁读取\r\n  3. **FTS5** — 全文搜索索引\r\n  4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n  5. **Evidence** — 五维证据评分\r\n  6. **Reflection** — 每 8 条消息触发一次反思\r\n  7. **Persona** — 运行时 Trait 固化\r\n  8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n  ## 情感引擎\r\n\r\n  基于 PAD 3D 模型：\r\n  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n  - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n  ## 数据目录\r\n\r\n  所有持久化数据存储在 `~/.atrium/`：\r\n  ```\r\n  ~/.atrium/\r\n  ├── canned/          ← 罐装知识 (*.ack)\r\n  ├── data/            ← sled 持久化数据\r\n  ├── persona/         ← 角色卡\r\n  └── logs/            ← 审计日志\r\n  ```\r\n---\r\n\r\n|------|------|\r\n    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n    ## 通信架构\r\n\r\n    ```\r\n    用户 → HTTP :8080 → Gateway (Python)\r\n                             │\r\n                        gRPC :50051\r\n                             │\r\n                        Rust Core\r\n                        ├── 情感引擎 (PAD 3D)\r\n                        ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                        ├── 人格防御 (PersonaGuard)\r\n                        ├── 行为规则 (RuleEngine)\r\n                        ├── 回放管道 (ReplayPipeline)\r\n                        └── 罐装知识 (CannedManager)\r\n                             │\r\n                        共享内存 (<100μs)\r\n                             │\r\n                        Unity / Live2D / VR 渲染\r\n    ```\r\n\r\n    ## 记忆系统\r\n\r\n    八层管线，全模块 sled 持久化：\r\n    1. **STM** — 环形缓冲区，最近 100 条消息\r\n    2. **LTM** — sled 持久化，无锁读取\r\n    3. **FTS5** — 全文搜索索引\r\n    4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n    5. **Evidence** — 五维证据评分\r\n    6. **Reflection** — 每 8 条消息触发一次反思\r\n    7. **Persona** — 运行时 Trait 固化\r\n    8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n    ## 情感引擎\r\n\r\n    基于 PAD 3D 模型：\r\n    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n    - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n    ## 数据目录\r\n\r\n    所有持久化数据存储在 `~/.atrium/`：\r\n    ```\r\n    ~/.atrium/\r\n    ├── canned/          ← 罐装知识 (*.ack)\r\n    ├── data/            ← sled 持久化数据\r\n    ├── persona/         ← 角色卡\r\n    └── logs/            ← 审计日志\r\n    ```\r\n  ---\r\n\r\n  # Atrium 框架架构\r\n\r\n  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n  ## 核心模块\r\n\r\n  | 模块 | 语言 | 职责 |\r\n  |------|------|------|\r\n  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n  ## 通信架构\r\n\r\n  ```\r\n  用户 → HTTP :8080 → Gateway (Python)\r\n                           │\r\n                      gRPC :50051\r\n                           │\r\n                      Rust Core\r\n                      ├── 情感引擎 (PAD 3D)\r\n                      ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                      ├── 人格防御 (PersonaGuard)\r\n                      ├── 行为规则 (RuleEngine)\r\n                      ├── 回放管道 (ReplayPipeline)\r\n                      └── 罐装知识 (CannedManager)\r\n                           │\r\n                      共享内存 (<100μs)\r\n                           │\r\n                      Unity / Live2D / VR 渲染\r\n  ```\r\n\r\n  ## 记忆系统\r\n\r\n  八层管线，全模块 sled 持久化：\r\n  1. **STM** — 环形缓冲区，最近 100 条消息\r\n  2. **LTM** — sled 持久化，无锁读取\r\n  3. **FTS5** — 全文搜索索引\r\n  4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n  5. **Evidence** — 五维证据评分\r\n  6. **Reflection** — 每 8 条消息触发一次反思\r\n  7. **Persona** — 运行时 Trait 固化\r\n  8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n  ## 情感引擎\r\n\r\n  基于 PAD 3D 模型：\r\n  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n  - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n  ## 数据目录\r\n\r\n  所有持久化数据存储在 `~/.atrium/`：\r\n  ```\r\n  ~/.atrium/\r\n  ├── canned/          ← 罐装知识 (*.ack)\r\n  ├── data/            ← sled 持久化数据\r\n  ├── persona/         ← 角色卡\r\n  └── logs/            ← 审计日志\r\n  ```\r\n---\r\n\r\n---|------|------|\r\n  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n  ## 通信架构\r\n\r\n  ```\r\n  用户 → HTTP :8080 → Gateway (Python)\r\n                           │\r\n                      gRPC :50051\r\n                           │\r\n                      Rust Core\r\n                      ├── 情感引擎 (PAD 3D)\r\n                      ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                      ├── 人格防御 (PersonaGuard)\r\n                      ├── 行为规则 (RuleEngine)\r\n                      ├── 回放管道 (ReplayPipeline)\r\n                      └── 罐装知识 (CannedManager)\r\n                           │\r\n                      共享内存 (<100μs)\r\n                           │\r\n                      Unity / Live2D / VR 渲染\r\n  ```\r\n\r\n  ## 记忆系统\r\n\r\n  八层管线，全模块 sled 持久化：\r\n  1. **STM** — 环形缓冲区，最近 100 条消息\r\n  2. **LTM** — sled 持久化，无锁读取\r\n  3. **FTS5** — 全文搜索索引\r\n  4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n  5. **Evidence** — 五维证据评分\r\n  6. **Reflection** — 每 8 条消息触发一次反思\r\n  7. **Persona** — 运行时 Trait 固化\r\n  8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n  ## 情感引擎\r\n\r\n  基于 PAD 3D 模型：\r\n  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n  - **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n  ## 数据目录\r\n\r\n  所有持久化数据存储在 `~/.atrium/`：\r\n  ```\r\n  ~/.atrium/\r\n  ├── canned/          ← 罐装知识 (*.ack)\r\n  ├── data/            ← sled 持久化数据\r\n  ├── persona/         ← 角色卡\r\n  └── logs/            ← 审计日志\r\n  ```\r\n---\r\n\r\n# Atrium 框架架构\r\n\r\nAtrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。\r\n\r\n## 核心模块\r\n\r\n| 模块 | 语言 | 职责 |\r\n|------|------|------|\r\n| `atrium-core` | Rust | 主调度器、事件循环、CoreService |\r\n| `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |\r\n| `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |\r\n| `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |\r\n| `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |\r\n| `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |\r\n| `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |\r\n\r\n## 通信架构\r\n\r\n```\r\n用户 → HTTP :8080 → Gateway (Python)\r\n                         │\r\n                    gRPC :50051\r\n                         │\r\n                    Rust Core\r\n                    ├── 情感引擎 (PAD 3D)\r\n                    ├── 记忆系统 (STM→LTM→FTS5→...)\r\n                    ├── 人格防御 (PersonaGuard)\r\n                    ├── 行为规则 (RuleEngine)\r\n                    ├── 回放管道 (ReplayPipeline)\r\n                    └── 罐装知识 (CannedManager)\r\n                         │\r\n                    共享内存 (<100μs)\r\n                         │\r\n                    Unity / Live2D / VR 渲染\r\n```\r\n\r\n## 记忆系统\r\n\r\n八层管线，全模块 sled 持久化：\r\n1. **STM** — 环形缓冲区，最近 100 条消息\r\n2. **LTM** — sled 持久化，无锁读取\r\n3. **FTS5** — 全文搜索索引\r\n4. **FactStore** — 结构化事实存储（SPO 三元组）\r\n5. **Evidence** — 五维证据评分\r\n6. **Reflection** — 每 8 条消息触发一次反思\r\n7. **Persona** — 运行时 Trait 固化\r\n8. **KeyFact** — 高置信度关键事实缓存\r\n\r\n## 情感引擎\r\n\r\n基于 PAD 3D 模型：\r\n- **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪\r\n- **Arousal（唤醒度）** [-1, 1] — 兴奋/平静\r\n- **Dominance（支配度）** [-1, 1] — 控制/顺从\r\n\r\n9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性\r\n\r\n## 数据目录\r\n\r\n所有持久化数据存储在 `~/.atrium/`：\r\n```\r\n~/.atrium/\r\n├── canned/          ← 罐装知识 (*.ack)\r\n├── data/            ← sled 持久化数据\r\n├── persona/         ← 角色卡\r\n└── logs/            ← 审计日志\r\n```"
---

|------|
        | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
        | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
        | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
        | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
        | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
        | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
        | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

        ## 通信架构

        ```
        用户 → HTTP :8080 → Gateway (Python)
                                 │
                            gRPC :50051
                                 │
                            Rust Core
                            ├── 情感引擎 (PAD 3D)
                            ├── 记忆系统 (STM→LTM→FTS5→...)
                            ├── 人格防御 (PersonaGuard)
                            ├── 行为规则 (RuleEngine)
                            ├── 回放管道 (ReplayPipeline)
                            └── 罐装知识 (CannedManager)
                                 │
                            共享内存 (<100μs)
                                 │
                            Unity / Live2D / VR 渲染
        ```

        ## 记忆系统

        八层管线，全模块 sled 持久化：
        1. **STM** — 环形缓冲区，最近 100 条消息
        2. **LTM** — sled 持久化，无锁读取
        3. **FTS5** — 全文搜索索引
        4. **FactStore** — 结构化事实存储（SPO 三元组）
        5. **Evidence** — 五维证据评分
        6. **Reflection** — 每 8 条消息触发一次反思
        7. **Persona** — 运行时 Trait 固化
        8. **KeyFact** — 高置信度关键事实缓存

        ## 情感引擎

        基于 PAD 3D 模型：
        - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
        - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
        - **Dominance（支配度）** [-1, 1] — 控制/顺从

        9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

        ## 数据目录

        所有持久化数据存储在 `~/.atrium/`：
        ```
        ~/.atrium/
        ├── canned/          ← 罐装知识 (*.ack)
        ├── data/            ← sled 持久化数据
        ├── persona/         ← 角色卡
        └── logs/            ← 审计日志
        ```
      ---

      # Atrium 框架架构

      Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

      ## 核心模块

      | 模块 | 语言 | 职责 |
      |------|------|------|
      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

      ## 通信架构

      ```
      用户 → HTTP :8080 → Gateway (Python)
                               │
                          gRPC :50051
                               │
                          Rust Core
                          ├── 情感引擎 (PAD 3D)
                          ├── 记忆系统 (STM→LTM→FTS5→...)
                          ├── 人格防御 (PersonaGuard)
                          ├── 行为规则 (RuleEngine)
                          ├── 回放管道 (ReplayPipeline)
                          └── 罐装知识 (CannedManager)
                               │
                          共享内存 (<100μs)
                               │
                          Unity / Live2D / VR 渲染
      ```

      ## 记忆系统

      八层管线，全模块 sled 持久化：
      1. **STM** — 环形缓冲区，最近 100 条消息
      2. **LTM** — sled 持久化，无锁读取
      3. **FTS5** — 全文搜索索引
      4. **FactStore** — 结构化事实存储（SPO 三元组）
      5. **Evidence** — 五维证据评分
      6. **Reflection** — 每 8 条消息触发一次反思
      7. **Persona** — 运行时 Trait 固化
      8. **KeyFact** — 高置信度关键事实缓存

      ## 情感引擎

      基于 PAD 3D 模型：
      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
      - **Dominance（支配度）** [-1, 1] — 控制/顺从

      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

      ## 数据目录

      所有持久化数据存储在 `~/.atrium/`：
      ```
      ~/.atrium/
      ├── canned/          ← 罐装知识 (*.ack)
      ├── data/            ← sled 持久化数据
      ├── persona/         ← 角色卡
      └── logs/            ← 审计日志
      ```
    ---

    ---|------|------|
      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

      ## 通信架构

      ```
      用户 → HTTP :8080 → Gateway (Python)
                               │
                          gRPC :50051
                               │
                          Rust Core
                          ├── 情感引擎 (PAD 3D)
                          ├── 记忆系统 (STM→LTM→FTS5→...)
                          ├── 人格防御 (PersonaGuard)
                          ├── 行为规则 (RuleEngine)
                          ├── 回放管道 (ReplayPipeline)
                          └── 罐装知识 (CannedManager)
                               │
                          共享内存 (<100μs)
                               │
                          Unity / Live2D / VR 渲染
      ```

      ## 记忆系统

      八层管线，全模块 sled 持久化：
      1. **STM** — 环形缓冲区，最近 100 条消息
      2. **LTM** — sled 持久化，无锁读取
      3. **FTS5** — 全文搜索索引
      4. **FactStore** — 结构化事实存储（SPO 三元组）
      5. **Evidence** — 五维证据评分
      6. **Reflection** — 每 8 条消息触发一次反思
      7. **Persona** — 运行时 Trait 固化
      8. **KeyFact** — 高置信度关键事实缓存

      ## 情感引擎

      基于 PAD 3D 模型：
      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
      - **Dominance（支配度）** [-1, 1] — 控制/顺从

      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

      ## 数据目录

      所有持久化数据存储在 `~/.atrium/`：
      ```
      ~/.atrium/
      ├── canned/          ← 罐装知识 (*.ack)
      ├── data/            ← sled 持久化数据
      ├── persona/         ← 角色卡
      └── logs/            ← 审计日志
      ```
    ---

    # Atrium 框架架构

    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

    ## 核心模块

    | 模块 | 语言 | 职责 |
    |------|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  |------|------|
      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

      ## 通信架构

      ```
      用户 → HTTP :8080 → Gateway (Python)
                               │
                          gRPC :50051
                               │
                          Rust Core
                          ├── 情感引擎 (PAD 3D)
                          ├── 记忆系统 (STM→LTM→FTS5→...)
                          ├── 人格防御 (PersonaGuard)
                          ├── 行为规则 (RuleEngine)
                          ├── 回放管道 (ReplayPipeline)
                          └── 罐装知识 (CannedManager)
                               │
                          共享内存 (<100μs)
                               │
                          Unity / Live2D / VR 渲染
      ```

      ## 记忆系统

      八层管线，全模块 sled 持久化：
      1. **STM** — 环形缓冲区，最近 100 条消息
      2. **LTM** — sled 持久化，无锁读取
      3. **FTS5** — 全文搜索索引
      4. **FactStore** — 结构化事实存储（SPO 三元组）
      5. **Evidence** — 五维证据评分
      6. **Reflection** — 每 8 条消息触发一次反思
      7. **Persona** — 运行时 Trait 固化
      8. **KeyFact** — 高置信度关键事实缓存

      ## 情感引擎

      基于 PAD 3D 模型：
      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
      - **Dominance（支配度）** [-1, 1] — 控制/顺从

      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

      ## 数据目录

      所有持久化数据存储在 `~/.atrium/`：
      ```
      ~/.atrium/
      ├── canned/          ← 罐装知识 (*.ack)
      ├── data/            ← sled 持久化数据
      ├── persona/         ← 角色卡
      └── logs/            ← 审计日志
      ```
    ---

    # Atrium 框架架构

    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

    ## 核心模块

    | 模块 | 语言 | 职责 |
    |------|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  ---|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  # Atrium 框架架构

  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

  ## 核心模块

  | 模块 | 语言 | 职责 |
  |------|------|------|
  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

  ## 通信架构

  ```
  用户 → HTTP :8080 → Gateway (Python)
                           │
                      gRPC :50051
                           │
                      Rust Core
                      ├── 情感引擎 (PAD 3D)
                      ├── 记忆系统 (STM→LTM→FTS5→...)
                      ├── 人格防御 (PersonaGuard)
                      ├── 行为规则 (RuleEngine)
                      ├── 回放管道 (ReplayPipeline)
                      └── 罐装知识 (CannedManager)
                           │
                      共享内存 (<100μs)
                           │
                      Unity / Live2D / VR 渲染
  ```

  ## 记忆系统

  八层管线，全模块 sled 持久化：
  1. **STM** — 环形缓冲区，最近 100 条消息
  2. **LTM** — sled 持久化，无锁读取
  3. **FTS5** — 全文搜索索引
  4. **FactStore** — 结构化事实存储（SPO 三元组）
  5. **Evidence** — 五维证据评分
  6. **Reflection** — 每 8 条消息触发一次反思
  7. **Persona** — 运行时 Trait 固化
  8. **KeyFact** — 高置信度关键事实缓存

  ## 情感引擎

  基于 PAD 3D 模型：
  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
  - **Dominance（支配度）** [-1, 1] — 控制/顺从

  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

  ## 数据目录

  所有持久化数据存储在 `~/.atrium/`：
  ```
  ~/.atrium/
  ├── canned/          ← 罐装知识 (*.ack)
  ├── data/            ← sled 持久化数据
  ├── persona/         ← 角色卡
  └── logs/            ← 审计日志
  ```
---

---|------|
      | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
      | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
      | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
      | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
      | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
      | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
      | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

      ## 通信架构

      ```
      用户 → HTTP :8080 → Gateway (Python)
                               │
                          gRPC :50051
                               │
                          Rust Core
                          ├── 情感引擎 (PAD 3D)
                          ├── 记忆系统 (STM→LTM→FTS5→...)
                          ├── 人格防御 (PersonaGuard)
                          ├── 行为规则 (RuleEngine)
                          ├── 回放管道 (ReplayPipeline)
                          └── 罐装知识 (CannedManager)
                               │
                          共享内存 (<100μs)
                               │
                          Unity / Live2D / VR 渲染
      ```

      ## 记忆系统

      八层管线，全模块 sled 持久化：
      1. **STM** — 环形缓冲区，最近 100 条消息
      2. **LTM** — sled 持久化，无锁读取
      3. **FTS5** — 全文搜索索引
      4. **FactStore** — 结构化事实存储（SPO 三元组）
      5. **Evidence** — 五维证据评分
      6. **Reflection** — 每 8 条消息触发一次反思
      7. **Persona** — 运行时 Trait 固化
      8. **KeyFact** — 高置信度关键事实缓存

      ## 情感引擎

      基于 PAD 3D 模型：
      - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
      - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
      - **Dominance（支配度）** [-1, 1] — 控制/顺从

      9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

      ## 数据目录

      所有持久化数据存储在 `~/.atrium/`：
      ```
      ~/.atrium/
      ├── canned/          ← 罐装知识 (*.ack)
      ├── data/            ← sled 持久化数据
      ├── persona/         ← 角色卡
      └── logs/            ← 审计日志
      ```
    ---

    # Atrium 框架架构

    Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

    ## 核心模块

    | 模块 | 语言 | 职责 |
    |------|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  ---|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  # Atrium 框架架构

  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

  ## 核心模块

  | 模块 | 语言 | 职责 |
  |------|------|------|
  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

  ## 通信架构

  ```
  用户 → HTTP :8080 → Gateway (Python)
                           │
                      gRPC :50051
                           │
                      Rust Core
                      ├── 情感引擎 (PAD 3D)
                      ├── 记忆系统 (STM→LTM→FTS5→...)
                      ├── 人格防御 (PersonaGuard)
                      ├── 行为规则 (RuleEngine)
                      ├── 回放管道 (ReplayPipeline)
                      └── 罐装知识 (CannedManager)
                           │
                      共享内存 (<100μs)
                           │
                      Unity / Live2D / VR 渲染
  ```

  ## 记忆系统

  八层管线，全模块 sled 持久化：
  1. **STM** — 环形缓冲区，最近 100 条消息
  2. **LTM** — sled 持久化，无锁读取
  3. **FTS5** — 全文搜索索引
  4. **FactStore** — 结构化事实存储（SPO 三元组）
  5. **Evidence** — 五维证据评分
  6. **Reflection** — 每 8 条消息触发一次反思
  7. **Persona** — 运行时 Trait 固化
  8. **KeyFact** — 高置信度关键事实缓存

  ## 情感引擎

  基于 PAD 3D 模型：
  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
  - **Dominance（支配度）** [-1, 1] — 控制/顺从

  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

  ## 数据目录

  所有持久化数据存储在 `~/.atrium/`：
  ```
  ~/.atrium/
  ├── canned/          ← 罐装知识 (*.ack)
  ├── data/            ← sled 持久化数据
  ├── persona/         ← 角色卡
  └── logs/            ← 审计日志
  ```
---

|------|------|
    | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
    | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
    | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
    | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
    | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
    | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
    | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

    ## 通信架构

    ```
    用户 → HTTP :8080 → Gateway (Python)
                             │
                        gRPC :50051
                             │
                        Rust Core
                        ├── 情感引擎 (PAD 3D)
                        ├── 记忆系统 (STM→LTM→FTS5→...)
                        ├── 人格防御 (PersonaGuard)
                        ├── 行为规则 (RuleEngine)
                        ├── 回放管道 (ReplayPipeline)
                        └── 罐装知识 (CannedManager)
                             │
                        共享内存 (<100μs)
                             │
                        Unity / Live2D / VR 渲染
    ```

    ## 记忆系统

    八层管线，全模块 sled 持久化：
    1. **STM** — 环形缓冲区，最近 100 条消息
    2. **LTM** — sled 持久化，无锁读取
    3. **FTS5** — 全文搜索索引
    4. **FactStore** — 结构化事实存储（SPO 三元组）
    5. **Evidence** — 五维证据评分
    6. **Reflection** — 每 8 条消息触发一次反思
    7. **Persona** — 运行时 Trait 固化
    8. **KeyFact** — 高置信度关键事实缓存

    ## 情感引擎

    基于 PAD 3D 模型：
    - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
    - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
    - **Dominance（支配度）** [-1, 1] — 控制/顺从

    9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

    ## 数据目录

    所有持久化数据存储在 `~/.atrium/`：
    ```
    ~/.atrium/
    ├── canned/          ← 罐装知识 (*.ack)
    ├── data/            ← sled 持久化数据
    ├── persona/         ← 角色卡
    └── logs/            ← 审计日志
    ```
  ---

  # Atrium 框架架构

  Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

  ## 核心模块

  | 模块 | 语言 | 职责 |
  |------|------|------|
  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

  ## 通信架构

  ```
  用户 → HTTP :8080 → Gateway (Python)
                           │
                      gRPC :50051
                           │
                      Rust Core
                      ├── 情感引擎 (PAD 3D)
                      ├── 记忆系统 (STM→LTM→FTS5→...)
                      ├── 人格防御 (PersonaGuard)
                      ├── 行为规则 (RuleEngine)
                      ├── 回放管道 (ReplayPipeline)
                      └── 罐装知识 (CannedManager)
                           │
                      共享内存 (<100μs)
                           │
                      Unity / Live2D / VR 渲染
  ```

  ## 记忆系统

  八层管线，全模块 sled 持久化：
  1. **STM** — 环形缓冲区，最近 100 条消息
  2. **LTM** — sled 持久化，无锁读取
  3. **FTS5** — 全文搜索索引
  4. **FactStore** — 结构化事实存储（SPO 三元组）
  5. **Evidence** — 五维证据评分
  6. **Reflection** — 每 8 条消息触发一次反思
  7. **Persona** — 运行时 Trait 固化
  8. **KeyFact** — 高置信度关键事实缓存

  ## 情感引擎

  基于 PAD 3D 模型：
  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
  - **Dominance（支配度）** [-1, 1] — 控制/顺从

  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

  ## 数据目录

  所有持久化数据存储在 `~/.atrium/`：
  ```
  ~/.atrium/
  ├── canned/          ← 罐装知识 (*.ack)
  ├── data/            ← sled 持久化数据
  ├── persona/         ← 角色卡
  └── logs/            ← 审计日志
  ```
---

---|------|------|
  | `atrium-core` | Rust | 主调度器、事件循环、CoreService |
  | `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
  | `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
  | `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
  | `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
  | `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
  | `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

  ## 通信架构

  ```
  用户 → HTTP :8080 → Gateway (Python)
                           │
                      gRPC :50051
                           │
                      Rust Core
                      ├── 情感引擎 (PAD 3D)
                      ├── 记忆系统 (STM→LTM→FTS5→...)
                      ├── 人格防御 (PersonaGuard)
                      ├── 行为规则 (RuleEngine)
                      ├── 回放管道 (ReplayPipeline)
                      └── 罐装知识 (CannedManager)
                           │
                      共享内存 (<100μs)
                           │
                      Unity / Live2D / VR 渲染
  ```

  ## 记忆系统

  八层管线，全模块 sled 持久化：
  1. **STM** — 环形缓冲区，最近 100 条消息
  2. **LTM** — sled 持久化，无锁读取
  3. **FTS5** — 全文搜索索引
  4. **FactStore** — 结构化事实存储（SPO 三元组）
  5. **Evidence** — 五维证据评分
  6. **Reflection** — 每 8 条消息触发一次反思
  7. **Persona** — 运行时 Trait 固化
  8. **KeyFact** — 高置信度关键事实缓存

  ## 情感引擎

  基于 PAD 3D 模型：
  - **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
  - **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
  - **Dominance（支配度）** [-1, 1] — 控制/顺从

  9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

  ## 数据目录

  所有持久化数据存储在 `~/.atrium/`：
  ```
  ~/.atrium/
  ├── canned/          ← 罐装知识 (*.ack)
  ├── data/            ← sled 持久化数据
  ├── persona/         ← 角色卡
  └── logs/            ← 审计日志
  ```
---

# Atrium 框架架构

Atrium 是一个从零构建的情感 AI 框架，采用 **Rust 核心引擎 + Python 网关** 的异构架构。

## 核心模块

| 模块 | 语言 | 职责 |
|------|------|------|
| `atrium-core` | Rust | 主调度器、事件循环、CoreService |
| `atrium-memory` | Rust | 八层记忆管线（STM→LTM→FTS5→FactStore→Evidence→Reflection→Persona→KeyFact） |
| `atrium-emotion` | Rust | PAD 3D 情感模型（愉悦度/唤醒度/支配度），SIMD 加速 <5ns |
| `atrium-persona` | Rust | 双轨人格（静态 YAML→mmap + 运行时 Trait 固化） |
| `atrium-bridge` | Rust | gRPC 服务器 + 共享内存桥接层 |
| `services/gateway` | Python | FastAPI HTTP 网关（端口 8080） |
| `services/llm-orchestrator` | Python | LLM 编排层（多模型路由、ReAct 推理、流式输出） |

## 通信架构

```
用户 → HTTP :8080 → Gateway (Python)
                         │
                    gRPC :50051
                         │
                    Rust Core
                    ├── 情感引擎 (PAD 3D)
                    ├── 记忆系统 (STM→LTM→FTS5→...)
                    ├── 人格防御 (PersonaGuard)
                    ├── 行为规则 (RuleEngine)
                    ├── 回放管道 (ReplayPipeline)
                    └── 罐装知识 (CannedManager)
                         │
                    共享内存 (<100μs)
                         │
                    Unity / Live2D / VR 渲染
```

## 记忆系统

八层管线，全模块 sled 持久化：
1. **STM** — 环形缓冲区，最近 100 条消息
2. **LTM** — sled 持久化，无锁读取
3. **FTS5** — 全文搜索索引
4. **FactStore** — 结构化事实存储（SPO 三元组）
5. **Evidence** — 五维证据评分
6. **Reflection** — 每 8 条消息触发一次反思
7. **Persona** — 运行时 Trait 固化
8. **KeyFact** — 高置信度关键事实缓存

## 情感引擎

基于 PAD 3D 模型：
- **Pleasure（愉悦度）** [-1, 1] — 正面/负面情绪
- **Arousal（唤醒度）** [-1, 1] — 兴奋/平静
- **Dominance（支配度）** [-1, 1] — 控制/顺从

9 种基本情绪：高兴、悲伤、兴奋、平静、自信、害羞、愤怒、恐惧、中性

## 数据目录

所有持久化数据存储在 `~/.atrium/`：
```
~/.atrium/
├── canned/          ← 罐装知识 (*.ack)
├── data/            ← sled 持久化数据
├── persona/         ← 角色卡
└── logs/            ← 审计日志
```