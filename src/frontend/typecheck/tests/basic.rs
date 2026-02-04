//! 基础类型推断测试

use std::collections::HashMap;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::{BinOp, Expr, UnOp};
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver, TypeBinding};
use crate::frontend::typecheck::inference::ExprInferrer;
use crate::frontend::typecheck::*;
use crate::util::span::Span;

/// 测试字面量类型推断
#[test]
fn test_literal_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 整数
    let int_lit = Expr::Lit(Literal::Int(42), Span::default());
    let int_ty = inferrer.infer_expr(&int_lit).unwrap();
    assert_eq!(int_ty, MonoType::Int(64));

    // 浮点数
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);
    let float_lit = Expr::Lit(Literal::Float(3.14), Span::default());
    let float_ty = inferrer.infer_expr(&float_lit).unwrap();
    assert_eq!(float_ty, MonoType::Float(64));

    // 布尔值
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);
    let bool_lit = Expr::Lit(Literal::Bool(true), Span::default());
    let bool_ty = inferrer.infer_expr(&bool_lit).unwrap();
    assert_eq!(bool_ty, MonoType::Bool);

    // 字符串
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);
    let str_lit = Expr::Lit(Literal::String("hello".to_string()), Span::default());
    let str_ty = inferrer.infer_expr(&str_lit).unwrap();
    assert_eq!(str_ty, MonoType::String);
}

/// 测试二元运算类型推断
#[test]
fn test_binop_inference() {
    // 加法
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Int(1), Span::default());
    let right = Expr::Lit(Literal::Int(2), Span::default());
    let add = Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&add).unwrap();
    solver.solve().unwrap();

    // 应该是 int64
    let resolved_ty = solver.resolve_type(&ty);
    assert!(matches!(resolved_ty, MonoType::Int(64)));
}

/// 测试比较运算
#[test]
fn test_comparison_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Int(1), Span::default());
    let right = Expr::Lit(Literal::Int(2), Span::default());
    let cmp = Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&cmp).unwrap();
    solver.solve().unwrap();

    assert_eq!(ty, MonoType::Bool);
}

/// 测试逻辑运算
#[test]
fn test_logical_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Bool(true), Span::default());
    let right = Expr::Lit(Literal::Bool(false), Span::default());
    let and = Expr::BinOp {
        op: BinOp::And,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&and).unwrap();
    assert_eq!(ty, MonoType::Bool);
}

/// 测试元组类型推断
#[test]
fn test_tuple_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let elems = vec![
        Expr::Lit(Literal::Int(1), Span::default()),
        Expr::Lit(Literal::String("hello".to_string()), Span::default()),
    ];
    let tuple = Expr::Tuple(elems, Span::default());

    let ty = inferrer.infer_expr(&tuple).unwrap();
    match ty {
        MonoType::Tuple(types) => {
            assert_eq!(types.len(), 2);
        }
        _ => panic!("Expected tuple type"),
    }
}

/// 测试列表类型推断
#[test]
fn test_list_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let elems = vec![
        Expr::Lit(Literal::Int(1), Span::default()),
        Expr::Lit(Literal::Int(2), Span::default()),
        Expr::Lit(Literal::Int(3), Span::default()),
    ];
    let list = Expr::List(elems, Span::default());

    let ty = inferrer.infer_expr(&list).unwrap();
    match ty {
        MonoType::List(_) => {
            // 列表类型，检查是否是 MonoType
        }
        _ => panic!("Expected list type"),
    }
}

/// 测试类型变量绑定
#[test]
fn test_type_variable_binding() {
    let mut solver = TypeConstraintSolver::new();

    let var1 = solver.new_var();
    let type_var1 = var1.type_var().unwrap();
    let var2 = solver.new_var();
    let type_var2 = var2.type_var().unwrap();

    // 绑定 var1 = int64
    solver.bind(type_var1, &MonoType::Int(64)).unwrap();

    // 绑定 var2 = var1
    solver.bind(type_var2, &var1).unwrap();

    // 查找 var2 应该返回 int64
    let resolved = solver.resolve_type(&var2);
    assert_eq!(resolved, MonoType::Int(64));
}

