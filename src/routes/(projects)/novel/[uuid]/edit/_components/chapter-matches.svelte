<script lang="ts">
  // 选中章节命中行列表（哑组件）：标题栏固定、内部行列表滚动；
  // 行号 + 正文预览，点击跳到编辑器对应行；
  // 预览取自筛选快照，正文改动后由过期提示说明偏差。
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import type { LineMatch } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";

  let {
    matches,
    content,
    stale,
    onJump,
  }: {
    matches: LineMatch[];
    content: string;
    stale: boolean;
    onJump: (line: number) => void;
  } = $props();

  /** 快照行文本：越界回落空串，不抛错。 */
  function lineText(line: number): string {
    return content.split("\n")[line - 1] ?? "";
  }

  /** 本章命中处总数：标题计数用。 */
  const spanCount = $derived(matches.reduce((total, match) => total + match.spans.length, 0));
</script>

<section
  aria-label={m.edit_filter_matches_title({ count: spanCount })}
  class="flex min-h-0 flex-1 flex-col gap-1 p-3"
>
  <div class="shrink-0 pb-1">
    <div class="flex items-baseline justify-between gap-2">
      <h3 class="text-sm font-medium">{m.edit_filter_matches_title({ count: spanCount })}</h3>
    </div>
    {#if stale}
      <p class="flex items-start gap-1.5 text-xs text-muted-foreground">
        <TriangleAlertIcon class="mt-0.5 size-3.5 shrink-0" />
        <span>{m.edit_filter_stale()}</span>
      </p>
    {/if}
  </div>
  <div class="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
    {#each matches as match (match.line)}
      <button
        type="button"
        onclick={() => onJump(match.line)}
        class="flex w-full items-baseline justify-between gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
      >
        <span class="min-w-0 flex-1 truncate text-muted-foreground">
          {lineText(match.line)}
        </span>
        <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
          {m.edit_filter_line_label({ line: match.line })}
        </span>
      </button>
    {/each}
  </div>
</section>
