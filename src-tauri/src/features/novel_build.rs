//! 小说构建：`content.json` + `metadata.json` + `EPUB33-NOVEL` 模板 → `<uuid>/build/` 目录树。
//!
//! 构建即 EPUB 打包前的目录结构：模板占位符按解析产物与元数据填充，
//! 固定文件（`mimetype` / `container.xml`）原样拷贝；每次构建先清空重生成，
//! 前端在覆盖前弹确认框。落盘格式即前端契约（`snake_case`）。

use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use chrono::{SecondsFormat, Utc};
use quick_xml::Reader;
use quick_xml::events::{BytesEnd, BytesRef, BytesStart, BytesText, Event};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::novel_files::NovelContent;
use super::projects::ProjectType;

/// 构建产物目录：`<uuid>/build/`，与 `sources/` / `metadata.json` 同级。
const BUILD_DIR: &str = "build";
/// 只读固定文件（相对构建根，POSIX 口径）：模板原样拷贝，不接受写入。
const FIXED_FILES: &[&str] = &["mimetype", "META-INF/container.xml"];
/// 只读但可格式化：`container.xml` 无占位符、无需手改，但允许空白规范化。
const FORMATTABLE_FIXED: &[&str] = &["META-INF/container.xml"];
/// 可编辑文本扩展名：命中才接受写入，其余按二进制处理。
const EDITABLE_EXTS: &[&str] = &["xhtml", "opf", "css", "xml"];
/// 封面落盘子目录与主干名：扩展名沿用导入时的原扩展名（小写）。
const COVER_DIR: &str = "EPUB/images";
const COVER_STEM: &str = "cover";

/// 构建文件树节点：`children` 为 `Some` 即目录（不可编辑），`None` 即文件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BuildFileNode {
    /// 显示名（文件含扩展名）。
    pub name: String,
    /// 构建根起的相对路径（POSIX 分隔）。
    pub path: String,
    /// 是否可编辑保存：固定文件与二进制为假。
    pub editable: bool,
    /// 子节点；目录为 `Some`，文件为 `None`。
    #[allow(clippy::use_self)]
    pub children: Option<Vec<BuildFileNode>>,
}

/// 构建文件内容：二进制回 `None`，前端展示不支持预览提示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BuildFileContent {
    /// 文本内容；二进制为 `None`。
    pub content: Option<String>,
    /// 是否可编辑保存。
    pub editable: bool,
    /// 是否可格式化（可编辑文件 + `container.xml`，后者只读、仅内存格式化）。
    pub formattable: bool,
}

/// 构建图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BuildImageInfo {
    /// 图片绝对路径。
    pub path: String,
    /// 图片字节数（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
}

/// 构建结果：文件树根节点清单与文件总数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BuildOutcome {
    /// 构建根下的顶层节点（已排序，目录在前）。
    pub files: Vec<BuildFileNode>,
    /// 全部文件数（不计目录）。
    pub file_count: u32,
}

/// 批量格式化结果：成功数 + 跳过（`mimetype` 与二进制）+ 失败清单（内容保持原样）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct FormatOutcome {
    /// 成功格式化的文件数（含已规范无需改动的）。
    pub formatted: u32,
    /// 跳过的文件。
    pub skipped: Vec<String>,
    /// 格式化失败的文件。
    pub failed: Vec<String>,
}

/// XML 文本转义：`&` 必须最先替换，避免二次转义。
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 校验构建内相对路径：非空、相对、无跳出成分；返回拼接后的文件系统路径。
fn resolve_build_path(build_dir: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    if rel.is_empty() {
        anyhow::bail!("build path must not be empty");
    }
    let mut buf = PathBuf::new();
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(part) => buf.push(part),
            _ => anyhow::bail!("invalid build path: {rel}"),
        }
    }
    if buf.as_os_str().is_empty() {
        anyhow::bail!("invalid build path: {rel}");
    }
    Ok(build_dir.join(buf))
}

