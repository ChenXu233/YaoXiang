//! 预编译二进制分发（RFC-014b Phase 5d）
//!
//! `[binaries]` 是唯一二进制分发机制（014a 决议 3：.yxpkg 只含源码）。
//! 决策树第一步：当前平台有条目且「下载成功 + SHA-256 校验通过」才走预编译，
//! 任一条件不满足回退源码构建（RFC「跳过构建的条件」）——回退前清理半成品，
//! 避免与后续源码构建产物混装。

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::package::cache::sanitize_key;
use crate::package::error::PackageResult;
use crate::package::manifest::PackageManifest;
use crate::package::vendor::checksum::compute_file_checksum;
use crate::package::yxpkg;

use super::current_triple;

/// 预编译资产下载超时（构建产物可能远大于源码包，放宽到 10 分钟）
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// 尝试预编译路径：命中返回产物目录（`build/native/<triple>/`），未命中返回
/// None（调用方回退源码构建）
///
/// sha256 未声明的条目视为不可用（完整性不可验证即不启用）；相对 URL 相对
/// 包仓库地址解析（`[package].repository` / git 来源 URL）。
pub(crate) async fn try_prebuilt(
    pkg_dir: &Path,
    manifest: &PackageManifest,
    source_base: Option<&str>,
) -> PackageResult<Option<PathBuf>> {
    let Some(entry) = manifest.binaries.get(&current_triple()) else {
        return Ok(None);
    };
    let Some(expected) = entry.sha256.as_deref().map(str::to_lowercase) else {
        eprintln!(
            "⚠ [binaries] {} 无 sha256，完整性不可验证，回退源码构建",
            current_triple()
        );
        return Ok(None);
    };
    let Some(url) = resolve_url(&entry.url, source_base) else {
        eprintln!(
            "⚠ [binaries] 相对 URL '{}' 无仓库基址可解析，回退源码构建",
            entry.url
        );
        return Ok(None);
    };

    let native_dir = pkg_dir.join("build").join("native").join(current_triple());
    let scratch = std::env::temp_dir().join(format!(
        "yx-bin-{}-{}",
        std::process::id(),
        sanitize_key(&url)
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch)?;

    let file_name = url
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("artifact.tar.gz");
    let archive = scratch.join(sanitize_key(file_name));

    let client = crate::package::http::client(false);
    let token = std::env::var("YX_GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    let outcome = install_artifact(
        &client,
        &url,
        token.as_deref(),
        &archive,
        &expected,
        &native_dir,
    )
    .await;

    let _ = std::fs::remove_dir_all(&scratch);
    match outcome {
        Ok(dir) => Ok(Some(dir)),
        Err(e) => {
            // 回退语义（RFC）：条件不满足 → 源码构建，不硬失败
            eprintln!("⚠ 预编译产物不可用（{e}），回退源码构建");
            let _ = std::fs::remove_dir_all(&native_dir);
            Ok(None)
        }
    }
}

/// 下载 → 整包校验 → 安全解包到 `native_dir`
async fn install_artifact(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    archive: &Path,
    expected_sha256: &str,
    native_dir: &Path,
) -> PackageResult<PathBuf> {
    crate::package::http::download_file(
        client,
        url,
        token,
        archive,
        DOWNLOAD_TIMEOUT,
        Duration::from_secs(1),
    )
    .await?;

    let actual = compute_file_checksum(archive)?;
    if actual != expected_sha256 {
        return Err(crate::package::PackageError::ChecksumMismatch {
            expected: expected_sha256.to_string(),
            actual,
        });
    }

    // 外部 tarball 无清单；总量封顶沿用源码包上限（20 MiB 为源码初值，
    // 构建产物放宽——决议 5：产物大小不设上限，用 512 MiB 作解压护栏）
    yxpkg::extract_archive(archive, native_dir, 512 * 1024 * 1024)?;
    Ok(native_dir.to_path_buf())
}

/// 产物 URL 解析：绝对 http(s) 直用；相对路径拼仓库基址
fn resolve_url(
    url: &str,
    source_base: Option<&str>,
) -> Option<String> {
    if url.starts_with("https://") || url.starts_with("http://") {
        return Some(url.to_string());
    }
    let base = source_base?;
    if !base.starts_with("https://") && !base.starts_with("http://") {
        return None;
    }
    Some(format!(
        "{}/{}",
        base.trim_end_matches('/'),
        url.trim_start_matches('/')
    ))
}
