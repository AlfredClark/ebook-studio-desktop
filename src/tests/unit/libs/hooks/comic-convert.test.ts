import { afterEach, describe, expect, it, vi } from "vitest";
import commands from "#libs/commands";
import {
  convertJobOf,
  ensureConvertListener,
  refreshConvertStatus,
  setConvertRunning,
} from "#libs/hooks/comic-convert.svelte";

vi.mock("#libs/commands", () => ({
  default: { getComicConvertStatus: vi.fn() },
}));

function stubStatus(result: unknown) {
  vi.mocked(commands.getComicConvertStatus).mockReturnValue({
    result: () => Promise.resolve(result),
  } as never);
}

afterEach(() => {
  vi.clearAllMocks();
  setConvertRunning("uuid-a", false);
  setConvertRunning("uuid-b", false);
});

describe("漫画转换跨页状态", () => {
  it("默认空闲，按项目隔离", () => {
    expect(convertJobOf("uuid-a")).toEqual({ running: false, done: 0, total: 0 });
    setConvertRunning("uuid-a", true);
    expect(convertJobOf("uuid-a").running).toBe(true);
    expect(convertJobOf("uuid-b").running).toBe(false);
  });

  it("结束复位进度", () => {
    setConvertRunning("uuid-a", true);
    setConvertRunning("uuid-a", false);
    expect(convertJobOf("uuid-a")).toEqual({ running: false, done: 0, total: 0 });
  });

  it("回填远端运行态", async () => {
    stubStatus({
      status: "ok",
      data: { running: true, done_files: 3, total_files: 10 },
    });
    await refreshConvertStatus("uuid-a");
    expect(convertJobOf("uuid-a")).toEqual({ running: true, done: 3, total: 10 });
  });

  it("回填失败保持本地", async () => {
    setConvertRunning("uuid-a", true);
    stubStatus({ status: "error", error: { kind: "Internal", message: "boom" } });
    await refreshConvertStatus("uuid-a");
    expect(convertJobOf("uuid-a").running).toBe(true);
  });

  it("非 Tauri 环境订阅不抛错", () => {
    expect(() => ensureConvertListener()).not.toThrow();
  });
});
