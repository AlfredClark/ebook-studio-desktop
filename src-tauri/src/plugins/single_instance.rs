//! 单实例插件初始化（仅桌面端）：二次启动聚焦已有窗口并转发深链参数。

use tauri::{AppHandle, Emitter, Manager, plugin::TauriPlugin};

/// 单实例插件（仅桌面端）：二次启动时聚焦已有主窗口，并把深链参数转发给前端。
///
/// 与前端 `onOpenUrl` 路径互斥：首启走 deep-link 事件，次启走此处回调，
/// 两处最终都进前端同一 `handle`（另有短窗去重），故此处不做去重。
///
/// 构造器是 `Wry` 具体的，故桌面端 `BuilderExt` 实现收窄到 `Builder<Wry>`；
/// 经 `with_single_instance` 进注册链（移动端为空操作），调用处须保持先于深链注册。
pub fn init() -> TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app: &AppHandle, args: Vec<String>, _cwd: String| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
        let urls = crate::cores::deep_link::extract_deep_link_urls(&args);
        if !urls.is_empty() {
            log::info!(
                "forwarding {} deep link url(s) from second instance",
                urls.len()
            );
            let _ = app.emit("app:open-urls", urls);
        }
    })
}
