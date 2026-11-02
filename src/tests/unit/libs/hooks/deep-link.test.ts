import { beforeEach, describe, expect, it, vi } from "vitest";
import { handleDeepLinkUrl, isDeepLink, routeForUrl } from "#libs/hooks/deep-link.svelte";

const gotoMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({
  info: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("$app/navigation", () => ({ goto: gotoMock }));
vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("isDeepLink", () => {
  it("仅接受本应用协议", () => {
    expect(isDeepLink("ebook-studio://settings")).toBe(true);
    expect(isDeepLink("ebook-studio://")).toBe(true);
    expect(isDeepLink("https://example.com")).toBe(false);
    expect(isDeepLink("other://settings")).toBe(false);
    expect(isDeepLink("")).toBe(false);
  });
});

describe("routeForUrl", () => {
  it("映射已知路径到应用内路由", () => {
    expect(routeForUrl("ebook-studio://settings")).toBe("/settings");
    expect(routeForUrl("ebook-studio:///settings")).toBe("/settings");
    expect(routeForUrl("ebook-studio://about")).toBe("/about");
    expect(routeForUrl("ebook-studio://about?tab=1")).toBe("/about");
    expect(routeForUrl("ebook-studio://")).toBe("/");
    expect(routeForUrl("ebook-studio:///")).toBe("/");
  });

  it("未知路径与非本协议回落空", () => {
    expect(routeForUrl("ebook-studio://unknown")).toBeNull();
    expect(routeForUrl("https://example.com")).toBeNull();
  });
});

describe("handleDeepLinkUrl", () => {
  it("命中跳转并提示", () => {
    handleDeepLinkUrl("ebook-studio://settings");

    expect(gotoMock).toHaveBeenCalledWith("/settings");
    expect(toastMocks.info).toHaveBeenCalled();
  });

  it("未知路径停留并警告", () => {
    handleDeepLinkUrl("ebook-studio://nope");

    expect(gotoMock).not.toHaveBeenCalled();
    expect(toastMocks.warning).toHaveBeenCalled();
  });

  it("短窗内重复到达只处理一次", () => {
    handleDeepLinkUrl("ebook-studio://about");
    handleDeepLinkUrl("ebook-studio://about");

    expect(gotoMock).toHaveBeenCalledTimes(1);
  });

  it("去重表有界：同窗突发超限淘汰最旧", () => {
    // query 参与去重键、剥离后同路由，保证走跳转分支而非未知分支
    vi.useFakeTimers();
    try {
      vi.setSystemTime(1_000_000);
      handleDeepLinkUrl("ebook-studio://about?victim=1");
      for (let index = 0; index < 150; index += 1) {
        handleDeepLinkUrl(`ebook-studio://about?burst=${index}`);
      }
      // 受害者早被挤出，同窗重到会再次处理
      const before = gotoMock.mock.calls.length;
      handleDeepLinkUrl("ebook-studio://about?victim=1");
      expect(gotoMock.mock.calls.length).toBe(before + 1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("淘汰只丢最旧：窗内新条目去重语义不变", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(1_000_000);
      for (let index = 0; index < 110; index += 1) {
        handleDeepLinkUrl(`ebook-studio://about?fill=${index}`);
      }
      handleDeepLinkUrl("ebook-studio://about?fresh=1");
      const before = gotoMock.mock.calls.length;
      // fresh 未被淘汰（淘汰的是最旧的 filler），重复到达仍忽略
      handleDeepLinkUrl("ebook-studio://about?fresh=1");
      expect(gotoMock.mock.calls.length).toBe(before);
      // 已淘汰的最旧 filler 重到会再次处理
      handleDeepLinkUrl("ebook-studio://about?fill=0");
      expect(gotoMock.mock.calls.length).toBe(before + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
