// 外观状态唯一来源：布局、配色与字体等纯前端 UI 偏好走 localStorage 持久化，与语言走后端 config.json 互不干扰。
// 后端无外观业务，故不经 commands 链，避免跨层跳跃。系统字体列表仅经插件读取做下拉候选，偏好本身仍存前端。
// 布局注册表集中在此，新增布局只需加文件并扩展映射，容器无需改动。
// 布局组件禁止反向导入本模块，否则形成容器到布局的循环依赖。
// 布局名用语义名（如 tabs / sidebar），禁用 default 之类只表达"被选中"的占位名——该值会落盘。
import type { Component, Snippet } from "svelte";
import Tabs from "#components/layout/tabs.svelte";
import { DEFAULT_DASHBOARD_COLLAPSED, setDashboardCollapsed } from "#libs/hooks/dashboard.svelte";

/** 可选布局取值：新增布局时同步扩展该元组与下方加载器（元组即真值来源，避免 `Object.keys` 断言） */
const LAYOUT_NAME_TUPLE = ["tabs", "sidebar", "dashboard"] as const;

/** 可选布局取值，新增布局时同步扩展该联合类型与下方映射 */
export type LayoutName = (typeof LAYOUT_NAME_TUPLE)[number];

/** 布局组件形态：仅接收子内容片段 */
export type LayoutComponent = Component<{ children: Snippet }>;

/** 布局名到加载器的映射：tabs 同步兜底保证首帧，其余懒加载，首屏不下非当前布局代码 */
export const LAYOUTS: Record<LayoutName, () => Promise<LayoutComponent>> = {
  tabs: () => Promise.resolve(Tabs),
  sidebar: () => import("#components/layout/sidebar.svelte").then((module) => module.default),
  dashboard: () => import("#components/layout/dashboard.svelte").then((module) => module.default),
};

/** 持久化键名，改名即视为放弃老用户存量 */
export const LAYOUT_STORAGE_KEY = "layout-name";

/** 默认布局：首选项，改名即视为放弃老用户存量 */
export const DEFAULT_LAYOUT: LayoutName = "tabs";

const LAYOUT_NAMES: readonly LayoutName[] = LAYOUT_NAME_TUPLE;

// 跨页面共享的布局状态，页面私有状态仍用局部 $state
export const layoutState = $state<{ name: LayoutName }>({ name: DEFAULT_LAYOUT });

/** 校验布局取值，脏数据回落时使用 */
export function isLayoutName(value: unknown): value is LayoutName {
  return typeof value === "string" && (LAYOUT_NAMES as readonly string[]).includes(value);
}

/** 读取持久化布局，缺失或非法一律回落默认值 */
export function loadLayoutName(): LayoutName {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_LAYOUT;
    }
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    return isLayoutName(stored) ? stored : DEFAULT_LAYOUT;
  } catch {
    // 隐私模式等存储不可用时按默认值渲染，不阻塞首帧
    return DEFAULT_LAYOUT;
  }
}

/** 容器首帧前调用，重复调用无副作用 */
export function initLayout(): void {
  layoutState.name = loadLayoutName();
}

/** 切换布局，先落盘再改内存，刷新不丢失 */
export function setLayoutName(next: LayoutName): void {
  if (!isLayoutName(next)) {
    return;
  }
  try {
    localStorage.setItem(LAYOUT_STORAGE_KEY, next);
  } catch {
    // 落盘失败仍切换内存状态，保证本次会话可用
  }
  layoutState.name = next;
}

// ---- 配色主题：与布局同为纯前端 UI 偏好，读写模式与上节保持一致 ----
// 明暗（light/dark/system）由 mode-watcher 管 `.dark` 类，此处只管配色
// （鲜艳系 ocean/teal/emerald/amber/tangerine/rose/amethyst + 暗色美学 nord/dracula/tokyo-night），
// 经根元素 `data-color-theme` 属性生效（`themes.css` 的 `[data-color-theme]` 块），两者正交组合。
// 属性名刻意避开 `data-theme`：mode-watcher 的 custom theme 功能硬编码占用该属性，挂载即覆盖为空串。
// 新增配色时同步扩展该元组、`themes.css` 的两块令牌（浅色 + `:root.dark` 暗色）与设置页下拉候选。
// 旧取值（blue/green/violet）已更名，存量经 `isColorTheme` 校验自动回落默认值，无需迁移逻辑。

/** 可选配色取值：新增配色时同步扩展该元组、联合类型与 `themes.css` 对应块（元组即真值来源） */
const COLOR_THEME_TUPLE = [
  "neutral",
  "ocean",
  "teal",
  "emerald",
  "amber",
  "tangerine",
  "rose",
  "amethyst",
  "nord",
  "dracula",
  "tokyo-night",
] as const;

