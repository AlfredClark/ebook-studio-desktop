//! 小说原文命令：`sources/content.txt` 导入、状态查询、卷章解析与编号重排的薄封装，具体文件操作在 `features::novel_files`。

use crate::cores::types::CommandResult;
use crate::features::novel_files::{self, NovelTextStatus, ParseOutline};

/// 查询小说原文状态；缺文件回落空态，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_text_status(app: tauri::AppHandle, id: String) -> CommandResult<NovelTextStatus> {
    Ok(novel_files::text_status(&app, &id)?)
}

/// 导入小说原文：把用户选取的 TXT 覆盖拷贝为 `sources/content.txt`，返回最新状态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn import_novel_text(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<NovelTextStatus> {
    Ok(novel_files::import_text(&app, &id, &source_path)?)
}

/// 解析小说原文：按卷章正则逐行切分后落盘 `content.json`，返回识别大纲（正文行不出 IPC）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn parse_novel_text(
    app: tauri::AppHandle,
    id: String,
    volume_pattern: String,
    chapter_pattern: String,
) -> CommandResult<ParseOutline> {
    Ok(novel_files::parse_text(
        &app,
        &id,
        &volume_pattern,
        &chapter_pattern,
    )?)
}

/// 读取已落盘的识别大纲：缺失、损坏或版本不符回落空，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_content_outline(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<ParseOutline>> {
    Ok(novel_files::content_outline(&app, &id)?)
}

/// 强制重排编号：按文档顺序重写卷号与章号后落盘，返回新大纲。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn renumber_novel_content(app: tauri::AppHandle, id: String) -> CommandResult<ParseOutline> {
    Ok(novel_files::renumber_content(&app, &id)?)
}
