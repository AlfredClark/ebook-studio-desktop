//! 漫画图片格式转换：将 `content.json` 全量页统一转码为指定格式。
//!
//! 预览（按源格式分组计数）与执行（事务性两阶段）共用同一份扫描口径；
//! 执行成功后更新 `content.json` 的 path 并清空下游 `build/` 与 `dist/`。
//! 解码与 JPEG/PNG 编码走纯 Rust 的 `image`，WebP 有损走 libwebp 官方绑定。

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::comic_files::{
    self, BUILD_DIR, CONTENT_NAME, ComicOutline, DIST_DIR, IMAGES_DIR, SOURCES_DIR,
};

/// 目标格式：与源格式集合（jpg/jpeg/png/webp）对齐，前端下拉直接用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ImageTargetFormat {
    Jpeg,
    Png,
    Webp,
}

impl ImageTargetFormat {
    /// 落盘扩展名（小写）。
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    /// 同组判定：大小写不敏感，`jpeg` 归一为 `jpg`。
    pub fn matches_source(self, ext: &str) -> bool {
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => self == Self::Jpeg,
            "png" => self == Self::Png,
            "webp" => self == Self::Webp,
            _ => false,
        }
    }
}

/// 转换参数：随目标格式生效其余忽略（前端按格式切换控件）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertOptions {
    /// JPEG/WebP 有损质量，1-100。
    pub quality: u8,
    /// WebP 无损开关（开则忽略质量）。
    pub webp_lossless: bool,
    /// PNG 压缩级别，0-9。
    pub png_level: u8,
    /// 同格式也重编码（默认关 = 跳过，幂等）。
    pub force: bool,
}

impl ConvertOptions {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=100).contains(&self.quality) {
            anyhow::bail!("quality must be within 1..=100, got {}", self.quality);
        }
        if self.png_level > 9 {
            anyhow::bail!("png_level must be within 0..=9, got {}", self.png_level);
        }
        Ok(())
    }
}

/// 预览行：某源格式的待转张数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertPreviewItem {
    /// 小写源扩展名（`jpeg` 归一为 `jpg`）。
    pub from: String,
    pub count: u32,
}

/// 转换预览：落点确认弹窗按行展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ConvertPreview {
    pub items: Vec<ConvertPreviewItem>,
    pub total: u32,
}

/// 转换进度事件名：前端 hook 与确认弹窗进度条会合，改名需两端同步。
pub const CONVERT_PROGRESS_EVENT: &str = "comic-images-convert-progress";

/// 转换进度载荷。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ComicConvertProgress {
    pub id: String,
    pub done_files: u32,
    pub total_files: u32,
}

/// 转换状态探针：供切回页面时回填进度；从未开始返回空闲零值。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ComicConvertStatus {
    pub running: bool,
    pub done_files: u32,
    pub total_files: u32,
}

/// 进行中的转换作业：uuid → (done, total)。单进程内互斥与状态查询共用；
/// 崩溃重启即清空，暂存区残留由下次转换开头清理。
static CONVERTING: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (u32, u32)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 是否有转换在跑（解析 / 导入互斥用）。
pub fn is_converting(id: &str) -> bool {
    CONVERTING.lock().is_ok_and(|jobs| jobs.contains_key(id))
}

/// 查询作业状态：未在跑回空闲零值。
pub fn convert_status(id: &str) -> ComicConvertStatus {
    CONVERTING
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(id).copied())
        .map_or_else(
            || ComicConvertStatus {
                running: false,
                done_files: 0,
                total_files: 0,
            },
            |(done_files, total_files)| ComicConvertStatus {
                running: true,
                done_files,
                total_files,
            },
        )
}

