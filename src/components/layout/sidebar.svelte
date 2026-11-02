<script lang="ts">
  // 侧边栏布局：左侧全高侧边栏 + 右侧内容列（标题栏 / 内容 / 底边），窗口逻辑复用 TitleBar。
  import type { Snippet } from "svelte";
  import AppIcon from "#assets/icons/app-icon.svelte";
  import {
    Sidebar,
    SidebarContent,
    SidebarHeader,
    SidebarInset,
    SidebarProvider,
    SidebarTrigger,
  } from "#components/shadcn-svelte/sidebar";
  import Copyright from "#components/layout/parts/copyright.svelte";
  import SideNavBar from "#components/layout/parts/side-nav-bar.svelte";
  import TitleBar from "#components/layout/parts/title-bar.svelte";
  import { cn } from "#libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();
</script>

<SidebarProvider data-layout="sidebar" class={cn("h-svh overflow-hidden")}>
  <Sidebar>
    <SidebarHeader>
      <div class={cn("flex items-center gap-2 px-2 py-1 select-none")}>
        <AppIcon class={cn("size-5")} />
        <span class={cn("truncate text-sm font-medium")}>
          {__APP_TAURI_CONF__.app.windows[0].title}
        </span>
      </div>
    </SidebarHeader>
    <SidebarContent>
      <SideNavBar />
    </SidebarContent>
  </Sidebar>
  <SidebarInset class={cn("h-svh overflow-hidden")}>
    <div
      class={cn("flex min-h-0 w-full flex-1 flex-col items-center justify-center overflow-hidden")}
    >
      {@render children()}
    </div>
    <footer class={cn("flex w-full shrink-0 items-center justify-center border-t")}>
      <Copyright class={cn("m-1 text-xs text-muted-foreground select-none")} />
    </footer>

    <!-- 标题栏在 DOM 末尾、靠 `order-first` 提到视觉首位：DOM 顺序即焦点与朗读顺序，
         含"关闭"的窗口控制与侧栏开关不再占住内容之前的 Tab 停靠点 -->
    <header class={cn("order-first w-full shrink-0 border-b")}>
      <TitleBar>
        {#snippet left()}
          <SidebarTrigger class={cn("ml-1")} />
        {/snippet}
      </TitleBar>
    </header>
  </SidebarInset>
</SidebarProvider>
