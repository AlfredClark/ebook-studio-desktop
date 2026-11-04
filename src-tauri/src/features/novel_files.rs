//! 小说原文：`<uuid>/sources/content.txt` 的导入、状态查询与卷章解析。
//!
//! 导入即覆盖拷贝；解析按卷章正则逐行切分后落盘 `<uuid>/content.json`（`metadata.json` 同级），
//! 三种版式（纯章节 / 分卷分章 / 分卷+卷简介+分章）由同一套行走查覆盖，无需模式切换。

use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use chinese_number::{ChineseCountMethod, ChineseToNumber};
use chrono::{DateTime, SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;

/// 原文落盘目录：与封面共用 `<uuid>/sources/`。
const SOURCES_DIR: &str = "sources";
/// 原文文件名：导入即覆盖，无版本保留。
const CONTENT_FILE: &str = "content.txt";
/// 原文大小上限：100MB（长篇 TXT 常见体量，超限拒绝复制）。
const MAX_TEXT_BYTES: u64 = 100 * 1024 * 1024;
/// 解析产物文件名：落盘项目根目录，与 `metadata.json` 同级。
const CONTENT_NAME: &str = "content.json";
/// `content.json` 的结构版本：初始开发阶段固定 `1`，不做跨版本迁移。
const CONTENT_SCHEMA_VERSION: u32 = 1;
/// 解析上限：超限报错而非截断，避免静默丢文。
const MAX_VOLUMES: usize = 2000;
/// 解析上限：全书章节总数，跨卷连续计数与卷内重计数都不影响切分。
const MAX_CHAPTERS: usize = 20000;
/// 解析上限：单章正文行数。
const MAX_CHAPTER_LINES: usize = 200_000;
/// 筛选上限：单章命中行数，超限截断本章（置 `truncated`），避免单章打爆 IPC。
const MAX_FILTER_LINE_MATCHES: usize = 500;
/// 筛选上限：全文命中文本段总数，超限停止全书扫描（置 `truncated`）。
const MAX_FILTER_SPANS: usize = 20_000;

/// 小说原文状态：左栏文件卡片的只读展示；缺文件时 `exists` 为假。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NovelTextStatus {
    /// `sources/content.txt` 是否存在。
    pub exists: bool,
    /// 原文字节数；不存在时为 `0`（`u32` 以满足 specta 导出约束，上限内足够）。
    pub size_bytes: u32,
    /// 原文更新时间（UTC `RFC3339`）；不存在或不可读时为 `None`。
    pub updated_at: Option<String>,
}

/// 查询原文状态（不建文件）：缺失直接回落空态，不报错。
pub fn status_in(project_dir: &Path) -> anyhow::Result<NovelTextStatus> {
    let path = project_dir.join(SOURCES_DIR).join(CONTENT_FILE);
    if !path.is_file() {
        return Ok(NovelTextStatus {
            exists: false,
            size_bytes: 0,
            updated_at: None,
        });
    }
    let metadata =
        std::fs::metadata(&path).with_context(|| format!("failed to stat {}", path.display()))?;
    let updated_at = metadata
        .modified()
        .ok()
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true));
    Ok(NovelTextStatus {
        exists: true,
        size_bytes: u32::try_from(metadata.len()).unwrap_or(u32::MAX),
        updated_at,
    })
}

/// 导入原文：校验通过后覆盖拷贝为 `sources/content.txt`，返回最新状态。
pub fn import_txt_in(project_dir: &Path, source: &Path) -> anyhow::Result<NovelTextStatus> {
    if !source.is_file() {
        anyhow::bail!("text source is not a file: {}", source.display());
    }
    let bytes = source
        .metadata()
        .with_context(|| format!("failed to stat text source: {}", source.display()))?
        .len();
    if bytes == 0 || bytes > MAX_TEXT_BYTES {
        anyhow::bail!("text size out of range (max 100MB): {}", source.display());
    }
    let is_txt = source
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
    if !is_txt {
        anyhow::bail!(
            "unsupported text type (expected .txt): {}",
            source.display()
        );
    }
    // 先读后拷：非 UTF-8 直接拒绝，不留下坏文件。
    std::fs::read_to_string(source)
        .with_context(|| format!("text source is not valid UTF-8: {}", source.display()))?;
    let sources = project_dir.join(SOURCES_DIR);
    std::fs::create_dir_all(&sources)
        .with_context(|| format!("failed to create {}", sources.display()))?;
    std::fs::copy(source, sources.join(CONTENT_FILE)).context("failed to copy text")?;
    status_in(project_dir)
}

/// 查询原文状态（`AppHandle` 入口）：项目目录不存在视为目标缺失。
pub fn text_status(app: &tauri::AppHandle, id: &str) -> anyhow::Result<NovelTextStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    status_in(&dir)
}

/// 导入原文（`AppHandle` 入口）。
pub fn import_text(
    app: &tauri::AppHandle,
    id: &str,
    source_path: &str,
) -> anyhow::Result<NovelTextStatus> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    import_txt_in(&dir, Path::new(source_path))
}

/// 单章：纯标题供后续自定义标题格式直接调用，完整行保留备查。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NovelChapter {
    /// 章纯标题（第二捕获组）；缺组或为空时回落完整行。
    pub title: String,
    /// 章完整标题（命中行整行，已修边）。
    pub full_title: String,
    /// 章编号：第一捕获组解析出的阿拉伯数字；缺组或不可解析时为 `None`。
    pub number: Option<u32>,
    /// 章正文：每元素一行，已去空去首尾空白。
    pub lines: Vec<String>,
}

/// 单卷：纯章节格式的隐式卷标题为 `None`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NovelVolume {
    /// 卷纯标题（第二捕获组）；隐式卷为 `None`，缺组或为空时回落完整行。
    pub title: Option<String>,
    /// 卷完整标题（命中行整行）；隐式卷为 `None`。
    pub full_title: Option<String>,
    /// 卷编号：第一捕获组解析出的阿拉伯数字；缺组或不可解析时为 `None`。
    pub number: Option<u32>,
    /// 卷简介：卷标题行之后、该卷首章标题之前的正文行。
    pub intro: Vec<String>,
    /// 卷内章节（跨卷连续计数与卷内重计数都按命中顺序收纳）。
    pub chapters: Vec<NovelChapter>,
}

/// 单章正文：编辑页中栏的读写单位；标题只读展示，正文按行往返。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ChapterContent {
    /// 章完整标题（只读展示用）。
    pub full_title: String,
    /// 章正文：每元素一行（与解析口径一致：去首尾空白、无空行）。
    pub lines: Vec<String>,
}

