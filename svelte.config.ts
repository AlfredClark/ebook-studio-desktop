import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import("@sveltejs/kit").Config} */
const config: import("@sveltejs/kit").Config = {
  // 使用 vitePreprocess 支持 <style lang="postcss"> 或 <script lang="ts">
  preprocess: vitePreprocess(),
  kit: {
    // 使用适配器静态并回退到 index.html，将网站置于SPA模式
    adapter: adapter({
      fallback: "index.html",
    }),
    // 路径别名（#assets / #components / #libs）统一在 package.json 的 imports 字段声明
  },
};

export default config;
