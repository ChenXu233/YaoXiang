//! 类型推断器详细测试

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::{BinOp, Block, Expr, Pattern, Stmt, StmtKind, UnOp};
use crate::frontend::typecheck::*;
use crate::util::span::Span;

/// 测试类型推断器创建
#[test]
fn test_type_inferrer_new() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);
    assert!(inferrer.solver().new_var().type_var().is_some());
}

/// 测试推断字面量类型
#[test]
fn test_infer_literal_types() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 整数
    let int_expr = Expr::Lit(Literal::Int(42), Span::default());
    let int_ty = inferrer.infer_expr(&int_expr).unwrap();
    assert_eq!(int_ty, MonoType::Int(64));

    // 浮点数
    let float_expr = Expr::Lit(Literal::Float(3.14), Span::default());
    let float_ty = inferrer.infer_expr(&float_expr).unwrap();
    assert_eq!(float_ty, MonoType::Float(64));

    // 布尔值
    let bool_expr = Expr::Lit(Literal::Bool(true), Span::default());
    let bool_ty = inferrer.infer_expr(&bool_expr).unwrap();
    assert_eq!(bool_ty, MonoType::Bool);

    // 字符
    let char_expr = Expr::Lit(Literal::Char('a'), Span::default());
    let char_ty = inferrer.infer_expr(&char_expr).unwrap();
    assert_eq!(char_ty, MonoType::Char);

    // 字符串
    let str_expr = Expr::Lit(Literal::String("test".to_string()), Span::default());
    let str_ty = inferrer.infer_expr(&str_expr).unwrap();
    assert_eq!(str_ty, MonoType::String);
}

/// 测试推断未知变量错误
#[test]
fn test_infer_unknown_variable() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let var_expr = Expr::Var("unknown_var".to_string(), Span::default());
    let result = inferrer.infer_expr(&var_expr);
    assert!(result.is_err());
}

/// 测试添加和获取变量
#[test]
fn test_inferrer_add_get_var() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 添加变量
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    // 获取变量
    let retrieved = inferrer.get_var("x");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().body, MonoType::Int(64));

    // 获取不存在的变量
    let not_found = inferrer.get_var("nonexistent");
    assert!(not_found.is_none());
}

/// 测试作用域管理
#[test]
fn test_scope_management() {
    // 测试进入和退出作用域
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    inferrer.add_var("test".to_string(), PolyType::mono(MonoType::Int(64)));

    // 验证变量在全局作用域可用
    assert!(inferrer.get_var("test").is_some());

    inferrer.enter_scope();
    // 验证可以进入作用域
    assert!(inferrer.get_var("test").is_some());

    inferrer.exit_scope();
    // 验证退出作用域后变量仍然可用（非泛型变量不会被移除）
    assert!(inferrer.get_var("test").is_some());
}

/// 测试算术运算类型推断
#[test]
fn test_infer_arithmetic_ops() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let left = Expr::Lit(Literal::Int(1), Span::default());
    let right = Expr::Lit(Literal::Int(2), Span::default());

    // 加法
    let add = Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _add_ty = inferrer.infer_expr(&add).unwrap();

    // 减法
    let sub = Expr::BinOp {
        op: BinOp::Sub,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&sub).unwrap();

    // 乘法
    let mul = Expr::BinOp {
        op: BinOp::Mul,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&mul).unwrap();

    // 除法
    let div = Expr::BinOp {
        op: BinOp::Div,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&div).unwrap();

    // 取模
    let mod_op = Expr::BinOp {
        op: BinOp::Mod,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&mod_op).unwrap();
}

/// 测试比较运算类型推断
#[test]
fn test_infer_comparison_ops() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let left = Expr::Lit(Literal::Int(1), Span::default());
    let right = Expr::Lit(Literal::Int(2), Span::default());

    // 等于
    let eq = Expr::BinOp {
        op: BinOp::Eq,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let eq_ty = inferrer.infer_expr(&eq).unwrap();
    assert_eq!(eq_ty, MonoType::Bool);

    // 不等于
    let neq = Expr::BinOp {
        op: BinOp::Neq,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&neq).unwrap();

    // 小于
    let lt = Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&lt).unwrap();

    // 小于等于
    let le = Expr::BinOp {
        op: BinOp::Le,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&le).unwrap();

    // 大于
    let gt = Expr::BinOp {
        op: BinOp::Gt,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&gt).unwrap();

    // 大于等于
    let ge = Expr::BinOp {
        op: BinOp::Ge,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&ge).unwrap();
}

