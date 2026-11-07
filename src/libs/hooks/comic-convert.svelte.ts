// 漫画图片转换的跨页面状态：切页不丢失进度，编辑页与工具面板共享同一份。
// 进度事件名与 Rust 侧 `features::comic_images::CONVERT_PROGRESS_EVENT` 会合，改名需两端同步。
import commands from "#libs/commands";
import { reportCommandFailure } from "#libs/commands/cores";

/** 转换进度事件名（见 Rust 侧同名常量）。 */
const CONVERT_PROGRESS_EVENT = "comic-images-convert-progress";

/** 单个项目的转换作业：运行中 + 已完成 / 总数。 */
export interface ComicConvertJob {
  running: boolean;
  done: number;
  total: number;
}

const idleJob = (): ComicConvertJob => ({ running: false, done: 0, total: 0 });

/** 按项目 id 记录的转换作业；切页 / 重挂载不丢失。 */
const convertJobs = $state<Record<string, ComicConvertJob>>({});

let listening = false;

/** 订阅全局转换进度（单例，切页不重复订阅）；非 Tauri 环境静默跳过。 */
export function ensureConvertListener(): void {
  if (listening) {
    return;
  }
  listening = true;
  void import("@tauri-apps/api/event").then(async ({ listen }) => {
    try {
      await listen<{ id: string; done_files: number; total_files: number }>(
        CONVERT_PROGRESS_EVENT,
        (event) => {
          convertJobs[event.payload.id] = {
            running: true,
            done: event.payload.done_files,
            total: event.payload.total_files,
          };
        },
      );
    } catch (error) {
      listening = false;
      reportCommandFailure("[comic-convert] progress listener unavailable", error);
    }
  });
}

/** 读取某项目的转换作业快照（渲染期调用建立响应式依赖）。 */
export function convertJobOf(uuid: string): ComicConvertJob {
  return convertJobs[uuid] ?? idleJob();
}

/** 标记开始 / 结束（由发起转换的页面维护，卸载后延续照常写入）。 */
export function setConvertRunning(uuid: string, running: boolean): void {
  if (running) {
    const current = convertJobs[uuid] ?? idleJob();
    current.running = true;
    convertJobs[uuid] = current;
    return;
  }
  convertJobs[uuid] = idleJob();
}

/** 回填远端状态：切回页面 / 重挂载时以 Rust 侧注册表为准。 */
export async function refreshConvertStatus(uuid: string): Promise<void> {
  ensureConvertListener();
  try {
    const result = await commands.getComicConvertStatus(uuid).result();
    if (result.status !== "ok") {
      return;
    }
    const { running, done_files, total_files } = result.data;
    convertJobs[uuid] = running
      ? { running: true, done: done_files, total: total_files }
      : idleJob();
  } catch (error) {
    reportCommandFailure("[comic-convert] status refresh failed", error);
  }
}
