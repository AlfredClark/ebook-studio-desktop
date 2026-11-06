// 漫画导出步骤页纯逻辑：命名选项合并、表单可用性与标题格式校验；落盘规则以后端为准。
import type {
  ComicExportItems,
  ComicExportStatus,
  ComicImagesMode,
  ExportNaming,
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
export function canExport(items: ComicExportItems, status: ComicExportStatus): boolean {
  return (
    (items.epub && status.has_epub) ||
    (items.images && status.has_content) ||
    (items.cover && status.has_cover)
  );
}

/** 话标题格式候选（固定顺序）：含原文。 */
export const COMIC_VOLUME_TITLE_KINDS: TxtTitleKind[] = [
  "original",
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 页标题格式候选（固定顺序）：页无原文格式。 */
export const COMIC_PAGE_TITLE_KINDS: TxtTitleKind[] = [
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 自定义模板可用：去首尾空白后非空。 */
export function isValidTitlePattern(pattern: string): boolean {
  return pattern.trim().length > 0;
}

/** 原图标题格式合法：话页自定义模板各自非空。 */
export function isValidImageTitles(
  pageKind: TxtTitleKind,
  pagePattern: string,
  volumeKind: TxtTitleKind,
  volumePattern: string,
): boolean {
  if (pageKind === "custom" && !isValidTitlePattern(pagePattern)) {
    return false;
  }
  if (volumeKind === "custom" && !isValidTitlePattern(volumePattern)) {
    return false;
  }
  return true;
}

/** 原图导出形态候选（固定顺序）：文件夹优先。 */
export const COMIC_IMAGES_MODES: ComicImagesMode[] = ["folder", "zip"];
