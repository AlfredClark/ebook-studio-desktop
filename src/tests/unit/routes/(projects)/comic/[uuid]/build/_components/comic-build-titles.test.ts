import { describe, expect, it } from "vitest";
import {
  COMIC_PAGE_TITLE_KINDS,
  COMIC_VOLUME_TITLE_KINDS,
  DEFAULT_COMIC_PAGE_PATTERN,
  DEFAULT_COMIC_VOLUME_PATTERN,
  isValidComicTitlePattern,
  isValidComicTitles,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/build/_components/comic-build-titles";

describe("漫画构建标题格式", () => {
  it("话格式 6 种、页格式 5 种（页无原文）", () => {
    expect(COMIC_VOLUME_TITLE_KINDS).toEqual([
      "original",
      "arabic",
      "padded",
      "chinese_lower",
      "chinese_upper",
      "custom",
    ]);
    expect(COMIC_PAGE_TITLE_KINDS).toEqual([
      "arabic",
      "padded",
      "chinese_lower",
      "chinese_upper",
      "custom",
    ]);
  });

  it("自定义模板默认值非空且不带原名（话不带 #T、页为纯数字）", () => {
    expect(isValidComicTitlePattern(DEFAULT_COMIC_VOLUME_PATTERN)).toBe(true);
    expect(isValidComicTitlePattern(DEFAULT_COMIC_PAGE_PATTERN)).toBe(true);
    expect(DEFAULT_COMIC_VOLUME_PATTERN).not.toContain("#T");
    expect(DEFAULT_COMIC_PAGE_PATTERN).toBe("#N");
  });

  it("模板正确性预检：空串无效", () => {
    expect(isValidComicTitlePattern("")).toBe(false);
    expect(isValidComicTitlePattern("   ")).toBe(false);
    expect(isValidComicTitlePattern("第#N话")).toBe(true);
  });

  it("标题合法：话页自定义模板各自非空", () => {
    expect(isValidComicTitles("arabic", "", "original", "")).toBe(true);
    expect(isValidComicTitles("custom", "", "original", "")).toBe(false);
    expect(isValidComicTitles("arabic", "", "custom", "")).toBe(false);
    expect(isValidComicTitles("custom", "#N", "custom", "第#N话")).toBe(true);
  });
});
