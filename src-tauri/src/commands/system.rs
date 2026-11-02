//! 系统命令：系统信息查询与应用退出的薄封装。

use crate::cores::system::{self, SystemInfo};
use crate::cores::types::CommandResult;

/// 采集运行平台信息；单项缺失时回落 `"unknown"`，绝不抛错
#[tauri::command]
#[specta::specta]
pub fn get_system_info() -> CommandResult<SystemInfo> {
    Ok(system::gather_system_info())
}

/// 真退出应用进程；调用后进程结束，结果体永不可达（按 `CommandResult<()>` 保持命令类型统一）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn quit_app(app: tauri::AppHandle) -> CommandResult<()> {
    system::quit_app(&app);
    Ok(())
}

/// 在系统文件管理器中打开日志目录
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn open_log_dir(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(system::open_log_dir(&app)?)
}

/// 在系统文件管理器中打开配置目录（`config.json` 所在目录）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn open_config_dir(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(system::open_config_dir(&app)?)
}

/// 复制系统信息到剪贴板；文本由后端组装，前端只调命令
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn copy_system_info(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(system::copy_system_info(&app)?)
}
