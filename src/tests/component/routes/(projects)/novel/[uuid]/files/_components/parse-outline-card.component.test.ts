import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ParseOutline } from "../../../../../../../../libs/commands/types";
import ParseOutlineCard from "../../../../../../../../routes/(projects)/novel/[uuid]/files/_components/parse-outline-card.svelte";

const outline: ParseOutline = {
  volume_count: 2,
  chapter_count: 3,
  line_count: 120,
  char_count: 2400,
  preface_lines: 5,
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

const renumberMock = vi.hoisted(() => vi.fn());
const undoMock = vi.hoisted(() => vi.fn());
const openDirMock = vi.hoisted(() => vi.fn());

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("解析结果卡", () => {
  it("两行展示计数，无标题卷回落未分卷", () => {
    render(ParseOutlineCard, {
      props: { outline, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    expect(screen.getByText("Volumes")).not.toBeNull();
    expect(screen.getByText("Chapters")).not.toBeNull();
    expect(screen.getByText("Lines")).not.toBeNull();
    expect(screen.getByText("Characters")).not.toBeNull();
    expect(screen.getByText("Untitled")).not.toBeNull();
    expect(screen.getByText("第二卷 承接")).not.toBeNull();
    // 前言行不再展示，问题区显示无问题。
    expect(screen.queryByText("Preface · 5 lines")).toBeNull();
    expect(screen.getByText("No issues")).not.toBeNull();
  });

  it("分卷书默认折叠，展开后查看章节与字数", async () => {
    const user = userEvent.setup();
    render(ParseOutlineCard, {
      props: { outline, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    // 卷头右侧显示本卷章节数，章节默认收起。
    expect(screen.getByText("2 chapters")).not.toBeNull();
    expect(screen.getByText("1 chapters")).not.toBeNull();
    const firstTrigger = screen.getByText("Untitled").closest("button");
    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("false");

    await user.click(screen.getByText("Untitled"));

    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("第一章 引子")).not.toBeNull();
    expect(screen.getByText("100 chars")).not.toBeNull();
    expect(screen.getByText("80 chars")).not.toBeNull();
    // 未展开的卷仍收起。
    expect(screen.getByText("第二卷 承接").closest("button")?.getAttribute("aria-expanded")).toBe(
      "false",
    );
  });

  it("纯章节单卷平铺展示", () => {
    render(ParseOutlineCard, {
      props: { outline: flat, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    expect(screen.getByText("第一章 引子")).not.toBeNull();
    expect(screen.getByText("80 chars")).not.toBeNull();
  });

  it("超长卷按 200 章截断并提示剩余", async () => {
    const user = userEvent.setup();
    const long: ParseOutline = {
      volume_count: 1,
      chapter_count: 205,
      line_count: 205,
      char_count: 1000,
      preface_lines: 0,
      numbers_missing: false,
      titles_missing: false,
      issues: [],
      backup_exists: false,
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
    render(ParseOutlineCard, {
      props: { outline: long, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    expect(screen.getByText("205 chapters")).not.toBeNull();
    const trigger = screen.getByText("第一卷 长卷").closest("button");
    expect(trigger?.getAttribute("aria-expanded")).toBe("false");

    await user.click(screen.getByText("第一卷 长卷"));

    expect(trigger?.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("第200章")).not.toBeNull();
    expect(screen.queryByText("第201章")).toBeNull();
    expect(screen.getByText("+ 5 more chapters")).not.toBeNull();
  });

  it("未分卷超 100 章按序号分组折叠", async () => {
    const user = userEvent.setup();
    const flat: ParseOutline = {
      volume_count: 1,
      chapter_count: 205,
      line_count: 205,
      char_count: 2050,
      preface_lines: 0,
      numbers_missing: false,
      titles_missing: false,
      issues: [],
      backup_exists: false,
      volumes: [
        {
          title: null,
          chapters: Array.from({ length: 205 }, (_, i) => ({
            title: `第${i + 1}章`,
            chars: 10,
          })),
        },
      ],
    };
    render(ParseOutlineCard, {
      props: { outline: flat, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    expect(screen.getByText("Chapters 001–100")).not.toBeNull();
    expect(screen.getByText("Chapters 101–200")).not.toBeNull();
    expect(screen.getByText("Chapters 201–205")).not.toBeNull();
    const firstTrigger = screen.getByText("Chapters 001–100").closest("button");
    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("false");

    await user.click(screen.getByText("Chapters 001–100"));

    expect(firstTrigger?.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("第1章")).not.toBeNull();
    // 三组内容全挂载（收起态 CSS 隐藏），总数即全书章节数。
    expect(screen.getAllByText("10 chars")).toHaveLength(205);
  });

  it("问题区展示条目与 pattern 级提示", () => {
    render(ParseOutlineCard, {
      props: {
        outline: {
          ...outline,
          issues: [
            { kind: "DuplicateNumber", volume: "第一卷 开篇", chapter: "第一章 重复", number: 1 },
            { kind: "MissingChapter", volume: null, chapter: null, number: 2 },
            { kind: "MissingTitle", volume: "第三卷", chapter: null, number: null },
          ],
        },
        onRenumber: renumberMock,
        onUndo: undoMock,
        onOpenDir: openDirMock,
      },
    });

    expect(screen.getByText("Issues (3)")).not.toBeNull();
    expect(screen.getByText("Duplicate number: 第一章 重复 (1)")).not.toBeNull();
    expect(screen.getByText("Missing volume 2")).not.toBeNull();
    expect(screen.getByText("Failed to extract clean title: 第三卷")).not.toBeNull();
    expect(screen.queryByText("No issues")).toBeNull();
  });

  it("缺组时只显示单条提示并计入总数", () => {
    render(ParseOutlineCard, {
      props: {
        outline: { ...outline, numbers_missing: true, titles_missing: true },
        onRenumber: renumberMock,
        onUndo: undoMock,
        onOpenDir: openDirMock,
      },
    });

    expect(screen.getByText("Issues (2)")).not.toBeNull();
    expect(
      screen.getByText(
        "No numbers recognized (patterns lack a number capture group), stored in original order",
      ),
    ).not.toBeNull();
    expect(
      screen.getByText(
        "No clean titles recognized (patterns lack a title capture group), using full lines",
      ),
    ).not.toBeNull();
  });

  it("有问题时显示强制重排按钮，确认后走回调", async () => {
    const user = userEvent.setup();
    render(ParseOutlineCard, {
      props: {
        outline: {
          ...outline,
          issues: [
            { kind: "DuplicateNumber", volume: "第一卷 开篇", chapter: "第一章 重复", number: 1 },
          ],
        },
        onRenumber: renumberMock,
        onUndo: undoMock,
        onOpenDir: openDirMock,
      },
    });

    await user.click(screen.getByRole("button", { name: "Force renumber" }));

    expect(screen.getByText("Force renumber?")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Confirm" }));

    expect(renumberMock).toHaveBeenCalledOnce();
  });

  it("无备份时不显示撤销重排按钮", () => {
    render(ParseOutlineCard, {
      props: { outline, onRenumber: renumberMock, onUndo: undoMock, onOpenDir: openDirMock },
    });

    expect(screen.queryByRole("button", { name: "Undo renumber" })).toBeNull();
  });

  it("有备份时显示撤销重排按钮，点击走回调", async () => {
    const user = userEvent.setup();
    render(ParseOutlineCard, {
      props: {
        outline: { ...outline, backup_exists: true },
        onRenumber: renumberMock,
        onUndo: undoMock,
        onOpenDir: openDirMock,
      },
    });

    await user.click(screen.getByRole("button", { name: "Undo renumber" }));

    expect(undoMock).toHaveBeenCalledOnce();
  });
});
