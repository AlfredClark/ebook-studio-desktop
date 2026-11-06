import { describe, expect, it } from "vitest";
import {
  findPage,
  flattenPages,
  movePage,
  moveVolume,
  sameOrder,
  toWorking,
  validateVolumeTitle,
  type WorkingVolume,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-order";

function volumes(): WorkingVolume[] {
  return [
    { title: null, pages: ["a.jpg", "b.jpg", "c.jpg"] },
    { title: "第1卷", pages: ["第1卷/01.jpg", "第1卷/02.jpg"] },
  ];
}

describe("漫画页序", () => {
  it("大纲派生工作卷并可比对脏态", () => {
    const working = toWorking([
      { title: null, pages: [{ path: "a.jpg" }] },
      { title: "第1卷", pages: [{ path: "第1卷/01.jpg" }] },
    ]);
    expect(working).toEqual([
      { title: null, pages: ["a.jpg"] },
      { title: "第1卷", pages: ["第1卷/01.jpg"] },
    ]);
    expect(sameOrder(working, toWorking([]))).toBe(false);
    expect(sameOrder(working, structuredClone(working))).toBe(true);
  });

  it("按 path 定位与展平", () => {
    expect(findPage(volumes(), "第1卷/02.jpg")).toEqual({ volume: 1, index: 1 });
    expect(findPage(volumes(), "missing.jpg")).toBeNull();
    expect(flattenPages(volumes())).toEqual([
      "a.jpg",
      "b.jpg",
      "c.jpg",
      "第1卷/01.jpg",
      "第1卷/02.jpg",
    ]);
  });

  it("卷内换位", () => {
    const moved = movePage(
      volumes(),
      { volume: 0, index: 0 },
      { volume: 0, index: 2, edge: "after" },
    );
    expect(moved[0].pages).toEqual(["b.jpg", "c.jpg", "a.jpg"]);
    const up = movePage(
      volumes(),
      { volume: 0, index: 2 },
      { volume: 0, index: 0, edge: "before" },
    );
    expect(up[0].pages).toEqual(["c.jpg", "a.jpg", "b.jpg"]);
    // 原数组不受影响。
    expect(volumes()[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg"]);
  });

  it("跨卷移动与卷末追加", () => {
    const moved = movePage(
      volumes(),
      { volume: 0, index: 0 },
      { volume: 1, index: 0, edge: "before" },
    );
    expect(moved[0].pages).toEqual(["b.jpg", "c.jpg"]);
    expect(moved[1].pages).toEqual(["a.jpg", "第1卷/01.jpg", "第1卷/02.jpg"]);
    const appended = movePage(volumes(), { volume: 1, index: 1 }, { volume: 0, edge: "append" });
    expect(appended[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg", "第1卷/02.jpg"]);
    expect(appended[1].pages).toEqual(["第1卷/01.jpg"]);
  });

  it("非法引用直接返回原数组", () => {
    const base = volumes();
    expect(movePage(base, { volume: 5, index: 0 }, { volume: 0, edge: "append" })).toBe(base);
    expect(movePage(base, { volume: 0, index: 9 }, { volume: 0, edge: "append" })).toBe(base);
    expect(movePage(base, { volume: 0, index: 0 }, { volume: 9, edge: "append" })).toBe(base);
  });

  it("移动卷", () => {
    const moved = moveVolume(volumes(), 0, { index: 1, edge: "after" });
    expect(moved.map((volume) => volume.title)).toEqual(["第1卷", null]);
    expect(moved[0].pages).toEqual(["第1卷/01.jpg", "第1卷/02.jpg"]);
    const up = moveVolume(volumes(), 1, { index: 0, edge: "before" });
    expect(up.map((volume) => volume.title)).toEqual(["第1卷", null]);
    // 原数组不受影响。
    expect(volumes().map((volume) => volume.title)).toEqual([null, "第1卷"]);
    // 非法下标返回原数组。
    const base = volumes();
    expect(moveVolume(base, 5, { index: 0, edge: "before" })).toBe(base);
    expect(moveVolume(base, 0, { index: 9, edge: "before" })).toBe(base);
  });

  it("校验卷标题", () => {
    expect(validateVolumeTitle("  ", ["第1卷"])).toBe("empty");
    expect(validateVolumeTitle("新卷", ["新卷", "第1卷"])).toBe("duplicate");
    expect(validateVolumeTitle(" 新卷 ", ["新卷"])).toBe("duplicate");
    expect(validateVolumeTitle("卷".repeat(51), [])).toBe("too_long");
    expect(validateVolumeTitle("番外", ["第1卷"])).toBeNull();
  });
});
