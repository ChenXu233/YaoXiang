//! T2：三张内置类型名表的逐名比对。
//!
//! 三表（各自语义不同，故按「映射规则」而不是简单集合相等比对）：
//! - **A** `from_builtin_name`（`MONO_REL`）：名字 → MonoType 的解析表；每个 match 臂的**首个写法**
//!   视为规范名，其余为别名（依据：源码每臂都把规范大写写法放首位）。
//! - **B** `CONST_PARAM_TYPES`（`AST_REL`）：可作 const 泛型参数的类型名（**A 的子集**语义）。
//! - **C** LSP 内置类型名清单（`LSP_REL`）：hover/definition 展示名（**A 规范名的超集**语义）。
//!
//! 三条检查：
//! - `C1` B ⊆ A（B 的每个名字 A 必须能解析，否则 const 参数注解 lower 不出类型）
//! - `C2` A 的规范名 ⊆ C（LSP 应认识每个可解析的内置类型名）
//! - `C3` C ⊆ A（LSP 不能展示编译器不认的类型名）

use std::collections::BTreeSet;
use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprLit, ImplItem, Item, Lit, Pat};

use crate::dispositions::{self, Disposition};
use crate::{AST_REL, LSP_REL, MONO_REL};

/// WBS 6.1.2（2026-10-27 定稿）删除的 22 个内置名「实现内拼写」。
///
/// 它们是 T2 的**常驻哨兵**：只要 `from_builtin_name` 再次接受其中任何一个，
/// 本报告就在「T2.5 已删除拼写」一节报**未登记漂移**（无处置可登记——那是
/// 回退，不是待办）。同一条判据在 `types/tests/mono.rs` 有对应的单元用例。
pub const REMOVED_SPELLINGS: &[&str] = &[
    "int", "int64", "i64", "datetime", "int32", "i32", "int16", "i16", "int8", "i8", "float",
    "float64", "f64", "float32", "f32", "bool", "char", "string", "str", "bytes", "void", "never",
];

/// `from_builtin_name` 的一个 match 臂
#[derive(Debug, Clone)]
pub struct BuiltinArm {
    /// 该臂接受的写法（首个为规范名）
    pub names: Vec<String>,
    pub line: usize,
}

impl BuiltinArm {
    pub fn canonical(&self) -> &str {
        &self.names[0]
    }
    pub fn aliases(&self) -> &[String] {
        &self.names[1..]
    }
}

/// 一条差异（含已登记处置）
#[derive(Debug)]
pub struct Difference {
    pub check: &'static str,
    pub name: String,
    pub detail: String,
    pub disposition: Option<&'static Disposition>,
}

/// T2 报告
#[derive(Debug)]
pub struct NameTableReport {
    pub builtin: Vec<BuiltinArm>,
    pub const_param: Vec<String>,
    pub const_param_line: usize,
    pub lsp: Vec<String>,
    pub lsp_line: usize,
    pub differences: Vec<Difference>,
    /// 被 `from_builtin_name` 重新接受的、WBS 6.1.2 已删除的拼写（应为空）
    pub removed_accepted: Vec<String>,
}

impl NameTableReport {
    pub fn unlisted(&self) -> usize {
        self.differences
            .iter()
            .filter(|d| d.disposition.is_none())
            .count()
    }

    fn accepted(&self) -> BTreeSet<String> {
        self.builtin
            .iter()
            .flat_map(|a| a.names.iter().cloned())
            .collect()
    }

    fn canonical(&self) -> BTreeSet<String> {
        self.builtin
            .iter()
            .map(|a| a.canonical().to_string())
            .collect()
    }

