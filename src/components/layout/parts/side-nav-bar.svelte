<script lang="ts">
  // 侧边栏导航标签列表：标签清单复用 #libs/navigation/nav-tabs.ts，与顶部标签栏同源，此处只负责渲染与路由跳转。
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
  } from "#components/shadcn-svelte/sidebar";
  import { NAV_TABS, resolveNavCurrent } from "#libs/navigation/nav-tabs";
  import { cn } from "#libs/utils/shadcn-svelte";

  const pathname = $derived(page.url.pathname);
</script>

<SidebarMenu class={cn("gap-1 px-2")}>
  {#each NAV_TABS as tab (tab.path)}
    {@const Icon = tab.icon}
    {@const current = resolveNavCurrent(tab.path, pathname)}
    <SidebarMenuItem>
      <SidebarMenuButton
        isActive={current !== undefined}
        aria-current={current}
        tooltipContent={tab.label()}
        onclick={() => void goto(resolve(tab.path))}
      >
        <Icon />
        <span>{tab.label()}</span>
      </SidebarMenuButton>
    </SidebarMenuItem>
  {/each}
</SidebarMenu>
