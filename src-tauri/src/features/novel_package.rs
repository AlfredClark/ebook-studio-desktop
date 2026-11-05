//! 小说打包：`build/` 目录树 → `<uuid>/dist/书名.epub`，随后用 `epubveri` 校验。
//!
//! 打包即 OCF 容器落盘：`mimetype` 首条明存（20 字节 `application/epub+zip`，
//! 无 extra field），其余 `Deflate` 压缩，文件名 `UTF-8`；每次打包先清理 `dist/`
//! 下旧 `.epub`，只保留最新一件。落盘格式即前端契约（`snake_case`）。

use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

/// 构建产物目录：`<uuid>/build/`，打包的唯一输入。
const BUILD_DIR: &str = "build";
/// 打包交付目录：`<uuid>/dist/`，与 `sources/` / `build/` 同级。
const DIST_DIR: &str = "dist";
/// 产物扩展名：文件名恒为 `书名.epub`。
const PACKAGE_EXT: &str = "epub";
/// `mimetype` 规范内容：20 字节 ASCII，原样首条明存。
const MIMETYPE_BYTES: &[u8] = b"application/epub+zip";
/// 书名清洗后的主干长度上限（字符数，CJK 按 3 字节计仍远小于文件系统限制）。
const MAX_STEM_CHARS: usize = 100;

/// 校验严重度：`epubveri` 五级词汇原样映射，前端按此着色与计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// 致命：校验中途停止。
    Fatal,
    /// 错误：判无效。
    Error,
    /// 警告：上报但不判无效。
    Warning,
    /// 信息：中性事实。
    Info,
    /// 建议：特性使用提示，永不判无效。
    Usage,
}

/// 单条校验信息：`id` 沿用 epubcheck 系 message ID（如 `OPF-002`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ValidationMessage {
    /// 严重度。
    pub severity: ValidationSeverity,
    /// 标准消息 ID。
    pub id: String,
    /// 人类可读描述（英文，`epubveri` 原文）。
    pub message: String,
    /// 归属文件（包内路径，无位置时为 `None`）。
    pub path: Option<String>,
    /// 行号（1-based，无位置时为 `None`）。
    pub line: Option<u32>,
    /// 列号（1-based，无位置时为 `None`）。
    pub column: Option<u32>,
}

/// 校验结果：`valid` 为假当且仅当存在 `Fatal` / `Error`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ValidationOutcome {
    /// 是否通过（无 `Fatal` / `Error`）。
    pub valid: bool,
    /// 全部信息（文档顺序）。
    pub messages: Vec<ValidationMessage>,
}

/// 打包产物信息：重进页恢复展示的唯一依据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PackageInfo {
    /// 文件名（如 `雾镇.epub`）。
    pub file_name: String,
    /// 产物字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
    /// 打包时间（UTC `RFC3339`）。
    pub updated_at: String,
}

/// 打包结果：产物信息 + 紧随其后的自动校验。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PackageOutcome {
    /// 产物信息。
    pub package: PackageInfo,
    /// 自动校验结果。
    pub validation: ValidationOutcome,
}

/// 书名转文件主干：文件系统非法字符（`/ \ : * ? " < > |` 与控制字符）
///
/// 按 `_` 替换并合并连续，全空或全非法回落 `book`；首尾空格与点一并去除
///（`Windows` 禁止尾点 / 尾空），超长截断。
fn sanitize_stem(title: &str) -> String {
    let mut stem = String::new();
    let mut last_underscore = false;
    for ch in title.chars() {
        let illegal =
            ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|');
        if illegal {
            if !last_underscore && !stem.is_empty() {
                stem.push('_');
            }
            last_underscore = true;
        } else {
            stem.push(ch);
            last_underscore = false;
        }
    }
    let stem = stem.trim().trim_matches('.').trim();
    if stem.is_empty() {
        return "book".to_string();
    }
    stem.chars().take(MAX_STEM_CHARS).collect()
}

/// 构建内相对路径拼接：非空、相对、无跳出成分。
fn resolve_under(base: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    if rel.is_empty() {
        anyhow::bail!("package path must not be empty");
    }
    let mut buf = PathBuf::new();
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(part) => buf.push(part),
            _ => anyhow::bail!("invalid package path: {rel}"),
        }
    }
    if buf.as_os_str().is_empty() {
        anyhow::bail!("invalid package path: {rel}");
    }
    Ok(base.join(buf))
}

/// 相对路径转 POSIX 口径（包内条目统一用 `/`）。
fn to_posix(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 递归收集构建产物文件（POSIX 相对路径，字典序）。
fn collect_files(build_dir: &Path, dir: &Path, out: &mut Vec<String>) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", dir.display()))?;
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        let file_type = std::fs::symlink_metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?
            .file_type();
        if file_type.is_dir() {
            collect_files(build_dir, &path, out)?;
        } else if file_type.is_file() {
            out.push(to_posix(
                path.strip_prefix(build_dir)
                    .context("build path escapes build dir")?,
            ));
        }
    }
    Ok(())
}

