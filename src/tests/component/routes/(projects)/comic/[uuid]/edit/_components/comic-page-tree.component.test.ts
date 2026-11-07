import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { tick } from "svelte";
import Sortable from "sortablejs";
import type { Options as SortableFullOptions, SortableEvent } from "sortablejs";
import type { WorkingVolume } from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-order";
import ComicPageTree from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-page-tree.svelte";

// Sortable 截桩：库本身不跑，只捕获各列表实例的 options；测试人调 onStart/onEnd
// 模拟手势，落点 DOM 顺序走真实 jsdom 读出（挪节点 + 调 onEnd）。
vi.mock("sortablejs", () => {
  class FakeSortable {
    static instances: { el: HTMLElement; options: SortableFullOptions }[] = [];
    constructor(el: HTMLElement, options: SortableFullOptions) {
      FakeSortable.instances.push({ el, options });
    }
    destroy(): void {}
  }
  return { default: FakeSortable };
});

type FakeSortable = { instances: { el: HTMLElement; options: SortableFullOptions }[] };
const sortableInstances = () => (Sortable as unknown as FakeSortable).instances;

// 页序树拖拽接线：落点提交按 DOM 顺序，折叠悬停展开，拖后点击屏蔽。
const selectMock = vi.hoisted(() => vi.fn());
const dropOrderMock = vi.hoisted(() => vi.fn());
const renameMock = vi.hoisted(() => vi.fn());
const deleteRequestMock = vi.hoisted(() => vi.fn());

function volumes(): WorkingVolume[] {
  return [
    { id: "v0", title: null, pages: ["a.jpg", "b.jpg"] },
    { id: "v1", title: "第1话", pages: ["第1话/01.jpg"] },
  ];
}

function renderTree(
  current: WorkingVolume[] = volumes(),
  sizes: Map<string, number> = new Map(),
  selectedPath: string | null = null,
  locateRequest = 0,
) {
  return render(ComicPageTree, {
    props: {
      volumes: current,
      selectedPath,
      sizes,
      locateRequest,
      onSelect: selectMock,
      onDropOrder: dropOrderMock,
      onRename: renameMock,
      onDeleteRequest: deleteRequestMock,
    },
  });
}

/** 取某话页列表的 Sortable 实例。 */
function pageInstance(volumeId: string) {
  const found = sortableInstances().find(
    (entry) => (entry.el as HTMLElement).dataset.volumeList === volumeId,
  );
  if (found == null) {
    throw new Error(`missing page list for ${volumeId}`);
  }
  return found.options;
}

/** 取话列表实例（grouped 才有）。 */
function volumeInstance() {
  const found = sortableInstances().find(
    (entry) => (entry.el as HTMLElement).dataset.volumeList == null,
  );
  if (found == null) {
    throw new Error("missing volume list");
  }
  return found.options;
}

async function startDrag(options: SortableFullOptions, item: Element) {
  options.onStart?.({ item } as unknown as SortableEvent);
  // 直接调回调不经过事件派发，手动刷 Svelte 状态。
  await tick();
}

function endDrag(options: SortableFullOptions, item: Element) {
  options.onEnd?.({ item } as unknown as SortableEvent);
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.useRealTimers();
  sortableInstances().length = 0;
});

/** 分话默认折叠：先点开全部话头，再操作行。 */
async function expandAll() {
  await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
  await fireEvent.click(screen.getByRole("button", { name: /第1话/ }));
}

