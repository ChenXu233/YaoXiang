//! T1：`ast::Type` 的构造点分列与零构造清单。
//!
//! 三个数字必须分开报（03 §5.7 + 6.0 勘察的裁决）：
//! - **前向零构造**（parser 前向计数为 0）= 13 个：Int / Float / Char / String / Bytes / Bool /
//!   Void / Union / Enum / Option / Result / Sum / AssocType
//! - **全局零构造**（三类合计为 0）= 1 个：仅 `Enum`
//! - 反向桥重建（A: `From<MonoType> for IrType` 9 变体；B: `mono_to_ast_type` 8 变体）单独分列
//!
//! `Void` 的唯一前向点是 `_ => (Vec::new(), Type::Void)` 错误兜底（03 §5.2 #8 / 6.4 #12 消除），
//! 本模块按「通配 match 臂内的兜底点」结构性识别并单列，故 Void 计入前向零构造但仍保留原始计数。

use crate::baseline;
use crate::scan::{ScanResult, Site, SiteClass, VariantDecl};

/// 03 §5.2 的删除名单（前向口径的零构造变体，13 个）——用于「13 名单内」的交叉统计。
pub const DEAD_VARIANTS_03: &[&str] = &[
    "Int",
    "Float",
    "Char",
    "String",
    "Bytes",
    "Bool",
    "Void",
    "Union",
    "Enum",
    "Option",
    "Result",
    "Sum",
    "AssocType",
];

/// 已登记的前向零构造变体（白名单）。6.6 转硬后：新增前向零构造变体必须删除或在此登记理由。
pub struct RegisteredZeroVariant {
    pub name: &'static str,
    pub reason: &'static str,
}

pub const REGISTERED_ZERO_FORWARD: &[RegisteredZeroVariant] = &[
    RegisteredZeroVariant {
        name: "Int",
        reason: "03 §5.2 #2：位宽由 from_builtin_name 的 Name 路径承载",
    },
    RegisteredZeroVariant {
        name: "Float",
        reason: "03 §5.2 #3：同 Int，位宽由 Name 路径承载",
    },
    RegisteredZeroVariant {
        name: "Char",
        reason: "03 §5.2 #4：走 Name 路径",
    },
    RegisteredZeroVariant {
        name: "String",
        reason: "03 §5.2 #5：已由 Generic{name:String} 表示；反向桥 A/B 各 1 点随 6.2/6.3 改写",
    },
    RegisteredZeroVariant {
        name: "Bytes",
        reason: "03 §5.2 #6：已由 Generic{name:Bytes} 表示；反向桥 A/B 各 1 点随 6.2/6.3 改写",
    },
    RegisteredZeroVariant {
        name: "Bool",
        reason: "03 §5.2 #7：走 Name 路径",
    },
    RegisteredZeroVariant {
        name: "Void",
        reason: "03 §5.2 #8：唯一前向点是错误兜底（_ => (…, Type::Void)），随 6.4 #12 消除",
    },
    RegisteredZeroVariant {
        name: "Union",
        reason: "03 §5.2 #11：合成类型走 Struct/Ptr；AST→AST 重建 2 点随 6.3 改写",
    },
    RegisteredZeroVariant {
        name: "Enum",
        reason: "03 §5.2 #12：枚举走 Struct + TypeBodyItem；唯一全局零构造变体",
    },
    RegisteredZeroVariant {
        name: "Option",
        reason: "03 §5.2 #15：走 Generic 路径；AST→AST 重建 2 点随 6.3 改写",
    },
    RegisteredZeroVariant {
        name: "Result",
        reason: "03 §5.2 #16：走 Generic 路径；AST→AST 重建 2 点随 6.3 改写",
    },
    RegisteredZeroVariant {
        name: "Sum",
        reason: "03 §5.2 #19：合成类型走 Struct；AST→AST 重建 2 点随 6.3 改写",
    },
    RegisteredZeroVariant {
        name: "AssocType",
        reason: "03 §5.2 #18 / D8：语法层无 :: 关联类型产出路径；前端零构造、AST→AST 2 点（口径按前向，不加豁免）",
    },
];

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

    /// 前向计数：**原始**（含通配臂兜底点）
    pub fn forward_raw(&self) -> usize {
        self.forward.len()
    }

    /// 前向计数：**扣除已登记的错误兜底点**（当前只影响 `Void`）
    pub fn forward_effective(&self) -> usize {
        self.forward.iter().filter(|s| !s.wildcard_arm).count()
    }

    pub fn reverse_total(&self) -> usize {
        self.reverse_from.len() + self.reverse_mono.len()
    }

    pub fn total(&self) -> usize {
        self.forward.len() + self.ast_to_ast.len() + self.reverse_total()
    }
}