/// 相对路径转 POSIX 口径（树节点与前后端契约统一用 `/`）。
fn to_posix(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 是否固定只读文件。
fn is_fixed(rel_posix: &str) -> bool {
    FIXED_FILES.contains(&rel_posix)
}

/// 是否可编辑文本：非固定 + 扩展名命中。
fn is_editable_text(rel_posix: &str) -> bool {
    if is_fixed(rel_posix) {
        return false;
    }
    Path::new(rel_posix)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EDITABLE_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// 是否可格式化文本：可编辑文件 + 只读的 `container.xml`（`mimetype` 与二进制永不参与）。
fn is_formattable(rel_posix: &str) -> bool {
    if FORMATTABLE_FIXED.contains(&rel_posix) {
        return true;
    }
    is_editable_text(rel_posix)
}

/// 封面扩展名到 EPUB 媒体类型：导入白名单的子集，未命中直接报错。
fn cover_media_type(ext: &str) -> anyhow::Result<&'static str> {
    match ext {
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "png" => Ok("image/png"),
        "webp" => Ok("image/webp"),
        "gif" => Ok("image/gif"),
        _ => anyhow::bail!("unsupported cover type: {ext}"),
    }
}

/// 构建产物写入：建父目录后落盘。
fn write_bytes(build_dir: &Path, rel: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let path = resolve_build_path(build_dir, rel)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// 正文行转段落：每行一个 `<p>`，空行已在解析口径丢弃。
fn body_paragraphs(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("      <p>{}</p>", xml_escape(line)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 同类数量位数（文件名补零宽度），最低 2 位，保证全书命名格式统一。
fn count_width(count: usize) -> usize {
    count.to_string().len().max(2)
}

/// 构建全量生成：先清空构建目录再写，调用前由前端确认框兜底。
/// 文件命名：平铺沿用全局三位计数；分卷用 `chapter-<卷>-<章>`（文档顺序），
/// 卷宽取卷总数位数、章宽取各卷最大章数位数（最低 2 位）。
#[allow(clippy::too_many_lines)]
pub fn build_in(
    project_dir: &Path,
    template_base: &Path,
    titles: &super::novel_export::BuildTitleOptions,
) -> anyhow::Result<BuildOutcome> {
    use super::epub_template::read_template_file_in;
    use super::novel_export::{
        ChapterNumbering, TxtTitleKind, chapter_number, chapter_number_width, heading,
    };
    use super::project_metadata::load_or_init_metadata_in;

    if matches!(titles.chapter_kind, TxtTitleKind::Custom)
        && titles.chapter_pattern.trim().is_empty()
    {
        anyhow::bail!("custom chapter pattern must not be empty");
    }
    if matches!(titles.volume_kind, TxtTitleKind::Custom) && titles.volume_pattern.trim().is_empty()
    {
        anyhow::bail!("custom volume pattern must not be empty");
    }

    let metadata = load_or_init_metadata_in(project_dir)?;
    let content: NovelContent = super::novel_files::read_content(project_dir)?;
    let build_dir = project_dir.join(BUILD_DIR);
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .with_context(|| format!("failed to clear {}", build_dir.display()))?;
    }

    let template = |rel: &str| read_template_file_in(template_base, rel);
    let chapter_template = template("EPUB/text/chapter-template.xhtml")?;
    let volume_template = template("EPUB/text/volume-template.xhtml")?;

    // 固定与静态文件原样拷贝。
    for rel in ["mimetype", "META-INF/container.xml", "EPUB/styles/base.css"] {
        let text = template(rel)?;
        write_bytes(&build_dir, rel, text.as_bytes())?;
    }

    let book_title = xml_escape(&metadata.title);
    let book_author = xml_escape(&metadata.creator);
    let mut manifest_items = Vec::new();
    let mut spine_items = Vec::new();
    let mut toc_entries = Vec::new();
    let mut chapter_counter = 0u32;
    // 首个正文页：导览 landmarks 指向，卷页优先于章节页。
    let mut first_bodymatter: Option<String> = None;

    // 分卷判定与识别卡同口径：多卷，或单卷有标题。
    let grouped = content.volumes.len() > 1
        || content
            .volumes
            .first()
            .is_some_and(|v| v.full_title.is_some());
    let volume_width = count_width(content.volumes.len());
    let chapter_width = count_width(
        content
            .volumes
            .iter()
            .map(|volume| volume.chapters.len())
            .max()
            .unwrap_or(0),
    );
    // 跨卷连续的文件名章号取全局序号，宽度按全书总章数位数（最低 2 位）。
    let continuous_width = count_width(
        content
            .volumes
            .iter()
            .map(|volume| volume.chapters.len())
            .sum(),
    );
    // 标题模板数字补零宽度：与导出同口径（最低 2 位）。
    let volume_number_width =
        super::novel_export::number_width(content.volumes.iter().map(|volume| volume.number));
    let chapter_number_width = chapter_number_width(&content, titles.chapter_numbering);
    let mut global_seq = 0u32;

    for (volume_index, volume) in content.volumes.iter().enumerate() {
        let volume_no = volume_index + 1;
        // 有标题行才生成卷页；纯章节的隐式卷跳过，章节直接平铺。
        let mut volume_href: Option<String> = None;
        if let Some(full_title) = volume.full_title.as_deref() {
            let stem = format!("volume-{volume_no:0>volume_width$}");
            let file = format!("{stem}.xhtml");
            let href = format!("EPUB/text/{file}");
            let intro = volume
                .intro
                .iter()
                .map(|line| format!("      <p class=\"volume-intro\">{}</p>", xml_escape(line)))
                .collect::<Vec<_>>()
                .join("\n");
            let rendered = heading(
                titles.volume_kind,
                &titles.volume_pattern,
                full_title,
                volume.title.as_deref().unwrap_or(""),
                volume.number,
                volume_number_width,
                true,
            );
            let page = volume_template
                .replace("{{VOLUME_TITLE}}", &xml_escape(&rendered))
                .replace("{{VOLUME_INTRO}}", &intro);
            write_bytes(&build_dir, &href, page.as_bytes())?;
            manifest_items.push(format!(
                "    <item id=\"{stem}\" href=\"text/{file}\" media-type=\"application/xhtml+xml\"/>"
            ));
            spine_items.push(format!("    <itemref idref=\"{stem}\"/>"));
            toc_entries.push(format!(
                "        <li><a href=\"text/{file}\">{}</a>\n          <ol>",
                xml_escape(&rendered)
            ));
            volume_href = Some(href.clone());
            first_bodymatter.get_or_insert(href);
        }
        for (chapter_index, chapter) in volume.chapters.iter().enumerate() {
            global_seq += 1;
            let stem = if grouped {
                match titles.chapter_numbering {
                    ChapterNumbering::Continuous => format!(
                        "chapter-{volume_no:0>volume_width$}-{global_seq:0>continuous_width$}"
                    ),
                    ChapterNumbering::PerVolume => format!(
                        "chapter-{volume_no:0>volume_width$}-{chapter_no:0>chapter_width$}",
                        chapter_no = chapter_index + 1
                    ),
                }
            } else {
                chapter_counter += 1;
                format!("chapter-{chapter_counter:03}")
            };
            let file = format!("{stem}.xhtml");
            let href = format!("EPUB/text/{file}");
            let rendered = heading(
                titles.chapter_kind,
                &titles.chapter_pattern,
                &chapter.full_title,
                &chapter.title,
                chapter_number(global_seq, chapter.number, titles.chapter_numbering),
                chapter_number_width,
                false,
            );
            let page = chapter_template
                .replace("{{CHAPTER_TITLE}}", &xml_escape(&rendered))
                .replace("{{CHAPTER_BODY}}", &body_paragraphs(&chapter.lines));
            write_bytes(&build_dir, &href, page.as_bytes())?;
            manifest_items.push(format!(
                "    <item id=\"{stem}\" href=\"text/{file}\" media-type=\"application/xhtml+xml\"/>"
            ));
            spine_items.push(format!("    <itemref idref=\"{stem}\"/>"));
            let entry = format!(
                "          <li><a href=\"text/{file}\">{}</a></li>",
                xml_escape(&rendered)
            );
            if volume_href.is_some() {
                toc_entries.push(format!("          {entry}"));
            } else {
                toc_entries.push(format!("        {entry}"));
            }
            first_bodymatter.get_or_insert(href);
        }
        if volume_href.is_some() {
            toc_entries.push("          </ol>\n        </li>".to_string());
        }
    }

    // 封面：有导入才拷贝图片并挂清单，无封面回落书名块。
    let cover_rel = metadata.cover.as_deref().and_then(|rel| {
        let source = project_dir.join(rel);
        let bytes = std::fs::read(&source).ok()?;
        let ext = source
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)?;
        let media = cover_media_type(&ext).ok()?;
        let href = format!("{COVER_DIR}/{COVER_STEM}.{ext}");
        write_bytes(&build_dir, &href, &bytes).ok()?;
        Some((href, media))
    });
    let (cover_item, cover_block) = match &cover_rel {
        // 清单 `href` 与正文 `src` 均相对 `EPUB/`（OPF 与 cover 页同级），包内写盘路径的前缀须剥掉。
        Some((href, media)) => {
            let opf_href = href.trim_start_matches("EPUB/");
            (
                format!(
                    "    <item id=\"cover-image\" href=\"{opf_href}\" media-type=\"{media}\"/>"
                ),
                format!("      <img src=\"{opf_href}\" alt=\"{book_title}\"/>"),
            )
        }
        None => (
            String::new(),
            format!("      <p class=\"book-title\">{book_title}</p>"),
        ),
    };

    // 扉页：简介非空才出描述块。
    let description_block = if metadata.description.is_empty() {
        String::new()
    } else {
        let lines = metadata
            .description
            .iter()
            .map(|line| {
                format!(
                    "        <span class=\"desc-line\">{}</span>",
                    xml_escape(line)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("      <div class=\"book-description\">\n{lines}\n      </div>")
    };

    // 其它元数据：空值省略，系列三件套成组出现。
    let mut other_metadata = Vec::new();
    if !metadata.publisher.is_empty() {
        other_metadata.push(format!(
            "    <dc:publisher>{}</dc:publisher>",
            xml_escape(&metadata.publisher)
        ));
    }
    for subject in &metadata.subjects {
        other_metadata.push(format!(
            "    <dc:subject>{}</dc:subject>",
            xml_escape(subject)
        ));
    }
    if let Some(date) = metadata.date_published.as_deref() {
        other_metadata.push(format!("    <dc:date>{}</dc:date>", xml_escape(date)));
    }
    if !metadata.rights.is_empty() {
        other_metadata.push(format!(
            "    <dc:rights>{}</dc:rights>",
            xml_escape(&metadata.rights)
        ));
    }
    if !metadata.series.is_empty() {
        other_metadata.push(format!(
            "    <meta property=\"belongs-to-collection\" id=\"series\">{}</meta>",
            xml_escape(&metadata.series)
        ));
        other_metadata.push(
            "    <meta refines=\"#series\" property=\"collection-type\">series</meta>".to_string(),
        );
        if !metadata.volume.is_empty() {
            other_metadata.push(format!(
                "    <meta refines=\"#series\" property=\"group-position\">{}</meta>",
                xml_escape(&metadata.volume)
            ));
        }
    }
    if !metadata.source.is_empty() {
        other_metadata.push(format!(
            "    <dc:source>{}</dc:source>",
            xml_escape(&metadata.source)
        ));
    }

    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut opf = template("EPUB/content.opf")?
        .replace("{{BOOK_UUID}}", &metadata.identifier)
        .replace("{{BOOK_TITLE}}", &book_title)
        .replace("{{BOOK_MODIFIED}}", &now)
        .replace("{{BOOK_AUTHOR}}", &book_author)
        .replace("{{CONTRIBUTOR}}", &xml_escape(&metadata.contributor))
        .replace("{{OTHER_METADATA}}", &other_metadata.join("\n"))
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
        "        <li><a epub:type=\"cover\" href=\"cover.xhtml\">封面</a></li>\n        <li><a epub:type=\"titlepage\" href=\"titlepage.xhtml\">扉页</a></li>\n        <li><a epub:type=\"bodymatter\" href=\"{}\">正文开始</a></li>",
        first_bodymatter
            .as_deref()
            .unwrap_or("titlepage.xhtml")
            .trim_start_matches("EPUB/"),
    );
    let nav = template("EPUB/nav.xhtml")?
        .replace("{{CHAPTER_TOC_LIST}}", &toc_entries.join("\n"))
        .replace("{{LANDMARKS_LIST}}", &landmarks);
    write_bytes(&build_dir, "EPUB/nav.xhtml", nav.as_bytes())?;

    let titlepage = template("EPUB/titlepage.xhtml")?
        .replace("{{BOOK_TITLE}}", &book_title)
        .replace("{{BOOK_AUTHOR}}", &book_author)
        .replace("{{BOOK_DESCRIPTION_BLOCK}}", &description_block);
    write_bytes(&build_dir, "EPUB/titlepage.xhtml", titlepage.as_bytes())?;

    let cover = template("EPUB/cover.xhtml")?
        .replace("{{BOOK_TITLE}}", &book_title)
        .replace("{{COVER_BLOCK}}", &cover_block);
    write_bytes(&build_dir, "EPUB/cover.xhtml", cover.as_bytes())?;

    tree_in(project_dir).with_context(|| format!("failed to list {}", build_dir.display()))
}

/// 构建文件树：构建目录缺失回 `None`，前端展示空态引导构建。
pub fn tree_in(project_dir: &Path) -> Option<BuildOutcome> {
    let build_dir = project_dir.join(BUILD_DIR);
    if !build_dir.is_dir() {
        return None;
    }
    let files = list_dir(&build_dir, &build_dir).ok()?;
    let file_count = count_files(&files);
    Some(BuildOutcome { files, file_count })
}

/// 目录递归列举：目录在前、同类按名排序，路径为 POSIX 口径。
fn list_dir(build_dir: &Path, dir: &Path) -> anyhow::Result<Vec<BuildFileNode>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        let name = entry
            .file_name()
            .to_str()
            .context("non-utf8 file name")?
            .to_string();
        let rel = to_posix(
            entry
                .path()
                .strip_prefix(build_dir)
                .context("build path escapes build dir")?,
        );
        if file_type.is_dir() {
            dirs.push(BuildFileNode {
                name,
                path: rel,
                editable: false,
                children: Some(list_dir(build_dir, &entry.path())?),
            });
        } else if file_type.is_file() {
            files.push(BuildFileNode {
                name,
                path: rel.clone(),
                editable: is_editable_text(&rel),
                children: None,
            });
        }
    }
    dirs.sort_by(|a: &BuildFileNode, b: &BuildFileNode| a.name.cmp(&b.name));
    files.sort_by(|a: &BuildFileNode, b: &BuildFileNode| a.name.cmp(&b.name));
    dirs.extend(files);
    Ok(dirs)
}

/// 统计文件数（不计目录）。
fn count_files(nodes: &[BuildFileNode]) -> u32 {
    nodes
        .iter()
        .map(|node| {
            node.children
                .as_ref()
                .map_or(1, |children| count_files(children))
        })
        .sum()
}

/// 是否图片：预览白名单，与前端 `isImageFile` 同口径。
fn is_image(rel_posix: &str) -> bool {
    Path::new(rel_posix)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["jpg", "jpeg", "png", "webp", "gif"].contains(&ext.to_ascii_lowercase().as_str())
        })
}

