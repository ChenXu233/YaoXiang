//! 导入排序规则测试
//!
//! 对应 formatter 规范 §14 (imports)

use crate::formatter::rules::sort_imports::{classify_import, sort_imports, ImportKind};
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;

fn make_use_stmt(path: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Use {
            path: path.to_string(),
            path_span: Span::dummy(),
            path_parts: vec![],
            items: None,
            alias: None,
        },
        span: Span::dummy(),
    }
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
    let mut stmts = vec![
        make_use_stmt("b"),
        make_use_stmt("a"),
        make_use_stmt("std"),
        make_use_stmt("c"),
        make_use_stmt("z"),
        make_use_stmt("std::collections"),
        make_use_stmt("./foo"),
        make_use_stmt("../bar"),
    ];

    sort_imports(&mut stmts);

    // 提取所有导入路径，验证精确顺序
    let paths: Vec<String> = stmts
        .iter()
        .filter_map(|s| {
            if let StmtKind::Use { path, .. } = &s.kind {
                Some(path.clone())
            } else {
                None
            }
        })
        .collect();

    // 验证精确顺序：标准库 -> 外部 -> 相对路径
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
