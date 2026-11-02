import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// 文案键集守卫：加键漏译不会报错、只会静默回落，用测试兜住漂移。
// 相对本测试文件定位，避免依赖运行目录；改文案后先跑 pnpm i18n:compile 再对键。
function read(relativePath: string): string {
  return readFileSync(new URL(relativePath, import.meta.url), "utf8");
}

describe("前端文案键集", () => {
  const en = JSON.parse(read("../../../../../src/libs/i18n/messages/en.json")) as Record<
    string,
    string
  >;
  const zhCN = JSON.parse(read("../../../../../src/libs/i18n/messages/zh-CN.json")) as Record<
    string,
    string
  >;

  it("en 与 zh-CN 的键集完全一致", () => {
    expect(Object.keys(zhCN).sort()).toEqual(Object.keys(en).sort());
  });

  it("两份文案的值都是非空字符串", () => {
    for (const [locale, messages] of [
      ["en", en],
      ["zh-CN", zhCN],
    ] as const) {
      const emptyKeys = Object.entries(messages)
        .filter(([, value]) => typeof value !== "string" || value.trim().length === 0)
        .map(([key]) => `${locale}:${key}`);
      expect(emptyKeys).toEqual([]);
    }
  });
});

describe("后端文案（tray.yml）", () => {
  it("每个文案键同时提供 en 与 zh-CN", () => {
    // 行级结构检查：不引入 YAML 解析依赖；嵌套缩进由 prettier 稳定，超出该形状即报错提醒更新守卫
    const localeLines = read("../../../../../src-tauri/locales/tray.yml")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => /^(en|zh-CN): \S/.test(line));

    expect(localeLines.length).toBeGreaterThan(0);
    expect(localeLines.length % 2).toBe(0);
    for (let index = 0; index < localeLines.length; index += 2) {
      expect(localeLines[index]).toMatch(/^en: \S/);
      expect(localeLines[index + 1]).toMatch(/^zh-CN: \S/);
    }
  });
});