/// 读构建文件：二进制回 `content: None`；固定文件标记不可编辑。
pub fn read_file_in(project_dir: &Path, rel: &str) -> anyhow::Result<BuildFileContent> {
    let build_dir = project_dir.join(BUILD_DIR);
    let path = resolve_build_path(&build_dir, rel)?;
    let bytes =
        std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let rel_posix = to_posix(
        path.strip_prefix(&build_dir)
            .context("build path escapes build dir")?,
    );
    Ok(BuildFileContent {
        content: String::from_utf8(bytes).ok(),
        editable: is_editable_text(&rel_posix),
        formattable: is_formattable(&rel_posix),
    })
}

/// 写构建文件：固定文件与二进制拒绝写入。
pub fn write_file_in(project_dir: &Path, rel: &str, text: &str) -> anyhow::Result<()> {
    let build_dir = project_dir.join(BUILD_DIR);
    let path = resolve_build_path(&build_dir, rel)?;
    let rel_posix = to_posix(
        path.strip_prefix(&build_dir)
            .context("build path escapes build dir")?,
    );
    if !is_editable_text(&rel_posix) {
        anyhow::bail!("build file is not editable: {rel}");
    }
    if path.is_file() {
        let existing =
            std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        if String::from_utf8(existing).is_err() {
            anyhow::bail!("build file is not editable: {rel}");
        }
    }
    std::fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// XML 行内元素：起止标签不换行（`li > a` 保持单行）；
/// 其余元素按块级换行缩进，行内文本原样内联，CJK 首尾不塞空格。
const XML_INLINE_TAGS: &[&str] = &["a"];

/// 上一个输出单元的种类：换行只发生在“标签后跟标签”时，
/// 正文与行内标签后保持同行，CJK 首尾不塞空格。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prev {
    /// 行首（刚换行或刚开工）。
    Fresh,
    /// 行尾落在块标签上。
    Tag,
    /// 行尾落在正文或行内标签上。
    Inline,
}

