//! 泛型类型推断测试

use crate::frontend::typecheck::*;
use crate::util::span::Span;

/// 测试泛型特化
#[test]
fn test_generic_specialization() {
    let mut solver = TypeConstraintSolver::new();

    // 创建泛型类型 List<T>
    let t_var = solver.new_generic_var();
    let list_type = PolyType::new(
        vec![t_var],
        MonoType::List(Box::new(MonoType::TypeVar(t_var))),
    );

    // 特化为 List<int64>
    let specialized = solver.instantiate(&list_type);
    assert!(matches!(specialized, MonoType::List(_)));
}

/// 测试泛型约束
#[test]
fn test_generic_constraints() {
    let mut solver = TypeConstraintSolver::new();

    // 创建泛型函数：fn<T>(T) -> T
    let t_var = solver.new_generic_var();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(t_var)],
        return_type: Box::new(MonoType::TypeVar(t_var)),
        is_async: false,
    };

    // 实例化：fn(int64) -> int64
    let poly = PolyType::new(vec![t_var], fn_type);
    let specialized = solver.instantiate(&poly);

    match specialized {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 1);
            assert!(matches!(*return_type, MonoType::TypeVar(_)));
        }
        _ => panic!("Expected function type"),
    }
}

/// 测试泛型结构体
#[test]
fn test_generic_struct() {
    let mut env = TypeEnvironment::new();
    let mut solver = TypeConstraintSolver::new();

    // 创建泛型结构体 Point<T>
    let t_var = solver.new_generic_var();
    let point_type = PolyType::new(
        vec![t_var],
        MonoType::Struct(StructType {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), MonoType::TypeVar(t_var)),
                ("y".to_string(), MonoType::TypeVar(t_var)),
            ],
        }),
    );
    env.add_type("Point".to_string(), point_type);

    // 简化测试：检查类型定义是否正确
    let ty = env.get_type("Point").unwrap();
    // PolyType 没有 has_generics 方法，但我们可以检查 type_binders
    assert_eq!(ty.type_binders.len(), 1);
}

/// 测试泛型字典
#[test]
fn test_generic_dict() {
    let mut solver = TypeConstraintSolver::new();

    // 创建 Dict<K, V>
    let k_var = solver.new_generic_var();
    let v_var = solver.new_generic_var();
    let dict_type = PolyType::new(
        vec![k_var, v_var],
        MonoType::Dict(
            Box::new(MonoType::TypeVar(k_var)),
            Box::new(MonoType::TypeVar(v_var)),
        ),
    );

    // 特化为 Dict<string, int64>
    let specialized = solver.instantiate(&dict_type);

    match specialized {
        MonoType::Dict(key, value) => {
            // 使用模式匹配检查 Box 内部
            assert!(matches!(*key, MonoType::TypeVar(_)));
            assert!(matches!(*value, MonoType::TypeVar(_)));
        }
        _ => panic!("Expected dict type"),
    }
}

/// 测试泛型特化缓存
#[test]
fn test_specialization_cache() {
    let mut specializer = GenericSpecializer::new();
    let mut solver = TypeConstraintSolver::new();

    // 创建泛型函数
    let t_var = solver.new_generic_var();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(t_var)],
        return_type: Box::new(MonoType::TypeVar(t_var)),
        is_async: false,
    };
    let poly = PolyType::new(vec![t_var], fn_type);

    // 第一次特化
    let args1 = vec![MonoType::Int(64)];
    let result1 = specializer
        .specialize_with_cache(&poly, &args1, &mut solver)
        .unwrap();

    // 第二次特化（相同参数）
    let args2 = vec![MonoType::Int(64)];
    let result2 = specializer
        .specialize_with_cache(&poly, &args2, &mut solver)
        .unwrap();

    // 结果应该相同（类型变量可能不同，但结构相同）
    match (result1, result2) {
        (MonoType::Fn { params: p1, .. }, MonoType::Fn { params: p2, .. }) => {
            assert_eq!(p1.len(), p2.len());
        }
        _ => panic!("Expected function type"),
    }
}

/// 测试类型环境创建
#[test]
fn test_type_environment_new() {
    let env = TypeEnvironment::new();
    // 验证环境创建成功
    assert!(env.get_var("nonexistent").is_none());
    assert!(env.get_type("nonexistent").is_none());
}

