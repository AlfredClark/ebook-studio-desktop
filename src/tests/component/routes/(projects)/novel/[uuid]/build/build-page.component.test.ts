import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import type { BuildOutcome, ParseOutline } from "../../../../../../../libs/commands/types";
import BuildPage from "../../../../../../../routes/(projects)/novel/[uuid]/build/+page.svelte";

const treeMock = vi.hoisted(() => vi.fn());
const outlineMock = vi.hoisted(() => vi.fn());
const buildMock = vi.hoisted(() => vi.fn());
const clearMock = vi.hoisted(() => vi.fn());
const previewMock = vi.hoisted(() => vi.fn());
const fileMock = vi.hoisted(() => vi.fn());
const updateMock = vi.hoisted(() => vi.fn());
const textMock = vi.hoisted(() => vi.fn());
const formatAllMock = vi.hoisted(() => vi.fn());
const gotoMock = vi.hoisted(() => vi.fn());

vi.mock("#libs/commands", () => ({
  default: {
    getNovelBuildTree: treeMock,
    getNovelContentOutline: outlineMock,
    buildNovelContent: buildMock,
    clearNovelBuildContent: clearMock,
    previewNovelTxtTitle: previewMock,
    getNovelBuildFile: fileMock,
    updateNovelBuildFile: updateMock,
    formatNovelBuildText: textMock,
    formatAllNovelBuildFiles: formatAllMock,
  },
}));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/novel/abc-uuid/build"),
    params: { uuid: "abc-uuid" },
  },
}));

vi.mock("$app/navigation", () => ({ goto: gotoMock }));

// jsdom 缺少 CodeMirror 测量所需的 DOM API，此处用 textarea 语义替身承托查看器。
const listeners = vi.hoisted(() => ({
  onDocChanged: null as ((text: string) => void) | null,
}));

vi.mock("codemirror", () => ({
  basicSetup: {},
  EditorView: class {
    static lineWrapping = {};
    static editable = { of: () => ({}) };
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
    area: HTMLTextAreaElement | null = null;
    constructor(config: { doc: string; parent: Element }) {
      const area = document.createElement("textarea");
      area.className = "cm-content";
      area.value = config.doc;
      area.addEventListener("input", () => listeners.onDocChanged?.(area.value));
      config.parent.appendChild(area);
      this.area = area;
    }
    state = {
      doc: {
        toString: () => this.area?.value ?? "",
        get length() {
          return 0;
        },
      },
    };
    dispatch(tr?: { changes?: { insert?: string } }) {
      // 语义替身：把替换文本写回 textarea 并触发 input，保证视图与 draft 一致。
      if (tr?.changes?.insert != null && this.area != null) {
        this.area.value = tr.changes.insert;
        this.area.dispatchEvent(new Event("input"));
      }
    }
    destroy() {}
  },
}));

