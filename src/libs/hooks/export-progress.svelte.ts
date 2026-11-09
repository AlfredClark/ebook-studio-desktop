// 导出进度跨页面状态：切页不丢失进度，漫画 / 小说导出页共用同一份注册表；
// 事件名由调用方传入（两端各一），全局单例订阅，改名需两端同步。
import { SvelteSet } from "svelte/reactivity";
import type { ComicExportState } from "#libs/commands/types";

/** 导出作业状态：运行中 + 已完成的文件或 PDF 页处理单元数。 */
export type ExportJob = ComicExportState;

const idleJob = (): ComicExportState => ({ running: false, done_units: 0, total_units: 0 });

/** 按项目 id 记录的导出作业；切页 / 重挂载不丢失（comic 与 novel 共用一张表，id 全局唯一）。 */
const exportJobs = $state<Record<string, ComicExportState>>({});

/** 已订阅的事件名：每种事件只订阅一次。 */
const listenedEvents = new SvelteSet<string>();

/** 订阅某类导出进度事件；非 Tauri 环境静默跳过，失败时允许下次重试。 */
export function ensureExportListener(eventName: string): void {
  if (listenedEvents.has(eventName)) {
    return;
  }
  listenedEvents.add(eventName);
  import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<{
        id: string;
        done_units: number;
        total_units: number;
      }>(eventName, (event) => {
        exportJobs[event.payload.id] = {
          running: true,
          done_units: event.payload.done_units,
          total_units: event.payload.total_units,
        };
      }),
    )
    .catch(() => {
      listenedEvents.delete(eventName);
    });
}

/** 读取某项目的导出作业快照（渲染期调用建立响应式依赖）。 */
export function exportJobOf(uuid: string): ComicExportState {
  return exportJobs[uuid] ?? idleJob();
}

/** 标记开始 / 结束（由发起导出的页面维护，卸载后延续照常写入）。 */
export function setExportRunning(uuid: string, running: boolean): void {
  if (running) {
    const current = exportJobs[uuid] ?? idleJob();
    current.running = true;
    exportJobs[uuid] = current;
    return;
  }
  exportJobs[uuid] = idleJob();
}

/** 回填远端状态：切回页面 / 重挂载时以 Rust 侧注册表为准；查询失败保持本地。 */
export async function refreshExportStatus(
  uuid: string,
  query: () => Promise<ComicExportState | null>,
): Promise<void> {
  const status = await query().catch(() => null);
  if (status == null) {
    return;
  }
  exportJobs[uuid] = status.running ? status : idleJob();
}