/** 可选配色取值，`neutral` 即 `layout.css` 的 `:root/.dark` 默认值，不写覆盖块 */
export type ColorTheme = (typeof COLOR_THEME_TUPLE)[number];

/** 配色持久化键名，改名即视为放弃老用户存量 */
export const COLOR_THEME_STORAGE_KEY = "color-theme";

/** 默认配色：样式表默认值，无需覆盖变量 */
export const DEFAULT_COLOR_THEME: ColorTheme = "neutral";

const COLOR_THEMES: readonly ColorTheme[] = COLOR_THEME_TUPLE;

// 跨页面共享的配色状态，页面私有状态仍用局部 $state
export const colorThemeState = $state<{ name: ColorTheme }>({ name: DEFAULT_COLOR_THEME });

/** 校验配色取值，脏数据回落时使用 */
export function isColorTheme(value: unknown): value is ColorTheme {
  return typeof value === "string" && (COLOR_THEMES as readonly string[]).includes(value);
}

/** 读取持久化配色，缺失或非法一律回落默认值 */
export function loadColorTheme(): ColorTheme {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_COLOR_THEME;
    }
    const stored = localStorage.getItem(COLOR_THEME_STORAGE_KEY);
    return isColorTheme(stored) ? stored : DEFAULT_COLOR_THEME;
  } catch {
    // 隐私模式等存储不可用时按默认值渲染，不阻塞首帧
    return DEFAULT_COLOR_THEME;
  }
}

/** 把内存态写入根元素属性，无文档环境直接返回；默认配色删属性回落样式表 */
export function applyColorTheme(): void {
  if (typeof document === "undefined") {
    return;
  }
  if (colorThemeState.name === DEFAULT_COLOR_THEME) {
    document.documentElement.removeAttribute("data-color-theme");
  } else {
    document.documentElement.setAttribute("data-color-theme", colorThemeState.name);
  }
}

/** 切换配色，先落盘再改内存并即时应用，刷新不丢失 */
export function setColorTheme(next: ColorTheme): void {
  if (!isColorTheme(next)) {
    return;
  }
  try {
    localStorage.setItem(COLOR_THEME_STORAGE_KEY, next);
  } catch {
    // 落盘失败仍切换内存状态，保证本次会话可用
  }
  colorThemeState.name = next;
  applyColorTheme();
}

// ---- 字体偏好：与布局同为纯前端 UI 偏好，读写模式与上节保持一致 ----

/** 字体持久化键名，改名即视为放弃老用户存量 */
export const FONT_FAMILY_STORAGE_KEY = "font-family";
export const FONT_WEIGHT_STORAGE_KEY = "font-weight";
export const FONT_SIZE_STORAGE_KEY = "font-size";

/** 跟随系统默认字体栈的哨兵取值，语义明确而非占位名 */
export const DEFAULT_FONT_FAMILY = "system";

export const DEFAULT_FONT_WEIGHT = 400;
export const MIN_FONT_WEIGHT = 100;
export const MAX_FONT_WEIGHT = 900;
export const FONT_WEIGHT_STEP = 100;

/** 字号以相对浏览器默认的百分比存储，默认不缩放 */
export const DEFAULT_FONT_SIZE = 100;
export const MIN_FONT_SIZE = 75;
export const MAX_FONT_SIZE = 125;
export const FONT_SIZE_STEP = 5;

/** 字号下拉候选项，与校验区间同源，避免两处各写一份 */
export const FONT_SIZE_OPTIONS: readonly number[] = (() => {
  const options: number[] = [];
  for (let size = MIN_FONT_SIZE; size <= MAX_FONT_SIZE; size += FONT_SIZE_STEP) {
    options.push(size);
  }
  return options;
})();

/** 默认字体栈，与样式表 `:root` 的回退值保持一致 */
export const DEFAULT_FONT_STACK = `"Geist Variable", sans-serif`;

// 跨页面共享的字体状态，页面私有状态仍用局部 $state
export const fontState = $state<{ family: string; weight: number; size: number }>({
  family: DEFAULT_FONT_FAMILY,
  weight: DEFAULT_FONT_WEIGHT,
  size: DEFAULT_FONT_SIZE,
});

/** 清洗字体族名：插件返回的系统字体是外部输入，组装样式前剥离可破坏声明的字符 */
export function sanitizeFontFamily(value: string): string {
  return value.replace(/["';\\]/g, "").trim();
}

/** 校验字重取值，仅接受区间内的整百数 */
export function isFontWeight(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= MIN_FONT_WEIGHT &&
    value <= MAX_FONT_WEIGHT &&
    value % FONT_WEIGHT_STEP === 0
  );
}

/** 校验字号取值，仅接受区间内步长倍数的百分比 */
export function isFontSize(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= MIN_FONT_SIZE &&
    value <= MAX_FONT_SIZE &&
    value % FONT_SIZE_STEP === 0
  );
}