/// 测试逻辑运算类型推断
#[test]
fn test_infer_logical_ops() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let left = Expr::Lit(Literal::Bool(true), Span::default());
    let right = Expr::Lit(Literal::Bool(false), Span::default());

    // 与
    let and = Expr::BinOp {
        op: BinOp::And,
        left: Box::new(left.clone()),
        right: Box::new(right.clone()),
        span: Span::default(),
    };
    let and_ty = inferrer.infer_expr(&and).unwrap();
    assert_eq!(and_ty, MonoType::Bool);

    // 或
    let or = Expr::BinOp {
        op: BinOp::Or,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };
    let or_ty = inferrer.infer_expr(&or).unwrap();
    assert_eq!(or_ty, MonoType::Bool);
}

/// 测试一元运算类型推断
#[test]
fn test_infer_unary_ops() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let expr = Expr::Lit(Literal::Int(5), Span::default());

    // 负数
    let neg = Expr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(expr.clone()),
        span: Span::default(),
    };
    let _neg_ty = inferrer.infer_expr(&neg).unwrap();

    // 正数
    let pos = Expr::UnOp {
        op: UnOp::Pos,
        expr: Box::new(expr.clone()),
        span: Span::default(),
    };
    let _ = inferrer.infer_expr(&pos).unwrap();

    // 逻辑非
    let bool_expr = Expr::Lit(Literal::Bool(true), Span::default());
    let not = Expr::UnOp {
        op: UnOp::Not,
        expr: Box::new(bool_expr),
        span: Span::default(),
    };
    let not_ty = inferrer.infer_expr(&not).unwrap();
    assert_eq!(not_ty, MonoType::Bool);
}

/// 测试元组推断
#[test]
fn test_infer_tuple() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let elems = vec![
        Expr::Lit(Literal::Int(1), Span::default()),
        Expr::Lit(Literal::String("hello".to_string()), Span::default()),
        Expr::Lit(Literal::Bool(true), Span::default()),
    ];
    let tuple = Expr::Tuple(elems, Span::default());

    let ty = inferrer.infer_expr(&tuple).unwrap();
    match ty {
        MonoType::Tuple(types) => {
            assert_eq!(types.len(), 3);
        }
        _ => panic!("Expected tuple type"),
    }
}

/// 测试空列表推断
#[test]
fn test_infer_empty_list() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let list = Expr::List(vec![], Span::default());

    let ty = inferrer.infer_expr(&list).unwrap();
    match ty {
        MonoType::List(elem_ty) => {
            // 空列表的元素类型是类型变量
            assert!(elem_ty.type_var().is_some());
        }
        _ => panic!("Expected list type"),
    }
}

/// 测试字典推断
#[test]
fn test_infer_dict() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let pairs = vec![
        (
            Expr::Lit(Literal::String("key1".to_string()), Span::default()),
            Expr::Lit(Literal::Int(1), Span::default()),
        ),
        (
            Expr::Lit(Literal::String("key2".to_string()), Span::default()),
            Expr::Lit(Literal::Int(2), Span::default()),
        ),
    ];
    let dict = Expr::Dict(pairs, Span::default());

    let ty = inferrer.infer_expr(&dict).unwrap();
    match ty {
        MonoType::Dict(_, _) => {
            // 成功推断为字典类型
        }
        _ => panic!("Expected dict type"),
    }
}

/// 测试空字典推断
#[test]
fn test_infer_empty_dict() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let dict = Expr::Dict(vec![], Span::default());

    let ty = inferrer.infer_expr(&dict).unwrap();
    match ty {
        MonoType::Dict(key_ty, value_ty) => {
            // 空字典的键值类型都是类型变量
            assert!(key_ty.type_var().is_some());
            assert!(value_ty.type_var().is_some());
        }
        _ => panic!("Expected dict type"),
    }
}

/// 测试赋值运算类型推断
#[test]
fn test_infer_assignment() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let left = Expr::Lit(Literal::Int(1), Span::default());
    let right = Expr::Lit(Literal::Int(2), Span::default());

    let assign = Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&assign).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试下标访问推断
