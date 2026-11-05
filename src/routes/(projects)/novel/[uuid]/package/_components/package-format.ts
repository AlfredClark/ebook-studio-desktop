// 小说打包步骤页纯逻辑：产物大小 / 时间展示与校验计数；校验语义以后端 `epubveri` 为准。
import type { ValidationMessage } from "#libs/commands/types";

/** 字节数展示：B / KB / MB，KB 以上保留一位小数。 */
export function formatPackageSize(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

/** UTC 时间转本地展示：非法回落原文。 */
export function formatPackageTime(iso: string): string {
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) {
    return iso;
  }
  return new Date(time).toLocaleString();
}

/** 校验计数：错误（fatal + error）与警告数，汇总行展示用。 */
export function countValidation(messages: ValidationMessage[]): {
  errors: number;
  warnings: number;
} {
  let errors = 0;
  let warnings = 0;
  for (const message of messages) {
    if (message.severity === "fatal" || message.severity === "error") {
      errors += 1;
    } else if (message.severity === "warning") {
      warnings += 1;
    }
  }
  return { errors, warnings };
}
