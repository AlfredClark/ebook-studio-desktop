import { error as logError } from "@tauri-apps/plugin-log";
import type { AnyFn, AnyResult, CommandFailure, FnResult, OkData, WrappedFn } from "./types";

/** 命令调用的链式封装：取值、分支、副作用三条路都收敛到同一个结算结果上。 */
///
/// 回调内只做幂等赋值 / toast 且保证不抛错：`success` / `failed` 回调抛错只上报不扩散，
/// `await` 仍得原结果，关键分支若放回调内会形成"状态未落但结果为 ok"的不一致。
/// 多步分支 / 跨状态机流转（如语言切换的 `setLocale`）应在 `await result()`
/// 按 `status` 分支后执行。

// 取值（失败或数据为 null/undefined 时回落默认值，默认值可省略）：
//   const config = await commands.getConfig().value(fallback);
// 需要自己按成败分支：
//   const result = await commands.getConfig().result();
// 事务（回调可为 async；await 整条链会等回调执行完）：
//   await commands.updateConfig({ locale: "zh-CN" }).success(applyLocale).failed(showError);
export class EnhancedCommand<R extends AnyResult> implements PromiseLike<R> {
  private readonly onSuccess: ((data: OkData<R>) => unknown)[] = [];
  private readonly onFailure: ((failure: CommandFailure<R>) => unknown)[] = [];
  private readonly settled: Promise<R>;

  constructor(invocation: Promise<R>) {
    this.settled = invocation
      // IPC 层的 rejection（传输错误等）也归一化到 error 分支，因此 await 永不 reject
      .then(
        (result) => result,
        (reason: unknown) => ({ status: "error", error: reason }) as R,
      )
      // 回调在结算链内部按注册顺序串行执行，await 因此天然等到它们跑完
      .then(async (result) => {
        const handlers = result.status === "ok" ? this.onSuccess : this.onFailure;
        if (result.status === "error" && handlers.length === 0) {
          reportCommandFailure("unhandled command failure", result.error);
        }
        for (const handler of handlers) {
          try {
            await handler(result.status === "ok" ? result.data : result.error);
          } catch (reason) {
            reportCommandFailure("command handler failed", reason);
          }
        }
        return result;
      });
  }

  then<TResult1 = R>(
    onFulfilled?: ((value: R) => TResult1 | PromiseLike<TResult1>) | null,
  ): Promise<TResult1> {
    return this.settled.then(onFulfilled);
  }

  /** 完整结果，需要按 status 分支时用 */
  result(): Promise<R> {
    return this.settled;
  }

  /** 取数据；失败或数据为 null/undefined 时返回 fallback（省略则返回 undefined） */
  // 注意：`value()` 把“后端错 / IPC 断 / 数据 null”压成同一个回落值，
  // 关键取值（如语言）请用 `result()` 按 `status` 分支，避免把回落误当真值
  value(): Promise<NonNullable<OkData<R>> | undefined>;
  value<D>(fallback: D): Promise<NonNullable<OkData<R>> | D>;
  async value<D>(fallback?: D): Promise<NonNullable<OkData<R>> | D | undefined> {
    const result = await this.settled;
    return result.status === "ok" && result.data != null ? result.data : fallback;
  }

  /** 成功时的处理逻辑；回调可为 async，await 整条链会等它执行完 */
  success(handler: (data: OkData<R>) => unknown): this {
    this.onSuccess.push(handler);
    return this;
  }

  /** 失败时的处理逻辑；注册后即视为已处理，不会再走自动上报 */
  failed(handler: (failure: CommandFailure<R>) => unknown): this {
    this.onFailure.push(handler);
    return this;
  }
}

/** 把 bindings 里的原始命令包成 EnhancedCommand */
export function wrapFn<F extends AnyFn>(fn: F): WrappedFn<F> {
  return ((...args: Parameters<F>) =>
    new EnhancedCommand(fn(...args) as Promise<FnResult<F>>)) as WrappedFn<F>;
}

/** 统一的失败上报：Tauri 内写入日志文件，其它环境（如单元测试）退回控制台，且永不抛错 */
// 控制台直写的白名单之一（另一处是 `error-boundary` 的同步留底）：调用方一律经此函数上报，禁止各处散写 `console.*`
export function reportCommandFailure(message: string, detail: unknown): void {
  const text = `[commands] ${message}: ${describe(detail)}`;
  console.error(text);
  try {
    void logError(text).catch(() => {});
  } catch {
    // 非 Tauri 环境（window.__TAURI_INTERNALS__ 不存在）时忽略
  }
}

/** 把任意错误值转成可读文本 */
function describe(detail: unknown): string {
  if (detail instanceof Error) {
    const brief = `${detail.name}: ${detail.message}`;
    // 开发环境附堆栈便于排障；生产仅保留简述，避免日志膨胀
    if (detail.stack && import.meta.env.DEV) return `${brief}\nStack:\n${detail.stack}`;
    return brief;
  }
  if (typeof detail === "string") return detail;
  try {
    return JSON.stringify(detail) ?? String(detail);
  } catch {
    return String(detail);
  }
}