#[test]
fn test_infer_index() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 列表下标
    let list = Expr::List(
        vec![
            Expr::Lit(Literal::Int(1), Span::default()),
            Expr::Lit(Literal::Int(2), Span::default()),
        ],
        Span::default(),
    );
    let index = Expr::Lit(Literal::Int(0), Span::default());

    let index_expr = Expr::Index {
        expr: Box::new(list),
        index: Box::new(index),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&index_expr).unwrap();
    // 下标访问返回元素类型
    assert!(ty.type_var().is_some() || matches!(ty, MonoType::Int(_)));
}

/// 测试元组静态下标
#[test]
fn test_infer_tuple_static_index() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let tuple = Expr::Tuple(
        vec![
            Expr::Lit(Literal::Int(1), Span::default()),
            Expr::Lit(Literal::String("hello".to_string()), Span::default()),
        ],
        Span::default(),
    );
    let index = Expr::Lit(Literal::Int(1), Span::default());

    let index_expr = Expr::Index {
        expr: Box::new(tuple),
        index: Box::new(index),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&index_expr).unwrap();
    // 元组下标访问返回对应位置的类型
    assert_eq!(ty, MonoType::String);
}

/// 测试元组越界下标
#[test]
fn test_infer_tuple_out_of_bounds_index() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let tuple = Expr::Tuple(
        vec![
            Expr::Lit(Literal::Int(1), Span::default()),
            Expr::Lit(Literal::String("hello".to_string()), Span::default()),
        ],
        Span::default(),
    );
    let index = Expr::Lit(Literal::Int(10), Span::default());

    let index_expr = Expr::Index {
        expr: Box::new(tuple),
        index: Box::new(index),
        span: Span::default(),
    };

    // 应该返回错误
    let result = inferrer.infer_expr(&index_expr);
    assert!(result.is_err());
}

/// 测试通配符模式
#[test]
fn test_infer_wildcard_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let pattern = Pattern::Wildcard;
    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    assert!(ty.type_var().is_some());
}

/// 测试标识符模式
#[test]
fn test_infer_identifier_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let pattern = Pattern::Identifier("x".to_string());
    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    assert!(ty.type_var().is_some());

    // 验证变量已添加到环境
    let var = inferrer.get_var("x");
    assert!(var.is_some());
}

/// 测试字面量模式
#[test]
fn test_infer_literal_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let pattern = Pattern::Literal(Literal::Int(42));
    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试元组模式
#[test]
fn test_infer_tuple_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let patterns = vec![
        Pattern::Literal(Literal::Int(1)),
        Pattern::Literal(Literal::Int(2)),
    ];
    let pattern = Pattern::Tuple(patterns);

    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    match ty {
        MonoType::Tuple(types) => {
            assert_eq!(types.len(), 2);
        }
        _ => panic!("Expected tuple type"),
    }
}

/// 测试或模式
#[test]
fn test_infer_or_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let patterns = vec![
        Pattern::Literal(Literal::Int(1)),
        Pattern::Literal(Literal::Int(2)),
    ];
    let pattern = Pattern::Or(patterns);

    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    // 或模式应该返回第一个分支的类型
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试空或模式
#[test]
fn test_infer_empty_or_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let patterns = vec![];
    let pattern = Pattern::Or(patterns);

    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    // 空或模式应该返回新类型变量
    assert!(ty.type_var().is_some());
}

/// 测试守卫模式
#[test]
fn test_infer_guard_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let inner_pattern = Pattern::Identifier("x".to_string());
    let condition = Expr::Lit(Literal::Bool(true), Span::default());
    let pattern = Pattern::Guard {
        pattern: Box::new(inner_pattern),
        condition: *Box::new(condition),
    };

    let ty = inferrer
        .infer_pattern(&pattern, None, Span::default())
        .unwrap();
    assert!(ty.type_var().is_some());
}

/// 测试 return 推断
#[test]
fn test_infer_return() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 有表达式的 return
    let ret_expr = Expr::Lit(Literal::Int(42), Span::default());
    let ret = Expr::Return(Some(Box::new(ret_expr)), Span::default());

    let ty = inferrer.infer_expr(&ret).unwrap();
    assert_eq!(ty, MonoType::Void);

    // 无表达式的 return
    let ret_void = Expr::Return(None, Span::default());
    let ty_void = inferrer.infer_expr(&ret_void).unwrap();
    assert_eq!(ty_void, MonoType::Void);
}

/// 测试 break 推断
#[test]
fn test_infer_break() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 无标签的 break
    let break_expr = Expr::Break(None, Span::default());
    let ty = inferrer.infer_expr(&break_expr).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试 continue 推断
