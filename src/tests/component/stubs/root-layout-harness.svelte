<script lang="ts">
  // 根布局回归探针：真实根布局包裹真实外观 hook，按钮做未落盘的字重预览。
  import RootLayout from "../../../routes/+layout.svelte";
  import { fontState, setFontWeight } from "#libs/hooks/appearance.svelte";
</script>

<RootLayout>
  <button onclick={() => setFontWeight(500, false)}>preview-500</button>
  <span data-testid="weight">{fontState.weight}</span>
</RootLayout>
