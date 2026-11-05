// 小说导出步骤页纯逻辑：命名选项合并与表单可用性；落盘规则以后端为准。
import type { ExportItems, ExportNaming, ExportStatus } from "#libs/commands/types";

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
export function canExport(items: ExportItems, status: ExportStatus): boolean {
  return (
    (items.epub && status.has_epub) ||
    (items.txt && status.has_content) ||
    (items.cover && status.has_cover)
  );
}
