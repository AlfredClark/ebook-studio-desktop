import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/settings/_components/appearance-settings.svelte";

// jsdom 缺少指针捕获与滚动 API，bits-ui 下拉用得到，仅在本文件内就地补齐。
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

// 纯替身：分组依赖的外部状态（主题、外观、系统字体）全部 mock，
// 只验证分组的接线逻辑——选项渲染、切换调用与成功/失败分支。
const setModeMock = vi.hoisted(() => vi.fn());
const setLayoutNameMock = vi.hoisted(() => vi.fn());
const layoutStateMock = vi.hoisted(() => ({ name: "tabs" }));
const setColorThemeMock = vi.hoisted(() => vi.fn());
const colorThemeStateMock = vi.hoisted(() => ({ name: "neutral" }));
const setFontFamilyMock = vi.hoisted(() => vi.fn());
const setFontWeightMock = vi.hoisted(() => vi.fn());
const setFontSizeMock = vi.hoisted(() => vi.fn());
const fontStateMock = vi.hoisted(() => ({ family: "system", weight: 400, size: 100 }));
const getSystemFontsMock = vi.hoisted(() => vi.fn());

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
}));

vi.mock("tauri-plugin-system-fonts-api", () => ({
  getSystemFonts: getSystemFontsMock,
}));

afterEach(() => {
  cleanup();
  // bits-ui 下拉打开时给 body 加滚动锁定样式，jsdom 内卸载后不会自动还原，
  // 这里手动清理，避免泄漏到后续用例导致点击被 pointer-events 拦截。
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  layoutStateMock.name = "tabs";
  colorThemeStateMock.name = "neutral";
  fontStateMock.family = "system";
  fontStateMock.weight = 400;
  fontStateMock.size = 100;
  // 内嵌的字体选择器同样走插件，默认给空列表使其保持禁用，不干扰本组用例
  getSystemFontsMock.mockResolvedValue([]);
});

describe("外观设置分组", () => {
  /** 在下拉中按值点选：jsdom 无布局，选项始终不可见，只能按 `data-value` 定位。 */
  async function chooseOption(
    user: ReturnType<typeof userEvent.setup>,
    triggerName: string,
    value: string,
  ): Promise<void> {
    await user.click(screen.getByRole("combobox", { name: triggerName }));
    await waitFor(() => {
      expect(document.querySelector(`[data-value="${value}"]`)).not.toBeNull();
    });
    await user.click(document.querySelector(`[data-value="${value}"]`) as HTMLElement);
  }

  it("渲染外观分组及当前取值", () => {
    render(Component);

    expect(screen.getByText("Appearance")).not.toBeNull();
    expect(screen.getByText("Theme")).not.toBeNull();
    expect(screen.getByText("Color theme")).not.toBeNull();
    expect(screen.getByText("Layout")).not.toBeNull();
    expect(screen.getByText("Font weight")).not.toBeNull();
    expect(screen.getByText("Font size")).not.toBeNull();
    // 下拉触发器展示当前选中项的文案
    const themeTrigger = screen.getByRole("combobox", { name: "Theme" });
    const colorThemeTrigger = screen.getByRole("combobox", { name: "Color theme" });
    const layoutTrigger = screen.getByRole("combobox", { name: "Layout" });
    const fontSizeTrigger = screen.getByRole("combobox", { name: "Font size" });
    expect(themeTrigger.textContent).toContain("System");
    expect(colorThemeTrigger.textContent).toContain("Default");
    expect(layoutTrigger.textContent).toContain("Tabs");
    expect(fontSizeTrigger.textContent).toContain("100%");
  });

  it("切换布局调用 setLayoutName，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Layout", "sidebar");

    expect(setLayoutNameMock).toHaveBeenCalledWith("sidebar");
    expect(setModeMock).not.toHaveBeenCalled();
  });

  it("切换到 dashboard 布局同样调用 setLayoutName", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Layout", "dashboard");

    expect(setLayoutNameMock).toHaveBeenCalledWith("dashboard");
    expect(setModeMock).not.toHaveBeenCalled();
  });

  it("布局下拉渲染持久化的当前取值", () => {
    layoutStateMock.name = "sidebar";
    render(Component);

    expect(screen.getByRole("combobox", { name: "Layout" }).textContent).toContain("Sidebar");
  });

  it("切换配色调用 setColorTheme，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Color theme", "ocean");

    expect(setColorThemeMock).toHaveBeenCalledWith("ocean");
    expect(setModeMock).not.toHaveBeenCalled();
  });

  it("配色下拉渲染持久化的当前取值", () => {
    colorThemeStateMock.name = "rose";
    render(Component);

    expect(screen.getByRole("combobox", { name: "Color theme" }).textContent).toContain("Rose");
  });

  it("切换主题即时调用 setMode，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Theme", "dark");

    expect(setModeMock).toHaveBeenCalledWith("dark");
  });

  it("字重滑块渲染当前取值并经键盘切换调用 setFontWeight，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    const slider = screen.getByRole("slider", { name: "Font weight" });
    expect(slider.getAttribute("aria-valuenow")).toBe("400");
    // jsdom 无布局，click 会按零尺寸命中最小值，改用聚焦后按键
    (slider as HTMLElement).focus();
    await user.keyboard("{ArrowRight}");

    expect(setFontWeightMock).toHaveBeenCalledWith(500, false);
  });

  it("字号下拉切换调用 setFontSize，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Font size", "105");

    expect(setFontSizeMock).toHaveBeenCalledWith(105);
    expect(setModeMock).not.toHaveBeenCalled();
  });

  it("字号下拉渲染持久化的当前取值", () => {
    fontStateMock.size = 110;
    render(Component);

    expect(screen.getByRole("combobox", { name: "Font size" }).textContent).toContain("110%");
  });
});