fn tag_name(bytes: &[u8]) -> anyhow::Result<&str> {
    std::str::from_utf8(bytes).context("non-utf8 tag name")
}

fn raw_text<'a>(bytes: &'a [u8], what: &str) -> anyhow::Result<&'a str> {
    std::str::from_utf8(bytes).with_context(|| format!("non-utf8 {what}"))
}

fn attrs_to_string(element: &BytesStart) -> anyhow::Result<String> {
    let mut out = String::new();
    for attr in element.attributes() {
        let attr = attr.context("invalid attribute")?;
        out.push(' ');
        out.push_str(raw_text(attr.key.as_ref(), "attr name")?);
        out.push_str("=\"");
        out.push_str(raw_text(&attr.value, "attr value")?);
        out.push('"');
    }
    Ok(out)
}

/// XML 美化器：事件流重排，块级按深度缩进、行内原样内联；
/// 纯空白文本丢弃（由结构换行替代），注释 / CDATA / 声明按原文切片保留。
struct XmlFormatter<'a> {
    reader: Reader<&'a [u8]>,
    text: &'a str,
    out: String,
    depth: usize,
    prev: Prev,
    just_opened: bool,
    /// 未闭合标签栈：文档结束时非空即报错（读器只校验配对，不查缺失）。
    stack: Vec<String>,
}

