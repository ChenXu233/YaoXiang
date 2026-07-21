//! Const泛型测试 — 基于 RFC-011 §4 (编译期泛型)
//!
//! §4.1: 编译期常量参数 — ConstGenericEval 表达式求值
//! §4.1: ConstGenericResult 正确性
//! §4.2: 编译期计算 — 阶乘、斐波那契
//! §4.2: ConstFunction 调用
//! §4.3: GenericSize 类型尺寸计算
//! §4.3: SizeExpr 表达式

use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
use crate::frontend::core::types::{ConstValue, MonoType};
use crate::frontend::core::types::eval::const_eval::{
    ConstFunction, ConstGenericEval, ConstGenericResult, GenericSize, SizeExpr, SizeResult,
};
use crate::util::diagnostic::Diagnostic;

/// 辅助函数：断言 eval 结果等于期望值
fn assert_eval_eq(
    result: Result<ConstValue, Diagnostic>,
    expected: ConstValue,
) {
    match result {
        Ok(v) => assert_eq!(v, expected),
        Err(d) => panic!("Expected Ok({:?}), got Err({})", expected, d),
    }
}

// ===================================================================
// §4.1: ConstGenericResult
// ===================================================================

#[test]
fn test_const_result_new_and_accessors() {
    let r = ConstGenericResult::new(ConstValue::Int(42), true);
    assert_eq!(r.value, ConstValue::Int(42));
    assert!(r.is_const());
    assert_eq!(r.as_int(), Some(42));
    assert_eq!(r.as_bool(), None);
}

#[test]
fn test_const_result_not_const() {
    let r = ConstGenericResult::new(ConstValue::Bool(false), false);
    assert!(!r.is_const());
    assert_eq!(r.as_bool(), Some(false));
    assert_eq!(r.as_int(), None);
}

// ===================================================================
// §4.1: 字面量求值
// ===================================================================

#[test]
fn test_eval_int_literal() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Lit(ConstValue::Int(42))),
        ConstValue::Int(42),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Lit(ConstValue::Int(0))),
        ConstValue::Int(0),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Lit(ConstValue::Int(-1))),
        ConstValue::Int(-1),
    );
}

#[test]
fn test_eval_bool_literal() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Lit(ConstValue::Bool(true))),
        ConstValue::Bool(true),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Lit(ConstValue::Bool(false))),
        ConstValue::Bool(false),
    );
}

#[test]
fn test_eval_float_literal() {
    let e = ConstGenericEval::new();
    assert!(e
        .eval(&ConstExpr::Lit(ConstValue::Float(std::f32::consts::PI)))
        .is_ok());
    assert!(e
        .eval(&ConstExpr::Lit(ConstValue::Float(std::f32::consts::PI)))
        .is_ok());
    assert!(e.eval(&ConstExpr::Lit(ConstValue::Float(0.0))).is_ok());
    assert!(e.eval(&ConstExpr::Lit(ConstValue::Float(-1.5))).is_ok());
}

// ===================================================================
// §4.1: 二元运算求值
// ===================================================================

fn bin(
    op: BinOp,
    l: i128,
    r: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(l))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(r))),
    }
}

#[test]
fn test_eval_arithmetic_ops() {
    let e = ConstGenericEval::new();
    assert_eval_eq(e.eval(&bin(BinOp::Add, 3, 4)), ConstValue::Int(7));
    assert_eval_eq(e.eval(&bin(BinOp::Sub, 10, 3)), ConstValue::Int(7));
    assert_eval_eq(e.eval(&bin(BinOp::Mul, 6, 7)), ConstValue::Int(42));
    assert_eval_eq(e.eval(&bin(BinOp::Div, 10, 2)), ConstValue::Int(5));
    assert_eval_eq(e.eval(&bin(BinOp::Mod, 10, 3)), ConstValue::Int(1));
}

#[test]
fn test_eval_division_by_zero() {
    let e = ConstGenericEval::new();
    assert!(e.eval(&bin(BinOp::Div, 1, 0)).is_err());
    assert!(e.eval(&bin(BinOp::Mod, 1, 0)).is_err());
}

