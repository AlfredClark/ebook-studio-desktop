// Tauri 没有 Node.js 服务器来做 SSR，使用静态适配器并回退到 `index.html`，将网站置于 SPA 模式
export const ssr = false;

// 注意：不在此文件的 `load()` 里调后端命令做配置水合——Tauri `invoke`
// 底层走 `window.fetch("ipc://...")`，SvelteKit 会在 `load` 执行期间告警
// “请用传给 `load` 的 `fetch`”，而 `invoke` 接不进 `event.fetch`。
// 本项目本就是 SPA，`load` 相对首帧并无时序优势，水合搬到 `+layout.svelte`
// 的挂载期后台执行，行为与原来一致（不阻塞首帧、失败回落）。
