//! Tauri 应用后端入口：串联插件注册、命令处理器与核心初始化。
//!
//! 模块分层：`commands`（命令封装）/ `cores`（通用能力与跨层共享类型）/ `features`（业务逻辑）/
//! `plugins`（插件初始化）。新增命令的流程见 `commands/mod.rs`。

mod commands;
mod cores;
mod features;
mod plugins;

use crate::plugins::BuilderExt;

rust_i18n::i18n!("locales", fallback = "en");

/// 运行 Tauri 应用程序
///
/// # Panics
///
/// 如果应用程序无法初始化或运行，则会出现 panics
// `generate_context!` 把各插件的 JS API 与权限表嵌进同一闭包逐项装配，
// 上游已为其单开 8MiB 栈线程（见 `tauri-codegen` 的 context 生成），
// 加插件即涨栈帧是预期行为，此处仅豁免宏生成闭包，不掩盖手写代码
#[allow(clippy::large_stack_frames)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    cores::system::init_system();

    let specta_builder = cores::specta::init_builder();

    tauri::Builder::default()
        .plugin(plugins::log::init())
        .plugin(plugins::store::init())
        .plugin(plugins::opener::init())
        .plugin(plugins::clipboard::init())
        .plugin(plugins::system_fonts::init())
        .with_autostart()
        .with_updater()
        .with_window_state()
        .with_single_instance()     // 单实例必须先于 deep-link 注册（官方顺序要求），且仅桌面端启用（移动端为空操作）
        .with_deep_link()
        .with_shortcut()
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            cores::setup(app, &specta_builder);
            plugins::setup(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
