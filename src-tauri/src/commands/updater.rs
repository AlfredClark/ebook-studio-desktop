//! 更新命令：检查 / 下载安装 / 重启的薄封装，业务在 `cores::updater`。

use crate::cores::types::CommandResult;
use crate::cores::updater::{self, UpdateInfo};

/// 检查更新；无新版返回 `None`（仅桌面端有更新能力）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn check_update(app: tauri::AppHandle) -> CommandResult<Option<UpdateInfo>> {
    Ok(updater::check_for_update(&app).await?)
}

/// 下载并安装更新，进度经 `app-updater-progress` 事件推送；
/// 装完不自动重启，由前端手动触发（仅桌面端有更新能力）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn download_and_install_update(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(updater::download_and_install(&app).await?)
}

/// 重启应用以完成更新；桌面端本命令不返回，前端调用时不要 `await`
/// （移动端返回错误，前端经 `.failed()` 处理）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn restart_app(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(updater::restart(&app)?)
}