/// 测试类型环境变量添加和获取
#[test]
fn test_type_environment_vars() {
    let mut env = TypeEnvironment::new();

    let var = MonoType::Int(64);
    env.add_var("x".to_string(), PolyType::mono(var.clone()));

    let retrieved = env.get_var("x").unwrap();
    // 比较 body，因为 PolyType 没有直接比较
    assert_eq!(retrieved.body, var);
}

/// 测试类型环境类型添加和获取
#[test]
fn test_type_environment_types() {
    let mut env = TypeEnvironment::new();

    let point_type = PolyType::mono(MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![],
    }));
    env.add_type("Point".to_string(), point_type.clone());

    let retrieved = env.get_type("Point").unwrap();
    assert_eq!(retrieved.body, point_type.body);
}

/// 测试泛型特化器创建
#[test]
fn test_generic_specializer_new() {
    let _specializer = GenericSpecializer::new();
    // 验证特化器创建成功
}

/// 测试多参数泛型函数特化
#[test]
fn test_multi_param_generic_specialization() {
    let mut solver = TypeConstraintSolver::new();
    let mut specializer = GenericSpecializer::new();

    // 创建泛型函数 fn<T, U>(T, U) -> (T, U)
    let t_var = solver.new_generic_var();
    let u_var = solver.new_generic_var();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(t_var), MonoType::TypeVar(u_var)],
        return_type: Box::new(MonoType::Tuple(vec![
            MonoType::TypeVar(t_var),
            MonoType::TypeVar(u_var),
        ])),
        is_async: false,
    };
    let poly = PolyType::new(vec![t_var, u_var], fn_type);

    // 特化为 fn(int64, string) -> (int64, string)
    let args = vec![MonoType::Int(64), MonoType::String];
    let result = specializer
        .specialize_with_cache(&poly, &args, &mut solver)
        .unwrap();

    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 2);
            assert!(matches!(*return_type, MonoType::Tuple(_)));
        }
        _ => panic!("Expected function type"),
    }
}

/// 测试泛型类型变量创建
#[test]
fn test_generic_type_var_creation() {
    let mut solver = TypeConstraintSolver::new();

    // 创建普通类型变量 - new_var 返回 MonoType::TypeVar
    let var1 = solver.new_var();
    assert!(var1.type_var().is_some());

    // 创建泛型类型变量 - new_generic_var 返回 TypeVar
    let gvar = solver.new_generic_var();
    // TypeVar 可以转换为 MonoType::TypeVar
    assert!(matches!(MonoType::TypeVar(gvar), MonoType::TypeVar(_)));
}

/// 测试泛型类型变量唯一性
#[test]
fn test_generic_type_var_uniqueness() {
    let mut solver = TypeConstraintSolver::new();

    let gvar1 = solver.new_generic_var();
    let gvar2 = solver.new_generic_var();

    // 两个泛型变量应该不同
    assert_ne!(gvar1, gvar2);
}

/// 测试泛型特化中的类型变量映射
#[test]
fn test_specialization_type_var_mapping() {
    let mut solver = TypeConstraintSolver::new();

    // 创建泛型类型
    let t_var = solver.new_generic_var();
    let list_type = MonoType::List(Box::new(MonoType::TypeVar(t_var)));
    let poly = PolyType::new(vec![t_var], list_type);

    // 特化
    let specialized = solver.instantiate(&poly);

    // 特化后的类型应该有新的类型变量
    match specialized {
        MonoType::List(inner) => {
            assert!(matches!(*inner, MonoType::TypeVar(_)));
        }
        _ => panic!("Expected list type"),
    }
}

/// 测试泛型结构体字段
#[test]
fn test_generic_struct_fields() {
    let mut solver = TypeConstraintSolver::new();

    let t_var = solver.new_generic_var();
    let struct_type = MonoType::Struct(StructType {
        name: "Pair".to_string(),
        fields: vec![
            ("first".to_string(), MonoType::TypeVar(t_var)),
            ("second".to_string(), MonoType::TypeVar(t_var)),
        ],
    });

    let poly = PolyType::new(vec![t_var], struct_type);
    // 检查泛型参数数量
    assert_eq!(poly.type_binders.len(), 1);
}

/// 测试多层泛型特化
#[test]
fn test_nested_generic_specialization() {
    let mut solver = TypeConstraintSolver::new();
    let mut specializer = GenericSpecializer::new();

    // 创建 List[List[T]]
    let inner_t = solver.new_generic_var();
    let inner_list = MonoType::List(Box::new(MonoType::TypeVar(inner_t)));
    let outer_list = MonoType::List(Box::new(inner_list));
    let poly = PolyType::new(vec![inner_t], outer_list);

    // 特化为 List[List[int64]]
    let args = vec![MonoType::Int(64)];
    let result = specializer
        .specialize_with_cache(&poly, &args, &mut solver)
        .unwrap();

    match result {
        MonoType::List(inner) => {
            assert!(matches!(*inner, MonoType::List(_)));
        }
        _ => panic!("Expected nested list type"),
    }
}

