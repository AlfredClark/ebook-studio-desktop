import type {
  CloseBehavior,
  CommandError,
  Config,
  ConfigPatch,
  Config_Serialize,
  Locale,
  SystemInfo,
  UpdateInfo,
  commands as rawCommands,
} from "#libs/commands/bindings";
import type { EnhancedCommand } from "#libs/commands/cores";

/** 命令的结算结果：`ok` 分支携带数据，`error` 分支携带失败信息 */
export type AnyResult<D = unknown, E = unknown> =
  { status: "ok"; data: D } | { status: "error"; error: E };
/** 命令失败的可能来源：Rust 侧的 `CommandError`，或 IPC 传输层的 `Error` */
export type CommandFailure<Z extends AnyResult> = ErrData<Z> | Error;
/** 任意命令函数：`never[]` 表示"任意参数"，任何函数都能赋值给它，又不必引入 `any` */
export type AnyFn = (...args: never[]) => Promise<AnyResult>;
/** 从命令函数推导其结算结果的类型 */
export type FnResult<F extends AnyFn> = Awaited<ReturnType<F>>;
/** 取结果的 `ok` 分支数据类型 */
export type OkData<Z extends AnyResult> = Extract<Z, { status: "ok" }>["data"];
/** 取结果的 `error` 分支类型 */
export type ErrData<Z extends AnyResult> = Extract<Z, { status: "error" }>["error"];
/** 把 bindings 里的命令函数包装成同参数、返回 `EnhancedCommand` 的函数 */
export type WrappedFn<F extends AnyFn> = (...args: Parameters<F>) => EnhancedCommand<FnResult<F>>;
/** 由 bindings 推导出的命令表原始类型 */
export type CommandsMap = typeof rawCommands;
/** 增强后的命令表类型：与 bindings 同名同参，但返回值是 `EnhancedCommand` */
export type EnhancedCommands = { [K in keyof CommandsMap]: WrappedFn<CommandsMap[K]> };

export type {
  CloseBehavior,
  CommandError,
  Config,
  ConfigPatch,
  Config_Serialize,
  Locale,
  SystemInfo,
  UpdateInfo,
};
