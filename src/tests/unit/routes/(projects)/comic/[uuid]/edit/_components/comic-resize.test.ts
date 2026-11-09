import { describe, expect, it } from "vitest";
import {
  DEFAULT_RESIZE_HEIGHT_PX,
  DEFAULT_RESIZE_WIDTH_PX,
  RESIZE_ASPECT_LOCKS,
  buildResizeOptions,
  isNoUpscaleEnabled,
  linkedResizeHeight,
  linkedResizeWidth,
  parseResizeAspectRatio,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-resize";
import type { ResizeFormValues } from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-resize";

function base(): ResizeFormValues {
  return {
    mode: "width",
    width_px: "1860",
    height_px: "2480",
    fill: "contain",
    background: "white",
    horizontal_alignment: "center",
    vertical_alignment: "center",
    no_upscale: true,
    quality: 85,
    webp_lossless: false,
    png_level: 6,
  };
}

describe("尺寸调整表单", () => {
  it("默认值可直接导出", () => {
    expect(DEFAULT_RESIZE_WIDTH_PX).toBe("1860");
    expect(DEFAULT_RESIZE_HEIGHT_PX).toBe("2480");
    expect(buildResizeOptions(base())).toEqual({
      mode: "width",
      width_px: 1860,
      height_px: 0,
      fill: "contain",
      background: "white",
      horizontal_alignment: "center",
      vertical_alignment: "center",
      no_upscale: true,
      quality: 85,
      webp_lossless: false,
      png_level: 6,
    });
  });

  it("各模式必填字段缺失返回空值", () => {
    expect(buildResizeOptions({ ...base(), width_px: "0" })).toBeNull();
    expect(buildResizeOptions({ ...base(), width_px: "1.5" })).toBeNull();
    expect(buildResizeOptions({ ...base(), mode: "height", height_px: "" })).toBeNull();
    expect(buildResizeOptions({ ...base(), mode: "long_edge", width_px: "" })).toBeNull();
    expect(
      buildResizeOptions({ ...base(), mode: "fixed", width_px: "60000", height_px: "60000" }),
    ).toBeNull();
    expect(buildResizeOptions({ ...base(), quality: 0 })).toBeNull();
    expect(buildResizeOptions({ ...base(), quality: 101 })).toBeNull();
    expect(buildResizeOptions({ ...base(), png_level: 10 })).toBeNull();
  });

  it("固定尺寸边界内可用", () => {
    expect(
      buildResizeOptions({ ...base(), mode: "fixed", width_px: "60000", height_px: "1" }),
    ).not.toBeNull();
  });

  it("仅缩小开关可用性：铺满/拉伸恒禁用", () => {
    expect(isNoUpscaleEnabled("width", "contain")).toBe(true);
    expect(isNoUpscaleEnabled("height", "cover")).toBe(true);
    expect(isNoUpscaleEnabled("long_edge", "stretch")).toBe(true);
    expect(isNoUpscaleEnabled("fixed", "contain")).toBe(true);
    expect(isNoUpscaleEnabled("fixed", "cover")).toBe(false);
    expect(isNoUpscaleEnabled("fixed", "stretch")).toBe(false);
  });

  it("比例锁定：宽高按所选比例联动换算", () => {
    expect(RESIZE_ASPECT_LOCKS[0]).toBe("free");
    expect(RESIZE_ASPECT_LOCKS).toContain("3:4");
    expect(parseResizeAspectRatio("free")).toBeNull();
    expect(parseResizeAspectRatio("3:4")).toEqual([3, 4]);
    expect(linkedResizeHeight("1860", "3:4")).toBe("2480");
    expect(linkedResizeWidth("2480", "3:4")).toBe("1860");
    expect(linkedResizeHeight("1860", "16:9")).toBe("1046");
    expect(linkedResizeWidth("1046", "16:9")).toBe("1860");
    expect(linkedResizeHeight("1860", "free")).toBe("");
    expect(linkedResizeWidth("2480", "free")).toBe("");
    expect(linkedResizeHeight("", "3:4")).toBe("");
    expect(linkedResizeHeight("-10", "3:4")).toBe("");
    expect(linkedResizeWidth("abc", "1:1")).toBe("");
  });
});