describe("页序树拖拽", () => {
  it("分话默认折叠，点击话头后展开行", async () => {
    renderTree();
    expect(screen.queryByRole("button", { name: /a\.jpg/ })).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    expect(screen.getByRole("button", { name: /a\.jpg/ })).not.toBeNull();
  });

  it("点击行触发选中回调", async () => {
    renderTree();
    await expandAll();
    await fireEvent.click(screen.getByRole("button", { name: /a\.jpg/ }));
    expect(selectMock).toHaveBeenCalledOnce();
    expect(selectMock).toHaveBeenCalledWith("a.jpg");
  });

  it("落点按 DOM 顺序提交", async () => {
    const { container } = renderTree();
    await expandAll();
    const body = container.querySelector('[data-volume-list="v0"]');
    expect(body).not.toBeNull();
    const rowA = screen.getByRole("button", { name: /a\.jpg/ });
    const rowB = screen.getByRole("button", { name: /b\.jpg/ });
    // 模拟库已把 b 搬到 a 之前。
    body!.insertBefore(rowB, rowA);
    const options = pageInstance("v0");
    await startDrag(options, rowB);
    expect(screen.getByRole("navigation").getAttribute("aria-busy")).toBe("true");
    endDrag(options, rowB);
    expect(dropOrderMock).toHaveBeenCalledOnce();
    expect(dropOrderMock).toHaveBeenCalledWith({
      volumeIds: ["v0", "v1"],
      pages: new Map([
        ["v0", ["b.jpg", "a.jpg"]],
        ["v1", ["第1话/01.jpg"]],
      ]),
    });
  });

  it("无效落点提交原序，节点保留", async () => {
    renderTree();
    await expandAll();
    const options = pageInstance("v0");
    const rowA = screen.getByRole("button", { name: /a\.jpg/ });
    await startDrag(options, rowA);
    endDrag(options, rowA);
    expect(dropOrderMock).toHaveBeenCalledOnce();
    expect(dropOrderMock).toHaveBeenCalledWith({
      volumeIds: ["v0", "v1"],
      pages: new Map([
        ["v0", ["a.jpg", "b.jpg"]],
        ["v1", ["第1话/01.jpg"]],
      ]),
    });
    // 同序提交不动 DOM：被拖行仍在文档中。
    expect(rowA.isConnected).toBe(true);
    expect(screen.getByRole("button", { name: /a\.jpg/ })).not.toBeNull();
  });

  it("拖后点击被屏蔽一次", async () => {
    renderTree();
    await expandAll();
    const options = pageInstance("v0");
    const rowA = screen.getByRole("button", { name: /a\.jpg/ });
    await startDrag(options, rowA);
    endDrag(options, rowA);
    await fireEvent.click(rowA);
    expect(selectMock).not.toHaveBeenCalled();
  });

  it("悬停折叠话头计时展开", async () => {
    vi.useFakeTimers();
    const { container } = renderTree();
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    expect(screen.queryByRole("button", { name: /01\.jpg/ })).toBeNull();
    const header = container.querySelector('[data-volume-header="1"]');
    expect(header).not.toBeNull();
    document.elementFromPoint = vi.fn().mockReturnValue(header);
    const options = pageInstance("v0");
    const rowA = screen.getByRole("button", { name: /a\.jpg/ });
    await startDrag(options, rowA);
    await fireEvent.mouseMove(window, { clientX: 30, clientY: 200 });
    await vi.advanceTimersByTimeAsync(450);
    // 行按钮名取文件名：跨话路径只显示 basename。
    expect(screen.getByRole("button", { name: /01\.jpg/ })).not.toBeNull();
    endDrag(options, rowA);
  });

  it("话拖拽悬停折叠话头不展开", async () => {
    vi.useFakeTimers();
    const { container } = renderTree();
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    const header = container.querySelector('[data-volume-header="1"]');
    document.elementFromPoint = vi.fn().mockReturnValue(header);
    const options = volumeInstance();
    const section = container.querySelector('section[data-volume-section][data-id="v0"]');
    expect(section).not.toBeNull();
    await startDrag(options, section as Element);
    await fireEvent.mouseMove(window, { clientX: 30, clientY: 200 });
    await vi.advanceTimersByTimeAsync(450);
    expect(screen.queryByRole("button", { name: /01\.jpg/ })).toBeNull();
    endDrag(options, section as Element);
  });

  it("拖话头到另一话头提交话排序", async () => {
    const { container } = renderTree();
    const sections = [...container.querySelectorAll("section[data-volume-section]")];
    expect(sections.length).toBe(2);
    // 模拟库已把第 2 话搬到前面。
    sections[0].parentElement!.insertBefore(sections[1], sections[0]);
    const options = volumeInstance();
    await startDrag(options, sections[1] as HTMLElement);
    endDrag(options, sections[1] as HTMLElement);
    expect(dropOrderMock).toHaveBeenCalledOnce();
    expect(dropOrderMock).toHaveBeenCalledWith({
      volumeIds: ["v1", "v0"],
      pages: new Map(),
    });
  });

  it("改名提交走回调，空标题内联报错不提交", async () => {
    renderTree();
    const pencils = screen.getAllByRole("button", { name: /Rename episode/ });
    await fireEvent.click(pencils[1]);
    const input = screen.getByRole("textbox");
    await fireEvent.input(input, { target: { value: "  新话  " } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(renameMock).toHaveBeenCalledOnce();
    expect(renameMock).toHaveBeenCalledWith(1, "新话");

    vi.clearAllMocks();
    const freshPencils = screen.getAllByRole("button", { name: /Rename episode/ });
    await fireEvent.click(freshPencils[0]);
    const emptyInput = screen.getByRole("textbox");
    await fireEvent.input(emptyInput, { target: { value: "   " } });
    await fireEvent.keyDown(emptyInput, { key: "Enter" });
    expect(renameMock).not.toHaveBeenCalled();
    expect(screen.getByText("Episode title is required")).not.toBeNull();
  });

  it("序号按总数补零对齐", () => {
    const pages = Array.from({ length: 12 }, (_, i) => `p${String(i + 1).padStart(2, "0")}.jpg`);
    renderTree([{ id: "v", title: null, pages }]);
    expect(screen.getByText("01")).not.toBeNull();
    expect(screen.getByText("12")).not.toBeNull();
    // 未补零的个位数不应出现。
    expect(screen.queryByText("1", { exact: true })).toBeNull();
  });

  it("定位请求展开并露出当前页", async () => {
    renderTree(volumes(), new Map(), "第1话/01.jpg", 1);
    await tick();
    expect(screen.getByRole("button", { name: /01\.jpg/ })).not.toBeNull();
  });

  it("行尾展示图片大小，缺失不展示", async () => {
    renderTree(volumes(), new Map([["a.jpg", 2048]]));
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    expect(screen.getByText("2.0 KB")).not.toBeNull();
    // 无大小记录的行不渲染大小列。
    expect(screen.queryAllByText("0 B").length).toBe(0);
  });

  it("仅空话显示删除按钮，点击走删除请求", async () => {
    renderTree([
      { id: "empty", title: "空话", pages: [] },
      { id: "body", title: "正文", pages: ["b.jpg"] },
    ]);
    const deletes = screen.getAllByRole("button", { name: /Delete episode/ });
    expect(deletes.length).toBe(1);
    expect(screen.getByTitle("Move all pages out before deleting")).not.toBeNull();
    await fireEvent.click(deletes[0]);
    expect(deleteRequestMock).toHaveBeenCalledOnce();
    expect(deleteRequestMock).toHaveBeenCalledWith(0);
  });
});
