//! 漫画 PDF 页面尺寸、布局与逐页写入。

use serde::{Deserialize, Serialize};
use specta::Type;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::ImageDecoder;
use image::metadata::Orientation;
use pdf_writer::{Chunk, Content, Filter, Name, Rect, Ref, TextStr};

use super::comic_files::ComicContent;
use super::novel_export::{MetadataField, TxtTitleKind};
use super::project_metadata::ProjectMetadata;

const MIN_DPI: u32 = 72;
const MAX_DPI: u32 = 600;
const MAX_PAGE_DIMENSION_PT: f64 = 14_400.0;
const MAX_IMAGE_PIXELS: u64 = 50_000_000;
const CATALOG_REF: Ref = Ref::new(1);
const PAGES_REF: Ref = Ref::new(2);

/// PDF 页面尺寸来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ComicPdfPageSize {
    /// 按漫画页的平均像素宽高生成统一画布。
    Average,
    /// 使用用户指定的像素宽高生成统一画布。
    Fixed,
    /// 每页采用原图宽高，一图一页；填充与对齐此时无意义。
    Original,
}

/// 图片在 PDF 页面内的填充方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ComicPdfFillMode {
    /// 保持比例并完整显示图片。
    Contain,
    /// 保持比例并裁切超出页面的部分。
    Cover,
    /// 分别缩放宽高以填满页面。
    Stretch,
}

/// 图片的水平对齐位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ComicPdfHorizontalAlignment {
    /// 左对齐。
    Left,
    /// 水平居中。
    Center,
    /// 右对齐。
    Right,
}

/// 图片的垂直对齐位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ComicPdfVerticalAlignment {
    /// 顶部对齐。
    Top,
    /// 垂直居中。
    Center,
    /// 底部对齐。
    Bottom,
}

/// 漫画 PDF 导出参数；固定尺寸字段仅在固定模式下使用，原图尺寸模式下填充与对齐无意义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ComicPdfOptions {
    /// 页面尺寸来源。
    pub page_size: ComicPdfPageSize,
    /// 固定页面宽度（像素）。
    pub width_px: u32,
    /// 固定页面高度（像素）。
    pub height_px: u32,
    /// 像素到 PDF 物理页面的换算分辨率。
    pub dpi: u32,
    /// 图片填充方式。
    pub fill_mode: ComicPdfFillMode,
    /// 图片水平对齐。
    pub horizontal_alignment: ComicPdfHorizontalAlignment,
    /// 图片垂直对齐。
    pub vertical_alignment: ComicPdfVerticalAlignment,
    /// 是否将项目封面作为第一页。
    pub include_cover: bool,
    /// 话标题格式：多话 PDF 书签的卷标题渲染口径，与原图话目录同源。
    pub volume_kind: TxtTitleKind,
    /// 话标题自定义模板：`volume_kind` 为自定义时生效。
    pub volume_pattern: String,
    /// 写入文档元数据：为空即关闭；非空时按所选字段写入 Info（为空跳过）。
    pub metadata: Vec<MetadataField>,
}

impl Default for ComicPdfOptions {
    fn default() -> Self {
        Self {
            page_size: ComicPdfPageSize::Average,
            width_px: 0,
            height_px: 0,
            dpi: 300,
            fill_mode: ComicPdfFillMode::Contain,
            horizontal_alignment: ComicPdfHorizontalAlignment::Center,
            vertical_alignment: ComicPdfVerticalAlignment::Center,
            include_cover: false,
            volume_kind: TxtTitleKind::Arabic,
            volume_pattern: String::new(),
            metadata: Vec::new(),
        }
    }
}

/// 单张 PDF 图片及其解码头信息。
#[derive(Debug, Clone)]
pub(super) struct PdfPageImage {
    /// 经项目根目录校验的源图路径。
    pub path: PathBuf,
    /// 源图宽度（像素）。
    pub width_px: u32,
    /// 源图高度（像素）。
    pub height_px: u32,
    /// 源图格式，用于导出期间检测被替换的文件。
    pub format: image::ImageFormat,
    /// 源图方向，用于导出期间检测元信息变化。
    pub orientation: Orientation,
}

/// PDF 图像对象使用的已编码样本及颜色解释。
struct EncodedImageSamples {
    bytes: Vec<u8>,
    color_space: &'static [u8],
    bits_per_component: i32,
    filter: Filter,
}

/// 读取图片尺寸，避免在平均尺寸模式下把整本漫画解码进内存。
pub(super) fn inspect_page_image(path: &Path) -> anyhow::Result<PdfPageImage> {
    let reader = image::ImageReader::open(path)
        .with_context(|| format!("failed to open PDF image {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("failed to inspect PDF image {}", path.display()))?;
    let format = reader
        .format()
        .context("PDF image format could not be determined")?;
    let mut decoder = reader
        .into_decoder()
        .with_context(|| format!("failed to initialize PDF image decoder {}", path.display()))?;
    let orientation = decoder
        .orientation()
        .with_context(|| format!("failed to read PDF image orientation {}", path.display()))?;
    let (width_px, height_px) = oriented_dimensions(decoder.dimensions(), orientation);
    validate_image_dimensions(width_px, height_px)?;
    Ok(PdfPageImage {
        path: path.to_path_buf(),
        width_px,
        height_px,
        format,
        orientation,
    })
}

/// 把 EXIF 旋转方向转换为页面实际显示的宽高。
const fn oriented_dimensions(dimensions: (u32, u32), orientation: Orientation) -> (u32, u32) {
    let (width, height) = dimensions;
    match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (height, width),
        _ => (width, height),
    }
}

/// 单页图片在 PDF 画布中的物理布局，单位为 PDF 点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PageLayout {
    /// 页面宽度。
    pub page_width_pt: f64,
    /// 页面高度。
    pub page_height_pt: f64,
    /// 图片绘制宽度。
    pub image_width_pt: f64,
    /// 图片绘制高度。
    pub image_height_pt: f64,
    /// 图片左下角相对页面左下角的水平偏移。
    pub offset_x_pt: f64,
    /// 图片左下角相对页面左下角的垂直偏移。
    pub offset_y_pt: f64,
    /// 是否需要裁剪页面边界外的图片内容。
    pub clips: bool,
}

/// 校验后端收到的 PDF 参数，避免前端绕过输入约束。
pub(super) fn validate_pdf_options(options: &ComicPdfOptions) -> anyhow::Result<()> {
    anyhow::ensure!(
        (MIN_DPI..=MAX_DPI).contains(&options.dpi),
        "PDF DPI must be between {MIN_DPI} and {MAX_DPI}"
    );
    if options.page_size == ComicPdfPageSize::Fixed {
        anyhow::ensure!(
            options.width_px > 0 && options.height_px > 0,
            "fixed PDF page dimensions must be positive"
        );
        let point_scale = 72.0 / f64::from(options.dpi);
        anyhow::ensure!(
            f64::from(options.width_px) * point_scale <= MAX_PAGE_DIMENSION_PT
                && f64::from(options.height_px) * point_scale <= MAX_PAGE_DIMENSION_PT,
            "PDF page dimensions exceed the supported limit"
        );
    }
    Ok(())
}

/// 校验解析后的像素画布换算为物理点后未超限；平均模式也需前置拦截，避免写中途才失败。
fn validate_resolved_page_size(page_size_px: (f64, f64), dpi: u32) -> anyhow::Result<()> {
    let (width_px, height_px) = page_size_px;
    anyhow::ensure!(
        width_px.is_finite() && height_px.is_finite() && width_px > 0.0 && height_px > 0.0,
        "PDF page dimensions must be finite and positive"
    );
    let point_scale = 72.0 / f64::from(dpi);
    anyhow::ensure!(
        width_px * point_scale <= MAX_PAGE_DIMENSION_PT
            && height_px * point_scale <= MAX_PAGE_DIMENSION_PT,
        "PDF page dimensions exceed the supported limit"
    );
    Ok(())
}

