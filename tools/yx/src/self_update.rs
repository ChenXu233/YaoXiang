//! `yx self update`：替换前门本体为最新 stable 包内的 yx

use crate::error::{Error, Result};
use crate::{dl, home, pin, platform, settings};

const GITHUB_REPO: &str = "ChenXu233/YaoXiang";

pub fn run() -> Result<()> {
    let mirror = settings::Settings::load(&home::yaoxiang_home()?)?.mirror;
    let api = settings::download_url(
        mirror.as_deref(),
        &format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest"),
    );
    let json: serde_json::Value = serde_json::from_str(&dl::get_text(&api)?)?;
    let version = pin::normalize_version(
        json.get("tag_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("GitHub API response missing tag_name".into()))?,
    );

    let os = std::env::consts::OS;
    let target = platform::triple()?;
    let asset = platform::asset_name(&version, target, os);
    let base = format!("https://github.com/{GITHUB_REPO}/releases/download/v{version}");
    let url = settings::download_url(mirror.as_deref(), &format!("{base}/{asset}"));

    println!("yx: downloading {asset}");
    let tmp = home::yaoxiang_home()?.join(format!(".self-update-{asset}"));
    dl::download_to_file(&url, &tmp)?;

    // 解包到临时目录，只取 bin/yx 覆盖自身（Windows 运行中不能覆盖，走 rename 换位）
    let unpack_dir = home::yaoxiang_home()?.join(format!(".self-update-unpack-{version}"));
    let _ = std::fs::remove_dir_all(&unpack_dir);
    dl::unpack(&tmp, os, &unpack_dir)?;
    let _ = std::fs::remove_file(&tmp);

    let new_yx = unpack_dir.join("bin").join(platform::yx_file_name());
    let current = std::env::current_exe().map_err(|e| Error::Message(e.to_string()))?;
    let bin_dir = current
        .parent()
        .ok_or_else(|| Error::Message("cannot locate yx binary directory".into()))?
        .to_path_buf();

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
