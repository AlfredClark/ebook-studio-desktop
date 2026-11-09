//! 漫画图片格式转换与压缩：将 `content.json` 全量页转码或原地重编码。
//!
//! 预览（按源格式分组计数）与执行（事务性两阶段）共用同一份扫描口径；
//! 转换成功后更新 `content.json` 的 path，压缩保持路径不变；两者都清空下游 `build/` 与 `dist/`。
//! 解码与 JPEG/PNG 编码走纯 Rust 的 `image`，WebP 有损走 libwebp 官方绑定。

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use image::{GenericImageView, ImageEncoder};
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

/// 进行中的压缩作业：与转换作业同构，暂存区独立（`.tmp-compress`）。
static COMPRESSING: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (u32, u32)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 取消标记：`cancelComicImages` 置旗，编码循环在文件边界处检查；
/// 进程重启即清空，与进行中注册表同一生命周期。
static CANCEL_REQUESTS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

/// 与导出共用的项目级独占锁，防止图片或 `content.json` 在读取期间被替换。
static PROJECT_IMAGE_LOCKS: std::sync::LazyLock<std::sync::Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));

/// 锁定一个项目的图片与页序操作；转换、压缩和导出共用该锁。
pub(super) fn lock_project_images(id: &str) -> anyhow::Result<()> {
    let mut projects = PROJECT_IMAGE_LOCKS
        .lock()
        .map_err(|_| anyhow::anyhow!("project image lock registry poisoned"))?;
    anyhow::ensure!(
        projects.insert(id.to_string()),
        "comic project operation already in progress for {id}"
    );
    drop(projects);
    Ok(())
}

/// 解锁项目图片操作；锁中毒时尽力结算。
pub(super) fn unlock_project_images(id: &str) {
    if let Ok(mut projects) = PROJECT_IMAGE_LOCKS.lock() {
        projects.remove(id);
    }
}

/// 查询项目图片是否被转换、压缩或导出任务独占。
#[cfg(test)]
fn project_images_locked(id: &str) -> bool {
    PROJECT_IMAGE_LOCKS
        .lock()
        .is_ok_and(|projects| projects.contains(id))
}

/// 同步项目操作的锁守卫；作用域结束时自动释放共享项目锁。
pub(super) struct ProjectImagesGuard {
    id: String,
}

impl ProjectImagesGuard {
    /// 获取项目级图片锁；已有导出、转换或压缩任务时直接失败。
    pub(super) fn acquire(id: &str) -> anyhow::Result<Self> {
        lock_project_images(id)?;
        Ok(Self { id: id.to_string() })
    }
}

impl Drop for ProjectImagesGuard {
    fn drop(&mut self) {
        unlock_project_images(&self.id);
    }
}

/// 取消信号：前后端约定的错误标记，入口处映射为 [`CommandError::Cancelled`]。
pub const CANCELLED_MARKER: &str = "cancelled by user";

/// 请求取消作业：在跑才置旗，调用方据返回值决定 toast 措辞。
pub fn request_cancel_images(id: &str) -> bool {
    let running = is_busy(id);
    if running && let Ok(mut flags) = CANCEL_REQUESTS.lock() {
        flags.insert(id.to_string());
    }
    running
}

/// 是否有取消请求：编码循环每文件检查一次，阶段二（快速改名）不检查。
fn is_cancel_requested(id: &str) -> bool {
    CANCEL_REQUESTS.lock().is_ok_and(|flags| flags.contains(id))
}

/// 清除取消标记：作业无论成功失败取消，结算时都清理，避免影响下次同项目作业。
fn clear_cancel(id: &str) {
    if let Ok(mut flags) = CANCEL_REQUESTS.lock() {
        flags.remove(id);
    }
}

/// 是否有转换在跑（解析 / 导入互斥用）。
pub fn is_converting(id: &str) -> bool {
    CONVERTING.lock().is_ok_and(|jobs| jobs.contains_key(id))
}

/// 是否有压缩在跑（解析 / 导入 / 转换互斥用）。
pub fn is_compressing(id: &str) -> bool {
    COMPRESSING.lock().is_ok_and(|jobs| jobs.contains_key(id))
}

/// 是否有图片作业在跑：转换或压缩任一在跑即忙，两者互斥。
pub fn is_busy(id: &str) -> bool {
    is_converting(id) || is_compressing(id)
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
    lock_project_images(id)?;
    let result = (|| {
        let mut jobs = CONVERTING
            .lock()
            .map_err(|_| anyhow::anyhow!("convert registry poisoned"))?;
        anyhow::ensure!(
            !jobs.contains_key(id),
            "conversion already in progress for project {id}"
        );
        jobs.insert(id.to_string(), (0, 0));
        drop(jobs);
        Ok(())
    })();
    if result.is_err() {
        unlock_project_images(id);
    }
    result
}

/// 更新作业进度：注册表缺失时静默跳过，只写不读，不影响主流程。
fn update_convert_progress(id: &str, done: u32, total: u32) {
    if let Ok(mut jobs) = CONVERTING.lock()
        && let Some(entry) = jobs.get_mut(id)
    {
        *entry = (done, total);
    }
}

/// 注销作业：成功失败取消都走，锁中毒也尽力清理；同时清取消标记。
fn finish_convert(id: &str) {
    if let Ok(mut jobs) = CONVERTING.lock() {
        jobs.remove(id);
    }
    clear_cancel(id);
    unlock_project_images(id);
}

