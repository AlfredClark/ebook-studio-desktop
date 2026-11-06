// 构建文件辅助：语言判定与默认展开规则（哑逻辑，可单测）。
// 与小说构建同口径：opf 与 xhtml 按 XML 高亮。

/** 按扩展名判定编辑器语言：opf 与 xhtml 按 XML 高亮。 */
export function langOf(path: string): "xml" | "css" | "text" {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "xml" || ext === "xhtml" || ext === "opf") {
    return "xml";
  }
  if (ext === "css") {
    return "css";
  }
  return "text";
}

/** 是否图片文件：树中换图标，二进制不预览。 */
export function isImageFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return ["jpg", "jpeg", "png", "webp", "gif"].includes(ext);
}

/** 图片大小展示：B / KB / MB，KB 以上保留一位小数。 */
export function formatImageSize(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

/** 默认展开：仅顶层 `EPUB/` 与 `META-INF/`，其余（含嵌套子目录）收起。 */
export function isExpandedByDefault(path: string): boolean {
  return path === "EPUB" || path === "META-INF";
}
