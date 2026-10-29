//! 括号化函数返回注解的值级分解测试 — 基于 RFC-004 括号语义 / RFC-010 §3 / RFC-010a 规则①
//!
//! issue #501（WBS 3.22.1）：`g: () -> ((x: Int) -> Int) = <body>` 的值级分解
//! 必须把「第一组参数 = 声明的值参数组、内层 Fn = 被返回值的类型」作为单一口径：
//!
//! - RFC-004：返回位置是 `Paren` ⇔ 柯里链条终止——括号声明「这是完整类型（值）」。
//!   因此值级形态 = 外层函数本体，`value_ret` = 内层 Fn **整体**，不得被剥层吸收。
//! - RFC-010 §3：`name: <fn type> = <lambda>` 的 lambda 是函数体语法糖；
//!   RHS 为非 lambda 表达式时，表达式即外层函数体的尾值。
//! - RFC-010a 规则①：函数体尾值 / return 值与声明返回位做统一（E1012）。
//!
//! 四形态（探针 target/yx-scratch/3.22/probe/，行为以语料 03-semantics/paren_return_fn.yx 为锚）：
//! A 函数名 RHS、B lambda 块体 return、C lambda 尾表达式（基线已绿）、
//! D 单层标量返回 + 函数名 RHS（合法报错，必须保持红）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;

/// 辅助函数：解析源代码并类型检查（与 rfc010a.rs 同一 harness 形态）
fn check_source(source: &str) -> TypeCheckResult {
    use crate::util::diagnostic::Diagnostic;
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("词法错误: {}", e),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
    };
    let module = match parse(&tokens) {
        result if result.has_errors => {
            let diag = crate::util::diagnostic::ErrorCodeDefinition::invalid_syntax(&format!(
                "解析错误: {:?}",
                result.errors
            ))
            .build();
            return TypeCheckResult {
                diagnostics: vec![diag],
                ..Default::default()
            };
        }
        result => result.module,
    };
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

