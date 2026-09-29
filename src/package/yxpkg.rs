//! `.yxpkg` 源码包格式（RFC-014a 决议 3/7）
//!
//! 包 = gzip 压缩的 tar，内容为项目根下除排除项外的全部源码文件，附
//! `SHA256SUMS` 清单（coreutils 风格 `<sha256>␣␣<path>`，LF 结尾，路径以
//! `/` 分隔、字典序、不含清单自身）。源码包内容总量上限 20 MiB（决议 7）。
//!
//! 打包是确定性的：条目按路径排序、mtime/uid/gid 归零、固定权限位，同一
//! 目录两次打包逐字节一致——publish 的版本比对与 Release 资产 digest 都
//! 依赖这一点。
//!
//! 解包侧强制校验：清单缺失、文件被篡改、清单外的多余文件、路径逃逸
//! （绝对路径 / `..` / 非常规条目类型）与解压总量超限一律报错。
//!
//! 排除项取黑名单而非白名单：`[exports]` 允许把 src/ 之外的文件纳入导出
//! 面（RFC-015/029f），白名单会静默漏掉它们。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Builder, EntryType, Header};

use crate::package::error::{PackageError, PackageResult};
use crate::package::vendor::checksum::compute_file_checksum;

/// 校验清单文件名
pub const SUMS_FILE: &str = "SHA256SUMS";

/// 源码包内容总量上限（RFC-014a 2026-09-15 决议 7：20 MiB，初值可调）
pub const MAX_PACKAGE_BYTES: u64 = 20 * 1024 * 1024;

/// 打包时排除的目录名
const EXCLUDED_DIRS: &[&str] = &[".git", ".hg", ".svn", ".yaoxiang", "target", "node_modules"];

/// 打包时排除的文件名
const EXCLUDED_FILES: &[&str] = &[".DS_Store", "Thumbs.db", SUMS_FILE];

/// 打包时排除的文件后缀
const EXCLUDED_SUFFIXES: &[&str] = &[".yxpkg"];

/// 发布产物的标准文件名：`{name}-{version}.yxpkg`
pub fn artifact_name(
    name: &str,
    version: &str,
) -> String {
    format!("{}-{}.yxpkg", name, version)
}

/// 将项目目录打包为 `.yxpkg`，写入 `out`，返回源码内容总字节数
pub fn pack(
    project_dir: &Path,
    out: &Path,
) -> PackageResult<u64> {
    let files = collect_source_files(project_dir)?;
    let total: u64 = files.iter().map(|(_, size)| size).sum();
    if total > MAX_PACKAGE_BYTES {
        return Err(PackageError::PackageTooLarge(format!(
            "{} bytes of source content, limit is {} bytes",
            total, MAX_PACKAGE_BYTES
        )));
    }

    let sums = build_sums(project_dir, &files)?;

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = PathBuf::from(format!("{}.tmp", out.display()));
    let file = std::fs::File::create(&tmp)?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(gz);

    for (rel, size) in &files {
        builder = append_file_entry(builder, project_dir.join(rel), rel, *size)?;
    }
    builder.append_data(
        &mut sums_header(sums.len() as u64),
        SUMS_FILE,
        sums.as_bytes(),
    )?;

    let gz = builder.into_inner()?;
    gz.finish()?;

    std::fs::rename(&tmp, out)?;
    Ok(total)
}