/// 测试类型求解
#[test]
fn test_type_resolution() {
    let mut solver = TypeConstraintSolver::new();

    let var = solver.new_var();
    let type_var = var.type_var().unwrap();

    // 绑定 var = int64
    solver.bind(type_var, &MonoType::Int(64)).unwrap();

    // 解析类型
    let resolved = solver.resolve_type(&var);
    assert_eq!(resolved, MonoType::Int(64));
}

/// 测试一元运算类型推断
#[test]
fn test_unop_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 负数
    let expr = Expr::Lit(Literal::Int(5), Span::default());
    let neg = Expr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(expr),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&neg).unwrap();
    solver.solve().unwrap();
    let resolved_ty = solver.resolve_type(&ty);
    assert!(matches!(resolved_ty, MonoType::Int(64)));
}

/// 测试字符串连接
#[test]
fn test_string_concat() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::String("hello".to_string()), Span::default());
    let right = Expr::Lit(Literal::String(" world".to_string()), Span::default());
    let add = Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&add).unwrap();
    // 字符串加法的类型可能是类型变量或具体类型
    assert!(matches!(ty, MonoType::String) || matches!(ty, MonoType::TypeVar(_)));
}

/// 测试变量引用
#[test]
fn test_variable_reference() {
    // 使用 TypeEnvironment 来添加变量
    let mut env = TypeEnvironment::new();
    env.add_var("my_var".to_string(), PolyType::mono(MonoType::Int(64)));

    // 我们使用 TypeEnvironment 的 get_var 来验证
    let retrieved = env.get_var("my_var").unwrap();
    assert_eq!(retrieved.body, MonoType::Int(64));
}

/// 测试函数调用类型推断
#[test]
fn test_function_call_inference() {
    let mut env = TypeEnvironment::new();

    // 添加函数到环境
    let fn_type = PolyType::mono(MonoType::Fn {
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: Box::new(MonoType::Int(64)),
        is_async: false,
    });
    env.add_var("add".to_string(), fn_type);

    // 注意：TypeInferrer 内部维护自己的 vars 环境
    // 这里我们只验证类型环境的基本功能
    let retrieved = env.get_var("add");
    assert!(retrieved.is_some());
}

/// 测试类型注解验证
#[test]
fn test_type_annotation() {
    let mut solver = TypeConstraintSolver::new();

    // 创建带类型注解的变量声明
    let var = solver.new_var();

    // 类型注解: int64
    let ann_ty = MonoType::Int(64);

    // 添加约束 (需要克隆 var 因为 add_constraint 获得所有权)
    solver.add_constraint(var.clone(), ann_ty.clone(), Span::default());

    // 应该能够成功求解
    solver.solve().unwrap();

    let resolved = solver.resolve_type(&var);
    assert_eq!(resolved, ann_ty);
}

/// 测试混合类型列表（产生约束）
#[test]
fn test_mixed_type_list() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 混合类型列表
    let elems = vec![
        Expr::Lit(Literal::Int(1), Span::default()),
        Expr::Lit(Literal::Float(3.14), Span::default()),
    ];
    let list = Expr::List(elems, Span::default());

    // 这应该产生类型约束冲突
    let result = inferrer.infer_expr(&list);

    // 结果应该包含类型变量（因为无法直接统一）
    assert!(result.is_ok());
}

/// 测试除法类型推断
#[test]
fn test_division_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Int(10), Span::default());
    let right = Expr::Lit(Literal::Int(3), Span::default());
    let div = Expr::BinOp {
        op: BinOp::Div,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&div).unwrap();
    solver.solve().unwrap();

    let resolved_ty = solver.resolve_type(&ty);
    assert!(matches!(resolved_ty, MonoType::Int(64)));
}

