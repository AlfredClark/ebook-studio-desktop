// 漫画文件步骤左栏纯逻辑：字节数与页码范围展示。
// 后端已按自然排序落盘，前端只做展示格式化，不重排。

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

/** 页码范围补零：宽度取总页数位数，起止同宽对齐（如 205 页取 3 位）。 */
export function formatPaddedPageRange(
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

/** 取 POSIX 路径的文件名：`第1话/001.jpg` 取 `001.jpg`。 */
export function pageFileName(path: string): string {
  const index = path.lastIndexOf("/");
  return index < 0 ? path : path.slice(index + 1);
}
