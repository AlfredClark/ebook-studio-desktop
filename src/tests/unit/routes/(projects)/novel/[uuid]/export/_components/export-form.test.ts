import { describe, expect, it } from "vitest";
import {
  canExport,
  isValidCustomName,
  isValidTitlePattern,
  isValidTxtOptions,
  namingKinds,
  namingValue,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/export/_components/export-form";
import type { ExportStatus } from "#libs/commands/types";

function statusWith(overrides: Partial<ExportStatus>): ExportStatus {
  return {
    has_content: false,
    content: null,
    has_epub: false,
    epub_name: null,
    has_cover: false,
    cover_ext: null,
    project_name: "雾镇",
    book_title: "雾镇",
    ...overrides,
  };
}

describe("导出表单", () => {
  it("项目名与书名相同合并命名选项", () => {
    expect(namingKinds("雾镇", "雾镇")).toEqual(["project", "custom"]);
    expect(namingKinds("雾镇", "雾镇别传")).toEqual(["project", "book", "custom"]);
  });

  it("命名取值按来源回落", () => {
    expect(namingValue("project", "雾镇", "雾镇别传")).toBe("雾镇");
    expect(namingValue("book", "雾镇", "雾镇别传")).toBe("雾镇别传");
    expect(namingValue("custom", "雾镇", "雾镇别传")).toBe("雾镇");
  });

  it("自定义名去空白后非空才可用", () => {
    expect(isValidCustomName(" 雾镇 ")).toBe(true);
    expect(isValidCustomName("   ")).toBe(false);
    expect(isValidCustomName("")).toBe(false);
  });

  it("至少勾选一项现存产物才可导出", () => {
    const status = statusWith({ has_content: true, has_epub: true });
    expect(canExport({ epub: false, txt: false, cover: false }, status)).toBe(false);
    expect(canExport({ epub: true, txt: false, cover: false }, status)).toBe(true);
    // 勾选了不存在的产物不算数。
    expect(canExport({ epub: false, txt: false, cover: true }, status)).toBe(false);
    expect(canExport({ epub: false, txt: false, cover: false }, statusWith({}))).toBe(false);
  });

  it("TXT 参数合法性", () => {
    expect(isValidTitlePattern("第 #c 章")).toBe(true);
    expect(isValidTitlePattern("   ")).toBe(false);
    expect(isValidTxtOptions("original", "", "original", "", false, [])).toBe(true);
    expect(isValidTxtOptions("custom", "", "original", "", false, [])).toBe(false);
    expect(isValidTxtOptions("custom", "第 #n 章", "original", "", false, [])).toBe(true);
    expect(isValidTxtOptions("original", "", "custom", "", false, [])).toBe(false);
    expect(isValidTxtOptions("original", "", "custom", "第 #n 卷", false, [])).toBe(true);
    expect(isValidTxtOptions("original", "", "original", "", true, [])).toBe(false);
    expect(isValidTxtOptions("original", "", "original", "", true, ["creator"])).toBe(true);
  });
});
