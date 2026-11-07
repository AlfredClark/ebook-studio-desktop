//! 小说打包命令：`build/` → `dist/书名.epub` 的落盘、状态查询与 `epubveri` 校验的薄封装，
//! 具体打包与校验逻辑在 `features::novel_package`。

use crate::cores::types::CommandResult;
use crate::features::novel_package::{
    self, PackageInfo, PackageOutcome, PackagingStatus, ValidationOutcome,
};

/// 打包小说：落盘 `dist/书名.epub` 后自动校验一次，进度经 `novel-package-progress` 推送。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn package_novel_content(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<PackageOutcome> {
    Ok(novel_package::package_novel(&app, &id).await?)
}

/// 查询打包状态：无产物回 `None`，前端展示待打包空态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_package_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<PackageInfo>> {
    Ok(novel_package::package_status(&app, &id)?)
}

/// 查询打包作业状态：切回页面时回填进度；从未开始返回空闲零值。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_novel_package_progress(id: String) -> CommandResult<PackagingStatus> {
    Ok(novel_package::packaging_status(&id))
}

/// 校验已有产物：无包直接报错，前端引导先打包。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn validate_novel_package(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ValidationOutcome> {
    Ok(novel_package::validate_package(&app, &id).await?)
}