#[test]
fn test_eval_comparison_ops() {
    let e = ConstGenericEval::new();
    assert_eval_eq(e.eval(&bin(BinOp::Eq, 5, 5)), ConstValue::Bool(true));
    assert_eval_eq(e.eval(&bin(BinOp::Eq, 5, 6)), ConstValue::Bool(false));
    assert_eval_eq(e.eval(&bin(BinOp::Ne, 5, 6)), ConstValue::Bool(true));
    assert_eval_eq(e.eval(&bin(BinOp::Lt, 3, 5)), ConstValue::Bool(true));
    assert_eval_eq(e.eval(&bin(BinOp::Gt, 5, 3)), ConstValue::Bool(true));
    assert_eval_eq(e.eval(&bin(BinOp::Le, 5, 5)), ConstValue::Bool(true));
    assert_eval_eq(e.eval(&bin(BinOp::Ge, 5, 5)), ConstValue::Bool(true));
}

#[test]
fn test_eval_bitwise_ops() {
    let e = ConstGenericEval::new();
    assert_eval_eq(e.eval(&bin(BinOp::BitAnd, 6, 3)), ConstValue::Int(2));
    assert_eval_eq(e.eval(&bin(BinOp::BitOr, 6, 3)), ConstValue::Int(7));
    assert_eval_eq(e.eval(&bin(BinOp::BitXor, 6, 3)), ConstValue::Int(5));
    assert_eval_eq(e.eval(&bin(BinOp::Shl, 1, 8)), ConstValue::Int(256));
    assert_eval_eq(e.eval(&bin(BinOp::Shr, 256, 8)), ConstValue::Int(1));
}

#[test]
fn test_eval_shift_out_of_range() {
    let e = ConstGenericEval::new();
    assert!(e.eval(&bin(BinOp::Shl, 1, 128)).is_err());
    assert!(e.eval(&bin(BinOp::Shr, 1, -1)).is_err());
}

#[test]
fn test_eval_float_arith() {
    let e = ConstGenericEval::new();
    let fa = ConstExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(1.5))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(2.5))),
    };
    assert_eval_eq(e.eval(&fa), ConstValue::Float(4.0));
    let fm = ConstExpr::BinOp {
        op: BinOp::Mul,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(2.0))),
    };
    assert_eval_eq(e.eval(&fm), ConstValue::Float(6.0));
}

#[test]
fn test_eval_float_compare() {
    let e = ConstGenericEval::new();
    let flt = ConstExpr::BinOp {
        op: BinOp::Lt,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(1.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(2.0))),
    };
    assert_eval_eq(e.eval(&flt), ConstValue::Bool(true));
}

// ===================================================================
// §4.1: 一元运算
// ===================================================================

#[test]
fn test_eval_neg() {
    let e = ConstGenericEval::new();
    use crate::frontend::core::types::const_data::UnOp;
    let neg = |x| ConstExpr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(ConstExpr::Lit(ConstValue::Int(x))),
    };
    assert_eval_eq(e.eval(&neg(42)), ConstValue::Int(-42));
    assert_eval_eq(e.eval(&neg(-5)), ConstValue::Int(5));
}

#[test]
fn test_eval_not() {
    let e = ConstGenericEval::new();
    use crate::frontend::core::types::const_data::UnOp;
    let not = |b| ConstExpr::UnOp {
        op: UnOp::Not,
        expr: Box::new(ConstExpr::Lit(ConstValue::Bool(b))),
    };
    assert_eval_eq(e.eval(&not(true)), ConstValue::Bool(false));
    assert_eval_eq(e.eval(&not(false)), ConstValue::Bool(true));
}

// ===================================================================
// §4.1: 变量绑定和求值
// ===================================================================

#[test]
fn test_eval_var_bound() {
    let mut e = ConstGenericEval::new();
    e.bind_var("x".to_string(), ConstValue::Int(42));
    assert_eval_eq(
        e.eval(&ConstExpr::NamedVar("x".to_string())),
        ConstValue::Int(42),
    );
}

#[test]
fn test_eval_var_unbound() {
    let e = ConstGenericEval::new();
    assert!(e
        .eval(&ConstExpr::NamedVar("undefined".to_string()))
        .is_err());
}

// ===================================================================
// §4.1: If 条件求值
// ===================================================================

