import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ParseOutline } from "../../../../../../../../libs/commands/types";
import ChapterTree from "../../../../../../../../routes/(projects)/novel/[uuid]/edit/_components/chapter-tree.svelte";

const outline: ParseOutline = {
  volume_count: 2,
  chapter_count: 3,
  line_count: 30,
  char_count: 300,
  preface_lines: 0,
  numbers_missing: false,
  titles_missing: false,
  issues: [],
  backup_exists: false,
  volumes: [
    {
      title: null,
      chapters: [
        { title: "第一章 引子", chars: 100 },
        { title: "第二章 发展", chars: 80 },
      ],
    },
    { title: "第二卷 承接", chapters: [{ title: "第一章 再起", chars: 50 }] },
  ],
};

const flat: ParseOutline = {
  volume_count: 1,
  chapter_count: 2,
  line_count: 10,
  char_count: 180,
  preface_lines: 0,
  numbers_missing: false,
  titles_missing: false,
  issues: [],
  backup_exists: false,
  volumes: [
    {
      title: null,
      chapters: [
        { title: "第一章 引子", chars: 100 },
        { title: "第二章 发展", chars: 80 },
      ],
    },
  ],
};

const selectMock = vi.hoisted(() => vi.fn());

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("章节树", () => {
  it("分卷书默认折叠，展开后点击章节抛选中", async () => {
    const user = userEvent.setup();
    render(ChapterTree, {
      props: { outline, selected: null, visible: null, onSelect: selectMock },
    });

    const firstTrigger = screen.getByText("Untitled").closest("button");
    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("false");

    await user.click(screen.getByText("Untitled"));
    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("true");

    await user.click(screen.getByText("第一章 引子"));
    expect(selectMock).toHaveBeenCalledOnce();
    expect(selectMock).toHaveBeenCalledWith(0, 0);
  });

  it("纯章节单卷平铺展示，选中章高亮", () => {
    render(ChapterTree, {
      props: {
        outline: flat,
        selected: { volume: 0, chapter: 1 },
        visible: null,
        onSelect: selectMock,
      },
    });

    expect(screen.getByText("第一章 引子")).not.toBeNull();
    const active = screen.getByText("第二章 发展").closest("button");
    expect(active?.getAttribute("aria-current")).toBe("true");
    const inactive = screen.getByText("第一章 引子").closest("button");
    expect(inactive?.getAttribute("aria-current")).toBeNull();
  });

  it("超长卷全量渲染不截断", async () => {
    const user = userEvent.setup();
    const long: ParseOutline = {
      ...outline,
      volume_count: 1,
      chapter_count: 205,
      volumes: [
        {
          title: "第一卷 长卷",
          chapters: Array.from({ length: 205 }, (_, i) => ({
            title: `第${i + 1}章`,
            chars: 10,
          })),
        },
      ],
    };
    render(ChapterTree, {
      props: { outline: long, selected: null, visible: null, onSelect: selectMock },
    });

    expect(screen.getByText("第一卷 长卷")).not.toBeNull();
    // 分卷单卷默认折叠：展开后第 205 章仍在（files 页此处截断）。
    await user.click(screen.getByText("第一卷 长卷"));
    expect(screen.getByText("第205章")).not.toBeNull();
  });

  it("可见集过滤章节与空卷，卷头显示命中数", async () => {
    const user = userEvent.setup();
    render(ChapterTree, {
      props: { outline, selected: null, visible: new Set(["0:1"]), onSelect: selectMock },
    });

    // 无标题卷仅保留第二章，有标题卷整组隐藏。
    expect(screen.getByText("Untitled")).not.toBeNull();
    expect(screen.queryByText("第二卷 承接")).toBeNull();
    expect(screen.getByText("1/2 matched")).not.toBeNull();
    await user.click(screen.getByText("Untitled"));

    expect(screen.queryByText("第一章 引子")).toBeNull();
    expect(screen.getByText("第二章 发展")).not.toBeNull();
  });
});