/// `dist/` 下最新的 `.epub`：多件并存时保留最新、顺手清理其余。
fn latest_package(project_dir: &Path) -> Option<PathBuf> {
    let dist_dir = project_dir.join(DIST_DIR);
    let entries = std::fs::read_dir(&dist_dir).ok()?;
    let mut packages: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(PACKAGE_EXT))
        })
        .collect();
    if packages.is_empty() {
        return None;
    }
    packages.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    let latest = packages.pop()?;
    for stale in packages {
        let _ = std::fs::remove_file(stale);
    }
    Some(latest)
}

/// 产物 `stat`：大小 + 时间（mtime 不可读时回落当前时间，时间仅作展示）。
fn stat_package(epub_path: &Path) -> anyhow::Result<PackageInfo> {
    let meta = std::fs::metadata(epub_path)
        .with_context(|| format!("failed to stat {}", epub_path.display()))?;
    let updated_at = meta.modified().ok().map_or_else(
        || Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        |time| chrono::DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true),
    );
    let file_name = epub_path
        .file_name()
        .and_then(|name| name.to_str())
        .context("non-utf8 package name")?
        .to_string();
    Ok(PackageInfo {
        file_name,
        size_bytes: u32::try_from(meta.len()).unwrap_or(u32::MAX),
        updated_at,
    })
}

/// `epubveri` 报告转前端契约。
fn from_report(report: &epubveri::report::Report) -> ValidationOutcome {
    use epubveri::report::Severity as External;

    let messages = report
        .messages
        .iter()
        .map(|message| ValidationMessage {
            severity: match message.severity {
                External::Fatal => ValidationSeverity::Fatal,
                External::Error => ValidationSeverity::Error,
                External::Warning => ValidationSeverity::Warning,
                External::Info => ValidationSeverity::Info,
                External::Usage => ValidationSeverity::Usage,
            },
            id: message.id.to_string(),
            message: message.text.clone(),
            path: message.location.clone(),
            line: message.position.map(|position| position.line),
            column: message.position.map(|position| position.column),
        })
        .collect();
    ValidationOutcome {
        valid: report.is_valid(),
        messages,
    }
}

/// 打包全量生成：`build/` → `dist/书名.epub`，随后自动校验一次。
pub fn package_in(project_dir: &Path) -> anyhow::Result<PackageOutcome> {
    use super::project_metadata::load_or_init_metadata_in;

    let build_dir = project_dir.join(BUILD_DIR);
    if !build_dir.is_dir() {
        anyhow::bail!("no build output to package");
    }
    let metadata = load_or_init_metadata_in(project_dir)?;
    let file_name = format!("{}.{PACKAGE_EXT}", sanitize_stem(&metadata.title));

    let dist_dir = project_dir.join(DIST_DIR);
    std::fs::create_dir_all(&dist_dir)
        .with_context(|| format!("failed to create {}", dist_dir.display()))?;
    for entry in std::fs::read_dir(&dist_dir)
        .with_context(|| format!("failed to read {}", dist_dir.display()))?
    {
        let path = entry
            .with_context(|| format!("failed to read {}", dist_dir.display()))?
            .path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case(PACKAGE_EXT))
        {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }

    let mut files = Vec::new();
    collect_files(&build_dir, &build_dir, &mut files)?;
    if !files.iter().any(|path| path == "META-INF/container.xml") {
        anyhow::bail!("build output misses META-INF/container.xml");
    }

    let epub_path = dist_dir.join(&file_name);
    let file = File::create(&epub_path)
        .with_context(|| format!("failed to write {}", epub_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    // `mimetype` 首条明存：无压缩、无 extra field。
    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )
    .context("failed to write mimetype")?;
    zip.write_all(MIMETYPE_BYTES)
        .context("failed to write mimetype")?;
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for rel in &files {
        if rel == "mimetype" {
            continue;
        }
        let bytes = std::fs::read(resolve_under(&build_dir, rel)?)
            .with_context(|| format!("failed to read build file {rel}"))?;
        zip.start_file(rel, deflated)
            .with_context(|| format!("failed to add {rel}"))?;
        zip.write_all(&bytes)
            .with_context(|| format!("failed to add {rel}"))?;
    }
    zip.finish().context("failed to finish epub")?;

    let package = stat_package(&epub_path)?;
    let validation = validate_in(project_dir)?;
    Ok(PackageOutcome {
        package,
        validation,
    })
}

/// 打包状态：无产物回 `None`，前端据此展示待打包空态。
pub fn status_in(project_dir: &Path) -> anyhow::Result<Option<PackageInfo>> {
    match latest_package(project_dir) {
        None => Ok(None),
        Some(path) => Ok(Some(stat_package(&path)?)),
    }
}

/// 校验已有产物：无包直接报错，前端引导先打包。
pub fn validate_in(project_dir: &Path) -> anyhow::Result<ValidationOutcome> {
    let epub_path = latest_package(project_dir).context("no package to validate")?;
    let report = epubveri::validate_path(&epub_path)
        .map_err(|error| anyhow::anyhow!("epub validation failed: {error}"))?;
    Ok(from_report(&report))
}

/// 打包小说（`AppHandle` 入口）：落盘后自动校验一次。
pub fn package_novel(app: &tauri::AppHandle, id: &str) -> anyhow::Result<PackageOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    package_in(&dir)
}

