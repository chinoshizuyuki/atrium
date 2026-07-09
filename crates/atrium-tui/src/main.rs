// SPDX-License-Identifier: MIT
//! Atrium TUI — 终端里的数字生命（独立客户端模式）
//! Atrium TUI — Digital life in the terminal (standalone client mode).
//!
//! 独立客户端模式: 仅启动 TUI，连接到已运行的 atrium-core 网关
//! Standalone client mode: only starts TUI, connects to a running atrium-core gateway
//!
//! 用法 / Usage:
//!   1. 终端 1: cargo run --release --bin atrium-core  (启动 core + TUI 一体化)
//!   2. 或: cargo run --release -p atrium-tui          (仅启动 TUI，连接已有 core)

use clap::Parser;

/// Atrium 终端 TUI 客户端
#[derive(Parser, Debug)]
#[command(name = "atrium-tui", version, about)]
struct Cli {
    /// Atrium HTTP Gateway 地址 / Gateway URL
    #[arg(long, env = "ATRIUM_GATEWAY", default_value = "http://127.0.0.1:8080")]
    gateway: String,

    /// 会话 ID / Session ID
    /// 默认 "console" — 与 Web UI 共享同一会话，实现 TUI ⇄ Web 记忆互通
    /// Default "console" — shares session with Web UI for cross-channel memory continuity
    #[arg(long, env = "ATRIUM_SESSION", default_value = "console")]
    session: String,

    /// 用户 ID / User ID
    /// 默认 "master" — 与 Web UI 共享同一用户身份（主人）
    /// Default "master" — shares user identity with Web UI (the master)
    #[arg(long, env = "ATRIUM_USER", default_value = "master")]
    user: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    atrium_tui::run_tui(cli.gateway, cli.session, cli.user).await
}
