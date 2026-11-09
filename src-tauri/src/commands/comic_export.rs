//! 漫画导出命令：状态探针、改名落盘与原图文件夹 / 压缩包导出的薄封装，具体逻辑在 `features::comic_export`。

use crate::cores::types::CommandResult;
use crate::features::comic_export::{
    self, ComicExportItems, ComicExportState, ComicExportStatus, ComicImagesMode,
};
use crate::features::comic_pdf::ComicPdfOptions;
use crate::features::novel_export::{BuildTitleOptions, ExportNaming, ExportOutcome};

/// 查询导出状态：各产物有无一次查全，前端据此渲染表单与禁用态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_export_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ComicExportStatus> {
    Ok(comic_export::export_status(&app, &id)?)
}

/// 查询导出作业状态：切回页面时回填进度；从未开始返回空闲零值。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_export_progress(id: String) -> CommandResult<ComicExportState> {
    Ok(comic_export::export_progress_status(&id))
}

/// 导出漫画：改名拷贝 EPUB / 封面、按 content.json 顺序输出 PDF 或原图，落到自选目录；
/// 阻塞池执行并推送文件 / PDF 页处理进度，前端据此渲染进度条。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub async fn export_comic_content(
    app: tauri::AppHandle,
    id: String,
    dest_dir: String,
    naming: ExportNaming,
    custom_name: String,
    items: ComicExportItems,
    titles: BuildTitleOptions,
    pdf_options: ComicPdfOptions,
    images_mode: ComicImagesMode,
) -> CommandResult<ExportOutcome> {
    Ok(comic_export::export_comic(
        &app,
        &id,
        &dest_dir,
        naming,
        &custom_name,
        &items,
        &titles,
        &pdf_options,
        images_mode,
    )
    .await?)
}
