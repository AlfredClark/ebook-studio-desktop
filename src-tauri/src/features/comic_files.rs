//! 漫画文件：`<uuid>/sources/images/` 的目录导入、状态查询与顺序解析。
//!
//! 用户选择的漫画目录整体复制为 `sources/images`，解析时按文件名自然排序后落盘
//! `<uuid>/content.json`（`metadata.json` 同级），记录阅读顺序供后续步骤调整。

use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

/// 原文落盘目录：与封面共用 `<uuid>/sources/`。
pub const SOURCES_DIR: &str = "sources";
/// 漫画图片落盘子目录：导入目录整体复制到此处。
pub const IMAGES_DIR: &str = "images";
/// 解析产物文件名：落盘项目根目录，与 `metadata.json` 同级。
pub const CONTENT_NAME: &str = "content.json";
/// 下游构建目录：与 `features::novel_build::BUILD_DIR` 同值，格式转换时连带清空。
pub const BUILD_DIR: &str = "build";
/// 下游打包目录：与 `features::novel_package::DIST_DIR` 同值，格式转换时连带清空。
pub const DIST_DIR: &str = "dist";
/// `content.json` 的结构版本：初始开发阶段固定 `1`，不做跨版本迁移。
const CONTENT_SCHEMA_VERSION: u32 = 1;
/// 混合形态下顶层散图的收纳目录名：固定排首，展示标题为“额外”。
const EXTRA_DIR_NAME: &str = "extra";
/// 收纳话的展示标题：固定为“额外”。
const EXTRA_TITLE: &str = "额外";
/// 导入元信息文件名：隐藏文件，记录复制时跳过的文件数，供解析大纲展示“无效项”。
const IMPORT_META_NAME: &str = ".import-meta.json";
/// 支持的图片扩展名：其余文件导入与解析时一律忽略并计数。
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];
/// 单张图片上限：50MB，超限拒绝导入。
const MAX_IMAGE_BYTES: u64 = 50 * 1024 * 1024;
/// 图片总数上限：超限拒绝导入，避免静默丢页。
const MAX_PAGES: usize = 5000;
/// 分话数上限：含收纳话，超限拒绝导入。
const MAX_VOLUMES: usize = 500;
/// 图片总字节上限：2GB，超限拒绝导入。
const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// 话标题长度上限：字符数，超限拒绝更新。
const MAX_VOLUME_TITLE_CHARS: usize = 50;

/// 漫画原稿状态：左栏文件卡片的只读展示；缺目录时 `exists` 为假。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicImagesStatus {
    /// `sources/images` 是否存在且含有效图片。
    pub exists: bool,
    /// 话数：平铺为 1（含收纳话在内按实际话计数）。
    pub volume_count: u32,
    /// 图片总页数。
    pub page_count: u32,
    /// 图片总字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
    /// 原稿更新时间（UTC `RFC3339`，取目录 mtime）；不存在时为 `None`。
    pub updated_at: Option<String>,
}

/// 漫画组织形态：平铺单话，或按子目录分话。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ComicMode {
    Flat,
    Volumes,
}

/// 单页：解析产物中的一页，相对 `images/` 的 POSIX 路径即阅读顺序。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicPage {
    /// 相对 `images/` 的路径（POSIX 分隔，如 `001.jpg` 或 `第1话/001.jpg`）。
    pub path: String,
}

/// 单话：平铺时标题为 `None`，分话时为目录名，收纳话固定为“额外”。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicVolume {
    /// 话标题；平铺为 `None`，分话为目录名。
    pub title: Option<String>,
    /// 话内页面（已按自然排序）。
    pub pages: Vec<ComicPage>,
}

/// 话输入：编辑页全量提交的单话（标题可改、话可增删排序，页须全量覆盖）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicVolumeInput {
    /// 话标题；`None` 仅允许单话（平铺保持无标题）。
    pub title: Option<String>,
    /// 话内有序路径（相对 `images/` 的 POSIX 路径）。
    pub pages: Vec<String>,
}

/// 解析产物：`content.json` 的落盘格式即前端契约（`snake_case`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicContent {
    /// 结构版本，固定 `1`，由后端断言。
    pub schema_version: u32,
    /// 组织形态。
    pub mode: ComicMode,
    /// 解析时间（UTC `RFC3339`）。
    pub parsed_at: String,
    /// 分话（含平铺的单个无标题话，收纳话排首）。
    pub volumes: Vec<ComicVolume>,
}

/// 单页大纲：右栏识别结果的只读条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicPageOutline {
    /// 相对 `images/` 的路径（POSIX 分隔，前端取图用）。
    pub path: String,
    /// 本页字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
}

/// 话大纲：右栏识别结果的只读条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicVolumeOutline {
    /// 话标题；平铺无标题话为 `None`，前端回落“未分话”。
    pub title: Option<String>,
    /// 话内页面（含路径与字节数）。
    pub pages: Vec<ComicPageOutline>,
}

/// 解析大纲：右栏识别结果的只读展示，图片二进制不出 IPC，只出清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicOutline {
    /// 组织形态。
    pub mode: ComicMode,
    /// 话数。
    pub volume_count: u32,
    /// 图片总页数。
    pub page_count: u32,
    /// 图片总字节数。
    pub total_bytes: u32,
    /// 复制与解析时跳过的无效文件数：导入元信息记账 + 落盘扫描合并，展示用。
    pub ignored_count: u32,
    /// 话页树（已排序）。
    pub volumes: Vec<ComicVolumeOutline>,
}

/// 漫画图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicImageInfo {
    /// 图片绝对路径。
    pub path: String,
    /// 图片字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
}

/// 导入元信息：复制时跳过的文件数（隐藏项按 1 项计，非图片按文件计，
/// 深层嵌套目录按其下文件数计），解析大纲的“无效项”即此数与落盘扫描数之和。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct ImportMeta {
    skipped_count: u32,
}

/// 是否导入元信息文件：隐藏点文件，扫描时显式跳过，不计入忽略。
fn is_import_meta(name: &str) -> bool {
    name == IMPORT_META_NAME
}

/// 递归统计目录下全部文件数（不跟随符号链接，失败项直接跳过）。
fn count_files_recursive(dir: &Path) -> u32 {
    let mut count = 0u32;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(file_type) if file_type.is_file() => count = count.saturating_add(1),
            Ok(file_type) if file_type.is_dir() => {
                count = count.saturating_add(count_files_recursive(&entry.path()));
            }
            _ => {}
        }
    }
    count
}

/// 写导入元信息：复制完成后落盘，供解析时合并跳过数。
fn write_import_meta(images: &Path, skipped_count: u32) -> anyhow::Result<()> {
    let meta = ImportMeta { skipped_count };
    let text = serde_json::to_string_pretty(&meta).context("failed to serialize import meta")?;
    std::fs::write(images.join(IMPORT_META_NAME), text).with_context(|| {
        format!(
            "failed to write {}",
            images.join(IMPORT_META_NAME).display()
        )
    })
}

