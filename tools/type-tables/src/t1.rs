//! T1：`ast::Type` 的构造点分列与零构造清单。
//!
//! 三个数字必须分开报（03 §5.7 + 6.0 勘察的裁决）：前向零构造 / 全局零构造 / 反向桥重建。
//!
//! **WBS 6.3 之后的现状**：03 §5.2 的 13 个死变体已删除，`ast::Type` 收敛为 13 变体，
//! 因此 `REGISTERED_ZERO_FORWARD` 与 `DEAD_VARIANTS_03` 均为空表——**不再有零构造变体**。
//! 空表不是「门禁失效」：登记机制（`RegisteredZeroVariant` / `FallbackExemption`）与
//! 注入式核对（`*_with(registry)`）保留，供 6.6 转硬后对新增变体重新登记。
//!
//! **兜底豁免是逐条登记的（F1）**：只有登记表里显式登记的兜底点才从前向计数里扣除；
//! 未登记的通配臂构造点**照常计入前向**——否则真实构造点会隐身，门禁仍报「名单一致」。

use std::collections::BTreeSet;

use crate::baseline;
use crate::scan::{ScanResult, Site, SiteClass, VariantDecl};

/// 03 §5.2 的删除名单（前向口径的零构造变体）。
///
/// **WBS 6.3 已把这 13 个变体从 `ast::Type` 删除**，故此表清空：它只服务于
/// 「13 名单内」的交叉统计（T1.5 的标记），清空后该类统计恒为 0 变体 / 0 点。
pub const DEAD_VARIANTS_03: &[&str] = &[];

/// 一条**已登记的通配臂兜底豁免**（1 条 = 1 点）。
///
/// 只豁免「通配臂内的前向构造点」这一结构形态，不按行号（行号会漂移）；文件必须一致。
pub struct FallbackExemption {
    /// 该兜底点所在文件（仓库相对路径）
    pub file: &'static str,
    /// 为什么它可以被豁免（含消除它的任务项）
    pub why: &'static str,
}

/// 已登记的前向零构造变体（白名单）。6.6 转硬后：新增前向零构造变体必须删除或在此登记理由；
/// **兜底豁免同样必须逐条登记**，未登记的兜底点不予豁免。
pub struct RegisteredZeroVariant {
    pub name: &'static str,
    pub reason: &'static str,
    /// 已登记的通配臂兜底豁免点（空 = 该变体不豁免任何兜底点）
    pub exemptions: &'static [FallbackExemption],
}

/// 前向零构造变体的登记表（白名单 + 逐条兜底豁免）。
///
/// **WBS 6.3 清空**：13 个死变体已从 `ast::Type` 删除，实测前向零构造为 0 个，
/// 故本表为空——旧登记（含 `Void` 的 `parser/statements/types.rs` 错误兜底豁免）
/// 全部失效并已删除，否则 T1.7 会报「登记但已不成立」与「豁免未命中」。
/// 6.6 转硬后若新增前向零构造变体，在此登记理由（兜底点还须逐条给 `FallbackExemption`）。
pub const REGISTERED_ZERO_FORWARD: &[RegisteredZeroVariant] = &[];

/// 单变体的三类计数
#[derive(Debug, Clone)]
pub struct VariantStats {
    pub name: String,
    pub decl_line: usize,
    pub forward: Vec<Site>,
    pub ast_to_ast: Vec<Site>,
    pub reverse_from: Vec<Site>,
    pub reverse_mono: Vec<Site>,
}

impl VariantStats {
    fn from_decl(decl: &VariantDecl) -> Self {
        Self {
            name: decl.name.clone(),
            decl_line: decl.line,
            forward: Vec::new(),
            ast_to_ast: Vec::new(),
            reverse_from: Vec::new(),
            reverse_mono: Vec::new(),
        }
    }

    fn push(
        &mut self,
        site: Site,
    ) {
        match site.class {
            SiteClass::Forward => self.forward.push(site),
            SiteClass::AstToAst => self.ast_to_ast.push(site),
            SiteClass::ReverseFromIrType => self.reverse_from.push(site),
            SiteClass::ReverseMonoToAst => self.reverse_mono.push(site),
        }
    }

