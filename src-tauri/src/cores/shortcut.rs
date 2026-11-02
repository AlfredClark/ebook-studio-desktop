//! 显示应用的全局快捷键：配置驱动的动态注册，触发时显示主窗口。
//!
//! 注册时机有两处：启动时按存量配置注册一次（`plugins::shortcut::setup`），
//! 用户改值时注销旧值、注册新值（经 `cores::config::update` 的 `sync_shortcut_or_rollback`）。
//! 空字符串表示未设置，只注销不注册。

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// 解析用户配置的快捷键字符串；去空白后为空表示未设置，返回 `None`
pub fn parse(input: &str) -> anyhow::Result<Option<Shortcut>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<Shortcut>()
        .map(Some)
        .map_err(|err| anyhow::anyhow!("invalid shortcut {trimmed:?}: {err:#}"))
}

/// 同步全局快捷键：值未变直接跳过；否则先注销旧值再注册新值。
/// 旧值注销失败只记日志（手改配置写坏旧值不应挡住新值生效），新值注册失败如实返回。
pub fn sync(app: &AppHandle, before: &str, after: &str) -> anyhow::Result<()> {
    if before == after {
        return Ok(());
    }
    let manager = app.global_shortcut();
    if !before.trim().is_empty()
        && let Err(err) = manager.unregister(before.trim())
    {
        log::warn!("failed to unregister old shortcut {before:?}: {err:#}");
    }
    let Some(next) = parse(after)? else {
        return Ok(());
    };
    manager
        .register(next)
        .map_err(|err| anyhow::anyhow!("failed to register shortcut {after:?}: {err:#}"))?;
    log::info!("registered show-window shortcut {after:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_means_unset() {
        for raw in ["", "   ", "\t\n "] {
            assert!(parse(raw).expect("never fails").is_none());
        }
    }

    #[test]
    fn parses_supported_format() {
        for raw in ["Ctrl+Shift+S", "ctrl+shift+s", "  Alt+Space  ", "Super+F12"] {
            assert!(parse(raw).expect("parses").is_some(), "{raw} should parse");
        }
    }

    #[test]
    fn rejects_broken_format() {
        for raw in ["Ctrl++S", "Ctrl+Shift", "NotAKey", "Ctrl+S+A"] {
            assert!(parse(raw).is_err(), "{raw} should fail");
        }
    }
}
