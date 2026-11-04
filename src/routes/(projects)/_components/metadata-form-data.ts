// 元数据表单纯逻辑：主题词与简介输入的解析与回填；最终校验以 Rust 后端为准。
// 分隔符接受连续空格、半角逗号、中文逗号与竖线，保序去重。

/** 把主题词输入框文本解析为数组：去空白、丢空、保序去重。 */
export function parseSubjectsInput(text: string): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const part of text.split(/[,，|\s]+/)) {
    if (part !== "" && !seen.has(part)) {
      seen.add(part);
      result.push(part);
    }
  }
  return result;
}

/** 把简介输入框文本解析为数组：按行切分、去每行首尾空白、丢空行，保序不去重。 */
export function parseDescriptionInput(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

/** 把简介数组回填为输入框文本：每行一行。 */
export function formatDescriptionInput(description: string[]): string {
  return description.join("\n");
}

/** 把主题词数组回填为输入框文本：单行逗号分隔。 */
export function formatSubjectsInput(subjects: string[]): string {
  return subjects.join(", ");
}
