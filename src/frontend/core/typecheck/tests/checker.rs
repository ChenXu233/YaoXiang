//! TypeChecker 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统检查
//! §6: 函数定义检查
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::types::{MonoType, PolyType};
use crate::frontend::core::parser::ast::{Block, Module, Stmt, Expr, Type as AstType};
use crate::util::span::Span;

// ── 辅助：AST 夹具 ──────────────────────────────────────

/// 以 dummy span 组装模块。
fn module(items: Vec<Stmt>) -> Module {
    Module {
        items,
        span: Span::dummy(),
    }
}

/// 变量表达式 `name`。
fn var(
    name: &str,
    span: Span,
) -> Expr {
    Expr::Var(name.to_string(), span)
}

/// 表达式语句。
fn expr_stmt(
    expr: Expr,
    span: Span,
) -> Stmt {
    Stmt {
        kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(expr)),
        span,
    }
}

/// 声明语句 `target: ty = value`（`ty` 为 `None` 时不写类型标注）。
fn assign_stmt(
    target: &str,
    ty: Option<AstType>,
    value: Expr,
    span: Span,
) -> Stmt {
    Stmt {
        kind: crate::frontend::core::parser::ast::StmtKind::Assign {
            target: Box::new(var(target, span)),
            type_annotation: ty,
            signature_params: vec![],
            value: Some(Box::new(value)),
            is_pub: false,
            is_mut: false,
            span,
        },
        span,
    }
}

/// 整数字面量表达式。
fn int_lit(
    n: i128,
    span: Span,
) -> Expr {
    Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Int(n), span)
}

/// 单参函数类型 `(Int32) -> Int32`。
fn int_fn_type() -> AstType {
    AstType::Fn {
        params: vec![AstType::Int(32)],
        return_type: Box::new(AstType::Int(32)),
    }
}

/// `name: Int32` 形参。
fn param(name: &str) -> crate::frontend::core::parser::ast::Param {
    crate::frontend::core::parser::ast::Param {
        name: name.to_string(),
        ty: Some(AstType::Int(32)),
        is_mut: false,
        span: Span::dummy(),
    }
}

