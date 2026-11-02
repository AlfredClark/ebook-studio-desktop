import { describe, expect, it } from "vitest";
import {
  SHORTCUT_MODIFIER_ORDER,
  keyToMainKey,
  keyToModifier,
  normalizeRecorded,
  splitShortcut,
  type ShortcutModifier,
} from "#libs/shortcut/shortcut";

describe("快捷键字符串工具", () => {
  it("修饰键顺序固定为 Ctrl/Alt/Shift/Super", () => {
    expect([...SHORTCUT_MODIFIER_ORDER]).toEqual(["Ctrl", "Alt", "Shift", "Super"]);
  });

  it("按加号切分并收窄空白与空片段", () => {
    expect(splitShortcut("Ctrl+Shift+S")).toEqual(["Ctrl", "Shift", "S"]);
    expect(splitShortcut("  Alt + Space ")).toEqual(["Alt", "Space"]);
    expect(splitShortcut("")).toEqual([]);
    expect(splitShortcut("Ctrl++S")).toEqual(["Ctrl", "S"]);
  });

  it("修饰键映射含 Meta 归一", () => {
    expect(keyToModifier("Control")).toBe("Ctrl");
    expect(keyToModifier("Alt")).toBe("Alt");
    expect(keyToModifier("Shift")).toBe("Shift");
    expect(keyToModifier("Meta")).toBe("Super");
    expect(keyToModifier("a")).toBeNull();
  });

  it("主键映射覆盖常用键", () => {
    expect(keyToMainKey("a")).toBe("A");
    expect(keyToMainKey("5")).toBe("5");
    expect(keyToMainKey(" ")).toBe("Space");
    expect(keyToMainKey("F12")).toBe("F12");
    expect(keyToMainKey("f5")).toBe("F5");
    expect(keyToMainKey("ArrowUp")).toBe("Up");
    expect(keyToMainKey("Delete")).toBe("Delete");
  });

  it("控制键与未知键不记为主键", () => {
    for (const key of [
      "Control",
      "Enter",
      "Escape",
      "Backspace",
      "AudioVolumeUp",
      "Unidentified",
    ]) {
      expect(keyToMainKey(key)).toBeNull();
    }
  });

  it("归一化按规范顺序拼接", () => {
    const modifiers = new Set<ShortcutModifier>(["Shift", "Ctrl"]);
    expect(normalizeRecorded(modifiers, "S")).toBe("Ctrl+Shift+S");
  });

  it("缺主键或零修饰键拒绝提交", () => {
    expect(normalizeRecorded(new Set<ShortcutModifier>(["Ctrl"]), null)).toBeNull();
    expect(normalizeRecorded(new Set<ShortcutModifier>(), "S")).toBeNull();
  });
});
