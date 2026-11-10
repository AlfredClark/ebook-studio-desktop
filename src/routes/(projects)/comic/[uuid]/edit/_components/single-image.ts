// 单图处理表单纯逻辑：默认值、参数组装与合法性判定；落盘规则以后端为准。
import type { SingleImageOptions } from "#libs/commands/types";

/** 旋转候选项：固定四挡，下拉直接用。 */
export const SINGLE_ROTATIONS = [0, 90, 180, 270] as const;

/** 旋转角度：仅允许四挡。 */
export type SingleRotation = (typeof SINGLE_ROTATIONS)[number];

/** 单图表单字段：数字输入以字符串接收原始输入，滑块值为数字。 */
export interface SingleFormValues {
  rotate_deg: number;
  flip_h: boolean;
  flip_v: boolean;
  auto_trim: boolean;
  trim_threshold: number;
  trim_margin: string;
  brightness: number;
  contrast: number;
  grayscale: boolean;
  scale_percent: number;
  quality: number;
  webp_lossless: boolean;
  png_level: number;
}

/** 表单默认值：全直通（预览与源一致）。 */
export function defaultSingleForm(): SingleFormValues {
  return {
    rotate_deg: 0,
    flip_h: false,
    flip_v: false,
    auto_trim: false,
    trim_threshold: 10,
    trim_margin: "0",
    brightness: 0,
    contrast: 0,
    grayscale: false,
    scale_percent: 100,
    quality: 85,
    webp_lossless: false,
    png_level: 6,
  };
}

/** 是否直通：全默认时应用无意义，调用方禁用执行。 */
export function isSingleDefault(values: SingleFormValues): boolean {
  return (
    values.rotate_deg === 0 &&
    !values.flip_h &&
    !values.flip_v &&
    !values.auto_trim &&
    values.brightness === 0 &&
    values.contrast === 0 &&
    !values.grayscale &&
    values.scale_percent === 100
  );
}

/** 解析非负整数输入；非法返回空值。 */
function parseNonNegativeInt(text: string): number | null {
  if (!/^\d+$/.test(text.trim())) {
    return null;
  }
  const value = Number(text.trim());
  return Number.isSafeInteger(value) ? value : null;
}

/** 单图参数可用时返回命令参数；非法输入返回空值并禁用执行。 */
export function buildSingleOptions(values: SingleFormValues): SingleImageOptions | null {
  if (!(SINGLE_ROTATIONS as readonly number[]).includes(values.rotate_deg)) {
    return null;
  }
  if (
    !Number.isInteger(values.trim_threshold) ||
    values.trim_threshold < 1 ||
    values.trim_threshold > 100 ||
    !Number.isInteger(values.brightness) ||
    values.brightness < -100 ||
    values.brightness > 100 ||
    !Number.isInteger(values.contrast) ||
    values.contrast < -100 ||
    values.contrast > 100 ||
    !Number.isInteger(values.scale_percent) ||
    values.scale_percent < 10 ||
    values.scale_percent > 300 ||
    !Number.isInteger(values.quality) ||
    values.quality < 1 ||
    values.quality > 100 ||
    !Number.isInteger(values.png_level) ||
    values.png_level < 0 ||
    values.png_level > 9
  ) {
    return null;
  }
  const trimMargin = parseNonNegativeInt(values.trim_margin);
  if (trimMargin === null || trimMargin > 500) {
    return null;
  }
  return {
    rotate_deg: values.rotate_deg,
    flip_h: values.flip_h,
    flip_v: values.flip_v,
    auto_trim: values.auto_trim,
    trim_threshold: values.trim_threshold,
    trim_margin: trimMargin,
    brightness: values.brightness,
    contrast: values.contrast,
    grayscale: values.grayscale,
    scale_percent: values.scale_percent,
    quality: values.quality,
    webp_lossless: values.webp_lossless,
    png_level: values.png_level,
  };
}
