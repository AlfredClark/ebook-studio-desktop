<script lang="ts">
  // 漫画导出页：进页探针查全产物，表单勾选文件与命名，目录框选定后改名落盘；
  // PDF 与原图均按 content.json 顺序导出，原图保留分话结构。
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { resolve } from "$app/paths";
  import { open } from "@tauri-apps/plugin-dialog";
  import DownloadIcon from "@lucide/svelte/icons/download";
  import { Button } from "#components/shadcn-svelte/button";
  import { Checkbox } from "#components/shadcn-svelte/checkbox";
  import { Input } from "#components/shadcn-svelte/input";
  import { Progress } from "#components/shadcn-svelte/progress";
  import * as RadioGroup from "#components/shadcn-svelte/radio-group";
  import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { Card, CardContent } from "#components/shadcn-svelte/card";
  import CardSection from "#components/common/card-section.svelte";
  import commands from "#libs/commands";
  import type {
    ChapterNumbering,
    ComicExportStatus,
    ComicImagesMode,
    ComicPdfFillMode,
    ComicPdfHorizontalAlignment,
    ComicPdfOptions,
    ComicPdfPageSize,
    ComicPdfVerticalAlignment,
    ExportNaming,
    ExportOutcome,
    TxtTitleKind,
  } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import {
    ensureExportListener,
    exportJobOf,
    refreshExportStatus,
    setExportRunning,
  } from "#libs/hooks/export-progress.svelte";
  import { toast } from "#libs/utils/toast";
  import {
    COMIC_PAGE_TITLE_KINDS,
    COMIC_VOLUME_TITLE_KINDS,
    DEFAULT_COMIC_PAGE_PATTERN,
    DEFAULT_COMIC_VOLUME_PATTERN,
    buildComicPdfOptions,
    canExport,
    isValidCustomName,
    isValidImageTitles,
    namingKinds,
    namingValue,
  } from "./_components/comic-export-form";

  const uuid = $derived(page.params.uuid ?? "");

  /** 跨页面导出作业：切页不丢进度，以 Rust 侧注册表为准回填。 */
  const exportJob = $derived(exportJobOf(uuid));
  const exporting = $derived(exportJob.running);
  /** 导出进度：确定性百分比；`null` 表无可显示进度。 */
  const progress = $derived(
    !exportJob.running || exportJob.total_units === 0
      ? null
      : Math.round((exportJob.done_units / exportJob.total_units) * 100),
  );

  let status = $state<ComicExportStatus | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);
  let checkEpub = $state(false);
  let checkPdf = $state(false);
  let checkImages = $state(false);
  let imagesMode = $state<ComicImagesMode>("folder");
  let checkCover = $state(false);
  let pdfPageSize = $state<ComicPdfPageSize>("average");
  let pdfWidthPx = $state("");
  let pdfHeightPx = $state("");
  let pdfDpi = $state("300");
  let pdfFillMode = $state<ComicPdfFillMode>("contain");
  let pdfHorizontalAlignment = $state<ComicPdfHorizontalAlignment>("center");
  let pdfVerticalAlignment = $state<ComicPdfVerticalAlignment>("center");
  let includePdfCover = $state(false);
  let naming = $state<ExportNaming>("project");
  let customName = $state("");
  let outcome = $state<ExportOutcome | null>(null);
  /** 打开导出目录进行中：按钮禁用态。 */
  let openingDir = $state(false);
  /** 话页标题格式与自定义模板：与构建页同一套逻辑；话默认阿拉伯数字（第n话，不带原名），页默认补零。 */
  let pageKind = $state<TxtTitleKind>("padded");
  let pagePattern = $state("");
  let volumeKind = $state<TxtTitleKind>("arabic");
  let volumePattern = $state("");
  /** 页编号方式：按话重置为现状口径，跨话连续按文档顺序全局累加。 */
  let numbering = $state<ChapterNumbering>("per_volume");
  /** 格式示例：首话/首页各渲染一行；自定义按输入模板实时渲染（防抖），空白模板不展示，失败静默隐藏。 */
  let pagePreview = $state<string | null>(null);
  let volumePreview = $state<string | null>(null);
  let pageNonce = $state(0);
  let volumeNonce = $state(0);
  /** 自定义模板输入防抖定时器：避免每敲一键一次 IPC。 */
  let pageDebounce: ReturnType<typeof setTimeout> | undefined;
  let volumeDebounce: ReturnType<typeof setTimeout> | undefined;

  /** 是否空态：三类产物皆无时引导回文件步骤。 */
  const isEmpty = $derived(
    status !== null && !status.has_content && !status.has_epub && !status.has_cover,
  );
  /** 命名候选：项目名与书名相同合并为一项。 */
  const kinds = $derived(status == null ? [] : namingKinds(status.project_name, status.book_title));
  /** 下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const namingItems = $derived.by(() => {
    if (status == null) {
      return [];
    }
    const { project_name, book_title } = status;
    return kinds.map((kind) => ({
      value: kind,
      label:
        kind === "custom"
          ? m.export_naming_custom()
          : `${kind === "book" ? m.export_naming_book() : m.export_naming_project()} · ${namingValue(kind, project_name, book_title)}`,
    }));
  });
  /** PDF 表单值：固定尺寸未填写或 DPI 越界时返回空值并禁用导出。 */
  const pdfOptions = $derived(
    buildComicPdfOptions({
      page_size: pdfPageSize,
      width_px: pdfWidthPx,
      height_px: pdfHeightPx,
      dpi: pdfDpi,
      fill_mode: pdfFillMode,
      horizontal_alignment: pdfHorizontalAlignment,
      vertical_alignment: pdfVerticalAlignment,
      include_cover: includePdfCover,
    }),
  );
  const pdfHorizontalAlignmentItems = $derived([
    { value: "left" as const, label: m.comic_export_pdf_align_left() },
    { value: "center" as const, label: m.comic_export_pdf_align_center() },
    { value: "right" as const, label: m.comic_export_pdf_align_right() },
  ]);
  const pdfVerticalAlignmentItems = $derived([
    { value: "top" as const, label: m.comic_export_pdf_align_top() },
    { value: "center" as const, label: m.comic_export_pdf_align_center() },
    { value: "bottom" as const, label: m.comic_export_pdf_align_bottom() },
  ]);
  /** 分话书：多话时展示话标题格式与编号方式，单话两种编号等价故隐藏。 */
  const isEpisodeBook = $derived(status !== null && status.volume_count > 1);
  /** 话目录补零提示：10 话以上且话格式不补零时，原图目录在词法排序下可能错序。 */
  const showVolumePadHint = $derived(
    checkImages &&
      (status?.volume_count ?? 0) >= 10 &&
      (volumeKind === "custom" ? !volumePattern.includes("#N") : volumeKind !== "padded"),
  );
  /** 导出可用：有勾选、命名合法、原图标题参数合法、空闲中。 */
  const ready = $derived(
    status !== null &&
      canExport(
        { epub: checkEpub, pdf: checkPdf, images: checkImages, cover: checkCover },
        status,
      ) &&
      (naming !== "custom" || isValidCustomName(customName)) &&
      (!checkImages || isValidImageTitles(pageKind, pagePattern, volumeKind, volumePattern)) &&
      (!checkPdf || pdfOptions !== null) &&
      !exporting &&
      !loading,
  );

  /** 标题格式下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const volumeTitleKindItems = COMIC_VOLUME_TITLE_KINDS.map((kind) => ({
    value: kind,
    label: titleKindLabel(kind),
  }));
  const pageTitleKindItems = COMIC_PAGE_TITLE_KINDS.map((kind) => ({
    value: kind,
    label: titleKindLabel(kind),
  }));

  /** 标题格式下拉文案。 */
  function titleKindLabel(kind: TxtTitleKind): string {
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

  /** 进页探针：所有导出项默认不选中，由用户按需勾选。 */
  async function loadStatus(): Promise<void> {
    loading = true;
    loadFailed = false;
    const result = await commands.getComicExportStatus(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.export_status_failed());
      return;
    }
    status = result.data;
    checkEpub = false;
    checkPdf = false;
    checkImages = false;
    imagesMode = "folder";
    checkCover = false;
    pdfPageSize = "average";
    pdfWidthPx = "";
    pdfHeightPx = "";
    pdfDpi = "300";
    pdfFillMode = "contain";
    pdfHorizontalAlignment = "center";
    pdfVerticalAlignment = "center";
    includePdfCover = false;
    naming = "project";
    customName = "";
    pageKind = "padded";
    pagePattern = "";
    // 话数≥10 时话标题默认补零，避免原图目录在词法排序下错序；10 话以下保持第n话审美。
    volumeKind = status.volume_count >= 10 ? "padded" : "arabic";
    volumePattern = "";
    numbering = "per_volume";
    outcome = null;
  }

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleNamingChange(value: string): void {
    if (value === "project" || value === "book" || value === "custom") {
      naming = value;
    }
  }

  /** 原图形态切换：文件夹 / 压缩包二选一。 */
  function handleImagesModeChange(value: string): void {
    if (value === "folder" || value === "zip") {
      imagesMode = value;
    }
  }

  /** PDF 页面尺寸模式下拉值收窄。 */
  function handlePdfPageSizeChange(value: string): void {
    if (value === "average" || value === "fixed") {
      pdfPageSize = value;
    }
  }

  /** PDF 填充模式下拉值收窄。 */
  function handlePdfFillModeChange(value: string): void {
    if (value === "contain" || value === "cover" || value === "stretch") {
      pdfFillMode = value;
    }
  }

  /** PDF 水平对齐下拉值收窄。 */
  function handlePdfHorizontalAlignmentChange(value: string): void {
    if (value === "left" || value === "center" || value === "right") {
      pdfHorizontalAlignment = value;
    }
  }

  /** PDF 垂直对齐下拉值收窄。 */
  function handlePdfVerticalAlignmentChange(value: string): void {
    if (value === "top" || value === "center" || value === "bottom") {
      pdfVerticalAlignment = value;
    }
  }

  /** 标题格式下拉：同上收窄，切 custom 空模板时预填默认，切换后刷新对应示例。 */
  function handleTitleKindChange(value: string, target: "page" | "volume"): void {
    if (target === "volume") {
      if ((COMIC_VOLUME_TITLE_KINDS as string[]).includes(value)) {
        volumeKind = value as TxtTitleKind;
        if (volumeKind === "custom" && volumePattern.trim() === "") {
          volumePattern = DEFAULT_COMIC_VOLUME_PATTERN;
        }
        void refreshTitlePreview("volume");
      }
      return;
    }
    if ((COMIC_PAGE_TITLE_KINDS as string[]).includes(value)) {
      pageKind = value as TxtTitleKind;
      if (pageKind === "custom" && pagePattern.trim() === "") {
        pagePattern = DEFAULT_COMIC_PAGE_PATTERN;
      }
      void refreshTitlePreview("page");
    }
  }

  /** 编号方式切换：按话重置为现状口径，跨话连续按文档顺序全局累加；刷新页预览。 */
  function handleNumberingChange(value: string): void {
    if (value === "continuous" || value === "per_volume") {
      numbering = value;
      void refreshTitlePreview("page");
    }
  }

  /** 自定义模板输入：更新后防抖 400ms 刷新预览。 */
  function handlePatternInput(target: "page" | "volume", value: string): void {
    if (target === "page") {
      pagePattern = value;
      clearTimeout(pageDebounce);
      pageDebounce = setTimeout(() => void refreshTitlePreview("page"), 400);
    } else {
      volumePattern = value;
      clearTimeout(volumeDebounce);
      volumeDebounce = setTimeout(() => void refreshTitlePreview("volume"), 400);
    }
  }

  /** 格式示例：内置格式取首话/首页渲染，自定义按输入模板实时渲染；空模板不展示，失败静默隐藏；序号防迟到覆盖。 */
  async function refreshTitlePreview(target: "page" | "volume"): Promise<void> {
    if (!checkImages) {
      return;
    }
    const kind = target === "page" ? pageKind : volumeKind;
    const pattern = target === "page" ? pagePattern : volumePattern;
    if (target === "page") {
      pagePreview = null;
    } else {
      volumePreview = null;
    }
    if (kind === "custom" && pattern.trim() === "") {
      return;
    }
    if (target === "page") {
      const nonce = (pageNonce += 1);
      const result = await commands
        .previewComicTxtTitle(uuid, kind, kind === "custom" ? pattern : "", false, numbering)
        .result();
      if (nonce !== pageNonce || result.status === "error") {
        return;
      }
      pagePreview = result.data;
    } else {
      const nonce = (volumeNonce += 1);
      const result = await commands
        .previewComicTxtTitle(uuid, kind, kind === "custom" ? pattern : "", true, numbering)
        .result();
      if (nonce !== volumeNonce || result.status === "error") {
        return;
      }
      volumePreview = result.data;
    }
  }

  /** 导出：目录框选定后直接落盘，取消静默返回；进度经全局监听推送，切页不丢。 */
  async function runExport(): Promise<void> {
    if (!ready || status == null || exportJob.running) {
      return;
    }
    let picked: string | string[] | null;
    try {
      picked = await open({ directory: true, multiple: false });
    } catch {
      toast.error(m.export_failed());
      return;
    }
    if (picked == null) {
      return;
    }
    const dest = Array.isArray(picked) ? picked[0] : picked;
    if (dest == null) {
      return;
    }
    const exportPdfOptions: ComicPdfOptions = pdfOptions ?? {
      page_size: "average",
      width_px: 0,
      height_px: 0,
      dpi: 300,
      fill_mode: "contain",
      horizontal_alignment: "center",
      vertical_alignment: "center",
      include_cover: false,
    };
    setExportRunning(uuid, true);
    const result = await commands
      .exportComicContent(
        uuid,
        dest,
        naming,
        customName,
        {
          epub: checkEpub,
          pdf: checkPdf,
          images: checkImages,
          cover: checkCover,
        },
        {
          chapter_kind: pageKind,
          chapter_pattern: pagePattern,
          volume_kind: volumeKind,
          volume_pattern: volumePattern,
          chapter_numbering: numbering,
        },
        exportPdfOptions,
        imagesMode,
      )
      .result();
    setExportRunning(uuid, false);
    if (result.status === "error") {
      toast.error(m.export_failed());
      return;
    }
    outcome = result.data;
    toast.success(m.export_success());
  }

  /** 打开导出目录：调系统文件管理器定位落盘目录，失败 toast。 */
  async function openExportDir(): Promise<void> {
    if (outcome == null || openingDir) {
      return;
    }
    openingDir = true;
    const result = await commands.openExportDir(outcome.dir).result();
    openingDir = false;
    if (result.status === "error") {
      toast.error(m.export_open_dir_failed());
    }
  }

  onMount(() => {
    ensureExportListener("comic-export-progress");
    void refreshExportStatus(uuid, async () => {
      const status = await commands.getComicExportProgress(uuid).result();
      return status.status === "ok" ? status.data : null;
    });
    void loadStatus();
  });
