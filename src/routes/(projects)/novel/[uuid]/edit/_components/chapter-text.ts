// 章节正文行切分：与后端 `update_chapter_in` 归一化口径一致（去首尾空白、丢弃空行），
// 前端落盘前先切分，后端再断言一次；dirty 对比走编辑器原始字符串，不走本函数。
/** 编辑器字符串切分为落盘行：兼容 `\r\n`，去首尾空白并丢弃空行。 */
export function splitChapterLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** 落盘行拼回编辑器字符串：单 `\n` 连接。 */
export function joinChapterLines(lines: string[]): string {
  return lines.join("\n");
}

/** Rust `char` 索引换算为 CodeMirror 列偏移（UTF-16 单位）： 中文常用字落在 BMP，两者一致；超平面字符逐字换算；越界钳制。 */
export function charSpanToRange(
  lineText: string,
  start: number,
  len: number,
): { from: number; to: number } {
  const chars = Array.from(lineText);
  const safeStart = Math.max(0, Math.min(start, chars.length));
  const safeEnd = Math.max(safeStart, Math.min(start + len, chars.length));
  const from = chars.slice(0, safeStart).join("").length;
  const to = from + chars.slice(safeStart, safeEnd).join("").length;
  return { from, to };
}
