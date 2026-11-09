import { describe, expect, it } from "vitest";
import {
  COMIC_IMAGES_MODES,
  COMIC_PAGE_TITLE_KINDS,
  COMIC_VOLUME_TITLE_KINDS,
  DEFAULT_COMIC_PAGE_PATTERN,
  DEFAULT_COMIC_VOLUME_PATTERN,
  canExport,
  buildComicPdfOptions,
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
    expect(canExport({ epub: false, images: false, cover: false, pdf: false }, full)).toBe(false);
    expect(canExport({ epub: true, images: false, cover: false, pdf: false }, full)).toBe(true);
    expect(canExport({ epub: true, images: false, cover: false, pdf: false }, statusWith({}))).toBe(
      false,
    );
  });

  it("PDF 仅在有解析图片时可导出", () => {
    expect(
      canExport(
        { epub: false, images: false, cover: false, pdf: true },
        statusWith({ has_content: true, page_count: 1 }),
      ),
    ).toBe(true);
    expect(
      canExport(
        { epub: false, images: false, cover: false, pdf: true },
        statusWith({ has_content: true, page_count: 0 }),
      ),
    ).toBe(false);
    expect(
      canExport(
        { epub: false, images: false, cover: false, pdf: true },
        statusWith({ has_content: false }),
      ),
    ).toBe(false);
  });

  it("PDF 平均尺寸允许宽高留空，DPI 默认设置可用", () => {
    expect(
      buildComicPdfOptions({
        page_size: "average",
        width_px: "",
        height_px: "",
        dpi: "300",
        fill_mode: "contain",
        horizontal_alignment: "center",
        vertical_alignment: "center",
        include_cover: true,
      }),
    ).toEqual({
      page_size: "average",
      width_px: 0,
      height_px: 0,
      dpi: 300,
      fill_mode: "contain",
      horizontal_alignment: "center",
      vertical_alignment: "center",
      include_cover: true,
    });
  });

  it("PDF 固定尺寸要求正整数宽高且 DPI 在 72–600 范围内", () => {
    const base = {
      page_size: "fixed" as const,
      width_px: "2480",
      height_px: "3508",
      dpi: "300",
      fill_mode: "cover" as const,
      horizontal_alignment: "right" as const,
      vertical_alignment: "top" as const,
      include_cover: false,
    };
    expect(buildComicPdfOptions(base)?.width_px).toBe(2480);
    expect(buildComicPdfOptions({ ...base, width_px: "0" })).toBeNull();
    expect(buildComicPdfOptions({ ...base, width_px: "1.5" })).toBeNull();
    expect(buildComicPdfOptions({ ...base, dpi: "71" })).toBeNull();
    expect(buildComicPdfOptions({ ...base, dpi: "601" })).toBeNull();
  });

  it("PDF 固定尺寸 14400pt 边界：300DPI 下 60000 通过、60001 拒绝", () => {
    const base = {
      page_size: "fixed" as const,
      width_px: "60000",
      height_px: "60000",
      dpi: "300",
      fill_mode: "contain" as const,
      horizontal_alignment: "center" as const,
      vertical_alignment: "center" as const,
      include_cover: false,
    };
    expect(buildComicPdfOptions(base)).not.toBeNull();
    expect(buildComicPdfOptions({ ...base, width_px: "60001" })).toBeNull();
    expect(buildComicPdfOptions({ ...base, height_px: "60001" })).toBeNull();
    // 72DPI 下边界为 14400px。
    expect(
      buildComicPdfOptions({ ...base, dpi: "72", width_px: "14400", height_px: "14400" }),
    ).not.toBeNull();
    expect(
      buildComicPdfOptions({ ...base, dpi: "72", width_px: "14401", height_px: "14400" }),
    ).toBeNull();
  });

  it("PDF 原图尺寸透传且忽略宽高", () => {
    expect(
      buildComicPdfOptions({
        page_size: "original",
        width_px: "",
        height_px: "",
        dpi: "300",
        fill_mode: "cover",
        horizontal_alignment: "right",
        vertical_alignment: "top",
        include_cover: false,
      }),
    ).toEqual({
      page_size: "original",
      width_px: 0,
      height_px: 0,
      dpi: 300,
      fill_mode: "cover",
      horizontal_alignment: "right",
      vertical_alignment: "top",
      include_cover: false,
    });
  });

  it("话 6 种页 5 种格式，页无原文", () => {
    expect(COMIC_VOLUME_TITLE_KINDS).toContain("original");
    expect(COMIC_PAGE_TITLE_KINDS).not.toContain("original");
    expect(COMIC_PAGE_TITLE_KINDS).toContain("custom");
  });

  it("自定义默认值与构建页一致：话不带原名、页为纯数字", () => {
    expect(DEFAULT_COMIC_VOLUME_PATTERN).toBe("第#N话");
    expect(DEFAULT_COMIC_PAGE_PATTERN).toBe("#N");
  });

  it("原图标题合法：话页自定义模板各自非空", () => {
    expect(isValidTitlePattern("  ")).toBe(false);
    expect(isValidImageTitles("arabic", "", "original", "")).toBe(true);
    expect(isValidImageTitles("custom", "", "original", "")).toBe(false);
    expect(isValidImageTitles("arabic", "", "custom", "  ")).toBe(false);
    expect(isValidImageTitles("custom", "#N", "custom", "第#N话")).toBe(true);
  });

  it("原图形态候选：文件夹优先", () => {
    expect(COMIC_IMAGES_MODES).toEqual(["folder", "zip"]);
  });
});
