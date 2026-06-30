// SPDX-License-Identifier: MIT
//! atrium-persona — 人格核心
//! atrium-persona — Persona core crate.
//!
//! 角色卡定义、序列化/反序列化、mmap 零开销加载、运行时人格实例。
//! Character card definition, serialization/deserialization, mmap zero-copy loading, runtime persona instances.
//!
//! 设计原则 / Design principles:
//! - 人格文件 → bincode → mmap，零运行时开销 / Persona file → bincode → mmap, zero runtime overhead
//! - 每对话只读一次，之后常驻内存 / Loaded once per conversation, then memory-resident
//! - 人格不随模型更新漂移，由这套代码强制保证 / Persona never drifts with model updates, enforced by this crate

pub mod error;
pub mod loader;
pub mod manager;
pub mod types;

pub use error::PersonaError;
pub use loader::PersonaLoader;
pub use manager::PersonaManager;
pub use types::*;