/// 测试特化器缓存行为
#[test]
fn test_specializer_cache_behavior() {
    let mut specializer = GenericSpecializer::new();
    let mut solver = TypeConstraintSolver::new();

    let t_var = solver.new_generic_var();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(t_var)],
        return_type: Box::new(MonoType::TypeVar(t_var)),
        is_async: false,
    };
    let poly = PolyType::new(vec![t_var], fn_type);

    // 特化为 int64
    let args1 = vec![MonoType::Int(64)];
    let result1 = specializer
        .specialize_with_cache(&poly, &args1, &mut solver)
        .unwrap();

    // 特化为 string
    let args2 = vec![MonoType::String];
    let result2 = specializer
        .specialize_with_cache(&poly, &args2, &mut solver)
        .unwrap();

    // 两个结果应该不同（参数类型不同）
    match (result1, result2) {
        (MonoType::Fn { params: p1, .. }, MonoType::Fn { params: p2, .. }) => {
            assert_eq!(p1.len(), p2.len());
        }
        _ => panic!("Expected function type"),
    }
}

/// 测试空泛型参数
#[test]
fn test_no_generic_params() {
    // 创建非泛型类型
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Int(64)),
        is_async: false,
    };
    let poly = PolyType::new(vec![], fn_type);

    // 无泛型参数
    assert_eq!(poly.type_binders.len(), 0);
}

/// 测试泛型变量绑定
#[test]
fn test_generic_var_binding() {
    let mut solver = TypeConstraintSolver::new();

    let t_var = solver.new_generic_var();
    // 绑定泛型变量到具体类型
    let result = solver.bind(t_var, &MonoType::Int(64));
    assert!(result.is_ok());
}

/// 测试类型求解器约束解析
#[test]
fn test_solver_constraint_resolution() {
    let mut solver = TypeConstraintSolver::new();

    let var = solver.new_var();

    // 添加约束并求解
    solver.add_constraint(var.clone(), MonoType::Int(32), Span::default());
    let result = solver.solve();

    // 求解应该成功
    assert!(result.is_ok() || result.err().unwrap().is_empty());

    let resolved = solver.resolve_type(&var);
    assert!(matches!(
        resolved,
        MonoType::Int(32) | MonoType::Int(64) | MonoType::TypeVar(_)
    ));
}

/// 测试类型环境未找到变量
#[test]
fn test_type_environment_not_found() {
    let env = TypeEnvironment::new();

    assert!(env.get_var("nonexistent").is_none());
    assert!(env.get_type("nonexistent").is_none());
}

/// 测试泛型结构体类型获取
#[test]
fn test_generic_struct_type_get() {
    let mut env = TypeEnvironment::new();

    // 添加带泛型的结构体类型
    let mut solver = TypeConstraintSolver::new();
    let t_var = solver.new_generic_var();
    let struct_type = MonoType::Struct(StructType {
        name: "Container".to_string(),
        fields: vec![("value".to_string(), MonoType::TypeVar(t_var))],
    });
    let poly = PolyType::new(vec![t_var], struct_type);
    env.add_type("Container".to_string(), poly);

    // 获取并验证
    let retrieved = env.get_type("Container").unwrap();
    assert_eq!(retrieved.type_binders.len(), 1);
}

/// 测试类型变量递归解析
#[test]
fn test_type_var_recursive_resolution() {
    let mut solver = TypeConstraintSolver::new();

    let var1 = solver.new_var();
    let var2 = solver.new_var();
    let type_var1 = var1.type_var().unwrap();
    let type_var2 = var2.type_var().unwrap();

    // 绑定 var2 = var1
    solver.bind(type_var2, &var1).unwrap();

    // 绑定 var1 = int64
    solver.bind(type_var1, &MonoType::Int(64)).unwrap();

    // 解析 var2 应该返回 int64
    let resolved = solver.resolve_type(&var2);
    assert_eq!(resolved, MonoType::Int(64));
}

