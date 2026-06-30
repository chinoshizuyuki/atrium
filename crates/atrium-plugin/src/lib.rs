// SPDX-License-Identifier: MIT
//! Atrium 插件系统 — 动态/静态插件加载与生命周期管理
//! Atrium plugin system — dynamic/static plugin loading and lifecycle management.
//!
//! ## 功能
//!
//! - **静态插件**：编译时注册，零开销
//! - **动态插件**：运行时通过 `libloading` 加载 .so/.dll/.dylib
//! - **插件发现**：扫描 plugins/ 目录下的 manifest.toml
//! - **生命周期**：on_load → on_message/on_tick → on_unload/on_shutdown
//! - **per-plugin 配置**：manifest.toml 中的 [plugin.config] 传递给 on_load
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use atrium_plugin::PluginManager;
//!
//! let mut mgr = PluginManager::new("/path/to/plugins");
//!
//! // 注册静态插件
//! mgr.register(Box::new(MyPlugin));
//!
//! // 发现并加载动态插件
//! mgr.discover_and_load()?;
//!
//! // 调用生命周期
//! mgr.load_all()?;           // 初始化所有已注册插件
//! mgr.on_message("hello")?;  // 广播消息给所有插件
//! mgr.on_tick()?;            // 周期 tick
//! mgr.unload_all()?;         // 卸载所有插件
//! ```
//!
//! ## 动态插件 ABI
//!
//! 动态插件必须导出 C 函数 `atrium_plugin_entry() -> PluginVTable`。
//! 详见 [`vtable::PluginVTable`]。

pub mod dynamic;
pub mod error;
pub mod manifest;
pub mod vtable;

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use error::PluginError;

// ── 插件 trait ──

/// 插件特质 — 所有插件（静态或动态）需实现此 trait
/// Plugin trait — all plugins (static or dynamic) must implement this.
pub trait Plugin: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;

    /// 插件版本（默认 "0.1.0"）
    fn version(&self) -> &str {
        "0.1.0"
    }

    /// 初始化插件。config 为 JSON 配置字符串（来自 manifest.toml 的 [plugin.config]）。
    /// 默认实现忽略配置。
    fn on_load(&self, _config: &str) -> Result<()> {
        Ok(())
    }

    /// 反初始化插件。默认实现无操作。
    fn on_unload(&self) -> Result<()> {
        Ok(())
    }

    /// 处理消息事件。返回 Some(response) 表示插件产生了输出（注入上下文）。
    /// 默认实现忽略消息。
    fn on_message(&self, _msg: &str) -> Option<String> {
        None
    }

    /// 周期性 tick 回调（由 Scheduler 驱动）。
    /// 默认实现无操作。
    fn on_tick(&self) -> Result<()> {
        Ok(())
    }

    /// 关机通知（进程退出前调用）。
    /// 默认实现无操作。
    fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }
}

// ── 插件状态 ──

/// 插件运行状态
#[derive(Debug, Clone)]
pub enum PluginState {
    /// 已注册但尚未初始化
    Registered,
    /// 已成功加载（on_load 成功）
    Loaded,
    /// 加载或运行时出错
    Errored(String),
    /// 已卸载
    Unloaded,
}

impl std::fmt::Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginState::Registered => write!(f, "registered"),
            PluginState::Loaded => write!(f, "loaded"),
            PluginState::Errored(e) => write!(f, "error: {}", e),
            PluginState::Unloaded => write!(f, "unloaded"),
        }
    }
}

// ── 插件条目 ──

/// 已加载的插件条目（内部使用）
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    state: PluginState,
    /// 来源：static 或 dynamic
    source: PluginSource,
}

/// 插件来源
#[derive(Debug, Clone, Copy)]
pub enum PluginSource {
    /// 编译时静态注册
    Static,
    /// 运行时动态加载
    Dynamic,
}

// ── 插件管理器 ──

/// 插件管理器 — 负责注册、发现、加载、卸载插件
pub struct PluginManager {
    /// 已注册/已加载的插件（按名称索引）
    entries: HashMap<String, PluginEntry>,
    /// 插件目录（用于动态插件发现）
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// 创建插件管理器
    ///
    /// # Arguments
    /// * `plugin_dir` - 插件目录路径，用于动态插件发现
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        Self {
            entries: HashMap::new(),
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }

    /// 创建无插件目录的管理器（仅支持静态注册）
    pub fn new_static_only() -> Self {
        Self {
            entries: HashMap::new(),
            plugin_dir: PathBuf::new(),
        }
    }

    // ── 静态注册 ──