/// 压缩参数：原地重编码，保持源格式与路径不变。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CompressOptions {
    /// 缩放比例（相对当前尺寸等比），25-100，100 为不缩放。
    pub scale_percent: u8,
    /// 灰度转换开关；已灰度图在无需缩放时跳过。
    pub grayscale: bool,
    /// JPEG/WebP 有损质量，1-100。
    pub quality: u8,
    /// WebP 无损开关（开则忽略质量）。
    pub webp_lossless: bool,
    /// PNG 压缩级别，0-9。
    pub png_level: u8,
}

impl CompressOptions {
    fn validate(&self) -> anyhow::Result<()> {
        if !(25..=100).contains(&self.scale_percent) {
            anyhow::bail!(
                "scale_percent must be within 25..=100, got {}",
                self.scale_percent
            );
        }
        if !(1..=100).contains(&self.quality) {
            anyhow::bail!("quality must be within 1..=100, got {}", self.quality);
        }
        if self.png_level > 9 {
            anyhow::bail!("png_level must be within 0..=9, got {}", self.png_level);
        }
        Ok(())
    }
}

/// 压缩预览：只读 `content.json` + 文件大小，不解码图片。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CompressPreview {
    pub total: u32,
    pub total_bytes: u32,
}

/// 压缩结果：原地覆盖后的大纲 + 何度跳过剩下的灰度。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CompressOutcome {
    pub outline: ComicOutline,
    pub processed: u32,
    pub skipped: u32,
}

/// 压缩进度事件名：与转换事件独立，前端 hook 分开订阅。
pub const COMPRESS_PROGRESS_EVENT: &str = "comic-images-compress-progress";

/// 压缩进度载荷。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ComicCompressProgress {
    pub id: String,
    pub done_files: u32,
    pub total_files: u32,
}

/// 压缩状态探针：供切回页面时回填进度；从未开始返回空闲零值。
/// `has_backup` 供工具条“撤销/清除备份”按钮显隐，与进度无关、每次探针实时计算。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ComicCompressStatus {
    pub running: bool,
    pub done_files: u32,
    pub total_files: u32,
    pub has_backup: bool,
}

/// 查询压缩作业状态：未在跑回空闲零值。
pub fn compress_status(id: &str) -> ComicCompressStatus {
    COMPRESSING
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(id).copied())
        .map_or_else(
            || ComicCompressStatus {
                running: false,
                done_files: 0,
                total_files: 0,
                has_backup: false,
            },
            |(done_files, total_files)| ComicCompressStatus {
                running: true,
                done_files,
                total_files,
                has_backup: false,
            },
        )
}

/// 登记压缩作业：转换/压缩互斥，同项目已有在跑任务整单拒绝。
fn register_compress(id: &str) -> anyhow::Result<()> {
    lock_project_images(id)?;
    let result = (|| {
        let mut jobs = COMPRESSING
            .lock()
            .map_err(|_| anyhow::anyhow!("compress registry poisoned"))?;
        anyhow::ensure!(
            !jobs.contains_key(id),
            "compression already in progress for project {id}"
        );
        jobs.insert(id.to_string(), (0, 0));
        drop(jobs);
        Ok(())
    })();
    if result.is_err() {
        unlock_project_images(id);
    }
    result
}

/// 更新压缩作业进度：注册表缺失时静默跳过。
fn update_compress_progress(id: &str, done: u32, total: u32) {
    if let Ok(mut jobs) = COMPRESSING.lock()
        && let Some(entry) = jobs.get_mut(id)
    {
        *entry = (done, total);
    }
}