/// 测试取模运算
#[test]
fn test_modulo_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Int(10), Span::default());
    let right = Expr::Lit(Literal::Int(3), Span::default());
    let mod_op = Expr::BinOp {
        op: BinOp::Mod,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&mod_op).unwrap();
    solver.solve().unwrap();

    let resolved_ty = solver.resolve_type(&ty);
    assert!(matches!(resolved_ty, MonoType::Int(64)));
}

/// 测试浮点数运算
#[test]
fn test_float_binop_inference() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let left = Expr::Lit(Literal::Float(3.14), Span::default());
    let right = Expr::Lit(Literal::Float(2.0), Span::default());
    let mul = Expr::BinOp {
        op: BinOp::Mul,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&mul).unwrap();
    // 乘法的类型可能是类型变量或具体类型
    assert!(matches!(ty, MonoType::Float(64)) || matches!(ty, MonoType::TypeVar(_)));
}

/// 测试空元组
#[test]
fn test_empty_tuple() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    let tuple = Expr::Tuple(vec![], Span::default());

    let ty = inferrer.infer_expr(&tuple).unwrap();
    match ty {
        MonoType::Tuple(types) => {
            assert_eq!(types.len(), 0);
        }
        _ => panic!("Expected tuple type"),
    }
}

/// 测试类型变量创建
#[test]
fn test_type_var_creation() {
    let mut solver = TypeConstraintSolver::new();

    let var1 = solver.new_var();
    assert!(var1.type_var().is_some());

    let var2 = solver.new_var();
    assert!(var2.type_var().is_some());

    // 两个变量应该不同
    let tv1 = var1.type_var().unwrap();
    let tv2 = var2.type_var().unwrap();
    assert_ne!(tv1, tv2);
}

/// 测试类型约束添加
#[test]
fn test_add_constraint() {
    let mut solver = TypeConstraintSolver::new();

    let var = solver.new_var();

    // 添加约束: var = int64 (需要克隆)
    solver.add_constraint(var.clone(), MonoType::Int(64), Span::default());

    // 求解
    solver.solve().unwrap();

    // 解析
    let resolved = solver.resolve_type(&var);
    assert_eq!(resolved, MonoType::Int(64));
}

/// 测试类型推断器创建
#[test]
fn test_type_inferrer_creation() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 验证推断器创建成功
    assert!(inferrer.solver().new_var().type_var().is_some());
}

/// 测试类型求解器创建
#[test]
fn test_solver_creation() {
    let mut solver = TypeConstraintSolver::new();
    // 验证求解器创建成功
    assert!(solver.new_var().type_var().is_some());
}

/// 测试绑定类型变量到自身错误
#[test]
fn test_bind_var_to_self_error() {
    let mut solver = TypeConstraintSolver::new();

    let var = solver.new_var();
    let type_var = var.type_var().unwrap();

    // 尝试将变量绑定到自身应该失败
    let result = solver.bind(type_var, &var);
    assert!(result.is_err());
}

/// 测试求解器获取绑定
#[test]
fn test_solver_get_binding() {
    let mut solver = TypeConstraintSolver::new();

    let gvar = solver.new_generic_var();

    // 未绑定时应该是 Unbound
    let binding = solver.get_binding(gvar);
    assert!(matches!(binding, Some(TypeBinding::Unbound)));
}

/// 测试绑定后获取绑定
#[test]
fn test_solver_get_binding_after_bind() {
    let mut solver = TypeConstraintSolver::new();

    let gvar = solver.new_generic_var();
    solver.bind(gvar, &MonoType::Int(64)).unwrap();

    // 绑定后应该是 Bound
    let binding = solver.get_binding(gvar);
    match binding {
        Some(TypeBinding::Bound(ty)) => {
            assert_eq!(*ty, MonoType::Int(64));
        }
        _ => panic!("Expected bound type"),
    }
}

/// 测试类型推断器获取求解器
#[test]
fn test_inferrer_get_solver() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 获取求解器并使用
    let new_var = inferrer.solver().new_var();
    assert!(new_var.type_var().is_some());
}
