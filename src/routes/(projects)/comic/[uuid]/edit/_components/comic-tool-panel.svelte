<script lang="ts">
  // 漫画右栏图片工具（哑组件）：顶部工具选择固定，中间参数区按工具切换，
  // 底部执行 + 重置贴底；参数与开关状态由父组件持有（$bindable 透传）。
  import { Button } from "#components/shadcn-svelte/button";
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
  import type { ImageTargetFormat } from "#libs/commands/types";
  import { m } from "#libs/i18n/paraglide/messages";

  /** 工具类型：格式转换 / 图片压缩（压缩后续接入）。 */
  export type ComicTool = "convert" | "compress";

  let {
    tool = $bindable("convert"),
    target = $bindable("jpeg"),
    quality = $bindable(85),
    webpLossless = $bindable(false),
    pngLevel = $bindable(6),
    forceConvert = $bindable(false),
    running = false,
    canRun = false,
    hasOutcome = false,
    progress = null,
    onRun,
    onClear,
  }: {
    tool: ComicTool;
    target: ImageTargetFormat;
    quality: number;
    webpLossless: boolean;
    pngLevel: number;
    forceConvert: boolean;
    running: boolean;
    canRun: boolean;
    hasOutcome: boolean;
    progress: number | null;
    onRun: () => void;
    onClear: () => void;
  } = $props();

  /** 工具下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const toolItems = [
    { value: "convert", label: m.comic_edit_tool_convert() },
    { value: "compress", label: m.comic_edit_tool_compress() },
  ];

  /** 目标格式下拉候选项：格式名通用，无需国际化。 */
  const formatItems: { value: ImageTargetFormat; label: string }[] = [
    { value: "jpeg", label: "JPEG" },
    { value: "png", label: "PNG" },
    { value: "webp", label: "WebP" },
  ];

  /** 工具切换：下拉给字符串，经候选项收窄后提交，无类型断言。 */
  function handleToolChange(value: string): void {
    if (value === "convert" || value === "compress") {
      tool = value;
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
  class="flex h-full min-h-0 flex-col gap-4 px-6 pt-4 pb-6"
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
  <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
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
    {:else}
      <!-- 占位：图片压缩参数区，后续接入具体实现时替换并补国际化。 -->
      <p class="text-sm text-muted-foreground">图片压缩参数占位，后续接入</p>
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
    <Button class="min-w-0 flex-1" size="sm" disabled={!canRun} onclick={onRun}>
      {running ? m.comic_edit_tool_running() : m.comic_edit_tool_run()}
    </Button>
    <Button variant="outline" size="sm" disabled={!hasOutcome} onclick={onClear}>
      {m.comic_edit_tool_reset()}
    </Button>
  </div>
</section>
