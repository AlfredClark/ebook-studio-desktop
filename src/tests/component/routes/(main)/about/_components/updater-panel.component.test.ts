import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/about/_components/updater-panel.svelte";

// 纯替身：更新全局状态全部 mock，只验证更新面板各阶段的展示与入口接线。
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

vi.mock("#libs/hooks/updater.svelte", () => ({
  updaterState: updaterStateMock,
  checkForUpdate: checkForUpdateMock,
  downloadAndInstall: downloadAndInstallMock,
  restartApp: restartAppMock,
}));

beforeEach(() => {
  vi.clearAllMocks();
  updaterStateMock.phase = "idle";
  updaterStateMock.latest = null;
  updaterStateMock.downloaded = 0;
  updaterStateMock.total = null;
  updaterStateMock.error = null;
  updaterStateMock.autoChecked = false;
});

afterEach(() => {
  cleanup();
});

describe("更新状态面板", () => {
  it("空闲时不渲染任何面板", () => {
    render(Component);

    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("有新版时展示版本说明与下载入口", async () => {
    // mock 版本号刻意与应用版本拉开，避免与版本行重复匹配
    updaterStateMock.phase = "available";
    updaterStateMock.latest = { version: "9.9.9", current_version: "0.1.0", body: "notes" };
    const user = userEvent.setup();
    render(Component);

    expect(screen.getByText(/9\.9\.9/)).not.toBeNull();
    expect(screen.getByText("notes")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Download & install" }));

    expect(downloadAndInstallMock).toHaveBeenCalledOnce();
  });

  it("无版本说明时只展示版本与按钮", () => {
    updaterStateMock.phase = "available";
    updaterStateMock.latest = { version: "9.9.9", current_version: "0.1.0", body: null };
    render(Component);

    expect(screen.getByText(/9\.9\.9/)).not.toBeNull();
    expect(screen.getByRole("button", { name: "Download & install" })).not.toBeNull();
  });

  it("下载中展示进度百分比", () => {
    updaterStateMock.phase = "downloading";
    updaterStateMock.downloaded = 50;
    updaterStateMock.total = 100;
    render(Component);

    expect(screen.getByText("Downloading… 50%")).not.toBeNull();
    expect(screen.getByRole("progressbar")).not.toBeNull();
  });

  it("总量缺失时回落不确定进度", () => {
    updaterStateMock.phase = "downloading";
    updaterStateMock.downloaded = 50;
    updaterStateMock.total = null;
    render(Component);

    expect(screen.getByText(/Downloading/, { exact: false })).not.toBeNull();
    expect(screen.getByRole("progressbar")).not.toBeNull();
  });

  it("就绪后重启按钮触发重启", async () => {
    updaterStateMock.phase = "ready";
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Restart now" }));

    expect(restartAppMock).toHaveBeenCalledOnce();
  });

  it("失败时展示错误信息，无信息时不渲染", () => {
    updaterStateMock.phase = "error";
    updaterStateMock.error = "network down";
    const { unmount } = render(Component);

    expect(screen.getByText("network down")).not.toBeNull();
    unmount();
    cleanup();

    updaterStateMock.phase = "error";
    updaterStateMock.error = null;
    render(Component);

    expect(screen.queryByRole("button")).toBeNull();
  });
});
