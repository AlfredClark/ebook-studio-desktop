<script lang="ts">
  // 项目步骤导航：返回项目列表 + 六步面包屑；props 进类型与 uuid，当前步按地址末段解析。
  // 步骤链接仅换末段，类型与 uuid 透传；窄栏下横向滚动（滚动条隐藏）。
  import ArrowLeftIcon from "@lucide/svelte/icons/arrow-left";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Breadcrumb from "#components/shadcn-svelte/breadcrumb";
  import { m } from "#libs/i18n/paraglide/messages";
  import type { ProjectType } from "#libs/commands/types";
  import { PROJECT_STEPS, resolveProjectStep } from "#libs/navigation/project-steps";
  import { readProjectEntry } from "#libs/navigation/project-entry";
  import { cn } from "#libs/utils/shadcn-svelte";
  import WindowControls from "./window-controls.svelte";

  let { type, uuid }: { type: ProjectType; uuid: string } = $props();

  // 当前步骤随地址变化自动重算；未知末段回落首步（注册表内收敛）。
  const current = $derived(resolveProjectStep(page.url.pathname));
</script>

<nav
  aria-label={m.projects_steps_nav()}
  class={cn("flex h-10 w-full shrink-0 items-center gap-1 border-b px-1 select-none")}
>
  <Button
    variant="ghost"
    size="icon-sm"
    class="shrink-0"
    onclick={() => void goto(resolve(readProjectEntry()))}
    aria-label={m.projects_back()}
  >
    <ArrowLeftIcon />
  </Button>
  <Breadcrumb.Root class="min-w-0 shrink-0">
    <Breadcrumb.List class="flex-nowrap">
      {#each PROJECT_STEPS as step, index (step.slug)}
        <Breadcrumb.Item>
          {#if step.slug === current.slug}
            <Breadcrumb.Page aria-current="page">{step.label()}</Breadcrumb.Page>
          {:else}
            <Breadcrumb.Link href={resolve(`/${type}/${uuid}/${step.slug}`)}>
              {step.label()}
            </Breadcrumb.Link>
          {/if}
        </Breadcrumb.Item>
        {#if index < PROJECT_STEPS.length - 1}
          <Breadcrumb.Separator />
        {/if}
      {/each}
    </Breadcrumb.List>
  </Breadcrumb.Root>
  <!-- 空白拖拽区：撑满返回/面包屑与窗口控制之间的空隙，按住拖动窗口 -->
  <div class="h-full min-w-0 flex-1" data-tauri-drag-region></div>
  <WindowControls />
</nav>
