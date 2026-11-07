//! 类型检查器测试 — 基于 RFC-010 统一类型语法
//!
//! RFC-010: https://github.com/YaoXiang/YaoXiang/docs/src/rfc/accepted/010-unified-type-syntax.md
//!
//! 测试点：
//! - §3.1: 变量声明 `x: Int = 42`
//! - §3.2: 函数定义 `add: (a: Int, b: Int) -> Int = a + b`
//! - §3.3: 记录类型 `Point: Type = { x: Float, y: Float }`
//! - §3.4: 接口类型 `Drawable: Type = { draw: (Surface) -> Void }`
//! - §3.5: 泛型类型 `List: (T: Type) -> Type = { ... }`
//! - §3.6: 方法定义 `Point.draw: (self: Point, ...) -> ...`

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

/// RFC-010：类型 base 的 `Point.get_x = get_x` 源码（方法登记走类型空间）。
const TYPE_BASE_SRC: &str = r#"
        Point: Type = { x: Float, y: Float }
        get_x: (p: Point) -> Float = { return 1.0 }
        Point.get_x = get_x
    "#;

/// RFC-010：值 base 的 `p.get_x = get_x` 源码（走 pass3 schema 校验）。
const VALUE_BASE_SRC: &str = r#"
        Point: Type = { x: Float, y: Float }
        get_x: (p: Point) -> Float = { return 1.0 }
        p: Point = Point(1.0, 2.0)
        p.get_x = get_x
    "#;

/// 规范（issue #180 F 组）：赋值按 base 语义分流——
/// base 是类型 → 类型空间方法登记；base 是值（实例）→ 不在 pass2 登记为方法
///（值空间字段赋值由 pass3 schema 校验处理）。判别靠类型表/变量表，不靠语法形式。
#[test]
fn test_semantic_dispatch_type_vs_value_base() {
    // 类型 base：Point.get_x = get_x → 登记类型空间方法
    let tokens = tokenize(TYPE_BASE_SRC).expect("tokenize failed");
    let r = parse(&tokens);
    assert!(!r.has_errors, "parse failed: {:?}", r.errors);
    let mut checker = TypeChecker::new("test");
    checker.check_module(&r.module, &TypeLevelNamespace::scan_one(&r.module));
    assert!(
        checker.env().method_bindings.contains_key("Point.get_x"),
        "类型 base 应登记类型空间方法"
    );

    // 值 base：p.get_x = get_x → pass2 不当方法登记（走 pass3 schema 校验）
    let tokens = tokenize(VALUE_BASE_SRC).expect("tokenize failed");
    let r = parse(&tokens);
    assert!(!r.has_errors, "parse failed: {:?}", r.errors);
    let mut checker = TypeChecker::new("test");
    checker.check_module(&r.module, &TypeLevelNamespace::scan_one(&r.module));
    assert!(
        !checker.env().method_bindings.contains_key("p.get_x"),
        "值 base 不应在 pass2 登记为方法（应走 pass3 schema 校验）"
    );
}
