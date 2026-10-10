//! 表达式推断测试 — 基于语言规范 §3.2 & §4 & RFC-010
//!
//! §3.2: 原类型（Int 默认 8 字节 = Int(64)）
//! §4.1-§4.10: 表达式分类
//! RFC-010: 统一类型语法
//! RFC-029: 模块限定名 `{module_key}.{bare}`（ir_gen 的 `qualify_names` 与
//!   `SymbolTable::qualify` 同源）
//! RFC-039 §3.4.8 ①: 跨模块限定调用 `lib.f(x)` 发泛型实例化请求（generic_id
//!   用限定名，与 merged-IR 的函数表键对齐）

use crate::frontend::core::typecheck::inference::expressions::ExpressionInferrer;
use crate::frontend::core::typecheck::inference::scope::ScopeManager;
use crate::frontend::core::types::{MonoType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::Expr;
use crate::util::span::Span;

use std::collections::HashMap;

use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::module::ModuleInfo;
use crate::middle::passes::mono::instance::InstantiationRequest;

/// 整数字面量表达式 `n`。
fn int_expr(n: i128) -> Expr {
    Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(n),
        Span::dummy(),
    )
}

/// `left <op> right` 二元表达式（dummy span）。
fn binop_expr(
    op: crate::frontend::core::parser::ast::BinOp,
    left: Expr,
    right: Expr,
) -> Expr {
    Expr::BinOp {
        op,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::dummy(),
    }
}

/// 按多文件编排的导出提取管线（tokenize → parse → extract_module_info）构造
/// 模块导出面——full_path 为 `{module_key}.{bare}`，与 `build_registry_from` 同源。
fn module_info_from_source(
    module_key: &str,
    source: &str,
) -> ModuleInfo {
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("模块源码必须可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(
        !parsed.has_errors,
        "模块源码必须可解析：{:?}",
        parsed.errors
    );
    crate::frontend::module::orchestrator::extract_module_info(module_key, &parsed.module)
}

/// 以真实 tokenize → parse → check 管线检查入口源码；`modules` 预先注册进
/// 检查器的模块注册表（等价于多文件编排预构建的用户模块集）。
fn check_entry_with_modules(
    source: &str,
    modules: Vec<ModuleInfo>,
) -> TypeCheckResult {
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("入口源码必须可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(
        !parsed.has_errors,
        "入口源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = crate::frontend::core::typecheck::TypeChecker::new("main");
    for module in modules {
        checker.env().module_registry.register(module);
    }
    checker.check_module_collect_all(&parsed.module)
}

/// 实例化请求快照：`(generic_id 名, 声明序类型参数名, 类型实参)`。
fn request_triples(requests: &[InstantiationRequest]) -> Vec<(String, Vec<String>, Vec<MonoType>)> {
    requests
        .iter()
        .map(|r| {
            (
                r.generic_id().name().to_string(),
                r.generic_id().type_params().to_vec(),
                r.type_args().to_vec(),
            )
        })
        .collect()
}

/// Test context that owns the dependencies borrowed by ExpressionInferrer.
struct TestContext {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    overload_candidates:
        HashMap<String, Vec<crate::frontend::core::typecheck::passes::overload::OverloadCandidate>>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            scope: ScopeManager::new(),
            solver: TypeConstraintSolver::default(),
            overload_candidates: HashMap::new(),
        }
    }

    fn inferrer(&mut self) -> ExpressionInferrer<'_> {
        ExpressionInferrer::new(&mut self.scope, &mut self.solver, &self.overload_candidates)
    }
}

// Happy path 测试

#[test]
fn test_expression_inferrer_creation() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();

    // Assert - 验证创建后能正常推断
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(0),
        Span::dummy(),
    );
    let result = inferrer.infer_expr(&expr);
    assert!(
        result.is_ok(),
        "newly created inferrer should handle basic expressions"
    );
}

/// §3.2: 整数字面量默认推断为 Int(64)（8 字节）
#[test]
fn test_infer_integer_literal() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(42),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert - 规范 §3.2：Int 默认 8 字节 = Int(64)
    assert!(result.is_ok(), "should infer integer literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "integer literal should be Int(64) per spec §3.2"
    );
}

/// §3.2: 字符串字面量推断为 String
#[test]
fn test_infer_string_literal() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::String("hello".to_string()),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should infer string literal");
    assert_eq!(
        result.unwrap(),
        MonoType::make_string(),
        "string literal should be String"
    );
}

/// §3.2: 布尔字面量推断为 Bool
#[test]
fn test_infer_bool_literal() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Bool(true),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should infer bool literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Bool,
        "bool literal should be Bool"
    );
}

