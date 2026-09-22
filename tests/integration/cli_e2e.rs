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
///
/// T4 入口语义：Script 模式下 `main` 不再隐式执行，想跑得写 `main()`。
/// 本 helper 经 `fixture::with_main_invoked` 适配：只对**定义了 main 但未调用**
/// 的夹具追加调用，使它们真正执行；未定义 main 的源（如故意写坏的语法用例）
/// 原样写入，不会因追加而掩盖错误。
fn write_yx(
    dir: &std::path::Path,
    name: &str,
    content: &str,
) -> PathBuf {
    let content = crate::fixture::with_main_invoked_in(dir, content);
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {name}: {e}"));
    path
}

/// 跑一个子进程，捕获 stdout/stderr，返回 (退出码, stdout, stderr)
fn run_yx(
    args: &[&str],
    cwd: &std::path::Path,
) -> (i32, String, String) {
    run_yx_env(args, cwd, &[])
}

/// 同上，但可注入环境变量（用于验证 `YAOXIANG_LANG` 语言选择）
fn run_yx_env(
    args: &[&str],
    cwd: &std::path::Path,
    envs: &[(&str, &str)],
) -> (i32, String, String) {
    let mut cmd = Command::new(yx_bin());
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd
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
    let src = write_yx(tmp.path(), "ok.yx", "main = () => { print(42) }");

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
    let src = write_yx(tmp.path(), "prog.yx", "main = () => { print(42) }");
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
    let src = write_yx(tmp.path(), "prog.yx", "main = () => { print(1) }");
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
    let src = write_yx(tmp.path(), "prog.yx", "main = () => { print(1) }");
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
    let src = write_yx(tmp.path(), "ok.yx", "main = () => { x = 1 }");

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
        "dead_fn = (x: Int) => x\nmain = () => { x = 1 }",
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
    let src = write_yx(
        tmp.path(),
        "dead_import.yx",
        "use std.io\nmain = () => { x = 1 }",
    );

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
        "dead_fn = (x: Int) => x\nmain = () => { x = 1 }",
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
    let src = write_yx(tmp.path(), "clean.yx", "main = () => { x = 1 }");

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
        "pub dead_api = (x: Int) => x\nmain = () => { x = 1 }",
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
    let _main = write_yx(
        tmp.path(),
        "main.yx",
        "use lib\nmain = () => { lib.lib_fn(1) }",
    );
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
    let _main = write_yx(tmp.path(), "main.yx", "main = () => { x = 1 }");
    let test_file = tmp.path().join("tests");
    std::fs::create_dir(&test_file).unwrap();
    let test_src = test_file.join("util_test.yx");
    std::fs::write(
        &test_src,
        "pub helper = (x: Int) => x\nmain = () => { x = 1 }",
    )
    .unwrap();

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
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use dep\nmain = () => { dep.api(1) }",
    );

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
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use dep.hidden\nmain = () => { x = 1 }",
    );

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
    let _main = write_yx(tmp.path(), "main.yx", "main = () => { x = 1 }");
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
        "use internal.{util}\nmain = () => { util(1) }",
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
    let _main = write_yx(tmp.path(), "main.yx", "main = () => { x = 1 }");
    let checks = tmp.path().join("checks");
    std::fs::create_dir(&checks).unwrap();
    let guard = checks.join("guard.yx");
    std::fs::write(&guard, "pub ensure = (x: Int) => x\nmain = () => { x = 1 }").unwrap();

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", guard.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0);
    assert!(
        !stderr.contains("W1001"),
        "自定义 patterns 命中的文件应按 Test 角色豁免，实际: {stderr:?}"
    );
}

// dump 命令 — 字节码 ↔ 源码行对照（RFC-034 阶段零：ip→span 映射可见）

