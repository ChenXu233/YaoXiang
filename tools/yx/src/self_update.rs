//! `yx self update`：替换前门本体为最新 stable 包内的 yx

use std::path::Path;

use crate::error::{Error, Result};
use crate::{dl, home, platform, settings, toolchain};

pub fn run() -> Result<()> {
    // 清理上次中断留下的临时残骸（.self-update-*）
    let home_path = home::yaoxiang_home()?;
    home::clean_stale(&home_path, &[".self-update-"]);

    let mirror = settings::Settings::load(&home_path)?.mirror;
    let version = toolchain::latest_stable_version(mirror.as_deref())?;

    // 替换的是 yx 本体，先确认所在目录可写再下载——安装器（Inno，{autopf}）
    // 或系统目录安装时无写权限，提前给出可行动错误而不是白下载后 io error
    let current = std::env::current_exe().map_err(|e| Error::Message(e.to_string()))?;
    let bin_dir = current
        .parent()
        .ok_or_else(|| Error::Message("cannot locate yx binary directory".into()))?
        .to_path_buf();
    ensure_dir_writable(&bin_dir)?;

    let os = std::env::consts::OS;
    let target = platform::triple()?;
    let asset = platform::asset_name(&version, target, os);
    let base = format!(
        "https://github.com/{}/releases/download/v{version}",
        toolchain::GITHUB_REPO
    );
    let url = settings::download_url(mirror.as_deref(), &format!("{base}/{asset}"));
    let sha_url = settings::download_url(mirror.as_deref(), &format!("{base}/{asset}.sha256"));

    let tmp = home_path.join(format!(".self-update-{asset}"));
    let tmp_sha = home_path.join(format!(".self-update-{asset}.sha256"));
    // 自更新替换的是 yx 本体，校验与 install 同规格
    toolchain::fetch_and_verify(&url, &sha_url, &tmp, &tmp_sha)?;

    // 解包到临时目录，只取 bin/yx 覆盖自身（Windows 运行中不能覆盖，走 rename 换位）
    let unpack_dir = home_path.join(format!(".self-update-unpack-{version}"));
    let _ = std::fs::remove_dir_all(&unpack_dir);
    dl::unpack(&tmp, os, &unpack_dir)?;
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_sha);

    let new_yx = unpack_dir.join("bin").join(platform::yx_file_name());

    let staged = bin_dir.join("yx.new");
    std::fs::copy(&new_yx, &staged)?;
    let old = bin_dir.join("yx.old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&current, &old)?;
    std::fs::rename(&staged, &current)?;
    let _ = std::fs::remove_dir_all(&unpack_dir);

    println!(
        "yx: updated to {version} (old binary left as {})",
        old.display()
    );
    Ok(())
}

/// 写探针：确认 `dir` 可写，不可写时报可行动错误（安装器渠道升级 / 提权重试）。
/// 端到端探测而非看权限位——Windows 目录 ACL 与只读标志都不可靠
pub(crate) fn ensure_dir_writable(dir: &Path) -> Result<()> {
    let probe = dir.join(".yx-write-probe");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(e) => Err(Error::Message(format!(
            "cannot write to {}: {e}; if yx was installed by the YaoXiang \
             installer, update via the installer, or re-run elevated",
            dir.display()
        ))),
    }
}
