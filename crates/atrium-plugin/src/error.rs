// SPDX-License-Identifier: MIT
//! 插件系统错误类型
//! Plugin system error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    /// 插件未找到
    #[error("plugin not found: {0}")]
    NotFound(String),

    /// 插件已加载
    #[error("plugin already loaded: {0}")]
    AlreadyLoaded(String),

    /// 插件加载失败
    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    /// 插件卸载失败
    #[error("plugin unload failed: {0}")]
    UnloadFailed(String),

    /// 插件清单解析错误
    #[error("manifest error: {0}")]
    ManifestError(String),

    /// 动态库加载错误
    #[error("dynamic library error: {0}")]
    LibraryError(String),

    /// 插件入口符号未找到
    #[error("plugin entry point not found in {0}")]
    EntryPointNotFound(String),

    /// 插件 on_load 返回错误码
    #[error("plugin on_load returned error code: {0}")]
    OnLoadFailed(i32),

    /// 插件 on_unload 返回错误码
    #[error("plugin on_unload returned error code: {0}")]
    OnUnloadFailed(i32),

    /// 插件 on_tick 返回错误码
    #[error("plugin on_tick returned error code: {0}")]
    OnTickFailed(i32),

    /// 插件 on_shutdown 返回错误码
    #[error("plugin on_shutdown returned error code: {0}")]
    OnShutdownFailed(i32),

    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
