<script lang="ts">
  // 识别结果卡（哑组件）：统计置顶 + 卷页树中层独占滚动；
  // 分卷按卷折叠（默认收起），平铺超 100 页按序号分段折叠，
  // 大卷内再按 100 页分组展示，页按钮点选后由步骤页加载预览。
  import * as Accordion from "#components/shadcn-svelte/accordion";
  import type { ComicOutline, ComicPageOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { formatBytes, formatPaddedPageRange, pageFileName } from "./comic-files";

  /** 未分卷时的分组大小：超限按序号分段折叠；大卷内同口径分组。 */
  const PAGE_GROUP_SIZE = 100;

  let {
    outline,
    selectedPath = null,
    onSelect,
  }: {
    outline: ComicOutline;
    selectedPath?: string | null;
    onSelect: (path: string) => void;
  } = $props();

  /** 是否按卷折叠：多卷，或单卷有标题时折叠；单个无标题卷按分组规则处理。 */
  const grouped = $derived(outline.volumes.length > 1 || outline.volumes[0]?.title != null);

  /** 未分卷时的序号分组：仅一组时不折叠。 */
  const flatGroups = $derived(
    !grouped && outline.volumes.length === 1
      ? chunk(outline.volumes[0].pages, PAGE_GROUP_SIZE)
      : [],
  );
  const flatGrouped = $derived(flatGroups.length > 1);

  /** 数组按固定大小分块。 */
  function chunk<T>(items: T[], size: number): T[][] {
    const groups: T[][] = [];
    for (let i = 0; i < items.length; i += size) {
      groups.push(items.slice(i, i + size));
    }
    return groups;
  }

  /** 卷内分组：大卷按 100 页分段展示组头，避免一次性渲染上千按钮。 */
  function volumeGroups(pages: ComicPageOutline[]): ComicPageOutline[][] {
    return chunk(pages, PAGE_GROUP_SIZE);
  }

  /** 卷内序号基址：分组头按卷内序号展示范围。 */
  function groupRange(
    volume: { pages: ComicPageOutline[] },
    groupIndex: number,
    group: ComicPageOutline[],
  ): string {
    const start = groupIndex * PAGE_GROUP_SIZE + 1;
    const range = formatPaddedPageRange(start, start + group.length - 1, volume.pages.length);
    return m.comic_files_outline_page_range({ start: range.start, end: range.end });
  }
</script>

<section aria-label={m.files_parse_result_title()} class="flex h-full min-h-0 flex-col">
  <h2 class="shrink-0 text-sm font-semibold">{m.files_parse_result_title()}</h2>
  <div class="flex shrink-0 flex-col divide-y divide-border border-b border-border">
    <div class="grid grid-cols-2 gap-x-4 py-3">
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.comic_files_source_volumes()}</p>
        <p class="text-sm tabular-nums">{outline.volume_count}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.comic_files_source_pages()}</p>
        <p class="text-sm tabular-nums">{outline.page_count}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.comic_files_source_size()}</p>
        <p class="text-sm tabular-nums">{formatBytes(outline.total_bytes)}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.comic_files_outline_invalid()}</p>
        <p class="text-sm tabular-nums">{outline.ignored_count}</p>
      </div>
    </div>
  </div>
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if grouped}
      <Accordion.Root type="single" class="w-full">
        {#each outline.volumes as volume, i (i)}
          <Accordion.Item
            value={`volume-${i}`}
            class="[&>[data-heading-level]]:sticky [&>[data-heading-level]]:top-0 [&>[data-heading-level]]:z-10 [&>[data-heading-level]]:bg-background"
          >
            <Accordion.Trigger>
              <span class="min-w-0 flex-1 truncate text-left text-sm font-medium">
                {volume.title ?? m.files_untitled_volume()}
              </span>
              <span class="mr-2 shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
                {m.comic_files_outline_volume_pages({ count: volume.pages.length })}
              </span>
            </Accordion.Trigger>
            <Accordion.Content>
              <div class="flex flex-col gap-3">
                {#each volumeGroups(volume.pages) as group, g (g)}
                  {#if volume.pages.length > PAGE_GROUP_SIZE}
                    <p class="text-xs font-medium text-muted-foreground">
                      {groupRange(volume, g, group)}
                    </p>
                  {/if}
                  <div class="flex flex-col gap-0.5">
                    {#each group as page (page.path)}
                      <button
                        type="button"
                        aria-current={selectedPath === page.path ? "true" : undefined}
                        onclick={() => onSelect(page.path)}
                        class="flex items-baseline justify-between gap-2 rounded-sm px-1 py-0.5 text-left text-sm hover:bg-accent"
                        class:bg-accent={selectedPath === page.path}
                      >
                        <span class="min-w-0 flex-1 truncate break-words text-muted-foreground">
                          {pageFileName(page.path)}
                        </span>
                        <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                          {formatBytes(page.size_bytes)}
                        </span>
                      </button>
                    {/each}
                  </div>
                {/each}
              </div>
            </Accordion.Content>
          </Accordion.Item>
        {/each}
      </Accordion.Root>
    {:else if flatGrouped}
      <Accordion.Root type="single" class="w-full">
        {#each flatGroups as group, g (g)}
          {@const start = g * PAGE_GROUP_SIZE + 1}
          {@const end = start + group.length - 1}
          {@const range = formatPaddedPageRange(start, end, outline.page_count)}
          <Accordion.Item
            value={`pages-${g}`}
            class="[&>[data-heading-level]]:sticky [&>[data-heading-level]]:top-0 [&>[data-heading-level]]:z-10 [&>[data-heading-level]]:bg-background"
          >
            <Accordion.Trigger>
              <span class="min-w-0 flex-1 truncate text-left text-sm font-medium">
                {m.comic_files_outline_page_range({ start: range.start, end: range.end })}
              </span>
              <span class="mr-2 shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
                {m.comic_files_outline_volume_pages({ count: group.length })}
              </span>
            </Accordion.Trigger>
            <Accordion.Content>
              <div class="flex flex-col gap-0.5">
                {#each group as page (page.path)}
                  <button
                    type="button"
                    aria-current={selectedPath === page.path ? "true" : undefined}
                    onclick={() => onSelect(page.path)}
                    class="flex items-baseline justify-between gap-2 rounded-sm px-1 py-0.5 text-left text-sm hover:bg-accent"
                    class:bg-accent={selectedPath === page.path}
                  >
                    <span class="min-w-0 flex-1 truncate break-words text-muted-foreground">
                      {pageFileName(page.path)}
                    </span>
                    <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                      {formatBytes(page.size_bytes)}
                    </span>
                  </button>
                {/each}
              </div>
            </Accordion.Content>
          </Accordion.Item>
        {/each}
      </Accordion.Root>
    {:else}
      {#each outline.volumes as volume, i (i)}
        <div class="flex flex-col gap-1 py-3">
          <p class="text-sm font-medium">{volume.title ?? m.files_untitled_volume()}</p>
          <div class="flex flex-col gap-0.5">
            {#each volume.pages as page (page.path)}
              <button
                type="button"
                aria-current={selectedPath === page.path ? "true" : undefined}
                onclick={() => onSelect(page.path)}
                class="flex items-baseline justify-between gap-2 rounded-sm px-1 py-0.5 text-left text-sm hover:bg-accent"
                class:bg-accent={selectedPath === page.path}
              >
                <span class="min-w-0 flex-1 truncate break-words text-muted-foreground">
                  {pageFileName(page.path)}
                </span>
                <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                  {formatBytes(page.size_bytes)}
                </span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</section>
