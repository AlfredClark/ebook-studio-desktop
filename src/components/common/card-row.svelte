<script lang="ts">
  // 通用卡片行：左侧文案（标签 + 值或说明）、右侧可选控件插槽；控件状态一律由调用方持有，本组件只做布局。
  // `value` 是这行的答案（版本号、路径、网址），`description` 是解释这个设置干什么的帮助文案，二者样式分档。
  import type { Snippet } from "svelte";

  let {
    label,
    description,
    value,
    control,
  }: { label: string; description?: string; value?: string; control?: Snippet } = $props();
</script>

{#snippet text()}
  <div class="min-w-0">
    <p class="text-sm font-medium">{label}</p>
    {#if value}
      <p class="text-sm break-words tabular-nums">{value}</p>
    {/if}
    {#if description}
      <p class="text-sm break-words text-muted-foreground">{description}</p>
    {/if}
  </div>
{/snippet}

<!-- 有控件时右侧固定 `--control-column-width` 控件列：所有行的文案换行宽度因此一致，
     控件右缘也对齐；无控件时文案占满整行，长值不被无谓截窄 -->
{#if control}
  <div class="grid grid-cols-[minmax(0,1fr)_var(--control-column-width)] items-center gap-4 py-3">
    {@render text()}
    <div class="flex justify-end">{@render control()}</div>
  </div>
{:else}
  <div class="py-3">
    {@render text()}
  </div>
{/if}
