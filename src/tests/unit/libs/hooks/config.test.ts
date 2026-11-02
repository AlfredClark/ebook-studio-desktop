import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import commands from "#libs/commands";
import type { AnyResult, Config_Serialize } from "#libs/commands/types";
import { configState, hydrateConfig, resetConfig, updateConfig } from "#libs/hooks/config.svelte";

vi.mock("#libs/commands", () => ({
  default: { getConfig: vi.fn(), resetConfig: vi.fn(), updateConfig: vi.fn() },
}));

const saved: Config_Serialize = {
  locale: "zh-CN",
  auto_start: true,
  remember_window: true,
  auto_check_update: true,
  tray_enabled: true,
  close_behavior: "prompt",
  shortcut_show: "",
  schema_version: 1,
};

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

function stubGetConfig(result: FakeResult): void {
  vi.mocked(commands.getConfig).mockReturnValue(fakeCommand(result) as never);
}

function stubUpdateConfig(result: FakeResult): void {
  vi.mocked(commands.updateConfig).mockReturnValue(fakeCommand(result) as never);
}

function stubResetConfig(result: FakeResult): void {
  vi.mocked(commands.resetConfig).mockReturnValue(fakeCommand(result) as never);
}

beforeEach(() => {
  configState.value = null;
  // 失败路径会走上报，这里统一静音，需要断言的用例再单独取用
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("hydrateConfig", () => {
  it("成功时写入配置状态", async () => {
    stubGetConfig({ status: "ok", data: saved });

    await hydrateConfig();

    expect(configState.value).toEqual(saved);
  });

  it("失败时保持未水合并上报", async () => {
    stubGetConfig({ status: "error", error: { kind: "Internal", message: "boom" } });

    await hydrateConfig();

    expect(configState.value).toBeNull();
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("[config] failed to load backend config"),
    );
  });

  it("首次失败重试一次，第二次成功则水合", async () => {
    vi.mocked(commands.getConfig)
      .mockReturnValueOnce(
        fakeCommand({ status: "error", error: { kind: "Internal", message: "boom" } }) as never,
      )
      .mockReturnValueOnce(fakeCommand({ status: "ok", data: saved }) as never);

    await hydrateConfig();

    expect(commands.getConfig).toHaveBeenCalledTimes(2);
    expect(configState.value).toEqual(saved);
  });
});

describe("updateConfig", () => {
  it("原样提交补丁，并以命令返回的写后值回写", async () => {
    const previous: Config_Serialize = {
      locale: "en",
      auto_start: false,
      remember_window: false,
      auto_check_update: false,
      tray_enabled: true,
      close_behavior: "prompt",
      shortcut_show: "",
      schema_version: 1,
    };
    configState.value = previous;
    const patch = { locale: "zh-CN" } as const;
    stubUpdateConfig({ status: "ok", data: saved });

    await updateConfig(patch);

    // 不注入默认值：未提交的键必须留在后端不动
    expect(commands.updateConfig).toHaveBeenCalledWith(patch);
    expect(configState.value).toEqual(saved);
  });

  it("成功时返回真", async () => {
    stubUpdateConfig({ status: "ok", data: saved });

    await expect(updateConfig({ locale: "zh-CN" })).resolves.toBe(true);
  });

  it("失败时不乐观更新并上报", async () => {
    const previous: Config_Serialize = {
      locale: "en",
      auto_start: false,
      remember_window: false,
      auto_check_update: false,
      tray_enabled: true,
      close_behavior: "prompt",
      shortcut_show: "",
      schema_version: 1,
    };
    configState.value = previous;
    stubUpdateConfig({ status: "error", error: { kind: "Internal", message: "boom" } });

    await expect(updateConfig({ locale: "zh-CN" })).resolves.toBe(false);

    expect(configState.value).toEqual(previous);
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("[config] failed to update backend config"),
    );
  });
});

describe("resetConfig", () => {
  const defaults: Config_Serialize = {
    locale: "en",
    auto_start: false,
    remember_window: false,
    auto_check_update: false,
    tray_enabled: true,
    close_behavior: "prompt",
    shortcut_show: "",
    schema_version: 1,
  };

  it("成功时回写并返回写后配置", async () => {
    stubResetConfig({ status: "ok", data: defaults });

    const result = await resetConfig();

    expect(commands.resetConfig).toHaveBeenCalledOnce();
    expect(configState.value).toEqual(defaults);
    expect(result).toEqual(defaults);
  });

  it("失败时保持原状态并返回空", async () => {
    configState.value = saved;
    stubResetConfig({ status: "error", error: { kind: "Internal", message: "boom" } });

    const result = await resetConfig();

    expect(result).toBeNull();
    expect(configState.value).toEqual(saved);
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("[config] failed to reset backend config"),
    );
  });
});
