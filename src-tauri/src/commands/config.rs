//! 配置命令：`get` / `update` / `reset` 薄封装，写锁与迁移逻辑在 `cores::config`。

use crate::cores::config::{self, Config, ConfigPatch};
use crate::cores::types::CommandResult;

/// 读取完整应用配置；各字段缺失或无法识别时逐项回落默认值
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_config(app: tauri::AppHandle) -> CommandResult<Config> {
    Ok(config::load_config(&app))
}

/// 局部更新配置：只写提交的字段，返回写后的完整配置（前端据此回写内存态，无需再读一次）。
/// 界面语言亦经此切换——运行时 `rust_i18n` 由 `config::apply_runtime_effects` 统一同步。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn update_config(app: tauri::AppHandle, patch: ConfigPatch) -> CommandResult<Config> {
    Ok(config::update(&app, &patch)?)
}

/// 重置全部已知配置项为默认值，返回写后的完整配置
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn reset_config(app: tauri::AppHandle) -> CommandResult<Config> {
    Ok(config::reset(&app)?)
}
