import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import commands from "#libs/commands";
import type { AnyResult, UpdateInfo } from "#libs/commands/types";
import {
  __resetRestartForTests,
  checkForUpdate,
  downloadAndInstall,
  maybeAutoCheckForUpdate,
  restartApp,
  updaterState,
} from "#libs/hooks/updater.svelte";

vi.mock("#libs/commands", () => ({
  default: { checkUpdate: vi.fn(), downloadAndInstallUpdate: vi.fn(), restartApp: vi.fn() },
}));

vi.mock("#libs/utils/toast", () => ({
  toast: { message: vi.fn(), info: vi.fn(), error: vi.fn() },
}));

import { toast as toastMock } from "#libs/utils/toast";

const latest: UpdateInfo = { version: "0.2.0", current_version: "0.1.0", body: "notes" };

type FakeResult = AnyResult;
type FakeChain = {
  success: (handler: (data: never) => unknown) => FakeChain;
  failed: (handler: (failure: never) => unknown) => FakeChain;
  then: (resolve: (value: FakeResult) => unknown) => Promise<unknown>;
};

/** 与 `EnhancedCommand` 行为一致的替身：先按分支跑回调，再结算结果 */
function fakeCommand(result: FakeResult): FakeChain {
  const onSuccess: ((data: never) => unknown)[] = [];
  const onFailure: ((failure: never) => unknown)[] = [];

  const chain: FakeChain = {
    success(handler) {
      onSuccess.push(handler);
      return chain;
    },
    failed(handler) {
      onFailure.push(handler);
      return chain;
    },
    then(resolve) {
      const handlers = result.status === "ok" ? onSuccess : onFailure;
      const payload = (result.status === "ok" ? result.data : result.error) as never;
      for (const handler of handlers) {
        handler(payload);
      }
      return Promise.resolve(result).then(resolve);
    },
  };
  return chain;
}

function stubCheck(result: FakeResult): void {
  vi.mocked(commands.checkUpdate).mockReturnValue(fakeCommand(result) as never);
}

function stubDownload(result: FakeResult): void {
  vi.mocked(commands.downloadAndInstallUpdate).mockReturnValue(fakeCommand(result) as never);
}

function resetUpdaterState(): void {
  updaterState.phase = "idle";
  updaterState.latest = null;
  updaterState.downloaded = 0;
  updaterState.total = null;
  updaterState.error = null;
  updaterState.autoChecked = false;
}

beforeEach(() => {
  resetUpdaterState();
  __resetRestartForTests();
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("checkForUpdate", () => {
  it("有新版时进入待处理并记住版本信息", async () => {
    stubCheck({ status: "ok", data: latest });

    await checkForUpdate();

    expect(updaterState.phase).toBe("available");
    expect(updaterState.latest).toEqual(latest);
  });

  it("已是最新时进最新态并提示", async () => {
    stubCheck({ status: "ok", data: null });

    await checkForUpdate();

    expect(updaterState.phase).toBe("up-to-date");
    expect(vi.mocked(toastMock.message)).toHaveBeenCalled();
  });

  it("失败时进错误态并提示", async () => {
    stubCheck({ status: "error", error: { kind: "Internal", message: "boom" } });

    await checkForUpdate();

    expect(updaterState.phase).toBe("error");
    expect(updaterState.error).toBe("boom");
    expect(vi.mocked(toastMock.error)).toHaveBeenCalled();
  });

  it("静默失败只记日志不打扰", async () => {
    stubCheck({ status: "error", error: { kind: "Internal", message: "boom" } });

    await checkForUpdate({ silent: true });

    expect(updaterState.phase).toBe("error");
    expect(vi.mocked(toastMock.error)).not.toHaveBeenCalled();
    expect(console.error).toHaveBeenCalledWith(expect.stringContaining("[updater] silent check"));
  });

  it("检查中不重复提交", async () => {
    updaterState.phase = "checking";
    stubCheck({ status: "ok", data: null });

    await checkForUpdate();

    expect(commands.checkUpdate).not.toHaveBeenCalled();
  });

  it("待重启态不被新检查结果覆盖", async () => {
    updaterState.phase = "ready";
    updaterState.latest = latest;
    stubCheck({ status: "ok", data: { ...latest, version: "0.3.0" } });

    await checkForUpdate();

    expect(updaterState.phase).toBe("ready");
    expect(updaterState.latest).toEqual(latest);
  });

  it("静默失败重置自动检查标记，允许下次重试", async () => {
    updaterState.autoChecked = true;
    stubCheck({ status: "error", error: { kind: "Internal", message: "boom" } });

    await checkForUpdate({ silent: true });

    expect(updaterState.phase).toBe("error");
    expect(updaterState.autoChecked).toBe(false);
  });
});

describe("downloadAndInstall", () => {
  it("仅从有新版状态进入下载，成功后待重启", async () => {
    updaterState.phase = "available";
    updaterState.latest = latest;
    stubDownload({ status: "ok", data: null });

    await downloadAndInstall();

    expect(updaterState.phase).toBe("ready");
  });

  it("非待处理状态不提交", async () => {
    stubDownload({ status: "ok", data: null });

    await downloadAndInstall();

    expect(commands.downloadAndInstallUpdate).not.toHaveBeenCalled();
  });

  it("失败时进错误态并提示", async () => {
    updaterState.phase = "available";
    updaterState.latest = latest;
    stubDownload({ status: "error", error: { kind: "Internal", message: "boom" } });

    await downloadAndInstall();

    expect(updaterState.phase).toBe("error");
    expect(vi.mocked(toastMock.error)).toHaveBeenCalled();
  });
});

describe("restartApp", () => {
  it("仅从待重启状态调用命令", () => {
    vi.mocked(commands.restartApp).mockReturnValue(
      fakeCommand({ status: "ok", data: null }) as never,
    );
    restartApp();

    expect(commands.restartApp).not.toHaveBeenCalled();

    updaterState.phase = "ready";
    restartApp();

    expect(commands.restartApp).toHaveBeenCalledOnce();
  });

  it("重启连点只发一次请求", () => {
    vi.mocked(commands.restartApp).mockReturnValue(
      fakeCommand({ status: "ok", data: null }) as never,
    );
    updaterState.phase = "ready";
    restartApp();
    restartApp();

    expect(commands.restartApp).toHaveBeenCalledOnce();
  });
});

describe("maybeAutoCheckForUpdate", () => {
  it("开关关闭或已查过时不执行", () => {
    maybeAutoCheckForUpdate(false);

    expect(commands.checkUpdate).not.toHaveBeenCalled();

    updaterState.autoChecked = true;
    maybeAutoCheckForUpdate(true);

    expect(commands.checkUpdate).not.toHaveBeenCalled();
  });

  it("非 Tauri 环境标记已查但不发请求", () => {
    maybeAutoCheckForUpdate(true);

    expect(updaterState.autoChecked).toBe(true);
    expect(commands.checkUpdate).not.toHaveBeenCalled();
  });
});