/// 注销压缩作业：成功失败取消都走；同时清取消标记。
fn finish_compress(id: &str) {
    if let Ok(mut jobs) = COMPRESSING.lock() {
        jobs.remove(id);
    }
    clear_cancel(id);
    unlock_project_images(id);
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

/// 单张编码：JPEG 去 alpha（灰度按 L8 落盘更小）；PNG 保留原色彩类型；WebP 按开关选有损 / 无损。
fn encode_image(
    img: &image::DynamicImage,
    target: ImageTargetFormat,
    options: &ConvertOptions,
) -> anyhow::Result<Vec<u8>> {
    match target {
        ImageTargetFormat::Jpeg => {
            // 灰度图按 L8 编码：体积显著小于 RGB 展开。
            if matches!(img.color(), image::ColorType::L8 | image::ColorType::La8) {
                let gray = img.to_luma8();
                let mut buf = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, options.quality)
                    .encode(
                        &gray,
                        gray.width(),
                        gray.height(),
                        image::ExtendedColorType::L8,
                    )
                    .context("failed to encode JPEG")?;
                return Ok(buf);
            }
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

/// 取消映射：含取消标记的失败转 `Cancelled`，其余走 `Internal`。
fn map_job_error(err: &anyhow::Error) -> crate::cores::types::CommandError {
    let text = format!("{err:#}");
    if text.contains(CANCELLED_MARKER) {
        crate::cores::types::CommandError::Cancelled(text)
    } else {
        crate::cores::types::CommandError::Internal(text)
    }
}

/// 请求取消图片作业（`AppHandle` 入口）：转换/压缩互斥，任一在跑即置旗；
/// 返回是否有在跑作业供前端措辞，无作业时不做任何事。
pub fn cancel_images(app: &tauri::AppHandle, id: &str) -> anyhow::Result<bool> {
    let _dir = super::projects::resolve_project_dir(app, id)?;
    Ok(request_cancel_images(id))
}

/// 格式转换（`AppHandle` 入口）：阻塞池执行并推送逐文件进度，前端据此渲染进度。
pub async fn convert_images(
    app: &tauri::AppHandle,
    id: &str,
    target: ImageTargetFormat,
    options: &ConvertOptions,
) -> Result<ComicOutline, crate::cores::types::CommandError> {
    use tauri::Emitter;

    let dir = super::projects::resolve_project_dir(app, id)?;
    register_convert(id)?;
    let options = options.clone();
    let handle = app.clone();
    let report_id = id.to_string();
    let progress_id = id.to_string();
    let cancel_id = id.to_string();
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
        let should_cancel = || is_cancel_requested(&cancel_id);
        convert_images_in(
            &dir,
            target,
            &options,
            Some(&on_progress),
            Some(&should_cancel),
        )
    })
    .await;
    finish_convert(id);
    match outcome.map_err(|err| crate::cores::types::CommandError::Internal(format!("{err:#}"))) {
        Ok(result) => result.map_err(|err| map_job_error(&err)),
        Err(err) => Err(err),
    }
}

/// 格式转换内核：事务性两阶段（暂存区编码→批量替换），成功后更新
/// `content.json` 的 path 并清空下游 `build/` 与 `dist/`（`content.json` 保留）。
fn convert_images_in(
    project_dir: &Path,
    target: ImageTargetFormat,
    options: &ConvertOptions,
    progress: Option<&dyn Fn(u32, u32)>,
    should_cancel: Option<&dyn Fn() -> bool>,
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
    // 格式转换改写路径与字节：旧压缩备份不再对应当前文件，直接失效。
    clear_compress_backup(project_dir);
    // 阶段一：全部解码编码到暂存区；任一步失败清理暂存区、原状不动。
    let staging = project_dir.join(STAGING_DIR);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).context("failed to create convert staging dir")?;
    let staged = stage_all(
        &images,
        &staging,
        &jobs,
        target,
        options,
        progress,
        should_cancel,
    )
    .inspect_err(|_| {
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
    super::projects::touch_updated(project_dir);
    comic_files::outline_in(project_dir)?
        .with_context(|| "content.json missing after convert".to_string())
}

/// 阶段一：逐张解码编码进暂存区并上报进度；取消标记在文件边界处检查。
fn stage_all(
    images: &Path,
    staging: &Path,
    jobs: &[ConvertJob],
    target: ImageTargetFormat,
    options: &ConvertOptions,
    progress: Option<&dyn Fn(u32, u32)>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> anyhow::Result<Vec<PathBuf>> {
    let total = u32::try_from(jobs.len()).unwrap_or(u32::MAX);
    let mut staged = Vec::with_capacity(jobs.len());
    for (index, job) in jobs.iter().enumerate() {
        if should_cancel.is_some_and(|cancelled| cancelled()) {
            anyhow::bail!(CANCELLED_MARKER);
        }
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

/// 压缩暂存目录：与转换暂存区独立，避免互斥作业互相清理。
const COMPRESS_STAGING_DIR: &str = ".tmp-compress";

/// 压缩备份目录：阶段二每次覆盖前先把原图拷一份进来（仅实际改写的文件），
/// 撤销时拷回；一次性有效，恢复/清除/新压缩/格式转换/重解析时整目录删除。
const COMPRESS_BACKUP_DIR: &str = ".bak-images";

/// 备份目录下拼接相对路径：与 `join_images_path` 同口径（防路径穿越、最多一层话隔板）。
fn backup_images_path(project_dir: &Path, rel: &str) -> anyhow::Result<std::path::PathBuf> {
    let images = project_dir.join(COMPRESS_BACKUP_DIR);
    comic_files::join_images_path(&images, rel)
}

/// 是否有压缩备份：备份目录存在即真（恢复/清除后删除）。
pub fn has_compress_backup(project_dir: &Path) -> bool {
    project_dir.join(COMPRESS_BACKUP_DIR).is_dir()
}

/// 清除压缩备份：新压缩开始、格式转换、重解析/换源时调用，旧快照不再能安全还原。
pub fn clear_compress_backup(project_dir: &Path) {
    let _ = std::fs::remove_dir_all(project_dir.join(COMPRESS_BACKUP_DIR));
}

/// 灰度判定：Luma 系直接真，RGB 系逐像素比对（忽略 alpha）。
fn is_grayscale(img: &image::DynamicImage) -> bool {
    use image::ColorType;
    match img.color() {
        ColorType::L8 | ColorType::L16 | ColorType::La8 | ColorType::La16 => true,
        _ => img.pixels().all(|(_, _, pixel)| {
            let channels = pixel.0;
            channels[0] == channels[1] && channels[1] == channels[2]
        }),
    }
}

/// 源扩展名映射回目标格式：压缩保持格式不变，未知格式报错。
fn target_of_ext(ext: &str) -> anyhow::Result<ImageTargetFormat> {
    match ext {
        "jpg" => Ok(ImageTargetFormat::Jpeg),
        "png" => Ok(ImageTargetFormat::Png),
        "webp" => Ok(ImageTargetFormat::Webp),
        _ => anyhow::bail!("unsupported image format: {ext}"),
    }
}

/// 压缩变换：先等比缩放（`scale == 100` 跳过），再按开关灰度。
fn apply_compress_transform(
    img: image::DynamicImage,
    options: &CompressOptions,
) -> image::DynamicImage {
    let mut current = img;
    if options.scale_percent != 100 {
        let (width, height) = current.dimensions();
        let scale = u64::from(options.scale_percent);
        let new_width =
            u32::try_from(((u64::from(width) * scale + 50) / 100).clamp(1, u64::from(width)))
                .unwrap_or(width);
        let new_height =
            u32::try_from(((u64::from(height) * scale + 50) / 100).clamp(1, u64::from(height)))
                .unwrap_or(height);
        if new_width != width || new_height != height {
            current = current.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        }
    }
    if options.grayscale {
        current = current.grayscale();
    }
    current
}

/// 预览压缩：只读 `content.json` + 文件大小，不解码图片。
pub fn preview_compression_in(project_dir: &Path) -> anyhow::Result<CompressPreview> {
    let content = comic_files::read_content(project_dir)?;
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let mut total = 0u32;
    let mut total_bytes = 0u32;
    for volume in &content.volumes {
        for page in &volume.pages {
            let ext = normalized_ext(&page.path)?;
            if !matches!(ext.as_str(), "jpg" | "png" | "webp") {
                anyhow::bail!("unsupported image format: {}", page.path);
            }
            total = total.saturating_add(1);
            let abs = comic_files::join_images_path(&images, &page.path)?;
            let size = std::fs::metadata(&abs).map_or(0, |meta| meta.len());
            total_bytes = total_bytes.saturating_add(u32::try_from(size).unwrap_or(u32::MAX));
        }
    }
    Ok(CompressPreview { total, total_bytes })
}

/// 预览压缩（`AppHandle` 入口）。
pub fn preview_compression(app: &tauri::AppHandle, id: &str) -> anyhow::Result<CompressPreview> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    preview_compression_in(&dir)
}

/// 查询压缩状态（`AppHandle` 入口）：供切回页面时回填进度；备份位实时计算。
pub fn compress_status_of(project_dir: &Path, id: &str) -> ComicCompressStatus {
    let mut status = compress_status(id);
    status.has_backup = has_compress_backup(project_dir);
    status
}

/// 撤销压缩备份（`AppHandle` 入口）：一次性有效，无备份直接报错。
pub fn restore_backup(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicOutline> {
    let _lock = ProjectImagesGuard::acquire(id)?;
    let dir = super::projects::resolve_project_dir(app, id)?;
    restore_compress_backup_in(&dir)
}

/// 清除压缩备份（`AppHandle` 入口）：只删备份目录，当前图片不动。
pub fn clear_backup(app: &tauri::AppHandle, id: &str) -> anyhow::Result<bool> {
    let _lock = ProjectImagesGuard::acquire(id)?;
    let dir = super::projects::resolve_project_dir(app, id)?;
    clear_compress_backup_in(&dir)
}

/// 压缩全量图片（`AppHandle` 入口）：阻塞池执行并推送逐文件进度。
pub async fn compress_images(
    app: &tauri::AppHandle,
    id: &str,
    options: &CompressOptions,
) -> Result<CompressOutcome, crate::cores::types::CommandError> {
    use tauri::Emitter;

    let dir = super::projects::resolve_project_dir(app, id)?;
    register_compress(id)?;
    let options = options.clone();
    let handle = app.clone();
    let report_id = id.to_string();
    let progress_id = id.to_string();
    let cancel_id = id.to_string();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let on_progress = move |done: u32, total: u32| {
            update_compress_progress(&progress_id, done, total);
            let _ = handle.emit(
                COMPRESS_PROGRESS_EVENT,
                ComicCompressProgress {
                    id: report_id.clone(),
                    done_files: done,
                    total_files: total,
                },
            );
        };
        let should_cancel = || is_cancel_requested(&cancel_id);
        compress_images_in(&dir, &options, Some(&on_progress), Some(&should_cancel))
    })
    .await;
    finish_compress(id);
    match outcome.map_err(|err| crate::cores::types::CommandError::Internal(format!("{err:#}"))) {
        Ok(result) => result.map_err(|err| map_job_error(&err)),
        Err(err) => Err(err),
    }
}

/// 撤销压缩：把备份原图拷回 `sources/images`，一次性有效（恢复后删备份）；
/// 无备份直接报错，前端据 `has_backup` 决定是否展示入口。
/// 注意：恢复的是压缩前字节，`content.json` 路径不变故无需改写。
pub fn restore_compress_backup_in(project_dir: &Path) -> anyhow::Result<ComicOutline> {
    let backup = project_dir.join(COMPRESS_BACKUP_DIR);
    if !backup.is_dir() {
        anyhow::bail!("no compress backup to restore");
    }
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let mut restored = 0u32;
    let mut stack = vec![backup.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .with_context(|| format!("failed to read backup dir {}", dir.display()))?;
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
            let rel = entry
                .path()
                .strip_prefix(&backup)
                .with_context(|| format!("bad backup path {}", entry.path().display()))?
                .to_str()
                .with_context(|| format!("bad backup name {}", entry.path().display()))?
                .replace('\\', "/");
            let dest = comic_files::join_images_path(&images, &rel)?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            std::fs::copy(entry.path(), &dest).with_context(|| {
                format!(
                    "failed to restore {} to {}",
                    entry.path().display(),
                    dest.display()
                )
            })?;
            restored = restored.saturating_add(1);
        }
    }
    if restored == 0 {
        anyhow::bail!("compress backup is empty");
    }
    let _ = std::fs::remove_dir_all(&backup);
    // 恢复的是旧字节，下游产物全部失效。
    let _ = std::fs::remove_dir_all(project_dir.join(BUILD_DIR));
    let _ = std::fs::remove_dir_all(project_dir.join(DIST_DIR));
    super::projects::touch_updated(project_dir);
    comic_files::outline_in(project_dir)?
        .with_context(|| "content.json missing after restore".to_string())
}

/// 清除压缩备份：只删备份目录，当前图片不动；无备份返回假。
pub fn clear_compress_backup_in(project_dir: &Path) -> anyhow::Result<bool> {
    let backup = project_dir.join(COMPRESS_BACKUP_DIR);
    if !backup.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(&backup).context("failed to clear compress backup")?;
    super::projects::touch_updated(project_dir);
    Ok(true)
}
/// 成功后清空下游 `build/` 与 `dist/`。已灰度且无需缩放的张直接跳过。
/// 压缩内核：事务性两阶段（暂存区编码→原地覆盖），路径不变故不写 `content.json`；
/// 成功后清空下游 `build/` 与 `dist/`。已灰度且无需缩放的张直接跳过。
fn compress_images_in(
    project_dir: &Path,
    options: &CompressOptions,
    progress: Option<&dyn Fn(u32, u32)>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> anyhow::Result<CompressOutcome> {
    options.validate()?;
    let content = comic_files::read_content(project_dir)?;
    let rels: Vec<String> = content
        .volumes
        .iter()
        .flat_map(|volume| volume.pages.iter().map(|page| page.path.clone()))
        .collect();
    if rels.is_empty() {
        let outline = comic_files::outline_in(project_dir)?
            .with_context(|| "content.json missing, parse the images first".to_string())?;
        return Ok(CompressOutcome {
            outline,
            processed: 0,
            skipped: 0,
        });
    }
    let images = project_dir.join(SOURCES_DIR).join(IMAGES_DIR);
    let staging = project_dir.join(COMPRESS_STAGING_DIR);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).context("failed to create compress staging dir")?;
    // 新压缩开始即失效旧备份：原图集合即将变化，旧快照不再对应当前文件。
    clear_compress_backup(project_dir);
    let outcome = stage_compress_all(&images, &staging, &rels, options, progress, should_cancel)
        .inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&staging);
        })?;
    for (rel, staged) in rels.iter().zip(outcome.staged.iter()) {
        let Some(path) = staged else { continue };
        let dest = comic_files::join_images_path(&images, rel)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        // 先备份原图再覆盖：撤销时按原路径拷回；跳过的文件不碰，无需备份。
        let backup = backup_images_path(project_dir, rel)?;
        if let Some(parent) = backup.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::copy(&dest, &backup)
            .with_context(|| format!("failed to back up {}", dest.display()))?;
        std::fs::rename(path, &dest)
            .with_context(|| format!("failed to move {} to {}", path.display(), dest.display()))?;
    }
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(project_dir.join(BUILD_DIR));
    let _ = std::fs::remove_dir_all(project_dir.join(DIST_DIR));
    let outline = comic_files::outline_in(project_dir)?
        .with_context(|| "content.json missing after compress".to_string())?;
    super::projects::touch_updated(project_dir);
    Ok(CompressOutcome {
        outline,
        processed: outcome.processed,
        skipped: outcome.skipped,
    })
}

