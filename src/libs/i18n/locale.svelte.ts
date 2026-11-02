// 语言响应式桥：Paraglide 的 getLocale 是普通函数，Svelte 感知不到语言变化，
// 启动水合对齐（reload: false）后已渲染的文案不会重渲染——首次启动会残留 baseLocale 文案，刷新才恢复。
// 这里把 getLocale 覆盖为读取 $state：渲染期所有 m.*() 调用自动建立响应式依赖，对齐时原地重渲染。
// 不重载的语言变更只允许经 alignLocale——裸调 setLocale(reload: false) 会绕过本桥，文案停在旧语言。
import { getLocale, overwriteGetLocale, setLocale } from "#libs/i18n/paraglide/runtime";
import type { Locale } from "#libs/i18n/paraglide/runtime";

/** 当前语言：初值取运行时解析结果（localStorage 缓存或 baseLocale） */
const localeState = $state<{ current: Locale }>({ current: getLocale() });

overwriteGetLocale(() => localeState.current);

/** 启动水合对齐：写 localStorage 缓存（下次冷启动首帧即正确）并驱动已渲染文案重渲染；不整页重载 */
export function alignLocale(next: Locale): void {
  if (next === localeState.current) {
    return;
  }
  // 先经运行时写 localStorage 缓存，再更新响应式状态驱动重渲染
  setLocale(next, { reload: false });
  localeState.current = next;
}
