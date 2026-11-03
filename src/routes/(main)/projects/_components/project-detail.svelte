<script lang="ts">
  // 右栏详情：只读展示选中项目的基础信息，无选中走空态；详情交互后续接通。
  import { Badge } from "#components/shadcn-svelte/badge";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { formatDateTime } from "./project-data";
  import type { ProjectInfo } from "#libs/commands/types";

  let { project }: { project: ProjectInfo | undefined } = $props();
</script>

<div class="flex h-full min-h-0 flex-col gap-4 overflow-auto p-6">
  {#if project}
    <div class="flex min-w-0 items-center gap-2">
      <h2 class="min-w-0 flex-1 truncate text-lg font-medium">{project.name}</h2>
      <Badge variant="outline" class="shrink-0">
        {project.type === "novel" ? m.projects_type_novel() : m.projects_type_comic()}
      </Badge>
    </div>
    <dl class="flex flex-col gap-2 text-sm">
      <div class="flex items-center justify-between gap-2">
        <dt class="shrink-0 text-muted-foreground">{m.projects_detail_type()}</dt>
        <dd class="min-w-0 truncate">
          {project.type === "novel" ? m.projects_type_novel() : m.projects_type_comic()}
        </dd>
      </div>
      <div class="flex items-center justify-between gap-2">
        <dt class="shrink-0 text-muted-foreground">{m.projects_detail_created()}</dt>
        <dd class="min-w-0 truncate">{formatDateTime(project.created_at, getLocale())}</dd>
      </div>
      <div class="flex items-center justify-between gap-2">
        <dt class="shrink-0 text-muted-foreground">{m.projects_detail_updated()}</dt>
        <dd class="min-w-0 truncate">{formatDateTime(project.updated_at, getLocale())}</dd>
      </div>
    </dl>
    <p class="text-sm text-muted-foreground">{m.projects_detail_pending()}</p>
  {:else}
    <div class="flex min-h-0 flex-1 items-center justify-center p-6 text-center">
      <p class="text-sm text-muted-foreground">{m.projects_detail_empty()}</p>
    </div>
  {/if}
</div>
