//! 勘察基线——**T1 门禁的一部分**（6.6.2 起：基线漂移与零构造走同一条失败路径）。
//!
//! 数字出自 syn AST 扫描（口径与 `scan.rs` 相同：构造点=表达式位置，排除 tests），
//! 逐项为「变体 / 定义行 / 合计 / 前向 / AST→AST / 反向桥」。
//! 与实测不一致时 [`mismatches`] 逐变体给出「实测 vs 基线」，`build.rs` 据此**拒绝编译**：
//! 不一致只有两种成立解释——源码新增/删除了构造点（须同步本表），或扫描口径漂了（须修工具）。
//!
//! **6.6.2 刷新（2026-10-30）**：裁决 1A 把 `Type::without_fn_param_names`（6.5 批引入的
//! 保真重建）归入第二类 AST→AST 重建后，逐列变化如下（每处变化的来源逐一写明）：
//! - **前向**：11 个变体各 −1，回到 6.3 冻结值——那 1 点本就不是 parser 前向构造点，
//!   而是该函数 `match self` 各臂里重造 AST 的点（`ast.rs` 的 `without_fn_param_names`）。
//!   退出的 11 个变体 = Struct / NamedStruct / Tuple / Fn / Generic / Literal / Ptr / Ref /
//!   MetaType / Paren / NamedParen（`Name` 与 `ConstExpr` 走 `self.clone()`，本就不产点）。
//! - **AST→AST**：同一批 11 个变体各 +1——点没变，只是改归第二类（裁决 1A）。
//! - **合计**：同一批 11 个变体各 +1（相对 6.3 冻结值）——这是**真实新增**的重建点：
//!   `without_fn_param_names` 由 6.5 批（`59803b4af`）引入，6.3 的基线不可能覆盖它；
//!   旧口径把它算进前向，才表现为「前向虚高、第二类不动」。
//! - **反向桥**：不变。
//! - **定义行**：按刷新后的树重取（6.3 之后 `ast.rs` 的变体行已随 6.4/6.5 平移）。
//!
//! 上一次冻结（6.3，删除 03 §5.2 的 13 个死变体后）见 git 历史；6.0 原始勘察冻结于
//! `bfe2f6ba366ff0c45f517a4bec8d7e87b0a59696`（见 09a §P6 6.0）。

use crate::t1::VariantStats;

/// 一行勘察基线
pub struct BaselineRow {
    pub variant: &'static str,
    pub decl_line: usize,
    pub total: usize,
    pub forward: usize,
    pub ast_to_ast: usize,
    pub reverse: usize,
}

/// 基线冻结 revision：本表的源码树。
///
/// 6.6.2 的改动只落在 `tools/type-tables/` 与 `build.rs`（不改 `src/`），故基线数字的
/// 源码侧就是该 revision；任何人 checkout 它并用本批之后的口径跑 `type-tables --json`
/// 都应得到逐列相同的数字（差异只可能来自口径漂移 = 门禁要抓的对象）。
pub const SCOUT_BASELINE_REV: &str = "b37bd8b5e63a83592a0c33611964d1461acd2fb7";

