//! Shared backend contract; no host runtime, account storage or platform SDK.
pub mod account;
pub mod adapter;
pub mod business;
pub mod cover;
pub mod identity;
pub mod legacy;
pub mod migration;
pub mod resource;
use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    Unauthenticated,
    NotFound,
    PermissionDenied,
    RegionRestricted,
    RateLimited,
    Network,
    Unsupported,
    SourceUnavailable,
    Cancelled,
    DeadlineExceeded,
    StaleContext,
    InvalidData,
    Internal,
}

/// Public errors carry stable codes only. Raw platform errors are not public messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicError {
    pub code: ErrorCode,
    pub retry_after_ms: Option<u64>,
}

impl From<ErrorCode> for MusicError {
    fn from(code: ErrorCode) -> Self {
        Self {
            code,
            retry_after_ms: None,
        }
    }
}

impl std::fmt::Display for MusicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.code {
            ErrorCode::Unauthenticated => "登录已失效，请重新登录",
            ErrorCode::NotFound => "音乐资源不存在",
            ErrorCode::PermissionDenied => "当前资源、账号权限或音质不可用；试听不作为完整歌曲播放",
            ErrorCode::RegionRestricted => "当前地区不可播放",
            ErrorCode::RateLimited => "音乐来源繁忙，请稍后重试",
            ErrorCode::Network => "音乐请求失败，请检查网络后重试",
            ErrorCode::Unsupported => "音乐来源不支持此能力",
            ErrorCode::SourceUnavailable => "音乐来源已停用或不可用",
            ErrorCode::Cancelled => "音乐请求已取消",
            ErrorCode::DeadlineExceeded => "音乐请求超时或播放资源已过期",
            ErrorCode::StaleContext => "账号或音乐来源状态已变化，请重试",
            ErrorCode::InvalidData => "音乐来源返回了无效数据",
            ErrorCode::Internal => "音乐服务不可用",
        })
    }
}

impl std::error::Error for MusicError {}

pub type MusicResult<T> = Result<T, MusicError>;
