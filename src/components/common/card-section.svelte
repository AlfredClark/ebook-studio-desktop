<script lang="ts">
  // 通用卡片分组容器：标题 + 说明 + 若干信息行，只负责布局，不持有任何业务状态。
  import type { Snippet } from "svelte";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "#components/shadcn-svelte/card";

  let {
    title,
    description,
    children,
    actions,
  }: {
    title: string;
    description?: string;
    children: Snippet;
    /** 卡头右侧操作区（如打开目录图标按钮），无则不渲染。 */
    actions?: Snippet;
  } = $props();
</script>

<Card>
  <CardHeader>
    <div class="flex items-start justify-between gap-2">
      <div class="min-w-0 flex-1">
        <CardTitle>{title}</CardTitle>
        {#if description}
          <CardDescription>{description}</CardDescription>
        {/if}
      </div>
      {#if actions}
        <div class="flex shrink-0 items-center gap-1">
          {@render actions()}
        </div>
      {/if}
    </div>
  </CardHeader>
  <CardContent class="flex flex-col divide-y divide-border">
    {@render children()}
  </CardContent>
</Card>
