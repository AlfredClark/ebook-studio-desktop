import { describe, expect, it } from "vitest";
import { commands as rawCommands } from "#libs/commands/bindings";
import commands, { commands as namedCommands } from "#libs/commands/index";

// 纯断言：入口只做透出与包装映射，无副作用、不调后端；
// 包装表应与生成物同键但引用不同，证明映射真实执行而非直接透出原始表。
describe("commands 入口", () => {
  it("默认导出与命名导出为同一对象", () => {
    expect(namedCommands).toBe(commands);
  });

  it("包装表与生成物键一致，无遗漏无多余", () => {
    expect(Object.keys(commands).sort()).toEqual(Object.keys(rawCommands).sort());
  });

  it("每个命令都被包装而非直接透出", () => {
    for (const key of Object.keys(rawCommands) as (keyof typeof rawCommands)[]) {
      expect(typeof commands[key]).toBe("function");
      expect(commands[key]).not.toBe(rawCommands[key]);
    }
  });
});