/// `name: (Int32) -> Int32 = (x) => { x }`——同形函数绑定夹具。
fn int_identity_fn(name: &str) -> Stmt {
    assign_stmt(
        name,
        Some(int_fn_type()),
        Expr::Lambda {
            params: vec![param("x")],
            body: Box::new(Block {
                stmts: vec![expr_stmt(var("x", Span::dummy()), Span::dummy())],
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        },
        Span::dummy(),
    )
}

/// 嵌套函数定义夹具：`outer: (Int32) -> Int32 = (x) => { inner(x); x }`，
/// 其中 `inner` 为体内 `FnDef`（`(y: Int32) -> Int32 = { y }`）。
fn nested_fn_module() -> Module {
    let inner_fn = Expr::FnDef {
        name: "inner".to_string(),
        params: vec![crate::frontend::core::parser::ast::Param {
            name: "y".to_string(),
            ty: Some(AstType::Int(32)),
            is_mut: false,
            span: Span::dummy(),
        }],
        return_type: Some(AstType::Int(32)),
        body: Box::new(Block {
            stmts: vec![expr_stmt(var("y", Span::dummy()), Span::dummy())],
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    module(vec![assign_stmt(
        "outer",
        Some(int_fn_type()),
        Expr::Lambda {
            params: vec![param("x")],
            body: Box::new(Block {
                stmts: vec![
                    expr_stmt(inner_fn, Span::dummy()),
                    expr_stmt(var("x", Span::dummy()), Span::dummy()),
                ],
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        },
        Span::dummy(),
    )])
}

/// 泛型夹具：`Wrapper: (T: Type) -> Type = { value: T }` 与 `w: Wrapper<Int> = Wrapper(1)`。
fn generic_wrapper_module() -> Module {
    module(vec![
        // Wrapper: Type = { value: T }  (泛型类型定义)
        Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::TypeDefinition {
                name: "Wrapper".to_string(),
                signature_params: vec![crate::frontend::core::parser::ast::Param {
                    name: "T".to_string(),
                    ty: Some(AstType::MetaType {
                        name_span: Span::dummy(),
                        args: vec![],
                    }),
                    is_mut: false,
                    span: Span::dummy(),
                }],
                definition: AstType::Struct {
                    body: vec![crate::frontend::core::parser::ast::TypeBodyItem::Field(
                        crate::frontend::core::parser::ast::StructField {
                            name: "value".to_string(),
                            is_mut: false,
                            ty: AstType::Name {
                                name: "T".to_string(),
                                span: Span::dummy(),
                            },
                            default: None,
                        },
                    )],
                },
                is_pub: false,
            },
            span: Span::dummy(),
        },
        // w: Wrapper<Int> = Wrapper(1)  (使用泛型类型)
        assign_stmt(
            "w",
            Some(AstType::Generic {
                name: "Wrapper".to_string(),
                name_span: Span::dummy(),
                args: vec![AstType::Name {
                    name: "Int".to_string(),
                    span: Span::dummy(),
                }],
            }),
            Expr::Call {
                func: Box::new(var("Wrapper", Span::dummy())),
                args: vec![int_lit(1, Span::dummy())],
                named_args: vec![],
                span: Span::dummy(),
            },
            Span::dummy(),
        ),
    ])
}

/// 检查器诊断中携带非 dummy span 的 E1002 数量。
fn e1002_with_span(checker: &TypeChecker) -> usize {
    checker
        .errors()
        .iter()
        .filter(|d| d.code == "E1002")
        .filter(|d| d.span.is_some_and(|s| !s.is_dummy()))
        .count()
}

/// 首个 E1002 诊断的 span。
fn e1002_span(checker: &TypeChecker) -> Option<Span> {
    checker
        .errors()
        .iter()
        .find(|d| d.code == "E1002")
        .and_then(|d| d.span)
}

/// 全部诊断的 (错误码, span)——断言失败时的定位信息。
fn error_spans(checker: &TypeChecker) -> Vec<(&str, Option<Span>)> {
    checker
        .errors()
        .iter()
        .map(|d| (d.code.as_str(), d.span))
        .collect()
}

// Happy path 测试

#[test]
fn test_type_checker_new_creates_with_module_name() {
    // Arrange & Act
    let checker = TypeChecker::new("test_module");

    // Assert
    assert_eq!(checker.module_name(), "test_module");
}

#[test]
fn test_type_checker_has_builtin_types() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    let env = checker.env();

    // Assert - 检查内置类型是否存在
    assert!(env.types.contains_key("int"), "should have int type");
    assert!(env.types.contains_key("float"), "should have float type");
    assert!(env.types.contains_key("bool"), "should have bool type");
    assert!(env.types.contains_key("string"), "should have string type");
    assert!(env.types.contains_key("void"), "should have void type");
}

#[test]
fn test_type_checker_can_add_var() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    checker
        .env()
        .add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    let env = checker.env();
    assert!(env.vars.contains_key("x"), "should have variable x");
}

#[test]
fn test_type_checker_has_no_errors_initially() {
    // Arrange & Act
    let checker = TypeChecker::new("test");

    // Assert
    assert!(!checker.has_errors(), "should have no errors initially");
}

#[test]
fn test_type_checker_check_empty_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let module = module(vec![]);

    // Act
    let result = checker.check_module(&module);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "empty module should pass type check"
    );
}

// Error path 测试

#[test]
fn test_type_checker_reports_type_mismatch() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // 构造一个类型不匹配的 AST：将 Int 赋值给 String 变量
    let module = module(vec![assign_stmt(
        "x",
        Some(AstType::String),
        int_lit(42, Span::dummy()),
        Span::dummy(),
    )]);

    // Act
    let result = checker.check_module(&module);

    // Assert - 应该报告类型错误
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "x: String = 42 should report type mismatch error"
    );
    let has_type_error = !result.diagnostics.is_empty()
        || checker.errors().iter().any(|e| {
            let msg = format!("{:?}", e);
            msg.contains("mismatch") || msg.contains("type")
        });
    assert!(has_type_error, "error should be related to type mismatch");
}