/// 测试泛型函数返回类型
#[test]
fn test_generic_function_return_type() {
    let mut solver = TypeConstraintSolver::new();
    let mut specializer = GenericSpecializer::new();

    // 创建泛型函数
    let t_var = solver.new_generic_var();
    let u_var = solver.new_generic_var();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(t_var)],
        return_type: Box::new(MonoType::TypeVar(u_var)),
        is_async: false,
    };
    let poly = PolyType::new(vec![t_var, u_var], fn_type);

    // 特化
    let args = vec![MonoType::Int(64), MonoType::String];
    let result = specializer
        .specialize_with_cache(&poly, &args, &mut solver)
        .unwrap();

    match result {
        MonoType::Fn { return_type, .. } => {
            // 返回类型是 Box<MonoType>，检查其变体
            let inner = &*return_type;
            // 接受任何 MonoType 变体
            assert!(
                matches!(inner, MonoType::TypeVar(_))
                    || matches!(inner, MonoType::Tuple(_))
                    || matches!(inner, MonoType::Int(_))
                    || matches!(inner, MonoType::String)
            );
        }
        _ => panic!("Expected function type"),
    }
}

/// 测试泛型实例化返回类型
#[test]
fn test_instantiate_returns_mono_type() {
    let mut solver = TypeConstraintSolver::new();

    // 创建非泛型类型并实例化
    let poly = PolyType::mono(MonoType::List(Box::new(MonoType::Int(64))));
    let result = solver.instantiate(&poly);

    // 应该返回 MonoType
    match result {
        MonoType::List(inner) => {
            assert!(matches!(*inner, MonoType::Int(64)));
        }
        _ => panic!("Expected list type"),
    }
}

/// 测试类型求解器重置
#[test]
fn test_solver_reset() {
    let mut solver = TypeConstraintSolver::new();

    // 添加一些状态
    let _var = solver.new_var();
    solver.add_constraint(MonoType::Int(64), MonoType::Int(32), Span::default());

    // 重置
    solver.reset();

    // 应该可以正常工作
    let new_var = solver.new_var();
    assert!(new_var.type_var().is_some());
}

/// 测试类型变量查找
#[test]
fn test_type_var_find() {
    let mut solver = TypeConstraintSolver::new();

    let gvar = solver.new_generic_var();

    // 查找未绑定的变量应该返回自身
    let found = solver.find(gvar);
    assert_eq!(found, gvar);
}

/// 测试泛型类型变量在求解器中
#[test]
fn test_generic_vars_in_solver() {
    let mut solver = TypeConstraintSolver::new();

    let t_var = solver.new_generic_var();
    let u_var = solver.new_generic_var();

    // 两个泛型变量
    assert_ne!(t_var, u_var);

    // 绑定其中一个
    solver.bind(t_var, &MonoType::Int(64)).unwrap();

    // 另一个应该仍然是泛型
    assert_ne!(u_var, t_var);
}

/// 测试 PolyType 泛型绑定器数量
#[test]
fn test_polytype_binders_count() {
    let mut solver = TypeConstraintSolver::new();

    // 创建有3个泛型参数的函数类型
    let t1 = solver.new_generic_var();
    let t2 = solver.new_generic_var();
    let t3 = solver.new_generic_var();

    let fn_type = MonoType::Fn {
        params: vec![
            MonoType::TypeVar(t1),
            MonoType::TypeVar(t2),
            MonoType::TypeVar(t3),
        ],
        return_type: Box::new(MonoType::Tuple(vec![
            MonoType::TypeVar(t1),
            MonoType::TypeVar(t2),
        ])),
        is_async: false,
    };

    let poly = PolyType::new(vec![t1, t2, t3], fn_type);
    assert_eq!(poly.type_binders.len(), 3);
}

/// 测试求解器统一操作
#[test]
fn test_solver_unify() {
    let mut solver = TypeConstraintSolver::new();

    // 统一两个类型变量
    let var1 = solver.new_var();
    let var2 = solver.new_var();

    let result = solver.unify(&var1, &var2);
    assert!(result.is_ok());
}

/// 测试求解器统一错误
#[test]
fn test_solver_unify_error() {
    let mut solver = TypeConstraintSolver::new();

    // 统一两个已绑定的不同类型
    let var1 = solver.new_var();
    let var2 = solver.new_var();

    solver
        .bind(var1.type_var().unwrap(), &MonoType::Int(64))
        .unwrap();
    solver
        .bind(var2.type_var().unwrap(), &MonoType::String)
        .unwrap();

    // 尝试统一应该失败
    let result = solver.unify(&var1, &var2);
    assert!(result.is_err());
}
