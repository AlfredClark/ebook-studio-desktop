//! 全局快捷键插件初始化（仅桌面端）：动态注册由配置中心按存量驱动，
//! 此处只装插件与触发回调——按下已注册的“显示应用”快捷键即显示主窗口。
//! 纯 Rust 侧 `GlobalShortcutExt` 调用，不走 JS invoke，故无需 capability 声明。

use tauri::{Runtime, plugin::Plugin};

/// 全局快捷键插件（仅桌面端）：初始不预注册任何组合，
/// 具体组合由 `cores::shortcut::sync` 按配置动态管理
pub fn init<R: Runtime>() -> impl Plugin<R> {
    use tauri_plugin_global_shortcut::ShortcutState;

    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::cores::tray::show_main_window(app);
            }
        })
        .build()
}

/// 启动收尾：按存量配置注册显示应用快捷键；失败只记日志（前端未挂载，无法 toast）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn setup(app: &tauri::App) {
    let handle = app.handle();
    let current = crate::cores::config::load_config(handle).shortcut_show;
    if current.trim().is_empty() {
        return;
    }
    if let Err(err) = crate::cores::shortcut::sync(handle, "", &current) {
        log::warn!("failed to register startup shortcut: {err:#}");
    }
}

/// 移动端空实现：该平台未声明 `global-shortcut` 依赖
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn setup(_app: &tauri::App) {}
