//! 6.9.5-B.3 P0：类型级证据字段路径的解析 / 转换 / 谓词登记
//!
//! 规范锚：RFC-027 §14.5；SPEC §8.3.5；09a §P6 6.9.5-B.3 契约 §四（P0）与 §七（T01–T04）。
//!
//! P0 只覆盖**表示与登记**，不含 admission 语义（D2 裁决后另落）。每个判据都走真实
//! lex → parse，或对真实解析产物直接驱动转换点；不构造第二套项表示。

use crate::ast::{Expr, StmtKind, Type, TypeBodyItem};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::predicate_resolver::PredicateResolver;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::record_model::{field_path_base_matches, witness_premise_admitted};
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::core::types::const_data::ConstExpr;
use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;
use crate::frontend::core::types::mono::MonoType;
use crate::util::span::Span;

/// 真实 lex → parse；解析失败时把错误列表附在断言消息里。
fn parse_ok(source: &str) -> crate::ast::Module {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须能解析；错误: {:?}",
        parsed.errors
    );
    parsed.module
}

/// 真实 lex → parse → 收集式类型检查（与 B.2 测试同一入口）。
fn check_collect(
    source: &str
) -> (
    crate::frontend::core::sema::types::TypeCheckResult,
    TypeChecker,
) {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须能解析；错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("record-witness-term");
    let result = checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    (result, checker)
}

fn has_code(
    result: &TypeCheckResult,
    code: &str,
) -> bool {
    result.diagnostics.iter().any(|diag| diag.code == code)
}

/// 取语句 name = value 的右值（测试夹具里的唯一赋值形态）。
fn assignment_value<'a>(
    module: &'a crate::ast::Module,
    name: &str,
) -> &'a Expr {
    for stmt in &module.items {
        if let StmtKind::Assign {
            target,
            value: Some(value),
            ..
        } = &stmt.kind
        {
            if matches!(target.as_ref(), Expr::Var(n, _) if n == name) {
                return value;
            }
        }
    }
    panic!("夹具缺少赋值目标 {name}");
}

/// T01：类型位单层路径 bb.w 解析为 Type::ConstExpr(Expr::FieldAccess{Var(bb), w})。
#[test]
fn test_type_level_field_path_parses_to_field_access() {
    // Arrange：Form B 的谓词声明体（真实源码）。
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n";

    // Act
    let module = parse_ok(source);
    let holds = module.items.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::TypeDefinition {
            name, definition, ..
        } if name == "Holds" => Some(definition),
        _ => None,
    });

    // Assert
    let Some(Type::Struct { body }) = holds else {
        panic!("Holds 必须是 TypeDefinition 的 Struct 体；实际: {holds:?}");
    };
    let constraint = body.iter().find_map(|item| match item {
        TypeBodyItem::Expr(Type::ConstExpr(expr)) => Some(expr.as_ref()),
        _ => None,
    });
    let Some(Expr::FieldAccess { expr, field, .. }) = constraint else {
        panic!("Holds 的约束体必须是 FieldAccess；实际: {constraint:?}");
    };
    assert!(
        matches!(expr.as_ref(), Expr::Var(base, _) if base == "bb"),
        "字段路径基座必须是 Var(bb)；实际: {expr:?}"
    );
    assert_eq!(field, "w", "字段名必须是源码字段名 w");
}

/// T02：转换点对 Var.field 产出 ConstExpr::FieldPath（不是 NamedVar 折平）。
#[test]
fn test_convert_field_access_var_base_yields_field_path() {
    // Arrange
    let expr = Expr::FieldAccess {
        expr: Box::new(Expr::Var("b".to_string(), Span::dummy())),
        field: "w".to_string(),
        span: Span::dummy(),
    };

    // Act
    let converted = convert_expr_to_const_expr(&expr);

    // Assert
    let Some(ConstExpr::FieldPath { base, field }) = converted else {
        panic!("Var.field 必须转换为 FieldPath；实际: {converted:?}");
    };
    assert!(
        matches!(base.as_ref(), ConstExpr::NamedVar(name) if name == "b"),
        "FieldPath 基座必须是 NamedVar(b)；实际: {base:?}"
    );
    assert_eq!(field, "w", "FieldPath 字段名必须保持源码字段名");
}

/// T03：非 Var 基座（嵌套路径 / 调用 / 字面量）一律不产生 FieldPath。
#[test]
fn test_convert_non_var_base_does_not_yield_field_path() {
    // Arrange：真实解析产物（嵌套路径、调用基座）＋ 一个字面量基座。
    let nested_module = parse_ok("t = a.b.c\n");
    let nested = assignment_value(&nested_module, "t");
    let call_module = parse_ok("f: () -> Int = { 1 }\nt = f().w\n");
    let call = assignment_value(&call_module, "t");
    let literal_module = parse_ok("t = 3\n");
    let literal = assignment_value(&literal_module, "t");
    let literal_base = Expr::FieldAccess {
        expr: Box::new(literal.clone()),
        field: "w".to_string(),
        span: Span::dummy(),
    };

    // Act + Assert
    for (label, expr) in [("a.b.c", nested), ("f().w", call), ("3.w", &literal_base)] {
        assert!(
            convert_expr_to_const_expr(expr).is_none(),
            "{label} 的基座不是 Var，不得产生 FieldPath；实际: {:?}",
            convert_expr_to_const_expr(expr)
        );
    }
}

