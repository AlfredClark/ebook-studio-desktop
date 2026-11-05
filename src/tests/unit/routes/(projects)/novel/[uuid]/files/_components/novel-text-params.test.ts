import { describe, expect, it } from "vitest";
import {
  DEFAULT_CHAPTER_PATTERN,
  DEFAULT_VOLUME_PATTERN,
  formatBytes,
  formatPaddedChapterRange,
  isValidPattern,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/files/_components/novel-text-params";

describe("小说原文参数", () => {
  it("默认表达式非空且行首起配", () => {
    expect(DEFAULT_VOLUME_PATTERN.startsWith("^")).toBe(true);
    expect(DEFAULT_CHAPTER_PATTERN.startsWith("^")).toBe(true);
    expect(new RegExp(DEFAULT_VOLUME_PATTERN).test("第一卷 楔子")).toBe(true);
    expect(new RegExp(DEFAULT_CHAPTER_PATTERN).test("第一章 引子")).toBe(true);
  });

  it("默认表达式第一捕获组为数字", () => {
    const volume = "第一万二千三百四十五卷 卷末语".match(new RegExp(DEFAULT_VOLUME_PATTERN));
    expect(volume?.[1]).toBe("一万二千三百四十五");
    expect(volume?.[2]).toBe("卷末语");
    const chapter = "第100章 尾声".match(new RegExp(DEFAULT_CHAPTER_PATTERN));
    expect(chapter?.[1]).toBe("100");
    expect(chapter?.[2]).toBe("尾声");
  });

  it("默认表达式支持大写数字与万", () => {
    expect(new RegExp(DEFAULT_VOLUME_PATTERN).test("第壹卷 卷首语")).toBe(true);
    expect(new RegExp(DEFAULT_CHAPTER_PATTERN).test("第一万二千三百四十五章 大结局")).toBe(true);
    expect(new RegExp(DEFAULT_CHAPTER_PATTERN).test("第100章 尾声")).toBe(true);
  });

  it("正则正确性预检：空串与坏语法无效", () => {
    expect(isValidPattern(DEFAULT_VOLUME_PATTERN)).toBe(true);
    expect(isValidPattern(DEFAULT_CHAPTER_PATTERN)).toBe(true);
    expect(isValidPattern("")).toBe(false);
    expect(isValidPattern("   ")).toBe(false);
    expect(isValidPattern("^(unclosed")).toBe(false);
  });

  it("字节数按单位展示", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 MB");
  });

  it("章节范围按总章数位数补零", () => {
    expect(formatPaddedChapterRange(1, 100, 205)).toEqual({ start: "001", end: "100" });
    expect(formatPaddedChapterRange(101, 200, 205)).toEqual({ start: "101", end: "200" });
    expect(formatPaddedChapterRange(201, 205, 205)).toEqual({ start: "201", end: "205" });
    expect(formatPaddedChapterRange(1, 100, 1200)).toEqual({ start: "0001", end: "0100" });
    expect(formatPaddedChapterRange(1, 2, 2)).toEqual({ start: "1", end: "2" });
  });
});
