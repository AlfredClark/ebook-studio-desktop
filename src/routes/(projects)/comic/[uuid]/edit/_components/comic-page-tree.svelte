<script lang="ts">
  // 漫画页序树（哑组件）：平铺直列，分话用自有可折叠话（默认折叠，拖拽悬停展开）；
  // 不用 Accordion——折叠卸载重建会替换被拖节点导致拖拽夭折，且跨组件事件透传不可靠；
  // 页行原生 HTML5 拖拽（上下半区定插入点），话头整行可拖拽排序；
  // 页拖到话头 / 话内容区为追加，话拖到话头为排序、拖到行忽略；
  // 话头支持改名（内联输入）与删空话；只改本地工作话，落盘走父组件保存回调。
  import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";
  import GripVerticalIcon from "@lucide/svelte/icons/grip-vertical";
  import PencilIcon from "@lucide/svelte/icons/pencil";
  import TrashIcon from "@lucide/svelte/icons/trash-2";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";
  import type { DropTarget, PageRef, VolumeMoveTarget, WorkingVolume } from "./comic-order";
  import { validateVolumeTitle, type VolumeTitleError } from "./comic-order";
  import { formatBytes, pageFileName } from "../../files/_components/comic-files";

  /** 折叠话悬停展开延迟：给用户把页拖进去的机会，太短易误触。 */
  const HOVER_EXPAND_MS = 450;

  /** 拖拽载荷：页带来源与 path 自检，话只带下标。 */
  type DragPayload =
    | { kind: "page"; volume: number; index: number; path: string }
    | { kind: "volume"; index: number };

  let {
    volumes,
    selectedPath = null,
    sizes = new Map<string, number>(),
    onSelect,
    onMove,
    onMoveVolume,
    onRename,
    onDeleteRequest,
  }: {
    volumes: WorkingVolume[];
    selectedPath?: string | null;
    /** 页大小表：path → 字节数，按 path 查表，缺失不展示。 */
    sizes?: Map<string, number>;
    onSelect: (path: string) => void;
    onMove: (from: PageRef, to: DropTarget) => void;
    onMoveVolume: (from: number, to: VolumeMoveTarget) => void;
    onRename: (volume: number, title: string) => void;
    onDeleteRequest: (volume: number) => void;
  } = $props();

  /** 是否按话折叠：多话，或单话有标题时折叠；单个无标题话平铺。 */
  const grouped = $derived(volumes.length > 1 || volumes[0]?.title != null);
  /** 展开话：默认全折叠（拖拽悬停自动展开）。 */
  let openVolumes = $state<string[]>([]);
  /** 拖拽源：页 path 或话下标，行 / 话头半透明 + 放置自检用。 */
  let dragPage = $state<string | null>(null);
  let dragVolume = $state<number | null>(null);
  /** 放置提示：行间插入位、话末追加或话间排序位。 */
  let hint = $state<DropTarget | null>(null);
  /** 改名态：正在编辑的话与草稿，错误内联展示。 */
  let editingVolume = $state<number | null>(null);
  let draft = $state("");
  let editError = $state<VolumeTitleError | null>(null);
  /** 改名输入框：挂载后聚焦（不用 autofocus 属性，a11y 规则不推荐）。 */
  let renameInput: HTMLInputElement | null = $state(null);
  $effect(() => {
    renameInput?.focus();
  });
  /** 折叠话悬停展开计时器：离开 / 放下 / 结束时清理。 */
  let expandTimer: ReturnType<typeof setTimeout> | null = null;

  /** 话标识：下标稳定（本页不跨组件复用，按渲染序即可）。 */
  function volumeId(volume: number): string {
    return `volume-${volume}`;
  }

  /** 话是否展开。 */
  function isOpen(volume: number): boolean {
    return openVolumes.includes(volumeId(volume));
  }

  /** 话头点击：展开 / 折叠切换。 */
  function toggleVolume(volume: number): void {
    const id = volumeId(volume);
    openVolumes = isOpen(volume)
      ? openVolumes.filter((entry) => entry !== id)
      : [...openVolumes, id];
  }

  /** 事件目标元素：Svelte 代理事件下 currentTarget 可能非元素，做收窄。 */
  function targetOf(event: DragEvent): HTMLElement | null {
    return event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
  }

  /** 是否本次拖拽：只响应行内发起的文本载荷，外部文件拖入不参与。 */
  function isInternalDrag(event: DragEvent): boolean {
    return event.dataTransfer?.types.includes("text/plain") ?? false;
  }

  /** 拖拽载荷：解析失败回 null。 */
  function readPayload(event: DragEvent): DragPayload | null {
    const raw = event.dataTransfer?.getData("text/plain") ?? "";
    if (raw === "") {
      return null;
    }
    try {
      const parsed = JSON.parse(raw) as DragPayload;
      if (parsed.kind === "page") {
        if (
          typeof parsed.volume !== "number" ||
          typeof parsed.index !== "number" ||
          typeof parsed.path !== "string"
        ) {
          return null;
        }
        return parsed;
      }
      if (parsed.kind === "volume") {
        if (typeof parsed.index !== "number") {
          return null;
        }
        return parsed;
      }
      return null;
    } catch {
      return null;
    }
  }

  /** 放置边：鼠标在上 / 下半区定 before / after。 */
  function edgeOf(event: DragEvent, element: HTMLElement): "before" | "after" {
    const rect = element.getBoundingClientRect();
    return event.clientY - rect.top < rect.height / 2 ? "before" : "after";
  }

  /** 清理悬停展开计时器。 */
  function clearExpandTimer(): void {
    if (expandTimer != null) {
      clearTimeout(expandTimer);
      expandTimer = null;
    }
  }

  /** 拖拽结束兜底：无论放下与否都清理态。 */
  function resetDragState(): void {
    dragPage = null;
    dragVolume = null;
    hint = null;
    clearExpandTimer();
  }

  /** 行拖起：写入页载荷，标记源行，冒泡到话头前截停。 */
  function handleRowDragStart(event: DragEvent, volume: number, index: number, path: string): void {
    event.stopPropagation();
    event.dataTransfer?.setData(
      "text/plain",
      JSON.stringify({ kind: "page", volume, index, path }),
    );
    if (event.dataTransfer != null) {
      event.dataTransfer.effectAllowed = "move";
    }
    dragPage = path;
  }

  /** 话拖起：写入话载荷，标记源话。 */
  function handleVolumeDragStart(event: DragEvent, volume: number): void {
    event.dataTransfer?.setData("text/plain", JSON.stringify({ kind: "volume", index: volume }));
    if (event.dataTransfer != null) {
      event.dataTransfer.effectAllowed = "move";
    }
    dragVolume = volume;
  }

  /** 行悬停：页载荷定插入提示，话载荷忽略，冒泡到话容器前截停。 */
  function handleRowDragOver(event: DragEvent, volume: number, index: number): void {
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer != null) {
      event.dataTransfer.dropEffect = "move";
    }
    const element = targetOf(event);
    if (element == null) {
      return;
    }
    const payload = readPayload(event);
    if (payload?.kind !== "page") {
      return;
    }
    const edge = edgeOf(event, element);
    const current = hint;
    if (current?.volume !== volume || current?.index !== index || current?.edge !== edge) {
      hint = { volume, index, edge };
    }
  }

  /** 行放下：页载荷自检参照项后提交，话载荷忽略。 */
  function handleRowDrop(event: DragEvent, volume: number, index: number): void {
    event.preventDefault();
    event.stopPropagation();
    const element = targetOf(event);
    const payload = readPayload(event);
    resetDragState();
    if (payload?.kind !== "page" || element == null) {
      return;
    }
    const reference = volumes[volume]?.pages[index];
    if (reference === payload.path) {
      return;
    }
    onMove(
      { volume: payload.volume, index: payload.index },
      { volume, index, edge: edgeOf(event, element) },
    );
  }

  /** 话追加 / 排序悬停：页载荷追加，话载荷排序；折叠话悬停展开。 */
  function handleHeaderDragOver(event: DragEvent, volume: number): void {
    event.preventDefault();
    event.stopPropagation();
    if (!isInternalDrag(event)) {
      return;
    }
    if (event.dataTransfer != null) {
      event.dataTransfer.dropEffect = "move";
    }
    const element = targetOf(event);
    if (element == null) {
      return;
    }
    const payload = readPayload(event);
    if (payload?.kind === "page") {
      const current = hint;
      if (current?.volume !== volume || current?.edge !== "append") {
        hint = { volume, edge: "append" };
      }
      if (!isOpen(volume) && expandTimer == null) {
        expandTimer = setTimeout(() => {
          expandTimer = null;
          if (!isOpen(volume)) {
            openVolumes = [...openVolumes, volumeId(volume)];
          }
        }, HOVER_EXPAND_MS);
      }
      return;
    }
    if (payload?.kind === "volume") {
      if (payload.index === volume) {
        hint = null;
        return;
      }
      const edge = edgeOf(event, element);
      const current = hint;
      if (current?.volume !== volume || current?.index !== undefined || current?.edge !== edge) {
        hint = { volume, edge };
      }
    }
  }

  /** 话头放下：页载荷追加，话载荷按边排序。 */
  function handleHeaderDrop(event: DragEvent, volume: number): void {
    event.preventDefault();
    event.stopPropagation();
    const element = targetOf(event);
    const payload = readPayload(event);
    resetDragState();
    if (payload == null || element == null) {
      return;
    }
    if (payload.kind === "page") {
      onMove({ volume: payload.volume, index: payload.index }, { volume, edge: "append" });
      return;
    }
    if (payload.index === volume) {
      return;
    }
    onMoveVolume(payload.index, { index: volume, edge: edgeOf(event, element) });
  }

  /** 离开行且未进入行内子节点时清提示。 */
  function handleRowDragLeave(event: DragEvent, volume: number, index: number): void {
    const current = targetOf(event);
    if (current == null) {
      return;
    }
    const entered = event.relatedTarget as Node | null;
    if (entered != null && current.contains(entered)) {
      return;
    }
    if (hint?.volume === volume && hint?.index === index && hint?.edge !== "append") {
      hint = null;
    }
  }

  /** 离开话区时清追加提示并取消悬停展开。 */
  function handleVolumeDragLeave(event: DragEvent, volume: number): void {
    const current = targetOf(event);
    if (current == null) {
      return;
    }
    const entered = event.relatedTarget as Node | null;
    if (entered != null && current.contains(entered)) {
      return;
    }
    clearExpandTimer();
    if (hint?.volume === volume && hint?.edge === "append") {
      hint = null;
    }
  }

  /** 根兜底：间隙地带保持拖拽存活，放下即取消。 */
  function handleRootDragOver(event: DragEvent): void {
    event.preventDefault();
    if (event.dataTransfer != null) {
      event.dataTransfer.dropEffect = "move";
    }
  }

  /** 根放下：未命中任何目标时取消本次拖拽。 */
  function handleRootDrop(event: DragEvent): void {
    event.preventDefault();
    resetDragState();
  }

  /** 根离开：彻底离开左栏时清提示。 */
  function handleRootDragLeave(event: DragEvent): void {
    const current = targetOf(event);
    if (current == null) {
      return;
    }
    const entered = event.relatedTarget as Node | null;
    if (entered != null && current.contains(entered)) {
      return;
    }
    hint = null;
    clearExpandTimer();
  }

  /** 行是否命中插入提示。 */
  function hintEdge(volume: number, index: number): "before" | "after" | null {
    if (hint?.volume === volume && hint?.index === index && hint?.edge !== "append") {
      return hint.edge;
    }
    return null;
  }

  /** 话是否命中追加提示。 */
  function appendHinted(volume: number): boolean {
    return hint?.volume === volume && hint?.edge === "append";
  }

  /** 话是否命中排序提示：返回边，无命中回 null。 */
  function volumeHintEdge(volume: number): "before" | "after" | null {
    if (hint?.volume === volume && hint?.index === undefined && hint?.edge !== "append") {
      return hint.edge;
    }
    return null;
  }

  /** 开始改名：草稿回填当前标题，错误清空。 */
  function startRename(volume: number): void {
    editingVolume = volume;
    draft = volumes[volume]?.title ?? "";
    editError = null;
  }

  /** 取消改名：草稿丢弃。 */
  function cancelRename(): void {
    editingVolume = null;
    draft = "";
    editError = null;
  }

  /** 提交改名：客户端先做非空 / 长度 / 重名校验，失败内联提示；通过走父组件。 */
  function commitRename(volume: number): void {
    const taken = volumes.filter((_, i) => i !== volume).map((entry) => entry.title ?? "");
    const error = validateVolumeTitle(draft, taken);
    if (error != null) {
      editError = error;
      return;
    }
    const title = draft.trim();
    editingVolume = null;
    draft = "";
    editError = null;
    onRename(volume, title);
  }
