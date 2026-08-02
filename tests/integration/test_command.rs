//! `yaoxiang test` 命令集成测试 — 基于 RFC-036（accepted/036-test-framework.md）
//!
//! §1 CLI 设计：发现 → 子进程执行 → 报告
//! §5 发现与执行：默认 tests/**/*.yx、[tool.test].patterns、显式路径优先、exit code 判定
//! 规则 9.1：happy path / error path / boundary 三条路径

#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// 编译产物路径（cargo 在测试构建时注入）
fn yx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang"))
}

/// 在临时目录写文件（自动创建父目录）
fn write_file(
    dir: &Path,
    name: &str,
    content: &str,
) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {name}: {e}"));
    }
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {name}: {e}"));
}

/// 在指定工作目录跑 `yaoxiang test [args]`，返回 (退出码, stdout, stderr)
fn run_test_cmd(
    args: &[&str],
    cwd: &Path,
) -> (i32, String, String) {
    let output = Command::new(yx_bin())
        .arg("test")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("spawn yaoxiang test: {e}"));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

const PASS_TEST: &str = r#"
use std.assert
main = {
    assert.assert(1 + 1 == 2, "math works")
}
"#;

const FAIL_TEST: &str = r#"
use std.assert
main = {
    assert.assert(1 == 2, "one is not two")
}
"#;

#[test]
fn test_test_command_passing_file_exits_zero() {
    // Arrange - 自包含通过用例（无 toml，单文件模式）
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/ok_test.yx", PASS_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert - 默认发现 tests/**/*.yx，报告显示 PASS，退出码 0
    assert_eq!(code, 0, "全部通过应退出 0，输出:\n{stdout}");
    assert!(stdout.contains("ok_test.yx"), "报告应包含文件名:\n{stdout}");
    assert!(stdout.contains("PASS"), "报告应标记 PASS:\n{stdout}");
    assert!(
        stdout.contains("Results: 1 passed, 0 failed"),
        "汇总应计数 1 passed:\n{stdout}"
    );
}

#[test]
fn test_test_command_failing_file_reports_fail_and_exits_nonzero() {
    // Arrange
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/broken_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert - exit code 非 0，报告 FAIL 且透传断言诊断（RFC-036 §1 输出格式）
    assert_eq!(code, 1, "有失败应退出 1，输出:\n{stdout}");
    assert!(stdout.contains("FAIL"), "报告应标记 FAIL:\n{stdout}");
    assert!(
        stdout.contains("one is not two"),
        "应透传子进程的断言消息:\n{stdout}"
    );
    assert!(
        stdout.contains("Results: 0 passed, 1 failed"),
        "汇总应计数 1 failed:\n{stdout}"
    );
}

#[test]
fn test_test_command_mixed_results_summary_counts() {
    // Arrange - 一个通过一个失败
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/ok_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/broken_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert
    assert_eq!(code, 1, "混合结果应退出 1:\n{stdout}");
    assert!(
        stdout.contains("Results: 1 passed, 1 failed"),
        "汇总应分别计数:\n{stdout}"
    );
}

#[test]
fn test_test_command_explicit_path_runs_only_given_file() {
    // Arrange - tests/ 下有一个会失败的文件，但显式指定只跑通过的那个
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/ok_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/broken_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["tests/ok_test.yx"], dir.path());

    // Assert - 显式路径优先于发现（RFC-036 §5）
    assert_eq!(code, 0, "只跑通过的文件应退出 0:\n{stdout}");
    assert!(stdout.contains("ok_test.yx"), "应跑指定文件:\n{stdout}");
    assert!(
        !stdout.contains("broken_test.yx"),
        "不应跑未指定的文件:\n{stdout}"
    );
}

#[test]
fn test_test_command_no_tests_found_exits_zero() {
    // Arrange - 空目录（无 tests/）
    let dir = TempDir::new().expect("tempdir");

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert - 边界：零测试不算失败（与 cargo test 一致）
    assert_eq!(code, 0, "零测试应退出 0:\n{stdout}");
    assert!(
        stdout.contains("No tests found"),
        "应提示未发现测试:\n{stdout}"
    );
}

#[test]
fn test_test_command_project_test_imports_project_module() {
    // Arrange - RFC-036 核心场景：tests/ 下的测试导入项目根的模块
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "yaoxiang.toml", "[project]\nname = \"demo\"\n");
    write_file(
        dir.path(),
        "lib.yx",
        "helper: (x: Int) -> Int = (x) => {\n    return x * 2\n}\n",
    );
    write_file(
        dir.path(),
        "tests/lib_test.yx",
        r#"
use std.assert
use lib.{helper}
main = {
    assert.assert(helper(21) == 42, "helper(21) should be 42")
}
"#,
    );

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert - 子进程经 orchestrator 双根解析找到项目根模块
    assert_eq!(code, 0, "项目模块导入应通过:\n{stdout}");
    assert!(
        stdout.contains("Results: 1 passed, 0 failed"),
        "项目测试应通过:\n{stdout}"
    );
}

#[test]
fn test_test_command_config_patterns_respected() {
    // Arrange - [tool.test].patterns 指向自定义目录（RFC-036 §2 / RFC-015）
    let dir = TempDir::new().expect("tempdir");
    write_file(
        dir.path(),
        "yaoxiang.toml",
        "[project]\nname = \"demo\"\n\n[tool.test]\npatterns = [\"specs/**/*.yx\"]\n",
    );
    write_file(dir.path(), "specs/custom_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/ignored_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&[], dir.path());

    // Assert - 配置 pattern 生效，默认 tests/ 不再发现
    assert_eq!(code, 0, "只跑 specs/ 通过文件应退出 0:\n{stdout}");
    assert!(
        stdout.contains("custom_test.yx"),
        "应发现配置目录的文件:\n{stdout}"
    );
    assert!(
        !stdout.contains("ignored_test.yx"),
        "默认 tests/ 不应再被发现:\n{stdout}"
    );
}
