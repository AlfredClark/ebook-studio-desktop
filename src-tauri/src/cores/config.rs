//! 配置中心：后端 `config.json` 的读写、迁移与运行时副作用（语言/自启/托盘/窗口恢复）。

use anyhow::Context;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use specta::Type;
use std::sync::{Mutex, OnceLock};
use tauri::Runtime;
use tauri_plugin_store::{Store, StoreExt};

use crate::cores::locale::Locale;

/// 应用配置在 `tauri-plugin-store` 里的文件名（落在系统应用数据目录）
pub const STORE_FILE: &str = "config.json";
/// 配置结构版本在配置文件中的键名
const KEY_SCHEMA_VERSION: &str = "schema_version";
/// 界面语言在配置文件中的键名
const KEY_LOCALE: &str = "locale";
/// 开机自启在配置文件中的键名
const KEY_AUTO_START: &str = "auto_start";
/// 记住窗口状态在配置文件中的键名
const KEY_REMEMBER_WINDOW: &str = "remember_window";
/// 自动检查更新在配置文件中的键名
const KEY_AUTO_CHECK_UPDATE: &str = "auto_check_update";
/// 系统托盘开关在配置文件中的键名
const KEY_TRAY_ENABLED: &str = "tray_enabled";
/// 关闭窗口行为在配置文件中的键名
const KEY_CLOSE_BEHAVIOR: &str = "close_behavior";
/// 快捷键类配置在配置文件中的键前缀：后续新增快捷键项一律用 `shortcut_` 开头
/// （如本项的 `shortcut_show`），便于归类与排查。
/// 显示应用的全局快捷键在配置文件中的键名
const KEY_SHORTCUT_SHOW: &str = "shortcut_show";
/// 当前配置结构版本：变更字段语义时递增，并在迁移链中补对应升级步骤
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 关闭窗口行为：落盘值为 `snake_case` 字符串，未知一律回落弹窗提示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    #[default]
    Prompt,
    Exit,
    MinimizeToTray,
}

impl CloseBehavior {
    /// 取对应的落盘标签，与 `serde` 值一致，前后端契约共用
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Exit => "exit",
            Self::MinimizeToTray => "minimize_to_tray",
        }
    }

    /// 容错解析关闭行为标签：未知或空标签一律回落弹窗提示
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "exit" => Self::Exit,
            "minimize_to_tray" => Self::MinimizeToTray,
            _ => Self::default(),
        }
    }
}

/// 应用完整配置：各持久化项聚合于此，`store` 内仍按扁平键（`KEY_*`）逐项存储。
/// 新增字段必须能 `Default`（否则旧文件缺键会解析失败）；写路径统一走 `apply_patch` 逐键写入，
/// 禁止用陈旧的 `Config` 整体覆盖。
///
/// 新增一个配置项的固定步骤：
/// `KEY_*` 常量 → 本结构体加字段 → `ConfigPatch` 加同名 `Option` 字段 → `load_config` 回填
/// → 需要运行时副作用则加进 `apply_runtime_effects` → `reset_patch` 补默认值
/// → 需跨键规整（如托盘关闭时禁最小化）则加进 `coerce_patch` → `cargo test` 重生成绑定。
///
/// 只有**改动已有字段的语义或位置**时才需要递增 `CURRENT_SCHEMA_VERSION` 并在 `MIGRATIONS`
/// 末尾补一级迁移，否则老用户配置会静默失效（纯新增字段由 `Default` + 容错读取兜住）。
/// 布尔开关天然扁平并存（与 `store` 扁平键一一对应），不为 lint 拆结构。
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Config {
    /// 配置结构版本：旧文件缺键时回落 `0`，任何写入都会带上当前版本号
    #[serde(deserialize_with = "de_schema_version")]
    #[specta(type = u32)]
    pub schema_version: u32,
    /// 界面语言：缺失、类型不符或无法识别时回落默认值，绝不让整包解析失败
    #[serde(deserialize_with = "de_locale")]
    #[specta(type = Locale)]
    pub locale: Locale,
    /// 开机自启：缺失或类型不符时回落 `false`，绝不让整包解析失败
    #[serde(deserialize_with = "de_auto_start")]
    #[specta(type = bool)]
    pub auto_start: bool,
    /// 记住窗口状态：缺失或类型不符时回落 `false`，绝不让整包解析失败
    #[serde(deserialize_with = "de_remember_window")]
    #[specta(type = bool)]
    pub remember_window: bool,
    /// 自动检查更新：缺失或类型不符时回落 `false`，绝不让整包解析失败
    #[serde(deserialize_with = "de_auto_check_update")]
    #[specta(type = bool)]
    pub auto_check_update: bool,
    /// 系统托盘开关：缺失或类型不符时回落默认值（`true`），绝不让整包解析失败
    #[serde(default = "default_tray_enabled", deserialize_with = "de_tray_enabled")]
    #[specta(type = bool)]
    pub tray_enabled: bool,
    /// 关闭窗口行为：缺失、类型不符或无法识别时回落弹窗提示，绝不让整包解析失败
    #[serde(deserialize_with = "de_close_behavior")]
    #[specta(type = CloseBehavior)]
    pub close_behavior: CloseBehavior,
    /// 显示应用的全局快捷键：`Ctrl+Shift+S` 风格，空字符串表示未设置；
    /// 缺失、类型不符或纯空白时回落空字符串，绝不让整包解析失败
    #[serde(deserialize_with = "de_shortcut_show")]
    #[specta(type = String)]
    pub shortcut_show: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 0,
            locale: Locale::default(),
            auto_start: false,
            remember_window: false,
            auto_check_update: false,
            tray_enabled: default_tray_enabled(),
            close_behavior: CloseBehavior::default(),
            shortcut_show: String::new(),
        }
    }
}

