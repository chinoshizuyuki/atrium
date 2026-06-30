// SPDX-License-Identifier: MIT
//! 人格模块错误类型
//! Persona module error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersonaError {
    #[error("角色卡加载失败: {0}")]
    LoadFailed(String),

    #[error("角色卡未找到: {0}")]
    NotFound(String),

    #[error("序列化/反序列化失败: {0}")]
    Serde(String),

    #[error("mmap 映射失败: {0}")]
    Mmap(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<std::io::Error> for PersonaError {
    fn from(e: std::io::Error) -> Self {
        PersonaError::Mmap(e.to_string())
    }
}
