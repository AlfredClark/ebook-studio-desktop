import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ComicToolPanel from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-tool-panel.svelte";

// jsdom 缺少指针捕获 API，bits-ui 下拉用得到，仅在本文件内就地补齐。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}

// 右栏图片工具：工具切换走回调，参数区按工具切换占位，按钮门禁透传。
const runMock = vi.hoisted(() => vi.fn());
const clearMock = vi.hoisted(() => vi.fn());

function renderPanel(tool: "convert" | "compress" = "convert") {
  return render(ComicToolPanel, {
    props: {
      tool,
      running: false,
      canRun: false,
      hasOutcome: false,
      onRun: runMock,
      onClear: clearMock,
    },
  });
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("图片工具面板", () => {
  it("按工具切换参数占位，按钮默认禁用", () => {
    const { unmount } = renderPanel("convert");
    expect(screen.getByText("格式转换参数占位，后续接入")).not.toBeNull();
    expect(screen.queryByText("图片压缩参数占位，后续接入")).toBeNull();
    expect(screen.getByRole("button", { name: "Apply" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Reset" })).toHaveProperty("disabled", true);
    unmount();

    renderPanel("compress");
    expect(screen.getByText("图片压缩参数占位，后续接入")).not.toBeNull();
    expect(screen.queryByText("格式转换参数占位，后续接入")).toBeNull();
  });

  it("切换下拉更新参数区", async () => {
    const user = userEvent.setup();
    renderPanel("convert");
    // jsdom 无布局，选项按 `data-value` 定位。
    await user.click(screen.getByRole("combobox"));
    await waitFor(() => {
      expect(document.querySelector('[data-value="compress"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="compress"]') as HTMLElement);
    expect(screen.getByText("图片压缩参数占位，后续接入")).not.toBeNull();
    expect(screen.queryByText("格式转换参数占位，后续接入")).toBeNull();
  });
});
