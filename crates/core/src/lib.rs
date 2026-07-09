// SPDX-License-Identifier: MIT
//! Atrium 核心库 — 调度器、事件循环与模块接口
//! Atrium Core Library — Scheduler, event loop, and module interfaces.
//!
//! 单进程即生命体 — atrium-core 启动后直接显示 TUI 界面，日志重定向到文件。
//! Single-process digital life — atrium-core starts TUI directly, logs redirected to file.

pub mod audit;
pub mod config;
pub mod expression_orchestrator;
pub mod guard;
pub mod http_gateway;
pub mod llm_client;
pub mod metrics;
pub mod proactive;
pub mod room;
pub mod scheduler;
pub mod service;

use futures_util::future::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub use config::Config;
pub use scheduler::Scheduler;

/// Atrium 核心实例 — 应用程序入口点
///
/// Atrium core instance — Application entry point.
pub struct Atrium;

impl Atrium {
    /// 初始化所有模块并启动主循环（TUI 集成模式）
    ///
    /// Initialize all modules and start the main loop (TUI integration mode).
    ///
    /// 启动流程 / Startup flow:
    /// 1. 加载配置 + 初始化 tracing（日志输出到 ~/.atrium/logs/core.log）
    /// 2. 启动 tokio runtime
    /// 3. 后台 spawn: Prometheus exporter + scheduler 主循环（含 panic 自愈）
    /// 4. 等待 HTTP 网关就绪（/health 探测）
    /// 5. 前台进入 TUI 主循环（ratatui + crossterm）
    /// 6. TUI 退出后发送 shutdown signal，scheduler 优雅关闭
    ///
    /// @param config_path  配置文件路径 / Path to the configuration file
    ///
    /// @return 运行结果 / Run result
    pub fn run(config_path: &str) -> anyhow::Result<()> {
        let config = Config::load(config_path)?;

        // 初始化 tracing — 日志输出到文件（TUI 模式下终端留给 ratatui）
        // Initialize tracing — logs to file (terminal reserved for ratatui in TUI mode)
        let log_level = config.log_level.as_deref().unwrap_or("info");
        if let Err(e) = crate::metrics::init_tracing_to_file(log_level) {
            // 文件日志初始化失败 — 回退到终端日志（TUI 可能会被刷屏，但至少有日志）
            // File logging init failed — fall back to terminal logging (TUI may be flooded, but at least we have logs)
            warn!("文件日志初始化失败，回退到终端日志 / File logging init failed, falling back to terminal: {}", e);
            let json_format = config.observability.log_format == "json";
            crate::metrics::init_tracing(log_level, json_format);
        }

        // 初始化 metrics 前缀 / Initialize metrics prefix
        crate::metrics::init_prefix(&config.observability.metrics_prefix);

        info!("Atrium 启动 (TUI 集成模式), 配置文件: {}", config_path);

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // 启动 Prometheus exporter（后台 task）/ Start Prometheus exporter (background task)
            if config.observability.enabled {
                let addr = config.observability.prometheus_addr();
                tokio::spawn(async move {
                    if let Err(e) = crate::metrics::start_prometheus(addr).await {
                        tracing::error!("Prometheus exporter 启动失败: {}", e);
                    }
                });
            }

            let mut scheduler = Scheduler::new(config);
            scheduler.start_all().await;
            info!("所有模块已启动，scheduler 后台运行中");

            // Shutdown signal — TUI 退出后通知 scheduler 主循环退出
            // Shutdown signal — notify scheduler main loop to exit after TUI quits
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_clone = shutdown.clone();

            // 后台 spawn: scheduler 主循环（含 panic 自愈）/ Background spawn: scheduler main loop (with panic self-healing)
            let scheduler_handle = tokio::spawn(async move {
                let mut backoff_secs: u64 = 1;
                let mut panic_count: u32 = 0;
                let mut panic_window_start = std::time::Instant::now();

                while !shutdown_clone.load(Ordering::Relaxed) {
                    let result = AssertUnwindSafe(scheduler.tick()).catch_unwind().await;
                    match result {
                        Ok(()) => {
                            // 正常 tick — 重置退避与计数 / Normal tick — reset backoff and counters
                            backoff_secs = 1;
                            panic_count = 0;
                            panic_window_start = std::time::Instant::now();
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(panic_payload) => {
                            // panic 捕获 — 数字生命自愈 / Panic caught — digital life self-healing
                            panic_count += 1;
                            let payload_str = panic_payload
                                .downcast_ref::<String>()
                                .map(|s| s.as_str())
                                .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                                .unwrap_or("<非字符串 panic payload / non-string panic payload>");

                            tracing::error!(
                                "主循环 panic 捕获 — 数字生命自愈中 / Main loop panic caught — digital life self-healing. \
                                 payload: {}, panic_count: {}, backoff: {}s",
                                payload_str, panic_count, backoff_secs
                            );

                            // 连续 panic 防雪崩 — 30s 内 >5 次则封顶 30s + critical 告警
                            // Anti-avalanche — cap backoff at 30s + critical alert if >5 panics in 30s
                            if panic_window_start.elapsed() > Duration::from_secs(30) {
                                panic_count = 1;
                                panic_window_start = std::time::Instant::now();
                            }
                            if panic_count > 5 {
                                backoff_secs = 30;
                                tracing::error!(
                                    "主循环连续 panic {} 次 — 退避封顶 30s / Main loop consecutive panics {} — backoff capped at 30s",
                                    panic_count, panic_count
                                );
                            }

                            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                            backoff_secs = (backoff_secs * 2).min(30);
                        }
                    }
                }

                // 收到 shutdown signal — 优雅关闭 / Received shutdown signal — graceful shutdown
                info!("scheduler 收到 shutdown signal — 优雅关闭中 / scheduler received shutdown signal — graceful shutdown");
                scheduler.shutdown();
                info!("数字生命已优雅关闭 / Digital life gracefully shut down");
            });

            // 等待 HTTP 网关就绪（最多 10 秒）/ Wait for HTTP gateway to be ready (up to 10s)
            let gateway = "http://127.0.0.1:8080";
            let http_probe = reqwest::Client::new();
            let mut gateway_ready = false;
            for _ in 0..100 {
                if http_probe
                    .get(format!("{}/health", gateway))
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
                {
                    gateway_ready = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if !gateway_ready {
                tracing::warn!("HTTP 网关未就绪 — TUI 可能无法连接 / HTTP gateway not ready — TUI may fail to connect");
            }

            // 前台进入 TUI 主循环 / Enter TUI main loop in foreground
            // 但若设置了 ATRIUM_NO_TUI 环境变量（如自动化测试场景），则跳过 TUI，只运行服务
            // If ATRIUM_NO_TUI env var is set (e.g. automated testing), skip TUI and run service only
            let no_tui = std::env::var("ATRIUM_NO_TUI").is_ok();
            if no_tui {
                info!("ATRIUM_NO_TUI 已设置 — 纯服务模式（无 TUI），等待 Ctrl+C 退出 / ATRIUM_NO_TUI set — service-only mode (no TUI), Ctrl+C to exit");
                // 等待 Ctrl+C 信号 / Wait for Ctrl+C signal
                tokio::signal::ctrl_c().await.ok();
                shutdown.store(true, Ordering::Relaxed);
                let _ = tokio::time::timeout(Duration::from_secs(5), scheduler_handle).await;
                info!("数字生命已优雅关闭 / Digital life gracefully shut down");
                return Ok(());
            }

            info!("启动 TUI 界面 / Starting TUI");
            let tui_result = atrium_tui::run_tui(
                gateway.to_string(),
                "console".to_string(),
                "master".to_string(),
            )
            .await;

            // TUI 退出 — 发送 shutdown signal / TUI exited — send shutdown signal
            shutdown.store(true, Ordering::Relaxed);
            // 等待 scheduler 优雅关闭（最多 5 秒）/ Wait for scheduler graceful shutdown (up to 5s)
            let _ = tokio::time::timeout(Duration::from_secs(5), scheduler_handle).await;

            tui_result
        })
    }
}
