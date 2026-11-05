// 构建文件辅助：语言判定、首文件查找与默认展开规则（哑逻辑，可单测）。
import type { BuildFileNode } from "#libs/commands/types";

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

/** 深度优先首个文件路径：构建后默认选中用。 */
export function firstFilePath(nodes: BuildFileNode[]): string | null {
  for (const node of nodes) {
    if (node.children == null) {
      return node.path;
    }
    const found = firstFilePath(node.children);
    if (found != null) {
      return found;
    }
  }
  return null;
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
