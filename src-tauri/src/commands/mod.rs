//! 命令层：每个命令都是轻量的 `#[tauri::command]` 封装，业务逻辑放在 `features`，
//! 通用能力放在 `cores`，这里只做参数校验与结果转换。
//!
//! 本层按前端调用域分组（配置 / 系统 / 更新），不与 `features` 一一对应：
//! 具体业务委托给 `features` 中对应业务的文件，多业务通用的能力委托给 `cores`。

#![allow(clippy::unnecessary_wraps)]

pub mod config;
pub mod metadata;
pub mod novel_files;
pub mod projects;
pub mod system;
pub mod updater;

/// 汇总全部命令：既用于 `specta` 生成前端绑定，也用于挂载 `invoke_handler`。
///
/// 未在此注册的命令不会出现在绑定里，前端也就调用不到。
macro_rules! collect_commands {
    () => {
        tauri_specta::collect_commands![
            $crate::commands::config::get_config,
            $crate::commands::config::reset_config,
            $crate::commands::config::update_config,
            $crate::commands::projects::list_projects,
            $crate::commands::projects::create_project,
            $crate::commands::projects::rename_project,
            $crate::commands::projects::delete_projects,
            $crate::commands::metadata::get_project_metadata,
            $crate::commands::metadata::update_project_metadata,
            $crate::commands::metadata::reset_project_metadata,
            $crate::commands::metadata::import_project_cover,
            $crate::commands::metadata::get_project_cover_path,
            $crate::commands::metadata::get_project_overview,
            $crate::commands::novel_files::get_novel_text_status,
            $crate::commands::novel_files::import_novel_text,
            $crate::commands::novel_files::parse_novel_text,
            $crate::commands::novel_files::get_novel_content_outline,
            $crate::commands::novel_files::renumber_novel_content,
            $crate::commands::system::get_system_info,
            $crate::commands::system::quit_app,
            $crate::commands::system::open_log_dir,
            $crate::commands::system::open_config_dir,
            $crate::commands::system::copy_system_info,
            $crate::commands::updater::check_update,
            $crate::commands::updater::download_and_install_update,
            $crate::commands::updater::restart_app,
        ]
    };
}

pub(crate) use collect_commands;