/// 筛选种类：右栏筛选器的前端契约；关键词含匹配与正则二选一。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "type")]
pub enum NovelFilter {
    /// 关键词含匹配：空关键词前后端都拦截。
    Keyword {
        /// 关键词原文（大小写不敏感时内部转小写比对，正文索引不受影响）。
        keyword: String,
        /// 是否区分大小写。
        case_sensitive: bool,
    },
    /// 正则表达式：Rust 正则语法，非法 pattern 直接报错。
    Regex {
        /// 用户填写的正则原文。
        pattern: String,
    },
}

/// 命中文本段：Rust `char` 索引；中文常用字落在 BMP，前后端一致，超平面字符由前端换算。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TextSpan {
    /// 起始字符索引（0-based）。
    pub start: u32,
    /// 命中字符数。
    pub len: u32,
}

/// 单行命中：行号为章内 1-based，与编辑器行号同源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct LineMatch {
    /// 章内行号（1-based）。
    pub line: u32,
    /// 本行全部非重叠命中（空跨度已剔除）。
    pub spans: Vec<TextSpan>,
}

/// 单章命中汇总：下标与大纲数组同源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ChapterMatches {
    /// 卷下标。
    pub volume_index: u32,
    /// 章下标。
    pub chapter_index: u32,
    /// 命中行（按行号升序）。
    pub matches: Vec<LineMatch>,
}

/// 筛选结果：命中章节清单；超限截断时置位，前端 toast 提示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct FilterOutcome {
    /// 有命中的章节（文档顺序）。
    pub chapters: Vec<ChapterMatches>,
    /// 是否因命中上限截断。
    pub truncated: bool,
}

/// 解析产物：`content.json` 的落盘格式即前端契约（`snake_case`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct NovelContent {
    /// 结构版本，固定 `1`，由后端断言。
    pub schema_version: u32,
    /// 本次切分使用的卷正则（追溯用）。
    pub volume_pattern: String,
    /// 本次切分使用的章正则（追溯用）。
    pub chapter_pattern: String,
    /// 解析时间（UTC `RFC3339`）。
    pub parsed_at: String,
    /// 书首前言：首个标题行之前的所有正文行。
    pub preface: Vec<String>,
    /// 分卷（含纯章节格式的单个无标题卷）。
    pub volumes: Vec<NovelVolume>,
}

/// 章大纲：右栏识别结果的只读条目，字数仅计章正文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ChapterOutline {
    /// 章完整标题。
    pub title: String,
    /// 本章正文字数（`lines` 字符数之和）。
    pub chars: u32,
}

/// 卷大纲：右栏识别结果的只读条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VolumeOutline {
    /// 卷标题；无标题卷为 `None`，前端回落“未分卷”。
    pub title: Option<String>,
    /// 卷内章节（含完整标题与正文字数）。
    pub chapters: Vec<ChapterOutline>,
}

/// 问题种类：解析结果的警告项，只做展示，不阻塞解析。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum IssueKind {
    /// 编号空缺：卷/章的 `number` 为 `None`。
    MissingNumber,
    /// 编号重复：同卷内章号重复或全局卷号重复。
    DuplicateNumber,
    /// 缺失章节：卷内章号或全局卷号存在断号。
    MissingChapter,
    /// 标题空缺：纯标题回落为完整行。
    MissingTitle,
}

/// 解析问题条目：卷/章均存完整行，前端定位与组装文案用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ContentIssue {
    /// 问题种类。
    pub kind: IssueKind,
    /// 所属卷完整行；卷级问题即该卷本身，全局断号时为 `None`。
    pub volume: Option<String>,
    /// 所属章完整行；卷级问题为 `None`。
    pub chapter: Option<String>,
    /// 相关编号：重复/缺失的编号值；空缺类为 `None`。
    pub number: Option<u32>,
}

/// 解析大纲：右栏识别结果的只读展示，正文行不出 IPC。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ParseOutline {
    /// 卷数（含无标题卷）。
    pub volume_count: u32,
    /// 全书章节总数。
    pub chapter_count: u32,
    /// 正文总行数：仅各章 `lines` 之和（前言、卷简介不计）。
    pub line_count: u32,
    /// 正文字数：仅各章 `lines` 字符数之和（不计换行符）。
    pub char_count: u32,
    /// 前言行数（暂不展示，保留备查）。
    pub preface_lines: u32,
    /// 卷章标题树。
    pub volumes: Vec<VolumeOutline>,
    /// 两套正则都没有数字组（第一捕获组）时为真；此时全量编号为 `None`。
    /// 个别数字串不可解析不置位，仅 pattern 级缺组置位。
    pub numbers_missing: bool,
    /// 两套正则都没有标题组（第二捕获组）时为真；此时纯标题全员回落。
    pub titles_missing: bool,
    /// 解析问题：文档顺序收集，展示用。
    pub issues: Vec<ContentIssue>,
}

