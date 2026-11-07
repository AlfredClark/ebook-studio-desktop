// 打包进度跨页面状态：切页不丢失进度，漫画 / 小说打包页共享同一份注册表；
// 事件名与 Rust 侧 `features::novel_package` 的两份常量会合，改名需两端同步。
// 状态形状复用 specta 生成的 `PackagingStatus`，改名需两端同步。
import { SvelteSet } from "svelte/reactivity";
import type { PackagingStatus } from "#libs/commands/types";

/** 打包作业：跨页面共享，与 Rust 侧注册表同构。 */
export type PackagingJob = PackagingStatus;

const idleJob = (): PackagingStatus => ({
  running: false,
  stage: null,
  done_files: 0,
  total_files: 0,
});

/** 按项目 id 记录的打包作业；切页 / 重挂载不丢失（comic 与 novel 共用一张表，id 全局唯一）。 */
const packagingJobs = $state<Record<string, PackagingStatus>>({});

/** 已订阅的事件名：每种事件只订阅一次。 */
const listenedEvents = new SvelteSet<string>();

/** 订阅某类打包进度事件；非 Tauri 环境静默跳过，失败时允许下次重试。 */
export function ensurePackagingListener(eventName: string): void {
  if (listenedEvents.has(eventName)) {
    return;
  }
  listenedEvents.add(eventName);
  import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<{
        id: string;
        stage: "zip" | "validate";
        done_files: number;
        total_files: number;
      }>(eventName, (event) => {
        packagingJobs[event.payload.id] = {
          running: true,
          stage: event.payload.stage,
          done_files: event.payload.done_files,
          total_files: event.payload.total_files,
        };
      }),
    )
    .catch(() => {
      listenedEvents.delete(eventName);
    });
}

/** 读取某项目的打包作业快照（渲染期调用建立响应式依赖）。 */
export function packagingJobOf(uuid: string): PackagingStatus {
  return packagingJobs[uuid] ?? idleJob();
}

/** 标记开始 / 结束（由发起打包的页面维护，卸载后延续照常写入）。 */
export function setPackagingRunning(uuid: string, running: boolean): void {
  if (running) {
    const current = packagingJobs[uuid] ?? idleJob();
    current.running = true;
    packagingJobs[uuid] = current;
    return;
  }
  packagingJobs[uuid] = idleJob();
}

/** 回填远端状态：切回页面 / 重挂载时以 Rust 侧注册表为准；查询失败保持本地。 */
export async function refreshPackagingStatus(
  uuid: string,
  query: () => Promise<PackagingStatus | null>,
): Promise<void> {
  const status = await query().catch(() => null);
  if (status == null) {
    return;
  }
  packagingJobs[uuid] = status.running ? status : idleJob();
}
