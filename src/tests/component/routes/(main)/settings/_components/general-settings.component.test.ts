import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { CloseBehavior, Locale } from "#libs/commands/types";
import Component from "../../../../../../routes/(main)/settings/_components/general-settings.svelte";

// jsdom 缺少指针捕获与滚动 API，bits-ui 下拉用得到，仅在本文件内就地补齐。
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

// 纯替身：分组依赖的外部状态（后端配置、运行时语言、提示）全部 mock，
// 只验证分组的接线逻辑——选项渲染、切换调用与成功/失败分支。
const updateConfigMock = vi.hoisted(() => vi.fn());
const setLocaleMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({
  loading: vi.fn(() => 1),
  dismiss: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));
const configStateMock = vi.hoisted(() => ({
  value: {
    locale: "en",
    auto_start: false,
    remember_window: false,
    auto_check_update: false,
    tray_enabled: true,
    close_behavior: "prompt",
    schema_version: 1,
  },
}));

vi.mock("#libs/hooks/config.svelte", () => ({
  configState: configStateMock,
  updateConfig: updateConfigMock,
}));

vi.mock("#libs/i18n/paraglide/runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("#libs/i18n/paraglide/runtime")>();
  return {
    ...actual,
    getLocale: () => configStateMock.value.locale,
    setLocale: setLocaleMock,
  };
});

vi.mock("#libs/utils/toast", () => ({ toast: toastMocks }));

afterEach(() => {
  cleanup();
  // bits-ui 下拉打开时给 body 加滚动锁定样式，jsdom 内卸载后不会自动还原，
  // 这里手动清理，避免泄漏到后续用例导致点击被 pointer-events 拦截。
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  configStateMock.value = {
    locale: "en",
    auto_start: false,
    remember_window: false,
    auto_check_update: false,
    tray_enabled: true,
    close_behavior: "prompt",
    schema_version: 1,
  };
  // 成功路径：写后回填配置状态并返回真，与真实 `updateConfig` 的回写行为一致。
  updateConfigMock.mockImplementation(
    async (patch: {
      locale?: Locale;
      auto_start?: boolean;
      remember_window?: boolean;
      auto_check_update?: boolean;
      tray_enabled?: boolean;
      close_behavior?: CloseBehavior;
    }) => {
      configStateMock.value = {
        locale: patch.locale ?? configStateMock.value.locale,
        auto_start: patch.auto_start ?? configStateMock.value.auto_start,
        remember_window: patch.remember_window ?? configStateMock.value.remember_window,
        auto_check_update: patch.auto_check_update ?? configStateMock.value.auto_check_update,
        tray_enabled: patch.tray_enabled ?? configStateMock.value.tray_enabled,
        close_behavior: patch.close_behavior ?? configStateMock.value.close_behavior,
        schema_version: 1,
      };
      return true;
    },
  );
});