/// 生成多页 PDF；仅保留当前页的解码数据，进度按实际 PDF 页数推进。
/// 书签为空时不写 Outline 对象，文档元数据为空时不写 Info 对象，
/// 单话无元数据输出与此前字节形态一致。
pub(super) fn write_pdf_file(
    destination: &Path,
    comic_pages: &[PdfPageImage],
    cover: Option<&PdfPageImage>,
    options: &ComicPdfOptions,
    outline: &[PdfOutlineEntry],
    doc_info: &PdfDocInfo,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    validate_pdf_options(options)?;
    anyhow::ensure!(!comic_pages.is_empty(), "no comic pages to export as PDF");

    // 统一画布模式先解析并前置校验；原图尺寸模式逐页采用自身宽高，在页序确定后逐页校验。
    let shared_page_size_px = if options.page_size == ComicPdfPageSize::Original {
        None
    } else {
        let size = resolve_page_size(comic_pages, options)?;
        validate_resolved_page_size(size, options.dpi)?;
        Some(size)
    };
    let pages = output_pages(comic_pages, cover, options.include_cover)?;
    if options.page_size == ComicPdfPageSize::Original {
        for page in &pages {
            validate_resolved_page_size(
                (f64::from(page.width_px), f64::from(page.height_px)),
                options.dpi,
            )?;
        }
    }
    let temp_path = temporary_sibling_path(destination)?;

    if let Err(error) = write_pdf_temp(
        &temp_path,
        &pages,
        shared_page_size_px,
        options,
        outline,
        doc_info,
        progress,
    ) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    if let Err(error) = replace_pdf_file(&temp_path, destination) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

/// 解析最终 PDF 页序，封面存在且启用时位于漫画页之前。
fn output_pages(
    comic_pages: &[PdfPageImage],
    cover: Option<&PdfPageImage>,
    include_cover: bool,
) -> anyhow::Result<Vec<PdfPageImage>> {
    let mut pages = Vec::with_capacity(comic_pages.len() + usize::from(include_cover));
    if include_cover {
        pages.push(
            cover
                .context("project cover is missing for PDF export")?
                .clone(),
        );
    }
    pages.extend_from_slice(comic_pages);
    Ok(pages)
}

/// PDF 文档元数据：Info 条目与目录语言；两者皆空时不写任何元数据对象。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PdfDocInfo {
    /// Info 字典条目：键不含斜杠，值非空。
    pub entries: Vec<(&'static [u8], String)>,
    /// 目录语言（BCP 47，如 `zh-CN`）。
    pub lang: Option<String>,
}

/// 应用名：文档 Creator/Producer 落款，与打包产物外壳一致。
const APP_NAME: &str = "Ebook Studio";

/// 解析文档元数据映射：标题/作者/主题词/简介进 Info，语言进目录；
/// 空值跳过，其余字段无对应项直接忽略。
pub(super) fn collect_doc_info(metadata: &ProjectMetadata, fields: &[MetadataField]) -> PdfDocInfo {
    fn push_non_empty(entries: &mut Vec<(&'static [u8], String)>, key: &'static [u8], value: &str) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            entries.push((key, trimmed.to_string()));
        }
    }

    let mut info = PdfDocInfo::default();
    for field in fields {
        match field {
            MetadataField::Title => push_non_empty(&mut info.entries, b"Title", &metadata.title),
            MetadataField::Creator => {
                push_non_empty(&mut info.entries, b"Author", &metadata.creator);
            }
            MetadataField::Subjects => {
                let joined = metadata
                    .subjects
                    .iter()
                    .map(|subject| subject.trim())
                    .filter(|subject| !subject.is_empty())
                    .collect::<Vec<_>>()
                    .join("、");
                push_non_empty(&mut info.entries, b"Keywords", &joined);
            }
            MetadataField::Description => {
                let joined = metadata
                    .description
                    .iter()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                push_non_empty(&mut info.entries, b"Subject", &joined);
            }
            MetadataField::Language => {
                let language = metadata.language.trim();
                if !language.is_empty() && info.lang.is_none() {
                    info.lang = Some(language.to_string());
                }
            }
            _ => {}
        }
    }
    info
}

/// PDF 时间戳：`D:年月日时分秒+00'00'`，业务 UTC 口径。
fn pdf_timestamp(now: chrono::DateTime<chrono::Utc>) -> String {
    now.format("D:%Y%m%d%H%M%S+00'00'").to_string()
}

/// 写入文档信息字典；中文自动 UTF-16 编码，软件落款恒写。返回对象编号供 trailer 引用。
fn write_info_object<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    reference: Ref,
    info: &PdfDocInfo,
    timestamp: &str,
) -> anyhow::Result<Ref> {
    let mut chunk = Chunk::new();
    {
        let mut dict = chunk.indirect(reference).dict();
        for (key, value) in &info.entries {
            dict.pair(Name(key), TextStr(value));
        }
        dict.pair(Name(b"Creator"), TextStr(APP_NAME))
            .pair(Name(b"Producer"), TextStr(APP_NAME))
            .pair(Name(b"CreationDate"), TextStr(timestamp))
            .pair(Name(b"ModDate"), TextStr(timestamp));
    }
    sink.write_object(&chunk)?;
    Ok(reference)
}

/// 是否有可写文档元数据：条目与语言皆空时不写任何元数据对象。
const fn has_doc_meta(info: &PdfDocInfo) -> bool {
    !info.entries.is_empty() || info.lang.is_some()
}

/// 文档信息字典编号：紧随书签对象之后，无书签时紧随页面对象。
fn info_object_ref(pages_len: usize, outline_len: usize) -> anyhow::Result<Ref> {
    let first = pages_len
        .checked_mul(3)
        .and_then(|value| value.checked_add(3))
        .context("PDF info object id overflow")?;
    let id = if outline_len == 0 {
        first
    } else {
        first
            .checked_add(1 + outline_len)
            .context("PDF info object id overflow")?
    };
    let id = i32::try_from(id).context("PDF info object id exceeds format limit")?;
    Ok(Ref::new(id))
}

/// PDF 书签条目：话标题与目标输出页下标（含封面页偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PdfOutlineEntry {
    /// 渲染后的卷标题，与原图话目录名同源（不做文件系统清洗）。
    pub title: String,
    /// 目标页在最终输出页序中的下标。
    pub page_index: usize,
}

/// 按话构建 PDF 书签；单话返回空（无处可跳），封面页下标由调用方计入偏移。
pub(super) fn collect_pdf_outline(
    content: &ComicContent,
    options: &ComicPdfOptions,
    first_page_offset: usize,
) -> Vec<PdfOutlineEntry> {
    if content.volumes.len() < 2 {
        return Vec::new();
    }
    let volume_width = super::novel_build::count_width(content.volumes.len());
    let mut page_index = first_page_offset;
    content
        .volumes
        .iter()
        .enumerate()
        .map(|(volume_index, volume)| {
            let title = super::comic_export::render_volume_title(
                volume.title.as_deref(),
                volume_index + 1,
                volume_width,
                options.volume_kind,
                &options.volume_pattern,
            );
            let entry = PdfOutlineEntry { title, page_index };
            page_index += volume.pages.len();
            entry
        })
        .collect()
}

