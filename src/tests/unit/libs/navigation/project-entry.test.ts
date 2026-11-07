import { afterEach, describe, expect, it, vi } from "vitest";
import { readProjectEntry, storeProjectEntry } from "#libs/navigation/project-entry";

// localStorage 的内存替身：node 环境无 DOM，逐用例隔离。
function memoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, String(value)),
    removeItem: (key: string) => void data.delete(key),
    clear: () => data.clear(),
    key: (index: number) => [...data.keys()][index] ?? null,
    get length() {
      return data.size;
    },
  } as Storage;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("project-entry", () => {
  it("缺失时回落项目页", () => {
    vi.stubGlobal("localStorage", memoryStorage());
    expect(readProjectEntry()).toBe("/projects");
  });

  it("非法值回落项目页", () => {
    const storage = memoryStorage();
    vi.stubGlobal("localStorage", storage);
    storage.setItem("app.projects.entryFrom", "/settings");
    expect(readProjectEntry()).toBe("/projects");
  });

  it("首页与项目页往返读写一致", () => {
    vi.stubGlobal("localStorage", memoryStorage());
    storeProjectEntry("/");
    expect(readProjectEntry()).toBe("/");
    storeProjectEntry("/projects");
    expect(readProjectEntry()).toBe("/projects");
  });

  it("无存储环境静默回落，不抛错", () => {
    vi.stubGlobal("localStorage", undefined);
    expect(() => storeProjectEntry("/")).not.toThrow();
    expect(readProjectEntry()).toBe("/projects");
  });
});
