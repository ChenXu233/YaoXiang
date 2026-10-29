//! T2 / T3 的**已登记差异与处置**（人工复核过的基线；未登记即视为漂移）。
//!
//! 每条 = 检查号 + 名字 + 事实（reason）+ 处置（action）。6.1「名称规范化」按 action 落地后，
//! 相应条目应从本表删除并让 T2/T3 转全绿——本文件是**待办清单**，不是豁免白名单：
//! 新增差异必须在此登记理由，否则 `--report` 计入「未登记漂移」。
//!
//! **WBS 6.1.2（2026-10-27 定稿）与 6.4（2026-10-29）之后的剩余项**：
//! - `from_builtin_name` 已收敛为 15 个内置名 + `DateTime` 临时兼容别名，22 个实现内
//!   拼写已删除（T2.5 的逐名哨兵守着，任一回退即报红）。6.0 勘察阶段登记在本表的
//!   「补 Never 进 LSP 清单」「删 LSP 的 Int128 / Uint」三条**已按 action 兑现并删除**。
//! - WBS 6.4 按 action 兑现了 10 条 T2.C1：`CONST_PARAM_TYPES` 删掉 10 个旧拼写
//!   （I8/I16/I32/I64/U8/U16/U32/U64/F32/F64）后只剩规范名，C1（本表 ⊆ 内置名表）清零。
//! - T2 剩余差异只有一类且属刻意选择：LSP 清单不收录 `DateTime`（待迁出的 std 层别名，
//!   不属内置名）。
//! - T3 的差异全部是「RFC-011 不承载内置类型名清单」这一事实的登记，不是待修缺陷。

/// 一条已登记差异
#[derive(Debug)]
pub struct Disposition {
    /// 检查号：`T2.C1` / `T2.C2` / `T2.C3` / `T3.D2C` / `T3.C2D`
    pub check: &'static str,
    pub name: &'static str,
    pub reason: &'static str,
    pub action: &'static str,
}

/// T2：三表逐名差异
///
/// WBS 6.4：`CONST_PARAM_TYPES` 删掉 10 个旧拼写后，C1（本表 ⊆ 内置名表）已清零，
/// 原先 10 条 T2.C1 登记随之兑现并删除。剩余只有 C2 一条（DateTime 刻意不收录）。
pub const T2: &[Disposition] = &[
    Disposition {
        check: "T2.C2",
        name: "DateTime",
        reason: "LSP 清单只列 15 个内置名，刻意不收录 DateTime——它是 std 层临时兼容别名\
                 （src/std/time.rs 与 src/std/interfaces/time.yx 的签名串仍在用），不属内置名",
        action: "P10 S11（issue #513）把它迁出 from_builtin_name 后本差异自动消失；在此之前不收录是定稿的刻意选择",
    },
];

/// T3：RFC-011 差异
///
/// `T3.D2C` = 文档类型表有、A 不解析；`T3.C2D` = A 的规范名不在文档类型表
pub const T3: &[Disposition] = &[
    Disposition {
        check: "T3.D2C",
        name: "Type",
        reason: "元类型关键字，由 ast::Type::MetaType 承载；不是值类型名",
        action: "无需处置：Type 不进 from_builtin_name（它不是值类型名）",
    },
    Disposition {
        check: "T3.D2C",
        name: "If",
        reason: "RFC-011 §5.1 的类型级条件构造器，由 Generic 路径承载",
        action: "无需处置：If 不是内置类型名",
    },
    Disposition {
        check: "T3.D2C",
        name: "Match",
        reason: "RFC-011 §5.2 的类型级匹配构造器，由 Generic 路径承载",
        action: "无需处置：Match 不是内置类型名",
    },
    Disposition {
        check: "T3.C2D",
        name: "Float",
        reason: "RFC-011 正文（原语值复制 / Dup 表）用到该名，但类型表的 inline code token 只有 Int/String",
        action: "登记为「文档非穷尽」：RFC-011 不是内置类型名清单的载体；内置名清单由 SPEC §2.1 与 from_builtin_name 承接",
    },
    Disposition {
        check: "T3.C2D",
        name: "Bool",
        reason: "同 Float（正文使用、类型表未单列）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Char",
        reason: "同 Float（正文使用、类型表未单列）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Bytes",
        reason: "同 Float（正文使用、类型表未单列）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Void",
        reason: "同 Float（正文使用、类型表未单列；「Never 与 Void」一节有定义）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Never",
        reason: "同 Float（「Never 与 Void」一节有定义，但不在任何表格里）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "DateTime",
        reason: "RFC-011 全文未提该名（它是 #338 的语义别名，不属于 RFC-011 讨论面）",
        action: "登记为「文档非穷尽」：别名不必进 RFC-011；迁出见 P10 S11（issue #513）",
    },
    Disposition {
        check: "T3.C2D",
        name: "Int8",
        reason: "RFC-011 全文未提位宽变体（它只讨论泛型机制，不枚举位宽族）",
        action: "登记为「文档非穷尽」：位宽族由 from_builtin_name 承接",
    },
    Disposition {
        check: "T3.C2D",
        name: "Int16",
        reason: "同上",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Int32",
        reason: "同上",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Int64",
        reason: "同上；Int64 自 WBS 6.1.2 起是独立规范名（此前是 Int 臂内的别名写法）",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Float32",
        reason: "同上",
        action: "登记为「文档非穷尽」",
    },
    Disposition {
        check: "T3.C2D",
        name: "Float64",
        reason: "同上；Float64 自 WBS 6.1.2 起是独立规范名（此前是 Float 臂内的别名写法）",
        action: "登记为「文档非穷尽」",
    },
];

/// 按 (检查号, 名字) 查处置
pub fn lookup(
    check: &str,
    name: &str,
) -> Option<&'static Disposition> {
    T2.iter()
        .chain(T3.iter())
        .find(|d| d.check == check && d.name == name)
}