    /// 前向计数：**原始**（含未豁免的通配臂构造点）
    pub fn forward_raw(&self) -> usize {
        self.forward.len()
    }

    /// 通配臂内的前向构造点（结构事实；是否豁免由登记表决定）
    pub fn forward_wildcard_sites(&self) -> Vec<&Site> {
        self.forward
            .iter()
            .filter(|site| site.wildcard_arm)
            .collect()
    }

    pub fn reverse_total(&self) -> usize {
        self.reverse_from.len() + self.reverse_mono.len()
    }

    pub fn total(&self) -> usize {
        self.forward.len() + self.ast_to_ast.len() + self.reverse_total()
    }
}

/// 一条登记的豁免点及其命中情况
pub struct ExemptionOutcome<'a> {
    pub variant: &'static str,
    pub entry: &'static FallbackExemption,
    /// 命中的兜底点；`None` = 登记已失效（该文件里找不到这个变体的通配臂前向构造点）
    pub site: Option<&'a Site>,
}

/// T1.7 白名单核对结果（渲染与 build.rs 告警共用同一口径）
#[derive(Debug, Default)]
pub struct WhitelistCheck {
    pub registered: usize,
    /// 实测前向零构造（扣除已登记豁免后）的变体
    pub measured_zero: Vec<String>,
    /// 实测前向零构造但**未登记**的变体
    pub unregistered: Vec<String>,
    /// 已登记但前向有效计数 > 0 的变体（名单与实现不一致）
    pub broken: Vec<String>,
    /// 已登记但**未命中**的豁免点（兜底点消失后必须同步删除登记）
    pub stale_exemptions: Vec<String>,
    /// 未登记的通配臂兜底点（不予豁免，照常计入前向）
    pub unregistered_fallbacks: Vec<String>,
}

impl WhitelistCheck {
    pub fn is_consistent(&self) -> bool {
        self.unregistered.is_empty()
            && self.broken.is_empty()
            && self.stale_exemptions.is_empty()
            && self.unregistered_fallbacks.is_empty()
    }
}

/// T1 报告
#[derive(Debug)]
pub struct ForwardReport {
    pub scanned_files: usize,
    pub stats: Vec<VariantStats>,
    /// 别名解析结果（别名 → 声明它的文件），供报告展示
    pub aliases: std::collections::BTreeMap<String, Vec<String>>,
}

