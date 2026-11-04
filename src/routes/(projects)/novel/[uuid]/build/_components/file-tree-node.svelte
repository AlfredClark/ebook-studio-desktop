<script lang="ts">
  // 文件树节点（递归）：文件夹用 Collapsible 折叠（默认展开），文件用 ghost 按钮选中；
  // 实现逻辑对标 shadcn-svelte ai-elements file-tree；图片换图标，只读文件挂锁。
  import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
  import FileIcon from "@lucide/svelte/icons/file";
  import FileImageIcon from "@lucide/svelte/icons/file-image";
  import FolderIcon from "@lucide/svelte/icons/folder";
  import LockIcon from "@lucide/svelte/icons/lock";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Collapsible from "#components/shadcn-svelte/collapsible";
  import type { BuildFileNode } from "#libs/commands/types";
  import FileTreeNode from "./file-tree-node.svelte";
  import { cn } from "#libs/utils/shadcn-svelte";
  import { isImageFile } from "./build-files";

  let {
    node,
    selected,
    onSelect,
  }: {
    node: BuildFileNode;
    selected: string | null;
    onSelect: (path: string) => void;
  } = $props();
</script>

{#if node.children != null}
  <Collapsible.Root open class="group/folder">
    <Collapsible.Trigger>
      {#snippet child({ props })}
        <Button {...props} variant="ghost" size="sm" class="w-full justify-start gap-1.5 px-2">
          <ChevronRightIcon
            class="size-4 shrink-0 transition-transform group-data-[state=open]/folder:rotate-90"
          />
          <FolderIcon class="size-4 shrink-0 text-muted-foreground" />
          <span class="min-w-0 flex-1 truncate text-left">{node.name}</span>
        </Button>
      {/snippet}
    </Collapsible.Trigger>
    <Collapsible.Content>
      <div class="ml-4 flex flex-col">
        {#each node.children as child (child.path)}
          <FileTreeNode node={child} {selected} {onSelect} />
        {/each}
      </div>
    </Collapsible.Content>
  </Collapsible.Root>
{:else}
  {@const active = selected === node.path}
  <Button
    variant="ghost"
    size="sm"
    aria-current={active || undefined}
    onclick={() => onSelect(node.path)}
    class={cn(
      "w-full justify-start gap-1.5 px-2",
      active && "bg-muted font-medium text-foreground",
    )}
  >
    {#if isImageFile(node.path)}
      <FileImageIcon class="size-4 shrink-0 text-muted-foreground" />
    {:else}
      <FileIcon class="size-4 shrink-0 text-muted-foreground" />
    {/if}
    <span class="min-w-0 flex-1 truncate text-left">{node.name}</span>
    {#if !node.editable}
      <LockIcon class="size-3.5 shrink-0 text-muted-foreground" />
    {/if}
  </Button>
{/if}
