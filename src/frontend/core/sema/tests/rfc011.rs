//! 泛型系统测试 — 基于 RFC-011 泛型系统设计
//!
//! RFC-011: https://github.com/YaoXiang/YaoXiang/docs/src/rfc/accepted/011-generic-type-system.md
//!
//! 测试点：
//! - §1: 基础泛型（泛型参数、类型推导、单态化）
//! - §2: 类型约束系统（单一约束、多重约束）
//! - §3: 关联类型（GAT）
//! - §4: 编译期泛型（N: Int、编译期计算）
//! - §6: 函数重载特化

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

/// 辅助函数：解析源代码并类型检查
fn check_source(source: &str) -> crate::frontend::core::sema::types::TypeCheckResult {
    let tokens = tokenize(source).expect("tokenize failed");
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
    let module = result.module;
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module, &TypeLevelNamespace::scan_one(&module))
}

/// 验证 Type 自描述推断：返回类型被正确解析为具体类型
///
/// id(42) 应推断返回 Int，而非未解析的 TypeVar
#[test]
fn test_rfc011_type_description_resolves_return_type() {
    // Arrange
    let source = r#"
        id: (T: Type) -> ((x: T) -> T) = (x) => x
        result = id(42)
    "#;

    // Act
    let check_result = check_source(source);
    assert!(
        check_result.diagnostics.is_empty(),
        "type check should pass, got: {:?}",
        check_result.diagnostics
    );

    // Assert - result 的类型应被推断为 Int
    let result_ty = check_result
        .bindings
        .get("result")
        .expect("result binding should exist");
    let mono = result_ty.body.clone();
    // 展开后应为 Int(64)，而非 TypeVar
    assert!(
        matches!(mono, crate::frontend::core::types::MonoType::Int(_)),
        "id(42) should infer as Int, got: {:?}",
        mono
    );
}
