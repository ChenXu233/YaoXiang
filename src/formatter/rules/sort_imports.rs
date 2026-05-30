//! 导入语句排序规则

use std::collections::HashSet;

use crate::frontend::core::parser::ast::*;

/// 导入类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportKind {
    /// 标准库 (std, core, alloc)
    Std,
    /// 外部 crates
    External,
    /// 项目内部 (相对路径)
    Relative,
}

/// 判断导入路径类型
pub fn classify_import(path: &str) -> ImportKind {
    // std, core, alloc 开头的是标准库
    if path.starts_with("std") || path.starts_with("core") || path.starts_with("alloc") {
        return ImportKind::Std;
    }

    // 以 . 或 .. 开头的是相对路径
    if path.starts_with('.') {
        return ImportKind::Relative;
    }

    // 其他是外部 crate
    ImportKind::External
}

/// 提取导入路径用于排序（去除 items 和 alias）
fn get_import_path_for_sorting(stmt: &Stmt) -> String {
    match &stmt.kind {
        StmtKind::Use { path, .. } => path.clone(),
        _ => String::new(),
    }
}

/// 排序导入语句
pub fn sort_imports(stmts: &mut Vec<Stmt>) {
    // 收集导入语句的索引
    let use_indices: Vec<usize> = stmts
        .iter()
        .enumerate()
        .filter(|(_, stmt)| matches!(stmt.kind, StmtKind::Use { .. }))
        .map(|(i, _)| i)
        .collect();

    if use_indices.is_empty() {
        return;
    }

    // 提取导入语句并分类
    let mut std_imports = Vec::new();
    let mut external_imports = Vec::new();
    let mut relative_imports = Vec::new();

    for &idx in &use_indices {
        let stmt = &stmts[idx];
        let path = get_import_path_for_sorting(stmt);
        let kind = classify_import(&path);
        match kind {
            ImportKind::Std => std_imports.push((idx, path)),
            ImportKind::External => external_imports.push((idx, path)),
            ImportKind::Relative => relative_imports.push((idx, path)),
        }
    }

    // 组内按字母顺序排序
    std_imports.sort_by(|a, b| a.1.cmp(&b.1));
    external_imports.sort_by(|a, b| a.1.cmp(&b.1));
    relative_imports.sort_by(|a, b| a.1.cmp(&b.1));

    // 重新组织语句顺序
    let mut sorted_stmts = Vec::new();

    // 添加排序后的导入语句（按顺序：标准库 -> 外部 -> 相对路径）
    for (idx, _) in std_imports {
        sorted_stmts.push(stmts[idx].clone());
    }
    for (idx, _) in external_imports {
        sorted_stmts.push(stmts[idx].clone());
    }
    for (idx, _) in relative_imports {
        sorted_stmts.push(stmts[idx].clone());
    }

    // 添加非导入语句
    let use_indices_set: HashSet<usize> = use_indices.iter().copied().collect();
    let non_use_indices: Vec<usize> = (0..stmts.len())
        .filter(|i| !use_indices_set.contains(i))
        .collect();

    for idx in non_use_indices {
        sorted_stmts.push(stmts[idx].clone());
    }

    *stmts = sorted_stmts;
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
