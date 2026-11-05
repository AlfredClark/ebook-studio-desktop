//! 小说导出：`content.json` → 标准 TXT、`dist/*.epub` 与封面的改名拷贝，
//! 落到用户自选目录下的 `<命名>/` 子目录。
//!
//! 目标目录由前端目录选择框给出（后端只认存在目录）；目标子目录已存在且非空
//! 直接报错，不动用户旧文件。落盘格式即前端契约（`snake_case`）。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use super::novel_files::NovelContent;

/// TXT 标题格式：原文之外的五种全部由模板引擎渲染。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TxtTitleKind {
    /// 原文格式：完整标题行原样。
    Original,
    /// 阿拉伯数字。
    Arabic,
    /// 数字补零（宽度按全书同类最大位数，最低 2 位）。
    Padded,
    /// 中文小写数字。
    ChineseLower,
    /// 中文大写数字。
    ChineseUpper,
    /// 自定义模板（`#n #N #c #C #T`，`##` 转义）。
    Custom,
}

/// 元数据字段：导出勾选源；渲染顺序固定为著录序，与选择顺序无关。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MetadataField {
    /// 书名。
    Title,
    /// 副标题。
    Alternative,
    /// 作者。
    Creator,
    /// 贡献者。
    Contributor,
    /// 语言。
    Language,
    /// 出版社。
    Publisher,
    /// 系列名。
    Series,
    /// 卷次。
    Volume,
    /// 出版日期。
    DatePublished,
    /// 版权。
    Rights,
    /// 来源。
    Source,
    /// 主题词（多值，`、` 连接）。
    Subjects,
    /// 简介（多行，`简介：` 独占首行）。
    Description,
}

/// TXT 导出选项：标题格式 + 元数据清单；`items.txt` 为假时忽略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TxtOptions {
    /// 标题格式。
    pub title_kind: TxtTitleKind,
    /// 自定义模板；非 `custom` 时忽略。
    pub title_pattern: String,
    /// 勾选的元数据字段。
    pub metadata: Vec<MetadataField>,
}

impl Default for TxtOptions {
    fn default() -> Self {
        Self {
            title_kind: TxtTitleKind::Original,
            title_pattern: String::new(),
            metadata: Vec::new(),
        }
    }
}

/// 解析产物摘要：存在性之外的展示信息。
///
/// 字段名与 `ParseOutline` 同口径（`struct_field_names` 豁免）。
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ContentSummary {
    /// 卷数（含无标题卷）。
    pub volume_count: u32,
    /// 全书章节总数。
    pub chapter_count: u32,
    /// 正文字数：仅各章 `lines` 字符数之和（不计换行符）。
    pub char_count: u32,
}

/// 导出状态：进页一次查全，前端据此渲染表单与禁用态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportStatus {
    /// 是否有可导出的解析产物。
    pub has_content: bool,
    /// 解析产物摘要；无产物时为 `None`。
    pub content: Option<ContentSummary>,
    /// 是否有可导出的打包产物。
    pub has_epub: bool,
    /// 打包文件名；无产物时为 `None`。
    pub epub_name: Option<String>,
    /// 是否有可导出的封面。
    pub has_cover: bool,
    /// 封面扩展名（小写）；无封面时为 `None`。
    pub cover_ext: Option<String>,
    /// 项目名（`project.json`）。
    pub project_name: String,
    /// 书名（`metadata.json`）。
    pub book_title: String,
}

/// 命名来源：项目名 / 书名 / 自定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ExportNaming {
    /// 用项目名。
    Project,
    /// 用书名。
    Book,
    /// 用用户填写的自定义名。
    Custom,
}

/// 导出勾选项：全假直接报错，至少选一项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportItems {
    /// 导出 EPUB。
    pub epub: bool,
    /// 导出标准 TXT。
    pub txt: bool,
    /// 导出封面。
    pub cover: bool,
}

/// 导出结果：最终目录绝对路径 + 落盘文件名清单。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExportOutcome {
    /// 最终目录（`<自选目录>/<命名>/` 绝对路径）。
    pub dir: String,
    /// 落盘文件名（已排序）。
    pub files: Vec<String>,
}

