<script lang="ts">
  // 左栏章节树（哑组件）：分卷书按卷折叠（默认收起），单个无标题卷全量平铺；
  // 点击章节抛选中，选中态高亮；分组判定与 files 页识别结果卡同口径。
  import * as Accordion from "#components/shadcn-svelte/accordion";
  import type { ChapterOutline, ParseOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";

  let {
    outline,
    selected,
    visible,
    onSelect,
  }: {
    outline: ParseOutline;
    selected: { volume: number; chapter: number } | null;
    /** 可见章节集（`"v:c"`）：筛选激活时仅渲染命中章节，`null` 表全量。 */
    visible: Set<string> | null;
    onSelect: (volume: number, chapter: number) => void;
  } = $props();

  /** 是否按卷折叠：多卷，或单卷有标题时折叠；单个无标题卷平铺。 */
  const grouped = $derived(outline.volumes.length > 1 || outline.volumes[0]?.title != null);

  const isSelected = (volume: number, chapter: number): boolean =>
    selected?.volume === volume && selected?.chapter === chapter;

  /** 卷内可见章节：下标保持原数组位置，跳转与后端寻址同源。 */
  function visibleEntries(volume: number): { chapter: ChapterOutline; index: number }[] {
    const chapters = outline.volumes[volume]?.chapters ?? [];
    return chapters
      .map((chapter, index) => ({ chapter, index }))
      .filter(({ index }) => visible == null || visible.has(`${volume}:${index}`));
  }

  /** 卷头计数：筛选中显示 命中数/总数，否则显示总数。 */
  function volumeCount(volume: number, total: number): string {
    if (visible == null) {
      return m.files_outline_volume_chapters({ count: total });
    }
    return m.edit_filter_volume_matches({ matched: visibleEntries(volume).length, count: total });
  }
</script>

{#snippet chapterButtons(volume: number, entries: { chapter: ChapterOutline; index: number }[])}
  {#each entries as entry (entry.index)}
    {@const active = isSelected(volume, entry.index)}
    <button
      type="button"
      aria-current={active || undefined}
      onclick={() => onSelect(volume, entry.index)}
      class={cn(
        "flex w-full items-baseline justify-between gap-2 rounded-md px-2 py-1 text-left text-sm",
        active ? "bg-muted font-medium text-foreground" : "text-muted-foreground hover:bg-muted",
      )}
    >
      <span class="min-w-0 flex-1 truncate">{entry.chapter.title}</span>
      <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
        {m.files_outline_chapter_chars({ count: entry.chapter.chars })}
      </span>
    </button>
  {/each}
{/snippet}

<nav aria-label={m.edit_chapters_title()} class="flex flex-col gap-0.5 pt-3 pb-3">
  {#if grouped}
    <Accordion.Root type="single" class="w-full">
      {#each outline.volumes as volume, i (i)}
        {@const entries = visibleEntries(i)}
        {#if entries.length > 0}
          <!-- 卷头吸顶：sticky 只能在父盒子内位移，按钮的父级 Header 恰与按钮等高会锁死，
               故直接让 Header 吸顶；滚动容器自身无内边距，标题通栏贴顶，文字缩进由 px-3 补回 -->
          <Accordion.Item
            value={`volume-${i}`}
            class="[&>[data-heading-level]]:sticky [&>[data-heading-level]]:top-0 [&>[data-heading-level]]:z-10 [&>[data-heading-level]]:bg-background [&>[data-heading-level]]:px-3"
          >
            <Accordion.Trigger>
              <span class="min-w-0 flex-1 truncate text-left text-sm font-medium">
                {volume.title ?? m.files_untitled_volume()}
              </span>
              <span class="shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
                {volumeCount(i, volume.chapters.length)}
              </span>
            </Accordion.Trigger>
            <Accordion.Content class="px-3">
              {@render chapterButtons(i, entries)}
            </Accordion.Content>
          </Accordion.Item>
        {/if}
      {/each}
    </Accordion.Root>
  {:else}
    {#each outline.volumes as volume, i (i)}
      {@const entries = visibleEntries(i)}
      {#if entries.length > 0}
        <!-- 平铺单卷标题同样吸顶，与分卷行为一致 -->
        <p class="sticky top-0 z-10 bg-background px-3 text-sm font-medium">
          {volume.title ?? m.files_untitled_volume()}
        </p>
        <div class="flex flex-col gap-0.5 px-3">
          {@render chapterButtons(i, entries)}
        </div>
      {/if}
    {/each}
  {/if}
</nav>
