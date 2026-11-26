//! 解析器子树接线自检（WBS 8.8.3）——`mod` 声明 ↔ 文件存在、无孤儿 `.rs`。
//!
//! 守的性质（与实现选择无关）：**parser 子树里的每一个 `.rs` 文件都参与编译，
//! 每一条文件形式 `mod` 声明都指向真实文件**。这是 Rust 模块系统的规则本身：
//! 未被任何 `mod` 声明的文件既不参与编译、也不会因语法或断言失败而红——它的
//! 测试永远不跑，而覆盖率与门禁报告仍把它算作已覆盖。
//!
//! 门依据（coding-rules 7.2）：**病史门**。本仓发生过一次同类缺陷：`lexer/tests/`
//! 整棵子树因 `lexer/mod.rs` 缺 `mod tests;` 而从未参与编译（实测 629 行 / 5 个
//! 真实测试文件，见 05-frontend-paradigm.md「词法层只有一个测试文件在跑」）；
//! 同一缺陷形态在本子树复现时已有一处：`tests/syntax_surface.rs` 的 8 个用例
//! 未被 `tests/mod.rs` 声明（本批接线）。换一个正确的排序/命名实现，本断言不会红
//! ——它冻的是模块接线事实，不是实现选择。
//!
//! 只读源码树（按 `CARGO_MANIFEST_DIR` 定位，不依赖工作目录、不写文件），
//! 判据是静态事实，可重复执行。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// parser 子树根目录（相对 `CARGO_MANIFEST_DIR`，与测试运行目录无关）。
fn parser_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/frontend/core/parser")
}

/// 递归收集目录下全部 `.rs` 文件（路径为绝对路径）。
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("读取目录失败：{}：{e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("读取目录项失败：{}：{e}", dir.display()))
            .path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// 递归收集目录下全部 `mod.rs`（路径为绝对路径）。
fn mod_rs_files(dir: &Path) -> Vec<PathBuf> {
    rust_files(dir)
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "mod.rs"))
        .collect()
}

/// 去掉可见性前缀（`pub ` / `pub(crate) ` / `pub(super) `），返回剩余部分。
fn strip_visibility(line: &str) -> &str {
    let Some(after) = line.strip_prefix("pub") else {
        return line;
    };
    let after = after.trim_start();
    match after.strip_prefix('(') {
        Some(inner) => match inner.find(')') {
            Some(end) => inner[end + 1..].trim_start(),
            None => line,
        },
        None => after,
    }
}

/// 一行文件形式 `mod name;` → 模块名（不是模块声明返回 `None`）。
fn parse_mod_decl(line: &str) -> Option<String> {
    let body = strip_visibility(line).strip_prefix("mod ")?.trim();
    let name = body.strip_suffix(';')?.trim();
    let is_ident = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    is_ident.then(|| name.to_string())
}

/// 一条 `mod` 声明对应的**期望文件**（`#[path]` 指定时只有一项）。
struct ModDecl {
    name: String,
    candidates: Vec<PathBuf>,
}

/// 解析 `mod.rs` 里的文件形式 `mod` 声明（含紧邻的 `#[path = "..."]`）。
fn mod_decls(mod_rs: &Path) -> Vec<ModDecl> {
    let text = fs::read_to_string(mod_rs)
        .unwrap_or_else(|e| panic!("读取失败：{}：{e}", mod_rs.display()));
    let dir = mod_rs
        .parent()
        .unwrap_or_else(|| panic!("mod.rs 没有父目录：{}", mod_rs.display()));
    let mut out = Vec::new();
    let mut pending_path: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("//") {
            continue;
        }
        if let Some(attr) = line.strip_prefix("#[path") {
            if let (Some(start), Some(end)) = (attr.find('"'), attr.rfind('"')) {
                if start < end {
                    pending_path = Some(attr[start + 1..end].to_string());
                    continue;
                }
            }
            pending_path = None;
            continue;
        }
        let Some(name) = parse_mod_decl(line) else {
            if !line.starts_with('#') {
                pending_path = None;
            }
            continue;
        };
        let candidates = match pending_path.take() {
            Some(rel) => vec![dir.join(rel)],
            None => vec![
                dir.join(format!("{name}.rs")),
                dir.join(&name).join("mod.rs"),
            ],
        };
        out.push(ModDecl { name, candidates });
    }
    out
}

/// 全部 `mod` 声明**解析到**的文件（存在者优先取第一个存在的候选）。
fn declared_targets(root: &Path) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    for mod_rs in mod_rs_files(root) {
        for decl in mod_decls(&mod_rs) {
            let resolved = decl
                .candidates
                .iter()
                .find(|path| path.is_file())
                .or_else(|| decl.candidates.first())
                .cloned();
            out.extend(resolved);
        }
    }
    out
}

/// 路径相对子树根的展示形式（失败消息里给人看）。
fn relative(
    root: &Path,
    path: &Path,
) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

#[test]
fn test_parser_module_declarations_resolve_to_existing_files() {
    // Arrange：子树里每一条 `mod name;` 声明的期望文件候选。
    let root = parser_root();
    let mut missing: Vec<String> = Vec::new();

    // Act：逐条声明核对候选文件中是否至少一个真实存在。
    for mod_rs in mod_rs_files(&root) {
        for decl in mod_decls(&mod_rs) {
            if !decl.candidates.iter().any(|path| path.is_file()) {
                missing.push(format!(
                    "{} 的 `mod {};` 无对应文件（候选：{}）",
                    relative(&root, &mod_rs),
                    decl.name,
                    decl.candidates
                        .iter()
                        .map(|path| relative(&root, path))
                        .collect::<Vec<_>>()
                        .join(" / ")
                ));
            }
        }
    }

    // Assert：一条都不能缺——缺了说明声明写错（编译期本已红，此处把哪一条说清楚）。
    assert!(
        missing.is_empty(),
        "parser 子树存在指向不存在文件的 `mod` 声明：{missing:#?}"
    );
}

#[test]
fn test_parser_module_tree_has_no_orphan_rust_files() {
    // Arrange：子树内全部 `.rs` 文件、全部声明解析到的目标文件、子树根 mod.rs。
    let root = parser_root();
    let files = rust_files(&root);
    let declared = declared_targets(&root);
    let root_mod = root.join("mod.rs");

    // Act：找出未被任何 `mod` 声明接线的文件（子树根 mod.rs 由父模块声明，不在本子树核对）。
    let orphans: Vec<String> = files
        .iter()
        .filter(|path| **path != root_mod && !declared.contains(*path))
        .map(|path| relative(&root, path))
        .collect();

    // Assert：零孤儿——孤儿文件不参与编译，其中的测试永远不跑。
    assert!(
        orphans.is_empty(),
        "parser 子树存在未被 `mod` 声明接线的 `.rs` 文件（其内容从不参与编译）：{orphans:#?}"
    );
}
