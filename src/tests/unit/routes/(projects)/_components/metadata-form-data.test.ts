import { describe, expect, it } from "vitest";
import {
  formatSubjectsInput,
  parseSubjectsInput,
} from "../../../../../routes/(projects)/_components/metadata-form-data";

describe("主题词输入", () => {
  it("按空格逗号竖线切分并去重", () => {
    expect(parseSubjectsInput("玄幻  仙侠,武侠|玄幻，推理")).toEqual([
      "玄幻",
      "仙侠",
      "武侠",
      "推理",
    ]);
  });

  it("空输入解析为空数组", () => {
    expect(parseSubjectsInput("  ,，|| ")).toEqual([]);
  });

  it("回填为单行逗号分隔", () => {
    expect(formatSubjectsInput(["玄幻", "仙侠"])).toBe("玄幻, 仙侠");
  });

  it("解析与回填互逆", () => {
    const subjects = ["玄幻", "仙侠", "武侠"];
    expect(parseSubjectsInput(formatSubjectsInput(subjects))).toEqual(subjects);
  });
});
