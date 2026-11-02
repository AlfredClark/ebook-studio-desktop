//! 剪贴板插件初始化：仅 Rust 侧使用——about 页经 `copy_system_info` 命令复制系统信息（纯文本），
//! 前端无 JS 直调，故不声明 capability（能力最小化，见 AGENTS.md 的「安全红线」）。

use tauri::{Runtime, plugin::Plugin};

/// 剪贴板插件：写入走 Rust API，不经过 `WebView` 权限层
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_clipboard_manager::init()
}
