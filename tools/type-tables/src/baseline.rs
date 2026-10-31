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
//!
//! **数字必须同时钉住两样东西（6.6.3 复审 F1/F2 的修复）**：
//! 1. **源码树**（[`SCOUT_BASELINE_REV`]）——同一份源码才谈得上同样的点数；
//! 2. **口径**（[`SCOUT_BASELINE_CALIBER_REV`] + [`SCOUT_BASELINE_CALIBER`]）——同一份
//!    `scan.rs` 分类登记表才谈得上同样的归类。只钉源码不钉口径时，把某函数的归类改掉
//!    同样会得到「基线漂移」，却看不出是**口径变了**（数字根本不可比），复审只能靠人读注释。
//!    故口径快照与现行 `REBUILD_FUNCTIONS` 在门禁里**机器化对账**：不同代即报
//!    「口径与基线不同代」，并把基线漂移比对**跳过**（跨口径的数字没有可比性）。
//! 3. 每条漂移都带**构造点 `file:line`**（[`SiteTrace`]，数据取自 `VariantStats` 的
//!    现成 `Vec<Site>`）——与其余几类违规一样可定位，不停在「数字不对」上。

use crate::scan::{RebuildEntry, Site, REBUILD_FUNCTIONS};
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

/// 口径 revision：上表的数字**按哪一代 `scan.rs` 分类登记表**量出来的。
///
/// 口径 = [`crate::scan::REBUILD_FUNCTIONS`]（文件 + 函数名 → 类别）。它与
/// [`SCOUT_BASELINE_CALIBER`] 逐条对账：不一致即「口径与基线不同代」，门禁拒绝编译
/// （跨口径的基线数字不可比，比对必须跳过——否则只会退化成一条看不出原因的基线漂移）。
///
/// 改口径（新增/删除重建函数、改某函数的类别）时，**同批**更新：
/// 本常量 + [`SCOUT_BASELINE_CALIBER`] + 受影响的 [`SCOUT_BASELINE`] 数字 + [`SCOUT_BASELINE_REV`]。
pub const SCOUT_BASELINE_CALIBER_REV: &str = "63ef9ca7cfb1287eed4019d324cbb736b8e2447e";

/// 口径快照的一行：**文件 + 函数 → 类别键**（[`SiteClass::key`]，不是展示文案）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaliberEntry {
    /// 函数所在文件（仓库相对路径）
    pub file: &'static str,
    /// 函数名
    pub function: &'static str,
    /// 该函数里构造点的类别键（`ast_to_ast` / `reverse_mono_to_ast` / …）
    pub class: &'static str,
}

/// 冻结口径快照（= [`SCOUT_BASELINE_CALIBER_REV`] 上的 `scan.rs::REBUILD_FUNCTIONS`）。
///
/// 它不与 `REBUILD_FUNCTIONS` 共表，正是为了让「口径被改」成为**可检测事件**：
/// 共表则改一处两处一起变，门禁永远看不见口径变更。
pub const SCOUT_BASELINE_CALIBER: &[CaliberEntry] = &[
    CaliberEntry {
        file: crate::FUNCTION_REL,
        function: "substitute_type_ast",
        class: "ast_to_ast",
    },
    CaliberEntry {
        file: crate::FUNCTION_REL,
        function: "substitute_type_in_ast",
        class: "ast_to_ast",
    },
    CaliberEntry {
        file: crate::AST_REL,
        function: "without_fn_param_names",
        class: "ast_to_ast",
    },
    CaliberEntry {
        file: crate::FUNCTION_REL,
        function: "mono_to_ast_type",
        class: "reverse_mono_to_ast",
    },
];

/// 口径快照 ↔ 现行登记表的差异（空 = 同代）。
///
/// 三条判据（比照 [`crate::scan::missing_rebuild_functions`] 的「登记表必须对得上别的东西」形态）：
/// 登记表新增了快照里没有的函数 / 快照里有而登记表删了 / 同名函数的**类别变了**。
pub fn caliber_drift(
    registry: &[RebuildEntry],
    snapshot: &[CaliberEntry],
) -> Vec<String> {
    let mut out = Vec::new();
    for entry in registry {
        match snapshot
            .iter()
            .find(|s| s.file == entry.file && s.function == entry.function)
        {
            Some(s) if s.class != entry.class.key() => out.push(format!(
                "类别不同：{}::{} 冻结口径 {} → 现行 {}",
                entry.file,
                entry.function,
                s.class,
                entry.class.key()
            )),
            Some(_) => {}
            None => out.push(format!(
                "登记表新增：{}::{}（现行 {}）——冻结口径里没有这个函数",
                entry.file,
                entry.function,
                entry.class.key()
            )),
        }
    }
    for snap in snapshot {
        if !registry
            .iter()
            .any(|entry| entry.file == snap.file && entry.function == snap.function)
        {
            out.push(format!(
                "登记表删除了冻结口径里的函数：{}::{}（口径 {}）",
                snap.file, snap.function, snap.class
            ));
        }
    }
    out
}

