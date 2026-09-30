//! `.yxpkg` 打包/解包测试 — 基于 RFC-014a §包格式（.yxpkg）+ 决议 3/7
//!
//! 决议 3：仅含源码（SHA256SUMS 清单），`build/native/` 预编译产物目录删除；
//! 决议 7：源码包内容总量 20 MiB 上限。覆盖：round-trip、排除项、打包确定性
//! （mtime/uid/gid 归零——publish 与 Release digest 的依赖）、篡改/夹带/路径
//! 逃逸/超限的拒绝路径。

use std::io::Write as _;
use std::path::Path;

use crate::package::error::PackageError;
use crate::package::yxpkg;

fn write(
    path: &Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs for fixture");
    }
    std::fs::write(path, content)
        .unwrap_or_else(|e| panic!("write fixture {}: {e}", path.display()));
}

fn sample_project(root: &Path) {
    write(
        &root.join("yaoxiang.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    );
    write(&root.join("src/lib.yx"), "pub x = 1\n");
    write(&root.join("src/deep/util.yx"), "pub y = 2\n");
    write(&root.join("build.yx"), "// build script\n");
}

#[test]
fn test_round_trip_preserves_content() {
    // Arrange
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let artifact = tmp.path().join("demo-1.0.0.yxpkg");

    // Act：打包 → 解包
    let total = yxpkg::pack(&project, &artifact).unwrap();
    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();

    // Assert：内容逐文件一致，清单为 coreutils 双空格格式（/ 分隔路径）
    assert!(artifact.is_file(), "产物应存在");
    assert!(total > 0, "源码内容字节数应大于 0");
    assert_eq!(
        std::fs::read_to_string(dest.join("yaoxiang.toml")).unwrap(),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("src/lib.yx")).unwrap(),
        "pub x = 1\n"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("src/deep/util.yx")).unwrap(),
        "pub y = 2\n"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("build.yx")).unwrap(),
        "// build script\n"
    );
    let sums = std::fs::read_to_string(dest.join(yxpkg::SUMS_FILE)).unwrap();
    assert!(
        sums.contains("  src/lib.yx\n"),
        "清单应为「双空格 + / 分隔路径」格式: {sums:?}"
    );
}

#[test]
fn test_pack_omits_excluded_entries() {
    // Arrange：各类排除项（VCS/缓存/构建产物/旧归档/系统文件）+ 正常源码
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    write(&project.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&project.join(".yaoxiang/vendor/x/y.yx"), "junk\n");
    write(&project.join("target/debug/a.rmeta"), "junk\n");
    write(&project.join("node_modules/m.js"), "junk\n");
    write(&project.join("junk.yxpkg"), "old artifact\n");
    write(&project.join(".DS_Store"), "junk\n");
    let artifact = tmp.path().join("demo.yxpkg");

    // Act
    yxpkg::pack(&project, &artifact).unwrap();
    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();

    // Assert：排除项全部不出现，正常源码保留
    assert!(!dest.join(".git").exists(), "VCS 目录应被排除");
    assert!(!dest.join(".yaoxiang").exists(), "缓存目录应被排除");
    assert!(!dest.join("target").exists(), "构建产物目录应被排除");
    assert!(!dest.join("node_modules").exists(), "node_modules 应被排除");
    assert!(!dest.join("junk.yxpkg").exists(), "旧归档应被排除");
    assert!(!dest.join(".DS_Store").exists(), "系统文件应被排除");
    assert!(dest.join("src/lib.yx").exists(), "正常源码应保留");
}

#[test]
fn test_pack_is_deterministic() {
    // Arrange
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let a = tmp.path().join("a.yxpkg");
    let b = tmp.path().join("b.yxpkg");

    // Act：先按真实 mtime 打包一次；再用显式 mtime 覆盖后重打
    // （规范反模式禁 thread::sleep——用 set_modified 确定性改变 mtime）
    yxpkg::pack(&project, &a).unwrap();
    let fixed = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(42);
    let mut set_any = false;
    for entry in std::fs::read_dir(&project).expect("read project dir") {
        let path = entry.expect("read dir entry").path();
        if path.is_file() {
            let file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open fixture file for mtime change");
            file.set_modified(fixed)
                .unwrap_or_else(|e| panic!("set mtime on {}: {e}", path.display()));
            set_any = true;
        }
    }
    assert!(set_any, "至少应改动一个文件的 mtime");
    yxpkg::pack(&project, &b).unwrap();

    // Assert：mtime 不同，两次打包逐字节一致
    assert_eq!(
        std::fs::read(&a).unwrap(),
        std::fs::read(&b).unwrap(),
        "两次打包应逐字节一致（publish/Release digest 依赖）"
    );
}

