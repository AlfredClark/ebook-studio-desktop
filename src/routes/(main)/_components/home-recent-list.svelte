<script lang="ts">
  // 首页继续工作列表（哑组件）：封面缩略图 + 书名/类型/时间 + 四段状态 + 下一步直达；
  // 无封面用首字占位；主区整块可点进项目入口，下一步直达其卡住的步骤。
  import { resolve } from "$app/paths";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import ArrowRightIcon from "@lucide/svelte/icons/arrow-right";
  import BookOpenIcon from "@lucide/svelte/icons/book-open";
  import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
  import ImagesIcon from "@lucide/svelte/icons/images";
  import { Badge } from "#components/shadcn-svelte/badge";
  import type { ProjectOverviewItem, ProjectStageStatus, ProjectType } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { formatDateTime } from "#libs/utils/datetime";

  let {
    projects,
    onOpen,
  }: {
    projects: ProjectOverviewItem[];
    onOpen: (project: ProjectOverviewItem) => void;
  } = $props();

  /** 四段状态：文件（素材）/解析复用步骤文案，解析新增键；顺序即管线顺序。 */
  function stagesOf(stages: ProjectStageStatus): { done: boolean; label: string }[] {
    return [
      { done: stages.has_source, label: m.project_step_files() },
      { done: stages.has_content, label: m.page_home_stage_parsed() },
      { done: stages.has_build, label: m.project_step_build() },
      { done: stages.has_package, label: m.project_step_package() },
    ];
  }

  /** 无障碍读数：各段逐项播报完成态。 */
  function stageState(done: boolean): string {
    return done ? m.page_home_stage_done() : m.page_home_stage_todo();
  }

  /** 下一步：首个未完成阶段对应的步骤，落点与项目步骤路由一致；全齐去导出。 */
  function nextStep(project: ProjectOverviewItem): {
    label: string;
    href: `/${ProjectType}/${string}/${"files" | "build" | "package" | "export"}`;
  } {
    const prefix: `/${ProjectType}/${string}` = `/${project.info.type}/${project.info.id}`;
    const stages = project.stages;
    if (!stages.has_source || !stages.has_content) {
      return { label: m.project_step_files(), href: `${prefix}/files` };
    }
    if (!stages.has_build) {
      return { label: m.project_step_build(), href: `${prefix}/build` };
    }
    if (!stages.has_package) {
      return { label: m.project_step_package(), href: `${prefix}/package` };
    }
    return { label: m.project_step_export(), href: `${prefix}/export` };
  }

  /** 封面地址：缺封面回 null 走首字占位；拼更新时间戳，替换封面后强制刷新。 */
  function coverUrl(project: ProjectOverviewItem): string | null {
    if (project.cover_path == null) {
      return null;
    }
    return `${convertFileSrc(project.cover_path)}?v=${project.info.updated_at}`;
  }

  /** 首字占位：取书名首字符，空名回落问号。 */
  function initialOf(name: string): string {
    const [first] = name.trim();
    return first ?? "?";
  }
</script>

<ul class="grid min-w-0 grid-cols-1 gap-2 md:grid-cols-2">
  {#each projects as project (project.info.id)}
    {@const cover = coverUrl(project)}
    {@const next = nextStep(project)}
    <li class="flex min-w-0 flex-col rounded-lg hover:bg-muted">
      <button
        type="button"
        onclick={() => onOpen(project)}
        class="flex w-full min-w-0 flex-1 items-center gap-4 rounded-lg rounded-b-none px-3 pt-3 pb-1.5 text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        {#if cover == null}
          <span
            class="flex h-[110px] w-20 shrink-0 flex-col items-center justify-center gap-1.5 rounded border"
            aria-hidden="true"
          >
            <span class="text-2xl font-medium text-muted-foreground">
              {initialOf(project.info.name)}
            </span>
            {#if project.info.type === "novel"}
              <BookOpenIcon class="size-4 text-muted-foreground" />
            {:else}
              <ImagesIcon class="size-4 text-muted-foreground" />
            {/if}
          </span>
        {:else}
          <img
            src={cover}
            alt=""
            loading="eager"
            class="h-[110px] w-20 shrink-0 rounded object-cover"
          />
        {/if}
        <span class="flex min-w-0 flex-1 flex-col gap-2 py-1">
          <span class="flex w-full min-w-0 items-center gap-2">
            <span class="min-w-0 flex-1 truncate text-base font-medium">{project.info.name}</span>
            <Badge variant="outline" class="shrink-0">
              {project.info.type === "novel" ? m.projects_type_novel() : m.projects_type_comic()}
            </Badge>
            <ChevronRightIcon class="size-4 shrink-0 text-muted-foreground" />
          </span>
          <span class="w-full min-w-0 truncate text-sm text-muted-foreground tabular-nums">
            {formatDateTime(project.info.updated_at, getLocale())}
          </span>
          <span class="flex w-full min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
            {#each stagesOf(project.stages) as stage (stage.label)}
              <span class="flex shrink-0 items-center gap-1">
                <span
                  class="size-2 rounded-full {stage.done ? 'bg-primary' : 'bg-muted'}"
                  aria-hidden="true"
                ></span>
                <span class="text-sm {stage.done ? 'text-foreground' : 'text-muted-foreground'}">
                  {stage.label}
                </span>
                <span class="sr-only">{stageState(stage.done)}</span>
              </span>
            {/each}
          </span>
        </span>
      </button>
      <a
        href={resolve(next.href)}
        class="flex w-full min-w-0 items-center gap-1 rounded-lg rounded-t-none px-3 pt-1 pb-3 pl-[108px] text-sm text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        {m.page_home_continue_to({ step: next.label })}
        <ArrowRightIcon class="size-3.5 shrink-0" />
      </a>
    </li>
  {/each}
</ul>
