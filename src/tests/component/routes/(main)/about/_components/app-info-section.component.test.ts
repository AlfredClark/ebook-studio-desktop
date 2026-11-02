import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/about/_components/app-info-section.svelte";

// 纯替身：更新全局状态全部 mock（含内嵌更新面板的下载与重启），
// 只验证应用信息分组的接线——元信息行、检查按钮与各阶段透传。
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

describe("应用信息分组", () => {
  it("渲染应用元信息行", () => {
    render(Component);

    expect(screen.getByText("App info")).not.toBeNull();
    expect(screen.getByText("App name")).not.toBeNull();
    expect(screen.getByText("Tauri Desktop Template")).not.toBeNull();
    expect(screen.getByText("Current version")).not.toBeNull();
    // 版本号以 tauri.conf.json 构建常量为准，写死会随 bump 脚本失效
    expect(screen.getByText(__APP_TAURI_CONF__.version)).not.toBeNull();
    expect(screen.getByText("Description")).not.toBeNull();
    expect(screen.getByText("License")).not.toBeNull();
    expect(screen.getByText("Author")).not.toBeNull();
  });

  it("空闲时检查按钮触发检查", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(checkForUpdateMock).toHaveBeenCalledOnce();
  });

  it("检查中按钮禁用并提示", () => {
    updaterStateMock.phase = "checking";
    render(Component);

    expect(screen.getByRole("button", { name: "Checking…" }) as HTMLButtonElement).toHaveProperty(
      "disabled",
      true,
    );
  });

  it("已是最新时徽章常驻且按钮变为重新检查", async () => {
    updaterStateMock.phase = "up-to-date";
    const user = userEvent.setup();
    render(Component);

    expect(screen.getByText("Already up to date")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Check again" }));

    expect(checkForUpdateMock).toHaveBeenCalledOnce();
  });
});
