<script lang="ts">
  // 原文卡片（哑组件）：展示 sources/content.txt 状态，导入按钮回调由步骤页负责。
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import type { NovelTextStatus } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { formatBytes } from "./novel-text-params";

  let {
    status,
    importing = false,
    onChoose,
  }: {
    status: NovelTextStatus;
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

<CardSection title={m.files_source_section()} description={m.files_source_description()}>
  {#if status.exists}
    <CardRow label={m.files_source_size()} value={formatBytes(status.size_bytes)} />
    {#if updatedLabel !== ""}
      <CardRow label={m.files_source_updated()} value={updatedLabel} />
    {/if}
  {:else}
    <CardRow label={m.files_source_empty()} description={m.files_source_hint()} />
  {/if}
  <div class="py-3">
    <Button class="w-full" disabled={importing} onclick={onChoose}>
      {importing
        ? m.files_source_importing()
        : status.exists
          ? m.files_source_change()
          : m.files_source_choose()}
    </Button>
  </div>
</CardSection>