    /// 注册一个静态插件（编译时已知）
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        tracing::info!("注册静态插件: {} v{}", name, plugin.version());
        self.entries.insert(
            name,
            PluginEntry {
                plugin,
                state: PluginState::Registered,
                source: PluginSource::Static,
            },
        );
    }

    // ── 动态发现与加载 ──

    /// 扫描插件目录，发现并加载所有动态插件
    ///
    /// 遍历 `plugin_dir` 下的子目录，查找 `manifest.toml`，
    /// 加载对应的动态库并调用 on_load。
    pub fn discover_and_load(&mut self) -> Result<(), PluginError> {
        let discoveries = manifest::discover_plugins(&self.plugin_dir);

        for result in discoveries {
            match result {
                Ok((dir, manifest)) => {
                    let lib_path = manifest.library_path(&dir);
                    let plugin_name = manifest.plugin.name.clone();

                    if self.entries.contains_key(&plugin_name) {
                        tracing::warn!("插件已存在，跳过: {}", plugin_name);
                        continue;
                    }

                    if !lib_path.exists() {
                        tracing::warn!(
                            "动态库不存在: {} (manifest: {}/manifest.toml)",
                            lib_path.display(),
                            dir.display()
                        );
                        continue;
                    }

                    match dynamic::DynamicPlugin::load_from_file(&lib_path) {
                        Ok(dyn_plugin) => {
                            let config_json = manifest.config_json();
                            let name = dyn_plugin.name().to_string();
                            let version = dyn_plugin.version().to_string();

                            // 注册到 entries
                            self.entries.insert(
                                name.clone(),
                                PluginEntry {
                                    plugin: Box::new(dyn_plugin),
                                    state: PluginState::Registered,
                                    source: PluginSource::Dynamic,
                                },
                            );

                            // 调用 on_load
                            if let Some(entry) = self.entries.get_mut(&name) {
                                match entry.plugin.on_load(&config_json) {
                                    Ok(()) => {
                                        entry.state = PluginState::Loaded;
                                        tracing::info!("动态插件已加载: {} v{}", name, version);
                                    }
                                    Err(e) => {
                                        entry.state = PluginState::Errored(e.to_string());
                                        tracing::error!("动态插件 on_load 失败: {} - {}", name, e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("动态插件加载失败: {} - {}", lib_path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("插件清单解析失败: {}", e);
                }
            }
        }

        Ok(())
    }

    // ── 生命周期管理 ──

    /// 初始化所有已注册但未加载的插件（调用 on_load）
    pub fn load_all(&mut self) -> Result<()> {
        let names: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| matches!(e.state, PluginState::Registered))
            .map(|(name, _)| name.clone())
            .collect();

        for name in names {
            if let Some(entry) = self.entries.get_mut(&name) {
                match entry.plugin.on_load("{}") {
                    Ok(()) => {
                        entry.state = PluginState::Loaded;
                        tracing::info!("插件已加载: {}", name);
                    }
                    Err(e) => {
                        entry.state = PluginState::Errored(e.to_string());
                        tracing::error!("插件 on_load 失败: {} - {}", name, e);
                    }
                }
            }
        }
        Ok(())
    }

    /// 卸载所有已加载的插件（调用 on_unload）
    pub fn unload_all(&mut self) -> Result<()> {
        let names: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| matches!(e.state, PluginState::Loaded))
            .map(|(name, _)| name.clone())
            .collect();

        for name in &names {
            if let Some(entry) = self.entries.get_mut(name) {
                match entry.plugin.on_unload() {
                    Ok(()) => {
                        entry.state = PluginState::Unloaded;
                        tracing::info!("插件已卸载: {}", name);
                    }
                    Err(e) => {
                        entry.state = PluginState::Errored(e.to_string());
                        tracing::error!("插件 on_unload 失败: {} - {}", name, e);
                    }
                }
            }
        }
        Ok(())
    }

    /// 通知所有插件关机（调用 on_shutdown）
    pub fn shutdown_all(&self) -> Result<()> {
        for (name, entry) in &self.entries {
            if let Err(e) = entry.plugin.on_shutdown() {
                tracing::error!("插件 on_shutdown 失败: {} - {}", name, e);
            }
        }
        Ok(())
    }

    // ── 消息广播 ──

    /// 向所有已加载插件广播消息，收集插件响应
    ///
    /// 返回所有非 None 的插件响应（可用于注入上下文）
    pub fn on_message(&self, msg: &str) -> Vec<(String, String)> {
        let mut responses = Vec::new();
        for (name, entry) in &self.entries {
            if !matches!(entry.state, PluginState::Loaded) {
                continue;
            }
            if let Some(response) = entry.plugin.on_message(msg) {
                responses.push((name.clone(), response));
            }
        }
        responses
    }

    /// 周期 tick 所有已加载插件
    pub fn on_tick(&self) -> Result<()> {
        for (name, entry) in &self.entries {
            if !matches!(entry.state, PluginState::Loaded) {
                continue;
            }
            if let Err(e) = entry.plugin.on_tick() {
                tracing::warn!("插件 on_tick 失败: {} - {}", name, e);
            }
        }
        Ok(())
    }

    // ── 查询 ──

    /// 获取已加载插件数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否无插件
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 获取指定插件的状态
    pub fn state(&self, name: &str) -> Option<&PluginState> {
        self.entries.get(name).map(|e| &e.state)
    }

    /// 获取所有插件名称
    pub fn plugin_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// 获取所有已加载插件的名称
    pub fn loaded_names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, e)| matches!(e.state, PluginState::Loaded))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// 生成健康状态报告
    pub fn health_status(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .map(|(name, entry)| {
                let status = format!(
                    "{} v{} [{:?}] {}",
                    name,
                    entry.plugin.version(),
                    entry.source,
                    entry.state
                );
                (name.clone(), status)
            })
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new_static_only()
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用静态插件
    struct TestPlugin {
        name_str: String,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                name_str: name.to_string(),
            }
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name_str
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn on_load(&self, _config: &str) -> Result<()> {
            Ok(())
        }
        fn on_unload(&self) -> Result<()> {
            Ok(())
        }
        fn on_message(&self, msg: &str) -> Option<String> {
            Some(format!("[echo] {}", msg))
        }
        fn on_tick(&self) -> Result<()> {
            Ok(())
        }
        fn on_shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_register_and_load() {
        let mut mgr = PluginManager::new_static_only();
        assert!(mgr.is_empty());

        mgr.register(Box::new(TestPlugin::new("echo")));
        assert_eq!(mgr.len(), 1);
        assert!(mgr.state("echo").is_some());

        assert!(mgr.load_all().is_ok());
        assert!(matches!(mgr.state("echo"), Some(PluginState::Loaded)));
    }

    #[test]
    fn test_unload_all() {
        let mut mgr = PluginManager::new_static_only();
        mgr.register(Box::new(TestPlugin::new("echo")));
        assert!(mgr.load_all().is_ok());
        assert!(mgr.unload_all().is_ok());
        assert!(matches!(mgr.state("echo"), Some(PluginState::Unloaded)));
    }

    #[test]
    fn test_on_message() {
        let mut mgr = PluginManager::new_static_only();
        mgr.register(Box::new(TestPlugin::new("echo")));
        assert!(mgr.load_all().is_ok());

        let responses = mgr.on_message("hello");
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].0, "echo");
        assert_eq!(responses[0].1, "[echo] hello");
    }

    #[test]
    fn test_on_tick() {
        let mut mgr = PluginManager::new_static_only();
        mgr.register(Box::new(TestPlugin::new("echo")));
        assert!(mgr.load_all().is_ok());
        assert!(mgr.on_tick().is_ok());
    }

    #[test]
    fn test_multiple_plugins() {
        let mut mgr = PluginManager::new_static_only();
        mgr.register(Box::new(TestPlugin::new("echo")));
        mgr.register(Box::new(TestPlugin::new("sentiment")));
        assert_eq!(mgr.len(), 2);

        assert!(mgr.load_all().is_ok());
        let loaded = mgr.loaded_names();
        assert_eq!(loaded.len(), 2);

        let responses = mgr.on_message("test");
        assert_eq!(responses.len(), 2);
    }

    #[test]
    fn test_health_status() {
        let mut mgr = PluginManager::new_static_only();
        mgr.register(Box::new(TestPlugin::new("echo")));
        assert!(mgr.load_all().is_ok());

        let status = mgr.health_status();
        assert!(status.contains_key("echo"));
        assert!(status["echo"].contains("loaded"));
    }

    #[test]
    fn test_default() {
        let mgr = PluginManager::default();
        assert!(mgr.is_empty());
    }

    #[test]
    fn test_plugin_state_display() {
        assert_eq!(format!("{}", PluginState::Registered), "registered");
        assert_eq!(format!("{}", PluginState::Loaded), "loaded");
        assert_eq!(format!("{}", PluginState::Unloaded), "unloaded");
        assert_eq!(
            format!("{}", PluginState::Errored("fail".into())),
            "error: fail"
        );
    }
}