/// 系统托盘开关的默认值：开箱即见托盘
const fn default_tray_enabled() -> bool {
    true
}

/// 局部更新补丁：字段缺省或为 `null` 均表示"不改该键"，非 `null` 表示写入该值
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct ConfigPatch {
    /// 界面语言
    pub locale: Option<Locale>,
    /// 开机自启
    pub auto_start: Option<bool>,
    /// 记住窗口状态
    pub remember_window: Option<bool>,
    /// 自动检查更新
    pub auto_check_update: Option<bool>,
    /// 系统托盘开关
    pub tray_enabled: Option<bool>,
    /// 关闭窗口行为
    pub close_behavior: Option<CloseBehavior>,
    /// 显示应用的全局快捷键：`None`（或缺省 / `null`）表示不改该键
    pub shortcut_show: Option<String>,
}

/// 容错解析界面语言：经 `serde_json::Value` 中转，非字符串或未知标签一律回落默认值
fn de_locale<'de, D>(deserializer: D) -> Result<Locale, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_str().map(Locale::parse).unwrap_or_default())
}

/// 容错解析开机自启：经 `serde_json::Value` 中转，非布尔值一律回落默认值（`false`）
fn de_auto_start<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_bool().unwrap_or_default())
}

/// 容错解析记住窗口状态：经 `serde_json::Value` 中转，非布尔值一律回落默认值（`false`）
fn de_remember_window<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_bool().unwrap_or_default())
}

/// 容错解析自动检查更新：经 `serde_json::Value` 中转，非布尔值一律回落默认值（`false`）
fn de_auto_check_update<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_bool().unwrap_or_default())
}

/// 容错解析系统托盘开关：经 `serde_json::Value` 中转，非布尔值一律回落默认值（`true`）
fn de_tray_enabled<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_bool().unwrap_or(default_tray_enabled()))
}

/// 容错解析关闭窗口行为：经 `serde_json::Value` 中转，非字符串或未知标签一律回落默认值
fn de_close_behavior<'de, D>(deserializer: D) -> Result<CloseBehavior, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_str().map(CloseBehavior::parse).unwrap_or_default())
}

/// 容错解析显示应用的全局快捷键：经 `serde_json::Value` 中转，非字符串一律回落空字符串；
/// 首尾空白直接收窄掉，避免手改配置多出的空格导致无谓的注销重注册
fn de_shortcut_show<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value.as_str().map(str::trim).unwrap_or_default().to_owned())
}

/// 容错解析结构版本：经 `parse_schema_version` 统一处理脏值
fn de_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(parse_schema_version(Some(&value)))
}

/// 版本键原始值到 `u32` 的唯一转换口径：缺失、非无符号整数、超 `u32` 范围一律回落 `0`
fn parse_schema_version(value: Option<&Value>) -> u32 {
    value
        .and_then(Value::as_u64)
        .and_then(|raw| u32::try_from(raw).ok())
        .unwrap_or_default()
}

/// 启动装配：先把磁盘配置迁移到当前结构版本，再确定运行时语言、开机自启与窗口状态
pub fn setup(app: &tauri::App) {
    let handle = app.handle();
    migrate(handle);
    let locale = load_locale(handle).unwrap_or_else(|| {
        let detected = detect_locale();
        if let Err(err) = save_locale(handle, detected) {
            log::warn!("failed to persist detected locale: {err:#}");
        }
        detected
    });
    // 启动无 `before` 可比，直接经统一的语言应用入口生效，避免与更新路径分叉
    // `store` 打不开时 `load_locale` 回落 `None`，此处用探测值仅设运行时、不覆盖用户存量
    // （`save_locale` 失败只记日志，待存储恢复后下次启动再落盘）
    apply_locale(locale);
    // 存储不可用时跳过自启 enforcement：此时 `load_config` 全回落默认值，
    // 若按 `false` 执行会误关用户自启；只记日志，待恢复后下次启动按盘纠正
    if app.handle().store(STORE_FILE).is_err() {
        log::warn!("skipping autostart enforcement and window restore: {STORE_FILE} unavailable");
        // 窗口初始不可见，任何提前返回前必须显示，否则应用永久黑屏
        show_windows(handle);
        return;
    }
    // 开机自启以后端配置为权威，外部删改后重启自动修复；失败只记日志，不阻断启动
    if let Err(err) = sync_autostart(handle, load_config(handle).auto_start) {
        log::warn!("failed to enforce autostart state: {err:#}");
    }
    // 记住窗口开启时恢复上次几何；关闭时插件已跳过自动恢复，窗口按默认配置打开
    // 窗口初始不可见（`tauri.conf.json` 的 `visible: false` 防恢复闪烁），此处统一显示
    if load_config(handle).remember_window {
        restore_windows(handle);
    }
    show_windows(handle);
}

/// 仅显示主窗口；其余窗口由各自流程管理，此处不碰（恢复也只针对 `main`，见 `restore_windows`）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn show_windows(app: &tauri::AppHandle) {
    use tauri::Manager;

    for (label, window) in app.webview_windows() {
        if label != "main" {
            continue;
        }
        if let Err(err) = window.show() {
            log::warn!("failed to show window {label}: {err:#}");
        }
    }
}

/// 仅显示主窗口；恢复流程依赖此调用，失败只记日志（窗口已存在，仅可见性受影响）
#[cfg(any(target_os = "android", target_os = "ios"))]
fn show_windows(_app: &tauri::AppHandle) {}

/// 仅恢复主窗口状态；插件已跳过 `main` 的启动自动恢复，其余窗口无视用户开关照常由插件恢复，
/// 此处不碰它们。关闭记住窗口开关不删除已存快照（插件无公开清理接口），重开后恢复陈旧几何
/// 为已知残留，接受此行为。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn restore_windows(app: &tauri::AppHandle) {
    use tauri::Manager;
    use tauri_plugin_window_state::{StateFlags, WindowExt};

    for (label, window) in app.webview_windows() {
        if label != "main" {
            continue;
        }
        if let Err(err) = window.restore_state(StateFlags::all()) {
            log::warn!("failed to restore window {label}: {err:#}");
        }
    }
}

