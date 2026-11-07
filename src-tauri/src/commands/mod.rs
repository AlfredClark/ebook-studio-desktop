//! 命令层：每个命令都是轻量的 `#[tauri::command]` 封装，业务逻辑放在 `features`，
//! 通用能力放在 `cores`，这里只做参数校验与结果转换。
//!
//! 本层按前端调用域分组（配置 / 系统 / 更新），不与 `features` 一一对应：
//! 具体业务委托给 `features` 中对应业务的文件，多业务通用的能力委托给 `cores`。

#![allow(clippy::unnecessary_wraps)]

pub mod comic_build;
pub mod comic_export;
pub mod comic_files;
pub mod comic_package;
pub mod config;
pub mod metadata;
pub mod novel_build;
pub mod novel_export;
pub mod novel_files;
pub mod novel_package;
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
            $crate::commands::comic_files::get_comic_images_status,
            $crate::commands::comic_files::import_comic_images,
            $crate::commands::comic_files::parse_comic_images,
            $crate::commands::comic_files::get_comic_content_outline,
            $crate::commands::comic_files::get_comic_image_path,
            $crate::commands::comic_files::update_comic_volumes,
            $crate::commands::comic_files::get_comic_convert_status,
            $crate::commands::comic_files::preview_comic_image_conversion,
            $crate::commands::comic_files::convert_comic_images,
            $crate::commands::comic_build::build_comic_content,
            $crate::commands::comic_build::preview_comic_txt_title,
            $crate::commands::comic_build::clear_comic_build_content,
            $crate::commands::comic_build::get_comic_build_tree,
            $crate::commands::comic_build::get_comic_build_file,
            $crate::commands::comic_build::get_comic_build_image_path,
            $crate::commands::comic_build::update_comic_build_file,
            $crate::commands::comic_build::format_comic_build_text,
            $crate::commands::comic_build::format_all_comic_build_files,
            $crate::commands::comic_package::package_comic_content,
            $crate::commands::comic_package::get_comic_package_status,
            $crate::commands::comic_package::get_comic_package_progress,
            $crate::commands::comic_package::validate_comic_package,
            $crate::commands::comic_export::get_comic_export_status,
            $crate::commands::comic_export::get_comic_export_progress,
            $crate::commands::comic_export::export_comic_content,
            $crate::commands::projects::list_projects,
            $crate::commands::projects::create_project,
            $crate::commands::projects::rename_project,
            $crate::commands::projects::delete_projects,
            $crate::commands::projects::open_project_dir,
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
            $crate::commands::novel_files::get_novel_chapter,
            $crate::commands::novel_files::update_novel_chapter,
            $crate::commands::novel_files::filter_novel_content,
            $crate::commands::novel_build::build_novel_content,
            $crate::commands::novel_build::clear_novel_build_content,
            $crate::commands::novel_build::get_novel_build_tree,
            $crate::commands::novel_build::get_novel_build_file,
            $crate::commands::novel_build::get_novel_build_image_path,
            $crate::commands::novel_build::update_novel_build_file,
            $crate::commands::novel_build::format_novel_build_text,
            $crate::commands::novel_build::format_all_novel_build_files,
            $crate::commands::novel_package::package_novel_content,
            $crate::commands::novel_package::get_novel_package_status,
            $crate::commands::novel_package::get_novel_package_progress,
            $crate::commands::novel_package::validate_novel_package,
            $crate::commands::novel_export::get_novel_export_status,
            $crate::commands::novel_export::preview_novel_txt_title,
            $crate::commands::novel_export::export_novel_content,
            $crate::commands::system::get_system_info,
            $crate::commands::system::quit_app,
            $crate::commands::system::open_log_dir,
            $crate::commands::system::open_config_dir,
            $crate::commands::system::open_export_dir,
            $crate::commands::system::copy_system_info,
            $crate::commands::updater::check_update,
            $crate::commands::updater::download_and_install_update,
            $crate::commands::updater::restart_app,
        ]
    };
}

pub(crate) use collect_commands;
