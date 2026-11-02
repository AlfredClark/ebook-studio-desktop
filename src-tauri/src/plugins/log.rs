//! 日志插件初始化：`stdout` / 日志文件 / `Webview` 控制台三路输出。

use tauri::{Runtime, plugin::Plugin};

/// 日志插件：同时输出到 `stdout`、日志文件（单文件 1MB、保留 10 份轮转）与 `Webview` 控制台；
/// 级别为开发 `Debug` / 发布 `Info`
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_log::Builder::new()
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                file_name: Some("app".to_string()),
            }),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
        ])
        .max_file_size(1_000_000)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(10))
        .file_open_strategy(tauri_plugin_log::FileOpenStrategy::Rotate)
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .build()
}
