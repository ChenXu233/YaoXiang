//! CLI 子命令端到端测试（子进程级）
//!
//! 通过 `CARGO_BIN_EXE_yaoxiang-rs` 调用编译出来的二进制，
//! 验证用户真实路径：命令行参数解析、退出码、stdout/stderr。
//!
//! 规范来源：
//! - RFC-014: 包管理系统 (init/new/add/rm/install/list/update)
//! - docs/src/design/language-spec.md: 执行与编译章节
//! - docs/src/dev/test-specification.md §集成测试规范 规则 9.1: E2E 三条路径
//! - docs/superpowers/specs/2026-07-26-issue231-bytecode-run-magic-probe-design.md: 魔数探针
//! 覆盖命令：run / build / check / init
//! 函数级 API 契约见 `cli.rs`，本文件只验证"二进制入口"行为。

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// 编译产物路径（cargo 在测试构建时注入）
fn yx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang-rs"))
}

/// 在临时目录写一个 .yx 源文件
fn write_yx(
    dir: &std::path::Path,
    name: &str,
    content: &str,
) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {name}: {e}"));
    path
}

/// 跑一个子进程，捕获 stdout/stderr，返回 (退出码, stdout, stderr)
fn run_yx(
    args: &[&str],
    cwd: &std::path::Path,
) -> (i32, String, String) {
    let output = Command::new(yx_bin())
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn yaoxiang: {e}"));
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// run 命令 — 退出码契约：成功 0，编译/运行错误 1
// 规范来源：language-spec.md 执行章节

#[test]
fn test_e2e_run_valid_program_exits_zero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "ok.yx", "main = { print(42) }");

    // Act
    let (code, stdout, _stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "valid program should exit 0");
    assert!(
        stdout.contains("42"),
        "stdout should contain program output, got: {stdout:?}"
    );
}

#[test]
fn test_e2e_run_compile_error_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = ");

    // Act
    let (code, _stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_ne!(code, 0, "compile error should exit non-zero");
    assert!(
        !stderr.is_empty(),
        "stderr should have error diagnostics on compile error"
    );
}

#[test]
fn test_e2e_run_nonexistent_file_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();

    // Act
    let (code, _stdout, _stderr) = run_yx(&["run", "/nonexistent/path.yx"], tmp.path());

    // Assert
    assert_ne!(code, 0, "missing file should exit non-zero");
}

// run 命令 + .42 字节码文件
// 规范来源：design spec — 魔数探针判定字节码 vs 源码 (issue #231)

#[test]
fn test_e2e_run_42_file() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "prog.yx", "main = { print(42) }");
    let out = tmp.path().join("prog.42");

    // 先 build
    let (code, _, _) = run_yx(
        &["build", src.to_str().unwrap(), "-o", out.to_str().unwrap()],
        tmp.path(),
    );
    assert_eq!(code, 0, "build should succeed before running .42 file");

    // Act: run .42 文件
    let (code, stdout, stderr) = run_yx(&["run", out.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "run .42 should exit 0; stderr: {stderr:?}");
    assert!(
        stdout.contains("42"),
        "stdout should contain program output, got: {stdout:?}"
    );
}

#[test]
fn test_e2e_run_bytecode_by_content_not_extension() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "prog.yx", "main = { print(1) }");
    let out_42 = tmp.path().join("prog.42");
    let out_bin = tmp.path().join("prog.bin");

    // 先 build 为 .42，再 cp 为 .bin（无 .42 扩展名）
    let (code, _, _) = run_yx(
        &[
            "build",
            src.to_str().unwrap(),
            "-o",
            out_42.to_str().unwrap(),
        ],
        tmp.path(),
    );
    assert_eq!(
        code, 0,
        "build should succeed before testing content-based detection"
    );
    std::fs::copy(&out_42, &out_bin).expect("copy .42 to .bin");

    // Act: run .bin 文件（无 .42 扩展名）
    let (code, stdout, stderr) = run_yx(&["run", out_bin.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(
        code, 0,
        "run .bin (with YXBC magic) should exit 0; stderr: {stderr:?}"
    );
    assert!(
        stdout.contains("1"),
        "stdout should contain program output, got: {stdout:?}"
    );
}

#[test]
fn test_e2e_run_non_42_binary_file_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let not_bytecode = tmp.path().join("not_bytecode.bin");
    std::fs::write(&not_bytecode, b"garbage data that is not bytecode").expect("write garbage");

    // Act
    let (code, _stdout, stderr) = run_yx(&["run", not_bytecode.to_str().unwrap()], tmp.path());

    // Assert
    assert_ne!(code, 0, "non-bytecode binary should exit non-zero");
    assert!(
        !stderr.is_empty(),
        "stderr should have error diagnostics: {stderr:?}"
    );
}