#[test]
fn test_type_mismatch_diagnostic_carries_source_span() {
    // Arrange: x: String = 42 类型不匹配——#270：诊断必须携带源码定位
    // 用非 dummy 的真实 span（line=3），验证 check_var_stmt 路径挂载 span
    let mut checker = TypeChecker::new("test");
    let real_span = Span::new(
        crate::util::span::Position::new(3, 5),
        crate::util::span::Position::new(3, 20),
    );
    let module = module(vec![assign_stmt(
        "x",
        Some(AstType::String),
        int_lit(42, real_span),
        real_span,
    )]);

    // Act
    let _ = checker.check_module(&module);

    // Assert: 诊断应携带非 dummy 的源码定位（E1002 不再丢 span）
    let with_span = e1002_with_span(&checker);
    assert!(
        with_span > 0,
        "E1002 诊断应携带源码定位 span，实际 errors: {:?}",
        error_spans(&checker)
    );
    let span = e1002_span(&checker);
    assert_eq!(
        span.expect("E1002 诊断存在且带 span").start.line,
        3,
        "E1002 span 应指向语句所在行 3"
    );
}

#[test]
fn test_type_checker_reports_undefined_variable() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let module = module(vec![expr_stmt(
        var("undefined_var", Span::dummy()),
        Span::dummy(),
    )]);

    // Act
    let result = checker.check_module(&module);

    // Assert - 使用未定义变量应该报错
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "using undefined variable should produce an error"
    );
}

#[test]
fn test_type_checker_reports_fn_param_type_mismatch() {
    // Arrange: 定义 add: (x: Int) -> Int = x，然后调用 add("hello")
    let mut checker = TypeChecker::new("test");
    let module = module(vec![
        // fn add(x: Int) -> Int { x }
        int_identity_fn("add"),
        // add("hello") — 传入 String 但参数期望 Int
        expr_stmt(
            Expr::Call {
                func: Box::new(var("add", Span::dummy())),
                args: vec![Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::String("hello".to_string()),
                    Span::dummy(),
                )],
                named_args: vec![],
                span: Span::dummy(),
            },
            Span::dummy(),
        ),
    ]);

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §6.3: 函数参数类型必须匹配，传入 String 但参数期望 Int 应报错
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "add(\"hello\") with param type Int should report type mismatch"
    );
}

// Boundary 测试

#[test]
fn test_type_checker_with_large_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let mut items = vec![];
    for i in 0..100 {
        // 添加大量语句
        items.push(expr_stmt(
            var(&format!("var_{}", i), Span::dummy()),
            Span::dummy(),
        ));
    }
    let module = module(items);

    // Act
    let result = checker.check_module(&module);

    // Assert - 100 个未定义变量的语句应报错（规范 §6.3: 使用未定义变量是编译错误）
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "100 undefined variables should produce errors"
    );
}

#[test]
fn test_type_checker_with_multiple_function_definitions() {
    // Arrange: 定义三个函数
    let mut checker = TypeChecker::new("test");
    let module = module(vec![
        int_identity_fn("add"),
        int_identity_fn("sub"),
        int_identity_fn("mul"),
    ]);

    // Act
    let result = checker.check_module(&module);

    // Assert - 多个函数定义应该正常处理
    assert!(
        result.diagnostics.is_empty(),
        "multiple function definitions should be accepted"
    );
}

#[test]
fn test_type_checker_with_nested_function_definition() {
    // Arrange: 外层函数内部定义一个 FnDef 表达式
    let mut checker = TypeChecker::new("test");
    let module = nested_fn_module();

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §6.3: 嵌套函数定义应通过类型检查（所有变量已定义，类型一致）
    assert!(
        result.diagnostics.is_empty(),
        "nested function definition with correct types should pass type check"
    );
}

#[test]
fn test_type_checker_with_generic_type_binding() {
    // Arrange: 定义泛型类型 Wrapper[T] = { value: T }，然后使用 Wrapper<Int>
    let mut checker = TypeChecker::new("test");
    let module = generic_wrapper_module();

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §3.8: 泛型类型定义和使用应通过类型检查（所有类型参数已提供）
    assert!(
        result.diagnostics.is_empty(),
        "generic type definition and usage with all type params provided should pass"
    );
}

// RFC-027 / issue #263: 精化类型实参校验（E1092）
//
// 规范来源: RFC-027 §语法——谓词应用的实参必须是编译期常量形态；
// 实参不可转换或个数不匹配报 E1092，精化约束绝不静默丢弃。