/// 解析固定或平均像素画布；平均值仅统计 content.json 的漫画页，原图尺寸模式不经过此函数。
fn resolve_page_size(
    comic_pages: &[PdfPageImage],
    options: &ComicPdfOptions,
) -> anyhow::Result<(f64, f64)> {
    if options.page_size == ComicPdfPageSize::Fixed {
        return Ok((f64::from(options.width_px), f64::from(options.height_px)));
    }
    let dimensions: Vec<(u32, u32)> = comic_pages
        .iter()
        .map(|page| (page.width_px, page.height_px))
        .collect();
    average_page_dimensions(&dimensions)
}

/// 解析单页画布；原图尺寸模式下每页采用自身宽高，其余模式共用统一画布。
fn page_canvas_size(
    page: &PdfPageImage,
    shared_page_size_px: Option<(f64, f64)>,
    options: &ComicPdfOptions,
) -> anyhow::Result<(f64, f64)> {
    if options.page_size == ComicPdfPageSize::Original {
        Ok((f64::from(page.width_px), f64::from(page.height_px)))
    } else {
        shared_page_size_px.context("shared PDF page size is missing")
    }
}

/// 在同目录创建唯一临时文件，成功后再替换最终目标。
fn temporary_sibling_path(destination: &Path) -> anyhow::Result<PathBuf> {
    let parent = destination
        .parent()
        .context("PDF destination has no parent directory")?;
    let file_name = destination
        .file_name()
        .context("PDF destination has no file name")?;
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".{}.tmp", uuid::Uuid::new_v4()));
    Ok(parent.join(temp_name))
}

/// 在同一目录用标准库 `rename` 原子替换已存在的 PDF 文件。
fn write_pdf_temp(
    temp_path: &Path,
    pages: &[PdfPageImage],
    shared_page_size_px: Option<(f64, f64)>,
    options: &ComicPdfOptions,
    outline: &[PdfOutlineEntry],
    doc_info: &PdfDocInfo,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
        .with_context(|| format!("failed to create temporary PDF {}", temp_path.display()))?;
    let mut writer = BufWriter::new(file);
    write_pdf_to(
        &mut writer,
        pages,
        shared_page_size_px,
        options,
        outline,
        doc_info,
        progress,
    )?;
    writer.flush().context("failed to flush temporary PDF")?;
    writer
        .get_ref()
        .sync_all()
        .context("failed to sync temporary PDF")?;
    Ok(())
}

fn replace_pdf_file(temp_path: &Path, destination: &Path) -> anyhow::Result<()> {
    fs::rename(temp_path, destination).with_context(|| {
        format!(
            "failed to replace PDF destination {}",
            destination.display()
        )
    })
}

/// 将各页图片按顺序流式写入 PDF 对象及交叉引用表。
fn write_pdf_to<W: Write>(
    writer: &mut W,
    pages: &[PdfPageImage],
    shared_page_size_px: Option<(f64, f64)>,
    options: &ComicPdfOptions,
    outline: &[PdfOutlineEntry],
    doc_info: &PdfDocInfo,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    validate_pdf_options(options)?;
    anyhow::ensure!(!pages.is_empty(), "no PDF pages to write");
    for entry in outline {
        anyhow::ensure!(
            entry.page_index < pages.len(),
            "PDF outline target is out of range"
        );
    }
    let page_count = i32::try_from(pages.len()).context("PDF page count exceeds format limit")?;
    let outline_objects = if outline.is_empty() {
        0
    } else {
        outline
            .len()
            .checked_add(1)
            .context("PDF outline object count overflow")?
    };
    let info_objects = usize::from(has_doc_meta(doc_info));
    let object_count = pages
        .len()
        .checked_mul(3)
        .and_then(|count| count.checked_add(2))
        .and_then(|count| count.checked_add(outline_objects))
        .and_then(|count| count.checked_add(info_objects))
        .context("PDF object count overflow")?;
    anyhow::ensure!(
        i32::try_from(object_count).is_ok(),
        "PDF object count exceeds format limit"
    );

    let header = b"%PDF-1.7\n%\x80\x80\x80\x80\n";
    let mut sink = PdfObjectSink::new(writer, object_count + 1);
    sink.write_bytes(header)?;
    let mut page_refs = Vec::with_capacity(pages.len());

    for (index, page) in pages.iter().enumerate() {
        let refs = page_object_refs(index)?;
        let canvas = page_canvas_size(page, shared_page_size_px, options)?;
        let layout = calculate_page_layout((page.width_px, page.height_px), canvas, options)?;
        let samples = encode_image_samples(page)?;
        write_image_object(&mut sink, refs.image, page, &samples)?;
        write_content_object(&mut sink, refs.content, &layout)?;
        write_page_object(&mut sink, refs.page, refs.content, refs.image, &layout)?;
        page_refs.push(refs.page);
        if let Some(emit) = progress {
            emit(
                u32::try_from(index + 1).context("PDF progress exceeds u32")?,
                u32::try_from(pages.len()).context("PDF page count exceeds u32")?,
            );
        }
    }

    let outline_root = if outline.is_empty() {
        None
    } else {
        Some(write_outline_tree(
            &mut sink,
            &page_refs,
            outline,
            outline_first_ref(pages.len())?,
        )?)
    };
    let info_ref = if has_doc_meta(doc_info) {
        let timestamp = pdf_timestamp(chrono::Utc::now());
        Some(write_info_object(
            &mut sink,
            info_object_ref(pages.len(), outline.len())?,
            doc_info,
            &timestamp,
        )?)
    } else {
        None
    };

    write_page_tree(
        &mut sink,
        &page_refs,
        page_count,
        outline_root,
        doc_info.lang.as_deref(),
    )?;
    sink.finish(info_ref)
}

#[derive(Debug, Clone, Copy)]
struct PdfPageObjectRefs {
    image: Ref,
    content: Ref,
    page: Ref,
}

/// 为每一页分配三个不重叠对象编号。
fn page_object_refs(index: usize) -> anyhow::Result<PdfPageObjectRefs> {
    let first = index
        .checked_mul(3)
        .and_then(|value| value.checked_add(3))
        .context("PDF page object id overflow")?;
    let first = i32::try_from(first).context("PDF page object id exceeds format limit")?;
    Ok(PdfPageObjectRefs {
        image: Ref::new(first),
        content: Ref::new(first + 1),
        page: Ref::new(first + 2),
    })
}

/// 书签根节点编号：紧随全部页面对象之后，条目依次后排。
fn outline_first_ref(page_len: usize) -> anyhow::Result<Ref> {
    let first = page_len
        .checked_mul(3)
        .and_then(|value| value.checked_add(3))
        .context("PDF outline object id overflow")?;
    let first = i32::try_from(first).context("PDF outline object id exceeds format limit")?;
    Ok(Ref::new(first))
}

/// 写入书签目录：根节点 + 每话一条全部展开的条目，标题非 ASCII 时自动 UTF-16 编码。
fn write_outline_tree<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    page_refs: &[Ref],
    outline: &[PdfOutlineEntry],
    outline_root: Ref,
) -> anyhow::Result<Ref> {
    let first_id = outline_root.get();
    let item_ref = |index: usize| {
        let step = i32::try_from(index).ok()?.checked_add(1)?;
        first_id.checked_add(step).map(Ref::new)
    };
    let count = i32::try_from(outline.len()).context("PDF outline count exceeds format limit")?;
    let first = item_ref(0).context("PDF outline item id overflow")?;
    let last = item_ref(outline.len() - 1).context("PDF outline item id overflow")?;

    for (index, entry) in outline.iter().enumerate() {
        let reference = item_ref(index).context("PDF outline item id overflow")?;
        let mut chunk = Chunk::new();
        {
            let mut item = chunk.outline_item(reference);
            item.title(TextStr(&entry.title)).parent(outline_root);
            if index > 0 {
                item.prev(item_ref(index - 1).context("PDF outline item id overflow")?);
            }
            if index + 1 < outline.len() {
                item.next(item_ref(index + 1).context("PDF outline item id overflow")?);
            }
            item.dest()
                .page(
                    *page_refs
                        .get(entry.page_index)
                        .context("PDF outline target is out of range")?,
                )
                .fit();
        }
        sink.write_object(&chunk)?;
    }

    let mut chunk = Chunk::new();
    chunk
        .outline(outline_root)
        .first(first)
        .last(last)
        .count(count);
    sink.write_object(&chunk)?;
    Ok(outline_root)
}

