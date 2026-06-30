// SPDX-License-Identifier: MIT
//! atrium-bridge — Rust 核心与外部系统之间的通信桥
//! atrium-bridge — Communication bridge between Rust core and external systems.
//!
//! 提供 gRPC 服务端、共享内存桥接和协议定义。
//! Provides gRPC server, shared memory bridge, and protocol definitions.

pub mod error;
pub mod grpc;
pub mod protocol;
pub mod shm;
pub mod types;

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};

use crate::error::BridgeError;
use crate::protocol::{BridgeConfig, BridgeEvent};
use crate::shm::SharedMemory;

#[allow(dead_code)]
pub struct Bridge {
    config: BridgeConfig,
    event_tx: Option<flume::Sender<BridgeEvent>>,
    event_rx: Option<flume::Receiver<BridgeEvent>>,
    shm: Option<SharedMemory>,
    started_at: Instant,
    processed_events: AtomicU64,
}

impl Bridge {
    pub fn new(config: BridgeConfig) -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            config,
            event_tx: Some(tx),
            event_rx: Some(rx),
            shm: None,
            started_at: Instant::now(),
            processed_events: AtomicU64::new(0),
        }
    }

    pub fn event_sender(&self) -> Option<flume::Sender<BridgeEvent>> {
        self.event_tx.clone()
    }

    pub fn event_receiver(&mut self) -> Option<flume::Receiver<BridgeEvent>> {
        self.event_rx.take()
    }

    pub fn shared_memory(&self) -> Option<&SharedMemory> {
        self.shm.as_ref()
    }
    pub fn shared_memory_mut(&mut self) -> Option<&mut SharedMemory> {
        self.shm.as_mut()
    }

    pub async fn start(
        &mut self,
        service: Arc<dyn crate::grpc::AtriumCoreService>,
    ) -> Result<(), BridgeError> {
        info!("桥接层启动中...");

        // 初始化共享内存
        match SharedMemory::create_or_open(&self.config.shm_path) {
            Ok(shm) => {
                info!("共享内存已就绪: {}", self.config.shm_path);
                self.shm = Some(shm);
            }
            Err(e) => {
                tracing::warn!("共享内存初始化失败（跳过）: {}", e);
            }
        }

        // 启动 gRPC 服务器
        let tx = self.event_tx.clone().expect("event_tx 已被取出");
        let addr = self
            .config
            .grpc_addr
            .clone()
            .replace("/tmp/", "127.0.0.1:")
            .replace(".sock", "50051");
        tokio::spawn(async move {
            if let Err(e) = grpc::start_grpc_server(addr, tx, service).await {
                error!("gRPC 服务器异常退出: {}", e);
            }
        });

        info!("桥接层就绪");
        Ok(())
    }

    pub fn handle_event(&self, _event: BridgeEvent) -> Result<(), BridgeError> {
        self.processed_events
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // 后续通过共享内存同步事件
        Ok(())
    }

    pub fn health(&self) -> types::BridgeSnapshot {
        let shm_version = self
            .shm
            .as_ref()
            .map(|s| {
                s.render_state()
                    .version
                    .load(std::sync::atomic::Ordering::Acquire)
            })
            .unwrap_or(0);

        types::BridgeSnapshot {
            grpc_connections: 0,
            shm_version,
            processed_events: self
                .processed_events
                .load(std::sync::atomic::Ordering::Relaxed),
            is_active: self.shm.is_some(),
        }
    }
}
