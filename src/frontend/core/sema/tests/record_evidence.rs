//! 依赖记录 Σ 的声明期分类与构造期义务 — 6.9.5-B.2（B.2a）
//!
//! 规范锚：RFC-027 §14.1（space→position→category）/ §14.2（构造点义务）/
//! §14.6（擦除与码分工）；SPEC §8.3.1 / §8.3.2 / §8.3.6；09a
//! §P6 6.9.5-B.2 契约 §七（T01–T21 的 B.2a 面）。
//!
//! 本文件用**真实源码**走 lex → parse → check：B.2 起声明期分类产 witness，
//! 不再需要 B.1 的直驱标量词（那条通道保留在 witness_fields.rs，作为表示层判据）。
//! 每个正向判据都配同形负控，防止「把普通记录一并拒掉」被误当成通过。

use crate::ast::Quantity;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::record_model::RecordModel;
use crate::frontend::core::sema::TypeCheckResult;

/// 真实 lex → parse → 收集式类型检查。
fn check(source: &str) -> TypeCheckResult {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须能解析；错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("record-evidence");
    checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 真实管线 + 证明函数执行（与 driver 的 ProofExecution 臂同一个执行器）。
fn check_and_execute_proofs(
    source: &str
) -> (TypeCheckResult, Vec<crate::util::diagnostic::Diagnostic>) {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须能解析；错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("record-evidence-proofs");
    let result = checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let proof_errors = crate::frontend::proof_execution::execute_proof_calls(
        result.proof_calls(),
        &parsed.module,
        &result,
    );
    (result, proof_errors)
}

/// 诊断集里是否有指定码。
fn has_code(
    result: &TypeCheckResult,
    code: &str,
) -> bool {
    result.diagnostics.iter().any(|diag| diag.code == code)
}

/// 诊断码列表（断言失败时附在消息里）。
fn codes_of(result: &TypeCheckResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|diag| diag.code.as_str())
        .collect()
}

/// 记录模型（不存在即测试夹具问题）。
fn model_of<'a>(
    result: &'a TypeCheckResult,
    name: &str,
) -> &'a RecordModel {
    result.record_models.get(name).unwrap_or_else(|| {
        panic!(
            "夹具必须为 {name} 产出记录模型；实际键: {:?}",
            result.record_models.keys().collect::<Vec<_>>()
        )
    })
}

/// 字段量词序列。
fn quantities(model: &RecordModel) -> Vec<Option<Quantity>> {
    model.fields.iter().map(|field| field.quantity).collect()
}

/// T01：谓词应用 + 严格前序数据字段 ⇒ evidence（Some(Zero)），并产出义务与索引。
#[test]
fn test_record_classification_marks_evidence_fields() {
    // Arrange
    let source = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
        "\n",
        "b = Box(3, 5)\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "Box(3, 5) 必须零诊断；实际: {:?}",
        codes_of(&result)
    );
    let model = model_of(&result, "Box");
    assert_eq!(
        quantities(model),
        vec![None, Some(Quantity::Zero), None],
        "只有 w 是 evidence；实际量词: {:?}",
        quantities(model)
    );
    assert_eq!(
        model.evidence.len(),
        1,
        "evidence 槽必须与 evidence 字段同长"
    );
    assert_eq!(model.obligations.len(), 1, "每条 evidence 字段一条义务");
    assert_eq!(
        model.obligations[0].depends_on,
        vec![0],
        "w: Pos(v) 依赖严格前序数据字段 v（声明序 0）"
    );
    assert_eq!(
        model
            .field_path
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["v", "w", "z"],
        "field_path 必须保声明序"
    );
}

/// T02：数据族应用（Array(Int, n)）不因引用前序字段变为 evidence。
#[test]
fn test_data_family_field_stays_data() {
    // Arrange
    let source = concat!(
        "Buf: Type = {\n",
        "    n: Int,\n",
        "    data: Array(Int, n),\n",
        "}\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "数据族自引用是合法数据字段；实际: {:?}",
        codes_of(&result)
    );
    let model = model_of(&result, "Buf");
    assert_eq!(
        quantities(model),
        vec![None, None],
        "Array(Int, n) 的空间是 Data，data 仍是数据字段；实际: {:?}",
        quantities(model)
    );
}

/// T03：自引用精化是 admission——数据字段 + 构造点证明自引用命题。
#[test]
fn test_self_refined_field_checks_admission() {
    // Arrange
    let source = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Pos(v),\n",
        "}\n",
        "\n",
        "b = Box(5)\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "admission 字段的值类型是 base（Int），Box(5) 必须零诊断；实际: {:?}",
        codes_of(&result)
    );
    let model = model_of(&result, "Box");
    assert_eq!(
        quantities(model),
        vec![None],
        "admission 是数据字段（不是 witness）；实际: {:?}",
        quantities(model)
    );
    assert_eq!(
        model.admission_fields.len(),
        1,
        "admission 必须进模型（构造点义务的来源）"
    );
}