/// 写入一页图像对象；兼容 JPEG 直接嵌入，其余格式使用无损 RGB 样本。
fn write_image_object<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    reference: Ref,
    page: &PdfPageImage,
    samples: &EncodedImageSamples,
) -> anyhow::Result<()> {
    let mut chunk = Chunk::new();
    chunk
        .image_xobject(reference, &samples.bytes)
        .width(i32::try_from(page.width_px).context("image width exceeds PDF limit")?)
        .height(i32::try_from(page.height_px).context("image height exceeds PDF limit")?)
        .color_space_name(Name(samples.color_space))
        .bits_per_component(samples.bits_per_component)
        .filter(samples.filter);
    sink.write_object(&chunk)
}

/// 写入单页绘制命令，裁切模式通过页面矩形裁剪超出区域。
fn write_content_object<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    reference: Ref,
    layout: &PageLayout,
) -> anyhow::Result<()> {
    let mut content = Content::new();
    content.save_state();
    if layout.clips {
        content
            .rect(
                0.0,
                0.0,
                pdf_float(layout.page_width_pt),
                pdf_float(layout.page_height_pt),
            )
            .clip_nonzero()
            .end_path();
    }
    content.transform([
        pdf_float(layout.image_width_pt),
        0.0,
        0.0,
        pdf_float(layout.image_height_pt),
        pdf_float(layout.offset_x_pt),
        pdf_float(layout.offset_y_pt),
    ]);
    content.x_object(Name(b"Im0")).restore_state();

    let mut chunk = Chunk::new();
    chunk.stream(reference, content.as_bytes());
    sink.write_object(&chunk)
}

/// 写入页面字典与图片资源引用。
fn write_page_object<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    page_ref: Ref,
    content_ref: Ref,
    image_ref: Ref,
    layout: &PageLayout,
) -> anyhow::Result<()> {
    let mut chunk = Chunk::new();
    {
        let mut page = chunk.page(page_ref);
        page.parent(PAGES_REF)
            .media_box(Rect::new(
                0.0,
                0.0,
                pdf_float(layout.page_width_pt),
                pdf_float(layout.page_height_pt),
            ))
            .contents(content_ref);
        page.resources().x_objects().pair(Name(b"Im0"), image_ref);
    }
    sink.write_object(&chunk)
}

/// 写入页树、目录根节点与文档语言；有书签时目录挂载 Outline 并默认展开目录面板。
fn write_page_tree<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    page_refs: &[Ref],
    page_count: i32,
    outline_root: Option<Ref>,
    lang: Option<&str>,
) -> anyhow::Result<()> {
    let mut pages = Chunk::new();
    pages
        .pages(PAGES_REF)
        .kids(page_refs.iter().copied())
        .count(page_count);
    sink.write_object(&pages)?;

    let mut catalog = Chunk::new();
    {
        let mut dict = catalog.indirect(CATALOG_REF).dict();
        dict.pair(Name(b"Type"), Name(b"Catalog"))
            .pair(Name(b"Pages"), PAGES_REF);
        if let Some(root) = outline_root {
            dict.pair(Name(b"Outlines"), root)
                .pair(Name(b"PageMode"), Name(b"UseOutlines"));
        }
        if let Some(lang) = lang {
            dict.pair(Name(b"Lang"), TextStr(lang));
        }
    }
    sink.write_object(&catalog)
}

/// 当前 PDF 对象流及对象偏移；只记录小型 xref，不缓存整本 PDF。
struct PdfObjectSink<'a, W: Write> {
    writer: &'a mut W,
    offset: u64,
    offsets: Vec<Option<u64>>,
}

