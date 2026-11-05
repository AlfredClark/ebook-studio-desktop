// 小说文件步骤左栏纯逻辑：卷章正则默认值与字节数展示；最终校验以后端解析命令为准。

/** 卷标题默认表达式：`第X卷` 行首起配，第一捕获组为数字、尾部捕获卷名。 */
export const DEFAULT_VOLUME_PATTERN =
  "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)卷[：:\\s]*(.*)$";
/** 章标题默认表达式：`第X章` 行首起配，第一捕获组为数字、尾部捕获章名。 */
export const DEFAULT_CHAPTER_PATTERN =
  "^第([0-9一二三四五六七八九十百千万零〇两壹贰叁肆伍陆柒捌玖拾佰仟]+)章[：:\\s]*(.*)$";

/** 正则正确性预检：空串视为无效，可编译即通过（最终以 Rust 解析命令为准）。 */
export function isValidPattern(pattern: string): boolean {
  if (pattern.trim() === "") {
    return false;
  }
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

/** 字节数展示：B / KB / MB，KB 以上保留一位小数。 */
export function formatBytes(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

/** 章节范围补零：宽度取总章数位数，起止同宽对齐（如 205 章取 3 位）。 */
export function formatPaddedChapterRange(
  start: number,
  end: number,
  total: number,
): { start: string; end: string } {
  const width = Math.max(String(Math.max(total, 1)).length, 1);
  return {
    start: String(start).padStart(width, "0"),
    end: String(end).padStart(width, "0"),
  };
}