/// 读导入元信息：缺失或损坏回落 0，不建文件。
fn read_import_skipped(images: &Path) -> u32 {
    let path = images.join(IMPORT_META_NAME);
    if !path.is_file() {
        return 0;
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<ImportMeta>(&text) {
            Ok(meta) => meta.skipped_count,
            Err(err) => {
                log::warn!("跳过损坏的漫画导入元信息 {}: {err:#}", path.display());
                0
            }
        },
        Err(err) => {
            log::warn!("跳过不可读的漫画导入元信息 {}: {err:#}", path.display());
            0
        }
    }
}
/// 是否收纳话路径：`extra/` 前缀即收纳话页，标题改名不影响判定。
fn is_extra_path(rel: &str) -> bool {
    rel.strip_prefix(EXTRA_DIR_NAME)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}
/// 是否隐藏文件：文件名以 `.` 开头；隐藏文件与目录一律忽略。
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// 是否支持的图片：扩展名白名单，大小写不敏感。
fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| IMAGE_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// 宽松转 `u32`：大纲计数上溢时钳制，不报错。
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 自然排序比较：数字块按数值比（去前导零后先比长度再比字典序），其余块按小写比。
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let chunks_a = split_natural_chunks(a);
    let chunks_b = split_natural_chunks(b);
    for (ca, cb) in chunks_a.iter().zip(chunks_b.iter()) {
        let ord = cmp_natural_chunk(ca, cb);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    chunks_a.len().cmp(&chunks_b.len()).then_with(|| a.cmp(b))
}

/// 自然块：数字与非数字交替切分。
fn split_natural_chunks(text: &str) -> Vec<NaturalChunk<'_>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut last_is_digit: Option<bool> = None;
    for (index, ch) in text.char_indices() {
        let is_digit = ch.is_ascii_digit();
        match last_is_digit {
            None => {
                last_is_digit = Some(is_digit);
                start = index;
            }
            Some(prev) if prev == is_digit => {}
            Some(_) => {
                chunks.push(NaturalChunk::new(
                    &text[start..index],
                    last_is_digit.unwrap_or(false),
                ));
                start = index;
                last_is_digit = Some(is_digit);
            }
        }
    }
    if start < text.len() {
        chunks.push(NaturalChunk::new(
            &text[start..],
            last_is_digit.unwrap_or(false),
        ));
    }
    chunks
}

/// 自然块：数字块存去前导零后的有效位，非数字块存原文。
struct NaturalChunk<'a> {
    text: &'a str,
    is_digit: bool,
}

impl<'a> NaturalChunk<'a> {
    const fn new(text: &'a str, is_digit: bool) -> Self {
        Self { text, is_digit }
    }
}

/// 自然块比较：双数字块按数值，双非数字块按小写，数字块恒小于非数字块。
fn cmp_natural_chunk(a: &NaturalChunk<'_>, b: &NaturalChunk<'_>) -> std::cmp::Ordering {
    match (a.is_digit, b.is_digit) {
        (true, true) => {
            let a_trim = a.text.trim_start_matches('0');
            let b_trim = b.text.trim_start_matches('0');
            a_trim
                .len()
                .cmp(&b_trim.len())
                .then_with(|| a_trim.cmp(b_trim))
        }
        (false, false) => a
            .text
            .to_lowercase()
            .cmp(&b.text.to_lowercase())
            .then_with(|| a.text.cmp(b.text)),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
    }
}

/// 目录更新时间：取目录自身的 mtime，失败回落 `None`。
fn dir_updated_at(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(DateTime::<Utc>::from(modified).to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// 收集目录直属图片：仅一层非隐藏文件 + 白名单扩展名，按自然排序返回文件名。
fn direct_images(dir: &Path) -> anyhow::Result<(Vec<String>, u32)> {
    let mut names = Vec::new();
    let mut ignored = 0u32;
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("failed to read dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        let name = entry
            .file_name()
            .to_str()
            .with_context(|| format!("file name is not valid UTF-8: {}", entry.path().display()))?
            .to_string();
        if is_import_meta(&name) {
            // 导入元信息：内部记账文件，不计入忽略。
            continue;
        }
        if is_hidden(&name) {
            if file_type.is_file() || file_type.is_dir() {
                ignored = ignored.saturating_add(1);
            }
            continue;
        }
        if file_type.is_file() {
            if is_supported_image(&entry.path()) {
                names.push(name);
            } else {
                ignored = ignored.saturating_add(1);
            }
        } else if file_type.is_dir() {
            // 深层嵌套不展开：其下文件全部计入跳过。
            ignored = ignored.saturating_add(count_files_recursive(&entry.path()));
        }
    }
    names.sort_by(|a, b| natural_cmp(a, b));
    Ok((names, ignored))
}

/// 查询原稿状态（不建文件）：缺目录回落空态，不报错。
pub fn status_in(project_dir: &Path) -> anyhow::Result<ComicImagesStatus> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    if !images.is_dir() {
        return Ok(ComicImagesStatus {
            exists: false,
            volume_count: 0,
            page_count: 0,
            size_bytes: 0,
            updated_at: None,
        });
    }
    let (volumes, pages, size, _) = scan_images_dir(&images)?;
    if pages == 0 {
        return Ok(ComicImagesStatus {
            exists: false,
            volume_count: 0,
            page_count: 0,
            size_bytes: 0,
            updated_at: dir_updated_at(&images),
        });
    }
    Ok(ComicImagesStatus {
        exists: true,
        volume_count: saturating_u32(volumes.len()),
        page_count: saturating_u32(pages),
        size_bytes: u32::try_from(size).unwrap_or(u32::MAX),
        updated_at: dir_updated_at(&images),
    })
}

/// 扫描已落盘的 `sources/images`：返回（话数，页数，总字节，忽略数）。
/// 话口径与解析一致：无子目录为平铺单话，否则每子目录一话，顶层散图并入收纳话排首。
fn scan_images_dir(images: &Path) -> anyhow::Result<(Vec<String>, usize, u64, u32)> {
    let mut ignored = 0u32;
    let mut subdirs: Vec<String> = Vec::new();
    let mut top_images: Vec<String> = Vec::new();
    let mut total_size = 0u64;
    let mut total_pages = 0usize;
    let entries = std::fs::read_dir(images)
        .with_context(|| format!("failed to read images dir {}", images.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", images.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        let name = entry
            .file_name()
            .to_str()
            .with_context(|| format!("file name is not valid UTF-8: {}", entry.path().display()))?
            .to_string();
        if is_import_meta(&name) {
            // 导入元信息：内部记账文件，不计入忽略。
            continue;
        }
        if is_hidden(&name) {
            ignored = ignored.saturating_add(1);
            continue;
        }
        if file_type.is_dir() {
            // 话内深层嵌套不展开为话：直属图片计入本话，深层文件计入跳过。
            let (names, sub_ignored) = direct_images(&entry.path())?;
            ignored = ignored.saturating_add(sub_ignored);
            for file in &names {
                let meta = std::fs::metadata(entry.path().join(file))
                    .with_context(|| format!("failed to stat {}", entry.path().display()))?;
                total_size = total_size.saturating_add(meta.len());
            }
            // 空话不计入展示，避免空目录撑大话数。
            if !names.is_empty() {
                subdirs.push(name);
            }
            total_pages += names.len();
        } else if file_type.is_file() {
            if is_supported_image(&entry.path()) {
                let meta = std::fs::metadata(entry.path())
                    .with_context(|| format!("failed to stat {}", entry.path().display()))?;
                total_size = total_size.saturating_add(meta.len());
                top_images.push(name);
                total_pages += 1;
            } else {
                ignored = ignored.saturating_add(1);
            }
        }
    }
    // 话清单：收纳话排首，其余按自然排序；顶层散图存在时补收纳话。
    let mut volumes = subdirs;
    volumes.sort_by(|a, b| natural_cmp(a, b));
    if let Some(pos) = volumes.iter().position(|name| name == EXTRA_DIR_NAME) {
        let extra = volumes.remove(pos);
        volumes.insert(0, extra);
    }
    if !top_images.is_empty() && !volumes.contains(&EXTRA_DIR_NAME.to_string()) {
        volumes.insert(0, EXTRA_DIR_NAME.to_string());
    }
    Ok((volumes, total_pages, total_size, ignored))
}

/// 待复制的图片项：源绝对路径 + 目标相对 `images/` 的 POSIX 路径。
struct CopyItem {
    source: PathBuf,
    /// 相对 `images/` 的 POSIX 路径（如 `001.jpg` 或 `第1话/001.jpg`）。
    dest_rel: String,
}

/// 文件名转 `String`：非 UTF-8 大声失败，不做静默替换。
fn file_name_utf8(path: &Path) -> anyhow::Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("file name is not valid UTF-8: {}", path.display()))
        .map(str::to_string)
}

