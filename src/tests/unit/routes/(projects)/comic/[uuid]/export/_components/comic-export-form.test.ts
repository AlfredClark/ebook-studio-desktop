import { describe, expect, it } from "vitest";
import {
  COMIC_IMAGES_MODES,
  COMIC_PAGE_TITLE_KINDS,
  COMIC_VOLUME_TITLE_KINDS,
  canExport,
  isValidCustomName,
  isValidImageTitles,
  isValidTitlePattern,
  namingKinds,
  namingValue,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/export/_components/comic-export-form";
import type { ComicExportStatus } from "#libs/commands/types";

function statusWith(overrides: Partial<ComicExportStatus>): ComicExportStatus {
  return {
    has_content: false,
    volume_count: 0,
    page_count: 0,
    total_bytes: 0,
    has_epub: false,
    epub_name: null,
    has_cover: false,
    cover_ext: null,
    project_name: "雾镇",
    book_title: "雾镇",
    ...overrides,
  };
}

describe("漫画导出表单", () => {
  it("命名候选：项目名与书名相同合并", () => {
    expect(namingKinds("雾镇", "雾镇")).toEqual(["project", "custom"]);
    expect(namingKinds("雾镇", "雾镇·改")).toEqual(["project", "book", "custom"]);
    expect(namingValue("book", "雾镇", "雾镇·改")).toBe("雾镇·改");
    expect(namingValue("custom", "雾镇", "雾镇·改")).toBe("雾镇");
  });

  it("自定义名可用：去首尾空白后非空", () => {
    expect(isValidCustomName("  ")).toBe(false);
    expect(isValidCustomName(" 精选 ")).toBe(true);
  });

  it("导出可用：至少勾选一项现存产物", () => {
    const full = statusWith({ has_content: true, has_epub: true, has_cover: true });
    expect(canExport({ epub: false, images: false, cover: false }, full)).toBe(false);
    expect(canExport({ epub: true, images: false, cover: false }, full)).toBe(true);
    expect(canExport({ epub: true, images: false, cover: false }, statusWith({}))).toBe(false);
  });

  it("话 6 种页 5 种格式，页无原文", () => {
    expect(COMIC_VOLUME_TITLE_KINDS).toContain("original");
    expect(COMIC_PAGE_TITLE_KINDS).not.toContain("original");
    expect(COMIC_PAGE_TITLE_KINDS).toContain("custom");
  });

  it("原图标题合法：话页自定义模板各自非空", () => {
    expect(isValidTitlePattern("  ")).toBe(false);
    expect(isValidImageTitles("arabic", "", "original", "")).toBe(true);
    expect(isValidImageTitles("custom", "", "original", "")).toBe(false);
    expect(isValidImageTitles("arabic", "", "custom", "  ")).toBe(false);
    expect(isValidImageTitles("custom", "第#N页", "custom", "第#N话 #T")).toBe(true);
  });

  it("原图形态候选：文件夹优先", () => {
    expect(COMIC_IMAGES_MODES).toEqual(["folder", "zip"]);
  });
});
