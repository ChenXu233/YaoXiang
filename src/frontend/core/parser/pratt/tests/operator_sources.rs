//! 两套 BinOp 表示的同源互校（WBS 8.3.1）。
//!
//! - 第一套：`ast::BinOp` —— parser 产出的语言级算子（表 `pratt/precedence.rs` 的
//!   `op` 字段是 token → 算子的**唯一**声明）。
//! - 第二套：`types::const_data::BinOp` —— 编译期常量求值的算子（18 变体）。
//!   唯一映射点 = `types/eval/const_eval.rs::convert_ast_binop`（私有）；
//!   公共可观测入口 = `types/eval::convert_expr_to_const_expr`。
//!
//! 本测试是「同源互校」的执行面（生产唯一映射点 + **测试侧手写期望表**互校）：
//! 表里声明的每个 AST 算子都经公共入口转换一次，
//! 断言转换结果与表声明同名同义；**测试侧 `expected_const_op` 是本测试的手写 oracle**，
//! 生产侧 token → 算子 → 第二套表示的映射点仍唯一（`convert_ast_binop`），表里没有第三条映射。
//!
//! 三扇门：证明门——「同一算子在两套表示里指同一个运算」是与实现选择无关的性质；
//! 若任一映射漂移（改名/换臂/丢算子），本测试必红。

use crate::ast::{BinOp, Expr, Literal};
use crate::frontend::core::parser::pratt::precedence::INFIX_OPS;
use crate::frontend::core::types::const_data::{BinOp as ConstBinOp, ConstExpr};
use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;
use crate::util::span::Span;

/// 表里声明的 AST 算子 → 期望的第二套表示。
///
/// `Assign` / `Range` 不是常量求值算子：前者是语句级赋值，后者是运行期 Range 值，
/// 两者经公共入口都必须返回 `None`（转换器不认识它们，也就不存在第二份映射）。
fn expected_const_op(op: BinOp) -> Option<ConstBinOp> {
    match op {
        BinOp::Add => Some(ConstBinOp::Add),
        BinOp::Sub => Some(ConstBinOp::Sub),
        BinOp::Mul => Some(ConstBinOp::Mul),
        BinOp::Div => Some(ConstBinOp::Div),
        BinOp::Mod => Some(ConstBinOp::Mod),
        BinOp::Eq => Some(ConstBinOp::Eq),
        BinOp::Neq => Some(ConstBinOp::Ne),
        BinOp::Lt => Some(ConstBinOp::Lt),
        BinOp::Le => Some(ConstBinOp::Le),
        BinOp::Gt => Some(ConstBinOp::Gt),
        BinOp::Ge => Some(ConstBinOp::Ge),
        BinOp::And => Some(ConstBinOp::And),
        BinOp::Or => Some(ConstBinOp::Or),
        BinOp::BitAnd => Some(ConstBinOp::BitAnd),
        BinOp::BitOr => Some(ConstBinOp::BitOr),
        BinOp::BitXor => Some(ConstBinOp::BitXor),
        BinOp::Shl => Some(ConstBinOp::Shl),
        BinOp::Shr => Some(ConstBinOp::Shr),
        BinOp::Assign | BinOp::Range => None,
    }
}

/// 构造 `left <op> right` 的最小 AST（变量引用可转成常量变量）。
fn binary_expr(op: BinOp) -> Expr {
    let span = Span::dummy();
    Expr::BinOp {
        op,
        left: Box::new(Expr::Var("lhs".to_string(), span)),
        right: Box::new(Expr::Lit(Literal::Int(2), span)),
        span,
    }
}

#[test]
fn test_every_table_operator_maps_to_the_same_const_data_operator() {
    let mut checked = 0usize;
    for entry in INFIX_OPS {
        let Some(op) = entry.op else {
            continue;
        };
        let converted = convert_expr_to_const_expr(&binary_expr(op));
        match expected_const_op(op) {
            Some(expected) => {
                let Some(ConstExpr::BinOp { op: actual, .. }) = converted else {
                    panic!("{op:?}（token {:?}）必须可转成常量表达式", entry.token);
                };
                assert_eq!(
                    actual, expected,
                    "{op:?} 在第二套表示里必须仍是同一运算（表行 {:?}）",
                    entry.token
                );
                checked += 1;
            }
            None => assert!(
                converted.is_none(),
                "{op:?} 不是常量求值算子，转换器不得接受它",
            ),
        }
    }
    // 18 = 可映射的 AST 算子全集（Eq/Neq 分别映射 Eq/Ne；Lt/Gt 在表里各出现两次：
    // 比较行与移位行——移位行的 op 是 Shl/Shr，不重复计数）。
    assert_eq!(
        checked, 18,
        "表里可映射到常量算子的 AST 算子必须逐个互校（实际 {checked}）"
    );
}