pub const SCOUT_BASELINE: &[BaselineRow] = &[
    BaselineRow {
        variant: "Name",
        decl_line: 442,
        total: 6,
        forward: 3,
        ast_to_ast: 0,
        reverse: 3,
    },
    BaselineRow {
        variant: "Struct",
        decl_line: 446,
        total: 4,
        forward: 1,
        ast_to_ast: 3,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedStruct",
        decl_line: 450,
        total: 4,
        forward: 1,
        ast_to_ast: 3,
        reverse: 0,
    },
    BaselineRow {
        variant: "Tuple",
        decl_line: 455,
        total: 6,
        forward: 3,
        ast_to_ast: 3,
        reverse: 0,
    },
    BaselineRow {
        variant: "Fn",
        decl_line: 463,
        total: 8,
        forward: 4,
        ast_to_ast: 3,
        reverse: 1,
    },
    BaselineRow {
        variant: "Generic",
        decl_line: 467,
        total: 5,
        forward: 1,
        ast_to_ast: 3,
        reverse: 1,
    },
    BaselineRow {
        variant: "Literal",
        decl_line: 475,
        total: 4,
        forward: 1,
        ast_to_ast: 3,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ptr",
        decl_line: 484,
        total: 4,
        forward: 1,
        ast_to_ast: 3,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ref",
        decl_line: 487,
        total: 6,
        forward: 2,
        ast_to_ast: 3,
        reverse: 1,
    },
    BaselineRow {
        variant: "MetaType",
        decl_line: 496,
        total: 3,
        forward: 2,
        ast_to_ast: 1,
        reverse: 0,
    },
    BaselineRow {
        variant: "ConstExpr",
        decl_line: 505,
        total: 3,
        forward: 3,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "Paren",
        decl_line: 517,
        total: 4,
        forward: 2,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedParen",
        decl_line: 533,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
];

// 删除前的 6.0 基线逐行数字（仅供追溯，不参与比对）：
//
// `Int` / `Float` / `Char` / `String` / `Bytes` / `Bool` / `Void` / `Union` / `Enum` /
// `Option` / `Result` / `AssocType` / `Sum` 共 13 行，已随变体删除。

/// 一处基线漂移
#[derive(Debug, Clone)]
pub struct BaselineMismatch {
    pub variant: String,
    /// 实测四列（合计 / 前向 / AST→AST / 反向桥）；`None` = 实测里没有该变体
    pub measured: Option<[usize; 4]>,
    /// 基线四列，列序同上；`None` = 基线里没有该变体（基线外新增）
    pub baseline: Option<[usize; 4]>,
}

impl BaselineMismatch {
    /// 一行可读的「实测 vs 基线」——报错与报告共用（合约要求列出三类列）
    pub fn render(&self) -> String {
        match (self.measured, self.baseline) {
            (Some(m), Some(b)) => {
                let mut line = format!(
                    "{}：前向 实测{}(基线{}) / AST→AST 实测{}(基线{}) / 反向桥 实测{}(基线{})",
                    self.variant, m[1], b[1], m[2], b[2], m[3], b[3]
                );
                if m[0] != b[0] {
                    line.push_str(&format!("；合计 实测{}(基线{})", m[0], b[0]));
                }
                line
            }
            (Some(m), None) => format!(
                "{}：基线外新增变体（实测 合计{} / 前向{} / AST→AST{} / 反向桥{}）",
                self.variant, m[0], m[1], m[2], m[3]
            ),
            (None, Some(b)) => format!(
                "{}：实测缺失（基线 合计{} / 前向{} / AST→AST{} / 反向桥{}）",
                self.variant, b[0], b[1], b[2], b[3]
            ),
            (None, None) => format!("{}：实测与基线同时缺失（不应发生）", self.variant),
        }
    }
}

/// 与基线逐列对账；空 = 一致（`build.rs` 的判定与报告的 T1.6 节共用本函数）
pub fn mismatches(stats: &[VariantStats]) -> Vec<BaselineMismatch> {
    let mut out = Vec::new();
    for row in SCOUT_BASELINE {
        let measured = stats.iter().find(|s| s.name == row.variant).map(|s| {
            [
                s.total(),
                s.forward_raw(),
                s.ast_to_ast.len(),
                s.reverse_total(),
            ]
        });
        let baseline = [row.total, row.forward, row.ast_to_ast, row.reverse];
        match measured {
            Some(m) if m != baseline => out.push(BaselineMismatch {
                variant: row.variant.to_string(),
                measured: Some(m),
                baseline: Some(baseline),
            }),
            Some(_) => {}
            None => out.push(BaselineMismatch {
                variant: row.variant.to_string(),
                measured: None,
                baseline: Some(baseline),
            }),
        }
    }
    for s in stats {
        if !SCOUT_BASELINE.iter().any(|r| r.variant == s.name) {
            out.push(BaselineMismatch {
                variant: s.name.clone(),
                measured: Some([
                    s.total(),
                    s.forward_raw(),
                    s.ast_to_ast.len(),
                    s.reverse_total(),
                ]),
                baseline: None,
            });
        }
    }
    out
}

/// 渲染「与勘察基线对账」一节
pub fn render_comparison(stats: &[VariantStats]) -> String {
    let drift = mismatches(stats);
    let mut out = String::new();
    out.push_str("── T1.6 与勘察基线对账 ────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "基线 revision：{}（逐变体表；口径 = syn AST，构造点=表达式位置）\n",
        SCOUT_BASELINE_REV
    ));
    out.push_str("（单元格 = 实测(=基线)；四列依次为 合计 / 前向 / AST→AST / 反向桥）\n");
    out.push_str(&format!(
        "{:<12} {:>16} {:>16} {:>16} {:>16}\n",
        "变体", "合计", "前向", "AST→AST", "反向桥"
    ));
    for row in SCOUT_BASELINE {
        let Some(s) = stats.iter().find(|s| s.name == row.variant) else {
            out.push_str(&format!("{:<12} 实测缺失\n", row.variant));
            continue;
        };
        let cells = [
            (s.total(), row.total),
            (s.forward_raw(), row.forward),
            (s.ast_to_ast.len(), row.ast_to_ast),
            (s.reverse_total(), row.reverse),
        ];
        let mut line = format!("{:<12}", row.variant);
        for (got, want) in cells {
            let mark = if got == want { "=" } else { "✗" };
            line.push_str(&format!(" {:>7}({}{})", got, mark, want));
        }
        if drift.iter().any(|m| m.variant == row.variant) {
            line.push_str("  ← 不一致");
        }
        out.push_str(&line);
        out.push('\n');
    }
    if !drift.is_empty() {
        out.push_str("逐项差异（实测 vs 基线）：\n");
        for m in &drift {
            out.push_str(&format!("  {}\n", m.render()));
        }
    }
    let consistent = SCOUT_BASELINE
        .iter()
        .filter(|row| !drift.iter().any(|m| m.variant == row.variant))
        .count();
    out.push_str(&format!(
        "结论：{} / {} 行一致{}\n\n",
        consistent,
        SCOUT_BASELINE.len(),
        if drift.is_empty() {
            "（扫描口径与冻结基线完全一致）"
        } else {
            "——基线对账失败：T1 门禁拒绝编译（逐项差异见上，须以可复现证据更新基线或修口径）"
        }
    ));
    out
}
