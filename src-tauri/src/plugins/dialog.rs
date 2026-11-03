//! 文件对话框插件初始化：封面选图经前端 `open()` 弹框，后端只做注册。
//!
//! 选出的源路径不直接可信：复制落盘前的存在性 / 类型 / 大小校验在
//! `features::project_metadata::import_cover_in` 内完成，此处不做参数网关。

use tauri::Runtime;
use tauri::plugin::Plugin;

/// 文件对话框插件：让前端弹出系统文件选择框
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_dialog::init()
}
