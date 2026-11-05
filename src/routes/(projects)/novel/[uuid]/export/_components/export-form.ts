// 小说导出步骤页纯逻辑：命名选项合并、表单可用性与 TXT 参数校验；落盘规则以后端为准。
import type {
  ExportItems,
  ExportNaming,
  ExportStatus,
  MetadataField,
  TxtTitleKind,
} from "#libs/commands/types";

/** 命名候选：项目名与书名相同合并为一项（取项目名）。 */
export function namingKinds(projectName: string, bookTitle: string): ExportNaming[] {
  if (projectName === bookTitle) {
    return ["project", "custom"];
  }
  return ["project", "book", "custom"];
}

/** 命名取值：`custom` 由输入框另取，此处回落项目名。 */
export function namingValue(kind: ExportNaming, projectName: string, bookTitle: string): string {
  if (kind === "book") {
    return bookTitle;
  }
  return projectName;
}

/** 自定义名可用：去首尾空白后非空（非法字符由后端按打包口径清洗）。 */
export function isValidCustomName(name: string): boolean {
  return name.trim().length > 0;
}

/** 导出可用：至少勾选一项现存产物。 */
export function canExport(items: ExportItems, status: ExportStatus): boolean {
  return (
    (items.epub && status.has_epub) ||
    (items.txt && status.has_content) ||
    (items.cover && status.has_cover)
  );
}

/** TXT 标题格式候选（固定顺序）。 */
export const TXT_TITLE_KINDS: TxtTitleKind[] = [
  "original",
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 元数据候选（著录序，与后端渲染一致）：书名、作者、语言、系列、卷次、 版权、来源、副标题、主题词、贡献者、出版社、出版时间、简介。 */
export const METADATA_FIELDS: MetadataField[] = [
  "title",
  "creator",
  "language",
  "series",
  "volume",
  "rights",
  "source",
  "alternative",
  "subjects",
  "contributor",
  "publisher",
  "date_published",
  "description",
];

/** 自定义模板可用：去首尾空白后非空。 */
export function isValidTitlePattern(pattern: string): boolean {
  return pattern.trim().length > 0;
}

/** TXT 参数合法：自定义模板非空，元数据开启时至少选一项。 */
export function isValidTxtOptions(
  titleKind: TxtTitleKind,
  titlePattern: string,
  metaEnabled: boolean,
  metaFields: MetadataField[],
): boolean {
  if (titleKind === "custom" && !isValidTitlePattern(titlePattern)) {
    return false;
  }
  if (metaEnabled && metaFields.length === 0) {
    return false;
  }
  return true;
}