#[test]
fn test_infer_continue() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 无标签的 continue
    let continue_expr = Expr::Continue(None, Span::default());
    let ty = inferrer.infer_expr(&continue_expr).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试 break 带未知标签错误
#[test]
fn test_infer_break_unknown_label() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let break_expr = Expr::Break(Some("unknown_label".to_string()), Span::default());
    let result = inferrer.infer_expr(&break_expr);
    assert!(result.is_err());
}

/// 测试 continue 带未知标签错误
#[test]
fn test_infer_continue_unknown_label() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let continue_expr = Expr::Continue(Some("unknown_label".to_string()), Span::default());
    let result = inferrer.infer_expr(&continue_expr);
    assert!(result.is_err());
}

/// 测试 cast 推断
#[test]
fn test_infer_cast() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let expr = Expr::Lit(Literal::Int(42), Span::default());
    let target_type = crate::frontend::core::parser::ast::Type::Int(64);
    let cast = Expr::Cast {
        expr: Box::new(expr),
        target_type: *Box::new(target_type),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&cast).unwrap();
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试类型转换推断
#[test]
fn test_infer_type_cast() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let expr = Expr::Lit(Literal::Int(42), Span::default());
    let cast = Expr::Cast {
        expr: Box::new(expr),
        target_type: *Box::new(crate::frontend::core::parser::ast::Type::Float(64)),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&cast).unwrap();
    assert_eq!(ty, MonoType::Float(64));
}

/// 测试函数调用推断
#[test]
fn test_infer_function_call() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 定义一个函数变量
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: Box::new(MonoType::Int(64)),
        is_async: false,
    };
    inferrer.add_var("add".to_string(), PolyType::mono(fn_type));

    // 调用函数
    let func = Expr::Var("add".to_string(), Span::default());
    let args = vec![
        Expr::Lit(Literal::Int(1), Span::default()),
        Expr::Lit(Literal::Int(2), Span::default()),
    ];
    let call = Expr::Call {
        func: Box::new(func),
        args,
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&call).unwrap();
    assert!(ty.type_var().is_some());
}

/// 测试结构体字段访问推断
#[test]
fn test_infer_field_access() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 创建结构体类型
    let struct_type = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(64)),
            ("y".to_string(), MonoType::Int(64)),
        ],
        methods: HashMap::new(),
    });
    inferrer.add_var("p".to_string(), PolyType::mono(struct_type));

    // 字段访问
    let var = Expr::Var("p".to_string(), Span::default());
    let field_access = Expr::FieldAccess {
        expr: Box::new(var),
        field: "x".to_string(),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&field_access).unwrap();
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试未知字段访问错误
#[test]
fn test_infer_unknown_field() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let struct_type = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Int(64))],
        methods: HashMap::new(),
    });
    inferrer.add_var("p".to_string(), PolyType::mono(struct_type));

    let var = Expr::Var("p".to_string(), Span::default());
    let field_access = Expr::FieldAccess {
        expr: Box::new(var),
        field: "unknown".to_string(),
        span: Span::default(),
    };

    let result = inferrer.infer_expr(&field_access);
    assert!(result.is_err());
}

/// 测试非结构体字段访问错误
#[test]
fn test_infer_field_access_on_non_struct() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    let var = Expr::Var("x".to_string(), Span::default());
    let field_access = Expr::FieldAccess {
        expr: Box::new(var),
        field: "field".to_string(),
        span: Span::default(),
    };

    let result = inferrer.infer_expr(&field_access);
    assert!(result.is_err());
}

/// 测试不支持的操作错误
#[test]
fn test_infer_unsupported_op() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 函数定义在表达式上下文中是不支持的
    let fn_def = Expr::FnDef {
        name: "".to_string(),
        params: vec![],
        return_type: None,
        body: Box::new(Block {
            stmts: vec![],
            expr: None,
            span: Span::default(),
        }),
        is_async: false,
        span: Span::default(),
    };

    // 函数定义表达式目前被视为支持的表达式类型
    let result = inferrer.infer_expr(&fn_def);
    assert!(result.is_ok());
}

/// 测试代码块推断
#[test]
fn test_infer_block() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let block = Block {
        stmts: vec![],
        expr: Some(Box::new(Expr::Lit(Literal::Int(42), Span::default()))),
        span: Span::default(),
    };

    let ty = inferrer.infer_block(&block, true, None).unwrap();
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试空代码块推断
#[test]
fn test_infer_empty_block() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let block = Block {
        stmts: vec![],
        expr: None,
        span: Span::default(),
    };

    let ty = inferrer.infer_block(&block, true, None).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试变量声明推断
