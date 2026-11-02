//! 窗口状态插件初始化（仅桌面端）：关闭自动保存，启动恢复交由配置中心手动执行。

use tauri::{Runtime, plugin::Plugin};

/// 窗口状态插件（仅桌面端）：关闭时自动保存全部窗口状态；
/// `main` 跳过启动自动恢复，改为 `cores::config::setup` 按用户配置手动恢复
/// （`tauri.conf.json` 未给窗口取名时默认 `label` 即 `main`，多窗口时由 P1-8 重访）
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_window_state::Builder::default()
        .skip_initial_state("main")
        .build()
}
