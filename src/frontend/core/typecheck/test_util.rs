//! RFC-027 测试共享构造助手
//!
//! 收编各 rfc027 测试里手搓 `ConstExpr::BinOp{...} + MonoType::Refined{...}` AST 树的样板，
//! 与 test-compliance 规则 4.2（≤5 行免 AAA）同向。

use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
use crate::frontend::core::types::mono::MonoType;

/// 二元运算
pub fn binop(
    op: BinOp,
    left: ConstExpr,
    right: ConstExpr,
) -> ConstExpr {
    ConstExpr::BinOp {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// Refined 类型（约束挂在 Int(64) 基类型上）
pub fn refined_int(constraint: ConstExpr) -> MonoType {
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    }
}