/// 解析产物转摘要。
fn summarize(content: &NovelContent) -> ContentSummary {
    let chapter_count: usize = content.volumes.iter().map(|v| v.chapters.len()).sum();
    let char_count: usize = content
        .volumes
        .iter()
        .flat_map(|v| v.chapters.iter())
        .flat_map(|c| c.lines.iter())
        .map(|line| line.chars().count())
        .sum();
    ContentSummary {
        volume_count: u32::try_from(content.volumes.len()).unwrap_or(u32::MAX),
        chapter_count: u32::try_from(chapter_count).unwrap_or(u32::MAX),
        char_count: u32::try_from(char_count).unwrap_or(u32::MAX),
    }
}

/// 阿拉伯数字转中文数字：`upper` 为假用小写（一二…十百千万），为真用大写
///（壹贰…拾佰仟萬億）；零位合并、尾零舍去、十位省一（如 `十` 而非 `一十`）。
fn format_chinese(value: u32, upper: bool) -> String {
    const LOWER_DIGITS: &[char] = &['零', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    const UPPER_DIGITS: &[char] = &['零', '壹', '贰', '叁', '肆', '伍', '陆', '柒', '捌', '玖'];
    const LOWER_GROUP_UNITS: &[char] = &['千', '百', '十'];
    const UPPER_GROUP_UNITS: &[char] = &['仟', '佰', '拾'];
    const LOWER_SECTIONS: &[&str] = &["", "万", "亿", "兆"];
    const UPPER_SECTIONS: &[&str] = &["", "萬", "億", "兆"];

    /// 四位节内渲染（`1..=9999`，不含节单位）。
    fn format_group(value: u16, digits: &[char], units: &[char]) -> String {
        let places = [
            (value / 1000, units[0]),
            ((value / 100) % 10, units[1]),
            ((value / 10) % 10, units[2]),
        ];
        let mut text = String::new();
        let mut started = false;
        let mut zero_pending = false;
        for (digit, unit) in places {
            if digit == 0 {
                if started {
                    zero_pending = true;
                }
            } else {
                if zero_pending {
                    text.push(digits[0]);
                    zero_pending = false;
                }
                // 十位省一：节内首个非零位恰为十时省略 `一`。
                if !(digit == 1 && !started && (unit == '十' || unit == '拾')) {
                    text.push(digits[digit as usize]);
                }
                text.push(unit);
                started = true;
            }
        }
        let ones = value % 10;
        if ones != 0 {
            if zero_pending {
                text.push(digits[0]);
            }
            text.push(digits[ones as usize]);
        }
        text
    }

    if value == 0 {
        return LOWER_DIGITS[0].to_string();
    }
    let (digits, group_units, sections) = if upper {
        (UPPER_DIGITS, UPPER_GROUP_UNITS, UPPER_SECTIONS)
    } else {
        (LOWER_DIGITS, LOWER_GROUP_UNITS, LOWER_SECTIONS)
    };
    // 小端四位分节。
    let mut rest = value;
    let mut groups = Vec::new();
    while rest > 0 {
        groups.push((rest % 10000) as u16);
        rest /= 10000;
    }
    let mut out = String::new();
    let mut zero_pending = false;
    for (index, group) in groups.iter().enumerate().rev() {
        if *group == 0 {
            zero_pending = true;
            continue;
        }
        // 低节不足千位且高位已出时补单零（如 `一万零一百`）。
        if zero_pending || (!out.is_empty() && *group < 1000) {
            out.push(digits[0]);
        }
        zero_pending = false;
        out.push_str(&format_group(*group, digits, group_units));
        out.push_str(sections.get(index).copied().unwrap_or(""));
    }
    // 整体十位省一（如 `十万` 而非 `一十万`）。
    for prefix in ["一十", "壹拾"] {
        if let Some(rest) = out.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    out
}

/// 自定义模板渲染：`#n` 阿拉伯数字、`#N` 补零、`#c` 中文小写、`#C` 中文大写、
/// `#T` 纯标题；`##` 转义为字面 `#`，未知占位符与孤 `#` 原样保留；
/// 编号缺失时数字占位符原文保留，不做任何回落。
fn render_title(pattern: &str, number: Option<u32>, width: usize, title: &str) -> String {
    let mut out = String::new();
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch != '#' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            None | Some('#') => out.push('#'),
            Some('T') => out.push_str(title),
            Some(kind @ ('n' | 'N' | 'c' | 'C')) => match number {
                None => {
                    out.push('#');
                    out.push(kind);
                }
                Some(n) => match kind {
                    'n' => out.push_str(&n.to_string()),
                    // `String` 落盘永不失败，返回值忽略。
                    'N' => {
                        let _ = write!(out, "{n:0>width$}");
                    }
                    'c' => out.push_str(&format_chinese(n, false)),
                    _ => out.push_str(&format_chinese(n, true)),
                },
            },
            // 未知占位符整体原样保留。
            Some(other) => {
                out.push('#');
                out.push(other);
            }
        }
    }
    out
}

