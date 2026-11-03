//! 元数据命令：`metadata.json` 读写与封面导入的薄封装，具体文件操作在 `features::project_metadata`。

use crate::cores::types::CommandResult;
use crate::features::project_metadata::{self, ProjectMetadata, ProjectOverview};

/// 读取项目元数据；文件缺失时按项目信息新建并落盘后返回。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_project_metadata(app: tauri::AppHandle, id: String) -> CommandResult<ProjectMetadata> {
    Ok(project_metadata::load_or_init_metadata(&app, &id)?)
}

/// 保存项目元数据：服务端字段（标识 / 版本 / 小说语言 / 存量封面）一律回填，返回写后的完整内容。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_project_metadata(
    app: tauri::AppHandle,
    id: String,
    metadata: ProjectMetadata,
) -> CommandResult<ProjectMetadata> {
    Ok(project_metadata::save_metadata(&app, &id, metadata)?)
}

/// 重置项目元数据：删除 `metadata.json` 后按项目信息重建默认值，返回新建内容。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn reset_project_metadata(app: tauri::AppHandle, id: String) -> CommandResult<ProjectMetadata> {
    Ok(project_metadata::reset_metadata(&app, &id)?)
}

/// 导入封面：把用户选取的图片复制为 `sources/cover.<ext>` 并回写 `cover` 字段，返回更新后的完整元数据。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn import_project_cover(
    app: tauri::AppHandle,
    id: String,
    source_path: String,
) -> CommandResult<ProjectMetadata> {
    Ok(project_metadata::import_cover(&app, &id, &source_path)?)
}

/// 查询封面绝对路径：供前端转 `asset:` 预览 URL；未设置或文件缺失返回 `None`。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_project_cover_path(app: tauri::AppHandle, id: String) -> CommandResult<Option<String>> {
    Ok(project_metadata::cover_absolute_path(&app, &id)?)
}

/// 读取项目概览：详情页的只读快照，无元数据时回落空，不建文件。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_project_overview(app: tauri::AppHandle, id: String) -> CommandResult<ProjectOverview> {
    Ok(project_metadata::overview(&app, &id)?)
}
