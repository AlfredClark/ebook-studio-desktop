// 漫画左栏拖拽 action：SortableJS forceFallback 封装。
// Tauri WebKitGTK 的原生 HTML5 DnD 不可靠（上游 tauri#6695），故强制走库内
// fallback 引擎（pointer/mouse 自研手势），手势生死不经过 webview 的 DnD。
// 拖拽全程由库操作 DOM（幽灵 + 位移动画），只在 onEnd 落点回调一次，调用方
// 按 DOM 顺序提交 state（见 comic-order 的 applyDomOrder）；无效落点库自动
// 播回滚动画，无需快照。
import Sortable from "sortablejs";
import type { Options as SortableFullOptions, SortableEvent } from "sortablejs";

/** 分组：页列表互拖，话列表另起分组天然隔离。 */
export const PAGE_GROUP = "comic-pages";
export const VOLUME_GROUP = "comic-volumes";

/** 落点 DOM 顺序：话 id 序 + 每话页 id（= path）序。 */
export interface ComicDropOrder {
  volumeIds: string[];
  pages: Map<string, string[]>;
}

/** 落点回调：调用方读 DOM 顺序提交。 */
export interface SortableCallbacks {
  onDragStart: (item: HTMLElement) => void;
  onDropEnd: (event: SortableEvent) => void;
}

/** 共享配置：fallback 阈值 6px（点击照常），150ms 位移动画，自带边缘滚动。 */
function baseOptions(
  get: () => SortableCallbacks,
  extra: SortableFullOptions,
): SortableFullOptions {
  return {
    forceFallback: true,
    fallbackOnBody: true,
    fallbackTolerance: 6,
    animation: 150,
    easing: "cubic-bezier(1, 0, 0, 1)",
    scrollSensitivity: 60,
    scrollSpeed: 10,
    emptyInsertThreshold: 12,
    preventOnFilter: true,
    ghostClass: "comic-sortable-ghost",
    chosenClass: "comic-sortable-chosen",
    dragClass: "comic-sortable-drag",
    fallbackClass: "comic-sortable-fallback",
    dataIdAttr: "data-id",
    ...extra,
    onStart: (event) => get().onDragStart(event.item as HTMLElement),
    onEnd: (event) => get().onDropEnd(event),
  };
}

/** 页列表 action：挂到每话展开的 body 上（`data-volume-list` 标记话归属）。 */
export function pageList(node: HTMLElement, params: SortableCallbacks) {
  let current = params;
  const sortable = new Sortable(
    node,
    baseOptions(() => current, {
      group: PAGE_GROUP,
      draggable: "button[data-page-row]",
      filter: "[data-no-drag]",
    }),
  );
  return {
    update: (next: SortableCallbacks) => {
      current = next;
    },
    destroy: () => {
      sortable.destroy();
    },
  };
}

/** 话列表 action：挂到话容器上；拖拽柄限话头按钮，行上按下不触发。 */
export function volumeList(node: HTMLElement, params: SortableCallbacks) {
  let current = params;
  const sortable = new Sortable(
    node,
    baseOptions(() => current, {
      group: VOLUME_GROUP,
      draggable: "section[data-volume-section]",
      handle: "button[data-volume-handle]",
      filter: "[data-no-drag]",
    }),
  );
  return {
    update: (next: SortableCallbacks) => {
      current = next;
    },
    destroy: () => {
      sortable.destroy();
    },
  };
}

/** 读落点 DOM 顺序：话按 section 序，页按各 body 内行序；无 data-id 的跳过。 */
export function readDomOrder(root: HTMLElement): ComicDropOrder {
  const volumeIds: string[] = [];
  for (const section of root.querySelectorAll("[data-volume-section]")) {
    const id = (section as HTMLElement).dataset.id;
    if (id != null && id !== "") {
      volumeIds.push(id);
    }
  }
  const pages = new Map<string, string[]>();
  for (const body of root.querySelectorAll("[data-volume-list]")) {
    const list = body as HTMLElement;
    const volumeId = list.dataset.volumeList;
    if (volumeId == null || volumeId === "") {
      continue;
    }
    const ids: string[] = [];
    for (const child of list.children) {
      if (
        child instanceof HTMLElement &&
        child.matches("button[data-page-row]") &&
        child.dataset.id != null &&
        child.dataset.id !== ""
      ) {
        ids.push(child.dataset.id);
      }
    }
    pages.set(volumeId, ids);
  }
  return { volumeIds, pages };
}
