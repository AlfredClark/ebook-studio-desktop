<script lang="ts">
  // 漫画编辑页：左栏页序树（拖拽 + Alt+方向键跨话重排，脏态+保存）+ 中栏大图预览 +
  // 右栏页信息占位；无解析产物时沿用步骤引导。
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import ImageIcon from "@lucide/svelte/icons/image";
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import LocateFixedIcon from "@lucide/svelte/icons/locate-fixed";
  import TrashIcon from "@lucide/svelte/icons/trash-2";
  import { Button } from "#components/shadcn-svelte/button";
  import * as Resizable from "#components/shadcn-svelte/resizable";
  import ConfirmDialog from "#components/common/confirm-dialog.svelte";
  import commands from "#libs/commands";
  import type {
    ComicOutline,
    ComicPdfHorizontalAlignment,
    ComicPdfVerticalAlignment,
    ResizeBackground,
    ResizeFill,
    ResizeMode,
    ResizePreview,
    SinglePreview,
  } from "#libs/commands/types";
  import {
    convertJobOf,
    ensureConvertListener,
    refreshConvertStatus,
    setConvertRunning,
  } from "#libs/hooks/comic-convert.svelte";
  import {
    compressJobOf,
    ensureCompressListener,
    refreshCompressStatus,
    setCompressRunning,
  } from "#libs/hooks/comic-compress.svelte";
  import {
    refreshResizeStatus,
    resizeJobOf,
    ensureResizeListener,
    setResizeRunning,
  } from "#libs/hooks/comic-resize.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import ComicStepGuide from "../../_components/comic-step-guide.svelte";
  import ComicPageTree from "./_components/comic-page-tree.svelte";
  import {
    applyDomOrder,
    cloneVolumes,
    createVolume,
    findPage,
    flattenPages,
    movePage,
    moveVolume,
    sameOrder,
    toWorking,
    validateVolumeTitle,
    type DropTarget,
    type PageRef,
    type VolumeTitleError,
    type WorkingVolume,
  } from "./_components/comic-order";
  import {
    accumulateWheelFlip,
    normalizeWheelDelta,
    resolveArrowStep,
  } from "./_components/comic-navigate";
  import ComicToolPanel, { type ComicTool } from "./_components/comic-tool-panel.svelte";
  import { toolWithRunningJob } from "./_components/comic-tools";
  import {
    DEFAULT_RESIZE_HEIGHT_PX,
    DEFAULT_RESIZE_WIDTH_PX,
    buildResizeOptions,
    type ResizeAspectLock,
  } from "./_components/comic-resize";
  import {
    buildSingleOptions,
    defaultSingleForm,
    isSingleDefault,
    type SingleFormValues,
  } from "./_components/single-image";
  import type { ComicDropOrder } from "./_components/comic-sortable";
  import type { CompressPreview, ConvertPreview, ImageTargetFormat } from "#libs/commands/types";
  import { formatBytes, pageFileName } from "../files/_components/comic-files";

  const uuid = $derived(page.params.uuid ?? "");

  let outline = $state<ComicOutline | null>(null);
  let loading = $state(true);
  let loadFailed = $state(false);

  /** 工作话（拖拽只改本地）与已落盘快照：JSON 比对判定脏态。 */
  let working = $state<WorkingVolume[]>([]);
  let saved = $state<WorkingVolume[]>([]);
  const dirty = $derived(!sameOrder(working, saved));
  let saving = $state(false);
  /** 放弃更改确认框：纯 UI 态，执行走回滚回调。 */
  let confirmOpen = $state(false);
  /** 定位请求：递增即通知页序树展开并滚动到当前选中页。 */
  let locateNonce = $state(0);
  /** 打开解析目录进行中：按钮禁用态。 */
  let openingContentDir = $state(false);

  /** 预览选中：相对 images/ 的 POSIX 路径，拖拽后按 path 跟随。 */
  let selectedPath = $state<string | null>(null);
  /** 预览图：asset URL 与字节大小，迟到结果按 token 丢弃。 */
  let previewUrl = $state<string | null>(null);
  let previewSize = $state<number | null>(null);
  let previewDims = $state<{ width: number; height: number } | null>(null);
  let previewLoading = $state(false);
  let previewFailed = $state(false);
  let previewToken = 0;
  /** 预览容器：挂滚轮翻页监听（显式非被动监听以便吞掉翻页手势）。 */
  let previewBox: HTMLElement | null = $state(null);
  /** 滚轮翻页累积量：非渲染态，容器重挂时随 effect 清理重置。 */
  let wheelAcc = 0;

  /** 页大小表：path → 字节数，按 path 查表，拖拽改序不影响。 */
  const pageSizes = $derived(
    new Map(
      (outline?.volumes ?? []).flatMap((volume) =>
        volume.pages.map((page) => [page.path, page.size_bytes] as const),
      ),
    ),
  );

  /** 选中位置：话下标 + 话内序号 + 全局序号，供工具条与右栏展示。 */
  const location = $derived.by(() => {
    if (selectedPath == null) {
      return null;
    }
    const ref = findPage(working, selectedPath);
    if (ref == null) {
      return null;
    }
    const flat = flattenPages(working);
    const volumePages = working[ref.volume]?.pages ?? [];
    return {
      ref,
      title: working[ref.volume]?.title ?? null,
      episode: ref.volume + 1,
      serial: String(ref.index + 1).padStart(String(volumePages.length).length, "0"),
      index: flat.indexOf(selectedPath) + 1,
      total: flat.length,
    };
  });

  /** 进页拉取大纲：缺失回落引导，失败走错误态可重试，正常默认选中首个有效页。 */
  async function loadOutline(): Promise<void> {
    loading = true;
    loadFailed = false;
    outline = null;
    working = [];
    saved = [];
    selectedPath = null;
    const result = await commands.getComicContentOutline(uuid).result();
    loading = false;
    if (result.status === "error") {
      loadFailed = true;
      toast.error(m.comic_edit_load_failed());
      return;
    }
    outline = result.data;
    if (outline == null) {
      return;
    }
    working = toWorking(outline.volumes);
    saved = toWorking(outline.volumes);
    selectFirstPage();
  }

  /** 默认选中首话首个有效页。 */
  function selectFirstPage(): void {
    const first = working.flatMap((volume) => volume.pages)[0] ?? null;
    selectedPath = first;
    if (first != null) {
      void loadPreview(first);
    }
  }

  /** 点选页面：切换选中并加载预览。 */
  function handleSelect(path: string): void {
    if (selectedPath === path) {
      return;
    }
    selectedPath = path;
    void loadPreview(path);
  }

  /** 拖拽/键盘提交移动：只改本地工作话，选中按 path 跟随；转换中锁定。 */
  function handleMove(from: PageRef, to: DropTarget): void {
    if (imageLocked) {
      return;
    }
    working = movePage(working, from, to);
  }

  /** 落点提交：Sortable onEnd 后按 DOM 顺序一次性落本地工作话。 */
  function handleDropOrder(order: ComicDropOrder): void {
    working = applyDomOrder(working, order.volumeIds, order.pages);
  }

  /** 右栏格式转换：参数与执行态；确认弹窗按预览分组逐行展示。 */
  let convertTarget: ImageTargetFormat = $state("jpeg");
  let convertQuality = $state(85);
  let convertWebpLossless = $state(false);
  let convertPngLevel = $state(6);
  let convertForce = $state(false);
  let convertPreview = $state<ConvertPreview | null>(null);
  let toolPreviewToken = 0;
  let convertConfirmOpen = $state(false);
  /** 右栏图片压缩：缩放 + 灰度 + 按源格式重编码；确认弹窗展示参数摘要。 */
  let compressScale = $state(100);
  let compressGrayscale = $state(false);
  let compressQuality = $state(85);
  let compressWebpLossless = $state(false);
  let compressPngLevel = $state(6);
  let compressPreview = $state<CompressPreview | null>(null);
  let compressConfirmOpen = $state(false);
  /** 右栏尺寸调整：统一全量图片尺寸；备份与压缩共用，撤销回到备份时状态。 */
  let resizeMode = $state<ResizeMode>("width");
  let resizeWidth = $state(DEFAULT_RESIZE_WIDTH_PX);
  let resizeHeight = $state(DEFAULT_RESIZE_HEIGHT_PX);
  let resizeAspectLock = $state<ResizeAspectLock>("free");
  let resizeFill = $state<ResizeFill>("contain");
  let resizeBackground = $state<ResizeBackground>("white");
  let resizeHAlign = $state<ComicPdfHorizontalAlignment>("center");
  let resizeVAlign = $state<ComicPdfVerticalAlignment>("center");
  let resizeNoUpscale = $state(true);
  let resizeQuality = $state(85);
  let resizeWebpLossless = $state(false);
  let resizePngLevel = $state(6);
  let resizePreview = $state<ResizePreview | null>(null);
  let resizeConfirmOpen = $state(false);
  let resizePreviewDebounce: ReturnType<typeof setTimeout> | undefined;
  /** 单图处理表单：右栏哑组件双向绑定，本页只读组装。 */
  let singleRotate = $state(0);
  let singleFlipH = $state(false);
  let singleFlipV = $state(false);
  let singleAutoTrim = $state(false);
  let singleTrimThreshold = $state(10);
  let singleTrimMargin = $state("0");
  let singleBrightness = $state(0);
  let singleContrast = $state(0);
  let singleGrayscale = $state(false);
  let singleScale = $state(100);
  let singleQuality = $state(85);
  let singleWebpLossless = $state(false);
  let singlePngLevel = $state(6);
  /** 单图预览：中栏原图/预览切换，真编码落临时槽位；确认后替换源文件（无撤销）。 */
  let singleView = $state<"original" | "preview">("preview");
  let singlePreview = $state<SinglePreview | null>(null);
  let singlePreviewUrl = $state<string | null>(null);
  let singleState = $state<"idle" | "loading" | "ready" | "failed">("idle");
  let singleApplying = $state(false);
  let singleConfirmOpen = $state(false);
  let singleToken = 0;
  let singleDebounce: ReturnType<typeof setTimeout> | undefined;
  /** 跨页面图片作业：切页不丢进度，以 Rust 侧注册表为准回填；转换、压缩与尺寸调整互斥。 */
  const convertJob = $derived(convertJobOf(uuid));
  const compressJob = $derived(compressJobOf(uuid));
  const resizeJob = $derived(resizeJobOf(uuid));
  const toolRunning = $derived(convertJob.running || compressJob.running || resizeJob.running);
  const toolProgress = $derived.by(() => {
    const active = convertJob.running ? convertJob : compressJob.running ? compressJob : resizeJob;
    return active.total === 0 ? null : Math.round((active.done / active.total) * 100);
  });
  /** 图片作业中锁定编辑：除预览翻页外，禁止一切改 content.json 的操作（含单图应用）。 */
  const imageLocked = $derived(toolRunning || singleApplying);

  /** 右栏工具：当前所选工具，单图处理首位。 */
  let tool: ComicTool = $state("single");

  /** 参数是否偏离默认值：重置按钮门禁（按当前工具判定）。 */
  const hasOutcome = $derived.by(() => {
    if (tool === "compress") {
      return (
        compressScale !== 100 ||
        compressGrayscale !== false ||
        compressQuality !== 85 ||
        compressWebpLossless !== false ||
        compressPngLevel !== 6
      );
    }
    if (tool === "resize") {
      return (
        resizeMode !== "width" ||
        resizeWidth !== DEFAULT_RESIZE_WIDTH_PX ||
        resizeHeight !== DEFAULT_RESIZE_HEIGHT_PX ||
        resizeAspectLock !== "free" ||
        resizeFill !== "contain" ||
        resizeBackground !== "white" ||
        resizeHAlign !== "center" ||
        resizeVAlign !== "center" ||
        resizeNoUpscale !== true ||
        resizeQuality !== 85 ||
        resizeWebpLossless !== false ||
        resizePngLevel !== 6
      );
    }
    return (
      convertTarget !== "jpeg" ||
      convertQuality !== 85 ||
      convertWebpLossless !== false ||
      convertPngLevel !== 6 ||
      convertForce !== false
    );
  });

  /** 尺寸调整命令参数：非法输入返回空值并禁用执行。 */
  const resizeOptions = $derived(
    buildResizeOptions({
      mode: resizeMode,
      width_px: resizeWidth,
      height_px: resizeHeight,
      fill: resizeFill,
      background: resizeBackground,
      horizontal_alignment: resizeHAlign,
      vertical_alignment: resizeVAlign,
      no_upscale: resizeNoUpscale,
      quality: resizeQuality,
      webp_lossless: resizeWebpLossless,
      png_level: resizePngLevel,
    }),
  );

  /** 单图表单值：右栏输入的原始值组装，非法由组装函数收敛为空。 */
  const singleForm = $derived<SingleFormValues>({
    rotate_deg: singleRotate,
    flip_h: singleFlipH,
    flip_v: singleFlipV,
    auto_trim: singleAutoTrim,
    trim_threshold: singleTrimThreshold,
    trim_margin: singleTrimMargin,
    brightness: singleBrightness,
    contrast: singleContrast,
    grayscale: singleGrayscale,
    scale_percent: singleScale,
    quality: singleQuality,
    webp_lossless: singleWebpLossless,
    png_level: singlePngLevel,
  });

  /** 单图命令参数：非法输入返回空值并禁用执行。 */
  const singleOptions = $derived(buildSingleOptions(singleForm));

  /** 单图是否直通：全默认时应用无意义。 */
  const singleIsDefault = $derived(isSingleDefault(singleForm));

  /** 单图可应用：预览就绪、选中页在、空闲中；批量作业进行时锁定。 */
  const singleCanApply = $derived(
    tool === "single" &&
      singleOptions !== null &&
      !singleIsDefault &&
      singleState === "ready" &&
      selectedPath !== null &&
      !singleApplying &&
      !toolRunning,
  );

  /** 开始按钮门禁：脏态、保存中、运行中、无可处理页时禁用。 */
  const toolRunnable = $derived.by(() => {
    if (outline == null || dirty || saving || toolRunning || singleApplying) {
      return false;
    }
    if (tool === "compress") {
      return (compressPreview?.total ?? 0) > 0;
    }
    if (tool === "resize") {
      return (resizePreview?.total ?? 0) > 0 && resizeOptions !== null;
    }
    return (convertPreview?.total ?? 0) > 0;
  });

  /** 面板执行门禁：单图走预览就绪口径，批量走原口径。 */
  const panelCanRun = $derived(tool === "single" ? singleCanApply : toolRunnable);

  /** 面板重置门禁：单图偏离默认即允许重置。 */
  const panelHasOutcome = $derived(tool === "single" ? !singleIsDefault : hasOutcome);

  /** 中栏是否在看预览：单图工具 + 预览就绪 + 切到预览档。 */
  const viewingPreview = $derived(
    tool === "single" && singleView === "preview" && singlePreview !== null,
  );

  /** 中栏展示尺寸/体积：看预览时用预览值，否则用原图值。 */
  const displayedDims = $derived(
    viewingPreview && singlePreview !== null
      ? { width: singlePreview.width_px, height: singlePreview.height_px }
      : previewDims,
  );
  const displayedSize = $derived(
    viewingPreview && singlePreview !== null ? singlePreview.size_bytes : previewSize,
  );

  /** 转换预览跟随：工具 / 目标格式 / 强制开关 / 大纲变化时刷新分组。 */
  $effect(() => {
    const currentTool = tool;
    const target = convertTarget;
    const force = convertForce;
    const hasOutline = outline != null;
    if (currentTool !== "convert" || !hasOutline) {
      convertPreview = null;
      return;
    }
    const token = (toolPreviewToken += 1);
    void (async () => {
      const result = await commands.previewComicImageConversion(uuid, target, force).result();
      if (token !== toolPreviewToken) {
        return;
      }
      convertPreview = result.status === "ok" ? result.data : null;
    })();
  });

  /** 压缩预览跟随：仅工具 / 大纲变化时刷新（不跟随滑块，避免解码风暴）。 */
  $effect(() => {
    const currentTool = tool;
    const hasOutline = outline != null;
    if (currentTool !== "compress" || !hasOutline) {
      compressPreview = null;
      return;
    }
    const token = (toolPreviewToken += 1);
    void (async () => {
      const result = await commands.previewComicImageCompression(uuid).result();
      if (token !== toolPreviewToken) {
        return;
      }
      compressPreview = result.status === "ok" ? result.data : null;
    })();
  });

  /** 尺寸调整预览跟随：目标尺寸影响跳过集合，输入防抖 400ms 后刷新。 */
  $effect(() => {
    const currentTool = tool;
    const hasOutline = outline != null;
    const options = resizeOptions;
    clearTimeout(resizePreviewDebounce);
    if (currentTool !== "resize" || !hasOutline || options == null) {
      resizePreview = null;
      return;
    }
    resizePreviewDebounce = setTimeout(() => {
      const token = (toolPreviewToken += 1);
      void (async () => {
        const result = await commands.previewComicImageResize(uuid, options).result();
        if (token !== toolPreviewToken) {
          return;
        }
        resizePreview = result.status === "ok" ? result.data : null;
      })();
    }, 400);
  });

  /** 单图预览跟随：工具 / 选中页 / 参数变化时防抖刷新真编码预览；直通参数与锁定态下清空。 */
  $effect(() => {
    const isSingle = tool === "single";
    const target = selectedPath;
    const options = singleOptions;
    const isDefault = singleIsDefault;
    const locked = toolRunning || singleApplying;
    clearTimeout(singleDebounce);
    if (!isSingle || target == null || locked || options == null || isDefault) {
      singlePreview = null;
      singlePreviewUrl = null;
      singleState = "idle";
      return;
    }
    singleState = "loading";
    singleDebounce = setTimeout(() => {
      const token = (singleToken += 1);
      void (async () => {
        const result = await commands.previewSingleComicImage(uuid, target, options).result();
        if (token !== singleToken) {
          return;
        }
        if (result.status === "error" || selectedPath !== target) {
          if (selectedPath === target) {
            singlePreview = null;
            singlePreviewUrl = null;
            singleState = "failed";
          }
          return;
        }
        singlePreview = result.data;
        singlePreviewUrl = `${convertFileSrc(result.data.path)}?v=${result.data.size_bytes}-${Date.now()}`;
        singleState = "ready";
      })();
    }, 350);
  });
  /** 确认弹窗行：源格式按列对齐，`→` 后统一为目标格式大写。 */
  function convertConfirmLines(preview: ConvertPreview, targetLabel: string): string[] {
    const fromWidth = Math.max(...preview.items.map((item) => item.from.length));
    const digitWidth = Math.max(...preview.items.map((item) => String(item.count).length));
    const unit = m.comic_edit_convert_count_unit();
    return preview.items.map((item) => {
      const count = `${String(item.count).padStart(digitWidth)} ${unit}`;
      return `${item.from.toUpperCase().padEnd(fromWidth)}  ${count}  →  ${targetLabel}`;
    });
  }

  /** 压缩确认行：参数摘要逐行展示。 */
  function compressConfirmLines(): string[] {
    const unit = m.comic_edit_convert_count_unit();
    const count = compressPreview == null ? `0 ${unit}` : `${compressPreview.total} ${unit}`;
    return [
      `${m.comic_edit_tool_scale()}  ${compressScale}%`,
      `${m.comic_edit_tool_grayscale()}  ${compressGrayscale ? m.comic_edit_tool_switch_on() : m.comic_edit_tool_switch_off()}`,
      `${m.comic_edit_tool_quality()}  ${compressQuality}`,
      `${m.comic_edit_tool_png_level()}  ${compressPngLevel}`,
      count,
    ];
  }

  /** 尺寸调整确认行：目标摘要 + 总数逐行展示。 */
  function resizeConfirmLines(): string[] {
    const unit = m.comic_edit_convert_count_unit();
    const count = resizePreview == null ? `0 ${unit}` : `${resizePreview.total} ${unit}`;
    const target =
      resizeMode === "fixed"
        ? `${resizeWidth}×${resizeHeight}`
        : resizeMode === "height"
          ? resizeHeight
          : resizeWidth;
    return [
      `${m.comic_edit_tool_resize_mode()}  ${resizeModeLabel()}`,
      `${m.comic_edit_tool_resize()}  ${target}`,
      count,
    ];
  }

  /** 尺寸调整模式文案（确认行用）。 */
  function resizeModeLabel(): string {
    switch (resizeMode) {
      case "width":
        return m.comic_edit_tool_resize_mode_width();
      case "height":
        return m.comic_edit_tool_resize_mode_height();
      case "long_edge":
        return m.comic_edit_tool_resize_mode_long_edge();
      case "fixed":
        return m.comic_edit_tool_resize_mode_fixed();
    }
  }
  /** 工具执行：单图走确认后替换，批量刷新预览后弹确认框（无可处理页直接提示）。 */
  async function handleToolRun(): Promise<void> {
    if (toolRunning || singleApplying) {
      return;
    }
    if (tool === "single") {
      if (!singleCanApply) {
        return;
      }
      singleConfirmOpen = true;
      return;
    }
    if (tool === "resize") {
      const options = resizeOptions;
      if (options == null) {
        return;
      }
      const preview = await commands.previewComicImageResize(uuid, options).result();
      if (preview.status === "error") {
        toast.error(m.comic_edit_resize_failed());
        return;
      }
      resizePreview = preview.data;
      if (resizePreview.total === 0) {
        toast.info(m.comic_edit_resize_nothing());
        return;
      }
      resizeConfirmOpen = true;
      return;
    }
    if (tool === "compress") {
      const preview = await commands.previewComicImageCompression(uuid).result();
      if (preview.status === "error") {
        toast.error(m.comic_edit_compress_failed());
        return;
      }
      compressPreview = preview.data;
      if (compressPreview.total === 0) {
        toast.info(m.comic_edit_compress_nothing());
        return;
      }
      compressConfirmOpen = true;
      return;
    }
    const preview = await commands
      .previewComicImageConversion(uuid, convertTarget, convertForce)
      .result();
    if (preview.status === "error") {
      toast.error(m.comic_edit_convert_failed());
      return;
    }
    convertPreview = preview.data;
    if (convertPreview.total === 0) {
      toast.info(m.comic_edit_convert_nothing());
      return;
    }
    convertConfirmOpen = true;
  }

  /** 工具重置：单图回全直通并清空预览，批量回各自默认值。 */
  function handleToolReset(): void {
    if (tool === "single") {
      resetSingleForm();
      return;
    }
    if (tool === "compress") {
      compressScale = 100;
      compressGrayscale = false;
      compressQuality = 85;
      compressWebpLossless = false;
      compressPngLevel = 6;
      return;
    }
    if (tool === "resize") {
      resizeMode = "width";
      resizeWidth = DEFAULT_RESIZE_WIDTH_PX;
      resizeHeight = DEFAULT_RESIZE_HEIGHT_PX;
      resizeAspectLock = "free";
      resizeFill = "contain";
      resizeBackground = "white";
      resizeHAlign = "center";
      resizeVAlign = "center";
      resizeNoUpscale = true;
      resizeQuality = 85;
      resizeWebpLossless = false;
      resizePngLevel = 6;
      return;
    }
    convertTarget = "jpeg";
    convertQuality = 85;
    convertWebpLossless = false;
    convertPngLevel = 6;
    convertForce = false;
  }

  /** 单图重置：全直通并清空预览，中栏回原图。 */
  function resetSingleForm(): void {
    const defaults = defaultSingleForm();
    singleRotate = defaults.rotate_deg;
    singleFlipH = defaults.flip_h;
    singleFlipV = defaults.flip_v;
    singleAutoTrim = defaults.auto_trim;
    singleTrimThreshold = defaults.trim_threshold;
    singleTrimMargin = defaults.trim_margin;
    singleBrightness = defaults.brightness;
    singleContrast = defaults.contrast;
    singleGrayscale = defaults.grayscale;
    singleScale = defaults.scale_percent;
    singleQuality = defaults.quality;
    singleWebpLossless = defaults.webp_lossless;
    singlePngLevel = defaults.png_level;
    singlePreview = null;
    singlePreviewUrl = null;
    singleState = "idle";
    singleView = "preview";
  }

  /** 单图确认行：参数摘要逐行展示。 */
  function singleConfirmLines(): string[] {
    const lines: string[] = [];
    if (singleRotate !== 0) {
      lines.push(`${m.comic_edit_tool_single_rotate()}  ${singleRotate}°`);
    }
    if (singleFlipH) {
      lines.push(m.comic_edit_tool_single_flip_h());
    }
    if (singleFlipV) {
      lines.push(m.comic_edit_tool_single_flip_v());
    }
    if (singleAutoTrim) {
      lines.push(
        `${m.comic_edit_tool_single_auto_trim()}  ${m.comic_edit_tool_single_trim_threshold()} ${singleTrimThreshold} · ${m.comic_edit_tool_single_trim_margin()} ${singleTrimMargin}px`,
      );
    }
    if (singleBrightness !== 0 || singleContrast !== 0) {
      lines.push(
        `${m.comic_edit_tool_single_brightness()} ${singleBrightness} · ${m.comic_edit_tool_single_contrast()} ${singleContrast}`,
      );
    }
    if (singleGrayscale) {
      lines.push(m.comic_edit_tool_grayscale());
    }
    if (singleScale !== 100) {
      lines.push(`${m.comic_edit_tool_single_scale()}  ${singleScale}%`);
    }
    if (singlePreview !== null) {
      lines.push(
        `${singlePreview.original_width_px}×${singlePreview.original_height_px} → ${singlePreview.width_px}×${singlePreview.height_px}`,
      );
    }
    return lines;
  }

  /** 单图预览重试：失败后按当前参数即时重跑一次。 */
  async function retrySinglePreview(): Promise<void> {
    const options = singleOptions;
    if (tool !== "single" || selectedPath === null || options === null || singleIsDefault) {
      return;
    }
    const target = selectedPath;
    const token = (singleToken += 1);
    singleState = "loading";
    const result = await commands.previewSingleComicImage(uuid, target, options).result();
    if (token !== singleToken || selectedPath !== target) {
      return;
    }
    if (result.status === "error") {
      singlePreview = null;
      singlePreviewUrl = null;
      singleState = "failed";
      return;
    }
    singlePreview = result.data;
    singlePreviewUrl = `${convertFileSrc(result.data.path)}?v=${result.data.size_bytes}-${Date.now()}`;
    singleState = "ready";
  }

  /** 确认单图应用：变换后原子替换源文件（不可撤销），成功后重置表单并刷新中栏。 */
  async function handleSingleApply(): Promise<void> {
    const options = singleOptions;
    if (
      options === null ||
      singleIsDefault ||
      selectedPath === null ||
      singleApplying ||
      toolRunning
    ) {
      return;
    }
    const target = selectedPath;
    singleApplying = true;
    try {
      const result = await commands.applySingleComicImage(uuid, target, options).result();
      if (result.status === "error") {
        toast.error(m.comic_edit_tool_single_failed());
        return;
      }
      outline = result.data;
      resetSingleForm();
      void loadPreview(target);
      toast.success(m.comic_edit_tool_single_success());
    } finally {
      singleApplying = false;
    }
  }

  /** 确认转换：全局进度监听在 hook 侧常驻，成功后按原全局序号跟随选中。 */
  async function handleToolConfirm(): Promise<void> {
    if (toolRunning) {
      return;
    }
    if (tool === "resize") {
      await handleResizeConfirm();
      return;
    }
    if (tool === "compress") {
      await handleCompressConfirm();
      return;
    }
    setConvertRunning(uuid, true);
    try {
      const result = await commands
        .convertComicImages(uuid, convertTarget, {
          quality: convertQuality,
          webp_lossless: convertWebpLossless,
          png_level: convertPngLevel,
          force: convertForce,
        })
        .result();
      if (result.status === "error") {
        if (result.error.kind === "Cancelled") {
          toast.info(m.comic_edit_tool_cancelled());
          return;
        }
        toast.error(m.comic_edit_convert_failed());
        return;
      }
      const prevIndex = location?.index ?? 1;
      outline = result.data;
      working = toWorking(result.data.volumes);
      saved = toWorking(result.data.volumes);
      const flat = flattenPages(working);
      selectedPath = flat[Math.min(prevIndex - 1, flat.length - 1)] ?? null;
      if (selectedPath != null) {
        void loadPreview(selectedPath);
      } else {
        previewUrl = null;
        previewSize = null;
        previewDims = null;
        previewLoading = false;
        previewFailed = false;
      }
      toast.success(m.comic_edit_convert_success());
    } finally {
      setConvertRunning(uuid, false);
      void refreshCompressStatus(uuid);
    }
  }

  /** 确认压缩：原地重编码，路径不变故选中无需跟随；成功提示处理/跳过数。 */
  async function handleCompressConfirm(): Promise<void> {
    setCompressRunning(uuid, true);
    try {
      const result = await commands
        .compressComicImages(uuid, {
          scale_percent: compressScale,
          grayscale: compressGrayscale,
          quality: compressQuality,
          webp_lossless: compressWebpLossless,
          png_level: compressPngLevel,
        })
        .result();
      if (result.status === "error") {
        if (result.error.kind === "Cancelled") {
          toast.info(m.comic_edit_tool_cancelled());
          return;
        }
        toast.error(m.comic_edit_compress_failed());
        return;
      }
      outline = result.data.outline;
      working = toWorking(result.data.outline.volumes);
      saved = toWorking(result.data.outline.volumes);
      if (selectedPath != null) {
        void loadPreview(selectedPath);
      }
      toast.success(
        m.comic_edit_compress_success_detail({
          processed: result.data.processed,
          skipped: result.data.skipped,
        }),
      );
    } finally {
      setCompressRunning(uuid, false);
      void refreshCompressStatus(uuid);
    }
  }

  /** 确认尺寸调整：原地重编码，路径与格式不变故选中无需跟随；成功提示处理/跳过数。 */
  async function handleResizeConfirm(): Promise<void> {
    const options = resizeOptions;
    if (options == null) {
      return;
    }
    setResizeRunning(uuid, true);
    try {
      const result = await commands.resizeComicImages(uuid, options).result();
      if (result.status === "error") {
        if (result.error.kind === "Cancelled") {
          toast.info(m.comic_edit_tool_cancelled());
          return;
        }
        toast.error(m.comic_edit_resize_failed());
        return;
      }
      outline = result.data.outline;
      working = toWorking(result.data.outline.volumes);
      saved = toWorking(result.data.outline.volumes);
      if (selectedPath != null) {
        void loadPreview(selectedPath);
      }
      toast.success(
        m.comic_edit_resize_success_detail({
          processed: result.data.processed,
          skipped: result.data.skipped,
        }),
      );
    } finally {
      setResizeRunning(uuid, false);
      void refreshResizeStatus(uuid);
      void refreshCompressStatus(uuid);
    }
  }

  /** 取消图片作业：转换/压缩互斥，任一在跑即置旗；在跑的任务随后以取消结算并轻提示。 */
  async function handleToolCancel(): Promise<void> {
    if (!toolRunning) {
      return;
    }
    const result = await commands.cancelComicImages(uuid).result();
    if (result.status === "error") {
      toast.error(m.comic_edit_tool_cancel_failed());
      return;
    }
    if (!result.data) {
      toast.info(m.comic_edit_tool_cancel_none());
    }
  }

  /** 清除备份确认框：纯 UI 态，执行走确认回调。 */
  let clearBackupConfirmOpen = $state(false);

  /** 撤销压缩：备份原图拷回原位，一次性有效；成功后刷新预览戳（同路径缓存）。 */
  async function handleCompressUndo(): Promise<void> {
    if (toolRunning) {
      return;
    }
    setCompressRunning(uuid, true);
    try {
      const result = await commands.restoreComicCompressBackup(uuid).result();
      if (result.status === "error") {
        toast.error(m.comic_edit_tool_undo_failed());
        return;
      }
      outline = result.data;
      working = toWorking(result.data.volumes);
      saved = toWorking(result.data.volumes);
      if (selectedPath != null) {
        void loadPreview(selectedPath);
      }
      toast.success(m.comic_edit_tool_undo_success());
    } finally {
      setCompressRunning(uuid, false);
      void refreshCompressStatus(uuid);
      void refreshResizeStatus(uuid);
    }
  }

  /** 确认清除备份：只删备份目录，当前图片不动；备份一次性，清除后撤销入口消失。 */
  async function handleClearBackupConfirm(): Promise<void> {
    const result = await commands.clearComicCompressBackup(uuid).result();
    if (result.status === "error") {
      toast.error(m.comic_edit_tool_clear_backup_failed());
      return;
    }
    if (!result.data) {
      toast.info(m.comic_edit_tool_clear_backup_none());
    } else {
      toast.success(m.comic_edit_tool_clear_backup_success());
    }
    void refreshCompressStatus(uuid);
    void refreshResizeStatus(uuid);
  }

  /** 键盘话重排：选中页所在话整体上移 / 下移一位；转换中锁定。 */
  function keyboardMoveVolume(delta: 1 | -1): void {
    if (imageLocked || selectedPath == null) {
      return;
    }
    const ref = findPage(working, selectedPath);
    if (ref == null) {
      return;
    }
    const target = ref.volume + delta;
    if (target < 0 || target >= working.length) {
      return;
    }
    working = moveVolume(working, ref.volume, {
      index: target,
      edge: delta > 0 ? "after" : "before",
    });
  }

  /** 改名提交：标题已由树组件校验，直接落本地工作话。 */
  function handleRename(volume: number, title: string): void {
    if (imageLocked) {
      return;
    }
    if (working[volume] == null) {
      return;
    }
    working[volume] = { ...working[volume], title };
  }

  /** 删除空话：请求时弹确认框，确认后移除。 */
  let deleteConfirmOpen = $state(false);
  let pendingDelete = $state<number | null>(null);
  const deleteTargetTitle = $derived(
    pendingDelete == null ? null : (working[pendingDelete]?.title ?? null),
  );

  function handleDeleteRequest(volume: number): void {
    if (imageLocked) {
      return;
    }
    pendingDelete = volume;
    deleteConfirmOpen = true;
  }

  function handleDeleteConfirm(): void {
    if (imageLocked) {
      return;
    }
    const target = pendingDelete == null ? null : working[pendingDelete];
    pendingDelete = null;
    if (target == null || target.pages.length > 0) {
      return;
    }
    working = working.filter((entry) => entry !== target);
    if (selectedPath == null || findPage(working, selectedPath) == null) {
      selectFirstPage();
    }
  }

  /** 新建分话：内联输入态，提交时做非空 / 长度 / 重名校验。 */
  let creatingVolume = $state(false);
  let newTitle = $state("");
  /** 新建输入框：挂载后聚焦（不用 autofocus 属性，a11y 规则不推荐）。 */
  let newVolumeInput: HTMLInputElement | null = $state(null);
  $effect(() => {
    newVolumeInput?.focus();
  });

  function titleErrorOf(title: string): VolumeTitleError | null {
    return validateVolumeTitle(
      title,
      working.map((entry) => entry.title ?? ""),
    );
  }

  function handleCreateVolume(): void {
    if (imageLocked) {
      return;
    }
    const error = titleErrorOf(newTitle);
    if (error === "empty") {
      toast.error(m.comic_edit_episode_title_empty());
      return;
    }
    if (error === "duplicate") {
      toast.error(m.comic_edit_episode_title_duplicate());
      return;
    }
    if (error === "too_long") {
      toast.error(m.comic_edit_episode_title_too_long());
      return;
    }
    working = [...working, createVolume(newTitle.trim())];
    newTitle = "";
    creatingVolume = false;
  }

  /** 保存：全量分话结构覆写落盘，成功后快照同步、失败保留本地可再保存。 */
  async function handleSave(): Promise<void> {
    if (imageLocked) {
      return;
    }
    if (!dirty || saving) {
      return;
    }
    saving = true;
    const result = await commands
      .updateComicVolumes(
        uuid,
        working.map((volume) => ({ title: volume.title, pages: [...volume.pages] })),
      )
      .result();
    saving = false;
    if (result.status === "error") {
      toast.error(m.comic_edit_save_failed());
      return;
    }
    outline = result.data;
    working = toWorking(result.data.volumes);
    saved = toWorking(result.data.volumes);
    toast.success(m.comic_edit_save_success());
  }

  /** 打开解析目录：调系统文件管理器定位项目根（`content.json` 所在），失败 toast。 */
  async function openContentDir(): Promise<void> {
    if (openingContentDir) {
      return;
    }
    openingContentDir = true;
    const result = await commands.openProjectDir(uuid).result();
    openingContentDir = false;
    if (result.status === "error") {
      toast.error(m.comic_files_open_dir_failed());
    }
  }

  /** 删除页记录：只从工作话移除顺序记录，sources/images 中的图片文件保留； 未保存前可在左栏放弃更改找回。 */
  function handleDeletePage(): void {
    if (imageLocked) {
      return;
    }
    if (selectedPath == null) {
      return;
    }
    const ref = findPage(working, selectedPath);
    if (ref == null) {
      return;
    }
    const flat = flattenPages(working);
    const index = flat.indexOf(selectedPath);
    working = working.map((volume, i) =>
      i === ref.volume
        ? { ...volume, pages: volume.pages.filter((_, j) => j !== ref.index) }
        : volume,
    );
    const rest = flattenPages(working);
    const next = rest[Math.min(index, rest.length - 1)] ?? null;
    selectedPath = next;
    if (next == null) {
      previewUrl = null;
      previewSize = null;
      previewDims = null;
      previewLoading = false;
      previewFailed = false;
      return;
    }
    void loadPreview(next);
  }

  /** 放弃更改：回到已落盘快照，选中页丢失时回首个有效页。 */
  function handleDiscard(): void {
    working = cloneVolumes(saved);
    if (selectedPath == null || findPage(working, selectedPath) == null) {
      selectFirstPage();
    }
  }

  /** 上一页 / 下一页：按全局顺序导航。 */
  function stepPage(delta: 1 | -1): void {
    if (selectedPath == null) {
      return;
    }
    const flat = flattenPages(working);
    const index = flat.indexOf(selectedPath);
    const next = flat[index + delta];
    if (next != null) {
      handleSelect(next);
    }
  }

  /** 键盘重排：在话内换位，到边界自动跨入相邻话首 / 末；转换中锁定。 */
  function keyboardMove(delta: 1 | -1): void {
    if (imageLocked || selectedPath == null) {
      return;
    }
    const ref = findPage(working, selectedPath);
    if (ref == null) {
      return;
    }
    const current = working[ref.volume];
    const neighbor = ref.index + delta;
    if (neighbor >= 0 && neighbor < current.pages.length) {
      handleMove(ref, {
        volume: ref.volume,
        index: neighbor,
        edge: delta > 0 ? "after" : "before",
      });
      return;
    }
    const sibling = ref.volume + delta;
    if (sibling < 0 || sibling >= working.length) {
      return;
    }
    handleMove(
      ref,
      delta > 0
        ? { volume: sibling, index: 0, edge: "before" }
        : { volume: sibling, edge: "append" },
    );
  }

  /** 全局快捷键：保存 / 切页（上下左右）/ 键盘重排，门禁由各自回调把守。 输入框聚焦时只响应保存，其余方向键留给输入框自身。 */
  function handleGlobalKeydown(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void handleSave();
      return;
    }
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }
    if (event.altKey && event.key === "ArrowUp") {
      event.preventDefault();
      if (event.shiftKey) {
        keyboardMoveVolume(-1);
      } else {
        keyboardMove(-1);
      }
      return;
    }
    if (event.altKey && event.key === "ArrowDown") {
      event.preventDefault();
      if (event.shiftKey) {
        keyboardMoveVolume(1);
      } else {
        keyboardMove(1);
      }
      return;
    }
    const step = resolveArrowStep(event.key, {
      alt: event.altKey,
      ctrl: event.ctrlKey,
      meta: event.metaKey,
    });
    if (step !== 0) {
      stepPage(step);
    }
  }

  /** 预览区滚轮翻页：主导轴累积超阈值翻页；缩放手势放过。 */
  $effect(() => {
    const box = previewBox;
    if (box == null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) {
        return;
      }
      const delta = normalizeWheelDelta(
        Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX,
        event.deltaMode,
      );
      const { flip, acc } = accumulateWheelFlip(wheelAcc, delta);
      wheelAcc = acc;
      if (flip !== 0) {
        event.preventDefault();
        stepPage(flip);
      }
    };
    box.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      box.removeEventListener("wheel", onWheel);
      wheelAcc = 0;
    };
  });

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
    // 同路径原地重编码（压缩）后 URL 字符串不变，WebView 会命中缓存：
    // 拼版本戳强制重取，`asset` 协议忽略 query 只按路径取文件。
    previewUrl = `${convertFileSrc(image.data.path)}?v=${image.data.size_bytes}-${Date.now()}`;
    previewSize = image.data.size_bytes;
  }

  /** 预览失败重试：选中仍在时重取当前页。 */
  function retryPreview(): void {
    if (selectedPath != null) {
      void loadPreview(selectedPath);
    }
  }

  onMount(() => {
    void loadOutline();
    ensureConvertListener();
    ensureCompressListener();
    ensureResizeListener();
    // 切页返回：有在跑的长时间作业则直接打开对应工具，仅恢复一次，不跟随。
    void (async () => {
      await Promise.all([
        refreshConvertStatus(uuid),
        refreshCompressStatus(uuid),
        refreshResizeStatus(uuid),
      ]);
      const running = toolWithRunningJob({
        convert: convertJobOf(uuid).running,
        compress: compressJobOf(uuid).running,
        resize: resizeJobOf(uuid).running,
      });
      if (running != null) {
        tool = running;
      }
    })();
  });
