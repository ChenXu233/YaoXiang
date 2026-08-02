//! 签名解析测试 — 基于语言规范 §3.7 & RFC-010
//!
//! §3.7: 函数类型
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::signature::parse_signature;
use crate::frontend::core::types::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_parse_signature_simple_function() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Void", &mut env);

    // Assert - 应该返回零参数的函数类型
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert!(params.is_empty(), "零参数函数签名的 params 应为空");
            assert!(
                matches!(*return_type, MonoType::Void),
                "返回类型应为 Void，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_with_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int, Float) -> String", &mut env);

    // Assert - 应该解析为包含两个参数的函数类型
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert_eq!(params.len(), 2, "应有 2 个参数，实际: {}", params.len());
            assert!(
                matches!(params[0], MonoType::Int(64)),
                "第 1 个参数应为 Int(64)，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(params[1], MonoType::Float(64)),
                "第 2 个参数应为 Float(64)，实际: {:?}",
                params[1]
            );
            assert!(
                matches!(*return_type, MonoType::String),
                "返回类型应为 String，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_parse_signature_invalid_syntax() {
    // Arrange - 不以 '(' 开头的非法签名，解析为常量类型名（TypeRef）
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("invalid -> syntax", &mut env);

    // Assert - 非法签名不应该是 Fn 类型，而是降级为 TypeRef
    assert!(
        !matches!(result, MonoType::Fn { .. }),
        "非法签名不应解析为 Fn 类型，实际: {:?}",
        result
    );
    assert!(
        matches!(result, MonoType::TypeRef(_)),
        "非法签名应降级为 TypeRef，实际: {:?}",
        result
    );
}

#[test]
fn test_parse_signature_unmatched_paren() {
    // Arrange - 缺少右括号的签名，触发 unmatched '(' 错误路径
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int", &mut env);

    // Assert - 错误路径返回带类型变量的降级 Fn
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 1, "降级 Fn 应有 1 个类型变量参数");
            assert!(
                matches!(*return_type, MonoType::Void),
                "降级 Fn 返回类型应为 Void，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望降级 Fn 类型，实际得到: {:?}", other),
    }
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_parse_signature_empty_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Int", &mut env);

    // Assert - 空参数列表应该有效
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert!(params.is_empty(), "空参数列表应解析为空 Vec");
            assert!(
                matches!(*return_type, MonoType::Int(64)),
                "返回类型应为 Int(64)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_many_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int, Int, Int, Int, Int) -> Int", &mut env);

    // Assert - 多参数应该有效
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 5, "应有 5 个参数，实际: {}", params.len());
            for (i, param) in params.iter().enumerate() {
                assert!(
                    matches!(param, MonoType::Int(64)),
                    "第 {} 个参数应为 Int(64)，实际: {:?}",
                    i + 1,
                    param
                );
            }
            assert!(
                matches!(*return_type, MonoType::Int(64)),
                "返回类型应为 Int(64)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_nested_function_type() {
    // Arrange - 嵌套函数类型: (Int) -> (Float) -> String
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int) -> (Float) -> String", &mut env);

    // Assert - 外层应为 Fn(Int) -> Fn(Float)->String
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            // 外层参数
            assert_eq!(params.len(), 1, "外层应有 1 个参数，实际: {}", params.len());
            assert!(
                matches!(params[0], MonoType::Int(64)),
                "外层参数应为 Int(64)，实际: {:?}",
                params[0]
            );

            // 内层返回类型应为 Fn(Float) -> String
            match *return_type {
                MonoType::Fn {
                    params: ref inner_params,
                    return_type: ref inner_return,
                    ..
                } => {
                    assert_eq!(
                        inner_params.len(),
                        1,
                        "内层应有 1 个参数，实际: {}",
                        inner_params.len()
                    );
                    assert!(
                        matches!(inner_params[0], MonoType::Float(64)),
                        "内层参数应为 Float(64)，实际: {:?}",
                        inner_params[0]
                    );
                    assert!(
                        matches!(**inner_return, MonoType::String),
                        "内层返回类型应为 String，实际: {:?}",
                        inner_return
                    );
                }
                ref other => panic!("返回类型应为嵌套 Fn，实际: {:?}", other),
            }
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

// ===================================================================
// issue #242：std 签名的真实模式覆盖
// ===================================================================

