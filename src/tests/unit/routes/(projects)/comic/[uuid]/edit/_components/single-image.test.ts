import { describe, expect, it } from "vitest";
import {
  buildSingleOptions,
  defaultSingleForm,
  isSingleDefault,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/single-image";

describe("单图处理表单", () => {
  it("默认值直通且可组装", () => {
    const values = defaultSingleForm();
    expect(isSingleDefault(values)).toBe(true);
    const options = buildSingleOptions(values);
    expect(options).not.toBeNull();
    expect(options?.rotate_deg).toBe(0);
    expect(options?.scale_percent).toBe(100);
  });

  it("任一变换即非直通", () => {
    expect(isSingleDefault({ ...defaultSingleForm(), rotate_deg: 90 })).toBe(false);
    expect(isSingleDefault({ ...defaultSingleForm(), flip_h: true })).toBe(false);
    expect(isSingleDefault({ ...defaultSingleForm(), auto_trim: true })).toBe(false);
    expect(isSingleDefault({ ...defaultSingleForm(), brightness: 1 })).toBe(false);
    expect(isSingleDefault({ ...defaultSingleForm(), grayscale: true })).toBe(false);
    expect(isSingleDefault({ ...defaultSingleForm(), scale_percent: 50 })).toBe(false);
  });

  it("非法旋转与越界滑块返回空值", () => {
    expect(buildSingleOptions({ ...defaultSingleForm(), rotate_deg: 45 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), scale_percent: 9 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), scale_percent: 301 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), brightness: 101 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), contrast: -101 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), quality: 0 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), png_level: 10 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), trim_threshold: 0 })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), trim_margin: "501" })).toBeNull();
    expect(buildSingleOptions({ ...defaultSingleForm(), trim_margin: "abc" })).toBeNull();
  });
});
