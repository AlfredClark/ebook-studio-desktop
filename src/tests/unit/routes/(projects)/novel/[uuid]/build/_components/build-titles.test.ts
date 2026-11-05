import { describe, expect, it } from "vitest";
import {
  BUILD_TITLE_KINDS,
  DEFAULT_BUILD_CHAPTER_PATTERN,
  DEFAULT_BUILD_VOLUME_PATTERN,
  isValidBuildTitlePattern,
  isValidBuildTitles,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/build/_components/build-titles";

describe("构建标题格式", () => {
  it("候选固定顺序且默认模板非空", () => {
    expect(BUILD_TITLE_KINDS).toEqual([
      "original",
      "arabic",
      "padded",
      "chinese_lower",
      "chinese_upper",
      "custom",
    ]);
    expect(isValidBuildTitlePattern(DEFAULT_BUILD_VOLUME_PATTERN)).toBe(true);
    expect(isValidBuildTitlePattern(DEFAULT_BUILD_CHAPTER_PATTERN)).toBe(true);
    expect(isValidBuildTitlePattern("   ")).toBe(false);
  });

  it("卷章自定义模板各自校验", () => {
    expect(isValidBuildTitles("original", "", "original", "")).toBe(true);
    expect(isValidBuildTitles("custom", "", "original", "")).toBe(false);
    expect(isValidBuildTitles("custom", "第 #n 章", "original", "")).toBe(true);
    expect(isValidBuildTitles("original", "", "custom", "")).toBe(false);
    expect(isValidBuildTitles("original", "", "custom", "第 #n 卷")).toBe(true);
  });
});
