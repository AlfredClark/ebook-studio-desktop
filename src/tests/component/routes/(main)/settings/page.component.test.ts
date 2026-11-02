import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { Config_Serialize } from "#libs/commands/types";
import Page from "../../../../../routes/(main)/settings/+page.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同通用设置分组测试）。
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

// 纯替身：三个设置分组依赖的外部状态（后端配置、运行时语言、外观、主题、系统字体、提示）全部 mock，
// 只验证设置页的组装——标题、通用/外观两分组、重置入口与开关接线。
const updateConfigMock = vi.hoisted(() => vi.fn());
const resetConfigMock = vi.hoisted(() => vi.fn());
const setLocaleMock = vi.hoisted(() => vi.fn());
const setModeMock = vi.hoisted(() => vi.fn());
const resetAppearanceMock = vi.hoisted(() => vi.fn());
const setLayoutNameMock = vi.hoisted(() => vi.fn());
const setColorThemeMock = vi.hoisted(() => vi.fn());
const setFontFamilyMock = vi.hoisted(() => vi.fn());
const setFontWeightMock = vi.hoisted(() => vi.fn());
const setFontSizeMock = vi.hoisted(() => vi.fn());
const getSystemFontsMock = vi.hoisted(() => vi.fn());
const layoutStateMock = vi.hoisted(() => ({ name: "tabs" }));
const colorThemeStateMock = vi.hoisted(() => ({ name: "neutral" }));
const fontStateMock = vi.hoisted(() => ({ family: "system", weight: 400, size: 100 }));
const toastMocks = vi.hoisted(() => ({
  loading: vi.fn(() => 1),
  dismiss: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

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

const configStateMock = vi.hoisted((): { value: Config_Serialize | null } => ({ value: null }));

vi.mock("#libs/hooks/config.svelte", () => ({
  configState: configStateMock,
  updateConfig: updateConfigMock,
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

vi.mock("mode-watcher", () => ({
  userPrefersMode: { current: "system" },
  setMode: setModeMock,
}));

vi.mock("#libs/hooks/appearance.svelte", () => ({
  layoutState: layoutStateMock,
  setLayoutName: setLayoutNameMock,
  colorThemeState: colorThemeStateMock,
  setColorTheme: setColorThemeMock,
  DEFAULT_FONT_FAMILY: "system",
  MIN_FONT_WEIGHT: 100,
  MAX_FONT_WEIGHT: 900,
  FONT_WEIGHT_STEP: 100,
  FONT_SIZE_OPTIONS: [75, 80, 85, 90, 95, 100, 105, 110, 115, 120, 125],
  fontState: fontStateMock,
  buildFontStack: (family: string) =>
    family === "system" ? "default-stack" : `"${family}", default-stack`,
  sanitizeFontFamily: (value: string) => value.trim(),
  setFontFamily: setFontFamilyMock,
  setFontWeight: setFontWeightMock,
  setFontSize: setFontSizeMock,
  resetAppearance: resetAppearanceMock,
}));

vi.mock("tauri-plugin-system-fonts-api", () => ({
  getSystemFonts: getSystemFontsMock,
}));

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
  configStateMock.value = { ...defaults };
  layoutStateMock.name = "tabs";
  colorThemeStateMock.name = "neutral";
  fontStateMock.family = "system";
  fontStateMock.weight = 400;
  fontStateMock.size = 100;
  getSystemFontsMock.mockResolvedValue([]);
  updateConfigMock.mockResolvedValue(true);
  resetConfigMock.mockResolvedValue({ ...defaults });
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("设置页", () => {
  it("渲染页标题与通用外观快捷键三分组及重置入口", () => {
    render(Page);

    expect(screen.getByText("Settings")).not.toBeNull();
    expect(screen.getByText("General")).not.toBeNull();
    expect(screen.getByText("Appearance")).not.toBeNull();
    expect(screen.getByText("Shortcuts")).not.toBeNull();
    expect(screen.getByText("Language")).not.toBeNull();
    expect(screen.getByText("Show app")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Reset to defaults" })).not.toBeNull();
  });

  it("开关渲染后端配置的当前态", () => {
    render(Page);

    expect(screen.getByRole("switch", { name: "Autostart" }).getAttribute("data-state")).toBe(
      "unchecked",
    );
    expect(screen.getByRole("switch", { name: "System tray" }).getAttribute("data-state")).toBe(
      "checked",
    );
  });

  it("打开自启开关经命令落盘，成功后提示", async () => {
    const user = userEvent.setup();
    render(Page);

    await user.click(screen.getByRole("switch", { name: "Autostart" }));

    expect(updateConfigMock).toHaveBeenCalledWith({ auto_start: true });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("语言下拉展示当前语言", () => {
    render(Page);

    const trigger = screen.getByRole("combobox", { name: "Language" });
    expect(trigger.textContent).toContain("English");
  });
});
