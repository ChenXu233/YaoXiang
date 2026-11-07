//! Lsp 形态语义数据面钉测试 — 基于 WBS 4.6.1（LSP 语义数据管线统一）。
//!
//! 判据（09 验收「语义数据与诊断出自同一次编译；项目内跨文件跳转命中定义」）：
//! - Lsp 形态的 `DriverOutcome.semantic_db` 以**单元文件路径**为键覆盖目标文件
//!   （原 checker 统一命名 "<module>"，文件身份丢失）；
//! - 跨文件引用（`use lib.{add_one}` 的调用点）的 `resolves_to` 指向 lib.yx 的
//!   真实定义位置（原落 use 行——且旧 LSP 手工管线连注册表都没有，引用事件
//!   不产生）；未打开文件的顶层定义经 `external_definitions` 注入面可命中；
//! - std/native 导入无定义位置，resolves_to 维持落 use 行的现状语义；
//! - 目标文件带类型错误时语义数据保留（CollectAll——染色/跳转不随打字
//!   中间态消失）。

use std::path::{Path, PathBuf};

use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;

/// 落一个带 manifest 的双文件项目（lib.yx 定义 `add_one`，main.yx 导入并调用），
/// 返回 (dir 守卫, 入口路径, lib 路径)。
/// 清单合法（含 version）：WBS 3.23.1 / #503 起坏清单报 E3024 中止。
fn make_import_project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest failed");
    let lib = dir.path().join("lib.yx");
    std::fs::write(
        &lib,
        "add_one: (x: Int) -> Int = (x) => {\n    return x + 1\n}\n",
    )
    .expect("write lib.yx failed");
    let main = dir.path().join("main.yx");
    (dir, main, lib)
}

/// 以 Lsp 形态跑 Driver：占位入口单元携带内存缓冲区源码。
fn run_lsp(
    entry: &Path,
    buffer: &str,
) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Lsp,
        vec![Unit::new("main", entry, buffer)],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

#[test]
fn test_lsp_semantic_db_kept_when_target_has_type_error() {
    // Arrange - 缓冲区把 add_one 的实参改成错误类型
    let (_dir, entry, _lib) = make_import_project();
    let buffer = "use lib.{add_one}\n\ny = add_one(\"nope\")\n";
    std::fs::write(&entry, "use lib.{add_one}\n\ny = add_one(1)\n").expect("write main.yx failed");

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert - 诊断收集到类型错误，但语义数据面不随错误丢弃（编辑器哲学）
    let key = entry.display().to_string();
    let main_diags = outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.as_path() == entry.as_path())
        .map(|(_, ds)| ds)
        .expect("目标文件诊断条目必须存在");
    assert!(
        main_diags.iter().any(|d| d.code == "E1002"),
        "缓冲区的类型错误必须被收集: {main_diags:?}",
    );
    assert!(
        outcome
            .semantic_db
            .get_tokens(&key)
            .is_some_and(|t| !t.is_empty()),
        "类型错误文件的语义 tokens 必须保留（染色不随打字中间态消失）",
    );
}
