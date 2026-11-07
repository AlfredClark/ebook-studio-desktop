//! 漫画构建：`content.json` + `metadata.json` + `EPUB33-COMIC` 模板 → `<uuid>/build/` 目录树。
//!
//! 与小说构建同构（清空重生成 + 固定文件原样拷 + 模板占位替换 + 返回文件树），
//! 差异：页为整图（`page-template.xhtml` + `EPUB/images/` 归一化命名）；
//! 话标题与页标题格式可配（复用小说模板引擎），默认与编辑步骤原文一致。

use std::path::Path;

use anyhow::Context;
use chrono::{SecondsFormat, Utc};

use super::comic_files::{ComicContent, read_content};
use super::epub_template::read_template_file_in;
use super::novel_build::{
    BuildOutcome, count_width, cover_assets_in, cover_media_type, other_metadata_xml, tree_in,
    write_bytes, xml_escape,
};
use super::novel_export::{BuildTitleOptions, ChapterNumbering, TxtTitleKind, render_title};
use super::project_metadata::load_or_init_metadata_in;
use super::projects::ProjectType;

/// 未分话回落标题：与前端 `comic_untitled_episode` 同文案。
const UNTITLED_VOLUME: &str = "未分话";

/// 宽松转 `u32`：大纲计数上溢时钳制，不报错。
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 话标题行：原文格式用标题原文（无标题回落“未分话”）；内置格式统一为
/// `第<编号>话 <标题>`；自定义用其模板；无标题话 `#T` 按模板严格替换为空串，
/// 模板渲染结果统一去首尾空格。
pub fn comic_heading_volume(
    kind: TxtTitleKind,
    pattern: &str,
    title: Option<&str>,
    number: u32,
    width: usize,
) -> String {
    if kind == TxtTitleKind::Original {
        return title.unwrap_or(UNTITLED_VOLUME).to_string();
    }
    let pure = title.unwrap_or("");
    let rendered = match kind {
        TxtTitleKind::Original => pure.to_string(),
        TxtTitleKind::Arabic => render_title("第#n话 #T", Some(number), width, pure),
        TxtTitleKind::Padded => render_title("第#N话 #T", Some(number), width, pure),
        TxtTitleKind::ChineseLower => render_title("第#c话 #T", Some(number), width, pure),
        TxtTitleKind::ChineseUpper => render_title("第#C话 #T", Some(number), width, pure),
        TxtTitleKind::Custom => render_title(pattern, Some(number), width, pure),
    };
    rendered.trim().to_string()
}

/// 页标题行：页无原文格式（`Original` 按阿拉伯数字处理）；内置统一为 `第<编号>页`；
/// 自定义模板的 `#T` 为所属话标题（平铺无标题话为空串）；结果统一去首尾空格。
pub fn comic_heading_page(
    kind: TxtTitleKind,
    pattern: &str,
    volume_title: Option<&str>,
    number: u32,
    width: usize,
) -> String {
    let owner = volume_title.unwrap_or("");
    let rendered = match kind {
        TxtTitleKind::Original | TxtTitleKind::Arabic => {
            render_title("第#n页", Some(number), width, owner)
        }
        TxtTitleKind::Padded => render_title("第#N页", Some(number), width, owner),
        TxtTitleKind::ChineseLower => render_title("第#c页", Some(number), width, owner),
        TxtTitleKind::ChineseUpper => render_title("第#C页", Some(number), width, owner),
        TxtTitleKind::Custom => render_title(pattern, Some(number), width, owner),
    };
    rendered.trim().to_string()
}

/// 页标题编号宽度：连续模式取全书总页数位数，重置模式取各话最大页数位数（最低 2 位）。
pub fn page_title_width(content: &ComicContent, numbering: ChapterNumbering) -> usize {
    match numbering {
        ChapterNumbering::Continuous => count_width(
            content
                .volumes
                .iter()
                .map(|volume| volume.pages.len())
                .sum(),
        ),
        ChapterNumbering::PerVolume => count_width(
            content
                .volumes
                .iter()
                .map(|volume| volume.pages.len())
                .max()
                .unwrap_or(0),
        ),
    }
}

