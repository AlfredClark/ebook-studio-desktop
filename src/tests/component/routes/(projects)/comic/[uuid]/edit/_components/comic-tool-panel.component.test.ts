import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ComicToolPanel from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-tool-panel.svelte";

// jsdom 缺少指针捕获与 ResizeObserver，bits-ui 组件用得到，仅在本文件内就地补齐。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

// 右栏图片工具：工具切换走回调，参数区按工具切换，按钮门禁透传。
const runMock = vi.hoisted(() => vi.fn());
const clearMock = vi.hoisted(() => vi.fn());
const cancelMock = vi.hoisted(() => vi.fn());

function renderPanel(
  tool: "convert" | "compress" = "convert",
  target: "jpeg" | "png" | "webp" = "jpeg",
  running = false,
) {
  return render(ComicToolPanel, {
    props: {
      tool,
      target,
      quality: 85,
      webpLossless: false,
      pngLevel: 6,
      forceConvert: false,
      compressScale: 100,
      compressGrayscale: false,
      compressQuality: 85,
      compressWebpLossless: false,
      compressPngLevel: 6,
      running,
      canRun: false,
      hasOutcome: false,
      progress: null,
      onRun: runMock,
      onClear: clearMock,
      onCancel: cancelMock,
    },
  });
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("图片工具面板", () => {
  it("转换工具展示格式与参数控件，按钮默认禁用", () => {
    renderPanel("convert", "jpeg");
    expect(screen.getByRole("combobox", { name: "Target format" })).not.toBeNull();
    expect(screen.getByText("Quality")).not.toBeNull();
    expect(screen.getByText("Convert files already in target format")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Apply" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Reset" })).toHaveProperty("disabled", true);
  });

  it("压缩工具展示缩放与灰度参数控件", () => {
    renderPanel("compress");
    expect(screen.getByText("Scale")).not.toBeNull();
    expect(screen.getByText("Grayscale")).not.toBeNull();
    expect(screen.queryByText("Target format")).toBeNull();
  });

  it("按目标格式切换参数控件", () => {
    const { unmount } = renderPanel("convert", "jpeg");
    expect(screen.getByText("Quality")).not.toBeNull();
    expect(screen.queryByText("Lossless")).toBeNull();
    expect(screen.queryByText("Compression level")).toBeNull();
    unmount();

    renderPanel("convert", "webp");
    expect(screen.getByText("Quality")).not.toBeNull();
    expect(screen.getByText("Lossless")).not.toBeNull();
    expect(screen.queryByText("Compression level")).toBeNull();
    cleanup();
    vi.clearAllMocks();

    renderPanel("convert", "png");
    expect(screen.queryByText("Quality")).toBeNull();
    expect(screen.getByText("Compression level")).not.toBeNull();
    expect(screen.queryByText("Lossless")).toBeNull();
    cleanup();
    vi.clearAllMocks();
  });

  it("切换工具下拉更新参数区", async () => {
    const user = userEvent.setup();
    renderPanel("convert");
    // jsdom 无布局，选项按 `data-value` 定位。
    await user.click(screen.getByRole("combobox", { name: "Tool" }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="compress"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="compress"]') as HTMLElement);
    expect(screen.getByText("Scale")).not.toBeNull();
    expect(screen.queryByText("Target format")).toBeNull();
  });

  it("运行时主按钮切取消并可用，点击走取消回调", async () => {
    const user = userEvent.setup();
    renderPanel("convert", "jpeg", true);

    const cancel = screen.getByRole("button", { name: "Cancel" });
    expect(cancel).toHaveProperty("disabled", false);
    await user.click(cancel);
    expect(cancelMock).toHaveBeenCalledOnce();
    expect(runMock).not.toHaveBeenCalled();
  });
});
