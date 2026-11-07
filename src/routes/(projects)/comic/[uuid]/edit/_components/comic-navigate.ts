// 漫画预览导航纯逻辑：滚轮累积翻页 + 方向键映射；手势接线在 +page.svelte，判定可单测。

/** 滚轮翻页阈值：同向累积超此像素翻一页（鼠标一格约 100px+，触控板需一段滑动）。 */
export const WHEEL_FLIP_THRESHOLD = 60;

/** 行模式滚轮归一系数：Firefox 行模式多为小整数，折算成像素量级。 */
export const WHEEL_LINE_TO_PX = 33;

/** 滚轮增量归一：行模式转像素，其余原样透传。 */
export function normalizeWheelDelta(delta: number, deltaMode: number): number {
  return deltaMode === 1 ? delta * WHEEL_LINE_TO_PX : delta;
}

/**
 * 滚轮翻页累积：同向累积、换向重置，超阈值触发一次并清零。 调用方按 flip 调 stepPage（1 下一页、-1 上一页、0 不动），acc
 * 回存待下次输入。
 */
export function accumulateWheelFlip(acc: number, delta: number): { flip: 0 | 1 | -1; acc: number } {
  if (delta === 0) {
    return { flip: 0, acc };
  }
  if (acc !== 0 && Math.sign(delta) !== Math.sign(acc)) {
    acc = 0;
  }
  const next = acc + delta;
  if (next >= WHEEL_FLIP_THRESHOLD) {
    return { flip: 1, acc: 0 };
  }
  if (next <= -WHEEL_FLIP_THRESHOLD) {
    return { flip: -1, acc: 0 };
  }
  return { flip: 0, acc: next };
}

/** 方向键翻页映射：上下左右→±1；带修饰键（Alt 重排等）一律让路。 */
export function resolveArrowStep(
  key: string,
  modifiers: { alt: boolean; ctrl: boolean; meta: boolean },
): 1 | -1 | 0 {
  if (modifiers.alt || modifiers.ctrl || modifiers.meta) {
    return 0;
  }
  if (key === "ArrowRight" || key === "ArrowDown") {
    return 1;
  }
  if (key === "ArrowLeft" || key === "ArrowUp") {
    return -1;
  }
  return 0;
}
