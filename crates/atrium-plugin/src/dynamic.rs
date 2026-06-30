// SPDX-License-Identifier: MIT
//! 动态库插件 — 通过 libloading 加载 .so/.dll/.dylib
//! Dynamic library plugin — loads .so/.dll/.dylib via libloading.
//!
//! ## 工作原理
//!
//! 1. 使用 `libloading::Library::new()` 加载动态库
//! 2. 查找 `atrium_plugin_entry` 符号，获取 VTable
//! 3. 通过 VTable 函数指针调用插件方法
//! 4. `DynamicPlugin` 拥有 `Library` 对象，确保库在插件存活期间不被卸载

use crate::error::PluginError;
use crate::vtable::{cstr_to_string, PluginEntryFn, PluginVTable, PLUGIN_ENTRY_SYMBOL};
use crate::Plugin;
use anyhow::Result;
use libloading::Library;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use tracing;

/// 动态库插件 — 封装 libloading 加载的 .so/.dll/.dylib
pub struct DynamicPlugin {
    /// 插件名称（从 VTable.name() 缓存）
    name: String,
    /// 插件版本（从 VTable.version() 缓存）
    version: String,
    /// 动态库句柄（必须最后 drop，否则 VTable 函数指针悬垂）
    _library: Library,
    /// C ABI VTable（函数指针指向 _library 的代码段）
    vtable: PluginVTable,
}

impl DynamicPlugin {
    /// 从动态库文件加载插件
    ///
    /// # Safety
    ///
    /// 调用者必须信任动态库的实现：
    /// - 库必须导出 `atrium_plugin_entry` 符号
    /// - VTable 中所有函数指针必须指向有效的函数
    /// - 插件实现必须是线程安全的（`Send + Sync`）
    pub unsafe fn load(path: &Path) -> Result<Self, PluginError> {
        let library = Library::new(path)
            .map_err(|e| PluginError::LibraryError(format!("{}: {}", path.display(), e)))?;

        // 查找入口符号
        let entry_fn: libloading::Symbol<PluginEntryFn> = library
            .get(PLUGIN_ENTRY_SYMBOL)
            .map_err(|_| PluginError::EntryPointNotFound(path.display().to_string()))?;

        // 调用入口函数获取 VTable
        let vtable = (*entry_fn)();

        // 缓存 name 和 version（从 C 字符串转换为 Rust String）
        let name = cstr_to_string((vtable.name)());
        let version = cstr_to_string((vtable.version)());

        tracing::info!("动态插件已加载: {} v{} ({})", name, version, path.display());

        // 注意：Library 必须移入 Self，否则 drop 后 VTable 悬垂
        Ok(Self {
            name,
            version,
            _library: library,
            vtable,
        })
    }

    /// 从动态库文件加载插件（safe 封装）
    ///
    /// 内部调用 `unsafe load()`，调用者需理解上述安全约束。
    pub fn load_from_file(path: &Path) -> Result<Self, PluginError> {
        // SAFETY: 调用者通过 manifest 发现插件，信任插件实现
        unsafe { Self::load(path) }
    }
}

impl Plugin for DynamicPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn on_load(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).unwrap_or_default();
        let ret = (self.vtable.on_load)(config_cstr.as_ptr());
        if ret == 0 {
            tracing::info!("插件 on_load 成功: {}", self.name);
            Ok(())
        } else {
            Err(PluginError::OnLoadFailed(ret).into())
        }
    }

    fn on_unload(&self) -> Result<()> {
        let ret = (self.vtable.on_unload)();
        if ret == 0 {
            tracing::info!("插件 on_unload 成功: {}", self.name);
            Ok(())
        } else {
            Err(PluginError::OnUnloadFailed(ret).into())
        }
    }

    fn on_message(&self, msg: &str) -> Option<String> {
        let msg_cstr = CString::new(msg).unwrap_or_default();
        let mut out_buf = vec![0u8; crate::vtable::MESSAGE_BUFFER_SIZE];

        let ret = {
            (self.vtable.on_message)(
                msg_cstr.as_ptr(),
                out_buf.as_mut_ptr() as *mut c_char,
                out_buf.len(),
            )
        };

        match ret.cmp(&0) {
            std::cmp::Ordering::Greater => {
                // ret = 写入的字节数（不含 null 终止符）
                let len = ret as usize;
                if len < out_buf.len() {
                    Some(String::from_utf8_lossy(&out_buf[..len]).to_string())
                } else {
                    tracing::warn!("插件 on_message 返回长度 {} 超过缓冲区，截断", len);
                    Some(String::from_utf8_lossy(&out_buf).to_string())
                }
            }
            std::cmp::Ordering::Equal => {
                // 无输出
                None
            }
            std::cmp::Ordering::Less => {
                // 错误
                tracing::warn!("插件 on_message 返回错误码: {} ({})", ret, self.name);
                None
            }
        }
    }

    fn on_tick(&self) -> Result<()> {
        let ret = (self.vtable.on_tick)();
        if ret == 0 {
            Ok(())
        } else {
            Err(PluginError::OnTickFailed(ret).into())
        }
    }

    fn on_shutdown(&self) -> Result<()> {
        let ret = (self.vtable.on_shutdown)();
        if ret == 0 {
            Ok(())
        } else {
            Err(PluginError::OnShutdownFailed(ret).into())
        }
    }
}

// Safety: DynamicPlugin 的 VTable 函数指针指向已加载的动态库代码段。
// 只要 _library 存活，这些指针就有效。插件作者需保证实现是线程安全的。
unsafe impl Send for DynamicPlugin {}
unsafe impl Sync for DynamicPlugin {}
