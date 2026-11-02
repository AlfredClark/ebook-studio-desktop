<script lang="ts">
  // 外观设置分组：主题/配色/布局/字重/字号简单行 + 字体选择器，纯前端持久化，零 props。
  // 主题经 mode-watcher、配色/布局与字体经外观 hook 即时生效，均不经过后端。
  import { setMode, userPrefersMode } from "mode-watcher";
  import type { ColorTheme, LayoutName } from "#libs/hooks/appearance.svelte";
  import {
    colorThemeState,
    FONT_SIZE_OPTIONS,
    FONT_WEIGHT_STEP,
    fontState,
    layoutState,
    MAX_FONT_WEIGHT,
    MIN_FONT_WEIGHT,
    setColorTheme,
    setFontSize,
    setFontWeight,
    setLayoutName,
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
  import FontFamilyPicker from "./font-family-picker.svelte";

  /** 主题下拉的候选项：`items` 让触发器在下拉关闭时也能解析出展示文案。 */
  const themeItems: { value: "light" | "dark" | "system"; label: string }[] = [
    { value: "light", label: m.settings_theme_option_light() },
    { value: "dark", label: m.settings_theme_option_dark() },
    { value: "system", label: m.settings_theme_option_system() },
  ];

  /** 布局下拉的候选项，同上。 */
  const layoutItems: { value: LayoutName; label: string }[] = [
    { value: "tabs", label: m.settings_layout_option_tabs() },
    { value: "sidebar", label: m.settings_layout_option_sidebar() },
    { value: "dashboard", label: m.settings_layout_option_dashboard() },
  ];

  /** 配色下拉的候选项，同上；顺序与取值元组一致。 */
  const colorThemeItems: { value: ColorTheme; label: string }[] = [
    { value: "neutral", label: m.settings_color_theme_option_neutral() },
    { value: "teal", label: m.settings_color_theme_option_teal() },
    { value: "tangerine", label: m.settings_color_theme_option_tangerine() },
    { value: "rose", label: m.settings_color_theme_option_rose() },
    { value: "amethyst", label: m.settings_color_theme_option_amethyst() },
    { value: "nord", label: m.settings_color_theme_option_nord() },
    { value: "ocean", label: m.settings_color_theme_option_ocean() },
    { value: "emerald", label: m.settings_color_theme_option_emerald() },
    { value: "amber", label: m.settings_color_theme_option_amber() },
    { value: "dracula", label: m.settings_color_theme_option_dracula() },
    { value: "tokyo-night", label: m.settings_color_theme_option_tokyo_night() },
  ];

  /** 字号下拉的候选项，同上；取值存字符串，提交时收窄为数值。 */
  const fontSizeItems: { value: string; label: string }[] = FONT_SIZE_OPTIONS.map((size) => ({
    value: String(size),
    label: `${size}%`,
  }));

  /** 主题即时生效，仅前端持久化，不经过后端。 */
  // 入参用 `string` 是 `bits-ui` 的 `onValueChange` 约束，内部经候选项收窄后才生效，无类型断言
  function handleThemeChange(value: string): void {
    const theme = themeItems.find((item) => item.value === value)?.value;
    if (theme) setMode(theme);
  }

  /** 布局即时生效，仅前端 `localStorage` 持久化，不经过后端。 */
  // 同上：`bits-ui` 给 `string`，经 `layoutItems` 收窄为 `LayoutName` 后才提交
  function handleLayoutChange(value: string): void {
    const layout = layoutItems.find((item) => item.value === value)?.value;
    if (layout) setLayoutName(layout);
  }

  /** 配色即时生效，同上；经 `colorThemeItems` 收窄为 `ColorTheme` 后提交。 */
  function handleColorThemeChange(value: string): void {
    const theme = colorThemeItems.find((item) => item.value === value)?.value;
    if (theme) setColorTheme(theme);
  }

  /** 字重即时生效，同上；单值滑块直接给数值。 */
  // 拖动中只预览（落盘延后到松手提交），避免逐 tick 同步 IO
  function handleFontWeightChange(value: number): void {
    setFontWeight(value, false);
  }

  /** 字重松手提交：预览值落盘，刷新不丢失。 */
  function handleFontWeightCommit(value: number): void {
    setFontWeight(value);
  }

  /** 字号即时生效，同上；下拉给字符串，经候选项收窄为数值后提交。 */
  function handleFontSizeChange(value: string): void {
    const size = fontSizeItems.find((item) => item.value === value)?.value;
    if (size !== undefined) setFontSize(Number(size));
  }
</script>

<CardSection
  title={m.settings_appearance_title()}
  description={m.settings_appearance_description()}
>
  <CardRow label={m.settings_theme_label()} description={m.settings_theme_description()}>
    {#snippet control()}
      <Select
        type="single"
        value={userPrefersMode.current}
        items={themeItems}
        onValueChange={handleThemeChange}
      >
        <SelectTrigger class="w-(--control-column-width)" aria-label={m.settings_theme_label()}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="light">{m.settings_theme_option_light()}</SelectItem>
          <SelectItem value="dark">{m.settings_theme_option_dark()}</SelectItem>
          <SelectItem value="system">{m.settings_theme_option_system()}</SelectItem>
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_color_theme_label()}
    description={m.settings_color_theme_description()}
  >
    {#snippet control()}
      <Select
        type="single"
        value={colorThemeState.name}
        items={colorThemeItems}
        onValueChange={handleColorThemeChange}
      >
        <SelectTrigger
          class="w-(--control-column-width)"
          aria-label={m.settings_color_theme_label()}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {#each colorThemeItems as item (item.value)}
            <SelectItem value={item.value}>{item.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
  <CardRow label={m.settings_layout_label()} description={m.settings_layout_description()}>
    {#snippet control()}
      <Select
        type="single"
        value={layoutState.name}
        items={layoutItems}
        onValueChange={handleLayoutChange}
      >
        <SelectTrigger class="w-(--control-column-width)" aria-label={m.settings_layout_label()}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="tabs">{m.settings_layout_option_tabs()}</SelectItem>
          <SelectItem value="sidebar">{m.settings_layout_option_sidebar()}</SelectItem>
          <SelectItem value="dashboard">{m.settings_layout_option_dashboard()}</SelectItem>
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
  <FontFamilyPicker />
  <CardRow
    label={m.settings_font_weight_label()}
    description={m.settings_font_weight_description()}
  >
    {#snippet control()}
      <div class="flex w-(--control-column-width) items-center gap-3">
        <Slider
          type="single"
          value={fontState.weight}
          min={MIN_FONT_WEIGHT}
          max={MAX_FONT_WEIGHT}
          step={FONT_WEIGHT_STEP}
          onValueChange={handleFontWeightChange}
          onValueCommit={handleFontWeightCommit}
          ariaLabel={m.settings_font_weight_label()}
        />
        <!-- 读数与滑块的 aria-valuenow 同源，重复播报，故对 AT 隐藏 -->
        <span
          class="w-8 shrink-0 text-right text-sm text-muted-foreground tabular-nums"
          aria-hidden="true"
        >
          {fontState.weight}
        </span>
      </div>
    {/snippet}
  </CardRow>
  <CardRow label={m.settings_font_size_label()} description={m.settings_font_size_description()}>
    {#snippet control()}
      <Select
        type="single"
        value={String(fontState.size)}
        items={fontSizeItems}
        onValueChange={handleFontSizeChange}
      >
        <SelectTrigger class="w-(--control-column-width)" aria-label={m.settings_font_size_label()}>
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
</CardSection>
