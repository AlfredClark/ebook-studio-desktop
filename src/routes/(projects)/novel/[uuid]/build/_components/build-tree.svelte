<script lang="ts">
  // 构建文件树（哑组件）：根节点列表渲染，选中态与回调由父级持有；
  // 展开集初值为顶层 `EPUB/` 与 `META-INF/`（`isExpandedByDefault`），进页 / 重构建由父级重建复位。
  import { SvelteSet } from "svelte/reactivity";
  import type { BuildFileNode } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { isExpandedByDefault } from "./build-files";
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

  /** 初始展开集：仅挂载算一次；进页 / 重构建由父级 `{#key}` 重建复位。 */
  function initialExpanded(): SvelteSet<string> {
    return new SvelteSet(
      files
        .filter((node) => node.children != null && isExpandedByDefault(node.path))
        .map((node) => node.path),
    );
  }

  /** 已展开目录：`SvelteSet` 天然响应式，增删即重渲染。 */
  let expanded = initialExpanded();

  /** 折叠切换回写：开则记入，关则移除。 */
  function handleToggle(path: string, open: boolean): void {
    if (open) {
      expanded.add(path);
    } else {
      expanded.delete(path);
    }
  }
</script>

<nav aria-label={m.build_tree_title()} class="flex flex-col gap-0.5 pt-3 pb-3">
  {#each files as node (node.path)}
    <FileTreeNode {node} {selected} {onSelect} {expanded} onToggle={handleToggle} />
  {/each}
</nav>
