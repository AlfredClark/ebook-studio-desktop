<script lang="ts">
  // 漫画右栏图片工具（哑组件）：顶部工具选择固定，中间参数区按工具切换，
  // 底部执行 + 重置贴底；参数与开关状态由父组件持有（$bindable 透传）。
  import { Button } from "#components/shadcn-svelte/button";
  import { Input } from "#components/shadcn-svelte/input";
  import { Progress } from "#components/shadcn-svelte/progress";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { Slider } from "#components/shadcn-svelte/slider";
  import { Switch } from "#components/shadcn-svelte/switch";
  import type {
    ComicPdfHorizontalAlignment,
    ComicPdfVerticalAlignment,
    ImageTargetFormat,
    ResizeBackground,
    ResizeFill,
    ResizeMode,
  } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";
  import {
    DEFAULT_RESIZE_HEIGHT_PX,
    DEFAULT_RESIZE_WIDTH_PX,
    RESIZE_ASPECT_LOCKS,
    isNoUpscaleEnabled,
    linkedResizeHeight,
    linkedResizeWidth,
    type ResizeAspectLock,
  } from "./comic-resize";

  /** 工具类型：格式转换 / 图片压缩 / 尺寸调整。 */
  export type ComicTool = "convert" | "compress" | "resize";

  let {
    tool = $bindable("convert"),
    target = $bindable("jpeg"),
    quality = $bindable(85),
    webpLossless = $bindable(false),
    pngLevel = $bindable(6),
    forceConvert = $bindable(false),
    compressScale = $bindable(100),
    compressGrayscale = $bindable(false),
    compressQuality = $bindable(85),
    compressWebpLossless = $bindable(false),
    compressPngLevel = $bindable(6),
    resizeMode = $bindable("width"),
    resizeWidth = $bindable(DEFAULT_RESIZE_WIDTH_PX),
    resizeHeight = $bindable(DEFAULT_RESIZE_HEIGHT_PX),
    resizeAspectLock = $bindable("free"),
    resizeFill = $bindable("contain"),
    resizeBackground = $bindable("white"),
    resizeHAlign = $bindable("center"),
    resizeVAlign = $bindable("center"),
    resizeNoUpscale = $bindable(true),
    resizeQuality = $bindable(85),
    resizeWebpLossless = $bindable(false),
    resizePngLevel = $bindable(6),
    running = false,
    canRun = false,
    hasOutcome = false,
    progress = null,
    backupExists = false,
    onRun,
    onClear,
    onCancel,
    onUndo,
    onRequestClearBackup,
  }: {
    tool: ComicTool;
    target: ImageTargetFormat;
    quality: number;
    webpLossless: boolean;
    pngLevel: number;
    forceConvert: boolean;
    compressScale: number;
    compressGrayscale: boolean;
    compressQuality: number;
    compressWebpLossless: boolean;
    compressPngLevel: number;
    resizeMode: ResizeMode;
    resizeWidth: string;
    resizeHeight: string;
    resizeAspectLock: ResizeAspectLock;
    resizeFill: ResizeFill;
    resizeBackground: ResizeBackground;
    resizeHAlign: ComicPdfHorizontalAlignment;
    resizeVAlign: ComicPdfVerticalAlignment;
    resizeNoUpscale: boolean;
    resizeQuality: number;
    resizeWebpLossless: boolean;
    resizePngLevel: number;
    running: boolean;
    canRun: boolean;
    hasOutcome: boolean;
    progress: number | null;
    backupExists: boolean;
    onRun: () => void;
    onClear: () => void;
    onCancel: () => void;
    onUndo: () => void;
    onRequestClearBackup: () => void;
  } = $props();

  /** 工具下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const toolItems = [
    { value: "convert", label: m.comic_edit_tool_convert() },
    { value: "compress", label: m.comic_edit_tool_compress() },
    { value: "resize", label: m.comic_edit_tool_resize() },
  ];

  /** 尺寸调整模式候选项：固定顺序。 */
  const resizeModeItems = [
    { value: "width" as const, label: m.comic_edit_tool_resize_mode_width() },
    { value: "height" as const, label: m.comic_edit_tool_resize_mode_height() },
    { value: "long_edge" as const, label: m.comic_edit_tool_resize_mode_long_edge() },
    { value: "fixed" as const, label: m.comic_edit_tool_resize_mode_fixed() },
  ];

  /** 固定画布填充候选项：与 PDF 导出同语义，标签复用。 */
  const resizeFillItems = [
    { value: "contain" as const, label: m.comic_export_pdf_fill_contain() },
    { value: "cover" as const, label: m.comic_export_pdf_fill_cover() },
    { value: "stretch" as const, label: m.comic_export_pdf_fill_stretch() },
  ];

  /** 补底颜色候选项：固定顺序。 */
  const resizeBackgroundItems = [
    { value: "white" as const, label: m.comic_edit_tool_resize_background_white() },
    { value: "black" as const, label: m.comic_edit_tool_resize_background_black() },
    {
      value: "transparent" as const,
      label: m.comic_edit_tool_resize_background_transparent(),
    },
  ];

  /** 对齐候选项：标签复用 PDF 导出。 */
  const resizeHAlignItems = [
    { value: "left" as const, label: m.comic_export_pdf_align_left() },
    { value: "center" as const, label: m.comic_export_pdf_align_center() },
    { value: "right" as const, label: m.comic_export_pdf_align_right() },
  ];
  const resizeVAlignItems = [
    { value: "top" as const, label: m.comic_export_pdf_align_top() },
    { value: "center" as const, label: m.comic_export_pdf_align_center() },
    { value: "bottom" as const, label: m.comic_export_pdf_align_bottom() },
  ];

  /** 仅缩小开关可用：铺满/拉伸按定义必重采样（纯函数收敛，见 comic-resize）。 */
  const noUpscaleEnabled = $derived(isNoUpscaleEnabled(resizeMode, resizeFill));

  /** 比例锁定候选项：`free` 显示文案，其余直接显示宽高比（标签复用 PDF 导出）。 */
  const resizeAspectLockItems = $derived(
    RESIZE_ASPECT_LOCKS.map((value) => ({
      value,
      label: value === "free" ? m.comic_export_pdf_aspect_free() : value,
    })),
  );

  /** 目标格式下拉候选项：格式名通用，无需国际化。 */
  const formatItems: { value: ImageTargetFormat; label: string }[] = [
    { value: "jpeg", label: "JPEG" },
    { value: "png", label: "PNG" },
    { value: "webp", label: "WebP" },
  ];

  /** 工具切换：下拉给字符串，经候选项收窄后提交，无类型断言。 */
  function handleToolChange(value: string): void {
    if (value === "convert" || value === "compress" || value === "resize") {
      tool = value;
    }
  }

  /** 尺寸调整下拉值收窄（模式 / 填充 / 背景 / 对齐各自收窄）。 */
  function handleResizeModeChange(value: string): void {
    if (value === "width" || value === "height" || value === "long_edge" || value === "fixed") {
      resizeMode = value;
    }
  }

  /** 比例锁定下拉值收窄；切到具体比例时按宽重算高，宽高皆非法回默认值。 */
  function handleResizeAspectLockChange(value: string): void {
    if (!(RESIZE_ASPECT_LOCKS as string[]).includes(value)) {
      return;
    }
    resizeAspectLock = value as ResizeAspectLock;
    if (resizeAspectLock === "free") {
      return;
    }
    const byWidth = linkedResizeHeight(resizeWidth, resizeAspectLock);
    if (byWidth !== "") {
      resizeHeight = byWidth;
      return;
    }
    const byHeight = linkedResizeWidth(resizeHeight, resizeAspectLock);
    if (byHeight !== "") {
      resizeWidth = byHeight;
      return;
    }
    resizeWidth = DEFAULT_RESIZE_WIDTH_PX;
    resizeHeight = DEFAULT_RESIZE_HEIGHT_PX;
  }

  /** 尺寸宽度输入：固定模式锁定比例时按宽联动高度，非法输入清空高度并禁用执行。 */
  function handleResizeWidthInput(value: string): void {
    resizeWidth = value;
    if (resizeMode === "fixed" && resizeAspectLock !== "free") {
      resizeHeight = linkedResizeHeight(value, resizeAspectLock);
    }
  }

  /** 尺寸高度输入：固定模式锁定比例时按高联动宽度，非法输入清空宽度并禁用执行。 */
  function handleResizeHeightInput(value: string): void {
    resizeHeight = value;
    if (resizeMode === "fixed" && resizeAspectLock !== "free") {
      resizeWidth = linkedResizeWidth(value, resizeAspectLock);
    }
  }

  /** 固定画布填充切换：同上收窄。 */
  function handleResizeFillChange(value: string): void {
    if (value === "contain" || value === "cover" || value === "stretch") {
      resizeFill = value;
    }
  }

  /** 补底颜色切换：同上收窄。 */
  function handleResizeBackgroundChange(value: string): void {
    if (value === "white" || value === "black" || value === "transparent") {
      resizeBackground = value;
    }
  }

  /** 对齐切换：水平 / 垂直各自收窄。 */
  function handleResizeHAlignChange(value: string): void {
    if (value === "left" || value === "center" || value === "right") {
      resizeHAlign = value;
    }
  }

  /** 垂直对齐切换：同上收窄。 */
  function handleResizeVAlignChange(value: string): void {
    if (value === "top" || value === "center" || value === "bottom") {
      resizeVAlign = value;
    }
  }

  /** 目标格式切换：同上收窄。 */
  function handleFormatChange(value: string): void {
    if (value === "jpeg" || value === "png" || value === "webp") {
      target = value;
    }
  }