</script>

<!-- 漫画导出页：产物勾选 + 命名 + PDF / 原图选项 + 目录导出 -->
{#snippet progressView()}
  {#if exporting && progress !== null}
    <div class="flex w-full flex-col gap-1.5" role="status" aria-live="polite">
      <p class="text-sm text-muted-foreground">
        {m.export_running()}
        <span class="tabular-nums">{progress}%</span>
      </p>
      <Progress value={progress} />
    </div>
  {/if}
{/snippet}
<h1 class="sr-only">{m.project_step_export()}</h1>
<div class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 overflow-y-auto p-6">
  {#if loading}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.export_loading()}</p>
    </div>
  {:else if loadFailed}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <p class="text-sm text-muted-foreground">{m.export_status_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadStatus()}>
        {m.files_retry()}
      </Button>
    </div>
  {:else if status !== null && isEmpty}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <DownloadIcon class="size-10 text-muted-foreground" />
      <p class="text-sm font-medium">{m.comic_guide_empty_title()}</p>
      <p class="text-sm text-muted-foreground">{m.comic_guide_empty_description()}</p>
      <Button class="w-full max-w-md" onclick={() => void goto(resolve(`/comic/${uuid}/files`))}>
        {m.comic_guide_goto_files()}
      </Button>
    </div>
  {:else if status !== null}
    <CardSection title={m.export_files_title()}>
      <label class="flex cursor-pointer items-center gap-2 py-3 text-sm">
        <Checkbox
          checked={checkEpub}
          disabled={!status.has_epub || exporting}
          onCheckedChange={(value) => (checkEpub = value === true)}
        />
        <span class="min-w-0 flex-1">
          <span class="font-medium">{m.export_epub_label()}</span>
          {#if status.has_epub && status.epub_name !== null}
            <span class="block truncate font-mono text-xs text-muted-foreground">
              {status.epub_name}
            </span>
          {:else}
            <span class="block text-xs text-muted-foreground">{m.export_missing_epub()}</span>
          {/if}
        </span>
      </label>
      <div class="flex flex-col">
        <label class="flex cursor-pointer items-center gap-2 py-2 text-sm">
          <Checkbox
            checked={checkPdf}
            disabled={!status.has_content || status.page_count === 0 || exporting}
            onCheckedChange={(value) => (checkPdf = value === true)}
          />
          <span class="min-w-0 flex-1">
            <span class="font-medium">{m.comic_export_pdf_label()}</span>
            {#if status.has_content && status.page_count > 0}
              <span class="block text-xs text-muted-foreground">
                {m.comic_export_pdf_summary({
                  pages:
                    status.page_count + (checkPdf && includePdfCover && status.has_cover ? 1 : 0),
                })}
              </span>
            {:else}
              <span class="block text-xs text-muted-foreground">{m.export_missing_txt()}</span>
            {/if}
          </span>
        </label>
        {#if checkPdf}
          <div class="flex items-center gap-x-4 py-2">
            <span class="shrink-0 text-sm font-medium">{m.comic_export_pdf_size_label()}</span>
            <RadioGroup.Root
              value={pdfPageSize}
              onValueChange={handlePdfPageSizeChange}
              class="flex items-center gap-x-4"
            >
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="average"
                  aria-label={m.comic_export_pdf_size_average()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_pdf_size_average()}</span>
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="fixed"
                  aria-label={m.comic_export_pdf_size_fixed()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_pdf_size_fixed()}</span>
              </label>
            </RadioGroup.Root>
          </div>
          {#if pdfPageSize === "fixed"}
            <div class="flex flex-wrap items-center gap-2 py-2">
              <label class="flex items-center gap-2 text-sm">
                <span class="font-medium">{m.comic_export_pdf_width_label()}</span>
                <Input
                  type="number"
                  min={1}
                  step={1}
                  value={pdfWidthPx}
                  aria-label={m.comic_export_pdf_width_label()}
                  aria-invalid={pdfOptions === null}
                  disabled={exporting}
                  oninput={(event) => (pdfWidthPx = event.currentTarget.value)}
                  class="w-28"
                />
              </label>
              <span aria-hidden="true" class="text-muted-foreground">×</span>
              <label class="flex items-center gap-2 text-sm">
                <span class="font-medium">{m.comic_export_pdf_height_label()}</span>
                <Input
                  type="number"
                  min={1}
                  step={1}
                  value={pdfHeightPx}
                  aria-label={m.comic_export_pdf_height_label()}
                  aria-invalid={pdfOptions === null}
                  disabled={exporting}
                  oninput={(event) => (pdfHeightPx = event.currentTarget.value)}
                  class="w-28"
                />
              </label>
            </div>
          {:else}
            <p class="text-xs text-muted-foreground">
              {m.comic_export_pdf_average_hint({ pages: status.page_count })}
            </p>
          {/if}
          <div class="flex flex-wrap items-center gap-2 py-2">
            <label class="flex items-center gap-2 text-sm">
              <span class="font-medium">{m.comic_export_pdf_dpi_label()}</span>
              <Input
                type="number"
                min={72}
                max={600}
                step={1}
                value={pdfDpi}
                aria-label={m.comic_export_pdf_dpi_label()}
                aria-invalid={pdfOptions === null}
                disabled={exporting}
                oninput={(event) => (pdfDpi = event.currentTarget.value)}
                class="w-24"
              />
            </label>
            <span class="text-xs text-muted-foreground">{m.comic_export_pdf_dpi_unit()}</span>
            <span class="min-w-0 flex-1 text-xs text-muted-foreground">
              {m.comic_export_pdf_dpi_hint()}
            </span>
          </div>
          <div class="flex items-center gap-x-4 py-2">
            <span class="shrink-0 text-sm font-medium">{m.comic_export_pdf_fill_label()}</span>
            <RadioGroup.Root
              value={pdfFillMode}
              onValueChange={handlePdfFillModeChange}
              class="flex items-center gap-x-4"
            >
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="contain"
                  aria-label={m.comic_export_pdf_fill_contain()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_pdf_fill_contain()}</span>
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="cover"
                  aria-label={m.comic_export_pdf_fill_cover()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_pdf_fill_cover()}</span>
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="stretch"
                  aria-label={m.comic_export_pdf_fill_stretch()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_pdf_fill_stretch()}</span>
              </label>
            </RadioGroup.Root>
          </div>
          <div class="flex flex-wrap items-center gap-2 py-2">
            <span class="shrink-0 text-sm font-medium">{m.comic_export_pdf_alignment_label()}</span>
            <Select
              type="single"
              value={pdfHorizontalAlignment}
              items={pdfHorizontalAlignmentItems}
              onValueChange={handlePdfHorizontalAlignmentChange}
            >
              <SelectTrigger
                class="w-28"
                aria-label={m.comic_export_pdf_horizontal_alignment_label()}
                disabled={exporting || pdfFillMode === "stretch"}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {#each pdfHorizontalAlignmentItems as item (item.value)}
                    <SelectItem value={item.value} label={item.label}>{item.label}</SelectItem>
                  {/each}
                </SelectGroup>
              </SelectContent>
            </Select>
            <Select
              type="single"
              value={pdfVerticalAlignment}
              items={pdfVerticalAlignmentItems}
              onValueChange={handlePdfVerticalAlignmentChange}
            >
              <SelectTrigger
                class="w-28"
                aria-label={m.comic_export_pdf_vertical_alignment_label()}
                disabled={exporting || pdfFillMode === "stretch"}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {#each pdfVerticalAlignmentItems as item (item.value)}
                    <SelectItem value={item.value} label={item.label}>{item.label}</SelectItem>
                  {/each}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          {#if status.has_cover}
            <label class="flex cursor-pointer items-center gap-2 py-2 text-sm">
              <Checkbox
                checked={includePdfCover}
                disabled={exporting}
                onCheckedChange={(value) => (includePdfCover = value === true)}
              />
              <span>{m.comic_export_pdf_include_cover()}</span>
            </label>
          {/if}
          {#if pdfOptions === null}
            <p role="alert" class="text-xs text-destructive">
              {m.comic_export_pdf_settings_invalid()}
            </p>
          {/if}
        {/if}
      </div>
      <!-- 原图一组：复选 + 文件夹/压缩包 + 标题格式，无分隔线 -->
      <div class="flex flex-col">
        <label class="flex cursor-pointer items-center gap-2 py-2 text-sm">
          <Checkbox
            checked={checkImages}
            disabled={!status.has_content || exporting}
            onCheckedChange={(value) => {
              checkImages = value === true;
              if (checkImages) {
                void refreshTitlePreview("volume");
                void refreshTitlePreview("page");
              }
            }}
          />
          <span class="min-w-0 flex-1">
            <span class="font-medium">{m.comic_export_images_label()}</span>
            {#if status.has_content}
              <span class="block text-xs text-muted-foreground">
                {m.comic_export_summary({
                  volumes: status.volume_count,
                  pages: status.page_count,
                })}
              </span>
            {:else}
              <span class="block text-xs text-muted-foreground">{m.export_missing_txt()}</span>
            {/if}
          </span>
        </label>
        {#if checkImages}
          <div class="flex items-center gap-x-4 py-2">
            <span class="shrink-0 text-sm font-medium">{m.comic_export_images_mode_label()}</span>
            <RadioGroup.Root
              value={imagesMode}
              onValueChange={handleImagesModeChange}
              class="flex items-center gap-x-4"
            >
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="folder"
                  aria-label={m.comic_export_images_folder()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_images_folder()}</span>
              </label>
              <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                <RadioGroup.Item
                  value="zip"
                  aria-label={m.comic_export_images_zip()}
                  disabled={exporting}
                />
                <span class="whitespace-nowrap">{m.comic_export_images_zip()}</span>
              </label>
            </RadioGroup.Root>
          </div>
          {#if isEpisodeBook}
            <div class="flex items-center gap-2 py-2">
              <span class="shrink-0 text-sm font-medium">{m.comic_build_volume_title_label()}</span>
              <Select
                type="single"
                value={volumeKind}
                items={volumeTitleKindItems}
                onValueChange={(value) => handleTitleKindChange(value, "volume")}
              >
                <SelectTrigger
                  class="w-36 shrink-0"
                  aria-label={m.comic_build_volume_title_label()}
                  disabled={exporting}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each COMIC_VOLUME_TITLE_KINDS as kind (kind)}
                    <SelectItem value={kind}>{titleKindLabel(kind)}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
              {#if volumeKind === "custom"}
                <Input
                  type="text"
                  value={volumePattern}
                  oninput={(event) => handlePatternInput("volume", event.currentTarget.value)}
                  placeholder={m.comic_build_volume_custom_placeholder()}
                  aria-label={m.build_title_custom()}
                  disabled={exporting}
                  class="min-w-0 flex-1"
                />
              {:else if volumePreview !== null}
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {m.build_txt_preview_label()}: {volumePreview}
                </span>
              {/if}
            </div>
            {#if volumeKind === "custom"}
              <div class="flex min-h-6 items-center gap-2 py-1">
                {#if volumePreview !== null}
                  <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                    {m.build_txt_preview_label()}: {volumePreview}
                  </span>
                  <span class="shrink-0 text-xs text-muted-foreground">
                    {m.build_txt_custom_legend()}
                  </span>
                {/if}
              </div>
            {/if}
            {#if showVolumePadHint}
              <p class="text-xs text-muted-foreground">{m.comic_export_volume_pad_hint()}</p>
            {/if}
          {/if}
          <div class="flex items-center gap-2 py-2">
            <span class="shrink-0 text-sm font-medium">{m.comic_build_page_title_label()}</span>
            <Select
              type="single"
              value={pageKind}
              items={pageTitleKindItems}
              onValueChange={(value) => handleTitleKindChange(value, "page")}
            >
              <SelectTrigger
                class="w-36 shrink-0"
                aria-label={m.comic_build_page_title_label()}
                disabled={exporting}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each COMIC_PAGE_TITLE_KINDS as kind (kind)}
                  <SelectItem value={kind}>{titleKindLabel(kind)}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
            {#if pageKind === "custom"}
              <Input
                type="text"
                value={pagePattern}
                oninput={(event) => handlePatternInput("page", event.currentTarget.value)}
                placeholder={m.comic_build_page_custom_placeholder()}
                aria-label={m.build_title_custom()}
                disabled={exporting}
                class="min-w-0 flex-1"
              />
            {:else if pagePreview !== null}
              <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                {m.build_txt_preview_label()}: {pagePreview}
              </span>
            {/if}
          </div>
          {#if pageKind === "custom"}
            <div class="flex min-h-6 items-center gap-2 py-1">
              {#if pagePreview !== null}
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                  {m.build_txt_preview_label()}: {pagePreview}
                </span>
                <span class="shrink-0 text-xs text-muted-foreground">
                  {m.build_txt_custom_legend()}
                </span>
              {/if}
            </div>
          {/if}
          {#if isEpisodeBook}
            <div class="flex items-center gap-x-4 py-2">
              <span class="shrink-0 text-sm font-medium">{m.comic_build_numbering_label()}</span>
              <RadioGroup.Root
                value={numbering}
                onValueChange={handleNumberingChange}
                class="flex items-center gap-x-4"
              >
                <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                  <RadioGroup.Item
                    value="continuous"
                    aria-label={m.comic_build_numbering_continuous()}
                    disabled={exporting}
                  />
                  <span class="whitespace-nowrap">{m.comic_build_numbering_continuous()}</span>
                </label>
                <label class="flex cursor-pointer items-center gap-1.5 text-sm">
                  <RadioGroup.Item
                    value="per_volume"
                    aria-label={m.comic_build_numbering_per_volume()}
                    disabled={exporting}
                  />
                  <span class="whitespace-nowrap">{m.comic_build_numbering_per_volume()}</span>
                </label>
              </RadioGroup.Root>
            </div>
          {/if}
        {/if}
      </div>
      <label class="flex cursor-pointer items-center gap-2 py-3 text-sm">
        <Checkbox
          checked={checkCover}
          disabled={!status.has_cover || exporting}
          onCheckedChange={(value) => (checkCover = value === true)}
        />
        <span class="min-w-0 flex-1">
          <span class="font-medium">{m.export_cover_label()}</span>
          {#if status.has_cover && status.cover_ext !== null}
            <span class="block font-mono text-xs text-muted-foreground">
              cover.{status.cover_ext}
            </span>
          {:else}
            <span class="block text-xs text-muted-foreground">{m.export_missing_cover()}</span>
          {/if}
        </span>
      </label>
      <div class="flex items-center gap-2 py-2">
        <span class="shrink-0 text-sm font-medium">{m.export_naming_title()}</span>
        <Select type="single" value={naming} items={namingItems} onValueChange={handleNamingChange}>
          <SelectTrigger
            class="min-w-0 flex-1"
            aria-label={m.export_naming_title()}
            disabled={exporting}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each kinds as kind (kind)}
              <SelectItem value={kind}>
                {kind === "custom"
                  ? m.export_naming_custom()
                  : `${kind === "book" ? m.export_naming_book() : m.export_naming_project()} · ${namingValue(kind, status.project_name, status.book_title)}`}
              </SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>
      {#if naming === "custom"}
        <div class="flex items-center gap-2 py-2">
          <Input
            type="text"
            value={customName}
            oninput={(event) => (customName = event.currentTarget.value)}
            placeholder={m.export_custom_placeholder()}
            aria-label={m.export_naming_custom()}
            disabled={exporting}
            class="min-w-0 flex-1"
          />
        </div>
      {/if}
    </CardSection>
    <Button class="w-full" disabled={!ready} onclick={() => void runExport()}>
      {exporting ? m.export_running() : m.export_start()}
    </Button>
    {@render progressView()}
    {#if outcome !== null}
      <Card>
        <CardContent class="flex items-center gap-2 py-3">
          <span class="shrink-0 text-sm font-medium">{m.export_success()}</span>
          <span
            class="min-w-0 flex-1 truncate font-mono text-sm text-muted-foreground"
            title={outcome.dir}
          >
            {outcome.dir}
          </span>
          <Button
            variant="outline"
            size="sm"
            class="shrink-0"
            disabled={openingDir}
            onclick={() => void openExportDir()}
          >
            {m.export_open_dir()}
          </Button>
        </CardContent>
      </Card>
    {/if}
  {/if}
</div>
