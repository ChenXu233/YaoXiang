//! 目标平台识别与发行包命名（RFC-037）

use crate::error::{Error, Result};

/// 当前主机的发行包 target triple
pub fn triple() -> Result<&'static str> {
    triple_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// (os, arch) → target triple（与 RFC-037 §平台支持 的 5 个 target 一致）
pub fn triple_for(
    os: &str,
    arch: &str,
) -> Result<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        (os, arch) => Err(Error::UnsupportedPlatform {
            os: os.to_string(),
            arch: arch.to_string(),
        }),
    }
}

/// 发行包归档扩展名（package-dist.sh 产出：Windows zip，其余 tar.gz）
pub fn archive_ext_for(os: &str) -> &'static str {
    if os == "windows" {
        "zip"
    } else {
        "tar.gz"
    }
}

/// 引擎二进制文件名
pub fn engine_file_name_for(os: &str) -> &'static str {
    if os == "windows" {
        "yaoxiang-rs.exe"
    } else {
        "yaoxiang-rs"
    }
}

/// 前门二进制文件名
pub fn yx_file_name_for(os: &str) -> &'static str {
    if os == "windows" {
        "yx.exe"
    } else {
        "yx"
    }
}

/// 发行包资产名（package-dist.sh 的命名约定）
pub fn asset_name(
    version: &str,
    target: &str,
    os: &str,
) -> String {
    format!("yaoxiang-{}-{}.{}", version, target, archive_ext_for(os))
}

pub fn engine_file_name() -> String {
    engine_file_name_for(std::env::consts::OS).to_string()
}

pub fn yx_file_name() -> String {
    yx_file_name_for(std::env::consts::OS).to_string()
}

#[cfg(test)]
mod tests;
