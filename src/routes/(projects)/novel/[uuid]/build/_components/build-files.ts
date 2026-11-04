// 构建文件辅助：语言判定与首文件查找（哑逻辑，可单测）。
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
