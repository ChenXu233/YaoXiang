//! `yx toolchain` 动词组：install / default / list / uninstall / update

use crate::dl;
use crate::error::{Error, Result};
use crate::{home, pin, platform, settings};

pub(crate) const GITHUB_REPO: &str = "ChenXu233/YaoXiang";

/// 入口：`yx toolchain <action> [args]`
pub fn run(args: &[String]) -> Result<()> {
    let action = args.first().map(String::as_str);
    match (action, args.get(1)) {
        (Some("install"), version) => install(version.map(String::as_str).unwrap_or("stable")),
        (Some("default"), Some(version)) => default(version),
        (Some("list") | Some("ls"), _) => list(),
        (Some("uninstall"), Some(version)) => uninstall(version),
        (Some("update"), _) => update(),
        _ => {
            print_toolchain_help();
            std::process::exit(2);
        }
    }
}

fn print_toolchain_help() {
    println!(
        "usage: yx toolchain <command>\n\
         \n\
         commands:\n\
           install [version|stable]   install a toolchain (default: latest stable)\n\
           default <version>          set the default toolchain version\n\
           list                       list installed versions\n\
           uninstall <version>        remove an installed version\n\
           update                     install latest stable and set it as default"
    );
}

/// 发行包下载地址（镜像为 ghproxy 风格前缀）
fn release_urls(
    mirror: Option<&str>,
    version: &str,
    asset: &str,
) -> (String, String) {
    let base = format!("https://github.com/{GITHUB_REPO}/releases/download/v{version}");
    (
        settings::download_url(mirror, &format!("{base}/{asset}")),
        settings::download_url(mirror, &format!("{base}/{asset}.sha256")),
    )
}

/// 最新 stable 版本号（剥 `v` 前缀）。
///
/// 主路径走 GitHub Releases API（镜像同样前缀拼接）；API 限流或不可达时
/// 回退到 releases/latest 页面的重定向落地 URL 提取 tag——无需 API 权限，
/// 不受限流影响。
pub(crate) fn latest_stable_version(mirror: Option<&str>) -> Result<String> {
    let api = settings::download_url(
        mirror,
        &format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest"),
    );
    if let Ok(text) = dl::get_text(&api) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
                return Ok(pin::normalize_version(tag));
            }
        }
    }

    let page = settings::download_url(
        mirror,
        &format!("https://github.com/{GITHUB_REPO}/releases/latest"),
    );
    let landing = dl::get_redirect_target(&page)?;
    parse_tag_from_release_url(&landing).ok_or_else(|| {
        Error::Message(format!(
            "cannot determine latest version: API failed and release page URL \
             does not carry a tag: {landing}"
        ))
    })
}

/// 从 releases/latest 的重定向落地 URL 提取版本
/// （`…/releases/tag/v0.8.0` → `0.8.0`；无 tag 形态返回 None）
fn parse_tag_from_release_url(url: &str) -> Option<String> {
    const MARKER: &str = "/releases/tag/";
    let idx = url.find(MARKER)? + MARKER.len();
    let rest = url[idx..].split(['?', '#']).next()?;
    Some(pin::normalize_version(rest))
}

