//! 漫画构建命令：`build/` 目录树的生成、列举与单文件读写回存的薄封装。
//!
//! 生成逻辑在 `features::comic_build`，树与文件的读写、格式化直接复用
//! `features::novel_build` 的通用实现（与项目类型无关的 `build/` 操作）。

use crate::cores::types::CommandResult;
use crate::features::comic_build;
use crate::features::novel_build::{
    self, BuildFileContent, BuildImageInfo, BuildOutcome, FormatOutcome,
};
use crate::features::novel_export::{BuildTitleOptions, ChapterNumbering, TxtTitleKind};

/// 构建漫画：清空构建目录后全量重生成，返回文件树。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn build_comic_content(
    app: tauri::AppHandle,
    id: String,
    titles: BuildTitleOptions,
) -> CommandResult<BuildOutcome> {
    Ok(comic_build::build_comic(&app, &id, &titles)?)
}

/// 清空构建产物：删除 `build/` 与打包目录 `dist/`，前端回到构建引导，
/// 下游打包与导出页回到空态；目录不存在视为成功。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn clear_comic_build_content(app: tauri::AppHandle, id: String) -> CommandResult<()> {
    Ok(comic_build::clear_comic(&app, &id)?)
}

/// 预览标题格式：按话取首个标题渲染一行供下拉后方展示，失败时前端隐藏预览。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_comic_txt_title(
    app: tauri::AppHandle,
    id: String,
    kind: TxtTitleKind,
    pattern: String,
    is_volume: bool,
    numbering: ChapterNumbering,
) -> CommandResult<String> {
    Ok(comic_build::preview_title(
        &app, &id, kind, &pattern, is_volume, numbering,
    )?)
}

/// 读取构建树：未构建回 `None`，前端展示空态引导构建。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_build_tree(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<BuildOutcome>> {
    Ok(novel_build::build_tree(&app, &id)?)
}

/// 读取构建文件：二进制回 `content: None`，固定文件标记不可编辑。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_build_file(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<BuildFileContent> {
    Ok(novel_build::build_file(&app, &id, &path)?)
}

/// 查询构建图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示；非图片或缺失直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_build_image_path(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<BuildImageInfo> {
    Ok(novel_build::build_image_path(&app, &id, &path)?)
}

/// 保存构建文件：固定文件与二进制拒绝写入。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_comic_build_file(
    app: tauri::AppHandle,
    id: String,
    path: String,
    content: String,
) -> CommandResult<()> {
    Ok(novel_build::write_build_file(&app, &id, &path, &content)?)
}

/// 格式化编辑器文本：纯内存返回格式化文本，不读盘不落盘；路径仅作可格式化校验。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn format_comic_build_text(
    app: tauri::AppHandle,
    id: String,
    path: String,
    content: String,
) -> CommandResult<String> {
    Ok(novel_build::format_build_text(&app, &id, &path, &content)?)
}

/// 批量格式化构建文件：逐个落盘，单个失败不中断。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn format_all_comic_build_files(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<FormatOutcome> {
    Ok(novel_build::format_all_build_files(&app, &id)?)
}

/// 打开构建目录：实现复用小说构建页的通用逻辑。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn open_comic_build_dir(app: tauri::AppHandle, id: String) -> CommandResult<()> {
    Ok(novel_build::open_build_dir(&app, &id)?)
}

/// 打开构建文件所在目录：实现复用小说构建页的通用逻辑。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn open_comic_build_file_dir(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<()> {
    Ok(novel_build::open_build_file_dir(&app, &id, &path)?)
}
