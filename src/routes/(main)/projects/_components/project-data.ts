// 项目页纯逻辑：筛选/排序/多选三态/本地移除；数据源自后端命令，禁放演示数据。
// 后端契约变更时同步此处字段；纯函数可直接单测。

import type { ProjectInfo, ProjectType } from "#libs/commands/types";

/** 排序键：更新/创建时间各倒序正序，名称升序降序。 */
export type SortKey =
  "updated_desc" | "updated_asc" | "created_desc" | "created_asc" | "name_asc" | "name_desc";

/** 分类筛选：全部或单类型。 */
export type TypeFilter = "all" | ProjectType;

/** 批量操作：暂定仅批量删除，后续补充。 */
export type BatchAction = "delete";

/** 可见列表查询条件：locale 仅用于名称排序，缺省走默认语言。 */
export interface ProjectQuery {
  query: string;
  sortKey: SortKey;
  filter: TypeFilter;
  locale?: string;
}

/** 可见列表：先类型筛选，再名称子串（大小写不敏感），最后排序；不改原数组。 */
export function filterAndSortProjects(projects: ProjectInfo[], query: ProjectQuery): ProjectInfo[] {
  const keyword = query.query.trim().toLowerCase();
  const filtered = projects.filter(
    (project) =>
      (query.filter === "all" || project.type === query.filter) &&
      (keyword === "" || project.name.toLowerCase().includes(keyword)),
  );
  const sorted = [...filtered];
  switch (query.sortKey) {
    case "updated_asc":
      sorted.sort((a, b) => a.updated_at.localeCompare(b.updated_at));
      break;
    case "created_desc":
      sorted.sort((a, b) => b.created_at.localeCompare(a.created_at));
      break;
    case "created_asc":
      sorted.sort((a, b) => a.created_at.localeCompare(b.created_at));
      break;
    case "name_asc":
      sorted.sort((a, b) => a.name.localeCompare(b.name, query.locale));
      break;
    case "name_desc":
      sorted.sort((a, b) => b.name.localeCompare(a.name, query.locale));
      break;
    default:
      sorted.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
      break;
  }
  return sorted;
}

/** 时间转本地展示：年月日时分秒显式齐备，保证行内精度到秒；locale 缺省走默认语言。 */
export function formatDateTime(iso: string, locale?: string): string {
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    hourCycle: "h23",
  }).format(new Date(iso));
}

/** 全选框三态：按可见列表计算；空列表视为未选中且禁用。 */
export function computeSelectAllState(
  visibleIds: string[],
  checkedIds: string[],
): { checked: boolean; indeterminate: boolean; disabled: boolean } {
  if (visibleIds.length === 0) {
    return { checked: false, indeterminate: false, disabled: true };
  }
  const count = visibleIds.filter((id) => checkedIds.includes(id)).length;
  return {
    checked: count === visibleIds.length,
    indeterminate: count > 0 && count < visibleIds.length,
    disabled: false,
  };
}

/** 切换单个勾选：返回新数组，不改原数组。 */
export function toggleCheckedId(checkedIds: string[], id: string, checked: boolean): string[] {
  if (checked) {
    return checkedIds.includes(id) ? checkedIds : [...checkedIds, id];
  }
  return checkedIds.filter((current) => current !== id);
}

/** 全选/全不选：仅作用于可见列表，保留不可见项的勾选。 */
export function toggleAllChecked(
  checkedIds: string[],
  visibleIds: string[],
  checked: boolean,
): string[] {
  if (checked) {
    return [...new Set([...checkedIds, ...visibleIds])];
  }
  const visible = new Set(visibleIds);
  return checkedIds.filter((id) => !visible.has(id));
}

/** 本地移除项目：后端删除成功后同步本地态，不改原数组。 */
export function removeProjects(projects: ProjectInfo[], ids: string[]): ProjectInfo[] {
  const removed = new Set(ids);
  return projects.filter((project) => !removed.has(project.id));
}
