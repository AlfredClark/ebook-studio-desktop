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
  MetadataField,
  TxtTitleKind,
} from "#libs/commands/types";

const MIN_PDF_DPI = 72;
const MAX_PDF_DPI = 600;
const MAX_PDF_PAGE_SIZE_PT = 14_400;

/** PDF 文档元数据字段：有落点的 5 项（著录序），开启时整体发送，为空跳过。 */
export const PDF_METADATA_FIELDS: MetadataField[] = [
  "title",
  "creator",
  "subjects",
  "description",
  "language",
];

/** 漫画 PDF 固定尺寸默认值：3:4 竖构图，300DPI 下约 446×595pt。 */
export const DEFAULT_PDF_WIDTH_PX = "1860";
export const DEFAULT_PDF_HEIGHT_PX = "2480";

/** 漫画 PDF 宽高比例锁定：`free` 为不锁定，其余为 `宽:高`。 */
export type ComicPdfAspectLock = "free" | "1:1" | "3:4" | "4:3" | "2:3" | "3:2" | "9:16" | "16:9";

/** 比例锁定候选项（固定顺序）：不锁定首位。 */
export const PDF_ASPECT_LOCKS: ComicPdfAspectLock[] = [
  "free",
  "1:1",
  "3:4",
  "4:3",
  "2:3",
  "3:2",
  "9:16",
  "16:9",
];

/** 解析比例锁定为宽高比；不锁定或非法返回空值。 */
export function parseAspectRatio(lock: ComicPdfAspectLock): [number, number] | null {
  if (lock === "free") {
    return null;
  }
  const [width, height] = lock.split(":").map(Number);
  if (
    width === undefined ||
    height === undefined ||
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width <= 0 ||
    height <= 0
  ) {
    return null;
  }
  return [width, height];
}

/** 按锁定比例由宽度换算高度；不锁定或宽度非法时返回空字符串。 */
export function linkedHeightForWidth(widthPx: string, lock: ComicPdfAspectLock): string {
  const ratio = parseAspectRatio(lock);
  const width = Number(widthPx);
  if (ratio === null || !Number.isFinite(width) || width <= 0) {
    return "";
  }
  return String(Math.max(1, Math.round((width * ratio[1]) / ratio[0])));
}

/** 按锁定比例由高度换算宽度；不锁定或高度非法时返回空字符串。 */
export function linkedWidthForHeight(heightPx: string, lock: ComicPdfAspectLock): string {
  const ratio = parseAspectRatio(lock);
  const height = Number(heightPx);
  if (ratio === null || !Number.isFinite(height) || height <= 0) {
    return "";
  }
  return String(Math.max(1, Math.round((height * ratio[0]) / ratio[1])));
}

/** 漫画 PDF 表单字段以字符串接收原始输入；话标题格式仅多话书签使用。 */
export type ComicPdfFormValues = {
  page_size: ComicPdfPageSize;
  width_px: string;
  height_px: string;
  dpi: string;
  fill_mode: ComicPdfFillMode;
  horizontal_alignment: ComicPdfHorizontalAlignment;
  vertical_alignment: ComicPdfVerticalAlignment;
  include_cover: boolean;
  volume_kind: TxtTitleKind;
  volume_pattern: string;
  metadata: MetadataField[];
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
    volume_kind: values.volume_kind,
    volume_pattern: values.volume_pattern,
    metadata: values.metadata,
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

/** PDF 话标题格式合法：仅多话书签使用，单话直接通过。 */
export function isValidPdfVolumeTitle(
  volumeKind: TxtTitleKind,
  volumePattern: string,
  isEpisodeBook: boolean,
): boolean {
  if (!isEpisodeBook) {
    return true;
  }
  if (volumeKind === "custom") {
    return isValidTitlePattern(volumePattern);
  }
  return true;
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