/// T11：evidence 命题在构造点被判伪 ⇒ E4018（带反例）。
#[test]
fn test_evidence_failure_reports_e4018() {
    // Arrange
    let source = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
        "\n",
        "b = Box(-3, 5)\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        has_code(&result, "E4018"),
        "Box(-3, 5) 的 w: Pos(v) 必须报 E4018；实际: {:?}",
        codes_of(&result)
    );
}

/// T14：嵌套位置的构造点也要判（表达式任意位置，不只语句位置）。
#[test]
fn test_nested_construction_checks_obligation() {
    // Arrange
    let source = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
        "\n",
        "take: (b: Box) -> Void = {\n",
        "}\n",
        "\n",
        "f: () -> Void = {\n",
        "    take(Box(-1, 99))\n",
        "}\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        has_code(&result, "E4018"),
        "嵌套构造 take(Box(-1, 99)) 必须报 E4018；实际: {:?}",
        codes_of(&result)
    );
}

/// T12：命题含无法静态求值的依赖 ⇒ E2031（不静默放行）。
#[test]
fn test_unprovable_construction_reports_e2031() {
    // Arrange
    let source = concat!(
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
        "\n",
        "Box2: Type = {\n",
        "    v: Int,\n",
        "    w: Eq(v, 3),\n",
        "}\n",
        "\n",
        "f: (n: Int) -> Void = {\n",
        "    b: Box2 = Box2(n)\n",
        "}\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        has_code(&result, "E2031"),
        "参数依赖的构造义务判不了时必须报 E2031；实际: {:?}",
        codes_of(&result)
    );
}

/// T13：证明函数证据——正例零诊断、反例 E4018（编译期执行证明函数）。
///
/// 证明函数的执行在 driver 的 `Stage::ProofExecution` 臂；本测直接调用同一
/// 执行器（`execute_proof_calls`），避免把「收集到证明调用」误当成「已证明」。
#[test]
fn test_proof_function_evidence_is_executed() {
    // Arrange
    let positive = concat!(
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
        "\n",
        "Box2: Type = {\n",
        "    v: Int,\n",
        "    w: Eq(v, 3),\n",
        "}\n",
        "\n",
        "b = Box2(3)\n",
    );
    let negative = concat!(
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
        "\n",
        "Box2: Type = {\n",
        "    v: Int,\n",
        "    w: Eq(v, 3),\n",
        "}\n",
        "\n",
        "b = Box2(4)\n",
    );

    // Act
    let (positive_result, positive_proofs) = check_and_execute_proofs(positive);
    let (negative_result, negative_proofs) = check_and_execute_proofs(negative);

    // Assert
    assert!(
        positive_result.diagnostics.is_empty() && positive_proofs.is_empty(),
        "Eq(3, 3) 应由证明函数证出；诊断: {:?}，证明错误: {:?}",
        codes_of(&positive_result),
        positive_proofs
    );
    assert!(
        negative_proofs.iter().any(|diag| diag.code == "E4018")
            || has_code(&negative_result, "E4018"),
        "Eq(4, 3) 必须报 E4018（证明函数返回 false）；诊断: {:?}，证明错误: {:?}",
        codes_of(&negative_result),
        negative_proofs
    );
}

/// T10/T16：构造 arity = 数据字段数——证据不占位，多传/缺参报 E1010。
#[test]
fn test_witness_constructor_arity_skips_evidence() {
    // Arrange
    let base = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
    );
    let positive = format!("{base}\nb = Box(3, 5)\n");
    let extra = format!("{base}\nb = Box(3, 5, 9)\n");
    let missing = format!("{base}\nb = Box(3)\n");

    // Act
    let positive_result = check(&positive);
    let extra_result = check(&extra);
    let missing_result = check(&missing);

    // Assert
    assert!(
        positive_result.diagnostics.is_empty(),
        "Box(3, 5) 是合法构造（2 个数据字段）；实际: {:?}",
        codes_of(&positive_result)
    );
    assert!(
        has_code(&extra_result, "E1010"),
        "多传证据实参 Box(3, 5, 9) 必须报 E1010；实际: {:?}",
        codes_of(&extra_result)
    );
    assert!(
        has_code(&missing_result, "E1010"),
        "缺数据字段 Box(3) 必须报 E1010；实际: {:?}",
        codes_of(&missing_result)
    );
}

/// T15：命名实参按数据字段入槽——未知证据名报 E1014，正例零诊断。
#[test]
fn test_named_arguments_bind_data_fields_only() {
    // Arrange
    let base = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
    );
    let positive = format!("{base}\nb = Box(v = 3, z = 5)\n");
    let unknown = format!("{base}\nb = Box(w = 9)\n");

    // Act
    let positive_result = check(&positive);
    let unknown_result = check(&unknown);

    // Assert
    assert!(
        positive_result.diagnostics.is_empty(),
        "Box(v = 3, z = 5) 必须零诊断；实际: {:?}",
        codes_of(&positive_result)
    );
    assert!(
        has_code(&unknown_result, "E1014"),
        "命名证据字段 Box(w = 9) 必须报 E1014（可用名只有数据字段）；实际: {:?}",
        codes_of(&unknown_result)
    );
}

