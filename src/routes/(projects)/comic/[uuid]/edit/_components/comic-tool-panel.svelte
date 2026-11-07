<script lang="ts">
  // 漫画右栏图片工具（哑组件）：顶部工具下拉固定，中间参数区按工具切换，
  // 底部执行 + 重置贴底；参数区现阶段为纯文本占位，后续接入具体实现。
  import { Button } from "#components/shadcn-svelte/button";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { m } from "#libs/i18n/paraglide/messages";

  /** 工具类型：格式转换 / 图片压缩。 */
  export type ComicTool = "convert" | "compress";

  let {
    tool = $bindable("convert"),
    running = false,
    canRun = false,
    hasOutcome = false,
    onRun,
    onClear,
  }: {
    tool: ComicTool;
    running: boolean;
    canRun: boolean;
    hasOutcome: boolean;
    onRun: () => void;
    onClear: () => void;
  } = $props();

  /** 工具下拉候选项：触发器关闭时靠 `items` 解析展示文案。 */
  const toolItems = [
    { value: "convert", label: m.comic_edit_tool_convert() },
    { value: "compress", label: m.comic_edit_tool_compress() },
  ];

  /** 下拉给字符串，经候选项收窄后才提交，无类型断言。 */
  function handleToolChange(value: string): void {
    if (value === "convert" || value === "compress") {
      tool = value;
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
      <!-- 占位：格式转换参数区，后续接入具体实现时替换并补国际化。 -->
      <p class="text-sm text-muted-foreground">格式转换参数占位，后续接入</p>
    {:else}
      <!-- 占位：图片压缩参数区，后续接入具体实现时替换并补国际化。 -->
      <p class="text-sm text-muted-foreground">图片压缩参数占位，后续接入</p>
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
