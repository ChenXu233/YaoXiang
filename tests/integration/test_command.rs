//! `yaoxiang test` 命令集成测试 — 基于 RFC-036（accepted/036-test-framework.md）
//!
//! §1 CLI 设计：发现 → 子进程执行 → 报告
//! §1 Phase 2：--filter / --fail-fast / --verbose / --list / --no-progress / --json
//! §5 发现与执行：默认 tests/**/*.yx、[tool.test].patterns、显式路径优先、
//!     exit code 判定、--filter 文件名包含
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

/// 带 stdout 输出的通过用例（--verbose 显示测试 stdout 的观察对象）
const NOISY_PASS_TEST: &str = r#"
use std.io
use std.assert
main = {
    io.println("noisy stdout marker")
    assert.assert(1 + 1 == 2, "math works")
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
        stdout.contains("Results: 1 file passed, 0 files failed"),
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
        stdout.contains("Results: 0 files passed, 1 file failed"),
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
        stdout.contains("Results: 1 file passed, 1 file failed"),
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
        stdout.contains("Results: 1 file passed, 0 files failed"),
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

#[test]
fn test_test_command_filter_runs_only_matching_files() {
    // Arrange - alpha 通过，beta 失败；RFC-036 §5：--filter 按文件名包含过滤
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/alpha_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/beta_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--filter", "alpha"], dir.path());

    // Assert
    assert_eq!(code, 0, "过滤后只跑通过文件应退出 0:\n{stdout}");
    assert!(
        stdout.contains("alpha_test.yx"),
        "应包含匹配文件:\n{stdout}"
    );
    assert!(
        !stdout.contains("beta_test.yx"),
        "不应包含不匹配文件:\n{stdout}"
    );
}

#[test]
fn test_test_command_filter_without_match_reports_no_tests() {
    // Arrange - 边界：过滤后零文件与零发现同语义（RFC-036 §5）
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/alpha_test.yx", PASS_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--filter", "zzz_no_match"], dir.path());

    // Assert - 零匹配不算失败（与 cargo test 零测试一致）
    assert_eq!(code, 0, "过滤后零文件应退出 0:\n{stdout}");
    assert!(
        stdout.contains("No tests found"),
        "应提示未发现测试:\n{stdout}"
    );
}

#[test]
fn test_test_command_list_prints_files_without_running() {
    // Arrange - 通过与失败文件各一个；--list 只列出，不执行（RFC-036 §1）
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/alpha_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/beta_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--list"], dir.path());

    // Assert - 失败文件也不触发执行，故退出码 0 且无 PASS/汇总
    assert_eq!(code, 0, "列表模式不应执行测试:\n{stdout}");
    assert!(
        stdout.contains("alpha_test.yx") && stdout.contains("beta_test.yx"),
        "应列出全部发现文件:\n{stdout}"
    );
    assert!(
        !stdout.contains("Results:"),
        "列表模式不应有汇总:\n{stdout}"
    );
    assert!(
        !stdout.contains("PASS"),
        "列表模式不应有执行结果:\n{stdout}"
    );
}

#[test]
fn test_test_command_fail_fast_stops_after_first_failure() {
    // Arrange - 字典序 a_test 失败在先；RFC-036 §5：--fail-fast 首个失败即停
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/a_test.yx", FAIL_TEST);
    write_file(dir.path(), "tests/b_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/c_test.yx", PASS_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--fail-fast"], dir.path());

    // Assert - 后续文件不执行，汇总只计已执行文件
    assert_eq!(code, 1, "有失败应退出 1:\n{stdout}");
    assert!(
        stdout.contains("a_test.yx"),
        "应执行并报告首个失败:\n{stdout}"
    );
    assert!(
        !stdout.contains("b_test.yx") && !stdout.contains("c_test.yx"),
        "首个失败后不应执行后续文件:\n{stdout}"
    );
    assert!(
        stdout.contains("Stopped by --fail-fast"),
        "应提示 fail-fast 提前停止:\n{stdout}"
    );
    assert!(
        stdout.contains("Results: 0 files passed, 1 file failed"),
        "汇总只计已执行文件:\n{stdout}"
    );
}

#[test]
fn test_test_command_no_progress_suppresses_pass_lines_but_keeps_failures() {
    // Arrange - 一通过一失败；RFC-036 §1：--no-progress 抑制进度，失败不可静默
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/ok_test.yx", PASS_TEST);
    write_file(dir.path(), "tests/broken_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--no-progress"], dir.path());

    // Assert - PASS 行被抑制；FAIL 行、诊断与汇总保留
    assert_eq!(code, 1, "有失败应退出 1:\n{stdout}");
    assert!(
        !stdout.contains("ok_test.yx"),
        "PASS 进度行应被抑制:\n{stdout}"
    );
    assert!(
        stdout.contains("broken_test.yx"),
        "FAIL 行应保留:\n{stdout}"
    );
    assert!(
        stdout.contains("one is not two"),
        "失败诊断应保留:\n{stdout}"
    );
    assert!(
        stdout.contains("Results: 1 file passed, 1 file failed"),
        "汇总应保留:\n{stdout}"
    );
}

