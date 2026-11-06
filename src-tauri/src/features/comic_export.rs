//! 漫画导出：`content.json` 顺序 + 图片文件 → 自选目录下的 `<命名>/`（文件夹）或 `<命名>.zip`（压缩包）。
//!
//! EPUB / 封面照拷小说口径；原图按话建子目录（平铺无子目录），卷目录名与文件名均用
//! 构建页同一套标题逻辑渲染后清洗；渲染冲突（重名文件/目录）直接报错，不静默合并。

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;
use zip::write::SimpleFileOptions;

use super::comic_build::{comic_heading_page, page_number, page_title_width};
use super::comic_files::read_content;
use super::novel_build::count_width;
use super::novel_export::{
    BuildTitleOptions, ExportNaming, ExportOutcome, TxtTitleKind, cover_ext_of, target_dir,
};
use super::novel_package;
use super::project_metadata::cover_absolute_path_in;
use super::project_metadata::load_or_init_metadata_in;

/// 原图导出形态：文件夹 mirror 落盘，或打成单个压缩包。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ComicImagesMode {
    /// 文件夹：`dest/<命名>/` 下按话建子目录。
    Folder,
    /// 压缩包：`dest/<命名>.zip`，包内与文件夹模式同结构。
    Zip,
}

/// 漫画导出勾选：EPUB / 原图 / 封面。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicExportItems {
    /// EPUB 电子书。
    pub epub: bool,
    /// 原图（文件夹或压缩包，见 `ComicImagesMode`）。
    pub images: bool,
    /// 封面图片。
    pub cover: bool,
}

/// 导出进度事件名：前端据此订阅原图落盘进度；改名时两端同步。
pub const EXPORT_PROGRESS_EVENT: &str = "comic-export-progress";

/// 导出进度载荷：`id` 供前端多窗口过滤。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicExportProgress {
    /// 项目 id。
    pub id: String,
    /// 已写文件数。
    pub done_files: u32,
    /// 文件总数。
    pub total_files: u32,
}

/// 漫画导出状态：进页一次查全，前端据此渲染表单与禁用态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicExportStatus {
    /// 是否有可导出的解析产物。
    pub has_content: bool,
    /// 话数。
    pub volume_count: u32,
    /// 图片总页数。
    pub page_count: u32,
    /// 图片总字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub total_bytes: u32,
    /// 是否有可导出的打包产物。
    pub has_epub: bool,
    /// 打包文件名；无产物时为 `None`。
    pub epub_name: Option<String>,
    /// 是否有可导出的封面。
    pub has_cover: bool,
    /// 封面扩展名（小写）；无封面时为 `None`。
    pub cover_ext: Option<String>,
    /// 项目名。
    pub project_name: String,
    /// 书名。
    pub book_title: String,
}

/// 宽松转 `u32`：计数上溢时钳制，不报错。
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 导出状态（`&Path` 入口）：只读探针，不建业务文件。
pub fn status_in(project_dir: &Path) -> anyhow::Result<ComicExportStatus> {
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = load_or_init_metadata_in(project_dir)?;
    let content = read_content(project_dir).ok();
    let (volume_count, page_count, total_bytes) = content.as_ref().map_or((0, 0, 0), |content| {
        let pages: Vec<&str> = content
            .volumes
            .iter()
            .flat_map(|volume| volume.pages.iter().map(|page| page.path.as_str()))
            .collect();
        let bytes: u64 = pages
            .iter()
            .map(|rel| {
                std::fs::metadata(project_dir.join("sources").join("images").join(rel))
                    .map_or(0, |meta| meta.len())
            })
            .sum();
        (
            saturating_u32(content.volumes.len()),
            saturating_u32(pages.len()),
            u32::try_from(bytes).unwrap_or(u32::MAX),
        )
    });
    let epub_name = novel_package::latest_package(project_dir).and_then(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    });
    let cover_ext = cover_absolute_path_in(project_dir)?
        .and_then(|absolute| cover_ext_of(Path::new(&absolute)).ok());
    Ok(ComicExportStatus {
        has_content: content.is_some(),
        volume_count,
        page_count,
        total_bytes,
        has_epub: epub_name.is_some(),
        epub_name,
        has_cover: cover_ext.is_some(),
        cover_ext,
        project_name: info.name,
        book_title: metadata.title,
    })
}

