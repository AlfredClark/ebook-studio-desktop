import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import { m } from "#libs/i18n/paraglide/messages";
import { toast } from "#libs/utils/toast";
import PlatformInfoSection from "../../../../../../routes/(main)/about/_components/platform-info-section.svelte";

const getSystemInfoMock = vi.hoisted(() => vi.fn());
const reportFailureMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: { getSystemInfo: getSystemInfoMock },
}));

vi.mock("#libs/commands/cores", () => ({
  reportCommandFailure: reportFailureMock,
}));

vi.mock("#libs/utils/toast", () => ({
  toast: { error: vi.fn() },
}));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：按分支跑回调后返回自身 */
function fakeCommand(result: FakeResult): {
  success: (handler: (data: never) => unknown) => unknown;
  failed: (handler: (failure: never) => unknown) => unknown;
} {
  const chain = {
    success(handler: (data: never) => unknown) {
      if (result.status === "ok") handler(result.data as never);
      return chain;
    },
    failed(handler: (failure: never) => unknown) {
      if (result.status === "error") handler(result.error as never);
      return chain;
    },
  };
  return chain;
}

const sysInfo = {
  platform: "linux",
  os_version: "22.04",
  arch: "x86_64",
  hostname: "dev-machine",
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("平台信息分组", () => {
  it("加载成功后逐行展示", async () => {
    getSystemInfoMock.mockReturnValue(fakeCommand({ status: "ok", data: sysInfo }) as never);
    render(PlatformInfoSection);

    await vi.waitFor(() => {
      expect(screen.getByText("dev-machine")).not.toBeNull();
    });
    expect(screen.getByText("linux")).not.toBeNull();
    expect(screen.getByText("22.04")).not.toBeNull();
    expect(screen.getByText("x86_64")).not.toBeNull();
  });

  it("加载失败时转为常驻错误态并提示", async () => {
    getSystemInfoMock.mockReturnValue(
      fakeCommand({ status: "error", error: { kind: "Internal", message: "boom" } }) as never,
    );
    render(PlatformInfoSection);

    // toast 几秒后自行消失，故失败必须留在行内：不白屏、不抛错，也不再留"加载中"的省略号
    await vi.waitFor(() => {
      expect(toast.error).toHaveBeenCalledOnce();
    });
    expect(reportFailureMock).toHaveBeenCalledWith(
      "[about] failed to load system info",
      expect.anything(),
    );
    expect(screen.getByText(m.about_system_failed())).not.toBeNull();
    expect(screen.queryAllByText("…")).toHaveLength(0);
  });
});
