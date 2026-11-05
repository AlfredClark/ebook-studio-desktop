//! 小说导出：`content.json` → 标准 TXT、`dist/*.epub` 与封面的改名拷贝，
//! 落到用户自选目录下的 `<命名>/` 子目录。
//!
//! 目标目录由前端目录选择框给出（后端只认存在目录）；目标子目录已存在且非空
//! 直接报错，不动用户旧文件。落盘格式即前端契约（`snake_case`）。

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::novel_files::NovelContent;

/// 解析产物摘要：存在性之外的展示信息。
///
/// 字段名与 `ParseOutline` 同口径（`struct_field_names` 豁免）。
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ContentSummary {
    /// 卷数（含无标题卷）。
    pub volume_count: u32,
    /// 全书章节总数。
    pub chapter_count: u32,
    /// 正文字数：仅各章 `lines` 字符数之和（不计换行符）。
    pub char_count: u32,
}

/// 导出状态：进页一次查全，前端据此渲染表单与禁用态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportStatus {
    /// 是否有可导出的解析产物。
    pub has_content: bool,
    /// 解析产物摘要；无产物时为 `None`。
    pub content: Option<ContentSummary>,
    /// 是否有可导出的打包产物。
    pub has_epub: bool,
    /// 打包文件名；无产物时为 `None`。
    pub epub_name: Option<String>,
    /// 是否有可导出的封面。
    pub has_cover: bool,
    /// 封面扩展名（小写）；无封面时为 `None`。
    pub cover_ext: Option<String>,
    /// 项目名（`project.json`）。
    pub project_name: String,
    /// 书名（`metadata.json`）。
    pub book_title: String,
}

/// 命名来源：项目名 / 书名 / 自定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ExportNaming {
    /// 用项目名。
    Project,
    /// 用书名。
    Book,
    /// 用用户填写的自定义名。
    Custom,
}

/// 导出勾选项：全假直接报错，至少选一项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportItems {
    /// 导出 EPUB。
    pub epub: bool,
    /// 导出标准 TXT。
    pub txt: bool,
    /// 导出封面。
    pub cover: bool,
}

/// 导出结果：最终目录绝对路径 + 落盘文件名清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportOutcome {
    /// 最终目录（`<自选目录>/<命名>/` 绝对路径）。
    pub dir: String,
    /// 落盘文件名（已排序）。
    pub files: Vec<String>,
}

/// 解析产物转摘要。
fn summarize(content: &NovelContent) -> ContentSummary {
    let chapter_count: usize = content.volumes.iter().map(|v| v.chapters.len()).sum();
    let char_count: usize = content
        .volumes
        .iter()
        .flat_map(|v| v.chapters.iter())
        .flat_map(|c| c.lines.iter())
        .map(|line| line.chars().count())
        .sum();
    ContentSummary {
        volume_count: u32::try_from(content.volumes.len()).unwrap_or(u32::MAX),
        chapter_count: u32::try_from(chapter_count).unwrap_or(u32::MAX),
        char_count: u32::try_from(char_count).unwrap_or(u32::MAX),
    }
}

/// 解析产物转标准 TXT：前言置顶，卷标题 / 卷简介 / 章标题各独行，章间空一行。
///
/// 换行一律 `LF`、无 `BOM`；隐式卷跳过卷标题行；全文无可写行直接报错。
fn build_txt(content: &NovelContent) -> anyhow::Result<String> {
    let mut blocks: Vec<String> = Vec::new();
    if !content.preface.is_empty() {
        blocks.push(content.preface.join("\n"));
    }
    for volume in &content.volumes {
        let mut block = String::new();
        if let Some(full_title) = volume.full_title.as_deref() {
            block.push_str(full_title);
            block.push('\n');
        }
        for line in &volume.intro {
            block.push_str(line);
            block.push('\n');
        }
        for chapter in &volume.chapters {
            block.push_str(&chapter.full_title);
            block.push('\n');
            for line in &chapter.lines {
                block.push_str(line);
                block.push('\n');
            }
            block.push('\n');
        }
        let block = block.trim_end().to_string();
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    if blocks.is_empty() {
        anyhow::bail!("no exportable text in content");
    }
    let mut text = blocks.join("\n\n");
    text.push('\n');
    Ok(text)
}

/// 封面扩展名（小写）：导入白名单的子集，未命中直接报错。
fn cover_ext_of(path: &Path) -> anyhow::Result<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .context("cover file has no extension")?;
    if !["jpg", "jpeg", "png", "webp", "gif"].contains(&ext.as_str()) {
        anyhow::bail!("unsupported cover type: {ext}");
    }
    Ok(ext)
}

