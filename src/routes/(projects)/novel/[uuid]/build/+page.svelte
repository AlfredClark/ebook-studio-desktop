<script lang="ts">
  // 小说构建页：左栏构建产物文件树 + 右栏文件查看 / 编辑；
  // 进页拉取构建树，缺失引导构建；生成文件可编辑保存，固定文件与图片只读；
  // 脏态下切文件先确认，重新构建前弹确认框（清空全量重生成）。
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { BuildOutcome } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import BuildFileView from "./_components/build-file-view.svelte";
  import { formatImageSize, isImageFile, langOf } from "./_components/build-files";
  import BuildTree from "./_components/build-tree.svelte";

  const uuid = $derived(page.params.uuid ?? "");

  let outcome = $state<BuildOutcome | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let building = $state(false);

  /** 当前选中文件：构建根起的相对路径（POSIX）。 */
  let selectedPath = $state<string | null>(null);
  /** 文件快照与编辑器实时文本；二进制文件快照为空串。 */
  let savedContent = $state("");
  let draft = $state("");
  let fileEditable = $state(false);
  let isBinary = $state(false);
  /** 图片预览：`asset:` 地址与字节大小，失败回落不支持提示。 */
  let preview = $state<{ url: string; size: number } | null>(null);
  let fileLoading = $state(false);
  let fileFailed = $state(false);
  let saving = $state(false);
  let formatting = $state(false);
  let formattingAll = $state(false);

  /** 脏态：编辑器文本与落盘快照不一致时保存可用。 */
  const dirty = $derived(draft !== savedContent);
  /** 单文件格式化可用：只整理编辑器当前文本，不读盘，脏态下同样可用。 */
  const canFormatFile = $derived(fileEditable && !saving && !formatting && selectedPath !== null);
  /** 批量格式化可用：脏态先保存；结果与落盘一致。 */
  const canFormatAll = $derived(
    outcome !== null && !dirty && !building && !loading && !formattingAll && !saving,
  );
  /** 文件唯一键：驱动编辑器切文件重建。 */
  const docKey = $derived(selectedPath ?? "");
  /** 切文件确认框：脏态下点选他文件先弹框，默认停留。 */
  let confirmSwitch = $state(false);
  let pendingPath = $state<string | null>(null);
  /** 重构建确认框：清空全量重生成，含手改内容。 */
  let confirmRebuild = $state(false);
  /** 文件树版本：拉取或重构建后递增，驱动树重建并回到默认展开。 */
  let treeVersion = $state(0);

  /** 进页拉取构建树：缺失回落空态引导构建；不断言选中，右侧空态待用户点选。 */
  async function loadTree(): Promise<void> {
    loading = true;
    loadFailed = false;
    outcome = null;
    selectedPath = null;
    const result = await commands.getNovelBuildTree(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.build_load_failed());
      return;
    }
    outcome = result.data;
    treeVersion += 1;
  }

  /** 构建：清空后全量重生成，成功后静默格式化全部文件（成功无感，失败 toast）。 */
  async function runBuild(): Promise<void> {
    if (building) {
      return;
    }
    building = true;
    const result = await commands.buildNovelContent(uuid).result();
    if (result.status === "error") {
      building = false;
      toast.error(m.build_failed());
      return;
    }
    outcome = result.data;
    treeVersion += 1;
    // 重建后面板回到未选中：残留的编辑器快照一并清空，避免旧脏态挡住批量格式化。
    selectedPath = null;
    preview = null;
    fileFailed = false;
    fileEditable = false;
    isBinary = false;
    savedContent = "";
    draft = "";
    const formatted = await commands.formatAllNovelBuildFiles(uuid).result();
    building = false;
    if (formatted.status === "error") {
      toast.error(m.build_format_failed());
    }
  }

  /** 打开文件：取内容快照并同步编辑器，二进制只展示提示，图片另取预览。 */
  async function openFile(path: string): Promise<void> {
    selectedPath = path;
    preview = null;
    fileLoading = true;
    fileFailed = false;
    const result = await commands.getNovelBuildFile(uuid, path).result();
    if (result.status === "error") {
      fileLoading = false;
      fileFailed = true;
      toast.error(m.build_file_load_failed());
      return;
    }
    fileEditable = result.data.editable;
    isBinary = result.data.content == null;
    savedContent = result.data.content ?? "";
    draft = savedContent;
    if (isBinary && isImageFile(path)) {
      await loadPreview(path);
    }
    fileLoading = false;
  }

  /** 图片预览：绝对路径转 `asset:` URL，切走后回来的迟到结果直接丢弃。 */
  async function loadPreview(path: string): Promise<void> {
    const image = await commands.getNovelBuildImagePath(uuid, path).result();
    if (image.status === "error" || selectedPath !== path) {
      return;
    }
    try {
      preview = { url: convertFileSrc(image.data.path), size: image.data.size_bytes };
    } catch {
      preview = null;
    }
  }

  /** 点选文件：同文件直接返回，脏态先弹确认框，否则直接打开。 */
  function requestSelect(path: string): void {
    if (selectedPath === path) {
      return;
    }
    if (dirty) {
      pendingPath = path;
      confirmSwitch = true;
      return;
    }
    void openFile(path);
  }

  /** 保存：落盘当前文本并清脏态。 */
  async function handleSave(): Promise<void> {
    if (selectedPath == null || !dirty || saving) {
      return;
    }
    saving = true;
    const result = await commands.updateNovelBuildFile(uuid, selectedPath, draft).result();
    saving = false;
    if (result.status === "error") {
      toast.error(m.build_save_failed());
      return;
    }
    savedContent = draft;
    toast.success(m.build_save_success());
  }

  /** 文件加载失败重试：选中仍在时重开当前文件。 */
  function retryFile(): void {
    if (selectedPath == null) {
      return;
    }
    void openFile(selectedPath);
  }

  /** 全局保存快捷键：Ctrl/Cmd+S 与保存按钮同等效果，门禁由 handleSave 统一把守。 */
  function handleGlobalKeydown(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void handleSave();
    }
  }

  /** 格式化单个文件：整理编辑器当前文本并回填，未保存的修改随之保留，手动保存落盘。 */
  async function handleFormatFile(): Promise<void> {
    if (selectedPath == null || !canFormatFile) {
      return;
    }
    formatting = true;
    const result = await commands.formatNovelBuildText(uuid, selectedPath, draft).result();
    formatting = false;
    if (result.status === "error") {
      toast.error(m.build_format_file_failed());
      return;
    }
    draft = result.data;
    toast.success(m.build_format_file_done());
  }

  /** 全部格式化：直接落盘全部可编辑文件，当前文件重载刷新。 */
  async function handleFormatAll(): Promise<void> {
    if (!canFormatAll) {
      return;
    }
    formattingAll = true;
    const result = await commands.formatAllNovelBuildFiles(uuid).result();
    formattingAll = false;
    if (result.status === "error") {
      toast.error(m.build_failed());
      return;
    }
    const { formatted, skipped, failed } = result.data;
    if (failed.length > 0) {
      toast.error(m.build_format_failed());
    } else if (skipped.length > 0) {
      toast.success(m.build_format_done_skipped({ formatted, skipped: skipped.length }));
    } else {
      toast.success(m.build_format_done({ count: formatted }));
    }
    if (selectedPath != null) {
      await openFile(selectedPath);
    }
  }

  onMount(() => {
    void loadTree();
  });
