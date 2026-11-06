import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import type { WorkingVolume } from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-order";
import ComicPageTree from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-page-tree.svelte";

// 页序树拖拽接线：用模拟 DataTransfer 走 dragstart→dragover→drop，断言 onMove 参数；
// jsdom 无真实拖拽引擎，只验证事件接线与自放 guard，不验证浏览器拖拽行为。
const selectMock = vi.hoisted(() => vi.fn());
const moveMock = vi.hoisted(() => vi.fn());
const moveVolumeMock = vi.hoisted(() => vi.fn());
const renameMock = vi.hoisted(() => vi.fn());
const deleteRequestMock = vi.hoisted(() => vi.fn());

function volumes(): WorkingVolume[] {
  return [
    { title: null, pages: ["a.jpg", "b.jpg"] },
    { title: "第1卷", pages: ["第1卷/01.jpg"] },
  ];
}

function renderTree(current: WorkingVolume[] = volumes(), sizes: Map<string, number> = new Map()) {
  return render(ComicPageTree, {
    props: {
      volumes: current,
      selectedPath: null,
      sizes,
      onSelect: selectMock,
      onMove: moveMock,
      onMoveVolume: moveVolumeMock,
      onRename: renameMock,
      onDeleteRequest: deleteRequestMock,
    },
  });
}

/** 单次手势共用的 DataTransfer 替身：setData 存载荷，getData 回放。 */
function makeTransfer() {
  const store = new Map<string, string>();
  return {
    setData: vi.fn((type: string, data: string) => store.set(type, data)),
    getData: vi.fn((type: string) => store.get(type) ?? ""),
    effectAllowed: "",
    dropEffect: "",
    types: ["text/plain"],
  };
}

function dragEvent(type: string, transfer: ReturnType<typeof makeTransfer>): Event {
  // jsdom 无 DragEvent 构造器：处理器只用 preventDefault/dataTransfer/clientY，
  // 普通 Event 挂载同样属性即可走通接线。
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: transfer });
  // jsdom 布局全零：clientY=0 落在上半区判定之外，稳定走 after。
  Object.defineProperty(event, "clientY", { value: 0 });
  return event;
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

/** 分卷默认折叠：先点开全部卷头，再操作行。 */
async function expandAll() {
  await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
  await fireEvent.click(screen.getByRole("button", { name: /第1卷/ }));
}

describe("页序树拖拽", () => {
  it("分卷默认折叠，点击卷头后展开行", async () => {
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

  it("行间放下提交跨位移动", async () => {
    renderTree();
    await expandAll();
    const transfer = makeTransfer();
    const source = screen.getByRole("button", { name: /a\.jpg/ });
    await fireEvent(source, dragEvent("dragstart", transfer));
    const target = screen.getByRole("button", { name: /b\.jpg/ });
    await fireEvent(target, dragEvent("dragover", transfer));
    await fireEvent(target, dragEvent("drop", transfer));
    expect(moveMock).toHaveBeenCalledOnce();
    expect(moveMock).toHaveBeenCalledWith(
      { volume: 0, index: 0 },
      { volume: 0, index: 1, edge: "after" },
    );
  });

  it("放在自身行上视为取消，不提交移动", async () => {
    renderTree();
    await expandAll();
    const transfer = makeTransfer();
    const source = screen.getByRole("button", { name: /a\.jpg/ });
    await fireEvent(source, dragEvent("dragstart", transfer));
    await fireEvent(source, dragEvent("dragover", transfer));
    await fireEvent(source, dragEvent("drop", transfer));
    expect(moveMock).not.toHaveBeenCalled();
  });

  it("放在卷头追加到该卷末尾，支持跨卷", async () => {
    renderTree();
    // 卷头本身始终可见：展开源卷取行，直接往折叠卷头上放。
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    const transfer = makeTransfer();
    const source = screen.getByRole("button", { name: /a\.jpg/ });
    await fireEvent(source, dragEvent("dragstart", transfer));
    const header = screen.getByRole("button", { name: /第1卷/ });
    await fireEvent(header, dragEvent("dragover", transfer));
    await fireEvent(header, dragEvent("drop", transfer));
    expect(moveMock).toHaveBeenCalledOnce();
    expect(moveMock).toHaveBeenCalledWith({ volume: 0, index: 0 }, { volume: 1, edge: "append" });
  });

  it("拖卷头到另一卷头提交卷排序", async () => {
    const { container } = renderTree();
    const headers = container.querySelectorAll('button[draggable="true"]');
    expect(headers.length).toBe(2);
    const transfer = makeTransfer();
    await fireEvent(headers[0], dragEvent("dragstart", transfer));
    await fireEvent(headers[1], dragEvent("dragover", transfer));
    await fireEvent(headers[1], dragEvent("drop", transfer));
    expect(moveVolumeMock).toHaveBeenCalledOnce();
    expect(moveVolumeMock).toHaveBeenCalledWith(0, { index: 1, edge: "after" });
  });

  it("改名提交走回调，空标题内联报错不提交", async () => {
    renderTree();
    const pencils = screen.getAllByRole("button", { name: /Rename volume/ });
    await fireEvent.click(pencils[1]);
    const input = screen.getByRole("textbox");
    await fireEvent.input(input, { target: { value: "  新卷  " } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(renameMock).toHaveBeenCalledOnce();
    expect(renameMock).toHaveBeenCalledWith(1, "新卷");

    vi.clearAllMocks();
    const freshPencils = screen.getAllByRole("button", { name: /Rename volume/ });
    await fireEvent.click(freshPencils[0]);
    const emptyInput = screen.getByRole("textbox");
    await fireEvent.input(emptyInput, { target: { value: "   " } });
    await fireEvent.keyDown(emptyInput, { key: "Enter" });
    expect(renameMock).not.toHaveBeenCalled();
    expect(screen.getByText("Volume title is required")).not.toBeNull();
  });

  it("行尾展示图片大小，缺失不展示", async () => {
    renderTree(volumes(), new Map([["a.jpg", 2048]]));
    await fireEvent.click(screen.getByRole("button", { name: /Untitled/ }));
    expect(screen.getByText("2.0 KB")).not.toBeNull();
    // 无大小记录的行不渲染大小列。
    expect(screen.queryAllByText("0 B").length).toBe(0);
  });

  it("仅空卷显示删除按钮，点击走删除请求", async () => {
    renderTree([
      { title: "空卷", pages: [] },
      { title: "正文", pages: ["b.jpg"] },
    ]);
    const deletes = screen.getAllByRole("button", { name: /Delete volume/ });
    expect(deletes.length).toBe(1);
    expect(screen.getByTitle("Move all pages out before deleting")).not.toBeNull();
    await fireEvent.click(deletes[0]);
    expect(deleteRequestMock).toHaveBeenCalledOnce();
    expect(deleteRequestMock).toHaveBeenCalledWith(0);
  });
});
