//! 小说导出命令：导出状态探针与改名落盘的薄封装，
//! 具体导出与文件生成在 `features::novel_export`。

use crate::cores::types::CommandResult;
use crate::features::novel_export::{self, ExportItems, ExportNaming, ExportOutcome, ExportStatus};

/// 查询导出状态：各产物有无一次查全，前端据此渲染表单与禁用态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_export_status(app: tauri::AppHandle, id: String) -> CommandResult<ExportStatus> {
    Ok(novel_export::export_status(&app, &id)?)
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
) -> CommandResult<ExportOutcome> {
    Ok(novel_export::export_novel(
        &app,
        &id,
        &dest_dir,
        naming,
        &custom_name,
        &items,
    )?)
}
