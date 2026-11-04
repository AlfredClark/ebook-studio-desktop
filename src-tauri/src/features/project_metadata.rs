//! 项目元数据：`<uuid>/metadata.json` 的读取（缺失即新建）/ 保存与封面导入。
//!
//! 落盘格式即前端契约（`snake_case`，与 `project.json` 同口径）；`identifier` 复用
//! 项目 id 只读，`title` 默认取项目展示名，`cover` 只经封面导入命令写入——
//! 保存命令忽略传入的 `cover`，防止 JSON 与 `sources/` 下的实际文件不一致。

use std::path::Path;

use anyhow::Context;
use chrono::{NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::projects::{ProjectInfo, ProjectType};

/// `metadata.json` 的结构版本：字段变更时递增并在读取处做迁移（当前 `v2`，简介为数组）。
const SCHEMA_VERSION: u32 = 2;

/// 封面落盘位置：`<uuid>/sources/cover.<ext>`，相对路径存入 `cover` 字段。
const SOURCES_DIR: &str = "sources";
/// 封面文件名主干：导入时清掉旧的 `cover.*` 再写入新的。
const COVER_FILE_STEM: &str = "cover";
/// 封面源文件大小上限：20MB，超限拒绝复制。
const MAX_COVER_BYTES: u64 = 20 * 1024 * 1024;
/// 封面扩展名白名单（小写）：EPUB3 图像核心类型的常用子集。
const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];
/// 小说模板的语言写死 `zh-CN`，保存时强制回填，前端只读展示。
const NOVEL_LANGUAGE: &str = "zh-CN";

/// 项目元数据：都柏林核心 + EPUB3 构建所需的著录信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProjectMetadata {
    /// 结构版本，固定 `2`，由后端断言。
    pub schema_version: u32,
    /// 项目标识符，复用项目 id（`dc:identifier` 取 `urn:uuid:` 前缀），只读。
    pub identifier: String,
    /// 书名（`dc:title`），默认取项目展示名。
    pub title: String,
    /// 副标题（`dc:alternative`）。
    pub alternative: String,
    /// 作者（`dc:creator`）。
    pub creator: String,
    /// 贡献者（`dc:contributor`），单值。
    pub contributor: String,
    /// 语言（`dc:language`，BCP47）；小说固定 `zh-CN`。
    pub language: String,
    /// 出版社（`dc:publisher`）。
    pub publisher: String,
    /// 简介（`dc:description`，兼小说扉页描述块）：每元素一行，已去空去首尾空白。
    pub description: Vec<String>,
    /// 主题词（`dc:subject`，可多值）。
    pub subjects: Vec<String>,
    /// 出版日期（`dc:date`），`YYYY-MM-DD` 或空。
    pub date_published: Option<String>,
    /// 版权（`dc:rights`）。
    pub rights: String,
    /// 系列名（EPUB `belongs-to-collection`，`collection-type` 为 `series`）。
    pub series: String,
    /// 卷次（EPUB `group-position`，如 `上` / `3`）。
    pub volume: String,
    /// 来源（`dc:source`，如原著或改编出处）。
    pub source: String,
    /// 封面相对路径（如 `sources/cover.jpg`），只经封面导入命令写入。
    pub cover: Option<String>,
    /// 更新时间（UTC `RFC3339`），后端每次保存刷新。
    pub updated_at: String,
}

/// 标题长度上限：与项目展示名同口径，副标题同限。
const MAX_TITLE_LEN: usize = 200;
/// 短文本字段（作者 / 贡献者 / 出版社 / 系列）长度上限。
const MAX_SHORT_LEN: usize = 100;
/// 卷次长度上限：自由文本（如 `上` / `3`），短而足。
const MAX_VOLUME_LEN: usize = 50;
/// 主题词数量与单条长度上限。
const MAX_SUBJECTS: usize = 20;
const MAX_SUBJECT_LEN: usize = 50;
/// 简介与版权长度上限。
const MAX_DESCRIPTION_LEN: usize = 5000;
const MAX_RIGHTS_LEN: usize = 500;
/// 来源长度上限：可能含较长的出处链接。
const MAX_SOURCE_LEN: usize = 500;