/** 读取持久化字体族名，缺失或非法一律回落系统默认 */
export function loadFontFamily(): string {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_FONT_FAMILY;
    }
    const cleaned = sanitizeFontFamily(localStorage.getItem(FONT_FAMILY_STORAGE_KEY) ?? "");
    return cleaned ? cleaned : DEFAULT_FONT_FAMILY;
  } catch {
    // 隐私模式等存储不可用时按默认值渲染，不阻塞首帧
    return DEFAULT_FONT_FAMILY;
  }
}

/** 读取持久化字重，缺失或非法一律回落默认值 */
export function loadFontWeight(): number {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_FONT_WEIGHT;
    }
    const parsed = Number(localStorage.getItem(FONT_WEIGHT_STORAGE_KEY));
    return isFontWeight(parsed) ? parsed : DEFAULT_FONT_WEIGHT;
  } catch {
    return DEFAULT_FONT_WEIGHT;
  }
}

/** 读取持久化字号，缺失或非法一律回落默认值 */
export function loadFontSize(): number {
  try {
    if (typeof localStorage === "undefined") {
      return DEFAULT_FONT_SIZE;
    }
    const parsed = Number(localStorage.getItem(FONT_SIZE_STORAGE_KEY));
    return isFontSize(parsed) ? parsed : DEFAULT_FONT_SIZE;
  } catch {
    return DEFAULT_FONT_SIZE;
  }
}

/** 组装字体栈：选中族名优先，默认栈兜底已卸载等缺失情形 */
export function buildFontStack(family: string): string {
  if (!family || family === DEFAULT_FONT_FAMILY) {
    return DEFAULT_FONT_STACK;
  }
  return `"${family}", ${DEFAULT_FONT_STACK}`;
}

/** 把内存态写入根元素变量，无文档环境直接返回 */
export function applyAppearance(): void {
  if (typeof document === "undefined") {
    return;
  }
  const root = document.documentElement;
  root.style.setProperty("--app-font-family", buildFontStack(fontState.family));
  root.style.setProperty("--app-font-weight", String(fontState.weight));
  root.style.setProperty("--app-font-size", `${fontState.size}%`);
}

/** 切换字体族，先落盘再改内存并即时应用，刷新不丢失 */
export function setFontFamily(next: string): void {
  const cleaned = sanitizeFontFamily(next);
  if (!cleaned) {
    return;
  }
  try {
    localStorage.setItem(FONT_FAMILY_STORAGE_KEY, cleaned);
  } catch {
    // 落盘失败仍切换内存状态，保证本次会话可用
  }
  fontState.family = cleaned;
  applyAppearance();
}

/** 切换字重，先落盘再改内存并即时应用，非法取值直接拒绝 */
export function setFontWeight(next: number, persist = true): void {
  if (!isFontWeight(next)) {
    return;
  }
  if (persist) {
    try {
      localStorage.setItem(FONT_WEIGHT_STORAGE_KEY, String(next));
    } catch {
      // 落盘失败仍切换内存状态，保证本次会话可用
    }
  }
  fontState.weight = next;
  applyAppearance();
}

/** 切换字号，先落盘再改内存并即时应用，非法取值直接拒绝 */
export function setFontSize(next: number): void {
  if (!isFontSize(next)) {
    return;
  }
  try {
    localStorage.setItem(FONT_SIZE_STORAGE_KEY, String(next));
  } catch {
    // 落盘失败仍切换内存状态，保证本次会话可用
  }
  fontState.size = next;
  applyAppearance();
}

/** 根布局首帧前调用，重复调用无副作用 */
export function initAppearance(): void {
  fontState.family = loadFontFamily();
  fontState.weight = loadFontWeight();
  fontState.size = loadFontSize();
  applyAppearance();
  colorThemeState.name = loadColorTheme();
  applyColorTheme();
}

/** 恢复全部外观偏好为默认值：复用各 `set*` 的落盘+应用路径，不另写旁路 */
export function resetAppearance(): void {
  setLayoutName(DEFAULT_LAYOUT);
  setDashboardCollapsed(DEFAULT_DASHBOARD_COLLAPSED);
  setColorTheme(DEFAULT_COLOR_THEME);
  setFontFamily(DEFAULT_FONT_FAMILY);
  setFontWeight(DEFAULT_FONT_WEIGHT);
  setFontSize(DEFAULT_FONT_SIZE);
}
