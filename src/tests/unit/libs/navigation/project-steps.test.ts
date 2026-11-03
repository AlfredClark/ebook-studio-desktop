import { describe, expect, it } from "vitest";
import { PROJECT_STEPS, resolveProjectStep } from "#libs/navigation/project-steps";

describe("项目步骤注册表", () => {
  it("六步齐备且顺序固定：元数据 -> 文件 -> 编辑 -> 构建 -> 打包 -> 导出", () => {
    expect(PROJECT_STEPS.map((step) => step.slug)).toEqual([
      "metadata",
      "files",
      "edit",
      "build",
      "package",
      "export",
    ]);
  });

  it("按路径末段命中当前步骤", () => {
    for (const step of PROJECT_STEPS) {
      expect(resolveProjectStep(`/novel/some-uuid/${step.slug}`)).toBe(step);
      expect(resolveProjectStep(`/comic/some-uuid/${step.slug}`)).toBe(step);
    }
  });

  it("未登记的末段回落首步", () => {
    expect(resolveProjectStep("/novel/some-uuid/unknown")).toBe(PROJECT_STEPS[0]);
  });
});