impl<'a> XmlFormatter<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            reader: Reader::from_str(text),
            text,
            out: String::with_capacity(text.len() + text.len() / 8),
            depth: 0,
            prev: Prev::Fresh,
            just_opened: false,
            stack: Vec::new(),
        }
    }

    /// 当前读位（字节偏移）：原文切片定位用。
    fn position(&self) -> anyhow::Result<usize> {
        usize::try_from(self.reader.buffer_position()).context("xml offset out of range")
    }

    /// 换行并按深度缩进（2 空格，与前端 Prettier 口径一致）。
    fn break_line(&mut self) {
        self.out.push('\n');
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    /// 原文切片：注释 / CDATA / 声明等按原文回填，不重组定界符。
    fn raw_slice(&self, start: usize) -> anyhow::Result<&'a str> {
        let end = self.position()?;
        let text = self.text;
        text.get(start..end).context("xml offset out of range")
    }

    /// 序言（声明 / 文档类型）：独占一行。
    fn emit_prolog(&mut self, start: usize) -> anyhow::Result<()> {
        if self.prev != Prev::Fresh {
            self.break_line();
        }
        let raw = self.raw_slice(start)?.to_string();
        self.out.push_str(&raw);
        self.out.push('\n');
        self.prev = Prev::Fresh;
        self.just_opened = false;
        Ok(())
    }

    /// 原样块（注释 / CDATA / 处理指令）：块标签后换行，否则内联。
    fn emit_verbatim(&mut self, start: usize) -> anyhow::Result<()> {
        if self.prev == Prev::Tag {
            self.break_line();
        }
        let raw = self.raw_slice(start)?.to_string();
        self.out.push_str(&raw);
        self.prev = Prev::Tag;
        self.just_opened = false;
        Ok(())
    }

    /// 起始标签：块级在“标签后”换行缩进并进栈加深，行内一律内联。
    fn emit_start(&mut self, element: &BytesStart) -> anyhow::Result<()> {
        let name = tag_name(element.name().as_ref())?.to_string();
        let inline = XML_INLINE_TAGS.contains(&name.as_str());
        if !inline && self.prev == Prev::Tag {
            self.break_line();
        }
        self.out.push('<');
        self.out.push_str(&name);
        self.out.push_str(&attrs_to_string(element)?);
        self.out.push('>');
        if !inline {
            self.depth += 1;
        }
        self.stack.push(name);
        self.prev = if inline { Prev::Inline } else { Prev::Tag };
        self.just_opened = true;
        Ok(())
    }

    /// 空元素：块标签后换行，行内（正文中的自闭合）保持同行。
    fn emit_empty(&mut self, element: &BytesStart) -> anyhow::Result<()> {
        let qualified = element.name();
        let name = tag_name(qualified.as_ref())?;
        if self.prev == Prev::Tag {
            self.break_line();
        }
        self.out.push('<');
        self.out.push_str(name);
        self.out.push_str(&attrs_to_string(element)?);
        self.out.push_str("/>");
        self.prev = Prev::Tag;
        self.just_opened = false;
        Ok(())
    }

    /// 结束标签：紧随起始标签或正文/行内后直接闭合，否则换行缩进。
    fn emit_end(&mut self, element: &BytesEnd) -> anyhow::Result<()> {
        let qualified = element.name();
        let name = tag_name(qualified.as_ref())?;
        let opened = self.stack.pop();
        if opened.as_deref() != Some(name) {
            anyhow::bail!("mismatched end tag: {name}");
        }
        let inline = XML_INLINE_TAGS.contains(&name);
        if !inline {
            self.depth = self.depth.saturating_sub(1);
            if !self.just_opened && self.prev == Prev::Tag {
                self.break_line();
            }
        }
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
        self.prev = if inline { Prev::Inline } else { Prev::Tag };
        self.just_opened = false;
        Ok(())
    }

    /// 正文：纯空白丢弃（由结构换行替代），其余原样内联（不 trim，不断行）。
    fn emit_text(&mut self, element: &BytesText) -> anyhow::Result<()> {
        let raw = raw_text(element.as_ref(), "text")?;
        if raw.trim().is_empty() {
            return Ok(());
        }
        self.out.push_str(raw);
        self.prev = Prev::Inline;
        self.just_opened = false;
        Ok(())
    }

    /// 实体引用：原样回填，视为正文延续。
    fn emit_ref(&mut self, element: &BytesRef) -> anyhow::Result<()> {
        self.out.push('&');
        self.out.push_str(raw_text(element, "entity ref")?);
        self.out.push(';');
        self.prev = Prev::Inline;
        self.just_opened = false;
        Ok(())
    }

    /// 主循环：读事件、分发、收尾断言（栈空 + 非空输出）。
    fn run(mut self) -> anyhow::Result<String> {
        loop {
            let start = self.position()?;
            match self.reader.read_event().context("invalid xml")? {
                Event::Eof => break,
                Event::Decl(_) | Event::DocType(_) => self.emit_prolog(start)?,
                Event::Comment(_) | Event::CData(_) | Event::PI(_) => {
                    self.emit_verbatim(start)?;
                }
                Event::Start(element) => self.emit_start(&element)?,
                Event::Empty(element) => self.emit_empty(&element)?,
                Event::End(element) => self.emit_end(&element)?,
                Event::Text(element) => self.emit_text(&element)?,
                Event::GeneralRef(element) => self.emit_ref(&element)?,
            }
        }
        if !self.stack.is_empty() {
            anyhow::bail!("unclosed tag: {}", self.stack.join(", "));
        }
        let mut formatted = self.out.trim_end().to_string();
        if formatted.is_empty() {
            anyhow::bail!("empty xml document");
        }
        formatted.push('\n');
        Ok(formatted)
    }
}

/// XML 美化入口：非法 XML 直接报错，调用方保留原文。
fn format_xml(text: &str) -> anyhow::Result<String> {
    XmlFormatter::new(text).run()
}

/// CSS 美化：解析后重打；失败直接报错，调用方保留原文。
fn format_css(text: &str) -> anyhow::Result<String> {
    use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

    // anyhow 的 context 要求 error 'static，解析错误先转成 owned 字符串。
    let sheet = StyleSheet::parse(text, ParserOptions::default())
        .map_err(|error| anyhow::anyhow!("invalid css: {error}"))?;
    let printed = sheet
        .to_css(PrinterOptions {
            minify: false,
            ..Default::default()
        })
        .context("failed to print css")?;
    let mut formatted = printed.code.trim_end().to_string();
    formatted.push('\n');
    Ok(formatted)
}

/// 按扩展名格式化文本：纯内存，`xhtml` / `opf` / `xml` 走 XML 美化，`css` 走 CSS 美化。
fn format_text_by_ext(rel: &str, text: &str) -> anyhow::Result<String> {
    let ext = Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "xhtml" | "opf" | "xml" => format_xml(text),
        "css" => format_css(text),
        _ => anyhow::bail!("unsupported file type: {rel}"),
    }
}

/// 格式化单个构建文件：只读内存，不落盘；`mimetype` 与二进制拒绝。
pub fn format_file_in(project_dir: &Path, rel: &str) -> anyhow::Result<String> {
    let build_dir = project_dir.join(BUILD_DIR);
    let path = resolve_build_path(&build_dir, rel)?;
    let rel_posix = to_posix(
        path.strip_prefix(&build_dir)
            .context("build path escapes build dir")?,
    );
    if !is_formattable(&rel_posix) {
        anyhow::bail!("build file is not formattable: {rel}");
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    format_text_by_ext(&rel_posix, &text)
}

/// 格式化给定文本：纯内存，不读盘不落盘；路径仅作穿越守卫与可格式化校验。
pub fn format_text_in(project_dir: &Path, rel: &str, text: &str) -> anyhow::Result<String> {
    let build_dir = project_dir.join(BUILD_DIR);
    let path = resolve_build_path(&build_dir, rel)?;
    let rel_posix = to_posix(
        path.strip_prefix(&build_dir)
            .context("build path escapes build dir")?,
    );
    if !is_formattable(&rel_posix) {
        anyhow::bail!("build file is not formattable: {rel}");
    }
    format_text_by_ext(&rel_posix, text)
}

/// 收集可格式化文件路径（文档顺序）：可编辑文件 + `container.xml`。
fn formattable_files(nodes: &[BuildFileNode], out: &mut Vec<String>) {
    for node in nodes {
        if let Some(children) = &node.children {
            formattable_files(children, out);
        } else if is_formattable(&node.path) {
            out.push(node.path.clone());
        }
    }
}

/// 收集跳过文件路径（`mimetype` 与二进制）。
fn skipped_files(nodes: &[BuildFileNode], out: &mut Vec<String>) {
    for node in nodes {
        if let Some(children) = &node.children {
            skipped_files(children, out);
        } else if !is_formattable(&node.path) {
            out.push(node.path.clone());
        }
    }
}

/// 批量格式化：逐个格式化可格式化文件并落盘（内容无变化则跳写）；
/// 单个失败不中断，其余继续，结果分别计数。
pub fn format_all_in(project_dir: &Path) -> anyhow::Result<FormatOutcome> {
    let build_dir = project_dir.join(BUILD_DIR);
    let tree = tree_in(project_dir).context("no build output to format")?;
    let mut targets = Vec::new();
    formattable_files(&tree.files, &mut targets);
    let mut formatted = 0u32;
    let mut failed = Vec::new();
    for rel in &targets {
        match format_file_in(project_dir, rel) {
            Ok(next) => {
                let path = resolve_build_path(&build_dir, rel)?;
                let current = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                if current != next {
                    std::fs::write(&path, next)
                        .with_context(|| format!("failed to write {}", path.display()))?;
                }
                formatted += 1;
            }
            Err(_) => failed.push(rel.clone()),
        }
    }
    let mut skipped = Vec::new();
    skipped_files(&tree.files, &mut skipped);
    Ok(FormatOutcome {
        formatted,
        skipped,
        failed,
    })
}

/// 格式化编辑器文本（`AppHandle` 入口）：纯内存，不读盘不落盘。
pub fn format_build_text(
    app: &tauri::AppHandle,
    id: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<String> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    format_text_in(&dir, path, content)
}

/// 批量格式化（`AppHandle` 入口）：落盘，单个失败不中断。
pub fn format_all_build_files(app: &tauri::AppHandle, id: &str) -> anyhow::Result<FormatOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    format_all_in(&dir)
}