impl<'a, W: Write> PdfObjectSink<'a, W> {
    fn new(writer: &'a mut W, object_slots: usize) -> Self {
        Self {
            writer,
            offset: 0,
            offsets: vec![None; object_slots],
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.writer
            .write_all(bytes)
            .context("failed to write PDF bytes")?;
        self.offset = self
            .offset
            .checked_add(u64::try_from(bytes.len()).context("PDF byte length overflow")?)
            .context("PDF file offset overflow")?;
        Ok(())
    }

    fn write_object(&mut self, chunk: &Chunk) -> anyhow::Result<()> {
        let mut references = chunk.refs();
        let reference = references.next().context("PDF object chunk is empty")?;
        anyhow::ensure!(
            references.next().is_none(),
            "PDF chunk must contain one object"
        );
        let index = usize::try_from(reference.get()).context("invalid PDF object id")?;
        let offset = self.offset;
        self.write_bytes(chunk.as_bytes())?;
        let entry = self
            .offsets
            .get_mut(index)
            .context("PDF object id exceeds xref table")?;
        anyhow::ensure!(entry.is_none(), "duplicate PDF object id");
        *entry = Some(offset);
        Ok(())
    }

    fn finish(&mut self, info: Option<Ref>) -> anyhow::Result<()> {
        let xref_offset = self.offset;
        self.write_bytes(format!("xref\n0 {}\n", self.offsets.len()).as_bytes())?;
        self.write_bytes(b"0000000000 65535 f \n")?;
        for index in 1..self.offsets.len() {
            let offset =
                self.offsets[index].context("PDF xref table contains an unwritten object")?;
            anyhow::ensure!(offset <= 9_999_999_999, "PDF file exceeds xref table limit");
            self.write_bytes(format!("{offset:010} 00000 n \n").as_bytes())?;
        }
        let trailer = if let Some(info) = info {
            format!(
                "trailer\n<< /Size {} /Root {} 0 R /Info {} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                self.offsets.len(),
                CATALOG_REF.get(),
                info.get()
            )
        } else {
            format!(
                "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                self.offsets.len(),
                CATALOG_REF.get()
            )
        };
        self.write_bytes(trailer.as_bytes())
    }
}

/// 嵌入可直接使用的 JPEG；其他格式解码后使用无损 RGB 样本。
fn encode_image_samples(page: &PdfPageImage) -> anyhow::Result<EncodedImageSamples> {
    let reader = image::ImageReader::open(&page.path)
        .with_context(|| format!("failed to open PDF image {}", page.path.display()))?
        .with_guessed_format()
        .with_context(|| format!("failed to inspect PDF image {}", page.path.display()))?;
    let format = reader
        .format()
        .context("PDF image format could not be determined")?;
    anyhow::ensure!(
        format == page.format,
        "PDF image format changed during export: {}",
        page.path.display()
    );
    let mut decoder = reader.into_decoder().with_context(|| {
        format!(
            "failed to initialize PDF image decoder {}",
            page.path.display()
        )
    })?;
    let orientation = decoder.orientation().with_context(|| {
        format!(
            "failed to read PDF image orientation {}",
            page.path.display()
        )
    })?;
    let color_type = decoder.color_type();
    let (source_width, source_height) = decoder.dimensions();
    let (width, height) = oriented_dimensions((source_width, source_height), orientation);
    validate_image_dimensions(width, height)?;
    anyhow::ensure!(
        orientation == page.orientation && width == page.width_px && height == page.height_px,
        "PDF image dimensions changed during export: {}",
        page.path.display()
    );

    if format == image::ImageFormat::Jpeg && orientation == Orientation::NoTransforms {
        let jpeg_bytes = std::fs::read(&page.path)
            .with_context(|| format!("failed to read PDF JPEG {}", page.path.display()))?;
        if let Some((components, has_jfif)) = jpeg_component_count(&jpeg_bytes) {
            let color_space = match (components, color_type, has_jfif) {
                (1, image::ColorType::L8, _) => Some(b"DeviceGray" as &'static [u8]),
                (3, image::ColorType::Rgb8, true) => Some(b"DeviceRGB" as &'static [u8]),
                _ => None,
            };
            if let Some(color_space) = color_space {
                anyhow::ensure!(
                    i32::try_from(jpeg_bytes.len()).is_ok(),
                    "PDF JPEG stream exceeds format limit"
                );
                return Ok(EncodedImageSamples {
                    bytes: jpeg_bytes,
                    color_space,
                    bits_per_component: 8,
                    filter: Filter::DctDecode,
                });
            }
        }
    }

    let mut image = image::DynamicImage::from_decoder(decoder)
        .with_context(|| format!("failed to decode PDF image {}", page.path.display()))?;
    image.apply_orientation(orientation);
    anyhow::ensure!(
        image.width() == page.width_px && image.height() == page.height_px,
        "PDF image dimensions changed during orientation: {}",
        page.path.display()
    );

    let rgba = image.into_rgba8();
    let row_size = usize::try_from(page.width_px)
        .context("image width exceeds addressable memory")?
        .checked_mul(3)
        .context("RGB row length overflow")?;
    let mut row = Vec::with_capacity(row_size);
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    for (x, _, pixel) in rgba.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        row.extend_from_slice(&[
            composite_white(red, alpha),
            composite_white(green, alpha),
            composite_white(blue, alpha),
        ]);
        if x + 1 == page.width_px {
            encoder
                .write_all(&row)
                .context("failed to compress PDF image row")?;
            row.clear();
        }
    }
    Ok(EncodedImageSamples {
        bytes: encoder
            .finish()
            .context("failed to finish PDF image compression")?,
        color_space: b"DeviceRGB",
        bits_per_component: 8,
        filter: Filter::FlateDecode,
    })
}

/// JPEG 的 PDF 直嵌仅接受 PDF 标准可识别的单通道图或 JFIF 三通道图。
fn jpeg_component_count(bytes: &[u8]) -> Option<(u8, bool)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2;
    let mut has_jfif = false;
    while offset < bytes.len() {
        if bytes.get(offset) != Some(&0xff) {
            return None;
        }
        while bytes.get(offset) == Some(&0xff) {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if marker == 0xda || marker == 0xd9 {
            return None;
        }
        if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = usize::from(u16::from_be_bytes([
            *bytes.get(offset)?,
            *bytes.get(offset + 1)?,
        ]));
        if length < 2 || offset.checked_add(length)? > bytes.len() {
            return None;
        }
        if marker == 0xe0 && bytes.get(offset + 2..offset + 7) == Some(&b"JFIF\0"[..]) {
            has_jfif = true;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if length < 8 {
                return None;
            }
            return Some((*bytes.get(offset + 7)?, has_jfif));
        }
        offset += length;
    }
    None
}

/// 把透明像素与白色页面合成，避免阅读器以黑底显示透明通道。
fn composite_white(channel: u8, alpha: u8) -> u8 {
    let alpha = u16::from(alpha);
    let value = (u16::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255;
    u8::try_from(value).expect("composited color channel fits in u8")
}

/// 拒绝超大像素面，避免损坏图像诱发不可控的解码内存消耗。
fn validate_image_dimensions(width: u32, height: u32) -> anyhow::Result<()> {
    anyhow::ensure!(width > 0 && height > 0, "image dimensions must be positive");
    let pixels = u64::from(width) * u64::from(height);
    anyhow::ensure!(
        pixels <= MAX_IMAGE_PIXELS,
        "image exceeds the supported PDF pixel limit"
    );
    Ok(())
}

/// PDF content operands use f32; bounds are capped before conversion.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "pdf-writer accepts f32 coordinates and dimensions"
)]
fn pdf_float(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value.abs() <= f64::from(f32::MAX));
    value as f32
}

/// 按页面尺寸与填充方式计算图片绘制矩形。
pub(super) fn calculate_page_layout(
    image_size_px: (u32, u32),
    page_size_px: (f64, f64),
    options: &ComicPdfOptions,
) -> anyhow::Result<PageLayout> {
    validate_pdf_options(options)?;
    let (image_width_px, image_height_px) = image_size_px;
    let (canvas_width_pixels, canvas_height_pixels) = page_size_px;
    validate_image_dimensions(image_width_px, image_height_px)?;
    validate_resolved_page_size((canvas_width_pixels, canvas_height_pixels), options.dpi)?;

    let point_scale = 72.0 / f64::from(options.dpi);
    let canvas_width_points = canvas_width_pixels * point_scale;
    let canvas_height_points = canvas_height_pixels * point_scale;

    let (draw_width_points, draw_height_points) = image_size_in_points(
        image_width_px,
        image_height_px,
        canvas_width_points,
        canvas_height_points,
        options.fill_mode,
    );
    let horizontal_offset_pt = horizontal_offset(
        canvas_width_points,
        draw_width_points,
        options.horizontal_alignment,
    );
    let vertical_offset_pt = vertical_offset(
        canvas_height_points,
        draw_height_points,
        options.vertical_alignment,
    );

    Ok(PageLayout {
        page_width_pt: canvas_width_points,
        page_height_pt: canvas_height_points,
        image_width_pt: draw_width_points,
        image_height_pt: draw_height_points,
        offset_x_pt: horizontal_offset_pt,
        offset_y_pt: vertical_offset_pt,
        clips: options.fill_mode == ComicPdfFillMode::Cover,
    })
}

/// 按用户指定的水平方向计算图片偏移。
fn horizontal_offset(page: f64, image: f64, alignment: ComicPdfHorizontalAlignment) -> f64 {
    match alignment {
        ComicPdfHorizontalAlignment::Left => 0.0,
        ComicPdfHorizontalAlignment::Center => (page - image) / 2.0,
        ComicPdfHorizontalAlignment::Right => page - image,
    }
}

/// 按用户指定的垂直方向计算图片偏移；PDF 的纵轴从页面底部开始。
fn vertical_offset(page: f64, image: f64, alignment: ComicPdfVerticalAlignment) -> f64 {
    match alignment {
        ComicPdfVerticalAlignment::Top => page - image,
        ComicPdfVerticalAlignment::Center => (page - image) / 2.0,
        ComicPdfVerticalAlignment::Bottom => 0.0,
    }
}

/// 在页面尺寸内计算保比例的绘制大小，拉伸模式直接使用页面尺寸。
fn image_size_in_points(
    source_width_px: u32,
    source_height_px: u32,
    canvas_width_pt: f64,
    canvas_height_pt: f64,
    fill_mode: ComicPdfFillMode,
) -> (f64, f64) {
    if fill_mode == ComicPdfFillMode::Stretch {
        return (canvas_width_pt, canvas_height_pt);
    }

    let scale_x = canvas_width_pt / f64::from(source_width_px);
    let scale_y = canvas_height_pt / f64::from(source_height_px);
    let scale = if fill_mode == ComicPdfFillMode::Contain {
        scale_x.min(scale_y)
    } else {
        scale_x.max(scale_y)
    };
    (
        f64::from(source_width_px) * scale,
        f64::from(source_height_px) * scale,
    )
}

