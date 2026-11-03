//! 项目管理：`APPDATA/Projects/<uuid>/project.json` 的增删查。
//!
//! 文件夹名即项目 id（`UUID v4`，天然唯一且杜绝路径穿越拼接风险）；项目展示名只进
//! `project.json`、允许重名，从不参与文件系统路径。核心函数以 `&Path` 为参，
//! `AppHandle` 只做根目录解析，便于无 Tauri 运行时单测。

use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// 项目类型：落盘与契约为小写字符串，前后端共用 `"novel" | "comic"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Novel,
    Comic,
}

/// 项目基本信息：`project.json` 的落盘格式即前端契约，时间全链路 UTC。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProjectInfo {
    /// 项目 id，与文件夹名一致（`UUID v4` 字符串）
    pub id: String,
    /// 项目展示名，允许重名，不参与路径
    pub name: String,
    /// 项目类型；`type` 是 Rust 关键字，落盘与契约仍用 `type`
    #[serde(rename = "type")]
    pub project_type: ProjectType,
    /// 创建时间（UTC `RFC3339`）
    pub created_at: String,
    /// 更新时间（UTC `RFC3339`，内容编辑时刷新，创建时与创建时间一致）
    pub updated_at: String,
}

/// 项目名称长度上限：名称不进文件系统，仅防异常长串撑爆展示与落盘。
const MAX_NAME_LEN: usize = 100;

/// 解析项目根目录并确保存在：`APPDATA/Projects/`。
fn resolve_projects_root(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    use tauri::Manager;

    let root = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data dir")?
        .join("Projects");
    std::fs::create_dir_all(&root).context("failed to create projects dir")?;
    Ok(root)
}

/// 校验展示名：非空（去首尾空白后）且不超长；名称不参与路径，无需字符黑名单。
fn sanitize_name(name: &str) -> anyhow::Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("project name must not be empty");
    }
    if trimmed.chars().count() > MAX_NAME_LEN {
        anyhow::bail!("project name is too long (max {MAX_NAME_LEN} chars)");
    }
    Ok(trimmed.to_string())
}

/// 列出全部项目：逐个解析子目录的 `project.json`，损坏条目跳过并记警告，
/// 不因单个坏文件夹拖死整个列表；返回顺序即文件系统顺序，排序由前端负责。
pub fn list_projects_in(root: &Path) -> anyhow::Result<Vec<ProjectInfo>> {
    let mut projects = Vec::new();
    let entries = std::fs::read_dir(root)
        .with_context(|| format!("failed to read projects dir: {}", root.display()))?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("跳过不可读的项目条目: {err:#}");
                continue;
            }
        };
        let manifest = entry.path().join("project.json");
        if !manifest.is_file() {
            continue;
        }
        match read_manifest(&manifest) {
            Ok(info) => projects.push(info),
            Err(err) => log::warn!("跳过损坏的项目清单 {}: {err:#}", manifest.display()),
        }
    }
    Ok(projects)
}

/// 读取并解析单个 `project.json`。
fn read_manifest(manifest: &Path) -> anyhow::Result<ProjectInfo> {
    let text = std::fs::read_to_string(manifest)
        .with_context(|| format!("failed to read manifest: {}", manifest.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse manifest: {}", manifest.display()))
}

/// 读取全部项目；根目录不存在视为尚无项目，返回空列表而非报错。
pub fn list_projects(app: &tauri::AppHandle) -> anyhow::Result<Vec<ProjectInfo>> {
    let root = resolve_projects_root(app)?;
    list_projects_in(&root)
}

/// 新建项目：生成 `UUID` 建目录并落盘 `project.json`，返回完整信息。
pub fn create_project_in(
    root: &Path,
    name: &str,
    project_type: ProjectType,
) -> anyhow::Result<ProjectInfo> {
    let name = sanitize_name(name)?;
    let id = uuid::Uuid::new_v4().to_string();
    let dir = root.join(&id);
    if dir.exists() {
        anyhow::bail!("project id collision, please retry");
    }
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create project dir: {}", dir.display()))?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let info = ProjectInfo {
        id,
        name,
        project_type,
        created_at: now.clone(),
        updated_at: now,
    };
    let text = serde_json::to_string_pretty(&info).context("failed to serialize project")?;
    std::fs::write(dir.join("project.json"), text).context("failed to write project.json")?;
    Ok(info)
}

/// 新建项目（`AppHandle` 入口）。
pub fn create_project(
    app: &tauri::AppHandle,
    name: &str,
    project_type: ProjectType,
) -> anyhow::Result<ProjectInfo> {
    let root = resolve_projects_root(app)?;
    create_project_in(&root, name, project_type)
}

/// 重命名项目：校验展示名并刷新更新时间后回写 `project.json`，返回更新后的信息；
/// 清单缺失视为目标不存在，直接报错（显式重命名不做静默跳过）。
pub fn rename_project_in(root: &Path, id: &str, name: &str) -> anyhow::Result<ProjectInfo> {
    uuid::Uuid::parse_str(id).with_context(|| format!("invalid project id: {id}"))?;
    let name = sanitize_name(name)?;
    let manifest = root.join(id).join("project.json");
    let mut info = read_manifest(&manifest)?;
    info.name = name;
    info.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let text = serde_json::to_string_pretty(&info).context("failed to serialize project")?;
    std::fs::write(&manifest, text)
        .with_context(|| format!("failed to write project.json: {}", manifest.display()))?;
    Ok(info)
}

/// 重命名项目（`AppHandle` 入口）。
pub fn rename_project(app: &tauri::AppHandle, id: &str, name: &str) -> anyhow::Result<ProjectInfo> {
    let root = resolve_projects_root(app)?;
    rename_project_in(&root, id, name)
}
/// 批量删除项目：id 必须为合法 `UUID`（兼作路径穿越守卫，`..` 与分隔符一律拒绝）；
/// 不存在的目录跳过（删除幂等），返回实际删除数。
pub fn delete_projects_in(root: &Path, ids: &[String]) -> anyhow::Result<u32> {
    let mut deleted = 0;
    for id in ids {
        uuid::Uuid::parse_str(id).with_context(|| format!("invalid project id: {id}"))?;
        let dir = root.join(id);
        if !dir.exists() {
            continue;
        }
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("failed to delete project dir: {}", dir.display()))?;
        deleted += 1;
    }
    Ok(deleted)
}