</script>

<!-- 小说构建页：左栏文件树 + 右栏文件查看 / 编辑 -->
<svelte:window onkeydown={handleGlobalKeydown} />
<h1 class="sr-only">{m.project_step_build()}</h1>
<div class="flex h-full min-h-0 w-full min-w-0">
  <Resizable.PaneGroup direction="horizontal">
    <Resizable.Pane defaultSize={30} minSize={20} maxSize={45} class="min-w-0">
      <div class="flex h-full min-h-0 flex-col">
        <div class="flex shrink-0 items-center justify-between gap-2 px-3 pt-3">
          <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{m.build_tree_title()}</h2>
          {#if outcome !== null}
            <Button
              variant="outline"
              size="sm"
              disabled={!canFormatAll}
              onclick={() => void handleFormatAll()}
            >
              {formattingAll ? m.build_formatting() : m.build_format_all()}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={building}
              onclick={() => (confirmRebuild = true)}
            >
              {building ? m.build_building() : m.build_rebuild()}
            </Button>
          {/if}
        </div>
        <div class="min-h-0 flex-1 overflow-y-auto">
          {#if loading}
            <p class="px-3 pt-3 text-sm text-muted-foreground" aria-busy="true">
              {m.edit_loading()}
            </p>
          {:else if loadFailed}
            <div class="flex flex-col items-center justify-center gap-3 px-2 py-8 text-center">
              <p class="text-sm text-muted-foreground">{m.build_load_failed()}</p>
              <Button variant="outline" size="sm" onclick={() => void loadTree()}>
                {m.files_retry()}
              </Button>
            </div>
          {:else if outcome !== null}
            {#key treeVersion}
              <BuildTree files={outcome.files} selected={selectedPath} onSelect={requestSelect} />
            {/key}
          {/if}
        </div>
      </div>
    </Resizable.Pane>
    <Resizable.Handle withHandle aria-label={m.build_resize_label()} />
    <Resizable.Pane defaultSize={70} minSize={40} class="min-w-0">
      <div class="h-full min-h-0">
        {#if loading || fileLoading}
          <div class="flex h-full min-h-0 items-center justify-center">
            <p class="text-sm text-muted-foreground" aria-busy="true">{m.edit_loading()}</p>
          </div>
        {:else if loadFailed}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">{m.build_load_failed()}</p>
            <Button variant="outline" size="sm" onclick={() => void loadTree()}>
              {m.files_retry()}
            </Button>
          </div>
        {:else if outcome === null}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-2 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm font-medium">{m.build_empty_title()}</p>
            <p class="text-sm text-muted-foreground">{m.build_empty_description()}</p>
            <Button size="sm" disabled={building} onclick={() => void runBuild()}>
              {building ? m.build_building() : m.build_build()}
            </Button>
          </div>
        {:else if fileFailed}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">{m.build_file_load_failed()}</p>
            {#if selectedPath !== null}
              <Button variant="outline" size="sm" onclick={retryFile}>
                {m.files_retry()}
              </Button>
            {/if}
          </div>
        {:else if selectedPath === null}
          <div
            class="flex h-full min-h-0 flex-col items-center justify-center gap-2 p-6 text-center"
          >
            <FileTextIcon class="size-10 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">{m.build_no_file_selected()}</p>
          </div>
        {:else if isBinary}
          {#if preview !== null && selectedPath !== null && isImageFile(selectedPath)}
            <section aria-label={selectedPath} class="flex h-full min-h-0 flex-col">
              <div class="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
                <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{selectedPath}</h2>
                <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                  {formatImageSize(preview.size)}
                </span>
                <Badge variant="secondary">{m.build_readonly()}</Badge>
              </div>
              <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-4">
                <img
                  src={preview.url}
                  alt={selectedPath}
                  class="max-h-full max-w-full rounded-md border object-contain"
                />
              </div>
            </section>
          {:else}
            <div
              class="flex h-full min-h-0 flex-col items-center justify-center gap-2 p-6 text-center"
            >
              <FileTextIcon class="size-10 text-muted-foreground" />
              <p class="text-sm font-medium">{selectedPath}</p>
              <p class="text-sm text-muted-foreground">{m.build_binary_preview()}</p>
            </div>
          {/if}
        {:else}
          <section aria-label={selectedPath} class="flex h-full min-h-0 flex-col">
            <div class="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
              <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{selectedPath}</h2>
              {#if fileEditable}
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!canFormatFile}
                  onclick={() => void handleFormatFile()}
                >
                  {formatting ? m.build_formatting() : m.build_format()}
                </Button>
                <Button size="sm" disabled={!dirty || saving} onclick={() => void handleSave()}>
                  {saving ? m.build_saving() : m.build_save()}
                </Button>
              {:else}
                <Badge variant="secondary">{m.build_readonly()}</Badge>
              {/if}
            </div>
            <div class="min-h-0 flex-1">
              {#key docKey}
                <BuildFileView
                  content={draft}
                  editable={fileEditable}
                  language={langOf(selectedPath)}
                  onEdit={(text) => (draft = text)}
                />
              {/key}
            </div>
          </section>
        {/if}
      </div>
    </Resizable.Pane>
  </Resizable.PaneGroup>
</div>
<ConfirmDialog
  bind:open={confirmSwitch}
  title={m.edit_unsaved_title()}
  description={m.edit_unsaved_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.edit_unsaved_discard()}
  onConfirm={() => {
    if (pendingPath != null) {
      void openFile(pendingPath);
    }
    pendingPath = null;
  }}
/>
<ConfirmDialog
  bind:open={confirmRebuild}
  title={m.build_rebuild_confirm_title()}
  description={m.build_rebuild_confirm_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.build_rebuild_confirm_ok()}
  onConfirm={() => void runBuild()}
/>
