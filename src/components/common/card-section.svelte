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

  let { title, description, children }: { title: string; description?: string; children: Snippet } =
    $props();
</script>

<Card>
  <CardHeader>
    <CardTitle>{title}</CardTitle>
    {#if description}
      <CardDescription>{description}</CardDescription>
    {/if}
  </CardHeader>
  <CardContent class="flex flex-col divide-y divide-border">
    {@render children()}
  </CardContent>
</Card>
