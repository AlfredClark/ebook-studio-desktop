//! 系统能力：`panic` 钩子（崩溃兜底）与本机信息采集。

use std::fs::OpenOptions;
use std::io::Write;
use std::{env, panic};

use serde::{Deserialize, Serialize};
use specta::Type;

/// 安装 panic 钩子：先走常规日志，日志不可用时落盘到临时文件兜底
fn setup_panic_hook() {
    // 获取原有的 panic hook（保留系统的默认行为）
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        let error_log = format_crash_report(info);
        // 保留默认输出（带格式/颜色输出到 stderr）
        default_hook(info);
        // 尝试走 tauri_plugin_log / log 系统，失败则落盘兜底
        if !emit_via_log(&error_log) {
            persist_fallback(&error_log);
        }
    }));
}

/// 拼接崩溃报告：本地时间（`RFC3339` 带时区偏移）+ 位置 + 原因 + 堆栈
fn format_crash_report(info: &panic::PanicHookInfo) -> String {
    let backtrace = std::backtrace::Backtrace::force_capture();
    format!(
        "=== [CRASH PANIC] ===\nTime: {}\nLocation: {}\nReason: {}\nBacktrace:\n{}\n=====================\n",
        chrono::Local::now().to_rfc3339(),
        panic_location(info),
        panic_payload(info),
        backtrace
    )
}

/// 提取 panic 位置，缺失时回落占位
fn panic_location(info: &panic::PanicHookInfo) -> String {
    info.location().map_or_else(
        || "unknown location".into(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    )
}

/// 提取 panic 负载，`&str` / `String` 之外回落占位
fn panic_payload(info: &panic::PanicHookInfo) -> String {
    info.payload()
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "Unknown panic payload".into())
}

/// 经 `log` 上报崩溃；`logger` 不可用或二次 panic 时返回 `false` 走文件兜底
fn emit_via_log(error_log: &str) -> bool {
    if log::max_level() < log::LevelFilter::Error {
        return false;
    }
    // 使用 catch_unwind 防止 logger 内部持有锁导致死锁或二次 panic
    panic::catch_unwind(panic::AssertUnwindSafe(|| {
        log::error!(target: "panic", "{error_log}");
        log::logger().flush();
    }))
    .is_ok()
}

/// 兜底落盘到临时目录的 `my_app_crash.log`，写入失败静默忽略
///
/// 无限追加会撑满临时目录：超 512KB 时先轮转旧文件为 `.1`（仅保留一份），再写入本次崩溃
fn persist_fallback(error_log: &str) {
    const MAX_BYTES: u64 = 512 * 1024;
    let log_path = env::temp_dir().join("my_app_crash.log");
    if std::fs::metadata(&log_path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
        let rotated = log_path.with_extension("log.1");
        let _ = std::fs::remove_file(&rotated);
        let _ = std::fs::rename(&log_path, &rotated);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(error_log.as_bytes());
        let _ = file.flush();
    }
}

/// 是否运行在 `Wayland` 会话下（`Wayland` 与 `WebKitGTK` 的 `DMABUF` 渲染器存在兼容问题）
#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    env::var("XDG_SESSION_TYPE")
        .is_ok_and(|session_type| session_type.eq_ignore_ascii_case("wayland"))
        || env::var_os("WAYLAND_DISPLAY").is_some()
}

/// 是否由 `AppImage` 启动（打包环境下关掉合成模式，规避渲染异常）
#[cfg(target_os = "linux")]
fn is_app_image() -> bool {
    env::var_os("APPDIR").is_some()
}

/// 启动前的一次性系统初始化：安装 panic 钩子，并处理 Linux 下的显示环境兼容
pub fn init_system() {
    // 设置 panic 钩子
    setup_panic_hook();
    // 设置特殊环境下的兼容性环境变量
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            unsafe {
                env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
            if is_app_image() {
                unsafe {
                    env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
                }
            }
        }
    }
}

/// 真退出应用进程：关闭行为“直接退出”与关闭确认弹窗走此入口。
/// `hide` 只藏窗口不断进程，退出必须经此处；调用后进程即结束（不可单测，走真机 QA）。
pub fn quit_app(app: &tauri::AppHandle) {
    app.exit(0);
}

/// 运行平台信息：经 `tauri-plugin-os` 采集，全字段必填，前端逐行展示
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct SystemInfo {
    /// 操作系统平台（如 `linux` / `windows` / `macos`）
    pub platform: String,
    /// 操作系统版本
    pub os_version: String,
    /// 系统架构（如 `x86_64`）
    pub arch: String,
    /// 主机名
    pub hostname: String,
}

/// 采集运行平台信息；单项缺失时回落 `"unknown"`，绝不抛错
pub fn gather_system_info() -> SystemInfo {
    SystemInfo {
        platform: tauri_plugin_os::platform().to_string(),
        os_version: tauri_plugin_os::version().to_string(),
        arch: tauri_plugin_os::arch().to_string(),
        hostname: {
            let name = tauri_plugin_os::hostname();
            if name.is_empty() {
                "unknown".into()
            } else {
                name
            }
        },
    }
}

/// 组装可复制的诊断文本：应用名 + 版本 + 四行系统信息，纯函数便于单测
pub fn format_system_info(app_name: &str, app_version: &str, info: &SystemInfo) -> String {
    format!(
        "{app_name} {app_version}\nPlatform: {}\nOS version: {}\nArchitecture: {}\nHostname: {}",
        info.platform, info.os_version, info.arch, info.hostname
    )
}

/// 复制系统信息到剪贴板：版本号取打包元信息，前端只调命令不拼字符串
pub fn copy_system_info(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use anyhow::Context;
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let package = app.package_info();
    let text = format_system_info(
        &package.name,
        &package.version.to_string(),
        &gather_system_info(),
    );
    app.clipboard()
        .write_text(text)
        .context("failed to write clipboard")?;
    Ok(())
}

/// 解析诊断目录并确保存在：日志目录首次可能尚未创建，先建目录再打开
fn resolve_app_dir(app: &tauri::AppHandle, log: bool) -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context;
    use tauri::Manager;

    let dir = if log {
        app.path()
            .app_log_dir()
            .context("failed to resolve log dir")?
    } else {
        app.path()
            .app_data_dir()
            .context("failed to resolve config dir")?
    };
    std::fs::create_dir_all(&dir).context("failed to create app dir")?;
    Ok(dir)
}

/// 在系统文件管理器中打开日志目录
pub fn open_log_dir(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use anyhow::Context;
    use tauri_plugin_opener::OpenerExt;

    let dir = resolve_app_dir(app, true)?;
    let dir = dir.to_str().context("log dir path is not valid UTF-8")?;
    app.opener()
        .open_path(dir, None::<&str>)
        .context("failed to open log dir")?;
    Ok(())
}

/// 在系统文件管理器中打开配置目录（`config.json` 所在目录）
pub fn open_config_dir(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use anyhow::Context;
    use tauri_plugin_opener::OpenerExt;

    let dir = resolve_app_dir(app, false)?;
    let dir = dir.to_str().context("config dir path is not valid UTF-8")?;
    app.opener()
        .open_path(dir, None::<&str>)
        .context("failed to open config dir")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_system_info_as_plain_lines() {
        let info = SystemInfo {
            platform: "linux".into(),
            os_version: "22.04".into(),
            arch: "x86_64".into(),
            hostname: "dev-machine".into(),
        };
        assert_eq!(
            format_system_info("my-app", "0.2.0", &info),
            "my-app 0.2.0\nPlatform: linux\nOS version: 22.04\nArchitecture: x86_64\nHostname: dev-machine"
        );
    }
}