    pub fn render(&self) -> String {
        let accepted = self.accepted();
        let canonical = self.canonical();
        let const_param: BTreeSet<String> = self.const_param.iter().cloned().collect();
        let lsp: BTreeSet<String> = self.lsp.iter().cloned().collect();

        let mut out = String::new();
        out.push_str("── T2 三表一致 ────────────────────────────────────────────────────────\n");
        out.push_str(&format!(
            "T2.1 三表原文（A: {}，B: {}，C: {}）\n",
            MONO_REL, AST_REL, LSP_REL
        ));
        let alias_total: usize = self.builtin.iter().map(|a| a.aliases().len()).sum();
        out.push_str(&format!(
            "A `from_builtin_name`：{} 臂 / {} 个写法（别名 {} 个；首写法=规范名）\n",
            self.builtin.len(),
            self.builtin.len() + alias_total,
            alias_total
        ));
        for arm in &self.builtin {
            out.push_str(&format!(
                "  :{:<4} {:<9} → 别名：{}\n",
                arm.line,
                arm.canonical(),
                if arm.aliases().is_empty() {
                    "（无）".to_string()
                } else {
                    arm.aliases().join(" | ")
                }
            ));
        }
        out.push_str("  权威清单（WBS 6.1.2 / SPEC §2.1）：15 个内置名 + DateTime 临时兼容别名。\n");
        out.push_str("  15 名 = Never / Void / () / Bool / Int / Int8 / Int16 / Int32 / Int64 /\n");
        out.push_str("          Float / Float32 / Float64 / Char / String / Bytes（() 是 Void 的类型写法）。\n");
        out.push_str("  DateTime 注记：不属内置名，只是 std 层语义别名——src/std/time.rs 与\n");
        out.push_str("  src/std/interfaces/time.yx 的签名串仍在用；迁出属 P10 S11（issue #513），\n");
        out.push_str("  故 LSP 清单（C）刻意不收录它，对应 T2.C2 差异已登记。\n");
        out.push_str(&format!(
            "B `CONST_PARAM_TYPES`（:{}）：{} 项 —— {}\n",
            self.const_param_line,
            self.const_param.len(),
            self.const_param.join(", ")
        ));
        out.push_str(&format!(
            "C LSP 内置类型名清单（:{}）：{} 项 —— {}\n\n",
            self.lsp_line,
            self.lsp.len(),
            self.lsp.join(", ")
        ));

        out.push_str("T2.2 逐名矩阵（A=能被 from_builtin_name 解析；规范=是 A 的规范名）\n");
        out.push_str(&format!(
            "  {:<10} {:>2} {:>4} {:>2} {:>2}   判定\n",
            "名字", "A", "规范", "B", "C"
        ));
        let mut union: BTreeSet<String> = BTreeSet::new();
        union.extend(accepted.iter().cloned());
        union.extend(const_param.iter().cloned());
        union.extend(lsp.iter().cloned());
        for name in &union {
            let in_a = accepted.contains(name);
            let is_canon = canonical.contains(name);
            let in_b = const_param.contains(name);
            let in_c = lsp.contains(name);
            let mut flags: Vec<&str> = Vec::new();
            if in_b && !in_a {
                flags.push("C1");
            }
            if is_canon && !in_c {
                flags.push("C2");
            }
            if in_c && !in_a {
                flags.push("C3");
            }
            let verdict = if flags.is_empty() {
                "ok".to_string()
            } else {
                format!("差异（{}）", flags.join("/"))
            };
            out.push_str(&format!(
                "  {:<10} {:>2} {:>4} {:>2} {:>2}   {}\n",
                name,
                if in_a { "✓" } else { "-" },
                if is_canon { "✓" } else { "-" },
                if in_b { "✓" } else { "-" },
                if in_c { "✓" } else { "-" },
                verdict
            ));
        }
        out.push('\n');

        out.push_str("T2.3 差异清单（逐条处置）\n");
        out.push_str(&format!(
            "  差异 {} 条；已登记 {} 条；未登记 {} 条\n",
            self.differences.len(),
            self.differences.len() - self.unlisted(),
            self.unlisted()
        ));
        for d in &self.differences {
            out.push_str(&format!("  [{}] {}：{}\n", d.check, d.name, d.detail));
            match d.disposition {
                Some(disp) => {
                    out.push_str(&format!("        事实：{}\n", disp.reason));
                    out.push_str(&format!("        处置：{}\n", disp.action));
                }
                None => out.push_str(
                    "        处置：**未登记漂移**——必须登记事实与处置（dispositions.rs）\n",
                ),
            }
        }
        out.push('\n');

        out.push_str("T2.5 已删除拼写哨兵（WBS 6.1.2 定稿删除的 22 个实现内拼写）\n");
        out.push_str(&format!(
            "  检查 {} 条；被 from_builtin_name 重新接受 {} 条\n",
            REMOVED_SPELLINGS.len(),
            self.removed_accepted.len()
        ));
        if self.removed_accepted.is_empty() {
            out.push_str("  结论：全绿——22 个拼写均不被解析表接受\n\n");
        } else {
            out.push_str(&format!(
                "  结论：**报红**——{} 被重新接受。那是回退（不是待办）：删掉对应 match 臂；\n",
                self.removed_accepted.join(", ")
            ));
            out.push_str("        同一条判据在 src/frontend/core/types/tests/mono.rs 有单元用例。\n\n");
        }

        out.push_str("T2.4 结论：");
        if !self.removed_accepted.is_empty() {
            out.push_str(&format!(
                "**报红**——{} 条已删除拼写被重新接受（见 T2.5），三表一致性结论不成立\n\n",
                self.removed_accepted.len()
            ));
        } else if self.differences.is_empty() {
            out.push_str("全绿（三表逐名一致）\n\n");
        } else if self.unlisted() == 0 {
            out.push_str(&format!(
                "非全绿：{} 条差异，全部已登记处置（WBS 6.1.2 后剩余两类：\
                 CONST_PARAM_TYPES 的大写位宽缩写留给 6.2 名称统一；DateTime 待 P10 S11 迁出）\n\n",
                self.differences.len()
            ));
        } else {
            out.push_str(&format!(
                "非全绿：{} 条差异，其中 {} 条未登记（需补 dispositions.rs）\n\n",
                self.differences.len(),
                self.unlisted()
            ));
        }
        out
    }
}