/// 导入漫画目录：整体复制为 `sources/images`，返回最新状态。
///
/// 三形态归一：纯平铺直拷顶层图片；纯分话每子目录一话；混合将顶层散图收进
/// `extra/`（收纳话排首，标题“额外”）。隐藏文件与非图片一律忽略；空源（有效
/// 图片为 0）直接报错；成功后清空下游产物（`content.json`、`build/`、`dist/`）。
#[allow(clippy::too_many_lines)]
pub fn import_dir_in(project_dir: &Path, source: &Path) -> anyhow::Result<ComicImagesStatus> {
    if !source.is_dir() {
        anyhow::bail!("comic source is not a directory: {}", source.display());
    }
    let mut subdirs: Vec<(String, PathBuf)> = Vec::new();
    let mut top_images: Vec<PathBuf> = Vec::new();
    // 复制时跳过的文件数：隐藏项按 1 项计，非图片按文件计，深层嵌套按其下文件数计。
    let mut skipped = 0u32;
    let entries = std::fs::read_dir(source)
        .with_context(|| format!("failed to read source dir {}", source.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", source.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        let name = file_name_utf8(&entry.path())?;
        if is_hidden(&name) {
            skipped = skipped.saturating_add(1);
            continue;
        }
        if file_type.is_dir() {
            // 仅展开一层：子目录内的深层嵌套不跟进，计入跳过。
            subdirs.push((name, entry.path()));
        } else if file_type.is_file() {
            if is_supported_image(&entry.path()) {
                top_images.push(entry.path());
            } else {
                skipped = skipped.saturating_add(1);
            }
        }
    }
    subdirs.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    top_images.sort_by(|a, b| {
        natural_cmp(
            a.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            b.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        )
    });

    let mut items: Vec<CopyItem> = Vec::new();
    if subdirs.is_empty() {
        // 平铺：顶层图片直拷为 images/ 根。
        for path in &top_images {
            items.push(CopyItem {
                source: path.clone(),
                dest_rel: file_name_utf8(path)?,
            });
        }
    } else {
        // 分话：每子目录一话，直属图片按自然排序收纳，深层嵌套计入跳过。
        for (dirname, dirpath) in &subdirs {
            if dirname == EXTRA_DIR_NAME {
                anyhow::bail!("source must not contain an `extra` directory, it is reserved");
            }
            let (names, sub_ignored) = direct_images(dirpath)?;
            skipped = skipped.saturating_add(sub_ignored);
            for name in &names {
                items.push(CopyItem {
                    source: dirpath.join(name),
                    dest_rel: format!("{dirname}/{name}"),
                });
            }
        }
        // 混合：顶层散图收进 extra/，排首由解析侧保证。
        let mut extra_names: Vec<String> = top_images
            .iter()
            .map(|path| file_name_utf8(path))
            .collect::<anyhow::Result<Vec<_>>>()?;
        extra_names.sort_by(|a, b| natural_cmp(a, b));
        for (path, name) in top_images.iter().zip(extra_names.iter()) {
            items.push(CopyItem {
                source: path.clone(),
                dest_rel: format!("{EXTRA_DIR_NAME}/{name}"),
            });
        }
    }
    if items.is_empty() {
        anyhow::bail!(
            "no supported images found (jpg/jpeg/png/webp): {}",
            source.display()
        );
    }
    if items.len() > MAX_PAGES {
        anyhow::bail!("too many images (max {MAX_PAGES})");
    }
    let mut volume_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for item in &items {
        if let Some((volume, _)) = item.dest_rel.split_once('/') {
            volume_names.insert(volume);
        }
    }
    // 平铺无话隔板，不计话上限；分话含收纳话一并约束。
    if !volume_names.is_empty() && volume_names.len() > MAX_VOLUMES {
        anyhow::bail!("too many volumes (max {MAX_VOLUMES})");
    }
    let mut total_bytes = 0u64;
    for item in &items {
        let meta = std::fs::metadata(&item.source)
            .with_context(|| format!("failed to stat {}", item.source.display()))?;
        if !meta.is_file() {
            anyhow::bail!(
                "comic source changed during import: {}",
                item.source.display()
            );
        }
        if meta.len() > MAX_IMAGE_BYTES {
            anyhow::bail!("image is too large (max 50MB): {}", item.source.display());
        }
        total_bytes = total_bytes.saturating_add(meta.len());
    }
    if total_bytes > MAX_TOTAL_BYTES {
        anyhow::bail!("images are too large in total (max 2GB)");
    }

    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    if images.exists() {
        std::fs::remove_dir_all(&images)
            .with_context(|| format!("failed to clear {}", images.display()))?;
    }
    std::fs::create_dir_all(&images)
        .with_context(|| format!("failed to create {}", images.display()))?;
    for item in &items {
        let dest = join_images_path(&images, &item.dest_rel)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::copy(&item.source, &dest).with_context(|| {
            format!(
                "failed to copy {} to {}",
                item.source.display(),
                dest.display()
            )
        })?;
    }
    // 记账复制时跳过的文件数：解析大纲的“无效项”即此数与落盘扫描数之和。
    write_import_meta(&images, skipped)?;
    clear_downstream(project_dir);
    super::projects::touch_updated(project_dir);
    status_in(project_dir)
}

/// 拼接 `images/` 下的相对路径：仅允许 `Normal` 成分，防路径穿越。
pub fn join_images_path(images: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    if rel.is_empty() {
        anyhow::bail!("image path must not be empty");
    }
    let mut buf = PathBuf::new();
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(part) => buf.push(part),
            _ => anyhow::bail!("invalid image path: {rel}"),
        }
    }
    if buf.as_os_str().is_empty() {
        anyhow::bail!("invalid image path: {rel}");
    }
    // 仅允许一层话隔板 + 文件名，拒绝更深嵌套。
    if buf.components().count() > 2 {
        anyhow::bail!("invalid image path: {rel}");
    }
    Ok(images.join(buf))
}

