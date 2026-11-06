import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatPaddedPageRange,
  pageFileName,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/files/_components/comic-files";

describe("漫画文件展示", () => {
  it("字节数按单位展示", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 MB");
  });

  it("页码范围按总页数位数补零", () => {
    expect(formatPaddedPageRange(1, 100, 205)).toEqual({ start: "001", end: "100" });
    expect(formatPaddedPageRange(201, 205, 205)).toEqual({ start: "201", end: "205" });
    expect(formatPaddedPageRange(1, 2, 2)).toEqual({ start: "1", end: "2" });
  });

  it("取 POSIX 路径的文件名", () => {
    expect(pageFileName("001.jpg")).toBe("001.jpg");
    expect(pageFileName("第1卷/001.jpg")).toBe("001.jpg");
    expect(pageFileName("extra/cover.png")).toBe("cover.png");
  });
});
