// 快捷键字符串的展示切分与录制归一化：后端按 `Shortcut::from_str` 解析，
// 前端只产出其可解析子集（修饰键 Ctrl/Alt/Shift/Super 在前 + 单个主键）。
// `Meta` 统一归一为 `Super`（macOS Command 的跨平台稳定名，见后端 `cores::shortcut`）。

/** 修饰键的规范顺序：展示与归一化共用，顺序错后端会拒收 */
export const SHORTCUT_MODIFIER_ORDER = ["Ctrl", "Alt", "Shift", "Super"] as const;

/** 规范修饰键名 */
export type ShortcutModifier = (typeof SHORTCUT_MODIFIER_ORDER)[number];

/** 按 `+` 切分展示：收窄空白并过滤空片段，`Ctrl+Shift+S` → `["Ctrl", "Shift", "S"]` */
export function splitShortcut(value: string): string[] {
  return value
    .split("+")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

/** 按键名映射到修饰键；非修饰键返回 `null`（`Meta` 即 macOS Command，归一为 `Super`） */
export function keyToModifier(key: string): ShortcutModifier | null {
  switch (key) {
    case "Control":
      return "Ctrl";
    case "Alt":
      return "Alt";
    case "Shift":
      return "Shift";
    case "Meta":
      return "Super";
    default:
      return null;
  }
}

/** 经后端校验的命名主键白名单（`KeyboardEvent.key` 口径，大小写敏感处已收窄） */
const NAMED_KEYS: ReadonlyMap<string, string> = new Map([
  ["Delete", "Delete"],
  ["End", "End"],
  ["Home", "Home"],
  ["Insert", "Insert"],
  ["PageUp", "PageUp"],
  ["PageDown", "PageDown"],
  ["ArrowUp", "Up"],
  ["ArrowDown", "Down"],
  ["ArrowLeft", "Left"],
  ["ArrowRight", "Right"],
  ["Tab", "Tab"],
  ["PrintScreen", "PrintScreen"],
  ["ScrollLock", "ScrollLock"],
  ["Pause", "Pause"],
  ["CapsLock", "CapsLock"],
  ["NumLock", "NumLock"],
]);

/**
 * 按键名映射到主键；返回 `null` 表示不记为按键——修饰键、确认/取消/清空控制键
 * （Enter/Escape/Backspace）与未知键都由调用方作流程控制或忽略。 纯字符串入参保证 node 单测可跑，不依赖 DOM 的
 * `KeyboardEvent`。
 */
export function keyToMainKey(key: string): string | null {
  if (keyToModifier(key) !== null) return null;
  if (key === "Enter" || key === "Escape" || key === "Backspace") return null;
  if (key === " ") return "Space";
  if (key.length === 1) return key.toUpperCase();
  if (/^F\d{1,2}$/i.test(key)) return key.toUpperCase();
  return NAMED_KEYS.get(key) ?? null;
}

/** 归一化录制结果：修饰键按规范顺序 + 单个主键；主键缺失或零修饰键返回 `null` （零修饰键的裸键不允许提交，避免注册一个永远按不出的组合）。 */
export function normalizeRecorded(
  modifiers: ReadonlySet<ShortcutModifier>,
  key: string | null,
): string | null {
  if (key === null) return null;
  const ordered = SHORTCUT_MODIFIER_ORDER.filter((modifier) => modifiers.has(modifier));
  if (ordered.length === 0) return null;
  return [...ordered, key].join("+");
}
