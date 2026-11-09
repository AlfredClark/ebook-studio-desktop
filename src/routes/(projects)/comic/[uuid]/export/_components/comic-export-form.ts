// 漫画导出步骤页纯逻辑：命名选项合并、表单可用性与标题格式校验；落盘规则以后端为准。
import type {
  ComicExportItems,
  ComicExportStatus,
  ComicPdfFillMode,
  ComicPdfHorizontalAlignment,
  ComicPdfOptions,
  ComicPdfPageSize,
  ComicPdfVerticalAlignment,
  ComicImagesMode,
  ExportNaming,
  TxtTitleKind,
} from "#libs/commands/types";

const MIN_PDF_DPI = 72;
const MAX_PDF_DPI = 600;
const MAX_PDF_PAGE_SIZE_PT = 14_400;

/** 漫画 PDF 表单字段以字符串接收原始输入。 */
export type ComicPdfFormValues = {
  page_size: ComicPdfPageSize;
  width_px: string;
  height_px: string;
  dpi: string;
  fill_mode: ComicPdfFillMode;
  horizontal_alignment: ComicPdfHorizontalAlignment;
  vertical_alignment: ComicPdfVerticalAlignment;
  include_cover: boolean;
};

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
    (items.cover && status.has_cover) ||
    (items.pdf && status.has_content && status.page_count > 0)
  );
}

/** PDF 设置可用时返回命令参数；固定尺寸必须为正整数且未超过 PDF 页面上限。 */
export function buildComicPdfOptions(values: ComicPdfFormValues): ComicPdfOptions | null {
  const dpi = Number(values.dpi);
  if (!Number.isInteger(dpi) || dpi < MIN_PDF_DPI || dpi > MAX_PDF_DPI) {
    return null;
  }

  let widthPx = 0;
  let heightPx = 0;
  if (values.page_size === "fixed") {
    widthPx = Number(values.width_px);
    heightPx = Number(values.height_px);
    if (
      !Number.isInteger(widthPx) ||
      !Number.isInteger(heightPx) ||
      widthPx <= 0 ||
      heightPx <= 0
    ) {
      return null;
    }
    if (
      (widthPx * 72) / dpi > MAX_PDF_PAGE_SIZE_PT ||
      (heightPx * 72) / dpi > MAX_PDF_PAGE_SIZE_PT
    ) {
      return null;
    }
  }

  return {
    page_size: values.page_size,
    width_px: widthPx,
    height_px: heightPx,
    dpi,
    fill_mode: values.fill_mode,
    horizontal_alignment: values.horizontal_alignment,
    vertical_alignment: values.vertical_alignment,
    include_cover: values.include_cover,
  };
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

/** 自定义模板默认值：切 custom 空模板时预填，话页各一（话不带原名、页为纯数字，与构建页一致）。 */
export const DEFAULT_COMIC_VOLUME_PATTERN = "第#N话";
export const DEFAULT_COMIC_PAGE_PATTERN = "#N";

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
