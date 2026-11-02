//! 插件初始化层：每个 Tauri 插件一个文件，只暴露 `init()` 供 `lib.rs` 注册；
//! 运行时收尾统一走 `plugins::setup`（对标 `cores::setup`），保持 `lib.rs` 的 `setup` 只有两行。

use tauri::Runtime;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod autostart;
pub mod clipboard;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod deep_link;
pub mod log;
pub mod opener;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod shortcut;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod single_instance;
pub mod store;
pub mod system_fonts;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod updater;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod window_state;

/// 为 `tauri::Builder` 补充平台感知的插件注册方法，使 `lib.rs` 的注册链保持单一形式
pub trait BuilderExt<R: Runtime> {
    /// 注册开机自启插件；移动端为空操作（该平台未声明 `autostart` 依赖）
    #[must_use]
    fn with_autostart(self) -> Self;
    /// 注册自动更新插件；移动端为空操作（该平台未声明 `updater` 依赖）
    #[must_use]
    fn with_updater(self) -> Self;
    /// 注册窗口状态插件；移动端为空操作（该平台未声明 `window-state` 依赖）
    #[must_use]
    fn with_window_state(self) -> Self;
    /// 注册单实例插件；移动端为空操作（该平台未声明 `single-instance` 依赖）。
    /// 必须先于深链插件注册（官方顺序要求），调用处保持该先后顺序
    #[must_use]
    fn with_single_instance(self) -> Self;
    /// 注册全局快捷键插件；移动端为空操作（该平台未声明 `global-shortcut` 依赖）
    #[must_use]
    fn with_shortcut(self) -> Self;
    /// 注册深链插件；移动端为空操作（该平台未声明依赖）
    #[must_use]
    fn with_deep_link(self) -> Self;
}

// 桌面端实现收窄到 `Wry`：单实例构造器是 `Wry` 具体的，泛型 `R` 调不动
// `single_instance::init()`；应用只用 `Builder::default()`（即 `Builder<Wry>`），收窄无影响
#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl BuilderExt<tauri::Wry> for tauri::Builder<tauri::Wry> {
    fn with_autostart(self) -> Self {
        self.plugin(autostart::init())
    }

    fn with_updater(self) -> Self {
        self.plugin(updater::init())
    }

    fn with_window_state(self) -> Self {
        self.plugin(window_state::init())
    }

    fn with_single_instance(self) -> Self {
        self.plugin(single_instance::init())
    }

    fn with_shortcut(self) -> Self {
        self.plugin(shortcut::init())
    }

    fn with_deep_link(self) -> Self {
        self.plugin(deep_link::init())
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl<R: Runtime> BuilderExt<R> for tauri::Builder<R> {
    fn with_autostart(self) -> Self {
        self
    }

    fn with_updater(self) -> Self {
        self
    }

    fn with_window_state(self) -> Self {
        self
    }

    fn with_single_instance(self) -> Self {
        self
    }

    fn with_shortcut(self) -> Self {
        self
    }

    fn with_deep_link(self) -> Self {
        self
    }
}

/// 应用启动时的插件运行时收尾（对标 `cores::setup`）：深链协议的 Win/Linux 运行时注册 +
/// 显示应用快捷键的按存量注册；各插件的具体收尾放在各自文件的 `setup` 内，这里只做串联
pub fn setup(app: &tauri::App) {
    deep_link::setup(app);
    shortcut::setup(app);
}