/// 相对路径转 POSIX 口径（产物与前后端契约统一用 `/`）。
fn to_posix(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 清空下游产物：解析、构建与打包结果全部过期，忽略删除失败（缺失即一致）。
/// 换源后图片集合重置，旧压缩备份一并失效（重解析不碰图片集合，备份保留）。
fn clear_downstream(project_dir: &Path) {
    let _ = std::fs::remove_file(project_dir.join(CONTENT_NAME));
    let _ = std::fs::remove_dir_all(project_dir.join(BUILD_DIR));
    let _ = std::fs::remove_dir_all(project_dir.join(DIST_DIR));
    super::comic_images::clear_compress_backup(project_dir);
}

/// 查询原稿状态（`AppHandle` 入口）：项目目录不存在视为目标缺失。
pub fn images_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicImagesStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 导入漫画目录（`AppHandle` 入口）：转换/压缩进行中拒绝，避免与批量替换互斥。
pub fn import_images(
    app: &tauri::AppHandle,
    id: &str,
    source_path: &str,
) -> anyhow::Result<ComicImagesStatus> {
    if super::comic_images::is_busy(id) {
        anyhow::bail!("image job in progress for project {id}, try again later");
    }
    let dir = super::projects::resolve_project_dir(app, id)?;
    import_dir_in(&dir, Path::new(source_path))
}

/// 孤儿图片扫描结果：`content.json` 未引用、但落在 `sources/images/` 下的图片；
/// 封面在 `sources/` 根目录天然排除，隐藏文件与导入元信息不计入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicOrphans {
    /// 孤儿相对路径（POSIX），自然排序。
    pub paths: Vec<String>,
    /// 孤儿总字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub total_bytes: u32,
}

/// 扫描孤儿图片：删页只删顺序记录，源文件永久残留，攒多了从这里一次性清理。
pub fn scan_orphans_in(project_dir: &Path) -> anyhow::Result<ComicOrphans> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    if !images.is_dir() {
        return Ok(ComicOrphans {
            paths: Vec::new(),
            total_bytes: 0,
        });
    }
    let mut referenced = std::collections::HashSet::new();
    // 无解析产物时没有“引用”基线：此时一切落盘图都可能是待解析的，直接回空，
    // 避免刚导入（尚未解析）就提示清理。
    let Ok(content) = read_content(project_dir) else {
        return Ok(ComicOrphans {
            paths: Vec::new(),
            total_bytes: 0,
        });
    };
    for volume in &content.volumes {
        for page in &volume.pages {
            referenced.insert(page.path.clone());
        }
    }
    let mut orphans: Vec<(String, u64)> = Vec::new();
    let mut stack = vec![images.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .with_context(|| format!("failed to read images dir {}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
            let file_type = entry
                .file_type()
                .with_context(|| format!("failed to stat {}", entry.path().display()))?;
            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let name = file_name_utf8(&entry.path())?;
            if is_hidden(&name) || is_import_meta(&name) {
                continue;
            }
            // 只认图片：其它遗留文件不在清理范围内，避免误删用户手放的东西。
            if !is_supported_image(&entry.path()) {
                continue;
            }
            let abs = entry.path();
            let rel = abs
                .strip_prefix(&images)
                .with_context(|| format!("bad image path {}", abs.display()))?;
            let rel = to_posix(rel);
            if referenced.contains(&rel) {
                continue;
            }
            let size = std::fs::metadata(entry.path()).map_or(0, |meta| meta.len());
            orphans.push((rel, size));
        }
    }
    orphans.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    let total_bytes = orphans.iter().map(|(_, size)| *size).sum::<u64>();
    Ok(ComicOrphans {
        paths: orphans.into_iter().map(|(rel, _)| rel).collect(),
        total_bytes: u32::try_from(total_bytes).unwrap_or(u32::MAX),
    })
}

/// 删除孤儿图片：只删扫描命中的无引用图片，`content.json` 不动；
/// 构建产物只含引用页，无需失效；返回刷新后的识别大纲。
pub fn delete_orphans_in(project_dir: &Path) -> anyhow::Result<ComicOutline> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let orphans = scan_orphans_in(project_dir)?;
    for rel in &orphans.paths {
        let abs = join_images_path(&images, rel)?;
        let _ = std::fs::remove_file(&abs);
    }
    super::projects::touch_updated(project_dir);
    outline_in(project_dir)?
        .with_context(|| "content.json missing, parse the images first".to_string())
}

/// 扫描孤儿图片（`AppHandle` 入口）。
pub fn scan_orphans(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicOrphans> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    scan_orphans_in(&dir)
}

/// 删除孤儿图片（`AppHandle` 入口）。
pub fn delete_orphans(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicOutline> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    delete_orphans_in(&dir)
}