struct CompressStageOutcome {
    staged: Vec<Option<PathBuf>>,
    processed: u32,
    skipped: u32,
}

/// 压缩阶段一：逐张解码→判灰跳过→变换→按源格式编码进暂存区；取消标记在文件边界处检查。
fn stage_compress_all(
    images: &Path,
    staging: &Path,
    rels: &[String],
    options: &CompressOptions,
    progress: Option<&dyn Fn(u32, u32)>,
    should_cancel: Option<&dyn Fn() -> bool>,
) -> anyhow::Result<CompressStageOutcome> {
    let total = u32::try_from(rels.len()).unwrap_or(u32::MAX);
    // 质量参数复用转换编码器：按源格式分发，其余忽略。
    let encode_options = ConvertOptions {
        quality: options.quality,
        webp_lossless: options.webp_lossless,
        png_level: options.png_level,
        force: true,
    };
    let mut staged = Vec::with_capacity(rels.len());
    let mut processed = 0u32;
    let mut skipped = 0u32;
    for (index, rel) in rels.iter().enumerate() {
        if should_cancel.is_some_and(|cancelled| cancelled()) {
            anyhow::bail!(CANCELLED_MARKER);
        }
        let ext = normalized_ext(rel)?;
        let target = target_of_ext(&ext)?;
        let src = comic_files::join_images_path(images, rel)?;
        let bytes =
            std::fs::read(&src).with_context(|| format!("failed to read {}", src.display()))?;
        let img =
            image::load_from_memory(&bytes).with_context(|| format!("failed to decode {rel}"))?;
        // 已灰度且无需缩放：跳过重编码（`quality` 变化亦跳过，见确认弹窗明示）。
        if options.grayscale && options.scale_percent == 100 && is_grayscale(&img) {
            staged.push(None);
            skipped = skipped.saturating_add(1);
        } else {
            let transformed = apply_compress_transform(img, options);
            let encoded = encode_image(&transformed, target, &encode_options)?;
            let staged_path = staging.join(format!("{index:06}"));
            std::fs::write(&staged_path, &encoded)
                .with_context(|| format!("failed to stage {}", staged_path.display()))?;
            staged.push(Some(staged_path));
            processed = processed.saturating_add(1);
        }
        if let Some(report) = progress {
            let done = u32::try_from(index + 1).unwrap_or(u32::MAX);
            report(done, total);
        }
    }
    Ok(CompressStageOutcome {
        staged,
        processed,
        skipped,
    })
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
        assert!(project_images_locked(id));
        let status = convert_status(id);
        assert!(status.running);
        assert_eq!((status.done_files, status.total_files), (0, 0));
        update_convert_progress(id, 3, 10);
        let status = convert_status(id);
        assert_eq!((status.done_files, status.total_files), (3, 10));
        finish_convert(id);
        assert!(!is_converting(id));
        assert!(!project_images_locked(id));
    }

    #[test]
    fn synchronous_project_image_guard_releases_on_scope_exit() {
        let id = "sync-project-image-lock";
        {
            let _guard = ProjectImagesGuard::acquire(id).expect("acquire project lock");
            assert!(project_images_locked(id));
            assert!(lock_project_images(id).is_err());
        }
        assert!(!project_images_locked(id));
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
    fn cancel_flag_lifecycle() {
        let id = "cancel-flag-lifecycle";
        // 无在跑作业：不置旗，返回 false。
        assert!(!request_cancel_images(id));
        register_convert(id).expect("register");
        assert!(request_cancel_images(id));
        assert!(is_cancel_requested(id));
        // 结算即清理：下次同项目作业不受影响。
        finish_convert(id);
        assert!(!is_cancel_requested(id));
        assert!(!request_cancel_images(id));
    }

    #[test]
    fn convert_aborts_on_cancel_before_touching_anything() {
        let (root, dir) = test_project("cancel-convert");
        seed_images(&dir, &["a.png", "b.png"]);
        let result = convert_images_in(
            &dir,
            ImageTargetFormat::Jpeg,
            &valid_options(),
            None,
            Some(&|| true),
        );
        let err = format!("{:#}", result.expect_err("cancelled convert"));
        assert!(err.contains(CANCELLED_MARKER));
        // 原子性：content.json 与源文件都未动，暂存区已清理。
        assert_eq!(
            read_paths(&dir),
            vec!["a.png".to_string(), "b.png".to_string()]
        );
        assert!(!dir.join(".tmp-convert").exists());
        cleanup(&root);
    }

    #[test]
    fn compress_aborts_on_cancel_before_touching_anything() {
        let (root, dir) = test_project("cancel-compress");
        seed_images(&dir, &["a.png", "b.png"]);
        let before: Vec<u8> =
            std::fs::read(dir.join("sources").join("images").join("a.png")).expect("read image");
        let result = compress_images_in(&dir, &valid_compress_options(), None, Some(&|| true));
        let err = format!("{:#}", result.expect_err("cancelled compress"));
        assert!(err.contains(CANCELLED_MARKER));
        // 原地覆盖未发生：字节一致，暂存区已清理。
        let after: Vec<u8> =
            std::fs::read(dir.join("sources").join("images").join("a.png")).expect("read image");
        assert_eq!(before, after);
        assert!(!dir.join(".tmp-compress").exists());
        cleanup(&root);
    }

    #[test]
    fn cancel_marker_maps_to_cancelled_error() {
        let err = map_job_error(&anyhow::anyhow!(CANCELLED_MARKER));
        assert!(matches!(
            err,
            crate::cores::types::CommandError::Cancelled(_)
        ));
        let err = map_job_error(&anyhow::anyhow!("boom"));
        assert!(matches!(
            err,
            crate::cores::types::CommandError::Internal(_)
        ));
    }

    #[test]
    fn rejects_invalid_options() {
        let (root, dir) = test_project("options");
        seed_images(&dir, &["a.png"]);
        let bad_quality = ConvertOptions {
            quality: 0,
            ..valid_options()
        };
        assert!(
            convert_images_in(&dir, ImageTargetFormat::Jpeg, &bad_quality, None, None).is_err()
        );
        let bad_level = ConvertOptions {
            png_level: 10,
            ..valid_options()
        };
        assert!(convert_images_in(&dir, ImageTargetFormat::Png, &bad_level, None, None).is_err());
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
            None,
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
        let result = convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None, None);
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
        let result = convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None, None);
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
        let outline =
            convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None, None)
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
        let outline = convert_images_in(&dir, ImageTargetFormat::Jpeg, &options, None, None)
            .expect("convert");
        assert_eq!(outline.page_count, 1);
        let images = dir.join("sources").join("images");
        assert!(images.join("a.jpg").is_file());
        let magic = std::fs::read(images.join("a.jpg")).expect("read converted");
        assert!(magic.starts_with(&[0xFF, 0xD8]));
        cleanup(&root);
    }

    fn valid_compress_options() -> CompressOptions {
        CompressOptions {
            scale_percent: 100,
            grayscale: false,
            quality: 85,
            webp_lossless: false,
            png_level: 6,
        }
    }

    /// 写一张指定尺寸的彩色图，供缩放断言用。
    fn write_sized_image(dir: &Path, rel: &str, width: u32, height: u32) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create image parent");
        }
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([20, 120, 200]));
        image::DynamicImage::ImageRgb8(img)
            .save(&path)
            .expect("save test image");
    }

    /// 写一张灰度图（L8 落盘），供跳过断言用。
    fn write_gray_image(dir: &Path, rel: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create image parent");
        }
        let img = image::GrayImage::from_pixel(4, 4, image::Luma([128]));
        image::DynamicImage::ImageLuma8(img)
            .save(&path)
            .expect("save test image");
    }

    #[test]
    fn compress_rejects_invalid_options() {
        let (root, dir) = test_project("compress-options");
        seed_images(&dir, &["a.png"]);
        let bad_scale = CompressOptions {
            scale_percent: 24,
            ..valid_compress_options()
        };
        assert!(compress_images_in(&dir, &bad_scale, None, None).is_err());
        let bad_scale_high = CompressOptions {
            scale_percent: 101,
            ..valid_compress_options()
        };
        assert!(compress_images_in(&dir, &bad_scale_high, None, None).is_err());
        let bad_quality = CompressOptions {
            quality: 0,
            ..valid_compress_options()
        };
        assert!(compress_images_in(&dir, &bad_quality, None, None).is_err());
        cleanup(&root);
    }

    #[test]
    fn compress_registry_lifecycle_and_mutex() {
        let id = "compress-registry-lifecycle";
        assert!(!is_compressing(id));
        assert!(!is_busy(id));
        register_compress(id).expect("register");
        assert!(is_compressing(id));
        assert!(is_busy(id));
        // 转换与压缩互斥：任一在跑另一整单拒绝。
        assert!(register_convert(id).is_err());
        finish_compress(id);
        assert!(!is_busy(id));
        register_convert(id).expect("convert after compress");
        assert!(register_compress(id).is_err());
        finish_convert(id);
        assert!(!is_busy(id));
    }

    #[test]
    fn compress_backup_restore_and_clear_lifecycle() {
        let (root, dir) = test_project("compress-backup");
        seed_images(&dir, &["a.png", "b.png"]);
        let images = dir.join("sources").join("images");
        let before: Vec<u8> = std::fs::read(images.join("a.png")).expect("read original");
        assert!(!has_compress_backup(&dir));

        let mut scaled = valid_compress_options();
        scaled.scale_percent = 50;
        let outcome = compress_images_in(&dir, &scaled, None, None).expect("compress");
        assert_eq!(outcome.processed, 2);
        // 备份落盘：仅实际改写的文件，原字节一致。
        assert!(has_compress_backup(&dir));
        let backed: Vec<u8> =
            std::fs::read(dir.join(".bak-images").join("a.png")).expect("read backup");
        assert_eq!(backed, before);
        let after: Vec<u8> = std::fs::read(images.join("a.png")).expect("read compressed");
        assert_ne!(after, before);

        // 撤销：字节恢复，备份一次性用完。
        let outline = restore_compress_backup_in(&dir).expect("restore");
        assert_eq!(outline.page_count, 2);
        let restored: Vec<u8> = std::fs::read(images.join("a.png")).expect("read restored");
        assert_eq!(restored, before);
        assert!(!has_compress_backup(&dir));
        assert!(restore_compress_backup_in(&dir).is_err());

        // 清除：有备份回真并删目录，当前图片不动；无备份回假。
        compress_images_in(&dir, &scaled, None, None).expect("compress again");
        assert!(has_compress_backup(&dir));
        assert!(clear_compress_backup_in(&dir).expect("clear"));
        assert!(!has_compress_backup(&dir));
        assert!(!clear_compress_backup_in(&dir).expect("clear empty"));
        cleanup(&root);
    }

    #[test]
    fn skipped_files_need_no_backup() {
        let (root, dir) = test_project("compress-backup-skip");
        seed_images(&dir, &["a.png"]);
        let mut gray_only = valid_compress_options();
        gray_only.grayscale = true;
        gray_only.scale_percent = 100;
        // 非灰度图仍会处理；断言点在于跳过路径不写备份——用全跳过场景：
        // 灰度开关 + 不缩放 + 已灰度图。先压成灰度，再跑一次即全跳过。
        compress_images_in(&dir, &gray_only, None, None).expect("first compress");
        assert!(has_compress_backup(&dir));
        clear_compress_backup_in(&dir).expect("clear");
        let outcome = compress_images_in(&dir, &gray_only, None, None).expect("second compress");
        assert_eq!(outcome.processed, 0);
        assert!(!has_compress_backup(&dir));
        cleanup(&root);
    }

    #[test]
    fn convert_invalidates_compress_backup() {
        let (root, dir) = test_project("compress-backup-invalidate");
        seed_images(&dir, &["a.png"]);
        let mut scaled = valid_compress_options();
        scaled.scale_percent = 50;
        compress_images_in(&dir, &scaled, None, None).expect("compress");
        assert!(has_compress_backup(&dir));
        // 格式转换改写路径与字节：旧备份直接失效。
        convert_images_in(&dir, ImageTargetFormat::Jpeg, &valid_options(), None, None)
            .expect("convert");
        assert!(!has_compress_backup(&dir));
        assert!(!dir.join(".bak-images").exists());
        cleanup(&root);
    }

    #[test]
    fn compress_preview_counts_pages_without_decoding() {
        let (root, dir) = test_project("compress-preview");
        seed_images(&dir, &["a.png", "b.jpg"]);
        let preview = preview_compression_in(&dir).expect("preview");
        assert_eq!(preview.total, 2);
        assert!(preview.total_bytes > 0);
        cleanup(&root);
    }

    #[test]
    fn compress_scales_and_keeps_paths_and_clears_downstream() {
        let (root, dir) = test_project("compress-scale");
        let images = dir.join("sources").join("images");
        write_sized_image(&images, "a.png", 8, 8);
        // 最小 content.json：单张 8x8。
        let content = ComicContent {
            schema_version: 1,
            mode: ComicMode::Flat,
            parsed_at: "2026-01-01T00:00:00Z".to_string(),
            volumes: vec![ComicVolume {
                title: None,
                pages: vec![ComicPage {
                    path: "a.png".to_string(),
                }],
            }],
        };
        let text = serde_json::to_string_pretty(&content).expect("serialize content");
        std::fs::write(dir.join("content.json"), text).expect("write content");
        std::fs::create_dir_all(dir.join("build")).expect("create build");
        std::fs::create_dir_all(dir.join("dist")).expect("create dist");
        let options = CompressOptions {
            scale_percent: 50,
            ..valid_compress_options()
        };
        let outcome = compress_images_in(&dir, &options, None, None).expect("compress");
        assert_eq!(outcome.processed, 1);
        assert_eq!(outcome.skipped, 0);
        // 路径不变，尺寸减半。
        assert_eq!(read_paths(&dir), vec!["a.png"]);
        let bytes = std::fs::read(images.join("a.png")).expect("read compressed");
        let img = image::load_from_memory(&bytes).expect("decode compressed");
        assert_eq!(img.dimensions(), (4, 4));
        assert_eq!(outcome.outline.page_count, 1);
        assert!(!dir.join(".tmp-compress").exists());
        assert!(!dir.join("build").exists());
        assert!(!dir.join("dist").exists());
        cleanup(&root);
    }

    #[test]
    fn compress_skips_already_gray_when_no_scale() {
        let (root, dir) = test_project("compress-gray-skip");
        let images = dir.join("sources").join("images");
        write_gray_image(&images, "gray.png");
        write_sized_image(&images, "color.png", 4, 4);
        let content = ComicContent {
            schema_version: 1,
            mode: ComicMode::Flat,
            parsed_at: "2026-01-01T00:00:00Z".to_string(),
            volumes: vec![ComicVolume {
                title: None,
                pages: vec![
                    ComicPage {
                        path: "gray.png".to_string(),
                    },
                    ComicPage {
                        path: "color.png".to_string(),
                    },
                ],
            }],
        };
        let text = serde_json::to_string_pretty(&content).expect("serialize content");
        std::fs::write(dir.join("content.json"), text).expect("write content");
        let before_gray = std::fs::read(images.join("gray.png")).expect("read gray");
        let options = CompressOptions {
            grayscale: true,
            ..valid_compress_options()
        };
        let outcome = compress_images_in(&dir, &options, None, None).expect("compress");
        assert_eq!(outcome.processed, 1);
        assert_eq!(outcome.skipped, 1);
        // 已灰度原文件不动，彩色图被灰度化。
        let after_gray = std::fs::read(images.join("gray.png")).expect("read gray after");
        assert_eq!(before_gray, after_gray);
        let color_bytes = std::fs::read(images.join("color.png")).expect("read color");
        let color_img = image::load_from_memory(&color_bytes).expect("decode color");
        assert!(is_grayscale(&color_img));
        cleanup(&root);
    }

    #[test]
    fn compress_does_not_skip_gray_when_scaling() {
        let (root, dir) = test_project("compress-gray-scale");
        let images = dir.join("sources").join("images");
        write_gray_image(&images, "gray.png");
        let content = ComicContent {
            schema_version: 1,
            mode: ComicMode::Flat,
            parsed_at: "2026-01-01T00:00:00Z".to_string(),
            volumes: vec![ComicVolume {
                title: None,
                pages: vec![ComicPage {
                    path: "gray.png".to_string(),
                }],
            }],
        };
        let text = serde_json::to_string_pretty(&content).expect("serialize content");
        std::fs::write(dir.join("content.json"), text).expect("write content");
        let options = CompressOptions {
            scale_percent: 50,
            grayscale: true,
            ..valid_compress_options()
        };
        let outcome = compress_images_in(&dir, &options, None, None).expect("compress");
        assert_eq!((outcome.processed, outcome.skipped), (1, 0));
        cleanup(&root);
    }

    #[test]
    fn compress_corrupt_aborts_transactionally() {
        let (root, dir) = test_project("compress-corrupt");
        seed_images(&dir, &["good.png", "bad.png"]);
        let images = dir.join("sources").join("images");
        std::fs::write(images.join("bad.png"), b"not-an-image").expect("corrupt image");
        let before = std::fs::read(images.join("good.png")).expect("read good");
        let result = compress_images_in(&dir, &valid_compress_options(), None, None);
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(images.join("good.png")).expect("read good after"),
            before
        );
        assert!(!dir.join(".tmp-compress").exists());
        cleanup(&root);
    }
}
