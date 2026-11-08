import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { ChapterContent, ParseOutline } from "../../../../../../../libs/commands/types";
import EditPage from "../../../../../../../routes/(projects)/novel/[uuid]/edit/+page.svelte";

// jsdom 缺少浮层定位所需的浏览器 API，bits-ui 下拉用得到，仅在本文件内就地补齐。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

const outlineMock = vi.hoisted(() => vi.fn());
const chapterMock = vi.hoisted(() => vi.fn());
const updateMock = vi.hoisted(() => vi.fn());
const filterMock = vi.hoisted(() => vi.fn());
const gotoMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: {
    getNovelContentOutline: outlineMock,
    getNovelChapter: chapterMock,
    updateNovelChapter: updateMock,
    filterNovelContent: filterMock,
  },
}));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/novel/abc-uuid/edit"),
    params: { uuid: "abc-uuid" },
  },
}));

vi.mock("$app/navigation", () => ({ goto: gotoMock }));

// jsdom 缺少 CodeMirror 测量所需的 DOM API（Range.getClientRects），
// 此处用 textarea 语义替身承托编辑器：保留 `.cm-content` 查询点与
// updateListener → onEdit 回调链路，页面状态机（脏态/保存/确认）可确定性测试。
const listeners = vi.hoisted(() => ({
  onDocChanged: null as ((text: string) => void) | null,
}));

vi.mock("codemirror", () => ({
  basicSetup: {},
  EditorView: class {
    static lineWrapping = {};
    static decorations = { from: () => ({}) };
    static theme() {
      return {};
    }
    static updateListener = {
      of: (
        callback: (update: { docChanged: boolean; state: { doc: { toString(): string } } }) => void,
      ) => {
        listeners.onDocChanged = (text: string) =>
          callback({ docChanged: true, state: { doc: { toString: () => text } } });
        return {};
      },
    };
    state: {
      doc: {
        toString: () => string;
        lines: number;
        line: (n: number) => { from: number; to: number; text: string };
      };
    };
    constructor(config: { doc: string; parent: Element }) {
      const area = document.createElement("textarea");
      area.className = "cm-content";
      area.value = config.doc;
      area.addEventListener("input", () => listeners.onDocChanged?.(area.value));
      config.parent.appendChild(area);
      const lines = config.doc.split("\n");
      this.state = {
        doc: {
          toString: () => area.value,
          lines: lines.length,
          line: (n: number) => {
            const text = lines[n - 1] ?? "";
            const from = lines.slice(0, n - 1).join("\n").length + (n > 1 ? 1 : 0);
            return { from, to: from + text.length, text };
          },
        },
      };
    }
    dispatch() {}
    focus() {}
    destroy() {}
  },
}));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：只实现本页面用到的 `result()` */
function fakeCommand(result: FakeResult): { result: () => Promise<FakeResult> } {
  return { result: () => Promise.resolve(result) };
}

const outline: ParseOutline = {
  volume_count: 1,
  chapter_count: 2,
  line_count: 2,
  char_count: 8,
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
        { title: "第一章 引子", chars: 4 },
        { title: "第二章 发展", chars: 4 },
      ],
    },
  ],
};

const firstChapter: ChapterContent = { full_title: "第一章 引子", lines: ["正文甲", "正文乙"] };
const secondChapter: ChapterContent = { full_title: "第二章 发展", lines: ["正文丙", "正文丁"] };

function mockLoaded(): void {
  outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
  chapterMock.mockImplementation((_: string, volume: number, chapter: number) =>
    fakeCommand({
      status: "ok",
      data: volume === 0 && chapter === 1 ? secondChapter : firstChapter,
    }),
  );
  updateMock.mockReturnValue(fakeCommand({ status: "ok", data: outline }));
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  localStorage.clear();
});