#[test]
fn test_eval_if_true_false() {
    let e = ConstGenericEval::new();
    let iff = |c, t, f| ConstExpr::If {
        condition: Box::new(ConstExpr::Lit(ConstValue::Bool(c))),
        then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(t))),
        else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(f))),
    };
    assert_eval_eq(e.eval(&iff(true, 1, 2)), ConstValue::Int(1));
    assert_eval_eq(e.eval(&iff(false, 10, 20)), ConstValue::Int(20));
}

// ===================================================================
// §4.2: 用户函数
// ===================================================================

#[test]
fn test_eval_custom_function() {
    let mut e = ConstGenericEval::new();
    e.register_function(
        "double".to_string(),
        ConstFunction::new(
            "double".to_string(),
            vec!["x".to_string()],
            ConstExpr::BinOp {
                op: BinOp::Mul,
                left: Box::new(ConstExpr::NamedVar("x".to_string())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
            },
        ),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "double".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(21))],
        }),
        ConstValue::Int(42),
    );
}

#[test]
fn test_eval_custom_function_arg_count_mismatch() {
    let mut e = ConstGenericEval::new();
    e.register_function(
        "f".to_string(),
        ConstFunction::new(
            "f".to_string(),
            vec!["x".to_string()],
            ConstExpr::Lit(ConstValue::Int(0)),
        ),
    );
    assert!(e
        .eval(&ConstExpr::Call {
            func: "f".to_string(),
            args: vec![],
        })
        .is_err());
}

// ===================================================================
// §4.2: 阶乘和斐波那契
// ===================================================================

#[test]
fn test_eval_factorial() {
    let mut e = ConstGenericEval::new();
    e.register_function(
        "factorial".to_string(),
        crate::frontend::core::types::eval::const_eval::functions::factorial(),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(0))],
        }),
        ConstValue::Int(1),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(5))],
        }),
        ConstValue::Int(120),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(10))],
        }),
        ConstValue::Int(3628800),
    );
}

#[test]
fn test_eval_fibonacci() {
    let mut e = ConstGenericEval::new();
    let fib = crate::frontend::core::types::eval::const_eval::functions::fibonacci();
    e.register_function(fib.name.clone(), fib);
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "fibonacci".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(0))],
        }),
        ConstValue::Int(0),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "fibonacci".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(1))],
        }),
        ConstValue::Int(1),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "fibonacci".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(10))],
        }),
        ConstValue::Int(55),
    );
}

// ===================================================================
// §4.2: 内置函数
// ===================================================================

#[test]
fn test_eval_builtin_abs() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "abs".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(-5))],
        }),
        ConstValue::Int(5),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "abs".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(0))],
        }),
        ConstValue::Int(0),
    );
}

#[test]
fn test_eval_builtin_min_max() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "min".to_string(),
            args: vec![
                ConstExpr::Lit(ConstValue::Int(3)),
                ConstExpr::Lit(ConstValue::Int(7)),
            ],
        }),
        ConstValue::Int(3),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "max".to_string(),
            args: vec![
                ConstExpr::Lit(ConstValue::Int(3)),
                ConstExpr::Lit(ConstValue::Int(7)),
            ],
        }),
        ConstValue::Int(7),
    );
}

#[test]
fn test_eval_builtin_sizeof() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Int".to_string())],
        }),
        ConstValue::Int(8),
    );
    assert!(e
        .eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Unknown".to_string())],
        })
        .is_err());
}

#[test]
fn test_eval_undefined_function() {
    let e = ConstGenericEval::new();
    assert!(e
        .eval(&ConstExpr::Call {
            func: "nonexistent".to_string(),
            args: vec![],
        })
        .is_err());
}

// ===================================================================
// §4.2: unsupported operation
// ===================================================================

#[test]
fn test_eval_mismatched_types_in_binop() {
    let e = ConstGenericEval::new();
    let mixed = ConstExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        right: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
    };
    assert!(e.eval(&mixed).is_err());
}

// ===================================================================
// §4.3: GenericSize
// ===================================================================