/// T2 入口
pub fn check(root: &Path) -> Result<NameTableReport, String> {
    let builtin = parse_builtin_arms(root)?;
    let (const_param, const_param_line) = parse_const_param_types(root)?;
    let (lsp, lsp_line) = parse_lsp_builtin_types(root)?;

    let accepted: BTreeSet<String> = builtin
        .iter()
        .flat_map(|a| a.names.iter().cloned())
        .collect();
    let canonical: BTreeSet<String> = builtin.iter().map(|a| a.canonical().to_string()).collect();
    let lsp_set: BTreeSet<String> = lsp.iter().cloned().collect();

    let mut differences = Vec::new();
    // C1：B ⊆ A
    for name in &const_param {
        if !accepted.contains(name) {
            let hint = if canonical.contains(name) {
                "（A 的规范名同名，但同一张表里 B 用的是另一种拼写）".to_string()
            } else {
                "（A 无该名字的任何写法）".to_string()
            };
            differences.push(Difference {
                check: "T2.C1",
                name: name.clone(),
                detail: format!("CONST_PARAM_TYPES 含该名，但 from_builtin_name 不解析{hint}"),
                disposition: dispositions::lookup("T2.C1", name),
            });
        }
    }
    // C2：A 规范名 ⊆ C
    for name in &canonical {
        if !lsp_set.contains(name) {
            differences.push(Difference {
                check: "T2.C2",
                name: name.clone(),
                detail: "from_builtin_name 的规范名不在 LSP 内置类型名清单".to_string(),
                disposition: dispositions::lookup("T2.C2", name),
            });
        }
    }
    // C3：C ⊆ A
    for name in &lsp {
        if !accepted.contains(name) {
            differences.push(Difference {
                check: "T2.C3",
                name: name.clone(),
                detail: "LSP 清单含该名，但 from_builtin_name 不解析".to_string(),
                disposition: dispositions::lookup("T2.C3", name),
            });
        }
    }

    let removed_accepted = deprecated_acceptances(&accepted);

    Ok(NameTableReport {
        builtin,
        const_param,
        const_param_line,
        lsp,
        lsp_line,
        differences,
        removed_accepted,
    })
}

/// WBS 6.1.2 的哨兵：解析表重新接受的已删除拼写（应为空）。
///
/// 单独成函数是为了能对「判据能红」做单元测试——`check()` 要读仓库文件，
/// 测不到「加回一个拼写会怎样」。
fn deprecated_acceptances(accepted: &BTreeSet<String>) -> Vec<String> {
    REMOVED_SPELLINGS
        .iter()
        .filter(|name| accepted.contains(**name))
        .map(|name| (*name).to_string())
        .collect()
}

fn parse_file(path: &Path) -> Result<syn::File, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    syn::parse_file(&content).map_err(|e| format!("解析 {} 失败: {e}", path.display()))
}

fn parse_builtin_arms(root: &Path) -> Result<Vec<BuiltinArm>, String> {
    let file = parse_file(&root.join(MONO_REL))?;
    for item in &file.items {
        let Item::Impl(imp) = item else { continue };
        for impl_item in &imp.items {
            let ImplItem::Fn(f) = impl_item else { continue };
            if f.sig.ident != "from_builtin_name" {
                continue;
            }
            let mut arms = Vec::new();
            for stmt in &f.block.stmts {
                let syn::Stmt::Expr(Expr::Match(m), _) = stmt else {
                    continue;
                };
                for arm in &m.arms {
                    let mut names = Vec::new();
                    names_of_pat(&arm.pat, &mut names);
                    if names.is_empty() {
                        // `_ => None` 兜底臂
                        continue;
                    }
                    arms.push(BuiltinArm {
                        names,
                        line: arm.pat.span().start().line,
                    });
                }
            }
            if arms.is_empty() {
                return Err(format!(
                    "{MONO_REL} 的 from_builtin_name 没有可解析的字符串 match 臂"
                ));
            }
            return Ok(arms);
        }
    }
    Err(format!("{MONO_REL} 中未找到 fn from_builtin_name"))
}

