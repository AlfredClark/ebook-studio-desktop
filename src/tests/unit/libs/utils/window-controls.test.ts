import { describe, expect, it } from "vitest";
import {
  closeWindow,
  isWindowAlwaysOnTop,
  isWindowControlsAvailable,
  isWindowMaximized,
  minimizeWindow,
  onWindowResized,
  toggleAlwaysOnTopWindow,
  toggleMaximizeWindow,
} from "#libs/utils/window-controls";

// 节点环境无 Tauri 运行时，全部分支应回落默认值且不抛错。
describe("window-controls", () => {
  it("无运行时时报告不可用", async () => {
    await expect(isWindowControlsAvailable()).resolves.toBe(false);
  });

  it("控制调用失败回落为假", async () => {
    await expect(minimizeWindow()).resolves.toBe(false);
    await expect(toggleMaximizeWindow()).resolves.toBe(false);
    await expect(closeWindow()).resolves.toBe(false);
    await expect(isWindowMaximized()).resolves.toBe(false);
    await expect(isWindowAlwaysOnTop()).resolves.toBe(false);
    await expect(toggleAlwaysOnTopWindow()).resolves.toBe(false);
  });

  it("订阅失败回落为空", async () => {
    await expect(onWindowResized(() => {})).resolves.toBeNull();
  });
});
