import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ensureExportListener,
  exportJobOf,
  refreshExportStatus,
  setExportRunning,
} from "#libs/hooks/export-progress.svelte";

afterEach(() => {
  vi.clearAllMocks();
  setExportRunning("uuid-a", false);
  setExportRunning("uuid-b", false);
});

describe("导出进度跨页状态", () => {
  it("默认空闲，按项目隔离", () => {
    expect(exportJobOf("uuid-a")).toEqual({ running: false, done_files: 0, total_files: 0 });
    setExportRunning("uuid-a", true);
    expect(exportJobOf("uuid-a").running).toBe(true);
    expect(exportJobOf("uuid-b").running).toBe(false);
  });

  it("结束复位进度", () => {
    setExportRunning("uuid-a", true);
    setExportRunning("uuid-a", false);
    expect(exportJobOf("uuid-a")).toEqual({ running: false, done_files: 0, total_files: 0 });
  });

  it("回填远端运行态", async () => {
    await refreshExportStatus("uuid-a", () =>
      Promise.resolve({ running: true, done_files: 3, total_files: 10 }),
    );
    expect(exportJobOf("uuid-a")).toEqual({ running: true, done_files: 3, total_files: 10 });
  });

  it("回填远端空闲态", async () => {
    setExportRunning("uuid-a", true);
    await refreshExportStatus("uuid-a", () =>
      Promise.resolve({ running: false, done_files: 0, total_files: 0 }),
    );
    expect(exportJobOf("uuid-a").running).toBe(false);
  });

  it("查询失败保持本地", async () => {
    setExportRunning("uuid-a", true);
    await refreshExportStatus("uuid-a", () => Promise.resolve(null));
    expect(exportJobOf("uuid-a").running).toBe(true);
    await refreshExportStatus("uuid-a", () => Promise.reject(new Error("no ipc")));
    expect(exportJobOf("uuid-a").running).toBe(true);
  });

  it("非 Tauri 环境订阅不抛错", () => {
    expect(() => ensureExportListener("test-event")).not.toThrow();
  });
});
