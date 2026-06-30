// SPDX-License-Identifier: MIT
//! Bridge 错误类型
//! Bridge error types for gRPC and shared memory operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("共享内存错误: {0}")]
    Shm(String),

    #[error("gRPC 服务器错误: {0}")]
    Grpc(String),

    #[error("序列化错误: {0}")]
    Serialize(String),

    #[error("反序列化错误: {0}")]
    Deserialize(String),

    #[error("协议版本不匹配: 期望 {expected}, 收到 {got}")]
    VersionMismatch { expected: u32, got: u32 },

    #[error("缓冲区溢出: {0}")]
    BufferOverflow(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for BridgeError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        BridgeError::Internal(e.to_string())
    }
}