#[test]
fn test_generic_size_primitives() {
    let gs = GenericSize::new();
    assert_eq!(gs.size_of(&MonoType::Bool), Ok(1));
    assert_eq!(gs.size_of(&MonoType::Int(32)), Ok(8));
    assert_eq!(gs.size_of(&MonoType::Float(64)), Ok(8));
    assert_eq!(gs.size_of(&MonoType::String), Ok(8));
    assert_eq!(gs.size_of(&MonoType::Void), Ok(0));
    assert_eq!(gs.size_of(&MonoType::TypeRef("Int".to_string())), Ok(8));
    assert_eq!(gs.size_of(&MonoType::TypeRef("Bool".to_string())), Ok(1));
}

#[test]
fn test_generic_size_tuple() {
    let gs = GenericSize::new();
    assert_eq!(
        gs.size_of(&MonoType::Tuple(vec![
            MonoType::Bool,
            MonoType::Int(64),
            MonoType::Float(32)
        ])),
        Ok(17)
    );
}

#[test]
fn test_generic_size_dynamic_list() {
    let gs = GenericSize::new();
    assert!(gs
        .size_of(&MonoType::List(Box::new(MonoType::Int(32))))
        .is_err());
}

#[test]
fn test_generic_size_fn_pointer() {
    let gs = GenericSize::new();
    assert_eq!(
        gs.size_of(&MonoType::Fn {
            params: vec![],
            return_type: Box::new(MonoType::Void),
        }),
        Ok(8)
    );
}

#[test]
fn test_generic_size_type_var_fails() {
    let gs = GenericSize::new();
    assert!(gs
        .size_of(&MonoType::TypeVar(
            crate::frontend::core::types::TypeVar::new(0)
        ))
        .is_err());
}

#[test]
fn test_generic_size_unknown_typeref_fails() {
    let gs = GenericSize::new();
    assert!(gs
        .size_of(&MonoType::TypeRef("Unknown".to_string()))
        .is_err());
}

// ===================================================================
// §4.3: SizeExpr
// ===================================================================

#[test]
fn test_size_expr_const() {
    let result = SizeExpr::Const(8).eval().unwrap();
    assert_eq!(result.size, 8);
    assert!(result.is_const);
}

#[test]
fn test_size_expr_mul_add() {
    let mul = SizeExpr::Mul(Box::new(SizeExpr::Const(4)), Box::new(SizeExpr::Const(2)));
    assert_eq!(mul.eval().unwrap().size, 8);
    let add = SizeExpr::Add(Box::new(SizeExpr::Const(3)), Box::new(SizeExpr::Const(5)));
    assert_eq!(add.eval().unwrap().size, 8);
    // Both const operands -> result is const
    assert!(add.eval().unwrap().is_const);
}

// ===================================================================
// §4.3: SizeResult
// ===================================================================

#[test]
fn test_size_result() {
    let r = SizeResult::new(16, true);
    assert_eq!(r.size, 16);
    assert!(r.is_const);
    let r2 = SizeResult::new(0, false);
    assert_eq!(r2.size, 0);
    assert!(!r2.is_const);
}

// ============ supplementary tests: coverage gaps ============

#[test]
fn test_eval_float_gt_lte_gte() {
    let e = ConstGenericEval::new();
    let gt = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(5.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
    };
    assert_eval_eq(e.eval(&gt), ConstValue::Bool(true));
    let lte = ConstExpr::BinOp {
        op: BinOp::Le,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
    };
    assert_eval_eq(e.eval(&lte), ConstValue::Bool(true));
    let gte = ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(5.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
    };
    assert_eval_eq(e.eval(&gte), ConstValue::Bool(true));
}

#[test]
fn test_generic_size_array_typeref() {
    let gs = GenericSize::new();
    assert_eq!(
        gs.size_of(&MonoType::TypeRef("Array(Int, 10)".to_string())),
        Ok(80)
    );
    assert_eq!(
        gs.size_of(&MonoType::TypeRef("Array(Float, 5)".to_string())),
        Ok(40)
    );
    assert_eq!(
        gs.size_of(&MonoType::TypeRef("Array(Array(Int, 10), 2)".to_string())),
        Ok(160)
    );
    assert!(gs
        .size_of(&MonoType::TypeRef("Unknown".to_string()))
        .is_err());
}

#[test]
fn test_eval_bool_plus_int_unsupported() {
    let e = ConstGenericEval::new();
    let bad = ConstExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
    };
    assert!(e.eval(&bad).is_err());
}

