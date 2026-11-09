// 漫画尺寸调整表单纯逻辑：目标值校验与条件显隐判定；落盘规则以后端为准。
import type {
  ComicPdfHorizontalAlignment,
  ComicPdfVerticalAlignment,
  ResizeBackground,
  ResizeFill,
  ResizeMode,
  ResizeOptions,
} from "#libs/commands/types";

const MAX_RESIZE_PIXELS = 50_000_000;

/** 固定尺寸默认值：与 PDF 固定画布一致（3:4 竖构图）。 */
export const DEFAULT_RESIZE_WIDTH_PX = "1860";
export const DEFAULT_RESIZE_HEIGHT_PX = "2480";

/** 宽高比例锁定：`free` 为不锁定，其余为 `宽:高`； 与导出 PDF 的同名逻辑同语义（分属不同页面模块，各自维护，行为保持一致）。 */
export type ResizeAspectLock = "free" | "1:1" | "3:4" | "4:3" | "2:3" | "3:2" | "9:16" | "16:9";

/** 比例锁定候选项（固定顺序）：不锁定首位。 */
export const RESIZE_ASPECT_LOCKS: ResizeAspectLock[] = [
  "free",
  "1:1",
  "3:4",
  "4:3",
  "2:3",
  "3:2",
  "9:16",
  "16:9",
];

/** 解析比例锁定为宽高比；不锁定或非法返回空值。 */
export function parseResizeAspectRatio(lock: ResizeAspectLock): [number, number] | null {
  if (lock === "free") {
    return null;
  }
  const [width, height] = lock.split(":").map(Number);
  if (
    width === undefined ||
    height === undefined ||
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width <= 0 ||
    height <= 0
  ) {
    return null;
  }
  return [width, height];
}

/** 按锁定比例由宽度换算高度；不锁定或宽度非法时返回空字符串。 */
export function linkedResizeHeight(widthPx: string, lock: ResizeAspectLock): string {
  const ratio = parseResizeAspectRatio(lock);
  const width = Number(widthPx);
  if (ratio === null || !Number.isFinite(width) || width <= 0) {
    return "";
  }
  return String(Math.max(1, Math.round((width * ratio[1]) / ratio[0])));
}

/** 按锁定比例由高度换算宽度；不锁定或高度非法时返回空字符串。 */
export function linkedResizeWidth(heightPx: string, lock: ResizeAspectLock): string {
  const ratio = parseResizeAspectRatio(lock);
  const height = Number(heightPx);
  if (ratio === null || !Number.isFinite(height) || height <= 0) {
    return "";
  }
  return String(Math.max(1, Math.round((height * ratio[0]) / ratio[1])));
}

/** 尺寸调整表单字段：宽高以字符串接收原始输入，滑块值为数字。 */
export type ResizeFormValues = {
  mode: ResizeMode;
  width_px: string;
  height_px: string;
  fill: ResizeFill;
  background: ResizeBackground;
  horizontal_alignment: ComicPdfHorizontalAlignment;
  vertical_alignment: ComicPdfVerticalAlignment;
  no_upscale: boolean;
  quality: number;
  webp_lossless: boolean;
  png_level: number;
};

/** 仅缩小开关可用：铺满/拉伸按定义必重采样，后端忽略该开关。 */
export function isNoUpscaleEnabled(mode: ResizeMode, fill: ResizeFill): boolean {
  return mode !== "fixed" || fill === "contain";
}

/** 尺寸调整参数可用时返回命令参数；非法输入返回空值并禁用执行。 */
export function buildResizeOptions(values: ResizeFormValues): ResizeOptions | null {
  if (
    !Number.isInteger(values.quality) ||
    values.quality < 1 ||
    values.quality > 100 ||
    !Number.isInteger(values.png_level) ||
    values.png_level < 0 ||
    values.png_level > 9
  ) {
    return null;
  }

  let widthPx = 0;
  let heightPx = 0;
  switch (values.mode) {
    case "width":
    case "long_edge": {
      widthPx = Number(values.width_px);
      if (!Number.isInteger(widthPx) || widthPx <= 0) {
        return null;
      }
      break;
    }
    case "height": {
      heightPx = Number(values.height_px);
      if (!Number.isInteger(heightPx) || heightPx <= 0) {
        return null;
      }
      break;
    }
    case "fixed": {
      widthPx = Number(values.width_px);
      heightPx = Number(values.height_px);
      if (
        !Number.isInteger(widthPx) ||
        !Number.isInteger(heightPx) ||
        widthPx <= 0 ||
        heightPx <= 0 ||
        widthPx * heightPx > MAX_RESIZE_PIXELS
      ) {
        return null;
      }
      break;
    }
  }

  return {
    mode: values.mode,
    width_px: widthPx,
    height_px: heightPx,
    fill: values.fill,
    background: values.background,
    horizontal_alignment: values.horizontal_alignment,
    vertical_alignment: values.vertical_alignment,
    no_upscale: values.no_upscale,
    quality: values.quality,
    webp_lossless: values.webp_lossless,
    png_level: values.png_level,
  };
}
