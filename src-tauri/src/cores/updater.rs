//! 应用更新能力：检查、下载安装与重启。
//!
//! 更新检查与安装是跨业务的通用能力，故放在 `cores` 而非按业务归属的 `features`。
//! 命令层（`commands::updater`）只做薄封装；下载进度经 `APP_UPDATER_PROGRESS_EVENT`
//! 事件推送，前端在 `libs/hooks/updater.svelte.ts` 订阅（事件名两端以该常量为准）。
use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

/// 下载进度事件名：前后端以此字符串会合，改名需两端同步
/// （前端 `libs/hooks/updater.svelte.ts` 的 `PROGRESS_EVENT` 持有同一字面量，
/// `progress_event_name_is_stable` 测试锁定取值，改名时两处同改）
pub const APP_UPDATER_PROGRESS_EVENT: &str = "app-updater-progress";

/// 可用的更新信息：只含前端展示所需字段，插件的 `Update` 句柄不出命令边界
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UpdateInfo {
    /// 远端版本号
    pub version: String,
    /// 当前版本号
    pub current_version: String,
    /// 发布说明（`latest.json` 的 `body`，可能缺失）
    pub body: Option<String>,
}

/// 下载进度事件载荷：`total` 缺失时前端展示不确定进度
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ProgressPayload {
    /// 累计已下载字节数（插件回调给的是单包长度，此处已累加）
    pub downloaded: u64,
    /// 总字节数（服务端未提供时为 `None`）
    pub total: Option<u64>,
}

/// 下载进度发射节流器：插件回调逐包触发，此处合并为 200ms 一次或 ≥1% 进展一次；
/// 终态（成功/失败）由命令返回值保证，不依赖末包事件
struct ProgressThrottle {
    last_emit: Option<std::time::Instant>,
    last_percent: u64,
}

impl ProgressThrottle {
    const fn new() -> Self {
        Self {
            last_emit: None,
            last_percent: 0,
        }
    }

    /// 是否发射本次进度：首次必放行；之后需满 200ms 或前进 ≥1%（总数未知时只看时间窗）
    fn allow(&mut self, downloaded: u64, total: Option<u64>) -> bool {
        const WINDOW_MS: u128 = 200;
        let due = self
            .last_emit
            .is_none_or(|at| at.elapsed().as_millis() >= WINDOW_MS);
        let advanced = total
            .filter(|total| *total > 0)
            .is_some_and(|total| downloaded.saturating_mul(100) / total > self.last_percent);
        if !due && !advanced {
            return false;
        }
        self.last_emit = Some(std::time::Instant::now());
        if let Some(total) = total.filter(|total| *total > 0) {
            self.last_percent = downloaded.saturating_mul(100) / total;
        }
        true
    }
}

/// 检查更新；无新版返回 `None`（仅桌面端，移动端直接报错）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn check_for_update(app: &tauri::AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater()
        .context("updater backend unavailable")?
        .check()
        .await
        .context("failed to check for updates")?;
    Ok(update.map(|found| UpdateInfo {
        version: found.version,
        current_version: found.current_version,
        body: found.body,
    }))
}

/// 检查更新；无新版返回 `None`（仅桌面端，移动端直接报错）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn check_for_update(_app: &tauri::AppHandle) -> anyhow::Result<Option<UpdateInfo>> {
    Err(anyhow::anyhow!("updater is not supported on mobile"))
}

/// 下载并安装更新，进度经事件推送；装完不自动重启，由前端手动触发
/// （仅桌面端，移动端直接报错）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn download_and_install(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use tauri::Emitter;
    use tauri_plugin_updater::UpdaterExt;

    // 有意重查一次：前端 `check` 到下载之间可能已发新版，`Update` 句柄不可序列化，
    // 此处以最新一次 `check` 的句柄为准，避免装旧版
    let Some(update) = app
        .updater()
        .context("updater backend unavailable")?
        .check()
        .await
        .context("failed to check for updates before download")?
    else {
        return Err(anyhow::anyhow!("no update available"));
    };
    // 插件回调的 `downloaded` 是单包长度，需在此累加成累计值再发出
    let mut downloaded_total = 0_u64;
    let mut throttle = ProgressThrottle::new();
    update
        .download_and_install(
            |downloaded, total| {
                downloaded_total = downloaded_total.saturating_add(downloaded as u64);
                if !throttle.allow(downloaded_total, total) {
                    return;
                }
                if let Err(err) = app.emit(
                    APP_UPDATER_PROGRESS_EVENT,
                    ProgressPayload {
                        downloaded: downloaded_total,
                        total,
                    },
                ) {
                    // 高频路径只记 `debug`，避免刷屏；前端卡 0% 时可开调试日志排查
                    log::debug!("failed to emit updater progress: {err:#}");
                }
            },
            || {},
        )
        .await
        .context("failed to download and install update")?;
    Ok(())
}

/// 下载并安装更新，进度经事件推送；装完不自动重启，由前端手动触发
/// （仅桌面端，移动端直接报错）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn download_and_install(_app: &tauri::AppHandle) -> anyhow::Result<()> {
    Err(anyhow::anyhow!("updater is not supported on mobile"))
}

/// 重启应用以完成更新；调用前确保配置已落盘（前端经命令成功回调后调用）
///
/// 移动端无更新能力，返回错误而非 panic，前端经 `.failed()` 统一处理
/// （仅桌面端真正重启）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[allow(clippy::unnecessary_wraps)]
pub fn restart(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use tauri::Manager;

    tauri::process::restart(&app.env());
    #[allow(unreachable_code)]
    Ok(())
}

/// 重启应用以完成更新；调用前确保配置已落盘（前端经命令成功回调后调用）
///
/// 移动端无更新能力，返回错误而非 panic，前端经 `.failed()` 统一处理
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn restart(_app: &tauri::AppHandle) -> anyhow::Result<()> {
    Err(anyhow::anyhow!("updater is not supported on mobile"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_event_name_is_stable() {
        // 前端硬编码同一字面量（见 `libs/hooks/updater.svelte.ts`），改名需两端同步
        assert_eq!(APP_UPDATER_PROGRESS_EVENT, "app-updater-progress");
    }

    #[test]
    fn progress_throttle_allows_first_emit() {
        let mut throttle = ProgressThrottle::new();
        assert!(throttle.allow(0, Some(100)));
    }

    #[test]
    fn progress_throttle_suppresses_repeat_without_advance() {
        let mut throttle = ProgressThrottle::new();
        assert!(throttle.allow(10, Some(100)));
        // 同值紧随其后：未满 200ms 且无进展，跳过
        assert!(!throttle.allow(10, Some(100)));
    }

    #[test]
    fn progress_throttle_allows_one_percent_advance_immediately() {
        let mut throttle = ProgressThrottle::new();
        assert!(throttle.allow(0, Some(100)));
        // 前进 ≥1% 即放行，不受时间窗限制
        assert!(throttle.allow(1, Some(100)));
        // 同一百分比重复：跳过
        assert!(!throttle.allow(1, Some(100)));
    }

    #[test]
    fn progress_throttle_unknown_total_uses_time_window_only() {
        let mut throttle = ProgressThrottle::new();
        assert!(throttle.allow(10, None));
        assert!(!throttle.allow(20, None));
        // 总数为零视为未知，同样只看时间窗
        assert!(!throttle.allow(30, Some(0)));
    }
}
