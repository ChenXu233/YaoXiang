//! 解包与校验辅助测试（RFC-037：版本目录 = 发行包解压根目录）
//!
//! 另含 HTTP 面的环境代理探针：dl.rs 的 `http_agent` 与 src/std/net.rs 的
//! `ureq_agent` 同链显式禁用环境代理（对位 ureq 2 未启用 proxy-from-env 的
//! 不读默认，#489 迁移对账）。代理变量是进程级的，agent 每次请求都重新
//! 构造——进程内 set_var 会污染同进程并行运行的其它测试，探针走「重新拉起
//! 本测试可执行文件、只在子进程注入环境」的形态（先例：
//! `src/package/source/tests/git.rs` 的 GIT_DIR 重拉形态）。

use sha2::Digest;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::process::Command;
use tempfile::TempDir;

use crate::dl::{entry_path_is_safe, get_text, strip_first, verify_sha256};

/// 探针环境变量：标记子进程为代理探针形态。
const PROXY_PROBE_ENV: &str = "YX_DL_PROXY_PROBE";

/// 环境代理变量全集：ureq `Proxy::try_from_env` 的读取面（ALL_PROXY/HTTPS_PROXY/
/// HTTP_PROXY 及小写形态）+ NO_PROXY 旁路面。
const PROXY_ENV_VARS: [&str; 8] = [
    "ALL_PROXY",
    "all_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// 重新拉起本测试可执行文件（只跑 `probe_test` 一个用例），先清掉全部代理变量，
/// 再注入 `http_proxy` 指向的 HTTP_PROXY。
fn relaunch_self_with_http_proxy(
    http_proxy: &str,
    probe_test: &str,
) -> std::process::Output {
    let exe = std::env::current_exe().expect("测试可执行文件路径");
    let mut cmd = Command::new(exe);
    cmd.arg("--exact").arg(probe_test).env(PROXY_PROBE_ENV, "1");
    for var in PROXY_ENV_VARS {
        cmd.env_remove(var);
    }
    cmd.env("HTTP_PROXY", http_proxy);
    cmd.output().expect("重新拉起测试进程")
}

/// Assert: 探针必须真的跑到了本用例并通过，否则探针形同虚设。
fn assert_probe_child_passed(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "探针进程应通过其 1 个用例；stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// 起一个只服务单次请求的本地 HTTP 服务器（不依赖外部网络）。
/// 返回 (url, 服务器线程句柄)——不解析请求内容，应答写完即结束。
fn serve_once(response: &str) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().unwrap().port();
    let response = response.to_string();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one connection");
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    (format!("http://127.0.0.1:{port}/probe"), handle)
}

#[test]
fn test_dl_http_agent_ignores_http_proxy_env_var() {
    // Arrange：HTTP_PROXY 注入必失败的假代理地址（127.0.0.1:1 无监听）。
    // 契约：dl.rs http_agent 显式禁用环境代理——若读了环境代理，原先直连成功的
    // 下载/版本查询会改道假代理而必败（#489 迁移对账）。
    if std::env::var_os(PROXY_PROBE_ENV).is_none() {
        // Act：重新拉起自身，只在子进程注入假代理
        let output = relaunch_self_with_http_proxy(
            "http://127.0.0.1:1",
            "tests::dl::test_dl_http_agent_ignores_http_proxy_env_var",
        );

        // Assert
        assert_probe_child_passed(&output);
        return;
    }

    // 子进程形态：本地真目标已在监听，get_text 必须直连它成功
    let (url, server) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");

    // Act
    let text = get_text(&url)
        .expect("HTTP_PROXY 指向无监听地址时 dl.get_text 仍应直连本地目标（不读环境代理）");

    // Assert
    assert_eq!(text, "ok");
    server.join().expect("server thread");
}

#[test]
fn test_strip_first_removes_top_level_package_dir() {
    // Arrange: 发行包内路径 yaoxiang-0.7.14-x86_64-unknown-linux-gnu/bin/yaoxiang-rs
    let entry = std::path::Path::new("yaoxiang-0.7.14-x86_64-unknown-linux-gnu/bin/yaoxiang-rs");
    let dest = TempDir::new().unwrap();

    // Act
    let stripped = strip_first(entry, dest.path()).unwrap();

    // Assert
    assert_eq!(
        stripped,
        dest.path().join("bin/yaoxiang-rs"),
        "first component must be stripped so versions/<ver>/ is the extraction root"
    );
}

#[test]
fn test_strip_first_rejects_toplevel_loose_entry() {
    // Arrange: 没有顶层目录的散文件
    let entry = std::path::Path::new("README.md");
    let dest = TempDir::new().unwrap();

    // Act
    let stripped = strip_first(entry, dest.path());

    // Assert: 安全起见拒绝，不写出归档根之外的路径
    assert!(
        stripped.is_none(),
        "entry without top-level dir must be rejected"
    );
}

#[test]
fn test_verify_sha256_accepts_matching_digest_file_content() {
    // Arrange: 内容 "hex  filename" 格式（sha256sum 输出）
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.tar.gz");
    fs::write(&file, b"payload").unwrap();
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"payload");
    // 不用输出类型的 LowerHex——sha2 0.11 已无此 trait，逐字节十六进制化
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    // Act
    let result = verify_sha256(&file, &format!("{digest}  a.tar.gz"));

    // Assert
    assert!(result.is_ok(), "matching digest must pass, got {result:?}");
}

