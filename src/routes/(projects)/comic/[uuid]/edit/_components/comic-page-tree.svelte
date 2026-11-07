<script lang="ts">
  // 漫画页序树（哑组件）：平铺直列，分话用自有可折叠话（默认折叠，悬停展开）；
  // 不用 Accordion——折叠卸载重建会替换被拖节点，且跨组件事件透传不可靠；
  // 拖拽走 SortableJS forceFallback（原生 HTML5 DnD 在 Tauri WebKitGTK 下不可靠，
  // 上游 tauri#6695）：拖拽全程由库操作 DOM（幽灵 + 位移动画），只在 onEnd
  // 落点回调一次，调用方按 DOM 顺序提交 state；无效落点库自动播回滚动画；
  // 话头整行是话排序柄，行上按下只归页排序；改名输入与图标按钮不参与拖拽；
  // 话头支持改名（内联输入）与删空话；只改本地工作话，落盘走父组件保存回调。
  import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";
  import ImageIcon from "@lucide/svelte/icons/image";
  import PencilIcon from "@lucide/svelte/icons/pencil";
  import TrashIcon from "@lucide/svelte/icons/trash-2";
  import { m } from "#libs/i18n/paraglide/messages";
  import { cn } from "#libs/utils/shadcn-svelte";
  import { pageList, readDomOrder, volumeList, type ComicDropOrder } from "./comic-sortable";
  import { findPage, type WorkingVolume } from "./comic-order";
  import { validateVolumeTitle, type VolumeTitleError } from "./comic-order";
  import { tick } from "svelte";
  import { formatBytes, pageFileName } from "../../files/_components/comic-files";

  /** 折叠话悬停展开延迟：给用户把页拖进去的机会，太短易误触。 */
  const HOVER_EXPAND_MS = 450;

  /** 拖后点击屏蔽时长：Sortable 落点后浏览器仍会派发 click，拖拽需吃掉它。 */
  const CLICK_SUPPRESS_MS = 250;

  let {
    volumes,
    selectedPath = null,
    sizes = new Map<string, number>(),
    locateRequest = 0,
    onSelect,
    onDropOrder,
    onRename,
    onDeleteRequest,
  }: {
    volumes: WorkingVolume[];
    selectedPath?: string | null;
    /** 页大小表：path → 字节数，按 path 查表，缺失不展示。 */
    sizes?: Map<string, number>;
    /** 定位请求序号：递增即展开并滚动到当前选中页。 */
    locateRequest?: number;
    onSelect: (path: string) => void;
    /** 落点提交：Sortable onEnd 后按 DOM 顺序一次性提交。 */
    onDropOrder: (order: ComicDropOrder) => void;
    onRename: (volume: number, title: string) => void;
    onDeleteRequest: (volume: number) => void;
  } = $props();

  /** 是否按话折叠：多话，或单话有标题时折叠；单个无标题话平铺。 */
  const grouped = $derived(volumes.length > 1 || volumes[0]?.title != null);
  /** 展开话：存话 id（排序中下标漂移），默认全折叠（拖拽悬停自动展开）。 */
  let openVolumes = $state<string[]>([]);
  /** 拖拽中：行最小高度放大命中区、无障碍播报用；页拖拽才高亮话头。 */
  let dragActive = $state(false);
  let pageDragging = $state(false);
  /** 话末追加高亮：页悬停折叠话头时提示可展开。 */
  let appendHint = $state<string | null>(null);
  /** 改名态：正在编辑的话与草稿，错误内联展示。 */
  let editingVolume = $state<number | null>(null);
  let draft = $state("");
  let editError = $state<VolumeTitleError | null>(null);
  /** 改名输入框：挂载后聚焦（不用 autofocus 属性，a11y 规则不推荐）。 */
  let renameInput: HTMLInputElement | null = $state(null);
  $effect(() => {
    renameInput?.focus();
  });
  /** 左栏根节点：落点读 DOM 顺序与定位滚动的起点。 */
  let navEl: HTMLElement | null = $state(null);
  /** 定位请求去重：只响应序号递增，选中变化不自动跟随。 */
  let lastLocate = 0;
  $effect(() => {
    const request = locateRequest;
    if (request === lastLocate) {
      return;
    }
    lastLocate = request;
    if (selectedPath == null || navEl == null) {
      return;
    }
    const ref = findPage(volumes, selectedPath);
    if (ref == null) {
      return;
    }
    const id = volumes[ref.volume]?.id;
    if (id != null && !openVolumes.includes(id)) {
      openVolumes = [...openVolumes, id];
    }
    // 等展开渲染后再滚动：就近对齐，不抢焦点。
    void tick().then(() => {
      if (navEl == null || selectedPath == null) {
        return;
      }
      for (const row of navEl.querySelectorAll("button[data-page-row]")) {
        if ((row as HTMLElement).dataset.id === selectedPath) {
          row.scrollIntoView?.({ block: "nearest", inline: "nearest" });
          break;
        }
      }
    });
  });
  /** 折叠话悬停展开计时器：命中移走 / 落点时清理。 */
  let expandTimer: ReturnType<typeof setTimeout> | null = null;
  let suppressClickUntil = 0;

  /** 话标识：用内存 id（排序下标漂移），缺失回退下标。 */
  function volumeKey(volume: number): string {
    return volumes[volume]?.id ?? `volume-${volume}`;
  }

  /** 话是否展开。 */
  function isOpen(volume: number): boolean {
    return openVolumes.includes(volumeKey(volume));
  }

  /** 话头点击：展开 / 折叠切换（拖拽刚结束的冒泡点击会被吃掉）。 */
  function toggleVolume(volume: number): void {
    if (Date.now() < suppressClickUntil) {
      return;
    }
    const id = volumeKey(volume);
    openVolumes = isOpen(volume)
      ? openVolumes.filter((entry) => entry !== id)
      : [...openVolumes, id];
  }

  /** 行点选：拖拽刚结束的冒泡点击会被吃掉。 */
  function selectPage(path: string): void {
    if (Date.now() < suppressClickUntil) {
      return;
    }
    onSelect(path);
  }

  /** 清理悬停展开计时器。 */
  function clearExpandTimer(): void {
    if (expandTimer != null) {
      clearTimeout(expandTimer);
      expandTimer = null;
    }
  }

  /** 拖拽开始：置视觉态，挂悬停展开侦测（Sortable fallback 幽灵不拦截命中）。 */
  function handleDragStart(item: HTMLElement): void {
    dragActive = true;
    pageDragging = item.matches("button[data-page-row]");
    window.addEventListener("mousemove", handleDragHover);
  }

  /** 拖拽悬停：只在页拖拽中命中折叠话头才计时展开，话拖拽直接忽略。 */
  function handleDragHover(event: MouseEvent): void {
    if (!pageDragging) {
      clearExpandTimer();
      if (appendHint != null) {
        appendHint = null;
      }
      return;
    }
    const element =
      typeof document.elementFromPoint === "function"
        ? document.elementFromPoint(event.clientX, event.clientY)
        : null;
    const header = element?.closest?.("[data-volume-header]");
    const index = header instanceof HTMLElement ? Number(header.dataset.volumeHeader) : Number.NaN;
    if (!Number.isInteger(index) || isOpen(index)) {
      clearExpandTimer();
      if (appendHint != null) {
        appendHint = null;
      }
      return;
    }
    const id = volumeKey(index);
    if (appendHint !== id) {
      appendHint = id;
    }
    if (expandTimer == null) {
      expandTimer = setTimeout(() => {
        expandTimer = null;
        const current = volumes.findIndex((entry) => entry.id === id);
        if (current >= 0 && !openVolumes.includes(id)) {
          openVolumes = [...openVolumes, id];
        }
      }, HOVER_EXPAND_MS);
    }
  }

  /**
   * 落点提交：绝不手动碰 DOM（已证：Svelte keyed each 能收敛外部搬移后的节点； 反之手动摘节点 + 同序提交会导致 Svelte
   * 跳过协调、节点永久丢失）。 无论落点是否变化都按 DOM 顺序提交，Svelte 负责收敛；无效落点库已播回滚。
   */
  function handleDropEnd(): void {
    const order = navEl == null ? null : readDomOrder(navEl);
    dragActive = false;
    pageDragging = false;
    appendHint = null;
    clearExpandTimer();
    window.removeEventListener("mousemove", handleDragHover);
    suppressClickUntil = Date.now() + CLICK_SUPPRESS_MS;
    if (order != null) {
      onDropOrder(order);
    }
  }

  /** 话是否命中追加高亮。 */
  function appendHinted(volume: number): boolean {
    return appendHint != null && appendHint === volumeKey(volume);
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

{#snippet pageRow(index: number, total: number, path: string)}
  {@const active = selectedPath === path}
  {@const size = sizes.get(path)}
  {@const serial = String(index + 1).padStart(String(total).length, "0")}
  <button
    type="button"
    data-page-row
    data-id={path}
    aria-current={active || undefined}
    onclick={() => selectPage(path)}
    class={cn(
      "flex w-full cursor-grab items-center gap-2 rounded-md px-2 py-1 text-left text-sm active:cursor-grabbing",
      active ? "bg-muted font-medium text-foreground" : "text-muted-foreground hover:bg-muted",
    )}
  >
    <ImageIcon class="size-4 shrink-0 text-muted-foreground/60" />
    <span class="shrink-0 text-xs text-muted-foreground tabular-nums">{serial}</span>
    <span class="min-w-0 flex-1 truncate select-none">{pageFileName(path)}</span>
    {#if size != null}
      <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
        {formatBytes(size)}
      </span>
    {/if}
  </button>
{/snippet}

{#snippet volumeSection(volume: number, title: string | null, pages: string[])}
  {@const open = isOpen(volume)}
  {@const editing = editingVolume === volume}
  {@const hinted = appendHinted(volume)}
  <section
    data-volume-section
    data-id={volumes[volume]?.id}
    aria-label={title ?? m.comic_untitled_episode()}
  >
    <div
      data-volume-header={volume}
      class={cn(
        "sticky top-0 z-10 bg-background px-3",
        hinted && "rounded-md bg-accent/60 ring-1 ring-primary/60 ring-inset",
      )}
    >
      <div class="flex w-full items-center gap-1 rounded-md py-1">
        {#if editing}
          <input
            bind:this={renameInput}
            bind:value={draft}
            data-no-drag
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
            data-volume-handle
            aria-expanded={open}
            onclick={() => toggleVolume(volume)}
            class="flex min-w-0 flex-1 cursor-grab items-center gap-2 rounded-md px-1 py-1 text-left text-sm font-medium active:cursor-grabbing"
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
            data-no-drag
            aria-label={m.comic_edit_episode_rename()}
            title={m.comic_edit_episode_rename()}
            onclick={() => startRename(volume)}
            class="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <PencilIcon class="size-4" />
          </button>
          {#if pages.length === 0}
            <button
              type="button"
              data-no-drag
              aria-label={m.comic_edit_episode_delete()}
              title={m.comic_edit_episode_delete()}
              onclick={() => onDeleteRequest(volume)}
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
        data-volume-list={volumes[volume]?.id}
        use:pageList={{ onDragStart: handleDragStart, onDropEnd: handleDropEnd }}
        class={cn(
          "flex flex-col gap-0.5 px-3 pb-1",
          hinted && "rounded-md bg-accent/60 ring-1 ring-primary/60 ring-inset",
        )}
      >
        {#if pages.length === 0}
          <p class="px-2 py-3 text-center text-xs text-muted-foreground">
            {m.comic_edit_empty_episode()}
          </p>
        {:else}
          {#each pages as path, index (path)}
            {@render pageRow(index, pages.length, path)}
          {/each}
        {/if}
      </div>
    {/if}
  </section>
{/snippet}

<nav
  bind:this={navEl}
  aria-label={m.edit_chapters_title()}
  aria-busy={dragActive || undefined}
  class="flex h-full min-h-0 flex-col pt-3 pb-3 select-none"
>
  {#if grouped}
    <div
      class="flex w-full flex-col"
      use:volumeList={{ onDragStart: handleDragStart, onDropEnd: handleDropEnd }}
    >
      {#each volumes as entry, i (entry.id)}
        {@render volumeSection(i, entry.title, entry.pages)}
      {/each}
    </div>
  {:else}
    <div
      role="group"
      aria-label={volumes[0]?.title ?? m.comic_untitled_episode()}
      data-volume-list={volumes[0]?.id}
      use:pageList={{ onDragStart: handleDragStart, onDropEnd: handleDropEnd }}
      class="flex flex-col gap-0.5 px-3"
    >
      {#each volumes[0]?.pages ?? [] as path, index (path)}
        {@render pageRow(index, volumes[0]?.pages?.length ?? 0, path)}
      {/each}
    </div>
  {/if}
</nav>
