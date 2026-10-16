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
use crate::frontend::core::typecheck::semantic_db::DefinitionKind;

/// 落一个带 manifest 的双文件项目（lib.yx 定义 `add_one`，main.yx 导入并调用），
/// 返回 (dir 守卫, 入口路径, lib 路径)。
fn make_import_project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\n",
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
fn test_lsp_semantic_db_covers_target_file_by_path() {
    // Arrange - main.yx 引入并调用 lib 的 add_one
    let (_dir, entry, _lib) = make_import_project();
    let buffer = "use lib.{add_one}\n\ny = add_one(1)\n";
    std::fs::write(&entry, buffer).expect("write main.yx failed");

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert - 语义数据面以文件路径为键覆盖目标文件；与诊断同一次编译
    let key = entry.display().to_string();
    let defs = outcome.semantic_db.get_definitions(&key);
    assert!(
        defs.iter().any(|d| d.name == "y"),
        "目标文件的顶层绑定 y 必须在定义面里: {defs:?}",
    );
    assert!(
        outcome
            .semantic_db
            .get_tokens(&key)
            .is_some_and(|t| !t.is_empty()),
        "目标文件的语义 tokens 必须就位（染色数据源）",
    );
    assert!(
        outcome
            .check_diagnostics
            .iter()
            .any(|(p, _)| p.as_path() == entry.as_path()),
        "同一次编译的诊断通道必须也有目标文件条目（4.6.1 同源判据）",
    );
}

#[test]
fn test_lsp_cross_file_reference_resolves_to_real_definition() {
    // Arrange
    let (_dir, entry, lib) = make_import_project();
    let buffer = "use lib.{add_one}\n\ny = add_one(1)\n";
    std::fs::write(&entry, buffer).expect("write main.yx failed");

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert - 引用 add_one @ 行 3 的 resolves_to 指向 lib.yx 的真实定义（1:1）
    let key = entry.display().to_string();
    let lib_key = lib.display().to_string();
    let refs = outcome.semantic_db.get_references(&key);
    let add_one_ref = refs
        .iter()
        .find(|r| r.name == "add_one")
        .unwrap_or_else(|| panic!("调用点 add_one 必须有引用事件: {refs:?}"));
    assert_eq!(
        add_one_ref.resolves_to.file_path, lib_key,
        "跨文件引用的 resolves_to 必须指向定义文件 lib.yx",
    );
    assert_eq!(
        (
            add_one_ref.resolves_to.span.start.line,
            add_one_ref.resolves_to.span.start.column
        ),
        (1, 1),
        "resolves_to 必须命中 add_one 在 lib.yx 的名源位置",
    );
    // 外部定义面：lib.yx 未参与 typecheck，其顶层定义经注入面可命中
    let externals = outcome.semantic_db.external_definitions();
    assert!(
        externals.iter().any(|d| d.name == "add_one"
            && d.file_path == lib_key
            && d.kind == DefinitionKind::Function),
        "lib.yx 的 add_one 必须在外部定义面（未打开也能跳转）: {externals:?}",
    );
    assert!(
        externals.iter().all(|d| d.file_path != key),
        "目标文件自身不得进外部定义面（其全量数据已在 by_file）",
    );
    // 往返：跳转解析必须命中注入的定义条目
    let hit = outcome.semantic_db.resolve_reference(&key, 3, 5);
    let hit = hit.expect("跨文件跳转解析不得落空");
    assert_eq!(hit.file_path, lib_key, "跳转落点必须是 lib.yx");
    assert_eq!(hit.span.start.line, 1, "跳转落点必须是定义行");
}

#[test]
fn test_lsp_std_import_resolution_stays_on_use_line() {
    // Arrange - std 导入（native 导出无定义位置）
    let (_dir, entry, _lib) = make_import_project();
    let buffer = "use std.io.{println}\n\nmain = {\n    println(\"hi\")\n}\n";
    std::fs::write(&entry, buffer).expect("write main.yx failed");

    // Act
    let outcome = run_lsp(&entry, buffer).expect("driver run failed");

    // Assert - println 引用的 resolves_to 维持落 use 行（行 1 的项名位置），
    // 且文件身份仍是本文件（std 无定义文件，不越出）
    let key = entry.display().to_string();
    let refs = outcome.semantic_db.get_references(&key);
    let println_ref = refs
        .iter()
        .find(|r| r.name == "println")
        .unwrap_or_else(|| panic!("println 调用点必须有引用事件: {refs:?}"));
    assert_eq!(
        println_ref.resolves_to.file_path, key,
        "std 导入的 resolves_to 不得越出本文件（无定义文件，回退 use 行）",
    );
    assert_eq!(
        println_ref.resolves_to.span.start.line, 1,
        "std 导入的 resolves_to 必须落 use 行（现状语义保持）",
    );
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

#[test]
fn test_single_file_collect_all_semantic_db_keyed_by_path() {
    // Arrange - LSP 单文件兜底形态（Program{SingleFile, CollectAll}）
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("test", "test.yx", "x = 42\n")],
        CompileConfig::default(),
    )
    .with_aggregation(crate::driver::Aggregation::CollectAll);

    // Act
    let outcome = Driver::new().run(program).expect("driver run failed");

    // Assert - 语义数据面以单元路径为键（原 checker 硬编码 "main"）
    let defs = outcome.semantic_db.get_definitions("test.yx");
    assert!(
        defs.iter().any(|d| d.name == "x"),
        "单文件兜底必须把 x 写进定义面: {defs:?}",
    );
}

#[test]
fn test_single_file_collect_all_semantic_db_kept_on_error() {
    // Arrange - 带未定义名的错误源（CollectAll 收集模式）
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new(
            "test",
            "test.yx",
            "x = 42\ny = no_such_name + x\n",
        )],
        CompileConfig::default(),
    )
    .with_aggregation(crate::driver::Aggregation::CollectAll);

    // Act
    let outcome = Driver::new().run(program).expect("driver run failed");

    // Assert - 错误已收集，语义数据面同样保留（错误文件染色不丢）
    assert!(
        !outcome.result.errors.is_empty(),
        "未定义名必须产生类型错误",
    );
    let defs = outcome.semantic_db.get_definitions("test.yx");
    assert!(
        defs.iter().any(|d| d.name == "x"),
        "错误文件的既有定义必须保留在语义数据面: {defs:?}",
    );
}
