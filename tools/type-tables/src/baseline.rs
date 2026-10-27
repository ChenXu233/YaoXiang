//! 6.0 勘察基线（冻结 `bfe2f6ba3`）——用于 `--report` 的自校验，**不参与** build.rs 的判定。
//!
//! 数字出自 P6 6.0 勘察（syn AST 扫描，口径与 `scan.rs` 相同：构造点=表达式位置，排除 tests），
//! 逐项为「变体 / 定义行 / 合计 / 前向 / AST→AST / 反向桥」。
//! 与实测不一致时报告里逐项标 ✗——说明扫描口径漂了，而不是「基线过期」就当没事。

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

/// 冻结 revision：`bfe2f6ba366ff0c45f517a4bec8d7e87b0a59696`
pub const SCOUT_BASELINE_REV: &str = "bfe2f6ba366ff0c45f517a4bec8d7e87b0a59696";

pub const SCOUT_BASELINE: &[BaselineRow] = &[
    BaselineRow {
        variant: "Name",
        decl_line: 445,
        total: 4,
        forward: 2,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Int",
        decl_line: 449,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Float",
        decl_line: 450,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Char",
        decl_line: 451,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "String",
        decl_line: 452,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Bytes",
        decl_line: 453,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Bool",
        decl_line: 454,
        total: 2,
        forward: 0,
        ast_to_ast: 0,
        reverse: 2,
    },
    BaselineRow {
        variant: "Void",
        decl_line: 455,
        total: 6,
        forward: 1,
        ast_to_ast: 0,
        reverse: 5,
    },
    BaselineRow {
        variant: "Struct",
        decl_line: 456,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedStruct",
        decl_line: 460,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Union",
        decl_line: 465,
        total: 2,
        forward: 0,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Enum",
        decl_line: 466,
        total: 0,
        forward: 0,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "Tuple",
        decl_line: 467,
        total: 6,
        forward: 3,
        ast_to_ast: 2,
        reverse: 1,
    },
    BaselineRow {
        variant: "Fn",
        decl_line: 468,
        total: 7,
        forward: 4,
        ast_to_ast: 2,
        reverse: 1,
    },
    BaselineRow {
        variant: "Option",
        decl_line: 472,
        total: 2,
        forward: 0,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Result",
        decl_line: 473,
        total: 2,
        forward: 0,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Generic",
        decl_line: 474,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "AssocType",
        decl_line: 480,
        total: 2,
        forward: 0,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Sum",
        decl_line: 489,
        total: 2,
        forward: 0,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Literal",
        decl_line: 493,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ptr",
        decl_line: 502,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ref",
        decl_line: 505,
        total: 4,
        forward: 2,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "MetaType",
        decl_line: 514,
        total: 2,
        forward: 2,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "ConstExpr",
        decl_line: 523,
        total: 3,
        forward: 3,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "Paren",
        decl_line: 535,
        total: 3,
        forward: 2,
        ast_to_ast: 1,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedParen",
        decl_line: 550,
        total: 2,
        forward: 1,
        ast_to_ast: 1,
        reverse: 0,
    },
];

/// 渲染「与 6.0 勘察基线对账」一节
pub fn render_comparison(stats: &[VariantStats]) -> String {
    let mut out = String::new();
    out.push_str("── T1.6 与 6.0 勘察基线对账 ───────────────────────────────────────────\n");
    out.push_str(&format!(
        "基线 revision：{}（勘察报告的 §2 逐变体表；口径 = syn AST，构造点=表达式位置）\n",
        SCOUT_BASELINE_REV
    ));
    out.push_str("（单元格 = 实测(=基线)；四列依次为 合计 / 前向 / AST→AST / 反向桥）\n");
    out.push_str(&format!(
        "{:<12} {:>16} {:>16} {:>16} {:>16}\n",
        "变体", "合计", "前向", "AST→AST", "反向桥"
    ));
    let mut mismatches = 0usize;
    for row in SCOUT_BASELINE {
        let Some(s) = stats.iter().find(|s| s.name == row.variant) else {
            out.push_str(&format!("{:<12} 实测缺失\n", row.variant));
            mismatches += 1;
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
        if cells.iter().any(|(got, want)| got != want) {
            mismatches += 1;
            line.push_str("  ← 不一致");
        }
        out.push_str(&line);
        out.push('\n');
    }
    // 基线里没有的变体（新增变体）
    for s in stats {
        if !SCOUT_BASELINE.iter().any(|r| r.variant == s.name) {
            out.push_str(&format!(
                "{:<12} 基线外新增变体（合计 {}）\n",
                s.name,
                s.total()
            ));
            mismatches += 1;
        }
    }
    out.push_str(&format!(
        "结论：{} / {} 行一致{}\n\n",
        SCOUT_BASELINE.len().saturating_sub(mismatches),
        SCOUT_BASELINE.len(),
        if mismatches == 0 {
            "（扫描口径与 6.0 勘察完全一致）"
        } else {
            "——不一致项须以可复现证据裁决（见任务契约）"
        }
    ));
    out
}
