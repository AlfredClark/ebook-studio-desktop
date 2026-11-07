<script lang="ts">
  // 右栏详情：展示选中项目的封面与元数据，标题栏附带打开项目目录入口；无选中走空态，
  // 无元数据回落默认封面（2:3 项目名）与项目创建时间。
  // 作者 / 主题词 / 简介常显，其余属性收进末尾的更多信息折叠区。
  import { convertFileSrc } from "@tauri-apps/api/core";
  import * as Accordion from "#components/shadcn-svelte/accordion";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import commands from "#libs/commands";
  import type { ProjectInfo, ProjectMetadata, ProjectOverview } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { toast } from "#libs/utils/toast";
  import { formatDateTime } from "#libs/utils/datetime";

  let { project }: { project: ProjectInfo | undefined } = $props();

  let overview = $state<ProjectOverview | null>(null);
  let coverUrl = $state<string | null>(null);
  /** 打开目录在途时禁用按钮，避免重复调起文件管理器。 */
  let openingDir = $state(false);

  /** 折叠区是否有内容：九项附加属性任一非空即展示。 */
  function hasMoreInfo(meta: ProjectMetadata): boolean {
    return (
      meta.alternative !== "" ||
      meta.contributor !== "" ||
      meta.language !== "" ||
      meta.publisher !== "" ||
      meta.date_published !== null ||
      meta.series !== "" ||
      meta.volume !== "" ||
      meta.source !== "" ||
      meta.rights !== ""
    );
  }

  /** 绝对路径转预览 URL：非 Tauri 环境（单测 / 纯浏览器）转失败时回落空，走默认封面。 */
  function toCoverUrl(path: string): string | null {
    try {
      return convertFileSrc(path);
    } catch {
      return null;
    }
  }

  /** 打开项目目录：调系统文件管理器定位项目文件夹，失败 toast。 */
  async function openProjectDir(): Promise<void> {
    const id = project?.id;
    if (id == null || openingDir) {
      return;
    }
    openingDir = true;
    const result = await commands.openProjectDir(id).result();
    // 切走选中时不碰残留态：按钮禁用态由 openingDir 统一收尾。
    openingDir = false;
    if (result.status === "error") {
      toast.error(m.projects_detail_open_dir_failed());
    }
  }
  /** 拉取概览快照：只读不建文件；失败静默回落默认展示（后端已记日志）。 */
  async function loadOverview(id: string): Promise<void> {
    const result = await commands.getProjectOverview(id).result();
    if (result.status === "error" || id !== project?.id) {
      return;
    }
    overview = result.data;
    const path = result.data.cover_path;
    coverUrl = path == null ? null : toCoverUrl(path);
  }

  // 选中变化即重拉；切空选中时清空旧快照，避免残留上一个项目。
  $effect(() => {
    const id = project?.id;
    overview = null;
    coverUrl = null;
    if (id !== undefined) {
      void loadOverview(id);
    }
  });
</script>

<div class="flex h-full min-h-0 flex-col gap-4 overflow-auto p-6">
  {#if project}
    {@const meta = overview?.metadata}
    <div class="flex min-w-0 items-center gap-2">
      <h2 class="min-w-0 flex-1 truncate text-lg font-medium">{meta?.title || project.name}</h2>
      <Button
        variant="outline"
        size="sm"
        class="shrink-0"
        disabled={openingDir}
        onclick={() => void openProjectDir()}
      >
        {m.projects_detail_open_dir()}
      </Button>
      <Badge variant="outline" class="shrink-0">
        {project.type === "novel" ? m.projects_type_novel() : m.projects_type_comic()}
      </Badge>
    </div>
    {#if coverUrl !== null}
      <div class="flex justify-center">
        <img
          src={coverUrl}
          alt={m.metadata_cover_preview_alt()}
          class="aspect-[2/3] w-40 rounded-md border object-cover"
        />
      </div>
    {:else}
      <div class="flex justify-center">
        <div
          class="flex aspect-[2/3] w-40 items-center justify-center rounded-md border bg-muted p-3 text-center"
        >
          <span class="line-clamp-4 text-sm font-medium">{meta?.title || project.name}</span>
        </div>
      </div>
    {/if}
    <dl class="flex flex-col gap-2 text-sm">
      {#if meta?.creator}
        <div class="flex items-center justify-between gap-2">
          <dt class="shrink-0 text-muted-foreground">{m.metadata_creator_label()}</dt>
          <dd class="min-w-0 truncate">{meta.creator}</dd>
        </div>
      {/if}
      {#if meta && meta.description.length > 0}
        <div class="flex flex-col gap-1">
          <dt class="shrink-0 text-muted-foreground">{m.metadata_description_label()}</dt>
          <dd class="line-clamp-3 flex flex-col gap-0.5">
            {#each meta.description as line, i (i)}
              <span class="block">{line}</span>
            {/each}
          </dd>
        </div>
      {/if}
      {#if meta && meta.subjects.length > 0}
        <div class="flex items-center justify-between gap-2">
          <dt class="shrink-0 text-muted-foreground">{m.metadata_subjects_label()}</dt>
          <dd class="min-w-0 truncate">{meta.subjects.join("，")}</dd>
        </div>
      {/if}
      <div class="flex items-center justify-between gap-2">
        <dt class="shrink-0 text-muted-foreground">{m.projects_detail_created()}</dt>
        <dd class="min-w-0 truncate">{formatDateTime(project.created_at, getLocale())}</dd>
      </div>
      <div class="flex items-center justify-between gap-2">
        <dt class="shrink-0 text-muted-foreground">{m.projects_detail_updated()}</dt>
        <dd class="min-w-0 truncate">{formatDateTime(project.updated_at, getLocale())}</dd>
      </div>
      {#if meta && hasMoreInfo(meta)}
        <Accordion.Root type="single" class="w-full">
          <Accordion.Item value="more-info">
            <Accordion.Trigger class="text-sm text-muted-foreground">
              {m.metadata_more_info()}
            </Accordion.Trigger>
            <Accordion.Content>
              <div class="flex flex-col gap-2 text-sm">
                {#if meta.alternative}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_alternative_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.alternative}</dd>
                  </div>
                {/if}
                {#if meta.contributor}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_contributor_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.contributor}</dd>
                  </div>
                {/if}
                {#if meta.publisher}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_publisher_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.publisher}</dd>
                  </div>
                {/if}
                {#if meta.rights}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_rights_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.rights}</dd>
                  </div>
                {/if}
                {#if meta.language}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_language_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.language}</dd>
                  </div>
                {/if}
                {#if meta.series}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_series_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.series}</dd>
                  </div>
                {/if}
                {#if meta.volume}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_volume_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.volume}</dd>
                  </div>
                {/if}
                {#if meta.source}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">{m.metadata_source_label()}</dt>
                    <dd class="min-w-0 truncate">{meta.source}</dd>
                  </div>
                {/if}
                {#if meta.date_published}
                  <div class="flex items-center justify-between gap-2">
                    <dt class="shrink-0 text-muted-foreground">
                      {m.metadata_date_published_label()}
                    </dt>
                    <dd class="min-w-0 truncate">{meta.date_published}</dd>
                  </div>
                {/if}
              </div>
            </Accordion.Content>
          </Accordion.Item>
        </Accordion.Root>
      {/if}
    </dl>
  {:else}
    <div class="flex min-h-0 flex-1 items-center justify-center p-6 text-center">
      <p class="text-sm text-muted-foreground">{m.projects_detail_empty()}</p>
    </div>
  {/if}
</div>
