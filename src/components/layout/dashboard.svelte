<script lang="ts">
  // 仪表盘布局：顶部标题栏 + 左侧固定导航 + 内容区 + 底边，适合页面多的后台型应用。
  // 与 sidebar 的差异：侧栏带折叠状态机，收起/展开由底部把手控制，偏好落 localStorage（默认展开），窄屏自动收为图标栏。
  import { untrack } from "svelte";
  import type { Snippet } from "svelte";
  import { mergeProps } from "bits-ui";
  import PanelLeftCloseIcon from "@lucide/svelte/icons/panel-left-close";
  import PanelLeftOpenIcon from "@lucide/svelte/icons/panel-left-open";
  import {
    dashboardState,
    initDashboard,
    setDashboardCollapsed,
  } from "#libs/hooks/dashboard.svelte";
  import { getSharedIsMobile } from "#libs/hooks/is-mobile.svelte";
  import { buttonVariants } from "#components/shadcn-svelte/button";
  import * as Tooltip from "#components/shadcn-svelte/tooltip/index.js";
  import Copyright from "#components/layout/parts/copyright.svelte";
  import DashboardNavBar from "#components/layout/parts/dashboard-nav-bar.svelte";
  import TitleBar from "#components/layout/parts/title-bar.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();

  // 窄屏强制图标栏；桌面端由底部把手控制，偏好落 localStorage，布局重挂载后按持久化值恢复，缺省展开。
  // 断点监听复用共享单例，同断点多处订阅不断重复注册。
  const isMobile = getSharedIsMobile();

  // init 只写折叠状态：包 untrack 切断订阅，避免"改动→重初始化"回路（对标容器层的 initLayout）
  $effect.pre(() => {
    untrack(() => initDashboard());
  });

  const collapsed = $derived(dashboardState.collapsed);
  const compact = $derived(isMobile.current || collapsed);
</script>

<div data-layout="dashboard" class={cn("flex h-screen w-full flex-col overflow-hidden")}>
  <div class={cn("flex min-h-0 w-full flex-1 flex-row overflow-hidden")}>
    <aside
      class={cn(
        "flex shrink-0 flex-col overflow-x-hidden overflow-y-auto border-r transition-[width] duration-200",
        compact ? "w-14" : "w-52",
      )}
    >
      <Tooltip.Provider delayDuration={0}>
        <DashboardNavBar {compact} />
        {#if !isMobile.current}
          <div class={cn("mt-auto shrink-0 p-2")}>
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  {@const mergedProps = mergeProps(
                    {
                      class: cn(
                        buttonVariants({ variant: "ghost" }),
                        "w-full justify-start gap-2",
                        collapsed && "justify-center px-0",
                      ),
                      // 不写 aria-expanded：ghost 变体会给展开态加选中底色，与“中性开关”语义冲突
                      "aria-label": collapsed
                        ? m.dashboard_toggle_expand()
                        : m.dashboard_toggle_collapse(),
                      onclick: () => {
                        setDashboardCollapsed(!collapsed);
                      },
                    },
                    props,
                  )}
                  <button {...mergedProps}>
                    {#if collapsed}
                      <PanelLeftOpenIcon />
                    {:else}
                      <PanelLeftCloseIcon />
                    {/if}
                    {#if !collapsed}
                      <span class={cn("truncate")}>{m.dashboard_toggle_collapse()}</span>
                    {/if}
                  </button>
                {/snippet}
              </Tooltip.Trigger>
              {#if collapsed}
                <Tooltip.Content side="right">
                  {m.dashboard_toggle_expand()}
                </Tooltip.Content>
              {/if}
            </Tooltip.Root>
          </div>
        {/if}
      </Tooltip.Provider>
    </aside>
    <main class={cn("flex min-h-0 flex-1 flex-col overflow-y-auto")}>
      {@render children()}
    </main>
  </div>
  <footer class={cn("flex w-full shrink-0 items-center justify-center border-t")}>
    <Copyright class={cn("m-1 text-xs text-muted-foreground select-none")} />
  </footer>

  <!-- 标题栏在 DOM 末尾、靠 `order-first` 提到视觉首位：DOM 顺序即焦点与朗读顺序，
       含"关闭"的窗口控制不再占住内容之前的 Tab 停靠点 -->
  <header class={cn("order-first w-full shrink-0 border-b")}>
    <TitleBar />
  </header>
</div>
