<script lang="ts">
  // 根布局只负责全局能力：注入全局样式，并用主题监听器、Toast 挂载点与错误边界包裹全部页面。
  // 启动水合与界面语言对齐在此完成（挂载期后台执行，不阻断首帧）。
  // 布局容器下沉到 (main) 分组，特殊页面另起分组即可绕开布局。
  // 窗口关闭拦截也在此统一处理：按后端关闭行为分流（弹窗确认 / 藏窗口 / 真退出）。
  import { ModeWatcher } from "mode-watcher";
  import { untrack } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Snippet } from "svelte";
  import { initAppearance } from "#libs/hooks/appearance.svelte";
  import { configState, hydrateConfig } from "#libs/hooks/config.svelte";
  import { initDeepLinks } from "#libs/hooks/deep-link.svelte";
  import { maybeAutoCheckForUpdate } from "#libs/hooks/updater.svelte";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import ErrorBoundary from "#components/common/error-boundary.svelte";
  import { Toaster } from "#components/shadcn-svelte/sonner";
  import commands from "#libs/commands";
  import { reportCommandFailure } from "#libs/commands/cores";
  import { alignLocale } from "#libs/i18n/locale.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import "./layout.css";

  const { children }: { children: Snippet } = $props();

  // 关闭确认弹窗开关；确认即真退出，取消仅关弹窗（窗口早已被 preventDefault 留住）
  let confirmCloseOpen = $state(false);

  // 外观首帧前对齐：读本地偏好并写入根变量，缺失时样式表默认值兜底，不阻断首帧
  // untrack 切断订阅：init 内读写同一批外观状态，被跟踪即形成“改动→重初始化”回路，
  // 未落盘的拖拽预览会被存量覆盖导致控件冻住
  $effect.pre(() => {
    untrack(() => initAppearance());
  });

  // 启动水合：以后端 config.json 为权威，水合完成后把前端语言对齐到它
  // （经 libs/i18n/locale.svelte.ts 的响应式桥 alignLocale 对齐：不整页重载，已渲染文案原地重渲染；
  // 水合失败则回落 Paraglide 本地策略，不阻断首帧；
  // 刻意不在 +layout.ts 的 load() 里做：Tauri invoke 底层走 window.fetch，SvelteKit
  // 会在 load 期间告警让用传给 load 的 fetch，而 invoke 接不进 event.fetch；
  // 本项目是纯 SPA，挂载期后台执行与原来 load 里 fire-and-forget 时序等价）
  // 本 effect 同步体内不读 configState（只在异步水合后读），不会订阅成跟踪回路
  $effect(() => {
    void hydrateInBackground();
  });

  /** 后台水合：与首帧渲染并发，完成后对齐语言；失败回落，不抛错 */
  async function hydrateInBackground(): Promise<void> {
    try {
      await hydrateConfig();
    } catch (error) {
      // 链式 API 永不 reject，此处仅防命令构造期同步抛错导致水合中断
      reportCommandFailure("[config] hydrate threw", error);
      return;
    }

    const locale = configState.value?.locale;
    if (locale && locale !== getLocale()) {
      alignLocale(locale);
    }
  }

  // 启动静默检查更新：开着开关才执行，单会话一次，非 Tauri 环境跳过
  $effect(() => {
    maybeAutoCheckForUpdate(configState.value?.auto_check_update ?? false);
  });

  // 窗口关闭统一接管：先阻止默认关闭，再按后端关闭行为分流；
  // 配置未水合时回落弹窗确认，绝不静默退出；浏览器预览无运行时则跳过注册
  $effect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        const stop = await getCurrentWindow().onCloseRequested(async (event) => {
          await event.preventDefault();
          const behavior = configState.value?.close_behavior ?? "prompt";
          if (behavior === "minimize_to_tray") {
            await getCurrentWindow()
              .hide()
              .catch((error: unknown) =>
                reportCommandFailure("[window] failed to hide on close", error),
              );
            return;
          }
          if (behavior === "exit") {
            await commands
              .quitApp()
              .failed((failure) => reportCommandFailure("[window] failed to quit", failure));
            return;
          }
          confirmCloseOpen = true;
        });
        if (cancelled) {
          stop();
        } else {
          unlisten = stop;
        }
      } catch {
        // 非 Tauri 环境（浏览器预览）无窗口事件，直接跳过
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  /** 确认弹窗中真退出；弹窗已由通用组件关闭，进程结束故无需处理返回值，失败仅上报 */
  async function handleConfirmQuit(): Promise<void> {
    await commands
      .quitApp()
      .failed((failure) => reportCommandFailure("[window] failed to quit", failure));
  }

  // 深链入口：冷启动 + 运行中 + 次实例转发三路订阅，单会话一次，非 Tauri 环境跳过
  $effect(() => {
    let cancelled = false;
    let stop: (() => void) | null = null;
    (async () => {
      try {
        const stopLinks = await initDeepLinks();
        if (cancelled) {
          stopLinks();
        } else {
          stop = stopLinks;
        }
      } catch {
        // 非 Tauri 环境（浏览器预览）无深链事件，直接跳过
      }
    })();
    return () => {
      cancelled = true;
      stop?.();
    };
  });
</script>

<!-- 跟随系统主题，并把 .dark 类同步到根元素 -->
<ModeWatcher defaultMode="system" />

<!-- 应用本体：DOM 顺序即焦点顺序，内容在前，浮层挂载点在内容之后 -->
<ErrorBoundary>
  {@render children()}
</ErrorBoundary>

<!-- 全局唯一的 Toast 挂载点：主题由 sonner 内部跟随 mode-watcher。
     挂在内容之后，避免有 toast 时它的关闭按钮抢到页面第一个 Tab 停靠点 -->
<Toaster position="bottom-right" richColors closeButton />

<!-- 关闭行为为弹窗提示时的确认框：平时不挂载，由关闭拦截按需打开 -->
<ConfirmDialog
  bind:open={confirmCloseOpen}
  title={m.settings_close_confirm_title()}
  description={m.settings_close_confirm_description()}
  cancelLabel={m.settings_close_confirm_cancel()}
  confirmLabel={m.settings_close_confirm_ok()}
  onConfirm={() => void handleConfirmQuit()}
/>