impl ForwardReport {
    pub fn new(scan: ScanResult) -> Self {
        let mut stats: Vec<VariantStats> =
            scan.variants.iter().map(VariantStats::from_decl).collect();
        let index: std::collections::BTreeMap<String, usize> = stats
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.clone(), i))
            .collect();
        for site in scan.sites {
            if let Some(&i) = index.get(&site.variant) {
                stats[i].push(site);
            }
        }
        Self {
            scanned_files: scan.scanned_files,
            stats,
            aliases: scan.aliases,
        }
    }

    /// 逐条登记项的命中情况（按登记表顺序；每条登记只消费一个点）
    pub fn exemption_outcomes(&self) -> Vec<ExemptionOutcome<'_>> {
        self.exemption_outcomes_with(REGISTERED_ZERO_FORWARD)
    }

    /// 同上，登记表可注入（6.3 之后生产登记表为空，机制仍须可测）
    fn exemption_outcomes_with(
        &self,
        registry: &'static [RegisteredZeroVariant],
    ) -> Vec<ExemptionOutcome<'_>> {
        let mut consumed: BTreeSet<(String, usize)> = BTreeSet::new();
        let mut out = Vec::new();
        for registered in registry {
            let stats = self.stats.iter().find(|s| s.name == registered.name);
            for entry in registered.exemptions {
                let hit = stats.and_then(|s| {
                    s.forward.iter().find(|site| {
                        site.wildcard_arm
                            && site.file == entry.file
                            && !consumed.contains(&(site.file.clone(), site.line))
                    })
                });
                if let Some(site) = hit {
                    consumed.insert((site.file.clone(), site.line));
                }
                out.push(ExemptionOutcome {
                    variant: registered.name,
                    entry,
                    site: hit,
                });
            }
        }
        out
    }

    /// 该变体被豁免的兜底点数（= 命中登记项的点数）
    pub fn exempt_count(
        &self,
        stats: &VariantStats,
    ) -> usize {
        self.exempt_count_with(stats, REGISTERED_ZERO_FORWARD)
    }

    fn exempt_count_with(
        &self,
        stats: &VariantStats,
        registry: &'static [RegisteredZeroVariant],
    ) -> usize {
        self.exemption_outcomes_with(registry)
            .iter()
            .filter(|outcome| outcome.variant == stats.name && outcome.site.is_some())
            .count()
    }

    /// 前向有效计数 = 原始前向 − 已登记豁免点数
    pub fn forward_effective(
        &self,
        stats: &VariantStats,
    ) -> usize {
        self.forward_effective_with(stats, REGISTERED_ZERO_FORWARD)
    }

    fn forward_effective_with(
        &self,
        stats: &VariantStats,
        registry: &'static [RegisteredZeroVariant],
    ) -> usize {
        stats
            .forward_raw()
            .saturating_sub(self.exempt_count_with(stats, registry))
    }

    /// 前向零构造（扣除已登记豁免后为 0）
    pub fn forward_zero(&self) -> Vec<&VariantStats> {
        self.forward_zero_with(REGISTERED_ZERO_FORWARD)
    }

    fn forward_zero_with(
        &self,
        registry: &'static [RegisteredZeroVariant],
    ) -> Vec<&VariantStats> {
        self.stats
            .iter()
            .filter(|stats| self.forward_effective_with(stats, registry) == 0)
            .collect()
    }

    /// 全局零构造（三类合计为 0）
    pub fn global_zero(&self) -> Vec<&VariantStats> {
        self.stats.iter().filter(|s| s.total() == 0).collect()
    }

    /// 未被豁免的通配臂兜底点（这些照常计入前向 → 变体不得算前向零构造）
    pub fn unregistered_fallbacks(&self) -> Vec<&Site> {
        self.unregistered_fallbacks_with(REGISTERED_ZERO_FORWARD)
    }

    fn unregistered_fallbacks_with(
        &self,
        registry: &'static [RegisteredZeroVariant],
    ) -> Vec<&Site> {
        let consumed: BTreeSet<(String, usize)> = self
            .exemption_outcomes_with(registry)
            .iter()
            .filter_map(|outcome| outcome.site)
            .map(|site| (site.file.clone(), site.line))
            .collect();
        self.stats
            .iter()
            .flat_map(|stats| stats.forward.iter())
            .filter(|site| site.wildcard_arm && !consumed.contains(&(site.file.clone(), site.line)))
            .collect()
    }

    /// T1.7 白名单核对（渲染、--json、build.rs 告警共用）
    pub fn whitelist_check(&self) -> WhitelistCheck {
        self.whitelist_check_with(REGISTERED_ZERO_FORWARD)
    }

    /// 同上，登记表可注入（生产登记表为空时仍可验证豁免/失效机制）
    fn whitelist_check_with(
        &self,
        registry: &'static [RegisteredZeroVariant],
    ) -> WhitelistCheck {
        let measured_zero: Vec<String> = self
            .forward_zero_with(registry)
            .iter()
            .map(|stats| stats.name.clone())
            .collect();
        let mut check = WhitelistCheck {
            registered: registry.len(),
            unregistered: measured_zero
                .iter()
                .filter(|name| !registry.iter().any(|r| r.name == name.as_str()))
                .cloned()
                .collect(),
            broken: registry
                .iter()
                .filter(
                    |registered| match self.stats.iter().find(|s| s.name == registered.name) {
                        Some(stats) => self.forward_effective_with(stats, registry) > 0,
                        None => true,
                    },
                )
                .map(|registered| registered.name.to_string())
                .collect(),
            stale_exemptions: self
                .exemption_outcomes_with(registry)
                .iter()
                .filter(|outcome| outcome.site.is_none())
                .map(|outcome| format!("{}({})", outcome.variant, outcome.entry.file))
                .collect(),
            unregistered_fallbacks: self
                .unregistered_fallbacks()
                .iter()
                .map(|site| format!("{}@{}", site.variant, site.location()))
                .collect(),
            measured_zero,
        };
        check.measured_zero.sort();
        check.unregistered.sort();
        check.broken.sort();
        check.stale_exemptions.sort();
        check.unregistered_fallbacks.sort();
        check
    }

    fn dead_list_points(&self) -> Vec<(&VariantStats, usize)> {
        self.stats
            .iter()
            .filter(|s| DEAD_VARIANTS_03.contains(&s.name.as_str()))
            .map(|s| (s, s.ast_to_ast.len()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("── 口径定义（T1，先读这里）────────────────────────────────────────────\n");
        out.push_str(
            "构造点 = **表达式位置**的 Type::<变体> 路径（Expr::Path / Expr::Struct）。\n",
        );
        out.push_str(
            "         模式位置不计（match 臂左侧 / matches! 第 2 参 / if let / 参数模式）。\n",
        );
        out.push_str("         宏体不解析；tests 目录整棵排除，内联测试模块按 cfg 谓词判定（要求 test 成立的才排除：\n");
        out.push_str("         即 cfg(test) / all(test, …)；cfg(not(test)) 与 cfg(feature = \"test\") 仍扫描）。\n");
        out.push_str(
            "识别为 ast::Type 的限定名：基础名 Type，以及本文件按 use 声明解析出的别名（如\n",
        );
        out.push_str("         use …::ast::Type as AstType / use …::ir::Type as IrType）；泛型形参同名时以形参为准（遮蔽）。\n");
        out.push_str("三类（+两处反向桥）不得混算：\n");
        out.push_str(
            "  1) parser 前向        ：不在下面任何重建位置的构造点（当前全部落在 parser）\n",
        );
        out.push_str(
            "  2) AST→AST 重建       ：middle/passes/mono/function.rs 的 substitute_type_ast\n",
        );
        out.push_str(
            "                          与 substitute_type_in_ast——把已有 AST 类型重造一份\n",
        );
        out.push_str(
            "                          （区间由 AST 派生，不在散文里写死行号，见 WBS 6.6）\n",
        );
        out.push_str(
            "  3) 反向桥 A           ：middle/core/bytecode.rs 的 impl From<MonoType> for IrType\n",
        );
        out.push_str(
            "     反向桥 B           ：function.rs 的嵌套 fn mono_to_ast_type（MonoType → AST）\n",
        );
        out.push_str(
            "「前向零构造」= 前向有效计数为 0 = 原始前向点数 − **已登记**的通配臂兜底豁免点数；\n",
        );
        out.push_str("         豁免逐条登记在 REGISTERED_ZERO_FORWARD（WBS 6.3 后为空表：零构造变体已删除），\n");
        out.push_str(
            "         未登记的兜底点不予豁免、照常计入前向；登记失效（兜底点消失）由 T1.7 报警。\n",
        );
        out.push_str("「全局零构造」= 三类合计为 0。两个数字分开报，不得混用。\n");
        out.push('\n');

        out.push_str(&format!(
            "── T1.1 扫描面 ────────────────────────────────────────────────────────\n\
             非测试 .rs 文件：{}\n\
             ast::Type 变体：{}（定义行 {} - {}）\n",
            self.scanned_files,
            self.stats.len(),
            self.stats.first().map(|s| s.decl_line).unwrap_or(0),
            self.stats.last().map(|s| s.decl_line).unwrap_or(0),
        ));
        if self.aliases.is_empty() {
            out.push_str("别名解析（按 use 声明）：无\n\n");
        } else {
            out.push_str(&format!(
                "别名解析（按 use 声明，F4）：{} 个别名 —— {}\n\n",
                self.aliases.len(),
                self.aliases
                    .iter()
                    .map(|(alias, files)| format!("{} ← {} 文件", alias, files.len()))
                    .collect::<Vec<_>>()
                    .join("；")
            ));
        }

        out.push_str("── T1.2 逐变体三类构造点 ──────────────────────────────────────────────\n");
        out.push_str("（前向=原始计数；括号内为「通配臂兜底/已登记豁免」点数，有效=扣豁免后）\n");
        out.push_str(&format!(
            "{:<13} {:>4} {:>16} {:>10} {:>10} {:>8}\n",
            "变体", "定义", "前向（兜底/豁免→有效）", "AST→AST", "反向桥", "合计"
        ));
        for stats in &self.stats {
            let wildcard = stats.forward_wildcard_sites().len();
            let exempt = self.exempt_count(stats);
            let effective = self.forward_effective(stats);
            let forward = if wildcard > 0 || exempt > 0 {
                format!(
                    "{}({}/{})→{}",
                    stats.forward_raw(),
                    wildcard,
                    exempt,
                    effective
                )
            } else {
                stats.forward_raw().to_string()
            };
            out.push_str(&format!(
                "{:<13} {:>4} {:>16} {:>10} {:>10} {:>8}\n",
                stats.name,
                stats.decl_line,
                forward,
                stats.ast_to_ast.len(),
                stats.reverse_total(),
                stats.total()
            ));
        }
        out.push('\n');

        out.push_str("── T1.3 前向零构造清单 ────────────────────────────────────────────────\n");
        out.push_str(&format!("数量：{}\n", self.forward_zero().len()));
        for stats in self.forward_zero() {
            let exempt = self.exempt_count(stats);
            let extra = if exempt > 0 {
                format!(
                    "（原始前向 {} 点，其中 {} 点是已登记豁免：{}）",
                    stats.forward_raw(),
                    exempt,
                    self.exemption_outcomes()
                        .iter()
                        .filter(|o| o.variant == stats.name && o.site.is_some())
                        .map(|o| o.site.map(|s| s.location()).unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join("；")
                )
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  {:<12} 合计 {}{}\n",
                stats.name,
                stats.total(),
                extra
            ));
        }
        out.push('\n');
        out.push_str("── T1.3b 全局零构造清单（三类合计为 0）────────────────────────────────\n");
        out.push_str(&format!("数量：{}\n", self.global_zero().len()));
        for stats in self.global_zero() {
            out.push_str(&format!("  {}\n", stats.name));
        }
        out.push('\n');

        out.push_str("── T1.4 反向桥重建点分列 ──────────────────────────────────────────────\n");
        out.push_str(&format!(
            "反向桥 A（impl From<MonoType> for IrType，{}）：\n",
            crate::BYTECODE_REL
        ));
        let a: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|stats| !stats.reverse_from.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个\n",
            a.len(),
            a.iter()
                .map(|stats| stats.reverse_from.len())
                .sum::<usize>()
        ));
        for stats in &a {
            out.push_str(&format!(
                "  {:<12} {}\n",
                stats.name,
                stats
                    .reverse_from
                    .iter()
                    .map(|site| format!(":{}", site.line))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push_str(&format!(
            "反向桥 B（fn mono_to_ast_type，{} 的 substitute_type_in_ast 内嵌套 fn）：\n",
            crate::FUNCTION_REL
        ));
        let b: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|stats| !stats.reverse_mono.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个\n",
            b.len(),
            b.iter()
                .map(|stats| stats.reverse_mono.len())
                .sum::<usize>()
        ));
        for stats in &b {
            out.push_str(&format!(
                "  {:<12} {}\n",
                stats.name,
                stats
                    .reverse_mono
                    .iter()
                    .map(|site| format!(":{}", site.line))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push('\n');

        out.push_str("── T1.5 AST→AST 重建点分列 ────────────────────────────────────────────\n");
        let c: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|stats| !stats.ast_to_ast.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个（6.3 已删除 5 变体 / 10 点；本列是剩余重建点）\n",
            c.len(),
            c.iter().map(|stats| stats.ast_to_ast.len()).sum::<usize>()
        ));
        for stats in &c {
            let mark = if DEAD_VARIANTS_03.contains(&stats.name.as_str()) {
                "  ← 13 名单内"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {:<12} {}{}\n",
                stats.name,
                stats
                    .ast_to_ast
                    .iter()
                    .map(|site| format!(":{}", site.line))
                    .collect::<Vec<_>>()
                    .join(" "),
                mark
            ));
        }
        let dead_points: usize = self.dead_list_points().iter().map(|(_, n)| *n).sum();
        out.push_str(&format!(
            "  其中 WBS 6.3 删除名单（DEAD_VARIANTS_03，已清空）：{} 个变体 / {} 点\n\n",
            self.dead_list_points().len(),
            dead_points
        ));

        out.push_str(&baseline::render_comparison(&self.stats));

        out.push_str("── T1.7 白名单核对（6.6 转硬的前提）────────────────────────────────────\n");
        let check = self.whitelist_check();
        out.push_str(&format!(
            "已登记 {} 个；实测前向零构造 {} 个；未登记 {} 个；登记但已不成立 {} 个；\n\
             登记豁免点 {} 条（命中 {} 条）；未登记的通配臂兜底点 {} 个\n",
            check.registered,
            check.measured_zero.len(),
            check.unregistered.len(),
            check.broken.len(),
            REGISTERED_ZERO_FORWARD
                .iter()
                .map(|r| r.exemptions.len())
                .sum::<usize>(),
            self.exemption_outcomes()
                .iter()
                .filter(|o| o.site.is_some())
                .count(),
            check.unregistered_fallbacks.len(),
        ));
        if check.is_consistent() {
            out.push_str("  结论：名单一致（未登记 0 / 失效 0 / 豁免未命中 0 / 未登记兜底 0）\n");
        } else {
            if !check.unregistered.is_empty() {
                out.push_str(&format!("  未登记的零构造变体：{:?}\n", check.unregistered));
            }
            if !check.broken.is_empty() {
                out.push_str(&format!(
                    "  **登记但已不成立**（前向有效计数 > 0）：{:?}\n",
                    check.broken
                ));
            }
            if !check.stale_exemptions.is_empty() {
                out.push_str(&format!(
                    "  **登记豁免点未命中**（兜底点已消失，登记须同步删除）：{:?}\n",
                    check.stale_exemptions
                ));
            }
            if !check.unregistered_fallbacks.is_empty() {
                out.push_str(&format!(
                    "  **未登记的通配臂兜底点**（不予豁免，已计入前向）：{:?}\n",
                    check.unregistered_fallbacks
                ));
            }
            out.push_str("  结论：名单不一致（详见上方清单）\n");
        }
        out.push('\n');
        out
    }

    /// `build.rs` 用的 warning 行（只报告，不失败）
    pub fn warnings(&self) -> Vec<String> {
        let check = self.whitelist_check();
        let mut out = Vec::new();
        let global: Vec<&str> = self
            .global_zero()
            .iter()
            .map(|stats| stats.name.as_str())
            .collect();
        let reverse_a = self
            .stats
            .iter()
            .filter(|stats| !stats.reverse_from.is_empty())
            .count();
        let reverse_b = self
            .stats
            .iter()
            .filter(|stats| !stats.reverse_mono.is_empty())
            .count();
        let ast_to_ast_variants = self
            .stats
            .iter()
            .filter(|stats| !stats.ast_to_ast.is_empty())
            .count();
        let dead_points: usize = self.dead_list_points().iter().map(|(_, n)| *n).sum();
        out.push(format!(
            "type-tables T1（只报告，6.6 才转硬）：扫描 {} 个非测试 .rs；ast::Type {} 变体；前向零构造 {}；全局零构造 {}〔{}〕；反向桥 A {} 变体；反向桥 B {} 变体；AST→AST 重建 {} 变体（6.3 删除名单内 {} 点）",
            self.scanned_files,
            self.stats.len(),
            check.measured_zero.len(),
            global.len(),
            global.join("/"),
            reverse_a,
            reverse_b,
            ast_to_ast_variants,
            dead_points,
        ));

        if !check.unregistered.is_empty() {
            out.push(format!(
                "type-tables T1：未登记的前向零构造变体 {:?} —— 新增零构造变体必须删除或登记理由（03 §5.7）",
                check.unregistered
            ));
        }
        if !check.broken.is_empty() {
            out.push(format!(
                "type-tables T1：登记的零构造变体已出现前向构造点 {:?} —— 名单与实现不一致，需更新（03 §5.7）",
                check.broken
            ));
        }
        if !check.stale_exemptions.is_empty() {
            out.push(format!(
                "type-tables T1：登记的通配臂兜底豁免点未命中 {:?} —— 兜底点消失后登记须同步删除（03 §5.7）",
                check.stale_exemptions
            ));
        }
        if !check.unregistered_fallbacks.is_empty() {
            out.push(format!(
                "type-tables T1：存在未登记的通配臂兜底点 {:?} —— 不予豁免（已计入前向）；如确需豁免须逐条登记（03 §5.7）",
                check.unregistered_fallbacks
            ));
        }
        out
    }

    /// `--json` 的结构化结果
    pub fn to_json(&self) -> serde_json::Value {
        let check = self.whitelist_check();
        let variants: Vec<serde_json::Value> = self
            .stats
            .iter()
            .map(|stats| {
                serde_json::json!({
                    "name": stats.name,
                    "def_line": stats.decl_line,
                    "forward": stats.forward_raw(),
                    "forward_wildcard": stats.forward_wildcard_sites().len(),
                    "forward_exempt": self.exempt_count(stats),
                    "forward_effective": self.forward_effective(stats),
                    "ast_to_ast": stats.ast_to_ast.len(),
                    "reverse_from_ir_type": stats.reverse_from.len(),
                    "reverse_mono_to_ast": stats.reverse_mono.len(),
                    "total": stats.total(),
                })
            })
            .collect();
        let sites: Vec<serde_json::Value> = self
            .stats
            .iter()
            .flat_map(|stats| {
                stats
                    .forward
                    .iter()
                    .chain(stats.ast_to_ast.iter())
                    .chain(stats.reverse_from.iter())
                    .chain(stats.reverse_mono.iter())
            })
            .map(|site| {
                serde_json::json!({
                    "variant": site.variant,
                    "file": site.file,
                    "line": site.line,
                    "class": site.class.label(),
                    "wildcard_arm": site.wildcard_arm,
                })
            })
            .collect();
        let exemptions: Vec<serde_json::Value> = self
            .exemption_outcomes()
            .iter()
            .map(|outcome| {
                serde_json::json!({
                    "variant": outcome.variant,
                    "file": outcome.entry.file,
                    "why": outcome.entry.why,
                    "hit": outcome.site.map(|site| site.location()),
                })
            })
            .collect();
        serde_json::json!({
            "scanned_files": self.scanned_files,
            "variant_count": self.stats.len(),
            "forward_zero": self.forward_zero().iter().map(|stats| stats.name.clone()).collect::<Vec<_>>(),
            "global_zero": self.global_zero().iter().map(|stats| stats.name.clone()).collect::<Vec<_>>(),
            "aliases": self.aliases,
            "whitelist": {
                "consistent": check.is_consistent(),
                "unregistered": check.unregistered,
                "broken": check.broken,
                "stale_exemptions": check.stale_exemptions,
                "unregistered_fallbacks": check.unregistered_fallbacks,
            },
            "exemptions": exemptions,
            "variants": variants,
            "sites": sites,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 生产登记表已随 WBS 6.3 清空（不再有零构造变体）；机制测试用合成登记表：
    /// Void 带一条兜底豁免（验命中/失效），Int 无豁免（验「登记但已不成立」）。
    const TEST_FALLBACK: &[FallbackExemption] = &[FallbackExemption {
        file: "src/frontend/core/parser/statements/types.rs",
        why: "test-only：验证兜底豁免命中与失效报警",
    }];
    const TEST_REGISTRY: &[RegisteredZeroVariant] = &[
        RegisteredZeroVariant {
            name: "Void",
            reason: "test-only",
            exemptions: TEST_FALLBACK,
        },
        RegisteredZeroVariant {
            name: "Int",
            reason: "test-only（无豁免）",
            exemptions: &[],
        },
    ];

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
        variant: &str,
        class: SiteClass,
        wildcard_arm: bool,
    ) -> Site {
        Site {
            variant: variant.to_string(),
            file: "src/fixture.rs".to_string(),
            line: 1,
            class,
            wildcard_arm,
        }
    }

    fn report(
        variants: Vec<VariantDecl>,
        sites: Vec<Site>,
    ) -> ForwardReport {
        ForwardReport::new(ScanResult {
            scanned_files: 1,
            variants,
            sites,
            aliases: Default::default(),
        })
    }

    // 三类分列是 T1 的核心纪律：混算会把 03 §5.2 的删除名单抹平。
    #[test]
    fn test_variant_stats_keeps_three_classes_separate() {
        // Arrange
        let mut stats = VariantStats::from_decl(&variant("AssocType", 480));
        stats.push(site("AssocType", SiteClass::AstToAst, false));
        stats.push(site("AssocType", SiteClass::AstToAst, false));

        // Assert
        assert_eq!(stats.forward_raw(), 0, "AST→AST 重建点不得计入前向构造点");
        assert_eq!(stats.ast_to_ast.len(), 2, "AST→AST 应各自归类并计数");
        assert_eq!(stats.reverse_total(), 0, "反向桥计数不得被 AST→AST 污染");
    }

    // 已登记的兜底点（Void 的 parser 错误兜底）才豁免
    #[test]
    fn test_registered_fallback_is_exempt() {
        // Arrange：Void 的通配臂兜底点，文件与登记一致
        let mut void_site = site("Void", SiteClass::Forward, true);
        void_site.file = "src/frontend/core/parser/statements/types.rs".to_string();
        let report = report(vec![variant("Void", 455)], vec![void_site]);

        // Act
        let stats = &report.stats[0];

        // Assert
        assert_eq!(stats.forward_raw(), 1, "原始前向计数应保留兜底点");
        assert_eq!(
            report.forward_effective_with(stats, TEST_REGISTRY),
            0,
            "已登记的兜底点应被豁免（有效计数 0）"
        );
        assert!(
            report
                .whitelist_check_with(TEST_REGISTRY)
                .stale_exemptions
                .is_empty(),
            "命中登记项时不得报「豁免未命中」"
        );
    }

    // F1 回归（CE6）：未登记的通配臂兜底点必须照常计入前向，且 T1.7 不得再报「名单一致」
    #[test]
    fn test_unregistered_wildcard_fallback_counts_as_forward() {
        // Arrange：生产代码在通配臂里构造了 Type::Int(64)；Int 未登记任何豁免点
        let report = report(
            vec![variant("Int", 449)],
            vec![site("Int", SiteClass::Forward, true)],
        );

        // Act
        let stats = &report.stats[0];
        let check = report.whitelist_check_with(TEST_REGISTRY);

        // Assert
        assert_eq!(
            report.forward_effective_with(stats, TEST_REGISTRY),
            1,
            "Int 未登记豁免点，通配臂里的 Type::Int 必须计入前向（否则门禁漏报真实构造点）"
        );
        assert!(
            report.forward_zero_with(TEST_REGISTRY).is_empty(),
            "Int 有真实前向构造点，不得再列为「前向零构造」"
        );
        assert!(
            check.broken.contains(&"Int".to_string()),
            "T1.7 必须报「登记但已不成立」，不得再报名单一致"
        );
        assert!(
            !check.is_consistent(),
            "存在未登记兜底点时白名单核对必须判不一致"
        );
        assert!(
            !check.unregistered_fallbacks.is_empty(),
            "未登记的兜底点应单列（不予豁免）"
        );
    }

    // 登记失效：兜底点消失后（如 6.4 消除），登记得报「未命中」而不是静默通过
    #[test]
    fn test_stale_exemption_is_reported() {
        // Arrange：扫描里没有 Void 的任何前向点，但登记表里仍有 Void 的兜底豁免
        let report = report(vec![variant("Void", 455)], Vec::new());

        // Act
        let check = report.whitelist_check_with(TEST_REGISTRY);

        // Assert
        assert_eq!(
            check.stale_exemptions.len(),
            1,
            "登记了但未命中的豁免点必须单列（兜底点消失后要同步删登记）"
        );
        assert!(!check.is_consistent(), "豁免未命中时白名单核对必须判不一致");
    }
}
