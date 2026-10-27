//! T1 扫描：从 `AST_REL` 取 `pub enum Type` 的变体清单，再对 `src/` 做全量 AST 遍历，
//! 把每一处 `Type::<变体>` 的**表达式位置**构造点按三类归档，另标出通配 match 臂内的兜底点。
//!
//! 口径（与 03 §5.7 及 6.0 勘察一致）：
//! - **构造点** = 表达式位置的路径（`Expr::Path` / `Expr::Struct`）。模式位置（match 臂左侧、
//!   `matches!` 第 2 参、`if let`、函数参数模式）不是构造点，不计入。
//! - 宏体内不解析（`vec![Type::Int(1)]` 一类不计）——与勘察口径一致，避免把格式串/断言里的
//!   路径当构造点。
//! - 被识别为 `ast::Type` 的限定名：基础名 `Type`，加上**本文件按 `use` 声明解析出的别名**
//!   （如 `use …::ast::Type as AstType`、再导出 `use …::ir::Type as IrType`）——
//!   不再按文件级文本包含判定（F4），且泛型形参同名时以形参为准（遮蔽）。
//! - 排除 `tests` 目录与**要求 `cfg(test)` 成立**的内联测试模块（按 cfg 谓词判定，F3：
//!   `#[cfg(not(test))]` / `#[cfg(feature = "test")]` 不算测试模块）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::punctuated::Punctuated;
use syn::{
    ExprMatch, ExprPath, ExprStruct, ImplItemFn, ItemFn, ItemImpl, ItemMod, ItemUse, Macro, Pat,
    Token, TraitItemFn, UseTree,
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
    /// 位于 `_ => …` 通配 match 臂内。**是否豁免由 T1 的登记表决定**，本字段只记录结构事实。
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
#[derive(Debug, Default)]
pub struct ScanResult {
    /// 解析成功的非测试 `.rs` 文件数
    pub scanned_files: usize,
    /// `ast::Type` 的变体清单（定义顺序）
    pub variants: Vec<VariantDecl>,
    /// 全部构造点（未分组）
    pub sites: Vec<Site>,
    /// 类型别名解析结果：别名 → 声明该绑定的文件清单（按 `use` 声明解析，不按文本包含）
    pub aliases: BTreeMap<String, Vec<String>>,
}

/// 单个文件的扫描产物（`scan_source` 供扫描与单元测试共用）
#[derive(Debug, Default)]
pub struct ScannedFile {
    pub sites: Vec<Site>,
    /// 该文件里把 `ast::Type`（或它的再导出 `ir::Type`）绑定到别的名字的别名
    pub aliases: BTreeSet<String>,
}

/// 扫 `src/`（排除 tests）与变体清单
pub fn scan(root: &Path) -> Result<ScanResult, String> {
    let ast_path = root.join(AST_REL);
    let variants = parse_variants(&ast_path)?;
    let variant_set: BTreeSet<String> = variants.iter().map(|v| v.name.clone()).collect();

    let files = collect_files(&root.join(SCAN_ROOT_REL))?;
    let mut result = ScanResult {
        variants,
        ..ScanResult::default()
    };
    let mut parse_failures = Vec::new();

    for path in &files {
        let rel = rel_path(root, path);
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
        let scanned = match scan_source(&rel, &content, &variant_set) {
            Ok(scanned) => scanned,
            Err(e) => {
                parse_failures.push(format!("{rel}: {e}"));
                continue;
            }
        };
        result.scanned_files += 1;
        for alias in &scanned.aliases {
            result
                .aliases
                .entry(alias.clone())
                .or_default()
                .push(rel.clone());
        }
        result.sites.extend(scanned.sites);
    }

    if !parse_failures.is_empty() {
        return Err(format!(
            "{} 个文件无法解析（T1 需要全量覆盖，解析失败即口径不完整）：\n  {}",
            parse_failures.len(),
            parse_failures.join("\n  ")
        ));
    }
    for files in result.aliases.values_mut() {
        files.sort();
        files.dedup();
    }

    Ok(result)
}

/// 扫一个文件的源码（`scan` 与单元测试共用；`file` 只用于别名/分类与报告定位）
pub fn scan_source(
    file: &str,
    content: &str,
    variants: &BTreeSet<String>,
) -> Result<ScannedFile, String> {
    let parsed = syn::parse_file(content).map_err(|e| e.to_string())?;
    let aliases = collect_type_aliases(&parsed);
    let mut collector = Collector {
        file,
        variants,
        sites: Vec::new(),
        fn_stack: Vec::new(),
        impl_stack: Vec::new(),
        wildcard_depth: 0,
        pat_depth: 0,
        alias_scopes: vec![BTreeSet::new()],
        generic_scopes: Vec::new(),
        all_aliases: BTreeSet::new(),
    };
    collector.visit_file(&parsed);
    Ok(ScannedFile {
        sites: collector.sites,
        aliases,
    })
}

/// 按 `use` 声明解析「别名 → `ast::Type`」绑定（F4：不再用文件级文本包含判定）。
///
/// 识别形态：`use …::Type as X;`（含分组 `use …::{self, Type as X}` 与再导出
/// `pub use …::{Type as IrType}`）。全仓只有一个 `Type` 枚举（`parser/ast.rs`），
/// `ir::Type` 是它的再导出，故任何 `Type as X` 都指向同一个类型。
fn collect_type_aliases(file: &syn::File) -> BTreeSet<String> {
    let mut visitor = AliasCollector::default();
    visitor.visit_file(file);
    visitor.aliases
}

#[derive(Default)]
struct AliasCollector {
    aliases: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for AliasCollector {
    fn visit_item_use(
        &mut self,
        node: &'ast ItemUse,
    ) {
        collect_use_tree(&node.tree, &mut self.aliases);
        visit::visit_item_use(self, node);
    }
}

fn collect_use_tree(
    tree: &UseTree,
    out: &mut BTreeSet<String>,
) {
    match tree {
        UseTree::Path(path) => collect_use_tree(&path.tree, out),
        // 直接 `use …::Type;`：基础名 Type 本就始终识别，无需登记别名
        UseTree::Name(_) => {}
        UseTree::Rename(rename) => {
            if rename.ident == "Type" {
                out.insert(rename.rename.to_string());
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, out);
            }
        }
        UseTree::Glob(_) => {}
    }
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
    sites: Vec<Site>,
    fn_stack: Vec<String>,
    impl_stack: Vec<ImplKind>,
    wildcard_depth: usize,
    pat_depth: usize,
    /// 别名作用域栈：模块与函数/方法各自一层，`use … Type as X` 登记到当前层；
    /// 可见性 = 栈内任一层包含（内层函数看得到外层模块的 import）。
    alias_scopes: Vec<BTreeSet<String>>,
    /// 泛型形参作用域栈：同名泛型形参遮蔽别名（`fn f<T>()` 里的 `T::X` 不是 `ast::Type`）
    generic_scopes: Vec<BTreeSet<String>>,
    /// 本文件出现过的全部别名（用于报告）
    all_aliases: BTreeSet<String>,
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
        let shadowed = self
            .generic_scopes
            .iter()
            .any(|scope| scope.contains(&qualifier));
        let aliased = self
            .alias_scopes
            .iter()
            .any(|scope| scope.contains(&qualifier));
        let is_type_path = !shadowed && (qualifier == "Type" || aliased);
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

fn generic_param_names(generics: &syn::Generics) -> BTreeSet<String> {
    generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(t) => Some(t.ident.to_string()),
            syn::GenericParam::Const(c) => Some(c.ident.to_string()),
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect()
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(
        &mut self,
        node: &'ast ItemMod,
    ) {
        if is_test_module(node) {
            return;
        }
        self.alias_scopes.push(BTreeSet::new());
        visit::visit_item_mod(self, node);
        self.alias_scopes.pop();
    }

    fn visit_item_fn(
        &mut self,
        node: &'ast ItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        self.alias_scopes.push(BTreeSet::new());
        self.generic_scopes
            .push(generic_param_names(&node.sig.generics));
        visit::visit_item_fn(self, node);
        self.generic_scopes.pop();
        self.alias_scopes.pop();
        self.fn_stack.pop();
    }

    fn visit_impl_item_fn(
        &mut self,
        node: &'ast ImplItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        self.alias_scopes.push(BTreeSet::new());
        self.generic_scopes
            .push(generic_param_names(&node.sig.generics));
        visit::visit_impl_item_fn(self, node);
        self.generic_scopes.pop();
        self.alias_scopes.pop();
        self.fn_stack.pop();
    }

    fn visit_trait_item_fn(
        &mut self,
        node: &'ast TraitItemFn,
    ) {
        self.fn_stack.push(node.sig.ident.to_string());
        self.alias_scopes.push(BTreeSet::new());
        self.generic_scopes
            .push(generic_param_names(&node.sig.generics));
        visit::visit_trait_item_fn(self, node);
        self.generic_scopes.pop();
        self.alias_scopes.pop();
        self.fn_stack.pop();
    }

    fn visit_item_impl(
        &mut self,
        node: &'ast ItemImpl,
    ) {
        self.impl_stack.push(impl_kind(node));
        self.generic_scopes
            .push(generic_param_names(&node.generics));
        visit::visit_item_impl(self, node);
        self.generic_scopes.pop();
        self.impl_stack.pop();
    }

    fn visit_item_use(
        &mut self,
        node: &'ast ItemUse,
    ) {
        let mut renamed = BTreeSet::new();
        collect_use_tree(&node.tree, &mut renamed);
        if let Some(scope) = self.alias_scopes.last_mut() {
            scope.extend(renamed.iter().cloned());
        }
        self.all_aliases.extend(renamed);
        visit::visit_item_use(self, node);
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

    /// 通配臂内的构造点单独标记（Void 的兜底形态；**是否豁免由 T1 的登记表决定**）
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

/// 是否测试模块（F3：按 cfg 谓词判定，不按 token 子串）
fn is_test_module(node: &ItemMod) -> bool {
    // cfg 属性存在时以谓词为准：只有「要求 test 通过才可能编译」的模块才算测试模块，
    // 因此 `#[cfg(not(test))]` 与 `#[cfg(feature = "test")]` 仍会被扫描。
    let cfgs: Vec<&syn::Attribute> = node
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .collect();
    if !cfgs.is_empty() {
        return cfgs.iter().any(|attr| cfg_requires_test(attr));
    }
    // 无 cfg：名字为 tests/test 的模块按仓库约定视为测试模块（README 已登记的口径）
    node.ident == "tests" || node.ident == "test"
}

/// `#[cfg(<谓词>)]` 是否「要求 `test` 通过」——即 `cfg(test)` 为假时该模块必然不编译。
///
/// 按谓词语义判定（不是 token 子串）：
/// - `test` → true；`not(…)` → false（否定谓词在非 test 构建里反而为真）
/// - `all(a, b, …)` → 任一子句要求 test 即 true
/// - `any(a, b, …)` → **全部**子句都要求 test 才 true（否则在非 test 构建里仍可编译）
/// - 名值形式（`feature = "test"` / `target_os = "test"`）→ false
fn cfg_requires_test(attr: &syn::Attribute) -> bool {
    let Ok(list) = attr.meta.require_list() else {
        return false;
    };
    let Ok(predicates) = list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
    else {
        return false;
    };
    predicates.iter().any(meta_requires_test)
}

fn meta_requires_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let Some(ident) = list.path.get_ident() else {
                return false;
            };
            let Ok(children) =
                list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
            else {
                return false;
            };
            match ident.to_string().as_str() {
                "all" => children.iter().any(meta_requires_test),
                "any" => !children.is_empty() && children.iter().all(meta_requires_test),
                // `not(…)`：否定谓词不要求 test 成立
                _ => false,
            }
        }
        // `feature = "test"` 这类名值形式不是 `cfg(test)`
        syn::Meta::NameValue(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variants(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn scan_snippet(src: &str) -> ScannedFile {
        scan_source("src/fixture.rs", src, &variants(&["Int", "Void"])).expect("fixture 应能解析")
    }

    // F3 回归：#[cfg(not(test))] 不是测试模块，必须仍被扫描（旧实现按 token 子串判定会整棵排除）
    #[test]
    fn test_cfg_not_test_module_is_still_scanned() {
        // Arrange：生产模块用 #[cfg(not(test))] 包住
        let src = "#[cfg(not(test))]\nmod x {\n    fn f() -> Type { Type::Int(64) }\n}\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert
        assert_eq!(
            scanned.sites.len(),
            1,
            "#[cfg(not(test))] 的模块必须仍被扫描（不得判为测试模块）"
        );
    }

    // F3 回归：feature 名里含 "test" 不是 cfg(test)
    #[test]
    fn test_cfg_feature_named_test_is_still_scanned() {
        // Arrange
        let src = "#[cfg(feature = \"test\")]\nmod x {\n    fn f() -> Type { Type::Int(64) }\n}\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert
        assert_eq!(
            scanned.sites.len(),
            1,
            "#[cfg(feature = \"test\")] 不是 cfg(test)，模块必须仍被扫描"
        );
    }

    // F3 回归：真正的测试模块仍整棵排除
    #[test]
    fn test_cfg_test_module_is_skipped() {
        // Arrange：两种等价写法（cfg(test) 与 all(test, unix)）
        let src = "#[cfg(test)]\nmod tests {\n    fn f() -> Type { Type::Int(64) }\n}\n\
                   #[cfg(all(test, unix))]\nmod t2 {\n    fn g() -> Type { Type::Void }\n}\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert
        assert_eq!(
            scanned.sites.len(),
            0,
            "cfg(test) 与 all(test, …) 的模块必须被排除"
        );
    }

    // F4 回归：别名必须来自 use 声明，且非 T 的拼写同样识别
    #[test]
    fn test_alias_from_import_is_recognized() {
        // Arrange：非 T 的别名拼写（MyTy）
        let src = "use crate::frontend::core::parser::ast::Type as MyTy;\n\
                   fn f() -> MyTy { MyTy::Int(64) }\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert
        assert_eq!(
            scanned.sites.len(),
            1,
            "use …::ast::Type as MyTy 的别名必须被识别"
        );
        assert!(
            scanned.aliases.contains("MyTy"),
            "别名集合应记录 import 解析出的 MyTy"
        );
    }

    // F4 回归：没有 import 的别名不是 ast::Type（旧实现按文件级文本包含判定会误认）
    #[test]
    fn test_unimported_alias_is_not_recognized() {
        // Arrange：裸 AstType:: 无任何 import
        let src = "fn f() -> Type { let _ = AstType::Int(1); Type::Int(2) }\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert
        assert_eq!(
            scanned.sites.len(),
            1,
            "只有 Type::Int 是构造点，AstType::Int 无 import 不算"
        );
        assert!(scanned.aliases.is_empty(), "无 import 时别名集合应为空");
    }

    // F4 回归：同文件里泛型形参 T 遮蔽别名，不得把 T::X 当成 ast::Type
    #[test]
    fn test_generic_param_shadows_type_alias() {
        // Arrange：模块级 import 了 Type as T，同时有个泛型函数形参也叫 T
        let src = "use crate::frontend::core::parser::ast::Type as T;\n\
                   fn ok() -> Type { T::Int(64) }\n\
                   fn generic<T>(v: T) -> Type { let _ = T::Void(v); Type::Int(1) }\n";

        // Act
        let scanned = scan_snippet(src);

        // Assert：T::Int（无泛型遮蔽）= 1 点；T::Void 在 <T> 作用域内 = 不算；Type::Int = 1 点
        assert_eq!(
            scanned.sites.len(),
            2,
            "泛型形参 T 必须遮蔽别名：T::Void 不得计入，T::Int 与 Type::Int 应计入"
        );
        assert!(
            scanned.sites.iter().all(|site| site.variant == "Int"),
            "遮蔽后不应出现 Void 构造点"
        );
    }
}