/// 解析图片顺序：扫描 `sources/images` 后落盘 `content.json`，返回识别大纲。
///
/// 话口径：无子目录为平铺单话；否则每子目录一话（`extra` 固定排首，标题“额外”，
/// 其余按自然排序），顶层散图（手动放入的）并入收纳话排首。
#[allow(clippy::too_many_lines)]
pub fn parse_images_in(project_dir: &Path) -> anyhow::Result<ComicOutline> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    if !images.is_dir() {
        anyhow::bail!("no images found, import a comic directory first");
    }
    let mut ignored = 0u32;
    let mut subdirs: Vec<String> = Vec::new();
    let mut top_images: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&images)
        .with_context(|| format!("failed to read images dir {}", images.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", images.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        let name = file_name_utf8(&entry.path())?;
        if is_import_meta(&name) {
            // 导入元信息：内部记账文件，不计入忽略。
            continue;
        }
        if is_hidden(&name) {
            ignored = ignored.saturating_add(1);
            continue;
        }
        if file_type.is_dir() {
            subdirs.push(name);
        } else if file_type.is_file() {
            if is_supported_image(&entry.path()) {
                top_images.push(name);
            } else {
                ignored = ignored.saturating_add(1);
            }
        }
    }

    let mut volumes: Vec<ComicVolume> = Vec::new();
    if subdirs.is_empty() && !top_images.is_empty() {
        top_images.sort_by(|a, b| natural_cmp(a, b));
        volumes.push(ComicVolume {
            title: None,
            pages: top_images
                .iter()
                .map(|name| ComicPage { path: name.clone() })
                .collect(),
        });
    } else if !subdirs.is_empty() {
        subdirs.sort_by(|a, b| natural_cmp(a, b));
        // 收纳话固定排首。
        if let Some(pos) = subdirs.iter().position(|name| name == EXTRA_DIR_NAME) {
            let extra = subdirs.remove(pos);
            subdirs.insert(0, extra);
        }
        for dirname in &subdirs {
            let (names, sub_ignored) = direct_images(&images.join(dirname))?;
            ignored = ignored.saturating_add(sub_ignored);
            if names.is_empty() {
                continue;
            }
            let title = if dirname == EXTRA_DIR_NAME {
                Some(EXTRA_TITLE.to_string())
            } else {
                Some(dirname.clone())
            };
            volumes.push(ComicVolume {
                title,
                pages: names
                    .iter()
                    .map(|name| ComicPage {
                        path: format!("{dirname}/{name}"),
                    })
                    .collect(),
            });
        }
        // 顶层散图并入收纳话（排首）：按 `extra/` 路径前缀定位，标题改名不影响合并。
        if !top_images.is_empty() {
            top_images.sort_by(|a, b| natural_cmp(a, b));
            let extra_pages: Vec<ComicPage> = top_images
                .iter()
                .map(|name| ComicPage { path: name.clone() })
                .collect();
            // 收纳话定位：话内含 `extra/` 前缀页即命中，与标题无关。
            let extra_pos = volumes
                .iter()
                .position(|volume| volume.pages.iter().any(|page| is_extra_path(&page.path)));
            if let Some(pos) = extra_pos {
                let merged = std::mem::take(&mut volumes[pos].pages);
                // 已有 extra/ 的页路径带前缀，顶层散图不带前缀：统一自然排序。
                let mut all: Vec<String> = merged
                    .iter()
                    .map(|page| page.path.clone())
                    .chain(extra_pages.iter().map(|page| page.path.clone()))
                    .collect();
                all.sort_by(|a, b| natural_cmp(a, b));
                volumes[pos].pages = all.into_iter().map(|path| ComicPage { path }).collect();
            } else {
                volumes.insert(
                    0,
                    ComicVolume {
                        title: Some(EXTRA_TITLE.to_string()),
                        pages: extra_pages,
                    },
                );
            }
        }
    }
    if volumes.is_empty() {
        anyhow::bail!(
            "no supported images found (jpg/jpeg/png/webp), import a comic directory first"
        );
    }
    let page_total: usize = volumes.iter().map(|volume| volume.pages.len()).sum();
    if page_total > MAX_PAGES {
        anyhow::bail!("too many images (max {MAX_PAGES})");
    }
    let mode = if volumes.len() == 1 && volumes[0].title.is_none() {
        ComicMode::Flat
    } else {
        ComicMode::Volumes
    };
    let content = ComicContent {
        schema_version: CONTENT_SCHEMA_VERSION,
        mode,
        parsed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        volumes,
    };
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    // “无效项”口径：复制时跳过数（导入元信息）+ 落盘扫描时跳过数（手动增删）。
    let skipped = read_import_skipped(&images).saturating_add(ignored);
    super::projects::touch_updated(project_dir);
    outline_of(project_dir, &content, skipped)
}

/// 由完整内容派生识别大纲（落盘与返回共用同一口径，文件大小实时统计）。
fn outline_of(
    project_dir: &Path,
    content: &ComicContent,
    ignored: u32,
) -> anyhow::Result<ComicOutline> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let mut total_bytes = 0u64;
    let mut volumes = Vec::with_capacity(content.volumes.len());
    for volume in &content.volumes {
        let mut pages = Vec::with_capacity(volume.pages.len());
        for page in &volume.pages {
            let abs = join_images_path(&images, &page.path)?;
            let size = std::fs::metadata(&abs).map_or(0, |meta| meta.len());
            total_bytes = total_bytes.saturating_add(size);
            pages.push(ComicPageOutline {
                path: page.path.clone(),
                size_bytes: u32::try_from(size).unwrap_or(u32::MAX),
            });
        }
        volumes.push(ComicVolumeOutline {
            title: volume.title.clone(),
            pages,
        });
    }
    let page_count: usize = volumes.iter().map(|volume| volume.pages.len()).sum();
    Ok(ComicOutline {
        mode: content.mode,
        volume_count: saturating_u32(volumes.len()),
        page_count: saturating_u32(page_count),
        total_bytes: u32::try_from(total_bytes).unwrap_or(u32::MAX),
        ignored_count: ignored,
        volumes,
    })
}

/// 读取已落盘的识别大纲：缺失或损坏回落空，不建文件。
pub fn outline_in(project_dir: &Path) -> anyhow::Result<Option<ComicOutline>> {
    let path = project_dir.join(CONTENT_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            log::warn!("跳过不可读的漫画解析产物 {}: {err:#}", path.display());
            return Ok(None);
        }
    };
    let content: ComicContent = match serde_json::from_str(&text) {
        Ok(content) => content,
        Err(err) => {
            log::warn!("跳过损坏的漫画解析产物 {}: {err:#}", path.display());
            return Ok(None);
        }
    };
    if content.schema_version != CONTENT_SCHEMA_VERSION {
        log::warn!("跳过版本不符的漫画解析产物 {}", path.display());
        return Ok(None);
    }
    // 落盘后用户可能手动增删图片：忽略数无法追溯，展示为 0。
    Ok(Some(outline_of(project_dir, &content, 0)?))
}

/// 解析图片顺序（`AppHandle` 入口）：转换/压缩进行中拒绝，避免与批量替换互斥。
pub fn parse_images(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicOutline> {
    if super::comic_images::is_busy(id) {
        anyhow::bail!("image job in progress for project {id}, try again later");
    }
    let dir = super::projects::resolve_project_dir(app, id)?;
    parse_images_in(&dir)
}

/// 读取已落盘的识别大纲（`AppHandle` 入口）：只读，不建文件。
pub fn content_outline(app: &tauri::AppHandle, id: &str) -> anyhow::Result<Option<ComicOutline>> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    outline_in(&dir)
}

