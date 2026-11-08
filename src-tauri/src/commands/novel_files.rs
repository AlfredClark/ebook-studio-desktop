//! 小说原文命令：`sources/content.txt` 导入、状态查询、卷章解析与编号重排的薄封装，具体文件操作在 `features::novel_files`。

use crate::cores::types::CommandResult;
use crate::features::novel_files::{
    self, ChapterContent, FilterOutcome, NovelFilter, NovelImportOutcome, NovelTextStatus,
    ParseOutline,
};

/// 查询小说原文状态；缺文件回落空态，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_text_status(app: tauri::AppHandle, id: String) -> CommandResult<NovelTextStatus> {
    Ok(novel_files::text_status(&app, &id)?)
}

/// 导入小说原文：把用户选取的 TXT 解码后覆盖为 `sources/content.txt`，返回最新状态与来源编码。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn import_novel_text(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<NovelImportOutcome> {
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

/// 撤销重排：用备份恢复 `content.json`，一次性有效，无备份直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn restore_novel_content_backup(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ParseOutline> {
    Ok(novel_files::restore_backup(&app, &id)?)
}

/// 读取单章正文：按卷章下标寻址，标题只读、正文按行返回，越界或产物异常直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_chapter(
    app: tauri::AppHandle,
    id: String,
    volume_index: u32,
    chapter_index: u32,
) -> CommandResult<ChapterContent> {
    Ok(novel_files::chapter_content(
        &app,
        &id,
        volume_index,
        chapter_index,
    )?)
}

/// 保存单章正文：归一化落盘后返回新大纲（左栏字数即时刷新）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_novel_chapter(
    app: tauri::AppHandle,
    id: String,
    volume_index: u32,
    chapter_index: u32,
    lines: Vec<String>,
) -> CommandResult<ParseOutline> {
    Ok(novel_files::update_chapter(
        &app,
        &id,
        volume_index,
        chapter_index,
        lines,
    )?)
}

/// 全书筛选：按筛选种类与参数扫描各章正文，返回命中清单（正文行不出 IPC，只出命中位置）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn filter_novel_content(
    app: tauri::AppHandle,
    id: String,
    filter: NovelFilter,
) -> CommandResult<FilterOutcome> {
    Ok(novel_files::filter_content(&app, &id, &filter)?)
}