#[test]
fn test_verify_sha256_rejects_tampered_content() {
    // Arrange
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.tar.gz");
    fs::write(&file, b"payload").unwrap();

    // Act
    let result = verify_sha256(&file, "deadbeef  a.tar.gz");

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::ChecksumMismatch { .. })),
        "tampered content must fail checksum, got {result:?}"
    );
}

#[test]
fn test_entry_path_is_safe_rejects_parent_traversal() {
    // Act & Assert: 带 .. 的条目不安全（纵深防御，zip 侧由 enclosed_name 保护）
    assert!(
        !entry_path_is_safe(std::path::Path::new("pkg/../../evil.yx")),
        "parent traversal entry must be flagged unsafe"
    );
    assert!(
        entry_path_is_safe(std::path::Path::new("pkg/bin/yaoxiang-rs")),
        "normal entry must be considered safe"
    );
}

/// 把未压缩 tar 字节流 gzip 后写到 `archive`
fn write_gz_tar(
    archive: &std::path::Path,
    tar_bytes: Vec<u8>,
) {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, &tar_bytes).unwrap();
    fs::write(archive, gz.finish().unwrap()).unwrap();
}

#[test]
fn test_unpack_rejects_tar_symlink_entry() {
    // Arrange: 构造含符号链接条目的归档（链接目标是写穿解包目录的向量；
    // 发行包由 package-dist.sh 产出，永远不含链接条目）
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(0);
    header.set_mode(0o777);
    header.set_entry_type(tar::EntryType::Symlink);
    header
        .set_link_name(std::path::Path::new("../../outside"))
        .unwrap();
    header.set_cksum();
    builder
        .append_data(&mut header, "pkg/evil", std::io::empty())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());

    // Act
    let result = crate::dl::unpack(&archive, "linux", &dir.path().join("out"));

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::Message(ref m)) if m.contains("symlink")),
        "symlink entry must be rejected, got {result:?}"
    );
}

#[test]
fn test_unpack_rejects_tar_hardlink_entry() {
    // Arrange: 构造含硬链接条目的归档（hard link 可把包外既有文件链入解包树）
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(0);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Link);
    header
        .set_link_name(std::path::Path::new("pkg/real.bin"))
        .unwrap();
    header.set_cksum();
    builder
        .append_data(&mut header, "pkg/alias", std::io::empty())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());

    // Act
    let result = crate::dl::unpack(&archive, "linux", &dir.path().join("out"));

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::Message(ref m)) if m.contains("hardlink")),
        "hardlink entry must be rejected, got {result:?}"
    );
}

#[test]
fn test_unpack_tar_regular_files_still_extract() {
    // Arrange: 正常归档（目录 + 普通文件），链接拒绝不得误伤常规解包路径
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut dir_header = tar::Header::new_gnu();
    dir_header.set_size(0);
    dir_header.set_mode(0o755);
    dir_header.set_entry_type(tar::EntryType::Directory);
    dir_header.set_cksum();
    builder
        .append_data(&mut dir_header, "pkg/bin", std::io::empty())
        .unwrap();
    let mut file_header = tar::Header::new_gnu();
    file_header.set_size(5);
    file_header.set_mode(0o644);
    file_header.set_entry_type(tar::EntryType::Regular);
    file_header.set_cksum();
    builder
        .append_data(&mut file_header, "pkg/bin/yx", b"hello".as_slice())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());
    let out = dir.path().join("out");

    // Act
    crate::dl::unpack(&archive, "linux", &out).unwrap();

    // Assert
    assert_eq!(
        fs::read(out.join("bin/yx")).unwrap(),
        b"hello",
        "regular file must extract with content intact under stripped path"
    );
}
/// 按 zip 归档写出条目（entries: 名字 → 内容；deflate 压缩，覆盖解压读取路径）
fn write_zip(
    archive: &std::path::Path,
    entries: &[(&str, &[u8])],
) {
    use std::io::Write;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in entries {
        w.start_file(*name, opts).unwrap();
        w.write_all(content).unwrap();
    }
    fs::write(archive, w.finish().unwrap().into_inner()).unwrap();
}

#[test]
fn test_unpack_zip_extracts_deflated_entry() {
    // Arrange: Windows 发行包形态（zip，deflate 压缩的普通文件条目）
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.zip");
    write_zip(&archive, &[("pkg/bin/yx.exe", b"hello")]);
    let out = dir.path().join("out");

    // Act
    crate::dl::unpack(&archive, "windows", &out).unwrap();

    // Assert
    assert_eq!(
        fs::read(out.join("bin/yx.exe")).unwrap(),
        b"hello",
        "deflated zip entry must extract with content intact under stripped path"
    );
}

#[test]
fn test_unpack_zip_rejects_parent_traversal_entry() {
    // Arrange: 正/反斜杠两种分隔符的 .. 条目——写穿解包目录的经典向量
    for name in ["pkg/../evil.txt", "pkg\\..\\..\\evil2.txt"] {
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("a.zip");
        write_zip(&archive, &[(name, b"evil")]);
        let out = dir.path().join("out");

        // Act
        let result = crate::dl::unpack(&archive, "windows", &out);

        // Assert: 拒绝，且文件不得落进解包根的上一级
        assert!(
            result.is_err(),
            "traversal entry {name:?} must be rejected, got {result:?}"
        );
        assert!(
            !dir.path().join("evil.txt").exists(),
            "traversal entry {name:?} must not write outside the extraction root"
        );
    }
}
