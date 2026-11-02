<script lang="ts">
  // (main) 分组布局：仅常规页面走布局容器，特殊页面另起分组即可绕开。
  import type { Snippet } from "svelte";
  import LayoutContainer from "#components/layout/layout-container.svelte";

  let { children }: { children: Snippet } = $props();
</script>

<LayoutContainer>
  {@render children()}
</LayoutContainer>