/// 批量删除项目（`AppHandle` 入口），返回实际删除数。
pub fn delete_projects(app: &tauri::AppHandle, ids: &[String]) -> anyhow::Result<u32> {
    let root = resolve_projects_root(app)?;
    delete_projects_in(&root, ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用例隔离的临时根目录：`temp_dir` 下按进程与用例名拼路径，跑完清理。
    fn test_root(case: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-projects-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        root
    }

    #[test]
    fn rejects_blank_and_oversize_names() {
        let root = test_root("names");
        assert!(create_project_in(&root, "   ", ProjectType::Novel).is_err());
        assert!(create_project_in(&root, "", ProjectType::Comic).is_err());
        assert!(create_project_in(&root, &"名".repeat(101), ProjectType::Novel).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn creates_lists_and_deletes_projects() {
        let root = test_root("crud");
        let novel = create_project_in(&root, "星海长歌", ProjectType::Novel).expect("create novel");
        let comic = create_project_in(&root, "雾镇", ProjectType::Comic).expect("create comic");
        assert!(uuid::Uuid::parse_str(&novel.id).is_ok());
        assert!(root.join(&novel.id).join("project.json").is_file());
        assert_eq!(novel.created_at, novel.updated_at);

        let mut listed = list_projects_in(&root).expect("list");
        listed.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "星海长歌");

        let deleted = delete_projects_in(&root, &[novel.id, comic.id]).expect("delete");
        assert_eq!(deleted, 2);
        assert!(
            list_projects_in(&root).expect("relist").is_empty(),
            "expected empty list"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn renames_project_and_bumps_updated_at() {
        let root = test_root("rename");
        let created = create_project_in(&root, "雾镇", ProjectType::Comic).expect("create");
        std::thread::sleep(std::time::Duration::from_secs(1));
        let renamed = rename_project_in(&root, &created.id, "雾镇·改").expect("rename");
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.name, "雾镇·改");
        assert_eq!(renamed.created_at, created.created_at);
        assert!(renamed.updated_at > created.updated_at);
        let listed = list_projects_in(&root).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "雾镇·改");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_bad_rename_requests() {
        let root = test_root("rename-bad");
        let created = create_project_in(&root, "归墟", ProjectType::Novel).expect("create");
        assert!(rename_project_in(&root, &created.id, "   ").is_err());
        assert!(rename_project_in(&root, &created.id, &"名".repeat(101)).is_err());
        assert!(rename_project_in(&root, "..", "雾镇").is_err());
        assert!(rename_project_in(&root, &uuid::Uuid::new_v4().to_string(), "雾镇").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_path_traversal_ids_and_skips_missing() {
        let root = test_root("traversal");
        assert!(delete_projects_in(&root, &["..".to_string()]).is_err());
        assert!(delete_projects_in(&root, &["a/b".to_string()]).is_err());
        assert!(delete_projects_in(&root, &["not-a-uuid".to_string()]).is_err());
        let deleted = delete_projects_in(&root, &[uuid::Uuid::new_v4().to_string()])
            .expect("missing is skipped");
        assert_eq!(deleted, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_corrupt_manifests_without_failing_list() {
        let root = test_root("corrupt");
        let kept = create_project_in(&root, "归墟", ProjectType::Novel).expect("create");
        let bad_dir = root.join("not-a-project");
        std::fs::create_dir_all(&bad_dir).expect("create stray dir");
        std::fs::write(bad_dir.join("project.json"), "{ broken").expect("write broken json");
        let listed = list_projects_in(&root).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, kept.id);
        let _ = std::fs::remove_dir_all(&root);
    }
}
