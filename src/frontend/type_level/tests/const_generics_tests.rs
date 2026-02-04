//! Const 泛型测试 (RFC-011)
//!
//! 测试 Const 泛型系统的功能：
//! - 字面量类型解析
//! - Const 参数验证
//! - Const 表达式求值
//! - 编译期计算

use crate::frontend::core::parser::ast::{Type, GenericParam, GenericParamKind};
use crate::frontend::core::type_system::{ConstValue, ConstKind, MonoType};
use crate::frontend::type_level::const_generics::{
    ConstGenericEval, LiteralTypeValidator, ConstExpr, ConstBinOp,
};
use crate::frontend::type_level::const_generics::eval::ConstFunction;

/// 测试字面量类型解析
#[test]
fn test_literal_type_parsing() {
    // 测试从 AST 字面量类型解析
    let ty = Type::Literal {
        name: "5".to_string(),
        base_type: Box::new(Type::Name("Int".to_string())),
    };

    // 验证字面量名称解析为 ConstValue
    let value = ConstValue::from_literal_name("5");
    assert!(value.is_some());
    assert_eq!(value.unwrap(), ConstValue::Int(5));
}

/// 测试布尔字面量解析
#[test]
fn test_bool_literal_parsing() {
    let true_val = ConstValue::from_literal_name("true");
    assert!(true_val.is_some());
    assert_eq!(true_val.unwrap(), ConstValue::Bool(true));

    let false_val = ConstValue::from_literal_name("false");
    assert!(false_val.is_some());
    assert_eq!(false_val.unwrap(), ConstValue::Bool(false));
}

/// 测试无效字面量名称
#[test]
fn test_invalid_literal_name() {
    let value = ConstValue::from_literal_name("hello");
    assert!(value.is_none());

    // 浮点数现在应该是有效的
    let value = ConstValue::from_literal_name("3.14");
    assert!(value.is_some());
}

/// 测试浮点数字面量解析
#[test]
fn test_float_literal_parsing() {
    let value = ConstValue::from_literal_name("3.14");
    assert!(value.is_some());
    match value.unwrap() {
        ConstValue::Float(f) => assert!((f - 3.14).abs() < 0.001),
        _ => panic!("Expected Float variant"),
    }

    let value = ConstValue::from_literal_name("-2.5");
    assert!(value.is_some());
    match value.unwrap() {
        ConstValue::Float(f) => assert!((f - (-2.5)).abs() < 0.001),
        _ => panic!("Expected Float variant"),
    }
}

/// 测试 ConstExpr Float 类型
#[test]
fn test_const_expr_float() {
    use crate::frontend::type_level::const_generics::ConstExpr;

    let expr = ConstExpr::Float(3.14);
    assert!(matches!(expr, ConstExpr::Float(f) if (f - 3.14).abs() < 0.001));
}

/// 测试 Const 求值器 - 位运算
#[test]
fn test_const_eval_bitwise() {
    let eval = ConstGenericEval::new();

    // 测试位与
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::BitAnd,
        lhs: Box::new(ConstExpr::Int(0b1111)),
        rhs: Box::new(ConstExpr::Int(0b1010)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(0b1010));

    // 测试位或
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::BitOr,
        lhs: Box::new(ConstExpr::Int(0b1111)),
        rhs: Box::new(ConstExpr::Int(0b0000)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(0b1111));

    // 测试位异或
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::BitXor,
        lhs: Box::new(ConstExpr::Int(0b1111)),
        rhs: Box::new(ConstExpr::Int(0b1010)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(0b0101));

    // 测试左移
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Shl,
        lhs: Box::new(ConstExpr::Int(1)),
        rhs: Box::new(ConstExpr::Int(4)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(16));

    // 测试右移
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Shr,
        lhs: Box::new(ConstExpr::Int(16)),
        rhs: Box::new(ConstExpr::Int(4)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(1));
}

