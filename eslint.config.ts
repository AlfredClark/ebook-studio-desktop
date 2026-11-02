import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import svelte from "eslint-plugin-svelte";
import { defineConfig, globalIgnores, includeIgnoreFile } from "eslint/config";
import globals from "globals";
import svelteParser from "svelte-eslint-parser";
import ts from "typescript-eslint";
import svelteConfig from "./svelte.config.js";

const gitignorePath = fileURLToPath(new URL("./.gitignore", import.meta.url));

// vite.config.ts 的 define 注入的全局常量（类型声明见 src/app.d.ts），TS 侧由 app.d.ts 提供，ESLint 需显式登记
const viteDefineGlobals = {
  __APP_TAURI_CONF__: "readonly",
  __APP_PKG__: "readonly",
};

export default defineConfig(
  // 包含 .gitignore
  includeIgnoreFile(gitignorePath),
  // 全局忽略配置
  globalIgnores(["src/libs/commands/bindings.ts"]),
  // 规则集扩展
  {
    extends: [
      js.configs.recommended,
      ...ts.configs.recommended,
      ...svelte.configs["flat/recommended"],
      prettier,
      ...svelte.configs["flat/prettier"],
    ],
  },
  // 全局默认运行环境
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },

  // 前端 TS 源码配置
  {
    files: ["src/**/*.ts", "src/**/*.svelte.ts"],
    languageOptions: {
      globals: { ...globals.browser, ...viteDefineGlobals },
    },
  },

  // Svelte 模板组件与 rune 模块专有配置（.svelte.ts 内可用 $state 等 runes，需同走 svelte 解析器）
  {
    files: ["src/**/*.svelte", "src/**/*.svelte.ts", "src/**/*.svelte.js"],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: ts.parser,
        extraFileExtensions: [".svelte"],
        svelteConfig,
      },
      globals: { ...globals.browser, ...viteDefineGlobals },
    },
  },

  // shadcn-svelte 生成组件：原生 <a href> / 外部链接属性等非 SvelteKit 导航，
  // 关闭仅适用于 SvelteKit 内部导航的误报规则
  {
    files: ["src/components/shadcn-svelte/**/*.svelte"],
    rules: {
      "svelte/no-navigation-without-resolve": "off",
    },
  },

  // Node 脚本与配置文件
  {
    files: ["*.config.ts", "eslint.config.ts", "scripts/**/*.mjs", "scripts/**/*.ts"],
    languageOptions: {
      globals: { ...globals.node },
    },
  },
);
