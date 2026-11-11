//! 签名解析测试 — 基于语言规范 §3.5/§4.1 & RFC-010
//!
//! §3.5: 函数类型
//! §4.1: 泛型参数语法
//! RFC-010: 统一类型语法
//! #391: 畸形签名（结构错误/裸容器/垃圾 token）一律 Err 硬拒绝，不再降级

use crate::frontend::core::sema::environment::TypeEnvironment;
use crate::frontend::core::sema::signature::parse_signature;
use crate::frontend::core::types::MonoType;

// Happy path 测试

#[test]
fn test_parse_signature_simple_function() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Void", &mut env).expect("合法签名应解析成功");

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
    let result = parse_signature("(Int, Float) -> String", &mut env).expect("合法签名应解析成功");

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
                matches!(*return_type, MonoType::Generic { ref name, .. } if name == "String"),
                "返回类型应为 String，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

// Error path 测试（#391：硬拒绝，不再降级）

#[test]
fn test_parse_signature_invalid_syntax_rejected() {
    // #324：单测直调需模拟 walk 上下文（builder 在 debug 下强制 span）
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange - 含 '->' 与空白的残余不是合法类型表达式
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("invalid -> syntax", &mut env);

    // Assert - 畸形签名报 Err（invalid_signature），不再静默降级 TypeRef
    let diag = result.expect_err("畸形签名应硬拒绝");
    assert_eq!(
        diag.code, "E2090",
        "应报 invalid_signature，实际: {}",
        diag.code
    );
}

// #391：裸容器类型构造期拒绝

#[test]
fn test_parse_signature_bare_container_in_return_rejected() {
    // #324：单测直调需模拟 walk 上下文（builder 在 debug 下强制 span）
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange - 旧 std.os.read_dir 的真实事故形态：`-> List`
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(path: &String) -> List", &mut env);

    // Assert - 专用码 E2096，不再落成垃圾 TypeRef 让用户代码报 E1002
    let diag = result.expect_err("裸容器返回类型应硬拒绝");
    assert_eq!(diag.code, "E2096", "应报 invalid_signature_bare_container");
    assert!(
        diag.message.contains("List"),
        "诊断应点名裸容器名，实际: {}",
        diag.message
    );
}

#[test]
fn test_parse_signature_parameterized_container_accepted() {
    // Arrange - 参数化容器是合法形态（SPEC §4.1.1 泛型构造器）
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(xs: &Vec(String)) -> Dict(String, Int)", &mut env)
        .expect("参数化容器签名应解析成功");

    // Assert
    match result {
        MonoType::Fn { return_type, .. } => {
            assert!(
                matches!(*return_type, MonoType::Generic { ref name, .. } if name == "Dict"),
                "返回类型应为 Dict(Generic)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_unknown_single_token_type_still_typeref() {
    // Arrange - 单 token 未知名是自定义类型的合法通道（File/DateTime/Error）
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(f: File) -> DateTime", &mut env).expect("未知名应走 TypeRef");

    // Assert
    match result {
        MonoType::Fn { return_type, .. } => {
            assert!(
                matches!(*return_type, MonoType::TypeRef(ref n) if n == "DateTime"),
                "返回应为 TypeRef(DateTime)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

// Boundary 测试

#[test]
fn test_parse_signature_empty_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Int", &mut env).expect("合法签名应解析成功");

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
fn test_parse_signature_paren_result_with_concrete_args() {
    // Arrange - 圆括号泛型 Result(Int, Error)
    let mut env = TypeEnvironment::new();

    // Act
    let result =
        parse_signature("(s: String) -> Result(Int, Error)", &mut env).expect("合法签名应解析成功");

    // Assert - Result(Int64, TypeRef("Error"))
    match result {
        MonoType::Fn { return_type, .. } => match return_type.as_ref() {
            MonoType::Generic { name, args } if name == "Result" => {
                assert!(
                    matches!(&args[0], MonoType::Int(64)),
                    "Result 的 Ok 应为 Int(64)，实际: {:?}",
                    args[0]
                );
                assert!(
                    matches!(&args[1], MonoType::TypeRef(n) if n == "Error"),
                    "Result 的 Err 应为 TypeRef(Error)，实际: {:?}",
                    args[1]
                );
            }
            other => panic!("返回应为 Result(Generic)，实际: {:?}", other),
        },
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_format_with_variadic() {
    // Arrange - 命名参数后跟变参
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(format: String, ...args) -> String", &mut env)
        .expect("合法签名应解析成功");

    // Assert - [String, Any] -> String
    match result {
        MonoType::Fn {
            params,
            return_type,
        } => {
            assert!(
                matches!(&params[0], MonoType::Generic { name, .. } if name == "String"),
                "format 参数应为 String，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::TypeRef(n) if n == "Any"),
                "变参应为 Any 占位，实际: {:?}",
                params[1]
            );
            assert!(
                matches!(return_type.as_ref(), MonoType::Generic { name, .. } if name == "String"),
                "返回应为 String，实际: {:?}",
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
    let result = parse_signature("(value, type_name: String) -> String", &mut env)
        .expect("合法签名应解析成功");

    // Assert - 无标注参数降级为 TypeRef 占位，有标注参数精确
    match result {
        MonoType::Fn { params, .. } => {
            assert!(
                matches!(&params[0], MonoType::TypeRef(_)),
                "无标注参数应为 TypeRef 占位，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(&params[1], MonoType::Generic { name, .. } if name == "String"),
                "type_name 应为 String，实际: {:?}",
                params[1]
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}
