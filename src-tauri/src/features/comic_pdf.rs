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
use pdf_writer::{Chunk, Content, Filter, Name, Rect, Ref};

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

/// 漫画 PDF 导出参数；固定尺寸字段在平均模式下忽略。
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

/// 生成多页 PDF；仅保留当前页的解码数据，进度按实际 PDF 页数推进。
pub(super) fn write_pdf_file(
    destination: &Path,
    comic_pages: &[PdfPageImage],
    cover: Option<&PdfPageImage>,
    options: &ComicPdfOptions,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    validate_pdf_options(options)?;
    anyhow::ensure!(!comic_pages.is_empty(), "no comic pages to export as PDF");

    let page_size_px = resolve_page_size(comic_pages, options)?;
    let pages = output_pages(comic_pages, cover, options.include_cover)?;
    let temp_path = temporary_sibling_path(destination)?;

    if let Err(error) = write_pdf_temp(&temp_path, &pages, page_size_px, options, progress) {
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

/// 解析固定或平均像素画布；平均值仅统计 content.json 的漫画页。
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
    page_size_px: (f64, f64),
    options: &ComicPdfOptions,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
        .with_context(|| format!("failed to create temporary PDF {}", temp_path.display()))?;
    let mut writer = BufWriter::new(file);
    write_pdf_to(&mut writer, pages, page_size_px, options, progress)?;
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
    page_size_px: (f64, f64),
    options: &ComicPdfOptions,
    progress: Option<&dyn Fn(u32, u32)>,
) -> anyhow::Result<()> {
    validate_pdf_options(options)?;
    anyhow::ensure!(!pages.is_empty(), "no PDF pages to write");
    let page_count = i32::try_from(pages.len()).context("PDF page count exceeds format limit")?;
    let object_count = pages
        .len()
        .checked_mul(3)
        .and_then(|count| count.checked_add(2))
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
        let layout = calculate_page_layout((page.width_px, page.height_px), page_size_px, options)?;
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

    write_page_tree(&mut sink, &page_refs, page_count)?;
    sink.finish()
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

/// 写入页树和目录根节点。
fn write_page_tree<W: Write>(
    sink: &mut PdfObjectSink<'_, W>,
    page_refs: &[Ref],
    page_count: i32,
) -> anyhow::Result<()> {
    let mut pages = Chunk::new();
    pages
        .pages(PAGES_REF)
        .kids(page_refs.iter().copied())
        .count(page_count);
    sink.write_object(&pages)?;

    let mut catalog = Chunk::new();
    catalog
        .indirect(CATALOG_REF)
        .dict()
        .pair(Name(b"Type"), Name(b"Catalog"))
        .pair(Name(b"Pages"), PAGES_REF);
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

    fn finish(&mut self) -> anyhow::Result<()> {
        let xref_offset = self.offset;
        self.write_bytes(format!("xref\n0 {}\n", self.offsets.len()).as_bytes())?;
        self.write_bytes(b"0000000000 65535 f \n")?;
        for index in 1..self.offsets.len() {
            let offset =
                self.offsets[index].context("PDF xref table contains an unwritten object")?;
            anyhow::ensure!(offset <= 9_999_999_999, "PDF file exceeds xref table limit");
            self.write_bytes(format!("{offset:010} 00000 n \n").as_bytes())?;
        }
        let trailer = format!(
            "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            self.offsets.len(),
            CATALOG_REF.get()
        );
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
    anyhow::ensure!(
        canvas_width_pixels.is_finite()
            && canvas_height_pixels.is_finite()
            && canvas_width_pixels > 0.0
            && canvas_height_pixels > 0.0,
        "PDF page dimensions must be finite and positive"
    );

    let point_scale = 72.0 / f64::from(options.dpi);
    let canvas_width_points = canvas_width_pixels * point_scale;
    let canvas_height_points = canvas_height_pixels * point_scale;
    anyhow::ensure!(
        canvas_width_points <= MAX_PAGE_DIMENSION_PT
            && canvas_height_points <= MAX_PAGE_DIMENSION_PT,
        "PDF page dimensions exceed the supported limit"
    );

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

#[cfg(test)]
mod tests {
    use super::{
        ComicPdfFillMode, ComicPdfHorizontalAlignment, ComicPdfOptions, ComicPdfPageSize,
        ComicPdfVerticalAlignment, Orientation, PdfPageImage, average_page_dimensions,
        calculate_page_layout, inspect_page_image, validate_image_dimensions, validate_pdf_options,
        write_pdf_file, write_pdf_to,
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
        };
        assert!(validate_pdf_options(&options).is_ok());

        options.width_px = 0;
        assert!(validate_pdf_options(&options).is_err());
        options.width_px = 1000;
        options.dpi = 601;
        assert!(validate_pdf_options(&options).is_err());
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

        write_pdf_to(&mut pdf, &pages, (1.0, 1.0), &options, Some(&emit_progress))
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

        write_pdf_file(&output, &pages, Some(&cover), &options, None).expect("write PDF");

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

        assert!(write_pdf_file(&output, &[page], None, &ComicPdfOptions::default(), None).is_err());
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

        write_pdf_file(&output, &[page], None, &ComicPdfOptions::default(), None)
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