#[test]
fn test_tampered_file_rejected() {
    // Arrange：打包 → 解包 → 篡改一个已登记文件
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let artifact = tmp.path().join("demo.yxpkg");
    yxpkg::pack(&project, &artifact).unwrap();
    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();
    write(&dest.join("src/lib.yx"), "pub x = 999 // tampered\n");

    // Act
    let result = yxpkg::verify(&dest);

    // Assert：校验和不匹配（报错携带实际哈希）
    match result {
        Err(PackageError::ChecksumMismatch { actual, .. }) => {
            assert!(!actual.is_empty(), "报错应携带实际校验和");
        }
        other => panic!("expected ChecksumMismatch, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn test_unlisted_file_rejected() {
    // Arrange：打包 → 解包 → 夹带清单外文件
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let artifact = tmp.path().join("demo.yxpkg");
    yxpkg::pack(&project, &artifact).unwrap();
    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();
    write(&dest.join("src/evil.yx"), "smuggled\n");

    // Act
    let result = yxpkg::verify(&dest);

    // Assert：报错指出未登记文件
    match result {
        Err(PackageError::InvalidPackage(msg)) => {
            assert!(msg.contains("src/evil.yx"), "报错应指出未登记文件: {msg}");
        }
        other => panic!("expected InvalidPackage, got {:?}", other.map(|_| ())),
    }
}

/// 手工构造归档（缺清单等畸形包；tar::Builder 在打包侧就拒绝 `..`，
/// 恶意包的字节须手工拼装）
fn build_raw_archive(entries: &[(&str, &str)]) -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let gz = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut builder = tar::Builder::new(gz);
    for (name, content) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        builder
            .append_data(&mut header, name, content.as_bytes())
            .expect("append raw archive entry");
    }
    builder
        .into_inner()
        .expect("finish raw archive builder")
        .finish()
        .expect("finish raw archive")
        .into_inner()
}

/// 手工拼装单个 ustar 条目（512B 头 + 内容 + 补齐）
fn raw_ustar_entry(
    name: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut header = vec![0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{:011o}\0", content.len()).as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    header[148..156].copy_from_slice(b"        "); // chksum 先置空格参与计算
    header[156] = b'0';
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let chksum: u32 = header.iter().map(|b| *b as u32).sum();
    header[148..156].copy_from_slice(format!("{:06o}\0 ", chksum).as_bytes());

    let mut out = header;
    out.extend_from_slice(content);
    let pad = (512 - out.len() % 512) % 512;
    out.extend(std::iter::repeat_n(0u8, pad));
    out
}

#[test]
fn test_missing_sums_rejected() {
    // Arrange：无 SHA256SUMS 清单的归档
    let tmp = tempfile::tempdir().unwrap();
    let raw = build_raw_archive(&[("src/lib.yx", "pub x = 1\n")]);
    let archive = tmp.path().join("no-sums.yxpkg");
    std::fs::write(&archive, &raw).expect("write raw archive");

    // Act
    let err = yxpkg::unpack(&archive, &tmp.path().join("out")).unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidPackage(_)),
        "缺清单应报 InvalidPackage: {err}"
    );
}

#[test]
fn test_path_escape_rejected() {
    // Arrange：手工拼装含 `../` 逃逸条目的恶意归档
    let tmp = tempfile::tempdir().unwrap();
    let sums = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  ../evil.txt\n";
    let mut raw = raw_ustar_entry("../evil.txt", b"escape\n");
    raw.extend(raw_ustar_entry(yxpkg::SUMS_FILE, sums.as_bytes()));
    raw.extend(vec![0u8; 1024]); // 归档结束块
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&raw).expect("gzip raw archive");
    let archive = tmp.path().join("escape.yxpkg");
    std::fs::write(&archive, gz.finish().expect("finish gzip")).expect("write archive");

    // Act
    let out = tmp.path().join("out");
    let err = yxpkg::unpack(&archive, &out).unwrap_err();

    // Assert：报 InvalidPackage 且逃逸目标绝不能被创建
    assert!(
        matches!(err, PackageError::InvalidPackage(_)),
        "路径逃逸应报 InvalidPackage: {err}"
    );
    assert!(!tmp.path().join("evil.txt").exists(), "逃逸目标不得被创建");
}

#[test]
fn test_size_limit_enforced() {
    // Arrange：内容总量超过 20 MiB（决议 7）
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let big = vec![0u8; yxpkg::MAX_PACKAGE_BYTES as usize + 1];
    std::fs::write(project.join("big.bin"), &big).expect("write oversized fixture");

    // Act
    let err = yxpkg::pack(&project, &tmp.path().join("big.yxpkg")).unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::PackageTooLarge(_)),
        "超限应报 PackageTooLarge: {err}"
    );
}

#[test]
fn test_artifact_name_format() {
    // Act / Assert：RFC 包格式 `{name}-{version}.yxpkg`
    assert_eq!(yxpkg::artifact_name("demo", "1.2.3"), "demo-1.2.3.yxpkg");
}
