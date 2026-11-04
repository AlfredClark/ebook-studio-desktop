//! 小说构建命令：`build/` 目录树的生成、列举与单文件读写回存的薄封装，
//! 具体文件操作在 `features::novel_build`。

use crate::cores::types::CommandResult;
use crate::features::novel_build::{self, BuildFileContent, BuildOutcome, FormatOutcome};

/// 构建小说：清空构建目录后全量重生成，返回文件树。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn build_novel_content(app: tauri::AppHandle, id: String) -> CommandResult<BuildOutcome> {
    Ok(novel_build::build_novel(&app, &id)?)
}

/// 读取构建树：未构建回 `None`，前端展示空态引导构建。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_build_tree(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<BuildOutcome>> {
    Ok(novel_build::build_tree(&app, &id)?)
}

/// 读取构建文件：二进制回 `content: None`，固定文件标记不可编辑。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_build_file(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<BuildFileContent> {
    Ok(novel_build::build_file(&app, &id, &path)?)
}

/// 保存构建文件：固定文件与二进制拒绝写入。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_novel_build_file(
    app: tauri::AppHandle,
    id: String,
    path: String,
    content: String,
) -> CommandResult<()> {
    Ok(novel_build::write_build_file(&app, &id, &path, &content)?)
}

/// 格式化单个构建文件：只读内存返回格式化文本，不落盘。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn format_novel_build_file(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<String> {
    Ok(novel_build::format_build_file(&app, &id, &path)?)
}

/// 批量格式化构建文件：逐个落盘，单个失败不中断。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn format_all_novel_build_files(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<FormatOutcome> {
    Ok(novel_build::format_all_build_files(&app, &id)?)
}
