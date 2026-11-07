import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ProjectStepsNav from "../../../../../routes/(projects)/_components/project-steps-nav.svelte";

// 面包屑在 jsdom 下定位为隐藏子树时，用 hidden 查询兜底。
const gotoMock = vi.hoisted(() => vi.fn());

vi.mock("$app/navigation", () => ({ goto: gotoMock }));

vi.mock("$app/state", () => ({
  page: {
    url: new URL("http://localhost/novel/abc-uuid/metadata"),
    params: { uuid: "abc-uuid" },
  },
}));

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
  window.localStorage.clear();
  document.body.removeAttribute("style");
});

describe("项目步骤导航", () => {
  it("渲染返回按钮与六步，当前步高亮、其余为同级链接", () => {
    render(ProjectStepsNav, { type: "novel", uuid: "abc-uuid" });

    const nav = screen.getByRole("navigation", { name: "Project steps" });
    expect(within(nav).getByText("Metadata").getAttribute("aria-current")).toBe("page");
    expect(within(nav).getByText("Files").closest("a")?.getAttribute("href")).toBe(
      "/novel/abc-uuid/files",
    );
    expect(within(nav).getByText("Export").closest("a")?.getAttribute("href")).toBe(
      "/novel/abc-uuid/export",
    );
    // 空白拖拽区 + 窗口控制随导航栏提供，不再复用 TitleBar
    expect(nav.querySelector("[data-tauri-drag-region]")).not.toBeNull();
    expect(within(nav).getByRole("button", { name: "Close" })).not.toBeNull();
  });

  it("返回按钮按入口来源回跳，无记录回项目列表", async () => {
    const user = userEvent.setup();
    window.localStorage.clear();
    const { unmount } = render(ProjectStepsNav, { type: "comic", uuid: "abc-uuid" });

    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(gotoMock).toHaveBeenCalledWith("/projects");
    unmount();
    cleanup();

    window.localStorage.setItem("app.projects.entryFrom", "/");
    render(ProjectStepsNav, { type: "comic", uuid: "abc-uuid" });
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(gotoMock).toHaveBeenCalledWith("/");
  });
});
