import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tauriConf from "./src-tauri/tauri.conf.json" with { type: "json" };
import pkg from "./package.json" with { type: "json" };

// 获取 TAURI_DEV_HOST
const host = process.env.TAURI_DEV_HOST;

// 参考：https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    tailwindcss(),
    await sveltekit(),
    paraglideVitePlugin({
      project: "./src/libs/i18n/project.inlang",
    }),
  ],

  define: {
    __APP_TAURI_CONF__: JSON.stringify(tauriConf),
    __APP_PKG__: JSON.stringify(pkg),
  },

  // 防止 Vite 清空终端屏幕，保留 Rust 编译输出日志
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    ws: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },

  // 路径别名由 package.json 的 imports 字段统一提供（Node subpath imports，Vite 原生解析），
  // 此处不要再复制一份，避免两处声明漂移
}));
