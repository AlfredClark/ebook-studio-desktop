import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { ClassValue } from "clsx";

/**
 * 合并类名（基于 `clsx` 与 `tailwind-merge`）
 *
 * @param inputs 类名列表
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** 组件 props 中剔除 `child` 简写（svelte 5 的 snippet 写法） */
export type WithoutChild<T> = T extends { child?: unknown } ? Omit<T, "child"> : T;
/** 组件 props 中剔除 `children` */
export type WithoutChildren<T> = T extends { children?: unknown } ? Omit<T, "children"> : T;
/** 同时剔除 `child` 与 `children` */
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
/** 给组件 props 追加 `ref` 绑定（默认 `HTMLElement`） */
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