// build 命令 — 退出码契约 + 输出文件存在性
// 规范来源：language-spec.md 编译章节

#[test]
fn test_e2e_build_valid_source_produces_bytecode_file() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "prog.yx", "main = { print(1) }");
    let out = tmp.path().join("prog.42");

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["build", src.to_str().unwrap(), "-o", out.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_eq!(code, 0, "build on valid source should exit 0");
    assert!(out.exists(), "bytecode output file should exist");
    assert!(
        out.metadata().unwrap().len() > 0,
        "bytecode file should not be empty"
    );
}

#[test]
fn test_e2e_build_compile_error_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = ");
    let out = tmp.path().join("bad.42");

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["build", src.to_str().unwrap(), "-o", out.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_ne!(code, 0, "build on compile error should exit non-zero");
    assert!(
        !out.exists(),
        "bytecode file should not be produced on compile error"
    );
}

// check 命令 — 退出码契约：无错 0，有错 1，无 .yx 文件 2
// 规范来源：language-spec.md 类型检查章节

#[test]
fn test_e2e_check_valid_file_exits_zero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "ok.yx", "main = { x = 1 }");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "check on valid file should exit 0");
}

#[test]
fn test_e2e_check_type_error_exits_one() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = \"not an int\"");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 1, "check on type error should exit 1");
    assert!(
        !stderr.is_empty(),
        "stderr should contain diagnostics on type error"
    );
}

#[test]
fn test_e2e_check_nonexistent_file_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();

    // Act
    let (code, _stdout, _stderr) = run_yx(&["check", "/nonexistent/path.yx"], tmp.path());

    // Assert
    assert_ne!(code, 0, "check on missing file should exit non-zero");
}

// check 命令 — 警告通道（#321 M2 / 定案 B）
// 验收契约：触发 W 码的源文件编译成功（exit 0）且输出 `warning[W####]`；
// `--deny-warnings` 将警告升级为失败（exit 1）。

#[test]
fn test_e2e_check_unused_private_fn_warns_but_exits_zero() {
    // Arrange: 私有函数无任何引用 → W1001（pub 函数是对外接口，永不报）
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "dead_fn.yx",
        "dead_fn = (x: Int) => x\nmain = { x = 1 }",
    );

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "存在警告不构成错误，check 应 exit 0");
    assert!(
        stderr.contains("warning [W1001]"),
        "stderr 应含 warning[W1001] 前缀渲染，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_unused_import_warns_but_exits_zero() {
    // Arrange: 整模块导入且未被引用 → W1003
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "dead_import.yx", "use std.io\nmain = { x = 1 }");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "存在警告不构成错误，check 应 exit 0");
    assert!(
        stderr.contains("warning [W1003]"),
        "stderr 应含 warning[W1003] 前缀渲染，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_deny_warnings_exits_one() {
    // Arrange: 同一警告文件，--deny-warnings 升级为失败（CI 严格模式）
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "dead_fn.yx",
        "dead_fn = (x: Int) => x\nmain = { x = 1 }",
    );

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["check", "--deny-warnings", src.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_eq!(code, 1, "--deny-warnings 存在警告时应以非零码退出");
}

#[test]
fn test_e2e_check_deny_warnings_clean_file_exits_zero() {
    // Arrange: 无警告文件在 --deny-warnings 下仍应通过
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "clean.yx", "main = { x = 1 }");

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["check", "--deny-warnings", src.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_eq!(code, 0, "无警告文件在 --deny-warnings 下应 exit 0");
}

