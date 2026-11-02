//! 跨层共享能力：从进程参数提取本应用协议 URL 的纯函数，不依赖运行时。
//!
//! 非单独业务，供单实例插件（`plugins::single_instance`）复用，故放在 `cores` 而非 `features`。

/// 深链参数提取：从进程参数里挑出本应用协议的 URL，纯函数，不依赖运行时。
///
/// 协议 `scheme` 的三处 touch 点（模板二次开发自定义时同步改）：
/// `tauri.conf.json` 的 `plugins.deep-link.desktop.schemes`、
/// 此处的 `SCHEME_PREFIX`、前端 `deep-link.svelte.ts` 的同名常量。
pub const SCHEME_PREFIX: &str = "tdt://";

/// 从启动参数中提取深链 URL：仅保留本协议前缀项，其余一律忽略
pub fn extract_deep_link_urls(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.starts_with(SCHEME_PREFIX))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_only_own_scheme_urls() {
        let args = [
            "tauri-desktop-template".to_owned(),
            "tdt://settings".to_owned(),
            "https://example.com".to_owned(),
            "tdt://about?tab=1".to_owned(),
            "--single-instance".to_owned(),
        ];

        assert_eq!(
            extract_deep_link_urls(&args),
            vec!["tdt://settings".to_owned(), "tdt://about?tab=1".to_owned()]
        );
    }

    #[test]
    fn ignores_other_schemes_and_empty_args() {
        let args = ["TDT://settings".to_owned(), "myapp://x".to_owned()];
        assert_eq!(extract_deep_link_urls(&args), Vec::<String>::new());

        let empty: [String; 0] = [];
        assert_eq!(extract_deep_link_urls(&empty), Vec::<String>::new());
    }
}
