//! 开机自启插件初始化（仅桌面端）：落盘失败时由配置中心回滚操作系统状态。

use tauri::{Runtime, plugin::Plugin};
use tauri_plugin_autostart::MacosLauncher;

/// 开机自启插件（仅桌面端）：用户级 `LaunchAgent` 注册，开机参数暂不定制
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)
}
