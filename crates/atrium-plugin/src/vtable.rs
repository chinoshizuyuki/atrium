// SPDX-License-Identifier: MIT
//! C 兼容插件 VTable — 动态库插件的 ABI 契约
//! C-compatible plugin VTable — ABI contract for dynamic library plugins.
//!
//! ## 插件 ABI 规范
//!
//! 每个动态插件必须导出一个 C 函数：
//!
//! ```rust,ignore
//! #[no_mangle]
//! pub extern "C" fn atrium_plugin_entry() -> PluginVTable { ... }
//! ```
//!
//! 返回的 `PluginVTable` 包含所有生命周期回调的函数指针。
//! 所有字符串通过 C 字符串（null-terminated UTF-8）传递。
//!
//! ### 字符串所有权约定
//!
//! | 函数 | 字符串参数 | 所有权 | 返回字符串 | 所有权 |
//! |------|-----------|--------|-----------|--------|
//! | `name()` | 无 | — | 静态字符串 | 插件拥有，永不释放 |
//! | `version()` | 无 | — | 静态字符串 | 插件拥有，永不释放 |
//! | `on_load(config)` | JSON 配置 | 宿主拥有，只读 | 无 | — |
//! | `on_unload()` | 无 | — | 无 | — |
//! | `on_message(msg, out, out_len)` | 消息 JSON | 宿主拥有，只读 | 写入 out 缓冲区 | 宿主拥有 |
//! | `on_tick()` | 无 | — | 无 | — |
//! | `on_shutdown()` | 无 | — | 无 | — |

use std::os::raw::{c_char, c_int};

/// C 兼容插件 VTable。
/// C-compatible plugin VTable.
///
/// 插件通过 `atrium_plugin_entry()` 导出此结构体。
/// Plugins export this struct via `atrium_plugin_entry()`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PluginVTable {
    /// 返回插件名称（静态 C 字符串，插件拥有，永不释放）
    /// Returns plugin name (static C string, plugin-owned, never freed)
    pub name: extern "C" fn() -> *const c_char,

    /// 返回插件版本（静态 C 字符串，插件拥有，永不释放）
    /// Returns plugin version (static C string, plugin-owned, never freed)
    pub version: extern "C" fn() -> *const c_char,

    /// 初始化插件。config_json: JSON 配置字符串（宿主拥有，只读）。返回 0 表示成功。
    /// Initialize plugin. config_json: JSON config string (host-owned, read-only). Returns 0 on success.
    pub on_load: extern "C" fn(config_json: *const c_char) -> c_int,

    /// 反初始化插件。返回 0 表示成功。
    /// Deinitialize plugin. Returns 0 on success.
    pub on_unload: extern "C" fn() -> c_int,

    /// 处理消息事件。
    /// Handle message event.
    ///
    /// - `msg_json`: 输入消息 JSON（宿主拥有，只读）
    /// - `out_buf`: 输出缓冲区（宿主拥有，插件写入）
    /// - `out_buf_len`: 输出缓冲区容量（字节数）
    /// - 返回值：写入 out_buf 的字节数（不含 null 终止符），0 表示无输出，-1 表示错误
    pub on_message:
        extern "C" fn(msg_json: *const c_char, out_buf: *mut c_char, out_buf_len: usize) -> c_int,

    /// 周期性 tick 回调。返回 0 表示成功。
    /// Periodic tick callback. Returns 0 on success.
    pub on_tick: extern "C" fn() -> c_int,

    /// 关机通知。返回 0 表示成功。
    /// Shutdown notification. Returns 0 on success.
    pub on_shutdown: extern "C" fn() -> c_int,
}

/// 插件入口点符号名（null-terminated，供 libloading::Library::get 使用）
/// Plugin entry point symbol name (null-terminated, for libloading::Library::get)
pub const PLUGIN_ENTRY_SYMBOL: &[u8] = b"atrium_plugin_entry\0";

/// 插件入口点函数类型
/// Plugin entry point function type
pub type PluginEntryFn = unsafe extern "C" fn() -> PluginVTable;

/// `on_message` 输出缓冲区默认大小（64 KB）
/// Default `on_message` output buffer size (64 KB)
pub const MESSAGE_BUFFER_SIZE: usize = 65536;

// ── 辅助函数 ──

/// 将 C 字符串指针安全转换为 Rust String。
/// 如果指针为 null，返回空字符串。
///
/// # Safety
///
/// 调用者必须确保 `ptr` 指向一个有效的 null-terminated UTF-8 字符串。
pub unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_symbol_is_null_terminated() {
        // 确保符号名以 null 结尾
        assert_eq!(PLUGIN_ENTRY_SYMBOL.last(), Some(&0));
    }

    #[test]
    fn test_vtable_is_repr_c() {
        // PluginVTable 的大小应该是 7 个指针（在 64-bit 上 = 56 字节）
        // name + version + on_load + on_unload + on_message + on_tick + on_shutdown
        assert_eq!(
            std::mem::size_of::<PluginVTable>(),
            7 * std::mem::size_of::<usize>()
        );
    }
}
