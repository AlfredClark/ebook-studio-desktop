//! 语言类型：前后端共用的 `Locale` 枚举与系统语言归一解析。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 应用支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub enum Locale {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

impl Locale {
    /// 取对应的 BCP-47 标签，与 `serde` 落盘值一致，前后端契约共用
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }

    /// 容错解析 BCP-47 语言标签（`zh-Hans-CN`、`zh_CN`、`en-US` 等）
    ///
    /// 有意归一：当前仅支持 `en` / `zh-CN`，`zh-HK` / `zh-TW` / `zh-Hant` 等一律归 `zh-CN`，
    /// 未知语言一律回落默认 `en`。新增语言时同步扩展此处、`locales/*.yml` 与前端文案。
    pub fn parse(raw: &str) -> Self {
        let primary = raw.trim().split(['-', '_']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("zh") {
            Self::ZhCn
        } else if primary.eq_ignore_ascii_case("en") {
            Self::En
        } else {
            Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locale_tags() {
        let cases = [
            ("en", Locale::En),
            ("EN", Locale::En),
            ("en-US", Locale::En),
            ("zh", Locale::ZhCn),
            ("zh-CN", Locale::ZhCn),
            ("zh_CN", Locale::ZhCn),
            ("zh-Hans-CN", Locale::ZhCn),
            ("zh-HK", Locale::ZhCn),
            ("zh-TW", Locale::ZhCn),
            ("zh-Hant", Locale::ZhCn),
            ("ZH-cn", Locale::ZhCn),
            ("  zh-CN  ", Locale::ZhCn),
        ];
        for (raw, expected) in cases {
            assert_eq!(
                Locale::parse(raw),
                expected,
                "unexpected result for {raw:?}"
            );
        }
    }

    #[test]
    fn falls_back_to_default() {
        assert_eq!(Locale::parse(""), Locale::En);
        assert_eq!(Locale::parse("fr"), Locale::En);
    }

    #[test]
    fn round_trips_through_as_str() {
        for locale in [Locale::En, Locale::ZhCn] {
            assert_eq!(Locale::parse(locale.as_str()), locale);
        }
    }
}