/// T04：P0 登记——Form B 声明进 predicate_defs，且 Holds(b) 正格化为
/// Refined { base: Box, constraint: FieldPath(NamedVar(b), w) }。
#[test]
fn test_predicate_body_field_path_registers_and_resolves() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n\nneedW: (b: Box, h: Holds(b)) -> Int = { b.v }\n";

    // Act
    let (result, mut checker) = check_collect(source);

    // Assert：声明期无未知类型/未知谓词/arity 诊断（P0 要求零新诊断）。
    for code in ["E0010", "E1002", "E1003", "E1010"] {
        assert!(
            !has_code(&result, code),
            "P0 声明期不得出现 {code}；诊断: {:?}",
            result
                .diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>()
        );
    }
    let env = checker.env();
    let def = env
        .predicate_defs
        .get("Holds")
        .expect("Holds 必须登记为编译期谓词");
    assert_eq!(
        def.param_type,
        MonoType::TypeRef("Box".to_string()),
        "谓词形参类型必须是 Box（记录类型）"
    );
    assert!(
        matches!(
            &def.constraint,
            ConstExpr::FieldPath { base, field }
                if matches!(base.as_ref(), ConstExpr::NamedVar(name) if name == "bb") && field == "w"
        ),
        "谓词约束体必须是 FieldPath(NamedVar(bb), w)；实际: {:?}",
        def.constraint
    );

    // Holds(b) 应用：代入后基名与实参同名（应用面规则在 D2 后落地）。
    let resolved =
        PredicateResolver::try_resolve(env, "Holds", &[MonoType::TypeRef("Box".to_string())]);
    let Some(Ok(MonoType::Refined { base, constraint })) = resolved else {
        panic!("Holds(Box) 必须正格化为 Refined；实际: {resolved:?}");
    };
    assert!(
        matches!(base.as_ref(), MonoType::TypeRef(name) if name == "Box"),
        "Refined.base 必须是 Box；实际: {base:?}"
    );
    assert!(
        matches!(
            &constraint,
            ConstExpr::FieldPath { base, field }
                if matches!(base.as_ref(), ConstExpr::NamedVar(name) if name == "Box") && field == "w"
        ),
        "Refined.constraint 必须是 FieldPath(NamedVar(Box), w)；实际: {constraint:?}"
    );
}
/// T05：admission 正控——needW(b, b) 的第二实参位 Holds(b) 由 admission 放行。
#[test]
fn test_admission_accepts_evidence_path_premise() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n\nneedW: (b: Box, h: Holds(b)) -> Int = { b.v }\n\nmain: () -> Void = {\n    b = Box(3)\n    x = needW(b, b)\n}\n";

    // Act
    let (result, _checker) = check_collect(source);

    // Assert
    assert!(
        !has_code(&result, "E2031") && !has_code(&result, "E4018"),
        "admission 必须放行 Holds(b)；诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
    );
}

/// T06：值级读取仍报 E1114（B.3 不改既有拒绝面）。
#[test]
fn test_value_level_witness_read_still_rejected() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nmain: () -> Void = {\n    b = Box(3)\n    x = b.w\n}\n";

    // Act
    let (result, _checker) = check_collect(source);

    // Assert
    assert!(
        has_code(&result, "E1114"),
        "值级读取必须保持 E1114；诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
    );
}

/// T07/T08：声明面拒绝——数据字段路径报 E1092；未知字段报 E1042。
#[test]
fn test_declaration_face_rejects_data_and_unknown_fields() {
    // Arrange
    let data_source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nBad: (bb: Box) -> Type = { bb.v }\n";
    let unknown_source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nBad: (bb: Box) -> Type = { bb.zzz }\n";

    // Act
    let (data_result, _) = check_collect(data_source);
    let (unknown_result, _) = check_collect(unknown_source);

    // Assert
    assert!(has_code(&data_result, "E1092"), "数据字段路径必须报 E1092");
    assert!(has_code(&unknown_result, "E1042"), "未知字段必须报 E1042");
}

/// T09：应用面拒绝——非 NamedVar 基座（Holds(3)）与注解里的证据路径实参。
#[test]
fn test_application_face_rejects_literal_base_and_path_arg() {
    // Arrange
    let literal_source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n\nf: (x: Holds(3)) -> Int = { x.v }\n";
    let path_arg_source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n\nf: (x: Holds(g.w)) -> Int = { x.v }\n";

    // Act
    let (literal_result, _) = check_collect(literal_source);
    let (path_arg_result, _) = check_collect(path_arg_source);

    // Assert
    assert!(
        has_code(&literal_result, "E1092"),
        "Holds(3) 的字面量基座必须报 E1092"
    );
    assert!(
        has_code(&path_arg_result, "E1092"),
        "注解实参里的证据路径必须报 E1092"
    );
}

