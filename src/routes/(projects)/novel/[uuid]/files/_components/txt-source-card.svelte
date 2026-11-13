<script lang="ts">
  // 原文卡片（哑组件）：展示 sources/content.txt 状态，导入按钮回调由步骤页负责。
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import type { NovelTextStatus } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { formatBytes } from "./novel-text-params";

  let {
    status,
    importing = false,
    openingDir = false,
    onChoose,
    onOpenDir,
  }: {
    status: NovelTextStatus;
    importing: boolean;
    /** 打开原文目录进行中：按钮禁用态。 */
    openingDir?: boolean;
    onChoose: () => void;
    /** 打开原文目录：调系统文件管理器定位 `sources/`，失败 toast 由父组件把守。 */
    onOpenDir: () => void;
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

<CardSection title={m.files_source_section()} description={m.files_source_description()}>
  {#snippet actions()}
    <Button
      variant="ghost"
      size="icon-sm"
      class="shrink-0"
      aria-label={m.files_open_sources_dir()}
      title={m.files_open_sources_dir()}
      disabled={!status.exists || openingDir}
      onclick={onOpenDir}
    >
      <FolderOpenIcon />
    </Button>
  {/snippet}
  {#if status.exists}
    <div class="grid grid-cols-2 gap-x-4 py-3">
      <div class="min-w-0">
        <p class="text-sm font-medium">{m.files_source_size()}</p>
        <p class="text-sm break-words tabular-nums">{formatBytes(status.size_bytes)}</p>
      </div>
      {#if updatedLabel !== ""}
        <div class="min-w-0">
          <p class="text-sm font-medium">{m.files_source_updated()}</p>
          <p class="text-sm break-words tabular-nums">{updatedLabel}</p>
        </div>
      {/if}
    </div>
  {:else}
    <CardRow label={m.files_source_empty()} description={m.files_source_hint()} />
  {/if}
  <div class="flex flex-col gap-2 py-3">
    {#if status.exists}
      <p class="text-xs text-muted-foreground">{m.files_source_reselect_note()}</p>
    {/if}
    <Button class="w-full" disabled={importing} onclick={onChoose}>
      {importing
        ? m.files_source_importing()
        : status.exists
          ? m.files_source_change()
          : m.files_source_choose()}
    </Button>
  </div>
</CardSection>