/// 渲染后的卷目录名：标题渲染 + 文件系统清洗。
fn volume_dir_name(
    title: Option<&str>,
    volume_no: usize,
    volume_width: usize,
    titles: &BuildTitleOptions,
) -> String {
    let rendered = super::comic_build::comic_heading_volume(
        titles.volume_kind,
        &titles.volume_pattern,
        title,
        saturating_u32(volume_no),
        volume_width,
    );
    novel_package::sanitize_stem(&rendered)
}

/// 渲染后的页文件名（不含扩展名）：标题渲染 + 文件系统清洗。
fn page_stem(
    volume_title: Option<&str>,
    number: u32,
    page_width: usize,
    titles: &BuildTitleOptions,
) -> String {
    let rendered = comic_heading_page(
        titles.chapter_kind,
        &titles.chapter_pattern,
        volume_title,
        number,
        page_width,
    );
    novel_package::sanitize_stem(&rendered)
}

/// 导出全量生成：EPUB / 封面照拷，原图按话建目录拷贝或打成单个压缩包；
/// `progress` 在每个文件落盘后回调（分子/分母），调用方据此推送进度事件。
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn export_in(
    project_dir: &Path,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ComicExportItems,
    titles: &BuildTitleOptions,
    images_mode: ComicImagesMode,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<ExportOutcome> {
    if !items.epub && !items.images && !items.cover {
        anyhow::bail!("no export items selected");
    }
    if matches!(titles.chapter_kind, TxtTitleKind::Custom)
        && titles.chapter_pattern.trim().is_empty()
    {
        anyhow::bail!("custom page pattern must not be empty");
    }
    if matches!(titles.volume_kind, TxtTitleKind::Custom) && titles.volume_pattern.trim().is_empty()
    {
        anyhow::bail!("custom volume pattern must not be empty");
    }
    let dest = Path::new(dest_dir)
        .canonicalize()
        .with_context(|| format!("export directory not found: {dest_dir}"))?;
    if !dest.is_dir() {
        anyhow::bail!("export directory is not a directory: {dest_dir}");
    }
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = load_or_init_metadata_in(project_dir)?;
    let stem = match naming {
        ExportNaming::Project => novel_package::sanitize_stem(&info.name),
        ExportNaming::Book => novel_package::sanitize_stem(&metadata.title),
        ExportNaming::Custom => {
            if custom_name.trim().is_empty() {
                anyhow::bail!("custom name must not be empty");
            }
            novel_package::sanitize_stem(custom_name)
        }
    };

    // 落盘条目分两组，互不混入：
    // 根直出组（多选进书名目录 <stem>/，单选直出 dest 根）：EPUB 与独立封面；
    // 原画组（与前两者并列）：文件夹模式落 <stem>/<stem>/（多选）或 <stem>/（单选），
    // 压缩包模式打 <stem>/<stem>.zip（多选）或 <stem>.zip（单选），包内仅含
    // cover.<ext> + 卷子目录 / 平铺图片，不套书名层。
    let mut root_entries: Vec<(String, PathBuf)> = Vec::new();
    let mut image_entries: Vec<(String, PathBuf)> = Vec::new();
    if items.epub {
        let source =
            novel_package::latest_package(project_dir).context("no epub package to export")?;
        root_entries.push((format!("{stem}.epub"), source));
    }
    if items.cover {
        let absolute = cover_absolute_path_in(project_dir)?.context("no cover to export")?;
        let ext = cover_ext_of(Path::new(&absolute))?;
        root_entries.push((format!("{stem}.{ext}"), PathBuf::from(absolute)));
    }
    if items.images {
        let content = read_content(project_dir)?;
        let mut seen_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        // 原画必带封面（有则必含），与封面复选框相互独立。
        if let Some(absolute) = cover_absolute_path_in(project_dir)? {
            let ext = cover_ext_of(Path::new(&absolute))?;
            let rel = format!("cover.{ext}");
            if !seen_files.insert(rel.clone()) {
                anyhow::bail!("duplicate page file name: {rel}");
            }
            image_entries.push((rel, PathBuf::from(absolute)));
        }
        let volume_width = count_width(content.volumes.len());
        let page_width = page_title_width(&content, titles.chapter_numbering);
        // 平铺（单个无标题卷）无子目录，其余按话建子目录。
        let grouped = content.volumes.len() > 1
            || content
                .volumes
                .first()
                .is_some_and(|volume| volume.title.is_some());
        let mut seen_dirs = std::collections::HashSet::new();
        let mut global_seq = 0u32;
        for (volume_index, volume) in content.volumes.iter().enumerate() {
            let volume_no = volume_index + 1;
            let dir_name =
                volume_dir_name(volume.title.as_deref(), volume_no, volume_width, titles);
            if grouped && !seen_dirs.insert(dir_name.clone()) {
                anyhow::bail!("duplicate volume directory name: {dir_name}");
            }
            for (page_index, page) in volume.pages.iter().enumerate() {
                global_seq += 1;
                let local = page_index + 1;
                let number =
                    page_number(global_seq, saturating_u32(local), titles.chapter_numbering);
                let stem_name = page_stem(volume.title.as_deref(), number, page_width, titles);
                let source = project_dir
                    .join("sources")
                    .join("images")
                    .join(Path::new(&page.path));
                let ext = source
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(str::to_ascii_lowercase)
                    .with_context(|| format!("image has no extension: {}", source.display()))?;
                let file_name = format!("{stem_name}.{ext}");
                let rel = if grouped {
                    format!("{dir_name}/{file_name}")
                } else {
                    file_name.clone()
                };
                if !seen_files.insert(rel.clone()) {
                    anyhow::bail!("duplicate page file name: {rel}");
                }
                image_entries.push((rel, source));
            }
        }
        if image_entries.is_empty() {
            anyhow::bail!("no images to export");
        }
    }
    let selected_count = [items.epub, items.images, items.cover]
        .iter()
        .filter(|&&checked| checked)
        .count();
    // 多选（≥2 项）时统一加一层书名目录；单选直出 dest 根
    //（原画文件夹本身即该层，无需重复建层）。
    let layered = selected_count >= 2;
    let base = if layered || (items.images && matches!(images_mode, ComicImagesMode::Folder)) {
        Some(target_dir(&dest, &stem)?)
    } else {
        None
    };
    let out_dir = base.as_ref().unwrap_or(&dest);
    let dir = out_dir
        .to_str()
        .context("export dir is not valid UTF-8")?
        .to_string();
    let prefix = base
        .as_ref()
        .map(|_| format!("{stem}/"))
        .unwrap_or_default();
    // 原画组前缀：与 EPUB / 封面并列，多选时自成 dest/<stem>/<stem>/ 一层；
    // 单选原画文件夹时即为 dest/<stem>/（压缩包单选不建目录，前缀仅用于清单）。
    let images_prefix = if layered {
        format!("{stem}/{stem}/")
    } else {
        format!("{stem}/")
    };
    let mut files: Vec<String> = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    // 进度分母：根直出文件 + 原图条目（压缩包按条目数计，落盘为一次性写包）。
    let total = saturating_u32(root_entries.len() + image_entries.len());
    let mut done = 0u32;
    let mut bump = |files: &mut Vec<String>, rel: String| {
        done += 1;
        if let Some(emit) = progress {
            emit(done, total);
        }
        files.push(rel);
    };
    // 根直出组：EPUB / 独立封面（多选时进书名目录，单选直出 dest 根；覆盖旧文件）。
    for (name, source) in &root_entries {
        let rel = format!("{prefix}{name}");
        if !seen_paths.insert(rel.clone()) {
            anyhow::bail!("duplicate export file name: {rel}");
        }
        let dest_path = dest.join(&rel);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::copy(source, &dest_path)
            .with_context(|| format!("failed to copy {}", dest_path.display()))?;
        bump(&mut files, rel);
    }
    match images_mode {
        ComicImagesMode::Folder => {
            if !image_entries.is_empty() {
                for (rel, source) in &image_entries {
                    let final_rel = format!("{images_prefix}{rel}");
                    if !seen_paths.insert(final_rel.clone()) {
                        anyhow::bail!("duplicate export file name: {final_rel}");
                    }
                    let dest_path = dest.join(&final_rel);
                    if let Some(parent) = dest_path.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("failed to create {}", parent.display()))?;
                    }
                    std::fs::copy(source, &dest_path)
                        .with_context(|| format!("failed to copy {}", dest_path.display()))?;
                    bump(&mut files, final_rel);
                }
            }
            files.sort();
            Ok(ExportOutcome { dir, files })
        }
        ComicImagesMode::Zip => {
            if image_entries.is_empty() {
                files.sort();
                return Ok(ExportOutcome { dir, files });
            }
            let zip_rel = if layered {
                format!("{stem}/{stem}.zip")
            } else {
                format!("{stem}.zip")
            };
            if !seen_paths.insert(zip_rel.clone()) {
                anyhow::bail!("duplicate export file name: {zip_rel}");
            }
            let zip_path = dest.join(&zip_rel);
            if zip_path.exists() {
                anyhow::bail!("export target already exists: {}", zip_path.display());
            }
            if let Some(parent) = zip_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            let file = std::fs::File::create(&zip_path)
                .with_context(|| format!("failed to create {}", zip_path.display()))?;
            let mut zip = zip::ZipWriter::new(file);
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            for (rel, source) in &image_entries {
                let bytes = std::fs::read(source)
                    .with_context(|| format!("failed to read {}", source.display()))?;
                zip.start_file(rel, options)
                    .with_context(|| format!("failed to add {rel}"))?;
                zip.write_all(&bytes)
                    .with_context(|| format!("failed to write {rel}"))?;
                done += 1;
                if let Some(emit) = progress {
                    emit(done, total);
                }
            }
            zip.finish().context("failed to finish zip")?;
            files.push(zip_rel);
            files.sort();
            Ok(ExportOutcome { dir, files })
        }
    }
}