/// 测试 Const 求值器 - 浮点数运算
#[test]
fn test_const_eval_float_operations() {
    let eval = ConstGenericEval::new();

    // 测试浮点数加法
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Add,
        lhs: Box::new(ConstExpr::Float(3.14)),
        rhs: Box::new(ConstExpr::Float(2.86)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    match result.unwrap() {
        ConstValue::Float(f) => assert!((f - 6.0).abs() < 0.001),
        _ => panic!("Expected Float variant"),
    }

    // 测试浮点数乘法
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Mul,
        lhs: Box::new(ConstExpr::Float(2.0)),
        rhs: Box::new(ConstExpr::Float(3.0)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    match result.unwrap() {
        ConstValue::Float(f) => assert!((f - 6.0).abs() < 0.001),
        _ => panic!("Expected Float variant"),
    }

    // 测试浮点数比较
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Lt,
        lhs: Box::new(ConstExpr::Float(1.5)),
        rhs: Box::new(ConstExpr::Float(2.5)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Bool(true));
}

/// 测试 Const 参数信息提取
#[test]
fn test_const_param_extraction() {
    use crate::frontend::type_level::const_generics::validation::extract_const_param_info;

    // 测试提取 Const 参数信息
    let param = GenericParam {
        name: "N".to_string(),
        kind: GenericParamKind::Const {
            const_type: Box::new(Type::Name("Int".to_string())),
        },
        constraints: vec![],
    };

    let info = extract_const_param_info(&param);
    assert!(info.is_some());
    let (name, kind) = info.unwrap();
    assert_eq!(name, "N");
    assert_eq!(kind, ConstKind::Int(None));
}

/// 测试 Const 求值器 - 基础运算
#[test]
fn test_const_eval_basic_operations() {
    let mut eval = ConstGenericEval::new();

    // 测试整数加法
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Add,
        lhs: Box::new(ConstExpr::Int(5)),
        rhs: Box::new(ConstExpr::Int(3)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(8));

    // 测试整数乘法
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Mul,
        lhs: Box::new(ConstExpr::Int(6)),
        rhs: Box::new(ConstExpr::Int(7)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(42));
}

/// 测试 Const 求值器 - 比较运算
#[test]
fn test_const_eval_comparison() {
    let eval = ConstGenericEval::new();

    // 测试等于
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Eq,
        lhs: Box::new(ConstExpr::Int(5)),
        rhs: Box::new(ConstExpr::Int(5)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Bool(true));

    // 测试小于
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Lt,
        lhs: Box::new(ConstExpr::Int(3)),
        rhs: Box::new(ConstExpr::Int(5)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Bool(true));
}

/// 测试 Const 求值器 - 变量绑定
#[test]
fn test_const_eval_variable_binding() {
    let mut eval = ConstGenericEval::new();

    // 绑定变量
    eval.bind_var("n".to_string(), ConstValue::Int(10));

    // 测试变量引用
    let expr = ConstExpr::Var("n".to_string());
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(10));

    // 测试使用变量的表达式
    let expr = ConstExpr::BinOp {
        op: ConstBinOp::Mul,
        lhs: Box::new(ConstExpr::Var("n".to_string())),
        rhs: Box::new(ConstExpr::Int(2)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(20));
}

/// 测试 Const 求值器 - 条件表达式
#[test]
fn test_const_eval_conditional() {
    let eval = ConstGenericEval::new();

    // 测试 true 分支
    let expr = ConstExpr::If {
        condition: Box::new(ConstExpr::BinOp {
            op: ConstBinOp::Lt,
            lhs: Box::new(ConstExpr::Int(5)),
            rhs: Box::new(ConstExpr::Int(10)),
        }),
        true_branch: Box::new(ConstExpr::Int(100)),
        false_branch: Box::new(ConstExpr::Int(200)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(100));

    // 测试 false 分支 (10 > 15 应该是 false)
    let expr = ConstExpr::If {
        condition: Box::new(ConstExpr::BinOp {
            op: ConstBinOp::Gt,
            lhs: Box::new(ConstExpr::Int(10)),
            rhs: Box::new(ConstExpr::Int(15)),
        }),
        true_branch: Box::new(ConstExpr::Int(100)),
        false_branch: Box::new(ConstExpr::Int(200)),
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(200));
}

/// 测试字面量类型验证器
#[test]
fn test_literal_type_validator() {
    let mut validator = LiteralTypeValidator::new();

    // 注册字面量类型
    validator.register_literal_type("SIZE".to_string(), ConstValue::Int(100), MonoType::Int(64));

    // 验证解析
    let ty = Type::Literal {
        name: "SIZE".to_string(),
        base_type: Box::new(Type::Name("Int".to_string())),
    };
    let result = validator.parse_literal_type(&ty);
    assert!(result.is_some());
    let (name, value) = result.unwrap();
    assert_eq!(name, "SIZE");
    assert_eq!(value, ConstValue::Int(100));
}

/// 测试 Const 参数绑定
#[test]
fn test_const_param_binding() {
    let mut validator = LiteralTypeValidator::new();

    // 绑定 Const 参数
    validator.bind_const_param("n".to_string(), ConstValue::Int(5));

    // 验证解析
    let ty = Type::Name("n".to_string());
    let result = validator.parse_literal_type(&ty);
    assert!(result.is_some());
    let (name, value) = result.unwrap();
    assert_eq!(name, "n");
    assert_eq!(value, ConstValue::Int(5));
}

/// 测试 Const 类型验证
#[test]
fn test_const_type_validation() {
    let validator = LiteralTypeValidator::new();

    // 验证 Int 类型
    let ty = Type::Name("Int".to_string());
    let kind = validator.validate_const_type(&ty);
    assert!(kind.is_some());
    assert_eq!(kind.unwrap(), ConstKind::Int(None));

    // 验证 Bool 类型
    let ty = Type::Name("Bool".to_string());
    let kind = validator.validate_const_type(&ty);
    assert!(kind.is_some());
    assert_eq!(kind.unwrap(), ConstKind::Bool);
}

/// 测试 Const 值类型匹配
#[test]
fn test_const_value_type_matching() {
    let validator = LiteralTypeValidator::new();

    // Int 值匹配 Int 类型
    assert!(validator.matches_type(&ConstValue::Int(5), &ConstKind::Int(None)));

    // Bool 值匹配 Bool 类型
    assert!(validator.matches_type(&ConstValue::Bool(true), &ConstKind::Bool));

    // 类型不匹配
    assert!(!validator.matches_type(&ConstValue::Int(5), &ConstKind::Bool));
    assert!(!validator.matches_type(&ConstValue::Bool(true), &ConstKind::Int(None)));
}

/// 测试 Const 求值器 - 阶乘
#[test]
fn test_const_eval_factorial() {
    // 手动定义阶乘函数
    let factorial_fn = ConstFunction::new(
        "factorial".to_string(),
        vec!["n".to_string()],
        ConstExpr::If {
            condition: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Lte,
                lhs: Box::new(ConstExpr::Var("n".to_string())),
                rhs: Box::new(ConstExpr::Int(1)),
            }),
            true_branch: Box::new(ConstExpr::Int(1)),
            false_branch: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Mul,
                lhs: Box::new(ConstExpr::Var("n".to_string())),
                rhs: Box::new(ConstExpr::Call {
                    name: "factorial".to_string(),
                    args: vec![ConstExpr::BinOp {
                        op: ConstBinOp::Sub,
                        lhs: Box::new(ConstExpr::Var("n".to_string())),
                        rhs: Box::new(ConstExpr::Int(1)),
                    }],
                }),
            }),
        },
    );

    let mut eval = ConstGenericEval::new();
    eval.register_function("factorial".to_string(), factorial_fn);

    // 计算 5!
    let expr = ConstExpr::Call {
        name: "factorial".to_string(),
        args: vec![ConstExpr::Int(5)],
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(120));

    // 计算 0!
    let expr = ConstExpr::Call {
        name: "factorial".to_string(),
        args: vec![ConstExpr::Int(0)],
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(1));
}

