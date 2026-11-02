<script lang="ts">
  // 无边框窗口的自定义标题栏：左侧应用名（取注入的 Tauri 配置），右侧窗口操作按钮。
  // 无 Tauri 运行时（浏览器预览 / 单测）时窗口操作静默失败，界面保持可用。
  import CopyIcon from "@lucide/svelte/icons/copy";
  import MinusIcon from "@lucide/svelte/icons/minus";
  import PinIcon from "@lucide/svelte/icons/pin";
  import PinOffIcon from "@lucide/svelte/icons/pin-off";
  import SquareIcon from "@lucide/svelte/icons/square";
  import XIcon from "@lucide/svelte/icons/x";
  import type { Snippet } from "svelte";
  import AppIcon from "#assets/icons/app-icon.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";
  import {
    closeWindow,
    isWindowAlwaysOnTop,
    isWindowMaximized,
    minimizeWindow,
    onWindowResized,
    toggleAlwaysOnTopWindow,
    toggleMaximizeWindow,
  } from "#libs/utils/window-controls";

  // 左区默认渲染应用图标与标题，传入 left 片段时由调用方接管（如侧边栏布局的折叠按钮）。
  let {
    title = __APP_TAURI_CONF__.app.windows[0].title,
    left,
  }: { title?: string; left?: Snippet } = $props();

  let alwaysOnTop = $state(false);
  let maximized = $state(false);

  const pinLabel = $derived(alwaysOnTop ? m.title_bar_unpin() : m.title_bar_pin());
  const maximizeLabel = $derived(maximized ? m.title_bar_restore() : m.title_bar_maximize());

  async function handleToggleAlwaysOnTop() {
    alwaysOnTop = await toggleAlwaysOnTopWindow();
  }

  async function handleMinimize() {
    await minimizeWindow();
  }

  async function handleToggleMaximize() {
    await toggleMaximizeWindow();
    maximized = await isWindowMaximized();
  }

  async function handleClose() {
    await closeWindow();
  }

  // 鼠标拖动窗口边缘或双击标题栏唤起系统最大化时，按钮图标需随之切换；
  // resize 高频触发，rAF 合并为每帧一次 IPC，尾帧补齐保证图标终态
  $effect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    let rafId = 0;
    let trailing = false;

    async function refreshMaximized(): Promise<void> {
      maximized = await isWindowMaximized();
    }

    void (async () => {
      alwaysOnTop = await isWindowAlwaysOnTop();
      maximized = await isWindowMaximized();
      const off = await onWindowResized(() => {
        if (rafId !== 0) {
          trailing = true;
          return;
        }
        void refreshMaximized();
        rafId = requestAnimationFrame(() => {
          rafId = 0;
          if (trailing) {
            trailing = false;
            void refreshMaximized();
          }
        });
      });
      if (disposed) {
        off?.();
      } else {
        unlisten = off;
      }
    })();

    return () => {
      disposed = true;
      if (rafId !== 0) {
        cancelAnimationFrame(rafId);
      }
      unlisten?.();
    };
  });
</script>

<div class={cn("flex h-10 w-full items-center justify-between bg-background select-none")}>
  <div class={cn("flex h-full items-center justify-center pt-1 pl-1")}>
    {#if left}
      {@render left()}
    {:else}
      <AppIcon class={cn("ml-2 size-5")} />
      <span class={cn("ml-2 truncate text-sm font-medium")} data-tauri-drag-region>{title}</span>
    {/if}
  </div>
  <div class={cn("h-full w-full flex-1 pt-1")}>
    <div class={cn("h-full w-full flex-1")} data-tauri-drag-region></div>
  </div>
  <div class={cn("flex h-full shrink-0 items-center gap-0.5 p-1")}>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={pinLabel}
      aria-pressed={alwaysOnTop}
      title={pinLabel}
      onclick={handleToggleAlwaysOnTop}
    >
      {#if alwaysOnTop}
        <PinOffIcon />
      {:else}
        <PinIcon />
      {/if}
    </Button>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={m.title_bar_minimize()}
      title={m.title_bar_minimize()}
      onclick={handleMinimize}
    >
      <MinusIcon />
    </Button>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={maximizeLabel}
      title={maximizeLabel}
      onclick={handleToggleMaximize}
    >
      {#if maximized}
        <CopyIcon />
      {:else}
        <SquareIcon />
      {/if}
    </Button>
    <Button
      variant="ghost"
      size="icon-sm"
      class={cn("hover:bg-destructive/10 hover:text-destructive")}
      aria-label={m.title_bar_close()}
      title={m.title_bar_close()}
      onclick={handleClose}
    >
      <XIcon />
    </Button>
  </div>
</div>
