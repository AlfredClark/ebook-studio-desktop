<script lang="ts">
  // 漫画项目布局：步骤导航 + 内容槽；uuid 透传面包屑，各步骤页填充内容。
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import ProjectStepsNav from "../../_components/project-steps-nav.svelte";

  let { children }: { children: Snippet } = $props();

  const uuid = $derived(page.params.uuid ?? "");
</script>

<div class="flex min-h-0 w-full flex-1 flex-col overflow-hidden">
  <ProjectStepsNav type="comic" {uuid} />
  <div class="flex min-h-0 w-full flex-1 flex-col overflow-hidden">
    {@render children()}
  </div>
</div>