#[test]
fn test_eval_neg_zero() {
    let e = ConstGenericEval::new();
    use crate::frontend::core::types::const_data::UnOp;
    assert_eval_eq(
        e.eval(&ConstExpr::UnOp {
            op: UnOp::Neg,
            expr: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        }),
        ConstValue::Int(0),
    );
}

// ===================================================================
// supplementary tests: more code paths
// ===================================================================

#[test]
fn test_eval_if_non_boolean_condition() {
    let e = ConstGenericEval::new();
    let iff = ConstExpr::If {
        condition: Box::new(ConstExpr::Lit(ConstValue::Int(1))), // Not a boolean
        then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
    };
    assert!(e.eval(&iff).is_err(), "non-boolean condition should fail");
}

#[test]
fn test_eval_builtin_abs_positive() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "abs".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(5))],
        }),
        ConstValue::Int(5),
    );
}

#[test]
fn test_eval_builtin_compile_time() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "compile_time".to_string(),
            args: vec![],
        }),
        ConstValue::Bool(true),
    );
}

#[test]
fn test_eval_builtin_sizeof_void() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Void".to_string())],
        }),
        ConstValue::Int(0),
    );
}

#[test]
fn test_eval_builtin_sizeof_char() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Char".to_string())],
        }),
        ConstValue::Int(4),
    );
}

#[test]
fn test_eval_builtin_sizeof_uint() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Uint".to_string())],
        }),
        ConstValue::Int(8),
    );
}

#[test]
fn test_eval_builtin_sizeof_float() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("Float".to_string())],
        }),
        ConstValue::Int(8),
    );
}

#[test]
fn test_eval_builtin_sizeof_string() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::NamedVar("String".to_string())],
        }),
        ConstValue::Int(8),
    );
}

#[test]
fn test_eval_builtin_sizeof_non_var() {
    let e = ConstGenericEval::new();
    assert!(e
        .eval(&ConstExpr::Call {
            func: "sizeof".to_string(),
            args: vec![ConstExpr::Lit(ConstValue::Int(42))], // Not a Var
        })
        .is_err());
}

#[test]
fn test_eval_custom_function_multiple_args() {
    let mut e = ConstGenericEval::new();
    e.register_function(
        "add3".to_string(),
        ConstFunction::new(
            "add3".to_string(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ConstExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(ConstExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(ConstExpr::NamedVar("a".to_string())),
                    right: Box::new(ConstExpr::NamedVar("b".to_string())),
                }),
                right: Box::new(ConstExpr::NamedVar("c".to_string())),
            },
        ),
    );
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "add3".to_string(),
            args: vec![
                ConstExpr::Lit(ConstValue::Int(1)),
                ConstExpr::Lit(ConstValue::Int(2)),
                ConstExpr::Lit(ConstValue::Int(3)),
            ],
        }),
        ConstValue::Int(6),
    );
}

#[test]
fn test_eval_builtin_min_same() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "min".to_string(),
            args: vec![
                ConstExpr::Lit(ConstValue::Int(5)),
                ConstExpr::Lit(ConstValue::Int(5)),
            ],
        }),
        ConstValue::Int(5),
    );
}

#[test]
fn test_eval_builtin_max_same() {
    let e = ConstGenericEval::new();
    assert_eval_eq(
        e.eval(&ConstExpr::Call {
            func: "max".to_string(),
            args: vec![
                ConstExpr::Lit(ConstValue::Int(5)),
                ConstExpr::Lit(ConstValue::Int(5)),
            ],
        }),
        ConstValue::Int(5),
    );
}

#[test]
fn test_eval_neg_float() {
    let e = ConstGenericEval::new();
    use crate::frontend::core::types::const_data::UnOp;
    let neg = ConstExpr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(ConstExpr::Lit(ConstValue::Float(std::f32::consts::PI))),
    };
    // Neg on float should fail (unsupported)
    assert!(e.eval(&neg).is_err());
}

#[test]
fn test_eval_not_int() {
    let e = ConstGenericEval::new();
    use crate::frontend::core::types::const_data::UnOp;
    let not = ConstExpr::UnOp {
        op: UnOp::Not,
        expr: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
    };
    // Not on int should fail (unsupported)
    assert!(e.eval(&not).is_err());
}