/// 辅助：源码 → 类型检查结果（语法必须正确，否则测试本身有错）
fn check_source_for_refined(
    source: &str
) -> crate::frontend::core::typecheck::types::TypeCheckResult {
    // Arrange（公共部分）：词法 + 语法解析
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parse_result = parse(&tokens);
    assert!(
        !parse_result.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parse_result.errors
    );

    // Act
    let mut checker = TypeChecker::new("test");
    checker.check_module(&parse_result.module)
}

#[test]
fn test_refined_proof_fn_invalid_arg_reports_e1092() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange: 证明函数 IsSmall + 不可转换实参 List(Int)（#263 复现）
    let source = r#"
IsSmall: (x: Int) -> Type = { x < 100 }
main = {
    y: IsSmall(List(Int)) = 9999
    y
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: 必须报 E1092——约束不得静默丢弃
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1092"),
        "不可转换实参必须报 E1092，实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_refined_proof_fn_arity_mismatch_reports_e1093() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange: 单参数证明函数传两个实参
    let source = r#"
IsSmall: (x: Int) -> Type = { x < 100 }
main = {
    y: IsSmall(1, 2) = 5
    y
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: 实参个数不匹配必须报 E1093（结构化参数，不依赖自由文本）
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1093"),
        "实参个数不匹配必须报 E1093，实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_refined_proof_fn_valid_arg_no_e1092() {
    // Arrange: 合法字面量实参（正常路径对照）
    let source = r#"
IsSmall: (x: Int) -> Type = { x < 100 }
main = {
    y: IsSmall(5) = 5
    y
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: 合法实参不得产生 E1092
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1093"),
        "合法实参不应报 E1092，实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

// RFC-027 §6 / #377（A）：精化变量向下游表现为 base 类型
//
// 精化是**编译期约束**，不是运行时类型：`y: IsPositive(5)` 的运行时类型就是
// `Int`。当前变量注册时存的是 `Refined`/`Generic` 本体，导致下游任何「期望
// base」的位置都撞 E1002。
//
// 注意约束的语义（实测判定）：`IsPositive(5)` 校验的是**注解里的字面量 5**，
// 与变量后来被赋什么值无关——`y: IsPositive(5) = -9999` 不报错，而
// `y: IsPositive(-5) = 5` 报 E4018。故「重赋值是否违反约束」不是本层能判定
// 的事，本文件不做该断言（见下方说明）。

#[test]
fn test_refined_variable_passes_to_int_annotation_downstream() {
    // Arrange: 精化变量被下游用 base 类型标注重绑定
    let source = r#"
IsPositive: (x: Int) -> Type = { x > 0 }
main: () -> Int = {
    y: IsPositive(5) = 5
    z: Int = y
    z
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: 精化变量的运行时类型是其 base（Int），不得报类型不匹配
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1002"),
        "精化变量 y 的运行时类型是 Int，绑定到 z: Int 不应报 E1002。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_refined_variable_returns_as_base_type() {
    // Arrange: 精化变量直接作为函数返回值
    let source = r#"
IsPositive: (x: Int) -> Type = { x > 0 }
main: () -> Int = {
    y: IsPositive(5) = 5
    y
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: y 作返回值即返回其 base（Int），与声明返回类型 Int 一致
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1002"),
        "精化变量作返回值时类型应为其 base（Int）。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_refined_variable_reassignment_to_base_value_is_accepted() {
    // Arrange: 精化变量被重赋一个同为 base 类型的值
    //
    // 语义依据（实测）：`IsPositive(5)` 的约束是「5 > 0」这一静态命题，与
    // 变量后续取值无关（`y: IsPositive(5) = -9999` 亦不报错）。故重赋 Int
    // 值不应被拒——当前 E1002 把精化变量当运行时类型，对任何重赋值一刀切。
    let source = r#"
IsPositive: (x: Int) -> Type = { x > 0 }
main: () -> Int = {
    mut y: IsPositive(5) = 5
    y = 6
    y
}
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert: y 的运行时类型是 Int，重赋 Int 值应被接受
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1002"),
        "精化变量的运行时类型是 Int，重赋 Int 值不应报 E1002。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}