/// 仅恢复主窗口状态；插件已跳过 `main` 的启动自动恢复，其余窗口无视用户开关照常由插件恢复，
/// 此处不碰它们。关闭记住窗口开关不删除已存快照（插件无公开清理接口），重开后恢复陈旧几何
/// 为已知残留，接受此行为。
#[cfg(any(target_os = "android", target_os = "ios"))]
fn restore_windows(_app: &tauri::AppHandle) {}

/// 读取完整应用配置；各字段缺失或无法识别时逐项回落默认值（无落盘副作用）
pub fn load_config(app: &tauri::AppHandle) -> Config {
    Config {
        locale: load_locale(app).unwrap_or_default(),
        auto_start: load_auto_start(app),
        remember_window: load_remember_window(app),
        auto_check_update: load_auto_check_update(app),
        tray_enabled: load_tray_enabled(app),
        close_behavior: load_close_behavior(app),
        shortcut_show: load_shortcut_show(app),
        schema_version: load_schema_version(app),
    }
}

/// 读取持久化的界面语言；未设置、类型不符或无法识别时返回 `None`
fn load_locale(app: &tauri::AppHandle) -> Option<Locale> {
    read_key(app, KEY_LOCALE)?.as_str().map(Locale::parse)
}

/// 读取持久化的开机自启；未设置或类型不符时回落默认值（`false`）
fn load_auto_start(app: &tauri::AppHandle) -> bool {
    read_key(app, KEY_AUTO_START)
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
}

/// 读取持久化的记住窗口状态；未设置或类型不符时回落默认值（`false`）
fn load_remember_window(app: &tauri::AppHandle) -> bool {
    read_key(app, KEY_REMEMBER_WINDOW)
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
}

/// 读取持久化的自动检查更新；未设置或类型不符时回落默认值（`false`）
fn load_auto_check_update(app: &tauri::AppHandle) -> bool {
    read_key(app, KEY_AUTO_CHECK_UPDATE)
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
}

/// 读取持久化的系统托盘开关；未设置或类型不符时回落默认值（`true`）
fn load_tray_enabled(app: &tauri::AppHandle) -> bool {
    read_key(app, KEY_TRAY_ENABLED)
        .and_then(|value| value.as_bool())
        .unwrap_or(default_tray_enabled())
}

/// 读取持久化的关闭窗口行为；未设置、类型不符或无法识别时回落弹窗提示
fn load_close_behavior(app: &tauri::AppHandle) -> CloseBehavior {
    read_key(app, KEY_CLOSE_BEHAVIOR)
        .and_then(|value| value.as_str().map(CloseBehavior::parse))
        .unwrap_or_default()
}

/// 读取持久化的显示应用全局快捷键；未设置、类型不符或纯空白时回落空字符串
fn load_shortcut_show(app: &tauri::AppHandle) -> String {
    read_key(app, KEY_SHORTCUT_SHOW)
        .and_then(|value| value.as_str().map(str::trim).map(str::to_owned))
        .unwrap_or_default()
}

/// 读取持久化的配置结构版本；未设置或类型不符时回落 `0`
fn load_schema_version(app: &tauri::AppHandle) -> u32 {
    parse_schema_version(read_key(app, KEY_SCHEMA_VERSION).as_ref())
}

/// 配置写互斥：`update` 的读-改-写非原子，`tauri::command` 可并发执行，
///
/// 无锁时两路更新交错即后写覆盖先写（`lost update`），此处以进程内锁串行化。
/// 加锁点收敛在 `update` / `save_locale` / `migrate` 三个入口，内部函数
/// （`apply_patch` / `migrate_inner` / `persist_diff`）一律不加锁，
/// 避免重复加锁自死锁——新增写入路径必须走这三个入口之一。
fn config_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// 取写锁；锁毒化时记日志后继续（`store.save()` 是原子写盘，半写风险低）
fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    config_write_lock().lock().unwrap_or_else(|poison| {
        log::error!("config write lock poisoned, continuing with recovered guard");
        poison.into_inner()
    })
}

/// 应用局部更新并执行副作用，返回写后的最新配置。
/// 返回写后值是刻意的：前端据此回写内存态，无需再读一次，也就不会持陈旧值。
/// 开机自启先同步操作系统再落盘：落盘失败时回滚操作系统状态，整条命令失败，内存不动。
pub fn update(app: &tauri::AppHandle, patch: &ConfigPatch) -> anyhow::Result<Config> {
    // 串行化并发更新；`store` 打不开时直接失败，避免基于陈旧 `before` 做自启决策
    let _guard = lock_config();
    if app.store(STORE_FILE).is_err() {
        anyhow::bail!("failed to open {STORE_FILE}: store unavailable");
    }
    // 幂等迁移：保证任何写入前磁盘已升到当前版本，不依赖调用方先调 `migrate`
    migrate_inner(app);
    let before = load_config(app);
    // 跨键规整先于一切副作用与落盘：规整后的补丁即生效值，写后值直接返回给前端
    let patch = coerce_patch(&before, patch);
    if let Some(target) = wants_autostart_sync(&patch, before.auto_start) {
        sync_autostart(app, target)?;
    }
    if let Err(err) = apply_patch(app, &patch) {
        // 落盘失败时把已翻转的操作系统自启状态回滚，避免 OS≠盘≠内存
        if wants_autostart_sync(&patch, before.auto_start).is_some()
            && let Err(rollback_err) = sync_autostart(app, before.auto_start)
        {
            log::error!("failed to roll back autostart state: {rollback_err:#}");
        }
        return Err(err).context("failed to persist config patch");
    }
    let after = load_config(app);
    if needs_shortcut_sync(&before, &after) {
        sync_shortcut_or_rollback(app, &before, &after)?;
    }
    apply_runtime_effects(app, &before, &after);
    Ok(after)
}