/// T1 报告
#[derive(Debug)]
pub struct ForwardReport {
    pub scanned_files: usize,
    pub stats: Vec<VariantStats>,
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
        }
    }

    /// 前向零构造（扣除已登记兜底点后为 0）
    pub fn forward_zero(&self) -> Vec<&VariantStats> {
        self.stats
            .iter()
            .filter(|s| s.forward_effective() == 0)
            .collect()
    }

    /// 全局零构造（三类合计为 0）
    pub fn global_zero(&self) -> Vec<&VariantStats> {
        self.stats.iter().filter(|s| s.total() == 0).collect()
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
        out.push_str("         宏体不解析；tests 目录与 #[cfg(test)] mod tests 整棵排除。\n");
        out.push_str("识别为 ast::Type 的限定名：Type / AstType / IrType，以及文件内 import 了 Type as T 时的 T。\n");
        out.push_str("三类（+两处反向桥）不得混算：\n");
        out.push_str(
            "  1) parser 前向        ：不在下面任何重建位置的构造点（当前全部落在 parser）\n",
        );
        out.push_str("  2) AST→AST 重建       ：middle/passes/mono/function.rs 的 substitute_type_ast（320-497）\n");
        out.push_str("                          与 substitute_type_in_ast（499-655）——把已有 AST 类型重造一份\n");
        out.push_str(
            "  3) 反向桥 A           ：middle/core/bytecode.rs 的 impl From<MonoType> for IrType\n",
        );
        out.push_str(
            "     反向桥 B           ：function.rs 的嵌套 fn mono_to_ast_type（MonoType → AST）\n",
        );
        out.push_str("「前向零构造」= 第 1 类计数为 0（Void 的唯一前向点是 _ => (…, Type::Void) 错误兜底，\n");
        out.push_str(
            "                 已登记并单列，故仍计入前向零构造）；「全局零构造」= 三类合计为 0。\n",
        );
        out.push_str("两个数字分开报，不得混用。\n");
        out.push('\n');

        out.push_str(&format!(
            "── T1.1 扫描面 ────────────────────────────────────────────────────────\n\
             非测试 .rs 文件：{}\n\
             ast::Type 变体：{}（定义行 {} - {}）\n\n",
            self.scanned_files,
            self.stats.len(),
            self.stats.first().map(|s| s.decl_line).unwrap_or(0),
            self.stats.last().map(|s| s.decl_line).unwrap_or(0),
        ));

        out.push_str("── T1.2 逐变体三类构造点 ──────────────────────────────────────────────\n");
        out.push_str("（前向=原始计数，括号内为其中「通配臂兜底」点数；反向=反向桥 A + B）\n");
        out.push_str(&format!(
            "{:<13} {:>4} {:>12} {:>10} {:>10} {:>8}\n",
            "变体", "定义", "前向", "AST→AST", "反向桥", "合计"
        ));
        for s in &self.stats {
            let wildcard = s.forward.iter().filter(|x| x.wildcard_arm).count();
            let forward = if wildcard > 0 {
                format!("{}(兜底{})", s.forward_raw(), wildcard)
            } else {
                s.forward_raw().to_string()
            };
            out.push_str(&format!(
                "{:<13} {:>4} {:>12} {:>10} {:>10} {:>8}\n",
                s.name,
                s.decl_line,
                forward,
                s.ast_to_ast.len(),
                s.reverse_total(),
                s.total()
            ));
        }
        out.push('\n');

        out.push_str("── T1.3 前向零构造清单 ────────────────────────────────────────────────\n");
        out.push_str(&format!("数量：{}\n", self.forward_zero().len()));
        for s in self.forward_zero() {
            let extra = if s.forward_raw() > 0 {
                format!(
                    "（原始前向 {} 点，均为通配臂兜底：{}）",
                    s.forward_raw(),
                    s.forward
                        .iter()
                        .map(|x| x.location())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                String::new()
            };
            out.push_str(&format!("  {:<10} 合计 {}{}\n", s.name, s.total(), extra));
        }
        out.push('\n');
        out.push_str("── T1.3b 全局零构造清单（三类合计为 0）────────────────────────────────\n");
        out.push_str(&format!("数量：{}\n", self.global_zero().len()));
        for s in self.global_zero() {
            out.push_str(&format!("  {}\n", s.name));
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
            .filter(|s| !s.reverse_from.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个\n",
            a.len(),
            a.iter().map(|s| s.reverse_from.len()).sum::<usize>()
        ));
        for s in &a {
            out.push_str(&format!(
                "  {:<10} {}\n",
                s.name,
                s.reverse_from
                    .iter()
                    .map(|x| format!(":{}", x.line))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push_str(&format!(
            "反向桥 B（fn mono_to_ast_type，{} 499-655 内嵌套 fn）：\n",
            crate::FUNCTION_REL
        ));
        let b: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|s| !s.reverse_mono.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个\n",
            b.len(),
            b.iter().map(|s| s.reverse_mono.len()).sum::<usize>()
        ));
        for s in &b {
            out.push_str(&format!(
                "  {:<10} {}\n",
                s.name,
                s.reverse_mono
                    .iter()
                    .map(|x| format!(":{}", x.line))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        out.push('\n');

        out.push_str("── T1.5 AST→AST 重建点分列 ────────────────────────────────────────────\n");
        let c: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|s| !s.ast_to_ast.is_empty())
            .collect();
        out.push_str(&format!(
            "  变体 {} 个 / 点 {} 个（03 与 09 均未登记这一类；6.3 删变体时必须同批改写）\n",
            c.len(),
            c.iter().map(|s| s.ast_to_ast.len()).sum::<usize>()
        ));
        for s in &c {
            let mark = if DEAD_VARIANTS_03.contains(&s.name.as_str()) {
                "  ← 13 名单内"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {:<10} {}{}\n",
                s.name,
                s.ast_to_ast
                    .iter()
                    .map(|x| format!(":{}", x.line))
                    .collect::<Vec<_>>()
                    .join(" "),
                mark
            ));
        }
        let dead_points: usize = self.dead_list_points().iter().map(|(_, n)| *n).sum();
        out.push_str(&format!(
            "  其中 13 名单内：{} 个变体 / {} 点（Union/Option/Result/Sum/AssocType 各 2）\n\n",
            self.dead_list_points().len(),
            dead_points
        ));

        out.push_str(&baseline::render_comparison(&self.stats));

        out.push_str("── T1.7 白名单核对（6.6 转硬的前提）────────────────────────────────────\n");
        let measured: Vec<&str> = self
            .forward_zero()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        let mut unregistered: Vec<&str> = Vec::new();
        for name in &measured {
            if !REGISTERED_ZERO_FORWARD.iter().any(|r| r.name == *name) {
                unregistered.push(name);
            }
        }
        let mut broken: Vec<&str> = Vec::new();
        for reg in REGISTERED_ZERO_FORWARD {
            match self.stats.iter().find(|s| s.name == reg.name) {
                Some(s) if s.forward_effective() == 0 => {}
                _ => broken.push(reg.name),
            }
        }
        out.push_str(&format!(
            "已登记 {} 个；实测前向零构造 {} 个；未登记 {} 个；登记但已不成立 {} 个\n",
            REGISTERED_ZERO_FORWARD.len(),
            measured.len(),
            unregistered.len(),
            broken.len()
        ));
        out.push_str(if unregistered.is_empty() && broken.is_empty() {
            "  结论：名单一致（未登记 0 / 失效 0）\n"
        } else {
            "  结论：白名单需更新（见上两行清单）\n"
        });
        out.push('\n');
        out
    }

    /// `build.rs` 用的 warning 行（只报告，不失败）
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        let zero: Vec<&str> = self
            .forward_zero()
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        let global: Vec<&str> = self.global_zero().iter().map(|s| s.name.as_str()).collect();
        let reverse_a: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|s| !s.reverse_from.is_empty())
            .collect();
        let reverse_b: Vec<&VariantStats> = self
            .stats
            .iter()
            .filter(|s| !s.reverse_mono.is_empty())
            .collect();
        let ast_to_ast_variants = self
            .stats
            .iter()
            .filter(|s| !s.ast_to_ast.is_empty())
            .count();
        let dead_points: usize = self.dead_list_points().iter().map(|(_, n)| *n).sum();

        out.push(format!(
            "type-tables T1（只报告，6.6 才转硬）：扫描 {} 个非测试 .rs；ast::Type {} 变体；前向零构造 {}；全局零构造 {}〔{}〕；反向桥 A {} 变体；反向桥 B {} 变体；AST→AST 重建 {} 变体（13 名单内 {} 点）",
            self.scanned_files,
            self.stats.len(),
            zero.len(),
            global.len(),
            global.join("/"),
            reverse_a.len(),
            reverse_b.len(),
            ast_to_ast_variants,
            dead_points,
        ));

        let mut unregistered: Vec<&str> = Vec::new();
        for name in &zero {
            if !REGISTERED_ZERO_FORWARD.iter().any(|r| r.name == *name) {
                unregistered.push(name);
            }
        }
        if !unregistered.is_empty() {
            out.push(format!(
                "type-tables T1：未登记的前向零构造变体 {:?} —— 新增零构造变体必须删除或登记理由（03 §5.7）",
                unregistered
            ));
        }
        let mut broken: Vec<&str> = Vec::new();
        for reg in REGISTERED_ZERO_FORWARD {
            match self.stats.iter().find(|s| s.name == reg.name) {
                Some(s) if s.forward_effective() == 0 => {}
                _ => broken.push(reg.name),
            }
        }
        if !broken.is_empty() {
            out.push(format!(
                "type-tables T1：登记的零构造变体已出现前向构造点 {:?} —— 名单与实现不一致，需更新（03 §5.7）",
                broken
            ));
        }
        out
    }

    /// `--json` 的结构化结果
    pub fn to_json(&self) -> serde_json::Value {
        let variants: Vec<serde_json::Value> = self
            .stats
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "def_line": s.decl_line,
                    "forward": s.forward_raw(),
                    "forward_effective": s.forward_effective(),
                    "forward_wildcard_fallback": s.forward.iter().filter(|x| x.wildcard_arm).count(),
                    "ast_to_ast": s.ast_to_ast.len(),
                    "reverse_from_ir_type": s.reverse_from.len(),
                    "reverse_mono_to_ast": s.reverse_mono.len(),
                    "total": s.total(),
                })
            })
            .collect();
        let sites: Vec<serde_json::Value> = self
            .stats
            .iter()
            .flat_map(|s| {
                s.forward
                    .iter()
                    .chain(s.ast_to_ast.iter())
                    .chain(s.reverse_from.iter())
                    .chain(s.reverse_mono.iter())
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
        serde_json::json!({
            "scanned_files": self.scanned_files,
            "variant_count": self.stats.len(),
            "forward_zero": self.forward_zero().iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            "global_zero": self.global_zero().iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            "variants": variants,
            "sites": sites,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(
        variant: &str,
        class: SiteClass,
        wildcard_arm: bool,
    ) -> Site {
        Site {
            variant: variant.to_string(),
            file: "fixture.rs".to_string(),
            line: 1,
            class,
            wildcard_arm,
        }
    }

    // Void 的唯一前向点是错误兜底：原始计数 1，但「前向零构造」判定必须按有效计数 0。
    #[test]
    fn test_forward_effective_excludes_registered_wildcard_fallback() {
        // Arrange：一个变体的前向点全在通配臂内（Void 的形态），另一个有正常前向点
        let mut void_stats = VariantStats::from_decl(&VariantDecl {
            name: "Void".to_string(),
            line: 455,
        });
        void_stats.push(site("Void", SiteClass::Forward, true));
        let mut name_stats = VariantStats::from_decl(&VariantDecl {
            name: "Name".to_string(),
            line: 445,
        });
        name_stats.push(site("Name", SiteClass::Forward, false));

        // Act + Assert
        assert_eq!(void_stats.forward_raw(), 1, "原始前向计数应保留兜底点");
        assert_eq!(
            void_stats.forward_effective(),
            0,
            "通配臂兜底点不计入前向零构造判定"
        );
        assert_eq!(name_stats.forward_effective(), 1, "正常前向构造点必须计入");
        assert_eq!(
            void_stats.total(),
            1,
            "合计仍应包含兜底点（全局零构造用原始计数）"
        );
    }

    // 三类分列是 T1 的核心纪律：混算会把 03 §5.2 的删除名单抹平。
    #[test]
    fn test_variant_stats_keeps_three_classes_separate() {
        // Arrange
        let mut stats = VariantStats::from_decl(&VariantDecl {
            name: "AssocType".to_string(),
            line: 480,
        });
        stats.push(site("AssocType", SiteClass::AstToAst, false));
        stats.push(site("AssocType", SiteClass::AstToAst, false));

        // Act
        let forward_zero = stats.forward_effective() == 0;

        // Assert
        assert!(forward_zero, "AST→AST 重建点不得计入前向构造点");
        assert_eq!(stats.ast_to_ast.len(), 2, "AST→AST 应各自归类并计数");
        assert_eq!(stats.reverse_total(), 0, "反向桥计数不得被 AST→AST 污染");
    }
}
