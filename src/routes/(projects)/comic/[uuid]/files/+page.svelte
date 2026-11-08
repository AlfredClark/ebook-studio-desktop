<script lang="ts">
  // 漫画文件步骤页：左栏原稿目录导入与解析，右栏识别结果树 + 单图预览。
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import ImageIcon from "@lucide/svelte/icons/image";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { ComicImagesStatus, ComicOutline } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import ComicOutlineCard from "./_components/comic-outline-card.svelte";
  import ComicSourceCard from "./_components/comic-source-card.svelte";
  import { formatBytes, pageFileName } from "./_components/comic-files";

  const uuid = $derived(page.params.uuid ?? "");
  let status = $state<ComicImagesStatus | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let importing = $state(false);
  /** 识别大纲：解析成功后落本地态，右栏渲染话页树。 */
  let outline = $state<ComicOutline | null>(null);
  let parsing = $state(false);
  /** 预览选中：相对 images/ 的 POSIX 路径，与大纲数组同源。 */
  let selectedPath = $state<string | null>(null);
  /** 孤儿图片：删页残留的无引用落盘图，左栏展示清理入口；失败静默（不挡主流程）。 */
  let orphans = $state<{ paths: string[]; total_bytes: number } | null>(null);
  /** 清理确认框：纯 UI 态，执行走确认回调。 */
  let orphanConfirmOpen = $state(false);
  let orphanCleaning = $state(false);
  /** 预览图：asset URL 与字节大小，迟到结果按 token 丢弃。 */
  let previewUrl = $state<string | null>(null);
  let previewSize = $state<number | null>(null);
  let previewDims = $state<{ width: number; height: number } | null>(null);
  let previewLoading = $state(false);
  let previewFailed = $state(false);
  let previewToken = 0;

  /** 进页拉取一次：原稿缺目录回落空态，已有 content.json 直接回显大纲，失败走错误态可重试。 */
  async function loadStatus(): Promise<void> {
    loading = true;
    loadFailed = false;
    outline = null;
    selectedPath = null;
    previewUrl = null;
    previewSize = null;
    previewDims = null;
    previewFailed = false;
    const result = await commands.getComicImagesStatus(uuid).result();
    if (result.status === "error") {
      loading = false;
      loadFailed = true;
      toast.error(m.comic_files_load_failed());
      return;
    }
    status = result.data;
    const outlineResult = await commands.getComicContentOutline(uuid).result();
    loading = false;
    if (outlineResult.status === "error") {
      loadFailed = true;
      toast.error(m.files_parse_failed());
      return;
    }
    outline = outlineResult.data;
    void refreshOrphans();
    selectFirstPage();
  }

  /** 默认选中首话首个有效页，便于右侧直接预览。 */
  function selectFirstPage(): void {
    const first = outline?.volumes.find((volume) => volume.pages.length > 0)?.pages[0];
    selectedPath = first?.path ?? null;
    if (selectedPath != null) {
      void loadPreview(selectedPath);
    }
  }

  /** 选目录并导入：对话框取消直接返回；成功刷新状态，失败 toast。 */
  async function handleChoose(): Promise<void> {
    let picked: string | string[] | null;
    try {
      picked = await open({ multiple: false, directory: true });
    } catch {
      toast.error(m.comic_files_source_failed());
      return;
    }
    if (typeof picked !== "string" || picked === "") {
      return;
    }
    // 重选即换源：后端已清空解析、构建与打包产物，成功后 toast 提醒重新解析。
    const wasReselect = status?.exists ?? false;
    importing = true;
    const result = await commands.importComicImages(uuid, picked).result();
    importing = false;
    if (result.status === "error") {
      toast.error(m.comic_files_source_failed());
      return;
    }
    status = result.data;
    // 新原稿使旧大纲失效，待重新解析；孤儿基线（content.json）已清空，同步刷新。
    outline = null;
    selectedPath = null;
    previewUrl = null;
    previewSize = null;
    previewDims = null;
    previewFailed = false;
    void refreshOrphans();
    if (wasReselect) {
      toast.success(m.comic_files_source_reselect_cleared());
    }
  }

  /** 开始解析：扫描 sources/images 并落盘 content.json，大纲进右栏，失败 toast。 */
  async function handleParse(): Promise<void> {
    if (status == null || !status.exists || parsing) {
      return;
    }
    parsing = true;
    const result = await commands.parseComicImages(uuid).result();
    parsing = false;
    if (result.status === "error") {
      toast.error(m.files_parse_failed());
      return;
    }
    outline = result.data;
    void refreshOrphans();
    selectFirstPage();
  }

  /** 孤儿扫描：只读 walk，失败静默（不挡大纲与预览主流程）。 */
  async function refreshOrphans(): Promise<void> {
    const result = await commands.scanComicOrphans(uuid).result();
    orphans = result.status === "ok" ? result.data : null;
  }

  /** 确认清理孤儿：只删无引用图片，成功后整页刷新（状态字节数跟上）。 */
  async function handleOrphanClean(): Promise<void> {
    if (orphanCleaning) {
      return;
    }
    orphanCleaning = true;
    const result = await commands.deleteComicOrphans(uuid).result();
    orphanCleaning = false;
    if (result.status === "error") {
      toast.error(m.comic_files_orphan_clean_failed());
      return;
    }
    toast.success(m.comic_files_orphan_clean_success());
    await loadStatus();
  }

  /** 点选页面：切换选中并加载预览。 */
  function handleSelect(path: string): void {
    if (selectedPath === path) {
      return;
    }
    selectedPath = path;
    void loadPreview(path);
  }

  /** 图片预览：绝对路径转 asset: URL，切走后回来的迟到结果直接丢弃。 */
  async function loadPreview(path: string): Promise<void> {
    const token = (previewToken += 1);
    previewLoading = true;
    previewFailed = false;
    previewDims = null;
    const image = await commands.getComicImagePath(uuid, path).result();
    if (token !== previewToken) {
      return;
    }
    previewLoading = false;
    if (image.status === "error" || selectedPath !== path) {
      if (selectedPath === path) {
        previewFailed = true;
        toast.error(m.comic_files_preview_failed());
      }
      return;
    }
    previewUrl = convertFileSrc(image.data.path);
    previewSize = image.data.size_bytes;
  }

  /** 预览失败重试：选中仍在时重取当前页。 */
  function retryPreview(): void {
    if (selectedPath != null) {
      void loadPreview(selectedPath);
    }
  }

  onMount(() => {
    void loadStatus();
  });
