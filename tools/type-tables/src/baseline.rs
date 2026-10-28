//! 勘察基线——用于 `--report` 的自校验，**不参与** build.rs 的判定。
//!
//! 数字出自 syn AST 扫描（口径与 `scan.rs` 相同：构造点=表达式位置，排除 tests），
//! 逐项为「变体 / 定义行 / 合计 / 前向 / AST→AST / 反向桥」。
//! 与实测不一致时报告里逐项标 ✗——说明扫描口径漂了，而不是「基线过期」就当没事。
//!
//! **6.3 刷新**：WBS 6.3 删除 03 §5.2 的 13 个死变体后，基线只保留现存的 13 个变体行
//! （旧基线见文末注释）。本次数字在删除后的树上实测，与 T1.2 逐项相等。

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

/// 基线冻结 revision（本表描述 6.3 落地后的树）。
///
/// 删除前的 6.0 勘察冻结于 `bfe2f6ba366ff0c45f517a4bec8d7e87b0a59696`（见 09a §P6 6.0）；
/// 6.3 的父提交是 `c5a77d569e98745f81e2b5a4463d15b9dee0dfa7`——本表即在该树上删除
/// 13 个变体后由 T1.2 实测重取（删除前后行数与列含义不变，只有变体集变化）。
pub const SCOUT_BASELINE_REV: &str = "c5a77d569e98745f81e2b5a4463d15b9dee0dfa7";

pub const SCOUT_BASELINE: &[BaselineRow] = &[
    BaselineRow {
        variant: "Name",
        decl_line: 445,
        total: 6,
        forward: 3,
        ast_to_ast: 0,
        reverse: 3,
    },
    BaselineRow {
        variant: "Struct",
        decl_line: 449,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedStruct",
        decl_line: 453,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Tuple",
        decl_line: 458,
        total: 5,
        forward: 3,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Fn",
        decl_line: 459,
        total: 7,
        forward: 4,
        ast_to_ast: 2,
        reverse: 1,
    },
    BaselineRow {
        variant: "Generic",
        decl_line: 463,
        total: 4,
        forward: 1,
        ast_to_ast: 2,
        reverse: 1,
    },
    BaselineRow {
        variant: "Literal",
        decl_line: 471,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ptr",
        decl_line: 480,
        total: 3,
        forward: 1,
        ast_to_ast: 2,
        reverse: 0,
    },
    BaselineRow {
        variant: "Ref",
        decl_line: 483,
        total: 5,
        forward: 2,
        ast_to_ast: 2,
        reverse: 1,
    },
    BaselineRow {
        variant: "MetaType",
        decl_line: 492,
        total: 2,
        forward: 2,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "ConstExpr",
        decl_line: 501,
        total: 3,
        forward: 3,
        ast_to_ast: 0,
        reverse: 0,
    },
    BaselineRow {
        variant: "Paren",
        decl_line: 513,
        total: 3,
        forward: 2,
        ast_to_ast: 1,
        reverse: 0,
    },
    BaselineRow {
        variant: "NamedParen",
        decl_line: 528,
        total: 2,
        forward: 1,
        ast_to_ast: 1,
        reverse: 0,
    },
];

/// 删除前的 6.0 基线逐行数字（仅供追溯，不参与比对）：
///
/// `Int` / `Float` / `Char` / `String` / `Bytes` / `Bool` / `Void` / `Union` / `Enum` /
/// `Option` / `Result` / `AssocType` / `Sum` 共 13 行，已随变体删除。

/// 渲染「与勘察基线对账」一节
pub fn render_comparison(stats: &[VariantStats]) -> String {
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
            "（扫描口径与冻结基线完全一致）"
        } else {
            "——不一致项须以可复现证据裁决（见任务契约）"
        }
    ));
    out
}
