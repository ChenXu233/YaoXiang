//! 高阶类型参数（构造器变量）调用路径测试 — WBS 3.5 / RFC-010 元类型层级
//!
//! RFC-010 §元类型层级：`Type0` 是日常类型、`Type1` 是类型构造器（`List`、`Box`）、
//! `Type2+` 是高阶构造器。`(T: Type) -> Type` 声明的参数是**构造器变量**，在类型体
//! 的应用头部出现（`F(A)`），调用侧从实参类型的头部解出真实构造器。
//!
//! 覆盖：
//! - 3.5.1 推断传类型实参：`Holder(Box(1))` / `Holder([1, 2])` 从实参类型头部解出
//! - 3.5.2 显式类型实参：`Holder(Box, Int)(Box(1))` 与 `probe(Box, Int)(7)`

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;

/// 辅助函数：解析源码并类型检查，返回诊断文本列表（空列表 = 通过）
fn check_source(source: &str) -> Vec<String> {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let result = parse(&tokens);
    assert!(result.errors.is_empty(), "解析失败: {:?}", result.errors);
    let mut checker = TypeChecker::new("test");
    checker
        .check_module(&result.module)
        .diagnostics
        .iter()
        .map(|d| format!("{d}"))
        .collect()
}

#[test]
fn test_hkt_inferred_type_arg_binds_head_constructor() {
    // Arrange：`Holder` 的 F 是构造器变量（Type1）；实参 `Box(1)` 的类型头部是 `Box`
    let source = r#"
        Box: (T: Type) -> Type = { v: T }
        Holder: (F: (T: Type) -> Type, A: Type) -> Type = { field: F(A) }
        main: () -> Void = {
            h = Holder(Box(1))
            x: Box(Int) = h.field
        }
        main()
    "#;

    // Act
    let diagnostics = check_source(source);

    // Assert：头部位置匹配 F=Box、A=Int；此前报 E1003（F 未知名）+ E1002（F(A) 对不上 Box）
    assert!(
        diagnostics.is_empty(),
        "推断传类型实参路径应无诊断，实际: {diagnostics:?}"
    );
}

#[test]
fn test_hkt_explicit_type_arg_accepts_constructor() {
    // Arrange：显式类型实参列表里第一位是构造器名 `Box`
    let source = r#"
        Box: (T: Type) -> Type = { v: T }
        Holder: (F: (T: Type) -> Type, A: Type) -> Type = { field: F(A) }
        main: () -> Void = {
            h: Holder(Box, Int) = Holder(Box, Int)(Box(1))
        }
        main()
    "#;

    // Act
    let diagnostics = check_source(source);

    // Assert：注解与两层构造都不再报 E1002 / E1010
    assert!(
        diagnostics.is_empty(),
        "显式类型实参（构造器）路径应无诊断，实际: {diagnostics:?}"
    );
}

#[test]
fn test_hkt_explicit_type_arg_function_form_accepts_constructor() {
    // Arrange：显式类型实参用在**函数**调用上（`probe(Box, Int)(7)`）——
    // 构造器参数 F 在值级签名里没有槽位，类型实参按声明序跳过它绑定 A
    let source = r#"
        Box: (T: Type) -> Type = { v: T }
        probe: (F: (T: Type) -> Type, A: Type) -> (x: A) -> A = x
        main: () -> Void = {
            n = probe(Box, Int)(7)
        }
        main()
    "#;

    // Act
    let diagnostics = check_source(source);

    // Assert：此前报 E1010（expected 1 arguments, found 2）
    assert!(
        diagnostics.is_empty(),
        "显式类型实参（函数形态）应无诊断，实际: {diagnostics:?}"
    );
}
