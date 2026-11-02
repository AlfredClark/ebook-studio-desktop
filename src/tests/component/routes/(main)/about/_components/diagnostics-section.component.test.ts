import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import DiagnosticsSection from "../../../../../../routes/(main)/about/_components/diagnostics-section.svelte";

const openLogDirMock = vi.hoisted(() => vi.fn());
const openConfigDirMock = vi.hoisted(() => vi.fn());
const copySystemInfoMock = vi.hoisted(() => vi.fn());
const reportFailureMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));

vi.mock("#libs/commands", () => ({
  default: {
    openLogDir: openLogDirMock,
    openConfigDir: openConfigDirMock,
    copySystemInfo: copySystemInfoMock,
  },
}));

vi.mock("#libs/commands/cores", () => ({
  reportCommandFailure: reportFailureMock,
}));

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：按分支跑回调后结算结果 */
function fakeCommand(result: FakeResult): {
  success: (handler: (data: never) => unknown) => unknown;
  failed: (handler: (failure: never) => unknown) => unknown;
  result: () => Promise<FakeResult>;
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
    result() {
      return Promise.resolve(result);
    },
  };
  return chain;
}

function stubOk(mock: ReturnType<typeof vi.fn>): void {
  mock.mockReturnValue(fakeCommand({ status: "ok", data: null }) as never);
}

function stubFailed(mock: ReturnType<typeof vi.fn>): void {
  mock.mockReturnValue(
    fakeCommand({ status: "error", error: { kind: "Internal", message: "boom" } }) as never,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  stubOk(openLogDirMock);
  stubOk(openConfigDirMock);
  stubOk(copySystemInfoMock);
});

afterEach(() => {
  cleanup();
});

describe("诊断分组", () => {
  it("渲染三行操作", () => {
    render(DiagnosticsSection);

    expect(screen.getByText("Diagnostics")).not.toBeNull();
    expect(screen.getByText("Log directory")).not.toBeNull();
    expect(screen.getByText("Config directory")).not.toBeNull();
    expect(screen.getByText("Copy system info")).not.toBeNull();
  });

  it("打开目录成功时无提示", async () => {
    const user = userEvent.setup();
    render(DiagnosticsSection);

    await user.click(screen.getAllByRole("button", { name: "Open" })[0]);

    expect(openLogDirMock).toHaveBeenCalledOnce();
    expect(toastMocks.error).not.toHaveBeenCalled();
  });

  it("打开目录失败时提示", async () => {
    stubFailed(openConfigDirMock);
    const user = userEvent.setup();
    render(DiagnosticsSection);

    await user.click(screen.getAllByRole("button", { name: "Open" })[1]);

    expect(openConfigDirMock).toHaveBeenCalledOnce();
    expect(toastMocks.error).toHaveBeenCalledOnce();
  });

  it("复制成功与失败分别提示", async () => {
    const user = userEvent.setup();
    render(DiagnosticsSection);

    await user.click(screen.getByRole("button", { name: "Copy" }));

    expect(copySystemInfoMock).toHaveBeenCalledOnce();
    expect(toastMocks.success).toHaveBeenCalledOnce();
    expect(toastMocks.error).not.toHaveBeenCalled();
  });

  it("复制失败时提示", async () => {
    stubFailed(copySystemInfoMock);
    const user = userEvent.setup();
    render(DiagnosticsSection);

    await user.click(screen.getByRole("button", { name: "Copy" }));

    expect(toastMocks.error).toHaveBeenCalledOnce();
    expect(toastMocks.success).not.toHaveBeenCalled();
  });
});
