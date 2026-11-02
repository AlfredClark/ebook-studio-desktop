import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  DASHBOARD_COLLAPSED_STORAGE_KEY,
  DEFAULT_DASHBOARD_COLLAPSED,
  dashboardState,
  initDashboard,
  loadDashboardCollapsed,
  setDashboardCollapsed,
} from "#libs/hooks/dashboard.svelte";

// 节点环境无 localStorage，用内存实现替身，保证单测不依赖浏览器
function installLocalStorageMock() {
  const store = new Map<string, string>();
  const mock = {
    getItem: (key: string): string | null => store.get(key) ?? null,
    setItem: (key: string, value: string): void => {
      store.set(key, String(value));
    },
    removeItem: (key: string): void => {
      store.delete(key);
    },
    clear: (): void => {
      store.clear();
    },
  };
  vi.stubGlobal("localStorage", mock);
  return mock;
}

beforeEach(() => {
  installLocalStorageMock();
  dashboardState.collapsed = DEFAULT_DASHBOARD_COLLAPSED;
  vi.restoreAllMocks();
});

describe("loadDashboardCollapsed", () => {
  it("缺失时回落默认展开", () => {
    expect(loadDashboardCollapsed()).toBe(DEFAULT_DASHBOARD_COLLAPSED);
  });

  it("读取已持久化的收起状态", () => {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "true");
    expect(loadDashboardCollapsed()).toBe(true);
  });

  it("读取已持久化的展开状态", () => {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "false");
    expect(loadDashboardCollapsed()).toBe(false);
  });

  it("脏数据回落默认值", () => {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "1");
    expect(loadDashboardCollapsed()).toBe(DEFAULT_DASHBOARD_COLLAPSED);
  });

  it("读取异常回落默认值", () => {
    const mock = installLocalStorageMock();
    vi.spyOn(mock, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(loadDashboardCollapsed()).toBe(DEFAULT_DASHBOARD_COLLAPSED);
  });

  it("无存储环境回落默认值", () => {
    vi.stubGlobal("localStorage", undefined);
    expect(loadDashboardCollapsed()).toBe(DEFAULT_DASHBOARD_COLLAPSED);
  });
});

describe("setDashboardCollapsed", () => {
  it("先落盘再改内存状态", () => {
    setDashboardCollapsed(true);
    expect(localStorage.getItem(DASHBOARD_COLLAPSED_STORAGE_KEY)).toBe("true");
    expect(dashboardState.collapsed).toBe(true);
  });

  it("落盘失败仍切换内存状态", () => {
    const mock = installLocalStorageMock();
    vi.spyOn(mock, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    setDashboardCollapsed(true);
    expect(dashboardState.collapsed).toBe(true);
  });
});

describe("initDashboard", () => {
  it("按持久化值初始化", () => {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "true");
    initDashboard();
    expect(dashboardState.collapsed).toBe(true);
  });

  it("无存量时回到默认展开", () => {
    dashboardState.collapsed = true;
    initDashboard();
    expect(dashboardState.collapsed).toBe(DEFAULT_DASHBOARD_COLLAPSED);
  });

  it("重复调用无副作用", () => {
    localStorage.setItem(DASHBOARD_COLLAPSED_STORAGE_KEY, "true");
    initDashboard();
    initDashboard();
    expect(dashboardState.collapsed).toBe(true);
  });
});
