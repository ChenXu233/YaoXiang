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