/// 测试 Const 求值器 - 斐波那契
#[test]
fn test_const_eval_fibonacci() {
    // 手动定义斐波那契函数
    let fib_fn = ConstFunction::new(
        "fibonacci".to_string(),
        vec!["n".to_string()],
        ConstExpr::If {
            condition: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Lte,
                lhs: Box::new(ConstExpr::Var("n".to_string())),
                rhs: Box::new(ConstExpr::Int(1)),
            }),
            true_branch: Box::new(ConstExpr::Var("n".to_string())),
            false_branch: Box::new(ConstExpr::BinOp {
                op: ConstBinOp::Add,
                lhs: Box::new(ConstExpr::Call {
                    name: "fibonacci".to_string(),
                    args: vec![ConstExpr::BinOp {
                        op: ConstBinOp::Sub,
                        lhs: Box::new(ConstExpr::Var("n".to_string())),
                        rhs: Box::new(ConstExpr::Int(1)),
                    }],
                }),
                rhs: Box::new(ConstExpr::Call {
                    name: "fibonacci".to_string(),
                    args: vec![ConstExpr::BinOp {
                        op: ConstBinOp::Sub,
                        lhs: Box::new(ConstExpr::Var("n".to_string())),
                        rhs: Box::new(ConstExpr::Int(2)),
                    }],
                }),
            }),
        },
    );

    let mut eval = ConstGenericEval::new();
    eval.register_function("fibonacci".to_string(), fib_fn);

    // 计算 fib(10)
    let expr = ConstExpr::Call {
        name: "fibonacci".to_string(),
        args: vec![ConstExpr::Int(10)],
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(55));

    // 计算 fib(0)
    let expr = ConstExpr::Call {
        name: "fibonacci".to_string(),
        args: vec![ConstExpr::Int(0)],
    };
    let result = eval.eval(&expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstValue::Int(0));
}

