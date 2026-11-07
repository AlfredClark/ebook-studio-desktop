import { describe, expect, it } from "vitest";
import type { ProjectInfo } from "#libs/commands/types";
import {
  computeSelectAllState,
  filterAndSortProjects,
  removeProjects,
  toggleAllChecked,
  toggleCheckedId,
} from "../../../../../../routes/(main)/projects/_components/project-data";

/** 后端契约形状的本地固件：时间错开便于看出排序效果。 */
const FIXTURES: ProjectInfo[] = [
  {
    id: "11111111-1111-4111-8111-111111111111",
    name: "星海长歌",
    type: "novel",
    created_at: "2026-09-01T08:00:00Z",
    updated_at: "2026-10-28T14:20:00Z",
  },
  {
    id: "22222222-2222-4222-8222-222222222222",
    name: "雾镇",
    type: "comic",
    created_at: "2026-08-14T10:30:00Z",
    updated_at: "2026-10-30T09:12:00Z",
  },
  {
    id: "33333333-3333-4333-8333-333333333333",
    name: "长夜将明",
    type: "novel",
    created_at: "2026-07-02T16:45:00Z",
    updated_at: "2026-10-11T20:05:00Z",
  },
  {
    id: "44444444-4444-4444-8444-444444444444",
    name: "纸飞机",
    type: "comic",
    created_at: "2026-10-05T12:00:00Z",
    updated_at: "2026-10-25T18:40:00Z",
  },
];

describe("filterAndSortProjects", () => {
  it("默认按更新时间倒序", () => {
    const visible = filterAndSortProjects(FIXTURES, {
      query: "",
      sortKey: "updated_desc",
      filter: "all",
    });
    expect(visible.map((project) => project.id)).toEqual([
      "22222222-2222-4222-8222-222222222222",
      "11111111-1111-4111-8111-111111111111",
      "44444444-4444-4444-8444-444444444444",
      "33333333-3333-4333-8333-333333333333",
    ]);
  });

  it("按类型筛选", () => {
    const visible = filterAndSortProjects(FIXTURES, {
      query: "",
      sortKey: "updated_desc",
      filter: "novel",
    });
    expect(visible.every((project) => project.type === "novel")).toBe(true);
    expect(visible).toHaveLength(2);
  });

  it("名称子串大小写不敏感且去首尾空格", () => {
    const visible = filterAndSortProjects(FIXTURES, {
      query: "  星海 ",
      sortKey: "updated_desc",
      filter: "all",
    });
    expect(visible.map((project) => project.name)).toEqual(["星海长歌"]);
  });

  it("按创建时间倒序", () => {
    const visible = filterAndSortProjects(FIXTURES, {
      query: "",
      sortKey: "created_desc",
      filter: "all",
    });
    expect(visible[0].id).toBe("44444444-4444-4444-8444-444444444444");
  });

  it("更新/创建时间支持正序", () => {
    const updatedAsc = filterAndSortProjects(FIXTURES, {
      query: "",
      sortKey: "updated_asc",
      filter: "all",
    });
    expect(updatedAsc[0].id).toBe("33333333-3333-4333-8333-333333333333");
    const createdAsc = filterAndSortProjects(FIXTURES, {
      query: "",
      sortKey: "created_asc",
      filter: "all",
    });
    expect(createdAsc[0].id).toBe("33333333-3333-4333-8333-333333333333");
  });

  it("名称支持降序", () => {
    const ascii: ProjectInfo[] = [
      {
        id: "a",
        name: "apple",
        type: "novel",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
      {
        id: "b",
        name: "banana",
        type: "novel",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ];
    const desc = filterAndSortProjects(ascii, {
      query: "",
      sortKey: "name_desc",
      filter: "all",
      locale: "en",
    });
    expect(desc.map((project) => project.name)).toEqual(["banana", "apple"]);
  });

  it("空输入返回空数组", () => {
    expect(
      filterAndSortProjects([], { query: "", sortKey: "updated_desc", filter: "all" }),
    ).toEqual([]);
  });
});

describe("computeSelectAllState", () => {
  it("空列表：未选中且禁用", () => {
    expect(computeSelectAllState([], [])).toEqual({
      checked: false,
      indeterminate: false,
      disabled: true,
    });
  });

  it("全选 / 半选 / 全不选三态", () => {
    const visible = ["a", "b"];
    expect(computeSelectAllState(visible, ["a", "b"]).checked).toBe(true);
    const partial = computeSelectAllState(visible, ["a"]);
    expect(partial.checked).toBe(false);
    expect(partial.indeterminate).toBe(true);
    const none = computeSelectAllState(visible, []);
    expect(none.checked).toBe(false);
    expect(none.indeterminate).toBe(false);
  });
});

describe("toggleCheckedId", () => {
  it("勾选追加、重复勾选不重复、取消勾选移除", () => {
    expect(toggleCheckedId([], "a", true)).toEqual(["a"]);
    expect(toggleCheckedId(["a"], "a", true)).toEqual(["a"]);
    expect(toggleCheckedId(["a", "b"], "a", false)).toEqual(["b"]);
  });
});

describe("toggleAllChecked", () => {
  it("全选合并可见项，全不选仅清可见项、保留不可见项勾选", () => {
    expect(toggleAllChecked(["x"], ["a", "b"], true)).toEqual(["x", "a", "b"]);
    expect(toggleAllChecked(["x", "a"], ["a", "b"], false)).toEqual(["x"]);
  });
});

describe("removeProjects", () => {
  it("按 id 集合移除，不改原数组", () => {
    const remaining = removeProjects(FIXTURES, [
      "11111111-1111-4111-8111-111111111111",
      "22222222-2222-4222-8222-222222222222",
    ]);
    expect(remaining).toHaveLength(FIXTURES.length - 2);
    expect(remaining.some((project) => project.name === "星海长歌")).toBe(false);
    expect(FIXTURES).toHaveLength(4);
  });

  it("空集合返回等长新数组", () => {
    expect(removeProjects(FIXTURES, [])).toHaveLength(FIXTURES.length);
  });
});