// init 命令 — 目录结构契约 + 退出码
// 规范来源：RFC-014 包管理系统 — 项目初始化

#[test]
fn test_e2e_init_binary_project_creates_expected_files() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my_app");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["init", "my_app"], tmp.path());

    // Assert
    assert_eq!(code, 0, "init should exit 0 on success");
    assert!(
        project_dir.join("src/main.yx").exists(),
        "should have src/main.yx"
    );
    assert!(
        project_dir.join("yaoxiang.toml").exists(),
        "should have yaoxiang.toml"
    );
    assert!(
        project_dir.join("yaoxiang.lock").exists(),
        "should have yaoxiang.lock"
    );
    assert!(
        project_dir.join(".gitignore").exists(),
        "should have .gitignore"
    );
    assert!(
        !project_dir.join("src/lib.yx").exists(),
        "binary project should not have src/lib.yx"
    );
}

#[test]
fn test_e2e_init_library_project_creates_lib_yx() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my_lib");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["init", "my_lib", "--lib"], tmp.path());

    // Assert
    assert_eq!(code, 0, "init --lib should exit 0");
    assert!(
        project_dir.join("src/lib.yx").exists(),
        "library should have src/lib.yx"
    );
    assert!(
        !project_dir.join("src/main.yx").exists(),
        "library project should not have src/main.yx"
    );
}

#[test]
fn test_e2e_init_on_existing_directory_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let existing = tmp.path().join("dup");
    std::fs::create_dir_all(&existing).unwrap();
    std::fs::write(existing.join("placeholder.txt"), "").unwrap();

    // Act
    let (code, _stdout, stderr) = run_yx(&["init", "dup"], tmp.path());

    // Assert
    assert_ne!(code, 0, "init on existing dir should exit non-zero");
    assert!(!stderr.is_empty(), "stderr should explain why init failed");
}

// RFC-029f：编译目标角色与导入面语义（check 命令端到端）

/// 写一个最小的 yaoxiang.toml（[package] 头）
fn write_manifest(
    dir: &std::path::Path,
    name: &str,
    extra: &str,
) {
    std::fs::write(
        dir.join("yaoxiang.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n{extra}"),
    )
    .unwrap();
}

#[test]
fn test_e2e_check_bin_role_reports_unused_pub_fn() {
    // Arrange: 无声明面项目——含 main 且无人 use 的入口文件推断为 Bin，
    // 其未使用 pub 可报（RFC-029f 角色语义；pub 仍豁免时此测试红）
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "pub dead_api = (x: Int) => x\nmain = { x = 1 }",
    );

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "警告不构成错误，check 应 exit 0");
    assert!(
        stderr.contains("W1001"),
        "Bin 角色未使用 pub 应报 W1001，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_lib_file_unused_pub_exempt() {
    // Arrange: [lib] 声明的库文件是对外接口——未使用 pub 豁免（定案 B 语义）
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "[lib]\npath = \"lib.yx\"\n");
    let _main = write_yx(tmp.path(), "main.yx", "use lib\nmain = { lib.lib_fn(1) }");
    let lib = write_yx(
        tmp.path(),
        "lib.yx",
        "pub lib_fn = (x: Int) => x\npub lib_dead = (y: Int) => y",
    );

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", lib.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("W1001"),
        "Lib 角色 pub 是对外接口不应报 W1001，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_tests_dir_exempt_from_dead_code() {
    // Arrange: tests/ 目录文件是 Test 角色——不参与死代码判定
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let _main = write_yx(tmp.path(), "main.yx", "main = { x = 1 }");
    let test_file = tmp.path().join("tests");
    std::fs::create_dir(&test_file).unwrap();
    let test_src = test_file.join("util_test.yx");
    std::fs::write(&test_src, "pub helper = (x: Int) => x\nmain = { x = 1 }").unwrap();

    // Act: check 目录——tests/ 下的文件作为入口被检查
    let (code, _stdout, stderr) = run_yx(&["check", tmp.path().to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("W1001"),
        "Test 角色文件不应报死代码警告，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_vendor_import_surface_allows_exported() {
    // Arrange: vendor 依赖声明 [exports] 只露 src/dep.yx——
    // use dep（导出面内）应正常解析
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let dep = tmp.path().join(".yaoxiang/vendor/dep-0.1.0");
    std::fs::create_dir_all(dep.join("src")).unwrap();
    write_manifest(&dep, "dep", "[exports]\n\".\" = \"src/dep.yx\"\n");
    std::fs::write(dep.join("src/dep.yx"), "pub api = (x: Int) => x").unwrap();
    std::fs::write(dep.join("src/hidden.yx"), "pub secret = (x: Int) => x").unwrap();
    let src = write_yx(tmp.path(), "main.yx", "use dep\nmain = { dep.api(1) }");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "导出面内的 use 应正常解析，实际: {stderr:?}");
}