// Error path 测试

/// §4: 未定义变量应报错
#[test]
fn test_infer_undefined_variable() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Var("undefined_var".to_string(), Span::dummy());

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_err(), "should fail for undefined variable");
}

// Boundary 测试

/// §3.2 & §4: 嵌套表达式 (1 + 2) * 3 应推断为 Int(64)
#[test]
fn test_infer_nested_expressions() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    // 嵌套表达式：(1 + 2) * 3
    let expr = binop_expr(
        crate::frontend::core::parser::ast::BinOp::Mul,
        binop_expr(
            crate::frontend::core::parser::ast::BinOp::Add,
            int_expr(1),
            int_expr(2),
        ),
        int_expr(3),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert - 断言推断出的具体类型为 Int(64)，而非仅检查 is_ok()
    assert!(result.is_ok(), "should handle nested expressions");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "nested int arithmetic expression should infer to Int(64)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// RFC-039 §3.4.8 ①：跨模块限定调用 lib.f(x) 的泛型实例化请求
// ─────────────────────────────────────────────────────────────────────────────

/// RFC-039 §3.4.8 ① / RFC-029：`use lib` 后限定调用泛型函数 `lib.f(x)` 必须
/// 发出实例化请求，且 `generic_id` 用与 merged-IR 同源的限定名 `lib.f`
///（ir_gen 的 `qualify_names` 把每个模块的顶层函数重写为 `{module_key}.{bare}`；
/// 用裸名 `f` 在 merged-IR 的函数表里匹配不到，请求会被下游静默跳过）。
#[test]
fn test_instantiation_request_qualified_module_call_uses_module_qualified_name() {
    // Arrange：lib 导出泛型 f: (T) -> T；入口整体导入后限定调用
    let lib = module_info_from_source("lib", "f: (T: Type) -> (x: T) -> T = (x) => x\n");
    let source = "use lib\nv = lib.f(42)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：请求携带限定名 lib.f，实参已解成 Int(64)
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![("lib.f".to_string(), Vec::new(), vec![MonoType::Int(64)])],
        "限定调用的 generic_id 必须是限定名 lib.f（与 merged-IR 函数表键同源）"
    );
}

/// RFC-039 §3.4.8 ①：限定名必须能按名解析出被调方的声明序类型参数名——
/// `use std.list` 的 `list.len` 声明为 `(A: Type) -> (&Vec(A)) -> Int`，
/// 请求的 `generic_id` 名 = `std.list.len`、类型参数名 = ["A"]。
#[test]
fn test_instantiation_request_qualified_std_call_resolves_declared_type_params() {
    // Arrange：std.list 由 with_std 预载（含 yx 表面导出），入口限定调用 len
    let source = "use std.list\nxs = [1, 2]\nv = list.len(xs)\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert：限定名 + 声明名 A（限定名解析不出声明名时 arity 无从判定，请求发不出）
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "std.list.len".to_string(),
            vec!["A".to_string()],
            vec![MonoType::Int(64)]
        )],
        "限定名 std.list.len 必须解析出声明序类型参数名 [\"A\"]"
    );
}

/// 行为守恒（验收红线）：无 `use` 的单文件路径一字不改——裸 `Var` 调用仍发
/// **裸名**请求（不带模块前缀），声明名与实参照旧。
#[test]
fn test_instantiation_request_bare_call_keeps_unqualified_name() {
    // Arrange：本模块定义泛型 identity，无模块别名世界
    let source = "identity: (T: Type) -> (x: T) -> T = (x) => x\nv = identity(42)\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "泛型裸调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "identity".to_string(),
            vec!["T".to_string()],
            vec![MonoType::Int(64)]
        )],
        "裸 Var 调用的请求名保持裸名 identity（不得被限定化）"
    );
}

/// 边界：被调方是 FieldAccess 但接收者是**值**（方法调用），不是导入模块别名——
/// 不产生任何实例化请求（请求只属于模块限定调用 `lib.f(x)`）。
#[test]
fn test_instantiation_request_value_receiver_method_call_not_collected() {
    // Arrange：Point 类型上声明方法 get_x，值为接收者调用 p.get_x()
    let source =
        "Point: Type = { x: Int }\nPoint.get_x: (self: &Point) -> Int = (self) => self.x\n\
                p = Point(1)\nv = p.get_x()\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert：值接收者方法调用不是模块限定调用，零请求
    assert!(
        result.diagnostics.is_empty(),
        "方法调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        result.instantiation_requests.is_empty(),
        "值接收者方法调用不得产生实例化请求，实际：{:?}",
        request_triples(&result.instantiation_requests)
    );
}
