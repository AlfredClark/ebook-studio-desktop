import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Page from "../../../../../routes/(main)/about/+page.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同设置页测试）。
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

// 纯替身：更新全局状态、系统信息命令、外链与提示全部 mock，
// 按阶段分别渲染后断言，避免真实网络与 Tauri 运行时。
const updaterStateMock = vi.hoisted(
  (): {
    phase: string;
    latest: { version: string; current_version: string; body: string | null } | null;
    downloaded: number;
    total: number | null;
    error: string | null;
    autoChecked: boolean;
  } => ({
    phase: "idle",
    latest: null,
    downloaded: 0,
    total: null,
    error: null,
    autoChecked: false,
  }),
);
const checkForUpdateMock = vi.hoisted(() => vi.fn());
const downloadAndInstallMock = vi.hoisted(() => vi.fn());
const restartAppMock = vi.hoisted(() => vi.fn());
const getSystemInfoMock = vi.hoisted(() => vi.fn());
const openLogDirMock = vi.hoisted(() => vi.fn());
const openConfigDirMock = vi.hoisted(() => vi.fn());
const copySystemInfoMock = vi.hoisted(() => vi.fn());
const openExternalMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({
  message: vi.fn(),
  info: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  loading: vi.fn(() => 1),
  dismiss: vi.fn(),
}));

vi.mock("#libs/hooks/updater.svelte", () => ({
  updaterState: updaterStateMock,
  checkForUpdate: checkForUpdateMock,
  downloadAndInstall: downloadAndInstallMock,
  restartApp: restartAppMock,
}));

vi.mock("#libs/commands", () => ({
  default: {
    getSystemInfo: getSystemInfoMock,
    openLogDir: openLogDirMock,
    openConfigDir: openConfigDirMock,
    copySystemInfo: copySystemInfoMock,
  },
}));

vi.mock("#libs/utils/opener", async (importOriginal) => {
  const actual = await importOriginal<typeof import("#libs/utils/opener")>();
  // 只替身有副作用的 openExternal；纯函数（normalizeExternalUrl）保留真身
  return { ...actual, openExternal: openExternalMock };
});

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：先按分支跑回调，再结算结果 */
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

function resetUpdaterState(): void {
  updaterStateMock.phase = "idle";
  updaterStateMock.latest = null;
  updaterStateMock.downloaded = 0;
  updaterStateMock.total = null;
  updaterStateMock.error = null;
  updaterStateMock.autoChecked = false;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetUpdaterState();
  openExternalMock.mockResolvedValue(false);
  getSystemInfoMock.mockReturnValue(fakeCommand({ status: "ok", data: sysInfo }) as never);
  const ok = fakeCommand({ status: "ok", data: null }) as never;
  openLogDirMock.mockReturnValue(ok);
  openConfigDirMock.mockReturnValue(ok);
  copySystemInfoMock.mockReturnValue(ok);
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("关于页", () => {
  it("渲染应用元信息与运行平台行", async () => {
    render(Page);

    expect(screen.getByText("App name")).not.toBeNull();
    expect(screen.getByText("App info")).not.toBeNull();
    expect(screen.getByText("Project info")).not.toBeNull();
    expect(screen.getByText("Platform info")).not.toBeNull();
    expect(screen.getByText("Tauri Desktop Template")).not.toBeNull();
    expect(screen.getByText("Current version")).not.toBeNull();
    // 版本号以 tauri.conf.json 构建常量为准，写死会随 bump 脚本失效
    expect(screen.getByText(__APP_TAURI_CONF__.version)).not.toBeNull();
    expect(screen.getByText("Description")).not.toBeNull();
    expect(screen.getByText("License")).not.toBeNull();
    expect(screen.getByText("Author")).not.toBeNull();
    expect(screen.getByText("Homepage")).not.toBeNull();
    expect(screen.getByText("Repository")).not.toBeNull();
    expect(screen.getByText("Feedback")).not.toBeNull();
    expect(screen.getByText("Diagnostics")).not.toBeNull();
    expect(screen.getByText("Log directory")).not.toBeNull();
    expect(screen.getByText("Config directory")).not.toBeNull();
    expect(screen.getByText("Copy system info")).not.toBeNull();

    // 平台信息异步加载完成后逐行展示
    await vi.waitFor(() => {
      expect(screen.getByText("dev-machine")).not.toBeNull();
    });
    expect(screen.getByText("linux")).not.toBeNull();
    expect(screen.getByText("22.04")).not.toBeNull();
    expect(screen.getByText("x86_64")).not.toBeNull();
  });

  it("空闲时检查按钮触发检查", async () => {
    const user = userEvent.setup();
    render(Page);

    await user.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(checkForUpdateMock).toHaveBeenCalledOnce();
  });

  it("检查中按钮禁用并提示", () => {
    updaterStateMock.phase = "checking";
    render(Page);

    expect(screen.getByRole("button", { name: "Checking…" }) as HTMLButtonElement).toHaveProperty(
      "disabled",
      true,
    );
  });

  it("已是最新时徽章常驻且按钮变为重新检查", async () => {
    updaterStateMock.phase = "up-to-date";
    const user = userEvent.setup();
    render(Page);

    expect(screen.getByText("Already up to date")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Check again" }));

    expect(checkForUpdateMock).toHaveBeenCalledOnce();
  });

  it("有新版时展示版本说明与下载入口", async () => {
    // mock 版本号刻意与应用版本拉开，避免与版本行重复匹配
    updaterStateMock.phase = "available";
    updaterStateMock.latest = { version: "9.9.9", current_version: "0.1.0", body: "notes" };
    const user = userEvent.setup();
    render(Page);

    expect(screen.getByText(/9\.9\.9/)).not.toBeNull();
    expect(screen.getByText("notes")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Download & install" }));

    expect(downloadAndInstallMock).toHaveBeenCalledOnce();
  });

  it("下载中展示进度百分比", () => {
    updaterStateMock.phase = "downloading";
    updaterStateMock.downloaded = 50;
    updaterStateMock.total = 100;
    render(Page);

    expect(screen.getByText("Downloading… 50%")).not.toBeNull();
    expect(screen.getByRole("progressbar")).not.toBeNull();
  });

  it("就绪后重启按钮触发重启", async () => {
    updaterStateMock.phase = "ready";
    const user = userEvent.setup();
    render(Page);

    await user.click(screen.getByRole("button", { name: "Restart now" }));

    expect(restartAppMock).toHaveBeenCalledOnce();
  });

  it("失败时展示错误信息", () => {
    updaterStateMock.phase = "error";
    updaterStateMock.error = "network down";
    render(Page);

    expect(screen.getByText("network down")).not.toBeNull();
  });

  it("外链打不开时提示", async () => {
    const user = userEvent.setup();
    render(Page);

    await user.click(screen.getAllByRole("button", { name: "Open" })[0]);

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
  });
});
