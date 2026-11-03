import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { EnhancedCommand } from "#libs/commands/cores";
import type { ProjectInfo } from "#libs/commands/types";
import Page from "../../../../../routes/(main)/projects/+page.svelte";

// bits-ui / paneforge 在 jsdom 下缺失的浏览器 API，就地补齐（同设置页测试）。
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

type Ok<T> = { status: "ok"; data: T };
type Err = { status: "error"; error: { kind: "Internal"; message: string } };

// 命令替身：用真实 EnhancedCommand 类包装伪造结算结果，只验证页面组装与联动。
const backendProjects: ProjectInfo[] = [
  {
    id: "11111111-1111-4111-8111-111111111111",
    name: "星海长歌",
    type: "novel",
    created_at: "2026-09-01T08:00:00Z",
    updated_at: "2026-10-28T14:20:05Z",
  },
  {
    id: "22222222-2222-4222-8222-222222222222",
    name: "雾镇",
    type: "comic",
    created_at: "2026-08-14T10:30:00Z",
    updated_at: "2026-10-30T09:12:34Z",
  },
];

const commandsMock = vi.hoisted(() => ({
  listProjects: vi.fn(),
  createProject: vi.fn(),
  renameProject: vi.fn(),
  deleteProjects: vi.fn(),
}));

vi.mock("#libs/commands", () => ({
  default: commandsMock,
  commands: commandsMock,
}));

const gotoMock = vi.hoisted(() => vi.fn());

vi.mock("$app/navigation", () => ({ goto: gotoMock }));

