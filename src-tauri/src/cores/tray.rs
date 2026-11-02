//! 系统托盘：图标 + 双菜单项（显示/隐藏、退出），菜单文案跟随界面语言。
//!
//! 开关控制图标显隐，关闭行为由 `config::coerce_patch` 规整：托盘关闭时“最小化到托盘”
//! 无处可去，落盘前已回落弹窗提示，此处只读生效值，不做二次决策。

use rust_i18n::t;
use tauri::{AppHandle, Manager};

/// 托盘图标 id：动态显隐与文案重建经此查找，禁止多处各起一名
const TRAY_ID: &str = "main-tray";
/// 显示/隐藏菜单项 id
const MENU_TOGGLE: &str = "tray-toggle";
/// 退出菜单项 id
const MENU_QUIT: &str = "tray-quit";

/// 托盘图标资源：专用 `tray.png`（由 `64x64.png` 复制而来，独立演进不跟随打包图标），
/// 编译期打进二进制，不走 `resources` 配置，
/// 故无需改 `tauri.conf.json`、`capabilities` 与 CSP（托盘与退出全在 Rust 侧）。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../../icons/tray.png");

/// 构建托盘图标：菜单文案按当前运行时语言生成；开关只控制显隐，图标常驻。
/// 托盘构建失败仅记日志（窗口照常可用，托盘相关入口自动降级为空操作）。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn setup(app: &tauri::App) {
    use tauri::{image::Image, tray::TrayIconBuilder};

    let handle = app.handle();
    let icon = match Image::from_bytes(TRAY_ICON_BYTES) {
        Ok(icon) => icon,
        Err(err) => {
            log::error!("failed to decode tray icon: {err:#}");
            return;
        }
    };
    let menu = match build_menu(handle) {
        Ok(menu) => menu,
        Err(err) => {
            log::error!("failed to build tray menu: {err:#}");
            return;
        }
    };
    let tray = match TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event)
        .build(handle)
    {
        Ok(tray) => tray,
        Err(err) => {
            log::error!("failed to build tray icon: {err:#}");
            return;
        }
    };
    let enabled = crate::cores::config::load_config(handle).tray_enabled;
    if let Err(err) = tray.set_visible(enabled) {
        log::warn!("failed to apply initial tray visibility: {err:#}");
    }
}

/// 构建托盘菜单：文案每次按当前语言生成，重建即换语言，无需比对 diff
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn build_menu(app: &AppHandle) -> anyhow::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};

    let toggle = MenuItemBuilder::with_id(MENU_TOGGLE, t!("tray.toggle")).build(app)?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, t!("tray.quit")).build(app)?;
    Ok(MenuBuilder::new(app).items(&[&toggle, &quit]).build()?)
}

/// 托盘菜单事件：切换主窗口显隐，真退出进程（`hide` 只藏窗口，不断进程）
/// 回调签名由 `TrayIconBuilder` 固定，`event` 按值传递无法避免
#[allow(clippy::needless_pass_by_value)]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn on_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        MENU_TOGGLE => toggle_main_window(app),
        MENU_QUIT => app.exit(0),
        other => log::warn!("unhandled tray menu item: {other}"),
    }
}

/// 托盘图标事件：仅双击切换显隐；单击故意无操作，避免与双击判定打架；
/// 右键菜单为平台默认行为，不另行处理（Linux `AppIndicator` 下双击可能不可靠，以菜单为准）
/// 回调签名由 `TrayIconBuilder` 固定，`event` 按值传递无法避免
#[allow(clippy::needless_pass_by_value)]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn on_tray_icon_event(tray: &tauri::tray::TrayIcon, event: tauri::tray::TrayIconEvent) {
    use tauri::tray::{MouseButton, TrayIconEvent};

    if matches!(
        event,
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
    ) {
        toggle_main_window(tray.app_handle());
    }
}

/// 切换主窗口显隐：可见则藏起，否则恢复、显示并聚焦
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        log::warn!("main window not found for tray toggle");
        return;
    };
    if window.is_visible().unwrap_or(true) {
        if let Err(err) = window.hide() {
            log::warn!("failed to hide main window from tray: {err:#}");
        }
        return;
    }
    show_main_window(app);
}

/// 显示主窗口：取消最小化、显示并聚焦（托盘菜单与全局“显示应用”快捷键共用）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        log::warn!("main window not found when showing");
        return;
    };
    if let Err(err) = window.unminimize().and_then(|()| window.show()) {
        log::warn!("failed to show main window: {err:#}");
        return;
    }
    let _ = window.set_focus();
}

/// 设置托盘图标显隐；托盘不存在（如构建失败）时记日志后跳过
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn set_visible(app: &AppHandle, visible: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log::warn!("tray icon not found when applying visibility");
        return;
    };
    if let Err(err) = tray.set_visible(visible) {
        log::warn!("failed to set tray visibility: {err:#}");
    }
}

/// 用当前语言重建托盘菜单文案；托盘不存在时静默跳过
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn refresh_texts(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let menu = match build_menu(app) {
        Ok(menu) => menu,
        Err(err) => {
            log::warn!("failed to rebuild tray menu: {err:#}");
            return;
        }
    };
    if let Err(err) = tray.set_menu(Some(menu)) {
        log::warn!("failed to apply rebuilt tray menu: {err:#}");
    }
}

/// 移动端无系统托盘：以下皆为空操作，保证 `config` 副作用调用方无需 `cfg` 分支
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn setup(_app: &tauri::App) {}

/// 移动端无系统托盘：以下皆为空操作，保证 `config` 副作用调用方无需 `cfg` 分支
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn set_visible(_app: &AppHandle, _visible: bool) {}

/// 移动端无系统托盘：以下皆为空操作，保证 `config` 副作用调用方无需 `cfg` 分支
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn refresh_texts(_app: &AppHandle) {}

/// 移动端无系统托盘：显示主窗口为空操作（本应用专注桌面端，此处仅保签一致）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn show_main_window(_app: &tauri::AppHandle) {}