/// 导出状态（`&Path` 入口）：只读探针，不建业务文件。
pub fn status_in(project_dir: &Path) -> anyhow::Result<ExportStatus> {
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = super::project_metadata::load_or_init_metadata_in(project_dir)?;
    let content = super::novel_files::read_content(project_dir).ok();
    let epub_name = super::novel_package::latest_package(project_dir).and_then(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    });
    let cover_ext = super::project_metadata::cover_absolute_path_in(project_dir)?
        .and_then(|absolute| cover_ext_of(Path::new(&absolute)).ok());
    Ok(ExportStatus {
        has_content: content.is_some(),
        content: content.as_ref().map(summarize),
        has_epub: epub_name.is_some(),
        epub_name,
        has_cover: cover_ext.is_some(),
        cover_ext,
        project_name: info.name,
        book_title: metadata.title,
    })
}

/// 目标子目录：`<自选目录>/<命名>/`；已存在且非空直接报错。
fn target_dir(dest: &Path, stem: &str) -> anyhow::Result<PathBuf> {
    let target = dest.join(stem);
    if target.exists() {
        if !target.is_dir() {
            anyhow::bail!("export target is not a directory");
        }
        let non_empty = std::fs::read_dir(&target)
            .with_context(|| format!("failed to read {}", target.display()))?
            .find_map(Result::ok)
            .is_some();
        if non_empty {
            anyhow::bail!("export directory is not empty");
        }
    } else {
        std::fs::create_dir_all(&target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }
    Ok(target)
}

/// 导出全量生成：改名拷贝 EPUB / 封面、`content.json` 转 TXT 落盘。
pub fn export_in(
    project_dir: &Path,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ExportItems,
) -> anyhow::Result<ExportOutcome> {
    if !items.epub && !items.txt && !items.cover {
        anyhow::bail!("no export items selected");
    }
    let dest = Path::new(dest_dir)
        .canonicalize()
        .with_context(|| format!("export directory not found: {dest_dir}"))?;
    if !dest.is_dir() {
        anyhow::bail!("export directory is not a directory: {dest_dir}");
    }
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = super::project_metadata::load_or_init_metadata_in(project_dir)?;
    let stem = match naming {
        ExportNaming::Project => super::novel_package::sanitize_stem(&info.name),
        ExportNaming::Book => super::novel_package::sanitize_stem(&metadata.title),
        ExportNaming::Custom => {
            if custom_name.trim().is_empty() {
                anyhow::bail!("custom name must not be empty");
            }
            super::novel_package::sanitize_stem(custom_name)
        }
    };
    let target = target_dir(&dest, &stem)?;

    let mut files = Vec::new();
    if items.epub {
        let source = super::novel_package::latest_package(project_dir)
            .context("no epub package to export")?;
        let name = format!("{stem}.epub");
        std::fs::copy(&source, target.join(&name))
            .with_context(|| format!("failed to copy {name}"))?;
        files.push(name);
    }
    if items.txt {
        let content = super::novel_files::read_content(project_dir)?;
        let name = format!("{stem}.txt");
        std::fs::write(target.join(&name), build_txt(&content)?)
            .with_context(|| format!("failed to write {name}"))?;
        files.push(name);
    }
    if items.cover {
        let absolute = super::project_metadata::cover_absolute_path_in(project_dir)?
            .context("no cover to export")?;
        let ext = cover_ext_of(Path::new(&absolute))?;
        let name = format!("{stem}.{ext}");
        std::fs::copy(&absolute, target.join(&name))
            .with_context(|| format!("failed to copy {name}"))?;
        files.push(name);
    }
    files.sort();
    let dir = target
        .to_str()
        .context("export dir is not valid UTF-8")?
        .to_string();
    Ok(ExportOutcome { dir, files })
}

/// 导出状态（`AppHandle` 入口）。
pub fn export_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ExportStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 导出小说（`AppHandle` 入口）。
pub fn export_novel(
    app: &tauri::AppHandle,
    id: &str,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ExportItems,
) -> anyhow::Result<ExportOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    export_in(&dir, dest_dir, naming, custom_name, items)
}

#[cfg(test)]
mod tests {
    use super::super::novel_build;
    use super::super::novel_files;
    use super::super::novel_package;
    use super::super::project_metadata;
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

    const VOLUME_PATTERN: &str =
        "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)卷[：:\\s]*(.*)$";
    const CHAPTER_PATTERN: &str =
        "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)章[：:\\s]*(.*)$";

