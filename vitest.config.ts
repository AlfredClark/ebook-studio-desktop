import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";
import tauriConf from "./src-tauri/tauri.conf.json" with { type: "json" };
import pkg from "./package.json" with { type: "json" };

// 单元测试配置：复用 SvelteKit 插件以获得 Svelte 编译能力；路径别名（#assets 等）
// 由 package.json 的 imports 字段提供、Vite 原生解析，因此这里不需要再抄一份 resolve.alias。
// 用两个 project 隔离环境：纯逻辑跑 node（不加 browser 解析条件，否则 `$app/*`
// 客户端模块会误判 BROWSER 而崩）；组件测试跑 jsdom + svelteTesting。
// 测试集中在 src/tests 下按源路径镜像：unit 放纯逻辑，component 放组件（目录已隔离，无需 exclude）。
// 构建常量与 vite.config.ts 同源（值仍以两个 json 文件为准，此处只做透传）。
export default defineConfig({
  plugins: [sveltekit()],
  define: {
    __APP_TAURI_CONF__: JSON.stringify(tauriConf),
    __APP_PKG__: JSON.stringify(pkg),
  },
  test: {
    projects: [
      {
        test: {
          name: "unit",
          environment: "node",
          include: ["src/tests/unit/**/*.{test,spec}.{js,ts}"],
        },
      },
      {
        plugins: [svelteTesting()],
        test: {
          name: "component",
          environment: "jsdom",
          include: ["src/tests/component/**/*.component.{test,spec}.{js,ts}"],
        },
      },
    ],
  },
});
