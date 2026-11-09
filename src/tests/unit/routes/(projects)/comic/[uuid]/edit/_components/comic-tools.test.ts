import { describe, expect, it } from "vitest";
import { toolWithRunningJob } from "../../../../../../../../routes/(projects)/comic/[uuid]/edit/_components/comic-tools";

describe("图片工具恢复", () => {
  it("无在跑作业时保持当前选择", () => {
    expect(toolWithRunningJob({ convert: false, compress: false, resize: false })).toBeNull();
  });

  it("有在跑作业时打开对应工具", () => {
    expect(toolWithRunningJob({ convert: true, compress: false, resize: false })).toBe("convert");
    expect(toolWithRunningJob({ convert: false, compress: true, resize: false })).toBe("compress");
    expect(toolWithRunningJob({ convert: false, compress: false, resize: true })).toBe("resize");
  });

  it("互斥兜底：多路在跑按固定顺序取首个", () => {
    expect(toolWithRunningJob({ convert: true, compress: true, resize: true })).toBe("convert");
    expect(toolWithRunningJob({ convert: false, compress: true, resize: true })).toBe("compress");
  });
});
