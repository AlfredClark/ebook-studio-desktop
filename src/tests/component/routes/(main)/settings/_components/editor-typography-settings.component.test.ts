import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/settings/_components/editor-typography-settings.svelte";

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

// 纯替身：分组依赖的外部状态（编辑器排版、界面字体、系统字体）全部 mock，
// 只验证分组的接线逻辑——选项渲染、切换调用与成功/失败分支。
const setEditorFontFamilyMock = vi.hoisted(() => vi.fn());
const setEditorFontWeightMock = vi.hoisted(() => vi.fn());
const setEditorFontSizeMock = vi.hoisted(() => vi.fn());
const setEditorIndentMock = vi.hoisted(() => vi.fn());
const editorTypographyStateMock = vi.hoisted(() => ({
  family: "system",
  weight: 400,
  size: 14,
  indent: 2,
}));
const setFontFamilyMock = vi.hoisted(() => vi.fn());
const fontStateMock = vi.hoisted(() => ({ family: "system" }));
const getSystemFontsMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/hooks/appearance.svelte", () => ({
  DEFAULT_FONT_FAMILY: "system",
  MIN_FONT_WEIGHT: 100,
  MAX_FONT_WEIGHT: 900,
  FONT_WEIGHT_STEP: 100,
  EDITOR_FONT_SIZE_OPTIONS: [12, 13, 14, 15, 16, 18, 20, 22, 24],
  EDITOR_INDENT_STEP: 1,
  MAX_EDITOR_INDENT: 4,
  MIN_EDITOR_INDENT: 0,
  editorTypographyState: editorTypographyStateMock,
  fontState: fontStateMock,
  buildFontStack: (family: string) =>
    family === "system" ? "default-stack" : `"${family}", default-stack`,
  sanitizeFontFamily: (value: string) => value.trim(),
  setFontFamily: setFontFamilyMock,
  setEditorFontFamily: setEditorFontFamilyMock,
  setEditorFontWeight: setEditorFontWeightMock,
  setEditorFontSize: setEditorFontSizeMock,
  setEditorIndent: setEditorIndentMock,
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
  editorTypographyStateMock.family = "system";
  editorTypographyStateMock.weight = 400;
  editorTypographyStateMock.size = 14;
  editorTypographyStateMock.indent = 2;
  fontStateMock.family = "system";
  // 内嵌的字体选择器同样走插件，默认给空列表使其保持禁用，不干扰本组用例
  getSystemFontsMock.mockResolvedValue([]);
});

describe("编辑器排版设置分组", () => {
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

  it("渲染编辑器排版分组及当前取值", () => {
    render(Component);

    expect(screen.getByText("Editor typography")).not.toBeNull();
    expect(screen.getByText("Editor font weight")).not.toBeNull();
    expect(screen.getByText("Editor font size")).not.toBeNull();
    expect(screen.getByText("First-line indent")).not.toBeNull();
    const fontSizeTrigger = screen.getByRole("combobox", { name: "Editor font size" });
    expect(fontSizeTrigger.textContent).toContain("14px");
  });

  it("字重滑块渲染当前取值并经键盘切换调用 setEditorFontWeight，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    const slider = screen.getByRole("slider", { name: "Editor font weight" });
    expect(slider.getAttribute("aria-valuenow")).toBe("400");
    // jsdom 无布局，click 会按零尺寸命中最小值，改用聚焦后按键
    (slider as HTMLElement).focus();
    await user.keyboard("{ArrowRight}");

    expect(setEditorFontWeightMock).toHaveBeenCalledWith(500, false);
  });

  it("字号下拉切换调用 setEditorFontSize，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Editor font size", "18");

    expect(setEditorFontSizeMock).toHaveBeenCalledWith(18);
  });

  it("字号下拉渲染持久化的当前取值", () => {
    editorTypographyStateMock.size = 20;
    render(Component);

    expect(screen.getByRole("combobox", { name: "Editor font size" }).textContent).toContain(
      "20px",
    );
  });

  it("缩进滑块渲染当前取值并经键盘切换调用 setEditorIndent，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    const slider = screen.getByRole("slider", { name: "First-line indent" });
    expect(slider.getAttribute("aria-valuenow")).toBe("2");
    (slider as HTMLElement).focus();
    await user.keyboard("{ArrowRight}");

    expect(setEditorIndentMock).toHaveBeenCalledWith(3, false);
  });

  it("缩进滑块渲染持久化的当前取值", () => {
    editorTypographyStateMock.indent = 0;
    render(Component);

    expect(
      screen.getByRole("slider", { name: "First-line indent" }).getAttribute("aria-valuenow"),
    ).toBe("0");
  });
});
