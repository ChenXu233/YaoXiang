//! T1 扫描：从 `AST_REL` 取 `pub enum Type` 的变体清单，再对 `src/` 做全量 AST 遍历，
//! 把每一处 `Type::<变体>` 的**表达式位置**构造点按三类归档，另标出通配 match 臂内的兜底点。
//!
//! 口径（与 03 §5.7 及 6.0 勘察一致）：
//! - **构造点** = 表达式位置的路径（`Expr::Path` / `Expr::Struct`）。模式位置（match 臂左侧、
//!   `matches!` 第 2 参、`if let`、函数参数模式）不是构造点，不计入。
//! - 宏体内不解析（`vec![Type::Int(1)]` 一类不计）——与勘察口径一致，避免把格式串/断言里的
//!   路径当构造点。
//! - 被识别为 `ast::Type` 的限定名：`Type`、`AstType`、`IrType`（`ir::Type` 别名），
//!   以及**仅当该文件 import 了 `Type as T` 时**的 `T`。
//! - 排除 `tests` 目录与 `#[cfg(test)] mod tests` 一类内联测试模块。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{
    ExprMatch, ExprPath, ExprStruct, ImplItemFn, ItemFn, ItemImpl, ItemMod, Macro, Pat, TraitItemFn,
};

use crate::{AST_REL, BYTECODE_REL, FUNCTION_REL, SCAN_ROOT_REL};

/// 构造点的类别。**三类（加反向桥两类）不得混算**——混算会把 03 §5.2 的删除名单抹平。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SiteClass {
    /// parser 前向：不在下面任何重建函数/impl 内的构造点（当前全部落在 parser）。
    Forward,
    /// AST→AST 重建：`mono/function.rs` 的 `substitute_type_ast` 与 `substitute_type_in_ast`
    /// （把已有 AST 类型重造一份以替换类型变量，不是 MonoType→AST）。
    AstToAst,
    /// 反向桥 A：`bytecode.rs` 的 `impl From<MonoType> for IrType` 重建点。
    ReverseFromIrType,
    /// 反向桥 B：`function.rs` 的嵌套 fn `mono_to_ast_type` 重建点。
    ReverseMonoToAst,
}

impl SiteClass {
    pub fn label(self) -> &'static str {
        match self {
            SiteClass::Forward => "parser 前向",
            SiteClass::AstToAst => "AST→AST 重建",
            SiteClass::ReverseFromIrType => "反向桥 A（From<MonoType> for IrType）",
            SiteClass::ReverseMonoToAst => "反向桥 B（mono_to_ast_type）",
        }
    }
}

/// 一处构造点
#[derive(Debug, Clone)]
pub struct Site {
    pub variant: String,
    pub file: String,
    pub line: usize,
    pub class: SiteClass,
    /// 位于 `_ => …` 通配 match 臂内（当前只有 `Type::Void` 的错误兜底一类）
    pub wildcard_arm: bool,
}

