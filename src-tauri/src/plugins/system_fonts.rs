//! 系统字体插件初始化：供设置页读取已安装字体族名。

use tauri::{Runtime, plugin::Plugin};

/// 系统字体插件：仅供设置页读取已安装字体族名做下拉候选，字体偏好本身仍走前端 `localStorage`。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_system_fonts::init()
}