/// T11/T12：判定面/应用面纯函数直驱（与 T05 的端到端互补）。
#[test]
fn test_admission_predicates_direct_drive() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nPoint: Type = {\n    x: Int,\n}\n";
    let (_result, mut checker) = check_collect(source);
    let env = checker.env();
    let models = &env.record_models;
    let path = |base: &str, field: &str| ConstExpr::FieldPath {
        base: Box::new(ConstExpr::NamedVar(base.to_string())),
        field: field.to_string(),
    };
    let refined = |base: MonoType, constraint: ConstExpr| MonoType::Refined {
        base: Box::new(base),
        constraint,
    };

    // Act + Assert：判定面只判模型+槽（不看 base 名）。
    assert!(witness_premise_admitted(
        models,
        &refined(MonoType::TypeRef("Box".into()), path("b", "w"))
    ));
    assert!(!witness_premise_admitted(
        models,
        &refined(MonoType::TypeRef("Box".into()), path("b", "v"))
    ));
    assert!(!witness_premise_admitted(
        models,
        &refined(MonoType::TypeRef("Int".into()), path("b", "w"))
    ));
    assert!(!witness_premise_admitted(
        models,
        &refined(MonoType::TypeRef("Point".into()), path("p", "x"))
    ));
    // 应用面：基名 == 实参名；非 NamedVar 基座一律 false。
    assert!(field_path_base_matches(
        &refined(MonoType::TypeRef("Box".into()), path("zz", "w")),
        "zz"
    ));
    assert!(!field_path_base_matches(
        &refined(MonoType::TypeRef("Box".into()), path("bb", "w")),
        "zz"
    ));
    let literal_base = refined(
        MonoType::TypeRef("Box".into()),
        ConstExpr::FieldPath {
            base: Box::new(ConstExpr::Lit(
                crate::frontend::core::types::const_data::ConstValue::Int(3),
            )),
            field: "w".to_string(),
        },
    );
    assert!(!field_path_base_matches(&literal_base, "3"));
}

/// T15：未支持求值器遇 FieldPath 返回 Err（不静默当 NamedVar）。
#[test]
fn test_const_evaluator_rejects_field_path() {
    // Arrange
    use crate::frontend::core::types::eval::const_eval::ConstGenericEval;
    let term = ConstExpr::FieldPath {
        base: Box::new(ConstExpr::NamedVar("b".into())),
        field: "w".into(),
    };

    // Act：基名已绑定也必须拒绝（不得按 NamedVar 查表取值）
    let mut evaluator = ConstGenericEval::new();
    evaluator.bind_var(
        "b".to_string(),
        crate::frontend::core::types::const_data::ConstValue::Int(3),
    );
    let evaluated = evaluator.eval(&term);

    // Assert
    assert!(
        evaluated.is_err(),
        "FieldPath 在基名已绑定时仍不得被求值器当值算出；实际: {evaluated:?}"
    );
}
/// T10：声明面拒绝非形参基座（other.w）——E1092（不是静默 Unproven）。
#[test]
fn test_declaration_face_rejects_non_parameter_base() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nBad: (bb: Box) -> Type = { other.w }\n";

    // Act
    let (result, _) = check_collect(source);

    // Assert
    assert!(
        has_code(&result, "E1092"),
        "非形参基座必须报 E1092；诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
    );
}

/// T14：返回位同源——make 返回 Box 值，返回位 Holds(r) 由 admission 放行。
#[test]
fn test_return_position_admission() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n\nHolds: (bb: Box) -> Type = { bb.w }\n\nmake: () -> (r: Holds(r)) = { Box(3) }\n";

    // Act
    let (result, _) = check_collect(source);

    // Assert
    assert!(
        !has_code(&result, "E2031") && !has_code(&result, "E4018"),
        "返回位 admission 必须放行；诊断: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
    );
}
/// T16：admission 在判定面短路——即使 Full 档、无绑定，也不进 SMT/求值器。
#[test]
fn test_admission_short_circuits_before_smt() {
    // Arrange
    use crate::frontend::core::sema::layers::predicate::PredicateMode;
    use crate::frontend::core::sema::proof::context::ProofContext;
    let source =
        "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n}\n";
    let (_result, mut checker) = check_collect(source);
    let env = checker.env();
    let ctx = ProofContext::new(env);
    let refined = MonoType::Refined {
        base: Box::new(MonoType::TypeRef("Box".into())),
        constraint: ConstExpr::FieldPath {
            base: Box::new(ConstExpr::NamedVar("b".into())),
            field: "w".into(),
        },
    };

    // Act
    let verdict = crate::frontend::core::sema::layers::predicate::check_predicate_with_mode(
        &ctx,
        &refined,
        &std::collections::HashMap::new(),
        PredicateMode::Full,
    );

    // Assert
    assert!(
        verdict.is_ok(),
        "admission 必须在求值器/SMT 之前放行；实际: {verdict:?}"
    );
}