/// 宽松转 `u32`：大纲计数上溢时钳制，不报错。
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 由完整内容派生识别大纲（落盘与返回共用同一口径）。
fn outline_of(content: &NovelContent) -> ParseOutline {
    let numbers_missing =
        lacks_number_group(&content.volume_pattern) && lacks_number_group(&content.chapter_pattern);
    let titles_missing =
        lacks_title_group(&content.volume_pattern) && lacks_title_group(&content.chapter_pattern);
    // 行数与字数仅统计章正文，前言与卷简介不计。
    let chapter_lines = content
        .volumes
        .iter()
        .flat_map(|v| v.chapters.iter())
        .flat_map(|c| c.lines.iter());
    let lines: Vec<&String> = chapter_lines.collect();
    ParseOutline {
        volume_count: saturating_u32(content.volumes.len()),
        chapter_count: saturating_u32(content.volumes.iter().map(|v| v.chapters.len()).sum()),
        line_count: saturating_u32(lines.len()),
        char_count: saturating_u32(lines.iter().map(|line| line.chars().count()).sum()),
        preface_lines: saturating_u32(content.preface.len()),
        numbers_missing,
        titles_missing,
        issues: collect_issues(content, numbers_missing, titles_missing),
        volumes: content
            .volumes
            .iter()
            .map(|v| VolumeOutline {
                // 大纲展示用完整行，纯标题仅供后续构建调用。
                title: v.full_title.clone(),
                chapters: v
                    .chapters
                    .iter()
                    .map(|c| ChapterOutline {
                        title: c.full_title.clone(),
                        chars: saturating_u32(
                            c.lines.iter().map(|line| line.chars().count()).sum(),
                        ),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// 标题组缺失判定：正则连第二捕获组都没有；编译失败也视为缺失。
fn lacks_title_group(pattern: &str) -> bool {
    Regex::new(pattern)
        .ok()
        .is_none_or(|re| re.captures_len() < 3)
}

/// 纯标题是否回落：隐式无标题卷（双 `None`）不算回落。
fn is_fallback_title(title: Option<&str>, full_title: Option<&str>) -> bool {
    matches!((title, full_title), (Some(t), Some(f)) if t == f)
}

/// 收集解析问题：文档顺序遍历，pattern 级缺组时跳过对应细则。
fn collect_issues(
    content: &NovelContent,
    numbers_missing: bool,
    titles_missing: bool,
) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    if numbers_missing {
        // 编号检查全跳过，由前端单条提示 pattern 缺组。
        return issues;
    }
    collect_item_issues(content, titles_missing, &mut issues);
    collect_duplicate_issues(content, &mut issues);
    collect_gap_issues(content, &mut issues);
    issues
}

/// 逐条检查：编号空缺与纯标题回落（文档顺序）。
fn collect_item_issues(
    content: &NovelContent,
    titles_missing: bool,
    issues: &mut Vec<ContentIssue>,
) {
    for volume in &content.volumes {
        // 隐式无标题卷没有标题行，不参与卷级编号与标题检查；其下章节照常检查。
        let has_title_line = volume.full_title.is_some();
        if has_title_line && volume.number.is_none() {
            issues.push(ContentIssue {
                kind: IssueKind::MissingNumber,
                volume: volume.full_title.clone(),
                chapter: None,
                number: None,
            });
        }
        if !titles_missing
            && is_fallback_title(volume.title.as_deref(), volume.full_title.as_deref())
        {
            issues.push(ContentIssue {
                kind: IssueKind::MissingTitle,
                volume: volume.full_title.clone(),
                chapter: None,
                number: None,
            });
        }
        for chapter in &volume.chapters {
            if chapter.number.is_none() {
                issues.push(ContentIssue {
                    kind: IssueKind::MissingNumber,
                    volume: volume.full_title.clone(),
                    chapter: Some(chapter.full_title.clone()),
                    number: None,
                });
            }
            if !titles_missing && chapter.title == chapter.full_title {
                issues.push(ContentIssue {
                    kind: IssueKind::MissingTitle,
                    volume: volume.full_title.clone(),
                    chapter: Some(chapter.full_title.clone()),
                    number: None,
                });
            }
        }
    }
}

/// 编号重复：章号同卷内判重，卷号全局判重。
fn collect_duplicate_issues(content: &NovelContent, issues: &mut Vec<ContentIssue>) {
    let mut seen = HashSet::new();
    let mut reported = HashSet::new();
    for volume in &content.volumes {
        if let Some(n) = volume.number
            && !seen.insert(n)
            && reported.insert(n)
        {
            issues.push(ContentIssue {
                kind: IssueKind::DuplicateNumber,
                volume: volume.full_title.clone(),
                chapter: None,
                number: Some(n),
            });
        }
    }
    for volume in &content.volumes {
        let mut seen = HashSet::new();
        let mut reported = HashSet::new();
        for chapter in &volume.chapters {
            if let Some(n) = chapter.number
                && !seen.insert(n)
                && reported.insert(n)
            {
                issues.push(ContentIssue {
                    kind: IssueKind::DuplicateNumber,
                    volume: volume.full_title.clone(),
                    chapter: Some(chapter.full_title.clone()),
                    number: Some(n),
                });
            }
        }
    }
}

/// 断号：卷号全局按 min..max，章号每卷内按 min..max。
fn collect_gap_issues(content: &NovelContent, issues: &mut Vec<ContentIssue>) {
    let mut volume_nums: Vec<u32> = content.volumes.iter().filter_map(|v| v.number).collect();
    volume_nums.sort_unstable();
    volume_nums.dedup();
    if let (Some(&min), Some(&max)) = (volume_nums.first(), volume_nums.last()) {
        for expected in min..=max {
            if volume_nums.binary_search(&expected).is_err() {
                issues.push(ContentIssue {
                    kind: IssueKind::MissingChapter,
                    volume: None,
                    chapter: None,
                    number: Some(expected),
                });
            }
        }
    }
    for volume in &content.volumes {
        let mut nums: Vec<u32> = volume.chapters.iter().filter_map(|c| c.number).collect();
        nums.sort_unstable();
        nums.dedup();
        if let (Some(&min), Some(&max)) = (nums.first(), nums.last()) {
            for expected in min..=max {
                if nums.binary_search(&expected).is_err() {
                    issues.push(ContentIssue {
                        kind: IssueKind::MissingChapter,
                        volume: volume.full_title.clone(),
                        chapter: None,
                        number: Some(expected),
                    });
                }
            }
        }
    }
}
/// 取当前卷的可变引用；尚无卷时先建无标题卷（纯章节格式入口）。
fn current_volume(content: &mut NovelContent) -> &mut NovelVolume {
    if content.volumes.is_empty() {
        content.volumes.push(NovelVolume {
            title: None,
            full_title: None,
            number: None,
            intro: Vec::new(),
            chapters: Vec::new(),
        });
    }
    content.volumes.last_mut().expect("volume just ensured")
}

/// 非标准逐位数字的字符映射：`一一零五` 类写法按位直读。
const fn digit_value(ch: char) -> Option<char> {
    match ch {
        '零' | '〇' | '○' => Some('0'),
        '一' | '壹' => Some('1'),
        '二' | '贰' | '两' | '兩' => Some('2'),
        '三' | '叁' | '參' => Some('3'),
        '四' | '肆' => Some('4'),
        '五' | '伍' => Some('5'),
        '六' | '陆' | '陸' => Some('6'),
        '七' | '柒' => Some('7'),
        '八' | '捌' => Some('8'),
        '九' | '玖' => Some('9'),
        _ => None,
    }
}

/// 逐位回退：全由数字字符组成时按位拼成阿拉伯数字；含单位字的不处理。
fn parse_digit_sequence(raw: &str) -> Option<u32> {
    let mut digits = String::with_capacity(raw.len());
    for ch in raw.chars().filter(|ch| !ch.is_whitespace()) {
        digits.push(digit_value(ch)?);
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// 数字串→阿拉伯数字：阿拉伯直解 → 标准中文数字（万进）→ 非标准逐位回退。
fn parse_numeral(raw: &str) -> Option<u32> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(value) = text.parse::<u32>() {
        return Some(value);
    }
    let parsed: Result<i64, _> = text.to_number(ChineseCountMethod::TenThousand);
    if let Ok(value) = parsed {
        return u32::try_from(value).ok();
    }
    parse_digit_sequence(text)
}

/// 纯标题提取：第二捕获组修边后非空即用，否则回落完整行。
fn pure_title(caps: &regex::Captures<'_>, line: &str) -> String {
    caps.get(2)
        .map(|matched| matched.as_str().trim())
        .filter(|text| !text.is_empty())
        .unwrap_or(line)
        .to_string()
}

/// 数字组缺失判定：正则连第一捕获组都没有；编译失败也视为缺失。
fn lacks_number_group(pattern: &str) -> bool {
    Regex::new(pattern)
        .ok()
        .is_none_or(|re| re.captures_len() < 2)
}

/// 解析原文：按卷章正则逐行切分后落盘 `content.json`，返回识别大纲。
///
/// 单遍行走查：卷命中开新卷（同行双命中归卷），章命中开新章（无卷时先建无标题卷），
/// 普通行按位置归属前言 / 卷简介 / 章正文；空行与首尾空白在判定前统一移除。
pub fn parse_txt_in(
    project_dir: &Path,
    volume_pattern: &str,
    chapter_pattern: &str,
) -> anyhow::Result<ParseOutline> {
    let volume_re = Regex::new(volume_pattern).context("invalid volume pattern")?;
    let chapter_re = Regex::new(chapter_pattern).context("invalid chapter pattern")?;
    let text = std::fs::read_to_string(project_dir.join(SOURCES_DIR).join(CONTENT_FILE))
        .context("failed to read content.txt, import text first")?;
    let mut content = NovelContent {
        schema_version: CONTENT_SCHEMA_VERSION,
        volume_pattern: volume_pattern.to_string(),
        chapter_pattern: chapter_pattern.to_string(),
        parsed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        preface: Vec::new(),
        volumes: Vec::new(),
    };
    let mut chapter_total = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = volume_re.captures(line) {
            if content.volumes.len() >= MAX_VOLUMES {
                anyhow::bail!("too many volumes (max {MAX_VOLUMES})");
            }
            let number = caps
                .get(1)
                .and_then(|matched| parse_numeral(matched.as_str()));
            content.volumes.push(NovelVolume {
                title: Some(pure_title(&caps, line)),
                full_title: Some(line.to_string()),
                number,
                intro: Vec::new(),
                chapters: Vec::new(),
            });
        } else if let Some(caps) = chapter_re.captures(line) {
            if chapter_total >= MAX_CHAPTERS {
                anyhow::bail!("too many chapters (max {MAX_CHAPTERS})");
            }
            chapter_total += 1;
            let number = caps
                .get(1)
                .and_then(|matched| parse_numeral(matched.as_str()));
            let title = pure_title(&caps, line);
            current_volume(&mut content).chapters.push(NovelChapter {
                title,
                full_title: line.to_string(),
                number,
                lines: Vec::new(),
            });
        } else if content.volumes.is_empty() {
            content.preface.push(line.to_string());
        } else {
            let volume = content.volumes.last_mut().expect("volume exists");
            if volume.chapters.is_empty() {
                volume.intro.push(line.to_string());
            } else {
                let chapter = volume.chapters.last_mut().expect("chapter exists");
                if chapter.lines.len() >= MAX_CHAPTER_LINES {
                    anyhow::bail!("chapter is too long (max {MAX_CHAPTER_LINES} lines)");
                }
                chapter.lines.push(line.to_string());
            }
        }
    }
    if chapter_total == 0 {
        anyhow::bail!("no chapters recognized, check title patterns against the text");
    }
    let outline = outline_of(&content);
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    Ok(outline)
}

/// 强制重排编号：按文档顺序重写卷号 1..N、章号每卷内 1..M，落盘并返回新大纲。
/// 标题与正文不动；重排后编号类问题自然消除，标题类问题保留。
pub fn renumber_in(project_dir: &Path) -> anyhow::Result<ParseOutline> {
    let path = project_dir.join(CONTENT_NAME);
    let text = std::fs::read_to_string(&path)
        .context("failed to read content.json, parse the text first")?;
    let mut content: NovelContent =
        serde_json::from_str(&text).context("failed to parse content.json")?;
    for (i, volume) in content.volumes.iter_mut().enumerate() {
        volume.number = Some(saturating_u32(i + 1));
        for (j, chapter) in volume.chapters.iter_mut().enumerate() {
            chapter.number = Some(saturating_u32(j + 1));
        }
    }
    content.parsed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    Ok(outline_of(&content))
}
/// 读取已落盘的识别大纲：缺失或损坏回落空，不建文件（初始开发阶段不做版本门）。
pub fn outline_in(project_dir: &Path) -> Option<ParseOutline> {
    let path = project_dir.join(CONTENT_NAME);
    if !path.is_file() {
        return None;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            log::warn!("跳过不可读的解析产物 {}: {err:#}", path.display());
            return None;
        }
    };
    let content: NovelContent = match serde_json::from_str(&text) {
        Ok(content) => content,
        Err(err) => {
            log::warn!("跳过损坏的解析产物 {}: {err:#}", path.display());
            return None;
        }
    };
    Some(outline_of(&content))
}

/// 强制重排编号（`AppHandle` 入口）。
pub fn renumber_content(app: &tauri::AppHandle, id: &str) -> anyhow::Result<ParseOutline> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    renumber_in(&dir)
}

/// 解析原文（`AppHandle` 入口）。
pub fn parse_text(
    app: &tauri::AppHandle,
    id: &str,
    volume_pattern: &str,
    chapter_pattern: &str,
) -> anyhow::Result<ParseOutline> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    parse_txt_in(&dir, volume_pattern, chapter_pattern)
}

/// 读取已落盘的识别大纲（`AppHandle` 入口）：只读，不建文件。
pub fn content_outline(app: &tauri::AppHandle, id: &str) -> anyhow::Result<Option<ParseOutline>> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    Ok(outline_in(&dir))
}

/// 读 `content.json` 全量并断言结构版本：版本不符直接报错，不静默回落。
fn read_content(project_dir: &Path) -> anyhow::Result<NovelContent> {
    let path = project_dir.join(CONTENT_NAME);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}, parse the text first", path.display()))?;
    let content: NovelContent = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if content.schema_version != CONTENT_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported content schema version: {}",
            content.schema_version
        );
    }
    Ok(content)
}

