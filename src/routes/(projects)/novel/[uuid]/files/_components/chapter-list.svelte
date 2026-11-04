<script lang="ts">
  // 章节列表（哑组件）：标题左、字数右；超 200 章截断并提示剩余。
  import type { ChapterOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";

  /** 单卷展示上限：超长卷截断并提示剩余章数，避免一次性渲染上万行。 */
  const CHAPTER_DISPLAY_LIMIT = 200;

  let { chapters }: { chapters: ChapterOutline[] } = $props();
</script>

<div class="flex flex-col gap-0.5">
  {#each chapters.slice(0, CHAPTER_DISPLAY_LIMIT) as chapter, j (j)}
    <div class="flex items-baseline justify-between gap-2">
      <span class="min-w-0 text-sm break-words text-muted-foreground">
        {chapter.title}
      </span>
      <span class="shrink-0 text-sm text-muted-foreground tabular-nums">
        {m.files_outline_chapter_chars({ count: chapter.chars })}
      </span>
    </div>
  {/each}
  {#if chapters.length > CHAPTER_DISPLAY_LIMIT}
    <span class="block text-sm text-muted-foreground">
      {m.files_outline_more({ count: chapters.length - CHAPTER_DISPLAY_LIMIT })}
    </span>
  {/if}
</div>