#[test]
fn test_e2e_dump_annotates_instructions_with_source_line() {
    // Arrange: 单文件模式 module.source_files 为空，translator 固定 file_id 0，
    // 文件名退化为 <src>；io.println(1) 落在第 4 行第 15 列。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "loc.yx",
        "use std.io\n\nmain = () => {\n    io.println(1)\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["dump", src.to_str().unwrap()], tmp.path());

    // Assert: dump 走 tracing（stdout sink）
    assert_eq!(code, 0, "dump should exit 0; stderr: {stderr:?}");
    assert!(
        stdout.contains("CallNative"),
        "dump should list instructions; stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("; <src>:4:15"),
        "dump should annotate instructions with ip→span; stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_dump_bytecode_file_carries_source_path() {
    // Arrange: .42 携带 debug section 时，dump 应解析出真实文件路径而非 <src>
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "loc.yx",
        "use std.io\n\nmain = () => {\n    io.println(1)\n}\n",
    );
    let out = tmp.path().join("loc.42");

    // Act
    let (build_code, _o, build_err) = run_yx(
        &[
            "build",
            src.to_str().unwrap(),
            "--debug-info",
            "-o",
            out.to_str().unwrap(),
        ],
        tmp.path(),
    );
    assert_eq!(
        build_code, 0,
        "build --debug-info should exit 0; {build_err:?}"
    );

    let (code, stdout, stderr) = run_yx(&["dump", out.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "dump .42 should exit 0; stderr: {stderr:?}");
    assert!(
        stdout.contains("; ") && stdout.contains("loc.yx:4:15"),
        "dump .42 should resolve real source path; stdout: {stdout:?}"
    );
}

// T4：入口点语义（RFC-029f 角色驱动）

/// Bin 角色（有 manifest）缺 main → E3020 编译错误。
///
/// 此前 `find_entry_point` 找不到 main 就静默返回 0——执行函数表里任意
/// 第一个函数（用户以为在跑 main，实际跑了 helper）。属 #271 静默错误族。
#[test]
fn test_e2e_bin_missing_main_reports_e3020() {
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let src = write_yx(tmp.path(), "main.yx", "helper: () -> Void = { }\n");

    let (code, _stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    assert_ne!(code, 0, "Bin 缺 main 应编译失败");
    assert!(
        stderr.contains("E3020"),
        "应报 E3020（缺入口），实际 stderr: {stderr:?}"
    );
}

/// Bin 角色 main 是值绑定而非函数 → E3021 编译错误。
///
/// 此前 `main: Int = 5` 被当作入口编译成零参访问器函数，静默“成功”且无输出。
#[test]
fn test_e2e_bin_main_not_function_reports_e3021() {
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let src = write_yx(tmp.path(), "main.yx", "main: Int = 5\n");

    let (code, _stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    assert_ne!(code, 0, "main 非函数应编译失败");
    assert!(
        stderr.contains("E3021"),
        "应报 E3021（入口非函数），实际 stderr: {stderr:?}"
    );
}

/// Bin 角色入口 main 带参数 → E3022（#357）。
///
/// 入口总是以**零参**调用（`execute_module` 传 `&[]`）。带参 main 此前
/// 静默运行：参数位是未初始化值（读成 Void），
/// `main: (x: Int) -> Void = (x) => { io.println(x) }` 打出 "void" 且退出码 0。
#[test]
fn test_e2e_bin_main_with_params_reports_e3022() {
    // Arrange: Bin 角色（有 manifest）+ 带参 main
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use std.io\nmain: (x: Int) -> Void = (x) => { io.println(x) }\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert: 必须编译失败，且不得静默运行
    assert_ne!(code, 0, "带参 main 应编译失败");
    assert!(
        stderr.contains("E3022"),
        "应报 E3022（入口签名不符），实际 stderr: {stderr:?}"
    );
    assert!(
        !stdout.contains("void"),
        "不得静默把未初始化参数当 Void 打印；stdout: {stdout:?}"
    );
}

/// Bin 角色重复定义 main → E2002（#358）。
///
/// 此前两个同名 main 都进函数表，静默取第一个作入口。
#[test]
fn test_e2e_bin_duplicate_main_reports_e2002() {
    // Arrange: Bin 角色 + 两个同名同签名的 main
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "app", "");
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use std.io\nmain: () -> Void = { io.println(\"first\") }\nmain: () -> Void = { io.println(\"second\") }\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert: 必须编译失败，且不得静默执行第一个定义
    assert_ne!(code, 0, "重复 main 应编译失败");
    assert!(
        stderr.contains("E2002"),
        "应报 E2002（重复定义），实际 stderr: {stderr:?}"
    );
    assert!(
        !stdout.contains("first"),
        "不得静默执行第一个定义；stdout: {stdout:?}"
    );
}

/// Script 角色（无 manifest）：顶层语句即程序主体，无入口概念。
///
/// 这条覆盖「默认情况零门槛」——单文件直跑不需要 main。
#[test]
fn test_e2e_script_top_level_statement_runs() {
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "script.yx",
        "use std.io\n\nio.println(\"script ran\")\n",
    );

    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    assert_eq!(code, 0, "Script 顶层语句应正常执行; stderr: {stderr:?}");
    assert!(
        stdout.contains("script ran"),
        "顶层语句应被执行，实际 stdout: {stdout:?}"
    );
}

/// Script 角色无 main 时**不得**执行函数表里第一个函数。
///
/// 这是 T4 的核心修复：旧实现 `find_entry_point` 兜底返回 0，
/// 于是「只有 helper 没有 main」的文件会打印 helper 的输出。
#[test]
fn test_e2e_script_no_main_does_not_run_arbitrary_function() {
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "nomain.yx",
        "use std.io\n\nhelper: () -> Void = { io.println(\"helper ran\") }\n",
    );

    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    assert_eq!(code, 0, "无 main 的 Script 不应失败; stderr: {stderr:?}");
    assert!(
        !stdout.contains("helper ran"),
        "无 main 时不得执行任意函数（旧兜底行为），实际 stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_dump_covers_statement_level_instructions() {
    // Arrange: 无可提取 span 的算术/加载指令此前无位置（#阶段零缺口）。
    // 语句侧表落地后，整条语句覆盖的指令都应带上源码行。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "stmt.yx",
        "use std.io\n\nmain = () => {\n    a = 1\n    b = a + 2\n    io.println(b)\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["dump", src.to_str().unwrap()], tmp.path());

    // Assert: 第 5 行 `b = a + 2` 的 I64Add（无可提取 span）应归属到该语句
    assert_eq!(code, 0, "dump should exit 0; stderr: {stderr:?}");
    assert!(
        stdout.contains("; <src>:5:"),
        "语句级位置应覆盖算术指令；stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("; <src>:6:"),
        "语句级位置应覆盖调用语句；stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_dump_lists_local_variable_names() {
    // Arrange: 变量名在生成期由 register_local 就地对写入 IR 槽位。
    // dump 应列出具名槽位（名字@槽位号），临时寄存器不入表。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "names.yx",
        "main = () => {\n    alpha = 1\n    beta = alpha + 2\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["dump", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "dump should exit 0; stderr: {stderr:?}");
    // 不锚定标签文案：locales/*.json 由 i18n 自动翻译机器人维护，
    // 断言 "locals:" 会被它改名（实测已改成 "Local Variables:"）而误报。
    // 断言值本身：名字@槽位 才是本功能的契约。
    assert!(
        stdout.contains("alpha@0"),
        "应列出 alpha 及其槽位号；stdout: {stdout:?}"
    );
    // 槽位号不需断言具体值：临时寄存器会占位（此例 beta 落在 2），
    // 断言"名字带上某个槽位号"即可，避免把寄存器分配细节写进测试。
    assert!(
        stdout.contains("beta@"),
        "应列出 beta 及其槽位号；stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_dump_local_names_survive_bytecode_roundtrip() {
    // Arrange: 名字只存在于 .42 调试段（代码段不序列化）。
    // build --debug-info 写出的文件，dump 读回后名字必须还在。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "roundtrip.yx",
        "main = () => {\n    gamma = 7\n    gamma\n}\n",
    );
    let out = tmp.path().join("roundtrip.42");

    // Act: 先编译落盘，再对 .42 做 dump（走的是读取路径而非重新编译）
    let (build_code, _, build_err) = run_yx(
        &[
            "build",
            "--debug-info",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ],
        tmp.path(),
    );
    assert_eq!(build_code, 0, "build should exit 0; stderr: {build_err:?}");

    let (dump_code, stdout, dump_err) = run_yx(&["dump", out.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(dump_code, 0, "dump .42 should exit 0; stderr: {dump_err:?}");
    assert!(
        stdout.contains("gamma@0"),
        "名字应经 .42 往返保持；stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_runtime_bounds_error_names_the_variable() {
    // Arrange: 索引来自具名局部变量时，E6003 应指出是哪个变量越界，
    // 而不只是给数值——`(idx)` 比 `10` 好定位得多。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "oob.yx",
        "use std.io\n\nmain = () => {\n    a = [1, 2, 3]\n    idx = 10\n    io.println(a[idx])\n}\n\nmain()\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_ne!(
        code, 0,
        "越界应是非零退出；stdout: {stdout:?} stderr: {stderr:?}"
    );
    assert!(
        combined.contains("E6003"),
        "应报 E6003 索引越界；combined: {combined:?}"
    );
    assert!(
        combined.contains("(idx)"),
        "错误信息应指出越界变量名；combined: {combined:?}"
    );
}

#[test]
fn test_e2e_runtime_bounds_error_omits_clause_for_literals() {
    // Arrange: 字面量索引没有变量名——不能留下空的 "( )" 子句。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "lit.yx",
        "use std.io\n\nmain = () => {\n    a = [1, 2, 3]\n    io.println(a[10])\n}\n\nmain()\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_ne!(code, 0, "越界应是非零退出");
    assert!(
        combined.contains("E6003"),
        "应报 E6003；combined: {combined:?}"
    );
    assert!(
        !combined.contains("()"),
        "无变量名时不应留空子句；combined: {combined:?}"
    );
}

#[test]
fn test_e2e_script_mode_top_level_diagnostic_has_span_and_var_name() {
    // Arrange: #368——顶层语句编入合成函数 `__yx_module_init`。
    // 此前该函数的调试元数据整段为空：运行期错误既没有 `-->` 源码行，
    // 也报不出索引变量名（只有数值）。不是函数体内专属问题。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "top_oob.yx",
        "use std.io\n\na = [1, 2, 3]\ni = 5\nio.println(a[i])\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_ne!(
        code, 0,
        "顶层越界应是非零退出；stdout: {stdout:?} stderr: {stderr:?}"
    );
    assert!(
        combined.contains("E6003"),
        "应报 E6003 索引越界；combined: {combined:?}"
    );
    assert!(
        combined.contains("--> "),
        "顶层语句的错误应带 `-->` 源码位置（此前完全缺失）；combined: {combined:?}"
    );
    assert!(
        combined.contains("(i)"),
        "顶层绑定的索引变量名应可见（走全局槽位名表）；combined: {combined:?}"
    );
}

#[test]
fn test_e2e_script_mode_top_level_for_loop_completes() {
    // Arrange: #368 附带修复——顶层 `for` 的循环出口跳转目标落在
    // 初始化序列段尾，`translate_init_sequence` 缺段尾哨兵时该目标查不到，
    // 偏移停在占位 0 → 运行时 E6007「跳转偏移非法」。
    // 函数体内的同名循环一直正常，差别只在合成函数的回填表。
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "top_for.yx",
        "use std.io\n\nfor i in 0..3 {\n    io.println(i)\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_eq!(code, 0, "顶层 for 应正常跑完；combined: {combined:?}");
    assert!(
        !combined.contains("E6007"),
        "不应出现跳转回填错误 E6007；combined: {combined:?}"
    );
    assert!(
        stdout.contains('0') && stdout.contains('1') && stdout.contains('2'),
        "循环体应执行三次（0/1/2）；stdout: {stdout:?}"
    );
}

#[test]
fn test_e2e_bin_mode_top_level_statement_is_user_error() {
    // Arrange: 规范 §3.11——Bin 角色（有 yaoxiang.toml）顶层不允许可执行语句。
    // 这是**文档化的用户写法错误**，必须报可行动的编译错误；
    // 此前它挨 E3005「IR 内部错误，请报告此问题」并把 `Discriminant(N)` 泄给用户，
    // 既指错方向又误导提单（同族：#360 索引赋值、#311 break）。
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"bintl\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use std.io\n\nio.println(\"top level\")\n\nmain: () -> Void = {\n    io.println(\"main\")\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_ne!(code, 0, "编译应失败；combined: {combined:?}");
    assert!(
        combined.contains("E3023"),
        "应报专门的 E3023 顶层语句错误码；combined: {combined:?}"
    );
    assert!(
        !combined.contains("E3005"),
        "不应再落入 IR 内部错误码 E3005；combined: {combined:?}"
    );
    assert!(
        !combined.contains("Discriminant"),
        "不应把内部判别式泄给用户；combined: {combined:?}"
    );
    assert!(
        !combined.contains("please report this issue"),
        "用户写法错误不应叫用户提单；combined: {combined:?}"
    );
}

#[test]
fn test_e2e_multifile_init_failure_points_at_owning_file() {
    // Arrange: #368 遗留项——多文件下各文件的初始化序列被拼成一条 `init`，
    // 而 `Instruction` 只带 Span（行/列）不带文件。没有逐条 file_id 表时，
    // 所有段的错误都指向同一个（错的）文件：用户按报出的位置去看，
    // 那里根本没有那行代码。
    let tmp = TempDir::new().unwrap();
    write_manifest(tmp.path(), "mfsrc", "");
    std::fs::write(tmp.path().join("liba.yx"), "a_val: Int = 1\n").unwrap();
    // 出错的绑定在第二个库里——报错须指向 libb.yx，而非 main.yx 或 liba.yx。
    std::fs::write(tmp.path().join("libb.yx"), "b_val: Int = 2 / 0\n").unwrap();
    let src = write_yx(
        tmp.path(),
        "main.yx",
        "use std.assert\nuse liba.{a_val}\nuse libb.{b_val}\n\nmain: () -> Void = {\n    assert.assert(a_val == 1, \"a\")\n}\n",
    );

    // Act
    let (code, stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());
    let combined = format!("{stdout}{stderr}");

    // Assert
    assert_ne!(
        code, 0,
        "libb 的 2 / 0 应让运行失败；combined: {combined:?}"
    );
    assert!(
        combined.contains("E6001"),
        "应报除零错误 E6001；combined: {combined:?}"
    );
    assert!(
        combined.contains("libb.yx"),
        "报错应指向拥有该绑定的 libb.yx；combined: {combined:?}"
    );
    assert!(
        !combined.contains("liba.yx:"),
        "不应把错误归到 liba.yx；combined: {combined:?}"
    );
}

/// `YAOXIANG_LANG` 必须能选中所有随包发布的语言。
///
/// 回归：`src/main.rs` 曾硬编码白名单 `["en","zh","zh-x-miao","zh-miao"]`，
/// `ja`/`ru`/`zh-classical` 被静默丢弃后回落 `en`——译文就在 locales/*.json 里，
/// 用户却永远看到英文。`zh-miao` 更是个**从未存在**的语言（locales 里只有
/// `zh-x-miao`）。白名单现已改为从 `i18n::available_langs()` 派生。
#[test]
fn test_e2e_yaoxiang_lang_env_selects_every_shipped_language() {
    // Arrange: 取一个必然报错的源，借错误文案判定实际生效的语言
    let tmp = TempDir::new().unwrap();
    let src = write_yx(
        tmp.path(),
        "lang_probe.yx",
        "pick: (xs: List[Int]) -> Int = (xs) => xs[0]\nmain: () -> Void = { }\n",
    );

    // 每个语言：locales/<lang>.json 里 E1103 template 的特征片段。
    // 任一语言若被白名单拦掉，就会回落 en——由下面的英文标记断言揭穿。
    let expectations: &[(&str, &str)] = &[
        ("zh", "不是类型语法"),
        ("ja", "は型構文ではありません"),
        ("ru", "не является синтаксисом типа"),
        ("zh-classical", "非类型语法"),
        ("zh-x-miao", "圆括号喵"),
    ];
    // en 的 template 特征——它是回落目标，出现即说明语言选择失效
    let english_marker = "is not type syntax";

    // Act & Assert
    for (lang, expected_fragment) in expectations {
        let (_, stdout, stderr) = run_yx_env(
            &["check", src.to_str().unwrap()],
            tmp.path(),
            &[("YAOXIANG_LANG", lang)],
        );
        let combined = format!("{stdout}{stderr}");

        assert!(
            combined.contains("E1103"),
            "应报 E1103；lang={lang} combined: {combined:?}"
        );
        assert!(
            combined.contains(expected_fragment),
            "YAOXIANG_LANG={lang} 应显示该语言译文（期望片段 {expected_fragment:?}）；combined: {combined:?}"
        );
        assert!(
            !combined.contains(english_marker),
            "YAOXIANG_LANG={lang} 不应回落英文；combined: {combined:?}"
        );
    }
}
