import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/editor-typography-popover.svelte";

// jsdom 缺少指针捕获与滚动 API，bits-ui 弹窗用得到，仅在本文件内就地补齐。
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

// 纯替身：快捷入口依赖的外部状态（编辑器排版、界面字体、系统字体）全部 mock，
// 只验证接线逻辑——弹层打开、当前取值渲染与切换调用。
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
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  editorTypographyStateMock.family = "system";
  editorTypographyStateMock.weight = 400;
  editorTypographyStateMock.size = 14;
  editorTypographyStateMock.indent = 2;
  fontStateMock.family = "system";
  getSystemFontsMock.mockResolvedValue([]);
});

describe("编辑器排版快捷入口", () => {
  // 弹层内容经浮动定位挂到 body，jsdom 零布局下整体 `visibility: hidden`，
  // role 查询不可靠，一律按 `aria-label` 直查（选项仍按既有用例的 `data-value` 定位）。
  function querySizeTrigger(): HTMLElement | null {
    return document.querySelector('[aria-label="Editor font size"]');
  }

  function queryWeightSlider(): HTMLElement | null {
    return document.querySelector('[aria-label="Editor font weight"]');
  }

  function queryIndentSlider(): HTMLElement | null {
    return document.querySelector('[aria-label="First-line indent"]');
  }

  /** 打开弹层并等字号触发器挂载：内容异步挂载。 */
  async function openPopover(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    await user.click(screen.getByRole("button", { name: "Typography settings" }));
    await waitFor(() => {
      expect(querySizeTrigger()).not.toBeNull();
    });
  }

  it("头部渲染排版按钮，点击展开当前排版取值", async () => {
    const user = userEvent.setup();
    render(Component);

    await openPopover(user);

    expect(querySizeTrigger()?.textContent).toContain("14px");
    expect(queryWeightSlider()?.getAttribute("aria-valuenow")).toBe("400");
    expect(queryIndentSlider()?.getAttribute("aria-valuenow")).toBe("2");
  });

  it("字号下拉切换调用 setEditorFontSize，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await openPopover(user);
    await user.click(querySizeTrigger() as HTMLElement);
    await waitFor(() => {
      expect(document.querySelector('[data-value="18"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="18"]') as HTMLElement);

    expect(setEditorFontSizeMock).toHaveBeenCalledWith(18);
  });

  it("字重滑块经键盘切换调用 setEditorFontWeight，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await openPopover(user);
    // 弹层 FocusScope 会干扰 user.keyboard 的焦点投递，改用 keydown 直达手柄；
    // 手柄落键同时触发 change 与 commit（各一次），断言只认预览分支那一次
    await fireEvent.keyDown(queryWeightSlider() as HTMLElement, { key: "ArrowRight" });

    expect(setEditorFontWeightMock).toHaveBeenCalledWith(500, false);
  });

  it("缩进滑块经键盘切换调用 setEditorIndent，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await openPopover(user);
    await fireEvent.keyDown(queryIndentSlider() as HTMLElement, { key: "ArrowRight" });

    expect(setEditorIndentMock).toHaveBeenCalledWith(3, false);
  });
});
