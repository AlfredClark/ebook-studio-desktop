import { describe, expect, it } from "vitest";
import { cn } from "#libs/utils/shadcn-svelte";

describe("cn", () => {
  it("合并字符串、数组与对象形式的类名", () => {
    expect(cn("px-2", ["py-1", "text-sm"], { block: true, hidden: false })).toBe(
      "px-2 py-1 text-sm block",
    );
  });

  it("忽略假值", () => {
    expect(cn("px-2", undefined, null, false, "")).toBe("px-2");
  });

  it("冲突的 tailwind 工具类以后者为准", () => {
    expect(cn("p-4 text-sm", "p-2")).toBe("text-sm p-2");
  });
});
