<script lang="ts">
  // 解析结果卡（哑组件）：两行统计 + 卷章树 + 问题列表；超长卷按 200 章截断。
  // 分卷书按卷折叠（默认收起，点击展开），纯章节单卷平铺；右栏内不套 Card 边框。
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import * as Accordion from "#components/shadcn-svelte/accordion";
  import { Button } from "#components/shadcn-svelte/button";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import type { ContentIssue, ParseOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import ChapterList from "./chapter-list.svelte";

  /** 未分卷时的分组大小：超限按序号分段折叠。 */
  const FLAT_GROUP_SIZE = 100;

  let {
    outline,
    renumbering = false,
    onRenumber,
  }: {
    outline: ParseOutline;
    renumbering?: boolean;
    onRenumber: () => void;
  } = $props();

  /** 强制重排确认框开关：纯 UI 态，执行走父组件回调。 */
  let confirmOpen = $state(false);

  /** 是否按卷折叠：多卷，或单卷有标题时折叠；单个无标题卷按分组规则处理。 */
  const grouped = $derived(outline.volumes.length > 1 || outline.volumes[0]?.title != null);

  /** 未分卷时的序号分组：仅一组时不折叠。 */
  const flatGroups = $derived(
    !grouped && outline.volumes.length === 1
      ? chunk(outline.volumes[0].chapters, FLAT_GROUP_SIZE)
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

  /** 问题总数：逐条问题 + pattern 级单条提示。 */
  const issueCount = $derived(
    outline.issues.length + (outline.numbers_missing ? 1 : 0) + (outline.titles_missing ? 1 : 0),
  );

  /** 问题条目文案：按种类与关联位置组装，标题缺失时回落“未分卷”。 */
  function formatIssue(issue: ContentIssue): string {
    const title = issue.chapter ?? issue.volume ?? m.files_untitled_volume();
    switch (issue.kind) {
      case "MissingNumber":
        return m.files_issue_missing_number({ title });
      case "DuplicateNumber":
        return m.files_issue_duplicate_number({ title, number: issue.number ?? 0 });
      case "MissingChapter":
        if (issue.volume == null) {
          return m.files_issue_missing_volume({ number: issue.number ?? 0 });
        }
        return m.files_issue_missing_chapter({ volume: issue.volume, number: issue.number ?? 0 });
      case "MissingTitle":
        return m.files_issue_missing_title({ title });
    }
  }
</script>

<section aria-label={m.files_parse_result_title()} class="flex flex-col">
  <h2 class="text-sm font-semibold">{m.files_parse_result_title()}</h2>
  <div class="flex flex-col divide-y divide-border">
    <div class="grid grid-cols-2 gap-x-4 py-3">
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.files_outline_volumes()}</p>
        <p class="text-sm tabular-nums">{outline.volume_count}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.files_outline_chapters()}</p>
        <p class="text-sm tabular-nums">{outline.chapter_count}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.files_outline_lines()}</p>
        <p class="text-sm tabular-nums">{outline.line_count}</p>
      </div>
      <div class="flex items-baseline justify-between gap-2">
        <p class="text-sm font-medium">{m.files_outline_chars()}</p>
        <p class="text-sm tabular-nums">{outline.char_count}</p>
      </div>
    </div>
  </div>
  {#if grouped}
    <Accordion.Root type="single" class="w-full">
      {#each outline.volumes as volume, i (i)}
        <Accordion.Item value={`volume-${i}`}>
          <Accordion.Trigger>
            <span class="min-w-0 flex-1 truncate text-left text-sm font-medium">
              {volume.title ?? m.files_untitled_volume()}
            </span>
            <span class="shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
              {m.files_outline_volume_chapters({ count: volume.chapters.length })}
            </span>
          </Accordion.Trigger>
          <Accordion.Content>
            <ChapterList chapters={volume.chapters} />
          </Accordion.Content>
        </Accordion.Item>
      {/each}
    </Accordion.Root>
  {:else if flatGrouped}
    <Accordion.Root type="single" class="w-full">
      {#each flatGroups as group, g (g)}
        {@const start = g * FLAT_GROUP_SIZE + 1}
        <Accordion.Item value={`chapters-${g}`}>
          <Accordion.Trigger>
            <span class="min-w-0 flex-1 truncate text-left text-sm font-medium">
              {m.files_outline_chapter_range({ start, end: start + group.length - 1 })}
            </span>
            <span class="shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
              {m.files_outline_volume_chapters({ count: group.length })}
            </span>
          </Accordion.Trigger>
          <Accordion.Content>
            <ChapterList chapters={group} />
          </Accordion.Content>
        </Accordion.Item>
      {/each}
    </Accordion.Root>
  {:else}
    {#each outline.volumes as volume, i (i)}
      <div class="flex flex-col gap-1 py-3">
        <p class="text-sm font-medium">{volume.title ?? m.files_untitled_volume()}</p>
        <ChapterList chapters={volume.chapters} />
      </div>
    {/each}
  {/if}
  <div class="flex flex-col gap-1 py-3">
    <p class="text-sm font-medium">{m.files_issues_title({ count: issueCount })}</p>
    {#if outline.numbers_missing}
      <div class="flex items-start gap-2">
        <TriangleAlertIcon class="mt-0.5 size-4 shrink-0 text-destructive" />
        <p class="text-sm break-words text-muted-foreground">{m.files_numbers_missing()}</p>
      </div>
    {/if}
    {#if outline.titles_missing}
      <div class="flex items-start gap-2">
        <TriangleAlertIcon class="mt-0.5 size-4 shrink-0 text-destructive" />
        <p class="text-sm break-words text-muted-foreground">{m.files_titles_missing()}</p>
      </div>
    {/if}
    {#each outline.issues as issue, k (k)}
      <div class="flex items-start gap-2">
        <TriangleAlertIcon class="mt-0.5 size-4 shrink-0 text-destructive" />
        <p class="text-sm break-words text-muted-foreground">{formatIssue(issue)}</p>
      </div>
    {:else}
      {#if !outline.numbers_missing && !outline.titles_missing}
        <p class="text-sm break-words text-muted-foreground">{m.files_issues_empty()}</p>
      {/if}
    {/each}
    {#if issueCount > 0}
      <div class="pt-2">
        <Button
          class="w-full"
          variant="outline"
          disabled={renumbering}
          onclick={() => (confirmOpen = true)}
        >
          {m.files_renumber_button()}
        </Button>
      </div>
    {/if}
  </div>
</section>
<ConfirmDialog
  bind:open={confirmOpen}
  title={m.files_renumber_confirm_title()}
  description={m.files_renumber_confirm_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.files_renumber_confirm_ok()}
  onConfirm={onRenumber}
/>
