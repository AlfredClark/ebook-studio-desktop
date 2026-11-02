import { afterEach, describe, expect, it, vi } from "vitest";
import { normalizeExternalUrl, openExternal } from "#libs/utils/opener";

describe("normalizeExternalUrl", () => {
  it("剥掉 npm 的 git+ 传输前缀", () => {
    expect(normalizeExternalUrl("git+https://github.com/example/repo.git")).toBe(
      "https://github.com/example/repo.git",
    );
  });

  it("已是普通地址时原样返回", () => {
    expect(normalizeExternalUrl("https://example.com")).toBe("https://example.com");
    expect(normalizeExternalUrl("https://github.com/example/repo/issues")).toBe(
      "https://github.com/example/repo/issues",
    );
  });
});

describe("openExternal", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("非 Tauri 环境返回假，调用方据此提示", async () => {
    await expect(openExternal("https://example.com")).resolves.toBe(false);
  });

  it("git+ 前缀剥离后按 https 处理", async () => {
    // 非 Tauri 环境动态导入失败，断言点在于前缀剥离不触发 scheme 拒绝上报
    vi.spyOn(console, "error").mockImplementation(() => {});
    await expect(openExternal("git+https://github.com/example/repo")).resolves.toBe(false);
    expect(console.error).not.toHaveBeenCalledWith(
      expect.stringContaining("[opener] unsupported url scheme"),
    );
  });

  it("非 http(s) scheme 直接拒绝并上报", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    await expect(openExternal("mailto:someone@example.com")).resolves.toBe(false);
    await expect(openExternal("file:///etc/passwd")).resolves.toBe(false);
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("[opener] unsupported url scheme"),
    );
  });
});
