<script lang="ts">
  // 中栏编辑器（哑组件）：顶部左标题右保存 + 正文区 + 命中列表；
  // 保存仅脏态可用；正文区按章节键重建，切换章节不串历史、不残留草稿；
  // 命中列表仅有命中时出现，点击跳到编辑器对应行。
  import { Button } from "#components/shadcn-svelte/button";
  import type { LineMatch } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import ChapterMatches from "./chapter-matches.svelte";
  import ChapterTextView from "./chapter-text-view.svelte";

  let {
    docKey,
    title,
    content,
    dirty,
    saving,
    matches = [],
    stale = false,
    jumpTo = null,
    onEdit,
    onSave,
    onJump,
  }: {
    docKey: string;
    title: string;
    content: string;
    dirty: boolean;
    saving: boolean;
    matches?: LineMatch[];
    stale?: boolean;
    jumpTo?: { line: number; nonce: number } | null;
    onEdit: (text: string) => void;
    onSave: () => void;
    onJump: (line: number) => void;
  } = $props();
</script>

<section aria-label={m.edit_editor_label()} class="flex h-full min-h-0 flex-col">
  <div class="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
    <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{title}</h2>
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
    <div class="max-h-56 shrink-0 overflow-y-auto border-t">
      <ChapterMatches {matches} {content} {stale} {onJump} />
    </div>
  {/if}
</section>