#[test]
fn test_e2e_check_vendor_import_surface_blocks_hidden() {
    // Arrange: use dep.hidden 不在依赖包导出面内 → 模块未找到（越界不可见）
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let dep = tmp.path().join(".yaoxiang/vendor/dep-0.1.0");
    std::fs::create_dir_all(dep.join("src")).unwrap();
    write_manifest(&dep, "dep", "[exports]\n\".\" = \"src/dep.yx\"\n");
    std::fs::write(dep.join("src/dep.yx"), "pub api = (x: Int) => x").unwrap();
    std::fs::write(dep.join("src/hidden.yx"), "pub secret = (x: Int) => x").unwrap();
    let src = write_yx(tmp.path(), "main.yx", "use dep.hidden\nmain = { x = 1 }");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_ne!(code, 0, "越界 use 应失败");
    assert!(
        stderr.contains("E5001"),
        "越界 use 应报既有 module_not_found（E5001），实际: {stderr:?}"
    );
}

// RFC-029f Phase 2：Internal pub 收紧 + [tool.test] patterns 级 Test 判定

#[test]
fn test_e2e_check_internal_unused_pub_reports() {
    // Arrange: internal.yx 无人 use、无 main → Internal 角色；其 pub 无引用
    // （Phase 2 收紧：包内 use 图不可达即报）
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let _main = write_yx(tmp.path(), "main.yx", "main = { x = 1 }");
    let internal = write_yx(
        tmp.path(),
        "internal.yx",
        "pub orphan = (x: Int) => x\nconst_used = 1",
    );

    // Act: 单文件入口检查 internal.yx
    let (code, _stdout, stderr) = run_yx(&["check", internal.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        stderr.contains("W1001"),
        "Internal 角色 unreferenced pub 应报 W1001，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_internal_pub_used_elsewhere_alive() {
    // Arrange: 同样的 pub 定义，但被其他文件具名导入并调用——
    // 引用池来自全项目扫描，使用方在发现集之外也能救活
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let _main = write_yx(
        tmp.path(),
        "main.yx",
        "use internal.{util}\nmain = { util(1) }",
    );
    let internal = write_yx(tmp.path(), "internal.yx", "pub util = (x: Int) => x");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", internal.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("W1001"),
        "被包内其他文件使用的 Internal pub 不应报，实际: {stderr:?}"
    );
}

#[test]
fn test_e2e_check_custom_test_patterns() {
    // Arrange: [tool.test] 自定义 patterns 指向 checks/ 目录——
    // 该目录下文件按 Test 角色处理（不参与死代码判定）
    let tmp = TempDir::new().unwrap();
    write_manifest(
        tmp.path(),
        "app",
        "[tool.test]\npatterns = [\"checks/**/*.yx\"]\n",
    );
    let _main = write_yx(tmp.path(), "main.yx", "main = { x = 1 }");
    let checks = tmp.path().join("checks");
    std::fs::create_dir(&checks).unwrap();
    let guard = checks.join("guard.yx");
    std::fs::write(&guard, "pub ensure = (x: Int) => x\nmain = { x = 1 }").unwrap();

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", guard.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("W1001"),
        "自定义 patterns 命中的文件应按 Test 角色豁免，实际: {stderr:?}"
    );
}