/// 构建小说（`AppHandle` 入口）：清空后全量重生成，返回文件树。
pub fn build_novel(
    app: &tauri::AppHandle,
    id: &str,
    titles: &super::novel_export::BuildTitleOptions,
) -> anyhow::Result<BuildOutcome> {
    use super::epub_template::template_dir;
    let dir = super::projects::resolve_project_dir(app, id)?;
    let base = template_dir(app, ProjectType::Novel)?;
    build_in(&dir, &base, titles)
}

/// 清空构建产物：删除 `build/` 目录（不存在视为成功），供确认重构建后回到引导。
pub fn clear_in(project_dir: &Path) -> anyhow::Result<()> {
    let build_dir = project_dir.join(BUILD_DIR);
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .with_context(|| format!("failed to clear {}", build_dir.display()))?;
    }
    Ok(())
}

/// 清空构建产物（`AppHandle` 入口）。
pub fn clear_build(app: &tauri::AppHandle, id: &str) -> anyhow::Result<()> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    clear_in(&dir)
}

/// 读构建树（`AppHandle` 入口）：未构建回 `None`。
pub fn build_tree(app: &tauri::AppHandle, id: &str) -> anyhow::Result<Option<BuildOutcome>> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    Ok(tree_in(&dir))
}

/// 构建图片信息：绝对路径供前端转 `asset:`，大小供标题栏展示；非图片或文件缺失直接报错。
pub fn image_path_in(project_dir: &Path, rel: &str) -> anyhow::Result<BuildImageInfo> {
    let build_dir = project_dir.join(BUILD_DIR);
    let path = resolve_build_path(&build_dir, rel)?;
    let meta =
        std::fs::metadata(&path).with_context(|| format!("failed to stat {}", path.display()))?;
    if !meta.is_file() {
        anyhow::bail!("build file not found: {rel}");
    }
    let rel_posix = to_posix(
        path.strip_prefix(&build_dir)
            .context("build path escapes build dir")?,
    );
    if !is_image(&rel_posix) {
        anyhow::bail!("build file is not an image: {rel}");
    }
    Ok(BuildImageInfo {
        path: path
            .to_str()
            .context("build path is not valid UTF-8")?
            .to_string(),
        size_bytes: u32::try_from(meta.len()).unwrap_or(u32::MAX),
    })
}

/// 读构建文件（`AppHandle` 入口）。
pub fn build_file(
    app: &tauri::AppHandle,
    id: &str,
    path: &str,
) -> anyhow::Result<BuildFileContent> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    read_file_in(&dir, path)
}

/// 构建图片信息（`AppHandle` 入口）。
pub fn build_image_path(
    app: &tauri::AppHandle,
    id: &str,
    path: &str,
) -> anyhow::Result<BuildImageInfo> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    image_path_in(&dir, path)
}

