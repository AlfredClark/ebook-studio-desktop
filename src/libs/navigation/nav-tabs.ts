// 顶部导航标签注册表：`path` 与 SvelteKit 路由一一对应，同时作为 tabs 的取值。
// 文案直接取 Paraglide 消息函数；运行时的 getLocale 已由 #libs/i18n/locale.svelte.ts 覆盖为
// 响应式读取，启动水合对齐（不重载）后自动重渲染，故此处无需响应式包装。
import InfoIcon from "@lucide/svelte/icons/info";
import HouseIcon from "@lucide/svelte/icons/house";
import SettingsIcon from "@lucide/svelte/icons/settings";
import { m } from "#libs/i18n/paraglide/messages";

/** 导航标签清单，数组顺序即渲染顺序；`as const` 保留路径字面量类型，注册错路由会在编译期报错 */
export const NAV_TABS = [
  { path: "/", label: m.nav_home, icon: HouseIcon },
  { path: "/settings", label: m.nav_settings, icon: SettingsIcon },
  { path: "/about", label: m.nav_about, icon: InfoIcon },
] as const;

/** 单个导航标签 */
export type NavTab = (typeof NAV_TABS)[number];

/** 已登记的页面路径，`resolve()` 需要具体的路由字面量而非 `string` */
export type NavTabPath = NavTab["path"];

/** 按路径匹配标签，未命中返回 `undefined`（子页面等不选中任何标签） */
export function resolveNavTab(pathname: string): NavTab | undefined {
  return NAV_TABS.find((tab) => tab.path === pathname);
}

/**
 * 导航项的语义状态：三套布局共用，保证同一份目的地清单在三种形态下语义一致。
 *
 * 返回 `aria-current` 的取值；未命中注册表的路径（如子页面）不选中任何项。 `href` / `goto()` 的参数由各渲染点自行
 * `resolve()`——导航目标属于路由层，不在这里代劳。
 */
export function resolveNavCurrent(path: NavTabPath, pathname: string): "page" | undefined {
  return resolveNavTab(pathname)?.path === path ? "page" : undefined;
}
