// SPDX-License-Identifier: MIT
//! Bridge 类型与 trait 定义
//! Bridge types and trait definitions for external dispatchers.

use crate::error::BridgeError;
use crate::protocol::BridgeEvent;

pub trait BridgeDispatcher: Send + Sync {
    fn dispatch(&self, event: BridgeEvent) -> Result<(), BridgeError>;
    fn snapshot(&self) -> BridgeSnapshot;
}

#[derive(Debug, Clone)]
pub struct BridgeSnapshot {
    pub grpc_connections: u32,
    pub shm_version: u32,
    pub processed_events: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct BridgeStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub shm_writes: u64,
    pub shm_reads: u64,
    pub grpc_calls: u64,
    pub errors: u64,
    pub audio_samples_written: u64,
    pub last_active: std::time::Instant,
}

impl Default for BridgeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeStats {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            shm_writes: 0,
            shm_reads: 0,
            grpc_calls: 0,
            errors: 0,
            audio_samples_written: 0,
            last_active: std::time::Instant::now(),
        }
    }
    pub fn record_send(&mut self) {
        self.messages_sent += 1;
        self.last_active = std::time::Instant::now();
    }
    pub fn record_receive(&mut self) {
        self.messages_received += 1;
        self.last_active = std::time::Instant::now();
    }
    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}
