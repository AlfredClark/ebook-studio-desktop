//! EPUB 模板只读访问：定位打包进应用的两套 EPUB3.3 模板骨架并按相对路径读取。
//!
//! 打包声明见 `tauri.conf.json` 的 `bundle.resources`；运行时主路径为
//! `resource_dir/templates/<模板名>`，`tauri dev` 与 `cargo test` 下回落到
//! `CARGO_MANIFEST_DIR/templates/<模板名>`。核心函数以 `&Path` 为参，
//! `AppHandle` 只做根目录解析，便于无 Tauri 运行时单测（模式同 `projects`）。
//!
//! 本模块只读不写：模板套用到项目目录的时机与落盘位置待后续业务确定后再实现。
//!
//! Phase-1 暂无命令层调用，公开 API 的 `dead_code` 豁免待套用阶段由 `commands` 接线后移除。

#![allow(dead_code)]

use std::path::{Component, Path, PathBuf};

use anyhow::Context;

use super::projects::ProjectType;

/// 模板目录名：`ProjectType` 到打包目录的一一映射。
const fn template_dir_name(kind: ProjectType) -> &'static str {
    match kind {
        ProjectType::Novel => "EPUB33-NOVEL",
        ProjectType::Comic => "EPUB33-COMIC",
    }
}

/// 在候选根目录下定位模板目录：依次拼接 `templates/<模板名>`，命中首个存在的目录。
fn find_template_dir_in(roots: &[PathBuf], kind: ProjectType) -> anyhow::Result<PathBuf> {
    let name = template_dir_name(kind);
    for root in roots {
        let candidate = root.join("templates").join(name);
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }
    anyhow::bail!("epub template not found: templates/{name}");
}

/// 定位模板目录：先找打包资源目录，再回落到源码目录（dev 与单测兜底）。
pub fn template_dir(app: &tauri::AppHandle, kind: ProjectType) -> anyhow::Result<PathBuf> {
    use tauri::Manager;

    let resource_root = app
        .path()
        .resource_dir()
        .context("failed to resolve resource dir")?;
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    find_template_dir_in(&[resource_root, manifest_root], kind)
}

/// 解析模板内的相对路径：拒绝绝对路径与 `..` / 前缀等非普通组件（路径穿越守卫）。
fn resolve_relative(base: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        anyhow::bail!("template path must be relative: {rel}");
    }
    for component in rel_path.components() {
        if !matches!(component, Component::Normal(_)) {
            anyhow::bail!("invalid template path: {rel}");
        }
    }
    Ok(base.join(rel_path))
}

/// 按相对路径读取模板文本文件（模板全为 `UTF-8` 文本）。
pub fn read_template_file_in(base: &Path, rel: &str) -> anyhow::Result<String> {
    let path = resolve_relative(base, rel)?;
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read template file: {}", path.display()))
}

/// 按相对路径读取模板二进制文件（后续图片等非文本资源预留）。
pub fn read_template_bytes_in(base: &Path, rel: &str) -> anyhow::Result<Vec<u8>> {
    let path = resolve_relative(base, rel)?;
    std::fs::read(&path)
        .with_context(|| format!("failed to read template file: {}", path.display()))
}

/// 按相对路径读取模板文本文件（`AppHandle` 入口）。
pub fn read_template_file(
    app: &tauri::AppHandle,
    kind: ProjectType,
    rel: &str,
) -> anyhow::Result<String> {
    let base = template_dir(app, kind)?;
    read_template_file_in(&base, rel)
}

/// 校验模板骨架完整性：`mimetype` 内容 + `container.xml` + `content.opf` 三件套缺一不可。
pub fn validate_template_in(base: &Path) -> anyhow::Result<()> {
    let mimetype = read_template_file_in(base, "mimetype")?;
    if mimetype.trim() != "application/epub+zip" {
        anyhow::bail!("invalid template mimetype: {}", mimetype.trim());
    }
    for required in ["META-INF/container.xml", "EPUB/content.opf"] {
        let path = resolve_relative(base, required)?;
        if !path.is_file() {
            anyhow::bail!("missing template file: {required}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 源码目录下的模板实物：`cargo test` 的工作目录即 `src-tauri/`，亦可用 `CARGO_MANIFEST_DIR`。
    fn fixture_root(kind: ProjectType) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("templates")
            .join(template_dir_name(kind))
    }

    #[test]
    fn validates_bundled_templates() {
        for kind in [ProjectType::Novel, ProjectType::Comic] {
            let base = fixture_root(kind);
            validate_template_in(&base).expect("bundled template must be valid");
        }
    }

    #[test]
    fn reads_template_text_with_placeholders_intact() {
        let novel = fixture_root(ProjectType::Novel);
        let nav = read_template_file_in(&novel, "EPUB/nav.xhtml").expect("read novel nav");
        assert!(
            nav.contains("{{CHAPTER_TOC_LIST}}"),
            "structural placeholders must stay intact until apply phase"
        );
        let comic = fixture_root(ProjectType::Comic);
        let page = read_template_file_in(&comic, "EPUB/pages/page-template.xhtml")
            .expect("read comic page");
        assert!(page.contains("{{PAGE_IMAGE_HREF}}"));
    }

    #[test]
    fn rejects_path_traversal_and_absolute_paths() {
        let base = fixture_root(ProjectType::Novel);
        assert!(read_template_file_in(&base, "../Cargo.toml").is_err());
        assert!(read_template_file_in(&base, "EPUB/../../Cargo.toml").is_err());
        assert!(read_template_file_in(&base, "/etc/hostname").is_err());
        assert!(read_template_file_in(&base, "").is_err());
    }

    #[test]
    fn rejects_incomplete_template_dir() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-template-test-{}-incomplete",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("mimetype"), "application/epub+zip").expect("write mimetype");
        assert!(validate_template_in(&dir).is_err(), "missing opf must fail");

        std::fs::write(dir.join("mimetype"), "text/plain").expect("write bad mimetype");
        std::fs::create_dir_all(dir.join("META-INF")).expect("create meta-inf");
        std::fs::write(dir.join("META-INF/container.xml"), "<container/>").expect("write stub");
        std::fs::create_dir_all(dir.join("EPUB")).expect("create epub");
        std::fs::write(dir.join("EPUB/content.opf"), "<package/>").expect("write stub");
        assert!(
            validate_template_in(&dir).is_err(),
            "bad mimetype must fail"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefers_first_existing_root() {
        let missing = PathBuf::from("/definitely/not/here");
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let found = find_template_dir_in(&[missing.clone(), manifest], ProjectType::Comic)
            .expect("fallback root must hit");
        assert_eq!(found, fixture_root(ProjectType::Comic));
        assert!(find_template_dir_in(&[missing], ProjectType::Novel).is_err());
    }
}