/// 清单条目的确定性头部（与源码条目同规格）
fn sums_header(size: u64) -> Header {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(size);
    header.set_mode(0o644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header
}

/// 追加一个普通文件条目（确定性元数据：mtime/uid/gid 归零，0644）
fn append_file_entry(
    mut builder: Builder<GzEncoder<std::fs::File>>,
    src: PathBuf,
    arc_path: &str,
    size: u64,
) -> PackageResult<Builder<GzEncoder<std::fs::File>>> {
    let mut header = sums_header(size);
    let file = std::fs::File::open(&src)?;
    builder.append_data(&mut header, arc_path, file)?;
    Ok(builder)
}

/// 收集打包文件清单：`(归档内路径 / 分隔，字节数)`，按路径字典序
fn collect_source_files(project_dir: &Path) -> PackageResult<Vec<(String, u64)>> {
    let mut files = Vec::new();
    collect_dir(project_dir, project_dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn collect_dir(
    base: &Path,
    dir: &Path,
    files: &mut Vec<(String, u64)>,
) -> PackageResult<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        if path.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect_dir(base, &path, files)?;
        } else {
            if EXCLUDED_FILES.contains(&name.as_str())
                || EXCLUDED_SUFFIXES.iter().any(|s| name.ends_with(s))
            {
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let size = entry.metadata()?.len();
            files.push((rel, size));
        }
    }
    Ok(())
}

/// 生成 SHA256SUMS 清单内容
fn build_sums(
    project_dir: &Path,
    files: &[(String, u64)],
) -> PackageResult<String> {
    let mut sums = String::new();
    for (rel, _) in files {
        let hex = compute_file_checksum(&project_dir.join(rel))?;
        sums.push_str(&hex);
        sums.push_str("  ");
        sums.push_str(rel);
        sums.push('\n');
    }
    Ok(sums)
}

/// 解包 `.yxpkg` 到 `dest`，完整性校验通过后保留文件，否则留下部分文件并报错
///
/// 调用方（安装/publish 校验）通常解包到临时目录，校验通过后再移动到位。
pub fn unpack(
    archive: &Path,
    dest: &Path,
) -> PackageResult<()> {
    let file = std::fs::File::open(archive)?;
    let gz = GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    std::fs::create_dir_all(dest)?;
    let mut total: u64 = 0;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if !matches!(entry_type, EntryType::Regular | EntryType::Directory) {
            return Err(PackageError::InvalidPackage(format!(
                "unsupported entry type {:?} for {}",
                entry_type,
                entry.path()?.display()
            )));
        }

        let rel = sanitize_entry_path(&entry.path()?)?;
        if rel.is_empty() {
            continue;
        }
        let target = dest.join(&rel);
        if entry_type == EntryType::Directory {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        total += entry.header().size().unwrap_or(0);
        if total > MAX_PACKAGE_BYTES {
            return Err(PackageError::PackageTooLarge(format!(
                "decompressed content exceeds {} bytes",
                MAX_PACKAGE_BYTES
            )));
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        // 声明 size 之上再对实际读取封顶：声明值可以被伪造
        let mut limited = (&mut entry).take(MAX_PACKAGE_BYTES);
        std::io::copy(&mut limited, &mut out)?;
    }

    verify(dest)
}

/// 归档条目路径 → 包内相对路径（`/` 分隔）；拒绝绝对路径与 `..` 逃逸
fn sanitize_entry_path(p: &Path) -> PackageResult<String> {
    let mut parts: Vec<String> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::Normal(c) => parts.push(c.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => {
                return Err(PackageError::InvalidPackage(format!(
                    "path escapes package root: {}",
                    p.display()
                )))
            }
        }
    }
    Ok(parts.join("/"))
}

/// 对解包后的目录做完整性校验（安装路径与测试共用）
pub(crate) fn verify(dest: &Path) -> PackageResult<()> {
    let sums_path = dest.join(SUMS_FILE);
    if !sums_path.is_file() {
        return Err(PackageError::InvalidPackage(format!(
            "missing {} manifest",
            SUMS_FILE
        )));
    }
    let content = std::fs::read_to_string(&sums_path)?;

    let mut listed: BTreeMap<String, String> = BTreeMap::new();
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((hex, rel)) = line.split_once("  ") else {
            return Err(PackageError::InvalidPackage(format!(
                "malformed manifest line: {:?}",
                line
            )));
        };
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PackageError::InvalidPackage(format!(
                "malformed checksum in manifest: {:?}",
                hex
            )));
        }
        listed.insert(rel.to_string(), hex.to_string());
    }

    let mut present: BTreeSet<String> = BTreeSet::new();
    collect_present(dest, dest, &mut present)?;
    present.remove(SUMS_FILE);

    for rel in &present {
        if !listed.contains_key(rel) {
            return Err(PackageError::InvalidPackage(format!(
                "file not listed in {}: {}",
                SUMS_FILE, rel
            )));
        }
    }
    for rel in listed.keys() {
        if !present.contains(rel) {
            return Err(PackageError::InvalidPackage(format!(
                "listed file missing from package: {}",
                rel
            )));
        }
    }
    for (rel, expected) in &listed {
        let actual = compute_file_checksum(&dest.join(rel))?;
        if &actual != expected {
            return Err(PackageError::ChecksumMismatch {
                expected: expected.clone(),
                actual,
            });
        }
    }
    Ok(())
}

/// 收集目录下全部文件（相对路径 / 分隔）——校验侧不做任何排除
fn collect_present(
    base: &Path,
    dir: &Path,
    out: &mut BTreeSet<String>,
) -> PackageResult<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_present(base, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel);
        }
    }
    Ok(())
}
