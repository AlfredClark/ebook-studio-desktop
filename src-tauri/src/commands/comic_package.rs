//! 漫画打包命令：`build/` → `dist/书名.epub` 的落盘、状态查询与 `epubveri` 校验的薄封装。
//!
//! 打包与校验逻辑复用 `features::novel_package` 的通用实现（与项目类型无关），
//! 本层只做漫画入口与进度事件名的接线。

use crate::cores::types::CommandResult;
use crate::features::comic_package;
use crate::features::novel_package::{
    self, PackageInfo, PackageOutcome, PackagingStatus, ValidationOutcome,
};

/// 打包漫画：落盘 `dist/书名.epub` 后自动校验一次，进度经 `comic-package-progress` 推送。
/// 事件名唯一来源是 `features::comic_package::PACKAGE_PROGRESS_EVENT`，改名时前端订阅同步改。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn package_comic_content(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<PackageOutcome> {
    Ok(comic_package::package_comic(&app, &id).await?)
}

/// 查询打包状态：无产物回 `None`，前端展示待打包空态。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_package_status(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<Option<PackageInfo>> {
    Ok(novel_package::package_status(&app, &id)?)
}

/// 查询打包作业状态：切回页面时回填进度；从未开始返回空闲零值。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_comic_package_progress(id: String) -> CommandResult<PackagingStatus> {
    Ok(novel_package::packaging_status(&id))
}

/// 校验已有产物：无包直接报错，前端引导先打包。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn validate_comic_package(
    app: tauri::AppHandle,
    id: String,
) -> CommandResult<ValidationOutcome> {
    Ok(novel_package::validate_package(&app, &id).await?)
}

/// 打开打包目录：实现复用小说打包页的通用逻辑。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn open_comic_package_dir(app: tauri::AppHandle, id: String) -> CommandResult<()> {
    Ok(novel_package::open_dist_dir(&app, &id)?)
}
