//! T2 / T3 的**已登记差异与处置**（人工复核过的基线；未登记即视为漂移）。
//!
//! 每条 = 检查号 + 名字 + 事实（reason）+ 处置（action）。6.1「名称规范化」按 action 落地后，
//! 相应条目应从本表删除并让 T2/T3 转全绿——本文件是**待办清单**，不是豁免白名单：
//! 新增差异必须在此登记理由，否则 `--report` 计入「未登记漂移」。

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
pub const T2: &[Disposition] = &[
    Disposition {
        check: "T2.C1",
        name: "I8",
        reason: "CONST_PARAM_TYPES 用大写位宽缩写；from_builtin_name 只认小写 i8 与规范名 Int8",
        action: "6.1：CONST_PARAM_TYPES 改用规范名 Int8（A 无需增补大写缩写）",
    },
    Disposition {
        check: "T2.C1",
        name: "I16",
        reason: "同上；A 认 i16 / Int16",
        action: "6.1：CONST_PARAM_TYPES 改用 Int16",
    },
    Disposition {
        check: "T2.C1",
        name: "I32",
        reason: "同上；A 认 i32 / Int32",
        action: "6.1：CONST_PARAM_TYPES 改用 Int32",
    },
    Disposition {
        check: "T2.C1",
        name: "I64",
        reason: "同上；A 认 i64 / Int64",
        action: "6.1：CONST_PARAM_TYPES 改用 Int64",
    },
    Disposition {
        check: "T2.C1",
        name: "F32",
        reason: "同上；A 认 f32 / Float32",
        action: "6.1：CONST_PARAM_TYPES 改用 Float32",
    },
    Disposition {
        check: "T2.C1",
        name: "F64",
        reason: "同上；A 认 f64 / Float64",
        action: "6.1：CONST_PARAM_TYPES 改用 Float64",
    },
    Disposition {
        check: "T2.C1",
        name: "U8",
        reason: "from_builtin_name 无无符号族（MonoType 无 UInt 变体；eval 的 Uint 臂已是死臂）",
        action: "6.1：从 CONST_PARAM_TYPES 删除（保留会产生无法 lower 的 const 参数名）",
    },
    Disposition {
        check: "T2.C1",
        name: "U16",
        reason: "同上",
        action: "6.1：从 CONST_PARAM_TYPES 删除",
    },
    Disposition {
        check: "T2.C1",
        name: "U32",
        reason: "同上",
        action: "6.1：从 CONST_PARAM_TYPES 删除",
    },
    Disposition {
        check: "T2.C1",
        name: "U64",
        reason: "同上",
        action: "6.1：从 CONST_PARAM_TYPES 删除",
    },
    Disposition {
        check: "T2.C2",
        name: "DateTime",
        reason: "LSP 清单未收录语义别名 DateTime（A 把它归一到 Int(64)，#338 连带修复）",
        action: "6.1：决定别名是否入 LSP 清单（建议收录，供 hover/definition）；不阻塞 T2",
    },
    Disposition {
        check: "T2.C2",
        name: "Never",
        reason: "LSP 清单未收录 Never（RFC-011 的「Never 与 Void」一节的内建类型名，A 解析它）",
        action: "6.1：补入 LSP 清单",
    },
    Disposition {
        check: "T2.C3",
        name: "Int128",
        reason:
            "LSP 清单有 Int128，from_builtin_name 不解析（用户写 Int128 会落成用户类型 TypeRef）",
        action: "6.1：从 LSP 清单删除 Int128",
    },
    Disposition {
        check: "T2.C3",
        name: "Uint",
        reason:
            "LSP 清单有 Uint，A 不解析；eval/const_eval.rs 的 Uint 臂已是死臂（03 §2.3 第 6 项）",
        action: "6.1：从 LSP 清单删除 Uint",
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
        name: "Int",
        reason: "——",
        action: "无需处置（文档类型表已列）",
    },
    Disposition {
        check: "T3.C2D",
        name: "String",
        reason: "——",
        action: "无需处置（文档类型表已列）",
    },
    Disposition {
        check: "T3.C2D",
        name: "Float",
        reason: "RFC-011 正文（原语值复制 / Dup 表）用到该名，但类型表的 inline code token 只有 Int/String",
        action: "登记为「文档非穷尽」：RFC-011 不是内置类型名清单的载体；6.1 后由 03 §5.3 的清单承接",
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
        action: "登记为「文档非穷尽」：别名不必进 RFC-011",
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
        name: "Float32",
        reason: "同上",
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
