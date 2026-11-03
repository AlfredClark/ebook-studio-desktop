<script lang="ts">
  // 项目分组窗口控制：置顶 / 最小化 / 最大化 / 关闭四按钮，接线与主标题栏一致；
  // 仅 (projects) 分组使用（步骤导航栏右端），后续出现第三个调用方时再提升到 common。
  import CopyIcon from "@lucide/svelte/icons/copy";
  import MinusIcon from "@lucide/svelte/icons/minus";
  import PinIcon from "@lucide/svelte/icons/pin";
  import PinOffIcon from "@lucide/svelte/icons/pin-off";
  import SquareIcon from "@lucide/svelte/icons/square";
  import XIcon from "@lucide/svelte/icons/x";
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