/// 读 `content.json` 全量并断言结构版本：版本不符直接报错，不静默回落。
/// 后续构建 / 编辑步骤共用同一份解析产物。
pub fn read_content(project_dir: &Path) -> anyhow::Result<ComicContent> {
    let path = project_dir.join(CONTENT_NAME);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}, parse the images first", path.display()))?;
    let content: ComicContent = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if content.schema_version != CONTENT_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported content schema version: {}",
            content.schema_version
        );
    }
    Ok(content)
}

/// 查询漫画图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示；非图片或缺失直接报错。
pub fn image_path_in(project_dir: &Path, rel: &str) -> anyhow::Result<ComicImageInfo> {
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let path = join_images_path(&images, rel)?;
    let meta =
        std::fs::metadata(&path).with_context(|| format!("failed to stat {}", path.display()))?;
    if !meta.is_file() {
        anyhow::bail!("comic image not found: {rel}");
    }
    let rel_posix = to_posix(
        path.strip_prefix(&images)
            .context("image path escapes images dir")?,
    );
    if !is_supported_image(Path::new(&rel_posix)) {
        anyhow::bail!("comic file is not an image: {rel}");
    }
    Ok(ComicImageInfo {
        path: path
            .to_str()
            .context("image path is not valid UTF-8")?
            .to_string(),
        size_bytes: u32::try_from(meta.len()).unwrap_or(u32::MAX),
    })
}

/// 查询漫画图片信息（`AppHandle` 入口）。
pub fn image_path(app: &tauri::AppHandle, id: &str, path: &str) -> anyhow::Result<ComicImageInfo> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    image_path_in(&dir, path)
}

/// 全量更新分话结构：标题可改、话可增删排序、可删除页记录，页须全量覆盖后落盘，返回新大纲。
///
/// 话标题修边后判空（显式空串拒绝，`None` 视为无标题话展示回落）、不超长、
/// 话间唯一；传入路径须为现有页面的子集（允许删除记录，被删图片文件保留在
/// `sources/images` 不动），未知 / 重复路径拒绝，逐个防穿越并校验存在。
/// 文件不随话搬家，只改分组归属记录。
#[allow(clippy::too_many_lines)]
pub fn update_volumes_in(
    project_dir: &Path,
    volumes: &[ComicVolumeInput],
) -> anyhow::Result<ComicOutline> {
    if volumes.is_empty() {
        anyhow::bail!("at least one volume is required");
    }
    let mut titles: Vec<&str> = Vec::with_capacity(volumes.len());
    for volume in volumes {
        match volume.title.as_deref().map(str::trim) {
            // 无标题话：平铺遗留，多话时展示回落“未分话”，不强制命名。
            None => {}
            Some("") => anyhow::bail!("volume title must not be empty"),
            Some(text) => {
                if text.chars().count() > MAX_VOLUME_TITLE_CHARS {
                    anyhow::bail!("volume title is too long (max {MAX_VOLUME_TITLE_CHARS} chars)");
                }
                titles.push(text);
            }
        }
    }
    {
        let mut sorted = titles.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != titles.len() {
            anyhow::bail!("volume titles must be unique");
        }
    }
    let mut content = read_content(project_dir)?;
    let existing: std::collections::HashSet<&str> = content
        .volumes
        .iter()
        .flat_map(|volume| volume.pages.iter().map(|page| page.path.as_str()))
        .collect();
    // 子集校验：未知路径与重复路径拒绝，缺页视为删除记录（文件保留不动）。
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for volume in volumes {
        for rel in &volume.pages {
            if !existing.contains(rel.as_str()) {
                anyhow::bail!("unknown page in update: {rel}");
            }
            if !seen.insert(rel.as_str()) {
                anyhow::bail!("duplicate page in update: {rel}");
            }
        }
    }
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    for rel in &seen {
        let path = join_images_path(&images, rel)?;
        if !path.is_file() {
            anyhow::bail!("comic image not found: {rel}");
        }
    }
    content.volumes = volumes
        .iter()
        .map(|volume| ComicVolume {
            title: volume
                .title
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_string),
            pages: volume
                .pages
                .iter()
                .map(|path| ComicPage { path: path.clone() })
                .collect(),
        })
        .collect();
    content.mode = if content.volumes.len() == 1 && content.volumes[0].title.is_none() {
        ComicMode::Flat
    } else {
        ComicMode::Volumes
    };
    content.parsed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    let live_ignored = scan_images_dir(&images).map_or(0, |(_, _, _, ignored)| ignored);
    let skipped = read_import_skipped(&images).saturating_add(live_ignored);
    super::projects::touch_updated(project_dir);
    outline_of(project_dir, &content, skipped)
}

