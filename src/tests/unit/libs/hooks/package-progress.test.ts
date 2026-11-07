import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ensurePackagingListener,
  packagingJobOf,
  refreshPackagingStatus,
  setPackagingRunning,
} from "#libs/hooks/package-progress.svelte";

afterEach(() => {
  vi.clearAllMocks();
  setPackagingRunning("uuid-a", false);
  setPackagingRunning("uuid-b", false);
});

describe("打包进度跨页状态", () => {
  it("默认空闲，按项目隔离", () => {
    expect(packagingJobOf("uuid-a")).toEqual({
      running: false,
      stage: null,
      done_files: 0,
      total_files: 0,
    });
    setPackagingRunning("uuid-a", true);
    expect(packagingJobOf("uuid-a").running).toBe(true);
    expect(packagingJobOf("uuid-b").running).toBe(false);
  });

  it("结束复位进度", () => {
    setPackagingRunning("uuid-a", true);
    setPackagingRunning("uuid-a", false);
    expect(packagingJobOf("uuid-a")).toEqual({
      running: false,
      stage: null,
      done_files: 0,
      total_files: 0,
    });
  });

  it("回填远端运行态", async () => {
    await refreshPackagingStatus("uuid-a", () =>
      Promise.resolve({ running: true, stage: "zip" as const, done_files: 3, total_files: 10 }),
    );
    expect(packagingJobOf("uuid-a")).toEqual({
      running: true,
      stage: "zip",
      done_files: 3,
      total_files: 10,
    });
  });

  it("回填远端空闲态", async () => {
    setPackagingRunning("uuid-a", true);
    await refreshPackagingStatus("uuid-a", () =>
      Promise.resolve({ running: false, stage: null, done_files: 0, total_files: 0 }),
    );
    expect(packagingJobOf("uuid-a").running).toBe(false);
  });

  it("查询失败保持本地", async () => {
    setPackagingRunning("uuid-a", true);
    await refreshPackagingStatus("uuid-a", () => Promise.resolve(null));
    expect(packagingJobOf("uuid-a").running).toBe(true);
    await refreshPackagingStatus("uuid-a", () => Promise.reject(new Error("no ipc")));
    expect(packagingJobOf("uuid-a").running).toBe(true);
  });

  it("非 Tauri 环境订阅不抛错", () => {
    expect(() => ensurePackagingListener("test-event")).not.toThrow();
  });
});
