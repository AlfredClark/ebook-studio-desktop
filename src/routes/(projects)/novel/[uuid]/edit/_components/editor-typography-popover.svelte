<script lang="ts">
  // 编辑器排版快捷入口：头部保存按钮旁的 ghost 图标按钮，弹层内纵向排布字体/字号/字重/缩进。
  // 与设置页完整版读写同一份编辑器排版状态，即时生效，不经过后端。
  import TypeIcon from "@lucide/svelte/icons/type";
  import {
    EDITOR_FONT_SIZE_OPTIONS,
    EDITOR_INDENT_STEP,
    editorTypographyState,
    FONT_WEIGHT_STEP,
    MAX_EDITOR_INDENT,
    MAX_FONT_WEIGHT,
    MIN_EDITOR_INDENT,
    MIN_FONT_WEIGHT,
    setEditorFontFamily,
    setEditorFontSize,
    setEditorFontWeight,
    setEditorIndent,
  } from "#libs/hooks/appearance.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import {
    Content as PopoverContent,
    Root as PopoverRoot,
    Title as PopoverTitle,
    Trigger as PopoverTrigger,
  } from "#components/shadcn-svelte/popover";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { Slider } from "#components/shadcn-svelte/slider";
  import { m } from "#libs/i18n/paraglide/messages";
  import FontFamilyPicker from "#components/common/font-family-picker.svelte";

  /** 字号下拉的候选项：`items` 让触发器在下拉关闭时也能解析出展示文案。 */
  const fontSizeItems: { value: string; label: string }[] = EDITOR_FONT_SIZE_OPTIONS.map(
    (size) => ({
      value: String(size),
      label: `${size}px`,
    }),
  );

  /** 字重即时生效，仅前端持久化；拖动中只预览，松手提交才落盘。 */
  function handleFontWeightChange(value: number): void {
    setEditorFontWeight(value, false);
  }

  /** 字重松手提交：预览值落盘，刷新不丢失。 */
  function handleFontWeightCommit(value: number): void {
    setEditorFontWeight(value);
  }

  /** 字号即时生效；下拉给字符串，经候选项收窄为数值后提交。 */
  function handleFontSizeChange(value: string): void {
    const size = fontSizeItems.find((item) => item.value === value)?.value;
    if (size !== undefined) {
      setEditorFontSize(Number(size));
    }
  }

  /** 首行缩进即时生效，同字重；拖动中只预览，松手提交才落盘。 */
  function handleIndentChange(value: number): void {
    setEditorIndent(value, false);
  }

  /** 首行缩进松手提交：预览值落盘，刷新不丢失。 */
  function handleIndentCommit(value: number): void {
    setEditorIndent(value);
  }
</script>

<PopoverRoot>
  <PopoverTrigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="ghost"
        size="icon-sm"
        aria-label={m.edit_typography_label()}
        title={m.edit_typography_label()}
      >
        <TypeIcon />
      </Button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent class="flex w-72 flex-col gap-4 p-4" align="end">
    <PopoverTitle class="text-sm font-medium">{m.settings_editor_title()}</PopoverTitle>
    <div class="flex flex-col gap-1.5">
      <p class="text-sm text-muted-foreground">{m.settings_editor_font_family_label()}</p>
      <FontFamilyPicker
        layout="plain"
        triggerClass="w-full justify-between"
        family={editorTypographyState.family}
        onSelectFamily={setEditorFontFamily}
        label={m.settings_editor_font_family_label()}
        description={m.settings_editor_font_family_description()}
      />
    </div>
    <div class="flex flex-col gap-1.5">
      <p class="text-sm text-muted-foreground">{m.settings_editor_font_size_label()}</p>
      <Select
        type="single"
        value={String(editorTypographyState.size)}
        items={fontSizeItems}
        onValueChange={handleFontSizeChange}
      >
        <SelectTrigger class="w-full" aria-label={m.settings_editor_font_size_label()}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {#each fontSizeItems as item (item.value)}
            <SelectItem value={item.value}>{item.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </div>
    <div class="flex flex-col gap-1.5">
      <div class="flex items-baseline justify-between">
        <p class="text-sm text-muted-foreground">{m.settings_editor_font_weight_label()}</p>
        <!-- 读数与滑块的 aria-valuenow 同源，重复播报，故对 AT 隐藏 -->
        <span class="text-sm text-muted-foreground tabular-nums" aria-hidden="true">
          {editorTypographyState.weight}
        </span>
      </div>
      <Slider
        type="single"
        value={editorTypographyState.weight}
        min={MIN_FONT_WEIGHT}
        max={MAX_FONT_WEIGHT}
        step={FONT_WEIGHT_STEP}
        onValueChange={handleFontWeightChange}
        onValueCommit={handleFontWeightCommit}
        ariaLabel={m.settings_editor_font_weight_label()}
      />
    </div>
    <div class="flex flex-col gap-1.5">
      <div class="flex items-baseline justify-between">
        <p class="text-sm text-muted-foreground">{m.settings_editor_indent_label()}</p>
        <!-- 读数与滑块的 aria-valuenow 同源，重复播报，故对 AT 隐藏 -->
        <span class="text-sm text-muted-foreground tabular-nums" aria-hidden="true">
          {editorTypographyState.indent}
        </span>
      </div>
      <Slider
        type="single"
        value={editorTypographyState.indent}
        min={MIN_EDITOR_INDENT}
        max={MAX_EDITOR_INDENT}
        step={EDITOR_INDENT_STEP}
        onValueChange={handleIndentChange}
        onValueCommit={handleIndentCommit}
        ariaLabel={m.settings_editor_indent_label()}
      />
    </div>
  </PopoverContent>
</PopoverRoot>