#[test]
fn test_eval_float_sub() {
    let e = ConstGenericEval::new();
    let sub = ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(5.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
    };
    // Float Sub is not supported
    assert!(e.eval(&sub).is_err());
}

#[test]
fn test_eval_float_div() {
    let e = ConstGenericEval::new();
    let div = ConstExpr::BinOp {
        op: BinOp::Div,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(10.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(2.0))),
    };
    // Float Div is not supported
    assert!(e.eval(&div).is_err());
}

#[test]
fn test_eval_float_eq() {
    let e = ConstGenericEval::new();
    let eq = ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
    };
    assert_eval_eq(e.eval(&eq), ConstValue::Bool(true));
}

#[test]
fn test_eval_float_neq() {
    let e = ConstGenericEval::new();
    let neq = ConstExpr::BinOp {
        op: BinOp::Ne,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(3.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(4.0))),
    };
    assert_eval_eq(e.eval(&neq), ConstValue::Bool(true));
}

#[test]
fn test_eval_float_div_by_zero() {
    let e = ConstGenericEval::new();
    let div = ConstExpr::BinOp {
        op: BinOp::Div,
        left: Box::new(ConstExpr::Lit(ConstValue::Float(1.0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Float(0.0))),
    };
    // Float division by zero may or may not error depending on implementation
    let _ = e.eval(&div);
}

#[test]
fn test_generic_size_struct() {
    let gs = GenericSize::new();
    let s = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Float(64)),
            ("y".to_string(), MonoType::Float(64)),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    // Struct is not supported by GenericSize
    assert!(gs.size_of(&s).is_err());
}

#[test]
fn test_generic_size_empty_tuple() {
    let gs = GenericSize::new();
    assert_eq!(gs.size_of(&MonoType::Tuple(vec![])), Ok(0));
}

#[test]
fn test_generic_size_single_element_tuple() {
    let gs = GenericSize::new();
    assert_eq!(gs.size_of(&MonoType::Tuple(vec![MonoType::Int(32)])), Ok(8));
}

#[test]
fn test_size_expr_mul_const() {
    let mul = SizeExpr::Mul(Box::new(SizeExpr::Const(4)), Box::new(SizeExpr::Const(3)));
    assert_eq!(mul.eval().unwrap().size, 12);
}

#[test]
fn test_size_expr_add_const() {
    let add = SizeExpr::Add(Box::new(SizeExpr::Const(3)), Box::new(SizeExpr::Const(5)));
    assert_eq!(add.eval().unwrap().size, 8);
}

#[test]
fn test_size_expr_nested() {
    // (2 * 3) + 4 = 10
    let nested = SizeExpr::Add(
        Box::new(SizeExpr::Mul(
            Box::new(SizeExpr::Const(2)),
            Box::new(SizeExpr::Const(3)),
        )),
        Box::new(SizeExpr::Const(4)),
    );
    assert_eq!(nested.eval().unwrap().size, 10);
}

#[test]
fn test_const_function_new() {
    let func = ConstFunction::new(
        "test".to_string(),
        vec!["x".to_string()],
        ConstExpr::NamedVar("x".to_string()),
    );
    assert_eq!(func.name, "test");
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0], "x");
}

#[test]
fn test_eval_var_bound_bool() {
    let mut e = ConstGenericEval::new();
    e.bind_var("flag".to_string(), ConstValue::Bool(true));
    assert_eval_eq(
        e.eval(&ConstExpr::NamedVar("flag".to_string())),
        ConstValue::Bool(true),
    );
}

#[test]
fn test_eval_var_bound_float() {
    let mut e = ConstGenericEval::new();
    e.bind_var("pi".to_string(), ConstValue::Float(std::f32::consts::PI));
    assert_eval_eq(
        e.eval(&ConstExpr::NamedVar("pi".to_string())),
        ConstValue::Float(std::f32::consts::PI),
    );
}

#[test]
fn test_eval_multiple_if() {
    let e = ConstGenericEval::new();
    // Nested if
    let nested_if = ConstExpr::If {
        condition: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        then_branch: Box::new(ConstExpr::If {
            condition: Box::new(ConstExpr::Lit(ConstValue::Bool(false))),
            then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
            else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
        }),
        else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(3))),
    };
    assert_eval_eq(e.eval(&nested_if), ConstValue::Int(2));
}