/// 计算所有漫画页宽、高的算术平均值；封面不属于统计样本。
pub(super) fn average_page_dimensions(dimensions: &[(u32, u32)]) -> anyhow::Result<(f64, f64)> {
    anyhow::ensure!(!dimensions.is_empty(), "no comic page dimensions available");
    anyhow::ensure!(
        dimensions
            .iter()
            .all(|(width, height)| *width > 0 && *height > 0),
        "image dimensions must be positive"
    );

    let count = f64::from(
        u32::try_from(dimensions.len()).context("comic page count exceeds average limit")?,
    );
    let (width_sum, height_sum) =
        dimensions
            .iter()
            .fold((0.0, 0.0), |(width_sum, height_sum), (width, height)| {
                (
                    width_sum + f64::from(*width),
                    height_sum + f64::from(*height),
                )
            });
    Ok((width_sum / count, height_sum / count))
}

/// 解析 PDF 文本串：十六进制串按 UTF-16BE 解码，字面串按转义解码，供单测跨模块复用。
#[cfg(test)]
pub(super) fn decoded_text_values(pdf: &[u8], key: &str) -> Vec<String> {
    fn find_key(pdf: &[u8], key: &str, from: usize) -> Option<usize> {
        let needle = format!("/{key} ");
        pdf.get(from..)?
            .windows(needle.len())
            .position(|window| window == needle.as_bytes())
            .map(|index| from + index)
    }

    fn hex_value(byte: u8) -> u8 {
        u8::try_from((byte as char).to_digit(16).expect("hex outline title"))
            .expect("hex outline title")
    }

    let mut titles = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = find_key(pdf, key, cursor) {
        cursor = offset + "/".len() + key.len() + " ".len();
        let title = if pdf[cursor] == b'<' {
            cursor += 1;
            let mut raw = Vec::new();
            while pdf[cursor] != b'>' {
                raw.push(hex_value(pdf[cursor]) << 4 | hex_value(pdf[cursor + 1]));
                cursor += 2;
            }
            cursor += 1;
            // pdf-writer 按 u16 逐字转十六进制，逐字节拼回大端字。
            let mut words = Vec::new();
            let mut index = 0;
            while index + 1 < raw.len() {
                words.push(u16::from_be_bytes([raw[index], raw[index + 1]]));
                index += 2;
            }
            let words = words.strip_prefix(&[0xFEFF]).unwrap_or(&words);
            String::from_utf16(words).expect("UTF-16 outline title")
        } else if pdf[cursor] == b'(' {
            cursor += 1;
            let mut raw = Vec::new();
            loop {
                let byte = pdf[cursor];
                if byte == b'\\' {
                    cursor += 1;
                    raw.push(pdf[cursor]);
                } else if byte == b')' {
                    break;
                } else {
                    raw.push(byte);
                }
                cursor += 1;
            }
            cursor += 1;
            // 字面串仅出现在纯 ASCII 标题，与 PDFDocEncoding 一致。
            String::from_utf8(raw).expect("ASCII outline title")
        } else {
            panic!("unsupported outline title form");
        };
        titles.push(title);
    }
    titles
}

/// 解析 PDF 书签标题（`/Title` 的特化），供单测跨模块复用。
#[cfg(test)]
pub(super) fn decoded_outline_titles(pdf: &[u8]) -> Vec<String> {
    decoded_text_values(pdf, "Title")
}

#[cfg(test)]
mod tests {
    use super::{
        ComicPdfFillMode, ComicPdfHorizontalAlignment, ComicPdfOptions, ComicPdfPageSize,
        ComicPdfVerticalAlignment, Orientation, PdfDocInfo, PdfOutlineEntry, PdfPageImage,
        average_page_dimensions, calculate_page_layout, inspect_page_image,
        validate_image_dimensions, validate_pdf_options, write_pdf_file, write_pdf_to,
    };

    #[test]
    fn contain_preserves_aspect_ratio_and_centers_image() {
        let options = ComicPdfOptions {
            fill_mode: ComicPdfFillMode::Contain,
            ..ComicPdfOptions::default()
        };
        let layout = calculate_page_layout((100, 200), (200.0, 200.0), &options).expect("layout");

        assert_eq!(layout.page_width_pt, 48.0);
        assert_eq!(layout.page_height_pt, 48.0);
        assert_eq!(layout.image_width_pt, 24.0);
        assert_eq!(layout.image_height_pt, 48.0);
        assert_eq!(layout.offset_x_pt, 12.0);
        assert_eq!(layout.offset_y_pt, 0.0);
        assert!(!layout.clips);
    }

    #[test]
    fn cover_crops_to_the_selected_top_right_alignment() {
        let options = ComicPdfOptions {
            fill_mode: ComicPdfFillMode::Cover,
            horizontal_alignment: ComicPdfHorizontalAlignment::Right,
            vertical_alignment: ComicPdfVerticalAlignment::Top,
            ..ComicPdfOptions::default()
        };
        let layout = calculate_page_layout((100, 200), (200.0, 100.0), &options).expect("layout");

        assert_eq!(layout.image_width_pt, 48.0);
        assert_eq!(layout.image_height_pt, 96.0);
        assert_eq!(layout.offset_x_pt, 0.0);
        assert_eq!(layout.offset_y_pt, -72.0);
        assert!(layout.clips);
    }

    #[test]
    fn alignment_positions_use_available_whitespace_on_either_axis() {
        let horizontal_positions = [
            (ComicPdfHorizontalAlignment::Left, 0.0),
            (ComicPdfHorizontalAlignment::Center, 12.0),
            (ComicPdfHorizontalAlignment::Right, 24.0),
        ];
        for (horizontal_alignment, expected_x) in horizontal_positions {
            let options = ComicPdfOptions {
                horizontal_alignment,
                ..ComicPdfOptions::default()
            };
            let layout =
                calculate_page_layout((50, 100), (200.0, 200.0), &options).expect("layout");
            assert_eq!(layout.offset_x_pt, expected_x);
            assert_eq!(layout.offset_y_pt, 0.0);
        }

        let vertical_positions = [
            (ComicPdfVerticalAlignment::Top, 24.0),
            (ComicPdfVerticalAlignment::Center, 12.0),
            (ComicPdfVerticalAlignment::Bottom, 0.0),
        ];
        for (vertical_alignment, expected_y) in vertical_positions {
            let options = ComicPdfOptions {
                vertical_alignment,
                ..ComicPdfOptions::default()
            };
            let layout =
                calculate_page_layout((100, 50), (200.0, 200.0), &options).expect("layout");
            assert_eq!(layout.offset_x_pt, 0.0);
            assert_eq!(layout.offset_y_pt, expected_y);
        }
    }

    #[test]
    fn stretch_uses_the_whole_page_without_clipping() {
        let options = ComicPdfOptions {
            fill_mode: ComicPdfFillMode::Stretch,
            horizontal_alignment: ComicPdfHorizontalAlignment::Right,
            vertical_alignment: ComicPdfVerticalAlignment::Top,
            ..ComicPdfOptions::default()
        };
        let layout = calculate_page_layout((100, 200), (200.0, 100.0), &options).expect("layout");

        assert_eq!(
            (layout.image_width_pt, layout.image_height_pt),
            (48.0, 24.0)
        );
        assert_eq!((layout.offset_x_pt, layout.offset_y_pt), (0.0, 0.0));
        assert!(!layout.clips);
    }

    #[test]
    fn average_page_dimensions_are_calculated_per_axis() {
        assert_eq!(
            average_page_dimensions(&[(100, 200), (300, 100)]).expect("average"),
            (200.0, 150.0)
        );
        assert!(average_page_dimensions(&[]).is_err());
    }

