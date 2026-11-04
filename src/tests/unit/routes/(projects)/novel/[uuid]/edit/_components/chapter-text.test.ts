import { describe, expect, it } from "vitest";
import {
  charSpanToRange,
  joinChapterLines,
  splitChapterLines,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/chapter-text";

describe("章节正文行切分", () => {
  it("按换行切分并去首尾空白、丢弃空行", () => {
    expect(splitChapterLines("  正文甲  \n\n正文乙\n   \n正文丙  ")).toEqual([
      "正文甲",
      "正文乙",
      "正文丙",
    ]);
  });

  it("兼容 CRLF 换行", () => {
    expect(splitChapterLines("正文甲\r\n\r\n正文乙")).toEqual(["正文甲", "正文乙"]);
  });

  it("空文本与纯空白切分为空数组", () => {
    expect(splitChapterLines("")).toEqual([]);
    expect(splitChapterLines("  \n \n ")).toEqual([]);
  });

  it("落盘行拼回编辑器字符串", () => {
    expect(joinChapterLines(["正文甲", "正文乙"])).toBe("正文甲\n正文乙");
    expect(joinChapterLines([])).toBe("");
  });

  it("切分与拼接往返一致", () => {
    const lines = ["第一章", "正文甲", "正文乙"];
    expect(splitChapterLines(joinChapterLines(lines))).toEqual(lines);
  });

  it("char 索引换算列偏移：中文 BMP 一致", () => {
    expect(charSpanToRange("甲乙丙甲", 0, 1)).toEqual({ from: 0, to: 1 });
    expect(charSpanToRange("甲乙丙甲", 3, 1)).toEqual({ from: 3, to: 4 });
  });

  it("超平面字符按 UTF-16 单位换算", () => {
    // "a𐀀b"：3 个字符，5 个 UTF-16 单位。
    expect(charSpanToRange("a𐀀b", 1, 1)).toEqual({ from: 1, to: 3 });
    expect(charSpanToRange("a𐀀b", 2, 1)).toEqual({ from: 3, to: 4 });
  });

  it("越界索引钳制到行范围", () => {
    expect(charSpanToRange("正文", 10, 5)).toEqual({ from: 2, to: 2 });
    expect(charSpanToRange("正文", 1, 99)).toEqual({ from: 1, to: 2 });
  });
});
