import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { normalizeExternalUrl } from "#libs/utils/opener";
import Component from "../../../../../../routes/(main)/about/_components/project-links-section.svelte";

// 纯替身：外链打开与提示全部 mock，只验证项目外链分组的接线——三行渲染与打开分支。
// `normalizeExternalUrl` 是纯函数，保留真身（组件展示层要用它）。
const openExternalMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));

vi.mock("#libs/utils/opener", async (importOriginal) => {
  const actual = await importOriginal<typeof import("#libs/utils/opener")>();
  return { ...actual, openExternal: openExternalMock };
});

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
  openExternalMock.mockResolvedValue(true);
});

afterEach(() => {
  cleanup();
});

describe("项目外链分组", () => {
  it("渲染三行外链", () => {
    render(Component);

    expect(screen.getByText("Project info")).not.toBeNull();
    expect(screen.getByText("Homepage")).not.toBeNull();
    expect(screen.getByText("Repository")).not.toBeNull();
    expect(screen.getByText("Feedback")).not.toBeNull();
    expect(screen.getByText(__APP_PKG__.homepage)).not.toBeNull();
    expect(screen.getByText(__APP_PKG__.bugs.url)).not.toBeNull();
    // 仓库地址是 npm 的 `git+https://` 形式：展示层必须还原，用户不该看到传输前缀
    expect(screen.queryByText(/git\+/)).toBeNull();
    expect(screen.getByText(normalizeExternalUrl(__APP_PKG__.repository.url))).not.toBeNull();
    expect(screen.getAllByRole("button", { name: "Open" })).toHaveLength(3);
  });

  it("点击外链透传对应地址，成功不提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getAllByRole("button", { name: "Open" })[0]);

    expect(openExternalMock).toHaveBeenCalledWith(__APP_PKG__.homepage);
    expect(toastMocks.error).not.toHaveBeenCalled();
  });

  it("打开失败时提示", async () => {
    openExternalMock.mockResolvedValue(false);
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getAllByRole("button", { name: "Open" })[1]);

    expect(openExternalMock).toHaveBeenCalledWith(normalizeExternalUrl(__APP_PKG__.repository.url));
    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledOnce();
    });
  });
});