/// 页标题编号：连续模式取文档顺序全局序号，重置模式用话内序号。
pub const fn page_number(global_seq: u32, local: u32, numbering: ChapterNumbering) -> u32 {
    match numbering {
        ChapterNumbering::Continuous => global_seq,
        ChapterNumbering::PerVolume => local,
    }
}

/// 标题格式预览：按话取首个标题渲染一行，供前端下拉后方展示效果；
/// 无对应标题直接报错，调用方隐藏预览。
pub fn preview_title_in(
    project_dir: &Path,
    kind: TxtTitleKind,
    pattern: &str,
    is_volume: bool,
    numbering: ChapterNumbering,
) -> anyhow::Result<String> {
    let content = read_content(project_dir)?;
    if is_volume {
        let (index, volume) = content
            .volumes
            .iter()
            .enumerate()
            .next()
            .context("no volumes to preview")?;
        let width = count_width(content.volumes.len());
        return Ok(comic_heading_volume(
            kind,
            pattern,
            volume.title.as_deref(),
            saturating_u32(index + 1),
            width,
        ));
    }
    let (volume_index, page_index) = content
        .volumes
        .iter()
        .enumerate()
        .find_map(|(volume_index, volume)| {
            if volume.pages.is_empty() {
                None
            } else {
                Some((volume_index, 0))
            }
        })
        .context("no pages to preview")?;
    let width = page_title_width(&content, numbering);
    Ok(comic_heading_page(
        kind,
        pattern,
        content.volumes[volume_index].title.as_deref(),
        page_number(1, saturating_u32(page_index + 1), numbering),
        width,
    ))
}

/// 标题格式预览（`AppHandle` 入口）。
pub fn preview_title(
    app: &tauri::AppHandle,
    id: &str,
    kind: TxtTitleKind,
    pattern: &str,
    is_volume: bool,
    numbering: ChapterNumbering,
) -> anyhow::Result<String> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    preview_title_in(&dir, kind, pattern, is_volume, numbering)
}