/// 全量更新分话结构（`AppHandle` 入口）：转换/压缩进行中拒绝，避免与批量替换互斥。
#[allow(clippy::needless_pass_by_value)]
pub fn update_volumes(
    app: &tauri::AppHandle,
    id: &str,
    volumes: Vec<ComicVolumeInput>,
) -> anyhow::Result<ComicOutline> {
    if super::comic_images::is_busy(id) {
        anyhow::bail!("image job in progress for project {id}, try again later");
    }
    let dir = super::projects::resolve_project_dir(app, id)?;
    update_volumes_in(&dir, &volumes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

    /// 用例隔离的项目：建真实项目目录（含 `project.json`），跑完清理。
    fn test_project(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-comic-files-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let info =
            projects::create_project_in(&root, "雾镇", ProjectType::Comic).expect("create project");
        let dir = root.join(&info.id);
        (root, dir)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    /// 写一个源图片文件（内容随意，后缀决定类型），返回其路径。
    fn write_source_image(dir: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create source parent");
        }
        std::fs::write(&path, bytes).expect("write source image");
        path
    }

    /// 建一个源漫画目录：`files` 为相对路径清单，自动建父目录。
    fn source_dir(root: &Path, name: &str, files: &[&str]) -> PathBuf {
        let dir = root.join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create source dir");
        for file in files {
            write_source_image(&dir, file, b"fake-image");
        }
        dir
    }

    #[test]
    fn missing_images_reports_empty_status() {
        let (root, dir) = test_project("empty");
        let status = status_in(&dir).expect("status");
        assert!(!status.exists);
        assert_eq!(status.page_count, 0);
        assert!(outline_in(&dir).expect("outline").is_none());
        cleanup(&root);
    }

    #[test]
    fn orphan_scan_and_delete_round_trip() {
        let (root, dir) = test_project("orphans");
        let source = source_dir(&root, "orphan-src", &["第1话/01.jpg", "02.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        // 模拟删页残留 + 手工杂物：孤儿图、非图片、隐藏文件。
        let images = dir.join(SOURCES_DIR).join(IMAGES_DIR);
        write_source_image(&images, "loose.jpg", b"fake-image");
        write_source_image(&images, "note.txt", b"note");
        write_source_image(&images, ".hidden.jpg", b"fake-image");
        let orphans = scan_orphans_in(&dir).expect("scan");
        assert_eq!(orphans.paths, vec!["loose.jpg".to_string()]);
        assert!(orphans.total_bytes > 0);
        let outline = delete_orphans_in(&dir).expect("delete");
        assert_eq!(outline.page_count, 2);
        assert!(!images.join("loose.jpg").exists());
        // 非图片与隐藏文件不在清理范围。
        assert!(images.join("note.txt").is_file());
        assert!(images.join(".hidden.jpg").is_file());
        // 无孤儿时扫描为空，删除仍成功（幂等）。
        let orphans = scan_orphans_in(&dir).expect("rescan");
        assert!(orphans.paths.is_empty(), "unexpected orphans: {orphans:?}");
        delete_orphans_in(&dir).expect("delete empty");
        cleanup(&root);
    }

    #[test]
    fn orphan_scan_empty_without_parse_baseline() {
        let (root, dir) = test_project("orphans-nobaseline");
        let source = source_dir(&root, "orphan-src", &["01.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        // 尚未解析：落盘图都可能是待解析的，不提示清理。
        let orphans = scan_orphans_in(&dir).expect("scan");
        assert!(orphans.paths.is_empty(), "unexpected orphans: {orphans:?}");
        cleanup(&root);
    }

    #[test]
    fn imports_flat_directory_in_natural_order() {
        let (root, dir) = test_project("flat");
        let source = source_dir(
            &root,
            "flat-src",
            &["10.jpg", "2.jpg", "1.JPG", ".hidden.jpg"],
        );
        // 非图片直接忽略，不计入页数。
        write_source_image(&source, "note.txt", b"note");
        let status = import_dir_in(&dir, &source).expect("import");
        assert!(status.exists);
        assert_eq!(status.page_count, 3);
        assert_eq!(status.volume_count, 1);
        let images = dir.join(SOURCES_DIR).join(IMAGES_DIR);
        assert!(images.join("1.JPG").is_file());
        assert!(!images.join(".hidden.jpg").exists());
        assert!(!images.join("note.txt").exists());
        let outline = parse_images_in(&dir).expect("parse");
        assert_eq!(outline.mode, ComicMode::Flat);
        assert_eq!(outline.page_count, 3);
        // 隐藏文件与非图片各 1 个，复制时跳过并记账。
        assert_eq!(outline.ignored_count, 2);
        assert_eq!(outline.volumes.len(), 1);
        assert_eq!(outline.volumes[0].title, None);
        let paths: Vec<&str> = outline.volumes[0]
            .pages
            .iter()
            .map(|page| page.path.as_str())
            .collect();
        assert_eq!(paths, vec!["1.JPG", "2.jpg", "10.jpg"]);
        cleanup(&root);
    }

    #[test]
    fn imports_volumes_and_sorts_extra_first() {
        let (root, dir) = test_project("volumes");
        let source = source_dir(
            &root,
            "vol-src",
            &[
                "第10话/02.png",
                "第10话/01.png",
                "第2话/01.png",
                "第2话/.hidden.png",
                "第2话/sub/01.png",
                "第2话/sub/02.png",
            ],
        );
        let status = import_dir_in(&dir, &source).expect("import");
        assert_eq!(status.volume_count, 2);
        assert_eq!(status.page_count, 3);
        let outline = parse_images_in(&dir).expect("parse");
        assert_eq!(outline.mode, ComicMode::Volumes);
        assert_eq!(outline.volumes.len(), 2);
        assert_eq!(outline.volumes[0].title.as_deref(), Some("第2话"));
        assert_eq!(outline.volumes[1].title.as_deref(), Some("第10话"));
        // 话内隐藏文件 1 个 + 深层嵌套 2 个文件，复制时跳过并记账，不进页数。
        assert_eq!(outline.volumes[0].pages.len(), 1);
        assert_eq!(outline.ignored_count, 3);
        let images = dir.join(SOURCES_DIR).join(IMAGES_DIR);
        assert!(!images.join("第2话").join("sub").exists());
        cleanup(&root);
    }

    #[test]
    fn mixed_source_collects_top_images_into_extra() {
        let (root, dir) = test_project("mixed");
        let source = source_dir(
            &root,
            "mixed-src",
            &["第1话/01.jpg", "cover.jpg", "02.jpg", "note.txt"],
        );
        import_dir_in(&dir, &source).expect("import");
        let images = dir.join(SOURCES_DIR).join(IMAGES_DIR);
        assert!(images.join(EXTRA_DIR_NAME).join("cover.jpg").is_file());
        assert!(images.join(EXTRA_DIR_NAME).join("02.jpg").is_file());
        let outline = parse_images_in(&dir).expect("parse");
        assert_eq!(outline.volumes.len(), 2);
        assert_eq!(outline.volumes[0].title.as_deref(), Some(EXTRA_TITLE));
        assert_eq!(outline.volumes[1].title.as_deref(), Some("第1话"));
        // 顶层非图片 1 个，复制时跳过并记账。
        assert_eq!(outline.ignored_count, 1);
        cleanup(&root);
    }

    #[test]
    fn rejects_empty_and_reserved_extra() {
        let (root, dir) = test_project("bad");
        let empty = source_dir(&root, "empty-src", &["note.txt"]);
        assert!(import_dir_in(&dir, &empty).is_err());
        let reserved = source_dir(&root, "reserved-src", &["extra/01.jpg"]);
        assert!(import_dir_in(&dir, &reserved).is_err());
        cleanup(&root);
    }

    #[test]
    fn reimport_clears_downstream_outputs() {
        let (root, dir) = test_project("clear");
        let first = source_dir(&root, "first-src", &["01.jpg"]);
        import_dir_in(&dir, &first).expect("import");
        parse_images_in(&dir).expect("parse");
        assert!(dir.join(CONTENT_NAME).is_file());
        let build_dummy = dir.join(BUILD_DIR).join("dummy.txt");
        std::fs::create_dir_all(build_dummy.parent().expect("build parent")).expect("create build");
        std::fs::write(&build_dummy, "build").expect("write build dummy");
        let second = source_dir(&root, "second-src", &["第1话/01.jpg"]);
        import_dir_in(&dir, &second).expect("reimport");
        assert!(!dir.join(CONTENT_NAME).exists());
        assert!(!dir.join(BUILD_DIR).exists());
        let outline = parse_images_in(&dir).expect("parse again");
        assert_eq!(outline.mode, ComicMode::Volumes);
        cleanup(&root);
    }

    #[test]
    fn rejects_path_traversal_image_paths() {
        let (root, dir) = test_project("traversal");
        let source = source_dir(&root, "t-src", &["01.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        assert!(image_path_in(&dir, "../content.json").is_err());
        assert!(image_path_in(&dir, "01.jpg").is_ok());
        assert!(image_path_in(&dir, "missing.jpg").is_err());
        cleanup(&root);
    }

    fn input(title: Option<&str>, pages: &[&str]) -> ComicVolumeInput {
        ComicVolumeInput {
            title: title.map(str::to_string),
            pages: pages.iter().map(|page| (*page).to_string()).collect(),
        }
    }

    #[test]
    fn updates_pages_titles_and_order() {
        let (root, dir) = test_project("volumes-update");
        let source = source_dir(
            &root,
            "vu-src",
            &["第1话/01.jpg", "第1话/02.jpg", "第2话/01.jpg"],
        );
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        // 改名 + 话排序 + 话内换位 + 跨话移动：第2话改名排首并收下 2 页。
        let outline = update_volumes_in(
            &dir,
            &[
                input(Some("新话"), &["第2话/01.jpg", "第1话/01.jpg"]),
                input(Some("第1话"), &["第1话/02.jpg"]),
            ],
        )
        .expect("update");
        assert_eq!(outline.page_count, 3);
        assert_eq!(outline.volumes.len(), 2);
        assert_eq!(outline.volumes[0].title.as_deref(), Some("新话"));
        assert_eq!(outline.volumes[1].title.as_deref(), Some("第1话"));
        let first: Vec<&str> = outline.volumes[0]
            .pages
            .iter()
            .map(|page| page.path.as_str())
            .collect();
        assert_eq!(first, vec!["第2话/01.jpg", "第1话/01.jpg"]);
        // 落盘可读，大纲回显一致。
        let reread = outline_in(&dir).expect("reread").expect("outline");
        assert_eq!(reread, outline);
        cleanup(&root);
    }

    #[test]
    fn creates_and_deletes_volumes() {
        let (root, dir) = test_project("volumes-crud");
        let source = source_dir(&root, "vc-src", &["第1话/01.jpg", "第1话/02.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        // 新建空话。
        let outline = update_volumes_in(
            &dir,
            &[
                input(Some("第1话"), &["第1话/01.jpg", "第1话/02.jpg"]),
                input(Some("番外"), &[]),
            ],
        )
        .expect("create");
        assert_eq!(outline.volumes.len(), 2);
        assert_eq!(outline.volumes[1].pages.len(), 0);
        // 页拖入新话后再删空旧话。
        let outline = update_volumes_in(
            &dir,
            &[input(Some("番外"), &["第1话/01.jpg", "第1话/02.jpg"])],
        )
        .expect("move and delete");
        assert_eq!(outline.volumes.len(), 1);
        assert_eq!(outline.volumes[0].title.as_deref(), Some("番外"));
        cleanup(&root);
    }

    #[test]
    fn merges_loose_pages_by_extra_dir_prefix_after_rename() {
        let (root, dir) = test_project("extra-merge");
        let source = source_dir(&root, "em-src", &["第1话/01.jpg", "cover.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        // 收纳话改名后标题不再是“额外”，重解析仍按 extra/ 前缀合并散图。
        update_volumes_in(
            &dir,
            &[
                input(Some("散图"), &["extra/cover.jpg"]),
                input(Some("第1话"), &["第1话/01.jpg"]),
            ],
        )
        .expect("rename extra");
        std::fs::write(
            dir.join(SOURCES_DIR).join(IMAGES_DIR).join("loose.jpg"),
            b"pic",
        )
        .expect("write loose");
        let outline = parse_images_in(&dir).expect("reparse");
        let extra = outline
            .volumes
            .iter()
            .find(|volume| {
                volume
                    .pages
                    .iter()
                    .any(|page| page.path.starts_with("extra/"))
            })
            .expect("extra volume");
        let mut paths: Vec<&str> = extra.pages.iter().map(|page| page.path.as_str()).collect();
        paths.sort_unstable();
        assert_eq!(paths, vec!["extra/cover.jpg", "loose.jpg"]);
        cleanup(&root);
    }

    #[test]
    fn deletes_page_records_but_keeps_files() {
        let (root, dir) = test_project("delete-page");
        let source = source_dir(&root, "dp-src", &["第1话/01.jpg", "第1话/02.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        // 删一条记录：子集接受，大纲只剩 1 页，磁盘文件保留。
        let outline =
            update_volumes_in(&dir, &[input(Some("第1话"), &["第1话/02.jpg"])]).expect("delete");
        assert_eq!(outline.page_count, 1);
        assert!(
            dir.join(SOURCES_DIR)
                .join(IMAGES_DIR)
                .join("第1话/01.jpg")
                .is_file()
        );
        let reread = outline_in(&dir).expect("reread").expect("outline");
        assert_eq!(reread, outline);
        cleanup(&root);
    }

    #[test]
    fn rejects_invalid_volume_updates() {
        let (root, dir) = test_project("volumes-bad");
        let source = source_dir(&root, "vb-src", &["第1话/01.jpg", "第1话/02.jpg"]);
        import_dir_in(&dir, &source).expect("import");
        parse_images_in(&dir).expect("parse");
        let good = || vec![input(Some("第1话"), &["第1话/01.jpg", "第1话/02.jpg"])];
        // 空话清单。
        assert!(update_volumes_in(&dir, &[]).is_err());
        // 缺页即删除记录：子集接受（见 deletes_page_records_but_keeps_files）。
        // 重复。
        assert!(
            update_volumes_in(
                &dir,
                &[input(Some("第1话"), &["第1话/01.jpg", "第1话/01.jpg"])]
            )
            .is_err()
        );
        // 外来路径。
        assert!(
            update_volumes_in(
                &dir,
                &[input(Some("第1话"), &["第1话/01.jpg", "第1话/99.jpg"])]
            )
            .is_err()
        );
        // 路径穿越。
        assert!(
            update_volumes_in(
                &dir,
                &[input(Some("第1话"), &["../content.json", "第1话/01.jpg"])]
            )
            .is_err()
        );
        // 空标题（多话）。
        assert!(
            update_volumes_in(
                &dir,
                &[
                    input(Some("  "), &["第1话/01.jpg"]),
                    input(Some("第2话"), &["第1话/02.jpg"])
                ]
            )
            .is_err()
        );
        // 重复标题。
        assert!(
            update_volumes_in(
                &dir,
                &[
                    input(Some("同名"), &["第1话/01.jpg"]),
                    input(Some("同名"), &["第1话/02.jpg"])
                ]
            )
            .is_err()
        );
        // 超长标题。
        assert!(
            update_volumes_in(
                &dir,
                &[input(
                    Some(&"话".repeat(51)),
                    &["第1话/01.jpg", "第1话/02.jpg"]
                )]
            )
            .is_err()
        );
        // 失败不落盘，原顺序 intact。
        let outline = update_volumes_in(&dir, &good()).expect("good update");
        let paths: Vec<&str> = outline.volumes[0]
            .pages
            .iter()
            .map(|page| page.path.as_str())
            .collect();
        assert_eq!(paths, vec!["第1话/01.jpg", "第1话/02.jpg"]);
        cleanup(&root);
    }
}
