// vite define 注入的全局常量类型声明（与 vite.config.ts 的 define 保持一致）
// 类型直接引用 JSON 文件字面量推导，与配置源天然同步
import type pkg from "../package.json";
import type tauriConf from "../src-tauri/tauri.conf.json";

declare global {
  const __APP_TAURI_CONF__: typeof tauriConf;
  const __APP_PKG__: typeof pkg;
}

export {};
