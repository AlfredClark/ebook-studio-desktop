// 应用更新的全局状态：检查中 / 下载中切页不丢失，关于页与根布局共享同一份状态。
// 进度事件名与 Rust 侧 `cores::updater::APP_UPDATER_PROGRESS_EVENT` 会合，改名需两端同步。
import commands from "#libs/commands";
import { reportCommandFailure } from "#libs/commands/cores";
import type { UpdateInfo } from "#libs/commands/types";
import { m } from "#libs/i18n/paraglide/messages";
import { toast } from "#libs/utils/toast";

/** 下载进度事件名（见 Rust 侧同名常量） */
const PROGRESS_EVENT = "app-updater-progress";

/**
 * 更新流程阶段：`idle` 空闲（尚未检查）/ `checking` 检查中 / `up-to-date` 已是最新 / `available` 有新版
 * / `downloading` 下载中 / `ready` 待重启 / `error` 失败
 */
export type UpdaterPhase =
  "idle" | "checking" | "up-to-date" | "available" | "downloading" | "ready" | "error";

/** 跨页面共享的更新状态；`autoChecked` 保证启动静默检查单会话只跑一次 */
export const updaterState = $state<{
  phase: UpdaterPhase;
  latest: UpdateInfo | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  autoChecked: boolean;
}>({ phase: "idle", latest: null, downloaded: 0, total: null, error: null, autoChecked: false });

let progressListening = false;

/** 订阅下载进度事件；浏览器 / 单测下动态导入失败则警告并允许下次重试 */
async function ensureProgressListener(): Promise<void> {
  if (progressListening) return;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{ downloaded: number; total: number | null }>(PROGRESS_EVENT, (event) => {
      updaterState.downloaded = event.payload.downloaded;
      updaterState.total = event.payload.total;
    });
    progressListening = true;
  } catch (error) {
    // 非 Tauri 环境无事件可订：经统一上报走 plugin-log，下次下载再试
    reportCommandFailure("[updater] progress listener unavailable", error);
  }
}

/** 检查更新；静默模式下仅在“有新版”时提示，发现最新与失败都不打扰 */
export async function checkForUpdate(options?: { silent?: boolean }): Promise<void> {
  if (updaterState.phase === "checking" || updaterState.phase === "downloading") return;
  // 待重启态不被覆盖：记下进入前的相位，成功回调里若之前已下好包则恢复，避免丢包重下
  const prevPhase = updaterState.phase;
  const prevLatest = updaterState.latest;
  updaterState.phase = "checking";
  updaterState.error = null;
  // IPC 永不结算时兜底复位，避免永久卡在 `checking` 挡掉后续检查
  const checkTimer = setTimeout(() => {
    if (updaterState.phase === "checking") {
      updaterState.phase = "error";
      updaterState.error = "check timed out";
      updaterState.autoChecked = false;
      reportCommandFailure("[updater] check timed out", null);
    }
  }, 120_000);
  await commands
    .checkUpdate()
    .success((info) => {
      clearTimeout(checkTimer);
      if (prevPhase === "ready" && prevLatest) {
        updaterState.phase = "ready";
        updaterState.latest = prevLatest;
        return;
      }
      if (!info) {
        updaterState.phase = "up-to-date";
        updaterState.latest = null;
        if (!options?.silent) toast.message(m.updater_up_to_date());
      } else {
        updaterState.phase = "available";
        updaterState.latest = info;
        // 静默检查的唯一打扰：告诉用户去关于页处理，不弹确认框
        if (options?.silent) toast.info(m.updater_update_available());
      }
    })
    .failed((failure) => {
      clearTimeout(checkTimer);
      updaterState.phase = "error";
      updaterState.error = failure.message;
      // 静默失败允许下次重试：重置标记，网络恢复后切配置/手动检查可再跑
      if (options?.silent) {
        updaterState.autoChecked = false;
        reportCommandFailure("[updater] silent check failed", failure);
      } else {
        toast.error(m.updater_check_failed());
      }
    });
}

/** 下载并安装当前已发现的版本；成功后进入待重启，不自动重启 */
export async function downloadAndInstall(): Promise<void> {
  if (updaterState.phase !== "available" || !updaterState.latest) return;
  updaterState.phase = "downloading";
  updaterState.downloaded = 0;
  updaterState.total = null;
  updaterState.error = null;
  await ensureProgressListener();
  await commands
    .downloadAndInstallUpdate()
    .success(() => {
      updaterState.phase = "ready";
    })
    .failed((failure) => {
      updaterState.phase = "error";
      updaterState.error = failure.message;
      toast.error(m.updater_download_failed());
    });
}

/** 重启以完成更新；桌面端命令永不结算（移动端返回错误），调用方不要 `await` */
let restarting = false;
/** 仅测试用：重置重启互斥（模块变量无法经 `updaterState` 复位） */
export function __resetRestartForTests(): void {
  restarting = false;
}
export function restartApp(): void {
  if (updaterState.phase !== "ready" || restarting) return;
  // 成功永不结算故保持互斥防重重启；若 OS 拦截重启而进程未死，需手动结束进程（有意为之）
  restarting = true;
  void commands.restartApp().failed((failure) => {
    // 重启失败才允许再点（如移动端返回错误），成功（永不结算）则保持互斥
    restarting = false;
    reportCommandFailure("[updater] failed to restart", failure);
  });
}

/** 启动静默检查：开着开关且本会话未查过才执行；非 Tauri 环境直接跳过 */
export function maybeAutoCheckForUpdate(enabled: boolean): void {
  if (!enabled || updaterState.autoChecked) return;
  updaterState.autoChecked = true;
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
  void checkForUpdate({ silent: true });
}