#[test]
fn test_test_command_verbose_shows_test_stdout() {
    // Arrange - 通过文件带 stdout 输出；RFC-036 §1：--verbose 显示详细输出
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/noisy_test.yx", NOISY_PASS_TEST);

    // Act - 同一文件分别以默认与 --verbose 运行
    let (code, plain, _) = run_test_cmd(&[], dir.path());
    let (code_verbose, verbose, _) = run_test_cmd(&["--verbose"], dir.path());

    // Assert - 默认隐藏测试 stdout，--verbose 显示
    assert_eq!(code, 0, "默认运行应通过:\n{plain}");
    assert_eq!(code_verbose, 0, "--verbose 运行应通过:\n{verbose}");
    assert!(
        !plain.contains("noisy stdout marker"),
        "默认应隐藏测试 stdout:\n{plain}"
    );
    assert!(
        verbose.contains("noisy stdout marker"),
        "--verbose 应显示测试 stdout:\n{verbose}"
    );
}

#[test]
fn test_test_command_json_report_shape_on_pass() {
    // Arrange - 单个通过文件；RFC-036 §1 JSON 输出
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/ok_test.yx", PASS_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--json"], dir.path());

    // Assert - stdout 是合法 JSON：summary + files，通过文件仅 file/passed/time_secs
    assert_eq!(code, 0, "全部通过应退出 0:\n{stdout}");
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout 应为合法 JSON");
    assert_eq!(report["summary"]["total"], 1, "total 应为 1:\n{stdout}");
    assert_eq!(report["summary"]["passed"], 1, "passed 应为 1:\n{stdout}");
    assert_eq!(report["summary"]["failed"], 0, "failed 应为 0:\n{stdout}");
    assert_eq!(
        report["files"][0]["passed"], true,
        "文件应标记通过:\n{stdout}"
    );
    assert!(
        report["files"][0]["file"]
            .as_str()
            .unwrap_or("")
            .contains("ok_test.yx"),
        "文件名应保留:\n{stdout}"
    );
    assert!(
        report["files"][0].get("exit_code").is_none(),
        "通过文件不应有 exit_code 字段:\n{stdout}"
    );
}

#[test]
fn test_test_command_json_report_includes_failure_detail() {
    // Arrange - 单个失败文件；RFC-036 §1：失败文件附 exit_code 与 stderr
    let dir = TempDir::new().expect("tempdir");
    write_file(dir.path(), "tests/broken_test.yx", FAIL_TEST);

    // Act
    let (code, stdout, _) = run_test_cmd(&["--json"], dir.path());

    // Assert - CI 取证字段在位：exit_code 与断言诊断
    assert_eq!(code, 1, "有失败应退出 1:\n{stdout}");
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout 应为合法 JSON");
    assert_eq!(report["summary"]["failed"], 1, "failed 应为 1:\n{stdout}");
    assert_eq!(
        report["files"][0]["exit_code"], 1,
        "失败文件应带 exit_code:\n{stdout}"
    );
    let stderr = report["files"][0]["stderr"].as_str().unwrap_or("");
    assert!(
        stderr.contains("one is not two"),
        "JSON 应内嵌断言诊断:\n{stdout}"
    );
}

#[test]
fn test_test_command_json_empty_discovery_outputs_empty_report() {
    // Arrange - 边界：空目录的 --json 输出零报告（CI 消费者无需特判）
    let dir = TempDir::new().expect("tempdir");

    // Act
    let (code, stdout, _) = run_test_cmd(&["--json"], dir.path());

    // Assert - 合法 JSON、total 0、files 空数组、退出 0
    assert_eq!(code, 0, "零发现应退出 0:\n{stdout}");
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout 应为合法 JSON");
    assert_eq!(report["summary"]["total"], 0, "total 应为 0:\n{stdout}");
    assert_eq!(
        report["files"].as_array().map(Vec::len),
        Some(0),
        "files 应为空数组:\n{stdout}"
    );
}