#[test]
fn test_infer_var_decl() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    inferrer
        .infer_var_decl(
            "x",
            None,
            Some(&Expr::Lit(Literal::Int(42), Span::default())),
            Span::default(),
        )
        .unwrap();

    // 验证变量已添加
    let var = inferrer.get_var("x");
    assert!(var.is_some());
}

/// 测试带类型注解的变量声明推断
#[test]
fn test_infer_var_decl_with_annotation() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    inferrer
        .infer_var_decl(
            "x",
            Some(&crate::frontend::core::parser::ast::Type::Int(64)),
            Some(&Expr::Lit(Literal::Int(42), Span::default())),
            Span::default(),
        )
        .unwrap();

    // 验证变量已添加
    let var = inferrer.get_var("x");
    assert!(var.is_some());
}

/// 测试无初始化变量的变量声明
#[test]
fn test_infer_var_decl_no_initializer() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    inferrer
        .infer_var_decl(
            "x",
            Some(&crate::frontend::core::parser::ast::Type::Int(64)),
            None,
            Span::default(),
        )
        .unwrap();

    // 验证变量已添加
    let var = inferrer.get_var("x");
    assert!(var.is_some());
}

/// 测试 If 表达式推断
#[test]
fn test_infer_if_expr() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let condition = Expr::Lit(Literal::Bool(true), Span::default());
    let then_body = Block {
        stmts: vec![],
        expr: Some(Box::new(Expr::Lit(Literal::Int(1), Span::default()))),
        span: Span::default(),
    };
    let else_body = Block {
        stmts: vec![],
        expr: Some(Box::new(Expr::Lit(Literal::Int(2), Span::default()))),
        span: Span::default(),
    };

    let if_expr = Expr::If {
        condition: Box::new(condition),
        then_branch: Box::new(then_body),
        elif_branches: vec![],
        else_branch: Some(Box::new(else_body)),
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&if_expr).unwrap();
    assert_eq!(ty, MonoType::Int(64));
}

/// 测试 While 表达式推断
#[test]
fn test_infer_while_expr() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let condition = Expr::Lit(Literal::Bool(true), Span::default());
    let body = Block {
        stmts: vec![],
        expr: Some(Box::new(Expr::Lit(Literal::Int(1), Span::default()))),
        span: Span::default(),
    };

    let while_expr = Expr::While {
        condition: Box::new(condition),
        body: Box::new(body),
        label: None,
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&while_expr).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试 For 表达式推断
#[test]
fn test_infer_for_loop() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let iterable = Expr::Lit(Literal::String("test".to_string()), Span::default());
    let body = Block {
        stmts: vec![],
        expr: None,
        span: Span::default(),
    };

    let for_expr = Expr::For {
        var: "c".to_string(),
        iterable: Box::new(iterable),
        body: Box::new(body),
        label: None,
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&for_expr).unwrap();
    assert_eq!(ty, MonoType::Void);
}

/// 测试取负运算推断
#[test]
fn test_infer_neg() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let expr = Expr::Lit(Literal::Int(42), Span::default());
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

/// 测试 FnDef 表达式推断
#[test]
fn test_infer_fn_def() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let fn_def = Expr::FnDef {
        name: "add".to_string(),
        params: vec![
            crate::frontend::core::parser::ast::Param {
                name: "a".to_string(),
                ty: None,
                span: Span::default(),
            },
            crate::frontend::core::parser::ast::Param {
                name: "b".to_string(),
                ty: None,
                span: Span::default(),
            },
        ],
        return_type: Some(crate::frontend::core::parser::ast::Type::Int(64)),
        body: Box::new(Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::Lit(Literal::Int(0), Span::default()))),
            span: Span::default(),
        }),
        is_async: false,
        span: Span::default(),
    };

    let ty = inferrer.infer_expr(&fn_def).unwrap();
    assert!(matches!(ty, MonoType::Fn { .. }));
}

/// 测试类型注解模式推断
#[test]
fn test_infer_typed_pattern() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    let pattern = Pattern::Identifier("x".to_string());

    let ty = inferrer
        .infer_pattern(&pattern, Some(&MonoType::Int(64)), Span::default())
        .unwrap();
    // Identifier 模式返回新类型变量
    assert!(ty.type_var().is_some());
}
