import { describe, expect, it } from "vitest";
import { formatDateTime } from "#libs/utils/datetime";

describe("formatDateTime", () => {
  it("精度到秒且含年月日", () => {
    const text = formatDateTime("2026-10-28T14:20:05Z", "en-US");
    expect(text).toContain("2026");
    expect(text).toMatch(/:05$/);
  });
});
