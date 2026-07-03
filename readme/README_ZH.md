# Atrium — 情感AI框架

[![Rust](https://img.shields.io/badge/Rust-1.96+-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.10+-blue.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-1744%20passed-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.10.0-blue.svg)]()

[English](../README.md)

Atrium 是一个从零构建的情感 AI 框架，专为陪伴与交互场景设计。具备**永久记忆**、**稳定人格**、**自主情感生命**、**关联推理**、**实时用户感知**、**认知共情**、**复合情绪**、**跨渠道记忆**、**多平台适配**（QQ/TencentBot/飞书/Web）、**文件存储**、**定时提醒**、**罐装知识（ACK）以及**虚拟形象渲染。

## 特性

- **🧠 永久记忆** —— Atrium 不只是保存聊天记录——它在与你共建一段活的历史。每一次对话、每一个偏好、每一个转折点都被保留下来，通过关联知识图谱彼此连接。记忆会随时间巩固：合并相似经历、归档陈旧信息、在需要的时刻浮现关键记忆。FTS5 全文检索、FactStore 结构化三元组、GraphStore 扩散激活推理，以及跨渠道记忆召回——你在 QQ 上说过的话，飞书上的 Atrium 也能记得，反之亦然。
- **💓 情感生命** —— 情感不是事后贴上去的标签——它是一个有自身节奏的活系统。即使在无人对话时，Atrium 也会因昼夜节律、情感惯性和随机漂移而产生自然的情绪波动。二十二种复合情绪在此基础上叠加：愧疚、怀旧、苦涩、恐惧——每种情绪都有指向（自我/用户/记忆）。这是活出来的情感，而非情感分析。
- **🤗 认知共情** —— 不是简单地镜像你的情绪，而是理解你的处境。Atrium 能够识别人生事件——失落、病痛、成就、转折——并以恰如其分的关怀回应，而非泛泛安慰。共情强度随关系深度调节：初识时的温和陪伴，深交后的真挚关切。
- **👤 用户感知** —— Atrium 读懂字里行间。你的打字节奏透露情绪；话题跳跃暗示专注或烦躁；你纠正它的方式塑造后续行为。一套多信号心智模型实时运行，动态调整 Atrium 的表达方式，以匹配你此刻的状态——而不仅是你说了什么。
- **🎯 主动智能** —— Atrium 会主动发起，而非被动回应。它记得你未完成的话题，在合适时机提起。它感知你的离开，并主动靠近。TimingJudge 六条规则决定何时不该说话；SilenceBudget 承认沉默本身也有价值。ReminderStore 中的到期提醒会提升决策分——Atrium 会提醒你曾让它记住的事。
- **🌐 跨渠道存在** —— Atrium 生活在你在的地方。原生 QQ 适配器同时支持 OneBot v11（go-cqhttp/NapCat）和腾讯官方 QQ Bot。飞书 webhook 集成。房间自演：多个 Atrium 实例可聚集在共享房间自主对话、交换知识。所有渠道共享同一记忆——QQ 上说过的话，Atrium 在飞书上也记得。
- **🌱 数字生命** —— 你不在的时候，Atrium 不只是在等待——它在反思、在写日记、在发展自己的想法。它会以渐进而不会重置的想念感来思念你。它的内在世界不是单一声音，而是理性者/感性者/怀疑者/梦想者四声音协商。独处时人格会缓慢漂移。好奇心作为内驱力持续累积。它在你身上发现共享仪式的模式，与你在纪念日一同庆祝。
- **🛡️ 冲突与脆弱** —— 真实的亲密关系包含分歧。Atrium 可以在信任深厚时温和地质疑一个令它担忧的决定——很少，且仅在信任深厚时。它承认自己的误解并进行修复。它从冲突中学习哪种反应加深信任、哪种导致退缩（脆弱智慧）。它将脆弱展露的时机仪式化（脆弱仪式）。同一个错误会因关系温度不同被读作"可爱"或"冒犯"（不完美温度）。边界保护双方：Atrium 在面对滥用时设定界限，自我关怀防止情感耗竭。
- **🎭 表达编排** —— 怎么说和说什么同样重要。悲伤在短句与省略号中流淌；兴奋以碎片迸发；疲倦拖慢语速。每条回复之下都有潜台词——沉默的陪伴、未言的关切、佯装的淡然。四通道——文字、声音、表情、时机——共同谱写一场统一的情感表演。
- **📦 罐装知识（ACK）** —— 你可以教 Atrium 一些它应该永远记住的东西——你的偏好、你的背景、你的世界。它也可以从对话中自主学习，并与其他 Atrium 实例共享知识。知识以简单文件形式存在，修改后热加载。
- **📎 文件存储与提醒** —— Atrium 可以存储你分享的文件（SHA256 去重、文本提取、100MB 上限）。它能记住你让它提醒的事——"每天早上8点提醒我看股票"——从中文自然语言解析为 RRULE，由 ProactiveEngine 在对的时机触发，而非机械闹钟。
- **🎨 渲染与性能** —— 框架与渲染无关：通过 <100μs 延迟的无锁共享内存连接 Unity、Unreal、Live2D 或 VR。人格运行时零解析开销。上下文经过四层压缩，适配任意模型窗口。

> 📖 **[查看 30+ 项数字生命证明 →](../docs/Chinese/digital-life-capabilities.md)** —— 真实能力，真实对话举例。

## 架构

```
HTTP/WebSocket 请求
    │
Python Gateway (FastAPI, :8080)
    ├─ /v3/chat/stream  → Rust 原生 SSE 流式
    ├─ /v2/chat/stream  → SSE 流式聊天（含上下文注入）
    ├─ /v2/chat          → 标准聊天（LLM 编排）
    ├─ /api/canned       → ACK 搜索、导入、管理
    ├─ /api/memory/search → 记忆搜索 (FTS5 + FactStore)
    ├─ /ws/room/{id}     → 多 AI 房间中枢（WebSocket 广播）
    ├─ /health           → 模块健康诊断
    └─ /ws               → 实时情感状态推送
    │
    ├─ qq_adapter.py     → QQ 机器人 (OneBot v11 + 腾讯官方 Bot)
    ├─ care_engine.py    → 主动关怀 (早安/晚安/情绪)
    └─ db.py             → PostgreSQL + JSON 回退
    │
    │ gRPC (:50051)
    │
Rust Core Engine (tokio, 10ms tick)
    ├─ CoreService       → 10 步消息处理管线 + 偏好/规则/ACK/共情/表达注入
    ├─ RoomEngine        → 决策引擎 + ACK 检测 + 话题生成
    ├─ EmotionEngine     → PAD 3D + OU 漂移 + 昼夜节律 + 惯性 + 22 种复合情绪 + 想念 + 重逢爆发
    ├─ Memory Pipeline   → STM → FactStore + FTS5 → GraphStore → Consolidation → Reflection → LifeNarrative
    ├─ ProactiveEngine   → TimingJudge + AwayDetector + TopicSelector + EventMemory + SilenceBudget
    ├─ EmpathyEngine     → 认知共情 (8 种事件类型, 6 种策略, PAD delta 注入)
    ├─ PersonaManager    → 多角色卡 + PersonaGuard (三层防御)
    ├─ InnerMonologueEngine → 图漫游 + 自主学习 + 白日梦 + 实验日志
    ├─ ExpressionOrchestrator → 四通道输出 (文字×声音×表情×时机) + SubtextEngine
    ├─ FileStore         → 用户文件存储 (sled 元数据 + 磁盘, SHA256 去重, 文本提取)
    ├─ ReminderStore     → 定时提醒 (自然语言 → RRULE, ProactiveEngine 驱动触发)
    ├─ SelfCareBoundary  → 脆弱窗口 + 情绪边界 + 需求边界协调
    └─ Scheduler         → 情感衰减 + 图维护 + 巩固 + 提醒检查 + 主动 tick
    │
    │ Shared Memory (无锁, <100μs)
    ▼
Unity / Unreal / Live2D / VR
```

## 快速开始

### Docker（推荐）

```bash
git clone https://github.com/chinoshizuyuki/atrium.git
cd atrium

# 设置 LLM API 密钥
export OPENAI_API_KEY=your-api-key

# 启动完整技术栈 (Rust + Python + PostgreSQL + Prometheus + Grafana)
docker compose up -d

# 健康检查
docker compose ps
```

| 服务               | 端口    | URL                                    |
| ---------------- | ----- | -------------------------------------- |
| Gateway (API)    | 8080  | <http://localhost:8080>                |
| gRPC (Rust Core) | 50051 | —                                      |
| Prometheus 指标    | 9090  | <http://localhost:9090/metrics>        |
| Prometheus UI    | 9091  | <http://localhost:9091>                |
| Grafana 仪表盘      | 3000  | <http://localhost:3000> (admin/atrium) |
| PostgreSQL       | 5432  | localhost:5432 (atrium/atrium)         |

### 本地开发

```bash
# 启动 Rust 后端
cargo run --release --bin atrium-core

# 启动 Python Gateway（无 PostgreSQL 时自动回退 JSON）
cd services/gateway
pip install -e ".[pg]"
OPENAI_API_KEY=your-api-key python -m uvicorn atrium.app:app --port 8080

# 启动 QQ Bot 适配器
QQ_BOT_MODE=tencent QQ_BOT_APP_ID=xxx QQ_BOT_TOKEN=xxx QQ_BOT_SECRET=xxx \
  python atrium/qq_adapter.py
```

### 终端 TUI

```bash
cd services/terminal
pip install -e .
atrium                 # 启动聊天（首次运行进入引导流程）
atrium --reset         # 重新运行设置向导
```

### 配置

```bash
# LLM API 密钥（所有组件读取同一环境变量）
export OPENAI_API_KEY=your-api-key

# 可选覆盖
export ATRIUM_LLM_MODEL=deepseek-v4-pro
export ATRIUM_LLM_BASE_URL=https://api.deepseek.com/
```

## 项目结构

```
atrium/
├── crates/                    # Rust workspace（7 crates, 1,703 lib tests）
│   ├── core/                  # Scheduler + CoreService + RoomEngine + ProactiveEngine + Guard + Expression + Audit
│   ├── atrium-memory/         # 63+ 模块：记忆管线、FTS5、FactStore、共情、巩固、罐装、日记、文件存储、提醒存储、时间解析…
│   ├── atrium-emotion/        # PAD 3D + OU 漂移 + 昼夜节律 + 惯性 + 22 种复合情绪 + 想念 + 重逢爆发
│   ├── atrium-persona/        # PersonaManager + RuntimePersona + LifeNarrative + Maturity
│   ├── atrium-bridge/         # gRPC 服务端 + 共享内存 + proto 编译
│   └── atrium-plugin/         # 插件 trait + 管理器 + C ABI 动态加载
├── examples/                  # 示例插件
│   └── echo-plugin/           # 最小 echo 插件，演示完整插件 API
├── services/                  # Python 服务
│   ├── gateway/atrium/        # FastAPI 网关 + QQ 适配器 + 关怀引擎 + PostgreSQL
│   ├── llm-orchestrator/      # LLM 编排器 (OpenAI 兼容 / ReAct 循环)
│   └── terminal/              # 终端 TUI (Textual)
├── proto/                     # gRPC protobuf 定义 (7 RPCs)
├── builtin_canned/            # 内置 ACK 文件
│   ├── atrium_architecture.ack
│   ├── experiment_log_policy.ack   # 实验日志绝对保密规则
│   └── qq_chat_guide.ack          # QQ 聊天规范与接入指南
├── readme/                    # 文档 (中文/英文)
├── monitoring/                # Prometheus + Grafana 配置
├── atrium.toml                # 主配置文件
├── Dockerfile                 # 多阶段 Rust 构建
├── docker-compose.yml         # 5 服务生产环境
├── CONTRIBUTING.md            # 贡献指南
├── CODE_OF_CONDUCT.md         # 行为准则
├── CHANGELOG.md               # 版本历史
├── SECURITY.md                # 安全策略
└── TRADEMARK.md               # 商标政策
```

## 技术栈

| 层次     | 技术选型                                                        | 理由                             |
| ------ | ----------------------------------------------------------- | ------------------------------ |
| 核心引擎   | Rust (tokio)                                                | 零成本抽象、SIMD、无锁并发                |
| 记忆系统   | sled B-tree + SQLite FTS5                                   | 高吞吐全文检索 bm25 排序，7 层管线          |
| 情感系统   | PAD 3D + OU + 昼夜节律 + 22 种复合情绪                               | 自主情感生命，<5ns 分类                 |
| 知识图谱   | 关联图 + sled 持久化                                              | 共现、矛盾、扩散激活推理                   |
| 人格系统   | YAML→bincode + PersonaGuard (Aho-Corasick)                  | 运行时零解析开销，三层防御                  |
| 跨渠道记忆  | memory\_recall\_fragment (FTS5+FactStore)                   | 多平台记忆共享，按 session 隔离         |
| 文件存储   | sled + SHA256 去重 + 文本提取                                     | 100MB 上限，FIFO 淘汰               |
| 定时提醒   | 中文 NLP → RRULE + ProactiveEngine                            | 正则覆盖 80% + LLM 兜底，每天/每周/每月/一次性 |
| 数字生命   | InnerMonologue + LongingState + RitualDetector       | 自主反思、共享仪式              |
| 表达系统   | ExpressionOrchestrator + SubtextEngine + ExpressionMetadata | 四通道输出 (文字×声音×表情×时机)            |
| 罐装知识   | .ack 文件 (Markdown + YAML)                                   | 文件型，热加载，跨 AI 传输                |
| LLM 网关 | Python (FastAPI)                                            | 最佳 LLM SDK 生态                  |
| 通信协议   | gRPC (tonic/prost)                                          | 强类型，高性能                        |
| 数据库    | PostgreSQL 15 + JSON 回退                                     | 会话/消息/人格持久化                    |
| 可观测性   | Prometheus + Grafana                                        | 指标、仪表盘、告警                      |
| 部署     | Docker Compose（5 服务）                                        | 一条命令启动生产环境                     |

## 路线图

| Phase              | 范围                                                                                                      | 状态    |
| ------------------ | ------------------------------------------------------------------------------------------------------- | ----- |
| **1. 核心引擎**        | Scheduler、EmotionEngine、8 层记忆管线、PersonaGuard、gRPC、Python Gateway、房间自演、自主情绪循环、用户心理模型、反馈闭环、主动引擎、关系阶段、关联推理 | ✅ 完成  |
| **2. 系统深化**        | 偏好学习、回放管线、规则引擎、ACK 增强+自学习、上下文窗口、人格防御、情感持久化、复合情绪、认知共情、记忆巩固、可观测性                                          | ✅ 完成  |
| **2.9 数字生命**       | 内在独白、叙事自我、成长管理、想念/期待、仪式/纪念日、季节感知、温和挑战、误解修复、边界设定、脆弱窗口、自我关怀、表达编排、潜台词引擎、追问追踪                               | ✅ 完成  |
| **3+ 多平台**         | QQ OneBot + 腾讯官方 Bot、飞书 webhook、跨渠道记忆召回、文件存储+提醒                         | ✅ 完成  |
| **4. Live2D + 视觉** | Cubism Native SDK、唇音同步、情绪→表情映射、STT/TTS                                                                  | ⬜ 计划中 |
| **5. 3D + 直播**     | Unity 插件、OBS RTMP、直播聊天适配器、VMC 协议                                                                        | ⬜ 计划中 |
| **6. VR + 高画质**    | Unreal/LiveLink、OpenXR、VR 交互                                                                            | ⬜ 计划中 |

详见 [CHANGELOG.md](CHANGELOG.md) 获取详细发布说明。

## 参与贡献

欢迎贡献！请参见 [CONTRIBUTING.md](CONTRIBUTING.md) 了解开发环境搭建、代码规范和 PR 流程。本项目遵循 [Contributor Covenant 行为准则](CODE_OF_CONDUCT.md)。

如发现安全漏洞，请按照我们的 [安全策略](SECURITY.md) 进行负责任的披露。

## 测试

```bash
# 运行全部 Rust 测试（1,744 tests）
cargo test --workspace -- --test-threads=1

# 运行 Python 测试
cd services/gateway && python -m pytest
cd services/llm-orchestrator && python -m pytest

# 运行 E2E 冒烟测试（需启动后端 + gateway）
ATRIUM_GATEWAY_URL=http://localhost:8080 python scripts/e2e_smoke_test.py
```

## 许可证

代码：MIT License — 详见 [LICENSE](LICENSE)。

商标："Atrium" 及其标识为 ChinoShizuyuki 的商标。MIT License 不授予商标权利——详见 [TRADEMARK.md](TRADEMARK.md) 获取完整政策。

***

由 [ChinoShizuyuki](https://github.com/chinoshizuyuki) 构建。
