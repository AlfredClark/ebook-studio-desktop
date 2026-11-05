//! 小说导出命令：导出状态探针与改名落盘的薄封装，
//! 具体导出与文件生成在 `features::novel_export`。

use crate::cores::types::CommandResult;
use crate::features::novel_export::{
    self, ExportItems, ExportNaming, ExportOutcome, ExportStatus, TxtOptions, TxtTitleKind,
};

/// 查询导出状态：各产物有无一次查全，前端据此渲染表单与禁用态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_export_status(app: tauri::AppHandle, id: String) -> CommandResult<ExportStatus> {
    Ok(novel_export::export_status(&app, &id)?)
}

/// 预览标题格式：取首章渲染一行供下拉后方展示，失败时前端隐藏预览。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_novel_txt_title(
    app: tauri::AppHandle,
    id: String,
    kind: TxtTitleKind,
    pattern: String,
) -> CommandResult<String> {
    Ok(novel_export::preview_title(&app, &id, kind, &pattern)?)
}

/// 导出小说：改名拷贝 EPUB / 封面、`content.json` 转 TXT，落到自选目录的 `<命名>/` 下。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn export_novel_content(
    app: tauri::AppHandle,
    id: String,
    dest_dir: String,
    naming: ExportNaming,
    custom_name: String,
    items: ExportItems,
    txt_options: TxtOptions,
) -> CommandResult<ExportOutcome> {
    Ok(novel_export::export_novel(
        &app,
        &id,
        &dest_dir,
        naming,
        &custom_name,
        &items,
        &txt_options,
    )?)
}
