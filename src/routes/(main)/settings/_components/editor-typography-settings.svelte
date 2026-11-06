<script lang="ts">
  // 编辑器排版设置分组：字体/字重/字号/首行缩进，纯前端持久化，零 props。
  // 与编辑器头部快捷入口读写同一份编辑器排版状态，即时生效，均不经过后端。
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
  import { Slider } from "#components/shadcn-svelte/slider";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { m } from "#libs/i18n/paraglide/messages";
  import FontFamilyPicker from "#components/common/font-family-picker.svelte";

  /** 字号下拉的候选项：`items` 让触发器在下拉关闭时也能解析出展示文案。 */
  const fontSizeItems: { value: string; label: string }[] = EDITOR_FONT_SIZE_OPTIONS.map(
    (size) => ({
      value: String(size),
      label: `${size}px`,
    }),
  );

  /** 字重即时生效，仅前端持久化，不经过后端。 */
  // 拖动中只预览（落盘延后到松手提交），避免逐 tick 同步 IO
  function handleFontWeightChange(value: number): void {
    setEditorFontWeight(value, false);
  }

  /** 字重松手提交：预览值落盘，刷新不丢失。 */
  function handleFontWeightCommit(value: number): void {
    setEditorFontWeight(value);
  }

  /** 字号即时生效，同上；下拉给字符串，经候选项收窄为数值后提交。 */
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

<CardSection title={m.settings_editor_title()} description={m.settings_editor_description()}>
  <FontFamilyPicker
    family={editorTypographyState.family}
    onSelectFamily={setEditorFontFamily}
    label={m.settings_editor_font_family_label()}
    description={m.settings_editor_font_family_description()}
  />
  <CardRow
    label={m.settings_editor_font_weight_label()}
    description={m.settings_editor_font_weight_description()}
  >
    {#snippet control()}
      <div class="flex w-(--control-column-width) items-center gap-3">
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
        <!-- 读数与滑块的 aria-valuenow 同源，重复播报，故对 AT 隐藏 -->
        <span
          class="w-8 shrink-0 text-right text-sm text-muted-foreground tabular-nums"
          aria-hidden="true"
        >
          {editorTypographyState.weight}
        </span>
      </div>
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_editor_font_size_label()}
    description={m.settings_editor_font_size_description()}
  >
    {#snippet control()}
      <Select
        type="single"
        value={String(editorTypographyState.size)}
        items={fontSizeItems}
        onValueChange={handleFontSizeChange}
      >
        <SelectTrigger
          class="w-(--control-column-width)"
          aria-label={m.settings_editor_font_size_label()}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {#each fontSizeItems as item (item.value)}
            <SelectItem value={item.value}>{item.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_editor_indent_label()}
    description={m.settings_editor_indent_description()}
  >
    {#snippet control()}
      <div class="flex w-(--control-column-width) items-center gap-3">
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
        <!-- 读数与滑块的 aria-valuenow 同源，重复播报，故对 AT 隐藏 -->
        <span
          class="w-8 shrink-0 text-right text-sm text-muted-foreground tabular-nums"
          aria-hidden="true"
        >
          {editorTypographyState.indent}
        </span>
      </div>
    {/snippet}
  </CardRow>
</CardSection>
