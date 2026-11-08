<script lang="ts">
  // 中栏编辑器（哑组件）：顶部左标题、右自动保存开关 + 排版快捷与保存 + 正文区 + 命中列表；
  // 保存与自动保存开关仅脏态/开关态展示差异，实际落盘由父组件把守；
  // 正文区按章节键重建，切换章节不串历史、不残留草稿；命中列表仅有命中时出现，点击跳到编辑器对应行。
  import SaveIcon from "@lucide/svelte/icons/save";
  import { Button } from "#components/shadcn-svelte/button";
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
    autoSave = false,
    matches = [],
    stale = false,
    jumpTo = null,
    onEdit,
    onSave,
    onToggleAutoSave,
    onJump,
  }: {
    docKey: string;
    title: string;
    content: string;
    dirty: boolean;
    saving: boolean;
    autoSave?: boolean;
    matches?: LineMatch[];
    stale?: boolean;
    jumpTo?: { line: number; nonce: number } | null;
    onEdit: (text: string) => void;
    onSave: () => void;
    onToggleAutoSave: () => void;
    onJump: (line: number) => void;
  } = $props();
</script>

<section aria-label={m.edit_editor_label()} class="flex h-full min-h-0 flex-col">
  <div class="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
    <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{title}</h2>
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