/// 构建全量生成：先清空构建目录再重写，调用前由前端确认框兜底。
/// 文件命名：平铺 `page-<全局>`（宽度按总数位数、最低 3 位）；
/// 分话 `page-<话>-<页>`（话宽取话总数位数、页宽取各话最大页数位数，最低 2 位）；
/// 图片与 XHTML 同主干，扩展名沿用源文件（小写），天然防跨话重名。
/// 标题格式经 `titles` 传入：话标题默认原文，页标题默认阿拉伯数字（与旧版输出一致）。
#[allow(clippy::too_many_lines)]
pub fn build_in(
    project_dir: &Path,
    template_base: &Path,
    titles: &BuildTitleOptions,
) -> anyhow::Result<BuildOutcome> {
    if matches!(titles.chapter_kind, TxtTitleKind::Custom)
        && titles.chapter_pattern.trim().is_empty()
    {
        anyhow::bail!("custom page pattern must not be empty");
    }
    if matches!(titles.volume_kind, TxtTitleKind::Custom) && titles.volume_pattern.trim().is_empty()
    {
        anyhow::bail!("custom volume pattern must not be empty");
    }
    let metadata = load_or_init_metadata_in(project_dir)?;
    let content = read_content(project_dir)?;
    let build_dir = project_dir.join("build");
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .with_context(|| format!("failed to clear {}", build_dir.display()))?;
    }

    let template = |rel: &str| read_template_file_in(template_base, rel);
    let page_template = template("EPUB/pages/page-template.xhtml")?;

    // 固定与静态文件原样拷贝。
    for rel in ["mimetype", "META-INF/container.xml", "EPUB/styles/base.css"] {
        let text = template(rel)?;
        write_bytes(&build_dir, rel, text.as_bytes())?;
    }

    let book_title = xml_escape(&metadata.title);
    let book_language = xml_escape(&metadata.language);
    let lang_attr = format!("lang=\"{book_language}\"");
    // 分话判定与文件 / 编辑页识别卡同口径：多话，或单话有标题。
    let grouped = content.volumes.len() > 1
        || content
            .volumes
            .first()
            .is_some_and(|volume| volume.title.is_some());
    let volume_width = count_width(content.volumes.len());
    // 页标题补零宽度：连续按总数、重置按各话最大，与小说同口径。
    let page_width = page_title_width(&content, titles.chapter_numbering);
    // 文件名宽度与标题选项解耦：固定按计数，保证同内容构建产物稳定。
    let file_page_width = count_width(
        content
            .volumes
            .iter()
            .map(|volume| volume.pages.len())
            .max()
            .unwrap_or(0),
    );
    // 平铺计数宽度：沿用小说口径（总数位数、最低 3 位）。
    let flat_width = count_width(
        content
            .volumes
            .iter()
            .map(|volume| volume.pages.len())
            .sum(),
    )
    .max(3);
    let mut manifest_items = Vec::new();
    let mut spine_items = Vec::new();
    let mut toc_entries = Vec::new();
    // 首个正文页：导览 landmarks 指向。
    let mut first_bodymatter: Option<String> = None;
    let mut global_seq = 0u32;

    for (volume_index, volume) in content.volumes.iter().enumerate() {
        let volume_no = volume_index + 1;
        let rendered_volume = comic_heading_volume(
            titles.volume_kind,
            &titles.volume_pattern,
            volume.title.as_deref(),
            saturating_u32(volume_no),
            volume_width,
        );
        let title_escaped = xml_escape(&rendered_volume);
        let mut volume_href: Option<String> = None;
        for (page_index, page) in volume.pages.iter().enumerate() {
            global_seq += 1;
            let local = page_index + 1;
            let stem = if grouped {
                format!("page-{volume_no:0>volume_width$}-{local:0>file_page_width$}")
            } else {
                format!("page-{global_seq:0>flat_width$}")
            };
            let label = comic_heading_page(
                titles.chapter_kind,
                &titles.chapter_pattern,
                volume.title.as_deref(),
                page_number(global_seq, saturating_u32(local), titles.chapter_numbering),
                page_width,
            );
            // 图片拷贝：扩展名沿用源文件（小写），媒体类型由扩展名推导。
            let source = project_dir
                .join("sources")
                .join("images")
                .join(Path::new(&page.path));
            let bytes = std::fs::read(&source)
                .with_context(|| format!("failed to read {}", source.display()))?;
            let ext = source
                .extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_ascii_lowercase)
                .with_context(|| format!("image has no extension: {}", source.display()))?;
            let media = cover_media_type(&ext)?;
            // 固定版式视口：读图头宽高填 viewport，失败回落 800x1280（墨水屏 6 寸常见比例）。
            let (page_width_px, page_height_px) =
                image::image_dimensions(&source).unwrap_or((800, 1280));
            let image_href = format!("EPUB/images/{stem}.{ext}");
            write_bytes(&build_dir, &image_href, &bytes)?;
            let file = format!("{stem}.xhtml");
            let href = format!("EPUB/pages/{file}");
            let rendered = page_template
                .replace("{{PAGE_TITLE}}", &xml_escape(&label))
                .replace("{{PAGE_IMAGE_HREF}}", &format!("../images/{stem}.{ext}"))
                .replace("{{PAGE_WIDTH}}", &page_width_px.to_string())
                .replace("{{PAGE_HEIGHT}}", &page_height_px.to_string())
                .replace("lang=\"zh-CN\"", &lang_attr);
            write_bytes(&build_dir, &href, rendered.as_bytes())?;
            // 固定版式声明放 spine itemref；manifest 只声明媒体类型（`rendition:*` 不属 manifest 词表）。
            manifest_items.push(format!(
                "    <item id=\"{stem}\" href=\"pages/{file}\" media-type=\"application/xhtml+xml\"/>"
            ));
            manifest_items.push(format!(
                "    <item id=\"{stem}-image\" href=\"images/{stem}.{ext}\" media-type=\"{media}\"/>"
            ));
            spine_items.push(format!(
                "    <itemref idref=\"{stem}\" properties=\"rendition:layout-pre-paginated rendition:page-spread-center\"/>"
            ));
            volume_href.get_or_insert_with(|| href.clone());
            first_bodymatter.get_or_insert_with(|| href.clone());
        }
        if grouped {
            toc_entries.push(format!(
                "        <li><a href=\"{}\">{}</a></li>",
                volume_href
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches("EPUB/"),
                title_escaped
            ));
        } else {
            // 平铺无话：目录仅保留首面入口，避免数百页条目拖慢墨水屏。
            if toc_entries.is_empty()
                && let Some(first) = volume_href.as_deref()
            {
                toc_entries.push(format!(
                    "        <li><a href=\"{}\">正文</a></li>",
                    first.trim_start_matches("EPUB/"),
                ));
            }
        }
    }
    if global_seq == 0 {
        anyhow::bail!("no pages to build, parse the images or restore the order first");
    }

    // 封面：有导入才拷贝图片并挂清单，无封面回落书名块。
    let (cover_item, cover_block) =
        cover_assets_in(project_dir, &build_dir, &metadata, &book_title);
    // 封面视口：固定版式每份内容文档都要有 viewport，读已落盘封面尺寸，失败回落 800x1280。
    let (cover_width, cover_height) = (|| {
        let images = build_dir.join("EPUB/images");
        let Ok(entries) = std::fs::read_dir(&images) else {
            return None;
        };
        entries.flatten().find_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("cover.") {
                image::image_dimensions(entry.path()).ok()
            } else {
                None
            }
        })
    })()
    .unwrap_or((800, 1280));

    // 其它元数据：空值省略，系列三件套成组出现（与小说同口径）。
    let other_metadata = other_metadata_xml(&metadata);

    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut opf = template("EPUB/content.opf")?
        .replace("{{BOOK_UUID}}", &metadata.identifier)
        .replace("{{BOOK_TITLE}}", &book_title)
        .replace("{{BOOK_LANGUAGE}}", &book_language)
        .replace("{{BOOK_MODIFIED}}", &now)
        .replace("{{BOOK_AUTHOR}}", &xml_escape(&metadata.creator))
        .replace("{{CONTRIBUTOR}}", &xml_escape(&metadata.contributor))
        .replace("{{OTHER_METADATA}}", &other_metadata)
        .replace("{{COVER_ITEM}}", &cover_item)
        .replace("{{MANIFEST_ITEMS}}", &manifest_items.join("\n"))
        .replace("{{NAV_ITEMREF}}", "")
        .replace("{{SPINE_ITEMS}}", &spine_items.join("\n"));
    if metadata.creator.is_empty() {
        opf = opf
            .lines()
            .filter(|line| !line.contains("creator"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    if metadata.contributor.is_empty() {
        opf = opf
            .lines()
            .filter(|line| !line.contains("contributor"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    write_bytes(&build_dir, "EPUB/content.opf", opf.as_bytes())?;

    let landmarks = format!(
        "        <li><a epub:type=\"cover\" href=\"cover.xhtml\">封面</a></li>\n        <li><a epub:type=\"bodymatter\" href=\"{}\">正文开始</a></li>",
        first_bodymatter
            .as_deref()
            .unwrap_or("cover.xhtml")
            .trim_start_matches("EPUB/"),
    );
    let nav = template("EPUB/nav.xhtml")?
        .replace("{{CHAPTER_TOC_LIST}}", &toc_entries.join("\n"))
        .replace("{{LANDMARKS_LIST}}", &landmarks)
        .replace("lang=\"zh-CN\"", &lang_attr);
    write_bytes(&build_dir, "EPUB/nav.xhtml", nav.as_bytes())?;

    let cover = template("EPUB/cover.xhtml")?
        .replace("{{BOOK_TITLE}}", &book_title)
        .replace("{{COVER_BLOCK}}", &cover_block)
        .replace("{{COVER_WIDTH}}", &cover_width.to_string())
        .replace("{{COVER_HEIGHT}}", &cover_height.to_string())
        .replace("lang=\"zh-CN\"", &lang_attr);
    write_bytes(&build_dir, "EPUB/cover.xhtml", cover.as_bytes())?;

    super::projects::touch_updated(project_dir);
    tree_in(project_dir).with_context(|| format!("failed to list {}", build_dir.display()))
}

/// 构建漫画（`AppHandle` 入口）：清空后全量重生成，返回文件树。
pub fn build_comic(
    app: &tauri::AppHandle,
    id: &str,
    titles: &BuildTitleOptions,
) -> anyhow::Result<BuildOutcome> {
    use super::epub_template::template_dir;

    let dir = super::projects::resolve_project_dir(app, id)?;
    let base = template_dir(app, ProjectType::Comic)?;
    build_in(&dir, &base, titles)
}

/// 清空构建产物（`AppHandle` 入口）：删除 `build/`，供确认重构建后回到引导。
pub fn clear_comic(app: &tauri::AppHandle, id: &str) -> anyhow::Result<()> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    super::novel_build::clear_in(&dir)?;
    super::novel_package::clear_dist_in(&dir)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::comic_files;
    use super::super::comic_files::{ComicContent, ComicMode, ComicPage, ComicVolume};
    use super::super::novel_export::BuildTitleOptions;
    use super::super::project_metadata;
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

    /// 用例隔离的项目：建真实漫画项目目录，跑完清理。
    fn test_project(case: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "ebook-studio-comic-build-test-{}-{case}",
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

    /// 写源图片并导入解析，返回项目目录。
    fn fixture_project(case: &str, files: &[&str]) -> (PathBuf, PathBuf) {
        let (root, dir) = test_project(case);
        let source = root.join(format!("{case}-src"));
        let _ = std::fs::remove_dir_all(&source);
        for file in files {
            let path = source.join(file);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create source parent");
            }
            std::fs::write(&path, b"fake-image").expect("write source image");
        }
        comic_files::import_dir_in(&dir, &source).expect("import");
        comic_files::parse_images_in(&dir).expect("parse");
        (root, dir)
    }

    fn template_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/EPUB33-COMIC")
    }

    fn read_built(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join("build").join(rel)).expect("read built file")
    }

    #[test]
    fn builds_flat_pages_with_images_and_toc() {
        let (root, dir) = fixture_project("flat", &["02.jpg", "01.png"]);
        let outcome =
            build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        // 页与图同主干，平铺全局编号。
        assert!(dir.join("build/EPUB/pages/page-001.xhtml").is_file());
        assert!(dir.join("build/EPUB/pages/page-002.xhtml").is_file());
        assert!(dir.join("build/EPUB/images/page-001.png").is_file());
        assert!(dir.join("build/EPUB/images/page-002.jpg").is_file());
        let opf = read_built(&dir, "EPUB/content.opf");
        assert!(opf.contains(r#"<item id="page-001" href="pages/page-001.xhtml""#));
        assert!(opf.contains("rendition:layout-pre-paginated"));
        assert!(opf.contains(
            r#"<item id="page-001-image" href="images/page-001.png" media-type="image/png"/>"#
        ));
        assert!(opf.contains(r#"<itemref idref="page-001" properties="rendition:layout-pre-paginated rendition:page-spread-center"/>"#));
        assert!(opf.contains(r#"xml:lang="zh-CN""#));
        assert!(opf.contains(r#"page-progression-direction="ltr""#));
        // 平铺目录仅首面入口，避免数百页条目拖慢墨水屏。
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains("正文"));
        assert!(!nav.contains("第1页"));
        let page = read_built(&dir, "EPUB/pages/page-001.xhtml");
        assert!(page.contains("../images/page-001.png"));
        assert!(page.contains("第1页"));
        // 假图片读不到尺寸，回落 800x1280 视口。
        assert!(page.contains("width=800,height=1280"));
        // 无封面回落书名块。
        let cover = read_built(&dir, "EPUB/cover.xhtml");
        assert!(cover.contains("book-title"));
        assert!(outcome.file_count > 0);
        cleanup(&root);
    }

    #[test]
    fn builds_volumes_with_nested_toc_and_cover() {
        let (root, dir) =
            fixture_project("volumes", &["第1话/02.jpg", "第1话/01.jpg", "第2话/01.jpg"]);
        // 封面：假 JPEG 头即过导入校验。
        let source = root.join("cover-src.jpg");
        std::fs::write(&source, [0xFF, 0xD8, 0xFF, 0x00]).expect("write fake cover");
        project_metadata::import_cover_in(&dir, &source).expect("import cover");
        let outcome =
            build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        assert!(dir.join("build/EPUB/pages/page-01-01.xhtml").is_file());
        assert!(dir.join("build/EPUB/pages/page-01-02.xhtml").is_file());
        assert!(dir.join("build/EPUB/pages/page-02-01.xhtml").is_file());
        assert!(dir.join("build/EPUB/images/page-01-01.jpg").is_file());
        assert!(dir.join("build/EPUB/images/cover.jpg").is_file());
        // 目录仅话级，页不再进 toc。
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains("第1话") && nav.contains("第2话"));
        assert!(!nav.contains(">第1页<"));
        let opf = read_built(&dir, "EPUB/content.opf");
        assert!(opf.contains(
            r#"<item id="cover-image" href="images/cover.jpg" media-type="image/jpeg" properties="cover-image"/>"#
        ));
        let cover = read_built(&dir, "EPUB/cover.xhtml");
        assert!(cover.contains("images/cover.jpg"));
        assert!(outcome.file_count > 0);
        cleanup(&root);
    }

    #[test]
    fn rejects_build_without_pages() {
        let (root, dir) = test_project("empty");
        // 无 content.json 直接报错，不建半成品。
        assert!(build_in(&dir, &template_base(), &BuildTitleOptions::default()).is_err());
        assert!(!dir.join("build").exists());
        cleanup(&root);
    }

    #[test]
    fn viewport_uses_real_image_dimensions() {
        let (root, dir) = test_project("viewport");
        // 真图片：10x20，构建视口应原样落盘。
        let source = root.join("viewport-src");
        std::fs::create_dir_all(&source).expect("create source");
        let img = image::RgbImage::from_pixel(10, 20, image::Rgb([10, 20, 30]));
        image::DynamicImage::ImageRgb8(img)
            .save(source.join("01.jpg"))
            .expect("save test image");
        comic_files::import_dir_in(&dir, &source).expect("import");
        comic_files::parse_images_in(&dir).expect("parse");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let page = read_built(&dir, "EPUB/pages/page-001.xhtml");
        assert!(page.contains("width=10,height=20"));
        cleanup(&root);
    }

    #[test]
    fn renders_volume_and_page_headings() {
        use super::super::novel_export::{ChapterNumbering, TxtTitleKind};
        use super::{comic_heading_page, comic_heading_volume, page_title_width};

        // 话标题：原文回落未分话，模板渲染后去首尾空格。
        assert_eq!(
            comic_heading_volume(TxtTitleKind::Original, "", None, 1, 2),
            "未分话"
        );
        assert_eq!(
            comic_heading_volume(TxtTitleKind::Arabic, "", Some("开篇"), 3, 2),
            "第3话 开篇"
        );
        assert_eq!(
            comic_heading_volume(TxtTitleKind::Padded, "", Some("开篇"), 3, 2),
            "第03话 开篇"
        );
        assert_eq!(
            comic_heading_volume(TxtTitleKind::Custom, "话#n：#T", None, 2, 2),
            "话2："
        );
        assert_eq!(
            comic_heading_volume(TxtTitleKind::Custom, "##n #T", Some("开篇"), 2, 2),
            "#n 开篇"
        );
        // 页标题：原文按阿拉伯数字处理，`#T` 为所属话标题。
        assert_eq!(
            comic_heading_page(TxtTitleKind::Original, "", Some("开篇"), 5, 2),
            "第5页"
        );
        assert_eq!(
            comic_heading_page(TxtTitleKind::Arabic, "", Some("开篇"), 5, 2),
            "第5页"
        );
        assert_eq!(
            comic_heading_page(TxtTitleKind::Custom, "第#N页（#T）", Some("开篇"), 5, 2),
            "第05页（开篇）"
        );
        assert_eq!(
            comic_heading_page(TxtTitleKind::Custom, "P#n", None, 5, 2),
            "P5"
        );
        // 补零宽度：连续按总数、重置按各话最大。
        let content = ComicContent {
            schema_version: 1,
            mode: ComicMode::Volumes,
            parsed_at: String::new(),
            volumes: vec![
                ComicVolume {
                    title: Some("甲".to_string()),
                    pages: (0..9)
                        .map(|i| ComicPage {
                            path: format!("甲/{i:02}.jpg"),
                        })
                        .collect(),
                },
                ComicVolume {
                    title: Some("乙".to_string()),
                    pages: (0..150)
                        .map(|i| ComicPage {
                            path: format!("乙/{i:03}.jpg"),
                        })
                        .collect(),
                },
            ],
        };
        assert_eq!(page_title_width(&content, ChapterNumbering::Continuous), 3);
        assert_eq!(page_title_width(&content, ChapterNumbering::PerVolume), 3);
        // 页编号：连续取全局，重置取话内。
        assert_eq!(super::page_number(7, 2, ChapterNumbering::Continuous), 7);
        assert_eq!(super::page_number(7, 2, ChapterNumbering::PerVolume), 2);
    }

    #[test]
    fn previews_first_volume_and_page_titles() {
        use super::super::novel_export::{ChapterNumbering, TxtTitleKind};
        let (root, dir) = fixture_project("preview", &["第1话/01.jpg", "第1话/02.jpg"]);
        let volume = preview_title_in(
            &dir,
            TxtTitleKind::Arabic,
            "",
            true,
            ChapterNumbering::PerVolume,
        )
        .expect("preview volume");
        assert_eq!(volume, "第1话 第1话");
        let page = preview_title_in(
            &dir,
            TxtTitleKind::Arabic,
            "",
            false,
            ChapterNumbering::PerVolume,
        )
        .expect("preview page");
        assert_eq!(page, "第1页");
        cleanup(&root);
    }

    #[test]
    fn builds_with_custom_titles_and_continuous_numbering() {
        use super::super::novel_export::{BuildTitleOptions, ChapterNumbering, TxtTitleKind};
        let (root, dir) =
            fixture_project("custom", &["第1话/01.jpg", "第1话/02.jpg", "第2话/01.jpg"]);
        let titles = BuildTitleOptions {
            chapter_kind: TxtTitleKind::Custom,
            chapter_pattern: "P#N".to_string(),
            volume_kind: TxtTitleKind::Arabic,
            volume_pattern: String::new(),
            chapter_numbering: ChapterNumbering::Continuous,
        };
        build_in(&dir, &template_base(), &titles).expect("build");
        // 文件名与标题选项解耦：仍为计数命名；目录仅话级，页标题只进页面标题。
        assert!(dir.join("build/EPUB/pages/page-01-01.xhtml").is_file());
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains(">第1话 第1话<"));
        assert!(!nav.contains(">P03<"));
        let page = read_built(&dir, "EPUB/pages/page-02-01.xhtml");
        assert!(page.contains("<title>P03</title>"));
        cleanup(&root);
    }
}
