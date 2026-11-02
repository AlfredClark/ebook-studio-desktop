//! 外部打开插件初始化：前端经网关统一放行，后端只做注册。

use tauri::Runtime;
use tauri::plugin::Plugin;

/// 外部打开插件：让前端用系统默认程序打开链接或文件
///
/// 安全网关在前端 `src/libs/utils/opener.ts` 的 `openExternal`（先剥 `git+` 前缀再限 `http(s)`），
/// 禁止绕过该网关直引 `@tauri-apps/plugin-opener` 的 `openUrl`。当前两处后端 `open_path`
/// 入参均为后端派生的固定应用目录（见 `cores/system.rs`），无前端可控输入，故此处无需逐参校验；
/// 若新增接受前端 URL / 路径的 opener 命令，必须在此加后端二次校验（`http(s)` 限域 + 路径收敛）。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_opener::Builder::default().build()
}