/// 跨键规整补丁：托盘关闭时“最小化到托盘”无处可去，一律回落弹窗提示并写盘，
/// 前端靠写后返回值自动回滚 UI，无需另写补偿逻辑。纯函数，便于单测回滚分支。
fn coerce_patch(before: &Config, patch: &ConfigPatch) -> ConfigPatch {
    let mut coerced = patch.clone();
    let tray_enabled = patch.tray_enabled.unwrap_or(before.tray_enabled);
    let behavior = patch.close_behavior.unwrap_or(before.close_behavior);
    if !tray_enabled && behavior == CloseBehavior::MinimizeToTray {
        coerced.close_behavior = Some(CloseBehavior::Prompt);
    }
    coerced
}

/// 本次补丁是否要求变更操作系统自启状态；纯函数，便于单测决策分支
fn wants_autostart_sync(patch: &ConfigPatch, current: bool) -> Option<bool> {
    patch.auto_start.filter(|target| *target != current)
}

/// 显示应用快捷键是否发生变化（落盘值口径，已在读取侧收窄空白）；纯函数，便于单测决策分支
fn needs_shortcut_sync(before: &Config, after: &Config) -> bool {
    before.shortcut_show != after.shortcut_show
}

/// 同步全局快捷键到新值：先注销旧值再注册新值；注册失败时把落盘值回滚到旧值
/// 并尽力恢复旧注册，整条命令失败——前端据此保持旧内存态，UI 自动回滚。
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn sync_shortcut_or_rollback(
    app: &tauri::AppHandle,
    before: &Config,
    after: &Config,
) -> anyhow::Result<()> {
    if let Err(err) = super::shortcut::sync(app, &before.shortcut_show, &after.shortcut_show) {
        log::error!("failed to register global shortcut: {err:#}, rolling back persisted value");
        if let Err(rollback_err) = apply_patch(
            app,
            &ConfigPatch {
                shortcut_show: Some(before.shortcut_show.clone()),
                ..ConfigPatch::default()
            },
        ) {
            log::error!("failed to roll back shortcut config: {rollback_err:#}");
        }
        // 尽力恢复旧注册：失败只记日志，下次更新按配置值重试即可恢复
        if let Err(restore_err) =
            super::shortcut::sync(app, &after.shortcut_show, &before.shortcut_show)
        {
            log::error!("failed to restore previous shortcut: {restore_err:#}");
        }
        return Err(err).context("failed to register global shortcut");
    }
    Ok(())
}

/// 移动端无全局快捷键能力：直接视为成功（本应用专注桌面端，此处仅保签一致）
#[cfg(any(target_os = "android", target_os = "ios"))]
fn sync_shortcut_or_rollback(
    _app: &tauri::AppHandle,
    _before: &Config,
    _after: &Config,
) -> anyhow::Result<()> {
    Ok(())
}

/// 把开机自启状态同步到操作系统；移动端无此依赖，直接视为成功
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn sync_autostart(app: &tauri::AppHandle, enabled: bool) -> anyhow::Result<()> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|err| anyhow::anyhow!("failed to sync autostart to {enabled}: {err:#}"))?;
    Ok(())
}

/// 把开机自启状态同步到操作系统；移动端无此依赖，直接视为成功
#[cfg(any(target_os = "android", target_os = "ios"))]
fn sync_autostart(_app: &tauri::AppHandle, _enabled: bool) -> anyhow::Result<()> {
    Ok(())
}

/// 重置全部已知配置项为默认值；未知键保留（容错解析本就忽略它们，不做清理）
pub fn reset(app: &tauri::AppHandle) -> anyhow::Result<Config> {
    update(app, &reset_patch())
}

/// 重置所用的补丁：必须覆盖全部已知键，新增配置项时同步
/// （`reset_covers_every_known_key` 测试会拦截遗漏）
fn reset_patch() -> ConfigPatch {
    ConfigPatch {
        locale: Some(Locale::default()),
        auto_start: Some(false),
        remember_window: Some(false),
        auto_check_update: Some(false),
        tray_enabled: Some(default_tray_enabled()),
        close_behavior: Some(CloseBehavior::default()),
        shortcut_show: Some(String::new()),
    }
}

/// 写入界面语言并立即落盘（启动探测落库路径，同样持写锁防与并发更新交错）
fn save_locale(app: &tauri::AppHandle, locale: Locale) -> anyhow::Result<()> {
    let _guard = lock_config();
    apply_patch(
        app,
        &ConfigPatch {
            locale: Some(locale),
            ..ConfigPatch::default()
        },
    )
}

/// 写后副作用：仅对发生变化的字段生效，避免无谓的全局状态切换
fn apply_runtime_effects(app: &tauri::AppHandle, before: &Config, after: &Config) {
    if needs_locale_apply(before, after) {
        apply_locale(after.locale);
        // 托盘菜单文案跟随界面语言，运行时重建（托盘不存在时内部跳过）
        crate::cores::tray::refresh_texts(app);
    }
    if before.tray_enabled != after.tray_enabled {
        crate::cores::tray::set_visible(app, after.tray_enabled);
    }
}

/// 运行时语言的唯一应用入口：启动与更新两条路径共用，避免直接散调 `set_locale`
fn apply_locale(locale: Locale) {
    rust_i18n::set_locale(locale.as_str());
}

/// 语言是否需要重新应用到运行时
fn needs_locale_apply(before: &Config, after: &Config) -> bool {
    before.locale != after.locale
}

