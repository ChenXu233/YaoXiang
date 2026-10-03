//! 导入排序规则测试
//!
//! 对应 formatter 规范 §14 (imports)

use crate::formatter::rules::sort_imports::{classify_import, sort_imports, ImportKind};
use crate::formatter::source_map::SourceMap;
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;

fn default_source_map() -> SourceMap {
    SourceMap::build("")
}

fn make_use_stmt(path: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Use {
            path: path.to_string(),
            path_span: Span::dummy(),
            path_parts: vec![],
            items: None,
            alias: None,
            item_aliases: None,
        },
        span: Span::dummy(),
    }
}

/// Helper: test_sort_imports 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 8 个乱序 import 语句（标准库 / 外部 crate / 相对路径）。
fn unsorted_import_stmts() -> Vec<Stmt> {
    vec![
        make_use_stmt("b"),
        make_use_stmt("a"),
        make_use_stmt("std"),
        make_use_stmt("c"),
        make_use_stmt("z"),
        make_use_stmt("std::collections"),
        make_use_stmt("./foo"),
        make_use_stmt("../bar"),
    ]
}

/// Act: 排序并提取全部导入路径。
fn sorted_import_paths(stmts: &mut Vec<Stmt>) -> Vec<String> {
    sort_imports(stmts, &mut default_source_map());
    stmts
        .iter()
        .filter_map(|s| {
            if let StmtKind::Use { path, .. } = &s.kind {
                Some(path.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn test_classify_import() {
    assert_eq!(classify_import("std"), ImportKind::Std);
    assert_eq!(classify_import("std::collections"), ImportKind::Std);
    assert_eq!(classify_import("core"), ImportKind::Std);
    assert_eq!(classify_import("alloc"), ImportKind::Std);

    assert_eq!(classify_import("serde"), ImportKind::External);
    assert_eq!(classify_import("serde::Deserialize"), ImportKind::External);
    assert_eq!(classify_import("some_crate::module"), ImportKind::External);

    assert_eq!(classify_import("."), ImportKind::Relative);
    assert_eq!(classify_import(".."), ImportKind::Relative);
    assert_eq!(classify_import("./foo"), ImportKind::Relative);
    assert_eq!(classify_import("../bar"), ImportKind::Relative);
}

#[test]
fn test_sort_imports() {
    // Arrange — 8 个乱序 import（标准库 / 外部 / 相对路径）
    let mut stmts = unsorted_import_stmts();

    // Act — 排序后提取导入路径
    let paths = sorted_import_paths(&mut stmts);

    // Assert — 验证精确顺序：标准库 -> 外部 -> 相对路径
    assert_eq!(paths.len(), 8);
    assert_eq!(paths[0], "std");
    assert_eq!(paths[1], "std::collections");
    assert_eq!(paths[2], "a");
    assert_eq!(paths[3], "b");
    assert_eq!(paths[4], "c");
    assert_eq!(paths[5], "z");
    assert_eq!(paths[6], "../bar");
    assert_eq!(paths[7], "./foo");
}
