---
name: ack_guide
title: ACK 罐装知识使用指南
kind: TechnicalKnowledge
version: "1.0.0"
tags: [ack, canned, 罐装知识, knowledge, skill, 教程]
summary: ACK (Atrium Canned Knowledge) 是 Atrium 的知识封装格式，支持用户教授、文件导入和 AI 自学
trigger:
  type: OnKeyword
  keywords: [ack是什么, 罐装知识, 怎么用ack, ack格式, canned knowledge, 怎么教我知识, 记住这个]
depends_on: []
---

# ACK 罐装知识 — 使用指南

ACK（Atrium Canned Knowledge）是 Atrium 的知识封装系统，对标 OpenClaw Skill。

## 核心概念

- **文件格式**：Markdown + YAML front matter，后缀 `.ack`
- **存储位置**：`~/.atrium/canned/*.ack`
- **工作方式**：AI 在对话时自动搜索匹配的 ACK，将知识注入 System Prompt 作为参考

## 创建 ACK 的三种方式

### 方式一：用户教授
直接对 AI 说："记住，XXX 的用法是 YYY"
→ AI 自动生成 `.ack` 文件保存到 `~/.atrium/canned/`

### 方式二：文件导入
将 `.ack` 文件放到 `~/.atrium/canned/` 目录
→ AI 自动检测并加载（热加载）

### 方式三：AI 自学
AI 在空闲时通过回放管道分析历史对话
→ 自动发现有用模式 → 生成 `.ack`

## ACK 文件格式

```markdown
---
name: 机器名（英文，下划线连接）
title: 人类可读标题
kind: TechnicalKnowledge | ToolUsage | BehaviorStrategy | ConversationStrategy | ProtocolConfig
version: "1.0.0"（语义版本）
tags: [标签1, 标签2]
summary: 一句话摘要
trigger:
  type: OnKeyword
  keywords: [触发词1, 触发词2]  ← 用户消息包含这些词时自动激活
depends_on: []  ← 依赖的其他 ACK 名称
---

# 正文标题
Markdown 格式的正文内容，支持所有 Markdown 语法。
```

## 知识类型（kind）

| 类型 | 说明 | 示例 |
|------|------|------|
| `TechnicalKnowledge` | 技术知识 | Rust 语法、API 用法 |
| `ToolUsage` | 工具用法 | gRPC 调试、飞书 API |
| `BehaviorStrategy` | 行为策略 | 深度思考模式、代码审查模式 |
| `ConversationStrategy` | 对话策略 | 客服话术、教学风格 |
| `ProtocolConfig` | 协议配置 | MCP 连接配置 |

## 触发条件（trigger）

| 类型 | 说明 | 示例 |
|------|------|------|
| `OnKeyword` | 关键词匹配（最常用） | 用户提到"飞书"→激活飞书连接配置 |
| `OnIntent` | 意图匹配 | 用户意图为"写代码"→激活编码模式 |
| `Always` | 始终加载 | 基础知识，每次对话都注入 |
| `OnContext` | 上下文匹配 | 特定渠道/设备才激活 |

## 跨 AI 传输

ACK 支持在 AI 之间分享知识：

导出格式：
```
=== Canned Knowledge v1 ===
name: xxx
title: xxx
...
body: |
  ...
=== End Canned Knowledge ===
```

通过 `POST /api/canned/import` 导入，或直接放到 `~/.atrium/canned/` 目录。
