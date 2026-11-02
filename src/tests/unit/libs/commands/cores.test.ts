import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EnhancedCommand } from "#libs/commands/cores";
import type { AnyResult } from "#libs/commands/types";

type TestError = { kind: "Internal"; message: string };
type TestResult = AnyResult<string | null, TestError>;

const failure: TestError = { kind: "Internal", message: "boom" };

function succeeds(data: string | null): Promise<TestResult> {
  return Promise.resolve({ status: "ok", data });
}

function fails(): Promise<TestResult> {
  return Promise.resolve({ status: "error", error: failure });
}

beforeEach(() => {
  // 失败路径会走自动上报，这里统一静音，需要断言的用例再单独取用
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("value", () => {
  it("省略默认值时：成功返回数据，失败返回 undefined", async () => {
    await expect(new EnhancedCommand(succeeds("你好")).value()).resolves.toBe("你好");
    await expect(new EnhancedCommand(fails()).value()).resolves.toBeUndefined();
  });

  it("失败或数据为 null 时回落默认值", async () => {
    await expect(new EnhancedCommand(fails()).value("默认")).resolves.toBe("默认");
    await expect(new EnhancedCommand(succeeds(null)).value("默认")).resolves.toBe("默认");
    await expect(new EnhancedCommand(succeeds("你好")).value("默认")).resolves.toBe("你好");
  });

  it("IPC rejection 会被归一化，value 正常回落", async () => {
    const rejected = Promise.reject<TestResult>(new Error("ipc down"));

    await expect(new EnhancedCommand(rejected).value("兜底")).resolves.toBe("兜底");
  });
});

describe("result", () => {
  it("返回完整结果", async () => {
    await expect(new EnhancedCommand(succeeds("你好")).result()).resolves.toEqual({
      status: "ok",
      data: "你好",
    });
  });

  it("await 命令本身等价于 result()", async () => {
    await expect(await new EnhancedCommand(fails())).toEqual({ status: "error", error: failure });
  });
});

describe("success / failed", () => {
  it("只触发对应分支", async () => {
    const onSuccess = vi.fn();
    const onFailure = vi.fn();

    await new EnhancedCommand(succeeds("你好")).success(onSuccess).failed(onFailure);
    expect(onSuccess).toHaveBeenCalledWith("你好");
    expect(onFailure).not.toHaveBeenCalled();

    onSuccess.mockClear();
    await new EnhancedCommand(fails()).success(onSuccess).failed(onFailure);
    expect(onSuccess).not.toHaveBeenCalled();
    expect(onFailure).toHaveBeenCalledWith(failure);
  });

  it("await 会等待 async 回调执行完", async () => {
    const order: string[] = [];

    await new EnhancedCommand(succeeds("你好")).success(async () => {
      await new Promise((resolve) => setTimeout(resolve, 1));
      order.push("handler");
    });
    order.push("after-await");

    expect(order).toEqual(["handler", "after-await"]);
  });

  it("回调抛错不会让 await 失败，也不改变结果", async () => {
    const result = await new EnhancedCommand(succeeds("你好")).success(() => {
      throw new Error("handler boom");
    });

    expect(result).toEqual({ status: "ok", data: "你好" });
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("command handler failed: Error: handler boom"),
    );
  });
});

describe("失败上报", () => {
  it("未注册 .failed() 时自动上报", async () => {
    await new EnhancedCommand(fails()).value("默认");

    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining('unhandled command failure: {"kind":"Internal","message":"boom"}'),
    );
  });

  it("注册了 .failed() 就不再自动上报", async () => {
    await new EnhancedCommand(fails()).failed(() => {});

    expect(console.error).not.toHaveBeenCalled();
  });
});
