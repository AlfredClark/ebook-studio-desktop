import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast as sonnerToast } from "svelte-sonner";
import { __resetToastDedupeForTests, toast } from "#libs/utils/toast";

vi.mock("svelte-sonner", () => ({
  toast: {
    dismiss: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    loading: vi.fn(),
    message: vi.fn(),
    promise: vi.fn(),
    success: vi.fn(),
    warning: vi.fn(),
  },
}));

const mocked = vi.mocked(sonnerToast);

describe("通用 toast 封装", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __resetToastDedupeForTests();
  });

  it("各通道透传文案并补齐默认时长", () => {
    toast.success("成功");
    toast.error("失败");
    toast.info("信息");
    toast.warning("警告");
    toast.message("消息");
    toast.loading("加载中");

    for (const fn of [
      mocked.success,
      mocked.error,
      mocked.info,
      mocked.warning,
      mocked.message,
      mocked.loading,
    ]) {
      expect(fn).toHaveBeenCalledOnce();
      expect(fn.mock.calls[0][1]).toMatchObject({ duration: 3000 });
    }
  });

  it("调用方显式选项优先于默认值", () => {
    toast.success("成功", { duration: 5000 });

    expect(mocked.success).toHaveBeenCalledWith("成功", {
      closeButton: false,
      duration: 5000,
    });
  });

  it("透出 promise 与 dismiss 能力", () => {
    expect(toast.promise).toBe(mocked.promise);
    expect(toast.dismiss).toBe(mocked.dismiss);
  });

  it("同 kind 同文案短窗内合并，不同文案或跨通道不合并", () => {
    const first = toast.error("失败");
    const deduped = toast.error("失败");

    expect(mocked.error).toHaveBeenCalledOnce();
    expect(deduped).toBe(first);

    toast.error("另一种失败");
    toast.warning("失败");
    expect(mocked.error).toHaveBeenCalledTimes(2);
    expect(mocked.warning).toHaveBeenCalledOnce();
  });
});
