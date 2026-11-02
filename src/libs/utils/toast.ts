import { toast as sonnerToast, type ExternalToast } from "svelte-sonner";

/** 全局默认停留时长：统一各业务提示的消失节奏，避免各处散落魔数。 */
const DEFAULT_DURATION = 3000;

/** 同类合并窗口：同 kind + 文案短时间内重复触发只保留首个，避免失败风暴堆 DOM */
const DEDUPE_WINDOW_MS = 3000;

/** 上一条已发提示：合并判断依据，仅测试复位可改 */
let lastToast: { kind: string; message: string; at: number; id: string | number } | null = null;

/** 仅测试用：复位合并记忆，隔离同 kind 同文案的用例 */
export function __resetToastDedupeForTests(): void {
  lastToast = null;
}

/** 同类合并发射：窗口内重复直接返回首个 id，不再建新提示 */
function deduped(
  kind: string,
  message: string,
  options: ExternalToast | undefined,
  emit: (message: string, options: ExternalToast) => string | number,
): string | number {
  const now = Date.now();
  if (
    lastToast &&
    lastToast.kind === kind &&
    lastToast.message === message &&
    now - lastToast.at < DEDUPE_WINDOW_MS
  ) {
    return lastToast.id;
  }
  const id = emit(message, withDefaults(options));
  lastToast = { kind, message, at: now, id };
  return id;
}

/** 补齐默认时长，调用方显式传入的选项优先。 */
function withDefaults(options?: ExternalToast): ExternalToast {
  return { closeButton: false, duration: DEFAULT_DURATION, ...options };
}

/** 全局通用提示：收敛 svelte-sonner 入口，统一默认时长。 */
export const toast = {
  /** 普通消息提示。 */
  message(message: string, options?: ExternalToast): string | number {
    return deduped("message", message, options, (text, resolved) =>
      sonnerToast.message(text, resolved),
    );
  },

  /** 成功提示。 */
  success(message: string, options?: ExternalToast): string | number {
    return deduped("success", message, options, (text, resolved) =>
      sonnerToast.success(text, resolved),
    );
  },

  /** 信息提示。 */
  info(message: string, options?: ExternalToast): string | number {
    return deduped("info", message, options, (text, resolved) => sonnerToast.info(text, resolved));
  },

  /** 警告提示。 */
  warning(message: string, options?: ExternalToast): string | number {
    return deduped("warning", message, options, (text, resolved) =>
      sonnerToast.warning(text, resolved),
    );
  },

  /** 错误提示。 */
  error(message: string, options?: ExternalToast): string | number {
    return deduped("error", message, options, (text, resolved) =>
      sonnerToast.error(text, resolved),
    );
  },

  /** 加载中提示，需配合关闭。 */
  loading(message: string, options?: ExternalToast): string | number {
    return sonnerToast.loading(message, withDefaults(options));
  },

  /** 异步任务提示：按结算状态自动切换成功 / 失败文案。 */
  promise: sonnerToast.promise,

  /** 关闭指定或全部提示。 */
  dismiss: sonnerToast.dismiss,
};