/// 校验短文本字段：允许空，超长拒绝。
fn check_short_len(value: &str, field: &str) -> anyhow::Result<()> {
    if value.chars().count() > MAX_SHORT_LEN {
        anyhow::bail!("{field} is too long (max {MAX_SHORT_LEN} chars)");
    }
    Ok(())
}

/// 校验 BCP47 语言标签：首段 2-3 字母，后续段 1-8 位字母数字。
fn is_bcp47(value: &str) -> bool {
    let mut parts = value.split('-');
    let first = parts.next().unwrap_or_default();
    if !((2..=3).contains(&first.len()) && first.chars().all(|c| c.is_ascii_alphabetic())) {
        return false;
    }
    parts.all(|part| {
        (1..=8).contains(&part.len()) && part.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// 校验元数据（服务端字段已由调用方回填）：标题非空、语言与日期格式合法、各长度收敛。
fn validate_metadata(meta: &ProjectMetadata, kind: ProjectType) -> anyhow::Result<()> {
    if meta.title.trim().is_empty() {
        anyhow::bail!("title must not be empty");
    }
    if meta.title.chars().count() > MAX_TITLE_LEN {
        anyhow::bail!("title is too long (max {MAX_TITLE_LEN} chars)");
    }
    if meta.alternative.chars().count() > MAX_TITLE_LEN {
        anyhow::bail!("alternative is too long (max {MAX_TITLE_LEN} chars)");
    }
    check_short_len(&meta.creator, "creator")?;
    check_short_len(&meta.contributor, "contributor")?;
    check_short_len(&meta.publisher, "publisher")?;
    check_short_len(&meta.series, "series")?;
    if meta.volume.chars().count() > MAX_VOLUME_LEN {
        anyhow::bail!("volume is too long (max {MAX_VOLUME_LEN} chars)");
    }
    if meta.source.chars().count() > MAX_SOURCE_LEN {
        anyhow::bail!("source is too long (max {MAX_SOURCE_LEN} chars)");
    }
    if kind == ProjectType::Novel && meta.language != NOVEL_LANGUAGE {
        anyhow::bail!("novel language must be {NOVEL_LANGUAGE}");
    }
    if !is_bcp47(&meta.language) {
        anyhow::bail!("invalid language tag: {}", meta.language);
    }
    if description_len(&meta.description) > MAX_DESCRIPTION_LEN {
        anyhow::bail!("description is too long (max {MAX_DESCRIPTION_LEN} chars)");
    }
    if meta.subjects.len() > MAX_SUBJECTS {
        anyhow::bail!("too many subjects (max {MAX_SUBJECTS})");
    }
    for subject in &meta.subjects {
        if subject.chars().count() > MAX_SUBJECT_LEN {
            anyhow::bail!("subject is too long (max {MAX_SUBJECT_LEN} chars): {subject}");
        }
    }
    if meta.rights.chars().count() > MAX_RIGHTS_LEN {
        anyhow::bail!("rights is too long (max {MAX_RIGHTS_LEN} chars)");
    }
    if let Some(date) = &meta.date_published
        && NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
    {
        anyhow::bail!("invalid published date (expected YYYY-MM-DD): {date}");
    }
    if let Some(cover) = &meta.cover
        && !(cover.starts_with("sources/cover.")
            && COVER_EXTENSIONS
                .iter()
                .any(|ext| cover == &format!("sources/cover.{ext}")))
    {
        anyhow::bail!("invalid cover path: {cover}");
    }
    Ok(())
}

/// 初始元数据：标识与标题复用项目信息，语言按类型给默认，其余留空。
fn default_metadata(info: &ProjectInfo) -> ProjectMetadata {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    ProjectMetadata {
        schema_version: SCHEMA_VERSION,
        identifier: info.id.clone(),
        title: info.name.clone(),
        alternative: String::new(),
        creator: String::new(),
        contributor: String::new(),
        language: NOVEL_LANGUAGE.to_string(),
        publisher: String::new(),
        description: Vec::new(),
        subjects: Vec::new(),
        date_published: None,
        rights: String::new(),
        series: String::new(),
        volume: String::new(),
        source: String::new(),
        cover: None,
        updated_at: now,
    }
}

/// 写回 `metadata.json`（紧凑可读的 pretty 格式，与 `project.json` 同风格）。
fn write_metadata(project_dir: &Path, meta: &ProjectMetadata) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(meta).context("failed to serialize metadata")?;
    std::fs::write(project_dir.join("metadata.json"), text)
        .context("failed to write metadata.json")?;
    Ok(())
}

/// 读取项目信息：`project.json` 缺失视为项目损坏，直接报错。
fn read_project_info(project_dir: &Path) -> anyhow::Result<ProjectInfo> {
    super::projects::read_manifest(&project_dir.join("project.json"))
}

/// 读取元数据；文件缺失即按项目信息新建并落盘后返回。
pub fn load_or_init_metadata_in(project_dir: &Path) -> anyhow::Result<ProjectMetadata> {
    let path = project_dir.join("metadata.json");
    if !path.is_file() {
        let info = read_project_info(project_dir)?;
        let meta = default_metadata(&info);
        write_metadata(project_dir, &meta)?;
        return Ok(meta);
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let meta: ProjectMetadata = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if meta.schema_version != SCHEMA_VERSION {
        anyhow::bail!("unsupported metadata version: {}", meta.schema_version);
    }
    Ok(meta)
}

/// 重置元数据：删除 `metadata.json` 后按项目信息重建默认值，返回新建内容。
pub fn reset_metadata_in(project_dir: &Path) -> anyhow::Result<ProjectMetadata> {
    let path = project_dir.join("metadata.json");
    if path.is_file() {
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to delete {}", path.display()))?;
    }
    load_or_init_metadata_in(project_dir)
}

/// 保存元数据：服务端字段（标识 / 版本 / 小说语言 / 存量封面 / 更新时间）一律回填，
/// 简介与主题词做归一化后校验落盘，返回写后的完整内容。
pub fn save_metadata_in(
    project_dir: &Path,
    mut incoming: ProjectMetadata,
) -> anyhow::Result<ProjectMetadata> {
    let info = read_project_info(project_dir)?;
    let stored_cover = load_or_init_metadata_in(project_dir)?.cover;
    incoming.schema_version = SCHEMA_VERSION;
    incoming.identifier.clone_from(&info.id);
    if info.project_type == ProjectType::Novel {
        incoming.language = NOVEL_LANGUAGE.to_string();
    }
    incoming.cover = stored_cover;
    incoming.description = normalize_description(&incoming.description);
    incoming.subjects = dedupe_subjects(&incoming.subjects);
    validate_metadata(&incoming, info.project_type)?;
    incoming.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    write_metadata(project_dir, &incoming)?;
    Ok(incoming)
}

/// 简介总字符数：各行相加（换行符不计），口径与旧字符串版对齐。
fn description_len(description: &[String]) -> usize {
    description.iter().map(|line| line.chars().count()).sum()
}

/// 简介归一化：去每行首尾空白、丢空行，保序不去重。
fn normalize_description(description: &[String]) -> Vec<String> {
    description
        .iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// 主题词归一化：去首尾空白、丢空、保序去重。
fn dedupe_subjects(subjects: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    subjects
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && seen.insert(s.clone()))
        .collect()
}

/// 导入封面：校验源文件后复制为 `sources/cover.<ext>`（清掉旧的 `cover.*`），
/// 回写 `cover` 字段并返回更新后的完整元数据。
pub fn import_cover_in(project_dir: &Path, source: &Path) -> anyhow::Result<ProjectMetadata> {
    if !source.is_file() {
        anyhow::bail!("cover source is not a file: {}", source.display());
    }
    let bytes = source
        .metadata()
        .with_context(|| format!("failed to stat cover source: {}", source.display()))?
        .len();
    if bytes == 0 || bytes > MAX_COVER_BYTES {
        anyhow::bail!("cover size out of range (max 20MB): {}", source.display());
    }
    let ext = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .filter(|ext| COVER_EXTENSIONS.contains(&ext.as_str()))
        .with_context(|| format!("unsupported cover type: {}", source.display()))?;
    let sources = project_dir.join(SOURCES_DIR);
    std::fs::create_dir_all(&sources)
        .with_context(|| format!("failed to create {}", sources.display()))?;
    for entry in std::fs::read_dir(&sources)
        .with_context(|| format!("failed to read {}", sources.display()))?
        .flatten()
    {
        if entry
            .path()
            .file_stem()
            .is_some_and(|stem| stem == COVER_FILE_STEM)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    std::fs::copy(source, sources.join(format!("{COVER_FILE_STEM}.{ext}")))
        .context("failed to copy cover")?;
    let info = read_project_info(project_dir)?;
    let mut meta = load_or_init_metadata_in(project_dir)?;
    meta.cover = Some(format!("{SOURCES_DIR}/{COVER_FILE_STEM}.{ext}"));
    meta.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    validate_metadata(&meta, info.project_type)?;
    write_metadata(project_dir, &meta)?;
    Ok(meta)
}

/// 读取元数据（`AppHandle` 入口）：项目目录不存在视为目标缺失。
pub fn load_or_init_metadata(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ProjectMetadata> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    load_or_init_metadata_in(&dir)
}

/// 保存元数据（`AppHandle` 入口）。
pub fn save_metadata(
    app: &tauri::AppHandle,
    id: &str,
    metadata: ProjectMetadata,
) -> anyhow::Result<ProjectMetadata> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    save_metadata_in(&dir, metadata)
}

/// 重置元数据（`AppHandle` 入口）。
pub fn reset_metadata(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ProjectMetadata> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    reset_metadata_in(&dir)
}

/// 导入封面（`AppHandle` 入口）。
pub fn import_cover(
    app: &tauri::AppHandle,
    id: &str,
    source_path: &str,
) -> anyhow::Result<ProjectMetadata> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    import_cover_in(&dir, Path::new(source_path))
}

/// 项目概览：详情页的只读快照；无元数据文件或解析失败时回落空，不建文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProjectOverview {
    /// 元数据内容；缺失或损坏时为 `None`，前端回落默认展示。
    pub metadata: Option<ProjectMetadata>,
    /// 封面绝对路径（供前端转 `asset:` 预览）；未设置或文件缺失时为 `None`。
    pub cover_path: Option<String>,
}

/// 读取项目概览（不建文件）：损坏的元数据记警告后按缺失处理，不拖死详情页。
pub fn overview_in(project_dir: &Path) -> ProjectOverview {
    let none = || ProjectOverview {
        metadata: None,
        cover_path: None,
    };
    let path = project_dir.join("metadata.json");
    if !path.is_file() {
        return none();
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            log::warn!("跳过不可读的元数据 {}: {err:#}", path.display());
            return none();
        }
    };
    let meta: ProjectMetadata = match serde_json::from_str(&text) {
        Ok(meta) => meta,
        Err(err) => {
            log::warn!("跳过损坏的元数据 {}: {err:#}", path.display());
            return none();
        }
    };
    let cover_path = meta.cover.as_ref().and_then(|cover| {
        let absolute = project_dir.join(cover);
        if !absolute.is_file() {
            return None;
        }
        absolute.to_str().map(str::to_string)
    });
    ProjectOverview {
        metadata: Some(meta),
        cover_path,
    }
}