/// 元数据著录序：书名 → 作者 → 语言 → 系列 → 卷次 → 版权 → 来源 →
/// 副标题 → 主题词 → 贡献者 → 出版社 → 出版日期 → 简介；渲染与前端候选一致。
const METADATA_ORDER: &[MetadataField] = &[
    MetadataField::Title,
    MetadataField::Creator,
    MetadataField::Language,
    MetadataField::Series,
    MetadataField::Volume,
    MetadataField::Rights,
    MetadataField::Source,
    MetadataField::Alternative,
    MetadataField::Subjects,
    MetadataField::Contributor,
    MetadataField::Publisher,
    MetadataField::DatePublished,
    MetadataField::Description,
];

/// 元数据块：勾选字段按著录序每行 `<属性>：<内容>`；空值跳过；
/// 主题词用 `、` 连接；简介为 `简介：` 独占首行后跟多行。
/// 无可写行回 `None`，调用方不输出块与分隔符。
fn metadata_block(
    metadata: &super::project_metadata::ProjectMetadata,
    fields: &[MetadataField],
) -> Option<String> {
    /// 非空文本字段取用。
    fn text(value: &str) -> Option<&str> {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    }
    /// 单行字段落盘：空值跳过。
    fn push(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
        if let Some(content) = value {
            lines.push(format!("{label}：{content}"));
        }
    }
    let mut lines: Vec<String> = Vec::new();
    // 著录序固定，与勾选顺序无关。
    for field in METADATA_ORDER {
        if !fields.contains(field) {
            continue;
        }
        match field {
            MetadataField::Title => push(&mut lines, "书名", text(&metadata.title)),
            MetadataField::Alternative => push(&mut lines, "副标题", text(&metadata.alternative)),
            MetadataField::Creator => push(&mut lines, "作者", text(&metadata.creator)),
            MetadataField::Contributor => {
                push(&mut lines, "贡献者", text(&metadata.contributor));
            }
            MetadataField::Language => push(&mut lines, "语言", text(&metadata.language)),
            MetadataField::Publisher => push(&mut lines, "出版社", text(&metadata.publisher)),
            MetadataField::Series => push(&mut lines, "系列", text(&metadata.series)),
            MetadataField::Volume => push(&mut lines, "卷次", text(&metadata.volume)),
            MetadataField::DatePublished => {
                push(&mut lines, "出版日期", metadata.date_published.as_deref());
            }
            MetadataField::Rights => push(&mut lines, "版权", text(&metadata.rights)),
            MetadataField::Source => push(&mut lines, "来源", text(&metadata.source)),
            MetadataField::Subjects => {
                let subjects: Vec<&str> = metadata
                    .subjects
                    .iter()
                    .map(String::as_str)
                    .filter(|subject| !subject.trim().is_empty())
                    .collect();
                if !subjects.is_empty() {
                    lines.push(format!("主题词：{}", subjects.join("、")));
                }
            }
            MetadataField::Description => {
                let intro: Vec<&str> = metadata
                    .description
                    .iter()
                    .map(String::as_str)
                    .filter(|line| !line.trim().is_empty())
                    .collect();
                if !intro.is_empty() {
                    lines.push(format!("简介：\n{}", intro.join("\n")));
                }
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// 卷章标题行：原文格式用完整标题行；内置格式统一为 `第<编号>卷/章 <标题>`
///（卷行用 `卷`、章行用 `章`）；自定义卷章共用同一模板；缺号留位。
fn heading(
    kind: TxtTitleKind,
    pattern: &str,
    full: &str,
    title: &str,
    number: Option<u32>,
    width: usize,
    is_volume: bool,
) -> String {
    match (kind, is_volume) {
        (TxtTitleKind::Original, _) => full.to_string(),
        (TxtTitleKind::Arabic, false) => render_title("第#n章 #T", number, width, title),
        (TxtTitleKind::Arabic, true) => render_title("第#n卷 #T", number, width, title),
        (TxtTitleKind::Padded, false) => render_title("第#N章 #T", number, width, title),
        (TxtTitleKind::Padded, true) => render_title("第#N卷 #T", number, width, title),
        (TxtTitleKind::ChineseLower, false) => render_title("第#c章 #T", number, width, title),
        (TxtTitleKind::ChineseLower, true) => render_title("第#c卷 #T", number, width, title),
        (TxtTitleKind::ChineseUpper, false) => render_title("第#C章 #T", number, width, title),
        (TxtTitleKind::ChineseUpper, true) => render_title("第#C卷 #T", number, width, title),
        (TxtTitleKind::Custom, _) => render_title(pattern, number, width, title),
    }
}

/// 分隔线：32 个 `―` 独占一行。
const SEPARATOR_LINE: &str = "――――――――――――――――――――――――――――――――";

/// 同类编号最大位数（补零宽度），最低 2 位；全缺号时宽度无意义，回落 2。
fn number_width(numbers: impl Iterator<Item = Option<u32>>) -> usize {
    numbers
        .flatten()
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2)
}

/// 解析产物转标准 TXT：卷标题 / 卷简介 / 章标题各独行，章间空一行；前言不导出。
///
/// 标题行按 `options.title_kind` 渲染；勾选元数据时前置元数据块，块与正文之间
/// 以分隔线独占一行、前后各 1 空行分隔。换行一律 `LF`、无 `BOM`；
/// 隐式卷跳过卷标题行；正文与元数据皆无可写行直接报错。
fn build_txt(
    content: &NovelContent,
    metadata: &super::project_metadata::ProjectMetadata,
    options: &TxtOptions,
) -> anyhow::Result<String> {
    if matches!(options.title_kind, TxtTitleKind::Custom) && options.title_pattern.trim().is_empty()
    {
        anyhow::bail!("custom title pattern must not be empty");
    }
    let chapter_width = number_width(
        content
            .volumes
            .iter()
            .flat_map(|volume| volume.chapters.iter())
            .map(|chapter| chapter.number),
    );
    let volume_width = number_width(content.volumes.iter().map(|volume| volume.number));
    let mut blocks: Vec<String> = Vec::new();
    for volume in &content.volumes {
        let mut block = String::new();
        if let Some(full_title) = volume.full_title.as_deref() {
            block.push_str(&heading(
                options.title_kind,
                &options.title_pattern,
                full_title,
                volume.title.as_deref().unwrap_or(""),
                volume.number,
                volume_width,
                true,
            ));
            block.push('\n');
        }
        for line in &volume.intro {
            block.push_str(line);
            block.push('\n');
        }
        for chapter in &volume.chapters {
            block.push_str(&heading(
                options.title_kind,
                &options.title_pattern,
                &chapter.full_title,
                &chapter.title,
                chapter.number,
                chapter_width,
                false,
            ));
            block.push('\n');
            for line in &chapter.lines {
                block.push_str(line);
                block.push('\n');
            }
            block.push('\n');
        }
        let block = block.trim_end().to_string();
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    let meta = metadata_block(metadata, &options.metadata);
    if blocks.is_empty() && meta.is_none() {
        anyhow::bail!("no exportable text in content");
    }
    // 有元数据无正文时只出元数据块。
    let body = blocks.join("\n\n");
    let mut text = match meta {
        None => body,
        Some(meta) if body.is_empty() => meta,
        Some(meta) => format!("{meta}\n\n{SEPARATOR_LINE}\n\n{body}"),
    };
    text.push('\n');
    Ok(text)
}

/// 封面扩展名（小写）：导入白名单的子集，未命中直接报错。
fn cover_ext_of(path: &Path) -> anyhow::Result<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .context("cover file has no extension")?;
    if !["jpg", "jpeg", "png", "webp", "gif"].contains(&ext.as_str()) {
        anyhow::bail!("unsupported cover type: {ext}");
    }
    Ok(ext)
}

/// 标题格式预览：取首章按给定格式渲染一行，供前端下拉后方展示效果；
/// 无章节直接报错，调用方隐藏预览。
pub fn preview_title_in(
    project_dir: &Path,
    kind: TxtTitleKind,
    pattern: &str,
) -> anyhow::Result<String> {
    let content = super::novel_files::read_content(project_dir)?;
    let chapter = content
        .volumes
        .iter()
        .flat_map(|volume| volume.chapters.iter())
        .next()
        .context("no chapters to preview")?;
    let width = number_width(
        content
            .volumes
            .iter()
            .flat_map(|volume| volume.chapters.iter())
            .map(|chapter| chapter.number),
    );
    Ok(heading(
        kind,
        pattern,
        &chapter.full_title,
        &chapter.title,
        chapter.number,
        width,
        false,
    ))
}

/// 标题格式预览（`AppHandle` 入口）。
pub fn preview_title(
    app: &tauri::AppHandle,
    id: &str,
    kind: TxtTitleKind,
    pattern: &str,
) -> anyhow::Result<String> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    preview_title_in(&dir, kind, pattern)
}

/// 导出状态（`&Path` 入口）：只读探针，不建业务文件。
pub fn status_in(project_dir: &Path) -> anyhow::Result<ExportStatus> {
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = super::project_metadata::load_or_init_metadata_in(project_dir)?;
    let content = super::novel_files::read_content(project_dir).ok();
    let epub_name = super::novel_package::latest_package(project_dir).and_then(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    });
    let cover_ext = super::project_metadata::cover_absolute_path_in(project_dir)?
        .and_then(|absolute| cover_ext_of(Path::new(&absolute)).ok());
    Ok(ExportStatus {
        has_content: content.is_some(),
        content: content.as_ref().map(summarize),
        has_epub: epub_name.is_some(),
        epub_name,
        has_cover: cover_ext.is_some(),
        cover_ext,
        project_name: info.name,
        book_title: metadata.title,
    })
}

/// 目标子目录：`<自选目录>/<命名>/`；已存在且非空直接报错。
fn target_dir(dest: &Path, stem: &str) -> anyhow::Result<PathBuf> {
    let target = dest.join(stem);
    if target.exists() {
        if !target.is_dir() {
            anyhow::bail!("export target is not a directory");
        }
        let non_empty = std::fs::read_dir(&target)
            .with_context(|| format!("failed to read {}", target.display()))?
            .find_map(Result::ok)
            .is_some();
        if non_empty {
            anyhow::bail!("export directory is not empty");
        }
    } else {
        std::fs::create_dir_all(&target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }
    Ok(target)
}

/// 导出全量生成：改名拷贝 EPUB / 封面、`content.json` 转 TXT 落盘。
pub fn export_in(
    project_dir: &Path,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ExportItems,
    txt_options: &TxtOptions,
) -> anyhow::Result<ExportOutcome> {
    if !items.epub && !items.txt && !items.cover {
        anyhow::bail!("no export items selected");
    }
    let dest = Path::new(dest_dir)
        .canonicalize()
        .with_context(|| format!("export directory not found: {dest_dir}"))?;
    if !dest.is_dir() {
        anyhow::bail!("export directory is not a directory: {dest_dir}");
    }
    let info = super::projects::read_manifest(&project_dir.join("project.json"))?;
    let metadata = super::project_metadata::load_or_init_metadata_in(project_dir)?;
    let stem = match naming {
        ExportNaming::Project => super::novel_package::sanitize_stem(&info.name),
        ExportNaming::Book => super::novel_package::sanitize_stem(&metadata.title),
        ExportNaming::Custom => {
            if custom_name.trim().is_empty() {
                anyhow::bail!("custom name must not be empty");
            }
            super::novel_package::sanitize_stem(custom_name)
        }
    };
    let target = target_dir(&dest, &stem)?;

    let mut files = Vec::new();
    if items.epub {
        let source = super::novel_package::latest_package(project_dir)
            .context("no epub package to export")?;
        let name = format!("{stem}.epub");
        std::fs::copy(&source, target.join(&name))
            .with_context(|| format!("failed to copy {name}"))?;
        files.push(name);
    }
    if items.txt {
        let content = super::novel_files::read_content(project_dir)?;
        let metadata = super::project_metadata::load_or_init_metadata_in(project_dir)?;
        let name = format!("{stem}.txt");
        std::fs::write(
            target.join(&name),
            build_txt(&content, &metadata, txt_options)?,
        )
        .with_context(|| format!("failed to write {name}"))?;
        files.push(name);
    }
    if items.cover {
        let absolute = super::project_metadata::cover_absolute_path_in(project_dir)?
            .context("no cover to export")?;
        let ext = cover_ext_of(Path::new(&absolute))?;
        let name = format!("{stem}.{ext}");
        std::fs::copy(&absolute, target.join(&name))
            .with_context(|| format!("failed to copy {name}"))?;
        files.push(name);
    }
    files.sort();
    let dir = target
        .to_str()
        .context("export dir is not valid UTF-8")?
        .to_string();
    Ok(ExportOutcome { dir, files })
}

/// 导出状态（`AppHandle` 入口）。
pub fn export_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ExportStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 导出小说（`AppHandle` 入口）。
pub fn export_novel(
    app: &tauri::AppHandle,
    id: &str,
    dest_dir: &str,
    naming: ExportNaming,
    custom_name: &str,
    items: &ExportItems,
    txt_options: &TxtOptions,
) -> anyhow::Result<ExportOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    export_in(&dir, dest_dir, naming, custom_name, items, txt_options)
}

#[cfg(test)]
mod tests {
    use super::super::novel_build;
    use super::super::novel_files;
    use super::super::novel_package;
    use super::super::project_metadata;
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
            "ebook-studio-novel-export-test-{}-{case}",
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

    fn all_items() -> ExportItems {
        ExportItems {
            epub: true,
            txt: true,
            cover: true,
        }
    }

    /// 默认 TXT 选项直调导出：原文格式、无元数据。
    fn export_default(
        dir: &Path,
        dest: &str,
        naming: ExportNaming,
        custom: &str,
        items: &ExportItems,
    ) -> anyhow::Result<ExportOutcome> {
        export_in(dir, dest, naming, custom, items, &TxtOptions::default())
    }

    #[test]
    fn status_reflects_available_artifacts() {
        let (root, dir) = fixture_project("status", "第一章 引子\n正文\n");
        let status = status_in(&dir).expect("status");
        assert!(status.has_content);
        assert!(!status.has_epub);
        assert!(!status.has_cover);
        assert_eq!(status.project_name, "雾镇");
        assert_eq!(status.book_title, "雾镇");
        assert_eq!(
            status.content.expect("summary"),
            ContentSummary {
                volume_count: 1,
                chapter_count: 1,
                char_count: 2,
            }
        );
        cleanup(&root);
    }

    #[test]
    fn builds_standard_txt_layout() {
        let (root, dir) = fixture_project(
            "txt",
            "卷首语\n第一卷 开篇\n卷简介\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n",
        );
        let content = super::super::novel_files::read_content(&dir).expect("read");
        let metadata = project_metadata::load_or_init_metadata_in(&dir).expect("metadata");
        let text = build_txt(&content, &metadata, &TxtOptions::default()).expect("build txt");
        assert_eq!(
            text,
            "第一卷 开篇\n卷简介\n第一章 引子\n正文甲\n\n第二章 发展\n正文乙\n"
        );
        assert!(!text.contains('\r'));
        cleanup(&root);
    }

    #[test]
    fn exports_all_files_with_book_naming() {
        let (root, dir) = fixture_project("full", "第一章 引子\n正文甲\n");
        novel_build::build_in(&dir, &template_base()).expect("build");
        novel_package::write_epub_in(&dir, None).expect("package");
        let source = root.join("cover-source.png");
        std::fs::write(&source, [0x89, 0x50, 0x4E, 0x47]).expect("write fake cover");
        project_metadata::import_cover_in(&dir, &source).expect("import cover");

        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let outcome = export_default(
            &dir,
            dest.to_str().expect("utf8 dest"),
            ExportNaming::Book,
            "",
            &all_items(),
        )
        .expect("export");
        assert_eq!(
            outcome.files,
            vec![
                "雾镇.epub".to_string(),
                "雾镇.png".to_string(),
                "雾镇.txt".to_string(),
            ]
        );
        let target = dest.join("雾镇");
        assert!(target.join("雾镇.epub").is_file());
        assert!(target.join("雾镇.png").is_file());
        let txt = std::fs::read_to_string(target.join("雾镇.txt")).expect("read txt");
        assert_eq!(txt, "第一章 引子\n正文甲\n");
        assert_eq!(outcome.dir, target.to_str().expect("utf8 target"));
        cleanup(&root);
    }

    #[test]
    fn rejects_empty_items_custom_and_occupied_target() {
        let (root, dir) = fixture_project("reject", "第一章 引子\n正文\n");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        let none = ExportItems {
            epub: false,
            txt: false,
            cover: false,
        };
        assert!(
            export_default(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &none
            )
            .is_err()
        );

        let txt_only = ExportItems {
            epub: false,
            txt: true,
            cover: false,
        };
        assert!(
            export_default(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Custom,
                "  ",
                &txt_only
            )
            .is_err()
        );

        // 非法字符按打包口径清洗。
        let outcome = export_default(
            &dir,
            dest.to_str().expect("utf8"),
            ExportNaming::Custom,
            "a/b",
            &txt_only,
        )
        .expect("export with sanitized name");
        assert_eq!(outcome.files, vec!["a_b.txt".to_string()]);

        // 非空目录阻断，空目录复用。
        assert!(
            export_default(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Custom,
                "a/b",
                &txt_only
            )
            .is_err()
        );
        std::fs::create_dir_all(dest.join("empty")).expect("create empty");
        let reused = export_default(
            &dir,
            dest.to_str().expect("utf8"),
            ExportNaming::Custom,
            "empty",
            &txt_only,
        )
        .expect("reuse empty dir");
        assert_eq!(reused.files, vec!["empty.txt".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn rejects_missing_artifacts_per_item() {
        let (root, dir) = fixture_project("missing", "第一章 引子\n正文\n");
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).expect("create dest");
        // 有解析产物但无打包产物、无封面：EPUB 与封面分别报错。
        let epub_only = ExportItems {
            epub: true,
            txt: false,
            cover: false,
        };
        assert!(
            export_default(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &epub_only
            )
            .is_err()
        );
        let cover_only = ExportItems {
            epub: false,
            txt: false,
            cover: true,
        };
        assert!(
            export_default(
                &dir,
                dest.to_str().expect("utf8"),
                ExportNaming::Book,
                "",
                &cover_only
            )
            .is_err()
        );
        cleanup(&root);
    }

    #[test]
    fn converts_arabic_to_chinese_numerals() {
        for (value, lower, upper) in [
            (0, "零", "零"),
            (1, "一", "壹"),
            (9, "九", "玖"),
            (10, "十", "拾"),
            (11, "十一", "拾壹"),
            (20, "二十", "贰拾"),
            (101, "一百零一", "壹佰零壹"),
            (110, "一百一十", "壹佰壹拾"),
            (1001, "一千零一", "壹仟零壹"),
            (10000, "一万", "壹萬"),
            (10001, "一万零一", "壹萬零壹"),
            (10100, "一万零一百", "壹萬零壹佰"),
            (100_000, "十万", "拾萬"),
            (100_000_000, "一亿", "壹億"),
            (
                123_456_789,
                "一亿二千三百四十五万六千七百八十九",
                "壹億贰仟叁佰肆拾伍萬陆仟柒佰捌拾玖",
            ),
        ] {
            assert_eq!(format_chinese(value, false), lower, "lower {value}");
            assert_eq!(format_chinese(value, true), upper, "upper {value}");
        }
    }

    #[test]
    fn renders_title_formats_and_keeps_missing_placeholders() {
        // 六格式 × 有号。
        assert_eq!(
            heading(
                TxtTitleKind::Original,
                "",
                "第一章 引子",
                "引子",
                Some(1),
                2,
                false
            ),
            "第一章 引子"
        );
        assert_eq!(
            heading(
                TxtTitleKind::Arabic,
                "",
                "第一章 引子",
                "引子",
                Some(12),
                2,
                false
            ),
            "第12章 引子"
        );
        assert_eq!(
            heading(
                TxtTitleKind::Arabic,
                "",
                "第一卷 开篇",
                "开篇",
                Some(1),
                2,
                true
            ),
            "第1卷 开篇"
        );
        assert_eq!(
            heading(
                TxtTitleKind::Padded,
                "",
                "第一章 引子",
                "引子",
                Some(3),
                2,
                false
            ),
            "第03章 引子"
        );
        assert_eq!(
            heading(
                TxtTitleKind::ChineseLower,
                "",
                "第一章 引子",
                "引子",
                Some(11),
                2,
                false
            ),
            "第十一章 引子"
        );
        assert_eq!(
            heading(
                TxtTitleKind::ChineseUpper,
                "",
                "第一章 引子",
                "引子",
                Some(11),
                2,
                false
            ),
            "第拾壹章 引子"
        );
        // 自定义：转义、未知占位符与缺号留位。
        assert_eq!(
            render_title("第 #c 章： #T", Some(1), 2, "重生"),
            "第 一 章： 重生"
        );
        assert_eq!(render_title("##n #x #", Some(2), 2, "重生"), "#n #x #");
        assert_eq!(
            render_title("第 #c 章： #T", None, 2, "重生"),
            "第 #c 章： 重生"
        );
        assert_eq!(render_title("#N", Some(123), 2, ""), "123");
    }

    #[test]
    fn previews_first_chapter_title() {
        let (root, dir) = fixture_project("preview", "第一章 引子\n正文甲\n第二章 发展\n正文乙\n");
        assert_eq!(
            preview_title_in(&dir, TxtTitleKind::Padded, "").expect("preview"),
            "第01章 引子"
        );
        assert_eq!(
            preview_title_in(&dir, TxtTitleKind::Custom, "第#N章 #T").expect("preview custom"),
            "第01章 引子"
        );
        cleanup(&root);
    }

    #[test]
    fn builds_txt_with_numbered_titles_and_metadata() {
        let (root, dir) = fixture_project(
            "numbered",
            "第一卷 开篇\n第一章 引子\n正文甲\n第二章 发展\n正文乙\n",
        );
        let content = super::super::novel_files::read_content(&dir).expect("read");
        let mut metadata = project_metadata::load_or_init_metadata_in(&dir).expect("metadata");
        metadata.creator = "作者甲".to_string();
        metadata.rights = "版权所有".to_string();
        metadata.description = vec!["简介壹".to_string(), "简介贰".to_string()];
        metadata.subjects = vec!["玄幻".to_string(), String::new()];
        let options = TxtOptions {
            title_kind: TxtTitleKind::Custom,
            title_pattern: "第 #c 章： #T".to_string(),
            metadata: vec![
                MetadataField::Creator,
                MetadataField::Subjects,
                MetadataField::Description,
                MetadataField::Rights,
            ],
        };
        let text = build_txt(&content, &metadata, &options).expect("build txt");
        assert_eq!(
            text,
            "作者：作者甲\n版权：版权所有\n主题词：玄幻\n简介：\n简介壹\n简介贰\n\n――――――――――――――――――――――――――――――――\n\n第 一 章： 开篇\n第 一 章： 引子\n正文甲\n\n第 二 章： 发展\n正文乙\n"
        );
        assert!(!text.contains('\r'));
        // 空自定义模板直接报错。
        let empty = TxtOptions {
            title_kind: TxtTitleKind::Custom,
            title_pattern: "  ".to_string(),
            metadata: Vec::new(),
        };
        assert!(build_txt(&content, &metadata, &empty).is_err());
        cleanup(&root);
    }
}
