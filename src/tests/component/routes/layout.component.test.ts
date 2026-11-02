import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { tick } from "svelte";
import { configState, hydrateConfig } from "#libs/hooks/config.svelte";
import { reportCommandFailure } from "#libs/commands/cores";
import type { Config_Serialize } from "#libs/commands/types";
import { alignLocale } from "#libs/i18n/locale.svelte";
import { getLocale } from "#libs/i18n/paraglide/runtime";
import Harness from "../stubs/root-layout-harness.svelte";

// 根布局回归：外观初始化只跑一次，未落盘的预览不得被重初始化覆盖。
// 背景：initAppearance 读写同一批外观状态，若被包进跟踪型 effect，
// 每次改动都会触发“从磁盘全量重读”，字重拖拽预览会被存量打回导致滑块冻住。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}
if (typeof window.matchMedia === "undefined") {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

vi.mock("mode-watcher", async (importOriginal) => {
  const Empty = (await import("../stubs/empty.svelte")).default;
  return {
    ...(await importOriginal<typeof import("mode-watcher")>()),
    ModeWatcher: Empty,
    userPrefersMode: { current: "system" },
    setMode: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ onCloseRequested: async () => async () => {} }),
}));

vi.mock("#libs/hooks/config.svelte", () => ({
  configState: { value: null },
  hydrateConfig: vi.fn(async () => {}),
}));

vi.mock("#libs/hooks/deep-link.svelte", () => ({
  initDeepLinks: vi.fn(async () => async () => {}),
}));

vi.mock("#libs/hooks/updater.svelte", () => ({
  maybeAutoCheckForUpdate: vi.fn(),
}));

vi.mock("#libs/commands", () => ({
  default: { quitApp: () => ({ failed: () => {} }) },
}));

vi.mock("#libs/commands/cores", () => ({
  reportCommandFailure: vi.fn(),
}));

vi.mock("#libs/i18n/locale.svelte", () => ({
  alignLocale: vi.fn(),
}));

vi.mock("#libs/i18n/paraglide/runtime", () => ({
  getLocale: vi.fn(),
}));

/** 等挂载期后台水合走完：$effect → hydrateConfig → 对齐语言，全是跨微任务的异步链 */
async function flushHydrate(): Promise<void> {
  for (let i = 0; i < 5; i += 1) {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  configState.value = null;
  vi.mocked(hydrateConfig).mockResolvedValue(undefined);
  vi.mocked(getLocale).mockReturnValue("en");
});

describe("根布局外观对齐", () => {
  it("首帧按存量对齐字重", async () => {
    localStorage.setItem("font-weight", "700");
    render(Harness);

    expect(screen.getByTestId("weight").textContent).toBe("700");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("700");
  });

  it("未落盘的字重预览不被重初始化覆盖", async () => {
    const user = userEvent.setup();
    render(Harness);
    expect(screen.getByTestId("weight").textContent).toBe("400");

    await user.click(screen.getByRole("button", { name: "preview-500" }));
    expect(screen.getByTestId("weight").textContent).toBe("500");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("500");
    // 预览语义：内存与样式即时生效，磁盘保持不动
    expect(localStorage.getItem("font-weight")).toBeNull();
    // 等 effect 全部 flush：若初始化被重复订阅，此处会被存量打回 400
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.getByTestId("weight").textContent).toBe("500");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("500");
  });
});

describe("根布局启动水合", () => {
  const saved: Config_Serialize = {
    locale: "zh-CN",
    auto_start: true,
    remember_window: true,
    auto_check_update: true,
    tray_enabled: true,
    close_behavior: "prompt",
    shortcut_show: "",
    schema_version: 1,
  };

  it("水合成功且语言不一致时对齐且不重载", async () => {
    configState.value = saved;
    render(Harness);
    await flushHydrate();

    expect(hydrateConfig).toHaveBeenCalledOnce();
    expect(alignLocale).toHaveBeenCalledWith("zh-CN");
  });

  it("语言已一致时不调用对齐", async () => {
    configState.value = { ...saved, locale: "en" };
    render(Harness);
    await flushHydrate();

    expect(hydrateConfig).toHaveBeenCalledOnce();
    expect(alignLocale).not.toHaveBeenCalled();
  });

  it("水合构造期抛错时上报且不阻断首帧", async () => {
    vi.mocked(hydrateConfig).mockRejectedValueOnce(new Error("boom"));
    render(Harness);
    await flushHydrate();

    expect(reportCommandFailure).toHaveBeenCalledWith("[config] hydrate threw", expect.anything());
    expect(alignLocale).not.toHaveBeenCalled();
    // 首帧照常渲染，不白屏（getBy* 找不到会直接抛错，本身即断言）
    expect(screen.getByTestId("weight").textContent).not.toBeNull();
  });
});
