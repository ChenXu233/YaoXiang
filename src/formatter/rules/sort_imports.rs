//! 导入语句排序规则

use std::collections::HashSet;

use crate::formatter::source_map::SourceMap;
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
pub fn sort_imports(
    stmts: &mut Vec<Stmt>,
    source_map: &mut SourceMap,
) {
    let use_indices: Vec<usize> = stmts
        .iter()
        .enumerate()
        .filter(|(_, stmt)| matches!(stmt.kind, StmtKind::Use { .. }))
        .map(|(i, _)| i)
        .collect();

    if use_indices.is_empty() {
        return;
    }

    // 分类
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

    // 构建新顺序（相对于 use_indices 的索引）
    let old_order: Vec<usize> = (0..use_indices.len()).collect();
    let mut new_order: Vec<usize> = Vec::new();
    for (idx, _) in &std_imports {
        new_order.push(use_indices.iter().position(|&i| i == *idx).unwrap());
    }
    for (idx, _) in &external_imports {
        new_order.push(use_indices.iter().position(|&i| i == *idx).unwrap());
    }
    for (idx, _) in &relative_imports {
        new_order.push(use_indices.iter().position(|&i| i == *idx).unwrap());
    }

    // 获取导入语句的引用
    let import_stmts: Vec<&Stmt> = use_indices.iter().map(|&i| &stmts[i]).collect();

    // 重建注释顺序
    source_map.rebuild_comments_for_imports(&old_order, &new_order, &import_stmts);

    // 重组 stmts
    let mut sorted_stmts = Vec::new();

    // 按新顺序添加导入语句
    for &new_idx in &new_order {
        sorted_stmts.push(stmts[use_indices[new_idx]].clone());
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