/// 门禁用：现行登记表是否仍与冻结口径同代。
///
/// `Err` 时**必须停止比对基线数字**（见 [`mismatches`] 的调用方 t1::gate_failures）：
/// 跨口径的点数没有可比性，继续比对只会把「口径变了」伪装成「数字漂了」。
pub fn caliber_drift_live() -> Vec<String> {
    caliber_drift(REBUILD_FUNCTIONS, SCOUT_BASELINE_CALIBER)
}

/// 「口径与基线不同代」的报错正文（`build.rs` 的失败路径与 `--report` 共用）
pub fn caliber_error() -> Option<String> {
    let drift = caliber_drift_live();
    if drift.is_empty() {
        return None;
    }
    Some(format!(
        "口径与基线不同代（口径 revision {SCOUT_BASELINE_CALIBER_REV}）：\n  {}\n  ——改口径必须同批刷新 SCOUT_BASELINE_CALIBER / \
         SCOUT_BASELINE_CALIBER_REV 与受影响的 SCOUT_BASELINE 数字；在此之前基线漂移比对已跳过（跨口径的数字不可比）。",
        drift.join("\n  ")
    ))
}

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

/// 一个变体在某一列上的构造点位置（`file:line`）——报错与报告定位用
#[derive(Debug, Clone)]
pub struct SiteTrace {
    /// 列名：前向 / AST→AST / 反向桥
    pub column: &'static str,
    /// 该列的构造点位置（`file:line`；空 = 实测该列没有构造点）
    pub locations: Vec<String>,
}

impl SiteTrace {
    fn render(&self) -> String {
        if self.locations.is_empty() {
            format!("{}（实测无该列构造点）", self.column)
        } else {
            format!("{} {}", self.column, self.locations.join("、"))
        }
    }
}

/// 一处基线漂移
#[derive(Debug, Clone)]
pub struct BaselineMismatch {
    pub variant: String,
    /// 实测四列（合计 / 前向 / AST→AST / 反向桥）；`None` = 实测里没有该变体
    pub measured: Option<[usize; 4]>,
    /// 基线四列，列序同上；`None` = 基线里没有该变体（基线外新增）
    pub baseline: Option<[usize; 4]>,
    /// 该变体在 `ast::Type` 里的定义行；实测存在时总有（`AST_REL:decl_line`）
    pub decl_line: Option<usize>,
    /// 定位用的构造点位置，按列分组：`Counts` 只列**有差异**的列，
    /// 基线外新增变体列全部非空列；实测缺失（基线独有）时为空。
    pub sites: Vec<SiteTrace>,
}

/// 取某变体某列的构造点位置（列号：1 前向 / 2 AST→AST / 3 反向桥，与 measured 的列序一致）
fn sites_of(
    stats: &VariantStats,
    column: usize,
) -> Vec<String> {
    match column {
        1 => stats.forward.iter().map(Site::location).collect(),
        2 => stats.ast_to_ast.iter().map(Site::location).collect(),
        3 => stats
            .reverse_from
            .iter()
            .chain(stats.reverse_mono.iter())
            .map(Site::location)
            .collect(),
        _ => Vec::new(),
    }
}

fn column_name(column: usize) -> &'static str {
    match column {
        1 => "前向",
        2 => "AST→AST",
        3 => "反向桥",
        _ => "合计",
    }
}