/// 登记作业：同项目已有在跑任务时整单拒绝（暂存区按项目独占，不可并发）。
fn register_convert(id: &str) -> anyhow::Result<()> {
    let mut jobs = CONVERTING
        .lock()
        .map_err(|_| anyhow::anyhow!("convert registry poisoned"))?;
    if jobs.contains_key(id) {
        anyhow::bail!("conversion already in progress for project {id}");
    }
    jobs.insert(id.to_string(), (0, 0));
    drop(jobs);
    Ok(())
}

/// 更新作业进度：注册表缺失时静默跳过，只写不读，不影响主流程。
fn update_convert_progress(id: &str, done: u32, total: u32) {
    if let Ok(mut jobs) = CONVERTING.lock()
        && let Some(entry) = jobs.get_mut(id)
    {
        *entry = (done, total);
    }
}

/// 注销作业：成功失败都走，锁中毒也尽力清理。
fn finish_convert(id: &str) {
    if let Ok(mut jobs) = CONVERTING.lock() {
        jobs.remove(id);
    }
}

/// 暂存目录：项目根下的隐藏目录，编码期中转，成功 / 失败后清理。
const STAGING_DIR: &str = ".tmp-convert";

/// 单个转换任务：源相对路径 + 目标相对路径（POSIX）。
struct ConvertJob {
    src_rel: String,
    dest_rel: String,
}

/// 取小写扩展名并将 `jpeg` 归一为 `jpg`；无扩展名直接报错。
fn normalized_ext(rel: &str) -> anyhow::Result<String> {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let ext = file
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| !ext.is_empty())
        .with_context(|| format!("image has no extension: {rel}"))?;
    let lower = ext.to_ascii_lowercase();
    Ok(if lower == "jpeg" {
        "jpg".to_string()
    } else {
        lower
    })
}

/// 目标相对路径：同目录换扩展名。
fn dest_rel(src_rel: &str, target: ImageTargetFormat) -> String {
    let (dir, file) = src_rel.rsplit_once('/').unwrap_or(("", src_rel));
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let name = format!("{stem}.{}", target.extension());
    if dir.is_empty() {
        name
    } else {
        format!("{dir}/{name}")
    }
}

/// 扫描全量页组装任务：未知格式直接报错；同格式按 force 决定跳过与否。
/// 返回任务与按源格式分组的计数（BTreeMap 保证行序稳定）。
fn collect_jobs(
    project_dir: &Path,
    target: ImageTargetFormat,
    force: bool,
) -> anyhow::Result<(Vec<ConvertJob>, BTreeMap<String, u32>)> {
    let content = comic_files::read_content(project_dir)?;
    let mut jobs = Vec::new();
    let mut groups: BTreeMap<String, u32> = BTreeMap::new();
    for volume in &content.volumes {
        for page in &volume.pages {
            let ext = normalized_ext(&page.path)?;
            if !target.matches_source(&ext) && !matches!(ext.as_str(), "jpg" | "png" | "webp") {
                anyhow::bail!("unsupported image format: {}", page.path);
            }
            if target.matches_source(&ext) && !force {
                continue;
            }
            *groups.entry(ext).or_insert(0) += 1;
            jobs.push(ConvertJob {
                src_rel: page.path.clone(),
                dest_rel: dest_rel(&page.path, target),
            });
        }
    }
    Ok((jobs, groups))
}

/// 冲突预检：同批多源撞同一目标，或目标撞到不动的文件，一律整单报错。
fn check_conflicts(images_dir: &Path, jobs: &[ConvertJob]) -> anyhow::Result<()> {
    let mut seen = HashSet::new();
    let mut conflicts = Vec::new();
    for job in jobs {
        if !seen.insert(job.dest_rel.clone()) {
            conflicts.push(job.dest_rel.clone());
            continue;
        }
        if job.dest_rel == job.src_rel {
            continue;
        }
        let dest = comic_files::join_images_path(images_dir, &job.dest_rel)?;
        if dest.exists() {
            conflicts.push(job.dest_rel.clone());
        }
    }
    if !conflicts.is_empty() {
        conflicts.sort();
        conflicts.dedup();
        anyhow::bail!(
            "target files already exist, convert aborted: {}",
            conflicts.join(", ")
        );
    }
    Ok(())
}