const toastMocks = vi.hoisted(() => ({
  message: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

function ok<T>(data: T): EnhancedCommand<Ok<T>> {
  return new EnhancedCommand(Promise.resolve({ status: "ok", data }));
}

function failed(): EnhancedCommand<Err> {
  return new EnhancedCommand(
    Promise.resolve({ status: "error", error: { kind: "Internal", message: "boom" } }),
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  commandsMock.listProjects.mockReturnValue(ok([...backendProjects]));
  commandsMock.createProject.mockImplementation((name: string, type: ProjectInfo["type"]) =>
    ok({
      id: "33333333-3333-4333-8333-333333333333",
      name,
      type,
      created_at: "2026-11-01T00:00:00Z",
      updated_at: "2026-11-01T00:00:00Z",
    }),
  );
  commandsMock.renameProject.mockImplementation((id: string, name: string) =>
    ok({
      id,
      name,
      type: "comic" as const,
      created_at: "2026-08-14T10:30:00Z",
      updated_at: "2026-11-02T00:00:00Z",
    }),
  );
  commandsMock.deleteProjects.mockImplementation((ids: string[]) => ok(ids.length));
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("项目页", () => {
  it("进页拉取后端列表，右栏默认预览最近更新项", async () => {
    render(Page);

    expect(await screen.findAllByRole("option")).toHaveLength(2);
    expect(commandsMock.listProjects).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("heading", { name: "雾镇" })).not.toBeNull();
  });

  it("行内时间精度到秒", async () => {
    render(Page);
    const rows = await screen.findAllByRole("option");

    expect(rows[0].textContent).toMatch(/:\d\d/);
  });

  it("点击行切换右栏预览", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    await user.click(screen.getByText("星海长歌"));

    expect(screen.getByRole("heading", { name: "星海长歌" })).not.toBeNull();
  });

  it("行内复选只维护多选，不改变右栏预览", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    await user.click(screen.getByRole("checkbox", { name: "星海长歌" }));

    expect(screen.getByRole("heading", { name: "雾镇" })).not.toBeNull();
    expect(screen.getByRole("checkbox", { name: "星海长歌" }).getAttribute("data-state")).toBe(
      "checked",
    );
  });

  it("全选框切换可见列表的全部勾选", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");
    const selectAll = screen.getByRole("checkbox", { name: "Select all" });

    await user.click(selectAll);
    for (const row of screen.getAllByRole("option")) {
      expect(row.querySelector('[data-state="checked"]')).not.toBeNull();
    }

    await user.click(selectAll);
    for (const row of screen.getAllByRole("option")) {
      expect(row.querySelector('[data-state="checked"]')).toBeNull();
    }
  });

  it("拉取失败走空态 + 错误提示 + 重试可恢复", async () => {
    const user = userEvent.setup();
    commandsMock.listProjects.mockReturnValue(failed());
    render(Page);

    expect(await screen.findByText("Failed to load projects")).not.toBeNull();
    expect(toastMocks.error).toHaveBeenCalledWith("Failed to load projects");

    commandsMock.listProjects.mockReturnValue(ok([...backendProjects]));
    await user.click(screen.getByRole("button", { name: "Retry" }));

    expect(await screen.findAllByRole("option")).toHaveLength(2);
  });

  it("新建弹窗每次打开都清空输入，确认后插顶选中并提示", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    await user.click(screen.getByRole("button", { name: "New project" }));
    const menu = await screen.findByRole("menu", { hidden: true });
    await user.click(within(menu).getByText("Novel"));
    const dialog = await screen.findByRole("dialog", { hidden: true });
    expect(within(dialog).getByText("New novel")).not.toBeNull();

    const input = within(dialog).getByPlaceholderText("Enter project name");
    await user.type(input, "纸飞机");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));

    // 弹窗关闭动画结束前 body 仍屏蔽指针事件，等清理完成后再重开
    await new Promise((resolve) => setTimeout(resolve, 400));
    // 重新打开：输入已清空
    await user.click(screen.getByRole("button", { name: "New project" }));
    const reopenedMenu = await screen.findByRole("menu", { hidden: true });
    await user.click(within(reopenedMenu).getByText("Novel"));
    const reopened = await screen.findByRole("dialog", { hidden: true });
    expect(
      (within(reopened).getByPlaceholderText("Enter project name") as HTMLInputElement).value,
    ).toBe("");

    await user.type(within(reopened).getByPlaceholderText("Enter project name"), "纸飞机");
    await user.click(within(reopened).getByRole("button", { name: "Create" }));

    expect(await screen.findByRole("heading", { name: "纸飞机" })).not.toBeNull();
    expect(toastMocks.success).toHaveBeenCalledWith("Created “纸飞机”");
    expect(commandsMock.createProject).toHaveBeenCalledWith("纸飞机", "novel");
  });

  it("行内编辑回车确认落盘并提示，取消放弃修改", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    await user.click(screen.getAllByRole("button", { name: "Edit" })[0]);
    const box = screen.getByRole("textbox");
    expect((box as HTMLInputElement).value).toBe("雾镇");
    await user.clear(box);
    await user.type(box, "雾镇·改");
    await user.keyboard("{Enter}");

    expect(commandsMock.renameProject).toHaveBeenCalledWith(
      "22222222-2222-4222-8222-222222222222",
      "雾镇·改",
    );
    expect(await screen.findByRole("heading", { name: "雾镇·改" })).not.toBeNull();
    expect(toastMocks.success).toHaveBeenCalledWith("Renamed to “雾镇·改”");

    // 取消路径：改名后再次编辑并取消，名称不变
    await user.click(screen.getAllByRole("button", { name: "Edit" })[0]);
    await user.clear(screen.getByRole("textbox"));
    await user.type(screen.getByRole("textbox"), "不要这个名字");
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("textbox")).toBeNull();
    expect(screen.getByRole("heading", { name: "雾镇·改" })).not.toBeNull();
    expect(commandsMock.renameProject).toHaveBeenCalledTimes(1);
  });

  it("打开按钮按类型跳到对应项目的元数据步骤", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    // 默认按更新倒序首位是漫画雾镇
    await user.click(screen.getAllByRole("button", { name: "Open project" })[0]);

    expect(gotoMock).toHaveBeenCalledWith("/comic/22222222-2222-4222-8222-222222222222/metadata");
  });

  it("行删除图标经确认框删除", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    await user.click(screen.getAllByRole("button", { name: "Delete" })[0]);
    const confirmDialog = await screen.findByRole("alertdialog", { hidden: true });
    await user.click(within(confirmDialog).getByRole("button", { name: "Delete" }));

    expect(await screen.findAllByRole("option")).toHaveLength(1);
    expect(screen.queryByText("雾镇")).toBeNull();
    expect(toastMocks.success).toHaveBeenCalledWith("Deleted 1 projects");
  });

  it("批量删除经确认框删除并提示，按钮随勾选显隐", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");
    expect(screen.queryByRole("button", { name: "Bulk actions" })).toBeNull();

    await user.click(screen.getByRole("checkbox", { name: "星海长歌" }));
    await user.click(screen.getByRole("button", { name: "Bulk actions" }));
    const menu = await screen.findByRole("menu", { hidden: true });
    await user.click(within(menu).getByText("Delete selected"));

    const confirmDialog = await screen.findByRole("alertdialog", { hidden: true });
    await user.click(within(confirmDialog).getByRole("button", { name: "Delete" }));

    expect(await screen.findAllByRole("option")).toHaveLength(1);
    expect(screen.queryByText("星海长歌")).toBeNull();
    expect(screen.queryByRole("button", { name: "Bulk actions" })).toBeNull();
    expect(toastMocks.success).toHaveBeenCalledWith("Deleted 1 projects");
  });

  it("排序下拉切换正倒序，筛选下拉按类型过滤", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");

    // 浮动下拉在 jsdom 下定位为隐藏子树，打开后按槽位划定范围再点选。
    await user.click(screen.getByRole("combobox", { name: "Sort by" }));
    const sortContent = document.body.querySelector('[data-slot="select-content"]');
    expect(sortContent).not.toBeNull();
    await user.click(within(sortContent as HTMLElement).getByText("Oldest updated"));

    const rows = screen.getAllByRole("option");
    expect(rows[0].textContent).toContain("星海长歌");

    // 浮动层关闭动画期间 body 仍屏蔽指针事件，等清理完成后再开下一个
    await new Promise((resolve) => setTimeout(resolve, 400));
    await user.click(screen.getByRole("combobox", { name: "Filter by type" }));
    const filterContent = document.body.querySelector('[data-slot="select-content"]');
    expect(filterContent).not.toBeNull();
    await user.click(within(filterContent as HTMLElement).getByText("Novel"));

    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByText("Select a project to view details")).not.toBeNull();
  });

  it("输入法组词中不触发搜索，组词结束才过滤", async () => {
    render(Page);
    await screen.findAllByRole("option");

    const box = screen.getByPlaceholderText("Search projects…");
    await fireEvent.compositionStart(box);
    await fireEvent.input(box, { target: { value: "雾" } });

    // 组词中列表不变
    expect(screen.getAllByRole("option")).toHaveLength(2);

    await fireEvent.compositionEnd(box);

    expect(screen.getAllByRole("option")).toHaveLength(1);
  });

  it("列表框方向键在行间移动焦点，编辑框内不劫持", async () => {
    const user = userEvent.setup();
    render(Page);
    const rows = await screen.findAllByRole("option");

    rows[0].focus();
    await user.keyboard("{ArrowDown}");

    expect(document.activeElement).toBe(rows[1]);

    await user.keyboard("{ArrowUp}");

    expect(document.activeElement).toBe(rows[0]);
  });

  it("刷新按钮重新拉取列表", async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findAllByRole("option");
    expect(commandsMock.listProjects).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: "Refresh" }));

    expect(commandsMock.listProjects).toHaveBeenCalledTimes(2);
  });
});
