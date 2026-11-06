//! 漫画文件命令：`sources/images/` 目录导入、状态查询与顺序解析的薄封装，具体文件操作在 `features::comic_files`。

use crate::cores::types::CommandResult;
use crate::features::comic_files::{self, ComicImageInfo, ComicImagesStatus, ComicOutline};

/// 查询漫画原稿状态；缺目录回落空态，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_images_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicImagesStatus> {
    Ok(comic_files::images_status(&app, &id)?)
}

/// 导入漫画目录：整体复制为 `sources/images`，返回最新状态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn import_comic_images(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<ComicImagesStatus> {
    Ok(comic_files::import_images(&app, &id, &source_path)?)
}

/// 解析漫画顺序：扫描 `sources/images` 后落盘 `content.json`，返回识别大纲（图片二进制不出 IPC）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn parse_comic_images(app: tauri::AppHandle, id: String) -> CommandResult<ComicOutline> {
    Ok(comic_files::parse_images(&app, &id)?)
}

/// 读取已落盘的识别大纲：缺失、损坏或版本不符回落空，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_content_outline(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<ComicOutline>> {
    Ok(comic_files::content_outline(&app, &id)?)
}

/// 查询漫画图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示；非图片或缺失直接报错。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_image_path(
    app: tauri::AppHandle,
    id: String,
    path: String,
) -> CommandResult<ComicImageInfo> {
    Ok(comic_files::image_path(&app, &id, &path)?)
}
