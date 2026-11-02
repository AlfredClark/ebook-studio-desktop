<script lang="ts">
  // 顶部标签布局
  import type { Snippet } from "svelte";
  import NavTabsBar from "#components/layout/parts/nav-tabs-bar.svelte";
  import TitleBar from "#components/layout/parts/title-bar.svelte";
  import Copyright from "#components/layout/parts/copyright.svelte";
  import { cn } from "#libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();
</script>

<div data-layout="tabs" class={cn("flex h-screen w-full flex-col overflow-hidden")}>
  <nav class={cn("w-full shrink-0 border-b")}>
    <NavTabsBar />
  </nav>
  <main
    class={cn("flex min-h-0 w-full flex-1 flex-col items-center justify-center overflow-hidden")}
  >
    {@render children()}
  </main>

  <footer class={cn("flex w-full shrink-0 items-center justify-center border-t")}>
    <Copyright class={cn("m-1 text-xs text-muted-foreground select-none")} />
  </footer>

  <!-- 标题栏在 DOM 末尾、靠 `order-first` 提到视觉首位：DOM 顺序即焦点与朗读顺序，
       含"关闭"的窗口控制不再占住前四个 Tab 停靠点 -->
  <header class={cn("order-first w-full shrink-0 border-b")}>
    <TitleBar />
  </header>
</div>