/// 单张编码：JPEG 去 alpha；PNG 保留原色彩类型；WebP 按开关选有损 / 无损。
fn encode_image(
    img: &image::DynamicImage,
    target: ImageTargetFormat,
    options: &ConvertOptions,
) -> anyhow::Result<Vec<u8>> {
    match target {
        ImageTargetFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut buf = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, options.quality)
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .context("failed to encode JPEG")?;
            Ok(buf)
        }
        ImageTargetFormat::Png => {
            let compression = match options.png_level {
                0 => image::codecs::png::CompressionType::Uncompressed,
                level @ 1..=9 => image::codecs::png::CompressionType::Level(level),
                _ => anyhow::bail!("png_level out of range"),
            };
            let mut buf = Vec::new();
            image::codecs::png::PngEncoder::new_with_quality(
                &mut buf,
                compression,
                image::codecs::png::FilterType::default(),
            )
            .write_image(
                img.as_bytes(),
                img.width(),
                img.height(),
                img.color().into(),
            )
            .context("failed to encode PNG")?;
            Ok(buf)
        }
        ImageTargetFormat::Webp => {
            let rgba = img.to_rgba8();
            let encoded = if options.webp_lossless {
                webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
                    .encode_lossless()
            } else {
                webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
                    .encode(f32::from(options.quality))
            };
            Ok(encoded.to_vec())
        }
    }
}

/// 预览转换分组：只读 `content.json`，不碰图片文件。
pub fn preview_conversion_in(
    project_dir: &Path,
    target: ImageTargetFormat,
    force: bool,
) -> anyhow::Result<ConvertPreview> {
    let (_jobs, groups) = collect_jobs(project_dir, target, force)?;
    let items = groups
        .into_iter()
        .map(|(from, count)| ConvertPreviewItem { from, count })
        .collect::<Vec<_>>();
    let total = items.iter().map(|item| item.count).sum();
    Ok(ConvertPreview { items, total })
}

/// 预览转换分组（`AppHandle` 入口）。
pub fn preview_conversion(
    app: &tauri::AppHandle,
    id: &str,
    target: ImageTargetFormat,
    force: bool,
) -> anyhow::Result<ConvertPreview> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    preview_conversion_in(&dir, target, force)
}

/// 查询转换状态（`AppHandle` 入口）：供切回页面时回填进度。
pub fn convert_status_of(id: &str) -> ComicConvertStatus {
    convert_status(id)
}

/// 格式转换（`AppHandle` 入口）：阻塞池执行并推送逐文件进度，前端据此渲染进度。
pub async fn convert_images(
    app: &tauri::AppHandle,
    id: &str,
    target: ImageTargetFormat,
    options: &ConvertOptions,
) -> anyhow::Result<ComicOutline> {
    use tauri::Emitter;

    let dir = super::projects::resolve_project_dir(app, id)?;
    register_convert(id)?;
    let options = options.clone();
    let handle = app.clone();
    let report_id = id.to_string();
    let progress_id = id.to_string();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let on_progress = move |done: u32, total: u32| {
            update_convert_progress(&progress_id, done, total);
            let _ = handle.emit(
                CONVERT_PROGRESS_EVENT,
                ComicConvertProgress {
                    id: report_id.clone(),
                    done_files: done,
                    total_files: total,
                },
            );
        };
        convert_images_in(&dir, target, &options, Some(&on_progress))
    })
    .await;
    finish_convert(id);
    outcome.context("convert task panicked")?
}

