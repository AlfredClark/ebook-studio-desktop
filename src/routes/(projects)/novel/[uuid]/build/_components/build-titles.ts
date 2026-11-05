// 构建标题格式纯逻辑：卷章格式候选、自定义模板校验与默认值；落盘规则以后端为准。
import type { TxtTitleKind } from "#libs/commands/types";

/** 构建标题格式候选（固定顺序，与导出一致）。 */
export const BUILD_TITLE_KINDS: TxtTitleKind[] = [
  "original",
  "arabic",
  "padded",
  "chinese_lower",
  "chinese_upper",
  "custom",
];

/** 自定义模板默认值：切 custom 空模板时预填，卷章各一。 */
export const DEFAULT_BUILD_VOLUME_PATTERN = "第#N卷 #T";
export const DEFAULT_BUILD_CHAPTER_PATTERN = "第#N章 #T";

/** 自定义模板可用：去首尾空白后非空。 */
export function isValidBuildTitlePattern(pattern: string): boolean {
  return pattern.trim().length > 0;
}

/** 构建标题合法：卷章自定义模板各自非空。 */
export function isValidBuildTitles(
  chapterKind: TxtTitleKind,
  chapterPattern: string,
  volumeKind: TxtTitleKind,
  volumePattern: string,
): boolean {
  if (chapterKind === "custom" && !isValidBuildTitlePattern(chapterPattern)) {
    return false;
  }
  if (volumeKind === "custom" && !isValidBuildTitlePattern(volumePattern)) {
    return false;
  }
  return true;
}
