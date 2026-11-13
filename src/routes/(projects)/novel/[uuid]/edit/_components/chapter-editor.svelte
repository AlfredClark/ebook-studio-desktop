<script lang="ts">
  // 中栏编辑器（哑组件）：顶部左标题 + 就地改名（铅笔进编辑，输入框 + 确认/取消）、右自动保存开关 + 排版快捷与保存 + 正文区 + 命中列表；
  // 保存与自动保存开关仅脏态/开关态展示差异，实际落盘由父组件把守；
  // 改名只收标题输入并回抛，落盘与大纲刷新由父组件把守，与正文脏态正交；
  // 正文区按章节键重建，切换章节不串历史、不残留草稿；命中列表仅有命中时出现，点击跳到编辑器对应行。
  import CheckIcon from "@lucide/svelte/icons/check";
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import PencilIcon from "@lucide/svelte/icons/pencil";
  import SaveIcon from "@lucide/svelte/icons/save";
  import XIcon from "@lucide/svelte/icons/x";
  import { Button } from "#components/shadcn-svelte/button";
  import { Input } from "#components/shadcn-svelte/input";
  import type { LineMatch } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import ChapterMatches from "./chapter-matches.svelte";
  import ChapterTextView from "./chapter-text-view.svelte";
  import EditorTypographyPopover from "./editor-typography-popover.svelte";

  let {
    docKey,
    title,
    content,
    dirty,
    saving,
    renaming = false,
    openingDir = false,
    autoSave = false,
    matches = [],
    stale = false,
    jumpTo = null,
    onEdit,
    onSave,
    onRename,
    onOpenDir,
    onToggleAutoSave,
    onJump,
  }: {
    docKey: string;
    title: string;
    content: string;
    dirty: boolean;
    saving: boolean;
    /** 改名落盘中：禁用改名按钮与确认，正文保存不受影响。 */
    renaming?: boolean;
    /** 打开解析目录进行中：按钮禁用态。 */
    openingDir?: boolean;
    autoSave?: boolean;
    matches?: LineMatch[];
    stale?: boolean;
    jumpTo?: { line: number; nonce: number } | null;
    onEdit: (text: string) => void;
    onSave: () => void;
    /** 改名提交：已去首尾空白，落盘由父组件把守。 */
    onRename: (fullTitle: string) => void;
    /** 打开解析目录：调系统文件管理器定位项目根（`content.json` 所在），失败 toast 由父组件把守。 */
    onOpenDir: () => void;
    onToggleAutoSave: () => void;
    onJump: (line: number) => void;
  } = $props();

  /** 就地改名态：打开时以当前标题预填并记住所属，切章（标题变化）直接取消防串改。 */
  let editingTitle = $state(false);
  let renameDraft = $state("");
  let renameForTitle = $state("");
  /** 标题编辑组与输入框引用：失焦判定落点、挂载后聚焦全选。 */
  let renameGroup: HTMLDivElement | null = $state(null);
  let renameInput: HTMLInputElement | null = $state(null);
  const canRenameConfirm = $derived(renameDraft.trim() !== "" && !renaming);

  /** 改名输入框挂载后聚焦并全选（不用 autofocus 属性，a11y 规则不推荐）。 */
  $effect(() => {
    if (editingTitle) {
      renameInput?.focus();
      renameInput?.select();
    }
  });

  /** 切章即取消改名：打开时记住所属标题，标题变化说明已切章，草稿直接丢弃。 */
  $effect(() => {
    if (editingTitle && title !== renameForTitle) {
      editingTitle = false;
    }
  });

  /** 开始改名：草稿回填当前标题。 */
  function startRename(): void {
    renameDraft = title;
    renameForTitle = title;
    editingTitle = true;
  }

  /** 取消改名：草稿丢弃。 */
  function cancelRename(): void {
    editingTitle = false;
  }

  /** 提交改名：空标题不提交；提交即退编辑态，落盘由父组件把守。 */
  function confirmRename(): void {
    if (!canRenameConfirm) {
      return;
    }
    editingTitle = false;
    onRename(renameDraft.trim());
  }

  /** 改名框按键：组词中只确认组词，不触发提交；回车确认、Esc 取消。 */
  function handleRenameKeydown(event: KeyboardEvent): void {
    if (event.isComposing) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      confirmRename();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelRename();
    }
  }

  /** 改名框失焦：焦点仍在编辑组内（点确认/取消按钮）则由按钮接管；落到别处即确认。 */
  function handleRenameBlur(event: FocusEvent): void {
    if (event.relatedTarget instanceof Node && renameGroup?.contains(event.relatedTarget)) {
      return;
    }
    confirmRename();
  }
</script>

<section aria-label={m.edit_editor_label()} class="flex h-full min-h-0 flex-col">
  <div class="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
    <div bind:this={renameGroup} class="flex min-w-0 flex-1 items-center gap-1">
      {#if editingTitle}
        <Input
          bind:ref={renameInput}
          bind:value={renameDraft}
          onkeydown={handleRenameKeydown}
          onblur={handleRenameBlur}
          type="text"
          maxlength={200}
          aria-label={m.edit_rename_button_label()}
          placeholder={m.edit_rename_placeholder()}
          autocomplete="off"
          class="h-7 min-w-0 flex-1"
        />
        <Button
          variant="ghost"
          size="icon-sm"
          class="shrink-0"
          disabled={!canRenameConfirm}
          onclick={confirmRename}
          aria-label={m.projects_row_confirm()}
          title={m.projects_row_confirm()}
        >
          <CheckIcon />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class="shrink-0"
          onclick={cancelRename}
          aria-label={m.projects_row_cancel()}
          title={m.projects_row_cancel()}
        >
          <XIcon />
        </Button>
      {:else}
        <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{title}</h2>
        <Button
          variant="ghost"
          size="icon-sm"
          class="shrink-0"
          aria-label={m.edit_rename_button_label()}
          title={m.edit_rename_button_label()}
          disabled={renaming}
          onclick={startRename}
        >
          <PencilIcon />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          class="shrink-0"
          aria-label={m.edit_open_content_dir()}
          title={m.edit_open_content_dir()}
          disabled={openingDir}
          onclick={onOpenDir}
        >
          <FolderOpenIcon />
        </Button>
      {/if}
    </div>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-pressed={autoSave}
      aria-label={autoSave ? m.edit_autosave_on() : m.edit_autosave_off()}
      title={autoSave ? m.edit_autosave_on() : m.edit_autosave_off()}
      class={autoSave ? "bg-accent text-accent-foreground" : ""}
      onclick={onToggleAutoSave}
    >
      <SaveIcon />
    </Button>
    <EditorTypographyPopover />
    <Button size="sm" disabled={!dirty || saving} onclick={onSave}>
      {saving ? m.edit_saving() : m.edit_save()}
    </Button>
  </div>
  <div class="min-h-0 flex-1">
    {#key docKey}
      <ChapterTextView {content} {matches} {jumpTo} {onEdit} />
    {/key}
  </div>
  {#if matches.length > 0}
    <div class="flex max-h-56 shrink-0 flex-col overflow-hidden border-t">
      <ChapterMatches {matches} {content} {stale} {onJump} />
    </div>
  {/if}
</section>