/// 导出状态（`AppHandle` 入口）：只读探针，不建业务文件。
pub fn export_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ComicExportStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 导出漫画（`AppHandle` 入口）：阻塞池执行并推送原图落盘进度，前端据此渲染进度条。
#[allow(clippy::too_many_arguments)]
pub async fn export_comic(
    app: &tauri::AppHandle,
    id: &str,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ComicExportItems,
    titles: &BuildTitleOptions,
    images_mode: ComicImagesMode,
) -> anyhow::Result<ExportOutcome> {
    use tauri::Emitter;

    let dir = super::projects::resolve_project_dir(app, id)?;
    let dest = dest_dir.to_string();
    let name = custom_name.to_string();
    let items = items.clone();
    let titles = titles.clone();
    let handle = app.clone();
    let report_id = id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let on_progress = move |done: u32, total: u32| {
            let _ = handle.emit(
                EXPORT_PROGRESS_EVENT,
                ComicExportProgress {
                    id: report_id.clone(),
                    done_files: done,
                    total_files: total,
                },
            );
        };
        export_in(
            &dir,
            &dest,
            naming,
            &name,
            &items,
            &titles,
            images_mode,
            Some(&on_progress),
        )
    })
    .await
    .context("export task panicked")?
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::comic_build;
    use super::super::comic_files;
    use super::super::novel_export::BuildTitleOptions;
    use super::super::novel_export::ChapterNumbering;
    use super::super::novel_export::ExportNaming;
    use super::super::novel_export::TxtTitleKind;
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

    /// 用例隔离的项目：建真实漫画项目目录，跑完清理。
    fn test_project(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-comic-export-test-{}-{case}",
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

    /// 1x1 真 PNG 字节：epub 校验嗅探内容，假字节过不了。
    const PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x48,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    /// 建一个已解析的漫画项目：写源图并导入解析，返回项目目录。
    fn parsed_project(case: &str, files: &[&str]) -> (PathBuf, PathBuf) {
        let (root, dir) = test_project(case);
        let source = root.join(format!("{case}-src"));
        let _ = std::fs::remove_dir_all(&source);
        for file in files {
            let path = source.join(file);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create source parent");
            }
            std::fs::write(&path, PIXEL_PNG).expect("write source image");
        }
        comic_files::import_dir_in(&dir, &source).expect("import");
        comic_files::parse_images_in(&dir).expect("parse");
        (root, dir)
    }

    fn images_only() -> ComicExportItems {
        ComicExportItems {
            epub: false,
            images: true,
            cover: false,
        }
    }

    fn default_titles() -> BuildTitleOptions {
        BuildTitleOptions {
            chapter_kind: TxtTitleKind::Arabic,
            chapter_pattern: String::new(),
            volume_kind: TxtTitleKind::Original,
            volume_pattern: String::new(),
            chapter_numbering: ChapterNumbering::PerVolume,
        }
    }

    #[test]
    fn exports_images_to_folder_with_volume_dirs() {
        let (root, dir) = parsed_project("folder", &["第1话/01.png", "第1话/02.png", "02.png"]);
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Project,
            "",
            &images_only(),
            &default_titles(),
            ComicImagesMode::Folder,
            None,
        )
        .expect("export");
        // 混合源：顶层散图收进收纳话，页名按话内编号；清单记相对 dest 的路径。
        let target = dest.join("雾镇");
        assert!(target.join("第1话").join("第1页.png").is_file());
        assert!(target.join("第1话").join("第2页.png").is_file());
        assert!(target.join("额外").join("第1页.png").is_file());
        assert_eq!(outcome.dir, target.to_str().expect("utf8 dest"));
        assert!(outcome.files.contains(&"雾镇/第1话/第1页.png".to_string()));
        cleanup(&root);
    }

    #[test]
    fn images_output_always_bundles_cover() {
        let (root, dir) = parsed_project("cover-auto", &["01.png"]);
        let source = root.join("cover-src.png");
        std::fs::write(&source, PIXEL_PNG).expect("write fake cover");
        super::super::project_metadata::import_cover_in(&dir, &source).expect("import cover");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Project,
            "",
            &images_only(),
            &default_titles(),
            ComicImagesMode::Folder,
            None,
        )
        .expect("export");
        // 未勾选封面，原画目录仍必带封面。
        assert!(dest.join("雾镇").join("cover.png").is_file());
        assert!(dest.join("雾镇").join("第1页.png").is_file());
        cleanup(&root);
    }

    #[test]
    fn separates_epub_and_standalone_cover_at_dest_root() {
        let (root, dir) = parsed_project("mixed", &["01.png"]);
        let source = root.join("cover-src.png");
        std::fs::write(&source, PIXEL_PNG).expect("write fake cover");
        super::super::project_metadata::import_cover_in(&dir, &source).expect("import cover");
        let template_base =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC");
        super::super::comic_build::build_in(&dir, &template_base, &BuildTitleOptions::default())
            .expect("build");
        super::super::novel_package::write_epub_in(&dir, None).expect("package");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Project,
            "",
            &ComicExportItems {
                epub: true,
                images: true,
                cover: true,
            },
            &default_titles(),
            ComicImagesMode::Folder,
            None,
        )
        .expect("export");
        // EPUB 与独立封面进书名目录，原画自成并列的书名/书名子目录。
        assert!(dest.join("雾镇").join("雾镇.epub").is_file());
        assert!(!dest.join("雾镇.epub").exists());
        assert!(dest.join("雾镇").join("雾镇.png").is_file());
        assert!(dest.join("雾镇").join("雾镇").join("cover.png").is_file());
        assert!(dest.join("雾镇").join("雾镇").join("第1页.png").is_file());
        assert!(!dest.join("雾镇").join("雾镇").join("雾镇.epub").exists());
        assert!(outcome.files.contains(&"雾镇/雾镇.epub".to_string()));
        assert!(outcome.files.contains(&"雾镇/雾镇.png".to_string()));
        assert!(outcome.files.contains(&"雾镇/雾镇/cover.png".to_string()));
        cleanup(&root);
    }

    #[test]
    fn single_item_exports_without_layer() {
        let (root, dir) = parsed_project("single", &["01.png"]);
        let template_base =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC");
        super::super::comic_build::build_in(&dir, &template_base, &BuildTitleOptions::default())
            .expect("build");
        super::super::novel_package::write_epub_in(&dir, None).expect("package");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        // 单勾 EPUB：根直出，不建书名目录。
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Project,
            "",
            &ComicExportItems {
                epub: true,
                images: false,
                cover: false,
            },
            &default_titles(),
            ComicImagesMode::Folder,
            None,
        )
        .expect("export epub");
        assert!(dest.join("雾镇.epub").is_file());
        assert!(!dest.join("雾镇").exists());
        assert_eq!(outcome.files, vec!["雾镇.epub".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn layers_book_dir_for_multi_selection_without_images() {
        let (root, dir) = parsed_project("layered", &["01.png"]);
        let source = root.join("cover-src.png");
        std::fs::write(&source, PIXEL_PNG).expect("write fake cover");
        super::super::project_metadata::import_cover_in(&dir, &source).expect("import cover");
        let template_base =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC");
        super::super::comic_build::build_in(&dir, &template_base, &BuildTitleOptions::default())
            .expect("build");
        super::super::novel_package::write_epub_in(&dir, None).expect("package");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        // 仅 EPUB + 封面（无原画）：仍加一层书名目录。
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Project,
            "",
            &ComicExportItems {
                epub: true,
                images: false,
                cover: true,
            },
            &default_titles(),
            ComicImagesMode::Folder,
            None,
        )
        .expect("export");
        assert!(dest.join("雾镇").join("雾镇.epub").is_file());
        assert!(dest.join("雾镇").join("雾镇.png").is_file());
        assert_eq!(outcome.dir, dest.join("雾镇").to_str().expect("utf8"));
        // 压缩包 + EPUB：包与 EPUB 并列进书名目录。
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Custom,
            "精选集",
            &ComicExportItems {
                epub: true,
                images: true,
                cover: false,
            },
            &default_titles(),
            ComicImagesMode::Zip,
            None,
        )
        .expect("export zip");
        assert!(dest.join("精选集").join("精选集.zip").is_file());
        assert!(dest.join("精选集").join("精选集.epub").is_file());
        assert!(outcome.files.contains(&"精选集/精选集.zip".to_string()));
        cleanup(&root);
    }

    #[test]
    fn exports_images_to_zip_with_same_structure() {
        let (root, dir) = parsed_project("zip", &["第1话/01.png", "第1话/02.png"]);
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Book,
            "",
            &images_only(),
            &default_titles(),
            ComicImagesMode::Zip,
            None,
        )
        .expect("export");
        assert_eq!(outcome.files, vec!["雾镇.zip".to_string()]);
        // 包内与文件夹模式同结构。
        let bytes = std::fs::read(dest.join("雾镇.zip")).expect("read zip");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("open zip");
        let mut names: Vec<String> = (0..archive.len())
            .map(|index| {
                archive
                    .by_index(index)
                    .expect("zip entry")
                    .name()
                    .expect("zip name")
                    .to_string()
            })
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec!["第1话/第1页.png".to_string(), "第1话/第2页.png".to_string(),]
        );
        cleanup(&root);
    }

    #[test]
    fn applies_custom_titles_and_continuous_numbering() {
        let (root, dir) = parsed_project("custom", &["甲/01.png", "乙/01.png", "乙/02.png"]);
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let titles = BuildTitleOptions {
            chapter_kind: TxtTitleKind::Custom,
            chapter_pattern: "P#N".to_string(),
            volume_kind: TxtTitleKind::Arabic,
            volume_pattern: String::new(),
            chapter_numbering: ChapterNumbering::Continuous,
        };
        let outcome = export_in(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Custom,
            "精选",
            &images_only(),
            &titles,
            ComicImagesMode::Folder,
            None,
        )
        .expect("export");
        // 自然排序下乙在前：乙为第1话（P01/P02），甲为第2话（P03）。
        let target = dest.join("精选");
        assert!(target.join("第1话 乙").join("P01.png").is_file());
        assert!(target.join("第2话 甲").join("P03.png").is_file());
        assert!(outcome.files.contains(&"精选/第2话 甲/P03.png".to_string()));
        cleanup(&root);
    }

    #[test]
    fn rejects_empty_items_and_name_collisions() {
        let (root, dir) = parsed_project("bad", &["01.png", "02.png"]);
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let dest_str = dest.to_str().expect("utf8 dest");
        // 空勾选拒绝。
        assert!(
            export_in(
                &dir,
                dest_str,
                ExportNaming::Project,
                "",
                &ComicExportItems {
                    epub: false,
                    images: false,
                    cover: false,
                },
                &default_titles(),
                ComicImagesMode::Folder,
                None,
            )
            .is_err()
        );
        // 自定义模板忘加编号 → 文件重名，拒绝。
        let titles = BuildTitleOptions {
            chapter_pattern: "P".to_string(),
            chapter_kind: TxtTitleKind::Custom,
            ..default_titles()
        };
        assert!(
            export_in(
                &dir,
                dest_str,
                ExportNaming::Project,
                "",
                &images_only(),
                &titles,
                ComicImagesMode::Folder,
                None,
            )
            .is_err()
        );
        // 目标 zip 已存在拒绝覆盖。
        std::fs::write(dest.join("雾镇.zip"), b"old").expect("write dummy zip");
        assert!(
            export_in(
                &dir,
                dest_str,
                ExportNaming::Project,
                "",
                &images_only(),
                &default_titles(),
                ComicImagesMode::Zip,
                None,
            )
            .is_err()
        );
        cleanup(&root);
    }

    #[test]
    fn reports_export_status() {
        let (root, dir) = parsed_project("status", &["01.png"]);
        let status = status_in(&dir).expect("status");
        assert!(status.has_content);
        assert_eq!(status.page_count, 1);
        assert_eq!(status.volume_count, 1);
        assert!(status.total_bytes > 0);
        assert!(!status.has_epub);
        assert_eq!(status.project_name, "雾镇");
        // 构建打包后状态跟上。
        let template_base =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC");
        comic_build::build_in(&dir, &template_base, &BuildTitleOptions::default()).expect("build");
        super::super::novel_package::write_epub_in(&dir, None).expect("package");
        let status = status_in(&dir).expect("status again");
        assert!(status.has_epub);
        assert!(status.epub_name.is_some_and(|name| {
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("epub"))
        }));
        cleanup(&root);
    }
}
