// 项目入口来源：用户从首页还是项目页进入项目内部，返回按钮据此回跳。
// 存 localStorage（`app.` 前缀，新增键统一加前缀）；无痕/禁用存储时回落项目页。

/** 入口来源：首页或项目页。 */
export type ProjectEntry = "/" | "/projects";

/** 持久化键：返回后刷新仍记得上次入口。 */
const ENTRY_KEY = "app.projects.entryFrom";

/** 读取入口来源；缺失或非法回落项目页。 */
export function readProjectEntry(): ProjectEntry {
  try {
    const stored = localStorage.getItem(ENTRY_KEY);
    if (stored === "/" || stored === "/projects") {
      return stored;
    }
  } catch {
    // 存储不可用时视为无存量
  }
  return "/projects";
}

/** 记录入口来源；失败静默跳过，不影响主流程。 */
export function storeProjectEntry(entry: ProjectEntry): void {
  try {
    localStorage.setItem(ENTRY_KEY, entry);
  } catch {
    // 存储不可用时跳过持久化
  }
}
