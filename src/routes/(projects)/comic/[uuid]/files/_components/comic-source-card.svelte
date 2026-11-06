<script lang="ts">
  // 原稿卡（哑组件）：展示 sources/images 状态，导入按钮回调由步骤页负责。
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import type { ComicImagesStatus } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { formatBytes } from "./comic-files";

  let {
    status,
    importing = false,
    onChoose,
  }: {
    status: ComicImagesStatus;
    importing: boolean;
    onChoose: () => void;
  } = $props();

  /** 更新时间展示：UTC 落盘转本地时区，不可用时回落原文。 */
  const updatedLabel = $derived.by(() => {
    if (status.updated_at == null) {
      return "";
    }
    const time = new Date(status.updated_at).getTime();
    if (Number.isNaN(time)) {
      return status.updated_at;
    }
    return new Date(time).toLocaleString();
  });
</script>

<CardSection
  title={m.comic_files_source_section()}
  description={m.comic_files_source_description()}
>
  {#if status.exists}
    <div class="grid grid-cols-2 gap-x-4 py-3">
      <div class="min-w-0">
        <p class="text-sm font-medium">{m.comic_files_source_episodes()}</p>
        <p class="text-sm break-words tabular-nums">{status.volume_count}</p>
      </div>
      <div class="min-w-0">
        <p class="text-sm font-medium">{m.comic_files_source_pages()}</p>
        <p class="text-sm break-words tabular-nums">{status.page_count}</p>
      </div>
      <div class="min-w-0">
        <p class="text-sm font-medium">{m.comic_files_source_size()}</p>
        <p class="text-sm break-words tabular-nums">{formatBytes(status.size_bytes)}</p>
      </div>
      {#if updatedLabel !== ""}
        <div class="min-w-0">
          <p class="text-sm font-medium">{m.comic_files_source_updated()}</p>
          <p class="text-sm break-words tabular-nums">{updatedLabel}</p>
        </div>
      {/if}
    </div>
  {:else}
    <CardRow label={m.comic_files_source_empty()} description={m.comic_files_source_hint()} />
  {/if}
  <div class="flex flex-col gap-2 py-3">
    {#if status.exists}
      <p class="text-xs text-muted-foreground">{m.comic_files_source_reselect_note()}</p>
    {/if}
    <Button class="w-full" disabled={importing} onclick={onChoose}>
      {importing
        ? m.comic_files_source_importing()
        : status.exists
          ? m.comic_files_source_change()
          : m.comic_files_source_choose()}
    </Button>
  </div>
</CardSection>