/// 格式转换内核：事务性两阶段（暂存区编码→批量替换），成功后更新
/// `content.json` 的 path 并清空下游 `build/` 与 `dist/`（`content.json` 保留）。
fn convert_images_in(
    project_dir: &Path,
    target: ImageTargetFormat,
    options: &ConvertOptions,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<ComicOutline> {
    options.validate()?;
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let (jobs, _groups) = collect_jobs(project_dir, target, options.force)?;
    if jobs.is_empty() {
        // 幂等空转：直接回当前大纲，不碰文件与下游。
        return comic_files::outline_in(project_dir)?
            .with_context(|| "content.json missing, parse the images first".to_string());
    }
    check_conflicts(&images, &jobs)?;
    // 阶段一：全部解码编码到暂存区；任一步失败清理暂存区、原状不动。
    let staging = project_dir.join(STAGING_DIR);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).context("failed to create convert staging dir")?;
    let staged =
        stage_all(&images, &staging, &jobs, target, options, progress).inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&staging);
        })?;
    // 阶段二：批量替换（rename 覆盖 + 删原文件），随后落盘 content.json。
    for (index, job) in jobs.iter().enumerate() {
        let dest = comic_files::join_images_path(&images, &job.dest_rel)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::rename(&staged[index], &dest).with_context(|| {
            format!(
                "failed to move {} to {}",
                staged[index].display(),
                dest.display()
            )
        })?;
        let src = comic_files::join_images_path(&images, &job.src_rel)?;
        if dest != src {
            std::fs::remove_file(&src)
                .with_context(|| format!("failed to remove {}", src.display()))?;
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    let mut content = comic_files::read_content(project_dir)?;
    let renamed: std::collections::HashMap<&str, &str> = jobs
        .iter()
        .map(|job| (job.src_rel.as_str(), job.dest_rel.as_str()))
        .collect();
    for volume in &mut content.volumes {
        for page in &mut volume.pages {
            if let Some(dest) = renamed.get(page.path.as_str()) {
                page.path = (*dest).to_string();
            }
        }
    }
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    // 下游构建与打包产物含旧格式图片，一并失效（`content.json` 保留）。
    let _ = std::fs::remove_dir_all(project_dir.join(BUILD_DIR));
    let _ = std::fs::remove_dir_all(project_dir.join(DIST_DIR));
    comic_files::outline_in(project_dir)?
        .with_context(|| "content.json missing after convert".to_string())
}

/// 阶段一：逐张解码编码进暂存区并上报进度。
fn stage_all(
    images: &Path,
    staging: &Path,
    jobs: &[ConvertJob],
    target: ImageTargetFormat,
    options: &ConvertOptions,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<Vec<PathBuf>> {
    let total = u32::try_from(jobs.len()).unwrap_or(u32::MAX);
    let mut staged = Vec::with_capacity(jobs.len());
    for (index, job) in jobs.iter().enumerate() {
        let src = comic_files::join_images_path(images, &job.src_rel)?;
        let bytes =
            std::fs::read(&src).with_context(|| format!("failed to read {}", src.display()))?;
        let img = image::load_from_memory(&bytes)
            .with_context(|| format!("failed to decode {}", job.src_rel))?;
        let encoded = encode_image(&img, target, options)?;
        let staged_path = staging.join(format!("{index:06}"));
        std::fs::write(&staged_path, &encoded)
            .with_context(|| format!("failed to stage {}", staged_path.display()))?;
        staged.push(staged_path);
        if let Some(report) = progress {
            let done = u32::try_from(index + 1).unwrap_or(u32::MAX);
            report(done, total);
        }
    }
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::comic_files::{ComicContent, ComicMode, ComicPage, ComicVolume};
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

    /// 用例隔离的项目：建真实项目目录（含 `project.json`），跑完清理。
    fn test_project(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-comic-images-test-{}-{case}",
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

    /// 写一张 4x4 纯色真图片（格式由扩展名决定）。
    fn write_test_image(dir: &Path, rel: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create image parent");
        }
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([200, 30, 30]));
        image::DynamicImage::ImageRgb8(img)
            .save(&path)
            .expect("save test image");
        path
    }

    /// 按给定相对路径清单落 sources/images，并写最小 content.json。
    fn seed_images(dir: &Path, rels: &[&str]) {
        let images = dir
            .join(super::super::comic_files::SOURCES_DIR)
            .join(super::super::comic_files::IMAGES_DIR);
        for rel in rels {
            if Path::new(rel)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"))
            {
                let path = images.join(rel);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).expect("create image parent");
                }
                std::fs::write(&path, b"not-an-image").expect("write corrupt image");
            } else {
                write_test_image(&images, rel);
            }
        }
        let content = ComicContent {
            schema_version: 1,
            mode: ComicMode::Flat,
            parsed_at: "2026-01-01T00:00:00Z".to_string(),
            volumes: vec![ComicVolume {
                title: None,
                pages: rels
                    .iter()
                    .map(|rel| ComicPage {
                        path: (*rel).to_string(),
                    })
                    .collect(),
            }],
        };
        let text = serde_json::to_string_pretty(&content).expect("serialize content");
        std::fs::write(dir.join("content.json"), text).expect("write content");
    }

    fn read_paths(dir: &Path) -> Vec<String> {
        let content = comic_files::read_content(dir).expect("read content");
        content.volumes[0]
            .pages
            .iter()
            .map(|page| page.path.clone())
            .collect()
    }

    fn valid_options() -> ConvertOptions {
        ConvertOptions {
            quality: 85,
            webp_lossless: false,
            png_level: 6,
            force: false,
        }
    }

    #[test]
    fn convert_registry_lifecycle() {
        let id = "registry-lifecycle";
        assert!(!is_converting(id));
        let idle = convert_status(id);
        assert!(!idle.running);
        register_convert(id).expect("register");
        assert!(is_converting(id));
        let status = convert_status(id);
        assert!(status.running);
        assert_eq!((status.done_files, status.total_files), (0, 0));
        update_convert_progress(id, 3, 10);
        let status = convert_status(id);
        assert_eq!((status.done_files, status.total_files), (3, 10));
        finish_convert(id);
        assert!(!is_converting(id));
    }

    #[test]
    fn convert_registry_rejects_reentrant_run() {
        let id = "registry-reentrant";
        register_convert(id).expect("register");
        assert!(register_convert(id).is_err());
        finish_convert(id);
        register_convert(id).expect("register after finish");
        finish_convert(id);
    }

    #[test]
    fn rejects_invalid_options() {
        let (root, dir) = test_project("options");
        seed_images(&dir, &["a.png"]);
        let bad_quality = ConvertOptions {
            quality: 0,
            ..valid_options()
        };
        assert!(convert_images_in(&dir, ImageTargetFormat::Jpeg, &bad_quality, None).is_err());
        let bad_level = ConvertOptions {
            png_level: 10,
            ..valid_options()
        };
        assert!(convert_images_in(&dir, ImageTargetFormat::Png, &bad_level, None).is_err());
        cleanup(&root);
    }

    #[test]
    fn preview_groups_by_source_format() {
        let (root, dir) = test_project("preview");
        seed_images(&dir, &["a.png", "b.PNG", "c.jpg", "d.jpeg"]);
        let preview = preview_conversion_in(&dir, ImageTargetFormat::Jpeg, false).expect("preview");
        assert_eq!(
            preview.items,
            vec![ConvertPreviewItem {
                from: "png".to_string(),
                count: 2,
            }]
        );
        assert_eq!(preview.total, 2);
        let forced = preview_conversion_in(&dir, ImageTargetFormat::Jpeg, true).expect("preview");
        assert_eq!(
            forced.items,
            vec![
                ConvertPreviewItem {
                    from: "jpg".to_string(),
                    count: 2,
                },
                ConvertPreviewItem {
                    from: "png".to_string(),
                    count: 2,
                },
            ]
        );
        assert_eq!(forced.total, 4);
        cleanup(&root);
    }

    #[test]
    fn converts_and_updates_content_and_clears_downstream() {
        let (root, dir) = test_project("convert");
        seed_images(&dir, &["a.png", "第1话/b.png", "c.jpg"]);
        // 下游残留：转换后应清空。
        std::fs::create_dir_all(dir.join("build")).expect("create build");
        std::fs::create_dir_all(dir.join("dist")).expect("create dist");
        let progress_calls = std::cell::Cell::new(0u32);
        let outline = convert_images_in(
            &dir,
            ImageTargetFormat::Jpeg,
            &valid_options(),
            Some(&|_done: u32, _total: u32| {
                progress_calls.set(progress_calls.get() + 1);
            }),
        )
        .expect("convert");
        assert_eq!(progress_calls.get(), 2);
        assert_eq!(read_paths(&dir), vec!["a.jpg", "第1话/b.jpg", "c.jpg"]);
        // 落盘确为 JPEG 且原 PNG 已删。
        let images = dir.join("sources").join("images");
        assert!(!images.join("a.png").exists());
        let magic = std::fs::read(images.join("a.jpg")).expect("read converted");
        assert!(magic.starts_with(&[0xFF, 0xD8]));
        // content.json 大小随新文件更新，大纲页数不变。
        assert_eq!(outline.page_count, 3);
        assert!(outline.volumes[0].pages.iter().all(|p| p.size_bytes > 0));
        // 暂存区清理，下游清空，content.json 保留。
        assert!(!dir.join(".tmp-convert").exists());
        assert!(!dir.join("build").exists());
        assert!(!dir.join("dist").exists());
        assert!(dir.join("content.json").is_file());
        cleanup(&root);
    }

    #[test]
    fn conflict_aborts_without_touching_anything() {
        let (root, dir) = test_project("conflict");
        seed_images(&dir, &["a.jpg", "a.png"]);
        let before = read_paths(&dir);
        let result = convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None);
        assert!(result.is_err());
        assert_eq!(read_paths(&dir), before);
        let images = dir.join("sources").join("images");
        assert!(images.join("a.jpg").is_file());
        assert!(images.join("a.png").is_file());
        assert!(!dir.join(".tmp-convert").exists());
        cleanup(&root);
    }

    #[test]
    fn corrupt_image_aborts_transactionally() {
        let (root, dir) = test_project("corrupt");
        seed_images(&dir, &["good.png", "bad.png", "other.png"]);
        // bad.png 写入垃圾字节。
        let images = dir.join("sources").join("images");
        std::fs::write(images.join("bad.png"), b"not-an-image").expect("corrupt image");
        let result = convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None);
        assert!(result.is_err());
        assert!(images.join("good.png").is_file());
        assert!(!images.join("good.jpg").exists());
        assert!(!images.join("other.jpg").exists());
        assert_eq!(read_paths(&dir), vec!["good.png", "bad.png", "other.png"]);
        assert!(!dir.join(".tmp-convert").exists());
        cleanup(&root);
    }

    #[test]
    fn noop_when_everything_matches_and_no_force() {
        let (root, dir) = test_project("noop");
        seed_images(&dir, &["a.jpg", "b.jpg"]);
        let outline = convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None)
            .expect("noop convert");
        assert_eq!(outline.page_count, 2);
        assert_eq!(read_paths(&dir), vec!["a.jpg", "b.jpg"]);
        cleanup(&root);
    }

    #[test]
    fn force_reencodes_same_format() {
        let (root, dir) = test_project("force");
        seed_images(&dir, &["a.jpg"]);
        let options = ConvertOptions {
            force: true,
            ..valid_options()
        };
        let outline =
            convert_images_in(&dir, ImageTargetFormat::Jpeg, &options, None).expect("convert");
        assert_eq!(outline.page_count, 1);
        let images = dir.join("sources").join("images");
        assert!(images.join("a.jpg").is_file());
        let magic = std::fs::read(images.join("a.jpg")).expect("read converted");
        assert!(magic.starts_with(&[0xFF, 0xD8]));
        cleanup(&root);
    }
}
