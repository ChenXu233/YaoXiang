//! `run_format_command` 退出码契约 — #419：语义错误与 FormatterBug 必须使
//! 命令失败（Err → CLI exit 1），不得静默 exit 0。
//!
//! 历史缺陷（#419 实测）：`Err(FormatError::Semantic)` / `Err(FormatError::
//! FormatterBug)` 两臂只打印后 continue，stdout / dry-run / write 三种形态
//! 对不可格式化的文件一律 exit 0——CI 的「跑 format 看退出码」格式门禁全部
//! 放行；dry-run 的 exit 2 门禁只拦「未格式化」（needs_formatting），
//! 「不可格式化」反而无信号。
//!
//! 规范来源：`docs/src/design/formatter/index.md`（format 子命令契约）+
//! `docs/src/dev/test-specification.md` 规则 3.1（命名）、规则 4.1（AAA）、
//! 规则 6.3（断言消息）。

use std::fs;
use tempfile::TempDir;

use crate::formatter::{run_format_command, FormatOptions};

/// 写一个临时 yx 文件并返回其路径。
fn write_fixture(
    dir: &TempDir,
    name: &str,
    content: &str,
) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap_or_else(|e| panic!("临时用例文件 {name} 写入失败：{e}"));
    path
}

/// 语义错误用例：调用未定义函数（format_source 语义校验必拦，E1001）。
const SEMANTIC_ERROR_SOURCE: &str = "main: () -> Void = {\n    nonexistent_fn()\n}\n";

/// 合法但未格式化用例：多余空格，格式化后必然变化。
const DIRTY_VALID_SOURCE: &str = "main: () -> Void = {\n    println(   1+2 )\n}\nmain()\n";

/// #419：语义错误文件在三种形态（stdout / dry-run / write）下都必须使命令
/// 返回 Err——CI 门禁的退出码消费方依赖非零退出。
#[test]
fn test_format_command_semantic_error_fails_all_modes() {
    // Arrange：三种 (dry_run, write) 形态 + 一个语义错误文件
    let modes = [(false, false), (true, false), (false, true)];

    for (dry_run, write) in modes {
        let dir = TempDir::new().expect("临时目录创建不应失败");
        write_fixture(&dir, "bad.yx", SEMANTIC_ERROR_SOURCE);

        // Act
        let result = run_format_command(
            &dir.path().join("bad.yx"),
            &FormatOptions::default(),
            dry_run,
            write,
        );

        // Assert：命令必须失败，且错误汇总为格式化失败聚合消息
        //（per-file 明细「semantic error(s) in …」走 stderr，anyhow 只带聚合）
        let err = result.err().unwrap_or_else(|| {
            panic!("dry_run={dry_run} write={write} 下语义错误必须使命令失败（#419）")
        });
        assert!(
            err.to_string()
                .contains("error(s) occurred during formatting"),
            "错误汇总应为格式化失败聚合消息，实际：{err}"
        );
    }
}

/// #419：混合目录（一个语义错误 + 一个合法未格式化）下，命令以 Err 收场，
/// 但合法文件仍被写回——先处理完全部文件、最后统一失败。
#[test]
fn test_format_command_formats_valid_file_alongside_error_file() {
    // Arrange：bad.yx（语义错误）+ dirty.yx（合法未格式化），write 形态
    let dir = TempDir::new().expect("临时目录创建不应失败");
    write_fixture(&dir, "bad.yx", SEMANTIC_ERROR_SOURCE);
    let dirty = write_fixture(&dir, "dirty.yx", DIRTY_VALID_SOURCE);

    // Act
    let result = run_format_command(dir.path(), &FormatOptions::default(), false, true);

    // Assert：命令失败，但合法文件的格式化结果已落盘
    assert!(
        result.is_err(),
        "目录内含语义错误文件时命令必须失败（#419），实际：{:?}",
        result.ok()
    );
    let formatted = fs::read_to_string(&dirty).expect("读回 dirty.yx 不应失败");
    assert!(
        formatted.contains("println(1 + 2)"),
        "合法文件应在命令失败前照常写回格式化结果，实际：{formatted}"
    );
}
