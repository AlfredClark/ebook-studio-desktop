import { describe, expect, it } from "vitest";
import {
  applyDomOrder,
  cloneVolumes,
  createVolume,
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
    { id: "v0", title: null, pages: ["a.jpg", "b.jpg", "c.jpg"] },
    { id: "v1", title: "第1话", pages: ["第1话/01.jpg", "第1话/02.jpg"] },
  ];
}

describe("漫画页序", () => {
  it("大纲派生工作话并可比对脏态", () => {
    const working = toWorking([
      { title: null, pages: [{ path: "a.jpg" }] },
      { title: "第1话", pages: [{ path: "第1话/01.jpg" }] },
    ]);
    expect(working).toEqual([
      { id: "volume-0", title: null, pages: ["a.jpg"] },
      { id: "volume-1", title: "第1话", pages: ["第1话/01.jpg"] },
    ]);
    expect(sameOrder(working, toWorking([]))).toBe(false);
    expect(sameOrder(working, structuredClone(working))).toBe(true);
  });

  it("脏态比对忽略内存 id", () => {
    const working = toWorking([{ title: null, pages: [{ path: "a.jpg" }] }]);
    const renamed = cloneVolumes(working).map((volume) => ({ ...volume, id: "other" }));
    expect(sameOrder(working, renamed)).toBe(true);
    const reordered = movePage(working, { volume: 0, index: 0 }, { volume: 0, edge: "append" });
    expect(sameOrder(working, reordered)).toBe(true);
  });

  it("按 path 定位与展平", () => {
    expect(findPage(volumes(), "第1话/02.jpg")).toEqual({ volume: 1, index: 1 });
    expect(findPage(volumes(), "missing.jpg")).toBeNull();
    expect(flattenPages(volumes())).toEqual([
      "a.jpg",
      "b.jpg",
      "c.jpg",
      "第1话/01.jpg",
      "第1话/02.jpg",
    ]);
  });

  it("新建空话与深拷贝快照", () => {
    const first = createVolume("番外");
    const second = createVolume("番外");
    expect(first.title).toBe("番外");
    expect(first.pages).toEqual([]);
    expect(first.id).not.toBe(second.id);
    const snapshot = cloneVolumes(volumes());
    snapshot[0].pages.push("x.jpg");
    expect(volumes()[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg"]);
  });

  it("话内换位且 id 跟随", () => {
    const moved = movePage(
      volumes(),
      { volume: 0, index: 0 },
      { volume: 0, index: 2, edge: "after" },
    );
    expect(moved[0].pages).toEqual(["b.jpg", "c.jpg", "a.jpg"]);
    expect(moved.map((volume) => volume.id)).toEqual(["v0", "v1"]);
    const up = movePage(
      volumes(),
      { volume: 0, index: 2 },
      { volume: 0, index: 0, edge: "before" },
    );
    expect(up[0].pages).toEqual(["c.jpg", "a.jpg", "b.jpg"]);
    // 原数组不受影响。
    expect(volumes()[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg"]);
  });

  it("跨话移动与话末追加", () => {
    const moved = movePage(
      volumes(),
      { volume: 0, index: 0 },
      { volume: 1, index: 0, edge: "before" },
    );
    expect(moved[0].pages).toEqual(["b.jpg", "c.jpg"]);
    expect(moved[1].pages).toEqual(["a.jpg", "第1话/01.jpg", "第1话/02.jpg"]);
    const appended = movePage(volumes(), { volume: 1, index: 1 }, { volume: 0, edge: "append" });
    expect(appended[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg", "第1话/02.jpg"]);
    expect(appended[1].pages).toEqual(["第1话/01.jpg"]);
  });

  it("按落点 DOM 顺序重建", () => {
    const base = volumes();
    // 话内换位。
    const moved = applyDomOrder(
      base,
      ["v0", "v1"],
      new Map([
        ["v0", ["b.jpg", "c.jpg", "a.jpg"]],
        ["v1", ["第1话/01.jpg", "第1话/02.jpg"]],
      ]),
    );
    expect(moved[0].pages).toEqual(["b.jpg", "c.jpg", "a.jpg"]);
    expect(moved.map((volume) => volume.id)).toEqual(["v0", "v1"]);
    // 跨话搬移 + 话排序。
    const cross = applyDomOrder(
      base,
      ["v1", "v0"],
      new Map([
        ["v1", ["第1话/01.jpg", "a.jpg", "第1话/02.jpg"]],
        ["v0", ["b.jpg", "c.jpg"]],
      ]),
    );
    expect(cross.map((volume) => volume.id)).toEqual(["v1", "v0"]);
    expect(cross[0].pages).toEqual(["第1话/01.jpg", "a.jpg", "第1话/02.jpg"]);
    expect(cross[1].pages).toEqual(["b.jpg", "c.jpg"]);
    // 折叠话无 body 表时保持原序；未知 path 丢弃；漏 path 缀回原话末尾。
    const partial = applyDomOrder(base, ["v0", "v1"], new Map([["v0", ["b.jpg", "x.jpg"]]]));
    expect(partial[0].pages).toEqual(["b.jpg", "a.jpg", "c.jpg"]);
    expect(partial[1].pages).toEqual(["第1话/01.jpg", "第1话/02.jpg"]);
    // 原数组不受影响。
    expect(base[0].pages).toEqual(["a.jpg", "b.jpg", "c.jpg"]);
  });

  it("非法引用直接返回原数组", () => {
    const base = volumes();
    expect(movePage(base, { volume: 5, index: 0 }, { volume: 0, edge: "append" })).toBe(base);
    expect(movePage(base, { volume: 0, index: 9 }, { volume: 0, edge: "append" })).toBe(base);
    expect(movePage(base, { volume: 0, index: 0 }, { volume: 9, edge: "append" })).toBe(base);
  });

  it("移动话且 id 跟随", () => {
    const moved = moveVolume(volumes(), 0, { index: 1, edge: "after" });
    expect(moved.map((volume) => volume.title)).toEqual(["第1话", null]);
    expect(moved.map((volume) => volume.id)).toEqual(["v1", "v0"]);
    expect(moved[0].pages).toEqual(["第1话/01.jpg", "第1话/02.jpg"]);
    const up = moveVolume(volumes(), 1, { index: 0, edge: "before" });
    expect(up.map((volume) => volume.title)).toEqual(["第1话", null]);
    // 原数组不受影响。
    expect(volumes().map((volume) => volume.title)).toEqual([null, "第1话"]);
    // 非法下标返回原数组。
    const base = volumes();
    expect(moveVolume(base, 5, { index: 0, edge: "before" })).toBe(base);
    expect(moveVolume(base, 0, { index: 9, edge: "before" })).toBe(base);
  });

  it("校验话标题", () => {
    expect(validateVolumeTitle("  ", ["第1话"])).toBe("empty");
    expect(validateVolumeTitle("新话", ["新话", "第1话"])).toBe("duplicate");
    expect(validateVolumeTitle(" 新话 ", ["新话"])).toBe("duplicate");
    expect(validateVolumeTitle("话".repeat(51), [])).toBe("too_long");
    expect(validateVolumeTitle("番外", ["第1话"])).toBeNull();
  });
});
