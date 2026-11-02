import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "../../../../components/common/confirm-dialog.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同设置页测试）。
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

const labels = {
  title: "确认退出",
  description: "未保存的内容将丢失",
  cancelLabel: "取消",
  confirmLabel: "退出",
};

const onConfirmMock = vi.hoisted(() => vi.fn());

/** 打开弹窗：bits-ui 对话框经 portal 挂载，全局查询即可 */
function renderOpen() {
  render(Component, {
    props: { open: true, ...labels, onConfirm: onConfirmMock },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("通用确认弹窗", () => {
  it("打开时渲染标题描述与双按钮", async () => {
    renderOpen();

    await screen.findByText(labels.title);
    expect(screen.getByText(labels.description)).not.toBeNull();
    expect(screen.getByRole("button", { name: labels.cancelLabel })).not.toBeNull();
    expect(screen.getByRole("button", { name: labels.confirmLabel })).not.toBeNull();
  });

  it("点确认先关弹窗再调回调", async () => {
    const user = userEvent.setup();
    renderOpen();

    await user.click(await screen.findByRole("button", { name: labels.confirmLabel }));

    expect(onConfirmMock).toHaveBeenCalledOnce();
    await vi.waitFor(() => {
      expect(screen.queryByText(labels.title)).toBeNull();
    });
  });

  it("点取消不调回调并关闭", async () => {
    const user = userEvent.setup();
    renderOpen();

    await user.click(await screen.findByRole("button", { name: labels.cancelLabel }));

    expect(onConfirmMock).not.toHaveBeenCalled();
    await vi.waitFor(() => {
      expect(screen.queryByText(labels.title)).toBeNull();
    });
  });
});
