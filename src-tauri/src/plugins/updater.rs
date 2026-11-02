//! 自动更新插件初始化（仅桌面端）。

use tauri::{Runtime, plugin::Plugin};

/// 自动更新插件（仅桌面端）：更新源与签名公钥配置在 `tauri.conf.json` 的 `plugins.updater`
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_updater::Builder::new().build()
}
