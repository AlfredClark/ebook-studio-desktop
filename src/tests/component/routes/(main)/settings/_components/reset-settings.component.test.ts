import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { Config_Serialize } from "#libs/commands/types";
import Component from "../../../../../../routes/(main)/settings/_components/reset-settings.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同设置页测试）。
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

// 纯替身：外观重置、颜色模式、后端配置、语言回落桥、运行时语言、提示全部 mock，
// 只验证重置的接线逻辑——确认链路、三方调用顺序与语言回落分支。
const resetAppearanceMock = vi.hoisted(() => vi.fn());
const setModeMock = vi.hoisted(() => vi.fn());
const resetConfigMock = vi.hoisted(() => vi.fn());
const setLocaleMock = vi.hoisted(() => vi.fn());
const alignLocaleMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
const configStateMock = vi.hoisted((): { value: Config_Serialize | null } => ({ value: null }));

const defaults: Config_Serialize = {
  locale: "en",
  auto_start: false,
  remember_window: false,
  auto_check_update: false,
  tray_enabled: true,
  close_behavior: "prompt",
  shortcut_show: "",
  schema_version: 1,
};

vi.mock("#libs/hooks/appearance.svelte", () => ({
  resetAppearance: resetAppearanceMock,
}));

vi.mock("mode-watcher", () => ({
  setMode: setModeMock,
}));

vi.mock("#libs/hooks/config.svelte", () => ({
  configState: configStateMock,
  resetConfig: resetConfigMock,
}));

vi.mock("#libs/i18n/paraglide/runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("#libs/i18n/paraglide/runtime")>();
  return {
    ...actual,
    getLocale: () => configStateMock.value?.locale ?? "en",
    setLocale: setLocaleMock,
  };
});

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

// 语言回落桥单独替身：验证重置把语言变更交给 alignLocale（内部 reload:false）而不是绕过它，
// 真实桥的不重载行为由 locale.test.ts 覆盖
vi.mock("#libs/i18n/locale.svelte", () => ({ alignLocale: alignLocaleMock }));

beforeEach(() => {
  vi.clearAllMocks();
  configStateMock.value = { ...defaults };
  // 成功路径：后端回填默认值，与真实 `resetConfig` 的回写行为一致
  resetConfigMock.mockImplementation(async () => {
    configStateMock.value = { ...defaults };
    return { ...defaults };
  });
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("恢复默认设置", () => {
  /** 打开确认弹窗并点确认：bits-ui 对话框经 portal 挂载，全局查询即可 */
  async function confirmReset(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    await user.click(screen.getByRole("button", { name: "Reset to defaults" }));
    await user.click(screen.getByRole("button", { name: "Reset" }));
  }

  it("渲染全宽 destructive 按钮", () => {
    render(Component);

    const button = screen.getByRole("button", { name: "Reset to defaults" });
    expect(button.className).toContain("w-full");
  });

  it("确认后依次重置前端与后端，语言回落交给不重载桥", async () => {
    const user = userEvent.setup();
    render(Component);

    await confirmReset(user);

    expect(resetAppearanceMock).toHaveBeenCalledOnce();
    expect(setModeMock).toHaveBeenCalledWith("system");
    expect(resetConfigMock).toHaveBeenCalledOnce();
    expect(toastMocks.success).toHaveBeenCalledOnce();
    expect(toastMocks.error).not.toHaveBeenCalled();
    // 语言变更一律交给 alignLocale；组件不得自己裸调 setLocale 绕过桥
    expect(alignLocaleMock).toHaveBeenCalledWith("en");
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("后端失败时报错且不走语言回落", async () => {
    resetConfigMock.mockResolvedValue(null);
    const user = userEvent.setup();
    render(Component);

    await confirmReset(user);

    // 前端重置照常执行，后端失败仅提示
    expect(resetAppearanceMock).toHaveBeenCalledOnce();
    expect(toastMocks.error).toHaveBeenCalledOnce();
    expect(toastMocks.success).not.toHaveBeenCalled();
    expect(alignLocaleMock).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("重置改变语言时经不重载桥回落，成功提示不被刷新吃掉", async () => {
    configStateMock.value = { ...defaults, locale: "zh-CN" };
    const user = userEvent.setup();
    render(Component);

    // locale 已切中文，文案跟随渲染中文，用中文名操作
    await user.click(screen.getByRole("button", { name: "恢复默认设置" }));
    await user.click(screen.getByRole("button", { name: "恢复" }));

    expect(alignLocaleMock).toHaveBeenCalledWith("en");
    // 走 alignLocale（内部 reload:false）而非整页重载，成功提示才留得下来
    expect(toastMocks.success).toHaveBeenCalledOnce();
  });

  it("当前语言非默认时，范围说明提前讲清语言后果", () => {
    configStateMock.value = { ...defaults, locale: "zh-CN" };
    render(Component);

    // 默认语言是 English（= paraglide baseLocale），重置会连带换语言，必须在点之前讲清
    expect(screen.getByText(/English/)).not.toBeNull();
  });

  it("当前语言已是默认时，范围说明不提语言后果", () => {
    render(Component);

    expect(screen.queryByText(/English/)).toBeNull();
  });

  it("取消时不执行任何重置", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Reset to defaults" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(resetAppearanceMock).not.toHaveBeenCalled();
    expect(resetConfigMock).not.toHaveBeenCalled();
    expect(toastMocks.success).not.toHaveBeenCalled();
  });
});
