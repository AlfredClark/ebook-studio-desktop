import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../../../routes/(main)/settings/_components/shortcut-settings.svelte";

// 纯替身：分组依赖的后端配置与提示全部 mock，只验证接线——
// 展示、录制状态机、提交调用与成功静默/失败 sticky 提示分支。
const updateConfigMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({
  loading: vi.fn(() => 1),
  dismiss: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));
const configStateMock = vi.hoisted(() => ({ value: { shortcut_show: "" } }));

vi.mock("#libs/hooks/config.svelte", () => ({
  configState: configStateMock,
  updateConfig: updateConfigMock,
}));

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

afterEach(() => {
  cleanup();
});

beforeEach(() => {
  vi.clearAllMocks();
  configStateMock.value = { shortcut_show: "" };
  // 成功路径：写后回填配置状态并返回真，与真实 `updateConfig` 的回写行为一致
  updateConfigMock.mockImplementation(async (patch: { shortcut_show?: string }) => {
    configStateMock.value = {
      shortcut_show: patch.shortcut_show ?? configStateMock.value.shortcut_show,
    };
    return true;
  });
});

describe("快捷键设置分组", () => {
  /** 在录制态按下按键：jsdom 下直接派发，组件监听 window keydown */
  function press(key: string): void {
    window.dispatchEvent(new KeyboardEvent("keydown", { key }));
  }

  it("渲染快捷键分组及未设置态", () => {
    render(Component);

    expect(screen.getByText("Shortcuts")).not.toBeNull();
    expect(screen.getByText("Show app")).not.toBeNull();
    expect(screen.getByText("Not set")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Edit shortcut" })).not.toBeNull();
  });

  it("存量快捷键按徽章加号展示", () => {
    configStateMock.value = { shortcut_show: "Ctrl+Shift+S" };
    render(Component);

    expect(screen.getByText("Ctrl")).not.toBeNull();
    expect(screen.getByText("Shift")).not.toBeNull();
    expect(screen.getByText("S")).not.toBeNull();
    expect(screen.queryByText("Not set")).toBeNull();
  });

  it("录制组合后 Enter 提交并静默成功", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Edit shortcut" }));
    press("Control");
    press("Shift");
    press("s");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));

    await waitFor(() => {
      expect(updateConfigMock).toHaveBeenCalledWith({ shortcut_show: "Ctrl+Shift+S" });
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(toastMocks.success).not.toHaveBeenCalled();
    // 成功后退出录制态，编辑按钮恢复
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Edit shortcut" })).not.toBeNull();
    });
  });

  it("Esc 取消录制且不提交", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Edit shortcut" }));
    press("Control");
    press("s");
    press("Escape");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Edit shortcut" })).not.toBeNull();
    });
    expect(updateConfigMock).not.toHaveBeenCalled();
  });

  it("仅修饰键时 Enter 不提交", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Edit shortcut" }));
    press("Control");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));

    // 停留录制态：确认按钮保持禁用且无提交
    expect((screen.getByRole("button", { name: "Confirm" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    expect(updateConfigMock).not.toHaveBeenCalled();
  });

  it("Backspace 清空为未设置并提交", async () => {
    const user = userEvent.setup();
    configStateMock.value = { shortcut_show: "Ctrl+Shift+S" };
    render(Component);

    await user.click(screen.getByRole("button", { name: "Edit shortcut" }));
    press("Backspace");

    await waitFor(() => {
      expect(updateConfigMock).toHaveBeenCalledWith({ shortcut_show: "" });
    });
  });

  it("注册失败时弹必须手动关闭的错误提示", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await user.click(screen.getByRole("button", { name: "Edit shortcut" }));
    press("Control");
    press("s");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));

    await waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledWith(
        "Failed to register shortcut",
        expect.objectContaining({ duration: Infinity }),
      );
    });
    expect(toastMocks.success).not.toHaveBeenCalled();
  });
});