</script>

{#snippet pageRow(volume: number, index: number, path: string)}
  {@const edge = hintEdge(volume, index)}
  {@const active = selectedPath === path}
  {@const size = sizes.get(path)}
  <div class="flex flex-col">
    {#if edge === "before"}
      <div class="mx-1 h-0.5 rounded-full bg-primary" aria-hidden="true"></div>
    {/if}
    <button
      type="button"
      draggable="true"
      aria-current={active || undefined}
      onclick={() => onSelect(path)}
      ondragstart={(event) => handleRowDragStart(event, volume, index, path)}
      ondragover={(event) => handleRowDragOver(event, volume, index)}
      ondragleave={(event) => handleRowDragLeave(event, volume, index)}
      ondrop={(event) => handleRowDrop(event, volume, index)}
      ondragend={resetDragState}
      class={cn(
        "flex w-full cursor-grab items-center gap-2 rounded-md px-2 py-1 text-left text-sm active:cursor-grabbing",
        active ? "bg-muted font-medium text-foreground" : "text-muted-foreground hover:bg-muted",
        dragPage === path && "opacity-50",
      )}
    >
      <GripVerticalIcon class="size-4 shrink-0 text-muted-foreground/60" />
      <span class="shrink-0 text-xs text-muted-foreground tabular-nums">{index + 1}</span>
      <span class="min-w-0 flex-1 truncate select-none">{pageFileName(path)}</span>
      {#if size != null}
        <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
          {formatBytes(size)}
        </span>
      {/if}
    </button>
    {#if edge === "after"}
      <div class="mx-1 h-0.5 rounded-full bg-primary" aria-hidden="true"></div>
    {/if}
  </div>
{/snippet}

{#snippet volumeSection(volume: number, title: string | null, pages: string[])}
  {@const open = isOpen(volume)}
  {@const orderEdge = volumeHintEdge(volume)}
  {@const editing = editingVolume === volume}
  <section aria-label={title ?? m.comic_untitled_episode()}>
    {#if orderEdge === "before"}
      <div class="mx-3 h-0.5 rounded-full bg-primary" aria-hidden="true"></div>
    {/if}
    <div class="sticky top-0 z-10 bg-background px-3">
      <div
        class={cn(
          "flex w-full items-center gap-1 rounded-md py-1",
          appendHinted(volume) && "bg-accent/50",
        )}
      >
        {#if editing}
          <input
            bind:this={renameInput}
            bind:value={draft}
            aria-label={m.comic_edit_episode_rename()}
            placeholder={title ?? ""}
            onkeydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                commitRename(volume);
              } else if (event.key === "Escape") {
                event.preventDefault();
                cancelRename();
              }
            }}
            onblur={() => commitRename(volume)}
            class="min-w-0 flex-1 rounded-md border border-input bg-background px-2 py-1 text-sm outline-none focus-visible:border-ring"
          />
          {#if editError === "empty"}
            <p class="shrink-0 text-xs text-destructive">{m.comic_edit_episode_title_empty()}</p>
          {:else if editError === "duplicate"}
            <p class="shrink-0 text-xs text-destructive">
              {m.comic_edit_episode_title_duplicate()}
            </p>
          {:else if editError === "too_long"}
            <p class="shrink-0 text-xs text-destructive">{m.comic_edit_episode_title_too_long()}</p>
          {/if}
        {:else}
          <button
            type="button"
            aria-expanded={open}
            draggable="true"
            onclick={() => toggleVolume(volume)}
            ondragstart={(event) => handleVolumeDragStart(event, volume)}
            ondragover={(event) => handleHeaderDragOver(event, volume)}
            ondragleave={(event) => handleVolumeDragLeave(event, volume)}
            ondrop={(event) => handleHeaderDrop(event, volume)}
            ondragend={resetDragState}
            class={cn(
              "flex min-w-0 flex-1 cursor-grab items-center gap-2 rounded-md px-1 py-1 text-left text-sm font-medium active:cursor-grabbing",
              dragVolume === volume && "opacity-50",
            )}
          >
            <ChevronDownIcon
              class={cn(
                "size-4 shrink-0 text-muted-foreground transition-transform",
                !open && "-rotate-90",
              )}
            />
            <span class="min-w-0 flex-1 truncate">
              {title ?? m.comic_untitled_episode()}
            </span>
            <span class="shrink-0 text-sm font-normal text-muted-foreground tabular-nums">
              {m.comic_files_outline_episode_pages({ count: pages.length })}
            </span>
          </button>
          <button
            type="button"
            aria-label={m.comic_edit_episode_rename()}
            title={m.comic_edit_episode_rename()}
            onclick={() => startRename(volume)}
            ondragstart={(event) => {
              event.preventDefault();
              event.stopPropagation();
            }}
            class="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <PencilIcon class="size-4" />
          </button>
          {#if pages.length === 0}
            <button
              type="button"
              aria-label={m.comic_edit_episode_delete()}
              title={m.comic_edit_episode_delete()}
              onclick={() => onDeleteRequest(volume)}
              ondragstart={(event) => {
                event.preventDefault();
                event.stopPropagation();
              }}
              class="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-destructive"
            >
              <TrashIcon class="size-4" />
            </button>
          {:else}
            <span
              role="img"
              aria-label={m.comic_edit_episode_delete_disabled_hint()}
              title={m.comic_edit_episode_delete_disabled_hint()}
              class="shrink-0 rounded-md p-1 text-muted-foreground/40"
            >
              <TrashIcon class="size-4" />
            </span>
          {/if}
        {/if}
      </div>
    </div>
    {#if open}
      <div
        role="group"
        aria-label={title ?? m.comic_untitled_episode()}
        ondragover={(event) => handleHeaderDragOver(event, volume)}
        ondragleave={(event) => handleVolumeDragLeave(event, volume)}
        ondrop={(event) => handleHeaderDrop(event, volume)}
        class={cn("flex flex-col gap-0.5 px-3 pb-1", appendHinted(volume) && "bg-accent/50")}
      >
        {#if pages.length === 0}
          <p class="px-2 py-3 text-center text-xs text-muted-foreground">
            {m.comic_edit_empty_episode()}
          </p>
        {:else}
          {#each pages as path, index (path)}
            {@render pageRow(volume, index, path)}
          {/each}
        {/if}
      </div>
    {/if}
    {#if orderEdge === "after"}
      <div class="mx-3 h-0.5 rounded-full bg-primary" aria-hidden="true"></div>
    {/if}
  </section>
{/snippet}

<nav
  aria-label={m.edit_chapters_title()}
  ondragover={handleRootDragOver}
  ondrop={handleRootDrop}
  ondragleave={handleRootDragLeave}
  class="flex h-full min-h-0 flex-col pt-3 pb-3 select-none"
>
  {#if grouped}
    <div class="flex w-full flex-col">
      {#each volumes as entry, i (i)}
        {@render volumeSection(i, entry.title, entry.pages)}
      {/each}
    </div>
  {:else}
    <div
      role="group"
      aria-label={volumes[0]?.title ?? m.comic_untitled_episode()}
      class="flex flex-col gap-0.5 px-3"
      ondragover={(event) => handleHeaderDragOver(event, 0)}
      ondragleave={(event) => handleVolumeDragLeave(event, 0)}
      ondrop={(event) => handleHeaderDrop(event, 0)}
    >
      {#each volumes[0]?.pages ?? [] as path, index (path)}
        {@render pageRow(0, index, path)}
      {/each}
    </div>
  {/if}
</nav>