/// 变体锚：对第二套表示做**穷举 match**（无通配臂）。
///
/// 复审 t29 F9：原先反向枚举是硬编码 18 项列表 + contains，新增 `const_data::BinOp`
/// 变体时测试不会红。本函数把「第二套表示有哪些变体」钉成编译期事实——
/// 新增/删除变体都会让本 match 编译失败，迫使同步表与 oracle。
fn const_bin_op_name(op: ConstBinOp) -> &'static str {
    match op {
        ConstBinOp::Add => "Add",
        ConstBinOp::Sub => "Sub",
        ConstBinOp::Mul => "Mul",
        ConstBinOp::Div => "Div",
        ConstBinOp::Mod => "Mod",
        ConstBinOp::Eq => "Eq",
        ConstBinOp::Ne => "Ne",
        ConstBinOp::Lt => "Lt",
        ConstBinOp::Le => "Le",
        ConstBinOp::Gt => "Gt",
        ConstBinOp::Ge => "Ge",
        ConstBinOp::And => "And",
        ConstBinOp::Or => "Or",
        ConstBinOp::BitAnd => "BitAnd",
        ConstBinOp::BitOr => "BitOr",
        ConstBinOp::BitXor => "BitXor",
        ConstBinOp::Shl => "Shl",
        ConstBinOp::Shr => "Shr",
    }
}

/// 第二套表示的变体全量（与 [`const_bin_op_name`] 的穷举 match 配对）。
const CONST_BIN_OP_VARIANTS: [ConstBinOp; 18] = [
    ConstBinOp::Add,
    ConstBinOp::Sub,
    ConstBinOp::Mul,
    ConstBinOp::Div,
    ConstBinOp::Mod,
    ConstBinOp::Eq,
    ConstBinOp::Ne,
    ConstBinOp::Lt,
    ConstBinOp::Le,
    ConstBinOp::Gt,
    ConstBinOp::Ge,
    ConstBinOp::And,
    ConstBinOp::Or,
    ConstBinOp::BitAnd,
    ConstBinOp::BitOr,
    ConstBinOp::BitXor,
    ConstBinOp::Shl,
    ConstBinOp::Shr,
];

/// 结论范围（复审 t29 F9 限定）：本测试证明「表声明的 AST 算子 → 第二套表示」
/// **每个都有同义对应**，且第二套表示的变体集合由穷举 match + 计数锚锁定；
/// 它不证明「第二套表示里每个变体都必须在表里出现」——该方向由构造性来源保证
/// （表是 token → 算子的唯一生产声明）。新增变体时 `const_bin_op_name` 编译失败，
/// 届时必须同时更新本数组与表映射。
#[test]
fn test_second_representation_variants_are_enumerated_exhaustively() {
    // Arrange：变体集合由穷举 match 锚定（新增变体 ⇒ 编译失败）
    let mut derived: Vec<&str> = Vec::new();
    for entry in INFIX_OPS {
        if let Some(op) = entry.op {
            if let Some(mapped) = expected_const_op(op) {
                derived.push(const_bin_op_name(mapped));
            }
        }
    }

    // Assert：第二套表示的每个变体都能由表派生（名字比对），且全量计数吻合
    assert_eq!(
        CONST_BIN_OP_VARIANTS.len(),
        18,
        "第二套表示的变体数由本数组 + 穷举 match 锚定；计数变化必须同步本测试与表"
    );
    for variant in CONST_BIN_OP_VARIANTS {
        let name = const_bin_op_name(variant);
        assert!(
            derived.contains(&name),
            "常量求值算子 {name} 必须能由表里的 AST 算子派生（否则是第二份映射）"
        );
    }
}
