import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { NovelTextStatus, ParseOutline } from "../../../../../../../libs/commands/types";
import FilesPage from "../../../../../../../routes/(projects)/novel/[uuid]/files/+page.svelte";
import {
  DEFAULT_CHAPTER_PATTERN,
  DEFAULT_VOLUME_PATTERN,
} from "../../../../../../../routes/(projects)/novel/[uuid]/files/_components/novel-text-params";

const getStatusMock = vi.hoisted(() => vi.fn());
const importMock = vi.hoisted(() => vi.fn());
const parseMock = vi.hoisted(() => vi.fn());
const outlineMock = vi.hoisted(() => vi.fn());
const renumberMock = vi.hoisted(() => vi.fn());
const openMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: {
    getNovelTextStatus: getStatusMock,
    importNovelText: importMock,
    parseNovelText: parseMock,
    getNovelContentOutline: outlineMock,
    renumberNovelContent: renumberMock,
  },
}));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/novel/abc-uuid/files"),
    params: { uuid: "abc-uuid" },
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：只实现本页面用到的 `result()` */
function fakeCommand(result: FakeResult): { result: () => Promise<FakeResult> } {
  return { result: () => Promise.resolve(result) };
}

const status: NovelTextStatus = {
  exists: true,
  size_bytes: 1024,
  updated_at: "2026-02-01T00:00:00Z",
};

const outline: ParseOutline = {
  volume_count: 1,
  chapter_count: 2,
  line_count: 10,
  char_count: 60,
  preface_lines: 0,
  numbers_missing: false,
  titles_missing: false,
  issues: [],
  backup_exists: false,
  scanned_lines: 0,
  unmatched_samples: [],
  unmatched_total: 0,
  volumes: [
    {
      title: null,
      chapters: [
        { title: "第一章 引子", chars: 10 },
        { title: "第二章 发展", chars: 20 },
      ],
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  getStatusMock.mockReturnValue(fakeCommand({ status: "ok", data: status }));
  outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
});

afterEach(() => {
  cleanup();
});

describe("小说文件步骤页", () => {
  it("解析成功后右栏展示识别大纲", async () => {
    const user = userEvent.setup();
    parseMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
    render(FilesPage);

    await screen.findByText("Body text");
    await user.click(screen.getByRole("button", { name: "Start parsing" }));

    expect(parseMock).toHaveBeenCalledOnce();
    expect(parseMock.mock.calls[0]).toEqual([
      "abc-uuid",
      DEFAULT_VOLUME_PATTERN,
      DEFAULT_CHAPTER_PATTERN,
    ]);
    expect(await screen.findByText("Untitled")).not.toBeNull();
    expect(screen.getByText("第一章 引子")).not.toBeNull();
  });

  it("打开页面时已有大纲直接回显，无需重新解析", async () => {
    outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
    render(FilesPage);

    await screen.findByText("Body text");
    expect(outlineMock).toHaveBeenCalledOnce();
    expect(outlineMock.mock.calls[0]).toEqual(["abc-uuid"]);
    expect(await screen.findByText("Untitled")).not.toBeNull();
    expect(screen.getByText("第一章 引子")).not.toBeNull();
    expect(parseMock).not.toHaveBeenCalled();
  });

  it("强制重排确认后刷新大纲", async () => {
    const user = userEvent.setup();
    const issueOutline = {
      ...outline,
      issues: [
        { kind: "DuplicateNumber", volume: "第一卷 开篇", chapter: "第一章 重复", number: 1 },
      ],
    };
    parseMock.mockReturnValue(fakeCommand({ status: "ok", data: issueOutline }));
    renumberMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
    render(FilesPage);

    await screen.findByText("Body text");
    await user.click(screen.getByRole("button", { name: "Start parsing" }));
    await user.click(await screen.findByRole("button", { name: "Force renumber" }));

    expect(screen.getByText("Force renumber?")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Confirm" }));

    expect(renumberMock).toHaveBeenCalledOnce();
    expect(renumberMock.mock.calls[0]).toEqual(["abc-uuid"]);
    expect(await screen.findByText("No issues")).not.toBeNull();
  });

  it("解析失败时右栏保持空态", async () => {
    const user = userEvent.setup();
    parseMock.mockReturnValue(
      fakeCommand({ status: "error", error: { kind: "Internal", message: "no chapters" } }),
    );
    render(FilesPage);

    await screen.findByText("Body text");
    await user.click(screen.getByRole("button", { name: "Start parsing" }));

    expect(parseMock).toHaveBeenCalledOnce();
    expect(
      screen.getByText(
        "Volume and chapter results will be previewed here after import and parsing",
      ),
    ).not.toBeNull();
    expect(screen.queryByText("Untitled")).toBeNull();
  });
});
