<script lang="ts">
  // 首页工作台：继续工作（按更新时间倒序取前 6 个，直达项目入口）+ 新建项目；
  // 空态沿用项目页 hero + 新建按钮；数据进页拉一次，失败可重试。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import AppIcon from "#assets/icons/app-icon.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import { Skeleton } from "#components/shadcn-svelte/skeleton";
  import CreateProjectDialog from "#components/common/create-project-dialog.svelte";
  import commands from "#libs/commands";
  import type { ProjectOverviewItem, ProjectType } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { storeProjectEntry } from "#libs/navigation/project-entry";
  import { toast } from "#libs/utils/toast";
  import HomeRecentList from "./_components/home-recent-list.svelte";

  const appName = __APP_TAURI_CONF__.productName;

  /** 首页继续工作条数：双列网格按 2 补齐，完整管理走项目页。 */
  const RECENT_COUNT = 6;

  let overviews = $state<ProjectOverviewItem[]>([]);
  let loading = $state(true);
  let loadFailed = $state(false);
  let dialogOpen = $state(false);
  let dialogType = $state<ProjectType>("novel");
  let creating = $state(false);

  /** 最近项目：按更新时间倒序截断，不改原数组。 */
  const recentProjects = $derived(
    [...overviews]
      .sort((a, b) => b.info.updated_at.localeCompare(a.info.updated_at))
      .slice(0, RECENT_COUNT),
  );

  /** 进页拉取一次：失败走错误态可重试，成功落本地态。 */
  async function loadOverview(): Promise<void> {
    loading = true;
    loadFailed = false;
    const result = await commands.getProjectsOverview().result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.projects_list_load_failed());
      return;
    }
    overviews = result.data;
  }

  /** 直达项目入口：落点与项目页一致（元数据步），并记下来源供返回按钮回跳。 */
  function handleOpen(project: ProjectOverviewItem): void {
    storeProjectEntry("/");
    void goto(resolve(`/${project.info.type}/${project.info.id}/metadata`));
  }

  /** 新建按钮：按类型开弹窗，类型决定弹窗标题。 */
  function handleCreateMenu(type: ProjectType): void {
    dialogType = type;
    dialogOpen = true;
  }

  /** 弹窗确认：后端建目录落盘，成功后直达新项目入口。 */
  async function handleCreateConfirm(name: string): Promise<void> {
    creating = true;
    const result = await commands.createProject(name, dialogType).result();
    creating = false;
    if (result.status === "error") {
      toast.error(m.projects_toast_create_failed());
      return;
    }
    dialogOpen = false;
    toast.success(m.projects_toast_created({ name: result.data.name }));
    storeProjectEntry("/");
    void goto(resolve(`/${result.data.type}/${result.data.id}/metadata`));
  }

  onMount(() => {
    void loadOverview();
  });
</script>

<h1 class="sr-only">{appName}</h1>
{#if loading}
  <div
    class="mx-auto flex w-full max-w-5xl flex-1 flex-col justify-center gap-2 p-6"
    aria-busy="true"
    aria-label={m.projects_list_loading()}
  >
    {#each [0, 1, 2] as index (index)}
      <Skeleton class="h-10 w-full" />
    {/each}
  </div>
{:else if loadFailed}
  <div
    class="mx-auto flex w-full max-w-5xl flex-1 flex-col items-center justify-center gap-3 p-6 text-center"
  >
    <p class="text-sm text-muted-foreground">{m.projects_list_load_failed()}</p>
    <Button variant="outline" size="sm" onclick={() => void loadOverview()}>
      {m.projects_list_retry()}
    </Button>
  </div>
{:else if overviews.length === 0}
  <div class="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-center">
    <AppIcon class="size-16" />
    <h2 class="text-2xl font-medium">{m.projects_empty_title()}</h2>
    <p class="max-w-md text-sm text-muted-foreground">{m.projects_empty_description()}</p>
    <div class="flex items-center gap-2">
      <Button size="sm" onclick={() => handleCreateMenu("novel")}>
        {m.page_home_new_novel()}
      </Button>
      <Button size="sm" variant="outline" onclick={() => handleCreateMenu("comic")}>
        {m.page_home_new_comic()}
      </Button>
    </div>
  </div>
{:else}
  <div class="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-6 p-6">
    <div class="flex flex-wrap items-center justify-between gap-3 py-4">
      <div class="flex min-w-0 items-center gap-3">
        <AppIcon class="size-11 shrink-0" />
        <div class="min-w-0">
          <h2 class="truncate text-xl font-medium">{appName}</h2>
          <p class="truncate text-xs text-muted-foreground tabular-nums">
            {m.page_home_stats({
              total: overviews.length,
              novel: overviews.filter((item) => item.info.type === "novel").length,
              comic: overviews.filter((item) => item.info.type === "comic").length,
            })}
          </p>
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <Button size="sm" onclick={() => handleCreateMenu("novel")}>
          {m.page_home_new_novel()}
        </Button>
        <Button size="sm" variant="outline" onclick={() => handleCreateMenu("comic")}>
          {m.page_home_new_comic()}
        </Button>
      </div>
    </div>
    <section aria-labelledby="home-recent-title" class="flex min-w-0 flex-col gap-2">
      <h2 id="home-recent-title" class="text-base font-medium">{m.page_home_recent_title()}</h2>
      <HomeRecentList projects={recentProjects} onOpen={handleOpen} />
    </section>
  </div>
{/if}

<CreateProjectDialog
  bind:open={dialogOpen}
  type={dialogType}
  {creating}
  onConfirm={(name) => void handleCreateConfirm(name)}
/>