/// 读单章正文（下标寻址）：下标越界直接报错，由前端按分支处理。
pub fn chapter_in(
    project_dir: &Path,
    volume_index: usize,
    chapter_index: usize,
) -> anyhow::Result<ChapterContent> {
    let content = read_content(project_dir)?;
    let volume = content.volumes.get(volume_index).with_context(|| {
        format!("chapter address out of range: volume {volume_index}, chapter {chapter_index}")
    })?;
    let chapter = volume.chapters.get(chapter_index).with_context(|| {
        format!("chapter address out of range: volume {volume_index}, chapter {chapter_index}")
    })?;
    Ok(ChapterContent {
        full_title: chapter.full_title.clone(),
        lines: chapter.lines.clone(),
    })
}

/// 存单章正文并返回新大纲：行归一化与解析口径一致（去首尾空白、丢弃空行），
/// 单章行数仍受解析上限约束；标题与编号不动。
pub fn update_chapter_in(
    project_dir: &Path,
    volume_index: usize,
    chapter_index: usize,
    lines: Vec<String>,
) -> anyhow::Result<ParseOutline> {
    let mut content = read_content(project_dir)?;
    let normalized: Vec<String> = lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if normalized.len() > MAX_CHAPTER_LINES {
        anyhow::bail!("chapter is too long (max {MAX_CHAPTER_LINES} lines)");
    }
    let volume = content.volumes.get_mut(volume_index).with_context(|| {
        format!("chapter address out of range: volume {volume_index}, chapter {chapter_index}")
    })?;
    let chapter = volume.chapters.get_mut(chapter_index).with_context(|| {
        format!("chapter address out of range: volume {volume_index}, chapter {chapter_index}")
    })?;
    chapter.lines = normalized;
    content.parsed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let text = serde_json::to_string_pretty(&content).context("failed to serialize content")?;
    std::fs::write(project_dir.join(CONTENT_NAME), text).context("failed to write content.json")?;
    Ok(outline_of(&content))
}

