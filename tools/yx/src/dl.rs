//! 下载、校验与解包（RFC-037）

use std::io::Read;
use std::path::Path;

use crate::error::{Error, Result};

const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

/// 把 URL 内容流式下载到 `dest`，返回字节数
pub fn download_to_file(
    url: &str,
    dest: &Path,
) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build();
    let mut reader = agent.get(url).call()?.into_reader();
    let mut file = std::fs::File::create(dest)?;
    let mut count = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])?;
        count += n as u64;
    }
    Ok(count)
}

/// GET 一个短文本（版本号查询等）
pub fn get_text(url: &str) -> Result<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .build();
    Ok(agent.get(url).call()?.into_string()?)
}

/// GET 只为拿最终落地 URL（releases/latest 页面会 302 到 /releases/tag/v<ver>），
/// 借此在 API 不可达时探测最新版本；不读 body
pub fn get_redirect_target(url: &str) -> Result<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .build();
    Ok(agent.get(url).call()?.get_url().to_string())
}

/// 文件的 SHA-256（与 package-dist.sh 产出的 .sha256 比对）
pub fn file_sha256(path: &Path) -> Result<String> {
    use sha2::Digest;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 校验 `archive` 与 `.sha256` 旁证文件（首字段为 hex digest）
pub fn verify_sha256(
    archive: &Path,
    expected: &str,
) -> Result<()> {
    let actual = file_sha256(archive)?;
    let expected = expected
        .split_whitespace()
        .next()
        .unwrap_or(expected)
        .to_lowercase();
    if actual != expected {
        return Err(Error::ChecksumMismatch {
            path: archive.to_path_buf(),
            expected,
            actual,
        });
    }
    Ok(())
}

/// 解包发行包到 `dest`，剥掉顶层目录（`yaoxiang-<ver>-<triple>/`）——
/// 版本目录即发行包解压根目录（RFC-037）
pub fn unpack(
    archive: &Path,
    os: &str,
    dest: &Path,
) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    if os == "windows" {
        unpack_zip(archive, dest)
    } else {
        unpack_tar_gz(archive, dest)
    }
}

fn unpack_tar_gz(
    archive: &Path,
    dest: &Path,
) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let target = strip_first(&path, dest).ok_or_else(|| {
            Error::Message(format!("unexpected archive entry: {}", path.display()))
        })?;
        if path.components().count() <= 1 {
            continue;
        }
        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
        }
    }
    Ok(())
}

fn unpack_zip(
    archive: &Path,
    dest: &Path,
) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry
            .enclosed_name()
            .ok_or_else(|| Error::Message(format!("unsafe archive entry: {}", entry.name())))?;
        if name.components().count() <= 1 {
            continue;
        }
        let target = strip_first(&name, dest).ok_or_else(|| {
            Error::Message(format!("unexpected archive entry: {}", name.display()))
        })?;
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// 剥掉首个路径组件后接到 `dest`（顶层目录 = 发行包名）
fn strip_first(
    path: &Path,
    dest: &Path,
) -> Option<std::path::PathBuf> {
    let mut components = path.components();
    components.next()?;
    Some(components.collect::<std::path::PathBuf>())
        .filter(|rest| !rest.as_os_str().is_empty())
        .map(|rest| dest.join(rest))
}

#[cfg(test)]
mod tests;