/// 逐键写入补丁：只写提交的键 + 版本号，不做整包覆盖。
/// 调用方必须已持 `lock_config` 守卫（当前为 `update` / `save_locale`），本函数不重复加锁。
fn apply_patch(app: &tauri::AppHandle, patch: &ConfigPatch) -> anyhow::Result<()> {
    let store = app.store(STORE_FILE)?;
    for (key, value) in write_entries(patch) {
        store.set(key, value);
    }
    store.save()?;
    Ok(())
}

/// 补丁对应的待写键值对：**唯一的写入清单**，`apply_patch` 与测试共用
fn write_entries(patch: &ConfigPatch) -> Vec<(&'static str, Value)> {
    let mut entries = Vec::new();
    if let Some(locale) = patch.locale {
        entries.push((KEY_LOCALE, Value::from(locale.as_str())));
    }
    if let Some(auto_start) = patch.auto_start {
        entries.push((KEY_AUTO_START, Value::from(auto_start)));
    }
    if let Some(remember_window) = patch.remember_window {
        entries.push((KEY_REMEMBER_WINDOW, Value::from(remember_window)));
    }
    if let Some(auto_check_update) = patch.auto_check_update {
        entries.push((KEY_AUTO_CHECK_UPDATE, Value::from(auto_check_update)));
    }
    if let Some(tray_enabled) = patch.tray_enabled {
        entries.push((KEY_TRAY_ENABLED, Value::from(tray_enabled)));
    }
    if let Some(close_behavior) = patch.close_behavior {
        entries.push((KEY_CLOSE_BEHAVIOR, Value::from(close_behavior.as_str())));
    }
    if let Some(shortcut_show) = &patch.shortcut_show {
        entries.push((KEY_SHORTCUT_SHOW, Value::from(shortcut_show.trim())));
    }
    entries.push((KEY_SCHEMA_VERSION, Value::from(CURRENT_SCHEMA_VERSION)));
    entries
}

/// 读取某个键的原始值；打开配置文件失败时记日志并返回 `None`（读路径一律不阻断）。
///
/// 注意：`update` / `setup` 的决策路径不依赖此回落——`update` 在存储不可用时直接失败，
/// `setup` 在存储不可用时跳过自启 enforcement，避免基于全默认快照误决策。
fn read_key(app: &tauri::AppHandle, key: &str) -> Option<Value> {
    let store = match app.store(STORE_FILE) {
        Ok(store) => store,
        Err(err) => {
            log::warn!("failed to open {STORE_FILE}: {err:#}");
            return None;
        }
    };
    store.get(key)
}

/// 配置迁移步骤：把快照从 `MIGRATIONS` 的索引版本升到索引 + 1，就地修改。
/// 纯函数、不依赖 Tauri 运行时，可脱离 store 单测；未识别键原样保留。
type Migration = fn(&mut Map<String, Value>);

/// 迁移链：索引即源版本号，长度必须等于 `CURRENT_SCHEMA_VERSION`
const MIGRATIONS: &[Migration] = &[migrate_v0_to_v1];

/// v0 → v1：引入结构版本键，并把早期写坏的 `locale` 就地规整为合法标签，
/// 使磁盘值与运行时读取语义一致（读取侧本就经 `Locale::parse` 容错）
fn migrate_v0_to_v1(map: &mut Map<String, Value>) {
    if let Some(raw) = map.get(KEY_LOCALE).and_then(Value::as_str) {
        map.insert(
            KEY_LOCALE.to_owned(),
            Value::from(Locale::parse(raw).as_str()),
        );
    }
}

/// 读取快照里的结构版本；缺失或脏值一律视为 `0`
fn schema_version_of(map: &Map<String, Value>) -> u32 {
    parse_schema_version(map.get(KEY_SCHEMA_VERSION))
}

/// 把快照升到当前版本；返回是否需要写回磁盘
fn migrate_snapshot(map: &mut Map<String, Value>) -> bool {
    if map.is_empty() {
        // 全新安装没有存量可迁移；首次真实写入自会带上版本号
        return false;
    }

    let from = schema_version_of(map);
    log::debug!("config schema v{from}, supported v{CURRENT_SCHEMA_VERSION}");
    if from > CURRENT_SCHEMA_VERSION {
        log::warn!(
            "config schema v{from} is newer than supported v{CURRENT_SCHEMA_VERSION}, leaving it untouched"
        );
        return false;
    }
    if from == CURRENT_SCHEMA_VERSION {
        return false;
    }

    for migration in MIGRATIONS.iter().skip(from as usize) {
        migration(map);
    }
    map.insert(
        KEY_SCHEMA_VERSION.to_owned(),
        Value::from(CURRENT_SCHEMA_VERSION),
    );
    true
}

/// 启动迁移：把磁盘上的配置升到当前结构版本（加锁入口）。
pub fn migrate(app: &tauri::AppHandle) {
    let _guard = lock_config();
    migrate_inner(app);
}

/// 启动迁移（无锁内部实现，调用方必须已持 `lock_config` 守卫）。
///
/// 首次落库会直接写上当前版本号，迁移必须跑在任何写入之前，否则旧盘再也升不上来。
/// `update` 内已幂等调用，此处供 `setup` 首行调用。
fn migrate_inner(app: &tauri::AppHandle) {
    let store = match app.store(STORE_FILE) {
        Ok(store) => store,
        Err(err) => {
            log::warn!("failed to open {STORE_FILE}: {err:#}");
            return;
        }
    };

    let before: Map<String, Value> = store.entries().into_iter().collect();
    let mut after = before.clone();
    if !migrate_snapshot(&mut after) {
        return;
    }

    log::info!("migrating {STORE_FILE} to schema v{CURRENT_SCHEMA_VERSION}");
    if let Err(err) = persist_diff(&store, &before, &after) {
        log::warn!("failed to persist migrated config: {err:#}");
    }
}