    #[test]
    fn fixed_size_requires_positive_dimensions_and_supported_dpi() {
        let mut options = ComicPdfOptions {
            page_size: ComicPdfPageSize::Fixed,
            width_px: 1000,
            height_px: 1500,
            dpi: 300,
            fill_mode: ComicPdfFillMode::Contain,
            horizontal_alignment: ComicPdfHorizontalAlignment::Center,
            vertical_alignment: ComicPdfVerticalAlignment::Center,
            include_cover: false,
            ..ComicPdfOptions::default()
        };
        assert!(validate_pdf_options(&options).is_ok());

        options.width_px = 0;
        assert!(validate_pdf_options(&options).is_err());
        options.width_px = 1000;
        options.dpi = 601;
        assert!(validate_pdf_options(&options).is_err());
    }

    #[test]
    fn page_size_point_limit_covers_fixed_boundary_and_average() {
        // 固定尺寸边界：300 DPI 下 60000px 恰为 14400pt，通过；任一轴多 1px 即超限。
        assert!(validate_pdf_options(&fixed_options(60_000, 60_000, 300)).is_ok());
        for (width_px, height_px) in [(60_001, 60_000), (60_000, 60_001)] {
            assert!(
                validate_pdf_options(&fixed_options(width_px, height_px, 300)).is_err(),
                "fixed {width_px}x{height_px} at 300 DPI must exceed the limit"
            );
        }
        // 72 DPI 下边界为 14400px。
        assert!(validate_pdf_options(&fixed_options(14_400, 14_400, 72)).is_ok());
        assert!(validate_pdf_options(&fixed_options(14_401, 14_400, 72)).is_err());

        // 平均模式超限同样拦截：60001px 平均宽在 300 DPI 下约 14400.24pt。
        assert!(average_page_dimensions(&[(60_001, 1)]).is_ok());
        assert!(
            calculate_page_layout((1, 1), (60_001.0, 1.0), &ComicPdfOptions::default()).is_err()
        );
    }

    #[test]
    fn average_page_size_over_limit_fails_before_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-average-limit-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let output = dir.join("book.pdf");
        std::fs::write(&output, b"existing PDF").expect("write existing PDF");
        let page = PdfPageImage {
            path: dir.join("dummy.png"),
            width_px: 60_001,
            height_px: 1,
            format: image::ImageFormat::Png,
            orientation: Orientation::NoTransforms,
        };

        let result = write_pdf_file(
            &output,
            &[page],
            None,
            &ComicPdfOptions::default(),
            &[],
            &PdfDocInfo::default(),
            None,
        );
        assert!(
            result.is_err_and(|error| error.to_string().contains("exceed the supported limit")),
            "oversize average must fail with a limit error"
        );
        assert_eq!(
            std::fs::read(&output).expect("read existing PDF"),
            b"existing PDF"
        );
        assert_eq!(
            std::fs::read_dir(&dir).expect("read test dir").count(),
            1,
            "oversize average must fail before creating a temp PDF"
        );
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn original_size_gives_each_page_its_own_dimensions() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-original-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let wide_path = dir.join("wide.png");
        let tall_path = dir.join("tall.png");
        write_test_png_size(&wide_path, [255, 0, 0], 10, 5);
        write_test_png_size(&tall_path, [0, 0, 255], 5, 10);
        let pages = [
            inspect_page_image(&wide_path).expect("inspect wide page"),
            inspect_page_image(&tall_path).expect("inspect tall page"),
        ];
        let output = dir.join("book.pdf");
        let options = ComicPdfOptions {
            page_size: ComicPdfPageSize::Original,
            ..ComicPdfOptions::default()
        };

        write_pdf_file(
            &output,
            &pages,
            None,
            &options,
            &[],
            &PdfDocInfo::default(),
            None,
        )
        .expect("write PDF");

        let pdf = std::fs::read(&output).expect("read PDF");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Count 2"));
        // 300DPI 下 10px 为 2.4pt、5px 为 1.2pt，两页画布各随原图。
        assert!(text.contains("/MediaBox [0 0 2.4 1.2]"));
        assert!(text.contains("/MediaBox [0 0 1.2 2.4]"));
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn outline_entries_link_volumes_and_open_panel() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-outline-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let image_path = dir.join("page.png");
        std::fs::write(&image_path, PIXEL_PNG).expect("write test image");
        let image = inspect_page_image(&image_path).expect("inspect test image");
        let pages = vec![image.clone(), image];
        let outline = vec![
            PdfOutlineEntry {
                title: "第1话".to_string(),
                page_index: 0,
            },
            PdfOutlineEntry {
                title: "第2话".to_string(),
                page_index: 1,
            },
        ];
        let output = dir.join("book.pdf");

        write_pdf_file(
            &output,
            &pages,
            None,
            &ComicPdfOptions::default(),
            &outline,
            &PdfDocInfo::default(),
            None,
        )
        .expect("write PDF");

        let pdf = std::fs::read(&output).expect("read PDF");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Type /Outlines"));
        assert!(text.contains("/PageMode /UseOutlines"));
        assert_eq!(
            super::decoded_outline_titles(&pdf),
            vec!["第1话".to_string(), "第2话".to_string()]
        );
        // 两页均为 1x1：页对象编号为 5 与 8，书签直指对应页。
        assert!(text.contains("/Dest [5 0 R /Fit]"));
        assert!(text.contains("/Dest [8 0 R /Fit]"));
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn doc_info_writes_info_dict_and_lang() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-docinfo-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let image_path = dir.join("page.png");
        std::fs::write(&image_path, PIXEL_PNG).expect("write test image");
        let image = inspect_page_image(&image_path).expect("inspect test image");
        let info = PdfDocInfo {
            entries: vec![
                (&b"Title"[..], "测试书".to_string()),
                (&b"Author"[..], "测试作者".to_string()),
            ],
            lang: Some("zh-CN".to_string()),
        };
        let output = dir.join("book.pdf");

        write_pdf_file(
            &output,
            &[image],
            None,
            &ComicPdfOptions::default(),
            &[],
            &info,
            None,
        )
        .expect("write PDF");

