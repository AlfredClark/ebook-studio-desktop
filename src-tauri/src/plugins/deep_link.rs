//! 深链协议插件初始化（仅桌面端）。

use tauri::{Runtime, plugin::Plugin};

/// 深链插件（仅桌面端）：协议在 `tauri.conf.json` 的 `plugins.deep-link` 注册；
/// Win/Linux 的运行时注册在下面的 `setup` 内完成，冷启动与运行中 URL
/// 经 `onOpenUrl` / `getCurrent` 到达前端，次实例的参数走单实例插件转发。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_deep_link::init()
}

/// Win/Linux 的协议关联运行时注册（macOS 走包内 Info.plist，Linux 的
/// `.desktop` `MimeType` 由打包器自动生成）；失败只记日志，不阻断启动
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn setup(app: &tauri::App) {
    use tauri_plugin_deep_link::DeepLinkExt;
    if let Err(err) = app.deep_link().register_all() {
        log::warn!("failed to register deep link schemes: {err:#}");
    }
}

/// 移动端空实现：该平台未声明 `deep-link` 依赖
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn setup(_app: &tauri::App) {}