/// 测试 MonoType 字面量类型
#[test]
fn test_mono_type_literal() {
    let literal = MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    };

    // 验证类型名称
    assert_eq!(literal.type_name(), "int64::5");
}

/// 测试 ConstValue 哈希
#[test]
fn test_const_value_hash() {
    use std::collections::HashMap;

    let mut map: HashMap<ConstValue, String> = HashMap::new();

    map.insert(ConstValue::Int(42), "answer".to_string());
    map.insert(ConstValue::Bool(true), "yes".to_string());

    assert_eq!(map.get(&ConstValue::Int(42)), Some(&"answer".to_string()));
    assert_eq!(map.get(&ConstValue::Bool(true)), Some(&"yes".to_string()));
}

/// 测试 GenericParam Kind
#[test]
fn test_generic_param_kind() {
    // 测试 Type 参数
    let type_param = GenericParam {
        name: "T".to_string(),
        kind: GenericParamKind::Type,
        constraints: vec![Type::Name("Clone".to_string())],
    };
    match &type_param.kind {
        GenericParamKind::Type => {}
        _ => panic!("Expected Type kind"),
    }

    // 测试 Const 参数
    let const_param = GenericParam {
        name: "N".to_string(),
        kind: GenericParamKind::Const {
            const_type: Box::new(Type::Name("Int".to_string())),
        },
        constraints: vec![],
    };
    match &const_param.kind {
        GenericParamKind::Const { const_type } => match const_type.as_ref() {
            Type::Name(name) => assert_eq!(name, "Int"),
            _ => panic!("Expected Int type"),
        },
        _ => panic!("Expected Const kind"),
    }
}

/// 测试泛型尺寸计算 - 数组类型
#[test]
fn test_generic_size_array() {
    use crate::frontend::type_level::const_generics::GenericSize;

    let size_calc = GenericSize::new();

    // 测试 Array<Int, 10> - Int 是 8 字节，所以 8 * 10 = 80
    let array_type = MonoType::TypeRef("Array<Int, 10>".to_string());
    let result = size_calc.size_of(&array_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 80);

    // 测试 Array<Float, 5> - Float 是 8 字节，所以 8 * 5 = 40
    let array_type = MonoType::TypeRef("Array<Float, 5>".to_string());
    let result = size_calc.size_of(&array_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 40);

    // 测试 Array<Bool, 100> - Bool 是 1 字节，所以 1 * 100 = 100
    let array_type = MonoType::TypeRef("Array<Bool, 100>".to_string());
    let result = size_calc.size_of(&array_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 100);

    // 测试嵌套数组 Array<Array<Int, 2>, 3> - 外层 3 * (内层 2 * 8) = 48
    let nested_type = MonoType::TypeRef("Array<Array<Int, 2>, 3>".to_string());
    let result = size_calc.size_of(&nested_type);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 48);
}
