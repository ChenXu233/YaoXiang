//! `.yxpkg` 打包/解包测试（RFC-014a 决议 3/7）

use std::io::Write as _;
use std::path::Path;

use crate::package::error::PackageError;
use crate::package::yxpkg;

fn write(
    path: &Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
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
fn round_trip_preserves_content() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    let artifact = tmp.path().join("demo-1.0.0.yxpkg");
    let total = yxpkg::pack(&project, &artifact).unwrap();
    assert!(artifact.is_file());
    assert!(total > 0);

    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();

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
        "coreutils 风格双空格 + / 分隔路径"
    );
}

#[test]
fn pack_excludes_vcs_build_and_artifacts() {
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
    yxpkg::pack(&project, &artifact).unwrap();

    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();
    assert!(!dest.join(".git").exists());
    assert!(!dest.join(".yaoxiang").exists());
    assert!(!dest.join("target").exists());
    assert!(!dest.join("node_modules").exists());
    assert!(!dest.join("junk.yxpkg").exists());
    assert!(!dest.join(".DS_Store").exists());
    assert!(dest.join("src/lib.yx").exists());
}

#[test]
fn pack_is_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    let a = tmp.path().join("a.yxpkg");
    let b = tmp.path().join("b.yxpkg");
    yxpkg::pack(&project, &a).unwrap();
    // 同一目录再打一次（mtime 变化也不影响）
    std::thread::sleep(std::time::Duration::from_millis(20));
    yxpkg::pack(&project, &b).unwrap();

    assert_eq!(
        std::fs::read(&a).unwrap(),
        std::fs::read(&b).unwrap(),
        "两次打包应逐字节一致（publish/Release digest 依赖）"
    );
}

#[test]
fn tampered_file_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    let artifact = tmp.path().join("demo.yxpkg");
    yxpkg::pack(&project, &artifact).unwrap();

    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();
    write(&dest.join("src/lib.yx"), "pub x = 999 // tampered\n");

    match yxpkg::verify(&dest) {
        Err(PackageError::ChecksumMismatch { actual, .. }) => {
            assert!(!actual.is_empty());
        }
        other => panic!("expected ChecksumMismatch, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn unlisted_file_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);

    let artifact = tmp.path().join("demo.yxpkg");
    yxpkg::pack(&project, &artifact).unwrap();

    let dest = tmp.path().join("out");
    yxpkg::unpack(&artifact, &dest).unwrap();
    write(&dest.join("src/evil.yx"), "smuggled\n");

    match yxpkg::verify(&dest) {
        Err(PackageError::InvalidPackage(msg)) => {
            assert!(msg.contains("src/evil.yx"), "报错应指出未登记文件: {msg}");
        }
        other => panic!("expected InvalidPackage, got {:?}", other.map(|_| ())),
    }
}

/// 手工构造归档（测试路径逃逸 / 缺清单等恶意或畸形包）
///
/// tar::Builder 在打包侧就拒绝 `..` 条目，恶意包的字节须手工拼装。
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
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap().into_inner()
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
fn missing_sums_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let raw = build_raw_archive(&[("src/lib.yx", "pub x = 1\n")]);
    let archive = tmp.path().join("no-sums.yxpkg");
    std::fs::write(&archive, &raw).unwrap();

    let err = yxpkg::unpack(&archive, &tmp.path().join("out")).unwrap_err();
    assert!(matches!(err, PackageError::InvalidPackage(_)), "got: {err}");
}

#[test]
fn path_escape_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let sums = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  ../evil.txt\n";
    let mut raw = raw_ustar_entry("../evil.txt", b"escape\n");
    raw.extend(raw_ustar_entry(yxpkg::SUMS_FILE, sums.as_bytes()));
    raw.extend(vec![0u8; 1024]); // 归档结束块
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&raw).unwrap();
    let archive = tmp.path().join("escape.yxpkg");
    std::fs::write(&archive, gz.finish().unwrap()).unwrap();

    let out = tmp.path().join("out");
    let err = yxpkg::unpack(&archive, &out).unwrap_err();
    assert!(matches!(err, PackageError::InvalidPackage(_)), "got: {err}");
    // 逃逸目标绝不能被创建
    assert!(!tmp.path().join("evil.txt").exists());
}

#[test]
fn size_limit_enforced() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    sample_project(&project);
    let big = vec![0u8; yxpkg::MAX_PACKAGE_BYTES as usize + 1];
    std::fs::write(project.join("big.bin"), &big).unwrap();

    let err = yxpkg::pack(&project, &tmp.path().join("big.yxpkg")).unwrap_err();
    assert!(
        matches!(err, PackageError::PackageTooLarge(_)),
        "got: {err}"
    );
}

#[test]
fn artifact_name_format() {
    assert_eq!(yxpkg::artifact_name("demo", "1.2.3"), "demo-1.2.3.yxpkg");
}
