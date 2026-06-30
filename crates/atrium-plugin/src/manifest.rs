// SPDX-License-Identifier: MIT
//! 插件清单 — 描述动态插件的元信息与配置
//! Plugin manifest — metadata and configuration for dynamic plugins.
//!
//! 每个动态插件目录下需要一个 `manifest.toml` 文件：
//!
//! ```toml
//! [plugin]
//! name = "echo"
//! version = "1.0.0"
//! description = "Echo plugin example"
//! author = "Atrium"
//!
//! # 动态库文件名（仅 stem，平台扩展自动添加）
//! # 如不指定，默认为 lib<name>.so / lib<name>.dylib / <name>.dll
//! library = "libecho"
//!
//! # 插件专属配置（传递给 on_load 的 JSON）
//! [plugin.config]
//! prefix = "[echo]"
//! max_length = 1024
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::PluginError;

/// 插件清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
}

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// 插件名称（唯一标识）
    pub name: String,
    /// 插件版本（semver）
    pub version: String,
    /// 插件描述
    #[serde(default)]
    pub description: String,
    /// 插件作者
    #[serde(default)]
    pub author: String,
    /// 动态库路径（相对于清单文件所在目录）
    /// 如不指定，按平台约定自动推断
    #[serde(default)]
    pub library: Option<String>,
    /// 插件专属配置（任意 TOML 值，序列化为 JSON 传给 on_load）
    #[serde(default)]
    pub config: Option<toml::Value>,
}

impl PluginManifest {
    /// 从文件加载清单
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| PluginError::ManifestError(e.to_string()))
    }

    /// 从 TOML 字符串解析清单
    pub fn parse_toml(content: &str) -> Result<Self, PluginError> {
        toml::from_str(content).map_err(|e| PluginError::ManifestError(e.to_string()))
    }

    /// 获取动态库的完整路径
    ///
    /// 如果 `library` 字段已指定，直接使用（相对于 plugin_dir）；
    /// 否则按平台约定推断：Linux/macOS → `lib<name>.so`/`.dylib`，Windows → `<name>.dll`
    pub fn library_path(&self, plugin_dir: &Path) -> PathBuf {
        if let Some(ref lib) = self.plugin.library {
            // 如果是绝对路径，直接使用；否则相对于 plugin_dir
            let lib_path = PathBuf::from(lib);
            if lib_path.is_absolute() {
                lib_path
            } else {
                plugin_dir.join(lib)
            }
        } else {
            // 自动推断：plugins/<name>/lib<name>.so 或 <name>.dll
            let ext = platform_library_extension();
            let filename = if cfg!(target_os = "windows") {
                format!("{}{}", self.plugin.name, ext)
            } else {
                format!("lib{}{}", self.plugin.name, ext)
            };
            plugin_dir.join(&self.plugin.name).join(filename)
        }
    }

    /// 将插件配置序列化为 JSON 字符串（传给 on_load）
    pub fn config_json(&self) -> String {
        match &self.plugin.config {
            Some(value) => serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
            None => "{}".to_string(),
        }
    }
}

/// 返回当前平台的动态库扩展名
pub fn platform_library_extension() -> &'static str {
    if cfg!(target_os = "linux") {
        ".so"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else if cfg!(target_os = "windows") {
        ".dll"
    } else {
        ".so" // fallback
    }
}

/// 在指定目录下发现所有插件清单
///
/// 扫描策略：遍历 `plugin_dir` 下的每个子目录，查找 `manifest.toml`
pub fn discover_plugins(plugin_dir: &Path) -> Vec<Result<(PathBuf, PluginManifest), PluginError>> {
    let mut results = Vec::new();

    let entries = match std::fs::read_dir(plugin_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("插件目录不存在或无法读取: {} ({})", plugin_dir.display(), e);
            return results;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.toml");
        if !manifest_path.exists() {
            tracing::debug!("跳过（无 manifest.toml）: {}", path.display());
            continue;
        }
        results.push(PluginManifest::load(&manifest_path).map(|m| (path.clone(), m)));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let toml_str = r#"
[plugin]
name = "echo"
version = "1.0.0"
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        assert_eq!(manifest.plugin.name, "echo");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert!(manifest.plugin.description.is_empty());
        assert!(manifest.plugin.library.is_none());
        assert!(manifest.plugin.config.is_none());
    }

    #[test]
    fn test_parse_full_manifest() {
        let toml_str = r#"
[plugin]
name = "sentiment"
version = "2.1.0"
description = "Sentiment analysis plugin"
author = "Atrium Team"
library = "libsentiment_ext.so"

[plugin.config]
model = "distilbert"
threshold = 0.7
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        assert_eq!(manifest.plugin.name, "sentiment");
        assert_eq!(manifest.plugin.version, "2.1.0");
        assert_eq!(manifest.plugin.description, "Sentiment analysis plugin");
        assert_eq!(manifest.plugin.author, "Atrium Team");
        assert_eq!(
            manifest.plugin.library.as_deref(),
            Some("libsentiment_ext.so")
        );
        assert!(manifest.plugin.config.is_some());
    }

    #[test]
    fn test_config_json() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[plugin.config]
key = "value"
count = 42
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        let json = manifest.config_json();
        assert!(json.contains("key"));
        assert!(json.contains("value"));
    }

    #[test]
    fn test_config_json_empty() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        assert_eq!(manifest.config_json(), "{}");
    }

    #[test]
    fn test_library_path_auto_infer() {
        let toml_str = r#"
[plugin]
name = "echo"
version = "1.0.0"
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        let dir = PathBuf::from("/opt/atrium/plugins");
        let lib_path = manifest.library_path(&dir);

        if cfg!(target_os = "windows") {
            assert!(lib_path.to_str().unwrap().contains("echo.dll"));
        } else {
            assert!(lib_path.to_str().unwrap().contains("libecho"));
        }
    }

    #[test]
    fn test_library_path_explicit() {
        let toml_str = r#"
[plugin]
name = "custom"
version = "1.0.0"
library = "my_custom_lib.so"
"#;
        let manifest = PluginManifest::parse_toml(toml_str).unwrap();
        let dir = PathBuf::from("/opt/atrium/plugins");
        let lib_path = manifest.library_path(&dir);
        assert!(lib_path.to_str().unwrap().contains("my_custom_lib.so"));
    }
}
