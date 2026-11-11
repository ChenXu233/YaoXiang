//! 语料遍历共享助手（禁令三单点）：lib 内判据测试的语料发现与适用范围判定。
//!
//! 消费方：release_plan_spans（WBS 2.4.4）、verify（WBS 2.1.2 全语料跑绿）。
//! 集成测试侧（tests/ 目录）的同名逻辑在 tests/common/mod.rs——lib 测试与
//! 集成测试是两个 crate，不可跨用，各自就近单点。

use std::path::{Path, PathBuf};

use crate::util::test_markers::{Expectation, TestFileSpec};

fn collect_yx(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) {
    if !dir.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yx(&path, out);
        } else if path.extension().is_some_and(|e| e == "yx") {
            out.push(path);
        }
    }
}

/// 语料文件全集（tests/yaoxiang/ + src/std/tests/，排序保证确定性）。
pub fn corpus_files() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_yx(&manifest.join("tests").join("yaoxiang"), &mut files);
    collect_yx(&manifest.join("src").join("std").join("tests"), &mut files);
    files.sort();
    files
}

/// 判据合同面判定：skip / expect: compile-error 的文件不到达 IR 生成，
/// 不在 IR 层判据的适用范围。
pub fn reaches_ir_generation(path: &Path) -> bool {
    let spec = TestFileSpec::parse(path);
    spec.skip_reason.is_none() && !matches!(spec.expectation, Expectation::CompileError(_))
}
