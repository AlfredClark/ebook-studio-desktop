<script lang="ts">
  // 顶部导航条：标签清单来自 #libs/navigation/nav-tabs.ts，此处只负责渲染为真正的导航链接。
  // 刻意不用 tabs 原语：那会把导航讲成 tablist、让方向键变成跳页、并让整条导航只占一个 Tab 停靠点。
  // 渲染成 `<a>` 后与 dashboard 布局语义一致：每项可单独 Tab 到、方向键不跳页、当前页由 `aria-current` 标记。
  // 视觉规格沿用 tabs line 变体（h-8 容器 + 3px 内边距 + 下划线压在分隔线上），故列表保持 inline-flex 不改盒模型。
  import { page } from "$app/state";
  import { resolve } from "$app/paths";
  import { NAV_TABS, resolveNavCurrent } from "#libs/navigation/nav-tabs";
  import { cn } from "#libs/utils/shadcn-svelte";

  const pathname = $derived(page.url.pathname);
</script>

<div class={cn("w-full")}>
  <div
    class={cn(
      "inline-flex h-8 w-fit items-center justify-center gap-1 rounded-none p-[3px] text-muted-foreground",
    )}
  >
    {#each NAV_TABS as tab (tab.path)}
      {@const Icon = tab.icon}
      {@const current = resolveNavCurrent(tab.path, pathname)}
      <a
        href={resolve(tab.path)}
        aria-current={current}
        class={cn(
          "relative inline-flex h-[calc(100%_-_1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-2 py-0.5 text-sm font-medium whitespace-nowrap transition-all after:absolute after:inset-x-0 after:bottom-[-5px] after:h-0.5 after:bg-foreground after:transition-opacity hover:text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1 focus-visible:outline-ring [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
          current
            ? "text-foreground after:opacity-100"
            : "text-foreground/60 after:opacity-0 dark:text-muted-foreground",
        )}
      >
        <Icon />
        {tab.label()}
      </a>
    {/each}
  </div>
</div>