    fn template_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("templates")
            .join("EPUB33-NOVEL")
    }

    /// 用例隔离的项目：建真实项目目录，写原文并解析，跑完清理。
    fn fixture_project(case: &str, text: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-novel-export-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let info =
            projects::create_project_in(&root, "雾镇", ProjectType::Novel).expect("create project");
        let dir = root.join(&info.id);
        let sources = dir.join("sources");
        std::fs::create_dir_all(&sources).expect("create sources");
        std::fs::write(sources.join("content.txt"), text).expect("write content");
        novel_files::parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        (root, dir)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    fn all_items() -> ExportItems {
        ExportItems {
            epub: true,
            txt: true,
            cover: true,
        }
    }

    #[test]
    fn status_reflects_available_artifacts() {
        let (root, dir) = fixture_project("status", "第一章 引子\n正文\n");
        let status = status_in(&dir).expect("status");
        assert!(status.has_content);
        assert!(!status.has_epub);
        assert!(!status.has_cover);
        assert_eq!(status.project_name, "雾镇");
        assert_eq!(status.book_title, "雾镇");
        assert_eq!(
            status.content.expect("summary"),
            ContentSummary {
                volume_count: 1,
                chapter_count: 1,
                char_count: 2,
            }
        );
        cleanup(&root);
    }

    #[test]
    fn builds_standard_txt_layout() {
        let (root, dir) = fixture_project(
            "txt",
            "卷首语\n第一卷 开篇\n卷简介\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n",
        );
        let content = super::super::novel_files::read_content(&dir).expect("read");
        let text = build_txt(&content).expect("build txt");
        assert_eq!(
            text,
            "卷首语\n\n第一卷 开篇\n卷简介\n第一章 引子\n正文甲\n\n第二章 发展\n正文乙\n"
        );
        assert!(!text.contains('\r'));
        cleanup(&root);
    }

    #[test]
    fn exports_all_files_with_book_naming() {
        let (root, dir) = fixture_project("full", "第一章 引子\n正文甲\n");
        novel_build::build_in(&dir, &template_base()).expect("build");
        novel_package::package_in(&dir).expect("package");
        let source = root.join("cover-source.png");
        std::fs::write(&source, [0x89, 0x50, 0x4E, 0x47]).expect("write fake cover");
        project_metadata::import_cover_in(&dir, &source).expect("import cover");

        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Book,
            "",
            &all_items(),
        )
        .expect("export");
        assert_eq!(
            outcome.files,
            vec![
                "雾镇.epub".to_string(),
                "雾镇.png".to_string(),
                "雾镇.txt".to_string(),
            ]
        );
        let target = dest.join("雾镇");
        assert!(target.join("雾镇.epub").is_file());
        assert!(target.join("雾镇.png").is_file());
        let txt = std::fs::read_to_string(target.join("雾镇.txt")).expect("read txt");
        assert_eq!(txt, "第一章 引子\n正文甲\n");
        assert_eq!(outcome.dir, target.to_str().expect("utf8 target"));
        cleanup(&root);
    }

    #[test]
    fn rejects_empty_items_custom_and_occupied_target() {
        let (root, dir) = fixture_project("reject", "第一章 引子\n正文\n");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let none = ExportItems {
            epub: false,
            txt: false,
            cover: false,
        };
        assert!(
            export_in(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &none
            )
            .is_err()
        );

        let txt_only = ExportItems {
            epub: false,
            txt: true,
            cover: false,
        };
        assert!(
            export_in(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Custom,
                "  ",
                &txt_only
            )
            .is_err()
        );

        // 非法字符按打包口径清洗。
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8"),
            ExportNaming::Custom,
            "a/b",
            &txt_only,
        )
        .expect("export with sanitized name");
        assert_eq!(outcome.files, vec!["a_b.txt".to_string()]);

        // 非空目录阻断，空目录复用。
        assert!(
            export_in(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Custom,
                "a/b",
                &txt_only
            )
            .is_err()
        );
        std::fs::create_dir_all(dest.join("empty")).expect("create empty");
        let reused = export_in(
            &dir,
            dest.to_str().expect("utf8"),
            ExportNaming::Custom,
            "empty",
            &txt_only,
        )
        .expect("reuse empty dir");
        assert_eq!(reused.files, vec!["empty.txt".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn rejects_missing_artifacts_per_item() {
        let (root, dir) = fixture_project("missing", "第一章 引子\n正文\n");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        // 有解析产物但无打包产物、无封面：EPUB 与封面分别报错。
        let epub_only = ExportItems {
            epub: true,
            txt: false,
            cover: false,
        };
        assert!(
            export_in(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &epub_only
            )
            .is_err()
        );
        let cover_only = ExportItems {
            epub: false,
            txt: false,
            cover: true,
        };
        assert!(
            export_in(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &cover_only
            )
            .is_err()
        );
        cleanup(&root);
    }
}
