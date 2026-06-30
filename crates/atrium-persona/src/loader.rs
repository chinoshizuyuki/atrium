// SPDX-License-Identifier: MIT
//! 角色卡加载器 — 支持 YAML / bincode / mmap
//! Character card loader — Supports YAML / bincode / mmap formats.

use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::PersonaError;
use crate::types::PersonaDef;

/// 角色卡加载器
pub struct PersonaLoader {
    /// 搜索路径列表（优先级从高到低）
    search_paths: Vec<PathBuf>,
}

impl PersonaLoader {
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("profiles"),
                PathBuf::from("config/profiles"),
                PathBuf::from("/etc/atrium/profiles"),
            ],
        }
    }

    pub fn with_search_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths: paths,
        }
    }

    /// 添加搜索路径
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// 按名称查找并加载角色卡
    pub fn load(&self, name: &str) -> Result<PersonaDef, PersonaError> {
        for dir in &self.search_paths {
            // 先试 .yaml
            let yaml_path = dir.join(format!("{}.yaml", name));
            if yaml_path.exists() {
                return self.load_yaml(&yaml_path);
            }
            // 再试 .yml
            let yml_path = dir.join(format!("{}.yml", name));
            if yml_path.exists() {
                return self.load_yaml(&yml_path);
            }
            // 最后试 .bin (bincode 预编译)
            let bin_path = dir.join(format!("{}.bin", name));
            if bin_path.exists() {
                return self.load_bincode(&bin_path);
            }
        }
        Err(PersonaError::NotFound(format!(
            "未找到角色卡 '{}'，搜索路径: {:?}",
            name, self.search_paths
        )))
    }

    /// 从 YAML 文件加载
    pub fn load_yaml(&self, path: impl AsRef<Path>) -> Result<PersonaDef, PersonaError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            PersonaError::LoadFailed(format!("读取 {:?} 失败: {}", path.as_ref(), e))
        })?;

        let def: PersonaDef = serde_yaml::from_str(&content)
            .map_err(|e| PersonaError::Serde(format!("解析 {:?} 失败: {}", path.as_ref(), e)))?;

        info!("角色卡加载成功: {} (来源: {:?})", def.name, path.as_ref());
        Ok(def)
    }

    /// 从 bincode 文件加载（mmap 零拷贝）
    pub fn load_bincode(&self, path: impl AsRef<Path>) -> Result<PersonaDef, PersonaError> {
        let file = std::fs::File::open(path.as_ref())?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let def: PersonaDef = bincode::deserialize(&mmap[..]).map_err(|e| {
            PersonaError::Serde(format!("bincode 反序列化 {:?} 失败: {}", path.as_ref(), e))
        })?;

        info!(
            "角色卡加载成功 (mmap): {} (来源: {:?})",
            def.name,
            path.as_ref()
        );
        Ok(def)
    }

    /// 将 YAML 预编译为 bincode（产品部署用）
    pub fn compile_to_bincode(
        yaml_path: impl AsRef<Path>,
        bin_path: impl AsRef<Path>,
    ) -> Result<(), PersonaError> {
        let content = std::fs::read_to_string(yaml_path.as_ref()).map_err(|e| {
            PersonaError::LoadFailed(format!("读取 {:?} 失败: {}", yaml_path.as_ref(), e))
        })?;

        let def: PersonaDef = serde_yaml::from_str(&content).map_err(|e| {
            PersonaError::Serde(format!("解析 {:?} 失败: {}", yaml_path.as_ref(), e))
        })?;

        let bytes = bincode::serialize(&def)
            .map_err(|e| PersonaError::Serde(format!("序列化失败: {}", e)))?;

        if let Some(parent) = bin_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(bin_path.as_ref(), &bytes)?;

        info!(
            "角色卡预编译完成: {:?} → {:?} ({} bytes)",
            yaml_path.as_ref(),
            bin_path.as_ref(),
            bytes.len()
        );
        Ok(())
    }
}

impl Default for PersonaLoader {
    fn default() -> Self {
        Self::new()
    }
}
