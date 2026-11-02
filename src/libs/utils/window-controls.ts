// 无边框窗口控制薄封装：调用失败一律回落为默认值且不抛错，
// 浏览器预览与单测环境（无 Tauri 运行时）下调用方无需分支处理。
import type { UnlistenFn } from "@tauri-apps/api/event";

/** 当前环境是否支持窗口控制（Tauri 窗口内为真，浏览器预览为假） */
export async function isWindowControlsAvailable(): Promise<boolean> {
  try {
    await currentWindow();
    return true;
  } catch {
    return false;
  }
}

/** 是否置顶，成功返回真 */
export async function isWindowAlwaysOnTop(): Promise<boolean> {
  try {
    return (await currentWindow()).isAlwaysOnTop();
  } catch {
    return false;
  }
}

/** 置顶与取消置顶之间切换，返回切换后的状态 */
export async function toggleAlwaysOnTopWindow(): Promise<boolean> {
  try {
    const window = await currentWindow();
    await window.setAlwaysOnTop(!(await window.isAlwaysOnTop()));
    return await window.isAlwaysOnTop();
  } catch {
    return false;
  }
}

/** 最小化，成功返回真 */
export async function minimizeWindow(): Promise<boolean> {
  try {
    await (await currentWindow()).minimize();
    return true;
  } catch {
    return false;
  }
}

/** 最大化与还原之间切换，成功返回真 */
export async function toggleMaximizeWindow(): Promise<boolean> {
  try {
    await (await currentWindow()).toggleMaximize();
    return true;
  } catch {
    return false;
  }
}

/** 关闭窗口，成功返回真 */
export async function closeWindow(): Promise<boolean> {
  try {
    await (await currentWindow()).close();
    return true;
  } catch {
    return false;
  }
}

/** 是否已最大化，失败回落为假 */
export async function isWindowMaximized(): Promise<boolean> {
  try {
    return await (await currentWindow()).isMaximized();
  } catch {
    return false;
  }
}

/** 订阅窗口尺寸变化，失败回落为空（调用方无需解绑） */
export async function onWindowResized(handler: () => void): Promise<UnlistenFn | null> {
  try {
    return await (await currentWindow()).onResized(handler);
  } catch {
    return null;
  }
}

// 运行时动态导入：模块顶层不触碰 Tauri API，浏览器与 Node 下 import 本文件不抛错。
async function currentWindow() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}
