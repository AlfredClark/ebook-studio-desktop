// 漫画构建标题格式纯逻辑：话格式 6 种、页格式 5 种（页无原文），
// 自定义模板校验与默认值；落盘规则以后端为准。
import type { TxtTitleKind } from "#libs/commands/types";

/** 话标题格式候选（固定顺序，与导出一致）。 */
export const COMIC_VOLUME_TITLE_KINDS: TxtTitleKind[] = [
  "original",
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 页标题格式候选：页无原文格式，其余与话一致。 */
export const COMIC_PAGE_TITLE_KINDS: TxtTitleKind[] = [
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 自定义模板默认值：切 custom 空模板时预填，话页各一。 */
export const DEFAULT_COMIC_VOLUME_PATTERN = "第#N话 #T";
export const DEFAULT_COMIC_PAGE_PATTERN = "第#N页";

/** 自定义模板可用：去首尾空白后非空。 */
export function isValidComicTitlePattern(pattern: string): boolean {
  return pattern.trim().length > 0;
}

/** 漫画构建标题合法：话页自定义模板各自非空。 */
export function isValidComicTitles(
  pageKind: TxtTitleKind,
  pagePattern: string,
  volumeKind: TxtTitleKind,
  volumePattern: string,
): boolean {
  if (pageKind === "custom" && !isValidComicTitlePattern(pagePattern)) {
    return false;
  }
  if (volumeKind === "custom" && !isValidComicTitlePattern(volumePattern)) {
    return false;
  }
  return true;
}
