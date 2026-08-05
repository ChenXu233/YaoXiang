//! PredicateResolver 模块测试
//!
//! RFC-027：编译期求值类型——编译期谓词正格化。
//! RFC-027 §语法（谓词应用实参规则）+ issue #263：
//! try_resolve 三值语义——未注册谓词返回 None，已注册但用法非法返回
//! Some(Err)，精化约束绝不静默丢弃。

use crate::frontend::core::typecheck::predicate_resolver::{PredicateDef, PredicateResolver};
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

fn make_positive_def() -> PredicateDef {
    PredicateDef {
        param_name: "x".into(),
        param_type: MonoType::Int(64),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    }
}

#[test]
fn test_resolve_positive_with_literal() {
    // Arrange: 注册 Positive 谓词，实参为字面量 5
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    // Act
    let result = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::Literal {
            name: "5".into(),
            base_type: Box::new(MonoType::Int(64)),
            value: ConstValue::Int(5),
        }],
    );

    // Assert: 解析成功，约束应是 5 > 0
    match result.expect("已注册谓词应被识别（#263 三值语义）") {
        Ok(MonoType::Refined { base, constraint }) => {
            assert_eq!(*base, MonoType::Int(64), "基类型应为 Int(64)");
            match constraint {
                ConstExpr::BinOp { op, left, right } => {
                    assert_eq!(op, BinOp::Gt, "Positive 约束应为大于比较");
                    assert_eq!(
                        *left,
                        ConstExpr::Lit(ConstValue::Int(5)),
                        "实参 5 应代入约束左侧"
                    );
                    assert_eq!(
                        *right,
                        ConstExpr::Lit(ConstValue::Int(0)),
                        "约束右侧应为字面量 0"
                    );
                }
                other => panic!("约束应为 BinOp，实际: {:?}", other),
            }
        }
        other => panic!("应解析为 Refined，实际: {:?}", other),
    }
}

#[test]
fn test_resolve_positive_with_variable() {
    // Arrange: 实参为变量 b（按名绑定）
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    // Act
    let result = PredicateResolver::try_resolve(&env, "Positive", &[MonoType::TypeRef("b".into())]);

    // Assert: 解析成功，约束左侧为 NamedVar("b")
    match result.expect("已注册谓词应被识别（#263 三值语义）") {
        Ok(MonoType::Refined { base, constraint }) => {
            assert_eq!(*base, MonoType::Int(64), "基类型应为 Int(64)");
            match constraint {
                ConstExpr::BinOp { op, left, .. } => {
                    assert_eq!(op, BinOp::Gt, "Positive 约束应为大于比较");
                    assert_eq!(
                        *left,
                        ConstExpr::NamedVar("b".into()),
                        "变量实参应按名绑定到约束"
                    );
                }
                other => panic!("约束应为 BinOp，实际: {:?}", other),
            }
        }
        other => panic!("应解析为 Refined，实际: {:?}", other),
    }
}

#[test]
fn test_resolve_unknown_predicate_returns_none() {
    // Arrange: 空环境——谓词未注册
    let env = TypeEnvironment::new();

    // Act
    let result = PredicateResolver::try_resolve(&env, "UnknownPredicate", &[MonoType::Int(64)]);

    // Assert: 未注册谓词返回 None（调用方继续其他解析路径）
    assert!(
        result.is_none(),
        "未注册谓词必须返回 None，不得与非法用法混淆"
    );
}

#[test]
fn test_resolve_registered_predicate_invalid_arg_errors() {
    // Arrange: 注册 Positive 谓词，实参为不可转换形态（元组类型）
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());
    let tuple_arg = MonoType::Tuple(vec![MonoType::Int(64), MonoType::Int(64)]);

    // Act
    let result = PredicateResolver::try_resolve(&env, "Positive", &[tuple_arg]);

    // Assert: #263——已注册谓词非法实参必须返回 Some(Err)，绝不静默放行
    let err = result
        .expect("已注册谓词必须被识别，不得返回 None（#263）")
        .expect_err("不可转换实参必须返回 Err（#263：约束不得静默丢弃）");
    assert!(
        err.contains("字面量、变量或单参数类型应用"),
        "错误原因应说明合法实参形态，实际: {}",
        err
    );
}

#[test]
fn test_resolve_registered_predicate_zero_args_errors() {
    // Arrange: 注册 Positive 谓词，零实参调用
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    // Act
    let result = PredicateResolver::try_resolve(&env, "Positive", &[]);

    // Assert: #263——实参个数不匹配必须返回 Some(Err)
    let err = result
        .expect("已注册谓词必须被识别，不得返回 None（#263）")
        .expect_err("零实参必须返回 Err（#263：约束不得静默丢弃）");
    assert!(
        err.contains("期望 1 个实参"),
        "错误原因应说明期望实参个数，实际: {}",
        err
    );
}

#[test]
fn test_resolve_registered_predicate_extra_args_errors() {
    // Arrange: 注册 Positive 谓词，两个实参（阶段 1 谓词为单参数）
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    // Act
    let result = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::TypeRef("a".into()), MonoType::TypeRef("b".into())],
    );

    // Assert: #263——多余实参必须返回 Some(Err)
    let err = result
        .expect("已注册谓词必须被识别，不得返回 None（#263）")
        .expect_err("多余实参必须返回 Err（#263：约束不得静默丢弃）");
    assert!(
        err.contains("期望 1 个实参，实际 2 个"),
        "错误原因应说明实参个数不匹配，实际: {}",
        err
    );
}
