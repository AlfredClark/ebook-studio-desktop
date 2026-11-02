import { defineConfig } from "@inlang/paraglide-js";

// Paraglide 编译器选项的唯一来源：CLI（pnpm i18n:compile）与 vite 插件都读取本文件。
// 不要再把 outdir/strategy 等参数重复写进 package.json 或 vite.config.ts——两处各写一份必然漂移。
// 体积基线（首屏包内，改动语言数时复评）：runtime.js 约 80KB（含用不上的路由/server 逻辑，
// experimentalMiddlewareSplitting=false 注定单包）+ messages 约 25KB（中英双语同包）。
// 重做阈值：语言数 >4 或 i18n 合计 >150KB 时，再评估按 locale 异步拆 messages（需改全部 m.* 调用点）。
export default defineConfig({
  outdir: "./src/libs/i18n/paraglide",
  strategy: ["localStorage", "baseLocale"],
  emitTsDeclarations: true,
  cleanOutdir: true,
});
