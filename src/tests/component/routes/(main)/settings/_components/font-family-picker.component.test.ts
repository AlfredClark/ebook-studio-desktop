import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/settings/_components/font-family-picker.svelte";

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

// jsdom 无 IntersectionObserver，懒加载哨兵用得到，回调由用例手动触发。
let intersectionObserverCallback: IntersectionObserverCallback | null = null;
if (typeof IntersectionObserver === "undefined") {
  globalThis.IntersectionObserver = class {
    constructor(callback: IntersectionObserverCallback) {
      intersectionObserverCallback = callback;
    }
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof IntersectionObserver;
}

/** 手动触发哨兵相交，模拟列表触底追加。 */
function fireIntersecting(): void {
  intersectionObserverCallback?.(
    [{ isIntersecting: true } as IntersectionObserverEntry],
    {} as IntersectionObserver,
  );
}

// 纯替身：选择器依赖的外部状态（外观、系统字体）全部 mock，
// 只验证接线逻辑——候选渲染、搜索过滤、懒加载、切换调用与失败分支。
const setFontFamilyMock = vi.hoisted(() => vi.fn());
const fontStateMock = vi.hoisted(() => ({ family: "system" }));
const getSystemFontsMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/hooks/appearance.svelte", () => ({
  DEFAULT_FONT_FAMILY: "system",
  fontState: fontStateMock,
  buildFontStack: (family: string) =>
    family === "system" ? "default-stack" : `"${family}", default-stack`,
  sanitizeFontFamily: (value: string) => value.trim(),
  setFontFamily: setFontFamilyMock,
}));

vi.mock("tauri-plugin-system-fonts-api", () => ({
  getSystemFonts: getSystemFontsMock,
}));

afterEach(() => {
  cleanup();
  // bits-ui 弹窗打开时给 body 加滚动锁定样式，jsdom 内卸载后不会自动还原，
  // 这里手动清理，避免泄漏到后续用例导致点击被 pointer-events 拦截。
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  intersectionObserverCallback = null;
  fontStateMock.family = "system";
  // 默认返回三个字体族，与真实插件的变体数组形状一致
  getSystemFontsMock.mockResolvedValue([
    { fontName: "Beta", name: "Beta" },
    { fontName: "Alpha", name: "Alpha" },
    { fontName: "Gamma Delta", name: "Gamma Delta" },
  ]);
});

describe("字体选择器", () => {
  /** 打开字体弹窗并等待首个选项：拉取发生在首次打开时。 */
  async function openFontPopover(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    await user.click(screen.getByRole("combobox", { name: "Interface font" }));
    await waitFontOption("Alpha");
  }

  /** 按文本查找字体选项：选项内联的勾选图标无 `aria-hidden`，可访问名计算不可靠，按文本匹配。 */
  function queryFontOption(name: string): HTMLElement | null {
    return (
      (screen
        .queryAllByRole("option", { hidden: true })
        .find((el) => el.textContent?.trim() === name) as HTMLElement | undefined) ?? null
    );
  }

  /** 断言字体选项出现（弹窗内容异步挂载）。 */
  async function waitFontOption(name: string): Promise<HTMLElement> {
    await waitFor(() => {
      expect(queryFontOption(name)).not.toBeNull();
    });
    return queryFontOption(name) as HTMLElement;
  }

  it("渲染字体行及当前取值", async () => {
    const user = userEvent.setup();
    render(Component);

    expect(screen.getByText("Interface font")).not.toBeNull();
    // 触发器挂载即用，候选在首次打开时加载
    await openFontPopover(user);
    expect(screen.getByRole("combobox", { name: "Interface font" }).textContent).toContain(
      "System default",
    );
  });

  it("挂载时不拉取字体，首次打开拉取一次", async () => {
    const user = userEvent.setup();
    render(Component);

    expect(getSystemFontsMock).not.toHaveBeenCalled();
    await openFontPopover(user);
    expect(getSystemFontsMock).toHaveBeenCalledTimes(1);
    // 关闭重开不重复拉取
    await user.click(screen.getByRole("combobox", { name: "Interface font" }));
    await openFontPopover(user);
    expect(getSystemFontsMock).toHaveBeenCalledTimes(1);
  });

  it("字体搜索过滤并切换调用 setFontFamily，不经过后端", async () => {
    const user = userEvent.setup();
    render(Component);

    await openFontPopover(user);
    // 弹窗内容异步挂载，选项就绪后再点选
    await user.click(await waitFontOption("Alpha"));

    expect(setFontFamilyMock).toHaveBeenCalledWith("Alpha");
  });

  it("字体搜索按子串过滤候选", async () => {
    const user = userEvent.setup();
    render(Component);

    await openFontPopover(user);
    const searchbox = await screen.findByPlaceholderText("Search fonts…");
    // jsdom 内逐字键入首字符后焦点漂移（生产环境无此问题），单次 input 事件直达过滤逻辑
    await fireEvent.input(searchbox, { target: { value: "alp" } });

    await waitFor(() => {
      expect(queryFontOption("Beta")).toBeNull();
      expect(queryFontOption("Alpha")).not.toBeNull();
    });
  });

  it("字体搜索忽略空格匹配", async () => {
    const user = userEvent.setup();
    render(Component);

    await openFontPopover(user);
    const searchbox = await screen.findByPlaceholderText("Search fonts…");
    // 关键词与族名的空格差异不影响匹配
    await fireEvent.input(searchbox, { target: { value: "gammadelta" } });

    await waitFor(() => {
      expect(queryFontOption("Gamma Delta")).not.toBeNull();
      expect(queryFontOption("Alpha")).toBeNull();
    });
    await fireEvent.input(searchbox, { target: { value: "ga mma" } });

    await waitFor(() => {
      expect(queryFontOption("Gamma Delta")).not.toBeNull();
    });
  });

  it("字体列表懒加载：首屏封顶，触底追加", async () => {
    const user = userEvent.setup();
    getSystemFontsMock.mockResolvedValueOnce(
      Array.from({ length: 200 }, (_, index) => ({
        fontName: `Font${index}`,
        name: `Font${index}`,
      })),
    );
    render(Component);

    await user.click(screen.getByRole("combobox", { name: "Interface font" }));
    // 系统默认 + 首屏 80，弹窗内容异步挂载
    // jsdom 零布局下浮动定位给弹窗加 `visibility: hidden`（真机定位后移除），查询放宽隐藏过滤
    await waitFor(() => {
      expect(screen.getAllByRole("option", { hidden: true }).length).toBe(81);
    });
    fireIntersecting();
    await waitFor(() => {
      expect(screen.getAllByRole("option", { hidden: true }).length).toBe(161);
    });
  });

  it("字体下拉渲染持久化的当前取值", async () => {
    fontStateMock.family = "Beta";
    render(Component);

    expect(screen.getByRole("combobox", { name: "Interface font" }).textContent).toContain("Beta");
  });

  it("插件不可用时字体下拉禁用并提示仅桌面端可用", async () => {
    getSystemFontsMock.mockRejectedValueOnce(new Error("no tauri runtime"));
    render(Component);
    const user = userEvent.setup();
    // 拉取发生在首次打开，失败后禁用并提示
    await user.click(screen.getByRole("combobox", { name: "Interface font" }));

    await waitFor(() => {
      expect(
        screen.getByRole("combobox", { name: "Interface font" }).hasAttribute("disabled"),
      ).toBe(true);
    });
    expect(screen.getByText("Desktop only")).not.toBeNull();
    expect(setFontFamilyMock).not.toHaveBeenCalled();
  });
});
