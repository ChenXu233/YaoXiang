//! 下载、校验与解包（RFC-037）

use std::io::Read;
use std::path::Path;

use ureq::ResponseExt;

use crate::error::{Error, Result};

const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

/// 重定向上限。ureq 2 默认 `redirects: 5`；ureq 3 默认 10——显式对齐 2.x 行为（#489）。
const UREQ_MAX_REDIRECTS: u32 = 5;

/// 共享 HTTP agent 构造（统一连接超时与重定向上限；大文件下载另行加读超时）。
///
/// `.proxy(None)` 显式禁用环境代理——ureq 3 从环境拾取代理是默认行为（
/// `Config::default()` 即含 `Proxy::try_from_env()`），且 `Agent::config_builder()`
/// 的起点同为 `Config::default()`，不显式禁用就会读；禁用对位 ureq 2（
/// proxy-from-env 为 opt-in feature 且未启用，不读环境代理），与 src/std/net.rs
/// 的 ureq_agent 同链一致。4xx/5xx 保持默认按错误处理（对位 ureq 2 的默认）。
fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .proxy(None)
        .timeout_connect(Some(std::time::Duration::from_secs(30)))
        .max_redirects(UREQ_MAX_REDIRECTS)
        .build()
        .into()
}

/// 把 URL 内容流式下载到 `dest`，返回字节数
pub fn download_to_file(
    url: &str,
    dest: &Path,
) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut reader = http_agent()
        .get(url)
        // 大文件下载：读段逐次限时（对位 ureq 2 的 AgentBuilder::timeout_read）
        .config()
        .timeout_per_read(Some(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS)))
        .build()
        .call()?
        .into_body()
        .into_reader();
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
    Ok(http_agent().get(url).call()?.body_mut().read_to_string()?)
}

/// GET 只为拿最终落地 URL（releases/latest 页面会 302 到 /releases/tag/v<ver>），
/// 借此在 API 不可达时探测最新版本；不读 body
pub fn get_redirect_target(url: &str) -> Result<String> {
    Ok(http_agent().get(url).call()?.get_uri().to_string())
}

/// 文件的 SHA-256（与 package-dist.sh 产出的 .sha256 比对）。
/// 不用 hasher 的 Write impl、也不用输出类型的 LowerHex——sha2 0.11
/// （hybrid-array 取代 generic-array）两者皆无；手动循环 + 逐字节
/// 十六进制化，对 0.10/0.11 都成立
pub fn file_sha256(path: &Path) -> Result<String> {
    use sha2::Digest;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
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
        // 纵深防御：拒绝带 .. 的条目（zip 侧由 enclosed_name 保护，tar 侧在此把关）
        if !entry_path_is_safe(&path) {
            return Err(Error::Message(format!(
                "unsafe archive entry (parent traversal): {}",
                path.display()
            )));
        }
        // 发行包不含任何链接条目（package-dist.sh 只铺真实文件）。符号链接/硬链接
        // 的目标是写穿解包目录的经典向量（tar-rs 的 Entry::unpack 不校验链接目标），
        // 按构造拒绝而非逐一校验目标
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            let kind_name = if kind.is_symlink() {
                "symlink"
            } else {
                "hardlink"
            };
            return Err(Error::Message(format!(
                "unsafe archive entry ({kind_name}): {}",
                path.display()
            )));
        }
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
        // zip 9 起 ZipFile::name 返回 Result（非 UTF-8 条目名取不到），错误信息退回占位符
        let raw = entry
            .name()
            .map(std::borrow::Cow::into_owned)
            .unwrap_or_else(|_| "<non-UTF-8 entry name>".to_string());
        // zip 9 起 enclosed_name 由「拒绝 ..」改为「规范化掉 ..」（实测 `pkg/../evil.txt` 得
        // `Some("evil.txt")`——条目被静默改写到另一个落点）。故先用与 tar 侧同一个
        // entry_path_is_safe 判据拒掉，enclosed_name 只用于挡绝对路径 / 盘符等其余形态
        if !entry_path_is_safe(std::path::Path::new(&raw)) {
            return Err(Error::Message(format!("unsafe archive entry: {raw}")));
        }
        let name = entry
            .enclosed_name()
            .ok_or_else(|| Error::Message(format!("unsafe archive entry: {raw}")))?;
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

/// 条目路径安全检查：不得包含 `..` 组件（防写穿解包目标目录）
pub(crate) fn entry_path_is_safe(path: &Path) -> bool {
    !path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

/// 剥掉首个路径组件后接到 `dest`（顶层目录 = 发行包名）
pub(crate) fn strip_first(
    path: &Path,
    dest: &Path,
) -> Option<std::path::PathBuf> {
    let mut components = path.components();
    components.next()?;
    Some(components.collect::<std::path::PathBuf>())
        .filter(|rest| !rest.as_os_str().is_empty())
        .map(|rest| dest.join(rest))
}
