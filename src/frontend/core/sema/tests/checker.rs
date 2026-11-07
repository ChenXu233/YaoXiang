//! TypeChecker 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统检查
//! §6: 函数定义检查
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::ast::{Module, PlanId, Stmt, Expr, Type as AstType};
use crate::util::diagnostic::Severity;
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

/// 声明语句 `target: ty = value`（`ty` 为 `None` 时不写类型标注）。
fn assign_stmt(
    target: &str,
    ty: Option<AstType>,
    value: Expr,
    span: Span,
) -> Stmt {
    Stmt {
        kind: crate::ast::StmtKind::Assign {
            target: Box::new(var(target, span)),
            type_annotation: ty,
            value: Some(Box::new(value)),
            is_mut: false,
            span,
        },
        span,
        plan_id: PlanId::UNPLANNED,
    }
}

/// 整数字面量表达式。
fn int_lit(
    n: i128,
    span: Span,
) -> Expr {
    Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Int(n), span)
}

/// 内建类型在 AST 里的唯一写法：Name（WBS 6.3 删除了 Int/Float/… 专用变体）。
fn builtin_ty(name: &str) -> AstType {
    AstType::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
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
fn test_type_checker_has_no_errors_initially() {
    // Arrange & Act
    let checker = TypeChecker::new("test");

    // Assert
    assert!(!checker.has_errors(), "should have no errors initially");
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
        Some(builtin_ty("String")),
        int_lit(42, real_span),
        real_span,
    )]);

    // Act
    let _ = checker.check_module(&module, &TypeLevelNamespace::scan_one(&module));

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

// RFC-027 / issue #263: 精化类型实参校验（E1092）
//
// 规范来源: RFC-027 §语法——谓词应用的实参必须是编译期常量形态；
// 实参不可转换或个数不匹配报 E1092，精化约束绝不静默丢弃。

/// 辅助：源码 → 类型检查结果（语法必须正确，否则测试本身有错）
fn check_source_for_refined(source: &str) -> crate::frontend::core::sema::types::TypeCheckResult {
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
    checker.check_module(
        &parse_result.module,
        &TypeLevelNamespace::scan_one(&parse_result.module),
    )
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

// ==================== WBS 4.4.3：所有权层降级留痕（02 §S5 (d)） ====================

/// 无 `body_checker` 时所有权层输入降级 → **必须**有诊断（WBS 4.4.3）
///
/// 事实：`body_checker` 为 `None` 时所有权层拿到空类型账本 + 空调用点所有权表，
/// 分析退化为「什么都不冲突」——**静默不动作**（RFC-039 D59 C 类 / 02 §S5 (d)）。
/// 生产路径上 `init_body_checker` 先于所有权阶段执行、输入恒存在，故降级分支
/// 不可达；判据抽在 `TypeChecker::ownership_inputs` 上，本用例即它的钉子。
#[test]
fn test_ownership_inputs_without_body_checker_reports_degradation_warning() {
    // Arrange — `TypeChecker::new` 之后、`init_body_checker` 之前：正是降级形态
    let checker = TypeChecker::new("degradation-probe");

    // Act
    let (ledger, call_ownership, degradation) = checker.ownership_inputs(Span::dummy());

    // Assert — 保守方向不变（空输入照跑），但必须留痕
    assert!(
        ledger.is_empty() && call_ownership.is_empty(),
        "无 body_checker 时输入必须是空账本 + 空调用表（保守方向不变）"
    );
    let warning = degradation.expect("无 body_checker 时所有权降级必须留诊断（WBS 4.4.3）");
    assert_eq!(
        warning.code, "W1082",
        "降级诊断须用专用码（既有 W 码事实不重叠：W1081 专指求解器缺失）"
    );
    assert!(
        matches!(warning.severity, Severity::Warning),
        "降级是警告级（不阻断编译），实际: {:?}",
        warning.severity
    );
}

// ── issue #469 处 3：致命绑定错误后的级联 E1001 抑制 ──────────────
//
// 判据（issue #469）：E1014（命名实参名错）使该语句左侧绑定失败后，
// 后续引用不得再级联 E1001 Unknown variable——同一根因只报一条诊断；
// 真正的未定义引用（独立 E1001 场景）行为不变。

#[test]
fn test_failed_binding_error_type_suppresses_cascading_e1001() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange：E1014 形态（issue #469 c4g 样例）——`b` 绑定失败，随后两处引用 `b`
    let source = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int) -> Int = (self, n) => { return n }
main: () -> Void = {
    t = Temp(1)
    b = t.pick(s = 1)
    r = b
    q = b
}
main()
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert
    let e1001_count = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1001")
        .count();
    assert_eq!(
        e1001_count,
        0,
        "绑定失败的毒化登记必须抑制后续引用的级联 E1001（issue #469 处 3）。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
    let e1014_count = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "E1014")
        .count();
    assert_eq!(
        e1014_count,
        1,
        "E1014 本身必须恰好报一次（根因诊断不增不减）. 实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result.diagnostics.len(),
        1,
        "同一根因只报一条诊断：E1014 之后不得出现任何级联（E1001/E2014 等）。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_genuinely_undefined_variable_still_reports_e1001() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange：`y` 从未定义——这是真正的未定义引用，与绑定失败毒化无关
    let source = r#"
main: () -> Void = {
    x = y + 1
}
main()
"#;

    // Act
    let result = check_source_for_refined(source);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1001"),
        "独立的未定义引用必须保持报 E1001（毒化登记不得吞掉真实错误）。实际诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

// #511: 导出登记口（`collect_signatures` 的 Assign 臂）对泛型形参形态的收名判据
//
// 规范来源: type-system.md §泛型（(T: Type) 类型参数 / (T: Add) 接口约束 /
// (N: Int) 值依赖 const 参数）。登记口只认 `Type::MetaType` 时，接口约束与
// 值依赖形态的形参名不随导出携带 ⇒ 跨单元实例化请求走签名反推兜底（params=[]）。
// 判据单点 = `ast::classify_generic_params`；Const 候选须过 RFC-011 §4.1
// 两步判定的第二步（用途精筛），`(n: Int)` 这类值参数段的候选不得混入。

/// 辅助：只跑签名收集阶段（登记口）后取声明名快照。
///
/// `collect_signatures` 不建 body_checker ⇒ `generic_fn_type_params_snapshot`
/// 返回的就是登记口那份 `declared_fn_type_params`——与完整三遍的合并结果可区分。
fn collect_signatures_declared_names(
    source: &str
) -> std::collections::HashMap<String, Vec<String>> {
    // Arrange（公共部分）：词法 + 语法解析
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parse_result = parse(&tokens);
    assert!(
        !parse_result.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parse_result.errors
    );

    // Act：仅收集顶层签名（登记口所在阶段）
    let mut checker = TypeChecker::new("test");
    checker.collect_signatures(
        &parse_result.module,
        &TypeLevelNamespace::scan_one(&parse_result.module),
    );
    checker.generic_fn_type_params_snapshot()
}

#[test]
fn test_collect_signatures_declares_meta_type_annotation_param_name() {
    // Arrange：`(T: Type)` —— MetaType 形态，既有行为回归锚
    let source = "f: (T: Type) -> (x: T) -> T = (x) => x\n";

    // Act
    let names = collect_signatures_declared_names(source);

    // Assert
    assert_eq!(
        names.get("f"),
        Some(&vec!["T".to_string()]),
        "(T: Type) 的形参名必须进登记口声明名表（按声明序）"
    );
}