/// T17：普通记录的构造字段类型检查（D59 C 类：静默给错答案 → 编译期 E1002）。
#[test]
fn test_non_generic_constructor_checks_field_types() {
    // Arrange
    let bad = concat!(
        "Point: Type = {\n",
        "    x: Int,\n",
        "    y: Int,\n",
        "}\n",
        "\n",
        "p = Point(\"a\", 2)\n",
    );
    let good = concat!(
        "Point: Type = {\n",
        "    x: Int,\n",
        "    y: Int,\n",
        "}\n",
        "\n",
        "p = Point(1, 2)\n",
    );

    // Act
    let bad_result = check(bad);
    let good_result = check(good);

    // Assert
    assert!(
        has_code(&bad_result, "E1002"),
        "Point(\"a\", 2) 必须在编译期报 E1002；实际: {:?}",
        codes_of(&bad_result)
    );
    assert!(
        good_result.diagnostics.is_empty(),
        "Point(1, 2) 必须零诊断；实际: {:?}",
        codes_of(&good_result)
    );
}

/// T19/T20：E1114/E1115 的生产可达（B.2 起分类产 witness）。
#[test]
fn test_witness_read_and_mutation_are_rejected_in_production() {
    // Arrange
    let read = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
        "\n",
        "b = Box(3, 5)\n",
        "s = b.w\n",
    );
    let mutate = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v mut: Int,\n",
        "    w: Pos(v),\n",
        "}\n",
    );

    // Act
    let read_result = check(read);
    let mutate_result = check(mutate);

    // Assert
    assert!(
        has_code(&read_result, "E1114"),
        "值级 b.w 必须报 E1114；实际: {:?}",
        codes_of(&read_result)
    );
    assert!(
        has_code(&mutate_result, "E1115"),
        "witness-bearing 记录的 mut 字段声明必须报 E1115；实际: {:?}",
        codes_of(&mutate_result)
    );
}

/// T18：普通记录（无 evidence）的构造/读取全路径零新码（负控）。
#[test]
fn test_plain_record_paths_stay_clean() {
    // Arrange
    let source = concat!(
        "Point: Type = {\n",
        "    x: Int,\n",
        "    y: Int,\n",
        "}\n",
        "\n",
        "p = Point(1, 2)\n",
        "s = p.x + p.y\n",
    );

    // Act
    let result = check(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "普通记录的构造/读取不得出现新诊断；实际: {:?}",
        codes_of(&result)
    );
}

/// T04/T05/T06：声明期非法位置被拒（码位待 D2 裁决，本测只断言「硬错误」）。
#[test]
fn test_illegal_evidence_positions_are_rejected() {
    // Arrange
    let forward = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Bad: Type = {\n",
        "    w: Pos(z),\n",
        "    z: Int,\n",
        "}\n",
    );
    let evidence_ref = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "NonNeg: (v: Int) -> Type = { v >= 0 }\n",
        "\n",
        "Bad: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    e: NonNeg(w),\n",
        "}\n",
    );
    let with_default = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Bad: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v) = 7,\n",
        "}\n",
    );

    // Act
    let forward_result = check(forward);
    let evidence_ref_result = check(evidence_ref);
    let default_result = check(with_default);

    // Assert
    for (name, result) in [
        ("前向引用", &forward_result),
        ("引用 evidence 字段", &evidence_ref_result),
        ("evidence 默认值", &default_result),
    ] {
        assert!(
            !result.diagnostics.is_empty(),
            "{name}必须被声明期拒绝（码位待 D2 裁决）；实际: {:?}",
            codes_of(result)
        );
    }
}

/// T09：字段位非法谓词实参走既有码（E1093）；合法形态零诊断。
#[test]
fn test_malformed_predicate_arg_reports_existing_code() {
    // Arrange
    let bad = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(1, 2),\n",
        "}\n",
    );

    // Act
    let result = check(bad);

    // Assert
    assert!(
        has_code(&result, "E1093") || has_code(&result, "E1092"),
        "字段位的 Pos(1, 2) 必须走既有精化码（E1093/E1092）；实际: {:?}",
        codes_of(&result)
    );
}

/// T07：Registry 阶段与完整检查阶段的量词读数一致（两阶段不分叉）。
#[test]
fn test_registry_and_full_stage_quantities_agree() {
    // Arrange
    let source = concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
    );
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "夹具必须能解析");

    // Act
    let mut registry_checker = TypeChecker::new("record-evidence-registry");
    registry_checker.collect_signatures(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let registry_quantities = registry_checker
        .env()
        .record_models
        .get("Box")
        .map(quantities);
    let full = check(source);
    let full_model = model_of(&full, "Box");

    // Assert
    assert_eq!(
        registry_quantities,
        Some(quantities(full_model)),
        "Registry 面与实检面的 quantity 必须逐位相等（否则导出面与实检面分叉）"
    );
    assert_eq!(
        registry_quantities,
        Some(vec![None, Some(Quantity::Zero), None]),
        "两阶段都必须认出 w 是 evidence；实际: {:?}",
        registry_quantities
    );
}