</script>

<h1 class="sr-only">{m.project_step_files()}</h1>
<div class="flex h-full min-h-0 w-full min-w-0">
  <Resizable.PaneGroup direction="horizontal">
    <Resizable.Pane defaultSize={42} minSize={30} maxSize={60} class="min-w-0">
      <div class="flex h-full min-h-0 flex-col gap-4 overflow-auto p-6">
        {#if loading}
          <p class="text-sm text-muted-foreground" aria-busy="true">{m.comic_files_loading()}</p>
        {:else if loadFailed || status === null}
          <div class="flex flex-col items-center justify-center gap-3 text-center">
            <p class="text-sm text-muted-foreground">{m.comic_files_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadStatus()}>
              {m.files_retry()}
            </Button>
          </div>
        {:else}
          <ComicSourceCard {status} {importing} onChoose={() => void handleChoose()} />
          {#if orphans != null && orphans.paths.length > 0}
            <div class="flex flex-col gap-2 py-3">
              <p class="text-sm text-muted-foreground">
                {m.comic_files_orphan_hint({
                  count: orphans.paths.length,
                  size: formatBytes(orphans.total_bytes),
                })}
              </p>
              <Button
                variant="outline"
                class="w-full"
                size="sm"
                disabled={orphanCleaning}
                onclick={() => (orphanConfirmOpen = true)}
              >
                {m.comic_files_orphan_clean()}
              </Button>
            </div>
          {/if}
          <div class="mt-auto flex flex-col gap-2 pt-2">
            {#if !status.exists}
              <p class="text-xs text-muted-foreground">{m.comic_files_parse_empty()}</p>
            {/if}
            <Button
              class="w-full"
              disabled={!status.exists || parsing}
              onclick={() => void handleParse()}
            >
              {parsing ? m.files_parsing() : m.files_parse_start()}
            </Button>
          </div>
        {/if}
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.page_files_resize_label()} />
    <Resizable.Pane class="min-w-0">
      <div class="flex h-full min-h-0 flex-col p-6">
        {#if parsing}
          <p class="text-sm text-muted-foreground" aria-busy="true">{m.files_parsing()}</p>
        {:else if outline !== null}
          <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
            <ComicOutlineCard {outline} {selectedPath} onSelect={handleSelect} />
          </div>
          <div class="flex h-[38%] shrink-0 flex-col gap-2 border-t border-border pt-3">
            {#if selectedPath == null}
              <p class="text-sm text-muted-foreground">{m.comic_files_preview_select_hint()}</p>
            {:else if previewLoading}
              <p class="text-sm text-muted-foreground" aria-busy="true">
                {m.comic_files_preview_loading()}
              </p>
            {:else if previewFailed || previewUrl == null}
              <div class="flex flex-col items-start gap-2">
                <p class="text-sm text-muted-foreground">{m.comic_files_preview_failed()}</p>
                <Button variant="outline" size="sm" onclick={retryPreview}>
                  {m.files_retry()}
                </Button>
              </div>
            {:else}
              <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden">
                <img
                  src={previewUrl}
                  alt={pageFileName(selectedPath)}
                  onload={(event) => {
                    const img = event.currentTarget as HTMLImageElement | null;
                    if (img != null) {
                      previewDims = { width: img.naturalWidth, height: img.naturalHeight };
                    }
                  }}
                  class="max-h-full max-w-full object-contain"
                />
              </div>
              {#if previewSize != null && previewDims != null}
                <p class="shrink-0 text-center text-xs text-muted-foreground tabular-nums">
                  {pageFileName(selectedPath)} · {previewDims.width} × {previewDims.height} ·
                  {formatBytes(previewSize)}
                </p>
              {/if}
            {/if}
          </div>
        {:else}
          <div
            class="flex h-full min-h-0 flex-1 flex-col items-center justify-center gap-2 text-center"
          >
            <ImageIcon class="size-10 text-muted-foreground" />
            <p class="text-sm font-medium">{m.files_parse_result_title()}</p>
            <p class="text-sm text-muted-foreground">{m.comic_files_preview_empty_description()}</p>
          </div>
        {/if}
      </div>
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>
<ConfirmDialog
  bind:open={orphanConfirmOpen}
  title={m.comic_files_orphan_confirm_title()}
  description={orphans == null
    ? ""
    : m.comic_files_orphan_confirm_description({
        count: orphans.paths.length,
        size: formatBytes(orphans.total_bytes),
      })}
  details={orphans == null ? [] : orphans.paths.slice(0, 10)}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_files_orphan_clean()}
  onConfirm={() => void handleOrphanClean()}
/>