/// 安装一个版本：下载 → 校验 → 解包进 versions/<ver>/
fn install(version_input: &str) -> Result<()> {
    let mirror = settings::Settings::load(&home::yaoxiang_home()?)?.mirror;
    let version = if version_input == "stable" {
        latest_stable_version(mirror.as_deref())?
    } else {
        pin::normalize_version(version_input)
    };
    let home = home::yaoxiang_home()?;
    let target = platform::triple()?;
    let os = std::env::consts::OS;
    let asset = platform::asset_name(&version, target, os);
    let dest = home::version_dir(&home, &version);

    if dest.join("bin").join(platform::engine_file_name()).exists() {
        println!("yx: {version} already installed");
        return Ok(());
    }

    let (asset_url, sha_url) = release_urls(mirror.as_deref(), &version, &asset);
    let tmp = home
        .join("versions")
        .join(format!(".downloading-{version}-{asset}"));
    let tmp_sha = home
        .join("versions")
        .join(format!(".downloading-{version}-{asset}.sha256"));

    println!("yx: downloading {asset_url}");
    dl::download_to_file(&asset_url, &tmp)?;
    println!("yx: verifying checksum");
    match dl::download_to_file(&sha_url, &tmp_sha) {
        Ok(_) => {
            let expected = std::fs::read_to_string(&tmp_sha)?;
            dl::verify_sha256(&tmp, &expected)?;
        }
        // 无 .sha256 旁证时降级为只提示（老版本 Release 可能没有）
        Err(Error::Network(_)) => {
            println!("yx: warning: .sha256 not available, skipping verification");
        }
        Err(e) => return Err(e),
    }

    println!("yx: unpacking into {}", dest.display());
    dl::unpack(&tmp, os, &dest)?;
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_sha);

    // 首个安装的版本自动设为默认
    let mut settings = settings::Settings::load(&home)?;
    if settings.default.is_none() {
        settings.default = Some(version.clone());
        settings.save(&home)?;
        println!("yx: {version} set as default");
    }
    println!("yx: installed {version}");
    Ok(())
}

/// 设置默认版本
fn default(version_input: &str) -> Result<()> {
    let version = pin::normalize_version(version_input);
    let home = home::yaoxiang_home()?;
    let engine = home::version_dir(&home, &version)
        .join("bin")
        .join(platform::engine_file_name());
    if !engine.exists() {
        return Err(Error::NotInstalled { version });
    }
    let mut settings = settings::Settings::load(&home)?;
    settings.default = Some(version.clone());
    settings.save(&home)?;
    println!("yx: default toolchain set to {version}");
    Ok(())
}

/// 列出已安装版本（标注默认与项目 pin）
fn list() -> Result<()> {
    let home = home::yaoxiang_home()?;
    let versions_dir = home::versions_dir()?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let pinned = pin::find_pin_from(&cwd);
    let default = settings::Settings::load(&home)?.default;

    let mut versions: Vec<String> = match std::fs::read_dir(&versions_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .collect(),
        // 安装根尚不存在 = 还没装过任何版本
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    versions.sort();
    versions.reverse();

    if versions.is_empty() {
        println!("yx: no toolchains installed");
        return Ok(());
    }
    for version in &versions {
        let mut tags = Vec::new();
        if default.as_deref() == Some(version) {
            tags.push("default");
        }
        if pinned.as_deref() == Some(version) {
            tags.push("pinned");
        }
        if tags.is_empty() {
            println!("  {version}");
        } else {
            println!("  {version} ({})", tags.join(", "));
        }
    }
    Ok(())
}

/// 卸载前置检查：默认版本与项目 pin 都不允许直接卸载（抽纯函数供测试）
fn check_uninstall_allowed(
    version: &str,
    default: Option<&str>,
    pinned: Option<&str>,
) -> Result<()> {
    if default == Some(version) {
        return Err(Error::IsDefault {
            version: version.to_string(),
        });
    }
    if pinned == Some(version) {
        return Err(Error::Message(format!(
            "toolchain {version} is pinned by yx-toolchain.toml; remove the pin first"
        )));
    }
    Ok(())
}

/// 卸载版本（默认版本须先切换，防止把正在用的引擎删掉）
fn uninstall(version_input: &str) -> Result<()> {
    let version = pin::normalize_version(version_input);
    let home = home::yaoxiang_home()?;
    let dir = home::version_dir(&home, &version);
    if !dir.exists() {
        return Err(Error::NotInstalled { version });
    }
    let current_default = settings::Settings::load(&home)?.default;
    let cwd = std::env::current_dir().unwrap_or_default();
    let pinned = pin::find_pin_from(&cwd);
    check_uninstall_allowed(&version, current_default.as_deref(), pinned.as_deref())?;
    std::fs::remove_dir_all(&dir)?;
    println!("yx: uninstalled {version}");
    Ok(())
}

/// 更新到最新 stable 并设为默认
fn update() -> Result<()> {
    let mirror = settings::Settings::load(&home::yaoxiang_home()?)?.mirror;
    let version = latest_stable_version(mirror.as_deref())?;
    install(&version)?;
    default(&version)
}

#[cfg(test)]
mod tests;
