//! 漫画导出命令：状态探针、改名落盘与原图文件夹 / 压缩包导出的薄封装，具体逻辑在 `features::comic_export`。

use crate::cores::types::CommandResult;
use crate::features::comic_export::{self, ComicExportItems, ComicExportStatus, ComicImagesMode};
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

/// 导出漫画：改名拷贝 EPUB / 封面、原图按话建目录拷贝或打成压缩包，落到自选目录的 `<命名>/` 下；
/// 阻塞池执行并推送原图落盘进度，前端据此渲染进度条。
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
        images_mode,
    )
    .await?)
}
