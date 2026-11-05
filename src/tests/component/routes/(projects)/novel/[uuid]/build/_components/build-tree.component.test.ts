import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { BuildFileNode } from "../../../../../../../../libs/commands/types";
import BuildTree from "../../../../../../../../routes/(projects)/novel/[uuid]/build/_components/build-tree.svelte";

const selectMock = vi.hoisted(() => vi.fn());

const files: BuildFileNode[] = [
  {
    name: "EPUB",
    path: "EPUB",
    editable: false,
    children: [
      {
        name: "text",
        path: "EPUB/text",
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
      { name: "nav.xhtml", path: "EPUB/nav.xhtml", editable: true, children: null },
    ],
  },
  { name: "mimetype", path: "mimetype", editable: false, children: null },
];

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("构建文件树", () => {
  it("渲染目录与文件，点击文件抛选中", async () => {
    const user = userEvent.setup();
    render(BuildTree, { props: { files, selected: null, onSelect: selectMock } });

    expect(screen.getByText("EPUB")).not.toBeNull();
    expect(screen.getByText("mimetype")).not.toBeNull();

    await user.click(screen.getByText("chapter-001.xhtml"));
    expect(selectMock).toHaveBeenCalledOnce();
    expect(selectMock).toHaveBeenCalledWith("EPUB/text/chapter-001.xhtml");
  });

  it("选中文件高亮，只读文件挂锁", () => {
    render(BuildTree, {
      props: { files, selected: "mimetype", onSelect: selectMock },
    });

    const active = screen.getByText("mimetype").closest("button");
    expect(active?.getAttribute("aria-current")).toBe("true");
    // 选中态高亮底色。
    expect(active?.className).toContain("bg-muted");
    // 只读文件行：文件图标 + 锁图标共两个 svg。
    expect(active?.querySelectorAll("svg")).toHaveLength(2);
    const editable = screen.getByText("nav.xhtml").closest("button");
    expect(editable?.getAttribute("aria-current")).toBeNull();
    expect(editable?.querySelectorAll("svg")).toHaveLength(1);
  });

  it("目录可折叠收起", async () => {
    const user = userEvent.setup();
    render(BuildTree, { props: { files, selected: null, onSelect: selectMock } });

    const trigger = screen.getByText("EPUB").closest("button");
    expect(trigger?.getAttribute("aria-expanded")).toBe("true");
    await user.click(screen.getByText("EPUB"));
    expect(trigger?.getAttribute("aria-expanded")).toBe("false");
    // 收起态内容仍挂载（CSS 隐藏），仅断言折叠状态翻转。
    await user.click(screen.getByText("EPUB"));
    expect(trigger?.getAttribute("aria-expanded")).toBe("true");
  });

  it("箭头跟随本节点展开态，不受祖先影响", async () => {
    const user = userEvent.setup();
    render(BuildTree, { props: { files, selected: null, onSelect: selectMock } });
    const chevronClass = (name: string): string =>
      screen.getByText(name).closest("button")?.querySelector("svg")?.getAttribute("class") ?? "";

    // EPUB 展开、嵌套 text 收起：两者箭头方向必须不同。
    expect(chevronClass("EPUB")).toContain("rotate-90");
    expect(chevronClass("text")).not.toContain("rotate-90");
    // 展开 text 后箭头翻转。
    await user.click(screen.getByText("text"));
    expect(chevronClass("text")).toContain("rotate-90");
  });
});