/// 读单章正文（`AppHandle` 入口）：下标寻址，越界直接报错。
pub fn chapter_content(
    app: &tauri::AppHandle,
    id: &str,
    volume_index: u32,
    chapter_index: u32,
) -> anyhow::Result<ChapterContent> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    chapter_in(&dir, volume_index as usize, chapter_index as usize)
}

/// 存单章正文（`AppHandle` 入口）：落盘后返回新大纲。
pub fn update_chapter(
    app: &tauri::AppHandle,
    id: &str,
    volume_index: u32,
    chapter_index: u32,
    lines: Vec<String>,
) -> anyhow::Result<ParseOutline> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    update_chapter_in(&dir, volume_index as usize, chapter_index as usize, lines)
}

/// 筛选种类编译为正则：关键词转义后统一走正则引擎，命中字节区间落在原文上，
/// 大小写不敏感经 `(?i)` 由引擎处理，不挪动原文索引。
fn build_filter_regex(filter: &NovelFilter) -> anyhow::Result<Regex> {
    match filter {
        NovelFilter::Keyword {
            keyword,
            case_sensitive,
        } => {
            if keyword.trim().is_empty() {
                anyhow::bail!("keyword must not be empty");
            }
            let pattern = if *case_sensitive {
                regex::escape(keyword)
            } else {
                format!("(?i){}", regex::escape(keyword))
            };
            Regex::new(&pattern).context("invalid keyword pattern")
        }
        NovelFilter::Regex { pattern } => {
            if pattern.trim().is_empty() {
                anyhow::bail!("pattern must not be empty");
            }
            Regex::new(pattern).context("invalid regex pattern")
        }
    }
}

/// 单行全部命中：字节区间换算为 `char` 索引，空跨度（如 `x*` 类）直接剔除。
fn collect_line_spans(re: &Regex, line: &str) -> Vec<TextSpan> {
    let char_index_of = |byte: usize| {
        line.char_indices()
            .enumerate()
            .find_map(|(char_index, (byte_index, _))| (byte_index == byte).then_some(char_index))
            .unwrap_or_else(|| line.chars().count())
    };
    re.find_iter(line)
        .filter(|matched| !matched.is_empty())
        .map(|matched| TextSpan {
            start: saturating_u32(char_index_of(matched.start())),
            len: saturating_u32(matched.as_str().chars().count()),
        })
        .collect()
}

/// 全书筛选：按文档顺序逐章逐行扫描，命中章节进清单；超限截断并置位。
pub fn filter_content_in(
    project_dir: &Path,
    filter: &NovelFilter,
) -> anyhow::Result<FilterOutcome> {
    let re = build_filter_regex(filter)?;
    let content = read_content(project_dir)?;
    let mut outcome = FilterOutcome {
        chapters: Vec::new(),
        truncated: false,
    };
    let mut span_total = 0usize;
    'volumes: for (volume_index, volume) in content.volumes.iter().enumerate() {
        for (chapter_index, chapter) in volume.chapters.iter().enumerate() {
            let mut matches = Vec::new();
            for (line_index, line) in chapter.lines.iter().enumerate() {
                let spans = collect_line_spans(&re, line);
                if spans.is_empty() {
                    continue;
                }
                span_total += spans.len();
                matches.push(LineMatch {
                    line: saturating_u32(line_index + 1),
                    spans,
                });
                if span_total >= MAX_FILTER_SPANS {
                    outcome.truncated = true;
                    break 'volumes;
                }
                if matches.len() >= MAX_FILTER_LINE_MATCHES {
                    outcome.truncated = true;
                    break;
                }
            }
            if !matches.is_empty() {
                outcome.chapters.push(ChapterMatches {
                    volume_index: saturating_u32(volume_index),
                    chapter_index: saturating_u32(chapter_index),
                    matches,
                });
            }
        }
    }
    Ok(outcome)
}