/// 只写回迁移改动过的键、删除被移除的键，未触碰的未知键保持原样
fn persist_diff<R: Runtime>(
    store: &Store<R>,
    before: &Map<String, Value>,
    after: &Map<String, Value>,
) -> anyhow::Result<()> {
    let mut changed = false;
    for (key, value) in after {
        if before.get(key) != Some(value) {
            store.set(key.clone(), value.clone());
            changed = true;
        }
    }
    for key in before.keys() {
        if !after.contains_key(key) {
            store.delete(key);
            changed = true;
        }
    }
    if changed {
        store.save()?;
    }
    Ok(())
}

/// 探测操作系统语言（BCP-47），失败或无法识别时回落默认语言
fn detect_locale() -> Locale {
    tauri_plugin_os::locale().map_or_else(Locale::default, |raw| Locale::parse(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 取键值对清单里的键，便于断言写入覆盖面
    fn keys_of(entries: &[(&'static str, Value)]) -> Vec<&'static str> {
        entries.iter().map(|(key, _)| *key).collect()
    }

    #[test]
    fn defaults_to_english() {
        let config = Config::default();
        assert_eq!(config.locale, Locale::En);
        assert!(!config.auto_start);
        assert!(!config.remember_window);
        assert!(!config.auto_check_update);
        assert_eq!(config.shortcut_show, "");
        assert_eq!(config.schema_version, 0);
    }

    #[test]
    fn round_trips_through_json() {
        let config = Config {
            locale: Locale::ZhCn,
            auto_start: true,
            remember_window: true,
            auto_check_update: true,
            tray_enabled: false,
            close_behavior: CloseBehavior::MinimizeToTray,
            shortcut_show: "Ctrl+Shift+S".to_owned(),
            schema_version: CURRENT_SCHEMA_VERSION,
        };
        let raw = serde_json::to_value(&config).expect("config serializes");
        assert_eq!(
            raw,
            json!({ "locale": "zh-CN", "auto_start": true, "remember_window": true, "auto_check_update": true, "tray_enabled": false, "close_behavior": "minimize_to_tray", "shortcut_show": "Ctrl+Shift+S", "schema_version": 1 })
        );
        let back: Config = serde_json::from_value(raw).expect("config deserializes");
        assert_eq!(back, config);
    }

    #[test]
    fn tolerates_missing_or_unrecognized_locale() {
        for raw in [
            json!({}),
            json!({ "locale": "fr" }),
            json!({ "locale": 5 }),
            json!({ "locale": null }),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.locale, Locale::En);
        }
    }

    #[test]
    fn tolerates_missing_or_invalid_auto_start() {
        for (raw, expected) in [
            (json!({}), false),
            (json!({ "auto_start": true }), true),
            (json!({ "auto_start": "yes" }), false),
            (json!({ "auto_start": 1 }), false),
            (json!({ "auto_start": null }), false),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.auto_start, expected);
        }
    }

    #[test]
    fn tolerates_missing_or_invalid_remember_window() {
        for (raw, expected) in [
            (json!({}), false),
            (json!({ "remember_window": true }), true),
            (json!({ "remember_window": "yes" }), false),
            (json!({ "remember_window": 1 }), false),
            (json!({ "remember_window": null }), false),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.remember_window, expected);
        }
    }

    #[test]
    fn tolerates_missing_or_invalid_auto_check_update() {
        for (raw, expected) in [
            (json!({}), false),
            (json!({ "auto_check_update": true }), true),
            (json!({ "auto_check_update": "yes" }), false),
            (json!({ "auto_check_update": 1 }), false),
            (json!({ "auto_check_update": null }), false),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.auto_check_update, expected);
        }
    }

    #[test]
    fn tray_enabled_defaults_to_on_and_tolerates_dirt() {
        for (raw, expected) in [
            (json!({}), true),
            (json!({ "tray_enabled": true }), true),
            (json!({ "tray_enabled": false }), false),
            (json!({ "tray_enabled": "yes" }), true),
            (json!({ "tray_enabled": 1 }), true),
            (json!({ "tray_enabled": null }), true),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.tray_enabled, expected);
        }
    }

    #[test]
    fn close_behavior_falls_back_to_prompt() {
        for (raw, expected) in [
            (json!({}), CloseBehavior::Prompt),
            (json!({ "close_behavior": "prompt" }), CloseBehavior::Prompt),
            (json!({ "close_behavior": "exit" }), CloseBehavior::Exit),
            (
                json!({ "close_behavior": "minimize_to_tray" }),
                CloseBehavior::MinimizeToTray,
            ),
            (json!({ "close_behavior": "quit" }), CloseBehavior::Prompt),
            (json!({ "close_behavior": 5 }), CloseBehavior::Prompt),
            (json!({ "close_behavior": null }), CloseBehavior::Prompt),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.close_behavior, expected);
        }
    }

    #[test]
    fn close_behavior_round_trips_through_as_str() {
        for behavior in [
            CloseBehavior::Prompt,
            CloseBehavior::Exit,
            CloseBehavior::MinimizeToTray,
        ] {
            assert_eq!(CloseBehavior::parse(behavior.as_str()), behavior);
        }
    }

    #[test]
    fn tolerates_missing_or_unrecognized_schema_version() {
        for (raw, expected) in [
            (json!({}), 0_u32),
            (json!({ "schema_version": 1 }), 1),
            (json!({ "schema_version": "x" }), 0),
            (json!({ "schema_version": -1 }), 0),
            (json!({ "schema_version": null }), 0),
            (json!({ "schema_version": 5_000_000_000_u64 }), 0),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.schema_version, expected);
        }
    }

    #[test]
    fn tolerates_missing_or_invalid_shortcut_show() {
        for (raw, expected) in [
            (json!({}), ""),
            (json!({ "shortcut_show": "Ctrl+Shift+S" }), "Ctrl+Shift+S"),
            (json!({ "shortcut_show": "  Alt+Space  " }), "Alt+Space"),
            (json!({ "shortcut_show": "   " }), ""),
            (json!({ "shortcut_show": 5 }), ""),
            (json!({ "shortcut_show": null }), ""),
        ] {
            let config: Config = serde_json::from_value(raw).expect("never fails");
            assert_eq!(config.shortcut_show, expected);
        }
    }

    #[test]
    fn ignores_unknown_keys() {
        let config: Config = serde_json::from_value(json!({ "locale": "en", "theme": "dark" }))
            .expect("unknown keys ignored");
        assert_eq!(config.locale, Locale::En);
    }

    #[test]
    fn patch_treats_absent_and_null_as_no_change() {
        let absent: ConfigPatch = serde_json::from_value(json!({})).expect("patch deserializes");
        assert_eq!(absent.locale, None);
        assert_eq!(absent.auto_start, None);
        assert_eq!(absent.remember_window, None);
        assert_eq!(absent.auto_check_update, None);

        let cleared: ConfigPatch =
            serde_json::from_value(json!({ "locale": null })).expect("null tolerated");
        assert_eq!(cleared.locale, None);

        let set: ConfigPatch =
            serde_json::from_value(json!({ "locale": "zh-CN" })).expect("locale parsed");
        assert_eq!(set.locale, Some(Locale::ZhCn));

        let toggled: ConfigPatch =
            serde_json::from_value(json!({ "auto_start": true })).expect("flag parsed");
        assert_eq!(toggled.auto_start, Some(true));

        let remembered: ConfigPatch =
            serde_json::from_value(json!({ "remember_window": true })).expect("flag parsed");
        assert_eq!(remembered.remember_window, Some(true));

        let auto_checked: ConfigPatch =
            serde_json::from_value(json!({ "auto_check_update": true })).expect("flag parsed");
        assert_eq!(auto_checked.auto_check_update, Some(true));

        let trayed: ConfigPatch =
            serde_json::from_value(json!({ "tray_enabled": false })).expect("flag parsed");
        assert_eq!(trayed.tray_enabled, Some(false));

        let closed: ConfigPatch =
            serde_json::from_value(json!({ "close_behavior": "exit" })).expect("behavior parsed");
        assert_eq!(closed.close_behavior, Some(CloseBehavior::Exit));

        let shown: ConfigPatch = serde_json::from_value(json!({ "shortcut_show": "Ctrl+Shift+S" }))
            .expect("shortcut parsed");
        assert_eq!(shown.shortcut_show, Some("Ctrl+Shift+S".to_owned()));
    }

    #[test]
    fn patch_writes_only_submitted_keys() {
        let untouched = write_entries(&ConfigPatch::default());
        assert_eq!(
            keys_of(&untouched),
            vec![KEY_SCHEMA_VERSION],
            "未提交的键不得写入"
        );

        let patched = write_entries(&ConfigPatch {
            locale: Some(Locale::ZhCn),
            auto_start: Some(true),
            remember_window: Some(true),
            auto_check_update: Some(true),
            tray_enabled: Some(false),
            close_behavior: Some(CloseBehavior::Exit),
            shortcut_show: Some("Ctrl+Shift+S".to_owned()),
        });
        assert_eq!(
            keys_of(&patched),
            vec![
                KEY_LOCALE,
                KEY_AUTO_START,
                KEY_REMEMBER_WINDOW,
                KEY_AUTO_CHECK_UPDATE,
                KEY_TRAY_ENABLED,
                KEY_CLOSE_BEHAVIOR,
                KEY_SHORTCUT_SHOW,
                KEY_SCHEMA_VERSION
            ]
        );
        assert_eq!(patched[0].1, json!("zh-CN"));
        assert_eq!(patched[1].1, json!(true));
        assert_eq!(patched[2].1, json!(true));
        assert_eq!(patched[3].1, json!(true));
        assert_eq!(patched[4].1, json!(false));
        assert_eq!(patched[5].1, json!("exit"));
        assert_eq!(patched[6].1, json!("Ctrl+Shift+S"));
    }

    #[test]
    fn reset_covers_every_known_key() {
        // 期望派生自全量补丁的写入清单：`write_entries` 加新键而 `reset_patch` 遗漏时必失败
        let full = ConfigPatch {
            locale: Some(Locale::En),
            auto_start: Some(true),
            remember_window: Some(true),
            auto_check_update: Some(true),
            tray_enabled: Some(false),
            close_behavior: Some(CloseBehavior::Exit),
            shortcut_show: Some("Ctrl+Shift+S".to_owned()),
        };
        let entries = write_entries(&reset_patch());
        assert_eq!(
            keys_of(&entries),
            keys_of(&write_entries(&full)),
            "新增配置项时必须同步 reset_patch"
        );
        assert_eq!(entries[0].1, json!(Locale::default().as_str()));
        assert_eq!(entries[1].1, json!(false));
        assert_eq!(entries[2].1, json!(false));
        assert_eq!(entries[3].1, json!(false));
        assert_eq!(entries[4].1, json!(true));
        assert_eq!(entries[5].1, json!(CloseBehavior::default().as_str()));
        assert_eq!(entries[6].1, json!(""));
    }

    #[test]
    fn locale_side_effect_triggers_only_on_change() {
        let base = Config {
            locale: Locale::En,
            auto_start: false,
            remember_window: false,
            auto_check_update: false,
            tray_enabled: true,
            close_behavior: CloseBehavior::Prompt,
            shortcut_show: String::new(),
            schema_version: CURRENT_SCHEMA_VERSION,
        };
        assert!(!needs_locale_apply(&base, &base));

        let switched = Config {
            locale: Locale::ZhCn,
            ..Config::default()
        };
        assert!(needs_locale_apply(&base, &switched));
    }

    #[test]
    fn autostart_sync_triggers_only_on_change() {
        let off = ConfigPatch {
            auto_start: Some(false),
            ..ConfigPatch::default()
        };
        assert_eq!(wants_autostart_sync(&off, false), None);

        let on = ConfigPatch {
            auto_start: Some(true),
            ..ConfigPatch::default()
        };
        assert_eq!(wants_autostart_sync(&on, false), Some(true));
        assert_eq!(wants_autostart_sync(&on, true), None);

        // 关闭路径：当前开启时提交关闭，必须触发一次同步
        let off_while_on = ConfigPatch {
            auto_start: Some(false),
            ..ConfigPatch::default()
        };
        assert_eq!(wants_autostart_sync(&off_while_on, true), Some(false));

        assert_eq!(wants_autostart_sync(&ConfigPatch::default(), false), None);
    }

    #[test]
    fn shortcut_sync_triggers_only_on_change() {
        let base = Config {
            shortcut_show: "Ctrl+Shift+S".to_owned(),
            ..Config::default()
        };
        assert!(!needs_shortcut_sync(&base, &base));

        let cleared = Config {
            shortcut_show: String::new(),
            ..Config::default()
        };
        assert!(needs_shortcut_sync(&base, &cleared));
        assert!(needs_shortcut_sync(&cleared, &base));
    }

    /// 关托盘时“最小化到托盘”无处可去，补丁与存量任一侧触发都要回落弹窗提示
    fn minimized(before: &Config) -> Config {
        Config {
            close_behavior: CloseBehavior::MinimizeToTray,
            ..before.clone()
        }
    }

    #[test]
    fn disabling_tray_rolls_back_minimize_to_prompt() {
        let before = minimized(&Config::default());
        let patch = ConfigPatch {
            tray_enabled: Some(false),
            ..ConfigPatch::default()
        };

        assert_eq!(
            coerce_patch(&before, &patch).close_behavior,
            Some(CloseBehavior::Prompt)
        );
    }

    #[test]
    fn inline_disable_with_minimize_rolls_back_to_prompt() {
        let patch = ConfigPatch {
            tray_enabled: Some(false),
            close_behavior: Some(CloseBehavior::MinimizeToTray),
            ..ConfigPatch::default()
        };

        assert_eq!(
            coerce_patch(&Config::default(), &patch).close_behavior,
            Some(CloseBehavior::Prompt)
        );
    }

    #[test]
    fn tray_enabled_keeps_minimize_untouched() {
        let before = minimized(&Config::default());
        let patch = ConfigPatch {
            tray_enabled: Some(true),
            ..ConfigPatch::default()
        };

        assert_eq!(coerce_patch(&before, &patch).close_behavior, None);
    }

    #[test]
    fn unrelated_patch_leaves_close_behavior_alone() {
        let before = Config::default();
        let patch = ConfigPatch {
            auto_start: Some(true),
            ..ConfigPatch::default()
        };

        assert_eq!(coerce_patch(&before, &patch), patch);
    }

    /// 把 JSON fixture 转成迁移链使用的快照
    fn snapshot(raw: &Value) -> Map<String, Value> {
        raw.as_object().cloned().expect("fixture is an object")
    }

    #[test]
    fn migration_chain_matches_current_version() {
        assert_eq!(
            MIGRATIONS.len(),
            CURRENT_SCHEMA_VERSION as usize,
            "每次递增 CURRENT_SCHEMA_VERSION 必须补一级迁移"
        );
    }

    #[test]
    fn migrates_legacy_snapshot_without_losing_unknown_keys() {
        let mut map = snapshot(&json!({ "locale": "zh_Hans_CN", "theme": "dark" }));

        assert!(migrate_snapshot(&mut map));

        assert_eq!(map.get(KEY_LOCALE).and_then(Value::as_str), Some("zh-CN"));
        assert_eq!(schema_version_of(&map), CURRENT_SCHEMA_VERSION);
        assert_eq!(
            map.get("theme").and_then(Value::as_str),
            Some("dark"),
            "未知键不得丢失"
        );
    }

    #[test]
    fn migrates_unrecognized_locale_to_default() {
        let mut map = snapshot(&json!({ "locale": "fr" }));

        assert!(migrate_snapshot(&mut map));

        assert_eq!(map.get(KEY_LOCALE).and_then(Value::as_str), Some("en"));
    }

    #[test]
    fn snapshot_at_current_version_needs_no_write() {
        let mut map =
            snapshot(&json!({ "locale": "en", "schema_version": CURRENT_SCHEMA_VERSION }));
        let original = map.clone();

        assert!(!migrate_snapshot(&mut map));
        assert_eq!(map, original);
    }

    #[test]
    fn snapshot_from_newer_version_is_left_untouched() {
        let mut map = snapshot(&json!({
            "locale": "en",
            "schema_version": CURRENT_SCHEMA_VERSION + 7
        }));
        let original = map.clone();

        assert!(!migrate_snapshot(&mut map), "降级运行不得回写配置");
        assert_eq!(map, original);
    }

    #[test]
    fn empty_snapshot_needs_no_write() {
        let mut map: Map<String, Value> = Map::new();

        assert!(!migrate_snapshot(&mut map), "全新安装不写入");
        assert_eq!(map, Map::new());
    }

    #[test]
    fn dirty_version_key_is_treated_as_legacy() {
        let mut map = snapshot(&json!({ "locale": "zh-CN", "schema_version": "x" }));

        assert!(migrate_snapshot(&mut map));

        assert_eq!(schema_version_of(&map), CURRENT_SCHEMA_VERSION);
    }
}
