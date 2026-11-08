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

// ─────────────────────────────────────────────────────────────
// SPEC type-system §7.1 —— 编译期值参数的用途精筛（类型构造器面）
//
// 「类型位置」= 声明中的全部 `Type` 子树；类型构造器的落空候选报 E1094、
// 命中者在类型体里被引用则不报。判定单点在 `const_param::used_as_const_in_annotation`。
// ─────────────────────────────────────────────────────────────

/// 规范：SPEC §7.1 真值表第 4 行 —— `Foo: (T: Type, N: Int) -> Type = { x: T }`
/// 的 N 未在类型体引用 ⇒ 落空，类型构造器路径报 E1094（RFC-011 §4.1）。
#[test]
fn test_rfc011_type_constructor_unused_const_param_reports_e1094() {
    // Arrange — SPEC 真值表第 4 行原文
    let source = "Foo: (T: Type, N: Int) -> Type = { x: T }";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1094"),
        "N 标注 Int 但未在类型体引用 ⇒ 类型构造器路径必须报 E1094。实际: {:#?}",
        result.diagnostics
    );
}

/// 规范：SPEC §7.1 真值表第 2 行（正控）—— N 在类型构造实参位 `Array(T, N)`
/// 被引用 ⇒ 编译期值参数，不得报 E1094。
///
/// 正控的作用：防止「一律报 E1094」这类把落空面说过头的实现被误判为合格。
#[test]
fn test_rfc011_type_constructor_used_const_param_is_clean() {
    // Arrange — SPEC §7.1 的 Measure 例
    let source = "Measure: (T: Type, N: Int) -> Type = { data: Array(T, N) }";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1094"),
        "N 在类型构造实参位被引用 ⇒ 命中，不得报 E1094。实际: {:#?}",
        result.diagnostics
    );
}

/// 规范：SPEC §7.1「类型位置指声明中的全部 `Type` 子树」+ RFC-004 括号透明 ——
/// 类型体字段类型写成 `(N)`（`Type::Paren`）时，N 仍是类型位置引用 ⇒ 不得报 E1094。
///
/// 补 `Paren` 臂之前本用例必红：类型构造器面的扫描面漏掉该变体，命中被判成落空。
#[test]
fn test_rfc011_type_constructor_const_param_in_paren_field_is_used() {
    // Arrange — 字段类型是括号形式的 N
    let source = "MeasureP: (T: Type, N: Int) -> Type = { data: (N) }";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1094"),
        "括号类型 `(N)` 的内层是类型位置引用 ⇒ N 命中，不得报 E1094。实际: {:#?}",
        result.diagnostics
    );
}