/// 全书筛选（`AppHandle` 入口）：只读，不建文件。
pub fn filter_content(
    app: &tauri::AppHandle,
    id: &str,
    filter: &NovelFilter,
) -> anyhow::Result<FilterOutcome> {
    let dir = super::projects::resolve_project_dir(app, id)?;
    filter_content_in(&dir, filter)
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
            "ebook-studio-novel-files-test-{}-{case}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        let info =
            projects::create_project_in(&root, "雾镇", ProjectType::Novel).expect("create project");
        let dir = root.join(&info.id);
        (root, dir)
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    /// 写一个源文本文件，返回其路径。
    fn write_source(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        std::fs::write(&path, bytes).expect("write source");
        path
    }

    /// 直接写 `sources/content.txt`（跳过导入校验，聚焦切分逻辑）。
    fn write_content(dir: &Path, text: &str) {
        let sources = dir.join(SOURCES_DIR);
        std::fs::create_dir_all(&sources).expect("create sources");
        std::fs::write(sources.join(CONTENT_FILE), text).expect("write content");
    }

    /// 默认卷章正则：与前端默认值同口径，第一捕获组为数字。
    const VOLUME_PATTERN: &str =
        "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)卷[：:\\s]*(.*)$";
    const CHAPTER_PATTERN: &str =
        "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)章[：:\\s]*(.*)$";

    #[test]
    fn missing_text_reports_empty_status() {
        let (root, dir) = test_project("empty");
        let status = status_in(&dir).expect("status");
        assert_eq!(
            status,
            NovelTextStatus {
                exists: false,
                size_bytes: 0,
                updated_at: None,
            }
        );
        assert!(!dir.join(SOURCES_DIR).join(CONTENT_FILE).exists());
        cleanup(&root);
    }

    #[test]
    fn imports_txt_and_overwrites_on_reimport() {
        let (root, dir) = test_project("import");
        let first = write_source(&root, "first.TXT", "第一卷\n第一章 引子\n正文".as_bytes());
        let status = import_txt_in(&dir, &first).expect("import");
        assert!(status.exists);
        assert_eq!(
            u64::from(status.size_bytes),
            first.metadata().expect("stat").len()
        );
        assert!(status.updated_at.is_some());
        let stored = dir.join(SOURCES_DIR).join(CONTENT_FILE);
        assert!(stored.is_file());
        assert_eq!(
            std::fs::read_to_string(&stored).expect("read stored"),
            "第一卷\n第一章 引子\n正文"
        );
        let second = write_source(&root, "second.txt", "第二卷\n".as_bytes());
        let status = import_txt_in(&dir, &second).expect("reimport");
        assert!(status.exists);
        assert_eq!(
            std::fs::read_to_string(&stored).expect("read stored"),
            "第二卷\n"
        );
        cleanup(&root);
    }

    #[test]
    fn rejects_bad_text_sources() {
        let (root, dir) = test_project("bad");
        let md = write_source(&root, "note.md", "正文".as_bytes());
        assert!(import_txt_in(&dir, &md).is_err());
        let empty = write_source(&root, "empty.txt", &[]);
        assert!(import_txt_in(&dir, &empty).is_err());
        let binary = write_source(&root, "binary.txt", &[0xFF, 0xFE, 0x00]);
        assert!(import_txt_in(&dir, &binary).is_err());
        assert!(import_txt_in(&dir, &root.join("missing.txt")).is_err());
        // 全部拒绝后不应留下半成品。
        assert!(!dir.join(SOURCES_DIR).join(CONTENT_FILE).exists());
        cleanup(&root);
    }

    #[test]
    fn reads_and_updates_single_chapter() {
        let (root, dir) = test_project("chapter-rw");
        write_content(
            &dir,
            "第一卷 开篇\n第一章 引子\n正文甲\n正文乙\n第二章 发展\n正文丙\n第二卷 承接\n第一章 再起\n正文丁\n",
        );
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert_eq!(outline.chapter_count, 3);
        let chapter = chapter_in(&dir, 0, 1).expect("read");
        assert_eq!(chapter.full_title, "第二章 发展");
        assert_eq!(chapter.lines, vec!["正文丙"]);
        let outline = update_chapter_in(
            &dir,
            0,
            1,
            vec![
                "  新正文  ".to_string(),
                String::new(),
                "第二行".to_string(),
            ],
        )
        .expect("update");
        // 保存归一化与解析口径一致：去首尾空白、丢弃空行。
        let chapter = chapter_in(&dir, 0, 1).expect("reread");
        assert_eq!(chapter.full_title, "第二章 发展");
        assert_eq!(chapter.lines, vec!["新正文", "第二行"]);
        assert_eq!(outline.volumes[0].chapters[1].chars, 6);
        cleanup(&root);
    }

    #[test]
    fn chapter_address_out_of_range_errors() {
        let (root, dir) = test_project("chapter-range");
        write_content(&dir, "第一章 引子\n正文\n");
        parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert!(chapter_in(&dir, 0, 5).is_err());
        assert!(chapter_in(&dir, 3, 0).is_err());
        assert!(update_chapter_in(&dir, 0, 9, vec!["正文".to_string()]).is_err());
        cleanup(&root);
    }

    #[test]
    fn chapter_without_content_json_errors() {
        let (root, dir) = test_project("chapter-missing");
        assert!(chapter_in(&dir, 0, 0).is_err());
        assert!(update_chapter_in(&dir, 0, 0, vec!["正文".to_string()]).is_err());
        cleanup(&root);
    }

    /// 筛选夹具：两卷三章， mixed 中英文行。
    fn write_filter_text(dir: &Path) {
        write_content(
            dir,
            "第一卷 开篇\n第一章 引子\nHe whispered hello\n甲乙丙甲\n第二章 发展\nNothing here\n第二卷 承接\n第一章 再起\nHELLO world\n",
        );
        parse_txt_in(dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
    }

    #[test]
    fn filters_keyword_case_insensitive_with_char_spans() {
        let (root, dir) = test_project("filter-keyword");
        write_filter_text(&dir);
        let outcome = filter_content_in(
            &dir,
            &NovelFilter::Keyword {
                keyword: "hello".to_string(),
                case_sensitive: false,
            },
        )
        .expect("filter");
        assert!(!outcome.truncated);
        assert_eq!(outcome.chapters.len(), 2);
        assert_eq!(
            outcome.chapters[0].matches[0],
            LineMatch {
                line: 1,
                spans: vec![TextSpan { start: 13, len: 5 }],
            }
        );
        assert_eq!(
            outcome.chapters[1].matches[0],
            LineMatch {
                line: 1,
                spans: vec![TextSpan { start: 0, len: 5 }],
            }
        );
        cleanup(&root);
    }

    #[test]
    fn filters_keyword_case_sensitive_and_chinese_chars() {
        let (root, dir) = test_project("filter-case");
        write_filter_text(&dir);
        let outcome = filter_content_in(
            &dir,
            &NovelFilter::Keyword {
                keyword: "甲".to_string(),
                case_sensitive: true,
            },
        )
        .expect("filter");
        assert_eq!(outcome.chapters.len(), 1);
        assert_eq!(
            outcome.chapters[0].matches[0],
            LineMatch {
                line: 2,
                spans: vec![TextSpan { start: 0, len: 1 }, TextSpan { start: 3, len: 1 },],
            }
        );
        let sensitive = filter_content_in(
            &dir,
            &NovelFilter::Keyword {
                keyword: "hello".to_string(),
                case_sensitive: true,
            },
        )
        .expect("filter");
        assert_eq!(sensitive.chapters.len(), 1);
        cleanup(&root);
    }

    #[test]
    fn filters_regex_and_rejects_bad_patterns() {
        let (root, dir) = test_project("filter-regex");
        write_filter_text(&dir);
        let outcome = filter_content_in(
            &dir,
            &NovelFilter::Regex {
                pattern: "^He".to_string(),
            },
        )
        .expect("filter");
        assert_eq!(outcome.chapters.len(), 1);
        assert_eq!(outcome.chapters[0].matches[0].line, 1);
        assert!(
            filter_content_in(
                &dir,
                &NovelFilter::Regex {
                    pattern: "(".to_string(),
                },
            )
            .is_err()
        );
        assert!(
            filter_content_in(
                &dir,
                &NovelFilter::Keyword {
                    keyword: "   ".to_string(),
                    case_sensitive: false,
                },
            )
            .is_err()
        );
        assert!(
            filter_content_in(
                &dir,
                &NovelFilter::Regex {
                    pattern: String::new(),
                },
            )
            .is_err()
        );
        cleanup(&root);
    }

    #[test]
    fn filter_truncates_huge_match_sets() {
        let (root, dir) = test_project("filter-truncate");
        let text = format!(
            "第一章 引子\n{}",
            (0..600).map(|_| "命中行").collect::<Vec<_>>().join("\n")
        );
        write_content(&dir, &text);
        parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        let outcome = filter_content_in(
            &dir,
            &NovelFilter::Keyword {
                keyword: "命中".to_string(),
                case_sensitive: true,
            },
        )
        .expect("filter");
        assert!(outcome.truncated);
        assert_eq!(outcome.chapters.len(), 1);
        assert_eq!(outcome.chapters[0].matches.len(), MAX_FILTER_LINE_MATCHES);
        cleanup(&root);
    }

    #[test]
    fn filter_without_content_json_errors() {
        let (root, dir) = test_project("filter-missing");
        assert!(
            filter_content_in(
                &dir,
                &NovelFilter::Keyword {
                    keyword: "正文".to_string(),
                    case_sensitive: true,
                },
            )
            .is_err()
        );
        cleanup(&root);
    }

    #[test]
    fn parses_pure_chapters_into_untitled_volume() {
        let (root, dir) = test_project("parse-flat");
        write_content(
            &dir,
            "第一章 引子\n\n  正文第一行  \n\n正文第二行\n第二章 发展\n尾声\n",
        );
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert_eq!(outline.volume_count, 1);
        assert_eq!(outline.chapter_count, 2);
        assert_eq!(outline.line_count, 3);
        assert_eq!(outline.char_count, 12);
        assert_eq!(outline.preface_lines, 0);
        assert_eq!(outline.volumes[0].title, None);
        assert!(!outline.numbers_missing);
        assert!(!outline.titles_missing);
        assert_eq!(outline.issues, Vec::new());
        assert_eq!(
            outline.volumes[0]
                .chapters
                .iter()
                .map(|c| c.title.clone())
                .collect::<Vec<_>>(),
            vec!["第一章 引子".to_string(), "第二章 发展".to_string()]
        );
        assert_eq!(outline.volumes[0].chapters[0].chars, 10);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.schema_version, 1);
        assert_eq!(stored.preface, Vec::<String>::new());
        assert_eq!(stored.volumes.len(), 1);
        // 空行移除、首尾空白修剪均已生效。
        assert_eq!(
            stored.volumes[0].chapters[0].lines,
            vec!["正文第一行".to_string(), "正文第二行".to_string()]
        );
        assert_eq!(stored.volumes[0].chapters[0].title, "引子");
        assert_eq!(stored.volumes[0].chapters[0].full_title, "第一章 引子");
        assert_eq!(stored.volumes[0].chapters[0].number, Some(1));
        assert_eq!(stored.volumes[0].chapters[1].number, Some(2));
        assert_eq!(
            stored.volumes[0].chapters[1].lines,
            vec!["尾声".to_string()]
        );
        cleanup(&root);
    }

    #[test]
    fn parses_volumes_with_restarted_numbering_in_order() {
        let (root, dir) = test_project("parse-volumes");
        write_content(
            &dir,
            "第一卷 开篇\n第一章 起\n甲\n第二卷 承接\n第一章 再起\n乙\n",
        );
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert_eq!(outline.volume_count, 2);
        assert_eq!(outline.chapter_count, 2);
        assert_eq!(outline.volumes[1].chapters.len(), 1);
        assert_eq!(outline.volumes[1].chapters[0].title, "第一章 再起");
        assert_eq!(outline.volumes[1].chapters[0].chars, 1);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.volumes[0].title.as_deref(), Some("开篇"));
        assert_eq!(stored.volumes[0].full_title.as_deref(), Some("第一卷 开篇"));
        assert_eq!(stored.volumes[0].number, Some(1));
        assert_eq!(stored.volumes[0].chapters[0].title, "起");
        assert_eq!(stored.volumes[0].chapters[0].full_title, "第一章 起");
        assert_eq!(stored.volumes[0].chapters[0].number, Some(1));
        assert_eq!(stored.volumes[1].number, Some(2));
        assert_eq!(stored.volumes[1].chapters[0].number, Some(1));
        assert_eq!(stored.volumes[0].intro, Vec::<String>::new());
        assert_eq!(stored.volumes[1].title.as_deref(), Some("承接"));
        assert_eq!(stored.volumes[1].full_title.as_deref(), Some("第二卷 承接"));
        assert_eq!(stored.volumes[1].chapters[0].lines, vec!["乙".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn parses_preface_and_volume_intro() {
        let (root, dir) = test_project("parse-intro");
        write_content(
            &dir,
            "楔子前言\n\n第一卷 卷一\n  卷简介第一行\n卷简介第二行\n\n第一章 首章\n正文\n",
        );
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert_eq!(outline.preface_lines, 1);
        // 行数与字数仅统计章正文，前言与卷简介不计。
        assert_eq!(outline.line_count, 1);
        assert_eq!(outline.char_count, 2);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.preface, vec!["楔子前言".to_string()]);
        assert_eq!(stored.volumes[0].title.as_deref(), Some("卷一"));
        assert_eq!(
            stored.volumes[0].intro,
            vec!["卷简介第一行".to_string(), "卷简介第二行".to_string()]
        );
        assert_eq!(
            stored.volumes[0].chapters[0].lines,
            vec!["正文".to_string()]
        );
        cleanup(&root);
    }

    #[test]
    fn volume_match_wins_over_chapter_match() {
        let (root, dir) = test_project("parse-priority");
        write_content(&dir, "第一卷 开篇\n第一章 首章\n文\n");
        // 卷行同时命中宽泛的章正则时仍归卷。
        let outline = parse_txt_in(&dir, "^第.+卷.*$", "^第.+.*$").expect("parse");
        assert_eq!(outline.volume_count, 1);
        assert_eq!(outline.chapter_count, 1);
        assert_eq!(outline.volumes[0].title.as_deref(), Some("第一卷 开篇"));
        // 无捕获组时纯标题与编号都回落：标题取整行，编号为空并置位。
        assert!(outline.numbers_missing);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(
            stored.volumes[0].chapters[0].title,
            stored.volumes[0].chapters[0].full_title
        );
        cleanup(&root);
    }

    #[test]
    fn rejects_unparsable_text_and_bad_patterns() {
        let (root, dir) = test_project("parse-bad");
        write_content(&dir, "只有正文没有标题\n第二行\n");
        assert!(parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).is_err());
        write_content(&dir, "第一章 首章\n文\n");
        assert!(parse_txt_in(&dir, "^(unclosed", CHAPTER_PATTERN).is_err());
        assert!(parse_txt_in(&dir, VOLUME_PATTERN, "^(unclosed").is_err());
        // 失败不应留下半成品。
        assert!(!dir.join(CONTENT_NAME).exists());
        let missing = test_project("parse-missing");
        assert!(parse_txt_in(&missing.1, VOLUME_PATTERN, CHAPTER_PATTERN).is_err());
        cleanup(&root);
        cleanup(&missing.0);
    }

    #[test]
    fn parses_numerals_across_scripts() {
        assert_eq!(parse_numeral("100"), Some(100));
        assert_eq!(parse_numeral("  007  "), Some(7));
        assert_eq!(parse_numeral("一"), Some(1));
        assert_eq!(parse_numeral("十二"), Some(12));
        assert_eq!(parse_numeral("一百二十三"), Some(123));
        assert_eq!(parse_numeral("一百二"), Some(120));
        assert_eq!(parse_numeral("两千三"), Some(2300));
        assert_eq!(parse_numeral("一万二千三百四十五"), Some(12345));
        assert_eq!(parse_numeral("壹万贰仟"), Some(12000));
        assert_eq!(parse_numeral("一亿五"), Some(150_000_000));
        // 非标准逐位写法按位直读。
        assert_eq!(parse_numeral("一一零五"), Some(1105));
        assert_eq!(parse_numeral("〇二"), Some(2));
        // 空串、非法与溢出归空。
        assert_eq!(parse_numeral(""), None);
        assert_eq!(parse_numeral("   "), None);
        assert_eq!(parse_numeral("卷首语"), None);
        assert_eq!(parse_numeral("一千零五万亿"), None);
    }

    #[test]
    fn unparsable_number_leaves_null_without_aborting() {
        let (root, dir) = test_project("parse-partial-number");
        // 卷名组捕获到无法解析的文本时仅该卷编号为 None。
        write_content(&dir, "序卷 开篇\n第一章 首章\n文\n");
        let outline = parse_txt_in(&dir, "^(.+)卷.*$", CHAPTER_PATTERN).expect("parse");
        assert!(!outline.numbers_missing);
        assert_eq!(outline.chapter_count, 1);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.volumes[0].number, None);
        assert_eq!(stored.volumes[0].title.as_deref(), Some("序卷 开篇"));
        assert_eq!(stored.volumes[0].chapters[0].number, Some(1));
        cleanup(&root);
    }

    #[test]
    fn collects_gaps_duplicates_and_fallbacks_in_order() {
        let (root, dir) = test_project("parse-issues");
        write_content(
            &dir,
            "第一卷 开篇\n第一章 起\n甲\n第一章 重复\n乙\n第三章 跳号\n丙\n\
             第一卷 又一卷\n第一章 再来\n丁\n第三卷\n第十百千章 杂记\n戊\n",
        );
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert!(!outline.numbers_missing);
        assert!(!outline.titles_missing);
        let kinds: Vec<(IssueKind, Option<u32>)> = outline
            .issues
            .iter()
            .map(|issue| (issue.kind, issue.number))
            .collect();
        assert_eq!(
            kinds,
            vec![
                // 文档顺序：第三卷纯标题回落。
                (IssueKind::MissingTitle, None),
                // 卷号全局判重，章号同卷内判重。
                (IssueKind::DuplicateNumber, Some(1)),
                (IssueKind::DuplicateNumber, Some(1)),
                // 卷号全局断号，章号卷内断号。
                (IssueKind::MissingChapter, Some(2)),
                (IssueKind::MissingChapter, Some(2)),
            ]
        );
        // 缺失卷号无卷宗关联，缺失章号关联所在卷。
        assert_eq!(outline.issues[3].volume, None);
        assert_eq!(outline.issues[4].volume.as_deref(), Some("第一卷 开篇"));
        cleanup(&root);
    }

    #[test]
    fn chapter_without_parsable_number_leaves_null() {
        let (root, dir) = test_project("parse-partial-chapter");
        // 章名组捕获到无法解析的文本时仅该章编号为 None。
        write_content(&dir, "第一卷 开篇\n第序章 杂记\n文\n");
        let outline = parse_txt_in(&dir, VOLUME_PATTERN, "^第(.+)章[：:\\s]*(.*)$").expect("parse");
        assert!(!outline.numbers_missing);
        assert_eq!(outline.issues.len(), 1);
        assert_eq!(outline.issues[0].kind, IssueKind::MissingNumber);
        assert_eq!(outline.issues[0].chapter.as_deref(), Some("第序章 杂记"));
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.volumes[0].chapters[0].number, None);
        cleanup(&root);
    }

    #[test]
    fn patterns_without_title_group_degrade_to_single_notice() {
        let (root, dir) = test_project("parse-no-titles");
        write_content(&dir, "第一卷 开篇\n第一章 起\n甲\n");
        let outline = parse_txt_in(&dir, "^第.+卷.*$", "^第.+章.*$").expect("parse");
        // 数字组与标题组双缺：编号跳过细则，标题跳过逐条。
        assert!(outline.numbers_missing);
        assert!(outline.titles_missing);
        assert_eq!(outline.issues, Vec::new());
        cleanup(&root);
    }

    #[test]
    fn renumbers_volumes_and_chapters_in_document_order() {
        let (root, dir) = test_project("renumber");
        write_content(
            &dir,
            "第一卷 开篇\n第一章 起\n甲\n第一章 重复\n乙\n第三章 跳号\n丙\n",
        );
        let parsed = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        assert_eq!(parsed.issues.len(), 2);
        let renumbered = renumber_in(&dir).expect("renumber");
        // 卷号与章号按文档顺序重写为连续编号。
        assert_eq!(renumbered.volume_count, 1);
        assert_eq!(renumbered.chapter_count, 3);
        let stored: NovelContent = serde_json::from_str(
            &std::fs::read_to_string(dir.join(CONTENT_NAME)).expect("read content"),
        )
        .expect("parse content");
        assert_eq!(stored.volumes[0].number, Some(1));
        assert_eq!(
            stored.volumes[0]
                .chapters
                .iter()
                .map(|c| c.number)
                .collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(3)]
        );
        // 编号类问题消除，标题与正文不动。
        assert_eq!(renumbered.issues, Vec::new());
        assert_eq!(stored.volumes[0].chapters[0].title, "起");
        assert_eq!(stored.volumes[0].chapters[0].lines, vec!["甲".to_string()]);
        cleanup(&root);
    }

    #[test]
    fn renumber_rejects_missing_content() {
        let (root, dir) = test_project("renumber-missing");
        assert!(renumber_in(&dir).is_err());
        assert!(!dir.join(CONTENT_NAME).exists());
        cleanup(&root);
    }

    #[test]
    fn outline_round_trips_and_tolerates_damage() {
        let (root, dir) = test_project("outline");
        // 缺文件回落空。
        assert_eq!(outline_in(&dir), None);
        write_content(&dir, "第一卷 开篇\n第一章 起\n甲\n");
        let parsed = parse_txt_in(&dir, VOLUME_PATTERN, CHAPTER_PATTERN).expect("parse");
        // 落盘后再读与解析返回值一致。
        assert_eq!(outline_in(&dir), Some(parsed));
        // 损坏回落空；旧版无版本门，直接按当前形状读取。
        std::fs::write(dir.join(CONTENT_NAME), "{ broken").expect("corrupt");
        assert_eq!(outline_in(&dir), None);
        std::fs::write(
            dir.join(CONTENT_NAME),
            r#"{"schema_version":0,"volume_pattern":"","chapter_pattern":"","parsed_at":"","preface":[],"volumes":[]}"#,
        )
        .expect("write legacy");
        let legacy = outline_in(&dir).expect("legacy accepted without version gate");
        assert_eq!(legacy.volume_count, 0);
        cleanup(&root);
    }
}
