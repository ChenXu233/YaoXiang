//! 安装根与目录约定（RFC-037）
//!
//! `~/.yaoxiang/`（Windows `%USERPROFILE%\.yaoxiang`）为唯一安装根，
//! `YAOXIANG_HOME` 环境变量覆盖（先例 RUSTUP_HOME / DENO_INSTALL，服务 CI 与容器）：
//!
//! ```text
//! ~/.yaoxiang/
//! ├── bin/yx                  # 前门本体
//! ├── settings.toml           # 默认版本、镜像源
//! └── versions/<ver>/         # 版本目录 = 发行包解压根目录
//!     ├── bin/{yaoxiang-rs, libz3.*}
//!     └── lib/yaoxiang/std/
//! ```

use std::path::Path;

use crate::error::{Error, Result};

/// 安装根：`YAOXIANG_HOME` 优先，缺省 `~/.yaoxiang`
pub fn yaoxiang_home() -> Result<std::path::PathBuf> {
    yaoxiang_home_from(
        std::env::var("YAOXIANG_HOME").ok().as_deref(),
        user_home().as_deref(),
    )
}

/// `yaoxiang_home` 的纯函数形态（env 与用户目录可注入，供测试）
pub fn yaoxiang_home_from(
    env_home: Option<&str>,
    user_home: Option<&str>,
) -> Result<std::path::PathBuf> {
    if let Some(home) = env_home.filter(|h| !h.is_empty()) {
        return Ok(std::path::PathBuf::from(home));
    }
    user_home
        .map(|h| std::path::PathBuf::from(h).join(".yaoxiang"))
        .ok_or(Error::NoHome)
}

fn user_home() -> Option<String> {
    if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    }
}

/// 前门本体目录 `~/.yaoxiang/bin/`
pub fn bin_dir() -> Result<std::path::PathBuf> {
    Ok(yaoxiang_home()?.join("bin"))
}

/// 版本树 `~/.yaoxiang/versions/`
pub fn versions_dir() -> Result<std::path::PathBuf> {
    Ok(yaoxiang_home()?.join("versions"))
}

/// 单个版本目录 `versions/<ver>/`
pub fn version_dir(
    home: &Path,
    version: &str,
) -> std::path::PathBuf {
    home.join("versions").join(version)
}