/// 写构建文件（`AppHandle` 入口）：固定与二进制拒绝。
/// 写构建文件（`AppHandle` 入口）：固定与二进制拒绝。
pub fn write_build_file(
    app: &tauri::AppHandle,
    id: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<()> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    write_file_in(&dir, path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::novel_export::BuildTitleOptions;
    use super::super::novel_export::ChapterNumbering;
    use super::super::novel_export::TxtTitleKind;
    use super::super::novel_files;
    use super::super::projects;
    use super::super::projects::ProjectType;
    use super::*;

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
            "ebook-studio-novel-build-test-{}-{case}",
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

    fn read_built(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join(BUILD_DIR).join(rel)).expect("read built file")
    }

    #[test]
    fn builds_volumes_chapters_and_manifest() {
        let (root, dir) = fixture_project(
            "full",
            "第一卷 开篇\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n第二卷 承接\n第一章 再起\n正文丙\n",
        );
        let outcome =
            build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        assert!(outcome.file_count >= 10);

        // 章节页：标题转义 + 正文分段（分卷命名 chapter-<卷>-<章>）。
        let chapter = read_built(&dir, "EPUB/text/chapter-01-01.xhtml");
        assert!(chapter.contains("<h1 class=\"chapter-title\">第一章 引子</h1>"));
        assert!(chapter.contains("<p>正文甲</p>"));
        // 卷页：标题 + 简介（本例简介为空，卷页仍生成）。
        let volume = read_built(&dir, "EPUB/text/volume-01.xhtml");
        assert!(volume.contains("<h1 class=\"volume-title\">第一卷 开篇</h1>"));
        // 目录嵌套 + 导览；opf 清单与书脊按文档顺序。
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains("text/chapter-01-02.xhtml"));
        assert!(nav.contains("bodymatter"));
        // 地标语义用官方词表无连字符形（`titlepage`），连字符形会吃 `OPF-088`。
        assert!(nav.contains("epub:type=\"titlepage\""));
        assert!(!nav.contains("title-page"));
        let opf = read_built(&dir, "EPUB/content.opf");
        let manifest_pos = opf.find("chapter-01-01").expect("manifest");
        let spine_pos = opf.find("<spine>").expect("spine");
        assert!(manifest_pos < spine_pos);
        assert!(opf.contains("<itemref idref=\"volume-02\"/>"));
        assert!(opf.contains("<itemref idref=\"chapter-02-01\"/>"));
        // 生成文本无残留占位符。
        for rel in [
            "EPUB/content.opf",
            "EPUB/nav.xhtml",
            "EPUB/titlepage.xhtml",
            "EPUB/cover.xhtml",
            "EPUB/text/chapter-01-01.xhtml",
            "EPUB/text/volume-01.xhtml",
        ] {
            assert!(
                !read_built(&dir, rel).contains("{{"),
                "placeholder left in {rel}"
            );
        }
        // 空作者与空贡献者整组删除（空元素通不过 OPF 模式校验）。
        assert!(!opf.contains("creator"));
        assert!(!opf.contains("contributor"));
        cleanup(&root);
    }

    #[test]
    fn builds_volume_chapters_with_title_kinds() {
        let (root, dir) = fixture_project(
            "titled",
            "第一卷 开篇\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n",
        );
        let options = BuildTitleOptions {
            chapter_kind: TxtTitleKind::Padded,
            chapter_pattern: String::new(),
            volume_kind: TxtTitleKind::Arabic,
            volume_pattern: String::new(),
            chapter_numbering: ChapterNumbering::PerVolume,
        };
        build_in(&dir, &template_base(), &options).expect("build");
        let chapter = read_built(&dir, "EPUB/text/chapter-01-01.xhtml");
        assert!(chapter.contains("<h1 class=\"chapter-title\">第01章 引子</h1>"));
        let volume = read_built(&dir, "EPUB/text/volume-01.xhtml");
        assert!(volume.contains("<h1 class=\"volume-title\">第1卷 开篇</h1>"));
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains("第1卷 开篇"));
        assert!(nav.contains("第01章 引子"));
        cleanup(&root);
    }

    #[test]
    fn builds_volume_chapters_with_continuous_numbering() {
        let (root, dir) = fixture_project(
            "continuous",
            "第一卷 开篇\n第一章 起\n甲\n第二卷 续篇\n第一章 再起\n乙\n",
        );
        let options = BuildTitleOptions {
            chapter_kind: TxtTitleKind::Padded,
            chapter_pattern: String::new(),
            volume_kind: TxtTitleKind::Original,
            volume_pattern: String::new(),
            chapter_numbering: ChapterNumbering::Continuous,
        };
        build_in(&dir, &template_base(), &options).expect("build");
        // 文件名章号跟随编号方式：卷 2 首章是全书第 2 章。
        let chapter = read_built(&dir, "EPUB/text/chapter-02-02.xhtml");
        assert!(chapter.contains("<h1 class=\"chapter-title\">第02章 再起</h1>"));
        cleanup(&root);
    }

    #[test]
    fn rejects_empty_custom_title_patterns() {
        let (root, dir) = fixture_project("custom-bad", "第一章 引子\n正文\n");
        let options = BuildTitleOptions {
            chapter_kind: TxtTitleKind::Custom,
            chapter_pattern: "  ".to_string(),
            ..BuildTitleOptions::default()
        };
        assert!(build_in(&dir, &template_base(), &options).is_err());
        let options = BuildTitleOptions {
            volume_kind: TxtTitleKind::Custom,
            volume_pattern: "  ".to_string(),
            ..BuildTitleOptions::default()
        };
        assert!(build_in(&dir, &template_base(), &options).is_err());
        cleanup(&root);
    }

    #[test]
    fn escapes_xml_special_chars() {
        let (root, dir) = fixture_project("escape", "第一章 A&B<尾>\n正文含\"引号\"与<标签>\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let chapter = read_built(&dir, "EPUB/text/chapter-001.xhtml");
        assert!(chapter.contains("A&amp;B&lt;尾&gt;"));
        assert!(chapter.contains("正文含&quot;引号&quot;与&lt;标签&gt;"));
        cleanup(&root);
    }

    #[test]
    fn clears_build_output_back_to_guide() {
        let (root, dir) = fixture_project("clear", "第一章 引子\n正文\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        assert!(tree_in(&dir).is_some());
        clear_in(&dir).expect("clear");
        assert!(tree_in(&dir).is_none());
        assert!(!dir.join(BUILD_DIR).exists());
        // 缺失时重复清空仍成功。
        clear_in(&dir).expect("clear again");
        cleanup(&root);
    }

    #[test]
    fn untitled_volume_skips_volume_page_and_flattens_toc() {
        let (root, dir) = fixture_project("flat", "第一章 引子\n正文\n第二章 发展\n正文\n");
        let outcome =
            build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let paths: Vec<String> = outcome.files.iter().flat_map(flatten).collect();
        assert!(!paths.iter().any(|p| p.contains("volume-")));
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(!nav.contains("<ol>\n          <ol>"));
        cleanup(&root);
    }

    /// 收集树中全部文件路径。
    fn flatten(node: &BuildFileNode) -> Vec<String> {
        node.children.as_ref().map_or_else(
            || vec![node.path.clone()],
            |children| children.iter().flat_map(flatten).collect(),
        )
    }

    #[test]
    fn tree_round_trips_and_rejects_fixed_and_traversal_writes() {
        let (root, dir) = fixture_project("tree", "第一章 引子\n正文\n");
        assert!(tree_in(&dir).is_none());
        let outcome =
            build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        assert_eq!(tree_in(&dir), Some(outcome));

        // 固定文件只读：树中标记 + 写入拒绝。
        let mimetype = read_file_in(&dir, "mimetype").expect("read");
        assert_eq!(mimetype.content.as_deref(), Some("application/epub+zip"));
        assert!(!mimetype.editable);
        assert!(write_file_in(&dir, "mimetype", "x").is_err());
        assert!(write_file_in(&dir, "META-INF/container.xml", "x").is_err());

        // 生成文件可读写回。
        let opf = read_file_in(&dir, "EPUB/content.opf").expect("read");
        assert!(opf.editable);
        assert!(opf.content.is_some());
        write_file_in(&dir, "EPUB/content.opf", "<edited/>").expect("write");
        assert_eq!(
            read_file_in(&dir, "EPUB/content.opf")
                .expect("reread")
                .content
                .as_deref(),
            Some("<edited/>")
        );

        // 路径穿越与空路径拒绝。
        assert!(read_file_in(&dir, "../project.json").is_err());
        assert!(read_file_in(&dir, "/abs").is_err());
        assert!(read_file_in(&dir, "").is_err());
        assert!(write_file_in(&dir, "EPUB/../../x", "x").is_err());
        cleanup(&root);
    }

    #[test]
    fn copies_cover_image_and_links_manifest() {
        let (root, dir) = fixture_project("cover", "第一章 引子\n正文\n");
        let source = root.join("cover-source.jpg");
        std::fs::write(&source, [0xFF, 0xD8, 0xFF, 0x00]).expect("write fake cover");
        super::super::project_metadata::import_cover_in(&dir, &source).expect("import cover");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        assert!(dir.join(BUILD_DIR).join("EPUB/images/cover.jpg").is_file());
        let opf = read_built(&dir, "EPUB/content.opf");
        assert!(opf.contains("cover-image"));
        assert!(opf.contains("image/jpeg"));
        // 清单 `href` 相对 OPF 所在目录（`EPUB/`），包内写盘前缀须剥掉。
        assert!(opf.contains("href=\"images/cover.jpg\""));
        assert!(!opf.contains("EPUB/images/cover.jpg"));
        let cover = read_built(&dir, "EPUB/cover.xhtml");
        assert!(cover.contains("images/cover.jpg"));
        // 封面图片按二进制处理：内容空 + 不可写。
        let image = read_file_in(&dir, "EPUB/images/cover.jpg").expect("read");
        assert_eq!(image.content, None);
        assert!(!image.editable);
        assert!(write_file_in(&dir, "EPUB/images/cover.jpg", "x").is_err());
        // 图片预览信息：图片可取，非图片、缺失与穿越拒绝。
        let image = image_path_in(&dir, "EPUB/images/cover.jpg").expect("image info");
        assert!(
            Path::new(&image.path).ends_with(Path::new("EPUB").join("images").join("cover.jpg"))
        );
        assert!(image.size_bytes > 0);
        assert!(image_path_in(&dir, "EPUB/content.opf").is_err());
        assert!(image_path_in(&dir, "EPUB/missing.png").is_err());
        assert!(image_path_in(&dir, "../project.json").is_err());
        cleanup(&root);
    }

    #[test]
    fn formats_chapter_pretty_and_idempotent() {
        let (root, dir) = fixture_project("format-xml", "第一卷 开篇\n第一章 引子\n正文甲\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        // 单卷有标题按分卷口径命名。
        let first = format_file_in(&dir, "EPUB/text/chapter-01-01.xhtml").expect("format");
        assert!(first.contains("<p>正文甲</p>"));
        assert!(first.ends_with(">\n") && !first.ends_with("\n\n"));
        assert!(!first.contains("{{"));
        // 幂等：格式化两次结果一致。
        std::fs::write(
            dir.join(BUILD_DIR).join("EPUB/text/chapter-01-01.xhtml"),
            &first,
        )
        .expect("write");
        let second = format_file_in(&dir, "EPUB/text/chapter-01-01.xhtml").expect("reformat");
        assert_eq!(first, second);
        cleanup(&root);
    }

    #[test]
    fn formats_inline_markup_without_extra_whitespace() {
        // 行内元素不断行：CJK 首尾不塞空格。
        let (root, dir) = fixture_project("format-inline", "第一章 引子\n正文\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let nav = read_built(&dir, "EPUB/nav.xhtml");
        assert!(nav.contains("<li><a href=\"cover.xhtml\">封面</a></li>"));
        let out = format_xml(&nav).expect("format nav");
        assert!(out.contains("<li><a href=\"cover.xhtml\">封面</a></li>"));
        // TOC 章节行同样保持单行。
        assert!(out.contains("<li><a href=\"text/chapter-001.xhtml\">第一章 引子</a></li>"));
        cleanup(&root);
    }

    #[test]
    fn formats_given_text_without_touching_disk() {
        let (root, dir) = fixture_project("format-text", "第一章 引子\n正文\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let rel = "EPUB/text/chapter-001.xhtml";
        let before = std::fs::read_to_string(dir.join(BUILD_DIR).join(rel)).expect("read disk");
        // 输入与落盘不同：输出跟输入走，证明不读盘；落盘内容保持不动。
        let out = format_text_in(&dir, rel, "<p>改</p>").expect("format text");
        assert!(out.contains("<p>改</p>"));
        assert!(!out.contains("正文"));
        assert_eq!(
            std::fs::read_to_string(dir.join(BUILD_DIR).join(rel)).expect("reread"),
            before
        );
        // 非法路径、不可格式化与坏输入拒绝。
        assert!(format_text_in(&dir, "../project.json", "<p>x</p>").is_err());
        assert!(format_text_in(&dir, "mimetype", "x").is_err());
        assert!(format_text_in(&dir, rel, "<p>unclosed").is_err());
        cleanup(&root);
    }

    #[test]
    fn formats_css_and_rejects_broken_input() {
        let (root, dir) = fixture_project("format-css", "第一章 引子\n正文\n");
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let out = format_file_in(&dir, "EPUB/styles/base.css").expect("format css");
        assert!(out.contains(".cover-frame"));
        // 非法输入直接报错，调用方保留原文。
        assert!(format_xml("<p>unclosed").is_err());
        assert!(format_xml("").is_err());
        // lightningcss 对缺省值/未闭合括号很宽容（自动修复），只断言真正非法的输入报错。
        assert!(format_css("{{{").is_err());
        // `mimetype` 与二进制拒绝格式化；只读的 `container.xml` 允许内存格式化。
        assert!(format_file_in(&dir, "mimetype").is_err());
        let container = format_file_in(&dir, "META-INF/container.xml").expect("format container");
        assert!(container.contains("EPUB/content.opf"));
        assert!(container.ends_with(">\n"));
        // 写入仍拒绝：只读不因可格式化而放行。
        assert!(write_file_in(&dir, "META-INF/container.xml", "x").is_err());
        cleanup(&root);
    }

    #[test]
    fn formats_all_files_with_counts() {
        let (root, dir) = fixture_project(
            "format-all",
            "第一卷 开篇\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n",
        );
        build_in(&dir, &template_base(), &BuildTitleOptions::default()).expect("build");
        let outcome = format_all_in(&dir).expect("format all");
        assert!(outcome.formatted > 0);
        assert!(outcome.failed.is_empty(), "unexpected format failures");
        assert!(outcome.skipped.contains(&"mimetype".to_string()));
        assert!(
            !outcome
                .skipped
                .contains(&"META-INF/container.xml".to_string())
        );
        // 再次执行内容不变：幂等。
        let again = format_all_in(&dir).expect("format all again");
        assert_eq!(again.formatted, outcome.formatted);
        cleanup(&root);
    }
}
