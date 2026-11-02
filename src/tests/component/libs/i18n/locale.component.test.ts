import { cleanup, render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { alignLocale } from "#libs/i18n/locale.svelte";
import LocaleProbe from "../../stubs/locale-probe.svelte";

// 语言响应式桥回归：启动水合对齐（reload: false）后，已渲染文案必须原地重渲染。
// 背景：Paraglide 的 getLocale 是普通函数，首帧渲染的文案不会随语言对齐更新——
// 首次启动会残留 baseLocale（en）文案直到手动刷新，本桥把 getLocale 覆盖为响应式读取。
beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  cleanup();
});

describe("语言响应式桥", () => {
  it("alignLocale 后已渲染文案原地重渲染，并写入 localStorage 缓存", async () => {
    render(LocaleProbe);
    expect(screen.getByTestId("probe").textContent).toBe("Home");

    alignLocale("zh-CN");
    await tick();

    expect(screen.getByTestId("probe").textContent).toBe("首页");
    // 缓存同步写入：下次冷启动首帧即按缓存解析，避免先英文后对齐
    expect(localStorage.getItem("PARAGLIDE_LOCALE")).toBe("zh-CN");
  });
});
