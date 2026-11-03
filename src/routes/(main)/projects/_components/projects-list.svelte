<script lang="ts">
  // 项目列表：行从左到右为复选框 + 名称，右侧为更新时间 + 类型标签 + 编辑/删除图标按钮；
  // 行点击预览，行键盘可达（聚焦行内控件时不抢回车）；编辑态名称变输入框，回车/✓确认、Esc/✕取消。
  import CheckIcon from "@lucide/svelte/icons/check";
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import PencilIcon from "@lucide/svelte/icons/pencil";
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import XIcon from "@lucide/svelte/icons/x";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import { Input } from "#components/shadcn-svelte/input";
  import { ScrollArea } from "#components/shadcn-svelte/scroll-area";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale } from "#libs/i18n/paraglide/runtime";
  import { cn } from "#libs/utils/shadcn-svelte";
  import { formatDateTime } from "./project-data";
  import type { ProjectInfo } from "#libs/commands/types";

  let {
    projects,
    activeId,
    checkedIds,
    editingId,
    renaming,
    onSelect,
    onToggle,
    onEditStart,
    onEditCancel,
    onEditConfirm,
    onDelete,
  }: {
    projects: ProjectInfo[];
    activeId: string | undefined;
    checkedIds: string[];
    editingId: string | null;
    renaming: boolean;
    onSelect: (id: string) => void;
    onToggle: (id: string, checked: boolean) => void;
    onEditStart: (id: string) => void;
    onEditCancel: () => void;
    onEditConfirm: (id: string, name: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  let draftName = $state("");

  /** 进入编辑态：草稿取当前名；退出时草稿保留，下次进入重取。 */
  function startEdit(id: string, name: string): void {
    draftName = name;
    onEditStart(id);
  }

  /** 草稿去首尾空白后仍有内容且非提交中才可确认。 */
  const canConfirm = $derived(draftName.trim() !== "" && !renaming);

  /** 类型徽标文案；两类共用 outline 变体（透明底 + 描边），重量介于实心与柔和底之间。 */
  function typeLabel(type: ProjectInfo["type"]): string {
    return type === "novel" ? m.projects_type_novel() : m.projects_type_comic();
  }

  /** 图标按钮悬浮：图标放大一档（150ms 内完成，只服务反馈）；描边与底色走 ghost 原 tokens。 */
  const iconHover = "hover:[&_svg]:scale-110 [&_svg]:transition-transform";

  /** 列表框方向键：在各行间移动焦点；焦点落在输入框等可编辑元素时放行，不劫持光标。 */
  function handleListNavigation(event: KeyboardEvent): void {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") {
      return;
    }
    const target = event.target as HTMLElement | null;
    if (target === null || target.closest("input, textarea, [contenteditable]") !== null) {
      return;
    }
    const container = event.currentTarget as HTMLElement;
    const options = Array.from(container.querySelectorAll<HTMLElement>('[role="option"]'));
    const current = target.closest('[role="option"]');
    const index = current === null ? -1 : options.indexOf(current as HTMLElement);
    const next = event.key === "ArrowDown" ? index + 1 : index - 1;
    if (next >= 0 && next < options.length) {
      event.preventDefault();
      options[next].focus();
    }
  }

  /** 行回车/空格选中：事件来自行内控件时放行，不重复触发。 */
  function handleRowKeydown(event: KeyboardEvent, id: string): void {
    if (event.target !== event.currentTarget) {
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onSelect(id);
    }
  }

  /** 编辑框按键：回车确认、Esc 取消；阻止冒泡避免触发行选中。 */
  function handleDraftKeydown(event: KeyboardEvent, id: string): void {
    event.stopPropagation();
    if (event.key === "Enter") {
      event.preventDefault();
      if (canConfirm) {
        onEditConfirm(id, draftName.trim());
      }
    } else if (event.key === "Escape") {
      event.preventDefault();
      onEditCancel();
    }
  }
</script>

{#if projects.length === 0}
  <div class="flex min-h-0 flex-1 items-center justify-center p-6 text-center">
    <p class="text-sm text-muted-foreground">{m.projects_list_empty()}</p>
  </div>
{:else}
  <ScrollArea class="min-h-0 flex-1">
    <div
      role="listbox"
      aria-label={m.page_projects_title()}
      tabindex={-1}
      onkeydown={handleListNavigation}
      class="flex flex-col gap-1 p-1"
    >
      {#each projects as project (project.id)}
        {@const active = project.id === activeId}
        {@const checked = checkedIds.includes(project.id)}
        {@const editing = project.id === editingId}
        <div
          role="option"
          aria-selected={active}
          tabindex={0}
          onclick={() => onSelect(project.id)}
          onkeydown={(event) => handleRowKeydown(event, project.id)}
          class={cn(
            "flex min-w-0 items-center gap-2 rounded-lg px-2 py-2 outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50",
            active && "bg-accent",
          )}
        >
          <Checkbox
            {checked}
            onclick={(event) => event.stopPropagation()}
            onCheckedChange={(value) => onToggle(project.id, value === true)}
            aria-label={project.name}
            class="shrink-0"
          />
          {#if editing}
            <Input
              value={draftName}
              oninput={(event) => (draftName = event.currentTarget.value)}
              onkeydown={(event) => handleDraftKeydown(event, project.id)}
              onclick={(event) => event.stopPropagation()}
              type="text"
              maxlength={100}
              aria-label={m.projects_list_edit()}
              class="h-7 min-w-0 flex-1"
            />
            <Button
              variant="ghost"
              size="icon-sm"
              class="shrink-0 {iconHover}"
              disabled={!canConfirm}
              onclick={(event) => {
                event.stopPropagation();
                onEditConfirm(project.id, draftName.trim());
              }}
              aria-label={m.projects_row_confirm()}
            >
              <CheckIcon />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              class="shrink-0 {iconHover}"
              onclick={(event) => {
                event.stopPropagation();
                onEditCancel();
              }}
              aria-label={m.projects_row_cancel()}
            >
              <XIcon />
            </Button>
          {:else}
            <span class="min-w-0 flex-1 truncate text-sm font-medium">{project.name}</span>
            <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
              {formatDateTime(project.updated_at, getLocale())}
            </span>
            <Badge variant="outline" class="shrink-0">{typeLabel(project.type)}</Badge>
            <Button
              variant="ghost"
              size="icon-sm"
              class="shrink-0 {iconHover}"
              onclick={() => void goto(resolve(`/${project.type}/${project.id}/metadata`))}
              aria-label={m.projects_open()}
            >
              <FolderOpenIcon />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              class="shrink-0 {iconHover}"
              onclick={() => startEdit(project.id, project.name)}
              aria-label={m.projects_list_edit()}
            >
              <PencilIcon />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              class="shrink-0 {iconHover} hover:bg-destructive/10 hover:text-destructive dark:hover:bg-destructive/20"
              onclick={() => onDelete(project.id)}
              aria-label={m.projects_list_delete()}
            >
              <Trash2Icon />
            </Button>
          {/if}
        </div>
      {/each}
    </div>
  </ScrollArea>
{/if}
