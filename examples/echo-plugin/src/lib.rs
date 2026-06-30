//! Atrium Echo 示例插件 — 演示动态插件的完整生命周期
//!
//! 编译为 cdylib 后，可被 PluginManager 通过 libloading 动态加载。
//!
//! ## 构建
//!
//! ```bash
//! cargo build -p atrium-echo-plugin
//! # 产物: target/debug/libatrium_echo_plugin.so (Linux)
//! #        target/debug/atrium_echo_plugin.dll   (Windows)
//! ```

use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};

use atrium_plugin::vtable::PluginVTable;

/// 插件是否已初始化
static LOADED: AtomicBool = AtomicBool::new(false);

/// 缓存的配置前缀（默认 "[echo]"）
static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();

// ── VTable 函数实现 ──

extern "C" fn plugin_name() -> *const c_char {
    // 返回静态 C 字符串（永不释放）
    static NAME: &[u8] = b"echo\0";
    NAME.as_ptr() as *const c_char
}

extern "C" fn plugin_version() -> *const c_char {
    static VERSION: &[u8] = b"1.0.0\0";
    VERSION.as_ptr() as *const c_char
}

extern "C" fn plugin_on_load(config_json: *const c_char) -> c_int {
    // 从 JSON 配置中读取 prefix
    let config = if config_json.is_null() {
        "{}"
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(config_json)
                .to_str()
                .unwrap_or("{}")
        }
    };

    // 简易 JSON 解析：查找 "prefix" 字段
    let prefix = if let Some(start) = config.find("\"prefix\"") {
        if let Some(val_start) = config[start..].find('"') {
            let val_start = start + val_start + 1;
            if let Some(val_end) = config[val_start..].find('"') {
                &config[val_start..val_start + val_end]
            } else {
                "[echo]"
            }
        } else {
            "[echo]"
        }
    } else {
        "[echo]"
    };

    let _ = PREFIX.set(prefix.to_string());
    LOADED.store(true, Ordering::SeqCst);
    0 // 成功
}

extern "C" fn plugin_on_unload() -> c_int {
    LOADED.store(false, Ordering::SeqCst);
    0
}

extern "C" fn plugin_on_message(
    msg_json: *const c_char,
    out_buf: *mut c_char,
    out_buf_len: usize,
) -> c_int {
    if !LOADED.load(Ordering::SeqCst) {
        return -1;
    }

    // 读取输入消息
    let msg = if msg_json.is_null() {
        ""
    } else {
        unsafe { std::ffi::CStr::from_ptr(msg_json).to_str().unwrap_or("") }
    };

    // 构建响应
    let prefix = PREFIX.get().map(|s| s.as_str()).unwrap_or("[echo]");
    let response = format!("{} {}", prefix, msg);

    // 写入输出缓冲区
    let bytes = response.as_bytes();
    let write_len = bytes.len().min(out_buf_len.saturating_sub(1)); // 留一个字节给 null 终止符

    if write_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, write_len);
            *out_buf.add(write_len) = 0; // null 终止符
        }
    }

    write_len as c_int
}

extern "C" fn plugin_on_tick() -> c_int {
    // Echo 插件不需要周期工作
    0
}

extern "C" fn plugin_on_shutdown() -> c_int {
    LOADED.store(false, Ordering::SeqCst);
    0
}

/// 插件入口点 — 返回 VTable
///
/// # Safety
///
/// 此函数返回的 VTable 中所有函数指针均指向本模块的静态函数，
/// 在动态库存活期间始终有效。
#[no_mangle]
pub extern "C" fn atrium_plugin_entry() -> PluginVTable {
    PluginVTable {
        name: plugin_name,
        version: plugin_version,
        on_load: plugin_on_load,
        on_unload: plugin_on_unload,
        on_message: plugin_on_message,
        on_tick: plugin_on_tick,
        on_shutdown: plugin_on_shutdown,
    }
}