fn names_of_pat(
    pat: &Pat,
    out: &mut Vec<String>,
) {
    match pat {
        Pat::Or(or) => {
            for case in &or.cases {
                names_of_pat(case, out);
            }
        }
        Pat::Lit(lit) => {
            if let Lit::Str(s) = &lit.lit {
                out.push(s.value());
            }
        }
        Pat::Paren(paren) => names_of_pat(&paren.pat, out),
        _ => {}
    }
}

fn parse_const_param_types(root: &Path) -> Result<(Vec<String>, usize), String> {
    let file = parse_file(&root.join(AST_REL))?;
    for item in &file.items {
        let Item::Const(c) = item else { continue };
        if c.ident != "CONST_PARAM_TYPES" {
            continue;
        }
        let mut names = Vec::new();
        collect_string_array(&c.expr, &mut names);
        let line = c.ident.span().start().line;
        return Ok((names, line));
    }
    Err(format!("{AST_REL} 中未找到 const CONST_PARAM_TYPES"))
}

fn collect_string_array(
    expr: &Expr,
    out: &mut Vec<String>,
) {
    let expr = match expr {
        Expr::Reference(reference) => &*reference.expr,
        other => other,
    };
    if let Expr::Array(array) = expr {
        for elem in &array.elems {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = elem
            {
                out.push(s.value());
            }
        }
    }
}

struct LspCollector {
    found: Option<(Vec<String>, usize)>,
}

impl<'ast> Visit<'ast> for LspCollector {
    fn visit_local(
        &mut self,
        node: &'ast syn::Local,
    ) {
        if self.found.is_none() {
            if let Pat::Ident(pat_ident) = &node.pat {
                if pat_ident.ident == "builtin_types" {
                    if let Some(init) = &node.init {
                        if let Expr::Array(array) = &*init.expr {
                            let mut names = Vec::new();
                            for elem in &array.elems {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(s), ..
                                }) = elem
                                {
                                    names.push(s.value());
                                }
                            }
                            self.found = Some((names, pat_ident.ident.span().start().line));
                        }
                    }
                }
            }
        }
        visit::visit_local(self, node);
    }
}

fn parse_lsp_builtin_types(root: &Path) -> Result<(Vec<String>, usize), String> {
    let file = parse_file(&root.join(LSP_REL))?;
    let mut collector = LspCollector { found: None };
    collector.visit_file(&file);
    collector
        .found
        .ok_or_else(|| format!("{LSP_REL} 中未找到 let builtin_types = [...] 清单"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // WBS 6.1.2 的哨兵必须能红能绿：把 22 个拼写之一加回解析表，本门禁就要报红。
    #[test]
    fn test_deprecated_acceptances_flags_reintroduced_spelling() {
        // Arrange：解析表接受了规范名 + 一个已删除拼写
        let mut accepted: BTreeSet<String> = BTreeSet::new();
        accepted.insert("Int".to_string());
        accepted.insert("int".to_string());

        // Act
        let flagged = deprecated_acceptances(&accepted);

        // Assert：只有已删除的那个被点名
        assert_eq!(
            flagged,
            vec!["int".to_string()],
            "把 int 加回 from_builtin_name 必须被哨兵抓到"
        );
    }

    #[test]
    fn test_deprecated_acceptances_silent_on_canonical_only_table() {
        // Arrange：只有 15 个规范名 + DateTime 临时别名
        let accepted: BTreeSet<String> = [
            "Never", "Void", "()", "Bool", "Int", "Int8", "Int16", "Int32", "Int64", "Float",
            "Float32", "Float64", "Char", "String", "Bytes", "DateTime",
        ]
        .iter()
        .map(|name| (*name).to_string())
        .collect();

        // Act
        let flagged = deprecated_acceptances(&accepted);

        // Assert：无回退
        assert!(
            flagged.is_empty(),
            "规范名 + DateTime 不应触发哨兵，实际 {flagged:?}"
        );
    }

    // 口径自校验：22 个拼写一个不多一个不少，且都不在规范名集合内。
    #[test]
    fn test_removed_spellings_list_is_the_agreed_twenty_two() {
        assert_eq!(
            REMOVED_SPELLINGS.len(),
            22,
            "2026-10-27 定稿删除的拼写数是 22"
        );
        let canonical = [
            "Never", "Void", "()", "Bool", "Int", "Int8", "Int16", "Int32", "Int64", "Float",
            "Float32", "Float64", "Char", "String", "Bytes", "DateTime",
        ];
        for name in REMOVED_SPELLINGS {
            assert!(
                !canonical.contains(name),
                "{name} 不能同时是规范名与已删除拼写"
            );
        }
    }
}
