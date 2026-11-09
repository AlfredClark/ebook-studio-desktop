// 漫画尺寸调整的跨页面状态：切页不丢失进度，与转换/压缩作业互斥。
// 进度事件名与 Rust 侧 `features::comic_images::RESIZE_PROGRESS_EVENT` 会合，改名需两端同步。
import commands from "#libs/commands";
import { reportCommandFailure } from "#libs/commands/cores";

/** 尺寸调整进度事件名（见 Rust 侧同名常量）。 */
const RESIZE_PROGRESS_EVENT = "comic-images-resize-progress";

/** 单个项目的尺寸调整作业：运行中 + 已完成 / 总数 + 是否有可撤销的备份（与压缩共用）。 */
export interface ComicResizeJob {
  running: boolean;
  done: number;
  total: number;
  backup: boolean;
}

const idleJob = (): ComicResizeJob => ({ running: false, done: 0, total: 0, backup: false });

/** 按项目 id 记录的尺寸调整作业；切页 / 重挂载不丢失。 */
const resizeJobs = $state<Record<string, ComicResizeJob>>({});

let listening = false;

/** 订阅全局尺寸调整进度（单例，切页不重复订阅）；非 Tauri 环境静默跳过。 */
export function ensureResizeListener(): void {
  if (listening) {
    return;
  }
  listening = true;
  void import("@tauri-apps/api/event").then(async ({ listen }) => {
    try {
      await listen<{ id: string; done_files: number; total_files: number }>(
        RESIZE_PROGRESS_EVENT,
        (event) => {
          resizeJobs[event.payload.id] = {
            running: true,
            done: event.payload.done_files,
            total: event.payload.total_files,
            backup: false,
          };
        },
      );
    } catch (error) {
      listening = false;
      reportCommandFailure("[comic-resize] progress listener unavailable", error);
    }
  });
}

/** 读取某项目的尺寸调整作业快照（渲染期调用建立响应式依赖）。 */
export function resizeJobOf(uuid: string): ComicResizeJob {
  return resizeJobs[uuid] ?? idleJob();
}

/** 标记开始 / 结束（由发起调整的页面维护，卸载后延续照常写入）。 */
export function setResizeRunning(uuid: string, running: boolean): void {
  if (running) {
    const current = resizeJobs[uuid] ?? idleJob();
    current.running = true;
    resizeJobs[uuid] = current;
    return;
  }
  resizeJobs[uuid] = idleJob();
}

/** 回填远端状态：切回页面 / 重挂载时以 Rust 侧注册表为准。 */
export async function refreshResizeStatus(uuid: string): Promise<void> {
  ensureResizeListener();
  try {
    const result = await commands.getComicResizeStatus(uuid).result();
    if (result.status !== "ok") {
      return;
    }
    const { running, done_files, total_files, has_backup } = result.data;
    resizeJobs[uuid] = running
      ? { running: true, done: done_files, total: total_files, backup: false }
      : { running: false, done: 0, total: 0, backup: has_backup };
  } catch (error) {
    reportCommandFailure("[comic-resize] status refresh failed", error);
  }
}
