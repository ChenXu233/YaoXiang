//! 多文件语料层运行器（RFC-039 D40/D48，WBS 2.3.2）
//!
//! 发现 `tests/yaoxiang-multifile/` 下的项目夹具（含 `yaoxiang.toml` 的一级
//! 子目录），以 CLI 端到端方式执行并判定。执行单元是**项目**而非单文件——
//! 这是 P4「统一 Driver」唯一可执行的行为判据来源。
//!
//! 判定契约与单文件语料（yx_runner）完全同构：
//! - 入口约定：`<夹具目录>/main.yx`，头部指令经
//!   `yaoxiang::util::test_markers::TestFileSpec` 解析（库侧单点实现）；
//! - 无 expect 指令 = 行为测试：`run` 退出码 0 = PASS；
//! - `// expect: compile-error EXXXX`：`check` 退出码非 0 且全部预期码出现；
//! - `// expect: runtime-error EXXXX`：`check` 通过 + `run` 失败且预期码出现；
//! - `// skip: <原因>` 跳过；指令解析失败 = panic（构造期拒绝，RFC-036 §8.2）。
//!
//! 二进制定位与子进程拉起共享自 `tests/common/`（禁令三：与 yx_runner 同一实现）。

mod common;

use std::path::{Path, PathBuf};

use common::{binary_name, spawn_check, spawn_run};
use yaoxiang::util::test_markers::{Expectation, TestFileSpec};

/// 入口文件约定名（见 tests/yaoxiang-multifile/README.md）。
const ENTRY_NAME: &str = "main.yx";

/// 发现全部项目夹具：tests/yaoxiang-multifile/ 下含 yaoxiang.toml 的一级子目录，排序。
fn discover_projects() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("yaoxiang-multifile");
    let mut dirs = Vec::new();
    if root.is_dir() {
        for entry in std::fs::read_dir(&root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() && path.join("yaoxiang.toml").is_file() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    dirs
}

#[test]
fn test_all_multifile_projects_pass() {
    let projects = discover_projects();
    assert!(
        !projects.is_empty(),
        "No multifile corpus projects found under tests/yaoxiang-multifile/!"
    );

    let binary = binary_name();

    for dir in &projects {
        let relative = dir
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(dir)
            .display()
            .to_string();
        let entry = dir.join(ENTRY_NAME);
        assert!(
            entry.is_file(),
            "多文件夹具缺少入口 {ENTRY_NAME}: {relative}（入口约定见 tests/yaoxiang-multifile/README.md）"
        );

        // 头部指令经 TestFileSpec 统一解析（与单文件语料、yaoxiang test CLI 同一实现）
        let spec = TestFileSpec::parse(&entry);

        if let Some(reason) = &spec.skip_reason {
            eprintln!("  [SKIP] {relative}: {reason}");
            continue;
        }
        if let Some(reason) = &spec.invalid {
            panic!("Invalid header directive: {relative}/{ENTRY_NAME}: {reason}");
        }

        match &spec.expectation {
            Expectation::Behavior => {
                let output = spawn_run(&binary, &entry, spec.mode.as_ref());
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code == 0,
                    "Multifile project failed: {relative} (exit: {code})\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
            }
            Expectation::CompileError(_) => {
                let output = spawn_check(&binary, &entry);
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code != 0,
                    "Expected compile error, but check passed: {relative}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
                if let Err(note) = spec.check_expected_codes(&stderr) {
                    panic!("预期错误码比对失败: {relative}\n{note}\nSTDERR:\n{stderr}");
                }
            }
            Expectation::RuntimeError(_) => {
                let check = spawn_check(&binary, &entry);
                let check_stderr = String::from_utf8_lossy(&check.stderr).to_string();
                assert!(
                    check.status.code().unwrap_or(-1) == 0,
                    "Expected runtime error, but compilation rejected the project: {relative}\nSTDERR:\n{check_stderr}"
                );
                let output = spawn_run(&binary, &entry, spec.mode.as_ref());
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code != 0,
                    "Expected runtime error, but run exited successfully: {relative}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
                if let Err(note) = spec.check_expected_codes(&stderr) {
                    panic!("预期错误码比对失败: {relative}\n{note}\nSTDERR:\n{stderr}");
                }
            }
        }
    }
}