describe("通用设置分组", () => {
  /** 在下拉中按值点选：jsdom 无布局，选项始终不可见，只能按 `data-value` 定位。 */
  async function chooseOption(
    user: ReturnType<typeof userEvent.setup>,
    triggerName: string,
    value: string,
  ): Promise<void> {
    await user.click(screen.getByRole("combobox", { name: triggerName }));
    await waitFor(() => {
      expect(document.querySelector(`[data-value="${value}"]`)).not.toBeNull();
    });
    await user.click(document.querySelector(`[data-value="${value}"]`) as HTMLElement);
  }

  it("渲染通用分组及当前取值", () => {
    render(Component);

    expect(screen.getByText("General")).not.toBeNull();
    expect(screen.getByText("Language")).not.toBeNull();
    expect(screen.getByText("Autostart")).not.toBeNull();
    expect(screen.getByText("Remember window")).not.toBeNull();
    expect(screen.getByText("Auto check for updates")).not.toBeNull();
    expect(screen.getByText("System tray")).not.toBeNull();
    expect(screen.getByText("Close behavior")).not.toBeNull();
    // 下拉触发器展示当前选中项的文案
    const languageTrigger = screen.getByRole("combobox", { name: "Language" });
    expect(languageTrigger.textContent).toContain("English");
    const closeBehaviorTrigger = screen.getByRole("combobox", { name: "Close behavior" });
    expect(closeBehaviorTrigger.textContent).toContain("Ask every time");
  });

  it("切换语言先落盘后端，成功后重载生效", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Language", "zh-CN");

    expect(updateConfigMock).toHaveBeenCalledWith({ locale: "zh-CN" });
    await vi.waitFor(() => {
      expect(setLocaleMock).toHaveBeenCalledWith("zh-CN");
    });
    expect(toastMocks.success).toHaveBeenCalled();
    expect(toastMocks.error).not.toHaveBeenCalled();
  });

  it("语言落盘失败时不重载并报错", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await chooseOption(user, "Language", "zh-CN");

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("语言落盘失败时加载提示关闭且下拉恢复可用", async () => {
    const user = userEvent.setup();
    // 生产真实失败路径：命令结算为失败（resolve 为 false 且不回写状态），而非抛错
    updateConfigMock.mockImplementationOnce(async () => false);
    render(Component);

    await chooseOption(user, "Language", "zh-CN");

    // 加载提示不自动消失，结算后必定关闭
    expect(toastMocks.loading).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ duration: Infinity }),
    );
    await vi.waitFor(() => {
      expect(toastMocks.dismiss).toHaveBeenCalled();
    });
    // finally 复位：下拉恢复可用，不会永久禁用
    await vi.waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Language" }).hasAttribute("disabled")).toBe(
        false,
      );
    });
  });

  it("自启落盘失败时开关恢复可用", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementationOnce(async () => false);
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Autostart" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(screen.getByRole("switch", { name: "Autostart" }).hasAttribute("disabled")).toBe(false);
  });

  it("自启开关渲染后端配置的当前状态", () => {
    configStateMock.value = {
      locale: "en",
      auto_start: true,
      remember_window: false,
      auto_check_update: false,
      tray_enabled: true,
      close_behavior: "prompt",
      schema_version: 1,
    };
    render(Component);

    expect(screen.getByRole("switch", { name: "Autostart" }).getAttribute("data-state")).toBe(
      "checked",
    );
  });

  it("打开自启开关经命令落盘，成功后提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Autostart" }));

    expect(updateConfigMock).toHaveBeenCalledWith({ auto_start: true });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("自启落盘失败时报错且不改语言", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Autostart" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("窗口开关渲染后端配置的当前状态", () => {
    configStateMock.value = {
      locale: "en",
      auto_start: false,
      remember_window: true,
      auto_check_update: false,
      tray_enabled: true,
      close_behavior: "prompt",
      schema_version: 1,
    };
    render(Component);

    expect(screen.getByRole("switch", { name: "Remember window" }).getAttribute("data-state")).toBe(
      "checked",
    );
  });

  it("打开窗口开关经命令落盘，成功后提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Remember window" }));

    expect(updateConfigMock).toHaveBeenCalledWith({ remember_window: true });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("窗口落盘失败时报错且不改语言", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Remember window" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("更新检查开关渲染后端配置的当前状态", () => {
    configStateMock.value = {
      locale: "en",
      auto_start: false,
      remember_window: false,
      auto_check_update: true,
      tray_enabled: true,
      close_behavior: "prompt",
      schema_version: 1,
    };
    render(Component);

    expect(
      screen.getByRole("switch", { name: "Auto check for updates" }).getAttribute("data-state"),
    ).toBe("checked");
  });

  it("打开更新检查开关经命令落盘，成功后提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Auto check for updates" }));

    expect(updateConfigMock).toHaveBeenCalledWith({ auto_check_update: true });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("更新检查落盘失败时报错且不改语言", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await user.click(screen.getByRole("switch", { name: "Auto check for updates" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("托盘开关渲染后端配置的当前状态", () => {
    configStateMock.value = {
      locale: "en",
      auto_start: false,
      remember_window: false,
      auto_check_update: false,
      tray_enabled: false,
      close_behavior: "prompt",
      schema_version: 1,
    };
    render(Component);

    expect(screen.getByRole("switch", { name: "System tray" }).getAttribute("data-state")).toBe(
      "unchecked",
    );
  });

  it("关闭托盘开关经命令落盘，成功后提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("switch", { name: "System tray" }));

    expect(updateConfigMock).toHaveBeenCalledWith({ tray_enabled: false });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("托盘关闭时最小化到托盘选项不可用", async () => {
    const user = userEvent.setup();
    configStateMock.value = {
      locale: "en",
      auto_start: false,
      remember_window: false,
      auto_check_update: false,
      tray_enabled: false,
      close_behavior: "prompt",
      schema_version: 1,
    };
    render(Component);

    await user.click(screen.getByRole("combobox", { name: "Close behavior" }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="minimize_to_tray"]')).not.toBeNull();
    });
    // bits-ui 给禁用项打 data-disabled 标记并排除出键盘导航
    expect(
      document.querySelector('[data-value="minimize_to_tray"]')?.hasAttribute("data-disabled"),
    ).toBe(true);
    expect(document.querySelector('[data-value="exit"]')?.hasAttribute("data-disabled")).toBe(
      false,
    );
  });

  it("切换关闭行为先落盘后端，成功后提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await chooseOption(user, "Close behavior", "exit");

    expect(updateConfigMock).toHaveBeenCalledWith({ close_behavior: "exit" });
    await vi.waitFor(() => {
      expect(toastMocks.success).toHaveBeenCalled();
    });
    expect(toastMocks.error).not.toHaveBeenCalled();
    expect(setLocaleMock).not.toHaveBeenCalled();
  });

  it("关闭行为落盘失败时报错且不改语言", async () => {
    const user = userEvent.setup();
    updateConfigMock.mockImplementation(async () => false);
    render(Component);

    await chooseOption(user, "Close behavior", "exit");

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalled();
    });
    expect(setLocaleMock).not.toHaveBeenCalled();
  });
});
