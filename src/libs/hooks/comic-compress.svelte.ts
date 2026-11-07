// 漫画图片压缩的跨页面状态：切页不丢失进度，与转换作业互斥。
// 进度事件名与 Rust 侧 `features::comic_images::COMPRESS_PROGRESS_EVENT` 会合，改名需两端同步。
import commands from "#libs/commands";
import { reportCommandFailure } from "#libs/commands/cores";

/** 压缩进度事件名（见 Rust 侧同名常量）。 */
const COMPRESS_PROGRESS_EVENT = "comic-images-compress-progress";

/** 单个项目的压缩作业：运行中 + 已完成 / 总数。 */
export interface ComicCompressJob {
  running: boolean;
  done: number;
  total: number;
}

const idleJob = (): ComicCompressJob => ({ running: false, done: 0, total: 0 });

/** 按项目 id 记录的压缩作业；切页 / 重挂载不丢失。 */
const compressJobs = $state<Record<string, ComicCompressJob>>({});

let listening = false;

/** 订阅全局压缩进度（单例，切页不重复订阅）；非 Tauri 环境静默跳过。 */
export function ensureCompressListener(): void {
  if (listening) {
    return;
  }
  listening = true;
  void import("@tauri-apps/api/event").then(async ({ listen }) => {
    try {
      await listen<{ id: string; done_files: number; total_files: number }>(
        COMPRESS_PROGRESS_EVENT,
        (event) => {
          compressJobs[event.payload.id] = {
            running: true,
            done: event.payload.done_files,
            total: event.payload.total_files,
          };
        },
      );
    } catch (error) {
      listening = false;
      reportCommandFailure("[comic-compress] progress listener unavailable", error);
    }
  });
}

/** 读取某项目的压缩作业快照（渲染期调用建立响应式依赖）。 */
export function compressJobOf(uuid: string): ComicCompressJob {
  return compressJobs[uuid] ?? idleJob();
}

/** 标记开始 / 结束（由发起压缩的页面维护，卸载后延续照常写入）。 */
export function setCompressRunning(uuid: string, running: boolean): void {
  if (running) {
    const current = compressJobs[uuid] ?? idleJob();
    current.running = true;
    compressJobs[uuid] = current;
    return;
  }
  compressJobs[uuid] = idleJob();
}

/** 回填远端状态：切回页面 / 重挂载时以 Rust 侧注册表为准。 */
export async function refreshCompressStatus(uuid: string): Promise<void> {
  ensureCompressListener();
  try {
    const result = await commands.getComicCompressStatus(uuid).result();
    if (result.status !== "ok") {
      return;
    }
    const { running, done_files, total_files } = result.data;
    compressJobs[uuid] = running
      ? { running: true, done: done_files, total: total_files }
      : idleJob();
  } catch (error) {
    reportCommandFailure("[comic-compress] status refresh failed", error);
  }
}
