//! 契约导出：`tauri-specta` 构建器初始化与 `bindings.ts` 生成入口。

use crate::commands::collect_commands;
use tauri_specta::Builder;

/// 构建 `specta` 构建器并注册全部命令（命令清单见 `commands` 的 `collect_commands!`）
pub fn init_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands!())
}

/// 挂载事件系统；命令处理器由 `lib.rs` 的 `invoke_handler` 挂载
pub fn setup(app: &tauri::App, builder: &Builder<tauri::Wry>) {
    builder.mount_events(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use specta_typescript::Typescript;

    /// 导出 `commands` 的 TS 绑定。
    ///
    /// 相对路径按 crate 根解析（`cargo test` 的工作目录即 `src-tauri/`），
    /// CI 会在测试后校验导出结果与提交内容一致。
    #[test]
    fn export_ts_bindings() {
        let builder = init_builder();

        builder
            .export(Typescript::default(), "../src/libs/commands/bindings.ts")
            .expect("Failed to export TypeScript bindings");
    }
}