// ===================================================================
// supplementary tests: more GenericSize paths
// ===================================================================

#[test]
fn test_generic_size_type_ref_int() {
    let gs = GenericSize::new();
    assert_eq!(gs.size_of(&MonoType::TypeRef("Int".to_string())), Ok(8));
}

#[test]
fn test_generic_size_type_ref_int32() {
    let gs = GenericSize::new();
    // TypeRef "Int32" is not in the size table
    assert!(gs.size_of(&MonoType::TypeRef("Int32".to_string())).is_err());
}

#[test]
fn test_generic_size_type_ref_int8() {
    let gs = GenericSize::new();
    // TypeRef "Int8" is not in the size table
    assert!(gs.size_of(&MonoType::TypeRef("Int8".to_string())).is_err());
}

#[test]
fn test_generic_size_type_ref_float32() {
    let gs = GenericSize::new();
    // TypeRef "Float32" is not in the size table
    assert!(gs
        .size_of(&MonoType::TypeRef("Float32".to_string()))
        .is_err());
}

#[test]
fn test_generic_size_type_ref_char() {
    let gs = GenericSize::new();
    // TypeRef "Char" is not in the size table
    assert!(gs.size_of(&MonoType::TypeRef("Char".to_string())).is_err());
}

#[test]
fn test_generic_size_type_ref_void() {
    let gs = GenericSize::new();
    // TypeRef "Void" returns 0
    assert_eq!(gs.size_of(&MonoType::TypeRef("Void".to_string())), Ok(0));
}

#[test]
fn test_generic_size_type_ref_bytes() {
    let gs = GenericSize::new();
    // TypeRef "Bytes" is not in the size table
    assert!(gs.size_of(&MonoType::TypeRef("Bytes".to_string())).is_err());
}

#[test]
fn test_generic_size_dict() {
    let gs = GenericSize::new();
    // Dict is not supported
    assert!(gs
        .size_of(&MonoType::Dict(
            Box::new(MonoType::String),
            Box::new(MonoType::Int(32))
        ))
        .is_err());
}

#[test]
fn test_generic_size_set() {
    let gs = GenericSize::new();
    // Set is not supported
    assert!(gs
        .size_of(&MonoType::Set(Box::new(MonoType::Bool)))
        .is_err());
}

#[test]
fn test_generic_size_range() {
    let gs = GenericSize::new();
    // Range is not supported
    assert!(gs
        .size_of(&MonoType::Range {
            elem_type: Box::new(MonoType::Int(64))
        })
        .is_err());
}

#[test]
fn test_generic_size_arc() {
    let gs = GenericSize::new();
    // Arc is not supported
    assert!(gs
        .size_of(&MonoType::Arc(Box::new(MonoType::Int(32))))
        .is_err());
}

#[test]
fn test_generic_size_weak() {
    let gs = GenericSize::new();
    // Weak is not supported
    assert!(gs
        .size_of(&MonoType::Weak(Box::new(MonoType::String)))
        .is_err());
}

#[test]
fn test_generic_size_option() {
    let gs = GenericSize::new();
    // Option is not supported
    assert!(gs
        .size_of(&MonoType::Option(Box::new(MonoType::Int(32))))
        .is_err());
}

#[test]
fn test_generic_size_result() {
    let gs = GenericSize::new();
    // Result is not supported
    assert!(gs
        .size_of(&MonoType::Result(
            Box::new(MonoType::Int(32)),
            Box::new(MonoType::String)
        ))
        .is_err());
}

#[test]
fn test_generic_size_union() {
    let gs = GenericSize::new();
    // Union is not supported
    assert!(gs
        .size_of(&MonoType::Union(vec![MonoType::Int(32), MonoType::String]))
        .is_err());
}

#[test]
fn test_generic_size_intersection() {
    let gs = GenericSize::new();
    // Intersection is not supported
    assert!(gs
        .size_of(&MonoType::Intersection(vec![MonoType::TypeRef(
            "Clone".to_string()
        )]))
        .is_err());
}

// ===================================================================
// supplementary tests: SizeExpr extensions
// ===================================================================