#[test]
fn test_parse_signature_bracket_generic_prefix_binds_shared_var() {
    // Arrange - [T] 前缀 + 尖括号实参 + 高阶函数参数
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature(
        "[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>",
        &mut env,
    );

    // Assert - T 应绑定为共享类型变量：参数、函数参数与返回值同源
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert_eq!(params.len(), 2, "应有 2 个参数，实际: {}", params.len());
            let list_elem = match &params[0] {
                MonoType::List(inner) => inner.as_ref().clone(),
                other => panic!("第 1 个参数应为 List，实际: {:?}", other),
            };
            let var_index = match list_elem {
                MonoType::TypeVar(tv) => tv.index(),
                other => panic!("List 元素应为绑定后的类型变量，实际: {:?}", other),
            };
            match &params[1] {
                MonoType::Fn {
                    params: fn_params,
                    return_type: fn_ret,
                } => {
                    assert!(
                        matches!(&fn_params[0], MonoType::TypeVar(tv) if tv.index() == var_index),
                        "高阶参数应共享同一类型变量，实际: {:?}",
                        fn_params[0]
                    );
                    assert!(
                        matches!(fn_ret.as_ref(), MonoType::Bool),
                        "高阶返回应为 Bool，实际: {:?}",
                        fn_ret
                    );
                }
                other => panic!("第 2 个参数应为 Fn，实际: {:?}", other),
            }
            match return_type.as_ref() {
                MonoType::List(inner) => assert!(
                    matches!(inner.as_ref(), MonoType::TypeVar(tv) if tv.index() == var_index),
                    "返回 List 元素应与参数共享类型变量，实际: {:?}",
                    inner
                ),
                other => panic!("返回类型应为 List，实际: {:?}", other),
            }
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_bracket_generic_args_arc_weak() {
    // Arrange - 无泛型前缀的方括号实参（T 作为隐式泛型提升，绑定为类型变量）
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(arc: Arc[T]) -> Weak[T]", &mut env);

    // Assert - Arc/Weak 应结构化，T 被提升为隐式泛型（不再是裸 TypeRef）
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert!(
                matches!(&params[0], MonoType::Arc(inner) if !matches!(inner.as_ref(), MonoType::TypeRef(n) if n == "T")),
                "参数应为 Arc[绑定T]，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(return_type.as_ref(), MonoType::Weak(inner) if !matches!(inner.as_ref(), MonoType::TypeRef(n) if n == "T")),
                "返回应为 Weak[绑定T]，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_nested_option_arc() {
    // Arrange - 嵌套方括号泛型 Option[Arc[T]]
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(weak: Weak[T]) -> Option[Arc[T]]", &mut env);

    // Assert - 返回 Option(Arc(T))
    match result {
        MonoType::Fn { return_type, .. } => {
            assert!(
                matches!(return_type.as_ref(), MonoType::Option(inner) if matches!(inner.as_ref(), MonoType::Arc(_))),
                "返回应为 Option[Arc[T]]，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_paren_result_with_concrete_args() {
    // Arrange - 圆括号泛型 Result(Int, Error)
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(s: String) -> Result(Int, Error)", &mut env);

    // Assert - Result(Int64, TypeRef("Error"))
    match result {
        MonoType::Fn { return_type, .. } => match return_type.as_ref() {
            MonoType::Result(ok, err) => {
                assert!(
                    matches!(ok.as_ref(), MonoType::Int(64)),
                    "Result 的 Ok 应为 Int(64)，实际: {:?}",
                    ok
                );
                assert!(
                    matches!(err.as_ref(), MonoType::TypeRef(n) if n == "Error"),
                    "Result 的 Err 应为 TypeRef(Error)，实际: {:?}",
                    err
                );
            }
            other => panic!("返回应为 Result，实际: {:?}", other),
        },
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_variadic_returns_void() {
    // Arrange - 变参 + 单位返回
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(...args) -> ()", &mut env);

    // Assert - 变参为 Any 占位，() 返回为 Void
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert_eq!(
                params.len(),
                1,
                "变参应占 1 个 Any 位，实际: {}",
                params.len()
            );
            assert!(
                matches!(&params[0], MonoType::TypeRef(n) if n == "Any"),
                "变参应为 Any 占位，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(return_type.as_ref(), MonoType::Void),
                "() 返回应为 Void，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_format_with_variadic() {
    // Arrange - 命名参数后跟变参
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(format: String, ...args) -> String", &mut env);

    // Assert - [String, Any] -> String
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert!(
                matches!(&params[0], MonoType::String),
                "format 参数应为 String，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::TypeRef(n) if n == "Any"),
                "变参应为 Any 占位，实际: {:?}",
                params[1]
            );
            assert!(
                matches!(return_type.as_ref(), MonoType::String),
                "返回应为 String，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_optional_param_marker() {
    // Arrange - 可选参数标记 ?msg
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(cond: Bool, ?msg: String) -> Void", &mut env);

    // Assert - 两个精确参数（1 参调用走宽容路径，2 参精确检查）
    match result {
        MonoType::Fn { params, .. } => {
            assert!(
                matches!(&params[0], MonoType::Bool),
                "cond 应为 Bool，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::String),
                "?msg 应按 String 解析，实际: {:?}",
                params[1]
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_bare_containers() {
    // Arrange - 裸容器无类型实参
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(a: List, b: Dict) -> Tuple", &mut env);

    // Assert - 裸 List/Dict 提升为隐式泛型（List(A)/Dict(A, B)，调用点推断）；
    // 裸 Tuple 保持 TypeRef（无开放元组表达）
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert!(
                matches!(&params[0], MonoType::List(inner) if !matches!(inner.as_ref(), MonoType::TypeRef(n) if n == "Any")),
                "裸 List 应为 List(隐式泛型)，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::Dict(..)),
                "裸 Dict 应为 Dict(隐式泛型, 隐式泛型)，实际: {:?}",
                params[1]
            );
            assert!(
                matches!(return_type.as_ref(), MonoType::TypeRef(n) if n == "Tuple"),
                "裸 Tuple 应为 TypeRef，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_untyped_param() {
    // Arrange - 第一个参数无类型标注
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(value, type_name: String) -> String", &mut env);

    // Assert - 无标注参数降级为 TypeRef 占位，有标注参数精确
    match result {
        MonoType::Fn { params, .. } => {
            assert!(
                matches!(&params[0], MonoType::TypeRef(_)),
                "无标注参数应为 TypeRef 占位，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::String),
                "type_name 应为 String，实际: {:?}",
                params[1]
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}