</script>

<section
  aria-label={m.comic_edit_tool_title()}
  class="flex h-full min-h-0 flex-col gap-4 px-6 pt-4 pb-3"
>
  <h2 class="text-sm font-medium">{m.comic_edit_tool_title()}</h2>
  <Select type="single" value={tool} items={toolItems} onValueChange={handleToolChange}>
    <SelectTrigger class="w-full shrink-0" aria-label={m.comic_edit_tool_kind_label()}>
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      {#each toolItems as item (item.value)}
        <SelectItem value={item.value}>{item.label}</SelectItem>
      {/each}
    </SelectContent>
  </Select>
  <div class="shrink-0 border-t"></div>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-x-hidden overflow-y-auto">
    {#if tool === "convert"}
      <div class="flex flex-col gap-1.5">
        <span id="comic-tool-format-label" class="text-sm font-medium">
          {m.comic_edit_tool_target()}
        </span>
        <Select type="single" value={target} items={formatItems} onValueChange={handleFormatChange}>
          <SelectTrigger class="w-full shrink-0" aria-labelledby="comic-tool-format-label">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each formatItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>
      {#if target === "jpeg" || target === "webp"}
        <div class="flex flex-col gap-1.5">
          <div class="flex items-baseline justify-between">
            <span id="comic-tool-quality-label" class="text-sm font-medium">
              {m.comic_edit_tool_quality()}
            </span>
            <span class="text-sm text-muted-foreground tabular-nums">{quality}</span>
          </div>
          <Slider
            type="single"
            value={quality}
            min={1}
            max={100}
            step={1}
            onValueChange={(value) => (quality = value)}
            aria-labelledby="comic-tool-quality-label"
          />
        </div>
      {/if}
      {#if target === "webp"}
        <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
          <Switch
            checked={webpLossless}
            onCheckedChange={(value) => (webpLossless = value === true)}
          />
          {m.comic_edit_tool_webp_lossless()}
        </label>
      {/if}
      {#if target === "png"}
        <div class="flex flex-col gap-1.5">
          <div class="flex items-baseline justify-between">
            <span id="comic-tool-png-level-label" class="text-sm font-medium">
              {m.comic_edit_tool_png_level()}
            </span>
            <span class="text-sm text-muted-foreground tabular-nums">{pngLevel}</span>
          </div>
          <Slider
            type="single"
            value={pngLevel}
            min={0}
            max={9}
            step={1}
            onValueChange={(value) => (pngLevel = value)}
            aria-labelledby="comic-tool-png-level-label"
          />
        </div>
      {/if}
      <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
        <Switch
          checked={forceConvert}
          onCheckedChange={(value) => (forceConvert = value === true)}
        />
        {m.comic_edit_tool_force()}
      </label>
    {:else if tool === "compress"}
      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between">
          <span id="comic-tool-scale-label" class="text-sm font-medium">
            {m.comic_edit_tool_scale()}
          </span>
          <span class="text-sm text-muted-foreground tabular-nums">{compressScale}%</span>
        </div>
        <Slider
          type="single"
          value={compressScale}
          min={25}
          max={100}
          step={1}
          onValueChange={(value) => (compressScale = value)}
          aria-labelledby="comic-tool-scale-label"
        />
        <p class="text-xs text-muted-foreground">{m.comic_edit_tool_scale_hint()}</p>
      </div>
      <div class="flex flex-col gap-1.5">
        <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
          <Switch
            checked={compressGrayscale}
            onCheckedChange={(value) => (compressGrayscale = value === true)}
          />
          {m.comic_edit_tool_grayscale()}
        </label>
        <p class="text-xs text-muted-foreground">{m.comic_edit_tool_grayscale_hint()}</p>
      </div>
      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between">
          <span id="comic-tool-compress-quality-label" class="text-sm font-medium">
            {m.comic_edit_tool_quality()}
          </span>
          <span class="text-sm text-muted-foreground tabular-nums">{compressQuality}</span>
        </div>
        <Slider
          type="single"
          value={compressQuality}
          min={1}
          max={100}
          step={1}
          onValueChange={(value) => (compressQuality = value)}
          aria-labelledby="comic-tool-compress-quality-label"
        />
      </div>
      <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
        <Switch
          checked={compressWebpLossless}
          onCheckedChange={(value) => (compressWebpLossless = value === true)}
        />
        {m.comic_edit_tool_webp_lossless()}
      </label>
      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between">
          <span id="comic-tool-compress-png-level-label" class="text-sm font-medium">
            {m.comic_edit_tool_png_level()}
          </span>
          <span class="text-sm text-muted-foreground tabular-nums">{compressPngLevel}</span>
        </div>
        <Slider
          type="single"
          value={compressPngLevel}
          min={0}
          max={9}
          step={1}
          onValueChange={(value) => (compressPngLevel = value)}
          aria-labelledby="comic-tool-compress-png-level-label"
        />
      </div>
      <p class="text-xs text-muted-foreground">{m.comic_edit_tool_format_note()}</p>
    {:else}
      <div class="flex flex-col gap-1.5">
        <span id="comic-tool-resize-mode-label" class="text-sm font-medium">
          {m.comic_edit_tool_resize_mode()}
        </span>
        <Select
          type="single"
          value={resizeMode}
          items={resizeModeItems}
          onValueChange={handleResizeModeChange}
        >
          <SelectTrigger
            class="w-full shrink-0"
            aria-labelledby="comic-tool-resize-mode-label"
            disabled={running}
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each resizeModeItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>
      {#if resizeMode === "fixed"}
        <div class="flex flex-col gap-1.5">
          <span id="comic-tool-resize-aspect-label" class="text-sm font-medium">
            {m.comic_export_pdf_aspect_label()}
          </span>
          <Select
            type="single"
            value={resizeAspectLock}
            items={resizeAspectLockItems}
            onValueChange={handleResizeAspectLockChange}
          >
            <SelectTrigger
              class="w-full shrink-0"
              aria-labelledby="comic-tool-resize-aspect-label"
              disabled={running}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each resizeAspectLockItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <label class="flex items-center gap-2 text-sm">
            <span class="font-medium">{m.comic_edit_tool_resize_width()}</span>
            <Input
              type="number"
              min={1}
              step={1}
              value={resizeWidth}
              aria-label={m.comic_edit_tool_resize_width()}
              disabled={running}
              oninput={(event) => handleResizeWidthInput(event.currentTarget.value)}
              class="w-28"
            />
          </label>
          <label class="flex items-center gap-2 text-sm">
            <span class="font-medium">{m.comic_edit_tool_resize_height()}</span>
            <Input
              type="number"
              min={1}
              step={1}
              value={resizeHeight}
              aria-label={m.comic_edit_tool_resize_height()}
              disabled={running}
              oninput={(event) => handleResizeHeightInput(event.currentTarget.value)}
              class="w-28"
            />
          </label>
        </div>
      {:else}
        <div class="flex flex-col gap-1.5">
          <label class="flex items-center gap-2 text-sm">
            <span class="font-medium">
              {resizeMode === "height"
                ? m.comic_edit_tool_resize_height()
                : resizeMode === "long_edge"
                  ? m.comic_edit_tool_resize_edge()
                  : m.comic_edit_tool_resize_width()}
            </span>
            {#if resizeMode === "height"}
              <Input
                type="number"
                min={1}
                step={1}
                value={resizeHeight}
                aria-label={m.comic_edit_tool_resize_height()}
                disabled={running}
                oninput={(event) => (resizeHeight = event.currentTarget.value)}
                class="w-28"
              />
            {:else}
              <Input
                type="number"
                min={1}
                step={1}
                value={resizeWidth}
                aria-label={resizeMode === "long_edge"
                  ? m.comic_edit_tool_resize_edge()
                  : m.comic_edit_tool_resize_width()}
                disabled={running}
                oninput={(event) => (resizeWidth = event.currentTarget.value)}
                class="w-28"
              />
            {/if}
            <span class="text-xs text-muted-foreground">px</span>
          </label>
        </div>
      {/if}
      {#if resizeMode === "fixed"}
        <div class="flex flex-col gap-1.5">
          <span id="comic-tool-resize-fill-label" class="text-sm font-medium">
            {m.comic_export_pdf_fill_label()}
          </span>
          <Select
            type="single"
            value={resizeFill}
            items={resizeFillItems}
            onValueChange={handleResizeFillChange}
          >
            <SelectTrigger
              class="w-full shrink-0"
              aria-labelledby="comic-tool-resize-fill-label"
              disabled={running}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each resizeFillItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
        {#if resizeFill === "stretch"}
          <p class="text-xs text-destructive">{m.comic_edit_tool_resize_stretch_warning()}</p>
        {/if}
        {#if resizeFill === "contain"}
          <div class="flex flex-col gap-1.5">
            <span id="comic-tool-resize-background-label" class="text-sm font-medium">
              {m.comic_edit_tool_resize_background()}
            </span>
            <Select
              type="single"
              value={resizeBackground}
              items={resizeBackgroundItems}
              onValueChange={handleResizeBackgroundChange}
            >
              <SelectTrigger
                class="w-full shrink-0"
                aria-labelledby="comic-tool-resize-background-label"
                disabled={running}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each resizeBackgroundItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
            <p class="text-xs text-muted-foreground">
              {m.comic_edit_tool_resize_transparent_note()}
            </p>
          </div>
        {/if}
        {#if resizeFill !== "stretch"}
          <div class="flex flex-wrap items-center gap-2">
            <span class="shrink-0 text-sm font-medium">
              {m.comic_export_pdf_horizontal_alignment_label()}
            </span>
            <Select
              type="single"
              value={resizeHAlign}
              items={resizeHAlignItems}
              onValueChange={handleResizeHAlignChange}
            >
              <SelectTrigger
                class="w-28"
                aria-label={m.comic_export_pdf_horizontal_alignment_label()}
                disabled={running}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each resizeHAlignItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
            <span class="shrink-0 text-sm font-medium">
              {m.comic_export_pdf_vertical_alignment_label()}
            </span>
            <Select
              type="single"
              value={resizeVAlign}
              items={resizeVAlignItems}
              onValueChange={handleResizeVAlignChange}
            >
              <SelectTrigger
                class="w-28"
                aria-label={m.comic_export_pdf_vertical_alignment_label()}
                disabled={running}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each resizeVAlignItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>
        {/if}
      {/if}
      <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
        <Switch
          checked={resizeNoUpscale}
          disabled={running || !noUpscaleEnabled}
          onCheckedChange={(value) => (resizeNoUpscale = value === true)}
        />
        {m.comic_edit_tool_resize_no_upscale()}
      </label>
      <p class="text-xs text-muted-foreground">{m.comic_edit_tool_resize_no_upscale_hint()}</p>
      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between">
          <span id="comic-tool-resize-quality-label" class="text-sm font-medium">
            {m.comic_edit_tool_resize_quality()}
          </span>
          <span class="text-sm text-muted-foreground tabular-nums">{resizeQuality}</span>
        </div>
        <Slider
          type="single"
          value={resizeQuality}
          min={1}
          max={100}
          step={1}
          onValueChange={(value) => (resizeQuality = value)}
          aria-labelledby="comic-tool-resize-quality-label"
        />
      </div>
      <label class="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
        <Switch
          checked={resizeWebpLossless}
          onCheckedChange={(value) => (resizeWebpLossless = value === true)}
        />
        {m.comic_edit_tool_resize_webp_lossless()}
      </label>
      <div class="flex flex-col gap-1.5">
        <div class="flex items-baseline justify-between">
          <span id="comic-tool-resize-png-level-label" class="text-sm font-medium">
            {m.comic_edit_tool_resize_png_level()}
          </span>
          <span class="text-sm text-muted-foreground tabular-nums">{resizePngLevel}</span>
        </div>
        <Slider
          type="single"
          value={resizePngLevel}
          min={0}
          max={9}
          step={1}
          onValueChange={(value) => (resizePngLevel = value)}
          aria-labelledby="comic-tool-resize-png-level-label"
        />
      </div>
      <p class="text-xs text-muted-foreground">{m.comic_edit_tool_format_note()}</p>
      <p class="text-xs text-muted-foreground">{m.comic_edit_tool_resize_shared_backup_note()}</p>
    {/if}
    {#if running && progress != null}
      <div class="flex flex-col gap-1.5" role="status" aria-live="polite">
        <p class="text-sm text-muted-foreground">
          {m.comic_edit_tool_progress({ progress })}
        </p>
        <Progress value={progress} />
      </div>
    {/if}
  </div>
  <div class="flex shrink-0 gap-2 border-t pt-3">
    <Button
      class="min-w-0 flex-1"
      size="sm"
      disabled={!running && !canRun}
      onclick={running ? onCancel : onRun}
    >
      {running ? m.comic_edit_tool_cancel() : m.comic_edit_tool_run()}
    </Button>
    <Button variant="outline" size="sm" disabled={!hasOutcome} onclick={onClear}>
      {m.comic_edit_tool_reset()}
    </Button>
  </div>
  {#if backupExists}
    <div class="flex shrink-0 gap-2">
      <Button
        variant="outline"
        class="min-w-0 flex-1"
        size="sm"
        disabled={running}
        onclick={onUndo}
      >
        {m.comic_edit_tool_undo()}
      </Button>
      <Button
        variant="outline"
        class="min-w-0 flex-1"
        size="sm"
        disabled={running}
        onclick={onRequestClearBackup}
      >
        {m.comic_edit_tool_clear_backup()}
      </Button>
    </div>
  {/if}
</section>