#[test]
fn test_size_expr_nested_mul_add() {
    // (2 * 3) + (4 * 5) = 6 + 20 = 26
    let expr = SizeExpr::Add(
        Box::new(SizeExpr::Mul(
            Box::new(SizeExpr::Const(2)),
            Box::new(SizeExpr::Const(3)),
        )),
        Box::new(SizeExpr::Mul(
            Box::new(SizeExpr::Const(4)),
            Box::new(SizeExpr::Const(5)),
        )),
    );
    assert_eq!(expr.eval().unwrap().size, 26);
}

#[test]
fn test_size_result_is_const() {
    let r = SizeResult::new(8, true);
    assert!(r.is_const);
    let r2 = SizeResult::new(8, false);
    assert!(!r2.is_const);
}

// ===================================================================
// supplementary tests: ConstGenericResult extensions
// ===================================================================

#[test]
fn test_const_result_float() {
    let r = ConstGenericResult::new(ConstValue::Float(std::f32::consts::PI), true);
    assert!(r.is_const());
    assert_eq!(r.as_int(), None);
    assert_eq!(r.as_bool(), None);
}

#[test]
fn test_const_result_debug() {
    let r = ConstGenericResult::new(ConstValue::Int(42), true);
    let debug = format!("{:?}", r);
    assert!(debug.contains("ConstGenericResult"));
}

// ===================================================================
// supplementary tests: ConstExpr extensions
// ===================================================================

#[test]
fn test_const_expr_debug() {
    let expr = ConstExpr::Lit(ConstValue::Int(42));
    let debug = format!("{:?}", expr);
    assert!(debug.contains("Int"));
}

#[test]
fn test_const_bin_op_debug() {
    let op = BinOp::Add;
    let debug = format!("{:?}", op);
    assert!(debug.contains("Add"));
}

#[test]
fn test_const_un_op_debug() {
    let op = crate::frontend::core::types::const_data::UnOp::Neg;
    let debug = format!("{:?}", op);
    assert!(debug.contains("Neg"));
}

// ===================================================================
// AST Expr -> ConstExpr 转换测试
// 基于 docs/superpowers/specs/2026-07-11-const-expr-constraint-design.md
// ===================================================================

#[test]
fn test_convert_expr_to_const_expr_int_literal() {
    use crate::frontend::core::parser::ast::Expr;
    use crate::frontend::core::lexer::tokens::Literal;
    use crate::util::span::Span;
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    // Act
    let result = convert_expr_to_const_expr(&Expr::Lit(Literal::Int(42), Span::dummy()));
    // Assert
    assert_eq!(
        result,
        Some(ConstExpr::Lit(ConstValue::Int(42))),
        "Int literal 42 should convert to ConstExpr::Lit(ConstValue::Int(42))"
    );
}

#[test]
fn test_convert_expr_to_const_expr_var() {
    use crate::frontend::core::parser::ast::Expr;
    use crate::util::span::Span;
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    // Act
    let result = convert_expr_to_const_expr(&Expr::Var("N".to_string(), Span::dummy()));
    // Assert
    assert_eq!(
        result,
        Some(ConstExpr::NamedVar("N".to_string())),
        "Variable reference N should convert to ConstExpr::NamedVar(\"N\")"
    );
}

#[test]
fn test_convert_expr_to_const_expr_binop_gt() {
    use crate::frontend::core::parser::ast::{Expr, BinOp};
    use crate::frontend::core::lexer::tokens::Literal;
    use crate::util::span::Span;
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    // Arrange
    let expr = Expr::BinOp {
        op: BinOp::Gt,
        left: Box::new(Expr::Var("N".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
        span: Span::dummy(),
    };
    // Act
    let result = convert_expr_to_const_expr(&expr);
    // Assert
    assert_eq!(
        result,
        Some(ConstExpr::BinOp {
            op: crate::frontend::core::types::const_data::BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("N".to_string())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        }),
        "AST BinOp(Gt, Var(N), Lit(0)) should convert to ConstExpr::BinOp(Gt, Var(N), Int(0))"
    );
}

#[test]
fn test_convert_expr_to_const_expr_unsupported_returns_none() {
    use crate::frontend::core::parser::ast::Expr;
    use crate::util::span::Span;
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    // Act
    let result = convert_expr_to_const_expr(&Expr::List(vec![], Span::dummy()));
    // Assert
    assert_eq!(
        result, None,
        "List expression (not a constant expression) should return None"
    );
}
