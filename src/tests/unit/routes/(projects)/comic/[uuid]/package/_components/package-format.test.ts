import { describe, expect, it } from "vitest";
import {
  countValidation,
  formatPackageSize,
  formatPackageTime,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/package/_components/package-format";
import type { ValidationMessage } from "#libs/commands/types";

function message(severity: ValidationMessage["severity"]): ValidationMessage {
  return { severity, id: "OPF-002", message: "missing", path: null, line: null, column: null };
}

describe("漫画打包产物展示", () => {
  it("字节数按 B / KB / MB 分档", () => {
    expect(formatPackageSize(0)).toBe("0 B");
    expect(formatPackageSize(512)).toBe("512 B");
    expect(formatPackageSize(2048)).toBe("2.0 KB");
    expect(formatPackageSize(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("UTC 时间转本地展示，非法回落原文", () => {
    expect(formatPackageTime("not-a-date")).toBe("not-a-date");
    const rendered = formatPackageTime("2026-11-05T00:00:00Z");
    expect(rendered).not.toBe("2026-11-05T00:00:00Z");
    expect(rendered.length).toBeGreaterThan(0);
  });

  it("校验计数只统计错误与警告", () => {
    expect(
      countValidation([
        message("fatal"),
        message("error"),
        message("warning"),
        message("info"),
        message("usage"),
      ]),
    ).toEqual({ errors: 2, warnings: 1 });
    expect(countValidation([])).toEqual({ errors: 0, warnings: 0 });
  });
});
