import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ComicImagesStatus, ComicOutline } from "../../../../../../../libs/commands/types";
import FilesPage from "../../../../../../../routes/(projects)/comic/[uuid]/files/+page.svelte";

const statusMock = vi.hoisted(() => vi.fn());
const outlineMock = vi.hoisted(() => vi.fn());
const scanMock = vi.hoisted(() => vi.fn());
const deleteMock = vi.hoisted(() => vi.fn());
const importMock = vi.hoisted(() => vi.fn());
const parseMock = vi.hoisted(() => vi.fn());
const imageMock = vi.hoisted(() => vi.fn());
const appendMock = vi.hoisted(() => vi.fn());
const openMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: {
    getComicImagesStatus: statusMock,
    getComicContentOutline: outlineMock,
    scanComicOrphans: scanMock,
    deleteComicOrphans: deleteMock,
    importComicImages: importMock,
    parseComicImages: parseMock,
    getComicImagePath: imageMock,
    appendComicImages: appendMock,
  },
}));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/comic/abc-uuid/files"),
    params: { uuid: "abc-uuid" },
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (path: string) => path }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：只实现本页面用到的 `result()` */
function fakeCommand(result: FakeResult): { result: () => Promise<FakeResult> } {
  return { result: () => Promise.resolve(result) };
}

const status: ComicImagesStatus = {
  exists: true,
  volume_count: 1,
  page_count: 2,
  size_bytes: 1024,
  updated_at: "2026-02-01T00:00:00Z",
};

const outline: ComicOutline = {
  mode: "volumes",
  volume_count: 1,
  page_count: 2,
  total_bytes: 1024,
  ignored_count: 0,
  volumes: [
    {
      title: "第1话",
      pages: [
        { path: "第1话/01.jpg", size_bytes: 512 },
        { path: "第1话/02.jpg", size_bytes: 512 },
      ],
    },
  ],
};

function mockLoaded(): void {
  statusMock.mockReturnValue(fakeCommand({ status: "ok", data: status }));
  outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
  imageMock.mockReturnValue(
    fakeCommand({ status: "ok", data: { path: "/tmp/01.jpg", size_bytes: 512 } }),
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mockLoaded();
  scanMock.mockReturnValue(fakeCommand({ status: "ok", data: { paths: [], total_bytes: 0 } }));
});

afterEach(() => {
  cleanup();
});

describe("漫画文件步骤页孤儿清理", () => {
  it("无孤儿时不显示清理入口", async () => {
    render(FilesPage);
    await screen.findByText("第1话");

    expect(screen.queryByRole("button", { name: "Clean unused images" })).toBeNull();
  });

  it("有孤儿时显示计数，确认后删除并刷新", async () => {
    const user = userEvent.setup();
    scanMock.mockReturnValue(
      fakeCommand({ status: "ok", data: { paths: ["loose.jpg"], total_bytes: 100 } }),
    );
    deleteMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
    render(FilesPage);
    await screen.findByText("第1话");

    expect(screen.getByText(/1 unused images/)).not.toBeNull();
    // 入口与确认框按钮同名：入口在左栏，确认框挂载在末尾。
    const buttons = screen.getAllByRole("button", { name: "Clean unused images" });
    await user.click(buttons[0]);

    expect(await screen.findByText("Clean unused images?")).not.toBeNull();
    const confirmButtons = screen.getAllByRole("button", { name: "Clean unused images" });
    await user.click(confirmButtons[confirmButtons.length - 1]);

    expect(deleteMock).toHaveBeenCalledOnce();
    expect(deleteMock).toHaveBeenCalledWith("abc-uuid");
    // 删除后整页刷新：状态重新拉取。
    await waitFor(() => {
      expect(statusMock.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
  });

  it("无原稿时不显示添加图片按钮", async () => {
    statusMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: { ...status, exists: false, volume_count: 0, page_count: 0 },
      }),
    );
    outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    render(FilesPage);
    await screen.findByText("Choose folder…");

    expect(screen.queryByRole("button", { name: "Add images" })).toBeNull();
  });

  it("添加图片成功后刷新状态并提示去解析", async () => {
    const user = userEvent.setup();
    openMock.mockReturnValue(Promise.resolve("/tmp/new-pics"));
    appendMock.mockReturnValue(fakeCommand({ status: "ok", data: { added: 2, skipped: 1 } }));
    render(FilesPage);
    await screen.findByText("第1话");

    await user.click(screen.getByRole("button", { name: "Add images" }));

    expect(openMock).toHaveBeenCalledOnce();
    expect(appendMock).toHaveBeenCalledWith("abc-uuid", "/tmp/new-pics");
    // 状态重新拉取（计数跟上），大纲保持有效不清空。
    await waitFor(() => {
      expect(statusMock.mock.calls.length).toBeGreaterThanOrEqual(2);
    });
    expect(screen.getByText("第1话")).not.toBeNull();
  });
});
