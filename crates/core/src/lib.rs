// SPDX-License-Identifier: MIT
//! Atrium 核心库 — 调度器、事件循环与模块接口
//! Atrium Core Library — Scheduler, event loop, and module interfaces.

pub mod audit;
pub mod config;
pub mod expression_orchestrator;
pub mod guard;
pub mod llm_client;
pub mod metrics;
pub mod proactive;
pub mod room;
pub mod scheduler;
pub mod service;

use std::time::Duration;
use tracing::info;

pub use config::Config;
pub use scheduler::Scheduler;

/// Atrium 核心实例 — 应用程序入口点
///
/// Atrium core instance — Application entry point.
pub struct Atrium;

impl Atrium {
    /// 初始化所有模块并启动主循环
    ///
    /// Initialize all modules and start the main loop.
    ///
    /// @param config_path  配置文件路径 / Path to the configuration file
    ///
    /// @return 运行结果 / Run result
    pub fn run(config_path: &str) -> anyhow::Result<()> {
        let config = Config::load(config_path)?;

        // 初始化 tracing subscriber（必须在第一次 tracing 调用之前）
        // Initialize tracing subscriber (must happen before any tracing call)
        let log_level = config.log_level.as_deref().unwrap_or("info");
        let json_format = config.observability.log_format == "json";
        crate::metrics::init_tracing(log_level, json_format);

        info!("Atrium 启动, 配置文件: {}", config_path);

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // 启动 Prometheus exporter（独立 tokio 任务，不阻塞主循环）
            // Start Prometheus exporter (separate tokio task, non-blocking)
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

            info!("所有模块已启动，进入主循环");

            loop {
                scheduler.tick().await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
    }
}
