//! 错误类型（RFC-037 yx 前门）

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported platform {os}/{arch} (prebuilt toolchains: linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64)")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("no toolchain installed; run `yx toolchain install stable` first")]
    NoToolchain,

    #[error("toolchain {version} is not installed; run `yx toolchain install {version}` first")]
    NotInstalled { version: String },

    #[error(
        "toolchain {version} is the default; run `yx toolchain default <other-version>` before uninstalling it"
    )]
    IsDefault { version: String },

    #[error("cannot determine home directory (set YAOXIANG_HOME or USERPROFILE/HOME)")]
    NoHome,

    #[error("network error: {0}")]
    Network(#[from] ureq::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("settings parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("settings serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 网络失败补镜像自助指引。E2E 实证：受限网络下报错只有
/// `network error: <url>: …`，用户不知道 settings.toml 的 mirror 可用。
/// 只包在"网络失败即硬失败"的下载点（发行包下载、版本探测），
/// 不得用于 .sha256 下载——那条路径靠匹配 `Network` 降级为告警。
pub(crate) fn with_mirror_hint(e: Error) -> Error {
    Error::Message(format!(
        "{e}; if this network cannot reach GitHub, set a `mirror` in \
         settings.toml (ghproxy-style URL prefix; see the installation guide)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_hint_appends_self_help() {
        let hinted = with_mirror_hint(Error::Message("network error: x: status code 404".into()));
        let text = hinted.to_string();
        assert!(text.starts_with("network error: x: status code 404;"));
        assert!(text.contains("mirror"));
        assert!(text.contains("settings.toml"));
    }
}
