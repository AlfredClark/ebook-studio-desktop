import type { EnhancedCommands } from "./types";
import { commands as raw } from "./bindings";
import { wrapFn } from "./cores";

// 逐个包装 bindings 的原始命令：Object.fromEntries 会丢失键与值的对应关系，
// 因此整体断言成由 bindings 推导出的 EnhancedCommands（全项目唯一的类型断点）。
export const commands = Object.fromEntries(
  Object.entries(raw).map(([name, fn]) => [name, wrapFn(fn)]),
) as unknown as EnhancedCommands;

export default commands;