impl BaselineMismatch {
    /// 一行可读的「实测 vs 基线 + 构造点位置」——报错与报告共用。
    ///
    /// 报错文本必须能定位：与零构造 / 兜底点几类违规一样带 `file:line`
    /// （复审 F1：此前只有数字，读者拿不到「去哪儿看」）。
    pub fn render(&self) -> String {
        let mut line = match (self.measured, self.baseline) {
            (Some(m), Some(b)) => {
                let mut head = format!(
                    "{}：前向 实测{}(基线{}) / AST→AST 实测{}(基线{}) / 反向桥 实测{}(基线{})",
                    self.variant, m[1], b[1], m[2], b[2], m[3], b[3]
                );
                if m[0] != b[0] {
                    head.push_str(&format!("；合计 实测{}(基线{})", m[0], b[0]));
                }
                head
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
        };
        if let Some(decl_line) = self.decl_line {
            line.push_str(&format!("；定义 {}:{}", crate::AST_REL, decl_line));
        }
        if !self.sites.is_empty() {
            line.push_str("；构造点：");
            line.push_str(
                &self
                    .sites
                    .iter()
                    .map(SiteTrace::render)
                    .collect::<Vec<_>>()
                    .join("；"),
            );
        }
        line
    }
}

/// 与基线逐列对账；空 = 一致（`build.rs` 的判定与报告的 T1.6 节共用本函数）
pub fn mismatches(stats: &[VariantStats]) -> Vec<BaselineMismatch> {
    let mut out = Vec::new();
    for row in SCOUT_BASELINE {
        let found = stats.iter().find(|s| s.name == row.variant);
        let baseline = [row.total, row.forward, row.ast_to_ast, row.reverse];
        match found {
            Some(s) => {
                let measured = [
                    s.total(),
                    s.forward_raw(),
                    s.ast_to_ast.len(),
                    s.reverse_total(),
                ];
                if measured == baseline {
                    continue;
                }
                // 只列**有差异**的列的构造点（1=前向 2=AST→AST 3=反向桥；合计是派生列）
                let sites = (1..=3)
                    .filter(|column| measured[*column] != baseline[*column])
                    .map(|column| SiteTrace {
                        column: column_name(column),
                        locations: sites_of(s, column),
                    })
                    .collect();
                out.push(BaselineMismatch {
                    variant: row.variant.to_string(),
                    measured: Some(measured),
                    baseline: Some(baseline),
                    decl_line: Some(s.decl_line),
                    sites,
                });
            }
            None => out.push(BaselineMismatch {
                variant: row.variant.to_string(),
                measured: None,
                baseline: Some(baseline),
                decl_line: None,
                sites: Vec::new(),
            }),
        }
    }
    for s in stats {
        if !SCOUT_BASELINE.iter().any(|r| r.variant == s.name) {
            let measured = [
                s.total(),
                s.forward_raw(),
                s.ast_to_ast.len(),
                s.reverse_total(),
            ];
            let sites = (1..=3)
                .map(|column| SiteTrace {
                    column: column_name(column),
                    locations: sites_of(s, column),
                })
                .filter(|trace| !trace.locations.is_empty())
                .collect();
            out.push(BaselineMismatch {
                variant: s.name.clone(),
                measured: Some(measured),
                baseline: None,
                decl_line: Some(s.decl_line),
                sites,
            });
        }
    }
    out
}

/// 渲染「与勘察基线对账」一节
pub fn render_comparison(stats: &[VariantStats]) -> String {
    let drift = mismatches(stats);
    let caliber = caliber_drift_live();
    let mut out = String::new();
    out.push_str("── T1.6 与勘察基线对账 ────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "基线 revision：{}（逐变体表；源码树）\n",
        SCOUT_BASELINE_REV
    ));
    out.push_str(&format!(
        "口径 revision：{}（scan.rs 的 REBUILD_FUNCTIONS 快照；口径 = 重建函数归类表）\n",
        SCOUT_BASELINE_CALIBER_REV
    ));
    if caliber.is_empty() {
        out.push_str("口径对账：一致（现行登记表 == 冻结快照）\n");
    } else {
        out.push_str(&format!(
            "口径对账：**不同代**（{} 处）——基线数字跨口径不可比，漂移比对不参与判定：\n",
            caliber.len()
        ));
        for line in &caliber {
            out.push_str(&format!("  {line}\n"));
        }
    }
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
        if !caliber.is_empty() {
            "——但**口径与基线不同代**：门禁拒绝编译，基线漂移比对已跳过（须同批刷新口径快照与基线数字）"
        } else if drift.is_empty() {
            "（源码树与口径都与冻结基线一致）"
        } else {
            "——基线对账失败：T1 门禁拒绝编译（逐项差异见上，含构造点 file:line；须以可复现证据更新基线或修口径）"
        }
    ));
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{ScanResult, SiteClass, VariantDecl};
    use crate::t1::ForwardReport;

    fn stats_with(
        variants: Vec<VariantDecl>,
        sites: Vec<Site>,
    ) -> Vec<VariantStats> {
        ForwardReport::new(ScanResult {
            scanned_files: 1,
            variants,
            sites,
            aliases: Default::default(),
        })
        .stats
    }

    fn variant(
        name: &str,
        line: usize,
    ) -> VariantDecl {
        VariantDecl {
            name: name.to_string(),
            line,
        }
    }

    fn site(
        variant_name: &str,
        file: &str,
        line: usize,
        class: SiteClass,
    ) -> Site {
        Site {
            variant: variant_name.to_string(),
            file: file.to_string(),
            line,
            class,
            wildcard_arm: false,
        }
    }

    // 复审 F1：基线漂移的报错必须带构造点 file:line（与其余几类违规同样可定位）
    #[test]
    fn test_mismatch_render_carries_site_file_line() {
        // Arrange：Struct 的前向比基线多 1 点，且该点有确定位置
        let stats = stats_with(
            vec![variant("Struct", 446)],
            vec![
                site(
                    "Struct",
                    "src/frontend/core/types/x.rs",
                    42,
                    SiteClass::Forward,
                ),
                site(
                    "Struct",
                    "src/frontend/core/types/y.rs",
                    77,
                    SiteClass::Forward,
                ),
            ],
        );

        // Act
        let mismatch = mismatches(&stats)
            .into_iter()
            .find(|m| m.variant == "Struct")
            .expect("Struct 前向 2 > 基线 1，必须报漂移");
        let rendered = mismatch.render();

        // Assert
        assert!(
            rendered.contains("src/frontend/core/types/x.rs:42"),
            "漂移报错必须带构造点 file:line，实际：{rendered}"
        );
        assert!(
            rendered.contains("src/frontend/core/types/y.rs:77"),
            "该列的全部构造点都要列出，实际：{rendered}"
        );
        assert!(
            rendered.contains("src/frontend/core/parser/ast.rs:446"),
            "漂移报错还应带变体定义行，实际：{rendered}"
        );
    }

    // 复审 F1：基线外新增变体至少带变体定义行（stats.decl_line + AST_REL）
    #[test]
    fn test_mismatch_render_carries_definition_line_for_new_variant() {
        // Arrange：一个基线里没有的变体，且没有任何构造点
        let stats = stats_with(vec![variant("Phantom", 600)], Vec::new());

        // Act
        let mismatch = mismatches(&stats)
            .into_iter()
            .find(|m| m.variant == "Phantom")
            .expect("基线外新增变体必须报漂移");
        let rendered = mismatch.render();

        // Assert
        assert!(
            rendered.contains("src/frontend/core/parser/ast.rs:600"),
            "基线外新增变体必须带定义行，实际：{rendered}"
        );
        assert!(
            rendered.contains("基线外新增变体"),
            "该分支应保留「基线外新增变体」的措辞，实际：{rendered}"
        );
    }

    // 复审 F2：口径被改（归类从 ast_to_ast 改回 forward）必须被识别成「不同代」，
    // 且点明是哪一条、从什么改到什么——不能退化成一条数字漂移。
    #[test]
    fn test_caliber_drift_flags_class_change() {
        // Arrange：现行登记表把 without_fn_param_names 归回 Forward（其余条目与冻结口径一致，
        // 以便断言「只有这一处口径变化」）
        let registry: Vec<RebuildEntry> = REBUILD_FUNCTIONS
            .iter()
            .map(|entry| RebuildEntry {
                class: if entry.function == "without_fn_param_names" {
                    SiteClass::Forward
                } else {
                    entry.class
                },
                ..*entry
            })
            .collect();

        // Act
        let drift = caliber_drift(&registry, SCOUT_BASELINE_CALIBER);

        // Assert
        assert_eq!(drift.len(), 1, "类别变化应恰好报 1 条，实际 {drift:?}");
        assert!(
            drift[0].contains("类别不同") && drift[0].contains("without_fn_param_names"),
            "报错应点名类别不同与函数名，实际：{}",
            drift[0]
        );
        assert!(
            drift[0].contains("ast_to_ast") && drift[0].contains("forward"),
            "报错应给出「冻结口径 → 现行」两侧的类别键，实际：{}",
            drift[0]
        );
    }

    // 判据能绿：现行登记表与冻结快照同代时不报
    #[test]
    fn test_caliber_drift_clean_on_frozen_snapshot() {
        // Act
        let drift = caliber_drift(REBUILD_FUNCTIONS, SCOUT_BASELINE_CALIBER);

        // Assert
        assert!(
            drift.is_empty(),
            "生产登记表必须与冻结口径一致，实际 {drift:?}"
        );
        assert!(
            caliber_error().is_none(),
            "同代时不得产生「口径与基线不同代」报错"
        );
    }

    // 快照里多/少条目同样算口径变更（新增或删除重建函数）
    #[test]
    fn test_caliber_drift_flags_added_and_removed_entries() {
        // Arrange：登记表删掉 mono_to_ast_type，另加一个快照里没有的函数
        let registry = [
            RebuildEntry {
                file: crate::FUNCTION_REL,
                function: "substitute_type_ast",
                class: SiteClass::AstToAst,
                why: "test-only",
            },
            RebuildEntry {
                file: crate::FUNCTION_REL,
                function: "brand_new_rebuild",
                class: SiteClass::AstToAst,
                why: "test-only：模拟新增重建函数",
            },
        ];

        // Act
        let drift = caliber_drift(&registry, SCOUT_BASELINE_CALIBER);

        // Assert
        assert!(
            drift
                .iter()
                .any(|line| line.contains("登记表新增") && line.contains("brand_new_rebuild")),
            "新增的重建函数应被点名，实际 {drift:?}"
        );
        assert!(
            drift.iter().any(|line| line.contains("mono_to_ast_type")),
            "快照里有而登记表删掉的函数应被点名，实际 {drift:?}"
        );
    }
}
