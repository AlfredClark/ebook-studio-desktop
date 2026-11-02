//! 跨层共享类型：统一命令错误与结果别名，前端契约由此经 `specta` 导出。

use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// 命令统一返回的错误类型。
///
/// `serde` 表示即前端契约：序列化为 `{ kind, message }`，前端 `.failed()` 回调按此形状收参。
#[derive(Debug, Error, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    /// 内部错误（`anyhow` 的链式上下文会展开成可读文本）
    #[error("Internal Error: {0}")]
    Internal(String),
}

/// 让命令内可以直接对 `anyhow::Result` 使用 `?`，错误自动转成 [`CommandError::Internal`]
#[allow(clippy::use_self)]
impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        CommandError::Internal(format!("{err:#}"))
    }
}

pub type CommandResult<D> = Result<D, CommandError>;
