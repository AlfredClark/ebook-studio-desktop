import { beforeEach, describe, expect, it, vi } from "vitest";
import { alignLocale } from "#libs/i18n/locale.svelte";

// 纯替身：桥只负责“写缓存 + 同步响应式状态”，运行时行为（localStorage 写入、重载判断）在此隔离。
// 响应式重渲染的回归在 src/tests/component/libs/i18n/locale.component.test.ts 用真实运行时验证。
const runtimeMock = vi.hoisted(() => ({
  getLocale: vi.fn(() => "en"),
  overwriteGetLocale: vi.fn(),
  setLocale: vi.fn(),
}));

vi.mock("#libs/i18n/paraglide/runtime", () => runtimeMock);

describe("语言响应式桥", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("对齐经运行时写缓存且不重载", () => {
    alignLocale("zh-CN");
    expect(runtimeMock.setLocale).toHaveBeenCalledWith("zh-CN", { reload: false });
  });

  it("语言一致时不再写缓存", () => {
    // 先归位到初始语言，保证后续断言与用例执行顺序无关
    alignLocale("en");
    runtimeMock.setLocale.mockClear();

    alignLocale("en");
    expect(runtimeMock.setLocale).not.toHaveBeenCalled();
  });
});