/// 规范 RFC-004：返回位括号声明「返回值是一个函数」——函数名作 RHS 时
/// 表达式即外层函数体的尾值，值级返回位 = 内层 Fn 整体（issue #501 形态 A）
#[test]
fn test_paren_return_fn_name_rhs_body_yields_fn_value() {
    // Arrange：声明返回 ((x: Int) -> Int)，体是返回函数值 `inc` 的引用
    let source = r#"
inc: (x: Int) -> Int = (x) => x + 1
g: () -> ((x: Int) -> Int) = inc
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：值级返回位必须保持内层 Fn 整体，不得剥层成 int64（E1012）
    let e1012: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1012")
        .collect();
    assert!(
        e1012.is_empty(),
        "括号化返回注解 + 函数名 RHS 应通过类型检查（值级返回位 = 内层 Fn 整体），实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范 RFC-010 §3 + RFC-010a 规则②：lambda 块体里 `return` 函数值，
/// return 值与值级返回位（内层 Fn 整体）统一（issue #501 形态 B）
#[test]
fn test_paren_return_lambda_block_body_return_fn_value() {
    // Arrange：体是带 return 的 lambda 块——作者 lambda 即外层函数本体
    let source = r#"
inc: (x: Int) -> Int = (x) => x + 1
g: () -> ((x: Int) -> Int) = () => {
    return inc
}
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：不得把作者 lambda 整体再包一层（否则 found 口径 = fn() -> fn(...)）
    let e1012: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1012")
        .collect();
    assert!(
        e1012.is_empty(),
        "括号化返回注解 + lambda 块体 return 函数值应通过类型检查，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范 RFC-004：lambda 尾表达式形态是括号返回的基线绿态（语料
/// 03-semantics/paren_return_fn.yx 的 mk 同形态）——不得被 A/B 修复波及
#[test]
fn test_paren_return_lambda_tail_expr_stays_green() {
    // Arrange：作者 lambda 的参数组即被返回函数的参数组（parser 包装形态）
    let source = r#"
g: () -> ((a: Int) -> Int) = (a) => a + 1
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：基线绿态零 E1012
    let e1012: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1012")
        .collect();
    assert!(
        e1012.is_empty(),
        "lambda 尾表达式的括号返回形态应保持绿态，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范 RFC-010a 规则①：单层标量返回 + 函数名 RHS 是**合法报错**——
/// 体尾值是函数、返回位是标量，E1012（expected 'int64'）必须保持，
/// 值级分解修复不得把它修绿（守卫，issue #501 形态 D）
#[test]
fn test_single_layer_fn_name_rhs_keeps_e1012_scalar_expected() {
    // Arrange：f 返回函数值，h 的返回位是标量 Int——类型不匹配必须报错
    let source = r#"
f: (x: Int) -> Int = (x) => x + 1
h: (x: Int) -> Int = f
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：E1012 必须存在，且期望/实际口径保持原文（expected 'int64'）
    let e1012 = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1012")
        .unwrap_or_else(|| {
            panic!(
                "单层标量返回 + 函数名 RHS 必须保持 E1012（体尾值是函数、返回位是标量），实际诊断: {:?}",
                result.diagnostics
            )
        });
    assert!(
        e1012.message.contains("expected 'int64'")
            && e1012.message.contains("found 'fn(int64) -> int64'"),
        "E1012 的期望/实际口径必须保持标量 vs 函数原文，实际消息: {}",
        e1012.message
    );
}

/// 规范 RFC-011 §4.1 + RFC-004：**裸 Fn 链**（返回位无括号）的柯里语义不变——
/// 平铺柯里块体的体是最内层值函数，值级剥层必须保留（守卫，语料
/// 02-type-system/functions/curry_tail_expr.yx 的 f 同形态）
#[test]
fn test_flat_curry_block_body_keeps_innermost_value_layer() {
    // Arrange：`(a: Int) -> (b: Int) -> Int` 裸链条 = 柯里化，体是最内层
    let source = r#"
f: (a: Int) -> (b: Int) -> Int = {
    a + b
}
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：柯里剥层语义不受括号返回修复影响
    let e1012: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1012")
        .collect();
    assert!(
        e1012.is_empty(),
        "裸 Fn 链柯里块体应保持绿态（体是最内层值函数），实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范 RFC-011 §4.1：const 泛型柯里的值级剥层语义必须保留
/// （守卫，语料 02-type-system/generics/fn_const_generic.yx 同形态）
#[test]
fn test_const_generic_curry_keeps_innermost_value_layer() {
    // Arrange：`(N: Int) -> (n: N) -> Int` const 柯里，体是最内层 n 函数
    let source = r#"
f: (N: Int) -> (n: N) -> Int = (n) => n
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：const 柯里剥层语义不受括号返回修复影响
    let e1012: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1012")
        .collect();
    assert!(
        e1012.is_empty(),
        "const 泛型柯里应保持绿态（体是最内层值函数），实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范 RFC-010a 规则①②：lambda 尾表达式形态（箭头糖 → 体末位是
/// `Return(Lambda)` 语句）的 return 值必须与值级返回位统一——手写嵌套
/// lambda 尾值 `fn() -> fn(int64) -> int64` ≠ 声明的 `fn(int64) -> int64`，
/// 必须编译期 E1012 拒绝，不得静默错配（issue #501 形态 gb2：
/// `g()(1)` 运行期返回 0 参函数对象而非 `inc(1) = 2`）
#[test]
fn test_paren_return_nested_lambda_tail_expr_keeps_e1012() {
    // Arrange：体尾值是 0 参 lambda（返回函数值 inc），参数组与声明内层
    // Fn 值参数组不对齐——作者 lambda 是外层本体，尾值类型是嵌套函数
    let source = r#"
inc: (x: Int) -> Int = (x) => x + 1
g: () -> ((x: Int) -> Int) = () => () => inc
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：E1012 必须存在，且期望/实际口径 = 内层 Fn 整体 vs 嵌套函数
    let e1012 = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1012")
        .unwrap_or_else(|| {
            panic!(
                "嵌套 lambda 尾值与值级返回位不匹配必须报 E1012（静默错配比编译期拒绝更危险），实际诊断: {:?}",
                result.diagnostics
            )
        });
    assert!(
        e1012.message.contains("expected 'fn(int64) -> int64'")
            && e1012.message.contains("found 'fn() -> fn(int64) -> int64'"),
        "E1012 的期望/实际口径必须保持内层 Fn 整体 vs 嵌套函数原文，实际消息: {}",
        e1012.message
    );
}
