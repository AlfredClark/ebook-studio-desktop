import { MediaQuery } from "svelte/reactivity";

const DEFAULT_MOBILE_BREAKPOINT = 768;

export class IsMobile extends MediaQuery {
  constructor(breakpoint: number = DEFAULT_MOBILE_BREAKPOINT) {
    super(`max-width: ${breakpoint - 1}px`);
  }
}

/** 共享默认断点实例：同断点多处订阅复用一个监听，避免重复注册 */
let sharedInstance: IsMobile | null = null;

/** 取共享实例：惰性创建，首用发生在组件初始化之后（测试垫片已就绪），禁止模块级直接 new */
export function getSharedIsMobile(): IsMobile {
  if (!sharedInstance) {
    sharedInstance = new IsMobile();
  }
  return sharedInstance;
}
