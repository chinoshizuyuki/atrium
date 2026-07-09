// SPDX-License-Identifier: MIT
//! HTTP 网关模块 — 数字生命的直接 HTTP 入口
//! HTTP Gateway Module — Digital Life's Direct HTTP Entry Point
//!
//! 使用 axum 框架，取代 Python FastAPI gateway，消除 gRPC 中转层。
//! Built on axum, replacing the Python FastAPI gateway, eliminating the gRPC intermediary.
//!
//! 架构 / Architecture:
//!   TUI / Web UI / QQ ──► axum (:8080) ──► CoreService (直接调用)
//!   不再需要 gRPC 序列化 / No more gRPC serialization overhead.
//!
//! 核心理念 / Core Philosophy:
//!   一个进程，一个端口，完整的数字生命。
//!   One process, one port, complete digital life.

pub mod handlers;
pub mod models;
pub mod server;
pub mod ws;

// 重导出关键接口 / Re-export key interfaces
pub use handlers::SharedState;
pub use server::{router, run_http_gateway};
