//! 通用能力与跨层共享类型：`types`（命令错误 / 结果）、`locale`（语言枚举）、
//! `config`（应用配置持久化）、`deep_link`（深链 URL 提取纯函数，供单实例插件复用）、
//! `shortcut`（显示应用的全局快捷键，仅桌面端）、
//! `specta`（绑定构建与导出）、`system`（panic 钩子与环境兼容）、
//! `tray`（系统托盘，仅桌面端）、`updater`（更新检查与安装，跨业务通用能力故放 `cores` 而非 `features`）。

use tauri_specta::Builder;

pub mod config;
pub mod deep_link;
pub mod locale;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod shortcut;
pub mod specta;
pub mod system;
pub mod tray;
pub mod types;
pub mod updater;

/// 应用启动时的核心初始化：挂载事件系统并装配应用配置（含界面语言），最后挂载系统托盘
pub fn setup(app: &tauri::App, builder: &Builder<tauri::Wry>) {
    specta::setup(app, builder);
    config::setup(app);
    tray::setup(app);
}
