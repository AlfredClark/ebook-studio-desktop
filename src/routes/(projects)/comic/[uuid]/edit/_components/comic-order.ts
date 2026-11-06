// 漫画编辑左栏纯逻辑：工作卷模型与跨卷移动；鼠标拖拽与键盘重排共用同一纯函数。
// 工作卷与后端 content.json 卷序同源，标题由落盘保持、只动页序。

/** 工作卷：标题沿落盘，页为相对 images/ 的有序路径。 */
export interface WorkingVolume {
  title: string | null;
  pages: string[];
}

/** 页引用：卷下标 + 卷内下标，与工作数组同源。 */
export interface PageRef {
  volume: number;
  index: number;
}

/** 放置位置：相对目标项之前 / 之后，或卷末追加。 */
export type DropEdge = "before" | "after" | "append";

/** 放置目标：卷下标 + 参照下标（append 时省略）+ 边。 */
export interface DropTarget {
  volume: number;
  index?: number;
  edge: DropEdge;
}

/** 由识别大纲派生工作卷（深拷贝，拖拽只改本地，落盘前不碰 saved）。 */
export function toWorking(
  volumes: { title: string | null; pages: { path: string }[] }[],
): WorkingVolume[] {
  return volumes.map((volume) => ({
    title: volume.title,
    pages: volume.pages.map((page) => page.path),
  }));
}

/** 工作卷比对：脏态判定用，序列化直比即可（同一派生口径）。 */
export function sameOrder(a: WorkingVolume[], b: WorkingVolume[]): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
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

/** 卷移动目标：参照卷下标 + 边（卷头上下半区）。 */
export interface VolumeMoveTarget {
  index: number;
  edge: "before" | "after";
}

/** 卷标题长度上限：与后端 `MAX_VOLUME_TITLE_CHARS` 同口径。 */
export const MAX_VOLUME_TITLE_CHARS = 50;

/** 卷标题校验失败原因：映射 i18n 提示用。 */
export type VolumeTitleError = "empty" | "duplicate" | "too_long";

/** 校验卷标题：修边后非空、不超长、与其它卷不重名（`taken` 为其它卷修边后标题）。 */
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
 * 移动卷：取出 from 卷按 to 插入；下标越界钳制；from 非法返回原数组。 to.index 为移动前参照卷下标；取出点在参照点之前时前移一位。
 * 调用方保证参照卷不是被拖卷自身。
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
  const next = volumes.map((volume) => ({ title: volume.title, pages: [...volume.pages] }));
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
 * 为移动前参照项下标；同卷且取出点在参照点之前时前移一位。 调用方保证参照项不是被拖页自身（对自身 before/after 应视为无放置）。
 */
export function movePage(volumes: WorkingVolume[], from: PageRef, to: DropTarget): WorkingVolume[] {
  const source = volumes[from.volume];
  if (source == null || from.index < 0 || from.index >= source.pages.length) {
    return volumes;
  }
  if (to.volume < 0 || to.volume >= volumes.length) {
    return volumes;
  }
  const next = volumes.map((volume) => ({ title: volume.title, pages: [...volume.pages] }));
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
