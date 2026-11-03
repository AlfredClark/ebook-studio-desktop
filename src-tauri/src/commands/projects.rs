//! 项目命令：列表 / 新建 / 批量删除的薄封装，具体文件操作在 `features::projects`。

use crate::cores::types::CommandResult;
use crate::features::projects::{self, ProjectInfo, ProjectType};

/// 读取全部项目；根目录不存在视为尚无项目，返回空列表
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn list_projects(app: tauri::AppHandle) -> CommandResult<Vec<ProjectInfo>> {
    Ok(projects::list_projects(&app)?)
}

/// 新建项目：在 `Projects/` 下以 `UUID` 建目录并落盘 `project.json`，返回完整信息
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn create_project(
    app: tauri::AppHandle,
    name: String,
    project_type: ProjectType,
) -> CommandResult<ProjectInfo> {
    Ok(projects::create_project(&app, &name, project_type)?)
}

/// 重命名项目：校验展示名后回写 `project.json` 并刷新更新时间，返回更新后的信息
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn rename_project(
    app: tauri::AppHandle,
    id: String,
    name: String,
) -> CommandResult<ProjectInfo> {
    Ok(projects::rename_project(&app, &id, &name)?)
}

/// 批量删除项目：永久递归删除对应目录，不存在的 id 跳过，返回实际删除数
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn delete_projects(app: tauri::AppHandle, ids: Vec<String>) -> CommandResult<u32> {
    Ok(projects::delete_projects(&app, &ids)?)
}
