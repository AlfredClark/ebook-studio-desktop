import { describe, expect, it } from "vitest";
import {
  accumulateWheelFlip,
  normalizeWheelDelta,
  resolveArrowStep,
  WHEEL_FLIP_THRESHOLD,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-navigate";

describe("预览导航", () => {
  it("滚轮增量归一", () => {
    expect(normalizeWheelDelta(120, 0)).toBe(120);
    expect(normalizeWheelDelta(3, 1)).toBe(99);
    expect(normalizeWheelDelta(0, 1)).toBe(0);
  });

  it("同向累积超阈值翻页并清零", () => {
    const half = Math.ceil(WHEEL_FLIP_THRESHOLD / 2);
    const first = accumulateWheelFlip(0, half);
    expect(first.flip).toBe(0);
    expect(first.acc).toBe(half);
    const second = accumulateWheelFlip(first.acc, half);
    expect(second.flip).toBe(1);
    expect(second.acc).toBe(0);
    const back = accumulateWheelFlip(0, -WHEEL_FLIP_THRESHOLD);
    expect(back).toEqual({ flip: -1, acc: 0 });
  });

  it("换向重置累积", () => {
    const forward = accumulateWheelFlip(0, WHEEL_FLIP_THRESHOLD - 1);
    expect(forward.flip).toBe(0);
    const reversed = accumulateWheelFlip(forward.acc, -10);
    expect(reversed).toEqual({ flip: 0, acc: -10 });
    expect(accumulateWheelFlip(5, 0)).toEqual({ flip: 0, acc: 5 });
  });

  it("方向键映射", () => {
    const plain = { alt: false, ctrl: false, meta: false };
    expect(resolveArrowStep("ArrowRight", plain)).toBe(1);
    expect(resolveArrowStep("ArrowDown", plain)).toBe(1);
    expect(resolveArrowStep("ArrowLeft", plain)).toBe(-1);
    expect(resolveArrowStep("ArrowUp", plain)).toBe(-1);
    expect(resolveArrowStep("Enter", plain)).toBe(0);
    // 修饰键一律让路（Alt 留给重排、Ctrl/Meta 留给系统）。
    expect(resolveArrowStep("ArrowUp", { ...plain, alt: true })).toBe(0);
    expect(resolveArrowStep("ArrowDown", { ...plain, ctrl: true })).toBe(0);
    expect(resolveArrowStep("ArrowLeft", { ...plain, meta: true })).toBe(0);
  });
});
