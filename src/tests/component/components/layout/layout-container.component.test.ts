import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { createRawSnippet } from "svelte";
import { layoutState } from "#libs/hooks/appearance.svelte";
import {
  DASHBOARD_COLLAPSED_STORAGE_KEY,
  DEFAULT_DASHBOARD_COLLAPSED,
  dashboardState,
} from "#libs/hooks/dashboard.svelte";
import LayoutContainer from "../../../../components/layout/layout-container.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同关于页测试）。
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
// jsdom 无 matchMedia，侧边栏折叠断点（IsMobile）就地给假，默认桌面宽度不断点
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

// 标题栏窗口操作在单测无 Tauri 运行时，全部按不可用回落
vi.mock("#libs/utils/window-controls", () => ({
  isWindowControlsAvailable: vi.fn().mockResolvedValue(false),
  isWindowAlwaysOnTop: vi.fn().mockResolvedValue(false),
  toggleAlwaysOnTopWindow: vi.fn().mockResolvedValue(false),
  minimizeWindow: vi.fn().mockResolvedValue(false),
  toggleMaximizeWindow: vi.fn().mockResolvedValue(false),
  closeWindow: vi.fn().mockResolvedValue(false),
  isWindowMaximized: vi.fn().mockResolvedValue(false),
  onWindowResized: vi.fn().mockResolvedValue(null),
}));

// 导航标签栏依赖 SvelteKit 运行时状态，就地提供最小桩
vi.mock("$app/state", () => ({
  page: { url: new URL("http://localhost/") },
}));

vi.mock("$app/navigation", () => ({
  goto: vi.fn(),
}));

vi.mock("$app/paths", () => ({
  resolve: (path: string): string => path,
}));

function renderWithProbe(layout: string): Promise<void> {
  const children = createRawSnippet(() => ({
    render: () => "<span>probe-content</span>",
  }));
  render(LayoutContainer, { props: { children } });
  // 布局经异步加载器 settle（动态 import 需多轮微任务），等目标布局落定；
  // 全量套件并行跑时动态 import 会被抢 CPU，默认 1s 会假失败，故放宽到 5s
  return waitFor(
    () => {
      expect(document.querySelector(`[data-layout="${layout}"]`)).not.toBeNull();
    },
    { timeout: 5000 },
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  // 容器挂载会从持久化重读，用例间先清空，单个用例按需预种
  localStorage.clear();
  layoutState.name = "tabs";
  dashboardState.collapsed = DEFAULT_DASHBOARD_COLLAPSED;
});

afterEach(() => {
  cleanup();
  layoutState.name = "tabs";
  dashboardState.collapsed = DEFAULT_DASHBOARD_COLLAPSED;
});

describe("布局容器", () => {
  it("tabs 布局渲染标签栏骨架与子内容", async () => {
    layoutState.name = "tabs";
    await renderWithProbe("tabs");
    expect(screen.getByText("probe-content")).not.toBeNull();
  });

  it("sidebar 布局渲染侧边栏骨架与子内容", async () => {
    // 容器挂载时 initLayout 会从持久化重读，先写存储再预设内存，两者一致才稳定
    localStorage.setItem("layout-name", "sidebar");
    layoutState.name = "sidebar";
    await renderWithProbe("sidebar");
    expect(screen.getByText("probe-content")).not.toBeNull();
  });

  it("dashboard 布局渲染固定侧栏骨架与子内容", async () => {
    localStorage.setItem("layout-name", "dashboard");
    layoutState.name = "dashboard";
    await renderWithProbe("dashboard");
    expect(screen.getByText("probe-content")).not.toBeNull();
  });

  it("切换布局时旧布局保持到新布局加载完成", async () => {
    layoutState.name = "tabs";
    await renderWithProbe("tabs");

    layoutState.name = "sidebar";
    await waitFor(() => {
      expect(document.querySelector('[data-layout="sidebar"]')).not.toBeNull();
    });
    expect(screen.getByText("probe-content")).not.toBeNull();
  });

  it("dashboard 侧栏默认展开，把手切换并落盘", async () => {
    const user = userEvent.setup();
    localStorage.setItem("layout-name", "dashboard");
    layoutState.name = "dashboard";
    await renderWithProbe("dashboard");

    const aside = document.querySelector("aside");
    // 无存量偏好：默认展开，导航标签文本可见
    expect(aside?.querySelectorAll("span").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Collapse sidebar" }));
    // 收起态仅剩图标，标签走悬浮 tooltip；偏好随手同步落盘
    expect(aside?.querySelectorAll("span").length).toBe(0);
    expect(localStorage.getItem(DASHBOARD_COLLAPSED_STORAGE_KEY)).toBe("true");

    await user.click(screen.getByRole("button", { name: "Expand sidebar" }));
    expect(aside?.querySelectorAll("span").length).toBeGreaterThan(0);
    expect(localStorage.getItem(DASHBOARD_COLLAPSED_STORAGE_KEY)).toBe("false");
  });

  it("dashboard 侧栏按存量偏好恢复收起态", async () => {
    // 内存态已被 beforeEach 重置为展开，只有存量是收起，才能证明挂载时确实读了存储
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "true");
    localStorage.setItem("layout-name", "dashboard");
    layoutState.name = "dashboard";
    await renderWithProbe("dashboard");

    await waitFor(() => {
      expect(document.querySelector("aside")?.querySelectorAll("span").length).toBe(0);
    });
    expect(screen.getByRole("button", { name: "Expand sidebar" })).not.toBeNull();
  });

  it("dashboard 侧栏折叠偏好跨布局切换保留", async () => {
    const user = userEvent.setup();
    localStorage.setItem("layout-name", "dashboard");
    layoutState.name = "dashboard";
    await renderWithProbe("dashboard");

    await user.click(screen.getByRole("button", { name: "Collapse sidebar" }));

    layoutState.name = "tabs";
    await waitFor(() => {
      expect(document.querySelector('[data-layout="tabs"]')).not.toBeNull();
    });

    layoutState.name = "dashboard";
    await waitFor(() => {
      expect(document.querySelector('[data-layout="dashboard"]')).not.toBeNull();
    });
    // 重挂载后按持久化值恢复收起，而非回到默认展开
    await waitFor(() => {
      expect(document.querySelector("aside")?.querySelectorAll("span").length).toBe(0);
    });
  });

  it("注册表缺 key 时回落 tabs 而非白屏", async () => {
    layoutState.name = "tabs";
    await renderWithProbe("tabs");

    // 脏数据在 hooks 层已回落，此处模拟注册表缺 key 的极端情形
    layoutState.name = "ghost" as never;
    await waitFor(() => {
      expect(document.querySelector('[data-layout="tabs"]')).not.toBeNull();
    });
    expect(screen.getByText("probe-content")).not.toBeNull();
  });
});