</script>

<svelte:window onkeydown={handleGlobalKeydown} />
<h1 class="sr-only">{m.project_step_edit()}</h1>
{#if loading}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-muted-foreground" aria-busy="true">{m.comic_edit_loading()}</p>
    </div>
  </div>
{:else if loadFailed}
  <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col p-6">
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
      <p class="text-sm text-muted-foreground">{m.comic_edit_load_failed()}</p>
      <Button variant="outline" class="w-full max-w-md" onclick={() => void loadOutline()}>
        {m.files_retry()}
      </Button>
    </div>
  </div>
{:else if outline === null}
  <ComicStepGuide {uuid} />
{:else}
  <div class="flex h-full min-h-0 w-full min-w-0">
    <Resizable.PaneGroup direction="horizontal">
      <Resizable.Pane defaultSize={22} minSize={15} maxSize={35} class="min-w-0">
        <div class="flex h-full min-h-0 flex-col">
          <div class="flex shrink-0 items-center gap-1 px-3 pt-3">
            <p class="min-w-0 flex-1 text-xs text-muted-foreground">
              {dirty ? m.comic_edit_dirty_hint() : m.comic_edit_drag_hint()}
            </p>
            <Button
              variant="ghost"
              size="icon"
              class="size-6 shrink-0"
              aria-label={m.comic_edit_locate_page()}
              title={m.comic_edit_locate_page()}
              disabled={selectedPath == null}
              onclick={() => {
                locateNonce += 1;
              }}
            >
              <LocateFixedIcon class="size-4" />
            </Button>
          </div>
          <div class="min-h-0 flex-1 [scrollbar-gutter:stable] overflow-y-auto">
            <ComicPageTree
              volumes={working}
              {selectedPath}
              sizes={pageSizes}
              locateRequest={locateNonce}
              locked={imageLocked}
              onSelect={handleSelect}
              onDropOrder={handleDropOrder}
              onRename={handleRename}
              onDeleteRequest={handleDeleteRequest}
            />
          </div>
          <div class="flex shrink-0 flex-col gap-2 border-t border-border px-3 py-2">
            {#if creatingVolume}
              <input
                bind:this={newVolumeInput}
                bind:value={newTitle}
                disabled={imageLocked}
                aria-label={m.comic_edit_episode_new()}
                placeholder={m.comic_edit_episode_name_placeholder()}
                onkeydown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    handleCreateVolume();
                  } else if (event.key === "Escape") {
                    event.preventDefault();
                    creatingVolume = false;
                    newTitle = "";
                  }
                }}
                class="w-full rounded-md border border-input bg-background px-2 py-1 text-sm outline-none focus-visible:border-ring"
              />
              <div class="flex gap-2">
                <Button
                  class="flex-1"
                  size="sm"
                  disabled={saving || imageLocked}
                  onclick={handleCreateVolume}
                >
                  {m.comic_edit_episode_create()}
                </Button>
                <Button
                  variant="outline"
                  class="flex-1"
                  size="sm"
                  onclick={() => {
                    creatingVolume = false;
                    newTitle = "";
                  }}
                >
                  {m.projects_create_cancel()}
                </Button>
              </div>
            {:else}
              <Button
                variant="outline"
                class="w-full"
                disabled={saving || imageLocked}
                onclick={() => {
                  creatingVolume = true;
                  newTitle = "";
                }}
              >
                {m.comic_edit_episode_new()}
              </Button>
            {/if}
          </div>
          {#if dirty}
            <div class="flex shrink-0 flex-col gap-2 border-t border-border px-3 py-2">
              <Button
                class="w-full"
                disabled={saving || imageLocked}
                onclick={() => void handleSave()}
              >
                {saving ? m.comic_edit_saving() : m.comic_edit_save()}
              </Button>
              <Button
                variant="outline"
                class="w-full"
                disabled={saving || imageLocked}
                onclick={() => (confirmOpen = true)}
              >
                {m.comic_edit_discard()}
              </Button>
            </div>
          {/if}
        </div>
      </Resizable.Pane>
      <Resizable.Handle withHandle aria-label={m.edit_resize_left_label()} />
      <Resizable.Pane defaultSize={56} minSize={30} class="min-w-0">
        <div class="flex h-full min-h-0 flex-col">
          <div class="flex shrink-0 items-center gap-3 border-b border-border px-4 py-2">
            {#if location != null && selectedPath != null}
              <span class="shrink-0 text-sm font-medium text-foreground">
                {m.comic_edit_episode_number({ index: location.episode })}
              </span>
              <span class="min-w-0 flex-1 truncate text-sm text-muted-foreground">
                {location.serial} - {selectedPath}
              </span>
              <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                {displayedDims == null
                  ? m.comic_files_preview_loading()
                  : `${displayedDims.width} × ${displayedDims.height}`}
              </span>
              {#if displayedSize != null}
                <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
                  {formatBytes(displayedSize)}
                </span>
              {/if}
            {:else}
              <span class="min-w-0 flex-1" aria-hidden="true"></span>
            {/if}
            <Button
              variant="ghost"
              size="icon"
              aria-label={m.comic_edit_open_content_dir()}
              title={m.comic_edit_open_content_dir()}
              disabled={openingContentDir}
              onclick={() => void openContentDir()}
            >
              <FolderOpenIcon class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label={m.comic_edit_delete_page()}
              title={m.comic_edit_delete_page_hint()}
              disabled={saving || location == null || imageLocked}
              onclick={handleDeletePage}
            >
              <TrashIcon class="size-4" />
            </Button>
          </div>
          {#if tool === "single" && selectedPath != null && singleState !== "idle"}
            <div class="flex shrink-0 items-center gap-2 border-b border-border px-4 py-1.5">
              <Button
                variant={singleView === "original" ? "secondary" : "ghost"}
                size="sm"
                onclick={() => (singleView = "original")}
              >
                {m.comic_edit_tool_single_view_original()}
              </Button>
              <Button
                variant={singleView === "preview" ? "secondary" : "ghost"}
                size="sm"
                disabled={singlePreview == null}
                onclick={() => (singleView = "preview")}
              >
                {m.comic_edit_tool_single_view_preview()}
              </Button>
              {#if singleState === "loading"}
                <span class="text-xs text-muted-foreground" aria-busy="true">
                  {m.comic_edit_tool_single_preview_loading()}
                </span>
              {:else if singleState === "failed"}
                <Button variant="ghost" size="sm" onclick={() => void retrySinglePreview()}>
                  {m.files_retry()}
                </Button>
              {:else if singlePreview != null}
                <span class="min-w-0 flex-1 truncate text-xs text-muted-foreground tabular-nums">
                  {singlePreview.original_width_px} × {singlePreview.original_height_px} → {singlePreview.width_px}
                  × {singlePreview.height_px}
                </span>
              {/if}
            </div>
          {/if}
          <div
            bind:this={previewBox}
            class="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-4"
          >
            {#if selectedPath == null}
              <div class="flex flex-col items-center gap-2 text-center">
                <ImageIcon class="size-10 text-muted-foreground" />
                <p class="text-sm text-muted-foreground">{m.comic_edit_no_selection()}</p>
              </div>
            {:else if previewLoading}
              <p class="text-sm text-muted-foreground" aria-busy="true">
                {m.comic_files_preview_loading()}
              </p>
            {:else if previewFailed || previewUrl == null}
              <div class="flex flex-col items-center gap-2 text-center">
                <p class="text-sm text-muted-foreground">{m.comic_files_preview_failed()}</p>
                <Button variant="outline" size="sm" onclick={retryPreview}>
                  {m.files_retry()}
                </Button>
              </div>
            {:else}
              <img
                src={viewingPreview && singlePreviewUrl != null ? singlePreviewUrl : previewUrl}
                alt={pageFileName(selectedPath)}
                onload={(event) => {
                  // 看预览时自然尺寸即预览尺寸，原图尺寸保持不动。
                  if (viewingPreview) {
                    return;
                  }
                  const img = event.currentTarget as HTMLImageElement | null;
                  if (img != null) {
                    previewDims = { width: img.naturalWidth, height: img.naturalHeight };
                  }
                }}
                class="max-h-full max-w-full object-contain"
              />
            {/if}
          </div>
          <div class="flex shrink-0 items-center gap-2 border-t border-border px-4 py-3">
            <Button
              variant="outline"
              size="sm"
              disabled={location == null || location.index <= 1}
              onclick={() => stepPage(-1)}
            >
              {m.comic_edit_prev()}
            </Button>
            <span class="min-w-0 flex-1 text-center text-sm text-muted-foreground tabular-nums">
              {#if location != null}
                {location.index} / {location.total}
              {/if}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={location == null || location.index >= location.total}
              onclick={() => stepPage(1)}
            >
              {m.comic_edit_next()}
            </Button>
          </div>
        </div>
      </Resizable.Pane>
      <Resizable.Handle withHandle aria-label={m.edit_resize_right_label()} />
      <Resizable.Pane defaultSize={22} minSize={15} maxSize={35} class="min-w-0">
        <div class="flex h-full min-h-0 flex-col overflow-hidden">
          <div class="min-h-0 flex-1">
            <ComicToolPanel
              bind:tool
              bind:target={convertTarget}
              bind:quality={convertQuality}
              bind:webpLossless={convertWebpLossless}
              bind:pngLevel={convertPngLevel}
              bind:forceConvert={convertForce}
              bind:compressScale
              bind:compressGrayscale
              bind:compressQuality
              bind:compressWebpLossless
              bind:compressPngLevel
              bind:resizeMode
              bind:resizeWidth
              bind:resizeHeight
              bind:resizeAspectLock
              bind:resizeFill
              bind:resizeBackground
              bind:resizeHAlign
              bind:resizeVAlign
              bind:resizeNoUpscale
              bind:resizeQuality
              bind:resizeWebpLossless
              bind:resizePngLevel
              bind:singleRotate
              bind:singleFlipH
              bind:singleFlipV
              bind:singleAutoTrim
              bind:singleTrimThreshold
              bind:singleTrimMargin
              bind:singleBrightness
              bind:singleContrast
              bind:singleGrayscale
              bind:singleScale
              bind:singleQuality
              bind:singleWebpLossless
              bind:singlePngLevel
              running={toolRunning}
              canRun={panelCanRun}
              hasOutcome={panelHasOutcome}
              progress={toolProgress}
              backupExists={compressJob.backup}
              singleHasSelection={selectedPath !== null}
              singlePreview={singleState}
              {singleApplying}
              singleLocked={toolRunning}
              runLabel={tool === "single"
                ? singleApplying
                  ? m.comic_edit_tool_running()
                  : m.comic_edit_tool_single_apply()
                : null}
              onRun={() => void handleToolRun()}
              onClear={handleToolReset}
              onCancel={() => void handleToolCancel()}
              onUndo={() => void handleCompressUndo()}
              onRequestClearBackup={() => (clearBackupConfirmOpen = true)}
            />
          </div>
        </div>
      </Resizable.Pane>
    </Resizable.PaneGroup>
  </div>
{/if}
<ConfirmDialog
  bind:open={confirmOpen}
  title={m.comic_edit_discard_title()}
  description={m.comic_edit_discard_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.edit_unsaved_discard()}
  onConfirm={handleDiscard}
/>
<ConfirmDialog
  bind:open={deleteConfirmOpen}
  title={m.comic_edit_episode_delete_title()}
  description={m.comic_edit_episode_delete_description({
    title: deleteTargetTitle ?? m.comic_untitled_episode(),
  })}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_episode_delete_ok()}
  onConfirm={handleDeleteConfirm}
/>
<ConfirmDialog
  bind:open={convertConfirmOpen}
  title={m.comic_edit_convert_confirm_title()}
  description={m.comic_edit_convert_confirm_description({
    target: convertTarget.toUpperCase(),
  })}
  details={convertPreview == null
    ? []
    : convertConfirmLines(convertPreview, convertTarget.toUpperCase())}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_tool_run()}
  onConfirm={() => void handleToolConfirm()}
/>
<ConfirmDialog
  bind:open={compressConfirmOpen}
  title={m.comic_edit_compress_confirm_title()}
  description={m.comic_edit_compress_confirm_description({
    total: compressPreview?.total ?? 0,
  })}
  details={compressConfirmLines()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_tool_run()}
  onConfirm={() => void handleToolConfirm()}
/>
<ConfirmDialog
  bind:open={resizeConfirmOpen}
  title={m.comic_edit_resize_confirm_title()}
  description={m.comic_edit_resize_confirm_description({
    total: resizePreview?.total ?? 0,
  })}
  details={resizeConfirmLines()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_tool_run()}
  onConfirm={() => void handleToolConfirm()}
/>
<ConfirmDialog
  bind:open={singleConfirmOpen}
  title={m.comic_edit_tool_single_confirm_title()}
  description={m.comic_edit_tool_single_confirm_description()}
  details={singleConfirmLines()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_tool_single_apply()}
  onConfirm={() => void handleSingleApply()}
/>
<ConfirmDialog
  bind:open={clearBackupConfirmOpen}
  title={m.comic_edit_tool_clear_backup_title()}
  description={m.comic_edit_tool_clear_backup_description()}
  cancelLabel={m.projects_create_cancel()}
  confirmLabel={m.comic_edit_tool_clear_backup()}
  onConfirm={() => void handleClearBackupConfirm()}
/>
