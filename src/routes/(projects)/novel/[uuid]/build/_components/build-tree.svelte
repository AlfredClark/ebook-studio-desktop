<script lang="ts">
  // 构建文件树（哑组件）：根节点列表渲染，选中态与回调由父级持有。
  import type { BuildFileNode } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import FileTreeNode from "./file-tree-node.svelte";

  let {
    files,
    selected,
    onSelect,
  }: {
    files: BuildFileNode[];
    selected: string | null;
    onSelect: (path: string) => void;
  } = $props();
</script>

<nav aria-label={m.build_tree_title()} class="flex flex-col gap-0.5 pt-3 pb-3">
  {#each files as node (node.path)}
    <FileTreeNode {node} {selected} {onSelect} />
  {/each}
</nav>