        let pdf = std::fs::read(&output).expect("read PDF");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Creator (Ebook Studio)"));
        assert!(text.contains("/Producer (Ebook Studio)"));
        assert!(text.contains("/CreationDate (D:"));
        assert!(text.contains("/ModDate (D:"));
        assert!(text.contains("/Lang (zh-CN)"));
        // 单页时信息字典编号为 6，trailer 引用同步。
        assert!(text.contains("/Info 6 0 R"));
        // 对象总数 2 + 3 + 1，交叉引用表同步扩容。
        assert!(text.contains("xref\n0 7\n"));
        assert_eq!(
            super::decoded_text_values(&pdf, "Title"),
            vec!["测试书".to_string()]
        );
        assert_eq!(
            super::decoded_text_values(&pdf, "Author"),
            vec!["测试作者".to_string()]
        );
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn original_canvas_makes_fill_and_alignment_noops() {
        let fills = [
            ComicPdfFillMode::Contain,
            ComicPdfFillMode::Cover,
            ComicPdfFillMode::Stretch,
        ];
        let horizontals = [
            ComicPdfHorizontalAlignment::Left,
            ComicPdfHorizontalAlignment::Center,
            ComicPdfHorizontalAlignment::Right,
        ];
        let verticals = [
            ComicPdfVerticalAlignment::Top,
            ComicPdfVerticalAlignment::Center,
            ComicPdfVerticalAlignment::Bottom,
        ];
        for fill_mode in fills {
            for horizontal_alignment in horizontals {
                for vertical_alignment in verticals {
                    let options = ComicPdfOptions {
                        page_size: ComicPdfPageSize::Original,
                        fill_mode,
                        horizontal_alignment,
                        vertical_alignment,
                        ..ComicPdfOptions::default()
                    };
                    let layout = calculate_page_layout((1860, 2480), (1860.0, 2480.0), &options)
                        .expect("layout");
                    // 同尺寸画布理论上恰好铺满；浮点连乘允许 1ulp 级误差。
                    for (drawn, page) in [
                        (layout.image_width_pt, layout.page_width_pt),
                        (layout.image_height_pt, layout.page_height_pt),
                    ] {
                        assert!(
                            (drawn - page).abs() < 1e-9,
                            "original canvas must fit exactly for {fill_mode:?}"
                        );
                    }
                    assert!(layout.offset_x_pt.abs() < 1e-9);
                    assert!(layout.offset_y_pt.abs() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn rejects_images_over_the_decoded_pixel_budget() {
        assert!(validate_image_dimensions(5001, 10_000).is_err());
        assert!(validate_image_dimensions(5000, 10_000).is_ok());
    }

    #[test]
    fn writes_one_pdf_page_and_progress_unit_per_image() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let image_path = dir.join("page.png");
        std::fs::write(&image_path, PIXEL_PNG).expect("write test image");
        let image = inspect_page_image(&image_path).expect("inspect test image");
        let pages = vec![image.clone(), image];
        let options = ComicPdfOptions::default();
        let progress = std::cell::RefCell::new(Vec::new());
        let emit_progress = |done, total| progress.borrow_mut().push((done, total));
        let mut pdf = std::io::Cursor::new(Vec::new());

        write_pdf_to(
            &mut pdf,
            &pages,
            Some((1.0, 1.0)),
            &options,
            &[],
            &PdfDocInfo::default(),
            Some(&emit_progress),
        )
        .expect("write PDF");

        let bytes = pdf.into_inner();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.7"));
        assert!(text.contains("/Count 2"));
        assert_eq!(text.matches("/Type /Page\n").count(), 2);
        assert!(text.ends_with("%%EOF\n"));
        assert_eq!(*progress.borrow(), vec![(1, 2), (2, 2)]);
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn writes_cover_before_comic_pages_and_replaces_existing_pdf() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-order-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let cover_path = dir.join("cover.png");
        let first_path = dir.join("first.png");
        let second_path = dir.join("second.png");
        write_test_png_size(&cover_path, [0, 255, 0], 2, 1);
        write_test_png(&first_path, [255, 0, 0]);
        write_test_png(&second_path, [0, 0, 255]);
        let cover = inspect_page_image(&cover_path).expect("inspect cover");
        let pages = [
            inspect_page_image(&first_path).expect("inspect first page"),
            inspect_page_image(&second_path).expect("inspect second page"),
        ];
        let output = dir.join("book.pdf");
        std::fs::write(&output, b"old PDF").expect("write existing PDF");
        let options = ComicPdfOptions {
            include_cover: true,
            ..ComicPdfOptions::default()
        };

        write_pdf_file(
            &output,
            &pages,
            Some(&cover),
            &options,
            &[],
            &PdfDocInfo::default(),
            None,
        )
        .expect("write PDF");

        let pdf = std::fs::read(&output).expect("read PDF");
        assert_eq!(
            decoded_pdf_images(&pdf),
            vec![vec![0, 255, 0, 0, 255, 0], vec![255, 0, 0], vec![0, 0, 255]]
        );
        let text = String::from_utf8_lossy(&pdf);
        assert_eq!(text.matches("/MediaBox [0 0 0.24 0.24]").count(), 3);
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn failed_pdf_generation_preserves_existing_output_and_removes_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-failure-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let broken_image = dir.join("broken.png");
        let output = dir.join("book.pdf");
        std::fs::write(&broken_image, b"not an image").expect("write broken image");
        std::fs::write(&output, b"existing PDF").expect("write existing PDF");
        let page = PdfPageImage {
            path: broken_image,
            width_px: 1,
            height_px: 1,
            format: image::ImageFormat::Png,
            orientation: Orientation::NoTransforms,
        };

        assert!(
            write_pdf_file(
                &output,
                &[page],
                None,
                &ComicPdfOptions::default(),
                &[],
                &PdfDocInfo::default(),
                None
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(&output).expect("read existing PDF"),
            b"existing PDF"
        );
        assert_eq!(
            std::fs::read_dir(&dir).expect("read test dir").count(),
            2,
            "failed export must not leave a temp PDF"
        );
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    #[test]
    fn embeds_compatible_jpeg_without_reencoding() {
        let dir = std::env::temp_dir().join(format!(
            "ebook-studio-comic-pdf-jpeg-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        let image_path = dir.join("page.jpg");
        write_test_jpeg(&image_path, [200, 100, 50]);
        let original_jpeg = std::fs::read(&image_path).expect("read source JPEG");
        let page = inspect_page_image(&image_path).expect("inspect source JPEG");
        let output = dir.join("book.pdf");

        write_pdf_file(
            &output,
            &[page],
            None,
            &ComicPdfOptions::default(),
            &[],
            &PdfDocInfo::default(),
            None,
        )
        .expect("write PDF");

        let pdf = std::fs::read(&output).expect("read PDF");
        assert!(
            pdf.windows(b"/Filter /DCTDecode".len())
                .any(|window| { window == b"/Filter /DCTDecode" })
        );
        assert!(
            pdf.windows(original_jpeg.len())
                .any(|window| window == original_jpeg)
        );
        std::fs::remove_dir_all(dir).expect("remove test dir");
    }

    fn write_test_png(path: &std::path::Path, pixel: [u8; 3]) {
        write_test_png_size(path, pixel, 1, 1);
    }

    fn fixed_options(width_px: u32, height_px: u32, dpi: u32) -> ComicPdfOptions {
        ComicPdfOptions {
            page_size: ComicPdfPageSize::Fixed,
            width_px,
            height_px,
            dpi,
            ..ComicPdfOptions::default()
        }
    }

    fn write_test_png_size(path: &std::path::Path, pixel: [u8; 3], width: u32, height: u32) {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb(pixel));
        let mut output = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut output, image::ImageFormat::Png)
            .expect("encode test PNG");
        std::fs::write(path, output.into_inner()).expect("write test PNG");
    }

    fn write_test_jpeg(path: &std::path::Path, pixel: [u8; 3]) {
        let image =
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(1, 1, image::Rgb(pixel)));
        let mut output = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut output, image::ImageFormat::Jpeg)
            .expect("encode test JPEG");
        std::fs::write(path, output.into_inner()).expect("write test JPEG");
    }

    fn decoded_pdf_images(pdf: &[u8]) -> Vec<Vec<u8>> {
        use std::io::Read;

        let mut images = Vec::new();
        let mut search_from = 0;
        while let Some(image_offset) = find_bytes(&pdf[search_from..], b"/Subtype /Image\n") {
            let object_start = search_from + image_offset;
            let stream_offset = object_start
                + find_bytes(&pdf[object_start..], b"stream\n").expect("image stream")
                + b"stream\n".len();
            let length_marker = pdf[..object_start]
                .windows(b"/Length ".len())
                .rposition(|window| window == b"/Length ")
                .expect("stream length");
            let length_start = length_marker + b"/Length ".len();
            let length_end = length_start
                + pdf[length_start..stream_offset]
                    .iter()
                    .position(|byte| !byte.is_ascii_digit())
                    .expect("length terminator");
            let length = std::str::from_utf8(&pdf[length_start..length_end])
                .expect("ASCII stream length")
                .parse::<usize>()
                .expect("numeric stream length");
            let mut samples = Vec::new();
            flate2::read::ZlibDecoder::new(&pdf[stream_offset..stream_offset + length])
                .read_to_end(&mut samples)
                .expect("decode image samples");
            images.push(samples);
            search_from = stream_offset + length + b"\nendstream".len();
        }
        images
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    const PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
}
