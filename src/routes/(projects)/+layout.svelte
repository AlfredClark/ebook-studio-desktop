<script lang="ts">
  // (projects) 分组布局：窗口 chrome 下沉到步骤导航栏（空白拖拽 + 窗口控制），
  // 本层只保留内容槽与底边版权。
  import type { Snippet } from "svelte";
  import Copyright from "#components/layout/parts/copyright.svelte";
  import { cn } from "#libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();
</script>

<div data-layout="projects" class={cn("flex h-screen w-full flex-col overflow-hidden")}>
  <main class={cn("flex min-h-0 w-full flex-1 flex-col overflow-hidden")}>
    {@render children()}
  </main>

  <footer class={cn("flex w-full shrink-0 items-center justify-center border-t")}>
    <Copyright class={cn("m-1 text-xs text-muted-foreground select-none")} />
  </footer>
</div>
