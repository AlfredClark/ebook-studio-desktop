<script lang="ts">
  // 小说构建页：有产物时左栏构建产物文件树 + 右栏文件查看 / 编辑；
  // 缺产物整页引导（与打包页同构）：无解析产物回文件步骤，无构建产物则引导构建；
  // 标题格式在空态构建按钮上方选择，重构建确认后删除产物回到引导；
  // 脏态下切文件先确认。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import FileTextIcon from "@lucide/svelte/icons/file-text";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import { Input } from "#components/shadcn-svelte/input";
  import * as RadioGroup from "#components/shadcn-svelte/radio-group";
  import CardSection from "#components/common/card-section.svelte";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type { BuildOutcome, ChapterNumbering, TxtTitleKind } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import BuildFileView from "./_components/build-file-view.svelte";
  import { formatImageSize, isImageFile, langOf } from "./_components/build-files";
  import {
    BUILD_TITLE_KINDS,
    DEFAULT_BUILD_CHAPTER_PATTERN,
    DEFAULT_BUILD_VOLUME_PATTERN,
    isValidBuildTitles,
  } from "./_components/build-titles";
  import BuildTree from "./_components/build-tree.svelte";

  const uuid = $derived(page.params.uuid ?? "");

  let outcome = $state<BuildOutcome | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let building = $state(false);
  /** 是否有解析产物：无构建树时决定整页引导去文件还是引导构建。 */
  let hasContent = $state(true);
  /** 解析产物卷数：多卷时展示卷标题格式与编号方式，单卷两种编号等价故隐藏。 */
  let volumeCount = $state(0);
  /** 卷章标题格式与自定义模板：重构建沿用当前选择，加载不重置。 */
  let volumeKind = $state<TxtTitleKind>("original");
  let volumePattern = $state("");
  let chapterKind = $state<TxtTitleKind>("original");
  let chapterPattern = $state("");
  /** 章编号方式：分卷重置为现状口径，跨卷连续按文档顺序全局累加。 */
  let numbering = $state<ChapterNumbering>("per_volume");
  /** 格式示例：首卷/首章各渲染一行；原文与自定义按模板渲染，空白模板不展示，失败静默隐藏。 */
  let volumePreview = $state<string | null>(null);
  let chapterPreview = $state<string | null>(null);
  let volumeNonce = $state(0);
  let chapterNonce = $state(0);
  /** 自定义模板输入防抖定时器：避免每敲一键一次 IPC。 */
  let volumeDebounce: ReturnType<typeof setTimeout> | undefined;
  let chapterDebounce: ReturnType<typeof setTimeout> | undefined;

  /** 构建可用：标题格式合法且空闲中。 */
  const titlesValid = $derived(
    isValidBuildTitles(chapterKind, chapterPattern, volumeKind, volumePattern),
  );

  /** 分卷书：多卷时展示卷标题格式与编号方式，单卷两种编号等价故隐藏。 */
  const isVolumeBook = $derived(volumeCount > 1);

  /** 标题格式下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const buildTitleKindItems = BUILD_TITLE_KINDS.map((kind) => ({
    value: kind,
    label: buildTitleKindLabel(kind),
  }));

  /** 标题格式下拉文案。 */
  function buildTitleKindLabel(kind: TxtTitleKind): string {
    switch (kind) {
      case "original":
        return m.build_title_original();
      case "arabic":
        return m.build_title_arabic();
      case "padded":
        return m.build_title_padded();
      case "chinese_lower":
        return m.build_title_chinese_lower();
      case "chinese_upper":
        return m.build_title_chinese_upper();
      case "custom":
        return m.build_title_custom();
    }
  }

  /** 标题格式下拉：同上收窄，切 custom 空模板时预填默认，切换后刷新对应示例。 */
  function handleTitleKindChange(value: string, target: "volume" | "chapter"): void {
    if ((BUILD_TITLE_KINDS as string[]).includes(value)) {
      if (target === "volume") {
        volumeKind = value as TxtTitleKind;
        if (volumeKind === "custom" && volumePattern.trim() === "") {
          volumePattern = DEFAULT_BUILD_VOLUME_PATTERN;
        }
      } else {
        chapterKind = value as TxtTitleKind;
        if (chapterKind === "custom" && chapterPattern.trim() === "") {
          chapterPattern = DEFAULT_BUILD_CHAPTER_PATTERN;
        }
      }
      void refreshTitlePreview(target);
    }
  }

  /** 编号方式切换：分卷重置为现状口径，跨卷连续按文档顺序全局累加；刷新章预览。 */
  function handleNumberingChange(value: string): void {
    if (value === "continuous" || value === "per_volume") {
      numbering = value;
      void refreshTitlePreview("chapter");
    }
  }

  /** 自定义模板输入：更新后防抖 400ms 刷新预览。 */
  function handlePatternInput(target: "volume" | "chapter", value: string): void {
    if (target === "volume") {
      volumePattern = value;
      clearTimeout(volumeDebounce);
      volumeDebounce = setTimeout(() => void refreshTitlePreview("volume"), 400);
    } else {
      chapterPattern = value;
      clearTimeout(chapterDebounce);
      chapterDebounce = setTimeout(() => void refreshTitlePreview("chapter"), 400);
    }
  }

  /** 格式示例：原文与内置取首卷/首章渲染，自定义按输入模板实时渲染；空模板不展示，失败静默隐藏；序号防迟到覆盖。 */
  async function refreshTitlePreview(target: "volume" | "chapter"): Promise<void> {
    const kind = target === "volume" ? volumeKind : chapterKind;
    const pattern = target === "volume" ? volumePattern : chapterPattern;
    if (target === "volume") {
      volumePreview = null;
    } else {
      chapterPreview = null;
    }
    if (kind === "custom" && pattern.trim() === "") {
      return;
    }
    if (target === "volume") {
      const nonce = (volumeNonce += 1);
      const result = await commands
        .previewNovelTxtTitle(uuid, kind, kind === "custom" ? pattern : "", true, numbering)
        // 探针调用：无标题卷是合法空态，静默隐藏，不走自动上报。
        .failed(() => {})
        .result();
      if (nonce !== volumeNonce || result.status === "error") {
        return;
      }
      volumePreview = result.data;
    } else {
      const nonce = (chapterNonce += 1);
      const result = await commands
        .previewNovelTxtTitle(uuid, kind, kind === "custom" ? pattern : "", false, numbering)
        // 探针调用：无章节是合法空态，静默隐藏，不走自动上报。
        .failed(() => {})
        .result();
      if (nonce !== chapterNonce || result.status === "error") {
        return;
      }
      chapterPreview = result.data;
    }
  }

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
  /** 重构建确认框：确认后删除构建产物并回到引导。 */
  let confirmRebuild = $state(false);
  /** 清空构建产物进行中：按钮禁用态。 */
  let clearing = $state(false);
  /** 文件树版本：拉取或重构建后递增，驱动树重建并回到默认展开。 */
  let treeVersion = $state(0);

  /** 进页拉取构建树：缺失时再查解析产物，决定整页引导去文件还是引导构建；卷数决定格式选项可见性，失败不拦页面；不断言选中，右侧空态待用户点选。 */
  async function loadTree(): Promise<void> {
    loading = true;
    loadFailed = false;
    outcome = null;
    selectedPath = null;
    hasContent = true;
    volumeCount = 0;
    const result = await commands.getNovelBuildTree(uuid).result();
    if (result.status === "error") {
      loading = false;
      loadFailed = true;
      toast.error(m.build_load_failed());
      return;
    }
    outcome = result.data;
    // 解析产物卷数：多卷才展示卷标题格式与编号方式；失败不拦页面，只隐藏选项。
    const outlineResult = await commands.getNovelContentOutline(uuid).result();
    if (outlineResult.status === "error") {
      if (outcome === null) {
        loading = false;
        loadFailed = true;
        toast.error(m.build_load_failed());
        return;
      }
    } else {
      const outline = outlineResult.data;
      if (outcome === null) {
        hasContent =
          outline !== null && outline.volumes.some((volume) => volume.chapters.length > 0);
      }
      volumeCount = outline?.volume_count ?? 0;
    }
    loading = false;
    treeVersion += 1;
    // 标题格式示例与解析产物同源，进页即刷（无产物时静默隐藏）。
    void refreshTitlePreview("volume");
    void refreshTitlePreview("chapter");
  }

  /** 重构建：确认后删除构建产物，回到构建引导（标题格式在引导页重选）。 */
  async function runClearBuild(): Promise<void> {
    if (clearing) {
      return;
    }
    clearing = true;
    const result = await commands.clearNovelBuildContent(uuid).result();
    clearing = false;
    if (result.status === "error") {
      toast.error(m.build_failed());
      return;
    }
    // 面板回到未选中：残留的编辑器快照一并清空。
    selectedPath = null;
    preview = null;
    fileFailed = false;
    fileEditable = false;
    isBinary = false;
    savedContent = "";
    draft = "";
    await loadTree();
  }

  /** 构建：清空后全量重生成，成功后静默格式化全部文件（成功无感，失败 toast）。 */
  async function runBuild(): Promise<void> {
    if (building || !titlesValid) {
      return;
    }
    building = true;
    const result = await commands
      .buildNovelContent(uuid, {
        chapter_kind: chapterKind,
        chapter_pattern: chapterPattern,
        volume_kind: volumeKind,
        volume_pattern: volumePattern,
        chapter_numbering: numbering,
      })
      .result();
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

<!-- 小说构建页：有产物时左栏文件树 + 右栏文件查看 / 编辑，缺产物整页引导 -->
<svelte:window onkeydown={handleGlobalKeydown} />
<h1 class="sr-only">{m.project_step_build()}</h1>
{#snippet formatSelectors()}
  <div>
    {#if isVolumeBook}
      <div class="flex flex-wrap items-center gap-2 py-1">
        <span class="shrink-0 text-sm font-medium">{m.build_volume_title_label()}</span>
        <Select
          type="single"
          value={volumeKind}
          items={buildTitleKindItems}
          onValueChange={(value) => handleTitleKindChange(value, "volume")}
        >
          <SelectTrigger
            class="w-36 shrink-0"
            aria-label={m.build_volume_title_label()}
            disabled={building}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each BUILD_TITLE_KINDS as kind (kind)}
              <SelectItem value={kind}>{buildTitleKindLabel(kind)}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
        {#if volumeKind === "custom"}
          <Input
            type="text"
            value={volumePattern}
            oninput={(event) => handlePatternInput("volume", event.currentTarget.value)}
            placeholder={m.build_volume_custom_placeholder()}
            aria-label={m.build_title_custom()}
            disabled={building}
            class="min-w-32 flex-1"
          />
        {:else if volumePreview !== null}
          <span class="max-w-full min-w-0 truncate text-xs text-muted-foreground">
            {m.build_txt_preview_label()}: {volumePreview}
          </span>
        {/if}
      </div>
      {#if volumeKind === "custom"}
        <div class="flex min-h-6 flex-wrap items-center justify-between gap-2 py-1">
          {#if volumePreview !== null}
            <span class="max-w-full min-w-0 truncate text-xs text-muted-foreground">
              {m.build_txt_preview_label()}: {volumePreview}
            </span>
            <span class="shrink-0 text-xs text-muted-foreground">{m.build_txt_custom_legend()}</span
            >
          {/if}
        </div>
      {/if}
    {/if}
    <div class="flex flex-wrap items-center gap-2 py-1">
      <span class="shrink-0 text-sm font-medium">{m.build_chapter_title_label()}</span>
      <Select
        type="single"
        value={chapterKind}
        items={buildTitleKindItems}
        onValueChange={(value) => handleTitleKindChange(value, "chapter")}
      >
        <SelectTrigger
          class="w-36 shrink-0"
          aria-label={m.build_chapter_title_label()}
          disabled={building}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {#each BUILD_TITLE_KINDS as kind (kind)}
            <SelectItem value={kind}>{buildTitleKindLabel(kind)}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
      {#if chapterKind === "custom"}
        <Input
          type="text"
          value={chapterPattern}
          oninput={(event) => handlePatternInput("chapter", event.currentTarget.value)}
          placeholder={m.build_chapter_custom_placeholder()}
          aria-label={m.build_title_custom()}
          disabled={building}
          class="min-w-32 flex-1"
        />
      {:else if chapterPreview !== null}
        <span class="max-w-full min-w-0 truncate text-xs text-muted-foreground">
          {m.build_txt_preview_label()}: {chapterPreview}
        </span>
      {/if}
    </div>
    {#if chapterKind === "custom"}
      <div class="flex min-h-6 flex-wrap items-center justify-between gap-2 py-1">
        {#if chapterPreview !== null}
          <span class="max-w-full min-w-0 truncate text-xs text-muted-foreground">
            {m.build_txt_preview_label()}: {chapterPreview}
          </span>
          <span class="shrink-0 text-xs text-muted-foreground">{m.build_txt_custom_legend()}</span>
        {/if}
      </div>
    {/if}
    {#if isVolumeBook}
      <div class="flex items-center gap-x-4 py-1">
        <span class="shrink-0 text-sm font-medium">{m.build_chapter_numbering_label()}</span>
        <RadioGroup.Root
          value={numbering}
          onValueChange={handleNumberingChange}
          class="flex items-center gap-x-4"
        >
          <label class="flex cursor-pointer items-center gap-1.5 text-sm">
            <RadioGroup.Item
              value="continuous"
              aria-label={m.build_numbering_continuous()}
              disabled={building}
            />
            <span class="whitespace-nowrap">{m.build_numbering_continuous()}</span>
          </label>
          <label class="flex cursor-pointer items-center gap-1.5 text-sm">
            <RadioGroup.Item
              value="per_volume"
              aria-label={m.build_numbering_per_volume()}
              disabled={building}
            />
            <span class="whitespace-nowrap">{m.build_numbering_per_volume()}</span>
          </label>
        </RadioGroup.Root>
      </div>
    {/if}
  </div>
{/snippet}
{#if loading}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.edit_loading()}</p>
    </div>
  </div>
{:else if loadFailed}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <FileTextIcon class="size-10 text-muted-foreground" />
      <p class="text-sm text-muted-foreground">{m.build_load_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadTree()}>
        {m.files_retry()}
      </Button>
    </div>
  </div>
{:else if outcome === null}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <FileTextIcon class="size-10 text-muted-foreground" />
      {#if !hasContent}
        <p class="text-sm font-medium">{m.edit_empty_title()}</p>
        <p class="text-sm text-muted-foreground">{m.edit_empty_description()}</p>
        <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/novel/${uuid}/files`))}>
          {m.edit_goto_files()}
        </Button>
      {:else}
        <p class="text-sm font-medium">{m.build_empty_title()}</p>
        <p class="text-sm text-muted-foreground">{m.build_empty_description()}</p>
        <div class="w-full max-w-xl text-left">
          <CardSection title={m.build_options_title()}>
            {@render formatSelectors()}
          </CardSection>
        </div>
        <Button
          class="w-full max-w-xl"
          disabled={building || !titlesValid}
          onclick={() => void runBuild()}
        >
          {building ? m.build_building() : m.build_build()}
        </Button>
      {/if}
    </div>
  </div>
{:else}
  <div class="flex h-full min-h-0 w-full min-w-0">
    <Resizable.PaneGroup direction="horizontal">
      <Resizable.Pane defaultSize={30} minSize={20} maxSize={45} class="min-w-0">
        <div class="flex h-full min-h-0 flex-col">
          <div class="flex shrink-0 items-center justify-between gap-2 px-3 pt-3">
            <h2 class="min-w-0 flex-1 truncate text-sm font-medium">{m.build_tree_title()}</h2>
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
              disabled={building || clearing}
              onclick={() => (confirmRebuild = true)}
            >
              {building ? m.build_building() : m.build_rebuild()}
            </Button>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto">
            {#key treeVersion}
              <BuildTree files={outcome.files} selected={selectedPath} onSelect={requestSelect} />
            {/key}
          </div>
        </div>
      </Resizable.Pane>
      <Resizable.Handle withHandle aria-label={m.build_resize_label()} />
      <Resizable.Pane defaultSize={70} minSize={40} class="min-w-0">
        <div class="h-full min-h-0">
          {#if fileLoading}
            <div class="flex h-full min-h-0 items-center justify-center">
              <p class="text-sm text-muted-foreground" aria-busy="true">{m.edit_loading()}</p>
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
{/if}
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
  onConfirm={() => void runClearBuild()}
/>
