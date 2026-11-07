// 漫画编辑左栏纯逻辑：工作话模型与跨话移动；鼠标拖拽与键盘重排共用同一纯函数。
// 工作话与后端 content.json 话序同源，标题由落盘保持、只动页序。

/** 工作话：id 仅内存排序与 key 用（派生时按序分配，新建时随机），不落盘；标题沿落盘，页为相对 images/ 的有序路径。 */
export interface WorkingVolume {
  id: string;
  title: string | null;
  pages: string[];
}

/** 页引用：话下标 + 话内下标，与工作数组同源。 */
export interface PageRef {
  volume: number;
  index: number;
}

/** 放置位置：相对目标项之前 / 之后，或话末追加。 */
export type DropEdge = "before" | "after" | "append";

/** 放置目标：话下标 + 参照下标（append 时省略）+ 边。 */
export interface DropTarget {
  volume: number;
  index?: number;
  edge: DropEdge;
}

/** 由识别大纲派生工作话（深拷贝，拖拽只改本地，落盘前不碰 saved）。id 按派生序分配，排序时跟随话走。 */
export function toWorking(
  volumes: { title: string | null; pages: { path: string }[] }[],
): WorkingVolume[] {
  return volumes.map((volume, index) => ({
    id: `volume-${index}`,
    title: volume.title,
    pages: volume.pages.map((page) => page.path),
  }));
}

/** 深拷贝工作话：拖拽开始时拍快照，取消回滚用。 */
export function cloneVolumes(volumes: WorkingVolume[]): WorkingVolume[] {
  return volumes.map((volume) => ({ ...volume, pages: [...volume.pages] }));
}

/** 新建空话：id 随机分配避免与派生 id 碰撞；标题由调用方修边后传入。 */
export function createVolume(title: string): WorkingVolume {
  return { id: crypto.randomUUID(), title, pages: [] };
}

/** 工作话比对：脏态判定用，只比标题与页序，忽略内存 id。 */
export function sameOrder(a: WorkingVolume[], b: WorkingVolume[]): boolean {
  const orderOf = (volumes: WorkingVolume[]): { title: string | null; pages: string[] }[] =>
    volumes.map((volume) => ({ title: volume.title, pages: volume.pages }));
  return JSON.stringify(orderOf(a)) === JSON.stringify(orderOf(b));
}

/** 定位页：按 path 查找，找不到回 null。 */
export function findPage(volumes: WorkingVolume[], path: string): PageRef | null {
  for (const [volume, entry] of volumes.entries()) {
    const index = entry.pages.indexOf(path);
    if (index >= 0) {
      return { volume, index };
    }
  }
  return null;
}

/** 展平：全局顺序的 path 清单（上一页 / 下一页导航用）。 */
export function flattenPages(volumes: WorkingVolume[]): string[] {
  return volumes.flatMap((volume) => volume.pages);
}

/**
 * 按落点 DOM 顺序重建工作话（Sortable 落点唯一入口；键盘重排仍走 movePage/moveVolume）。 话按 volumeIds
 * 排（标题跟话走），页按各话 id 表排；未知 path 丢弃， DOM 漏掉的已知 path 缀回原话末尾（库异常时也不丢数据）。
 */
export function applyDomOrder(
  working: WorkingVolume[],
  volumeIds: string[],
  pageOrders: Map<string, string[]>,
): WorkingVolume[] {
  const byId = new Map(working.map((volume) => [volume.id, volume]));
  const ordered = volumeIds
    .map((id) => byId.get(id))
    .filter((volume): volume is WorkingVolume => volume != null);
  for (const volume of working) {
    if (!volumeIds.includes(volume.id)) {
      ordered.push(volume);
    }
  }
  const known = new Set(working.flatMap((volume) => volume.pages));
  const seen = new Set<string>();
  const next = ordered.map((volume) => {
    const ids = pageOrders.get(volume.id) ?? volume.pages;
    const pages = ids.filter((path) => {
      if (!known.has(path) || seen.has(path)) {
        return false;
      }
      seen.add(path);
      return true;
    });
    return { ...volume, pages };
  });
  const missing = working.flatMap((volume) => volume.pages).filter((path) => !seen.has(path));
  if (missing.length > 0) {
    const merged = new Map(next.map((volume) => [volume.id, volume]));
    for (const volume of working) {
      const rest = missing.filter((path) => volume.pages.includes(path));
      if (rest.length > 0) {
        merged.get(volume.id)?.pages.push(...rest);
      }
    }
  }
  return next;
}

/** 话移动目标：参照话下标 + 边（话头上下半区）。 */
export interface VolumeMoveTarget {
  index: number;
  edge: "before" | "after";
}

/** 话标题长度上限：与后端 `MAX_VOLUME_TITLE_CHARS` 同口径。 */
export const MAX_VOLUME_TITLE_CHARS = 50;

/** 话标题校验失败原因：映射 i18n 提示用。 */
export type VolumeTitleError = "empty" | "duplicate" | "too_long";

/** 校验话标题：修边后非空、不超长、与其它话不重名（`taken` 为其它话修边后标题）。 */
export function validateVolumeTitle(title: string, taken: string[]): VolumeTitleError | null {
  const text = title.trim();
  if (text === "") {
    return "empty";
  }
  if (text.length > MAX_VOLUME_TITLE_CHARS) {
    return "too_long";
  }
  if (taken.map((entry) => entry.trim()).includes(text)) {
    return "duplicate";
  }
  return null;
}

/**
 * 移动话：取出 from 话按 to 插入；下标越界钳制；from 非法返回原数组。 to.index 为移动前参照话下标；取出点在参照点之前时前移一位。
 * 调用方保证参照话不是被拖话自身。
 */
export function moveVolume(
  volumes: WorkingVolume[],
  from: number,
  to: VolumeMoveTarget,
): WorkingVolume[] {
  if (from < 0 || from >= volumes.length) {
    return volumes;
  }
  if (to.index < 0 || to.index >= volumes.length) {
    return volumes;
  }
  const next = volumes.map((volume) => ({ ...volume, pages: [...volume.pages] }));
  const [entry] = next.splice(from, 1);
  let at = Math.max(0, Math.min(to.index, next.length));
  if (from < to.index) {
    at -= 1;
  }
  if (to.edge === "after") {
    at += 1;
  }
  at = Math.max(0, Math.min(at, next.length));
  next.splice(at, 0, entry);
  return next;
}

/**
 * 移动页：取出 from 页按 to 插入；下标越界钳制；from 非法直接返回原数组。 to.index
 * 为移动前参照项下标；同话且取出点在参照点之前时前移一位。 调用方保证参照项不是被拖页自身（对自身 before/after 应视为无放置）。
 */
export function movePage(volumes: WorkingVolume[], from: PageRef, to: DropTarget): WorkingVolume[] {
  const source = volumes[from.volume];
  if (source == null || from.index < 0 || from.index >= source.pages.length) {
    return volumes;
  }
  if (to.volume < 0 || to.volume >= volumes.length) {
    return volumes;
  }
  const next = volumes.map((volume) => ({ ...volume, pages: [...volume.pages] }));
  const [path] = next[from.volume].pages.splice(from.index, 1);
  const target = next[to.volume].pages;
  if (to.edge === "append" || to.index == null) {
    target.push(path);
    return next;
  }
  let at = Math.max(0, Math.min(to.index, target.length));
  if (to.volume === from.volume && from.index < to.index) {
    at -= 1;
  }
  if (to.edge === "after") {
    at += 1;
  }
  at = Math.max(0, Math.min(at, target.length));
  target.splice(at, 0, path);
  return next;
}