vi.mock("@codemirror/lang-xml", () => ({ xml: () => ({}) }));
vi.mock("@codemirror/lang-css", () => ({ css: () => ({}) }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：实现本页面用到的 `failed()` + `result()` 链 */
function fakeCommand(result: FakeResult): {
  failed: () => { result: () => Promise<FakeResult> };
  result: () => Promise<FakeResult>;
} {
  const tail = { result: () => Promise.resolve(result) };
  return { failed: () => tail, ...tail };
}

const outcome: BuildOutcome = {
  files: [
    {
      name: "EPUB",
      path: "EPUB",
      editable: false,
      children: [
        {
          name: "chapter-001.xhtml",
          path: "EPUB/text/chapter-001.xhtml",
          editable: true,
          children: null,
        },
      ],
    },
    { name: "mimetype", path: "mimetype", editable: false, children: null },
  ],
  file_count: 2,
};

function mockLoaded(): void {
  treeMock.mockReturnValue(fakeCommand({ status: "ok", data: outcome }));
  outlineMock.mockReturnValue(
    fakeCommand({
      status: "ok",
      data: {
        volume_count: 1,
        chapter_count: 1,
        line_count: 1,
        char_count: 3,
        preface_lines: 0,
        numbers_missing: false,
        titles_missing: false,
        issues: [],
        volumes: [{ title: null, chapters: [{ title: "第一章 引子", chars: 3 }] }],
      },
    }),
  );
  fileMock.mockImplementation((_: string, path: string) =>
    fakeCommand({
      status: "ok",
      data: {
        content: path === "mimetype" ? "application/epub+zip" : "<p>正文</p>",
        editable: path !== "mimetype",
        formattable: path !== "mimetype",
      },
    }),
  );
  buildMock.mockReturnValue(fakeCommand({ status: "ok", data: outcome }));
  clearMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
  previewMock.mockImplementation((_: string, _kind: string, _pattern: string, isVolume: boolean) =>
    fakeCommand({ status: "ok", data: isVolume ? "第一卷 开篇" : "第一章 引子" }),
  );
  updateMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
  textMock.mockReturnValue(fakeCommand({ status: "ok", data: "<p>正文</p>\n" }));
  formatAllMock.mockReturnValue(
    fakeCommand({
      status: "ok",
      data: { formatted: 9, skipped: ["mimetype"], failed: [] },
    }),
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

/** 点选首个生成文件：顶层 `EPUB/` 默认展开，行直达可点；等待树渲染完成。 */
async function openChapterFile(user: UserEvent): Promise<void> {
  await user.click(await screen.findByText("chapter-001.xhtml"));
}

describe("构建页", () => {
  it("无构建产物时引导构建，构建后展示树与待选空态", async () => {
    const user = userEvent.setup();
    treeMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    mockLoaded();
    treeMock.mockReturnValueOnce(fakeCommand({ status: "ok", data: null }));
    render(BuildPage);

    expect(await screen.findByText("No build yet")).not.toBeNull();
    // 平铺书只显示章标题格式，卷格式与编号方式隐藏。
    expect(screen.getByText("Chapter title format")).not.toBeNull();
    expect(screen.queryByText("Volume title format")).toBeNull();
    expect(screen.queryByText("Chapter numbering")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Build" }));

    expect(buildMock).toHaveBeenCalledOnce();
    expect(buildMock).toHaveBeenCalledWith("abc-uuid", {
      chapter_kind: "original",
      chapter_pattern: "",
      volume_kind: "original",
      volume_pattern: "",
      chapter_numbering: "per_volume",
    });
    // 构建后静默格式化全部文件。
    await vi.waitFor(() => expect(formatAllMock).toHaveBeenCalledOnce());
    expect(await screen.findByText("chapter-001.xhtml")).not.toBeNull();
    // 不默认打开文件：右侧待选空态，点选后才加载。
    expect(await screen.findByText("Select a file on the left to view")).not.toBeNull();
    expect(fileMock).not.toHaveBeenCalled();
    await openChapterFile(user);
    expect(fileMock).toHaveBeenCalledWith("abc-uuid", "EPUB/text/chapter-001.xhtml");
  });

  it("无解析产物时引导去文件步骤", async () => {
    const user = userEvent.setup();
    treeMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    render(BuildPage);

    expect(await screen.findByText("No parsed content yet")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Go to Files" }));
    expect(gotoMock).toHaveBeenCalledWith("/novel/abc-uuid/files");
    expect(buildMock).not.toHaveBeenCalled();
  });

  it("固定文件只读展示无保存按钮，生成文件可编辑保存", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    // 未点选时右侧待选空态；点选生成文件后保存按钮初始禁用。
    expect(await screen.findByText("Select a file on the left to view")).not.toBeNull();
    await openChapterFile(user);
    expect(await screen.findByText("EPUB/text/chapter-001.xhtml")).not.toBeNull();
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);

    // 选中固定文件：只读徽标，无保存按钮。
    await user.click(screen.getByText("mimetype"));
    expect(await screen.findByText("Read-only")).not.toBeNull();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();

    // 切回生成文件编辑并保存。
    await user.click(screen.getByText("chapter-001.xhtml"));
    const editor = await screen.findByLabelText("EPUB/text/chapter-001.xhtml");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "<p>改</p>");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(updateMock).toHaveBeenCalledOnce();
    expect(updateMock).toHaveBeenCalledWith("abc-uuid", "EPUB/text/chapter-001.xhtml", "<p>改</p>");
  });

  it("脏态切文件弹确认框，确认后切换", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await openChapterFile(user);
    const editor = await screen.findByLabelText("EPUB/text/chapter-001.xhtml");
    await user.type(editor.querySelector(".cm-content") as Element, "改");
    await user.click(screen.getByText("mimetype"));

    expect(await screen.findByText("Discard unsaved changes?")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Discard" }));
    expect(await screen.findByText("Read-only")).not.toBeNull();
  });

  it("分卷书显示编号方式，切连续后构建透传", async () => {
    const user = userEvent.setup();
    treeMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    mockLoaded();
    treeMock.mockReturnValueOnce(fakeCommand({ status: "ok", data: null }));
    const multiOutline: ParseOutline = {
      volume_count: 2,
      chapter_count: 3,
      line_count: 3,
      char_count: 9,
      preface_lines: 0,
      numbers_missing: false,
      titles_missing: false,
      issues: [],
      backup_exists: false,
      volumes: [
        {
          title: "第一卷 开篇",
          chapters: [
            { title: "第一章 起", chars: 3 },
            { title: "第二章 承", chars: 3 },
          ],
        },
        { title: "第二卷 续篇", chapters: [{ title: "第一章 再起", chars: 3 }] },
      ],
    };
    outlineMock.mockReturnValue(fakeCommand({ status: "ok", data: multiOutline }));
    render(BuildPage);

    expect(await screen.findByText("Chapter numbering")).not.toBeNull();
    expect(screen.getByText("Volume title format")).not.toBeNull();
    // 默认分卷重置选中。
    expect(
      (screen.getByRole("radio", { name: "Restart per volume" }) as HTMLElement).getAttribute(
        "aria-checked",
      ),
    ).toBe("true");
    await user.click(screen.getByRole("radio", { name: "Continuous across volumes" }));
    expect(
      (
        screen.getByRole("radio", { name: "Continuous across volumes" }) as HTMLElement
      ).getAttribute("aria-checked"),
    ).toBe("true");
    await user.click(screen.getByRole("button", { name: "Build" }));

    expect(buildMock).toHaveBeenCalledWith("abc-uuid", {
      chapter_kind: "original",
      chapter_pattern: "",
      volume_kind: "original",
      volume_pattern: "",
      chapter_numbering: "continuous",
    });
  });

  it("重构建确认后删除产物并回到构建引导", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await screen.findByText("chapter-001.xhtml");
    await user.click(screen.getByRole("button", { name: "Rebuild" }));

    expect(await screen.findByText("Rebuild?")).not.toBeNull();
    // 确认后清空产物：再次拉树回空，后续走构建引导。
    treeMock.mockReturnValue(fakeCommand({ status: "ok", data: null }));
    await user.click(screen.getByRole("button", { name: "Confirm rebuild" }));

    expect(clearMock).toHaveBeenCalledOnce();
    expect(clearMock).toHaveBeenCalledWith("abc-uuid");
    expect(buildMock).not.toHaveBeenCalled();
    expect(await screen.findByText("No build yet")).not.toBeNull();
    // 平铺大纲回到引导：章格式保留，卷格式与编号方式隐藏。
    expect(screen.getByText("Chapter title format")).not.toBeNull();
    expect(screen.queryByText("Volume title format")).toBeNull();
  });

  it("单文件格式化回填编辑器并变脏，保存后清脏", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await screen.findByText("chapter-001.xhtml");
    await openChapterFile(user);
    // 格式化整理编辑器当前文本：干净态直接可用。
    await user.click(screen.getByRole("button", { name: "Format" }));

    expect(textMock).toHaveBeenCalledOnce();
    expect(textMock).toHaveBeenCalledWith("abc-uuid", "EPUB/text/chapter-001.xhtml", "<p>正文</p>");
    // 回填后视图同步为格式化结果，并变脏，保存可用。
    const formatted = screen
      .getByLabelText("EPUB/text/chapter-001.xhtml")
      .querySelector(".cm-content") as HTMLTextAreaElement;
    expect(formatted.value).toBe("<p>正文</p>\n");
    // 回填后变脏，保存可用。
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(updateMock).toHaveBeenCalledOnce();
  });

  it("全部格式化落盘并重载当前文件", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await screen.findByText("chapter-001.xhtml");
    await openChapterFile(user);
    await user.click(screen.getByRole("button", { name: "Format all" }));

    expect(formatAllMock).toHaveBeenCalledOnce();
    expect(formatAllMock).toHaveBeenCalledWith("abc-uuid");
    // 批量后重载当前文件。
    await vi.waitFor(() => expect(fileMock).toHaveBeenCalledTimes(2));
  });

  it("Ctrl+S 触发保存，干净态下无操作", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await screen.findByText("chapter-001.xhtml");
    await openChapterFile(user);
    await fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(updateMock).not.toHaveBeenCalled();

    const editor = await screen.findByLabelText("EPUB/text/chapter-001.xhtml");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "<p>改</p>");
    await fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(updateMock).toHaveBeenCalledOnce();
    expect(updateMock).toHaveBeenCalledWith("abc-uuid", "EPUB/text/chapter-001.xhtml", "<p>改</p>");
  });

  it("只读的 container.xml 无格式化与保存按钮", async () => {
    const user = userEvent.setup();
    treeMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: {
          files: [
            {
              name: "META-INF",
              path: "META-INF",
              editable: false,
              children: [
                {
                  name: "container.xml",
                  path: "META-INF/container.xml",
                  editable: false,
                  children: null,
                },
              ],
            },
          ],
          file_count: 1,
        } satisfies BuildOutcome,
      }),
    );
    fileMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: { content: "<container/>", editable: false, formattable: true },
      }),
    );
    previewMock.mockReturnValue(fakeCommand({ status: "ok", data: "第一章 引子" }));
    render(BuildPage);

    // 点选只读文件：只读徽标在，无操作按钮。
    await user.click(await screen.findByText("container.xml"));
    expect(await screen.findByText("Read-only")).not.toBeNull();
    expect(screen.queryByRole("button", { name: "Format" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
    expect(textMock).not.toHaveBeenCalled();
  });

  it("脏态下格式化整理当前文本而不丢弃修改", async () => {
    const user = userEvent.setup();
    mockLoaded();
    render(BuildPage);

    await screen.findByText("chapter-001.xhtml");
    await openChapterFile(user);
    const editor = await screen.findByLabelText("EPUB/text/chapter-001.xhtml");
    await user.clear(editor.querySelector(".cm-content") as Element);
    await user.type(editor.querySelector(".cm-content") as Element, "<p>改</p>");
    // 脏态下格式化按钮仍可用。
    const formatButton = screen.getByRole("button", { name: "Format" });
    expect((formatButton as HTMLButtonElement).disabled).toBe(false);
    await user.click(formatButton);
    // 送的是编辑器当前文本，而非落盘版本。
    expect(textMock).toHaveBeenCalledOnce();
    expect(textMock).toHaveBeenCalledWith("abc-uuid", "EPUB/text/chapter-001.xhtml", "<p>改</p>");
  });
});
