//! 版本解析与引擎派发决策（RFC-037）
//!
//! 优先级：项目 pin（yx-toolchain.toml）> 默认版本（settings.toml）>
//! 相邻引擎（便携解压回退，行为与托管安装一致）。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::{home, pin, platform, settings};

/// 解析结果：Managed 指向 `versions/<ver>/bin/yaoxiang-rs`，Portable 为解压包内相邻引擎
#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Managed { version: String, engine: PathBuf },
    Portable { engine: PathBuf },
}

impl Resolution {
    pub fn engine(&self) -> &Path {
        match self {
            Resolution::Managed { engine, .. } => engine,
            Resolution::Portable { engine } => engine,
        }
    }
}

pub fn resolve() -> Result<Resolution> {
    let home = home::yaoxiang_home()?;
    let cwd = std::env::current_dir().map_err(|_| Error::Message("cannot get cwd".into()))?;
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()));
    resolve_in(&home, &cwd, exe_dir.as_deref())
}

/// `resolve` 的纯函数形态（home/cwd/exe 目录可注入，供测试）
pub fn resolve_in(
    home: &Path,
    cwd: &Path,
    exe_dir: Option<&Path>,
) -> Result<Resolution> {
    let engine_name = platform::engine_file_name();
    let installed = |version: &str| {
        let engine = home::version_dir(home, version)
            .join("bin")
            .join(&engine_name);
        engine.exists().then_some(engine)
    };

    // 1. 项目 pin：存在即权威；未安装则明确报错（显式失败优于静默回退）
    if let Some(version) = pin::find_pin_from(cwd) {
        return match installed(&version) {
            Some(engine) => Ok(Resolution::Managed { version, engine }),
            None => Err(Error::NotInstalled { version }),
        };
    }

    // 2. 默认版本（settings.toml）
    let configured_default = settings::Settings::load(home)?.default;
    if let Some(version) = configured_default {
        return match installed(&version) {
            Some(engine) => Ok(Resolution::Managed { version, engine }),
            None => Err(Error::NotInstalled { version }),
        };
    }

    // 3. 便携回退：解压即用场景，yx 与引擎同在 bin/
    if let Some(bin) = exe_dir {
        let engine = bin.join(&engine_name);
        if engine.exists() {
            return Ok(Resolution::Portable { engine });
        }
    }

    Err(Error::NoToolchain)
}

#[cfg(test)]
mod tests;