impl Site {
    pub fn location(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

/// `ast::Type` 的一个变体（定义顺序）
#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub line: usize,
}

/// 扫描结果
#[derive(Debug)]
pub struct ScanResult {
    /// 解析成功的非测试 `.rs` 文件数
    pub scanned_files: usize,
    /// `ast::Type` 的变体清单（定义顺序）
    pub variants: Vec<VariantDecl>,
    /// 全部构造点（未分组）
    pub sites: Vec<Site>,
}

/// 扫 `src/`（排除 tests）与变体清单
pub fn scan(root: &Path) -> Result<ScanResult, String> {
    let ast_path = root.join(AST_REL);
    let variants = parse_variants(&ast_path)?;
    let variant_set: BTreeSet<String> = variants.iter().map(|v| v.name.clone()).collect();

    let files = collect_files(&root.join(SCAN_ROOT_REL))?;
    let mut sites = Vec::new();
    let mut parse_failures = Vec::new();
    let mut scanned_files = 0usize;

    for path in &files {
        let rel = rel_path(root, path);
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
        let parsed = match syn::parse_file(&content) {
            Ok(f) => f,
            Err(e) => {
                parse_failures.push(format!("{rel}: {e}"));
                continue;
            }
        };
        scanned_files += 1;
        let mut collector = Collector {
            file: &rel,
            variants: &variant_set,
            has_t_alias: content.contains("Type as T"),
            sites: Vec::new(),
            fn_stack: Vec::new(),
            impl_stack: Vec::new(),
            wildcard_depth: 0,
            pat_depth: 0,
        };
        collector.visit_file(&parsed);
        sites.append(&mut collector.sites);
    }

    if !parse_failures.is_empty() {
        return Err(format!(
            "{} 个文件无法解析（T1 需要全量覆盖，解析失败即口径不完整）：\n  {}",
            parse_failures.len(),
            parse_failures.join("\n  ")
        ));
    }

    Ok(ScanResult {
        scanned_files,
        variants,
        sites,
    })
}

fn parse_variants(ast_path: &Path) -> Result<Vec<VariantDecl>, String> {
    let content = std::fs::read_to_string(ast_path)
        .map_err(|e| format!("读取 {} 失败: {e}", ast_path.display()))?;
    let file =
        syn::parse_file(&content).map_err(|e| format!("解析 {} 失败: {e}", ast_path.display()))?;
    for item in &file.items {
        if let syn::Item::Enum(item_enum) = item {
            if item_enum.ident == "Type" {
                return Ok(item_enum
                    .variants
                    .iter()
                    .map(|v| VariantDecl {
                        name: v.ident.to_string(),
                        line: v.ident.span().start().line,
                    })
                    .collect());
            }
        }
    }
    Err(format!("{} 中未找到 `pub enum Type`", ast_path.display()))
}

fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let entries =
            std::fs::read_dir(&cur).map_err(|e| format!("读取目录 {} 失败: {e}", cur.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录 {} 失败: {e}", cur.display()))?;
            let path = entry.path();
            if path.is_dir() {
                // tests 目录整棵排除（`src/**/tests/`）；其余目录递归
                if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn rel_path(
    root: &Path,
    path: &Path,
) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImplKind {
    /// `impl From<…MonoType> for IrType`
    FromIrType,
    Other,
}

struct Collector<'a> {
    file: &'a str,
    variants: &'a BTreeSet<String>,
    has_t_alias: bool,
    sites: Vec<Site>,
    fn_stack: Vec<String>,
    impl_stack: Vec<ImplKind>,
    wildcard_depth: usize,
    pat_depth: usize,
}

impl Collector<'_> {
    fn record(
        &mut self,
        path: &syn::Path,
        qself_present: bool,
    ) {
        // 模式位置不是构造点（syn 的 `Pat::Path` 复用 `ExprPath`，故按 visit_pat 计深度屏蔽）
        if qself_present || self.pat_depth > 0 {
            return;
        }
        let segments: Vec<&syn::Ident> = path.segments.iter().map(|s| &s.ident).collect();
        if segments.len() < 2 {
            return;
        }
        let last = segments[segments.len() - 1].to_string();
        if !self.variants.contains(&last) {
            return;
        }
        let qualifier = segments[segments.len() - 2].to_string();
        let is_type_path = matches!(qualifier.as_str(), "Type" | "AstType" | "IrType")
            || (qualifier == "T" && self.has_t_alias);
        if !is_type_path {
            return;
        }
        let line = segments[segments.len() - 1].span().start().line;
        self.sites.push(Site {
            variant: last,
            file: self.file.to_string(),
            line,
            class: self.classify(),
            wildcard_arm: self.wildcard_depth > 0,
        });
    }

    fn classify(&self) -> SiteClass {
        if self.file == BYTECODE_REL && self.impl_stack.last() == Some(&ImplKind::FromIrType) {
            return SiteClass::ReverseFromIrType;
        }
        if self.file == FUNCTION_REL {
            match self.fn_stack.last().map(String::as_str) {
                Some("mono_to_ast_type") => return SiteClass::ReverseMonoToAst,
                Some("substitute_type_ast") | Some("substitute_type_in_ast") => {
                    return SiteClass::AstToAst
                }
                _ => {}
            }
        }
        SiteClass::Forward
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(
        &mut self,
        node: &'ast ItemMod,
    ) {
        if is_test_module(node) {
            return;
        }
        visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(
        &mut self,
        node: &'ast ItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(
        &mut self,
        node: &'ast ImplItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.fn_stack.pop();
    }

    fn visit_trait_item_fn(
        &mut self,
        node: &'ast TraitItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        visit::visit_trait_item_fn(self, node);
        self.fn_stack.pop();
    }

    fn visit_item_impl(
        &mut self,
        node: &'ast ItemImpl,
    ) {
        self.impl_stack.push(impl_kind(node));
        visit::visit_item_impl(self, node);
        self.impl_stack.pop();
    }

    fn visit_expr_path(
        &mut self,
        node: &'ast ExprPath,
    ) {
        self.record(&node.path, node.qself.is_some());
        visit::visit_expr_path(self, node);
    }

    fn visit_expr_struct(
        &mut self,
        node: &'ast ExprStruct,
    ) {
        self.record(&node.path, false);
        visit::visit_expr_struct(self, node);
    }

    /// 通配臂内的构造点单独标记（`Type::Void` 的唯一前向点是 `_ => (…, Type::Void)` 错误兜底）
    fn visit_expr_match(
        &mut self,
        node: &'ast ExprMatch,
    ) {
        self.visit_expr(&node.expr);
        for arm in &node.arms {
            self.visit_pat(&arm.pat);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            let wildcard = matches!(arm.pat, Pat::Wild(_));
            if wildcard {
                self.wildcard_depth += 1;
            }
            self.visit_expr(&arm.body);
            if wildcard {
                self.wildcard_depth -= 1;
            }
        }
    }

    /// 模式位置整体屏蔽：syn 的 `Pat::Path` 复用 `ExprPath`，若只按 `visit_expr_path` 判断，
    /// match 臂左侧的 `Type::Int(_)` 会被误当成构造点（6.0 勘察同理，构造点=表达式位置）。
    fn visit_pat(
        &mut self,
        node: &'ast Pat,
    ) {
        self.pat_depth += 1;
        visit::visit_pat(self, node);
        self.pat_depth -= 1;
    }

    /// 宏体不解析（`vec!` / `matches!` / 断言宏内的路径不计为构造点）
    fn visit_macro(
        &mut self,
        _node: &'ast Macro,
    ) {
    }
}

fn impl_kind(node: &ItemImpl) -> ImplKind {
    let Some((_, trait_path, _)) = &node.trait_ else {
        return ImplKind::Other;
    };
    let is_from = trait_path
        .segments
        .last()
        .is_some_and(|s| s.ident == "From");
    let self_is_ir_type = match &*node.self_ty {
        syn::Type::Path(tp) => tp.path.segments.last().is_some_and(|s| s.ident == "IrType"),
        _ => false,
    };
    if is_from && self_is_ir_type {
        ImplKind::FromIrType
    } else {
        ImplKind::Other
    }
}

fn is_test_module(node: &ItemMod) -> bool {
    if node.ident == "tests" || node.ident == "test" {
        return true;
    }
    node.attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .meta
                .require_list()
                .is_ok_and(|list| list.tokens.to_string().contains("test"))
    })
}