/// 读取项目概览（`AppHandle` 入口）：只读，不建文件。
pub fn overview(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ProjectOverview> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    Ok(overview_in(&dir))
}

/// 封面绝对路径：供前端转 `asset:` 预览 URL；未设置或文件缺失视为无封面。
pub fn cover_absolute_path_in(project_dir: &Path) -> anyhow::Result<Option<String>> {
    let meta = load_or_init_metadata_in(project_dir)?;
    let Some(cover) = meta.cover else {
        return Ok(None);
    };
    let absolute = project_dir.join(&cover);
    if !absolute.is_file() {
        return Ok(None);
    }
    let text = absolute
        .to_str()
        .with_context(|| format!("cover path is not valid UTF-8: {}", absolute.display()))?;
    Ok(Some(text.to_string()))
}

/// 封面绝对路径（`AppHandle` 入口）。
pub fn cover_absolute_path(app: &tauri::AppHandle, id: &str) -> anyhow::Result<Option<String>> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    cover_absolute_path_in(&dir)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::projects;
    use super::*;

    /// 用例隔离的项目：建真实项目目录（含 `project.json`），跑完清理。
    fn test_project(case: &str, kind: ProjectType) -> (PathBuf, PathBuf, ProjectInfo) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-metadata-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let info = projects::create_project_in(&root, "雾镇", kind).expect("create project");
        let dir = root.join(&info.id);
        (root, dir, info)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn inits_metadata_with_project_defaults() {
        let (root, dir, info) = test_project("init", ProjectType::Novel);
        let meta = load_or_init_metadata_in(&dir).expect("init");
        assert_eq!(meta.identifier, info.id);
        assert_eq!(meta.title, "雾镇");
        assert_eq!(meta.language, "zh-CN");
        assert_eq!(meta.cover, None);
        assert!(dir.join("metadata.json").is_file());
        cleanup(&root);
    }

    #[test]
    fn rejects_bad_metadata_on_save() {
        let (root, dir, _) = test_project("save-bad", ProjectType::Comic);
        load_or_init_metadata_in(&dir).expect("init");
        let mut meta = load_or_init_metadata_in(&dir).expect("load");
        meta.title = "   ".to_string();
        assert!(save_metadata_in(&dir, meta.clone()).is_err());
        meta.title = "雾镇".to_string();
        meta.language = "not a tag!".to_string();
        assert!(save_metadata_in(&dir, meta.clone()).is_err());
        meta.language = "en".to_string();
        meta.date_published = Some("2026-13-01".to_string());
        assert!(save_metadata_in(&dir, meta.clone()).is_err());
        meta.date_published = Some("2026-02-01".to_string());
        meta.subjects = (0..21).map(|i| format!("subject-{i}")).collect();
        assert!(save_metadata_in(&dir, meta).is_err());
        cleanup(&root);
    }

    #[test]
    fn forces_novel_language_and_server_fields() {
        let (root, dir, _) = test_project("novel-force", ProjectType::Novel);
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        meta.language = "en".to_string();
        meta.identifier = "tampered".to_string();
        meta.cover = Some("sources/cover.jpg".to_string());
        let saved = save_metadata_in(&dir, meta).expect("save");
        assert_eq!(saved.language, "zh-CN");
        assert_ne!(saved.identifier, "tampered");
        assert_eq!(saved.cover, None);
        cleanup(&root);
    }

    #[test]
    fn cover_absolute_path_tracks_imported_file() {
        let (root, dir, _) = test_project("cover-path", ProjectType::Novel);
        assert_eq!(cover_absolute_path_in(&dir).expect("no cover yet"), None);
        let source = root.join("picked.png");
        std::fs::write(&source, vec![1u8; 16]).expect("write fake png");
        import_cover_in(&dir, &source).expect("import");
        let found = cover_absolute_path_in(&dir).expect("cover path");
        assert!(found.is_some_and(|p| p.ends_with("sources/cover.png")));
        let _ = std::fs::remove_file(dir.join("sources/cover.png"));
        assert_eq!(cover_absolute_path_in(&dir).expect("missing file"), None);
        cleanup(&root);
    }

    #[test]
    fn validates_new_dublin_core_fields() {
        let (root, dir, _) = test_project("dc-extra", ProjectType::Comic);
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        assert_eq!(meta.alternative, "");
        assert_eq!(meta.series, "");
        meta.alternative = "副标题".to_string();
        meta.series = "长夜系列".to_string();
        meta.volume = "上".to_string();
        meta.source = "原著《雾镇》".to_string();
        let saved = save_metadata_in(&dir, meta).expect("save dc extras");
        assert_eq!(saved.alternative, "副标题");
        assert_eq!(saved.series, "长夜系列");
        assert_eq!(saved.volume, "上");
        assert_eq!(saved.source, "原著《雾镇》");
        let mut bad = load_or_init_metadata_in(&dir).expect("reload");
        bad.volume = "x".repeat(51);
        assert!(save_metadata_in(&dir, bad.clone()).is_err());
        bad.volume = String::new();
        bad.source = "x".repeat(501);
        assert!(save_metadata_in(&dir, bad).is_err());
        cleanup(&root);
    }

    #[test]
    fn normalizes_description_lines_on_save() {
        let (root, dir, _) = test_project("desc-lines", ProjectType::Comic);
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        assert_eq!(meta.description, Vec::<String>::new());
        meta.description = vec![
            "  第一行  ".to_string(),
            String::new(),
            "   ".to_string(),
            "第二行".to_string(),
            "第二行".to_string(),
        ];
        let saved = save_metadata_in(&dir, meta).expect("save description");
        assert_eq!(saved.description, vec!["第一行", "第二行", "第二行"]);
        let reloaded = load_or_init_metadata_in(&dir).expect("reload");
        assert_eq!(reloaded.description, vec!["第一行", "第二行", "第二行"]);
        cleanup(&root);
    }

    #[test]
    fn rejects_overlong_description_total_and_legacy_string() {
        let (root, dir, _) = test_project("desc-bad", ProjectType::Comic);
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        meta.description = vec!["x".repeat(5001)];
        assert!(save_metadata_in(&dir, meta.clone()).is_err());
        meta.description = vec!["x".repeat(2500), "y".repeat(2501)];
        assert!(save_metadata_in(&dir, meta).is_err());
        let text = std::fs::read_to_string(dir.join("metadata.json")).expect("read metadata");
        let mut value: serde_json::Value = serde_json::from_str(&text).expect("parse metadata");
        value["description"] = serde_json::Value::String("旧简介第一行\n旧简介第二行".to_string());
        std::fs::write(
            dir.join("metadata.json"),
            serde_json::to_string_pretty(&value).expect("serialize legacy"),
        )
        .expect("write legacy metadata");
        assert!(load_or_init_metadata_in(&dir).is_err());
        cleanup(&root);
    }

    #[test]
    fn resets_metadata_to_defaults() {
        let (root, dir, info) = test_project("reset", ProjectType::Novel);
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        meta.title = "改过的书名".to_string();
        meta.publisher = "某社".to_string();
        save_metadata_in(&dir, meta).expect("save");
        let reset = reset_metadata_in(&dir).expect("reset");
        assert_eq!(reset.title, info.name);
        assert_eq!(reset.publisher, "");
        assert_eq!(reset.identifier, info.id);
        cleanup(&root);
    }

    #[test]
    fn overview_is_read_only_and_tolerates_damage() {
        let (root, dir, info) = test_project("overview", ProjectType::Comic);
        let empty = overview_in(&dir);
        assert_eq!(empty.metadata, None);
        assert_eq!(empty.cover_path, None);
        assert!(
            !dir.join("metadata.json").exists(),
            "overview must not create files"
        );
        let mut meta = load_or_init_metadata_in(&dir).expect("init");
        meta.title = "雾镇".to_string();
        save_metadata_in(&dir, meta).expect("save");
        let source = root.join("picked.png");
        std::fs::write(&source, vec![1u8; 16]).expect("write fake png");
        import_cover_in(&dir, &source).expect("import");
        let full = overview_in(&dir);
        let meta = full.metadata.expect("metadata present");
        assert_eq!(meta.identifier, info.id);
        assert!(
            full.cover_path
                .is_some_and(|p| p.ends_with("sources/cover.png"))
        );
        std::fs::write(dir.join("metadata.json"), "{ broken").expect("corrupt");
        let damaged = overview_in(&dir);
        assert_eq!(damaged.metadata, None);
        assert_eq!(damaged.cover_path, None);
        cleanup(&root);
    }

    #[test]
    fn imports_cover_and_replaces_old() {
        let (root, dir, _) = test_project("cover", ProjectType::Comic);
        let source = root.join("picked.png");
        std::fs::write(&source, vec![1u8; 16]).expect("write fake png");
        let meta = import_cover_in(&dir, &source).expect("import");
        assert_eq!(meta.cover.as_deref(), Some("sources/cover.png"));
        assert!(dir.join("sources/cover.png").is_file());
        let second = root.join("picked.JPG");
        std::fs::write(&second, vec![2u8; 16]).expect("write fake jpg");
        let meta = import_cover_in(&dir, &second).expect("reimport");
        assert_eq!(meta.cover.as_deref(), Some("sources/cover.jpg"));
        assert!(!dir.join("sources/cover.png").exists());
        assert!(import_cover_in(&dir, &root.join("missing.png")).is_err());
        let bad = root.join("note.txt");
        std::fs::write(&bad, "text").expect("write txt");
        assert!(import_cover_in(&dir, &bad).is_err());
        cleanup(&root);
    }
}