/// 打包状态（`AppHandle` 入口）：无产物回 `None`。
pub fn package_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<Option<PackageInfo>> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 校验产物（`AppHandle` 入口）：无包直接报错。
pub fn validate_package(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ValidationOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    validate_in(&dir)
}

#[cfg(test)]
mod tests {
    use super::super::novel_build;
    use super::super::novel_files;
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;
    use std::io::Read;

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
            "ebook-studio-novel-package-test-{}-{case}",
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

    #[test]
    fn sanitizes_title_to_file_stem() {
        assert_eq!(sanitize_stem("雾镇"), "雾镇");
        assert_eq!(sanitize_stem("上/下：卷"), "上_下：卷");
        assert_eq!(sanitize_stem("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
        assert_eq!(sanitize_stem("  雾镇. "), "雾镇");
        assert_eq!(sanitize_stem(""), "book");
        assert_eq!(sanitize_stem("///"), "book");
        assert_eq!(sanitize_stem("   "), "book");
    }

    #[test]
    fn rejects_packaging_without_build() {
        let (root, dir) = fixture_project("no-build", "第一章 引子\n正文\n");
        assert!(package_in(&dir).is_err());
        assert_eq!(status_in(&dir).expect("status"), None);
        assert!(validate_in(&dir).is_err());
        cleanup(&root);
    }

    #[test]
    fn packages_epub_with_mimetype_first_and_stored() {
        let (root, dir) = fixture_project("full", "第一章 引子\n正文甲\n第二章 发展\n正文乙\n");
        novel_build::build_in(&dir, &template_base()).expect("build");
        let outcome = package_in(&dir).expect("package");
        assert_eq!(outcome.package.file_name, "雾镇.epub");
        assert!(outcome.package.size_bytes > 0);

        let epub_path = dir.join(DIST_DIR).join(&outcome.package.file_name);
        let file = File::open(&epub_path).expect("open epub");
        let mut archive = zip::ZipArchive::new(file).expect("read epub");
        assert!(archive.len() >= 2);
        // 首条为明存的 `mimetype`。
        let mut first = archive.by_index(0).expect("first entry");
        assert_eq!(first.name().expect("entry name").as_ref(), "mimetype");
        assert_eq!(first.compression(), CompressionMethod::Stored);
        let mut content = String::new();
        first.read_to_string(&mut content).expect("read mimetype");
        assert_eq!(content, "application/epub+zip");
        drop(first);
        // 其余条目均为 `Deflate`。
        for index in 1..archive.len() {
            let entry = archive.by_index(index).expect("entry");
            assert_eq!(entry.compression(), CompressionMethod::Deflated);
        }

        // 状态往返：重进页读到同一产物。
        assert_eq!(status_in(&dir).expect("status"), Some(outcome.package));
        cleanup(&root);
    }

    #[test]
    fn packaged_book_passes_epubveri() {
        let (root, dir) = fixture_project("valid", "第一卷 开篇\n第一章 引子\n正文甲\n");
        novel_build::build_in(&dir, &template_base()).expect("build");
        let outcome = package_in(&dir).expect("package");
        assert!(
            outcome.validation.valid,
            "own template output must validate: {:?}",
            outcome.validation.messages
        );
        cleanup(&root);
    }

    #[test]
    fn keeps_single_package_on_repackage() {
        let (root, dir) = fixture_project("repack", "第一章 引子\n正文\n");
        novel_build::build_in(&dir, &template_base()).expect("build");
        let first = package_in(&dir).expect("first package");
        let second = package_in(&dir).expect("second package");
        assert_eq!(first.package.file_name, second.package.file_name);
        let entries: Vec<_> = std::fs::read_dir(dir.join(DIST_DIR))
            .expect("read dist")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(entries.len(), 1);
        cleanup(&root);
    }
}