describe("编辑页", () => {
  it("加载大纲并默认打开首章，未改动时保存禁用", async () => {
    mockLoaded();
    render(EditPage);

    // 左栏平铺章节，中栏标题栏显示首章标题。
    expect(await screen.findByText("第二章 发展")).not.toBeNull();
    expect(await screen.findByRole("heading", { name: "第一章 引子" })).not.toBeNull();
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("无解析产物时引导去文件步骤", async () => {
    const user = userEvent.setup();
    outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    render(EditPage);

    expect(await screen.findByText("No parsed content yet")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Go to Files" }));
    expect(gotoMock).toHaveBeenCalledWith("/novel/abc-uuid/files");
  });

  it("编辑正文后保存可用，落盘成功刷新大纲", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);

    const editor = await screen.findByLabelText("Chapter text editor");
    const content = editor.querySelector(".cm-content");
    expect(content).not.toBeNull();
    await user.clear(content as Element);
    await user.type(content as Element, "新正文");

    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(updateMock).toHaveBeenCalledOnce();
    expect(updateMock).toHaveBeenCalledWith("abc-uuid", 0, 0, ["新正文"]);
    // 保存成功后脏态清零，按钮重新禁用。
    expect(
      ((await screen.findByRole("button", { name: "Save" })) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("脏态切章弹确认框，确认后切换并加载新章", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);

    const editor = await screen.findByLabelText("Chapter text editor");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "新正文");
    await user.click(screen.getByText("第二章 发展"));

    expect(await screen.findByText("Discard unsaved changes?")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Discard" }));

    expect(await screen.findByRole("heading", { name: "第二章 发展" })).not.toBeNull();
    expect(chapterMock).toHaveBeenCalledWith("abc-uuid", 0, 1);
  });

  it("筛选命中后左栏只留命中章节，中栏出现命中列表", async () => {
    const user = userEvent.setup();
    mockLoaded();
    filterMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: {
          chapters: [
            {
              volume_index: 0,
              chapter_index: 1,
              matches: [{ line: 1, spans: [{ start: 0, len: 2 }] }],
            },
          ],
          truncated: false,
        },
      }),
    );
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    await user.type(screen.getByPlaceholderText("Enter keyword…"), "正文");
    await user.click(screen.getByRole("button", { name: "Filter" }));

    expect(filterMock).toHaveBeenCalledOnce();
    expect(filterMock).toHaveBeenCalledWith("abc-uuid", {
      type: "Keyword",
      keyword: "正文",
      case_sensitive: false,
    });
    // 当前选中首章无命中 → 自动跳到首个命中章节。
    expect(await screen.findByRole("heading", { name: "第二章 发展" })).not.toBeNull();
    // 左栏仅剩命中章节，中栏挂出命中列表。
    expect(screen.queryByText("第一章 引子")).toBeNull();
    expect(screen.getByText("Matches in this chapter (1)")).not.toBeNull();
    expect(screen.getByText("Line 1")).not.toBeNull();
  });

  it("脏态禁用筛选，编辑后挂过期提示，清除恢复全量", async () => {
    const user = userEvent.setup();
    mockLoaded();
    filterMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: {
          chapters: [
            {
              volume_index: 0,
              chapter_index: 0,
              matches: [{ line: 1, spans: [{ start: 0, len: 2 }] }],
            },
          ],
          truncated: false,
        },
      }),
    );
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    await user.type(screen.getByPlaceholderText("Enter keyword…"), "正文");
    await user.click(screen.getByRole("button", { name: "Filter" }));
    expect(await screen.findByText("Matches in this chapter (1)")).not.toBeNull();

    // 编辑正文 → 脏态禁用筛选，命中列表挂过期提示。
    const editor = await screen.findByLabelText("Chapter text editor");
    await user.type(editor.querySelector(".cm-content") as Element, "改");
    expect((screen.getByRole("button", { name: "Filter" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(screen.getByText("Text edited, results are based on the saved version")).not.toBeNull();

    // 清除筛选 → 左栏恢复全量。
    await user.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.getByText("第二章 发展")).not.toBeNull();
    expect(screen.queryByText("Matches in this chapter (1)")).toBeNull();
  });

  it("保存成功后自动重跑筛选", async () => {
    const user = userEvent.setup();
    mockLoaded();
    filterMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: {
          chapters: [
            {
              volume_index: 0,
              chapter_index: 0,
              matches: [{ line: 1, spans: [{ start: 0, len: 2 }] }],
            },
          ],
          truncated: false,
        },
      }),
    );
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    await user.type(screen.getByPlaceholderText("Enter keyword…"), "正文");
    await user.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenCalledOnce();

    const editor = await screen.findByLabelText("Chapter text editor");
    await user.type(editor.querySelector(".cm-content") as Element, "改");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(updateMock).toHaveBeenCalledOnce();
    expect(filterMock).toHaveBeenCalledTimes(2);
  });

  it("垃圾清理默认全选可运行，全取消后筛选禁用", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    // 类型下拉切到垃圾清理：jsdom 无布局，选项按 `data-value` 定位
    await user.click(screen.getByRole("combobox", { name: "Filter type" }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="cleanup"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="cleanup"]') as HTMLElement);

    // 浮层关闭动画在 jsdom 不跑、body 滚动锁残留，后续点击改用 fireEvent 直达；
    // 测的是规格组装与门禁，点击 mechanics 由组件库用例覆盖
    expect(await screen.findByRole("checkbox", { name: "URLs" })).not.toBeNull();
    filterMock.mockReturnValue(
      fakeCommand({ status: "ok", data: { chapters: [], truncated: false } }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenCalledWith("abc-uuid", {
      type: "Cleanup",
      presets: [
        "UrlFull",
        "UrlDomain",
        "PromoRelease",
        "PromoSite",
        "PromoVote",
        "PromoGroup",
        "ContactQq",
        "ContactWechat",
        "ContactGroup",
      ],
    });

    // 叶子乱序重勾不影响规格顺序：取消再重勾中间叶子，仍按固定顺序拼装
    await fireEvent.click(screen.getByRole("checkbox", { name: "Domain names" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Domain names" }));
    await fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenLastCalledWith("abc-uuid", {
      type: "Cleanup",
      presets: [
        "UrlFull",
        "UrlDomain",
        "PromoRelease",
        "PromoSite",
        "PromoVote",
        "PromoGroup",
        "ContactQq",
        "ContactWechat",
        "ContactGroup",
      ],
    });

    // 三个组头全关 → 规格为空，筛选禁用
    await fireEvent.click(screen.getByRole("checkbox", { name: "URLs" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Site promos" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Contact info" }));
    expect((screen.getByRole("button", { name: "Filter" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("标点符号默认全选可运行，全取消后筛选禁用", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    // 类型下拉切到标点符号：jsdom 无布局，选项按 `data-value` 定位
    await user.click(screen.getByRole("combobox", { name: "Filter type" }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="punct"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="punct"]') as HTMLElement);

    // 浮层关闭动画在 jsdom 不跑、body 滚动锁残留，后续点击改用 fireEvent 直达
    expect(await screen.findByRole("checkbox", { name: "Half-width punct" })).not.toBeNull();
    filterMock.mockReturnValue(
      fakeCommand({ status: "ok", data: { chapters: [], truncated: false } }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenCalledWith("abc-uuid", {
      type: "Punct",
      presets: ["HalfPunct", "FullSymbol", "ZeroWidth", "WeirdSpace", "SpecialSym", "RepeatPunct"],
    });

    // 三个组头全关 → 规格为空，筛选禁用
    await fireEvent.click(screen.getByRole("checkbox", { name: "Width norms" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Hidden chars" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Odd marks" }));
    expect((screen.getByRole("button", { name: "Filter" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("标点闭合默认按行全选可运行，切跨行后规格跟随，全取消后筛选禁用", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    // 类型下拉切到标点闭合：jsdom 无布局，选项按 `data-value` 定位
    await user.click(screen.getByRole("combobox", { name: "Filter type" }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="pairs"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="pairs"]') as HTMLElement);

    // 浮层关闭动画在 jsdom 不跑、body 滚动锁残留，后续点击改用 fireEvent 直达
    expect(await screen.findByRole("checkbox", { name: "Round pairs" })).not.toBeNull();
    filterMock.mockReturnValue(
      fakeCommand({ status: "ok", data: { chapters: [], truncated: false } }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenCalledWith("abc-uuid", {
      type: "Pairs",
      pairs: [
        "Round",
        "Square",
        "Brace",
        "TortBlack",
        "TortWhite",
        "BookDouble",
        "BookSingle",
        "DQuote",
        "SQuote",
        "CQuoteS",
        "CQuoteD",
        "AsciiS",
        "AsciiD",
        "RevQuote",
      ],
      scope: "Line",
    });

    // 切到跨行 → 规格 scope 跟随
    await fireEvent.click(screen.getByRole("radio", { name: "Cross line" }));
    await fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(filterMock).toHaveBeenLastCalledWith("abc-uuid", {
      type: "Pairs",
      pairs: [
        "Round",
        "Square",
        "Brace",
        "TortBlack",
        "TortWhite",
        "BookDouble",
        "BookSingle",
        "DQuote",
        "SQuote",
        "CQuoteS",
        "CQuoteD",
        "AsciiS",
        "AsciiD",
        "RevQuote",
      ],
      scope: "Chapter",
    });

    // 五个组头全关 → 规格为空，筛选禁用
    await fireEvent.click(screen.getByRole("checkbox", { name: "Round brackets" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Special brackets" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Title marks" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "Curved quotes" }));
    await fireEvent.click(screen.getByRole("checkbox", { name: "ASCII quotes" }));
    expect((screen.getByRole("button", { name: "Filter" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("Ctrl+S 触发保存，干净态下无操作", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);

    // 干净态：快捷键不调后端。
    await screen.findByRole("heading", { name: "第一章 引子" });
    await fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(updateMock).not.toHaveBeenCalled();

    // 脏态：快捷键与保存按钮同效。
    const editor = await screen.findByLabelText("Chapter text editor");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "新正文");
    await fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(updateMock).toHaveBeenCalledOnce();
    expect(updateMock).toHaveBeenCalledWith("abc-uuid", 0, 0, ["新正文"]);
  });

  it("自动保存开关可切换并持久化，默认关闭", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    const toggle = screen.getByRole("button", { name: "Auto-save: off" });
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    await user.click(toggle);

    expect(screen.getByRole("button", { name: "Auto-save: on" }).getAttribute("aria-pressed")).toBe(
      "true",
    );
    expect(localStorage.getItem("app.novel-edit-autosave")).toBe("1");
  });

  it("自动保存开启时脏态切章先落盘后打开，不弹确认框", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    await user.click(screen.getByRole("button", { name: "Auto-save: off" }));
    const editor = await screen.findByLabelText("Chapter text editor");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "新正文");
    await user.click(screen.getByText("第二章 发展"));

    expect(updateMock).toHaveBeenCalledOnce();
    expect(updateMock).toHaveBeenCalledWith("abc-uuid", 0, 0, ["新正文"]);
    expect(await screen.findByRole("heading", { name: "第二章 发展" })).not.toBeNull();
    expect(screen.queryByText("Discard unsaved changes?")).toBeNull();
  });

  it("自动保存失败时停留当前章", async () => {
    const user = userEvent.setup();
    mockLoaded();
    updateMock.mockReturnValueOnce(
      fakeCommand({ status: "error", error: { kind: "Internal", message: "no" } }),
    );
    render(EditPage);
    await screen.findByRole("heading", { name: "第一章 引子" });

    await user.click(screen.getByRole("button", { name: "Auto-save: off" }));
    const editor = await screen.findByLabelText("Chapter text editor");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "新正文");
    await user.click(screen.getByText("第二章 发展"));

    expect(updateMock).toHaveBeenCalledOnce();
    expect(chapterMock).not.toHaveBeenCalledWith("abc-uuid", 0, 1);
    expect(await screen.findByRole("heading", { name: "第一章 引子" })).not.toBeNull();
  });
});
